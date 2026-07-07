# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.89.0 — Claude Code 2.1.201 compatibility + reliability hardening

- **Keeps up with the latest Claude Code.** Rift now recognizes the full tool set from recent Claude Code releases (2.1.19x–2.1.20x) — the new planning, review, workflow, and background-task tools no longer trigger stray "allow this tool?" prompts, and older CLIs keep working unchanged. If a turn stops because it hit your per-turn spend cap, Rift now says so plainly instead of ending in silence, and any unexpected stop reason from a newer CLI surfaces its real message rather than a blank finish.
- **Dead workspace tools are no longer invisible.** If Rift's file/search/git helper fails to start for a chat, you now get a clear notice up front — before, the only symptom was tools quietly failing one by one.
- **The usage gauge is far harder to break.** A single unexpected value from the (undocumented) usage endpoint used to blank the entire usage panel *and* silently switch off the near-limit warning. It now tolerates odd values field-by-field, so the gauge and the limit warnings keep working.
- **Fixes for edge cases in multi-window and multi-pane use:**
  - Retrying a message no longer risks landing it in a different chat if you switch tabs while it's waiting to send.
  - A pane whose folder was deleted, renamed, or disconnected no longer silently starts reading files and git status from a *different* project — it now shows nothing for that pane instead of the wrong thing.
  - Two windows working the same conversation can no longer corrupt each other's saved "which folder / which model" markers.
  - Dictation always finishes transcribing with the same model it recorded with, even if you changed the model mid-recording.
- **Self-update is steadier.** A slow update check that timed out can no longer clobber a newer check's download, and update/repair actions won't quietly swap the pending update out from under an apply that's already running.
- **Under the hood:** the local request shim and loopback bridge got hardening (fail-loud on malformed input, constant-time token check); the "slow turn start" notice now measures pre-first-output wait correctly instead of being masked by later thinking; internal timing spans attribute to the right source file.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

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
