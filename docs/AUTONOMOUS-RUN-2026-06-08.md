# Autonomous run plan — 2026-06-08 (user away ~2h)

Self-contained task list for an unsupervised session. User granted control of the **dev server + CDP + tools** and said: *"make this super fat so it gets done correctly."* Read top-to-bottom, execute by phase. Guardrails at the bottom override everything.

## ⚠️ CONTEXT RULES WAIVED FOR THIS RUN
The user explicitly waived the normal context-management discipline for this session. **Do NOT wrap/handoff/`/clear` at 150–200K or 250–300K.** Keep working straight through — depth is expected, the whole point is to grind the list to completion. Ignore "cache fence" / "felt-latency" wrap triggers. Only stop for the real stop-conditions in Guardrails. Run long.

## State at handoff
- Branch `main`. **Heavy uncommitted WIP** in tree: cont.78 Settings Bento (`SettingsPage.svelte` + `static/_mock_settings_*.html`), per-workspace model WIP (`assistant.svelte.ts`+`assistant/helpers.ts`+`assistant/workspace.ts`), cont.80 backend tests (9 Rust files: `permission.rs`/`session_log.rs`/`secrets.rs` + `usage/{mod,store,aggregate,insights}.rs` + `mod.rs`/`git_local.rs`), plus staged doc archive moves.
- **DO NOT commit, bump, changelog, or `git add` anything. DO NOT sweep/stash the WIP.** This run is verify + audit + review + plan only — it produces findings, screenshots, review reports, and a HANDOFF extension, nothing shippable.
- Dev binary not running at handoff. Baseline (per cont.80): `cargo test --lib` 95/0, `cargo clippy --all-targets` clean, `vitest` 51/51, `svelte-check` 0/0.

---

## PHASE 0 — Static baseline (BEFORE booting dev)
Run these while no dev server is alive — `cargo` collides with `tauri dev`'s file lock, so this MUST go first.

1. **Rust tests + clippy:** `cargo test --lib --manifest-path src-tauri/Cargo.toml` → expect 95/0. `cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml` → expect clean. Log any drift verbatim (the WIP backend files could regress).
2. **Frontend:** `npm run check` → expect svelte-check 0/0. `npx vitest run` → expect 51/51. Log verbatim.
3. **Review the uncommitted WIP diff** (high value — lets the user commit confidently on return). Run `/code-review` (or `/quick-review`) over the working tree. Three logical buckets, review each:
   - cont.80 backend tests (additive, should be clean — confirm no behavior change leaked into the 6 extracted `aggregate.rs` helpers).
   - cont.78 `SettingsPage.svelte` Bento redesign.
   - per-workspace model WIP (`assistant.svelte.ts`/`helpers.ts`/`workspace.ts`).
   - Output: findings into HANDOFF scratch. **Do NOT auto-fix** unless a finding is a clear 1-3 line correctness bug AND in already-WIP files — even then, note it loudly.
4. **`/memory-check`** — validate memory files against current codebase (post pure-assistant rip + cont.78/79/80 churn may have staled refs). Report stale entries; don't auto-edit memory.
5. **`/self-check`** — audit `~/.claude/` config (hooks, skills, JSON). Report only.

---

## PHASE 1 — Boot dev + CDP
1. `scripts/run-dev.bat` → **background** (`run_in_background:true`). Sets `--remote-debugging-port=9222`. **Capture the exact PIDs spawned** — needed for safe shutdown.
2. `npm run cdp:serve` → **background**. Wait for `127.0.0.1:9223`.
3. `bash scripts/cdp/c.sh health` → green before driving.
4. Let first paint settle → `bash scripts/cdp/c.sh state` to confirm mount + read current workspace/model.

---

## PHASE 2 — Live-verify the blocked 🧪 items

### T1 — Permission bar  [ISSUES: Permission, 🧪→resolve]
Wired end-to-end (`--permission-prompt-tool stdio` → `can_use_tool` → `PermissionBar.svelte`), never runtime-confirmed.
- Create a **throwaway git repo under `%TEMP%`** (never inside the project). Seed it with a file + initial commit.
- Point a chat workspace at it. Drive a backend turn (CDP) that triggers a **git-write op** (ask the assistant to commit/modify) in default/acceptEdits mode.
- Confirm Allow/Deny surfaces. Test BOTH paths: Allow → op proceeds; new turn → Deny → op blocked. **Screenshot each state.**
- Record verdict in HANDOFF; flip ISSUES status if confirmed.

### T2 — Steer  [ISSUES: Steer, 🧪→resolve]
Confirmed on text turns; needs a **visible** redirect on a *multi-step tool* turn.
- Drive a turn doing several tool calls ("list dir, then grep X, then read 3 files, then summarize").
- Mid-stream, Alt+Enter a steer ("stop — instead do Y").
- Confirm redirect lands **visibly mid-turn**, not after completion. Screenshot the moment. Record verdict.

### T3 — Auth-Rec (best-effort only)  [ISSUES: Auth-Rec, ✅ in-tree]
The 401 sign-in recovery banner. **The dev box stays authed**, so the live login spawn can't truly fire here. Don't burn time forcing it — just confirm the banner *renders* its three states (CDP can toggle the auth gate) and note the spawn path remains compile-only-verified. Skip if it fights back.

---

## PHASE 3 — Full-app CDP audit  [ISSUES #4 + UI-drift]
Last full pass was cont.58 (2026-06-05) — **before** the cont.78 Settings Bento redesign + cont.79 updater-UI changes. Be thorough; this is the meat.

