# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.27-alpha-test — 2026-05-12 — Critical data-loss fix + immediate Cancel

**CRITICAL:** v0.2.26's post-upload `set_metadata(0o664)` call was silently truncating every uploaded file to zero bytes and clobbering mtime to epoch 1970. Root cause: `russh_sftp::protocol::FileAttributes::default()` returns `size: Some(0)`, `mtime: Some(0)`, `atime: Some(0)`, `uid/gid: Some(0)` — NOT all-None as the name implies. The SETSTAT packet was sending those values to the server, which honored them. Every Trey-side push during v0.2.26 destroyed its own content the moment the rename finished. `fxmanifest.lua`, `server/server.lua`, `client/client.lua` all went to 0 bytes + Jan 1 1970 mtime in the live FiveM tree.

### Landed
- **`FileAttributes::empty()` instead of `default()`** in [sftp/mod.rs](src-tauri/src/sftp/mod.rs) `upload_atomic_via`. `empty()` returns all `None`s so the SETSTAT packet only carries `permissions = 0o664` — partial-update semantics per SFTP spec. Comment-blocked w/ post-mortem to prevent regression.
- **Cancel scan takes effect immediately on slow links.** `scan_with_cancel` used to only check the cancel token *between folders*, but the slow part (`list_recursive_batch`, 30-60s on Trey's Tailscale) ran *before* the loop. Wrapped the listing in `tokio::select!` against `ct.cancelled()` so clicking Cancel during the listing returns immediately instead of waiting 60s for natural completion. Applies to Reconcile + the 10s background `drift_watcher::run_tick`.

### Recovery
- Three zeroed files (`fxmanifest.lua`, `server/server.lua`, `client/client.lua` in `[endure]/endure_shooting/`) need to be restored from local copies. After v0.2.27 push lands, Blazzer or Trey re-pushing any local non-empty copy fills them correctly.

### Verify
- `cargo check`: clean. `svelte-check`: 0 errors, 5 pre-existing a11y warnings.

v0.2.26 archived to git log.
