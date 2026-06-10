# Design — `MessageBubble.svelte` hot-file split

> #20 follow-on brief (queued behind composer-split C2-C7). Authoritative state in
> [src/lib/components/assistant/MessageBubble.svelte](../../src/lib/components/assistant/MessageBubble.svelte) —
> **1471L after H0 ✅** (mapped at 1742L: script L1-541 · markup L543-843 · style L845-1742,
> ~898L CSS — script now ~270L, markup/CSS line refs shifted ~-271L; re-anchor by snippet).
> Line numbers go stale fast — re-locate by symbol/snippet anchor before cutting.
> Pattern: children under `src/lib/components/assistant/bubble/` + one pure-helper TS module
> (same shape as composer-split.md).

## Invariants (carry forward)

- **No nervous-system keyboard handler here** (unlike Composer's `onKey`) — only `onclick` buttons.
  That makes this split lower-risk than Composer's.
- **`renderBlock` snippet (L665-737) is called from TWO sites** — the tool-group body (~L773) and
  the main `{#each grouped}` loop (~L800). Any child that needs it receives it as a snippet prop;
  never duplicate the block-renderer logic.
- **`stage-bounce` keyframe is shared** by `.stage-dot` (~L1087) and `.ta-dot` (~L1158). If
  StageStrip and TrailingActivity end up in separate files, the keyframe lives in ONE of them and
  the other re-declares it verbatim (scoped CSS can't import keyframes across components).
- **`:global` selectors move with their target element's owner:** `.boundary-pill :global(svg)`
  (~L895), `:global(.steer-marker-icon)` (~L1469), `:global(.tid-flash)` + its reduced-motion
  override (~L1736-1741).
- **`lastBlockKey`** is read by both the toolgroup branch and the block branch to pulse the
  "current" node while streaming — thread it as a prop wherever timeline rendering moves.
- **`--idx` custom property** (set in the `{#each}` at ~L755/L797, consumed by `.tl-node`) drives
  the entrance stagger — keep the index plumbing intact or the stagger silently dies.
- **`reducedMotion` const (L5-7)** feeds transition durations at ~L771/L806 — recompute locally in
  any child that owns those transitions.
- No behavior changes while moving; bodies/markup/CSS verbatim.

## H0 — `bubble/helpers.ts` first (~250L, LOW) — ✅ SHIPPED 2026-06-09

Done same-day as the brief: all 17 fns + `TextSegment`/`TimelineUnit`/`NodeStatus` types moved
verbatim; 22 vitest cases; MessageBubble 1742→1471L. `previewOf` was found DEAD in the component
(defined, never called) — moved + tested but not imported back; drop or wire it during B-passes.

The big win: this file is unusually rich in pure fns. Move (verbatim, with their types):
`isInlineDiffTool` (L20) · `shortToolName` (L24) · `isCardTool` (L27) · `isGroupableChip` (L35) ·
`TextSegment` + `parseTextBlock` (L48) · `reconcileSplitHeaders` (L83) · `TimelineUnit` types +
`statusOf` (L137) + `nodeKind` (L146) · `formatBoundaryAt` (L239) · `formatDuration` (L243) ·
`elapsedFor` (L252) · `previewOf` (L264) · `summarizeGroup` (L287) · `shortModel` (L315) ·
`lineDelta` (L341) · `coalesceToolGroups` (L409) · `numberActions` (L445).
All zero-DOM/zero-store. Vitest file mirrors composer C1 (`parseTextBlock`/`reconcileSplitHeaders`/
`coalesceToolGroups` are the high-value targets — real parsing logic, currently untested).

## Extraction order (blast-radius ascending, after H0)

- **B1 — `bubble/BoundaryBlock.svelte`** (~110L): markup L543-579 + `.boundary*` CSS (~L846-916) +
  `boundaryExpanded` state. Props: `boundaryBlock`. Cleanest seam in the file.
- **B2 — `bubble/TrailingActivity.svelte`** (~51L): markup L805-814 + `.trailing-activity`/`.ta-*`
  CSS (~L1135-1170). Props: `show`, `stageLabel`. Carries a re-declared `stage-bounce`.
- **B3 — `bubble/StageStrip.svelte`** (~69L): markup L649-662 + `.stage-*` CSS (~L1066-1110).
  Props: `stageLabel`. Owns the canonical `stage-bounce`.
- **B4 — `bubble/TurnSummary.svelte`** (~85L): markup L816-838 + `.turn-summary`/`.ts-*` CSS
  (~L1681-1733) + `turnStats`/`turnDurationMs`/`costLabel`/`autoApplied`/`bypassApplied` derives.
  `reviewDiff()` touches `assistant.ui` → callback prop `onreview`.
- **B5 — `bubble/ToolGroup.svelte`** (~90L): toolgroup branch of the `{#each}` (~L744-777) +
  `.tg-*`/`.tl-toolgroup` CSS (~L1320-1379) + `expandedGroups`/`toggleGroup`. Receives
  `renderBlock` as a snippet prop + `lastBlockKey`.
- **B6 — renderBlock extraction** (LAST, ~73L markup + scattered CSS): only worth it if residual
  size still warrants; CSS is interleaved across ~400L of the style block — the riskiest cut.

Residual target after B5: ~900-1000L (shell + turn-head + timeline loop + renderBlock).

## Hard rules for the executor

Same as composer-split.md: one child per commit · `npm run check` 0/0 after EACH ·
`npx vitest run` green (68 + new H0 tests) · CDP visual verify per extraction with dev running
(`bash scripts/cdp/c.sh look` on a chat with a streamed turn — bubbles are the core render surface) ·
verbatim moves, no restyling · props down, callbacks up.
