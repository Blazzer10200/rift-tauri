# rift-tauri — Changelog

> Current release only. Older release notes remain in Git history and on the
> [GitHub releases page](https://github.com/Blazzer10200/rift-tauri/releases).

## v0.152.0 — Accurate ChatGPT models and Fast mode

- ChatGPT model controls now use each route's exact effort catalog, including
  API None/Low, GPT Max, and Codex Ultra while preserving legacy saved tiers.
- Fast mode works across eligible ChatGPT subscription models, explains its
  credit tradeoff and best uses, and marks only provider-confirmed fast turns.
- Conversations remain pinned to their original subscription or API route;
  model selection cannot silently cross billing boundaries.
- Selected-model briefings now show route, context, modalities, default effort,
  Fast availability, and current upgrade guidance.
- ChatGPT helpers remain headless outside intentional interactive sign-in, and
  provider status continues to use the signed-in Codex App Server catalog.
- Frontend, provider, CDP, documentation, dependency, and Rust warning cleanup
  improves maintainability without changing stored conversations or settings.

## Known issues

- Elevated windows cannot accept drag-and-drop from lower-integrity Explorer;
  use the attachment picker instead.
- Web Speech may mask profanity; the on-device Parakeet engine is verbatim.
