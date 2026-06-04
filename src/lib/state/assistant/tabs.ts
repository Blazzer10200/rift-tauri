// M6 (per docs/design/assistant-svelte-split.md) — tab lifecycle + split-pane
// management lifted out of `src/lib/state/assistant.svelte.ts` as free fns
// operating on a host ref. Follows the M5 precedent (persistence.ts): the
// $state fields (openTabs, panes, focusedPaneIdx, currentConvoId, conversations,
// tabs map) STAY declared on AssistantStore; only the LIFECYCLE logic moves
// here. The TabState registry + construction (ensureTab/dropTab/wireTab/
// tabByCliSession) also stays on the store — this module never news a TabState.
//
// Bodies are ported verbatim from the store methods (this.* → host.*) so the
// baked-in bug-fix invariants survive the move unchanged: #143 (don't clobber
// ensureTab's cliSessionId seed via store setters), #144 (tear down per-tab
// state for removed tabs), #145 (debounced save snapshots the tab), #149
// (racy openTab during delete), #181 (persistTabs in finally on restore).

import { MAX_PANES, type PaneState } from "./types";

/** Subset of AssistantStore the tab/pane lifecycle touches. Structural —
 *  mirrors the live public surface of AssistantStore. */
export type TabsHost = {
  // ── $state fields (stay on store) ──
  openTabs: string[];
  panes: PaneState[];
  focusedPaneIdx: number;
  currentConvoId: string | null;
  currentCliSessionId: string | null;
  conversations: { id: string }[];
  tabs: Map<string, unknown>;
  // ── active-tab delegating accessors ──
  messages: { role: string }[];
  streaming: boolean;
  queue: { id: string; text: string }[];
  lastNotice: string | null;
  convoCreatedAt: number | null;
  convoTitle: string | null;
  composerDraft: string;
  composerAttachments: { id: string; mime: string; dataBase64: string; previewUrl: string; sizeBytes: number }[];
  telemetry: { event(name: string, data?: unknown): void };
  // ── methods that stay on store / live in other modules ──
  loadConversation(id: string): Promise<void>;
  persistTabs(): void;
  scheduleSave(flush?: boolean): void;
  stop(): Promise<void>;
  ensureTab(convoId: string, cliSessionId: string): unknown;
  dropTab(convoId: string): void;
  pruneTabUi(id: string): void;
  stashTabUi(id: string | null): void;
  restoreTabUi(id: string | null): void;
};

// ── Panes ────────────────────────────────────────────────────────────────

/** Add a new pane to the right of the focused one. Caps at MAX_PANES.
 *  New pane is auto-filled with the next openTab not already in any pane,
 *  else stays empty (drop a tab in from the tabsbar). Focus moves to new
 *  pane. Persists. */
export function addPane(host: TabsHost) {
  if (host.panes.length >= MAX_PANES) return;
  const taken = new Set(host.panes.map((p) => p.tabId).filter((x): x is string => !!x));
  const fill = host.openTabs.find((id) => !taken.has(id)) ?? null;
  const insertAt = host.focusedPaneIdx + 1;
  const next = host.panes.slice();
  next.splice(insertAt, 0, { tabId: fill });
  host.panes = next;
  host.telemetry.event("pane.add", { count: next.length, fill });
  // Focus the freshly-added pane so subsequent newTab/openTab assigns to it.
  host.stashTabUi(host.currentConvoId);
  host.focusedPaneIdx = insertAt;
  if (fill) {
    const inMeta = host.conversations.some((c) => c.id === fill);
    if (inMeta && !host.tabs.get(fill)) {
      void host.loadConversation(fill);
    } else {
      host.currentConvoId = fill;
    }
  }
  host.restoreTabUi(fill);
  host.persistTabs();
}

/** Close a pane (the pane container, not the tab inside it). Tabs stay in
 *  openTabs — closing a pane just unhooks it. Last pane never closes (always
 *  length≥1). Focused idx is clamped to the new array bounds. Persists. */
export function closePane(host: TabsHost, idx: number) {
  if (host.panes.length <= 1) return;
  if (idx < 0 || idx >= host.panes.length) return;
  const next = host.panes.slice();
  next.splice(idx, 1);
  host.panes = next;
  host.telemetry.event("pane.close", { remaining: next.length });
  // Clamp focused. If we closed the focused pane (or one before it), shift left.
  let newFocus = host.focusedPaneIdx;
  if (idx < host.focusedPaneIdx) newFocus -= 1;
  else if (idx === host.focusedPaneIdx) newFocus = Math.min(idx, next.length - 1);
  newFocus = Math.max(0, Math.min(newFocus, next.length - 1));
  if (newFocus !== host.focusedPaneIdx) {
    setFocusedPane(host, newFocus);
  } else {
    host.persistTabs();
  }
}

