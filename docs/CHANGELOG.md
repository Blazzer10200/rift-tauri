# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.42-alpha-test — 2026-05-12 — Connection liveness + real in-flight cancel + push visibility

Fixes the connection error you hit on Qbox, the hung pushes, and the dark Activity feed during pushes.

### Backend
- **Write probe bug fixed** (`sftp/transfer.rs:upload_bytes`): `create()` + `write_all()` never closed the SFTP file handle, so probe content didn't materialize server-side before `remove_file` ran → ENOENT. Added `f.shutdown().await` to flush + close. Also made probe cleanup best-effort (`sftp/ops.rs:probe_write_access`) — if create succeeded, write access is proven; a leftover probe file is harmless and shouldn't block the connection.
- **russh keepalive** in both `sftp/mod.rs:open_session` and `tunnel/mod.rs:start`: `keepalive_interval=20s`, `keepalive_max=3` → ~60s to detect a stalled server. Was `Config::default()` (zero keepalive) — half-dead TCP sockets hung indefinitely waiting on Windows' ~2hr OS-level timeout.
- **Real in-flight cancel** (`auto_sync.rs:process_entry`): each upload races against the cancel token via `tokio::select!`. When Stop fires, the upload future drops, russh stops emitting WRITE packets, the atomic-tmp file is left on the server (rename never ran so target is unchanged), and the entry requeues to dirty.
- Cancel token passed down `flush_batch → process_entry`. Pre-dispatch cancel check also catches entries queued in `futs` before they touch the socket.

### Frontend
- **Push activity parity**: `process_entry` now emits an "uploading…" / "deleting…" activity row at dispatch time, not just on completion. Hung pushes show a heartbeat per file instead of a dead modal.
- **StatusBar sync pill**: while `syncModal.busy && !syncModal.open` (i.e. you hit "Run in background"), a pulsing pill appears next to queue/locks showing mode (pulling/pushing/scanning). Click reopens the modal so you can hit Stop. "Run in background" is no longer a one-way trip.

### Research backing
russh's own `Config` struct exposes `keepalive_interval`, `keepalive_max`, `inactivity_timeout`. `tokio::select!` drops the losing future, which closes russh-sftp's file handle via `Drop` (caveat: in-flight packets already on the wire may still land server-side — atomic rename pattern protects against torn target files).

### Verify
`svelte-check` 0/0 · `cargo check` clean · `vitest` 6/6.
