# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## v0.4.26-alpha — 2026-05-22 — assistant timeline UI + center-on-work-area

- **Timeline UI.** Vertical timeline on the existing turn-rail. Each block = a node w/ status-colored bullet (hollow gray=thinking, green=done, pulsing accent=pending, red=error) + drop-shadow + inner highlight for depth. "Thought for Ns" flat single-line — `.reasoning` bordered surface gone. Tool chips drop card frame in collapsed state via new `variant="timeline"` prop; right-edge status pip removed (rail bullet shows it). Agent + TodoWrite cards unchanged. Step-N headers → uppercased dividers w/ trailing hairline; `StepGroup` numbered bubble dropped from main flow (component intact, unused). Rail recolored neutral gray (1.5px, `--fg-faint 38%`); streaming still glows accent + last bullet pulses. Hover lifts bullet 1.15×. Files: [MessageBubble.svelte](src/lib/components/assistant/MessageBubble.svelte), [ToolChip.svelte](src/lib/components/assistant/ToolChip.svelte).
- **Window centers on work area.** [tauri.conf.json](src-tauri/tauri.conf.json) `visible:false` + `center:true`; [lib.rs::run](src-tauri/src/lib.rs) `setup()` calls `center_in_work_area` then `show + set_focus`. Windows: `SystemParametersInfoW(SPI_GETWORKAREA)` via raw FFI (no new deps), centers in taskbar-excluded rect. Non-Windows: Tauri `center:true` fallback. **Not runtime-verified** at fresh launch — exercise via `scripts/run-dev.bat` if behavior looks off.

## v0.4.25-alpha — 2026-05-22 — SHIPPED at 2502cd8

S135 empty split-pane UX + S136 `.shell` layout-collapse fix (`position: fixed; inset: 0`). Details in CHANGELOG + git log.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.26-alpha** shipped + Velopack-published to [rift-releases](https://github.com/Blazzer10200/rift-releases/releases/tag/v0.4.26-alpha) (Setup.exe + Portable.zip + full nupkg, no delta). Working tree clean. Tauri 2 + Svelte 5 + Rust + russh.

**Velopack quirk to revisit:** `vpk pack` crashed on `Line 17, position 208` XmlException when reading either the prior .25 nupkg's manifest (delta build) OR a freshly-built .26 manifest (setup wrap). Workaround used this ship: `--delta None` + skip `--releaseNotes` flag. Smoking-gun is the markdown→HTML conversion vpk runs on CHANGELOG entries — special chars in `var(--fg-faint 38%)`/em-dashes/markdown link `()` syntax → bad entity reference. Reproduce + fix at top of next ship; release.ps1 is back to default state.

**Open queue → [docs/ISSUES.md](ISSUES.md#active-work--current-sprint).** This file = session state + don't-touch invariants only.

---

## CRITICAL DON'T-TOUCH

- russh `ring` + reqwest `rustls` only. russh `Config{keepalive 20s/3, window 2MiB, packet 32KiB}`.
- `~/.rift/*.json`: keep `serde(flatten) extra`. `VelopackApp::build().run()` FIRST in `lib.rs::run()`. `bundle.targets:["nsis"]`.
- DriftWatcher: never overwrite dirty local. `.rift-trail.jsonl` ignore mandatory.
- `GITHUB_OWNER`/`GITHUB_REPO` → public `rift-releases`, NOT source repo.
- `path_guard.rs` frozen; `rename_overwriting_via` ONLY for atomic upload tmp-swap.
- `FileAttributes::default()` for SETSTAT = data-loss — use `empty()`. `DriftBucket::ToDelete`=LOCAL; `ToDeleteRemote`=REMOTE.
- Time displays MUST pass `[], { hour12: true }`. `spawn_frontend_pump` 200/s rate-limit.
- MCP self-exec: `RIFT_MCP_SERVER=1` branch in `lib.rs::run()` BEFORE Tauri loop.
- Chat tabs: `openTabs` filters vs `assistant_list_conversations` on init. `send()` keys `isFirstTurn` off `convoCreatedAt`. Keybinds gated on `workspace.activeId === "chat"`.
- `assistant_send`: `--permission-mode bypassPermissions` + full `BUILTINS` in `--allowed-tools`.
- TabState: per-tab field → add to TabState + getter on AssistantStore. Never back on the store.
- Image paste: `assistant_send` flips `--input-format text→stream-json` when attachments present. 20MiB cap.
- Settings is workspace (kbd **8** post-S128), `Ctrl+,` flips; do NOT reintroduce slideover scrim.
- `UpdateService` managed Tauri state — `download_update` then `apply_pending_update`.
- **`tauri.conf.json` `dragDropEnabled: false`** — removing breaks cross-region HTML5 DnD. Rift has no file-drop Tauri events, cost = zero.
- **AssistantPane drop handlers on `.pane` outer div only** — never move to inner `.drop-zone` overlays; loses the continuous-preventDefault chain.
- **`compactionHistory[]` field name is camelCase** in persisted JSON (`compactionHistory`, not `compaction_history`) — Rust extracts via `Value::get("compactionHistory")` in `assistant_list_conversations`. Don't rename.
- **`.shell` MUST be `position: fixed; inset: 0`** (AppShell.svelte) — `height: 100%` chain via app.html's `display: contents` wrapper collapses in prod. `body.win-maximized .shell { inset: 8px }` compensates for the borderless-maximized invisible-frame.
