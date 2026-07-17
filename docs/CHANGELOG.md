# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.120.0 — Provider models, the full nine yards

- **Pick provider models right in the chat.** The composer's model menu now lists your provider's models (Kimi, DeepSeek, GLM, OpenRouter…) with the same picker and keyboard flow Claude gets — switch models mid-chat without losing the conversation.
- **Reasoning effort for providers.** Reasoning-capable endpoints (Kimi / DeepSeek / GLM by default — toggleable per provider on the Models page) get the full Low→X-High effort ladder, and their native thinking now streams into the transcript instead of being silently suppressed.
- **An honest context gauge.** The ring now knows kimi-k3 carries a 1M window (and friends their real sizes) instead of assuming 200K for every provider model.
- **The status bar tells the truth.** While a provider is live, the connection chip and the disclaimer name that model — not Claude — and the chip jumps straight to the Models page.
- **Fixed:** the Models page opened with blank fields (and a wrong effort toggle) for the already-active provider until you re-clicked its card.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
