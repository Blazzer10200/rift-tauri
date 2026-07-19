# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.129.0 — Verified close-out: no leftovers in Task Manager

**Closing Rift now proves it cleaned up after itself.** Hitting ✕ shows a quick confirm, then a live checklist: stop any running AI turns → shut down background helpers → **verify nothing is left running** — a real process-tree scan that only reports "all clear" at an actual count of zero. If something survives, it's force-closed and the count is shown honestly; if the scan is unavailable, it says so instead of faking a green. Kills are strictly scoped to processes Rift owns — your own terminal `claude` sessions and other apps are never touched. (Press ✕ twice if the confirm ever gets stuck — the second one always closes.)

- **CLI updates now finish the job** — after updating the Claude CLI in-app, the banner offers "Restart Rift to finish". Rift locks onto the CLI's location at startup, so the restart is what guarantees every conversation runs on the new version. The restart does the same verified cleanup first.
- **Path-bug hardening** — the `\\?\` prefix fix class (v0.127–128) now runs through one shared chokepoint instead of per-site patches, so it can't silently regress in future code.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
