# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.86.4 — Queued-message hardening + queue visibility

- **Queued messages now own their attachments end-to-end.** A message queued mid-turn used to hand its snapshotted image/files back through shared composer state when it fired — a concurrently-typed send could steal them (that turn got the queued files, the queued message went out bare), and the hand-off overwrote anything newly staged in the composer. Attachments now ride the send itself; the queue and the composer can no longer cross-contaminate.
- **Queue order survives races.** A queued message that lost the brief post-stop window to another send re-parks at the *front* of the queue instead of shuffling behind newer messages.
- **Queue chips show what they carry.** A chip with attachments gets a paperclip count badge; an image-only message no longer renders as a blank chip ("2 images" marker instead); and blanking a chip's text while editing no longer silently discards its attachments — the ✕ removes the whole item, editing only changes the text.
- **Parked messages are visible from the sidebar.** A chat with queued messages shows a count badge in the conversation list — background chats hold their queued sends until you return, and the badge is the reminder something is waiting.
- **`/model` caught up with the picker:** accepts `fable`, and no longer offers models that were removed.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

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
