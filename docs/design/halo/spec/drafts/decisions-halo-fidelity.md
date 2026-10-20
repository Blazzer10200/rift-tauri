---
title: Halo decision register — draft, lens = HALO FIDELITY
status: draft (one of several lenses; reconciler merges)
updated: 2026-10-19
branch: design/chat-ledger (HEAD 22727d24; Ledger 8bb71f4f)
inputs: docs/design/halo/map/01..12
---

# Decision register — Halo fidelity lens

Map cite shorthand: `M01 §4` = `docs/design/halo/map/01-shell.md` section 4, etc. Owner IDs (L*, D*, C*, T*, K*, M*, X*, Q*) = `M12`.

## Posture (how this lens decides)

1. **Default = Halo exactly as shipped** (`bundle.css` + ConsoleX `bundle.js` numbers, `M10`). Owner: "completely switch up the design to the Halo design"; "Electric blue is the one. Use it as is."
2. **Deviate only for** (a) a hard technical constraint — CSP `font-src 'self'`, WebView2 `backdrop-filter` mis-compositing, native child-webview z-order + `syncBounds`, Tauri drag regions / `position:fixed` shell, menu-inside-dialog z-order, `TabState` per tab, no DOM tests; or (b) a **recorded owner rejection** Halo would re-trigger (M1 blinking stream caret, M2 banded sweep, M3 orbiting light, L2 full-sidebar peek, L10 tabs pill, K1 pill buttons, C8 dots/specks, D8 full-width tool bars "tacky"). Each deviation names which.
3. **Graphite-era approvals do not bind** (L1/L6 chassis, D1 compact default, M16 `.shim`, T1 Inter, C17 hue dials). They are "renegotiate", per `M12 §0.1`. Where this lens overrides one, it is listed in expected disagreements.
4. **Halo silent ≠ conflict.** Streaming reveal, Ledger, diff, terminal output, HUDs, splash, status bar, window controls, 4-pane split, composer FLIP, dense tool rows, grep results: Rift-specific parts built on Halo tokens/motion. No fidelity question.
5. **Halo opt-ins stay off** (spotlight, magnetic, FocusGroup dim). That is fidelity-consistent, not a deviation.
6. **ConsoleX = Halo as the owner practises it** (`M11`): where ConsoleX shipped a Halo choice the owner saw and kept (info override, one-row composer card, compact header, monochrome diff, flat frame), it counts as owner evidence.

## Owner sign-off list (taste-critical AND costly to reverse)

| Gate | Default work proceeds on | Shown on canvas board |
|---|---|---|
| T1 frame | Halo flat docked frame | "Shell: flat vs Halo-islands" side by side |
| T6 type + density | Halo scale, 15px prose, 44 page / 36 toolbar controls, density pref kept | "Foundations: density tiers" + "Chat: 15 vs 13 prose" |
| T11 reveal feel | 22727d24 numbers, opacity-only fade, two correctness fixes | "Motion lab" (live sliders) |
| T18 syntax colour | monochrome ink theme (one-constant swap back) | "Chat: code/diff/terminal mono vs Shiki" |

Everything else: default + proceed. Non-blocking owner FYIs (deletions = owner per CLAUDE.md, never auto): retire `design-system/` cards + `styles.css` + manifest; deprecate Rift Style artifact; delete legacy `MessageBubble` assistant branch + Stream-view toggle; delete dead Diagnostics modal mode; drop Inter/JetBrains deps; new Rust overlay-lock IPC.

---

### T0 Prerequisites + cross-cutting (added; blocker-class)
- **Native child-webview overlay contract** (`M07 §9`, `browser/mod.rs:8-11`): any HTML overlay over `.wb-stage` is hidden under the native webview. DEFAULT: new IPC `browser_overlay_lock(on)` driven by a ref-counted `overlays` store, registered ONLY by scrim-bearing layers (dialog, drawer, palette) and menus/popovers anchored over the dock; while >0 and dock open → `browser_hide` + paint `bg-000` placeholder in stage with page title; restore on 0. Tooltips never lock (suppressed inside stage bounds); toasts never lock (anchored outside the stage, T20). Avoids hide/show flicker on hover/notify. Not a style choice; prerequisite for T20 near the dock. Rust change = verify on Windows only.
- **`floatingMotion` may be inert** (`M10 §5.2 #2`): `recipes.ts` uses `data-open:/data-closed:` variants bits-ui never emits. CDP-verify before building overlay motion; implement Halo exits on `[data-state=closed]` / `[data-ending-style]`.
- **End-of-turn blink** (`M03 §4.1`, unverified high suspicion): record via CDP before T12 work.
- **Markup hooks to keep or migrate in the same commit**: `.sturn`, `.bubble[data-role]`, `.stream`, `.csurf-col`, `.composer-wrap`, `.ctxring`, `.splan`, `#sacard-<id>`, `.pane-shell textarea`, `.ob-host`, `data-composer-popover`, `[data-qedit]`, `#card-*` + `.sflash` (Settings jumpTo), `settings-tab-*/panel-*` ids, `window.__riftDiag`, nav aria-labels used by `c.sh nav/tour` (`M01 §9`, `M02 §6`, `M06 §6`, `M07 §9`).
- **Keep-alive pages + entrance replay**: pages stay mounted (`WorkspaceShell`); entrances must fire on activation (T10 page glide keyed on `activeId`) and must NOT replay on remount/re-parent (split swap, tab drag): copy ConsoleX `.panes.swap` 0s guard (`M11 §4.1`).
- **Undefined vars** to fix in token phase: `WorkspaceShell.svelte:155-187` `--text-primary/-secondary/-muted/--border-subtle/--bg-raised` → `ink/ink-muted/ink-faint/line-100/bg-200`; `--bg-0` → `bg-000` (`M08 §1.4`).
- **Behaviour gaps out of scope, flagged**: stream-tree max_tokens/refusal notices (`M02 §2`); StreamWeb `sources` always `[]`.
- Unblocks: T8, T12, T20, every surface phase.

### T1 Material / frame
- Options: (a) Halo flat docked frame; (b) Rift islands re-skinned in Halo steps (bg-100 islands on bg-000, 12px gutter, radius-lg, line-100, keep AO/top-catch/seam); (c) keep onyx slab (gradients, grain, cavity, bevel, seam).
- **DEFAULT (a)**: ground `bg-000`; sidebar `bg-000` + `border-right:1px line-100`; sticky top bar `bg-000` + `border-bottom line-100`; content area `bg-000` (cards `bg-100` carry depth). Delete chassis gradient, 4 SVG grain layers, cavity insets, AO, bevel, `.rift-seam` in app chrome, `.app` 1s bg transition, page-local washes (onboarding radials + dot grid, welcome aura, hub card washes, advisor/diagnostics/update washes) — 93 gradient declarations (`M08 §0.6`).
- Why: Halo principle 1 "depth by lightness, no washes" (`M10 §4.1-2`); first Halo mock drew the flat frame (`M01 §7`, `Main.dc.html:27-60`); ConsoleX shipped flat (`M11 §3`). Rift's own C7 ("shit stacked on top of shit"), C8 (no dots) already point here.
- Deviation: none. Risk: C6 "still not impressed… more immersed" (07-16) — answered by Halo's 4-step lightness + bg-100 cards, not by texture.
- Reversibility: costly (slab recipe + 93 gradients; restore = re-author). **OWNER-GATE: yes.**
- Unblocks: T19 shell, all page backgrounds, `app.html:9` + Rust `Color(9,10,11)` → `#08090c` / `Color(8,9,12,255)`, splash panel colour.

