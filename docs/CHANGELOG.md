# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.131.0 — Boot that shows its work, and cleaner streaming

**Rift no longer opens on a dead, empty shell while it loads.** The conversation list, project switcher, status bar, home welcome, and "Jump back in" strip now shimmer as skeleton placeholders during the first load, then swap in place for the real thing — so a slow cold start reads as "loading" instead of "broken." Warm boots stay instant: the skeletons only appear if the load actually takes a moment, so nothing flickers when it's fast.

- **Cleaner streaming in the transcript** — fixed a display bug where a reply that ran text → tool → text could fuse two separate sentences together (`…real logic.Done.`) and render the tool card out of order. Text before and after a tool now stays as separate, correctly ordered blocks.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
