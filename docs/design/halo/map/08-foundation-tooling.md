---
title: Halo port map 08 - foundation + tooling
status: map only (read-only wave); nothing executed, no source edited
updated: 2026-10-19
branch: design/chat-ledger (clean tree at read time)
scope: tokens, CSS architecture, fonts, theming, gates, tests, verification tooling, design-system/, guardrails
---

# 08 - Foundation + tooling

Paths are relative to `C:/AI Workflow/projects/rift-tauri` unless prefixed. `HALO/` = the Halo brand-book dir under the session scratchpad (`.../scratchpad/artifact-files/705c5425-86e3-4761-a421-4ee89499e5c6/project/`). `CX/` = `C:/AI Workflow/projects/consolex/panel`. Counts are from greps run 2026-10-19 on this branch; "ext" = refs outside `src/app.css`, "int" = refs inside it. Not run: app, builds, tests, gates.

## 0. Headline findings

1. **The ConsoleX alias pattern is the proven port route and fits Rift's shape.** CX loads Halo `tokens.css` first, then one alias block maps every legacy name to a Halo var, pins `data-theme="blue"`, and self-hosts Geist via fontsource (`CX/src/main.ts:1-4`, `CX/src/app.css:1-66`). Rift has 4,806 external `var(--*)` refs (in `.svelte`/`.css`/`.ts`; 63 of 141 `.svelte` files carry scoped `<style>`), so an alias layer means ~zero consumer edits in the token phase (for `var()` sites; the literals inventoried in 1.5 are a separate sweep). CAVEAT: CX never had to resolve name collisions; Rift has 17 (section 1.3) because Halo reuses names Rift already defines with different values.
2. **Tailwind is a thin layer here, not the architecture.** Utilities appear almost only in `components/ui/` (shadcn/bits-ui primitives) + 3 other files; everything else is scoped `<style>` + `var()` + a handful of global classes (`app.css:394-736`, `stream.css`). The `@theme inline` bridge (`app.css:29-115`) is the contract `check-ui` enforces.
3. **Halo's sizing is touch/dashboard-first; Rift is a dense desktop tool.** Halo: 44px controls, 40px menu rows, 15px body, 12px floor (`HALO/components/bundle.css:43,419,842`; `HALO/tokens.json` type). Rift: 26/22/20px buttons, 26-36px rows, 13px body, 261 literal font-size declarations below 12px. A "Halo, compact" scale is unavoidable and is an owner decision (Q1).
4. **Accent dials are structurally incompatible with Halo's fixed-hex presets.** Rift derives the whole ramp from `--accent-h/-c/-a` in OKLCH (`app.css:171-180`) and exposes hue + vividness sliders (`SettingsPage.svelte:729,736`). Halo ships 8 fixed-hex themes (`HALO/01-themes-and-colour.md`). Two options in section 4; owner picks (Q2).
5. **Halo has no success/green token and its blue preset pairs accent with a LIME `info`** (`CX/design-systems/halo/tokens.css:63-69`). CX overrode `--info` to light blue because "the user wants no green anywhere (2026-10-17)" (`CX/src/app.css:62-66`). Rift's `--ok` is green and is used 101x ext. Owner call (Q3).
6. **Halo's "no colour washes / depth by lightness" vs Rift's onyx-slab chassis** (93 gradient declarations in 32 files, SVG grain in 3 files, bevel + AO shadows; `DESIGN.md` section 5, `AppShell.svelte:343-501`). Largest philosophical conflict; section 9.
7. **Both repo gates survive a Halo port if two rules hold:** keep the Tailwind bridge KEY names (only retarget their `var()` values), and either regenerate or retire `design-system/styles.css` + manifest. Details section 5.
8. **Tests are almost entirely decoupled from markup/CSS.** vitest runs `environment: "node"` (`vitest.config.ts:23`), 31 test files, zero DOM/component tests. Only `utils.test.ts` (Tailwind token names via `cn`) and `diffEmphasis.test.ts` (`dem-add`/`dem-del`) touch style vocabulary.
9. **Halo's Google-Fonts `@import` cannot load in the app:** CSP is `font-src 'self' data:` (`src-tauri/tauri.conf.json:28`). Fontsource (bundled) is the only route.
10. **Native-side color coupling exists:** `src-tauri/src/browser/mod.rs:218` hardcodes `Color(9,10,11,255)` for the browser child WebView, and `src/app.html:9` hardcodes `oklch(0.142 0.003 250)` for pre-JS paint. Both must move to Halo `bg-000` `#08090c`.

---

## 1. Token inventory

### 1.1 Source of truth and shape

- Rift tokens: ONE `:root` block, `src/app.css:129-285` (selector is `:root, [data-direction="linear"][data-mode="dark"]`, `app.css:129-130`), 99 custom properties. Dark only (`app.css:284` `color-scheme: dark`; `app.html:2` pins `data-direction="linear" data-mode="dark" data-density="compact"`). No light theme, no `prefers-color-scheme` handling anywhere in `src/` (grep: only `app.css:284`).
- Density overrides: `app.css:289-291` (`--row-h`, `--gap`, `--fs-md`); stream rhythm tokens `stream.css:16-23` (`--stream-gap`, `--stream-gap-turn`, `--stream-gap-between-turns`, `--stream-indent`, density-aware).
- Tailwind bridge: `@theme` wipe `app.css:18-28`; `@theme inline` re-registration `app.css:29-115`.
- Halo tokens: `HALO/tokens.json` (source) -> `CX/design-systems/halo/tokens.css` (generated, 219 lines, hex/rgba). Halo values are flat hex/rgba, no OKLCH, no derivation.

### 1.2 Token table (Rift name -> value -> usage -> Halo equivalent)

Usage = `var(--x)` refs, ext/int. `~` = Tailwind utilities in `ui/` are additional and not counted (e.g. `bg-surface`, `text-fg-2`). "none" = Halo ships no such token; decision needed.

**Surfaces + lines**

| Rift token | Value (`app.css`) | ext/int | Halo equivalent | Note |
|---|---|---|---|---|
| `--bg` :132 | oklch(.142 .003 250) | 44/4 | `bg-000` #08090c | page ground |
| `--bg-inset` :133 | oklch(.178 .004 250) | 47/2 | `bg-000` or `bg-300` | Rift = darker-than-card WELL (composer, code, popovers). Halo "Inset" `bg-300` is LIGHTER (tracks, skeletons). CX mapped wells to `bg-000` (`CX/src/app.css:24-25`). Design call. |
| `--bg-elev-1` :134 | oklch(.215 .004 250) | 56/3 | `bg-100` #0e1014 | cards |
| `--bg-elev-2` :135 | oklch(.262 .005 250) | 65/5 | `bg-200` #14171c | raised/hover |
| `--bg-elev-3` :136 | oklch(.300 .006 255) | 26/1 | `bg-300` #1b1f26 | highest |
| `--surface` :137 / `-hover` :138 / `-active` :139 | = elev-1 / 2 / 3 | 43/8, 85/4, 4/2 | `bg-100` / `bg-200` / `bg-300` | |
| `--code-bg` :143 / `-border` :144 / `-fg` :145 | color-mix fg 8% / 13% / fg | 7/1 each | none | Halo CodeBlock = `bg-100` + `line-100` (`bundle.css:877`); Halo never colours syntax, Rift runs shiki github-dark-dimmed (`state/highlighter.svelte.ts:8,58`) |
| `--field` :151 / `--field-border` :152 | mix fg 5% on surface / = border-strong | 11/1, 12/1 | `bg-100` / `line-300` | Halo input = bg-100 + line-300 (`bundle.css:842`) |
| `--track` :153 | mix bg 55% surface | 3/2 | `bg-300` | |
| `--border` :156 | oklch(.300 .006 255) | 319/17 | `line-100` #1f242c | hairline |
| `--border-strong` :157 | oklch(.365 .007 255) | 86/11 | `line-200` #2c323c (hover/popup) or `line-300` #5f6774 (controls) | Halo splits by role; Rift one token. 86 sites need triage. HANDOFF.md:75 flags Rift `border-strong` at 1.87:1 |
| `--border-focus` :158 | accent derived | 5/2 | `accent` | |

**Text ladder (5 steps in Rift, 3 in Halo)**

| Rift | Value | ext/int | Halo | Note |
|---|---|---|---|---|
| `--fg` :161 | oklch(.925 .004 250) | 363/16 | `ink` #eceff3 | |
| `--fg-2` :162 | oklch(.825 ...) | 156/6 | none | between ink and ink-muted; collapse decision |
| `--fg-muted` :163 | oklch(.700 ...) | 267/9 | `ink-muted` #9aa2af | |
| `--fg-subtle` :164 | oklch(.600 ...) | 159/6 | none | raised to .600 on 2026-10-19 for 4.5:1 (HANDOFF.md:64-67) |
| `--fg-faint` :165 | oklch(.430 ...) | 247/7 | `ink-faint` #7d8592 | Rift faint is far dimmer (L .43) than Halo's (~4.5:1 floor). Halo: faint only for section labels/ticks/timestamps, never on `bg-300` (`01-themes-and-colour.md`). 247 ext refs, many on meta text |

**Accent system (the incompatible part)**

