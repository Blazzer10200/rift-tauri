# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.146.0 — Fits your screen now

- **Zoom the whole app.** New UI scale control in Settings → Appearance (80%–150%), plus the shortcuts you'd expect: `Ctrl+=` / `Ctrl+-` to step, `Ctrl+0` to reset. Everything scales together — text, chrome, spacing — on any monitor.
- **The window remembers you.** Size, position, and maximized state persist across launches. If your saved spot is on a monitor that's no longer connected, Rift falls back to centered instead of restoring somewhere unreachable.
- **No more off-screen window on small or high-DPI screens.** The default 1600×1000 window overflowed 1366×768 laptops *and* 1080p displays at 125% scaling, hanging past the screen edge. It now shrinks to fit the monitor's work area before centering.

## v0.145.0 — Every turn leaves a trail

- **When something goes wrong, you can now see why.** Every turn gets one trace id that follows it across all three moving parts — the app, the Claude process, and the tools it drives — so a slow or failed turn reads as a single connected story in the Diagnostics console instead of three disconnected logs.
- **The quiet failures aren't quiet anymore.** The in-app browser now reports blocked or unsafe links, webview failures, and dev-server timeouts; the usage meter reports when the endpoint is unreachable or returns something unexpected — spots that used to fail silently and leave you guessing.
- **Sidebar polish.** The settings cog no longer pushes past the bottom edge of the rail, the notification bell lines up cleanly, and the active conversation title reads a touch bolder.

## v0.144.0 — The chassis update

- **The app has a place to live now.** The dark space behind the floating panels is a machined housing: a recessed, subtly accent-tinted surface the sidebar and main panel sit *into*, with real contact shadows where they meet it. Wider breathing room around the panels so the depth actually reads.
- **The rift is the brand — literally.** A faint accent seam runs in the gap between the two panels, and the new launch screen opens along it: the surface draws a line of light, then *parts in two* to reveal the app assembling underneath. The old glowing-logo loader is gone; the boot readout stays honest (real startup stages, no fake progress).
- **Lighter repo for contributors:** the README's product tour now streams from release assets instead of shipping 18MB of media in every clone.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
