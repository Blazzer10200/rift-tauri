# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older sessions in `docs/archive/HANDOFF-archive.md` and `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 119 — 2026-05-20 — Audit-batch sweep (26 issues, uncommitted)

25 fixes + 1 non-bug closed, pending /git-ship → v0.4.16-alpha. Full SHIPPED blockquotes in `docs/ISSUES.md` per item; summary by lane below.

- **Sync correctness:** #47 (apply_selected CT) · #49 (per-entry count delta) · #50 (process_entry select arm reorder) · #51 (rebaseline dir-segment ignore) · #52 (ToDeleteRemote fail snapshot.forget) · #42 closed as non-bug
- **I/O handles:** #53 (scan_drift snapshot-init close) · #58 (expand_download_jobs Result) · #59 / #60 (post-expansion re-validate) · #76 (delete_local_one floor) · #78 (FiveM bare-dir bypass) · #81 (snapshot save error log, partial) · #82 (heal_owned_dirs channel-close drain) · #85 (list batch retry timeout) · #87 (upload_bytes shutdown-on-err) · #88 (exec_bash eof on timeout) · #90 (shell_quote tab reject)
- **Assistant/MCP:** #62 (bridge tokens scoped: write-only gated by remote_shell) · #66 (stderr 64 KiB cap, tail-preserving) · #67 (child.id None warn) · #69 (MCP unauth flush+shutdown) · #71 (tool_grep streamed 8 KiB probe)
- **Cleanup:** #92 (DateTime::UNIX_EPOCH) · #100 (drop redundant lock release) · #104 (11 eprintln→log::debug)

**Verify:** `cargo check` clean (1 pre-existing `private_interfaces` warn in `update_service.rs:199`).

**Next:** `/git-ship` S119 batch → bump 0.4.15 → 0.4.16-alpha → CHANGELOG entry. Then Phase 6 OS-keychain (#37/#38/#9.3). Remaining open sync MEDs: #45 / #54 / #55 / #56 / #63 / #73 / #75 / #77 / #80 / #83 / #84 / #86 / #89.

---

## RESUME HERE — first read every new session

**Project:** `C:/AI Workflow/projects/rift-tauri/`. HEAD = **v0.4.15-alpha** + 25 uncommitted audit-batch fixes (S119). Next ship = v0.4.16-alpha. Tauri 2 + Svelte 5 + Rust + russh.

**Next session's first move:**
1. `/git-ship` S119 batch — CHANGELOG entry + 3-file bump 0.4.15 → 0.4.16-alpha. ~26 issues land at once.
2. #37 + #38 + #9.3 — Phase 6 OS-keychain (Stronghold / Tauri 2 secure-store).
3. Split Phase 3: resize handle on divider, `paneWidthPct` persistence, drag-FROM-pane.
4. Wave-2 FE MEDs: #146-#149 + #151-#177.

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