| Rift | Value | ext/int | Halo | Note |
|---|---|---|---|---|
| `--accent-h` :171 | 163 | 13/8 | none | hue dial; consumers read it directly in `oklch()` literals (section 4) |
| `--accent-c` :172 | 0.15 | 6/7 | none | vividness dial |
| `--accent-a` :173 | 0.12 | 0/3 | none | ghost-alpha dial (no UI exposes it; only defaults) |
| `--accent` :174 | oklch(.78 c h) | 617/18 | `accent` #6aa8ff (blue) | NAME COLLISION, hex vs derived. Largest consumer (617 ext) |
| `--accent-hover` :175 / `-active` :176 | oklch(.86 ..) / (.70 ..) | 6/2, 0/1 | none | Halo hover = glow-strong box-shadow or bg-200 shift (`bundle.css:47`) |
| `--accent-fg` :177 | oklch(.25 .05 h) | 14/3 | `accent-ink` #08090c | |
| `--accent-soft` :178 | accent @ --accent-a (.12) | 39/5 | `accent-soft` rgba(.14) | COLLISION, .12 vs .14 |
| `--ghost-border` :179 | accent @ a*2.9 (~.35) | 24/1 | none | "ghost accent" button/pill border |
| `--ring` :180 | accent @ a*3.75 (~.45) | 25/6 | `shadow-focus` | shape differs: Rift = 2-3px translucent ring; Halo = solid 2px gap + solid 2px accent (`tokens.css:42`) |

**Status**

| Rift | Value | ext/int | Halo | Note |
|---|---|---|---|---|
| `--status-busy` :184 `#22c55e` | hex | 8/1 | none (accent = "live state") | Halo LiveDot uses accent (`bundle.css:80`) |
| `--status-pending` :185 `#3b82f6` | hex | 0/1 | `info` | |
| `--status-warn` :186 `#e5a54b` | hex | 0/1 | `warn` | |
| `--ok` :189 / `--ok-soft` :190 | oklch(.73 .18 150) | 101/6, 11/4 | none / `accent-soft` | Halo: "positive = accent on accent-soft". Blue-accent success is a semantic change (Q3) |
| `--warn` :191 / `-soft` :192 | oklch(.78 .16 70) | 117/8, 11/4 | `warn` / `warn-soft` | COLLISION. Halo `warn` #ff9f5a = "negative" (orange), not caution-amber |
| `--danger` :193 / `-fg` :194 / `-soft` :195 | oklch(.71 .17 22) | 165/13, 1/1, 10/5 | `danger` #ff6b6b / none / `danger-soft` | COLLISION (value) |
| `--info` :196 / `-soft` :197 | oklch(.76 .13 230) | 22/6, 2/3 | `info` / `info-soft` | COLLISION. In `data-theme="blue"` Halo info = LIME #c6f35a |

**Shape**

| Rift | Value | ext/int | Halo | Note |
|---|---|---|---|---|
| `--radius-xs` :200 | 4px | 14/3 | none | smallest Halo = `radius-sm` 8px; Rift chips/kbd at 4px |
| `--radius-sm` :201 | 6px | 21/5 | `radius-sm` 8px | COLLISION: same name, +2px. Also drives Tailwind `rounded-sm` via self-ref (`app.css:83`), used in `button.svelte:7`, `recipes.ts:53` |
| `--radius` :202 | 8px | 36/2 | `radius-sm` 8px (by value) | |
| `--radius-lg` :203 | 10px | 15/2 | `radius-md` 12px (by value) | COLLISION: Halo `radius-lg` = 16px |
| `--radius-xl` :204 / `--r-card` :205 | 12px | 15/2, 16/1 | `radius-md` 12 / `radius-lg` 16 | Halo cards = `radius-lg` |
| `--radius-2xl` :206 | 16px | 8/1 | `radius-lg` 16 | |
| pill 999px | literal | many | `radius-pill` | |
| `--island-radius` :210 | 14px | 3/2 | none | |
| `--island-border` :211 / `--island-fill` :212 | = border / fg 2.8% | 11/2, 8/2 | none (card = `bg-100`+`line-100`) | |
| `--tile-*` :217-231 (radius, radius-inner, fill, fill-hover, border, border-hover/-live/-bad/-ask, well, head-pad, head-pad-compact, head-fs, meta-fs, edge) | see file | 14,4,13,... (`--tile-meta-fs` 13/3, `--tile-radius-inner` 13/1, `--tile-border` 14/3, `--tile-head-pad` 5/1) | none | The stream "machinery" vocabulary has no Halo analog; Halo has `Card`, `CodeBlock`, `Banner` only. Needs a Halo-style re-derivation (Q6) |

**Shadows**

| Rift | Value | ext/int | Halo |
|---|---|---|---|
| `--shadow-sm` :234 | 0 1px 2px .40 | 3/1 | none |
| `--shadow` :235 | 0 4px 14px .45 | 1/1 | none |
| `--shadow-lg` :236 | 0 24px 64px .60 | 6/1 | `shadow-pop` 0 12px 32px .45 |
| `--shadow-float` :237 | double | 4/3 | `shadow-pop` |

**Motion**

| Rift | Value | ext/int | Halo | Note |
|---|---|---|---|---|
| `--ease-page` :240 | cubic-bezier(.22,1,.36,1) | 103/3 | `ease-out` (.2,.8,.2,1) | 36 more literal copies in scoped CSS |
| `--ease-soft` :241 | (.4,0,.2,1) | 34/6 | `ease-out` | Halo has no standard curve; layout shifts use ease-out too |
| `--ease-spring` :242 | (.34,1.56,.64,1) | 8/1 | `ease-spring` (IDENTICAL value) | COLLISION but harmless |
| `--dur-fast` :247 | 140ms | 456/14 | `dur-fast` 150ms | COLLISION, +10ms, harmless |
| `--dur-base` :248 | 240ms | 111/8 | `dur-base` 300ms | COLLISION, +60ms on 111 sites: visible slowdown |
| `--dur-slow` :249 | 520ms | 5/0 | `dur-slow` 600ms | COLLISION |
| `--stagger` :246 | 62ms | 5/0 | `stagger` 60ms | COLLISION, harmless |
| `--dur-page` :243 400ms, `--dur-page-out` :244 140ms, `--dur-rise` :245 460ms | | 4/0, 2/1, 10/0 | none (page change ~520ms spec in `03-motion.md`; `halo-page-in` 520ms `bundle.css:24`) | |
| `--pulse-live` :250 | 1.6s | 18/1 | none (LiveDot ping 1.8s, `bundle.css:80`) | |

**Density, type, layout, layers**

| Rift | Value | ext/int | Halo | Note |
|---|---|---|---|---|
| `--row-h` :253 / `--gap` :254 | 28px / 8px (compact override 26/6) | 1/1 each | none | Halo has no density concept; `--row-h` mostly consumed via Tailwind `h-row`/`min-h-row` (`app.css:113`, `recipes.ts:27`) |
| `--font-ui` :257 | "Inter Variable", Inter, Segoe UI... | 6/3 | `font-sans` Geist | |
| `--font-mono` :258 | "JetBrains Mono Variable"... | 108/7 | `font-mono` Geist Mono | COLLISION. 108 ext refs |
| `--titlebar-h` :259 | 34px | 1/0 | none (Halo `AppShell` unknown) | custom titlebar, `decorations:false` (`tauri.conf.json`) |
| `--fs-xs/sm/md/lg/xl/2xl` :260-265 | 11/12/13/15/18/22px | 138/6, 131/5, 28/3, 2/1, 0/1, 0/1 | none as vars; Halo type STYLES: caption 12/16, small 13/18, body 15/22, heading 16/22, subtitle 20/28, title 32, display 48, mono label 11, data 13, metric 30 | Halo ships no `--fs-*`; a compact scale must be authored. `--fs-md` also moves with density (:289-291) |
| `--chat-col-max` :272 | min(1100px, 100%-48px) | 5/0 | none | |
| `--scrim` :276 | color-mix | 0/2 (+`bg-scrim` in `recipes.ts:58`) | `scrim` rgba(4,5,7,.72) | COLLISION. Both opaque, no blur; compatible |
| `--z-dialog` :281 4000 / `--z-popover` :282 9000 | | 0/0 ext (used as `z-(--z-popover)` in ui/, 4 sites) | `z-dropdown` 20 < `z-overlay` 50 < `z-toast` 60 < `z-tooltip` 70 | ORDER CONFLICT: Halo puts menus BELOW dialogs and dialogs BELOW toasts; Rift requires a menu opened inside a dialog to work (popover > dialog > toasts, `DESIGN.md` section 10) |

### 1.3 Name-collision matrix (Halo `tokens.css` would silently redefine these)

17 names exist in both with different values: `--accent`, `--accent-soft`, `--info`, `--info-soft`, `--warn`, `--warn-soft`, `--danger`, `--danger-soft`, `--scrim`, `--radius-sm`, `--radius-lg`, `--dur-fast`, `--dur-base`, `--dur-slow`, `--stagger`, `--ease-spring` (equal), `--font-mono`. Plus `--radius-md` and `--font-sans`: Halo defines them; Rift only has them as Tailwind bridge keys (`app.css:84,106`) or references them undefined.

