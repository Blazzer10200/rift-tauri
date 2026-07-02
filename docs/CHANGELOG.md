# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Reasoning ladder, transcript detail, settings redesign, workspace hub, live status bar

### Reasoning effort, honest and visible
- **The Thinking toggle is gone — it was an effort jump in disguise.** One four-rung ladder (Low / Medium / High / X-High): Low answers immediately, higher rungs think longer. Fable explains its always-on reasoning; Haiku simply hides the ladder.
- **A calmer picker:** segmented rung cards replace the drag slider, the hotkey cheat-strip is gone, and every model row shows its live rate-limit state.

### The transcript shows its work
- **Terminal blocks know their shell** — bash, PowerShell, and cmd each get a colored badge; output is ANSI-clean and one hover-button copies command + output together.
- **Every tool row reports what came back** — "→ 15 lines", "→ 91 files", "no matches", durations ≥1s, a red *failed* when it broke; MCP tools show their input and an output peek.
- **Created files show their content** in the transcript (up to 60 lines) — a new file is the payload, not a "+29 −0" stub.
- **Streaming text stays put** — narration you watched stream no longer collapses or demotes itself when the next block starts.
- **Markdown polish:** numbered lists show their numbers again; PowerShell and Batch code blocks get real syntax highlighting.

### Status bar is interactive · usage popover grew up
- **Click the 5h/7d pills** → full Plan-limits popover (reset countdowns, **IN USE** chip, manual refresh); pills tint amber/red as a window heats; weekly Fable limit + extra-usage credits now display. Project name → Workspace hub; Claude item → Settings → Claude.

### Chat welcome + composer, revamped
- Welcome: session eyebrow, one slim facts row, and **Jump back in** — your three freshest threads, one click to resume.
- Composer is **one card**: input + docked control deck with a **real send button** (accent fill when ready, stop square mid-turn, queued-count badge).
- **Hover timestamps** on new messages; glassier user bubble; the accent "pool of light" behind the transcript is now fully neutral.

### Settings — full redesign
- Every tab is one scrolling page; the header blends into your background texture; accent is just a color (swatches + hue dial + vividness + one-click reset); the "Chat" tab is now "Claude" with plain-English card descriptions.

### Added
- **Three new background textures** (Blueprint, Rings, Grain — 12 total); hover previews paint the real app background.
- **Workspace page is a real hub** — uniform project grid with live signal (chats · last active · spend), accent frame + in-place **Continue** on the active project, momentum stat tiles.
- **Project editor got a real face** — icon header, live glob counts, Enter saves / Escape closes.
- **Collapsed sidebar is a 52px mini-rail**; topbar ⋮ menu replaced by direct Split / New window / notification-bell buttons; Ctrl+K search redesigned; textures cover the whole app.

### Fixed
- **Margins texture actually visible** (proper dotted edge frame); notifications panel no longer covers its bell; repeated notifications coalesce into one entry.
- **Deleting a chat right after a reply holds** — a background auto-save can no longer resurrect it.
- **Split panes truly keep to themselves** — retries, continues, and queued sends land in *their* pane; no cursor-yank, no cross-pane branch/file bleed.
- **Errored turns keep their plain-English reason**; the "applied automatically" badge reflects the mode a turn *ran* with; dismissed voice/ask questions read "Dismissed — no answer given."
- AI Health reads honestly; the effort tip's Apply works with Thinking off; copy-diff only claims success when it copied; oversized project globs block Save loudly; assorted backend races hardened.

### Changed — under the hood
- One toggle-switch style app-wide, one danger-text token, ~250 lines of dead CSS/TS plus orphaned helpers removed.

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
