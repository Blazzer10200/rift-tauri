# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-05 (cont. 58) — Issue cleanup + tracker reorg (unshipped)

Cleared the last low-hanging open bug + reorganized the tracker. `npm run check` 0/0. WIP checkpoints — ride next `/git-ship`.
- **#36 fixed** (`d551856`): Settings scroll-spy. [SettingsPage.svelte:45-49](../src/lib/components/settings/SettingsPage.svelte) `onScroll()` bottom-detects (`scrollTop+clientHeight >= scrollHeight-2`) → spies the last `ST_SECTIONS` entry, so the bottom (incl. About nav → `jump()`) lights "About" not "Speech".
- **ISSUES.md reorganized** (`89f65dc`): legend → index table → blocks grouped status→tier, numeric within group. Durable IDs + all facts preserved.
- **Other docs left as-is:** DEVELOPING + IDEAS already clean; CHANGELOG is `/git-ship`-only.

### RESUME HERE (cont.58)
- All cont.52–58 UNSHIPPED → next `/git-ship` (3-file lockstep + `Cargo.lock`; CHANGELOG/bump deferred).
- ISSUES **#31–36** fixed in-tree (delete on ship). Tracker reorganized + accurate.
- **Open work is all blocked / needs-you / live-verify** (no quick code wins): #21 test harness (T1) · #30 Update-UI redesign (taste — your eye/CDP) · #4/#20/#17 strategic · #29 Tailwind-blocked · CR-UX trust-enum (sign-off).
- Pending USER live-verify: steer mid-turn on a tool turn · permission Allow/Deny bar · v0.5.0 auto-update on a real machine · beta onboarding on a fresh tester install.

---

## Prior unshipped (detail in git log, all ride next ship)
- **cont.57 Model-picker capability accuracy:** capability matrix on `ModelOpt` (`Composer.svelte`) drives every gate; per-model `effortStops` + `$effect` auto-clamp (Opus→Ultracode/xhigh, Sonnet→Deep/high, Haiku→none); amber Ultracode awareness caption; fast-mode hidden behind `FAST_MODE_WIRED=false` (→#31). CDP-verified live.
- **cont.56 Harness polish:** shared motion tokens (`app.css`); idle-gated spark + LEDs unified to 2s; staggered `.bento` cell entrance (reduced-motion-aware); reliability "all clear" collapse; calm "Awaiting first turn" hero.
- **cont.55:** Beta-notice onboarding step (`OnboardingFlow` step 4 + `betaNotice.svelte.ts` ack). See CRITICAL onboarding-gate note.
- **cont.52–54:** `Markdown.svelte` streaming reveal (52); Harness dead-wait split + `.tl-dead` (53); ISSUES #32–35 + Harness KPI-rail no-scroll redesign (54).

---

## Shipped + prior arcs — detail in `git log`
- **v0.5.0** (2026-06-04, cont.51, `62dae27`): Velopack stable to `rift-releases`. **Pending live-confirm:** 2nd auto-update proof point (v0.4.48 → apply-on-exit → relaunch) on a real machine. Older arcs in `git log`.
- **Open carry-over:** `check.yml` per-push email spam; prod app now ALSO `rift-tauri.exe` → revisit "never blanket-kill rift" rule.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. `configLoaded` gates timing (never flashes pre-probe). The `|| !betaNotice.acknowledged` clause makes the flow show for authed users too so everyone hits the **final beta-notice step** before working; `finishOnboarding()` sets both flags. Don't drop that clause or the beta ack is bypassed.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code accent hue. Status LEDs (`--ok/warn/danger/info`) stay fixed. **Tint mixes use `in oklab`, not `in oklch`** (oklch wraps warm hues to purple).
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · raised inputs `--field` 0.25 · seg track `--track` 0.175. Don't reintroduce near-black wells.
- **IA: 4 workspaces** — home·1 chat·2 **harness·3** settings·4. Nav in **titlebar**; switch via `workspace.setActive`/Ctrl+1-4 (positional `workspace.order`, NOT `kbd`). Settings = sidebar + 5 sections; Harness = single-page bento (NO sidebar). **Left chat rail retired** — history in History drawer only.
- **Harness fits ONE viewport — no scroll (cont.54).** Diagnostics (reliability/session-details/tools-granted/live-stream) live behind the **"Show details"** toggle — do NOT promote them into the always-visible grid. KPI rail is the single source for cost/turns/tools/tok-s/cache/ttfp.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` is the ONLY `$state`, written ONLY by the rAF loop — never inside a derived.
- **Activity panel split:** Steps = settled ACTIONS only (`logSteps` drops `cat==="write"`); Outputs owns writes/edits → Session Diff (`assistant.ui.diffOpen/diffTarget`).
- **Versions lockstep** `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`. **v0.5.0 stands** (shipped 2026-06-04, cont.51).
