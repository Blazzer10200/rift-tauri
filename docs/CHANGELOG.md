# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.94.0 — See what the assistant is doing, the moment it does it

- **Tool calls appear the instant the model commits to them.** A long file-write used to show nothing until it finished; now the block lands immediately, the file name fills in as it streams, and shell commands "type" themselves out live. Full compatibility pass against the current Claude Code CLI.
- **PowerShell was silently blocked on Windows — fixed.** Every PowerShell call the model made was being denied by the tool allowlist. It now runs, with full display parity: `PS>` badge live, real command captions in history.
- **Plan mode shows the actual plan.** The proposed plan renders as a readable card (open while it awaits your review, collapsed in history), and plan turns now say "Plan proposed" instead of falsely claiming "Applied 1 file" (the CLI writes its own plan artifact — that's not your repo changing).
- **Running agents get a floating fleet bar.** When delegated agents scroll out of view mid-turn, a glassy bar pins to the top: how many are running, what the newest one is doing right now, and one row per agent — click a row to jump to its card. Both floating bars (plan + agents) only appear while their in-chat cards are off-screen, so nothing displays twice, with a soft hand-off at the boundary.
- **A dozen tools now display honestly.** Background-task tail/stop no longer render as checklist cards; skills, slash commands, workflows, and symbol-lookups show real captions instead of bare names; images inside tool results show an `[image]` marker instead of silence.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.93.0** — Turns survive long-blocking tools (UAC/credential prompts get the full 15-min ceiling); interrupted-turn reconciliation note (no more forgotten in-flight deploys); background-agent guidance corrected; edit-card file menus containment-checked; browser dock stays dismissed.
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
