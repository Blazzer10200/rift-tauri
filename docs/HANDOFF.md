# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## Session 17 — 2026-05-09 — Velopack + buddy onboarding + first public release

### Completed
- Velopack UI wired end-to-end: `UpdateDialog.svelte` (Bootstrap-pattern), global `updates.svelte.ts` runes store, sidebar `TabRail` pulse-dot pill, auto-popup on launch, Settings → About button — all reading from one shared store, single `<UpdateDialog/>` instance in AppShell
- Audit #4 (last open CRITICAL) closed — `auto_sync.rs:277` mpsc bounded at 2048 + `try_send` + warn on overflow
- Bridge token wired in `~/.rift/rift.json` — sync_done hot-reload now fires against FXServer `rift_bridge`
- `scripts/release.ps1` publish pipeline — version-lockstep preflight (Cargo.toml + package.json + tauri.conf.json), `npm run tauri build`, staging dir, `vpk pack`, `vpk upload github --publish --pre --token $(gh auth token)`
- v0.2.7-alpha published to private GitHub repo + all assets uploaded (Setup.exe, Portable.zip, .nupkg, manifest)
- Trey (TREYDAY) pubkey appended to FXServer CT120 `/home/blazzer/.ssh/authorized_keys`; `docs/AUTHORIZED_KEYS.md` ledger committed
- Repo flipped private (was accidentally public since creation). Releases now private-only — auto-update check will 404 for unauthenticated clients
- Local install upgraded to v0.2.7-alpha (clean metadata, full self-replace dance)

### Key Decisions
- Single global UpdateDialog instance in AppShell (not Settings) so it works on every tab
- Release version must match across 3 files: `Cargo.toml` + `package.json` + `tauri.conf.json`; release.ps1 enforces this
- Repo private → Velopack AutoSource hits 404 for unauth. Two paths: stay manual (Trey gets Setup.exe out-of-band) or split source/releases into two repos. Deferred to next session.

### Next Steps
1. Decide: stay manual update or two-repo split (private source + public releases-only repo) for working auto-update
2. Wire `apply_updates` Tauri command to enable the "Install & restart" button in UpdateDialog
3. Phase B/C/D buddy onboarding docs: README rewrite, `docs/ONBOARDING.md`, `docs/CONTRIBUTING.md`, `docs/rift.json.example` — do when Trey is back
4. Add Trey's GitHub handle as repo collaborator (handle unknown — confirm w/ him on return)
5. Deferred audit items: M6, L8, L9, L14 (scoped work, not blocking)

### Files Modified
- `src/lib/components/dialogs/UpdateDialog.svelte` (new), `src/lib/state/updates.svelte.ts` (new)
- `src/lib/components/AppShell.svelte`, `src/lib/components/shell/TabRail.svelte`, `src/lib/components/settings/Settings.svelte`
- `src-tauri/src/sync/auto_sync.rs:277-285` (bounded mpsc), `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `package.json`
- `scripts/release.ps1` (new), `docs/AUTHORIZED_KEYS.md` (new), `.gitignore`, `docs/CHANGELOG.md`, `docs/archive/CHANGELOG-archive.md`

---

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
