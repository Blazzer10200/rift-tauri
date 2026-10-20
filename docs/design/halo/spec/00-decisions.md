---
title: Rift · Halo — decision register
status: draft-for-owner
updated: 2026-10-19
tags: [rift, halo, redesign, spec]
branch: design/chat-ledger @ 22727d24 (Ledger 8bb71f4f below it)
inputs: docs/design/halo/map/01-12 · spec/drafts/decisions-{halo-fidelity,owner-taste,engineering-risk}.md
---

# Rift · Halo — decision register

## How to read this

1. Owner: read only the shortlist. Each item is one yes/no. Work carries on with the recommended answer until you say otherwise.
2. Builders: the register table is the contract. Each `D-xx` section below it gives the options, the reasons, the exact values and which lens disagreed.
3. Ties were broken in this order: the owner's words this session, then hard technical limits, then recorded owner rejections, then Halo fidelity, then engineering convenience. `map NN §x` = `docs/design/halo/map/`. Owner IDs (L2, D1, C13…) are rows in map 12.

## Owner sign-off shortlist

Board 00 opens with one open question that comes before everything else: **what felt "rough" in the first Halo mock?** The answer can change items 1, 2 and 4.

1. **Frame.** Keep Rift's two islands (sidebar and main) but make them flat in Halo's colours? The alternative is Halo's flat docked frame.
   - Recommended: yes, keep the flat islands. Canvas `#08090c`, islands `#0e1014`, a 1px hairline, radius 16. The gradients, grain, bevel, cavity and accent seam are removed.
   - If no: the shell becomes Halo's flat frame, with the sidebar sitting on the canvas behind a border and a sticky top bar. That is about a day of shell CSS and top-bar markup. Only phase 4a waits.
2. **Size and density.** Make compact the default: chat text 14px, UI text 13px, controls 32px tall? Halo's own sizes would become the "comfy" setting.
   - Recommended: yes. It matches your ConsoleX calm pass (body 14, 32px icon buttons) and "denser and quieter".
   - If no: Halo's sizes become the default (15px prose, 36-44px controls). One token block flips, and each surface that has already been ported gets re-fitted.
3. **Streaming feel.** Should words fade in by opacity only, with no blur, at today's speed (30ms per word, 320ms catch-up, follow 0.35)? Should standard motion use Halo's 300ms instead of Rift's 240ms?
   - Recommended: decide by playing the motion-lab board. Until then each of these defaults sits in its own commit, unmerged.
   - If no: the values you pick in the lab replace them, one number per commit.
4. **Composer layout.** Keep the control bar below the text box, as it is today? The alternative is the controls inside one card, like the Halo mock and ConsoleX.
   - Recommended: keep the bar below for now. It is the cheapest option, and the hero-to-docked animation depends on it. Board 03 shows both layouts side by side.
   - If no: the composer becomes one card with the controls inside it. Phase 4c then re-measures the animation heights, which takes about 1-2 days.
5. **Accent dials.** Electric blue is now fixed. Retire the hue and vividness dials and the 8 swatches, in Settings and in onboarding step 1?
   - Recommended: yes.
   - If no: they stay as an opt-in "Custom accent", and blue stays the default.
6. **Deletions.** After each phase lands, may I delete these, one commit each?
   - the old assistant render path (the MessageBubble assistant branch and the Stream-view toggle);
   - the `design-system/` static cards, token mirror and manifest;
   - the dead Diagnostics modal mode;
   - the unused nav `kbd` field;
   - the dead CSS the maps list.

   May I also mark the published "Rift Style" design-system artifact as superseded?
   - Recommended: yes.
   - If no: they stay in place, frozen in the old look, and nothing new depends on them.
7. **One paid test turn.** Allow one short real turn on the cheapest model, to check scroll-follow in the real chat pane?
   - Recommended: yes, once.
   - If no: a dev-only fake stream is the only check, and scroll-follow is reported as "not verified on a real turn".

Pushing, and merging `design/halo` into `main`, are always asked at the time. They are not on this list.

## Decision register

