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

import {
  MAX_PANES,
  createPaneState,
  type ModelSel,
  type PaneState,
  type PermissionMode,
  type ThinkingEffort,
} from "./types";
import type { TextAttachment } from "./attachments";
import { asModelSel, asPermissionMode, loadModel } from "./helpers";
import { tabsStorageKey } from "./persistence";
import { notify } from "../toast.svelte";
import { shell } from "../shell.svelte";

/** Subset of AssistantStore the tab/pane lifecycle touches. Structural —
 *  mirrors the live public surface of AssistantStore. */
type TabsHost = {
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
  composerAttachments: { id: string; mime: string; dataBase64: string; sizeBytes: number }[];
  composerTextAttachments: TextAttachment[];
  // @-mention walk + branch caches — invalidated on focus change so a pane
  // with a different root re-walks instead of showing a sibling's file list.
  workspaceFiles: string[];
  workspaceBranch: string | null;
  activeRoot: string | null;
  workspaceCurrent: string | null;
  telemetry: { event(name: string, data?: unknown): void };
  // ── methods that stay on store / live in other modules ──
  loadConversation(id: string, opts?: { activate?: boolean; paneId?: string }): Promise<boolean>;
  persistTabs(): void;
  scheduleSave(flush?: boolean, forConvoId?: string): void;
  stop(tabId?: string | null): Promise<void>;
  ensureTab(convoId: string, cliSessionId: string): unknown;
  dropTab(convoId: string): void;
  pruneTabUi(id: string): void;
};

// ── Panes ────────────────────────────────────────────────────────────────

/** Add a new pane to the right of the focused one. Caps at MAX_PANES, and
 *  further at however many panes the current viewport can hold without
 *  unusable slivers (narrow laptops / scaled displays). New pane always starts
 *  EMPTY — its card offers New chat / recent picks / drag-in, which reads as a
 *  deliberate choice instead of a surprise tab appearing. Focus moves to the
 *  new pane. Persists. Returns true if a pane was added. */
export function addPane(host: TabsHost): boolean {
  if (host.panes.length >= MAX_PANES) return false;
  // Width-fit guard: don't split below the min usable pane width.
  const fitCap = shell.maxPanesForWidth();
  if (host.panes.length >= fitCap) {
    notify.warn("Not enough width to split", {
      detail: "Widen the window or collapse the sidebar to open another pane.",
    });
    return false;
  }
  const insertAt = host.focusedPaneIdx + 1;
  const next = host.panes.slice();
  next.splice(insertAt, 0, createPaneState());
  host.panes = next;
  host.telemetry.event("pane.add", { count: next.length });
  // Focus the freshly-added pane so subsequent newTab/openTab assigns to it.
  host.focusedPaneIdx = insertAt;
  host.persistTabs();
  return true;
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
  const prevRoot = host.activeRoot;
  const targetPane = host.panes[idx];
  host.focusedPaneIdx = idx;
  const next = targetPane.tabId;
  if (next) {
    const inMeta = host.conversations.some((c) => c.id === next);
    if (inMeta && !host.tabs.get(next)) {
      // Capture the stable pane identity. A late disk load may hydrate its tab,
      // but may activate it only while this exact pane is still focused and
      // still owns the requested tab.
      const load = host.loadConversation(next, { paneId: targetPane.id });
      host.currentConvoId = null;
      void load.then(() => {
        if (host.activeRoot !== prevRoot) {
          host.workspaceFiles = [];
          host.workspaceBranch = null;
        }
      });
    } else {
      host.currentConvoId = next;
    }
  } else {
    host.currentConvoId = null;
  }
  // Focus moved to a pane with a different folder → drop the @-mention + branch
  // caches so the next read reflects the newly-focused pane's root.
  if (host.activeRoot !== prevRoot) {
    host.workspaceFiles = [];
    host.workspaceBranch = null;
  }
  host.persistTabs();
}

