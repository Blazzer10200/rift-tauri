# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Settings overhaul + slash-menu skills

- **Transcript tool display sharpened** — web-search sources get per-domain colored favicons; test/lint result icons take the pass/fail color so the glyph and its pill read as one; the subagent card gets a soft accent-wash head so it stands out as the first-class card it is; read/grep/list rows get a subtle identity tint; a running shell prompt glows in its shell's color and an active tool row shows a gentle scan sweep. Vertical rhythm unified — every block in a turn now sits on one consistent, density-aware gap instead of a jitter of near-equal values. **Fixed:** a narration sentence split across a tool call (the CLI streams "…in p" then the tool then "arallel.") now rejoins into one line instead of rendering a word sliced in half.
- **Every Claude Code tool reads as itself** — gave real icons to eight CLI built-ins that used to show a generic wrench (ToolSearch, ScheduleWakeup, PushNotification, Cron create/list, MCP-resource reads, RemoteTrigger, TaskGet/List). Any tool a future CLI adds still renders as a clean named chip rather than vanishing, and unrecognized stream events now leave a console breadcrumb instead of silently dropping — so the chat stays forward-compatible with CLI updates.
- **Your Claude Code skills in the `/` menu** — user + project skills/commands scanned from `.claude` dirs with descriptions and argument hints; fuzzy filter with highlight, source badges, Tab-inserts-args vs Enter-runs. Menu chrome redesigned (glass panel, count header, styled scrollbar, sticky groups).
- **Settings restructured into 5 tabs** with a search box (press `/` or Ctrl+F) that jumps to and flashes the matching card; redundant cards merged. **Show-don't-tell everywhere:** live code + density preview, accent sample strip, mini activity-stream preview, classic-bubbles preview when stream view is off, real-pipeline reply preview, a live context gauge under the plan picker, collapsed option grids with animated expanders, and slider drag-bubbles.
- **One-click Looks** — five curated accent + texture + vividness combos at the top of Appearance; every dial stays individually tunable after.
- **Per-tab "Reset to defaults"** on Appearance, Chat, Claude, and Speech — with a confirm step; your API key, spending cap, and downloaded Whisper models are never touched.
- **Claude tab reworked:** CLI version chip, billing route strip, contextual Pro note, warn-only tab dot, tightened copy.
- **About tab redone:** version and stack collapse to a chip strip with an inline "Check for updates"; a "Source code & license" row links the GitHub repo; installed local tools shrink to hover-labelled chips (full rows only for missing ones); new "Report a bug" copies your diagnostic and opens a GitHub issue.
- **Update dialog rebuilt** — the version jump (v0.95 → v0.96) is now the hero, with the download size, a "checked N min ago" line, and one-tap release notes. Every state got human copy: a calm "dev build" panel instead of a scary error on dev machines, an offline/firewall message with the raw error tucked into a collapsible, and a reassuring "your chats and settings are kept" on a broken install with a one-click "Get Setup.exe". Downloads are now non-blocking — hide the dialog and the top bar tracks progress.
- **"Updated to vX" welcome-back** — after an update installs and Rift relaunches, a one-time toast confirms the new version with a "What's new" shortcut, and the dialog's up-to-date panel notes what you moved up from. Closes the loop that used to be silent.
- **Fixed:** interface density (compact/regular/comfy) silently never applied — a theme selector outranked it; dyslexia Lexend font never applied app-wide; sidebar scope-toggle thumb dislocating; background-texture picker tiles too faint to tell apart; the "Install" button in Settings → Local tools rendered unstyled (dead CSS class); a banner-started download briefly hid its own progress bar.

## v0.95.1 — Pre-open-source audit

A 13-reviewer adversarial sweep ahead of the source going public. Security surfaces (path containment, XSS sanitization, capability grants, TLS, token auth) all held. Fixed: killed helper processes always reaped (enhance/title/analyze timeouts leaked handles) · process kills moved off the async runtime (AV contention could block a worker) · package metadata now says MIT (was UNLICENSED/missing) · dev-machine paths and infra names scrubbed from scripts, fixtures, CI.


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
