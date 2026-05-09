# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.6-alpha — 2026-05-09 — End-to-end auto-sync unblocked: russh-sftp `write()` quirk fixed

First live multi-file sync test against the homelab FXServer surfaced a hard blocker: every upload returned `sync failed: write tmp …rift-tmp: No such file`. Root cause was russh-sftp 2.1.2's `session::write()` opening with `OpenFlags::WRITE` only (no `CREATE`, no `TRUNCATE`) — fine for overwriting an existing file, fails immediately when writing a fresh `.rift-tmp`. Same library quirk also bit `upload_bytes` (used by edit-trail + lock-presence heartbeats), where it would silently leave trailing garbage when a new payload was shorter than the existing remote file.

### Landed

- **`upload_atomic_via`** swapped `sftp.write()` → `sftp.create()` (`WRITE | CREATE | TRUNCATE`). File-handle scope ensures the SFTP close packet flushes before the rename. ([sftp/mod.rs:1024-1037](src-tauri/src/sftp/mod.rs#L1024-L1037))
- **`upload_bytes`** swapped to `sftp.create()` + `write_all`. Closes both the first-creation and short-payload-trailing-garbage cases on edit-trail and lock-presence writes. ([sftp/mod.rs:864-876](src-tauri/src/sftp/mod.rs#L864-L876))

### Verified end-to-end (live tests against FXServer @ 192.168.1.170)

- Single-file edit → synced byte-for-byte (`fxmanifest.lua` 0.2.0→0.2.2 round-trip).
- Burst write — 5 files in <1s — all 5 landed inside debounce window.
- 2 MB random binary — SHA1 match on both ends.
- Delete propagation — local `rm` → remote vanishes.
- All file types eligible (no allow-list / size cap); ignore module blocks only editor temp + build outputs.

### Found, not fixed

- **`.rift-lock` orphan after source delete** — lock-presence heartbeat not released on `Deleted` events. Minor, swept manually.
- **Logs not flushing in dev mode** — `~/.rift/rift-autosync.log` likely buffers until process exit. Diagnostic blind-spot.
- **Write-tool atomic save not seen by notify-rs** — Edit (in-place modify) reliable; tool-level atomic-rename creates may need IDE-real-save verification.

### Verify

- `cargo check`: clean
- end-to-end SFTP round-trips: passing

v0.2.5-alpha archived.
