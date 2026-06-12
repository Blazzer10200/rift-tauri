# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.9.0 — 2026-06-12 — Minimal core (buddy release) + #34 SessionDiff fix

> **Why.** First release distributed beyond the author. Rift slims to the core assistant loop — every power-user diagnostics surface that wasn't earning its complexity is gone (−7,407 lines / 43 files), and the one real T1/T2 bug pile (#33/#34) is closed.

**Removed (minimal-core strip, 7 slices):**
- **Harness workspace** (+ Cost/Swarm sub-tabs) — Rift is 3 workspaces now: Home · Chat · Settings. Ctrl+1/2/3 stay positional.
- **Edit swarm** backend (`swarm/`) + its Settings copy.
- **Session-log subsystem** (disk ring buffer + IPC) — in-memory telemetry (`/diag`, Activity panel, health alerts) unchanged.
- **Cost cockpit** (SQLite store/aggregate/budget/insights/pricing + the rusqlite dep). Plan-limit gauges survive (composer panel + Home tile); Home bento re-balanced to 3 tiles.
- **Compaction** — closes **#33** by removal. Long chats → Ctrl+T; the ≥70% context nudge survives with new copy; legacy boundary pills in old conversations still render.
- **Custom providers + compression proxy** — first-party Anthropic only. API-key fallback (keychain, `--bare`) unchanged.

**Fixed:**
- **#34** — Session Diff overlay no longer breaks down on long sessions: per-edit header counts memoized, groups default-collapsed above 8 files (or 400 changed lines per group), per-edit render capped at 200 lines with "Show N more", `content-visibility` on file groups. Live-verified against a synthetic 20-file/200-edit session.

**Kept (verified):** dictation/PTT, enhance wand, browser dock, per-turn cost cap, plan-limit gauges, 2-level git trust, Velopack auto-update chain.

**Verify.** svelte-check 0/0 · vitest 122/122 · cargo check clean · cargo test 52/52 · CDP sweep of all 3 workspaces green (0 console errors). 🧪 Pending on this build: #29 CSP prod verify, fresh-machine onboarding/sign-in, permission-bar live round-trip.

## Older versions

v0.8.26 composer slim + #29/#30/#12/CR-UX sweep · v0.8.25 dictation data-fence + PTT stuck-mic + #32 · v0.8.24 enhance wand v2 + voice commands · v0.8.23 Activity panel polish · v0.8.22 multi-tab stream survival + dead-code sweep · v0.8.21 loopback UI bridge (ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep · v0.8.17 Rail-v2 steer chips · v0.8.16 backend split COMPLETE · v0.8.13 Claude Fable 5 · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery · v0.7.0 cost cockpit · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
