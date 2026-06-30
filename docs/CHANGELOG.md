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

- **v0.71.7** — Maintenance (no behavior change): first half of the path-helper de-dup — consolidated `leafName`/`shortPath`/`prettyPath` into `utils/path.ts` (v0.71.8 folded in the `rootKey` half).

- **v0.71.6** — Split-pane stays in its lane: per-pane sub-agent panel (each pane shows its own agents, with a "N steps · Ms" summary, per-tool-type icons, and a blended-glass card), steer now tells you when a missed steer became a queued message, closing a background tab no longer drops its unsaved tail, and background-pane notifications/browser-dock stop popping in the pane you're looking at. Root cause: nearly everything read off the single global focused-tab (16 of 31 audit findings confirmed; backend session-isolation was already clean).

- **v0.71.5** — Fix: split-pane crosstalk (the inactive pane mirrored the active one's thinking timer + context readout) and dragging a project chip onto a pane (a copy/move drag-effect mismatch made WebView2 reject the drop).

- **v0.71.4** — Fix: a reused warm CLI process could fire an instant, off-topic reply (stale pipe frame) after an `ask_user` round-trip, plus a permission-prompt pairing race and broken per-turn latency telemetry (Issues #72 + #73).

- **v0.71.3** — Bug-fix sweep: safer "delete all conversations", tab layout survives a flaky restart, no empty-pane flicker on close, no ghost-save resurrecting a deleted chat, plus a warm-pool process-leak race + async-runtime + usage-refresh hygiene batch (7 fixes from a live stress-test + static audit).

- **v0.71.2** — Maintenance/housekeeping (no behavior change): the ~740-line turn-spawn function split into an orchestrator + `resolve_spawn`, every compiler lint cleared (14 → 0), dead code removed, internal trackers reconciled to reality.

- **v0.71.1** — Hotfix: the assistant could go passive on short messages — a benign plan-usage note in front of a brief message made the model give clipped/"winding down" replies. The note now appears only when usage is genuinely high, so short messages get a normal response again.

- **v0.71.0** — First-run onboarding rework: lighter welcome, a clearer Connect step when the CLI is missing, a scratch-space hint, and the Defaults step collapsed into one Cautious/Balanced/Fast choice.

- **v0.70.0** — Workspace + projects UI overhaul: one-click open/split projects via sidebar chips, the dashboard renamed to "Workspace" everywhere, an honest context gauge on 1M-context models, and a cleaner single-scroll Workspace + less self-contradictory AI Health. Plus a local-LLM corporate-TLS fix from a pre-release security sweep.

- **v0.68.0** — Reliability + safety hardening from a four-sweep full-codebase audit: Stop now cancels a pending permission prompt and tree-kills a wedged child instantly (closed a 9-min-wedge incident), `ring` became the sole TLS provider (`aws-lc-rs` out), and the release pipeline refuses to ship a stale update feed.

- **v0.67.0** — Fast by default: extended thinking became its own ON/OFF toggle (off by default), split from the effort slider, so everyday replies land in 1–2s instead of sitting silent for up to 40s. Plus the warm-pool persistent-process fix that keeps turns snappy across a whole session.

- **v0.66.0** — Just start chatting: open Rift with no folder and the assistant still reads/writes/edits/runs in a private scratch workspace (with a "Local" badge), plus a one-time migration that clears stale per-folder "thinking on" pins so no folder is mysteriously slow.

- **v0.60.0–v0.65.0** — Cross-machine + diagnostics era: the unified queue/steer model, honest mid-chat model switching, scaled-screen layout, guided first-run setup, human-readable errors, and a live per-subsystem diagnostics console.
- **v0.20.7–v0.53.0** — Foundation era: the full redesign port + stream design language, the warm-CLI process, multi-window sync, the Workspace dashboard + AI Health, voice mode, and the notification center.
