// v0.4.10 workspace shell — replaces right-pane.svelte.ts. ActivityBar swaps
// the main pane via workspace.setActive(); no sidecar widths to track.
// Chat is the default workspace.

export type WorkspaceId =
  | "home" | "chat" | "settings" | "local-llm";

export const WORKSPACE_IDS: readonly WorkspaceId[] = [
  "home", "chat", "settings", "local-llm",
] as const;

const ACTIVE_KEY = "rift.ui.workspace.v1";
const ORDER_KEY = "rift.ui.workspace-order.v1";

// Legacy keys swept on first launch under the new shell. Sweeping is
// unconditional — even if the legacy key was missing, removeItem is a no-op,
// so this also covers fresh installs.
const LEGACY_KEYS_TO_SWEEP = [
  "rift.ui.v03-shell.v1",
  "rift.ui.right-pane.v1",
  "rift.ui.right-pane-w.v1",
  "rift.ui.activitybar-order.v1",
  // Already-handled legacy keys from earlier dock era — keep sweeping for
  // users who skipped intermediate releases.
  "rift.ui.panels.v1",
  "rift.ui.dock-w.v1",
  "rift.ui.dock-split.v1",
  "rift.ui.maximized.v1",
  "rift.ui.preset-picked.v1",
  "rift.ui.dock-accordion.v1",
  // Terminal workspace removed 2026-05-25 — sweep its orphaned keys.
  "rift.terminal.open",
  "rift.terminal.height",
  "rift.terminal.defaultShell",
  "rift.terminal.fontSize",
  "rift.terminal.fontFamily",
  "rift.terminal.fontFamilyCustom",
  "rift.terminal.scrollback",
  "rift.terminal.cursorStyle",
  "rift.terminal.cursorBlink",
  "rift.terminal.bellStyle",
  "rift.terminal.copyOnSelect",
  "rift.terminal.rightClickPaste",
  "rift.terminal.themePreset",
  "rift.terminal.autoLaunch",
  "rift.terminal.savedTabs",
  "rift.terminal.activeTabIdx",
] as const;

const DISABLED: ReadonlySet<WorkspaceId> = new Set();
const DEFAULT_ORDER: readonly WorkspaceId[] = WORKSPACE_IDS;

function isWorkspaceId(v: unknown): v is WorkspaceId {
  return typeof v === "string" && (WORKSPACE_IDS as readonly string[]).includes(v);
}

class WorkspaceState {
  activeId = $state<WorkspaceId>("chat");
  order = $state<WorkspaceId[]>([...DEFAULT_ORDER]);
  /** Lazy-mount latch — once a workspace has been active, its component stays
   *  mounted (hidden via [hidden]) so scroll/terminal/etc. state survives
   *  workspace switches. Same pattern v0.4.1 RightPane used. */
  everOpened = $state<Set<WorkspaceId>>(new Set<WorkspaceId>());

  init() {
    if (typeof window === "undefined") return;
    this.migrateLegacy();

    const stored = localStorage.getItem(ACTIVE_KEY);
    if (stored && isWorkspaceId(stored) && !DISABLED.has(stored)) {
      this.activeId = stored;
    } else {
      this.activeId = "chat";
      localStorage.setItem(ACTIVE_KEY, "chat");
    }
    this.everOpened = new Set([this.activeId]);

    try {
      const raw = localStorage.getItem(ORDER_KEY);
      if (raw) {
        const arr = JSON.parse(raw) as unknown;
        if (Array.isArray(arr)) {
          const valid = arr.filter(isWorkspaceId);
          const seen = new Set(valid);
          // Backfill any ids missing from stored order — covers new workspace
          // additions in later releases for users w/ persisted older order.
          // Insert at the new id's DEFAULT_ORDER-relative slot (not the end) so
          // a mid-list addition like "harness" lands before "settings", keeping
          // positional Ctrl+N switching aligned with the kbd hints.
          for (const id of DEFAULT_ORDER) {
            if (seen.has(id)) continue;
            const di = DEFAULT_ORDER.indexOf(id);
            let at = valid.length;
            for (let i = 0; i < valid.length; i++) {
              if (DEFAULT_ORDER.indexOf(valid[i]) > di) { at = i; break; }
            }
            valid.splice(at, 0, id);
            seen.add(id);
          }
          this.order = valid;
        }
      }
    } catch (e) { console.warn('[workspace] ORDER_KEY parse failed, using default:', e); }
  }

  /** One-time migration on first launch under the new shell:
   *  - Seed activeId from the legacy right-pane key if user had a non-disabled
   *    panel open. Disabled stubs (agents/attachments) → default to chat.
   *  - Carry legacy activitybar-order forward so user reorders survive.
   *  - Sweep every legacy key (idempotent — safe to re-run). */
  private migrateLegacy() {
    const legacyActive = localStorage.getItem("rift.ui.right-pane.v1");
    if (legacyActive && isWorkspaceId(legacyActive) && !DISABLED.has(legacyActive)
        && !localStorage.getItem(ACTIVE_KEY)) {
      localStorage.setItem(ACTIVE_KEY, legacyActive);
    }
    const legacyOrder = localStorage.getItem("rift.ui.activitybar-order.v1");
    if (legacyOrder && !localStorage.getItem(ORDER_KEY)) {
      try {
        const arr = JSON.parse(legacyOrder) as unknown;
        if (Array.isArray(arr)) {
          const seeded: WorkspaceId[] = ["chat"];
          for (const id of arr) if (isWorkspaceId(id) && !seeded.includes(id)) seeded.push(id);
          for (const id of DEFAULT_ORDER) if (!seeded.includes(id)) seeded.push(id);
          localStorage.setItem(ORDER_KEY, JSON.stringify(seeded));
        }
      } catch { /* unparseable → leave new key unset, init() will use default */ }
    }
    for (const k of LEGACY_KEYS_TO_SWEEP) localStorage.removeItem(k);
  }

  setActive(id: WorkspaceId) {
    if (DISABLED.has(id)) return;
    this.activeId = id;
    if (!this.everOpened.has(id)) {
      const s = new Set(this.everOpened); s.add(id); this.everOpened = s;
    }
    localStorage.setItem(ACTIVE_KEY, id);
  }

  reorder(from: number, to: number) {
    if (from === to || from < 0 || from >= this.order.length) return;
    const next = [...this.order];
    const [moved] = next.splice(from, 1);
    next.splice(Math.max(0, Math.min(next.length, to)), 0, moved);
    this.order = next;
    localStorage.setItem(ORDER_KEY, JSON.stringify(next));
  }

  resetOrder() {
    this.order = [...DEFAULT_ORDER];
    localStorage.removeItem(ORDER_KEY);
  }
}

export const workspace = new WorkspaceState();
