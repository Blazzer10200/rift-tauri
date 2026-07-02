# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Settings redesign, workspace hub, composer + welcome revamp, live status bar

### Status bar is interactive · usage popover grew up
- **Click the 5h/7d pills** → full Plan-limits popover above the bar: every window w/ reset countdowns, an **IN USE** chip on the binding window, manual refresh, live "updated Xs ago". Pills tint amber/red as a window heats; tooltips carry reset times.
- **Weekly Fable limit shows now** (the endpoint moved model windows into a generic list — future models appear automatically), and **extra-usage credits** ($X of $Y) finally display.
- **Project name → Workspace hub; the Claude item → Settings → Claude** (the door to fixing "Not connected").

### Chat welcome + composer, revamped
- Welcome: session eyebrow over "What's next for *project*?", one slim facts row (branch · files · path + Switch folder/Activity), and **Jump back in** — your three freshest threads, one click to resume. Quick-start chips removed; "New to Rift?" opens into three step cards.
- Composer is **one card**: input + a docked control deck (perm · attach · dictate · draft tools | model · ctx ring · **real send button** — filled accent circle when ready, stop square mid-turn, queued-count badge). The ↵/⇧↵ hints retired into the send tooltip.

### Chat display
- **Hover timestamps** — new messages remember when they were sent; hover a turn head or your bubble for "2:42 AM" (older chats simply show none).
- Glassier user bubble, a hair more reading line-height, and the accent "pool of light" behind the transcript is now fully neutral — no more green wash under every texture.

### Settings — full redesign
- **Every tab is one scrolling page** — the second navigation layer inside each tab is gone; the column is narrower and centered.
- **The header blends into the page** — no more tinted band cutting the top off; titles sit directly on your background texture (Settings, AI Health, Local LLM all match).
- **Accent color is just a color now.** The "Looks" presets that silently changed texture/density are gone: swatches + a hue dial with live degree readout + vividness, and one-click **Reset accent**.
- **The "Chat" tab is now "Claude"**, and every previously bare card opens with a one-line plain-English description. The Vividness slider finally shows a filled track.

### Added
- **Three new background textures — and live preview.** The Appearance picker gains **Blueprint** (accent-tinted two-scale grid), **Rings** (concentric arcs), and **Grain** (film noise), for 12 options total. Hovering any tile now previews the *real* texture live on the app background behind the page — no more guessing from a thumbnail; click commits, move away reverts.
- **The New/Edit project editor got a real face.** Icon + subtitle header, Name and Folder on one row, live "N patterns" counts on valid Include/Exclude globs, a saving spinner, and proper keyboard flow: the Name field focuses on open, **Enter saves, Escape closes**.
- **The Workspace page is now a real hub.** Projects lead as one uniform grid with live signal per card (**chats · last active · spend**), ordered by where you actually work; the active project wears the accent frame with in-place **Continue**. Activity analytics sit below the launch targets, and the page fits the default window with no scrollbar.
- **Collapsed sidebar is now a mini-rail** — a slim 52px icon column keeps New chat, search, and Workspace/Chat/AI Health/Settings one click away.
- **Workspace dashboard shows momentum, not just totals** — 7d/30d stat tiles carry trend chips vs the prior window (rising spend reads neutral); the **spent** tile jumps to AI Health.

### Changed
- **The topbar ⋮ menu is gone — its actions are now one click:** Split editor, New window, and a **notifications bell** with an unread badge sit directly in the topbar.
- **The Ctrl+K search was redesigned** — single-line rows with key chips, recent chats tagged with project + last activity, Split editor / New window from the keyboard, less matching noise.
- **Your background texture now covers the whole app** — Workspace, Settings, and the in-app browser no longer paint solid panels over it.

### Fixed
- **The Margins texture actually shows up now.** Its old elliptical fade peaked at ~27% strength at the screen corners — effectively invisible. It now draws a proper dotted frame along all four edges.
- **The notifications panel no longer covers its own bell**, and repeated identical notifications collapse into one entry with a count.
- **Deleting a chat right after a reply no longer resurrects it** — a background auto-save/title-generation could re-create it moments later; deletions now hold, even mid-save.
- **AI Health reads honestly** (dropped a misleading "typical reply" figure; no mid-word truncation), and **dev builds don't cry wolf** with a red "reinstall needed" chip.

### Fixed — overnight full-stack review (split-pane correctness, error honesty, races)
- **Split panes truly keep to themselves now.** Retrying a turn, continuing a truncated reply, or a queued message auto-firing in a background pane all land in *that* pane's conversation — never the one you happen to be looking at — and a background pane finishing its work no longer yanks your cursor to it. An unfocused pane also stops briefly showing another pane's file count / git branch.
- **Errored turns say what actually went wrong.** A turn that stops on a thinking-budget or context-window limit now keeps its plain-English reason instead of being overwritten by a raw diagnostic dump.
- **The "applied automatically" badge on a past turn reflects the mode it *ran* with**, not whatever permission mode you switched to afterward.
- **AI Health's "Apply" for a reasoning-effort tip now works even when Thinking is off** (it flips Thinking on for you) instead of silently doing nothing while showing a green check.
- **Dismissed voice/ask questions** in history now read "Dismissed — no answer given." instead of leaking the internal instruction text; in-progress answers no longer get wiped if the question list changes mid-stream.
- **Copy-diff** only says "Copied" when the copy truly succeeded; a **too-many-patterns** project glob now blocks Save with a warning instead of silently truncating; assorted backend races hardened (concurrent model-download cancel, interrupted-download resume, project-folder resolution).
- **The sidebar's branch pill stays put** when you open Settings (it used to flip to "Local workspace" mid-navigation).

### Changed — under the hood
- **One toggle-switch style app-wide** (was three near-identical copies), one danger-text color token, ~250 lines of dead CSS/TS removed. No visible change; the UI is just more consistent and lighter.

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
