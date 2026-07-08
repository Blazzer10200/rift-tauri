# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.92.0 — Switching models mid-chat now actually switches

- **Picking a new model mid-conversation now takes effect on your next message.** Previously a conversation was silently locked to the model it started with — you could pick Fable mid-chat and the next reply still came from Sonnet, with only a small note buried in the picker admitting it. The lock existed to dodge an API rejection that could permanently wedge a resumed chat; the Claude CLI has since fixed that (verified against live sessions), so the lock is gone — your pick wins now.
- **The chat shows where the switch happened.** A slim "Model switched · Sonnet 5 → Fable 5" divider appears in the transcript right above the first message that runs on the new model — and it's saved with the conversation, so reopened chats keep the marker.
- **The model menu tells the truth about switching.** The old "Switching models only applies to a new chat" note now reads "Your next message switches it to <model>" — with "New chat in <model>" still offered as the fresh-start alternative.
- **Clicked file paths open an actions menu.** Clicking a file path in chat now pops the same file-actions menu used everywhere else in the app — open in VS Code (at the exact line), open with the default app, reveal in the file manager, or copy the path — instead of jumping straight into VS Code.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.91.0** — Background-agent completions stream into chat (no more silent-forever after "I'll wait for this to finish"); clickable file paths in chat; local previews wait for the dev server to accept connections; assistant-opened pages queue for backgrounded tabs; "Reopen <site>" pill; per-channel CLI update comparison + restart-free tool detection.
- **v0.90.0** — Prompt-suggestion groundwork (dormant until Anthropic enables it server-side); voice-input failure notices; assistant-opened pages pull you to Chat; AI-Health Undo survives Re-analyze; chat delete sweeps dead CLI session files; secondary windows always start fresh.
- **v0.89.0** — Claude Code 2.1.201 compatibility (full 2.1.19x–2.1.20x tool set, spend-cap + unknown-error surfacing); dead-workspace-tools notice; usage gauge tolerates odd values field-by-field; multi-window/multi-pane edge fixes (retry tab-identity, stale-folder honesty, sidecar cross-talk, dictation model pinning); steadier self-update; bridge/shim hardening.
- **v0.88.0** — Terminal-style tool blocks (input line + labeled output, `exit N`); progressive "Show N more lines"; honest live "working" line; WYSIWYG texture previews; stream-event handling extracted + tested.
- **v0.87.0** — Per-turn epoch on the event wire (stopped turns can't bleed into the next); self-update kills by exe-path; image paste/drop target-locking; window-scoped dictation; honest MCP notify/open_browser; advisory scans → daily CI.
- **v0.86.x** — Queued-message hardening (attachments ride the send, badges, ordering); project switching de-staled; clean mid-turn exit reaps every CLI child; a ten-fix adversarial stability sweep; diff word-level highlights; registry-PATH CLI discovery.
- **v0.84.x–v0.85.x** — Adaptability pass (CLI found via pnpm/Volta/Scoop/Bun; honest degradation on dead mic / stuck downloads); reasoning ladder; Settings redesign + workspace hub; pinned Plan HUD + stream polish; resumed convos restore their folder; "quick" tier retired.
- **v0.74.0–v0.82.x** — Warm-CLI process-leak fixes + density controls; Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling; command output in-stream; calmer narration.
- **v0.20.7–v0.72.x** — Foundation era: redesign port, warm-CLI, multi-window, workspace/projects overhaul, split-pane isolation, dashboard + AI Health, voice mode, diagnostics console.
