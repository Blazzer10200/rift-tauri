# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.105.1 — Dictation, unstuck

- **Spoken sentences now turn solid as you pause.** With the local engines (Parakeet/Whisper), dictated text used to stay dim until you stopped recording — then sat pulsing for up to a minute while the whole recording was re-transcribed and polished. Now each sentence commits about a second after you pause, while you keep talking.
- **Stopping is near-instant.** Only the last unfinished sentence is transcribed at stop, not the entire recording again.
- **The AI cleanup pass no longer blocks you.** The transcript is editable and sendable the moment you stop; the polish swaps in quietly a few seconds later — only if you haven't touched the text — with the same "restore raw" undo chip as before.
- **Voice commands now work mid-dictation on the local engines** — "scratch that", "send it", "new line" apply per sentence, matching the Web Speech engine.
- **Long dictations are safer:** committed sentences are kept as text, so only the sentence in flight is bounded by the 5-minute audio buffer.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
