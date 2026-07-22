# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.136.0 — Autocorrect that actually corrects

- **Autocorrect got a real dictionary** — ~3,800 common misspellings (generated from Wikipedia's curated list, filtered so it can never touch a valid English word), shared by typing-time correction and right-click → Auto-correct. Both paths worked before, but the 26-word starter list made them feel dead. Typing-time stays opt-in: composer settings → Autocorrect.
- **Typing no longer bounces the chat** — from the second draft line on, every keystroke jolted the transcript (worse per line). The composer now measures its height without disturbing the layout, and the transcript stays pinned as the box grows.
- **Sidebar fills with real history** — recent day-buckets show until the rail has a proper list (quiet date labels included), instead of one lonely chat floating over a huge void and "Show earlier".

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
