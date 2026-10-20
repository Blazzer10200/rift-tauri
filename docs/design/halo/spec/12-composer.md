---
title: "P4c · Composer"
status: draft
updated: 2026-10-19
phase: P4c
depends_on: [00-decisions, 01-foundation (P1), 02-kit (P3), 03-motion (P2), P4b chat column + AssistantPane FLIP host]
tags: [rift, halo, spec, composer]
---

# P4c · Composer

## 0 · Summary

- Split the 2,604-line `Composer.svelte` into ten leaf components + two pure modules, one pixel-identical commit each (target ≈1.6K lines; the state machines stay put).
- Port the well, bar, Send/Stop/Queue, chips, QueueRail and the six popovers onto P1 tokens, P3 kit primitives and P2 motion classes. D-22 layout (bar below) ships; inside-card is a dormant, fully specified `data-layout` flip.
- Make the shipped "Esc halts" hint true: Esc-to-stop becomes the last Esc layer.
- Kill the leftover magic (placeholder fade, kbd-soup placeholder, oklch literals, rAF follow loop); couple padding, mic ring and one-line height to density vars (D-33).
- No logic changes except three named ones: the one-line-height derivation (commit 13), Esc-to-stop (commit 22) and the SettingsMenu follow mechanism (commit 32).

## 1 · Scope and files

All line refs are at `design/chat-ledger` HEAD `22727d24`. P1/P2/P3/P4b land first and move lines; re-anchor each step by the quoted snippet, not the number.

| File | Lines | P4c role |
|---|---|---|
| `src/lib/components/assistant/Composer.svelte` | 2604 | split + port. script 1-1416 · template 1418-1843 · style 1845-2604 |
| `src/lib/components/assistant/composer/SettingsMenu.svelte` | 970 | panel recipe, follow loop, detent tokens |
| `src/lib/components/assistant/composer/SlashMenu.svelte` | 454 | ComposerPopover + recipes |
| `src/lib/components/assistant/composer/EnhanceBar.svelte` | 403 | ComposerPopover + recipes |
| `src/lib/components/assistant/composer/UsagePanel.svelte` | 353 | ComposerPopover + recipes; also mounted by `shell/StatusBar.svelte:210` |
| `src/lib/components/assistant/composer/QueueRail.svelte` | 206 | `haloFlip`, `.rise-stream` |
| `src/lib/components/assistant/composer/AttachmentsRow.svelte` | 147 | `.tag-in`, IconButton, Banner |
| `src/lib/components/assistant/composer/PermMenu.svelte` | 119 | trigger only (content already bits-ui) |
| `src/lib/components/assistant/composer/MentionPopover.svelte` | 110 | ComposerPopover + recipes |
| `src/lib/components/assistant/composer/CtxRing.svelte` | 84 | IconButton + shared tone helper |
| `src/lib/components/assistant/composer/helpers.ts` | 275 | gains `enhanceContext()`, `ctxTone()` |
| `src/lib/components/assistant/composer/modelMatrix.ts` | 607 | read-only (settings rows, dial stops) |
| `src/lib/components/assistant/GhPopover.svelte` | 285 | → `ui/popover` (mounted `AssistantWelcome:199`, `StatusBar:162`) |
| `src/lib/components/assistant/PermissionBar.svelte` | 147 | Allow/Deny → Button (mounted in the stream, see §11 R6) |
| `src/lib/components/assistant/AssistantPane.svelte` | 1563 | read-only here: host `:802-810`, FLIP `:359-384`, `composerH` `:819` |
| `src/app.css` | — | `.rift-menu*` `:695-736` loses its composer consumers (rule deletion gated, §8) |

**New files**

| File | From |
|---|---|
| `composer/slashCatalog.ts` (+ `.test.ts`) | `Composer.svelte:135-183` (`SlashCmd`, `SLASH_COMMANDS`, `SLASH_BUILTIN_ORDER`, `slashLane`) |
| `composer/layout.ts` | `COMPOSER_MAX_H = 340` (`:212,219,1975`), `COMPOSER_LAYOUT: "below" \| "inside"` |
| `composer/DropOverlay.svelte` | template `:1436-1443`, CSS `:1870-1904` |
| `composer/PromptSuggestion.svelte` | template `:1491-1522`, CSS `:2545-2575` |
| `composer/ComposerPlaceholder.svelte` | template `:1592-1604`, CSS `:1993-2014` |
| `composer/DictationGhost.svelte` | template `:1605-1609`, CSS `:2398-2451` |
| `composer/PermPill.svelte` | template `:1630-1657` |
| `composer/MicButton.svelte` | template `:1679-1729`, CSS `:2110-2193` |
| `composer/ModelPill.svelte` | template `:1757-1786`, CSS `:2470-2541,2585-2601` |
| `composer/SendButton.svelte` | template `:1812-1837`, CSS `:2218-2290` |
| `composer/ComposerBar.svelte` | template `:1624-1840` (hosts the above + attach `:1661-1678`, wand `:1730-1742`, char count `:1743-1751`, SettingsMenu mount `:1788-1798`, CtxRing `:1800-1808`), CSS `:2032-2108` |
| `composer/ComposerPopover.svelte` | shared shell for Slash / Mention / Usage / Enhance (positioning + panel recipe) |

**Not in scope:** the stream (P4b), AssistantPane's FLIP mechanics (P2 M36 + P4b), the welcome hero copy (P4b), `FilePathMenu` and `WebBrowserPage` (other `.rift-menu` consumers).

## 2 · Target design

### 2.1 Tokens used (resolved, dark blue theme)

| Role | Token | compact | regular | comfy |
|---|---|---|---|---|
| Well bg / border | `--bg-100` / `--line-300` | #0e1014 / #5f6774 | same | same |
| Well radius docked / hero | `--radius-md` / `--radius-lg` | 12 / 16 | 12 / 16 | 12 / 16 |
| Well focus | `--accent` border + `--shadow-focus` | `0 0 0 2px bg-100, 0 0 0 4px accent` | same | same |
| Textarea type | `--t-chat-size` / `--t-chat-line` | 14 / 22 | 15 / 24 | 16 / 26 |
| Textarea pad-x | `--control-px` (P3) | 12 | 14 | 16 |
| Textarea pad-y | `--composer-pad-y` (component-local, derived — A-12-1) = `calc((var(--control-h) + 8px - var(--t-chat-line)) / 2)` | 9 | 10 | 13 |
| One-line well height | `--control-h` + 8 | 40 | 44 | 52 |
| Max well height | `COMPOSER_MAX_H` | 340 | 340 | 340 |
| Bar controls | IconButton `size="icon"` ghost | 32 · r8 | 36 · r12 | 44 · r12 |
| Pills (perm, model) | Button `variant="ghost" size="default"` (one bar height, D-22 "32px ghost") | 32 · px12 · r8 | 36 · px14 · r12 | 44 · px16 · r12 |
| Send | Button `variant="primary" size="icon"` | 32 · r8 | 36 · r12 | 44 · r12 |
| Popover | `floatingPanel` + `--z-dropdown` 65 | | | |
| Hero text | `--t-body-size` / `--t-body-line` | 15 / 24 | 15 / 24 | 15 / 24 |
| Column | `--chat-col` | min(760px, 100% − 48px) | same | same |

D-22 says "Send 32×32 r8" — that is the compact rung of `size="icon"`; regular/comfy follow the kit ladder (D-07). No new Send geometry.

### 2.2 Docked, D-22 default (bar below) — compact, 760 col

