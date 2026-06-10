# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont.100) — autonomous 3-round bg session

C1 + both H0 pure-helper extractions shipped (Composer 3197→3131 · MessageBubble 1742→1471 · ChatTabsBar 1761→1717), portal deduped to canonical `$lib/actions/portal.ts` (+`portalFocus`), both next split briefs written + maps verified, pure-module test net added (`toolCaption`, `state/assistant/helpers`). **vitest 51→116 · svelte-check 0/0 every commit · all pushed (`c34de1f`).** `previewOf` found dead in MessageBubble (moved+tested, not re-imported). Visual work deliberately skipped — user was gaming, no dev window allowed.

### RESUME HERE — NEXT-SESSION RUNBOOK (user pre-authorized 2026-06-09: dev window + full CDP testing + fine-tune fixes. Run it, don't ask.)

1. **Launch:** `scripts/run-dev.bat` (NOT raw tauri dev — CDP port) → `npm run cdp:serve` background → `bash scripts/cdp/c.sh look` to baseline. Dev console = Rust verifier while dev is alive (no manual cargo check).
2. **Runtime smoke — clears 0.8.15+0.8.16+cont.99/100 debt:** splash (replay: `sessionStorage.removeItem("rift.splash.seen")` + reload) · real turn (stream/tools/thinking) · steer · stop · /retry · queue drain · prompt enhance + title gen + summarize/remint (oneshot moved!) · Settings config get/set + provider CRUD (config moved!) · History list/load/delete · auth pill · update chip · cont.100 surfaces: mention fuzzy, image paste, drag-drop, effort slider, bubble timeline (step dividers, tool-group collapse, thinking elapsed), tabs-bar popovers (keyboard nav, portalFocus focus).
3. **Fix what the smoke finds** (fine-tuning authorized). `.slideover`/`.tip` blur scuff: fix only if it shows.
4. **Then resume #20 visual splits, CDP `look` after EACH cut:** Composer C2 (AttachmentsRow) → C3 (QueueRail) per `composer-split.md`; then B1/T1+ if session has room. Hard rules in the briefs (one child per commit, verbatim moves, check 0/0, vitest 116+ green).
5. **User prod = 0.8.12** → still needs ONE manual `Setup.exe`; after that in-app update pulls latest. When a batch is ship-ready: bump ×3 + Cargo.lock → CHANGELOG → tag (CI does the rest).
- Parked: SEC-1 live pass (fold into smoke if convenient) · #29 CSP-nonce (app-wide — own arc) · CR-UX trust-enum + `previewOf` wire-or-drop (need user calls) · `.tmp/runner/` scripts fate.

## Prior arcs — detail in `git log` + CHANGELOG
cont.99 seam-reveal boot splash (CDP-verified ×2). cont.98 v0.8.16 (#20 backend split COMPLETE R1-R8 — mod.rs 4331→303L hub; CI green, rift-releases latest = v0.8.16). cont.97 v0.8.15 (TS split M0-M9 · honest update chip). cont.96 v0.8.14 update-dialog crash root-caused. cont.94 v0.8.13 Fable 5 limited-run (**Jun 22 sunset gate** — self-heals to Sonnet/Opus; boundary now vitest'd). cont.90 first tag-driven release on VM 100 `rift-runner`; **`RunnerKeepAlive` startup task load-bearing — DON'T delete.** PID-only kills, NEVER by image name.
[carried] runner perf roadmap · drag-reorder verify · `RELEASES_TOKEN` re-set.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. Don't drop the betaNotice clause.
- **Accent themeable via `--accent-h`** (app.css `:root` only); tint mixes `in oklab`, never `in oklch`. Status LEDs fixed.
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces** (home·chat·harness·settings), nav in titlebar, positional `workspace.order`. Harness = one viewport, no scroll. Left chat rail retired.
- **AssistantPane drop handlers on `.pane` outer only**; `dragDropEnabled:false`; `.shell` fixed inset 0.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` only `$state`, written only by rAF loop.
- **Activity split:** Steps = settled actions (drops `cat==="write"`); Outputs owns writes → Session Diff.
- **Versions lockstep** ×3 + `Cargo.lock` — only at ship. **v0.8.16 stands.**
- **`turn.rs::kill_all_session_children` re-export** (`pub(crate) use turn::…` in assistant/mod.rs) — load-bearing for Velopack apply.
- **Pure-helper modules + their vitest nets** (`composer/` `bubble/` `tabsbar/` helpers, `$lib/actions/portal.ts`) — don't re-inline during splits.
