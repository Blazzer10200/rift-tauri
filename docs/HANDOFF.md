# rift-tauri — Handoff

> Live handoff = current session block. Older sessions flow to `archive/HANDOFF-archive.md`.

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. WPF predecessor retired 2026-05-09 (S14). Path: `C:/AI Workflow/rift-tauri/`.

**Current state (post S15, source v0.2.4-alpha / installed binary v0.2.3-alpha):** UI port done + 46 audit findings landed (all 5 CRITICALs, 12 of 16 HIGHs, most MEDIUMs, 6 LOWs). Backend works end-to-end against the homelab FXServer (live test confirmed C2 TOFU + auto-connect). SSH commit signing live + verified by GitHub. **6 audit findings remain genuinely deferred** (POSIX-only / russh upstream / scoped-session work like Rust cancellation tokens). Installed at `%LOCALAPPDATA%\Rift\rift-tauri.exe` w/ desktop shortcut still at v0.2.3-alpha — next "build" command compiles + installs v0.2.4-alpha.

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

### Round-3 fix-pass (v0.2.4-alpha) — autopilot session continuation post-compaction
Round-3 closed 4 more findings + verified 3 already-correct. Autopilot while user AFK eating.
- **Closed:** M4 atomic_write_json hardening (sync_all + 5-attempt retry on Windows MoveFileExW sharing violations + tmp cleanup), M16 drag-drop catch logging in LocalPane+RemotePane, L11 disconnect listener doc comment, L2 dead utility types removed (`WithoutChild`/`WithoutChildren`/`WithoutChildrenOrChild` from `$lib/utils.ts`).
- **Verified no-op (already correct):** M10 selectedConflict prune effect already at AppShell.svelte:46-51; L7 capabilities already minimal; L10 Ctrl+P actually wired at AppShell.svelte:106.
- **Source bumped to 0.2.4-alpha but NOT built/installed** — user steered to dev-server-default workflow last session. Installed binary still at v0.2.3-alpha until next "build" command.

### Genuinely deferred (6 of original 60 — need scoped sessions, not autopilot)
- **M6** POSIX file perms — latent, no Windows impact.
- **M13** Bootstrap mid-chunk cancel — needs Rust cancellation token through download_paths. Chunk-level cancel already works.
- **L4** russh future-incompat — upstream crate.
- **L8** hostname binary on POSIX — latent.
- **L9** dual-crypto stack — needs rustls-platform-verifier feature-flag work to drop aws-lc-rs.
- **L14** ConflictResolver semantics — code-correct, audit asked behavioral verification only.

### Next session pickup
Run "build" if user wants v0.2.4-alpha installed. Otherwise smoke-test in dev. The audit is effectively closed for autopilot purposes — remaining 6 are scoped work.

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
