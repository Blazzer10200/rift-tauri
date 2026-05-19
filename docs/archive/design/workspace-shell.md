# Workspace Shell — v0.4.1 finalization

> **Status:** Plan only — not yet implemented. Owner session: next session executes; original planning session verifies after.
> **Created:** 2026-05-18 (S96) from S95 handoff "polish UI before retiring v0.2" ask.
> **Decision:** Model B — workspace-swap, no permanent sidecar. See §1.

---

## 0. TL;DR for the executing session

Replace the dual-shell (v0.2 tab-rail / v0.4.1 chat-first sidecar) with a single shell where the **40px right-side activity bar is the navigator** and **each item swaps the entire main pane** (a "workspace"). Chat is one workspace among many — the default-selected one — but not always-main. Adds Conflicts + Diagnostics + Settings entry points that don't exist in v0.4.1 today. Deletes ~600 LOC of v0.2 path + 43 `useV03Shell` branches across 14 files.

**Net effect:** users see one icon rail, click an icon, that page fills the screen. Chat tab-strip mounts only inside the Chat workspace. Settings gear sits at the bottom of the rail. No more cramped right-pane. No more dual-shell.

**Verification is fully autonomous** via [`scripts/cdp/smoke-v04-10.sh`](../../scripts/cdp/smoke-v04-10.sh) — 70+ DOM-level assertions + grep-cleanliness + file-deletion checks. Run after every phase. Exits non-zero on any FAIL. No human screenshotting required.

---

## 0.1 Pre-flight — environment setup (run FIRST)

The implementing session must do all of this before touching code:

```bash
# 1. Confirm Rift dev is running. If not, start it.
tasklist 2>/dev/null | grep -i rift-tauri  # expect a hit
# If empty, start: cmd //c "start \"\" scripts/run-dev.bat" then wait ~30s for vite + tauri

# 2. Confirm CDP wrapper is up on 9223.
bash scripts/cdp/c.sh health  # expect {"ok":true,"target":"http://localhost:1420/",...}
# If 'Failed to connect': `npm run cdp:serve &` in repo root, sleep 2, re-check.

# 3. Capture baseline state to .tmp/baseline-shell.json (so verifying session can diff).
mkdir -p .tmp
bash scripts/cdp/c.sh eval "({
  useV03: localStorage.getItem('rift.ui.v03-shell.v1'),
  activePane: localStorage.getItem('rift.ui.right-pane.v1'),
  paneOrder: localStorage.getItem('rift.ui.activitybar-order.v1'),
  abButtons: Array.from(document.querySelectorAll('nav.activitybar .ab-btn')).map(b => b.title),
  bodyGridCols: getComputedStyle(document.querySelector('.body')).gridTemplateColumns,
  rightPaneMounted: !!document.querySelector('aside.right-pane'),
  shell: 'v0.4.1-pre-workspace'
})" > .tmp/baseline-shell.json
cat .tmp/baseline-shell.json
```