### T4 — Workspace walk + screenshots
Screenshot every surface, eyes on pixels (not just DOM): Home · Chat · Harness (all 3 sub-tabs: Telemetry/Cost/Swarm) · Settings (all 5 pill-tabs: Appearance/Accessibility/Assistant/Speech/About) · command palette · History drawer · Web-browser panel · Panels menu.
- **cont.78 RESUME HERE:** Accessibility tab is a lone `sb-s8` block — **looks sparse**. Screenshot; propose widen or add a second block. Implement ONLY if trivial + clearly right (it's already-WIP `SettingsPage.svelte`); else leave a sharp proposal.
- Verify the cont.78 bento invariants hold live: surface tiers (no near-black wells), themeable accent (no oklch purple-wrap on warm hues), pill-tab switching, hero status banner green/amber.

### T5 — Responsive / layout regression (Settings Bento)
The Bento has breakpoints (`<1180px` 12-col, `<940px` → 1-col). Drive CDP viewport resizes across ~1400/1180/940/760px and screenshot Settings + Harness at each. Catch collapse bugs (cont.78 already hit one CSS-specificity collapse trap — confirm it stayed fixed). Flag any overflow/wrap/collision.

### T6 — UI-drift #100 (update surfaces)
Check the toast (`updates.svelte.ts`) vs Home/Settings status card read the **same** update state — the known bug is one saying "available" while another says "up to date." Screenshot side-by-side if reproducible. This is the cosmetic remainder after the v0.6.2 functional fix.

### T7 — Accessibility audit (real a11y, via CDP eval)
- Tab-order / focus-visible: drive Tab through each workspace, confirm focus ring follows logically, no traps.
- Check key controls have accessible names (aria-label / text) via `eval` over the DOM.
- Contrast spot-checks on the new Bento surfaces.
- Output findings as ISSUES #4 candidates (a11y sub-list).

### T8 — Stress pass (repeat cont.58 methodology)
- ~12 rapid workspace switches (Ctrl+1-4) — confirm no leak/error.
- Large hostile paste into composer (~15K chars w/ emoji/unicode/`<script>`) — confirm auto-grow caps, inert, no XSS, no console error.
- A real **read-only** backend turn end-to-end (CLI spawn → MCP `grep`/`glob`/`list_dir`/`read_file` → stream → cost/context/activity render). Confirm all panels populate.
- **Watch the console ring the ENTIRE phase** — log every error/warning verbatim with the action that triggered it. (cont.58 was 0/0; regressions since cont.78/79 are the thing to catch.)

---

## PHASE 4 — Updater verification

### T9 — Updater e2e log trace  [HANDOFF cont.79 RESUME HERE]
Read-only. Full in-app download→apply→relaunch has **never** been confirmed firing (0 events ever).
- Read `%LOCALAPPDATA%\com.blazzer.rift\logs\rift.log` directly.
- Trace: `download_update: command invoked` → `update download: starting` → `apply: scheduling swap` → child-reap/`app.exit(0)`. Report present/absent → bisects where it dies (or confirms success if the user tested v0.8.7→v0.8.8 toast→Download).
- Also confirm the v0.8.7 toast z-index fix (60→2000) — code-verified, never runtime-watched. If a toast can be coaxed in dev, screenshot it above overlays; if not, note it stays unwatched.

---

## PHASE 5 — Planning (no execution)

### T10 — assistant.svelte.ts M8/M9 split plan  [ISSUES #20]
M8 (streaming pump) + M9 (send orchestrator) — highest-blast-radius extractions, now backed by `assistant.playback.test.ts` (51 vitest). Brief: `docs/design/assistant-svelte-split.md`.
- **Do NOT execute** (high-risk + tree already has `assistant.svelte.ts` WIP). Re-read brief vs current code → produce a **sharpened, line-anchored M8 extraction plan** the next live session can transcribe verbatim. Same for M9 if time.

### T11 — #4 per-control wiring checklist (Settings)
The #4 sweep's stated approach: per-control checklist (control → wired? necessary? consistent?). Walk every Settings control via the source + live app; produce the checklist table. Flags dead/orphan controls. Planning artifact only.

---

## PHASE 6 — Reporting (end of run)
- Extend `docs/HANDOFF.md` (**cont.81**) with: per-task verdicts (T1–T11), screenshot paths, new findings, console-error log, test/lint baseline results, WIP review summary.
- Write new findings as **ISSUES.md candidates in a HANDOFF scratch section** — do NOT renumber or edit ISSUES.md directly (durable IDs; that's a reviewed action).
- Append a terse `Daily/2026-06-08.md` session log entry (per workspace CLAUDE.md).
- Shutdown: either leave dev running OR PID-kill cleanly (path-checked) — **note which** in HANDOFF.

---

## GUARDRAILS (override all of the above)
- **No commits / no version bumps / no CHANGELOG / no `npm run build` / no `npm run tauri build` / no release.** Verify-review-plan-report only.
- **Don't sweep, stash, or `git add` the existing WIP.** Auto-fix only clear 1-3 line correctness bugs inside already-modified files, and flag loudly.
- **Kill dev by PID ONLY**, after path-checking the PID resolves under `src-tauri/target/` (dev). NEVER under `%LOCALAPPDATA%` (prod). **Never `taskkill /IM rift-tauri.exe`** — name collision kills the user's live prod app. Unsure → leave it running.
- **Don't `cargo check`/`cargo test` while `tauri dev` is alive** (lock collision kills dev). All Rust testing is Phase 0, before boot. During dev, the tauri console IS the Rust verifier.
- Frontend trivial fix (if any) → `npm run check` after.
- Throwaway repos (T1) under `%TEMP%`, never in the project tree. Clean up after.
- **Stop + leave a note if:** dev won't boot, CDP unreachable after 2 tries, a backend turn can't auth, or any task needs a decision not pre-authorized here. Don't guess on irreversible things.
- Context rules waived (see top) — run long, don't self-truncate.