/** Move focus to a pane. Stashes outgoing composer draft + restores incoming
 *  so each pane carries its own draft. No-op in single-pane mode. */
export function setFocusedPane(host: TabsHost, idx: number) {
  if (idx < 0 || idx >= host.panes.length) return;
  if (host.focusedPaneIdx === idx && host.currentConvoId === host.panes[idx].tabId) return;
  host.stashTabUi(host.currentConvoId);
  host.focusedPaneIdx = idx;
  const next = host.panes[idx].tabId;
  if (next) {
    const inMeta = host.conversations.some((c) => c.id === next);
    if (inMeta && !host.tabs.get(next)) {
      void host.loadConversation(next);
    } else {
      host.currentConvoId = next;
    }
  } else {
    host.currentConvoId = null;
  }
  host.restoreTabUi(next);
  host.persistTabs();
}

/** Assign a tab to the currently-focused pane. Called by openTab/newTab so
 *  the focused pane's slot follows the active selection. Works in both
 *  single-pane (length=1) and split modes. */
function assignFocusedPane(host: TabsHost, tabId: string | null) {
  const cur = host.panes[host.focusedPaneIdx];
  if (!cur || cur.tabId === tabId) return;
  const next = host.panes.slice();
  next[host.focusedPaneIdx] = { tabId };
  host.panes = next;
}

/** Drop a tab from the tabsbar into a specific pane.
 *  - Single-pane mode (panes.length===1) + dropping a DIFFERENT tab on a
 *    half → enter 2-pane split (existing behavior).
 *  - Multi-pane mode → if target pane already holds this tab, just focus it.
 *    If a SIBLING pane holds it, swap. Else assign + focus.
 *  - paneIdx === panes.length is a sentinel meaning "drop in a new pane at
 *    the end" — auto-adds (cap-aware) and assigns. */
export function dropTabIntoPane(host: TabsHost, tabId: string, paneIdx: number) {
  if (!host.openTabs.includes(tabId)) return;
  if (paneIdx < 0) return;

  // Sentinel: "add new pane at end". Cap-respecting.
  if (paneIdx >= host.panes.length) {
    if (host.panes.length >= MAX_PANES) return;
    const next = host.panes.slice();
    next.push({ tabId });
    host.panes = next;
    const newIdx = next.length - 1;
    host.stashTabUi(host.currentConvoId);
    host.focusedPaneIdx = newIdx;
    const inMeta = host.conversations.some((c) => c.id === tabId);
    if (inMeta && !host.tabs.get(tabId)) {
      void host.loadConversation(tabId);
    } else {
      host.currentConvoId = tabId;
    }
    host.restoreTabUi(tabId);
    host.persistTabs();
    return;
  }

  if (host.panes.length === 1) {
    // Single-pane → drop on a half = enter split. paneIdx is 0 or 1 from
    // the half-detect. If the dragged tab IS the only-pane tab, ignore.
    if (tabId === host.currentConvoId) return;
    const other = paneIdx === 0 ? 1 : 0;
    const next: PaneState[] = [{ tabId: null }, { tabId: null }];
    next[paneIdx] = { tabId };
    next[other] = { tabId: host.currentConvoId };
    host.panes = next;
    host.telemetry.event("pane.split.on", { via: "drag", p0: next[0].tabId, p1: next[1].tabId });
  } else {
    // Already split: same tab in target = focus only.
    if (host.panes[paneIdx].tabId === tabId) {
      setFocusedPane(host, paneIdx);
      return;
    }
    // Same tab in a SIBLING pane = swap (mirror UX).
    const siblingIdx = host.panes.findIndex((p, i) => i !== paneIdx && p.tabId === tabId);
    if (siblingIdx !== -1) {
      const swapped = host.panes.slice();
      swapped[siblingIdx] = { tabId: host.panes[paneIdx].tabId };
      swapped[paneIdx] = { tabId };
      host.panes = swapped;
      setFocusedPane(host, paneIdx);
      return;
    }
    const next = host.panes.slice();
    next[paneIdx] = { tabId };
    host.panes = next;
  }
  // Move focus to the freshly-dropped pane + sync currentConvoId.
  host.stashTabUi(host.currentConvoId);
  host.focusedPaneIdx = paneIdx;
  if (tabId !== host.currentConvoId) {
    const inMeta = host.conversations.some((c) => c.id === tabId);
    if (inMeta && !host.tabs.get(tabId)) {
      void host.loadConversation(tabId);
    } else {
      host.currentConvoId = tabId;
    }
  }
  host.restoreTabUi(tabId);
  host.persistTabs();
}

