# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 93) — update-flow root cause FIXED + v0.8.12 SHIPPED

**User report:** v0.8.11 update "nothing popped up; UI scuffed; couldn't click." **Root cause:** pill `×` had version-permanently written `dismissed-version=0.8.11` to prod localStorage (backend logged "available" 3 launches, ZERO `download_update` invokes ever); scuff = WebView2 backdrop-filter mis-composite class on `.dialog-overlay`/`.dialog-shell`/`.toast`.

**FIXED (`feea28f`, svelte-check 0/0 + CDP DOM+pixel verified):** snooze now 24h `{version,until}` JSON (legacy bare-string self-discards; expiry timer mid-session + on launch; `unsnooze()`/`hasUpdate`/`snoozeActive`) · snooze-proof accent dot on settings gear · backdrop-filter stripped from dialog overlay/shell + toasts (opaque) · dialog says "Remind me tomorrow". `.slideover`+`.tip` still carry blur — same class, NOT update-path, flagged only.

**SHIPPED:** user manually went 0.8.10→0.8.11 (Settings→About, ignores snooze), then **v0.8.12 tagged + CI green** (publish 19:05Z, 4 assets on rift-releases; commits `feea28f`+`1ce83e7`+`905c08c`). Release VALIDATED vpk-idempotency (2nd warm-runner run) + `--no-bundle`. Prod = 0.8.11 (PID-only kills, NEVER by image name).

## cont.92/91 (same day, earlier) — debug sweep + autonomous sweep

cont.92: 18-agent Workflow sweep, 13 findings → 3 confirmed + fixed (`b78f2c5` vpk-install idempotent · `7cc2ce2` stt lock-across-await · `d0821fd` composer enhance race); all pushed, CI green. cont.91: `dirs_home` dedupe · Actions @v6 bump · Settings hero polish · ISSUES prune. Detail: `git log -- docs/HANDOFF.md`.

### RESUME HERE (cont.93)
- **End-to-end pill update test:** user's prod 0.8.11 → pill should show v0.8.12 on next launch (old snooze was for 0.8.11, doesn't gate it). Awaiting user report — if anything scuffs/fails, get the exact surface.
- **0.8.12 onward:** snooze = 24h `{version,until}` JSON; gear dot = snooze-proof affordance; dialog/toasts blur-free. `.slideover` + `.tip` (app.css) still carry backdrop-filter — same WebView2 bug class, not update-path, fix if scuff reports continue.
- **MODEL SWAP half-unblocked:** `claude-fable-5` live-verified. DO NOT wire until pricing + ctx + effort tiers publish. Rift stays on `opus` alias.
- **Perf roadmap (NOT applied):** runner `CARGO_INCREMENTAL=1` (vs sccache); `opt-level=2`; pre-bake gh+vpk; vpk `--noPortable`.
- [carried] drag-reorder verify; `RELEASES_TOKEN` non-ASCII (strip works; re-set cleanly).

## Prior arcs — detail in `git log`
cont.90 v0.8.11 SHIPPED — first real tag-driven release on VM 100 `rift-runner` (3 provisioning fixes: PS5.1, gh CLI, gh-zip layout) + `--no-bundle` perf fix. cont.88/89 Settings + Harness redesigns → in v0.8.11. cont.88 self-hosted runner LIVE (`docs/design/self-hosted-runner.md`): **`RunnerKeepAlive` startup task load-bearing — DON'T delete**. cont.87 v0.8.10. cont.72 v0.7.0 + edit-swarm.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step**; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = **hero + sticky pill-tabs + single-column titled cards** (`.st-block`=card, header band inside; cont.88). Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at ship. **v0.8.11 stands** (shipped 2026-06-09, cont.90). Harness has THREE sub-tabs (Telemetry · Cost · Swarm) — still one workspace, IA unchanged.
