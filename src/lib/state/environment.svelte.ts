// Optional host-tool presence (git/node/npm/cargo/code). Rift runs without
// these; individual features need them. Probed once via the `environment_check`
// backend command and cached — read reactively to hide dead affordances (e.g.
// "Open in VS Code" when `code` isn't on PATH) and surfaced in Settings → System.

import { invoke } from "@tauri-apps/api/core";

type EnvironmentInfo = {
  git: boolean;
  node: boolean;
  npm: boolean;
  cargo: boolean;
  code: boolean;
};

class Environment {
  // Optimistic defaults: assume present until the probe says otherwise, so a
  // working affordance never flickers hidden during the first async probe.
  git = $state(true);
  node = $state(true);
  npm = $state(true);
  cargo = $state(true);
  code = $state(true);
  loaded = $state(false);

  #inflight: Promise<void> | null = null;

  /** Probe once and cache. Concurrent callers share the in-flight promise. */
  ensureLoaded(): Promise<void> {
    if (this.loaded) return Promise.resolve();
    if (this.#inflight) return this.#inflight;
    this.#inflight = this.refresh();
    return this.#inflight;
  }

  /** Force a fresh probe (e.g. the user just installed a tool and reopened Settings). */
  async refresh(): Promise<void> {
    try {
      const e = await invoke<EnvironmentInfo>("environment_check");
      this.git = e.git;
      this.node = e.node;
      this.npm = e.npm;
      this.cargo = e.cargo;
      this.code = e.code;
      this.loaded = true;
      // A tool that now resolves is done installing — flip its row to Installed
      // (and let the post-install poll below wind down).
      const inst = { ...this.installing };
      let changed = false;
      for (const k of TOOL_KEYS) {
        if (inst[k] && e[k]) {
          inst[k] = false;
          changed = true;
        }
      }
      if (changed) this.installing = inst;
    } catch {
      // Probe failed (SSR / command missing) — keep optimistic defaults.
    } finally {
      this.#inflight = null;
    }
  }

  /** Tool keys currently mid-install (winget console open). Drives the button's
   *  "Installing…" state until a re-probe finds the tool. */
  installing = $state<Record<string, boolean>>({});
  installError = $state<string | null>(null);

  #pollTimer: ReturnType<typeof setInterval> | null = null;
  #pollUntil = 0;

  /** Launch a winget install for `key` in a visible console (backend handles the
   *  package mapping + UAC). Marks the tool "installing" optimistically, then
   *  polls the probe so the row flips to Installed by itself when the console
   *  finishes — no manual re-probe required. */
  async install(key: "git" | "node" | "npm" | "cargo" | "code"): Promise<void> {
    this.installError = null;
    this.installing = { ...this.installing, [key]: true };
    try {
      await invoke("install_local_tool", { key });
      this.#startPoll();
    } catch (e) {
      this.installError = typeof e === "string" ? e : String(e);
      this.installing = { ...this.installing, [key]: false };
    }
  }

  /** Re-probe every few seconds while an install is pending. Self-stops when
   *  nothing is installing anymore; a bounded deadline covers an abandoned
   *  console (flags reset so the Install button comes back, fail-visible). */
  #startPoll() {
    this.#pollUntil = Date.now() + 15 * 60_000;
    if (this.#pollTimer) return;
    this.#pollTimer = setInterval(() => {
      const pending = Object.values(this.installing).some(Boolean);
      if (!pending || Date.now() > this.#pollUntil) {
        if (pending) this.installing = {};
        clearInterval(this.#pollTimer!);
        this.#pollTimer = null;
        return;
      }
      void this.refresh();
    }, 5000);
  }

  /** Clear the poll timer (HMR teardown). */
  dispose() {
    if (this.#pollTimer != null) {
      clearInterval(this.#pollTimer);
      this.#pollTimer = null;
    }
  }
}

const TOOL_KEYS = ["git", "node", "npm", "cargo", "code"] as const;

export const environment = new Environment();

// HMR teardown — prevents a stale poll interval across hot-reloads.
const _hmrHot = (import.meta as { hot?: { dispose: (cb: () => void) => void } }).hot;
if (_hmrHot) _hmrHot.dispose(() => environment.dispose());