/** When a tab closes, scrub it from any pane that pointed at it. Panes
 *  become empty (null); the pane container stays so the user can drop a
 *  different tab in or close the pane manually. */
function scrubTabFromPanes(host: TabsHost, id: string) {
  let changed = false;
  const next = host.panes.map((p) => {
    if (p.tabId === id) { changed = true; return { tabId: null }; }
    return p;
  });
  if (changed) host.panes = next;
}

// ── Tab lifecycle ──────────────────────────────────────────────────────────

export async function restoreTabs(host: TabsHost) {
  // #181: persistTabs() in finally so a throw mid-restore doesn't leave the
  // disk record diverged from in-memory state.
  try {
    const raw = localStorage.getItem("rift.ui.tabs.v1");
    if (!raw) return;
    const parsed = JSON.parse(raw) as {
      openTabs?: unknown;
      activeTabId?: unknown;
      panes?: unknown;
      focusedPaneIdx?: unknown;
    };
    const ids = Array.isArray(parsed.openTabs)
      ? parsed.openTabs.filter((s): s is string => typeof s === "string")
      : [];
    const existing = new Set(host.conversations.map((c) => c.id));
    const valid = ids.filter((id) => existing.has(id));
    host.openTabs = valid;
    const active = typeof parsed.activeTabId === "string" ? parsed.activeTabId : null;
    // Restore split state — N-pane shape. Accepts length 1..MAX_PANES.
    // Stale tab refs are pruned to null (pane survives, empty). Legacy
    // null/missing keeps single-pane default.
    if (Array.isArray(parsed.panes) && parsed.panes.length >= 1 && parsed.panes.length <= MAX_PANES) {
      const norm = (p: unknown): PaneState => {
        const id = (p as { tabId?: unknown })?.tabId;
        return { tabId: typeof id === "string" && valid.includes(id) ? id : null };
      };
      const restored = parsed.panes.map(norm);
      // Keep at least one pane; if all restored panes are empty and we're
      // single-length, that's fine — assignFocusedPane will fill on next open.
      host.panes = restored.length > 0 ? restored : [{ tabId: null }];
      const fi = typeof parsed.focusedPaneIdx === "number" ? parsed.focusedPaneIdx : 0;
      host.focusedPaneIdx = Math.max(0, Math.min(fi, host.panes.length - 1));
    }
    // Single load: prefer focused pane tab, then active, then first valid.
    const focusedId = host.panes[host.focusedPaneIdx]?.tabId ?? null;
    const winner = (focusedId && valid.includes(focusedId))
      ? focusedId
      : (active && valid.includes(active))
        ? active
        : valid.length > 0 ? valid[0] : null;
    if (winner) {
      await host.loadConversation(winner);
    }
  } catch (e) {
    console.warn("restoreTabs failed", e);
    host.openTabs = [];
    host.panes = [{ tabId: null }];
  } finally {
    host.persistTabs();
  }
}

/** Open a saved convo as a tab. Push to openTabs if not already there;
 *  activate + load from disk. Unsaved new-tab ids (minted by newTab() but
 *  no send yet → no disk record) drop into a fresh in-memory state instead
 *  of disk-load. Singleton stream pipeline — mid-stream switch is handled
 *  by loadConversation() calling stop(). */
export async function openTab(host: TabsHost, id: string) {
  if (!host.openTabs.includes(id)) {
    host.openTabs = [...host.openTabs, id];
  }
  if (host.currentConvoId === id) {
    host.persistTabs();
    return;
  }
  host.telemetry.event("tab.switch", { from: host.currentConvoId, to: id });
  if (host.messages.length > 0 && host.currentConvoId) {
    host.scheduleSave(true);
  }
  // Stash outgoing tab's composer + attachments before any state change.
  host.stashTabUi(host.currentConvoId);
  const inMeta = host.conversations.some((c) => c.id === id);
  if (inMeta) {
    await host.loadConversation(id);
  } else {
    // Fresh in-memory tab (no disk record yet). Mint a TabState with
    // cliSessionId seeded from convoId — first send() finalizes. Don't
    // stop() here: if another tab was streaming, leave it running in the
    // bg. #143: per-tab fields are already null on the fresh TabState;
    // writing them via store setters would clobber cliSessionId.
    host.ensureTab(id, id);
    host.currentConvoId = id;
    host.queue = [];
    host.lastNotice = null;
  }
  // Restore incoming tab's composer + attachments (loadConversation cleared
  // them; we re-fill from cache if the user had a draft mid-typing).
  host.restoreTabUi(id);
  assignFocusedPane(host, id);
  host.persistTabs();
}

