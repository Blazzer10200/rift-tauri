# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.22 — 2026-06-11 — multi-tab stream survival + Harness mission control + dead-code sweep

> **Why.** Switching tabs killed the running session — two singleton-pipeline relics: `loadConversation` stopped the *outgoing* tab's stream on every switch, and `openTab` disk-reloaded tabs whose live TabState was already in memory, clobbering in-flight background streams.

- **Tab switches are pure pointer-switches now** — live TabState is authoritative over disk (`tabs.ts::openTab` guard; `loadConversation` stop removed). Backend was already concurrent-safe (per-session PIDs, per-turn MCP configs). 2 regression tests; CDP-proven end-to-end: turns survive new-tab + tab/workspace switches with full transcripts.
- **Harness live-ops arc:** active-sessions mission control (one row per streaming tab — title→jump, activity, elapsed, ctx%, tool count, stop — + live tool waterfall via new `liveTabs` getter) · turn drill-down (timeline bars → stats grid + per-tool list, live + archived) · health alerts (`healthAlerts.ts`: bg-turn finished/failed toasts w/ jump action + once-per-session deadWait/staleCache/≥3-toolErrors warnings) · plan-limit 5h/7d bars in the live hero.
- **`/history` fixed** — the slash command toggled a flag nothing rendered; now opens the History drawer (store request flag → ChatTabsBar). Live-verified.
- **Dead-code sweep (−331 lines, 12 icons, 4 npm pkgs):** diagnostics bus slimmed ~200L (26 sync-era stages, dead counters/ring/accessors, dead frontend-error chain incl. `utils/diag.ts` + backend command), dead `cache_path`/`safe_profile_key`, `SidePanel` passthrough inlined, dead `panelTab` state + `--muted` CSS var, `SKIP_DIRS`/`modelHue`/`shortModel`/`fmtClock` deduped, `@xterm/*` uninstalled, MSIX icon set + `cdp/send.sh` + completed `steer-and-queue.md` brief deleted, stale sync/SFTP/RCON/WPF comments purged across config.rs/turn.rs/state.
- **Hardening:** poison-safe `CACHE` locks in `usage/limits.rs`; assistant + budget config parse failures now log a warning instead of silently resetting to defaults.

**Verify.** svelte-check 0/0 (4093 files) · vitest 122/122 · cargo check + clippy zero warnings · cargo test 95/95 · live CDP pass: /history drawer opened, ActivityPanel dock renders, conversation load intact, 0 console errors.

## Older versions

v0.8.21 self-aware Rift — loopback UI bridge resurrected (`bridge.rs`: ask_user card round-trip / open_browser dock / notify toast) + per-turn env snapshot + localhost links open in-app · v0.8.20 live plan limits — cost-cockpit "Plan limits" card + `/usage` popover via undocumented OAuth usage endpoint (CLI token read-only, 60s cache) · v0.8.19 custom context menus app-wide + Fable 1M ctx fix + model menu reorg + new-user hardening batch · v0.8.18 UI sweep — 9 audit findings + per-chat model scoping, slash-menu palette grammar, Home/Welcome snippets · v0.8.17 Rail-v2 steer chips + `turn.rs` overlapping-turn registry race fix · v0.8.16 backend split COMPLETE (`assistant/mod.rs` 4331→303, R1-R8) · v0.8.15 hot-file splits + honest Settings update chip · v0.8.14 update-dialog render crash fix + swarm worktree-escape guard · v0.8.13 Claude Fable 5 limited-run model · v0.8.12 pill `×` = 24h snooze · v0.8.11 Settings redesign + Harness one-viewport · v0.8.10 stable singleton `UpdatePill` · v0.8.9 first tag-driven CI release · v0.8.5 corrupted install no longer "up to date" · v0.8.3 updater can't hang forever · v0.8.0 one-click 401 recovery + edit-swarm + compression · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
