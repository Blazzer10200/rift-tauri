# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.82.0 — Dial in how much the activity stream shows (two axes + one-tap presets)

### Added
- **Tool detail — a new density control for the work stream itself.** Until now you could tune the model's *narration* and a shell command's *output*, but not how much the tool/file rows themselves showed. The new three-way **Tool detail** control (Settings → Chat → Reading → Chat rendering) fixes that: **Balanced** (default) names each row's targets on the collapsed line (`Read a.ts · Searched "foo"`); **Minimal** collapses a work run to a single one-line outcome, still one click from the full list; **Detailed** auto-expands every row with full file paths and streams full command output. Clean readers get calm; power users get the play-by-play — same data, your zoom level.
- **Density presets.** A **Calm / Standard / Verbose** picker sets all three stream knobs (tool detail, narration, command output) together, so you don't have to tune three sliders to get a coherent feel. Pick one, then fine-tune any single axis afterward — the preset highlight simply clears once you drift off it (nothing is silently remembered to go stale).

### Fixed
- **Collapsed work rows no longer hide what they did.** A mixed turn used to summarize as a bare count — `Read 2 · searched 1` — throwing away the filenames it already knew. Rows now name their targets even across mixed tool kinds (`Read HANDOFF.md, README.md · Searched *.md`), so you can read a turn's work without expanding a thing.
- **No more silent setting override.** When Tool detail is set to Detailed (which forces full command output), the separate **Command output** control now visibly dims and shows a "· set by Detailed" note instead of just ignoring your choice — and restores it the moment you leave Detailed.

### Internal
- New `toolDetail` pref + `DensityPreset` map + apply-only `applyPreset()` with a *derived* `activePreset` getter (no 4th persisted key that could desync from the three it sets) in `ui-prefs.svelte.ts`; pure `workLineMode()` tier→render-mode helper in `streamModel.ts` (unit-tested); `groupNames()` rewritten to segment mixed groups per-kind, dominant-first. Wired through `WorkLine.svelte` (minimal keeps a chevron escape hatch; detailed auto-opens + full paths, wrap-not-ellipsis) and a one-line `StreamShell.svelte` override. Verified live via CDP across real sessions (mixed-row naming, Detailed→dimmed Command-output). Green: svelte-check 0/0 (4138) · vitest 54/54 (stream).

### Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.81.0** — Sonnet 5 gets its full 1M context window (the CLI defaulted it to 200K, so long chats compacted at ~14% while the gauge said 1M); the picker no longer offers unavailable Fable; small context-readout accuracy fixes.

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
