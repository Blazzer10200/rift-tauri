# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.128.0 — Claude-only: provider system removed + path-bug class fully swept

**Rift is now a pure Claude app.** The multi-provider system (local LLMs, Kimi, DeepSeek, custom endpoints) is gone — the Models page, provider picker, and all related plumbing were removed. Less surface, faster startup, one brain. Old chats and settings that referenced a provider open normally as Claude chats.

- **"No project" pill fixed** — chats could show "No project" even with a folder set. Same low-level `\\?\` path-prefix bug class as v0.127.0's switcher fix; this release sweeps it everywhere (saving, loading, matching, display). Existing data self-heals on launch. Regression-tested on both the app and engine sides.
- **Startup cleanup** — a chat deleted or corrupted on disk no longer leaves a dead pane pointer behind; panes are scrubbed at boot.
- **Smoother chat loading** — opening a conversation no longer blocks the app's async runtime while reading from disk.

Note: if you previously added a provider API key, Windows may still hold it in Credential Manager — remove it there if you want it gone.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
