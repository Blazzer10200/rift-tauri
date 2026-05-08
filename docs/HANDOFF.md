# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## Session 1 — 2026-05-08 — Phase 0 stub shipped (v0.1.0-alpha)

WPF→Tauri migration kicked off. Phase 0 = "prove the toolchain end-to-end before sinking real time." Goal met: stub installs through Velopack-Rust, the same `vpk pack` + GitHub release flow used by WPF Rift works identically against `Blazzer10200/rift-tauri`.

**What landed.** Toolchain installed clean (Rust 1.95.0, MSVC Build Tools, npm, Tauri CLI 2.11.1, pnpm dropped → npm). Scaffolded `C:/AI Workflow/rift-tauri/` via `pnpm create tauri-app` (Svelte+TS template, `com.blazzer.rift` identifier). Cargo.toml gets russh 0.54 (ring backend — bypasses NASM/aws-lc-sys C-asm dep), russh-sftp 2.1, notify 8, velopack 0.0.x, tokio full, anyhow, thiserror, chrono, async-trait. `lib.rs` has one Tauri command `sftp_list({host, port, user, key_path, remote_path})` that connects via russh, opens an sftp subsystem, reads the dir, returns `[{name, is_dir, size, mtime}]`. Plus `app_version()` so the UI can show what's running. `+page.svelte` is a connect form + results table (Rift palette: violet `#8B6BE6`, surface `#0F0F12`). `velopack::VelopackApp::build().run()` fires before `tauri::Builder::default()` — same lifecycle pattern as WPF.

**Gotchas hit + recorded.** pnpm v11's `[ERR_PNPM_IGNORED_BUILDS]` blocks Tauri's pre-build install gate even after `onlyBuiltDependencies` config — switched to npm, problem gone. `aws-lc-sys` needs NASM on Windows — switched russh to `default-features = false, features = ["ring", "rsa", "flate2", "async-trait"]`. `russh::Session::best_supported_rsa_hash()` returns `Result<Option<Option<HashAlg>>>` (triple-wrapped) — explicit `match Ok(Some(Some(h))) => Some(h)` was the cleanest fix. GitHub `PATCH /releases/<id>` 422s on empty repo — pushed initial commit before promoting from draft.

**Phase 0 GATE (runtime — needs you):** install `https://github.com/Blazzer10200/rift-tauri/releases/download/v0.1.0-alpha/RiftTauri-win-Setup.exe` on this machine. Open the app. Type host/port/user/key-path of your real server, hit List, confirm a remote directory appears. Then I bump to v0.1.0.1-alpha, ship, and confirm the auto-update banner fires on your installed copy. **If runtime test passes → Phase 1 (port AutoSync engine to Rust). If it fails → diagnose before continuing.**

### Open / next session
- **Phase 0 runtime gate** — pending your install + remote-list test.
- **Phase 1** — port AutoSync engine (FSW → debounce → upload → bridge ping). Riskiest backend piece — do first, test against existing snapshot/state JSON files from `~/.rift/`.
- **Phase 2** — UI shell: top bar, server picker, sidebar nav, connect/disconnect flow.
- **Phase 3+** — two-pane browser, drift, status hero, edge cases (per `Rift Project/docs/MIGRATION-STATUS.md`).

### State
v0.1.0-alpha live at https://github.com/Blazzer10200/rift-tauri/releases/tag/v0.1.0-alpha (prerelease, marked latest). `cargo check` green w/ all deps. `npm run tauri build` produces MSI + NSIS Setup.exe successfully. WPF Rift v13.55.29 still alive on `Blazzer10200/rift`, untouched. Both projects coexist; v14 cutover happens at end of Phase 6.
