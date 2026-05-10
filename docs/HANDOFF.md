# rift-tauri — Handoff

> Live handoff = current session block. Older sessions live in `git log -- docs/HANDOFF.md`.

## Session 21 — 2026-05-10 — Audit cleanup + auto-updater rebuild

### Completed
- **Audit cleanup pass.** Deleted orphan `ui/button` island + `utils.ts` + `__mocks__/` + scaffold SVGs + `state/discovery.rs` (Phase-6 reservation never wired) + `DiagState` struct (replaced by inline DiagStateDto in lib.rs). Removed 11 dead pub fns, downgraded 6 to private. Net **−410 LOC**.
- **Auto-updater REBUILT.** Discovered velopack-rust 0.0.1298's `AutoSource` for github URLs just hits `<url>/releases.win.json` as a flat static path — that 404s on real github releases. Wrote custom `GithubSource` (impls `UpdateSource` trait) that uses the GitHub REST API w/ `User-Agent` header, picks newest non-draft release w/ `allow_prerelease=true`, caches asset `browser_download_url`s by filename so nupkg downloads resolve. Added `ureq = "3"` as direct dep.
- **Banner UI.** Auto-popup-on-launch dialog → 32px top banner (`UpdateBanner.svelte`, accent-tinted, Details/Install/Dismiss). `updates.svelte.ts` dropped `launchPopupShown`, added `bannerDismissed`. Dialog kept as Details surface.

### Key Decisions
- `ALLOW_PRERELEASE=true` matches WPF `GithubSource(prerelease:true)`. Drop to false later for stable channel.
- `ureq` for the github API (User-Agent required); velopack's `download_url_to_file` for the asset download (works w/o UA on github asset CDN).

### Next Steps
1. **Test the new updater path live.** Local-feed dry-run via `RIFT_UPDATE_FEED` first, then ship a `0.2.10-alpha-test` and watch installed `0.2.9-alpha` pick it up.
2. Bridge fix verify (carried from S19) — `bridge_ack: success` on save.
3. Stale `.rift-lock` orphans (3 on FXServer from prior crashes).
4. Buddy-system bidirectional sync test.
5. Code-signing cert (audit H4) — pre-public-ship blocker.
6. Wire backend `rename_path` / `delete_paths` Tauri cmds for the ctx menu stubs.

### Files Modified
- `src-tauri/Cargo.toml` (ureq=3), `update_service.rs` (full rewrite), `Cargo.lock`
- 13 backend cleanups: `bootstrap`, `bridge`, `diagnostics`, `edit/in_place`, `local_fs`, `sftp`, `state/{mod,discovery}` (discovery deleted), `sync/{auto_sync,edit_trail,ignore,lock_presence}`, `transport/ssh_handler`
- Frontend: `AppShell.svelte`, `state/updates.svelte.ts`; new `UpdateBanner.svelte`; deleted `ui/button/`, `utils.{ts,test.ts}`, `__mocks__/`, scaffold SVGs

---

## RESUME HERE — first read every new session

**Project:** rift-tauri IS Rift. WPF predecessor retired 2026-05-09. Path: `C:/AI Workflow/rift-tauri/`.

**Current state (post S21):** v0.2.9-alpha shipped. Auto-updater rebuilt w/ proper `GithubSource` (the old `AutoSource` path was silently 404'ing — that's why "set up wrong" was the right hunch). Top banner replaces auto-popup dialog. Codebase trimmed by ~410 LOC of dead surface. All uncommitted on `main` — ready to commit + ship a test version next session.

## CRITICAL DON'T-TOUCH
- russh `ring` backend + reqwest `rustls` features only (NASM blocks aws-lc-rs)
- `~/.rift/*.json` file-format compat — never change rename rules; never drop `serde(flatten) extra` on `RiftConfig`
- `VelopackApp::build().run()` first call in `lib.rs::run()`
- `bundle.targets: ["nsis"]` while versions carry `-alpha`/`-beta` (MSI rejects non-numeric semver)
- DriftWatcher conflict-rename guard MUST stay — never overwrite a dirty local file
- `.rift-trail.jsonl` ignore rule MUST stay — pull→push loop reappears instantly without it
- `GITHUB_OWNER`/`GITHUB_REPO` in `update_service.rs` point at public `rift-releases`, NOT source repo (private)
