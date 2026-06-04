# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-04 (cont. 38) — HAND-FIXED THE 21 FLAGGED FE FINDINGS (FRONTEND AUDIT DONE)

Worked the cont.37 flagged list manually (no swarm — these needed judgment). `npm run check` 4080/0/0 at every commit; `edit-done.json` 88→104. **Frontend audit is now fully resolved.** Commits:
- `0657e8c` **state:** F140 (StreamEnvelope `usage` type, drop unsafe cast), F190/F191 (branch-load race guard + fail-loud via `lastError`), F204 (telemetry single-pass per-model timings).
- `cd2a9b4` **toast+greeting:** F44/F224 (toast pause/resume preserves remaining time via deadline tracking), F182/F189 (greeting ticks hourly via reactive `nowHour` clock — `$derived` alone was insufficient).
- `5d16612` **markdown:** F165 (latch `everStreamed` in `$effect` not the parsed `$derived`; `revealActive` accepts live `streaming` → no 1-frame gap).
- `d3379a2` **a11y:** F150 (arrow-key/Home/End nav for proj+view `role=menu`), F135 (`listbox`→`menu` + `menuitem`/`menuitemradio`/`menuitemcheckbox` across slash/mention/perm/settings menus).
- `50051d9` **perf:** F166 (adaptive ActivityPanel ticker — 1s streaming / 10s idle).
- **Verified already-fixed** (recorded, grep-confirmed): F24 (6 mutators), F146, F147, F226.

### Genuinely remaining (NOT done — by deliberate call)
- **F209** — wontfix-by-design: the toast `icon: any` is deliberate + documented (precise typing fights lucide-svelte defs).
- **F230** — moot: `EmptyState.svelte` is **dead** (zero imports/renders in `src`). 🟡 Flag: unused component — delete-or-keep is user's call (not deleted per flag-not-delete).
- **F33** — DEFER w/ rationale: per-tick DOM reparse is on the blur-reveal animation that has a CRITICAL DON'T-TOUCH invariant; cost is transient (streaming only); a wrong move breaks the cascade. Needs careful design + live CDP visual verify.
- **F187** — DEFER w/ rationale: ActivityBar pointer-capture edge only bites if `setPointerCapture` throws AND user drags fast past a button; the robust fix (window-listener drag model) is a real refactor with regression risk on a working drag-reorder. Low value vs risk.
- **F48 / F51 → RUST PHASE** (below): both need a Rust-side change. F48 = remove the dead `assistant_set/get_thinking_effort`/`permission_mode` commands (frontend uses localStorage; no observable live bug). F51 = persist `forceNextFirstTurn` — needs a field on the `Conversation` struct (`mod.rs:388`; `created_at` is non-optional `i64`, no extra-field capture). ⚠️ Also noticed `compactionHistory` doesn't round-trip through that struct either (latent).

### RESUME HERE — PHASE 2: clear the remaining 140 (backend + held frontend)
Frontend **swarmable/mechanical** work is done, but the audit has **140 findings still open** (worklist 244 − done 104). ⚠️ "frontend done" earlier was scoped to the swarm buckets — 25 non-Rust findings were HELD (security/dead-code/deps) or missed, incl. genuine bugs (**F34/F35** undeclared `opener` → runtime ReferenceError; **F39** pane each-block keyed by index).

**→ Full grouped worklist: `docs/audit-2026-06-04/PHASE2-backend.md`** (regenerate anytime: `python scripts/phase2-list.py`). Breakdown:
- security (rust) 30 · security (frontend) 18 · frontend held/missed 7 · rust 74 · deps 2 · dead-code 9.
- Notable: `git_local.rs:72` traversal (F13/F50), `commands/mod.rs:27` `cmd /C code` injection (F19), `browser` file://+javascript: (F17/F18), `stt/cleanup.rs` bypassPermissions (F7/F79), API-key plaintext fallback (F49). **F48/F51** confirmed Rust-side (dead commands; `Conversation` struct field at `mod.rs:388`).

**Procedure (in PHASE2 doc):** quit dev targeting **`rift-tauri.exe` EXACTLY** (never a `rift*` glob — `rift.exe` is prod) → `cargo check --manifest-path src-tauri/Cargo.toml` baseline → fix in file-clusters → re-`cargo check` + `npm run check` → scoped commit → append IDs to `.tmp/edit-done.json` → re-run `phase2-list.py`. Audit baseline is STALE — verify before editing; many will be already-fixed (record those as done too). Deferred-with-rationale (don't silently drop): **F33** (blur-reveal perf, invariant-protected), **F187** (drag pointer-capture edge), **F209** (toast icon `any`, by-design), **F230** (EmptyState dead component).

Tracking: `.tmp/edit-done.json` (104 IDs).

## Session 2026-06-04 (cont. 37) — EDIT-SWARM 2nd PASS: 47 REMAINING FE FINDINGS CLEARED

2nd swarm pass over all 3 buckets (state-ts 14 · comp-assistant 17 · comp-other 16 = 47). All 47 now **resolved or flagged**. `npm run check` 4080/0/0 at every commit. `edit-done.json` 62→88.
- **10 applied + committed:** `90ff50f` state-ts (F172 stt `destroy()`, F180 restoreTabs single-load, F215/F217 everOpened) · `80d24f1` comp-assistant (F30 no openTab on right-click, F134 effort-slider keyboard-reachable) · `ad04775` comp-other (F164 progressbar aria-label, F232 Confirm dontAsk reset, F198 Select aria-disabled+CSS) · `092f07f` hand-done F173 (stt per-channel listen try/catch).
- **12 verified already-fixed** (recorded as done, grep-confirmed): F20,F27,F29,F155,F184,F223,F228,F148,F174,F206,F207,F209 — fixed in 1st pass / coincidentally; swarm correctly deferred.
- **4 resolved-by-deletion:** F167–F170 target `ChatRail.svelte` (deleted cont.35).
- **21 FLAGGED judgment calls** → all worked in cont.38 above (12 fixed, 4 already-fixed, F209 wontfix, F230 moot, F33/F187 deferred, F48/F51 → Rust). Re-emit any bucket: `python scripts/edit-batch.py <bucket>`.

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