Hazards if Halo `tokens.css` is imported un-namespaced next to the current block:
- Last/most-specific declaration wins per property. `[data-theme]` blocks carry `(0,1,0)`; `:root` also `(0,1,0)`; source order decides. A naive import re-radii every `--radius-sm` (21 ext) and `--radius-lg` (15 ext) and the Tailwind `rounded-sm`/`rounded-lg` utilities that self-reference them (`app.css:83,85`), and slows every `--dur-base` (111) transition by 60ms, with no failing gate.
- `SettingsPage.svelte:1707` uses `var(--font-sans, inherit)` which is currently UNDEFINED (falls back to inherit); Halo defines `--font-sans`, so that rule would start resolving to Geist. Fine if intended, a latent trap if not.
- Required discipline: load Halo tokens into a namespaced layer (or rename on import), then ONE alias block declares every Rift name on top. Order is the contract.

### 1.4 Undefined and local variables (found by grep, not in `:root`)

| Var | Where | Status |
|---|---|---|
| `--text-primary`, `--text-secondary`, `--text-muted`, `--border-subtle`, `--bg-raised` | `shell/WorkspaceShell.svelte:155-187` | UNDEFINED anywhere (no fallback): the load-error panel renders with unset color/border/background today. Pre-existing bug; fix or delete in the port |
| `--bg-0` | `assistant/stream/StreamAskUser.svelte:400` | undefined, `transparent` fallback |
| `--font-sans` | `settings/SettingsPage.svelte:1707` | undefined, `inherit` fallback |
| `--model-color` (35 refs) | `Composer.svelte:1857` = `var(--accent)` | local, effectively accent (comment :1854-1856: composer chrome is emerald-only) |
| `--dc-ok/-warn/-bad/-info` (5/11/15/2 refs) | `diagnostics/DiagnosticsConsole.svelte:327-330` | local oklch palette; a parallel status system |
| `--ph` (21 refs) | `AssistantPane.svelte:673`, `ConversationList.svelte:297`, `ProjectSwitcher.svelte:165` via `utils/projectHue.ts` | per-project hue (0-359) from a name hash: multi-hue avatars (conflicts with Halo "one signal colour", Q5) |
| `--tone`, `--tone-soft` (15, 4) | `styles/severity-tone.css`, ToastHost, NotificationCenter | maps severity -> ok/warn/danger/info |
| `--sh-b`, `--sh-h`, `--sw`, `--sz`, `--cols`, `--fill`, `--hero-w`, `--sm-*`, `--lvl`, `--kx`, etc. | various scoped | local, no port impact |
| `--rail-w` | registered `@property <length>` `app.css:347-351`, published by `Sidebar.svelte:46` | animatable layout var; keep |

### 1.5 Literal (off-token) inventory (what bypasses tokens today)

From `check-ui.mjs` regexes (`:96-104`), baseline in `design-system/ui-baseline.json`: use:portal 11, aria-role overlays 17, hex 21 (includes false positives like `#100`/`#182` issue refs, e.g. `StreamShell.svelte:81`, `Markdown.svelte:445`), rgb/hsl 16 (UpdateDialog 3, McpServersDialog 3, CommandPalette 3, OutputBlock 2, MessageBubble 2, SettingsPage/Markdown/AssistantPane 1 each), `oklch(<number>` 131 (top: SettingsMenu 14, Markdown 13, WorkspacePage 10, AppShell 9, DiagnosticsConsole 8, Sidebar 7, AssistantPane 7), Tailwind arbitrary `-[...]` 2.

Beyond the ratchet: 380 literal `font-size: Npx` vs 299 `var(--fs-*)` (ext). Histogram (px: count): 10:82, 11:52, 10.5:44, 12:41, 11.5:34, 9.5:30, 13:23, 9:19, 12.5:17. So **261 declarations sit below 12px**, Halo's absolute sans floor (`caption` 12px, `tokens.json`). Literal easings: 36x `cubic-bezier(.22,1,.36,1)` + ~140 keyword eases; literal durations common (`1s` 23, `.14s` 11, `0.9s` 11, `80ms` 9...). 160 distinct `@keyframes` names in `src/` vs Halo's ~33. z-index literals: 25 distinct values (1, 2, 4, 10, 200 `app.css:616`, 950, 999, 1000, 1800, 2000, 2100, 3000, 9998, 9999...).

---

## 2. Styling architecture

### 2.1 Layers, in cascade-relevant order

1. `src/routes/+layout.svelte:2-5` imports fontsource CSS + `../app.css` (first-party only; `stream.css`/`onboarding.css`/`settings-controls.css`/`severity-tone.css` are imported by their owner components so the classes survive Svelte scoping, per file headers: `stream.css:1-7`, `onboarding.css:1-8`, `settings-controls.css:1-4`, `severity-tone.css:1-5`).
2. `app.css:1-2` `@import "tailwindcss"; @import "tw-animate-css"`.
3. `app.css:18-28` `@theme { --color-*: initial; --shadow-*, --inset-shadow-*, --drop-shadow-*, --text-shadow-*, --radius-*, --text-*, --font-*, --ease-*: initial }` wipes Tailwind defaults. NOT wiped: `--spacing`, `--breakpoint-*`, `--animate-*`, so `p-4`, `gap-2`, `max-w-4xl` (used in `dev/ui`) are stock Tailwind.
4. `app.css:29-115` `@theme inline` bridge: 51 color keys, radius ladder (`xs/sm/md/lg/xl/2xl/card/island/tile/tile-inner`), shadows (`sm/md/lg/float`), text (`2xs`..`2xl`), fonts, eases (`page/soft/spring`), `--spacing-row`, `--spacing-density`. `inline` => utilities emit `var(--token)` so runtime theming restyles them (header comment `app.css:4-17`). Same-name self-refs (`--radius-sm: var(--radius-sm)`) are deliberate: unlayered `:root` out-ranks Tailwind's `@layer theme` copy (:12-14). Whether `@theme inline` still emits a copy into `@layer theme`: the comment says so; not verified.
5. `app.css:120-122` global `[data-slot] { animation: none !important }` under reduced motion (the only global motion kill; tw-animate-css enter/exit).
6. `app.css:129-285` tokens; `:289-291` density; `:294-316` reset + `body`; `:320-340` scrollbars; `:340` selection; `:347-354` `@property --rail-w`.
7. Global component classes in `app.css` (counts are heuristic class-attribute greps, lower bounds): `.kbd` 357 (0 markup uses), `.slider-bubble` 371, `.btn` 394 (11 lines/2 files), `.input` 423 (1), `.field-label`/`.help`/`.dim`/`.divider`, `.pill` 449 (0 by this grep; likely used via dynamic classes, unverified), `.rift-toggle` 476 (16 lines/1 file = SettingsPage), `.card` 496 (20/1), `.island`/`.island-float` 505-510 (0 markup uses: tokens used instead, `AppShell`/`Sidebar` re-roll the slab recipe in scoped CSS), `.tile` family 525-606 (19 lines/14 files; `.tile-head` 10 files, `.tile-pill` 9, `.tile-glyph` 9, `.tile-chev` 10), `.hud-float` (3), `.dialog-overlay` 612 (1 file; z-index 200 literal), `.enter`/`@keyframes enter` 634-650 (0 markup uses), `.ctx-chip` 654 (1 file), `.branch-pill` 682, `.rift-menu*` 695-736 (7 files/4 files for row), a11y overrides 747-780.
8. `src/lib/styles/*.css` (4 files, section 2.3).
9. Scoped `<style>` in 63 of 141 `.svelte` files; ~4,806 `var()` refs. Biggest: Composer ~2660 ln, SettingsPage (1800+ CSS lines at :1700+), SettingsMenu ~970.
10. Primitives: `components/ui/` (66 files by glob: alert-dialog 13, button 2, context-menu 17, dropdown-menu 18, select 13, separator 2, + `recipes.ts`). Tailwind classes via `cn()` (`utils.ts:11-19`, `createCn` with extended theme names) and `tv()` (`button.svelte:6-35`).

### 2.2 How primitives are styled

- `recipes.ts` spells ONE floating panel / menu row / label / separator / field trigger / dialog layers (`:13,19,26,36-48,52,57,59`). `menuItem` (`:26`) uses `min-h-row`, `text-base`, `data-highlighted:bg-surface-hover`. `floatingMotion` (`:19`) = tw-animate-css fade + zoom-95 + slide-2 at `duration-(--dur-fast) ease-page`. `dialogPanel` (`:59`) `max-w-sm rounded-xl p-4.5 bg-surface border-border-strong shadow-lg`.
- `button.svelte:6-35` mirrors `.btn`: variants secondary/ghost/soft/primary/danger/warn/info/link; sizes `h-6.5` (26px) default, `5.5`, `5`, `lg h-8`; icon sizes. Halo has 4 variants (`primary/secondary/ghost/danger`) and 44/36px sizes (`bundle.css:43,56`).
- Only 8 non-ui importers of primitives (PermMenu, SettingsMenu, Composer, shared/Select, CloseConfirm, ContextMenuHost, DeleteProjectDialog, dev/ui). Everything else is bespoke: `.rift-menu*`, hand-rolled popovers (11 `use:portal`, 17 `role=menu|listbox|dialog`; ratchet baseline), settings controls (`.st-*`, `.seg`, `.rift-toggle`).

