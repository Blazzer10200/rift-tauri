# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.133.0 — Livelier "working" chrome

**A polish pass on the live status a turn shows while Claude is working** — the little "Working…" line at the top of a reply, the action/verb line at the bottom, and the elapsed·tokens readout.

- **The status now feels alive, not static.** When a turn starts, the status dot springs in with a soft bounce and then breathes a gentle accent halo the whole time it's running; the "Working…" word and the live action carry a slow accent-tinted shimmer; and the elapsed·tokens meter slides in a beat behind the rest so the row assembles as a small wave instead of appearing all at once.
- **No more doubled "Working…".** The top line and the bottom line used to show the same word stacked on top of each other. The top line now owns the turn's state ("Working…"), and the bottom line only speaks up when it has something different to say — the specific action running ("Reading HANDOFF.md"), a stall warning, or a prompt for you — otherwise it quietly shows just the last action and the meter.
- **Clearer sidebar wording.** The project scope selector's "All projects" now reads "All chats," since it filters the chat list — it never changed which folder you're working in, and the old wording made it look like it did.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