**Expected baseline** (captured 2026-05-18 S96 by the planning session on Blazzer's machine):

```json
{"value":{
  "useV03":"1",
  "activePane":null,
  "paneOrder":null,
  "abButtons":["Files · Ctrl+1","Sync · Ctrl+2","Activity · Ctrl+3","Terminal · Ctrl+4","Agents · Ctrl+5","Attachments · Ctrl+6","History · Ctrl+7"],
  "bodyGridCols":"1992px 0px 40px",
  "rightPaneMounted":false,
  "shell":"v0.4.1-pre-workspace"
}}
```

If the baseline differs significantly (e.g. `useV03=null`, or `abButtons` length is not 7), STOP and report — the codebase has drifted from what this plan was written against.

**Dev-server lifecycle constraint:** Tauri dev watches Rust files. The implementing session edits frontend (.svelte/.ts) which HMR-reloads cleanly. **DO NOT run `cargo check` while dev is alive** ([CLAUDE.md project rule](../../CLAUDE.md)). If a Rust edit becomes necessary, kill dev first (`taskkill /IM rift-tauri.exe /F`), run `cargo check`, then relaunch dev.

---

## 1. Decision rationale

**Model B (workspace-swap, peer panes)** is the 2025-2026 consensus for chat-first apps with heavy secondary surfaces:

- **Cursor** pivoted from forced-chat-main to 4 switchable layouts (2.3) after user backlash — [cursor.com/changelog/2-3](https://cursor.com/changelog/2-3), [forum megathread](https://forum.cursor.com/t/megathread-cursor-layout-and-ui-feedback/146790).
- **Warp** uses `Cmd+Enter` / `Esc` to swap between terminal mode and agent mode — full main-pane swap, zero permanent hierarchy. [warp docs](https://docs.warp.dev/agent-platform/local-agents/interacting-with-agents/terminal-and-agent-modes/).
- **Claude Code Desktop (Apr 2026)** shipped rearrangeable peer panes as the headline feature. [VentureBeat](https://venturebeat.com/orchestration/we-tested-anthropics-redesigned-claude-code-desktop-app-and-routines-heres-what-enterprises-should-know).
- Only **Zed** and **Continue.dev** stay on Model A: Zed sidesteps via inline diffs (no separate diff surface needs space); Continue is platform-constrained by VS Code's webview API. Neither chose Model A freely for a multi-surface app.

For Rift specifically: SFTP file browser, conflicts UI, sync diagnostics, and terminal are full-task surfaces, not widgets. Stuffing them in a 320-1200px sidecar suffocates them. Workspace-swap solves this without losing the chat-first feel (chat is still the default workspace).

**Settings placement:** gear at the bottom of the activity bar — VS Code + Linear pattern. Palette-only (`Ctrl+,`) is a documented discoverability antipattern. [Linear sidebar refresh Dec 2024](https://linear.app/changelog/2024-12-18-personalized-sidebar), [Smashing Magazine: Hidden vs Disabled](https://www.smashingmagazine.com/2024/05/hidden-vs-disabled-ux/).

**Stub panels (Agents, Attachments):** disable + tooltip, don't hide. Preserves discoverability without false-advertising clickability. [UX Tigers: Inactive controls](https://www.uxtigers.com/post/inactive-buttons).

---

## 2. Target architecture

```
┌────────────────────────────────────────────────────────────┐
│ Titlebar (44px) — brand · server picker · cmdk · winctl   │
├────────────────────────────────────────────────────────────┤
│ ChatTabsBar (34px) — mounts ONLY when active = "chat"     │
├──────────────────────────────────────────────┬─────────────┤
│                                              │             │
│                                              │  Activity   │
│                                              │  bar 40px   │
│                                              │  (right)    │
│       Active workspace                       │             │
│       (fills entire main pane)               │  ┌────────┐ │
│                                              │  │ chat   │ │ ← top group
│                                              │  │ sync   │ │   (workspaces)
│                                              │  │ files  │ │
│                                              │  │ conf.  │ │
│                                              │  │ diag.  │ │
│                                              │  │ term.  │ │
│                                              │  │ activ. │ │
│                                              │  │ hist.  │ │
│                                              │  │ agents │ │ (disabled)
│                                              │  │ attach │ │ (disabled)
│                                              │  ├────────┤ │   ← divider
│                                              │  │ gear   │ │ ← bottom group
│                                              │  └────────┘ │   (settings)
├──────────────────────────────────────────────┴─────────────┤
│ StatusBar (22px)                                           │
└────────────────────────────────────────────────────────────┘
```

**Key invariants:**
- Exactly one workspace active at all times. There is no "closed" state — clicking the active workspace does nothing (no toggle-off).
- `chat` is the default selection on first launch and on storage-missing.
- Settings = gear at bottom of rail → opens **the existing slide-over modal** (not a workspace). Keeping it modal preserves "Settings is transient" mental model and matches Linear.
- Activity bar items in top group are user-reorderable (drag-to-reorder, already wired). Bottom group (gear) is fixed.
- Disabled workspaces (Agents, Attachments today) render as dim icon w/ tooltip "Coming soon — Phase B". Click is a no-op.
- Workspaces lazy-mount on first visit, stay mounted thereafter (preserves scroll, terminal session, etc.). Same `everOpened` latch pattern that exists today.

---

## 3. Workspace registry

`src/lib/components/workspaces/index.ts` (new file — replaces `src/lib/components/right-pane/index.ts`):

| id | Component (full-pane) | Icon | Default kbd | Badge source | Tone | Phase |
|---|---|---|---|---|---|---|
| `chat` | `AssistantPage` | `MessageSquare` | Ctrl+1 | — | — | shipped |
| `sync` | `SyncPage` | `RefreshCcw` | Ctrl+2 | `connection.pendingCount` (if exists; else none) | info | shipped |
| `files` | `TwoPane` | `FolderOpen` | Ctrl+3 | — | — | shipped |
| `conflicts` | `ConflictsPage` | `AlertTriangle` | Ctrl+4 | `connection.conflictCount` | danger | shipped — currently unreachable in v0.4.1 |
| `diagnostics` | `Diagnostics` | `Stethoscope` (or `Activity` variant) | Ctrl+5 | error/warn count from diagBus (verify exists) | danger/warn | shipped — currently unreachable in v0.4.1 |
| `terminal` | `TerminalPanel` (full-pane mode) | `Terminal` | Ctrl+6 | — | — | shipped |
| `activity` | `ActivityFeed` | `Activity` | Ctrl+7 | — | — | shipped |
| `history` | `HistoryDrawer` (sheds overlay chrome in workspace mode) | `History` | Ctrl+8 | `assistant.conversations.length` | info | shipped — keep current pip |
| `agents` | `DisabledWorkspace` placeholder | `Bot` | — | — | — | **disabled, tooltip "Coming soon"** |
| `attachments` | `DisabledWorkspace` placeholder | `Paperclip` | — | — | — | **disabled, tooltip "Coming soon"** |

**Default order (top group):** `chat, sync, files, conflicts, diagnostics, terminal, activity, history, agents, attachments`. Persists to `rift.ui.workspace-order.v1` after first user drag.

**Bottom group:** `[settings]` (gear icon, fixed position, not reorderable). Opens existing Settings slide-over modal via `gotoSettings("appearance")`.

**Badge rule:** never on disabled (`agents`, `attachments`). Drop the existing `attachments` count pip wiring entirely if it ever existed — it was on a stub.

---

## 4. State migration

### 4.1 New file: `src/lib/state/workspace.svelte.ts`

Replaces `right-pane.svelte.ts`. **Full file — copy-paste ready** (no skeleton placeholders):

```ts
// v0.4.10 workspace shell — replaces right-pane.svelte.ts. ActivityBar swaps
// the main pane via workspace.setActive(); no sidecar widths to track.
// Chat is the default workspace. Agents + Attachments are DISABLED stubs —
// setActive() guards against them. Stub entries stay in the registry so the
// activity bar can render them as "Coming soon" tiles.

export type WorkspaceId =
  | "chat" | "sync" | "files" | "conflicts" | "diagnostics"
  | "terminal" | "activity" | "history" | "agents" | "attachments";

export const WORKSPACE_IDS: readonly WorkspaceId[] = [
  "chat", "sync", "files", "conflicts", "diagnostics",
  "terminal", "activity", "history", "agents", "attachments",
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
] as const;

const DISABLED: ReadonlySet<WorkspaceId> = new Set(["agents", "attachments"]);
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
  everOpened = $state<Set<WorkspaceId>>(new Set(["chat"]));

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
          for (const id of DEFAULT_ORDER) if (!seen.has(id)) valid.push(id);
          this.order = valid;
        }
      }
    } catch { /* parse fail → default order */ }
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
      // Filter out disabled-stub ids that may have been in legacy order, then
      // backfill so the new "chat" workspace appears (legacy didn't have it).
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
    if (DISABLED.has(id)) return; // no-op for stubs — click is dead
    this.activeId = id;
    if (!this.everOpened.has(id)) {
      this.everOpened = new Set([...this.everOpened, id]);
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
```

**No `toggle()`, no `close()`, no width-resize machinery.** Workspaces always fill the main pane.

**`isWorkspaceId` is internal** (not exported) — registry consumers use the `WorkspaceId` type directly.

**Stethoscope icon for Diagnostics is confirmed present** in `lucide-svelte@1.0.1` at `node_modules/lucide-svelte/dist/icons/stethoscope.svelte`. Verified 2026-05-18.

### 4.2 Delete `src/lib/state/right-pane.svelte.ts` after migration.
### 4.3 Delete `src/lib/state/panel-types.ts` (replaced by `WorkspaceId` in `workspace.svelte.ts`).

### 4.4 Update `src/lib/state/ui-prefs.svelte.ts`

Delete the `useV03Shell` field, storage key (`V03_SHELL_KEY`), and `setUseV03Shell()` method entirely. The toggle is gone — single shell only.

Storage key cleanup: on first init in new shell, `localStorage.removeItem("rift.ui.v03-shell.v1")` to sweep. Add to `workspace.migrateLegacy()` or `uiPrefs.init()`.

---

## 5. Component changes

### 5.1 NEW: `src/lib/components/shell/WorkspaceShell.svelte`

Replaces `RightPane.svelte`. Renders the active workspace full-pane with lazy-mount + keep-alive. **Full file — copy-paste ready:**

```svelte
<script lang="ts">
  import { WORKSPACES } from "../workspaces";
  import { workspace } from "$lib/state/workspace.svelte";
  import { WORKSPACE_IDS } from "$lib/state/workspace.svelte";
</script>

<div class="ws-shell">
  {#each WORKSPACE_IDS as id (id)}
    {#if workspace.everOpened.has(id)}
      {@const def = WORKSPACES[id]}
      <div class="ws-page" data-workspace={id} hidden={workspace.activeId !== id}>
        <def.component />
      </div>
    {/if}
  {/each}
</div>

<style>
  .ws-shell {
    flex: 1;
    min-height: 0; min-width: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }
  .ws-page {
    flex: 1;
    min-height: 0; min-width: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
  }
  .ws-page[hidden] { display: none; }
</style>
```

**Iteration is over `WORKSPACE_IDS`, not `Object.keys(WORKSPACES)`** — `WORKSPACE_IDS` is a stable `const` array; `Object.keys` insertion order is technically guaranteed in modern JS but the typed array is the safer contract.

**Disabled workspaces (Agents/Attachments) never enter `everOpened`** because `setActive` no-ops on them. They render in `WORKSPACES` registry but never get mounted in `WorkspaceShell`. The ActivityBar still draws their icons (disabled state) — that's the only place they appear in DOM.

### 5.2 NEW: `src/lib/components/workspaces/DisabledWorkspace.svelte`

A `props: { title, icon, message }` placeholder rendered for Agents/Attachments. Same visual as the current stubs but uses the full workspace pane (not the 200-line panel area).

### 5.3 NEW: `src/lib/components/workspaces/index.ts`

Workspace registry (see §3 table). Mirrors current `right-pane/index.ts` shape:

```ts
import type { WorkspaceId } from "$lib/state/workspace.svelte";
import { MessageSquare, RefreshCcw, FolderOpen, AlertTriangle, Stethoscope,
         Terminal, Activity, History, Bot, Paperclip } from "lucide-svelte";
import AssistantPage from "../assistant/AssistantPage.svelte";
import SyncPage from "../sync/SyncPage.svelte";
import TwoPane from "../browser/TwoPane.svelte";
import ConflictsPage from "../conflicts/ConflictsPage.svelte";
import Diagnostics from "../diagnostics/Diagnostics.svelte";
import TerminalPanel from "../terminal/TerminalPanel.svelte";
import ActivityFeed from "../activity/ActivityFeed.svelte";
import HistoryDrawer from "../assistant/HistoryDrawer.svelte";
import DisabledWorkspace from "./DisabledWorkspace.svelte";
import { connection } from "$lib/state/connection.svelte";
import { assistant } from "$lib/state/assistant.svelte";

export type WorkspaceIcon = typeof MessageSquare;
export type WorkspaceDef = {
  component: typeof AssistantPage; // type widen as needed
  title: string;
  icon: WorkspaceIcon;
  kbd: string;
  disabled?: boolean;
  getCount?: () => number;
  getTone?: "warn" | "danger" | "info";
};

export const WORKSPACES: Record<WorkspaceId, WorkspaceDef> = {
  chat:        { component: AssistantPage,   title: "Chat",        icon: MessageSquare,  kbd: "1" },
  sync:        { component: SyncPage,        title: "Sync",        icon: RefreshCcw,     kbd: "2" },
  files:       { component: TwoPane,         title: "Files",       icon: FolderOpen,     kbd: "3" },
  conflicts:   { component: ConflictsPage,   title: "Conflicts",   icon: AlertTriangle,  kbd: "4",
                 getCount: () => connection.conflictCount, getTone: "danger" },
  diagnostics: { component: Diagnostics,     title: "Diagnostics", icon: Stethoscope,    kbd: "5" /* getCount TBD — see §10 */ },
  terminal:    { component: TerminalPanel,   title: "Terminal",    icon: Terminal,       kbd: "6" },
  activity:    { component: ActivityFeed,    title: "Activity",    icon: Activity,       kbd: "7" },
  history:     { component: HistoryDrawer,   title: "History",     icon: History,        kbd: "8",
                 getCount: () => assistant.conversations.length, getTone: "info" },
  agents:      { component: DisabledWorkspace, title: "Agents",    icon: Bot,            kbd: "",  disabled: true },
  attachments: { component: DisabledWorkspace, title: "Attachments", icon: Paperclip,    kbd: "",  disabled: true },
};
```

### 5.4 REWRITE: `src/lib/components/shell/ActivityBar.svelte`

Two groups separated by a spacer + divider. Same drag-to-reorder for top group. Bottom group is fixed:

```svelte
<script lang="ts">
  import { Settings as SettingsIcon } from "lucide-svelte";
  import { WORKSPACES } from "../workspaces";
  import { workspace } from "$lib/state/workspace.svelte";

  let { onOpenSettings }: { onOpenSettings: () => void } = $props();

  // drag-reorder code — copy from existing ActivityBar.svelte, swap rightPane → workspace
</script>

<nav class="activitybar" aria-label="Workspaces">
  <div class="ab-top">
    {#each workspace.order as id, idx (id)}
      {@const def = WORKSPACES[id]}
      {@const isActive = workspace.activeId === id}
      {@const count = def.getCount?.() ?? 0}
      {@const tone = def.getTone ?? "neutral"}
      <button
        class="ab-btn"
        type="button"
        data-active={isActive}
        data-disabled={def.disabled}
        disabled={def.disabled}
        title={def.disabled ? `${def.title} — Coming soon` : `${def.title} · Ctrl+${idx + 1}`}
        aria-label="{def.title} {isActive ? '(active)' : ''}"
        aria-pressed={isActive}
        draggable={!def.disabled}
        onclick={() => !def.disabled && workspace.setActive(id)}
        {/* dnd handlers — same as today */}
      >
        <span class="ab-icon"><def.icon size={16}/></span>
        {#if !def.disabled && count > 0}
          <span class="ab-count count-pip" data-tone={tone}>{count > 99 ? "99+" : count}</span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="ab-bottom">
    <div class="ab-divider"></div>
    <button class="ab-btn" type="button" onclick={onOpenSettings} title="Settings · Ctrl+,">
      <span class="ab-icon"><SettingsIcon size={16}/></span>
    </button>
  </div>
</nav>

<style>
  .activitybar { width: 40px; height: 100%; display: flex; flex-direction: column;
                 background: var(--surface); border-left: 1px solid var(--border); user-select: none; }
  .ab-top    { display: flex; flex-direction: column; }
  .ab-bottom { margin-top: auto; display: flex; flex-direction: column; }
  .ab-divider { height: 1px; background: var(--border); margin: 4px 6px; }
  .ab-btn    { /* copy from current — add disabled state */ }
  .ab-btn[data-disabled="true"] { opacity: 0.4; cursor: not-allowed; }
  /* … */
</style>
```

**Active indicator:** the left-edge accent stripe (current `::before` rule) stays but its position changes — accent stripe should be on the LEFT edge of the rail (toward content). Current code already does `left: 0`; the rail being on the right side of the screen means `left: 0` is the content-facing edge. Good — no change.

### 5.5 REWRITE: `src/lib/components/AppShell.svelte`

Delete the v0.2 path entirely. The body becomes:

```svelte
<div class="shell">
  <Titlebar onOpenPalette={...} onAddServer={...} onEditCurrent={...} />

  <div class="middle">
    {#if connection.wireError}<div class="wire-error">…</div>{/if}

    {#if workspace.activeId === "chat"}
      <ChatTabsBar />
    {/if}

    <div class="body">
      <main class="pane">
        <WorkspaceShell />
      </main>
      <ActivityBar onOpenSettings={() => settingsModalOpen = true} />
    </div>
  </div>

  <StatusBar />

  <!-- dialogs unchanged -->
  <AddServer … />
  <Keygen … />
  <Bootstrap … />
  <Confirm … />
  {#if connection.pendingReuploads.length > 0}<Reupload … />{/if}
  <CommandPalette open={paletteOpen} {commands} onClose={…} />
  <UpdateDialog />
  <ActivityToast />

  <!-- settings slide-over — kept exactly as v0.4.1 -->
  {#if settingsModalOpen}
    <div class="slideover-scrim" onclick={closeSettingsModal} role="presentation"></div>
    <aside class="slideover" aria-label="Settings">
      <button class="slideover-close" onclick={closeSettingsModal} title="Close (Esc)"><X size={14}/></button>
      {#key settingsSection}
        <Settings initialSection={settingsSection}
                  onAddServer={…} onEditServer={…} onDeleteServer={…} onLaunchKeygen={…} />
      {/key}
    </aside>
  {/if}
</div>
```

CSS changes:
- `.body` becomes `grid-template-columns: minmax(0, 1fr) 40px` — chat takes all width left of the activity bar. No `--right-pane-w` variable. Drop `data-v04-1="true"` selector.
- Delete the `transition: grid-template-columns 220ms` rule (no width to animate).

**Delete from AppShell:**
- Lines 37-38 (Tab + SettingsSection types — keep `SettingsSection`, drop `Tab`).
- Lines 40-53 (`active`, `settingsSection`, `settingsModalOpen`, `visited` Set, `$effect` watcher — keep `settingsSection` + `settingsModalOpen`, drop the rest).
- Lines 106-113 (`gotoSettings` branching — collapse to `settingsModalOpen = true`).
- Lines 136-145 (`v02Commands` — delete).
- Lines 147-160 (`v03Commands` — keep but rename to `workspaceCommands`; regenerate from `workspace.order` not `rightPane.order`; rename `right-pane-${id}` → `workspace-${id}`).
- Lines 162-164 (`commands` derived — collapse to `[...sharedCommands, ...workspaceCommands]`).
- All `if (uiPrefs.useV03Shell)` / `if (!uiPrefs.useV03Shell)` branches in `onGlobalKey` — keep only the v0.3-gated logic, drop the v0.2 Ctrl+1..6 tab cycle (lines 310-315).
- Lines 466-516 (entire v0.2 body grid w/ visited Set + page-shell wrappers — gone).
- `import TabRail from "./shell/TabRail.svelte"` (line 8) — delete.
- `import TerminalPanel` (line 24) — keep only if needed for type; the workspace registry imports it directly now.

### 5.6 Update keybindings in `onGlobalKey`

Final map:
- `Ctrl+K` → command palette (unchanged).
- `Ctrl+,` → settings modal (unchanged).
- `Ctrl+N` → add server (unchanged).
- `Ctrl+P` → settings → servers section (unchanged for v0.4.1 muscle memory).
- `Ctrl+1..8` → workspace switch (chat/sync/files/conflicts/diagnostics/terminal/activity/history). Maps via `workspace.order[idx]`.
- `Ctrl+0` → switch to Chat workspace (replaces "close right pane" — closing has no meaning anymore).
- `Ctrl+` (backtick) `` → Terminal workspace switch (was `terminal.toggle()` / `rightPane.toggle("terminal")`).
- `Ctrl+Shift+D` → Diagnostics workspace (was "v2 placeholder").
- `Alt+1..9` → chat tab switch — **gate on `workspace.activeId === "chat"`** so it doesn't fire when in another workspace.
- `Ctrl+T` / `Ctrl+W` / `Ctrl+Tab` → new/close/cycle chat tabs — same gate.

### 5.7 ChatTabsBar mount gate

Currently mounts unconditionally under `useV03Shell`. New rule:

```svelte
{#if workspace.activeId === "chat"}
  <ChatTabsBar />
{/if}
```

This means swapping AWAY from Chat hides the tab strip; swapping BACK shows it with all tabs preserved (state is on the assistant store, not the bar).

### 5.8 Files to DELETE outright

| File | LOC | Reason |
|---|---|---|
| `src/lib/components/shell/TabRail.svelte` | 417 | Pure v0.2 |
| `src/lib/components/shell/RightPane.svelte` | 133 | Replaced by WorkspaceShell |
| `src/lib/components/right-pane/index.ts` | 44 | Replaced by `workspaces/index.ts` |
| `src/lib/components/right-pane/SyncPanel.svelte` | 10 | Workspace renders SyncPage directly |
| `src/lib/components/right-pane/FilesPanel.svelte` | 21 | Workspace renders TwoPane directly |
| `src/lib/components/right-pane/HistoryPanel.svelte` | 20 | Workspace renders HistoryDrawer directly |
| `src/lib/components/right-pane/ActivityPanel.svelte` | 20 | Workspace renders ActivityFeed directly |
| `src/lib/components/right-pane/TerminalDockPanel.svelte` | 23 | Workspace renders TerminalPanel directly |
| `src/lib/components/right-pane/AgentsStub.svelte` | 23 | Replaced by DisabledWorkspace |
| `src/lib/components/right-pane/AttachmentsStub.svelte` | 23 | Replaced by DisabledWorkspace |
| `src/lib/state/right-pane.svelte.ts` | 203 | Replaced by `workspace.svelte.ts` |
| `src/lib/state/panel-types.ts` | 19 | Folded into `WorkspaceId` |

**Total deleted:** ~956 LOC. Net after new files (~150 LOC for `workspaces/index.ts` + `WorkspaceShell.svelte` + `DisabledWorkspace.svelte` + `workspace.svelte.ts`): ~−806 LOC.

### 5.9 Files to UPDATE (drop `useV03Shell` references)

Per the grep, 43 occurrences across 14 files. The implementing session should:

1. `src/lib/state/ui-prefs.svelte.ts` — delete `useV03Shell` field, key, setter.
2. `src/lib/components/AppShell.svelte` (12 refs) — collapse branches as described in §5.5.
3. `src/lib/components/shell/ChatTabsBar.svelte` (1 ref) — drop the v0.3 doc-block comment.
4. `src/lib/components/settings/Settings.svelte` (3 refs) — likely the "Shell" preference toggle UI; delete the toggle row entirely.
5. `src/lib/components/assistant/AssistantHeader.svelte` (4 refs) — verify what's gated; collapse to v0.3 path.
6. `src/lib/components/assistant/AssistantPage.svelte` (2 refs) — same.
7. `src/lib/components/assistant/HistoryDrawer.svelte` (6 refs) — drop the v0.3 chrome-shedding branch; HistoryDrawer always renders in workspace mode now. Drop overlay/scrim/close-button code paths.
8. `src/lib/components/sync/SyncPage.svelte` (3 refs) — verify; collapse to v0.3 path.
9. `src/lib/components/browser/TwoPane.svelte` (2 refs) — same.
10. `src/lib/components/terminal/TerminalPanel.svelte` (1 ref) — drop the dock-vs-workspace branching if any; terminal always renders as a workspace now.
11. `src/lib/components/assistant/TasksDock.svelte` (2 refs) — verify if this still mounts; if it's a v0.3-only feature it stays, if it's v0.2 it goes.
12. `src/lib/state/assistant.svelte.ts` (2 refs) — verify what's gated; likely tab-management logic. Keep tab logic, drop the gate.
13. `src/lib/state/terminal.svelte.ts` (1 ref) — verify; drop the `terminal.toggle()` dock-mode path entirely (terminal is a workspace now).
14. `src/lib/components/right-pane/HistoryPanel.svelte` (1 ref) — deleted in §5.8.

Verification step: after edits, `grep -rn "useV03Shell\|V03_SHELL_KEY\|v03-shell" src/` should return **zero hits**.

**`.v03` CSS class cleanup (in same files):** four files use a `class:v03={uiPrefs.useV03Shell}` directive paired with `.v03 .foo { ... }` style rules — these gate the workspace-shape styles. After dropping `useV03Shell`, the implementer must collapse this two ways:

| File | Count | Action |
|---|---|---|
| `src/lib/components/sync/SyncPage.svelte` | 6 | Delete `class:v03={...}` from root element. Promote every `.v03 .foo { ... }` rule to plain `.foo { ... }`. If a non-v03 `.foo` rule already exists, the `.v03` version wins (it's the new default). |
| `src/lib/components/browser/TwoPane.svelte` | 1 | Same |
| `src/lib/components/assistant/HistoryDrawer.svelte` | 1 | Same |
| `src/lib/components/assistant/TasksDock.svelte` | 1 | Same |

Verification: `grep -rn "\.v03\|class:v03" src/` should return zero hits after Phase 2.

---

## 6. Phasing — autonomous, CDP-verified

The work splits into three sequential commits. Each has explicit copy-pasteable verification commands; no "click around and look" steps. **All assertions return JSON; PASS = `{"value":true}`.**

### Phase 1 — Foundation (~45 min, no UI-visible change)

**Goal:** new scaffolding compiles. Current shell still active. Zero risk of breaking the running app.

**Edits:**

1. Create [`src/lib/state/workspace.svelte.ts`](../../src/lib/state/workspace.svelte.ts) — full code in §4.1.
2. Create [`src/lib/components/workspaces/index.ts`](../../src/lib/components/workspaces/index.ts) — registry per §3 table, code skeleton in §5.3.
3. Create [`src/lib/components/workspaces/DisabledWorkspace.svelte`](../../src/lib/components/workspaces/DisabledWorkspace.svelte) — placeholder per §5.2.
4. Create [`src/lib/components/shell/WorkspaceShell.svelte`](../../src/lib/components/shell/WorkspaceShell.svelte) — full code in §5.1.
5. Find where `rightPane.init()` is called (`grep -rn "rightPane.init" src/`) — add a parallel `workspace.init()` call right after it. Don't remove `rightPane.init()` yet.

**Verify Phase 1:**

```bash
# Frontend type-check (zero errors expected — new files compile in isolation).
npm run check

# Confirm the four new files exist (file-system check).
for f in \
  "src/lib/state/workspace.svelte.ts" \
  "src/lib/components/workspaces/index.ts" \
  "src/lib/components/workspaces/DisabledWorkspace.svelte" \
  "src/lib/components/shell/WorkspaceShell.svelte"
do
  test -e "$f" && echo "PASS  $f" || echo "FAIL  missing: $f"
done

# Confirm the app still runs in the existing v0.4.1 sidecar shape (nothing rewired).
bash scripts/cdp/c.sh eval "({
  bodyV041: !!document.querySelector('.body[data-v04-1=\"true\"]'),
  abButtons: document.querySelectorAll('nav.activitybar .ab-btn').length,
  workspaceMounted: !!document.querySelector('main.pane > div')
})"
# Expect: bodyV041=true, abButtons=7, workspaceMounted=true
# Phase 1 is "scaffolding only" — visible UI MUST be unchanged.
```

**Commit:**
```
feat(shell): workspace registry + state + shell scaffold

Phase 1 of docs/design/workspace-shell.md. Adds workspace.svelte.ts,
workspaces/index.ts, WorkspaceShell.svelte, DisabledWorkspace.svelte.
Not yet wired to AppShell — current v0.4.1 sidecar still active.
```

---

### Phase 2 — Cutover (~90 min, breaking UI change)

**Goal:** workspace shell is live. v0.4.1 sidecar gone. All `useV03Shell` branches collapsed. Files §5.8 still on disk (deleted in Phase 3).

**Edits in this order** (each editable safely while dev is hot):

1. **Rewrite** [`src/lib/components/shell/ActivityBar.svelte`](../../src/lib/components/shell/ActivityBar.svelte) per §5.4. Two groups, top reorderable, bottom fixed (gear), disabled-stub handling, drop count pips on disabled items.

2. **Rewrite** [`src/lib/components/AppShell.svelte`](../../src/lib/components/AppShell.svelte) per §5.5. Body shape becomes `[1fr 40px]`. ChatTabsBar gated on `workspace.activeId === "chat"`. WorkspaceShell replaces TabRail + RightPane. v02Commands → workspaceCommands. Keybindings updated per §5.6.

3. **For each file in §5.9** (14 files, 43 occurrences total), collapse `useV03Shell` branches. The v0.3 path is the kept path. Order: simple-first, harder-last:
   - `src/lib/state/ui-prefs.svelte.ts` — drop field + storage key + setter.
   - `src/lib/components/shell/ChatTabsBar.svelte` — drop the doc comment ref.
   - `src/lib/components/settings/Settings.svelte` — drop the Shell-toggle row UI.
   - `src/lib/components/assistant/AssistantHeader.svelte` — collapse to v0.3 path.
   - `src/lib/components/assistant/AssistantPage.svelte` — same.
   - `src/lib/components/assistant/TasksDock.svelte` — same.
   - `src/lib/state/assistant.svelte.ts` — keep tab logic, drop gate.
   - `src/lib/state/terminal.svelte.ts` — drop `terminal.toggle()` dock-mode entirely.
   - `src/lib/components/sync/SyncPage.svelte` — collapse to v0.3 path.
   - `src/lib/components/browser/TwoPane.svelte` — same.
   - `src/lib/components/terminal/TerminalPanel.svelte` — drop dock-vs-workspace branching; always full-pane.
   - `src/lib/components/assistant/HistoryDrawer.svelte` — drop overlay/scrim/close-X path; always workspace mode.
   - `src/lib/components/right-pane/HistoryPanel.svelte` — leave for Phase 3 deletion.

**Verify Phase 2:**

```bash
# 1. Type-check clean.
npm run check

# 2. Force a fresh app state (clears stale localStorage, reloads).
bash scripts/cdp/c.sh eval "localStorage.removeItem('rift.ui.v03-shell.v1'); localStorage.removeItem('rift.ui.right-pane.v1'); localStorage.removeItem('rift.ui.right-pane-w.v1'); localStorage.removeItem('rift.ui.activitybar-order.v1'); location.reload(); true"
sleep 3

# 3. Run smoke sections that don't depend on file deletion yet.
bash scripts/cdp/smoke-v04-10.sh shell     # body grid + sidecar gone
bash scripts/cdp/smoke-v04-10.sh ab        # 11 buttons, 10+1 groups
bash scripts/cdp/smoke-v04-10.sh default   # Chat is default, ChatTabsBar visible
bash scripts/cdp/smoke-v04-10.sh swap      # workspace-swap mechanics
bash scripts/cdp/smoke-v04-10.sh stub      # disabled click is no-op
bash scripts/cdp/smoke-v04-10.sh settings  # gear opens modal
bash scripts/cdp/smoke-v04-10.sh kbd       # Ctrl+1..8, Ctrl+0, Ctrl+,
bash scripts/cdp/smoke-v04-10.sh storage   # localStorage migration

# All must exit 0. Any FAIL = stop and diagnose before continuing.
```

**Diagnosing a Phase 2 FAIL:**

| Failure mode | Likely cause | Fix |
|---|---|---|
| `body grid has 2 columns` FAIL | AppShell still has 3-column `data-v04-1` grid | Remove the `data-v04-1` attr + 3-col rule in AppShell `<style>` |
| `11 total buttons` got 7 | ActivityBar still using old `PANELS` registry | Rewrite per §5.4 — import `WORKSPACES` not `PANELS` |
| `ChatTabsBar gone` after Sync click FAIL | Mount gate not applied | Wrap `<ChatTabsBar />` in `{#if workspace.activeId === "chat"}` |
| `localStorage = sync` after click FAIL | `workspace.setActive()` not writing key | Check `workspace.svelte.ts` setActive method |
| Disabled-stub click activates anyway | ActivityBar onclick not gated on `def.disabled` | Add `!def.disabled &&` to onclick handler |
| Ctrl+, doesn't open settings | `onGlobalKey` still has v0.3-gated branch | Remove `if (uiPrefs.useV03Shell)` wrapper on Ctrl+, handler |

**Commit:**
```
feat(shell): workspace-swap model, drop v0.2 path (BREAKING)

Phase 2 of docs/design/workspace-shell.md. Activity bar now swaps the
main pane. Conflicts + Diagnostics + Settings gear reachable from UI
(previously palette-only or unreachable). ChatTabsBar gated on Chat
workspace. v0.2 tab-rail path collapsed; useV03Shell toggle removed.

BREAKING: existing v0.2 users (useV03Shell=false) now see workspace
shell. Sweep migration in workspace.svelte.ts handles localStorage
upgrade. Setup.exe v0.4.9-alpha preserves v0.2 for anyone who insists.
```

---

### Phase 3 — Demolition + polish (~30 min)

**Goal:** delete dead files. Bump version. Update docs. Final smoke.

**Edits:**

1. **Delete every file in §5.8** (12 files):
   ```bash
   git rm \
     src/lib/components/shell/TabRail.svelte \
     src/lib/components/shell/RightPane.svelte \
     src/lib/components/right-pane/index.ts \
     src/lib/components/right-pane/SyncPanel.svelte \
     src/lib/components/right-pane/FilesPanel.svelte \
     src/lib/components/right-pane/HistoryPanel.svelte \
     src/lib/components/right-pane/ActivityPanel.svelte \
     src/lib/components/right-pane/TerminalDockPanel.svelte \
     src/lib/components/right-pane/AgentsStub.svelte \
     src/lib/components/right-pane/AttachmentsStub.svelte \
     src/lib/state/right-pane.svelte.ts \
     src/lib/state/panel-types.ts \
     scripts/cdp/smoke-v04-1.sh
   # 13 files total — the smoke-v04-1.sh asserts about the now-dead sidecar.
   rmdir src/lib/components/right-pane 2>/dev/null || true
   ```

2. **Bump version in three files** (lockstep — `scripts/release.ps1` preflight bails on mismatch):
   - `package.json` → `"version": "0.4.10-alpha"`
   - `src-tauri/Cargo.toml` → `version = "0.4.10-alpha"` (under `[package]`)
   - `src-tauri/tauri.conf.json` → `"version": "0.4.10-alpha"` (top-level)

3. **Update [`docs/CHANGELOG.md`](../CHANGELOG.md)** — prepend new entry at top, truncate older entries past the 600-word cap:

   ```markdown
   ## v0.4.10-alpha — 2026-05-XX — Workspace shell

   Activity bar now swaps the main pane instead of opening a 320-1200px sidecar.
   Eight reachable workspaces in default order (Chat · Sync · Files · Conflicts
   · Diagnostics · Terminal · Activity · History). Agents + Attachments render
   as disabled "Coming soon" tiles (Phase B). Settings gear at the bottom of
   the activity bar (was palette/Ctrl+, only). ChatTabsBar mounts only inside
   the Chat workspace.

   Conflicts + Diagnostics were previously unreachable in v0.4.1 — exposed
   only through Ctrl+Shift+D fallback (which routed to Activity, not
   Diagnostics). Both now have first-class entries.

   Dropped: v0.2 tab-rail shell, `useV03Shell` toggle, RightPane sidecar,
   panel-types/right-pane state, 5 right-pane wrapper components, 2 stub
   components. ~956 LOC deleted, ~150 LOC added. localStorage `right-pane.v1`
   migrates to `workspace.v1`; legacy keys swept on first launch.
   ```

4. **Update [`docs/HANDOFF.md`](../HANDOFF.md)**:
   - Add S96 entry summarizing the arc (≤6 sentences).
   - Update "RESUME HERE" → version becomes v0.4.10-alpha; v0.4.1 shell line dropped (no fallback exists).
   - **Remove from CRITICAL DON'T-TOUCH:**
     - `**v0.4.1 right-pane:** keep useV03Shell storage key. Width 320-1200, default 560. Left-edge-resize only.` (toggle gone, no sidecar)
   - Add to CRITICAL DON'T-TOUCH:
     - `**v0.4.10 workspaces:** registry in workspaces/index.ts. ActiveId persists to rift.ui.workspace.v1. ChatTabsBar mount gated on workspace.activeId === "chat". DisabledWorkspace renders Agents/Attachments — do NOT remove these stub entries until real components ship (registry breaks if WorkspaceId enum members vanish).`
     - `**Activity bar layout:** top group (10 workspaces, reorderable, drag-persist to rift.ui.workspace-order.v1) + bottom group (settings gear, fixed). Adding a workspace = add to WorkspaceId enum + WORKSPACES registry + DEFAULT_ORDER + smoke test indices.`

5. **Final verify — full smoke (all sections including grep + file deletions):**

```bash
# Force fresh state to verify migration from clean slate.
bash scripts/cdp/smoke-v04-10.sh fresh

# Then run all sections including grep + file deletion checks.
bash scripts/cdp/smoke-v04-10.sh
# Expect: ~70 PASS, 0 FAIL.

# Final type-check.
npm run check
# Expect: clean. Zero errors, zero warnings.
```

**Commit:**
```
chore(shell): delete v0.2 path + dead wrappers · v0.4.10-alpha

Phase 3 of docs/design/workspace-shell.md. Deletes 13 dead files
(~956 LOC removed, ~150 added net). Version bump 0.4.9-alpha →
0.4.10-alpha across package.json/Cargo.toml/tauri.conf.json.
CHANGELOG + HANDOFF updated. Full smoke (smoke-v04-10.sh) passes.
```

---

### After Phase 3 — Handoff to verifying session

Leave the dev server + CDP wrapper running. Save the final smoke output:

```bash
bash scripts/cdp/smoke-v04-10.sh > .tmp/smoke-result.txt 2>&1
echo "Exit code: $?" >> .tmp/smoke-result.txt
```

In the commit message of Phase 3, include the line:
```
Smoke: scripts/cdp/smoke-v04-10.sh — N passed, 0 failed
```
where N is the number from the actual run. The verifying session will re-run the smoke + check that N matches.

---

## 7. Preserved guarantees (don't break these)

From HANDOFF.md CRITICAL DON'T-TOUCH that REMAIN valid:

- ✅ russh / reqwest crypto config — untouched (no Rust changes).
- ✅ `~/.rift/*.json` compat — untouched.
- ✅ DriftWatcher conflict-rename guard — untouched.
- ✅ `GITHUB_OWNER`/`GITHUB_REPO` → `rift-releases` — untouched.
- ✅ Time displays w/ `[], { hour12: true }` — workspaces don't change time formatting.
- ✅ `spawn_frontend_pump` rate-limit — untouched.
- ✅ Assistant tab MCP self-exec — untouched.
- ✅ S87 context pill + image paste — untouched (AssistantPage internals).
- ✅ S91 allowlist + S92 mode — untouched.
- ✅ S88 STT + Settings section id `"speech"` — untouched.

**RETIRED by this work:**
- ❌ "v0.4.1 right-pane: keep `useV03Shell` storage key" — toggle is gone, key is swept.
- ❌ "Width 320-1200, default 560. Left-edge-resize only." — no sidecar to size.
- ❌ "v0.4.1 dropped panel-maximize" — workspaces are always full pane.

---

## 8. Risks + mitigations

| Risk | Mitigation |
|---|---|
| Existing v0.4.1 users land on Chat workspace and lose their last-active panel | Migrate `rift.ui.right-pane.v1` → `rift.ui.workspace.v1` on first launch (see §4.1). If their last panel was a stub (agents/attachments), default to Chat instead. |
| Existing v0.2 users (`useV03Shell=false`) lose familiar tab-rail | Acceptable — the toggle was experimental, v0.4.1 has been default for 4 weeks. CHANGELOG calls it out + Setup.exe v0.4.9-alpha preserves v0.2 if anyone really needs it. |
| HistoryDrawer assumes overlay chrome under one shell, workspace under another | Drop the v0.2 chrome path entirely. HistoryDrawer always renders inline (workspace) — no overlay, no scrim. Remove the close button (use the activity bar to switch away). |
| TerminalPanel similar dual-mode | Same — always renders inline. Remove dock-mode branches. The terminal session state lives in `terminal.svelte.ts` and survives workspace switches via the `everOpened` latch. |
| Conflicts/Diagnostics components have never been rendered in v0.4.1's mounting context — could blow up | Smoke test in Phase 3 with real data. ConflictsPage was main-pane in v0.2 (full-width) and is the same here, so it should be fine. Diagnostics likewise. |
| Settings modal positioning under new shell | The slide-over is `position: fixed` per its existing CSS — independent of shell grid. Should work unchanged. |
| Ctrl+0 was "close right pane" — losing this changes muscle memory for power users | Repurpose to "go to Chat" (the default). One-line change. |

---

## 9. Verifying session — autonomous re-check protocol

After the implementing session pushes Phase 3, the verifying session (me, next round) re-checks every claim. **No new code edits — pure verification.** This section is the script.

```bash
# 1. Confirm dev + CDP still up (implementer was instructed to leave them running).
tasklist 2>/dev/null | grep -i rift-tauri  # expect a hit
bash scripts/cdp/c.sh health               # expect {"ok":true,...}

# 2. Re-run full smoke. Compare PASS count against the commit message claim.
bash scripts/cdp/smoke-v04-10.sh > /tmp/verify-smoke.txt 2>&1
echo "Exit: $?"
grep -E "passed|failed" /tmp/verify-smoke.txt | tail -1
# Compare against Phase 3 commit message — must match.

# 3. Re-run with fresh state (proves localStorage migration is idempotent).
bash scripts/cdp/smoke-v04-10.sh fresh
# Expect: same PASS count.

# 4. Build clean — kill dev first (cargo + tauri dev can't share lock).
taskkill //IM rift-tauri.exe //F 2>/dev/null
sleep 2
cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# Expect: 'Finished' line, zero `error[E\d+]`.
# Then relaunch dev: cmd //c "start \"\" scripts/run-dev.bat" (so further smokes work).

# 5. npm check.
npm run check 2>&1 | tail -10
# Expect: zero errors.

# 6. Git surface diff — exactly the deletions + additions §5.8/§5.9 prescribed.
git diff main --stat HEAD~3..HEAD  # adjust ~3 if more commits landed
# Expect deletions list matches §5.8 (12 src/ files + smoke-v04-1.sh).

# 7. Version lockstep.
grep '"version"' package.json
grep '^version' src-tauri/Cargo.toml
grep '"version"' src-tauri/tauri.conf.json
# All three must be 0.4.10-alpha.

# 8. CHANGELOG + HANDOFF updated.
head -30 docs/CHANGELOG.md   # expect v0.4.10-alpha at top
head -30 docs/HANDOFF.md     # expect S96 entry + RESUME HERE updated
```

**Failure handling:** If smoke FAILs anywhere, the verifying session writes a punch-list of which assertion failed + the relevant file:line, and stops. It does NOT attempt to fix — that's a new session's job. Pasting failing assertions verbatim into a `/incident` log is mandatory.

**Spot-checks beyond smoke** (sample 3, not all):

- Read `src/lib/components/workspaces/index.ts` — confirm `WORKSPACES.agents.disabled === true` and `WORKSPACES.attachments.disabled === true`.
- Read `src/lib/state/workspace.svelte.ts` — confirm `migrateLegacy()` reads the three legacy keys AND sweeps them.
- Read `src/lib/components/AppShell.svelte` `onGlobalKey` — confirm zero surviving `useV03Shell` references and that `Ctrl+0` routes to `workspace.setActive("chat")`.

---

## 10. Open questions for the implementing session

These are decisions the implementer should make **without asking the user** (just document the call in the commit message):

1. **Diagnostics badge source.** Read `src/lib/state/diagnostics.svelte.ts` (or wherever DiagBus lives — `grep -rn "DiagBus\|diagBus" src/lib/state/` to find). If an exposed error/warn count is available, wire it. If not, ship with no badge — better than fabricating one. The DiagBus surface is server-side originated, so frontend may not have a direct count. Skip the badge in that case.
2. **Sync badge.** `connection.status?.pending` is a candidate. If reliably non-zero during sync, show `info` tone. If it's transient (always 0 by the time the badge renders), skip.
3. ~~**`Stethoscope` icon for Diagnostics.**~~ ✅ **Resolved 2026-05-18 by planning session.** `lucide-svelte@1.0.1` ships `node_modules/lucide-svelte/dist/icons/stethoscope.svelte`. Use it. No fallback needed.
4. **HistoryDrawer chrome stripping.** Read [HistoryDrawer.svelte](../../src/lib/components/assistant/HistoryDrawer.svelte) (6 `useV03Shell` refs). Decisions:
   - Drop overlay scrim (workspace fills full pane — no overlay context).
   - Drop close-X button (the activity bar handles switching away).
   - Drop slide-in animation (workspace swap is instant per the design — no transitions).
   - Keep all content/list/search internals.
5. **Mobile/narrow-window behavior.** Tauri 2 on Windows desktop typically enforces a `minWidth` in `tauri.conf.json`. Check current value: `grep -A2 '"minWidth"\|"minHeight"' src-tauri/tauri.conf.json`. Workspace + 40px rail needs ≥640px width comfortable. If existing min-width is lower than 720px, raise to 720px. If already ≥720, leave.
6. **TerminalPanel dual-mode collapse.** Currently has dock-mode + workspace-mode branches (1 `useV03Shell` ref + dock plumbing in `terminal.svelte.ts`). Collapse to workspace-only: full pane, no dock, no `terminal.toggle()` global hotkey path. The Ctrl+\` keybind now does `workspace.setActive("terminal")` instead.

---

## 11. Out of scope for this arc

These are real ideas but punt to v0.5:
- Chat overlay/slide-in from right when in non-Chat workspace (Warp-style `Cmd+Enter`). Adds split-mode complexity.
- Terminal bottom-dock available from any workspace (VS Code pattern). Adds a second axis of layout.
- Workspace presets (Cursor 2.3 style — "Sync layout", "Chat layout"). Premature until users complain.
- Implementing Agents + Attachments stubs for real.
- Drag-to-detach a workspace into its own window (Claude Code Desktop pattern).

---

## 12. Research sources

Shell pattern decision:
- [Cursor 2.3 Layout Changelog](https://cursor.com/changelog/2-3)
- [Cursor Layout UI Megathread](https://forum.cursor.com/t/megathread-cursor-layout-and-ui-feedback/146790)
- [Zed New Panel System](https://zed.dev/blog/new-panel-system)
- [Zed Agent Panel Docs](https://zed.dev/docs/ai/agent-panel)
- [Warp Terminal & Agent Modes](https://docs.warp.dev/agent-platform/local-agents/interacting-with-agents/terminal-and-agent-modes/)
- [VS Code Copilot Chat](https://code.visualstudio.com/docs/copilot/chat/copilot-chat)
- [VS Code Detached Chat GH Issue #307421](https://github.com/microsoft/vscode/issues/307421)
- [Claude Code Desktop Redesign — VentureBeat](https://venturebeat.com/orchestration/we-tested-anthropics-redesigned-claude-code-desktop-app-and-routines-heres-what-enterprises-should-know)
- [Continue.dev — BetterStack](https://betterstack.com/community/guides/ai/continue-dev-ai/)

Navigation + discoverability:
- [VS Code Activity Bar UX Guidelines](https://code.visualstudio.com/api/ux-guidelines/activity-bar)
- [VS Code Sidebars UX Guidelines](https://code.visualstudio.com/api/ux-guidelines/sidebars)
- [Linear Sidebar Refresh](https://linear.app/changelog/2024-12-18-personalized-sidebar)
- [Smashing Magazine: Hidden vs Disabled in UX](https://www.smashingmagazine.com/2024/05/hidden-vs-disabled-ux/)
- [UX Tigers: Inactive GUI Controls](https://www.uxtigers.com/post/inactive-buttons)
- [MOJ Design System: Notification Badge](https://design-patterns.service.justice.gov.uk/components/notification-badge/)
- [Material Design 3: Badges](https://m3.material.io/components/badges/guidelines)
- [Command Palette UX Patterns](https://medium.com/design-bootcamp/command-palette-ux-patterns-1-d6b6e68f30c1)
