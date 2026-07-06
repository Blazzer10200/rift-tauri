// Listener-body extraction (cont.276) — the `assistant://stream|done|error`
// closure bodies from assistant.svelte.ts init(), lifted as free fns over a
// minimal structural host so the #80 epoch gates, the bg_task warn-once
// dedup, and the string-vs-object payload unions are unit-testable. Same
// thunk pattern as streaming.ts M8: init()'s listen() callbacks stay 1-line
// thunks (`(e) => handleStreamEvent(host, e.payload)`), so the event wire
// shape is unchanged.

import { isStaleTurnEpoch } from "./helpers";
import { notify } from "../toast.svelte";

/** Structural slice of TabState the listeners touch. */
export type ListenerTab = {
  turnEpoch: number;
  staleTerminalUntil: number;
  onStream(raw: string): void;
  onDone(): void;
  onError(message: string): void;
};

/** Structural slice of AssistantStore: session→tab routing + the
 *  once-per-session bg_task warn registry. `activeTab` must be a live
 *  getter (read at event time, not captured at init). */
export type ListenerHost = {
  readonly activeTab: ListenerTab | null;
  tabBySession(sid: string): ListenerTab | null;
  bgTaskWarnedSessions: Set<string>;
};

// Legacy payload shape (bare string) routes to activeTab for forward-compat
// during dev hot-reload.
export type StreamPayload =
  | { session_id?: string; line?: string; turn_epoch?: number }
  | string;
export type DonePayload = {
  session_id?: string;
  exit_code?: number;
  bg_task?: boolean;
  turn_epoch?: number;
};
export type ErrorPayload =
  | { session_id?: string; message?: string; turn_epoch?: number }
  | string;

/** `assistant://stream` — route by session_id to the right TabState so
 *  background tabs keep painting concurrently with the foreground. */
export function handleStreamEvent(host: ListenerHost, payload: StreamPayload | null | undefined): void {
  if (typeof payload === "string") {
    host.activeTab?.onStream(payload);
    return;
  }
  const { session_id, line, turn_epoch } = payload ?? {};
  const tab = session_id ? host.tabBySession(session_id) : host.activeTab;
  if (!tab || typeof line !== "string") return;
  // #80: a frame from a stopped/superseded turn must not paint into the
  // NEXT turn's bubble — drop it. (Terminals also consume the stop gate;
  // a mere frame doesn't — the stale turn's terminal is still inbound.)
  if (isStaleTurnEpoch(tab.turnEpoch, turn_epoch)) return;
  tab.onStream(line);
}

// Warn-once registry bound: evict the oldest entry (Set preserves insertion
// order) so a long-running session can't grow it without limit.
const MAX_BG_WARNED_SESSIONS = 200;

/** `assistant://done` — terminal routing + the backgrounded-Bash warning. */
export function handleDoneEvent(host: ListenerHost, payload: DonePayload | null | undefined): void {
  const sid = payload?.session_id;
  const tab = sid ? host.tabBySession(sid) : host.activeTab;
  // #80: a terminal from a stopped/superseded turn must not finalize the
  // LIVE turn (the old same-session race: stale DONE nulled the next
  // turn's streamingMsgId + dropped its currentTurnRecord) — consume the
  // post-stop gate so a deferred send proceeds, and skip ONLY onDone.
  // The bg_task warning below still runs: the superseded turn really did
  // background a task that won't auto-report.
  if (tab && isStaleTurnEpoch(tab.turnEpoch, payload?.turn_epoch)) {
    tab.staleTerminalUntil = 0;
  } else {
    tab?.onDone();
  }
  // B: the turn backgrounded a Bash task. In headless -p mode the CLI
  // kills that shell ~5s after the turn and never re-invokes the model,
  // so a "I'll report when it lands" promise can't be kept. Warn once per
  // session (not per turn) so a model that repeatedly backgrounds work
  // doesn't spam a 9s toast every turn.
  if (payload?.bg_task) {
    const key = sid ?? "__active__";
    if (!host.bgTaskWarnedSessions.has(key)) {
      if (host.bgTaskWarnedSessions.size >= MAX_BG_WARNED_SESSIONS) {
        const oldest = host.bgTaskWarnedSessions.values().next().value;
        if (oldest !== undefined) host.bgTaskWarnedSessions.delete(oldest);
      }
      host.bgTaskWarnedSessions.add(key);
      notify.warn("Background task won't auto-report", {
        detail: "This turn started a task in the background, but Rift can't notify you when it finishes. Send a message to ask how it went.",
        timeoutMs: 9000,
      });
    }
  }
}

/** `assistant://error` — same legacy-string + epoch discipline as stream. */
export function handleErrorEvent(host: ListenerHost, payload: ErrorPayload | null | undefined): void {
  if (typeof payload === "string") {
    host.activeTab?.onError(payload);
    return;
  }
  const { session_id, message, turn_epoch } = payload ?? {};
  const tab = session_id ? host.tabBySession(session_id) : host.activeTab;
  if (!tab || typeof message !== "string") return;
  // #80: mirror the done listener — a stale turn's error (e.g. its stop
  // marker was eaten and the DONE remapped to ERROR) must not banner the
  // live turn; consume the gate and drop it.
  if (isStaleTurnEpoch(tab.turnEpoch, turn_epoch)) {
    tab.staleTerminalUntil = 0;
    return;
  }
  tab.onError(message);
}
