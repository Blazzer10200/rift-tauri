# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.124.0 — Mid-turn steering

- **Talk to Claude while it works.** Sending a message during a live turn no longer parks it in a queue until the turn ends — it's injected straight into the running turn, and Claude reads it right after the current tool call finishes (same turn, same context, like the VSCode extension). A quiet "mid-turn" marker pins your message at the exact spot in the timeline where Claude read it.
- **The queue is now the fallback, not the default.** If a message can't be steered (turn just ended, no live session), it lands in the familiar queue and fires as its own turn — nothing is ever dropped.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
