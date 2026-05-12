# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.29-alpha-test — 2026-05-12 — Folder-delete fix (was failing on FiveM resource removals)

Deleting a FiveM resource directory locally (e.g. `rm -rf gt_zombies_qb`) was surfacing `delete failed: /opt/.../[endure]/gt_zombies_qb: No such file: No such file` in both Blazzer's and Trey's activity feeds. Local removal worked; remote did not — folder stayed orphaned server-side, drift queued the delete forever, never resolved.

Root cause: `SftpClient::delete` only called `remove_file`. SFTP's `remove_file` rejects directories (NO_SUCH_FILE / FAILURE depending on server), russh-sftp surfaces it as "No such file." The push pipeline doesn't distinguish file deletes from directory deletes — both arrive via the same `notify::Remove` event.

### Landed
- **`SftpClient::delete` now probes `remote_stat` first.** If the remote path is a directory, routes to `delete_recursive_via` (which already existed for explicit folder-tree deletes). Files still go straight to `remove_file` as before — no extra round-trip on the common case if the stat hits the directory branch first.
- **Non-existent remote = success.** If `remote_stat` returns `!exists` (local already deleted, remote already gone), report success so the local delete reconciles. Previously this could re-queue forever in some race orderings.

### Verify
- `cargo check`: clean (0.48s). `svelte-check`: 0 errors, 5 pre-existing a11y warnings.

v0.2.28 archived to git log.
