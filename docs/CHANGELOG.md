# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.85.1 — Plan HUD hardening, restored project roots, one Medium

- **Resumed conversations keep their project folder** — the per-tab root was saved with every conversation but never read back, so after a restart a resumed conversation silently ran against the wrong folder (file reads, project tools, @-mentions). It now restores exactly; legacy records still fall back to their pinned session cwd.
- **The plan HUD can't wedge or double-flash** — a failed/stopped turn used to leave a frozen, live-looking progress bar forever; completion could green-flash twice; the completion moment replayed the bar's entrance animation; switching tabs could flash another tab's long-finished plan or leave the expanded checklist open. The HUD is now strictly live-or-linger, resets per tab, flashes exactly once, and when it hides while holding keyboard focus it hands focus to the composer instead of dropping it.
- **Tool durations can't read "1m 60s"** — long tool-chip durations round the total once (59.7s reads 1m, not 60s).
- **One Medium** — the legacy hidden "quick" tier (a second Medium that sent the same wire flag as the default) is retired everywhere: picker, AI Health advice contract, backend. Stored preferences migrate automatically on load.
- **Fresher CLI status** — Rift re-checks your Claude install/version when you return to the window (max once a minute), so updating the CLI in a terminal no longer leaves a stale version badge or upgrade hint.

## v0.85.0 — Stream polish: pinned plan, calmer live line

- **The plan never scrolls away anymore** — a pinned, glassy Plan HUD floats at the top of the conversation while a plan is active: current task, slim progress bar, done/total count. Click it to expand the full checklist; when everything completes it flashes green for a moment and retires itself. The inline plan card in the transcript stays as the historical record.
- **The blinking green streaming caret is gone** — the flashing block at the tail of streaming text read as visual noise, not signal. The pulsing head dot + shimmer already say "live".
- **The live status line grew up** — elapsed time reads `3m 56s` instead of `236s` (the ticking "Thinking…" header too), the seconds/tokens meta moved to a quiet right-aligned mono cluster instead of crowding the verb, each verb swap eases in instead of hard-cutting, and the rotating words were curated to a calmer set (Tracing, Distilling, Untangling… — Sussing and Noodling retired).
- **Minute-aware durations everywhere** — "Thought for 236s" and long tool-chip durations now read `3m 56s` too.
- **Reduced motion is actually reduced** — the stream's entrance slides and pulsing live-dots now honor the OS reduced-motion setting (shimmer and counters already did).
- **Verified under the hood** — the warm-process stale-frame drain (#72) and the maximize-button state fix passed live end-to-end checks on a real dev run; the CDP dev tooling now reads stream-mode replies.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.84.x** — Adaptability pass (CLI found via pnpm/Volta/Scoop/Bun; honest degradation on dead mic / stuck downloads / corrupt config; credential-keyed warm processes); reasoning ladder; transcript detail; Settings redesign + workspace hub; self-verification follow-ups (unbounded stdout reader revert, maximize state, queue races, copy-failed state).
- **v0.82.x** — Warm-CLI process-leak fixes; stream density controls (Tool detail + presets).
- **v0.79.0–v0.81.0** — Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling.
- **v0.74.0–v0.78.0** — Command output in-stream, calmer narration, steer removed, permission/sub-agent/dictation fixes.
- **v0.66.0–v0.72.x** — Workspace/projects overhaul, fast-by-default, split-pane isolation, unified chat-block look.
- **v0.20.7–v0.65.0** — Foundation era: redesign port, warm-CLI, multi-window, dashboard + AI Health, voice mode, diagnostics console.
