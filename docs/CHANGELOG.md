# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.7-alpha — 2026-05-09 — Velopack self-update wired + audit #4 backpressure fix + buddy onboarding

First proper Velopack release. UI surface lets users see + trigger update checks; sidebar pill + auto-popup catch them on launch. Backend audit #4 (only critical still live) cleaned. Trey's pubkey added to the FXServer for shared-account access.

### Landed

- **Velopack UI** — `UpdateDialog.svelte` matches Bootstrap's variant-tinted icon + `.lead`/`.hint` typography, single global instance mounted in `AppShell`. Reads from new `updates.svelte.ts` runes-class store (state/info/dialogOpen + `checkOnLaunch()` one-time auto-popup). Settings → About has a "Check for updates" button; sidebar `TabRail` shows a pulse-dot pill when an update is available. Install button stubbed pending `apply_updates` Tauri command.
- **Audit #4 — bounded mpsc** — `notify` → tokio channel converted to `mpsc::channel(2048)` w/ `try_send` + `log::warn!` on overflow. Webpack/IDE rebuild bursts can't grow the queue unbounded under a stalled flush. ([sync/auto_sync.rs:277-285](../src-tauri/src/sync/auto_sync.rs#L277-L285))
- **Bridge token wired** — `~/.rift/rift.json` `bridgeToken` now set; sync_done callbacks fire against the FXServer's `rift_bridge` resource, enabling hot-reload on save.
- **Release pipeline** — `scripts/release.ps1`: version-sync preflight, `tauri build`, clean staging dir, `vpk pack`, `vpk upload github --publish` w/ auto `--pre` for alpha/beta/rc. Unsigned for now; signing deferred (audit H4).
- **Buddy onboarding** — `docs/AUTHORIZED_KEYS.md` ledger + Trey's pubkey appended to `/home/blazzer/.ssh/authorized_keys` on FXServer (CT 120). Defensive `.gitignore` globs for `src-tauri/src/state/` runtime artifacts.

### Verify

- `cargo check`: clean. `svelte-check`: 0 errors. Live e2e auto-sync: still passing.

v0.2.6-alpha archived.
