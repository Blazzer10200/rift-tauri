# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-04 (cont. 49) — Harness: real-data verified + 2 accuracy bugs fixed (UNSHIPPED)

Stress-tested HarnessPage live telemetry vs on-disk JSON (ground truth) via CDP — cont.48's real-data check **DONE**, every cumulative cell matches. Fixed 2 real accuracy bugs + polish. `npm run check` **0/0/0**.

- **BUG 1 (big, pre-existing) — token flow / cache donut / turn timeline EMPTY in Live.** `srcTurns = $derived(source.turns)` returned telemetry's `turns` by reference; it's mutated in-place → ref `===` across snapshots → memoized folds (`tok`/`cacheEff`/`turnViz`) never recompute (only right if first eval was AFTER turns existed, which masked it). Fix: `$derived([...source.turns])`.
- **BUG 2 — last turn's `doneAt` Live metrics stale** (output t/s, avg turn). `su.turns` trigger bumps on `result` event; `doneAt` written later in stream-end. Fix: `void assistant.streaming` added to `liveSnap` trigger. Archived/persisted always correct.
- **Polish:** hero glow centered (`top:42%`, kept OUTSIDE `.gauge` — nesting a `blur()` in its stacking ctx → WebView2 black rect). Live uptime ticks via 1s `nowTick`. Empty `—%`→`—` + dimmed dashes via class **`.nodata`** (NOT `.dash` — collides w/ page-root container class → black box). Strip "archived"→"logged".

### RESUME HERE (cont.49)
- All Harness work UNSHIPPED, uncommitted (cont.46b+48+49 = one batch). Ship path: version lockstep (3 files + `Cargo.lock`) → CHANGELOG → `/git-ship`. Stress-test artifacts cleaned up.

---

## Session 2026-06-04 (cont. 48) — Harness: multi-session telemetry logging (UNSHIPPED)

**Persistent multi-session logging** end-to-end. (Detail in git.) Backend `assistant/session_log.rs`: save/list/load/delete/`prune(keep)` → `~/.rift/assistant/session-logs/<id>.json` (atomic), registered `mod.rs`+`lib.rs`. `SessionTelemetry` got stable uuid `id` (re-minted on `reset()`); `sessionLog.ts` IPC; `recordSessionLog()` debounced 1.5s from `handleTurnComplete`; prunes to 40. HarnessPage cumulative cells read a `source` snapshot (Live/past/`EMPTY_SNAP`); session strip; adaptive hero; "All" aggregate.

### RESUME HERE (cont.48)
- Real-data check **DONE in cont.49**; nav wiring (index.ts/workspace.svelte.ts/CommandPalette, Ctrl+1-4) verified working. Whole batch (cont.46b+48+49) ships together.
- **Carry-over:** Velopack auto-apply e2e proof awaits USER (design-doc §6 R1); `notify` crate RUSTSEC-0153 removal; `check.yml` per-push email spam; prod app is now ALSO `rift-tauri.exe` → revisit "never blanket-kill rift" rule.

---

## Session 2026-06-04 (cont. 47) — CI green: russh dead-dep removal (SHIPPED `da159fc`)

Removed dead russh cluster (russh/russh-sftp/async-trait/rand/base64/sha1; kept `sha2`=STT) → cleared `cargo audit` RUSTSEC-2026-0154 CI spam. `Cargo.lock` −~1240 lines; checks exit 0; no version bump.

---

## Session 2026-06-04 (cont. 46b) — Harness dashboard REBUILT (UNSHIPPED; ships w/ cont.48)

Accuracy-first bento dashboard; cumulative metrics fold from session-wide `assistant.telemetry` (hero ctx ring per-tab); themeable `--accent-h`; Ctrl+3. Detail in git.

## Prior arcs — detail in `git log`
- **44–45 Velopack auto-update:** `velopack::UpdateManager` over native `GithubSource`; shipped v0.4.47/0.4.48; vpk CLI=crate `=1.2.0`. Design: `docs/design/velopack-auto-update.md`.
- **40–43:** Composer/Improve-prompt; Claude-update automation; onboarding rebuild; History v2 (export).
- **20/21 PURE-ASSISTANT:** SFTP/sync/server/RCON ripped; MCP→read/list/grep+`git_*`.

## CRITICAL DON'T-TOUCH
- **Onboarding gate:** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && !assistant.hasApiKey && !assistant.auth?.loggedIn`. `configLoaded` gates timing so it never flashes pre-probe.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** (was 3 until cont.46) — home·1 chat·2 **harness·3** settings·4. Nav in the **titlebar** (Home/Chat/Harness `.navitem`s + Settings gear); switch via `workspace.setActive`/Ctrl+1-4 (positional via `workspace.order`, NOT the `kbd` field). Settings = one scroll-doc, 5 sections; Harness = single-page bento dashboard (NO sidebar). **Left chat rail retired** — history lives ONLY in the History drawer.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → opens Session Diff. `SessionDiff.svelte` reads `tab.messages` via `EditDiff` `hideHead`; open via `assistant.ui.diffOpen/diffTarget`.
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. v0.4.46 stands.
