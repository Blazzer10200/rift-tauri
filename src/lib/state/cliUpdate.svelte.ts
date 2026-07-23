// Claude Code CLI update detector + in-app updater.
//
// Rift spawns the local `claude` CLI and already reads its installed version
// (assistant.auth.cliVersion ← `claude --version`). This store supplies the
// other half: the newest version on each RELEASE FEED. Two feeds, on purpose:
//   * npm installs track the npm registry's dist-tag manifest —
//     GET https://registry.npmjs.org/@anthropic-ai/claude-code/latest →
//     { version } — fetched from the webview (CSP `connect-src` grants it).
//   * native installs track the native installer channel (what `claude update`
//     and install.ps1/sh actually serve) — fetched via the `cli_native_latest`
//     backend command. npm routinely runs a few patches AHEAD of this channel,
//     so judging a native install against npm produced an unfixable nag: a
//     perpetual "Update available" whose update correctly reported up-to-date
//     and whose reinstall served the same version.
// Results + last-check time + the version the user dismissed all persist to
// localStorage, so the check is throttled (≤ once / 6h) and a dismissed
// version stops nagging until the next one ships.
//
// Staleness is judged against EVERY detected install (`assistant.auth.installs`
// — a box can carry both an npm-global and a native copy that drift apart),
// each compared against ITS OWN feed, so `availableAny`/`isAnyStale` flag an
// update if any one is behind, not just the active one Rift spawns.
//
// `runUpdate()` then applies the update IN-APP via the `assistant_update_cli`
// backend command, which updates ALL installs at once — npm once (`npm install
// -g …@latest`), native per-binary (`<exe> update`) — so the versions can't
// stay skewed. It returns the post-update enumeration; if a copy is STILL
// behind latest afterward (a native no-op that reports success), `updateStuck`
// is set so the UI can point the user at a manual reinstall. Per-install copy
// buttons (`commandFor`) show the right command for each method so a native
// user is never told to run npm (which would create a conflicting install).

import { invoke } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { assistant } from "./assistant.svelte";

const PKG = "@anthropic-ai/claude-code";
const LATEST_URL = `https://registry.npmjs.org/${PKG}/latest`;
const LS_KEY = "rift.cliUpdate.v1";
const STALE_MS = 6 * 60 * 60 * 1000; // re-check at most every 6 hours
const FETCH_TIMEOUT_MS = 10_000;
// A failed check used to stick for the whole session — the only re-trigger was a
// component remount, so a blip (offline at launch, npm hiccup) left the badge
// silently absent. Auto-retry a few times with backoff before giving up.
const RETRY_DELAYS_MS = [30_000, 120_000, 300_000];

type Persisted = {
  latest: string | null;
  nativeLatest?: string | null;
  checkedAt: number;
  dismissed: string | null;
};

/** The slice of a detected install the staleness logic needs. `method` decides
 *  which feed the install is judged against (absent = legacy caller → npm). */
type InstallRef = { version: string | null; method?: string | null };

/** Pull a `major.minor.patch` triple out of a version string, tolerating a
 *  leading `v` and trailing noise like `"2.1.111 (Claude Code)"`. */
