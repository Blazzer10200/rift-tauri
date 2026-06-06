// Claude Code CLI update detector + in-app updater.
//
// Rift spawns the local `claude` CLI and already reads its installed version
// (assistant.auth.cliVersion ← `claude --version`). This store supplies the
// other half: the newest version published to npm. It hits the npm registry's
// lightweight dist-tag manifest directly from the webview —
//   GET https://registry.npmjs.org/@anthropic-ai/claude-code/latest → { version }
// — which the CSP `connect-src` allowlist grants. Result + last-check time +
// the version the user dismissed all persist to localStorage, so the check is
// throttled (≤ once / 6h) and a dismissed version stops nagging until the next
// one ships.
//
// Staleness is judged against EVERY detected install (`assistant.auth.installs`
// — a box can carry both an npm-global and a native copy that drift apart), so
// `availableAny`/`isAnyStale` flag an update if any one is behind, not just the
// active one Rift spawns.
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

const PKG = "@anthropic-ai/claude-code";
const LATEST_URL = `https://registry.npmjs.org/${PKG}/latest`;
const LS_KEY = "rift.cliUpdate.v1";
const STALE_MS = 6 * 60 * 60 * 1000; // re-check at most every 6 hours
const FETCH_TIMEOUT_MS = 10_000;

type Persisted = { latest: string | null; checkedAt: number; dismissed: string | null };

/** Pull a `major.minor.patch` triple out of a version string, tolerating a
 *  leading `v` and trailing noise like `"2.1.111 (Claude Code)"`. */
