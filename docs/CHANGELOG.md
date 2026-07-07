# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.91.0 — The assistant finishes what it starts + clickable file paths

- **The assistant now reports back when a background agent finishes.** The biggest fix in this release: when Claude delegates work to a background agent (or kicks off a build and says "I'll let you know how it goes"), it used to go silent forever once that work completed — the answer never arrived. Rift now catches the assistant's follow-up and streams it into the chat as it happens, so "I'll wait for this to finish" actually turns into a real reply.
- **File paths in chat are clickable.** Any file path the assistant mentions (like `src-tauri/src/lib.rs`) now glows and opens straight in your editor when clicked — jumping to the exact line if one is given. Bare filenames get found anywhere in your workspace; folders open in your file manager.
- **Local previews wait for the server to be ready.** When the assistant opens a `localhost` preview right after starting a dev server, Rift now waits for the server to actually accept connections before loading — no more landing on a "can't reach this page" error because it opened a second too early.
- **Assistant-opened pages no longer get lost.** If the assistant opens a page while you're looking at a *different* chat tab, the page now waits and opens the instant you switch back to that chat — instead of being silently dropped (while the assistant thought it had shown you the page).
- **Reopen your last page in one click.** Closing the built-in browser and reopening it now offers a "Reopen <site>" button so you don't have to retype the address.
- **CLI update checks respect the right release channel.** Installs that track Claude Code's native installer channel are no longer nagged to "update" against npm's faster-moving channel — each install is now compared against its own feed, so "up to date" means up to date. Freshly-installed developer tools (Cargo, etc.) are also detected without an app restart.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.90.0** — Prompt-suggestion groundwork (dormant until Anthropic enables it server-side); voice-input failure notices; assistant-opened pages pull you to Chat; AI-Health Undo survives Re-analyze; chat delete sweeps dead CLI session files; secondary windows always start fresh.
- **v0.89.0** — Claude Code 2.1.201 compatibility (full 2.1.19x–2.1.20x tool set, spend-cap + unknown-error surfacing); dead-workspace-tools notice; usage gauge tolerates odd values field-by-field; multi-window/multi-pane edge fixes (retry tab-identity, stale-folder honesty, sidecar cross-talk, dictation model pinning); steadier self-update; bridge/shim hardening.
- **v0.88.0** — Terminal-style tool blocks (input line + labeled output, `exit N`); progressive "Show N more lines"; honest live "working" line; WYSIWYG texture previews; stream-event handling extracted + tested.
- **v0.87.0** — Per-turn epoch on the event wire (stopped turns can't bleed into the next); self-update kills by exe-path; image paste/drop target-locking; window-scoped dictation; honest MCP notify/open_browser; advisory scans → daily CI.
- **v0.86.x** — Queued-message hardening (attachments ride the send, badges, ordering); project switching de-staled; clean mid-turn exit reaps every CLI child; a ten-fix adversarial stability sweep; diff word-level highlights; registry-PATH CLI discovery.
- **v0.84.x–v0.85.x** — Adaptability pass (CLI found via pnpm/Volta/Scoop/Bun; honest degradation on dead mic / stuck downloads); reasoning ladder; Settings redesign + workspace hub; pinned Plan HUD + stream polish; resumed convos restore their folder; "quick" tier retired.
- **v0.74.0–v0.82.x** — Warm-CLI process-leak fixes + density controls; Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling; command output in-stream; calmer narration.
- **v0.20.7–v0.72.x** — Foundation era: redesign port, warm-CLI, multi-window, workspace/projects overhaul, split-pane isolation, dashboard + AI Health, voice mode, diagnostics console.
