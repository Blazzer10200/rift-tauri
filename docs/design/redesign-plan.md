---
title: Rift front-end redesign — audit + plan
status: Ledger built + committed on design/chat-ledger (8bb71f4f, not pushed/merged); chat-motion diagnosis done (see "Chat motion"); next = motion fixes, then Phase 1 tokens
updated: 2026-10-19
---

# Rift redesign — audit + plan (paused 2026-10-19, resume tomorrow)

Owner brief (2026-10-19): EVOLVE Graphite Ink — not a new look, not a layout
overhaul. Sharpen hierarchy, spacing, type, polish. Pain points: (1) chat
cluttered, (2) home/welcome flat, (3) Settings/AI Health messy + inconsistent,
(4) lacks personality / premium feel. First screen for directions: **Chat**.

Hard constraints (DESIGN.md §8): no stacked atmosphere, no dots/specks, no
diagonal sheen, no traveling shimmer / orbiting light, no 2nd chromatic accent,
no pill-round buttons/inputs, say each fact once, one island level deep.
Personality = material, type, precision — never effects.

## Where things stand

- Done: read AGENTS.md, HANDOFF, DESIGN.md (§1–§10 incl. §8 rejection log),
  `app.css` tokens 124–285, Rift Style README + rejection-log + bundle.css.
  Captured all four screens live via CDP + replayed `/dev/gallery`.
- Screenshots: `docs/design/redesign-audit/` (01 chat welcome, 02 chat
  conversation, 03 gallery expanded turn, 04 gallery tiles, 05 settings,
  06 AI health, 07 home/workspace). Viewport 2048×1104 @ DPR 1.25.
- Design canvas **Rift Chat Directions** (filled 2026-10-19):
  https://claude.ai/artifact/F9MZxDGhBHW4pwwDM89Gin — 5 boards in one row:
  Current (screenshot 02), Current turn opened (screenshot 03), A Ledger,
  B Spine, C Document. Rift Style installed under `project/ds/rift-style/`
  (tokens.json, bundle.css, fonts); shared mock CSS = `project/rift-mock.css`
  (tokens verbatim from app.css + AppShell slab recipe). A/B/C share the
  baseline fixes (C1, C2, C5, C6, C7, C13, hover actions) so only the idea
  differs — sticky note under the row says so. Local source of the canvas
  files: session scratchpad `canvas/project/` (not durable).
- Mock caveat: the Current boards are screenshots scaled to ~0.72× real size;
  A/B/C render at 1:1 CSS px in a 1280px-wide island, so their type reads a
  little larger than Current. Compare structure, not text size.
- Not done: owner pick. No production code touched.
- Contrast script: scratchpad `contrast.mjs` (oklch→WCAG); numbers below.

## Audit — evidence per pain point

### 1. Chat feels cluttered

