# 10 - Halo digest for the Svelte port (Electric blue, dark)

Reference sheet for implementers. Read-only wave; nothing in source was touched.

## 0. Sources and citation keys

- `HALO` = `C:/Users/BLAZZER/AppData/Local/Temp/claude/C--AI-Workflow-projects-rift-tauri/3cd5d05a-bb71-48d8-8d23-bc53e60827d8/scratchpad/artifact-files/705c5425-86e3-4761-a421-4ee89499e5c6/project`
- `css:N` = `HALO/components/bundle.css:N` (903 lines; byte-identical to `C:/AI Workflow/projects/consolex/panel/design-systems/halo/bundle.css`, verified with `cmp`).
- `js:N` = `C:/AI Workflow/projects/consolex/panel/design-systems/halo/bundle.js:N` (v2.3.0, 2134 lines, READABLE, not minified). **The brand-book folder has no bundle.js; ConsoleX's vendored copy does and it holds every behaviour number (count-up easing, toast drag thresholds, FLIP, spotlight, magnetic).** Treat `js:` as ground truth for behaviour; the READMEs round some of it.
- `dts:N` = `HALO/components/index.d.ts:N`. `RM:<Name>` = `HALO/components/<Name>/README.md`. `tok:N` = `HALO/tokens.json:N`. `tcss:N` = `C:/AI Workflow/projects/consolex/panel/design-systems/halo/tokens.css:N`. `bk:NN-x` = brand-book section file (01 colour, 02 type/layout, 03 motion, 04 perf, 05 content, 06 a11y, 07 categories, 08 upgrading; README = `HALO/README.md`).
- Rift side: `rift:` = `C:/AI Workflow/projects/rift-tauri/`. Installed: svelte 5.57.1, bits-ui 2.19.3, tailwind 4.3, vite 8 (`rift:package.json:40-76`, node_modules package.json).

## 1. Tokens

Theme = `data-theme="blue"` on the root. `[data-theme="blue"]` only overrides a handful (tcss:63-74); everything else is the default `dark` block (tcss:2-62). Rift is dark-only (`rift:src/app.css:125-126`), so only dark + blue matter; the 4 light themes can be dropped.

### 1.1 Colour (dark base, blue overrides marked)

| Token | Dark / Blue value | Use |
|---|---|---|
| bg-000 | #08090c | page ground |
| bg-100 | #0e1014 | cards, panels, inputs, popovers |
| bg-200 | #14171c | hover, menus, toasts, active nav, secondary buttons |
| bg-300 | #1b1f26 | tracks, skeletons, avatar tiles, neutral badges, toggle track |
| line-100 | #1f242c | hairline on cards/dividers (decorative) |
| line-200 | #2c323c | card hover, popup outlines |
| line-300 | #5f6774 | border of EVERY control (inputs, selects, checkbox, radio, toggle) |
| ink | #eceff3 | primary text |
| ink-muted | #9aa2af | secondary text |
| ink-faint | #7d8592 | section labels, ticks, timestamps only; never on bg-300 |
| accent | **blue #6aa8ff** (dark: #c6f35a) | the one signal colour |
| accent-ink | **#08090c** (dark #0b0d10) | text/icons on accent fill |
| accent-soft | **rgba(106,168,255,.14)** | tinted ground behind accent text |
| glow-strong | **rgba(106,168,255,.28)** | only under hovered primary button |
| info | **#c6f35a (LIME, swapped in blue)** (dark #6aa8ff) | informational, chart-2 |
| info-soft | **rgba(198,243,90,.14)** | |
| warn / warn-soft | #ff9f5a / rgba(255,159,90,.14) | negative delta, warnings, chart-4 |
| danger / danger-soft | #ff6b6b / rgba(255,107,107,.14) | destructive, errors |
| chart-1..4 | accent, info(lime), #b79cff, warn | series order (blue inherits chart-3 violet from dark; tcss:25) |
| scrim | rgba(4,5,7,.72) | behind dialog/drawer/palette |
| game-glass | rgba(14,16,20,.9) | Game category only; skip |
| shadow-pop | 0 12px 32px rgba(0,0,0,.45) | popovers, menus, dialogs, drawers, toasts only |
| shadow-focus | **0 0 0 2px #0e1014, 0 0 0 4px #6aa8ff** | focus ring on every control |
| opacity-disabled | .45 | all disabled controls |

Gotchas in the colour system:
- **Blue preset makes `info` LIME GREEN** (tcss:68-69). ConsoleX overrode it to #9cc6ff / rgba(156,198,255,.14) because the owner wants no green (`consolex/panel/src/app.css:62-67`, 2026-10-17). In blue, chart-2 is therefore lime too unless overridden. Rift has its own green `--ok` (oklch .73 .18 150, `rift:app.css:189`) and `--status-busy #22c55e` (`:184`). Owner decision needed.
- `shadow-focus` hard-codes the gap colour #0e1014 (= bg-100), not the surface it sits on (tcss:73). On bg-000/bg-200 the 2px gap shows bg-100. Cosmetic, but note when porting.
- Halo has NO `ok`/success token: positive = accent, negative = warn (not danger), destructive = danger (bk:01:32).
- tokens.css has no rgb triplets (ConsoleX added `--accent-rgb` by hand, app.css:6-9) and **no type-scale vars at all**; the type styles exist only in tokens.json and as inline `font:` shorthands in bundle.css. Implementers must define type utilities.

Contrast measured here (node script, WCAG ratios, dark/blue values):

| fg \ bg | bg-000 | bg-100 | bg-200 | bg-300 |
|---|---|---|---|---|
| ink | 17.26 | 16.51 | 15.57 | 14.33 |
| ink-muted | 7.74 | 7.40 | 6.98 | 6.42 |
| ink-faint | 5.35 | 5.12 | 4.83 | **4.44 (fails 4.5; Halo says never use here)** |
| accent #6aa8ff | 8.21 | 7.85 | 7.41 | 6.81 |
| warn | 9.82 | 9.39 | 8.86 | 8.15 |
| danger | 7.17 | 6.86 | 6.47 | 5.96 |
| line-300 (control border) | 3.49 | 3.34 | 3.15 | **2.90 (<3:1)** |
| accent on accent-soft tint | 6.82 | 6.35 | 5.88 | 5.34 |
| danger on danger-soft | 6.11 | 5.72 | 5.33 | 4.85 |

accent-ink #08090c on accent = 8.21. Toggle/checkbox/radio sit on a bg-300 fill with a line-300 border (css:141,152,566) so the border-vs-fill contrast is 2.90; against the surrounding surface it is 3.34. Fine, but "3:1 everywhere" in bk:01:31 is only true vs the outer surface.

### 1.2 Type (tok:335-431)

Families: sans `"Geist", ui-sans-serif, system-ui, sans-serif`; mono `"Geist Mono", ui-monospace, "SFMono-Regular", monospace` (tok:332-333). Only weights 400/500/600 are used anywhere in the CSS (700 appears once, in the font @import, css:1).

| Style | Family | Size/line | Wt | Tracking | Use |
|---|---|---|---|---|---|
| display | sans | 48/52 | 600 | -0.03em | one per page |
| title | sans | 32/38 | 600 | -0.03em | screen title |
| subtitle | sans | 20/28 | 600 | -0.02em | dialog/drawer(wide)/settings group titles |
| heading | sans | 16/22 | 600 | 0 | card/drawer/panel titles |
| body | sans | 15/22 | 400 | 0 | reading text |
| small | sans | 13/18 | 400 | 0 | helpers, labels |
| caption | sans | 12/16 | 400 | 0 | hints, chips, badges, timestamps |
| metric | mono | 30/34 | 600 | -0.03em | Stat value (tabular) |
| data | mono | 13/18 | 500 | 0 | amounts, ticks |
| label | mono | 11/14 | 500 | 0.12em, UPPERCASE | SectionLabel |

Mono always has `font-variant-numeric: tabular-nums` (css:19,64,176,...). No `font-feature-settings` anywhere in Halo; Rift's Inter ones (`rift:app.css:306`) must go.

### 1.3 Spacing, radius, layers, breakpoints

- space-1..8 = 4, 8, 12, 16, 24, 32, 48, 64 px. Card pad space-5 (24); grid gap space-4 (16); section gap space-6 (32); desktop page pad space-7 (48) (tok:435-477).
- radius-sm 8 (thumbs, small tiles, bars, menu rows), radius-md 12 (buttons, inputs, popups), radius-lg 16 (cards, dialogs, drawers, palette), radius-pill 999 (tok:479-501). In-CSS one-offs: toggle 999, checkbox 6, tag chip pill, kbd 6, skeleton bar 6, code/Accordion-cards md.
- z: raised 1, sticky 10, dropdown 20, overlay 50, toast 60, tooltip 70 (tok:581-614). Never raw numbers.
- Breakpoints sm 600 / md 900 / lg 1200 / xl 1440 (tok:616-639). **AppShell actually reflows at 760px of its OWN width via container query (css:722; RM:AppShell:9), not at bp-md as tok:627 claims.** `halo-screen` switches to auto-height at 900px wide OR 600px tall (css:18).

