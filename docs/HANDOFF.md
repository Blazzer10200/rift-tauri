# rift-tauri — Handoff

> Live handoff = current session block. Older sessions live in `git log -- docs/HANDOFF.md`.

## Session 22 — 2026-05-10 — Queue audit + rename/delete

### Completed
- **Queue audit.** Verified all S21 next-steps against live code. bridge_ack was already wired (false-positive in triage — `DiagStage::BridgeAck` at `auto_sync.rs:1060`). rename/delete stubs never existed — feature needed from scratch.
- **Doc-doctrine cleanup committed** (0562c4f). Dropped `docs/archive/` entirely; `git log -- docs/HANDOFF.md` is the history.
- **3 stale `.rift-lock` orphans** on FXServer CT 120 (`[endure]/endure_skills/`) — cleared via `pct exec 120`. `find` now returns empty.
- **Rename + Delete shipped** (0e7a48b). 4 new Tauri cmds (`remote_rename_path`, `remote_delete_paths`, `local_rename_path`, `local_delete_paths`) + recursive SFTP `delete_recursive_via`. Both browser panes wired w/ ctx menu entries + danger-hover styling. Multi-select delete works. cargo check + svelte-check clean.
- **Code-signing (audit H4) permanently dropped.** User decision 2026-05-10 — not deferred, gone. Saved to memory.

### Key Decisions
- `delete_recursive_via`: stack-based (files first, then `remove_dir` in reverse), avoids async recursion stack overflow on deep trees.
- Rename uses `window.prompt()`, delete uses `window.confirm()` — native dialogs, no custom modal needed.

### Next Steps
1. **Field-test rename/delete** — dev was launched but user signed off before clicking. Next session: launch dev, right-click any throwaway file in both local + remote panes, verify rename + single/multi/dir delete all refresh correctly.
2. **Field-test bridge_ack** — open Sync Inspector diag panel, save a watched Lua file, confirm `bridge_ping` → `bridge_ack` row w/ `success: true`. If warn-level, bridge resource isn't running in FXServer.
3. Buddy-system bidirectional sync test (explicitly deferred by user).

### Files Modified
- `src-tauri/src/sftp/mod.rs` — `delete_recursive` method + `delete_recursive_via` helper
- `src-tauri/src/lib.rs` — 4 new Tauri cmds + handler registration
- `src/lib/components/browser/RemotePane.svelte` — rename/delete fns + ctx menu items + `.ctx-danger` style
- `src/lib/components/browser/LocalPane.svelte` — same
- `CLAUDE.md`, `docs/HANDOFF.md`, `docs/archive/*` (deleted), `src-tauri/Cargo.lock` — doc cleanup commit

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. WPF predecessor retired 2026-05-09. Path: `C:/AI Workflow/rift-tauri/`.

**Current state (post S23 buddy handoff):** v0.2.18-alpha-test on `rift-releases`. Treyday onboarded as second client over tailscale. S23 caught 6 ship-blocking bugs: op-rail Delete stub (v0.2.14), upload/download dir-blindness (v0.2.15), manual-transfer-silence in Activity feed (v0.2.16), rename/delete error-swallowing (v0.2.17), pull→push loop from atomic-replace fs-events + bulk conflict resolve (v0.2.18). Loop fix: `AutoSyncEngine.recently_written` 5s suppression window covers the Delete+Modify fs-event burst that the `.rift-tmp` rule never caught. All ship-front bugs land via Velopack auto-update. Code-signing permanently off roadmap.

## CRITICAL DON'T-TOUCH
- russh `ring` backend + reqwest `rustls` features only (NASM blocks aws-lc-rs)
- `~/.rift/*.json` file-format compat — never change rename rules; never drop `serde(flatten) extra` on `RiftConfig`
- `VelopackApp::build().run()` first call in `lib.rs::run()`
- `bundle.targets: ["nsis"]` while versions carry `-alpha`/`-beta` (MSI rejects non-numeric semver)
- DriftWatcher conflict-rename guard MUST stay — never overwrite a dirty local file
- `.rift-trail.jsonl` ignore rule MUST stay — pull→push loop reappears instantly without it
- `GITHUB_OWNER`/`GITHUB_REPO` in `update_service.rs` point at public `rift-releases`, NOT source repo (private)
