# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.105) — #4 UI sweep: audit findings 1-6 + 8-10 SHIPPED

Autonomous run. svelte-check 0/0 (4089 files) · cargo check clean · vitest 119/119 (1 new) · live CDP end-to-end, 0 console errors.

- **9 of 13 ui-audit findings done** (see ISSUES #4 for the list). Highlights: `shellLabel` (helpers.ts) strips `cd … &&` hops in rail + live rows; SlashMenu rebuilt in palette grammar (icons/groups/highlight/kbd); empty-tab dock auto-collapse (`AssistantPane dockOpen ∧ !showEmpty`); **per-chat model scoping** — `TabState.modelOverride` + `store.effectiveModel`; explicit `setModel` writes default+override; `loadConversation` sets override only (no toast/persist); send/composer/tabsbar/harness read effective, Home+onboarding read default; convo save keeps per-tab model; `asModelSel` validator (fable-sunset aware) replaced stale allow-list. Backend: `ConversationMeta.last_snippet` (convo_store.rs) feeds Home/Welcome row snippets.
- Live-verified: rail row `git status --short` (cd stripped), Opus chat pill w/ Home default still Sonnet + no toast + nav works (audit's "doesn't navigate" = not a bug), insight stripes via computed styles, `sonnet · high` chip space.

### RESUME HERE

- **v0.8.18 SHIPPED this session** (bump ×3 + Cargo.lock → CHANGELOG → tag → CI) — verify the release run went green: `gh run list --limit 2`. Dev server + cdp:serve left running. User prod = 0.8.12 → still needs ONE manual Setup.exe to get on the Velopack train.
- Next bites: audit remainder (#7 charts · #12 chip affordance · #11/#13 design passes · `/history` + hover-actions checks), then Settings per-page checklist.
- Parked: SEC-1 live pass · #29 CSP-nonce (needs prod build) · CR-UX trust-enum (+ Permission bar verify rides it) · `.tmp/runner/` scripts fate.

## Prior arcs — detail in `git log` + CHANGELOG

cont.104 Rail-v2 shipped (per-chip steer/queue toggle, next-turn inject) + turn.rs registry race fix (`clear_session_pid_if`/`clear_steer_tx_if` — a turn only clears its own entries); v0.8.17 tagged, CI green. cont.103 effort ladder retuned to CLI 1:1 (smart=`--effort high` default; lockstep ×3 guarded by mirror test) + composer split COMPLETE C1-C7 (no repo file >2000L).

cont.102 first-run setup redesign (4 steps, chrome-leak fix). cont.101 smoke run + double-listen fix + C2. cont.100 C1+H0s, vitest 51→116. cont.98 v0.8.16 (#20 backend split). cont.97 v0.8.15 (TS split M0-M9). cont.94 Fable 5 limited-run (**Jun 22 sunset gate**). cont.90 first tag-driven release; **`RunnerKeepAlive` startup task load-bearing.** PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH

- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`.
- **Effort mapping lockstep:** `effortToFlag` (helpers.ts) ↔ `turn.rs` match arm ↔ `modelMatrix.ts` tables — change all three together; the vitest mirror test guards it.
- **Accent via `--accent-h`** (app.css `:root` only); tint mixes `in oklab`, never `in oklch`. Status LEDs fixed.
- **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces**, nav in titlebar, positional `workspace.order`. Harness = one viewport.
- **AssistantPane drop handlers on `.pane` outer only**; `dragDropEnabled:false`.
- **Blur-reveal:** `shownCount` only `$state`, written only by rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.18 stands.**
- **`turn.rs::kill_all_session_children` re-export** — load-bearing for Velopack apply.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo + composer/ children** — don't re-inline.
