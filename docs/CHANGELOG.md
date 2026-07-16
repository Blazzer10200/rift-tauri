# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.111.0 — Mission control + dictation that keeps up

- **ActivityHud — see and control what's running.** The pinned agents bar is now a full activity periscope: alongside live sub-agents it lists every shell process running under the current turn (real command, PID, elapsed). Hover a shell row for a per-process ✕ kill (PID-verified against the session's own process tree — it can never touch anything else), and a **Stop** button in the bar ends the whole turn, agents included (they can't die individually — the label is honest about that).
- **Dictation start is snappier.** The mic used to re-scan your workspace (git branch + up to 4000 filenames) on every single press to build its vocabulary hint; that scan is now cached for 2 minutes and runs in parallel with mic init on a miss. Repeat presses start about as fast as the mic hardware allows.
- **Early release no longer loses your words.** Releasing a push-to-talk key (hold-Ctrl+D / hold-Space) while the mic was still initializing used to silently no-op the stop — the mic kept recording and the ghosted words never committed. Stop/cancel now wait out an in-flight start, so a release is always a stop and the spoken tail always lands in the draft.
- **Cleaner shell captions in the HUD.** Rows strip the CLI's internal launcher prefix (`bash.exe -c -l SNAPSHOT_FILE=…`) and lead with the command you'd recognize.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
