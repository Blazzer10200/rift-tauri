# rift-tauri — Changelog

> Live changelog = current entry only. Older entries archive to `archive/CHANGELOG-archive.md` on next bump.

## v0.1.0-alpha — 2026-05-08 — Phase 0 stub

Toolchain probe + foundation. **Not user-facing.** Connects to a server, lists a remote directory. Nothing else. Purpose = prove that the WPF→Tauri migration path works end-to-end before sinking real time.

### What works
- Tauri 2.0 + Svelte 5 + TS scaffold builds on Win11 (MSVC + WebView2)
- `russh` 0.54 + `russh-sftp` 2.1 — pure Rust SSH/SFTP, ring crypto backend (no NASM/C-asm deps)
- `velopack` crate — auto-update wiring at run() entry, same lifecycle as WPF Velopack
- `vpk pack` + `vpk upload github` — same release flow as WPF Rift, against `Blazzer10200/rift-tauri`
- One Tauri command (`sftp_list`) round-trips: Svelte form → Rust async → russh-sftp → JSON entries → Svelte table

### What doesn't work yet
- Everything else. No persistence, no auto-sync, no drift scan, no fingerprint check (TOFU only), no key-passphrase support, no bridge HTTP, no edit-in-place, no tabs, no nothing. Phase 1+ work.

### Files
- `src-tauri/Cargo.toml` — deps + russh ring backend feature flags
- `src-tauri/src/lib.rs` — Tauri commands + russh client handler + Velopack hook
- `src/routes/+page.svelte` — connect form + results table (Rift palette)
- `package.json` — pnpm config dropped, npm runner
- `src-tauri/tauri.conf.json` — `beforeBuildCommand`/`beforeDevCommand` switched to npm
- `.gitignore` — node_modules, dist, .svelte-kit, target, gen, publish, Releases

### State
Build green: `cargo check` ✓ · `npm run check` ✓ · `npm run tauri build` ✓ (MSI + NSIS Setup.exe). Released to `Blazzer10200/rift-tauri` v0.1.0-alpha (prerelease, marked latest).
