# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-10 (cont.109) — self-aware Rift: UI bridge resurrection + AI-driven app surfaces

svelte-check 0/0 (4093) · cargo check clean (forced recheck, zero warnings) · full CDP live pass. **Shipped v0.8.21** — bump ×3 + Cargo.lock, CHANGELOG rewritten, annotated tag pushed.

- **Ghost-tool find:** `mcp__rift__ask_user` was steered-to (turn.rs deny msg) + allowlisted but UNREGISTERED since the pure-assistant rip killed `remote_bridge.rs` — model was told to call a tool that didn't exist. Frontend stack (FIFO binding, card UI, `assistant_answer_ask_user`) was dormant-intact.
- **`assistant/bridge.rs` (new):** minimal loopback resurrection — NDJSON over `127.0.0.1:<rand>`, single per-launch token, ops `ask_user` / `open_browser` / `notify`. Started in lib.rs setup; `write_mcp_config` injects `RIFT_BRIDGE_PORT/TOKEN` (absent → MCP child degrades gracefully, tools unlisted).
- **mcp_server.rs:** `bridge_enabled()`-gated tool trio + `bridge_call()` (660s read timeout for ask_user, 10s others). Deps `rand 0.10` + `base64 0.22` re-added.
- **turn.rs:** addendum rewritten — documents ask_user/open_browser/notify + env snapshot semantics; SAFE_MCP cleaned of ghost sync tools (`{BUILTINS},{SAFE_MCP},...` reuse in scoped branch). Per-turn "Rift environment snapshot" rides the user-msg `<system-reminder>` (cache-stable): dock current URL + plan usage via `limits::cached_snapshot()` (≤5min, non-blocking) + `spawn_background_refresh()` fire-and-forget warm-up.
- **Frontend:** `assistant://open-browser` → `browserDock.openUrl()` (new `pendingUrl`, consumed by WebBrowserPage `$effect` once stage mounts); `assistant://notify` → toast; Markdown localhost/127.0.0.1 links → dock instead of system browser.
- **CDP-verified live:** bridge boot log · open_browser opened dock + navigated (9223/health) · warn toast pixel-confirmed · ask_user full round-trip (card → Yes → model echoed "Yes") · link intercept (0 console errors, no external browser) · session JSONL carried snapshot w/ dock URL + "5-hour window 38% used".

### RESUME HERE

- **v0.8.21 tag pushed at session end — verify release CI green next session** (4 assets on rift-releases).
- `browser_screenshot` MCP tool (AI sees the dock page → self-verify loop) parked as its own design arc — needs image transport through the bridge.
- CDP screenshots can't capture the native child webview (dock area renders blank in shots) — verify dock content via address-bar sync / `browser_read_page`, not pixels.
- User prod = 0.8.12 → still needs ONE manual Setup.exe onto the Velopack train.
- `/usage` endpoint undocumented — if card shows "Unavailable", check `anthropic-beta: oauth-2025-04-20` header version.
- Next bites: audit remainder (#7 charts · #12 chip affordance · #11/#13 design passes · `/history` + hover-actions checks), Settings per-page checklist, POLISH tier (mic-permission deep link, model-download disk check, in-app help panel).
- Parked: SEC-1 live pass · #29 CSP-nonce (needs prod build) · CR-UX trust-enum · VM Administrator password rotation (cont.105).

## Prior arcs — detail in `git log` + CHANGELOG

cont.108 live plan limits (`usage/limits.rs` OAuth `/usage` fetch — CLI token READ-ONLY — + CostPage card + `/usage` popover) + **v0.8.20 shipped**. cont.107 new-user readiness + v0.8.19. cont.106 custom context menus + Fable 1M ctx fix. cont.105 #4 UI sweep + v0.8.18. cont.104 Rail-v2 + turn.rs registry race fix. cont.103 effort ladder CLI 1:1 + composer split. cont.94 Fable 5 limited-run (**Jun 22 sunset gate**). cont.90 first tag-driven release; **`RunnerKeepAlive` startup task load-bearing.** PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`.
- **Effort mapping lockstep:** `effortToFlag` (helpers.ts) ↔ `turn.rs` match arm ↔ `modelMatrix.ts` tables — change all three together; vitest mirror guards it.
- **Right-click ownership:** component context handlers MUST `preventDefault()` or the global fallback double-fires.
- **Accent via `--accent-h`** (app.css `:root` only); tint mixes `in oklab`, never `in oklch`.
- **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces**, nav in titlebar, positional `workspace.order`. Harness = one viewport.
- **AssistantPane drop handlers on `.pane` outer only**; `dragDropEnabled:false`.
- **Blur-reveal:** `shownCount` only `$state`, written only by rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.20 stands.**
- **`turn.rs::kill_all_session_children` re-export** — load-bearing for Velopack apply.
- **Bridge env injection in `write_mcp_config`** — load-bearing for ask_user/open_browser/notify; bridge OnceLock starts in lib.rs setup BEFORE first turn.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