/** Assign a tab to the currently-focused pane. Called by openTab/newTab so
 *  the focused pane's slot follows the active selection. Works in both
 *  single-pane (length=1) and split modes. */
function assignFocusedPane(host: TabsHost, tabId: string | null) {
  const cur = host.panes[host.focusedPaneIdx];
  if (!cur || cur.tabId === tabId) return;
  // Two panes must never key the same tab — the pane {#each} is keyed by
  // tabId, so a duplicate hard-crashes the chat surface AND persists via
  // persistTabs (found live 2026-07-10: closeTab's neighbor pick landed on a
  // tab already visible in a sibling pane). Focus the sibling instead.
  if (tabId != null) {
    const sib = host.panes.findIndex((p, i) => i !== host.focusedPaneIdx && p.tabId === tabId);
    if (sib !== -1) {
      setFocusedPane(host, sib);
      return;
    }
  }
  const next = host.panes.slice();
  next[host.focusedPaneIdx] = { ...cur, tabId };
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
  if (paneIdx < 0) return;
  // Drag-source can be the conversation list, where the convo may not be open
  // yet — admit it by pushing into openTabs first (mirrors openTab's insert), so
  // the load-on-drop path below hydrates it. A tab neither open nor on disk is
  // rejected.
  if (!host.openTabs.includes(tabId)) {
    if (!host.conversations.some((c) => c.id === tabId)) return;
    host.openTabs = [...host.openTabs, tabId];
  }
  // Snapshot before any focus/currentConvoId mutation below — the branches
  // that don't route through setFocusedPane must invalidate the @-mention +
  // branch caches themselves when the drop changes the effective root
  // (mirrors setFocusedPane's own guard at ~139-142).
  const prevRoot = host.activeRoot;

  if (host.panes.length === 1) {
    // Single-pane → drop on a half = enter split. The half-detect passes
    // paneIdx 0 (left) or 1 (right); the dragged tab fills that half. This
    // MUST be checked before the add-new-pane sentinel below: with one pane,
    // a right-half drop is paneIdx 1 and `1 >= panes.length` would otherwise
    // mis-route into the sentinel and just clone the tab into a 2nd pane.
    // The other half gets the current tab — or, when the user drags the tab
    // that's already showing, the next different open tab (empty slot if
    // none) so the gesture still splits instead of cloning/no-opping.
    const other = paneIdx === 0 ? 1 : 0;
    const counterpart =
      tabId === host.currentConvoId
        ? host.openTabs.find((t) => t !== tabId) ?? null
        : host.currentConvoId;
    const existing = host.panes[0] ?? createPaneState();
    const next: PaneState[] = [createPaneState(), createPaneState()];
    next[paneIdx] = createPaneState(tabId);
    next[other] = { ...existing, tabId: counterpart };
    host.panes = next;
    host.telemetry.event("pane.split.on", { via: "drag", p0: next[0].tabId, p1: next[1].tabId });
  } else if (paneIdx >= host.panes.length) {
    // Sentinel: "add new pane at end". Cap-respecting. Multi-pane only —
    // single-pane is handled above.
    // Already visible in some pane → focus it; never mint a duplicate pane
    // (same each-key invariant as assignFocusedPane).
    const visibleIdx = host.panes.findIndex((p) => p.tabId === tabId);
    if (visibleIdx !== -1) {
      setFocusedPane(host, visibleIdx);
      return;
    }
    if (host.panes.length >= MAX_PANES) return;
    // Width-fit guard (same as addPane): refuse a 3rd+ pane that won't fit.
    if (host.panes.length >= shell.maxPanesForWidth()) {
      notify.warn("Not enough width to split", {
        detail: "Widen the window or collapse the sidebar to open another pane.",
      });
      return;
    }
    const next = host.panes.slice();
    next.push(createPaneState(tabId));
    host.panes = next;
    const newIdx = next.length - 1;
    host.focusedPaneIdx = newIdx;
    const inMeta = host.conversations.some((c) => c.id === tabId);
    if (inMeta && !host.tabs.get(tabId)) {
      const paneId = next[newIdx].id;
      host.currentConvoId = null;
      void host.loadConversation(tabId, { paneId });
    } else {
      host.currentConvoId = tabId;
    }
    if (host.activeRoot !== prevRoot) {
      host.workspaceFiles = [];
      host.workspaceBranch = null;
    }
    host.persistTabs();
    return;
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
      swapped[siblingIdx] = { ...swapped[siblingIdx], tabId: host.panes[paneIdx].tabId };
      swapped[paneIdx] = { ...swapped[paneIdx], tabId };
      host.panes = swapped;
      setFocusedPane(host, paneIdx);
      return;
    }
    const next = host.panes.slice();
    next[paneIdx] = { ...next[paneIdx], tabId };
    host.panes = next;
  }
  // Move focus to the freshly-dropped pane + sync currentConvoId.
  host.focusedPaneIdx = paneIdx;
  if (tabId !== host.currentConvoId) {
    const inMeta = host.conversations.some((c) => c.id === tabId);
    if (inMeta && !host.tabs.get(tabId)) {
      // Same stale-root-cache race as setFocusedPane — re-check post-load.
      const paneId = host.panes[paneIdx].id;
      host.currentConvoId = null;
      void host.loadConversation(tabId, { paneId }).then(() => {
        if (host.activeRoot !== prevRoot) {
          host.workspaceFiles = [];
          host.workspaceBranch = null;
        }
      });
    } else {
      host.currentConvoId = tabId;
    }
  }
  if (host.activeRoot !== prevRoot) {
    host.workspaceFiles = [];
    host.workspaceBranch = null;
  }
  host.persistTabs();
}

