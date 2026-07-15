# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.108.0 — Sidebar peek, properly minimized

- **The hover-peek is now a compact flyout, not the whole sidebar.** Hovering the panel glyph while the sidebar is collapsed opens a small card — project switcher, New chat + search, your 8 most-recent chats, footer nav — that hugs its content instead of spanning the full window height. Same island look as the pinned sidebar, just the quick-access version of it.
- **A quiet "All chats · N" seam** at the bottom of the peek's list pins the sidebar open for the full scoped history (the This-project/All segment and "Show earlier" stay full-sidebar-only).
- **The peek breathes below the topbar** — it floats in with real separation from the top edge instead of hugging the window chrome, and it no longer repeats the Rift brand row the topbar cluster already shows.
- Pinned sidebar unchanged: full history, scope segment, resize, everything as before.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
