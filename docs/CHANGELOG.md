# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Settings redesign, workspace hub revamp, mini-rail, cleaner chrome

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
