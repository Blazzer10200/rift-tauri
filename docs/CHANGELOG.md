# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.13-alpha — 2026-05-19 — Assistant UI overhaul + update-flow restyle

**Assistant page redesign (S111).** Killed the redundant empty-tabs gate — first tab auto-opens on mount, click-twice-to-chat dead funnel is gone (~85 LOC deleted from `AssistantPage.svelte`). User messages now right-align with a neutral `--bg-elev-2` surface + 12px radius (was left-aligned with `--accent-soft` colliding with reasoning blocks); user avatar dropped entirely — bubble position carries the role signal. Claude turn-badge ("Sonnet 4.6 · $0.0421") sits inline beside the role name instead of floating right via `margin-left:auto`. Copy button claims the right edge of the role row. Messages container widened from 860px to `min(960px, 88ch)` w/ 20px gap + faint top-border between adjacent bubbles for turn rhythm.

**Header de-twinning.** `+` button now labeled "New" so it stops reading as an icon-twin of the tasks-toggle. Tasks toggle only renders when `taskCount > 0` — perpetually-empty corner button gone. Workspace chip recolored from accent-purple to neutral `--bg-elev-2` (accent reserved for AI-originated surfaces only).

**EmptyState anchoring + stagger.** Hero anchored at 12vh from top instead of vertical-centered (no more 200px void above the suggestions). Workspace card + suggestions cards both widened to 520px to match. Suggestion-card prompt clamp bumped 1→2 lines so the helpful detail isn't truncated. Stagger entrance (60ms/100ms/140ms delays) + 4.2s hero-glyph breathe animation + press-state scales on cards.

**Composer normalization.** Width lockstep with messages (`min(960px, 88ch)`). All controls baseline-centered: mic 26px (borderless, faded), hint 22px, effort pill 22px (smaller — secondary toggle), model pill 24px w/ ▾ caret indicating it opens a menu, send 28×28 (down from 32, less aggressive CTA). Composer auto-switches `align-items` to `flex-end` via `:has(textarea:not(:placeholder-shown))` when multi-line so controls hug the bottom row. Queue block recolored neutral (was accent-soft dashed border). Focus glow transition smoothed `140ms → 200ms cubic-bezier`. Send-press scale 0.96 for tactile feedback.

**Scrollbar nuke.** `.scroll` chat container and `.strip` tab-bar both fully hide their scrollbars (`scrollbar-width: none` + `::-webkit-scrollbar { display: none }` + `::-webkit-scrollbar-button { display: none }`). The native WebView2 arrow-buttons that were leaking through default `::-webkit-scrollbar` rendering in the top-right corner are gone. Scroll still works via wheel/keyboard/touch.

**Jump-to-latest pill.** Floating pill above composer (`bottom: 84px`, center-aligned) appears when user scrolls up away from tail. Click → smooth-scroll back + re-arm stick-to-bottom. Tab-switch scroll restore also upgraded to smooth-scroll (streaming-delta autoscroll stays instant to avoid fighting itself).

**ChatTabsBar entrance.** New tabs slide in over 220ms cubic-bezier (translateY -4px → 0).

**Update flow restyle (S110).** `UpdateInfoDto` extended with `sizeBytes` + `notesMarkdown` + `releaseUrl` + `publishedAt`; `UpdateService` is now managed Tauri state (`Arc<UpdateService>`) so pending `UpdateInfo` survives between commands. Monolithic `apply_updates` split into `download_update` (streams `update-progress` i16 events + emits `update-downloaded`) + `apply_pending_update`. Frontend store grew an 8-state machine + progress + `dismissedVersion` snooze + `pillVisible`/`sizeLabel`/`publishedLabel` derived getters. `UpdateDialog` restyled (gradient header + glow, version-diff chips, release-notes card w/ markdown-lite renderer, shimmer progress card, green rocket ready-card, per-state footer). New `UpdateToast.svelte` slides up bottom-right when an update is available (12s auto-dismiss paused on hover, snooze × button). StatusBar pill (pulsing dot + sparkles + version) visible when available/ready + toast dismissed + dialog closed. `scripts/release.ps1` gained conditional `--splashImage` flag for the Velopack installer (active iff `src-tauri/installer-splash.png` exists).

Net: Frontend `npm run check` 0/0/4051. 3-file bump 0.4.12-alpha → 0.4.13-alpha.

