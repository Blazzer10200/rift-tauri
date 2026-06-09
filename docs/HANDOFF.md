# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont. 93) — update-flow root cause FOUND + FIXED

**User report:** update to v0.8.11 "nothing popped up; UI scuffed out, couldn't click." **Root cause:** prod localStorage held `rift.updates.dismissed-version=0.8.11` — pill `×` (flush next to "View") version-permanently silenced it; prod `rift.log` shows "available v0.8.11" on 3 launches but ZERO `download_update` invokes ever on GitHub path. Scuff = WebView2 backdrop-filter mis-composite class (toast blur14 bottom-anchored + `.dialog-overlay`/`.dialog-shell`).

**FIX APPLIED (verified svelte-check 0/0 + CDP DOM+pixels):**
1. `updates.svelte.ts` — snooze now **time-based 24h** (`{version,until}` JSON in same key; legacy bare-string discarded by JSON.parse catch — self-heals). `snoozeTimer` wakes pill on expiry mid-session; re-armed on launch; `unsnooze()` added; `hasUpdate`/`snoozeActive` getters.
2. `Titlebar.svelte` — **snooze-proof accent dot** on settings gear when `hasUpdate` (tooltip says "Update available — vX"). A snoozed update is never invisible.
3. Stripped backdrop-filter (measured WebView2 garbage class): `.dialog-overlay`+`.dialog-shell` (app.css, shell now opaque `--bg-elev-1`), `.toast` (ToastHost). `.slideover`+`.tip` still carry blur — same class, NOT update-path, flagged not fixed.
4. Labels: dialog "Remind me tomorrow", pill × aria "Snooze for a day".

CDP-verified: pill→snooze(pill hides, dot stays, 24h JSON)→unsnooze(pill back)→dialog (overlay backdrop=none, opaque shell, clean shot). Download invoke chain unchanged (proven earlier this session).

**User unblock on prod 0.8.10 NOW:** Settings→About→Check for updates→Download (ignores snooze). Next release (0.8.12) supersedes the stale snooze anyway (version mismatch → pill shows).

**Env state:** tauri dev RUNNING (bg task bdhzwnz3v) + cdp:serve 9223 (b4u1m52mb); prod = 0.8.10 (PID-only kills). v0.8.11 live on rift-releases.

## Session 2026-06-09 (cont. 92) — workflow debug sweep + push

Multi-agent Workflow sweep (18 agents, find→adversarial-verify): 13 raw findings → **3 confirmed, 10 refuted**. All 3 fixed + verified (cargo check green via isolated `CARGO_TARGET_DIR` — dodges dev-lock collision, dev stayed alive; svelte-check 0/0):

- **vpk install idempotent** (`b78f2c5`) — `release.yml` bare `dotnet tool install` FAILS 2nd+ release on persistent runner; Get-Command guard → update|install.
- **stt lock-across-await** (`7cc2ce2`) — whisper model load moved OUTSIDE cache mutex; stop no longer hangs during load. Concurrent starts may double-load (benign).
- **composer enhance race** (`d0821fd`) — `enhanceSeq` token; dismissed preview can't reappear from in-flight stream.

**All 9 commits PUSHED** (`f7ac754..d0821fd`) — runner VM 100 back online, CI check run 27226306670 **GREEN** (1m30s, @v6 actions confirmed live).

### cont.91 (same day, earlier) — autonomous sweep
`dirs_home` dedupe (`4bbc805`) · Actions Node-20 bump (`062bc3b`) · Settings hero polish (`a825cb6`, CDP-verified) · ISSUES prune (`cd9cbf9`) · `claude-fable-5` probe answered live.

### RESUME HERE (cont.92)
- **Update test PENDING** — prod still **0.8.10**; user click on "Update available" pill verifies Velopack download→apply→relaunch.
- **Next release validates:** `--no-bundle` green + timing AND the vpk-idempotency fix (2nd run on warm runner).
- **MODEL SWAP half-unblocked:** `claude-fable-5` live-verified. DO NOT wire until pricing + ctx + effort tiers publish — picker/cost-cockpit metadata would be fabricated. Rift stays on `opus` alias.
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
