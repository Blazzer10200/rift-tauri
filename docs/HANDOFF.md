# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-12 (cont.118) — Buddy-release campaign PLANNED (no code changes)

User is releasing Rift to friends. Chose **minimal core** scope + ASAP timeline, delegated all cut decisions. This session = research + planning only; working tree has zero code edits.

- **Execution brief written: `docs/design/minimal-core-strip.md`** — locked decisions, slice order (S3→S1→S4→S5→S2→S6→S7), exact files/lines/symbols, verification gates, watchpoints. Read it FIRST next session; it is the whole Phase 2 spec.
- **CUT:** Harness workspace (+ Cost/Swarm sub-tabs) · swarm backend · cost cockpit except plan-limit gauges (`limits.rs` verified independent of UsageDb/store) · session-log subsystem (Harness-only consumers) · **compaction — closes #33 by removal, not repair** · custom providers + compression proxy (first-party only).
- **KEEP:** speech/dictation, enhance wand, browser dock (`open_browser` depends), API-key override, per-turn cost cap, in-memory `telemetry.turns` (ActivityPanel + healthAlerts).
- **FIX (Phase 3):** #34 SessionDiff pile-up (sketch in ISSUES).
- Campaign tasks #1-#5 in Tasks panel: #1 inventory ✅ · #2 strip (pending, brief-driven) · #3 fix #34 · #4 CDP walkthrough + fresh-machine pass · #5 ship + distribute.

### RESUME HERE

1. **Execute `docs/design/minimal-core-strip.md`** slice by slice; green gates between (svelte-check + vitest / cargo check + test). Line numbers are 2026-06-12-fresh.
2. Then Phase 3 (#34 fix) → Phase 4 walkthrough — folds in the carried live-verify list: v0.8.26 composer slim + cwd badge + #29 CSP prod verify + tool-chip hover + trust gating + v0.8.25 dictation items + permission-bar on a trust-standard throwaway repo + Auth-Rec (buddies' logged-out machines close it).
3. Phase 5 ship: tag-driven CI → install from real feed → smoke → distribute (unsigned — buddies get SmartScreen "More info → Run anyway" note).
4. Fable sunset sweep stays dated post-Jun 22 (ships to buddies via auto-update).

## Prior arcs — detail in `git log` + CHANGELOG

cont.117 composer slim + issue sweep + CR-UX trust 2-level → v0.8.26 (CI verified green). cont.116 dictation data-fence → v0.8.25. cont.115 enhance wand v2 + PTT → v0.8.24. cont.113 Activity polish → v0.8.23. cont.111 full-codebase audit → v0.8.22. cont.109 bridge.rs loopback v0.8.21. cont.108 live plan limits v0.8.20. cont.104 Rail-v2 + registry race fix. cont.94 Fable 5 (**Jun 22 sunset gate**). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Live TabState is authoritative over disk** — never re-add `stop()` to `loadConversation` or disk-reload a tab in `host.tabs` (cont.110; regression tests guard).
- **Trust enum is now 2-level** (cont.117) — `full` must stay rejected for new writes but MIGRATE read-side (config + `RIFT_TRUST_LEVEL` env); don't "clean up" the migration arms.
- **Onboarding gate (cont.55)** · **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest) · **Right-click ownership** (`preventDefault()` or global double-fires).
- **Accent via `--accent-h`**; tint mixes `in oklab`, never `in oklch`. **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175. **Spine-node icons stay opaque**.
- **IA: 3 workspaces** (Home · Chat · Settings — Harness cut in S3), nav in titlebar. **AssistantPane drop handlers on `.pane` outer only**. **Blur-reveal:** `shownCount` only `$state` via rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.26 stands.**
- **`turn.rs::kill_all_session_children` re-export** (sweeps `oneshot::ENHANCE_PIDS`) + **bridge env injection in `write_mcp_config`** — load-bearing.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
- **Strip-brief keepers:** `cleanup_retired_jsonls`, `environment_check`, `usage::limits::usage_rate_limits` — slices delete their neighbors, not these.