/** When a tab closes, scrub it from any pane that pointed at it. Panes
 *  become empty (null); the pane container stays so the user can drop a
 *  different tab in or close the pane manually. */
function scrubTabFromPanes(host: TabsHost, id: string) {
  let changed = false;
  const next = host.panes.map((p) => {
    if (p.tabId === id) { changed = true; return { ...p, tabId: null }; }
    return p;
  });
  if (changed) host.panes = next;
}

// ── Tab lifecycle ──────────────────────────────────────────────────────────

export async function restoreTabs(host: TabsHost) {
  // #181: persistTabs() in finally so a throw mid-restore doesn't leave the
  // disk record diverged from in-memory state.
  try {
    const raw = localStorage.getItem(tabsStorageKey());
    if (!raw) return;
    const parsed = JSON.parse(raw) as {
      openTabs?: unknown;
      activeTabId?: unknown;
      panes?: unknown;
      focusedPaneIdx?: unknown;
      ephemeralTabs?: unknown;
    };
    type EphemeralTab = {
      id: string;
      workspaceRoot: string | null;
      model: ModelSel | null;
      effort: ThinkingEffort | null;
      thinkingOn: boolean | null;
      permissionMode: PermissionMode;
      draft: string;
    };
    const isUuid = (value: unknown): value is string =>
      typeof value === "string"
      && /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value);
    const isAbsoluteRoot = (value: unknown): value is string =>
      typeof value === "string"
      && value.trim().toLowerCase() !== "all"
      && (/^[a-z]:[\\/]/i.test(value) || /^\\\\/.test(value) || value.startsWith("/"));
    const efforts: readonly ThinkingEffort[] = ["none", "low", "smart", "deep", "ultra", "max", "agentic"];
    const ephemeral = new Map<string, EphemeralTab>();
    if (Array.isArray(parsed.ephemeralTabs)) {
      for (const item of parsed.ephemeralTabs) {
        const raw = item as Record<string, unknown>;
        if (!isUuid(raw?.id) || ephemeral.has(raw.id)) continue;
        const root = raw.workspaceRoot == null
          ? null
          : isAbsoluteRoot(raw.workspaceRoot) ? raw.workspaceRoot : undefined;
        if (root === undefined) continue;
        ephemeral.set(raw.id, {
          id: raw.id,
          workspaceRoot: root,
          model: asModelSel(raw.model),
          effort: typeof raw.effort === "string" && (efforts as readonly string[]).includes(raw.effort)
            ? raw.effort as ThinkingEffort
            : null,
          thinkingOn: typeof raw.thinkingOn === "boolean" ? raw.thinkingOn : null,
          permissionMode: asPermissionMode(raw.permissionMode) ?? "bypassPermissions",
          draft: typeof raw.draft === "string" ? raw.draft.slice(0, 200_000) : "",
        });
      }
    }
    const ids = Array.isArray(parsed.openTabs)
      ? parsed.openTabs.filter((s): s is string => typeof s === "string")
      : [];
    const existing = new Set(host.conversations.map((c) => c.id));
    const valid = ids.filter((id) => existing.has(id) || ephemeral.has(id));
    host.openTabs = valid;
    for (const id of valid) {
      if (existing.has(id)) continue;
      const saved = ephemeral.get(id);
      if (!saved) continue;
      const tab = host.ensureTab(id, id) as {
        workspaceRoot: string | null;
        modelOverride: ModelSel | null;
        effortOverride: ThinkingEffort | null;
        thinkingOverride: boolean | null;
        permissionMode: PermissionMode;
        draft: string;
      };
      tab.workspaceRoot = saved.workspaceRoot;
      tab.modelOverride = saved.model;
      tab.effortOverride = saved.effort;
      tab.thinkingOverride = saved.thinkingOn;
      tab.permissionMode = saved.permissionMode;
      tab.draft = saved.draft;
    }
    const active = typeof parsed.activeTabId === "string" ? parsed.activeTabId : null;
    // Restore split state — N-pane shape. Accepts length 1..MAX_PANES.
    // Stale tab refs are pruned to null (pane survives, empty). Legacy
    // null/missing keeps single-pane default.
    if (Array.isArray(parsed.panes) && parsed.panes.length >= 1 && parsed.panes.length <= MAX_PANES) {
      // Dedup across panes: a poisoned pre-fix record could persist the same
      // tab in two panes — rendering that duplicate key crashes the whole
      // chat surface on EVERY load until storage is cleared by hand. Later
      // duplicates hydrate empty so old records self-heal here.
      const seen = new Set<string>();
      const seenPaneIds = new Set<string>();
      const norm = (p: unknown): PaneState => {
        const raw = p as { id?: unknown; tabId?: unknown };
        const restoredPaneId = typeof raw?.id === "string" && raw.id.trim() && !seenPaneIds.has(raw.id)
          ? raw.id
          : undefined;
        const pane = createPaneState(null, restoredPaneId);
        seenPaneIds.add(pane.id);
        const tabId = raw?.tabId;
        if (typeof tabId !== "string" || !valid.includes(tabId) || seen.has(tabId)) return pane;
        seen.add(tabId);
        pane.tabId = tabId;
        return pane;
      };
      const restored = parsed.panes.map(norm);
      // Keep at least one pane; if all restored panes are empty and we're
      // single-length, that's fine — assignFocusedPane will fill on next open.
      host.panes = restored.length > 0 ? restored : [createPaneState()];
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
      // If the focused pane's tab was pruned (stale/deleted convo), point it at
      // the winner so the visible focused pane and currentConvoId agree — else
      // the pane renders empty while a different convo is "current". When the
      // winner already renders in a sibling pane (e.g. its duplicate was just
      // deduped above), move focus there instead of re-minting the duplicate.
      const fp = host.panes[host.focusedPaneIdx];
      if (fp && fp.tabId !== winner) {
        const sib = host.panes.findIndex((p, i) => i !== host.focusedPaneIdx && p.tabId === winner);
        if (sib !== -1) host.focusedPaneIdx = sib;
        else fp.tabId = winner;
      }
      try {
        if (existing.has(winner)) {
          await host.loadConversation(winner, { paneId: host.panes[host.focusedPaneIdx]?.id });
        } else {
          // Ephemeral tabs are already hydrated above; activate without asking
          // the conversation backend for a file that intentionally does not exist.
          host.currentConvoId = winner;
        }
      } catch (e) {
        // A transient load failure must NOT fall through to the outer catch —
        // that resets openTabs=[] and the finally persists it, permanently wiping
        // a valid tab list. The parsed tabs are already restored; keep them.
        console.warn("restoreTabs: loadConversation failed", e);
      }
      // loadConversation swallows its own failures (toast + dropTab) — the
      // catch above never sees them. If the TabState didn't materialize, scrub
      // the pane pointer + openTabs entry too, else the pane renders dead
      // chrome (tabId set, no tab) that only a restart clears.
      if (!host.tabs.get(winner)) {
        for (const p of host.panes) if (p.tabId === winner) p.tabId = null;
        host.openTabs = host.openTabs.filter((t) => t !== winner);
      }
    }
  } catch (e) {
    console.warn("restoreTabs failed", e);
    host.openTabs = [];
    host.panes = [createPaneState()];
  } finally {
    host.persistTabs();
  }
}

