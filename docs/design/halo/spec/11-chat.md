---
title: "Spec 11 — Chat stream + pane (P4b)"
status: draft
updated: 2026-10-19
phase: P4b
depends_on: [00-decisions, 01-foundation, 02-kit, 03-motion]
---

# Spec 11 — Chat stream + pane (P4b)

## 0. Summary

- P4b moves the live chat (StreamTurn, its blocks, the pane and the user bubble) onto Halo tokens and kit parts. The Ledger IA is unchanged (D-21).
- The end-of-turn swap becomes a same-node fold (D-12): one keyed `{#each}` with zones as attributes, and the activity unmounts only after it has collapsed.
- Prose, tiles and the receipt share one left edge, and the turn spine moves into the gutter.
- Tiles become the Rift dense tile with one signal per state, and Badges replace pills.
- Code uses the `rift-halo` Shiki theme (D-18); diff and ANSI colours follow D-03.
- The column narrows to `--chat-col` (760), the `max_tokens` and refusal notices join the live tree as a Banner, and the legacy MessageBubble branch stays frozen (#6).

## 1. Scope + files

Line numbers are at `22727d24` (`design/chat-ledger`). P1, P2 and P3 land first and shift lines, so each commit anchors its change by selector or symbol and treats the line number as a hint. Every row says what P2 already did, so this spec does not specify it twice.

### 1.1 Prerequisites (must be merged on `design/halo` before C01)

| From | Item | Consumed by |
|---|---|---|
| P1 (spec 01) | Halo tokens and aliases (`--ok`→accent, `--fg-*`→ink rungs), `--t-*-size/-line`, density blocks, `--chat-col` (defined, unused), D-34 `Markdown` literal → token | everything |
| P2 (spec 03) | `motion.css` (`.rise-stream`, `.live-dot`, `.shim`, `.halo-spin`, `.acc-panel`, `.chev`), `transitions.ts` (`accordion`, `rise`, `pop`, `dockWidth`), `rm()`, `MS`, `settleGuard` (M27), M06 `--dur-stream`, M14 LiveDot, M15 one shim, M22, M28 `.sturn[data-live]`, M29 no `code-rise`, M33 fills, M35 `.wline` width, M36, M37 zero `backdrop-filter`, M38 one accordion + `.chev` | C03-C06, C14-C30 |
| P2 tail (spec 03 §6) | **B1-B7 Track B**, landing at the start of P4b: `reveal.ts`, `follow.ts`, carry, frame-time follow, un-latch, jumps through the engine, rail rAF, fake stream + lab | C19, C31, verification |
| P3 (spec 02) | Badge (K13), Kbd (K14), LiveDot/Spinner (K15), Divider (K16), Banner (K19), Button (K10), IconButton (K11), Collapse (K30), RollingNumber (K29), tooltip singleton (K31) | C05-C13, C20-C30 |

If spec 02's K35/K35b (`reveal.ts`, `follow.ts` with a `carry`/`followK` API) landed in P3, B1 becomes a reconcile commit that keeps spec 03's `stepReveal`/`followStep` API and deletes the P3 duplicates. See A-11-01.

### 1.2 Files this spec edits

| File | Lines | P4b change | Already done upstream |
|---|---|---|---|
| `src/lib/components/assistant/stream/StreamTurn.svelte` | 554 | same-node fold (`:412-493`), keys (`:356,418,427,487`), latch (`:123-128`), receipt (`:433-492`), footer (`:495-551`), Banner, copy feedback (`:150-160`) | M28 `data-live`, M38 accordion + `.chev` at `:450,474` |
| `src/lib/components/assistant/stream/groupKeys.ts` (+`.test.ts`) | new | stable group and seg keys | — |
| `src/lib/styles/stream.css` | 555 | spine `:51-64`, `.sledger` `:386-445`, `.wline` `:260-326`, `.wbatch` `:345-370`, `.splan` `:447-477`, `.sxplan`/`.sxa-btn` `:479-519`, `.sweb` `:524-537`, `.ssteer`, fold rules (new block) | M06, M14, M15, M28, M33, M35, M38, M42 |
| `src/app.css` | 780 | `.tile` block `:512-606`; tile aliases in the P1 alias block | P1 aliases; M37 if it covered `:598-603` |
| `src/lib/components/assistant/Markdown.svelte` | 1477 | `.md` inset `:678`, chat size, open-fence mono, code chrome `:1100-1140`, prose `:895-1069,1337-1384,1464-1476`, diff `:1302-1320`, 📋 `:482`, copy `:346` | B1 (reveal), M10, M29, M37 `:1212-1213`, D-34 literal (P1) |
| `src/lib/state/highlighter.svelte.ts` | 152 | theme import `:58`, `:135`; escape a literal NUL at `:126` | — |
| `src/lib/state/shiki/rift-halo.ts` | new | the D-18 theme | — |
| `src/lib/components/assistant/stream/StreamThinking.svelte` | 27 | tile on tokens | — |
| `src/lib/components/assistant/stream/WorkLine.svelte` | 113 | hairline-rail rows, LiveDot, Badge, `.chev` | M35 width, M38 |
| `src/lib/components/assistant/stream/WriteBatch.svelte` | 133 | Badge pills, dot | M38 `:89,123-131` |
| `src/lib/components/assistant/stream/StreamShell.svelte` | 299 | terminal surface, Badge, LiveDot | M12, M19, M38 `:211` |
| `src/lib/components/assistant/stream/OutputBlock.svelte` | 300 | ANSI `:221-228`, per-line stagger `:119-142` removed | — |
| `src/lib/components/assistant/stream/BlockHeader.svelte` | 118 | real buttons (`:59-88`), Badge, `ChevronDown` | M12 `:108` |
| `src/lib/components/assistant/stream/StreamAgent.svelte` | 328 | tile states, Badge | M14, M28 `:204` |
| `src/lib/components/assistant/stream/StreamAskUser.svelte` | 427 | needs-you state, option rows | M12, M14, M28 `:324` |
| `src/lib/components/assistant/stream/StreamExitPlan.svelte` | 180 | `.sxa-btn` → Button | — |
| `src/lib/components/assistant/stream/StreamPlan.svelte` / `StreamWeb.svelte` / `StreamResult.svelte` | 49 / 32 / 55 | tile on tokens, Badge | M33, M38 `StreamResult:51` |
| `src/lib/components/assistant/stream/ReadResult.svelte` / `GrepResult.svelte` | 95 / 139 | `rift-halo` code, match mark | — |
| `src/lib/components/assistant/stream/ActivityHud.svelte` / `PlanHud.svelte` | 459 / 329 | opaque `bg-200`, LiveDot, Badge | M14, M33, M37 |
| `src/lib/components/assistant/stream/AnimatedCount.svelte` | 113 | footer counters → RollingNumber | — |
| `src/lib/components/assistant/ToolChip.svelte` / `EditDiff.svelte` / `PermissionBar.svelte` | 773 / 682 / 147 | shared parts, both trees (D-21) | M11, M12, M38 |
| `src/lib/components/assistant/AssistantPane.svelte` | 1563 | `--chat-col` consumers, jump pill, exchange rail, alerts `:754-800`, pane head, empty card, drop zone, split mount | B3-B6, M27, M33, M35, M36 |
| `src/lib/components/assistant/AssistantPage.svelte` | 450 | split dividers `:401-449` | M11 `dockWidth` |
| `src/lib/components/assistant/MessageBubble.svelte` | 1410 | **user branch and system rows only**: `:1208-1223`, `:1238-1246`, `:1251-1280`; system rows `:277-280` | M11 `:6`, M22, M28 `:626` |
| `src/lib/components/assistant/bubble/BoundaryBlock.svelte` / `ModelSwitchBlock.svelte` | 149 / 76 | labelled Divider rows | M37 |
| `src/lib/components/assistant/AssistantWelcome.svelte` | 716 | cards on tokens, column | M16, M38 `:544,557` |
| `src/lib/components/assistant/FilePathMenu.svelte` | 97 | floating recipe | — |
| `src/lib/components/assistant/Composer.svelte` | 2604 | **one line**, `:1849` column width (C07 only; flagged, A-11-05) | — |
| `src/lib/dev/galleryFixtures.ts`, `src/routes/dev/gallery/+page.svelte` | 199 / 166 | P4b state fixtures | B7 |

### 1.3 Out of scope

- The MessageBubble assistant branch (`:283-561` minus the user rows), its `.steer-note`/`.unknown-note` (`:1100-1155`), `.stop-notice` (`:524-542`, `:585+`), `TurnSummary`, and the Stream-view toggle. They are frozen (D-21), and deleting them is shortlist #6. They do pick up shared-part and shared-token changes (`.tile` aliases, ToolChip, EditDiff, BlockHeader). That is accepted, as it is for P1.
- The composer (P4c). Only `Composer.svelte:1849` changes, in C07.
- `streamModel.ts` logic (its test file has 1,011 lines; D-12). Only its CSS-facing class names are read.
- Pacing numbers (shortlist #3, G1/G2 on `design/halo-gate-3`).
- The paid real turn (shortlist #7).

## 2. Target design

### 2.1 Tokens used (resolved values, Halo blue, after P1)

| Role | Token | Value |
|---|---|---|
| Pane canvas / terminal | `--bg-000` | #08090c |
| Tile, code block, receipt hover | `--bg-100` | #0e1014 |
| User bubble, HUD, steer, Kbd | `--bg-200` | #14171c |
| Neutral Badge | `--bg-300` | #1b1f26 |
| Hairline, tile border, spine | `--line-100` | #1f242c |
| Tile hover, Kbd border, floats | `--line-200` | #2c323c |
| Control border | `--line-300` | #5f6774 |
| Prose, titles | `--ink` | #eceff3 |
| Secondary text, icons | `--ink-muted` | #9aa2af |
| Meta, timestamps, receipt facts | `--ink-faint` | #7d8592 |
| Accent, ok, live | `--accent` / `--accent-soft` | #6aa8ff / 14% |
| Info, plan mode, keywords | `--info` | #9cc6ff |
| Strings, ANSI magenta | `--chart-2` | #b79cff |
| Warn | `--warn` | #ff9f5a |
| Danger | `--danger` | #ff6b6b |
| Radii | `--radius-xs/-sm/-md/-lg/-pill` | 6 / 8 / 12 / 16 / 999 |
| Motion | `--dur-fast` / `--dur-base` / `--dur-stream` / `--ease-out` | 150 / 240 (pinned, #3) / 240 / (.2,.8,.2,1) |

Type (P1 names; spec 02's `--t-x`/`-lh` shorthand is not used): chat prose `--t-chat-size/-line`; tile titles, receipt title, WorkLine verbs `--t-small-size/-line` 13/18; meta, receipt facts, Badge `--t-caption-size/-line` 12/16; Kbd and the code-block language label `--t-label-size/-line` 11/14 mono; headings in prose `--t-heading-size/-line` 16/22 600 (h1 `--t-subtitle-size` 18/24).

### 2.2 Per density tier

| Var / measure | compact | regular | comfy | Where |
|---|---|---|---|---|
| `--t-chat-size` / `--t-chat-line` | 14 / 22 | 15 / 24 | 16 / 26 | `.md` in `.sturn`, user bubble |
| `--tile-head-h` | 32 | 36 | 40 | tile head, code-block head, receipt row min-height |
| `--list-row-h` | 32 | 36 | 40 | WorkLine rows (D-21 "32px rows" = compact) |
| `--control-dense-h` | 28 | 32 | 36 | ghost Buttons in the receipt, copy IconButtons, jump pill |
| `--badge-h` | 20 | 22 | 24 | every Badge |
| `--card-pad` | 16 | 20 | 24 | empty-pane card |
| tile padding | 0 12 (head) · 8 12 (body) | same | same | D-21 fixes 8×12 |
| `--stream-gap` / `-turn` / `-between-turns` | 10 / 13 / 18 | 12 / 16 / 22 | 15 / 19 / 27 | unchanged (`stream.css:16-23`) |
| `--chat-col` | min(760, 100%−48) | same | same | D-06 is column-wide, not per tier |

Motion durations and distances are the same in every tier (spec 03 §2.1).

### 2.3 Geometry

```
pane (.stream, bg-000) ─────────────────────────────────────────────────────────
│ ≥24px │◀──────────── --chat-col = min(760px, 100% − 48px) ────────────▶│ ≥24px│
│       │                                                                 │      │
│   ┊   │ user bubble ─────────────────────▶ right-aligned, max min(560,80%)    │
│   ┊   │                                                                 │  ▪   │ ← exchange
│   ┊ 12│ prose starts here (no inset; .md padding-left 0)                │  ▪   │   rail ticks
│   │◀─▶│ ┌ tile ───────────────────────────────┐                         │  ▬   │   (right gutter,
│ spine │ │ head 32 · pad 0 12 · r12 · bg-100   │                         │  ▪   │    centre −32px)
│ 1px   │ └─────────────────────────────────────┘                         │      │
│line-100 receipt ──── same left edge as prose                            │      │
│       │                                                 [↓ Latest] ◀── pill: right edge of col
└──────────────────────────────────────────────────────────────────────────────────
```

- **Turn spine** (`.sturn-body.railed`, today `stream.css:51-64`): a 1px `--line-100` line at `left: -12px`, drawn per activity item (`.sfold[data-zone="activity"]::before`, `top: 0; bottom: 0`), not as one gradient down the turn. Live tint: the last live activity item's segment is `color-mix(in srgb, var(--accent) 40%, var(--line-100))`. `padding-left: 13px` on `.railed` is removed, so text never shifts when a turn gains or loses a spine.
- **No prose inset.** `Markdown.svelte:678` `.md { padding-left: 10px }` → `0`, and `.sledger` `margin-left: 10px` → `0` (`stream.css:393`). Prose, tiles, receipt rule and receipt glyph share one edge.
- **Exchange rail** (pane, `AssistantPane.svelte:825-846`, `:1409-1435`): `right: max(8px, calc((100% - var(--chat-col)) / 2 - 32px))`, so it is centred in the right gutter. Ticks are 2px × 10px `--line-200`; the active tick is `--accent` with `scaleX(2)` (M35 made it a transform).
- **Jump pill** (`:818-823`, `:1380-1407`): `right: max(24px, calc((100% - var(--chat-col)) / 2))`, `bottom: 16px`, Button `secondary` size `sm` (28/32/36), label "Latest" + `ArrowDown` 16. It is a float: `bg-100`, `line-200`, `shadow-pop`.

### 2.4 Turn lifecycle wireframes (D-12 same-node fold)

```
(a) LIVE  .sturn[data-live][data-phase=live]
  ┊ ▸ Thought for 4s                       (tile, compact)      zone=activity
  ┊ Let me look at the store first.        (say, Markdown)      zone=activity
  ┊ ┌ Read 3 files · 0.4s ───────── ⌄ ┐    (WorkLine chip)      zone=activity
  ┊ └────────────────────────────────┘
    The fix is in `follow.ts` …▍           (say, Markdown live) zone=answer
    ● Writing follow.ts · 12s · 1,204 tokens   (footer: LiveDot · .shim verb · RollingNumber)

(b) FOLDING  [data-phase=folding]  0 → 290ms (MS.base + 50)
    activity .sfold rows: grid-template-rows 1fr→0fr, opacity 1→0, 240ms ease-out
    footer: out:accordion 240ms (carrier B)
    The fix is in `follow.ts` …            (SAME Markdown node; reveal keeps draining)
    ───────────────────────────────────    receipt fades in: opacity 0→1, 150ms, +60ms delay
    ✓ Changes applied  2 files · 41s · $0.08                 ⌄      Copy  Retry

(c) SETTLED  [data-phase=settled]   (history mounts directly here, no motion)
    The fix is in `follow.ts` …            (answer zone only; activity unmounted)
    ───────────────────────────────────    Divider, line-100
    ✓ Changes applied  2 files · 41s · $0.08   1.2K tokens out   ⌄   Copy  Retry
                                               └ hover/focus-within ┘     └ hover/focus-within ┘

(d) SETTLED + receipt open   (transition:accordion, carrier B, A-4)
    …answer…
    ───────────────────────────────────
    ✓ Changes applied  2 files · 41s · $0.08                 ⌃      Copy  Retry
    ┊ 1 terminal result                               [Expand output]   (only if shells > 0)
    ┊ ▸ Thought for 4s
    ┊ Let me look at the store first.
    ┊ ┌ Read 3 files ─────────── ⌄ ┐
```

Receipt row (D-21): outcome icon 16 (`Check` in `--accent`; `TriangleAlert` in `--danger` for failed; `ListChecks` in `--info` for planned; `Sparkles` in `--ink-muted` for "Reasoning"/"Work completed") + title `--t-small-size` 500 `--ink` + facts in sans `--t-caption-size` `tabular-nums` `--ink-faint`, joined by `·` pips. Copy and Retry are Button `ghost` `sm`; they appear on `.sturn:hover` and `:focus-within` (opacity 0→1, `--dur-fast`). The tokens fact uses `ArrowUp` 12 + "1.2K tokens out" (no `↑` glyph, D-32) and keeps its hover-only rule. The row is `min-height: var(--tile-head-h)`. The Divider sits 8px under the answer and the row 6px under the Divider.

### 2.5 Tile (D-21, one family)

```
┌───────────────────────────────────────────────────────────┐  bg-100 · 1px line-100 (hover line-200, 240ms) · r12 (--radius-md)
│ [icon16] Title (small 13/18 ink)   meta caption · [Badge] ⌄ │  head: min-height --tile-head-h · padding 0 12 · gap 8
├───────────────────────────────────────────────────────────┤  (no divider line; body starts 0px under head)
│ body: padding 8 12 · small text                           │  body sits in Collapse (carrier A) or accordion (B)
└───────────────────────────────────────────────────────────┘
```

| State | Signal (exactly one) | Removed |
|---|---|---|
| live | LiveDot (8px, accent ping) + visible label (the verb) in the head | accent-tinted border `--tile-border-live`, box-shadow pulse |
| done | outcome icon 16 `--ink-muted` (no colour) | — |
| failed | `TriangleAlert` 16 `--danger` + Badge `danger` "Failed" | red border `--tile-border-bad`, 2px inset edge (`app.css:535-543`) |
| needs-you | 1px border `color-mix(in srgb, var(--accent) 32%, var(--line-100))` + Badge `accent` "Needs you" | 2px edge, accent fill |
| partial failure (WorkLine) | Badge `danger` "N failed" on the head; the row says "failed" in `--danger` | — |

Badges replace `.tile-pill`: new file = `accent`; edit, agent, poll = `neutral`; deleted = `danger`; counts (`12`) = `neutral` `mono`. The chevron is `ChevronDown` 16 `--ink-faint` with `.chev`; open is `rotate(180deg)` (M38). There is no lift and no shadow.

### 2.6 WorkLine (ConsoleX hairline rail, D-21; chip rule, D-12)

```
collapsed (fit-content chip, max 100%)       open (snaps to 100%, no width tween — M35)
┌ [Search16] Searched 4 patterns · 3 · ⌄ ┐   ┌ [Search16] Searched 4 patterns   [3] 1.2s ⌃ ───────┐
└───────────────────────────────────────┘   │ │ Searched  src/lib/motion   → 12 matches        │ 32px row
                                            │ │ Read      reveal.ts        → 214 lines         │ --list-row-h
                                            │ │ ● Reading follow.ts                  3s        │ running row: LiveDot + time
                                            └─┴──────────────────────────────────────────────────┘
                                              ↑ 1px line-100 rule at left 20px (under the head icon)
```

- Row: `height: var(--list-row-h)`; icon 16 `--ink-muted` (per tool kind, `WorkLine.svelte:19-31` map); verb sans `--t-small-size` `--ink-muted`; target mono `--t-caption-size` `--ink` (dir part `--ink-faint`); result meta `--ink-faint` after the target (the `→` stays; it is an arrow, not a status glyph); time only on the running row (`t.status === "pending"`). Finished rows drop `.wr-dur`.
- The single-active compact line (`WorkLine.svelte:55-60`, `.wline-active`) becomes a tile head with LiveDot + `.shim`-free verb (M15 leaves one shim, on the footer) + target mono.
- Row bodies (`.wr-out`) open through Collapse (carrier A) and sit on `--bg-000` with `--radius-sm` when they are terminal output; Read and Grep bodies sit on the tile surface.

### 2.7 User bubble and system rows (D-21)

```
                                  ┌──────────────────────────────────┐  bg-200 · 1px line-100
                                  │ Fix the end-of-turn blink.       │  r 16 16 4 16 · pad 10 14
                                  │ It happens on every turn.        │  --t-chat-size/-line · ink
                                  └──────────────────────────────────┘  max-width min(560px, 80%)
                                              Copy (hover/focus)  3:41 PM   caption · ink-faint
────────────────────────── CONTEXT COMPACTED · 41 → 12 messages ──────────────────────────  Divider label (K16)
────────────────────────── SWITCHED TO OPUS · HIGH EFFORT ─────────────────────────────────
```

No gradient, inset or shadow. Image thumbs (`:1251-1280`): `--radius-md`, 1px `--line-100`, max 320px. The timestamp is `--t-caption-size` `--ink-faint` `tabular-nums` at rest; Copy is Button `ghost` `sm`, shown on hover and focus-within. System rows: BoundaryBlock and ModelSwitchBlock render a labelled Divider (`label` prop, mono 11 caps `.12em` `--ink-faint`, `--line-100` hairlines). Their existing detail popovers and buttons keep working; they move to the right end of the row as IconButton `sm`.

### 2.8 Code block (D-18) and prose

```
┌ ts ──────────────────────────────────────────── [Copy] ┐  head: --tile-head-h · label mono 11 caps ink-faint
│ export function followStep(delta: number) {           │  surface bg-100 · 1px line-100 · r12
│   return delta * (1 - Math.exp(-dt / tau));           │  body: mono --t-small-size/--t-small-line (13/18) · pad 12
│ }                                                     │  rift-halo theme
└───────────────────────────────────────────────────────┘
┌ bash ─────────────────────────────────────────── [Copy] ┐  terminal fence (bash|sh|shell|console|powershell|ps1|cmd)
│ $ npm test                                             │  surface bg-000 · monochrome ink (D-18) · prompt ink-muted
└────────────────────────────────────────────────────────┘
```

`rift-halo` theme: base and identifiers `--ink` #eceff3; keywords, storage, control #9cc6ff; strings, numbers, constants #b79cff; comments `--ink-faint` #7d8592, not italic; punctuation and operators `--ink-muted` #9aa2af; invalid `--danger` #ff6b6b; background `transparent` (the surface is the block). No accent, no green. Copy: IconButton `sm` "Copy"; on success it shows `Check` + "Copied" in `--accent` for 1.8s; on failure `X` + "Copy failed" in `--danger` for 1.8s (today the failure is swallowed at `Markdown.svelte:346`). The tall-block fade (`.cm-fade`, `:1201-1215`) is a gradient to `--bg-100` with no `backdrop-filter` (M37).

Prose (inside `.sturn .md`):
- Headings: no accent bars (`:909-922` removed). h1 `--t-subtitle-size/-line` 600, h2 `--t-heading-size/-line` 600, h3 `--t-small-size/-line` 600 `--ink`. Margin-top 20 (h1/h2) or 16 (h3), bottom 8.
- Ordered-list markers: `--ink-faint` `tabular-nums` (not accent, `:895`). Bullets: 5px `--ink-faint` dot.
- Blockquote (`:945`): 2px `--line-200` left rule, padding-left 12, `--ink-muted`, no fill.
- Tables (`:972-1006`): no outer frame fill; header row `--t-caption-size` 500 `--ink-muted`, bottom 1px `--line-200`; body rows 1px `--line-100` separators; cells padding 6 12; numbers `tabular-nums`; horizontal scroll inside the wrapper.
- Alerts (`:1010-1069`, GitHub `> [!NOTE]` etc.): the Banner look in CSS (HTML string, so no component): tone background `color-mix(in srgb, var(--tone) 8%, var(--bg-100))`, border `color-mix(in srgb, var(--tone) 32%, var(--line-100))`, `--radius-md`, padding 12 16. NOTE/TIP → `--info`/`--accent`, IMPORTANT → `--accent`, WARNING → `--warn`, CAUTION → `--danger`.
- `kbd` (`:1337`): mirrors K14 in CSS: min-width 20, height 20, padding 0 6, `--radius-xs`, 1px `--line-200`, `--bg-200`, `--ink-muted`, 500 11px mono.
- `details` (`:1354-1384`): summary row `--t-small-size` 500, chevron-down rotating 180° via `.chev`-equivalent CSS on `[open]` (native `<details>` keeps its instant open; no grid animation).
- Inline code: `--bg-200`, 1px `--line-100`, `--radius-xs`, padding 1 5, mono 0.92em `--ink`.
- File paths (`.md-path`, `:1464-1476`): `--ink` with a 1px `--line-200` underline, `--accent` on hover. The glow is removed.
- The "Sent to Tasks" marker (`:482`): the 📋 emoji is replaced by a CSS icon (`.tasks-migrated-icon` gets `mask-image` of the lucide `ListChecks` path as a data-URI, 14px, `background: currentColor`), because the marker is an HTML string and the DOMPurify allow-list stays unchanged (D-18).

### 2.9 Diff and ANSI (D-03)

| Element | Colour |
|---|---|
| diff add row (`dem-add`) | background `--accent-soft`; `+` gutter sign `--accent` |
| diff delete row (`dem-del`) | background `color-mix(in srgb, var(--danger) 8%, transparent)`; `−` sign `--danger` |
| word emphasis | same tone at 28% |
| line numbers | `--ink-faint` `tabular-nums` |
| hunk/meta rows | `--ink-faint`, `--bg-000` |
| ANSI green / red / yellow | `--accent` / `--danger` / `--warn` |
| ANSI blue, cyan | `--info` |
| ANSI magenta | `--chart-2` (#b79cff) |
| ANSI white, black, others | `--ink-muted` (D-03 "others") |

Class names (`dem-add`/`dem-del`, `a-red`…`a-black` from `streamModel.ts:253-258`) are unchanged.

### 2.10 Pane chrome

- **Alerts** (`AssistantPane.svelte:754-800`, styles `:1437-1514`): each becomes a Banner (K19). `ℹ` → tone `info`, `⚠` → tone `warn`, errors → `danger`. Max-width `var(--chat-col)`, centred. The glyphs go (D-32).
- **Pane head** (`:925-1117`): height `var(--control-h)`; title `--t-small-size` 500 `--ink`; meta `--t-caption-size` `--ink-faint`; context bar track `--bg-300` 2px, fill `--accent` (`scaleX`, M33); live state `.live-dot` (M14). Icon buttons → IconButton `sm`.
- **Empty pane card** (`:1254-1369`): Card recipe (`bg-100`, `line-100`, `--radius-lg`, padding `--card-pad`), `.rise-page` once (catalogue #84).
- **Drop zone** (`:1520-1561`): 1px dashed `color-mix(in srgb, var(--accent) 50%, var(--line-200))`, background `--accent-soft`, `--radius-lg`, label `--t-small-size` `--ink`.
- **Split panes** (`AssistantPage.svelte:401-449`): divider 1px `--line-100`; hover/drag hit area 8px with the line going `--line-300`, and `--accent` while dragging; the focused pane gets no frame. A mounting pane fades in (catalogue #83, `AssistantPane:895,916`).
- **HUDs**: ActivityHud and PlanHud are opaque `--bg-200`, 1px `--line-200`, `--radius-md`, `shadow-pop`, rows `--list-row-h`, LiveDot for running items. IntersectionObserver logic is untouched (D-21).
- **AssistantWelcome**: launch cards on the Card recipe; column `var(--chat-col)` (was 844px, `:457-460`). Loops are already gone (M16).
- **FilePathMenu**: the floating recipe (spec 02 K05-K06): `bg-100`, `line-200`, `--radius-md`, `shadow-pop`; rows `--list-row-h`, icon 16 `--ink-muted`, label `--t-small-size`; `halo-pop-in` in, `halo-pop-out` out.

### 2.11 Live footer and the turn-level states

- Footer (`StreamTurn.svelte:495-517`): `.live-dot` (M14) · verb with the one `.shim` (M15) · elapsed `tabular-nums` (plain text, catalogue #65) · tokens via RollingNumber (catalogue #66, sampled on the 1s tick, spec 03 Q3). `·` pips stay `--ink-faint`. All in `--t-caption-size`.
- Stalled (`stallLevel ≥ 1`): verb static `--warn` (no shim), dot tone `warn`. Awaiting input: dot tone `idle`, verb `--ink-muted`.
- Stall notes (`:540-550`): Banner `warn` (level 1-2) or `danger` (level 3, `role=alert`), inline.
- Compaction card (`:518-539`): a tile with LiveDot "Compacting" head, a `--bg-300` 2px track with the `--accent` `scaleX` fill (M33), meta `--t-caption-size` `--ink-faint`, note `--t-small-size` `--ink-muted`.
- `max_tokens` / refusal: a Banner under the answer and above the receipt (C06). `max_tokens` → tone `warn`, title "Response cut off", description "It reached the output length limit.", action Button `secondary` `sm` "Continue" (only when `isLast`). Refusal → tone `info`, title "The model declined to finish this response." with no action.

## 3. States

| Element | States (each listed state must be visible in the gallery) |
|---|---|
| `.sturn` | `data-phase`: live · folding · settled; `data-live` while streaming (M28); `is-empty` |
| Fold row `.sfold` | `data-zone`: activity · answer |
| Receipt | no activity (facts only) · closed · open · failed outcome · planned · `isLast` (Retry) · copy idle / copied / failed |
| Banner | max_tokens (`isLast` with Continue / not last) · refusal · stall 1 · stall 3 |
| Tile | live · done · failed · needs-you · hover · focus-visible · open · closed · disabled head (not expandable) |
| WorkLine | single-active · collapsed · open · row expanded · partial failure · all failed |
| WriteBatch | live row · done · new / edit / deleted Badges |
| Shell | running (caret) · done · failed (exit ≠ 0) · output folded · bulk open |
| Agent | running · done · failed · children |
| AskUser | needs-you · answered · option hover / picked (`halo-pop`) |
| ExitPlan | pending (typewriter) · approved · rejected |
| Code block | highlighted · growing fence (plain mono, live) · terminal fence · tall (fade + "Show N more lines") · copy idle / copied / failed |
| User bubble | text · text + images · hover (Copy) · focus-within |
| System row | compaction boundary · model switch |
| Pane | empty · streaming · latched · unlatched (jump pill) · split (2 panes, focus in either) · drop-zone active · alerts (info / warn / danger) |
| HUD | ActivityHud running / linger · PlanHud progress |
| Reduced motion | every state above with `c.sh media reduce`: no entrance, instant fold, instant pin |
| Density | compact / regular / comfy for tile head, WorkLine rows, chat text, Badges |

## 4. Motion

Every row reuses a spec 03 catalogue number; no new durations or curves are introduced.

| Element | Trigger | Property | From → to | Duration | Easing | Delay / stagger | Reduced motion | Catalogue |
|---|---|---|---|---|---|---|---|---|
| Stream block (say, tile, WorkLine, bubble) | born live (`.sturn[data-live]`) | opacity, translateY | 0→1, 3px→0 (`.rise-stream`) | `--dur-stream` 240 | `--ease-out` | none | none (net) | #50, #70, #82 |
| History hydration | tab open, reload, split | none | — (`[data-settled]` 450ms) | — | — | — | — | #51 |
| Word reveal | text delta | reveal cursor | 1 word / 30ms; catch-up lag 11 | 30ms/word | — | carry remainder (B2) | full text | #52 |
| Word fade | word crosses cursor | opacity (blur stays until G2) | 0→1 | `--dur-word` 500 (G2: 300) | `--ease-out` | age × 30ms | instant | #53 |
| Code / chrome hold | first word of a block | opacity | 0→1 | `--dur-base` | `--ease-out` | — | instant | #54, #55 |
| Activity fold | `streaming` true→false | `.sfold` grid-template-rows, opacity | 1fr→0fr, 1→0 | `--dur-base` 240 | `--ease-out` | none (all rows together) | phase jumps to settled | #73 |
| Receipt appear | phase → folding | opacity | 0→1 (`halo-fade-in`) | `--dur-fast` 150 | `--ease-out` | 60ms | instant | #73 |
| Live tail leave (footer, stall note, compaction card) | streaming ends | grid-rows + opacity (`transition:accordion` out, carrier B) | 1fr→0fr | `--dur-base` | `--ease-out` | — | `rm()` → 0 | #73 |
| Phase settle | fold timer | DOM: activity unmounts | — | after `MS.base + 50` = 290 | — | — | immediate | (new step, no paint) |
| Receipt body | toggle | `transition:accordion` (carrier B) | 0fr↔1fr; content +60ms | `--dur-base` | `--ease-out` | content 60ms | instant | #71 |
| Disclosure chevron | toggle | rotate | 0→180° (`.chev`) | `--dur-base` | `--ease-out` | — | instant | #72 |
| Tile / WorkLine body | toggle | Collapse (carrier A, `.acc-panel`) | 0fr↔1fr; content y −4→0 | `--dur-base` | `--ease-out` | content 60ms | instant | #71 |
| WorkLine chip width | open | width | fit-content → 100%, **no tween** | 0 | — | — | — | #69 |
| Tile live → done | tool result | LiveDot → outcome icon crossfade | opacity | `--dur-fast` | `--ease-out` | — | instant | #68 |
| Tile border hover | pointer | border-color | `--line-100` → `--line-200` | `--dur-base` | `--ease-out` | — | instant | (D-21) |
| Receipt actions, tokens fact | `.sturn:hover`/`:focus-within` | opacity | 0→1 | `--dur-fast` | `--ease-out` | — | instant | — |
| Footer verb swap | verb changes | opacity crossfade | 0→1 | `--dur-fast` | `--ease-out` | — | instant | #62 |
| Footer verb shimmer | streaming | `.shim` | 100%→−100% | `--dur-shim` 1.5s | linear | — | static `--ink-muted` | #63 |
| Live dots | live | `.live-dot` ping | scale 1→3, opacity .7→0 | `--dur-ping` 1.8s | `--ease-out` | — | solid dot | #64, #86 |
| Token counter | 1s tick | RollingNumber | digit strip | `--dur-roll` 900 | `--ease-out` | `min(pos,8)×45ms` | lands | #66 |
| Diff rows | diff opens | none | — | — | — | no stagger | — | #74 |
| Shell lines | output mounts | none (per-line stagger removed) | — | — | — | — | — | #75 |
| Caret (shell, exit plan) | live | `.halo-caret` | blink | `--dur-caret` 1.06s | steps(1) | — | solid | #76 |
| Ask option pick | click | `halo-pop` | scale 1→1.18→1 | `--dur-base` | `--ease-spring` | — | none | #79 |
| Banner | mount / dismiss | `.banner-motion` rise / leave | 10px→0 / 0→−6px | `--dur-base` / `--dur-leave` 200 | `--ease-out` | — | instant | (K19) |
| Scroll follow | content grows, latched | scrollTop | τ 39ms | — | exp | — | instant pin | #56 |
| Un-latch | wheel up, PgUp, ↑, Home | latch | off at once | — | — | — | same | #57 |
| Jump pill | latch lost / regained | `halo-pop-in-up` / `halo-pop-out` | scale .96 y 4 → 1 | `--dur-fast` / `--dur-pop-out` 140 | `--ease-out` | — | net | #58 |
| Jump click, exchange tick click | click / End | follow engine glide | to target | ~τ×3 | exp | — | instant | #59, #60 |
| Exchange tick active | scroll | `scaleX`, colour | 1→2 | `--dur-fast` | `--ease-out` | — | net | #61 |
| Split pane mount | split | opacity | 0→1 | `--dur-base` | `--ease-out` | — | net | #83 |
| Empty pane card | empty | `.rise-page` | 10px→0 | `--dur-slow` 600 | `--ease-out` | — | net | #84 |
| Browser dock | toggle | `dockWidth` (real width) | 0↔W | `--dur-base` | `--ease-out` | — | `rm()` | #87 |
| FilePathMenu | open / close | `halo-pop-in` / `halo-pop-out` | — | 150 / 140 | `--ease-out` | rows: `--stagger-menu` 18, cap 12 | net | (spec 02 K05) |

The fold has no JS animation and no `transitionend` dependency. `transitionend` does not fire when the computed duration is 0s (reduced motion, or `[data-settled]` landing mid-fold on a tab switch), so the phase step is a timer (`MS.base + 50`) or, under reduced motion, immediate.

## 5. Parts

### 5.1 Kit primitives used

| Primitive | Where | Variant / props |
|---|---|---|
| Badge (K13) | tile heads, WorkLine counts and failures, WriteBatch file state, "Needs you", "mid-turn" on steers | `neutral` · `accent` · `danger`; `mono` for counts |
| LiveDot (K15) | live tile heads, WorkLine running row, footer, pane head, HUD rows | `accent` · `warn` (stalled) · `idle` (awaiting) |
| Spinner (K15) | BlockHeader `spin`, PermissionBar pending | `size 16`, `decorative` inside buttons |
| Divider (K16) | receipt rule; BoundaryBlock and ModelSwitchBlock rows | unlabelled · `label` |
| Banner (K19) | max_tokens, refusal, stall notes, pane alerts | `warn` · `info` · `danger` |
| Button (K10) | receipt Copy/Retry, "Expand output", jump pill, Continue, ExitPlan approve/reject | `ghost` `sm` · `secondary` `sm` · `primary` `sm` |
| IconButton (K11) | code-block Copy, BlockHeader copy, user Copy (icon form on narrow), pane head actions | `ghost` `sm`, required `label` |
| Kbd (K14) | jump pill hint (`End`); prose `kbd` mirrors its CSS | — |
| Collapse (K30) | carrier A: tile bodies, WorkLine list and rows, Shell output, EditDiff body, StreamResult, ToolChip | `open`, `id`, `labelledby` |
| `transition:accordion` (spec 03 carrier B) | `{#if}`-gated heavy content: receipt body, live tail leave | — |
| RollingNumber (K29) | footer tokens | sampled 1s |
| tooltip (K31) | every icon-only control | — |

`Collapse` and `.acc-panel` are the same carrier A (spec 02 K30 ports `bundle.css:499-504`; spec 03 §2.2 defines `.acc-panel`). Components use `<Collapse>`; markup that is not a Svelte child (none in P4b) would use the class. This is not a conflict (see A-11-02).

### 5.2 Rift parts (API sketches)

**`stream/groupKeys.ts`** (pure, no Svelte):

```ts
import type { Group } from "./streamModel";

/** Stable key per group: survives appends, text growth, and answerAt moving.
 *  work  → "w:<first tool id of its first seg>"
 *  say   → "s:<id of the nearest preceding tool, or ^>:<n>"   n = ordinal of
 *  steer → "t:<same anchor>:<n>"                               non-work groups since that anchor
 *  Duplicate guard: if a key repeats (malformed input), append "#<i>". */
export function groupKeys(groups: readonly Group[]): string[];

type WorkSeg = Extract<Group, { type: "work" }>["segs"][number];
/** Seg key inside a work group: the first tool id of the seg. */
export function segKey(seg: WorkSeg): string; // rich → seg.tool.id; edit / line → seg.tools[0].id
```

Vitest (`groupKeys.test.ts`, node env): replays incremental snapshots of one turn (say grows → tool appended → say after tool → steer → second tool → trailing say) through `messageToTurn` + `groupBlocks`, and asserts that every key present in snapshot *k* has the same value for the same group in snapshot *k+1*, that keys are unique, and that `answerStartIndex` moving does not change any key.

**StreamTurn fold state** (sketch; replaces `:412-493`):

```svelte
<script lang="ts">
  import { untrack, onDestroy } from "svelte";
  import { MS } from "$lib/motion/tokens";
  import { accordion } from "$lib/motion/transitions";
  import { prefersReducedMotion } from "$lib/state/accessibility.svelte";
  import { groupKeys, segKey } from "./groupKeys";

  const keys = $derived(groupKeys(groups));
  type Phase = "live" | "folding" | "settled";
  let phase = $state<Phase>(untrack(() => (streaming ? "live" : "settled")));
  let foldTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const live = streaming;                      // the only tracked input
    untrack(() => {
      if (live) { clearTimeout(foldTimer); phase = "live"; return; }
      if (phase !== "live") return;              // history mount, or already folding
      if (prefersReducedMotion()) { phase = "settled"; return; }   // no activity rows: folding still fades the receipt in
      phase = "folding";
      foldTimer = setTimeout(() => { phase = "settled"; }, MS.base + 50);
    });
  });
  onDestroy(() => clearTimeout(foldTimer));

  // Rows rendered in the fold region. Settled drops activity rows (already 0-height).
  const rows = $derived(
    groups.map((g, i) => ({ g, i, key: keys[i], zone: i < answerAt ? "activity" : "answer" }))
          .filter((r) => phase !== "settled" || r.zone === "answer"),
  );
</script>

<div class="sturn" class:is-empty={isEmpty} data-live={streaming ? "" : undefined} data-phase={phase}>
  …head (compaction) unchanged…
  <div class="sturn-body" class:railed>
    {#if turn.thinking && phase !== "settled"}
      <div class="sfold" data-zone="activity"><div class="sfold-in">
        <StreamThinking active={turn.thinking.active} … />
      </div></div>
    {/if}
    {#each rows as r (r.key)}
      <div class="sfold" data-zone={r.zone}><div class="sfold-in">
        {@render renderGroup(r.g, r.i, false, r.zone === "answer" && phase !== "live")}
      </div></div>
    {/each}
  </div>
  {#if phase !== "live"}
    …Banner (C06)…
    {#if hasFacts || completionTokens || plainText.length > 0 || isLast}
      <div class="sledger" class:ruled={hasFacts} data-outcome={turn.outcome}> …row… </div>
      {#if activityOpen && phase === "settled"}
        <div class="sactivity-body" transition:accordion><div class="acc-inner">
          ….sactivity-tools + .sturn-body (thinking + activityGroups keyed by groupKeys)…
        </div></div>
      {/if}
    {/if}
  {/if}
  {#if streaming && (compacting || !thinkingNow)}
    <div class="slive-tail" out:accordion><div class="acc-inner">
      …footer, compaction card, stall notes…
    </div></div>
  {/if}
</div>
```

`renderGroup` gains a fourth parameter `forceFull` that makes `sayMode` return `"full"` (the answer always rendered full when settled, as today at `:427-429`). The `liveShown` latch (`:123-128`) stores group **keys**, not indexes, so a group keeps its latch when `answerAt` or earlier groups change.

Fold CSS (new block in `stream.css`, after the spine rules):

```css
.sfold { display: grid; grid-template-rows: 1fr; opacity: 1;
  transition: grid-template-rows var(--dur-base) var(--ease-out), opacity var(--dur-base) var(--ease-out); }
.sfold > .sfold-in { min-height: 0; overflow: hidden; }
.sturn[data-phase="folding"] .sfold[data-zone="activity"] { grid-template-rows: 0fr; opacity: 0; }
.sturn[data-phase="folding"] .sledger { animation: halo-fade-in var(--dur-fast) var(--ease-out) 60ms both; }
/* spine: drawn per activity row, hanging in the gutter */
.sturn-body.railed .sfold[data-zone="activity"] { position: relative; }
.sturn-body.railed .sfold[data-zone="activity"]::before { content: ""; position: absolute;
  left: -12px; top: 0; bottom: 0; width: 1px; background: var(--line-100); }
.sturn[data-live] .sturn-body.railed .sfold[data-zone="activity"]:nth-last-child(1 of [data-zone="activity"])::before {
  background: color-mix(in srgb, var(--accent) 40%, var(--line-100)); }
```

`overflow: hidden` on `.sfold-in` clips the tile focus ring (`--shadow-focus`, 4px). The fix is `padding: 4px; margin: -4px` on `.sfold-in` (the ring fits inside the clip, and the layout does not move). C03 includes it.

**BlockHeader** (`stream/BlockHeader.svelte:59-88`), new markup contract:

```svelte
<div class="bh-row">
  <button class="bh" type="button" disabled={!expandable} aria-expanded={expandable ? expanded : undefined}
          aria-controls={bodyId} onclick={() => onToggle?.()}>
    {#if lead}{@render lead()}{/if}
    <span class="bh-title">{@render title()}</span>
    <span class="bh-meta">{#if meta}{@render meta()}{/if}
      {#if pill}<Badge tone={PILL_TONE[pill.tone]}>{pill.text}</Badge>{/if}
      {#if spin}<Spinner size={16} decorative />{/if}
      {#if durationLabel}<span class="tile-dur">{durationLabel}</span>{/if}
    </span>
    {#if expandable}<ChevronDown size={16} class="chev" />{/if}
  </button>
  {#if copyText}<IconButton label={copyFailed ? "Copy failed" : copied ? "Copied" : "Copy"} onclick={copy}>…</IconButton>{/if}
</div>
```

Props are unchanged (`lead`, `title`, `meta`, `pill`, `spin`, `durationLabel`, `copyText`, `expandable`, `expanded`, `onToggle`), plus an optional `bodyId` for `aria-controls`. `PILL_TONE` maps the old pill tones onto Badge tones (`ok`→`accent`, `bad`→`danger`, others→`neutral`). The nested `span role=button` goes (D-30 "real buttons only").

**Receipt copy feedback** (StreamTurn `:145-160`, MessageBubble user `:157-167`, Markdown `:346`, BlockHeader): one helper, `src/lib/actions/copyFeedback.svelte.ts`:

```ts
export class CopyFeedback {
  state = $state<"idle" | "copied" | "failed">("idle");
  async copy(text: string): Promise<void>;   // writeText; sets copied|failed; resets after 1800ms; logs on failure
  destroy(): void;
}
```

## 6. Ordered commits

Branch: `halo/p4b-chat`, cut from `design/halo` after P3 and Track B (B1-B7) have merged. Merge back to `design/halo` locally; the merge waits on shortlist #1-#4 (D-27). One concern per commit (D-25).

Shorthands used in the "Verified by" column:
- **V** = `npm run check` (svelte-check, `check:tokens`, `check:ui`) + `npm test`.
- **G** = open the gallery (`bash scripts/cdp/c.sh key "Ctrl+Alt+G"`), pick the C01 state, then `c.sh look ".sturn"`.
- **R** = gallery replay with the B7 fake stream (`c.sh eval "__riftFakeStream(…)"`, profile `steady`).
- **RM** = repeat with `c.sh media reduce`, then `c.sh media ''`.
- **D** = `c.sh diff` against the P4b-start baseline (§10.1) at 2048×1104 @ DPR 1.25.
- **E** = `c.sh errors` shows 0 console errors.

### 6a · Harness and preconditions

| # | Title | Files | Exact change | Logic preserved | Risk | Verified by |
|---|---|---|---|---|---|---|
| C01 | `dev(gallery): P4b state fixtures` | `src/lib/dev/galleryFixtures.ts:1-199`; `src/routes/dev/gallery/+page.svelte:58-97` | Add canned turns for every §3 state that the stream can show: failed tool, partial WorkLine failure, ask (needs-you), agent with children, shell exit≠0 with ANSI, write batch with new/edit/deleted, diff with word emphasis, growing fence + terminal fence, prose with h1-h3/table/quote/kbd/alert/details/inline code/path, steer mid-turn, compaction live, stall 1 and 3, `stopReason: "max_tokens"` and `"refusal"`, a user message with images, a boundary row and a model-switch row. Add a state `<select>` to the gallery header. | DEV-only (`import.meta.env.DEV` guard at `+page.svelte:64`); `buildReplaySteps` signature unchanged | fixtures drift from real `ChatMessage` shapes: build them through the same constructors the existing fixtures use | V; G on each state; E |
| C02 | `chore(highlighter): escape the literal NUL key separator` | `src/lib/state/highlighter.svelte.ts:126` | Replace the raw U+0000 byte between `${norm}` and `${code}` with the escape `\u0000`. The edit tools refuse a file with a NUL byte, so make it with a 3-line `.mjs` in the scratchpad (not `node -e`, whose quoting is fragile under Git Bash): read the file as UTF-8, `replace(String.fromCharCode(0), "\\u0000")` (the JS string of backslash-u-0000), write it back | the runtime cache key is byte-identical (the same U+0000 character) | none | `LC_ALL=C grep -c -P '\x00' src/lib/state/highlighter.svelte.ts` → 0; `git diff --stat` = 1 line; V |

### 6b · Same-node fold (D-12)

| # | Title | Files | Exact change | Logic preserved | Risk | Verified by |
|---|---|---|---|---|---|---|
| C03 | `chat(fold): stable group keys` | new `src/lib/components/assistant/stream/groupKeys.ts` + `groupKeys.test.ts` | §5.2 API. Anchor = nearest preceding tool id (`^` before the first tool); ordinal counts non-work groups since the anchor; a repeated key gets `#<index>` | `streamModel.ts` untouched | a key scheme that changes when `groupBlocks` merges two says: the test replays merges | `npm test -- groupKeys` (snapshot-stability, uniqueness, `answerAt` independence) |
| C04 | `chat(fold): key groups, segs and the live latch` | `StreamTurn.svelte:123-128,134-135,356,418,427,487` | `{#each g.segs as seg, si (si)}` → `(segKey(seg))`; the three group eaches key by `keys[gi]` (activity eaches use the key of the same group); `liveShown` becomes `Set<string>` of keys; `sayMode(text, isLiveLast, key)` | render order, `sayMode` results, PermissionBar placement | Svelte throws `each_key_duplicate` on a bad key: the C03 guard prevents it | V; R with a MutationObserver probe: `c.sh eval` installs an observer on `.stream` counting removed `.md` nodes into `window.__mdRm`; after a replay **before turn end** it reads 0 |
| C05 | `chat(fold): one template, zones as attributes` | `StreamTurn.svelte:1-4,40-51,412-493`; `src/lib/styles/stream.css:386-433` (new fold block after `:431`) | Replace the `{#if streaming}/{:else}` swap with the §5.2 template: `phase` state machine (timer `MS.base + 50`, immediate under reduced motion); one keyed `{#each rows}` with `.sfold[data-zone]`; the answer zone renders full when `phase !== "live"`; receipt mounts at `phase !== "live"`; the receipt body is `{#if activityOpen && phase === "settled"}` with `transition:accordion` (M38 already switched `:474` to it), its children wrapped in one `<div class="acc-inner">` (carrier B needs a single child, spec 03 §5.2); thinking renders in a zone `activity` row. Add the four fold rules of §5.2 (`.sfold`, `.sfold-in` with the 4px focus-ring pad, folding rows, receipt fade). Delete `.sanswer`/`.sanswer > .snarr:first-child` (`:431-432`), replace with `.sfold[data-zone="answer"]:first-of-type .snarr { margin-top: 0 }` | `answerStartIndex`, `activityTitle`, facts, `hasActivity`, `isEmpty`, `outputBulk*`, `historicalReceipt` (true only inside the receipt body), `pendingPerms`, every live block's props | (1) a user scrolled away from the bottom sees the answer move up by the folded height; (2) Chromium scroll anchoring could pick a folding row as the anchor; (3) `streaming` flipping back to true for the same message re-mounts activity rows with an entrance | P0 spike #8 recording vs. a new recording: no blink; R + the answer-node probe: before turn end `c.sh eval` stores `window.__ans = document.querySelector('.sturn:last-of-type .sfold[data-zone=answer]:last-of-type .md')`, and at +400ms `__ans.isConnected && __ans.isSameNode(<same query>)` is `true` (activity-zone `.md` removals at settle are expected and not counted); `c.sh eval "document.querySelectorAll('.sturn:last-of-type .sfold[data-zone=activity]').length"` = 0 at +400ms; RM: phase settles in the same frame; history reload: `getAnimations().length` = 0 on `.sturn` |
| C06 | `chat(fold): live tail leaves through the accordion` | `StreamTurn.svelte:495-552` | Wrap the footer, compaction card and stall notes in one `<div class="slive-tail" out:accordion><div class="acc-inner">…</div></div>` (carrier B, `rm()`-wrapped; one `.acc-inner` child) | the footer's conditions and copy | the wrapper must not take `--stream-gap` twice: move the footer's top margin onto `.slive-tail` | R: footer height animates to 0 over 240ms (`c.sh measure .slive-tail` sampled with M02 `c.sh sample 600`); RM: gone in one frame |

### 6c · Receipt, Banner, column, edges, type

| # | Title | Files | Exact change | Logic preserved | Risk | Verified by |
|---|---|---|---|---|---|---|
| C07 | `chat(receipt): Ledger receipt on Halo (D-21)` | `StreamTurn.svelte:3,145-160,433-472,475-482`; `stream.css:386-445`; new `src/lib/actions/copyFeedback.svelte.ts` (+test) | Receipt per §2.4: Divider (K16) instead of `.ruled` border; outcome icon map (`Check` accent / `TriangleAlert` danger / `ListChecks` info / `Sparkles` ink-muted); title `--t-small-size` 500 `--ink`; facts `--t-caption-size` sans `tabular-nums` `--ink-faint`; tokens fact `ArrowUp` 12 + "N tokens out"; row `min-height: var(--tile-head-h)`; Copy/Retry and "Expand output" → Button `ghost` `sm`; `ChevronRight` import dropped (M38 swapped the glyph); `.sactivity-body` loses its well (`background: transparent; padding: 0`). Copy uses `CopyFeedback` (copied/failed, 1.8s) | `assistant.retryLast(tabId)`, `copy()` text source (`plainText`), hover/focus-within reveal, `aria-expanded` | the D-32 "Copy failed" text widens the row: Buttons are `inline-flex` with the label, row keeps `flex-wrap: nowrap` and facts truncate first | V; `npm test -- copyFeedback` (mock `navigator.clipboard` resolve/reject); G "settled · applied / failed / planned"; `c.sh measure ".sledger-row"` = 32/36/40 at the three densities |
| C08 | `chat(banner): max_tokens and refusal notices in the live tree (D-21)` | `StreamTurn.svelte` (after the answer rows, before `.sledger`) | `{#if phase !== "live" && message.stopReason === "max_tokens"}<Banner tone="warn" title="Response cut off" description="It reached the output length limit.">` + action Button `secondary` `sm` "Continue" when `isLast` → `assistant.send("Continue from where you left off.", tabId)`; refusal → `<Banner tone="info" title="The model declined to finish this response." />` | the send text and `tabId` routing are copied verbatim from `MessageBubble.svelte:532`; the frozen `.stop-notice` stays | none; this is new UI the live tree never had (map 02 §2) | G "max_tokens · last", "max_tokens · not last", "refusal"; click Continue in the gallery stub logs the send (no network) |
| C09 | `chat(layout): one column, --chat-col (D-06, D-33)` | `AssistantPane.svelte:1232-1236,1382,1410,1444`; `Composer.svelte:1849`; `AssistantWelcome.svelte:457-460`; `src/app.css:272` (P1 alias block) | `.messages` `max-width: var(--chat-col)`; jump `right: max(24px, calc((100% - var(--chat-col)) / 2))`; exchange rail `right: max(8px, calc((100% - var(--chat-col)) / 2 - 32px))`; alerts `max-width: var(--chat-col)`; composer-wrap `max-width: calc(var(--chat-col) + 36px)` (its 18px side padding keeps the box = column); welcome `max-width: var(--chat-col)`; delete `--chat-col-max` (0 consumers left) | every geometry relationship (composer box = message column; pill on the column's right edge) | **owner-visible: the column narrows from 844 to 760px**; touches P4c/P4d files (A-11-05) | D on chat at 2048×1104: `c.sh measure ".messages"` width 760, `c.sh measure ".composer-wrap"` content box 760; at a 700px pane: both = 100% − 48 |
| C10 | `chat(stream): one left edge + gutter spine` | `Markdown.svelte:678`; `stream.css:47-68,393`; fold block (C05) | `.md { padding-left: 0 }` inside `.sturn` and the legacy bubble alike (shared component; the legacy `.text` wrapper keeps its own inset); `.sledger { margin: 8px 0 0 }`; delete `.sturn-body.railed { padding-left: 13px }` and both `::before` gradients (`:51-64`); add the spine rules of §5.2 (per activity row, `left: -12px`, live tail tint) | `railed` derivation (`StreamTurn:86`) unchanged | at pane widths < 72px of gutter the spine sits 12px into a 24px gutter: fine; `:nth-last-child(… of …)` needs Chromium ≥ 111 (WebView2 evergreen) | D chat: prose x = tile x = receipt x (`c.sh measure ".sturn .snarr, .sturn .tile, .sledger"`) |
| C11 | `chat(type): chat prose on --t-chat-size (D-06)` | `stream.css` `.snarr`/`.snarr-beat` rules; `Markdown.svelte` `.md` base rule | `.sturn .md { font-size: var(--t-chat-size); line-height: var(--t-chat-line) }`; `.snarr-beat { font-size: var(--t-small-size); color: var(--ink-muted) }` | Markdown in non-chat hosts (thinking body, tooltips) keeps `--t-small-size` | owner-visible: 13→14px prose in compact | `c.sh eval "getComputedStyle(document.querySelector('.sturn .md')).fontSize"` = 14px / 15px / 16px per density |

### 6d · Tiles (D-21)

| # | Title | Files | Exact change | Logic preserved | Risk | Verified by |
|---|---|---|---|---|---|---|
| C12 | `tokens(tile): tile aliases → Halo tile` | `src/app.css` P1 alias block (`--tile-*` lines, spec 01 §2.4 `:281-290`) | `--tile-fill: var(--bg-100); --tile-fill-hover: var(--bg-100); --tile-border-hover: var(--line-200); --tile-border-live: var(--line-100); --tile-border-bad: var(--line-100); --tile-border-ask: color-mix(in srgb, var(--accent) 32%, var(--line-100)); --tile-well: transparent; --tile-head-pad: 0 12px; --tile-head-pad-compact: 0 12px; --tile-head-fs: var(--t-small-size); --tile-meta-fs: var(--t-caption-size); --tile-edge: 0px` | every alias name (consumers unchanged) | restyles the frozen tree's tiles too (accepted, §1.3) | V (`check:tokens`); D chat; G legacy view via the Stream-view toggle: still renders |
| C13 | `chat(tile): one tile recipe` | `src/app.css:512-606` | `.tile { border-radius: var(--radius-md); transition: border-color var(--dur-base) var(--ease-out) }`; `.tile-head { min-height: var(--tile-head-h); gap: 8px; font-size: var(--t-small-size); line-height: var(--t-small-line); color: var(--ink) }`; `.tile-body { padding: 8px 12px }`; delete the `is-bad`/`is-ask` 2px inset edges (`:535-543`), keep `.is-ask` border via `--tile-border-ask`; delete any lift/translate on hover; `.hud-float` (`:598-603`): drop `backdrop-filter` if M37 missed it, background `var(--bg-200)`, border `var(--line-200)`, `box-shadow: var(--shadow-pop)`; mark `.tile-pill` (`:577-587`) and `.tile-chev` (`:592-593`) `/* legacy tree only — delete with shortlist #6 */` | class names (`.tile`, `.tile-head`, `.tile-body`, `.tile-well`, `.is-compact`, `.is-live`) | removing the edge loses the failed signal until C14/C15 land: land C13-C15 in one merge | `c.sh measure ".sturn .tile-head"` = 32/36/40; D |
| C14 | `chat(tile): BlockHeader real buttons, Badge, chevron` | `stream/BlockHeader.svelte:1-118` | §5.2 markup: `.bh-row` with the toggle `<button>` and a sibling copy IconButton (`CopyFeedback`); `tile-pill` → Badge via `PILL_TONE`; `Loader2` → Spinner 16 `decorative`; `ChevronRight` → `ChevronDown` + `.chev`; optional `bodyId` → `aria-controls` | props API (additive only); the toggle callback | consumers that styled `.bh` as the whole row: grep `.bh` in stream/ and move the flex rules to `.bh-row` | V; keyboard: Tab reaches toggle then Copy (`c.sh act key Tab` ×2 + `c.sh eval document.activeElement.className`); copy failure path via a rejected clipboard in DEV |
| C15 | `chat(tile): one signal per state` | `StreamShell.svelte`, `StreamAgent.svelte`, `StreamAskUser.svelte`, `StreamResult.svelte`, `StreamWeb.svelte`, `StreamPlan.svelte`, `StreamThinking.svelte` (head markup) | live → `<LiveDot label={verb} />` in the head; failed → `TriangleAlert` 16 `--danger` + `<Badge tone="danger">Failed</Badge>`; ask pending → `.is-ask` border + `<Badge tone="accent">Needs you</Badge>`; done → kind icon 16 `--ink-muted`; `class:is-bad` kept as a hook but carries no CSS | status derivations (`status === "running"` etc.), `#sacard-<id>` (D-33) | double signals left behind (e.g. a red glyph plus the badge): one grep per file for `--danger` in head rules | G "failed", "needs-you", "live" (RM: LiveDot solid) |
| C16 | `chat(workline): hairline-rail rows (D-21, D-12)` | `stream/WorkLine.svelte:1-113`; `stream.css:260-326` | Rows `height: var(--list-row-h)`, icon 16 per kind (reuse the `ICONS`/`NAME_ICONS` maps) `--ink-muted`, verb sans `--t-small-size` `--ink-muted`, target mono `--t-caption-size` `--ink`; `.wr-dur` only when `t.status === "pending"`; list rule = 1px `--line-100` at `left: 20px` (replaces the gradient `:274-275`); `.wline-list` and `.wr-out` bodies in `<Collapse>`; `.wline-active` head: LiveDot + verb (no shim, M15) + target; pills → Badge (`{failedCount} failed` danger, count neutral mono); `ChevronRight` → `ChevronDown`/`.chev` (row chevrons too) | `mode`/`userOpen`/`bodyOpen` logic, `hasBody`, ReadResult/GrepResult/OutputBlock routing, the fit-content → 100% snap (M35) | 32px rows make a 12-row group taller than today's 5px-gap rows (~1.6×): owner-visible density change, flagged | G "WorkLine open / partial failure / single active"; `c.sh measure ".wline-row"` = 32/36/40 |
| C17 | `chat(writebatch): batch tile on tokens` | `stream/WriteBatch.svelte:1-133`; `stream.css:345-370` | `.wbatch` uses `.tile` (drop its own border/radius/fill rules `:347-351`); rows `--list-row-h`; file state → Badge (`new` accent, `edit` neutral, `deleted` danger); `.wb-add` `--accent`, `.wb-del.real` `--danger`; `.wb-dot` → LiveDot `idle`/`accent` (no glow `:368-369`) | inline diff toggles, counts | — | G "write batch" |
| C18 | `chat(shell): terminal surface, monochrome prompt, no line cascade` | `stream/StreamShell.svelte`; `stream/OutputBlock.svelte:116-142`; `stream.css` `.sshell` rules | Output body = CodeBlock terminal surface (`--bg-000`, 1px `--line-100`, `--radius-md`, padding 8 12, mono `--t-small-size`/`--t-small-line`); command head and `$`/`PS>` prompt in `--ink` / `--ink-muted` (D-18 monochrome; kills kind-coloured prompts, map 02 §8.5); delete `lineDelay` and every `style:animation-delay` (spec 03 Q4) | fold/tier logic (`fold`, `foldView`, `tier`, `touched`), caret (`.halo-caret`, M19) | — | G "shell exit≠0"; `c.sh eval "document.querySelectorAll('.term-line[style*=animation-delay]').length"` = 0 |
| C19 | `chat(ansi): D-03 terminal colours` | `stream/OutputBlock.svelte:221-228` | `.a-green` `--accent`; `.a-red` `--danger`; `.a-yellow` `--warn`; `.a-blue`, `.a-cyan` `--info`; `.a-magenta` `--chart-2`; `.a-white`, `.a-black` `--ink-muted` | class names (`streamModel.ts:253-258`) | `.a-white` dims from ink to ink-muted (D-03 "others") | G "shell ANSI"; `check:ui` color literal count drops by 2 |
| C20 | `chat(diff): D-03 diff colours, no row stagger` | `EditDiff.svelte` (row rules, `:571-579` `diff-row-in`); `Markdown.svelte:1302-1320` | add rows `--accent-soft` + `+` `--accent`; delete rows `color-mix(in srgb, var(--danger) 8%, transparent)` + `−` `--danger`; word emphasis 28%; line numbers `--ink-faint` `tabular-nums`; delete the `diff-row-in` animation and the `--ri` stagger var if still present after P2 | `dem-add`/`dem-del` names (D-33, test-coupled); diff computation | — | G "diff"; `npm test` (dem-* tests); RM |

### 6e · Code and prose (D-18, D-11)

| # | Title | Files | Exact change | Logic preserved | Risk | Verified by |
|---|---|---|---|---|---|---|
| C21 | `chat(code): rift-halo Shiki theme (D-18)` | new `src/lib/state/shiki/rift-halo.ts`; `src/lib/state/highlighter.svelte.ts:8,58,135` | Theme object (`name: "rift-halo"`, `type: "dark"`, `colors: { "editor.background": "#00000000", "editor.foreground": "#eceff3" }`, `tokenColors` per §2.8 scopes: `keyword, storage, keyword.control` #9cc6ff; `string, constant.numeric, constant.language, constant.character` #b79cff; `comment` #7d8592 `fontStyle: ""`; `punctuation, keyword.operator` #9aa2af; `invalid` #ff6b6b); `themes: [import("./shiki/rift-halo")]` and `theme: "rift-halo"`; comment `:8` updated | the language list, lazy init, cache | Shiki needs hex, not CSS vars: the hexes are written once and pinned by a test that compares them with `tokens.css` | `npm test -- rift-halo` (theme hex = vendored token hex); G "prose + code" |
| C22 | `chat(code): CodeBlock chrome and terminal fences` | `Markdown.svelte:340-350,1100-1140,1201-1215` | Surface `--bg-100` + 1px `--line-100` + `--radius-md`; head `min-height: var(--tile-head-h)`, language label `--t-label-size` mono caps `--ink-faint`; copy → IconButton look (HTML string: a `<button class="md-copy">` styled to K11 `sm`), feedback via `CopyFeedback` ("Copied" accent / "Copy failed" danger, 1.8s; the `:346` catch stops swallowing); terminal-language fences render un-highlighted in `--ink` on `--bg-000`; `.cm-fade` gradient to `--bg-100` | DOMPurify allow-list (D-18); the tall-block clamp (~18 lines) | the copy button is created in an HTML string, so the feedback state is written as a `data-copy` attribute, not a component | G "code"; reject the clipboard in DEV → "Copy failed" visible 1.8s |
| C23 | `chat(md): open fences stay plain until closed (D-11 #6)` | `Markdown.svelte` (the highlight pass that feeds `highlight()`) | While `streaming`, a fence whose closing ``` has not arrived renders as plain `<pre><code>` mono `--ink`; it is highlighted once, when it closes | closed-fence highlighting and its cache | a fence closed by end-of-turn (no ``` sent) must highlight on settle: the pass also runs when `streaming` goes false | R "growing fence": `c.sh eval` counts `.shiki` in the live block = 0 until close, 1 after |
| C24 | `chat(md): prose on Halo` | `Markdown.svelte:470-484,895,909-922,945,972-1069,1337,1354-1384,1464-1476` | §2.8 prose list: heading sizes without accent bars; list markers `--ink-faint`; blockquote rule; tables; alert tones; `kbd` K14 mirror; `details` chevron; inline code; `.md-path` without glow; 📋 → `.tasks-migrated-icon` CSS mask (`ListChecks`); D-30 reading comfort: every `[data-a11y-*]` selector that targets `.bubble`/`.markdown-body` (grep `data-a11y` in `src/app.css`) also lists `.sturn .md`, with the Lexend override kept last | the HTML pipeline, DOMPurify config, task migration | `mask-image` data-URI must be in the scoped `<style>` (CSP `img-src` is not involved; masks load from CSS) | G "prose"; `c.sh text ".tasks-migrated"` contains no emoji; D-30 contrast: `c.sh eval` contrast of list markers ≥ 4.5:1 |

### 6f · User bubble, system rows, steers, blocks

| # | Title | Files | Exact change | Logic preserved | Risk | Verified by |
|---|---|---|---|---|---|---|
| C25 | `chat(user): user bubble (D-21)` | `MessageBubble.svelte:548-556,1208-1223,1238-1246,1251-1280` | `.bubble[data-role="user"] .text`: `background: var(--bg-200); border: 1px solid var(--line-100); border-radius: 16px 16px 4px 16px; padding: 10px 14px; max-width: min(560px, 80%); font-size: var(--t-chat-size); line-height: var(--t-chat-line); box-shadow: none` (no gradient); `.user-time` `--t-caption-size` `--ink-faint`; user Copy → Button `ghost` `sm` with its own `CopyFeedback` instance (the frozen assistant branch keeps `copy()`); thumbs `--radius-md` + 1px `--line-100` | `.bubble[data-role=user]` and `.body .content .text` (CDP `serve.cjs:876-881`); `white-space: pre-wrap`; lightbox | the user-branch and assistant-branch share `<style>`: every new rule is scoped under `[data-role="user"]` | G "user + images"; `c.sh measure ".bubble[data-role=user] .text"` max width 560 |
| C26 | `chat(system): boundary and model-switch rows as labelled Dividers` | `bubble/BoundaryBlock.svelte:1-149`; `bubble/ModelSwitchBlock.svelte:1-76` | Each renders `<Divider label=…>` (K16) with its existing text; detail/expand affordances → IconButton `sm` at the row end; popovers on the floating recipe | the boundary's summary expansion and `workspaceRoot` links | — | G "boundary", "model switch" |
| C27 | `chat(steer): steers and demoted beats` | `StreamTurn.svelte:339-354`; `stream.css` `.ssteer*` rules | `.ssteer`: `--bg-200`, 1px `--line-100`, `--radius-md`, padding 6 12; `CornerDownRight` 16 `--ink-muted`; text `--t-small-size` `--ink`; "mid-turn" → `<Badge tone="neutral">Mid-turn</Badge>`; thumbs `--radius-sm` | tooltip text, image MIME guard | — | G "steer" |
| C28 | `chat(blocks): thinking, plan, web, result` | `StreamThinking.svelte`, `StreamPlan.svelte`, `StreamWeb.svelte`, `StreamResult.svelte`; `stream.css:447-477,524-537` | Tiles on tokens; plan rows `--list-row-h`, done check `--accent`, active row LiveDot (its `.shim` is gone, M15); plan track `--bg-300` 2px + accent `scaleX` (M33); web/result meta `--t-caption-size` `--ink-faint`; bodies through `<Collapse>` | plan item derivation, `StreamResult` pass/fail counts | — | G "plan", "web", "test result" |
| C29 | `chat(blocks): agent, ask, exit plan` | `StreamAgent.svelte`, `StreamAskUser.svelte`, `StreamExitPlan.svelte`; `stream.css:479-519` | Agent: child rows `--list-row-h`, LiveDot per running child; ask: option rows `--control-h`, picked marker `halo-pop` (M-list), focus ring `--shadow-focus`; exit plan: `.sxa-btn` → Button (`primary` Approve, `secondary` Keep planning), plan body on the tile surface | `#sacard-<id>` (ActivityHud jump), answer submission, typewriter (#78) | `--bg-0` in StreamAskUser was fixed in P1 (D-33) | G "agent", "ask", "exit plan"; ActivityHud row click scrolls to `#sacard-…` |
| C30 | `chat(shared): ToolChip, PermissionBar, ReadResult, GrepResult` | `ToolChip.svelte` (`:479`, `:626`, `:730-731`, `:772`), `PermissionBar.svelte:1-147`, `stream/ReadResult.svelte`, `stream/GrepResult.svelte` | Tile tokens and Badges in both ToolChip variants; PermissionBar Allow = Button `primary` `sm`, Deny = `secondary` `sm`, pending Spinner; ReadResult uses the `rift-halo` highlight with line numbers `--ink-faint`; GrepResult match `<mark>` = `--accent-soft` background, `--ink` text, path rows `--list-row-h` mono | permission IPC, path click handlers | ToolChip is in both trees: no assistant-branch-only CSS here | G (stream) + Stream-view toggle off (legacy) both render |

### 6g · Footer, HUDs, pane, split, welcome, menu

| # | Title | Files | Exact change | Logic preserved | Risk | Verified by |
|---|---|---|---|---|---|---|
| C31 | `chat(footer): live footer and turn notes` | `StreamTurn.svelte:495-551`; `stream.css` `.sfooter`, `.sstall-note`, `.scompact-*` rules | Tokens via `<RollingNumber>` (sampled on the 1s tick; spec 03 Q3); elapsed stays plain `tabular-nums` text (`AnimatedCount` removed from the footer); stalled → verb `--warn`, LiveDot `warn`; awaiting → LiveDot `idle`; stall notes → Banner (`warn` for 1-2, `danger` for 3); compaction card → tile with LiveDot head | `stallLevel`, `footerVerb`, compaction estimates and copy | — | R; G "stall 1/3", "compaction"; RM: counter lands, dot solid |
| C32 | `chat(hud): ActivityHud and PlanHud on Halo (D-21)` | `stream/ActivityHud.svelte`, `stream/PlanHud.svelte` | Opaque `--bg-200`, 1px `--line-200`, `--radius-md`, `shadow-pop`; rows `--list-row-h`; running = LiveDot; counts Badge `neutral` `mono`; stop button IconButton `sm` "Stop" | IntersectionObserver visibility logic (`ActivityHud:166-200`), 350ms debounce, linger | after the fold, settled agent cards unmount (as today, when the receipt is closed): the linger sees no cards and shows the bar, same as today | G with an agent fleet; scroll the cards out of view → bar appears |
| C33 | `chat(pane): jump pill and exchange rail` | `AssistantPane.svelte:818-846,1380-1435` | Jump pill → Button `secondary` `sm` with `ArrowDown` + "Latest" + `<Kbd>End</Kbd>`, float surface, `halo-pop-in-up`/`halo-pop-out`; rail ticks per §2.3 | B4-B6 latch and glide logic | pill label width: "Latest" fixed text, no count | R: wheel up → pill at once; End → glide; `c.sh measure` pill right edge = column right edge |
| C34 | `chat(pane): alerts become Banners` | `AssistantPane.svelte:754-800,1437-1514` | Each alert → `<Banner tone=…>` with its title/description/action; `ℹ`/`⚠` glyphs deleted (D-32) | alert conditions and actions | — | G (or `c.sh eval` to set the alert state in DEV) |
| C35 | `chat(pane): pane head, empty card, drop zone` | `AssistantPane.svelte:925-1117,1254-1369,1520-1561` | §2.10 values; head icon buttons → IconButton `sm`; empty card on the Card recipe with `.rise-page`; drop zone dashed accent | head actions, drag-and-drop handlers | — | D chat (empty pane, streaming pane); drop: `c.sh eval` dispatch `dragenter` → zone visible |
| C36 | `chat(split): dividers and pane mount` | `AssistantPage.svelte:401-449`; `AssistantPane.svelte:895,916` | Divider 1px `--line-100`, 8px hit area, `--line-300` hover, `--accent` while dragging; pane mount → opacity only (`halo-fade-in` `--dur-base`) | resize maths, `dockWidth` (M11) | — | `c.sh act` drag the divider; D split view |
| C37 | `chat(welcome): AssistantWelcome cards` | `AssistantWelcome.svelte` | Cards on the Card recipe (`--bg-100`, `--line-100`, `--radius-lg`, `--card-pad`); titles `--t-small-size` 500; descriptions `--t-caption-size` `--ink-muted` | launchpad actions | — | D chat (new chat) |
| C38 | `chat(menu): FilePathMenu on the floating recipe` | `FilePathMenu.svelte:1-97` | `floatingSurface` recipe (K05-K06); rows `--list-row-h`, icon 16; `data-menu-stagger` on the list | open/close logic, actions | — | G: right-click a path in prose → menu; Esc closes |
| C39 | `chore(check-ui): ratchet after P4b` | `design-system/ui-baseline.json` | `npm run check:ui -- --update` **only if** counts dropped (spec 02 K38 rule); diff the JSON before committing | baseline never rises | baking in a rise | `npm run check:ui` |

C12-C15 land together in one merge (the failed signal is missing between C13 and C15). Everything else can be bisected one commit at a time.

## 7. Contracts (D-33)

| Contract | Status in P4b | Consumers checked |
|---|---|---|
| `.sturn` | kept; gains `data-phase` and keeps `data-live` (M28) | `scripts/cdp/serve.cjs:876-877`, rift-ui skill |
| `.snarr` | kept on every say row, in both zones (not in the D-33 list today; A-11-09) | `serve.cjs:880` reads its text |
| `.bubble[data-role=user]` and `.body .content .text` | kept (C25 restyles them in place) | `serve.cjs:876-879` |
| `.stream` | kept; `settleGuard` (M27) stays on it | ActivityHud `:177`, B4-B6 |
| `.csurf-col` | kept | ActivityHud `:161,176` |
| `.splan` | kept (C28 restyles in place) | PlanHud |
| `#sacard-<id>` | kept on StreamAgent (C15, C29) | ActivityHud `:162,183` |
| `#actnode-*` + `.tid-flash` | produced only by the frozen MessageBubble (`:510`, `:1369-1374`); untouched | lives and dies with shortlist #6 |
| `dem-add` / `dem-del` | kept (C20) | EditDiff tests |
| `a-red` … `a-black` | kept (C19) | `streamModel.ts:253-258` |
| `--chat-col` offsets (D-33 coupling) | resolved: pane, jump pill, exchange rail, alerts, composer-wrap and welcome all read `--chat-col` (C09) | `c.sh measure` in C09 |
| `--tile-head-h` | consumed by tile heads, code-block heads and the receipt row | density check in C13 |
| `[data-a11y-*]` | extended to `.sturn .md` (C24, D-30) | Settings → Reading comfort |
| New hooks | `.sfold[data-zone]`, `.slive-tail`, `.bh-row` | document in `.agents/skills/rift-ui/SKILL.md` in C05/C06/C14 |

No contract name is renamed in P4b.

## 8. Deletions

### 8.1 In P4b (no gate; every consumer moves in the same commit)

| What | Where | Commit |
|---|---|---|
| `{#if streaming}/{:else}` hard swap | `StreamTurn.svelte:412-493` | C05 |
| index keys `(gi)`/`(si)` | `StreamTurn.svelte:356,418,427,487` | C04 |
| `.sanswer` rules | `stream.css:431-432` | C05 |
| rail gradient + `padding-left: 13px` | `stream.css:51-64` | C10 |
| `.md` `padding-left: 10px`, `.sledger` `margin-left: 10px` | `Markdown.svelte:678`, `stream.css:393` | C10 |
| `--chat-col-max` | `app.css:272` + 5 consumers (+ the `MessageBubble.svelte:720` comment, reworded) | C09 |
| tile 2px edges | `app.css:535-543` | C13 |
| BlockHeader `span role=button` | `BlockHeader.svelte:74-84` | C14 |
| `.wline-list` gradient rule, `.wb-dot` glow | `stream.css:274-275,368-369` | C16, C17 |
| OutputBlock `lineDelay` | `OutputBlock.svelte:116-119` and 4 call sites | C18 |
| ANSI oklch literals | `OutputBlock.svelte:225-226` | C19 |
| `diff-row-in` (if P2 left it) | `EditDiff.svelte:571-579` | C20 |
| `github-dark-dimmed` | `highlighter.svelte.ts:58,135` | C21 |
| heading accent bars, `md-path` glow, 📋 | `Markdown.svelte:909-922,1464-1476,482` | C24 |
| `ℹ`/`⚠` glyphs | `AssistantPane.svelte:754-800` | C34 |
| footer `AnimatedCount` use | `StreamTurn.svelte:509,513` (the component stays: AiHealthPage uses it) | C31 |

### 8.2 Gated by shortlist #6 (not deleted in P4b)

- The MessageBubble assistant branch (`:283-546` minus the user rows), its `TurnSummary`, `.stop-notice` (`:524-542`), `.steer-note`/`.unknown-note` (`:1100-1155`), the legacy `transition:slide` sites (`:408,488`), `#actnode-*` and `.tid-flash`, and the Stream-view toggle.
- The legacy-only CSS this spec keeps alive for them: `.tile-pill` (`app.css:577-587`), `.tile-chev` (`:592-593`), the `--tile-edge`, `--tile-border-live` and `--tile-border-bad` aliases (C12 sets them to inert values), and the legacy ToolChip cards (`toolchip/AgentCard`, `AskUserCard`, `TodoCard`) if no live consumer remains.
- These are added to the #6 list (A-11-13). Each lands as one commit in P5 after the owner says yes.

## 9. Canvas board 02 and board 04

**Board 02 "Chat"** (D-25) is the numbers contract for this spec. It is plain markup on Halo tokens in the Claude Design project "Rift · Halo", played in Play mode; the owner never runs a dev server. Artboards:

1. **Live turn** at compact, regular, comfy (§2.4 a).
2. **Fold** (§2.4 b): a Play button runs live → folding → settled at the real timings, with a frame strip at 0, 120, 240 and 290ms. The timings are read from board 04's current preset, not owned here.
3. **Receipt**: closed, open, failed, planned, facts-only, hover (Copy/Retry), copy failed.
4. **Tile state grid**: live / done / failed / needs-you × three densities; WorkLine collapsed / open / partial failure.
5. **User bubble + system rows** (§2.7).
6. **Code colour A/B** (D-18 dissent): A = `rift-halo` muted (default), B = `rift-halo-ink` monochrome, on the same snippet, plus a terminal fence.
7. **Diff + ANSI** (§2.9).
8. **Column**: today's 844px vs `--chat-col` 760px, with the composer under each.
9. **Banners**: max_tokens (with Continue), refusal, stall 1, stall 3.

Exports back into this file: §2.1-§2.10 values (px, hex, ms). If the owner changes one, the change lands as its own commit on top of the C-list, one value per commit. Board 02 has no merge gate of its own (D-12, D-18, D-21 say "Gate: no"), but the P4b merge waits on shortlist #1-#4 (D-27).

**Board 04 "Motion lab"** (spec 03 §9) owns pacing: word ms, catch-up, word fade, blur, follow τ, `--dur-base`, `--dur-stream`, `--rise-stream`. It inlines the same `reveal.ts`/`follow.ts` that B1 creates. Board 02 shows motion only where anatomy depends on it (the fold), and reads its numbers from board 04's selected preset. When board 04 changes `--dur-base` (G1), the fold and every accordion in this spec change with it; nothing in this spec hard-codes 240 or 300 (the fold timer reads `MS.base`).

## 10. Verification and acceptance

There are no DOM tests in this repo (D-25), so P4b is verified with vitest for pure logic, the CDP bridge for the running app, and the gallery for states. All `c.sh` verbs below exist at `22727d24` (`scripts/cdp/c.sh:133-657`) except `media` (M01) and `sample` (M02), which P2 adds.

### 10.1 Baseline (before C01)

1. `bash scripts/cdp/c.sh baseline` after `c.sh tour chat` and on the gallery replay (B7), at 2048×1104 @ DPR 1.25.
2. The P0 spike #8 recording of the end-of-turn blink is the "before" for C05.
3. Record today's counts for §10.3 with the listed commands.

### 10.2 Per commit and per phase

- Per commit: **V**, plus the commit's own check (§6).
- Phase gate (before merging `halo/p4b-chat` into `design/halo`):
  - `npm run check` and `npm test` green;
  - `c.sh tour home chat settings ai-health` + `c.sh diff` per surface, reviewed (only chat should change; home only via AssistantWelcome);
  - `c.sh inspect` and `c.sh errors` show 0 console errors;
  - gallery: every §3 state looked at (`c.sh look ".sturn"` / `c.sh shot-sel`) at compact, regular and comfy;
  - `c.sh media reduce`: replay a turn; no entrances, fold instant, pin instant; then `c.sh media ''`;
  - `c.sh sample 3000` during a replay and during a scroll: no long frames introduced by the fold;
  - the Stream-view toggle off: the legacy tree still renders (frozen, recoloured only).
- A real turn is not run unless shortlist #7 is "yes"; without it, scroll-follow is reported as "verified on the fake stream only".

### 10.3 Counts that must go down

| Count | Command | Before → after |
|---|---|---|
| Index-keyed eaches in StreamTurn | Grep `\((gi\|si)\)\}` in `StreamTurn.svelte` | 4 → 0 |
| Answer `.md` remounts across turn end | `__mdRm` probe before turn end (C04) + `__ans` identity probe across it (C05); activity-zone removals at settle are expected | today 1 per say (the swap) → 0 |
| Entrance animations on history reload | `c.sh eval "document.getAnimations().length"` 100ms after reload of a 50-turn chat | (P0 #7 value) → 0 |
| `--chat-col-max` references | Grep in `src` | 7 (1 definition, 5 consumers, 1 comment) → 0 |
| `.tile-pill` in the live tree | Grep `tile-pill` in `stream/` | > 0 → 0 |
| `ChevronRight` in the live tree | Grep in `stream/` and `StreamTurn` | > 0 → 0 |
| Fake buttons | Grep `role="button"` in `stream/` | 1 → 0 |
| Swallowed copy failures | Grep `copy failed` in `StreamTurn.svelte`, `Markdown.svelte` | 2 → 0 |
| Status emoji / glyphs | Grep `📋\|ℹ\|⚠\|↑ ` in `Markdown.svelte`, `AssistantPane.svelte`, `StreamTurn.svelte` | > 0 → 0 |
| Shell per-line delays | Grep `animation-delay` in `OutputBlock.svelte` | 4 → 0 |
| oklch literals in OutputBlock | Grep `oklch\(` | 2 → 0 |
| `github-dark-dimmed` | Grep in `src` | 3 → 0 (incl. the comment) |
| Signals per failed tile | gallery "failed": icon + Badge only | 3 → 2 (one state, two cues: icon and word) |
| `check:ui` colour / px metrics | `npm run check:ui` | each ≤ baseline; C39 ratchets |

### 10.4 Acceptance checklist

- [ ] No blink at turn end; the answer's `.md` node is the same node before and after (C04/C05 probes).
- [ ] Settled turns from history mount with zero animations.
- [ ] Prose, tiles and the receipt share one left edge; the spine sits in the gutter.
- [ ] One signal per tile state; Badges replace every `.tile-pill` in the live tree.
- [ ] Every disclosure uses `.chev` (chevron-down, 180°) and one of the two accordion carriers.
- [ ] Code uses `rift-halo`; terminal fences and prompts are monochrome; copy failures are visible.
- [ ] Diff and ANSI follow D-03; no green anywhere.
- [ ] The column is `--chat-col`, and the composer box equals it.
- [ ] max_tokens and refusal Banners show in the live tree; Continue works on the last turn.
- [ ] Reduced motion: no entrances, instant fold, instant pin, LiveDots solid.
- [ ] All three densities: tile heads 32/36/40, WorkLine rows 32/36/40, chat text 14/15/16.
- [ ] 0 console errors; `check:ui` did not rise.

## 11. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Group keys are not stable across some real stream shape (e.g. a say split by a late thinking block), so a Markdown remounts | medium | the blink comes back for that shape | C03 test replays incremental snapshots; the `__mdRm` probe runs on all C01 fixtures; the dedupe guard prevents crashes |
| Folding moves the answer under a reader who scrolled up | medium | reading position jumps by the activity height | latched readers are bottom-anchored (scrollTop clamps); unlatched: verify Chromium scroll anchoring keeps the answer; if not, settle instantly when unlatched (Q7) |
| `overflow: hidden` on `.sfold-in` clips focus rings and popovers inside live tiles | medium | invisible focus, clipped menus | 4px pad/negative margin for rings; popovers are portalled (tooltip singleton, floating recipe) |
| Fold timer and `[data-settled]` interact on a tab switch mid-fold | low | activity unmounts without the collapse being seen | harmless: it is invisible either way; the timer, not `transitionend`, drives the phase |
| `streaming` toggles back to true on the same message (continue / resume) | low | activity rows re-mount with an entrance | accepted; they are born live |
| Restyling shared `.tile` aliases changes the frozen legacy tree | certain | legacy tiles look Halo-ish, not frozen | accepted (D-21 shared parts; same as P1); documented in §1.3 |
| Column narrows 844 → 760 | certain | owner-visible; long code lines scroll sooner | own commit (C09), board 02 artboard 8, Q1 |
| WorkLine 32px rows make open groups taller | likely | less on screen while a turn works | `--list-row-h`, board 02 artboard 4, Q3 |
| Shiki hex values drift from tokens | low | code colours differ from the palette | C21 test compares theme hex with `tokens.css` |
| K35/K35b and B1 both create `reveal.ts`/`follow.ts` | medium | two APIs | A-11-01: B1 reconciles to spec 03's API |
| WebView2 lacks `:nth-last-child(… of S)` | very low | live spine tint missing | it is decorative; the base spine still draws |
| ActivityHud linger shows the bar after the fold because cards unmounted | certain | same as today when the receipt is closed | no change in behaviour; noted in C32 |

## 12. Effort

| Block | Commits | Size | Sessions |
|---|---|---|---|
| Harness + NUL fix | C01-C02 | S | 0.5 |
| Fold | C03-C06 | L | 1.5 |
| Receipt, Banner, column, edge, type | C07-C11 | M | 1 |
| Tiles, WorkLine, batch, shell, ANSI, diff | C12-C20 | L | 1.5 |
| Code and prose | C21-C24 | M | 1 |
| User, system, steer, blocks, shared | C25-C30 | M | 1 |
| Footer, HUD, pane, split, welcome, menu, ratchet | C31-C39 | M | 1 |
| Board 02 build (canvas) | — | M | 1 |

Total ≈ 7.5 sessions for code, plus 1 for board 02 (L overall). Depends on P1, P2 (incl. Track B), P3 and P0 spikes #7 and #8. Unblocks the P4b merge, the gallery baselines and the shortlist #6 deletions in P5.

## 13. Amendments proposed to 00-decisions

- **A-11-01 (D-11 ownership, spec 02 K35/K35b vs spec 03 B1).** Two specs author `reveal.ts` and `follow.ts` with different APIs and phases. Proposal: spec 03's API (`stepReveal`, `followStep`, `FOLLOW_TAU_MS`) is canonical because it mirrors `Markdown.svelte:216-252` and `AssistantPane.svelte:169-190` and lands at P4b start as D-11 says. If K35/K35b landed in P3, B1 reconciles to that API and deletes the duplicates in the same commit.
- **A-11-02 (D-12 "one accordion").** Clarifies spec 03 A-4: `Collapse` (K30) *is* carrier A as a component, and `transition:accordion` is carrier B. This spec names the carrier per site (§5.1). No second visual.
- **A-11-03 (D-12 keys).** "Group keys are the first block id" cannot hold: say and steer groups carry no id (`streamModel.ts:186-189`). Proposal: `groupKeys.ts` keys work groups by their first tool id, and say/steer groups by the nearest preceding tool id plus an ordinal; `streamModel.ts` stays untouched.
- **A-11-04 (D-12 template).** "One keyed region present in both states" is implemented as one keyed `{#each}` over all groups with a `data-zone` attribute. Settled turns drop the activity rows after they collapse (honouring spec 03 A-4: settled turns don't mount every tile), and the receipt body mounts them on demand. DOM order equals visual order; no CSS `order` tricks.
- **A-11-05 (D-27 phase scope, D-25 rename rule).** C09 edits `Composer.svelte:1849` (P4c) and `AssistantWelcome.svelte:457-460` (P4d) because the `--chat-col-max` → `--chat-col` move must land with all consumers. It is one line each, and it is owner-visible (844 → 760).
- **A-11-06 (D-21 "32px rows").** WorkLine rows use `--list-row-h` (32/36/40), so "32px" holds at compact and scales with density.
- **A-11-07 (D-21 receipt).** D-21 says "outcome icon" without a set. Proposal: `Check` accent (applied/ok), `TriangleAlert` danger (failed), `ListChecks` info (planned, matching D-03 plan → info), `Sparkles` ink-muted (reasoning / work completed). The tokens fact loses `↑` for `ArrowUp` + "out" (D-32).
- **A-11-08 (D-18 copy feedback).** The 1.8s "Copied" / "Copy failed" rule is written for CodeBlock. Proposal: one `CopyFeedback` helper gives every chat copy affordance (receipt, user bubble, BlockHeader, code) the same 1.8s feedback, so D-32 "copy failures are shown" holds everywhere at once.
- **A-11-09 (D-33 contracts).** Add `.snarr` and `.bubble .body .content .text` (read by `scripts/cdp/serve.cjs:876-881`) to the stream contract list, and register the new hooks `.sturn[data-phase]`, `.sfold[data-zone]`, `.slive-tail`.
- **A-11-10 (D-21 freeze vs shared CSS).** The `.tile` aliases and recipe and the `.md` inset are shared, so the frozen legacy tree changes with them. Proposal: accept, as P1 already does; keep legacy-only rules (`.tile-pill`, `.tile-chev`) until #6.
- **A-11-11 (D-21 Banner tones).** D-21 says "inline Banner" without tones. Proposal: max_tokens → `warn` with Continue; refusal → `info` (not `danger`: nothing failed and nothing can be retried by the user, and `danger` would announce `role=alert`). See Q2.
- **A-11-12 (D-21 placement).** The Banner sits under the answer and above the receipt, so the notice is read with the text it qualifies and the Ledger line stays last.
- **A-11-13 (shortlist #6 list).** Add `.tile-pill`, `.tile-chev`, `--tile-edge`, `--tile-border-live`, `--tile-border-bad`, `#actnode-*`/`.tid-flash` and the legacy ToolChip cards to item 6's deletion list.
- **A-11-14 (D-12 entrances vs D-29).** The receipt is not "born live" content; it fades (150ms, +60ms) only on the folding phase and is static on history mounts, which keeps D-29's "hydration is static" rule.

Map and register facts corrected while verifying (this session):

- D-34 cites `Markdown.svelte:1205` `#22272e`. Line 1205 is a comment; the literal is `oklch(0.18 0.012 245)` at `:1208-1209`.
- D-18 cites the theme at `highlighter.svelte.ts:8`. Line 8 is a comment; the theme is imported at `:58` and named at `:135`.
- `highlighter.svelte.ts` is UTF-8, not UTF-16. One literal NUL byte at `:126` (the cache-key separator) makes the edit tools refuse the file; C02 escapes it.
- Map 02 §9 calls `.tile` app-wide. It is chat-only (`assistant/*` and `PermissionBar`); AI Health's `.ah-tile` is a different class.
- D-12 keys "by first block id"; say and steer groups have no id (`streamModel.ts:186-189`).
- `#actnode-*` is produced only by the frozen MessageBubble (`:510`); the live StreamTurn produces none.
- The `max_tokens`/refusal notice exists only in the legacy tree (`MessageBubble.svelte:524-542`), confirming map 02 §2.
- `--chat-col-max` has 5 consumers (4 in `AssistantPane.svelte`, plus `Composer.svelte:1849`, which is P4c), not only pane-side ones.

## 14. Open questions (each has a default; the build proceeds on the default)

1. **Q1 Column width.** Narrow to 760 now, or keep 844 until board 02 is played? Default: **760 in its own commit (C09), revertable in one commit.**
2. **Q2 Refusal tone.** `info` or `warn`? Default: **`info`, no action.**
3. **Q3 WorkLine row height.** `--list-row-h` (32/36/40) or the denser `--control-dense-h` (28/32/36)? Default: **`--list-row-h`, per D-21.**
4. **Q4 Settled answer demotion.** Should settled answer rows keep `sayMode` demotion? Default: **no; settled answers render full, as today.**
5. **Q5 Live spine tint.** Keep the 40% accent tint on the last live activity row? Default: **keep; it is stationary and disappears with the fold.**
6. **Q6 Receipt open state on settle.** Default: **closed, as today.**
7. **Q7 Fold while the reader is unlatched.** Fold anyway, or settle instantly when the reader has scrolled up? Default: **fold anyway; switch to instant only if the C05 check shows the answer moving under an unlatched reader.**
8. **Q8 "Needs you" wording.** Default: **"Needs you" (D-21).**
9. **Q9 Copy feedback everywhere at 1.8s** (A-11-08). Default: **yes.**
10. **Q10 Code body size.** Mono 13/18 (`--t-small-*`) for both code and terminal fences? Default: **yes; no new size token.**


