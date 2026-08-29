# rift-tauri — Changelog

> Current release only. Older release notes remain in Git history and on the
> [GitHub releases page](https://github.com/Blazzer10200/rift-tauri/releases).

## v0.155.0 — Organized output and queue flow

- Completed turns now keep the final answer in the open and fold commands,
  reasoning, and other process evidence into one expandable **Work completed**
  receipt instead of repeating completion badges and footers.
- Long command results use progressive previews in the transcript and open the
  complete output in a focused, copyable viewer, eliminating nested transcript
  scrollbars without hiding access to evidence.
- Queued follow-ups now live in one ordered, collapsible **Next up** shelf with
  a single local count, automatic-send context, and quieter edit/remove actions.
- The composer no longer pulses while a turn is active, duplicates queue state,
  or surfaces a second plan-status chip beside the model. Workspace and branch
  context are combined into one compact breadcrumb.
- Stream rendering settings now describe the answer-first receipt hierarchy and
  process-detail controls accurately.
- The canonical frontend test gate now uses a bounded four-worker pool, avoiding
  a reproduced Vitest shutdown stall without forcing slow serial execution.

## Known issues

- Elevated windows cannot accept drag-and-drop from lower-integrity Explorer;
  use the attachment picker instead.
- Web Speech may mask profanity; the on-device Parakeet engine is verbatim.