/** Close a tab. Removes from openTabs; convo stays on disk → still in History.
 *  Active-tab close picks the right neighbor (or left if at end); last-tab
 *  close drops to empty state w/ currentConvoId=null. */
export async function closeTab(host: TabsHost, id: string) {
  const idx = host.openTabs.indexOf(id);
  if (idx === -1) return;
  const wasActive = host.currentConvoId === id;
  host.telemetry.event("tab.close", { convoId: id, wasActive });
  const next = host.openTabs.slice();
  next.splice(idx, 1);
  host.openTabs = next;
  // Drop the closing tab's UI scratch + TabState. The convo itself stays
  // on disk via scheduleSave below; only in-memory streaming state is retired.
  host.pruneTabUi(id);
  scrubTabFromPanes(host, id);
  if (wasActive) {
    // Save unsaved tail of the closing tab before switching/clearing.
    if (host.messages.length > 0 && host.convoCreatedAt) {
      host.scheduleSave(true);
    }
    if (host.streaming) await host.stop();
  }
  host.dropTab(id);
  if (wasActive) {
    if (next.length === 0) {
      host.currentConvoId = null;
      host.currentCliSessionId = null;
      host.convoCreatedAt = null;
      host.convoTitle = null;
      host.queue = [];
      host.lastNotice = null;
    } else {
      // Right-priority: the entry that shifted into idx, else last.
      const neighbor = next[idx] ?? next[next.length - 1];
      const inMeta = host.conversations.some((c) => c.id === neighbor);
      if (inMeta) {
        await host.loadConversation(neighbor);
      } else {
        // #143: ensureTab seeds the fresh tab's cliSessionId to neighbor;
        // don't store-write null afterwards or the setter clobbers it.
        host.ensureTab(neighbor, neighbor);
        host.currentConvoId = neighbor;
        host.queue = [];
        host.lastNotice = null;
      }
      host.restoreTabUi(neighbor);
      assignFocusedPane(host, neighbor);
    }
  }
  host.persistTabs();
}

/** Open a fresh empty tab. Mints currentConvoId up-front so the tab can
 *  render before the first send; convoCreatedAt stays null so send() still
 *  flags isFirstTurn=true and the CLI gets --session-id, not --resume. */
export async function newTab(host: TabsHost) {
  // Don't stop the previous tab's stream — newTab leaves background tabs
  // streaming. Save unsaved tail of the previous tab before swapping.
  if (host.messages.length > 0 && host.currentConvoId) {
    host.scheduleSave(true);
  }
  // Snapshot outgoing tab's composer state before we mint the new one.
  host.stashTabUi(host.currentConvoId);
  const id = crypto.randomUUID();
  host.openTabs = [...host.openTabs, id];
  // Fresh TabState — empty messages, no streaming. cliSessionId defaults
  // to the convoId; first send() finalizes if needed.
  host.ensureTab(id, id);
  host.telemetry.event("tab.new", { convoId: id });
  host.currentConvoId = id;
  // #143: per-tab fields default to null/<id> on the freshly minted
  // TabState; writing through the store setters here would clobber
  // cliSessionId back to "" (loses ensureTab's seed value).
  host.queue = [];
  host.lastNotice = null;
  // Fresh tab → empty composer (no cache entry yet).
  host.composerDraft = "";
  host.composerAttachments = [];
  assignFocusedPane(host, id);
  host.persistTabs();
}

/** Clear the active conversation IN PLACE (Claude Code `/clear` semantics).
 *  The old convo is flushed to disk first so it stays in History (nothing
 *  lost), then the SAME tab slot / pane is re-keyed to a fresh empty convo —
 *  the chat view clears without spawning a second tab. No remint needed: the
 *  fresh convoId seeds a new cliSessionId, so the next send mints
 *  `--session-id`, not `--resume`. The old CLI session stays intact on the
 *  archived convo (resumable from History via /openincli). Distinct from
 *  newTab(), which appends a tab and leaves the old one open. */
