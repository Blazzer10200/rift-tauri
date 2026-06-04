# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-04 (cont. 37) — EDIT-SWARM 2nd PASS: 47 REMAINING FE FINDINGS CLEARED

2nd swarm pass over all 3 buckets (state-ts 14 · comp-assistant 17 · comp-other 16 = 47). All 47 now **resolved or flagged**. `npm run check` 4080/0/0 at every commit. `edit-done.json` 62→88.
- **10 applied + committed:** `90ff50f` state-ts (F172 stt `destroy()`, F180 restoreTabs single-load, F215/F217 everOpened) · `80d24f1` comp-assistant (F30 no openTab on right-click, F134 effort-slider keyboard-reachable) · `ad04775` comp-other (F164 progressbar aria-label, F232 Confirm dontAsk reset, F198 Select aria-disabled+CSS) · `092f07f` hand-done F173 (stt per-channel listen try/catch).
- **12 verified already-fixed** (recorded as done, grep-confirmed): F20,F27,F29,F155,F184,F223,F228,F148,F174,F206,F207,F209 — fixed in 1st pass / coincidentally; swarm correctly deferred.
- **4 resolved-by-deletion:** F167–F170 target `ChatRail.svelte` (deleted cont.35).
- **21 FLAGGED — genuine judgment, NOT applied, NOT in done.json:**
  - state-ts (9): **F24** config-mutator try/catch (6 mutators) · **F48** thinking-effort/permission-mode dual-storage split · **F51** compaction `forceNextFirstTurn` persistence · **F140** StreamEnvelope `usage` type · **F146** removeQueued tabId param · **F147** void send() catch · **F204** telemetry O(M×N) per-model avg · **F224** toast resume remaining-time · **F226** accessibility init try/catch.
  - comp-assistant (4): **F135** settings-menu mixed-ARIA roles · **F165** Markdown `everStreamed` $derived→$effect (1-frame reveal regression) · **F166** ActivityPanel 1s-ticker re-render · **F182** AssistantWelcome greeting reactivity (needs clock tick, $derived insufficient).
  - comp-other (8): **F33** Markdown per-tick DOM reparse · **F44** toast pause/resume remaining-time (store-side, pairs w/ F224) · **F150** role=menu arrow-key roving-tabindex · **F187** ActivityBar pointer-capture ownership · **F189** HomePage greeting (same as F182) · **F190** HomePage branch stale-write race · **F191** branch-load fail-loud (state-side) · **F230** EmptyState tone — *audit's suggested_fix is WRONG* (CSS targets `.empty-glyph`, not `.empty`; real fix needs a `.empty[data-tone]` rule or restructure).

### RESUME HERE — frontend audit swarm COMPLETE
All swarmable + mechanical FE findings done. Remaining work = the 21 judgment calls above (hand-do per-item, not swarmable) + the permanent HELD set below. Pick any flagged ID and implement manually.
- **HELD — never swarm (manual only):** 3 highs (`mod.rs:2143`, `mcp_server.rs:302`, F39 `AssistantPage:205`) · all Rust (quit tauri dev first) · all security · vitest F237 (`npm i -D vitest@^4.1.8`) · dead-CSS F233/F234 (flag-not-delete). `edit-batch.py` auto-holds these.

Tracking: `.tmp/edit-done.json` (88 IDs). Re-emit any bucket: `python scripts/edit-batch.py <bucket>` (auto-excludes done+held).

## Session 2026-06-04 (cont. 36) — EDIT-SWARM: 64 FRONTEND FIXES + TOOLING HARDENED

Edit-swarm 1st pass: 4 batches, each gated 4080/0/0, committed separately. **64 fixes** (`f1446bc`/`9d1f3d1`/`fea1df3`/`f127664`) — reactivity/keying, fail-loud .catch, a11y, type-safety, leaks. **Tooling hardened (`942ad5c`):** `edit-apply.py` atomic per-finding (+ writes `.tmp/edit-last-touched.txt` for scoped commits); `edit-batch.py` auto-holds Rust/security/deps/deletions/pkg + `HELD_IDS`. ⚠️ Use scoped `git add` (applier emits path list) — `git add -A` once swept a concurrent session's work. Detail: `git log`.

## Session 2026-06-04 (cont. 35) — BACKDROP CALM + LEFT CHAT-RAIL RETIRED (UNSHIPPED v0.4.46)

