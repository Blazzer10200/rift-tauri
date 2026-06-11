# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.108) — live plan limits (cockpit card + /usage) → v0.8.20 shipped

svelte-check 0/0 (4093) · Rust via live tauri-dev watcher (no manual cargo — dev was up all session) · live endpoint probe verified response shape before coding · CDP behavior pass (card pixels, slash entry, panel open/Esc/✕/click-outside).

- **Research first:** statusline JSON officially carries `rate_limits` now (Pro/Max, TUI-only — useless for Rift's headless `-p` spawns); NO official subscription-limits API; the community-proven path is `GET api.anthropic.com/api/oauth/usage`. Verified live: `utilization` 0–100 float, `resets_at` ISO 8601, nullable per-model weekly buckets, `extra_usage` block.
- **`usage/limits.rs` (new):** `usage_rate_limits` command — token from `~/.claude/.credentials.json` (**READ-ONLY, never refresh it** — one-time-use refresh tokens; external refresh breaks the CLI's own auth loop), headers `anthropic-beta: oauth-2025-04-20` + `User-Agent: claude-code/<ver>` (wrong UA = throttled bucket), tolerant serde, 60s cache, friendly 401/429/no-login errors.
- **Two surfaces, one data path:** CostPage "Plan limits" card (zone-colored bars + reset countdowns, fetch outside the cockpit `Promise.all`) + `/usage` composer popover (`UsagePanel.svelte`, SlashMenu Gauge/Info entry, `ui.usageOpen` flag, closes Esc/✕/outside-click).
- **Shipped v0.8.20** — bump ×3 + Cargo.lock (dev watcher synced it), CHANGELOG rewritten, annotated tag pushed.

### RESUME HERE

- **v0.8.20 release CI VERIFIED GREEN** — 4 assets live on rift-releases (Setup.exe, full.nupkg, RELEASES, releases.win.json). Nothing pending.
- User prod = 0.8.12 → still needs ONE manual Setup.exe onto the Velopack train.
- `/usage` endpoint is undocumented — if the card ever shows "Unavailable", check whether the `anthropic-beta: oauth-2025-04-20` header version changed (it has changed before).
- Next bites: audit remainder (#7 charts · #12 chip affordance · #11/#13 design passes · `/history` + hover-actions checks), then Settings per-page checklist. New-user polish leftovers (POLISH tier): mic-permission deep link, model-download disk check, in-app help panel.
- Parked: SEC-1 live pass · #29 CSP-nonce (needs prod build) · CR-UX trust-enum · VM Administrator password rotation (cont.105).

## Prior arcs — detail in `git log` + CHANGELOG

cont.107 new-user readiness audit (three tiers fixed: 5s version-probe bound, onboarding permissions picker, key-fallback warning, fail-loud control_response, bundle-ID lockstep test) + **v0.8.19 shipped**. cont.106 app-wide custom context menus (`contextMenu.svelte.ts` + `ContextMenuHost`, `preventDefault()` ownership convention, clipboard-manager plugin) + Fable 1M ctx fix + model-menu two-line reorg. cont.105 #4 UI sweep 9/13 + **v0.8.18 shipped** (annotated tags enforced). cont.104 Rail-v2 + turn.rs registry race fix. cont.103 effort ladder CLI 1:1 + composer split C1-C7. cont.94 Fable 5 limited-run (**Jun 22 sunset gate**). cont.90 first tag-driven release; **`RunnerKeepAlive` startup task load-bearing.** PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`.
- **Effort mapping lockstep:** `effortToFlag` (helpers.ts) ↔ `turn.rs` match arm ↔ `modelMatrix.ts` tables — change all three together; the vitest mirror test guards it.
- **Right-click ownership:** component context handlers MUST `preventDefault()` or the global fallback double-fires on top of them.
- **Accent via `--accent-h`** (app.css `:root` only); tint mixes `in oklab`, never `in oklch`. Status LEDs fixed.
- **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces**, nav in titlebar, positional `workspace.order`. Harness = one viewport.
- **AssistantPane drop handlers on `.pane` outer only**; `dragDropEnabled:false`.
- **Blur-reveal:** `shownCount` only `$state`, written only by rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.18 stands.**
- **`turn.rs::kill_all_session_children` re-export** — load-bearing for Velopack apply.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
