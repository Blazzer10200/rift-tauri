---
status: planning
created: 2026-05-19
session-origin: 2026-05-19 afternoon (post-#9.1/9.2/10/#1 ship batch)
scope: ~3 focused sessions
---

# UI Shell Redesign — Stable workspace frame + Sync-as-dashboard

## RESUME HERE — first read every new session

Map of the source-of-truth files this brief governs:

- Shell components: [src/lib/components/shell/](../../src/lib/components/shell/) -- `PageHeader.svelte`, `StatusBar.svelte`, `WorkspaceShell.svelte`, `ActivityBar.svelte`, `ChatTabsBar.svelte`, `Titlebar.svelte`, `EmptyState.svelte`
- Workspace registry: [src/lib/components/workspaces/index.ts](../../src/lib/components/workspaces/index.ts)
- Workspace state: [src/lib/state/workspace.svelte.ts](../../src/lib/state/workspace.svelte.ts)
- Pages this brief edits: `AssistantPage.svelte`, `SyncPage.svelte`, `TwoPane.svelte` (Files), `ActivityFeed.svelte`, `ConflictsPage.svelte`, `Diagnostics.svelte`, `TerminalPanel.svelte`, `HistoryDrawer.svelte`, `Settings.svelte`

Each phase below is independently shippable -- don't bundle them.

---

## Why this brief exists

Visual audit 2026-05-19 (Opus 4.7 via CDP, four shots across Chat/Sync/Files/Activity + Settings overlay) surfaced one systemic problem: **every workspace builds its own top-region from scratch.** Heights, alignment, and idioms vary per page. The active activity-bar icon is the only durable "where am I" landmark. Plus Sync -- the app's home page -- is 90% dead air.

Fix is structural, not cosmetic. Lock the shell frame first, then iterate per workspace.

---

## Existing assets to reuse, don't rebuild

`PageHeader.svelte` is already built (46px min-height, tone variants accent/info/warn/danger/ok/neutral, supports `actions` + `extras` snippets). **Currently used by ONE workspace: ConflictsPage.** Every other page rolled its own. The redesign applies the existing component everywhere -- no new component, no rewrite.

`StatusBar.svelte` is already built and mounted (22px, connection LED + state pill + watcher toggle + locks indicator + bg-sync pill). **Underused.** Missing: conflicts count, last-scan time, bridge state, queue depth (the `pending` field on `connection.status` is already there, just not rendered), app version. Phase 1 extends this component -- doesn't replace it.

`WorkspaceShell.svelte` is the layout root. Phase 1 doesn't touch its structure -- only sequences children differently if PageHeader needs to live at the workspace level vs inside each page (decision below).

---

## Target shell (every workspace)

```
+- Titlebar ----------------------------------------------------+
| [Rift]                                          [server][win] |
+- PageHeader (46px) -------------------------------------------+
| [icon] Page title - subtitle             [page actions ...]   |
+- Body (fills remaining height) ------------------+------------+
|                                                  |            |
|   workspace content                              | Activity   |
|   (no nested headers above content)              | bar (40px) |
|                                                  | right edge |
+- StatusBar (22px, extended) ---------------------+------------+
| (LED) Connected . 8 watching . 0 conflicts . synced 12s . v0.4.11 |
+---------------------------------------------------------------+
```

Three invariants:

1. **PageHeader is mandatory** for every workspace. Sync currently has none; Files has a `resources` chip pretending; Activity uses a filter row. Apply the existing component everywhere.
2. **StatusBar carries connection chrome.** Conflicts count, last-sync time, bridge state, app version all move there. Workspaces stop showing connection state in their local UI.
3. **Body is full-bleed.** No nested headers, no breadcrumb rows above content unless a workspace genuinely needs path nav (only Files does).

Decision (locked, don't re-debate): **PageHeader lives INSIDE each workspace component**, not at the shell level. Reason: actions vary per page and the snippet API is the right contract. The shell already trusts pages to fill the body region; same pattern.

---

## Phase 1 -- Shell foundation (1 session, ~2-3h)

**Goal:** every workspace renders inside the same frame. No new features, no visual polish. Just consistency.

### Deliverables

1. Extend [`StatusBar.svelte`](../../src/lib/components/shell/StatusBar.svelte) with the following slots, in order, left-to-right after the existing watcher toggle:
   - `pending` queue depth (already on `connection.status.pending`) -- only render when > 0
   - `conflicts` count -- only render when > 0, tone=danger
   - `last scan Xs ago` -- derive from a new `lastScanAt` field on `connection` store (set by `start_autosync` + `rescan` Tauri events; falls back to `--` if never scanned)
   - bridge state pill -- "bridge" if `connection.selected?.hasBridgeToken` (S106 / #9.1 field) AND server is connected, else hide
   - app version -- read once at module init via `invoke<string>("app_version")` (Tauri cmd already exists at lib.rs:1713 registry). Right-edge.
2. Apply `PageHeader` to every workspace that doesn't have one. Inventory:
   - Chat / AssistantPage -- HAS its own custom header (`AssistantHeader.svelte`). Convert to `PageHeader` + page-actions snippet for the context pill + Open folder + checkbox + chat-thread icons. Drop the `BETA` chip from the title (ages poorly).
   - Sync / SyncPage -- ADD. Title "Sync", subtitle binds to `connection.selected?.name`, actions snippet holds `...` overflow + Sync button. The existing right-flush mini-toolbar collapses into the actions snippet.
   - Files / TwoPane -- ADD. Title "Files", subtitle "{server name} - resources". Path breadcrumbs stay above each pane (they're nav, not chrome).
   - Activity / ActivityFeed -- ADD. Title "Activity", subtitle "{N} events". Filter chip row moves to a sub-toolbar below PageHeader; Pause + Clear move into the actions snippet.
   - Conflicts / ConflictsPage -- already uses PageHeader. Audit for parity (tone, height).
   - Diagnostics, Terminal, History -- ADD. Standard pattern.
3. Drop per-page connection-state badges that the new StatusBar now owns. Grep for occurrences of `connection.status?.state`, `pill =`, `SyncPill` -- consolidate.

### Acceptance

- Every workspace has a 46px PageHeader at the top of the body.
- Vertical position of "first line of content" is identical (within 4px) when switching workspaces via `Ctrl+1..8`.
- StatusBar shows pending / conflicts / last-scan / bridge / version when applicable.
- `npm run check` 0/0.
- CDP shot of each workspace shows the new shell (save under `state/audit-2026-05-19/ui-redesign-phase1-after/`).

### Don't-touch in Phase 1

- No content changes inside workspace bodies. Sync stays dead-air for now. Phase 2 fixes that.
- No new icons, no theme changes, no font changes.
- No changes to `ActivityBar` or `Titlebar` order.

---

## Phase 2 -- Sync dashboard rebuild (1 session, ~2-3h)

**Goal:** Sync page goes from 90% dead air to the app's actual home dashboard.

### Layout

Inside SyncPage body, below the new PageHeader:

```
+- Watched folders -----------------------+
| name              files . last . locks  |
| [endure]          412   . 2s   . 0      |
| [qbx]             1.2k  . 5s   . 0      |
| ...                                     |
+-----------------------------------------+
+- Recent activity ---+- Drift summary ---+
| 14:03  endure/...   | nothing diverged  |
| 14:02  qbx/...      | (or N divergent   |
| 14:01  ...          |  cards)           |
+---------------------+-------------------+
```

### Deliverables

1. `WatchedFoldersTable.svelte` -- compact list. Rows = folders from `connection.selected.foldersWatched` (or whatever the engine exposes -- if missing, add a `list_watched_folders` Tauri cmd returning `Vec<{name, file_count, last_event_at, lock_count}>` with the engine's existing state).
2. `RecentActivityCard.svelte` -- last 5 rows from `connection.activityFeed` (the store already keeps `.slice(0, 200)`). Tail with link "Open Activity" -> switches workspace to `activity`.
3. `DriftSummaryCard.svelte` -- pulls from existing `sync_get_drift_snapshot` cmd. Empty state shrinks to a green-check chip; otherwise cards per diverged path.
4. Click-to-filter: clicking a watched-folders row switches to Activity workspace pre-filtered to that folder (deeplink via workspace state's `setActive` + a new `activityFilter` field on connection store).

### Acceptance

- Sync page no longer has a centered empty-state hero. Watched folders + activity tail + drift summary fill the page.
- Empty states for each card render correctly when there's nothing to show.
- Click-deeplink from a folder row lands on Activity with the filter applied.
- `npm run check` 0/0.

### Don't-touch in Phase 2

- No changes to the sync engine. All data already exists -- this is pure UI.
- No changes to ActivityFeed itself (just consume from it). The deeplink hook is the only ActivityFeed-side change.

---

## Phase 3 -- Composer + Settings-as-workspace (1 session, ~2h)

**Goal:** reclaim vertical space in the Assistant composer + eliminate the Settings overlay.

### 3a. Composer compression

[Composer.svelte](../../src/lib/components/assistant/Composer.svelte) bottom row currently shows:

```
Enter send . Shift+Enter newline . / commands              [Quick] [Sonnet 4.6]
```

Compress to:

- Hint line collapses behind a `(?)` info icon adjacent to the mic. Hover/click reveals the three hints in a popover.
- `Quick` + `Sonnet 4.6` pills move to the right edge of the textarea row itself (same row as send button).
- Saves ~30px of vertical real estate per Assistant tab.

### 3b. Settings as workspace

- Add `settings` to the `WorkspaceId` union in [workspace.svelte.ts](../../src/lib/state/workspace.svelte.ts).
- Add registry entry in [workspaces/index.ts](../../src/lib/components/workspaces/index.ts) -- icon `Settings`, title "Settings", kbd `9`.
- Settings overlay component becomes a workspace component -- existing left-rail subnav stays as-is, just no longer slides out.
- Drop the gear button from the bottom of `ActivityBar.svelte`. The activity-bar's drag-reorder bottom-group goes away with it (Settings now lives in the main draggable group).
- `Ctrl+,` keyboard shortcut maps to `workspace.setActive("settings")` instead of opening the overlay.

### 3c. Tighten chat-tabs gutter (rolls in #6 from ISSUES.md)

- Move the `+` (new tab) button to the right end of the chat-tab strip, separated from the activity-bar boundary by 4-6px. Fixes scrollbar collision.
- Apply `scrollbar-gutter: stable` to the chat-thread scroller in [AssistantPage.svelte](../../src/lib/components/assistant/AssistantPage.svelte) to prevent the gutter shift on overflow.

### Acceptance

- Composer hint row is gone; hint accessible via info icon.
- Settings opens as a workspace, not an overlay. `Ctrl+,` works.
- The `+` button doesn't visually collide with the assistant scrollbar.
- Files page-toolbar (originally Phase 2, deferred here if Phase 2 ran long) lands w/ Filter input + Hide-ignored toggle + Show-divergent-only toggle + Refresh-both + Mirror-pair-jump.
- `npm run check` 0/0.

---

## Cross-phase invariants (don't break)

- **Per CLAUDE.md gotchas**: don't reintroduce `dock primitive`, `maximize-to-center`, `PanelState.slot`, `TabRail`, `OpRail`, `RightPane sidecar`. The current shell IS the shell.
- **Activity bar order is user-data** -- persisted to `rift.ui.workspace-order.v1`. Adding `settings` to the registry must append, not insert, so existing users' order isn't reshuffled.
- **Keyboard shortcuts** `Ctrl+1..8` follow activity-bar order (per ActivityBar.svelte title). With Settings as workspace 9, existing 1-8 stay stable.
- **PageHeader tone** should match activity-bar accent for the active workspace (already wired via `getTone`). Phase 1 must preserve this.
- **No new dependencies.** Everything fits in existing lucide-svelte + Svelte 5 runes + Tailwind 4.

---

## Decision log (so we don't re-argue)

| Question | Decision | Why |
|---|---|---|
| PageHeader at shell level vs per-page? | Per-page. | Page actions are page-specific; the snippet API is the right contract. |
| Settings workspace vs overlay? | Workspace. | Eliminates the overlay-occludes-content problem; matches the rest of the app's idiom. |
| Server selector in titlebar vs status bar? | Stays in titlebar. | Global context switch; belongs in chrome, not status. |
| Watched-folders source of truth? | Backend cmd. | Engine state is authoritative; frontend mirror would drift. |
| Drop `BETA` chip from Assistant title? | Yes. | Ages poorly; not load-bearing. |
| Migrate to a real left rail? | No. | Right-edge activity bar is the established pattern; flipping it is a much bigger ask. |

---

## Phase ordering rationale

1. **Phase 1 first** because every subsequent phase assumes the shell is stable. Doing Sync dashboard before PageHeader sweep means rebuilding Sync's chrome twice.
2. **Phase 2 second** because Sync is the highest-value content change and depends only on Phase 1's frame.
3. **Phase 3 last** because composer compression + Settings-as-workspace are independent of Sync, but Settings-as-workspace touches the workspace registry that Phase 1 audits. Cleaner to do registry changes once, in Phase 3.

---

## What this brief does NOT cover

- Onboarding flow (#7 in ISSUES.md) -- separate epic.
- New-user empty states (#7 partial overlap) -- pick up during Phase 2's empty-state pass.
- Tool-block rendering rhythm in chat (#2 in ISSUES.md) -- separate UX iteration inside Assistant.
- Status indicator hub above composer (#5 in ISSUES.md) -- partial overlap with Phase 3 composer compression. Re-evaluate after Phase 3 lands; may already be moot.
- App-wide UX consistency sweep (#4 in ISSUES.md) -- this brief IS the start of that sweep.
- Theming, density, accent-tint controls -- the "More coming soon" card in Settings stays a placeholder until a separate brief.

---

## Pre-flight checks before opening Phase 1

Run these once at the top of the Phase 1 session:

1. `git status` -- confirm S105/S106/#9/10/#1 batch is either committed or you've decided to layer Phase 1 on top of the uncommitted stack.
2. `cd "c:/AI Workflow/projects/rift-tauri" && bash scripts/cdp/c.sh health` -- CDP wrapper alive.
3. Read this brief top-to-bottom.
4. Read the four pre-redesign shots if they survived (`scripts/cdp/.tmp/snap-2026-05-19T13-25-32.jpeg` and three more from the same minute) -- compare against post-Phase-1 to confirm the shell unification landed.

If CDP wrapper is dead: `npm run cdp:serve` in a separate shell.
