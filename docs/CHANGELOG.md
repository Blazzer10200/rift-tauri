# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.134.1 — Startup crash fix

**Fixes a blank-screen crash on launch introduced in v0.134.0.**

- **Fixed: app could tear down to a blank screen on launch** if it was closed while sitting on the new Diagnostics page. Two parts of the app raced to initialize diagnostics at the same moment, attaching duplicate event listeners — every log line arrived twice and the UI crashed. Initialization is now single-flight; the page boots cleanly.
- Dev-only: new stream gallery (`Ctrl+Alt+G`) — a one-click replay that streams every transcript block kind through the real pipeline, for UI work on tool/command/code blocks.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
