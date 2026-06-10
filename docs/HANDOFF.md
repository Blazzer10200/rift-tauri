# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-10 (cont.101) — dev-window runbook executed

Full CDP smoke run + 2 real bugs found & fixed + C2 shipped. All pushed-ready on main (commits `b067407`, `49349bb`), tree clean, dev + CDP servers shut down.

**Bugs fixed (b067407):**
1. **Home boot-empty** — `assistant.init()` only ran on Chat/Settings mount; boot lands on Home → "Open a project to begin" + "0 saved" despite live config. AppShell now inits at mount.
2. **init() double-listen race** — same-flush init() calls both passed the `unlistens.length` guard (no push until after first await) → every tauri listener ×2 → **every stream line applied twice** ("TheThe quick brown fox…"). `initPromise` memo fixes it; `destroy()` resets. 12→6 listeners verified live.
3. CDP `type` helper now flushes a macrotask between input + Enter (stale-textarea was harness artifact, not app bug). Dev-only `window.__assistant` handle added beside `__updates`.

**Smoke results (all CDP-verified):** splash ✓ · real turn (stream/tool steps/thinking elapsed/title gen/cost chip) ✓ · steer ✓ · stop ✓ · /retry ✓ · queue+drain ✓ · enhance ✓ · compact correctly refuses <4 msgs (fail-loud) ✓ · Settings get/set (git-tools toggle round-trip) ✓ · provider CRUD (SmokeTest add→delete) ✓ · History list/load/delete ✓ · auth pill (Max sub, CLI up to date) ✓ · update chip honest "reinstall needed" (dev NotInstalled — correct) ✓ · mention fuzzy ✓ · image paste ✓ · drag-drop (target = `.composer-shell`, NOT `.pane` — pane ondrop is tab-reorder) ✓ · effort slider (Smart→Deep→Smart) ✓ · tabs-bar popovers + portalFocus (focus restores to pill) ✓. `.slideover`/`.tip` blur scuff did NOT show. Test convos deleted from history (77 = user's originals), config restored (trust readonly, workspace remotion-playground untouched).

**C2 shipped (49349bb):** `composer/AttachmentsRow.svelte` — Composer 3131→3048L. svelte-check 0/0 · vitest 116 · CDP pixel-verified.

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
cont.100 C1+H0 extractions, vitest 51→116. cont.99 boot splash. cont.98 v0.8.16 (#20 backend split, mod.rs→303L hub). cont.97 v0.8.15 (TS split M0-M9). cont.94 Fable 5 limited-run (**Jun 22 sunset gate**). cont.90 first tag-driven release; **`RunnerKeepAlive` startup task load-bearing.** PID-only kills, NEVER by image name.

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
