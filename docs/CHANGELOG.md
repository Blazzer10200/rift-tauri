# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.39-alpha-test — 2026-05-12 — Backend hardening + cleanup pass

Codex shipped 15 backend hardening items in parallel with a frontend audit cleanup. Path containment, symlink-safe recursion, structured per-path errors, dead-code sweep, and the v0.2.39+ backlog items (connect-time write probe, `.rift-lock` sweep).

### Backend
- Symlink-safe `delete_recursive_via` — lstat per child, refuses `/` / `.` / empty, never recurses through symlinked dirs.
- Rename collision split: `rename_via` preflights target existence + returns a clean collision error; only atomic upload tmp-swap calls `rename_overwriting_via`.
- Per-path `OpStatus { ok, error }` on remote/local delete commands — partial failures surface which path blocked.
- Profile path containment on remote rename/delete/list, local rename/delete, bootstrap, enqueue, and conflict resolve — destructive ops reject paths outside watched roots, drive prefixes, or `remote_root` itself.
- Connect-time write probe under `remote_root` — EACCES caught at connect, not on first push.
- `.rift-lock` cruft sweep on watch attach — local-user owned, age-gated, reuses lock parsing helpers.
- Autosync deadlock avoidance — engine cloned before `.await` to drop the autosync mutex across diagnostics + status calls.
- Retry map drop-after-final-failure (no longer grows indefinitely on chronic-failure files).
- 64-bit transport temp-id entropy + CRLF-safe `edit_trail` trim.
- `sftp/mod.rs` split into mod + list + transfer + ops + remote_exec (public API unchanged).
- `auto_sync/path.rs` extracted with tests; `watch.rs`/`flush.rs` left as `CODEX-FLAG` stubs (cross-cut private state).
- Dropped 3 dead pub fns (`ensure_remote_parent_dir`, `get_remote_folder_size`, `resource_name_for`).

### Frontend
- 5 reactivity / lifecycle audit fixes — AddServer destroy-guard on async IIFE, ConflictResolver redundant `$effect` removed (parent already remounts via `{#key}`), swallowed `catch {}` surfaced in `updates.svelte.ts`, Diagnostics fire-and-forget wire commented, `dirtyEdits` Set-replace invariant documented.
- 9 svelte-check warnings → 0. Fixed `splitEl` / `pickerEl` / `panelEl` non-`$state` reactivity, dropped dead `.val.ok` CSS, split compound `<!-- svelte-ignore -->` per rule (Svelte 5 quirk: compound directive silently drops 2nd rule).
- ConflictList S39 dev-seed stripped + `IS_DEV` gate removed.
- TwoPane many-tabs horizontal scroll + 20px edge fade mask.
- Dropped 5 unused global CSS rules (`.btn.lg`, `.pill.warn`, `.pill.xs`, `.vdivider`, `.count-pip.warn`).

### Repo
- `scripts/bg-backlog.sh` deleted (stale Session 30 backlog).
- `src-tauri/icons/{android,ios}/` deleted (desktop bundle only).
- `Releases/` pruned to last 5 versions (~194 MB freed).

### Verify
`cargo check` clean. `npm run check` 0/0/3994. `npm test` 6/6.
