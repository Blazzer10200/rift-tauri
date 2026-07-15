# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.109.0 — The stream stops going dark

Sourced from a 5½-minute screen recording of a real working turn, where the UI sat visually frozen for ~3 minutes while real work happened underneath.

- **No more dead air.** During long turns the footer no longer collapses to a bare "Working…" between tools — it carries the last finished action ("Working… · Read Composer.svelte") and says "Thinking…" during mid-turn reasoning passes, not just the opening one.
- **Tool activity is framed now.** Work-line groups, the in-flight row, and edit batches sit on the same island-card language as shell blocks — full detail kept, chrome framed. Composer/STT fixes from the prior patch (dictation ghost overlap, mic engage bounce, effort micro-chip) ship here too.
- **The token meter never runs backward.** The live estimate holds its high-water mark when real usage lands lower (the 429→409 dips are gone), and the odometer roll no longer fades digits to near-invisible mid-animation.
- **Honest diff gutters.** Created files show real 1-based line numbers; edits (which carry no absolute file offset) drop the misleading 1..N column and keep the +/− marks.
- **Live plans actually tick.** The assistant now marks plan steps in_progress/completed the moment they change, so the plan widget fills during the turn instead of jumping 0/N → N/N at the very end.
- Search chips show `…/src/lib`-style scopes instead of full `C:\` paths; Copy/Retry under a finished turn are readable at rest.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