| # | Finding | Evidence |
|---|---|---|
| C1 | Turn meta is invisible at rest | `.sactivity-quiet` (`↑ 22k`, turn time) = `fg-faint` × `opacity: 0.45`, 10px mono → **1.57:1** on the island (stream.css:423). Hover lifts to 0.9. Violates §8 "don't equate quiet with invisible". |
| C2 | Same timestamp twice per exchange | "Sep 25 · 9:09 PM" under the user bubble (`.user-time`, fg-faint 2.26:1) AND in the turn summary right below. |
| C3 | Turn summary row carries 6 facts | `✓ Changes applied · [5 files] · 5m 15s · $4.09 · ↑ 22k · Sep 25 · 9:09 PM · ›` — three text colors, two fonts, one row. |
| C4 | Summary sits ABOVE the reply | The receipt for the turn is read before the answer; prose is the protagonist but comes second. |
| C5 | Project + branch shown 3× on Chat | sidebar (`rift-tauri / main`), composer ctx row (`rift-tauri / main ●`), footer (`rift-tauri  main ●`). Welcome adds a 4th (`main ● · 446 files · …/rift-tauri`). |
| C6 | Columns don't line up | `.messages` x=605 w=1100; composer x=715 w=880 → composer edges sit 110px inside the text edges. |
| C7 | Uneven vertical rhythm | ~77px between turn 1 and user msg 2; ~200px dead gap before turn 2 (scroll-anchor min-height). |
| C8 | Boxes 3 deep when a turn opens | turn frame (bordered) → work-line tile (bordered) → per-call wells (bordered). §7: inside a tile use `tile-well`, never a 2nd bordered card. (03-gallery-expanded-turn) |
| C9 | Chrome colored by tool kind | `$` prompt amber, `PS>` blue, `>` amber, commands syntax-colored in tile heads. §7: glyph color = status, never kind; amber = "costs money". (04-gallery-tiles) |
| C10 | Status vocabulary drifts | same outcome reads `passed`, `ok`, `651 passed`; failed tile = red border + inset edge + red pill (3 signals, 1 fact). |
| C11 | Copy/Retry always visible | under every message, fg-subtle 11px — persistent noise on every turn. |
| C12 | Work-line header is a sentence | `Read streamModel.ts, …/assistant/stream · Searched "inputPartial" in …/lib/state +1 more · called 1`. |
| C13 | Mono used for chrome meta | timestamps, durations, `$`, pill counts all JetBrains Mono — §3 says mono is for code/paths/IDs. |
| C14 | Turn navigator overlaps column edge | the right-edge minimap ticks sit inside the user-bubble column (x≈1680). |

### 2. Home / welcome is flat

- Welcome (01): small greeting block floating mid-island in ~1700×1000 of empty
  slab; eyebrow "STILL UP" + greeting + facts row + one continue card + input.
  Nothing anchors it; the composer here is a different component/height than
  the in-chat composer.
- Welcome action buttons are pill-round: `.chip-btn` `border-radius: 999px`
  ("Switch folder", "View activity") — §8 bans pill buttons.
- Home/Workspace (07): `5,914` oversized hero numeral (§8 AI-slop tell);
  six project avatars in six hues + 5-hue model-mix bar (second-accent noise);
  amber **gradient wash** card "Summarize recent Claude news" (page-local
  atmosphere — banned; the amber cost line itself is correct); Activity card
  holds a bordered stats grid (2 levels).

### 3. Settings / AI Health messy + inconsistent

- Type scale ignored: SettingsPage.svelte uses **10 font sizes** (9, 10, 10.5,
  11, 11.5, 12, 12.5, 13, 14, 16px) and **12 radii** (2–12px + 999 ×14).
  AiHealthPage.svelte: 11 sizes. App-wide: **388 hard-coded `font-size: Npx`**
  across 44 files; `--fg-faint` used 261× across 49 files.
- Content columns differ per page: Chat text x=605 w=1100 · Settings header
  x=727 vs cards x=765 (38px offset) · Health header x=727 vs body x=591 ·
  Home x=511 w=1230. Four left edges.
- Settings (05): fake macOS window chrome (●●● traffic lights) on "updates
  live" previews — box in card in island = 3 levels; also reads as dots.
  Accent dot next to "Accent color" repeats the selected swatch.
- AI Health (06): plan usage shown **3×** on one screen (status pill
  `Plan 16%`, Plan-limits card, footer 5h/7d bars); left-border "Healthy"
  card (AI-trope + §8); accent-tinted `100% EFFICIENT` spotlight stat tile
  (§8 "no spotlight cards"); red sparkline as decoration (status ≠ decoration);
  Title Case "Claude Usage & Health" vs sentence-case rule; accent used ~8×
  as ghost besides the one solid CTA.
- Sidebar subtitle changes per page: `main` (branch chip) on Chat,
  `Local workspace` on Settings/Health/Home.

### 4. Lacks personality / premium

The material (onyx slab, bevel, seam) is good and shouldn't change. What
reads cheap is imprecision: 4 column edges, 10+ font sizes, invisible meta,
kind-colored chrome, repeated facts. Premium here = **every edge on a grid,
every number tabular, every fact once, type with real hierarchy**.

