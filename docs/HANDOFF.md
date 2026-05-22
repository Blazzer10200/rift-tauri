# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. History via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 134 — 2026-05-22 — Settings page rebuild (uncommitted — v0.4.23-alpha)

**Settings.svelte rewrite (1846L → ~1500L).** 8 nav rows → 7 (SSH keys folded into new **Network** section as header action next to *Add server*). Unified pattern across all sections: `.set-group` (uppercase title bar + hairline rows) + `.set-row` (left label/hint, right control). Lifted from polished Terminal section. New section subtitles. Outer PageHeader dropped from [SettingsPage.svelte](src/lib/components/settings/SettingsPage.svelte) (section title now carries the page).

**Bugs fixed.** Appearance kbd grid was unreadable — each `Ctrl+T` row flowed 4 kids into a 2-col grid so each `<kbd>` block-stacked. Wrapped combos in `.kbd-combo`. Killed "More coming soon" placeholder. Speech 12-lang picker stacked vertically in a narrow right column; switched language + engine rows to `set-row-stack`.

**Verify.** `npm run check` 0/0 (after dropping 2 dead CSS selectors). CDP-verified all 7 sections (Appearance / Terminal / Accessibility / Assistant / Speech / Network / About), active-state nav ramp, kbd grid renders correctly, engine flip mounts/unmounts whisper cards, dyslexia-disabled state propagates dim across child rows.

**Files.** [Settings.svelte](src/lib/components/settings/Settings.svelte), [SettingsPage.svelte](src/lib/components/settings/SettingsPage.svelte), package.json + Cargo.toml + tauri.conf.json bumped 0.4.22 → 0.4.23-alpha (lockstep), docs/CHANGELOG + HANDOFF consolidated v0.4.23-alpha entry.

---

## Session 133 — 2026-05-22 — Whisper STT backend (feature-gated) + dual-engine UI (uncommitted — v0.4.23-alpha)

Full detail in CHANGELOG v0.4.23-alpha entry. Compact summary:
- Backend under [src-tauri/src/stt/](src-tauri/src/stt/) — 5 modules (audio/vad/whisper/model_manager/cleanup), `!Send` cpal::Stream on dedicated thread, 14 commands + 5 events in [lib.rs](src-tauri/src/lib.rs). Feature-gated default = stub.
- Frontend dual-engine in [stt.svelte.ts](src/lib/state/stt.svelte.ts) + Speech section UI.
- **Whisper FFI UNVERIFIED live** — needs `winget install LLVM.LLVM` (admin) + `cargo build --release --features whisper-rs`. CDP verified all UI surfaces (cpal returned 3 real mics).

---

## Sessions 129–132 — 2026-05-21 / 2026-05-22 — collapsed (uncommitted — v0.4.23-alpha)

Full session detail folded into CHANGELOG v0.4.23-alpha entry. One-liners:
- **S132** SplashOverlay Glass Reveal cold-boot — `SplashOverlay.svelte`, app.html bg fix, AppShell `onMount` slimmed. Visual UNVERIFIED live.
- **S131** Assistant chat UI overhaul — turn rail, atmosphere, Shiki, agent + TodoWrite card branches, image attachments, DPI fixes. Shiki UNVERIFIED w/ live fenced block.
- **S130** UI audit fixes (6 files) + ISSUES.md prune (979 → 882L). CDP-verified.
- **S129** 5 assistant MCP sync tools + `is_dead_session_error()` wedge fix + `SyncActivityBanner.svelte`. Wedge fix + banner live-UNVERIFIED.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.22-alpha** shipped; **v0.4.23-alpha staged** (versions bumped, CHANGELOG/HANDOFF consolidated, ready to `/git-ship`). Tauri 2 + Svelte 5 + Rust + russh.

**Uncommitted batch (S129–S134, all rolling into v0.4.23-alpha):**
- S129: 5 MCP sync tools + wedge fix + SyncActivityBanner
- S130: UI audit fixes + ISSUES.md prune
- S131: Full assistant chat UI overhaul (rail-spans-turn, Shiki, atmosphere, agent card, image attachments, DPI fixes)
- S132: SplashOverlay Glass Reveal cold-boot intro + app.html bg fix + AppShell onMount slimmed
- S133: Whisper STT backend (feature-gated) + dual-engine Speech settings UI + Composer mic engine-aware
- S134: Settings page rebuild (this session)

**Smoke gate (open — do before shipping):**
- S129 banner: active + error-with-wedge **UNVERIFIED live** — drop network 90s, edit file, push → banner should go red + Reconnect
- S131 Shiki: **UNVERIFIED with live message** — send a msg with fenced code block (```rust / ```bash) → confirm header bar + syntax highlight render
- S132 splash: **visual UNVERIFIED** — needs cold-launch eyes-on; muddy-blur fallback = drop `backdrop-filter`, keep flat `--bg @ 86%`
- S133 Whisper FFI: **UNVERIFIED live** — needs LLVM install + `cargo build --features whisper-rs`. Settings UI + cpal mic enumeration CDP-verified; download/transcribe round-trip pending. CPU build first (LLVM only), CUDA second pass (CUDA Toolkit + `whisper-cuda` feature) for 10× speedup.
- S124 items (auto-compact, ctx stats) still in gate

**Next code lanes:**
1. Smoke gate (above) → `/git-ship` v0.4.23-alpha
2. Wave-2 audit bugs: #146 `mutateStreaming` O(n) rebuild (HIGH — fixes stagger animation safety too), #147 thinking dedup, #148 tab-switch race, #149 delete/openTab race
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
