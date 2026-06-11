# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.111) — full-codebase audit + cleanup → v0.8.22 ship

**Shipped v0.8.22** (tag-driven CI): cont.110's multi-tab stream-kill fix + Harness live-ops arc PLUS this session's audit sweep. All green pre-tag: svelte-check 0/0 (4093) · vitest 122/122 · cargo check/clippy 0 warnings · cargo test 95/95 · live CDP pass 0 console errors.

- **Audit sweep (3 parallel agents: backend/frontend/orphans, findings self-verified before edits):** diagnostics/mod.rs slimmed ~200L (26 dead sync-era DiagStage variants → `Log`+`System`, dead counters/ring/accessors, dead frontend-error chain incl. `utils/diag.ts` + `diag_log_frontend_error` + `emit`), dead `cache_path`/`safe_profile_key`, SidePanel passthrough inlined into AssistantPane, dead `panelTab`/`--muted`, dedups (`SKIP_DIRS` → mcp_server's pub(crate); `modelHue`/`shortModel` → new `workspaces/helpers.ts`; `fmtElapsed`→`fmtClock`), `@xterm/*` ×4 uninstalled, MSIX icons + `cdp/send.sh` + `steer-and-queue.md` deleted, stale sync/RCON/WPF comments purged.
- **Fixes:** `/history` slash command was a no-op → store one-shot request flag consumed by ChatTabsBar `$effect` (live-verified, closes the #4 second-Enter item) · poison-safe `CACHE` locks (limits.rs) · config/budget parse failures now `log::warn!` instead of silent defaults.
- **ISSUES.md:** Rail-v2 + #20 blocks pruned at ship; **new #31** = deferred remainder (legacy `base_url`/`provider_model` commands — needs user call; turn.rs 401-dup helper; blocking-fs-in-async; **Fable dead-branch sweep after Jun 22**).

### RESUME HERE

- **v0.8.22 tagged + pushed — confirm CI release green** (`gh run list` on `release.yml`; needs `RELEASES_TOKEN` repo secret as always).
- User prod app still needs one manual Setup.exe (pre-Velopack install) — unchanged.
- **New ISSUES #31** (audit deferred items) + #30 (workspace-chip drift, unconfirmed).
- CDP test wart: synthetic `.click()` doesn't fire row handlers (welcome recents need real pointer events — same as tabsbar/model-menu rows).
- Carried: `browser_screenshot` MCP design arc · audit remainder (#7 charts · #12 chip affordance · #11/#13) · Settings checklist · POLISH tier · SEC-1 · #29 CSP-nonce · CR-UX.

## Prior arcs — detail in `git log` + CHANGELOG

cont.110 multi-tab stream-kill fix (live TabState authoritative, pure pointer-switch) + Harness mission control/drill-down/health-alerts/plan-bars — shipped in v0.8.22. cont.109 self-aware Rift: `assistant/bridge.rs` loopback resurrection (ask_user/open_browser/notify), per-turn env snapshot, v0.8.21. cont.108 live plan limits + v0.8.20. cont.106 custom context menus + Fable 1M fix. cont.104 Rail-v2 + turn.rs registry race fix. cont.94 Fable 5 (**Jun 22 sunset gate**). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Live TabState is authoritative over disk** — never re-add `stop()` to `loadConversation` or disk-reload a tab in `host.tabs`; bg streams die/clobber (cont.110 fix; regression tests guard).
- **Onboarding gate (cont.55)** · **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest) · **Right-click ownership** (`preventDefault()` or global double-fires).
- **Accent via `--accent-h`**; tint mixes `in oklab`, never `in oklch`. **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces**, nav in titlebar. **AssistantPane drop handlers on `.pane` outer only**. **Blur-reveal:** `shownCount` only `$state` via rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.22 stands.**
- **`turn.rs::kill_all_session_children` re-export** + **bridge env injection in `write_mcp_config`** — load-bearing (Velopack apply / ask_user trio).
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
