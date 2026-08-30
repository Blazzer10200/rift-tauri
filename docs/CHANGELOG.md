# rift-tauri — Changelog

> Current release only. Older release notes remain in Git history and on the
> [GitHub releases page](https://github.com/Blazzer10200/rift-tauri/releases).

## v0.156.0 — Workspace and pane isolation

- Split panes now keep stable identities and explicit workspace ownership;
  switching focus, projects, or the **All** conversation view cannot rewrite a
  sibling pane's workspace, conversation, draft, provider settings, or browser.
- Unsaved split tabs restore their pane IDs, roots, drafts, model/effort, and
  permission modes after restart. Invalid persisted relationships and provider
  sessions bound to another workspace now fail closed.
- Async transcript, Git/GitHub, workspace metadata, speech vocabulary, browser,
  and file actions carry their originating pane/workspace/session context rather
  than consulting whichever pane is focused when a result arrives.
- OpenAI child commands now drain output concurrently and reap their process
  trees on cancellation or timeout. Native browser reads and navigation are
  restricted to the owning assistant session.
- Workspace/status context labels now follow the screen that owns them. The
  model picker opens on the pinned provider, including when that provider is
  offline, and production CSS no longer contains invalid Svelte-only selectors.

## Known issues

- Elevated windows cannot accept drag-and-drop from lower-integrity Explorer;
  use the attachment picker instead.
- Web Speech may mask profanity; the on-device Parakeet engine is verbatim.
