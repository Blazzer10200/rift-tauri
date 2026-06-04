// Claude Code CLI update detector — frontend-only.
//
// Rift spawns the local `claude` CLI and already reads its installed version
// (assistant.auth.cliVersion ← `claude --version`). This store supplies the
// other half: the newest version published to npm. It hits the npm registry's
// lightweight dist-tag manifest directly from the webview —
//   GET https://registry.npmjs.org/@anthropic-ai/claude-code/latest → { version }
// — which the CSP `connect-src` allowlist grants. Result + last-check time +
// the version the user dismissed all persist to localStorage, so the check is
// throttled (≤ once / 6h) and a dismissed version stops nagging until the next
// one ships. Rift only *informs*; the CLI self-updates via npm.

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

  readonly pkg = PKG;
  readonly updateCommand = `npm install -g ${PKG}`;
  readonly changelogUrl = `https://www.npmjs.com/package/${PKG}`;

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
}

export const cliUpdate = new CliUpdate();
