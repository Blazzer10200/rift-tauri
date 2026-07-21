# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.134.0 — Built-in debugger, streaming display heal, autocorrect

**Diagnostics grew into a real debugger surface, the "everything bugs out mid-session" display glitch is fixed, and the composer learned to fix typos as you type.**

- **Diagnostics is now its own sidebar page** (keyboard 5, between AI Health and Settings) — full-height console with health cards, filters, and search. It opens **with history**: the last 500 events (boot logs, update checks, spawn traces) are already there instead of a blank panel. A download button exports a support bundle (logs + turn traces) straight to Downloads.
- **Every turn now records which tools ran and how long each took**, and the assistant itself can read the app's own logs and turn traces via two new built-in tools — so it can debug Rift from inside Rift.
- **Fixed: long working turns no longer "bug out" the transcript.** On big thinking-heavy turns, shell blocks could render as a blank `PS> shell` with frozen spinners for minutes — the turn was actually fine underneath (nothing was ever lost), but the live view stopped filling in commands. Three-layer fix: a still-forming command now reads "running…", the final envelope always heals a pending block's input, and a tool result always ends the forming state.
- **New: opt-in composer autocorrect.** Toggle it on in the composer's model menu — common typos ("teh", "dont", "im") are fixed the moment you finish the word. It deliberately skips slash commands, file paths, flags, and code-like text, and Ctrl+Z restores exactly what you typed. Off by default.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