function parseSemver(v: string): [number, number, number] | null {
  const m = v.trim().match(/(\d+)\.(\d+)\.(\d+)/);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

/** The CLI version at which every Rift-gated spawn feature is available.
 *  Sits at 2.1.214: covers the HIGHEST gate in `cli_caps::mins`
 *  (`FORWARD_SUBAGENT_TEXT` = 2.1.211 — live sub-agent reasoning in agent
 *  cards) plus the ungated stream additions Rift renders (`tool_progress`
 *  heartbeats + the EndConversation tool, both added at 2.1.214) and the
 *  2.1.208 perf batch (7× faster tool rounds). Keep in lockstep with new
 *  Rust-side gates. */
export const CLI_RECOMMENDED_VERSION = "2.1.214";

/** >0 if a is newer than b, <0 if older, 0 if equal/unparseable. An
 *  unparseable operand fails closed (returns 0 → treated up-to-date), so warn
 *  once — else a CLI whose version string we can't read silently never badges. */
export function cmpSemver(a: string, b: string): number {
  const pa = parseSemver(a);
  const pb = parseSemver(b);
  if (!pa || !pb) {
    console.warn(`cliUpdate: unparseable version, skipping comparison (a=${JSON.stringify(a)} b=${JSON.stringify(b)})`);
    return 0;
  }
  for (let i = 0; i < 3; i++) {
    if (pa[i] !== pb[i]) return pa[i] - pb[i];
  }
  return 0;
}

export class CliUpdate {
  /** Newest version on npm (null until a successful check). */
  latest = $state<string | null>(null);
  /** Newest version on the native installer channel (null until read). */
  nativeLatest = $state<string | null>(null);
  /** Epoch ms of the last successful check. */
  checkedAt = $state<number | null>(null);
  status = $state<"idle" | "checking" | "ok" | "error">("idle");
  error = $state<string | null>(null);
  /** The version the user dismissed — suppresses the badge for that version. */
  dismissed = $state<string | null>(null);
  /** Transient flag for the "Copied!" affordance on the copy-command button. */
  copied = $state(false);
  private _copyTimer: ReturnType<typeof setTimeout> | null = null;
  /** Backoff state for auto-retrying a failed npm check. */
  private _retryTimer: ReturnType<typeof setTimeout> | null = null;
  private _retries = 0;

  /** Install method from the auth probe ("npm" | "native" | "unknown" | null).
   *  Components sync this via setMethod() so the command shown matches reality. */
  method = $state<string | null>(null);
  /** True while an in-app `assistant_update_cli` run is in flight. */
  updating = $state(false);
  /** Error from the most recent runUpdate(), or null. Shown inline. */
  updateError = $state<string | null>(null);
  /** Tail of the updater's output on success — brief "what happened" line. */
  updateOutput = $state<string | null>(null);
  /** True when the last runUpdate() finished but an install is STILL behind
   *  npm's latest — the signature of a native copy that reports success without
   *  actually bumping. Drives the "may need a manual reinstall" hint. */
  updateStuck = $state(false);
  /** True after a successful in-app CLI update: the backend caches the resolved
   *  CLI path at startup, so only a relaunch provably puts every turn on the
   *  new binary. Drives the "Restart Rift to finish" banner row. */
  restartReady = $state(false);
  /** True while app_restart_now is reaping children + relaunching. */
  restarting = $state(false);
  /** The command string most recently copied via copyValue() — lets a single
   *  per-row copy button flip to its "Copied!" state without affecting others. */
  copiedCmd = $state<string | null>(null);
  private _rowCopyTimer: ReturnType<typeof setTimeout> | null = null;

  readonly changelogUrl = `https://www.npmjs.com/package/${PKG}`;

  /** The exact upgrade command for the detected install method. Native installs
   *  self-update + accept `claude update`; everything else uses npm's `@latest`
   *  (NOT `npm update -g`, which respects the original semver range).
   *
   *  Stuck-native exception: once `claude update` has run and the version still
   *  didn't move (`updateStuck`), re-suggesting `claude update` is useless — a
   *  native install's update applies on RESTART, or its managed dir isn't
   *  writable. Hand the user the documented reinstall command instead. */
  get updateCommand(): string {
    if (this.updateStuck && this.method === "native") return this.reinstallCommand;
    return this.commandFor(this.method);
  }

  /** The upgrade command for a specific install method — so a multi-install box
   *  can show the right command per copy (npm vs native), not just the active
   *  one's. */
  commandFor(method: string | null | undefined): string {
    return method === "native" ? "claude update" : `npm install -g ${PKG}@latest`;
  }

  /** Official native (re)install command for the current OS — the documented fix
   *  for a native install whose `claude update` no-ops. Per code.claude.com/docs:
   *  Windows PowerShell uses the `.ps1` one-liner; macOS/Linux/WSL use the shell
   *  installer. UA sniff is best-effort; npm users never hit this path. */
  get reinstallCommand(): string {
    const ua = (typeof navigator !== "undefined" && navigator.userAgent) || "";
    return /Win(dows|32|64)/i.test(ua)
      ? "irm https://claude.ai/install.ps1 | iex"
      : "curl -fsSL https://claude.ai/install.sh | bash";
  }

  setMethod(m: string | null) {
    if (this.method !== m) this.method = m;
  }

  constructor() {
    try {
      const raw = localStorage.getItem(LS_KEY);
      if (raw) {
        const p = JSON.parse(raw) as Persisted;
        this.latest    = typeof p.latest    === "string" ? p.latest    : null;
        this.nativeLatest = typeof p.nativeLatest === "string" ? p.nativeLatest : null;
        this.checkedAt = typeof p.checkedAt === "number" ? p.checkedAt : null;
        this.dismissed = typeof p.dismissed === "string" ? p.dismissed : null;
      }
    } catch {
      /* corrupt cache — ignore, a fresh check rebuilds it */
    }
  }

  private persist() {
    try {
      const p: Persisted = {
        latest: this.latest,
        nativeLatest: this.nativeLatest,
        checkedAt: this.checkedAt ?? 0,
        dismissed: this.dismissed,
      };
      localStorage.setItem(LS_KEY, JSON.stringify(p));
    } catch {
      /* private mode / quota — non-fatal */
    }
  }

  /** The newest version on the feed THIS install method actually updates from.
   *  Only "native" reads the native channel; npm/unknown/absent read npm (the
   *  update command shown for those is `npm install -g …@latest`). A null feed
   *  fails quiet — better to under-report briefly than mis-compare a native
   *  install against npm and nag about a version its updater can't install. */
  latestFor(method: string | null | undefined): string | null {
    return method === "native" ? this.nativeLatest : this.latest;
  }

  /** The applicable feed's latest is strictly newer than the installed CLI.
   *  `method` picks the feed (absent = npm, the legacy behavior). */
  isNewer(installed: string | null, method?: string | null): boolean {
    const target = this.latestFor(method);
    if (!installed || !target) return false;
    return cmpSemver(target, installed) > 0;
  }

  /** An update exists AND the user hasn't dismissed this exact version. */
  available(installed: string | null): boolean {
    return this.isNewer(installed) && this.latest !== this.dismissed;
  }

  /** The version an update would take the stale install(s) TO — the newest
   *  applicable feed version among installs behind their OWN feed. Null when
   *  everything is current (or no feed data yet). This is what every update
   *  surface displays and what dismissals pin. Falls back to the single active
   *  version + synced method when the backend hasn't reported installs yet. */
  targetFor(
    installs: InstallRef[] | null | undefined,
    fallback: string | null,
  ): string | null {
    const list: InstallRef[] =
      installs && installs.length
        ? installs
        : fallback
          ? [{ version: fallback, method: this.method }]
          : [];
    let best: string | null = null;
    for (const inst of list) {
      if (!inst.version) continue;
      const target = this.latestFor(inst.method ?? this.method);
      if (!target || cmpSemver(target, inst.version) <= 0) continue;
      if (!best || cmpSemver(target, best) > 0) best = target;
    }
    return best;
  }

  /** AT LEAST ONE detected install is behind its own feed. A box can carry
   *  both an npm and a native copy that drift apart; if either is behind, an
   *  update is warranted. */
  isAnyStale(
    installs: InstallRef[] | null | undefined,
    fallback: string | null,
  ): boolean {
    return this.targetFor(installs, fallback) != null;
  }

  /** A CLI was found but the ACTIVE install's `--version` couldn't be read —
   *  the backend gates it conservative-old (`cli_caps`), so advanced spawn
   *  flags are silently off. Distinct from "no CLI at all" (empty installs —
   *  onboarding/auth owns that surface). (#42) */
  versionUnreadable(
    installs: InstallRef[] | null | undefined,
    activeVersion: string | null,
  ): boolean {
    return (installs?.length ?? 0) > 0 && !activeVersion;
  }

  /** True once the npm check failed AND the auto-retry backoff ladder is
   *  exhausted — detection is genuinely dead, not a blip a scheduled retry may
   *  still heal. Gates the quiet banner affordance (#42): outside Settings, a
   *  check failure only surfaces when it's persistent. */
  get checkFailedPersistently(): boolean {
    return (
      this.status === "error" &&
      this._retryTimer == null &&
      this._retries >= RETRY_DELAYS_MS.length
    );
  }

  /** Single source for the contextual "what's going on" line shown by every
   *  update surface (Home banner, tab-bar popover, Settings). Previously this
   *  5-way branch was hand-re-authored in three components and drifted; now they
   *  all read one tone + headline + detail. */
  summary(
    installs: InstallRef[] | null | undefined,
  ): { tone: "accent" | "warn" | "danger"; headline: string; detail: string } {
    const count = installs?.length ?? 0;
    if (this.updateError)
      return { tone: "danger", headline: "Update failed", detail: this.updateError };
    // A check in flight should read as "checking", not a stale "Update available"
    // from a prior result — surfaces that poll summary() mid-check showed the old
    // headline for a beat. (#42)
    if (this.status === "checking" && !this.updating)
      return { tone: "accent", headline: "Checking for updates…", detail: "Contacting the release feeds for the latest claude CLI version." };
    if (this.updateStuck) {
      // Post-update, each install is judged against its OWN feed — so a stuck
      // native here means the updater genuinely didn't move the version, not
      // "npm is ahead of the native channel" (that no longer flags). The swap
      // may still be settling (staged, applies when the CLI next starts) —
      // suggest a re-probe before escalating to reinstall.
      if (this.method === "native")
        return {
          tone: "warn",
          headline: "Update staged — not applied yet",
          detail:
            "The updater ran but this install still reports the old version. Give it a moment, then Re-probe — a staged update lands when the CLI next starts. If it stays behind, reinstall with the command below.",
        };
      return {
        tone: "warn",
        headline: "Still behind after update",
        detail:
          "The update ran but the version didn't change. Copy the command below and run it in a terminal, or reinstall.",
      };
    }
    if (count > 1)
      return {
        tone: "accent",
        headline: "Update available",
        detail: `${count} claude installs found — Rift updates them all.`,
      };
    if (this.method === "native")
      return {
        tone: "accent",
        headline: "Update available",
        detail: "Native installs auto-update in the background — or apply it now.",
      };
    return {
      tone: "accent",
      headline: "Update available",
      detail: "A newer claude CLI is on npm. Rift can update it for you.",
    };
  }

  /** Multi-install variant of available(): any install behind its own feed AND
   *  the version it would update TO not dismissed. */
  availableAny(
    installs: InstallRef[] | null | undefined,
    fallback: string | null,
  ): boolean {
    const target = this.targetFor(installs, fallback);
    return target != null && target !== this.dismissed;
  }

  /** Fetch latest from npm. Skips if checked within STALE_MS unless `force`. */
  async maybeCheck(force = false): Promise<void> {
    if (this.status === "checking") return;
    if (!force && this.checkedAt && Date.now() - this.checkedAt < STALE_MS) return;
    this.status = "checking";
    this.error = null;
    // Native channel poll, concurrent with the npm fetch. Backend-side on
    // purpose (CSP stays npm-only); failure is non-fatal — the last persisted
    // value keeps serving, and with none native staleness fails quiet.
    const nativeFut: Promise<string | null> = invoke<string>("cli_native_latest").catch((e) => {
      console.warn(`cliUpdate: native channel check failed (${e instanceof Error ? e.message : e})`);
      return null;
    });
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), FETCH_TIMEOUT_MS);
    try {
      const res = await fetch(LATEST_URL, {
        signal: ctrl.signal,
        headers: { Accept: "application/json" },
        // The registry's `latest` manifest is CDN-fronted (cache headers set);
        // without this the webview can serve a stale version and miss a new
        // release. We already throttle to once / 6h, so a forced round-trip is cheap.
        cache: "no-store",
      });
      if (!res.ok) throw new Error(`npm registry returned HTTP ${res.status}`);
      const json = (await res.json()) as { version?: string };
      if (!json.version) throw new Error("registry response had no version");
      // A different latest = a new release: drop any prior run-state so a stale
      // error/output never shows next to a fresh version.
      if (json.version !== this.latest) {
        this.updateError = null;
        this.updateOutput = null;
      }
      this.latest = json.version;
      this.checkedAt = Date.now();
      this.status = "ok";
      this.persist();
      // Recovered — drop any pending retry + reset the backoff ladder.
      this._retries = 0;
      if (this._retryTimer) { clearTimeout(this._retryTimer); this._retryTimer = null; }
    } catch (e) {
      this.status = "error";
      this.error =
        e instanceof DOMException && e.name === "AbortError"
          ? "Update check timed out."
          : e instanceof Error
            ? e.message
            : String(e);
      // Schedule a bounded auto-retry so a transient failure doesn't leave
      // detection silently dead until the next app restart / page remount.
      const delay = RETRY_DELAYS_MS[this._retries];
      if (delay != null && this._retryTimer == null) {
        this._retries++;
        this._retryTimer = setTimeout(() => {
          this._retryTimer = null;
          void this.maybeCheck(true);
        }, delay);
      }
    } finally {
      clearTimeout(timer);
      // Land the native channel result regardless of how the npm fetch went.
      const nv = (await nativeFut)?.trim() || null;
      if (nv) {
        if (nv !== this.nativeLatest) {
          // A new native release: drop stale run-state, same as the npm path.
          this.updateError = null;
          this.updateOutput = null;
          this.nativeLatest = nv;
        }
        this.persist();
      }
    }
  }

  /** Apply the update in-app via the backend (routes by install method).
   *  Returns true on success — callers should then re-probe auth so the new
   *  `cliVersion` lands and the badge clears. Falls loud: errors surface in
   *  `updateError` and the copy-command fallback stays available. */
  async runUpdate(): Promise<boolean> {
    if (this.updating) return false;
    // The backend reaps EVERY live CLI child (warm pool + all session PIDs) to
    // release the claude binary's file lock before the install — any in-flight
    // turn in any tab/window dies mid-stream. Never do that silently.
    const live = assistant.liveTabs.length;
    if (live > 0) {
      const what = live === 1 ? "1 conversation is" : `${live} conversations are`;
      // Async native confirm — window.confirm is a SYNCHRONOUS script dialog
      // that blocks the renderer main thread; if the host ever fails to show
      // or answer it, the whole UI freezes permanently (frozen frame, dead
      // input). Never use window.confirm/alert/prompt in Rift.
      const ok = await confirm(
        `${what} still running and will be interrupted by the CLI update.\n\nUpdate anyway?`,
        { title: "Update Claude CLI", kind: "warning", okLabel: "Update anyway", cancelLabel: "Not now" },
      );
      if (!ok) return false;
    }
    this.updating = true;
    this.updateError = null;
    this.updateOutput = null;
    this.updateStuck = false;
    try {
      const res = await invoke<{
        method: string;
        output: string;
        installs?: { version: string | null }[];
      }>("assistant_update_cli");
      this.updateOutput = res.output;
      // The backend updates EVERY install; if one is still behind latest after
      // that, it ran but didn't actually bump (classic native no-op) — flag it
      // so the UI can point the user at a manual reinstall instead of leaving a
      // banner stuck "out of date" with no explanation.
      this.updateStuck = this.isAnyStale(res.installs, null);
      this.restartReady = true;
      return true;
    } catch (e) {
      this.updateError = e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      this.updating = false;
    }
  }

  /** Relaunch Rift to finish applying the CLI update (reaps children first —
   *  shutdown.rs::app_restart_now). On failure, falls loud into updateError. */
  async restartNow(): Promise<void> {
    if (this.restarting) return;
    this.restarting = true;
    try {
      // Same exit class as update-apply: app_restart_now exits without
      // beforeunload, so persist every tab first (v0.131.0 data-loss class).
      await assistant.flushAllNow().catch((e) => console.warn("pre-restart flush failed", e));
      await invoke("app_restart_now");
    } catch (e) {
      this.restarting = false;
      this.updateError = e instanceof Error ? e.message : String(e);
    }
  }

  /** Stop surfacing the badge for `version` (the target the surface showed —
   *  defaults to npm's latest for legacy callers). */
  dismiss(version?: string | null) {
    const v = version ?? this.latest;
    if (v) {
      this.dismissed = v;
      this.persist();
    }
  }

  async copyCommand() {
    try {
      await navigator.clipboard?.writeText(this.updateCommand);
      this.copied = true;
      if (this._copyTimer) clearTimeout(this._copyTimer);
      this._copyTimer = setTimeout(() => {
        this.copied = false;
        this._copyTimer = null;
      }, 1600);
    } catch {
      /* clipboard blocked — the command is visible to copy manually */
    }
  }

  /** Copy an arbitrary string (a per-install command), tracking it in
   *  `copiedCmd` so only the clicked row flips to its "Copied!" state. */
  async copyValue(text: string) {
    try {
      await navigator.clipboard?.writeText(text);
      this.copiedCmd = text;
      if (this._rowCopyTimer) clearTimeout(this._rowCopyTimer);
      this._rowCopyTimer = setTimeout(() => {
        this.copiedCmd = null;
        this._rowCopyTimer = null;
      }, 1600);
    } catch {
      /* clipboard blocked — the command stays visible to copy manually */
    }
  }

  /** Clear copy timers (HMR teardown). */
  dispose() {
    if (this._copyTimer != null) { clearTimeout(this._copyTimer); this._copyTimer = null; }
    if (this._rowCopyTimer != null) { clearTimeout(this._rowCopyTimer); this._rowCopyTimer = null; }
    if (this._retryTimer != null) { clearTimeout(this._retryTimer); this._retryTimer = null; }
  }
}

export const cliUpdate = new CliUpdate();

// HMR teardown — prevents stale timers across hot-reloads.
const _hmrHot = (import.meta as { hot?: { dispose: (cb: () => void) => void } }).hot;
if (_hmrHot) _hmrHot.dispose(() => cliUpdate.dispose());
