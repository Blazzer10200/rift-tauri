---
title: Halo map 07 — AI Health, Diagnostics, embedded browser, dialogs/overlays
tags: [rift, halo, redesign, map]
status: map-only (read-only wave; no source edited, nothing run)
updated: 2026-10-19
---

Scope: `AiHealthPage`, `DiagnosticsPage/Console`, `WebBrowserPage`, `CommandPalette`, `UpdateDialog`, `McpServersDialog`, `severity-tone.css`, `tooltip.ts`, `portal.ts` (+ the two shared parts AiHealth/Diagnostics lean on: `PageHero`, `LimitBar`). All paths relative to `src/lib/` unless noted. Rift root = `C:/AI Workflow/projects/rift-tauri`. Halo = brand-book dir `.../705c5425.../project/` (README.md, 01-08 md, components/*/README.md, components/bundle.css, components/index.d.ts) + `consolex/panel/design-systems/halo/tokens.css`.

# 1 Files & roles

| File | Lines | Role |
|---|---|---|
| `components/ai-health/AiHealthPage.svelte` | 1520 | Workspace page `ai-health` (lazy-loaded, `shell/workspaceRegistry.ts:36`, kbd 6). Script 1-717 (all derivations/verdict logic/advisor), markup 719-1228, scoped CSS 1230-1520. Two forks: ChatGPT branch 720-822, Claude branch 823-1227 (switch = `chatGptActive`, :27). |
| `components/diagnostics/DiagnosticsPage.svelte` | 29 | Page shell: `PageHero` + `<DiagnosticsConsole page />` (:13-21). `diagnostics.init()` on mount (:7-9). Workspace `diagnostics`, kbd 5 (`workspaceRegistry.ts:37`). |
| `components/diagnostics/DiagnosticsConsole.svelte` | 512 | Console panel. Script 1-158 (virtual window, search debounce, export/copy), markup 160-321, CSS 323-512. Has a MODAL mode (`page=false`: portal + fade + Esc, :314-321, `onKey` :93) but **no importer uses it** — only `DiagnosticsPage` imports it, always `page` (grep, 2026-10-19). Modal branch = dead code. |
| `components/webview/WebBrowserPage.svelte` | 979 | Browser dock chrome (address bar, find bar, overflow menu, empty state). The page itself is a NATIVE child WebView2 positioned over `.wb-stage` by Rust (`src-tauri/src/browser/mod.rs:1-12`). Script 1-488, markup 490-674, CSS 676-979. Mounted only by `assistant/AssistantPage.svelte:324` inside `.dock-wrap` (`AssistantPage.svelte:301-330`). |
| `components/dialogs/CommandPalette.svelte` | 529 | Global Ctrl+P / Ctrl+K palette (`AppShell.svelte:300-301` binds, `:334` mounts). Items derived :46-160, fuzzy score :166-186, grouped :197-208, keyboard :241-288, markup 309-376, CSS 378-529. |
| `components/dialogs/UpdateDialog.svelte` | 562 | State-machine updater dialog (checking/available/downloading/installing/uptodate/error/idle), hero + release notes + footer. Markup 94-277, CSS 279-562. Mounted `AppShell.svelte:329`. |
| `components/dialogs/McpServersDialog.svelte` | 541 | MCP list + inline add form + inline remove-confirm. Markup 131-268, CSS 270-541. Mounted `AppShell.svelte:336`. |
| `styles/severity-tone.css` | 14 | Maps `data-severity` ok/warn/danger/info -> `--tone` on `.toast`/`.row`. Imported ONLY by `ToastHost.svelte:12` and `shell/NotificationCenter.svelte:14` — **none of this wave's files use it** (it is out of this surface in practice; belongs to the toast/notification map). |
| `actions/tooltip.ts` | 41 | **No-op shim** (see below). |
| `actions/portal.ts` | 14 | Moves a node to `document.body`; used by `DiagnosticsConsole:317`, `WebBrowserPage:599-600`. |
| `components/shared/PageHero.svelte` | 48 | Eyebrow + 24px title + sub; used by AiHealth, Diagnostics, Settings only. |
| `components/shared/LimitBar.svelte` | 82 | 3 variants (panel/card/compact) of one fill bar; AiHealth uses `card`. |
| `components/assistant/stream/AnimatedCount.svelte` | 113 | rAF ease-out count tween, JS-reads `prefersReducedMotion()` (`state/accessibility.svelte.ts:103`). Used by AiHealth :799-800. |

**Tooltips never paint — CONFIRMED.** `tooltip.ts:1-8` header: pulled 2026-06-15; `apply()` (:25-35) strips `title`, and only sets `aria-label` if the node has no aria-label/labelledby/text. So on any element with visible text (every AiHealth tile) it does literally nothing, not even a11y. Header comment says "~26 call sites"; actual `use:tooltip` lines app-wide = **174 across 37 .svelte files** (grep). In this wave: AiHealth 8, DiagnosticsConsole 10. `AiHealthPage.svelte:796-798` comment promises "exact value stays reachable via the tooltip" and `:942` says "Tooltips are banned app-wide" — the exact-value claim is false today (values at :799,800,992,1149-1151,1213-1214 are unreachable).

**Native `title=` DOES paint in WebView2** (OS tooltips): `WebBrowserPage` 16 `title=` (e.g. :492-498,512,530,545,558,565,569,592-594,607-609), `McpServersDialog` 5 (:154,230,232,233,234), `AiHealthPage` 0 real (the 2 hits are `PageHero title=` props :723,826). So Rift has an inconsistent policy: custom tooltip banned, native tooltip used in the browser + MCP dialog.

Out-of-file facts needed for the map: `app.css` owns `.dialog-overlay` (:612-619, z 200), `.btn` (:394-421), `.rift-menu*` (:695-736), `@keyframes enter` (:634), `pop-in` (:640), z tokens `--z-dialog:4000`, `--z-popover:9000` (:281-282), scrim `--scrim` (:276). `design-system/check-ui.mjs` + `ui-baseline.json` ratchet (see 9).

# 2 Layout & anatomy

## AI Health (Claude branch), `AiHealthPage.svelte`
- Root `.sb-main` flex col (:1231) -> `PageHero` (fixed top, 30px/40px pad, max 820px column, `PageHero.svelte:33-35`) -> `.ah-scroll` (:1232, the only scroller) -> `.ah-wrap` grid.
- `.ah-wrap` (:1236-1248): max 820px, pad `18px 40px 28px`, gap 14px, 1 col; **>=1160px viewport: max 1180px, 2 cols** (`.half` cards pair, rest span via `> :global(*) {grid-column:1/-1}` :1244). `align-items:start` (no equal heights, :1242). NOTE hero max stays 820px while body goes 1180px -> header/body left edges diverge at wide windows (audit flagged this in `docs/design/redesign-plan.md:83-85`; still true: `PageHero` default `maxWidth=820`, AiHealth passes none).
- Order of cards (DOM = visual): 1 Advisor strip (:845-871) · 2 Verdict strip `role=status` (:873-890, conditional) · 3 advice error (:892-894) · 4 Advice list (:896-937, conditional) · 5 Speed & efficiency card (:946-1048, conditional on `perfStats`) · 6 MCP servers `.half` (:1053-1073) · 7 Plan limits `.half` (:1076-1127) · 8 Where your usage goes `.half` (:1130-1182) · 9 Spend per day `.half` (:1187-1205, conditional) · 10 This session `.half` (:1208-1223, conditional). Because cards 2/4/5/9/10 are conditional, which cards pair in the 2-col grid changes with data (nth-child entrance stagger also shifts, :1253-1262).
- Card chrome (`.ah-card` :1431): padding 15px 20px, `--radius-xl` (12px), `--surface` + inset 1px `--border`. Heading `.ah-card-h` 13px/660 + 15px lucide icon tinted accent (:1432-1433).
- Tiles (`.ah-tiles` :1471): `repeat(auto-fill,minmax(110px,1fr))` gap 12; `.ah-tile` pad 12/14, 20px/720 value, 11.5px label (:1472-1480). Tint variants ok/warn/hot = colour wash + inset ring (:1491-1493) + inline `.ah-verdict` micro-pill 9.5px (:1494-1497).
- Bars: `LimitBar variant=card` 7px (`LimitBar.svelte:44-46`) for plan windows (:774,781,1097,1122); hand-rolled `.ah-track/.ah-fill` 5px for model share/spend rows (:1161,1175; CSS :1462-1469).
- Spend chart (:1190-1203, CSS :1379-1410): flex columns, 96px tall, bars max 26px wide, `%`-height, peak+newest labelled, day-letter under (9.5px).
- Latency sparkline (:1017-1024, CSS :1512-1519): inline `<polyline>` in 100x28 viewBox, `preserveAspectRatio="none"` + `vector-effect:non-scaling-stroke`.
- Advisor strip (`.ah-advisor` :1272): flex, accent-gradient wash (13%->4%) + 22% accent inset ring, 36px icon chip, text, CTA button (:1314).
- Verdict strip (`.ah-verdict-strip` :1282): 3px left accent/warn/danger rule, dot, label, note (2-line clamp), right-aligned value pills.

ChatGPT branch (:720-822) reuses `.ah-card`, `.ah-tiles`, `LimitBar`; no verdict strip, no perf; chip in hero (:726-730).

## Diagnostics
- `DiagnosticsPage` `.sb-main` (flex, transparent, :27) -> `PageHero maxWidth=1080` -> `.dg-body` max 1080px pad `0 40px 30px` (:28) -> `.dc-panel.page` fills 100% (`:344` no border/radius/shadow).
- Panel (`.dc-panel` :338): flex col, `border-radius 16px`, `overflow hidden`. Rows top->bottom: header (:164-198, 30px mark + title + live verdict + 4 icon buttons; accent gradient wash :351-352 and a 1px accent gradient hairline on top via `::before` :346-347) · optional action-error strip (:200-202) · **vitals grid** (:204-217, `auto-fill minmax(155px,1fr)` buttons = per-subsystem health dots, click filters) · filter row (:219-247: search, 5-segment min-level, source select/clear chip) · log body (:249-289) · "Jump to live" floating pill (:291-293, abs bottom 54px) · footer status bar (:295-310).
- Log body: **hand-windowed virtual list**: fixed `ROW_H=26` (:42, must equal `.dc-row min-height:26px` :451), overscan 8, spacer divs (:255,287); disables windowing while any row is expanded (:59). Row = time · level pill (46px) · resource tag · message ellipsis · chevron · file. Expanded `<pre>` of JSON fields (:278-284).
- Modal mode geometry (dead): `.dc-backdrop` z-index **70**, panel `min(1080px,93vw) x min(740px,88vh)` (:334-342).

## Browser dock
- `.wb-root` flex col, `container-type:inline-size` (:679-684). Rows: toolbar `.wb-bar` (:685-691, 8/10 pad: back/fwd/reload 30px buttons, favicon/globe glyph, owner pill, address input, AI-read pill, console badge, "Add to chat", overflow, close; loading comet `.wb-progress` bottom edge :736-744) · find bar (:576-596, conditional) · `.wb-stage` (:911-916, empty placeholder the native webview covers). Container queries hide owner pill <520px (:719) and button labels <470px (:811-813).
- Stage content when no page open (:624-673): hero chip (56px accent-tinted icon) + h3 + pitch, 3-item capability list, 2 quick buttons, kbd hint strip. Error = red text (:626).
- Overflow menu (:598-621): **portalled** scrim (`z 999`, :844) + menu (`z 1000`, :848) positioned by `getBoundingClientRect` (:24-28), chrome from global `.rift-menu`; contains Find, zoom stepper (50-300%, :606-610), Copy URL, Open external.
- Dock slide (AssistantPage, not this wave but drives bounds): `dockSlide` JS tween of **real width**, 220ms `cubicOut` (`AssistantPage.svelte:11-20`), divider 3px (:340+) -> every frame fires ResizeObserver -> `syncBounds` (rAF-coalesced IPC `browser_set_bounds`, :125-136, :430-433).

## Dialogs / overlays (z-order as coded)
| Layer | z-index | Source |
|---|---|---|
| Diagnostics modal (dead) | 70 | `DiagnosticsConsole.svelte:334` |
| `.dialog-overlay` (UpdateDialog) | 200 | `app.css:612-619` |
| CommandPalette scrim | 200 | `CommandPalette.svelte:381` |
| McpServersDialog scrim | 200 | `McpServersDialog.svelte:273` |
| WebBrowser menu scrim / menu | 999 / 1000 | `WebBrowserPage.svelte:845,849` |
| ToastHost | 2000 | `ToastHost.svelte:98` |
| NotificationCenter | 2100 | `NotificationCenter.svelte:270` |
| `--z-dialog` (ui/ alert-dialog etc.) | 4000 | `app.css:281` |
| `--z-popover` | 9000 | `app.css:282` |
| Splash | 9999 | `SplashOverlay.svelte:130` |
=> The three hand-rolled dialogs sit BELOW toasts and the notification center, contradicting the `app.css:277-281` comment ("Dialogs sit above every app surface incl. toasts"), which is only true for the `ui/` primitives. Halo's scale is `z-raised 1 · sticky 10 · dropdown 20 · overlay 50 · toast 60 · tooltip 70` (`tokens.css`): overlay < toast < tooltip. Rift has no concept equal to "toast above dialog" being intentional.

Sizes: palette `min(620px, 100vw-32)`, `max-height 68vh`, top-anchored `padding-top:13vh` (:379-388); MCP `min(560px,100vw-32)`, `max-height 64vh`, `margin-bottom 5vh`, centered (:271-282); Update fixed 500px, `max-height calc(100vh-48px)` (:280-285).

# 3 States

## AiHealth
- Empty/pre-data: hero chip "Getting started" when `!healthScore` (:833-837). Plan card: API-key note (:1078-1079) / error (:1080-1081) / "Loading your plan limits…" text (:1082-1083) / no limits (:1084-1085). History card: error (:1132-1133) / **loading = skeleton tiles + text** (:1134-1143) / empty (:1144-1145). MCP card: null "No status yet" / empty / list (:1055-1072). Speed card hidden entirely until `perfStats` (:946); inside, `!hasPerf` (<3 turns) text (:957-960). Speed tile has "learning"/"lifetime" states (:975-981, :973).
- Loading elsewhere: plan limits = a plain text line, no skeleton; spend/This-session just don't render. Perf/stat fetch failures are **swallowed silently** (`catch { /* absent panel is the degraded state */ }` :109; `stats` error shown only for history :143).
- Live/streaming: advisor "Analyzing…" (:849-855): cosmetic step ticker 3.2s (:603-605) floored by real backend stage events `usage-analyze-progress` (:140-143, :609-612); CTA shows spinning `Loader2` (:866). Plan limits re-poll 3 min, "ago" label clock 30 s (:146-147). Hot verdict dot pulses (:1293). `LimitBar` hot sheen loops (`LimitBar.svelte:51-56`).
- Error: advice error banner-like block (:892-894, 1324); ChatGPT account error (:743-744).
- Hover/focus: only `.ah-apply-btn:hover`, `.ah-undo:hover`, `.ah-range-b:hover`, `.ah-refresh:hover` (:1345,1349,1371,1435). **No focus-visible styles authored** for any button on the page (grep `:focus` in file = 0; the global `.btn` has one at `app.css:409` but `.ah-*` buttons don't use `.btn`). Cards have no hover state. Disabled: `.ah-adv-btn:disabled` 0.6 (:1315), `.ah-refresh:disabled` 0.6 (:1436) — not Halo's 0.45.
- Narrow: no @media except >=1160px two-col (:1245-1248); tiles reflow via auto-fill; verdict strip `flex-wrap` on pills only; no narrow breakpoint at all. `localStorage` per-user range key `rift.aihealth.perfWindow.v1` (:100).

## Diagnostics
- Empty: "Waiting for events…"/"No events match" (:250-253). Offline/paused/live footer state (:296-298); vitals idle = disabled, 0.5 opacity (:396). Actions: copy -> check icon 1.6 s (:99-100), export busy disables button (:187), failures flash a red strip 4.5 s (:19-23, 200-202). Dropped events count (:306-308). Scrolled-up -> "Jump to live" (:291). Expanded rows (:259-284). Hover rows (:456), `.open` (:457). Focus: only `.dc-search:focus-within` ring (:421); buttons rely on UA outline.
- Narrow: filters `flex-wrap` (:417), vitals grid auto-fill; no breakpoint.

## Browser
- Not opened = empty hero (:627-672); opened with page = native view covers stage; loading = comet + spinning globe (:502,706,736); error text (:626); disabled controls 0.4 (:730); add-to-chat states idle/adding/added/fail (:552-563, flash 1.5 s :48-53); console badge errors/warn-only/flash ok/fail (:536-549, 779-808); "Page read"/"Console read" shimmering pill auto-fades 2.6 s (:526-535, 86-93); find open/closed; menu open/closed; zoom clamp states (:607-609).
- Narrow dock: container queries (:719, :811). Native webview hidden **only when workspace != chat** (`WebBrowserPage.svelte:371-379` via `browser_hide`; Rust header `browser/mod.rs:8-11` says "floats above the main webview, so the frontend must hide it whenever the dock isn't actually visible").

## Dialogs
- Palette: empty "No matches" (:335-339), active row (:467-472), hover sets active (:353), reserved ↵ slot (:508-509). Reduced motion: **no handling** — `transition:fade/scale` (:311,318) are Svelte JS transitions and ignore `prefers-reduced-motion`; same in `McpServersDialog:133,141`, `UpdateDialog:95,103` (`app.css:622-629` only gates `.dialog-overlay`'s CSS animation, not the Svelte transitions). `UpdateBanner.svelte:159` does check `reducedMotion` for its slide — the dialogs don't.
- Update: 7 states + sub-states (devUnavailable / installBroken / downloadError) (:12-17, 117-183); `busy` (installing) locks Esc/backdrop/close (:23,34,38,111).
- MCP: loading skeleton rows (:211-214), empty (:215-222), all-ok / hint / failed footer (:255-265), add form (:168-208), per-row remove confirm (:239-250); Esc peels layers (:111-121, capture phase :129).

# 4 Motion present

Legend: K keep · T retune · X kill · H = what Halo rule says (`03-motion.md`, `04-performance.md`). Rift tokens (`app.css:243-250`): `--dur-fast 140`, `--dur-base 240`, `--dur-slow 520`, `--dur-page 400`, `--ease-page cubic-bezier(.22,1,.36,1)`, `--pulse-live 1.6s`. Halo: fast 150 / base 300 / slow 600 / stagger 60, `--ease-out cubic-bezier(.2,.8,.2,1)`, `--ease-spring`.

## AiHealth
| path:line | what / property / dur / ease / trigger | verdict |
|---|---|---|
| :1253-1262 | each `.ah-wrap` child `animation: enter 360ms var(--ease-page)`, delays 20/70/120/170/220/270/320 ms (`enter` = 4px rise, `app.css:634`), on mount; nth-child so late/conditional cards shift their slot | T -> Halo `halo-rise` (14px, `dur-slow` 600, ease-out, `animation-delay: i*stagger(60)` capped ~12). 4px is far subtler than Halo's 14px. Drive with `--i`, not nth-child. |
| :1282,1286 | verdict strip `ah-vs-in 0.42s`, -4px translateY fade, on mount | T/merge -> Banner (`halo-rise` `dur-base`) when shown; healthy = a calm Badge, no entrance of its own |
| :1293-1295 | hot dot `ah-vs-pulse 2.4s infinite` box-shadow ring | K intent (real "needs attention" state) but H: loops only for real-time activity -> a hot plan limit is not live data. Use static `danger` Badge; reserve `LiveDot` for live. Also animates `box-shadow` (paint, not transform/opacity). |
| :1316-1317 | `Loader2` spin 0.9 s linear inf | T -> Halo `Spinner` (900ms, `halo-spin`) — same speed, allowed loop |
| :1320-1323 | analyze step text swaps with `{#key stepIdx}`: translateX(-5px) fade 0.4s | T -> `halo-rise`-style crossfade / step text w/ dur-base; keep "real stage floors the ticker" logic |
| :1344,1348,1369 | button bg/color `var(--dur-fast)` (the range one uses `ease-out`) | K (hover = fast) |
| :1394 | spend bar column `transition: height var(--dur-base) var(--ease-page)` — **only on update**, no initial entrance (bars pop in with the card) | T -> Halo BarChart (bars grow on mount; peak = one accent) |
| :1464 | `.ah-fill` `transition: width 0.3s ease` (literal 300 ms + default ease; same literal in `LimitBar.svelte:46`) | T -> Halo `Progress` (eases to value, mono % counts with it); width transition animates layout vs Halo's `scaleX` transform (`halo-progress__bar` transform-origin left, `bundle.css:459`) |
| :1484-1487 | skeleton `ah-skel-pulse 1.4s` opacity 0.5<->1 loop | T -> `Skeleton` shimmer (translateX, 1.4 s linear, `halo-shimmer`) + 360ms blur-out `halo-skel-out` hand-off to content (skeletons "fade, never pop") |
| `LimitBar.svelte:51-56` | hot-fill sheen sweeps `2.6s` infinite, delay 900ms | X/T — a looping decoration; Halo bans loops except live state. Danger-tone `Progress` carries meaning without the sheen. |
| `AnimatedCount.svelte` + :799-800 | rAF tween 720ms ease-out cubic (default 420) | T -> `Stat` count-up (1100 ms default, ease-out); `AnimatedCount` is the sole in-repo count tween and only 2 of ~25 numbers use it |
| (none) | numbers elsewhere are static: tiles :965-994,1148-1152,1212-1219; plan `%` :1095; verdict pills :885 | gap — Halo wants every number to count/roll; large gap here |
| (none) | no LineChart draw-in: sparkline polyline appears instantly (:1021) | gap — Halo `Sparkline`/`LineChart` `halo-draw` |
| (none) | advice cards appear instantly after analyze (:904-933) | gap — Halo: content arrives (skeleton->content, `halo-rise` stagger) |
| (none) | range change (:111-115) re-renders aggregates with no transition | gap — `Segmented` thumb slide (dur-base) + `Stat` counts from old value |

## Diagnostics
| path:line | what | verdict |
|---|---|---|
| :163 | panel `transition:scale {180ms, start .98}` — **also on the page-mode panel** (applies at page mount, `{@render panel()}` :315) | T -> page mode: page-level `halo-page` (520ms, 10px rise); modal mode is dead -> drop |
| :317 | backdrop `fade 140ms` (dead modal) | X with the dead branch |
| :403-407 | every non-idle vitals dot has a `::after` halo `dc-breathe var(--pulse-live,1.6s) ease-in-out infinite` (scale 1->1.5, opacity .28->0), N dots at once (N = `diagnostics.health.length`) | T — H: loop only for real activity. Keep ONE `LiveDot` (the live/Offline state), static status dots elsewhere (each pulsing ring also forces a compositor layer; transform/opacity only so cost is low but visually busy) |
| :497-501 | footer state dot `dc-blink 1.6s` opacity loop when Live | K -> `LiveDot` (ping 1.8 s `halo-ping`, `bundle.css:82`); paused = `warn`, offline = `idle` (no pulse) |
| :375,394,420,425,430,454,490 | hover transitions on `--dur-fast, .14s` (fallbacks literal) | K |
| (none) | log rows: no entrance. New tail rows pop in; expand = instant | acceptable for a log — Halo says "rows ripple in on load, cap ~12"; do **not** stagger a streaming log; keep rows static, animate only the expand (grid-rows/height dur-base) |
| (none) | "Jump to live" pill appears/disappears via `{#if}` with no transition (:291) | T -> halo pop-in (`halo-pop-in` dur-fast) |
| :80-87 | `scrollTop = scrollHeight` on every new event while stuck, via rAF | K (functional; not a CSS motion) — but no `scroll-behavior`, which Halo reduced-motion block forces `auto` anyway |

## Browser
| path:line | what | verdict |
|---|---|---|
| :706-707 | globe glyph spins 0.9 s linear inf while `loading` | K (real activity) -> `Spinner`/halo-spin |
| :736-744 | top-bar comet `wb-sweep 1.1s ease-in-out infinite`, animates `transform` | T -> `Progress` indeterminate (`halo-indet 1.3s ease-out`, `bundle.css:460`); reduced-motion fallback already sane (:749-751) |
| :768-777 | "Page read" pill: `wb-shimmer 1.5s linear infinite` on `background-position` (paint, not transform) + `wb-airead-in var(--dur-fast)`; visible ~2.6 s | T — shimmer loop is decorative-ish but tied to a real event; replace with a one-shot Badge `halo-rise` + `LiveDot` maybe; at minimum don't animate `background-position` (H: only opacity/transform continuous) |
| :789,826,961 | hover transitions fast | K |
| menu | portalled menu appears with **no entrance animation** (static `{#if}` :598); `.rift-menu` has none either | T -> Halo `Menu`: `halo-pop-in` (dur-fast scale .96 + 4px) + `halo-menu-item` stagger (220ms, 18ms steps, cap 12) |
| `AssistantPage.svelte:11-20` | dock slide JS width tween 220ms cubicOut + opacity | T -> dur-base(300)/`ease-out`; but see risk: real-width tween = layout per frame + IPC per frame. Prefer an approach where bounds sync once at end + webview hidden during slide (unknown if acceptable — owner/impl question) |
| :623-673 | empty-state hero has no entrance | T -> `EmptyState` + `halo-rise` |

## Dialogs
| path:line | what | verdict |
|---|---|---|
| `CommandPalette:311` | scrim `fade 130ms` | T -> `halo-fade` dur-fast(150), ease-out |
| `CommandPalette:318` | panel `scale {start .97, 170ms, quintOut}` | T -> `halo-zoom` dur-base: from `scale(.94) translateY(8px)`; **leave**: `scale .96`, 200ms (`bundle.css:286`) — Rift's leave is the same reversed transition, Halo's leave is explicit and quicker |
| `CommandPalette:465,481` | row bg `80ms`, icon `dur-fast` | T -> dur-fast + ease-out (80ms is below Halo's smallest) |
| `McpServersDialog:133,141` | fade 130 / scale .97 170 quintOut | same as palette |
| `McpServersDialog:326` | refresh icon spin 900ms | K -> Spinner |
| `McpServersDialog:368-373` | remove-trash `opacity` reveal on row hover/focus-within `var(--dur-fast)` | K (nice, opacity only) |
| `McpServersDialog:514-516` | skeleton rows shimmer 1.1 s `ease-in-out` inf on `background-position`, stagger `120ms` delay | T -> `Skeleton variant=list` (translateX shimmer 1.4 s linear) with real->content crossfade |
| (none) | add form / remove-confirm appear with `{#if}`, no height/opacity animation (:168,239) | gap -> `Accordion`-like height, `halo-rise` |
| `UpdateDialog:95,103` | overlay fade 120ms; shell `fly {y:8, 180ms}` (no scale) + `app.css:612-619` `.dialog-overlay` also runs `overlay-in var(--dur-fast)` -> **double fade** (CSS anim + Svelte transition on the same node) | T -> `halo-zoom` (scale+8px) same as palette; remove double fade |
| `UpdateDialog:430` | download fill `transition: width var(--dur-fast)` (140 ms; progress arrives in discrete ticks so the bar steps) | T -> `Progress` (eases, mono % counts) — 40px hero % jumps without count tween |
| `UpdateDialog:556-561` | `:global(.spin)` re-defined `1s linear` (clobbers app.css `.spin 0.8s`) | T/de-dupe -> Spinner |
| `UpdateDialog:328,531` | close/link hover `--dur-fast ease` | K |
| (none) | state change in hero (checking->available->downloading) swaps content instantly | gap — crossfade/`halo-rise` per state; Halo "never pops" |

Reduced motion coverage: CSS-driven loops/animations here are gated (`@media (prefers-reduced-motion: reduce)` at AiHealth :1287,1295,1323,1488, Console :407,501, Browser :708,749,777, Update :558, `LimitBar:75-82`), the Svelte JS transitions and `AnimatedCount`'s tween (reads store) are the exceptions: palette/MCP/Update/Diagnostics transitions ignore it. Halo's blanket rule (`bundle.css:901-903`, durations -> 0.01ms) would also handle CSS but **not** Svelte `transition:` (JS-timed); a Halo port must make dialog open/close use CSS (`halo-zoom`/`halo-fade` + `.is-leaving`) or read `prefersReducedMotion()`.

# 5 Styling wiring

- **No Tailwind classes at all** in these 9 files (grep: scoped `<style>` + a few global classes: `.btn` (UpdateDialog footer, `app.css:394`), `.rift-menu*` (Browser menu), `.dialog-overlay`, `.spin`, `.enter` keyframe). `design-system/check-ui.mjs` Tailwind-arbitrary count = 0 here. None of these use the `components/ui/*` bits-ui primitives (alert-dialog, button, dropdown-menu, select, context-menu, separator exist — `components/ui/`); Rift's own ratchet says new UI should compose from them (`check-ui.mjs:2-8`), these three dialogs + menu are the hand-rolled debt.
- **Tokens used** (Rift naming): `--bg --bg-inset --bg-elev-1/2/3 --surface --surface-hover/-active --field --field-border --border --border-strong --border-focus --fg --fg-2 --fg-muted --fg-subtle --fg-faint --accent --accent-soft --accent-fg --ghost-border --ring --ok --warn --danger --info (+ -soft) --radius-xs/sm/(none)/lg/xl/2xl --shadow-lg --shadow-float --fs-xs(11)/sm(12)/md(13) --dur-fast/base/slow --ease-page/soft --pulse-live --scrim --font-mono`. Halo renaming needed everywhere (see 7).
- Hard-coded counts (this wave; `px` = occurrences of `N px` anywhere in the file, incl. markup):

| File | px | literal font-size decls | distinct font sizes | literal radii | colour literals |
|---|---|---|---|---|---|
| AiHealthPage | 216 | 27 literal vs 21 `var(--fs-sm)` | **12** (9.5/10/10.5/11/11.5/12/13/14/16/18/20 + fs-sm) | 999 x6, 10 x4, 8 x2, 6 x2, 7, 5, 4/2 + `--radius-xl` x4 | 0 hex/rgb/oklch; but ~20 `color-mix(in srgb/oklab, var(--accent|warn|danger) N%…)` tints |
| DiagnosticsConsole | 114 | 7 literal (9/10/10.5/11/11.5) + 12 tokens | 8 | 4,9,999,0 + `--radius`/`-lg`/`-2xl` | **8 oklch literals** (`--dc-ok/warn/bad/info` :327-330, backdrop :335, shadow :341,489, `.dc-stat.drop` :510 `oklch(.72 .12 50)` an off-palette orange) |
| WebBrowserPage | 110 | 15 (10/11/12/12.5/13/15) | 6 | 8 x4, 7 x4, 999 x3, 4, 3, 16 | 0 |
| CommandPalette | 49 | 3 literal (10/14) + 5 tokens | 5 | 7,4,10 + `--radius-2xl` | **3 rgba()** (scrim :385, shadows :397-398) |
| UpdateDialog | 89 | 5 literal (11/15/20/26/32/40) + tokens | 8 | 13 + `--radius-lg/sm/xs` + 999 x2 | **3 rgba()** (:288-290) |
| McpServersDialog | 93 | 2 literal (9/10) + 14 tokens | 4 | 10 x3, 8 x2, 7 x2, 5 x2, 6, 4, 99, 50% + `--radius-2xl` | **3 rgba()** (:275,287-288) |
| PageHero | 16 | 24/10 | 3 | 999 | 0 |
| LimitBar | 10 | — | — | 999 | 2 oklch (:34) |
(counts via grep of the files 2026-10-19; "literal radii" = `border-radius:` values, not per-corner.) Repo baseline for ratchets: hex 21, rgb/hsl 16, oklch-literal 131, use:portal 11, aria-role 17, arbitrary 2 (`design-system/ui-baseline.json`). This wave contributes: 9 rgba + 11 oklch + 2 portal + 4 role=dialog/menu (Console modal branch :163 `role=dialog`; Browser :600 `role=menu`; Palette/Update/Mcp `role=dialog` each).
- **Gradients / glows (Halo principle 1 "No colour washes… one gradient allowed: Card spotlight"; `README.md:9`)**: AiHealth advisor wash :1272, spend bar :1398, verdict-strip tints/tile tints :1491-1493, `.ah-advice-err` 8% wash :1324; Diagnostics header :351 + top hairline :346-347 + dot glows :365,367,369 + `.dc-statedot` glow :498; Browser AI-read shimmer :762-767; Palette outer accent glow `0 0 80px -32px accent` :399 + inset sheen :400; MCP same :289-290, dot glows :456,461; Update hero radial wash :349-356, text-shadow glows :381,413, `.btn.glow` accent halo :550-554, `box-shadow 0 0 22px` icon :366. All are Halo violations (Halo allows `glow-strong` only under a hovered primary button, `01-themes-and-colour.md`).
- Layering: three different dialog chromes: `.cp-panel`/`.mcp-panel` (`--bg-elev-2`, border-strong, `--radius-2xl`, 4-layer shadow) vs `.upd-shell` (`--bg-elev-1`, `--radius-lg` 12, 3-layer shadow) vs `.dc-panel` (`--bg-elev-1`, `--radius-2xl`, `--shadow-lg`). Scrims: `--scrim` (UpdateDialog via `.dialog-overlay`), `color-mix(--bg 40%, rgba(0,0,0,.6))` (Palette, MCP), `oklch(0 0 0/.6)` (Console). Three recipes for one thing.
- Page chrome: AiHealth/Diagnostics use `PageHero` + a body column; widths 820/1180 (AiHealth) vs 1080 (Diagnostics). Both use `transparent` backgrounds so the app dot-field shows (Diagnostics :25-27 comment) — Halo `halo-root` paints a flat `bg-000` ground; dot-field decision belongs to the shell map.
- Fonts: Rift body font = app default (Inter per `app.css` comments, not read this wave — unknown exact); mono = `--font-mono`; Halo = Geist/Geist Mono from Google Fonts (`02-type-space-and-layout.md`, loaded by `bundle.css`). A desktop app loading fonts from Google at runtime = CSP/offline question (`app.css`/CSP unknown — see 10).

# 6 Restyle boundary

## AiHealth — KEEP (logic)
- All derivations 25-715 unchanged: `usage`/`assistant` store reads, `invoke("query_turn_perf")` (:108), `invoke("assistant_stats")` (:135), `listen("usage-analyze-progress")` (:140), 3-min/30-s timers (:146-147), `healthScore` verdict composition (:436-499 — contains product logic incl. upstream-latency demotion), `latencyP90Source/Signal/Attribution/Learning`, `splitAttribution`, `cacheSignal`, `paceFor` (:223), `spendBars` (:239), `modelSpend/modelLat` (:275-287), `buildSnapshot` (:504-581 — JSON the advisor reads; **do not touch fields**), `analyze`, `applyAction/undoAction` + `applied` bookkeeping (:685-716, comment :679-684 explains why `applied` must survive re-analyze), `PW_KEY` localStorage, `ANALYZE_STEPS`/`STAGE_FLOOR`.
- Data shapes: `TurnPerfStats`, `ModelPerfStats` (:51-88).
## AiHealth — CHANGE (markup/CSS)
- All of 719-1228 markup (cards, tiles, strips, charts) and the whole `<style>` 1230-1520 (delete wholesale; dead rules already present: `.ah-trend*` :1506-1510 and `.ah-chip.ok/warn/hot` :1268-1270 have no markup users).
- `zone()` (:204) / `mcpTint` (:370) / `latencyTint` etc. return `"ok"|"warn"|"hot"` strings; Halo tones are `accent|warn|danger|info|neutral` -> one small mapper (`ok->accent`, `hot->danger`) at the boundary, not scattered renames. (`LimitBar` `zone` prop likewise.)
- ok != "good" semantic risk: Halo says status positive = `accent`. Fine in blue theme.
- `PageHero` (shared with Settings) — restyle once centrally (map 0x owner of shell/settings), not per page.
## Diagnostics — KEEP
- Store `state/diagnostics.svelte.ts` (+ `diagnosticsEvents.ts`, `diagnosticsHealth.ts`, tests `diagnosticsEvents.test.ts`, `diagnosticsHealth.test.ts`) untouched; `diagnostics.init/togglePause/clear/exportText/filtered/health/overall/live/paused/dropped/resources/search/minLevel/resourceFilter` API.
- Virtual window math (:40-91) + `ROW_H` coupling to CSS row height; search debounce 140 ms (:114-122); `diag_export_bundle` + `openPath` (:26-38); expand/`stick` behaviour (:59, 71, 74-87); `window.__riftDiag` hook (read by `scripts/cdp/c.sh:447-454`, installed by the store).
## Diagnostics — CHANGE
- All markup/CSS. Delete the modal branch (`page` prop, portal, backdrop, `onKey`, `onclose`) unless an owner wants it. **Replace the `--dc-*` oklch palette (:327-330)** with Halo status tokens.
## Browser — KEEP
- Every handler/IPC: `browser_open/_reload/_back/_forward/_set_bounds/_set_zoom/_find/_clear_find/_read_page/_read_console/_console_counts/_current_url/_show/_hide/_close`; listeners `browser://load`, `browser://icon` (:444-465); ownership logic (`browserOwnerTab`, `claimOwner`, `go()` :138-192); sentinel-neutralizing in `addToChat`/`addConsoleToChat` (:289-298, 337-343 — security-relevant prompt-injection hardening, keep verbatim); `syncBounds` rAF coalescing (:125-136) and the "push a fresh sync after open" (:182); 1.2 s URL poll (:436-441); `normalizeUrl` omnibox + http(s) allowlist note (:95-105); token effects for focus/find (:395-414); `onDestroy` cleanup incl. `browser_close` (:471-487).
- **The stage's geometry contract**: `.wb-stage` rect is the source of truth for the native webview; any restyle must keep a single un-transformed, non-clipped stage element and keep `container-type` semantics. Do not wrap the stage in anything with `transform`/`filter` (would break `getBoundingClientRect` assumptions less, but Halo `halo-page`/`halo-rise` transforms on an ancestor would animate the rect and make the webview chase it).
## Browser — CHANGE
- Toolbar/find/menu/empty-state markup + CSS. `use:portal` menu can become a Halo `Menu` only if it solves clipping and native-webview overlap (see 9).
## Dialogs — KEEP
- Palette: `commandPalette` store (`state/command-palette.svelte.ts`), item builder :46-160 (workspace order -> Ctrl+N, settings deep links, recent chats, split/new-window), scoring :166-186 (subsequence over label only — deliberate, comment :163-165), keyboard model :241-288, focus on open :215-225, refresh conversations :228-233.
- Update: `updates` store (`state/updates.svelte.ts`) state machine, `busy` lock, `notesLines()` (:61-70, plain text render by design), `ago()`, `mbProgress`.
- MCP: `mcpPanel` store, add/remove flows :18-87 (note args split on whitespace, no quoting :56-57), Esc layer-peeling capture-phase handler :111-129 (necessary because composer eats Esc), `statusMeta/mcpHint`.
## Dialogs — CHANGE
- Shells (scrim, panel, header, footer), focus trap (Palette :248-265 and Update :39-55 hand-roll Tab trapping; Halo Dialog/CommandPalette provide trap + focus return — drop the hand-rolled ones only if the Svelte port really does), all CSS.

# 7 Halo mapping

(`H:` = Halo path under `components/<Name>/README.md`, `index.d.ts`.)

| Rift element | Halo target | Notes / gaps |
|---|---|---|
| Page title block (`PageHero`) | `SectionLabel` (`01 / OVERVIEW`) + `display`/`title` type style; page entrance `halo-page` (520ms) | Halo has no "hero" component; eyebrow ("Experimental"/"Debug") -> `SectionLabel`. "Experimental" + Title Case copy flagged in prior audit. Page-level `halo-page` replaces the per-card nth-child `enter` cascade only if cards rise on top of it — Halo does both (`halo-page` on content area, `halo-rise` per card). |
| `.ah-card` | `Card` (bg-100, line-100 hairline -> line-200 hover, radius-lg 16, pad space-5 24) | Rift 12px radius/15x20 pad -> Halo 16px/24px: cards get ~30% bigger -> fewer cards per screen; Halo rule "fit one screen, list scrolls inside its card". Consider `spotlight` on ONE hero card (Speed). |
| Card heading + icon (`.ah-card-h`) | `Card` label -> big value -> detail anatomy; heading style `heading` | Halo anatomy has no leading coloured icon; the accent icons on every card header violate "one signal colour" density (prior audit: accent ~8x/page). Drop or use ink-muted. |
| Advisor strip (`.ah-advisor`) | `Card` + primary `Button` ("Analyze my usage") + `Spinner` in button while analyzing; steps text with crossfade | Halo has no gradient strip -> flat Card; the accent wash dies. |
| Analyze progress steps | `Progress` indeterminate (long job, `halo-indet`) or `Stepper` (5 labelled steps, track fills) | `Stepper` fits the 5 ANALYZE_STEPS (floored by real stage events); gap: Halo Stepper is for flows, steps here are cosmetic-then-real. Owner pick. |
| Verdict strip (healthy) | `Badge tone=accent dot` + `LiveDot`? no — static | A permanent "Healthy" is not a Banner ("a problem on this page that stays until fixed", `Banner/README.md`). |
| Verdict strip (warn/hot) | `Banner tone=warn|danger` (title = label, description = note, no dismiss) | `danger` announced as alert. Dims pills -> `Badge` in Banner children/action. "API is slow right now" = `Banner tone=info`: **in `blue`, `info` = lime `#c6f35a`** (`tokens.css` `[data-theme="blue"]` swaps info<->accent) — a lime banner on a blue app reads as "positive/accent-ish"; consider `warn`-soft or neutral for upstream-latency. |
| Value pills (Latency 4.2s, Cache 100%, Plan 16%) | `Badge` (tone by zone) | OK. Needs number formatting preserved. |
| Metric tiles (`.ah-tile`) | `Stat` in a `Card` grid | **Gaps:** `Stat.value: number` with `format: 'number'|'currency'|'percent'`, `decimals` only (`index.d.ts:50-63`) — Rift tiles show durations (`7.2s`, `850ms`, `—`), compact (`11M`, `$8.16k`) and text (plan name `Max`, top model `Opus`). Need a Rift `format(fn)` or wrapper around `RollingNumber`/`useCountUp` (Halo exposes `Halo.useCountUp`, `README.md` "Using it" 5). Text values (`.ah-tile-sm`) are not Stats. Verdict word (`snappy/laggy/learning/lifetime/efficient`) = `Badge` beside the label (Stat has only a numeric `delta` chip). `peek` rows = the exact numbers the dead tooltips were meant to show (`peek` is "show on hover or keyboard focus", Stat becomes focusable) — **fixes the tooltip gap without reviving Tooltip.** |
| Tile tints (ok/warn/hot washes) | `Badge` tone + coloured value only | Halo: "No colour washes". |
| Plan-limit bars (`LimitBar card`) | `Progress` (value 0-1, `label`, mono %, tone accent/warn/danger, eases) | Rift zones (<60 ok, <85 warn, else hot, :204) -> tones; reset text (`.ah-bar-reset`) + pace line (`.ah-pace`) have no Halo slot (put in `format` or beside label). Pace projection could be a second faint marker: gap (Halo Progress has no target tick). The "peak limit" headline could be one `Ring` (one per card, "of limit"). |
| Model share / spend rows | `Progress size=sm` list (4px bars, mono value) | Per-row fixed 130px label col. Spend rows value is `$`, not %: use `format`. |
| Spend per day (`.ah-spendchart`) | `BarChart` (README: "handful of categories"; 14 daily bars borderline) | Not in task list but the real match; one accent = the peak bar, others `ink-faint`/muted; hover tooltip. Peak+newest value labels -> hover tooltips; verify BarChart prop set (`index.d.ts` BarChartProps — not read this wave). |
| Latency sparkline (`.ah-spark`) | `Sparkline` (96x36, 2px, draws on mount, `tone` chart-1/chart-4) | Rift tints rising=hot, falling=ok (:189) -> `chart-4` warn for bad. Could upgrade to `LineChart` w/ day labels, gliding tooltip, `Segmented` range (reuse the range state). |
| Range picker (`.ah-range`) | `Segmented` (sliding accent thumb, dur-base) | 4 options (24h/7d/30d/All) fits (2-5 short options). Rift `aria-pressed` already. |
| Advice cards (`.ah-rec`) | `Card` + `Badge` (impact: high=accent, medium=warn, low=neutral) + ghost/secondary `Button` | from->to strikethrough delta has no Halo part (compose in mono ink-faint -> ink). Applied/Undo = `Badge accent` + ghost `Button`; could use `Toast` w/ Undo action instead (Halo Toast supports `action`). |
| Plan/API-key/error/loading text lines | `EmptyState` (empty), `Skeleton variant=stat/text` (loading), `Banner` (persistent error) | Today loading = plain text; Halo prefers Skeleton in cards. |
| MCP servers card rows | compact list: `LiveDot`-style status dot + `Badge`; or `DataTable` (name, target, status badge map) | `DataTable` `badge` prop maps value->tone (`index.d.ts:284`) — fits `status: {connected:'accent', 'needs-auth':'warn', failed:'danger'}`; sorting adds nothing here. Needs-auth note stays prose. |
| Glossary footnote (`.ah-glossary`) | prose under hairline or `Tooltip` per term | Owner decision: Halo `Tooltip` exists, Rift banned tooltips 2026-06-15. |
| Skeleton tiles | `Skeleton variant="stat"` | direct. |
| Hero chip ("Getting started") | `Badge neutral` | |
| Diagnostics header title/verdict | `Card`/page header + `Badge`/`LiveDot` | verdict "All systems healthy" -> `Badge accent dot`. |
| Diagnostics action buttons | icon `Button` (ghost) + `Tooltip` (names) | icon-only buttons currently have **no** accessible name unless `use:tooltip` promotes it: `tooltip.ts:30-34` sets `aria-label` only when no text — these SVG-only buttons do get it. Keep that a11y behaviour when swapping to Halo `Tooltip`/`ariaLabel`. |
| Vitals grid (`.dc-vital`) | `Card`-lite toggle chips: `Stat`-less; `Badge` dot per subsystem inside a `FocusGroup`? | No exact match: clickable filter tiles with label+detail+status dot. Rift-specific part "VitalChip" composed from Halo tokens (`bg-100`, `line-100`, active = accent hairline). Use `Segmented`? no (13+ options). |
| Search (`.dc-search`) | `SearchField size=sm` (36px, clear pops in, optional `shortcut`) | Halo clear + Esc clears; `loading` spinner unused. |
| Min-level 5-segment | `Segmented` (5 options at limit) | `trace/debug/info/warn/error` — capitalize per Halo? mono caption. |
| Source select | `Select` (Halo list) | Halo v2.3 `Select` renders a Halo list; WebView2 native `<select>` currently (:242) — `shared/Select.svelte` also exists in Rift (not read). |
| Level pill (`.dc-lvl`) | `Badge` tone: trace/debug neutral, info neutral or `info`, warn `warn`, error `danger` | **blue-theme caveat:** `info` = lime. Using `info` for log level `info` would make info lines lime on a blue app. Use `neutral` for info. |
| Log stream rows | **Rift-specific `LogStream`** on Halo tokens; borrow `CodeBlock kind=terminal` look (`bg-000`, mono `data` size, `ink-muted` output) | **Gap:** `DataTable` has no virtualization and staggers rows (`animation-delay min(i,12)*25ms`, `bundle.css:247`); 2000 rows at 26px would be a perf hit. Keep the windowed list. `CodeBlock` is a static block with copy button (no growing log) — README itself says put a growing log in a `SplitPane` pane / `maxHeight`. |
| Expanded fields `<pre>` | `CodeBlock` (copy button, `wrap`, `maxHeight`) | direct fit for JSON fields + UpdateDialog/MCP technical details `<pre>` (:174,195, :450-463). `CodeBlock` never colours syntax = matches Rift. Keep per-row expand animation as a fast height ease. |
| Footer (Live/Paused, counts, dropped) | status bar row with `LiveDot` + mono counts | `LiveDot` tones accent/info/warn/idle map to live/paused/offline. |
| Jump-to-live pill | small primary/secondary `Button` + `halo-pop-in` | |
| Action-error strip | `Banner tone=danger` (dismiss) or `Toast` | Banner semantics ("stays until fixed") vs a 4.5 s flash -> `Toast` (duration 0 for errors per Toast README) fits better. |
| Panel split (vitals/stream) | `SplitPane` (vertical) | Optional: user-resizable vitals vs log; not required. Rift has none today. |
| Browser toolbar buttons | `Button` ghost icon (44px touch target rule vs Rift 30px) + `Tooltip`/`ariaLabel` | **Halo "Touch targets >=44px"** vs 30px desktop controls: conflict for a dense toolbar; `SearchField size=sm` is 36px. Owner call: Halo desktop density? (see 10). |
| Address bar | `TextField`/`SearchField` (52px default, `sm` 36px) | Halo TextField is a floating-label 52px field — unfit for an omnibox; use `SearchField sm`-style with Rift omnibox behaviour. **No Halo equivalent of browser chrome** -> Rift-specific `BrowserBar` composed from Button/SearchField/Menu/Badge/Spinner. |
| "Page read" pill | `Badge accent` + `LiveDot`/one-shot rise | |
| Console badge | `Badge tone=danger|warn` (count) as a button | |
| Overflow menu | `Menu` (`items`: find w/ `hint`, copy, open external; zoom stepper = custom row) | **Gaps:** (1) zoom stepper row isn't a Halo menu item; (2) Halo `Menu` list is `position:absolute` relative to trigger (`.halo-pop` `bundle.css:406`), would be clipped by `.dock-wrap{overflow:hidden}` (`AssistantPage.svelte:361-367`) and `.browser-dock{overflow:hidden}`; Rift's `use:portal` exists exactly to dodge this; (3) native webview covers it (see 9). `Popover` for zoom/find. |
| Find bar | `SearchField` + icon buttons (or `Popover`) | |
| Empty state hero | `EmptyState` (ringed accent icon, title, 1-line why, ONE action) | Rift shows pitch + 3 capability bullets + 2 buttons + kbd hints: Halo wants one action; capabilities -> docs/tooltip; `Kbd` for hints. |
| Dock divider (3px) | `SplitPane` (13px grab area, grip, double-click reset, keyboard arrows, storageKey) | Would replace `AssistantPage` divider (other map). Native webview syncing: Halo says "content never shifts while you drag" (ok) but its drag emits size changes -> `syncBounds` coalescing still needed. |
| CommandPalette | `CommandPalette` | **Gaps vs `index.d.ts:320-328`:** items have `group` shown as a RIGHT-aligned faint label, not section headings (Rift groups: Go to / Recent chats / Actions, with sticky-style headings :342); no `sub` metadata (project · 2h ago), no `tone`, no per-row `kbd` AND active-row ↵ hint, no footer legend ("↑↓ navigate ↵ open esc close"), no "No matches for “q”" copy control; row min-height 44 vs Rift 36; list max 320px vs 68vh. Either extend Halo palette in the Svelte port (preferred — ConsoleX precedent unknown) or Rift-specific wrapper. Fuzzy scorer stays Rift's. Esc/Tab/arrow semantics: Halo "takes focus, traps Tab, hands focus back". |
| UpdateDialog | `Dialog` (title "Updates", description, footer buttons) + `Progress`/`Stat` for download % + `Banner`/`Badge` per state + `CodeBlock` for technical details | Hero-per-state has no Halo part: Rift-specific `UpdateHero` using `Stat` (version `v1.2 -> v1.3` mono, 30px metric) + `EmptyState`-like icon rings. **Kill** radial wash + glows. `Dialog` footer "secondary first, primary last" (Rift has ghost left / primary right — same). Release notes block: `Card` + prose. `busy` (no dismiss while installing) -> `Dialog` `onClose` undefined (type allows `onClose?`). Progress is stepwise ticks -> `Progress` easing hides the steps. |
| McpServersDialog | `Dialog` or `Drawer` (list + short form = Halo's Drawer use case "short forms") | **Gap:** header with 2 action buttons (Re-check, Add) + `Esc` kbd — `Dialog` has only title/description/close/footer; place actions in header slot via `description`/children or put in footer. Form fields -> `Field`+`TextField`/`Select`/`Segmented`/`RadioGroup`; TextField is 52px floating-label (heavy for 5 inline fields) — owner density question. Row status -> `Badge` + dot; remove-confirm inline strip -> `Banner tone=danger` (inline, with Confirm/Cancel) or `Dialog` stacked. **Drawer from the right lands on the native webview** if dock open. |
| Toasts / NotificationCenter / severity-tone.css | `Toast`, `NotificationPanel`, `Banner` | Out of this wave's files; `severity-tone.css` is consumed only there. Halo tones: accent/info/warn/danger (no `ok`) -> `ok` (Rift green) folds into `accent`; Halo has **no 'muted'/neutral toast tone**. Note for map owner of toasts. |
| `tooltip.ts` shim | `Tooltip` (120ms delay, 4px rise, inverted bubble, `aria-describedby`) | Decision needed (10). If kept banned: Halo `peek` + `Badge` + visible labels. If revived: Svelte `Tooltip` action/component, **bubble z-tooltip must not be clipped** by scrolling cards (`.ah-scroll`, `.dc-body overflow`), and cannot show over the native webview. |
| `portal.ts` | keep as util | Halo overlays are `position:fixed` (`.halo-overlay` `bundle.css:281`) so they don't need it; popovers/menus (absolute) do inside clipping ancestors. |

Not applicable / no Halo equivalent: virtualized log list, native webview stage, omnibox, updater state machine hero, MCP add form transport/scope semantics, advisor apply/undo delta, pace projection marker, vitals filter tile.

# 8 Pain points (evidence)

1. **Exact numbers are unreachable** — 8 `use:tooltip` call sites on AiHealth tiles do nothing (`tooltip.ts:24-41`); header comment says ~26 sites, real = 174/37 files; AiHealth comment :796-798 claims otherwise. DiagnosticsConsole's 10 icon buttons/filters rely on the same dead shim for hints ("Pause stream", "Show warn and above"...).
2. **Overlays are inconsistent**: 3 scrim recipes, 3 panel recipes, z 70/200/200/999/2000/2100/4000/9000 (section 2), Update shows a double fade (`app.css:612-619` animation + Svelte `fade` `UpdateDialog:95`), toasts outrank dialogs.
3. **Motion is thin and uneven**: only AiHealth cards have an entrance (4px, per-nth-child, order shifts with data, :1253-1262); Diagnostics scale-pops its page (:163) as if modal; numbers don't move except 2 tiles; charts don't draw; advice/state swaps pop; menus/forms/panels appear without animation (Browser menu :598, MCP form :168, update states :117-183).
4. **Loops that aren't real activity**: hot-verdict pulse (:1293), `LimitBar` hot sheen (`LimitBar:51-56`), N breathing vitals dots (:403-405), shimmer pill (:768) — violates "loops only for real activity" (`03-motion.md`). Several animate non-compositor props (`box-shadow` :1294; `background-position` :762-767, `:514-516`) vs Halo's "only opacity and transform run continuously" (`03-motion.md`, `04-performance.md`).
5. **Type scale sprawl**: AiHealth 12 distinct font sizes incl. 9.5/10.5/11.5 (:1409 etc.), Diagnostics 8, Update 8 (11…40px), PageHero 24/10; radii 4/5/6/7/8/9/10/13/16/999 mixed in single files; ~457 `px` literals across the 9 files. Prior audit (`docs/design/redesign-plan.md:79-85`): 388 `font-size: Npx` app-wide, "4 left edges"; AiHealth `PageHero` 820px vs body 1180px at >=1160px (:1246).
6. **Accent over-use / status redundancy**: accent on every card icon (:1433), advisor wash, verdict strip, tile tints, pills — plan usage shown multiple times (audit `redesign-plan.md:89-94`: verdict pill + Plan-limits card + footer bars; still true here: `Plan` pill :885, Plan card :1076, plus StatusBar elsewhere).
7. **No focus rings on AiHealth custom buttons** (none authored; contrast Halo "visible `shadow-focus` on every control") and Diagnostics buttons rely on UA outline except the search; Browser buttons `outline:none` only on inputs with border-color change (:718,901) — 2px/3:1 ring absent. a11y debt.
8. **Hard-coded duplicates of tokens**: `transition: width 0.3s ease` (:1464, `LimitBar:46`), literal colours in 3 dialogs, an off-palette orange `oklch(0.72 0.12 50)` (`Console:510`), `--dc-*` private palette (:327-330) that "mirrors" `--ok/--warn/--danger` but doesn't (oklch values differ from `app.css:189-197`).
9. **Dead code**: Diagnostics modal mode (never imported), `.ah-trend*` CSS, `.ah-chip.ok/warn/hot`, `UpdateDialog` `.btn.glow` helper (only one use), `severity-tone.css` unrelated to this wave.
10. **Reduced-motion gap**: Svelte `transition:` on palette/MCP/Update/Diagnostics ignore the media query (see 4).
11. **Native-webview z-order**: the HTML palette/dialog/menu cannot paint over the native browser when a page is open (documented in `browser/mod.rs:8-11`; only hide path is workspace != chat, `WebBrowserPage:371-379`). The browser's own overflow menu opens at `r.bottom+4` — directly over the stage (:26) — and its scrim/menu are in the MAIN webview, so with a live page they sit underneath it. NOT verified at runtime this wave (map-only); evidence is source + Rust doc.
12. **Doc drift**: comments referencing removed/changed things (`AiHealth :942`, `tooltip.ts:5` "~26", `app.css:277` "dialogs above toasts").

# 9 Risks

- **Tests referencing markup/classes**: none found. Vitest files touching this surface only test stores (`diagnosticsEvents.test.ts`, `diagnosticsHealth.test.ts`, `workspace.test.ts:23-220` references workspace id `"ai-health"/"diagnostics"`). No component/snapshot/e2e tests for these components; `scripts/cdp/c.sh` references `.ah-wrap` as a doc example (:22,137) and navigates by label "AI Health" (:484,517) + `window.__riftDiag` (:447-454). Renaming CSS classes breaks no test; renaming the **"AI Health"/"Diagnostics" titles** (`workspaceRegistry.ts:36-37`) or the `ai-health` id would break `c.sh nav/tour`. (Selectors/labels there are text-matching; unverified exactly how.)
- **`npm run check` = svelte-check + `check:tokens` + `check:ui`** (`package.json:10`). `check-ui` ratchets counts that may only go DOWN: hand-rolled `use:portal` (baseline 11), `role=dialog|menu|listbox|alertdialog` outside `ui/` (17), hex 21, rgb/hsl 16, oklch literal 131, arbitrary Tailwind 2. A Halo port that adds new hand-rolled dialogs/portals/`role="dialog"` ADDS to these counts and fails CI unless old ones are removed 1:1 or baseline is updated (`--update`). Replacing `rgba()`/`oklch` literals with Halo tokens lowers them (good: lock with `--update`). `check:tokens` compares `design-system/styles.css` vs `src/app.css` `:root` — introducing Halo token names (`--bg-100`...) into `app.css` requires updating the DS mirror or the check "FAILS on MISMATCH" for shared names; new names are "app-only" (INFO). Token strategy (map Halo names onto Rift vars vs new `--h-*`) is a cross-surface decision.
- **Native child webview + overlays/animation**: (a) any Halo overlay (Dialog, Drawer, CommandPalette, Popover, Toast stack, Tooltip bubble) overlapping the stage is hidden by the webview while a page is open — needs a "native overlay lock" (hide webview or snapshot) tied to overlay open state, which is a new Rust/IPC contract, not a restyle. (b) `halo-page`/`halo-rise` or any transform on an ancestor of `.wb-stage` desyncs bounds: `getBoundingClientRect` includes transforms, and each frame -> `browser_set_bounds` IPC. (c) Dock width tween (`AssistantPage:11-20`) = layout + IPC every frame; slower 300ms Halo duration prolongs it. (d) 220ms comment :177-182: bounds measured mid-animation were wrong — any new longer/entrance animation on the dock repeats that bug class.
- **WebView2 compositing bans**: app comments (`app.css:272-274`, `CommandPalette:382-384`, `Console:333`) — **no `backdrop-filter` on fixed overlays** (garbage rects, collapsed boxes). Halo `game-glass` blur and any glass dialog are out; Halo's default dialog scrim is opaque (`--scrim` rgba .72) = fine. `Skeleton` blur reveal (`halo-reveal` blur(6px) on content; `04-performance.md` says small areas only) — `filter: blur` on WebView2 text is unmeasured in this repo (unknown).
- **Perf**: Diagnostics streams events into up to 2000 rows -> keep windowing (`ROW_H` must match Halo row height; Halo `data` size 13-14px / min row 26px must be chosen once and reused in CSS+JS); Halo `DataTable` row ripple + FLIP on 2000 rows would be catastrophic. Halo `Card spotlight` pointer tracking (rAF) — hero only. `RollingNumber` on live tile numbers re-renders digits wheels; AiHealth re-renders every 30 s `nowTick` (:147) and 3 min poll -> avoid replaying entrances on those updates (Halo "lists only re-measure on change", "rise once").
- **a11y**: Halo `Dialog/Drawer/Palette` provide focus trap/return; Rift hand-rolls Tab trapping in Palette (:248-265) and Update (:39-55) and a capture-phase Esc in MCP (:129) because the composer textarea swallows Esc — a Halo `Dialog`'s own Esc handler may lose that fight (needs capture-phase or focus-first guarantee; MCP effect `:91-98` focuses the panel for this reason). `aria-modal` + `role=dialog` kept. Halo contrast: `ink-faint #7d8592` on `bg-100` for timestamps — Rift uses `--fg-faint` for log times (:458); Halo's floor is for short labels only (`06-accessibility.md`) — log metadata in `ink-faint` OK, long text not. Touch target 44px vs dense toolbars. Tooltip/aria: replacing the shim with Halo Tooltip must preserve the "promote to aria-label" behaviour for icon-only buttons.
- **Theme**: Halo `blue` swaps `info` to lime and keeps `chart-3` violet (`tokens.css` `[data-theme="blue"]`). Anything keyed on info (log info level, "API is slow" banner, MCP info) turns lime. Rift's current accent is user-configurable (`--accent-h/-c`, `app.css:171-173`, emerald default; Settings accent picker) vs Halo's 8 fixed presets: owner must confirm whether the accent picker dies. Halo status colours differ (`warn #ff9f5a` orange vs Rift amber `oklch(.78 .16 70)`; `danger #ff6b6b`; no `ok` green — positive = accent).
- **Fonts**: Halo `bundle.css` pulls Geist from Google Fonts; Tauri CSP / offline first-run impact unknown (not investigated); bundling Geist locally likely needed. `Geist Mono` tabular-nums matters for all counters.
- **Z-index migration**: moving 200/2000/2100/4000 -> Halo 50/60/70 touches ToastHost, NotificationCenter, ui/ primitives, Splash (cross-surface). Do it as one token change.
- **Bits-ui `components/ui/` coexistence**: Rift already has shadcn+bits-ui Dialog-ish primitives (`alert-dialog`, `dropdown-menu`, `select`) restyled to Rift tokens; Halo port would create a third system unless these are re-skinned to Halo.
- **Svelte specifics**: Halo is React; `index.d.ts` props are React nodes (`ReactNode`, `CSSProperties`) — Svelte port needs snippets/Snippet props (e.g., `trigger`, `footer`, `Stat` children sparkline). ConsoleX precedent exists (file not read this wave -> unknown how it handled Menu portals/focus return).

# 10 Open owner questions

1. **Tooltips**: revive (Halo `Tooltip`, 120ms, inverted bubble) or keep banned (2026-06-15) and rely on `Stat peek` + visible labels? Affects 174 call sites; native `title=` (browser/MCP) should also converge on one answer.
2. **Accent picker / themes**: Halo Electric blue only (`data-theme="blue"`), or keep user-configurable hue? (Rift `--accent-h/-c` vs Halo fixed presets.) Light themes ship eventually?
3. **`info` = lime in Halo blue**: acceptable for log-level info / upstream-slow banner, or override `info` to a blue-grey for Rift? (Otherwise log `info` lines go lime.)
4. **Density**: Halo targets 44px controls and 52px TextFields, 16px card radius, 24px pad. Browser toolbar (30px), MCP form fields (28px), Diagnostics rows (26px), palette rows (36px) are denser. Allow a "compact" variant for tool surfaces (Browser/Diagnostics/MCP) or go fully Halo-spacious?
5. **Native browser overlay contract**: OK to add a Rust/IPC "overlay open -> hide (or freeze) the native webview" so palette/dialogs/menus/toasts can cover the dock? Alternative: constrain overlays outside the dock. Needed before any Halo Dialog/Popover work near the browser.
6. **Browser menu**: keep the portal + `.rift-menu`, or Halo `Menu` (absolute, clipped by `overflow:hidden`, hidden by native webview)?
7. **Diagnostics modal mode**: confirm it can be deleted (no importer found); is a global "open Diagnostics as overlay" planned (e.g. from Settings -> System)?
8. **Diagnostics log**: keep windowed list on Halo tokens (my recommendation) vs `DataTable` (needs virtualization, which Halo lacks)? Column/level colour semantics: neutral for trace/debug/info, warn/danger only for warn/error?
9. **AiHealth structure**: Halo wants one screen / numbered `SectionLabel`s / list scrolls inside card. Keep a scrolling page with 2-col grid, or restructure to `01 / PLAN`, `02 / SPEED`, `03 / SPEND`, `04 / MCP`? Where does the Advisor live (hero card vs `Drawer`)? Add `LineChart` + `Segmented` range for latency, `BarChart` for spend, one `Ring` for peak limit?
10. **Verdict surface**: permanent Badge+LiveDot summary and Banner only when not healthy — OK? Should "Healthy" stay visible at all?
11. **Experimental/Title Case**: hero eyebrow "Experimental" + "Claude Usage & Health" (flagged in prior audit) — rename under Halo's sentence-case labels?
12. **CommandPalette extensions**: extend Halo's palette API (real group headings, sub text, row kbd, footer legend) or accept Halo's lighter version (group label on the right, no recents metadata)? Keep Ctrl+K/Ctrl+P bindings in `AppShell` (Halo adds none).
13. **Dialog family**: UpdateDialog and MCP as `Dialog`, or MCP as `Drawer`? Keep UpdateDialog's state-hero layout (Rift-specific) with Halo parts?
14. **Motion defaults**: adopt Halo durations (150/300/600, stagger 60, `ease-out`) globally replacing Rift's 140/240/520/400, `ease-page`? (This wave maps 1:1 retunes assuming yes.) Also: do dock-slide / webview bounds animate at all under Halo (hide-during-slide acceptable)?
15. **Reduced motion**: should the app's own accessibility store (`prefersReducedMotion()`, `state/accessibility.svelte.ts:103`) override the OS media query (it already does for `AnimatedCount`, `UpdateBanner`) — Halo CSS only honours the media query.
16. **Scope check**: `severity-tone.css` has no consumer in this wave (Toast/Notification only) — assign to the toasts map; confirm Halo `Toast` tones (no `ok`/neutral) are acceptable.
17. **Fonts**: Geist via Google Fonts vs bundled locally (offline/CSP)?
