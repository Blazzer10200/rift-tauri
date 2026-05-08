# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## RESUME HERE — first read every new session

**Project:** WPF→Tauri migration of Rift. Sibling repo to `Blazzer10200/rift` (the WPF v13.55.x line still ships from there as the daily driver). User's daily app = WPF Rift. This project = the v14.0.0 future. WPF stays alive until Phase 6 cutover.

**Current phase:** Phase 0 complete (toolchain + framework + Velopack pipe proven). Awaiting **runtime gate** — user installs `RiftTauri-win-Setup.exe` from v0.1.0-alpha, fills in real server creds, hits List, confirms a remote dir shows. If gate passes → Phase 1. If fails → diagnose first.

**Next phase:** Phase 1 — port AutoSync engine + DriftScanner + state caches (RemoteStateCache, SyncSnapshot, ResourceDiscoveryCache, EditTrail) from C# to Rust. Riskiest backend piece — do FIRST. Test against the existing JSON files at `~/.rift/snapshot-*.json` etc. (Rust serde reads them 1:1 via matching `#[serde(rename = "...")]`). Then gate w/ `cargo test` parity vs xunit tests in `Rift Project/tests/Rift.Tests/`.

**Phase ordering** (per `Rift Project/docs/MIGRATION-STATUS.md`): Phase 1 backend → 2 UI shell → 3 two-pane browser → 4 sync surfaces → 5 edge cases → 6 ship v14.0.0.

## Session 1 — 2026-05-08 — Phase 0 stub shipped (v0.1.0-alpha)

Toolchain installed clean (Rust 1.95.0, MSVC Build Tools, npm 11.12.1, Tauri CLI 2.11.1, vpk 0.0.1298). Scaffolded `C:/AI Workflow/rift-tauri/` via Tauri 2.0 + Svelte 5 + TS template (`com.blazzer.rift`). Cargo.toml: russh 0.54 (ring backend, no NASM), russh-sftp 2.1, notify 8, velopack, tokio full, anyhow, thiserror, chrono. `lib.rs` has one Tauri command `sftp_list({host,port,user,key_path,remote_path})` → russh connect → sftp subsystem → return `[{name,is_dir,size,mtime}]`. `+page.svelte` = connect form + results table (Rift palette). Velopack `VelopackApp::build().run()` fires before Tauri builder.

**Gotchas recorded.** pnpm v11 `[ERR_PNPM_IGNORED_BUILDS]` blocks Tauri's pre-build install gate → switched to npm. `aws-lc-sys` needs NASM on Win → russh `default-features=false, features=["ring","rsa","flate2","async-trait"]`. `russh::Session::best_supported_rsa_hash()` returns `Result<Option<Option<HashAlg>>>` (triple-wrapped) → explicit `match Ok(Some(Some(h))) => Some(h)`. GitHub `PATCH /releases/<id>` 422s on empty repo → push initial commit before promoting from draft.

### State
csproj (Cargo) v0.1.0. cargo check ✓ · npm run check ✓ · npm run tauri build ✓ (MSI + NSIS Setup.exe). Released to `Blazzer10200/rift-tauri` v0.1.0-alpha (prerelease, marked latest, draft promoted). `main` branch pushed w/ initial commit + docs commit.

### CRITICAL DON'T-TOUCH (carries forward into Phase 1+)
- russh `ring` backend — never re-enable `aws-lc-rs` default (NASM blocker)
- npm runner (NOT pnpm) for Tauri build hooks
- `tauri.conf.json` `beforeBuildCommand: "npm run build"` + `beforeDevCommand: "npm run dev"`
- File-format compat w/ WPF Rift's `~/.rift/*.json` — Rust serde must read existing files 1:1 (don't change shapes)
- Velopack `VelopackApp::build().run()` MUST be the first call in `run()` — handles `--veloapp-install`/`--veloapp-updated` lifecycle args before Tauri spins up
