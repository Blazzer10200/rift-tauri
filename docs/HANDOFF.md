# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## v0.4.25-alpha — 2026-05-22 — SHIPPED at 2502cd8 (S135 + S136 hotfix arc)

User installed v0.4.23-alpha → reported "fucked up" layout. Two real bugs surfaced + one misdiagnosis on the path.

- **S135** (v0.4.24-alpha @ ee2cfd7) — empty-pane UX. Orphan split-pane (no tab assigned) used to render the dead-end "No tab in this pane" string. Now an actionable card: `+ New chat` + `× Close pane` (when `panes.length > 1`) + `RECENT` quick-pick of top 3 convos not already mounted in a sibling pane. Handlers focus this pane first so `newTab`/`openTab` route through `assignFocusedPane`. CDP-verified all three actions. [AssistantPane.svelte](src/lib/components/assistant/AssistantPane.svelte).
- **S136 misdiagnosis** (a600d54) — assumed user's wide-but-short window was Windows Snap state and added `setSize+center` clamp on mount. Wrong: the window was full-size; the *layout* was collapsing inside it. Reverted.
- **S136 actual** (v0.4.25-alpha @ cfb1087) — `.shell` layout-collapse. Prod builds intermittently resolved the percentage chain `body 100% → app.html wrapper display:contents → .shell 100%` to auto-height, leaving the bottom of the window blank below StatusBar. Fix: `.shell` switched from `height: 100%` to `position: fixed; inset: 0`. `body.win-maximized` 8px padding moved onto `.shell` as `inset: 8px` since body padding doesn't push fixed children. [AppShell.svelte](src/lib/components/AppShell.svelte) + [app.css](src/app.css).

User confirmed fix landed live after Velopack pulled v0.4.25-alpha delta.

---

## v0.4.23-alpha — 2026-05-22 — SHIPPED at 0ec2cc5 (S129–S134)

Full detail in CHANGELOG. One-liners:
- **S134** Settings rebuild — 8→7 nav, `.set-group`/`.set-row` pattern, kbd grid + Speech bugs fixed
- **S133** Whisper STT backend (feature-gated) + dual-engine Speech UI — FFI UNVERIFIED live
- **S132** SplashOverlay Glass Reveal cold-boot — visual UNVERIFIED live
- **S131** Assistant chat UI overhaul — turn rail, Shiki, atmosphere, agent + TodoWrite cards, image paste, DPI
- **S130** UI audit fixes + ISSUES.md prune
- **S129** 5 MCP sync tools + dead-session wedge fix + `SyncActivityBanner` — wedge/banner UNVERIFIED live

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.25-alpha** shipped (2502cd8). Working tree clean. Tauri 2 + Svelte 5 + Rust + russh.

**Smoke gate (open — clear before next ship):**
- S129 banner: drop network 90s, edit file, push → banner should go red + Reconnect
- S131 Shiki: send ` ```rust ` fenced block → confirm header bar + syntax highlight
- S132 splash: cold-launch eyes-on; muddy-blur fallback = drop `backdrop-filter`, keep flat `--bg @ 86%`
- S133 Whisper FFI: `winget install LLVM.LLVM` (admin) + `cargo build --release --features whisper-rs`. CPU first, CUDA pass second (`whisper-cuda` feature).
- S124 items (auto-compact, ctx stats) still in gate

**Next code lanes:**
1. Smoke gate → ship next batch
2. Wave-2 audit bugs: #146 `mutateStreaming` O(n) rebuild (HIGH), #147 thinking dedup, #148 tab-switch race, #149 delete/openTab race
3. Files diff-dot per row (backend `drift_scanner` per-row verdict cmd needed)
4. Refactor queue: split `lib.rs` (2118L), `assistant.svelte.ts` (3109L), `assistant/mod.rs` (2244L)
5. Design brief: `git-rcon-tools.md` v2.2 (git + RCON MCP tools)

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
