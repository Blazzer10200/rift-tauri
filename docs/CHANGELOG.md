# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.80.0 — Stuck sub-agents now get caught

### Fixed
- **A wedged sub-agent no longer hangs the turn forever.** If Claude spawned a sub-agent (a Task/recon/scout) that got stuck — showing "Starting up…" in the activity dock — the turn could run indefinitely (observed: 40 minutes, spinner never stopping, nothing detecting it). Root cause: the stall watchdog re-armed on *every* line of output, and a stuck-but-chatty sub-agent kept dribbling frames that reset the watchdog, so it never tripped. Rift now tracks how long any tool has been continuously running with a **hard 15-minute ceiling that stream activity can't extend** — so a genuinely stuck sub-agent is force-ended, its process tree is killed, and the dock settles, instead of spinning forever. A legitimately long but live sub-agent finishes well within the ceiling and is unaffected.

### Internal
- The watchdog's stall decision is now a pure, unit-tested function (`watchdog_should_stall`): the dead-silent grace net AND the new absolute in-flight ceiling, either trips. Regression tests pin both the backend decision (the chatty-wedge case the old grace path missed) and the frontend cleanup (a Stalled turn ends via the error path → the spinning dock spawn is swept closed — previously only the normal-done path was covered). Full green: cargo check 0/0 · cargo test (turn) 5/5 · svelte-check 0/0 (4138) · vitest 406/406.

## v0.79.0 — Claude Sonnet 5

### Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

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
