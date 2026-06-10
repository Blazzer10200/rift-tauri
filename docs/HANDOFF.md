# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 98) — v0.8.16 SHIPPED: #20 backend split COMPLETE (R1-R8) + repo housekeeping

User said "knock out stuff, then release." Source-verified per commit (cargo check + clippy zero warnings · cargo test 95/95 · svelte-check 0/0 · vitest 51/51; no dev launch this session):

- **Housekeeping (`c283f24`):** deleted spent 2026-06-08 autonomous-run docs (M1/M2 confirmed fixed via wsKey) + `session-kickoffs.md` (arc complete); fixed every stale `docs/archive/` cross-ref (archive dir is gone — git log is history); ISSUES UI-drift block dropped (shipped 0.8.15); CLAUDE.md hot-files re-measured.
- **#20 backend DONE:** `35dd131` R2 `config` (569L) · `f201713` R6 `oneshot` (734L) · `782d3df` R8 `turn` (1372L — registry/steer/permission plumbing/`assistant_send`). **mod.rs 4331 → 303L hub.** Lesson 3 in the brief: glob re-export does NOT carry `pub(crate)`/`pub(super)` for external callers — `kill_all_session_children` needed explicit `pub(crate) use`. NOTE: R2 took two operator-agent bails (wrote config.rs, never deduped mod.rs) — finished inline w/ awk range-moves + compiler-as-checklist; that technique carried R6/R8 first-try.
- `83f3876` clippy auto-deref nit (mcp_server) — clippy fully clean again.
- **`composer-split.md` brief written** (C1-C7, blast-radius ascending; QueueRail before Rail-v2 lands small).
- **`770b7fc` release: v0.8.16** tagged + pushed → CI.

### RESUME HERE (cont.98)
- **CI CONFIRMED:** v0.8.16 release run 27245762669 green (3m0s); rift-releases latest = v0.8.16 (published 2026-06-10T01:00Z).
- **User prod = 0.8.12** → still needs ONE manual `Setup.exe`; after that, in-app update should pull 0.8.16.
- **Runtime smoke debt now TWO releases deep** (0.8.15 + 0.8.16 both source-verified only): next dev session run the CDP pass — real turn (stream/tools/thinking), steer, stop, /retry, queue drain, **prompt enhance + title gen + summarize/remint (oneshot moved!)**, Settings config get/set + provider CRUD (config moved!), History list/load/delete, auth pill, update chip.
- Next #20: Composer C1 (helpers.ts) per `composer-split.md`. Parked: SEC-1 live pass · #29 CSP-nonce · CR-UX trust-enum (needs user call) · `.tmp/runner/` setup-scripts fate (flagged to user).

## Prior arcs — detail in `git log` + CHANGELOG
cont.97 v0.8.15 (TS split complete M0-M9 · mod.rs 5/8 · honest update chip). cont.96 v0.8.14 update-dialog crash root-caused. cont.94 v0.8.13 Fable 5 limited-run (**Jun 22 sunset gate** — self-heals to Sonnet/Opus). cont.90 first tag-driven release on VM 100 `rift-runner`; **`RunnerKeepAlive` startup task load-bearing — DON'T delete.** PID-only kills, NEVER by image name.
[carried] `.slideover`/`.tip` blur (fix on new scuff only) · runner perf roadmap · drag-reorder verify · `RELEASES_TOKEN` re-set.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. Don't drop the betaNotice clause.
- **Accent themeable via `--accent-h`** (app.css `:root` only); tint mixes `in oklab`, never `in oklch`. Status LEDs fixed.
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces** (home·chat·harness·settings), nav in titlebar, positional `workspace.order`. Harness = one viewport, no scroll; diagnostics behind "Show details". Left chat rail retired.
- **AssistantPane drop handlers on `.pane` outer only**; `dragDropEnabled:false`; `.shell` fixed inset 0.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` only `$state`, written only by rAF loop.
- **Activity split:** Steps = settled actions (drops `cat==="write"`); Outputs owns writes → Session Diff.
- **Versions lockstep** ×3 + `Cargo.lock` — only at ship. **v0.8.16 stands** (2026-06-09 cont.98).
- **`turn.rs::kill_all_session_children` re-export** (`pub(crate) use turn::…` in assistant/mod.rs) is load-bearing for the Velopack apply — don't "clean up".
