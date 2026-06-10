# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-09 (cont.100) — autonomous: Composer C1 + next split briefs

**C1 shipped** (`refactor(#20 C1)`): `composer/helpers.ts` — `fmtClock·fuzzyScore·effortIdxFromX·bytesToBase64·portal·fmtSize·isFileDrag` moved verbatim + 17 vitest cases (fuzzy tiers, slider math, >32KB base64 chunking). Composer 3197→3130L. svelte-check 0/0 · vitest 68/68. **C2-C7 deliberately not attempted** — brief mandates a CDP visual pass per extraction and user was gaming (no dev window allowed). **Next #20 candidates mapped + briefed** (agent maps spot-verified vs source): `docs/design/messagebubble-split.md` (H0 ~17 pure fns incl. `parseTextBlock`/`coalesceToolGroups` — high-value untested parsing — then B1-B6) + `docs/design/chattabsbar-split.md` (H0 hoists `portal` to `$lib/actions/portal.ts` deduping Composer's copy + `menuKeydown`, then T1-T6; TabStrip drag-reorder unit moves LAST, window-dragend lifecycle intact). ISSUES #20 + composer-split follow-on updated. Pushed (splash + C1 + briefs).

### RESUME HERE (cont.100)
- **User prod = 0.8.12** → still needs ONE manual `Setup.exe`; after that, in-app update should pull latest.
- **Runtime smoke debt TWO releases deep** (0.8.15 + 0.8.16 source-verified only): next dev session run the CDP pass — real turn (stream/tools/thinking), steer, stop, /retry, queue drain, **prompt enhance + title gen + summarize/remint (oneshot moved!)**, Settings config get/set + provider CRUD (config moved!), History list/load/delete, auth pill, update chip. **+ splash (cont.99) + C1 helpers in a live composer** (mention fuzzy, image paste, drag-drop, effort slider).
- Next #20 bite: Composer C2 (AttachmentsRow) — dev running + `c.sh look` per extraction. Parked: SEC-1 live pass · #29 CSP-nonce · CR-UX trust-enum (needs user call) · `.tmp/runner/` setup-scripts fate (flagged to user).

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
