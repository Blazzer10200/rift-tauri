# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.147.0 — Opus 5 is here

- **Claude Opus 5.** Anthropic's newest, most capable Opus is now the default Opus in the picker — same price as 4.8, stronger at checking its own work and pushing a hard task through to done. Opus 4.8 moves into "More models" so anything pinned to it keeps running exactly as before.
- **Find on a page — `Ctrl+F`.** The in-app browser now has a real find bar: type to jump between matches, `Enter` / `Shift+Enter` to walk them, `Esc` to close. Native browser search, so it works on any page.
- **Zoom a page.** A zoom stepper in the browser's ⋯ menu scales just the page you're viewing (separate from the app-wide UI zoom), and it sticks as you click around.
- **Links stop vanishing.** Pages that open `target="_blank"` or `window.open` used to spawn an invisible popup that read as a dead click — those now just open in the dock you're looking at.
- **A friendlier empty browser.** The blank browser panel now explains what it's for at a glance — share a page with the assistant, hand it console errors, or have it open a page for you — with a search box and the shortcuts worth knowing.

## v0.146.0 — Fits your screen now

- **Zoom the whole app.** New UI scale control in Settings → Appearance (80%–150%), plus the shortcuts you'd expect: `Ctrl+=` / `Ctrl+-` to step, `Ctrl+0` to reset. Everything scales together — text, chrome, spacing — on any monitor.
- **The window remembers you.** Size, position, and maximized state persist across launches. If your saved spot is on a monitor that's no longer connected, Rift falls back to centered instead of restoring somewhere unreachable.
- **No more off-screen window on small or high-DPI screens.** The default 1600×1000 window overflowed 1366×768 laptops *and* 1080p displays at 125% scaling, hanging past the screen edge. It now shrinks to fit the monitor's work area before centering.

## v0.145.0 — Every turn leaves a trail

- **When something goes wrong, you can now see why.** Every turn gets one trace id that follows it across all three moving parts — the app, the Claude process, and the tools it drives — so a slow or failed turn reads as a single connected story in the Diagnostics console instead of three disconnected logs.
- **The quiet failures aren't quiet anymore.** The in-app browser now reports blocked or unsafe links, webview failures, and dev-server timeouts; the usage meter reports when the endpoint is unreachable or returns something unexpected — spots that used to fail silently and leave you guessing.
- **Sidebar polish.** The settings cog no longer pushes past the bottom edge of the rail, the notification bell lines up cleanly, and the active conversation title reads a touch bolder.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
