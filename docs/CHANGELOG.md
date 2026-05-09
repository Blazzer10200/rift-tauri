# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.8-alpha — 2026-05-09 — apply_updates wired + two-repo split + S17 soft-spot sweep

Update dialog button now actually installs. Auto-update path moved to a public sibling repo so unauthenticated clients can pull. Audit deferred items + onboarding docs cleaned up.

### Landed

- **`apply_updates` end-to-end** — `UpdateService.apply()` re-checks → `download_updates` → `apply_updates_and_restart` (blocking, exits the process). Tauri command stops AutoSync + tunnel before `spawn_blocking` so in-flight uploads don't die mid-transfer. Frontend store gets an `applying` state; dialog button no longer disabled. ([update_service.rs](../src-tauri/src/update_service.rs), [lib.rs](../src-tauri/src/lib.rs), [updates.svelte.ts](../src/lib/state/updates.svelte.ts), [UpdateDialog.svelte](../src/lib/components/dialogs/UpdateDialog.svelte))
- **Two-repo split for auto-update** — Releases now publish to public `Blazzer10200/rift-releases` (no airholes — Issues/Wiki/Projects/Discussions all off). Velopack-rust 0.0.1298 has no auth in `AutoSource`, so the public sibling is the only no-fork path. Source repo stays private. `release.ps1` threads `$releaseRepo` through preflight + `vpk upload` + post-publish verify.
- **Logger init** — `env_logger::Builder::from_env(...).try_init()` early in `run()`. All `log::info!/warn!` calls were silent no-ops before; `RUST_LOG=debug` now surfaces sync activity.
- **Onboarding docs** — README rewrite, `docs/ONBOARDING.md`, `docs/CONTRIBUTING.md`, `docs/rift.json.example`. Trey-targeted, ≤300 words each per project doc cap.
- **Audit hygiene** — L8 `sort_by_key` switched from `OsStr::len()` (WTF-16 on Win) to `.components().count()`. L9 ignore-path normalize allocates only when `\\` present (`Cow`). M6 + L14 confirmed already done.

### Verify

- `cargo check`: clean. `svelte-check`: clean (1 pre-existing warning unrelated). Existing e2e auto-sync paths untouched.

v0.2.7-alpha archived.
