# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.81.0 — Sonnet 5 gets its full 1M context (and no dead model options)

### Fixed
- **Sonnet 5 now actually uses its full 1,000,000-token context window.** The context gauge showed "1M" but the conversation was compacting at ~14% — because the Claude CLI defaults `claude-sonnet-5` to a **200K** window unless it's asked for the large one explicitly, so it auto-compacted at ~160–184K while Rift's gauge (correctly) measured against 1M. The two never agreed, and long chats got summarized far too early. Rift now requests the 1M window for Sonnet 5 (and Sonnet 4.6), so the CLI's compaction point and the on-screen gauge finally match — a Sonnet chat runs to nearly 1M tokens before it compacts, exactly like the gauge implies. Opus was already correct and is unchanged.
- **The model picker no longer offers a model that can't run.** Claude Fable 5 is currently unavailable at the API (a government access gate; Anthropic has said general access is being restored, but it isn't live yet). Rift was still listing Fable in the picker, so choosing it produced a hard "currently unavailable" error. Fable is hidden until access returns; any chat previously pinned to Fable falls back to Opus automatically. It flips back on the moment the API answers again.
- **Small context-readout accuracy fixes:** a token count in the 999.5K–999.9K range now reads "1.0M" instead of "1000k", and a resumed pre-rename Sonnet 4.5 session reports its real 200K window instead of over-stating 1M.

### Internal
- Backend appends the CLI's `[1m]` window-selector to the `--model` arg for the Sonnet ids the CLI gates at 200K (`cli_model_arg`), built *after* the session pin so the persisted, signature-preserving pin stays the bare id (a bracketed pin would fail validation and silently un-pin on resume). Fable kill-switch flipped in lockstep across `config.rs` + `helpers.ts`; a clean-flip trace confirmed re-enabling is a two-const change with zero test breakage. Added `scripts/fable-watch.ps1` to detect the moment Fable access returns. Full green: cargo test 131/131 · svelte-check 0/0 (4138) · vitest 406/406; verified end-to-end live (Sonnet turn → 1M gauge at true %; Fable absent from the live picker).

### Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.80.0** — Stuck sub-agents now get caught: a wedged (but still-chatty) sub-agent could spin the turn forever because the stall watchdog re-armed on every output line; a hard 15-minute in-flight ceiling that stream activity can't extend now force-ends it.
- **v0.79.0** — Claude Sonnet 5: "Sonnet" now actually runs Sonnet 5 (the shipped CLI's bare `sonnet` alias still resolved to 4.6, so Rift now pins the explicit id), reaches the X-High effort tier, and shows clean dateless model labels.
- **v0.78.0** — Queued messages keep their image attachments, short bleeped swear phrases get de-censored in voice dictation, and first-run onboarding fixes (no phantom Haiku option, corrected hints).

- **v0.77.0** — See command output in the stream (Peek/Full/Minimal), project-ghost fix, and one neutral surface for every chat block.

- **v0.76.0** — A calmer activity stream: between-step narration is demoted to quiet inline notes (new three-way **Narration** control: Focused / Balanced / Chatty), so a working turn reads as work-with-commentary, not chat-between-tools.
- **v0.75.0** — Removed the half-working "steer" feature (Alt+Enter live-injection) front-and-back; the message queue (type while it works → fires as the next turn) is now the single way to address a running turn.
- **v0.74.0** — Two bug fixes: permission prompts now appear on the live turn in every non-Bypass mode (gated tools were silently auto-denying after 2 min), and sub-agents reliably register as finished instead of spinning "working…" forever.
- **v0.72.0** — Plan-mode unfreeze (#75), terminal-grade work habits — batches tool calls + skips redundant re-reads (#76), and a unified look for every chat block (#77; the emerald tint from this is what v0.77.0 replaced with neutral gray).

- **v0.71.x** — Path-helper de-dup (one canonical `utils/path.ts`), split-pane isolation (per-pane sub-agent panel, no cross-pane crosstalk), a warm-CLI stale-frame/permission-race bug-fix sweep, the turn-spawn refactor (orchestrator + `resolve_spawn`, lints 14→0), and first-run onboarding rework.
- **v0.66.0–v0.70.0** — Workspace + projects UI overhaul, no-folder scratch workspace, fast-by-default (thinking split into its own toggle), and the warm-pool persistent-process fix.
- **v0.20.7–v0.65.0** — Foundation + diagnostics era: full redesign port, stream design language, warm-CLI process, multi-window sync, Workspace dashboard + AI Health, voice mode, honest mid-chat model switching, and the diagnostics console.
