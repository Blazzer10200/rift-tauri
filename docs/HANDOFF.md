# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. WPF predecessor retired 2026-05-09 (S14). Path: `C:/AI Workflow/rift-tauri/`.

**Current state (post S15, v0.2.2-alpha):** UI port done + 28 audit findings landed (incl. all 5 CRITICALs). Backend works end-to-end against the homelab FXServer. SSH commit signing live + verified by GitHub. Installed at `%LOCALAPPDATA%\Rift\rift-tauri.exe` w/ desktop shortcut.

## Session 15 — 2026-05-09 — Audit fix-pass + connection wiring + signing

### Two CRITICAL bugs the prior audit missed (both fixed this session)
- **WPF→Tauri host-key fingerprint format mismatch:** russh substring match expected `SHA256:<b64>` substring, but WPF Rift writes `<keytype> <bits> <b64>` w/o the `SHA256:` prefix. Bricked all migrated profiles. Fixed by stripping the `SHA256:` prefix before substring-checking the bare b64 in `ssh_handler.rs::check_server_key`.
- **Frontend had no `connect()` / never invoked `start_autosync`:** disconnect existed, connect didn't. UI permanently showed "Offline." Added `connection.connect()` + auto-connect on `select()` + Connect command in palette + clickable Auto-sync pill in TabRail.

### Audit fix-pass (28 of 60 findings landed)
Critical: C1 CSP set, C2 TOFU prompt (probe + confirm), C3 bootstrap_list_files non-ASCII, C4 wireEvents race + try/catch, C5 EditChangedEvent serverKey filter.
High: H1 openBootstrap guard, H3+H12 editor_for narrowed lock, H5 default_ssh_key_path, H6 stale-phase toasts, H7 lockForBasename → full-path, H11 reject_path_traversal, H13 toast handle leaks, H15 stepper bypass, H16 ssh-preview NaN.
Medium/cosmetic: M3 doc, M11 reset, M14/M19 Bootstrap, M7 $derived.by, M8/L1/L15/L16/M15.
Doc-only: H4 Velopack pinning TODO at call site (real fix needs signing keypair).
L5 verified: GITHUB_REPO_URL = Blazzer10200/rift-tauri ✅.

### Repo / signing
- SSH commit signing active (gpg.format=ssh, ~/.ssh/id_ed25519). All commits 0df5b2f→ HEAD verified. Pre-signing commit 6e9d5f1 stays unverified (no force-push).
- Bumped Cargo.toml/package.json/tauri.conf.json → 0.2.2-alpha. Built + installed + iconcache busted.

### Next session pickup
Smoke-test the v0.2.2-alpha install end-to-end (esp. C2 first-connect TOFU prompt — delete the Endure RP fingerprint via Edit, reconnect, verify the dialog appears). 32 audit findings remain (mostly LOW — see `Desktop/rift-tauri-debug-2026-05-09.md`).

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
