# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-04 (cont. 33) — ACTIVITY PANEL REORG + SESSION DIFF (UNSHIPPED v0.4.46)

Frontend-only. `npm run check` 4081/0/0. **Verified live against a REAL assistant turn (CDP)** — not just preview.

**Activity panel** (`ActivityPanel.svelte`): killed Now/Steps double-spin — live units live ONLY in the Now cluster (headline + `.now-live` rows at 2+ running); **Steps = settled, actions-only** (`logSteps` filters out `cat==="write"`). New **Outputs** section = deduped write/edit artifacts w/ net +/−, opens Diff. Timeline spine (`.rows::before`, icons z1 punch through). Done-cap enriched. Path rows rtl-clip (keep extension).

**Session Diff** (NEW `SessionDiff.svelte`): full-pane review of every edit this convo, grouped by file; reuses `EditDiff` via new `hideHead` prop; Write→all-adds (`old_string:""`), MultiEdit expands. Reads `tab.messages` (real). State `assistant.ui.diffOpen/diffTarget`. 5 entry points, all verified live: **Review-diff btn** (`MessageBubble.reviewDiff` — was DEAD, scrolled to removed `actnode-*` anchor; now opens overlay deep-linked via new `turnStats.firstEditFile`), Outputs Diff-link + row, `Ctrl+Shift+D` (AppShell), View-menu "Session diff" (`ChatTabsBar`).

**Preview** (flask toggle + `activity-preview.svelte.ts`) added to iterate, then **fully removed** per user — file deleted, every `preview.on?` branch reverted to real source. Grep-clean.

### 🟡 Next / flags
- Done-cap counts all settled tools (incl writes) → shows "7 steps" while Steps badge shows 5. Left as whole-turn total; relabel "actions" if it reads odd.
- `classifyTool` + SessionDiff don't strip `mcp__rift__` prefix (MessageBubble does, ln 355). CLI built-in Read/Write render fine; rift MCP `read_file/list_dir/grep` would show as generic `meta` rows — map+strip next pass.
- Scratch `.tmp/activity-test.md` left in dev workspace from live test — deletable.

## Session 2026-06-04 (cont. 32) — SHELL NAV REDESIGN (titlebar nav, UNSHIPPED v0.4.46)

Frontend-only. `npm run check` 4080/0/0; CDP-verified live (titlebar + chat both shotted).

**Left activity column removed.** 3-destination vertical rail read as wasted space → nav moved into the titlebar. `Titlebar.svelte`: horizontal Home/Chat nav (`.navitem`, accent-soft active pill @ 0.12α, Ctrl+1/2) + brand↔nav hairline `.nav-sep`; Settings → gear in `.right` group; ⌘K placeholder "Search or run a command…", max-width 360→300. `AppShell.svelte`: `<ActivityBar/>` removed, `.body` grid → single `1fr`.

**Chat-history rail toggle relocated.** Floating `>` sliver killed (`ChatRail.svelte`: collapsed = `width:0;border:0`; removed `.cr-expand-btn`, `ChevronRight`, local `RAIL_KEY`). New `.rail-toggle` PanelLeft at left edge of `ChatTabsBar.svelte` — contextual to Chat, mirrors the right-side Activity PanelRight. Collapse state centralized → `uiPrefs.toggleChatRail()` (key `rift.ui.chat-rail-collapsed.v1`).

**Two earlier-this-session fixes.** (1) Model dropdown opacity (`app.css .rift-menu`): translucent `color-mix(surface 86%,transparent)`+`backdrop-filter blur` → solid `var(--surface)`+`var(--border)`; shared chrome so slash/mention/file-path menus go opaque too. (2) Activity dock resize bar (`AssistantPane.svelte .dock-resize`): z-index 2→6 so the green seam clears the sticky `.dock-head` (z5) instead of being clipped at the header.

### 🟡 Flags / next
- **Orphaned (cleanup candidate):** `ActivityBar.svelte` no longer mounted (left in place, NOT deleted per safety); `railPinned`/`applyRail`/`--rail-w`/`RAIL_PINNED_KEY` in `ui-prefs.svelte.ts` now dead. Workspace **drag-to-reorder** lost with the bar (3 fixed dests → likely fine). Remove the lot in one pass if confirmed.
- `HomePage` "Still up" eyebrow = intentional time-greeting (`hr<5`), left as-is.
- CHANGELOG/version bump deferred to /git-ship per policy — cont.13–32 ships as ONE commit.

## Earlier this arc (cont. 13–31) — detail in `git log`
- **31/30:** logo on platform icons; theming `--bg-inset` 0.178 + `--field`/`--track`/`--code-*`; body hue 270→250; tint mixes oklch→**oklab** (13 files).
- **29–27:** remote-shell rip; Settings IA 6→5; `AskUserRegistry`; CLI-update detector. ⚠️ **CSP** `connect-src` keeps `https://registry.npmjs.org` — don't remove.
- **20/21 PURE-ASSISTANT:** SFTP/sync/server/RCON ripped; MCP→`read_file/list_dir/grep`+`git_*`; IA=3 workspaces.
- **Open:** orphaned `closeAllTabs()`; `cargo machete` deps + stale `SAFE_MCP` (`mod.rs:2425`); STT blocked (whisper-rs 0.13→0.16).
- **Entire cont.13–33 UNSHIPPED on v0.4.46 — ship as ONE commit.** CDP: `bash scripts/cdp/c.sh {shot|eval|click}` (dev via `run-dev.bat` + `npm run cdp:serve`). Nav Ctrl+digit; Alt+digit tabs.

## CRITICAL DON'T-TOUCH
- **Onboarding gate:** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && !assistant.hasApiKey && !assistant.auth?.loggedIn`. `configLoaded` gates timing so it never flashes pre-probe.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Components use `var(--accent)`/`--accent-soft`. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (cont.30 — oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 3 workspaces** — home·1 chat·2 settings·3. Nav lives in the **titlebar** now (cont.32, no left activity column): Home/Chat `.navitem`s + Settings gear in `Titlebar.svelte`; switching still via `workspace.setActive`/Ctrl+1-3. Settings = one scroll-doc, **5 sections** (Appearance landing · Accessibility · Assistant · Speech · About).
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split (cont.33):** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → opens Session Diff. Live units render ONLY in the Now cluster (don't re-add pending/writes to Steps). `SessionDiff.svelte` reads `tab.messages` (real) via `EditDiff` `hideHead`; open via `assistant.ui.diffOpen/diffTarget`. `MessageBubble.reviewDiff` deep-links by `firstEditFile` basename — don't repoint at `actnode-*` (removed).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. v0.4.46 stands.
