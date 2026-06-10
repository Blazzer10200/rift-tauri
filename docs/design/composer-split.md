# Design — `Composer.svelte` hot-file split

> Brief for the #20 frontend follow-on (queued in ISSUES #20). Authoritative state in
> [src/lib/components/assistant/Composer.svelte](../../src/lib/components/assistant/Composer.svelte) —
> **3131L after C1 ✅ (shipped 2026-06-09**; original map 3197L: script L1-1064 · markup L1064-1781 ·
> style L1781-3197, ~1416L CSS — markup/CSS boundaries shifted ~-66L by C1, script-only).
> C1 made `composer/helpers.ts` (7 pure fns + 17 vitest); `portal` later moved to canonical
> `$lib/actions/portal.ts` (ChatTabsBar H0 dedupe). Next: C2.
> Line numbers go stale fast — re-locate by symbol/snippet anchor before cutting.
> Pattern: **child components under `src/lib/components/assistant/composer/`** + one pure-helper TS module.
> This is NOT the TS-module pattern from assistant-svelte-split.md — markup+CSS+state move together per child.

## Invariants (carry forward)

- **`onKey` (L793-951, ~160L) is the nervous system** — one keyboard handler coordinates slash menu,
  mention popover, settings/perm menus, queue recall (`recallOffset`), Enter/Alt+Enter fire/steer.
  It stays in Composer.svelte until LAST; children receive open/close/index via props or small
  exported stores, never their own keydown listeners on the textarea.
- **`fire()`/`steer()`/`onBtnClick`/send-button `mode`** (`send|stop|queue`, L537-584 + L952-981) stay
  in the parent — they bind tab state + assistant store + textarea focus.
- **Scoped CSS moves with its markup.** Each extracted child takes its style rules verbatim; selectors
  that style across the boundary (e.g. `.composer-wrap` model tinting via `--model-color`) stay in the
  parent and keep working via CSS custom properties — never `:global()` as a shortcut.
- **`data-model` tinting:** `.composer-wrap` sets `--model-color`; children consume the variable only.
- **No behavior changes while moving.** Svelte 5 runes: child-local UI state (`$state`) moves with the
  child; anything derived from `tab`/`assistant` is re-derived in the child from props.

## Extraction order (blast-radius ascending)

### C1 — `composer/helpers.ts` (~120L, LOW)
Pure fns: `fuzzyScore` (L280), `fmtClock` (L137), `fmtSize` (L782), `bytesToBase64` (L692),
`portal` action (L706), `isFileDrag` (L984), `effortIdxFromClientX` math if separable.
Zero DOM/store deps → unit-testable; add a tiny vitest file (mirrors TS-split regression net).

### C2 — `composer/AttachmentsRow.svelte` (~80L + CSS, LOW)
Markup L1152-1181 (`{#each attachments}` chips + `attachError`). Props: `attachments`,
`attachError`, `onRemove`. Paste/drop/file-pick handlers STAY in parent (they write tab state).

### C3 — `composer/QueueRail.svelte` (~200L + CSS, MEDIUM-LOW)
Markup L1067-1151 (queued-chips strip + steer affordance) + state/handlers L44-99
(`editingId`/`editText`/`startEditQueued`/`commitEditQueued`/`onEditKey`/`removeQueued`/
`sendQueuedNow` + `dragId` drag-reorder trio). Mostly self-contained against `assistant` store.
NOTE: Rail-v2 (ISSUES) will rework this surface — extract first so Rail-v2 lands in a small file.

### C4 — `composer/LivePills.svelte` (~150L + CSS, MEDIUM-LOW)
Markup L1595-1687 + derive cluster L117-152 (`liveItems`/`agentCount`/`shellCount`/`toolCount`/
`turnElapsed`/`tokPerSec`/`fmtClock` use). Props: `tab`, `queue`, `streaming`. The 1s `now` ticker
moves with it.

### C5 — `composer/EnhanceBar.svelte` (~250L + CSS, MEDIUM)
Markup L1200-1268 (preview/diff/word-stagger) + state cluster L585-659 (`enhancing`/`enhancedPreview`/
`enhanceError`/`enhanceOriginal`/`showEnhanceDiff`/`groundEnhance`/`enhancedWords`/`runEnhance`/
`acceptEnhanced`/`dismissEnhanced`). Callback props for draft writes (`onAccept(text)`) — the child
never touches `tab.draft` directly.

### C6 — `composer/MentionPopover.svelte` + `composer/SlashMenu.svelte` (~180L + CSS, MEDIUM)
Mention: markup L1301-1326 + L250-345 (`detectMention`/`mentionState`/`mentionIdx`/`mentionResults`/
`pickMention`). Slash: markup L1280-1300 + `slashOpen`/`slashFiltered`/`slashIdx`/`pickSlash`.
Keyboard nav indices stay OWNED by parent `onKey`; children render + click only.
(ui-audit-2026-06-09 §2 wants the slash menu redesigned to palette language — extract first,
restyle in the new small file.)

### C7 — `composer/SettingsMenu.svelte` (~350L + CSS, HIGH — last)
Settings popover + model picker + effort slider + fast-mode + perm popover: markup L1327-1463 +
L1688-1760, state L346-535 (`settingsOpen/Idx`, `permOpen/Idx`, model/effort derives, drag-slider
trio, `positionPerm`/`onDocPermMousedown` portal positioning). Tightest coupling to `onKey` +
`assistant` store — do after C1-C6 shrink the file and the prop seams are proven.

Residual Composer.svelte after C7: textarea + autosize + ghost placeholder + gauge/divider +
toolbar row + drag-drop shell + `onKey`/fire/steer/queue logic. Target ≤900L.

## Hard rules for the executor

- One child per commit; `npm run check` 0/0 after EACH; `npx vitest run` stays green (51/51 + any new C1 tests).
- **CDP visual verify per extraction** (`bash scripts/cdp/c.sh look` with dev running) — svelte-check
  can't see visual regressions; composer is the most-touched surface in the app.
- Bodies/markup/CSS move verbatim — no restyling, no rune refactors, no prop renames while moving.
  Cleanups (incl. ui-audit slash-menu redesign) are follow-up commits.
- Props down, callbacks up; children never import the `assistant` store unless the cluster already
  reads it pervasively (QueueRail does — keep it, don't invent prop plumbing for 12 call sites).

## Follow-on

After C1-C7: `MessageBubble.svelte` 1742L and `ChatTabsBar.svelte` (shell/, 1761L) are next #20
candidates — mapped + briefed 2026-06-09: `messagebubble-split.md` + `chattabsbar-split.md`.
