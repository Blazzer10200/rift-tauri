# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont.100) — autonomous: Composer C1 + next split briefs

**Three pure-helper extractions shipped** (all no-window-safe; visual extractions C2-C7/B/T parked b/c brief mandates CDP pass per cut and user was gaming): **C1** `composer/helpers.ts` (7 fns + 17 tests, Composer 3197→3130L) · **MessageBubble H0** `bubble/helpers.ts` (17 fns + types + 22 tests — first coverage for `parseTextBlock`/`reconcileSplitHeaders`/`coalesceToolGroups`; 1742→1471L; **`previewOf` found dead** — moved+tested, not re-imported) · **ChatTabsBar H0** `tabsbar/helpers.ts` + portal dedupe (1761→1717L). **Portal discovery:** canonical `$lib/actions/portal.ts` already existed (WebBrowserPage/FilePathMenu) — tabs-bar focus-variant added as `portalFocus`, Composer re-pointed, composer copy removed. Also mapped + briefed both files (`messagebubble-split.md`, `chattabsbar-split.md` — maps spot-verified vs source). Each commit: svelte-check 0/0 · vitest green (final 95/95).

### RESUME HERE (cont.100)
- **User prod = 0.8.12** → still needs ONE manual `Setup.exe`; after that, in-app update should pull latest.
- **Runtime smoke debt TWO releases deep** (0.8.15 + 0.8.16 source-verified only): next dev session run the CDP pass — real turn (stream/tools/thinking), steer, stop, /retry, queue drain, **prompt enhance + title gen + summarize/remint (oneshot moved!)**, Settings config get/set + provider CRUD (config moved!), History list/load/delete, auth pill, update chip. **+ splash (cont.99) + cont.100 extraction surfaces:** composer (mention fuzzy, image paste, drag-drop, effort slider), bubble timeline (step dividers, tool-group collapse, thinking elapsed), tabs-bar popovers (keyboard nav + portalFocus focus behavior).
- Next #20 bite: Composer C2 (AttachmentsRow) — dev running + `c.sh look` per extraction. Decide: `previewOf` (dead in MessageBubble) — wire or drop during B-passes. Parked: SEC-1 live pass · #29 CSP-nonce · CR-UX trust-enum (needs user call) · `.tmp/runner/` setup-scripts fate (flagged to user).

## Prior arcs — detail in `git log` + CHANGELOG
cont.99 seam-reveal boot splash (RiftLogo + letter cascade, fake progress bar deleted, CDP-verified ×2, replay via `sessionStorage.removeItem("rift.splash.seen")`). cont.98 v0.8.16 (#20 backend split COMPLETE R1-R8 — mod.rs 4331→303L hub; CI green, rift-releases latest = v0.8.16). cont.97 v0.8.15 (TS split complete M0-M9 · honest update chip). cont.96 v0.8.14 update-dialog crash root-caused. cont.94 v0.8.13 Fable 5 limited-run (**Jun 22 sunset gate** — self-heals to Sonnet/Opus). cont.90 first tag-driven release on VM 100 `rift-runner`; **`RunnerKeepAlive` startup task load-bearing — DON'T delete.** PID-only kills, NEVER by image name.
[carried] `.slideover`/`.tip` blur (fix on new scuff only) · runner perf roadmap · drag-reorder verify · `RELEASES_TOKEN` re-set.

## CRITICAL DON'T-TOUCH
- **Onboarding gate (cont.55):** `showOnboarding = !onboarding.dismissed && assistant.configLoaded && ((!hasApiKey && !auth?.loggedIn) || !betaNotice.acknowledged)`. Don't drop the betaNotice clause.
- **Accent themeable via `--accent-h`** (app.css `:root` only); tint mixes `in oklab`, never `in oklch`. Status LEDs fixed.
- **Surface tiers:** page `--bg` 0.142 · card `--surface` 0.215 · wells `--bg-inset` 0.178 · field 0.25 · track 0.175.
- **IA: 4 workspaces** (home·chat·harness·settings), nav in titlebar, positional `workspace.order`. Harness = one viewport, no scroll; diagnostics behind "Show details". Left chat rail retired.
- **AssistantPane drop handlers on `.pane` outer only**; `dragDropEnabled:false`; `.shell` fixed inset 0.
- **Blur-reveal** (`Markdown.svelte`): `shownCount` only `$state`, written only by rAF loop.
- **Activity split:** Steps = settled actions (drops `cat==="write"`); Outputs owns writes → Session Diff.
- **Versions lockstep** ×3 + `Cargo.lock` — only at ship. **v0.8.16 stands** (2026-06-09 cont.98).
- **`turn.rs::kill_all_session_children` re-export** (`pub(crate) use turn::…` in assistant/mod.rs) is load-bearing for the Velopack apply — don't "clean up".
