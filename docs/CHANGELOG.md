# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.122.0 — Compacting says so

- **/compact no longer hides behind "Working…".** While the CLI condenses your conversation, the turn now says what's actually happening: the header reads "Compacting conversation…", the footer shows "Summarizing older messages" with the live timer, and a short note explains that nothing is deleted — the full transcript stays put and the chat picks up right where it left off. The before → after context pill still lands when it finishes.
- **No false alarms mid-compact.** Compaction is legitimately silent until the summary lands; the stall watchdog ("Waiting on the model…") no longer fires during that quiet stretch.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
