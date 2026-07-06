# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.88.0 — Terminal-style tool blocks + calmer live stream

- **Commands now read like a real terminal.** Every bash / PowerShell / cmd block shows what was run as an **input line** — a shell-colored prompt (`$` · `PS>` · `>`) followed by the command — and the result underneath as a clearly-labeled **output** section. Before, the command and its output blurred together in one gray slab; now the in-and-out is obvious at a glance, and a failed command reads `exit 1` with its error called out in red.
- **Long output no longer traps you in a tiny scroll box.** A big `git log`, a full build, or a chatty tool response now shows a readable chunk with a **"Show N more lines"** control that steps the view open — collapsed → more → all — and only a genuinely huge, fully-expanded block ever becomes a bounded scroll. Applies to shell output and tool responses alike.
- **The live "working" line stopped making things up.** Between tool calls it used to cycle invented words ("Mapping…", "Pondering…") that implied actions the model wasn't taking. It now shows the real action when a tool is running (e.g. *Running npm run build*) or an honest "Thinking…/Working…" otherwise — and the thinking indicator no longer double-prints in two places.
- **A running command shows up as its terminal block right away** instead of briefly appearing as a plain row and then jumping into a block once output arrived.
- **Appearance settings look right.** The background-texture picker previews now match what actually lands behind the workspace — Blueprint and Glow no longer render as an over-bright green wash — and each tile is legible enough to pick. The build badge for an in-development copy reads as a calm "dev" tag instead of an alarm-colored spinner, and the About → Build panel leads with a clear version identity.
- **Under the hood:** the live stream/done/error event handling was extracted and covered by tests (no behavior change); Markdown rendering now caps parse input so a multi-megabyte pasted file can't stall the UI; a first-run onboarding edge case no longer throws an unhandled rejection.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.87.0** — Per-turn epoch on the full event wire (stopped turns can't bleed into the next); self-update kills by exe-path not image-name; delete-vs-autosave resurrect fixed; paste/drop image target-locked; npm probe 5s-bounded; window-scoped dictation; honest MCP notify/open_browser; advisory scans → daily CI; project-highlight + switcher-coords fixes.
- **v0.86.4** — Queued-message hardening: attachments ride the send (no composer cross-contamination), requeue-front ordering, chip attachment badges, sidebar queued-count badge, `/model` de-staled.
- **v0.86.3** — Project switching actually switches (stale per-tab folder override); mic no longer stuck red after dictation; tool-summary overflow truncation.
- **v0.86.2** — Clean exit: closing Rift mid-turn reaps every CLI child (no more headless token-burning orphans); background-footprint audit confirmed idle is tight.
- **v0.86.1** — Stability sweep: ten adversarially-verified fixes (stopped-turn corruption, plan-mode stall watchdog, delete-vs-autosave race, model-reselect default rewrite, orphaned npm updates, stale budget cap).
- **v0.86.0** — Diff word-level highlights; quiet update-detection states; registry-PATH CLI discovery; corrected CLI version gates.
- **v0.85.x** — Pinned Plan HUD + stream polish (calmer live line, minute-aware durations, reduced-motion); plan-HUD hardening; resumed conversations restore their project folder; "quick" tier retired; fresher CLI status on window focus.
- **v0.84.x** — Adaptability pass (CLI found via pnpm/Volta/Scoop/Bun; honest degradation on dead mic / stuck downloads / corrupt config; credential-keyed warm processes); reasoning ladder; transcript detail; Settings redesign + workspace hub; self-verification follow-ups (unbounded stdout reader revert, maximize state, queue races, copy-failed state).
- **v0.82.x** — Warm-CLI process-leak fixes; stream density controls (Tool detail + presets).
- **v0.79.0–v0.81.0** — Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling.
- **v0.74.0–v0.78.0** — Command output in-stream, calmer narration, steer removed, permission/sub-agent/dictation fixes.
- **v0.66.0–v0.72.x** — Workspace/projects overhaul, fast-by-default, split-pane isolation, unified chat-block look.
- **v0.20.7–v0.65.0** — Foundation era: redesign port, warm-CLI, multi-window, dashboard + AI Health, voice mode, diagnostics console.
