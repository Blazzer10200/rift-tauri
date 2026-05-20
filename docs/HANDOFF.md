# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 122 — 2026-05-20 — sync rebuild-detection + assistant UI polish

Full lane in `docs/CHANGELOG.md` (v0.4.19-alpha). Two clusters:

- **Sync (3 fixes against Vite/Webpack rebuild-loop):** `.rift-rebuild` sentinel (scanner + watch suppress while fresh), zero-config rebuild-pair detection (unlink+create within 1500ms in same dir w/ matching hashed-filename signature → drop the Deleted), periodic failed-slot retry on the existing 5s root tick.
- **Assistant UI:** `--chat-col-max` CSS var bumped column 720→1100px + recentered; EditDiff collapsed-by-default w/ click-to-expand; single-side diff renders as unified one-column; StatusHub merged into composer's top edge w/ shared border + Stop consolidated; status-label path truncation; tab bar active-indicator top-stripe.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.19-alpha** (S122 shipped). Tauri 2 + Svelte 5 + Rust + russh.

**Next session's first move:**
1. **FE Wave-2 MEDs** — #146-#149 + #151-#177 (30 items, biggest remaining lane).
2. **#81 (full DiagBus)** if backend-heavy session preferred.
3. UI polish continuation: right rail compression + grouping, titlebar hierarchy, context-window utilization indicator near composer model pill (`ISSUES.md #1` left-overs).
4. LOW/INFO tail polish.

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
