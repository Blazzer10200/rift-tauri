# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.15-alpha-test — 2026-05-11 — Directory upload/download recursion

Treyday hit it first: right-click "Download to local" on a bracket directory failed because `download_paths` only ever supported file-shaped jobs. Adding a remote bracket directly to a fresh workspace was the most natural first move, and the app silently broke for it. Same blind spot existed on the upload side. Both fixed.

### Landed

- `expand_download_jobs` helper — stats every job; for directories, walks via `list_recursive` (max_depth 32) and emits one file-shaped job per leaf, with parent local dirs pre-created so the batch writer doesn't race on `create_dir_all` per file.
- `expand_upload_jobs` helper — local-side mirror using `walkdir`. Remote parent dirs auto-created by `upload_files_batch`'s existing per-file `mkdir_p`.
- Both `download_paths` and `upload_paths` now route through the expansion before dispatching to the batch transfer.
- Flash messages updated to report `N/M files` from the expanded count, not the original (now-meaningless) job count.

### Verify

- `cargo check --release`: clean (3.85s incremental).
- `svelte-check`: 0 errors, 5 pre-existing a11y warnings unrelated.

v0.2.14-alpha-test archived to git log.
