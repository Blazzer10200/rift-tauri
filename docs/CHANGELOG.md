# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.103.0 — Scroll that sticks

- **Auto-scroll actually follows now.** The transcript's follow-pin could silently die mid-turn: the pane's own scroll adjustments echo back asynchronously, and when a streaming burst grew the content in between, that echo read as "the user scrolled away" and killed follow. Programmatic pins are now tracked and their echoes ignored — only a real user scroll breaks away. Verified live against bursty multi-tool turns and heavy conversation loads.
- **Sending snaps you back.** Composing from up-thread left your own message (and the whole reply) streaming off-screen. Sending now always re-latches the pane to the bottom.
- **"Jump to latest" can't lose the race.** Clicking it mid-stream used to glide toward a stale target, land short, and un-latch follow. The glide now latches follow for its duration; a wheel flick still cancels it.
- **Tab switches stopped sweeping.** Restoring a tab's scroll position no longer smooth-glides from the *previous* tab's position across the new tab's content — it's instant.
- **Browser scroll-anchoring disabled in the transcript** — it fought the follow-pin whenever content above the viewport resized (late syntax highlight, output reveals), causing micro-jumps.
- **Dictation no longer overlaps the placeholder.** Interim ghost words rendered on top of "What are we working on today?"; while dictating an empty composer now shows a quiet "Listening…" until your first words arrive.
- **Work-group rows animate in** like every other block instead of popping (with the same reduced-motion opt-out).

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
