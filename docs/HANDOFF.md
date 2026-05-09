# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. WPF predecessor retired 2026-05-09 (S14). Path: `C:/AI Workflow/rift-tauri/`.

**Current state (post S15, v0.2.3-alpha):** UI port done + 42 audit findings landed (all 5 CRITICALs, 12 of 16 HIGHs, most MEDIUMs). Backend works end-to-end against the homelab FXServer (live test confirmed C2 TOFU + auto-connect). SSH commit signing live + verified by GitHub. Installed at `%LOCALAPPDATA%\Rift\rift-tauri.exe` w/ desktop shortcut. **18 audit findings remain** (mostly LOW/style + 2 latent POSIX-port items).

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

### Round-2 fix-pass (v0.2.3-alpha) — autopilot session continuation
Round-2 closed 14 more findings: H2 (refreshStatus error logging), H9 (askConfirm leak on unmount), M2 (path-traversal extended to local_list_dir/enqueue/resolve_conflict), M5 (mutex poison logging), M9 (loadServers atomic), M12 (typed openPath import), M18 (connCfg reactive), M21 (DriftReview selected pruning), L3 (velopack pinned to =0.0.1298), L12 (browser-tabs warn), L13 (Reupload Enter behavior).

L9 dual-crypto noted but deferred — `cargo tree -d` confirms `aws-lc-rs` enters via `rustls-platform-verifier`. Needs feature-flag work.

### Next session pickup
Smoke-test v0.2.3-alpha. Live C2 verified (user re-added Endure RP successfully — fingerprint pinned in canonical SHA256: form). 18 findings remain (audit doc on Desktop): mostly LOW (L2/L4/L7/L8/L10/L11/L14) + a few specific MEDIUMs (M4 atomic_write_json on Windows, M6 POSIX perms, M10 effect, M13 Bootstrap mid-chunk cancel, M16 drag-drop catch — note M16 patterns weren't found, may be obsolete).

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
