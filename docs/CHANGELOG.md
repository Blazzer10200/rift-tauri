# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.125.0 — Stuck-turn + post-crash recovery fixes

Field-reported on a second user's machine (2026-07-17): a turn wedged forever on "Calling 0 questions", Stop wouldn't kill it, and after force-closing the app their chat wouldn't reopen and sidebar clicks stopped registering. All four are fixed:

- **Turns can no longer wedge on a hidden question.** In Bypass mode the model could invoke the CLI's built-in AskUserQuestion tool, which has no surface in Rift's headless CLI — the turn stalled forever waiting for an answer nobody could give. The tool is now stripped from the model's toolset in every permission mode (`--disallowed-tools`); Rift's own in-chat question card keeps working as before.
- **Stop now reaches a wedged warm child.** If the per-turn PID entry was already cleared, Stop silently "succeeded" while the warm CLI child kept running. It now falls back to the warm pool's PID and kills the real process tree.
- **A chat that fails to open says so — and can be retried.** A failed conversation load used to be swallowed (error routed to an invisible surface) and left the app pointing at a tab that no longer existed, which silently blocked every further click on that chat. Failures now show a toast with the real error, clear the stale pointer, and refresh the sidebar list; clicking again genuinely retries the load.
- **Sidebar clicks can't dead-end anymore.** The "already open" fast path now verifies the tab actually exists before short-circuiting — the root cause of "switching chats does nothing" after a crash. This also fixes the workspace switcher showing a stale project while the pane ran elsewhere (both fell out of the same broken tab state).

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
