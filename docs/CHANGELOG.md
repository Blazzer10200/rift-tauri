# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.95.1 — Pre-open-source audit

A 13-reviewer adversarial sweep of the entire codebase — both stacks, security-first — ahead of the source going public. The security surfaces (path containment, XSS sanitization, capability grants, TLS, token auth) all held under adversarial reading. What it did catch, now fixed:

- **Killed helper processes are now always reaped.** The prompt-enhance, title, and AI-Health analyze timeouts killed their CLI child but never waited on it, leaking the process handle.
- **Process kills moved off the async runtime.** Tree-killing a CLI child could block an async worker thread for seconds under antivirus contention; kills from live-turn paths now run on the blocking pool.
- **Package metadata now tells the truth: MIT.** `package.json` still claimed UNLICENSED and the Rust manifest had no license field — both contradicted the repo's actual MIT license.
- Housekeeping: dev-machine paths removed from stress scripts and test fixtures, infra names scrubbed from CI comments, one hot-path allocation hoisted.


## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.95.0** — Deep-review hardening: Windows workspace-scoping globs actually apply (was fail-open); >30s dictations keep their start; honest per-turn AI-Health attribution; CLI update asks before killing live turns; enhance can't clobber drafts; MIT attribution row + README product tour.
- **v0.94.0** — Live tool-call display: blocks land the instant the model commits, shell commands type themselves out; PowerShell un-blocked on Windows; plan-mode renders the actual plan card; floating agent fleet bar; honest captions for a dozen tools.
- **v0.93.0** — Turns survive long-blocking tools (UAC/credential prompts get the full 15-min ceiling); interrupted-turn reconciliation note (no more forgotten in-flight deploys); background-agent guidance corrected; edit-card file menus containment-checked; browser dock stays dismissed.
- **v0.92.0** — Mid-chat model switching actually switches (transcript divider marks where); model menu copy honest; clicked file paths open an actions menu.
- **v0.91.0** — Background-agent completions stream into chat (no more silent-forever after "I'll wait for this to finish"); clickable file paths in chat; local previews wait for the dev server to accept connections; assistant-opened pages queue for backgrounded tabs; "Reopen <site>" pill; per-channel CLI update comparison + restart-free tool detection.
- **v0.87.0–v0.90.0** — Per-turn event epochs; terminal-style tool blocks; CLI 2.1.201 compatibility; prompt-suggestion groundwork; voice-failure notices; fresh secondary windows.
- **v0.86.x** — Queued-message hardening; project switching de-staled; ten-fix stability sweep; registry-PATH CLI discovery.
- **v0.84.x–v0.85.x** — Adaptability pass (alt package managers, honest degradation); reasoning ladder; Settings redesign + workspace hub; "quick" tier retired.
- **v0.74.0–v0.82.x** — Warm-CLI process-leak fixes + density controls; Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling; command output in-stream; calmer narration.
- **v0.20.7–v0.72.x** — Foundation era: redesign port, warm-CLI, multi-window, workspace/projects overhaul, split-pane isolation, dashboard + AI Health, voice mode, diagnostics console.
