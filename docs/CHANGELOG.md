# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.123.0 — No more ghost turns

- **Blank ghost turns are gone.** When you stop a turn and keep chatting, the Claude CLI quietly injects its own "Continue from where you left off." resume turn behind the scenes; with nothing to add, it answers itself with a suppressed "No response requested." — which Rift used to render as a dead blank bubble with a scary "Blank response" error. Rift now recognizes these no-op resume turns and drops them silently — you'll never see them.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
