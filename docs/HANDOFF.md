# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-04 (cont. 51) — SHIPPED v0.5.0 (Velopack)

Bundled the cont.46b–50 unshipped work into a release. Bumped 3 lockstep files + `Cargo.lock` → `0.5.0`; CHANGELOG v0.5.0 entry (Harness + Steer + cleanup), trimmed v0.4.46 to cap. `cargo check` 0/0 · `npm run check` 4061 0/0. Commit `62dae27` pushed to `origin/main`. `release.ps1` (Velopack) built + packed + published **stable** v0.5.0 to `Blazzer10200/rift-releases` (Setup.exe + Portable.zip + full/delta nupkg + releases.win.json; `prerelease:false`). Release notes auto-pulled from CHANGELOG top. No Rift running at build → no file-lock.

### RESUME HERE (cont.51)
- v0.5.0 is the **second auto-update proof point** after v0.4.48: an installed v0.4.47/0.4.48 client should now auto-detect v0.5.0 → download → apply-on-exit → relaunch (design §6 R1). Awaits USER live-confirm on a real machine.
- Steer: still want a *visible* mid-turn redirect on a multi-step tool turn via the UI (ISSUES → Active).

---

## Session 2026-06-04 (cont. 50) — Steer feature + dead-code/dead-file cleanup + docs destale (shipped in v0.5.0)

3 commits on `main`, no version bump: checkpoint `ad4694d`, cleanup `371d885`, docs `6147301`. cargo 0/0, `npm run check` 4061 0/0.

- **STEER (new, verified):** type during a streaming turn, **Alt+Enter** injects it into the live CLI stdin → agent course-corrects at its next step (no restart). Backend: `assistant_steer` cmd + `STEER_TX` registry (`Mutex<Option<HashMap>>`, mirrors SESSION_PIDS) + `tokio::select!` reader + shared `build_user_envelope` (mod.rs/lib.rs). Frontend: `assistant.steer()` + Alt+Enter (Shift+Enter stays newline) + toast. Proven: standalone CLI probe (tool-loop pivots) + live CDP (`steer=steered`). Brief `docs/design/steer-and-queue.md`. **Nuance:** visible redirect needs tool-step boundaries; pure-text turns finish first (by design).
- **DEAD CODE:** dropped unused Rust deps `dashmap`+`notify` (machete+grep verified — clears cont.48's `notify` carry-over). Removed 8 unreferenced frontend files (FlashToast, dialogs/Confirm, shell/{ActivityBar,EmptyState,PageToolbar}, browser-tabs, utils/{file-display,time}). ts-prune candidates all `.svelte` `use:` false positives.
- **DOCS destale:** README (Velopack self-update), DEVELOPING (full rewrite — sync/SSH/FXServer ledger/russh gone), ISSUES (removed-subsystem blocks pruned; #20/#21 rewritten), SECURITY (russh/rsa advisory removed), IDEAS (path fix).

### RESUME HERE (cont.50)
- **Harness WIP (cont.46b+48+49) is now COMMITTED** inside checkpoint `ad4694d` (was uncommitted). Restore-as-uncommitted: `git reset --soft ad4694d^`. Ship path unchanged: version lockstep (3 files+`Cargo.lock`) → CHANGELOG → `/git-ship`.
- **Steer:** live-verify a *visible* mid-turn redirect on a multi-step tool turn through the UI (ISSUES → Active).

---

## Prior arcs — detail in `git log` (all shipped in v0.5.0 unless noted)
- **48–49 Harness telemetry:** per-session logging (`session_log.rs` → `~/.rift/assistant/session-logs/<id>.json`, prune 40; `sessionLog.ts` IPC); HarnessPage Live/past/empty `source`; fixed 2 Live accuracy bugs (memoized-fold alias, stale `doneAt`).
- **46b Harness REBUILT:** accuracy-first bento dashboard; cumulative folds from session-wide `assistant.telemetry`; themeable `--accent-h`; Ctrl+3.
- **Open carry-over:** `check.yml` per-push email spam; prod app is now ALSO `rift-tauri.exe` → revisit "never blanket-kill rift" rule.
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
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. **v0.5.0 stands** (shipped 2026-06-04, cont.51).