/** Open a saved convo as a tab. Push to openTabs if not already there;
 *  activate + load from disk. Unsaved new-tab ids (minted by newTab() but
 *  no send yet → no disk record) drop into a fresh in-memory state instead
 *  of disk-load. Streams are per-tab — switching never stops a turn; a live
 *  TabState (possibly mid-stream in the bg) is a pure pointer switch. */
export async function openTab(host: TabsHost, id: string) {
  if (!host.openTabs.includes(id)) {
    host.openTabs = [...host.openTabs, id];
  }
  const alreadyVisible = host.panes.findIndex((p) => p.tabId === id);
  if (alreadyVisible !== -1 && alreadyVisible !== host.focusedPaneIdx) {
    setFocusedPane(host, alreadyVisible);
  }
  // Already-open fast path ONLY when the TabState actually exists: after a
  // failed loadConversation drops the half-built tab, a restored/stale
  // currentConvoId still pointing here would otherwise turn every retry click
  // into a silent no-op (the post-force-close "can't reopen my chat" wedge).
  if (host.currentConvoId === id && host.tabs.get(id)) {
    host.persistTabs();
    return;
  }
  host.telemetry.event("tab.switch", { from: host.currentConvoId, to: id });
  if (host.messages.length > 0 && host.currentConvoId) {
    host.scheduleSave(true);
  }
  // Stash outgoing tab's composer + attachments before any state change.
  const inMeta = host.conversations.some((c) => c.id === id);
  // Bind the selection to the pane NOW, before any disk await. The stable pane
  // id is the async correlation key; a later focus move cannot retarget it.
  assignFocusedPane(host, id);
  const targetPane = host.panes.find((p) => p.tabId === id);
  if (host.tabs.get(id)) {
    // Live TabState (possibly streaming in the bg) — its messages/queue ride
    // on the TabState; disk-reloading here would clobber the in-flight turn.
    host.currentConvoId = id;
    host.lastNotice = null;
  } else if (inMeta) {
    host.currentConvoId = null;
    const loaded = await host.loadConversation(id, { paneId: targetPane?.id });
    if (!loaded) {
      if (targetPane) {
        host.panes = host.panes.map((p) =>
          p.id === targetPane.id && p.tabId === id ? { ...p, tabId: null } : p,
        );
      }
      host.openTabs = host.openTabs.filter((tabId) => tabId !== id);
    }
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
  const closingTab = host.tabs.get(id) as
    | { queue?: { id: string; text: string }[]; streaming?: boolean }
    | undefined;
  if (closingTab?.queue?.length) {
    notify.warn(`${closingTab.queue.length} queued message(s) discarded`, { detail: "The tab was closed mid-queue." });
    // Clear the queue BEFORE stop()'s synchronous onTurnComplete->drainQueue
    // can peek it — else a queued send races the still-in-flight stop() and
    // fires into a tab that's about to be dropped (orphaned turn, silently
    // discarded once tabByCliSession can no longer find it).
    closingTab.queue = [];
  }
  host.pruneTabUi(id);
  // Stop the CLI subprocess for the CLOSING tab — `host.streaming` reads the
  // ACTIVE tab, so a streaming background tab would leak its subprocess (burning
  // tokens, events silently dropped once its TabState is gone). Mirror the
  // tab-targeted stop in closeOtherTabs/closeTabsToRight.
  if (closingTab?.streaming) await host.stop(id);
  // Scrub the pane pointer AFTER stop resolves — doing it before the await blanks
  // the pane (welcome-screen flash) for the whole stop IPC round-trip.
  scrubTabFromPanes(host, id);
  if (wasActive) {
    // Save unsaved tail of the closing tab before switching/clearing.
    if (host.messages.length > 0 && host.convoCreatedAt) {
      host.scheduleSave(true);
    }
  } else {
    // Split-pane: closing a BACKGROUND tab (wasActive=false) skipped the flush
    // above entirely — host.messages reads the ACTIVE tab, so the closing tab's
    // unsaved tail was lost when dropTab retired its TabState. Flush by convoId
    // (scheduleSave guards empty messages internally, persistence.ts) so a
    // bg-tab close persists like closeAllTabs does. MUST precede dropTab —
    // dropTab cancels the tab's saveTimer.
    host.scheduleSave(true, id);
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
      const visibleIdx = host.panes.findIndex((p) => p.tabId === neighbor);
      if (visibleIdx !== -1) {
        setFocusedPane(host, visibleIdx);
      } else {
        assignFocusedPane(host, neighbor);
        const paneId = host.panes[host.focusedPaneIdx]?.id;
        const inMeta = host.conversations.some((c) => c.id === neighbor);
        if (inMeta) {
          host.currentConvoId = null;
          await host.loadConversation(neighbor, { paneId });
        } else {
          // #143: ensureTab seeds the fresh tab's cliSessionId to neighbor;
          // don't store-write null afterwards or the setter clobbers it.
          host.ensureTab(neighbor, neighbor);
          host.currentConvoId = neighbor;
          host.queue = [];
          host.lastNotice = null;
        }
      }
    }
  }
  host.persistTabs();
}

