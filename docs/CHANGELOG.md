# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.0-alpha — 2026-05-17 — Chat tabs + split dock (experimental v0.4)

Two layered features on top of the v0.3 single-canvas shell: browser-style chat tabs at the top of Rift, and a right-side dock that can grow up to half-screen and split horizontally into left + right slots, each carrying its own panel stack. Both ride the existing `uiPrefs.useV03Shell` toggle (Settings → Appearance → Experimental). v0.2 path remains pixel-identical — the toggle is the rollback.

### Chat tabs (Phase 1)

A dedicated 34px row sits below the Titlebar, only when v0.3 is on. Each tab is one Claude conversation; opening many lets you context-switch without losing state. Close keeps the convo in History. `MessageSquare` icon swaps to a pulsing dot while that tab is mid-stream. Tab titles auto-fill from the first user message (40-char cap) — unsaved new tabs show "New chat" until the first send saves them.

`AssistantStore` grows `openTabs: string[]` plus `openTab`/`closeTab`/`newTab`/`reorderTabs`/`cycleTab`/`closeAllTabs`/`closeOthers`/`closeTabsToRight`. Persistence is `localStorage["rift.ui.tabs.v1"] = { openTabs, activeTabId }`. On init, stored tab ids filter against `assistant_list_conversations` — orphan ids drop silently. `send()` now keys "first turn vs resume" off `convoCreatedAt`, not `currentConvoId`, so newTab can mint the id up-front without breaking the CLI's `--session-id` path. Click-to-switch handles unsaved-new-tab targets in-memory instead of disk-loading a record that doesn't exist yet.

Keybinds: `Ctrl+T` new tab · `Ctrl+W` close active · `Ctrl+Tab` / `Ctrl+Shift+Tab` cycle · `Alt+1..9` jump. Drag a tab to reorder (HTML5 DnD + tail-zone for append). Empty state (no tabs) replaces chat + composer with a centered "+ New chat" card hinting at History.

### Split dock (Phase 2)

`PanelState` grows a `slot: "left" | "right"` field. localStorage migration on load — any panel without `slot` defaults to "left", so existing v0.3 users see no visible change. The outer dock width recomputes its max per resize: `Math.min(900, innerWidth - 480)`, reserving a 480px chat minimum. Double-click the outer resize handle to snap to ~50% viewport.

Inside the dock, a CSS grid `[left slot] [4px split-handle] [right slot]` collapses to a single column when the right slot is empty. Drag a panel header across the slot boundary to reassign — the right slot appears as a "Drop here → New right slot" target during left-source drag; an occupied slot shows a soft outline + dragover tint. New `dockSplitPct = $state(50)` drives `--dock-split-pct` on `:root`; the internal 10px split-handle hit area (2px visual via `::after`, RAF-throttled, persist-on-release) follows the same drag-vs-release pattern as the outer width handle. Min/max 20–80%, double-click snaps to 50.

Accordion sweep (`applyOpenState(closeOthers=true)`) restricts to the dragged panel's slot — opening a left-slot panel no longer collapses a right-slot panel. `Ctrl+1..8` still toggles the named panel regardless of which slot it lives in.

### Polish (Phase 3)

Settings → Appearance picks up a Layout sub-card under the v0.3 accordion toggle: "Reset dock split" button (→ 50%), "Close all chat tabs" button, and a kbd cheat sheet. New `scripts/cdp/smoke-v04.sh` runs 23 checks end-to-end (Ctrl+W loop reset → 3-tab open → keyboard cycle → close-middle → cross-slot drag → per-slot accordion → split resize → maximize+Esc → empty-right collapse). 23/23 green.

CDP wrapper (`scripts/cdp/serve.cjs`) — Key dispatch grows on-demand resolution for digits 0-9 + letters a-z; `KEY_DEFS` stays the source of truth for special keys.

### Don't-touch carryovers from v0.3

Registry-based PanelShell mount, `applyOpenState` clearing `maximized` when its panel closes, Terminal lazy-mount fallback, slide-over Settings, S76 dock-resize snappiness (RAF + `setDockWidthLive`/`persistDockWidth` split + no grid transition under v0.3). All v0.2 codepaths preserved verbatim.

### Verify

`svelte-check` clean. `cargo check` clean (no Rust changes this arc). CDP smoke `bash scripts/cdp/smoke-v04.sh` 23/23 PASS. v0.3 toggle OFF renders pixel-identical to v0.2.56-alpha.
