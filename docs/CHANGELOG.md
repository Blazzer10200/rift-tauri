# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.104.0 — Your files, where you can find them

- **Working locally now saves to `Documents\Rift Workspace`.** No-folder chats used to read and write in a hidden AppData scratch dir — your files worked, but you could never find them. The local workspace now lives in your Documents folder (resolved the way Explorer does, so a OneDrive-redirected Documents works too). Anything already in the old hidden location is moved over automatically; if the move can't happen, the old location keeps working — nothing is ever stranded.
- **The "Working locally" welcome card now shows where your files live** — a click on the path opens the folder in Explorer, and the copy names the real folder instead of "a private scratch workspace".

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