/** Open a fresh empty tab. Mints currentConvoId up-front so the tab can
 *  render before the first send; convoCreatedAt stays null so send() still
 *  flags isFirstTurn=true and the CLI gets --session-id, not --resume. */
export async function newTab(host: TabsHost): Promise<string> {
  // Don't stop the previous tab's stream — newTab leaves background tabs
  // streaming. Save unsaved tail of the previous tab before swapping.
  if (host.messages.length > 0 && host.currentConvoId) {
    host.scheduleSave(true);
  }
  // Snapshot outgoing tab's composer state before we mint the new one.
  // Inherit the focused tab's folder so a new chat opens in the same project
  // by default (a concrete snapshot — later folder switches elsewhere can't
  // leak in). When there is no focused tab, snapshot the global new-chat
  // default; an existing tab's null remains an explicit local/no-project scope.
  const current = host.tabs.get(host.currentConvoId ?? "") as
    | { workspaceRoot?: string | null }
    | undefined;
  const inheritRoot = current ? current.workspaceRoot ?? null : host.workspaceCurrent;
  const id = crypto.randomUUID();
  host.openTabs = [...host.openTabs, id];
  // Fresh TabState — empty messages, no streaming. cliSessionId defaults
  // to the convoId; first send() finalizes if needed.
  host.ensureTab(id, id);
  const minted = host.tabs.get(id) as { workspaceRoot: string | null; modelOverride: ModelSel | null };
  minted.workspaceRoot = inheritRoot;
  // A rooted tab pins its model to the FOLDER's choice — never the shared
  // global default, which tracks whatever pane is globally focused and would
  // leak a sibling pane's model into this one (split-pane fix, cont.339).
  if (inheritRoot) minted.modelOverride = loadModel(inheritRoot);
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
  host.composerTextAttachments = [];
  assignFocusedPane(host, id);
  host.persistTabs();
  return id;
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
  // Preserve THIS pane's folder AND model across the clear — clearing a pane
  // must keep its own project dir + model, never inherit another pane's.
  const oldTab = host.tabs.get(oldId) as
    | { workspaceRoot?: string | null; modelOverride?: ModelSel | null }
    | undefined;
  const keepRoot = oldTab?.workspaceRoot ?? null;
  const keepModel = oldTab?.modelOverride ?? (keepRoot ? loadModel(keepRoot) : null);
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
  host.panes = host.panes.map((p) => (p.tabId === oldId ? { ...p, tabId: newId } : p));
  // Fresh empty TabState; cliSessionId seeded to newId. #143: don't write the
  // per-tab fields via store setters afterwards or ensureTab's seed is lost.
  host.ensureTab(newId, newId);
  const cleared = host.tabs.get(newId) as { workspaceRoot: string | null; modelOverride: ModelSel | null };
  cleared.workspaceRoot = keepRoot;
  cleared.modelOverride = keepModel;
  host.telemetry.event("tab.clear", { from: oldId, to: newId });
  host.currentConvoId = newId;
  host.queue = [];
  notify.info("Conversation cleared", { detail: "Previous chat saved to History." });
  host.composerDraft = "";
  host.composerAttachments = [];
  host.composerTextAttachments = [];
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
    const t = host.tabs.get(id) as { streaming?: boolean; queue?: unknown[] } | undefined;
    // Clear the queue before stop() so its synchronous drainQueue can't fire a
    // send into a tab about to be dropped (same race as closeTab, tabs.ts:402-408).
    if (t?.queue?.length) t.queue = [];
    if (t?.streaming) await host.stop(id);
    host.dropTab(id);
    host.pruneTabUi(id);
    scrubTabFromPanes(host, id);
  }
  host.openTabs = [keepId];
  if (host.currentConvoId !== keepId) {
    await openTab(host, keepId);
  }
  host.persistTabs();
}