```
 ┌─ .composer-wrap  (QueueRail above when queued) ─────────────────────────┐
 │ ╭─ .composer-box  bg-100 · 1px line-300 · r12 ─────────────────────────╮ │
 │ │ [chip img ×] [chip file.ts ×]              ← AttachmentsRow .tag-in │ │
 │ │ Ask anything about this project  ← textarea 14/22, pad 9 12, 40 min │ │
 │ ╰──────────────────────────────────────────────────────────────────────╯ │
 │  6px                                                                     │
 │ [⛨ Ask ▾] │ [📎] [🎙] [✦]   1.2k      [● Opus 4.6 · high ▾] [◔] [ ➤ ]  │
 │  pill 32     icon 32 ghost   caption     pill 32 LiveDot+Badge    32 32 │
 └──────────────────────────────────────────────────────────────────────────┘
   .cbar-l (gap 4)                         .cbar-r (gap 4)   Send = primary
```

- Bar height = `--control-h`; `margin-top: 6px`; left group clips first (container ladder kept, named container `composer-bar`).
- Container ladder (from `:2055-2067`, values unchanged): ≤440 hide char count + perm label; ≤380 hide wand; ≤330 hide model label (dot + chevron remain).
- Streaming: well border `color-mix(in srgb, var(--accent) 45%, var(--line-300))` static (no pulse); Send becomes Stop.

### 2.3 Hero (empty pane, not engaged)

