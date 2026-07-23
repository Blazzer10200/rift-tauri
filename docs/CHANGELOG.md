# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.143.0 — Plan mode, made real

- **Plans are a real approval now.** In plan mode the proposed plan lands as a rich card in the chat: read it, **Edit** it right there (your edited version is what gets built), ask for changes with **Refine** (the card re-proposes as r2, r3…), approve with build rights or ask-first rights, or **Discard**. Approving flips your permission mode back automatically and the *same turn* rolls straight into execution — no re-prompting.
- **Watch the plan being written.** The card streams the draft in with a typing caret as it's composed, instead of popping in finished.
- **A plan chip lives by the composer** tracking where the plan stands — ready → building → built — with the full plan one click away. It fades out shortly after the build lands.
- **Plan mode thinks harder.** Entering plan mode raises the thinking dial to at least High (your own setting comes back when you leave, and a toast tells you), and the model is steered to read the code it would touch before proposing. The permission menu now says what plan mode really does: you approve the plan before anything is built.

## v0.142.0 — Agents you can watch, autocorrect that thinks

- Intentional launch choreography, live agent cards (heartbeat, markdown results, token costs, nesting, persistence), per-pane model/effort in split view, dictionary-backed autocorrect, effort-slider dress-up, steer-marker image thumbs.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