## Chat directions (to mock next — 2-3 artboards beside "Current")

All three keep the slab, tokens, accent and tile family. They differ in
where the turn's machinery and meta live.

**A — Ledger (quiet receipt).** Reply prose first; the turn receipt moves
BELOW the reply as one hairline-ruled line: `5 files · 5m 15s · $4.09`
in `fg-subtle` Inter tabular-nums (≥4.5:1). Time lives once, on the user
message (hover-reveal at fg-subtle, not 0.45 opacity). Copy/Retry appear on
hover/focus only. Fixes C1–C4, C11, C13. Lowest risk.

**B — Spine (engineered structure).** One 1px `border` hairline runs down
the left of an assistant turn, tying its tool rows to its prose (ruled
structure, the sanctioned texture). Tool runs collapse to ONE row per turn
(`Read 4 · Searched 2 · Edited 3 · Ran 2`), expanding to flat rows on
`tile-well` — never a box in a box. Shell glyphs neutral; only status
colors. Fixes C3, C8–C10, C12. Medium risk (WorkLine/StreamTurn).

**C — Document (editorial column).** One shared column for messages AND
composer (fixes C6), user turns as a quiet full-width `bg-elev-1` band with
`fg-2` text instead of a right-aligned bubble, prose at 14px/1.6, section
rhythm on an 8px grid with fixed turn spacing (fixes C7). Composer docks as
the island footer; project/branch removed from the composer row (C5 — footer
keeps it). Highest visual change; still evolutionary.

Expected recommendation: A + the column/rhythm fixes from C as the baseline,
B as an optional second step. Owner picks.

## Phased plan (each phase = before/after screenshots → owner review gate)

**Phase 0 — Owner picks a Chat direction** from the canvas. Gate: explicit pick.
DONE 2026-10-19: owner picked **A · Ledger** ("reply first, one quiet receipt
line under it"). Phase 3 builds A on top of the shared baseline (C1, C2, C5,
C6, C7, C13, hover-only Copy/Retry). B's flat-rows/no-nesting fixes (C8–C10,
C12) still land in Phase 2 as component hygiene. C's band treatment is
not taken.

**Phase 1 — Tokens.** Add a meta text role (`--fg-meta` ≥4.5:1 on island,
or retire the 0.45 opacity and use `fg-subtle`); a type scale for 10px/11px
meta (`--fs-2xs`?) so pages stop inventing 9.5/10.5/11.5; one page column
token (`--page-col-max`) + one chat column shared by messages and composer;
turn-spacing token. Mirror every change in `design-system/styles.css` +
`_ds_manifest.json` (or `check:tokens` fails) and republish Rift Style.
Accept: `npm run check` + `npx vitest run` green; contrast table recomputed.

**Phase 2 — Shared components.** Tile head/meta/pill vocabulary (one status
word set: `passed`/`failed`/`N failed`; failed = edge OR pill, not both);
neutral shell glyphs; meta row component (one dialect for turn receipt,
user time, card footers); welcome `.chip-btn` → `.btn` radius; island/tile
nesting audit (no bordered wells inside tiles). Accept: `/dev/gallery`
replay + `/dev/ui` screenshots, 0 console errors, check:ui baseline only
goes down.

**Phase 3 — Chat.** Apply the picked direction: receipt placement, single
timestamp, hover actions, aligned column, fixed rhythm, de-dup project/branch
(C5). Accept: before/after of 02 + 03 + 04 at the same viewport; every C#
finding marked fixed or deliberately kept.

**Phase 4 — Home / welcome.** Welcome anchored (greeting + composer as one
unit, same composer as chat), no pill buttons, one facts line. Workspace hub:
drop hero numeral to the type scale, neutral project avatars (initials on
elev-2, accent only for the active one), remove the amber wash (keep the
cost line). Accept: before/after 01 + 07.

