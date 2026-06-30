# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.78.0 — Queued images, voice profanity, onboarding polish

### Fixed
- **A queued message now keeps its image.** If you attached an image (or text file) and sent it *while the assistant was already working*, the message parked in the queue but the attachment was silently dropped — it fired off later as text-only. The queue now snapshots attachments when you queue, and replays them with the message when it sends, so the image rides along exactly as if you'd sent it normally.
- **Voice dictation: short swear phrases are no longer left bleeped.** The Web Speech engine masks profanity ("f***"), and the cleanup pass that restores it was skipping anything under three words — so the most common case ("f*** you") shipped with asterisks. Masked phrases now always get the restore pass regardless of length. *(Full uncensored dictation still wants the on-device Whisper engine — see Known Issues.)*
- **First-run onboarding fixes.** The model picker no longer offers Haiku 4.5 (currently unavailable — it was silently falling back to Sonnet after you "chose" it). Two stale hints that pointed at a non-existent title-bar button now point at the Workspace page, and the sign-in recovery instructions in the tester guide were corrected (they named a Settings path that doesn't exist).

### Changed
- **`ask_user` multiple-choice now reliably offers multi-select when it should.** The "pick all that apply" checkbox mode was fully built but the model rarely triggered it; the tool description now nudges it to use multi-select whenever the options aren't mutually exclusive.

### Internal
- Backend hardening: a pre-warm CLI child spawned during a stop is now reaped instead of orphaned; a per-session timing map is pruned on session end instead of growing for the process lifetime; `git rev-parse` env-hardening brought in line with the other git calls. (cargo test 128/128, svelte-check 0/0, vitest 405/405.)

### Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (transcribes verbatim, fully local), which is built but not yet in the shipped binary. Planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.77.0** — See command output in the stream (Peek/Full/Minimal), project-ghost fix, and one neutral surface for every chat block.

- **v0.76.0** — A calmer activity stream: between-step narration is demoted to quiet inline notes (new three-way **Narration** control: Focused / Balanced / Chatty), so a working turn reads as work-with-commentary, not chat-between-tools.
- **v0.75.0** — Removed the half-working "steer" feature (Alt+Enter live-injection) front-and-back; the message queue (type while it works → fires as the next turn) is now the single way to address a running turn.
- **v0.74.0** — Two bug fixes: permission prompts now appear on the live turn in every non-Bypass mode (gated tools were silently auto-denying after 2 min), and sub-agents reliably register as finished instead of spinning "working…" forever.
- **v0.72.0** — Plan-mode unfreeze (#75), terminal-grade work habits — batches tool calls + skips redundant re-reads (#76), and a unified look for every chat block (#77; the emerald tint from this is what v0.77.0 replaced with neutral gray).

- **v0.71.x** — Path-helper de-dup (one canonical `utils/path.ts`), split-pane isolation (per-pane sub-agent panel, no cross-pane crosstalk), a warm-CLI stale-frame/permission-race bug-fix sweep, the turn-spawn refactor (orchestrator + `resolve_spawn`, lints 14→0), and first-run onboarding rework.
- **v0.66.0–v0.70.0** — Workspace + projects UI overhaul, no-folder scratch workspace, fast-by-default (thinking split into its own toggle), and the warm-pool persistent-process fix.
- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: unified queue/steer model, honest mid-chat model switching, guided first-run setup, human-readable errors, live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
