# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 121 — 2026-05-20 — Phase 6 OS-keychain + backend HIGH/MED close-out (19 issues)

Full lane breakdown in `docs/CHANGELOG.md` (v0.4.18-alpha entry). Issue numbers (19):

- **Phase 6 (3):** #37 #9.3 #38 (keyring crate + Windows DACL on mcp-config)
- **Backend HIGH/MED tail (14):** #45 #63 #102 #103 #106 #108 #112 #113 #119 #120 #125 #127 #129 #131
- **Hygiene (2):** #27 #32 (plus #26 + #101 verified non-bug / already-shipped)

`cargo check` clean (same pre-existing `private_interfaces` warn).

**Backend HIGH/MED tier is now closed.** Remaining open backend: **#81 (full DiagBus — heavy)** is the only meaty one; the rest is LOW/INFO tail. **FE Wave-2 MEDs #146-#149 + #151-#177 untouched — that's the biggest remaining lane** (~30 items).

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.18-alpha** (S121 shipped). Tauri 2 + Svelte 5 + Rust + russh.

**Next session's first move:**
1. **FE Wave-2 MEDs** — #146-#149 + #151-#177 (30 items, biggest remaining lane).
2. **#81 (full DiagBus)** if backend-heavy session preferred.
3. LOW/INFO tail polish.

**Phase 6 keychain runtime verification on first launch:** old plaintext `bridgeToken` / `apiKey` values get auto-lifted to Windows Credential Manager on first `RiftConfig::load()` + `assistant::load_config()` call. Check `cmdkey /list:rift` for `rift/bridge.<server_key>` + `rift/assistant.api_key` entries. JSON files post-migration should NOT contain `bridgeToken` / `apiKey` fields.

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
