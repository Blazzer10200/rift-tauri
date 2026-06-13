# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-13 (cont.120) — UI Polish Arc EXECUTED (§1–§6) ✅

All six backlog items of `docs/design/ui-polish-arc.md` shipped as 6 commits on main, each svelte-check 0/0 + tests green, CDP-verified against the live dev UI. Map (statuses updated in the arc doc):
- **§1 token counter** (`253a2b8`) — live-verify found it frozen (CLI gives no mid-stream `output_tokens`); rewrote to climb from a streamed-char estimate snapping exact per message. +2 regression tests.
- **§2 notifications** (`a0fc902`) — ~22 `lastNotice` sites → severity toasts (`notify.*`); banner now = slash reference output only; path bug + invisible-Settings-errors fixed; UpdatePill left alone (regressing the click-fix was the doc's mistake).
- **§4 lightbox** (`cc7dfa9`) — dead `window.open` → portal'd overlay.
- **§5 drag-drop** (`aa9e2cb`) — window-level drop guard (no more file-nav break) + non-image feedback + deduped handlers.
- **§3 activity** (`4be207c`, 5/7) — timestamp dedup, quoted regex targets, colour-coded icons, promoted cost, live-token in Now strip.
- **§6 streaming** (`44cb8b2`) — conservative pacer tuning (less compounding latency); architectural merge/incremental-parse/code-reveal deferred as high-risk.

**Not shipped (no version bump):** v0.9.0 stands; these are uncommitted-to-release dev-branch fixes. Next ship gathers them into a bump.

## Session 2026-06-12 (cont.119) — minimal-core strip + v0.9.0 ship ✅

Strip shipped (`470845b`…`bd1709c`, **−7,407 lines**): Harness/Swarm/session-log/SQLite-cockpit/compaction/custom-providers all gone → **3 workspaces** (Home·Chat·Settings), `usage`=gauges-only. #34 diff-perf FIXED (`d5e8c3a`). **v0.9.0 tagged + CI-published** to rift-releases. Detail in git + CHANGELOG.

### RESUME HERE — §7 Harness rebuild (next), then ship

UI Polish Arc §1–§6 are DONE (above). Remaining:

1. **§7 Harness rebuild** (`docs/design/ui-polish-arc.md` §7) — rebuild Harness page + Cost cockpit + Swarm cleanly to match the new minimal-core aesthetic, NOT a `git revert` of the strip. Recovery source intact up to `ba1f7dc`. Scope it fresh when starting.
2. **Ship the arc** — gather §1–§6 into a version bump (`bump.ps1` → CHANGELOG → tag) when ready. v0.9.0 currently stands.
3. **Optional §3/§6 follow-ups** (deferred, low priority): §3 collapse-same-tool-runs, §6 full pacer merge / incremental tail-parse / code-block reveal — all noted in the arc doc as intentional-skips, not bugs.

**Older RESUME (v0.9.0 ship verify — still valid):** prod auto-updates to v0.9.0 → smoke 3 workspaces + real turn + #29 CSP. Fresh-machine buddy install. Fable sunset sweep post-Jun 22.

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
