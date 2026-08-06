# rift-tauri — Changelog

> Current release only. Older release notes remain in Git history and on the
> [GitHub releases page](https://github.com/Blazzer10200/rift-tauri/releases).

## v0.153.0 — Reliable ChatGPT activity and commands

- ChatGPT command, tool, and file activity now appears as work starts instead of
  disappearing until the final answer.
- Added, edited, moved, and deleted files render as individual cards with
  readable unified diffs and accurate line counts.
- Live command output and MCP progress update pending cards; completed output
  remains authoritative and replaces the partial stream.
- Successful MCP calls no longer turn red when App Server sends `error: null`,
  and dynamic tools retain their current structured results and success state.
- Current App Server plan, image, sleep, review, agent, and compaction items are
  normalized into Rift's shared activity surfaces.
- The slash-command rail now has stable provider-aware lanes, calmer styling,
  description/source search, deterministic selection, and correct `/` versus
  `$` insertion for Claude commands and ChatGPT account skills.

## Known issues

- Elevated windows cannot accept drag-and-drop from lower-integrity Explorer;
  use the attachment picker instead.
- Web Speech may mask profanity; the on-device Parakeet engine is verbatim.
