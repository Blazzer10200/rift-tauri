# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-04 (cont. 39) — PHASE 2 COMPLETE: ALL 244 AUDIT FINDINGS DISPOSITIONED

Cleared the remaining 140. `edit-done.json` 104→**244** (`phase2-list.py` = 0 remaining). cargo check clean, 14 cargo tests pass (incl. new absolute-`..` traversal test), npm 4067/0/0. **8 commits** `8a21c25`→`f3ba880`. **Full per-finding breakdown: `docs/audit-2026-06-04/PHASE2-dispositions.md`.**
- **FIXED security (hand):** git traversal F13/F50 (+test) · browser scheme allowlist F17/F18 · cmd-metachar F19 · stt empty `--allowed-tools` F7/F79/F113 + kill-on-timeout F16/F116 · update repo-pin F238 · Markdown style-attr DOMPurify F32 · secrets/roots F128/F59 · UpdateDialog F47/F175 · pane-keying F39 · data:-scheme F199.
- **FIXED panics/concur:** grep UTF-8+single-open+cap F11/F10/F80/F81 · fail-closed roots F240/F244 · stderr byte-index F70 · drain-EOF deadlock F4 · task-leak F6 · summarize panic F3/F5/F66/F68. **Other:** dead-cmd rip F48 · `#[serde(flatten)]` persists forceNextFirstTurn+compactionHistory F51 · vitest 2→4.1.8 F237 · settings F159/F160. **Already-fixed:** F34/F35, F46, F245.

### RESUME HERE — audit list CLEAN; new feature work unblocked
Low items needing **your decision** (recorded done; detail in dispositions doc):
- **FLAG dead-code/deps** (flag-not-delete, grep-confirmed dead, removal recommended): DiagStage variants, secrets fns, SAFE_MCP entries, stale docs, dead CSS/fields — **plus `russh`/`russh-cryptovec` UNUSED in Cargo.toml carrying RUSTSEC-0153/0154** (removal touches transitive rand/rustls → careful `cargo build`).
- **DEFER STT** (whisper-rs 0.13→0.16 blocked): 21 low items in inactive code.
- **DEFER prior:** F33 (blur-reveal, invariant), F187 (drag), F230 (EmptyState dead). **ACCEPT** ~36 (by-design / local-app threat model): silent-swallow / blocking-io / a11y.

## Audit arc (cont. 32–38) — UNSHIPPED v0.4.46 · detail in `git log`

- **38/37/36:** full 247-finding audit cleared on the frontend — edit-swarm 1st/2nd pass (64+47 fixes) + 21 hand-worked flagged judgment calls. `edit-done.json` →104. All gated 4080/0/0. Swarm infra: `scripts/edit-swarm.workflow.js` (propose-only) + `edit-apply.py` (atomic, scoped `git add` — never `-A`).
- **35:** Atmos backdrop (`AssistantPane .atmos-glow`); left chat-rail RETIRED (`ChatRail.svelte` deleted; history only in `HistoryDrawer`).
- **34/33/32:** audit-swarm built → `docs/audit-2026-06-04/` (247 findings, read `README.md`); Activity Now/Steps split + `SessionDiff.svelte`; titlebar nav.

## Earlier arc (cont. 13–31) — detail in `git log`
- **31–27:** logo platform icons; theming `--bg-inset`/`--field`/`--track`; body hue 270→250; tint mixes **oklab** not oklch; remote-shell rip; Settings IA 6→5; CLI-update detector. ⚠️ **CSP** `connect-src` keeps `https://registry.npmjs.org`.
- **20/21 PURE-ASSISTANT:** SFTP/sync/server/RCON ripped; MCP→`read_file/list_dir/grep`+`git_*`; IA=3 workspaces.
- **Entire cont.13–39 UNSHIPPED on v0.4.46 — ship as ONE commit.** CDP: `bash scripts/cdp/c.sh {shot|eval|click}` (dev via `run-dev.bat`+`npm run cdp:serve`). Nav Ctrl+digit; Alt+digit tabs.

## CRITICAL DON'T-TOUCH
- **Onboarding gate:** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && !assistant.hasApiKey && !assistant.auth?.loggedIn`. `configLoaded` gates timing so it never flashes pre-probe.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Components use `var(--accent)`/`--accent-soft`. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (cont.30 — oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 3 workspaces** — home·1 chat·2 settings·3. Nav lives in the **titlebar** now (cont.32, no left activity column): Home/Chat `.navitem`s + Settings gear in `Titlebar.svelte`; switching still via `workspace.setActive`/Ctrl+1-3. Settings = one scroll-doc, **5 sections** (Appearance landing · Accessibility · Assistant · Speech · About). **Left chat rail retired (cont.35)** — no `ChatRail`/`.rail-toggle`; chat history lives ONLY in the History drawer (View-menu → `HistoryDrawer`).
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split (cont.33):** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → opens Session Diff. Live units render ONLY in the Now cluster (don't re-add pending/writes to Steps). `SessionDiff.svelte` reads `tab.messages` (real) via `EditDiff` `hideHead`; open via `assistant.ui.diffOpen/diffTarget`. `MessageBubble.reviewDiff` deep-links by `firstEditFile` basename — don't repoint at `actnode-*` (removed).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. v0.4.46 stands.