### 1.4 Motion tokens

| Token | Value | Notes |
|---|---|---|
| dur-fast | 150ms | hover/press |
| dur-base | 300ms | state change, sliding thumbs, popups-as-dialog, accordion |
| dur-slow | 600ms | entrances, reveals, stepper fill |
| stagger | 60ms | per item; components cap ~12 (menus use 18ms, tables 25ms, see 3) |
| ease-out | cubic-bezier(0.2, 0.8, 0.2, 1) | the main curve |
| ease-spring | cubic-bezier(0.34, 1.56, 0.64, 1) | flip/grab/add only; **identical to Rift's existing `--ease-spring`** (`rift:app.css:242`) |
| count-up | easeOutCubic `1-(1-p)^3`, default 1100ms | `js:21-37`; NOT the ease-out bezier |
| FLIP | 460ms ease-out; new items 320ms fade+translateY(8px)+scale(.97) | `js:1020-1046` |
| Other fixed durations (not tokens) | page-in 520ms +60ms delay; skeleton-out 360ms (JS timer 380); reveal 600ms; leave 140ms (pop) / 200ms (modal, drawer, banner, notif) / 240ms (toast); roll strip 900ms; bell swing 900ms; ping loop 1.8s; shimmer loop 1.4s; spinner 900ms; indeterminate 1.3s; draw 1.2s; progress count 700ms; Ring count 1100ms; tooltip delay 120ms (show only) |

Doc-vs-CSS mismatches (use the CSS): bk:03:6 says popups scale in at dur-base, but `halo-pop-in` runs at **dur-fast** (css:406) and only dialogs/palette use dur-base (css:285). bk:03:5 says hover = dur-fast, but Card transitions use dur-base (css:30). RM:Button says magnetic leans "up to 6px"; code is translate(dx*6, dy*4) with dx,dy in [-0.5,0.5] so **max +-3px x, +-2px y** (js:75-81).

## 2. Global classes (exact CSS)

```css
/* css:3-4 */ .halo-root{background:var(--bg-000);color:var(--ink);font-family:var(--font-sans);-webkit-font-smoothing:antialiased;color-scheme:dark}
/* css:7-12 scrollbars */ ::-webkit-scrollbar{width:10px;height:10px} ::-webkit-scrollbar-button{display:none;width:0;height:0}
 ::-webkit-scrollbar-track,::-webkit-scrollbar-corner{background:transparent}
 ::-webkit-scrollbar-thumb{background:var(--line-200);border-radius:10px;border:3px solid transparent;background-clip:padding-box}
 ::-webkit-scrollbar-thumb:hover{background:var(--ink-faint);border:3px solid transparent;background-clip:padding-box}
/* css:15-19 */ .halo-screen{height:100vh;height:100dvh;overflow:auto;box-sizing:border-box}
 .halo-scroll{overflow:auto;min-height:0;overscroll-behavior:contain}
 @media (max-width:900px),(max-height:600px){.halo-screen{height:auto;min-height:100vh;min-height:100dvh}}
 .halo-mono{font-family:var(--font-mono);font-variant-numeric:tabular-nums}
/* css:22-27 */ .halo-rise{animation:halo-rise var(--dur-slow) var(--ease-out) both} @keyframes halo-rise{from{opacity:0;transform:translateY(14px)}}
 .halo-page{animation:halo-page-in 520ms var(--ease-out) 60ms both} @keyframes halo-page-in{from{opacity:0;transform:translateY(10px)}}
 .halo-draw{stroke-dasharray:1;stroke-dashoffset:1;animation:halo-draw 1.2s var(--ease-out) forwards} @keyframes halo-draw{to{stroke-dashoffset:0}}   /* needs pathLength="1" */
/* css:63 */ .halo-label{font:500 13px/18px var(--font-sans);color:var(--ink-muted)}   /* defined in the Stat block, used generically (Motion preview) */
/* css:384 */ .halo-sr{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}
/* css:103-104 */ .halo-reveal{animation:halo-reveal var(--dur-slow) var(--ease-out) both} @keyframes halo-reveal{from{opacity:0;filter:blur(6px);transform:translateY(6px)}}
/* css:119-120 */ .halo-row{position:relative;display:inline-flex;align-items:center;gap:var(--space-3);min-height:44px;font:400 14px/20px var(--font-sans);color:var(--ink);cursor:pointer}
/* css:842-847 */ .halo-input{box-sizing:border-box;width:100%;height:44px;padding:0 14px;background:var(--bg-100);border:1px solid var(--line-300);border-radius:var(--radius-md);color:var(--ink);font:400 15px/22px var(--font-sans);outline:none;transition:border-color/box-shadow/background var(--dur-fast) var(--ease-out)}
 .halo-input:hover{background:var(--bg-200)} .halo-input:focus{border-color:var(--accent);box-shadow:var(--shadow-focus);background:var(--bg-100)}
 .halo-input::placeholder{color:var(--ink-faint)} .halo-input:disabled{opacity:var(--opacity-disabled);cursor:not-allowed}
 textarea.halo-input{height:auto;min-height:88px;padding:10px 14px;resize:vertical}
/* css:66-68 */ .halo-chip{display:inline-flex;align-items:center;gap:var(--space-1);font:600 12px/1 var(--font-mono);padding:5px 8px;border-radius:var(--radius-pill);white-space:nowrap}
 .halo-chip--up{background:var(--accent-soft);color:var(--accent)} .halo-chip--down{background:var(--warn-soft);color:var(--warn)}
/* css:901-903 */ @media (prefers-reduced-motion:reduce){*,*::before,*::after{animation-duration:.01ms !important;animation-iteration-count:1 !important;transition-duration:.01ms !important;scroll-behavior:auto !important}}
```

Notes:
- Halo's reduced-motion rule is global `*` + `!important`; it turns every infinite loop into a single 0.01ms tick, so LiveDot ping, Spinner arc and skeleton shimmer FREEZE (spinner looks dead). Rift scopes to `[data-slot]{animation:none!important}` (`rift:app.css:120-122`) and has a JS probe `prefersReducedMotion` (`rift:src/lib/state/accessibility.svelte.ts:101-105`). Halo's JS honours it too: `reduced()` (js:9) skips count-up, FLIP, magnetic, bell swing, skeleton crossfade, roll mount-delay. Decide a static fallback for Spinner/LiveDot.
- Page entrance (`halo-page`) is applied to the CONTENT element only, re-keyed by `pageKey` (js:1676), never the sidebar. No blur on it (bk:03:11).
- The `.halo-toast__x` class is reused as the generic 32px close button in Dialog, Drawer, Banner, Panel, FileDrop (js:819,898,912,937,1629). Port it as a shared `IconClose` button.

## 3. Per component

