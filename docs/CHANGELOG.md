# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.130.0 — The slash menu keeps up with Claude Code

**Rift now asks the CLI what it can do instead of assuming.** The `/` menu picks up every slash command your installed Claude Code actually supports — including ones that ship inside the CLI itself, like `/code-review` — and refreshes automatically as the CLI updates. They appear in a new "Claude Code" group, remembered between launches.

- **Plugin skills listed too** — skills from marketplace-installed Claude Code plugins now show in the menu alongside your own.
- **Nothing silently dropped anymore** — if a newer CLI streams a content type this Rift build doesn't know yet, the transcript shows a small "unsupported content skipped" note instead of losing it invisibly.
- **Early warning on CLI renames** — Rift logs a warning when a tool it enables no longer exists in the CLI, so upstream renames get caught before they turn into permission popups.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
