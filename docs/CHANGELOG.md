# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.110.0 — One turn, one bubble

- **Manual `/compact`.** Type `/compact` (or hit "Compact conversation" in the context-ring popover) to hand-compact a long chat — same CLI compaction the auto path uses, rendered as the usual boundary pill. Compaction-only turns no longer trip a false "Blank response" error (auto-compaction included).
- **No more split turns.** When a background task finished right after a reply, the turn used to close ("Worked for <1s") and then keep going in a second block underneath. A follow-up that starts within a few seconds now continues in the same bubble, with duration and cost summed.
- **Polling folds into one card.** An assistant waiting on something (build, port, CI) that re-runs the same command 3+ times shows ONE terminal card with a quiet "polled ×N" tally and the latest output — not a stack of near-identical blocks.
- **Sharper tool captions.** Pending searches say "searching…" instead of "?", and shell captions drop the `cd "long/path" &&` prefix and keep both ends of a long command visible.
- **Readable errors + toasts.** Failed tools render through the shared folding output block (long stack traces collapse with "Show more"); long toast details clamp to 2 lines and expand on click.
- **Fewer typed questions.** The assistant is nudged to end short choose/confirm questions with a clickable options card instead of prose you have to type an answer to.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
