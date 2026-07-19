# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.132.0 — The update-freeze, found and fixed

**This release hunts down the caught-on-video bug where Rift came back from an update sluggish, unclickable, and missing the conversation you were in.** That was three stacked problems, all fixed:

- **Updating mid-conversation no longer loses your message.** Rift used to write a conversation to disk only when the reply *finished* — so applying an update (or any crash) mid-turn threw away your message and the running reply. Conversations now save the instant you hit send, keep saving progressively during long turns, and the updater does a full save-and-wait for every open tab before swapping versions.
- **Sidebar times are honest again.** After an update, every conversation claimed it was from "just now" because the list sorted and labeled by last-*saved* time, which updates and tab-switching bump. It now uses the real last-activity time carried on each conversation.
- **A frozen app now heals itself.** The unclickable-frozen-window state comes from WebView2's renderer process hard-hanging below the app's code — nothing in the page can run, so no in-app fix can see it. Rift now runs a heartbeat watchdog: the page pings the backend every 3 seconds, and if a *focused* window goes silent for 45+ seconds the backend reloads the webview; if that reload can't land (a truly hung renderer blocks it), it kills the hung renderer process — matched strictly to this app instance — and reloads again on a fresh one. Recovery takes ~1–3 minutes instead of a trip to Task Manager, and your conversations restore from disk (which the first fix now guarantees is current). Verified live by deliberately hard-freezing the renderer and watching it come back. `window.confirm`/`alert` (which can block the page forever) are banned in favor of native dialogs.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