### 2.3 Shared sheets

| File | Lines | Scope | Notes |
|---|---|---|---|
| `styles/stream.css` | 556 | chat turn: `.sturn*`, `.snarr*`, `.wline*`, `.wbatch`, `.splan*`, `.sxplan*`, `.sweb*`, `.sresult`, `.sledger*`, `.scompact*`, `.msg-act` | owns stream rhythm tokens (:16-23), 11 keyframes (`blockIn` :27 3px rise, `dotPop`, `headRise`, `streamLivePulse`, `shim` :35, `workOpen`, `acRoll`...), live-verb shimmer (:328-343), `interpolate-size` width morph (:260-264) |
| `styles/onboarding.css` | 549 | `.ob-*` first run | contains radial-gradient washes (:18-21, :39-42, :368-372) + a dot-grid `.ob-stage::before` (:373-377) - both on the DESIGN section 8 ban list; own `--gi-ghost-*` aliases (:12-14); a global-scope `.spin` (:361) |
| `styles/settings-controls.css` | 37 | `.st-input/.st-btn/.st-pill/.st-secret` | duplicates `.btn`/`.pill`/`.input` vocab in `st-` names (32px controls) |
| `styles/severity-tone.css` | 14 | `--tone` per severity | selectors `.toast[...]`, `.row[...]` |

Three parallel button/pill/input dialects exist besides the shadcn one: `.btn/.pill/.input` (`app.css`), `.st-*` (`settings-controls.css`), `.ob-*` (`onboarding.css`), plus `.sxa-btn` (`stream.css:493`). A Halo port must pick one `Button` and retire the rest or leave them aliased.

### 2.4 Recommended port architecture (derived from the ConsoleX precedent, not yet applied)

Rationale and proof: `CX/src/main.ts:1-4` (order: fontsource, Halo `tokens.css`, `app.css`), `CX/src/app.css:1-66` (alias block "ConsoleX names -> Halo"), `CX/design-systems/build-tokens.ts` + `README` (tokens.css is generated from tokens.json; a test fails if stale).

1. Vendor `tokens.json`/generated `tokens.css` as an input (do NOT hand-edit). Namespace on import or re-declare collided names (section 1.3).
2. Turn `app.css:129-285` into an ALIAS layer: keep every Rift token name, repoint values to Halo vars (`--surface: var(--bg-100)`), add the few net-new tokens Halo lacks (type scale, density, tile/island if kept, `--z-dialog/popover`). Zero consumer edits for the token phase.
3. Keep the `@theme inline` bridge KEYS (`bg-surface`, `text-fg-2`, `rounded-card`, `shadow-float`, `ease-page`, `h-row`) because `ui/*`, `recipes.ts`, `utils.ts:13-17`, `utils.test.ts` and `check-ui` all speak them. Retarget only the right-hand `var()`s (they already point at Rift token names, so step 2 suffices).
4. Components then migrate surface by surface off aliases onto Halo-native names only if the owner wants Halo vocabulary in markup; otherwise aliases are permanent (like CX).
5. Add Halo motion utilities (`halo-rise`, `halo-page`, `halo-draw`) as Svelte-side equivalents. CX has `CX/src/lib/motion.ts` (`tween` 750ms cubic-out, `decode`, reduced-motion aware, rAF text writes). Halo's helper set (`useCountUp`, `useFlip`, `useToasts`, `smoothPath`) is React-only (`HALO/README.md` "Using it" step 5): Svelte actions needed. `AnimatedCount.svelte` (mentioned `stream.css:362-364`) is Rift's existing odometer (not read).

---

## 3. Fonts

| Aspect | Rift today | Halo | Port impact |
|---|---|---|---|
| UI face | Inter Variable (`+layout.svelte:2`, `@fontsource-variable/inter ^5.2.8`, `package.json:41`) | Geist 400-700 | add `@fontsource-variable/geist` (CX uses `^5.3.0`, `CX/package.json:32`) |
| Mono | JetBrains Mono Variable (`+layout.svelte:3`, `package.json:42`) | Geist Mono 400-600 | add `@fontsource-variable/geist-mono` ^5.3.0 |
| Brand/a11y | Lexend Variable (`+layout.svelte:4`, `package.json:43`): greeting `AssistantWelcome.svelte:474`, splash `SplashOverlay.svelte:201`, dyslexia mode `app.css:747-756`, `accessibility.svelte.ts:91` | none (Halo has no dyslexia face) | Keep as accessibility font; brand use is an owner call (Q8). Settings radio label "Inter" `SettingsPage.svelte:908` becomes "Geist" |
| How loaded | npm fontsource, bundled by Vite, served `'self'` | `bundle.css:1` Google Fonts `@import` | Google import is BLOCKED by CSP `font-src 'self' data:`, `style-src 'self' 'unsafe-inline'` (`tauri.conf.json:28`) and offline-hostile. Fontsource only |
| OpenType features | `font-feature-settings: "cv02","cv03","cv04","cv11","ss01"`, ligatures off (`app.css:306-307`), `letter-spacing:-0.005em` (:314) | Halo uses `font-variant-numeric: tabular-nums` on mono (`bundle.css:19`); title tracking -0.03em/-0.02em | cv/ss sets are Inter-specific; Geist's feature set vs these tags: **unknown**, drop or re-derive. Body tracking -0.005em also set in `AppShell.svelte:358` |
| Code ligatures setting | `--code-liga` set from prefs (`ui-prefs.svelte.ts:303`), consumed in 2 places | n/a | whether Geist Mono ships ligatures: **unknown**; the setting may become a no-op |
| Mono usage rule | DESIGN section 3: mono for code/paths/IDs only; `.mono` 75 lines/16 files; `--font-mono` 108 ext refs | Halo: mono for numbers, `label` (uppercase section labels), `data` (`tokens.json`) | CONFLICT: Halo uses mono for UI labels/metrics; redesign-plan C13 flagged Rift's mono-in-chrome as a defect |
| Hard-coded names | 3 literals: `app.css:753`, `AssistantWelcome.svelte:474`, `SplashOverlay.svelte:201` (all Lexend) | | none for Inter/JetBrains (greps clean) |
| Pre-JS paint | `app.html:9` color only, no font | | |
| Variable weight range | Inter/JetBrains Variable full axis | Halo uses 400/500/600/700 | Rift ceiling is 600 (DESIGN section 3) ; Halo 700 only for display? (**unknown**; not grepped) |
| Size ladder | `--fs-*` 11-22; 261 literals under 12px | body 15, small 13, caption 12, label 11 mono | section 1.5, 9 |

---

## 4. Theming + accent mechanics, user-facing settings

### 4.1 Mechanics today

- `data-*` attributes on `<html>`: `data-direction="linear"`, `data-mode="dark"`, `data-density` (`app.html:2`). Nothing reads `data-direction`/`data-mode` except the selector alias at `app.css:129-130` (dead dimension; `data-theme` is never used in `src/`).
- `uiPrefs` singleton (`state/ui-prefs.svelte.ts:97-307`): density (`rift.ui.density.v1`), accentHue (`rift.ui.accent.v1`), vividness (`rift.ui.vividness.v1`, clamp 0.05-0.26, default 0.15), uiScale (0.8-1.5 via Tauri webview `setZoom`, `:285-289`), code prefs (`--code-fs`, `--code-tab`, `--code-liga` set as inline `<html>` style `:298-304`), stream-detail knobs (not theming). `apply()` runs at `onMount` (`+layout.svelte:51`). localStorage keys are `rift.ui.*.v1`.
- Accent application: `applyAccent()` sets `--accent-h` and `--accent-c` on `document.documentElement.style` (`ui-prefs.svelte.ts:291-296`); the whole ramp (`app.css:174-180`) recomputes in CSS.
- Accessibility (`state/accessibility.svelte.ts`): `data-a11y-dyslexia|font|line-height|warm-tint` on `<html>` (`:85-95`); rules in `app.css:747-780` target `.bubble`, `.markdown-body`, textarea. Warm tint uses hard-coded oklch backgrounds (`app.css:772-779`).
- 8 curated swatches `ACCENTS` (emerald 163, teal 195, sky 230, violet 275, magenta 328, rose 12, amber 70, lime 130; `ui-prefs.svelte.ts:82-93`).
- Direct `--accent-h/-c` consumers outside tokens: 13 ext refs - `AppShell.svelte:408`, `Sidebar.svelte:231,267` (bevel highlight), `WorkspacePage.svelte:919`, `LimitBar.svelte:39`, `UsagePanel.svelte:309`, `SettingsPage.svelte:718,1801,1841`, `dev/ui/+page.svelte:54,100`.

### 4.2 User-facing theme settings (SettingsPage > Appearance)

`SettingsPage.svelte:60` tab "Appearance: accent color, density, and code rendering". Controls: 8 swatches (:722), hue slider 0-360 (:729), vividness slider (:736), UI scale slider (:761), density radiogroup (:766-769), code font size/tab/ligatures, stream detail dials (tool detail :833, narration :841, command output), reading-comfort block (dyslexia :903, font Inter/Lexend :908-909, line spacing :914, warm tint :918). Reset buttons per section (:323, :345; `resetAccent` `ui-prefs:207`). There is NO light/dark switch and NO theme-preset picker in the app. The static DS settings card advertises "6 theme presets" and "background texture" (`design-system/pages/settings.html:1,107-113`, `_ds_manifest.json:30`): that card is STALE vs the app (texture picker retired 2026-07-11, `ui-prefs:141`).

