# Design — `ChatTabsBar.svelte` hot-file split

> #20 follow-on brief (queued behind composer-split C2-C7). Authoritative state in
> [src/lib/components/shell/ChatTabsBar.svelte](../../src/lib/components/shell/ChatTabsBar.svelte) —
> **1761L mapped 2026-06-09**: script L1-465 · markup L467-932 · style L934-1761 (~828L CSS).
> Line numbers go stale fast — re-locate by symbol/snippet anchor before cutting.
> Pattern: children under `src/lib/components/shell/tabsbar/` + shared helper module
> (same shape as composer-split.md).

## Invariants (carry forward)

- **`portal` action (L33)** — every popover uses it to escape the rail's `overflow:hidden`.
  Children re-import from a shared module (candidate: hoist to `$lib/actions/portal.ts` — it's now
  duplicated in Composer's `composer/helpers.ts` too; dedupe in this arc's H0, ONE canonical copy).
- **`menuKeydown` (L201)** — ARIA arrow-key nav shared by ProjectMenu + ViewMenu. Goes into the
  helper module; both children import it.
- **`viewAnchor` DOM ref crosses clusters** — it anchors the View menu AND `openHistory()` uses it
  to position the history popover. Splitting ViewMenu and HistoryLayer apart requires ref-forwarding
  via prop/callback — the tightest seam in the file; keep them together if it gets ugly.
- **Drag-reorder is the nervous system** (L249-355): `dragFromIdx`/`dragOverIdx`, six handlers,
  `window.addEventListener("dragend", onDragEnd)` (L348) + `onDestroy(onDragEnd)` (L355), and writes
  to `assistant.draggingTabId`. The window-level dragend listener is the WebView2 missed-dragend
  workaround — a partial extraction that leaves cleanup in the parent breaks it. Moves as one unit
  with the TabStrip, never piecemeal.
- **`ctxTone`** is consumed by the ctx-pill in `.actions` AND the ctx detail panel — share via prop.
- **`pulse` $effect (~L382-394)** — task updates drive `.panels-btn.pulse`; cross-cluster state,
  keep effect and consumer in one scope or pass as prop.
- **`:global` selectors move with their element's owner:** `.proj-pill :global(.proj-ico)` (L1201),
  `.proj-pill :global(.proj-chev)` (~L1206), `.branch-chip :global(svg)` (~L1216),
  `.history-full-panel :global(.drawer)` (~L1260), `.agents-pill :global(svg)` (~L1339),
  `.cli-badge :global(svg)` (~L1532), view-menu icon classes (~L1603-1724).
- No behavior changes while moving; bodies/markup/CSS verbatim.

## H0 — helper hoist (~40L, LOW) — ✅ SHIPPED 2026-06-09

`leafName`/`prettyPath`/`shortK` + `menuKeydown` → `tabsbar/helpers.ts` (formatters vitest'd;
menuKeydown excluded — needs live DOM). Portal discovery: a canonical `$lib/actions/portal.ts`
ALREADY existed (target param + isConnected guard; used by WebBrowserPage/FilePathMenu) — the
Composer + ChatTabsBar copies were dupes of it. ChatTabsBar's variant focuses the first
interactive descendant → added as `portalFocus` there; ChatTabsBar imports
`portalFocus as portal` (markup untouched), Composer re-pointed to canonical `portal`.
NOTE: `titleFor` (L275) reads a `$derived` — NOT pure, stays.

## Extraction order (blast-radius ascending, after H0)

- **T1 — `tabsbar/CliUpdatePanel.svelte`** (~90L): markup L765-810 + `.cli-*` CSS (~L1517-1623) +
  CLI cluster state (L90-131: derives + `runCliUpdate` + `toggleCliPanel` + close effect).
  Imports `cliUpdate` store directly (pervasive use — per composer-split rule, keep the store import).
- **T2 — `tabsbar/CtxDetailPanel.svelte`** (~110L): markup L698-763 + CSS ~L1441-1515 + ctx-window
  derive cluster (L401-464). Props: anchor ref + `open`; `ctxTone` shared back to the parent pill
  via the helper recompute or prop.
- **T3 — `tabsbar/ProjectMenu.svelte`** (~80L): markup L812-851 + CSS ~L1219-1228 + cluster
  L190-247 (minus shared `menuKeydown`, now imported).
- **T4 — `tabsbar/ViewMenu.svelte`** (~80L): markup L854-932 + CSS ~L1657-1728 + cluster L151-188.
  `openHistory()` crosses into the history cluster → callback prop `onOpenHistory`.
- **T5 — `tabsbar/HistoryLayer.svelte`** (~60L): markup L675-696 + CSS ~L1231-1295 + L39-80 state.
  Needs the `viewAnchor` forwarding decision (see invariants) — do AFTER T4 proves the seam.
- **T6 — `tabsbar/TabStrip.svelte`** (~130L, LAST): markup L467-531 + CSS ~L962-1121 + the whole
  drag-reorder unit L249-355 + `titleFor`/`isStreamingTab`/`paneIndexFor`. Highest risk — the
  window-listener lifecycle and `assistant.openTabs`/`reorderTabs` writes move as one commit.

Residual target after T6: ~500-600L (rail shell + `.actions` right-chip strip + OpenInPaneMenu wiring).
StatusChips (the `.actions` cluster) deliberately NOT extracted — it's the hub that anchors every
popover; extracting it last buys little and risks every seam at once. Revisit after T1-T6.

## Hard rules for the executor

Same as composer-split.md: one child per commit · `npm run check` 0/0 after EACH ·
`npx vitest run` green · CDP visual verify per extraction with dev running (`bash scripts/cdp/c.sh
look` — exercise tab drag-reorder + every popover after T-extractions touch them) · verbatim moves,
no restyling · props down, callbacks up.