export async function clearConversation(host: TabsHost) {
  const oldId = host.currentConvoId;
  // No active convo to clear → just behave like a fresh tab.
  if (!oldId) {
    await newTab(host);
    return;
  }
  // Stop any in-flight stream on this tab before swapping it out.
  if (host.streaming) await host.stop();
  // Flush the outgoing convo so it persists to History (nondestructive).
  if (host.messages.length > 0 && host.convoCreatedAt) {
    host.scheduleSave(true);
  }
  // Re-key the SAME tab slot + any pane showing it to a fresh convoId so the
  // clear happens in place (no new tab appended).
  const newId = crypto.randomUUID();
  const idx = host.openTabs.indexOf(oldId);
  const nextTabs = host.openTabs.slice();
  if (idx === -1) nextTabs.push(newId);
  else nextTabs[idx] = newId;
  host.openTabs = nextTabs;
  host.panes = host.panes.map((p) => (p.tabId === oldId ? { tabId: newId } : p));
  // Fresh empty TabState; cliSessionId seeded to newId. #143: don't write the
  // per-tab fields via store setters afterwards or ensureTab's seed is lost.
  host.ensureTab(newId, newId);
  host.telemetry.event("tab.clear", { from: oldId, to: newId });
  host.currentConvoId = newId;
  host.queue = [];
  host.lastNotice = "Conversation cleared — previous chat saved to History.";
  host.composerDraft = "";
  host.composerAttachments = [];
  // Retire the old tab's in-memory state; the disk record stays (still in
  // History). #144: drop both the TabState and its UI scratch.
  host.dropTab(oldId);
  host.pruneTabUi(oldId);
  host.persistTabs();
}

export function reorderTabs(host: TabsHost, fromIdx: number, toIdx: number) {
  if (fromIdx === toIdx) return;
  if (fromIdx < 0 || fromIdx >= host.openTabs.length) return;
  const next = host.openTabs.slice();
  const [moved] = next.splice(fromIdx, 1);
  const clamped = Math.max(0, Math.min(toIdx, next.length));
  next.splice(clamped, 0, moved);
  host.openTabs = next;
  host.persistTabs();
}

export async function cycleTab(host: TabsHost, direction: 1 | -1) {
  if (host.openTabs.length === 0) return;
  const cur = host.currentConvoId ? host.openTabs.indexOf(host.currentConvoId) : -1;
  const n = host.openTabs.length;
  const nextIdx = ((cur < 0 ? 0 : cur + direction) + n) % n;
  await openTab(host, host.openTabs[nextIdx]);
}

export async function closeOtherTabs(host: TabsHost, keepId: string) {
  const others = host.openTabs.filter((id) => id !== keepId);
  if (others.length === 0) return;
  // #144: tear down per-tab state for removed tabs so tabs Map +
  // tabDrafts/tabAttachments/tabScroll don't accumulate over long sessions.
  for (const id of others) {
    host.dropTab(id);
    host.pruneTabUi(id);
  }
  host.openTabs = [keepId];
  if (host.currentConvoId !== keepId) {
    await host.loadConversation(keepId);
  }
  host.persistTabs();
}

/** Wipe all open tabs and drop into the empty-tabs state. Flushes the
 *  current convo if it has messages so nothing's lost; closes streams. */
export async function closeAllTabs(host: TabsHost) {
  if (host.streaming) await host.stop();
  if (host.messages.length > 0 && host.convoCreatedAt) {
    host.scheduleSave(true);
  }
  // Drop every TabState; the convos persisted to disk above.
  for (const id of host.openTabs) {
    host.dropTab(id);
    host.pruneTabUi(id);
    scrubTabFromPanes(host, id);
  }
  host.tabs = new Map();
  host.openTabs = [];
  host.currentConvoId = null;
  host.currentCliSessionId = null;
  host.convoCreatedAt = null;
  host.convoTitle = null;
  host.queue = [];
  host.lastNotice = null;
  host.persistTabs();
}

export async function closeTabsToRight(host: TabsHost, anchorId: string) {
  const idx = host.openTabs.indexOf(anchorId);
  if (idx === -1 || idx === host.openTabs.length - 1) return;
  const kept = host.openTabs.slice(0, idx + 1);
  const removed = host.openTabs.slice(idx + 1);
  const removedActive = host.currentConvoId && !kept.includes(host.currentConvoId);
  // #144
  for (const id of removed) {
    host.dropTab(id);
    host.pruneTabUi(id);
  }
  host.openTabs = kept;
  if (removedActive) {
    await host.loadConversation(anchorId);
  }
  host.persistTabs();
}
