# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-10 overnight (cont.103) — effort retune + composer split COMPLETE

Autonomous overnight run (user-authorized). Everything below svelte-check 0/0 · vitest 116/116 · cargo check clean · CDP pixel-verified live, one commit per unit.

- **Effort ladder retuned to mirror the CLI 1:1** (`e62a896`). Was: default "Smart"=medium (below the API's `high` default and CC's `xhigh` agentic default), xhigh only via ultracode, invented latency hints. Now: `none→low · quick→medium · smart→high (default) · deep→xhigh · ultra→xhigh+ultracode`; Sonnet 4.6 caps at smart (xhigh is Opus/Fable-tier per claude-api ref), Haiku unchanged (API rejects effort). New `"smart"` id added to `ThinkingEffort` — stored prefs stay valid; frontend `effortToFlag` + `turn.rs` match + `config.rs` docs in lockstep (mirror test updated). **Proven end-to-end:** spawn log `model=sonnet effort=high` on a real turn. Panel now shows the literal flag (`--effort high`) beside the tier name (`b32196c`).
- **Composer split FINISHED — C3-C7 in one night** (ISSUES #20 ✅ threshold met: no repo file >2000L). Composer 3131→**1845L**; children: QueueRail 322 (C3) · LivePills 212 (C4) · EnhanceBar 264 (C5, presentational seam — state machine stays parent) · SlashMenu 75 + MentionPopover 110 (C6) · SettingsMenu 370 + PermMenu 147 + `modelMatrix.ts` (C7 — shared option tables; onKey + children can't drift). Deviations documented in `composer-split.md` header (kept as pattern ref).
- **Steer T2 live-verified → block deleted from ISSUES:** queued chip mid-stream, ✓Steered flash, visible mid-turn redirect (finished in-flight read → `REDIRECTED` → skipped remaining reads).
- **Permission T2 → blocked-by-design:** derived-trust workspace correctly hides git-write tools, so the Allow/Deny bar can't fire there. Verifying needs a pinned trust=standard repo — left for the CR-UX decision (trust pins one-way).
- Bonus fixes: enhance-error ✕ unstyled since C2 (now `.enhance-error-x`); missing `hint-in` keyframes (perm menu never tweened).

### RESUME HERE

- **Nothing in-flight.** Tree clean on `main`, dev server + cdp:serve were left running.
- Natural next bites: **Rail-v2** (steer chips + mode toggle — now lands in the small QueueRail child, see ISSUES), **#4 UI sweep** via `ui-audit-2026-06-09.md` 13 findings, or **ship batch** (user prod = 0.8.12 → still needs ONE manual Setup.exe; bump ×3 + Cargo.lock → CHANGELOG → tag, CI does the rest).
- Parked: SEC-1 live pass · #29 CSP-nonce (needs prod build) · CR-UX trust-enum (+ Permission bar verify rides it) · `.tmp/runner/` scripts fate.
- CDP wart noted in ISSUES: `look`'s error list accumulates since cdp:serve boot — trust the screenshot, or restart cdp:serve.

## Prior arcs — detail in `git log` + CHANGELOG

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
