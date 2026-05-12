# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.31-alpha-test — 2026-05-12 — Directory perms parity (the other half of v0.2.26)

v0.2.26 chmod'd uploaded **files** to 0664. It never chmod'd uploaded **directories** — they were left at umask 0022 default (0755, group has no write). When one teammate created a resource folder, the other teammate couldn't push files into it because the dir itself wasn't group-writable. Symptom on Blazzer's screen: cascading "sync failed: create tmp ...rift-tmp: permission denied" rows pointing at `endure_zombies/client.lua` (a Trey-created dir).

### Landed
- **`mkdir_p_via` chmods each segment to 2775** after creating it ([sftp/mod.rs](src-tauri/src/sftp/mod.rs)). Setgid (2___) + group-writable (_775) makes new dirs immediately usable by everyone in the shared group. `FileAttributes::empty()` (NOT `default()` — that bug shipped in v0.2.26, fixed in v0.2.27) so SETSTAT only carries `permissions`. Best-effort: SETSTAT on a dir you don't own fails silently — no harm, no error toast.
- **`SftpClient::heal_owned_dirs(root)` SSH-exec sweep**: `find <root> -type d -user "$(id -un)" -exec chmod 2775 {} +`. One round-trip per watched root, fixes the entire backlog of dirs Rift created at the old 0755 default. Fires fire-and-forget every time `AutoSyncEngine::add_folder_watch` attaches a root — async, doesn't block watch setup, swallows errors.
- **Convergence model:** going-forward NEW dirs land at 2775 via mkdir_p (everyone's pushes work). EXISTING broken dirs get healed by whichever teammate owns them when their Rift attaches a watch on that resource. After both Blazzer + Trey relaunch on v0.2.31, the whole tree is at 2775.

### Verify
- `cargo check`: clean (1.69s). `svelte-check`: 0 errors, 2 warnings (svelte-ignore quirk, documented).

### What user needs to do
- Both teammates relaunch Rift after v0.2.31 installs. Each side's heal sweep then chmods every dir they own under all watched roots.

v0.2.30 archived to git log.
