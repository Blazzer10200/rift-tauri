# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.4.26-alpha — 2026-05-22 — assistant timeline UI + center-on-work-area

**Claude Code VSCode-style timeline.** Replaces the in-bubble tool-call + thinking presentation w/ a vertical timeline anchored to the existing turn-rail. Each block becomes a node w/ a status-colored bullet (hollow gray=thinking, filled green=done tool, pulsing accent=pending, red=error) and a small drop-shadow + inner highlight for depth. "Thought for Ns" lines lose the bordered `.reasoning` surface → flat single-line. Tool chips drop the card frame in collapsed state via new `variant="timeline"` prop on [ToolChip.svelte](src/lib/components/assistant/ToolChip.svelte); right-edge status pip removed since the rail bullet carries status. Agent + TodoWrite cards keep their chrome (their body IS the content). Step-N prose headers → uppercased dividers w/ trailing hairline; numbered `StepGroup` bubble dropped from the main flow (component intact, just unused). Rail recolored neutral gray (1.5px, `--fg-faint 38%`) so it reads as universal timeline rather than accent branding; streaming still glows accent and the last bullet pulses as the "current activity" beat. Content gap tightened 6 → 5px, hover lifts the bullet 1.15× for interactivity signal. [MessageBubble.svelte](src/lib/components/assistant/MessageBubble.svelte).

**Window centers on work area at launch.** [tauri.conf.json](src-tauri/tauri.conf.json) flips `visible: false` + `center: true`; [lib.rs::run](src-tauri/src/lib.rs) `setup()` calls `center_in_work_area(&main)` then `show() + set_focus()`. On Windows the helper calls `SystemParametersInfoW(SPI_GETWORKAREA)` via raw FFI (no new deps) so the window centers in the taskbar-excluded rect; non-Windows falls back to Tauri's `center: true`. Best-effort fallbacks if FFI fails.

**Verify.** `npm run check` 0/0. CDP-verified live: 14-bubble convo w/ mixed tool calls, reasoning beats, EditDiff, table, and prose renders w/ bullets on the rail, naked single-line chips, flat thinking nodes, and zero leakage onto the user side. Window-centering not yet runtime-tested live — fresh launch via `scripts/run-dev.bat` recommended.

## v0.4.25-alpha — 2026-05-22 — S136 hotfix: shell layout-collapse

**Bottom-of-window blank-zone killed.** Prod builds intermittently collapsed `.shell` to its content height, leaving the bottom of the window blank below the StatusBar despite the window being full-size. Root cause: percentage-height chain `body 100% → app.html wrapper display:contents → .shell 100%` works in dev but breaks under prod build conditions (display:contents height-resolution edge case in the bundled chunks). [AppShell.svelte](src/lib/components/AppShell.svelte) — `.shell` switched from `height: 100%` to `position: fixed; inset: 0`, sidestepping every parent-height-resolution path. SplashOverlay is also `position: fixed` so flow ordering is unaffected. [app.css](src/app.css) — `body.win-maximized` 8px padding moved onto `.shell` directly (`.win-maximized .shell { inset: 8px }`) since body padding doesn't push fixed children.

**Verify.** `npm run check` 0/0.

## v0.4.24-alpha — 2026-05-22 — S135 hotfix: empty-pane UX

**Empty-pane card replaces dead-end string.** [AssistantPane.svelte](src/lib/components/assistant/AssistantPane.svelte) — when a split pane has no tab assigned, the slot used to render only the unhelpful "No tab in this pane" string w/ no recovery action. Now renders an actionable card: `+ New chat` (primary) + `× Close pane` (ghost, only when `panes.length > 1`) + a `RECENT` quick-pick listing the top 3 conversations not already mounted in a sibling pane. Each handler focuses this pane first so `newTab`/`openTab` route through `assignFocusedPane` → land in the empty slot. Recent picks filter out convos held by sibling panes to avoid cross-pane tab-yank.

**Verify.** `npm run check` 0/0. CDP-verified: card renders w/ correct copy + 3 recents; `New chat` mints a tab into the empty pane (panes 2→2, tabs 1→2); `Close pane` collapses (panes 2→1); recent-row click opens convo here (panes 2→2, tabs 1→2). Visual snapshot via `shot-sel .pane-empty-card` confirmed.

<!-- Older entries (v0.4.23-alpha, v0.4.22-alpha) preserved in git log. -->

