# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-12 (cont.119) — Phase 2 minimal-core strip EXECUTED ✅

All 7 slices of `docs/design/minimal-core-strip.md` shipped as 7 commits on main (`470845b`…`bd1709c`), green gates between each. **Net −7,407 lines across 43 files.** Final pass: svelte-check 0/0 · vitest 122/122 · cargo check clean · cargo test 52/52.

- **Gone:** Harness workspace (3 workspaces now) · `swarm/` · session-log subsystem · SQLite cost cockpit + **rusqlite dep** (usage = `limits.rs` only) · compaction (**#33 closed by removal**; ctx≥70% nudge survives w/ Ctrl+T copy; legacy boundary pills still render) · custom providers + compression proxy (turn.rs `ANTHROPIC_BASE_URL` seam deleted; API-key `--bare` path KEPT).
- **Home bento** re-balanced to 3 tiles (ws · jump · limits). `usage.svelte.ts` = gauges-only.
- **Keepers verified intact:** `cleanup_retired_jsonls` · `environment_check` · `usage_rate_limits` · dictation · enhance wand · browser dock · cost cap · `telemetry.turns` · trust-migration arms.
- Deviation from brief: none material. `send.ts` still passes `priorContextSummary: null` (backend param kept, harmless). Backend tests 93→52 (deleted modules carried their suites).
- Docs: ISSUES re-indexed (#33 🗄, #31 provider item superseded), project CLAUDE.md hot-files re-measured.

### Phase 3 + Phase 4 (dev side) — also DONE this session

- **#34 FIXED + live-verified** (`d5e8c3a`): memoized header counts (countFor cache) · default-collapse >8 files or >400-line group · `EditDiff maxLines` prop (SessionDiff passes 200, "Show N more" strip) · `content-visibility:auto` on `.dg`. CDP synthetic 20-file/200-edit repro: instant open, crisp collapsed headers, 0 errors; 400-line Write capped at 200 rows → reveal works.
- **CDP sweep of all 3 workspaces green** (dev build): Home 3-tile bento + live gauges · Chat welcome/composer · Settings→Assistant (compaction/providers/compression sections confirmed GONE, API-key + cost guard + 2-level trust intact). New ctx≥70% nudge copy renders; cwd badge fires on mismatch. 0 console errors everywhere. Dev binary killed by PID (11980, path-verified) — prod untouched.

### v0.9.0 SHIPPED + docs cleaned (user-authorized, same session)

- Bumped ×3 + Cargo.lock, CHANGELOG rewritten, **tag `v0.9.0` pushed → tag-driven CI** publishes to rift-releases (run 27448186035).
- **Docs sweep:** deleted retired design docs (`rift-roadmap` · `idea-phase-plan` · `edit-swarm-safety-layer` · completed `minimal-core-strip` + `scripts/proto/swarm-harness.ps1`); IDEAS.md rewritten w/ pivot note + surviving seeds only; ISSUES stale Harness refs pruned + verification section refreshed; README feature list updated. Kept: 6 docs/ files + 7 design refs (splits ×5, velopack, self-hosted-runner, ui-audit).

### RESUME HERE

1. **Verify the shipped build:** prod app auto-updates to v0.9.0 (or install from rift-releases) → smoke: 3 workspaces, a real turn, #29 CSP check (transitions animate, update progress fills, 0 violations).
2. Fresh-machine pass on a buddy install: onboarding → logged-out sign-in (closes Auth-Rec) → updater round-trip. Permission-bar on a trust-standard throwaway repo.
3. Distribute (unsigned — SmartScreen "More info → Run anyway" note).
4. Fable sunset sweep post-Jun 22.

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
