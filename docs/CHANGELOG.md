# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.144.0 — The chassis update

- **The app has a place to live now.** The dark space behind the floating panels is a machined housing: a recessed, subtly accent-tinted surface the sidebar and main panel sit *into*, with real contact shadows where they meet it. Wider breathing room around the panels so the depth actually reads.
- **The rift is the brand — literally.** A faint accent seam runs in the gap between the two panels, and the new launch screen opens along it: the surface draws a line of light, then *parts in two* to reveal the app assembling underneath. The old glowing-logo loader is gone; the boot readout stays honest (real startup stages, no fake progress).
- **Lighter repo for contributors:** the README's product tour now streams from release assets instead of shipping 18MB of media in every clone.

## v0.143.0 — Plan mode, made real

- Plan proposals land as an editable approval card (Edit / Refine rN / Approve with or without build rights / Discard); approving flips your mode back and the same turn rolls into execution. Typing-caret draft reveal, composer plan chip (ready → building → built), plan mode floors the thinking dial to High with restore + toast.

## v0.142.0 — Agents you can watch, autocorrect that thinks

- Intentional launch choreography, live agent cards (heartbeat, markdown results, token costs, nesting, persistence), per-pane model/effort in split view, dictionary-backed autocorrect, effort-slider dress-up, steer-marker image thumbs.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
