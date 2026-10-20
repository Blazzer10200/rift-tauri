---
title: Rift · Halo — spec 01 foundation (P0 + P1)
status: draft
updated: 2026-10-19
phase: P0 (spikes, baselines, branch) + P1 (tokens, fonts, z ladder, native colour, gates)
depends_on:
  - spec/00-decisions.md (D-02 D-03 D-04 D-05 D-06 D-07 D-08 D-23 D-25 D-28 D-30 D-33 D-34)
  - map/08-foundation-tooling.md
  - map/11-consolex-precedent.md
tags: [rift, halo, spec, tokens, p0, p1]
---

# Spec 01 — Foundation (P0 + P1)

Paths are relative to `C:/AI Workflow/projects/rift-tauri` unless prefixed. `CX/` = `C:/AI Workflow/projects/consolex/panel`. `HALO/` = the Halo brand-book export (`…/scratchpad/artifact-files/705c5425-86e3-4761-a421-4ee89499e5c6/project/`). Every `path:line` below was opened on `design/chat-ledger @ 22727d24` on 2026-10-19. `c.sh` = `bash scripts/cdp/c.sh`.

## 0. Summary

- **P0** cuts `design/halo` off 22727d24, captures CDP baselines, and runs the nine D-28 spikes. Two of them (#2 italic, #9 chart-2) are already settled from source. P0 lands no source commits.
- **P1** loads Halo's generated `tokens.css` before `app.css` and turns `app.css:129-285` into a permanent alias layer. Every Rift name is kept and points at a Halo var. A short `:root[data-theme="blue"]` deviation block holds the overrides, and a `[data-accent="custom"]` block keeps the dials as an opt-in.
- P1 also brings in Geist, the D-06 type and density tokens (defined but not yet used), the D-07 radius rename, the z ladder (24 layer literals tokenised), native and pre-JS colours, the undefined-var fixes, a rewritten `check:tokens`, new `check:ui` metrics, the token vitest suite and the DESIGN.md rewrite.
- **Visible result:** the whole app recolours (Electric blue, a darker Halo ground, no green) and refonts (Geist). Some radii grow by 2px. Floats lose their heavy shadows. **Layout, sizing and motion feel do not change** (`--dur-base` stays at 240 until shortlist #3).
- About 24 commits, one concern each, on `halo/p1-foundation`. Each commit passes `npm run check` + `npm test` on its own.

## 1. Scope + files

### 1.1 In scope
- P0: branch, CDP baselines and measure probes, D-28 spikes 1-9, stale-doc notes.
- P1: D-02 (accent default and opt-in mechanics), D-03 (semantic aliases), D-04 (dark `blue` only), D-05 (Geist), D-06 (tokens only, no consumers), D-07 (rename and ladder values), D-08 (alias layer, gates, z ladder), D-23 (scrollbars), D-30 (text ladder and the a11y reach fix), D-33 (undefined vars, `c.sh` coupling), D-34 (native and pre-JS colour).

### 1.2 Out of scope (owner phase in brackets)
- Layout and sizing consumers of the D-06 vars: `--row-h` 26→32, `--fs-md` 12.5→13, `--chat-col` 760, the 261 sub-12px lifts [P3/P4].
- Motion aliases `--ease-page`/`--ease-soft`→`ease-out`, `--dur-rise`→`dur-slow`, `--dur-page` 520, and the kill list [P2] (see Amendment A-2).
- Flattening the slab, grain or islands [P4a, gated #1]. The Shiki theme [P4b]. Any dial UI removal [gated #5]. Card, mirror and manifest deletion [gated #6].
- Removing the Inter and JetBrains packages [P5].

### 1.3 Files touched in P1

| File | Lines | P1 change |
|---|---|---|
| `src/app.css` | 780 | token region `:129-285` rewritten (alias, deviation, custom, density); `:306,314` dropped; scrollbars `:320-337`; `.slider-bubble` z `:372`; `.dialog-overlay` z `:616`; a11y `:747-780` |
| `src/app.html` | 22 | `:2` attributes; `:9` first-paint colour |
| `src/routes/+layout.svelte` | 67 | `:2-5` import order |
| `src/lib/styles/halo/tokens.json` | new (641) | vendored, byte-identical to `HALO/tokens.json` |
| `src/lib/styles/halo/tokens.css` | new (219) | generated, never hand-edited |
| `src/lib/styles/halo/PROVENANCE.md` | new | source, hashes, regen command |
| `design-system/halo-tokens.mjs` | new (~55) | generator (port of `CX/src/host/design-tokens.ts`, 45 lines) and CLI |
| `design-system/token-graph.mjs` | new (~180) | shared CSS block parser and resolver, plus the `DEVIATIONS`/`PENDING_HALO_WINS`/`BRIDGE_KEYS`/`RUNTIME_VARS` lists |
| `design-system/check-tokens.mjs` | 159 → ~140 | rewritten (§5.3) |
| `design-system/check-ui.mjs` | 188 | 4 metrics added; `.css` scan (§5.4) |
| `design-system/ui-baseline.json` | 8 | rewritten by `--update` (C6, C24) |
| `design-system/DESIGN.md` | 307 | rewritten "Rift on Halo" (§6 C23) |
| `src/lib/styles/halo-tokens.test.ts`, `src/lib/styles/tokens.test.ts`, `src/lib/styles/css-vars.test.ts` | new | vitest (§5.5) |
| `src/lib/state/ui-prefs.svelte.ts` | 307 | `:7-15,76-100,123-139,181-210,291-296` accent opt-in |
| `src/lib/components/settings/SettingsPage.svelte` | 2023 | `:718,723-724,908,1707,1624,1801,1841` |
| `src/lib/components/onboarding/OnboardingFlow.svelte` | 392 | `:235-245` swatch selection |
| `src/routes/dev/ui/+page.svelte` | 445 | `:52-67,92-100,231` |
| `src/lib/components/shell/WorkspaceShell.svelte` | 212 | `:155,177,180,183,184,187` |
| `src/lib/components/assistant/stream/StreamAskUser.svelte` | 427 | `:400` |
| `src/lib/components/AppShell.svelte` | 513 | `:358` (tracking), `:408` (accent literal) |
| `src/lib/components/shell/Sidebar.svelte` | 352 | `:218` (z), `:231,267` (accent literal) |
| `src/lib/components/SplashOverlay.svelte` | 293 | `:130` (z), `:146-148,159-160` (panels) |
| `src-tauri/src/browser/mod.rs` | 926 | `:216-218` |
| `scripts/cdp/c.sh` | 665 | `:549` |
| radius rename (15 sites) | — | §2.8 |
| z layer sites (24) | — | §2.9 |
| accent literal sites | — | `WorkspacePage.svelte:919`, `LimitBar.svelte:39`, `UsagePanel.svelte:309` |
| `package.json` / `package-lock.json` | 77 | 2 deps |

### 1.4 P0 results (fill in during P0; this table is the record P1 reads)

| # | Spike | Settles | Result | Evidence |
|---|---|---|---|---|
| 1 | `floatingMotion` dead? | D-09 `data-[state=open]:` fix | _pending_ | |
| 2 | Geist italic CSS; Geist Mono ligatures | D-05 import list, `--code-liga` toggle | italic: **settled, yes**. Ligatures: _pending_ | `CX/node_modules/@fontsource-variable/geist{,-mono}/wght-italic.css` (5.3.0) |
| 3 | `interpolate-size` in WebView2 | D-12 `.wline` morph | _pending_ | |
| 4 | Drag regions, double-click maximise, snap | D-19 / P4a top bar | _pending (manual)_ | |
| 5 | lucide twice under Vite prebundle | P3/P4 icon imports | _pending_ | |
| 6 | floating-ui import path | D-17 tooltip | _pending_ | `node_modules/@floating-ui/dom` 1.8.0 (bits-ui transitive) |
| 7 | Hydration replays `blockIn`? | D-29 size | _pending_ | |
| 8 | End-of-turn blink recorded | D-12 | _pending_ | |
| 9 | Halo blue `--chart-2` | D-03 override | **settled: lime**. `tokens.css:71` `--chart-2: var(--info)` and `:68` `--info: #c6f35a` | CX `design-systems/halo/tokens.css` |

## 2. Target design

### 2.1 Load order + cascade contract

```
+layout.svelte imports (order is the contract, D-08)
  1 @fontsource-variable/geist            (+ /wght-italic.css)
  2 @fontsource-variable/geist-mono       (+ /wght-italic.css)
  3 @fontsource-variable/lexend           (dyslexia + splash only)
  4 $lib/styles/halo/tokens.css           unlayered · :root,[data-theme=dark] (0,1,0) · [data-theme=blue] (0,1,0)
  5 ../app.css                            @import tailwindcss (layered) · then, unlayered and in this order:
       @halo:aliases        :root                                      (0,1,0)  Rift names → Halo vars + Rift-only tokens
       @halo:deviations     :root[data-theme="blue"]                   (0,2,0)  beats tokens.css [data-theme=blue]
       @halo:custom-accent  :root[data-theme="blue"][data-accent="custom"], [data-accent="custom"]   (0,3,0)
       @halo:density        :root[data-density="compact|regular|comfy"] (0,2,0)
<html lang="en" data-theme="blue" data-density="compact">   (+ data-accent="custom" only when a dial was touched)
```

Rules:
- **Halo-owned names** (any `--x` that `tokens.css` declares) appear in `app.css` only inside `@halo:deviations` or `@halo:custom-accent`, and each one is listed in `DEVIATIONS` (§5.2). `check:tokens` enforces this.
- `@theme inline` declarations are **bridge keys**, not tokens. The parser keeps them in a separate namespace. (`--font-sans: var(--font-ui)` in the bridge plus `--font-ui: var(--font-sans)` in the aliases is not a cycle: the layered bridge copy loses the cascade to the unlayered deviation.)
- Each block opens with a marker comment (`/* @halo:aliases */` …) on the line before its selector. The parser reads markers **before** it strips comments.

### 2.2 Vendored tokens

- **Path:** `src/lib/styles/halo/tokens.json` (vendored source) and `src/lib/styles/halo/tokens.css` (generated). Do not vendor `bundle.css`: it is React-oriented, and its Google `@import` violates CSP `font-src 'self' data:` (`src-tauri/tauri.conf.json:28`). `tokens.css` has no `@import`.
- **Generator:** `design-system/halo-tokens.mjs` exports `tokensCss(json)`. It is a line-for-line port of `CX/src/host/design-tokens.ts:15-45`, with one change: the header string names `design-system/halo-tokens.mjs`. CLI:
  - `node design-system/halo-tokens.mjs --write` regenerates `tokens.css`.
  - `node design-system/halo-tokens.mjs --check` exits 1 if the file is stale.
  - `node design-system/halo-tokens.mjs --upstream <dir>` compares sha256 against `<dir>/tokens.json` and only reports (INFO).
- **In-file header:** line 1 of the generated file is its only header: `/* Halo — generated from tokens.json (design-system/halo-tokens.mjs); do not edit */`. It must stay a single line, because the CX tail-diff (`tail -n +2`) depends on that.
- **Provenance** (`PROVENANCE.md`, not inside the generated file):
  - Source: Claude Design system "Halo", `tokens.json` `{"name":"Halo","version":2}`, from the export `artifact-files/705c5425-86e3-4761-a421-4ee89499e5c6`.
  - sha256 `3a5cf99cad6bacacd179f1422880d0ecb2176ece29e39cd805c84b477014094f`. This is identical to `CX/design-systems/halo/tokens.json` at CX `009f124`, vendored in CX `0eabd84` ("Halo v2.3", 2026-10-14).
  - Generated CSS: CX's file has sha256 `b7210a67…1e1c`. Rift's differs only in line 1 (the header).
  - Regenerate: `node design-system/halo-tokens.mjs --write`. Never hand-edit. Light blocks stay inert (D-04).
- **Freshness, three layers:**
  1. `check:tokens` rule STALE: `tokens.css === tokensCss(tokens.json)`, with CRLF normalised to LF.
  2. Vitest `halo-tokens.test.ts`: the same equality.
  3. At vendoring time (C3) only: `diff <(tail -n +2 src/lib/styles/halo/tokens.css) <(tail -n +2 "C:/AI Workflow/projects/consolex/panel/design-systems/halo/tokens.css")` must print nothing.

### 2.3 Full alias table — all 99 Rift tokens (`app.css:132-282`)

"Halo" = deleted from `app.css` so Halo's value wins. "dev" = the deviation block. "kept" = a Rift-only literal or formula left as it is (it re-resolves on Halo inputs). Resolved values are for `data-theme="blue"` with no custom accent. **C** = the P1 commit (§6).

**Surfaces, lines, text** (C13)

| Rift token | Today | P1 value | Resolves to |
|---|---|---|---|
| `--bg` :132 | oklch(.142 .003 250) | `var(--bg-000)` | #08090c |
| `--bg-inset` :133 | oklch(.178…) | `var(--bg-000)` | #08090c |
| `--bg-elev-1` :134 | oklch(.215…) | `var(--bg-100)` | #0e1014 |
| `--bg-elev-2` :135 | oklch(.262…) | `var(--bg-200)` | #14171c |
| `--bg-elev-3` :136 | oklch(.300…) | `var(--bg-300)` | #1b1f26 |
| `--surface` :137 | = elev-1 | `var(--bg-100)` | #0e1014 |
| `--surface-hover` :138 | = elev-2 | `var(--bg-200)` | #14171c |
| `--surface-active` :139 | = elev-3 | `var(--bg-300)` | #1b1f26 |
| `--code-bg` :143 | mix fg 8% | kept formula | ink 8% over parent |
| `--code-border` :144 | mix fg 13% | kept formula | ink 13% |
| `--code-fg` :145 | `var(--fg)` | kept | #eceff3 |
| `--field` :151 | mix fg 5% on surface | `var(--bg-100)` | #0e1014 |
| `--field-border` :152 | = border-strong | `var(--line-300)` | #5f6774 (3.34:1 on bg-100) |
| `--track` :153 | mix bg 55% surface | `var(--bg-300)` | #1b1f26 |
| `--border` :156 | oklch(.300…) | `var(--line-100)` | #1f242c |
| `--border-strong` :157 | oklch(.365…) | `var(--line-200)` | #2c323c |
| `--border-focus` :158 | accent derived | `var(--accent)` | #6aa8ff |
| `--fg` :161 | #E5E7EB | `var(--ink)` | #eceff3 |
| `--fg-2` :162 | #C3C8CF | `var(--ink)` | #eceff3 (D-30) |
| `--fg-muted` :163 | #9CA3AF | `var(--ink-muted)` | #9aa2af (7.40:1 on bg-100) |
| `--fg-subtle` :164 | #7B808A | `var(--ink-faint)` | #7d8592 (5.12:1 on bg-100) |
| `--fg-faint` :165 | #4B5056 | `var(--ink-faint)` | #7d8592 (lift, D-30) |

**Accent + semantic** (C13; `--accent-h/c/a` defaults set in C19)

| Rift token | Today | P1 value | Resolves to |
|---|---|---|---|
| `--accent-h` :171 | 163 | kept, **257** (dial input) | — |
| `--accent-c` :172 | 0.15 | kept, **0.143** | — |
| `--accent-a` :173 | 0.12 | kept 0.12. No consumer after C13; flagged §8 | — |
| `--accent` :174 | oklch derived | **Halo** | #6aa8ff (7.85:1 on bg-100) |
| `--accent-hover` :175 | oklch .86 | `var(--accent)` | #6aa8ff |
| `--accent-active` :176 | oklch .70 | `var(--accent)` | #6aa8ff |
| `--accent-fg` :177 | oklch(.25 .05 h) | `var(--accent-ink)` | #08090c (8.21:1) |
| `--accent-soft` :178 | a=.12 | **Halo** | rgba(106,168,255,.14) |
| `--ghost-border` :179 | accent ~.35 | `var(--line-200)` (D-08) | #2c323c |
| `--ring` :180 | accent ~.45 | `var(--accent)` (A-1) | #6aa8ff |
| `--status-busy` :184 | #22c55e | `var(--accent)` | #6aa8ff |
| `--status-pending` :185 | #3b82f6 | `var(--ink-muted)` | #9aa2af |
| `--status-warn` :186 | #e5a54b | `var(--warn)` | #ff9f5a |
| `--ok` :189 | green | `var(--accent)` | #6aa8ff |
| `--ok-soft` :190 | green .14 | `var(--accent-soft)` | rgba(106,168,255,.14) |
| `--warn` / `--warn-soft` :191-192 | oklch 70 | **Halo** | #ff9f5a / rgba(255,159,90,.14) |
| `--danger` / `--danger-soft` :193,195 | oklch 22 | **Halo** | #ff6b6b / rgba(255,107,107,.14) |
| `--danger-fg` :194 | oklch(.985…) | `var(--bg-000)` (A-5) | #08090c (7.17:1 on danger; ink would be 2.41:1) |
| `--info` / `--info-soft` :196-197 | oklch 230 | **dev** | #9cc6ff / rgba(156,198,255,.14) |
| `--scrim` :276 | color-mix | **Halo** | rgba(4,5,7,.72) |

**Shape + shadow** (C10 rename, C15 values)

| Rift token | Today | P1 value | Resolves to |
|---|---|---|---|
| `--radius-xs` :200 | 4px | `6px` (D-07) | 6 |
| `--radius-sm` :201 | 6px | **Halo** | 8 |
| `--radius` :202 | 8px | `var(--radius-sm)` | 8 |
| `--radius-lg` :203 | 10px | **Halo** (after the C10 rename) | 16 |
| `--radius-xl` :204 | 12px | `var(--radius-md)` | 12 |
| `--r-card` :205 | 12px | `var(--radius-md)` (kept 12, A-7) | 12 |
| `--radius-2xl` :206 | 16px | `var(--radius-lg)` | 16 |
| `--island-radius` :210 | 14px | kept 14 (P4a → 16, gated #1) | 14 |
| `--island-border` :211 | `var(--border)` | kept | #1f242c |
| `--island-fill` :212 | fg 2.8% | kept formula (P4a → bg-100) | ink 2.8% |
| `--tile-radius` :217 | `var(--radius-lg)`=10 | `var(--radius-md)` (C10) | 12 (= D-21 tile radius) |
| `--tile-radius-inner` :218 | `var(--radius-sm)`=6 | kept ref | 8 |
| `--tile-fill` / `-hover` :219-220 | fg 2.8% / 4% | kept formulas | ink 2.8% / 4% |
| `--tile-border` / `-hover` :221-222 | border / strong | kept refs | #1f242c / #2c323c |
| `--tile-border-live/-bad/-ask` :223-225 | mixes | kept formulas | accent 30% / danger 40% / accent 42% over line-100 |
| `--tile-well` :226 | bg-inset 70% | kept formula | bg-000 70% |
| `--tile-head-pad`, `-compact`, `-head-fs`, `-meta-fs` 10px, `-edge` 2px :227-231 | literals | kept (P4b) | — |
| `--shadow-sm` :234 | 0 1px 2px | `0 0 transparent` (A-4) | no shadow |
| `--shadow` :235 | 0 4px 14px | `0 0 transparent` (A-4) | no shadow |
| `--shadow-lg` :236 | 0 24px 64px .6 | `var(--shadow-pop)` | 0 12px 32px rgba(0,0,0,.45) |
| `--shadow-float` :237 | double | `var(--shadow-pop)` | same |

**Motion** (C16; see A-2)

| Rift token | Today | P1 value | Resolves to |
|---|---|---|---|
| `--ease-page` :240 | (.22,1,.36,1) | kept (P2 → `var(--ease-out)`) | — |
| `--ease-soft` :241 | (.4,0,.2,1) | kept (P2) | — |
| `--ease-spring` :242 | (.34,1.56,.64,1) | **Halo** (identical) | same |
| `--dur-page` / `-page-out` / `-rise` :243-245 | 400 / 140 / 460 | kept (P2) | — |
| `--stagger` :246 | 62ms | **Halo** | 60ms |
| `--dur-fast` :247 | 140ms | **Halo** | 150ms |
| `--dur-base` :248 | 240ms | **dev** 240ms (pending #3) | 240ms |
| `--dur-slow` :249 | 520ms | **Halo** | 600ms |
| `--pulse-live` :250 | 1.6s | kept (P2 retires) | — |

**Density, type, layout, layers** (C11 type, C14 fonts, C17 z)

| Rift token | Today | P1 value | Resolves to |
|---|---|---|---|
| `--row-h` / `--gap` :253-254 | 28 / 8 (+density) | kept (P3 → `--list-row-h`) | — |
| `--font-ui` :257 | Inter stack | `var(--font-sans)` | Geist Variable |
| `--font-mono` :258 | JetBrains stack | **dev** | Geist Mono Variable |
| `--titlebar-h` :259 | 34px | kept | — |
| `--fs-xs` :260 | 11 | `var(--t-label-size)` | 11 |
| `--fs-sm` :261 | 12 | `var(--t-caption-size)` | 12 |
| `--fs-md` :262 | 13 (compact 12.5) | `var(--t-small-size)`; the compact 12.5 override is kept (P3) | 12.5 compact / 13 regular |
| `--fs-lg` :263 | 15 | `var(--t-body-size)` | 15 |
| `--fs-xl` :264 | 18 | `var(--t-subtitle-size)` | 18 |
| `--fs-2xl` :265 | 22 | kept 22 (no rung; flagged §14) | 22 |
| `--chat-col-max` :272 | min(1100,…) | kept (P4b → `--chat-col`) | — |
| `--z-dialog` :281 | 4000 | `var(--z-overlay)` | 50 |
| `--z-popover` :282 | 9000 | `var(--z-dropdown)` | 65 |
| `color-scheme` :284 | dark | kept | — |

**New in P1** (Rift-only names, not in Halo): `--t-*` (§2.6), the density vars (§2.7), `--chat-col` (defined, not used), `--z-hud` 20, `--z-splash` 100, and `--a11y-warm-bg/-fg/-code-bg/-code-fg` (C22).

### 2.4 Target token region of `app.css` (end of P1, verbatim; replaces `:124-291`)

```css
/* ─────────────────────────────────────────────────────────────
   Rift on Halo — tokens. Halo values come from $lib/styles/halo/tokens.css
   (generated, loaded first by +layout.svelte). This file keeps every Rift name
   as an alias so consumers never change (D-08). Halo-owned names may appear
   here ONLY in @halo:deviations / @halo:custom-accent (check:tokens enforces).
   ───────────────────────────────────────────────────────────── */
/* @halo:aliases */
:root {
  /* surfaces */
  --bg: var(--bg-000);  --bg-inset: var(--bg-000);
  --bg-elev-1: var(--bg-100);  --bg-elev-2: var(--bg-200);  --bg-elev-3: var(--bg-300);
  --surface: var(--bg-100);  --surface-hover: var(--bg-200);  --surface-active: var(--bg-300);
  --code-bg: color-mix(in oklch, var(--fg) 8%, transparent);
  --code-border: color-mix(in oklch, var(--fg) 13%, transparent);
  --code-fg: var(--fg);
  --field: var(--bg-100);  --field-border: var(--line-300);  --track: var(--bg-300);
  /* lines */
  --border: var(--line-100);  --border-strong: var(--line-200);  --border-focus: var(--accent);
  /* text — five Rift rungs on Halo's three (D-30) */
  --fg: var(--ink);  --fg-2: var(--ink);  --fg-muted: var(--ink-muted);
  --fg-subtle: var(--ink-faint);  --fg-faint: var(--ink-faint);
  /* accent — fixed Electric blue from tokens.css (D-02). h/c feed the opt-in dial only. */
  --accent-h: 257;  --accent-c: 0.143;  --accent-a: 0.12;
  --accent-hover: var(--accent);  --accent-active: var(--accent);
  --accent-fg: var(--accent-ink);
  --ghost-border: var(--line-200);
  --ring: var(--accent);
  /* activity + status (D-03: no green; positive = accent) */
  --status-busy: var(--accent);  --status-pending: var(--ink-muted);  --status-warn: var(--warn);
  --ok: var(--accent);  --ok-soft: var(--accent-soft);
  --danger-fg: var(--bg-000);
  /* shape (D-07) — Halo owns --radius-sm 8 / -md 12 / -lg 16 / -pill */
  --radius-xs: 6px;  --radius: var(--radius-sm);  --radius-xl: var(--radius-md);
  --r-card: var(--radius-md);  --radius-2xl: var(--radius-lg);
  --island-radius: 14px;  --island-border: var(--border);
  --island-fill: color-mix(in oklab, var(--fg) 2.8%, transparent);
  --tile-radius: var(--radius-md);  --tile-radius-inner: var(--radius-sm);
  --tile-fill: color-mix(in oklab, var(--fg) 2.8%, transparent);
  --tile-fill-hover: color-mix(in oklab, var(--fg) 4%, transparent);
  --tile-border: var(--border);  --tile-border-hover: var(--border-strong);
  --tile-border-live: color-mix(in oklab, var(--accent) 30%, var(--border));
  --tile-border-bad: color-mix(in oklab, var(--danger) 40%, var(--border));
  --tile-border-ask: color-mix(in oklab, var(--accent) 42%, var(--border));
  --tile-well: color-mix(in oklab, var(--bg-inset) 70%, transparent);
  --tile-head-pad: 7px 11px;  --tile-head-pad-compact: 5px 10px;
  --tile-head-fs: var(--fs-sm);  --tile-meta-fs: 10px;  --tile-edge: 2px;
  /* shadow — Halo: shadows only on floating layers */
  --shadow-sm: 0 0 transparent;  --shadow: 0 0 transparent;
  --shadow-lg: var(--shadow-pop);  --shadow-float: var(--shadow-pop);
  /* motion — Halo owns --dur-fast/-slow, --stagger, --ease-spring, --ease-out. P2 re-points these: */
  --ease-page: cubic-bezier(0.22, 1, 0.36, 1);
  --ease-soft: cubic-bezier(0.4, 0, 0.2, 1);
  --dur-page: 400ms;  --dur-page-out: 140ms;  --dur-rise: 460ms;  --pulse-live: 1.6s;
  /* type (D-06) — sizes are bare lengths so a Tailwind text-* key can bridge them in P3 */
  --t-label-size: 11px;   --t-label-line: 14px;   --t-label-weight: 500;   --t-label-track: 0.12em;
  --t-caption-size: 12px; --t-caption-line: 16px;
  --t-small-size: 13px;   --t-small-line: 18px;
  --t-body-size: 15px;    --t-body-line: 24px;
  --t-heading-size: 16px; --t-heading-line: 22px; --t-heading-weight: 600;
  --t-subtitle-size: 18px; --t-subtitle-line: 24px; --t-subtitle-weight: 600;
  --t-title-size: 24px;   --t-title-line: 30px;   --t-title-weight: 600;   --t-title-track: -0.02em;
  --t-data-size: 13px;    --t-data-line: 18px;    --t-data-weight: 500;
  --t-metric-size: 28px;  --t-metric-line: 32px;  --t-metric-weight: 600;  --t-metric-track: -0.03em;
  --fs-xs: var(--t-label-size);  --fs-sm: var(--t-caption-size);  --fs-md: var(--t-small-size);
  --fs-lg: var(--t-body-size);   --fs-xl: var(--t-subtitle-size); --fs-2xl: 22px;
  --font-ui: var(--font-sans);
  /* density — compact values double as the no-attribute default (blocks below) */
  --row-h: 28px;  --gap: 8px;
  --t-chat-size: 14px;  --t-chat-line: 22px;
  --control-h: 32px;  --control-dense-h: 28px;  --list-row-h: 32px;  --tile-head-h: 32px;
  --toggle-w: 36px;  --toggle-h: 20px;  --check-size: 16px;  --card-pad: 16px;
  /* layout */
  --titlebar-h: 34px;
  --chat-col-max: min(1100px, calc(100% - 48px));   /* consumers move to --chat-col in P4b */
  --chat-col: min(760px, calc(100% - 48px));         /* D-06; unconsumed in P1 */
  /* layers (D-08) — Halo: raised 1 · sticky 10 · overlay 50 · toast 60 · tooltip 70 */
  --z-hud: 20;  --z-splash: 100;
  --z-dialog: var(--z-overlay);  --z-popover: var(--z-dropdown);
  /* reading comfort (D-34) — warm tint, dark-only */
  --a11y-warm-bg: oklch(0.225 0.014 60);       --a11y-warm-fg: oklch(0.94 0.018 80);
  --a11y-warm-code-bg: oklch(0.27 0.018 60);   --a11y-warm-code-fg: oklch(0.96 0.022 80);
  color-scheme: dark;
}
```

### 2.5 Deviation + custom-accent blocks (verbatim, directly after §2.4)

```css
/* @halo:deviations — Rift-on-Halo overrides of Halo-owned names. Each name is in
   DEVIATIONS (design-system/token-graph.mjs) with its decision id. (0,2,0) outranks
   tokens.css [data-theme="blue"] (0,1,0) regardless of import order. */
:root[data-theme="blue"] {
  /* D-03 — no green anywhere: info is light blue, not Halo blue's lime #c6f35a */
  --info: #9cc6ff;
  --info-soft: rgba(156, 198, 255, 0.14);
  /* D-03/D-24 — series: Halo blue's chart-2 = var(--info) = lime */
  --chart-2: #b79cff;
  --chart-3: #9cc6ff;
  /* D-05 — fontsource family names (Halo names the Google build "Geist") */
  --font-sans: 'Geist Variable', ui-sans-serif, system-ui, 'Segoe UI', sans-serif;
  --font-mono: 'Geist Mono Variable', ui-monospace, 'Cascadia Mono', monospace;
  /* D-08 — a menu inside a dialog must work: dropdown (65) above overlay (50); Halo has 20 */
  --z-dropdown: 65;
  /* D-10 / shortlist #3 — Rift's 240 held until the motion lab. The Halo 300 flip is
     commit "tokens(motion): --dur-base 300" on halo/p1-dur-base-300, unmerged. */
  --dur-base: 240ms;
}

/* @halo:custom-accent — opt-in dial path (D-02). ui-prefs sets <html data-accent="custom">
   only after a hue/vividness dial is touched. Derivatives are color-mix of --accent, never
   rgb triplets. The bare [data-accent] selector lets /dev/ui preview a swatch on one element. */
:root[data-theme="blue"][data-accent="custom"],
[data-accent="custom"] {
  --accent: oklch(0.727 var(--accent-c) var(--accent-h));
  --accent-soft: color-mix(in srgb, var(--accent) 14%, transparent);
  --glow-strong: color-mix(in srgb, var(--accent) 28%, transparent);
  --shadow-focus: 0 0 0 2px var(--bg-100), 0 0 0 4px var(--accent);
}
```

The custom path at `--accent-h: 257; --accent-c: 0.143` gives oklch(0.727 0.143 257), which is #6aa8ff within rounding (computed). `--accent-ink` stays Halo's #08090c. `--chart-1: var(--accent)` and `--border-focus` follow the custom accent on their own.

### 2.6 Type tokens `--t-*` (D-06)

| Token | size / line | weight | track | family + notes | Halo native (for the record) |
|---|---|---|---|---|---|
| label | 11 / 14 | 500 | .12em | mono, uppercase; SectionLabel + Kbd only | same |
| caption | 12 / 16 | 400 | 0 | sans; smallest sans | same |
| small | 13 / 18 | 400 | 0 | sans; UI default | same |
| chat | 14/22 · 15/24 · 16/26 | 400 | 0 | per density (§2.7) | — |
| body | 15 / 24 | 400 | 0 | sans | 15/22 |
| heading | 16 / 22 | 600 | 0 | sans | same |
| subtitle | 18 / 24 | 600 | 0 | sans | 20/28 −.02em |
| title | 24 / 30 | 600 | −.02em | sans | 32/38 −.03em |
| data | 13 / 18 | 500 | 0 | mono, `tabular-nums` | same |
| metric | 28 / 32 | 600 | −.03em | mono, `tabular-nums` | 30/34 |

No `display`. Weights 400/500/600 only. The 400-weight and zero-track vars are omitted on purpose: a consumer writes `font-weight: var(--t-x-weight, 400)` only where it needs one. Tailwind bridge entries (`--text-caption: var(--t-caption-size)`; `--text-caption--line-height: var(--t-caption-line)`) and `createCn` `text: [...]` registration land in P3, with the first `ui/` consumer (D-08). P1 adds no bridge keys.

### 2.7 Density blocks (verbatim; replace `app.css:287-291`)

```css
/* @halo:density — D-06 tiers. The legacy trio (--row-h --gap --fs-md) keeps its pre-Halo
   values until P3 re-points consumers; the D-06 vars are defined but unconsumed in P1.
   :root prefix → (0,2,0), ties the deviation block and wins by order. */
:root[data-density="compact"] {
  --row-h: 26px; --gap: 6px; --fs-md: 12.5px;
  --t-chat-size: 14px; --t-chat-line: 22px;
  --control-h: 32px; --control-dense-h: 28px; --list-row-h: 32px; --tile-head-h: 32px;
  --toggle-w: 36px; --toggle-h: 20px; --check-size: 16px; --card-pad: 16px;
}
:root[data-density="regular"] {
  --row-h: 30px; --gap: 9px;
  --t-chat-size: 15px; --t-chat-line: 24px;
  --control-h: 36px; --control-dense-h: 32px; --list-row-h: 36px; --tile-head-h: 36px;
  --toggle-w: 40px; --toggle-h: 22px; --check-size: 18px; --card-pad: 20px;
}
:root[data-density="comfy"] {
  --row-h: 36px; --gap: 12px; --fs-md: 14px;
  --t-chat-size: 16px; --t-chat-line: 26px;
  --control-h: 44px; --control-dense-h: 36px; --list-row-h: 40px; --tile-head-h: 40px;
  --toggle-w: 44px; --toggle-h: 26px; --check-size: 20px; --card-pad: 24px;
}
```

| Var | compact | regular | comfy | D-06 row |
|---|---|---|---|---|
| `--t-chat-size` / `--t-chat-line` | 14/22 | 15/24 | 16/26 | Chat prose |
| `--control-h` | 32 | 36 | 44 | Control |
| `--control-dense-h` | 28 | 32 | 36 | Dense-toolbar control |
| `--list-row-h` | 32 | 36 | 40 | List / menu / nav / sidebar row |
| `--tile-head-h` | 32 | 36 | 40 | Stream tile head |
| `--toggle-w` × `--toggle-h` | 36×20 | 40×22 | 44×26 | Toggle |
| `--check-size` | 16 | 18 | 20 | Checkbox |
| `--card-pad` | 16 | 20 | 24 | Card padding |
| legacy `--row-h` / `--gap` / `--fs-md` | 26/6/12.5 | 30/9/13 | 36/12/14 | unchanged (P3) |

If the owner answers **no** to shortlist #2, P1 itself does not change. Halo's sizes are the comfy column, so the flip is one commit: the default density goes `compact` → `comfy` in `app.html:2` and in `ui-prefs.svelte.ts:98,215` (the `density` state default and `resetAppearance`). Stored user choices are kept.

### 2.8 Radius rename plan (D-07, commit C10)

Rift's `--radius-lg` (10px, meaning "medium") is renamed to `--radius-md` (Halo 12px) at every consumer **before** Halo's `--radius-lg: 16px` takes the name in C15. Each site goes 10 → 12. Fallbacks are dropped, per §8 ("no divergent var() fallbacks").

| # | Site | Today | After |
|---|---|---|---|
| 1 | `src/lib/components/workspace/NewsFeed.svelte:243` `.dc-ic` | `var(--radius-lg)` | `var(--radius-md)` |
| 2 | `NewsFeed.svelte:270` | `var(--radius-lg)` | `var(--radius-md)` |
| 3 | `NewsFeed.svelte:316` | `var(--radius-lg)` | `var(--radius-md)` |
| 4 | `NewsFeed.svelte:328` `.retry` | `var(--radius-lg)` | `var(--radius-md)` |
| 5 | `src/lib/components/workspace/WorkspacePage.svelte:1012` `.rift-input` | `var(--radius-lg)` | `var(--radius-md)` |
| 6 | `WorkspacePage.svelte:1024` `.browse-btn` | `var(--radius-lg)` | `var(--radius-md)` |
| 7 | `WorkspacePage.svelte:1050` | `var(--radius-lg)` | `var(--radius-md)` |
| 8 | `WorkspacePage.svelte:1066` `.del-btn` | `var(--radius-lg)` | `var(--radius-md)` |
| 9 | `WorkspacePage.svelte:1069` `.ghost-btn` | `var(--radius-lg)` | `var(--radius-md)` |
| 10 | `WorkspacePage.svelte:1072` `.save-btn` | `var(--radius-lg)` | `var(--radius-md)` |
| 11 | `WorkspacePage.svelte:1166` | `var(--radius-lg)` | `var(--radius-md)` |
| 12 | `src/lib/components/dialogs/UpdateDialog.svelte:286` | `var(--radius-lg, 12px)` | `var(--radius-md)` |
| 13 | `src/lib/components/assistant/AssistantPane.svelte:1259` | `var(--r-card, var(--radius-lg))` | `var(--r-card)` |
| 14 | `src/lib/components/diagnostics/DiagnosticsConsole.svelte:393` | `var(--radius-lg, 10px)` | `var(--radius-md)` |
| 15 | `src/lib/components/assistant/ToolChip.svelte:684` | `var(--radius-lg)` | `var(--radius-md)` |
| int | `src/app.css:217` `--tile-radius` | `var(--radius-lg)` | `var(--radius-md)` (12 = D-21 tile radius) |
| tw | `src/routes/dev/ui/+page.svelte:231` `rounded-lg` | 10 | `rounded-xl` (12) |

`app.css:85` (the bridge `--radius-lg: var(--radius-lg)`) stays as a key. After C15 nothing uses `rounded-lg`. C10 verify: `rg -n "var\(--radius-lg" src` → 0 hits outside `app.css:85`.

### 2.9 z ladder + every raw z literal (D-08, commit C17)

**Ladder** (strictly increasing; asserted by vitest):

| Token | Value | Source | Use |
|---|---|---|---|
| `--z-raised` | 1 | Halo | lifted inside a card |
| `--z-sticky` | 10 | Halo | sticky bars |
| `--z-hud` | 20 | Rift | pinned HUDs, the peeking sidebar rail |
| `--z-overlay` (= `--z-dialog`) | 50 | Halo | dialog, drawer, palette, lightbox, scrims |
| `--z-toast` | 60 | Halo | toasts |
| `--z-dropdown` (= `--z-popover`) | 65 | Rift deviation (Halo 20) | menus, popovers, select, notification popover |
| `--z-tooltip` | 70 | Halo | tooltips, slider bubble |
| `--z-splash` | 100 | Rift | boot splash |

**The 25 distinct values sit on 78 declarations.** The register's "25 raw z literals" means 25 distinct values (map correction M-1). The 24 declarations with value ≥10 are global layers, and **all 24 change in C17, the same commit as the ladder values**. A surviving 200/2000/9998 would otherwise outrank toasts, tooltips and the splash.

| # | Site | Selector | Today | → |
|---|---|---|---|---|
| 1 | `src/app.css:372` | `.slider-bubble` | 9999 | `var(--z-tooltip)` |
| 2 | `src/app.css:616` | `.dialog-overlay` (UpdateDialog) | 200 | `var(--z-overlay)` |
| 3 | `src/lib/components/ToastHost.svelte:98` | toast stack | 2000 | `var(--z-toast)`; rewrite comment `:95-97` |
| 4 | `src/lib/components/SplashOverlay.svelte:130` | `.splash` | 9999 | `var(--z-splash)` |
| 5 | `diagnostics/DiagnosticsConsole.svelte:334` | `.dc-backdrop` | 70 | `var(--z-overlay)` |
| 6 | `workspace/WorkspacePage.svelte:1030` | `.recent-pop` | 20 | `var(--z-dropdown)` |
| 7 | `webview/WebBrowserPage.svelte:845` | `.wb-menu-scrim` | 999 | `var(--z-dropdown)` |
| 8 | `webview/WebBrowserPage.svelte:849` | `.wb-menu` | 1000 | `var(--z-dropdown)` (portalled after the scrim at `:599-600`, so DOM order keeps it on top) |
| 9 | `dialogs/McpServersDialog.svelte:273` | `.mcp-scrim` | 200 | `var(--z-overlay)` |
| 10 | `dialogs/CommandPalette.svelte:381` | `.cp-scrim` | 200 | `var(--z-overlay)` |
| 11 | `settings/SettingsPage.svelte:1624` | `.sset-results` | 30 | `var(--z-dropdown)` |
| 12 | `assistant/FilePathMenu.svelte:92` | `.menu` | 1000 | `var(--z-dropdown)` |
| 13 | `assistant/GhPopover.svelte:236` | `.gh-pop` | 950 | `var(--z-dropdown)` |
| 14 | `assistant/MessageBubble.svelte:1382` | `.lightbox` | 3000 | `var(--z-overlay)`; rewrite comment `:1376-1377` |
| 15 | `shell/Sidebar.svelte:218` | `.side-rail` | 30 | `var(--z-hud)` |
| 16 | `shell/ProjectSwitcher.svelte:264` | `.sw-menu` | 52 | `var(--z-dropdown)` |
| 17 | `shell/NotificationCenter.svelte:270` | bell popover | 2100 | `var(--z-dropdown)` |
| 18 | `shell/ConversationList.svelte:509` | `.conv-menu` | 51 | `var(--z-dropdown)` |
| 19 | `assistant/composer/EnhanceBar.svelte:212` | enhance panel | 10 | `var(--z-dropdown)` |
| 20 | `assistant/composer/UsagePanel.svelte:279` | usage panel | 10 | `var(--z-dropdown)` |
| 21 | `assistant/composer/SlashMenu.svelte:140` | slash menu | 10 | `var(--z-dropdown)` |
| 22 | `assistant/composer/MentionPopover.svelte:53` | mention list | 10 | `var(--z-dropdown)` |
| 23 | `assistant/composer/SettingsMenu.svelte:515` | settings panel | 9998 | `var(--z-dropdown)` |
| 24 | `assistant/stream/OutputBlock.svelte:264` | `.output-scrim` | 1800 | `var(--z-overlay)` |
| — | `ui/recipes.ts:14,58,60` | `z-(--z-popover)` / `z-(--z-dialog)` | 9000 / 4000 | unchanged text; the values come from the aliases |

(Rows 5-24 are under `src/lib/components/`.)

**Local stacking (54 declarations, values −1…8): kept as literals.** Each one orders siblings inside its component's own stacking context and is not a layer (A-3). Listed so the count is complete:
- `styles/onboarding.css:58(0),70(1),379(3),388(2)`
- `AppShell.svelte:368(1),376(-1),426(-1),439(-1),454(0),509(1)`
- `DiagnosticsConsole.svelte:435(1),489(4)`
- `AssistantPane.svelte:911(5),927(4),1125(1),1158(1),1170(4),1183(1),1199(1),1217(2),1389(3),1410(4),1439(1),1524(5)`
- `AssistantWelcome.svelte:571(-1),577(-1)`
- `Composer.svelte:1860(1),1873(8),1964(1),1971(1),2004(0),2034(1),2309(2),2342(0),2356(3),2401(2)`
- `Markdown.svelte:1196(2),1224(1)`
- `MessageBubble.svelte:694(1),852(1),1040(1)`
- `WorkspaceShell.svelte:144(1)`
- `UpdateBanner.svelte:239(0),242(1)`
- `StatusBar.svelte:228(1)`
- `Sidebar.svelte:246(6),279(-1)`
- `ConversationList.svelte:459(2)`
- `SlashMenu.svelte:216(1)`
- `QueueRail.svelte:157(0)`
- `SettingsMenu.svelte:870(0),882(0),896(1),920(2)`

The new `check:ui` metric `z:raw-layer` (≥10) ratchets the global ones: 24 → 0.

**Layer changes the owner may notice:**
- Toasts now sit above alert dialogs and the image lightbox, which is intended (DESIGN §10 reversal, register "Superseded").
- While the browser overflow menu is open, its click-catcher scrim (65) covers toasts (60). One click closes the menu first. D-20 replaces the scrim in P4f.
- Composer popovers (65) now draw above the peeking rail (20); before, the rail (30) drew above them (10).

### 2.10 `@theme inline` bridge — keys unchanged, effective values

No key is added, removed or renamed in P1 (`BRIDGE_KEYS` snapshot, §5.2). Each right-hand side still names a Rift token, and only the Rift token behind it changes:

| Bridge key (`app.css:29-114`) | RHS | Effective value after P1 |
|---|---|---|
| `--color-bg / -bg-inset / -bg-elev-1..3 / -surface* / -field / -track` | Rift names | Halo `bg-000/100/200/300` per §2.3 |
| `--color-field-border` | `--field-border` | `line-300` |
| `--color-code-bg / -code-border` | kept formulas | ink 8% / 13% |
| `--color-border / -border-strong / -border-focus` | | `line-100` / `line-200` / accent |
| `--color-fg / -fg-2 / -fg-muted / -fg-subtle / -fg-faint` | | ink / ink / ink-muted / ink-faint / ink-faint |
| `--color-accent / -accent-hover / -accent-active` | | #6aa8ff ×3 |
| `--color-accent-fg / -accent-soft / -ghost-border / -ring` | | #08090c / rgba(.14) / line-200 / #6aa8ff |
| `--color-ok / -ok-soft` | | accent / accent-soft |
| `--color-warn* / -danger* / -info*` | | Halo; info from the deviation block |
| `--color-danger-fg` | | #08090c |
| `--color-status-busy / -pending / -warn` | | accent / ink-muted / warn |
| `--color-island-* / -tile-*` | kept formulas | re-resolved on Halo |
| `--color-scrim` | | rgba(4,5,7,.72) |
| `--radius-xs` | self | 6 (was 4) |
| `--radius-sm` | self | 8 (was 6): `button.svelte:7`, `recipes.ts:53` |
| `--radius-md` | `var(--radius)` | **8 (unchanged)**. The Tailwind key ≠ the CSS var `--radius-md` (12); see A-6 |
| `--radius-lg` | self | 16 (no consumers after C10) |
| `--radius-xl / -2xl / -card / -island / -tile / -tile-inner` | | 12 / 16 / 12 / 14 / 12 / 8 |
| `--shadow-sm / -md` | | `0 0 transparent` |
| `--shadow-lg / -float` | | `shadow-pop` |
| `--text-2xs..2xl` | `--tile-meta-fs`, `--fs-*` | 10 / 11 / 12 / 12.5 (compact) / 15 / 18 / 22, unchanged |
| `--font-sans` | `var(--font-ui)` | Geist Variable |
| `--font-mono` | self | Geist Mono Variable |
| `--ease-page / -soft` | self | unchanged until P2 |
| `--ease-spring` | self | Halo (identical) |
| `--spacing-row / -density` | `--row-h` / `--gap` | unchanged |

Correction (M-6): `z-(--z-popover)` is Tailwind v4's CSS-variable shorthand. It needs no `@theme` key and no `createCn` registration, so D-08's "z names need an @theme entry" applies only if a named `z-overlay` utility is ever wanted.

### 2.11 Accent default + `[data-accent="custom"]` opt-in (D-02, commits C12 + C19)

`ui-prefs.svelte.ts` changes:
- `:8,14` — the keys become `ACCENT_KEY = "rift.ui.accent.v2"` and `VIVIDNESS_KEY = "rift.ui.vividness.v2"`. The v1 keys are **ignored, not deleted** (no `removeItem`).
- `:80` — `DEFAULT_VIVIDNESS = 0.143`. New `const DEFAULT_HUE = 257`.
- `:84-93` — `AccentSwatch` gains `fixed?: true`. `ACCENTS` gets a new first entry `{ id: "blue", label: "Electric blue", hue: 257, fixed: true }`. The other 8 are unchanged.
- `:98-100` — `accentHue = $state(DEFAULT_HUE)`. New field `customAccent = $state(false)`.
- `init()` `:123-139` — read the v2 keys. `customAccent = accent.v2 !== null || vividness.v2 !== null`.
- `setAccentHue(h)` and `setVividness(c)` always set `customAccent = true`, persist and call `applyAccent()`. There is no early-out at 257: the hue slider (`SettingsPage:729`) calls `setAccentHue` on every `oninput`, and dragging through 257 must not reset vividness or leave custom mode. Only `pickAccent` on the fixed swatch takes the exact-hex path.
- `resetAccent()` `:207-210` — `localStorage.removeItem` both v2 keys; `customAccent = false`; `accentHue = DEFAULT_HUE`; `vividness = DEFAULT_VIVIDNESS`; `applyAccent()`.
- New helpers:
  - `isAccent(a: AccentSwatch)` returns `a.fixed ? !this.customAccent : this.customAccent && this.accentHue === a.hue`.
  - `pickAccent(a)` calls `a.fixed ? this.resetAccent() : this.setAccentHue(a.hue)`.
- `applyAccent()` `:291-296`:
  - If custom: `root.dataset.accent = "custom"` plus the two `setProperty` calls.
  - Otherwise: `delete root.dataset.accent; root.style.removeProperty("--accent-h"); root.style.removeProperty("--accent-c")`.

Call sites:
- `SettingsPage.svelte:723-724`: `class:sel` / `aria-pressed` / the check use `uiPrefs.isAccent(a)`, and `onclick` uses `uiPrefs.pickAccent(a)`.
- `SettingsPage.svelte:718`: the dot background becomes `var(--accent)`.
- `SettingsPage.svelte:720`: the copy becomes "Electric blue by default. Pick a swatch or dial in your own hue to use a custom accent."
- `OnboardingFlow.svelte:239,242,243,245`: same swap.
- `dev/ui/+page.svelte:52-67`: the preview sets `root.dataset.accent = "custom"` while it previews and restores the previous `dataset.accent` on cleanup (`:60-67`).
- `dev/ui/+page.svelte:100`: the dot `<span>` gains `data-accent="custom"`.

The 13 hue-literal consumers (C12; all move off `--accent-h`):

| Site | Today | → |
|---|---|---|
| `AppShell.svelte:408`, `Sidebar.svelte:231`, `Sidebar.svelte:267` | `inset 0 1px 0 oklch(0.92 calc(var(--accent-c)*0.25) var(--accent-h) / 0.07)` | `inset 0 1px 0 color-mix(in oklab, color-mix(in oklab, var(--accent) 25%, white) 7%, transparent)` (deleted with the bevel in P4a) |
| `WorkspacePage.svelte:919` | `oklch(0.82 0.15 h)` → `oklch(0.66 0.13 h)` | `color-mix(in oklab, var(--accent) 85%, white)` → `color-mix(in oklab, var(--accent) 85%, black)` |
| `LimitBar.svelte:39`, `UsagePanel.svelte:309` | `oklch(0.62 0.15 h)` → `oklch(0.78 0.16 h)` | `color-mix(in oklab, var(--accent) 80%, black)` → `var(--accent)` |
| `SettingsPage.svelte:718` (dot), `:1841` (hue thumb) | `oklch(0.72/0.82 var(--accent-c) var(--accent-h))` | `var(--accent)` |
| `SettingsPage.svelte:1801` (dot glow) | `oklch(0.72 … / 0.6)` | `color-mix(in srgb, var(--accent) 60%, transparent)` |
| `dev/ui/+page.svelte:54,60,66,67,100` | writes `--accent-h` | kept, plus `data-accent="custom"` as above |

Swatch fills at `SettingsPage:723` and `OnboardingFlow` keep `oklch(0.72 0.16 {a.hue})`. The blue swatch therefore previews at hue 257, close to #6aa8ff, and adds no new literal.

**Pre-JS:** a custom-accent user sees blue for the frames before `uiPrefs.init()` runs in `+layout.svelte:51`. Accepted (§14 Q-6).

### 2.12 `app.html` + native colour (D-34, commits C8, C18, C20)

| Site | Today | After | Commit |
|---|---|---|---|
| `src/app.html:2` | `data-direction="linear" data-mode="dark" data-density="compact"` | `data-theme="blue" data-density="compact"` | C8 |
| `src/app.css:129-130` | `:root, [data-direction="linear"][data-mode="dark"]` | `:root` (marker `/* @halo:aliases */`) | C8 |
| `scripts/cdp/c.sh:549` | `ws: document.documentElement.dataset.mode` | `theme: document.documentElement.dataset.theme` | C8 |
| `src/app.html:9` | `background: oklch(0.142 0.003 250)` | `background: #08090c` (pre-JS: no tokens yet; comment "= Halo bg-000") | C18 |
| `src-tauri/src/browser/mod.rs:216-218` | `Color(9, 10, 11, 255)`; comment "oklch(0.142) → #090a0b" | `Color(8, 9, 12, 255)`; comment "Halo bg-000 #08090c" | C18 |
| `SplashOverlay.svelte:146-148` | slab gradient (black 30-42% over `--bg`, accent tint) | `background: var(--bg-000);` | C20 |
| `SplashOverlay.svelte:159-160` | SVG grain layer | removed in C20 (flagged §8 #4: grain only hid gradient banding) | C20 |
| `Markdown.svelte:1205` | — | nothing. The `#22272e` is in a comment (M-2). The live literals `:1208-1209` move in P4b with the CodeBlock | — |
| `app.css:772-779` warm tint | 4 oklch literals | `var(--a11y-warm-*)` (§2.4) | C22 |

`#first-paint` (`app.html:13-14`) is image-only and needs no colour.

### 2.13 Scrollbars (D-23, commit C9) — replaces `app.css:318-337`, matches `HALO/components/bundle.css:7-11`

```css
/* scrollbars — Halo thin (D-23). Sites that hide them today keep scrollbar-width:none. */
::-webkit-scrollbar { width: 10px; height: 10px; }
::-webkit-scrollbar-button { display: none; width: 0; height: 0; }
::-webkit-scrollbar-track, ::-webkit-scrollbar-corner { background: transparent; }
::-webkit-scrollbar-thumb {
  background: var(--line-200);
  border: 3px solid transparent; background-clip: padding-box; border-radius: 10px;
  transition: background var(--dur-fast) ease-out;
}
::-webkit-scrollbar-thumb:hover { background: var(--ink-faint); background-clip: padding-box; }
::-webkit-scrollbar-thumb:active { background: var(--ink-muted); background-clip: padding-box; }
```

What changes: the thumb inset goes 2 → 3px and the radius 999 → 10. The colours are the same values reached through the new names. The 5 hidden sites are untouched.

### 2.14 Expected visible diff per surface after P1 (what the owner will notice)

**Everywhere:**
- Emerald becomes Electric blue: buttons, toggles, selection, focus, active rows, live dots, context ring.
- Every green "ok" turns blue (101 refs): success pills, diff `+`, CI dots.
- Warn moves from amber to orange (#ff9f5a). Danger moves to #ff6b6b. Info becomes light blue.
- Geist replaces Inter, and Geist Mono replaces JetBrains Mono. Expect small width shifts and new truncations in the 34px rows, dock labels and status bar.
- Secondary text (`--fg-2`, 156 sites) brightens to `ink`, and faint text (247 sites) lifts from #4B5056 to #7d8592.
- Focus rings go from a translucent glow to solid blue.
- Menus, HUDs and popovers lose the big 64px shadow and get Halo's lighter `shadow-pop`.
- Small radii go up by 2px: kbd and chips 4→6, buttons, inputs and menu rows 6→8, the 16 renamed sites and stream tiles 10→12.
- Thumbs are slightly slimmer.

**The darker interim (an honest warning).** The shell has not been flattened yet (P4a, gated #1).
- Cards drop from #17191B to #0e1014.
- The AppShell slab (`AppShell.svelte:352-354`) still subtracts 30-42% black from the new `bg-000` and now carries a 1-2.5% **blue** cast.
- The composer well, code wells and popover wells (`--bg-inset`) become canvas-dark #08090c.
- Overall P1 reads darker and lower-contrast between canvas and card than the Halo target. The flat islands in P4a fix this. It is not a regression to chase in P1.

| Surface | What changes beyond "everywhere" |
|---|---|
| Splash / boot | Panels go flat `bg-000` with no grain. First paint and the browser child pre-paint go from #0c0d0e/#090a0b to #08090c (imperceptible). |
| Sidebar | Bevel highlight tint goes blue. Active-row bar and `.rift-menu-row.current` go blue. Rail z drops 30→20 (no visible change unless a composer popover is open during peek). |
| Home (WorkspacePage, NewsFeed) | Project-card accent gradient (`:919`) goes blue. Inputs and buttons at `:1012-1072` go 10→12 radius. News icon tiles go 12. |
| Chat stream | Tiles go r10→12 and inner r6→8. Live tile border and ask tile go blue. Diff adds go blue. Shell `$` and kind colours are unchanged (D-18 is P4b). The Shiki theme is unchanged. Inline code chip goes ink 8%. |
| Composer | Well becomes canvas-dark. Send button turns blue with dark ink. Popovers lose the heavy shadow and change z only. The `--model-color` composer chrome follows accent, so it turns blue. |
| Settings → Appearance | New first swatch "Electric blue", selected. Hue and vividness dials still present (#5 gated). Touching a dial switches to the custom accent. Dyslexia font option reads "Inter"→"Geist" (`:908`). `.seg button.on` loses its `shadow-sm`. |
| AI Health | Bars, limit bars and chart series go blue. Warn/danger tones shift. Gradients keep their shape. |
| Dialogs / toasts | Alert dialogs (z 50) now sit under toasts (60). The lightbox now sits under toasts. Window-close hover X turns dark on red (`--danger-fg`, 7.17:1). |
| Onboarding | Step-1 swatches gain Electric blue (selected). Radial washes and the dot grid are unchanged (P4d). |
| `/dev/ui` | Accent bar previews via `data-accent="custom"`. `rounded-lg` sample becomes `rounded-xl` (12). |

Not changing: any width, height, padding, gap or row height (except radius); font sizes; motion timing (except dur-fast +10ms, dur-slow +80ms, stagger −2ms); `--dur-base` 240.

## 3. States

| State | Trigger | `<html>` attributes | What resolves |
|---|---|---|---|
| Pre-JS first paint | window open | `data-theme="blue" data-density="compact"` (static) | inline `#08090c`; no tokens yet |
| Default | `uiPrefs.init()`, no v2 accent keys | + nothing | Halo blue: `--accent` #6aa8ff, aliases per §2.3 |
| Custom accent | Hue or vividness dial touched, or a non-blue swatch picked | + `data-accent="custom"`, inline `--accent-h`/`--accent-c` | `@halo:custom-accent`: oklch accent + color-mix derivatives + focus ring |
| Reset accent | "Electric blue" swatch (`pickAccent`) or Reset | `data-accent` removed, inline h/c removed | back to Default; v2 keys removed |
| Legacy v1 user | `rift.ui.accent.v1` present | none | Default (v1 ignored, kept on disk) |
| Density regular / comfy | Settings density radiogroup | `data-density="regular|comfy"` | §2.7 columns; the legacy trio as today |
| Dyslexia font | a11y pref | `data-a11y-font="lexend"` | `app.css:747-756` stays last in the cascade; Lexend beats Geist |
| Reduced motion | OS setting | — | unchanged in P1 (`app.css:120-122` `[data-slot]` kill); P2 adds the global net |
| Theme attribute missing (e.g. a test harness) | — | no `data-theme` | Halo `:root` defaults = **lime** accent. Guarded by the vitest "app.html pins blue" assertion |

## 4. Motion

P1 adds no motion. It only changes token values that existing transitions already read:

| Element | Trigger | Property | from → to | Duration | Easing | Delay / stagger | Reduced motion |
|---|---|---|---|---|---|---|---|
| Every `var(--dur-fast)` consumer (456 ext refs) | hover / press | as authored | unchanged | 140 → **150ms** | as authored | — | as today |
| Every `var(--dur-slow)` consumer (5) | data reveal | as authored | unchanged | 520 → **600ms** | as authored | — | as today |
| `var(--stagger)` consumers (5) | entrance | delay | unchanged | — | — | 62 → **60ms** | as today |
| `var(--dur-base)` consumers (111) | state | as authored | unchanged | **240ms held** (dev block; #3) | as authored | — | as today |
| `--ease-spring` consumers (8) | flip / grab | — | — | — | identical value | — | as today |
| Scrollbar thumb | hover | background | `line-200` → `ink-faint` | `dur-fast` 150 | `ease-out` keyword | — | none needed |
| Unmerged side commit (halo/p1-dur-base-300) | — | — | — | 240 → 300 | — | — | — |

`--ease-page`, `--ease-soft`, `--dur-page`, `--dur-page-out`, `--dur-rise` and `--pulse-live` keep their values. P2 re-points them (A-2).

## 5. Parts

### 5.1 Kit primitives used
None. `ui/` is untouched in P1, apart from effective values through unchanged bridge keys.

### 5.2 `design-system/token-graph.mjs` (new, shared by the gate and the tests)

```js
// Pure functions, no deps. Paths are resolved by the caller.
export function parseBlocks(cssText) → Block[]
//   Block = { selector, marker /* "aliases"|"deviations"|"custom-accent"|"density"|null */,
//             atRule /* "theme"|"theme inline"|"media …"|"property --x"|null */,
//             decls: [{ name, value, line }] }
//   Reads the `/* @halo:<marker> */` comment on the line before a selector BEFORE stripping comments.
//   Tracks brace depth, so blocks nested in @media keep their atRule.
export function haloDefs(tokensCssText, theme = "blue") → Map<name, value>   // :root block + [data-theme=blue] overrides
export function haloNames(tokensCssText) → Set<name>                        // every name in any block
export function riftDefs(blocks) → Map<name, { value, line, marker }>       // all non-@theme blocks in cascade order
export function resolve(name, maps, seen = []) → { value } | { undefinedRef, chain } | { cycle: string[] }
export const DEVIATIONS = {            // Halo-owned names Rift may redeclare, with the decision that allows it
  deviations: { "--info": "D-03", "--info-soft": "D-03", "--chart-2": "D-03/D-24", "--chart-3": "D-03/D-24",
                "--font-sans": "D-05", "--font-mono": "D-05", "--z-dropdown": "D-08", "--dur-base": "D-10 pending #3" },
  "custom-accent": { "--accent": "D-02", "--accent-soft": "D-02", "--glow-strong": "D-02", "--shadow-focus": "D-02" },
};
export const PENDING_HALO_WINS = [ /* C5: the 17 collided names Rift still declares; each P1 commit deletes its entries */ ];
export const BRIDGE_KEYS = [ /* the 79 `@theme inline` keys at app.css:30-114 on 22727d24 (51 color, 10 radius, 4 shadow, 7 text, 2 font, 3 ease, 2 spacing), verbatim */ ];
export const RUNTIME_VARS = { "--code-fs": "ui-prefs.svelte.ts:301", "--code-tab": ":302", "--code-liga": ":303",
                              "--rail-w": "Sidebar.svelte:46 + @property app.css:347", "--idx": "style= attrs" };
```

### 5.3 `check:tokens` rewrite — algorithm + failure messages (commit C5)

Inputs: `src/lib/styles/halo/tokens.json`, `src/lib/styles/halo/tokens.css`, `src/app.css`. The script no longer reads `design-system/styles.css` or `_ds_manifest.json`.

1. **STALE.** Fails if `normalizeEol(read(tokens.css)) !== tokensCss(JSON.parse(read(tokens.json)))`.
   `✗ STALE src/lib/styles/halo/tokens.css is not what tokens.json generates — run: node design-system/halo-tokens.mjs --write (never hand-edit tokens.css)`
2. **Parse.** `blocks = parseBlocks(app.css)`. If no `@halo:aliases` block is found, or it holds 0 declarations:
   `✗ PARSE no /* @halo:aliases */ :root block in src/app.css — the alias layer is missing`
   Load `H = haloNames(tokens.css)` and `HB = haloDefs(tokens.css, "blue")`.
3. **COLLISION.** For each declaration outside `@theme` whose name is in `H`:
   - allowed if `marker ∈ {deviations, custom-accent}` and the name is in `DEVIATIONS[marker]`;
   - else allowed if the name is in `PENDING_HALO_WINS`;
   - else:
     `✗ COLLISION --radius-sm redeclared at src/app.css:201 (:root) — Halo owns this name (D-08). Delete it so Halo wins, or move it into the @halo:deviations block and add it to DEVIATIONS with a decision id.`
   - Deviation marker but name not in the list:
     `✗ UNLISTED DEVIATION --accent-ink at src/app.css:L — add it to DEVIATIONS["deviations"] with a decision id, or delete it`
4. **PENDING hygiene.** Each `PENDING_HALO_WINS` entry that `app.css` no longer declares outside a deviation block:
   `✗ STALE PENDING --warn is no longer redeclared — delete it from PENDING_HALO_WINS (token-graph.mjs)`
   After the last P1 value commit the list must be `[]` (vitest asserts it).
5. **UNDEFINED / CYCLE.** For every Rift declaration (all non-`@theme` blocks), resolve each `var(--x)` against `HB ∪ riftDefs ∪ RUNTIME_VARS`, fallbacks ignored:
   `✗ UNDEFINED src/app.css:L --surface → var(--bg-101): --bg-101 is defined by neither halo/tokens.css nor app.css`
   `✗ CYCLE --a → --b → --a`
6. **BRIDGE.**
   - Every name in `BRIDGE_KEYS` must still be a key in `@theme inline`:
     `✗ BRIDGE KEY REMOVED --color-fg-2 — D-08 keeps every @theme key; retarget its value instead`
   - Every bridge RHS `var(--x)` must resolve as in step 5. New keys print as INFO.
7. **Report.**
   - `--verbose` prints every Rift alias as a resolved chain, e.g. `--surface → var(--bg-100) → #0e1014`.
   - Always print `Halo-owned overrides: N (deviations) + M (custom-accent); pending: K`.
   - Exit 1 on any ✗. Otherwise `PASS — <n> Rift tokens resolve through Halo (blue); tokens.css current`.

### 5.4 `check:ui` changes (commit C6)

- **Files:** `.svelte` plus `.css` under `src/`, excluding `src/lib/styles/halo/**`.
  - For `.css` only, strip `/* … */` before matching.
  - `.svelte` stripping is unchanged, which keeps today's baselines comparable: the `#100`-style issue refs stay counted.
- **New METRICS** (all scope `"all"`):
  - `"font:size-px"`: `/font-size\s*:\s*\d+(?:\.\d+)?px/g`
  - `"motion:backdrop-filter"`: `/(?<![-\w])backdrop-filter\s*:\s*(?!none\b)/g` (counts real declarations, not the `-webkit-` duplicate or `none`)
  - `"motion:cubic-bezier"`: `/cubic-bezier\(/g`, counted after removing custom-property declarations (`/--[\w-]+\s*:[^;{}]*;/g`), so token definitions are not counted
  - `"z:raw-layer"`: `/z-index\s*:\s*[1-9]\d+\b/g` (≥10)
- **Baselines** written by `--update` in C6, measured on 22727d24 with this exact code:

| Metric | .svelte | .css | Total |
|---|---|---|---|
| color:hex | 21 | 3 | 24 |
| color:rgb-hsl | 16 | 1 | 17 |
| color:oklch-literal | 131 | 48 | 179 |
| font:size-px | 348 | 38 | **386** (register says 380; that counted ext refs only, M-4) |
| motion:backdrop-filter | 15 | 1 | 16 |
| motion:cubic-bezier | 43 | 1 | 44 |
| z:raw-layer | 22 | 2 | 24 |
| overlay:use-portal / aria-role / tailwind:arbitrary | unchanged | — | 11 / 17 / 2 |

- **Expected after P1 (C24 `--update`):**
  - `z:raw-layer` 0.
  - `oklch-literal` ≈ −20 (the 9 accent-literal sites, shadows and status in `app.css`).
  - `rgb-hsl` 17 (−1 scrim, +1 info-soft).
  - `hex` 24 (−3 status, +3 deviation).
  - The others are unchanged.
  - The numbers `--update` prints are authoritative; record them in the C24 commit body.
- The `ui/` zero-colour rule (`check-ui.mjs:122-130`) is unchanged.

### 5.5 Vitest (node env; `fs` reads via `new URL(..., import.meta.url)`)

**`src/lib/styles/halo-tokens.test.ts`** (C7):
- "vendored tokens.css is current": `read(tokens.css)` (EOL-normalised) `=== tokensCss(tokens.json)`.
- "tokens.json is the recorded Halo v2": sha256 equals a constant hardcoded in the test (`3a5cf99c…094f`). The test is the source of truth, and `PROVENANCE.md` must quote the same value.
- "blue preset accent": `haloDefs(css,"blue").get("--accent") === "#6aa8ff"`.

**`src/lib/styles/tokens.test.ts`** (C7, extended by the commit named in brackets):
- every Rift token in the alias, deviation, custom and density blocks resolves (no undefined, no cycle);
- every `@theme inline` RHS resolves, and `BRIDGE_KEYS` ⊆ keys;
- [C13] `PENDING_HALO_WINS` has no colour names; `--accent` resolves to `#6aa8ff`, as do `--accent-hover` and `--border-focus`; `--ok` resolves to `--accent`;
- [C13] no `#hex` in `app.css` outside `@halo:deviations` (comments stripped first, using the markers read before stripping);
- [C11] each density block declares exactly `DENSITY_VARS` = `--t-chat-size --t-chat-line --control-h --control-dense-h --list-row-h --tile-head-h --toggle-w --toggle-h --check-size --card-pad` (plus the legacy trio where present). Every `--t-<name>-size` is ≥12px except `label` (11).
- [C17] the z ladder resolves numerically to raised < sticky < hud < overlay < toast < dropdown < tooltip < splash. `--z-dialog == --z-overlay` and `--z-popover == --z-dropdown`.
- [C16] `PENDING_HALO_WINS` is `[]`.
- [C8] `src/app.html` `<html>` has `data-theme="blue"` and no `data-direction` / `data-mode`.

**`src/lib/styles/css-vars.test.ts`** (C7): no undefined `var()` anywhere in `src/` (D-33).
- Comments are stripped first (`/* */`; `//` in `.ts`; `<!-- -->` in `.svelte`).
- Definitions are collected from `--x:` declarations in any `.svelte`/`.css`/`.ts`, `setProperty("--x"`, `style:--x`, `haloNames(tokens.css)` and `RUNTIME_VARS`.
- Every `var(--x` reference must be defined, whether or not it has a fallback.
- Today's failures, which C4 fixes: `--bg-0`, `--bg-raised`, `--border-subtle`, `--text-muted`, `--text-primary`, `--text-secondary`. `--font-sans` is fixed in C2 (verified with a scratch scanner).

### 5.6 Rift parts touched (API deltas only)
- `uiPrefs`: `+customAccent: boolean`, `+isAccent(a)`, `+pickAccent(a)`; `AccentSwatch.fixed?: true` (§2.11).
- `halo-tokens.mjs`: `export function tokensCss(json): string` plus CLI `--write | --check | --upstream <dir>`.

## 6. Ordered work list

### 6.0 P0 — branch, baselines, spikes (no source commits)

`C=scripts/cdp/c.sh`. Run everything from the repo root in Git Bash.

**P0-1 · Preflight.**
- `git status --porcelain` prints nothing.
- `git rev-parse --short HEAD` = `22727d24`, and `git branch --show-current` = `design/chat-ledger`.
- `node -v` → `v24.*`.

**P0-2 · Branch (D-25).**
- `git switch -c design/halo 22727d24`
- P1 then works on `git switch -c halo/p1-foundation design/halo` and merges back with `git switch design/halo && git merge --no-ff halo/p1-foundation` once §10 passes.
- No push, no merge to `main`: both are asked at the time.

**P0-3 · Launch the bridge.**
- `npm run cdp:dev`, as a background task.
- `npm run cdp:serve`, as a background task.
- `bash $C doctor`, then `bash $C ready 60000` → `[ready] ✓ app mounted`.
- Never kill `rift-tauri.exe` by image name (AGENTS.md).

**P0-4 · Record the viewport.**
- `bash $C eval "JSON.stringify([innerWidth,innerHeight,devicePixelRatio])"` → expected ≈ `[1638,883,1.25]` (the 2048×1104 @ 1.25 audit size, `docs/design/redesign-plan.md:26`).
- Write the value into `docs/design/halo/baselines/p0/README.md`. Every later `diff` must run at the same size, or `diff` prints `⚠ SIZE MISMATCH`.

**P0-5 · Baselines (D-25).** Names are fixed, so later phases reuse them.
```
bash $C nav home                        && bash $C baseline "" p0-home
bash $C nav chat                        # open the newest conversation that has ≥1 tool tile; record its title in the README
bash $C baseline "" p0-chat
bash $C baseline ".sidebar" p0-sidebar
bash $C baseline ".composer-wrap" p0-composer
bash $C nav settings && bash $C map '[id^="settings-tab-"]'   # confirm the appearance tab id (SettingsPage.svelte:640)
bash $C act click '#settings-tab-appearance' && bash $C baseline "" p0-settings-appearance
bash $C nav ai-health                   && bash $C baseline "" p0-ai-health
bash $C key u 3                         && bash $C baseline "" p0-dev-ui         # Ctrl+Alt+U (mods: Alt=1|Ctrl=2); if onDevKey misses it: bash $C eval "location.assign('/dev/ui')"
bash $C key g 3 && bash $C click '.chip.primary'                                   # /dev/gallery ▶ Play, speed 1×
bash $C wait "document.querySelector('.chip.primary')?.textContent.includes('Play')" 120000
bash $C baseline "" p0-gallery-end
bash $C key g 3                         # back to the app
```
Then:
- **Measure probes:** `bash $C measure <sel> nokids` for `.btn`, `.tile-head`, `.rift-menu-row`, `.pane-shell textarea`, `.crow`, `.kbd`, `.pill`, `.input` (a probe missing on a surface is recorded as "n/a"). Tee the output to `docs/design/halo/baselines/p0/measure.txt`.
- **Truncation count per surface** (font reflow guard): `bash $C eval "[...document.querySelectorAll('*')].filter(e=>getComputedStyle(e).textOverflow==='ellipsis'&&e.scrollWidth>e.clientWidth).length"` on home, chat, settings and ai-health. Record the 4 numbers.
- **Console:** `bash $C console error 20 1` → record the count (expect 0).
- **Copy:** `cp scripts/cdp/.tmp/base-p0-*.png docs/design/halo/baselines/p0/`, because `.tmp` rotates.

**P0-6 · Spikes (D-28).** Record each result in §1.4.

| # | Procedure | Pass / fail | Settles |
|---|---|---|---|
| 1 | `bash $C key u 3`; `bash $C click '[data-slot="dropdown-menu-trigger"]'`; then `bash $C eval "(()=>{const e=document.querySelector('[data-slot=dropdown-menu-content]');return JSON.stringify({state:e?.dataset.state,open:e?.hasAttribute('data-open'),anim:e&&getComputedStyle(e).animationName})})()"` | **Dead** = `open:false` or `anim:"none"` while `state:"open"` → the D-09 swap to `data-[state=open]:` stands. **Alive** = `anim` is `enter` → drop that P3 fix. | D-09 first P3 commit |
| 2 | After C1: italic is already settled (`wght-italic.css` exists in both packages, `CX/node_modules/@fontsource-variable/geist*/`, 5.3.0). **Ligatures:** `bash $C eval` to inject `<div id="liga" style="position:fixed;top:40px;left:40px;z-index:99999;font:16px 'Geist Mono Variable';font-variant-ligatures:normal;background:#000;color:#fff">=> -> != === <= ::</div>`; `bash $C baseline "#liga" liga`; eval `liga.style.fontVariantLigatures='none'`; `bash $C diff "#liga" liga 0.05`; eval `liga.remove()` | `IDENTICAL` → no ligatures: hide the `--code-liga` toggle and keep the stored value (D-05). Any changed px → keep the toggle. | D-05 |
| 3 | `bash $C eval "JSON.stringify({ii:CSS.supports('interpolate-size','allow-keywords'),ua:navigator.userAgent.match(/Edg\/[\d.]+/)?.[0]})"` | `ii:true` → the `.wline` morph is allowed; `false` → grid-rows accordion (D-12 uses grid-rows regardless) | D-12 |
| 4 | **Manual** (OS input, not CDP): drag the titlebar, double-click it (maximise/restore), hover the max button for Snap Layouts, Win+Arrow snap. Inventory: `bash $C eval "[...document.querySelectorAll('[data-tauri-drag-region]')].map(e=>e.className+' '+getComputedStyle(e).webkitAppRegion)"` (13 markup sites, e.g. `Titlebar.svelte:24-25`, `Topbar.svelte:22`, `Sidebar.svelte:123`, `StatusBar.svelte:123`) | Each gesture works = pass. Record failures. If Claude cannot drive the OS, mark "owner-check" | P4a top bar |
| 5 | After a cold `cdp:dev` start: read `node_modules/.vite/deps/_metadata.json`, and list `optimized` keys matching `svelte` or `lucide`. Then `bash $C eval "JSON.stringify(performance.getEntriesByType('resource').map(r=>r.name).filter(n=>/svelte_internal|chunk-|lucide/.test(n)))"` and `bash $C console error 20 1` | One `svelte` runtime chunk, `@lucide/svelte` prebundled once, 0 errors = pass. Rift imports the root `@lucide/svelte` in 63 files, not `/icons/*` like CX, so pass is expected | icon imports in P3/P4 |
| 6 | `node -e "import('@floating-ui/dom').then(m=>console.log(Object.keys(m).includes('computePosition')))"` | `true` today, but only as a bits-ui transitive (`node_modules/@floating-ui/dom` 1.8.0; bits-ui wants `^1.7.1`). Decision: P3 adds `@floating-ui/dom@^1.7.1` as a **direct** dependency before the tooltip | D-17 |
| 7 | Chat open on a settled conversation: `bash $C nav home && bash $C nav chat`; within 100ms `bash $C eval "document.getAnimations().filter(a=>/blockIn|rowIn|enter/.test(a.animationName)).length"` | `>0` → replay exists. Record the count and the animation names; this sizes D-29 | D-29 |
| 8 | `/dev/gallery`: eval `window.__blink=[];new MutationObserver(ms=>ms.forEach(m=>m.removedNodes.forEach(n=>n.nodeType==1&&n.matches?.('.md,.sturn,[class*=md]')&&__blink.push([performance.now()|0,n.className])))).observe(document.querySelector('.stage'),{subtree:true,childList:true})`; Play; wait as in P0-5; `bash $C eval "JSON.stringify(__blink)"` | Removed `.md` nodes at settle = remount blink confirmed (D-12 fixes it). Empty = no remount; then check `getAnimations()` restarting at settle | D-12 |
| 9 | Settled from source: `tokens.css:71` `--chart-2: var(--info)`, `:68` `--info: #c6f35a` (lime). After C13 confirm `bash $C eval "getComputedStyle(document.documentElement).getPropertyValue('--chart-2')"` → `#b79cff` | override stands | D-03 |

**P0-7 · Stale-doc notes (D-25).** Add one line each at `docs/HANDOFF.md:26-36` and `docs/design/redesign-plan.md:178-216`:
"Superseded by 22727d24 (WORD_MS 30, catch-up 11 words, lerp 0.35) and docs/design/halo/spec/." Both files are gitignored, so this is not a commit.

### 6.1 P1 commits (on `halo/p1-foundation`, in order)

Every commit must pass **V**: `npm run check` (svelte-check 0 errors, `check:tokens`, `check:ui`) and `npm test`, both green. "diff" means `bash $C diff "" <p0-name>` on the named surfaces.

**C1 · `chore(deps): add Geist + Geist Mono (fontsource-variable)`**
- Files: `package.json:41-43`, `package-lock.json`, `src/routes/+layout.svelte:2-4`.
- Change:
  - Run `npm install @fontsource-variable/geist@^5.3.0 @fontsource-variable/geist-mono@^5.3.0` (lands under `dependencies`, next to inter).
  - In `+layout.svelte`, add `import "@fontsource-variable/geist";`, `import "@fontsource-variable/geist/wght-italic.css";`, `import "@fontsource-variable/geist-mono";` and `import "@fontsource-variable/geist-mono/wght-italic.css";` above the inter import. Inter, JetBrains and Lexend stay.
- Preserved: no family uses Geist yet.
- Risk: lockfile churn. Install with Node 24 only.
- Verify:
  - V.
  - `ls node_modules/@fontsource-variable/geist/wght-italic.css`.
  - diff home/chat → IDENTICAL or <0.1% (AA only).
  - Run spike 2 ligatures now.

**C2 · `fix(settings): .sp-prose inherits its font (D-33)`**
- File: `src/lib/components/settings/SettingsPage.svelte:1707`.
- Change: delete `font-family: var(--font-sans, inherit);` (keep `color`).
- Why first: once C3 lands, Halo defines `--font-sans: "Geist"` (the Google name, not loaded), so this rule would fall back to system-ui. And under dyslexia mode it would beat the inherited Lexend.
- Preserved: today it already falls back to `inherit`.
- Risk: none.
- Verify: V. `measure .sp-prose` font matches its parent.

**C3 · `feat(tokens): vendor Halo tokens.css (generated) + generator`**
- Files (new): `src/lib/styles/halo/{tokens.json,tokens.css,PROVENANCE.md}`, `design-system/halo-tokens.mjs`. Edit: `+layout.svelte`, add `import "$lib/styles/halo/tokens.css";` directly before `import "../app.css";`.
- Change:
  - `cp "$HALO/tokens.json" src/lib/styles/halo/tokens.json`.
  - Write the generator (§2.2).
  - `node design-system/halo-tokens.mjs --write`.
  - Run the CX tail-diff (§2.2 freshness item 3), which must be empty.
- Preserved: zero visual change. `:root, [data-direction][data-mode]` in `app.css` (later and/or (0,2,0)) still declares every collided name. No consumer uses a Halo-only name (scanned: no `var(--radius-md|bg-000|ink|line-…|shadow-pop…)` in `src`).
- Risk: a forgotten import-order swap would let Halo's values win. The order is checked in §10.
- Verify:
  - V (the old `check:tokens` is still in place and unaffected).
  - diff on all 8 P0 baselines (§6.0 P0-5) → IDENTICAL.
  - `bash $C eval "getComputedStyle(document.documentElement).getPropertyValue('--bg-000')"` → `#08090c`.

**C4 · `fix(css): define the undefined vars (D-33)`**
- Files:
  - `src/lib/components/shell/WorkspaceShell.svelte:155` `--text-muted` → `--ink-faint`;
  - `:177,187` `--text-primary` → `--ink`;
  - `:180` `--border-subtle` → `--line-100`;
  - `:183` `--text-secondary` → `--ink-muted`;
  - `:184` `--bg-raised` → `--bg-200`;
  - `src/lib/components/assistant/stream/StreamAskUser.svelte:400` `var(--bg-0, transparent)` → `var(--bg-000)`.
- Preserved: the load-error panel's logic.
- Visible: the panel now has colours (it is a rarely seen error state), and the ask "Other" input gets a dark `bg-000` well instead of transparent.
- Risk: low.
- Verify: V; the scratch undefined-var scan returns nothing.

**C5 · `chore(gates): check:tokens resolves Rift aliases through Halo (retire the mirror)`**
- Files:
  - new `design-system/token-graph.mjs`;
  - rewrite `design-system/check-tokens.mjs` (§5.3);
  - `src/app.css:129`, add the line `/* @halo:aliases */` above the root selector so the parser finds the block. C8 rewrites the selector itself.
- Change: `PENDING_HALO_WINS` starts with the 17 collided names Rift still declares: `--accent --accent-soft --info --info-soft --warn --warn-soft --danger --danger-soft --scrim --radius-sm --radius-lg --dur-fast --dur-base --dur-slow --stagger --ease-spring --font-mono`. `package.json:12` is unchanged (same script).
- Preserved: the gate still fails on real drift. It now guards the live alias graph instead of a hand mirror.
- Risk: parser edge cases (`@property`, `@media` nesting, the `@theme` namespaces). Covered by C7 tests.
- Verify:
  - V.
  - Negative tests, each reverted:
    - add `--radius-md: 10px;` to the alias block → COLLISION;
    - point `--surface` at `var(--bg-101)` → UNDEFINED;
    - append a space to `tokens.css` → STALE;
    - delete `--color-fg-2` from the bridge → BRIDGE KEY REMOVED.

**C6 · `chore(gates): check:ui scans .css; font-size / backdrop-filter / cubic-bezier / z-layer metrics`**
- Files: `design-system/check-ui.mjs:94-131`, `design-system/ui-baseline.json`.
- Change: §5.4, then `node design-system/check-ui.mjs --update`. The new baseline must equal the §5.4 table; paste it in the commit body.
- Risk: any later commit that adds a literal now fails, which is intended.
- Verify: V.

**C7 · `test(tokens): Halo freshness, alias resolution, undefined-var scan`**
- Files (new): `src/lib/styles/halo-tokens.test.ts`, `src/lib/styles/tokens.test.ts`, `src/lib/styles/css-vars.test.ts` (§5.5, base assertions only).
- Verify: `npm test` passes. Temporarily break `StreamAskUser:400` back to `--bg-0` → `css-vars.test.ts` fails; revert.

**C8 · `feat(tokens): pin data-theme="blue"; drop dead data-direction/data-mode`**
- Files: `src/app.html:2`, `src/app.css:129-130` (selector → `:root`), `scripts/cdp/c.sh:549`, `tokens.test.ts` (+ the app.html assertion).
- Preserved: zero visual change. Halo's `[data-theme="blue"]` now applies, but `app.css`'s `:root` still declares every collided name later in the source.
- Risk: `c.sh ready` output key renamed (`ws` → `theme`); grep shows no other consumer of `dataset.mode` on `<html>`.
- Verify: V; diff all 6 surfaces → IDENTICAL; `bash $C ready` ✓.

**C9 · `style(css): Halo thin scrollbars (D-23)`**
- File: `src/app.css:318-337` → §2.13.
- Visible: thumb inset 2→3px, radius 999→10.
- Verify: V; `bash $C shot-sel ".stream" jpeg 70 hover` is unchanged (hidden site); one Settings scroll shot.

**C10 · `refactor(css): Rift --radius-lg (10) → --radius-md (D-07 rename)`**
- Files: the 15 sites + `app.css:217` + `dev/ui:231` (§2.8).
- Visible: those sites go 10→12.
- Preserved: nothing else.
- Risk: missing a site → it jumps to 16 in C15. The verify grep must be 0.
- Verify:
  - V.
  - `rg -n "var\(--radius-lg" src --glob '!app.css'` → 0.
  - `bash $C measure ".tile" nokids` → `r 12px`.

**C11 · `feat(tokens): D-06 type ladder + density tiers (defined, unconsumed); --fs-* alias the ladder`**
- File: `src/app.css` — add the `--t-*`, density defaults, `--chat-col` and the `--fs-*` aliases to the alias block (§2.4), and replace `:287-291` with the §2.7 blocks. Plus `tokens.test.ts` [C11] assertions.
- Preserved: `--fs-*` resolve to identical px; legacy `--row-h`/`--gap`/`--fs-md` are unchanged.
- Verify: V; diff all surfaces → IDENTICAL; `bash $C eval "getComputedStyle(document.documentElement).getPropertyValue('--list-row-h')"` → `32px`, then set density to comfy in Settings → `40px`.

**C12 · `refactor(css): accent hue literals → color-mix of --accent (D-02)`**
- Files: the 9 CSS sites in §2.11 (`AppShell:408`, `Sidebar:231,267`, `WorkspacePage:919`, `LimitBar:39`, `UsagePanel:309`, `SettingsPage:718,1801,1841`).
- Preserved: the same lightness steps, approximated by mixing with white/black.
- Visible: still emerald, because `--accent` is still Rift-derived; small tone shifts on those gradients.
- Verify: V; `check:ui` prints `oklch-literal improved` (do not lock it yet); diff home → < 1% in the card gradients.

**C13 · `feat(tokens): Rift colour names → Halo (alias layer: surfaces, lines, text ladder, accent, status) + D-03 deviations`** (the recolour)
- Files: `src/app.css:131-197,276` → the colour rows of §2.4. Add `@halo:deviations` with `--info`, `--info-soft`, `--chart-2`, `--chart-3` (the other deviation entries arrive with C14 and C16). Remove the 9 colour names from `PENDING_HALO_WINS`. `tokens.test.ts` [C13].
- Preserved: every Rift colour name still exists, and consumers are untouched.
- Visible: §2.14 colour rows.
- Risk:
  - This is the darker interim (§2.14).
  - `--ring` becomes a solid colour (A-1).
  - `--ghost-border` goes neutral.
  - `.btn.primary:hover` (`app.css:411`) keeps `--accent-hover` = accent, so it has no hover lift until P3. Accepted.
- Verify:
  - V.
  - `bash $C tour home chat settings ai-health`, then diff each surface (expect large % changed; review every region, none should be layout).
  - `bash $C measure ".sidebar" nokids` → bg resolves via `--island-fill` over #08090c.
  - `bash $C eval` for `--accent` → `#6aa8ff` and `--ok` → `#6aa8ff`.
  - Truncation counts are unchanged from P0 (no font change yet).
  - 0 console errors.

**C14 · `feat(type): Geist + Geist Mono as the UI and mono faces (D-05)`**
- Files:
  - `src/app.css`: alias `--font-ui: var(--font-sans)`; deviation `--font-sans`, `--font-mono`; delete `:306` (`font-feature-settings` cv/ss) and `:314` (`letter-spacing:-0.005em`). Keep `.mono` `:316` and the Lexend rule `:747-756`.
  - `AppShell.svelte:358`: delete `letter-spacing`.
  - `+layout.svelte`: remove the inter and jetbrains-mono imports (the packages stay until P5).
  - `SettingsPage.svelte:908-909`: the accessibility font radio label "Inter" → "Geist" (the stored value `inter` is unchanged).
  - Remove `--font-mono` from PENDING.
- Not touched: 4 more component-local `-0.005em` sites (`AiHealthPage:1507`, `CommandPalette:497`, `McpServersDialog:468`, `SettingsMenu:619`) are heading choices left to their surface ports (M-3).
- Risk: reflow and truncation (R-3).
- Verify:
  - V.
  - `bash $C eval "document.fonts.check('13px \"Geist Variable\"')"` → true.
  - `measure` probes show `Geist Variable` / `Geist Mono Variable`.
  - Truncation counts per surface vs P0: list every increase in the commit body. Any truncated nav or dock label is a P4a item, not a P1 fix.
  - Dyslexia mode on → `.app` font is Lexend.
  - Markdown `*em*` renders true italic (`measure` the font style).

**C15 · `feat(tokens): Halo radius ladder + shadows (D-07, D-08)`**
- Files: `src/app.css:200-206,234-237` → §2.4 shape rows; remove `--radius-sm` and `--radius-lg` from PENDING.
- Visible: radius-xs 4→6, radius-sm 6→8 (buttons, `ui/` buttons, field trigger, `.rift-menu-row`, tile inner); shadows lighter; `shadow-sm` gone.
- Risk: `0 0 transparent` keeps comma lists valid (`Markdown.svelte:776`, `SettingsPage.svelte:1841,1850`). `none` would void them (A-4).
- Verify:
  - V.
  - `bash $C measure ".btn" nokids` → `r 8px`.
  - `bash $C shot-sel ".seg" jpeg 70` in Settings → the selected segment is still distinguishable by fill.
  - `measure` on the hue thumb shows the ring box-shadow still present.

**C16 · `feat(tokens): Halo motion values win (fast 150, slow 600, stagger 60, spring); --dur-base held at 240 (#3)`**
- Files: `src/app.css:242,246-249`. Deviation `--dur-base: 240ms` (with the #3 comment). PENDING → `[]`. `tokens.test.ts` [C16].
- Risk: negligible (+10ms hover).
- Verify: V; `bash $C eval "getComputedStyle(document.documentElement).getPropertyValue('--dur-base')"` → `240ms`.
- **Side commit (unmerged):** `git switch -c halo/p1-dur-base-300` → delete the `--dur-base` deviation line and its `DEVIATIONS` entry → commit `tokens(motion): --dur-base 300 (Halo) — merge only on shortlist #3 = 300` → `git switch halo/p1-foundation`.

**C17 · `feat(tokens): z ladder; tokenise the 24 global z layers (D-08)`**
- Files:
  - `src/app.css`: alias block `--z-hud`, `--z-splash`, `--z-dialog`, `--z-popover`; deviation `--z-dropdown: 65`; `:372`, `:616`;
  - the 22 component sites in §2.9;
  - the comments `ToastHost:95-97`, `MessageBubble:1376-1377`;
  - `tokens.test.ts` [C17].
- **One commit**, for the reason given in §2.9.
- Preserved: the relative order of menus over dialogs, and the splash over everything.
- Visible: toasts above alert dialogs and the lightbox.
- Verify:
  - V; `check:ui` `z:raw-layer 24→0`.
  - CDP:
    - open the palette (`bash $C key k 2`), then fire a toast via a copy action → the toast is visible above the scrim;
    - in `/dev/ui`, open the alert-dialog, then open a select inside it → the menu is above the dialog;
    - the splash still covers everything on a cold start (`sessionStorage.removeItem("rift.splash.seen")` + reload).

**C18 · `fix(boot): first paint + browser child background = Halo bg-000 (D-34)`**
- Files: `src/app.html:9`, `src-tauri/src/browser/mod.rs:216-218`.
- Risk: a Rust change, so it needs `npm run verify` on Windows (clippy + cargo tests). No `cargo check` while `tauri dev` runs: stop the dev instance by PID first (`tools/dev-reap.ps1` / `c.sh reap`).
- Verify: `npm run verify` green; open the browser dock and load a page → no light flash; first paint has no colour step behind the splash.

**C19 · `feat(accent): Electric blue fixed by default; dials opt into [data-accent=custom]; prefs keys .v2 (D-02)`**
- Files:
  - `src/app.css` (`@halo:custom-accent` block + its `DEVIATIONS` entries);
  - `src/lib/state/ui-prefs.svelte.ts` (§2.11);
  - `SettingsPage.svelte:720,723-724`;
  - `OnboardingFlow.svelte:239-245`;
  - `dev/ui/+page.svelte:52-67,100`.
- Preserved: hue and vividness dials, swatches, Reset, persistence (on the new keys).
- Risk:
  - v1 users lose their custom hue silently. This is by design (D-02: v1 is ignored), and is listed in §10 for the owner.
  - One-frame blue before custom (Q-6).
- Verify:
  - V.
  - In Settings → Appearance, Electric blue is selected.
  - Click "Violet" → `<html data-accent="custom">`, accent ≈ oklch(.727 .143 275).
  - Click "Electric blue" → the attribute is gone and `localStorage` has no `rift.ui.accent.v2`.
  - Vividness slider → custom.
  - Reload preserves the state.
  - `bash $C eval "localStorage.getItem('rift.ui.accent.v1')"` is unchanged.
  - Focus ring under custom follows the hue (`measure` a focused button's shadow).

**C20 · `style(splash): panels on Halo bg-000, grain retired (D-26, D-34)`**
- File: `SplashOverlay.svelte:146-148,159-160`. Flagged deletion §8 #4.
- Verify: cold boot (clear `rift.splash.seen`, `bash $C reload`) → a screenshot mid-splash shows flat panels; the doors still part.

**C21 · `fix(a11y): reading-comfort line-height + warm tint reach stream prose (D-30)`**
- File: `src/app.css:758-779`. Add `.sturn .md` and `.sturn .md p, .sturn .md li` beside every `.markdown-body` selector.
- Verify: Settings → line spacing on → `bash $C measure ".sturn .md p" nokids` line-height = 1.85 × size.

**C22 · `refactor(css): warm-tint literals → --a11y-warm-* tokens (D-34)`**
- File: `src/app.css:772-779` + the alias block (§2.4). Zero diff with warm tint on.

**C23 · `docs(design): DESIGN.md → "Rift on Halo"`** — outline in §6.2.

**C24 · `chore(gates): lock check:ui baseline after P1`**
- `node design-system/check-ui.mjs --update`; paste the before → after numbers.

**Merge:** after §10 passes on `halo/p1-foundation`, run `git switch design/halo && git merge --no-ff halo/p1-foundation -m "merge: P1 foundation (Halo tokens)"`.

### 6.2 DESIGN.md rewrite outline (C23; `design-system/DESIGN.md`, 307 lines)

Title: "Rift on Halo — DESIGN.md (taste layer)". It opens with "Halo is the system (tokens.css, vendored); this file holds only Rift's deltas and the owner's rejection log."

1. **Posture.** The §1 question stays verbatim ("Does the chrome get louder than the content?"). Graphite Ink is retired.
2. **Colour.** The Halo roles table (bg-000…300, line-100…300, ink/muted/faint, accent fixed #6aa8ff). The Rift alias table (pointer to `app.css` @halo:aliases). The deviation table with decision ids. Status ≠ activity ≠ accent. No green. Solid accent = Send + one CTA. `ink-faint` scope (D-30).
3. **Type.** Geist / Geist Mono / Lexend roles (D-05). The `--t-*` ladder. Mono allowed for SectionLabel, Kbd, metric, data, code; banned for timestamps and receipt meta.
4. **Space, shape, density.** D-06 tiers table. Radius ladder 6/8/12/16/pill, no pill buttons, concentric 16→12→8.
5. **Depth.** Flat. `shadow-pop` only on floating layers. `shadow-focus`. Slab retirement pending #1.
6. **Motion.** 150/300/600, stagger 60, ease-out, spring only for flip/grab. `--dur-base` 240 pending #3. P2 owns the rest.
7. **Component idioms.** Keep the binding parts of today's §7 (D-21 one signal per state, chevron-down 180°).
8. **Rejection log.** Today's §8 **verbatim** (`DESIGN.md:129-251`). Each entry gets a tag `[binding]` or `[retired → D-xx]` per "Superseded rules" in 00-decisions.
9. **Agent guide.** Keep. Add: "tokens.css is generated — `node design-system/halo-tokens.mjs --write`", and "a Halo-owned name may only be overridden in @halo:deviations".
10. **Primitives + layers.** The z ladder table (§2.9). `ui/` first. Never `shadcn-svelte init`. `check:ui` metrics list.

Verify: none (docs). Confirm the §8 body is byte-identical apart from the tags: `git diff --word-diff design-system/DESIGN.md` shows only tag insertions in that range.

## 7. Contracts + couplings touched (D-33)

| Contract / coupling | Where | P1 handling |
|---|---|---|
| `<html>` attributes read by tooling | `scripts/cdp/c.sh:549` reads `dataset.mode` | renamed to `dataset.theme` in C8 (same commit as `app.html:2`) |
| Import order fontsource → tokens.css → app.css | `+layout.svelte:2-5` | the contract (§2.1); C3/C14; a §10 check |
| Tailwind bridge keys | `app.css:29-114`, `utils.ts:13-17`, `utils.test.ts:5-26` | keys frozen (`BRIDGE_KEYS`); no `createCn` change in P1 |
| `dem-add` / `dem-del` | `EditDiff.svelte`, `diffEmphasis.test.ts:59-103` | untouched (they now render blue/danger through `--ok`/`--danger`) |
| `[data-a11y-*]` | `app.css:747-780`, `accessibility.svelte.ts:85-95` | selectors widened in C21; Lexend rule stays last |
| `--rail-w` `@property` | `app.css:347-354`, `Sidebar.svelte:46`, `ToastHost.svelte:93` | untouched (D-19 retimes it in P4a) |
| `--code-fs/-tab/-liga` | `ui-prefs.svelte.ts:301-303` | untouched; ligature toggle fate from spike 2 (applied in P4e) |
| Accent storage keys | `rift.ui.accent.v1`, `rift.ui.vividness.v1` | v2 keys; v1 ignored, kept |
| Native colour | `browser/mod.rs:218` ↔ `app.html:9` ↔ `--bg-000` | all #08090c (C18) |
| JS geometry couplings (`thumbW=18`, `ROW_H=26`, mic `rx`, `composerH`, `measureInk`) | — | **not touched**: P1 changes no size. Each moves in the phase that resizes it |
| Contract class names (`.sturn`, `.bubble[data-role=user]`, `.sidebar`, `#card-*`, `settings-tab-*`, …) | — | none renamed |
| `ui/recipes.ts` `z-(--z-popover|--z-dialog)` | `:14,58,60` | names kept; values via aliases |

## 8. Deletions (flagged; gated by shortlist #6 unless noted)

| # | Item | Status in P1 | Gate |
|---|---|---|---|
| 1 | `design-system/styles.css` (152 ln), `_ds_manifest.json`, 15 static cards | **orphaned**: C5 stops reading them. Left in place, frozen. | #6 |
| 2 | `--accent-a` token (`app.css:173`) | no consumer after C13; kept so the dial path and dev tools don't break. Delete it with the dial UI if #5 = yes | #5 / #6 |
| 3 | Inter + JetBrains packages (`package.json:41-42`) | imports removed in C14, packages kept | P5 |
| 4 | Splash SVG grain (`SplashOverlay.svelte:159-160`) | **removed in C20**. It only hid gradient banding, and the panel is now flat. Flagged because it is owner-visible material | default proceed (D-01 grain retirement); revert = 1 commit |
| 5 | Rift Style design-system artifact | untouched; mark it superseded only on #6 | #6 |
| 6 | `@fontsource-variable/inter` CSS `font-feature-settings` rule (`app.css:306`) | removed in C14 (D-05 explicit) | none |

## 9. Canvas board — 00 Foundations (consumes P1, does not gate it)

D-25 says P0-P3 need no board. P1 builds on defaults and merges locally without board 00. Board 00 is the owner's view of exactly the values P1 ships, and it exports the answers to shortlist #2 and #5 and to Q21.

- **Board id:** `00` (D-25), in Claude Design project "Rift · Halo" with Halo installed (Electric blue).
- **Frame:** 1638 × 883. This is the audit viewport in CSS px, so specimens read at real size.
- **Contents, top to bottom:**
  1. **Q21 card:** "What felt rough in the first Halo mock?" with a free-text answer and chips (frame, size, colour, composer, motion, other).
  2. **Colour specimen:**
     - bg-000/100/200/300 swatches with hex;
     - line-100/200/300 on bg-100;
     - text ladder ink / ink-muted / ink-faint on bg-000/100/200, each with its contrast ratio (5.35 / 5.12 / 4.83 for faint);
     - accent #6aa8ff, a Send button (32×32 r8, ink #08090c);
     - status row: accent "Done", info #9cc6ff, warn #ff9f5a "costs money", danger #ff6b6b, each with icon + word.
  3. **Before/after strip:** Graphite (today's values, §2.3 "Today" column) beside Rift-on-Halo for the same 5 components (button, tile head, menu row, input, pill).
  4. **Type ladder:** all 10 `--t-*` rows in Geist / Geist Mono, with a 3-line chat paragraph at the current density.
  5. **Density specimen:** a control row (button, input, select, segmented, icon button), a dense toolbar, 4 list rows, a tile head, toggle, checkbox and card, at the current density.
  6. **Radius ladder:** 6/8/12/16/pill on real parts, plus the concentric 16→12→8 nest.
  7. **Accent card:** the Settings accent card as P1 ships it (Electric blue selected, dials present), next to the "dials retired" version.
- **Play-mode interactions:**
  - the density segmented control (compact/regular/comfy) swaps the board root's `data-density`;
  - the accent toggle (fixed/custom; the custom hue slider is live);
  - the Graphite/Halo toggle on strip 3;
  - hovering a swatch shows its hex.
- **`data-props` levers** on the board root: `data-density="compact|regular|comfy"` · `data-accent="fixed|custom"` · `data-hue="0..360"` · `data-compare="halo|graphite"` · `data-faint="ink-faint|ink-muted"` (for the D-30 scope question).
- **Owner decides:** #2 default density, #5 dial retirement, Q21. Optionally whether `--fg-2` → `ink` reads too bright (D-30 dissent).
- **Exports back into this spec:**
  - #2 = no → the one-commit default flip in §2.7;
  - #5 = yes → the P4e/P4d dial-removal commits, deleting `ACCENTS[1..]`, the dials, the `@halo:custom-accent` block, the `DEVIATIONS["custom-accent"]` entries and `--accent-h/-c/-a`;
  - any number the owner changes on the specimen → its token row in §2.6/§2.7, one commit per value.

## 10. Verification + acceptance (P1 merge gate)

Commands on the Windows dev machine:
1. `npm run check` → svelte-check `0 errors`; `check:tokens` `PASS`, with `pending: 0`; `check:ui` `PASS`.
2. `npm test` → all green, including the 3 new files. Test count = 31 files + 3.
3. `npm run verify` (Rust touched in C18) → green.
4. `rg -n "var\(--radius-lg" src --glob '!app.css'` → 0. `rg -n "data-direction|data-mode=\"dark\"" src scripts` → 0.
5. `node design-system/halo-tokens.mjs --check` → current. `--upstream "$HALO"` → `sha match`.

CDP (`C=scripts/cdp/c.sh`; app via `npm run cdp:dev` + `cdp:serve` in the background; same viewport as P0-4):
6. `bash $C ready`, then `bash $C console error 20 1` → **0 errors** on home, chat, settings, ai-health, /dev/ui and /dev/gallery.
7. Run `bash $C diff` against all 8 P0 baselines (full-window ones with `""`; `p0-sidebar` with `".sidebar"`; `p0-composer` with `".composer-wrap"`) and paste the % and region into the merge commit body. Acceptance: every region explained by §2.14. `SIZE MISMATCH` = viewport drift; rerun at the P0 size.
8. Resolved values (`bash $C eval "getComputedStyle(document.documentElement).getPropertyValue('<x>').trim()"`):
   - `--accent` #6aa8ff · `--bg` #08090c · `--surface` #0e1014 · `--border` #1f242c · `--fg-2` #eceff3 · `--fg-faint` #7d8592;
   - `--ok` #6aa8ff · `--info` #9cc6ff · `--chart-2` #b79cff;
   - `--z-dropdown` 65 · `--z-dialog` 50 · `--dur-base` 240ms · `--radius-sm` 8px · `--radius-md` 12px;
   - `--font-sans` starts `'Geist Variable'`.
9. `bash $C measure` probes vs P0 `measure.txt`: **box width/height/padding identical** for `.btn`, `.tile-head`, `.rift-menu-row`, `.pane-shell textarea`, `.crow`, `.kbd`, `.pill`, `.input`. Only colours, fonts and radii differ, and text width may change by up to ±8%.
10. Truncation counts vs P0: every increase is listed in the merge body (P4 inputs). There is no hard limit, because the font reflow is expected.
11. Accent flow as in C19. Density flow as in C11 (`--list-row-h` 32/36/40). Dyslexia → Lexend. Warm tint reaches `.sturn .md`.
12. Layering as in C17 (toast over palette scrim; select inside an alert dialog; splash on top).
13. Screenshots saved to `docs/design/halo/baselines/p1/` with the same names as P0.

**Counts that must go down** (from `check:ui`, locked in C24): `z:raw-layer` 24→0; `color:oklch-literal` 179→≈159. Everything else ≤ baseline. `check:tokens` pending 17→0.

**Not verified in P1:** a real streaming turn (not needed, since no motion changed); reduced-motion emulation (P2); light themes (D-04, out).

## 11. Risks + mitigations

| # | Risk | Mitigation |
|---|---|---|
| R-1 | Import order is swapped, so Halo's `:root` overrides Rift silently | §2.1 contract; C3 diff must be IDENTICAL; the deviations use (0,2,0) so they do not depend on order |
| R-2 | The darker interim reads as a regression to the owner | §2.14 says it plainly; P4a (flat islands) is the fix; nothing merges to `main` without the owner |
| R-3 | Geist reflow truncates labels (34px rows, dock, status bar) | P0 truncation counts + C14 deltas; fixes belong to the surface ports |
| R-4 | `0 0 transparent` vs `none` | lists stay valid; C15 verifies 3 comma-list sites |
| R-5 | A missed z site outranks the new ladder | the `z:raw-layer` metric goes to 0 in the same commit; vitest ladder order |
| R-6 | v1 accent users lose their hue | D-02 accepted; the v1 value is kept on disk, so a future migration can read it |
| R-7 | `check:tokens` parser misreads `@theme` / `@property` / `@media` | C5 negative tests; markers read before comment stripping; the bridge kept as its own namespace |
| R-8 | Rust colour change is unverifiable outside Windows | `npm run verify` on the dev machine (C18); CI covers `main` only |
| R-9 | `--ring` solid accent makes `.btn:focus-visible` (`app.css:409`) and `.input:focus` 3px rings heavy | accepted for P1 (Halo's ring is solid); P3 moves controls to `--shadow-focus` |
| R-10 | Tailwind `rounded-md` (8) ≠ CSS `--radius-md` (12) confuses porters | A-6; P3 resolves it when `recipes.ts` is restyled |
| R-11 | Composer popovers at 65 now cover the peeking rail | the rail peek is hover-transient; reviewed in P4a |
| R-12 | The browser overflow scrim (65) eats one click over toasts (60) | D-20 replaces the scrim in P4f |

## 12. Effort + dependencies

- **P0:** 1 session (M). It needs the app running. Spike 4 may need the owner's hands.
- **P1:** 2 sessions (L). Session 1 = C1-C12 (plumbing and zero-diff commits). Session 2 = C13-C24 (the visible commits, Rust verify, DESIGN.md).
- **Depends on:** nothing gated. Shortlist #2, #3 and #5 only affect follow-up flips (§2.7, the C16 side branch, §9 exports).
- **Unblocks:** P2 (motion tokens build on `--ease-out` and the deviation `--dur-base`); P3 (bridge `--t-*` and density consumers); every P4 port (acceptance uses the §10 recipe).

## 13. Amendments proposed to 00-decisions

- **A-1 (D-08 `--ring`).** D-08 maps `--ring` → `--shadow-focus`. But `--ring` is a **colour**: 25 `box-shadow: … var(--ring)` sites and the Tailwind `ring-ring` utility (`app.css:57`, `utils.test.ts:18`). Proposed: `--ring: var(--accent)` (a colour). Controls move to `box-shadow: var(--shadow-focus)` as they port (P3/P4).
- **A-2 (D-08 motion aliases → P2).** D-08 lists `--ease-page`/`--ease-soft` → `ease-out` and `--dur-rise` → `dur-slow` as aliases, and D-27 makes P1 "no layout change" with motion in P2. These are Rift-only names (no collision), and they change the feel of 137+10 sites. Proposed: they move in P2's first commit, not P1. Collided motion names follow D-08 (Halo wins) except `--dur-base`, which is held at 240 in the deviation block until shortlist #3. That reconciles D-08 "Halo wins `--dur-base`" with #3 "its own commit, unmerged".
- **A-3 (D-08 "all 25 raw z literals").** There are 25 distinct values on 78 declarations. Proposed: tokenise the 24 declarations ≥10 (global layers) and keep the 54 local values (−1…8) as literals, because they order siblings inside one component's stacking context. A `check:ui` metric `z:raw-layer` (≥10) holds the line.
- **A-4 (D-08 `--shadow`/`--shadow-sm` → `none`).** `none` voids comma lists (`Markdown.svelte:776`, `SettingsPage.svelte:1841,1850`). Proposed: `0 0 transparent`.
- **A-5 (`--danger-fg`, register silent).** Alias to `bg-000`: 7.17:1 on #ff6b6b, against 2.41:1 for `ink`. This mirrors `accent-ink`. The visible effect is that the window-close X turns dark on hover.
- **A-6 (D-08 bridge, `rounded-md`).** The key `--radius-md: var(--radius)` stays 8 in P1, because `menuItem` (`recipes.ts:27`), `alert-dialog-media:16` and `CloseConfirm:144` would otherwise jump to 12 and then come back to 8 in P3 (D-07 rows = 8). The Tailwind name and the CSS var name disagree until P3 restyles `recipes.ts`.
- **A-7 (D-07 card / island radius).** `--r-card` stays 12 and `--island-radius` stays 14 in P1. D-07 cards = 16 lands with the card ports, and the island r16 with P4a (gated #1).
- **A-8 (D-02 deviation scope).** Under `[data-accent=custom]`, Halo's literal `--accent-soft`, `--glow-strong` and `--shadow-focus` (`tokens.css:66-67,73`) would stay blue. So they are redeclared as `color-mix` in a `@halo:custom-accent` block, which extends D-08's deviation list (§2.5).
- **A-9 (D-34 splash).** Panels → `bg-000` as written. The grain goes with them (§8 #4). D-26 does not mention the grain.
- **A-10 (D-06 vars).** D-06 names no CSS variables. P1 adds new names (§2.7) and leaves `--row-h`/`--gap`/`--fs-md` at today's values until P3, so P1 stays layout-neutral.

**Map / register corrections found while verifying:**
- **M-1.** "25 raw z literals" = 25 distinct values, 78 declarations (§2.9).
- **M-2.** D-34 `Markdown.svelte:1205` `#22272e` is a comment. The live literals are `:1208-1209` (oklch, moves in P4b).
- **M-3.** D-05 names 2 `-0.005em` sites (`app.css:314`, `AppShell:358`). There are 6; the other 4 are component headings (C14 note).
- **M-4.** The font-size literal baseline is 386 (348 `.svelte` + 38 `.css`). The register's 380 counted refs outside `app.css` only.
- **M-5.** Spikes #2 (italic) and #9 (chart-2 = lime) are already settled from source (§1.4).
- **M-6.** `z-(--var)` needs no `@theme` key and no `createCn` registration (§2.10).
- **M-7.** `@floating-ui/dom` is present only transitively (1.8.0). D-17 needs a direct dependency.
- **M-8.** Rift imports lucide from the package root, not `/icons/*`, so the ConsoleX prebundle trap (map 11 §4.13) is less likely (spike 5 confirms).
- **M-9.** Map 08 §1.1 says 99 `:root` props. That is confirmed; a naive grep shows 101 because of two comment hits at `app.css:149,209`.
- **M-10.** `--radius-lg`: 15 external consumers confirmed, plus the internal `app.css:217` `--tile-radius` (→12 = D-21) and the bridge `:85`.

## 14. Open questions not settled by the register (default in **bold**)

- **Q-1.** `--fs-2xl` 22px has no rung on the D-06 ladder. **Keep 22 as a kept value. Its only consumer is the bridge key `--text-2xl` (`app.css:104`), and no `ui/` or `dev/ui` class uses `text-2xl`. Retire it in P5.**
- **Q-2.** `--fs-md` compact 12.5px vs D-06 "UI 13". **Keep 12.5 in P1. P3 lifts it with `--row-h`.**
- **Q-3.** `--code-bg`/`--code-border` use ink-mix chips, not Halo's CodeBlock `bg-100`. **Keep the formulas. P4b CodeBlock decides.**
- **Q-4.** `--ghost-border` goes neutral (`line-200`) per D-08, so accent-ghost button borders lose their tint. **Follow D-08.** Board 00 strip 3 shows it.
- **Q-5.** Tile and island formulas (`--tile-fill`, `--island-fill`, the tile border mixes). **Keep the formulas re-resolving on Halo inputs. P4a/P4b replace them.**
- **Q-6.** Custom-accent users get one blue frame before `init()`. An inline pre-JS script would need a CSP check. **Accept the flash. This is an opt-in path only.**
- **Q-7.** `--accent-c` default for the custom path: 0.143 exactly reproduces #6aa8ff at hue 257, but today's vividness slider is 0.05-0.26 with a 0.15 default. **Use 0.143. The slider % display shifts by 3 points.**
- **Q-8.** Geist Mono italic import: no consumer needs it (D-18 kills italic comments). **Import it anyway. Fontsource loads files lazily, so it costs nothing until used.**
- **Q-9.** Should the 4 component-local `-0.005em` heading sites be dropped in C14? **No. They belong to their surfaces.**
- **Q-10.** The density var names (`--control-h`, `--list-row-h`, …) are new vocabulary. **Use the §2.7 names. P3 registers matching Tailwind keys only if `ui/` needs them as classes.**
