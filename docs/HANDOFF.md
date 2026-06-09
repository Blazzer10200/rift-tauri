# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 89) — Harness page visual overhaul (unreleased)

**All 3 Harness sub-pages reorganized for consistency + one-viewport fit (dev-verified CDP, NOT shipped).** User goal: clean top-of-the-line data dashboard; HATES scroll wheels = north star.

- **Telemetry strip** (`HarnessPage.svelte`): killed the horizontal-scroll pill river (was **4193px** hidden scroll / 40 pills) → `Live · recent-4 · All 40→`. `.sesh-recent` flex `overflow:hidden`, responsive cap (`.ses:nth-child(n+4)` hidden <1280px, `n+3` <1080px). Full browsing stays in All view.
- **Archived session-overview hero**: lone centered $cost → horizontal split — `.ov-main` (cost/model/date) left + `.ov-stats` 2×2 grid right (total tokens·reasoning·avg turn·peak parallel; complements KPI rail, no dupe). Dropped `.hero-tag` for `.ov` (collided w/ model line; removal also got page to 0 scroll).
- **SwarmPage.svelte** (was the outlier): title `--fs-lg`→23px; card titles uppercase-tiny→sentence-case `fs-sm/650`; radius 14→16px; icon-btn 32→30px; centered `.results-empty` + Boxes icon; added shared dotted-grid+accent-glow bg.
- **CostPage.svelte**: `.cost` pad 4/14→8/8, `.kpi` 12→10 → full bento 1 viewport.

`npm run check` 0/0 (4070) per batch. Scroll measured **0** on live+archived Telemetry, Cost, Swarm.

**NOT done (intentional):** Live/Cost gauge heroes left — the gauge IS the unique focal vs KPI rail (archived hero had none → needed the grid). Per-page formatters NOT merged (differ on purpose: Cost rounds `$80`, Telemetry `$79.84`).

### RESUME HERE (cont.89)
- Harness visual pass DONE + verified, **UNCOMMITTED** (stacks on cont.88 working tree). Not shipped.
- Optional next: richer Cost daily-spend chart; Reliability all-clear card centers awkwardly (minor).
- [carried cont.88] Settings deferred: merge green banner + "Claude Code CLI" row; friendlier compression-proxy copy; strip redundant `(Claude Code)` from CLI version chip (also in Harness config card).
- [carried] drag-reorder verify; dedupe `dirs_home` (`assistant/mod.rs:1101`); ISSUES #100 hero-pill; `RELEASES_TOKEN` re-set if dirty; Node-20 action deprecation Jun 16 (`actions/*@v4`). Local = Win PowerShell 5.1 (no `pwsh`).

## Session 2026-06-09 (cont. 88) — Settings page redesign + dead-setting audit (unreleased)

**`SettingsPage.svelte` overhauled (dev-verified CDP, NOT shipped; detail in `git log`).** 12-col bento → single 820px column of titled cards (`.st-block`=card w/ header band inside, `.st-card`=body, `.st-block-label`=sentence-case header; `.st-row-desc` code de-boxed + capped 60ch). Assistant tab regrouped: Cost guard · Model & routing (API-key+custom-provider fused, precedence note) · Compression proxy (advanced). Audit (3 agents): all settings wired; CUT dead **Accent presence** (`uiPrefs.presence`+`[data-presence]`) + dead `data-ligatures` write. `npm run check` 0/0.

**[DONE cont.88] self-hosted runner LIVE — releases free** (full detail: `docs/design/self-hosted-runner.md` + Daily 2026-06-09): VM 100 `rift-runner` @ `blazzer-labs` (192.168.1.20, WS2022, 4 vCPU/8GB), `proxmox-win` online as Administrator service; `release.yml`+`check.yml` flipped (`4dad1fa`,`372f84f`); warm `D:\cargo-target`+sccache; snapshot `toolchain-ready`. rust-lld REJECTED (codegen-bound). **Power-loss recovery VERIFIED** via VM `onboot:1` + **`RunnerKeepAlive` startup task** (`C:\runner-keepalive.ps1` retry-starts the service) — **load-bearing, DON'T delete**; plain service auto-start fails cold-boot (runner exits clean before network → SCM won't recover).

## Prior arcs — detail in `git log`
cont.87 v0.8.10 SHIPPED (singleton `UpdatePill.svelte`; CI cache → `Swatinem/rust-cache`). cont.86 first tag-driven CI release (v0.8.9); `release.ps1` fixes (`RELEASES_TOKEN` non-ASCII strip `a4fbbce`). cont.85 env_check/Local-tools card. cont.82 MCP-nonce + Rail v1. cont.72 v0.7.0 + edit-swarm.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step** before working; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = **hero + sticky pill-tabs + single-column titled cards** (`.st-block`=card, header band inside; redesigned cont.88, was 12-col bento/cont.78); Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at ship. **v0.7.0 stands** (shipped 2026-06-07, cont.72). Harness now has THREE sub-tabs (Telemetry · Cost · Swarm) — still one workspace, IA unchanged.
