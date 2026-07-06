# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.86.3 — Project switching, mic state, and overflow fixes

- **Switching projects from the sidebar now actually switches.** Picking an already-added project changed the global folder but the focused tab's own folder override kept winning — the switcher header, branch pill, "This project" chat filter, and even the folder the AI worked in all stayed on the OLD project. A global switch now clears the focused tab's override (split panes scoped to other projects are untouched). Fixes the same staleness in the Workspace page's project cards and onboarding recents.
- **The microphone no longer stays red after dictation.** Sending a dictated message aborted the recognizer, and that self-inflicted abort was recorded as a sticky "Recording cancelled." error — painting the mic's red error outline until the next recording. Aborts are no longer treated as errors, and a session that ends with text committed clears any transient mid-session error. Real failures (permission denied, no mic) still show red with the reason in the tooltip.
- **Long tool-activity summaries no longer run off the right edge.** A collapsed "Searched … · Read …" row with a long pattern or path now truncates with an ellipsis (full text on hover / expand) instead of overflowing the chat column. Same guard added to the live "Working…" caption.
- Under the hood: folder comparisons now treat `/` and `\` as the same separator everywhere (one canonical `rootKey`), removing a duplicate implementation that had already drifted.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

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
