# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 135 — 2026-05-22 — Empty-pane UX (uncommitted, post-v0.4.23-alpha)

**Reported:** post-update Rift looked "fucked up" — screenshot showed wide-but-short window (Win-snap), 2 panes with one empty showing useless "No tab in this pane" + History popover open over a 0-msg fresh tab.

**Diagnosis.** Window-shortness was Windows Snap bypassing `minHeight: 800` — not a code bug. The visual confusion was the multi-pane orphan-empty state: 2 panes persisted but only 1 (or 0) tabs assigned → other pane stuck on the dead-end "No tab in this pane" string with no recovery action.

**Fix.** [AssistantPane.svelte](src/lib/components/assistant/AssistantPane.svelte) — replaced the orphan empty state w/ an actionable card: `+ New chat` (primary) + `× Close pane` (ghost, only when panes.length > 1) + a `RECENT` quick-pick listing the last 3 conversations not already mounted in another pane. Each handler focuses this pane first so `newTab`/`openTab` route through `assignFocusedPane` → land in the empty slot. Filters out convos already in sibling panes to avoid cross-pane tab-yank.

**Verify.** `npm run check` 0/0. CDP-verified: empty-pane card renders w/ correct copy, `New chat` mints a tab into this pane (panes 2→2, tabs 1→2), `Close pane` collapses (panes 2→1), Recent-row click opens the convo here (panes 2→2, tabs 1→2). Visual snapshot via `shot-sel .pane-empty-card` confirmed.

**Not bumped.** v0.4.23-alpha just shipped 0ec2cc5. User can `/git-ship` as 0.4.24-alpha when ready.

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

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.23-alpha** shipped (0ec2cc5). S135 empty-pane UX fix uncommitted on top. Tauri 2 + Svelte 5 + Rust + russh.

**Smoke gate (open — clear before next ship):**
- S129 banner: drop network 90s, edit file, push → banner should go red + Reconnect
- S131 Shiki: send ` ```rust ` fenced block → confirm header bar + syntax highlight
- S132 splash: cold-launch eyes-on; muddy-blur fallback = drop `backdrop-filter`, keep flat `--bg @ 86%`
- S133 Whisper FFI: `winget install LLVM.LLVM` (admin) + `cargo build --release --features whisper-rs`. CPU first, CUDA pass second (`whisper-cuda` feature).
- S135 empty-pane: cold-launch w/ 2-pane persisted state → verify new card renders, buttons work
- S124 items (auto-compact, ctx stats) still in gate

**Next code lanes:**
1. Smoke gate → `/git-ship` v0.4.24-alpha (includes S135)
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
