---
title: Rift · Halo — spec 02 · component kit (P3)
status: draft
updated: 2026-10-19
phase: P3
depends_on:
  - spec/00-decisions.md (D-06, D-07, D-08, D-09, D-10, D-11, D-13, D-14, D-16, D-17, D-20, D-23, D-24, D-25, D-27, D-29, D-30, D-31, D-32, D-33)
  - P1 token spec (vendored Halo tokens.css, alias block, --t-* type tokens, z ladder, radius rename)
  - P2 motion spec (styles/motion.css keyframe home, reduced-motion net, reactive reduced-motion probe, D-29 data-settled guard)
  - D-28 spikes #1, #2, #5 (spike #6 is made unnecessary by this spec, see §13 A-3)
tags: [rift, halo, redesign, spec, kit]
---

# 02 · Component kit (phase P3)

## 0. Summary

- One kit in `src/lib/components/ui/` (D-09): 15 bits-ui parts (10 added; dropdown, context menu, Select, alert dialog and Separator restyled), 22 CSS-only atoms, 7 motion helpers (pure TS + runes) in `src/lib/motion/`, a rewritten `use:tooltip` singleton, a Halo toast stack over the existing toast store, and `/dev/ui` showing every part in every state at all three densities.
- First fix is `recipes.ts:20,58,60`: bits-ui 2.19.3 never emits `data-open`/`data-closed`. That is confirmed from the installed source, see §1.3. Today every menu, select and alert-dialog opens and closes with no motion at all.
- D-27 says P3 changes are visible only in `/dev/ui`. That holds for every new part. It cannot hold for the four `ui/` parts the app already renders: dropdown menu, context menu, Select (incl. `shared/Select`) and alert dialog. K05-K07 and K23 restyle them in place and turn their dead motion on, app-wide. §13 A-14 records this deviation, and the P3 merge gate adds a `c.sh diff` of the surfaces that open them. The tooltip layer and the new toaster are mounted only on `/dev/ui` until P4a/P4f mount them app-wide in one-line commits.
- 50 commits in 7 sub-phases (P3a infra → P3g catalog), §6. Effort L, 6-7 sessions. No canvas board gates P3 (D-25); boards 00/04/08 export numbers back into the token commits only.

## 1. Scope + files

Every line number below was opened and checked on `design/chat-ledger` @ 22727d24, 2026-10-19.

### 1.1 Existing files this spec edits

| Path | Lines | What is there today | Touched by |
|---|---|---|---|
| `src/lib/components/ui/recipes.ts` | 60 | `floatingPanel` :13-14, `floatingMotion` :19-20 (dead `data-open:`), `menuItem` :26-27, `menuItemDanger` :30-31, `menuItemChecked`/`selectItemSelected`/`menuCheck` :36-40, `menuLabel` :43-44, `menuSeparator` :46, `menuShortcut` :48, `fieldTrigger` :52-53, `dialogOverlay` :57-58, `dialogPanel` :59-60 | K05, K06, K07 |
| `src/lib/components/ui/button/button.svelte` | 85 | `tv()` base :7, 8 variants :9-18, 8 sizes :19-28 | K10 |
| `src/lib/components/ui/button/index.ts` | 17 | barrel | K10, K11 |
| `src/lib/components/ui/separator/separator.svelte` | 21 | bits-ui Separator, `bg-border` | K16 |
| `src/lib/components/ui/alert-dialog/*` | 13 files, 7-40 lines each | content :21-30 uses `dialogPanel`; overlay :13-18 uses `dialogOverlay`; action/cancel use `buttonVariants` | K07, K24 |
| `src/lib/components/ui/dropdown-menu/*` | 18 files | content :20-34 (`sideOffset=4`, `min-w-40`, `origin-(--bits-dropdown-menu-content-transform-origin)`) | K06 |
| `src/lib/components/ui/context-menu/*` | 17 files | content :19-31 | K06 |
| `src/lib/components/ui/select/*` | 13 files | content :24-44 (viewport :39 `max-h-70`), trigger :18-33 (chevron `in-data-[state=open]:rotate-180` :31), item :17-35 | K06 |
| `src/lib/utils.ts` | 25 | `createCn` registry :11-19 (radius, shadow, ease only) | K03 |
| `src/lib/utils.test.ts` | 32 | 5 `cn` cases, :21-26 pin Rift theme names | K03 |
| `src/lib/actions/tooltip.ts` | 41 | a11y-only no-op: strips `title`, promotes text to `aria-label` :24-41. Type `TooltipOpts` :10-17 | K31 |
| `src/lib/actions/sliderBubble.ts` | 81 | `thumbW = 18` literal :28 (D-33 coupling) | K23b |
| `src/lib/state/toast.svelte.ts` | 286 | `ToastStore`: `push` :146, `dismiss` :203, `pause`/`resume` :212-225, `MAX_VISIBLE=3` :58, timeouts :65-71 (danger 8000, warn 6000, else 4000), severities `ok|warn|danger|info|muted` :8 | K33, K34 |
| `src/app.css` | 780 | bridge `@theme inline` :29-115, `[data-slot]` RM kill :120-122, density blocks :289-291, `.kbd` :357-367, `.slider-bubble` :371-392, `.btn` :394-421, `.input` :423-434, `.field-label`/`.help`/`.dim` :436-444, `.divider` :446, `.pill` :449-470, `.rift-toggle` :476-493, `.card` :496-500, `.dialog-overlay` :612-618, `.spin`/`.pulse` :623-629, `enter`/`pop-in` :634-650, `.rift-menu*` :695-736 | K01, K02 (additions only; legacy rules are deleted in later phases, §8) |
| `src/routes/dev/ui/+page.svelte` | 445 | catalog of button, separator, dropdown, context, select, alert-dialog + keyboard table; density + accent preview bar :41-69, :84-104 | K36, K37 |
| `design-system/ui-baseline.json` | 8 | ratchet counts | K38 (`--update` only if counts drop) |

### 1.2 Existing files this spec does NOT edit (consumers swap in P4; listed so nobody touches them in P3)

`src/lib/components/ToastHost.svelte` (212; mounted `AppShell.svelte:332`), `src/lib/components/AppShell.svelte` (513; mounts UpdateDialog :329, CommandPalette :334, McpServersDialog :336), `src/lib/components/shell/Skeleton.svelte` (57; one consumer, `AssistantWelcome.svelte`), `src/lib/components/assistant/stream/AnimatedCount.svelte` (113), `src/lib/components/shared/PageHero.svelte` (48), `shared/LimitBar.svelte` (82), `shared/Select.svelte` (56, wraps `ui/select`, inherits the restyle for free), `styles/settings-controls.css` (37), `styles/onboarding.css` (548), `styles/stream.css` (555), `routes/+layout.svelte` (66; P4a adds the tooltip mount).

### 1.3 Verified facts this spec builds on

