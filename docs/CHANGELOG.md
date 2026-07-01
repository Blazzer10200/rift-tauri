# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Workspace hub revamp, mini-rail, cleaner chrome, full-app polish

### Added
- **The Workspace page is now a real hub.** Projects lead the page as one uniform grid — every card carries live signal (**chat count · last active · total spend**) instead of repeated paths and default scope chips, and the grid orders itself by where you actually work. The active project wears the accent frame with an in-place **Continue** — the old duplicate hero card is gone. Retrospective **Activity** analytics now sit *below* the launch targets, and the whole page fits the default window with **no scrollbar** (scroll only appears when content genuinely needs it).
- **Collapsed sidebar is now a mini-rail, not a disappearance.** Collapsing the sidebar leaves a slim 52px icon column: the Rift mark (which becomes an expand button on hover), a **New chat** button, chat **search**, and your **Workspace / Chat / AI Health / Settings** nav — all still one click away.
- **Workspace dashboard now shows momentum, not just totals.** In the 7-day and 30-day views, each stat tile carries a small trend chip comparing it to the previous window. Rising spend reads as neutral, never as a win. The **spent** tile jumps straight to the AI Health cost breakdown.

### Changed
- **The topbar ⋮ menu is gone — its actions are now one click.** Split editor (in chat), New window, and a **notifications bell** with a visible unread badge sit directly in the topbar. The menu's Search row was a duplicate of the sidebar search / Ctrl+K and was dropped.
- **The Ctrl+K search was redesigned.** Single-line rows with key chips, recent chats tagged with their **project and last activity** ("exfil-v2 · 2h ago"), Split editor / New window reachable from the keyboard, and noticeably less matching noise.
- **Your background texture now covers the whole app.** Workspace, Settings, and the in-app browser were painting solid panels over the texture picked in Appearance; every surface now blends like the chat page always did.

### Fixed
- **The notifications panel no longer covers its own bell** — it opens anchored below it, so clicking the bell again closes it.
- **Deleting a chat right after a reply no longer resurrects it.** A background auto-save or title-generation could re-create a conversation moments after you deleted it, leaving a ghost row that wouldn't delete. Deletions now hold, even mid-save.
- **Notifications stop piling up duplicates.** Repeated identical notifications now collapse into a single entry with a count, instead of stacking and pinning the unread badge.
- **AI Health reads honestly** (dropped a misleading "typical reply" figure; no mid-word truncation), and **dev builds don't cry wolf** with a red "reinstall needed" chip.

## v0.83.0 — Sidebar redesign + Fable 5 always in the picker

### Changed
- **New sidebar layout — project-first and calmer.** The rail is reorganized top-to-bottom: a **project switcher** at the very top (project monogram, name, and current **git branch**), a **New chat + search** row, a **This project / All** scope toggle with a live chat count, your conversations filling the middle, and a compact **icon footer** (Workspace · Chat · AI Health) with a status strip below it (active model + connection state + Settings). Which project *and* branch you're in is now answered at a glance.
- **Project switcher replaces the projects block.** Clicking it opens a dropdown of every project plus **All projects** and **New project**. Everything the old projects list could do still works: open a project, right-click to *open in a split pane*, or drag one onto a pane to grow the split.
- **Simpler conversation history, now easier to manage.** The sidebar opens showing your most recent day's chats with a single **Show earlier** link for the rest. Pinned chats get a subtle accent wash so they read as distinct; date headers (Pinned / Today / Older) stay pinned to the top as you scroll; and hovering a chat reveals quick **pin** and **more** (rename / delete) actions.
- **Search shares the top action row** — the button opens the global **Ctrl+K** command palette scoped to chats. The whole rail reads calmer, with more breathing room.
- **Fable 5 is back in the model picker — and stays there.** The Fable row is now always visible, so the moment Anthropic reopens access it just works, with no update needed. While access is still gated upstream, selecting Fable and sending returns a graceful "currently unavailable" message rather than a crash — that's expected. Nothing changes for Opus, Sonnet, or Haiku.

