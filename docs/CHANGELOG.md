# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.116.0 — Deep-clean pass: sturdier downloads, honest agent cards

- **Voice-model downloads can't hang forever anymore.** A stalled connection (half-open Wi-Fi, blocked host) now errors out cleanly after 90 seconds with a visible message instead of sitting on a frozen progress bar.
- **Sub-agent cards stay cards.** A persisted `Task` agent spawn could previously get folded into a collapsed "N tools" group in old transcripts; agent cards are now always rendered first-class, matching live behavior.
- **Dev launcher fixed for stock Windows PowerShell** — `run-dev-deelevated.ps1` was unrunnable under PS 5.1 (encoding + stderr traps); it now works from any shell, elevated or not.
- **Full-repo audit, clean bill of health.** Backend, frontend, FE↔BE contracts, dependency and dead-code sweep, and docs all deep-reviewed: every lockstep verified, no critical findings, all 610 frontend + 182 backend tests green, zero console errors on a live tour.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
