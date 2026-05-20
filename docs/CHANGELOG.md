# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.15-alpha — 2026-05-20 — N-pane split + ActivityBar redesign + sync polish

**Split-pane assistant goes N-wide (S117 + this session).** Original 2-pane split generalized to 1..4 horizontal panes. Store: `panes: PaneState[]` (always length≥1, no more null/tuple branching), new `addPane()` / `closePane(idx)` / `canAddPane` / `MAX_PANES=4`. `dropTabIntoPane(tabId, paneIdx: number)` rewritten — sibling-pane swap via `findIndex`, sentinel `paneIdx === panes.length` auto-adds a new pane at the right edge (cap-aware). `scrubTabFromPanes` / `setFocusedPane` / `assignFocusedPane` / `restoreTabs` all array-driven; restore clamps focused idx + prunes stale tab refs. `AssistantPage` renders via `{#each panes as p, i}` w/ 1px dividers between. `AssistantPane` (extracted from `AssistantPage` in S117): `paneIdx: number`, `min-width: 320px` (4×320=1280 fits any modern window), new pane-chrome — numbered badge + ✕ close button, visible only when split, only on hover/focused. `ChatTabsBar` dropped 2-color `in-p0/p1` underlines for a single `.in-pane` underline + numeric `.pane-badge` (scales 1-4); split-toggle button now calls `addPane()`, disabled at cap, shows current pane count. `Ctrl+\` adds a pane; `Ctrl+Shift+\` closes the focused pane (last pane uncloseable). `StatusHub` pane lookup via `findIndex`. `tauri.conf.json` carries `dragDropEnabled: false` from S117 — required for cross-region HTML5 DnD; Rift uses no file-drop Tauri events.

**ActivityBar redesign (S116).** Rail width 40→44px, icon 16→17px, inset pill hover, blended edge gradient, 3px active capsule, press scale 0.94. Replaced HTML5 DnD w/ pointer-event drag (WebView2 `<button>` eats `dragstart`): 4px movement threshold, floating icon + drop-line indicator + pulse, click-suppression post-drop so the icon doesn't fire a workspace-switch on release. `AppShell.svelte` grid column updated 40→44px to match.

**Sync hardening (uncommitted carry).** `auto_sync.rs` + `auto_sync/flush.rs` + `lock_presence.rs` — additional poison-path safety + diag-event coverage on top of the v0.4.14 sync batch. Net: ~150 line delta across the three files, no public API change.

**Verify:** `npm run check` 0 errors / 3 pre-existing CSS warnings across 4053 files. Backend `cargo check` clean against v0.4.14 baseline; release pipeline runs full `npm run tauri build`.

