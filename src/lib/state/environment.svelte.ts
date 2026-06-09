// Optional host-tool presence (git/node/npm/cargo/code). Rift runs without
// these; individual features need them. Probed once via the `environment_check`
// backend command and cached — read reactively to hide dead affordances (e.g.
// "Open in VS Code" when `code` isn't on PATH) and surfaced in Settings → About.

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
    } catch {
      // Probe failed (SSR / command missing) — keep optimistic defaults.
    } finally {
      this.#inflight = null;
    }
  }
}

export const environment = new Environment();
