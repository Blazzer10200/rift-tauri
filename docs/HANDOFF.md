# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-04 (cont. 50) — Steer feature + dead-code/dead-file cleanup + docs destale (UNSHIPPED)

3 commits on `main`, no version bump: checkpoint `ad4694d`, cleanup `371d885`, docs `6147301`. cargo 0/0, `npm run check` 4061 0/0.

- **STEER (new, verified):** type during a streaming turn, **Alt+Enter** injects it into the live CLI stdin → agent course-corrects at its next step (no restart). Backend: `assistant_steer` cmd + `STEER_TX` registry (`Mutex<Option<HashMap>>`, mirrors SESSION_PIDS) + `tokio::select!` reader + shared `build_user_envelope` (mod.rs/lib.rs). Frontend: `assistant.steer()` + Alt+Enter (Shift+Enter stays newline) + toast. Proven: standalone CLI probe (tool-loop pivots) + live CDP (`steer=steered`). Brief `docs/design/steer-and-queue.md`. **Nuance:** visible redirect needs tool-step boundaries; pure-text turns finish first (by design).
- **DEAD CODE:** dropped unused Rust deps `dashmap`+`notify` (machete+grep verified — clears cont.48's `notify` carry-over). Removed 8 unreferenced frontend files (FlashToast, dialogs/Confirm, shell/{ActivityBar,EmptyState,PageToolbar}, browser-tabs, utils/{file-display,time}). ts-prune candidates all `.svelte` `use:` false positives.
- **DOCS destale:** README (Velopack self-update), DEVELOPING (full rewrite — sync/SSH/FXServer ledger/russh gone), ISSUES (removed-subsystem blocks pruned; #20/#21 rewritten), SECURITY (russh/rsa advisory removed), IDEAS (path fix).

### RESUME HERE (cont.50)
- **Harness WIP (cont.46b+48+49) is now COMMITTED** inside checkpoint `ad4694d` (was uncommitted). Restore-as-uncommitted: `git reset --soft ad4694d^`. Ship path unchanged: version lockstep (3 files+`Cargo.lock`) → CHANGELOG → `/git-ship`.
- **Steer:** live-verify a *visible* mid-turn redirect on a multi-step tool turn through the UI (ISSUES → Active).

---

## Session 2026-06-04 (cont. 49) — Harness: real-data verified + 2 accuracy bugs fixed (UNSHIPPED)

Stress-tested HarnessPage live telemetry vs on-disk JSON (ground truth) via CDP — cont.48's real-data check **DONE**, every cumulative cell matches. Fixed 2 real accuracy bugs + polish. `npm run check` **0/0/0**.

- Fixed 2 Live accuracy bugs (detail in git): memoized folds never recomputed (`$derived(source.turns)` returned a mutated-in-place ref → `$derived([...source.turns])`); last-turn `doneAt` stale (`void assistant.streaming` added to `liveSnap` trigger). Polish: hero glow centered, `.nodata` dimmed dashes, 1s uptime tick. (Now committed in `ad4694d` — see cont.50 RESUME.)

---

## Session 2026-06-04 (cont. 48) — Harness: multi-session telemetry logging (UNSHIPPED)

Persistent per-session logging end-to-end (`assistant/session_log.rs` save/list/load/delete/prune → `~/.rift/assistant/session-logs/<id>.json`; `sessionLog.ts` IPC; debounced from `handleTurnComplete`, prune 40). HarnessPage reads a Live/past/empty `source` snapshot. Detail in git.
- **Open carry-over:** Velopack auto-apply e2e proof awaits USER (design §6 R1); `check.yml` per-push email spam; prod app is now ALSO `rift-tauri.exe` → revisit "never blanket-kill rift" rule. (`notify` RUSTSEC removal DONE in cont.50.)

---

## Session 2026-06-04 (cont. 47) — CI green: russh dead-dep removal (SHIPPED `da159fc`)

Removed dead russh cluster (russh/russh-sftp/async-trait/rand/base64/sha1; kept `sha2`=STT) → cleared `cargo audit` RUSTSEC-2026-0154 CI spam. `Cargo.lock` −~1240 lines; checks exit 0; no version bump.

---

## Prior arcs — detail in `git log`
- **46b Harness REBUILT:** accuracy-first bento dashboard; cumulative folds from session-wide `assistant.telemetry`; themeable `--accent-h`; Ctrl+3.
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