### T2 Accent model
- Options: (a) lock `data-theme="blue"`, retire dials; (b) Halo preset picker (blue/lime/ember/violet, dark only); (c) keep OKLCH hue + vividness with blue default.
- **DEFAULT (a)**: `<html data-theme="blue">` in `app.html`; Halo `--accent #6aa8ff`, `--accent-ink #08090c`, `--accent-soft rgba(106,168,255,.14)`, `--glow-strong rgba(106,168,255,.28)` from vendored tokens. Remove Settings Accent card (8 swatches, hue, vividness, preview) and onboarding step-1 swatch strip; `/dev/ui` accent bar removed. 13 `oklch(… var(--accent-h))` literals → `color-mix(in srgb, var(--accent) N%, transparent)`.
- Migration: stop reading `rift.ui.accent.v1` / `rift.ui.vividness.v1`; leave keys in storage (no delete) so (c) can return. Drop `applyAccent()` writes.
- Why: owner "Electric blue is the one. Use it as is." Halo = fixed hex per preset (`M08 §4.3`). Dials break Halo contrast guarantees (`M10 §1.1` measured on fixed hex).
- Deviation: none. Overrides C17 (owner-built feature) → expected disagreement.
- Reversibility: cheap-medium (prefs keys kept; UI re-addable; (b) is a later additive). OWNER-GATE: no (owner already chose).
- Unblocks: T8 alias layer (no `--accent-h/-c/-a`), Settings Appearance tab, onboarding step 1.

