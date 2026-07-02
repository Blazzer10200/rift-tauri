# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Settings redesign, workspace hub revamp, mini-rail, cleaner chrome

### Settings — full redesign
- **Every tab is now one scrolling page.** The second navigation layer inside each tab (Theme/Layout, Session/Reading/Cost & keys…) is gone — pick a tab, scroll, done. The column is narrower and centered for comfortable reading.
- **The header blends into the page.** No more tinted band with its own border and icon badge cutting the top off — page titles now sit directly on your background texture (Settings, AI Health, and Local LLM all match).
- **Accent color is just a color now.** The "Looks" presets silently changed your background texture and density along with the accent — that coupling is removed. Accent is a clean set of swatches + a hue dial (with a live degree readout) + vividness, plus a one-click **Reset accent** back to stock emerald.
- **The "Chat" tab is now "Claude"** — that's where your session, plan, and API keys live. Its status dot explains itself on hover.
- **Every card explains itself.** Cards that had bare titles (Cost guard, Whisper model, Local tools, Paths…) now open with a one-line plain-English description. The Vividness slider finally shows a filled track, and About-tab rows align with their card titles.

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
- **Project-first sidebar:** project switcher (monogram + name + git branch) on top, New chat + search row, This-project/All scope toggle, recent-day history with Show earlier, icon footer + status strip.
- **Fable 5 always visible in the picker** — graceful "currently unavailable" while upstream access is gated; no update needed when it opens. Fixed the thinking-off request that would have failed every API-key Fable turn.

*(Frontend `ProjectRail`→`ProjectSwitcher`; `FABLE_DISABLED=false` lockstep config.rs + helpers.ts. Verified: cargo test 132/132, svelte-check 0/0, vitest 410/410.)*

## Known issues
- **Voice profanity on Web Speech:** fully-masked words (`******`, no leading letter) can't be recovered from Azure's servers — the real fix is the on-device **Whisper** engine (built but not yet in the shipped binary). Planned.
- **Dismissed ask-cards** render "(no answer recorded)" rather than a proper "Dismissed" state — cosmetic, fix planned.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.82.x** — Warm-CLI process-leak fixes (orphaned ~450 MB helpers, MCP grandchild reap); stream density controls (Tool detail + Calm/Standard/Verbose presets).
- **v0.79.0–v0.81.0** — Sonnet 5 (explicit id pin, X-High tier, full 1M context); stuck-sub-agent 15-min hard ceiling.
- **v0.74.0–v0.78.0** — Command output in the stream, calmer narration, steer removed (queue is the way), permission-prompt + sub-agent-finish fixes, queued-image + dictation fixes.
- **v0.66.0–v0.72.x** — Workspace/projects overhaul, fast-by-default, warm-pool fix, split-pane isolation, plan-mode unfreeze, unified chat-block look.
- **v0.20.7–v0.65.0** — Foundation + diagnostics era: full redesign port, stream design language, warm-CLI process, multi-window sync, Workspace dashboard + AI Health, voice mode, honest mid-chat model switching, and the diagnostics console.
