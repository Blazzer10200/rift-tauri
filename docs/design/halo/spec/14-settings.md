---
title: Rift · Halo — spec 14 · Settings (P4e)
status: draft
updated: 2026-10-19
phase: P4e
depends_on:
  - spec/00-decisions.md (D-02 D-03 D-05 D-06 D-07 D-12 D-13 D-15 D-16 D-17 D-20 D-29 D-30 D-31 D-32 D-33; shortlist #5, #6)
  - spec/01-foundation.md (P1 merged: C2 C12 C14 C15 C17 C19 touch SettingsPage)
  - spec/02-kit.md (P3 merged: K10-K30b primitives consumed here)
  - spec/03-motion.md (P2 merged: motion.css, reduced-motion net, settleGuard, page-sections; rows #136-#145)
  - board 06 (canvas, D-25)
  - maps 06-settings, 09-motion-inventory, 12-owner-preferences
tags: [rift, halo, spec, settings, p4e]
---

# Spec 14 — Settings (P4e)

Paths are relative to `C:/AI Workflow/projects/rift-tauri`. `SP` = `src/lib/components/settings/SettingsPage.svelte`. Every `SP:NNN` cite was opened on `design/chat-ledger @ 22727d24` on 2026-10-19. **P1, P2 and P3 edit SP before 4e starts, so those line numbers will have drifted: re-anchor every cite by the quoted selector/symbol, never by number.** `$C` = `bash scripts/cdp/c.sh`.

## 0. Summary

- Port the 2,023-line SettingsPage onto the P3 kit and split it: a ~350-line page shell (header, tabs, search, scroll body, confirm dialog) + 5 tab components + 2 provider stacks + 12 small Rift parts in `settings/parts/`. Page state (drafts, savers, derived provider state) moves to one context class owned by the shell, so drafts still survive tab switches.
- Visible result: one column edge (820), Halo Tabs with the sliding ink, SettingsRow anatomy (D-31), kit Switch/Segmented/Select/Input, native range sliders on the Halo slider look with the value bubble kept, `<details>` → animated accordions, opaque search popup with a zero-result state, bg-000 preview wells with no fake window dots, in-app confirm dialogs, ≤1 solid accent button per tab.
- Fixes: status banner shows accent/"ok" before auth resolves (and a check icon on warn states); search jumps landing on closed `<details>`; invisible focus on hue slider/segments/pick tiles; copy failure swallowed; 3 disabled opacities; 10 font sizes / 8 weights / 13 radii.
- Appearance follows D-02 (b): Electric blue is the default row; the dials live in a collapsed "Custom accent" accordion whose removal is the last, owner-gated commit (#5).
- 35 commits (+2 gated). Effort L, 5-6 sessions. Board 06 exports the open numbers.

## 1. Scope + files

### 1.1 In scope
D-31 SettingsRow + column edge; D-30 focus/disabled/text ladder; D-32 copy, glyphs, hit targets; D-02 Appearance accent UI (opt-in dials); D-05 Geist label + ligature fate; D-06 density on every control; D-12 one accordion; D-13/D-29 tab-switch motion; D-15 opaque popup; D-17 tooltip call sites kept; D-20 confirm → AlertDialog (Settings' two sites only, A-2); D-33 contracts; `settings-controls.css` import retirement.

### 1.2 Out of scope
Store-level native `confirm()` in `src/lib/state/cliUpdate.svelte.ts:432` and `src/lib/state/updates.svelte.ts:287` (4f). Mounting the tooltip layer / Toaster app-wide (4a). Onboarding step-1 swatches (4d). `PageHero.svelte` deletion (still used by `AiHealthPage.svelte:721,824` and `DiagnosticsPage.svelte:13`; after 4f). Shortcut list values tied to D-19's Ctrl+1 change (4a edits `SHORTCUTS` in the same commit as the key binding). Physical deletion of legacy CSS files (P5, §8).

### 1.3 Landed before 4e (do not re-spec; verify with the precheck in §6.0)

| Owner | Commit | What it already did to Settings |
|---|---|---|
| P1 | C2 | `SP:1707` `.sp-prose` font-family deleted |
| P1 | C12 / C19 | `SP:718,1801,1841` accent literals → `var(--accent)`; `SP:720` copy; `SP:723-724` swatches use `uiPrefs.isAccent(a)` / `pickAccent(a)`; ACCENTS gains "Electric blue" (fixed) first; `uiPrefs.customAccent`, keys `.v2` |
| P1 | C14 | `SP:908` font radio label "Inter" → "Geist" (stored value stays `"system"`, see map correction M-3) |
| P1 | C15 | radius ladder; `.seg button.on` loses `shadow-sm` |
| P1 | C17 | `SP:1624` `.sset-results` z → `var(--z-dropdown)` |
| P2 | M-series | `motion.css` (keyframes, `.page-sections`, `.chev`, `.acc-panel`, `[data-settled]` guard), global reduced-motion net, `settleGuard` action, `c.sh media`; M34 `SP:1762` `.iface-row` height transition gone; M35 `SP:1617` search width fixed 210; D-15 sweep removes the `SP:1624` `backdrop-filter` |
| P3 | K10-K30b | Button/IconButton, Card, Badge, Kbd, LiveDot, Spinner, SectionLabel, Banner, Segmented, Tabs, SearchField, Switch, RadioGroup, Field/Input/Textarea, Slider (unused here, A-1), Progress, PageHeader, Collapse, Accordion, AlertDialog `destructive`; `sliderBubble.ts:28` reads `--slider-thumb` (K23b) |

### 1.4 Files

| File | Lines today | 4e change |
|---|---|---|
| `src/lib/components/settings/SettingsPage.svelte` | 2023 | becomes the shell (~350): Tabs.Root, PageHeader, SettingsSearch, scroll body, Tabs.Content ×5, SettingsConfirm, `:global(.card.sflash)`, `onGlobalKey`, onMount loads |
| `src/lib/components/settings/sections.ts` | new (~120) | `Section`, `ST_SECTIONS` (`SP:51-65`), `secDomId` (`SP:56-58`), `RESET_COPY` (`SP:322-327`), `SHORTCUTS` (`SP:564-574`), `STT_LANGS` (`SP:362-375`), `LOCAL_TOOLS` (`SP:378-384`), `STT_ENGINES` (`SP:388-392`), `DENSITIES`/`NARRATIONS`/`COMMAND_OUTPUTS` (`SP:38-49`) |
| `src/lib/components/settings/search.ts` (+`search.test.ts`) | new (~90 / ~70) | `SEARCH_INDEX` (`SP:105-150`), `scoreSearch()` (`SP:167-182`), `ACCORDION_ANCHORS` |
| `src/lib/components/settings/settingsCtx.svelte.ts` | new (~330) | `class SettingsCtx` + `setSettingsCtx/getSettingsCtx` (state `SP:67-68,151-156,274-279,398-419`; derived `SP:420-481,553-562`; savers `SP:483-547`; `resetTab`/`repairInstall` `SP:330-360`; `copyDiagnostic` etc. `SP:281-317`) |
| `src/lib/components/settings/providerStatus.ts` (+test) | new (~30 / ~40) | `statusTone()` pure (S26) |
| `src/lib/components/settings/tabs/{Appearance,Chat,Providers,Voice,System}Tab.svelte` | new (~180 / ~170 / ~70 / ~260 / ~230) | markup from `SP:714-808`, `811-938`, `941-1223`, `1226-1426`, `1429-1578` on parts |
| `src/lib/components/settings/providers/{Claude,ChatGpt}Provider.svelte` | new (~230 / ~150) | `SP:960-1140`, `SP:1141-1221` |
| `src/lib/components/settings/parts/*.svelte` | new, 12 files (§5.2) | SettingsRow, SettingsNote, PreviewWell, SecretField, RangeControl, RouteStrip, ActionRow, ModelRow, ProviderStatus, ProviderSwitch, SettingsSearch, SettingsConfirm |
| `src/app.css` | 780 | `:369-392` `.slider-bubble` restyle (S08). `.rift-toggle :476-493` and `.card :496-500` lose their last consumer (deleted in P5) |
| `src/lib/styles/motion.css` | P2 (~260) | one line: `.sflash` settle exemption (S19, A-6) |
| `src/lib/styles/settings-controls.css` | 37 | import removed (S34); file deleted in P5 |
| `src/lib/actions/sliderBubble.ts` | 81 | untouched (K23b already reads `--slider-thumb`) |
| `src/routes/dev/ui/+page.svelte` (or `dev/ui/sections/SettingsParts.svelte` if K36 split the page) | 445 | one exhibit per part (S03-S15) |
| `design-system/ui-baseline.json` | 8 (+P1 metrics) | `--update` after counts drop (S35) |

## 2. Target design

Every size, weight, colour and duration below is written in source as its token (`--t-*` type vars, density vars such as `--control-h`/`--card-pad`, Halo colour/radius/motion vars). The px/hex shown are the resolved compact values for review only; a literal `font-size: 13px` in a Settings file fails §10 step 3.

**Section grouping (first-open ceiling).** `.page-sections > *` rises each direct child of a tab column, capped at 6 (03-motion §4.7: ≤ 6 rises + 1 fade). Every tab column therefore has ≤ 6 direct children: Appearance = 2 cards + reset (3); Chat = 2 cards + reset (3); Providers = three `<section>` groups, one per SectionLabel (01 label + paragraph + ProviderSwitch; 02 label + status/account card; 03 or the ChatGPT API card + reset) (3); Voice = `<section>` "engine" (engine card + engine-dependent cards + Banners), composer card, reset (3); System = up to 5 cards (5). Columns are `display:flex; flex-direction:column; gap:16px` (sections use the same gap inside).

### 2.1 Page anatomy (compact; main island at the audit viewport ≈ 1360×824 CSS px)

```
┌ .sb-main (Tabs.Root) ── --page-col: 820px ─────────────────────────────────────────┐
│ PageHeader  pad 24 top · 32 sides                                                    │
│        ┌──────────────── column 820 ─────────────────────────────────────┐          │
│        │ SETTINGS ───────────────────────────────────────────            │ SectionLabel (no index)
│        │ Appearance                                                       │ title 24/30 600 −.02em
│        │ Accent, density and code rendering. Every change applies…        │ lede 13/18 ink-muted
│        │                                                    (16)          │
│        │ ◐ Appearance  ▢ Chat  ✦ Providers•  ⌁ Voice  ⓘ System  [⌕ Find a setting…   / ] │ h32
│        │ ▔▔▔▔▔▔▔▔▔▔▔ 2px accent ink ──────────── hairline line-100 ─────── │
│        └──────────────────────────────────────────────────────────────────┘          │
│ .surface-body (scroll, scroll-behavior:auto)                                           │
│        ┌ Card #card-accent r16 bg-100 line-100 pad 16 ────────────────────┐ top 24   │
│        │ Accent                                         heading 16/22 600   │          │
│        │ The highlight colour across Rift…              small 13/18 muted  │ (8)      │
│        │ ─────────────────────────────────────────────────────────────── │ line-100 │
│        │ Electric blue                                   [■] (Default)      │ row ≥44  │
│        │ Rift's accent: Send, toggles, selection, focus. caption muted      │          │
│        │ ─────────────────────────────────────────────────────────────── │          │
│        │ Custom accent                              Optional        ⌄      │ accordion│
│        └────────────────────────────────────────────────────────────────────┘ gap 16  │
│        ┌ Card #card-interface … rows … PreviewWell ─────────────────────────┐          │
│        [↺ Reset Appearance to defaults]  ghost, left                    bottom pad 48 │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

Narrow (≤900px CSS, includes the 900×600 minimum window): tab icons hidden, search drops to its own full-width row under the tabs, SettingsRow wraps (control under label, right-aligned), ProviderSwitch one column, side padding 20. The `≤560px` block (`SP:2015-2022`) is unreachable (map 06 §2) and is not ported.

### 2.2 Shell values

| Element | Token(s) | Resolved (compact / regular / comfy where it differs) |
|---|---|---|
| Column | `--page-col` set on `.sb-main` (A-16) | 820px; header and body both `max-width: var(--page-col); margin-inline: auto` |
| Header host pad | PageHeader K28 | 24 top · 32 sides · 0 bottom (board 06 may export new values) |
| Eyebrow | SectionLabel, no index | "SETTINGS" mono 11/14 500 .12em `ink-faint` #7d8592 + `line-100` rule |
| Title | `--t-title` | 24/30 600 −0.02em `ink` #eceff3 = active tab label |
| Lede | `--t-small`, `ink-muted` | 13/18 #9aa2af, max 64ch, = `ST_SECTIONS[].sub` |
| Tab row | `.tabrow` flex, `align-items:center`, gap 16, `margin-top:16px`, `border-bottom:1px solid var(--line-100)` | Tabs.List overrides `border-b-0` (the row owns the hairline) |
| Tab trigger | kit K21b | h `--control-h` 32/36/44; gap 8 icon↔label; 13/18 500 `ink-muted` → `ink` (hover, active) over `dur-base`; list gap 24 (A-3) |
| Tab icon | lucide, 16, stroke 1.8 | `ink-faint` → `ink` active (board lever: accent) |
| Ink underline | kit indicator | 2px, `--radius-pill`, `accent` #6aa8ff, `translate`+`width` 300ms (`--dur-base`; 240 until #3) `ease-out` |
| Providers attention dot | span 6×6 r50% `warn` #ff9f5a | + `<span class="sr-only">needs attention</span>` + `use:tooltip` "{Claude|ChatGPT} connection needs attention" (replaces `title=` at `SP:648`) |
| Search field | kit SearchField K22 | fixed width 210; h 32/36/44; `bg-100` #0e1014, `line-300` #5f6774 border, `--control-r` 8/12/12; placeholder "Find a setting…" `ink-faint`; icon 16 `ink-muted` → `accent` on focus; Kbd `/` 20px shown empty+unfocused |
| Results popup | `floatingPanel` recipe | absolute `top: calc(100% + 8px); right: 0`; width 360; max-height 360; `bg-100`, `line-200` #2c323c, r12, pad 4, `shadow-pop` 0 12px 32px rgba(0,0,0,.45), `z-dropdown` 65; opaque, no backdrop |
| Result row | `menuItem` recipe + `min-h-11` | ≥44; r8; title 13/18 500 `ink`; card name 12/16 `ink-faint`; tab Badge neutral (bg-300 / ink-muted, h20); `CornerDownLeft` 14 `ink-muted` on the highlighted row; highlighted = `bg-200` #14171c |
| Zero-result block | `role="status" aria-live="polite"` | pad 12/16; "No settings match “{q}”" 13/18 `ink`; "Try density, API key or shortcuts." 12/16 `ink-muted` |
| Body | `.set-surface` | pad 24 top · 32 sides · 48 bottom (narrow: 20 sides); column as above |
| Card | kit Card K12, `class="card"` + `id` | `bg-100`, 1px `line-100` #1f242c → `line-200` hover (300ms), r16, pad `--card-pad` 16/20/24, gap between cards 16, `scroll-margin-top: 12px` |
| Card title | `--t-heading` | 16/22 600 `ink` (board lever: subtitle 18/24) |
| Card description | `--t-small`, `ink-muted` | 13/18, margin-top 4, then 8 before the first row |
| SettingsRow | §5.2 | see 2.3 |
| Reset button | Button `ghost` default + `RotateCcw` 16 | h `--control-h`; left-aligned; margin-top 8 after the last card |

### 2.3 SettingsRow (D-31)

```
──────────────────────────────────────────────────────────────── 1px line-100 (border-top on every row)
│ Title 13/18 500 ink [note]                         [ control ]  │  min-height 44
│ Description 12/16 ink-muted, max 62ch                            │  pad-block 10 / 12 / 14
│ message line (SettingsNote) full width, 12/16                    │  gap 16 label↔control
```

| Part | Value |
|---|---|
| Row | `display:flex; align-items:center; justify-content:space-between; gap:16px; min-height:44px; padding-block: calc(var(--card-pad) / 2 + 2px)` (10/12/14); `border-top:1px solid var(--line-100)` |
| `stack` | column; control full width below the label; gap 10 |
| `sub` (gated child of a toggle) | `margin-left:8px; padding-left:16px; border-left:1px solid var(--line-200)` |
| `disabled` | whole row `opacity: var(--opacity-disabled)` .45; control gets its own `disabled` too |
| `overridden` | label block .45 only; control keeps its disabled look |
| Title | `--t-small` 13/18 500 `ink`; rendered as `<label for>` when `forId` is set, else `<div id={titleId}>` |
| Description | `--t-caption` 12/16 400 `ink-muted` #9aa2af (D-30: descriptive copy never `ink-faint`) |
| Inline `<code>` in copy | mono 12/16, `background: var(--code-bg)`, 1px `var(--code-border)`, `--radius-xs` 6, pad 0 4 |
| Note slot (beside title) | Badge neutral, margin-left 8 ("Custom mix", "Set by Detailed") |
| Control column | `flex:none; max-width:60%`; `grow` variant `flex:1 1 auto; max-width:360px` (sliders) |
| Narrow ≤900 | `flex-wrap:wrap; align-items:flex-start`; control `margin-left:auto` |

### 2.4 Controls (per density; all from P3)

| Control | Part | compact / regular / comfy | Notes |
|---|---|---|---|
| Toggle | `ui/switch` | 36×20 knob 14 travel 16 / 40×22 16 18 / 44×26 18 18 | `aria-labelledby` row title unless a distinct aria-label exists today (kept verbatim, §7) |
| Segmented | `ui/segmented` | h 32/36/44, pad 3, option min-w 52, r `--control-r` | neutral `bg-300` thumb (02-kit A-6); `mono` for numeric sets (code size, tab width, context limit, auto-stop); `role=radio` + arrows/Home/End (D-31) |
| Button | `ui/button` | h 32/36/44 (sm 28/32/36) | primary only per §2.8 |
| IconButton | `ui/button/icon-button` | sm 28/32/36 | eye, copy, refresh, delete model (D-32 floor: today 24 / 22×20) |
| Input | `ui/input` | h 32/36/44, `line-300`, r8/12/12 | `mono` class for keys, style, budget |
| Select | `shared/Select.svelte` → `ui/select` | trigger h 32/36/44 | width 240 |
| Textarea | `ui/textarea` | min 88, max 360, mono 12/16 | counter is a SettingsNote, not `maxLength` (Q9) |
| Range | `parts/RangeControl` (A-1) | track 6 r3; thumb `--slider-thumb` 16/18/20 | value mono 13/18 600 `ink`, min-w 44 right |
| RadioGroup cards | `ui/radio-group` `variant="cards"` | card pad 16, r12, min col 180 | engine picker, ProviderSwitch |
| Progress | `ui/progress` `size="sm"` | 4px track `bg-300` | context gauge, downloads, plan limits |
| Badge | `ui/badge` | h 20 | Rift "ok" = `accent` (D-03) |
| Kbd | `ui/kbd` | 20px, r6 | shortcuts |

### 2.5 Appearance tab

**Card `#card-accent` "Accent"** — description: "The highlight colour across Rift: Send, toggles, selection and focus rings."

| Row / block | Content |
|---|---|
| Accent row | title `uiPrefs.customAccent ? "Custom accent" : "Electric blue"`; description "Rift's accent, #6aa8ff." / "Your own hue. Electric blue is one click away."; control: 20×20 chip r6 `background: var(--accent)` `aria-hidden` + (default) Badge neutral "Default" or (custom) Button secondary "Use Electric blue" → `uiPrefs.pickAccent(ACCENTS[0])` |
| Accordion "Custom accent" | Card-embedded single-item Accordion (A-8). Trigger 44 high, 13/18 500 `ink`, meta caption `ink-faint` "Optional", `ChevronDown` 16 `.chev` 180°. Default closed; open on mount when `uiPrefs.customAccent`. Content pad-bottom 4 |
| ↳ Swatches | grid 9 × 1fr, gap 8, max-w 460; swatch h 32, r8, fill `oklch(0.72 0.16 {hue})` (kept literal, P1); 1px `line-200`; hover border `ink-faint`; press .97; selected `box-shadow: var(--shadow-focus)` + `Check` 16 stroke 3 in `var(--bg-000)` (kills `rgba(0,0,0,.82)` `SP:724`); `aria-pressed`; `use:tooltip={a.label}` |
| ↳ Hue | SettingsRow "Hue" + RangeControl `variant="spectrum"` 0-360 step 1, value `{h}°`, bubble `{h}°`; track = the 9-stop gradient (`SP:1840`, kept until #5) |
| ↳ Vividness | SettingsRow "Vividness" "How saturated the accent reads." + RangeControl fill, value `{vivPct}%` |
| ↳ PreviewWell "Accent" | inert demo built from kit parts: Button primary "Send", Switch checked, Badge accent `dot` "Connected", span `background: color-mix(in srgb, var(--accent) 28%, transparent)` "selected text", link `accent` underline offset 3, Input with `data-demo-focus` (`box-shadow: var(--shadow-focus)`, `border-color: var(--accent)`) "focus ring". Replaces `.ap-*` (`SP:1751-1758`) |

The header dot `.ap-dot` (`SP:718,1801`) is dropped: the accent row chip is its one home (map 06 pain #16).

**Card `#card-interface` "Interface and code"** — "Spacing across the app, and how code renders in replies."

| Row | Control |
|---|---|
| UI scale — "Zoom the whole app. Ctrl+= / Ctrl+-, Ctrl+0 resets." (keys as `<Kbd>`) | RangeControl grow, 0.8-1.5 step .05, value `{scalePct}%` |
| Interface density — "Compact fits more on screen; comfy breathes." | Segmented Compact/Regular/Comfy |
| Code font size — "Size of code in replies." | Segmented mono 11px/12px/13px/14px |
| Tab width — "Spaces per indentation level." | Segmented mono 2/4 |
| Font ligatures — "Render → ≠ >= as joined glyphs in Geist Mono." | Switch; **hidden** when P0 spike 2 found no ligatures (stored value untouched, D-05) |
| PreviewWell "Interface and code" | density stage: 3 rows h `var(--list-row-h)` 32/36/40, gap `var(--gap)`, pad 0 12, r8, 13/18 `ink-muted`; active row `bg-200` + `ink` + 8px accent dot (D-19 NavItem look); meta right 12/16 sans `tabular-nums` `ink-faint` ("2m", "now", "1h"). Code `pre`: mono, `font-size: var(--code-fs)`, `tab-size: var(--code-tab)`, `font-variant-ligatures: var(--code-liga)`, line-height 1.65, colours per D-18: keywords `var(--info)` #9cc6ff, strings/numbers `var(--chart-2)` #b79cff, operators `ink-muted`, comments `ink-faint` no italic, base `ink`. Braces written as `{"{"}` / `{"}"}` (drops 2 false hex hits, `SP:803`) |

### 2.6 Chat tab

**Card `#card-rendering` "Chat rendering"** — Stream view Switch (legacy, frozen; deletion is the 4b #6 commit, A-14). When on: Process detail Segmented (Calm/Standard/Verbose; `value = activePreset ?? ""` → no thumb, note Badge "Custom mix"; A-7); Tool detail Segmented; Narration Segmented; Command output Segmented (`disabled` when `toolDetail === "detailed"`, row `overridden`, note Badge "Set by Detailed"). PreviewWell "Stream": schematic on `ink` tokens: header = LiveDot `tone="idle"` + "Live process · 3 actions · 12s" 12/16 sans; prose 14/22; note 12/16 `ink-muted`; tool row = `ChevronDown` 14 (`.chev`, rotated 180 when open) + verb sans + target mono `ink` (not accent); diff `+` `accent`, `−` `danger`; terminal lines mono `ink` (`$ npm run check`, `✓ 0 errors` stays — it is terminal content). When off: PreviewWell "Classic bubbles", markup and colours frozen on tokens (legacy).

**Card `#card-comfort` "Reading comfort"** — Dyslexia-friendly Switch; sub rows Font Segmented Geist/Lexend and Wider spacing Switch (`sub`, `disabled` when off); Warm tint Switch; Autocorrect Switch. PreviewWell "Reply": `<div class="bubble markdown-body cf-bubble">` kept (D-33/D-30 coupling); `:where(.cf-bubble)` `bg-100`, r12, pad 12/16, prose `var(--t-chat-size)`/`var(--t-chat-line)` 14/22 so it matches real chat; inline code mono r6.

### 2.7 Providers tab

```
01 / AI CONNECTIONS ─────────────────────────────
Keep each account, plan and connection in one place.        13/18 ink-muted
┌ (◇) ChatGPT                     (●Connected) ┐ ┌ (✦) Claude                    (●Set up) ┐  RadioGroup cards, 2 col, gap 12
│     Plus plan · 12 models                    │ │     Connect Claude Code                  │  selected: accent border + 6% wash
└──────────────────────────────────────────────┘ └──────────────────────────────────────────┘
02 / CLAUDE SETUP ──────────────────────────────  (or 02 / CHATGPT SETUP)
┌ #card-claude ProviderStatus ───────────────────────────────────────────────────────────────┐
│ (◎) Signed in · Claude Max                     [claude 2.1.3] (CLI up to date)  checked 2m │
│     Every turn runs through your own Claude Code CLI.                       [↻ Check again]│
│ ─────────────────────────────────────────────────────────────────────────────────────────│
│ installs (if >1) · [Update now] [npm i -g …      ⧉] · output                                 │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
03 / CLAUDE PREFERENCES ───────────────────────
Card #card-session · Card #card-api (accordion) · [↺ Reset Claude preferences]
```

- SectionLabels replace `.provider-section-head` (`SP:943-946,961-964,1043-1046,1142-1145`): index `ink-faint`, text, rule `line-100`. The paragraph below each keeps today's copy at 13/18 `ink-muted`, margin 8 0 12. Section gap 24.
- **ProviderSwitch** (`SP:947-958`): RadioGroup cards, `aria-label="AI providers"`, value bound to `ctx.providerFocus`. Card: icon tile 32 r8 `bg-200` icon 16 `ink-muted` (`ink` when selected); name 13/18 600 `ink`; sub 12/16 `ink-muted`, one line, ellipsis; Badge right: accent `dot` "Connected" / warn `dot` "Set up" / neutral "Checking" with Spinner 12 decorative. The `{#key authPulseKey}` ping (`SP:956,99,1602-1606`) goes (Q2).
- **ProviderStatus** `#card-claude` (`SP:966-1041`): Card + `data-tone`. Tones (S26): `loading` (no auth, checking) → plain card, icon chip `bg-200` + Spinner 16, title "Checking session…"; `ok` (pill green) → plain card, chip `accent-soft` + `CircleCheck` 16 `accent`; `attention` (pill yellow/red) and `unknown` (no auth, not checking) → Banner recipe: bg `color-mix(in srgb, var(--warn) 8%, var(--bg-100))`, border `color-mix(in srgb, var(--warn) 32%, var(--line-100))`, chip `color-mix(in srgb, var(--warn) 16%, transparent)` + `TriangleAlert` 16 `warn`. Chip 28 circle. Head: title 13/18 500 `ink`; description 12/16 `ink-muted`; CLI warn line → SettingsNote warn (glyph `⚠` at `SP:975` → icon); feature-floor line → SettingsNote muted. Right cluster (wrap): Badge neutral mono "claude {ver}" (tooltip kept); CLI state Badge: neutral+Spinner "Checking…" / neutral "Update available · {ver}" / warn "Check failed" (tooltip error) / accent `dot` "CLI up to date"; stamp "checked 2m ago" 12/16 sans `tabular-nums` `ink-faint` (was mono, D-05); Button secondary "Check again" `RefreshCw`, `loading` while any probe runs. Body (hairline above, only when present): install rows (mono 12/16, Badge neutral method, Badge accent "Active", Badge warn "Behind", Badge neutral "Version?" + tooltip, path mono `ink-faint` ellipsis + tooltip, IconButton sm "Copy this install's update command"); update row: Button **primary** "Update now" (`ArrowUpCircle`, `loading` "Updating…") + command box (mono 12/16, `bg-000`, `line-200`, r8, pad 6 6 6 10, max-w 360) with IconButton sm copy (`Check` accent 1.8s when copied); output: error SettingsNote danger (pre-wrap), ok SettingsNote muted, stuck SettingsNote warn.
- **Card `#card-session` "Claude session"**: Full config Switch (`aria-label="Use full Claude Code config"`, disabled when `hasApiKey`); Git tools Segmented Read-only/Standard; Output style row `forId="asst-output-style"`: Input mono w180 + `<datalist>` (kept) + Button secondary "Save" (disabled unless dirty, `loading` "Saving…") + Button ghost "Clear" (when set); message SettingsNote. Context limit Segmented mono 200K/1M + Progress sm below (label "Context window", value `.2 | 1`, `format` → "200K" / "1M", `ariaLabel` set; the 20% tick `SP:1084,1734` is dropped).
- **Card `#card-api` "Optional Claude API access"** (A-8): trigger meta = Badge accent "Configured" when `hasApiKey`, else caption "Optional". Content: description; RouteStrip `[Claude session] or [API key] → note`; API-key row `forId="asst-apikey"`: `hasApiKey` ? Badge accent `dot` "Configured" + Button danger "Clear" : SecretField w240 placeholder "sk-ant-api03-…" + Button secondary "Save"; message; env-key SettingsNote warn; cost-cap row (only `hasApiKey`) Input mono number w104 right-aligned + Save + Clear; message.
- **ChatGPT stack** (`SP:1141-1221`): Card `#card-codex` "ChatGPT account": description; RouteStrip `[ChatGPT sign-in] → [Codex App Server] → note`; row title plan or "ChatGPT sign-in", description email · models · skills; controls Badge (accent "Connected" / warn "Sign in needed" / warn "CLI needed" / neutral "Checking") + Button **primary** "Connect" (only when CLI present and not connected) + Button secondary "Check again" (`loading`); plan limits: 2-col grid gap 12 of Progress sm (label = limit name, value `usedPercent/100`, format "{n}% used", tone accent <60, warn <85, danger ≥85 — Q4); notes (runtime muted, limits error warn, codex error warn). Card `#card-chatgpt` accordion "Optional ChatGPT API access": meta Badge (Configured/Connected/Needs attention) or "Optional"; RouteStrip with Badge neutral mono "{n} Rift models"; key row SecretField + Button secondary "Connect"; notes.

### 2.8 Primary-button policy (D-02 "solid accent = Send + ≤1 CTA per view"; A-13)

| Tab | The one `primary` | Everything else |
|---|---|---|
| Appearance, Chat, Voice, System | none | secondary / ghost / danger |
| Providers · Claude | "Update now" (only while `cliNewer`) | Save ×3 → secondary |
| Providers · ChatGPT | "Connect" (codex sign-in, only while needed) | API "Connect" → secondary |

Today 8 `.st-btn.primary` (`SP:1022,1067,1113,1130,1165,1210,1320,1541`) → ≤1 visible per tab.

### 2.9 Voice tab

Card `#card-engine` "Engine": Speech-to-text Switch (`aria-label="Enable speech-to-text"`); Recognition engine `stack` row → RadioGroup cards 3-col (label 13/18 500, meta mono 12/16 `ink-muted`, per-option `disabled` when backend missing or STT off); Whisper-missing block → Banner `info` (#9cc6ff tone) title + description + action Button ghost sm "How to build it" (`aria-expanded`, `.chev`) + Collapse with the `<ol>` (12/16 `ink-muted`, code mono) (A-10). Card "Web Speech" (no id): Language row → Select w240 (12 options, `hint` = BCP-47 id) (S29; until then RadioGroup cards); Continuous Switch. Cards "Parakeet model" / "Whisper model": ModelRow list (§5.2). Card "Capture and cleanup": Input device row → Select w240 + IconButton sm `RefreshCw` "Refresh device list"; Clean up Switch; Beam search Switch (whisper). Card "Vocabulary priming": Style prompt `stack` Textarea mono rows 3; Vocabulary `stack` Textarea rows 4 + counter SettingsNote "{n} / 800" (`ink-faint`; `warn` ≥720; `danger` >800; no hard cap). Banners: Web Speech unsupported → Banner warn; `stt.lastError` → Banner danger. Card `#card-composer` "Composer integration": 3 Switches (aria-labels kept) + Auto-stop Segmented mono Off/3s/5s/10s (value `String(n)` ↔ number). Reset ghost.

### 2.10 System tab

Card `#card-admin` (when `elevation.supported`): Status row → Badge accent with `ShieldCheck` "Administrator" or Button secondary "Relaunch as administrator" (`loading` "Relaunching…"); Always elevated Switch; error SettingsNote danger. Card `#card-build` "Build and install": identity row (no hairline above) = `<RiftLogo size={40}>` (D-26 mark, replaces `Sparkles` tile `SP:1460`) + "Rift" 16/22 600 + version mono 13/18 `ink-muted` + channel Badge (dev → neutral "Dev build" + caption "not auto-updated"; warn → warn `{label}`; else accent `{label || "Installed"}`) + Button secondary "Check for updates"; stack Badges neutral mono (Tauri 2, Rust, SvelteKit, Svelte 5, Tailwind 4, Halo — Q5); Config / Logs rows (path mono 12/16 `ink-muted` ellipsis + tooltip, "—" when empty; Button secondary `FolderOpen` "Open"); ActionRow "Source code and license"; caption "Works with Claude and ChatGPT" 12/16 `ink-muted` (the fake-button `.st-powered` with `#d97757` `SP:1495-1498,1942-1946` goes). Card `#card-shortcuts`: 4 rows (label 13/18 `ink`, keys `<Kbd keys>`, "or" 12/16 `ink-faint`) + Collapse with the other 5 + Button ghost full-width "Show all 9 shortcuts" / "Show less" with `.chev`. Card `#card-tools`: installed → Badge accent `dot` per tool (tooltip = use) + Button ghost sm `RefreshCw` "Re-check"; description drops "hover one to see what it does" (Q11); missing tools → SettingsRow + Button secondary `Download` "Install" (`loading` "Installing…"); notes. Card `#card-help`: 5 ActionRows; "Copy diagnostic" has three states (S31).

## 3. States

| Element | State | Trigger | Look |
|---|---|---|---|
| Page | first open | Settings mounts (keep-alive: once per session) | `.page-sections` cascade on the first tab's cards |
| Page | tab switch | Tabs value change, palette deep-link, search jump | panel fades 150ms; new column `[data-settled]` 450ms (no card replay) |
| Page | idle | — | 0 running animations |
| Tab trigger | rest / hover / active / focus | | `ink-muted` / `ink` / `ink` + ink underline / `shadow-focus` r6 |
| Tab trigger | attention | `providerNeedsAttention` | 6px warn dot + sr text + tooltip |
| Search | empty, unfocused | | Kbd `/` visible |
| Search | focused, empty | | no popup |
| Search | results | q matches | popup listbox, row 0 highlighted |
| Search | zero results | q matches nothing | popup status block, `aria-expanded=false` |
| Search | jump | Enter / click | popup closes, tab switches, accordion opens if needed, card scrolls to top, `.sflash` 1.6s |
| Card | rest / hover | | `line-100` / `line-200` |
| Card | flash | jump | `.sflash` ring (§4) |
| SettingsRow | disabled | gating toggle off / STT off | whole row .45 |
| SettingsRow | overridden | Detailed tool detail | label .45 + note Badge |
| Accordion card | closed / open | click, Enter/Space, jump | chevron 0° / 180°; closed panel `inert` |
| Segmented | no selection | `activePreset === null` | no thumb; first enabled option tabbable |
| Switch / Segmented / Select / Input | disabled | | .45, `not-allowed` |
| ProviderStatus | loading / ok / attention / unknown | `statusTone()` | §2.7 |
| ProviderSwitch Badge | connected / set up / checking | | accent / warn / neutral+Spinner |
| Buttons | loading | saving, checking, updating, installing, relaunching | Spinner replaces icon, `aria-busy`, label kept |
| Save buttons | disabled | not dirty | .45 |
| SettingsNote | muted / ok / warn / danger | saver results | icon `Info` / `Check` / `TriangleAlert` / `CircleAlert` 14 + 12/16 text in `ink-muted` / `ink-muted` / `warn` / `danger` |
| ModelRow | not downloaded / downloading / downloaded / active | | Download / Progress + Cancel / Use + delete / Badge "Active" + delete |
| Copy diagnostic | idle / copied / failed | click | `Copy` "Copy diagnostic" / `Check` accent "Copied to clipboard" 1.4s / `CircleAlert` danger "Copy failed — clipboard blocked" 1.8s |
| Confirm dialog | closed / open / running | reset, repair | AlertDialog, Cancel focused (K24) |
| Empty | model catalog loading | `models.length === 0` | SettingsNote muted "Loading model catalog…" |
| Empty | path unknown | | "—" |
| Narrow | ≤900px | window | §2.1 |
| Reduced motion | OS | | all instant; Spinner keeps turning; no sflash (call-time check kept) |
| Dyslexia | `data-a11y-font="lexend"` | | every control in Lexend (verify §10, risk R-2) |

## 4. Motion

All values from `motion.css` / Halo tokens. "base" = `--dur-base` (240ms until shortlist #3, then 300). RM = reduced motion.

| Element | Trigger | Property | from → to | Duration | Easing | Delay / stagger | RM |
|---|---|---|---|---|---|---|---|
| Card sections (first open only) | first panel mount | opacity, transform | 0, translateY(10px) → 1, none | 520ms `--dur-page` | `--ease-out` | 60 + 60·i, cap 6 (`.page-sections`) | net: instant |
| Tab panel | tab switch | opacity | 0 → 1 | 150ms `--dur-fast` | `--ease-out` | — | instant |
| Cards on later switches | tab switch | — | `[data-settled]` → 0s | — | — | 0 | n/a |
| Tab ink | tab switch, resize, fonts ready | translate, width | old tab → new tab | base | `--ease-out` | — (first measure no transition, `data-ready`) | instant |
| Tab label / icon | hover, select | color | `ink-muted` → `ink` | base | `--ease-out` | — | instant |
| Results popup | first result / zero state appears | opacity, transform | 0, scale .96 translateY(−4px) → 1, none; origin top right | 150ms | `--ease-out` | — | instant |
| Result rows | popup open / new keyed row | opacity, transform | 0, translateY(−3px) → none | 220ms `--dur-menu-item` | `--ease-out` | `min(i,12)·18ms` (`.menu-rows`, 03-motion #141) | instant |
| Result row highlight | arrow / hover | background | → `bg-200` | 150ms | `--ease-out` | — | instant |
| Search clear × | text entered | opacity, scale | pop-in | 150ms | `--ease-out` | — | instant |
| Search field | focus | border-color, box-shadow | `line-300` → `accent` + `shadow-focus` | 150ms | `--ease-out` | — (no width tween, #142) | instant |
| `.sflash` | jump | box-shadow | `0 0 0 2px var(--bg-100), 0 0 0 4px var(--accent)` → `0 0 0 2px transparent, 0 0 0 4px transparent` | 1600ms | `--ease-out` | — (fires after 2 rAF; exempt from settle, A-6) | not added (call-time check) |
| Jump scroll | jump | scrollTop | → card top | `scrollIntoView` smooth (UA) | UA | — | `auto` |
| Accordion panel | open / close | grid-template-rows | 0fr ↔ 1fr | base | `--ease-out` | — | instant |
| Accordion content | open | opacity, transform | 0, translateY(−4px) → 1, none | base | `--ease-out` | 60ms on open | instant |
| Chevron (`.chev`) | open | transform | 0 → 180deg; `ink-muted` → `ink` | base | `--ease-out` | — | instant |
| Switch track | toggle | background, border | `bg-300`/`line-300` ↔ `accent` | base | `--ease-out` | — | instant |
| Switch knob | toggle | translate | 0 ↔ `--toggle-travel` | base | `--ease-spring` | — | instant |
| Switch knob | press | width | knob → knob + 6px | 150ms | `--ease-out` | — | instant |
| Segmented thumb | pick | translate, width | old → new option | base | `--ease-out` | — | instant |
| Segmented label | pick, hover | color | `ink-muted` → `ink` | base | `--ease-out` | — | instant |
| Range thumb | hover / active | scale | 1 → 1.15 / 1.3 | 150ms | `--ease-spring` | — | instant |
| Slider bubble | grab / key nudge | opacity, transform | 0, translate(−50%,−100%) scale(.7) → 1, scale(1) | 150ms | `--ease-spring` | key nudge hides after 900ms (kept) | opacity only, 0.01ms |
| RadioGroup card | pick | border, background | `line-200` → `accent` + 6% wash; dot scale 0 → 1 | base | `--ease-out` / dot `--ease-spring` | — | instant |
| Card border | hover | border-color | `line-100` → `line-200` | base | `--ease-out` | — | instant |
| Buttons | hover / press | bg, border, colour / scale | → hover / .97 | 150ms | `--ease-out` | — | colour instant, no scale |
| Progress (context, downloads, limits) | value change | transform | scaleX(old) → scaleX(new) | 700ms | `--ease-out` | — | instant |
| Progress value text | value change | number | count old → new | 700ms | easeOutCubic | — | snaps |
| Collapse (shortcuts, Whisper steps) | expand | grid-template-rows + content fade | as accordion | base | `--ease-out` | 60ms | instant |
| Spinner (buttons, status chip) | loading | rotate | 0 → 360deg ∞ | 900ms | linear | — | keeps turning (essential) |
| Banner (warn/danger/info) | mount | opacity, transform | 0, translateY(10px) → none | base | `--ease-out` | — (zeroed inside settle) | instant |
| Copy icon swap | copy | — | swap only | — | — | — | — |
| Killed | — | — | local `blockIn` 8px (`SP:1590,1676-1684`), `pill-ping` (`SP:1602-1606`), `reveal-in` (`SP:1792-1797`), `.sw` lift (`SP:1835-1836`), `.st-spin` (`settings-controls.css:25-27`), width tweens on gauge/progress/search (`SP:1733,1987,1617`), `scroll-behavior:smooth` on `.surface-body` (`SP:1642`), 4 per-selector RM blocks (`SP:1607-1610,1643-1647,1684,1797`) | | | | | |

Concurrency: idle Settings = 0 animations; first open ≤ 7 (6 sections + panel fade); jump = 1 (`.sflash`) + accordion transition.

## 5. Parts

### 5.1 Kit primitives used (P3, unchanged APIs unless an amendment says otherwise)

PageHeader (K28), Tabs (K21b; A-3 gap), SearchField (K22; A-5 input passthrough), Card (K12), SectionLabel (K17), Badge (K13), Kbd (K14), LiveDot (K15, idle only), Spinner (K15), Banner (K19), Button + IconButton (K10, K11), Switch (K22b), Segmented (K21; A-7 no-selection), RadioGroup cards (K22d), Input/Textarea (K23), Select via `shared/Select.svelte` (K06/K23), Progress (K27), Accordion (K30b), Collapse (K30), AlertDialog `destructive` (K07/K24), `floatingPanel` + `menuItem` recipes (K05/K06), `use:tooltip` (K31), `settleGuard` + `.page-sections` + `.chev` (P2). Not used: bits-ui Slider (A-1), Checkbox, Combobox, Toaster (copy feedback is inline).

### 5.2 Rift parts built in 4e (`src/lib/components/settings/parts/`)

Scoped CSS on Halo vars; no dashed class names inside `ui/` (these are not in `ui/`, so ordinary scoped classes are fine). Every part takes `class` and spreads `...rest` onto its root.

```ts
// SettingsRow.svelte
title: string
description?: string | Snippet
forId?: string                 // render title as <label for>
note?: Snippet                 // Badge beside the title
stack?: boolean
sub?: boolean                  // gated child row
disabled?: boolean
overridden?: boolean
grow?: boolean                 // control column grows (sliders), max 360
control: Snippet<[{ titleId: string; descId: string | undefined }]>
children?: Snippet             // below-row content: SettingsNote, counters
// ids from $props.id(); root <div data-part="settings-row" data-disabled data-overridden>

// SettingsNote.svelte
tone?: "muted" | "ok" | "warn" | "danger" = "muted"   // icon Info/Check/TriangleAlert/CircleAlert 14
live?: boolean                 // role=status aria-live=polite (save results)
children: Snippet
// 12/16, margin-top 6; danger also role=alert when live

// PreviewWell.svelte
label: string                  // "Accent", "Interface and code", "Stream", "Classic bubbles", "Reply"
note?: string = "Updates live"
pad?: "default" | "none"
children: Snippet
// root aria-hidden="true" inert; bg-000, 1px line-100, r12, margin-top 12; head pad 8/12 SectionLabel(label, count=note) + border-bottom line-100; body pad 12/16

// SecretField.svelte
value: string                  // $bindable
visible?: boolean              // $bindable
id: string
placeholder?: string
width?: number = 240
showLabel?: string = "Show API key"; hideLabel?: string = "Hide API key"
// ui/input mono, type password|text, padding-right 36; IconButton ghost sm Eye/EyeOff at right 2, aria-label toggles
// autocomplete="off" spellcheck="false"; visible resets to false when value becomes "" (moves SP:416-417 here)

// RangeControl.svelte  (native <input type=range>, A-1)
value: number; min: number; max: number; step: number
oninput: (v: number) => void
ariaLabel: string
format: (v: number) => string  // value text and bubble
variant?: "fill" | "spectrum" = "fill"
disabled?: boolean
// use:sliderBubble={{ format }}; CSS ported from Halo bundle.css:177-186:
//   track 6px r3, fill = linear-gradient(to right, var(--accent) var(--pct), var(--bg-300) var(--pct)); spectrum = the 9-stop hue gradient
//   thumb size var(--slider-thumb), bg var(--ink), 4px solid var(--accent) border, margin-top calc((6px - var(--slider-thumb)) / 2)
//   hover scale 1.15, active scale 1.3 (spring), :focus-visible thumb box-shadow var(--shadow-focus), disabled .45
//   value span mono 13/18 600 ink tabular, min-width 44, right-aligned; --pct written as an inline style var

// RouteStrip.svelte
nodes: Array<{ label: string; on: boolean }>
joiner?: "or" | "then" = "then"    // "or" caption ink-faint, "then" ArrowRight 14 ink-faint
note: string                       // 12/16 ink-muted (no italic)
meta?: Snippet                     // e.g. model-count Badge, pushed right
// nodes = Badge accent when on, neutral when off; row gap 8, wrap, margin-bottom 12; no dashed box; NOT aria-hidden (the note is unique info)

// ActionRow.svelte
title: string; description?: string
icon: Snippet                      // lucide 16
onclick: () => void
// <button> full width; margin-inline -8px; width calc(100% + 16px); pad 10px 8px; r8; min-h 52
// icon tile 32 r8 bg-200 ink-muted; title 13/18 500 ink; desc 12/16 ink-muted; hover bg-200, tile bg-300; focus shadow-focus; hairline between siblings

// ModelRow.svelte
model: ModelInfo; active: boolean; enabled: boolean
progress?: { downloaded: number; total: number; phase: string }
onUse: () => void; onDelete: () => void; onDownload: () => void; onCancel: () => void
// hairline row (no tile): name 13/18 500 ink; mono 12/16 ink-muted filename · size; Progress sm (value downloaded/total or null when total 0,
// format "{MB} / {MB} · {pct}", ariaLabel "Downloading {name}"); actions per §3; delete = IconButton danger sm `Delete {name} model`

// ProviderStatus.svelte
tone: "loading" | "ok" | "attention" | "unknown"
title: string
description?: Snippet; badges?: Snippet; action?: Snippet; body?: Snippet
// renders Card id="card-claude" class="card" data-tone; values §2.7

// ProviderSwitch.svelte
value: "chatgpt" | "claude"       // $bindable
items: Array<{ id: "chatgpt" | "claude"; name: string; sub: string; icon: Snippet; badge: Snippet }>
// ui/radio-group variant="cards", aria-label "AI providers", 2 columns (1 at ≤900)

// SettingsSearch.svelte
query: string                     // $bindable
inputRef?: HTMLInputElement       // $bindable (onGlobalKey focuses it)
// passes label="Search settings" (aria-label, D-33 selector), placeholder "Find a setting…", shortcut ["/"] to SearchField
results: SearchEntry[]
tabLabel: (s: Section) => string
onJump: (e: SearchEntry) => void
// owns: focused, idx; combobox attrs on the input via A-5; listbox id "settings-search-results", options "settings-search-result-{i}"
// keys: ArrowDown/Up wrap, Enter jumps, Esc: SearchField clears text; Esc on empty blurs; rows onpointerdown preventDefault (keeps focus)

// SettingsConfirm.svelte
request: { title: string; description: string; confirmLabel: string; run: () => Promise<void> | void } | null   // $bindable
// ui/alert-dialog destructive (Cancel focused); confirm Button variant "warn"; Cancel "Cancel"; closes, then awaits run()
```

### 5.3 Page model (`settingsCtx.svelte.ts`)

```ts
export class SettingsCtx {
  activeSec = $state<Section>("appearance");
  providerFocus = $state<"chatgpt" | "claude">("chatgpt");
  accordions = $state<Record<"card-api" | "card-chatgpt" | "custom-accent", boolean>>({ "card-api": false, "card-chatgpt": false, "custom-accent": false });
  confirm = $state<ConfirmRequest | null>(null);
  scrollEl = $state<HTMLDivElement>();
  nowTick = $state(Date.now());
  // drafts — moved verbatim from SP:398-417; msgs become { text, tone } (A-9)
  asstApiKeyDraft = $state(""); asstApiKeySaving = $state(false); asstApiKeyMsg = $state<Msg | null>(null); asstApiKeyVisible = $state(false);
  openAiApiKeyDraft … ; asstMaxBudgetDraft … ; asstOutputStyleDraft …   // same four triplets
  // derived — verbatim SP:420-481, 559-562 as class $derived fields
  // methods — verbatim: saveAsstApiKey, saveOpenAiApiKey, saveAsstOutputStyle, saveAsstMaxBudget, runCliUpdate, reprobeAll,
  //   loadAboutPaths, openDir, openRepo, reportBug, replayWalkthrough, copyDiagnostic, selectSec, jumpTo, resetTab, repairInstall
  #firstPanel = true;
  takeFirstPanel(): boolean { const f = this.#firstPanel; this.#firstPanel = false; return f; }
}
export function setSettingsCtx(c: SettingsCtx): SettingsCtx;   // setContext(Symbol)
export function getSettingsCtx(): SettingsCtx;
```

The shell does `const ctx = setSettingsCtx(new SettingsCtx())`. Lifetime = SettingsPage lifetime (keep-alive), so drafts survive tab switches exactly as today (map 06 §9 data-loss risk).

## 6. Ordered commit list

Branch `halo/p4e-settings` off `design/halo` (after P1-P3 merged; 4a need not have merged). Subjects `refactor(settings)`, `feat(settings)`, `style(settings)`, `fix(settings)`. **V** = `npm run check` (svelte-check 0 errors, `check:tokens`, `check:ui`) + `npm test`, both green, on every commit. "diff" = `$C diff "" <name>` against the named baseline.

### 6.0 Precheck (no commit)

1. Grep `src/`: `data-slot="switch"`, `data-slot="segmented"`, `data-slot="card"`, `ui/page-header`, `ui/search-field`, `ui/accordion`, `ui/collapse`, `ui/progress`, `ui/radio-group`, `ui/alert-dialog` with `destructive`, `--slider-thumb` in `sliderBubble.ts`, `export function settleGuard`, `.page-sections` and `.chev` in `motion.css`, `--rise-page`. Any miss → STOP; land it in its owning phase.
2. Kit gaps this spec needs (land as the first commits below if P3 did not): A-3 Tabs gap, A-5 SearchField passthrough, A-7 Segmented empty value.
3. Read P0 spike 2 result in `spec/01-foundation.md §1.4` (ligatures).
4. Baselines (fixed audit viewport, P0-4): `$C nav settings`, then for each tab `$C act click '#settings-tab-<id>'` + `$C baseline "" p4e-pre-<id>` (ids: appearance, chat, providers, speech, about). Also `$C measure '.card' nokids > docs/design/halo/baselines/p4e/pre-measure.txt`.

### 6.1 Kit gaps (only if missing)

| # | Title | Files | Exact change | Preserves | Risk | Verify |
|---|---|---|---|---|---|---|
| S00a | `fix(kit): Tabs list gap = Halo space-5 (24px)` | `src/lib/components/ui/tabs/tabs-list.svelte` | `gap-5` → `gap-6` (A-3) | everything else | all Tabs consumers widen 4px/gap | `$C measure '[data-slot=tabs-list]' ` gap 24 |
| S00b | `feat(kit): SearchField forwards input attributes` | `ui/search-field/search-field.svelte` | `...rest` (or `inputProps`) spread onto `<input>`; Esc-with-text still clears + stops propagation | API, visuals | none | dev/ui: `role`/`aria-*` land on the input (`$C eval`) |
| S00c | `feat(kit): Segmented accepts no selection` | `ui/segmented/segmented.svelte` | `value === ""` or not in options → no `aria-checked=true`, thumb `opacity:0` (`data-empty`), first enabled option `tabindex=0` | keys, thumb | none | dev/ui exhibit with `value=""` |

### 6.2 A · Page model (no visible change)

| # | Title | Files | Exact change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| S01 | `refactor(settings): search index + scorer → search.ts (+test)` | new `settings/search.ts`, `search.test.ts`; SP `:105-150,167-182` | move `SearchEntry`, `SEARCH_INDEX` verbatim; `export function scoreSearch(q: string, index = SEARCH_INDEX): SearchEntry[]` = body of `:167-182` (startsWith 3000−len, includes 2000−idx, haystack 1000−idx, sort desc, top 8); `export const ACCORDION_ANCHORS = new Set(["card-api","card-chatgpt"])`. SP's `searchResults` = `$derived(scoreSearch(searchQ))` | scoring + order identical | none | tests: `""`→`[]`; `"density"`→[0].title "Interface density"; `"api"`→ first is "API-key fallback"; ≤8 results; `"zzz"`→`[]`; every `anchor` is one of the 16 `card-*` ids |
| S02 | `refactor(settings): page model → sections.ts + settingsCtx (context)` | new `sections.ts`, `settingsCtx.svelte.ts`; SP `:38-68,151-156,184-360,362-481,483-562,564-591,594-619` | constants → `sections.ts` verbatim; state, derived, savers, `jumpTo`, `selectSec`, `resetTab`, `repairInstall`, copy/report/open helpers → `SettingsCtx` verbatim (still native `confirm`); template reads `ctx.*`. `onMount` stays in SP and writes `ctx` | every behaviour; drafts live as long as the page | `$derived` in class fields must read stores lazily (no top-level reads) | V; `$C diff` all 5 tabs vs `p4e-pre-*` → IDENTICAL; type in the output-style field, switch tab and back → draft kept |

### 6.3 B · Parts (dev/ui exhibits only; no app consumer)

Each commit adds the part + one dev/ui exhibit (all three densities via the K37 switch). Verify for each: V + `$C eval "location.assign('/dev/ui')"` + `$C shot-sel '<exhibit>' jpeg 70` (+ `hover`/`focus`).

| # | Title | Files | Exact change / notes | Risk |
|---|---|---|---|---|
| S03 | `feat(settings): SettingsRow part` | `parts/SettingsRow.svelte` | API §5.2, values §2.3 | label/for wiring: dev/ui exhibit with Switch + Input |
| S04 | `feat(settings): SettingsNote part` | `parts/SettingsNote.svelte` | 4 tones, live region | — |
| S05 | `feat(settings): PreviewWell part (no window dots)` | `parts/PreviewWell.svelte` | bg-000, r12, SectionLabel head, `inert` + `aria-hidden` | `inert` support (Chromium ≥102: fine) |
| S06 | `feat(settings): SecretField part` | `parts/SecretField.svelte` | Input + IconButton; hit 28 | — |
| S07 | `feat(settings): RangeControl part (native range, Halo slider, value bubble)` | `parts/RangeControl.svelte` | §5.2; `use:sliderBubble`; spectrum variant carries the 9 oklch stops (moved from `SP:1840`, count unchanged) | bubble centring: thumb uses `--slider-thumb` like the bubble |
| S08 | `style(app): slider bubble on the tooltip look` | `src/app.css:369-392` | `.slider-bubble`: `z-index: var(--z-tooltip)`; bg `var(--ink)`; colour `var(--bg-000)`; `font: 500 12px/16px var(--font-mono)`; pad 6 10; r8; no border, no glow, `box-shadow: var(--shadow-pop)`; transition opacity/transform 150ms `var(--ease-spring)`. `input.slider-zoom::-webkit-slider-thumb { scale: 1.3 }` (was transform 1.35). Delete the RM block `:389-392` (P2 net covers it) | only consumer = Settings sliders (visible today) |
| S09 | `feat(settings): RouteStrip part` | `parts/RouteStrip.svelte` | — | — |
| S10 | `feat(settings): ActionRow part` | `parts/ActionRow.svelte` | — | — |
| S11 | `feat(settings): ModelRow part` | `parts/ModelRow.svelte` | logic of snippet `SP:1290-1326` (per-row) | — |
| S12 | `feat(settings): ProviderStatus part` | `parts/ProviderStatus.svelte` | presentational; `tone` prop | — |
| S13 | `feat(settings): ProviderSwitch part` | `parts/ProviderSwitch.svelte` | RadioGroup cards | arrows switch provider (behaviour gain) |
| S14 | `feat(settings): SettingsSearch part (results + zero state)` | `parts/SettingsSearch.svelte` | SearchField + listbox on `floatingPanel`/`menuItem` + `.menu-rows`; status block when 0 | role=listbox count unchanged (moves from SP) |
| S15 | `feat(settings): SettingsConfirm part` | `parts/SettingsConfirm.svelte` | AlertDialog wrapper | — |

### 6.4 C · Shell

| # | Title | Files | Exact change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| S16 | `feat(settings): PageHeader + one column edge` | SP `:624-626,1587,1649-1651` | `PageHero` → `PageHeader title={activeMeta.label} lede={activeMeta.sub} eyebrow={{ label: "Settings" }} maxWidth={820}`; `.sb-main { --page-col: 820px }`; `.set-surface { max-width: var(--page-col); margin-inline: auto; padding: 24px 32px 48px }` → content box shares the header's left edge (fixes the 38px offset, map 06 §2) | tab row still in the slot | `--page-col` must reach the body (A-16) | `$C eval` left x of `.sb-main h1`/title vs first `.card`: equal ±0.5 |
| S17 | `feat(settings): kit Tabs with sliding ink (ids kept)` | SP `:67,69-95,223-241,628-655,706-712,1592-1614,2000-2004` | `.sb-main` becomes `<Tabs.Root bind:value={ctx.activeSec} onValueChange={() => ctx.scrollEl?.scrollTo({ top: 0 })}>` (two-way, so the deep-link effect and `jumpTo` writing `ctx.activeSec` drive the tabs); `Tabs.List aria-label="Settings sections"` in the header slot; triggers `id="settings-tab-{secDomId}"`, icon 16/1.8, attention dot; `.surface-body` = plain scroll div; inside, 5 × `Tabs.Content value id="settings-panel-{secDomId}" class="set-panel"` each wrapping `{#if ctx.activeSec === id}` + the existing markup. `selectSec` → `scrollEl.scrollTo({ top: 0 })`. Delete `tabEls`, `tabnavEl`, `inkX`, `inkW`, `measureInk`, its 2 effects, `focusTab`, `onTabKey`, `.tabnav/.snav/.tab-ink` CSS | ids (D-33), deep-link effect `:264-272`, scroll reset | ArrowUp/Down no longer move tabs (bits-ui horizontal; Q14); bits-ui must not override ids | `$C map '[id^="settings-tab-"]'` → 5 ids; `$C act key ArrowRight` on a tab moves + selects; `$C shot-sel '.tabrow'` |
| S18 | `feat(settings): search on SearchField, opaque popup, zero-result state` | SP `:151-166,183,210-219,656-701,1616-1632` | swap to `SettingsSearch`; popup opens when focused and `q.trim()`; `onGlobalKey` (`/`, Ctrl+F) focuses `inputRef` | scorer, jump, ids, `/`+Ctrl+F | the popup sits inside the header; check it is above `.surface-body` (positioned, z 65) | §10 search steps |
| S19 | `feat(settings): tab-switch motion — panel fade, first-open cascade, no replays` | SP `:1590,1607-1610,1635-1647,1676-1684`; each tab root; `src/lib/styles/motion.css` | delete local `blockIn`, nth-child delays, RM blocks, `scroll-behavior: smooth`; `.set-panel { animation: halo-fade-in var(--dur-fast) var(--ease-out) both }`; each tab's column (`SP:715,812,942,1227,1430`, moved with its tab in S20+) becomes `<div class="set-col page-sections" use:settleGuard={{ key: 0, armOnMount: !ctx.takeFirstPanel() }}>`; `:global(.card.sflash)` keyframe → §4 ring; motion.css add `[data-settled] .card.sflash { animation-duration: 1.6s !important; }` beside the other re-declares (A-6) | `jumpTo` flash timing (1700ms timer) | sflash zeroed if the exemption is missing | §10 motion steps |

### 6.5 D · Tabs (each port = that tab's markup onto parts + extraction into its file; the tab's scoped CSS leaves SP in the same commit)

Common rules for S20-S30:
- Cards → `<Card id="card-…" class="card">` (id + class passed through, K12). Untitled cards keep no id (as today).
- `.ctl-row` → SettingsRow; `.rift-toggle` → Switch (`checked`, `onCheckedChange`, `disabled`, the existing `aria-label` kept verbatim); `.seg` + `onRadioKey` → Segmented (`ariaLabel` = today's `aria-label`); `.st-btn` → Button per §2.8; `.st-pill` → Badge (`ok`→`accent`, `warn`→`warn`, plain→`neutral`, `accent`→`neutral` "Update available"); `.st-note`/`.st-warn`/`.st-info` → SettingsNote / Banner; `.code-preview` → PreviewWell; `.set-expand.set-reset` → Button ghost `RotateCcw` "Reset {Label} to defaults".
- Each commit verifies: V; `$C nav settings` + `$C act click '#settings-tab-<id>'` + `$C diff "" p4e-pre-<id>` (reviewed; changes only in the ported regions); `$C measure '[data-slot=switch]' nokids` = 36×20 at compact; `$C inspect` 0 errors; `$C eval "document.getAnimations().length"` = 0 after 2 s.

| # | Title | Files | Exact change | Logic preserved | Risk |
|---|---|---|---|---|---|
| S20 | `feat(settings): Appearance tab on Halo parts` | new `tabs/AppearanceTab.svelte`; SP `:714-808,1749-1778,1801-1802,1824-1853` | §2.5 minus the accent-model rows (accent card keeps today's order: swatches, hue, vividness, preview, now on RangeControl + PreviewWell); Interface card full; reset | `uiPrefs.set*`, `pickAccent`/`isAccent` (P1), `--code-*` + density vars read by the preview | `.hue-range:focus{outline:none}` (`SP:1843`) gone → focus ring visible |
| S21 | `feat(settings): Electric blue default row + Custom accent accordion (D-02 b)` | `tabs/AppearanceTab.svelte` | accent row (§2.5) above an Accordion "Custom accent" holding swatches, hue, vividness, Accent preview; `ctx.accordions["custom-accent"]` initialised to `uiPrefs.customAccent`; `.ap-dot` dropped | dial behaviour (P1); reset | owner may find the dials too hidden → board 06 |
| S22 | `feat(settings): ligatures toggle per Geist Mono (spike 2); copy` | `tabs/AppearanceTab.svelte`; `search.ts` | if spike 2 = no ligatures: hide the row (`{#if GEIST_MONO_LIGATURES}` const in `sections.ts`), stored `code.ligatures` untouched; else keep. Copy "JetBrains Mono" → "Geist Mono" (`SP:786`); search kw "jetbrains mono" → "geist mono" (`SP:113`); if hidden, drop the "Font ligatures" index entry (test updated) | stored pref | — |
| S23 | `feat(settings): Chat tab on Halo parts` | new `tabs/ChatTab.svelte`; SP `:811-938,1700-1727,1737-1747,1806-1829` | §2.6; Process detail `value={uiPrefs.activePreset ?? ""}` (S00c); Command output `disabled` + `overridden`; Font Segmented (Geist/Lexend) + spacing `sub` rows; previews | `applyPreset`, a11y setters, `.bubble.markdown-body` on the reply preview | dyslexia font reach into kit controls (R-2) |
| S24 | `feat(settings): Providers tab — Claude stack on Halo parts` | new `tabs/ProvidersTab.svelte`, `providers/ClaudeProvider.svelte`; SP `:941-1140,1652-1698,1729-1735,1862-1927` | SectionLabels 01-03, each heading its own `<section>` (≤ 6 column children, §2 grouping); ProviderStatus (still fed by today's `assistantDot ?? 'ok'` mapping: `ok`→`ok`, `warn`→`attention`); session card; API accordion card (Accordion `value` bound to `ctx.accordions["card-api"] ? "item" : ""`); messages as SettingsNote with `{text, tone}` (A-9: `"Saved."`, `"Cleared."` → ok; `"Couldn't …"`, `"Enter an amount…"` → danger); "Update now" primary | savers, `cliUpdate.*`, ids `asst-output-style`, `asst-apikey`, `asst-budget` | `<datalist>` inside kit Input — pass `list` through `...rest` |
| S25 | `feat(settings): Providers tab — ChatGPT stack + ProviderSwitch` | new `providers/ChatGptProvider.svelte`; ProvidersTab; SP `:947-958,1141-1221,1656-1675` | ProviderSwitch bound to `ctx.providerFocus`; codex card (Connect primary), limits as Progress, API accordion card (`id="openai-apikey"`) | `startCodexLogin`, refresh calls, `providerFocus` deep-link (`:268`) | — |
| S26 | `fix(settings): provider status tone while auth loads (+test)` | new `providerStatus.ts`, `providerStatus.test.ts`; ClaudeProvider | `statusTone(auth, authChecking)`: `!auth && authChecking` → `loading`; `!auth` → `unknown`; `auth.pill === "green"` → `ok`; else `attention`. Icon per tone (§2.7). Title: loading "Checking session…", unknown "Session unknown", else `auth.summary` | summary text, login hint | none; tests: 4 tones incl. `pill:"red"`→attention, `undefined`+checking→loading |
| S27 | `fix(settings): search jumps open the collapsed card` | `settingsCtx.svelte.ts` (`jumpTo`), `search.test.ts` | before the 2-rAF wait: `if (ACCORDION_ANCHORS.has(e.anchor)) this.accordions[e.anchor] = true`; rest unchanged (scrollIntoView, focus, sflash) | providerFocus selection `:187-189` | — |
| S28 | `feat(settings): Voice tab on Halo parts` | new `tabs/VoiceTab.svelte`; SP `:1226-1426,1885-1900,1968-1993` | §2.9 (engine card + engine-dependent cards + Banners inside one `<section>`, §2 grouping) with the language list still a RadioGroup cards grid + "Show all 12 languages" ghost button + Collapse; engine RadioGroup; ModelRow; Select; Textareas + counter; Banners; auto-stop Segmented | `stt.*` calls, `headWithSelected` | RadioGroup per-option `disabled` |
| S29 | `feat(settings): spoken language as a Select (droppable)` | VoiceTab; `settingsCtx` | Select w240, options `STT_LANGS` with `hint=id`; delete `langsExpanded`, `langsShown`, `headWithSelected` (`SP:580-589`) | `stt.setConfig({ language })` | owner preference (Q3) |
| S30 | `feat(settings): System tab on Halo parts` | new `tabs/SystemTab.svelte`; SP `:1429-1578,1929-1966,1994-1998` | §2.10; `RiftLogo`; shortcuts Collapse (`ctx` keeps `shortcutsExpanded`); `.st-powered` replaced by a caption | `elevation.*`, `environment.*`, `updates.open()`, `workspace.setActive("diagnostics")`, `replayWalkthrough` | — |
| S31 | `fix(settings): show copy failures (D-32)` | `settingsCtx` `copyDiagnostic`/`reportBug`; SystemTab | `diagCopy = $state<"idle"|"copied"|"failed">`; catch sets `failed` for 1800ms (still `console.error`); ActionRow title/icon per §3; `reportBug` still opens the issue after a failed copy | timers cleared on destroy | — |
| S32 | `feat(settings): in-app confirm dialogs replace native confirm()` | `settingsCtx` (`resetTab`, `repairInstall`), SP (mount `SettingsConfirm bind:request={ctx.confirm}`), drop `import { confirm }` (`SP:13`) | `resetTab(sec)`: `if (this.confirm) return;` then `this.confirm = { title: "Reset {Label}", description: "Reset all {Label} settings to their defaults? " + RESET_COPY[sec], confirmLabel: "Reset", run }` with the store calls of `SP:343-350`; `repairInstall` → title "Repair installation", text `SP:355`, label "Repair", run `updates.repair()`. Delete `resetBusy` (`SP:328-341`) | one dialog at a time (the latch's purpose); store toasts on failure; Cancel focused | Enter used to hit OK in the native dialog; now Cancel has focus (K24, intended) |
| S33 | `style(settings): copy and glyph sweep (D-32)` | `sections.ts`, tabs | `RESET_COPY.appearance` "…go back to the stock emerald look." → "Accent, density and code rendering go back to the default Electric blue look."; `ST_SECTIONS[0].sub` "Accent color, density, and code rendering — …" → "Accent, density and code rendering. Every change applies instantly."; "Interface &amp; code" → "Interface and code"; `⚠` (`SP:975,1119,1548`) → SettingsNote icons; `→` in the route strip → `ArrowRight`; card titles sentence case ("Build and install", "Help and diagnostics", "Capture and cleanup"); stack chip "OKLCH" → "Halo" (Q5) | search titles that match card names updated in `search.ts` (`card` field) with the test | search test |
| S34 | `chore(settings): retire settings-controls.css import` | SP `:2` | delete the import; `rg -n "st-(btn|pill|input|secret|eye|spin)" src` → only `src/lib/styles/settings-controls.css` itself | — | a missed `.st-*` renders unstyled → grep gate | V; grep = 0 outside the file |
| S35 | `chore(gates): check:ui ratchet after the Settings port` | `design-system/ui-baseline.json` | `node design-system/check-ui.mjs --update`; diff the JSON (no count may rise); paste the before/after in the body | — | baking in a rise | `npm run check:ui` |

### 6.6 E · Gated (separate, droppable; not merged without the owner)

| # | Gate | Title | Change |
|---|---|---|---|
| S36 | #5 = yes | `feat(settings): retire accent dials (Electric blue fixed)` | AppearanceTab: delete the "Custom accent" accordion (swatches, hue, vividness, Accent preview), the "Use Electric blue" branch, `ctx.accordions["custom-accent"]`; RangeControl drops `variant="spectrum"` (−9 oklch literals); `search.ts` drops "Vividness", retargets "Accent color" kw; `RESET_COPY.appearance` drops "Accent". `uiPrefs` dial API untouched (onboarding/dev-ui are 4d/P3) |
| S37 | #6 = yes | `chore(settings): drop dead CSS the maps list` | remove `.bg-opt`, `.card-tt-act`, `.card-divider`, `.set-pick-grid-2`, duplicate `.st-cli-warn` (`SP:1797,1750,1800,1970,1920/1926`) — the port commits leave them in SP's `<style>` untouched so this stays a separate yes/no |

The Stream-view toggle + classic preview removal is **not** a 4e commit: it ships inside 4b's #6 legacy-deletion commit (A-14).

## 7. Contracts and couplings touched (D-33)

| Contract | Where | 4e obligation |
|---|---|---|
| `#card-*` ids ×16 (accent, interface, rendering, comfort, claude, session, api, codex, chatgpt, engine, composer, admin, build, shortcuts, tools, help) + `.card` class | Cards in tab files | kept verbatim; `search.test.ts` asserts every index anchor is in this list |
| `.sflash` | `jumpTo` + `:global(.card.sflash)` in SP | kept; settle exemption (A-6); Card never sets `animation` (K12) |
| `settings-tab-{id}` / `settings-panel-{id}`, `providers` alias | Tabs Trigger / Content | kept (K21b passthrough) |
| `settings-search-results`, `settings-search-result-{i}` | SettingsSearch | kept |
| `asst-output-style`, `asst-apikey`, `asst-budget`, `openai-apikey`, `asst-output-style-list` | inputs / datalist | kept (label `for`) |
| `commandPalette.targetSettingsSection` deep-link | SP effect `:264-272` | kept in the shell |
| `[data-a11y-*]` → `.bubble.markdown-body` | reply preview | classes kept |
| `--code-fs`/`--code-tab`/`--code-liga` | code preview | read unchanged |
| density preview vars | interface preview | now `--list-row-h` + `--gap` (the sidebar row var after 4a); if 4a lands later, the preview shows the token value early |
| `sliderBubble` (`thumbW` via `--slider-thumb`, `.slider-bubble`, `.slider-zoom`) | RangeControl + `app.css` | thumb CSS reads the same var (K23b) |
| `measureInk` coupling | SP `:78-95` | retired (kit indicator) |
| `onRadioKey` | SP `:242-257` | retired (Segmented / RadioGroup) |
| Toggle aria-labels (15) | Switches | kept verbatim (screen-reader names unchanged) |
| Sidebar `[aria-label="Settings"]` for `c.sh nav` (`c.sh:483`) | shell, not 4e | untouched |
| `SHORTCUTS` Ctrl+0/Ctrl+1…6 text | `sections.ts` | coupled to D-19's Ctrl+1 → Home: 4a edits the list in its key-binding commit |
| Native confirm in `cliUpdate.runUpdate` (`cliUpdate.svelte.ts:432`) | "Update now" in ProviderStatus | still native until 4f |

## 8. Deletions (flagged)

| Item | Where | When / gate |
|---|---|---|
| Accent dials UI (custom accordion, swatches 2-9, hue spectrum, vividness, accent preview) | AppearanceTab | S36, shortlist **#5** |
| Dead CSS listed by map 06 §5 | SP `<style>` | S37, shortlist **#6** |
| Stream-view toggle + "Classic bubbles" preview | ChatTab | with 4b's #6 legacy commit (A-14), shortlist **#6** |
| `src/lib/styles/settings-controls.css` (file) | — | import gone in S34; file deleted in P5 (02-kit §8.1), **#6** "dead CSS" |
| `app.css` `.rift-toggle` `:476-493`, `.card` `:496-500` | — | 0 consumers after S30; deletion P5 (02-kit §8.1) |
| `shared/PageHero.svelte` | — | after 4f ports AiHealth + Diagnostics (02-kit §8.2) |
| In-port code removals (not data): `measureInk`/ink state, `focusTab`/`onTabKey`, `onRadioKey`, `resetBusy`, `authPulseKey` + `pill-ping`, local `blockIn`/`reveal-in` keyframes, `langsExpanded`/`headWithSelected` (S29), `.st-powered` | SP | in the named commits; replaced, not orphaned |

## 9. Canvas board 06 — Settings

- **Board id:** `06 Settings` in the Claude Design project "Rift · Halo" (D-25). Plain markup on vendored Halo tokens with the Rift alias + deviation blocks (spec 01 §2.4-2.5) and `motion.css`; played in Play mode. No dev server.
- **Frames:**
  - A — 1360×824: the main island at the audit viewport (1638×883 CSS px, spec 01 P0-4) with the sidebar expanded. This is the primary frame.
  - B — 876×566: the 900×600 minimum window, sidebar collapsed (narrow layout, §2.1).
- **Contents (Frame A, every one clickable):**
  1. Header + tab row + search, with all 5 tabs. Clicking a tab plays the real switch (ink 300/240ms, panel fade 150, settled cards). The first load plays the `.page-sections` cascade once.
  2. Appearance: the default accent row; the "Custom accent" accordion (opens and closes with grid-rows motion); swatches; hue and vividness sliders with the bubble while dragging; the Interface card with live density preview rows and the code preview.
  3. Chat: Stream view on and off; the Segmented "Custom mix" state; the overridden Command output row; dyslexia sub-rows disabled and enabled; the reply preview with warm tint toggled.
  4. Providers: ProviderSwitch (both cards); ProviderStatus in all four tones side by side (loading, ok, attention, unknown) plus the update row; the API accordion card closed and open; the plan-limit Progress bars.
  5. Voice: the engine RadioGroup; the Whisper-missing info Banner with the Collapse; ModelRow in 4 states (incl. downloading at 37%); the language control in both forms (Select vs card grid).
  6. System: build identity with the Rift mark; shortcuts collapsed and expanded; ActionRows including the "Copy failed" state.
  7. Search: typing "api" (results, row 0 highlighted), "zzz" (zero-result block), Enter (jump: tab switch, accordion opens, `.sflash` ring).
  8. Confirm dialog for "Reset Appearance" (Cancel focused).
- **Levers** (`data-prop-*` on the board root plus an in-board control rail that works in Play mode; spec 03 Q7):

| Lever | Values (default first) | What it changes |
|---|---|---|
| `data-prop-density` | compact · regular · comfy | every `--control-h`, `--card-pad`, row padding, toggle size |
| `data-prop-card-title` | heading 16/22 · subtitle 18/24 | card titles (Q1) |
| `data-prop-row-min` | 44 · 40 · 48 | SettingsRow min height |
| `data-prop-card-gap` | 16 · 12 · 24 | gap between cards |
| `data-prop-tab-icon` | ink · accent | active tab icon (Q7) |
| `data-prop-seg-thumb` | neutral · accent | Segmented thumb (02-kit A-6) |
| `data-prop-dials` | accordion · removed | shows Appearance with shortlist #5 = yes |
| `data-prop-primary` | one-per-tab · today | the §2.8 policy against today's 8 primaries |
| `data-prop-status-tint` | attention-only · all-tones | whether `ok` also gets a tinted Banner |
| `data-prop-lang` | select · grid | Voice language control (Q3) |
| `data-prop-previews` | all · trimmed | trimmed = no Stream/Reply previews (Q8) |
| `data-prop-panel-motion` | fade · rise · none | tab switch (spec 03 Q2) |
| `data-prop-header-pad` | 24/32 · 32/40 | PageHeader top/side padding |
| `data-prop-dur-base` | 240 · 300 | shortlist #3 feel on ink, accordion, toggles |

- **Owner decides here:** shortlist #5 (dials); Q1, Q3, Q7, Q8; the primary policy; whether attention-only tinting reads right; tab-switch fade vs rise.
- **Numbers exported back into this spec** (edit §2 and §4 only; each is a one-line change): card title step, SettingsRow min height, card gap, header padding (also into 02-kit K28), results popup width (default 360), search width (210), tab-list gap (24), Segmented thumb tone, panel-fade duration (150), ProviderStatus tinting rule, language control, preview set, `--dur-base`.

## 10. Verification + acceptance

Run on the Windows dev machine, fixed audit viewport (P0-4), after S35 and again before merging `halo/p4e-settings` into `design/halo`.

**Commands**
1. `npm run check` (svelte-check 0 errors, `check:tokens`, `check:ui`) and `npm test` (includes `search.test.ts`, `providerStatus.test.ts`). Rust untouched, so no `npm run verify` needed.
2. `rg -n "settings-controls|class=\"rift-toggle|st-(btn|pill|input|secret|eye)\b|onRadioKey|plugin-dialog|backdrop-filter|PageHero" src/lib/components/settings` → 0 hits.
3. `rg -c "font-size:\s*[0-9.]+px" src/lib/components/settings` → 0 (was 57 in SP).

**CDP (`$C` = `bash scripts/cdp/c.sh`)**
4. `$C nav settings`, then `$C inspect` → 0 console errors. Repeat after visiting every tab (`$C act click '#settings-tab-<id>'`).
5. Column edge: `$C eval "(()=>{const t=document.querySelector('[data-slot=page-header] h1, [data-slot=page-header] [data-part=title]').getBoundingClientRect().left;const c=document.querySelector('.set-panel .card').getBoundingClientRect().left;return Math.abs(t-c)})()"` → ≤ 0.5.
6. Density: for compact, regular, comfy (`$C eval "document.documentElement.dataset.density='regular'"` etc.): `$C measure '[data-slot=switch]' nokids` → 36×20 / 40×22 / 44×26; `$C measure '[data-part=settings-row]' nokids` → height ≥ 44; `$C measure '#card-interface [data-slot=segmented]' nokids` → h 32/36/44; `$C measure '#card-accent' nokids` → r 16, pad 16/20/24. Reset to compact.
7. Ids: `$C map '[id^="settings-tab-"]'` → 5; `$C eval "[...document.querySelectorAll('[id^=card-]')].length"` summed across tabs → 16 (admin only when supported).
8. Search: `$C type 'input[aria-label="Search settings"]' "api"`; `$C eval "document.querySelector('#settings-search-results')?.children.length"` ≥ 1; `$C eval "getComputedStyle(document.querySelector('#settings-search-results')).backdropFilter"` → `none`; `$C act key Enter` → `$C eval "document.querySelector('#card-api [data-state]')?.dataset.state"` → `open` and `document.querySelector('#card-api').classList.contains('sflash')` → true within 300ms. Then type "zzz" → `$C eval "document.querySelector('[data-part=search-empty]')?.textContent"` contains "No settings match".
9. Flash survives settle: run step 8 from the Appearance tab (the jump switches to Providers) → `$C eval "document.querySelector('.card.sflash')?.getAnimations()[0]?.effect.getTiming().duration"` → 1600.
10. Motion: `$C nav settings` from Home on a fresh page → `$C eval "document.getAnimations().length"` ≤ 7 within 100ms; switch tab → ≤ 2 (panel fade + ink); wait 2 s → 0.
11. Reduced motion (P2 verb): `$C media reduce`; switch tabs, open an accordion, jump via search → `$C eval "document.getAnimations().filter(a=>a.playState==='running').length"` → 0 (no Spinner on screen); `$C media ''`.
12. Status tone: on a cold app start open Settings → Providers before auth resolves: `$C eval "document.querySelector('#card-claude').dataset.tone"` → `loading` (never `ok` before `auth`); after resolution it matches `statusTone` (unit test is the real net).
13. Slider bubble: `$C shot-sel '#card-interface' jpeg 70 active` while a pointer drag is simulated via `$C act click` on the range; eyeball bubble centred over the thumb at 100% and 150% UI scale.
14. Focus: Tab through Appearance; `$C shot-sel '<focused>' jpeg 70 focus` on a swatch, a Segmented option, the hue slider, a RadioGroup card, an ActionRow → every one shows the 2+2px ring.
15. Dyslexia: Chat → Dyslexia on → `$C measure '[data-slot=segmented] button' nokids` font family contains "Lexend" (R-2).
16. Narrow: resize to 900×600 → tabs one row without icons, search on its own row, no horizontal scroll (`$C eval "document.querySelector('.surface-body').scrollWidth <= document.querySelector('.surface-body').clientWidth"`).
17. Truncation count on Settings vs P0 (`$C eval` from spec 01 P0-5) → not higher; list any increase.

**Screenshots** (`docs/design/halo/baselines/p4e/`): each tab at compact (`$C baseline "" p4e-<id>`), Appearance at regular and comfy, search results, zero state, confirm dialog, ProviderStatus tones (dev/ui exhibit). Compare against board 06 exports.

**Counts that must go down (record before/after in S35's body)**

| Metric | Before (22727d24, SP) | After 4e |
|---|---|---|
| SP lines | 2023 | ≤ 400 (shell) |
| `font-size: Npx` in settings/** | 57 | 0 |
| distinct font weights | 8 | 3 (400/500/600) |
| distinct radii | 13 values / 17 forms | ladder only: 6/8/12/16/pill (+50% for dots) |
| disabled opacities | 3 (.45/.5/.55) | 1 (.45) |
| `.st-*` control class uses | 40 (22 `st-btn`, 10 `st-pill`, 4 `st-input`, 2 `st-secret`, 2 `st-eye`) + 2 `st-spin` | 0 |
| `rift-toggle` uses | 16 | 0 |
| `<details>` | 3 | 0 |
| native `confirm(` in settings | 2 | 0 |
| `role="radio"` hand-rolled | 14 groups | 0 |
| solid primary buttons | 8 | ≤ 1 per tab |
| `color:hex` in SP | 3 (`#d97757`, `&#123;`, `&#125;`) | 0 |
| `color:rgb-hsl` in SP | 1 (`rgba(0,0,0,.82)`) | 0 |
| `color:oklch-literal` in settings/** | 11 after P1 | 10 (swatch fill + 9 hue stops; −9 more with S36) |
| `backdrop-filter` | 1 (if P2 missed it) | 0 |
| `overlay:aria-role` | 1 listbox | 1 (moved, unchanged) |
| idle running animations on Settings | pill-ping on mount | 0 |

**Acceptance:** steps 1-17 pass; counts as above; every §3 state reachable in the app or `/dev/ui`; board 06 numbers applied; no draft lost across tab switches; `#card-*` jump works from every tab to every anchor (`search.test.ts` covers the index, step 8 the live path).

## 11. Risks + mitigations

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R-1 | Drafts and in-flight saves reset when tabs become components | high if done naively | data loss (typed API key gone) | S02 puts all drafts in `SettingsCtx` owned by the shell; S02 verify types a draft and switches tabs |
| R-2 | Kit controls set `font` shorthand, so `[data-a11y-font="lexend"] button` (`app.css:747-756`, specificity 0,1,1) loses to scoped `[data-slot] button.svelte-x` | medium | dyslexia mode stops reaching Settings controls | §10 step 15; fix in the kit (`font-family: inherit` on atoms) as a separate kit commit, not in Settings CSS |
| R-3 | `.sflash` zeroed by `[data-settled]` after a cross-tab jump | certain without A-6 | jump lands with no orientation cue | A-6 exemption line; §10 step 9 |
| R-4 | bits-ui Tabs regenerates ids or renders all panels | low | D-33 ids break, extra mounts | ids passed explicitly (K21b); `{#if}` inside each Content guarantees one mounted tab |
| R-5 | Segmented has no "no selection" mode | medium | Custom mix shows a wrong active option | S00c; dev/ui exhibit |
| R-6 | Native slider + bubble under page zoom (`setZoom`) mis-centres (map 06 §9) | medium | cosmetic | §10 step 13 at 100% and 150% |
| R-7 | Owner misses the accent fill on Save buttons | medium | taste regression | board 06 `data-prop-primary` lever before merge |
| R-8 | Geist widths push the tab row + 210px search past 820 | low (~700px estimated) | wrap at full width | §10 step 16 plus a full-width shot; fall back to icons-hidden at ≤1000 |
| R-9 | SearchField lacks input passthrough | medium | combobox a11y lost | S00b |
| R-10 | Accordion inside Card breaks `scrollIntoView` target (content still collapsing) | low | jump lands slightly off | `block:"start"` targets the card top, which does not move as the card grows |
| R-11 | Store-level native confirm still pops from "Update now" | certain | mixed dialog styles | named in §7; 4f owns it |
| R-12 | `--page-col` published only on PageHeader's host misses the body | certain without A-16 | two edges again | set on `.sb-main` |

## 12. Effort + dependencies

**L: 5-6 sessions, 35 commits (+3 kit gaps if P3 left them, +2 gated).**

| Block | Commits | Sessions |
|---|---|---|
| 6.1 kit gaps | 0-3 | 0.25 |
| A page model | 2 | 0.75 |
| B parts | 13 | 1.5 |
| C shell | 4 | 1 |
| D tabs | 16 | 2 |
| verification + board numbers | — | 0.5 |

Dependencies: P1, P2, P3 merged into `design/halo` (D-27). P0 spike 2 result. Board 06 for the levers (the build proceeds on the defaults; board numbers are one-line edits). Not blocked by 4a; tooltips paint only after 4a mounts the layer (until then the a11y path holds). Parallel-safe with 4b/4c/4d/4f except `app.css` (`.slider-bubble` block only) and `motion.css` (one line).

## 13. Amendments proposed to 00-decisions (and specs 01-03)

| # | Amends | Proposal | Reason |
|---|---|---|---|
| A-1 | 02-kit Slider "Retires: Settings range inputs" vs K23b | Settings keeps native `<input type=range>` on a Rift `RangeControl` styled from Halo's own native slider (`bundle.css:177-186`) with `sliderBubble`; bits-ui Slider is for new surfaces | K23b says the bubble needs a native input; D-33 keeps the `thumbW` coupling; 03-motion #140 keeps the bubble |
| A-2 | 02-kit §2.4b "native confirm() (4f)"; D-20 | Settings' two `confirm()` sites (`SP:337,354`) move in 4e via `SettingsConfirm`; store-level ones (`cliUpdate.svelte.ts:432`, `updates.svelte.ts:287`) stay 4f | task scope; Settings owns its dialogs |
| A-3 | 02-kit K21b Tabs list `gap-5` | `gap-6` (24px) | `bundle.css:259` uses `--space-5` = 24px; 02-kit §1.5 E itself maps Halo 24 to Tailwind `6` |
| A-4 | 02-kit K22b "Retires `.rift-toggle` (AssistantWelcome + SettingsPage)" | SettingsPage is the only consumer | the AssistantWelcome hits are `.newrift-toggle` (`AssistantWelcome.svelte:243,538`) |
| A-5 | 02-kit K22 SearchField API | forward unknown props to the `<input>` (role, `aria-*`, `onkeydown`, `onfocus`, `onblur`, `spellcheck`) | a combobox needs its attributes on the input |
| A-6 | 03-motion §2.2 D-29 block | add `[data-settled] .card.sflash { animation-duration: 1.6s !important; }` | a cross-tab jump mounts the target inside the settle window; the flash is orientation (03-motion #143 keeps it) |
| A-7 | 02-kit K21 Segmented | `value` outside the options → no checked option, thumb hidden, first enabled option tabbable | Process detail "Custom mix" (`SP:824-828`) |
| A-8 | D-31 "<details> becomes Accordion"; 02-kit Accordion `cards` (r12) | collapsible Settings cards are `Card` (r16) + a single-item `Accordion` (rows variant, trigger restyled as the card title) | keeps one card radius in the column (D-07 nesting 16 → 12 → 8) and keeps `#card-*` + `.card` on the card |
| A-9 | logic boundary ("restyle, don't rewrite") | `asst*Msg` / `openAiApiKeyMsg` become `{ text, tone }` | SettingsNote needs a tone; strings unchanged |
| A-10 | D-31 / D-12 | the Whisper build steps (inside a Banner) use Banner `action` + Collapse | Banner has no children slot; same motion carrier |
| A-11 | D-13 / map 09 I3 | the one-shot `pill-ping` on probe is removed, not ported to LiveDot | the "checked Ns ago" stamp already reports a fresh probe; idle Settings must run 0 animations (03-motion §4.7) |
| A-12 | D-20 "always portalled" | the search results popup stays in-page (absolute, z 65) | focus stays in the input (`aria-activedescendant`); a portal needs `use:portal` (+1 on the `overlay:use-portal` ratchet); nothing clips it |
| A-13 | D-02 "≤1 CTA per view" | applied per Settings tab, table §2.8 | 8 primaries today (map 06 #10) |
| A-14 | shortlist #6 legacy deletion | the Stream-view toggle and classic preview go out inside 4b's legacy commit, which must also force `streamMode` on | removing only the toggle would strand users stored on `false` |
| A-15 | spec 01 C14 text | "stored value `inter`" → `"system"` | `accessibility.svelte.ts:9,19` |
| A-16 | 02-kit K28 "`--page-col` published on the host" | Settings sets `--page-col` on `.sb-main` (common ancestor of header and body) | the body is a sibling of PageHeader, so a var on the host never reaches it |

## 14. Open questions (default in bold; the build proceeds on the default)

1. Card title step: **heading 16/22** (Halo's docs say subtitle 18/24 for settings groups). Board lever.
2. Status-pill one-shot ping on probe: **removed** (A-11).
3. Spoken language: **Select** (S29, droppable) vs the card grid.
4. ChatGPT plan-limit tones: **accent < 60%, warn < 85%, danger ≥ 85%** (same thresholds as `AiHealthPage.svelte:204-206`; 4f may extract a shared helper).
5. Stack chip "OKLCH": **replace with "Halo"**.
6. Auth pill red: **attention (warn)** as today, not danger.
7. Active tab icon: **ink** (accent stays on the underline only).
8. Previews: **keep all five** (the accent preview moves inside "Custom accent").
9. Vocabulary budget: **soft counter, no hard cap** (today has no cap).
10. Save/Connect buttons: **secondary; one primary per tab** (§2.8).
11. Local tools copy: **drop "hover one to see what it does"**; tooltips stay as an enhancement.
12. Reset buttons: **ghost, left-aligned under the last card**.
13. Results popup width: **360** (today 330).
14. Tabs ArrowUp/ArrowDown (today also move tabs, `SP:234-235`): **dropped** (bits-ui horizontal Left/Right/Home/End).
15. Electric blue row when custom: **chip shows the custom colour, button "Use Electric blue"**.
16. "Works with Claude + ChatGPT": **plain caption, no Claude-brand tint**.

## Map corrections recorded

- M-1 map 06 §1/§10 counts 16 toggles (+1 decorative); SP has **15** functional `class="rift-toggle"` + 1 decorative (`:746`).
- M-2 02-kit K22b names AssistantWelcome as a `.rift-toggle` consumer; those hits are `.newrift-toggle`.
- M-3 spec 01 C14 says the stored font value is `inter`; it is `"system"` (`accessibility.svelte.ts:9`).
- M-4 02-kit K21b Tabs `gap-5` is 20px, not Halo's 24px (`bundle.css:259`).
- M-5 map 06 §3 misses a second status-banner bug: `SP:969` shows `CircleCheck` whenever `auth` exists, including yellow/red (warn) states.
- M-6 map 06 §3 misses that `copyDiagnostic` swallows clipboard failure (`SP:316`, console only), against D-32.
- M-7 `RESET_COPY.appearance` (`SP:323`) still promises "the stock emerald look", stale after P1.
- M-8 `check-ui.mjs:108` scans `.svelte` only at 22727d24, and `c.sh` has no `media` verb (P2 M01/M03 add both); §10 assumes P2.




