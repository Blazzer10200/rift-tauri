# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.17-alpha-test — 2026-05-11 — Rename/delete error visibility

Audit pass after v0.2.16 turned up the same swallowed-error pattern on the rename/delete commands. `remote_delete_paths` and `local_delete_paths` were capturing only a `bool` per path with `.is_ok()`, so partial failures surfaced as `"delete failed for 2/3 items"` with no reason — was it permissions, a lock, a missing file, a traversal block? Buddy would've had no way to tell.

### Landed

- `remote_rename_path` + `remote_delete_paths` + `local_rename_path` + `local_delete_paths` all take `app: AppHandle` and emit `autosync://activity` rows on every operation (success or fail), with the actual error string in the `action` field when it fails.
- Traversal-rejected delete paths now emit a `Block` kind row instead of being silently dropped.
- `basename_for_log` helper — keeps the activity rows readable (`"endure_skills/main.lua"` not `"/opt/fxserver/server/txData/Qbox_F8F761.base/resources/[endure]/endure_skills/main.lua"`).

### Verify

- `cargo check --release`: clean (3.84s incremental).
- `svelte-check`: 0 errors, 5 pre-existing a11y warnings unrelated.

v0.2.16-alpha-test archived to git log.