Frontend-only, concurrent w/ cont.34 swarm. 4080/0/0, CDP-verified. **Atmos backdrop** (`AssistantPane.svelte` `.atmos-glow`): killed top-edge accent band → ambient neutral pool behind hero (`radial 120% 80% at 50% 34%`, accent 3% + `--fg` 1.5%) + faint floor vignette, opacity 0.85, pane-wide. **Left chat rail RETIRED** — `ChatRail.svelte` deleted, all `chatRail*`/`.rail-toggle` state stripped; history lives only in `HistoryDrawer` (View menu). 🟡 `railPinned`/`applyRail()`/`--rail-w`/`RAIL_PINNED_KEY` in `ui-prefs.svelte.ts` still dead (cont.32 orphan, user-deferred). Detail: `git log`.

## Session 2026-06-04 (cont. 34) — SWARM AUDIT + EDIT-SWARM INFRA

**Read `docs/audit-2026-06-04/README.md` first.** Two multi-agent Workflow pipelines built:
- **Audit swarm** (`scripts/audit-swarm.workflow.js`): 217 finders → adversarial verify → synth. **247 confirmed findings** → `docs/audit-2026-06-04/` (durable worklist at `edit-worklist.json`).
- **Edit swarm** (`scripts/edit-swarm.workflow.js`): per-finding read-only patch propose → adversarial diff-verify; **never writes** (caller-side `scripts/edit-apply.py`, exact-match + uniqueness, `--apply` to write, dry by default). Baseline `e1a616f` committed → `git checkout <file>` = safe per-file revert.
- **Hybrid by design:** swarm only FE mechanical. Applier normalizes paths before grouping. `docs/IDEAS.md` = concept notes. Other scripts: audit-watch/assemble/split, edit-batch/watch.

---

## Session 2026-06-04 (cont. 33) — ACTIVITY PANEL REORG + SESSION DIFF (UNSHIPPED v0.4.46)

Frontend-only, 4081/0/0, CDP-verified. **Activity panel** Now/Steps double-spin killed (invariant in DON'T-TOUCH). **Session Diff** (`SessionDiff.svelte`): full-pane per-file review reusing `EditDiff` (`hideHead`), reads `tab.messages`, 5 live-verified entry points incl. revived `MessageBubble.reviewDiff` (deep-links `turnStats.firstEditFile`). Preview toggle added then fully removed (grep-clean). 🟡 flags: done-cap counts writes (shows "7 steps" vs badge 5); `classifyTool`+SessionDiff don't strip `mcp__rift__` prefix; scratch `.tmp/activity-test.md` deletable. Detail: `git log`.

## cont. 32 — SHELL NAV REDESIGN (UNSHIPPED v0.4.46)
Titlebar nav: left activity column removed → Home/Chat `.navitem`s + Settings gear in `Titlebar.svelte` (Ctrl+1/2); chat-rail toggle → `.rail-toggle` in `ChatTabsBar` (`uiPrefs.toggleChatRail()`); model/slash/mention menus → opaque (`app.css .rift-menu`); dock-resize z 2→6. **Orphans (cleanup):** `ActivityBar.svelte` unmounted (kept per safety) + dead `railPinned`/`applyRail`/`--rail-w`/`RAIL_PINNED_KEY` in `ui-prefs.svelte.ts`. Detail: `git log`.

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
- **IA: 3 workspaces** — home·1 chat·2 settings·3. Nav lives in the **titlebar** now (cont.32, no left activity column): Home/Chat `.navitem`s + Settings gear in `Titlebar.svelte`; switching still via `workspace.setActive`/Ctrl+1-3. Settings = one scroll-doc, **5 sections** (Appearance landing · Accessibility · Assistant · Speech · About). **Left chat rail retired (cont.35)** — no `ChatRail`/`.rail-toggle`; chat history lives ONLY in the History drawer (View-menu → `HistoryDrawer`).
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split (cont.33):** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → opens Session Diff. Live units render ONLY in the Now cluster (don't re-add pending/writes to Steps). `SessionDiff.svelte` reads `tab.messages` (real) via `EditDiff` `hideHead`; open via `assistant.ui.diffOpen/diffTarget`. `MessageBubble.reviewDiff` deep-links by `firstEditFile` basename — don't repoint at `actnode-*` (removed).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. v0.4.46 stands.
