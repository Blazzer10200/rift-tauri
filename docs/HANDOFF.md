# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 86) — First tag-driven CI release SHIPPED (v0.8.9)

**The CI tag-push pipeline is now PROVEN end-to-end.** `v0.8.9` is published in `rift-releases` (Setup.exe + full.nupkg + releases.win.json + RELEASES; portable dropped). First release ever produced by Actions, not a hand-run `release.ps1`.

**Two real bugs the test exposed + fixed (both in `release.ps1`):**
1. **Dirty `RELEASES_TOKEN` secret** — a non-ASCII char (BOM/zero-width/smart-quote from paste) in the PAT survived `.Trim()` → Octokit threw `Request headers must contain only ASCII characters` at `vpk upload`, AND silently broke `gh` CLI auth (which masked the "already exists" preflight + no-op'd the portable-drop). **Fix:** strip token to printable ASCII `[^\x21-\x7E]` ONCE up-front (commit `a4fbbce`), shared by both `gh` (GH_TOKEN) and `vpk`. Strip fired + upload succeeded → junk was leading/trailing.
2. **Targeted an already-shipped version** — v0.8.8 was already live in `rift-releases` (separate repo; source-repo had no tag, which misled me). **Fix:** bumped to v0.8.9.
- Also added `timeout-minutes: 40` to `release.yml` so a genuine hang fails loud (was 6h default).

**Timing reality (for next time):** cold Tauri release build = ~18 min in the Release step on `windows-latest`, cache or not (the rift crate's own opt+link dominates, recompiles every version bump). NOT a hang — felt long but normal.

### RESUME HERE (cont.86)
- **`RELEASES_TOKEN` secret is still dirty at source** — the strip fix self-heals it, but re-set the secret cleanly (`gh secret set RELEASES_TOKEN --repo Blazzer10200/rift-tauri`) to kill the root cause + the warning.
- **Node 20 action deprecation** — `actions/*@v4` forced to Node 24 on **June 16**; bump action majors before then.
- Carried from cont.85: drag-reorder manual-verify pending; dedupe `dirs_home` (`assistant/mod.rs:1101` vs `state::paths`); decide Whisper-in-release; ISSUES #100 hero-pill hardcode (`SettingsPage.svelte:318`).
- Local = Win PowerShell 5.1 (no `pwsh`).

## Prior arcs — detail in `git log`
cont.85 committed all WIP (4 logical groups) + standalone hardening (env_check cmd + Local-tools card, per-workspace model fix). cont.84 tag-driven CI scaffolding. cont.83 steer/queue (CDP-verified). cont.82 MCP-nonce + Rail v1. cont.72 v0.7.0 + edit-swarm.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step** before working; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = **hero + sticky pill-tabs + per-section bento** (redesigned cont.78, was sidebar+5-sections); Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at ship. **v0.7.0 stands** (shipped 2026-06-07, cont.72). Harness now has THREE sub-tabs (Telemetry · Cost · Swarm) — still one workspace, IA unchanged.