**Phase 5 — Settings / AI Health.** One page shell (header + body on the
same left edge and width), replace px literals with scale tokens, drop fake
window chrome on previews, plan usage once (Plan-limits card owns it; status
pill links there), replace left-border card + spotlight tile with plain tiles,
sentence case. Accept: before/after 05 + 06; font-size literal count in
those two files down to ~0; radius values only from the ladder.

Each phase: own branch, own commit(s), no push/merge without asking.

## Chat motion — measured 2026-10-19 (owner: "a little laggy, could be better")

Branch `design/chat-ledger`, Ledger committed as `8bb71f4f` (checkpoint).
Motion work = separate commits so each reverts alone.

**Measured (dev build, /dev/gallery replay, rAF sampler 70s):** 12,569
frames, p50/p95/p99 = 5.6/5.6/5.7ms (≈180Hz display), 2 frames >20ms,
1 long task (86ms). → NOT dropped frames. The lag is DELIBERATE PACING:
- Word reveal (`Markdown.svelte:169-172`): `WORD_MS` 42ms/word,
  `CATCHUP_LAG` ≈520ms of words before drain speeds up, 500ms blur-in per
  word → up to ~0.5s between a token landing and it being readable.
- Scroll follow (`AssistantPane.svelte:169-188` `pinToBottom`): rAF lerp
  closes 20%/frame → view trails new text ~170ms (≈90% of gap).
- Stacked: up to ~0.7s token → settled view.
- Gap: the gallery scrolls its own `.stage`, so `pinToBottom` was NOT
  exercised. Real numbers need one short real turn (cheap model, ~80 words;
  billed — ask owner first) with the same sampler on the pane.

**Do now (no visible change / explicit rule):**
1. `Markdown.svelte:688-691` `.md-w-hold` — drop `filter: blur(3px)` (span
   is opacity 0; blur paints nothing; up to hundreds of filters per burst).
2. `stream.css:63-73` `railGlint` + `.sturn-body.railed.live::after` —
   remove: traveling light along a fixed rail (§8 "liveness on chrome is
   stationary"). Live rail keeps its accent tint. Tell owner in one line.
3. `AssistantPane.svelte:1384` `.jump-latest` backdrop-filter → `.hud-float`
   recipe (surface 94% + border-strong); §10 bans backdrop-filter (WebView2).
4. Check whether `.bubble[data-streaming="true"] .tl-node`
   (`MessageBubble.svelte:817-823`, blur node-enter) is ever live — StreamTurn
   owns live rendering; likely dead → delete per "delete write-only state".
Then rerun the replay sampler for before/after.

**Owner's call (motion = most-rejected category) — offer as 3 single-value
toggles, one at a time, owner FEELS each live in the dev app:**
- Reveal: keep blur-in / opacity + 2px rise (compositor-only).
- Pacing: `WORD_MS` 42→28, `CATCHUP_LAG` 520→300ms.
- Follow: lerp 0.2→0.35, or instant pin while streaming (terminal-like).
Never ship a pacing/reveal change speculatively. Say "measured in dev" on
every number (release build is faster).

## Open owner calls

- Footer still repeats the live tool caption under the in-flow live row
  (kept on purpose 9/25) — revisit in Phase 3?
- `design-system/` static cards overlap Rift Style — retire? (flag only,
  nothing deleted.)
- Model-mix bar on Home is legit categorical data viz — keep multi-hue there?

## Resume steps (tomorrow)

1. `npm run cdp:dev` (background) + `npm run cdp:serve` (background).
2. Fill the canvas: `project/canvas.json` + `Current.dc.html` (upload
   `redesign-audit/02-chat-conversation.jpeg` as an asset, crop to main
   island) + `A-Ledger.dc.html`, `B-Spine.dc.html`, `C-Document.dc.html`,
   same conversation content in each, ~1280×880, one-line rationale in each
   board's `title`. Install Rift Style under `project/ds/rift-style/`.
3. Show owner the canvas link + this audit; get the pick (Phase 0).
4. Reap: `bash scripts/cdp/c.sh reap --all` then `c.sh shutdown`.
