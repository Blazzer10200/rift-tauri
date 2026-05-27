# Rift

A Tauri + SvelteKit + Rust dev workspace launcher for FiveM / RedM developers. SFTP-first sync, two-way drift detection, in-place remote edit, and a sidebar live-status pill — all running locally against your FXServer.

Pure-Rust SSH/SFTP via `russh` (no libssh2 / OpenSSL deps). NSIS installer for Windows. Self-update via Velopack.

## Status

Pre-1.0 (`-alpha`). The core sync loop is verified end-to-end against a live FXServer. Distribution (auto-update, code signing) is the next milestone. See [`docs/HANDOFF.md`](docs/HANDOFF.md) for the current state.

## Quick links

- **Install, build, contribute:** [`docs/DEVELOPING.md`](docs/DEVELOPING.md)
- **Profile config example:** [`docs/rift.json.example`](docs/rift.json.example)
- **Release history:** [`docs/CHANGELOG.md`](docs/CHANGELOG.md)
- **Issue tracker:** [`docs/ISSUES.md`](docs/ISSUES.md)
- **Accepted security advisories:** [`docs/SECURITY.md`](docs/SECURITY.md)

## What it does

- Watches a local folder, mirrors changes to a remote SFTP path, debounced + batched.
- Atomic uploads via `.rift-tmp` + rename so the running server never sees a half-written file.
- Drift scanner highlights local↔remote divergence before you start syncing.
- Edit-in-place: open a remote file, edit locally, save → uploads back. No manual SCP.
- Optional bridge token + SSH-tunneled HTTP probe for `txAdmin` / `rift_bridge` integration.

## Platforms

Windows 11 (primary). macOS / Linux builds are technically buildable from source but are not packaged or tested.

## License

Private — not yet published for general distribution.
