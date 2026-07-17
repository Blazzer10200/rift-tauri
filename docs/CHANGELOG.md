# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.119.0 — Tabs you can see + a full-app tune-up

- **Chat tabs are finally visible.** A quiet "N tabs" pill now sits in the titlebar whenever you have more than one chat tab open. Click it to see every tab — switch, close, or open a new one by mouse. (Before this, Ctrl+T made an invisible tab and Ctrl+Tab was the only way back.)
- **GitHub chip sees release runs.** Tag-triggered CI runs (your releases) now show up in the branch popover instead of silently missing, and a failed run tells you exactly which job and step broke — the "ask Rift to fix it" prompt carries that detail too.
- **Failures speak up.** Attaching an image that can't be read, or a dictation engine that fails to start, now shows a clear message instead of doing nothing.
- **Under-the-hood hardening** from a full-app review: fixed a rare leak that could strand a ~450MB background helper after a mid-turn eviction, smoother streaming on very large tool calls, and a batch of internal dead code removed.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
