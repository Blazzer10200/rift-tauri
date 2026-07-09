# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.95.0 — Deep-review hardening + public-debut polish

- **Workspace scoping rules now actually apply on Windows.** Per-project include/exclude patterns (e.g. hiding `secrets/**` from the assistant) were silently ignored — a Windows path-form mismatch made the filter never match, so it failed open. Fixed, with a regression test on real canonicalized paths.
- **Voice dictation over 30 seconds no longer loses its beginning.** The audio buffer only kept the last 30s and the final transcript was built from it; the cap is now 5 minutes.
- **AI Health numbers are honest again.** Model time was being recorded cumulatively instead of per-turn, inflating model attribution and zeroing Rift overhead from turn 2 onward.
- **Updating the CLI mid-conversation asks first.** The updater has to stop live CLI processes to free the binary — with turns running you now get a confirm instead of silent kills.
- **Prompt-enhance can't clobber your typing.** The box locks while enhancing, and undo restores the draft you actually had, not a stale snapshot.
- **Interrupted-turn reconciliation note survives the fast-restart path** (follow-up to v0.93.0 — the pre-warmed next session used to swallow it).
- **Plan diffs deduplicated** — the CLI's own plan artifact no longer renders a second diff under the plan card.
- **Powered by Claude Code, officially MIT.** Settings → About gains a clickable attribution row and a corrected MIT license row; the composer carries a subtle "Claude can make mistakes" note; the context ring opens a compact context-only popover (the full plan-limits panel stays on `/usage` and the status bar).
- **New README product tour** — a 45-second capture of the real app: workspace → chat → split panes → AI Health → live re-theme from Settings.
- Smaller fixes: conversation rename can no longer resurrect a just-deleted chat; retired-model cleanup; usage-cache lock recovery.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.94.0** — Live tool-call display: blocks land the instant the model commits, shell commands type themselves out; PowerShell un-blocked on Windows; plan-mode renders the actual plan card; floating agent fleet bar; honest captions for a dozen tools.
- **v0.93.0** — Turns survive long-blocking tools (UAC/credential prompts get the full 15-min ceiling); interrupted-turn reconciliation note (no more forgotten in-flight deploys); background-agent guidance corrected; edit-card file menus containment-checked; browser dock stays dismissed.
- **v0.92.0** — Mid-chat model switching actually switches (transcript divider marks where); model menu copy honest; clicked file paths open an actions menu.
- **v0.91.0** — Background-agent completions stream into chat (no more silent-forever after "I'll wait for this to finish"); clickable file paths in chat; local previews wait for the dev server to accept connections; assistant-opened pages queue for backgrounded tabs; "Reopen <site>" pill; per-channel CLI update comparison + restart-free tool detection.
- **v0.87.0–v0.90.0** — Per-turn event epochs; terminal-style tool blocks; CLI 2.1.201 compatibility; prompt-suggestion groundwork; voice-failure notices; fresh secondary windows.
- **v0.86.x** — Queued-message hardening; project switching de-staled; ten-fix stability sweep; registry-PATH CLI discovery.
- **v0.84.x–v0.85.x** — Adaptability pass (alt package managers, honest degradation); reasoning ladder; Settings redesign + workspace hub; "quick" tier retired.
- **v0.74.0–v0.82.x** — Warm-CLI process-leak fixes + density controls; Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling; command output in-stream; calmer narration.
- **v0.20.7–v0.72.x** — Foundation era: redesign port, warm-CLI, multi-window, workspace/projects overhaul, split-pane isolation, dashboard + AI Health, voice mode, diagnostics console.
