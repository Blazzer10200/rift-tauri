---
title: Rift · Halo — 03 Motion system
status: draft
updated: 2026-10-19
phase: P2 (+ app-wide motion catalogue, Track B streaming engine, canvas board 04 Motion lab)
depends_on:
  - spec/00-decisions.md (D-10, D-11, D-12, D-13, D-14, D-15, D-16, D-29; also D-08, D-19, D-20, D-33)
  - P0 spikes D-28 #1 (floatingMotion dead?), #3 (interpolate-size), #7 (hydration replays blockIn?), #8 (end-of-turn blink recorded)
  - P1 landed (vendored halo/tokens.css + alias block) with the motion pins described in §13 A-1
---

# Rift · Halo — 03 Motion system

## 0 · Summary

1. One home for motion: `src/lib/styles/motion.css` holds every keyframe, the Rift-only motion tokens and the reduced-motion net; Halo's `tokens.css` owns 150/300/600, stagger 60, `ease-out`, `ease-spring`.
2. Loops drop from 81 single-line `infinite` declarations to real activity only: one Spinner (900ms), one LiveDot ping (1.8s), one indeterminate bar (1.3s), one `.shim` live verb, three cursors.
3. Reduced motion becomes one global net + a `[data-essential-motion]` exemption, a reactive `prefersReducedMotion()` and an `rm()` wrapper for Svelte transitions; rAF engines jump to their end state.
4. Entrances run only for live-born content (`data-live`); re-mounts get `data-settled` (0s durations and delays for 450ms); hidden keep-alive pages pause their loops.
5. Track B extracts `reveal.ts` / `follow.ts` (pure, frame-rate-independent, unit-tested); canvas board 04 runs that same code and exports the pacing numbers back into §9.
6. Every value is a token, every literal is ratcheted, and every gated number (`--dur-base` 240→300, word fade 500 blur→300 opacity) sits in its own revertable commit on a side branch until the owner plays board 04.

## 1 · Scope + files

Branch `design/halo` (cut from `design/chat-ledger` @ 22727d24, D-25). Line numbers below are from 22727d24 and were opened this session unless marked `(map 09 Xn)`.

| File | Lines | What this spec changes |
|---|---|---|
| `src/lib/styles/motion.css` | new (~260) | keyframes, motion tokens, recipes, reduced-motion net, replay guard, keep-alive pause |
| `src/app.css` | 780 | `:3` import motion.css · `:109-111` bridge keys (`--animate-*` wipe deferred to P3) · `:117-122` delete `[data-slot]` RM rule · `:239-250` motion token block (P1 pins → removed one per commit) · `:353` `--rail-w` duration · `:403,431` press literals · `:459-465` pill-breathe · `:623-629` `.spin/.pulse` · `:634-650` `.enter` |
| `src/lib/state/accessibility.svelte.ts` | 106 | `:101-106` reactive reduced-motion state |
| `src/lib/motion/rm.ts` | new (~25) | `rm()` wrapper |
| `src/lib/motion/transitions.ts` | new (~120) | Svelte `css` transitions mirroring motion.css (rise, pop, zoom, drawer, toast, accordion), `easeOut` bezier |
| `src/lib/motion/tokens.ts` | new (~40) | JS mirror of the ms tokens + parity test |
| `src/lib/motion/reveal.ts` | new (~90) | Track B: word-reveal cursor (pure) |
| `src/lib/motion/follow.ts` | new (~60) | Track B: scroll-follow step (pure) |
| `src/lib/actions/settleGuard.ts` | new (~30) | D-29 `data-settled` action |
| `src/lib/styles/stream.css` | 555 | `:27` blockIn → halo-rise · `:31-33` bodyFadeIn/workOpen · `:34` ringspin · `:35,329-343` shim · `:40-44` streamLivePulse · `:73-88` dotPop/headRise · `:260-264` `.wline` width morph · `:372-384` RM block · `:455` `.splan-fill` |
| `src/lib/components/assistant/Markdown.svelte` | 1477 | `:158` snapshot · `:169-172` constants → reveal.ts · `:216-252` loop → reveal.ts · `:684,692-699` md-word · `:777,1088-1095` code-rise · `:1212-1213` backdrop |
| `src/lib/components/assistant/AssistantPane.svelte` | 1563 | `:145,256` native smooth scroll · `:167` snapshot · `:169-190` pinToBottom → follow.ts · `:252-257` jumpToLatest · `:359-384` FLIP literal · `:714` `.stream` guard · `:895,916` paneIn · `:1022` ctx fill width · `:1042-1049` live pulse · `:1148` composer-host enter · `:1156-1160` will-change · `:1263-1268` pane-empty-in · `:1390` jump-in · `:1432` tick width |
| `src/lib/components/assistant/stream/StreamTurn.svelte` | 554 | `:326` `.sturn` gets `data-live` · `:474` slide → accordion |
| `src/lib/components/shell/WorkspaceShell.svelte` | 212 | `:6-9,47-73` delete `.rising`/`SETTLE_MS` · `:128-148` page transitions · `:160-191` skeleton · `:194-211` delete ws-child-rise · keep-alive pause |
| `src/lib/components/AppShell.svelte` | 513 | `:360` `.app` 1s background (deleted in P4a per D-01; listed here only) · `:417` margin ease · `:483-486,493` delete `wsIn` |
| `src/lib/components/shell/Sidebar.svelte` | 352 | `:221` rail width · `:274` drop `top` from transition |
| `src/lib/components/shell/ConversationList.svelte` | 544 | `:479` rowIn first mount only · `:512` backdrop · `:533-543` workdots → LiveDot |
| `src/lib/components/SplashOverlay.svelte` | 293 | `:149` door curve token · `:177-184` exit blur · `:216` stagger 55→60 · `:275-280` caret blink |
| `src/lib/components/assistant/Composer.svelte` | 2604 | `:1833-1834,2251-2273` ripple · `:1886-1887,1921-1922` backdrop · `:1895-1904` drop-dot · `:2130-2131` mic-spin · `:2149` mic bars height · `:2333-2390` enhance magic · `:2533-2534` provider pulse · `:2591-2601` ultra-pulse |
| 25 spinner files | — | see C16 list (one keyframe each, deleted) |
| `scripts/cdp/serve.cjs` | 1508 | `:1350-1367` new `media` op |
| `scripts/cdp/c.sh` | 665 | new `media` and `sample` verbs (beside `reload)` at `:437`) |
| `scripts/cdp/raf-sampler.js` | new (~30) | frame-time sampler |
| `scripts/cdp/README.md` | 164 | `:107-124` command reference |
| `design-system/check-ui.mjs` | 188 | `:94-105` motion metrics · `:108` scan `.css` too |
| `src/routes/dev/gallery/+page.svelte` | 166 | Track B: param panel bound to reveal/follow params; `:162` `scroll-behavior:smooth` removed |

