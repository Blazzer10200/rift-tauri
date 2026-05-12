# rift-tauri — Handoff

> Live handoff = current session block. Older sessions live in `git log -- docs/HANDOFF.md`.

## Session 28 — 2026-05-12 — v0.2.27-v0.2.30: data-loss recovery + WAN scan speedup + cleanup sweep

Post-S27 compaction continuation. Four ships, escalating from emergency (data-loss fix) to feature (server-side `find` listing) to UX (folder-delete) to hygiene (warnings sweep). All driven by Trey running on residential Tailscale uplink — every bug surfaced because WAN latency exposed assumptions that LAN testing hid.

### Ship trail (newest first)
- **v0.2.30** — Whole-codebase audit pass. Clippy → 0 warnings (was 1, `io_other_error` in [paths.rs](src-tauri/src/state/paths.rs)). Svelte-check → 2 warnings (was 5). Fixed `state_referenced_locally` in [Settings.svelte](src/lib/components/settings/Settings.svelte) via `untrack(() => initialSection)`. Added `onkeydown` Escape handler to ctxmenu wrappers in LocalPane/RemotePane — closes the menu + clears `a11y_click_events_have_key_events`. The 2 remaining `<section>` warnings persist b/c the `svelte-ignore a11y_no_noninteractive_element_interactions` directive doesn't suppress in current svelte-check — known quirk, not a defect. Zero TODO/FIXME/HACK. One legitimate `#[allow(dead_code)]` (SSH session-keeper, documented). No orphan modules.
- **v0.2.29** — Folder-delete fix. Deleting a FiveM resource dir locally surfaced `delete failed: ...: No such file` b/c `SftpClient::delete` only called `remove_file` (SFTP-spec: rejects dirs). Now probes `remote_stat` first; dirs → `delete_recursive_via`; missing remote → success (avoids re-queue loop).
- **v0.2.28** — Server-side `find`-exec listing in [sftp/mod.rs](src-tauri/src/sftp/mod.rs) `list_via_exec`. One SSH-exec round-trip per root vs N round-trips per directory in SFTP. On Trey's link (verified direct Tailscale, 25ms DERP-fallback only) scans dropped from 30-60s → ~1-3s. Prunes match `sync::ignore::ignored_directory_names()`. Falls back to SFTP worker path on per-root failure (no `find` on PATH, non-POSIX shell). Exit 0+1 both tolerated (mid-walk ENOENT shouldn't fail the whole scan).
- **v0.2.27** — **CRITICAL data-loss fix.** v0.2.26's post-rename `set_metadata(0o664)` was using `russh_sftp::protocol::FileAttributes::default()` which (in russh-sftp 2.1.2) returns `size: Some(0), mtime: Some(0), atime: Some(0), uid/gid: Some(0)`. SETSTAT honored those → every Trey upload truncated to 0 bytes + epoch-1970 mtime the instant rename completed. Three real files (`fxmanifest.lua`, `server/server.lua`, `client/client.lua` in `[endure]/endure_shooting/`) were zeroed in the live FiveM tree — Blazzer restored them via FiveM session. Fix: `FileAttributes::empty()` (all `None`) so SETSTAT only carries `permissions`. Bonus: scan Cancel now races `list_recursive_batch` via `tokio::select!` so clicking Cancel during the listing returns immediately instead of waiting 30-60s.

### Tailscale diagnostic (capture for future ref)
Trey's `tailscale status` confirmed `direct 69.50.245.28:41641, tx 285M rx 533M` — direct P2P, not DERP-relayed. `netcheck`: `UDP: true`, `MappingVariesByDestIP: false`, `PortMapping: UPnP+NAT-PMP+PCP`. His router is permissive; SFTP protocol overhead was the entire bottleneck. v0.2.28's exec-listing addresses that root cause.

### Verify (post-v0.2.30)
- `cargo check`: clean. `cargo clippy --no-deps`: clean. `svelte-check`: 0 errors, 2 warnings (svelte-ignore quirk, documented).
- Releases v0.2.27-v0.2.30 on `rift-releases`. All source commits pushed.

### Flagged for v0.2.31+
- **Token-slot race** in `register_scan_cancel` — overlapping `run_tick` mid-`force_pull_now` can shadow user op. Move to `Vec<CancellationToken>` w/ `cancel_all` if it bites.
- **Per-folder streaming during scan listing** — v0.2.28 collapsed listing time to ~1s on WAN, but if it grows large, instrument per-root completion for progress.
- **Pre-flight write probe** on autosync start (catch EACCES at connect time, not first push).
- **`svelte-ignore` non-suppression** on `<section>` a11y warnings — investigate svelte-kit/svelte-check version bump.

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. Path: `C:/AI Workflow/projects/rift-tauri/`.

**Current state (post S28):** **v0.2.30-alpha-test SHIPPED** to `rift-releases`. 4 ships this session (v0.2.27-v0.2.30). Trey running on Tailscale direct path; bidirectional sync verified working post-v0.2.27. Scan latency on his link dropped 30-60s → ~1-3s w/ v0.2.28. Folder deletes work post-v0.2.29. Codebase clean post-v0.2.30 (clippy 0, svelte-check 2 warnings).

**Next session likely entry points:**
1. Confirm Trey's v0.2.30 update landed + sync stays stable.
2. Pick next item from v0.2.31+ flagged list, or move to brainstorm items (per-resource sync mode, buddy presence).

## CRITICAL DON'T-TOUCH
- russh `ring` backend + reqwest `rustls` features only (NASM blocks aws-lc-rs)
- `~/.rift/*.json` compat — don't change rename rules; don't drop `serde(flatten) extra`
- `VelopackApp::build().run()` first call in `lib.rs::run()`
- `bundle.targets: ["nsis"]` while `-alpha`/`-beta` (MSI rejects non-numeric semver)
- DriftWatcher conflict-rename guard — never overwrite dirty local
- `.rift-trail.jsonl` ignore rule — pull→push loop reappears w/o it
- `GITHUB_OWNER`/`GITHUB_REPO` point at public `rift-releases`, NOT source repo
- `path_guard.rs` API frozen (`validate_remote_child`, `validate_local_child`) — `edit/in_place.rs` + lib cmds depend
- `rename_via` is strict (user-facing); `rename_overwriting_via` is ONLY for atomic upload tmp-swap
- **Source `.secrets/env.sh` first on ship/auth tasks** — Claude Code bash is non-interactive, won't auto-load
- **`current_scan_cancel` + `last_scan_entries` are std::sync::Mutex** (NOT tokio) — `kick_drift_reconcile` is sync and called from notify event handler; tokio Mutex `blocking_lock` panics there. Don't "fix" it.
- **`force_pull_now` dispatches from cache, NOT a fresh scan** — re-scanning makes it identical to Reconcile (SFTP listing is the cost). drift_watcher's 10s tick keeps cache fresh.
- **NEVER use `FileAttributes::default()` for SETSTAT** — it sends `size: Some(0)`, `mtime: Some(0)`, `atime: Some(0)`, `uid/gid: Some(0)` which the server honors → file truncation + epoch mtime. Always use `FileAttributes::empty()` and explicitly set only the fields you want to change. See v0.2.27 post-mortem.
- **`SftpClient::delete` routes by remote stat** — dirs go through `delete_recursive_via`. Don't shortcut back to `remove_file` for "files only" — the push pipeline can't distinguish file from dir deletes ahead of time. See v0.2.29.
