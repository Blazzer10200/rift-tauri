# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.4.27-alpha — 2026-05-22 — Wave-3 security HIGHs + compaction UX + live tool dots

**Backend security (Wave-3 HIGHs).** Four issues closed. **#221** [assistant/mod.rs](src-tauri/src/assistant/mod.rs) — `is_valid_model_name()` (`[A-Za-z0-9._-]+`, no leading dash) rejects model in `assistant_send`, closing `--model -<flag>` injection. **#237** — spawn-log uses normalized `effort_level` instead of raw `effort`, closing log-injection vector. **#227** [diagnostics/mod.rs](src-tauri/src/diagnostics/mod.rs) — `scrub_log_message` extended with Ed25519, DSA, generic, and encrypted `BEGIN PRIVATE KEY` patterns. **#238** — `DiagBus::publish` scrubs `event.message` at the choke point; covers 58 direct `emit*` sites bypassing LogForwarder. Idempotent. Completes #8 Rust-side. **#228 deferred** — dialog plugin removal reverted at ship-time after JS-side ref was missed in the prior pass (`assistant.svelte.ts::pickFolder()` still imports `@tauri-apps/plugin-dialog`); ship-blocker resolved by restoring the plugin until a Rust-side folder-picker command lands.

**Velopack preflight.** [release.ps1](scripts/release.ps1) `Convert-ToAsciiSafe` + `[xml]` sanity probe BEFORE `vpk pack`. Catches the U+00D7-class chars (multiplication sign in literals like `1.15x`) that Velopack passes through unescaped, tripping XmlReader on the read-back path. Default flags (`--delta` + `--releaseNotes`) restored. Source kept ASCII-only with `\uXXXX` regex escapes so PS5.1's BOM-less Win-1252 read can't mojibake the table.

**Live tool dots.** [MessageBubble.svelte](src/lib/components/assistant/MessageBubble.svelte) — the `nodeStatus` override (last-node-while-streaming forced to `pending`) is now gated on `unit.status === "neutral"`. Prior behavior held the trailing tool of every turn at pulsing-yellow until the next block landed; tool + thinking blocks now honor their own per-block status and flip green/red the instant `fillToolResult` lands. Prose streaming still pulses on the last node.

**Compaction legibility.** [ChatTabsBar.svelte](src/lib/components/shell/ChatTabsBar.svelte) — ctx-pill tooltip rewritten to lead with the cap and state plainly that cache-read tokens sit in the model's window every turn (old copy framed them as "replayed from prior turns", which read as "free"). New `autoCompactDisabledNudge` chip beside the pill surfaces when `autoCompactThreshold === null && ctxPct >= 70`, going red at 85%; `data-tone="red"` added to the `.compact-warn` CSS. [Settings.svelte](src/lib/components/settings/Settings.svelte) — auto-compact threshold hint spells out that cache-read counts toward the window; the 80% option is marked `(recommended)`.

**Verify.** `npm run check` 0/0/0. `cargo check` 0 errors (1 pre-existing `private_interfaces` warning unrelated).

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

