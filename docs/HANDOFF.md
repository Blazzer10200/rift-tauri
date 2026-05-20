# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 120 — 2026-05-20 — Wave-2 backend MED+LOW sweep (~40 issues, shipped)

Full lane breakdown in `docs/CHANGELOG.md` (v0.4.17-alpha entry). Issue numbers (40):

- **MEDs (14):** #54 #55 #56 #68 #70 #72 #73 #75 #77 #79 #80 #83 #84 #86 #91
- **LOWs (24):** #93 #94 #95 #97 #98 #101 #105 #110 #111 #114 #116 #117 #118 #121 #122 #123 #124 #126 #128 #130 #132 #133 #136
- **INFOs (2):** #137 #138

`cargo check` clean (1 pre-existing `private_interfaces` warn in `update_service.rs:199`).

**Remaining open backend** for next picks: #45 #63 #81 (full DiagBus) #102 #103 #106 #108 #112 #113 #119 #120 #125 #127 #129 #131. Phase 6 OS-keychain (#37/#38/#9.3) is the next big lift. FE Wave-2 MEDs #146-#149 + #151-#177 untouched.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.17-alpha** (S120 shipped). Tauri 2 + Svelte 5 + Rust + russh.

**Next session's first move:**
1. Phase 6 OS-keychain (#37 / #38 / #9.3) — Stronghold / Tauri 2 secure-store.
2. Wave-2 FE MEDs: #146-#149 + #151-#177.
3. Remaining backend list above (S120 session block).

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
- Settings is workspace (kbd 9), `Ctrl+,` flips; do NOT reintroduce slideover scrim.
- Assistant scrollbar: `.scroll` + `.strip` BOTH `scrollbar-width: none` — don't reintroduce `scrollbar-gutter: stable`.
- AssistantPage `onMount` auto-fires `newTab()` if `openTabs.length === 0`. Don't reintroduce empty-tabs CTA.
- `UpdateService` managed Tauri state — `download_update` then `apply_pending_update`.
- **`tauri.conf.json` `dragDropEnabled: false`** — removing breaks cross-region HTML5 DnD. Rift has no file-drop Tauri events, cost = zero.
- **AssistantPane drop handlers on `.pane` outer div only** — never move to inner `.drop-zone` overlays; loses the continuous-preventDefault chain.
- **`composerDraft` stays store-level** — moving to per-pane requires Composer rewire across ~30 refs; not worth it.