1. **The menu motion is dead (D-28 #1, confirmed from source).** bits-ui 2.19.3 sets `data-state="open"|"closed"` (`node_modules/bits-ui/dist/internal/attrs.js:13-15`) and `data-starting-style`/`data-ending-style` (`:19-25`). It never emits `data-open`. `tw-animate-css` 1.4.0 defines no custom variant (a grep of `dist/tw-animate.css` for `data-open|data-state` returns 0). Tailwind 4 compiles `data-open:` to `[data-open]`, which never matches. So `recipes.ts:20,58,60` generate CSS but never fire. The P0 CDP read becomes a 30-second sanity check.
2. **CSS leave animations work.** `bits-ui/dist/internal/animations-complete.js:20-58` waits for `node.getAnimations()` on the content node itself before it unmounts. A `data-[state=closed]:` keyframe on the content element gets its full 140/200/240ms. A keyframe on a child gets cut off, so every leave animation sits on the bits-ui content node.
3. `--bits-floating-transform-origin` is published per part (`bits-ui/dist/bits/utilities/floating-layer/use-floating-layer.svelte.js:112`, aliased in `internal/floating-svelte/floating-utils.svelte.js:18`) and already consumed at `dropdown-menu-content.svelte:29`.
4. Accordion content only sets `hidden` under `hiddenUntilFound` (`bits/accordion/accordion.svelte.js:282-285`). So `forceMount` plus a grid-rows collapse works.
5. **`check:ui` rule 1** (`design-system/check-ui.mjs:54-91`) runs every `class="…"` string in `ui/` that has a dash or colon through Tailwind and HARD-FAILS on any token that generates no CSS. **So scoped-CSS atoms in `ui/` must not use dashed class names.** They style through `data-slot` / `data-part` / `data-variant` / `data-tone` / `data-size` attribute selectors. `class` stays reserved for the consumer pass-through via `cn()`. Rule 3 (`:122-130`) keeps `ui/` at zero raw colours and zero `-[…]` arbitrary values. Density-tiered sizes therefore use the parenthesised var form `h-(--control-h)`, which the ratchet regex `/-\[[^\]\s]*\](?!:)/` (`:104`) does not catch and which `z-(--z-popover)` already uses (`recipes.ts:14`).
6. `cn` is shadcn's `cn@0.4.0` (`utils.ts:9`). Its theme group ids include `animate`, `radius`, `shadow`, `ease`, `text`, `spacing` (`node_modules/cn/dist/config.d.ts:10`). Any new non-T-shirt key needs a `createCn` entry (D-08).
7. `/dev/ui` does not render inside `AppShell` (`routes/+page.svelte:2,5` mounts AppShell; `/dev/ui` is a sibling route under `routes/+layout.svelte`). App-level hosts (ToastHost, the future tooltip layer) are absent there unless the page mounts them.
8. `use:tooltip`: 174 call sites in 35 `.svelte` files, all imported as `import { tooltip } from "$lib/actions/tooltip"`. Only 2 use the object form: `Composer.svelte:1642` (`kbd`) and `EditDiff.svelte:385` (`placement: "bottom"`).
9. `ui/button` is imported directly only by `routes/dev/ui/+page.svelte`. The alert-dialog action/cancel import `buttonVariants`. No app code uses Button sizes `xs`/`lg`/`icon-xs`/`icon-lg` or variants `soft`/`warn`/`info`/`link` today (grep 0 outside `dev/ui`). The API can be reshaped without breaking app call sites.
10. `onRadioKey` is a local function in `SettingsPage.svelte:242-257`, not a shared util. `ui/segmented` absorbs it (D-31).
11. `prefersReducedMotion()` (`state/accessibility.svelte.ts:103-106`) is a one-shot snapshot. The kit consumes P2's reactive replacement (§1.4).
12. `@floating-ui/dom` 1.8.0 is only a transitive dependency of bits-ui (`bits-ui/package.json:41-42`, `^1.7.1`). The kit does not import it (§13 A-3).

### 1.4 Prerequisites (P1/P2 deliverables this spec consumes)

P3 starts only after P1 and P2 are merged into `design/halo` (D-27 order). Before commit K01, run the precheck below. A missing item means STOP and land it in its owning phase. Do not fork it here.

| Need | Owner | Precheck (Grep in `src/`) | Used by |
|---|---|---|---|
| Halo vars: `--bg-000..300`, `--line-100..300`, `--ink`, `--ink-muted`, `--ink-faint`, `--accent`, `--accent-ink`, `--accent-soft`, `--glow-strong`, `--info*`, `--warn*`, `--danger*`, `--chart-1..3`, `--scrim`, `--space-1..8`, `--radius-sm/md/lg/pill`, `--shadow-pop`, `--shadow-focus`, `--opacity-disabled`, `--z-raised..tooltip` | P1 (vendored `src/lib/styles/halo/tokens.css` + D-03/D-08 deviation block) | `--opacity-disabled` and `--shadow-pop` resolve in `lib/styles/halo/tokens.css` | every part |
| Rift radius rename: `--radius-xs` 6, bridge `--radius-md` → Halo 12, `--radius-lg` → Halo 16 | P1 (D-07) | `app.css` bridge line for `--radius-md` no longer reads `var(--radius)` (today :84) | every rounded part |
| z ladder: `--z-dropdown` 65, `--z-overlay` 50, `--z-toast` 60, `--z-tooltip` 70; `--z-dialog` → overlay, `--z-popover` → dropdown | P1 (D-08) | `--z-dropdown:\s*65` | overlays |
| Type tokens `--t-<step>` (size) + `--t-<step>-lh` for label 11/14, caption 12/16, small 13/18, chat 14/22 (density), body 15/24, heading 16/22, subtitle 18/24, title 24/30, data 13/18, metric 28/32 | P1 (D-06) | `--t-small` | K02 bridge |
| `--row-h` retuned 32/36/40 | P1 (D-06 token values) | `--row-h:\s*32px` | menu rows |
| Motion tokens `--dur-fast` 150, `--dur-base` 300, `--dur-slow` 600, `--dur-page` 520, `--stagger` 60, `--ease-out`, `--ease-spring`, rises `--rise-page` 10px, `--rise-reveal` 6px, `--rise-stream` 3px | P2 (D-10) | `--rise-page` | all motion |
| `src/lib/styles/motion.css` with Halo keyframes copied verbatim from `bundle.css`: `halo-rise` (rewritten to `translateY(var(--rise, var(--rise-page)))`), `halo-page-in`, `halo-fade` :195, `halo-ping` :82, `halo-spin` :451, `halo-indet` :461, `halo-pop` :160, `halo-pop-in` :411, `halo-pop-in-up` :413, `halo-menu-item` :420, `halo-zoom` :287, `halo-drawer-in` :363, `halo-drawer-in-left` :692, `halo-toast-in` :324, `halo-reveal` :104, `halo-skel-out` :107, `halo-tag-in` :621, `halo-bell-swing` :354 | P2 (D-27 "one home for keyframes") | `@keyframes halo-zoom` | K02 `--animate-*` keys |
| Reduced-motion net in `app.css` (D-13 text) plus a reactive probe exported as `motion.reduced` (a `$state` fed by a `matchMedia` change listener) | P2 (D-13) | `export const motion` | every JS-driven helper |
| `data-settled` zero-duration guard (D-29) | P2 | `\[data-settled\]` | catalog only |

If the P1 spec picked different names for the `--t-*` pairs, K02's bridge lines take the P1 names and nothing else in this spec changes. Atoms read type through the bridge keys or through the vars in one scoped `font:` line each.

### 1.5 Kit contracts this spec owns

**A. Density vars.** D-06 gives values but no names. This spec names them. They live in `app.css`. Defaults go on `:root` (compact) and overrides extend the existing blocks at `app.css:289-291`. Each value is set in exactly one place (K01).

| Var | compact (default) | regular | comfy | D-06 row |
|---|---|---|---|---|
| `--control-h` | 32px | 36px | 44px | control |
| `--control-dense-h` | 28px | 32px | 36px | dense-toolbar control |
| `--control-r` | 8px (`--radius-sm`) | 12px (`--radius-md`) | 12px | D-07: ≤32 → 8, ≥36 → 12 |
| `--control-dense-r` | 8px | 8px | 12px | D-07 |
| `--control-px` | 12px | 14px | 16px | button/input side padding |
| `--tile-head-h` | 32px | 36px | 40px | stream tile head (consumed in 4b) |
| `--toggle-w` × `--toggle-h` | 36 × 20px | 40 × 22px | 44 × 26px | toggle |
| `--toggle-inset` | 2px | 2px | 3px | knob gap (Halo 3px, `bundle.css:142`) |
| `--check-size` | 16px | 18px | 20px | checkbox / radio dot |
| `--card-pad` | 16px | 20px | 24px | card padding |
| `--badge-h` | 20px | 22px | 24px | badge (D-06 silent; see §14 Q3) |
| `--palette-input-h` | 44px | 48px | 52px | palette search row (Halo 52, `bundle.css:294`) |

Derived (declared once on `:root`; they resolve against the density overrides on the same element):

```css
:root {
  --toggle-knob: calc(var(--toggle-h) - 2px - 2 * var(--toggle-inset));          /* 14 / 16 / 18 */
  --toggle-travel: calc(var(--toggle-w) - 2px - 2 * var(--toggle-inset) - var(--toggle-knob)); /* 16 / 18 / 18 */
  --slider-thumb: calc(var(--check-size) + 0px);                                  /* 16 / 18 / 20 — D-33 sliderBubble */
  --dialog-w: min(480px, calc(100vw - 32px));
  --dialog-w-sm: min(400px, calc(100vw - 32px));
  --dialog-w-lg: min(560px, calc(100vw - 32px));
  --dialog-max-h: calc(100vh - 32px);
  --palette-top: 12vh;
  --drawer-w: min(760px, calc(100vw - 24px));
  --tooltip-max-w: 240px;
}
```

At comfy the toggle matches Halo exactly: knob 18, travel 18 (`bundle.css:142,144`).

**B. Bridge keys added to `@theme inline` (`app.css:29-115`), plus their `createCn` registrations (`utils.ts:11-19`).** Existing keys are untouched (D-08).

| Key group | Keys | `createCn` group |
|---|---|---|
| Type | `--text-label/caption/small/chat/body/heading/subtitle/title/data/metric: var(--t-…)`, each with `--text-<k>--line-height: var(--t-<k>-lh)`; `--text-label--letter-spacing: .12em`, `--text-label--font-weight: 500`, `--text-title--letter-spacing: -0.02em`, `--text-metric--letter-spacing: -0.03em`, `--text-metric--font-weight: 600` | `text: ["label","caption","small","chat","body","heading","subtitle","title","data","metric"]` |
| Shadow | `--shadow-pop: var(--shadow-pop)`, `--shadow-focus: var(--shadow-focus)`, `--shadow-glow: 0 8px 28px var(--glow-strong)` (`bundle.css:47`) | `shadow: ["float","pop","focus","glow"]` |
| Ease | `--ease-out: var(--ease-out)` (Tailwind's default was wiped at `app.css:27`) | already a known name |
| Animate | see table C | `animate: [all names in C]` |
| Utility | `@utility scrollbar-none { scrollbar-width: none; &::-webkit-scrollbar { display: none } }` (D-23 hidden sites; tab lists) | n/a |

`opacity-(--opacity-disabled)`, `h-(--control-h)`, `rounded-(--control-r)`, `z-(--z-overlay)` and similar use the parenthesised var form and need no key.

**C. Animation keys** (in `@theme inline`; `inline` makes each utility emit the literal shorthand). Halo keyframes come from P2's `motion.css`. The four `*-out` keyframes are Rift-derived equivalents of Halo's `.is-leaving` transitions. bits-ui needs a keyframe on the content node (§1.3 #2), and Halo uses a transition on a parent we don't have. Values are copied from the cited lines.

| Key | Value | Halo source |
|---|---|---|
| `--animate-pop-in` | `halo-pop-in var(--dur-fast) var(--ease-out) both` | `bundle.css:406,411` |
| `--animate-pop-in-up` | `halo-pop-in-up var(--dur-fast) var(--ease-out) both` | :412-413 |
| `--animate-pop-out` | `halo-pop-out 140ms var(--ease-out) both` | :410 (`opacity:0; scale(.97)` 140ms) |
| `--animate-menu-row` | `halo-menu-item 220ms var(--ease-out) calc(min(var(--i, 0), 12) * 18ms) backwards` | :419-420 |
| `--animate-zoom-in` | `halo-zoom var(--dur-base) var(--ease-out) both` | :285,287 |
| `--animate-zoom-out` | `halo-zoom-out 200ms var(--ease-out) both` | :286 (`scale(.96)` 200ms) + :284 opacity |
| `--animate-fade-in` | `halo-fade var(--dur-fast) var(--ease-out) both` | :281 |
| `--animate-fade-out` | `halo-fade-out 200ms var(--ease-out) both` | :284 |
| `--animate-drawer-in` | `halo-drawer-in var(--dur-base) var(--ease-out) both` | :362-363 |
| `--animate-drawer-out` | `halo-drawer-out 200ms var(--ease-out) both` | :364 |
| `--animate-check-pop` | `halo-pop var(--dur-base) var(--ease-spring)` | :156,160 |

Rift-derived keyframes added by K02 to `motion.css`, in their own commented block (`/* Rift: leave halves of Halo .is-leaving transitions — bits-ui needs a keyframe on the content node */`):

```css
@keyframes halo-pop-out   { to { opacity: 0; transform: scale(.97); } }
@keyframes halo-zoom-out  { to { opacity: 0; transform: scale(.96); } }
@keyframes halo-fade-out  { to { opacity: 0; } }
@keyframes halo-drawer-out{ to { opacity: 0; transform: translateX(40px); } }
@keyframes halo-tip-in    { from { opacity: 0; transform: translateY(var(--tip-rise, 4px)); } }
@keyframes halo-toast-in-top { from { opacity: 0; transform: translateY(-16px) scale(.96); } } /* top-anchored stack, D-20 */
```

Menu-row stagger index (K02, `motion.css`). bits-ui items cannot know their index, so a parent marked `data-menu-stagger` numbers its children. Groups carry the attribute too.

```css
:where([data-menu-stagger]) > :nth-child(2)  { --i: 1; }
/* … one line per child, :nth-child(3) {--i:2} through :nth-child(12) {--i:11} … */
:where([data-menu-stagger]) > :nth-child(n + 13) { --i: 12; }
```

**D. Colour vocabulary.** Tailwind recipes speak the bridge keys, which P1 repoints at Halo (D-08): `bg-bg`=bg-000, `bg-surface`=bg-100, `bg-surface-hover`=bg-200, `bg-surface-active`=bg-300, `border-border`=line-100, `border-border-strong`=line-200, `border-field-border`=line-300, `text-fg`=ink, `text-fg-muted`=ink-muted, `text-fg-subtle`=ink-faint, `text-accent-fg`=accent-ink. Scoped CSS in new atoms reads Halo names directly (`var(--bg-100)`, `var(--line-200)`, `var(--ink)`). Those are the canonical vendored vars, so it adds no new dialect.

**E. Spacing map.** Tailwind's `--spacing` is not wiped (`app.css:18-28`), and its 4px step equals Halo's grid: Halo `space-1/2/3/4/5/6/7` = 4/8/12/16/24/32/48px = Tailwind `1/2/3/4/6/8/12`. Halo `space-5` (24px) is Tailwind `6`, not `5`. Recipes below use Tailwind numbers.

## 2. Target design

Common rules for every part (they are not repeated per part):

- **Focus:** `outline:none; box-shadow: var(--shadow-focus)` on `:focus-visible` (Tailwind `focus-visible:outline-none focus-visible:shadow-focus`), `bundle.css:45`. Text inputs show it on `:focus` (Halo `:124-126`).
- **Disabled:** `opacity: var(--opacity-disabled)` (.45) plus `cursor: not-allowed` (D-30).
- **Press:** `scale: .97`, the individual property, on buttons only, at `--dur-fast` (D-16). Tailwind 4's `scale-97` writes the `scale` property, so a finished keyframe `transform` never blocks it (map 11 §2.1).
- **Transitions:** `--dur-fast` for hover/colour, `--dur-base` for state, curve `--ease-out`. Never bare `ease` and never a raw `cubic-bezier(` (D-10 ratchet).
- **Reduced motion:** the P2 global net covers every CSS animation and transition. Exceptions are listed per part. JS helpers read `motion.reduced`.
- **Icons:** lucide, stroke 1.8, 16px beside text, 18px in nav (D-32).
- **Leave animations:** on the bits-ui content node only (§1.3 #2).

### 2.1 Layer map (z, D-08)

```
 70  tooltip layer (singleton)            ui/tooltip/tooltip-layer.svelte
 65  dropdown · popover · select · context · combobox list   (menus inside dialogs work: 65 > 50)
 60  toast stack                          ui/toast/toaster.svelte
 50  dialog · alert-dialog · palette · drawer  (overlay + content)
 20  HUDs (4b)      10 sticky      1 raised
```

### 2.2 CSS-only atoms

Each atom is one `ui/<name>/<name>.svelte` plus `index.ts`, with scoped CSS on attribute selectors (§1.3 #5) and `data-slot="<name>"` on the root. Props spread `...rest` onto the root, and `class` merges via `cn()`.

#### Button — `ui/button/button.svelte` (restyle the existing `tv()`, K10)

- **API:**

  ```ts
  variant?: "primary" | "secondary" | "ghost" | "danger" | "soft" | "warn" | "link"
  size?: "default" | "sm" | "icon" | "icon-sm"
  loading?: boolean
  href?: string
  type?: string
  disabled?: boolean
  ref
  children
  ```

  - **Defaults:** `variant="secondary"`, `size="default"`.
  - **Removed:** sizes `xs`, `lg`, `icon-xs` and `icon-lg`, and the `info` variant (D-03: info is a status tone, not an action). There are no app consumers (§1.3 #9). `/dev/ui` is updated in K36.
- **Base:** `inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 whitespace-nowrap border border-transparent text-small font-medium outline-none select-none transition duration-(--dur-fast) ease-out focus-visible:shadow-focus active:not-disabled:not-aria-[haspopup]:scale-97 disabled:cursor-not-allowed disabled:opacity-(--opacity-disabled) aria-disabled:pointer-events-none aria-disabled:opacity-(--opacity-disabled) aria-busy:cursor-progress [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4`
- **Sizes (density via vars):**

  | size | classes | compact | regular | comfy |
  |---|---|---|---|---|
  | default | `h-(--control-h) px-(--control-px) rounded-(--control-r)` | 32 · 12 · r8 | 36 · 14 · r12 | 44 · 16 · r12 |
  | sm (dense toolbar) | `h-(--control-dense-h) px-2.5 rounded-(--control-dense-r)` | 28 · r8 | 32 · r8 | 36 · r12 |
  | icon | `size-(--control-h) rounded-(--control-r)` | 32 | 36 | 44 |
  | icon-sm | `size-(--control-dense-h) rounded-(--control-dense-r)` | 28 (hit floor, D-32) | 32 | 36 |
- **Variants** (Halo `bundle.css:46-53`, Rift extras marked R):

  | variant | rest | hover | open (`aria-expanded`) |
  |---|---|---|---|
  | primary | `bg-accent text-accent-fg font-semibold` (#6aa8ff / #08090c, 600) | `shadow-glow` (0 8px 28px rgba(106,168,255,.28)), D-16 | same as hover |
  | secondary | `bg-surface-hover border-border-strong text-fg` (bg-200 / line-200 / ink) | `bg-surface-active` (bg-300) | `bg-surface-active` |
  | ghost | `bg-transparent text-fg-muted` | `bg-surface-hover text-fg` | `bg-surface-hover text-fg` |
  | danger | `bg-transparent border-border-strong text-danger` (#ff6b6b) | `bg-surface-hover border-danger` | n/a |
  | soft (R) | `bg-accent-soft text-accent` (14% wash). This is the "selected ghost" for toggled toolbar buttons | `bg-accent-soft` (unchanged) | n/a |
  | warn (R) | `border-border-strong text-warn` | `bg-surface-hover border-warn` | n/a |
  | link (R) | `h-auto border-0 bg-transparent px-0 text-accent underline-offset-4` | `underline` | n/a |
- **States:**
  - rest, hover and active (scale .97, except `aria-haspopup` triggers so menus don't jiggle);
  - focus-visible ring;
  - disabled .45;
  - `loading`: a Spinner 16 replaces the first icon, with `aria-busy="true"` and `aria-disabled="true"`. The label stays, so the width doesn't jump.
- **Motion:** `transition` (Tailwind's default property list includes `scale`, `box-shadow`, `background-color`, `border-color`, `color`) 150ms ease-out. The glow fades in over 150ms.
- **A11y:** a real `<button>` (or `<a>` when given `href`, as today at :61-73). The icon-only sizes need an accessible name; IconButton enforces it.
- **Ports:** `bundle.css:42-56,401-402`.
- **Retires:**
  - `.btn` (`app.css:394-421`, 22 files);
  - `.st-btn` (`settings-controls.css:16-27`);
  - `.ob-btn` (`onboarding.css:133-148`);
  - `.sxa-btn` (`stream.css:493-496`, StreamExitPlan only).

  Phases are in §8.

#### IconButton — `ui/button/icon-button.svelte` (new, K11)

- **API:**

  ```ts
  label: string             // required
  variant?: "ghost" | "secondary" | "soft" | "danger" = "ghost"
  size?: "default" | "sm" = "sm"
  kbd?: string
  pressed?: boolean | undefined
  tooltip?: boolean = true
  loading?: boolean
  children: Snippet         // the icon
  ```

  - **Behaviour:** it renders `Button` with `size = icon | icon-sm`, sets `aria-label={label}`, applies `use:tooltip={tooltip ? { text: label, kbd } : null}` and sets `aria-pressed` when `pressed` is defined.
  - **Defaults:** default size `sm` (28px in compact), because icon buttons live in toolbars.
- **Why a separate file:** it makes the name and the tooltip impossible to forget (D-17 scope "icon-only buttons", D-32 hit target).

#### Card — `ui/card/card.svelte` (new, K12)

- **API:**

  ```ts
  as?: "div" | "section" | "article" | "li" = "div"
  interactive?: boolean     // whole-card click target lift, D-16
  pad?: "default" | "none"
  class
  children
  ```
- **CSS:**

  ```css
  [data-slot=card] {
    position: relative;
    background: var(--bg-100);
    border: 1px solid var(--line-100);
    border-radius: var(--radius-lg);
    padding: var(--card-pad);
    transition: border-color var(--dur-base) var(--ease-out), translate var(--dur-fast) var(--ease-out);
  }
  ```

  | state | change |
  |---|---|
  | hover | `border-color: var(--line-200)` |
  | `[data-interactive]:hover` | `translate: 0 -3px` |
  | `[data-interactive]:focus-within` | `box-shadow: var(--shadow-focus)` |
  | `[data-pad=none]` | `padding: 0` |

  - Resolved values: bg-100 #0e1014, line-100 #1f242c, radius 16, padding 16/20/24 by density.
  - No shadow, no spotlight, no wash (D-16, D-31). The consumer puts a real `<a>`/`<button>` inside an interactive card.
- **Ports:** `bundle.css:29-32` (lines 33-35, the spotlight, are deliberately not ported).
- **Retires:** `.card` (`app.css:496-500`). Settings keeps `class="card"` on its Card instances as the D-33 hook (`#card-*` + `.card` + `.sflash`). A single-word class is not checked by `check:ui`.

#### Badge — `ui/badge/badge.svelte` (new, K13)

- **API:**

  ```ts
  tone?: "neutral" | "accent" | "info" | "warn" | "danger" = "neutral"
  dot?: boolean
  mono?: boolean
  children
  ```
- **CSS:**
  - `display: inline-flex; align-items: center; gap: 6px; height: var(--badge-h); padding: 0 8px; border-radius: var(--radius-pill); font: 500 var(--t-caption)/1 var(--font-sans); white-space: nowrap`;
  - `[data-mono]` gives `font-family: var(--font-mono); font-variant-numeric: tabular-nums`;
  - dot: 6px circle, `background: currentColor`.
- **Tones** (`bundle.css:231-235`):

  | tone | background | text |
  |---|---|---|
  | neutral | `--bg-300` #1b1f26 | `--ink-muted` #9aa2af |
  | accent | `--accent-soft` | `--accent` |
  | info | `--info-soft` | `--info` #9cc6ff (D-03) |
  | warn | `--warn-soft` | `--warn` #ff9f5a |
  | danger | `--danger-soft` | `--danger` |
- **Semantics:**
  - Rift "ok" maps to `accent`; log level "info" maps to `neutral` (D-03).
  - Never an action. Text is required; the tone is a second cue.
- **Ports:** `bundle.css:228-235`.
- **Retires:** `.pill` (`app.css:449-470`, 17 files), `.st-pill` (`settings-controls.css:30-37`), `.tile-pill` (`app.css:577-587`), `.ctx-chip`/`.branch-pill` (`app.css:654-686`; Badge for read-only chips, Button ghost sm for the button form).

#### Kbd — `ui/kbd/kbd.svelte` (new, K14)

- **API:** `keys?: string[]` or `children`.
- **Markup:** one `<kbd data-part="key">` per key. `keys` render in a `<span data-part="group">` with a 4px gap (`bundle.css:444`).
- **Key CSS:** `display: inline-flex; align-items: center; justify-content: center; min-width: 20px; height: 20px; padding: 0 6px; border-radius: var(--radius-xs) /*6*/; border: 1px solid var(--line-200); background: var(--bg-200); color: var(--ink-muted); font: 500 11px/1 var(--font-mono)` (D-06: 11 only for mono label and Kbd).
- **Ports:** `bundle.css:305,444`. Halo's 22px is changed to 20 so the key fits a 28px dense row (§14 Q4).
- **Retires:** `.kbd` (`app.css:357-367`, 12 markup uses) and the `menuShortcut` text recipe (`recipes.ts:48`). Menu shortcuts become `<Kbd>`.

#### LiveDot — `ui/live-dot/live-dot.svelte` (new, K15)

- **API:**

  ```ts
  tone?: "accent" | "info" | "warn" | "idle" = "accent"
  label?: string
  srLabel?: string          // required when label is absent
  ```
- **CSS** (`bundle.css:74-82`):
  - host `display: inline-flex; gap: 8px; font: 500 var(--t-caption)/16px var(--font-sans); color: var(--ink-muted)`;
  - dot `width: 8px; height: 8px; border-radius: 50%; background: var(--accent)`;
  - dot `::after { inset: 0; border-radius: 50%; background: inherit; animation: halo-ping 1.8s var(--ease-out) infinite }`, with `halo-ping` scaling 1→3 and fading opacity .7→0;
  - idle: `background: var(--ink-faint)`, `::after { animation: none }`;
  - info `--info`, warn `--warn`.
- **Reduced motion:** the net stops the ping and the dot stays solid (D-13; no exemption).
- **A11y:** the dot is `aria-hidden`. The label is visible text; otherwise an `srLabel` span is rendered.
- **Mapping:**
  - `--status-busy` → accent;
  - pending → idle;
  - paused/degraded → warn (D-03).
- **Retires:**
  - box-shadow pulses (map 09 §4.1 rule 5);
  - `.dc-statedot`, `.live-dot`, `gh-dot.busy` (`app.css:676`);
  - the 9-dot working grid (in 4b).

#### Spinner — `ui/spinner/spinner.svelte` (new, K15)

- **API:**

  ```ts
  size?: number = 16
  tone?: "accent" | "muted" | "current" = "accent"
  label?: string = "Loading"
  decorative?: boolean
  ```
- **Markup:** `<svg viewBox="0 0 24 24" width={size} height={size} data-essential-motion>` with a track `<circle cx=12 cy=12 r=9>` (stroke `--bg-300`, width 3, no fill) and an arc on the same circle (stroke `currentColor`, width 3, `stroke-linecap: round`, `stroke-dasharray: 18 100`).
- **Animation:** `svg { animation: halo-spin 900ms linear infinite }` (`bundle.css:446-451`).
- **Reduced-motion exemption** (scoped, so it beats the net's `*` selector with `!important`; D-13):

  ```css
  @media (prefers-reduced-motion: reduce) {
    svg[data-essential-motion] { animation-duration: 900ms !important; animation-iteration-count: infinite !important; }
  }
  ```
- **A11y:**
  - not decorative: `role="status"` + a visually hidden `label`;
  - decorative (inside a Button with `aria-busy`): `aria-hidden="true"`.
- **Retires:** 25 spin keyframes (P2 kills them; consumers swap per surface), `.spin` (`app.css:625,628`), `.st-spin`, `mcp-spin`, `chip-spin`.

#### Divider — `ui/separator/separator.svelte` (restyle the existing part, K16; no second component)

- **Addition:** a `label?: string` prop. With a label it renders `<div role="separator" aria-orientation="horizontal" data-slot="separator" data-labelled>`, with the label between two flex hairlines: `display: flex; align-items: center; gap: 12px; font: 500 11px/14px var(--font-mono); letter-spacing: .12em; text-transform: uppercase; color: var(--ink-faint)`. Each `::before`/`::after` is `flex: 1; height: 1px; background: var(--line-100)` (`bundle.css:437-441`).
- **Without a label:** today's classes (`separator.svelte:16-18`) already speak `bg-border` (line-100 after P1). Add `data-[orientation=vertical]:min-h-4`.
- **Export:** `index.ts` adds `Divider` as an alias export.
- **Retires:** `.divider` (`app.css:446`), `.rift-menu-divider` gradient (`app.css:710-714`).

#### SectionLabel — `ui/section-label/section-label.svelte` (new, K17)

- **API:**

  ```ts
  index?: number
  count?: string | number
  as?: "div" | "h2" | "h3" = "div"
  children
  ```
- **Markup:** `[index "01 /"] [text] [rule] [count]`.
- **CSS:**
  - host `display: flex; align-items: center; gap: 12px; font: 500 var(--t-label)/var(--t-label-lh) var(--font-mono); letter-spacing: .12em; text-transform: uppercase; color: var(--ink-faint)`;
  - index `color: var(--ink-faint)` (D-31: not accent);
  - rule `flex: 1 1 auto; height: 1px; background: var(--line-100)`;
  - count `color: var(--ink-muted)`.
- **Format:** the index is zero-padded to 2 (`01`).
- **Scope:** Home, Health and Settings. Never in chat (D-31).
- **Ports:** `bundle.css:84-88` (`:86` is changed).
- **Retires:** `.tile-label` (`app.css:575`), `.rift-menu-head` (`:705-709`), the PageHero eyebrow (`PageHero.svelte:43-44`).

#### EmptyState — `ui/empty-state/empty-state.svelte` (new, K18)

- **API:**

  ```ts
  title: string
  description?: string
  icon?: Snippet
  action?: Snippet
  compact?: boolean
  ground?: "100" | "000" = "100"
  ```
- **Layout:**
  - column centred, gap 12, padding 48/24 (compact 24/16);
  - mark 56px circle, `background: var(--bg-200)`, icon 24 in `--accent`;
  - rings `box-shadow: 0 0 0 10px var(--g), 0 0 0 11px var(--line-100), 0 0 0 22px var(--g), 0 0 0 23px var(--line-100)`, where `--g` is `--bg-100`, or `--bg-000` when `ground="000"`. Compact drops the outer ring.
- **Type:**
  - title `600 var(--t-heading)/var(--t-heading-lh)` (16/22), `--ink`;
  - description `400 var(--t-small)/var(--t-small-lh)` (13/18), `--ink-muted`, `max-width: 340px`.
- **Motion:** `animation: halo-rise var(--dur-slow) var(--ease-out) both` with `--rise: var(--rise-page)` (10px; D-10). D-29 `data-settled` zeroes it on re-mounts.
- **Copy rule:** say what appears here, with one action (D-32).
- **Ports:** `bundle.css:337-343`.
- **Retires:** the browser empty hero (4f), palette "No matches" (4f), MCP empty (4f), Home empties (4d).

#### Banner — `ui/banner/banner.svelte` (new, K19)

- **API:**

  ```ts
  tone?: "info" | "accent" | "warn" | "danger" = "info"
  title: string
  description?: string
  icon?: Snippet            // default per tone: Info, Check, TriangleAlert, OctagonAlert
  action?: Snippet
  onDismiss?: () => void
  ```
- **CSS** (`bundle.css:386-396`, `--tone` set per `data-tone`):

  | element | style |
  |---|---|
  | host | `display: flex; flex-wrap: wrap; align-items: center; gap: 12px; padding: 12px 12px 12px 16px; border-radius: var(--radius-md); color: var(--ink)` |
  | host background | `color-mix(in srgb, var(--tone) 8%, var(--bg-100))` |
  | host border | `1px solid color-mix(in srgb, var(--tone) 32%, var(--line-100))` |
  | icon chip | 28px circle, `background: color-mix(in srgb, var(--tone) 16%, transparent)`, colour `--tone` |
  | body | `flex: 1 1 240px` |
  | title | `500 var(--t-small)/18px` |
  | description | `400 var(--t-small)/18px`, `--ink-muted` |
- **Motion:**
  - enter `halo-rise var(--dur-base) var(--ease-out) both` with `--rise: var(--rise-page)`;
  - dismiss sets `data-leaving` (`opacity: 0; transform: translateY(-6px); transition: opacity 200ms var(--ease-out), transform 200ms var(--ease-out)`), and `onDismiss` fires on `transitionend` (fallback timeout 220ms). D-20: rises 300, leaves 200.
- **A11y:**
  - `danger` gets `role="alert"`; the other tones get `role="status" aria-live="polite"`;
  - dismiss is an IconButton `label="Dismiss"`.
- **Retires:** UpdateBanner chrome (4f), the AiHealth verdict strip / `.ah-advice-err` (4f), the new `max_tokens` notice (4b, D-21).

#### Skeleton — `ui/skeleton/skeleton.svelte` (new, K20; D-14)

- **API:**

  ```ts
  loading: boolean
  variant?: "text" | "stat" | "list" = "text"
  lines?: number = 3
  delay?: number = 300
  children: Snippet
  ```
- **Behaviour state machine:**

  | state | condition | renders |
  |---|---|---|
  | `idle` | `loading` is false and bones never showed | children, no animation (fast loads never flash, D-14) |
  | `waiting` | `loading` is true, waiting `delay` ms | nothing, plus `aria-busy` |
  | `bones` | `delay` passed while still loading | bones, `aria-busy="true"`, bones `aria-hidden` |
  | `handoff` | `loading` becomes false from `bones` | children (`halo-reveal`), with bones overlaid absolutely (`halo-skel-out`) |

  - `handoff` runs 600ms, then returns to `idle`.
- **Bones:**
  - each bar: `background: var(--bg-200); height: 12px; border-radius: var(--radius-xs)` (6);
  - text widths cycle 100/82/64/90/72%, gap 8;
  - `stat`: label bar 40% · chip 48×22 pill · value bar 60% × 28px;
  - `list`: rows of a 36px circle plus two bars (70%/45%), gap 16.
  - **No sweep and no breathing** (D-14; M2).
- **Hand-off motion** (Halo exact, `bundle.css:103-107`):
  - content `animation: halo-reveal var(--dur-slow) var(--ease-out) both` (from opacity 0, blur 6px, translateY 6px);
  - bones `animation: halo-skel-out 360ms var(--ease-out) forwards` (to opacity 0, blur 4px), removed on `animationend`.
  - This is the only blur in the kit (D-15).
- **Reduced motion:** the net makes the swap instant.
- **Ports:** `bundle.css:98-114` without the `::after` shimmer (`:101-102`).
- **Retires:** `shell/Skeleton.svelte` (57; sweep + reduced-motion breathe), `ws-load-shimmer`, `skel-shimmer`, `mcp-shimmer`, `ah-skel-pulse` (surfaces 4d/4f).

#### Segmented — `ui/segmented/segmented.svelte` (new, K21; D-31)

- **API:**

  ```ts
  options: Array<string | { value: string; label: string; disabled?: boolean }>
  value: string                    // $bindable
  onValueChange?: (v: string) => void
  ariaLabel: string
  size?: "default" | "sm"
  tone?: "neutral" | "accent" = "neutral"
  mono?: boolean
  disabled?: boolean
  ```
- **Wireframe:**

  ```
  ┌───────────────────────────────────────┐  h = --control-h (sm: --control-dense-h)
  │▕████ Compact ████▏ Regular    Comfy   │  pad 3, gap 2, outer r = --control-r
  └───────────────────────────────────────┘  thumb r = calc(--control-r - 3px)
  ```
- **CSS:**
  - host `position: relative; display: inline-flex; padding: 3px; gap: 2px; background: var(--bg-100); border: 1px solid var(--line-100); border-radius: var(--control-r); height: var(--control-h)`;
  - option `position: relative; z-index: 1; min-width: 52px; padding: 0 12px; border-radius: calc(var(--control-r) - 3px); color: var(--ink-muted); font: 500 var(--t-small)/1 var(--font-sans)`;
  - mono: `600 … var(--font-mono)`;
  - hover `color: var(--ink)`;
  - checked `color: var(--ink)`, or `color: var(--accent-ink)` with the accent tone.
- **Thumb:**
  - `position: absolute; top: 3px; bottom: 3px; left: 0; translate: var(--ind-x) 0; width: var(--ind-w); background: var(--bg-300); box-shadow: inset 0 0 0 1px var(--line-200)`;
  - `[data-tone=accent]` gives `background: var(--accent); box-shadow: none`;
  - transition `translate var(--dur-base) var(--ease-out), width var(--dur-base) var(--ease-out)`, enabled only after the first measure (`data-ready`), so it never slides in from 0.
- **Neutral thumb by default:** see §13 A-6 (accent scarcity: Settings has 7 segmented controls).
- **A11y (D-31 overrides Halo's `aria-pressed`, `bundle.css:95`):**
  - root `role="radiogroup" aria-label`;
  - each option `<button type="button" role="radio" aria-checked tabindex={checked ? 0 : -1}`. If none is checked, the first enabled option gets 0;
  - keys exactly as `SettingsPage.svelte:242-257`: ArrowRight/ArrowDown next, ArrowLeft/ArrowUp previous (wrapping), Home and End, skipping disabled options. Each key focuses AND selects.
- **Ports:** `bundle.css:90-96`.
- **Retires:**
  - Settings `.seg` plus the local `onRadioKey` (4e);
  - `.ob-seg`/`.ob-seg-btn` (`onboarding.css:209-235`, 4d);
  - AiHealth `.ah-range` and Diagnostics level segments (4f);
  - WorkspacePage range tabs (4d).

#### SearchField — `ui/search-field/search-field.svelte` (new, K22)

- **API:**

  ```ts
  value: string                    // $bindable
  placeholder?: string
  label?: string = "Search"
  size?: "default" | "sm"
  shortcut?: string[]
  loading?: boolean
  clearable?: boolean = true
  disabled?: boolean
  onSubmit?: (v: string) => void
  inputRef                         // $bindable
  ```
- **Wireframe:**

  ```
  ┌─[⌕]─ Search settings…─────────────[Ctrl][K]┐   empty
  ┌─[⌕]─ density▌───────────────────────────[×]┐   with text: × pops in
  ```
- **CSS:**
  - input `height: var(--control-h)` (sm: `--control-dense-h`); `padding: 0 36px 0 34px; background: var(--bg-100); border: 1px solid var(--line-300); border-radius: var(--control-r); font: 400 var(--t-small)/18px var(--font-sans); color: var(--ink)`;
  - placeholder `--ink-faint`;
  - hover `background: var(--bg-200)`;
  - focus `border-color: var(--accent); box-shadow: var(--shadow-focus); background: var(--bg-100)`;
  - icon 16 at left 10, `--ink-muted`, `--accent` on `:focus-within`;
  - clear button 24×24 (sm 20×20) at right 4, radius-sm, `--ink-muted`, hover `bg-300` + ink, enters with `halo-pop-in` 150ms;
  - Kbd group at right 8, shown only while empty and not focused.
- **Keys:**
  - Esc with text clears and calls `stopPropagation`;
  - Esc when empty bubbles, so a dialog can close;
  - Enter calls `onSubmit`.
- **Loading:** a Spinner 14 replaces the icon.
- **A11y:** `<input type="search" aria-label={label}>`; native cancel/decoration hidden (`bundle.css:604`).
- **Ports:** `bundle.css:597-612`.
- **Retires:** `.dc-search` (4f), `.sset-search` (4e), the browser find input (4f).

#### Field · Input · Textarea — `ui/field/field.svelte`, `ui/input/input.svelte`, `ui/textarea/textarea.svelte` (new, K23)

Halo's TextField is a 52px floating-label input (`bundle.css:123-138`). It is too tall for compact and wrong for an omnibox (map 07 §7). This spec ports the plain Halo input (`bundle.css:842-847`) plus the Field wrapper (`:830-841`) instead.

- **Recipes** (added to `recipes.ts`, so bespoke inputs outside `ui/` can use them too):

  | recipe | classes |
  |---|---|
  | `fieldInput` | `h-(--control-h) w-full rounded-(--control-r) border border-field-border bg-surface px-(--control-px) text-small text-fg outline-none transition duration-(--dur-fast) ease-out placeholder:text-fg-subtle hover:not-disabled:bg-surface-hover focus:border-accent focus:bg-surface focus:shadow-focus disabled:cursor-not-allowed disabled:opacity-(--opacity-disabled) aria-invalid:border-danger` |
  | `fieldTextarea` | `fieldInput` with `h-auto min-h-22 py-2.5 resize-y field-sizing-content max-h-90` (88px min, 360px max, Halo `:586`, Textarea README) |
  | `fieldInputSm` | `h-(--control-dense-h) rounded-(--control-dense-r) px-2.5` |
- **Field API:**

  ```ts
  label: string
  hint?: string
  error?: string
  required?: boolean
  optional?: boolean
  action?: Snippet
  children: Snippet<[{ id: string; describedBy: string | undefined; invalid: boolean }]>
  ```

  It generates `id` with `$props.id()` and renders:

  | part | style |
  |---|---|
  | label row | `<label for={id}>`, `500 var(--t-small)/18px`, `--ink-muted` |
  | tag ("Required"/"Optional") | `--t-caption`, `--ink-faint` |
  | message (hint) | `400 var(--t-caption)/16px`, `--ink-muted` |
  | message (error) | same size, `--danger`, with a 14px CircleAlert icon |

  Error takes the place of the hint and sets `invalid`.
- **Input:** `<input class={cn(fieldInput, size==="sm" && fieldInputSm, className)} {...rest}>`, with `value` and `ref` bindable.
- **Textarea:**
  - `maxLength` shows a mono caption counter;
  - colour `--ink-faint`, then `--warn` at ≥90%, then `--danger` at 100% (`bundle.css:593-595`).
- **SecretField** (eye toggle) is a Settings composite in 4e; it is built from Input + IconButton.
- **Retires:**
  - `.input` (`app.css:423-434`, 10 files incl. `dev/ui`);
  - `.field-label`, `.help`, `.dim` (`:436-444`);
  - `.st-input` and `.st-secret` (`settings-controls.css:7-13`);
  - `.ob-input`;
  - `fieldTrigger` (`recipes.ts:52-53` becomes `fieldInput` + `flex items-center justify-between gap-2`).

#### StatTile — `ui/stat/stat-tile.svelte` (new, K25; D-24, D-16, D-17)

- **API:**

  ```ts
  label: string
  value: number | null
  format: (n: number) => string            // Rift formatter: "32k", "$8.2k", "7.2s"
  exact?: (n: number) => string            // full value; shown on tooltip when it differs from format()
  detail?: string | Snippet
  delta?: { value: number; goodWhen?: "up" | "down"; format?: (n: number) => string }
  rangeKey?: unknown                       // change => count-up (D-16)
  countUp?: boolean = true
  roll?: boolean                           // use RollingNumber (live counters only)
  peek?: Array<{ label: string; value: string }>
  loading?: boolean                        // Skeleton variant="stat"
  children?: Snippet                       // sparkline slot, right of the value
  ```
- **Wireframe:**

  ```
  Tokens today                    ↑ +12.4%   ← label small 13/18 ink-muted · delta chip
  32.4k  ╱╲╱                                 ← metric 28/32 mono 600 −0.03em ink · sparkline slot
  of 1.2M weekly limit                       ← detail caption 12/16 ink-muted
  ```
- **CSS:**
  - host `display: flex; flex-direction: column; gap: 8px; border-radius: var(--radius-sm)`;
  - focusable (`tabindex=0`) only when `peek` or `exact` is given, with `:focus-visible` `box-shadow: var(--shadow-focus)` (`bundle.css:59-60`);
  - value `font: 600 var(--t-metric)/var(--t-metric-lh) var(--font-mono); letter-spacing: -0.03em; font-variant-numeric: tabular-nums; white-space: nowrap`.
- **Delta chip:**
  - shape: `display: inline-flex; gap: 4px; padding: 4px 8px; border-radius: var(--radius-pill); font: 600 12px/1 var(--font-mono)`;
  - content: a lucide `ArrowUp`/`ArrowDown` (12px) plus a signed value (`+12.4%`, `−0.6%`); the arrow and the sign carry meaning (Halo content rule);
  - tone: good gives `accent-soft`/`accent`, bad gives `warn-soft`/`warn`. "Good" is `value > 0` when `goodWhen="up"` (default) and `value < 0` when `goodWhen="down"` (cost metrics, D-03).
- **Count-up:** a `CountUp` helper (§2.5), 700ms easeOutCubic. It animates from 0 on the first non-null value and from the old value on a `rangeKey` change. Any other value change (60s/30s ticks) snaps, per D-16.
- **Exact value:** `use:tooltip={exact && exact(value) !== format(value) ? exact(value) : null}` on the value span. The value span always carries `aria-label={exact?.(value) ?? format(value)}` and the animating digits are `aria-hidden`.
- **Peek:** shown on hover/focus. Same pop surface as menus (`bg-200`, `line-200`, `shadow-pop`, radius 12, padding 12/16, `min-width: 220px`), `z-index: var(--z-dropdown)`, opacity and `translateY(-4px)` to none over 150ms (`bundle.css:69-72`).
- **Removed vs Halo:** the en-US `$` format (D-24), and the 38px gradient numeral (D7).
- **Ports:** `bundle.css:58-72`.
- **Retires:** `.ah-tile` (4f), the Home hub stats (4d), `AnimatedCount` in hubs (4d/4f).

#### RollingNumber — `ui/rolling-number/rolling-number.svelte` (new, K29; D-16)

- **API:**

  ```ts
  value: number
  format?: (n: number) => string
  live?: boolean = true            // false snaps, as AnimatedCount does today
  monotonic?: boolean = true       // ignore decreases while live (X42 guard)
  ```
- **Markup:** one `<span data-part="col">` per character. Digit columns hold a vertical strip of 0-9. Non-digits render static (`data-part="sym"`).
- **CSS:**
  - host `display: inline-flex; align-items: baseline; font-variant-numeric: tabular-nums; --roll-h: 1.15em; line-height: var(--roll-h)`;
  - col `display: inline-block; height: var(--roll-h); overflow: hidden; vertical-align: bottom`;
  - strip `translate: 0 calc(var(--d) * var(--roll-h) * -1); transition: translate 900ms var(--ease-out) var(--delay)`, where `--delay = min(posFromRight, 8) * 45ms` (right-most digit first);
  - opacity floor .6 on a changing digit (X42).
- **A11y:** host `aria-label={format(value)}`, columns `aria-hidden`.
- **Reduced motion:** the net makes the transition instant.
- **Baseline:** `AnimatedCount.svelte:83-97` proves an inline-block sits about 2.4px off the text baseline. RollingNumber keeps `vertical-align: bottom` with the column height equal to the line-height. The verify step measures the delta against the adjacent unit text with `c.sh measure` (§10).
- **Ports:** `bundle.css:734-740`.
- **Replaces:** `AnimatedCount.svelte` (113) at its two uses, the live token readout and diff `+N/−M` (4b). Deletion is in §8.

#### ShareBar — `src/lib/components/shared/ShareBar.svelte` (new, K26; D-24)

It lives in `shared/` (beside `LimitBar`, `PageHero`), not in `ui/`, because it is a Rift composite used by two surfaces, Home (4d) and Health (4f). See §13 A-7.

- **API:**

  ```ts
  items: Array<{ label: string; value: number }>
  format: (n: number) => string
  exact?: (n: number) => string
  top?: number = 3
  ariaLabel: string
  rangeKey?: unknown
  ```
- **Data rule:** items are sorted descending. The top 3 are coloured `--chart-1` #6aa8ff, `--chart-2` #b79cff and `--chart-3` #9cc6ff; the rest collapse into "Other".
- **Bar:**
  - track: height 8px, `border-radius: var(--radius-pill)`, `overflow: hidden`, `background: var(--bg-300)`, flex row with 2px gaps;
  - "Other": `--line-200`, so it stays visible on the `bg-300` track (§13 A-7);
  - segments: `flex-basis` = share %, `transform-origin: left`;
  - mount and `rangeKey` change: `scale: 0 1` → `1 1` over 600ms `--ease-out`, staggered 60ms per segment;
  - hover a segment: siblings dim to .45 over `--dur-base` (`bundle.css:210-211`) and the segment shows `use:tooltip` with exact value + %.
- **Legend:** a `<ul>`. Each row is an 8px dot in the segment colour, the label in `--t-small` `--ink-muted`, and the value in mono `--t-data` `--ink`, right-aligned. The legend is the accessible value list; the bar is `aria-hidden`.

#### Progress — `ui/progress/` (`npx shadcn-svelte@latest add progress`, restyled, K27; D-24, D-13)

- **API (wrapper `progress.svelte` over bits-ui Progress):**

  ```ts
  value?: number | null            // 0–1; null => indeterminate
  label?: string
  showValue?: boolean = true
  format?: (v: number) => string = (v) => `${Math.round(v * 100)}%`
  tone?: "accent" | "info" | "warn" | "danger" = "accent"
  size?: "default" | "sm" = "default"
  ariaLabel?: string
  ```
- **Recipe:**

  | part | classes |
  |---|---|
  | top row | `flex justify-between items-baseline gap-3` |
  | label | `text-small text-fg-muted` |
  | value | `font-mono text-data font-semibold text-fg tabular-nums` |
  | track | `relative h-2 data-[size=sm]:h-1 overflow-hidden rounded-full bg-surface-active` (8px / 4px on bg-300) |
  | indicator | `absolute inset-0 rounded-[inherit] origin-left`, with inline `style="transform: scaleX({value})"` and `transition: transform 700ms var(--ease-out)` |

  - Tone backgrounds use `data-tone` classes: `bg-accent`, `data-[tone=warn]:bg-warn`, …
  - The value text counts with the bar through CountUp 700ms (Halo README "the mono percentage counts with it").
- **Indeterminate:** indicator `w-2/5 animate-[…]`. That would be arbitrary, so it is a bridge key instead: `--animate-indet: halo-indet 1.3s var(--ease-out) infinite` (`bundle.css:460-461`).
  - Under reduced motion the net leaves one 0.01ms iteration. With no fill-mode the bar returns to its base `transform: none`, a static 40% bar at the left. That is an honest "working" state with no exemption needed (D-13).
- **A11y:** `role=progressbar` with `aria-valuenow/min/max` from bits-ui. The label is linked through `aria-labelledby`. Indeterminate omits `aria-valuenow`.
- **Ports:** `bundle.css:453-461`.
- **Retires:** `LimitBar` (3 variants, pulse and sheen, `LimitBar.svelte:38-80`) in 4d/4f, `.ah-track/.ah-fill`, `.hero-fill` (UpdateDialog), and the browser comet `.wb-progress` (4f).

#### PageHeader — `ui/page-header/page-header.svelte` (new, K28; D-31)

- **API:**

  ```ts
  title: string
  lede?: string
  eyebrow?: { index?: number; label: string }   // rendered through SectionLabel
  maxWidth?: 820 | 1080 | number = 820
  actions?: Snippet
  children?: Snippet                             // tab bar slot
  ```
- **Layout:**

  ```
  │← pad 32 →│ 01 / SETTINGS ───────────────────            │
  │          │ Settings                       [actions]     │   title 24/30 600 −0.02em ink
  │          │ Accounts, models and how Rift looks.         │   lede 13/18 ink-muted, max 64ch
  │          │ [tab bar slot]                               │
  ```
- **CSS:**
  - host `padding: 24px 32px 0`; inner `max-width: var(--page-col, 820px); margin: 0 auto`;
  - `--page-col` is published on the host, so the page body below reads the same var. That gives one column edge (L18).
  - No accent eyebrow bar and no 760 weight (today `PageHero.svelte:43-45`).
- **Note:** board 05/06/07 values for the top/side padding replace 24/32 when exported (§9).
- **Retires:** `shared/PageHero.svelte` (48) once AiHealth, Diagnostics and Settings port (4e/4f).

#### Collapse — `ui/collapse/collapse.svelte` (new, K30; D-12 "one accordion for every disclosure")

The motion half of Accordion, with no trigger semantics. Bespoke disclosures (stream tiles, WorkLine, settings expanders driven elsewhere) use it so every disclosure in the app moves the same way.

- **API:**

  ```ts
  open: boolean
  id?: string
  labelledby?: string
  children: Snippet
  ```
- **CSS:**
  - root `display: grid; grid-template-rows: 0fr; transition: grid-template-rows var(--dur-base) var(--ease-out)`, with `[data-open]` giving `1fr`;
  - inner `min-height: 0; overflow: hidden`;
  - content `opacity: 0; translate: 0 -4px; transition: opacity var(--dur-base) var(--ease-out), translate var(--dur-base) var(--ease-out)`, with `[data-open]` giving `opacity: 1; translate: 0 0; transition-delay: 60ms`;
  - closed root carries `inert` (Tab skips it).
- **Ports:** `bundle.css:499-504`.
- **Replaces:** 10 `transition:slide` sites plus `workOpen`/`gcReveal`/`bodyFadeIn` (map 09 §4.1 rule 7), in 4b/4c.

### 2.3 Toast stack — `ui/toast/toaster.svelte` (new, K33-K34b; D-20)

**Store contract (unchanged; every call site keeps working):**

- `toast.push(opts) → id`;
- `dismiss(id, callHandler?)`, `pause(id)`, `resume(id)`, `clear()`;
- `items`, `history`, `centerOpen`, `unreadCount`;
- `notify.ok|info|warn|danger(title, opts?)`.

**Store additions (K33, `state/toast.svelte.ts`):**

```ts
pauseAll(): void   // pause every running timer (stack hovered/focused) — loops pause(id)
resumeAll(): void  // resume every paused one — loops resume(id)
export function toneOf(s: ToastSeverity): "accent" | "info" | "warn" | "danger" | "neutral"
// ok→accent · info→info · warn→warn · danger→danger · muted→neutral   (D-03)
```

**Behaviour change, separate commit (K34):** D-20 says errors are sticky, so `defaultTimeout("danger")` stops arming a timer and `push` treats `severity: "danger"` as sticky unless `timeoutMs` is given. Today danger auto-dismisses after 8000ms (`toast.svelte.ts:67`). It is revertable alone.

**Placement.** This keeps today's top-centre of the main island (`ToastHost.svelte:91-94`):

```css
position: fixed;
top: calc(var(--titlebar-h) + 20px);   /* = 54px, today's literal */
left: calc((100vw + var(--rail-w, 0px)) / 2);
translate: -50% 0;
width: min(360px, calc(100vw - var(--rail-w, 0px) - 32px));
z-index: var(--z-toast);                /* 60 */
```

A `contained` prop switches to `position: absolute` for `/dev/ui`.

**Stack math.** Top-anchored, newest first, `i = 0` is newest. Halo's bottom-anchored math (`bundle.css:318-323`) is mirrored on the y axis.

| | collapsed (rest) | expanded (stack `:hover` / `:focus-within`, or `expanded` prop) |
|---|---|---|
| `--y` | `i * 10px` (older ones peek below) | sum of the heights of toasts `0…i-1` plus `8px * i` |
| `--s` | `max(.86, 1 - .05 * i)` | 1 |
| `--o` | 1 for i < 3, else 0 | 1 |
| children of `i > 0` | `opacity: 0` (`bundle.css:320`) | 1 |
| container height | `h0 + 10px * min(n-1, 2)` | full sum, `transition: height var(--dur-base) var(--ease-out)` |

Each toast: `position: absolute; top: 0; left: 0; right: 0; transform-origin: 50% 0; transform: translateX(var(--dx, 0)) translateY(var(--y)) scale(var(--s)); opacity: calc(var(--o) * var(--fade, 1)); transition: transform var(--dur-base) var(--ease-out), opacity var(--dur-base) var(--ease-out)`. Heights are measured with `bind:offsetHeight` per toast.

**Motion:**

| event | animation |
|---|---|
| enter | `halo-toast-in-top var(--dur-base) var(--ease-out)`: from opacity 0, translateY(−16px), scale .96 (D-20 "enter from −16px over 300ms") |
| leave (×, timeout) | `opacity: 0; transform: translateX(calc(var(--dx) + 32px)) …` over 240ms `--ease-out` (`bundle.css:322`), then `toast.dismiss(id)` |
| swipe | pointerdown on the toast body (not on its buttons) captures the pointer; `--dx = max(0, pointerX - startX)` with no transition while dragging (`bundle.css:319`) |

- **Swipe release:**
  - `dx > 90` or velocity > 0.6px/ms: `translateX(110%)` over 240ms, then dismiss;
  - otherwise it springs back (`--dx: 0` with `transition: transform var(--dur-base) var(--ease-spring)`).
- **Hover/focus:** hovering or focusing the stack fans it out (`expanded`) and calls `toast.pauseAll()`. Leaving calls `resumeAll()`.

**Toast anatomy:**

```
┌──────────────────────────────────────────────┐  bg-200 · 1px line-200 · r12 · shadow-pop
│ ●  Saved settings                     [Undo] [×]│  padding 12 12 12 16 · gap 12
│    Density and fonts apply now.               │  dot 8px tone · title 500 13/18 ink
└──────────────────────────────────────────────┘  detail 12/16 ink-muted, 2-line clamp, click to expand (kept from ToastHost:22-31)
```

- **Dot colour:** by `toneOf`; neutral uses `--ink-faint`.
- **Action:** `item.action` renders as a Button `variant="ghost" size="sm"`. Clicking the body no longer fires it; only the button does (§14 Q6).
- **Close:** an IconButton `label="Dismiss"` that shows on hover and `:focus-within` (D-30) and is always visible to keyboard.
- **Detail:** `mono` keeps the mono detail.

**A11y:**

- container `role="region" aria-label="Notifications"`;
- each toast `role="status"` (polite), or `role="alert"` for danger;
- Esc inside a focused toast dismisses it;
- swipe always has the × as its keyboard path.

**Reduced motion:** enter, leave and fan are instant (net). Drag still follows the pointer, which is input, not animation.

**Ports:** `bundle.css:313-335`.

**Retires:** `components/ToastHost.svelte` (212), when AppShell swaps the mount in 4f. `styles/severity-tone.css` (14) goes when NotificationCenter also ports (4a bell).

### 2.4 Tooltip singleton — `actions/tooltip.ts` (rewrite) + `state/tooltip.svelte.ts` + `ui/tooltip/tooltip-layer.svelte` (K31-K32; D-17)

**Engine choice:** a port of ConsoleX's singleton (`consolex/panel/src/lib/tip.svelte.ts:1-135`, which has a pure unit-testable `place()`) with D-17's numbers. bits-ui Tooltip is per-trigger (Root/Trigger/Content), the wrong shape for 174 action sites. No floating-ui: `place()` handles top/bottom flip and viewport clamp, which is all `TooltipOpts.placement` allows (§13 A-3).

**Action API (unchanged type, `tooltip.ts:10-17` verbatim):**

```ts
type TooltipOpts = string | { text: string; placement?: "top" | "bottom"; delay?: number; kbd?: string };
export function tooltip(node: HTMLElement, opts: TooltipOpts | null | undefined | false): ActionReturn;
```

`null`/`false`/`""` means no tooltip (new; used by StatTile).

**Pure module `src/lib/motion/tip.ts` (K31, unit-tested):**

```ts
export const TIP = { COLD_MS: 400, WARM_MS: 120, WARM_WINDOW_MS: 500, GAP: 8, MARGIN: 8 } as const;
export function delayFor(now: number, lastHideAt: number, override?: number): number
  // override ?? (now - lastHideAt <= WARM_WINDOW_MS ? WARM_MS : COLD_MS)
export function place(anchor: Rect, tip: {w:number;h:number}, vp: {w:number;h:number},
                      side: "top" | "bottom" = "top", avoid?: Rect | null)
  : { x: number; y: number; side: "top" | "bottom" } | null
  // CX place() + `avoid`: if the bubble rect intersects `avoid` (the .wb-stage rect while a page is open)
  // on the preferred side, try the other side; if both intersect, return null (no tooltip, D-17)
export function repeatsVisibleText(text: string, nodeText: string): boolean
  // normalised (trim, lowercase, collapse spaces) nodeText includes text → true (D-17 "skip when it repeats")
```

**Store `state/tooltip.svelte.ts`:**

```ts
class TooltipState {
  layerMounted = $state(false);
  spec = $state<{ text: string; kbd?: string; side: "top"|"bottom" } | null>(null);
  anchor = $state<HTMLElement | null>(null);
  n = $state(0);            // bumps per new anchor → replays the rise
  lastHideAt = -Infinity;
}
export const tooltipState = new TooltipState();
export const TOOLTIP_ID = "rift-tooltip";
```

**Action behaviour, keeping the existing a11y promise:**

1. **On create and update:**
   - remove `title` (as today, :27);
   - if the node has no `aria-label`/`aria-labelledby` and no text, set `aria-label=text` and remember `promoted = true` (today :28-34; removed again on destroy if we set it).
2. **Show triggers:**
   - `pointerenter` with `pointerType !== "touch"` arms `setTimeout(show, delayFor(...))`;
   - `focus` shows immediately, but only if `node.matches(":focus-visible")`.
3. **Show** only when `tooltipState.layerMounted`, the text is non-empty, and `!repeatsVisibleText(text, node.textContent)`.
   - Sets `tooltipState.spec/anchor` and `n++` if the anchor changed.
   - Sets `aria-describedby=TOOLTIP_ID` unless `promoted` (the text already is the name; describing would announce it twice).
4. **Hide immediately** (D-17, no exit animation) on `pointerleave`, `blur`, `pointerdown` on the anchor, Escape (window capture, not stopped, so a dialog's Esc still works), any scroll whose target contains the anchor (capture), window `blur`, and destroy.
   - It records `lastHideAt = performance.now()`.
5. **Native browser stage:** if the anchor is inside `.wb-stage`, don't show. The layer passes `.wb-stage`'s rect as `avoid` when the stage exists and has non-zero size (D-17 "suppressed over .wb-stage").

**Layer `ui/tooltip/tooltip-layer.svelte`:**

- `onMount` sets `layerMounted = true` and back to false on destroy.
- It renders `{#key tooltipState.n}<div id="rift-tooltip" role="tooltip" data-side={side} style="left:{x}px;top:{y}px">{text}{#if kbd}<Kbd keys={[kbd]}/>{/if}</div>{/key}`.
- It positions after measuring its own box with a double rAF: render hidden, measure, then `place()`.

**Look (D-17):**

```css
position: fixed;
z-index: var(--z-tooltip);       /* 70 */
max-width: var(--tooltip-max-w); /* 240 */
padding: 6px 10px;
border-radius: var(--radius-sm); /* 8 */
background: var(--ink);          /* #eceff3 */
color: var(--bg-000);            /* #08090c */
font: 500 var(--t-caption)/16px var(--font-sans);
text-align: center;
pointer-events: none;
white-space: normal;
```

- The Kbd inside uses `--bg-300`/`--ink`, so it reads on the light bubble.
- **Motion:** `animation: halo-tip-in var(--dur-fast) var(--ease-out) both`. `--tip-rise` is 4px when the side is top and −4px when bottom (`bundle.css:309-311`). Instant under reduced motion.
- **Mount points:**
  - P3: `/dev/ui` only (K37).
  - P4a: one commit moves the mount to `routes/+layout.svelte` beside `<ContextMenuHost />` (`:62`), so the app, `/dev/gallery` and `/dev/ui` all get it, and removes the `/dev/ui` local mount.

Until P4a, the 174 app sites behave exactly as today (a11y-only), because `layerMounted` is false.

### 2.4b bits-ui-backed primitives

All of these are Tailwind recipe strings in `recipes.ts`, so `check:ui` validates them. Add new ones with `npx shadcn-svelte@latest add <name>`, never `init` (D-09).

#### Shared floating recipes (K05-K06, `recipes.ts`)

```ts
export const floatingPanel =
  "z-(--z-dropdown) rounded-md border border-border-strong bg-surface p-1 text-fg shadow-pop outline-none";
  // bg-100 #0e1014 · line-200 #2c323c · r12 · 0 12px 32px rgba(0,0,0,.45) · bundle.css:406 (.halo-pop)
export const floatingMotion =
  "data-[state=open]:not-data-[side=top]:animate-pop-in data-[state=open]:data-[side=top]:animate-pop-in-up data-[state=closed]:animate-pop-out";
  // open 150ms from opacity 0 · scale .96 · translateY(∓4px) · origin = --bits-*-content-transform-origin; leave 140ms opacity + scale .97
export const menuItem =
  "group/menu-item relative flex min-h-row w-full cursor-default select-none items-center gap-3 rounded-sm px-3 text-small text-fg-muted outline-none transition-colors duration-(--dur-fast) ease-out animate-menu-row data-highlighted:bg-surface-hover data-highlighted:text-fg data-disabled:pointer-events-none data-disabled:opacity-(--opacity-disabled) data-inset:pl-9 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 [&_svg:not([class*='text-'])]:text-fg-muted data-highlighted:[&_svg:not([class*='text-'])]:text-accent";
  // row 32/36/40 (--row-h) · r8 · 13/18 · ink-muted → ink + bg-200 + accent icon on highlight · bundle.css:419-427
export const menuItemDanger =
  "data-[variant=danger]:text-danger data-[variant=danger]:[&_svg:not([class*='text-'])]:text-danger data-[variant=danger]:data-highlighted:[&_svg:not([class*='text-'])]:text-danger";
  // Halo .is-danger: danger text, bg-200 highlight (no danger wash) · bundle.css:423-424
export const menuItemChecked = "data-[state=checked]:text-fg";      // no accent wash (D-02 scarcity); accent check icon carries it
export const selectItemSelected = "data-selected:text-fg";
export const menuCheck = "ml-auto size-4 text-accent";
export const menuLabel = "px-3 pt-2 pb-1 font-mono text-label uppercase text-fg-subtle data-inset:pl-9"; // 11/14 500 .12em ink-faint · bundle.css:430
export const menuSeparator = "mx-2 my-1 h-px bg-border";            // bundle.css:429
export const menuShortcut = "ml-auto pl-4";                          // wraps <Kbd>
export const fieldTrigger = fieldInput + " flex items-center justify-between gap-2 select-none data-[state=open]:border-accent data-placeholder:text-fg-subtle [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";
```

- **Content components:**
  - Each content component (`dropdown-menu-content.svelte:26-31`, `context-menu-content.svelte:23-28`, `select-content.svelte:30-35`, the sub-contents) keeps its `origin-(--bits-*-content-transform-origin)` and `max-h-(--bits-*-available-height)` classes.
  - It changes `min-w-40` to `min-w-55` (220px, `bundle.css:418`) and `sideOffset` from 4 to 8 (`bundle.css:406` `calc(100% + 8px)`).
  - It adds the `data-menu-stagger` attribute: on the content element for menus, and on `SelectPrimitive.Viewport` (`select-content.svelte:39`) for select. Group components add it too.
- **Select trigger:** the chevron classes change to `text-fg-muted transition-transform duration-(--dur-base) ease-out in-data-[state=open]:rotate-180 in-data-[state=open]:text-accent` (`bundle.css:170-171,798`). `data-[size=sm]` gives `h-(--control-dense-h) rounded-(--control-dense-r)`.
- **Slash and Mention popovers:** D-20 says they appear all at once (map 09 D13). Their bespoke listboxes (4c) simply do not set `data-menu-stagger`.
- **Recipes for bespoke listboxes:** the textarea-driven popovers (Slash, Mention, SettingsMenu, UsagePanel; focus stays in the composer, so they can't be bits-ui menus) import `floatingPanel`, `menuItem` and `menuLabel` directly. Highlight comes from `data-highlighted` set by their own code. This is how `.rift-menu*` (`app.css:695-736`) retires in 4c without a fifth dialect.
- **States:** closed (unmounted), opening (pop-in 150), open, highlighted row, checked row, disabled row (.45), danger row, closing (pop-out 140).
- **A11y:** bits-ui roles, roving focus, typeahead, Esc returns focus to the trigger. Menus inside dialogs work (65 > 50).
- **Ports:** `bundle.css:405-430,792-811` (Select list).

#### Dialog + AlertDialog — `ui/dialog/` (`add dialog`, K07) and `ui/alert-dialog/` (restyle, K07)

```ts
export const dialogOverlay =
  "fixed inset-0 z-(--z-overlay) bg-scrim data-[state=open]:animate-fade-in data-[state=closed]:animate-fade-out";
  // rgba(4,5,7,.72) · in 150ms · out 200ms · bundle.css:281,284 · opaque, never backdrop-filter (D-15)
export const dialogPanel =
  "fixed top-1/2 left-1/2 z-(--z-overlay) grid w-(--dialog-w) max-h-(--dialog-max-h) -translate-x-1/2 -translate-y-1/2 gap-4 overflow-auto rounded-lg border border-border-strong bg-surface p-6 text-fg shadow-pop outline-none data-[size=sm]:w-(--dialog-w-sm) data-[size=lg]:w-(--dialog-w-lg) data-[state=open]:animate-zoom-in data-[state=closed]:animate-zoom-out";
  // min(480px,100vw−32) · r16 · bg-100 · line-200 · pad 24 · in 300ms scale .94 + 8px · out 200ms scale .96 · bundle.css:285-287
export const dialogTitle = "text-subtitle font-semibold text-fg";            // 18/24 600 (D-06 has no 20 step; Halo 20/28)
export const dialogDescription = "text-small text-fg-muted";                  // 13/18
export const dialogFooter = "flex flex-wrap justify-end gap-2 pt-2";          // secondary first, primary last
export const paletteContent =
  "fixed top-(--palette-top) left-1/2 z-(--z-overlay) w-(--dialog-w-lg) -translate-x-1/2 overflow-hidden rounded-lg border border-border-strong bg-surface p-0 text-fg shadow-pop outline-none data-[state=open]:animate-zoom-in data-[state=closed]:animate-zoom-out";
  // top 12vh · min(560px,100vw−32) · bundle.css:283,292
export const paletteInput = "h-(--palette-input-h) w-full border-0 border-b border-border bg-transparent px-4 text-body text-fg outline-none placeholder:text-fg-subtle";
export const paletteRow = menuItem;                                           // 32/36/40 rows (D-20)
export const paletteFooter = "flex h-9 items-center gap-4 border-t border-border px-4 text-caption text-fg-subtle";
export const drawerContent =
  "fixed top-3 right-3 bottom-3 z-(--z-overlay) flex w-(--drawer-w) flex-col overflow-hidden rounded-lg border border-border-strong bg-surface text-fg shadow-pop outline-none data-[state=open]:animate-drawer-in data-[state=closed]:animate-drawer-out";
  // 760px right drawer (D-20 Output inspector) · in 300ms from +40px · out 200ms · bundle.css:360-364
```

- **The `-translate-x-1/2` and the keyframe `transform` compose.** Tailwind 4 writes `translate` (an individual property) and the keyframe writes `transform`.
- **`ui/dialog/dialog-content.svelte`:**
  - Props `size?: "default" | "sm" | "lg" | "palette" | "drawer"` and `showClose?: boolean = true`.
  - Palette uses `paletteContent` and drawer uses `drawerContent`.
  - The close X is an IconButton `label="Close"`, `absolute top-4 right-4`.
- **AlertDialog:** overlay and content swap to the same recipes. Action and cancel use `buttonVariants`.
- **Focus default changes (K24, behaviour, separate commit):** when `AlertDialog.Content` has `data-destructive` (new prop `destructive`), `onOpenAutoFocus` focuses `AlertDialog.Cancel` (D-20/K12). Today focus lands on the content container (`dev/ui/+page.svelte:422-426`). Non-destructive alerts keep today's behaviour.
- **Esc and backdrop:** bits-ui defaults. McpServersDialog keeps its own capture-phase Esc when it ports (4f, map 07 §6).
- **Overlay registry (K08, data only):** `state/overlays.svelte.ts`:

  ```ts
  class Overlays {
    modal = $state(0);
    floating = $state<Set<HTMLElement>>(new Set());
    openModal(): () => void { this.modal++; return () => { this.modal--; }; }
    trackFloating(el: HTMLElement): () => void { /* immutable Set add/remove */ }
  }
  export const overlays = new Overlays();
  ```

  - Dialog/AlertDialog/palette/drawer content calls `openModal()` in an `$effect` while `data-state=open`.
  - Popover/menu/select content calls `trackFloating(ref)`.
  - The only consumer is WebBrowserPage in 4f (it hides the native webview through the existing `browser_hide`/`browser_show`, `WebBrowserPage.svelte:375-377`). D-20 contract. No Rust.
- **A11y:** `role=dialog`/`alertdialog`, title and description wiring, focus trap and return, scroll lock (bits-ui `preventScroll`).
- **Ports:** `bundle.css:280-304,359-368,690-697`.
- **Retires:**
  - `.dialog-overlay` (`app.css:612-618`), its `overlay-in` keyframe (`:626`) and the double fade (map 07 §4);
  - the three hand-rolled scrims and panels (UpdateDialog, McpServersDialog, CommandPalette; 4f);
  - native `confirm()` (4f);
  - the Diagnostics modal branch (dead code, shortlist #6).

#### Popover — `ui/popover/` (`add popover`, K09)

- **Content:**
  - recipe `cn(floatingPanel, floatingMotion, "w-75 p-4 flex flex-col gap-3 origin-(--bits-popover-content-transform-origin)")` (300px, padding 16, gap 12; `bundle.css:432-435`), portalled, `sideOffset` 8;
  - title `text-small font-semibold text-fg`;
  - footer `flex justify-end gap-2 mt-1`.
- **Behaviour:** non-modal. Esc and outside click close it and return focus.
- **Upward variant:** comes from `data-side=top` (pop-in-up). Used by the bell (4a) and composer popovers (4c).

#### Tabs — `ui/tabs/` (`add tabs`, K21b)

| part | classes |
|---|---|
| list | `relative flex gap-5 border-b border-border overflow-x-auto overflow-y-hidden scrollbar-none` (`bundle.css:259-260`) |
| trigger | `inline-flex h-(--control-h) items-center gap-2 rounded-xs px-0 text-small font-medium text-fg-muted outline-none transition-colors duration-(--dur-base) ease-out hover:text-fg data-[state=active]:text-fg focus-visible:shadow-focus` |
| count | a Badge `mono`; it becomes the accent tone when active (`bundle.css:264-265`) |
| indicator | `<span data-part="ind">` inside the list: `absolute bottom-0 left-0 h-0.5 rounded-full bg-accent` with style `translate: var(--ind-x) 0; width: var(--ind-w)` and `transition: translate var(--dur-base) var(--ease-out), width var(--dur-base) var(--ease-out)` |

- The indicator is driven by the `slidingIndicator` action on the list (§2.5), selector `[data-state=active]`. 2px accent underline per `bundle.css:266`.
- **Ids pass through:** `settings-tab-*`/`settings-panel-*` (D-33) are passed as `id` props on Trigger and Content.
- **Keys:** bits-ui (arrows, automatic activation).
- **Retires:** Settings `.tab-ink` (`SettingsPage.svelte:1596`, 4e).

#### Accordion — `ui/accordion/` (`add accordion`, K30b)

- **Content:** uses `forceMount` plus the `child` snippet so Collapse's grid-rows motion runs (§1.3 #4). `inert={!open}` on the closed panel.
- **Trigger:** `flex w-full min-h-11 items-center gap-3 rounded-sm px-1 text-left text-small font-medium text-fg outline-none focus-visible:shadow-focus disabled:opacity-(--opacity-disabled)`. 44px matches the SettingsRow min height (D-31).
- **Chevron:** lucide `ChevronDown` 16, `text-fg-muted transition-transform duration-(--dur-base) ease-out group-data-[state=open]:rotate-180 group-data-[state=open]:text-fg` (D-12, one glyph app-wide).
- **Meta:** `font-mono text-caption text-fg-subtle`.
- **Variants:**
  - `rows`: items `border-b border-border`;
  - `cards`: items `rounded-md border border-border bg-surface data-[state=open]:border-border-strong`, trigger `px-4`, content `px-4 pb-4` (`bundle.css:484-489`).
- **Retires:** Settings `<details>` expanders (4e, D-31).

#### Switch (Toggle) — `ui/switch/` (`add switch`, K22b)

- **Root:** `relative inline-flex shrink-0 w-(--toggle-w) h-(--toggle-h) cursor-pointer rounded-full border border-field-border bg-surface-active outline-none transition duration-(--dur-base) ease-out data-[state=checked]:border-accent data-[state=checked]:bg-accent focus-visible:shadow-focus disabled:cursor-not-allowed disabled:opacity-(--opacity-disabled)`.
- **Thumb** (scoped CSS in `switch.svelte`; geometry needs `calc()`):

  ```css
  [data-slot="switch-thumb"] {
    position: absolute; top: var(--toggle-inset); left: var(--toggle-inset);
    width: var(--toggle-knob); height: var(--toggle-knob); border-radius: 999px;
    background: var(--ink-muted);
    transition: translate var(--dur-base) var(--ease-spring), width var(--dur-fast) var(--ease-out), background-color var(--dur-base) var(--ease-out);
  }
  [data-state="checked"] > [data-slot="switch-thumb"] { translate: var(--toggle-travel) 0; background: var(--accent-ink); }
  [data-slot="switch"]:active:not(:disabled) > [data-slot="switch-thumb"] { width: calc(var(--toggle-knob) + 6px); }
  [data-slot="switch"][data-state="checked"]:active:not(:disabled) > [data-slot="switch-thumb"] { translate: calc(var(--toggle-travel) - 6px) 0; }
  ```

- **Geometry:**

  | density | track | knob | travel |
  |---|---|---|---|
  | compact | 36×20 | 14 | 16 |
  | regular | 40×22 | 16 | 18 |
  | comfy | 44×26 | 18 | 18 (Halo exact, `bundle.css:141-148`) |
- **A11y:** `button role=switch aria-checked` (bits-ui). Without a visible label, pass `aria-label`.
- **Retires:** `.rift-toggle` (`app.css:476-493`; AssistantWelcome + SettingsPage), Settings `.sw`, SettingsMenu `.tt-switch` (4c/4d/4e).

#### Checkbox — `ui/checkbox/` (`add checkbox`, K22c)

- **Root:** `peer inline-flex size-(--check-size) shrink-0 items-center justify-center rounded-xs border border-field-border bg-surface-active outline-none transition duration-(--dur-fast) ease-out hover:not-disabled:border-fg-muted data-[state=checked]:border-accent data-[state=checked]:bg-accent data-[state=indeterminate]:border-accent data-[state=indeterminate]:bg-accent focus-visible:shadow-focus disabled:opacity-(--opacity-disabled)`.
  - Sizes by density: 16/18/20, r6.
- **Mark:** an inline SVG (`viewBox 0 0 16 16`, path `M3.5 8.5l3 3 6-7`, `pathLength="1"`).
  - Scoped CSS `stroke: var(--accent-ink); stroke-width: 2.4; stroke-dasharray: 1; stroke-dashoffset: 1; transition: stroke-dashoffset var(--dur-base) var(--ease-out)`.
  - Checked gives `stroke-dashoffset: 0` (the draw, `bundle.css:154,157`).
  - Indeterminate shows a 8×2 bar.
- **Pop:** `[data-touched][data-state=checked] { animation: halo-pop var(--dur-base) var(--ease-spring) }` (1.18 at 50%, `bundle.css:156,160`).
  - `data-touched` is set on the first user toggle. Pre-checked boxes do not pop when a page mounts (D-29 spirit).
- **Hit target:** a 28px row when it is wrapped by a label (D-32).

#### RadioGroup — `ui/radio-group/` (`add radio-group`, K22d)

- **Item:** `relative size-(--check-size) rounded-full border border-field-border bg-surface-active …` with the same hover, focus and disabled treatment as Checkbox. Checked gives `bg-accent border-accent`.
- **Dot:** `::after` `inset: 4px` (comfy 5px), `background: var(--accent-ink); scale: 0`. Checked transitions to `scale: 1` with `transition: scale var(--dur-base) var(--ease-spring)` (`bundle.css:566-571`).
- **Row:** label `text-small text-fg`, description `text-small text-fg-muted`, meta `font-mono text-data font-semibold`. `min-h-(--row-h)`.
- **`variant="cards"`:**
  - `grid grid-cols-[…]` is an arbitrary value, so the cards variant uses a scoped `grid-template-columns: repeat(auto-fit, minmax(180px, 1fr))` in `radio-group.svelte`;
  - card `p-4 rounded-md border border-border-strong bg-surface hover:bg-surface-hover`;
  - checked `border-accent` with `background: color-mix(in srgb, var(--accent) 6%, var(--bg-100))` (`bundle.css:575-582`).
- **Keys:** bits-ui arrows.

#### Slider — `ui/slider/` (`add slider`, K23b)

| part | classes |
|---|---|
| track | `relative h-1.5 w-full rounded-full bg-surface-active` (6px, r3) |
| range | `absolute h-full rounded-full bg-accent` |
| thumb | `block size-(--slider-thumb) rounded-full border-4 border-accent bg-fg outline-none transition-[…]` |

- The thumb can't use an arbitrary transition. A scoped rule handles it: `[data-slot=slider-thumb] { transition: scale var(--dur-fast) var(--ease-spring), box-shadow var(--dur-fast) var(--ease-out) } :hover { scale: 1.15 } :active { scale: 1.3 } :focus-visible { box-shadow: var(--shadow-focus) }` (`bundle.css:181-185`).
- **Value:** a label row with the value in `font-mono text-data font-semibold`, and `format` is required for units.
- **D-33 coupling (K23b):** `sliderBubble.ts:28` `const thumbW = 18` becomes `parseFloat(getComputedStyle(node).getPropertyValue("--slider-thumb")) || 18`. The bubble restyles to the tooltip look (ink bubble). Its `app.css:371-392` rules move to tokens in 4e.
- **Retires:** Settings range inputs (UI scale, vividness while #5 is pending), `input.slider-zoom` (`app.css:388`).

#### Combobox — `ui/combobox/` (K23c; it needs K23's `fieldInput`)

- **Source:** shadcn-svelte composes Combobox from Popover + Command and ships no `combobox` registry item. If `add combobox` reports "not found", build it on bits-ui's `Combobox` primitive (present at `node_modules/bits-ui/dist/bits/combobox`).
- **API:**

  ```ts
  options: Array<{ value: string; label: string; description?: string; meta?: string; disabled?: boolean }>
  value: string | null              // $bindable
  placeholder?: string
  maxResults?: number = 50
  emptyText?: string = "No matches"
  clearable?: boolean = true
  ariaLabel: string
  ```
- **Input:**
  - recipe `fieldInput` with `pl-9 pr-16`;
  - search icon 16 at left 10, accent on focus;
  - toggle chevron button 28 at right 4;
  - clear × 24 at right 32, entering with pop-in.
- **List:** `floatingPanel` + `floatingMotion`, `max-h-72` (288px, `bundle.css:800`). Rows use `menuItem` with a second line (`text-caption text-fg-muted`) and meta (`font-mono text-caption`).
- **Match:** a `<mark>` styled `bg-transparent font-semibold text-fg underline decoration-accent decoration-2 underline-offset-3` (`bundle.css:811`).
- **Footer:** "N more" `text-caption text-fg-muted border-t border-border` (`bundle.css:812-813`).
- **Filter and behaviour:**
  - case-insensitive substring over label + description;
  - leaving without picking restores the chosen label;
  - Esc on a closed field clears;
  - an `aria-live="polite"` sr-only line announces the result count.
- **First use:** the Diagnostics source filter (4f).

#### Command palette (no `add command`; §13 A-2)

- **Shell:** Dialog `size="palette"`.
- **Recipes:** `paletteInput`/`paletteRow`/`paletteFooter`/`menuLabel` for group headings.
- **Keyboard and scoring:** Rift's scorer and keyboard model (`CommandPalette.svelte:166-186,241-288`) stay as they are in 4f.
- **Footer legend:** `<Kbd keys={["↑","↓"]}/> navigate · <Kbd keys={["↵"]}/> open · <Kbd keys={["Esc"]}/> close`.

### 2.5 Motion helpers — `src/lib/motion/` (K04-K04d, K35-K35b; `tip.ts` ships in K31. Pure TS + runes, unit-tested; vitest env is `node`)

| File | API | Notes |
|---|---|---|
| `ease.ts` | `bezier(x1,y1,x2,y2): (t:number)=>number` · `easeOut = bezier(.2,.8,.2,1)` · `easeOutCubic = t => 1-(1-t)**3` · `easeSpring = bezier(.34,1.56,.64,1)` | Svelte easing functions for `animate:flip` and JS tweens. A Newton + bisection solver, 1e-6 precision |
| `reveal.ts` | `export const REVEAL = { WORD_MS: 30, CATCHUP_MS: 320, FADE_MS: 500 } as const` · `catchupLag(w=WORD_MS)=ceil(CATCHUP_MS/w)` (11) · `revealWindow(w, fade)=ceil(fade/w)+1` (18 today, 11 after the D-11 fade change) · `stepReveal(s:{shown:number; carry:number}, total:number, dtMs:number, w=WORD_MS) → {shown, carry}` | Pure port of `Markdown.svelte:216-246`. Target = `total + window`. `due = floor((carry+dt)/w)`; lag catch-up `ceil(due*lag/catchupLag)` (`:234-235`); **remainder carry** `carry = (carry+dt) - due*w` (D-11 #2, the fix for `:237` `last = t`). Constants stay owner-gated (D-11 pacing) |
| `follow.ts` | `followK(dtMs, k60 = 0.35) = 1 - (1-k60)**(dtMs/16.667)` · `stepFollow(delta:number, dtMs:number, opts?:{k60?:number; snapPx?:number=900; stopPx?:number=1}) → {move:number; done:boolean}` | Pure port of `AssistantPane.svelte:178-189` with frame-time normalisation (D-11 #3). Note: the code stops at 0.5px (`:182`); D-11 says 1px. The 1px value ships, and the follow test pins it |
| `countUp.svelte.ts` | `class CountUp { current = $state(0); constructor(o?:{duration?:number=700; ease?=easeOutCubic}); to(v:number, animate=true): void; set(v:number): void; destroy(): void }` | rAF tween. Snaps when `motion.reduced`. `to()` from the current displayed value. Used by StatTile, Progress value and ShareBar legend |
| `flip.ts` | `haloFlip(node, fromTo, params?) ⇒ svelte/animate flip(node, fromTo, { duration: motion.reduced ? 0 : 460, easing: easeOut, ...params })` | FlipList = Svelte `animate:flip` (already used at `ToastHost.svelte:52`), not a custom engine. 460ms (D-10). Consumers: QueueRail (4c), ranked lists |
| `indicator.ts` | `slidingIndicator(list: HTMLElement, opts: { selector: string; axis?: "x"\|"y" = "x"; target?: HTMLElement }) : ActionReturn` | Measures the active child relative to `list` and writes `--ind-x`/`--ind-w` (or `--ind-y`/`--ind-h`) on `target ?? list`. Re-measures on ResizeObserver(list), MutationObserver(attributeFilter: `data-state`,`aria-checked`,`aria-selected`,`aria-current`) and `document.fonts.ready`. Sets `data-ready` on target after the first write so CSS transitions start only afterwards. Consumers: Segmented, Tabs, sidebar active plate (4a, axis y, D-19) |
| `tip.ts` | `TIP`, `delayFor`, `place`, `repeatsVisibleText` | §2.4 |

`RollingNumber` digit math lives in `ui/rolling-number/roll.ts`:

```ts
columnsOf(text: string): Array<{ kind: "digit"|"sym"; ch: string; posFromRight: number }>
delayFor(posFromRight: number) = Math.min(posFromRight, 8) * 45
```

## 3. States

✓ means the state exists and is specified in §2. A dash means it does not apply. "open" means `data-state=open` for bits-ui parts.

| Part | rest | hover | focus-visible | active/press | disabled | open/checked | closed/leave | loading | empty | error |
|---|---|---|---|---|---|---|---|---|---|---|
| Button | ✓ | bg step / glow (primary) | ring | scale .97 | .45 | `aria-expanded` bg | – | Spinner + aria-busy | – | – |
| IconButton | ✓ | as ghost | ring + tooltip after 0ms | .97 | .45 | `aria-pressed` → soft | – | ✓ | – | – |
| Card | line-100 | line-200 (+3px lift if interactive) | focus-within ring (interactive) | – | – | – | – | Skeleton inside | EmptyState inside | – |
| Badge | ✓ per tone | – | – | – | – | – | – | – | – | danger tone |
| Kbd · Divider · SectionLabel | ✓ | – | – | – | – | – | – | – | – | – |
| LiveDot | ping (accent/info/warn) | – | – | – | – | – | idle = solid ink-faint | – | – | – |
| Spinner | spinning | – | – | – | – | – | – | always | – | – |
| EmptyState | rise-in once | – | action ring | – | – | – | – | – | ✓ | – |
| Banner | rise-in | dismiss ghost | ring | – | – | – | 200ms leave | – | – | danger = role alert |
| Skeleton | idle | – | – | – | – | – | bones out 360 | waiting(300ms) → bones | – | – |
| Segmented | thumb on value | label → ink | ring on radio | – | .45 per option | checked radio | – | – | – | – |
| SearchField | ✓ | bg-200 | border accent + ring | – | .45 | – | – | spinner icon | placeholder + kbd | – |
| Input / Textarea | ✓ | bg-200 | border accent + ring (`:focus`) | – | .45 | – | – | – | placeholder ink-faint | border danger + message |
| Field | label/hint | – | – | – | label row .45 | – | – | – | – | error replaces hint |
| Switch | off: bg-300 + knob ink-muted | – | ring | knob +6px stretch | .45 | accent track, ink knob, travel | – | – | – | – |
| Checkbox / Radio | bg-300 + line-300 | border ink-muted | ring | – | .45 | accent fill, mark draws (pop only if touched) | – | – | – | `aria-invalid` border danger |
| Slider | ✓ | thumb 1.15 | thumb ring | thumb 1.3 | .45 | – | – | – | – | – |
| Menu / Context / Select list | – | row highlight bg-200 + accent icon | row highlight (keyboard) | – | row .45 | pop-in 150 + rows 18ms stagger | pop-out 140 | – | – | – |
| Select trigger | ✓ | bg-200 | ring | – | .45 | border + chevron accent, chevron 180° | – | – | placeholder ink-faint | `aria-invalid` danger |
| Combobox | ✓ | bg-200 | ring | – | .45 | list + highlighted match | restores label | – | "No matches" | – |
| Dialog / Alert / Palette / Drawer | – | – | trap; destructive → Cancel | – | – | zoom/drawer in 300 | out 200 | – | palette empty → EmptyState compact | – |
| Popover | – | – | trigger ring | – | – | pop-in (up variant) | pop-out | – | – | – |
| Tabs | ink-muted | ink | ring | – | .45 | ink + underline slides | – | – | – | – |
| Accordion / Collapse | closed (inert) | – | trigger ring | – | .45 | rows 0fr→1fr, fade +60ms, chevron 180° | reverse | – | – | – |
| StatTile | value | peek opens | ring + peek | – | – | – | – | Skeleton stat | "—" with `format` | – |
| Progress | determinate | – | – | – | – | – | – | indeterminate sweep | value 0 | tone danger |
| ShareBar | segments | sibling dim .45 + tooltip | – | – | – | – | – | Skeleton text | EmptyState compact | – |
| Toast | collapsed stack | fanned + timers paused | fanned (focus-within) | drag | – | – | 240 leave / swipe | – | – | danger: alert, sticky |
| Tooltip | hidden | cold 400 / warm 120 | shows on focus-visible | hides on pointerdown | – | shown | hides at once | – | – | – |

## 4. Motion table

Durations are token names, with values: fast 150, base 300, slow 600. "Net" means the P2 global reduced-motion net.

| Element | Trigger | Property | From → to | Duration | Easing | Delay / stagger | Reduced motion |
|---|---|---|---|---|---|---|---|
| Button hover | pointer | background, border, colour, box-shadow | rest → hover; primary glow 0 → `0 8px 28px glow-strong` | 150 | ease-out | – | net (instant) |
| Button press | :active | `scale` | 1 → .97 | 150 | ease-out | – | net |
| Card (interactive) | hover | `translate`, border-colour | 0 → −3px; line-100 → line-200 | 150 / 300 | ease-out | – | net |
| Menu/popover/select open | `data-state=open` | opacity, transform | 0 · scale .96 · y ∓4px → none | 150 | ease-out | – | net |
| Menu/popover close | `data-state=closed` | opacity, transform | → 0 · scale .97 | 140 | ease-out | – | net |
| Menu row | mount | opacity, transform | 0 · y −3px → none | 220 | ease-out | `min(i,12) × 18ms` | net |
| Dialog open | open | opacity, transform | 0 · scale .94 · y 8px → none | 300 | ease-out | – | net |
| Dialog close | closed | opacity, transform | → 0 · scale .96 | 200 | ease-out | – | net |
| Scrim | open / closed | opacity | 0 → 1 / 1 → 0 | 150 / 200 | ease-out | – | net |
| Drawer | open / closed | opacity, transform | x 40px → 0 / → 40px | 300 / 200 | ease-out | – | net |
| Toast enter | push | opacity, transform | 0 · y −16 · scale .96 → none | 300 | ease-out | – | net |
| Toast fan | stack hover/focus | transform (`--y`, `--s`), height | collapsed → expanded | 300 | ease-out | – | net |
| Toast leave | dismiss / timeout | opacity, transform | → 0 · x +32px | 240 | ease-out | – | net |
| Toast swipe throw | release dx>90 or v>.6 | transform | → x 110% | 240 | ease-out | – | net |
| Toast spring-back | release short | transform | dx → 0 | 300 | ease-spring | – | net |
| Tooltip | show | opacity, transform | 0 · y ±4 → none | 150 | ease-out | cold 400 / warm 120 | net (instant) |
| Switch knob | toggle | `translate`, background | 0 → travel | 300 | ease-spring | – | net |
| Switch knob stretch | :active | width | knob → knob+6 | 150 | ease-out | – | net |
| Switch track | toggle | background, border | bg-300 → accent | 300 | ease-out | – | net |
| Checkbox mark | check | stroke-dashoffset | 1 → 0 | 300 | ease-out | – | net |
| Checkbox pop | check (touched) | transform | scale 1 → 1.18 → 1 | 300 | ease-spring | – | net |
| Radio dot | check | `scale` | 0 → 1 | 300 | ease-spring | – | net |
| Slider thumb | hover / active | `scale` | 1 → 1.15 / 1.3 | 150 | ease-spring | – | net |
| Segmented thumb | value | `translate`, width | old → new option | 300 | ease-out | – | net |
| Tabs underline | value | `translate`, width | old → new tab | 300 | ease-out | – | net |
| Select chevron | open | `rotate` | 0 → 180° | 300 | ease-out | – | net |
| Accordion / Collapse panel | open | grid-template-rows | 0fr → 1fr | 300 | ease-out | – | net |
| Accordion / Collapse content | open | opacity, `translate` | 0 · y −4 → none | 300 | ease-out | 60ms | net |
| Accordion chevron | open | `rotate` | 0 → 180° | 300 | ease-out | – | net |
| EmptyState | mount | opacity, transform | 0 · y 10px → none | 600 | ease-out | – | net; D-29 settled → 0s |
| Banner | mount / dismiss | opacity, transform | 0 · y 10 → none / → 0 · y −6 | 300 / 200 | ease-out | – | net |
| Skeleton bones out | loading → false | opacity, filter | 1 → 0 · blur 4px | 360 | ease-out | – | net |
| Skeleton content in | loading → false | opacity, filter, transform | 0 · blur 6px · y 6px → none | 600 | ease-out | – | net |
| StatTile value | first data / `rangeKey` | number (rAF) | old → new | 700 | easeOutCubic | – | snap (`motion.reduced`) |
| Progress bar | value | transform `scaleX` | old → new | 700 | ease-out | – | net |
| Progress value text | value | number (rAF) | old → new | 700 | easeOutCubic | – | snap |
| Progress indeterminate | while null | transform | x −100% → 250% | 1.3s ∞ | ease-out | – | net → static 40% bar |
| ShareBar segments | mount / `rangeKey` | `scale` x | 0 → 1 | 600 | ease-out | 60ms per segment | net |
| ShareBar sibling dim | hover | opacity | 1 → .45 | 300 | ease-out | – | net |
| RollingNumber digit | value | `translate` y | old digit → new | 900 | ease-out | `min(pos,8) × 45ms` right-first | net (instant) |
| FlipList (`haloFlip`) | reorder | transform | old pos → new | 460 | ease-out (bezier) | – | duration 0 |
| LiveDot ping | always while live | transform, opacity | scale 1 · .7 → scale 3 · 0 | 1.8s ∞ | ease-out | – | net → solid dot |
| Spinner | always | transform | rotate 0 → 360° | 900 ∞ | linear | – | **exempt**, keeps spinning (D-13) |
| SearchField clear | text appears | opacity, transform | pop-in | 150 | ease-out | – | net |

## 5. Parts

### 5.1 Kit primitives (`src/lib/components/ui/`)

| Part | Folder | Source | Commit |
|---|---|---|---|
| Button, IconButton | `button/` | existing `tv()` + new file | K10, K11 |
| Card | `card/` | new | K12 |
| Badge | `badge/` | new | K13 |
| Kbd | `kbd/` | new | K14 |
| LiveDot, Spinner | `live-dot/`, `spinner/` | new (wrap P2 keyframes) | K15 |
| Divider | `separator/` | existing + `label` | K16 |
| SectionLabel | `section-label/` | new | K17 |
| EmptyState | `empty-state/` | new | K18 |
| Banner | `banner/` | new | K19 |
| Skeleton | `skeleton/` | new | K20 |
| Segmented | `segmented/` | new | K21 |
| Tabs | `tabs/` | `shadcn add tabs` | K21b |
| SearchField | `search-field/` | new | K22 |
| Switch | `switch/` | `shadcn add switch` | K22b |
| Checkbox | `checkbox/` | `shadcn add checkbox` | K22c |
| RadioGroup | `radio-group/` | `shadcn add radio-group` | K22d |
| Field, Input, Textarea | `field/`, `input/`, `textarea/` | new + recipes | K23 |
| Slider | `slider/` | `shadcn add slider` | K23b |
| Combobox | `combobox/` | `shadcn add combobox`, or bits-ui Combobox directly | K23c |
| StatTile | `stat/` | new | K25 |
| Progress | `progress/` | `shadcn add progress` | K27 |
| PageHeader | `page-header/` | new | K28 |
| RollingNumber | `rolling-number/` | new | K29 |
| Collapse, Accordion | `collapse/`, `accordion/` | new + `shadcn add accordion` | K30, K30b |
| Tooltip layer | `tooltip/` | new (CX port) | K31, K32 |
| Toaster | `toast/` | new | K34b |
| Dialog (+palette/drawer sizes) | `dialog/` | `shadcn add dialog` | K07 |
| Popover | `popover/` | `shadcn add popover` | K09 |
| AlertDialog, DropdownMenu, ContextMenu, Select | existing | restyle via recipes | K05-K07, K24 |

### 5.2 Rift-specific parts built here

- **`shared/ShareBar.svelte`** (K26, API §2.2).
- **`state/overlays.svelte.ts`** (K08, API §2.4b).
- **`state/tooltip.svelte.ts`** (K31).
- **`state/toast.svelte.ts`** additions (K33, K34).
- **`src/lib/motion/*`** (K04-K04d, K35-K35b).

### 5.3 Rift composites NOT built here (they live with their surface, D-09)

| Composite | Surface |
|---|---|
| stream tiles, DiffBlock, TurnRail, receipt | 4b |
| SettingsRow, PreviewWell, SecretField | 4e |
| BrowserBar, LogStream, VitalChip | 4f |
| hub BarChart, Sparkline | 4d/4f (D-24; they consume ShareBar/Progress/StatTile) |

## 6. Ordered commits

50 commits in 7 sub-phases. Every commit leaves `npm run check`, `npm test` and `npm run check:ui` green, and the app runnable.

**Conventions.**
- "dev/ui" means `src/routes/dev/ui/+page.svelte`.
- Paths under `ui/` are `src/lib/components/ui/`.
- Commit subjects use `feat(kit): …`, `fix(kit): …`, `style(kit): …`.
- **Verify** names the cheapest proof. The full §10 pass runs once, at the end of each sub-phase.
- The dev/ui exhibit for each part lands in that part's own commit as a minimal section. K36 later polishes and orders the sections.

### 3a · Foundation (P2 must have landed)

| # | Files | Change | Preserves | Risk | Verify |
|---|---|---|---|---|---|
| K01 | `src/app.css` | Density vars per §1.5 A. Compact defaults go on `:root`; regular/comfy overrides extend the existing `:root[data-density=…]` blocks (`app.css:289-291`, keeping the `:root` prefix that comment :287-288 explains); the derived block is added | all existing `--*` tokens untouched, including `--row-h`/`--gap`/`--fs-md` | name clash with P1 tokens: run the §1.4 precheck first | `npm run check` |
| K02 | `src/app.css` (`@theme inline` :29-115), `src/lib/styles/motion.css` | §1.5 B bridge keys (type, shadow) and the `scrollbar-none` utility; §1.5 C `--animate-*` keys; the six Rift-derived keyframes (`halo-pop-out`, `halo-zoom-out`, `halo-fade-out`, `halo-drawer-out`, `halo-tip-in`, `halo-toast-in-top`) and the `data-menu-stagger` index rules, appended to P2's `motion.css` | existing bridge keys and P2's verbatim Halo keyframes untouched | keyframe name collides with tw-animate-css: the `halo-` prefix avoids it | `npm run check:ui` |
| K03 | `src/lib/utils.ts:11-19`, `src/lib/utils.test.ts` | Register K02's new theme keys (animate, text, shadow) in `createCn` so `cn()` dedupes them; add 3 merge tests | existing merge behaviour + tests | an unregistered key silently keeps both classes | `npm test -- utils` |
| K04 | `src/lib/motion/ease.ts` (+test) | `easeOutCubic`, `easeSpring` (sampled), the `DUR` constants mirroring tokens | – | drift from the CSS tokens: the test asserts the parsed `app.css` values | `npm test -- motion` |
| K04b | `src/lib/motion/indicator.ts` (+test) | `measureIndicator(list, active)` → `{x,w}` for the Segmented thumb and Tabs underline; ResizeObserver-driven | – | sub-pixel jitter: round to 0.5px | `npm test -- motion` |
| K04c | `src/lib/motion/countUp.svelte.ts` (+test) | rAF tween rune: 700ms, easeOutCubic, snaps under `motion.reduced`; first value counts up from 0 only when `fromZero` | `AnimatedCount` untouched (gated delete §8) | rAF in tests: inject the clock | `npm test -- motion` |
| K04d | `src/lib/motion/flip.ts` (+test) | `haloFlip` = `svelte/animate` flip preset, 460ms, bezier ease-out, duration 0 when reduced | – | – | `npm test -- motion` |

### 3b · Overlays

| # | Files | Change | Preserves | Risk | Verify |
|---|---|---|---|---|---|
| K05 | `ui/recipes.ts:20,58,60` | **Fix only.** `data-[open]` → `data-[state=open]` / `data-[state=closed]`. Nothing else changes, so the bisect is clean | all recipe class sets | menus that were instant now animate (tw-animate keys), which is intended | `npm run check:ui`; c.sh look a menu |
| K06 | `ui/recipes.ts`, `ui/dropdown-menu/*content*`, `ui/context-menu/*content*`, `ui/select/*content*` | Halo `floatingSurface` + `menuItem` recipes: `data-[state=open]:animate-pop-in data-[state=closed]:animate-pop-out` (§1.5 C), sideOffset 8, `min-w-55`, `data-menu-stagger` on the content; row highlight bg-200 with the icon in accent. `fieldTrigger` stays untouched until K23 | all bits-ui props pass through; `portalProps`; keyboard nav | exit animation must sit on the content node itself (bits-ui `getAnimations()`), not on a child | c.sh `shot-sel` open/close a menu |
| K07 | `ui/dialog/*` (`shadcn add dialog`), `ui/recipes.ts`, `ui/alert-dialog/*` | Dialog sizes sm/md/lg/palette/drawer; `dialogSurface`, `scrim` recipes; AlertDialog adopts them | AlertDialog API + every current call site | the `shadcn add` must not touch `recipes.ts` or `utils.ts`: review its diff | `npm run check`; c.sh look the dialog |
| K08 | `src/lib/state/overlays.svelte.ts` (+test) | Overlay registry: `open(id)`, `close(id)`, `top`, Esc routes to the top entry only | – | double Esc handling with bits-ui: the registry only *observes* bits-ui-owned overlays | `npm test -- overlays` |
| K09 | `ui/popover/*` (`shadcn add popover`) | Popover on the `floatingSurface` recipe (up variant) | – | – | dev/ui exhibit; `npm run check` |

### 3c · Atoms

| # | Files | Change | Preserves | Risk | Verify |
|---|---|---|---|---|---|
| K10 | `ui/button/button.svelte:1-85` | Halo variants per §2.2: primary/secondary/ghost/danger/soft; sizes sm/md/lg via density vars; `loading` prop (Spinner + `aria-busy`); press `.97`. Retired variants (xs/warn/info/link) are aliased to the nearest Halo variant for one release, not deleted | `href` → `<a>`, `ref` binding, `type` default | dev/ui is the only importer, so blast radius is small | c.sh `measure` button heights at 3 densities |
| K11 | `ui/button/icon-button.svelte`, `ui/button/index.ts` | IconButton: square, a required `label` → `aria-label` + tooltip, `pressed` → `aria-pressed` | – | a missing label is a type error, by design | `npm run check` |
| K12 | `ui/card/*` | Card: `interactive`, `padding`, `as` props; keeps `class` passthrough for the D-33 `.card` hook | the `#card-*` ids passed via `id` | – | c.sh look |
| K13 | `ui/badge/*` | Badge tones neutral/accent/ok/warn/danger/info; `dot` prop | – | – | dev/ui |
| K14 | `ui/kbd/*` | Kbd 20px (see A-12, Q4) | – | – | dev/ui |
| K15 | `ui/live-dot/*`, `ui/spinner/*` | LiveDot (ping) and Spinner (the D-13 exemption via `data-essential-motion` and its own scoped `!important` rule, §2.2) | – | the exemption must out-rank the P2 net's `*` rule: check specificity against the landed net | CDP reduced-motion: spinner still turns |
| K16 | `ui/separator/*` | `label` prop renders a centred SectionLabel-style divider | the existing Separator API | – | dev/ui |
| K17 | `ui/section-label/*` | SectionLabel: caps 11px ink-faint, optional `count` and action snippet | – | – | dev/ui |
| K18 | `ui/empty-state/*` | EmptyState: icon, title, body, action snippet; `compact`; rise-in once per mount | – | – | dev/ui |
| K19 | `ui/banner/*` | Banner tones; `dismissible`; danger → `role=alert`, others → `role=status` | – | – | dev/ui |
| K20 | `ui/skeleton/*` | Skeleton `loading` + shapes (text/stat/row/block); 300ms waiting threshold; bones-out/content-in crossfade | `shell/Skeleton` untouched (gated delete) | layout shift: bones size from the same density vars | c.sh `measure` before/after swap |

### 3d · Controls

| # | Files | Change | Preserves | Risk | Verify |
|---|---|---|---|---|---|
| K21 | `ui/segmented/*` | Segmented on bits-ui RadioGroup; thumb via `indicator.ts`; **neutral thumb** (A-6) | – | – | c.sh `shot-sel` value change |
| K21b | `ui/tabs/*` (`shadcn add tabs`) | Underline variant via `indicator.ts`; `value` bindable; Trigger/Content accept explicit `id` so Settings can keep its D-33 ids | `settings-tab-${secDomId}` / `settings-panel-${secDomId}` (`SettingsPage.svelte:640-641`) when 4e adopts Tabs | bits-ui generates ids unless `id` is passed through; the shadcn default pill style must be replaced, not layered | c.sh look |
| K22 | `ui/search-field/*` | SearchField: icon, clear button (pop-in), Kbd hint, `loading` | – | – | dev/ui |
| K22b | `ui/switch/*` (`shadcn add switch`) | Halo track/knob, stretch on press, spring travel | `checked` bindable, `onCheckedChange` | knob size and travel must read `--toggle-knob`, `--toggle-travel` (§1.5 A) | c.sh `measure` knob travel at 3 densities |
| K22c | `ui/checkbox/*` | Mark draw via stroke-dashoffset; pop only when the user touched it (`data-touched`) | indeterminate | the initial-render pop is suppressed by `data-touched` | dev/ui |
| K22d | `ui/radio-group/*` | Dot scale with spring | – | – | dev/ui |
| K23 | `ui/field/*`, `ui/input/*`, `ui/textarea/*`, `ui/recipes.ts` | Field (label/hint/error, `aria-describedby` wiring), Input, Textarea (autosize opt-in); adds the `fieldInput` recipe and **rewrites** `fieldTrigger` to share it | Select trigger behaviour; every `fieldTrigger` consumer | Select triggers app-wide restyle in this commit: c.sh look Settings | c.sh look Settings + dev/ui |
| K23b | `ui/slider/*`, `src/lib/actions/sliderBubble.ts:28` | Slider (bits-ui) with Halo thumb; sliderBubble's local `const thumbW = 18` (`:28`) reads `--slider-thumb` off the input, falling back to 18 | the `sliderBubble(node: HTMLInputElement, { format })` signature (D-33); the bubble stays attached to native range inputs (Settings), since a bits-ui Slider has no `<input>` | the bubble mis-centres if the var is set without the thumb CSS following: both read the same var | c.sh `shot-sel` drag a Settings slider bubble | c.sh `shot-sel` drag bubble |
| K23c | `ui/combobox/*` | Combobox on bits-ui with the `floatingSurface` list and "No matches" | – | – | dev/ui keyboard pass |

### 3e · Data and composition

| # | Files | Change | Preserves | Risk | Verify |
|---|---|---|---|---|---|
| K24 | `ui/alert-dialog/*content*` | `destructive` prop → focus Cancel on open (A-9) | every existing alert call site | user muscle memory (Enter confirmed before): noted in the release notes | dev/ui keyboard |
| K25 | `ui/stat/*` | StatTile: label, value (countUp), delta, `format`, peek snippet, `rangeKey` | – | – | dev/ui |
| K26 | `src/lib/components/shared/ShareBar.svelte` | ShareBar (A-7): segments, legend, "Other" on line-200, sibling dim, tooltip per segment | – | – | dev/ui |
| K27 | `ui/progress/*` (`shadcn add progress`) | Determinate (scaleX) + indeterminate sweep; tones; optional value text | – | – | CDP reduced-motion: static 40% bar |
| K28 | `ui/page-header/*` | PageHeader: eyebrow, title, meta, actions snippet; replaces the PageHero role later | `PageHero` untouched (gated) | – | dev/ui |
| K29 | `ui/rolling-number/*` | RollingNumber: per-digit columns, right-first stagger, `tabular-nums` | – | width jump on digit-count change: animate width over 300 | dev/ui |
| K30 | `ui/collapse/*` | Collapse on the grid-rows technique; `inert` when closed | – | – | dev/ui |
| K30b | `ui/accordion/*` (`shadcn add accordion`) | Accordion items reuse the Collapse panel CSS | – | – | dev/ui keyboard |

### 3f · Tooltip and toasts

| # | Files | Change | Preserves | Risk | Verify |
|---|---|---|---|---|---|
| K31 | `src/lib/motion/tip.ts` (+test), `src/lib/state/tooltip.svelte.ts`, `src/lib/actions/tooltip.ts:1-41` | Pure `place()` (CX port), singleton state with cold 400 / warm 120 timers, the action rewritten to drive the singleton instead of `title=` | **the import path and both call forms** (string + object), the 174 sites in 35 files compile unchanged | today the action is a11y-only (strips `title`, sets `aria-label`, `tooltip.ts:24-41`), so the app shows no visible tooltips. That stays true wherever no layer is mounted: the action keeps the `aria-label` path and only schedules a show when `tooltip.layerMounted` is true. Never fall back to `title=` | `npm test -- tip`; `npm run check` |
| K32 | `ui/tooltip/tooltip-layer.svelte` | One portal layer that renders the singleton | – | z-order versus dialogs: the layer sits above the overlay layer | c.sh `shot-sel` hover on dev/ui |
| K33 | `src/lib/state/toast.svelte.ts:58,65-71,146-225` | Additions only: `promise()`, `update(id)`, `dismissAll()`, `max` visible 3, pause-on-hover state | every existing export and signature (D-33) | – | `npm test -- toast` |
| K34 | `src/lib/state/toast.svelte.ts` | danger → sticky (A-8); fix the stale "bottom-right" comment at `:2` | the info/ok/warn timeouts | behaviour change | `npm test -- toast` |
| K34b | `ui/toast/toaster.svelte` | Stacked toaster (offset 10, scale .05, fan on hover/focus, swipe, leave 240); `contained` prop for dev/ui. Body click no longer fires the action (Q6) | ToastHost stays mounted in AppShell until P4a swaps it (gated) | two hosts rendering one store: dev/ui uses `contained`, AppShell keeps ToastHost | c.sh `shot-sel` hover the stack |

### 3g · Motion helpers and the /dev/ui gallery

| # | Files | Change | Preserves | Risk | Verify |
|---|---|---|---|---|---|
| K35 | `src/lib/motion/reveal.ts` (+test) | Authored and tested only. Wired in P4b (A-4) | – | – | `npm test -- motion` |
| K35b | `src/lib/motion/follow.ts` (+test) | Authored and tested only. Stop threshold 1px per D-11 (the code had 0.5px) | – | – | `npm test -- motion` |
| K36 | `src/routes/dev/ui/+page.svelte:1-445` | Orders the gallery sections to follow §5.1; each section shows every §3 state | – | the page grows: split into `sections/*.svelte` if over ~600 lines | c.sh look the full page |
| K37 | `src/routes/dev/ui/+page.svelte` | Header: density switch (sets `data-density`), accent bar (sets `data-accent=custom`), `title=` → tooltip action; mounts `tooltip-layer` and `<Toaster contained>` | – | – | c.sh `measure` at 3 densities |
| K38 | `design-system/ui-baseline.json` | Ratchet: `npm run check:ui -- --update` **only if** a count dropped (the alert-dialog/Select restyles can remove arbitrary values). If nothing dropped, K38 is a check-only step with nothing to commit. The final lock-in stays P5 (D-27) | the baseline never goes up | running `--update` with a count that rose would bake in a regression: diff the JSON before committing | `npm run check:ui` |

## 7. Contracts (D-33)

These are the names that live code, tests, c.sh selectors or CSS outside `ui/` rely on. P3 keeps every one of them. If a later phase renames one, it renames it in the same commit as all its consumers.

| Contract | Where it lives today | Kit obligation | Commit |
|---|---|---|---|
| `.card` class + `#card-*` ids (16 ids: accent, interface, rendering, comfort, session, api, codex, chatgpt, engine, composer, admin, build, shortcuts, tools, help, plus `#card-claude` on `.sb-status`) | `SettingsPage.svelte:717-1554` | Card passes `id` and `class` through unchanged and never sets its own `id` | K12 |
| `.sflash` search flash | `SettingsPage.svelte:200-204,616` adds/removes it on the card | Card must not set `animation` on its root, which would mask `.sflash` | K12 |
| Settings tab ids `settings-tab-${secDomId}` / `settings-panel-${secDomId}` | `SettingsPage.svelte:640-641` | Tabs Trigger/Content accept explicit `id` | K21b |
| `onRadioKey` keyboard model | `SettingsPage.svelte:242-257` (local fn) | Segmented reproduces arrows/Home/End plus roving tabindex. Settings keeps its local copy until 4e | K21 |
| sliderBubble geometry `thumbW` | `sliderBubble.ts:28` `const thumbW = 18` | reads `--slider-thumb` with fallback 18; the signature is unchanged | K23b |
| `toast` store API: `push`, `dismiss`, `pause`, `resume`, severities `ok\|warn\|danger\|info\|muted`, `MAX_VISIBLE` | `toast.svelte.ts:8,58,146-225` | additions only. The danger timeout changes (A-8) | K33, K34 |
| `import { tooltip } from "$lib/actions/tooltip"`, `TooltipOpts`, string + object call forms | `tooltip.ts:10-17`, 174 sites / 35 files | path, export name and type unchanged; `kbd` and `placement` honoured | K31 |
| per-part `--bits-<part>-content-transform-origin` (aliases `--bits-floating-transform-origin`, §1.3 #3) | `dropdown-menu-content.svelte:29` | every floating content keeps its `origin-(…)` class so scale animations grow from the trigger | K06 |
| `data-slot` values on ui parts | shadcn convention | treated as public selectors (c.sh, tests); renaming one is a contract change | all |
| `prefersReducedMotion()` | `state/accessibility.svelte.ts:103-106` | left alone. The kit reads P2's reactive `motion.reduced` | K04c, K29 |

## 8. Deletions

**P3 deletes nothing.** D-27 puts approved deletions in P5, and shortlist #6 gates the ones listed in §8.2. This section is the retirement map, so later phases know what each P3 part replaces.

### 8.1 Legacy dialect → kit part (deleted when its grep count reaches 0; the deletion lands in P5)

| Legacy | Defined at | Replaced by | Last consumers move in |
|---|---|---|---|
| `.btn` family | `app.css:394-421` | Button / IconButton | P4a-f (per surface) |
| `.pill` | `app.css:449-470` | Badge | P4b, P4e |
| `.input`, `.field-label`, `.help`, `.dim` | `app.css:423-444` | Field / Input / Textarea | P4e, P4d |
| `.st-*` controls | `styles/settings-controls.css` (used only by SettingsPage) | Segmented, Switch, Slider, Field | P4e |
| `.ob-btn`, `.ob-seg*` | `styles/onboarding.css:133-148,209-235` | Button, Segmented | P4d |
| `.sxa-btn` | scoped (grep before deleting) | Button | its surface's phase |
| `.rift-menu*` | `app.css:695-736` | DropdownMenu / ContextMenu recipes | P4f |
| `.rift-toggle` | `app.css:476-493` | Switch | P4e |
| `.kbd` (12 users) | `app.css:357-367` | Kbd | P5 sweep |
| `.card` *styles* (the class name stays as the D-33 hook) | `app.css:496-500` | Card | P4e |
| `.dialog-overlay` | `app.css:612-618` | Dialog `scrim` recipe | P4f |
| `.spin`, `.pulse` | `app.css:623-629` | Spinner, LiveDot (P2 keyframes) | P2/P5 |
| `.slider-bubble` rules | `app.css:371-392` | tokens + tooltip look | P4e |

### 8.2 Gated by shortlist #6 (owner sign-off before deleting)

| File / dependency | Replaced by | Precondition |
|---|---|---|
| `assistant/stream/AnimatedCount.svelte` | `countUp` + StatTile / RollingNumber | P4b swaps the consumers |
| `components/ToastHost.svelte` | `ui/toast/Toaster` | P4a mounts the Toaster in AppShell |
| `shared/PageHero.svelte` | PageHeader | P4d-f |
| `shared/LimitBar.svelte` | Progress | P4b/P4e |
| `shell/Skeleton.svelte` | `ui/skeleton` | AssistantWelcome swapped (P4b) |
| `styles/severity-tone.css` | Badge/Banner tones | grep 0 |
| `tw-animate-css` dependency (A-10) | `halo-*` keyframes | no `animate-in`/`fade-in`/`zoom-in` classes remain after K06/K07 |
| `[data-slot]` RM kill `app.css:120-122` (A-11) | P2 reduced-motion net | the P2 net is verified to cover `ui/` |

## 9. Canvas boards

No board gates P3 (D-25). Boards 00, 04 and 08 show kit parts, and the numbers below are what they export back into this spec. A changed board value is a one-line edit to the token or constant named here, never a structural change.

| Number | Value in this spec | Lands in |
|---|---|---|
| Tooltip delay cold / warm | 400 / 120 ms | `state/tooltip.svelte.ts` (K31) |
| Dialog enter scale / offset | .94 / 8px | `halo-zoom` keyframe (P2) via `--animate-zoom-in` (K02) |
| Menu enter | 150 ms, scale .96 | `halo-pop-in` (P2) via `--animate-pop-in` (K02) |
| Press scale | .97 | Button (K10) |
| Count-up | 700 ms easeOutCubic | `countUp` (K04c) |
| Digit roll | 900 ms, 45 ms stagger | RollingNumber (K29) |
| Toast stack offset / scale step | 10px / .05 | Toaster (K34b) |
| Toast leave | 240 ms (Halo). Rift's `--dur-base` is 300 | Toaster (K34b); the board decides |
| Density control heights `--control-h` / `--control-dense-h` | 32/36/44 · 28/32/36 (compact/regular/comfy) | §1.5 A vars (K01) |
| Kbd size | 20 (Halo 22) | Kbd (K14), Q4 |

Board 00 is the density board (D-06 shortlist #2). If it moves a control height, K01 changes and the `measure` acceptance numbers in §10 follow.

## 10. Verification and acceptance

The gates per commit are in §6. At the end of each sub-phase, and before the P3 merge, run the following on the Windows dev machine.

1. `npm run check` (includes `svelte-kit sync`), `npm test`, `npm run check:ui`. All green; `ui-baseline.json` counts not higher than at start.
2. Open `/dev/ui` (`bash scripts/cdp/c.sh eval "location.assign('/dev/ui')"`; `nav` has no dev/ui target), then `c.sh look` and confirm 0 errors (`c.sh errors`).
3. **Density.** For each of compact / regular / comfy (K37 switch sets `:root[data-density]`): `c.sh measure '[data-slot=button]' nokids` and the same for Input, Select trigger, Switch, Segmented. Heights must equal the §1.5 A values ±0.5px (md button 32/36/44; sm 28/32/36; toggle 36×20 / 40×22 / 44×26).
4. **States.** For each interactive part: `c.sh shot-sel '<part selector>' jpeg 70 hover`, then `focus`, then `active`. Compare with the §3 row.
5. **Open/close.** Open each menu, select, popover and dialog with `c.sh click`, then `c.sh eval "document.querySelector('[data-state=open]')?.getAnimations().length"`. Must be ≥1 on open and on close (proves K05).
6. **Reduced motion.** c.sh has no media-emulation verb. Use P2's reduced-motion override (or CDP `Emulation.setEmulatedMedia` through the rift-ui skill), then repeat step 5. Expect 0 animations, except the Spinner (D-13), which still turns.
7. **Tooltip.** Hover an IconButton: no tooltip before 400ms; one tooltip after; a second hover within the warm window shows at 120ms. On the app home (no layer): `c.sh eval "document.querySelector('[aria-label]')?.title"` is empty (unchanged a11y-only behaviour).
8. **Toasts.** On dev/ui, push 4 toasts: 3 are visible, hover fans them, and danger stays until dismissed.
9. **App-wide diff (A-14).** `c.sh baseline` before K05 and `c.sh diff` after K23 on Home, Settings and Chat with a menu, a Select and an alert dialog open. The only differences are the restyled parts.
10. **Rust:** not touched by P3, so `npm run verify` is needed only at the P3 merge (D-27).

**Acceptance:**
- every §5.1 part renders in `/dev/ui` in every §3 state at three densities;
- steps 1-9 pass;
- the 174 tooltip sites compile unchanged;
- no file listed in §1.2 has a diff.

## 11. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| App-wide restyle of menus/Select/alert leaks into P3 (A-14) | certain | medium | K05 lands alone; §10 step 9 diff; revert = K05-K07 |
| Exit animation placed on a child is cut off by bits-ui unmount | medium | low | §1.3 #2; step 5 checks `getAnimations()` on the content node |
| `check:ui` rule 1 rejects dashed scoped classes in atoms | high if ignored | blocks commit | style via `data-*` selectors only (§1.3 #5) |
| `cn()` drops or keeps conflicting new theme classes | medium | subtle styling bugs | K03 registry + tests |
| Tooltip singleton regresses 174 sites | low | high | a11y path unchanged; show gated on `layerMounted`; object form covered by tests |
| Two toast hosts render one store | medium | duplicate toasts | Toaster only `contained` in dev/ui until P4a removes ToastHost in the same commit |
| `shadcn add` overwrites `recipes.ts`/`utils.ts` | medium | lost fixes | review every `add` diff; never `init` (D-09) |
| Density vars clash with P1 names | low | wrong sizes | §1.4 precheck |
| Danger toasts now sticky / alert focuses Cancel | certain | user-visible behaviour change | A-8, A-9 call it out in the changelog |
| /dev/ui page grows past maintainable size | medium | low | K36 splits into section files |

## 12. Effort

**L: 6-7 sessions, 50 commits.**

| Sub-phase | Commits | Sessions |
|---|---|---|
| 3a foundation | 7 | 0.5 |
| 3b overlays | 5 | 1 |
| 3c atoms | 11 | 1 |
| 3d controls | 9 | 1.5 |
| 3e data + composition | 8 | 1 |
| 3f tooltip + toasts | 5 | 1 |
| 3g helpers + gallery | 5 | 0.5-1 |

## 13. Amendments proposed

| # | Amends | Proposal | Reason |
|---|---|---|---|
| A-1 | D-06 | Names the density vars (`--control-h`, `--control-dense-h`, `--control-r`, `--control-px`, `--toggle-w/-h/-inset`, `--check-size`, `--card-pad`, `--badge-h`, `--palette-input-h`, `--tile-head-h`, plus the derived `--toggle-knob`, `--toggle-travel`, `--slider-thumb`, §1.5 A) | D-06 fixes the values but not the names. P3 needs names that P4 consumers can bind to |
| A-2 | D-09 (shadcn add list) | Skip `shadcn add command`; build the palette on Dialog + recipes | Rift's palette scorer and keyboard model (`CommandPalette.svelte:166-186,241-288`) are kept, and `command` would duplicate them |
| A-3 | D-17, D-28 #6 | Tooltip = ConsoleX singleton with a pure `place()`, no floating-ui. D-28 spike #6 becomes moot | 174 action sites need one singleton. bits-ui Tooltip is per-trigger. `TooltipOpts.placement` only allows top/bottom, which `place()` covers |
| A-4 | D-11, D-27 | `reveal.ts` and `follow.ts` are written and tested in P3, wired in P4b. `follow` stop threshold 1px per D-11 (today's AssistantPane logic uses 0.5px) | keeps P3 free of chat-surface diffs; the helpers are testable alone |
| A-5 | D-17, D-20 | Tooltip layer and Toaster mount only on `/dev/ui` in P3; app-wide in P4a (one-line commits) | `/dev/ui` sits outside AppShell (§1.3 #7); D-27 keeps the app unchanged |
| A-6 | Halo board (accent Segmented thumb) | Neutral `bg-300` thumb; accent only as the opt-in `tone="accent"` | §2 accent scarcity: Settings alone has 7 segmented controls |
| A-7 | D-09 (all in `ui/`) | ShareBar lives in `components/shared/`; its "Other" segment uses `line-200` | two surfaces use it (hub + health). It is a composite, not a primitive |
| A-8 | toast timeouts (`toast.svelte.ts:65-71`) | danger becomes sticky (was 8000ms) | errors should not vanish unread. A behaviour change |
| A-9 | AlertDialog default focus | `destructive` dialogs focus Cancel | Enter must not confirm a destructive action. A behaviour change |
| A-10 | dependency list | `tw-animate-css` becomes dead after K06/K07 → P5 deletion (#6) | all motion moves to `halo-*` keyframes |
| A-11 | `app.css:120-122` | The `[data-slot]` reduced-motion kill becomes redundant after the P2 net → P5 deletion | one reduced-motion mechanism |
| A-12 | Halo Kbd 22px | Kbd 20px | fits the 28px compact row and the 12 existing `.kbd` sites |
| A-13 | ToastHost behaviour | Clicking the toast body no longer fires the action; only the action button does | swipe and fan-on-hover make body clicks ambiguous (Q6) |
| A-14 | D-27 ("P3 visible only in /dev/ui") | The 4 existing shared parts (dropdown, context menu, Select, alert dialog) restyle and gain motion app-wide in P3 | they are the same components the app renders. Forking parallel Halo copies until P4 would double the code and the review. Gated by the §10 step 9 diff |

## 14. Open questions (each has a default; the build proceeds on the default)

| # | Question | Default |
|---|---|---|
| Q1 | Should the K10 Button variant aliases (xs/warn/info/link) survive past P5? | No. Delete in P5 (only dev/ui uses them) |
| Q2 | Does the danger toast keep a countdown ring once sticky? | No ring, × only |
| Q3 | Badge height: 20 (Halo) or follow `--control-h` × .625? | fixed 20 at all densities |
| Q4 | Kbd 20 or 22? | 20 (A-12) |
| Q5 | Should Segmented ever use the accent thumb? | only via `tone="accent"`, used by onboarding step 1 |
| Q6 | Should the toast body click keep firing the action? | No (A-13) |
| Q7 | Accept A-14, or fork Halo copies of the 4 shared parts until P4? | Accept A-14 |
| Q8 | Toast leave 240 (Halo) or 300 (Rift `--dur-base`)? | 240 until board 08 says otherwise |
