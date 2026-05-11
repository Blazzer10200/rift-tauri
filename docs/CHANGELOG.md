# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.14-alpha-test — 2026-05-11 — Op-rail Delete + auto-refresh

Caught two UX bugs while wiring buddy onboarding. Op-rail `Delete` button was a stale stub from before S22 (`flash("Delete is not yet wired — coming in a backend follow-up")`), even though the underlying `remote_delete_paths` / `local_delete_paths` cmds already existed and were wired into the right-click ctx menus. Also, upload/download didn't refresh the destination pane — every transfer required a manual refresh-circle click to see the new files.

### Landed

- Op-rail `Delete` now invokes the same delete cmds the ctx menu uses, with confirm dialog + flash result. Handles mixed local + remote selection in one shot (`src/lib/components/browser/TwoPane.svelte`).
- New `refreshKey: number` prop on `LocalPane` + `RemotePane`. Bumping it triggers a fresh `load()` via the existing `$effect`.
- TwoPane bumps `remoteRefreshKey` after successful upload, `localRefreshKey` after successful download, both after delete. No more manual refresh after transfers.

### Verify

- `svelte-check`: 0 errors, 5 pre-existing a11y warnings unrelated.
- `cargo check --release`: no Rust changes this version.

v0.2.13-alpha-test archived to git log.
