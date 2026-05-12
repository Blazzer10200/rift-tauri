# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.32-alpha-test — 2026-05-12 — Phantom-conflict killer (upload pre-flight SHA collapse)

Trey's v0.2.31 diagnostic export showed **53 phantom conflicts** in `[ox]/ox_lib/web/build/` + `[ox]/ox_doorlock/web/build/` (FiveM UI bundle fonts, `index.html`, `index-{hash}.js/css`). Every conflict had `local_size === remote_size === last_known_size` — bytes identical on both sides, only mtimes differing. The drift scanner already collapses this shape via SHA-equality on scan, but the **upload pre-flight** at `auto_sync.rs:1494-1535` had no such guard: any mtime drift past the snapshot baseline raised CONFLICT, even when content was provably unchanged. The watcher fires on local mtime-only touches (npm rebuilds, git checkout, our own SETSTAT calls bumping ctime); pre-flight then saw remote moved (Blazzer or v0.2.31 heal touching it server-side) and CONFLICT'd 53 files Trey never actually edited.

### Landed
- **SHA-equality collapse in upload pre-flight** ([auto_sync.rs](src-tauri/src/sync/auto_sync.rs)). When remote_changed fires but local + remote + last-known sizes all match AND a baseline SHA exists, compute local SHA (cheap, local IO) — if it matches baseline, fetch remote SHA (one SSH-exec). If remote SHA also matches, content never changed; refresh baseline mtime and drop the push as `synced (mtime jitter)`. Real edits (size differs OR local SHA changed) skip the check entirely — common path unaffected. Emits `phantom-conflict collapsed (SHA-equal)` diag event for observability.

### Convergence (what user does)
- Both relaunch Rift to pick up v0.2.32. **Conflicts are in-memory only** (not persisted to `~/.rift/`) — Trey's 53 disappear on restart. New pre-flight prevents them recurring.
- Combined w/ v0.2.31's perm-heal sweep + mkdir_p chmod, both teammates' next sync session should be conflict-free + permission-clean.

### Verify
- `cargo check`: clean (1.20s). `svelte-check`: 0 errors, 2 warnings (svelte-ignore quirk, documented).

v0.2.31 archived to git log.
