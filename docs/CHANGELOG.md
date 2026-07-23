# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.145.0 — Every turn leaves a trail

- **When something goes wrong, you can now see why.** Every turn gets one trace id that follows it across all three moving parts — the app, the Claude process, and the tools it drives — so a slow or failed turn reads as a single connected story in the Diagnostics console instead of three disconnected logs.
- **The quiet failures aren't quiet anymore.** The in-app browser now reports blocked or unsafe links, webview failures, and dev-server timeouts; the usage meter reports when the endpoint is unreachable or returns something unexpected — spots that used to fail silently and leave you guessing.
- **Sidebar polish.** The settings cog no longer pushes past the bottom edge of the rail, the notification bell lines up cleanly, and the active conversation title reads a touch bolder.

## v0.144.0 — The chassis update

- **The app has a place to live now.** The dark space behind the floating panels is a machined housing: a recessed, subtly accent-tinted surface the sidebar and main panel sit *into*, with real contact shadows where they meet it. Wider breathing room around the panels so the depth actually reads.
- **The rift is the brand — literally.** A faint accent seam runs in the gap between the two panels, and the new launch screen opens along it: the surface draws a line of light, then *parts in two* to reveal the app assembling underneath. The old glowing-logo loader is gone; the boot readout stays honest (real startup stages, no fake progress).
- **Lighter repo for contributors:** the README's product tour now streams from release assets instead of shipping 18MB of media in every clone.

## v0.143.0 — Plan mode, made real

- Plan proposals land as an editable approval card (Edit / Refine rN / Approve with or without build rights / Discard); approving flips your mode back and the same turn rolls into execution. Typing-caret draft reveal, composer plan chip (ready → building → built), plan mode floors the thinking dial to High with restore + toast.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