### T3 Semantic colours on blue
- **info**: ConsoleX override `:root[data-theme="blue"]{--info:#9cc6ff;--info-soft:rgba(156,198,255,.14)}` (`M11 §1.2`, commit `1aebd1e`, owner "no green anywhere"). Also override `--chart-2` to drop lime (see T24).
- **ok / success**: fold into accent (Halo "positive = accent on accent-soft", `M10 §4.1-4`): `--ok: var(--accent)`, `--ok-soft: var(--accent-soft)`. Connected/pass/done/Allow/Auto-edit/ctx-ok all go blue. Auto-edit vs Send share a hue: fine — Send is the only solid fill.
- **status-busy** (#22c55e) → accent via `LiveDot`; **status-pending** → `info`; **status-warn** → `warn`.
- **Collision guard**: accent-blue vs info-light-blue are adjacent; route log-level `info`, plan-mode perm tone, MCP neutral notices to **neutral** (`bg-300`/`ink-muted` Badge). `info` reserved for "upstream / informational banner" only.
- **warn vs cost-amber**: Halo `warn #ff9f5a` (orange, "negative"). Keep Rift rule C3 "costs money = warn treatment at the control" on Halo `warn` — same family, semantics compatible.
- **diff add/del** (ConsoleX canvas `M11 §3.1`): add row wash `var(--accent-soft)`, `+` glyph accent; del row wash `rgba(255,107,107,.08)` (`color-mix(danger 8%)`), `−` glyph danger; word emphasis = same tone at 28%. No green.
- **danger**: Halo `#ff6b6b`. **Deltas**: Halo up=accent / down=warn by sign; Rift extension `goodWhen` so cost-up renders warn (Halo silent on inverted metrics).
- Deviation: info override (owner's own ConsoleX practice, not a Rift deviation); `goodWhen` (semantic correctness).
- Reversibility: cheap (token values). OWNER-GATE: no.
- Unblocks: T8, stream pills, Settings pills, toasts, diffs, AI Health tones.

### T4 Light themes
- Options: in scope now / later / never.
- **DEFAULT: out of scope.** Ship dark `blue` only; do not vendor light blocks into the active stylesheet (keep `tokens.json` source intact). Use `color-mix()` not rgb triplets from day one so light stays possible (ConsoleX lesson, `M11 §1.2`).
- Why: Rift dark-only (`project_rift_tauri.md:14`, C13); 131 oklch literals, shiki dark, black shadows assume dark (`M08 R9`). Halo itself ships light as separate presets — omitting presets ≠ deviating.
- Reversibility: cheap to add later. OWNER-GATE: no. Unblocks: scope of T8.

### T5 Fonts
- **DEFAULT**: `@fontsource-variable/geist` + `@fontsource-variable/geist-mono` ^5.3.0 imported in `+layout.svelte` before `app.css`; also `@fontsource-variable/geist/wght-italic.css` (markdown emphasis, else synthesized slant, `M10 §6`). Override `--font-sans:'Geist Variable', ui-sans-serif, system-ui, sans-serif` and `--font-mono:'Geist Mono Variable', ui-monospace, 'SFMono-Regular', monospace` (fontsource family names ≠ tokens.css "Geist"). Strip `bundle.css:1` Google `@import`.
- Retire Inter + JetBrains Mono imports; drop Inter `font-feature-settings` (`app.css:306-307`) and body `letter-spacing:-0.005em` (`app.css:314`, `AppShell:358`). Package removal = owner FYI.
- **Lexend**: dyslexia override only (T7 owner feature kept, `app.css:747-756`). Greeting + splash wordmark move to Geist 600 −0.03em. Settings font option label "Inter" → "Geist".
- **Mono role**: Halo — mono (`data` 13/18 500, tabular) for numbers/amounts/counts/durations/cost and `label` 11/14 caps; sans for words. Ledger receipt: words sans `caption`, numbers `.halo-mono`. Code/paths mono.
- `--code-liga` pref: keep; Geist Mono ligature support unknown → verify, pref may no-op.
- Deviation: none (bundling forced by CSP `tauri.conf.json:28`).
- Overrides: T1 (Inter), T4/C13 (no mono in chrome) → expected disagreement.
- Reversibility: medium (text metrics reflow everywhere). OWNER-GATE: no.
- Unblocks: T6, composer ghost-mirror re-measure (`dict-ghost`, `magic-text` padding), every surface sweep.

### T6 Type scale + density
- Halo scale (author as tokens; Halo ships none as vars, `M10 §1.2`): `caption 12/16`, `small 13/18`, `body 15/22`, `heading 16/22 600`, `subtitle 20/28 600 −.02em`, `title 32/38 600 −.03em`, `display 48/52` (max one per page; never for stats), mono `label 11/14 500 .12em caps`, `data 13/18 500`, `metric 30/34 600 −.03em`. Weights 400/500/600 only.
- **Floor**: 12px sans, 11px mono label only. All 261 sub-12 literals lift (`M08 §1.5`). 380 `font-size:Npx` literals → tokens.
- **Chat prose**: 15/24 (mock `Main.dc.html:17`), `ink`; column `--chat-col: min(760px, 100% - 48px)` shared by messages, composer, alerts, jump pill, turn rail (mock 760; fixes M03 §2 844/880/1100 mismatch).
- **Controls**: Halo `md` 44 (Button, Select, SearchField, Tabs, NavItem, TextField 52, Toggle 44×26) on page surfaces (Home, Settings, Health, dialogs, onboarding). Halo `sm` 36 "inside dense toolbars" (Halo's own sanction, `M10 §4.1-8`): composer bar, pane head, stream chrome + ledger actions, browser toolbar, diagnostics filters, sidebar conversation rows (36px, ConsoleX precedent `M11 §3.3`), status bar items.
- **Density pref**: keep `data-density` (persisted user choice): `regular` (default) = above; `compact` = page controls drop to `sm` 36, rows 32, chat prose stays `body` 15 (no off-scale 14), UI copy `small` 13; `comfy` → alias `regular`. Density never introduces a size outside the Halo scale. Stream rhythm vars keep their density hooks.
- Halo `touch ≥44` rationale is touch-first; Rift deviates only via Halo's own `sm` tier — no new size invented.
- Deviation: compact density mode (Halo has none; kept because it is a persisted user pref, removal = data loss).
- Overrides: D1 "compact default", 13px prose → expected disagreement.
- Reversibility: costly (reflow across all surfaces, FLIP heights, `ROW_H`). **OWNER-GATE: yes.**
- Unblocks: every component sizing, `Diagnostics ROW_H` (= 28, one constant CSS+JS), composer FLIP `dy`.

### T7 Radius ladder, pills, control heights
- **DEFAULT**: `radius-sm 8` (menu rows, small tiles, bars, thumbs, Segmented thumb), `radius-md 12` (buttons, inputs, selects, popovers, menus, toasts, CodeBlock, Accordion cards, stream tiles), `radius-lg 16` (cards, dialogs, drawers, palette, composer card, user bubble), `radius-pill` (Badge, chips, tags, count pills, toggle track, Progress track). Halo one-offs: checkbox 6, kbd 6, skeleton bone 6, inline code 6 (mock).
- **No pill buttons** (Halo + K1 agree): send = 36×36 `radius-md`; `.jump-latest` → `Button secondary sm` radius-md (not pill); `.chip-btn`, `.ps-chip`, `.char-count`, `.enhance-toggle` → radius-md or Badge.
- Heights per T6; Rift 26/22/20 button sizes retire.
- Kill off-ladder radii (2,3,4,5,7,9,10,11,13,14) in sweeps.
- Deviation: none. Reversibility: cheap (tokens). OWNER-GATE: no. Unblocks: T8 radius aliases, all component ports.
### T8 Token architecture
- Options: (a) ConsoleX alias layer, Rift names kept, collisions resolved Halo-wins; (b) full rename to Halo names in every consumer; (c) namespaced `--h-*` import.
- **DEFAULT (a), Halo-wins on collisions**:
  1. Vendor `tokens.json` + generated `tokens.css` (from `consolex/panel/design-systems/halo/`) into `src/lib/styles/halo/`; never hand-edit. Load order (`+layout.svelte`): geist → geist italic → geist-mono → `halo/tokens.css` → `app.css` (ConsoleX `main.ts:1-4`).
  2. `app.css :root` becomes the alias layer. **Delete** Rift declarations of the 17 collided names so Halo values stand: `--accent --accent-soft --info --info-soft --warn --warn-soft --danger --danger-soft --scrim --radius-sm --radius-lg --dur-fast --dur-base --dur-slow --stagger --ease-spring --font-mono` (`M08 §1.3`). The silent retime `M08 R1` calls a risk is here the intended outcome — made deliberate, in one commit.
  3. Overrides block: `:root[data-theme="blue"]{--info:#9cc6ff;--info-soft:rgba(156,198,255,.14);--chart-2:#b79cff;--chart-3:#9cc6ff}` (T3/T24); `--font-sans/--font-mono` (T5).
  4. Rift-only aliases: `--bg:var(--bg-000)`; `--surface,--bg-elev-1:var(--bg-100)`; `--surface-hover,--bg-elev-2:var(--bg-200)`; `--surface-active,--bg-elev-3,--track:var(--bg-300)`; `--bg-inset:var(--bg-000)` (wells; ConsoleX `--well`); `--field:var(--bg-100)`; `--field-border:var(--line-300)`; `--border:var(--line-100)`; `--border-strong:var(--line-200)` (control sites triaged to `line-300`); `--fg,--fg-2:var(--ink)`; `--fg-muted,--fg-subtle:var(--ink-muted)`; `--fg-faint:var(--ink-faint)` (247 sites triaged: body-ish text → muted; never on bg-300); `--accent-fg:var(--accent-ink)`; `--accent-hover,--accent-active:var(--accent)`; `--ghost-border:var(--line-200)`; `--ring` retired → `--shadow-focus` (2px gap + 2px accent); `--ok/--ok-soft/--status-*` per T3; `--radius:var(--radius-sm)`; `--radius-xs:6px`; `--radius-xl,--r-card,--radius-2xl,--island-radius,--tile-radius:var(--radius-lg)`/`var(--radius-md)` for tiles; `--shadow-lg,--shadow-float:var(--shadow-pop)`; `--shadow,--shadow-sm:none`; `--ease-page,--ease-soft:var(--ease-out)`; `--dur-rise:var(--dur-slow)`; `--dur-page:520ms`; `--dur-page-out:0ms`; `--pulse-live:1.8s`; type tokens per T6 (`--fs-xs,--fs-sm:12px` caption, `--fs-md:13px` small, `--fs-lg:15px`, `--fs-xl:16px`, `--fs-2xl:20px` + new `--fs-caption…--fs-metric`); `--row-h`/`--gap` per T6 density.
  5. **z-index**: Halo names, Rift order (hard constraint: menu inside dialog, `M08 §1.2`): `--z-raised 1 · --z-sticky 10 · --z-overlay 50 (dialog/drawer/palette scrim) · --z-toast 60 · --z-dropdown 65 (menus/popovers/select/context/HUD) · --z-tooltip 70 · --z-splash 90` (Rift-only). Aliases `--z-dialog:var(--z-overlay)`, `--z-popover:var(--z-dropdown)`. Kill all 25 raw z literals. Deviation: dropdown above overlay+toast (menu-in-dialog).
  6. Kill dead `data-direction/data-mode` selector (`app.css:129-130` → `:root`); `app.html` `<html data-theme="blue" data-density="regular">`, inline pre-JS bg `#08090c`; Rust `browser/mod.rs:218` `Color(8,9,12,255)`.
- **Tailwind bridge**: keep every `@theme inline` KEY (`bg-surface`, `text-fg-2`, `rounded-card`, `shadow-float`, `ease-page`, `h-row`…); retarget nothing (they already point at Rift names → aliases). Add keys only when `ui/` needs them; sync `utils.ts` `createCn` + `utils.test.ts`.
- **Gates**: `check:ui` unchanged; `--update` after each phase that drops counts; new Halo parts live in `components/ui/` (ratchet-exempt, token-only). `check:tokens` rewritten: read vendored `tokens.css` + `app.css` alias block, resolve chains across both, FAIL on (i) any collided name redeclared in `app.css`, (ii) alias → undefined var, (iii) `tokens.css` stale vs `tokens.json` (ConsoleX freshness test pattern). Drops the `design-system/styles.css` + manifest mirror from the gate (no deletion).
- **design-system/**: R3 (`M08 §8.2`) — Halo vendored = the DS; `DESIGN.md` rewritten in cleanup phase keeping §8 rejection log, each rule tagged binding/retired; 15 cards + `styles.css` + manifest flagged for owner retirement; Rift Style artifact marked deprecated, republish as "Rift · Halo" later (owner FYI).
- Why: ConsoleX proved (a) with ~0 consumer edits (`M11 §1.2`); Rift has 4,806 `var()` refs (`M08 §0.1`); (b) churns every file for no visual gain.
- Reversibility: cheap per value (one file), costly as a whole. OWNER-GATE: no.
- Unblocks: everything. Phase 1 critical path.

### T9 Component strategy
- Options: (a) restyle `components/ui/` bits-ui primitives + `recipes.ts` to Halo; (b) new `components/halo/` kit hand-ported; (c) both.
- **DEFAULT (a)+CSS port**: one stylesheet `src/lib/styles/halo.css` = `bundle.css` Product category verbatim (class names `.halo-*` kept for 1:1 diff vs reference; strip `:1` font import, Game category, `game-glass`). Components under `components/ui/<name>/` per `M10 §5.1`:
  - CSS-only: Card, Badge, Kbd, LiveDot, Spinner, Divider, SectionLabel, EmptyState, Banner, NavItem, Breadcrumbs, Stepper, Field, TextField, SearchField, Button, NotificationBell, Sparkline, Skeleton.
  - bits-ui + Halo CSS: Menu (DropdownMenu), ContextMenu, Select, Combobox, Dialog/AlertDialog, Drawer (Dialog), CommandPalette (Command+Dialog), Popover, Tooltip, Accordion, Tabs, Toggle (Switch; retarget `aria-checked` selectors to `data-state`), Checkbox, RadioGroup, Slider (spans: rewrite thumb CSS), Progress.
  - Custom behaviour (Svelte actions/runes, numbers from ConsoleX `bundle.js`): Toast stack + drag, FlipList (`animate:flip` 460ms with `cubicBezier(.2,.8,.2,1)` helper), RollingNumber, count-up (`Tween` easeOutCubic 1100), Segmented/Tabs indicator (`useIndicator`), SplitPane bar visuals, Textarea autogrow, CodeBlock copy.
- **Rift-specific parts** stay in their domain folders, composed only from `ui/` parts + tokens: `assistant/stream/*` (StreamTile dense, DiffBlock, LedgerLine, TerminalBlock, ResultList, AgentTimeline, HudBar), `assistant/*` (ChatColumn, UserMessage, TurnRail, JumpToLatest, ScrollFollow, WordReveal), `composer/*` (ComposerCard, Waveform, DetentSlider, QueueRail), `settings/*` (SettingsRow, PreviewWell, SecretField, ProviderCard), `workspace/*` (ShareBar), `diagnostics/LogStream`, `webview/BrowserBar`, `shell/*` (WindowControls, StatusBar).
- **Retire dialects** (`M08 §2.3`): `.btn/.pill/.input` (app.css), `.st-*`, `.ob-*` controls, `.sxa-btn`, `.rift-menu*`, `.tile*` → aliased to Halo classes during migration, deleted when 0 consumers (flag).
- Why: avoids a third system (`M07 §9`); bits-ui gives focus/ARIA/typeahead/portal; ratchet stays green; ConsoleX hand-wrote Svelte from Halo CSS (`M11 §1.1`) — same approach, with bits-ui behaviour on top.
- Deviation: none (React bundle unusable in Svelte).
- Reversibility: medium. OWNER-GATE: no. Unblocks: `/dev/ui` catalogue as Halo reference, all surfaces.

### T10 Motion durations + easings
- **DEFAULT**: Halo tokens globally: `dur-fast 150` (hover/press/pop-in), `dur-base 300` (state, thumbs, accordion, dialog/drawer/toast/palette), `dur-slow 600` (entrances, draws, Stepper fill), `stagger 60` capped 12; one curve `ease-out (.2,.8,.2,1)`; `ease-spring` only flip/grab/add (toggle knob, check/radio pop, slider thumb, tag chips, drop-zone icon, bell count, send/stop icon swap).
- Named Halo exceptions only: page 520ms +60ms/10px; skel-out 360; menu-item 220 / 18ms; toast leave 240; modal/drawer/banner leave 200; pop leave 140; FLIP 460; roll 900; count-up 1100; ping 1.8s; spin 900; indet 1.3s; tooltip delay 120.
- **Rises — two**: `halo-rise` 14px/600/stagger 60 for page/card entrances (Home, Settings, Health, onboarding, welcome, dialogs content); **stream-dense rise 3px / dur-base / ease-out** for in-stream blocks. Deviation, hard constraint: follow-scroll chases growth → 14px "lurched" (M6, `stream.css:25-26`, commit 22727d24).
- Page change: `halo-page` on content only, keyed on `workspace.activeId` (fires on every activation, kept-alive pages); outgoing page hidden instantly (Halo has no exit; `--dur-page-out:0`); kill `wsIn` + `.rising` + `SETTLE_MS` (3 stacked layers, `M01 §8.1`).
- Press `scale:.97` (individual property); hover lift 3px only on whole-clickable cards; nav hover = colour/bg only.
- Literal sweep: 36 raw `(.22,1,.36,1)`, ~104 bare `ease`, ~130 `ease-out` keywords, 70+ raw durations → tokens (`M09 §4.1`). 25 spinner keyframes → one `.halo-spin`. Svelte `transition:` directives get matching `css` fns + reactive reduced-motion (T13).
- Splash doors `(.65,0,.35,1)` → `ease-out` (Halo silent; vocabulary unification, cheap).
- Why: owner "proper motion… happy medium"; Halo "three speeds, one curve" (`M09 §3.1`). "Laggy" (M5) was the reveal pipeline, not menu durations; 22727d24 verdict still pending (`M12 §0.3`) → no recorded rejection of Halo timings.
- Overrides X33 (Rift-tuned 240 for chat/overlays) → expected disagreement.
- Reversibility: cheap (3 token values; each retune its own commit). OWNER-GATE: no.
- Unblocks: T11-T14, all overlay ports.

### T11 Streaming text reveal + scroll follow
- Halo silent (no per-word pattern); binding Halo rules: blur small + once at a time; continuous motion opacity/transform only.
- **DEFAULT**:
  - Per-word `md-word`: **opacity-only** 0→1, `dur-base 300`, `ease-out` (drop `blur(3px)`; ~18 concurrent blurred spans violates Halo rule, `M03 §8.4`). Inline spans can't transform → no rise.
  - Keep 22727d24 pacing numbers: `WORD_MS 30`, `CATCHUP_LAG ceil(320/30)=11`, `REVEAL_WINDOW = ceil(300/30)+1 = 11`.
  - **Correctness fixes (not taste, ship regardless)**: carry remainder (`last += due*WORD_MS`, not `last = t`) so cadence is ~33 w/s smooth not frame-quantised (`M03 §4.1, §8.2`); dt-normalise follow lerp `k = 1-(1-0.35)^(dt/16.667)` (60Hz-equivalent, frame-rate independent); stop lerp when `|delta|<1` (1px stall candidate `M03 §4.2`); jump-to-latest uses the same lerp engine (one writer, no native smooth); turn-rail active-tick reads rAF-throttled.
  - Keep: char pacer, 900px snap, 80px latch, instant pin on tab switch, `code-rise` killed (replays per delta) → chrome-hold opacity only.
  - Reduced motion: instant text, instant pin.
- **How the owner tunes it**: Claude Design canvas board "Rift · Halo / Motion lab": replays a canned 400-word turn with live sliders: unit (word / clause / line), fade ms, blur on/off, WORD_MS, catch-up lag, pacer on/off, follow (lerp k / instant). Owner plays, picks values; values transcribed into constants; confirmed once in `/dev/gallery` dev build (release is faster); one revertable commit per changed number (`M12 §3.3`, redesign-plan rule).
- Deviation: none. Reversibility: cheap per number. **OWNER-GATE: yes (felt-live; M5 most-rejected category).**
- Unblocks: T12, chat phase sign-off.

### T12 Turn lifecycle motion
- **live → settled** (today hard `{#if}` swap, `M09 §5.4`): render answer `say` groups from one keyed region present in both states so `Markdown` instances never remount (no snap of held words, no `blockIn` replay); live work groups collapse with Accordion grid-rows `1fr→0fr` dur-base + opacity; Ledger line enters stream-dense rise (3px, dur-base, 60ms delay). Gate implementation on the CDP recording (T0).
- **Block entrances**: stream-dense rise once, live turns only; settled/history hydration `animation:none` (`.sturn[data-hydrated]`) — Halo "enter once".
- **Disclosure**: Halo Accordion everywhere (grid-template-rows 0fr→1fr dur-base, content opacity + `translateY(-4px)` with 60ms delay, closed panel `inert`); replace 10 `transition:slide` + `workOpen`/`bodyFadeIn`/`gcReveal`/`flyout max-height`. Chevron: Halo chevron-down rotating 180° dur-base app-wide (replaces ChevronRight→90°).
- **WorkLine chip**: keep fit-content collapsed chip (deviation, recorded rejection D8 "full-width bars… tacky"); on open snap to full width (drop `interpolate-size` width morph), height via Accordion.
- Shell/term line stagger: cap `min(i,12)*60ms`→ Halo cap; diff rows no stagger (content).
- Reversibility: cheap-medium. OWNER-GATE: no. Unblocks: chat phase.

### T13 Loops + reduced motion
- Options: (a) Halo — loops only LiveDot/Spinner/indeterminate Progress; (b) (a) + `.shim` verb exception; (c) keep current loop set. Reduced motion: per-selector (today) / Halo global net / net + essential exemption. DEFAULT (a) + net + exemption.
- **Survive (real activity)**: `LiveDot` ping 1.8s (`::after` scale 1→3, opacity .7→0) for streaming/working/live states — replaces 20+ box-shadow pulses + 9-dot `workdots`; `Spinner` 900ms (one); indeterminate `Progress` 1.3s (browser load, digest, analyze); terminal block cursor (real tty), dictation caret, exit-plan drafting caret (`steps`); mic waveform + silence ring (real signal, K11); CtxRing/Progress value tweens.
- **Kill**: `.shim` text sweep on all 5 selectors (Halo loops = LiveDot/Spinner/Progress only; footer verb → `LiveDot` + static label) — overrides M16 approval → expected disagreement; enhance magic ×4 (shimmer, aura, stars, wand-pulse) → Spinner in wand button + "Enhancing…"; `ultra-pulse`, `drop-dot-pulse`, `up-pulse`, LimitBar pulse + sheen, `pill-breathe`, welcome mark/greeting breathes, splash caret blink, hub "Active" dot (not live → static accent Badge), AI-Health hot verdict pulse (→ static danger Badge), N breathing vitals dots (→ static, one LiveDot in footer), legacy rail glow/avatar halo, send ripple (→ Halo press + glow), `orb-breathe`, WebBrowser "Page read" shimmer (→ one-shot Badge rise).
- **Onboarding ObStage demos**: play once on step enter, hold final frame (Halo "never loop for attention"); reduced → static composed scene.
- **Reduced motion**: Halo global net verbatim in `app.css` (`*,*::before,*::after{animation-duration:.01ms!important;animation-iteration-count:1!important;transition-duration:.01ms!important;scroll-behavior:auto!important}`) + exemption `[data-essential]` for `Spinner` only (Halo's net freezes spinners — a Halo bug, keep state legible); LiveDot ping stops (dot + label remain). Make `prefersReducedMotion()` reactive (`matchMedia` change → `$state`); all Svelte transitions + JS tweens read it. Delete per-selector reduce blocks as the net lands.
- Reversibility: cheap. OWNER-GATE: no. Unblocks: T10 sweep, perf (box-shadow/background-position loops gone).

### T14 Skeleton
- Options: (a) Halo shimmer + handoff; (b) static bones + Halo handoff; (c) Rift idiom (idle cursor + timer + LiveDot, no bones).
- **DEFAULT**: Halo bones (`bg-200`, 12px, radius 6; variants text 100/82/64/90/72%, stat, list) **static — no `halo-shimmer`, no opacity breathe**. Keep Halo handoff exactly: bones `halo-skel-out` 360ms → opacity 0 + blur 4; content `halo-reveal` 600ms from opacity 0, blur 6, translateY 6. `aria-busy` while loading.
- Deviation: drop shimmer — recorded rejection M2 "that little loading blurry block… looks old school" (`DESIGN.md:227-231`).
- Replaces: `ws-load-shimmer`, `Skeleton.svelte` sweep, `skel-shimmer`, `mcp-shimmer`, `ah-skel-pulse`.
- Reversibility: cheap. OWNER-GATE: no. Unblocks: Home/Health/MCP/boot loading states.

### T15 Blur / backdrop-filter / glass
- Options: (a) zero backdrop-filter, opaque surfaces; (b) blur only on non-fixed small layers; (c) keep current glass. Unblocks: every popover/HUD/composer port; WebView2 garbage-rect class closed.
- **DEFAULT**: zero `backdrop-filter` anywhere (16 declarations: composer well, SettingsMenu, SlashMenu, EnhanceBar, ConversationList/ProjectSwitcher menus, HUDs, ToolChip, Boundary/ModelSwitch, Settings search results, onboarding footer, `.cm-fade`). No glass. Floating surfaces opaque: popovers/menus `bg-100` + `line-200` + `shadow-pop`; toasts/HUDs `bg-200`. Scrim opaque `rgba(4,5,7,.72)`.
- `filter:blur` allowed only: skeleton handoff (T14), splash mark settle (small, once). Word reveal blur removed (T11); EnhanceBar word blur → opacity-only; dead `placeholder-fade` deleted.
- Why: WebView2 mis-composites fixed backdrop layers (hard constraint, `app.css:609-611`) and Halo Product category has none (`M10 §5.2 #5`).
- Reversibility: cheap. OWNER-GATE: no.

### T16 Halo extras
- Options per extra: on everywhere / on in Halo-scoped places / off. Unblocks: Button, Card, Stat, NotificationBell ports; `AnimatedCount` retirement.
- `Card spotlight`: **off** (Halo opt-in "few hero cards"; also travelling light = M3/X5 risk). `Button magnetic`: off. `FocusGroup dim`: off.
- `glow-strong`: **on** under hovered primary only (Send, one CTA per view) — Halo core.
- Press `scale .97`: on all Buttons (incl. icon-only). Card lift 3px: on whole-clickable cards (project cards, resume tiles, starter cards); never stream tiles/rows.
- Numbers: count-up (`Tween` easeOutCubic 1100ms, from last value) for load-once Stats (Home, Health; on range change; not on 60s window tick); `RollingNumber` (per-digit 900ms, right-most first, delay `min(pos,8)*45ms`) for live-in-place values (tokens, elapsed, diff +N/−M, ctx %, usage %); keep monotonic token guard. Replaces `AnimatedCount`.
- Bell swing 900ms + ping once on count increase (Halo NotificationBell).
- Reversibility: cheap. OWNER-GATE: no.

### T17 Tooltips
- Options: revive Halo Tooltip / keep aria-only / per-surface visible labels.
- **DEFAULT revive**: `ui/tooltip` on bits-ui Tooltip (floating-ui collision — Halo's CSS-only bubble has none) with Halo look: inverted `ink` bubble, `bg-000` text, 500 12/16, pad 6/10, `radius-sm`, max-w 240, `z-tooltip`, show delay 120ms, hide immediate, rise 4px dur-fast, `aria-describedby`. Rewrite `actions/tooltip.ts` to mount it while keeping the aria-label promotion for icon-only buttons → all 174 call sites light up with zero call-site edits. Kbd chips only for bindings that exist. Native `title=` (browser, MCP) converge onto it. Suppressed inside `.wb-stage` bounds (never triggers the T0 lock). Numbers on Health/Home use `Stat peek` instead.
- Why: Halo pairs icon-only buttons with Tooltip (`M10 §4.1-12`); 174 sites + "hover to see" copy currently lead nowhere (`M07 §8.1`, `M06 §8.4`); the 2026-06-15 pull was against the old implementation's noise.
- Overrides 06-15 pull → expected disagreement.
- Reversibility: cheap (one action file). OWNER-GATE: no.
### T18 Syntax colour
- Options: (a) Halo monochrome everywhere (code, Read, diff, shell command, Markdown fences); (b) monochrome in stream, Shiki in Markdown fences; (c) muted chart-derived palette (Halo extension); (d) keep Shiki github-dark-dimmed.
- **DEFAULT (a)** implemented as a custom Shiki theme `rift-halo-ink` (tokens: code `ink`, keywords/names `ink`, strings `ink-muted`, comments `ink-faint`, punctuation `ink-muted`; no hue) — keeps tokenization + DOMPurify style allow-list unchanged, so reverting = swap one theme constant (`highlighter.svelte.ts:8`). Highlight lines = `accent-soft` row + accent number (Halo CodeBlock). Diffs per T3. Shell command head monochrome (kills kind-coloured `$`/`PS>` prompts, `M02 §8.5`). ANSI SGR → red→danger, yellow→warn, green/blue/cyan→accent, rest→ink-muted (output tone, not syntax). Copy "Copied" accent / "Copy failed" danger 1.8s (fixes silent copy failure `M03 §8.13`).
- Why: Halo CodeBlock "no syntax colouring: one signal colour" (`M10 §3.5`); ConsoleX shipped monochrome accent/danger diff, owner called the page "cleaner" (`M11 §5`). Trade: D3 "detail-first" (owner HOLD) pulls toward (c).
- Reversibility: cheap by construction (theme swap) but taste-critical across every code surface. **OWNER-GATE: yes (soft — default ships, owner reviews board).**
- Unblocks: stream tiles, Markdown code chrome, EditDiff, ReadResult.

### T19 Navigation
- **Tab strip**: none. Sidebar conversation rows remain the tab presentation; split panes keep 28→36px pane head. Deviation: recorded rejection L10 ("saw it live, wanted it gone"); the mock's `Tabs` strip would re-trigger it. Cheap to add later.
- **Sidebar**: keep Rift shell component (Tauri `position:fixed`, drag regions, `--rail-w` publisher — hard constraints, X46) but Halo anatomy: width 248 default (= Halo `--side-w`), resize handle kept (208–380; Halo silent); **collapse → Halo 72px icon rail** (labels fade dur-fast, collapse button `scaleX(-1)` dur-base, width + flex-basis dur-base ease-out) replacing collapse-to-nothing; **hover peek removed** (Halo has none; L2 history of peek being fragile). Conversations reachable collapsed via palette + Alt+1…9.
- **Nav**: replace 5-tile footer dock with Halo `NavItem` column under brand (44px rows, icon 18, active `bg-200` + accent icon + `halo-nav-in`, mono count): Home, Chat, AI Health, Diagnostics (now reachable — fixes `M01 §8.5`), Settings in footer group. Conversations below with `SectionLabel` (`CONVERSATIONS` + count), rows 36px (T6), active row = `bg-200` + accent marker (one sliding marker, ConsoleX `.plate` pattern, dur-base) instead of per-row `barPop`; row `rowIn` cascade only on first mount (stagger cap 12), not on "Show earlier"/remount.
- **Naming**: "Home" everywhere (tile, aria-label, registry title); Ctrl+1 = Home page; Ctrl+N stays new chat; fix unused/wrong registry `kbd` hints.
- **Top bar**: sticky, 52px (ConsoleX compact precedent vs Halo 64; owner "cleaner"), `border-bottom line-100`, drag region: left Breadcrumbs (project / chat title, `halo-crumb-in`), right SearchField-look trigger (opens palette, `Kbd` Ctrl+K), NotificationBell (moved from sidebar footer), window controls 40×36 (Rift-specific, one component shared with onboarding Titlebar). Deviation: height 52 vs Halo 64 — ConsoleX owner-driven compaction (commit `70e7e72`, header 76→52, "cleaner… no clutter").
- **Browser**: stays a dock inside Chat (not a page); toggle in Chat topbar trail.
- **Status bar**: Halo has none → keep as Rift-specific footer (S7 owner-proud) on Halo tokens: LiveDot, Badge, Progress sm, mono data; usage pills → Progress sm tones. Drag region kept.
- Reversibility: medium. OWNER-GATE: no. Unblocks: shell phase, `c.sh nav` labels (update in same commit).

### T20 Overlays
- Options: (a) Halo overlay family exactly (Dialog/Drawer/Popover/Menu/Toast/NotificationPanel/Banner) on bits-ui; (b) restyle current bespoke overlays only; (c) mixed. DEFAULT (a). Unblocks: P9, three scrim/panel recipes → one, z-literal kill, check:ui portal/role counts drop.
- **Dialog**: Halo `halo-zoom` (scale .94 + 8px, dur-base), leave scale .96 200ms; width min(480,100%), radius-lg, `bg-100`, `line-200`, `shadow-pop`, opaque scrim `halo-fade` dur-fast; focus trap/return via bits-ui. CloseConfirm, DeleteProject, Settings reset/repair (replace native `confirm()`), UpdateDialog (Rift `UpdateHero` inside), MCP Dialog.
- **Drawer** (right, 40px slide, dur-base, leave 200): NotificationPanel 400 (groups keep Rift age buckets), OutputBlock inspector width 760, project editor? no (stays inline Card, Halo silent on inline edit).
- **Popover / Menu / ContextMenu / Select**: `.halo-pop` surface, `halo-pop-in` dur-fast (`-up` variant for composer-anchored), items `halo-menu-item` 220ms / 18ms cap 12, leave 140ms; always portalled to body (composer `will-change` containing block, dock `overflow:hidden`). ProjectSwitcher, conversation row menu, FilePathMenu (point-anchored = ContextMenu layer), browser overflow menu.
- **CommandPalette**: Halo palette (top 12vh, min(560,100%), halo-zoom) extended in the Svelte port with Rift features (real group headings, sub text, row Kbd, footer legend); Rift fuzzy scorer + bindings kept.
- **Toast**: Halo stack exactly — bottom-right, collapsed offset 10px/scale −.05 (max .86), fan on hover/focus with timers paused (600ms floor), drag-right dismiss (90px or 24px@0.6px/ms), enter dur-base, leave 240, default 4000ms, errors sticky. Anchor bottom offset = status bar 32 + 16; when Chat active and window <1200 lift above composer (`composerH + 16`). Tones accent/info/warn/danger + Rift `neutral` (Halo lacks muted). Overrides 07-16 top-center choice → expected disagreement (cheap).
- **Banner**: UpdateBanner moves inside the content area above the page it concerns (Halo), rise dur-base, leave 200 — no full-window reflow (`M01 §8.3`). Stream alerts → Banner danger/info.
- **Native webview**: every overlay registers with the T0 overlay lock.
- Deviation: z-order (T8). Reversibility: medium. OWNER-GATE: no.

### T21 Chat anatomy
- Options: user = mock bubble / full-width band; tools = Rift dense tile on Halo tokens / Halo Card+CodeBlock / ConsoleX steps-rail only; history tree = port / freeze+delete. Unblocks: P5 chat phase, gallery replay baselines.
- **Ledger receipt**: keep IA exactly (owner pick 2026-10-19, S2): answer first, one `Divider` hairline, receipt row = outcome icon + title sans `small`, facts `.halo-mono` caption-size, Copy/Retry `Button ghost sm` (36) — matches Halo mock `Main.dc.html:84-91`. Hover-only actions get `:focus-within` reveal (exists).
- **User message**: Halo mock bubble — right-aligned, `bg-200`, radius-lg 16 (`16 16 4 16` tail per ConsoleX), pad 12/16, max 560px, body 15/22 `ink`, timestamp `caption ink-faint` below; no gradient/inset/shadow. Band (audit option C) not chosen.
- **Tool blocks**: Rift dense StreamTile (Halo has no row card, G1) on Halo surfaces: `bg-100`, `line-100` → `line-200` hover dur-base, radius-md 12, head 36 pad 8/12, text `small`, meta `.halo-mono caption`. State vocabulary (one signal each, kills C10 triple): live = `LiveDot` + label in head; failed = danger icon + `Badge danger` (no red border/edge); needs-you (ask/permission) = Halo `Banner tone=accent` recipe (8% tint, 32% border) — closest Halo idiom (G11). Pills → `Badge` (new=accent, edit/agent/poll=neutral, deleted=danger). Spinners/progress/chevrons/copy buttons → `ui/` parts.
- **Inside the expanded receipt**: grouped quiet rows (WorkLine) render as a ConsoleX-style hairline rail list (1px `line-100` left rule, rows 32px, 15px ink-muted icon, verb sans, target mono, time only on running row) (`M11 §3.2`); heavy blocks (shell, diff, ask, plan, agent) keep tiles.
- **Terminal**: CodeBlock `terminal` surface (`bg-000`, `line-100`, radius-md) + Rift OutputBlock body (tiers, head-tail fold, cursor, ANSI per T18).
- **HUDs**: floating `bg-200` + `line-200` + `shadow-pop` (popover-class layer), `halo-pop-in`, `Progress sm`, Spinner/LiveDot; IntersectionObserver logic untouched.
- **History tree**: port only user bubble + system rows (Boundary/ModelSwitch → Halo `Divider label`); legacy assistant branch frozen, flagged for deletion with Stream-view toggle (owner FYI). Shared parts (BlockHeader, OutputBlock, ReadResult, GrepResult, EditDiff) restyle both trees.
- Reversibility: cheap-medium (component CSS). OWNER-GATE: no.

### T22 Composer anatomy
- Options: (a) controls inside one card (mock, ConsoleX); (b) Rift v4 flat bar below the well; send = radius-md square / circle; Esc-to-stop = implement / delete hint. Unblocks: P6, FLIP height constants, `composerH`-based jump/toast offsets.
- **DEFAULT controls inside the card** (Halo mock `Main.dc.html:126-138`; ConsoleX shipped one-row composer card, owner "cleaner", `M11 §3.3`): card = Halo Textarea recipe, `bg-100`, `line-300`, radius-lg 16; focus → accent border + `shadow-focus` on the card (textarea `outline:0`); hover `bg-100` stays (deviation from `.halo-input:hover{bg-200}`: a typing surface the cursor rests on should not flicker shade). Textarea 15/22, `rows=1`, autogrow to 360 (Halo). Bottom toolbar row inside: perm Menu (ghost sm), attach, mic, wand | model trigger (ghost sm + effort `Badge`), CtxRing, Send. **Toolbar visible in hero too** (no layout shift on focus, `M04 §8.14`).
- **Send**: `Button primary sm` icon 36×36 radius-md, `glow-strong` on hover, press .97; Queue = same with list icon; **Stop** = Halo `danger` variant (transparent, danger text, `line-200`). Icon swap spring (flip). No ripple.
- **Esc-to-stop**: implement (tooltip already claims it, no handler, `M04 §7.20`): Esc with no open layer (enhance/undo/mention/settings/slash) while streaming → `assistant.stop(tabId)`; `Kbd Esc` in Send tooltip true.
- Popovers (Slash/Mention/Settings/Usage/Enhance): one `.halo-pop` recipe, `halo-pop-in-up`, portalled, opaque. SettingsMenu = Popover of Segmented (provider tabs) + Menu-row list + Toggle (Fast) + Rift DetentSlider (owner-specified spring behaviour K14, Halo Slider visuals). Keep textarea-driven virtual listbox + `data-composer-popover`.
- Drag-over = FileDrop `.is-over` look (solid accent border, 7% accent tint, `scale(1.01)`); copy "Drop files to attach". Chips = TagInput chip recipe (`halo-tag-in` spring). QueueRail rows FlipList 460 on reorder.
- Composer FLIP (hero↔docked) kept: 460ms (= Halo FLIP value) on `ease-out`; `will-change` only during the FLIP.
- Reversibility: medium (FLIP heights, `composerH`). OWNER-GATE: no.

### T23 Scrollbars
- Options: Halo thin / hidden everywhere / OS default. Unblocks: global reset block in `app.css`.
- **DEFAULT Halo thin**: 10px, no arrow buttons, transparent track, thumb `line-200` with 3px transparent border, hover `ink-faint`; sideways-only strips hide. Applies to chat `.stream` (today hidden) and every scroller.
- Evidence: ConsoleX hid them Oct 10 ("hates visible/white scrollbars"), Halo v2.3 restored thin ones Oct 14 with no recorded objection (`M11 §4.5`). Halo thumb is dark, not white.
- Reversibility: cheap. OWNER-GATE: no.

### T24 Data viz
- Options: Halo Stat/BarChart/LineChart/Progress/Avatar + chart-1..4 / Rift dense bars + multi-hue / hybrid. Unblocks: P7 Home + AI Health, LimitBar consumers (UsagePanel, StatusBar).
- **Stat**: label `small ink-muted` → value `metric` 30/34 mono → detail; delta chip arrow + sign (`↑ 12.4%` accent-soft / `↓` warn-soft) with `goodWhen` (T3); Rift formatter fn (compact `32k`, `$8.2k`, `7.2s`) because Halo hard-codes en-US `$`; `peek` for exact values. Kill 38px gradient hero numeral (D7).
- **Series palette**: `chart-1 accent #6aa8ff`, `chart-2 #b79cff` (violet), `chart-3 #9cc6ff` (info override), `chart-4 warn`; no lime/green. Model-mix: Rift `ShareBar` (Halo gap) top 3 models in chart-1..3 + neutral `bg-300` "Other".
- **Daily chart**: 7d → Halo `BarChart` 7 bars (peak accent, rest `bg-300`, grow-in dur-slow `i*40ms`, sibling dim .45); 30d / All → Halo `LineChart` (draw-in 1.2s, area accent-soft, gliding tooltip incl. sessions; All = weekly buckets). Replaces filter-brightness hovers.
- **Health**: spend/day `BarChart`; latency `Sparkline` (chart-4 when rising); plan limits `Progress` (tones accent/warn/danger, eases 700ms, mono %); LimitBar → `Progress` (3 variants collapse to default/sm).
- **Per-project hues**: Halo `Avatar` (initials, 22% tint of chart tone by name hash mod 4) replacing 360-hue `projectHue` in switcher/cards/tags; active project = accent `Badge`. Overrides C15 (pending) — Halo has its own answer.
- Reversibility: cheap-medium. OWNER-GATE: no.

### T25 Process
- **Canvas-first** (owner designs in Claude Design canvas, not a dev server): project "Rift · Halo" with Halo DS installed. Boards: 0 Foundations (type, colour, radius, density tiers), 1 Motion lab (T11), 2 Shell + nav (flat vs islands), 3 Chat live turn, 4 Chat settled + receipt + tiles + code/diff/terminal (T18 A/B), 5 Composer hero/docked/states/popovers, 6 Home, 7 Settings, 8 AI Health, 9 Diagnostics + Browser, 10 Overlays (dialog/drawer/menu/toast/tooltip/palette). Built as plain markup with Halo tokens (ConsoleX precedent). Owner plays each in Play mode → sign-off per board → port. Canvas board N+1 drafted while porting N.
- **Branch**: `design/halo` cut from `design/chat-ledger` HEAD `22727d24` (Ledger `8bb71f4f` rides along). No push/merge without owner.
- **Phases**: P0 prerequisites (T0: floatingMotion check, CDP baselines, end-of-turn recording, overlay lock IPC, stale docs) · P1 tokens + fonts (T8, T5, check:tokens rewrite) · P2 `ui/` Halo primitives in `/dev/ui` · P3 motion globals (T10, T13 net) · P4 shell + nav · P5 chat stream + Ledger (+T11 correctness fixes) · P6 composer · P7 Home / Health / Diagnostics / Browser · P8 Settings · P9 overlays, toasts, notifications · P10 cleanup (dead CSS, dialect retirement, `check:ui --update`, DESIGN.md rewrite).
- **Verification per phase** (no DOM tests): `npm run verify:frontend` (CI watches `main` only); `node design-system/check-ui.mjs --update` when counts drop; `c.sh baseline` before / `c.sh diff` + `c.sh measure` probes after at fixed viewport (`reset-viewport`); `/dev/ui` per density; `/dev/gallery` replay for stream; `c.sh inspect` 0 console errors; reduced-motion via CDP emulation. Rust change (overlay lock, bg colour) verified on Windows only. No paid turn without asking (Q16).
- **Commits**: one per phase sub-step; every motion number its own revertable commit.
- **Stale docs**: P0 updates `docs/HANDOFF.md:26-36` and `redesign-plan.md:178-216` pacing to 22727d24 values; `docs/design/PREFERENCES.md` gone → DESIGN.md §8 is the log.
- Reversibility: n/a. OWNER-GATE: no (board sign-offs are the gates above).

### T26 Logo / splash / brand
- Options: keep mark + seam splash retuned / Halo-style fade-rise splash / Halo Logo. Unblocks: boot choreography constants, `app.html` first paint.
- **DEFAULT**: keep Rift mark PNG (`RiftLogo`) — Halo `Logo` is Halo's own mark and says nothing about replacing another product's (`M12 X47`). Keep seam-parting splash (Halo silent; brand moment, real boot stages S13). Retune: doors curve → `ease-out`; letter stagger 55 → 60; mark settle blur 8→0 kept (small, once); **drop** caret blink (loop) and exit blur 3. Wordmark Geist 600 −0.03em (Lexend retires from brand). Splash panel colour → `bg-000` flat (T1). App-chrome `.rift-seam` removed with T1; seam lives only in splash.
- Reversibility: cheap. OWNER-GATE: no.

### T27 Page structure (added)
- Options: shared Halo-styled PageHeader + SectionLabels / per-page headers. Reversibility: cheap. OWNER-GATE: no. Unblocks: P7/P8 page layouts.
- One shared `PageHeader` (re-skinned `PageHero`): `title` 32/38 (or `subtitle` 20 for dense pages) + sentence-case lede; numbered `SectionLabel` (`01 / PROJECTS`, mono 11 caps .12em, **index in accent** per Halo, hairline, count) on Home/Settings/Health — not in chat. One column edge per page (fixes "four left edges", `M05 §8.9`, `M06 §2`): header and body share `max-width` + padding.
- Halo "fit one screen": Home + AI Health fit 1600×1000 with lists scrolling inside cards; Settings + chat scroll in their own region.
- Card = Halo Card (pad 24, radius-lg, no shadow); one nesting level (card → content; previews are `bg-000` wells without fake window dots).

### T28 Settings rows (added)
- Options: Rift SettingsRow on Halo tokens / Halo stacked Field layout. Reversibility: medium. OWNER-GATE: no. Unblocks: P8.
- Rift-specific `SettingsRow` (Halo gap: no label+description+trailing-control row; vertical Field stacking doubles page height): title `small 500 ink` + description `caption ink-muted` left, control right (Toggle 44×26 / Segmented / Select / Button sm), hairline between rows. Booleans = Toggle (apply-instantly, Halo rule). Segmented keeps `role=radio` + roving (Rift `onRadioKey`) with Halo sliding thumb visuals (semantics deviation: keyboard/SR parity with existing behaviour; cheap). `<details>` → Accordion cards; jumpTo opens the item. Accent card removed (T2). One primary per view.

### T29 Focus + a11y floor (added)
- Options: Halo floor everywhere / per-surface. Reversibility: cheap. OWNER-GATE: no. Unblocks: every port's acceptance criteria.
- `shadow-focus` (2px gap + 2px accent) on every control via `:focus-visible` (fixes AiHealth/Diagnostics/Settings missing rings, invisible hue-slider focus). Hover-only actions also reveal on `:focus-within`. Status never colour-only (icon/word). `ink-faint` only for labels/ticks/timestamps, never on `bg-300`. Disabled = `opacity .45` everywhere. Modals trap/return focus (bits-ui); onboarding `aria-modal` gets real trap or drops the role. Real `<button>` only (fix span-in-button `BlockHeader:74-84`).

### T30 Content + icons (added)
- Options: Halo content rules / leave copy. Reversibility: cheap. OWNER-GATE: no. Unblocks: copy sweep in P10.
- Halo content rules: sentence case, no emoji (`📋` `Markdown:482`), no `✓ ℹ ⚠` text glyphs → lucide icons stroke 1.8 at 16–18px; empty states = what appears + one action; errors say what happened + what to do. "Experimental" + Title Case hero copy on Health → sentence case.

---

## Expected sharpest disagreements (with a Rift-preserving lens)
1. T10 durations: Halo 150/300/600 globally vs Rift-tuned 140/240/460 for chat/overlays ("laggy").
2. T6 prose 15px + 44px page controls + `regular` default vs compact 13px default (D1).
3. T18 monochrome code vs Shiki colour (D3 detail-first).
4. T13 killing `.shim` live-verb sweep (M16 owner-approved).
5. T17 reviving tooltips (pulled 2026-06-15).
6. T1 flat frame vs onyx slab / islands (L6, C6).
7. T2 deleting hue/vividness dials (C17, persisted prefs).
8. T20 toast bottom-right vs top-center on island (07-16).
9. T19 72px icon rail replacing collapse + hover peek (S4).
10. T5 mono for receipt numbers vs tabular Inter (C13 / Ledger T4).

