# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.86.0 — Diff word-highlights, honest update states, sturdier CLI discovery

- **Edit diffs now highlight exactly what changed within a line** — modified lines get GitHub-style word-level emphasis layered over the existing syntax colors, so a one-word edit reads at a glance instead of spot-the-difference. Mostly-rewritten lines deliberately skip the confetti and keep the plain line tint.
- **Silent update-detection failures now speak up (quietly)** — if Rift finds your Claude CLI but can't read its version (which silently gates advanced features off), a calm banner row says so with a one-click path to Settings. If the update check keeps failing after its automatic retries, a quiet row offers Retry. Real update banners always outrank these, and dismissing one lasts for the session only.
- **The CLI is found even right after `npm i -g`** — Rift now also reads your user/system PATH straight from the Windows registry during discovery, so a CLI installed after login (invisible to the app's login-frozen PATH snapshot) is picked up without logging out or rebooting. Custom npm prefixes off the frozen PATH are covered too.
- **Older CLIs get the features they're entitled to** — three internal version gates were confirmed against the official Claude Code changelog and corrected downward (partial streaming, prompt-cache stability, budget caps), so an older install is no longer needlessly degraded.

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.85.x** — Pinned Plan HUD + stream polish (calmer live line, minute-aware durations, reduced-motion); plan-HUD hardening; resumed conversations restore their project folder; "quick" tier retired; fresher CLI status on window focus.
- **v0.84.x** — Adaptability pass (CLI found via pnpm/Volta/Scoop/Bun; honest degradation on dead mic / stuck downloads / corrupt config; credential-keyed warm processes); reasoning ladder; transcript detail; Settings redesign + workspace hub; self-verification follow-ups (unbounded stdout reader revert, maximize state, queue races, copy-failed state).
- **v0.82.x** — Warm-CLI process-leak fixes; stream density controls (Tool detail + presets).
- **v0.79.0–v0.81.0** — Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling.
- **v0.74.0–v0.78.0** — Command output in-stream, calmer narration, steer removed, permission/sub-agent/dictation fixes.
- **v0.66.0–v0.72.x** — Workspace/projects overhaul, fast-by-default, split-pane isolation, unified chat-block look.
- **v0.20.7–v0.65.0** — Foundation era: redesign port, warm-CLI, multi-window, dashboard + AI Health, voice mode, diagnostics console.
