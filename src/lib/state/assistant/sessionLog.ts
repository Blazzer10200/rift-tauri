// Multi-session telemetry persistence. Each Rift launch's SessionTelemetry
// snapshot is flushed (debounced) to ~/.rift/assistant/session-logs/<id>.json
// via the Rust commands in src-tauri/src/assistant/session_log.rs, so the
// Harness page can browse past sessions across restarts. Thin IPC wrappers
// only — all swallow errors to a warn (telemetry persistence is best-effort
// and must never surface as a user-facing failure).

import { invoke } from "@tauri-apps/api/core";
import { summarizeSession } from "./telemetry";

/** Thin per-session metadata for the Harness session picker. Mirrors the Rust
 *  `SessionLogMeta` (camelCase serde). */
export type SessionLogMeta = {
  id: string;
  startedAt: number;
  capturedAt: number;
  durationMs: number;
  turnCount: number;
  totalCostUsd: number;
  totalTurns: number;
  toolCallTotal: number;
  model: string | null;
  workspace: string | null;
};

/** Full stored snapshot — `SessionTelemetry.snapshot()` shape plus the
 *  `id`/`model`/`workspace` the save wiring merges in. Loaded when a past
 *  session is selected so the dashboard can re-render from frozen data. */
export type SessionSnapshot = ReturnType<
  import("./telemetry").SessionTelemetry["snapshot"]
> & { model?: string | null; workspace?: string | null };

export async function listSessionLogs(): Promise<SessionLogMeta[]> {
  try {
    return await invoke<SessionLogMeta[]>("assistant_list_session_logs");
  } catch (e) {
    console.warn("assistant_list_session_logs failed", e);
    return [];
  }
}

export async function loadSessionLog(id: string): Promise<SessionSnapshot | null> {
  try {
    const snap = await invoke<SessionSnapshot>("assistant_load_session_log", { id });
    // Re-derive the rollup from the raw turns instead of trusting the frozen
    // on-disk summary (#33): fields added after a log was written (e.g.
    // avgDeadWaitMs) are absent from older summaries and render as "—", while
    // the timeline markers — which recompute from turns[] — show data. One
    // source of truth, and future summarize() additions backfill for free.
    if (snap) snap.summary = summarizeSession(snap.turns ?? [], snap.events ?? []);
    return snap;
  } catch (e) {
    console.warn("assistant_load_session_log failed", e);
    return null;
  }
}

export async function saveSessionLog(record: unknown): Promise<void> {
  try {
    await invoke("assistant_save_session_log", { record });
  } catch (e) {
    console.warn("assistant_save_session_log failed", e);
  }
}

export async function deleteSessionLog(id: string): Promise<void> {
  try {
    await invoke("assistant_delete_session_log", { id });
  } catch (e) {
    console.warn("assistant_delete_session_log failed", e);
  }
}

export async function pruneSessionLogs(keep: number): Promise<void> {
  try {
    await invoke("assistant_prune_session_logs", { keep });
  } catch (e) {
    console.warn("assistant_prune_session_logs failed", e);
  }
}