### Fixed
- **Fable 5 won't hard-error the instant it reopens.** Fable's reasoning is always on, and its API rejects the "turn thinking off" request that every other model accepts. Rift's thinking-off path used to send that request for *any* model, which would have made every Fable turn fail for users on an API key. Fable now correctly skips that path (there's nothing to turn off on an always-thinking model). Nothing changes for the other models.

*(Sidebar redesign follows the "C+ Switcher-led" Claude Design brief; frontend-only, `ProjectRail.svelte` retired in favor of `ProjectSwitcher.svelte`. Fable now visible via the `FABLE_DISABLED = false` lockstep — config.rs + helpers.ts. Verified: cargo test 132/132, svelte-check 0/0, vitest 410/410, live picker CDP-checked.)*

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.82.1** — Warm-CLI process leaks fixed: a duplicate pre-warm race orphaned ~450 MB `claude` helpers invisibly until app exit (gigabytes on busy machines), and helper teardown didn't reap the MCP grandchild. Atomic pre-warm winner + tree-kill on every teardown path. *(ISSUES #76/#77)*

- **v0.82.0** — Dial in how much the activity stream shows: a new three-way **Tool detail** control (Balanced/Minimal/Detailed) plus **Calm/Standard/Verbose** density presets that set all three stream knobs at once; collapsed work rows now name their targets across mixed tool kinds; no more silent Command-output override under Detailed.
- **v0.81.0** — Sonnet 5 gets its full 1M context window (the CLI defaulted it to 200K, so long chats compacted at ~14% while the gauge said 1M); the picker no longer offers unavailable Fable; small context-readout accuracy fixes.

- **v0.80.0** — Stuck sub-agents now get caught: a wedged (but still-chatty) sub-agent could spin the turn forever because the stall watchdog re-armed on every output line; a hard 15-minute in-flight ceiling that stream activity can't extend now force-ends it.
- **v0.79.0** — Claude Sonnet 5: "Sonnet" now actually runs Sonnet 5 (the shipped CLI's bare `sonnet` alias still resolved to 4.6, so Rift now pins the explicit id), reaches the X-High effort tier, and shows clean dateless model labels.
- **v0.78.0** — Queued messages keep their image attachments, short bleeped swear phrases get de-censored in voice dictation, and first-run onboarding fixes (no phantom Haiku option, corrected hints).

- **v0.77.0** — See command output in the stream (Peek/Full/Minimal), project-ghost fix, and one neutral surface for every chat block.

- **v0.76.0** — A calmer activity stream: between-step narration is demoted to quiet inline notes (new three-way **Narration** control: Focused / Balanced / Chatty), so a working turn reads as work-with-commentary, not chat-between-tools.
- **v0.75.0** — Removed the half-working "steer" feature (Alt+Enter live-injection) front-and-back; the message queue (type while it works → fires as the next turn) is now the single way to address a running turn.
- **v0.74.0** — Two bug fixes: permission prompts now appear on the live turn in every non-Bypass mode (gated tools were silently auto-denying after 2 min), and sub-agents reliably register as finished instead of spinning "working…" forever.
- **v0.72.0** — Plan-mode unfreeze (#75), terminal-grade work habits — batches tool calls + skips redundant re-reads (#76), and a unified look for every chat block (#77; the emerald tint from this is what v0.77.0 replaced with neutral gray).

- **v0.71.x** — Path-helper de-dup (one canonical `utils/path.ts`), split-pane isolation (per-pane sub-agent panel, no cross-pane crosstalk), a warm-CLI stale-frame/permission-race bug-fix sweep, the turn-spawn refactor (orchestrator + `resolve_spawn`, lints 14→0), and first-run onboarding rework.
- **v0.66.0–v0.70.0** — Workspace + projects UI overhaul, no-folder scratch workspace, fast-by-default (thinking split into its own toggle), and the warm-pool persistent-process fix.
- **v0.20.7–v0.65.0** — Foundation + diagnostics era: full redesign port, stream design language, warm-CLI process, multi-window sync, Workspace dashboard + AI Health, voice mode, honest mid-chat model switching, and the diagnostics console.
