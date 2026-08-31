# rift-tauri — Changelog

> Current release only. Older release notes remain in Git history and on the
> [GitHub releases page](https://github.com/Blazzer10200/rift-tauri/releases).

## v0.158.0 — Calmer chat, clearer control

- Alerts is now one reliable footer destination with a full-size hit target,
  readable label, correctly placed unread badge, visible keyboard focus, and a
  notification panel that stays anchored inside the window.
- Older conversation history loads in batches of 40 instead of rendering the
  entire archive at once, keeping large workspaces responsive while preserving
  access to every chat.
- Completed-work receipts open as compact summaries. Terminal output stays
  collapsed until requested, and one **Expand output** action can reveal or
  collapse every terminal result in the receipt.
- Message copy and retry actions are easier to find, user messages can be copied
  directly, and the turn navigator now sits beside the reading column with
  larger controls and clearer accessibility semantics.
- The composer has steadier guidance and larger attachment, dictation, model,
  context, and send targets. Windows commands display readable path separators,
  while copied command text remains unchanged.
- The new-chat launchpad makes better use of wide panes, falls back to a calm
  single-column layout in split view, and gives **Switch folder**, **View
  activity**, and **Continue** actions more deliberate emphasis.
- Workspace transitions now use a structured loading skeleton, status actions
  announce what they open, and natural-language lists keep their intended
  single-column reading order.

## Known issues

- Elevated windows cannot accept drag-and-drop from lower-integrity Explorer;
  use the attachment picker instead.
- Web Speech may mask profanity; the on-device Parakeet engine is verbatim.
