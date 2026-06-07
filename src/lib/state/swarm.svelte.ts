// Edit-swarm store (idea-phase-plan §2 Phase 3b). Thin reactive layer over the
// Rust `swarm_*` commands. The orchestration (worktree isolate → edit → verify
// gate → adversarial diff review → cherry-pick) runs entirely in Rust; this
// holds the render-ready report + a live per-agent stage map fed by progress
// events. The SwarmPage (Harness sub-tab) reads off this.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Finding = {
  file: string;
  line?: number | null;
  evidence: string;
  suggestedFix: string;
};

export type AgentOutcome = {
  agent: string;
  file: string;
  findings: number;
  stage: string;
  gate: string; // "pass" | "fail" | "skipped" | "n/a"
  gateDetail: string | null;
  review: string; // "accept" | "reject" | "n/a"
  reviewDetail: string | null;
  merged: boolean;
  diff: string | null;
  error: string | null;
};

export type SwarmReport = {
  agents: AgentOutcome[];
  mergedCount: number;
  mainTreeIntact: boolean;
  root: string;
};

export type SwarmEnv = {
  isGitRepo: boolean;
  cleanTree: boolean;
  claudePresent: boolean;
  nodeModulesPresent: boolean;
  headShort: string | null;
};

type ProgressEvent = { agent: string; stage: string; detail: string };

class SwarmStore {
  env = $state<SwarmEnv | null>(null);
  running = $state(false);
  report = $state<SwarmReport | null>(null);
  error = $state<string | null>(null);
  // Live per-agent stage during a run (agent name → "edit" | "gate" | …).
  live = $state<Record<string, string>>({});

  #unlisten: UnlistenFn | null = null;

  /** Subscribe to swarm progress events. Idempotent. */
  async attach(): Promise<void> {
    if (this.#unlisten) return;
    this.#unlisten = await listen<ProgressEvent>("swarm://progress", (e) => {
      const { agent, stage, detail } = e.payload;
      this.live = { ...this.live, [agent]: `${stage}${detail ? " · " + detail : ""}` };
    });
  }

  detach(): void {
    this.#unlisten?.();
    this.#unlisten = null;
  }

  async checkEnv(root: string): Promise<void> {
    try {
      this.env = await invoke<SwarmEnv>("swarm_env_check", { root });
    } catch (e) {
      this.env = null;
      this.error = String(e);
    }
  }

  async run(root: string, findings: Finding[]): Promise<void> {
    if (this.running) return;
    this.running = true;
    this.error = null;
    this.report = null;
    this.live = {};
    try {
      this.report = await invoke<SwarmReport>("swarm_run", { root, findings });
    } catch (e) {
      this.error = String(e);
    } finally {
      this.running = false;
      this.live = {};
    }
  }
}

export const swarm = new SwarmStore();
