# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.4.22-alpha — 2026-05-21 — S128 UI/UX overhaul: shell, transitions, empty-state, History popover

**Shell polish.** Activity bar flipped to left edge (matches VS Code/Linear muscle memory). Titlebar gets `--border-strong` divider + faint elev so the two-stacked-titlebars look is gone. StatusBar bumped 22 → 26px, dot 6 → 8px w/ breathing pulse when watcher healthy. Cyan info-tone removed from rail badges. ([AppShell.svelte](src/lib/components/AppShell.svelte), [shell/ActivityBar.svelte](src/lib/components/shell/ActivityBar.svelte), [shell/Titlebar.svelte](src/lib/components/shell/Titlebar.svelte), [shell/StatusBar.svelte](src/lib/components/shell/StatusBar.svelte))

**Smooth workspace transitions.** Replaced `[hidden]` (`display:none`, no animation possible) w/ absolute-layered panes that cross-fade on switch. 160ms opacity + 200ms `(.2,.7,.2,1)` transform. `inert` on inactive panes scopes tab order + clicks. Chat tabs rail now always-mounted + collapses max-height + opacity instead of remounting on every workspace hop. `prefers-reduced-motion` short-circuits both. ([shell/WorkspaceShell.svelte](src/lib/components/shell/WorkspaceShell.svelte), [AppShell.svelte](src/lib/components/AppShell.svelte))

**Per-surface cleanup.** HistoryDrawer: date-grouped (Today / Yesterday / This week / Older) + "Hide tests" filter w/ count (persisted). Diagnostics: 14 flat KPI tiles → 3 grouped sections (Sync engine / Conflicts & locks / Drift activity). Settings Terminal Reset → 2-click confirm w/ 3s auto-disarm. WatchedFoldersTable: empty columns auto-hide; all-idle state collapses to single `idle` chip per row. Composer placeholder trimmed `Ask Claude — paste images, or type / for commands` → `Ask Claude`. ([HistoryDrawer.svelte](src/lib/components/assistant/HistoryDrawer.svelte), [Diagnostics.svelte](src/lib/components/diagnostics/Diagnostics.svelte), [Settings.svelte](src/lib/components/settings/Settings.svelte), [WatchedFoldersTable.svelte](src/lib/components/sync/WatchedFoldersTable.svelte), [Composer.svelte](src/lib/components/assistant/Composer.svelte))

**Activity bar diet.** Dropped disabled `agents` + `attachments` stubs (8 entries top→bottom: chat / sync / alert / `>_` / folder / activity / stethoscope / settings). Stale workspace.order entries auto-filter on load. `WorkspaceId` union + `WORKSPACE_IDS` + `DISABLED` set updated. ([workspaces/index.ts](src/lib/components/workspaces/index.ts), [state/workspace.svelte.ts](src/lib/state/workspace.svelte.ts))

**ChatTabsBar New-chat button.** Moved from far-right (buried after ctx pill / compact / split-pane) to flush after the last tab — browser-convention placement. Restyled to elevated pill on `--bg-elev-2`, accent ramp on hover, `:active scale(0.94)`. ([ChatTabsBar.svelte](src/lib/components/shell/ChatTabsBar.svelte))

**EmptyState refresh.** Glyph gains rotating conic-gradient halo (9s/loop, blurred, masked). Headline adapts: *"Pick up where you left off"* w/ history, *"What's on your mind?"* cold. 2×2 resume-tiles surface top 4 recent conversations (title + model · msg-count · time-ago) — click to resume. Dropped the redundant synced-server card (titlebar chip already carries that info). Folder-open card stays (X-to-close is actionable). ([EmptyState.svelte](src/lib/components/assistant/EmptyState.svelte))

**History → popover.** Removed History from the activity rail entirely; lives now as a "History N ▾" button in ChatTabsBar that pops a 420×540 floating panel with search + grouped list + Hide tests + New. Portaled to `<body>` to escape `.tabs-rail` overflow-clip, position:fixed anchored to button rect. Outside-click + Esc close. Settings kbd bumped to "8". ([ChatTabsBar.svelte](src/lib/components/shell/ChatTabsBar.svelte), [HistoryDrawer.svelte](src/lib/components/assistant/HistoryDrawer.svelte))

**Verify.** `npm run check` 0 errors (3 pre-existing CSS warnings unrelated). All surfaces probed live via CDP — chat / history popover / sync / diagnostics / settings / files.
