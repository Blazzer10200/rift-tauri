// #67 pre-warming — the frontend trigger.
//
// Spawns a warm `claude` child for a fresh chat tab BEFORE the user hits send,
// so the FIRST real turn skips cold-boot + (full-config) the ~6.3s SessionStart
// hook tax measured in the cont.214 spike. The spare registers in the backend
// warm pool under the tab's already-minted cliSessionId; the real send reuses it
// via the normal warm path when the SpawnKey matches (a picker change before
// send just cold-respawns, never worse than no pre-warm).
//
// Design (docs/design/warm-cli-process.md "DECIDED ARCHITECTURE"):
//  - ONE spare per call, keyed to the current picker signature.
//  - fires only for a FRESH tab (no convo started) WITH a workspace root —
//    a no-root chat is tool-less + conversational (low latency stakes) and a
//    started convo already has/had its warm child.
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

let debounceTimer: ReturnType<typeof setTimeout> | null = null;
// The (sessionId|signature) we last fired for — dedup so we don't re-spawn an
// identical spare. Cleared so a new session or a changed signature re-arms.
let lastFiredKey: string | null = null;

/** The fields that, if changed, make the existing spare mismatch at send time
 *  (mirror of the backend SpawnKey). A change here re-arms the trigger. */
function signatureOf(store: AssistantStore, sessionId: string, root: string): string {
  return [
    sessionId,
    store.effectiveModel,
    effortToFlag(store.thinkingEffort, store.effectiveModel),
    String(store.thinkingEnabled),
    store.permissionMode,
    root,
  ].join("|");
}

/** Request a pre-warm spare for the focused fresh tab, if eligible. Cheap +
 *  debounced + deduped; safe to call from a reactive `$effect` on every tick.
 *  No-ops unless the tab is brand-new (no convo started), has a workspace root,
 *  and isn't already streaming. */
export function requestPrewarm(store: AssistantStore): void {
  const tab = store.activeTab;
  if (!tab) return;
  // Only a FRESH tab benefits: a started convo already cold-spawned (and likely
  // still has) its warm child, and re-warming it would double-spawn.
  if (tab.convoCreatedAt || tab.streaming) return;
  const sessionId = tab.cliSessionId;
  if (!sessionId) return;
  // Require a workspace root: a no-root chat runs tool-less + conversational
  // (the latency that bothered the user is the tool-using, full-config path),
  // and pre-warming it would still pay the hook tax for little felt gain.
  const root = store.effectiveRoot(tab);
  if (!root) return;
  // Auth gate: a logged-out user can't spawn a usable child (the backend turn
  // would error) — don't burn a spawn. Mirrors send()'s auth chokepoint.
  if (!(store.auth?.pill === "green" || store.auth?.pill === "yellow")) return;

  const sig = signatureOf(store, sessionId, root);
  if (sig === lastFiredKey) return; // identical spare already requested

  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = null;
    // Re-validate at fire time — the user may have started typing/sending,
    // switched tabs, or changed the picker during the debounce window.
    const t = store.activeTab;
    if (!t || t.cliSessionId !== sessionId || t.convoCreatedAt || t.streaming) return;
    if (store.effectiveRoot(t) !== root) return;
    if (signatureOf(store, sessionId, root) !== sig) return;
    lastFiredKey = sig;
    void invoke("assistant_prewarm", {
      sessionId,
      model: store.effectiveModel,
      thinkingEffort: store.thinkingEffort,
      thinkingEnabled: store.thinkingEnabled,
      permissionMode: store.permissionMode,
      root,
    }).catch((e) => {
      // Best-effort: a failed spare just means a cold first turn. Reset the
      // dedup key so a later tick can retry rather than latching off forever.
      lastFiredKey = null;
      console.debug("assistant_prewarm failed (first turn will be cold):", e);
    });
  }, PREWARM_DEBOUNCE_MS);
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
