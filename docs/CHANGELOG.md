# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.121.0 — Every pane gets its own brain

- **Model choice is per-chat now.** Each tab and split pane pins its own provider + model — switching brains in one pane no longer rewires the chat sitting next to it. The pill, picker, effort ladder, and context ring all follow the pane you're typing in.
- **No more silent dead turns.** When a provider endpoint misbehaves (Kimi rate-limiting, auth failures, server errors), Rift now toasts the real status — and after 5 failures in a row it ends the turn with an honest error instead of letting the CLI retry silently for minutes.
- **Blame lands where it belongs.** The "slow turn start" hint names the actual endpoint (e.g. Kimi — Moonshot) when a provider chat stalls — it no longer pins provider slowness on the Anthropic API.
- **Old chats stay on Claude.** Reopening a conversation from before this release can't accidentally resume it through whatever provider is now your default.
- **Tidier sidebar footer.** The workspace icons sit in a proper dock with a sliding highlight under the active one; notifications and Settings stand apart on the right.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
