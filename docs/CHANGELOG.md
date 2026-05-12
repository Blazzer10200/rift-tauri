# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.40-alpha-test — 2026-05-12 — Hotfix: list-on-remote-root regression

v0.2.39 shipped the S43 path-containment hardening but applied the strict destructive-op guard to `remote_list_dir` too. Browsing the remote root (the standard entry point for navigation) errored with `remote list guard: refusing to operate on remote_root itself: …`. Browser pane wedged at the root.

### Fix
- Added `path_guard::validate_remote_listable` — permissive variant that allows `path == remote_root` since the browser's entry point IS the root. Still rejects `..`, backslash, and out-of-tree escapes.
- `remote_list_dir` (lib.rs:810) now uses the listable guard. All destructive ops (rename, delete, upload, download, edit-in-place) keep `validate_remote_child` strict.

Docstring on the original guard already said "Browser may navigate anywhere; only destructive ops are gated" — implementation just didn't match. List/browse path now matches that intent.

### Verify
`cargo check` clean. No other call-site changes needed.
