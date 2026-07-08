# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.93.0 — Turns survive long-blocking tools (and reconcile when they don't)

- **A tool call that waits on a human no longer gets the whole turn killed.** A command blocked on a UAC elevation prompt or credential dialog used to read as "wedged" after ~9 minutes of silence — Rift killed the turn mid-action and the assistant lost all memory of what it had just launched (it could even re-run a deploy it didn't remember doing). Silent-but-running tools now get the full 15-minute ceiling, and the timeout message says which limit actually fired.
- **If a turn IS lost mid-tool, the assistant reconciles instead of forgetting.** Its next message automatically carries a note listing exactly which tool calls were in flight ("may have fully completed — verify before re-running anything non-idempotent") — including on the automatic retry after a crashed CLI process, the case most at risk of a double-run.
- **The assistant now knows background agents survive turns.** Its embedded guidance wrongly claimed all background work dies when the turn ends, steering it into exactly the risky blocking calls above. It now delegates long or interactive work to background agents (which report back on their own) and detaches elevated commands instead of blocking on them.
- **Edit-card file menus validate paths like everywhere else.** The file-actions menu on an edit diff acted on the raw path from the tool call; it now resolves through the same workspace-containment check as clicked paths in chat — which also fixes relative paths that the open/reveal actions couldn't handle.
- **The browser dock stays closed when you close it.** Dismissing the dock while a local preview was still waiting for its dev server to come up could see the dock pop back open by itself seconds later.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.92.0** — Mid-chat model switching actually switches (transcript divider marks where); model menu copy honest; clicked file paths open an actions menu.
- **v0.91.0** — Background-agent completions stream into chat (no more silent-forever after "I'll wait for this to finish"); clickable file paths in chat; local previews wait for the dev server to accept connections; assistant-opened pages queue for backgrounded tabs; "Reopen <site>" pill; per-channel CLI update comparison + restart-free tool detection.
- **v0.90.0** — Prompt-suggestion groundwork (dormant until Anthropic enables it server-side); voice-input failure notices; AI-Health Undo survives Re-analyze; secondary windows always start fresh.
- **v0.89.0** — Claude Code 2.1.201 compatibility (full tool set, spend-cap + unknown-error surfacing); multi-window/multi-pane edge fixes; steadier self-update.
- **v0.88.0** — Terminal-style tool blocks with progressive output reveal; honest live "working" line; WYSIWYG texture previews.
- **v0.87.0** — Per-turn epoch on the event wire (stopped turns can't bleed into the next); window-scoped dictation; daily advisory CI.
- **v0.86.x** — Queued-message hardening; project switching de-staled; ten-fix adversarial stability sweep; registry-PATH CLI discovery.
- **v0.84.x–v0.85.x** — Adaptability pass (alt package managers, honest degradation); reasoning ladder; Settings redesign + workspace hub; "quick" tier retired.
- **v0.74.0–v0.82.x** — Warm-CLI process-leak fixes + density controls; Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling; command output in-stream; calmer narration.
- **v0.20.7–v0.72.x** — Foundation era: redesign port, warm-CLI, multi-window, workspace/projects overhaul, split-pane isolation, dashboard + AI Health, voice mode, diagnostics console.