function parseSemver(v: string): [number, number, number] | null {
  const m = v.trim().match(/(\d+)\.(\d+)\.(\d+)/);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

/** >0 if a is newer than b, <0 if older, 0 if equal/unparseable. */
function cmpSemver(a: string, b: string): number {
  const pa = parseSemver(a);
  const pb = parseSemver(b);
  if (!pa || !pb) return 0;
  for (let i = 0; i < 3; i++) {
    if (pa[i] !== pb[i]) return pa[i] - pb[i];
  }
  return 0;
}

class CliUpdate {
  /** Newest version on npm (null until a successful check). */
  latest = $state<string | null>(null);
  /** Epoch ms of the last successful check. */
  checkedAt = $state<number | null>(null);
  status = $state<"idle" | "checking" | "ok" | "error">("idle");
  error = $state<string | null>(null);
  /** The version the user dismissed — suppresses the badge for that version. */
  dismissed = $state<string | null>(null);
  /** Transient flag for the "Copied!" affordance on the copy-command button. */
  copied = $state(false);
  private _copyTimer: ReturnType<typeof setTimeout> | null = null;

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
  /** The command string most recently copied via copyValue() — lets a single
   *  per-row copy button flip to its "Copied!" state without affecting others. */
  copiedCmd = $state<string | null>(null);
  private _rowCopyTimer: ReturnType<typeof setTimeout> | null = null;

  readonly changelogUrl = `https://www.npmjs.com/package/${PKG}`;

  /** The exact upgrade command for the detected install method. Native installs
   *  self-update + accept `claude update`; everything else uses npm's `@latest`
   *  (NOT `npm update -g`, which respects the original semver range). */
  get updateCommand(): string {
    return this.commandFor(this.method);
  }

  /** The upgrade command for a specific install method — so a multi-install box
   *  can show the right command per copy (npm vs native), not just the active
   *  one's. */
  commandFor(method: string | null | undefined): string {
    return method === "native" ? "claude update" : `npm install -g ${PKG}@latest`;
  }

  setMethod(m: string | null) {
    this.method = m;
  }

  constructor() {
    try {
      const raw = localStorage.getItem(LS_KEY);
      if (raw) {
        const p = JSON.parse(raw) as Persisted;
        this.latest = p.latest ?? null;
        this.checkedAt = p.checkedAt ?? null;
        this.dismissed = p.dismissed ?? null;
      }
    } catch {
      /* corrupt cache — ignore, a fresh check rebuilds it */
    }
  }

  private persist() {
    try {
      const p: Persisted = {
        latest: this.latest,
        checkedAt: this.checkedAt ?? 0,
        dismissed: this.dismissed,
      };
      localStorage.setItem(LS_KEY, JSON.stringify(p));
    } catch {
      /* private mode / quota — non-fatal */
    }
  }

  /** npm's latest is strictly newer than the installed CLI. */
  isNewer(installed: string | null): boolean {
    if (!installed || !this.latest) return false;
    return cmpSemver(this.latest, installed) > 0;
  }

  /** An update exists AND the user hasn't dismissed this exact version. */
  available(installed: string | null): boolean {
    return this.isNewer(installed) && this.latest !== this.dismissed;
  }

  /** npm's latest is strictly newer than AT LEAST ONE detected install. A box
   *  can carry both an npm and a native copy that drift apart; if either is
   *  behind, an update is warranted. Falls back to the single active version
   *  when the backend hasn't reported an installs list yet. */
  isAnyStale(
    installs: { version: string | null }[] | null | undefined,
    fallback: string | null,
  ): boolean {
    if (!this.latest) return false;
    const versions =
      installs && installs.length
        ? installs.map((i) => i.version).filter((v): v is string => !!v)
        : fallback
          ? [fallback]
          : [];
    return versions.some((v) => cmpSemver(this.latest as string, v) > 0);
  }

  /** Single source for the contextual "what's going on" line shown by every
   *  update surface (Home banner, tab-bar popover, Settings). Previously this
   *  5-way branch was hand-re-authored in three components and drifted; now they
   *  all read one tone + headline + detail. */
  summary(
    installs: { version: string | null }[] | null | undefined,
  ): { tone: "accent" | "warn" | "danger"; headline: string; detail: string } {
    const count = installs?.length ?? 0;
    if (this.updateError)
      return { tone: "danger", headline: "Update failed", detail: this.updateError };
    if (this.updateStuck)
      return {
        tone: "warn",
        headline: "Still behind after update",
        detail:
          "A native install reported success without bumping. Copy its command and run it in a terminal, or reinstall it.",
      };
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

  /** Multi-install variant of available(): any install behind AND not dismissed. */
  availableAny(
    installs: { version: string | null }[] | null | undefined,
    fallback: string | null,
  ): boolean {
    return this.isAnyStale(installs, fallback) && this.latest !== this.dismissed;
  }

  /** Fetch latest from npm. Skips if checked within STALE_MS unless `force`. */
  async maybeCheck(force = false): Promise<void> {
    if (this.status === "checking") return;
    if (!force && this.checkedAt && Date.now() - this.checkedAt < STALE_MS) return;
    this.status = "checking";
    this.error = null;
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), FETCH_TIMEOUT_MS);
    try {
      const res = await fetch(LATEST_URL, {
        signal: ctrl.signal,
        headers: { Accept: "application/json" },
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
    } catch (e) {
      this.status = "error";
      this.error =
        e instanceof DOMException && e.name === "AbortError"
          ? "Update check timed out."
          : e instanceof Error
            ? e.message
            : String(e);
    } finally {
      clearTimeout(timer);
    }
  }

  /** Apply the update in-app via the backend (routes by install method).
   *  Returns true on success — callers should then re-probe auth so the new
   *  `cliVersion` lands and the badge clears. Falls loud: errors surface in
   *  `updateError` and the copy-command fallback stays available. */
  async runUpdate(): Promise<boolean> {
    if (this.updating) return false;
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
      return true;
    } catch (e) {
      this.updateError = e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      this.updating = false;
    }
  }

  /** Stop surfacing the badge for the current latest version. */
  dismiss() {
    if (this.latest) {
      this.dismissed = this.latest;
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
}

export const cliUpdate = new CliUpdate();
