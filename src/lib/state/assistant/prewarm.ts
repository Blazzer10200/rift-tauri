// #67 pre-warming — the frontend trigger.
//
// Spawns a warm `claude` child for a fresh chat tab BEFORE the user hits send,
// so the FIRST real turn skips cold-boot + (full-config) the ~6.3s SessionStart
// hook tax measured in the cont.214 spike. The spare registers in the backend
// warm pool under the tab's already-minted cliSessionId; the real send reuses it
// via the normal warm path when the SpawnKey matches (a picker change before
// send just cold-respawns, never worse than no pre-warm).
//
// Design (persistent-process model — see warm-pool-cold-start-diagnosis.md):
//  - ONE spare per call, keyed to the current picker signature.
//  - fires for any tab with NO live warm child yet WITH a warm-target root: a
//    FRESH tab (`--session-id` spare) OR a started chat reopened after an app
//    restart (`--resume` spare). The warm child is persistent for the session,
//    so a mid-session convo already has its child — the backend no-ops then.
//    Target root is the tab's folder OR the local scratch dir (local mode runs
//    full tools). Only an API-key/sandboxed no-folder chat (tool-less +
//    conversational) is skipped.
//  - debounced; deduped per (sessionId, signature) so a reactive re-tick or a
//    settled picker doesn't re-spawn. A signature change re-arms (the old spare
//    would mismatch at send anyway → backend drains it).
//  - best-effort + silent: a failed prewarm just means the first turn is cold,
//    exactly as today. Never surfaces an error to the user.

import { invoke } from "@tauri-apps/api/core";
import type { AssistantStore } from "../assistant.svelte";
import { effortToFlag } from "./helpers";

// Debounce so a burst of reactive ticks (focus + picker hydrate + draft) coalesce
// into one spawn. ~600ms: long enough to skip the mount thrash, short enough that
// a user reading the empty composer is warm before they type a sentence.
const PREWARM_DEBOUNCE_MS = 600;
// Fast first-fire (2026-06-28 cold-start arc): the 600ms window is what a fast
// typer BEATS — open a tab, type, send in <600ms and the spare never spawned,
// so the first turn is cold. For a NEVER-SEEN signature (a freshly focused tab)
// fire on a much shorter delay; it still coalesces the mount-thrash burst but
// starts the ~450MB spawn ~450ms sooner. Re-warms of an existing signature keep
// the generous window (no rush — the user just finished reading a reply).
const PREWARM_FIRST_FIRE_MS = 150;

let debounceTimer: ReturnType<typeof setTimeout> | null = null;
// The (sessionId|signature) we last fired for — dedup so we don't re-spawn an
// identical spare. Cleared so a new session or a changed signature re-arms.
let lastFiredKey: string | null = null;

/** The fields that, if changed, make the existing spare mismatch at send time
 *  (mirror of the backend SpawnKey). A change here re-arms the trigger. */
function signatureOf(
  store: AssistantStore,
  sessionId: string,
  root: string,
  isFirstTurn: boolean,
): string {
  return [
    sessionId,
    store.effectiveModel,
    effortToFlag(store.thinkingEffort, store.effectiveModel),
    String(store.thinkingEnabled),
    store.permissionMode,
    root,
    // A fresh-tab spare (`--session-id`) and a restart-history spare (`--resume`)
    // are different children; keep them distinct in the dedup key so opening a
    // started chat after the app reopens re-arms rather than matching a prior
    // fresh-tab fire.
    isFirstTurn ? "fresh" : "resume",
  ].join("|");
}

/** Request a pre-warm spare for the focused fresh tab, if eligible. Cheap +
 *  debounced + deduped; safe to call from a reactive `$effect` on every tick.
 *  No-ops unless the tab is brand-new (no convo started), has a workspace root,
 *  and isn't already streaming. */
export function requestPrewarm(store: AssistantStore): void {
  const tab = store.activeTab;
  if (!tab) return;
  // Never pre-warm a tab mid-turn — its warm child is busy.
  if (tab.streaming) return;
  // Pre-warm BOTH a fresh tab (no convo yet → `--session-id` spare) AND a STARTED
  // conversation that currently has no live child (→ `--resume` spare). With the
  // persistent-process model the warm child survives any active-use pause, so the
  // started-convo case is really only "app was reopened and the user clicked an
  // existing chat from history" — its child is gone, so warm it before they type.
  // The backend `assistant_prewarm` no-ops when a live child already exists, so
  // calling this on an already-warm convo is a cheap nothing.
  const isFirstTurn = !tab.convoCreatedAt;
  const sessionId = tab.cliSessionId;
  if (!sessionId) return;
  // Resolve the warm-target root: the tab's folder, else the local scratch dir
  // (`%LOCALAPPDATA%\Rift\local`) when in local mode — the backend keys the
  // no-folder OAuth turn's SpawnKey on that same scratch path, so warming it
  // makes the first real turn a warm hit instead of a cold spawn. Only the truly
  // root-less + scratch-less case (API-key/sandboxed no-folder) bails: there the
  // turn runs tool-less + conversational and pre-warming buys little.
  const root = store.effectiveRoot(tab) ?? store.localScratchPath;
  if (!root) return;
  // Auth gate: a logged-out user can't spawn a usable child (the backend turn
  // would error) — don't burn a spawn. Mirrors send()'s auth chokepoint.
  if (!store.authReady) return;

  const sig = signatureOf(store, sessionId, root, isFirstTurn);
  if (sig === lastFiredKey) return; // identical spare already requested

  // A fresh tab (first-turn, no prior fire latched) races the user's first
  // keystrokes → fire fast. A re-warm or a re-armed signature can take the
  // generous window. `lastFiredKey === null` ⇒ nothing pending/latched here.
  const delay = isFirstTurn && lastFiredKey === null
    ? PREWARM_FIRST_FIRE_MS
    : PREWARM_DEBOUNCE_MS;
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = null;
    // Re-validate at fire time — the user may have started typing/sending,
    // switched tabs, or changed the picker during the debounce window. A convo
    // that STARTED during the window flips isFirstTurn but is still warmable
    // (re-warm path), so we no longer bail on convoCreatedAt — only on a tab/
    // session switch or an in-flight turn.
    const t = store.activeTab;
    if (!t || t.cliSessionId !== sessionId || t.streaming) return;
    if ((store.effectiveRoot(t) ?? store.localScratchPath) !== root) return;
    if (signatureOf(store, sessionId, root, isFirstTurn) !== sig) return;
    lastFiredKey = sig;
    void invoke("assistant_prewarm", {
      sessionId,
      model: store.effectiveModel,
      thinkingEffort: store.thinkingEffort,
      thinkingEnabled: store.thinkingEnabled,
      permissionMode: store.permissionMode,
      root,
      isFirstTurn,
    }).catch((e) => {
      // Best-effort: a failed spare just means a cold first turn. Reset the
      // dedup key so a later tick can retry rather than latching off forever.
      lastFiredKey = null;
      console.debug("assistant_prewarm failed (first turn will be cold):", e);
    });
  }, delay);
}

/** Drop the dedup latch — call when the active tab/session changes so the next
 *  fresh tab re-arms even if its signature coincidentally matches the prior. */
export function resetPrewarmDedup(): void {
  lastFiredKey = null;
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }
}
