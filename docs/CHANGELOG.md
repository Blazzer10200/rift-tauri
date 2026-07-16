# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.112.0 — Updates that arrive while they're still news

- **Delta updates are back on.** Updating now downloads a small binary patch against your installed version instead of the whole app — the release pipeline diffs each release against the previous one (first-release and fallback cases still ship the full package automatically).
- **Hotfixes reach you fast.** Rift now re-checks for updates every 45 minutes (was every 6 hours) and also checks when you bring the window back into focus (politely debounced) — so a just-published fix shows its update pill within minutes for anyone actively using the app.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