### 4.3 Halo model

- 8 themes by `data-theme`: `dark` (lime default), `blue`, `ember`, `violet`, + 4 `light-*` (`HALO/01-themes-and-colour.md`). Fixed hex; accent is one value; light themes use deeper accent + `accent-ink` white. The colliding-hue swaps (`info` in blue = lime; `warn`/`danger` change in ember) are baked in per preset (`tokens.css:63-98`).
- No hue/chroma/alpha dials, no density, no UI zoom concept. `.halo-root` applies ground + ink + font + `color-scheme` (`bundle.css:3-4`).

### 4.4 Options (owner decides; Q2)

| | (a) Keep dials, Halo blue as default | (b) Adopt Halo presets, delete dials |
|---|---|---|
| Accent source | OKLCH h/c defaults approximating #6aa8ff (approx hue ~258, chroma ~0.12, L ~.74-.78; values are an estimate, not computed), ramp stays derived | `data-theme` presets, fixed hex |
| Kills | nothing; `--accent-hover/-active/-soft/ghost/ring` keep working | `ui-prefs` accent/vividness API + 2 Settings sliders + swatch grid; `/dev/ui` accent preview (`dev/ui/+page.svelte:52-55,90-103`, which writes `--accent-h` directly); 13 oklch-from-`--accent-h` literals must become `color-mix` of `--accent` |
| Halo fidelity | accent ramp not bit-identical to Halo (hover/active shades don't exist in Halo anyway) | exact |
| Light theme | possible later (needs a second token set: Rift is dark-only, `app.css:284`, a lot of literal oklch dark assumptions: 131) | Halo ships light for free, but 131 oklch + 93 gradients + hard-coded `rgb(0 0 0)` shadows assume dark; see risk R9 |
| Persisted prefs | `rift.ui.accent.v1`/`vividness.v1` stay valid | must migrate or ignore stored hue (a user's chosen hue silently disappears) |
| CDP/`/dev/ui` | unchanged | `/dev/ui` accent bar broken |

Recommendation for the owner to confirm: (a) first (zero migration, Halo-blue defaults), treat presets as an additive later phase. Either way set `data-theme="blue"` on `<html>` from `app.html` (CX had to defend it with a MutationObserver only because claude.ai's artifact shell rewrites it, `CX/src/main.ts:8-15`; Rift's webview has no such host, unverified).

### 4.5 Hard-coded color that must move with the theme

- `src/app.html:9` pre-JS `oklch(0.142 0.003 250)`; `#first-paint` image only.
- `src-tauri/src/browser/mod.rs:218` `Color(9,10,11,255)` ("matches oklch(0.142) -> #090a0b" comment).
- `AppShell.svelte:352-354,400-411` and `Sidebar.svelte:231,267` slab + bevel mix `oklch(0 0 0)` with `--bg`.
- `SettingsPage.svelte:1944` `#d97757` (Claude brand orange), `dev/gallery/+page.svelte:149-152` `#7aa2ff` (dev only), `Markdown.svelte:1205 #22272e`, `MessageBubble.svelte:1364-1365` `#000` (mask/gradient).
- shiki theme `github-dark-dimmed` (`state/highlighter.svelte.ts:8,58,135`) is outside the token system entirely.
- `WorkspacePage.svelte:919`, `LimitBar.svelte:39`, `UsagePanel.svelte:309`: gradients from `--accent-h`.

---

## 5. Gates

Entry points: `npm run check` = `svelte-kit sync && svelte-check --tsconfig ./tsconfig.json && npm run check:tokens && npm run check:ui` (`package.json:10`). `npm test` = `svelte-kit sync && vitest run --maxWorkers=4` (:16). `npm run verify` = `scripts/verify.ps1` (:17). Repo rule: do not run a production build as a routine correctness check; no `cargo check` while `tauri dev` runs (`AGENTS.md:21-24`).

| Gate | Enforces | Halo-port impact | Update path |
|---|---|---|---|
| `svelte-check` | TS + Svelte compiler diagnostics (incl. a11y and unused-selector warnings). No `--fail-on-warnings` flag (`package.json:10`) so only errors should fail; **not verified by running** | Deleting markup while leaving CSS yields `css-unused-selector` warnings (visible, probably not failing). HANDOFF cites "svelte-check 0/0" as the house standard (`HANDOFF.md:19`) so warnings matter by convention | none required |
| `check:tokens` (`design-system/check-tokens.mjs`) | `:28-29` extracts ALL `--tokens` from the FIRST `:root` block of `src/app.css` and `design-system/styles.css` (`rootBlock` `:45-60`, comments stripped first), resolves one-level `var()` chains (`:80-90`), and FAILS on any shared token whose resolved values differ (`:105-110,151-155`), plus every `_ds_manifest.json tokens[]` entry vs `styles.css` (`:114-128`). app-only/ds-only are advisory | (1) any changed shared value fails until `styles.css` + manifest mirror it. (2) Moving tokens out of `app.css` into an imported `tokens.css` leaves `rootBlock` finding only aliases like `var(--bg-000)` that don't resolve (`resolve` stops on unknown refs, `:86`) => mass MISMATCH, or "parsed 0 tokens" FAIL (`:95-98`). (3) Halo blocks keyed `[data-theme="blue"]` (not `:root`) are invisible to it. (4) 56 manifest tokens are Graphite literals (`_ds_manifest.json:34-90`) | Choose (section 8): regenerate `styles.css`+manifest as Halo-for-Rift and point the script at the new first `:root`; OR retire the script from `package.json:10,12`. Do not keep it while only `app.css` changes |
| `check:ui` (`design-system/check-ui.mjs`) | (1) `__unstable__loadDesignSystem(app.css, {base: src})` then every class string in `components/ui/**/*.{svelte,ts}` must produce CSS (`:54-91`); a leftover/wiped name HARD-FAILS. (2) ratchets (may only go DOWN) over all `.svelte`: use:portal, overlay ARIA roles (non-ui), hex, rgb/hsl, `oklch(<n>`, arbitrary `-[...]` (`:94-105`); (3) ui/ must have 0 raw colors/arbitrary values (`:122-130`). Baseline `ui-baseline.json` | Safe if bridge KEYS keep their names (the check only needs the key to exist in `@theme`). Renaming any key used in `ui/` (e.g. `bg-surface`->`bg-100`) hard-fails (`recipes.ts` + `button.svelte:7-28` use ~40 distinct names). Halo hex/rgba in a `.css` file is invisible to the ratchet (it scans `.svelte` only, `:108`); NEW hex in `.svelte` raises the count and fails. Porting components onto tokens will DROP counts: prints "improved ... lock it in" but passes. A `tailwindcss` major bump can break `__unstable__loadDesignSystem` (explicit throw `:57-59`) | After each phase that lowers counts: `node design-system/check-ui.mjs --update`. If new primitives are added keep them token-only |
| `vitest` | 31 files, `environment: "node"` (`vitest.config.ts:23`), no DOM; ~776 tests per `HANDOFF.md:19-20` | Only `utils.test.ts` + `diffEmphasis.test.ts` touch style names (section 6) | none if bridge names kept |
| `scripts/verify.ps1` | `:42-55` order: `npm run check` ("Svelte and design tokens") -> clippy -> Vitest -> cargo tests; `-FrontendOnly` skips Rust; refuses Rust gates while repo-scoped `tauri dev` runs (`:25-40`) | Use `npm run verify:frontend` during live UI work | none |
| CI `check.yml` | self-hosted Windows; triggers ONLY on push to `main`, paths-ignore `docs/**`,`**.md` (`:6-11`); frontend job = `npm ci`, `npm run check`, `npm test` (`:55-72`) | **Branch `design/chat-ledger` and any `design/halo-*` branch get NO CI until merged to main** (no PR trigger, deliberate `:4-6`). Locally run `npm run verify:frontend` before merge | none |
| `release.yml` | gates dedup against the `check.yml` verdict for the sha (`:165-175`); rest **unknown** (not read) | | |
| `npm run doctor` (`scripts/project-doctor.ps1`) | probes tools; no design checks found by grep | none | none |

Cargo/clippy are unaffected by the CSS work except the one Rust constant in section 4.5.

---

## 6. Tests coupled to markup / CSS classes

Environment: node, no `@testing-library`, no component mounts (`vitest.config.ts:19-36`; docs agree: "0 component tests", `ui-primitives-plan.md:30`). Of 31 test files, 12 matched a broad DOM/storage grep but nearly all are `localStorage`/Tauri-IPC stubs (`workspace.test.ts:28`, `assistant.test.ts:3-13,1013-1118`, `assistant/helpers.test.ts:18,167,201`, `assistant.playback.test.ts:13,1686` `document.hidden`), not markup.

| Test | What couples | Survives a Halo port if... |
|---|---|---|
| `src/lib/utils.test.ts:5-26` | `cn()` merge rules for Rift Tailwind names: `text-fg`+`text-sm`, `text-fg`->`text-fg-muted`, `bg-surface`->`bg-surface-hover`, `border-border`->`border-border-strong`, `ring-ring`+`ring-2`, `rounded-md`->`rounded-card`, `rounded-sm`->`rounded-island`, `shadow-lg`->`shadow-float`, `ease-soft`->`ease-page` | the bridge keys `fg`, `fg-muted`, `surface(-hover)`, `border(-strong)`, `ring`, `card`, `island`, `float`, `page`, `soft` keep existing, and `utils.ts:13-17` stays in sync with any new non-T-shirt key (header `app.css:16-17` mandates this) |
| `src/lib/components/assistant/diffEmphasis.test.ts:59-103` | `emphasizeHtml()` emits `class="dem-add"`/`"dem-del"`; fixtures include hex `style="color:#f00"` (opaque input, not app tokens) | the two class names stay (defined `EditDiff.svelte:309,619,623`) |
| `src/lib/utils/projectHue.test.ts:18-19` | 4 sample names hash to distinct hues; no color values | project hue algo untouched |
| `components/assistant/toolCaption.test.ts:47` etc. | copy strings ("Finding **/*.svelte"), not CSS | copy unchanged |
| Others (`streamModel`, `modelMatrix`, `composer/helpers`, `bubble/helpers`, `usageStats`, ...) | data/logic | n/a |

Not covered by any test (so a port gets no safety net): every `.svelte` template, `app.css`/`stream.css`, `ui-prefs` (no test file), `accessibility.svelte.ts`, token resolution. The only automated style-adjacent guards are `check:tokens`, `check:ui`, `svelte-check` and the live CDP checks (section 7). A new `tokens` test is cheap to add (CX has `design-tokens.test.ts` asserting generated CSS is not stale).

---

## 7. Visual verification tooling

### 7.1 Inventory

| Tool | What | Cites |
|---|---|---|
| `/dev/gallery` (Ctrl+Alt+G, DEV only) | one scripted replay streaming every block kind through the real `StreamTurn` pipeline; speed 0.5/1/2; toggles for tool-detail / narration / command-output | `routes/dev/gallery/+page.svelte:1-70`; fixtures `lib/dev/galleryFixtures.ts` (199 ln); scrolls its own `.stage`, so `AssistantPane.pinToBottom` is NOT exercised (`redesign-plan.md:192`) |
| `/dev/ui` (Ctrl+Alt+U, DEV only) | every `ui/` primitive x variant x state x density (compact/regular/comfy) x accent hue; preview-only (does not persist) | `routes/dev/ui/+page.svelte:1-130` (445 ln). Covers button, dropdown-menu, context-menu, select, alert-dialog, separator. NOT the bespoke `.rift-menu`, tiles, toggles, seg, settings controls |
| CDP bridge (`scripts/cdp/c.sh` 665 ln, `serve.cjs` 1508 ln) | `HTTP :9223 -> WebView2 CDP :9222`; dev-only; focus-safe | `scripts/cdp/README.md:1-163`; skill `.agents/skills/rift-ui/SKILL.md` (+ `.claude/skills/rift-ui/`) |
| Preview servers | `.claude/launch.json`: `rift-frontend` (:1420), `rift-frontend-preview` (:1440) = plain Vite in a browser, no Tauri (no `setZoom`, no IPC, no `window.__assistant`) | `.claude/launch.json` |
| Static DS cards | 15 HTML cards, open in any browser, linked to `design-system/styles.css` | `design-system/README.md:1-39` |

### 7.2 `c.sh` commands that matter for a visual port (confirmed by reading the script)

`inspect [sel]` (state + current-generation console errors + AX tree, no screenshot), `map [sel]` (every actionable control with verified selector), `find <text>`, `text [sel]`, `look [sel]` (state + errors + screenshot path), `act click|key ... [sel] [ms]` (action + DOM settle + verification), `measure <sel> [nokids|::before|::after]` (`c.sh:302`; REAL geometry, resolved colors, CSS vars, radius, shadow, font: the tool for "is this really #14171c / 12px / 16px radius"), `baseline [sel] [name]` (`:383`, PNG ref `.tmp/base-<name>.png`) + `diff [sel] [name] [thr]` (`:397`, pixelmatch YIQ anti-alias-aware, reports % changed, region, size mismatch), `nav home|chat|settings|ai-health` (`:471`, clicks sidebar aria-label), `tour a b c` (`:498`, many surfaces in one round trip, one screenshot each), `ready`, `shot`, `shot-sel <sel> jpeg 65 hover` (hover/focus capture), `console error 20 1`, `reload`, `reset-viewport`, `eval`, `batch`, `reap --all`, `shutdown`, `doctor`.

Launch: `npm run cdp:dev` (medium-integrity de-elevated launcher; run in background), then `npm run cdp:serve` (background); two-instance setup with ports 9224/9225 in README. Isolated profiles `EBWebView-*`. Never kill `rift-tauri.exe` by image name (`AGENTS.md:41-43`).

### 7.3 Known gotchas (from `HANDOFF.md:44-50,125-126,153-160`)

- `c.sh look` returns empty every other call: loop until a path prints.
- `look "<selector>"` crops land offset at DPR 1.25 on this machine: scrollIntoView + full-window `look` instead.
- `.tmp` keeps ~20 snaps; copy keepers to `docs/design/halo/` before they rotate.
- Never type a slash command + Enter into the composer over CDP (fires SlashMenu item; one real ChatGPT turn was sent that way). Set textarea via `eval` + `input` event, click `.send-btn`.
- Home/hero hides the composer (`{#if !hero}`), so `.model-pill` exists only inside a conversation.
- A real (paid) turn is needed to exercise scroll-follow and live streaming; ask owner first (`redesign-plan.md:194`).
- The embedded browser child WebView is a separate target (`-t browser`); it paints above DOM (`ui-primitives-plan.md:132-134`).
- Tooltips never paint (`actions/tooltip.ts:1-9`: no-op shim promoting text to `aria-label`): you cannot visually verify Halo `Tooltip` until one exists.

### 7.4 Per-phase usage (proposed; nothing run)

| Phase | Before | During | Gate/after |
|---|---|---|---|
| Tokens/alias layer | `c.sh baseline` the 4 `tour` surfaces (home, chat, settings, ai-health) full-window + `/dev/ui` + `/dev/gallery` at a fixed viewport (the prior audit used 2048x1104 @ DPR 1.25, `redesign-plan.md:26`; `reset-viewport` exists) | `c.sh measure` on 8 probes (`.btn`, `.tile-head`, `.rift-menu-row`, composer, sidebar row, kbd, pill, input) for exact resolved values | `c.sh diff` per surface (expect small controlled diffs), `npm run check` + `npm test`, `inspect` 0 console errors |
| Fonts | measure text widths of the same labels (Inter->Geist reflow; DESIGN-system fix at `HANDOFF.md:61-63` showed Lexend vs Inter 314 vs 330px greeting) | `measure` typography fields | `tour` + check truncations/wraps |
| Primitives/components | `/dev/ui` shots per density x accent | `shot-sel ... hover` for hover/focus states | `/dev/ui` side-by-side vs Halo reference |
| Motion | `/dev/gallery` replay + the rAF frame sampler from the prior session (scratchpad, not durable: p50 5.6ms etc., `redesign-plan.md:183-187`) | -- | rerun sampler; owner "feels" pacing (motion is the most-rejected category, `redesign-plan.md:209`) |
| Reduced motion | emulate `prefers-reduced-motion` (needs `Emulation.setEmulatedMedia`, not exposed as a `c.sh` command: **unknown**; use `eval`/devtools) | | confirm static end states |
| Pixel truth vs Halo | -- | `measure` Rift element vs Halo mock element values | `diff` can't compare against the React mock (different render path): visual judgement only |

---

## 8. `design-system/` folder + Rift Style DS: fate

### 8.1 What exists

`design-system/` (also described `DEVELOPING.md:71`: "static visual reference and token mirror"; not part of runtime, `README.md:3-6`):
- `README.md`, `DESIGN.md` (308-line taste layer, sections 1-10; section 8 rejection log at `:129-251`), `styles.css` (152 ln, Graphite mirror, `@import`s Google Fonts Inter/JetBrains/Lexend `:8`), `_ds_manifest.json` (15 cards, 56 token entries, brand fonts Inter/JetBrains `:10-13`), 15 HTML cards (3 foundations, 7 components, 5 pages), `assets/rift-logo.png`, `check-tokens.mjs`, `check-ui.mjs`, `ui-baseline.json`. `_briefs/` is gitignored (`.gitignore:59`).
- The folder mixes THREE roles: (1) taste rules (DESIGN.md, gate-adjacent, still valuable), (2) static cards (drift-prone, "no automated guard", `README.md:30-32`; settings card already stale, 4.2), (3) CI gates (the two scripts live here but protect `src/`). Retiring "the design system" must not silently drop gates.
- Published **Rift Style** Claude Design System artifact (private, `claude.ai/artifact/WSAPanWKnSrrtj47mrW4Tu`, built from `main@f2f4a82`, republished v5 with the `fg-subtle` change; `HANDOFF.md:54-77`): tokens, README, rejection log, 10 component cards, fonts, logo. Contents NOT re-read this wave (**unknown** beyond HANDOFF); Graphite-era. HANDOFF.md:76-77 and redesign-plan.md:221 already flag the static cards as overlapping it, owner's call.

### 8.2 Fate options

| Option | What | Pros | Cons / what breaks |
|---|---|---|---|
| R1 Retire cards, keep gates | delete the 15 cards + `_ds_manifest.json`; repoint `check-tokens.mjs` at a real source; keep `DESIGN.md` (rewrite) and `check-ui.mjs`/baseline | removes the stale duplicate; the redundancy is already flagged | per global rule **do not delete without owner say-so**: this is a FLAG, not an action. `check-tokens` loses its mirror target; needs rework or removal (`package.json:10,12`) |
| R2 Regenerate as "Halo-for-Rift" | `styles.css` <- Halo tokens + Rift compact scale; regenerate cards (or only foundations + the components that survive); manifest regenerated from tokens | keeps one brand reference + the drift guard, mirrors CX's generated `tokens.css` pattern with a freshness test (`CX/design-systems/build-tokens.ts`, README `npm run tokens`) | cards are hand-built static HTML (15 files); cost high, value moderate; risk of a second Graphite->Halo drift cycle |
| R3 Replace with Halo vendored | vendor `tokens.json`/`tokens.css`/(optional) `bundle.css` for reference, treat Halo's own docs as the DS; Rift-specific deltas in one `DESIGN.md` | single source = Halo; `check-tokens` can diff the alias layer vs Halo `tokens.css` (strongest guard) | Halo bundle is React; cards not usable as Svelte truth; need Rift-delta doc (compact scale, tiles, density) |
| R4 Do nothing | leave Graphite cards | zero work | cards actively wrong post-port; `check-tokens` forced green by editing a stale mirror (worse than no check) |
| Rift Style artifact | republish as Halo-Rift after tokens land, or leave private & stale and mark deprecated | | reading it this wave was out of scope; **unknown** how Halo/Claude design tooling expects it |

Suggested neutral recommendation for the owner: R3 + keep a rewritten `DESIGN.md` (taste + rejection log: this file encodes ~20 owner calls, see section 9) + keep `check-ui` and `ui-baseline.json`; repoint `check-tokens` to verify the alias layer against vendored Halo `tokens.css`. Requires Q4.

DESIGN.md will contradict Halo after the port in at least: section 1 (Graphite Ink), section 2 (`--bg-inset` ladder, solid-accent scarcity, 5-step text ladder), section 3 (Inter/JetBrains; mono for code only; 600 ceiling), section 5 (onyx slab, recessed chassis), section 6 (motion tokens), section 8 (several bans). Treat it as a rewrite item, not an edit.

---

## 9. Guardrails (AGENTS.md / ARCHITECTURE / DESIGN.md) that constrain a redesign

### 9.1 Hard repo rules

- Svelte 5 runes only; no legacy stores (`AGENTS.md:19-21`). Build new UI from `components/ui/` primitives, no hand-rolled portals/menus/dialogs (`AGENTS.md:45-49`); `npm run check` enforces via check-ui. Adding a primitive: `npx shadcn-svelte@latest add <name>` (NEVER `init`, it rewrites `app.css`), restyle to `recipes.ts`, add to `/dev/ui` (`DESIGN.md:304-307`).
- Gitignored working docs: `docs/HANDOFF.md`, `docs/ISSUES.md`, `docs/design/` (`.gitignore:58`, verified with `git check-ignore`; this map file is therefore untracked, and `design-system/_briefs/` is `:59`). `design-system/` itself is tracked.
- Node must be 24.x (`.node-version`, `package.json:31-34`); `svelte-kit sync` prefixes in `check`/`test` must stay (`CLAUDE.md` Verification). Cargo not buildable on Linux; a Rust change from cloud is unverified.
- Never kill `rift-tauri.exe` by image name; no `cargo check` while `tauri dev` runs (`AGENTS.md:41-43`).
- Keep Halo/token work provider-neutral: model names, capabilities, and billing are provider data, not UI constants (`AGENTS.md:51-73`). Do not repeat provider/model/plan status in several surfaces (`DESIGN.md` section 8 first bullets).
- ARCHITECTURE invariants relevant to UI: `TabState` is the frontend unit of truth; sidebar `ProjectSwitcher` is the single scope control; do not add parallel All/project toggles (`ARCHITECTURE.md:146-155`); `components/shell/workspaceRegistry.ts` lazy screen registry; removed subsystems stay removed (`:201-203`, `CLAUDE.md`).
- Window min size 900x600 (`tauri.conf.json:18-19`), `decorations:false` custom titlebar. Halo's `bp-md` 900 / min-height 600 flip to scrolling pages (`bundle.css:18`) - exactly Rift's minimum window, so Halo's "fit one screen" rule applies at all supported sizes.
- WebView2: **no `backdrop-filter` in dialogs/fixed layers** (mis-composites; `app.css:274,609`, `DESIGN.md` section 10). Rift still ships 12 active `backdrop-filter` declarations in 11 files (BoundaryBlock:113, ModelSwitchBlock:63, EnhanceBar:203, SettingsMenu:520, SlashMenu:143, Composer:1886/1921, Markdown:1212, ActivityHud:326, PlanHud:224, ToolChip:730, `styles/onboarding.css:154`). Halo uses it only in the Game category (`bundle.css:746,780`) and says solid for NUI. Compatible in intent; the existing violations should go in the port.
- Reduced motion: Rift = 48 files with per-component blocks (117 mentions) + global `[data-slot]` kill; Halo = one global `*{animation-duration:.01ms !important}` (`bundle.css:901-903`). Adopt Halo's global net as a belt, keep component rules where JS-driven (`prefersReducedMotion()` `accessibility.svelte.ts:103`).

### 9.2 DESIGN.md section 8 rejection log vs Halo

Rift's owner has rejected these explicitly (so Halo features that reproduce them will be rejected again):

| Halo feature / principle | Rift rule it collides with (cite `design-system/DESIGN.md`) |
|---|---|
| `Card spotlight`: radial light following cursor along the hairline (`bundle.css:33-35`) | "no traveling light", orbiting comet arc rejected 2026-07-22 (:245-251), banded sweep rejected 2026-07-21 (:227-234); "stationary signals only" |
| "No colour washes", depth by lightness (README principle 1) | Aligned in spirit with DON'T stack atmosphere (:190-196) but Rift's slab recipe is itself layered: vertical gradients + SVG grain + bevel + AO + cavity + seam (`AppShell.svelte:343-501`, `Sidebar.svelte`); owner explicitly approved it (:81-95). Port must decide: keep the chassis around Halo content, or flatten to `bg-000/100` (Q7) |
| `Button magnetic`, `FocusGroup dim` (opt-in) | motion exists for orientation, not delight (:99-103) |
| Mono for labels/numbers incl. uppercase `label` 11px 0.12em | Rift section label = 10px/600/uppercase/0.08em Inter, mono only for code (:43-44,38-39) |
| 44px controls, 40px menu rows, 15px body | compact is the default posture (:53), density var contract (:51) |
| `ink-faint` as the faint step | "don't equate quiet with invisible" (:186-189) - actually aligned: Halo's floor is stricter than Rift's L .43 |
| Positive = accent (no green) | Status vocabulary (:30-33) and many ok-colored diff/test chips; DESIGN says ok/warn/danger/info are accent-independent |
| Four accent presets, one signal colour | multi-hue project avatars + 5-hue model-mix bar (redesign-plan.md:71-73, open call :223) |
| Halo `Tooltip` is a required companion of icon-only buttons (`05-content-and-icons.md`) | Rift pulled tooltips on purpose 2026-06-15 (`actions/tooltip.ts:1-9`), ~26 `use:tooltip` call sites |
| Halo `Toast` stack that fans out / swipe | Rift `ToastHost` + `NotificationCenter` + severity-tone sheet exist with `z-index` 2000/2100 (`app.css:277-280` comment); behavior is bespoke |
| Halo `CommandPalette`, `Select` Halo list, `ContextMenu`, `Dialog`, `Drawer` | Rift has `dialogs/CommandPalette.svelte`, `ui/` select/context-menu/alert-dialog; must reconcile rather than add a second set |
| Halo layer order dropdown(20) < overlay(50) | Rift popover(9000) > dialog(4000) deliberately (`recipes` comments, `app.css:277-282`) |

### 9.3 Things the owner has already said (preserve unless overridden)

From DESIGN.md section 8 and HANDOFF: solid accent is rare (Send + <=1 CTA/screen); no pill buttons/inputs (pills only for badges/toggles); chrome quieter than content; island nesting one level; one fact one home; amber = costs money; no atmosphere stacking; no dots/specks; motion changes are offered as single-value toggles felt live, never shipped speculatively (`redesign-plan.md:209-215`); word-reveal blur and traveling rail glint already marked for removal (`redesign-plan.md:196-207`, `stream.css:65-68` notes glint removed). A Halo port is by definition an owner override of the Graphite brief; the 2026-10-19 brief was "EVOLVE Graphite Ink, not a new look" (`redesign-plan.md:9`) and the HALO decision supersedes it per the task's CONTEXT, but section 8 rules not tied to Graphite's palette should be re-confirmed rather than assumed dead.

---

## 10. Risks

| # | Risk | Evidence | Mitigation |
|---|---|---|---|
| R1 | Un-namespaced Halo import silently re-radii/re-times Rift (17 collided names) with no failing gate | section 1.3 | alias layer last; add a token-freshness/assert test |
| R2 | Renaming Tailwind bridge keys hard-fails `check:ui` and breaks `cn` test | `check-ui.mjs:54-91`, `utils.test.ts:21-26` | keep keys; retarget values only |
| R3 | `check:tokens` goes red or goes meaningless once tokens leave `app.css` | `check-tokens.mjs:45-60,95-98` | decide fate in same phase (section 8); never "fix" by editing a stale mirror |
| R4 | CSP blocks Halo font load; offline users get system font | `tauri.conf.json:28` | fontsource only |
| R5 | Geist reflow changes widths/truncations across 261+ small-text sites; Inter-specific OpenType tags dropped | `app.css:306-307` | `measure` before/after; per-surface sweep |
| R6 | Density system (3 modes, `--fs-md` shifts) has no Halo analog; 44px defaults would wreck the compact desktop layout (28px composer controls, 26px rows) | `app.css:289-291`, `bundle.css:43,419,842` | author a compact Halo scale; decide whether density modes survive (Q1) |
| R7 | Accent dial removal loses persisted user prefs; or accent derived from hex loses hover/active ramps | `ui-prefs.svelte.ts:123-127,291-296` | option (a) section 4.4 |
| R8 | Semantic color drift: `ok`->accent (blue success), Halo `warn` = negative orange, blue-preset `info` = lime, `--status-busy` green | `tokens.css:63-69`, `CX/src/app.css:62-66` | explicit semantic map + owner sign-off (Q3) |
| R9 | Light themes are not free: 131 oklch literals, 93 gradients, black-based shadows, shiki dark theme, warm-tint, mask `#000`s assume dark | section 4.5 | scope light out of the first wave (also out of scope in the primitives pilot, `ui-primitives-plan.md:125`) |
| R10 | Three-plus parallel button/pill/input dialects (`.btn`, `.st-*`, `.ob-*`, `.sxa-btn`, `ui/button`) mean "port the button" touches 4 sheets | section 2.3 | one canonical Button, alias or retire others per surface |
| R11 | Latent undefined-var bugs get masked/unmasked by new token names (`WorkspaceShell` panel, `--font-sans`, `--bg-0`) | section 1.4 | fix explicitly; do not rely on collisions |
| R12 | Native/Rust and pre-JS colors drift (browser child bg, `app.html`) -> visible flash on load | section 4.5 | change both with the token phase; Rust change is unverifiable from Linux |
| R13 | No CI on feature branches; only push-to-main runs gates | `check.yml:4-11` | run `npm run verify:frontend` locally before every merge |
| R14 | No DOM tests: a visual regression ships green | section 6 | per-phase `baseline`/`diff` + `/dev/*` shots; consider adding 2-3 pure token tests |
| R15 | Motion pacing changes are historically the most-rejected category (word reveal 42ms/word, scroll lerp 0.2) | `redesign-plan.md:178-215` | single-value toggles, owner feels live, separate revertable commits |
| R16 | Halo's global reduced-motion `!important` net would also clobber JS-driven animations that need end states | `bundle.css:901-903` | keep component-level handling for JS-driven reveals |
| R17 | 160 keyframes + 36 duplicated easing literals + 25 z-index literals make "motion happy medium" a sweep, not a token swap | section 1.5 | collect in a motion pass; lint for literals |
| R18 | `onboarding.css` contains wash gradients + dot grid already against DESIGN section 8 | `onboarding.css:18-21,373-377` | resolve in the onboarding surface map |
| R19 | Halo components are React-only; behavior (focus trap, FlipList, count-up, toast stack) must be re-implemented as Svelte actions/bits-ui | `HALO/README.md` "Using it" | follow CX `lib/motion.ts` precedent; reuse bits-ui for behavior |
| R20 | Halo Product docs unread at component level (per-component READMEs); sizes/behaviors for Segmented, Tabs, Toggle states beyond sampled CSS are **unknown** here | -- | other map files own surfaces |

---

## 11. Open owner questions

1. **Density + sizing.** Halo is 44px controls / 15px body; Rift is 26px / 13px with compact-regular-comfy. Keep the 3 density modes on a "Halo compact" scale (my assumption, CX did compact-ish: 32-40px controls, `CX/src/app.css` heights), collapse to one density, or move to Halo's real touch sizes? (Also: is the 12px sans floor OK, given 261 sub-12px declarations to lift?)
2. **Accent model.** Keep hue/vividness dials with Halo-blue as the default (a), or switch to Halo's fixed presets and delete the dials (b)? If (b), which presets does the app expose (blue only, or all 4 + light)?
3. **Semantic color.** Success/"ok" (101 ext refs, diff adds, test pass, connected): keep a green, or follow Halo (accent = positive)? Is "no green anywhere" (the 2026-10-17 ConsoleX call) a Rift rule too? Lime `info` in the blue preset is a no-go presumably.
4. **Fate of `design-system/` cards, `styles.css`, manifest, `check-tokens.mjs`, and the published Rift Style artifact:** R1/R2/R3/R4 in section 8.2. Deletion is your call; nothing was touched.
5. **"One signal colour."** Per-project hue avatars (`projectHue`), the per-model color idea (already collapsed to accent in the composer, `Composer.svelte:1854-1857`), and the Home model-mix multi-hue bar: keep, neutralize, or restrict to one place?
6. **Stream tile vocabulary** (`.tile*`, `.sledger*`, `.wline*`): keep as Rift-specific components restyled with Halo tokens, or rebuild on Halo `Card`/`CodeBlock`/`Banner` anatomy? (Halo has no analog for tool-run chips, plans, ledger receipt.)
7. **Chassis.** Keep the onyx-slab recessed-chassis + rift seam + grain (owner-approved 2026-07-16/23) around Halo content, or flatten to Halo's `bg-000`/`bg-100` + hairlines (and delete 93 gradient declarations)?
8. **Fonts.** Geist/Geist Mono replacing Inter/JetBrains: fully, or keep JetBrains for code? Lexend: keep as dyslexia face only, or also brand (greeting, splash)? Is Halo's "mono for labels/numbers" acceptable given DESIGN section 3 / redesign C13?
9. **Tooltips.** Halo expects them on icon-only buttons; Rift removed them on purpose (2026-06-15). Reintroduce (Halo `Tooltip`) or keep aria-only?
10. **Light themes.** In scope for the port (Halo ships 4) or explicitly out until a later wave?
11. **Branching/merge.** Port on `design/chat-ledger` (holds the unpushed Ledger commit `8bb71f4f`) or a new branch? CI only watches `main`.
12. **Layer order.** Keep popover > dialog (Rift) over Halo's dropdown < overlay? (I assume yes; menus inside dialogs must work.)
13. **Rejection log.** Which section-8 rules are retired by the Halo decision and which still bind (e.g. no pill buttons, solid-accent scarcity, no traveling light)? DESIGN.md needs a rewrite either way.
14. **Halo motion extras** (`Card spotlight`, `Button magnetic`, `FocusGroup dim`, toast fan, rolling numbers): adopt any, given the traveling-light bans? Suggested default: rolling numbers + page/rise entrances + tabular figures yes; spotlight/magnetic no unless you say so.

---

## Appendix A. Files read this wave (all read-only)

`src/app.css` (1-780), `src/app.html`, `src/routes/+layout.svelte`, `src/lib/styles/{stream,onboarding,settings-controls,severity-tone}.css`, `src/lib/components/ui/recipes.ts`, `ui/button/button.svelte`, `src/lib/utils.ts`, `utils.test.ts`, `state/ui-prefs.svelte.ts`, `state/accessibility.svelte.ts`, `utils/projectHue.ts`, `actions/tooltip.ts`, `AppShell.svelte:336-510`, `routes/dev/{ui,gallery}/+page.svelte` (heads), `vite.config.js`, `vitest.config.ts`, `svelte.config.js`, `components.json`, `package.json`, `scripts/verify.ps1`, `.github/workflows/check.yml`, `.claude/launch.json`, `src-tauri/tauri.conf.json` (excerpt), `browser/mod.rs:210-222`, `design-system/{README,DESIGN}.md`, `check-tokens.mjs`, `check-ui.mjs`, `ui-baseline.json`, `_ds_manifest.json`, `styles.css` (head), `AGENTS.md`, `CLAUDE.md`, `docs/ARCHITECTURE.md`, `docs/design/{README,ui-primitives-plan,redesign-plan}.md`, `docs/HANDOFF.md:1-160`, `docs/DEVELOPING.md` (excerpt), `.agents/skills/rift-ui/SKILL.md`, `scripts/cdp/README.md`, `scripts/cdp/c.sh` (selected regions). Halo: `README.md`, `01-03,04-06,08-*.md` (+05, 07), `design-system.json`, `tokens.json` (type/duration/easing/z), `components/index.d.ts` (head), `components/bundle.css` (grep samples only), `consolex tokens.css`, CX `main.ts`/`app.css` head/`motion.ts`/`build-tokens.ts`. NOT read: per-component Halo READMEs, `bundle.css` beyond samples, `serve.cjs`, `release.yml` beyond grep, published Rift Style artifact, most `.svelte` files (other maps own surfaces).
