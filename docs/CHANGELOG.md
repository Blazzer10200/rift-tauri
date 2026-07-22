# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.138.0 — The long-turn freeze is fixed

- **Fixed the recurring "app glitches out mid-turn" bug (#100)** — on long, heavy turns (hundreds of tool blocks), the live transcript could wedge: blank `$`/`PS>` rows with just a cursor, frozen "0s" pills, a stale footer, and eventually the whole app going unresponsive until restart. Root cause was render-side, never your data — every stream token re-rendered *every* block in the turn, each shell block re-ran syntax highlighting from scratch, and background-tab timer throttling starved the typewriter animation into permanent blanks. Four fixes:
  - Syntax highlighting is now cached — repeated highlights of the same command are free.
  - Unchanged blocks now keep their identity across stream updates, so only the block that actually changed re-renders (the big one — per-token render cost no longer grows with turn length).
  - The command typewriter now catches up on wall-clock time instead of counting starved timer ticks, snaps complete the moment a command finishes, and skips the animation entirely when the window is hidden.
  - Streamed text now lands immediately while the window is hidden instead of parking on animation frames and flooding in when you tab back.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
