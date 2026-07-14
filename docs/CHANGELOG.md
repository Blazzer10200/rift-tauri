# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.106.0 — Fast lane

- **Fast mode for Opus.** A new toggle in the model picker (Opus rows only) runs Opus with noticeably quicker output — same model, same quality tier. Turns that actually ran fast get a small ⚡ "fast" chip, so you always know whether it kicked in. Needs Claude CLI 2.1.209+.
- **Switching model or permission mode mid-chat is now instant.** Rift pushes the change to the already-running Claude process instead of restarting it — a model switch that used to cost ~1.5s of respawn now takes effect immediately. (Switching between Ask-style and Bypass-style permission modes still restarts — that change genuinely needs different launch flags.)
- **The context gauge now corrects itself from real data.** After each reply, Rift reads the context window the CLI actually ran against and uses that for the gauge — so accounts with a smaller window (e.g. Free plan) see honest numbers without configuring anything.
- Recommended Claude CLI version raised to 2.1.209 (fast mode + live switching + several upstream stream-json fixes).

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.
- **Dismissed ask-cards** render "(no answer recorded)" instead of a "Dismissed" state — cosmetic, fix planned.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
