# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.118.0 — Models: bring other frontier models into Rift

- **New Models workspace** (the chip icon, ⌨5). Add Kimi, DeepSeek, GLM, OpenRouter, a local Ollama/LiteLLM endpoint, or any custom Anthropic-compatible URL as a chat brain. Claude stays the default; switch back any time.
- **One-click presets.** Each preset chip prefills the endpoint and starting models — you just paste your API key. A **Detect** button lists what an endpoint serves (where supported), a **Test** button round-trips a one-line prompt before you switch your chats over, and free-text model names always work — no built-in catalog to go stale.
- **Keys stay in your OS keychain**, never in a config file, and never reach the page — the UI only ever sees "a key is set."
- **Switching brains starts a fresh chat** (the old one is saved in History) and shows a provider pill in the composer so you always know who's answering.
- **Your existing local-LLM setup migrates automatically** — it appears as a "Local (Ollama/LiteLLM)" card with its endpoint, model, and key intact. The Ollama context-window check and one-click "Optimize for Rift" moved into that card.
- Heads up: Rift still runs everything through the Claude CLI — a provider only swaps which model answers. How well a model handles edits and tool calls varies by model.

## Known issues
- **While elevated, dragging files from Explorer into the window doesn't work** (Windows blocks lower→higher integrity drag-drop); the attach button / file picker still works fine.
- **Web Speech still masks profanity** (Azure-side, unrecoverable when fully masked) — switch to the Parakeet engine; it transcribes verbatim on-device.

## Earlier

History via `git log -- docs/CHANGELOG.md`.