| ID | Topic | Decision | Key values | Reversibility | Gate? | Unblocks |
|---|---|---|---|---|---|---|
| D-01 | Frame (T1) | Flat islands in Halo material | canvas `bg-000` #08090c · islands `bg-100` #0e1014 + 1px `line-100` · r16 · gutter 12 | medium (~1 day to Halo frame) | yes (#1) | P4a, toast anchor, page entrance |
| D-02 | Accent (T2) | Fixed Electric blue, exact hex; dials only as opt-in custom | #6aa8ff · ink #08090c · soft 14% · glow 28% · keys `.v2` | tokens cheap; dial removal costly | yes (#5) | P1, Settings Appearance, onboarding 1 |
| D-03 | Semantic colour (T3) | No green; info is light blue | info #9cc6ff · ok → accent · warn #ff9f5a (also cost) · danger #ff6b6b · chart-2 #b79cff | cheap | no | every tone, diff, ANSI |
| D-04 | Light themes (T4) | Out of scope | dark `blue` only · `color-mix` derivatives | cheap | no | P1 scope |
| D-05 | Fonts (T5) | Geist + Geist Mono via fontsource | `'Geist Variable'` · `'Geist Mono Variable'` · Lexend = dyslexia + splash | cheap until P5 | no | every text measurement |
| D-06 | Type + density (T6) | Compact default; Halo sizes = comfy tier | chat 14/22 · UI 13/18 · control 32 · row 32 · column 760 | tokens cheap; surfaces re-fit | yes (#2) | P3 sizing, every surface |
| D-07 | Radius (T7) | Halo ladder + one 6px step | 6 · 8 · 12 · 16 · pill · buttons never pill | cheap | no | P1, P3 |
| D-08 | Tokens + gates (T8) | ConsoleX alias layer, Rift names kept | fontsource → `halo/tokens.css` → `app.css` · z 1/10/20/50/60/65/70/100 | cheap per value | card deletion (#6) | everything |
| D-09 | Components (T9) | One kit in `components/ui/` | shadcn-svelte `add`, never `init` · `data-[state=open]:` | medium | no | P3, overlays, Settings |
| D-10 | Motion tokens (T10) | Halo durations, one curve, three rises | 150/300/600 · stagger 60 · `(.2,.8,.2,1)` · rises 10/6/3px · page 520 +60 | cheap | 240 vs 300 in #3 | P2 |
| D-11 | Streaming (T11) | Mechanism fixes ship; numbers from motion lab | 30ms/word · catch-up 11 words · lerp .35@60Hz · fade 300ms opacity | cheap per constant | yes (#3, #7) | P4b |
| D-12 | Turn lifecycle (T12) | Same-node fold, no remount | collapse 300 grid-rows · receipt fade 150 +60 · chevron-down 180° | medium | no | P4b |
| D-13 | Loops + reduced motion (T13) | Real activity only; one live-verb shim | LiveDot 1.8s · Spinner 900ms · Progress 1.3s · global net + Spinner exempt | cheap | no | P2 |
| D-14 | Skeleton (T14) | Static bones + Halo hand-off | `bg-200` r6 · after 300ms · out 360 / in 600 + 6px | cheap | no | loading states |
| D-15 | Blur (T15) | Zero `backdrop-filter` | floats opaque · blur only skeleton + splash | n/a (defect) | no | P2 |
| D-16 | Halo extras (T16) | Signal-bearing extras only | spotlight/magnetic/dim off · press .97 · count-up 700 · roll 900 | cheap | no | P3, hubs |
| D-17 | Tooltips (T17) | Revive behind `use:tooltip` | singleton · inverted bubble · 400ms cold / 120ms warm | cheap | no | icon toolbars, exact numbers |
| D-18 | Syntax colour (T18) | Muted Halo-palette Shiki theme | ink · keywords #9cc6ff · strings #b79cff · comments `ink-faint` · no accent | cheap (one theme) | no (board 02) | P4b code |
| D-19 | Navigation (T19) | No tab strip; Rift sidebar behaviour, Halo NavItem look | collapse = one 300ms move · "Home" everywhere · Ctrl+1 Home | cheap | no | P4a |
| D-20 | Overlays (T20) | bits-ui + Halo motion; Rift placement + z order | dialog .94/8px 300 · menu pop 150 · toasts top-centre · browser hide via existing IPC | medium | no | P3, P4f, palette |
| D-21 | Chat anatomy (T21) | Ledger frozen; restyle only | receipt sans tabular · bubble `bg-200` 16 16 4 16 · tiles `bg-100` r12 head 32 | cheap | legacy deletion (#6) | P4b |
| D-22 | Composer (T22) | Bar below the well (today), on Halo tokens | well `bg-100` + `line-300` r12 · Send 32 r8 · Esc stops | costly to flip | yes (#4) | P4c |
| D-23 | Scrollbars (T23) | Halo thin; today's hidden sites stay hidden | 10px · thumb `line-200` · hover `ink-faint` · chat stream hidden | cheap | no | P1 |
| D-24 | Data viz (T24) | Halo Stat anatomy, Rift formatter + dense bars | metric 28/32 mono · bars `bg-300`, peak accent · series accent/violet/light blue | cheap | no | P4d, P4f |
| D-25 | Process (T25) | Canvas boards first; `design/halo` off 22727d24 | 9 boards · one concern per commit · no push or main-merge without the owner | n/a | no | sequencing |
| D-26 | Splash + brand (T26) | Keep mark + seam splash, retuned | panels `bg-000` · doors `(.65,0,.35,1)` · no caret blink, no exit blur | cheap | no | P1 |
| D-27 | Sequencing spine (added) | P0 → P5 phases | spikes · tokens · motion · primitives · surfaces 4a-f · cleanup | n/a | no | everything |
| D-28 | Phase-0 spikes (added) | Nine checks before building on them | see section | n/a | no | D-05, D-09, D-12, D-17 |
| D-29 | Entrance replay (added) | Entrances only for live-born content | `data-settled` → 0s durations for ~450ms | cheap | no | P2, before P4b |
| D-30 | Text ladder, focus, a11y (added) | 3 text steps + Halo focus ring | `fg-2` → ink · `fg-subtle` → ink-faint · `shadow-focus` 2+2px | cheap | no | every port |
| D-31 | Page shell + SettingsRow (added) | One PageHeader, one column edge | title 24/30 · Settings 820 · hubs 1080 · row min 44 | cheap-medium | no | P4d-f |
| D-32 | Copy, icons, hit targets (added) | Halo content rules | sentence case · lucide 1.8 at 16/18px · targets ≥28px | cheap | no | P5 sweep |
| D-33 | Contracts + couplings (added) | Keep hook names; tie JS geometry to CSS vars | rename in same commit · `ROW_H`, `thumbW`, mic `rx`, `composerH` | cheap | no | every port |
| D-34 | Native + pre-JS colour (added) | Move with tokens in P1 | `app.html` #08090c · `Color(8,9,12,255)` | cheap | no | P1 |

## Decisions

### D-01 · Frame / material (T1)
- **Options:** (A) flat islands: Rift's docked two-island structure in flat Halo material. (B) Halo flat docked frame: the sidebar on `bg-000` with a `border-right`, a sticky top bar, no gutters. (C) The onyx slab, recoloured.
- **Decision:** A.
  - Canvas `bg-000` #08090c. Sidebar and main islands `bg-100` #0e1014 with a 1px `line-100` #1f242c border, `--radius-lg` 16, 12px gutter, one level deep (canvas → island → content).
  - Delete the vertical gradient fills, the 4 SVG grain layers, the bevel top catch, the cavity insets, the contact AO, the shell `.rift-seam`, the `.app` 1s background transition, and the page-local washes (onboarding radials and dot grid, welcome aura, hub card washes).
  - The work lives in `AppShell.svelte:339-512` and `Sidebar.svelte:255-284`, CSS only.
- **Why:**
  - The owner asked for a "happy medium for literally everything". Option A sits between the slab and Halo's flat frame.
  - The owner iterated the island structure over 5 sessions (L1, L4, L5, L6).
  - Three sources agree the material should go flat: C7 "shit stacked on top of shit", the C8 grain retirement, and Halo's "no colour washes" (map 10 §4). Grain only existed to stop the gradients banding, so it goes with them.
  - Map 12 X2 / C6 sets a rule: don't ship flat without the owner seeing it beside the old look. Hence board 01, A beside B.
- **Dissent:**
  - The fidelity lens wanted B (Halo principle 1, the first mock, ConsoleX shipped flat).
  - Map 12 X2 suggested keeping the AO, top catch and seam. They were dropped because they are the slab recipe and the gutter already separates the islands. The seam survives in the splash (D-26).
- **Reversibility:** A → B is about 1 day (shell CSS + Topbar markup). A → C is a git revert of 2 files. **Gate:** yes, shortlist #1. P4a builds A; its merge waits on board 01. **Unblocks:** P4a, the toast anchor (D-20), the page-entrance target (D-10), first paint (D-34).

### D-02 · Accent model + saved prefs (T2)
- **Options:**
  - (a) Fixed `blue` preset and remove the dials.
  - (b) Fixed blue on the default path; the dials stay only as an opt-in custom accent.
  - (c) A Halo preset picker (lime, blue, ember, violet).
- **Decision:** b now, and a once the owner says yes (shortlist #5).
  - `<html data-theme="blue">`. With no custom accent, `--accent` is literally #6aa8ff, `--accent-ink` #08090c.
  - Derivatives always use `color-mix(in srgb, var(--accent) N%, transparent)`: `accent-soft` 14%, `glow-strong` 28%. Never rgb triplets.
  - `--accent-hover` and `--accent-active` alias to `var(--accent)` (Halo has no shades; hover = surface step or glow).
  - The oklch derivation (`app.css:171-180`) runs only under `[data-accent="custom"]`. Touching a dial sets it.
  - Storage keys `rift.ui.accent.v1` / `vividness.v1` → `.v2`. The v1 values are ignored, not deleted.
  - The first swatch is Electric blue (exact). The 13 `--accent-h` literal consumers (map 08 §4.1) become `color-mix` of `--accent`.
  - Solid accent stays rare: Send plus at most one CTA per view.
- **Why:**
  - "Electric blue is the one. Use it as is." is priority 1, so the default is the exact hex. The oklch dial cannot hit #6aa8ff.
  - The dials are owner-built (C17, S9), so removing them is a deletion and needs a yes.
  - Fixed hex keeps Halo's measured contrast table valid (map 10 §1.1).
- **Dissent:** the fidelity and taste lenses want the dials removed immediately. The engineering lens wants them kept permanently.
- **Reversibility:** tokens are cheap. Removing the UI (Settings card, onboarding swatch strip, `/dev/ui` accent bar) is costly-ish. **Gate:** yes for the dial removal only. **Unblocks:** P1, Settings Appearance (4e), onboarding step 1 (4d).

### D-03 · Semantic colours on blue (T3)
- **Options:** keep Rift's greens · Halo literal (positive = accent, info = lime #c6f35a) · the ConsoleX model (no green, info overridden to light blue).
- **Decision:** the ConsoleX model, as aliases in one deviation block.
  - **Info:** `:root[data-theme="blue"]{--info:#9cc6ff;--info-soft:rgba(156,198,255,.14);--chart-2:#b79cff;--chart-3:#9cc6ff}`.
  - **OK:** `--ok`/`--ok-soft` → `accent`/`accent-soft`. The name is kept (101 refs). Success always carries an icon or a word.
  - **Activity:** `--status-busy` → accent through LiveDot. `--status-pending` → `ink-muted` idle dot. `--status-warn` → `warn`.
  - **Warn:** `--warn` #ff9f5a covers caution AND "costs money" (C3, shown at the control).
  - **Danger:** `--danger` #ff6b6b.
  - **Modes:** plan mode → `info`. Auto-edit → accent.
  - **Logs:** the log level "info" → a neutral Badge (`bg-300`/`ink-muted`), which keeps logs quiet.
  - **Diff:** add = `accent-soft` row wash + accent `+`. Delete = `color-mix(in srgb, var(--danger) 8%, transparent)` wash + danger `−`. Word emphasis is the same tone at 28%. `dem-add`/`dem-del` class names stay (they are test-coupled).
  - **ANSI:** green → accent, red → danger, yellow → warn, blue/cyan → info, magenta → #b79cff, others → `ink-muted`.
  - **Deltas:** up = accent, down = warn. A `goodWhen` flag flips cost metrics, so spend going up shows warn.
  - The `--dc-*` private palette in DiagnosticsConsole becomes status tokens.
- **Why:**
  - The owner's newest colour call is "No green anywhere" (ConsoleX, 2026-10-17), made inside a Halo-blue app.
  - Lime info on blue reads as a second accent, and C2 calls a second decorative hue a bug.
  - Halo says positive = accent (map 10 §4.4).
  - C3 has "huge gap" stakes, and Halo's warn is in the same amber family.
- **Dissent:** the fidelity and engineering lenses map pending → `info`. That is rejected because accent blue next to light-blue info is the closest pair on screen; `ink-muted` keeps waiting states quiet. Strictly, "Use it as is" includes lime info; the override follows the owner's own ConsoleX practice.
- **Reversibility:** cheap (alias lines). **Gate:** no; board 02 shows diff and ANSI. **Unblocks:** P1, every badge, pill, toast and diff tone.

### D-04 · Light themes (T4)
- **Options:** in scope now · a later wave · never.
- **Decision:** out of scope. Ship dark `blue` only, with `color-scheme: dark` and no theme switch.
  - Vendor the generated `tokens.css` whole. Its light blocks are inert without their `data-theme`, and this keeps "never hand-edit" and the freshness check simple.
  - The `color-mix` discipline (D-02) keeps light possible later.
- **Why:** "Dark direction only" (C13, perpetual). The codebase assumes dark: 131 oklch literals, 93 gradients, black shadows, a dark Shiki theme (map 08 R9).
- **Dissent:** the fidelity and taste lenses would vendor only the dark and blue blocks. That is rejected because it means hand-editing a generated file.
- **Reversibility:** additive later. **Gate:** no. **Unblocks:** removes about 30% of P1 risk.

### D-05 · Fonts (T5)
- **Options:** Geist + Geist Mono everywhere · Geist for UI + JetBrains Mono for code · keep Inter/JetBrains on the Halo scale.
- **Decision:**
  - **Loading:** `@fontsource-variable/geist` + `@fontsource-variable/geist-mono` (^5.3.0, the ConsoleX pin), imported in `+layout.svelte` before `halo/tokens.css` and `app.css`. Also import the italic CSS if the package ships it (D-28), otherwise markdown emphasis gets a synthesized slant.
  - **Families:** `--font-sans`/`--font-ui` = `'Geist Variable', ui-sans-serif, system-ui, 'Segoe UI', sans-serif`. `--font-mono` = `'Geist Mono Variable', ui-monospace, 'Cascadia Mono', monospace`. These are the fontsource family names, not Halo's `"Geist"`. Strip Halo's Google `@import`.
  - **Drop:** Inter's `font-feature-settings` (`app.css:306`) and the body `-0.005em` (`app.css:314`, `AppShell:358`).
  - **Mono role:** mono for code, paths, IDs, Kbd, SectionLabel, Stat metric, table data. Sans with `tabular-nums` for timestamps, receipt facts and status-bar counts (T4 / C13: mono in chrome meta is a defect).
  - **Lexend:** only for dyslexia mode (`app.css:747-756`, kept last in the cascade) and the splash wordmark. The greeting moves to Geist. The Settings font label "Inter" becomes "Geist".
  - **Ligatures:** keep the `--code-liga` pref only if Geist Mono has ligatures. Otherwise hide the toggle and keep the stored value.
  - Remove the Inter and JetBrains packages in P5, not P1, to keep a cheap rollback window.
- **Why:** Geist is Halo's face. CSP `font-src 'self' data:` (`tauri.conf.json:28`) and offline-first rule out Google Fonts. The owner never asked for Inter itself; it was a Graphite rule.
- **Dissent:** the fidelity lens wanted Lexend out of the splash and mono for receipt numbers. Overruled by T4/C13 (priority 3) and the splash brand moment (D-26).
- **Reversibility:** cheap until P5. **Gate:** no; board 00 shows it. **Unblocks:** every text measurement and the truncation checks (34px rows, dock labels, status bar).

### D-06 · Type scale + density (T6)
- **Options:** Halo native (15 body, 44 controls, `regular` default) · Rift today (13 body, 26 controls) · a compact Halo tier using the owner's ConsoleX precedent.
- **Decision:** compact is the default and all three density tiers stay. Everything below is a token, so every number is a one-place change.
  - Type tokens (`--t-*`). Weights 400/500/600 only. No `display`.

    | Token | Size/line | Notes |
    |---|---|---|
    | `label` | 11/14 | mono 500, .12em, uppercase; SectionLabel and Kbd only |
    | `caption` | 12/16 | |
    | `small` | 13/18 | UI default |
    | `chat` | 14/22 | |
    | `body` | 15/24 | |
    | `heading` | 16/22 | 600 |
    | `subtitle` | 18/24 | 600 |
    | `title` | 24/30 | 600, −0.02em |
    | `data` | 13/18 | mono 500, tabular |
    | `metric` | 28/32 | mono 600, −0.03em |

  - Density tiers:

    | | compact (default) | regular | comfy |
    |---|---|---|---|
    | Chat prose | 14/22 | 15/24 | 16/26 |
    | Control (button, input, select, segmented, icon button) | 32 | 36 | 44 |
    | Dense-toolbar control (pane head, tile actions, browser bar) | 28 | 32 | 36 |
    | List / menu / nav / sidebar row | 32 | 36 | 40 |
    | Stream tile head | 32 | 36 | 40 |
    | Toggle | 36×20 | 40×22 | 44×26 |
    | Checkbox | 16 | 18 | 20 |
    | Card padding | 16 | 20 | 24 |

  - Chat column `--chat-col: min(760px, 100% - 48px)`, shared by messages, composer, alerts, receipt, jump pill and turn rail. This fixes the 844/880/1100 mismatch (map 03 §2).
  - Floors: 12px sans; 11px only for mono label and Kbd. The 261 sub-12px declarations get lifted. Icon-button hit floor is 28px.
  - Add a `font-size:Npx` literal ratchet to `check-ui.mjs` (baseline 380, map 08 §1.5) that also scans `.css` files.
- **Why:**
  - D1 "denser and quieter, not bigger and louder".
  - D5 rejected both "way too small" and "big and funky".
  - The owner asked for a "happy medium".
  - The ConsoleX calm pass landed at body 14, icon buttons 32, `.btn` 40, and the owner called it "cleaner… no clutter".
  - Halo's 44/15 overwhelms a 900×600 minimum window (map 08 R6).
  - 760px keeps prose at an ADHD-friendly line length.
- **Dissent:** the fidelity lens wanted Halo native as default (15/24 prose, 44 page / 36 toolbar controls, title 32, metric 30). The engineering lens wanted 30px controls; 32 was chosen for ConsoleX parity and hit targets.
- **Reversibility:** token values are cheap; surface fits (hub no-scroll, Settings rows, FLIP heights, `ROW_H`) need re-tuning. **Gate:** yes, shortlist #2, board 00. **Unblocks:** P3 primitive sizing, every surface, D-33 couplings.

### D-07 · Radius ladder + pills (T7)
- **Options:** Halo 8/12/16/pill as-is · keep Rift's 4-16 ladder · Halo plus one small step.
- **Decision:**
  - **Ladder:**
    - `--radius-xs` 6: kbd, inline code, checkbox, skeleton bone.
    - `--radius-sm` 8: controls up to 32px tall (incl. Send), menu rows, inner tiles, thumbs.
    - `--radius-md` 12: controls 36px and taller, popovers, menus, toasts, stream tiles, CodeBlock, accordion.
    - `--radius-lg` 16: islands, cards, dialogs, drawers, palette, user bubble (`16 16 4 16`).
    - `--radius-pill` 999: badges, count/status chips, toggle track, progress track.
  - **No pill buttons:** `.jump-latest`, `.chip-btn`, `.ps-chip` and `.char-count` drop to 8.
  - **Nesting:** concentric 16 → 12 → 8.
  - **Collision:** the 15 sites that use Rift `--radius-lg` (10, meaning "medium") are renamed to `--radius-md` in P1, before Halo's 16 takes the name.
  - **Off-ladder radii:** 2, 3, 4, 5, 7, 9, 10, 11, 13 and 14 die in each surface sweep.
- **Why:** K1 and Halo agree: no pill buttons or inputs. A radius of 12 on a 28-32px button reads pill-like (X25). Surfaces carry 13+ radii today.
- **Dissent:** the taste lens kept `.jump-latest` as a pill float. Overruled by K1 (priority 3) plus Halo.
- **Reversibility:** cheap. **Gate:** no. **Unblocks:** P1, P3.

### D-08 · Token architecture, gates, design-system/ (T8)
- **Options:** (a) the ConsoleX alias layer · (b) rename consumers to Halo names · (c) alias now, migrate later.
- **Decision:** (a), permanent.
  - **Load order (contract):** fontsource → `src/lib/styles/halo/tokens.css` (vendored, generated from `tokens.json`, never hand-edited) → `app.css`. Rift overrides that must beat `[data-theme="blue"]` use `:root[data-theme="blue"]`.
  - **Collisions (17 names, map 08 §1.3):**
    - Halo wins: `--accent*`, `--warn*`, `--danger*`, `--scrim`, `--radius-sm`, `--radius-lg` (after the D-07 rename), `--dur-fast`, `--dur-base`, `--dur-slow`, `--stagger`, `--ease-spring`.
    - One commented "Rift deviations" block holds the rest: `--info*` and `--chart-2/3` (D-03), `--font-sans`/`--font-mono` (D-05), and `--z-dropdown` 20 → 65.
  - **Aliases:**
    - Surfaces: `--bg`, `--bg-inset` → `bg-000`. `--surface`, `--bg-elev-1`, `--field` → `bg-100`. `--surface-hover`, `--bg-elev-2` → `bg-200`. `--surface-active`, `--bg-elev-3`, `--track` → `bg-300`.
    - Lines: `--border` → `line-100`. `--border-strong`, `--ghost-border` → `line-200`. `--field-border` → `line-300`.
    - Text: `--fg`, `--fg-2` → `ink`. `--fg-muted` → `ink-muted`. `--fg-subtle`, `--fg-faint` → `ink-faint` (D-30).
    - Accent: `--accent-fg` → `accent-ink`.
    - Focus and shadows: `--ring` → `--shadow-focus: 0 0 0 2px var(--bg-100), 0 0 0 4px var(--accent)`. `--shadow-lg`, `--shadow-float` → `shadow-pop`. `--shadow`, `--shadow-sm` → `none`.
    - Motion: `--ease-page`, `--ease-soft` → `ease-out`. `--dur-rise` → `dur-slow`.
  - **Tailwind bridge:** every `@theme inline` KEY stays (`bg-surface`, `text-fg-2`, `rounded-card`, `shadow-float`, `ease-page`, `h-row`…). A new token used as a class inside `ui/` (`--t-*`, `--radius-xs`, z names) needs an `@theme` entry and a `createCn` registration (`utils.ts:13-17`), or `check:ui` hard-fails.
  - **z ladder:** raised 1 · sticky 10 · hud 20 · overlay (dialog, drawer, palette) 50 · toast 60 · dropdown/popover 65 · tooltip 70 · splash 100. `--z-dialog` → overlay and `--z-popover` → dropdown. All 25 raw z literals get tokenised.
  - **`check:tokens`:** rewrite it to resolve through vendored `tokens.css` + the alias block. It must FAIL on three things:
    - a collided name redeclared outside the deviation block;
    - an alias that points to an undefined var;
    - `tokens.css` stale against `tokens.json`.

    It stops mirroring `design-system/styles.css`. Never make it green by hand-editing the stale mirror.
  - **`check:ui`:** the contract is unchanged and `--update` runs after each phase that lowers counts. New metrics: `backdrop-filter` (target 0), `font-size:Npx`, raw `cubic-bezier(`. Scan `.css` too.
  - **New vitest:** every Rift token resolves; no raw hex outside the deviation block; z ladder strictly increasing.
  - **`design-system/DESIGN.md`:** rewritten in P1 as "Rift on Halo". §8 is kept verbatim and each rule is tagged binding or retired (see "Superseded rules").
  - **Cards, mirror, manifest, Rift Style artifact:** untouched until shortlist #6.
- **Why:** 4,806 external `var()` refs (map 08 §0.1) make (b) a churn of ~63 files for zero visual gain. (a) is proven in ConsoleX ("restyle, don't rebuild"). The alias layer is also what makes D-02, D-03, D-07 and D-10 one-line reversible.
- **Dissent:** the fidelity lens would delete all 17 Rift declarations so Halo values win outright, including `--info` lime. The taste and engineering lenses differ only on which overrides survive. Settled per D-03, D-05 and D-10.
- **Reversibility:** cheap per value, costly as a whole. **Gate:** only for deleting cards/artifact (#6). **Unblocks:** everything; P1 critical path.

### D-09 · Component strategy (T9)
- **Options:** (a) restyle `components/ui/` + `recipes.ts` and add the missing bits-ui parts there · (b) a new `components/halo/` kit · (c) both.
- **Decision:** (a).
  - **bits-ui parts:** add with `npx shadcn-svelte@latest add <name>` (never `init`; it rewrites `app.css`): dialog, popover, tabs, accordion, switch, checkbox, radio-group, progress, slider, combobox, command. Then give them the Halo look in `recipes.ts`.
  - **CSS-only atoms:** small `ui/<name>.svelte` with scoped CSS on tokens: Button, Card, Badge, Kbd, LiveDot, Spinner, Divider, SectionLabel, EmptyState, Banner, Skeleton, Segmented, SearchField, Stat.
  - **Behaviour:** reveal, follow, count-up, RollingNumber, FlipList and the sliding indicator go in `src/lib/motion/` (runes) and `src/lib/actions/`.
  - **Rift composites live with their surface:** tiles, DiffBlock, TurnRail, SettingsRow, PreviewWell, SecretField, ShareBar, BrowserBar.
  - **First P3 fix:** `recipes.ts:20,58,60` uses `data-open:`/`data-closed:` variants, which bits-ui likely never emits. Switch to `data-[state=open]:`/`data-[state=closed]:` once D-28 confirms they are dead.
  - **Old dialects retire as surfaces port:** `.btn`/`.pill`/`.input`, `.st-*`, `.ob-*`, `.sxa-btn`, `.rift-menu*`.
- **Why:** `check:ui` exempts `ui/` and requires its classes to generate CSS. A parallel kit would be a fifth button dialect (map 08 §2.3) and would re-implement focus and keyboard handling that bits-ui already gives. Halo's React bundle cannot mount in Svelte (map 11 §1.1).
- **Dissent:** the fidelity lens wanted one global `halo.css` with Halo's `.halo-*` classes verbatim, for a 1:1 diff against the reference. Rejected: a global class sheet is another dialect.
- **Reversibility:** medium (shared parts; one commit per primitive). **Gate:** no. **Unblocks:** P3, overlays (D-20), Settings controls.

### D-10 · Motion durations, easings, rises (T10)
- **Options:** (a) Halo values globally · (b) Rift-tuned values under Halo names (base 240) · (c) two scales, one for chat and one for pages.
- **Decision:** (a), one scale.
  - **Core tokens:**
    - `--dur-fast` 150 (hover, press, pop-in) · `--dur-base` 300 (state, accordion, dialog, drawer, toast) · `--dur-slow` 600.
    - `--stagger` 60, capped at 6 items for page sections. Menu rows use 18ms, capped at 12.
    - `--ease-out` = `cubic-bezier(.2,.8,.2,1)`, with `--ease-page` and `--ease-soft` aliased to it.
    - `--ease-spring`, which is identical to Rift's today, is only for things you flip, grab or add: toggle knob, check/radio pop 1.18, slider thumb, detent snap (K14), tag chip, count pop, send/stop glyph swap.
  - **Named exceptions (only these):**
    - Page enter is `--dur-page` 520 + 60ms delay. Page leave is opacity 140ms.
    - Popup leave 140. Modal, drawer and banner leave 200. Toast leave 240.
    - FLIP 460 · roll 900 · count-up 700 · ping 1.8s · spin 900 · indeterminate 1.3s.
  - **Rises (three only):**
    - `--rise-page` 10px for pages, cards, dialog content and empty states.
    - `--rise-reveal` 6px for the skeleton hand-off.
    - `--rise-stream` 3px for stream blocks, paired with `--dur-stream` 240ms. That keeps today's stream value, and it is gated with D-11.
  - **Page change (one orchestrated entrance, M24):** the first open per session rises its sections 10px over 520ms, staggered 60ms and capped at 6. Later activations glide the content area only. Remove `wsIn`, `.rising` and `SETTLE_MS` (three stacked layers, map 01 §8.1).
  - **Splash curve:** the doors keep `(.65,0,.35,1)`, the only named non-token curve (D-26).
  - **P2 sweep:** 36 raw `(.22,1,.36,1)`, ~104 bare `ease` and ~150 duration literals become tokens. A `cubic-bezier(` ratchet then holds the line.
- **Why:**
  - The owner asked for "proper motion and animations" and a "happy medium".
  - There is no owner rejection of Halo's durations; "laggy" (M5) was streaming pacing, not menus. So Halo fidelity (priority 4) beats Rift's tuned 240 (priority 5).
  - Halo's own page value is 10px. Its 14px card rise was never owner-tested, while an 8px stream rise was rejected as "lurching" (M6).
- **Dissent:**
  - The engineering lens: 240, because 300 slows 111 sites by 25% in an app the owner called laggy. Kept as a single toggle in the motion lab (shortlist #3).
  - The fidelity lens: a 14px card rise and no page exit.
  - The taste lens: a 200ms stream rise.
- **Reversibility:** cheap once the literal sweep lands. **Gate:** no (240/300 rides on #3). **Unblocks:** P2, D-11 to D-14.

### D-11 · Streaming reveal + scroll follow (T11)
- **Options:** keep 22727d24 as-is · mechanism fixes + motion-lab tuning · instant text with a trailing fade · line/sentence chunks.
- **Decision (structure — ships in P4b without the owner):**
  1. Extract the reveal cursor and follow maths (`Markdown.svelte:216-252`, `AssistantPane.svelte:169-191`) into `src/lib/motion/reveal.ts` and `follow.ts`, with vitest asserting identical cadence at 60/120/144/180Hz. This is the first automated net for motion.
  2. Carry the remainder: `last += due*WORD_MS`, not `last = t`. Today's cadence is frame-quantised (map 03 §8.2).
  3. Normalise the follow to frame time: `k = 1 − (1 − 0.35)^(dt/16.67)`. That is τ ≈ 39ms and feels the same as today at 60Hz.
  4. Fix the follow quirks:
     - Any user-initiated upward wheel or key un-latches at once (no 80px magnet).
     - The follow stops when |Δ| < 1px.
     - Jump-to-latest uses the follow engine, not native smooth scroll.
     - The turn rail is rAF-throttled or uses IntersectionObserver.
  5. Kill the `code-rise` replay on every delta.
  6. Don't Shiki-highlight an open, still-growing fence; it stays plain mono until it closes.
  7. The end-of-turn swap preserves the live `Markdown` (D-12).
  8. Defer incremental parsing until a long real turn is measured (dev p99 5.7ms, map 09 §5.1).
- **Decision (pacing — the owner sets these in the motion lab):**
  - Keep `WORD_MS` 30, `CATCHUP_LAG` = `ceil(320/30)` = 11 words, and follow 0.35 @ 60Hz.
  - The per-word `md-word` goes from 500ms with `blur(3px)` and a raw curve (`Markdown.svelte:170,684`) to opacity-only 300ms `ease-out`. `REVEAL_WINDOW` becomes `ceil(300/30)+1` = 11.
  - Each change is its own revertable commit.
- **Motion lab:**
  - Canvas board 04 runs `reveal.ts` on canned turns (prose, code-heavy, tool-heavy).
  - Sliders: word ms 17-50 · catch-up 4-16 words · fade 0-500ms · blur on/off · follow τ 0-150ms vs instant pin · `--dur-base` 240/300 · stream rise duration.
  - Claude then confirms the picked values in WebView2 through `/dev/gallery` (with a parameter panel) and a DEV-only fake-stream harness in `AssistantPane`. The owner never has to run a dev server.
  - `pinToBottom` is only proven on a real turn (shortlist #7).
  - Reduced motion: instant text and instant pin.
- **Why:**
  - The owner's "a little laggy, could be better" was about pacing, not frames (M5, p95 5.6ms), and his verdict on 22727d24 is still pending (map 12 §0.3).
  - Halo says blur small areas, once. About 18 concurrent blurred spans break that and cost compositing (map 09 §6.2).
  - The owner's rule: no speculative pacing ships; one toggle at a time, felt live.
- **Dissent:** the taste lens wanted different starting points (catch-up 8, fade 180, τ 80). They become lab presets; today's feel stays the default because nothing has been judged yet.
- **Reversibility:** cheap per constant. **Gate:** yes for the numbers (#3) and the paid turn (#7). The structure ships regardless. **Unblocks:** P4b, the owner's #1 complaint.

### D-12 · Turn lifecycle motion (T12)
- **Options:** hard swap (today) · crossfade · same-node fold.
- **Decision:** same-node fold.
  - **Template:** one template for both states, with no `{#if streaming}/{:else}` swap. The answer say-groups live in one keyed region present in both states, so `Markdown` instances never remount, `blockIn` doesn't replay and held words don't snap. Settling only flips a class.
  - **Collapse:** the activity body collapses `grid-template-rows 1fr→0fr` over 300ms `ease-out` with an opacity fade. The receipt line fades in over 150ms after a 60ms delay.
  - **Keys:** group keys are the first block id, not the array index (`StreamTurn.svelte:418,426,487`). `streamModel.ts` is not touched (its test file is 1,011 lines).
  - **Entrances:** only blocks born live get an entrance (3px, `--dur-stream`). History hydration is static (D-29).
  - **Disclosures:** every disclosure (10 `transition:slide` sites + `workOpen`/`gcReveal`/`bodyFadeIn`) uses one accordion: grid-rows 300ms, content fade +60ms, closed panel `inert`. One glyph app-wide: chevron-down rotating 180°.
  - **WorkLine:** the collapsed chip stays fit-content (D8: full-width bars are "tacky") and snaps to full width when opened.
  - Diff rows don't stagger.
  - P0 first records the suspected end-of-turn blink through CDP (map 03 §4.1).
- **Why:** the hard swap is the most visible glitch candidate (map 03 §4.1, map 09 §5.4), right at the owner's Ledger moment. Halo: "content arrives, never pops".
- **Dissent:** the fidelity lens wanted the receipt to rise 3px rather than fade. A fade was chosen because the receipt is chrome, not content.
- **Reversibility:** medium (StreamTurn template). **Gate:** no; board 02 shows it. **Unblocks:** P4b.

### D-13 · Loops + reduced motion (T13)
- **Options:**
  - Halo strict: LiveDot, Spinner and indeterminate Progress only.
  - Halo strict plus the owner-approved verb shim.
  - Keep all 115 loops.
- **Decision:** Halo strict plus the shim.
  - **Survivors:**
    - LiveDot: ping 1.8s, `::after` scale 1→3 and opacity .7→0. It replaces the box-shadow pulses and the 9-dot working grid, which also gets an `m:ss` elapsed timer.
    - One Spinner at 900ms, replacing 25 keyframes.
    - Indeterminate Progress at 1.3s for browser loads and long jobs. It replaces the comet at `WebBrowserPage:735`.
    - `.shim` on the ONE live footer verb only, at most one per view. It is killed on `.wb-label`, `.splan-text`, `.wa-text` and `.sh-label`.
    - Terminal cursor, exit-plan caret and dictation caret.
    - The mic waveform, moving from `height` to `scaleY`.
  - **Kill list:**
    - Breathing loops: welcome mark/greeting breathe, `pill-ultra`, `.pill.ok` breathe, `up-pulse`, `orb-breathe`, and the drop-dot pulse.
    - LimitBar pulse and sheen.
    - The enhance "magic" (shimmer + aura + 4 stars + wand pulse) becomes a Spinner in the wand plus static dim text.
    - The splash caret blink.
    - The legacy rail and avatar glows.
    - The AI Health hot-verdict pulse becomes a static danger Badge.
    - The active-project pulse becomes a static accent dot.
    - The N breathing vitals become one LiveDot.
    - The "Page read" shimmer becomes a one-shot Badge.
  - **Onboarding demos:** play once when the step opens, hold the final frame, and offer a small "Replay" text button.
  - **Send ripple:** one ring, once, 600ms. Board 03 shows it against no ripple.
  - **Reduced motion:**
    - Use Halo's global net in `app.css` (`*,*::before,*::after{animation-duration:.01ms!important;animation-iteration-count:1!important;transition-duration:.01ms!important;scroll-behavior:auto!important}`).
    - Spinner alone is exempt via `[data-essential-motion]`. The LiveDot ping stops and the dot stays solid; the shim becomes plain `ink-muted`; cursors go solid.
    - `prefersReducedMotion()` becomes reactive (`$state` fed by a `matchMedia` change listener).
    - An `rm()` helper wraps the 10 unguarded Svelte transitions.
    - rAF engines jump to their end state.
    - Per-selector blocks are deleted as the net lands.
- **Why:**
  - Halo: "never loop for attention".
  - M17 asks for honest signals, no light show; M3 found motion "disturbing".
  - 115 infinite declarations exist, and 13 motion files have no reduced-motion handling (map 09 §7.2).
  - The owner explicitly approved the shim (M16), which outranks Halo's loop list.
- **Dissent:**
  - The fidelity lens kills the shim.
  - The taste lens freezes the Spinner to a static arc under reduced motion. Rejected: a frozen spinner reads as hung.
  - The fidelity lens drops the send ripple.
- **Reversibility:** cheap (one commit per kill group). **Gate:** no. **Unblocks:** P2.

### D-14 · Skeleton (T14)
- **Options:** Halo shimmer + hand-off · static bones + Halo hand-off · Rift idiom only (idle cursor + timer, no bones).
- **Decision:** one `ui/Skeleton`.
  - **Bones:** `bg-200`, 12px tall, `--radius-xs` 6. Text variants at 100/82/64/90/72% width. They show only after 300ms, so fast loads don't flash.
  - **Motion:** no travelling sweep and no breathing.
  - **Hand-off (Halo, exact):** the bones fade out over 360ms (opacity 0, blur 4px). The content then reveals over 600ms, from opacity 0, blur 6px and translateY 6px.
  - `aria-busy` is set while loading. Reduced motion swaps instantly.
  - It replaces `Skeleton.svelte`'s sweep, `ws-load-shimmer`, `skel-shimmer`, `mcp-shimmer` and the AI Health pulse.
- **Why:** M2 "that little loading blurry block… looks old school" bans banded sweeps. Halo's hand-off still gives "skeletons fade, never pop".
- **Dissent:** none; Halo's shimmer stays available if the owner ever asks.
- **Reversibility:** cheap. **Gate:** no. **Unblocks:** Home, Health, MCP and boot loading states.

### D-15 · Blur / backdrop-filter / glass (T15)
- **Options:** keep the current blurs · blur only on small non-fixed layers · zero `backdrop-filter` with opaque floats.
- **Decision:** zero `backdrop-filter` anywhere.
  - Remove all 16 declarations, including the three on fixed layers WebView2 mis-composites: `SettingsMenu:519`, `ConversationList:512`, `ProjectSwitcher:266`.
  - Floating surfaces become opaque (recipes in D-20).
  - `filter: blur` is allowed only for the skeleton hand-off and the splash mark settle. No per-word blur, no exit blur, no page blur.
  - Delete the `AssistantWelcome` blurred aura. `.cm-fade` becomes gradient-only.
  - Add a `check:ui` metric for `backdrop-filter`, ratcheted to 0.
- **Why:** WebView2 mis-compositing is a hard constraint and the project's own §10 ban. Halo's product category has no glass (map 10 §5.2 #5). The composer well's blur re-samples the streaming transcript on every frame.
- **Reversibility:** n/a (this removes a defect). **Gate:** no. **Unblocks:** P2, composer, menus, HUDs.

### D-16 · Halo extras (T16)
- **Options:** the whole Halo delight layer on · all off · signal-bearing extras only.
- **Decision:**
  - Card spotlight, Button magnetic and FocusGroup dim are all OFF. If ever wanted, they would go on one hero card only, after the owner feels them on a board.
  - `glow-strong` (rgba(106,168,255,.28)) only under a hovered solid primary: Send, plus at most one CTA per view.
  - Press uses `scale: .97` (the individual property) on buttons only, not rows, tiles or cards, at `dur-fast`.
  - Hover lift of 3px only on whole-clickable hub cards (project, resume, starter). Never on stream tiles or rows.
  - Count-up: 700ms easeOutCubic on the first data arrival and on range change. Never on the 60s/30s ticks.
  - RollingNumber: 900ms, right-most digit first, delay `min(pos,8)*45ms`. Only for the live token counter and diff `+N/−M`. It keeps the monotonic guard and the .6 opacity floor (X42).
  - Elapsed seconds stay plain tabular text.
  - NotificationBell (Halo): the count pops with a spring, and the bell swings for 900ms once per count increase. Both are off under reduced motion.
- **Why:**
  - The owner banned travelling light (M3, C8).
  - Built-then-rejected "surprise me" extras are a known risk class (map 12 §1.8 #2).
  - Digits rolling every second would put constant motion in the eye line.
- **Dissent:**
  - The fidelity lens wanted the Halo count-up at 1100ms and rolling numbers for elapsed time, context % and usage %.
  - The taste lens wanted no bell swing. It is kept because it fires once, on a real event.
- **Reversibility:** cheap (all opt-in). **Gate:** no. **Unblocks:** P3, hubs, composer Send.

### D-17 · Tooltips (T17)
- **Options:** keep aria-only · revive Halo's Tooltip as-is (120ms, inverted bubble) · revive with a calmer timing.
- **Decision:** revive behind the existing `use:tooltip` action. All 174 call sites across 37 files light up with no call-site edits.
  - **Mechanism:** one singleton tooltip layer, a single DOM node positioned with floating-ui (bits-ui's dependency; import path checked in D-28).
  - **Look:** Halo inverted bubble: background `ink` #eceff3, text `bg-000`, 500 12/16, padding 6×10, radius 8, max-width 240, `z-tooltip` 70.
  - **Motion:** a 4px rise at `dur-fast`.
  - **Timing:**
    - The first show waits 400ms.
    - It is "warm" at 120ms when the pointer reaches another tooltipped trigger within 500ms of the last hide.
    - It hides immediately.
  - **A11y:** keep the aria-label promotion for icon-only buttons, and add `aria-describedby`.
  - **Scope:**
    - Show on: icon-only buttons, abbreviated numbers (exact value), chart marks, truncated paths, shortcut hints.
    - Skip when the tooltip would repeat visible text.
    - Suppressed over `.wb-stage`.
  - **Native titles:** `title=` in WebBrowserPage (16) and McpServersDialog (5) move onto the action.
- **Why:**
  - The owner's own fix D6 ("abbreviate, exact on tooltip") assumes tooltips paint. AI Health promises exact values on hover, and Settings copy says "hover to see"; none of it works today (map 07 §8.1, map 06 §8.4).
  - Halo pairs icon-only buttons with tooltips.
  - The 2026-06-15 pull (`tooltip.ts:1-8`) called the themed glass popover "visual noise" (author unknown). The 400ms delay and the singleton are the calm answer.
- **Dissent:** the taste lens wanted a quiet `bg-200` bubble at 500ms; the fidelity lens wanted 120ms. Board 08 shows inverted vs quiet.
- **Reversibility:** cheap (one action file; flip back to a no-op). **Gate:** no. This is an FYI: it reverses a prior pull. **Unblocks:** icon-only toolbars, exact numbers on Home and Health.

### D-18 · Syntax colour (T18)
- **Options:** (a) Halo monochrome everywhere · (b) mono in the stream, Shiki in fences · (c) a muted Shiki theme from Halo's palette · (d) keep `github-dark-dimmed`.
- **Decision:** (c). A custom Shiki theme `rift-halo` replaces `github-dark-dimmed` (`highlighter.svelte.ts:8`) in Markdown fences, Read results and diff bodies.
  - **Palette:**
    - Base and identifiers `ink`; keywords #9cc6ff; strings, numbers and constants #b79cff.
    - Comments `ink-faint` (no italic). Punctuation and operators `ink-muted`. Invalid `danger`.
    - No accent and no green.
  - **Monochrome `ink`:** shell command heads and prompts (this kills kind-coloured `$`/`PS>`, map 02 §8.5), plus config, console and JSON detail blocks.
  - **CodeBlock chrome:**
    - Surface: `bg-100`, `line-100`, radius 12.
    - Head: density-tied (32/36/40), with a language label.
    - Copy: shows "Copied" in accent or "Copy failed" in danger, for 1.8s. Copy failures are swallowed today (map 03 §8.13).
  - **Terminal surface:** `bg-000`.
  - The DOMPurify allow-list is unchanged.
  - **Growing fences:** deferred (D-11).
- **Why:**
  - D3 "show as much detail as possible" is a recorded owner HOLD, and syntax colour is detail.
  - "Happy medium" sits between full Shiki colour and monochrome.
  - Keeping accent out of syntax protects accent scarcity (X11).
- **Dissent:** the fidelity lens wanted monochrome `rift-halo-ink` as the default (Halo CodeBlock rule; ConsoleX monochrome "cleaner"), soft-gated. The taste lens put accent on keywords. Board 02 shows muted vs monochrome.
- **Reversibility:** cheap (one theme constant). **Gate:** no. **Unblocks:** P4b code surfaces, diffs, Read.

### D-19 · Navigation (T19)
- **Options:** an open-chat tab strip (from the mock) · the sidebar list as tabs (today) · Halo's 72px icon rail vs Rift's collapse-to-nothing + peek.
- **Decision:** no tab strip. The sidebar ConversationList stays the tab list.
  - **Behaviour kept:** resize, collapse-to-nothing, mini peek and pin all stay.
  - **Peek:** animates `transform`, not `top`.
  - **Collapse:** width, margin and `--rail-w` move together over 300ms `ease-out` (one move). Reduced motion makes it instant.
  - **Rows and dock tiles use Halo NavItem anatomy:**
    - icon 18;
    - active = `bg-200` + accent icon + `aria-current`;
    - focus ring and mono count.
  - **Active marker:** one sliding marker (the ConsoleX plate pattern, 300ms) replaces the per-row `barPop`. The `rowIn` cascade runs on first mount only.
  - **Naming:** "Home" everywhere (tile, aria-label, registry title). Ctrl+1 opens the Home page; Ctrl+N stays new chat. This is a small behaviour change, flagged.
  - **Dead field:** the registry `kbd` field has no consumer and wrong values, so it joins shortlist #6.
  - **Diagnostics:** reachable from the AI Health header, the palette and Settings → System, with no dock tile.
  - **Browser:** stays a chat dock.
  - **Status bar** (S7): kept, on Halo tokens.
  - **Bell:** stays in the sidebar footer.
  - **Top bar:** keeps Rift's anatomy and drag regions. No breadcrumbs or search bar this wave.
- **Why:**
  - Tab rejection L10: "saw it live, wanted it gone".
  - The peek was iterated over 3 sessions (L2, L3).
  - Tauri constraints: `position:fixed`, drag regions, the `--rail-w` publisher (X46).
  - A tab strip would be a new stateful surface with no tests.
- **Dissent:** the fidelity lens wanted:
  - Halo's 72px icon rail, with the peek removed;
  - a NavItem column that includes Diagnostics;
  - a new 52px top bar with breadcrumbs, a search trigger and the bell.

  The engineering lens gated the tab strip. It is not gated here, because the default honours the owner's own rejection and a strip can be added later.
- **Reversibility:** cheap. **Gate:** no; board 01 asks about the tab strip. **Unblocks:** P4a; the `c.sh nav` labels update in the same commit.

### D-20 · Overlays (T20)
- **Options:** the Halo overlay family exactly (bottom-right toasts, right drawer, Halo z order) · Halo look + motion on Rift placement and z order · bespoke overlays restyled.
- **Decision:** Halo look and motion, Rift placement and z order, all on bits-ui.
  - **Dialog (bits-ui Dialog / AlertDialog):**
    - Motion: enters with `halo-zoom` (scale .94 + translateY 8px → none) over 300ms `ease-out`. Leaves at scale .96 + opacity over 200ms via `[data-state=closed]`.
    - Surface: width `min(480px, 100% − 32px)`, `bg-100` + `line-200` + `shadow-pop`, radius 16.
    - Scrim: rgba(4,5,7,.72), fading in over 150ms.
    - Destructive dialogs default to Cancel (K12).
    - Moving onto it: UpdateDialog, McpServersDialog, CommandPalette and every native `confirm()`. MCP keeps its capture-phase Esc.
  - **Palette:** bits-ui Dialog with Rift's scorer. Top 12vh, width `min(560px, 100% − 32px)`, rows 32/36/40, group headings, sub-meta, footer legend.
  - **Menus, popovers, select, context:**
    - Surface: Halo `.halo-pop` = `bg-100` + `line-200` + radius 12 + `shadow-pop`.
    - Enter: opacity 0 / scale .96 / translateY −4px → none, 150ms, from `--bits-floating-transform-origin`.
    - Rows: rise 3px over 220ms, staggered `min(i,12)*18ms`. Slash and Mention appear all at once (map 09 D13).
    - Leave: opacity + scale .97 over 140ms.
    - Always portalled. One recipe replaces the three shell popovers, `.rift-menu` and the custom SettingsMenu panel.
  - **Drawer:** the Output inspector becomes a right drawer, 760px wide, sliding 40px over 300ms and leaving over 200ms.
  - **Notifications:** the bell keeps its upward popover from the sidebar footer, not Halo's right drawer, which would land on the native webview. It gets Halo rows and focus move, trap and return (an a11y bug today).
  - **Toasts:**
    - Position: top-centre of the main island, offset by `--rail-w`.
    - Halo stack: collapsed offset 10px, scale −.05 per step (min .86), fanning out on hover/focus with timers paused.
    - Motion: enter from −16px over 300ms; leave over 240ms.
    - Dismiss: drag right past 90px or faster than 0.6px/ms.
    - Lifetime: 4000ms by default; errors are sticky.
    - Tones: accent/info/warn/danger plus neutral.
  - **Banner:** UpdateBanner moves inside the content area. It rises over 300ms and leaves over 200ms, with no full-window reflow.
  - **HUDs:** `bg-200` + `line-200` + `shadow-pop`, entering with `halo-pop-in`.
  - **Native browser contract** (needed before any overlay near the dock): an overlay registry store, frontend only, using the existing `browser_hide`/`browser_show` IPC (`WebBrowserPage.svelte:375-377`) and no new Rust command.
    - While the dock shows a page, modal layers (dialog, palette, drawer) hide the native webview and restore it when none remain.
    - Menus and popovers hide it only when their rect intersects `.wb-stage`.
    - Tooltips and toasts never trigger it; the toast stack stays in the main column left of the dock.
    - While hidden, the stage paints `bg-000` with the page title, so there is no blank hole.
    - The dock width slide keeps tweening real width (bounds sync), 300ms; nothing is transformed on an ancestor of `.wb-stage`.
- **Why:**
  - The native child webview paints over HTML (`browser/mod.rs:8-11`).
  - Menus inside dialogs must work (DESIGN §10).
  - The owner chose top-centre toasts on 2026-07-16 (X43, priority 3 over Halo). Bottom-right also collides with the composer.
  - Today there are three scrim recipes and three panel recipes (map 07:185).
- **Dissent:** the fidelity lens wanted bottom-right Halo toasts and a new Rust `browser_overlay_lock`. The taste lens gated the Rust IPC; that is moot, because the existing IPC suffices.
- **Reversibility:** medium (shared primitives; one commit per dialog). **Gate:** no. **Unblocks:** P3, P4f, the palette, every popover.

### D-21 · Chat anatomy (T21)
- **Options:**
  - Ledger + Rift dense tiles on Halo tokens.
  - Halo Card/CodeBlock for tool blocks.
  - The ConsoleX steps-rail.
  - The document band (option C, rejected 2026-10-19).
- **Decision:** the Ledger's information architecture is frozen (owner pick 2026-10-19, L16). Restyle only.
  - **Receipt:** answer first, then one hairline Divider, then the receipt row: outcome icon + title `small`, facts in sans `caption` `tabular-nums` `ink-faint`. Copy and Retry are ghost buttons, shown on hover and `:focus-within`.
  - **User message:**
    - Right-aligned bubble, `bg-200` + `line-100`, radius `16 16 4 16`, max `min(560px, 80%)`, padding 10×14, chat text size.
    - Timestamp in `caption` `ink-faint` below, with Copy on hover.
    - No gradient, inset or shadow.
  - **Tool tiles:** the Rift dense tile on Halo tokens: `bg-100`, `line-100` → `line-200` on hover (300ms), radius 12, density-tied head 32/36/40, padding 8×12, `small` text, `caption` meta. No lift.
  - **State, one signal each:**
    - live = LiveDot + label;
    - failed = danger icon + "Failed" Badge (no red border, no 2px edge);
    - needs-you = 1px accent border at 32% + a "Needs you" accent Badge.
  - **Pills → Badge:** new = accent; edit, agent and poll = neutral; deleted = danger.
  - **Inside the expanded receipt:**
    - Quiet WorkLine groups use a ConsoleX hairline rail: a 1px `line-100` left rule, 32px rows, a 16px `ink-muted` icon, the verb in sans, the target in mono, and a time only on the running row.
    - Heavy blocks keep tiles: shell, diff, ask, plan, agent.
  - **Terminal:** CodeBlock terminal surface (`bg-000`, `line-100`, radius 12) around the OutputBlock body.
  - **HUDs:** both kept (S6). They are opaque; the IntersectionObserver logic is untouched.
  - **New banner:** the `max_tokens` "Continue" and refusal notices become an inline Banner. Today they exist only in the legacy tree (map 02 §2). Separate commit.
  - **Legacy tree:** the MessageBubble assistant branch and the Stream-view toggle stay frozen and are not restyled. Deleting them is shortlist #6.
  - Shared parts (BlockHeader, OutputBlock, ReadResult, GrepResult, EditDiff) restyle both trees.
- **Why:**
  - The owner picked the Ledger today.
  - D3: detail by hierarchy, not by hiding.
  - K4 one tile family; K5 one fact, one home.
  - Today one failure carries three signals (map 02 §3).
- **Dissent:** the fidelity lens wanted mono receipt numbers and a 36px head. The engineering lens skipped the hairline rail for now and used a 30px head.
- **Reversibility:** cheap (CSS), except the IA, which is not changing. **Gate:** only the legacy deletion (#6). **Unblocks:** P4b, board 02, gallery baselines.

### D-22 · Composer anatomy (T22)
- **Options:** (a) bar below the well (Rift v4, `Composer.svelte:1906-1909`) · (b) controls inside one card (Halo mock `Main.dc.html:126-138`, ConsoleX one-row card).
- **Decision:** (a) on Halo tokens until the owner answers shortlist #4.
  - **Well:**
    - Halo textarea recipe: `bg-100`, `line-300`, radius 12 (16 in hero).
    - Focus: `:focus-within` gives an accent border plus `shadow-focus` around the well.
    - Not `--bg-inset` and no blur.
  - **Controls:** 32px ghost buttons.
  - **Send:**
    - Shape: 32×32, radius 8, accent fill when ready, `glow-strong` on hover, press .97.
    - Glyph: swaps send/stop/queue with a spring.
    - Stop state: `danger-soft` fill + square glyph.
    - Ripple: one ring, once (D-13).
  - **Esc-to-stop:** it becomes the LAST Esc layer, active only while streaming and no slash/mention/settings/enhance/undo layer is open. That makes the shipped hint true (map 04 §7.20).
  - **Popovers:** one `.halo-pop` recipe in its upward variant, portalled and opaque.
  - **SettingsMenu:**
    - The virtual listbox driven from the textarea stays.
    - Its rAF follow becomes a ResizeObserver plus the FLIP end event.
    - The detent slider spring stays (K14).
  - **Drag and chips:**
    - Drag-over: solid accent border, 7% accent tint, copy "Drop files to attach".
    - Chips: tag-chip spring.
    - QueueRail: FlipList, 460ms.
  - **Hero ↔ docked:** FLIP over 460ms `ease-out`, with `will-change` only during the FLIP.
  - **After Geist:** re-check the `dict-ghost` / `magic-text` padding and the mic-ring `rx` (D-33).
  - **Placeholder:** one quiet phrase.
- **Why:**
  - The evidence ties:
    - For (b): L15's north star is Claude Desktop (controls inside), ConsoleX's card was "cleaner", and the mock had it.
    - For (a): S5 iterated live four times and landed on v4.
    - The owner called the mock "rough" without saying which part (Q21).
  - The tie breaks on reversibility. (a) keeps the hero/docked FLIP, the `composerH` ResizeObserver and the focus-ring extent intact (map 04 §9), and it is the cheapest base to flip later.
- **Dissent:** the fidelity and taste lenses both default to (b).
- **Reversibility:** a flip to (b) costs about 1-2 days (FLIP heights, `composerH`, focus ring, hero bar logic). **Gate:** yes, shortlist #4. Board 03 should come before P4c starts. **Unblocks:** P4c, the `composerH`-based jump/toast offsets.

### D-23 · Scrollbars (T23)
- **Options:** hidden everywhere (ConsoleX, Oct 10) · Halo thin everywhere · Halo thin, keeping Rift's existing hidden sites.
- **Decision:** Halo thin scrollbars, applied by swapping tokens in `app.css:320-337`.
  - 10px, no arrow buttons, transparent track.
  - Thumb `line-200` with a 3px transparent border (`background-clip: padding-box`); hover `ink-faint`.
  - Keep `scrollbar-width:none` where Rift hides them today: the chat `.stream`, the home/welcome column and sideways strips.
- **Why:**
  - The task premise "scrollbars hidden" is false (X29): Rift already has thin bars and hides only 5 sites.
  - The owner "hates visible/white scrollbars" (ConsoleX, a recorded rejection). That outranks Halo fidelity on the reading surface.
  - The owner's reaction to ConsoleX's later thin bars is unrecorded.
- **Dissent:** the fidelity and taste lenses wanted thin bars in the chat stream too, for orientation (ADHD).
- **Reversibility:** cheap (one selector). **Gate:** no. **Unblocks:** P1.

### D-24 · Data viz (T24)
- **Options:** Halo Stat/BarChart/LineChart as-is · Halo anatomy with a Rift formatter and dense bars · keep today's hub viz, recoloured.
- **Decision:**
  - **StatTile:**
    - Anatomy: label `small` `ink-muted` → value `metric` 28/32 mono → detail `caption`.
    - Delta chip: arrow + sign. Up = `accent-soft`/accent, down = `warn-soft`/warn, and `goodWhen` inverts it for cost.
    - Values: a Rift `format(fn)` (`32k`, `$8.2k`, `7.2s`) replaces Halo's en-US `$`. Exact values on hover (D-17). Count-up per D-16.
    - The 38px gradient hero numeral is removed (D7).
  - **Daily chart:**
    - Rift dense bars (14/30/60): bars `bg-300`, the peak bar accent.
    - On hover: show the value and dim siblings to .45.
    - Bars grow (`scaleY`, 600ms) on mount and on range change, with an accessible value list.
  - **Model mix:** ShareBar, with the top 3 in chart-1 accent #6aa8ff, chart-2 #b79cff and chart-3 #9cc6ff. The rest is "Other" in `bg-300`, with a legend showing values.
  - **Projects:** neutral initials on `bg-300`, with an accent ring on the active project only. The `projectHue` util stays (it is tested) but unused.
  - **Limits:** the three LimitBar variants become one Halo Progress: `scaleX` over 700ms, tones accent/warn/danger, no pulse or sheen.
  - **Spend:** a Halo Sparkline drawn in.
- **Why:**
  - C2: a second decorative hue is a bug.
  - Per-project hues (C15) were built under delegation and are still pending.
  - D7: oversized numerals are an AI-slop tell.
  - The hubs must expose values without relying on tooltips.
- **Dissent:**
  - The fidelity lens wanted Halo BarChart (7d), LineChart (30d) and hash-tinted Avatars.
  - The engineering lens wanted `ink-muted` as the third series.
  - Tension: violet #b79cff brushes against §8's "DON'T use purple/lavender-on-dark defaults". It is a series colour, not a surface default, and it is flagged for board 05.
- **Reversibility:** cheap. **Gate:** no. **Unblocks:** P4d, P4f.

### D-25 · Process (T25)
- **Options:** canvas boards, then port · port first, then show in the app · a long single `design/halo` branch vs phase branches · code-first with no canvas.
- **Decision:**
  - **Canvas first.** A Claude Design project "Rift · Halo" with Halo installed. Boards:
    - 00 Foundations: type, density, colour and radius specimen. Opens with Q21.
    - 01 Shell frame, A beside B, plus the tab-strip question.
    - 02 Chat: live → settled, receipt, tiles, user bubble, code colour A/B, diff/ANSI.
    - 03 Composer: below vs inside; ripple vs none.
    - 04 Motion lab.
    - 05 Home + onboarding.
    - 06 Settings.
    - 07 Health + Diagnostics + Browser.
    - 08 Overlays: dialog, palette, menu, toast, tooltip.
  - **How boards work:**
    - Boards are plain markup on Halo tokens that the owner plays in Play mode. He is never asked to run a dev server.
    - Each board is a numbers contract: it exports px/ms/hex into `docs/design/halo/spec/<surface>.md`, and the port reads that file.
    - P0-P3 need no board.
  - **Branch:** `design/halo`, cut from `design/chat-ledger` @ 22727d24. Phase work merges into `design/halo` locally. Merging to `main` and any push wait for the owner.
  - **Commits:** one concern each. Token value changes are separate from token plumbing. Each pacing constant and each kill group gets its own commit. A rename lands in the same commit as all its consumers.
  - **Verify per phase (there are no DOM tests):**
    - `npm run check`: svelte-check, `check:tokens`, `check:ui`.
    - `npm test`.
    - CDP baseline and comparison:
      - `c.sh baseline` (in P0) on `tour home chat settings ai-health`;
      - `c.sh diff` + `c.sh measure` at the fixed audit viewport (2048×1104 @ DPR 1.25);
      - `c.sh inspect` with 0 console errors.
    - `/dev/ui` + `/dev/gallery` shots.
    - Reduced motion via CDP emulation.
    - Phases that touch Rust (P1 browser background) run the full `npm run verify` on Windows.
  - **Stale docs:** P0 adds a one-line "superseded by 22727d24 / this spec" note at `docs/HANDOFF.md:26-36` and `docs/design/redesign-plan.md:178-216`. It does not rewrite the audit.
- **Why:** the owner designs in the canvas and finds a dev server "irritating". He wants to "really see the pages" before porting ("or it gets done twice"), as in ConsoleX.
- **Dissent:** the engineering lens wanted `design/chat-ledger` merged into `main` locally first. Rejected: 22727d24's pacing verdict is still pending.
- **Reversibility:** n/a. **Gate:** no (push and merge to `main` always ask). **Unblocks:** sequencing.

### D-26 · Logo / splash / brand (T26)
- **Options:** keep the mark + seam splash, retuned · a Halo-style fade/rise splash · Halo Logo with no splash.
- **Decision:** keep the Rift mark PNG everywhere; Halo's `Logo` is Halo's own mark (X47). Keep the seam-parting splash with its real boot stages, and the Lexend RIFT letters.
  - **Retune:**
    - Panels `bg-000`.
    - The doors keep `(.65,0,.35,1)` as the one named non-token curve.
    - Letter stagger goes from 55 to 60ms.
    - The mark settle (blur 8px → 0) stays, since it happens once, at boot.
    - The caret blink and the 3px exit blur go.
  - **Brand head:** the sidebar brand head is Geist 600 −0.03em.
  - **First paint:** colours per D-34. Check the squircle PNG edge on `bg-000` in the P1 shots.
- **Why:**
  - The owner asked for "a whole rework" (M22) and got the seam splash. Its verdict is pending, not a rejection.
  - Halo has no splash.
- **Dissent:** the fidelity lens moved the wordmark to Geist and the doors to `ease-out`.
- **Reversibility:** cheap. **Gate:** no. **Unblocks:** P1, boot choreography.

### D-27 · Sequencing spine (added)

| Phase | Scope | Visible change | Merge gate (local) |
|---|---|---|---|
| P0 | CDP baselines, D-28 spikes, stale-doc notes | none | n/a |
| P1 | vendored tokens, alias layer, `data-theme="blue"`, Geist, z ladder, native colours (D-34), undefined vars (D-33), `check:tokens` rewrite, token tests, DESIGN.md rewrite | whole app recolours and refonts; no layout change | check + test green · `c.sh diff` per surface reviewed · 0 console errors · `npm run verify` (Rust) |
| P2 | `styles/motion.css` (one home for keyframes), reduced-motion net, one Spinner / LiveDot / three rises, kill list, zero `backdrop-filter`, layout → transform sweep, D-29 guard | calmer, fewer loops | check + test · gallery replay · reduced-motion pass |
| P3 | `ui/` primitives + atoms, `recipes.ts` fix, `/dev/ui` extended | only in `/dev/ui` | check (incl. class generation) · `/dev/ui` vs board shots |
| P4a-f | 4a shell · 4b chat stream + pane · 4c composer · 4d home + onboarding · 4e settings · 4f health, diagnostics, dialogs, browser chrome | per surface | check + test + `c.sh tour/diff/measure` + 0 errors + reduced motion |
| P5 | drop Inter/JetBrains, `check:ui --update` lock-ins, DESIGN.md final, approved deletions (#6) | none | full `npm run verify` on Windows |

- **Order:**
  - P1 comes first because 17 colliding names otherwise retune the app silently (map 08 §1.3).
  - P2 and P3 come before the surfaces so no surface re-invents spinners, rises or overlays (25 spinner keyframes and 9 rise keyframes today; hand-rolled overlays raise `check:ui`).
  - Surface order: shell (the frame colours everything) → chat (the owner's main complaint) → composer (FLIP contract with the pane) → dashboards (lowest risk).
- **While the owner is away:**
  - P0-P3 and 4d/4e/4f run fully on defaults.
  - 4a, 4b and 4c build on defaults; their merges wait on #1, #2, #3 and #4.
  - Each flip is isolated to 1-3 files or one token block.

### D-28 · Phase-0 verification spikes (added)
Nothing is built on these until each one is checked.
1. Is `floatingMotion` dead? Open a dropdown under CDP and read the computed `animation-name` (`recipes.ts:20,58,60` vs bits-ui `data-state`). This decides the D-09 fix.
2. Install `@fontsource-variable/geist` and `geist-mono`. Does an italic CSS file exist? Does Geist Mono have ligatures (`--code-liga`)?
3. Does the installed WebView2 support `interpolate-size` (the `.wline` morph)? If not, use the accordion grid-rows anyway.
4. Drag regions: `data-tauri-drag-region` vs `-webkit-app-region`; double-click to maximise; snap layouts.
5. Is Vite's `optimizeDeps` loading lucide twice (two runtimes) here, as it did in ConsoleX (map 11 §4.13)?
6. The floating-ui import path for the tooltip singleton (D-17).
7. Does settled-history hydration replay `blockIn`? This sizes D-29.
8. Record the suspected end-of-turn blink (map 03 §4.1) before D-12 work starts.
9. Read Halo blue's resolved `--chart-2`. If it is lime, the D-03 override stands.

### D-29 · Entrance replay + keep-alive pages (added)
- **Decision:**
  - Entrance animations run only on elements that mount while their container is `data-live`.
  - Some events re-mount content instead of adding it: history hydration, tab switch, split swap, re-parent, `{#key}` redraws and 60s data ticks. On these the container gets `data-settled`, which sets `animation-duration: 0s` (not `none`) for about 450ms. This is the ConsoleX `.panes.swap` fix, `51f4d02`.
  - Hidden keep-alive pages pause their loops (`animation-play-state: paused`).
  - It covers stream `blockIn` (~18 classes), the `.bubble` entrance, `.crow` `rowIn`, settings cards and diff rows.
- **Why:** this is a recurring bug class in ConsoleX, and Rift renders whole conversations without virtualization (map 03 §9). Replaying motion on every switch is motion for attention (M24).
- **Reversibility:** cheap. **Gate:** no. **Unblocks:** P2; it must land before P4b.

### D-30 · Text ladder, contrast, focus, a11y floor (added)
- **Decision:**
  - **Text ladder:** five rungs collapse to three.
    - `--fg` and `--fg-2` (#C3C8CF today) → `ink` #eceff3.
    - `--fg-muted` (#9CA3AF) → `ink-muted` #9aa2af, effectively the same value.
    - `--fg-subtle` (#7B808A) → `ink-faint` #7d8592, effectively the same value. The receipt keeps its look.
    - `--fg-faint` (#4B5056, about 1.6:1) → `ink-faint`, a legibility lift.
  - **`ink-faint` scope:** timestamps, labels, ticks and receipt meta only. Never body copy, never on `bg-300`. The 247 `fg-faint` sites are triaged surface by surface; descriptive copy goes to `ink-muted`.
  - **Borders:** control borders use `line-300` (3:1).
  - **Focus:** `--shadow-focus` (2px `bg-100` gap + 2px accent) on every control via `:focus-visible`. This fixes the missing or invisible rings on AI Health, Diagnostics, Settings `.seg`/`.sw` and the hue slider.
  - **Hover-only actions:** also shown on `:focus-within`.
  - **Status:** never colour alone.
  - **Disabled:** opacity .45 everywhere.
  - **Modals:** trap and return focus. The onboarding `aria-modal` gets a real trap. Esc on onboarding step 1 no longer dismisses setup.
  - **Buttons:** real `<button>`s only (fix `BlockHeader:74-84`).
  - **Reading comfort:** the `[data-a11y-*] .bubble/.markdown-body` selectors also match `.sturn .md`. Today dyslexia line-height and warm tint never reach stream prose (map 03 §3). The Lexend override stays last in the cascade.
- **Why:** C6 "quiet ≠ invisible" and C14 already lifted faint text. Halo's 4.5:1 floor fixes meta text sitting at 1.57:1 (X15). Dyslexia mode is an owner feature, so fixing its reach restores it.
- **Dissent:** the engineering lens mapped `--fg-2` → `ink-muted`. `ink` was chosen so stream narration stays the brightest thing on screen (§8, first rule).
- **Reversibility:** cheap. **Gate:** no. **Unblocks:** the acceptance criteria for every port.

### D-31 · Page shell + SettingsRow (added)
- **Decision:**
  - **PageHeader:** one shared component (re-skinned `PageHero`): `title` 24/30, a 13px `ink-muted` lede, and an optional SectionLabel eyebrow.
  - **Column edge:** one per page; header and body share max-width and padding. This fixes "four left edges" (L18).
    - Settings 820.
    - Home, Health and Diagnostics 1080, with two columns at ≥1160 inside the same edge.
  - **SectionLabels:** numbered (`01 / PROJECTS`), mono 11 caps .12em, index in `ink-faint` (not accent), hairline + count. Used on Home, Health and Settings, never in chat.
  - **Card:** `bg-100` + `line-100`, radius 16, no shadow, padding per density (D-06), one nesting level. Preview wells are `bg-000` without fake window dots.
  - **Fit one screen:** Home and Health fit 1600×1000, with lists scrolling inside their cards. Settings and chat scroll in their own region.
  - **SettingsRow** (Rift part; Halo has none):
    - Layout: title `small` 500 `ink` with a `caption` `ink-muted` description on the left, the control on the right, a hairline between rows, min height 44.
    - Booleans: Toggles that apply instantly.
    - Segmented: keeps `role=radio` + `onRadioKey`, with Halo's sliding thumb.
    - Expanders: `<details>` becomes Accordion.
    - Contract: `#card-*`, `.sflash` and `jumpTo` are preserved.
- **Why:** L18 (four edges), L5 (one level deep), K13 (fake chrome). Stacked Halo Field rows would roughly double page height. The accent index would break accent scarcity (X21).
- **Dissent:** the fidelity lens wanted a 32/38 title and an accent index (Halo SectionLabel).
- **Reversibility:** cheap-medium. **Gate:** no. **Unblocks:** P4d-f.

### D-32 · Copy, icons, hit targets (added)
- **Decision:**
  - Sentence case everywhere: "Claude usage & health", and the "Experimental" eyebrow goes.
  - No emoji or text glyphs in UI (`📋` at `Markdown:482`, `✓`, `ℹ`, `⚠`). Use lucide icons, stroke 1.8, 16px beside text and 18px in nav.
  - Copy failures are shown, not swallowed.
  - Empty states say what appears there and offer one action. Errors say what happened and what to do.
  - Hit targets are ≥28px in compact; the 18-20px copy/kill buttons grow.
  - Hover-only actions appear only where the owner chose them (Ledger Copy/Retry). Card actions sit visible at rest at reduced opacity.
- **Why:** owner P1/P3 (honest, plain copy). Halo's content rules agree.
- **Reversibility:** cheap. **Gate:** no. **Unblocks:** the P5 copy and icon sweep.

### D-33 · Markup contracts, geometry couplings, undefined vars (added)
- **Contracts:** these names stay. Renaming one means updating every consumer (CDP scripts, rift-ui skill docs, HUDs) in the same commit.
  - Stream: `.sturn`, `.bubble[data-role=user]`, `.stream`, `.csurf-col`, `.splan`, `#sacard-<id>`, `#actnode-*` + `.tid-flash`, `dem-add`/`dem-del`.
  - Composer: `.pane-shell textarea`, `.composer-wrap`, `.ctxring`, `data-composer-popover`, `[data-qedit]`, `#model-effort-menu`.
  - Shell and pages: `.sidebar`, `.ob-host`, `#card-*` + `.card` + `.sflash`, `settings-tab-*`/`settings-panel-*`, `.ah-wrap`.
  - Other hooks: `window.__riftDiag`, `[data-a11y-*]`, and the sidebar aria-labels used by `c.sh nav`.
- **Couplings:** each becomes a CSS var read by JS, or a shared TS constant, in the phase that changes its size:
  - `sliderBubble.ts` `thumbW=18` (Halo's thumb is 20);
  - Diagnostics `ROW_H=26`, which must equal `.dc-row` (it becomes the density row);
  - the mic-ring `rx=10` and its viewBox;
  - the `.dict-ghost`/`.magic-text` padding, which must equal the textarea padding (already 1-2px off);
  - `composerH`;
  - the `--chat-col` offsets;
  - Settings `measureInk`.
- **Undefined vars, fixed in P1:**
  - `WorkspaceShell.svelte:155-187`: `--text-primary/-secondary/-muted`, `--border-subtle` and `--bg-raised` → `ink` / `ink-muted` / `ink-faint` / `line-100` / `bg-200`.
  - `--bg-0` (StreamAskUser) → `bg-000`.
  - `--font-sans` (`SettingsPage:1707`) will start resolving to Geist; check that this is intended.
  - The token test flags any undefined `var()` name.
- **Reversibility:** cheap. **Gate:** no. **Unblocks:** every surface port.

### D-34 · Native + pre-JS colour (added)
- **Decision (all in P1, with the tokens):**
  - `app.html:9` inline background and `#first-paint` → `#08090c`. `<html data-theme="blue" data-density="compact">`.
  - The browser child's background, `src-tauri/src/browser/mod.rs:218` → `Color(8,9,12,255)`. Verify on Windows.
  - Splash panels → `bg-000`.
  - The `Markdown.svelte:1205` literal `#22272e` and the warm-tint literals → tokens.
  - The dead `data-direction`/`data-mode` selector (`app.css:129-130`) → `:root`.
- **Reversibility:** cheap. **Gate:** no. **Unblocks:** P1, so there is no colour flash at boot.

## Superseded rules

The source is `design-system/DESIGN.md` (the Graphite Ink taste layer). P1 rewrites it as "Rift on Halo". §8 is kept word for word, with each rule tagged.

### Retired by Halo
- **§1** "Graphite Ink" look and the emerald default accent → Halo `blue`, Electric blue #6aa8ff (D-02).
- **§2 surfaces:** the five-rung ladder `--bg` → `--bg-inset` → `--bg-elev-1/2/3` becomes `bg-000/100/200/300`. The rule "wells sit on a darker `--bg-inset`" (§2, §7) is replaced by the Halo input recipe (`bg-100` + `line-300`; terminals on `bg-000`) (D-08, D-22).
- **§2 accent:** "the ramp derives from `--accent-h/c/a`, a hue change restyles for free" → a fixed hex, with the dial path opt-in only (D-02).
- **§2 status:** the green `--ok` is gone; positive = accent (D-03). The rest of "status ≠ activity ≠ accent" still holds for warn, danger and info.
- **§3 fonts:** Inter (optical features, −0.005em) and JetBrains Mono → Geist and Geist Mono. The 11→22 scale → the D-06 tokens. Section label 10px/600/.08em → SectionLabel mono 11/14 500 .12em.
- **§3 mono:** "mono never for chrome labels" is narrowed. Mono is allowed for SectionLabel, Kbd, Stat metric and table data. It is still banned for timestamps and receipt meta (D-05).
- **§4** radius ladder 4/6/8/10/12/16 → 6/8/12/16/pill (D-07).
- **§5 depth:** the depth table (`elev-3` + `--shadow`), `--ring` and the whole atmosphere doctrine (onyx slab, recessed chassis, bevel, grain, contact AO, accent seam) → `bg-100`/`bg-200` + `shadow-pop`, `shadow-focus`, and flat islands. The slab's retirement is owner-gated (#1). The seam survives only in the splash.
- **§6 motion:** 140/240/460, stagger 62, `--ease-page`/`--ease-soft` → 150/300/600, stagger 60, one `ease-out` (D-10).
- **§7 stream tiles:** the 2px inset edge on `is-bad`/`is-ask`, `.tile-pill` and ChevronRight 90° → one signal per state, Badge, and chevron-down 180° (D-12, D-21).
- **§8 "frame tool activity as islands":** the "faint fg-2.5% fill" becomes a `bg-100` tile. The frame itself still binds.
- **§8 "island as one dialect":** `--island-fill` and the gradient docked tier → flat `bg-100`. HUDs' "frosted `--surface` 84%" divergence → opaque `bg-200` (D-15).
- **§10 buttons:** sizes 26/22/20 + `lg` 32 → density-tied 32/28 (D-06).
- **§10 floating tier:** `--surface` + `--border-strong` + `--shadow-lg` → `bg-100` + `line-200` + `shadow-pop`.
- **§10 layers:** "`--z-dialog` 4000 (above toasts + notification center)" is reversed. Toasts (60) now sit above dialogs (50), so "Copied" and "Saved" stay visible.
- **§10 motion:** "95% zoom + 8px, `--dur-fast`, `--ease-page`" → `halo-zoom` .94 + 8px, 300ms, `ease-out`.

### Still binding
- **§1:** "Does the chrome get louder than the content?" The conversation is the protagonist.
- **§2:**
  - Lift + hairline, shadows only for floating layers.
  - Solid accent = Send + at most one CTA per view.
  - One chromatic voice.
  - A second decorative hue is a bug.
  - Never hard-code an accent-ish hex.
- **§4:**
  - Density is user-set.
  - Compact is the default posture.
  - New components consume the density vars.
- **§6:** motion is for orientation, not delight. One orchestrated entrance. Respect reduced motion.
- **§7:**
  - Buttons are ghost-first.
  - Copy rules.
  - In-card action bars.
  - One tile family.
  - Glyph colour = status, never tool kind.
  - A component owns its content, not its chrome.
- **§8 (every entry, as taste memory):**
  - Chrome quieter than content.
  - One ChatGPT identity; matching provider cards; one fact, one home.
  - No second accent, no atmospheric gradients, no spotlight cards.
  - No purple/lavender defaults. Watch the violet series colour (D-24).
  - No oversized hero numerals. No pill buttons or inputs.
  - No hard-coded hex, row heights or durations.
  - No divergent `var()` fallbacks.
  - No stacked kbd hints in placeholders.
  - Tool activity framed.
  - The peek is a minimized sibling.
  - One level deep.
  - Quiet ≠ invisible (answered by the `bg-000` → `bg-100` step).
  - No stacked atmosphere.
  - No specks or dots.
  - Grep for material variants after any material change.
  - Cost = amber at the control.
  - No banded sweeps (D-14).
  - No sunken panel in a flat surface.
  - No orbiting light (D-13).
- **§9:** the agent guide. A rejection goes into §8 the same session.
- **§10:**
  - Compose from `ui/` first.
  - One vocabulary through the Tailwind bridge.
  - One `recipes.ts`.
  - A menu inside a dialog must work (dropdown 65 > overlay 50).
  - The scrim is opaque, never `backdrop-filter`.
  - Never run `shadcn-svelte init`.
  - The `check:ui` ratchet.

## Stale facts corrected

- **Pacing docs.** `docs/HANDOFF.md:26-36` and `docs/design/redesign-plan.md:178-216` still describe the old pacing (42ms/word, 520ms catch-up, 0.2 lerp). The code at 22727d24 is:
  - `WORD_MS` 30, `CATCHUP_LAG` = `ceil(320/30)` = 11 words, lerp 0.35;
  - `md-word` = 500ms + `blur(3px)` on `cubic-bezier(.22,1,.36,1)` (`Markdown.svelte:169-172,684`);
  - `blockIn` 3px on `--dur-base` (`stream.css:27`).
- **Preferences file.** `docs/design/PREFERENCES.md` no longer exists. The rejection log is `design-system/DESIGN.md` §8.
- **DESIGN.md location.** It lives at `design-system/DESIGN.md`, not under `docs/design/`.
- **Tooltip sites.** The `actions/tooltip.ts` header says "~26 sites"; there are 174 call sites in 37 files (map 07).
- **Dialog vs toast layering.** The `app.css:277` comment and DESIGN §10 say dialogs sit above toasts. Only `ui/` dialogs use `--z-dialog` 4000 (`app.css:281`). The bespoke dialogs (UpdateDialog, MCP, palette) sit at z 200, below toasts (map 07 §2).
- **Rail timing.** `--rail-w` tweens on `--dur-page-out` 140ms (`app.css:244,353`) while the rail it follows animates over 240ms, a confirmed desync. D-19 makes both 300ms.
- **Menu motion.** `recipes.ts:20,58,60` animates on `data-open:`/`data-closed:`, which bits-ui likely never emits (it sets `data-state`). The menu motion is suspected inert, not yet confirmed; D-28 #1 checks it.
- **Scrollbars.** The premise "scrollbars are hidden" is false. Rift has thin 10px bars, and only 5 sites hide them (X29).
- **Geist.** It is not installed. `node_modules/@fontsource-variable/` holds only inter, jetbrains-mono and lexend.
- **Halo blue info.** In Halo's `blue` preset `info` is lime #c6f35a, not blue. The `chart-2` value is still to be confirmed (D-28 #9).
- **Text colours.** `--fg-subtle` is already #7B808A (lifted in 7ffa450a), effectively Halo `ink-faint`. `--fg-faint` #4B5056 is the rung below 4.5:1.
- **Settings keyframe.** SettingsPage declares its own `blockIn` keyframe with an 8px rise (`SettingsPage.svelte:1590`), separate from the stream's 3px one. The P2 motion home unifies both under the D-10 rises.
- **Browser hide.** The native browser hide/show IPC already exists (`WebBrowserPage.svelte:375-377`), so the overlay contract (D-20) needs no new Rust command.
- **Halo popup speed.** Halo's docs say popups scale in at `dur-base`, but its CSS runs `halo-pop-in` at `dur-fast` (map 10:108). This register follows the CSS.
- **Spec folder.** `docs/design/halo/spec/` did not exist before this wave, and `docs/design/` is gitignored, so this spec is untracked by design.
