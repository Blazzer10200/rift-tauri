# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — stream polish: pinned plan, calmer live line

- **The plan never scrolls away anymore** — a pinned, glassy Plan HUD floats at the top of the conversation while a plan is active: current task, slim progress bar, done/total count. Click it to expand the full checklist; when everything completes it flashes green for a moment and retires itself. The inline plan card in the transcript stays as the historical record.
- **The blinking green streaming caret is gone** — the flashing block at the tail of streaming text read as visual noise, not signal. The pulsing head dot + shimmer already say "live".
- **The live status line grew up** — elapsed time reads `3m 56s` instead of `236s` (the ticking "Thinking…" header too), the seconds/tokens meta moved to a quiet right-aligned mono cluster instead of crowding the verb, each verb swap eases in instead of hard-cutting, and the rotating words were curated to a calmer set (Tracing, Distilling, Untangling… — Sussing and Noodling retired).
- **Minute-aware durations everywhere** — "Thought for 236s" and long tool-chip durations now read `3m 56s` too.
- **Reduced motion is actually reduced** — the stream's entrance slides and pulsing live-dots now honor the OS reduced-motion setting (shimmer and counters already did).

## v0.84.1 — Follow-up fixes from self-verification

- **Warm sessions no longer respawn spuriously** — a v0.84.0 stdout line-cap change accidentally capped the *whole* stream instead of a single line, so a long session that streamed more than ~8 MB total would falsely look like a dead process and cold-restart. Reverted; the reader is unbounded again (it reads our own CLI, not an untrusted socket).
- **The maximize button now shows its real state** on the normal-mode window chrome (v0.84.0 only fixed the onboarding titlebar, which isn't the one you see day-to-day) — the icon and tooltip switch to Restore when maximized.
- **Three more tab-close paths** (close others / close all / close-to-the-right) now clear a tab's queued messages before stopping it, closing the same misfire race v0.84.0 fixed for single-tab close.
- **Terminal copy button shows a failed state** on a clipboard error (the state was tracked but never rendered); a tiny edit to a CRLF file no longer defaults to collapsed.

## v0.84.0 — Built for your machine: adaptability & correctness pass, reasoning ladder, transcript detail, settings redesign

### Works on more than one machine
- **Rift now finds Claude wherever you installed it** — pnpm-global, Volta, Scoop, and Bun install locations are probed alongside npm and the native installer; a Scoop/other shim is no longer mislabeled "npm" (so the *right* update command is suggested).
- **Honest degradation when things aren't set up** — the usage gauge backs off instead of hammering a failing endpoint for signed-out / API-key users; one malformed rate-limit entry can no longer blank the whole usage panel; a corrupt `config.json` is backed up (not silently wiped) before defaults load, so your projects/roots survive.
- **Dead mic, stuck downloads, gone drives — all speak up now:** a mic that's denied or missing shows an actionable hint on the button (not just in Settings); a Whisper download that drops mid-stream shows *failed*, not a frozen bar; opening a project whose folder was deleted or unplugged surfaces a warning instead of dumping you on a blank tab.
- **Rotate your API key or local-LLM endpoint mid-session** and the next turn actually uses it — warm processes are keyed to the credential now, not just "is a key set".

### Fewer ways to lose work or get a wrong answer
- **Closing a busy tab can't misfire a queued message** into the tab that's about to vanish; dragging a project into a split pane refreshes its file list and branch (no more stale @-mentions from the wrong project).
- **Installing an update warns first** if a conversation is live, instead of tearing it down silently.
- **The Enhance "Undo" can't reach across tabs** and overwrite what you typed somewhere else; diffs line up even when line-endings differ (CRLF vs LF); a giant single-line tool result is capped so it can't bloat the view.
- **Links only open what they should** — the in-app link handler defaults to deny, not fall-through. Aborted ask-cards read as cancelled, not a perpetual "Connecting…". Copy buttons only claim success when the clipboard actually took it.

### Under the hood (backend hardening)
- Plugged process/PID leaks on spawn-failure paths; stalled turns clear pending ask/permission registrations; the child-stdout reader is length-capped; update-apply is guarded against double-fire and re-arms after a failure; the perf log rotates on long-lived processes; grep's file count no longer conflates skipped binaries with searched files.

### Also in this release (the cont.257–265 arc)
- **Reasoning ladder replaces the Thinking toggle** — one honest four-rung dial (Low / Medium / High / X-High) instead of a toggle that was secretly an effort jump. Segmented rung cards, live rate-limit chips per model row; Fable explains its always-on reasoning, Haiku hides the ladder.
- **The transcript shows its work** — shell badges (bash / PowerShell / cmd) with ANSI-clean copyable output; every tool row reports what came back ("→ 15 lines", *failed*, durations); created files show their content inline; streaming text no longer collapses mid-turn; chrome fades in with its first word instead of popping in as empty gray boxes; numbered lists and PowerShell/Batch highlighting restored.
- **Interactive status bar** — the 5h/7d pills open a full Plan-limits popover (reset countdowns, IN USE chip, manual refresh, weekly Fable limit + credits) and tint amber/red as a window heats.
- **Welcome + composer revamp** — Jump-back-in resumes your three freshest threads; the composer is one card with a real send button; hover timestamps; the accent wash behind the transcript went neutral.
- **Settings full redesign** — each tab one scrolling page, texture-blended header, accent-is-just-a-color; three new textures (12 total) with live hover previews; Workspace is a real hub (project grid with chats · last-active · spend); 52px collapsed mini-rail.
- **Fixes** — split panes truly isolate (retries/continues/queued sends stay in their pane); deleting a chat after a reply holds; errored turns keep their plain-English reason; ~250 lines of dead CSS/TS removed.

## v0.83.0 — Sidebar redesign + Fable 5 always in the picker
- **Project-first sidebar:** switcher (monogram + branch) on top, New chat + search, scope toggle, recent-day history, icon footer.
- **Fable 5 always visible** — graceful "currently unavailable" while gated; fixed the thinking-off request that failed API-key Fable turns.

*(`ProjectRail`→`ProjectSwitcher`; `FABLE_DISABLED=false` lockstep. cargo test 132/132, svelte-check 0/0, vitest 410/410.)*

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.82.x** — Warm-CLI process-leak fixes; stream density controls (Tool detail + presets).
- **v0.79.0–v0.81.0** — Sonnet 5 (X-High, 1M context); stuck-sub-agent 15-min ceiling.
- **v0.74.0–v0.78.0** — Command output in-stream, calmer narration, steer removed, permission/sub-agent/dictation fixes.
- **v0.66.0–v0.72.x** — Workspace/projects overhaul, fast-by-default, split-pane isolation, unified chat-block look.
- **v0.20.7–v0.65.0** — Foundation era: redesign port, warm-CLI, multi-window, dashboard + AI Health, voice mode, diagnostics console.
