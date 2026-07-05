# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.86.1 — Stability sweep: ten verified fixes from a deep adversarial audit

- **Stopping a turn can no longer corrupt the next one** — the stopped turn's final signal could land after you'd already sent the next message in the same chat, silently freezing the new reply mid-stream (and sometimes flashing a bogus error banner for the turn you stopped). The next turn now waits the split-second it takes for that signal to settle.
- **Long tools aren't falsely declared stalled in plan mode** — approving a plan while a slow web fetch ran could confuse the stall watchdog's bookkeeping and kill a healthy turn at the 3-minute mark.
- **Deleting a conversation can't be undone by a racing autosave** — delete and save now share a lock, so a just-deleted chat can't quietly reappear on disk.
- **Re-clicking the model a chat already uses no longer silently rewrites your global default model** (it only misfired when an old chat was pinned to a different model than your baseline).
- **Timed-out CLI updates clean up after themselves** — a stalled `npm install` is killed instead of orphaned in the background, where it could lock files and break the retry.
- **Changing the per-turn budget cap applies to the very next turn** — a warm CLI process previously kept enforcing the old cap until something else respawned it.
- Smaller fixes: back-to-back "Step" headers no longer drop the first one · switching panes across different folders refreshes the @-mention/branch caches reliably · the thinking-off proxy rejects truncated requests and can't leak stalled connections · unreadable conversation files are logged instead of silently vanishing from the list.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.86.0** — Diff word-level highlights; quiet update-detection states; registry-PATH CLI discovery; corrected CLI version gates.
- **v0.85.x** — Pinned Plan HUD + stream polish (calmer live line, minute-aware durations, reduced-motion); plan-HUD hardening; resumed conversations restore their project folder; "quick" tier retired; fresher CLI status on window focus.
- **v0.84.x** — Adaptability pass (CLI found via pnpm/Volta/Scoop/Bun; honest degradation on dead mic / stuck downloads / corrupt config; credential-keyed warm processes); reasoning ladder; transcript detail; Settings redesign + workspace hub; self-verification follow-ups (unbounded stdout reader revert, maximize state, queue races, copy-failed state).
- **v0.82.x** — Warm-CLI process-leak fixes; stream density controls (Tool detail + presets).
- **v0.79.0–v0.81.0** — Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling.
- **v0.74.0–v0.78.0** — Command output in-stream, calmer narration, steer removed, permission/sub-agent/dictation fixes.
- **v0.66.0–v0.72.x** — Workspace/projects overhaul, fast-by-default, split-pane isolation, unified chat-block look.
- **v0.20.7–v0.65.0** — Foundation era: redesign port, warm-CLI, multi-window, dashboard + AI Health, voice mode, diagnostics console.
