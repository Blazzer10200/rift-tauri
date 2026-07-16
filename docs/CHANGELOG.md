# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.114.0 — One island language, a real mission control

- **The island look is now the whole app's language.** One shared recipe (tokens + two tiers: docked and floating) drives the sidebar, main surface, settings cards, home tiles, and the workspace hub — translucent tint + hairline everywhere, so the canvas glow reads through every surface instead of dying on opaque grey cards.
- **The status bar joined the main island.** It's the island's footer now — rounded corners, hairline seam — not a strip glued to the window edge.
- **The Workspace hub grew up.** It opens at the top every time, headed "Workspace" with the active project name (the warm greeting stays on the chat home — no more twin greetings). Project cards, the activity dashboard, and the news zone all speak the island dialect.
- **"What's new in AI" is one tile now.** Freshness + refresh live in the strip's header instead of a toolbar floating below it, and per-release expanders say "N more notes" so they don't fight the "Show more releases" button.
- **Summarize this week in AI says what it costs.** The button is amber with "Runs one Claude turn — billed like a chat message" right in the row — accent means free, amber means spends.
- **The title bar went fully quiet.** The left cluster is just the sidebar toggle when collapsed — new-chat and search live in the sidebar and palette (Ctrl+N / Ctrl+K unchanged).

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
