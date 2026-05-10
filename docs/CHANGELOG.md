# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.10-alpha-test — 2026-05-10 — Auto-updater rebuild + audit cleanup

Live test of the rebuilt updater. The old velopack `AutoSource` path was silently 404'ing on real GitHub releases (it just hits `<url>/releases.win.json` as a flat static path); installed `0.2.9-alpha` clients should pick this build up via the new GitHub-API source.

### Landed

- **Auto-updater rebuilt** — custom `GithubSource` impls velopack's `UpdateSource` trait. Hits the GitHub REST API w/ `User-Agent` header (required), picks newest non-draft release w/ `allow_prerelease=true`, caches asset `browser_download_url`s by filename so nupkg downloads resolve. `ureq = "3"` added as direct dep (velopack pulls it transitively but w/o UA, which github API rejects). ([update_service.rs](../src-tauri/src/update_service.rs))
- **Update banner UI** — auto-popup-on-launch dialog replaced w/ 32px accent-tinted top banner (Details / Install / Dismiss). Dialog kept as Details surface. `updates.svelte.ts` dropped `launchPopupShown`, added `bannerDismissed`. ([UpdateBanner.svelte](../src/lib/components/UpdateBanner.svelte), [updates.svelte.ts](../src/lib/state/updates.svelte.ts), [AppShell.svelte](../src/lib/components/AppShell.svelte))
- **Audit cleanup — net −410 LOC.** Deleted orphan `ui/button` island, `utils.ts` + test, `__mocks__/`, scaffold SVGs, `state/discovery.rs` (Phase-6 reservation never wired), `DiagState` struct (replaced by inline `DiagStateDto` in lib.rs). Removed 11 dead `pub` fns, downgraded 6 to private. Touched `bootstrap`, `bridge`, `diagnostics`, `edit/in_place`, `local_fs`, `sftp`, `state/mod`, `sync/{auto_sync, edit_trail, ignore, lock_presence}`, `transport/ssh_handler`.

### Verify

- `cargo check`: clean. `svelte-check`: 0 errors.
- `ALLOW_PRERELEASE=true` matches WPF `GithubSource(prerelease:true)` — drop to false later for stable channel.

v0.2.9-alpha archived.
