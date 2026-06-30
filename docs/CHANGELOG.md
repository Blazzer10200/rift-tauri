# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.77.0 — See command output, cleaner projects, one calm block style

### Added
- **Command output now shows in the stream — the in-and-out, not just the command line.** When Claude runs a shell command you see its actual stdout/stderr, with a new three-way **Command output** control in Settings → Chat rendering: **Peek** (default — exit status + the last few lines, click to expand the rest), **Full** (the whole terminal output streams live as it runs), or **Minimal** (just the command line, the old behavior). A command with no output stays a quiet one-liner.

### Fixed
- **Removing a project no longer leaves a ghost.** Deleting a project used to leave its folder lingering in the "recent folders" list, so it kept re-appearing as if it were still a thing (e.g. a removed `exfil-v1` haunting the picker). Delete now fully forgets the folder. The Add-a-project area was also cleaned up: one quiet "Save this folder as a project" prompt instead of a grid of random recent-folder tiles, and recent folders moved into the new-project picker with a per-row "forget" (×) button to prune stale ones.
- **The same command no longer prints twice in the live stream** — it was showing once as the work row and again in the muted footer; the footer now keeps just the verb + timer + tokens.

### Changed
- **Every chat block now shares one neutral surface.** Terminals, file reads, grep/glob results, create/edit diffs, and the model's own code blocks were a mix of emerald-tinted "glassy" cards (v0.72.0) and plain gray, at different widths — so a single answer could show two clashing colors and crooked edges. They're all one neutral-gray, full-width, aligned family now; accent is reserved for live "running now" cues and prose (links, callouts). (CDP-verified: terminal, command output, and JSON code blocks render identical; svelte-check 0/0, vitest 128/128.)

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.76.0** — A calmer activity stream: between-step narration is demoted to quiet inline notes (new three-way **Narration** control: Focused / Balanced / Chatty), so a working turn reads as work-with-commentary, not chat-between-tools.
- **v0.75.0** — Removed the half-working "steer" feature (Alt+Enter live-injection) front-and-back; the message queue (type while it works → fires as the next turn) is now the single way to address a running turn.
- **v0.74.0** — Two bug fixes: permission prompts now appear on the live turn in every non-Bypass mode (gated tools were silently auto-denying after 2 min), and sub-agents reliably register as finished instead of spinning "working…" forever.
- **v0.72.0** — Plan-mode unfreeze (#75), terminal-grade work habits — batches tool calls + skips redundant re-reads (#76), and a unified look for every chat block (#77; the emerald tint from this is what v0.77.0 replaced with neutral gray).

- **v0.71.x** — Path-helper de-dup (one canonical `utils/path.ts`), split-pane isolation (per-pane sub-agent panel, no cross-pane crosstalk), a warm-CLI stale-frame/permission-race bug-fix sweep, the turn-spawn refactor (orchestrator + `resolve_spawn`, lints 14→0), and first-run onboarding rework.
- **v0.66.0–v0.70.0** — Workspace + projects UI overhaul, no-folder scratch workspace, fast-by-default (thinking split into its own toggle), and the warm-pool persistent-process fix.
- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: unified queue/steer model, honest mid-chat model switching, guided first-run setup, human-readable errors, live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
