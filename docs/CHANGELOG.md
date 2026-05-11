# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `git log -- docs/CHANGELOG.md`.

## v0.2.13-alpha-test — 2026-05-11 — Rename + delete in browser panes

Right-click any file or directory in the Remote or Local pane → rename + delete are now first-class context-menu actions. Multi-select delete supported across both sides. Backed by 4 new Tauri commands + a stack-based recursive SFTP delete that won't blow the async stack on deep trees.

### Landed

- 4 new Tauri cmds: `remote_rename_path`, `remote_delete_paths`, `local_rename_path`, `local_delete_paths` (`src-tauri/src/lib.rs`).
- `delete_recursive_via` SFTP helper — stack-based file-then-dir-reverse walk, avoids async-recursion overflow (`src-tauri/src/sftp/mod.rs`).
- Browser panes: ctx menu entries + `.ctx-danger` red hover for destructive actions, native `window.prompt`/`confirm` (no custom modal needed), refresh after every op (`src/lib/components/browser/RemotePane.svelte`, `src/lib/components/browser/LocalPane.svelte`).
- Path-traversal guards on local rename + delete (reuses existing `reject_path_traversal`).
- Confirmed S21's `bridge_ack` was already wired (`DiagStage::BridgeAck` at `auto_sync.rs:1060`) — false positive in triage.
- Code-signing (audit H4) permanently dropped — not deferred.

### Verify

- `cargo check --release`: clean.
- `svelte-check`: 0 errors, 5 pre-existing a11y warnings unrelated to this work.

v0.2.12-alpha-test archived to git log.
