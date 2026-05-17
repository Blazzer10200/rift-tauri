# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.1-alpha — 2026-05-17 — Right-pane refactor (drop the dock)

Daily-driver use of v0.4.0-alpha surfaced two model mismatches with what the user actually wanted, both corrected here. The `uiPrefs.useV03Shell` toggle name is reused (storage key kept for upgrade compat); v0.2 path stays pixel-identical.

### Tasks moved back to AssistantPage (Phase 1)

Tasks shows the multi-step plan Claude generates during a turn — it's an Assistant property, not a peer of Files / Sync / etc. v0.3 swept it into the dock alongside everything else; that was wrong. `tasks` is gone from `PanelId` / `PANEL_IDS` / `PRESETS` and from the dock registry, `TasksPanel.svelte` is deleted, and `TasksDock` renders inside `AssistantPage` in both shells. `AssistantHeader`'s Tasks toggle + pulse all collapse to `assistant.ui.dockOpen` unconditionally; `assistant.svelte.ts` drops the `uiPrefs.setPanelOpen("tasks", true)` half of every auto-open path. Remaining dock kbd labels rebump (Sync → 1 … Activity → 7).

### ActivityBar + RightPane shell (Phase 2)

The right side is now ONE full page at a time, picked by a 40px vertical activity bar on the far right edge (VS Code pattern). Seven entries — Files · Sync · Activity · Terminal · Agents · Attachments · History — drag-to-reorder via HTML5 DnD with the order persisted to `rift.ui.activitybar-order.v1`. `Ctrl+1..7` follows that order, `Ctrl+0` closes the pane. New `RightPane.svelte` lazy-mounts each page on first activate and keeps it mounted thereafter (everOpened latch — scroll, selection, terminal session survive toggles). Left-edge resize handle drags the pane width in `320..1200`, dblclick snaps to 50% viewport (drag/persist split + RAF-throttle).

New `state/right-pane.svelte.ts` runs a one-time storage migration on first launch: reads `rift.ui.panels.v1`, seeds `activeId` from the exactly-one-open panel (drops `tasks`), then deletes the legacy key. `rift.ui.dock-w.v1` becomes `rift.ui.right-pane-w.v1` (clamped to the new range, default 560). `rift.ui.dock-split.v1` / `maximized.v1` / `preset-picked.v1` / `dock-accordion.v1` all delete on the same boot.

Body grid is now `[chat | --right-pane-w (0 when closed) | 40px activity bar]` under `data-v04-1="true"`. `Files`Panel always renders `<TwoPane />` (summary card dropped); `SyncPage` always renders the full drift table + Mirror toolbar (`isDockSummary` branch + `.sync-summary` CSS dropped). `TerminalPanel` + `terminal.toggle()` route through `rightPane` under v0.4.1; the auto-maximize-on-open hack is gone with the maximize feature.

### Dock primitive retired (Phase 3)

`Dock.svelte` + `PanelShell.svelte` + `PresetPicker.svelte` deleted. `ui-prefs.svelte.ts` trimmed from ~325 lines to ~58 — only `density` / `railPinned` / `useV03Shell` survive. `panel-types.ts` keeps `PanelId` + `PANEL_IDS` (still feeds the right-pane registry); `PanelState` / `DockSlot` / `LayoutPreset` / `PRESETS` deleted. `AssistantPage` drops the Phase-C maximize JSX + restore-strip CSS + `PANELS`/`PanelId` imports. `TabRail` drops its panel-mode dead code (panelGroups + onPanelToggle/onOpenSettings props). `AppShell` drops the `togglePanel` helper + `PanelId` import; the command palette derives Ctrl+1..7 entries from `rightPane.order`. Settings → Appearance → Layout replaces "Reset dock split" with "Reset right pane" (`rightPane.reset()`); the dock-accordion switch is gone; kbd cheat sheet now lists Ctrl+1..7 / Ctrl+0.

### Verify

`npm run check` — 0 errors, 1 pre-existing warning (`.reasoning-meta.subtle` unused selector). No Rust changes this arc, so `cargo check` skipped (Tauri dev was alive; rule "don't `cargo check` while `npm run tauri dev` runs"). CDP smoke `bash scripts/cdp/smoke-v04-1.sh` green end-to-end. v0.3 toggle OFF renders pixel-identical to v0.4.0-alpha's v0.2 path.