Out of scope here (owned by other specs, motion values still defined in §4): overlay primitives and their markup (P3, D-20), NavItem marker markup (P4a, D-19), the D-12 same-node fold template (P4b), Shiki growing-fence rule (P4b, D-11 #6), tooltip singleton (P3, D-17), StatTile count-up component (P3, D-24).

## 2 · Target design

### 2.1 Token table

Halo-owned (from vendored `src/lib/styles/halo/tokens.css`, same values as `consolex/panel/design-systems/halo/tokens.css:43-48`):

| Token | Resolved | Use |
|---|---|---|
| `--dur-fast` | 150ms | hover, press, colour, pop-in, tooltip, chevron of small rows |
| `--dur-base` | 300ms (**gated #3**, pinned 240ms until the owner says yes) | state change, accordion, dialog, drawer, toast, chevron, sliding thumbs |
| `--dur-slow` | 600ms | entrances that draw or count, bar grow, skeleton reveal |
| `--stagger` | 60ms | page sections (cap 6) |
| `--ease-out` | `cubic-bezier(.2,.8,.2,1)` | every non-loop motion |
| `--ease-spring` | `cubic-bezier(.34,1.56,.64,1)` | flip/grab only: toggle knob, check/radio pop, slider thumb, detent snap, tag chip, count pop, send/stop glyph swap |

Rift aliases (P1 alias block, D-08): `--ease-page`, `--ease-soft` → `var(--ease-out)`; `--dur-rise` → `var(--dur-slow)`; `--pulse-live` → `var(--dur-ping)`.

Rift-only motion tokens (declared in `motion.css :root`, D-10 named exceptions):

| Token | Value | Use |
|---|---|---|
| `--dur-page` | 520ms | page enter |
| `--dur-page-delay` | 60ms | page enter delay |
| `--dur-page-out` | 140ms | page leave (opacity only) |
| `--dur-pop-out` | 140ms | menu / popover / select leave |
| `--dur-leave` | 200ms | modal, drawer, banner, scrim leave |
| `--dur-toast-out` | 240ms | toast leave |
| `--dur-menu-item` | 220ms | menu row rise |
| `--stagger-menu` | 18ms | menu rows, cap 12 |
| `--dur-stream` | 240ms (**gated with D-11**) | stream block entrance |
| `--dur-word` | 300ms (**gated #3**; today 500ms) | per-word fade; written inline by `Markdown` from `reveal.ts` `REVEAL_MS` so JS and CSS cannot drift |
| `--dur-flip` | 460ms | composer FLIP, FlipList, QueueRail |
| `--dur-count` | 700ms | count-up (first data, range change) |
| `--dur-roll` | 900ms | RollingNumber strip |
| `--dur-skel-delay` | 300ms | skeleton bones appear only after this |
| `--dur-skel-out` | 360ms | bones fade out |
| `--dur-ping` | 1.8s | LiveDot ping period |
| `--dur-spin` | 900ms | Spinner |
| `--dur-indet` | 1.3s | indeterminate Progress |
| `--dur-shim` | 1.5s | the one live verb (today's value, kept) |
| `--dur-ripple` | 600ms | send ripple, once |
| `--dur-caret` | 1.06s | terminal / dictation / exit-plan caret (`steps(1)`) |
| `--rise-page` | 10px | pages, cards, dialog content, empty states, banner |
| `--rise-reveal` | 6px | skeleton → content hand-off |
| `--rise-stream` | 3px | stream blocks (gated with D-11) |
| `--ease-door` | `cubic-bezier(.65,0,.35,1)` | splash doors only (D-26) |

Fixed geometry constants (not tokens, written once in the keyframes): pop scale .96 / y ∓4px; menu row y −3px; dialog scale .94 / y 8px, leave scale .96; drawer x 40px; toast y −16px / scale .96 (top-centre, D-20), leave x +32px; press scale .97; ping scale 1→3 / opacity .7→0; count pop scale 1.18; ripple scale .6→2.2 / opacity .9→0; accordion content y −4px.

Density tiers do not change motion. Durations and distances are identical in compact / regular / comfy (D-06 is geometry only).

### 2.2 `src/lib/styles/motion.css` (complete)

Imported from `src/app.css:3` (`@import "./lib/styles/motion.css";`) so it rides the Tailwind pipeline and sits after `tailwindcss` / `tw-animate-css` and before the Rift token block. All names carry a `halo-` prefix; none collide with the 173 existing `@keyframes` (verified: the global names in use are `spin`, `pulse-soft`, `pill-breathe`, `gh-dot-breathe`, `overlay-in`, `enter`, `pop-in`, the stream.css set, the onboarding set and `-global-swPopIn`/`-global-convPopIn`).

```css
/* ════════════════════════════════════════════════════════════════════
   Rift · Halo — motion home (spec 03). ONE place for keyframes + motion
   tokens + reduced-motion net + replay guard. Values: D-10…D-16, D-29.
   Halo-owned tokens (dur-fast/base/slow, stagger, ease-out, ease-spring)
   come from halo/tokens.css — never redeclare them here.
   ════════════════════════════════════════════════════════════════════ */

:root {
  --dur-page: 520ms;
  --dur-page-delay: 60ms;
  --dur-page-out: 140ms;
  --dur-pop-out: 140ms;
  --dur-leave: 200ms;
  --dur-toast-out: 240ms;
  --dur-menu-item: 220ms;
  --stagger-menu: 18ms;
  --dur-stream: 240ms;          /* gated with D-11 (board 04) */
  --dur-word: 300ms;            /* overwritten inline by Markdown from reveal.ts */
  --dur-flip: 460ms;
  --dur-count: 700ms;
  --dur-roll: 900ms;
  --dur-skel-delay: 300ms;
  --dur-skel-out: 360ms;
  --dur-ping: 1.8s;
  --dur-spin: 900ms;
  --dur-indet: 1.3s;
  --dur-shim: 1.5s;
  --dur-ripple: 600ms;
  --dur-caret: 1.06s;
  --rise-page: 10px;
  --rise-reveal: 6px;
  --rise-stream: 3px;           /* gated with D-11 (board 04) */
  --ease-door: cubic-bezier(0.65, 0, 0.35, 1);
}

/* ── Entrances ─────────────────────────────────────────────────────── */
/* One rise keyframe; the distance comes from --rise on the element
   (custom properties resolve per element inside keyframes in Chromium). */
@keyframes halo-rise { from { opacity: 0; transform: translateY(var(--rise, var(--rise-page))); } }
.rise-page   { --rise: var(--rise-page);   animation: halo-rise var(--dur-slow)   var(--ease-out) both; }
.rise-stream { --rise: var(--rise-stream); animation: halo-rise var(--dur-stream) var(--ease-out) both; }
.rise-row    { --rise: var(--rise-stream); animation: halo-rise var(--dur-base)   var(--ease-out) both;
               animation-delay: calc(min(var(--i, 0), 12) * 25ms); }

/* Skeleton → content hand-off (D-14, Halo exact). The only content blur. */
@keyframes halo-reveal { from { opacity: 0; filter: blur(6px); transform: translateY(var(--rise-reveal)); } }
.rise-reveal { animation: halo-reveal var(--dur-slow) var(--ease-out) both; }

/* Page sections: first open of a workspace only (they mount once; keep-alive). */
@keyframes halo-page-in { from { opacity: 0; transform: translateY(var(--rise-page)); } }
.page-sections > * { animation: halo-page-in var(--dur-page) var(--ease-out) var(--dur-page-delay) both; }
.page-sections > :nth-child(2) { animation-delay: calc(var(--dur-page-delay) + var(--stagger) * 1); }
.page-sections > :nth-child(3) { animation-delay: calc(var(--dur-page-delay) + var(--stagger) * 2); }
.page-sections > :nth-child(4) { animation-delay: calc(var(--dur-page-delay) + var(--stagger) * 3); }
.page-sections > :nth-child(5) { animation-delay: calc(var(--dur-page-delay) + var(--stagger) * 4); }
.page-sections > :nth-child(n+6) { animation-delay: calc(var(--dur-page-delay) + var(--stagger) * 5); }
@keyframes halo-page-out { to { opacity: 0; } }

@keyframes halo-fade-in  { from { opacity: 0; } }
@keyframes halo-fade-out { to   { opacity: 0; } }

/* ── Popups: menu, popover, select, context, HUD (D-20) ─────────────── */
@keyframes halo-pop-in    { from { opacity: 0; transform: scale(0.96) translateY(-4px); } }
@keyframes halo-pop-in-up { from { opacity: 0; transform: scale(0.96) translateY(4px); } }
@keyframes halo-pop-out   { to   { opacity: 0; transform: scale(0.97); } }
@keyframes halo-menu-item { from { opacity: 0; transform: translateY(-3px); } }
.menu-rows > * { animation: halo-menu-item var(--dur-menu-item) var(--ease-out) backwards;
                 animation-delay: calc(min(var(--i, 0), 12) * var(--stagger-menu)); }

/* ── Dialog, palette, scrim (D-20) ──────────────────────────────────── */
@keyframes halo-zoom-in  { from { opacity: 0; transform: scale(0.94) translateY(8px); } }
@keyframes halo-zoom-out { to   { opacity: 0; transform: scale(0.96); } }

/* ── Drawer (Output inspector, D-20) ────────────────────────────────── */
@keyframes halo-drawer-in  { from { opacity: 0; transform: translateX(40px); } }
@keyframes halo-drawer-out { to   { opacity: 0; transform: translateX(40px); } }

/* ── Toast (top-centre, D-20: enters from above) ────────────────────── */
@keyframes halo-toast-in { from { opacity: 0; transform: translateY(-16px) scale(0.96); } }
/* Leave + stack fan are TRANSITIONS on vars (Halo bundle.css:318-323), not
   keyframes, because the stack offset (--y, --s, --dx) must compose. */
.toast-motion {
  transform: translateX(var(--dx, 0px)) translateY(var(--y, 0px)) scale(var(--s, 1));
  opacity: calc(var(--o, 1) * var(--fade, 1));
  transition: transform var(--dur-base) var(--ease-out), opacity var(--dur-base) var(--ease-out);
  animation: halo-toast-in var(--dur-base) var(--ease-out);
}
.toast-motion.is-leaving {
  opacity: 0;
  transform: translateX(calc(var(--dx, 0px) + 32px)) translateY(var(--y, 0px)) scale(var(--s, 1));
  transition: transform var(--dur-toast-out) var(--ease-out), opacity var(--dur-toast-out) var(--ease-out);
}

/* ── Banner (UpdateBanner inside content, D-20) ─────────────────────── */
.banner-motion { --rise: var(--rise-page); animation: halo-rise var(--dur-base) var(--ease-out); }
@keyframes halo-banner-out { to { opacity: 0; transform: translateY(calc(var(--rise-reveal) * -1)); } }

/* ── Accordion: one look, two carriers (see §13 A-4) ────────────────── */
/* Carrier A — always mounted, light content (settings expanders, .wline rows). */
.acc-panel { display: grid; grid-template-rows: 0fr;
             transition: grid-template-rows var(--dur-base) var(--ease-out); }
.acc-panel[data-open] { grid-template-rows: 1fr; }
.acc-panel > .acc-inner { overflow: hidden; min-height: 0; }
.acc-panel .acc-content { opacity: 0; transform: translateY(-4px);
                          transition: opacity var(--dur-base) var(--ease-out), transform var(--dur-base) var(--ease-out); }
.acc-panel[data-open] .acc-content { opacity: 1; transform: none; transition-delay: 60ms; }
/* Carrier B — {#if}-gated heavy content: transitions.ts `accordion` emits the
   same grid-rows + content fade from JS (same tokens, same curve). */

/* One disclosure glyph app-wide: chevron-down, 180° (D-12). */
.chev { transition: transform var(--dur-base) var(--ease-out), color var(--dur-base) var(--ease-out); }
[aria-expanded="true"] > .chev, .chev[data-open] { transform: rotate(180deg); }

/* ── Loops: real activity only (D-13) ───────────────────────────────── */
@keyframes halo-spin { to { transform: rotate(360deg); } }
.halo-spin { animation: halo-spin var(--dur-spin) linear infinite; }

@keyframes halo-ping { from { transform: scale(1); opacity: 0.7; } to { transform: scale(3); opacity: 0; } }
.live-dot { position: relative; width: 8px; height: 8px; border-radius: 50%; background: var(--live-tone, var(--accent)); flex: none; }
.live-dot::after { content: ""; position: absolute; inset: 0; border-radius: 50%; background: inherit;
                   animation: halo-ping var(--dur-ping) var(--ease-out) infinite; }
.live-dot[data-idle]::after { animation: none; }

@keyframes halo-indet { from { transform: translateX(-100%); } to { transform: translateX(250%); } }
.indet-bar { width: 40%; animation: halo-indet var(--dur-indet) var(--ease-out) infinite; }

/* The ONE live verb (owner-approved M16). The single declared exception to
   "only opacity/transform loop": background-position on clipped text. */
@keyframes halo-shim { from { background-position: 100% 0; } to { background-position: -100% 0; } }
.shim {
  --sh-b: var(--ink-faint); --sh-h: var(--ink);
  background-image: linear-gradient(100deg, var(--sh-b) 0%, var(--sh-b) 38%, var(--sh-h) 50%, var(--sh-b) 62%, var(--sh-b) 100%);
  background-size: 220% 100%;
  -webkit-background-clip: text; background-clip: text;
  -webkit-text-fill-color: transparent; color: transparent;
  animation: halo-shim var(--dur-shim) linear infinite;
}

@keyframes halo-caret { 50% { opacity: 0; } }
.halo-caret { animation: halo-caret var(--dur-caret) steps(1) infinite; }

/* ── Skeleton (D-14): static bones, delayed, Halo hand-off ──────────── */
.skel-bones { animation: halo-fade-in var(--dur-fast) var(--ease-out) var(--dur-skel-delay) both; }
@keyframes halo-skel-out { to { opacity: 0; filter: blur(4px); } }
.skel-out { position: absolute; inset: 0; pointer-events: none;
            animation: halo-skel-out var(--dur-skel-out) var(--ease-out) forwards; }

/* ── One-shots ──────────────────────────────────────────────────────── */
@keyframes halo-pop { 50% { transform: scale(1.18); } }        /* count pop, check/radio pop */
.count-pop { animation: halo-pop var(--dur-base) var(--ease-spring); }
@keyframes halo-tag-in { from { opacity: 0; transform: scale(0.6); } }   /* attachment chip */
.tag-in { animation: halo-tag-in var(--dur-base) var(--ease-spring) both; }
@keyframes halo-draw { to { stroke-dashoffset: 0; } }          /* check path, sparkline */
@keyframes halo-ripple { from { opacity: 0.9; transform: scale(0.6); } to { opacity: 0; transform: scale(2.2); } }
.ripple-once { animation: halo-ripple var(--dur-ripple) var(--ease-out) both; }
@keyframes halo-bell-swing { 12% { transform: rotate(16deg); } 28% { transform: rotate(-13deg); }
  44% { transform: rotate(9deg); } 60% { transform: rotate(-5deg); } 76% { transform: rotate(2deg); } }
.bell-swing { transform-origin: 50% 3px; animation: halo-bell-swing var(--dur-roll) var(--ease-out); }
/* jump-to highlights (search jump, tool jump) keep their own one-shot keyframes
   (SettingsPage .sflash, MessageBubble .tid-flash): orientation, not decoration. */

/* ── D-29: entrances only for live-born content ─────────────────────── */
/* A container that re-MOUNTS content (hydration, tab switch, split swap,
   re-parent, {#key} redraw, 60s data tick) carries [data-settled] for 450ms
   (settleGuard action). Duration AND delay go to 0s (not none): rows land
   on their final frame at once and nothing replays when the attribute
   goes (ConsoleX 51f4d02, Claude.svelte:1344-1349). */
[data-settled], [data-settled] *, [data-settled] *::before, [data-settled] *::after {
  animation-duration: 0s !important;
  animation-delay: 0s !important;
}
/* …but real-activity loops keep running through a settle window. */
[data-settled] .halo-spin, [data-settled] [data-essential-motion] .halo-spin { animation-duration: var(--dur-spin) !important; }
[data-settled] .live-dot::after { animation-duration: var(--dur-ping) !important; }
[data-settled] .shim            { animation-duration: var(--dur-shim) !important; }
[data-settled] .halo-caret      { animation-duration: var(--dur-caret) !important; }
[data-settled] .indet-bar       { animation-duration: var(--dur-indet) !important; }

/* Hidden keep-alive pages (WorkspaceShell) pause every loop and entrance. */
.ws-page[data-active="false"], .ws-page[data-active="false"] *,
.ws-page[data-active="false"] *::before, .ws-page[data-active="false"] *::after {
  animation-play-state: paused !important;
}

/* ── Reduced motion: one global net (D-13, Halo bundle.css:901-902) ── */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    animation-delay: 0s !important;          /* Rift addition, §13 A-3 */
    transition-duration: 0.01ms !important;
    transition-delay: 0s !important;         /* Rift addition, §13 A-3 */
    scroll-behavior: auto !important;
  }
  /* The Spinner alone keeps turning: a frozen spinner reads as hung.
     Must re-declare with !important + higher specificity to beat the net;
     the svg CHILD is the element that spins. */
  [data-essential-motion], [data-essential-motion] * {
    animation-duration: var(--dur-spin) !important;
    animation-iteration-count: infinite !important;
  }
  .live-dot::after { animation: none !important; }              /* dot stays solid */
  .shim { animation: none !important; background: none !important;
          -webkit-text-fill-color: currentColor !important; color: var(--ink-muted) !important; }
  .halo-caret { animation: none !important; opacity: 1 !important; }   /* cursors go solid */
  .skel-out { display: none !important; }                       /* swap instantly */
}
```

Notes an engineer needs:

- `.page-sections`, `.menu-rows`, `.rise-*`, `.live-dot`, `.shim`, `.halo-spin`, `.acc-panel`, `.chev` are global classes by design (one vocabulary). Svelte components apply them in markup; a scoped `<style>` never re-declares their keyframes.
- `[data-settled] *` beats component CSS through `!important`; it does not touch `transition-*`. A re-mount never fires a CSS transition (no prior computed style), and Svelte 5 transitions are local (they do not play when an ancestor mounts), so zeroing transitions would only kill legitimate hover feedback during the 450ms window.
- The net zeroes what a Svelte `css` transition paints, but not Svelte's timer: an `out:` transition still delays the unmount by its declared duration. `rm()` stays mandatory on every Svelte transition (§5.3).
- `.live-dot` here is the minimum CSS so P2 can swap pulses before the P3 `ui/LiveDot.svelte` exists; P3 wraps this class, it does not fork it.

### 2.3 Tailwind bridge (so `ui/` recipes can use the keyframes)

Do **not** wipe `--animate-*` in P2: `tw-animate-css` registers `--animate-in` / `--animate-out` (verified in `node_modules/tw-animate-css/dist/tw-animate.css`) and `recipes.ts:20,58,60` use them, so an `initial` wipe would make `check:ui`'s class-generation check fail until P3 rewrites the recipes. The wipe moves to P3's recipes commit. `src/app.css:109-111` becomes:

```css
  --ease-page: var(--ease-page);
  --ease-soft: var(--ease-soft);
  --ease-spring: var(--ease-spring);
  --ease-out: var(--ease-out);
  --animate-halo-pop-in: halo-pop-in var(--dur-fast) var(--ease-out) both;
  --animate-halo-pop-in-up: halo-pop-in-up var(--dur-fast) var(--ease-out) both;
  --animate-halo-pop-out: halo-pop-out var(--dur-pop-out) var(--ease-out) both;
  --animate-halo-zoom-in: halo-zoom-in var(--dur-base) var(--ease-out) both;
  --animate-halo-zoom-out: halo-zoom-out var(--dur-leave) var(--ease-out) both;
  --animate-halo-fade-in: halo-fade-in var(--dur-fast) var(--ease-out) both;
  --animate-halo-fade-out: halo-fade-out var(--dur-leave) var(--ease-out) both;
  --animate-halo-drawer-in: halo-drawer-in var(--dur-base) var(--ease-out) both;
  --animate-halo-drawer-out: halo-drawer-out var(--dur-leave) var(--ease-out) both;
  --animate-halo-spin: halo-spin var(--dur-spin) linear infinite;
```

P3 then writes `data-[state=open]:animate-halo-pop-in data-[state=closed]:animate-halo-pop-out data-[side=top]:data-[state=open]:animate-halo-pop-in-up` in `recipes.ts:20` (D-09 fix; not this spec). `tw-animate-css` (`app.css:2`) is removed in P3 after the last `animate-in`/`zoom-in-95` utility leaves `recipes.ts`, and `--animate-*: initial;` is added in that same commit.

Name collision found while verifying: `tw-animate-css` defines global `@keyframes enter` / `exit` (the frames behind `animate-in` / `animate-out`), and Rift's own global `@keyframes enter` at `app.css:634` (opacity + 4px rise, used by `.enter` at `:645`) comes later in the cascade and replaces it. So the primitives' `animate-in` cannot be playing tw-animate's fade/zoom today; feed this into D-28 spike #1. Rift's `.enter` / `@keyframes enter` (`app.css:634-650`) is retired surface by surface: each consumer (composer bar, alerts, ToolChip tile, MessageBubble `.bubble`, composer-host) moves to `.rise-stream` or `.rise-page` in M28 (stream/pane consumers) or in the P4 port that touches it; the keyframe is deleted with its last consumer.

### 2.4 Layout notes where motion depends on layout

Sidebar collapse / peek (D-19), today `Sidebar.svelte:221,227-228,274`:

```
 expanded                       collapsed                    peeking (hover the edge)
┌──────┐┌────────────┐         ┌─┐┌──────────────────┐      ┌─┐┌──────────────────┐
│rail  ││ main       │  300ms  │ ││ main             │      │ │┌──────┐          │
│240px ││ margin-left│ ──────► │0││ margin-left 12   │      │0││island│ top 54   │
│      ││ = rail+12  │ width,  │ ││                  │      │ ││float │ shadow   │
└──────┘└────────────┘ margin, └─┘└──────────────────┘      │ │└──────┘          │
                       --rail-w all var(--dur-base) ease-out └─┘└──────────────────┘
```

- `.side-rail` width, `.main` margin-left (`AppShell.svelte:417`) and `html{--rail-w}` (`app.css:353`) all use `var(--dur-base) var(--ease-out)`: one move (fixes the 140ms vs 240ms desync recorded in 00 "Stale facts").
- Island: `transform` + `opacity` only. `top` leaves the transition list. Entering peek, the island is invisible when `top` changes, so it snaps; leaving peek it would jump while fading, so the collapsed state delays the snap: `.side-rail.collapsed:not(.peeking) .sidebar { transition: transform var(--dur-base) var(--ease-out), opacity var(--dur-base) var(--ease-out), top 0s linear var(--dur-base); }`.

Page change (D-10): every opened workspace stays mounted (`WorkspaceShell.svelte:79-111`).

```
first open of a workspace            later activations
(element mounts: {#if everOpened})   (data-active flips on a mounted page)
  section 1  ↑10px  60ms               whole .ws-page:
  section 2  ↑10px 120ms                 in : opacity 0→1, translateY 10→0,
  section 3  ↑10px 180ms                      520ms ease-out, delay 60ms
  …cap 6 (n≥6 share 360ms)               out: opacity 1→0, 140ms ease-out;
  each 520ms ease-out                          transform snaps after the fade
```

The `.ws-page` CSS (replaces `WorkspaceShell.svelte:128-148`):

```css
.ws-page { position: absolute; inset: 0; display: flex; flex-direction: column; overflow: hidden;
  opacity: 0; pointer-events: none; transform: translateY(var(--rise-page));
  transition: opacity var(--dur-page-out) var(--ease-out), transform 0s linear var(--dur-page-out); }
.ws-page[data-active="true"] { opacity: 1; pointer-events: auto; transform: none; z-index: 1;
  transition: opacity var(--dur-page) var(--ease-out) var(--dur-page-delay),
              transform var(--dur-page) var(--ease-out) var(--dur-page-delay); }
/* Boot: the intro handoff owns the entrance. Duration/delay 0s, NOT `animation: none`:
   a finished 0s animation stays finished when data-intro clears, whereas `none` → name
   flips back and the cascade would play a second time at 700ms. */
:global(.app[data-intro]) .page-sections > * { animation-duration: 0s; animation-delay: 0s; }
```

The same latent replay exists today at `AppShell.svelte:493` (`.app[data-intro] .workspace { animation: none; }` → `wsIn` starts when the intro clears); M30 deletes `wsIn`, which removes it.

Page components opt their section column in with `class="page-sections"` (Home `WorkspacePage`, `AiHealthPage` `.ah-wrap`, `SettingsPage` body column, Diagnostics page body). Chat does not use it (the chat pane has its own hero/docked choreography).

## 3 · States table

| State | Set by | Attribute / class | What moves | Reduced motion |
|---|---|---|---|---|
| rest | — | — | nothing; 0 loops on an idle screen (Home, Settings, idle chat) | same |
| hover | pointer | `:hover` | colour/bg/border `--dur-fast`; whole-clickable hub cards lift 3px `--dur-base`; rows never lift | colour only, instant |
| press | pointer/key | `:active` | buttons only: `scale: .97` `--dur-fast` | none |
| focus | keyboard | `:focus-visible` | `--shadow-focus` appears `--dur-fast` | instant |
| open / closed | user | `data-state=open/closed` (bits-ui), `data-open`, `aria-expanded` | popups pop/zoom/drawer in/out; accordion grid-rows; chevron 180° | instant; Svelte outros skip their delay via `rm()` |
| live | data | `.sturn[data-live]`, `.live-dot`, `.shim`, `[data-essential-motion]` | stream block entrances; one ping; one shim; spinners | entrances off; ping off (solid dot); shim → `ink-muted`; spinner keeps turning |
| settled | turn end | `data-live` removed | fold-to-receipt (D-12, P4b) | instant fold |
| settle window | re-mount | `[data-settled]` for 450ms | nothing replays: all animation durations and delays 0s; loops keep their period | n/a (net already instant) |
| hidden keep-alive | page switch | `.ws-page[data-active="false"]` | everything paused (`animation-play-state: paused`) | same |
| intro | cold boot | `.app[data-intro=veil|handoff]` | splash doors, mark, letters, island handoff; page sections suppressed | splash fades 100ms; intro skipped (`intro.svelte.ts:11`) |
| loading | data | `aria-busy`, `.skel-bones` | bones fade in after 300ms; out 360 + content reveal 600 | bones swap instantly |
| dragging | pointer | `.resizing`, `.is-dragging` | no transitions while the pointer drives (rail resize, toast drag, detent slider) | same |

## 4 · Motion catalogue (every moment in the app)

Legend: f = `--dur-fast` 150 · b = `--dur-base` 300 (pinned 240 until #3) · s = `--dur-slow` 600 · eo = `--ease-out` · sp = `--ease-spring` · lin = linear · RM = reduced motion · "net" = the global net makes it instant · `rm()` = the Svelte wrapper returns `{duration:0}`. "Today" cites 22727d24.

### 4.1 Boot and shell

| # | Moment | Trigger | Property: from → to | Dur | Ease | Delay / stagger | RM | Today | Why this value |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Splash mark settle | boot | opacity 0→1, y 8→0, scale .94→1, blur 8→0px | 560ms | eo | 140ms | splash fades 100ms lin | `SplashOverlay:189-193` (e-p) | brand moment, once (D-26); only the curve changes |
| 2 | Wordmark letters ×4 | boot | opacity 0→1, y 10→0, blur 4→0px | 460ms | eo | `300ms + i×60ms` | net | `:215-220` (55ms) | D-26 stagger 60 = Halo `stagger`; letter blur stays (splash scope, §13 A-6) |
| 3 | Boot readout | boot > 1400ms | opacity 0→1, y 4→0; exit opacity 200 | 400ms | eo | 80ms | net | `:236-237` | only on slow boots; honest progress |
| 4 | Readout bar | stage progress | `scaleX` via rAF smoother | per frame | — | — | jumps to target | `:47-63` | data-following, compositor-only |
| 5 | Boot stage line | stage change | opacity 0→1, y 3→0 | 260ms | eo | — | net | `:268-269` | splash scope literal |
| 6 | Boot caret | — | **killed** | — | — | — | — | `:275-280` | D-13/D-26; M1 hates blinking stream carets |
| 7 | Doors part | ready ∧ ≥900ms | panels translateX 0→∓101% | 520ms | `--ease-door` | 140ms | splash fades 100ms | `:149` | the one named non-token curve (D-26) |
| 8 | Lockup exit | exit | opacity 1→0 (**blur 3px removed**) | 240ms | eo | — | net | `:177-184` | D-15: no exit blur |
| 9 | Island handoff | `data-intro=handoff` | `.main` opacity 0→1, y 10→0 (was 12); sidebar island x −16→0, opacity | s 600 | eo | 60ms | skipped (`intro.svelte.ts:11`) | `AppShell:494-501`, `Sidebar:238-243` | three rises only (12 → `--rise-page`); `intro` settles at 700ms ≥ 600+60 (§7 coupling) |
| 10 | Page enter, first open | workspace mounts (`WorkspaceShell:80`) | sections opacity 0→1, y 10→0 | 520 | eo | 60 + 60×i, cap 6 | net | `WorkspaceShell:198-211` (6px, 460, 62) + `:136-148` + `AppShell:483-486` | one orchestrated entrance (M24) replaces 3 stacked layers |
| 11 | Page enter, later | `data-active` → true | `.ws-page` opacity 0→1, y 10→0 | 520 | eo | 60ms | net | `WorkspaceShell:140-148` (400, 3px) | Halo page glide; content area only |
| 12 | Page leave | `data-active` → false | opacity 1→0; transform snaps after | 140 | eo | — | net | `:136-139` | old page must not ghost under the new one |
| 13 | Hidden page loops | inactive | `animation-play-state: paused` | — | — | — | same | none | D-29; no motion off-screen |
| 14 | Workspace loading | chunk loading | static bones after 300ms; hand-off 360/600 | see #130 | eo | 300ms | instant | `WorkspaceShell:160-191` shimmer | D-14 |
| 15 | Sidebar collapse / expand | toggle, Ctrl+B | rail width, `.main` margin-left, `--rail-w` together; island x 0↔−16 + opacity | b | eo | — | net (instant) | `Sidebar:221,274`, `AppShell:417`, `app.css:353` (140 vs 240) | D-19 one move; width stays a layout animation (Halo's collapse is too) but it is one reflow stream, not three |
| 16 | Peek in | hover edge | island opacity 0→1, x −16→0, shadow on; `top` snaps 12→54 while invisible | b | eo | — | net | `Sidebar:227-232,274` | D-19 "transform, not top" |
| 17 | Peek out | leave | opacity/x reverse; `top` snaps back after b | b | eo | `top 0s` delayed b | net | same | no visible jump on exit |
| 18 | Rail resize | drag handle | none while dragging | — | — | — | — | `Sidebar:222` | pointer drives |
| 19 | Nav active marker (P4a) | selection | marker `translateY` to the row | b | eo | — | net | `ConversationList:483-484` barPop 280 | ConsoleX plate pattern; one moving thing instead of a pop per row |
| 20 | Conversation rows, first mount | list first render | opacity 0→1, y 3→0 (`.rise-row`) | b | eo | `min(i,12)×25ms` | net | `ConversationList:479,525` (220ms, 14ms) | Halo table-row cascade; first mount only (D-19), `data-settled` on later re-renders |
| 21 | Row hover actions | hover | opacity 0→1 (x 5px dropped) | f | eo | — | net | `:502,527` actsIn | quieter; no sideways jiggle on every hover |
| 22 | Chat working indicator | chat streaming | 9-dot grid → `.live-dot` ping; elapsed `m:ss` already shown (`.crow-time.busy`, `:494`) | 1.8s | eo | — | solid dot | `:533-543` workPulse | D-13 |
| 23 | Row menu / project switcher | open | `halo-pop-in` (scale .96, y −4) | f | eo | rows `min(i,12)×18` | net | `ConversationList:515,530`, `ProjectSwitcher:284-285` | one popup vocabulary (D-20) |
| 24 | Status bar counts | data | text only | — | — | — | — | — | numbers update quietly (D-16) |
| 25 | `.app` background | theme | **deleted with the gradients (P4a, D-01)** | — | — | — | — | `AppShell:360` 1s | no surface changes colour at runtime |

### 4.2 Overlays

| # | Moment | Trigger | Property: from → to | Dur | Ease | Delay / stagger | RM | Today | Why this value |
|---|---|---|---|---|---|---|---|---|---|
| 30 | Menu / popover / select / context enter | `data-state=open` | opacity 0→1, scale .96→1, y −4→0 (up variant +4) from `--bits-floating-transform-origin` | f | eo | — | net | `recipes.ts:20` (suspected inert, D-28 #1) | Halo CSS value (00 "Halo popup speed") |
| 31 | Menu rows | open | opacity 0→1, y −3→0 | 220 | eo | `min(i,12)×18ms` | net | none | Halo menu item |
| 32 | Slash / Mention rows | open | appear with the panel, no stagger | — | — | — | — | `MentionPopover:56-70` | autocomplete must feel immediate (map 09 D13) |
| 33 | Menu leave | `data-state=closed` | opacity 1→0, scale 1→.97 | 140 | eo | — | net | tw-animate zoom-out-95 | Halo |
| 34 | Dialog enter | open | opacity 0→1, scale .94→1, y 8→0 | b | eo | — | net + `rm()` | `UpdateDialog:95,103`, `McpServersDialog:133,141`, `CommandPalette:311,318` (fade 120-130 + scale .97 170) | D-20 `halo-zoom` |
| 35 | Dialog leave | close | opacity 1→0, scale 1→.96 | 200 | eo | — | net + `rm()` | same | Halo |
| 36 | Scrim | open / close | opacity 0→1 / 1→0 | 150 / 200 | eo | — | net | `app.css:612-618` | Halo overlay |
| 37 | Command palette | Ctrl+K | as #34; rows no stagger (results change per keystroke) | b | eo | — | net | `CommandPalette:318` | stagger per keystroke would replay constantly |
| 38 | Output inspector drawer | open / close | x 40→0 + opacity / reverse | b / 200 | eo | — | net | `OutputBlock:267-295` (18px, 180) | D-20 drawer |
| 39 | Toast enter | push | opacity 0→1, y −16→0, scale .96→1 | b | eo | — | net + `rm()` | `ToastHost:52-54` (fly −10 220) | top-centre (X43) so it enters from above |
| 40 | Toast stack reflow | push / dismiss | `--y`/`--s` transform (offset 10px, scale −.05/step, min .86) | b | eo | — | net | `animate:flip` 180 | Halo stack |
| 41 | Toast fan-out | hover / focus | expand to full offsets, timers paused | b | eo | — | net | none | Halo; reading a toast must not race its timer |
| 42 | Toast leave | timer 4000 / close | opacity 1→0, x +32 | 240 | eo | — | net | `out:fly` 140 | Halo |
| 43 | Toast drag | drag right | follows pointer (no transition); throw past 90px or > .6px/ms → x 110% over 240; else spring back b | 240 / b | eo | — | instant | none | Halo dismiss |
| 44 | Update banner | available | rise 10px + opacity in; leave opacity + y −6 | b / 200 | eo | — | net | `UpdateBanner:159` slide 200 | in-content Banner, no reflow of the window (D-20) |
| 45 | Tooltip (P3) | hover / focus | opacity 0→1, y 4→0 | f | eo | 400ms cold, 120ms warm (≤500ms since last hide); hide 0 | instant | none (aria-only) | D-17 calm timing |
| 46 | Notification bell count | count ↑ | count `halo-pop` 1.18 spring; bell swing once | b / 900 | sp / eo | — | none | none | D-16, real event only |
| 47 | Notification popover | open | `halo-pop-in-up`; rows as #31 | f | eo | rows 18ms cap 12 | net | `NotificationCenter:113,125,170` | it is a popover, so it uses popover motion (not Halo's drawer, D-20) |
| 48 | HUDs (plan / agents) | ≥350ms of activity | `halo-pop-in`; leave opacity 140 | f / 140 | eo | debounce 350ms (M23) | net | `ActivityHud:329-330`, `PlanHud:227-228` (y −10, scale .96, b) | M23 owner polish kept; vocabulary unified |

### 4.3 Chat stream and pane

| # | Moment | Trigger | Property: from → to | Dur | Ease | Delay / stagger | RM | Today | Why this value |
|---|---|---|---|---|---|---|---|---|---|
| 50 | Stream block entrance | block born live (`.sturn[data-live]`) | opacity 0→1, y 3→0 | `--dur-stream` 240 | eo | none | off | `stream.css:27` + ~18 uses (e-p, `--dur-base`) | M6: 8px lurched; 3px is the floor that still reads as "arrived"; gated with D-11 |
| 51 | History hydration | tab open / reload | **no entrance**; `[data-settled]` 450ms | — | — | — | — | replays today (D-28 #7 confirms) | D-29 |
| 52 | Word reveal cadence | text delta | cursor advances 1 word / 30ms; backlog > 11 words drains ×(lag/11) | 30ms/word | — | carry remainder | instant text | `Markdown:169-172,216-252` | D-11 numbers kept until board 04 |
| 53 | Word fade | word crosses cursor | opacity 0→1 (**blur 3px removed**) | `--dur-word` 300 (today 500) | eo | negative delay = age×30ms | instant | `:684,692-695` | gated #3; ~11 spans animate, opacity only = compositor-only |
| 54 | Chrome hold | element's first word reveals | opacity 0→1 for code, kbd, headings, li, tr, hr, pre, img | b | eo | — | instant | `:708-727` | M8; unchanged mechanism, curve token |
| 55 | Code block appear | fence closes while live | `.rise-stream` once; **no replay per delta** | 240 | eo | — | off | `:777,1088-1095` code-rise 460 6px .992 | D-11 #5 |
| 56 | Scroll follow | content grows while latched | `scrollTop += Δ·k`, `k = 1 − e^(−dt/τ)`, τ 39ms (≡ .35/frame at 60Hz); snap if Δ > 900px; stop if Δ < 1px | — | exp | — | instant pin | `AssistantPane:169-190` (.35/frame) | D-11 #3: same feel at 60Hz, identical at 120/144/180Hz |
| 57 | Un-latch | user wheel-up / PgUp / ↑ / Home | latch off immediately (no 80px magnet) | — | — | — | same | `:193-202` | D-11 #4 |
| 58 | Jump-to-latest pill | latch lost | `halo-pop-in-up` in / `halo-pop-out` out | f / 140 | eo | — | net | `:1390` jump-in b y 8 | it is a small float, so popup motion |
| 59 | Jump-to-latest click | click / End | follow engine glides to bottom (τ) | ~τ×3 | exp | — | instant | `:252-257` native smooth + 700ms latch | D-11 #4; one scroll engine |
| 60 | Exchange jump (turn rail) | click tick | follow engine to the exchange top − 16px | ~τ×3 | exp | — | instant | `:139-146` native smooth (unguarded) | one engine; RM fixed |
| 61 | Turn rail active tick | scroll | tick `scaleX` + colour | f | eo | — | net | `:1432` width | layout → transform |
| 62 | Live footer verb swap | verb changes | opacity 0→1 (crossfade) | f | eo | — | instant | `stream.css:195-197` verbIn | already calm (22727d24) |
| 63 | Live footer verb shimmer | streaming | `.shim` background-position 100%→−100% | 1.5s | lin | — | static `ink-muted` | `stream.css:329-343` (5 selectors) | M16 owner-approved; **one** per view (D-13) |
| 64 | Live footer dot | streaming | `.live-dot` ping | 1.8s | eo | — | solid | `stream.css:188-189` box-shadow pulse | compositor-only, D-13 |
| 65 | Elapsed seconds | 1s tick | text | — | — | — | — | `StreamTurn:187-193` | D-16 plain tabular text |
| 66 | Live token counter | value change | RollingNumber strip, right-most digit first, opacity floor .6, monotonic | 900 | eo | `min(pos,8)×45ms` | lands instantly | `AnimatedCount:35-76` (420) + `stream.css:366-367` | D-16; sampled on the 1s footer tick (§14 Q3) |
| 67 | Compacting head dot | compacting | `halo-pop` 1.18 once, then ping | b | sp | — | solid | `stream.css:77-88` dotPop 1.55 420 + headRise | Halo pop scale; no sideways head rise |
| 68 | Tool tile live → done | tool result | LiveDot → outcome icon crossfade; failed: danger icon + "Failed" badge fade | f | eo | — | instant | tile border/glyph swaps | D-21 one signal per state |
| 69 | Tool run chip open | click | chip snaps fit-content → 100% width; rows open via accordion A | b | eo | content +60 | instant | `stream.css:260-264` width morph | D-12 / D8; no layout tween |
| 70 | Tool rows while live | row born live | `.rise-stream` | 240 | eo | none | off | `stream.css:273,279,304` workOpen 240-300 | one stream entrance |
| 71 | Disclosures (10 slide sites + workOpen/gcReveal/bodyFadeIn) | toggle | grid-rows 0fr→1fr; content opacity 0→1, y −4→0 | b | eo | content 60ms | instant | `StreamTurn:474`, `StreamShell:211`, `StreamResult:51`, `QueueRail:115`, `ToolChip:479`, `EditDiff:406`, `WriteBatch:89`, `MessageBubble:408,488`, `UpdateBanner:159`; `stream.css:31,33`; `AssistantWelcome:544` | D-12 one accordion |
| 72 | Disclosure chevron | toggle | chevron-down rotate 0→180° | b | eo | — | instant | `.tile-chev` ChevronRight 90° (`StreamTurn:450`, `app.css:592`) | D-12 one glyph |
| 73 | Fold to receipt (P4b) | turn ends | activity body grid-rows 1fr→0fr + opacity 1→0; receipt line opacity 0→1 | b / f | eo | receipt +60ms | instant | hard `{#if}` swap `StreamTurn:412-493` | D-12 same-node fold |
| 74 | Diff rows | diff opens | **no per-row entrance** | — | — | — | — | `EditDiff:571-579` diff-row-in | D-12 "diff rows don't stagger" |
| 75 | Shell output lines | output mounts | **no per-line entrance** (block rise only) | — | — | — | — | `OutputBlock:119-142` `min(i×16,400)` | §14 Q4 |
| 76 | Terminal / exit-plan / dictation caret | live | `halo-caret` steps(1) | 1.06s | steps | — | solid | `StreamShell:248-266`, `stream.css:513-518`, `Composer:2430` | D-13 survivors |
| 77 | Secondary live-text pulses | live | **killed** (prompt pulse, plan-write pulse, dictation polish pulse) | — | — | — | — | `StreamShell:232-236`, `stream.css:513-516`, `Composer:2419,2441-2443` | implied by D-13's survivor list (§14 Q5) |
| 78 | Exit-plan typewriter | card pending | +20 chars / 30ms | — | — | — | full text | `StreamExitPlan:33-42` | Rift signature, unchanged |
| 79 | Ask-user card | born live | `.rise-stream`; picked option marker `halo-pop` 1.18 | 240 / b | eo / sp | — | instant | `StreamAskUser:324,388-390` dotPop | Halo check/radio pop |
| 80 | Compaction bar | estimate tick | `scaleX` | s | eo | — | instant | `stream.css:229-233` (900 e-s) | Halo Stepper fill |
| 81 | Plan progress fill | step done | `scaleX` (was width) | s | eo | — | instant | `stream.css:455`, `PlanHud:283` | layout → transform |
| 82 | User bubble | sent | `.rise-stream` | 240 | eo | — | off | `MessageBubble:626` `.enter` on every mount | born live only (D-29) |
| 83 | Split pane mount | split | opacity 0→1 (no rise) | b | eo | — | net | `AssistantPane:895,916` paneIn y 4 | a pane is a container, not content |
| 84 | Empty pane card | empty pane | `.rise-page` | s | eo | — | net | `:1263-1268` (320, y 6, .985) | empty states rise 10 (D-10) |
| 85 | Context % bar (pane head) | ctx change | `scaleX` | b | eo | — | instant | `:1022` width | layout → transform |
| 86 | Pane live dot | pane streaming | `.live-dot` | 1.8s | eo | — | solid | `:1042-1049` box-shadow | D-13 |
| 87 | Browser dock open / close | toggle | real `width` 0↔W + opacity (native webview tracks bounds) | b | eo | — | `rm()` instant | `AssistantPage:15-22,307` dockSlide 220 cubicOut | D-20: width on purpose, no transform on `.wb-stage` ancestors |
| 88 | Browser page load | navigation | indeterminate bar | 1.3s | eo | — | static 50% bar | `WebBrowserPage:736-750` wb-sweep 1.1s | D-13 |
| 89 | "Page read" badge | AI read | `halo-fade-in` once (shimmer killed) | f | eo | — | instant | `WebBrowserPage:755-770` | D-13 |

### 4.4 Composer

| # | Moment | Trigger | Property: from → to | Dur | Ease | Delay / stagger | RM | Today | Why this value |
|---|---|---|---|---|---|---|---|---|---|
| 90 | Well focus | focus-within | border `line-300`→accent; `--shadow-focus` on | f | eo | — | instant | `Composer:1912-1930` (b, raw curve) | D-22 |
| 91 | Hero ↔ docked FLIP | engage / send / disengage | `translateY(dy)→0`; `will-change: transform` only during | 460 | eo | — | skipped | `AssistantPane:359-384` (raw curve), `:1160` permanent will-change | M19 kept; curve token; layer only while moving |
| 92 | Welcome out / in | engage / disengage | opacity, y 0↔−10 | 200 / b | eo | — | `rm()` | `AssistantPane:332-357` (200 / 220 linear t) | leave = `--dur-leave`, distance = `--rise-page` |
| 93 | Composer bar / hero entrance | home mount | `.rise-page` | s | eo | 200ms | net | `AssistantPane:1148`, `Composer:2032-2045` | empty-state rise |
| 94 | Placeholder | idle | **static** (rotation + blur removed) | — | — | — | — | `Composer:1993-2014` 700 y 8 blur 3 | D-22 "one quiet phrase"; D-15 |
| 95 | Send press | click / Enter | `scale .97` | f | eo | — | none | `.send-btn` transform | D-16 press on buttons |
| 96 | Send ripple | fire | one ring scale .6→2.2, opacity .9→0, once | 600 | eo | — | none | `:1833-1834,2254-2270` two rings 620 | D-13 one ring |
| 97 | Send / stop / queue glyph | state change | icon opacity f + scale .6→1 spring b | f / b | eo / sp | — | instant | `:2279-2290` icon-slot | D-10 spring for flip things |
| 98 | Stop fill | streaming | bg → `danger-soft` | f | eo | — | instant | `.send-btn.stop` | colour only |
| 99 | Queue rail | queued items | rail opens accordion B; items FlipList; new item `.rise-stream`; removed opacity 140 | b / 460 / 240 / 140 | eo | — | `rm()` | `QueueRail:99,115,125-126` (quintOut, 20ms stagger) | D-22 FlipList 460 |
| 100 | Drag-over overlay | file drag | `halo-fade-in`; border solid accent, 7% tint | f | eo | — | instant | `:1875-1904` (scale .94, dot pulse, blur 12) | D-15 / D-22 |
| 101 | Attachment chip | added | `.tag-in` scale .6→1 | b | sp | — | instant | `AttachmentsRow:86` | Halo tag chip |
| 102 | Composer popovers (slash, mention, model, settings, enhance, usage) | open | `halo-pop-in-up`; leave 140 | f / 140 | eo | slash/mention no stagger; settings/model rows 18ms | net | `SlashMenu:134-155`, `MentionPopover:54-70`, `SettingsMenu:510-526`, `EnhanceBar:196-215`, `UsagePanel:280-289` | one upward popover |
| 103 | Settings-menu follow during FLIP | menu open | `ResizeObserver` + FLIP `transitionend` (rAF loop removed) | — | — | — | — | `SettingsMenu:74-81` per-frame rAF | D-22, perf |
| 104 | Effort detent slider | drag / release | free-follow while dragging; snap `translateX` spring on release | b | sp | — | instant | `SettingsMenu:891,932` (width, `left`) | K14 kept; layout → transform |
| 105 | Menu toggles (fast switch) | flip | knob `translateX` | b | sp | — | instant | `SettingsMenu` fast-switch | Halo toggle |
| 106 | Mic waveform | dictating | bars `scaleY` from the meter, 80ms smoothing | 80ms | eo | — | static mid bars | `Composer:2149` height, `stt.svelte.ts:733-752` | data-following, allow-listed literal |
| 107 | Mic idle bars | silent | **killed** (static 30% bars) | — | — | — | — | `Composer:2159` mic-idle loop | D-13 survivors exclude it |
| 108 | Mic ring | recording | `stroke-dashoffset` | 260ms | lin | — | static | `Composer` `.mic-ring rect` | data-following, allow-listed |
| 109 | Enhance in progress | enhancing | Spinner in the wand + static dim text (shimmer, aura, 4 stars, wand pulse, status pulse killed) | 900 | lin | — | spinner kept | `Composer:2333-2390,1613`, `EnhanceBar:305-310` | D-13 |
| 110 | Enhanced text reveal | result | opacity per word (blur 7px removed) | f | eo | 30ms/word | instant | `EnhanceBar:386-401` word-materialize | D-15 |
| 111 | Context ring | ctx change | `stroke-dashoffset` | s | eo | — | instant | `CtxRing:72,80` | Halo ring |
| 112 | Usage / limit bars | open, data | `scaleX` 0→p on first data, then to new p | 700 | eo | — | instant | `UsagePanel:308-312`, `LimitBar:38-68` (width, grow, pulse, sheen) | D-24 Progress; pulse + sheen killed |
| 113 | Model provider check | checking | `.live-dot` ping (was opacity loop) | 1.8s | eo | — | solid | `Composer:2533-2534` | real activity → LiveDot |
| 114 | "Ultra" badge | — | **static** | — | — | — | — | `:2591-2601` ultra-pulse | D-13 kill |

### 4.5 Pages: Home, onboarding, Settings, Health, Diagnostics, dialogs

| # | Moment | Trigger | Property: from → to | Dur | Ease | Delay / stagger | RM | Today | Why this value |
|---|---|---|---|---|---|---|---|---|---|
| 120 | Hub sections | first open | #10 | 520 | eo | 60 + 60i cap 6 | net | — | one entrance |
| 121 | Stat count-up | first data, range change | number old→new (never on 60s/30s ticks) | 700 | easeOutCubic | — | lands | Halo 1100 / ConsoleX 750 | D-16 |
| 122 | Daily bars | mount, range change | `scaleY` 0→v, origin bottom | s | eo | none | instant | `WorkspacePage:920` filter + scaleY 1.03 hover | D-24 |
| 123 | Bar hover | hover | siblings opacity → .45; value label opacity + y 4→0 | b / f | eo | — | instant | filter brightness | D-24 `is-dim` |
| 124 | Hub card hover | hover whole-clickable card | y 0→−3, border `line-100`→`line-200` | b | eo | — | colour only | `WorkspacePage:1111` (f, + `.save-btn` brightness `:1074`) | D-16 lift on hub cards only |
| 125 | Active project | — | static accent dot | — | — | — | — | `WorkspacePage:1090-1092` livePulse | D-13 kill |
| 126 | Model-mix ShareBar | mount | segments `scaleX` | s | eo | — | instant | `.mseg` filter hover `:947-948` | D-24 |
| 127 | Spend sparkline | mount | `halo-draw` stroke-dashoffset 1→0 | s | eo | — | drawn | none | §14 Q6 (Halo 1.2s) |
| 128 | News digest loading | loading | indeterminate bar; orb static | 1.3s | eo | — | static | `NewsFeed:255-260` orb + dl-slide | D-13 |
| 129 | Digest step label | 2.6s rotate | crossfade opacity | f | eo | — | instant | `NewsFeed:72` hard swap | quiet orientation |
| 130 | Skeletons (Home, Health, MCP, workspace, list) | loading > 300ms | bones fade in f after 300; on data: bones out 360 (opacity, blur 4); content reveal 600 (opacity, blur 6, y 6) | 150 / 360 / 600 | eo | 300ms | instant swap | `WorkspacePage:877-880`, `AiHealthPage:1484-1487`, `McpServersDialog:516` (+`:213` delays), `Skeleton.svelte:42-56`, `WorkspaceShell:160-191` | D-14 |
| 131 | Onboarding card | step 1 opens | `.rise-page` | s | eo | — | net | `onboarding.css:29-31` (360, y 10, .99) | empty-state rise |
| 132 | Onboarding step change | next | `.page-sections` cascade | 520 | eo | 60 + 60i cap 6 | net | `onboarding.css:110-115` ob-rise 440 (no stagger) | one entrance vocabulary |
| 133 | Onboarding spine | step | `scaleY` | s | eo | — | instant | `onboarding.css:59` height | layout → transform |
| 134 | Onboarding mini-demos | step opens | play once, hold final frame; "Replay" text button restarts | 5-7s (as today) | as today | as today | final frame | `onboarding.css:480-528` (22 infinite loops), markup `ObStage.svelte` | D-13 |
| 135 | Onboarding connection dot | connecting | `.live-dot` | 1.8s | eo | — | solid | `onboarding.css:323-326` ob-live | D-13 |
| 136 | Settings tabs | tab change | underline `transform` + `width` | b | eo | — | instant | `SettingsPage:1595-1596` (e-p) | Halo tabs bar (width on a 2px bar is cheap) |
| 137 | Settings panel swap | tab change | opacity 0→1 (no rise; cards' own blockIn 8px removed) | f | eo | — | instant | `:1590,1676,1684` | tab change is lateral, not an arrival |
| 138 | Toggle (P3) | flip | track bg/border b; knob `translateX` b spring; knob width f (press stretch) | b / f | eo / sp | — | instant | `app.css:480,488` (knob e-p) | Halo toggle |
| 139 | Segmented (P3) | pick | thumb `transform` + `width`; label colour | b | eo | — | instant | `.seg button` | Halo seg |
| 140 | Slider thumb + bubble | hover / drag | thumb scale spring f; bubble opacity + scale .7→1 | f | sp | — | instant | `app.css:384-392` | Halo slider = grab = spring |
| 141 | Settings search results | typing | `halo-pop-in`; rows `.menu-rows` | f | eo | `min(i,12)×18` | net | `SettingsPage:1792-1797` (22ms, cap 16), `:1624` blur | menu vocabulary, cap 12 |
| 142 | Settings search field | focus | **no width tween** (fixed 210px) | — | — | — | — | `:1617` width 180ms | layout → none |
| 143 | Search jump flash | jump | `.sflash` glow once | 1.6s | eo | — | instant | `:1635-1645` | orientation, kept |
| 144 | Interface rows height | density change | **no transition** | — | — | — | — | `:1762` height | density is a setting, not a moment |
| 145 | Plan gauge / progress | data | `scaleX` | s / f | eo | — | instant | `:1733,1987` | layout → transform |
| 146 | AI Health verdict strip | data | part of `.page-sections` | — | — | — | — | `AiHealthPage:1282-1295` ah-vs-in | one entrance |
| 147 | AI Health hot verdict | hot | static danger Badge | — | — | — | — | `:1293-1294` ah-vs-pulse | D-13 |
| 148 | AI Health steps | data | `.rise-row` | b | eo | `min(i,12)×25` | net | ah-step-in 0.4s x −5 | table-row cascade |
| 149 | AI Health bars | data | `scaleY` / `scaleX` | s | eo | — | instant | `:1394` height, `:1464` width | layout → transform |
| 150 | Diagnostics log rows | append | none (virtualised rows, `ROW_H`) | — | — | — | — | — | high-rate data; motion would cost |
| 151 | Diagnostics vitals | live | one `.live-dot` (dc-breathe + dc-blink killed) | 1.8s | eo | — | solid | `DiagnosticsConsole:405-406,499-500` | D-13 |
| 152 | MCP / Update dialogs | open | #34/#35; progress `scaleX` b | b | eo | — | net | `UpdateDialog:430` width | D-20 |
| 153 | CI badge busy (GitHub) | CI running | `.live-dot` warn tone | 1.8s | eo | — | solid | `app.css:676-678` gh-dot-breathe, `GhPopover:273` gp-breathe | real activity |
| 154 | Status "ok" pill | — | static | — | — | — | — | `app.css:459-465` pill-breathe | D-13 kill |
| 155 | Agent running dots | agent live | `.live-dot` | 1.8s | eo | — | solid | `StreamAgent:237-240,288-290`, `AgentCard:124-133` | D-13 |
| 156 | Permission bar | ask arrives | `.rise-stream`; spinner `data-essential-motion` | 240 | eo | — | spinner turns | `PermissionBar:79,135-145` | live-born block |

### 4.6 Controls everywhere

| # | Moment | Property | Dur | Ease | RM | Today |
|---|---|---|---|---|---|---|
| 160 | Button / icon-button hover | bg, border, colour | f | eo | instant | `app.css:403` (`ease`), `button.svelte:7` (ease-soft) |
| 161 | Button press | `scale: .97` (individual property, so it composes with transforms) | f | eo | none | `app.css:403` 80ms translate, `button.svelte:7` `translate-y-px`, `Sidebar` scale .96 / translateY 1px |
| 162 | Input focus | border, `--shadow-focus` | f | eo | instant | `app.css:431` 80ms `ease` |
| 163 | Row hover (menus, lists, palette) | bg, colour | f | eo | instant | 80-90ms literals (`CommandPalette:465`, `SlashMenu:240,406`) |
| 164 | Card / tile border hover | border-color | b | eo | instant | `app.css:531` (e-s) |
| 165 | Scrollbar thumb hover | bg | f | eo | instant | `app.css:326` |
| 166 | Hover arrow nudge (resume rows) | `translateX` 2px | f | eo (was spring) | none | `AssistantWelcome:522` |
| 167 | Disabled | no motion; opacity .45 | — | — | — | — |

### 4.7 Perf budget

**Compositor-only rule.** Anything that runs longer than one state change (loops, rAF engines, reveals) may animate only `opacity` and `transform`. One declared exception: `.shim` (`background-position` on clipped text), capped at one element per view. One-shot layout animations allowed by name only: sidebar collapse (#15), browser dock width (#87, native bounds sync), grid-rows accordion (#71, #73), Settings tab underline width (#136), Halo segmented thumb width (#139). Everything else in §6 M33-M35 moves to `transform`.

**Concurrency caps** (count of simultaneously running animations, checked with `document.getAnimations().length` via CDP):

| Screen state | Allowed at once | Ceiling |
|---|---|---|
| Idle (Home, Settings, idle chat, Health without load) | 0 loops | 0 infinite; ≤ 6 one-shots during a page entrance |
| Page change | ≤ 6 section rises + 1 page fade | 7 |
| Streaming turn | 1 LiveDot ping + 1 shim + ≤ 11 word fades (`REVEAL_WINDOW`) + ≤ 2 block rises + follow rAF + reveal rAF | ≤ 15 CSS animations, 2 rAF loops |
| Agents/plan running | + 1 ping per HUD row group, + spinners in visible tiles | ≤ 4 spinners on screen |
| Dictation | waveform (rAF-fed transforms) + caret | 1 loop |
| Onboarding step | its demo, once | 0 infinite |

Before P2 (estimate, map 09 §6.5): streaming 6-10 loops incl. box-shadow repaints and ~18 blurred spans; onboarding 22 infinite loops.

**Measurement protocol** (dev build, fixed audit viewport 2048×1104 @ DPR 1.25, D-25):

1. `bash scripts/cdp/c.sh nav chat`, then start the gallery replay (`c.sh eval "location.hash='#/dev/gallery'"` or Ctrl+Alt+G) and press Play (`c.sh act click ".chip.primary"`).
2. `bash scripts/cdp/c.sh sample 20000` (M02) runs `scripts/cdp/raf-sampler.js` for 20 s and returns `{frames, p50, p95, p99, over20, longTasks, maxAnimations}`.
3. Record in the P2 merge note: before (22727d24) vs after (P2 head), same replay, same viewport. Report every number as "measured in dev" (release is faster).
4. Gate: p95 ≤ 8.3ms (120Hz budget), `over20` not higher than baseline, `maxAnimations` ≤ 15 during the streaming replay, `document.getAnimations().length === 0` on an idle Home 2 s after load.
5. Shortlist #7 allows one paid real turn (cheapest model, ~80 words) to sample with the pane's own `pinToBottom`; without it, report "scroll-follow not verified on a real turn".

## 5 · Parts

### 5.1 Kit primitives that consume this system (built in P3, D-09)

| Primitive | Motion it uses | Contract with motion.css |
|---|---|---|
| `ui/Spinner` | `.halo-spin` 900ms | root carries `data-essential-motion`; svg is the spinning child |
| `ui/LiveDot` | `.live-dot` + `::after` ping | props `tone: 'accent'|'warn'|'info'|'idle'` → `--live-tone`; `idle` sets `data-idle` |
| `ui/Progress` | `scaleX` fill `--dur-count`; `.indet-bar` | `value?: number` (0-1); no value = indeterminate |
| `ui/Skeleton` | `.skel-bones`, `.skel-out`, `.rise-reveal` | `loading: boolean`, snippet `children`; `aria-busy` |
| `ui/Accordion` (bits-ui, `forceMount`) | `.acc-panel` carrier A + `.chev` | `data-state=open` → set `data-open` on the panel |
| Dialog / AlertDialog / Popover / DropdownMenu / Select / ContextMenu | `animate-halo-*` utilities (§2.3) | `data-[state]` variants in `recipes.ts` |
| `ui/Toast` (Rift ToastHost restyle) | `.toast-motion` | `--dx/--y/--s/--o/--fade` written by the stack logic |
| `ui/Toggle`, `ui/Segmented`, `ui/Slider` | catalogue #138-140 | knob/thumb transitions live in the primitive's scoped CSS on tokens |
| Tooltip singleton (D-17) | catalogue #45 | `use:tooltip` unchanged |

### 5.2 Rift motion parts built in this spec

**Reactive reduced-motion** (`src/lib/state/accessibility.svelte.ts:101-106`; keeps the exported name, D-33):

```ts
class MotionPref {
  reduced = $state(false);
  constructor() {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    this.reduced = mq.matches;
    mq.addEventListener("change", (e) => { this.reduced = e.matches; });
  }
}
const motionPref = new MotionPref();

/** Reactive inside $derived/$effect/templates; plain read elsewhere. */
export function prefersReducedMotion(): boolean {
  return motionPref.reduced;
}
```

Call sites (all 22 found this session, `grep prefersReducedMotion src`):

| Kind | Sites | Change |
|---|---|---|
| Snapshot `const` read inside an effect/derived | `Markdown.svelte:158` (used `:198,:217,:622`) | `const reduceMotion = $derived(prefersReducedMotion());` — the `:216` effect re-runs on a flip, tearing down the rAF loop |
| Snapshot `const` read in a plain function | `AssistantPane.svelte:167` (used `:171`) | delete the const; call `prefersReducedMotion()` at `:171` |
| Snapshot `const` feeding transition params | `ToastHost:14`, `UpdateBanner:21`, `NotificationCenter:19`, `ToolChip:33`, `EditDiff:27`, `MessageBubble:6`, `ActivityHud:30`, `TurnSummary:10`, `PlanHud:13`, `StreamShell:73`, `WriteBatch:11` | params lose the `reducedMotion ? 0 :` branch; the directive becomes an `rm()`-wrapped transition (M11) |
| Snapshot feeding JS motion | `AnimatedCount:23`, `OutputBlock:42` (`:119` lineDelay), `SplashOverlay:31` | read at use time |
| Already call-time | `AssistantPane:335,352,364`, `SettingsPage:194`, `intro.svelte.ts:11`, `StreamExitPlan:35` | none |

**`rm()`** (`src/lib/motion/rm.ts`):

```ts
import type { TransitionConfig } from "svelte/transition";
import { prefersReducedMotion } from "$lib/state/accessibility.svelte";

type Dir = { direction: "in" | "out" | "both" };
type TransitionFn<P> = (node: Element, params: P, options: Dir) => TransitionConfig | (() => TransitionConfig);

/** Reduced motion → a zero-length transition, so Svelte neither paints nor DELAYS the
 *  unmount (the CSS net zeroes paint only; Svelte's outro timer is JS). */
export function rm<P>(fn: TransitionFn<P>): TransitionFn<P> {
  return (node, params, options) => (prefersReducedMotion() ? { duration: 0 } : fn(node, params, options));
}
```

**`transitions.ts`** (`src/lib/motion/transitions.ts`): Svelte carriers that paint exactly what motion.css paints, for elements that must animate on unmount. Every export is pre-wrapped in `rm()`.

```ts
export const easeOut: (t: number) => number;            // cubic-bezier(.2,.8,.2,1) solved (Newton, 8 iters)
export const easeSpring: (t: number) => number;         // cubic-bezier(.34,1.56,.64,1)
export const rise:    TransitionFn<{ y?: number; duration?: number; delay?: number }>;    // defaults y 3, MS.stream
export const pop:     TransitionFn<{ up?: boolean; duration?: number }>;                   // in: MS.fast; out: MS.popOut
export const zoom:    TransitionFn<{ duration?: number }>;                                  // in: MS.base .94/8px; out: MS.leave .96
export const drawer:  TransitionFn<{ duration?: number }>;                                  // 40px
export const accordion: TransitionFn<{ duration?: number }>;                               // grid-template-rows ${t}fr + content opacity
export const dockWidth: TransitionFn<{ width: number }>;                                   // replaces AssistantPage:15-22 dockSlide; MS.base
```

`accordion` (carrier B) CSS emitted per frame: `display:grid; grid-template-rows:${t}fr;` on the wrapper; the wrapper's single child carries `class="acc-inner"` (`overflow:hidden; min-height:0`). Content opacity follows `max(0, (t·300 − 60)/240)` on the way in (the 60ms content delay of carrier A), linear `t` on the way out.

**`tokens.ts`** (`src/lib/motion/tokens.ts`): `export const MS = { fast: 150, base: 240 /* pinned until #3 */, slow: 600, stagger: 60, page: 520, pageDelay: 60, pageOut: 140, popOut: 140, leave: 200, toastOut: 240, menuItem: 220, staggerMenu: 18, stream: 240, flip: 460, count: 700, roll: 900, skelDelay: 300, skelOut: 360, settle: 450 } as const;` A vitest (`tokens.test.ts`) reads `src/lib/styles/motion.css` and the app.css deviation block as text and fails if any shared value differs (JS and CSS cannot drift).

**`settleGuard`** (`src/lib/actions/settleGuard.ts`, D-29):

```ts
import { MS } from "$lib/motion/tokens";

type GuardArg = { key: unknown; armOnMount: boolean };

/** Marks `node` [data-settled] for MS.settle (450ms) when `key` changes, and on mount
 *  only if `armOnMount` (a mount that hydrates existing content, not a first send). */
export function settleGuard(node: HTMLElement, arg: GuardArg) {
  let t: ReturnType<typeof setTimeout> | undefined;
  let key = arg.key;
  const arm = () => {
    node.dataset.settled = "";
    clearTimeout(t);
    t = setTimeout(() => { delete node.dataset.settled; }, MS.settle);
  };
  if (arg.armOnMount) arm();
  return {
    update(next: GuardArg) { if (next.key !== key) { key = next.key; arm(); } },
    destroy() { clearTimeout(t); },
  };
}
/** Imperative arm for events that are not a key change (history hydration). */
export function settle(node: HTMLElement | undefined) { /* same arm(), own timer map keyed by node */ }
```

Where `data-settled` / `data-live` are set:

| Container | Attribute | Set when | Cleared |
|---|---|---|---|
| `.stream` (`AssistantPane.svelte:714`) | `data-settled` via `use:settleGuard={{ key: tabId, armOnMount: messages.length > 2 }}` | tab switch; pane mount that hydrates history (split, re-parent, tab drag, opening a chat from an empty tab). `.stream` sits in `{:else if !showEmpty}` (`:713`), so it also mounts on a new chat's first send — `armOnMount` is false there (≤ 2 messages), so the first user bubble and first blocks still rise | 450ms timer |
| same | `data-settled` via `settle(scrollEl)` | history hydration: inside the `:220-227` effect, when `messages.length` grows by > 1 while `!streaming` | 450ms timer |
| `.sturn` (`StreamTurn.svelte:326`) | `data-live={streaming ? "" : undefined}` | turn streaming | turn ends (fold, D-12) |
| conversation list root (`ConversationList`) | `data-first` | first mount | 650ms (12×25 + 300 + 50) |
| any `{#key x}` that wraps content with entrances (15 `{#key}` blocks exist; audited per surface port) | `use:settleGuard={x}` on the keyed block's parent | key change | 450ms |
| `.ws-page` (`WorkspaceShell.svelte:84-91`) | `data-active="false"` → loops paused | page hidden | page shown |

Intended replays are exempt and must not get a guard: the send ripple `{#key fireKey}` (`Composer:687`), the footer verb `{#key}` (`stream.css:182-183`), the splash stage line `{#key stage.label}` (`SplashOverlay:115`).

**rAF engines' reduced-motion end state:**

| Engine | File | Under reduced motion |
|---|---|---|
| Word reveal | `Markdown.svelte:216-252` → `reveal.ts` | no loop; full text, no spans (`:198,:217,:622` already) |
| Scroll follow | `AssistantPane.svelte:169-190` → `follow.ts` | instant pin (`τ = 0`) |
| Jump-to-latest / exchange jump | `:139-146,:252-257` | instant `scrollTop` assignment |
| Composer FLIP | `:359-384` | skipped (`:364` already) |
| Count tween / RollingNumber | `AnimatedCount.svelte:35-76` | set final value, no roll |
| Splash readout smoother | `SplashOverlay.svelte:47-63` | jump to target |
| STT meter | `stt.svelte.ts:733-752` | keeps feeding (it is data); bars have no CSS transition (net) |
| Exit-plan typewriter | `StreamExitPlan.svelte:33-42` | full text (`:35` already) |

### 5.3 Track B parts (streaming engine; D-11 structure)

**`reveal.ts`** (pure; zero Svelte imports so board 04 can inline the same file):

```ts
export type RevealParams = { wordMs: number; catchupWords: number; revealMs: number };
export const WORD_MS = 30;
export const CATCHUP_LAG = 11;          // = ceil(320 / 30)
export const REVEAL_MS = 500;           // today; G2 sets 300
export const REVEAL_DEFAULTS: RevealParams = { wordMs: WORD_MS, catchupWords: CATCHUP_LAG, revealMs: REVEAL_MS };
export const revealWindow = (p: RevealParams) => Math.ceil(p.revealMs / p.wordMs) + 1;

export type RevealCursor = { shown: number; last: number };
/** One rAF step. Returns the next cursor and whether the loop may park. */
export function stepReveal(cur: RevealCursor, now: number, totalWords: number, p = REVEAL_DEFAULTS):
  RevealCursor & { parked: boolean } {
  const target = totalWords > 0 ? totalWords + revealWindow(p) : 0;
  if (cur.shown >= target) return { shown: cur.shown, last: 0, parked: true };
  const last = cur.last || now;
  const raw = Math.floor((now - last) / p.wordMs);
  if (raw <= 0) return { shown: cur.shown, last, parked: false };
  const lag = totalWords - cur.shown;
  const due = lag > p.catchupWords ? Math.ceil((raw * lag) / p.catchupWords) : raw;
  return { shown: Math.min(cur.shown + due, target), last: last + raw * p.wordMs, parked: false }; // B2: carry
}
```

(B1 ships `last: now` to be behaviour-identical; B2 switches to `last + raw * p.wordMs`.)

**`follow.ts`**:

```ts
export const FOLLOW_TAU_MS = 39;   // ≡ 0.35/frame at 60Hz: τ = −16.67 / ln(0.65) = 38.7
export const SNAP_PX = 900;
export const STOP_PX = 1;
/** px to add to scrollTop this frame. τ = 0 → instant pin. */
export function followStep(delta: number, dtMs: number, tauMs = FOLLOW_TAU_MS): number {
  if (Math.abs(delta) < STOP_PX || tauMs <= 0 || delta > SNAP_PX) return delta;
  return delta * (1 - Math.exp(-Math.min(dtMs, 100) / tauMs));
}
```

**DEV motion lab params** (`src/lib/motion/lab.svelte.ts`): a class with `$state` fields `{ wordMs, catchupWords, revealMs, blurPx, followTauMs, durBase, durStream, riseStreamPx }` defaulting to the constants; `Markdown`/`AssistantPane` read `lab.params` only when `import.meta.env.DEV`, else the constants. Setting `durBase`/`durStream`/`riseStreamPx` writes the CSS custom property on `document.documentElement` (DEV only).

**DEV fake-stream harness** (`src/lib/dev/fakeStream.ts`): `startFakeStream(tabId: string, turn: CannedTurn, profile: "steady" | "bursty" | "slow")` pushes text through the tab's real `enqueueText` path (`state/assistant/streaming.ts:393-409`) and tool blocks through the same store mutations the gallery fixtures use (`$lib/dev/galleryFixtures`), so `pinToBottom` runs in the real pane. Exposed as `window.__riftFakeStream` in DEV only; never billed. The canned turns are §9.2's, shared as `src/lib/dev/motionLabTurns.ts`.

## 6 · Ordered commit list

Branches: P2 commits (M) land on `design/halo` in order. Gated commits (G) live on `design/halo-gate-3`, cut after M42, and merge only after the owner answers shortlist #3. Track B (B) lands on `design/halo` at the start of P4b (D-11); it is specified here because board 04 exports into its files. Every commit runs `npm run check` + `npm test` before it is made; extra checks are listed per commit. "Visible" = what the owner would notice.

**Precondition (no commit).** P1 is merged with the motion pins in its deviation block (§13 A-1): `--dur-fast:140ms; --dur-base:240ms; --dur-slow:520ms; --stagger:62ms;` and `--ease-page`/`--ease-soft` still `(.22,1,.36,1)`/`(.4,0,.2,1)`. If P1 instead let Halo values through, skip M07/M08 and note it in the merge message.

| # | Title | Files (path:line) | Exact change | Logic preserved | Risk | Verified by |
|---|---|---|---|---|---|---|
| M01 | `tooling(cdp): media emulation verb` | `scripts/cdp/serve.cjs:1350-1367`, `scripts/cdp/c.sh:437`, `scripts/cdp/README.md:107-124`, `.agents/skills/rift-ui/SKILL.md` (command list) | add `case 'media'` → `cdp('Emulation.setEmulatedMedia', {features:[{name:'prefers-reduced-motion', value: params.value ?? ''}]}, 5000, t)`; c.sh `media) post media "$(jq -nc --arg v "${1:-}" '{value:$v}')" ;;`; document `c.sh media reduce` / `c.sh media ''` | no existing op touched | emulation persists until cleared; README says so | `c.sh media reduce && c.sh eval "matchMedia('(prefers-reduced-motion: reduce)').matches"` → `true`; `c.sh media ''` → `false` |
| M02 | `tooling(cdp): rAF sampler` | new `scripts/cdp/raf-sampler.js`; `c.sh` new `sample)` verb | sampler (§10.3) wrapped as an async IIFE taking `__MS__`; `sample) js="$(sed "s/__MS__/${1:-10000}/" "$HERE/raf-sampler.js")"; post eval "$(jq -nc --arg js "$js" '{js:$js, timeoutMs: 60000}')"` | — | eval timeout: pass `timeoutMs` ≥ ms + 5000 (`serve.cjs:329-332` default 30000) | `c.sh sample 3000` returns frames > 0 |
| M03 | `tooling(check-ui): scan .css, motion ratchets` | `design-system/check-ui.mjs:94-108`, `design-system/ui-baseline.json` | `walk(SRC).filter(p => /\.(svelte|css)$/.test(p))`; add the 5 metrics of §10.2 with `exclude: ["lib/styles/motion.css", "lib/styles/halo/tokens.css"]`; `motion:raw-duration` and `motion:cubic-bezier` skip any line containing `motion-literal:` (the M41 allow-list marker), so both can reach a locked 0; run `--update` | existing metrics unchanged in meaning; their totals rise because `.css` is now scanned (baseline rewritten in the same commit) | if spec 01 already widened the scan, only add the metrics | `npm run check:ui` green; baseline diff shows the new keys + raised `color:*` totals |
| M04 | `motion: add styles/motion.css (inert)` | new `src/lib/styles/motion.css`; `src/app.css:3` | file of §2.2; `@import "./lib/styles/motion.css";` after `tw-animate-css` | nothing consumes the classes yet; Rift `:root` declarations later in `app.css` still win for `--dur-page` etc. | name collision: none (`halo-*` prefix; checked against all 173 `@keyframes`) | `c.sh tour home chat settings ai-health` + `c.sh diff` per surface: 0 px changes; `c.sh inspect` 0 errors |
| M05 | `motion(tokens): Tailwind bridge for keyframes` | `src/app.css:109-111` | add the `@theme inline` keys of §2.3 (`--ease-out`, `--animate-halo-*`); no `--animate-*` wipe (P3) | tw-animate's `--animate-in/out` stay registered for `recipes.ts:20,58,60` | — | `npm run check:ui` (class generation) green |
| M06 | `motion(tokens): --dur-stream for stream entrances` | `src/lib/styles/stream.css:73,90,104,117,185,214,227,262,320,353,451,483,525,546`; `StreamAgent.svelte:204`; `StreamAskUser.svelte:324`; `StreamShell.svelte:227` | `blockIn var(--dur-base)` → `blockIn var(--dur-stream)` (17 sites) | both 240ms today → no visible change; decouples the stream from shortlist #3 | missing a site → it slows to 300 when G1 merges; grep `blockIn var(--dur-base)` must return 0 | grep count 0 |
| M07 | `motion(tokens): Halo fast 150 · slow 600 · stagger 60` | `src/app.css` deviation block (P1 pins) | delete the `--dur-fast`, `--dur-slow`, `--stagger` pin lines | — | visible: bars/ring fill 520→600; hover 140→150 (imperceptible) | `c.sh eval "getComputedStyle(document.documentElement).getPropertyValue('--dur-slow')"` → `600ms` |
| M08 | `motion(tokens): one curve — ease-page/soft → ease-out` | `src/app.css` alias block | `--ease-page: var(--ease-out); --ease-soft: var(--ease-out);` | spring untouched | visible: ~134 sites land softer; this is the most felt token change | gallery replay video before/after attached to the merge note |
| M09 | `motion(tokens): move page/rise/pulse tokens home` | `src/app.css:243-246,250` | delete `--dur-page` (400), `--dur-page-out`, `--dur-rise`, `--pulse-live` there; add alias lines `--dur-rise: var(--dur-slow); --pulse-live: var(--dur-ping);` | motion.css now supplies 520/140/1.8s | visible: page fade 400→520 until M30; live dots 1.6→1.8s | computed-style evals for the four tokens |
| M10 | `a11y(motion): reactive prefersReducedMotion` | `src/lib/state/accessibility.svelte.ts:101-106`; `Markdown.svelte:158`; `AssistantPane.svelte:167,171` | §5.2 class + the two snapshot conversions | same API name; SSR guard kept | `$derived` re-run tears down the reveal loop mid-stream on a flip — intended | vitest: mock `matchMedia`, fire `change`, assert `prefersReducedMotion()` flips; CDP: `c.sh media reduce` mid-replay → text completes instantly |
| M11 | `a11y(motion): rm() + transitions.ts + tokens.ts` | new `src/lib/motion/{rm,transitions,tokens}.ts` + tests; `UpdateDialog.svelte:95,103`; `DiagnosticsConsole.svelte:163,317`; `CommandPalette.svelte:311,318`; `McpServersDialog.svelte:133,141`; `QueueRail.svelte:99,125,126`; `AssistantPage.svelte:15-22,307`; `ToastHost.svelte:14,52-54`; `NotificationCenter.svelte:19,113,125,170`; `UpdateBanner.svelte:21`; `ToolChip.svelte:33`; `EditDiff.svelte:27`; `TurnSummary.svelte:10`; `ActivityHud.svelte:30`; `PlanHud.svelte:13`; `MessageBubble.svelte:6` | wrap every non-slide directive; overlay sites switch to `zoom`/`pop`/`fade` carriers with the catalogue values (#34-#36, #39, #47); `dockSlide` → `dockWidth` 300 `easeOut` | same mount/unmount points; dock still tweens real width | values change (dialogs 170→300 etc.); slides untouched until M38 | `npm test` (parity + bezier tests); `c.sh media reduce` + open palette → no animation, closes instantly |
| M12 | `motion(loops): one Spinner` | delete the 25 rotate keyframes: `app.css:625`, `settings-controls.css:26`, `onboarding.css:362`, `stream.css:34,504`, `AiHealthPage:1317`, `UpdateDialog:557`, `McpServersDialog:327`, `Composer:2131`, `GhPopover:258`, `WebBrowserPage:707`, `WorkspacePage:959,1194`, `ToolChip:626`, `NewsFeed:332`, `SettingsMenu:730`, `UsagePanel:348`, `PermissionBar:136`, `UpdateBanner:259`, `AgentCard:158`, `TodoCard:122`, `AskUserCard:502`, `BlockHeader:108`, `PlanHud:268`, `StreamAskUser:423` | each spinning element: `class="halo-spin"` (or `animation: halo-spin var(--dur-spin) linear infinite`); `data-essential-motion` goes on the spinner element itself (or its own icon-only root whose only descendant is the svg), never on a tile, bar or button wrapper — `[data-essential-motion] *` re-enables every animation inside as an infinite 900ms loop under reduced motion | spinners still spin; PermissionBar's "state indicator" intent (`:141-142`) becomes the global rule | visible: all spinners 900ms | grep `rotate(360deg)` outside motion.css = 0 |
| M13 | `a11y(motion): global reduced-motion net` | `motion.css` net already present → delete `app.css:117-122` (`[data-slot]` rule) | — | bits-ui exits still unmount (≈0ms animations) | a component that relied on `animation:none` landing styles behaves the same: `both` fill lands at end state | `c.sh media reduce` + `c.sh tour home chat settings ai-health`: `c.sh eval "document.getAnimations().filter(a=>a.playState==='running').length"` = number of visible spinners only |
| M14 | `motion(loops): LiveDot ping replaces pulses` | `stream.css:40-44,80,188-189,321,370`; `StreamAskUser.svelte:342`; `AssistantPane.svelte:1042-1049`; `app.css:676-678`; `GhPopover.svelte:273`; `DiagnosticsConsole.svelte:405-406,499-500`; `StreamAgent.svelte:237-240,288-290`; `AgentCard.svelte:124-133`; `onboarding.css:323-326`; `Composer.svelte:2533-2534`; `ConversationList.svelte:533-543` + markup | each live dot → `<span class="live-dot" style="--live-tone:…">`; keyframes deleted; 9-dot grid → one dot | live states unchanged | dot size moves to 8px (Halo) from 6px | grep `box-shadow` inside `@keyframes` = 0; gallery replay: one ping visible per live row |
| M15 | `motion(loops): one shim, on the footer verb` | `stream.css:328-343,372-375` | selector list → `.sfooter .sf-verb` only, using `.shim`; the other four go plain `ink-muted` | stalled/awaiting overrides (`:209-211,247-249`) unchanged | — | gallery: `document.querySelectorAll('.shim').length ≤ 1` during replay |
| M16 | `motion(loops): kill decorative breathing` | `AssistantWelcome.svelte:447-453,470-473`; `Composer.svelte:1895-1904,2159,2591-2601`; `app.css:459-465,623-629` (`.pulse` utility unused: 0 `class="…pulse"` matches); `NewsFeed.svelte:255-256` | delete loops; elements rest at their 100% state | — | owner-visible calm | grep each keyframe name = 0 |
| M17 | `motion(loops): LimitBar/Usage pulse + sheen` | `LimitBar.svelte:45-49,62-64`; `UsagePanel.svelte:312-313` | delete pulse + sheen; hot zone = colour only | hot thresholds unchanged | — | grep `filter: brightness` in keyframes = 0 |
| M18 | `motion(loops): enhance magic → Spinner` | `Composer.svelte:1613` (stars markup), `:2333-2390`; `EnhanceBar.svelte:305-310` | Spinner in the wand; text `ink-muted` static | enhance flow unchanged | — | gallery/composer: `getAnimations().length` while enhancing = 1 |
| M19 | `motion(loops): secondary live-text pulses` | `StreamShell.svelte:232-236`; `stream.css:513-516`; `Composer.svelte:2419,2441-2443` | delete pulses; carets stay (`halo-caret`) | — | §14 Q5 | grep keyframe names = 0 |
| M20 | `motion(loops): static hot verdict + active project` | `AiHealthPage.svelte:1293-1294`; `WorkspacePage.svelte:1090-1092` | delete loops (badge restyle is P4f/P4d) | — | — | grep = 0 |
| M21 | `motion(loops): browser comet → indeterminate, Page-read one-shot` | `WebBrowserPage.svelte:736-750,755-770` | `.wb-progress::before` → `.indet-bar`; delete `wb-shimmer` | loading flag unchanged | — | `c.sh -t browser` load page; bar visible, one animation |
| M22 | `motion(loops): legacy bubble glows` | `MessageBubble.svelte:677-684,703-713` | delete `rail-stream`, `avatar-halo` | — | conflicts with D-21 freeze (§13 A-5): droppable | grep = 0 |
| M23 | `motion(loops): static skeleton + hand-off` | `shell/Skeleton.svelte:42-56`; `WorkspaceShell.svelte:160-191`; `WorkspacePage.svelte:877-880`; `AiHealthPage.svelte:1484-1487`; `McpServersDialog.svelte:213,516` | bones `bg-200` r6 + `.skel-bones`; content mount wraps `.rise-reveal`; outgoing bones `.skel-out` | loading flags unchanged | — | throttle (`c.sh eval` a 1s fetch delay in DEV) → bones appear at 300ms, no sweep |
| M24 | `motion(loops): one send ripple` | `Composer.svelte:1834,2254-2270` | delete `.send-ripple-2` span + rule; ring uses `.ripple-once` | `{#key fireKey}` replay kept | — | visual: one ring |
| M25 | `motion(splash): retune` | `SplashOverlay.svelte:149,177-184,216,275-280` | door curve → `var(--ease-door)`; drop `filter: blur(3px)` from exit; `55ms` → `60ms`; delete caret | stage timings (`:18,:22,:78-80`) unchanged | — | cold boot recorded via `c.sh reload` + shot sequence |
| M26 | `motion(onboarding): demos play once + Replay` | `onboarding.css:480-528`; `onboarding/ObStage.svelte` | `infinite` → `1` + `forwards`; a "Replay" text button toggles a `data-run` key to restart | demo content unchanged | — | open onboarding: `getAnimations()` empty 8s after the step opens |
| M27 | `motion(guard): data-settled replay guard` | new `src/lib/actions/settleGuard.ts`; `AssistantPane.svelte:714,220-227` | `use:settleGuard={{ key: tabId, armOnMount: messages.length > 2 }}` on `.stream`; `settle(scrollEl)` on hydration | scroll restore (`:229-250`) unchanged; a new chat's first send still animates (`.stream` mounts at `:713-714` with ≤ 2 messages) | 450ms window could freeze a new live block's entrance on fast tab-switch-then-send — acceptable (lands instantly) | CDP: switch tabs on a 50-turn chat → `getAnimations().length` ≤ 2 within 100ms; new chat first send → user bubble rises |
| M28 | `motion(guard): stream entrances only when live` | `StreamTurn.svelte:326`; `stream.css` the 14 entrance rules (M06 list) + `StreamAgent:204`, `StreamAskUser:324`, `StreamShell:227`; `MessageBubble.svelte:626` (user bubble) | `.sturn` gets `data-live`; each entrance rule re-scoped `.sturn[data-live] .x { --rise: var(--rise-stream); animation: halo-rise var(--dur-stream) var(--ease-out) both; }`; delete `stream.css:27` `blockIn` | live behaviour identical (3px, 240) | P0 spike #7 decides how much this changes; if hydration never replayed, this is defence only | reload a long chat: 0 entrance animations; gallery replay: blocks still rise |
| M29 | `motion(stream): kill code-rise replay` | `Markdown.svelte:777,1088-1095` | delete `code-rise` on `pre` and `.shiki-block`; live code fades with the chrome-hold (`:708-727`) | chrome-hold untouched | — | gallery replay: code blocks fade once, no per-delta flicker |
| M30 | `motion(page): one orchestrated page entrance` | `WorkspaceShell.svelte:6-9,47-75,86,128-148,194-211`; `AppShell.svelte:483-486,493`; page roots: `WorkspacePage`, `AiHealthPage` (`.ah-wrap` enter `:1253-1254` removed), `SettingsPage` (`:1590,1676,1684` local blockIn removed), Diagnostics page | §2.4 `.ws-page` CSS; delete `risingIds`, `SETTLE_MS`, `riseHandle`, `ws-child-rise`, `wsIn`; add `class="page-sections"` to each page's section column | keep-alive stack, `inert`, `aria-hidden` unchanged | first-open cascade needs the page's sections as direct children of `.page-sections` | `c.sh nav settings` then `nav home` then `nav settings`: first visit cascades, second glides; `getAnimations()` ≤ 7 |
| M31 | `motion(shell): sidebar one move` | `Sidebar.svelte:221,274` (+ new collapsed rule); `AppShell.svelte:417`; `app.css:353` | all three `var(--dur-base) var(--ease-out)`; drop `top` from `:274`; add §2.4 delayed-top rule | resize disables transitions (`:222`) unchanged | — | `c.sh act key "Ctrl+B"` + `measure .sidebar`: `--rail-w` and margin end together |
| M32 | `motion(shell): conversation rows first mount` | `ConversationList.svelte:477-479,502,523-528` | rows use `.rise-row` only under `[data-first]`; `actsIn` → opacity `--dur-fast`; `barPop` stays until P4a's marker | list keying unchanged | — | new chat: no cascade replay |
| M33 | `motion(perf): fills width → scaleX` | `AssistantPane.svelte:1022`; `stream.css:455`; `PlanHud.svelte:283`; `UpdateBanner.svelte:238`; `UpdateDialog.svelte:430`; `SettingsPage.svelte:1733,1987`; `AiHealthPage.svelte:1464`; `LimitBar.svelte:38,40,48,53,68`; `UsagePanel.svelte:308-312` + each fill's inline `style:width` | fill `width:100%; transform-origin:left; transform:scaleX(p)`; transition `transform` per catalogue; delete `*-grow from{width:0}` | percentages identical | radius on a scaled bar distorts at tiny p: fills use `border-radius: inherit` on the track and `overflow:hidden` | `c.sh measure` each fill: computed width constant, transform varies |
| M34 | `motion(perf): heights → scaleY / none` | `AiHealthPage.svelte:1394`; `Composer.svelte:2149` + `stt.svelte.ts:733-752` writer; `SettingsPage.svelte:1762`; `SlashMenu.svelte:264,416`; `onboarding.css:59` | bars/spine → `scaleY`, origin bottom/top; `.iface-row` transition removed | meter values unchanged | mic bar writer must write `transform` not `height` | dictation: bars move, `height` constant |
| M35 | `motion(perf): knobs, ticks, chip morph` | `SettingsMenu.svelte:891,932`; `AssistantPane.svelte:1432`; `AssistantPage.svelte:425`; `SettingsPage.svelte:1617`; `stream.css:260-264` | `.er-track::after` scaleX; `.er-knob` `left`→`translateX` (spring kept, K14); tick/divider scaleX; search width fixed; `.wline` drop `transition: width` + `interpolate-size` | detent logic unchanged | — | effort slider drag/release feel check (shot sequence) |
| M36 | `motion(perf): FLIP layer only while moving` | `AssistantPane.svelte:359-384,1156-1160` | delete permanent `will-change`; FLIP sets `will-change: transform` with the inline transition `transform var(--dur-flip) var(--ease-out)` and clears it in `clear()` | 540ms safety timer kept (= 460 + 80) | — | `c.sh eval "getComputedStyle(document.querySelector('.composer-host')).willChange"` = `auto` at rest |
| M37 | `motion(perf): zero backdrop-filter` | `BoundaryBlock:113-114`; `ModelSwitchBlock:63-64`; `EnhanceBar:203-204`; `SettingsMenu:519-520`; `SlashMenu:143-144`; `Composer:1886-1887,1921-1922`; `Markdown:1212-1213`; `ActivityHud:326-327`; `PlanHud:224-225`; `ToolChip:730-731`; `SettingsPage:1624`; `onboarding.css:154`; `ConversationList:512`; `ProjectSwitcher:266` | delete both declarations; translucent backgrounds → opaque `var(--bg-100)` (floats) / `var(--bg-200)` (HUDs); `.cm-fade` gradient only | — | look changes on every float (D-15 decision) | `check:ui` `motion:backdrop-filter` = 0 and ratchet set to 0 |
| M38 | `motion(disclosure): one accordion + one chevron` | `StreamTurn.svelte:450,474`; `StreamShell.svelte:211`; `StreamResult.svelte:51`; `QueueRail.svelte:115`; `ToolChip.svelte:479`; `EditDiff.svelte:406`; `WriteBatch.svelte:89,123-131`; `UpdateBanner.svelte:159` (→ banner motion); `stream.css:31-33,273,279,304`; `AssistantWelcome.svelte:544,557` | `{#if}`-gated bodies → `transition:accordion`; always-mounted light bodies → `.acc-panel` + `inert` when closed; ChevronRight 90° → ChevronDown + `.chev` 180° | open/close state variables unchanged | MessageBubble `:408,488` slides are in the frozen tree → left for #6 | toggle each in gallery; `c.sh media reduce` → instant |
| M39 | `motion(sweep): raw curves → tokens` | all `cubic-bezier(` outside motion.css/tokens.css (47 occurrences at 22727d24 incl. definitions) | `(0.22,1,0.36,1)` and `(0.4,0,0.2,1)` → `var(--ease-out)`; `(0.34,1.56,0.64,1)` → `var(--ease-spring)`; `(0.65,0,0.35,1)` → `var(--ease-door)`; Svelte `quintOut`/`cubicOut` imports → `easeOut` | — | — | `check:ui` `motion:cubic-bezier` = 0 |
| M40 | `motion(sweep): bare easing keywords` | every `transition`/`animation` with `ease`, `ease-in-out`, `ease-out` keyword (66 bare `ease` by §10.2 regex) | → `var(--ease-out)`; loops keep `linear` (spin, shim) | — | — | `motion:bare-ease` = 0 |
| M41 | `motion(sweep): duration literals → tokens` | every raw `ms`/`s` in `transition`/`animation` outside motion.css (200 by §10.2 regex) | map: ≤ 100ms press/hover → `--dur-fast`; 120-200 hover → `--dur-fast`, leave → `--dur-leave`; 220-360 → `--dur-base`; 400-700 → `--dur-slow` (or the named token the catalogue gives); loops → named tokens. Allow-list (comment `/* motion-literal: reason */` on the line): splash (`SplashOverlay`), mic meter 80ms, mic ring 260ms, onboarding demo timelines, jump highlights `.sflash`/`.tid-flash` | — | some hovers move 80→150ms (M14 owner item, "hover feel"); listed in merge note | `motion:raw-duration` = allow-list count only |
| M42 | `motion(cleanup): per-selector reduce blocks + lock ratchets` | the reduce blocks in files P2 touched whose only effect is `animation:none` / `transition:none` (e.g. `stream.css:372-384` except the shim colour now in motion.css, `WorkspaceShell:189-192`, `ConversationList:475,528,543`, `Composer:2271-2273`) | delete; keep blocks that change layout or visibility (`Markdown.svelte:696-699`) | net covers them | a block that also changed opacity/colour must be kept | `c.sh media reduce` tour = same shots as before the commit; `check:ui --update` |
| G1 | `motion(gate-3): --dur-base 240 → 300` | `src/app.css` deviation pin line; `src/lib/motion/tokens.ts` `MS.base` | delete the pin; `MS.base: 300` | — | ~120 sites 25% slower (engineering dissent D-10) | owner plays board 04 "Halo calm" first |
| G2 | `motion(gate-3): word fade opacity-only 300` | `reveal.ts` `REVEAL_MS`; `Markdown.svelte:684,692-695` | `REVEAL_MS = 300`; `.md-w { animation: md-word var(--dur-word) var(--ease-out) both; }`; keyframe `from { opacity: 0; }`; Markdown writes `style="--dur-word:{REVEAL_MS}ms"` on `.md` | window recomputed: `ceil(300/30)+1 = 11` | — | gallery replay: ≤ 11 animating spans (was 18) |
| B1 | `motion(stream): extract reveal.ts + follow.ts (identical)` | new `src/lib/motion/{reveal,follow}.ts` + tests; `Markdown.svelte:169-172,216-252`; `AssistantPane.svelte:169-190` | move constants + step functions; components call them; follow keeps per-frame `.35` (`followStepFrame`) | byte-identical cadence (test pins today's sequence at 60Hz) | — | vitest |
| B2 | `motion(stream): reveal carries the remainder` | `reveal.ts` | `last = now` → `last + raw*wordMs` | — | — | vitest: shown(t) at 60/120/144/180Hz within one step of each other over 2s |
| B3 | `motion(stream): follow on frame time` | `follow.ts`; `AssistantPane.svelte` step | `followStep(delta, dt, FOLLOW_TAU_MS)`; stop < 1px (`:182` was 0.5) | 60Hz feel identical | — | vitest: 90% convergence time equal ±1 frame across rates |
| B4 | `motion(stream): immediate un-latch on user scroll-up` | `AssistantPane.svelte:193-202,714` | `onwheel` with `deltaY < 0`, and keydown PageUp/ArrowUp/Home on `.stream`, set `stickToBottom = false` before the gap test | echo guard (`pinEcho`) unchanged | — | CDP wheel-up during replay: pill appears at once |
| B5 | `motion(stream): jumps through the follow engine` | `AssistantPane.svelte:139-146,252-257,158` | jump/exchange set a target and run the same rAF step; delete `glideUntil` and native smooth | latch semantics unchanged | — | jump during replay lands and stays latched |
| B6 | `motion(stream): rAF-throttle the turn rail` | `AssistantPane.svelte:126-138,204` | `updateActiveXch` coalesced to one rAF per frame | same index result | — | sampler during scroll: no long tasks |
| B7 | `dev(motion): fake-stream harness + lab params panel` | new `src/lib/dev/{fakeStream,motionLabTurns}.ts`, `src/lib/motion/lab.svelte.ts`; `src/routes/dev/gallery/+page.svelte:58-97,162` | §5.3; gallery gets sliders for the 8 lab params + preset buttons; remove `.stage{scroll-behavior:smooth}` | DEV only (`import.meta.env.DEV`) | must not ship in release: tree-shaken check | `npm run build` + grep the bundle for `__riftFakeStream` = 0 |

Totals: 42 P2 commits (M01-M42), 2 gated (G1-G2, plus one G-commit per further number the owner changes on board 04), 7 Track B (B1-B7) = 51.

## 7 · Contracts and couplings touched (D-33)

| Contract / coupling | Where | How this spec keeps it |
|---|---|---|
| `prefersReducedMotion()` export name | `accessibility.svelte.ts:103` | name and signature kept; body becomes reactive |
| `.sturn`, `.stream`, `.bubble[data-role=user]`, `.composer-wrap`, `.sidebar`, `.ws-page`, `#card-*` + `.sflash`, `.tid-flash` | stream, pane, shell, settings | no class renamed; only attributes added (`data-live`, `data-settled`, `data-first`) |
| `c.sh nav` labels / CDP scripts | `scripts/cdp/c.sh:471-517` | untouched; new verbs `media`, `sample` only |
| Composer FLIP 540ms safety timer | `AssistantPane.svelte:382` | must stay ≥ `--dur-flip` + 80 → 540 stays; `tokens.ts` `MS.flip` is the source |
| Intro settle timer 700ms | `intro.svelte.ts:10-23` | must be ≥ `--dur-slow` + 60 + 40 = 700; a vitest asserts `INTRO_SETTLE_MS >= MS.slow + 100` |
| WorkspaceShell `SETTLE_MS` 810 | `WorkspaceShell.svelte:47-48` | deleted with `.rising` (M30) |
| Word fade window | `Markdown.svelte:170-171` | JS is the source (`REVEAL_MS`); CSS reads `--dur-word` set inline by Markdown, so they cannot drift |
| `--rail-w` vs rail width vs `.main` margin | `app.css:353`, `Sidebar.svelte:221`, `AppShell.svelte:417` | all three read `--dur-base` |
| Native browser bounds sync during dock slide | `AssistantPage.svelte:11-22`, `WebBrowserPage.svelte:375-377` | dock keeps animating real `width`; no `transform` added on any ancestor of `.wb-stage` |
| HUD debounce 350ms / linger 4s (M23 owner polish) | `ActivityHud`, `PlanHud` | timers untouched; only enter/leave keyframes change |
| Detent slider spring (K14) | `SettingsMenu.svelte:205-208,891,932` | spring kept; property moves `left` → `translateX` |
| `sliderBubble.ts` `thumbW=18` | `actions/sliderBubble.ts` | unchanged here (P3 sizes Halo's 20px thumb) |
| `data-slot` reduced-motion rule | `app.css:117-122` | superseded by the net (M13) |
| Ledger receipt markup | `StreamTurn.svelte:433-492` | only the `slide` directive and chevron change (M38); the fold template is P4b |

## 8 · Deletions (flagged)

Ungated (part of D-10/D-13/D-14/D-15/D-29 decisions; code inside files, no file deleted):

- Keyframes: 25 spinner copies (M12); `streamLivePulse`, `pane-live-pulse`, `livePulse`, `gh-dot-breathe`, `gp-breathe`, `dc-breathe`, `dc-blink`, `sa-dot`, `sa-beat`, `agent-dot`, `ob-live`, `provider-pulse`, `workPulse` (M14); `shim` (→ `halo-shim`, M15); `glyph-breathe`, `ge-breathe`, `ultra-pulse`, `pill-breathe`, `pulse-soft`, `orb-breathe`, `drop-dot-pulse`, `mic-idle` (M16); `limitbar-pulse`, `limitbar-sheen`, `up-pulse`, `limitbar-grow`, `up-grow` (M17, M33); `magic-shimmer`, `magic-aura-pulse`, `magic-twinkle`, `wand-pulse`, `status-pulse` (M18); `sshPromptPulse`, `dictate-polish` (M19); `ah-vs-pulse` (M20); `wb-sweep`, `wb-shimmer` (M21); `sk-sweep`, `sk-breathe`, `ws-load-shimmer`, `skel-shimmer`, `mcp-shimmer`, `ah-skel-pulse` (M23); `caret-blink` (M25); `blockIn` (stream + Settings local), `ws-child-rise`, `wsIn`, `code-rise`, `diff-row-in`, `bodyFadeIn`, `workOpen`, `gcReveal` (M28-M30, M38); `.send-ripple-2` (M24).
- State/code: `risingIds`, `SETTLE_MS`, `riseHandle` (`WorkspaceShell.svelte:6-9,47-75`); `glideUntil` (`AssistantPane.svelte:158`, B5); SettingsMenu rAF follow loop (`SettingsMenu.svelte:74-81`, with P4c); the `.pulse` utility (`app.css:624,627`, no users).
- Per-selector reduced-motion blocks (M42 for files P2 touched; the rest in each P4 surface port — checklist: the 48 files that mention `prefers-reduced-motion`).

Gated by shortlist #6 (not done in this spec):

- `MessageBubble.svelte` legacy glows `rail-stream`/`avatar-halo` (M22 is built as its own droppable commit; if the owner keeps the legacy tree frozen, drop M22 — the net already stills them under reduced motion).
- `MessageBubble.svelte:408,488,1002-1006` disclosures and `node-stream-enter` (`:810-825`): left as-is, removed with the legacy render path.
- `design-system/` motion cards / token mirror (`--dur-*` in `design-system/styles.css`): untouched until #6.

## 9 · Canvas board 04 — Motion lab

### 9.1 Board

| Field | Value |
|---|---|
| Board id | `04` "Motion lab" (D-25), project "Rift · Halo" in Claude Design, Halo installed, Electric blue |
| Frame | 1440 × 900, `bg-000` canvas; one `bg-100` island r16 (D-01) at 12px gutter |
| Layout | left control rail 320px · centre chat pane `--chat-col` 760px × 620px scroll region + docked composer bar 64px · right numbers panel 280px · bottom strip 120px "non-stream feel" |
| Engine | `reveal.ts` and `follow.ts` inlined verbatim (they have no imports); a 60-line scheduler (§9.3) |
| Interactions (Play mode) | Play / Pause / Restart · preset buttons · every lever live (reveal/follow levers apply mid-turn; others on Restart) · A/B compare toggle (splits the pane into two 370px panes, left = locked preset, right = current levers, same scheduler clock) · 0.25× inspect toggle (scales the scheduler clock and every duration together) · "Lock these" button |
| Levers as data-props | the board root declares each lever as `data-prop-<name>="min,max,default,step"` (e.g. `data-prop-word-ms="17,50,30,1"`); the board script builds the control rail from these and reads values back from `dataset`, so the canvas can expose them natively if it supports it, and the in-board rail works in Play mode regardless (§14 Q7) |

Bottom strip (so 240 vs 300 is felt off-stream, D-10 dissent): a button that opens a menu (`halo-pop-in`, rows 18ms), a button that opens a dialog (`halo-zoom-in` at `--dur-base`), a Toggle, a Segmented control, an Accordion row with chevron, a toast trigger. All read `--dur-base` from the lever.

### 9.2 Canned turns (exact content)

**Turn P — prose** (user, then assistant, 112 words):

> User: Why does my Vite dev server reload twice when I save a Svelte file?
>
> Two reloads on one save usually means two watchers saw the change. The first is Vite's own file watcher, which triggers hot module replacement for the component. The second often comes from an editor that writes a temporary file and then renames it over the original, so the watcher sees a create and a change in quick succession. Check whether your editor has a "safe write" or "atomic save" option and turn it off. If the double reload persists, run Vite with `--debug hmr` and look for two update lines with different timestamps. That tells you whether the duplicate comes from the file system or from a plugin.

**Turn C — code-heavy**:

> User: Write a debounced search helper that cancels stale requests.
>
> Here is a small helper that debounces input and cancels the request it replaces:
>
> ```ts
> export function useSearch(fetcher: (q: string, s: AbortSignal) => Promise<string[]>) {
>   let timer: ReturnType<typeof setTimeout> | undefined;
>   let ctrl: AbortController | undefined;
>   return (query: string, onResult: (r: string[]) => void) => {
>     clearTimeout(timer);
>     timer = setTimeout(async () => {
>       ctrl?.abort();
>       ctrl = new AbortController();
>       try {
>         onResult(await fetcher(query, ctrl.signal));
>       } catch (e) {
>         if ((e as Error).name !== "AbortError") throw e;
>       }
>     }, 250);
>   };
> }
> ```
>
> The `AbortController` makes sure a slow response for an old query never overwrites a newer result. The 250ms delay keeps the request count low while you type.

The fence stays plain mono while open and switches to the muted syntax colours when it closes (D-11 #6, D-18), so the owner sees that hand-off too.

**Turn T — tool-heavy** (event times in ms from Play; durations are how long each tile stays live):

| t | Event | Live for | Done state |
|---|---|---|---|
| 0 | user bubble "The theme setting resets after reinstall. Can you fix it?" (rises 3px) | — | — |
| 300 | footer appears: LiveDot + verb "Thinking" (shim) + `0:00` + token roll | until 6100 | folds |
| 300 | thinking row "Looking at how settings are saved" | 900 | "Thought for 1s" |
| 1200 | narration "I'll check where the theme preference is written." (word reveal) | — | — |
| 1700 | Read `src/lib/state/ui-prefs.svelte.ts`; verb "Reading" | 600 | "→ 212 lines" |
| 2400 | Grep `localStorage.setItem`; verb "Searching" | 500 | "→ 6 matches" |
| 3000 | Edit `src/lib/state/ui-prefs.svelte.ts` diff +4 −1; verb "Editing"; diff count rolls | 700 | Badge "edit" |
| 3800 | Shell `npm test -- ui-prefs`; verb "Running tests"; 6 output lines arrive 250ms apart: `✓ persists density (3 ms)` · `✓ persists narration (2 ms)` · `✓ reads legacy theme key once (4 ms)` · `✓ writes rift.ui.theme.v2 (2 ms)` · `Test Files  1 passed (1)` · `Tests  4 passed (4)` | 1800 | "exit 0 · 1.8s" |
| 5700 | answer: "The theme preference was written under an old key, so a fresh install never read it back. I renamed the key to `rift.ui.theme.v2`, kept a one-time read of the old key, and the tests pass." | — | — |
| 6100 | turn ends: activity collapses (grid-rows 300) and the receipt fades in (150, +60): "Applied · 1 file · 6.1s · $0.01" | — | — |

Text arrival profiles (seeded `mulberry32(7)` so every replay is identical):

| Profile | Words per delta | Gap between deltas | Effective rate |
|---|---|---|---|
| steady (default) | 2 | 50ms | 40 w/s |
| bursty | 18 | 450ms (± 120ms jitter) | 40 w/s average, arrives in clumps |
| slow | 1 | 85ms | ~12 w/s |

### 9.3 Timeline engine

One `requestAnimationFrame` loop owns the clock (`now = (performance.now() − t0) × speed`). Each frame: (1) pop scheduled events with `at ≤ now` (append text words to the active say-block's delivered count, mount/settle tiles, swap verbs); (2) `stepReveal(cursor, now, deliveredWords, params)` → toggle `.md-w` / `.md-w-shown` classes exactly as `Markdown.svelte:301-314` does, with `animation-delay = −age × wordMs`; (3) if latched, `scrollTop += followStep(gap, dt, τ)` (or `gap × 0.35` in "today" per-frame mode); (4) a wheel-up un-latches; the jump pill uses the same engine. Restart clears DOM and the seeded RNG.

### 9.4 Levers, presets, exports

| Lever (data-prop) | Min | Max | Default | Step | Maps to |
|---|---|---|---|---|---|
| `word-ms` | 17 | 50 | 30 | 1 | `reveal.ts` `WORD_MS` (was `Markdown.svelte:169`) |
| `catchup-words` | 4 | 16 | 11 | 1 | `reveal.ts` `CATCHUP_LAG` (was `Markdown.svelte:172`) |
| `fade-ms` | 0 | 500 | 300 | 10 | `reveal.ts` `REVEAL_MS` (was `Markdown.svelte:170`) → inline `--dur-word`; window = ceil(fade/word)+1 (`:171`) |
| `blur-px` | 0 | 3 | 0 | 0.5 | `Markdown.svelte:692-695` keyframe: 0 → opacity-only `md-word`; > 0 → `md-word` with `filter: blur(Npx)` |
| `fade-curve` | — | — | `ease-out` | `ease-out` \| `today (.22,1,.36,1)` | `Markdown.svelte:684` easing |
| `follow-mode` | — | — | `glide` | `glide` \| `per-frame .35` \| `instant` | `follow.ts` (`instant` ⇒ τ 0) |
| `follow-tau-ms` | 0 | 150 | 39 | 1 | `follow.ts` `FOLLOW_TAU_MS` (was `AssistantPane.svelte:185` `0.35`) |
| `dur-base` | 240 | 300 | 300 | 60 | `src/app.css` deviation pin (G1) + `tokens.ts` `MS.base` |
| `dur-stream` | 120 | 400 | 240 | 10 | `motion.css` `--dur-stream` + `MS.stream` |
| `rise-stream-px` | 0 | 8 | 3 | 1 | `motion.css` `--rise-stream` |
| `profile` | — | — | `steady` | steady \| bursty \| slow | board only |
| `turn` | — | — | `T` | P \| C \| T | board only |

| Preset | word | catch-up | fade | blur | curve | follow | dur-base | dur-stream | rise |
|---|---|---|---|---|---|---|---|---|---|
| **today 22727d24** | 30 | 11 | 500 | 3 | (.22,1,.36,1) | per-frame .35 | 240 | 240 | 3 |
| default (D-11 recommended) | 30 | 11 | 300 | 0 | ease-out | glide τ 39 | 300 | 240 | 3 |
| **Halo calm** | 30 | 8 | 180 | 0 | ease-out | glide τ 80 | 300 | 300 | 3 |
| **snappy** | 20 | 6 | 150 | 0 | ease-out | instant (τ 0) | 240 | 180 | 2 |

("Halo calm" carries the taste-lens starting points recorded as D-11 dissent; "snappy" is the "instant text with a trailing fade" option D-11 lists, made concrete.)

**Export.** "Lock these" writes a Decision card on the board (timestamped) holding one JSON block:

```json
{ "board": "04", "lockedAt": "<ISO time>", "wordMs": 30, "catchupWords": 11, "fadeMs": 300, "blurPx": 0,
  "fadeCurve": "ease-out", "followMode": "glide", "followTauMs": 39, "durBase": 300, "durStream": 240, "riseStreamPx": 3 }
```

Claude reads the Decision card from the Claude Design project, copies the values into the table below, and makes one G-commit per value that differs from the merged default (D-25 "one number per commit"). Then it replays the same canned turn in WebView2 through the B7 gallery panel with the same numbers, runs `c.sh sample 20000`, and posts both before asking to merge `design/halo-gate-3`.

| Owner decision on board 04 | Exported value | Status |
|---|---|---|
| Opacity-only vs blur | `blurPx` | pending |
| Fade length | `fadeMs` | pending |
| Word speed | `wordMs` | pending |
| Catch-up | `catchupWords` | pending |
| Follow: glide / instant, τ | `followMode`, `followTauMs` | pending |
| 240 vs 300 | `durBase` | pending |
| Stream rise | `durStream`, `riseStreamPx` | pending |

## 10 · Verification + acceptance criteria

### 10.1 Per commit and per phase

- Every commit: `npm run check` (svelte-check + `check:tokens` + `check:ui`) and `npm test` green.
- P2 merge gate (D-27): `c.sh tour home chat settings ai-health` + `c.sh diff` per surface against the P0 baselines, reviewed; `c.sh inspect` 0 console errors; gallery replay; reduced-motion pass (`c.sh media reduce` + tour + `c.sh media ''`); sampler numbers (§4.7).
- Nothing here touches Rust, so `npm run verify` is not required for P2 (it is for P1/P5).

### 10.2 Ratchet metrics (`design-system/check-ui.mjs`, M03)

Scope: `src/**/*.{svelte,css}`, excluding `src/lib/styles/motion.css` and `src/lib/styles/halo/tokens.css`. Baseline = the value `--update` writes at M03; the hand counts below are this session's `grep` over `svelte/css/ts` with a different method and are only orientation (map 09's 115 `infinite` vs 81 here is the same tree counted two ways).

| Metric | Regex | Orientation count @22727d24 | Target after P2 |
|---|---|---|---|
| `motion:cubic-bezier` | `/cubic-bezier\(/g` | 47 | 0 |
| `motion:backdrop-filter` | `/backdrop-filter\s*:\s*(?!none)/g` | 27 (15 unprefixed + 12 `-webkit-`) | 0 (locked) |
| `motion:infinite` | `/animation(?:-iteration-count)?\s*:[^;{}]*\binfinite\b/g` | 81 | ≤ 10 (cursors, onboarding-free; locked by `--update`) |
| `motion:bare-ease` | `/(?:transition|animation)\s*:[^;{}]*(?<![-\w])(?:ease|ease-in|ease-out|ease-in-out)(?![\w-])/g` (lookbehind excludes `var(--ease-out)`; catches `var(--dur-fast) ease`) | 66 bare `ease` after a literal duration only; map 09 §2.2 counts ~104 `ease` + ~130 `ease-out` + 46 `ease-in-out` | 0 |
| `motion:raw-duration` | `/(?:transition|animation)(?:-duration|-delay)?\s*:[^;{}]*?\b\d*\.?\d+m?s\b/g`, lines with `motion-literal:` skipped | 200 literal values | 0 (allow-listed lines are skipped, not counted) |

### 10.3 rAF sampler (`scripts/cdp/raf-sampler.js`, M02)

```js
(async () => {
  const ms = __MS__;
  const frames = [];
  let longTasks = 0;
  let maxAnimations = 0;
  const po = new PerformanceObserver((list) => { longTasks += list.getEntries().length; });
  try { po.observe({ type: "longtask" }); } catch { longTasks = null; } // older engine: report null, not 0
  await new Promise((done) => {
    const start = performance.now();
    let last = start;
    let n = 0;
    const step = (t) => {
      frames.push(t - last);
      last = t;
      if (++n % 10 === 0) maxAnimations = Math.max(maxAnimations, document.getAnimations().length);
      if (t - start < ms) requestAnimationFrame(step); else done();
    };
    requestAnimationFrame(step);
  });
  po.disconnect();
  frames.shift();
  frames.sort((a, b) => a - b);
  const q = (p) => +frames[Math.min(frames.length - 1, Math.floor(p * frames.length))].toFixed(2);
  return { frames: frames.length, p50: q(0.5), p95: q(0.95), p99: q(0.99),
           over20: frames.filter((f) => f > 20).length, longTasks, maxAnimations };
})()
```

### 10.4 CDP acceptance steps (c.sh verbs)

1. Idle calm: `c.sh nav home`, wait 2 s, `c.sh eval "document.getAnimations().length"` → `0`. Same for `nav settings`, idle `nav chat`.
2. Page entrance once: `c.sh nav settings` (first open) → `c.sh eval "document.getAnimations().length"` ≤ 7 within 100ms; `c.sh nav home`, `c.sh nav settings` again → only the `.ws-page` opacity/transform transition (≤ 2).
3. Keep-alive pause: with chat streaming in the gallery, `c.sh nav settings`; `c.sh eval "document.querySelector('.ws-page[data-active=\"false\"] .live-dot')?.getAnimations?.().length ?? 0"` → animations exist but `playState === 'paused'`.
4. Replay guard: open a 50-turn chat (`c.sh act click` on its row), `c.sh eval "document.querySelectorAll('.stream [data-settled], .stream[data-settled]').length"` → 1 within 450ms, 0 after; `getAnimations()` ≤ 2 during.
5. Live entrance: gallery Play → `c.sh eval "document.querySelectorAll('.sturn[data-live]').length"` → 1 during replay, 0 after.
6. Reduced motion: `c.sh media reduce`; repeat 1-5: no entrance animations; only `[data-essential-motion]` spinners run (`document.getAnimations().every(a => a.effect.target.closest('[data-essential-motion]'))` → `true`); text appears whole; jump-to-latest lands instantly; `c.sh media ''`.
7. One shim, one ping: gallery Play → `document.querySelectorAll('.shim').length` ≤ 1; `.live-dot` count equals live rows.
8. Zero blur: `c.sh eval "[...document.querySelectorAll('*')].filter(e => getComputedStyle(e).backdropFilter !== 'none').length"` → 0 on every surface in `tour`.
9. Sidebar one move: `c.sh act key "Ctrl+B"`; `c.sh measure .sidebar` before/after; `c.sh eval "[getComputedStyle(document.documentElement).transitionDuration, getComputedStyle(document.querySelector('.main')).transitionDuration, getComputedStyle(document.querySelector('.side-rail')).transitionDuration]"` → all `0.24s` (`0.3s` after G1).
10. Sampler: §4.7 step 2 numbers recorded before/after.

### 10.5 Screenshots

`c.sh shot` per tour surface at the audit viewport, before (P0 baseline) and after; `c.sh shot-sel ".sfooter"` during replay; `c.sh shot-sel ".jump-latest"`; reduced-motion pair of the chat surface. Expected diffs only: opaque floats (M37), 8px live dots (M14), static welcome mark (M16), skeleton bones (M23).

### 10.6 Counts that must go down

| Count | Before (22727d24) | After P2 |
|---|---|---|
| `@keyframes` definitions | 173 (46 files) | ≤ 70 (motion.css ~30 + splash + onboarding demos + per-surface leftovers until P4) |
| rotate-360 spinner keyframes | 25 | 1 |
| `box-shadow` keyframe pulses | ≥ 20 | 0 |
| Single-line `infinite` declarations | 81 | ≤ 10 |
| `backdrop-filter` declarations | 27 | 0 |
| raw `cubic-bezier(` outside token files | 47 | 0 |
| Unguarded Svelte transition directives | 19 in 10 files | 0 |
| Concurrent blurred word spans while streaming | ~18 | 18 until G2, then 0 blurred / ≤ 11 opacity |
| Loops on an idle screen | 1-3 (welcome breathe, ok-pill, CI dot) | 0 |
| Files with a `prefers-reduced-motion` block | 48 | ≤ 40 after M42 (→ ~2 after the P4 ports: motion.css + Markdown) |

### 10.7 Acceptance checklist

- [ ] Every keyframe name in use resolves to motion.css, the splash, or onboarding demos (grep `@keyframes` list reviewed).
- [ ] `check:ui` motion ratchets at target and locked with `--update`.
- [ ] §10.4 steps 1-10 pass; numbers pasted into the P2 merge note.
- [ ] Gallery replay: no visible re-entrance on tab switch; one shim; one ping; tool rows rise once.
- [ ] `c.sh inspect` 0 console errors on every tour surface, normal and reduced.
- [ ] G1/G2 exist on `design/halo-gate-3`, unmerged, each revertable alone.
- [ ] Board 04 built, plays all four presets, exports the JSON block.

## 11 · Risks + mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| P1's vendored `tokens.css` silently retunes motion (17-name collision, D-08) before P2 | high if unplanned | §13 A-1 pins in P1; P2 precondition check; M07/M08/G1 each remove one pin |
| The global net breaks a flow waiting on `animationend`/`transitionend` | low | only `AssistantPane.svelte:378-382` listens (verified); it skips under RM (`:364`) and has a 540ms timer |
| bits-ui presence waits for exit animations that `!important` 0.01ms ends instantly | low | that is the intended behaviour (immediate unmount); `[data-slot]` rule deleted only after the net is live |
| `[data-settled]` swallows a legit entrance (send right after a tab switch) | medium | content lands instantly (no flash); window is 450ms; loops exempt |
| Owner dislikes the softer curve or 150ms hovers | medium | M08 and M41's hover changes are separate commits; revert one |
| `scaleX` fills distort rounded ends at small values | medium | track `overflow:hidden` + fill `border-radius: inherit`; checked in M33 shots |
| `var(--rise)` inside keyframes unsupported | very low (Chromium supports custom properties in keyframes) | P0 shot of one `.rise-page`; fallback: three explicit keyframes |
| Grid-rows accordion jank on very tall receipts | medium | carrier B animates the wrapper only while open/closing; heavy content stays `{#if}`-gated |
| Removing `.wline` width morph reads abrupt | low | D-12/D8 decided "snap"; board 02 shows it |
| LiveDot 6→8px nudges stream layout | low | 2px; reviewed in `c.sh diff` |
| B7 harness leaks into release | low | DEV gate + bundle grep in B7 |
| Board 04 lever mechanism (`data-props`) not what the canvas supports | medium | in-board control rail works in Play mode regardless (§14 Q7) |
| Hand counts disagree with the ratchet | certain | baselines come only from `--update` (§10.2) |

## 12 · Effort + dependencies

| Block | Size | Sessions | Depends on |
|---|---|---|---|
| M01-M05 tooling + motion home | S | 0.5 | P1 merged |
| M06-M13 tokens + reduced motion | M | 1 | M01-M05 |
| M14-M26 loops kill list | M | 1 | M12-M13 |
| M27-M32 guard, page, sidebar, rows | M | 1 | M13; P0 spike #7 result |
| M33-M42 sweep + cleanup | L | 1.5 | all above |
| G1-G2 gated | S | 0.25 | board 04 verdict |
| B1-B7 Track B | M | 1 | P2 merged; lands at P4b start |
| Board 04 build | M | 1 | Halo installed in the Claude Design project; B1 (to inline the same `reveal.ts`/`follow.ts`) — or build first with today's code and swap in B1's file |

Total ≈ 7 sessions (P2 itself ≈ 5, L). Unblocks: P3 overlay recipes (keyframes + bridge), P4b (guard, Track B), every surface port (one motion vocabulary).

## 13 · Amendments proposed to 00-decisions

- **A-1 (D-08 vs shortlist #3, D-27).** D-08 says Halo wins the motion collisions in P1, but shortlist #3 keeps 300ms unmerged and D-27 describes P1 as "recolour and refont; no layout change". Proposal: P1's deviation block pins the colliding motion tokens at today's values (`--dur-fast 140`, `--dur-base 240`, `--dur-slow 520`, `--stagger 62`, and leaves `--ease-page`/`--ease-soft` as today's curves); P2 removes the pins one commit each (M07, M08) and `--dur-base` only in G1. Reason: keeps every felt motion change in its own revertable P2 commit.
- **A-2 (D-10 rise list).** D-10 names three rises; the catalogue also uses a dialog 8px and toast 16px offset and a −6px banner leave. These are Halo pattern geometry (zoom/toast/banner), not "rises", so they live inside their keyframes, not as tokens. The intro handoff 12px becomes `--rise-page` 10px.
- **A-3 (D-13 net).** Add `animation-delay: 0s !important` and `transition-delay: 0s !important` to Halo's net: Rift has stagger delays up to 360ms (pages) and 300ms (menus); without this, reduced-motion users wait through invisible delays with `both` fill.
- **A-4 (D-12 "one accordion").** One look, two carriers: carrier A (always mounted, CSS grid-rows, `inert` when closed) for light content; carrier B (Svelte `accordion` transition emitting the same grid-rows) for `{#if}`-gated heavy content such as the receipt body (`StreamTurn.svelte:473-489`), so settled turns do not mount every tool tile of every turn.
- **A-5 (D-13 kill list vs D-21 freeze).** D-13 kills the MessageBubble rail/avatar glows; D-21 freezes that tree until shortlist #6. Proposal: M22 is its own droppable commit; if #6 is "no", drop M22 (the net stills them under reduced motion either way).
- **A-6 (D-15 blur scope).** D-15 allows blur for "the splash mark settle"; the splash wordmark letters also blur 4px → 0 (`SplashOverlay.svelte:219`). Treat all splash blur as splash scope (once per boot), not a defect.
- **A-7 (D-29 guard).** Zero `animation-delay` as well as duration under `[data-settled]` (ConsoleX `51f4d02` does both, `Claude.svelte:1344-1349`); real-activity loops are re-declared so a spinner does not freeze for 450ms.
- **A-8 (D-11 ownership).** Track B (reveal/follow extraction, D-11 structure #1-#4) is specified here because board 04 exports into those files; it still lands at the start of P4b as D-11 says.
- **A-9 (tooling).** D-25's "reduced motion via CDP emulation" needs a `c.sh media` verb (none exists: `serve.cjs:1350-1367` has no media op) and D-08's "scan `.css` too" needs `check-ui.mjs:108` widened; both added as P2 tooling commits M01, M03.

Map / register facts corrected while verifying (this session):

- `backdrop-filter`: 15 unprefixed non-`none` declarations + 12 `-webkit-` twins, not 16 (map 09 headline).
- `infinite` count depends on method: 81 single-line declarations by regex vs map 09's 115; ratchet baselines come from `--update` only.
- Svelte transitions lacking a guard: 19 directives in 10 files (D-13 says "10 transitions"; it is 10 files).
- The `--rail-w` transition is at `app.css:353` (map 09 A5 cites 347-354; the declaration line is 353).
- The browser comet lives at `WebBrowserPage.svelte:736-750` (D-13 cites `:735`).
- `check-ui.mjs:108` scans `.svelte` only today; the CDP bridge has no media-emulation op.
- The live chat elapsed timer the D-13 "9-dot grid … gets an `m:ss` timer" asks for already exists as `.crow-time.busy` (`ConversationList.svelte:494`).
- `blockIn` has 17 users on `--dur-base` (14 in `stream.css` incl. `.snarr` `:90`, + 3 components), plus Settings' separate local 8px `blockIn` (`SettingsPage.svelte:1590,1676`).
- Rift's global `@keyframes enter` (`app.css:634`) shadows `tw-animate-css`'s `enter` (the frames behind `animate-in`), so primitive enters cannot be running tw-animate's fade/zoom today; input for D-28 spike #1.
- `.stream` is conditionally rendered (`AssistantPane.svelte:713-714`, `{:else if !showEmpty}`), so it mounts on a new chat's first send; any mount-time guard must exclude that case.

## 14 · Open questions not settled by the register (default in bold)

1. **Q1 Hover feel 80-120ms → 150ms** (map 12 M14 left "hover feel" unrecorded). Default: **150 everywhere, its own commit (M41)**.
2. **Q2 Settings tab change**: fade or rise? Default: **opacity-only 150ms (lateral move, not an arrival)**.
3. **Q3 RollingNumber cadence** on the live token counter (D-16 900ms roll) when tokens update many times a second. Default: **the rolled value is sampled on the existing 1s footer tick (`StreamTurn.svelte:187-193`), so it rolls at most once a second**.
4. **Q4 Shell output per-line stagger** (`OutputBlock.svelte:119`, 16ms, cap 400). Default: **remove; the block's own rise is enough and a terminal prints, it does not cascade**.
5. **Q5 Secondary live-text pulses** (prompt pulse, plan-write pulse, dictation polish pulse) are neither D-13 survivors nor named kills. Default: **kill (Halo strict); carets stay**.
6. **Q6 Sparkline draw length**: Halo `.halo-draw` is 1.2s. Default: **`--dur-slow` 600ms (no new exception)**.
7. **Q7 Canvas lever mechanism**: no source in this repo or the Halo docs defines "data-props". Default: **`data-prop-*` attributes on the board root + an in-board control rail that works in Play mode**.
8. **Q8 Palette rows stagger**: Default: **none (rows change per keystroke)**.
9. **Q9 HUD leave 160ms (M23 owner polish) → 140ms token**. Default: **140 (`--dur-pop-out`); the 350ms debounce and 4s linger stay**.
10. **Q10 Splash literals** (460/400/260ms, 140ms delay) are not tokens. Default: **keep, marked `motion-literal: splash` (D-26 "keep, retuned")**.
