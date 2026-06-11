# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-11 (cont.107) — new-user readiness audit + fix batch → v0.8.19 shipped

svelte-check 0/0 (4092) · vitest 120/120 · cargo check + new lockstep test clean (isolated `--target-dir` b/c dev was live) · CDP pixel pass (onboarding Step 4, localStorage flags restored after).

- **Three-agent audit** (hardcoded assumptions · first-run UX · missing-prereq failure handling) → tiered findings, then ALL tiers fixed. Two claims were FALSE on verify: onboarding gate race (refreshAuth IS awaited before configLoaded) and Whisper-not-gated (Settings already disables + warns). Three more already handled: swarm gate hints, usage-DB error log, env-key st-note.
- **Tier-1:** `probe_version_at` (cli_install.rs) bounded 5s via try_wait loop — hung `claude --version` used to wedge splash forever · onboarding Step 4 **Permissions picker** (default/acceptEdits/bypass from `MODE_OPTIONS`, bypass disclosed) · send.ts warns when custom provider w/o key falls back to Anthropic key.
- **Tier-2:** send.ts no-workspace notice + Fable-sunset one-shot warning (7d ahead) · `--bare` consequences explained at ClaudeConnect + Settings key entry · login console-behind-window hint · turn.rs `write_control_response` fail-loud (`io::Error::other`, was `unwrap_or_default` → CLI hang) · update_service taskkill sweep logs failures + image from `current_exe`.
- **Tier-3:** bundle-ID lockstep test (`diagnostics::tests`, `include_str!` tauri.conf.json) · About → rift-releases link · STT placeholder de-FiveM'd · BLAZZER out of redact fixtures. Deliberately skipped: FiveM welcome suggestions (intentional, fxmanifest-gated) · swarm `rift.local` git identity (product choice) · in-app help panel (bigger design work).
- **Shipped v0.8.19** — bundles cont.106 (context menus + Fable ctx fix + model menu) + this batch. Bump ×3 + Cargo.lock synced via `cargo metadata --offline`.

### RESUME HERE

- **v0.8.19 tagged + pushed — VERIFY release CI green** (`gh run list` on rift-tauri, then asset on rift-releases).
- User prod = 0.8.12 → still needs ONE manual Setup.exe onto the Velopack train.
- Next bites: audit remainder (#7 charts · #12 chip affordance · #11/#13 design passes · `/history` + hover-actions checks), then Settings per-page checklist. New-user polish leftovers (POLISH tier): mic-permission deep link, model-download disk check, in-app help panel.
- Parked: SEC-1 live pass · #29 CSP-nonce (needs prod build) · CR-UX trust-enum · VM Administrator password rotation (cont.105).

## Prior arcs — detail in `git log` + CHANGELOG

cont.106 app-wide custom context menus (`contextMenu.svelte.ts` + `ContextMenuHost`, `preventDefault()` ownership convention, clipboard-manager plugin) + Fable 1M ctx fix + model-menu two-line reorg. cont.105 #4 UI sweep 9/13 + **v0.8.18 shipped** (annotated tags enforced). cont.104 Rail-v2 + turn.rs registry race fix. cont.103 effort ladder CLI 1:1 + composer split C1-C7. cont.94 Fable 5 limited-run (**Jun 22 sunset gate**). cont.90 first tag-driven release; **`RunnerKeepAlive` startup task load-bearing.** PID-only kills, NEVER by image name.

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
