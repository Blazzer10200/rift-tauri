# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.79.0 — Claude Sonnet 5

### Fixed
- **"Sonnet" now actually runs Sonnet 5, not 4.6.** Anthropic released Claude Sonnet 5 (June 9) — the best speed/intelligence balance, with quality approaching Opus 4.8 at the same Sonnet price. Rift's picker already said "Sonnet 5", but every turn was silently running the previous generation (4.6): the shipped Claude CLI still resolves the bare `sonnet` alias to `claude-sonnet-4-6`, so passing the alias ran 4.6. Rift now pins the explicit `claude-sonnet-5` id before the turn, so picking Sonnet runs Sonnet 5. *(The `opus`/`haiku` aliases already resolved to their newest models — only `sonnet` lagged.)*
- **Sonnet now reaches the X-High effort tier.** Sonnet 5 honors `xhigh` server-side (Sonnet 4.6 rejected it and capped at High), so the thinking dial's top rungs and the ultracode workflow are now available on Sonnet — verified against the live model.

### Changed
- **Model labels read cleanly for major releases.** The model name shown on each turn now handles the dateless id format Anthropic uses from the 4.6 generation on (e.g. `claude-sonnet-5` → "Sonnet 5", `claude-fable-5` → "Fable 5"), instead of falling back to the raw id string.
- Pricing table + the voice-transcript cleanup helper updated to the current Sonnet id. In-flight conversations created on Sonnet 4.6 keep running 4.6 on resume (their reasoning is signed to that model) — only new chats move to Sonnet 5.

### Internal
- Alias→id resolution lives in one place per side (`canonical_model_alias` / `canonicalModelAlias`) with a regression test. Full green: cargo check 0/0 · cargo test (config) 9/9 · svelte-check 0/0 (4138) · vitest 405/405, plus live-CLI verification.

### Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.78.0** — Queued messages keep their image attachments, short bleeped swear phrases get de-censored in voice dictation, and first-run onboarding fixes (no phantom Haiku option, corrected hints).

- **v0.77.0** — See command output in the stream (Peek/Full/Minimal), project-ghost fix, and one neutral surface for every chat block.

- **v0.76.0** — A calmer activity stream: between-step narration is demoted to quiet inline notes (new three-way **Narration** control: Focused / Balanced / Chatty), so a working turn reads as work-with-commentary, not chat-between-tools.
- **v0.75.0** — Removed the half-working "steer" feature (Alt+Enter live-injection) front-and-back; the message queue (type while it works → fires as the next turn) is now the single way to address a running turn.
- **v0.74.0** — Two bug fixes: permission prompts now appear on the live turn in every non-Bypass mode (gated tools were silently auto-denying after 2 min), and sub-agents reliably register as finished instead of spinning "working…" forever.
- **v0.72.0** — Plan-mode unfreeze (#75), terminal-grade work habits — batches tool calls + skips redundant re-reads (#76), and a unified look for every chat block (#77; the emerald tint from this is what v0.77.0 replaced with neutral gray).

- **v0.71.x** — Path-helper de-dup (one canonical `utils/path.ts`), split-pane isolation (per-pane sub-agent panel, no cross-pane crosstalk), a warm-CLI stale-frame/permission-race bug-fix sweep, the turn-spawn refactor (orchestrator + `resolve_spawn`, lints 14→0), and first-run onboarding rework.
- **v0.66.0–v0.70.0** — Workspace + projects UI overhaul, no-folder scratch workspace, fast-by-default (thinking split into its own toggle), and the warm-pool persistent-process fix.
- **v0.20.7–v0.65.0** — Foundation + diagnostics era: full redesign port, stream design language, warm-CLI process, multi-window sync, Workspace dashboard + AI Health, voice mode, honest mid-chat model switching, and the diagnostics console.