Format: **Name** `[class]` css range | behaviour in js | d.ts. Class tags: CSS = CSS+markup only; BP = bits-ui primitive + CSS; JS = custom behaviour needed. Sizes are exact from CSS. "Tint" = `color-mix` of a semantic token (Halo's own CSS breaks its "no washes" rule in these spots, see 4).

### 3.1 Layout

**AppShell** `[JS]` css:699-732 | js:1660-1677 | dts:427
- Frame `.halo-shell` 100dvh, `container-type:inline-size`, bg-000. Sidebar `flex:0 0 var(--side-w)` (default 248px via inline `--side-w`, js:1663), padding 16/12, gap 16, border-right line-100, `overflow:hidden`; width + flex-basis transition dur-base ease-out.
- Collapsed rail 72px (css:704): labels/count/title opacity 0 at dur-fast, brand becomes a column, collapse button `scaleX(-1)` (dur-base), nav item padding `0 13px`. Labels stay in DOM for SR.
- Brand min-h 44, logo slot 32x32, title 600 15/20; collapse button 36x36, icon `panel` (18px). Nav: column gap 4, scrolls Y. Footer: border-top line-100, padding-top 12.
- Top bar: sticky, `z-sticky`, min-h 64, padding 12/32, border-bottom line-100, bg-000. Content: `overflow:auto; overscroll-behavior:contain; padding:32`; has `halo-page` and `key={pageKey}` so changing the key replays the 520ms rise.
- `<=760px` of own width: frame becomes a column, sidebar a horizontal strip (padding 8/12, border-bottom), collapse + footer hidden, nav row-scrolls with hidden scrollbar, top padding 12/16, content padding 16 (css:722-732).
- a11y: `<aside aria-label>`, `<nav aria-label>`, collapse `aria-expanded`.
- Note: Rift is a fixed desktop window; the narrow strip mode is unlikely to be needed. Rift already owns its shell (`rift:src/lib/components/shell/`).

**SplitPane** `[JS]` css:856-874 | js:1990-2064 | dts:875
- Grid `px auto minmax(0,1fr)` (js:2053). Bar is a 1px line-100 hairline (`line-300` hover, `accent` when focus-visible/dragging); hit area `inset:0 -6px` (13px), `-12px` on `pointer:coarse` (25px); grip 4x28 pill line-300, opacity 0 -> 1 on hover/focus/drag (dur-fast), accent while dragging/focused.
- Behaviour: pointer-capture drag, rAF-throttled commit, persist on release; arrows 16px (Shift 64), Home/End limits, Enter -> `onCollapsedChange`, dblclick resets to `defaultSize`; `min` default 160, `minSecond` defaults to `min`; `storageKey` -> localStorage `halo-split:<key>`, wrapped in try/catch. `role=separator tabIndex=0 aria-orientation aria-controls aria-valuenow(%)`.
- No transition on size while dragging (content never shifts). No bits-ui primitive.

**SectionLabel** `[CSS]` css:84-88 | js:210 | dts:415: flex gap 12, mono 500 11/14, tracking .12em UPPERCASE, ink-faint; index (zero-padded, `01`) in accent, a literal `/`, text, flexible 1px line-100 rule, optional count in ink-muted. Renders `div|h2|h3`.

**Divider** `[CSS]` css:437-441 | js:1079: 1px line-100; `--v` = inline-block stretch min-h 16; `--label` = mono uppercase caption between two rules.

**Logo** `[CSS]` css:787-790 | js:1735-1751 | dts:834: big mark = 64 viewBox: core r12 solid; ring r19.5 stroke 3 opacity .6; ring r26.5 stroke 3 opacity .28. Small (<32px) = 32 viewBox: core r7.5; ring r13 stroke 3. Fill = `var(--accent)` | `--ink` | currentColor. Name beside it Geist 600, `-0.03em`, font-size `max(14, round(size*.56))`, gap 12.

### 3.2 Surfaces

**Card** `[CSS + action]` css:29-35 | js:43-61 | dts:6
- bg-100, 1px line-100, radius-lg (16), padding 24, no shadow. Hover border line-200. Transitions transform/border/opacity at **dur-base**. `lift` = `translateY(-3px)` on hover.
- `spotlight`: `::before` is a 1px mask-composite ring (`radial-gradient(240px circle at var(--mx) var(--my), color-mix(accent 75%, ink), transparent 72%)`), `::after` a 360px 4% ink sheen; both opacity 0 -> 1 over dur-base on hover. JS: mousemove -> rAF -> set `--mx/--my` px relative to a rect cached 300ms; mouseleave drops the rect. Uses `-webkit-mask-composite:xor` + `mask-composite:exclude` (Chromium fine).
- Spotlight only on a few hero cards (bk:04:7).

**FocusGroup** `[CSS]` css:37-40 | js:64: `grid; gap:16; grid-template-columns:repeat(auto-fit,minmax(<min=220>px,1fr))`; children transition opacity/transform/border dur-base; `--dim`: `:has(>*:hover)>*:not(:hover){opacity:.5}`. Opt-in.

**Accordion** `[BP]` css:481-504 | js:1161-1186 | dts:461
- Rows: item border-bottom line-100; trigger min-h 52, padding `0 4px`, 500 15/22; cards variant: item `bg-100`, 1px line-100 radius-md, gap 8, trigger padding `0 16`, open item border line-200.
- Height anim: `.halo-acc__panel{display:grid;grid-template-rows:0fr;transition:grid-template-rows dur-base ease-out}` -> `1fr` when open; inner `overflow:hidden;min-height:0`. Content fades+rises from `translateY(-4px)` opacity 0 over dur-base with **60ms delay** on open.
- Chevron rotates 180deg (dur-base), icon turns accent when open. Closed panel is `inert`. Heading element wraps the button (`headingAs`).

### 3.3 Actions

**Button** `[CSS]` css:42-56, 401-404 | js:71-88 | dts:24
- md: min-h 44, padding `0 24`, radius-md, 1px transparent border, font `600 14px/1`, gap 8, svg 16. sm: min-h 36, padding `0 16`, 13px. Icon-only: `--icon` width 44 (sm 36), padding 0.
- primary: accent bg, accent-ink text; hover `box-shadow:0 8px 28px var(--glow-strong)` (the only coloured shadow). secondary: bg-200, line-200 border, hover bg-300. ghost: transparent, ink-muted, hover ink + bg-200. danger: transparent, danger text, line-200 border, hover bg-200 + danger border.
- Press `scale:.97` (individual `scale` property, so it does not fight a transform). Transitions transform/bg/border/shadow/color dur-fast. Focus `box-shadow:var(--shadow-focus)`. Disabled opacity .45, no shadow, `not-allowed`.
- `magnetic` (primary only, off by default): see 1.4. Plain trigger variant `.halo-trigger` (css:403-404): no chrome, radius-sm, focus ring.

**Segmented** `[JS]` css:90-96 | js:91-108 | dts:41
- Track: inline-flex, padding 4, gap 4, bg-100, 1px line-100, radius-md. Thumb: absolute `top:4 bottom:4`, radius-sm, accent; `transform:translateX(x) + width` transition dur-base ease-out. Buttons: min-w 52, min-h 36, padding `0 16`, mono 600 13, ink-muted -> ink hover; pressed = accent-ink (colour transition dur-base).
- JS `useIndicator` (js:222-238): `offsetLeft/offsetWidth` of active button, re-measured on `document.fonts.ready` + ResizeObserver. Thumb `opacity:0` until measured. Buttons are `aria-pressed`, group `role=group aria-label`.

**Menu** `[BP: DropdownMenu]` css:405-430, `.halo-pop` 406-413 | js:1267-1334 | dts:488
- Popup `.halo-pop`: abs `top:calc(100%+8px)`, bg-100, 1px line-200, radius-md, shadow-pop, `z-dropdown`, `transform-origin:top left` (end/top variants flip origin). Enter `halo-pop-in`: `from{opacity:0;transform:scale(.96) translateY(-4px)}` **dur-fast** (top side: translateY(4px)). Leave 140ms: opacity 0 + scale(.97).
- List: min-w 220, max-h 360, padding 4, scroll Y. Item: min-h 40, padding `0 12`, radius-sm, ink-muted 400 14/20, gap 12; enter `halo-menu-item` `from{opacity:0;transform:translateY(-3px)}` 220ms, delay `min(i,12)*18ms`. Active: bg-200 + ink, icon accent. Danger: danger text/icon. Disabled .45. Separator 1px line-100 margin `4px 8px`. Heading = label style (mono 11 uppercase).
- Trigger chevron rotates 180deg on `aria-expanded` (dur-base). Keyboard: Up/Down open on trigger, arrows wrap, Home/End, type-ahead by first letter, Enter/Space, Esc returns focus, Tab closes.

**ContextMenu** `[BP: ContextMenu]` css:849-854 | js:1897-1987 | dts:852: same popup/rows as Menu; portaled to body in a fixed layer carrying the nearest `data-theme`; flips to the other side of the pointer near an edge (8px margin) and sets `transform-origin` accordingly; closes on resize/blur/scroll/outside; opens on right-click, 500ms touch long-press (10px slop), Shift+F10 / Menu key. Rift already has bits-ui ContextMenu wrappers + `ContextMenuHost.svelte`.

### 3.4 Forms

**Field** `[CSS]` css:830-847 | js:1840-1874 | dts:196: column gap 6; top row (label 500 13/18 ink-muted, then "Required"/"Optional" tag 12px ink-faint, action link right); message row 12/16 ink-muted, with 14px alert icon + danger when `error`; error turns the child's border danger; disabled -> top row .45. Native input/select/textarea with no class get `.halo-input`. Non-native child => group (`role=group aria-labelledby`).

**TextField** `[CSS]` css:122-138 | js:261-275 | dts:92
- 52px tall; padding `20 14 6`; bg-100; 1px line-300; radius-md; 400 15/22. Hover bg-200. Focus: border accent + shadow-focus.
- Floating label: absolute `left:15 top:15`, 15/22 ink-muted, `transform-origin:left top`; on focus or `:not(:placeholder-shown)` -> `translateY(-9px) scale(.78)` (dur-base); accent colour on focus. **The input must have `placeholder=" "`** (js:267). Icon variant: input pad-left 44, label left 45, icon abs `left:15 top:17` 18px, accent on `:focus-within`. Hint 12/16 ink-muted pad-left 2; error -> danger.

**Textarea** `[JS]` css:584-595 | js:1389-1411 | dts:556: min-h 88, pad `12 14`, resize vertical, same border/focus as input; autoGrow sets `height=min(scrollHeight+2, maxHeight=360)` in a layout effect; counter mono 12 ink-faint, `is-near` (warn) at `left <= max(10, 10%)`, `is-full` (danger) at 0.

**SearchField** `[CSS]` css:597-612 | js:1414-1431 | dts:576: height 44 (sm 36), padding `0 44 0 42`; icon abs `left:14` ink-muted -> accent on focus-within; clear button 32 (sm 28) pops in with `halo-pop-in` dur-fast; Esc clears when there is text; shortcut `Kbd` abs right 10, hidden when text; `loading` swaps icon for 16px Spinner; native cancel button hidden.

**Select** `[BP: Select]` native css:162-171; Halo list css:792-813 | js:329-414 | dts:147
- Trigger = `.halo-select__input` (button, role=combobox): h 44, padding `0 40 0 14`, bg-100, 1px line-300, radius-md, 15/22; hover bg-200; chevron abs right 14, 16px, rotates 180deg + accent while open (dur-base). Open border accent.
- Listbox = `.halo-pop.halo-listbox`: `left:0;right:0;min-width:100%;max-height:288;padding:4`. Option: min-h 40, padding `6 12`, radius-sm, gap 12, ink-muted -> ink when selected, active bg-200+ink; entrance `halo-menu-item` 220ms stagger `min(i,12)*18ms`; description 12/16 ink-muted; meta mono 500 12 ink-muted; 16px accent check on the selected row.
- Behaviour: type-ahead (500ms buffer), arrows/Home/End/PageUp/PageDown(+-8), Alt+Up picks, Esc keeps old value, Tab commits the lit row and moves on, closed + letter changes value in place. Leave 140ms.

**Combobox** `[BP: Combobox]` css:814-828 | js:1754-1837 | dts:170: Select look with 16px search icon left (pad-left 40), clear (32px at right 40) + toggle (32px at right 6); filter = case-insensitive substring of label (and string description); match run rendered `<mark class="halo-option__match">` (ink, 600, accent underline 2px, offset 3px); caps at `maxResults` 50 with a footer "Showing N of M. Keep typing to narrow it down."; empty "No matches"; Esc closed + value => clears; leaving restores the chosen label; SR live region announces result count.

**Toggle** `[BP: Switch]` css:140-148 | js:278-287 | dts:110
- Track 44x26, pill, 1px line-300, bg-300; knob 18 (ink-muted) at `top:3 left:3`. On: accent track + border, accent-ink knob, `translateX(18px)`. Knob transition: transform **dur-base ease-spring**, width dur-fast, bg dur-base. Press stretches knob to **24px** (`translateX(12px)` when on). `role=switch aria-checked`; with a label the whole `.halo-row` (44px) is the target.

**Checkbox** `[BP: Checkbox]` css:150-160 | js:290-301 | dts:125: hidden native input drives a 20x20 box, radius 6, 1px line-300, bg-300; hover row -> border ink-muted; checked -> accent fill + `halo-pop` (`50%{transform:scale(1.18)}`, dur-base, ease-spring) and the check path (viewBox 16, `d="M3.5 8.5l3 3 6-7"`, stroke accent-ink 2.4, round) draws via `stroke-dashoffset 1->0` dur-base. Focus ring on the box.

**RadioGroup** `[BP: RadioGroup]` css:560-582 | js:1363-1386 | dts:541: radio dot 20, 1px line-300, bg-300; inner `::after` inset 4 accent-ink scales 0->1 with **ease-spring dur-base**; checked = accent fill/border. Row min-h 44. `cards` variant: grid `repeat(auto-fit,minmax(180px,1fr))` gap 12, tile pad 16 radius-md border line-200; checked border accent + 6% accent tint; `:active scale(.99)`. Horizontal = row with column-gap 24.

**Slider** `[CSS+JS]` css:173-186 | js:417-433 | dts:217: **native `<input type=range>`**, height 24, track 6px radius 3 painted by `linear-gradient(to right, accent var(--pct), bg-300 var(--pct))` (webkit) with `--pct` set inline from JS; thumb 20px circle ink with 4px accent border; hover scale 1.15, active 1.3 (dur-fast ease-spring); mono 600 13 readout. bits-ui Slider renders spans, not a native range, so the pseudo-element CSS cannot be reused as is.

**DatePicker + Calendar** `[BP: DatePicker/Calendar]` css:629-666 | js:1497-1585 | dts:611,626: field button 44 / min-w 220; popover pad 12; calendar width 296, nav buttons 36, title 600 14/20, header row mono 11 uppercase ink-faint; day cell square (max-h 40), mono 500 13; today = 4px accent dot at `bottom:5`; selected = accent fill + `halo-pop` ease-spring; outside-month ink-faint but clickable; disabled .4. Month change: `halo-cal-right/left` `from{opacity:0;transform:translateX(+-16px)}` dur-base on grid AND month name. Keys: arrows +-1/7, PgUp/PgDn month (clamped day), Home/End week ends. Values are `YYYY-MM-DD` strings; the footer has Today / Clear (ghost sm). Rift already has `@internationalized/date` (`rift:package.json:43`).

**TagInput** `[JS]` css:614-627 | js:1434-1482 | dts:594: container min-h 44 pad `6 8` gap 6 line-300 border, focus-within accent + ring; chip h 28 pill bg-300 pad `0 2 0 10`, 500 13/18, enter `halo-tag-in` (`from{opacity:0;transform:scale(.6)}`, dur-base **ease-spring**); armed chip = 18% danger tint + 1px danger ring; x button 24. Enter/comma add (paste of comma list adds each), dupes ignored case-insensitively, Backspace twice removes, blur adds draft.

**FileDrop** `[JS]` css:668-688 | js:1593-1631 | dts:648: dashed 1.5px line-300 zone, radius-lg, pad `32 24`; over = solid accent border + 7% accent tint + `scale(1.01)` (dur-base), icon 48 circle lifts `translateY(-4px)` and fills accent (ease-spring); file rows pad 12 radius-md bg-100 line-100, rise-in `min(i,8)*50ms`, done icon accent-soft + pop, error border 45% danger tint; thin `Progress sm` while uploading. Drag depth counter prevents flicker. **Tauri caveat:** Tauri 2 can intercept OS file drops (`dragDropEnabled`); whether HTML5 `dataTransfer.files` fires in Rift's window is unknown here.

### 3.5 Data

**Stat** `[JS]` css:58-72, 734-740 | js:119-143 | dts:50: column gap 12; top row label (`halo-label`) + delta chip (`↑ 12.4%` accent-soft/accent, `↓ 0.6%` warn-soft/warn, `delta >= 0` is "up"); value mono 600 30/34 -0.03em nowrap; `peek` makes the stat focusable and underlines the value dotted line-200 (offset 7), peek popup abs `top:100%` mt 8, min-w 220, bg-200, line-200, shadow-pop, `z-dropdown`, opacity 0 + `translateY(-4px)` -> shown on hover/focus (dur-fast). Count-up: rAF, **easeOutCubic, 1100ms default**, starts from the LAST displayed value (`from` ref) so range changes animate (js:21-37). Number format is hard-coded `en-US` + `$` (js:110-116). `roll` swaps to RollingNumber with `--roll-h:34px`.

**Badge** `[CSS]` css:228-235: inline-flex gap 6, pad `3 10`, pill, 500 12/18; neutral bg-300/ink-muted; accent/info/warn/danger use their `-soft` bg + solid text; optional 6px dot (currentColor).

**DataTable** `[JS]` css:237-256 | js:545-588 | dts:277: `th` 500 12/16 ink-faint pad `0 12 12`, sticky `top:0` `z-raised` with `box-shadow:inset 0 -1px 0 line-100`; `td` pad 12, bottom border line-100; numeric cols right-aligned mono. Row hover bg-200 (dur-fast). Rows ripple in (`halo-rise` at dur-base, delay `min(i,12)*25ms`). Sort button min-h 28; arrow `↓` opacity 0 -> .5 on hover -> 1 accent when active, rotates 180deg for asc (dur-base). First click = desc. With `rowKey`, rows get `data-flip` and glide via `useFlip` (deps rows+sort). Scroll box via `maxHeight`; `aria-sort` on `th`.

**Avatar** `[BP: Avatar]` css:463-472 | js:1124-1145: sizes xs24/sm28/md36/lg48/xl64; bg = 22% tint of a chart tone (hash of name mod 4) over bg-200; initials 600, font-size `round(px*.38)`; photo fades in over dur-base after load, falls back on error; status dot `max(8, round(px*.26))` with 2px bg-100 ring (online accent, away warn, busy danger, offline ink-faint); aria-label "Name, status".

**AvatarGroup** `[CSS]` css:474-479 | js:1148-1158: overlap `--ov = round(px*.3)`, 2px bg-100 ring; hover (spread) margin-left 4 over dur-base; hovered avatar `translateY(-3px)`; "+N" chip bg-300 mono.

**Kbd** `[CSS]` css:305, 443-444: min 22x22, pad `0 6`, radius 6, 1px line-200, bg-200, ink-muted, mono 500 11; group gap 4.

**CodeBlock** `[CSS+JS]` css:876-899 | js:2068-2116 | dts:903: bg-100 + 1px line-100 radius-md (`terminal` = bg-000); header min-h 40 pad `0 8 0 16` with mono 500 13 title, `language` label; copy button h 28 pad `0 10`, 12px text, "Copied" (accent) / "Copy failed" (danger) for 1800ms, clipboard API then `execCommand` fallback; `pre` mono 400 13/20 tab-size 2, focusable (inset 2px accent ring); line numbers 3ch ink-faint (not copied); `highlight` = accent-soft row + accent number; terminal prefixes `$ > #` ink-faint, output ink-muted. **No syntax colouring by design** (RM:CodeBlock:10). Rift renders code with Shiki (`rift:package.json:46-48,59`), so decide where Halo's CodeBlock applies (chrome/frame only?).

### 3.6 Charts

**Sparkline** `[JS util]` css:116-117 | js:146-155: default 96x36, pad 3, stroke 2 round, `halo-draw` 1.2s ease-out, tone via `var(--chart-N)`. `smoothPath` (js:11-19): cubic segments with control x at the midpoint `c=(x0+x1)/2`, control y = each endpoint's y (horizontal tangents).

**LineChart** `[JS]` css:188-206 | js:436-491 | dts:233: SVG viewBox 800 x height (default 240) `preserveAspectRatio=none`, y range `min-15%span .. max+10%span`; 4 gridlines line-100; main stroke 2.5 accent round; area `accent-soft` fades in dur-slow after 0.4s; compare dashed `5 6` stroke 1.5 ink-faint, non-scaling-stroke; re-keys on data change to redraw the 1.2s draw. Hover: nearest-index overlay with dashed line-200 rule, 12px accent dot + 5px accent-soft ring, tooltip `bg:ink color:bg-000` pad `8 12` radius-md shadow-pop; left/top/dot all transition dur-fast so it glides; tooltip x-anchor flips at <12% / >88%. X ticks mono 500 11 ink-faint, every `n>14 ? ceil(n/6) : n>8 ? 2 : 1`.

**BarChart** `[JS]` css:208-218 | js:494-513 | dts:247: columns gap 10; bar bg-300, peak accent, radius `8 8 3 3`; height transition **dur-slow ease-out** with `--d = i*40ms`, set to 0% until after two rAFs (`useMounted`, js:240-248); value label opacity 0 -> 1 for peak/hovered (dur-fast, +4px rise); non-hovered columns dim to .45 (dur-base); non-peak hover bg line-200.

**Ring** `[JS]` css:220-226 | js:516-531 | dts:258: default 160 / stroke 12; track bg-300; arc round cap, `strokeDashoffset = C*(1-v)` driven by count-up (1100ms easeOutCubic); centre value mono 600 `round(size*.19)`px.

### 3.7 Navigation

**Tabs** `[BP: Tabs + measured bar]` css:258-266 | js:591-614 | dts:300: gap 24, border-bottom line-100, x-overflow hidden scrollbar; tab min-h 44, 500 14/20, ink-muted -> ink (dur-base); count pill mono 11 bg-300 -> accent-soft/accent when selected; 2px accent underline (`transform:translateX + width`, dur-base ease-out, `useIndicator`); roving tabindex, Left/Right arrows select + focus (wraps).

**NavItem** `[CSS]` css:268-278 | js:617-626: min-h 44, pad `0 12`, radius-md, 500 14/20 ink-muted, gap 12; hover bg-200 + ink (dur-fast); active bg-200 + ink + icon accent + `aria-current=page`, and on mount `halo-nav-in` (`from{background:transparent;color:ink-muted}` dur-slow) fades the highlight in; icon 18; count mono 11 ink-faint.

**Breadcrumbs** `[CSS+state]` css:506-519 | js:1189-1207: 500 13/18; link min-h 28 pad `0 6` radius-sm ink-muted -> ink + bg-200; current ink with `aria-current`; chevron separators ink-faint; each item enters `halo-crumb-in` (`from{opacity:0;transform:translateX(-4px)}` dur-base, delay `min(i,6)*40ms`); beyond `maxItems`(5) the middle folds into a "…" button that expands.

**Pagination** `[BP: Pagination + measured thumb]` css:521-532 | js:1210-1242: buttons min 40x40 radius-sm mono 500 13; accent thumb slides under the current page (transform+width dur-base); prev/next 16px chevrons; `…` gaps; summary 13px ink-muted ml 12.

**Stepper** `[CSS]` css:534-558 | js:1245-1264 | dts:717: mark 32 circle, mono 600 13; track 2px bg-300 at `top:15`, fill accent `scaleX(var(--progress))` dur-slow ease-out (`--n` steps, `--progress = cur/(n-1)`); done = accent fill + `halo-pop` + check path draw (`halo-draw-check` dur-base after 120ms); current = bg-100 fill, accent border + 4px accent-soft ring; vertical orientation uses `scaleY`.

**CommandPalette** `[BP: Command + Dialog]` css:292-305 | js:629-672 | dts:320: overlay `--top` (align flex-start, padding-top 12vh), palette width `min(560px,100%)` radius-lg bg-100 line-200 shadow-pop, enter `halo-zoom`; search row h 52 + Esc kbd; list max-h 320 pad 8; item min-h 44 pad `0 12` radius-sm, active bg-200 + accent icon; group label 12px ink-faint right; empty "No matches for “q”". Note: Halo's palette has NO leave animation (returns null when closed); Rift has `rift:src/lib/components/dialogs/CommandPalette.svelte`.

### 3.8 Overlays

Overlay: `.halo-overlay` fixed inset 0, `z-overlay`, flex centred, pad 16, `background:var(--scrim)`, `animation:halo-fade var(--dur-fast)` (css:281); `.is-leaving` opacity 0 over 200ms (css:284). `contained` = `position:absolute`. Backdrop click closes (mousedown on the overlay itself).

**Dialog** `[BP: Dialog/AlertDialog]` css:285-291 | js:807-825 | dts:351: width `min(480px,100%)`, max-h `calc(100%-32px)`, pad 24, radius-lg, bg-100, **1px line-200**, shadow-pop, column gap 16. Enter `halo-zoom` `from{opacity:0;transform:scale(.94) translateY(8px)}` dur-base ease-out; leave `scale(.96)` 200ms. Title 600 20/28 -0.02em; desc 400 14/21 ink-muted; footer right-aligned wrap gap 8 (secondary first, primary last). 32px close X. Focus moves in, Tab trapped, focus returns, body scroll locked (ref-counted, js:981-994).

**Drawer** `[BP: Dialog]` css:359-368, 690-697 | js:1634-1657 | dts:527: overlay `--side` (justify end, stretch, pad 12), drawer `min(420px,100%)` (NotificationPanel 400), radius-lg, bg-100, line-200, shadow-pop; enter `halo-drawer-in` `from{opacity:0;transform:translateX(40px)}` dur-base (left: -40px); leave 200ms opacity 0 + translateX(40px). Head pad `12 12 12 24` border-bottom line-100, title 600 16/22; body scrolls, pad 24; footer border-top line-100 pad `12 24`.

**Popover** `[BP: Popover]` css:432-435 | js:1337-1360 | dts:507: `.halo-pop` (see Menu) + panel pad 16 gap 12, default width 300, title 600 14/20, footer right gap 8; Esc returns focus; outside pointerdown closes.

### 3.9 Feedback

**Tooltip** `[BP: Tooltip]` css:307-311 | js:675-689 | dts:331: pure CSS, NO collision handling: bubble abs `bottom:100%` centred, mb 8, `max-width:240`, pad `6 10`, radius-sm, **bg `ink` / text `bg-000`** (inverted), 500 12/16, `z-tooltip`; hidden = opacity 0 + `translate(-50%,4px)`; shown on hover/`:focus-within` with `transition-delay:120ms` ONLY on show (hide is immediate) over dur-fast. Linked via `aria-describedby`.

**Toast** `[JS]` css:313-335 | js:692-804 | dts:337
- Viewport fixed `right/bottom:24`, `width:min(360px,100%-32px)`, `z-toast`; list is `ol aria-live=polite`; toasts absolutely stacked: `transform: translateX(var(--dx)) translateY(calc(var(--y)*-1)) scale(var(--s))`, `opacity: var(--o)*var(--fade)`, origin `50% 100%`, transition transform+opacity dur-base. Toast pad `12 12 12 16`, radius-md, **bg-200**, 1px line-200, shadow-pop; 8px tone dot; title 500 14/22; desc 400 13/18 ink-muted; action slot; 32px X.
- Collapsed: offset `i*10px`, scale `max(.86, 1-i*.05)`, items i>2 opacity 0, rear items' content hidden (`is-behind`), viewport height = front toast + up to 2*10. Expanded (hover/focus/`expanded`): offset = cumulative `(height+8)`, scale 1; hover/focus pauses timers.
- Enter `halo-toast-in` `from{opacity:0;transform:translateY(16px) scale(.96)}` dur-base. Leave 240ms (`translateX(+32px)` + fade); swiped = `translateX(+110%)`.
- Timers (js:692-733): default 4000ms, `duration:0` = sticky, max 5 kept, pause keeps `left = max(600, remaining)`, removal 260ms after dismiss.
- Drag (js:735-759): pointer capture on non-button area; leftward drag damped x0.2; `--fade = max(.2, 1-|dx|/260)`; dismiss if `dx > 90` or (`dx > 24` and velocity `> 0.6 px/ms`), else spring back (`--dx:0`, 300ms transition).
- Rift has `ToastHost.svelte` (svelte `fly` + `animate:flip`, 180ms) to be replaced/reskinned.

**EmptyState** `[CSS]` css:337-343: column centred gap 12 pad `48 24`; icon disc 56 bg-200 accent with **concentric rings drawn by box-shadow** (`0 0 0 10px bg-100, 0 0 0 11px line-100, 0 0 0 22px bg-100, 0 0 0 23px line-100`); title 600 16/22; desc max 340 14/21 ink-muted; one action.

**Skeleton** `[JS state]` css:98-114 | js:166-207 | dts:82: bone = 12px bg-200 radius 6 with `::after` linear-gradient(90deg, transparent, bg-300, transparent) sliding `translateX(-100%->100%)` 1.4s linear infinite. Variants text (widths 100/82/64/90/72%), stat (38% label + 56x22 pill + 30px 62% value), list (36px circle + 2 bars, gap 16). On `loading` true->false: content mounts inside `halo-reveal` (600ms, from opacity 0, blur 6px, translateY 6px) while a copy of the bones overlays absolutely and plays `halo-skel-out` (360ms, to opacity 0 blur 4px); removed after 380ms. `aria-busy` while loading.

**Progress** `[BP: Progress]` css:453-461 | js:1105-1121 | dts:730: track 8px (sm 4) pill bg-300 overflow hidden; bar `scaleX(shown)` origin left, `shown` driven by count-up **700ms** easeOutCubic; mono 600 13 percent; indeterminate = 40% bar sliding `translateX(-100%->250%)` 1.3s ease-out infinite. Tone via `--chart-*`.

**Spinner** `[CSS]` css:446-451: 24 viewBox, r9, track bg-300 width 3, arc `stroke-dasharray:18 100` round, `halo-spin` 900ms linear infinite; `role=status` + sr text "Loading".

**LiveDot** `[CSS]` css:74-82: 8px dot accent/info/warn/idle(ink-faint); `::after` same colour `halo-ping` (`from{scale(1);opacity:.7}to{scale(3);opacity:0}`) 1.8s ease-out infinite; idle has no ping; always paired with text.

### 3.10 Notifications

**NotificationBell** `[CSS]` css:345-357 | js:843-862: 44x44 button radius-md bg-200 line-200; open = accent colour+border; press `scale:.96`; count pill min 20x20 mono 600 11, accent bg, 3px bg-000 ring, abs `-7,-7`, enter `halo-pop` ease-spring; **only when count increases** (and not reduced-motion): icon `halo-bell-swing` 900ms ease-out (keyframes 12% 16deg, 28% -13deg, 44% 9deg, 60% -5deg, 76% 2deg; origin `50% 3px`) and `halo-bell__ping` 900ms.

**NotificationPanel** `[BP: Dialog]` css:359-384 | js:875-918 | dts:391: Drawer anatomy + groups (label style) + rows: `max-height:160`, radius-md, hover bg-200; enter `halo-rise` 500ms backwards, delay `min(i,8)*50ms`; unread dot 8px with 4px 18% tone halo; dismiss: row slides `translateX(24px)`, fades, `max-height:0` over 200ms, then `onDismiss`; X shows on hover/focus-within. Empty state "You're all caught up".

**Banner** `[CSS]` css:386-396 | js:926-939 | dts:405: pad `12 12 12 16`, radius-md, bg = 8% tone tint over bg-100, border 32% tone over line-100, 28px round tone icon, title 500 14/20, desc 13/18 ink-muted; enter `halo-rise` dur-base; dismiss 200ms opacity+`translateY(-6px)`; `danger` -> `role=alert`, others `status`.

### 3.11 Motion

**RollingNumber** `[JS]` css:734-740 | js:1062-1076: each digit = a 10-row strip (`0..9`) in an `overflow:hidden` column of height `--roll-h` (1.15em; 34px in Stat), `translateY(-d*10%)`, **transition 900ms ease-out, delay `min(posFromRight,8)*45ms`** (right-most first); starts at 0 until two rAFs after mount; non-digits static; `halo-sr` carries the whole number once.

**FlipList / useFlip** `[JS -> animate:flip]` css:742-743 | js:1020-1059 | dts:765: measure rects of `[data-flip]` children relative to the container in a layout effect; on change `el.animate([{transform:translate(dx,dy)},{transform:none}], {duration:460, easing:ease-out})`; new keys `animate([{opacity:0,transform:'translateY(8px) scale(.97)'},{...none}], 320ms)`; only re-measures when the key list changes; cancels in-flight `halo-flip` animations first.

### 3.12 Game category (out of scope for Rift)

GameFrame css:745-769, KeyPrompt 771-776, Countdown 778-783 (`halo-countpop` 900ms). FiveM-NUI mini-games only; skip. Do note `game-glass` and `backdrop-filter` live ONLY here.

### 3.13 Keyframe inventory (every `@keyframes` in bundle.css)

| Name | css | Definition | Used by |
|---|---|---|---|
| halo-rise | 23 | `from{opacity:0;transform:translateY(14px)}` | `.halo-rise` (600ms), table rows (300ms), notif rows (500ms), files, banner |
| halo-page-in | 25 | `from{opacity:0;transform:translateY(10px)}` | `.halo-page` 520ms +60ms |
| halo-draw | 27 | `to{stroke-dashoffset:0}` | lines, sparkline (1.2s) |
| halo-ping | 82 | `from{scale(1);opacity:.7}to{scale(3);opacity:0}` | LiveDot 1.8s loop; bell ping 900ms once |
| halo-shimmer | 102 | `to{translateX(100%)}` | skeleton 1.4s linear loop |
| halo-reveal | 104 | `from{opacity:0;filter:blur(6px);transform:translateY(6px)}` | skeleton content in |
| halo-skel-out | 107 | `to{opacity:0;filter:blur(4px)}` | skeleton bones out 360ms |
| halo-pop | 160 | `50%{transform:scale(1.18)}` | checkbox, stepper done, selected day, bell count, file done; ease-spring dur-base |
| halo-fade | 195 | `from{opacity:0}` | overlay scrim (dur-fast), line area (dur-slow +.4s) |
| halo-zoom | 287 | `from{opacity:0;transform:scale(.94) translateY(8px)}` | dialog, palette (dur-base) |
| halo-toast-in | 324 | `from{opacity:0;transform:translateY(16px) scale(.96)}` | toast (dur-base) |
| halo-bell-swing | 354 | 12% rot16, 28% -13, 44% 9, 60% -5, 76% 2 (deg) | bell 900ms |
| halo-drawer-in / -left | 363 / 692 | `from{opacity:0;transform:translateX(40px / -40px)}` | drawer (dur-base) |
| halo-pop-in / -up | 411 / 413 | `from{opacity:0;transform:scale(.96) translateY(-4px / 4px)}` | menus, popovers, listboxes, clear buttons (dur-fast) |
| halo-menu-item | 420 | `from{opacity:0;transform:translateY(-3px)}` | menu + option rows 220ms, `min(i,12)*18ms` |
| halo-nav-in | 272 | `from{background:transparent;color:var(--ink-muted)}` | active nav (dur-slow) |
| halo-crumb-in | 509 | `from{opacity:0;transform:translateX(-4px)}` | crumbs dur-base, `min(i,6)*40ms` |
| halo-draw-check | 546 | `to{stroke-dashoffset:0}` | stepper check dur-base +120ms |
| halo-tag-in | 621 | `from{opacity:0;transform:scale(.6)}` | tag chip dur-base ease-spring |
| halo-cal-right / -left | 650-651 | `from{opacity:0;transform:translateX(+-16px)}` | calendar dur-base |
| halo-spin | 451 | `to{rotate(360deg)}` | spinner 900ms linear loop |
| halo-indet | 461 | `from{translateX(-100%)}to{translateX(250%)}` | progress 1.3s ease-out loop |
| halo-game-in / countpop | 749 / 783 | game only | skip |

Motion that has NO CSS (JS-driven; numbers from bundle.js): count-up (Stat, Ring, Progress), toast stack positions and drag, FLIP (WAAPI), spotlight position, magnetic lean, segmented/tab/pager indicator measuring, roll strip positions, textarea auto-height.

## 4. Rules (the contract)

Principles (README:5-11): depth by lightness; one accent; instrument-panel organisation (8px grid, bento, numbered SectionLabels, card anatomy label -> big value -> detail); motion with manners; interaction that rewards attention.

1. **Depth by lightness, not boxes.** bg-000 -> 100 -> 200 -> 300 (bk:01:23-28). Cards: line-100 hairline -> line-200 on hover. Cards NEVER cast shadows (tok:514).
2. **No colour washes; the only gradient is `Card spotlight`** (a light following the cursor along the hairline; faint 4% sheen). In practice Halo's own CSS uses small semantic tints (Banner 8%/32%, radio-card 6%, drop-zone 7%, avatar 22%, armed tag 18%, notif dot halo 18%, error file border 45%, timer 18%). Reading: tints of a semantic token are tolerated, decorative gradients/washes are not.
3. **One accent.** Primary action, live state, positive change, focus, main series. `glow-strong` only under a hovered primary; `shadow-pop` only on popovers/menus/dialogs/drawers/toasts; `shadow-focus` on every focused control.
4. **Status** = accent/accent-soft (positive), warn/warn-soft (negative), info (informational), danger (destructive); always plus arrow/icon/word, never colour alone. Deltas carry arrow + sign.
5. **Charts**: series chart-1..4 in order; comparison = dashed ink-faint; one accent per chart.
6. **Motion**: three speeds on ONE curve (`ease-out`); `ease-spring` ONLY for things you flip/grab/add (toggle knob, checkbox/radio pop, slider thumb, tag chips, chosen calendar day, drop-zone icon, bell count pill, stepper done mark, game pips); entrances stagger once and cap at ~12 items; never loop for attention; **loops only for real activity** (LiveDot, Spinner, indeterminate Progress, skeleton shimmer, game targets); `opacity` + `transform` only for continuous motion; **blur only on small areas** (skeleton fade, countdown veil); no blur on page change; page change glides content only, sidebar stays.
7. **Everything respects `prefers-reduced-motion`**: counts/rolls land instantly, lines appear drawn, glides and pulses stop.
8. **44px touch targets** (bk:02:15); `sm` 36px only inside dense toolbars (RM:Button:6). Disabled = opacity .45 everywhere.
9. **Layers by token**, never raw z numbers. Breakpoints written as numbers with the token named in a comment.
10. **Content**: sentence case ("Live activity"), no emoji, no exclamation marks in system text, empty states say what will appear + one action, unknown facts as `[PLACEHOLDER]`.
11. **Mono for numbers/labels**, sans for words; tabular figures everywhere numbers move.
12. **Icons**: inline stroke SVG on a 24 grid, `stroke-width` 1.8, round caps/joins, `currentColor`, Lucide-style; 16-18px beside text with a space-2 gap; icon-only buttons get `ariaLabel` + usually a Tooltip. Rift already ships `@lucide/svelte` (`rift:package.json:44`).
13. **Fit one screen**: app screens don't page-scroll; lists scroll inside their card with pinned headers; thin scrollbars (10px, no arrows); sideways-only strips hide the scrollbar.
14. **Perf**: spotlight on a few hero cards only; hover effects at most once per frame (rAF); lists re-measure only on order/data change; magnetic + FocusGroup dim are opt-in, one hero moment per page.
15. **A11y** (bk:06): every control a real `<button>/<a>/<input>`; visible `shadow-focus`; modals take focus, trap Tab, restore focus, lock scroll (not when `contained`); Tooltip uses `aria-describedby`; rolling numbers expose the whole value once; avatars announce name + status; Stepper announces "completed"/"current step".
16. **Rules of thumb for picking controls** (README:34-45): Toggle instant vs Checkbox with a button; Radio few-with-explanations vs Segmented filter vs Select many vs Combobox search; Tabs = views; Menu does, NavItem goes; Tooltip names, Popover explains, Dialog asks, Drawer edits; Skeleton in cards, Spinner in buttons/fields, indeterminate Progress for long jobs; Toast "just happened", NotificationPanel history, Banner persistent problem.

## 5. What porting to Svelte needs

Environment (verified): bits-ui 2.19.3 ships accordion, alert-dialog, avatar, calendar, checkbox, collapsible, combobox, command, context-menu, date-picker, dialog, dropdown-menu, menu, meter, pagination, popover, progress, radio-group, scroll-area, select, separator, slider, switch, tabs, toggle, toggle-group, toolbar, tooltip (+ more; `ls node_modules/bits-ui/dist/bits`). Rift wraps only alert-dialog, button, context-menu, dropdown-menu, select, separator (`rift:src/lib/components/ui/`). Rift also has `src/lib/actions/tooltip.ts`, `dialogs/CommandPalette.svelte`, `ToastHost.svelte`, `shell/ContextMenuHost.svelte`, `shell/NotificationCenter.svelte`. Svelte 5.57.1 gives `svelte/animate` flip (already used, `ToastHost.svelte:9,52`), `svelte/transition`, `svelte/motion` (Tween/Spring).

### 5.1 Classification

| Group | Components | Approach |
|---|---|---|
| **CSS-only** (port css range + markup) | Card (+tiny spotlight action), FocusGroup, Divider, SectionLabel, Badge, Kbd, LiveDot, Spinner, EmptyState, Banner (+leave flag), NavItem, Breadcrumbs (+fold state), Stepper, AvatarGroup, Logo, Field, TextField, SearchField, Button, NotificationBell (+ring-on-increase), Sparkline | plain `.svelte` + a Halo `.css`; no primitive needed |
| **bits-ui primitive + Halo CSS** | Menu -> DropdownMenu; ContextMenu -> ContextMenu; Select -> Select; Combobox -> Combobox; Dialog -> Dialog/AlertDialog; Drawer -> Dialog restyled; CommandPalette -> Command in Dialog; Popover -> Popover; Tooltip -> Tooltip; Accordion -> Accordion; Tabs -> Tabs; Toggle -> Switch; Checkbox -> Checkbox; RadioGroup -> RadioGroup; Slider -> Slider (spans, rewrite pseudo-element CSS); DatePicker/Calendar -> DatePicker/Calendar; Progress -> Progress; Avatar -> Avatar; Pagination -> Pagination | bits-ui supplies keyboard, focus return, ARIA, typeahead, positioning (floating-ui) and portals; apply Halo classes/tokens |
| **Custom behaviour** | AppShell (Rift's own shell stays), SplitPane, Toast stack + `useToasts`, FlipList/useFlip, RollingNumber, Stat/Ring/Progress count-up, Skeleton crossfade, DataTable (sort+flip), LineChart/BarChart/Ring (SVG + smoothPath), Segmented/Tabs/Pagination sliding indicator (`useIndicator`), TagInput, FileDrop, Textarea autogrow, CodeBlock copy, NotificationPanel | Svelte actions/attachments + runes classes |

Suggested Svelte equivalents for the behaviour-heavy ones:
- **count-up**: `Tween` from `svelte/motion` with a custom easing `t => 1-(1-t)**3`, duration 1100; reduced-motion => set instantly. Keep the "start from last displayed value" semantics (Tween does this by default on `.target =`).
- **useFlip / FlipList**: `animate:flip` is close but uses its own easing/duration signature; for an exact match write a `use:flip` action using WAAPI as in js:1020-1046 (460ms, `--ease-out`), or pass `animate:flip={{duration:460, easing: cubicBezier(.2,.8,.2,1)}}`. Needs a bezier easing fn (Svelte ships none for this curve).
- **Sliding indicator** (Segmented/Tabs/Pagination): action that reads `offsetLeft/offsetWidth`, re-measures on `document.fonts.ready` and ResizeObserver; with bits-ui Tabs, read the `[data-state=active]` trigger.
- **Toast**: no primitive. Port `ToastViewport` + `ToastItem` literally (stack offsets, `--y/--s/--o/--dx/--fade`, pointer-capture drag with the 90px / 24px+0.6px/ms thresholds, pause/resume with the 600ms floor).
- **Tooltip**: Halo's CSS-only bubble has no collision handling; use bits-ui Tooltip (floating-ui) and keep the 120ms show-delay, inverted colours, 4px rise. Rift's existing `actions/tooltip.ts` may already cover it; unknown whether it positions with collision.
- **RollingNumber**: straightforward `{#each}` of digit strips, transition-delay by position from the right.
- **SplitPane**: pointer-capture + `ResizeObserver`; persist with try/catch; no library.
- **Spotlight/magnetic**: one rAF-throttled pointer action each.

### 5.2 Port-specific traps

1. **Enter/exit animation hook.** Halo animates exits with an `.is-leaving` class + `usePresence` timers (140/200ms). bits-ui exposes `data-state="open|closed"` (`bits-ui/dist/internal/attrs.js:13`) and `data-starting-style` / `data-ending-style` (attrs.js:18-23); it unmounts when no animation is running (see comment `rift:app.css:117-119`). Implement Halo's exit as `[data-state=closed]` keyframes or transitions on `[data-ending-style]`.
2. **Rift's existing floating recipe may be inert.** `rift:src/lib/components/ui/recipes.ts:20,58,60` key off `data-open:` / `data-closed:` variants. A grep of `bits-ui/dist` finds ZERO occurrences of `data-open`/`data-closed`, `tw-animate-css` defines no such variant, and Rift has no `@custom-variant` (grep of `src/`). So those enter/exit animations may never fire. Not verified at runtime (no app runs allowed this wave); verify via the CDP bridge before building on `floatingMotion`.
3. **Transform origin.** bits-ui floating content exposes `--bits-floating-transform-origin`, `--bits-floating-anchor-width/-height`, `--bits-floating-available-width/-height` (`bits-ui/dist/internal/floating-svelte/floating-utils.svelte.js:18-22`). Map to Halo's `transform-origin: top left` on `.halo-pop` (css:406) and the listbox `min-width:100%` (css:800).
4. **Portals vs `.halo-pop` positioning.** Halo's popups are `position:absolute` inside the trigger's wrapper (css:405-406) and would clip inside `overflow:hidden` parents (AppShell sidebar, css:703); bits-ui portals escape that, so drop the abs positioning, keep surface + motion.
5. **WebView2**: Rift bans `backdrop-filter` (`rift:design-system/DESIGN.md:297`, scrim is opaque). Product-category Halo uses none (only `filter:blur()` on skeleton fade, css:104,107). Game glass/countdown veil use backdrop blur: skip those. `color-mix`, container queries, `grid-template-rows:0fr` transitions, `scale:`/`translate:` properties and `mask-composite` are all fine on evergreen WebView2.
6. **Required inline vars / attributes** (will silently break if forgotten): `placeholder=" "` on TextField input (css:129); `--pct` on Slider (css:178); `--n`/`--progress` on Stepper (css:541-542); `--i` stagger index on menu/option/table/notif/crumb/file rows; `--tone` on Avatar/Banner/Notif; `--ov` on AvatarGroup; `--side-w` on AppShell (css:703); `--mx/--my` on spotlight cards; `--y/--s/--o/--dx/--fade` on toasts; `--roll-h`.
7. **Individual-property `scale:`** is used for button/bell press (css:54,350); keep it so magnetic `transform` doesn't collide.
8. **Reduced motion**: match Halo's JS gates (js:9 callers) plus a static fallback for Spinner/LiveDot (see 2).
9. **`en-US` and `$` are hard-coded** in `formatValue` (js:110-116); Rift's Stat-like numbers (tokens, cost, time) need their own formatter.
10. **Toggle/Switch knob**: bits-ui Switch exposes `data-state=checked`; Halo's CSS uses `aria-checked="true"` selectors (css:143-146), so retarget selectors.

## 6. Font loading

- Halo loads Geist 400-700 + Geist Mono 400-600 from Google Fonts via `@import` on line 1 of bundle.css (css:1). Halo's perf note also tells you to `preconnect` to fonts.gstatic.com (bk:04:9).
- **That cannot work in Rift.** CSP is `font-src 'self' data:`, `style-src 'self' 'unsafe-inline'`, `connect-src` without Google (`rift:src-tauri/tauri.conf.json:28`): both the stylesheet and the woff2 would be blocked. It would also break offline, add a network round-trip/FOUT and leak the user's IP to Google from a local-first app. **Strip css:1 and self-host.**
- Precedent: ConsoleX (NUI has no internet) uses `@fontsource-variable/geist` + `@fontsource-variable/geist-mono` ^5.3.0 (`consolex/panel/package.json:32-33`, imported in `main.ts:1-2`). Rift already uses the same mechanism for Inter / JetBrains Mono / Lexend (`rift:src/routes/+layout.svelte:2-4`), so the swap is two imports + two deps.
- **Family names differ**: fontsource registers `'Geist Variable'` and `'Geist Mono Variable'` (`node_modules/@fontsource-variable/geist/index.css:3`, `geist-mono/index.css:3`), not `"Geist"` as tokens.css:60-61 spells it. Override `--font-sans/--font-mono` like ConsoleX does (`consolex/panel/src/app.css:57-59`), keeping fallbacks.
- Weight axis is 100-900 variable, so 400/500/600 all work from one file set. Size on disk (normal style): Geist cyrillic-ext 7.4KB, cyrillic 15.1KB, vietnamese 8.0KB, latin-ext 16.5KB, **latin 29.4KB**; Geist Mono 6.2 + 12.9 + 7.7 + 14.7 + **23.1 (latin)** + 5.8 (symbols2). All subsets ship in the bundle (~146KB) but the browser only fetches the ranges it renders (latin ~52.5KB for both).
- **Italics**: `import "@fontsource-variable/geist"` loads normal only; italic lives in `wght-italic.css` (separate files exist). Rift renders markdown emphasis, so without importing the italic CSS it gets synthesized slant.
- `font-display: swap` is baked into fontsource; with local asset serving the swap flash is negligible.
- Rift specifics to reconcile: Lexend stays as the dyslexia-mode font and brand/splash face (`rift:app.css:747-756`, `SplashOverlay.svelte:201`, `AssistantWelcome.svelte:474`, settings `SettingsPage.svelte:902-909`) and Halo says Geist only; keep Lexend as an accessibility override unless the owner says otherwise. Drop Inter's OpenType flags (`font-feature-settings:"cv02","cv03","cv04","cv11","ss01"`, `rift:app.css:306`) and the body `letter-spacing:-0.005em` (`:314`); Halo applies tracking only to display/title/subtitle/metric.
- Remove `design-system/styles.css:8` Google import only if that docs folder is rebuilt; it is documentation, not shipped.

## Appendix A. Rift token <-> Halo token (collisions the port must resolve)

| Rift (`rift:src/app.css`) | Value now | Halo equivalent | Value |
|---|---|---|---|
| --bg (:132) | oklch(.142) ~#0C0D0E | bg-000 | #08090c |
| --bg-inset (:133) | .178 | no direct; between bg-000/100 | |
| --bg-elev-1 / --surface (:134,137) | .215 ~#17191B | bg-100 | #0e1014 |
| --bg-elev-2 / --surface-hover (:135,138) | .262 | bg-200 | #14171c |
| --bg-elev-3 / --surface-active (:136,139) | .300 | bg-300 | #1b1f26 |
| --border (:156) | .300 | line-100 | #1f242c |
| --border-strong (:157) | .365 | line-200 (cards/popups) or line-300 (controls) | #2c323c / #5f6774 |
| --fg / --fg-2 / --fg-muted / --fg-subtle / --fg-faint (:161-165) | 5 steps | ink / (none) / ink-muted / ink-faint / (none; below ink-faint) | 3 steps only |
| --accent (:174, hue/chroma dials) | emerald, user-tunable | accent (fixed preset) | #6aa8ff |
| --ok / --warn / --danger / --info (:189-197) | green/amber/red/blue | (no ok) / warn / danger / info(lime in blue) | |
| --radius-sm / --radius / --radius-lg / --r-card (:201-205) | 6 / 8 / 10 / 12 | radius-sm / - / - / radius-lg | 8 / 12 / 16 |
| --dur-fast / base / slow (:247-249) | 140 / 240 / 520ms | dur-fast / base / slow | 150 / 300 / 600ms |
| --stagger (:246) | 62ms | stagger | 60ms |
| --ease-page (:240) | (.22,1,.36,1) | ease-out | (.2,.8,.2,1) |
| --ease-spring (:242) | (.34,1.56,.64,1) | ease-spring | identical |
| --scrim (:276) | opaque mix | scrim | rgba(4,5,7,.72) |
| --shadow / -lg (:235-236) | 14px/64px | shadow-pop | 0 12 32 |
| --z-dialog 4000 / --z-popover 9000 (:281-282) | | z-overlay 50 / z-dropdown 20 | Rift's layers are far above Halo's; keep Rift's numbers, map names |
| --font-ui / --font-mono (:257-258) | Inter / JetBrains Mono | --font-sans / --font-mono | Geist / Geist Mono |
| --row-h 26/30/36, --gap, --fs-md 12.5-14 (:289-291) | density modes | none (fixed 44/36) | see Appendix B |

Port surface: the `@theme inline` bridge (`rift:app.css:29-115`) maps Tailwind names to these vars. The tokens `check:tokens` (`rift:package.json:12`, `design-system/check-tokens.mjs`) and `check:ui` will need updating when names change.

## Appendix B. Risks and owner questions surfaced by this digest

1. **Density vs Halo's 44px.** Rift is a dense Windows desktop tool: 26/30/36px rows, 12.5-14px body, user-selectable density (`rift:app.css:289-291`). Halo specifies 44px controls, 15px body. Halo itself sanctions 36px (`Button sm`, SearchField `sm`, Segmented buttons, Accordion 52 is the other way). The "happy medium" is probably Halo look + motion with `sm`/36 metrics as the default and density modes mapped onto it. Owner call.
2. **No-green rule.** Blue preset puts lime in `info`/chart-2; ConsoleX overrode it. Rift has green `ok`/`status-busy`. Decide the replacement(s).
3. **Accent dials.** Rift lets users tune hue/vividness/strength; Halo's model is fixed presets. Keep, drop, or restrict to the 4 Halo accents?
4. **Light themes.** Rift is dark-only; Halo ships light variants (free if wanted).
5. **Syntax colouring.** Halo CodeBlock never colours syntax (single signal colour); Rift highlights with Shiki. Where does Halo's CodeBlock chrome apply?
6. **Lexend/Inter.** Geist replaces Inter; Lexend should stay for dyslexia mode and splash.
7. **Charts/data components** (LineChart, BarChart, Ring, DataTable, Stepper, FileDrop, DatePicker, TagInput) may have no Rift consumer; port only what screens need. Not determined by this digest.
8. **Halo's React hook semantics** (FLIP, toasts, count-up) are now fully known from `js:`, so no further reverse-engineering is needed; but the brand-book folder lacks bundle.js, so the implementers must read the ConsoleX copy.
9. **Possible dead animations in Rift today** (`data-open:` variant, 5.2 #2): verify before reusing `floatingMotion`.
10. **Halo internal inconsistencies to keep out of the port**: AppShell 760 vs bp-md 900; popup timing doc vs CSS; magnetic 6px claim vs +-3px; reduced-motion freezing loops; tinted surfaces vs "no washes".



