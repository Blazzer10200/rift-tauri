# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. WPF predecessor retired 2026-05-09. Path: `C:/AI Workflow/rift-tauri/`.

**Current state (post S16):** **v0.2.6-alpha shipped to GitHub + installed.** Source = `%LOCALAPPDATA%\Rift\rift-tauri.exe` = `0.2.6-alpha`. Auto-sync now end-to-end verified live against the homelab FXServer (single-file edit, 5-file burst write, 2 MB binary SHA1-match, delete propagation — all clean). Audit officially closed for autopilot purposes; remaining 4 deferreds (M6, L8, L9, L14) are all scoped work.

## Session 16 — 2026-05-09 — End-to-end auto-sync unblocked + ship v0.2.6-alpha

### Root-caused + fixed
**russh-sftp 2.1.2 `session::write()` opens with `OpenFlags::WRITE` only** — no CREATE, no TRUNCATE. Every fresh `.rift-tmp` upload failed `NO_SUCH_FILE`. Same code path also left trailing garbage in `upload_bytes` when new payloads were shorter than existing files.

Fix: swapped both call sites to `sftp.create()` (`WRITE | CREATE | TRUNCATE`):
- `upload_atomic_via` ([sftp/mod.rs:1024-1037](src-tauri/src/sftp/mod.rs#L1024-L1037)) — primary file-sync path.
- `upload_bytes` ([sftp/mod.rs:864-876](src-tauri/src/sftp/mod.rs#L864-L876)) — used by edit-trail + lock-presence heartbeats.

### Verified live (FXServer @ 192.168.1.170)
4-test autonomous pass: single-file edit ✓, 5-file burst ✓, 2 MB binary SHA1 match ✓, delete propagation ✓. All file types eligible (no allow-list / size cap); ignore module blocks junk only.

### Found, not fixed (deferred)
- **`.rift-lock` orphan after source delete** — lock-presence heartbeat not released on `Deleted` events. Minor, swept manually during teardown.
- **Logs not flushing in dev mode** — `~/.rift/rift-autosync.log` buffers until process exit. Diagnostic blind-spot.
- **Write-tool atomic save not seen by notify-rs** — Edit (in-place modify) reliable; tool-level atomic-rename creates may need IDE-real-save verification.

### Operational gotcha (added to project CLAUDE.md)
**Don't run `cargo check` while `npm run tauri dev` is alive** — it kills the running Rift Dev process via incremental-rebuild collision. Restart Rift Dev manually after Rust edits.

### Shipped
Commit `29da529` pushed to `main`. Bundles v0.2.5-alpha agent-driven sweep (M13 cancel-tokens, russh 0.60, Vitest harness) + v0.2.6-alpha russh-sftp fix.

## CRITICAL DON'T-TOUCH
- russh `ring` backend (NASM blocker on aws-lc-rs)
- reqwest `rustls` features only
- npm runner, NOT pnpm
- `~/.rift/*.json` file-format compat — never change rename rules; never drop `serde(flatten) extra` on `RiftConfig`
- `VelopackApp::build().run()` first call in `lib.rs::run()`
- shadcn-svelte components own themselves in `src/lib/components/ui/`
- `bundle.targets: ["nsis"]` while versions carry `-alpha`/`-beta` (MSI rejects non-numeric semver)
- Tauri 2: `core:default` lacks `window:allow-*` — explicit perms required for custom titlebar
- WPF fingerprint format: `<keytype> <bits> <b64>` w/o `SHA256:` — substring match strips the prefix to handle both shapes
- russh-sftp `session::write()` is WRITE-only; use `session::create()` for any "write a new file" path
