# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-10 evening (cont.104) — Rail-v2 SHIPPED + turn-registry race fix

Autonomous run (user-authorized). vitest 118/118 (2 new) · svelte-check 0/0 · Rust verified via tauri-dev rebuild (clean relaunch) · live CDP end-to-end.

- **Rail-v2 (ISSUES ✅):** per-chip queue/steer **mode toggle** (↳ button, accent-tinted steer chips, caption "Sends when ready · Steers next turn"), steer chips inject into the **next** turn at its first stream line, pulse-on-inject (sweep replay), all-steer queue degrades head → send (never strands). Plumbing: `TabState.onTurnStarted` hook (fires once per turn on first stream line, latch in `beginTurn`) → `send.ts::flushSteerChips`; `drainQueue` now picks the first non-steer chip. "Send now" unchanged.
- **Preexisting backend race found via live-verify and FIXED (turn.rs):** DONE emits on `result` before child reap → next turn re-registers `SESSION_PIDS`/`STEER_TX` under the same key → old turn's tail unconditionally cleared both. Broke steer AND `assistant_stop` for the first ~seconds of every drained follow-up turn (reap grace = 5s). Fix: `clear_session_pid_if` (PID match) + `clear_steer_tx_if` (`same_channel`) — a turn only clears its own entries. Proof: pre-fix the steer chip fell back to its own turn; post-fix the "You steered" marker landed inline in the drained turn's bubble.

### RESUME HERE

- **Nothing in-flight.** Dev server + cdp:serve left running; commit pending push? — check `git log origin/main..main`.
- Natural next bites: **#4 UI sweep** via `ui-audit-2026-06-09.md` 13 findings, or **ship batch** (user prod = 0.8.12 → still needs ONE manual Setup.exe; bump ×3 + Cargo.lock → CHANGELOG → tag, CI does the rest — Rail-v2 + race fix ride it).
- Parked: SEC-1 live pass · #29 CSP-nonce (needs prod build) · CR-UX trust-enum (+ Permission bar verify rides it) · `.tmp/runner/` scripts fate.

## Prior arcs — detail in `git log` + CHANGELOG

cont.103 effort ladder retuned to CLI 1:1 (smart=`--effort high` default; lockstep ×3 guarded by mirror test) + composer split COMPLETE C1-C7 (no repo file >2000L) + steer live-verified.

cont.102 first-run setup redesign (4 steps, chrome-leak fix). cont.101 smoke run + double-listen fix + C2. cont.100 C1+H0s, vitest 51→116. cont.98 v0.8.16 (#20 backend split). cont.97 v0.8.15 (TS split M0-M9). cont.94 Fable 5 limited-run (**Jun 22 sunset gate**). cont.90 first tag-driven release; **`RunnerKeepAlive` startup task load-bearing.** PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`.
- **Effort mapping lockstep:** `effortToFlag` (helpers.ts) ↔ `turn.rs` match arm ↔ `modelMatrix.ts` tables — change all three together; the vitest mirror test guards it.
- **Accent via `--accent-h`** (app.css `:root` only); tint mixes `in oklab`, never `in oklch`. Status LEDs fixed.
- **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces**, nav in titlebar, positional `workspace.order`. Harness = one viewport.
- **AssistantPane drop handlers on `.pane` outer only**; `dragDropEnabled:false`.
- **Blur-reveal:** `shownCount` only `$state`, written only by rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.16 stands.**
- **`turn.rs::kill_all_session_children` re-export** — load-bearing for Velopack apply.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
