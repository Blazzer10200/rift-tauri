# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.87.0 — Per-turn epoch + final-day deep-hunt fixes

- **Stopped turns can never bleed into the next one (#80 structural fix).** Every event a turn emits — stream frames, done, error, session-lost — now carries the turn's own epoch, and the app discards anything from a stopped or superseded turn no matter how late it arrives. Closes the residual window where a straggling terminal (watchdog kill, slow EOF) could silently finalize or corrupt the *next* message on the same chat. Live-verified: stop mid-stream + instant re-send runs clean, no spurious error banner.
- **Self-update can no longer kill another running Rift.** The update's process sweep matched by image *name*, so a dev build applying an update could force-kill the real installed app mid-session. It now kills only processes running the same executable file.
- **Deleting a chat sticks.** An autosave already in flight could land *after* the delete and resurrect the conversation as an undeletable ghost that returned next launch. Deletes now drain in-flight saves first.
- **Pasted/dropped images land in the right chat.** Attaching is async (base64 encode); switching tabs mid-encode used to drop the image into the *new* tab. The target is now locked at paste/drop time; the stale attach-error banner no longer leaks across tabs either.
- **A hung npm can't freeze Send.** The npm-prefix probe on the CLI-discovery path had no timeout (siblings did) — now bounded at 5s.
- **Dictation is window-scoped.** In a second window, the mic UI no longer mirrors a recording the other window owns (all `stt://` events now target the owning window).
- **Assistant honesty:** `notify`/`open_browser` MCP tools now report failure when the target window is gone instead of claiming success for a toast nobody saw.
- **CI hygiene:** advisory scans (cargo/npm audit) moved to a daily scheduled workflow — pushes only fail when code actually breaks, ending the failure-email spam; the `quick-xml` advisory itself fixed via `plist 1.10`.
- **Projects:** active-project highlight now derives purely from the open folder (stale stored id removed); switcher dropdown no longer floats at stale coords after a window resize.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

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
