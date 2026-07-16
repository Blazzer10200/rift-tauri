# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.113.0 — Calmer chrome, honest questions

- **The title bar is furniture now.** The split-editor, new-window, and notification icons no longer crowd the window controls — the top-right is minimize/maximize/close, period. Split editor and New window live in the command palette (Ctrl+K) and keep their shortcuts (Ctrl+\ still splits).
- **Notifications moved into the sidebar footer.** The bell sits with the workspace icons (badge included) and its panel opens upward from the corner — one click, same history, no title-bar clutter.
- **Questions from the assistant can no longer die silently.** When a turn ends before you answer an ask-card (timeout, stop, error, or an app restart mid-question), the card now renders an honest "Question expired" state — question text kept readable, dead Submit/Dismiss buttons gone. A question whose options are still streaming in shows "Preparing the question…" instead of an empty shell.
- **Dismissing a question looks dismissed.** The card is headed "Dismissed" with a neutral tint (not a green checkmark), and the previously-listed "(no answer recorded)" known issue is verified fixed.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
