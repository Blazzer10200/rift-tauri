# rift-tauri — Changelog

> Current release only. Older release notes remain in Git history and on the
> [GitHub releases page](https://github.com/Blazzer10200/rift-tauri/releases).

## v0.157.0 — Clearer navigation and faster startup

- The sidebar now reads as one navigation system: workspace context, primary
  chat actions, conversation history, and a labelled destination dock. The
  duplicate **This project / All** control is gone; the project switcher is the
  single, explicit owner of conversation scope.
- Project and All views keep their safety-critical identity cues while gaining
  more conversation space. The redesigned rail remains usable at its minimum
  and maximum widths, in collapsed hover-peek mode, and with keyboard focus.
- Completed-work receipts no longer change width when hovered or keyboard
  focused. The timestamp stays in layout at low emphasis and brightens without
  moving the conversation around.
- The 50,000-word fuzzy-autocorrect dictionary now loads on demand when the
  composer needs it. Exact typo fixes remain immediate and typing never waits
  for the optional dictionary.
- The production assistant startup chunk is down from 662.83 kB minified /
  272.28 kB gzip to 237.84 kB / 73.42 kB; the 425.27 kB dictionary is a separate
  deferred chunk.
- New regression coverage locks the lazy readiness boundary, synchronous exact
  correction, fuzzy warm-up, and concurrent-load deduplication.

## Known issues

- Elevated windows cannot accept drag-and-drop from lower-integrity Explorer;
  use the attachment picker instead.
- Web Speech may mask profanity; the on-device Parakeet engine is verbatim.
