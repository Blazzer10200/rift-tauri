# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-10 (cont.102) — first-run setup redesign

Full onboarding redo, visual + functional. svelte-check 0/0, every step CDP pixel-verified live (gate re-opened via localStorage clear, flags restored by finishing the flow).

- **OnboardingFlow** — 4 new steps: Welcome (beta notice folded in as warn row + inline accent strip) → Connect Claude → Open a project → Defaults. AppShell gate contract untouched (`onDone` = dismiss + acknowledge).
- **ClaudeAuth → ClaudeConnect** (git mv) — active step: 4s auto-poll probe; CLI-missing → copyable `irm https://claude.ai/install.ps1 | iex`; not-signed-in → `startLogin()` CTA + API-key alt path (`setApiKey`). CLI-missing/sign-in branches logic-verified only (this machine fully authed).
- **Step 3 Open a project** — `pickFolder()` + recent roots via `setRoot()`, skippable; `\\?\` long-path prefix stripped for display.
- **Step 4 Defaults** — model seg (Fable gated on `fableAvailable()`), effort seg capped at model `maxEffort` (`pickModel` clamps saved effort down), git-tools trust seg — all through existing store setters.
- **Setup chrome leak fix** — Titlebar `setupMode` prop hides workspace nav + cmdk pill + settings gear (brand + winctl stay); AppShell `onGlobalKey` early-returns while `showOnboarding` so Ctrl+K/P/1-9/, can't fire over setup. Verified: synthetic Ctrl+K/Ctrl+1 no-ops, chrome returns on finish.
- ObStage kinds now `welcome|claude|project|defaults`; onboarding.css gained seg/copy-block/input/recent/accent-inline patterns.

### RESUME HERE — C3 QueueRail (mapped, not started)

Per `composer-split.md` C3, anchors re-located 2026-06-10 (valid while Composer = 3048L):
- State cluster L47-99 (`editingId/editText/startEditQueued/commitEditQueued/onEditKey/removeQueued/sendQueuedNow` + `dragId` drag trio).
- Markup L1002-1085 (`.pending-rail` incl. steer/clear rail-actions).
- CSS L2210-2352 (`.pending-rail` → `.rail-clear` + `rail-sweep`/`rail-breathe` keyframes + the reduced-motion block L2348-2352).
- Seam: child keeps `assistant` import (sendQueuedNow → assistant.steer; brief allows). `steer()`/`steerFlash` STAY parent → props `steerFlash`, `draft`, `onSteer`; plus `tab`, `tabId`, `queue`, `streaming`. `fly/quintOut` transitions + Clock/Navigation/Pencil/X/Check icons move w/ markup.
- Then C4 LivePills (state cluster now L114-150) → C5-C7 per brief. One child per commit · verbatim moves · check 0/0 · vitest 116+ · CDP `look` per cut.

**User prod = 0.8.12** → still needs ONE manual `Setup.exe`; after that in-app update pulls latest. Ship-ready batch: bump ×3 + Cargo.lock → CHANGELOG → tag (CI does the rest).
- Parked: SEC-1 live pass · #29 CSP-nonce · CR-UX trust-enum + `previewOf` wire-or-drop · `.tmp/runner/` scripts fate.

## Prior arcs — detail in `git log` + CHANGELOG
cont.101 full CDP smoke run + Home boot-empty & init() double-listen fixes (`initPromise` memo) + C2 AttachmentsRow (Composer 3131→3048L). cont.100 C1+H0 extractions, vitest 51→116. cont.99 boot splash. cont.98 v0.8.16 (#20 backend split, mod.rs→303L hub). cont.97 v0.8.15 (TS split M0-M9). cont.94 Fable 5 limited-run (**Jun 22 sunset gate**). cont.90 first tag-driven release; **`RunnerKeepAlive` startup task load-bearing.** PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`.
- **Accent via `--accent-h`** (app.css `:root` only); tint mixes `in oklab`, never `in oklch`. Status LEDs fixed.
- **Surface tiers:** page 0.142 · card 0.215 · wells 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces**, nav in titlebar, positional `workspace.order`. Harness = one viewport.
- **AssistantPane drop handlers on `.pane` outer only**; `dragDropEnabled:false`.
- **Blur-reveal:** `shownCount` only `$state`, written only by rAF loop.
- **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.8.16 stands.**
- **`turn.rs::kill_all_session_children` re-export** — load-bearing for Velopack apply.
- **Pure-helper modules + vitest nets + `assistant.init()` initPromise memo** — don't re-inline / don't revert to bare guard.