```
            What are we working on today?            ← AssistantWelcome (P4b)
 ╭─ .composer-box  r16 · bg-100 · line-300 ─────────────────────────────╮
 │ What are we working on today?     ← one quiet phrase, t-body 15/24   │
 │                                                                      │
 ╰──────────────────────────────────────────────────────────────────────╯
   (no bar: `{#if !hero}` at :1624 unchanged — bar materialises mid-FLIP)
```

### 2.4 Inside-card alternative (dormant, `data-layout="inside"`)

```
 ╭─ .composer  bg-100 · line-300 · r16 · pad 0 ───────────────────────────╮
 │ [chip ×] [chip ×]                                                      │
 │ Ask anything about this project   pad (pad-y) (control-px), bottom 4px │
 │                                                                        │
 │ [⛨ Ask ▾] [📎] [🎙] [✦]  1.2k          [● Opus 4.6 · high ▾] [◔] [ ➤ ] │
 ╰─ bar inside: pad 0 6px 6px · no top border · same children ───────────╯
```

- Same markup; chrome (bg, border, radius, focus ring) moves from `.composer-box` to `.composer`; `.composer-box` becomes transparent, border 0.
- Card radius `--radius-lg` (16) docked and hero; Send stays `size="icon"`.
- Bar renders in hero too (`{#if !hero || layout === "inside"}`), so hero ↔ docked is a pure FLIP with no materialise step.
- Precedent: ConsoleX `panel/src/views/Claude.svelte:965-1127` (markup), `:2291-2600` (CSS: `.cbox` r16, `.ctools` pad 6 8 8, `.tb` 30, `.send` 34 r10) — P4c uses kit sizes instead of those literals.
- Flip cost: one constant + the CSS block already present → ≈1 day with board review (shortlist #4 said 1-2).

### 2.5 Send states

| State | Look | Glyph | Tooltip |
|---|---|---|---|
| idle (nothing to send) | ghost: transparent, `ink-faint` glyph, disabled | Send | "Send" · Enter |
| ready | primary: accent fill, `bg-000` glyph; hover `--glow-strong` ring; press .97 | Send | "Send" · Enter |
| queue (streaming + draft) | primary | ListPlus (Q1) | "Queue for next turn" · Enter |
| stop (streaming, empty draft) | `--danger-soft` fill, `--danger` glyph | Square filled | "Stop" · Esc |
| busy (firing) | primary, `aria-busy` | Send | — |

### 2.6 Drag-over

Overlay covers `.composer-shell`: `border: 1.5px solid var(--accent)`, `background: color-mix(in srgb, var(--accent) 7%, var(--bg-100))`, radius = well radius, centred `t-small` `ink` "Drop files to attach" with a 16px Paperclip. No scale, no blur, no dot.

## 3 · States

| Element | State | Visual | Trigger / source |
|---|---|---|---|
| Well | rest | bg-100 · line-300 | — |
| | hover (not focused) | bg `--bg-200`, as `fieldInput` `hover:not-disabled:bg-surface-hover` (02 §Input) | pointer |
| | focus-within | accent border + `--shadow-focus` | `:focus-within` |
| | streaming | static accent-mix border (§2.2) | `class:streaming` `:1487` |
| | multiline | bar unaffected; inner row `align-items: flex-end` | `multiline` (commit 13 derivation) |
| | at max | textarea scrolls; no fade | `atMaxHeight` `:219` |
| | drag-over | §2.6 overlay | `onDragOverShell` `:1373` |
| | dictating | DictationGhost over textarea, caret `.halo-caret` | `stt` |
| | enhancing | text `ink-muted`, wand shows Spinner (P2 M18) | `enhancing` |
| | disabled (auth gate) | placeholder states the gate; Send idle | `canFire` `:1340` |
| Placeholder | per mode | one phrase (§6 commit 15 copy) | `:1592-1604` |
| Send | §2.5 | | `mode` `:1321-1343` |
| Pills | rest / hover bg-200 / open (`aria-expanded`) accent text + chevron 180° | kit ghost | |
| Mic | idle / recording (ring + waveform) / transcribing (Spinner) / error (danger glyph) | | `:1679-1729` |
| Chips | in (`.tag-in`) / hover border `--line-200` / remove | | AttachmentsRow `:31,48` |
| Attach error | Banner danger, dismissible | | AttachmentsRow `:65-68` |
| QueueRail | closed / open (accordion) / editing (`[data-qedit]`) | | QueueRail `:99-126` |
| CtxRing | ok (<75) accent / warn (≥75) / danger (≥90) | | `CtxRing:27` |
| Popovers | closed / opening 150 / open / closing 140 | | §4 |
| Char count | rest `ink-faint` caption / warn `--warn` | | `:1743-1751`, `.char-count.warn` `:2211` |

## 4 · Motion

All values are P2 tokens/classes. "rm" = reduced-motion behaviour. Rows map to `03-motion.md` §4.4 #90-114; P2 commits already in place are noted, P4c does not redo them.

| Element | Trigger | Property | From → to | Duration | Easing | Delay | rm | Owner |
|---|---|---|---|---|---|---|---|---|
| Well border/ring (#90) | focus-within | border-color, box-shadow | line-300 → accent; none → `--shadow-focus` | `--dur-fast` 150 | `--ease-out` | — | instant | P4c c14 |
| Hero ↔ docked (#91) | engage / send / disengage | transform | translateY(dy) → 0 | `--dur-flip` 460 | `--ease-out` | — | skipped | P2 M36 + AP; P4c only consumes |
| Bar materialise (#93) | docked mount | `.rise-page` | opacity 0, y 6 → 1, 0 | `--dur-slow` | `--ease-out` | 200ms | net only | P4c c38 (Q4) |
| Placeholder (#94) | mode change | none (static) | — | — | — | — | — | P4c c15 |
| Send press (#95) | pointerdown | transform | scale 1 → .97 | 150 | ease-out | — | none | kit Button |
| Send hover | hover (ready) | box-shadow | none → `0 0 0 3px var(--glow-strong)` | 150 | ease-out | — | instant | P4c c20 |
| Send ripple (#96) | fire | `.ripple-once` | scale .6 → 2.2, opacity .9 → 0 | `--dur-ripple` 600 | ease-out | — | none | P2 M24; P4c c20 sets radius `var(--control-r)` |
| Glyph swap (#97) | mode change | opacity; transform | 0 → 1 (150); scale .6 → 1 (`--dur-base` 240) | 150 / 240 | ease-out / `--ease-spring` | — | instant | P4c c20 |
| Stop fill (#98) | streaming | background | accent → danger-soft | 150 | ease-out | — | instant | P4c c20 |
| QueueRail open (#99) | first queued | `transition:accordion` | height 0 → auto | 240 | ease-out | — | rm() | P2 M11/M38 |
| Queue items (#99) | add / reorder / remove | in `.rise-stream`; `animate:haloFlip`; out opacity | y 3 → 0; old → new pos; 1 → 0 | 240 / 460 / `--dur-pop-out` 140 | ease-out | — | rm() / 0 / instant | P4c c26 |
| Drag overlay (#100) | dragenter | `halo-fade-in` | opacity 0 → 1 | 150 | ease-out | — | instant | P4c c16 |
| Chip (#101) | added | `.tag-in` | scale .6 → 1 | 240 | `--ease-spring` | — | instant | P4c c25 |
| Popovers (#102) | open / close | `animate-pop-in-up` / `animate-halo-pop-out` | opacity + y 4 → 0 | 150 / 140 | ease-out | slash/mention none; settings rows 18ms (`--animate-menu-row`) | net | P4c c27-31 |
| Settings follow (#103) | well resize / FLIP end | top/left recompute | — | — | — | — | — | P4c c32 |
| Effort detent (#104) | drag / release | transform translateX | free → snap | 240 | `--ease-spring` | — | instant | P2 M35; P4c tokens only |
| Fast switch (#105) | toggle | knob translateX | 0 → `--toggle-w − --toggle-h` | 240 | spring | — | instant | P4c c31 via kit Switch |
| Mic waveform (#106) | dictating | transform scaleY | meter-driven | 80ms (allow-listed) | ease-out | — | static mid bars | P2 M34 |
| Mic ring (#108) | recording | stroke-dashoffset | countdown | 260ms linear (allow-listed) | linear | — | static | P4c c24 (rx coupling only) |
| Enhance (#109) | enhancing | Spinner in wand | — | 900 linear | — | — | spinner kept | P2 M18 |
| Enhance reveal (#110) | result | opacity per word | 0 → 1 | 150 | ease-out | 30ms/word | instant | P4c c30 |
| CtxRing (#111) | ctx change | stroke-dashoffset | old → new | `--dur-slow` | ease-out | — | instant | P4c c34 |
| Usage bars (#112) | open / data | scaleX | 0 → p | 700 | ease-out | — | instant | P2 M33 |
| Provider check (#113) | checking | `.live-dot` | ping | 1.8s | ease-out | — | solid | P2 M14; P4c c19 swaps to `<LiveDot>` |
| Chevrons | open | `.chev` | rotate 0 → 180 | 240 | ease-out | — | instant | P4c c18/c19 |
| Ultra badge (#114) | — | static | — | — | — | — | — | P2 M16 |

## 5 · Parts

### 5.1 Kit primitives consumed (02-kit)

| Primitive | Where in the composer |
|---|---|
| `Button` (`primary` / `ghost` / `danger` / `soft`; `icon` / `sm`) | SendButton, PermPill, ModelPill, PermissionBar Allow/Deny |
| `IconButton` (`label`, `kbd`, `pressed`, `loading`) | attach, mic, wand, CtxRing, chip remove (`icon-sm`), QueueRail row actions |
| `Badge` | effort tag in ModelPill, "Ultra" |
| `LiveDot` (`tone="accent"`) | provider checking in ModelPill |
| `Spinner` | mic transcribing, wand enhancing (P2), UsagePanel loading (P2) |
| `Banner` (`tone="danger"`, `onDismiss`) | attach error |
| `Kbd` | tooltips and popover footers only — never in the placeholder (K8) |
| `Switch` | Fast toggle in SettingsMenu |
| `Popover` (`ui/popover`, `data-side=top`) | GhPopover |
| `DropdownMenu` (already) | PermMenu content |
| recipes `floatingPanel`, `floatingMotion`, `menuItem`, `menuLabel` | ComposerPopover, SettingsMenu rows (02 §Recipes: the textarea-driven popovers are bespoke listboxes by design) |
| `haloFlip` (`motion/flip.ts`) | QueueRail |
| `tooltip` action + singleton | every bar control |

### 5.2 Rift parts API (new components)

```ts
// composer/layout.ts
export const COMPOSER_MAX_H = 340;
export const COMPOSER_LAYOUT: "below" | "inside" = "below";   // shortlist #4

// composer/slashCatalog.ts
export type SlashCmd = { name: string; desc: string; /* …exact fields from :135-147 */ };
export const SLASH_COMMANDS: SlashCmd[];
export const SLASH_BUILTIN_ORDER: Map<string, number>;
export function slashLane(c: SlashCmd): number;

// composer/helpers.ts (additions)
export function enhanceContext(input: { /* the values :802-820 reads */ }): string | undefined;
export function ctxTone(pct: number): "ok" | "warn" | "danger";   // 75 / 90, from CtxRing:27
```

| Component | Props (all `$props()`; callbacks, no stores passed) | Bindables | Notes |
|---|---|---|---|
| `DropOverlay` | `active: boolean` | — | pure view |
| `PromptSuggestion` | `text`, `onAccept()`, `onDismiss()` | — | inline handler at `:1491-1522` becomes a named fn in Composer |
| `ComposerPlaceholder` | `mode`, `hero`, `dictating`, `hasImage`, `gate?: string` | — | copy table lives here |
| `DictationGhost` | `draft`, `tail`, `pending` | `el` | Composer still scroll-syncs via `ghostEl` |
| `PermPill` | `tab`, `compact` | — | mounts `PermMenu` |
| `MicButton` | `state`, `level`, `remainingPct`, `onToggle()` | — | ring rx from CSS (D-33) |
| `ModelPill` | `model`, `effortLabel`, `checking`, `ultra`, `open`, `onToggle()` | `el` (= `modelWrap` `:1111`) | SettingsMenu anchors to `el` |
| `SendButton` | `mode: "send"\|"queue"\|"stop"`, `ready`, `fireKey`, `onClick()` | — | `{#key fireKey}` ripple replay kept |
| `ComposerBar` | `tab`, `mode`, `canFire`, … (one prop per value the bar template reads) + `children`-free | `modelEl` | named container `composer-bar` |
| `ComposerPopover` | `role?`, `label`, `onClose()`, `children`, `class?` | `el` | absolute in `.composer-shell`, bottom = well top + 8, `floatingPanel` + `animate-pop-in-up`, `data-composer-popover` on root |

The `role` attribute is passed as a literal at each call site (`role="menu"` etc.), so `check:ui overlay:aria-role` keeps counting honestly (§10).

## 6 · Ordered commits

Branch `halo/p4c-composer` off `design/halo` after P4b merges (D-25: merges stay local; no push or `main` merge without the owner). 40 commits on the main line + 1 on a side branch. Shorthand: **C** = `src/lib/components/assistant/Composer.svelte`, **cmp/** = `src/lib/components/assistant/composer/`, `$C` = `scripts/cdp/c.sh`. **V** = `npm run check && npm test` (+ `npm run check:ui`, counts may only drop). **D0** = `bash $C diff ".composer-wrap" composer-<state>-<density>` against the baseline taken before commit 1 → 0% changed pixels at compact, regular, comfy (§10.2).

### Phase A — extraction (pixel-identical, logic untouched)

Every Phase A commit: moves markup + its scoped CSS verbatim, including any `@container` ladder rule (C:2055-2067) that targets the moved element — the leaf carries its own rule from the moment it is extracted; props replace closed-over state 1:1; no class renamed (D-33 hooks stay on the same elements); risk = a scoped rule that targeted across the new boundary → fix with `:global()` only inside the new file, never by restyling. Verify = V + D0 on `idle`, `typing`, `streaming`, `hero`, at full width and at the four ladder widths 460/400/350/320.

Phases B-E: where a commit lists no Logic/Risk line, logic is untouched and the risk is the owner-visible diff, reviewed with `$C diff` per touched state. The only behaviour changes in P4c are commits 13 (multiline threshold), 22 (Esc-to-stop) and 32 (settings follow mechanism).

1. **`refactor(composer): slash catalog to slashCatalog.ts`** — C:135-183 (`SlashCmd`, `SLASH_COMMANDS`, `SLASH_BUILTIN_ORDER`, `slashLane`) → `cmp/slashCatalog.ts`; `cmp/SlashMenu.svelte:14-20` drops its duplicate `SlashCmd` and imports it. Preserved: catalog order, lanes, filters at C:381-414,624. Risk: the two `SlashCmd` shapes differ — union them, never narrow. Verify: V + new `slashCatalog.test.ts` (names unique; `SLASH_BUILTIN_ORDER` covers every builtin; lane order stable).
2. **`refactor(composer): enhanceContext() pure helper`** — C:802-820 `buildEnhanceContext` body → `cmp/helpers.ts` `enhanceContext(input)`; C keeps a 3-line wrapper passing the values it reads. Preserved: output string byte-identical. Verify: V + `helpers.test.ts` cases (empty tab, files only, long history truncation as today).
3. **`refactor(composer): DropOverlay`** — C:1436-1443 + CSS :1870-1904 → `cmp/DropOverlay.svelte` (`active`). Verify: V; D0 + `$C act` drag-enter shot equal.
4. **`refactor(composer): ComposerPlaceholder`** — C:1592-1604 + CSS :1993-2014 → `cmp/ComposerPlaceholder.svelte`. The focus-within dim rule :2008 moves as `:global(.composer-box:focus-within) .placeholder-ghost`. Verify: V + D0 in all five placeholder modes.
5. **`refactor(composer): DictationGhost`** — C:1605-1609 + CSS :2398-2451 → `cmp/DictationGhost.svelte`, `el` bindable to `ghostEl`. Preserved: scroll sync and `dg-tail.pending`. Verify: V + `$C eval` ghost `scrollTop` follows textarea during a scripted dictation (`stt` mock).
6. **`refactor(composer): PromptSuggestion`** — C:1491-1522 + CSS :2545-2575 → `cmp/PromptSuggestion.svelte`; inline handlers become `acceptSuggestion()` / `dismissSuggestion()` in C. Verify: V + D0 with a suggestion visible.
7. **`refactor(composer): PermPill`** — C:1630-1657 → `cmp/PermPill.svelte` (mounts `PermMenu`). `data-composer-popover` stays on PermMenu content (`PermMenu:81`). Verify: V + D0; Shift+Tab still cycles (C:1145).
8. **`refactor(composer): MicButton`** — C:1679-1729 + CSS :2110-2193 → `cmp/MicButton.svelte`. Verify: V + D0 idle; recording shot equal.
9. **`refactor(composer): ModelPill`** — C:1757-1786 + CSS :2470-2541, :2585-2601 → `cmp/ModelPill.svelte`; `el` binds to `modelWrap` (C:1111) so SettingsMenu's anchor is unchanged. Write-only `data-model` (:1764) moves as-is (deleted in 37). Verify: V + D0; settings opens anchored at the same rect (`$C measure '#model-effort-menu'`).
10. **`refactor(composer): SendButton`** — C:1812-1837 + CSS :2218-2290 → `cmp/SendButton.svelte`. `{#key fireKey}` ripple replay stays inside. Verify: V + D0 in send/stop/queue.
11. **`refactor(composer): ComposerBar`** — C:1624-1840 (now mostly child tags) + CSS :2032-2108 → `cmp/ComposerBar.svelte`; `container-type: inline-size` (:2041) gains `container-name: composer-bar`; children's `@container` rules (:2055-2067) become `@container composer-bar (…)` inside their own files. Preserved: `{#if !hero}` gate stays in C around `<ComposerBar>`. Risk: unnamed container queries resolving to a different ancestor → the naming is the fix. Verify: V + D0 at pane widths 460/400/350/320 (`$C eval` resize split).
12. **`refactor(composer): layout constants`** — `cmp/layout.ts` `COMPOSER_MAX_H = 340`, `COMPOSER_LAYOUT = "below"`; C:212,219 use it; CSS :1975 `max-height` reads `--composer-max-h` set inline from the constant (D-33 coupling). Verify: V + D0 + paste 60 lines → well stops at 340.

### Phase B — well and type

13. **`fix(composer): derive one-line height from computed style`** — C:197-224 autosize: `multiline = h > 40` (:216) → `multiline = h > oneLineH + 1`, where `oneLineH` is read once per density change (and on `document.fonts.ready`) from `getComputedStyle(ta)` (`lineHeight + paddingTop + paddingBottom`). Logic: today's textarea (C:1969-1983) is `padding: 9px 12px`, `font-size: var(--fs-md)`, `line-height: 1.5` → one line ≈ 37.5px (12.5px) / 37.5-38 (13px after P1), two lines ≈ 57 → the derived threshold (≈38.5) splits one vs two lines exactly as the literal 40 does → identical behaviour; this must land before 14, because 14 makes the one-line well 44/52 at regular/comfy and the literal would flip every one-line draft into multiline layout. Verify: V + `$C eval` `document.querySelector('.composer-box').classList.contains('multiline')` = false for "hi" at all three densities after 14.
14. **`style(composer): well on the Halo textarea recipe`** — CSS `.composer-box` :1916-1956: bg `--bg-100`, `1px solid var(--line-300)`, radius `--radius-md` (hero `--radius-lg`, replaces :1951 literals), hover bg `--bg-200` when not focused, `:focus-within` :1938 → accent border + `--shadow-focus`, streaming :1956 → static accent mix (§2.2). Textarea :1975: font `--t-chat-size/--t-chat-line`, padding `var(--composer-pad-y) var(--control-px)`; `--composer-pad-y` declared on `.composer` (A-12-1). DictationGhost and ComposerPlaceholder padding/top/left read the same two vars (kills the 1-2px drift, D-22/D-33). Hero textarea: `--t-body-size/--t-body-line`. Verify: V; `$C measure` ghost vs textarea content box offsets equal (±0) at 3 densities; owner-visible diff reviewed.
15. **`style(composer): one quiet placeholder`** — `ComposerPlaceholder`: copy per mode: idle `IDLE_HINT` kept as is ("Ask anything about this project", C:195), hero `What are we working on today?`, question `Answer Rift's question here`, queue `Type to queue a follow-up`, image `Ask about the image…`, dictating `Listening…`. Delete `.ph-k` spans (C:1599,1601) and rule :2007 (K8), `placeholder-fade` keyframes + `.static` rule :2003-2014, stale `placeholderKey` comment :1990-1992. Logic: mode selection unchanged. Risk: owner-visible copy → board 03 frame P. Verify: V; `rg "ph-k|placeholder-fade"` = 0.
16. **`style(composer): drag-over overlay`** — `DropOverlay` per §2.6; `.composer-shell.drag-over .composer-box` :1861 → border accent. Copy "Drop files to attach". Verify: V; `$C act` drag-enter → shot; `getAnimations()` on overlay ≤ 1 (fade).

### Phase C — bar controls

17. **`style(composer): bar controls on IconButton`** — ComposerBar attach (C:1661-1678), wand (:1730-1742) → `<IconButton label="Attach files" kbd=…>` / `label="Improve prompt"` ghost `size="icon"`; `.cbtn` rules :2070-2108 deleted; `.cbar-sep` → kit `Divider` vertical 16px. Tooltips via the kit action (same text). Logic: handlers unchanged. Verify: V; `rg "\.cbtn"` = 0; hover/focus ring visible.
18. **`style(composer): permission pill`** — PermPill trigger → `Button variant="ghost" size="default"` + mode icon + label + `ChevronDown class="chev"` (the svg must be a direct child of the `<button>` that carries `aria-expanded`, since 03's rule is `[aria-expanded="true"] > .chev`); warn modes keep `text-warn` label (C3 amber = cost); hover bg-200. Verify: V; open PermMenu → chevron 180°; `data-composer-popover` present.
19. **`style(composer): model pill`** — ModelPill → `Button variant="ghost" size="default"` (`.chev` a direct child, as in 18): `<LiveDot tone="accent">` while `checking` (replaces P2's `.live-dot` span), model label, effort `<Badge>`, static Sparkles for Ultra (`<Badge>` "Ultra"), `.chev`. Verify: V; provider-check state shows one `.live-dot`.
20. **`feat(composer): Send/Stop/Queue on Button`** — SendButton → `Button variant={ready ? "primary" : "ghost"} size="icon"`; stop: scoped `[data-mode=stop]` bg `--danger-soft`, glyph `--danger`; hover ready → `box-shadow: 0 0 0 3px var(--glow-strong)`; glyph slots: opacity 150 + scale .6→1 `--ease-spring` 240, no rotate; queue glyph `ListPlus` (Q1); ripple `.ripple-once` with `border-radius: var(--control-r)`. Logic: `mode`, `canFire`, `onBtnClick` (C:1321-1343) untouched. Verify: V; `$C shot-sel '.send-btn'` in 4 states; `getAnimations()` after fire = 1 ring.
21. **`style(composer): char count as caption`** — C:1743-1751 → `t-caption` `ink-faint`, warn `--warn`; tabular-nums. Verify: V.
22. **`feat(composer): Esc stops the stream (last Esc layer)`** — C onKey: insert after the slash Esc block (C:1302-1306), before the Enter branch (:1307):
    ```ts
    if (e.key === "Escape" && streaming && !tab?.usageOpen) {
      e.preventDefault();
      void assistant.stop(tabId);
      return;
    }
    ```
    Reached only when no enhance (:1151) / undo (:1156) / mention (:1201) / settings (:1275) / slash (:1302) layer returned first. `isComposing` guard (:1143) already covers IME. Same call as `onBtnClick` (:1325). `ActivityHud.svelte:151` and `CloseConfirm.svelte:55` stop paths untouched. Tooltip :1822 already says Esc — now true (map 04 §7.20). Logic change: yes, this one (D-22). Risk: Esc with a draft typed while streaming now stops (Q2). Verify: V + §10.2 step 5.
23. **`style(composer): mic on tokens; ring rx from CSS`** — MicButton → IconButton shell; ring `rect rx` reads `--control-r` via `getComputedStyle` on mount + density change, viewBox from `--control-h` (was rx 10, viewBox 38×36 literal, C:1679-1729; D-33). Recording: glyph `--danger`, ring stroke `--danger`, thin 1.5px (K11). Waveform scaleY already P2 M34. Verify: V; `$C measure` ring rect equals button rect ±1 at 3 densities.
24. **`style(composer): hero composer`** — C `.composer.hero` CSS: r16, `--t-body`, width `var(--chat-col)` shared with AssistantWelcome (P4b) so hero and welcome share one column. Verify: V; `$C measure '.composer-box'` width = welcome column width.

### Phase D — chips, queue, popovers

25. **`style(composer): attachment chips`** — `cmp/AttachmentsRow.svelte:31,48` chips: `.tag-in`, bg-200, line-200, r `--radius-xs`, `t-small`; remove buttons (:37,54) → `IconButton size="icon-sm" label="Remove …"`; error pill :65-68 + CSS :127 → `<Banner tone="danger" onDismiss>`. Logic: remove handlers unchanged. Verify: V; attach 3 files → 3 `.tag-in` animations once.
26. **`feat(composer): QueueRail on FlipList`** — `cmp/QueueRail.svelte:125-126` per-row `in:fly/out:fly` → `class="rise-stream"` in, `out:` opacity `--dur-pop-out`, list items `animate:haloFlip`; rail accordion (P2 M38) kept; `.rail-chev` :174 → `.chev`. `[data-qedit]` (:32,:135) unchanged. Verify: V; queue 3, delete middle → remaining rows FLIP 460 (`getAnimations()[0].effect.getTiming().duration` = 460).
27. **`feat(composer): ComposerPopover shell`** — new `cmp/ComposerPopover.svelte`: absolute in `.composer-shell`, `bottom: calc(100% + 8px)`, left/right = well edges, `class={cn(floatingPanel, "animate-pop-in-up")}`, `data-composer-popover`, `z-index: var(--z-dropdown)`, max-height `min(360px, 50vh)`. No consumer yet. Verify: V.
28. **`style(composer): SlashMenu on ComposerPopover`** — `cmp/SlashMenu.svelte:80-128` root → `<ComposerPopover role="menu" label="Commands and skills">`; rows `cn(menuItem, "animate-none")` (D-20: no stagger), group heads `menuLabel`, highlight via `data-highlighted`; delete `.rift-menu`/`.slash-menu` CSS :134-155 + compact override :348-453 where replaced; oklch :147,:357 gone. Logic: filtering, `slashIdx`, pick handlers untouched. Verify: V; `/` → menu above well; arrow keys move `data-highlighted`; `check:ui` oklch −2.
29. **`style(composer): MentionPopover on ComposerPopover`** — `cmp/MentionPopover.svelte:19` root → `<ComposerPopover role="menu">`; rows `menuItem` + `animate-none`; CSS :54-70 deleted. Verify: V; `@` → list; Tab picks.
30. **`style(composer): EnhanceBar on ComposerPopover`** — `cmp/EnhanceBar.svelte:80-189` root → `<ComposerPopover label="Improved prompt" aria-live="polite">` with **no** ARIA role (it is a preview strip, not a dialog; EnhanceBar has no role today, so `overlay:aria-role` must not rise); word reveal = opacity per word 150, 30ms stagger, no blur (03 #110, :386-401); buttons → kit `Button` (`primary` Use, `ghost` Dismiss); oklch :208 gone. The unguarded grounding `localStorage` in C:780-788 is not touched here (follow-up, §11 R11). Verify: V; enhance → panel above well; `getAnimations()` while enhancing = 1 (P2 M18 holds).
31. **`style(composer): SettingsMenu panel recipe`** — `cmp/SettingsMenu.svelte` root (:278-287, keeps `use:portal`, `id="model-effort-menu"`, `role="menu"`, `data-composer-popover`) → `floatingPanel` + `animate-pop-in-up`; rows (:288-332) `menuItem` (stagger kept, 18ms `--animate-menu-row`), tabs (:334-351) on `menuLabel` + kit tab look, Fast (:418-439) → kit `Switch`, effort slider (:442-495) track `--bg-300`, fill accent, knob 20px `--bg-000` ring accent, oklch literals :523-947 (15) → tokens. Virtual listbox driven from the textarea stays (D-22). Logic: `pickRow`, dial stops (`modelMatrix.ts:489-565`) untouched. Verify: V; `check:ui` oklch −15; keyboard rows walk as before.
32. **`perf(composer): SettingsMenu follows by ResizeObserver + FLIP end`** — `cmp/SettingsMenu.svelte:74-81` per-frame rAF loop → `ResizeObserver` on the anchor's `.composer-box` + `transitionend` (propertyName `transform`) on `.composer-host`, each calling the existing `place()`; doc mousedown close (:59-67) unchanged. Logic: same position function. Risk: menu lags the FLIP for 460ms → acceptable per D-22 (it lands on FLIP end); if the gap is visible, reposition on `transitionstart` + end (Q5). Verify: V; open settings in hero → send → menu ends anchored (`$C measure` delta ≤ 1px); `performance` trace shows no per-frame callback at rest.
33. **`style(composer): UsagePanel on ComposerPopover + ctxTone`** — `cmp/UsagePanel.svelte:193` root → `<ComposerPopover>` when `anchor !== "statusbar"`, plain root otherwise (statusbar mount `StatusBar:210` keeps its own positioning class); the file keeps exactly one `role="dialog"` literal, on an inner wrapper shared by both branches; zone thresholds :42-44 and `CtxRing:27` both call `helpers.ts ctxTone()`. Esc window (:182-191) unchanged. Verify: V; ctx 74/75/90% fixtures → ok/warn/danger in both ring and panel.
34. **`style(composer): CtxRing as IconButton`** — `cmp/CtxRing.svelte` wrapper → `IconButton label="Context used: N%"`, ring R6 kept, stroke tones `--accent`/`--warn`/`--danger`, transition `--dur-slow --ease-out` (was `--ease-soft`, :80). `.ctxring` class stays on the root (D-33). Verify: V; `$C look '.ctxring'`.
35. **`style(composer): GhPopover on ui/popover`** — `GhPopover.svelte:93-96` `use:portal` + `role="dialog"` → `<Popover.Root>` / `<Popover.Content side="top" align="start">` (bits supplies role + portal); `.gh-pop` z (:236, P1 → `--z-dropdown`) rule deleted; both mounts (`AssistantWelcome:199`, `StatusBar:162`) pass the trigger. Verify: V; `check:ui` use-portal −1, aria-role −1; opens upward from both mounts.
36. **`style(stream): PermissionBar on Button`** — `PermissionBar.svelte:48-62` → Allow `Button variant="primary" size="sm" loading={submitting}` (drops `Loader2 .perm-spin` for kit loading), Deny `Button variant="ghost" size="sm"`; `--ok` mixes :123-128 deleted. Coordinate with P4b (it owns the stream tile that hosts it). Verify: V; gated tool fixture → Allow/Deny render; Enter/Esc bindings unchanged.

### Phase E — cleanup, alternate layout, board

37. **`chore(composer): drop write-only hooks`** — delete `data-model` at C:1420 (`.composer-wrap`) and ModelPill (was :1764): zero readers in `src/`, `scripts/`, `.agents/` (grep at HEAD). `MessageBubble.svelte:283` `data-model` is a different element, untouched. Comment C:115 updated. Gated by shortlist #6 (§8). Verify: V; `rg "data-model" src/lib/components/assistant/composer src/lib/components/assistant/Composer.svelte` = 0.
38. **`style(composer): docked bar entrance`** — ComposerBar root gets `.rise-page` (03 #93, 200ms delay) on mount when docked after a hero FLIP; skipped on tab switch (`{#key}` not used; class applied only when `wasHero`). Verify: V; send from hero → bar rises once; `c.sh media reduce` → no animation.
39. **`feat(composer): inside-card layout behind COMPOSER_LAYOUT`** — `.composer` gets `data-layout={COMPOSER_LAYOUT}`; CSS block `.composer[data-layout=inside]` per §2.4 (chrome moves to `.composer`, bar padding `0 6px 6px`, `.composer-box` transparent); C gate becomes `{#if !hero || COMPOSER_LAYOUT === "inside"}`. Constant stays `"below"` → no visual change. Verify: V + D0 (below); `$C eval` set `data-layout=inside` on the live node → shot for board 03.
40. **`docs(composer): gallery + ARCHITECTURE`** — `/dev/ui` composer section (idle/typing/streaming/queue/drag/hero/inside) and `docs/ARCHITECTURE.md` frontend map rows for the new files. Verify: V.

**Side branch** `halo/p4c-inside-card` (same pattern as P1's `halo/p1-dur-base-300`; merged only if shortlist #4 = inside):
- S1 **`style(composer): flip to inside-card`** — `cmp/layout.ts` `COMPOSER_LAYOUT = "inside"`. One line. Verify: V + `$C diff` all composer states reviewed on board 03.

## 7 · Contracts (D-33)

**Hooks that keep their name and element** (grep at HEAD for consumers):

| Hook | Element after P4c | Consumers |
|---|---|---|
| `.composer-wrap` | Composer root (C:1420) | `c.sh shot-sel` docs (`scripts/cdp/c.sh:28`), rift-ui skill, P2 motion tests |
| `.pane-shell textarea` / `.assistant textarea` | the one textarea, still a descendant of `.pane-shell` (`AssistantPane:567`) | `c.sh type` (`c.sh:14`), rift-ui skill |
| `data-composer-popover` | PermMenu content (`PermMenu:81`), ComposerPopover root, SettingsMenu root | `AssistantPane:316` focusout guard (keeps the pane engaged while a popover has focus) |
| `#model-effort-menu` | SettingsMenu root (`:278-287`) | Composer focus logic, AP |
| `[data-qedit]` | QueueRail edit input (`:32,:135`) | Composer keyboard routing |
| `.ctxring` | CtxRing root (now the IconButton) | AP / StatusBar |
| `.composer-host` | AP host (`:802-810`) | P2 M36 FLIP; SettingsMenu `transitionend` listener (commit 32) |
| `.composer-box`, `.composer.hero`, `data-mode` on `.composer` | unchanged | ComposerPlaceholder `:global` rule, scoped CSS |

**Geometry couplings made explicit:**

| Coupling | Before | After |
|---|---|---|
| Max well height | literal 340 ×3 (C:212,219,1975) | `COMPOSER_MAX_H` → inline `--composer-max-h` |
| One-line height / `multiline` | literal 40 (C:216) | computed `lineHeight + padY×2` (commit 13) |
| Ghost / placeholder padding = textarea padding | literals, 1-2px off | `--composer-pad-y` + `--control-px` (commit 14) |
| Mic ring `rx` + viewBox | `rx=10`, 38×36 | read `--control-r` / `--control-h` (commit 23) |
| `composerH` (AP `:819` jump-latest offset) | measured from host | unchanged mechanism; inside-card changes its value, AP re-measures by ResizeObserver already — verify in S1 |
| Ripple radius | literal | `var(--control-r)` |

No hook is renamed in P4c. A future rename must update `c.sh`, `.agents/skills/rift-ui/SKILL.md` and `AssistantPane.svelte:316` in the same commit.

## 8 · Deletions (gated, shortlist #6)

Each is its own commit and waits for the owner's "yes" on shortlist #6 for this phase. Write-only or dead at HEAD (grep across `src/`, `scripts/`, `.agents/`):

| Item | Where | Evidence | Commit |
|---|---|---|---|
| `data-model` on `.composer-wrap` and the model pill | C:1420, C:1764 | no CSS or JS reader; only other hit is `MessageBubble:283`, a different element | 37 |
| `placeholder-fade` keyframes + `.static` override + stale `{#key placeholderKey}` comment | C:1990-2014 | every placeholder span carries `.static` (C:1593-1603); `placeholderKey` only appears in the comment | 15 |
| `.ph-k` kbd-in-placeholder | C:1599,1601,2007 | K8 owner pref | 15 |
| `.cbtn` family | C:2070-2108 | replaced by IconButton | 17 |
| composer use of `.rift-menu*` (12 class uses: Mention 3, Settings 6, Slash 2, Usage 1) | `cmp/*` | replaced by recipes | 28-33 |
| `app.css:695-736` `.rift-menu*` rules | `src/app.css` | **not deletable in P4c**: `FilePathMenu.svelte` (21 uses) and `webview/WebBrowserPage.svelte` (12) still consume it. Deleted by whichever phase ports the last of those two. | — |
| `.perm-spin` + `Loader2` in PermissionBar | `PermissionBar.svelte:62` | kit `loading` | 36 |

The `Composer.svelte` file is not deleted; it ends ≈1.6K lines (state machines + template shell). A deeper controller split (`useComposerKeys`, `useDictation`) is a possible follow-up, not P4c.

## 9 · Canvas board 03 — Composer

Per D-25: plain markup on Halo tokens in the Claude Design project "Rift · Halo", played in Play mode; no dev server. It exports the numbers this file uses; the port reads them back.

**Question it answers:** shortlist #4 (bar below vs inside one card) and D-13's ripple (one ring vs none).

| Frame | Content | Play-mode interaction |
|---|---|---|
| 03-A Below (D-22 default) | docked composer in a 760 column on `bg-000`, compact; idle / typing 3 lines / max height (scroll) / streaming | type in the textarea; focus ring; Send turns primary when text exists |
| 03-B Inside (alternative) | the same four states as one r16 card (§2.4) | identical interactions, side by side with A |
| 03-C Density | A and B at compact / regular / comfy (§2.1 table) | density toggle |
| 03-D Send strip | idle · ready · hover (glow-strong) · press .97 · queue (ListPlus) · stop (danger-soft) · busy | click: glyph spring; a "ripple on/off" toggle for the D-13 check |
| 03-E Hero ↔ docked | hero r16 at `--chat-col` under the welcome line → docked; 460 ease-out; bar materialises mid-flight (A) or is already there (B) | "Send" plays the FLIP; reduced-motion toggle |
| 03-F Popovers | slash (no stagger), mention, settings (rows 18ms stagger, effort detent slider with spring snap, Fast switch), enhance (word reveal), usage, permission menu, GitHub popover — all opening upward above the well | open each; arrow keys move highlight |
| 03-G Attach | three chips entering with `.tag-in`; attach-error Banner; drag-over overlay with "Drop files to attach" | drag a file onto the frame |
| 03-H Queue | QueueRail with 3 items, one in edit; delete the middle → FlipList 460 | add / delete |
| 03-I Dictation | mic recording: waveform bars (scaleY), thin countdown ring, ghost text with caret; transcribing spinner | play a canned meter sequence |
| 03-J Placeholder copy | the six phrases (§6 commit 15) | — |

**Exports back to this file:** composer pad-y per density, bar gap, Send size/radius per density, ring stroke width, popover offset (8) and max-height, queue glyph choice, A or B. Any number the owner changes on the board replaces the value here in a one-line amendment before the port starts.

## 10 · Verification

### 10.1 Commands

- Every commit: `npm run check` (svelte-check + `check:tokens` + `check:ui`) and `npm test`. Node 24.x.
- Phase end (Windows): `npm run verify`. P4c has no Rust change; a cloud session reports only `check` + `test` and says so (CLAUDE.md §Verification).
- New tests: `cmp/slashCatalog.test.ts` (1), `helpers.test.ts` additions for `enhanceContext` and `ctxTone` (2, 33).

### 10.2 CDP steps (`$C = bash scripts/cdp/c.sh`, audit viewport 2048×1104 @ DPR 1.25)

1. Before commit 1: `$C baseline ".composer-wrap" composer-<state>-<density>` for states idle, typing, multiline, streaming, queue (2 items), hero, drag-over, slash open, settings open — at compact, regular, comfy (refs land in `.tmp/base-<name>.png`).
2. Phase A commits: `$C diff ".composer-wrap" composer-<state>-<density>` → 0% changed pixels for every state. Any non-zero diff blocks the commit.
3. Phase B-E: `$C diff` per touched state, reviewed (owner-visible changes are expected and listed in the commit body).
4. `$C measure '.composer-box'` → height 40/44/52 one-line; ≤ 340 at max. `$C measure '.dict-ghost'` vs textarea content box → equal.
5. Esc-to-stop (commit 22): start a long turn → `$C act click ".assistant textarea"` (focus) → `$C act key "Escape" ".composer-wrap"` → `$C eval "document.querySelector('.composer').dataset.mode"` is no longer `stop` within 1s. Then, streaming again, `$C type ".assistant textarea" "/"`: the first Esc closes the slash menu (and clears the draft, C:1302), the stream continues; a second Esc stops it.
6. FLIP: `$C eval "getComputedStyle(document.querySelector('.composer-host')).willChange"` = `auto` at rest (P2 M36 holds).
7. Settings follow (commit 32): open settings in hero → send → `$C measure '#model-effort-menu'` bottom = model pill top − 8 ±1 after 500ms.
8. Reduced motion: `$C media reduce` → send, open popovers, add chips, queue: 0 running animations except Spinner.
9. `$C inspect` → 0 console errors.

### 10.3 Counts that must go down (never up)

| Metric | At HEAD | After P4c | Why |
|---|---|---|---|
| `check:ui overlay:use-portal` (`/use:portal\b/`, outside `ui/`) | 11 | 10 | GhPopover → `ui/popover` (35); SettingsMenu keeps its portal (02 §Recipes); Slash/Mention/Usage/Enhance were never portalled |
| `check:ui overlay:aria-role` (`role="menu\|listbox\|dialog\|alertdialog"`, outside `ui/`) | 17 | 16 | GhPopover's literal goes (bits supplies it). Rule: one role literal per file, at the call site — Slash 1, Mention 1, Settings 1, Usage 1 (as today); EnhanceBar adds none; `ComposerPopover.svelte` has no literal (it spreads `role` from props) |
| `check:ui color:oklch-literal` | 131 | 131 − (composer literals left after P2) | composer files hold 22 at HEAD (C 2 `:2238,:2323`; Settings 15; Slash 2; Usage 2; Enhance 1); P2 M18/M33 remove some; P4c removes the rest → composer files 0 |
| `.rift-menu` uses in `cmp/` | 12 | 0 | recipes |
| `.cbtn` rules in C | 1 family | 0 | IconButton |
| `Composer.svelte` lines | 2604 | ≈1.6K | Phase A |
| per-frame rAF callbacks at rest with settings open | 1 | 0 | commit 32 |
| keyframes defined in C | (P2 count) | −1 (`placeholder-fade`) | commit 15 |

## 11 · Risks

| # | Risk | Likelihood · impact | Mitigation |
|---|---|---|---|
| R1 | Phase A is not pixel-identical: a scoped rule in C targeted markup now in a child, or an unnamed `@container` resolves to another ancestor | med · low | D0 blocks each commit; named container `composer-bar`; `:global()` only inside the new file |
| R2 | Line drift: P1/P2/P3/P4b edit C before P4c (P2 alone touches ~15 sites) | high · low | anchor by quoted snippet; this file's lines are HEAD `22727d24` |
| R3 | Esc-to-stop surprises: user presses Esc to dismiss something that is not a layer (tooltip, OS IME) and halts a turn | med · med | only from the textarea; after every layer; `isComposing` guard; Q2 decides the non-empty-draft case |
| R4 | SettingsMenu lags the 460ms FLIP (RO + `transitionend` instead of rAF) | med · low | D-22 accepts end-only; fallback = also place on `transitionstart` (Q5) |
| R5 | `multiline` derivation reads computed style before fonts settle (Geist load) → wrong layout first frame | low · low | recompute on `document.fonts.ready` and density change |
| R6 | PermissionBar lives in the stream (`MessageBubble:420,423`, `StreamTurn:383-393`), owned by P4b's tiles → merge conflict | med · low | commit 36 is self-contained; rebase onto P4b; move to P4b if it lands later (Q8) |
| R7 | `.rift-menu` rule deletion is blocked by FilePathMenu / WebBrowserPage | certain · none | stays; recorded in §8 |
| R8 | UsagePanel has two mounts (composer, `StatusBar:210`); ComposerPopover positioning must not apply to the statusbar one | med · med | branch on the existing `anchor` prop (`:193` `from-statusbar`) |
| R9 | Inside-card flip changes `composerH` and the hero→docked dy; jump-latest pill offset (AP `:819`) shifts | med · low | S1 verification re-measures; AP already observes the host |
| R11 | Grounding toggle reads/writes `localStorage` unguarded (C:780-788); a blocked-storage profile throws at composer mount | low · med | out of P4c scope (no logic change); separate `fix(composer)` follow-up |
| R10 | Un-portalled composer popovers (amendment A-12-2) clipped by an `overflow` ancestor in split panes | low · med | they are un-portalled today and render; verify slash in 3-way split at 320px pane |

## 12 · Effort

| Phase | Commits | Sessions |
|---|---|---|
| A extraction | 1-12 | 1.5 |
| B well + type | 13-16 | 1 |
| C bar controls | 17-24 | 1.5 |
| D chips, queue, popovers | 25-36 | 2 |
| E cleanup, inside-card, docs | 37-40 | 0.5 |
| Board 03 (before the port) | — | 0.5 |
| **Total** | 40 (+S1) | **≈7 sessions (≈4 working days)** |

Inside-card flip after the gate (S1 + re-measure + board review): ≈1 day, inside shortlist #4's 1-2 day estimate.

## 13 · Amendments to 00-decisions

- **A-12-1 (D-22 / D-33):** textarea vertical padding is `--composer-pad-y: calc((var(--control-h) + 8px - var(--t-chat-line)) / 2)` (9/10/13px), a component-local derived var on `.composer`, not a new global token. Horizontal padding reuses the kit's `--control-px`. One-line well = `--control-h` + 8 (40/44/52).
- **A-12-2 (D-22 "portalled"):** Slash, Mention, Usage and Enhance stay absolute inside `.composer-shell` via `ComposerPopover` (today's behaviour) instead of being portalled. Reasons: focus stays in the textarea (02 §Recipes), they ride the hero↔docked FLIP without a follow loop, and `check:ui overlay:use-portal` cannot rise. They are opaque (`floatingPanel`) and on `--z-dropdown`. SettingsMenu keeps its existing portal; GhPopover moves to `ui/popover`.
- **A-12-3 (D-22 "Send 32×32 r8"):** that is the compact rung of kit `size="icon"`; regular 36 r12, comfy 44 r12 (D-07 ladder).
- **A-12-4 (D-22 glyph swap):** queue gets its own glyph (`ListPlus`, Q1); today it reuses Send (C:1828). Swap = opacity 150 + scale .6→1 spring, no rotation.
- **A-12-5 (D-22 well):** hover (not focused) raises bg to `--bg-200`, inherited from the kit `fieldInput` recipe; D-22 was silent.
- **A-12-6 (D-22 Esc-to-stop):** scope is the composer textarea's keydown only, not a pane-global Esc. ActivityHud's stop button stays the pointer path.
- **A-12-7 (D-22 hero):** hero textarea uses `--t-body` (15/24) at every density; docked uses `--t-chat`.
- **A-12-8 (02 §Recipes ".rift-menu retires in 4c"):** the composer's uses retire in 4c; the `app.css` rule retires with the last of FilePathMenu / WebBrowserPage.
- **A-12-9 (03 #102 "model" popover):** model and settings are one popover (`#model-effort-menu`); its rows keep the 18ms stagger.

## 14 · Open questions (with defaults)

| # | Question | Default if unanswered |
|---|---|---|
| Q1 | Queue glyph: `ListPlus` or `CornerDownLeft`? | `ListPlus` |
| Q2 | Esc while streaming with a non-empty draft: stop, or do nothing? | stop (the draft is kept; Enter would queue it) |
| Q3 | Placeholder copy (commit 15)? | idle "Ask anything about this project" (unchanged), hero "What are we working on today?", question "Answer Rift's question here", queue "Type to queue a follow-up", image "Ask about the image…", dictating "Listening…" |
| Q4 | Docked bar entrance: `.rise-page` (03 #93) or the smaller `.rise-stream` 3px? | `.rise-page` |
| Q5 | SettingsMenu during the FLIP: land at FLIP end, or also reposition at start? | end only |
| Q6 | Should Esc-to-stop also work when focus is elsewhere in the pane? | no, textarea only (A-12-6) |
| Q7 | Hero text 15/24 at every density, or follow `--t-chat`? | 15/24 (A-12-7) |
| Q8 | PermissionBar port in P4c or P4b? | P4c commit 36, rebased onto P4b |
| Q9 | Composer controller split (`useComposerKeys`, `useDictation`) in P4c? | no, follow-up |

## Map corrections (verified this pass)

- `PermissionBar.svelte` is `src/lib/components/assistant/PermissionBar.svelte` (not under `composer/`) and is mounted by the stream (`MessageBubble:420,423`, `StreamTurn:383,388,393`), not by Composer.
- Grounding `localStorage` sits at C:780-788 (map: 782-788) and is unguarded (no try/catch).
- `placeholderKey` survives only in a comment (C:1990); every placeholder is `.static`, so `placeholder-fade` (C:2003-2014, cited live by 03 #94) is dead.
- `data-model` on C:1420 and C:1764 has no reader anywhere (write-only).
- `multiline = h > 40` (C:216) hard-codes the compact one-line height; it breaks at regular/comfy once padding follows density.
- Queue mode shares the Send glyph (C:1828 `mode === "send" || mode === "queue"`).
- Esc layers: enhance :1151, undo :1156, mention :1201 (map: 1202), settings :1275, slash :1302; Enter branch :1307.
- `check:ui overlay:aria-role` also counts `listbox` and `alertdialog` (`design-system/check-ui.mjs:97`), and `overlay:use-portal` counts `use:portal` users (`:96`), so un-portalling lowers it.
- P1 C17 already moves the composer popover z-indexes (EnhanceBar :212, UsagePanel :279, SlashMenu :140, MentionPopover :53, SettingsMenu :515, GhPopover :236) to `--z-dropdown`; P4c does not redo them.
- The AP FLIP 540ms timer (`:382`) is a deliberate safety margin (460 + 80, 03 §contracts), not a mismatch.
- `.rift-menu` is also used by `FilePathMenu.svelte` and `WebBrowserPage.svelte`, so 02's "retires in 4c" holds only for the composer's uses.
- 02 §Recipes classes SettingsMenu as a bespoke textarea-driven listbox; it is not a `ui/popover` candidate (GhPopover is).
- File line counts in map 04 (Composer 2604, SettingsMenu 970, SlashMenu 454, EnhanceBar 403, UsagePanel 353, QueueRail 206, AttachmentsRow 147, PermMenu 119, MentionPopover 110, CtxRing 84, GhPopover 285, PermissionBar 147) are correct.