/** Wipe all open tabs and drop into the empty-tabs state. Flushes the
 *  current convo if it has messages so nothing's lost; closes streams. */
export async function closeAllTabs(host: TabsHost) {
  // Stop EVERY streaming tab, not just the active one (host.streaming reads
  // activeTab only) — a backgrounded stream would otherwise have its TabState
  // dropped while its CLI subprocess keeps running + burning tokens, its events
  // silently discarded. Mirrors closeOtherTabs/closeTabsToRight.
  for (const id of host.openTabs) {
    const t = host.tabs.get(id) as { streaming?: boolean; queue?: unknown[] } | undefined;
    // Clear the queue before stop() so its synchronous drainQueue can't fire a
    // send into a tab about to be dropped (same race as closeTab, tabs.ts:402-408).
    if (t?.queue?.length) t.queue = [];
    if (t?.streaming) await host.stop(id);
  }
  // RR10: flush EVERY tab with unsaved messages, not just the active one — a
  // background tab whose turn just completed has a pending 700ms debounce that
  // would fire against an already-dropped TabState (silent data loss).
  for (const [convoId, t] of host.tabs) {
    const tab = t as { messages?: unknown[] };
    if ((tab.messages?.length ?? 0) > 0) host.scheduleSave(true, convoId);
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
    const t = host.tabs.get(id) as { streaming?: boolean; queue?: unknown[] } | undefined;
    // Clear the queue before stop() so its synchronous drainQueue can't fire a
    // send into a tab about to be dropped (same race as closeTab, tabs.ts:402-408).
    if (t?.queue?.length) t.queue = [];
    if (t?.streaming) await host.stop(id);
    host.dropTab(id);
    host.pruneTabUi(id);
    scrubTabFromPanes(host, id);
  }
  host.openTabs = kept;
  if (removedActive) {
    await openTab(host, anchorId);
  }
  host.persistTabs();
}
