# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.90.0 — Quality-of-life fixes + prompt-suggestion groundwork

- **Prompt suggestions (groundwork, dormant for now).** Rift now asks Claude Code for a predicted "next prompt" after each turn and shows it as a quiet chip above the message box — click to insert it into the draft, × to dismiss, and it hides the moment you start typing. Anthropic currently has generation switched off server-side, so the chip stays hidden until they enable it; everything on Rift's side is wired, tested, and verified harmless to normal turns.
- **Voice input failures now speak up.** Microphone permission denied, no microphone, or a dictation session that can't start pops an auto-expiring notice — before, the only clue was the mic button quietly turning red.
- **Assistant-opened pages pull you to Chat.** When the assistant opens a link in Rift's built-in browser while you're on Settings or AI Health, Rift now switches to the chat workspace so the page actually appears (it used to queue invisibly until you wandered back).
- **AI Health: Undo survives Re-analyze.** Applying a recommendation and then re-analyzing no longer throws away the Undo button — you always keep a path back to the setting you had before.
- **Deleting a chat cleans up fully.** Conversations that had been auto-compacted left dead CLI session files on disk when deleted; those are now swept as part of the delete.
- **Second windows always start fresh.** A newly opened secondary window could inherit the pane/split layout from a window you had open in a *previous* app launch; window identities are now unique per launch and stale layouts are cleaned up at startup.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.89.0** — Claude Code 2.1.201 compatibility (full 2.1.19x–2.1.20x tool set, spend-cap + unknown-error surfacing); dead-workspace-tools notice; usage gauge tolerates odd values field-by-field; multi-window/multi-pane edge fixes (retry tab-identity, stale-folder honesty, sidecar cross-talk, dictation model pinning); steadier self-update; bridge/shim hardening.
- **v0.88.0** — Terminal-style tool blocks (input line + labeled output, `exit N` on failure); progressive "Show N more lines" for long output; the live "working" line stopped inventing actions; WYSIWYG texture previews + calm "dev" badge; stream event handling extracted + tested, Markdown parse cap, onboarding rejection fix.
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
