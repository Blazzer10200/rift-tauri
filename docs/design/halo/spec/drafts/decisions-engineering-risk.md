---
title: Halo port decision register, lens ENGINEERING RISK + SEQUENCING
status: draft (one of several lens drafts; synthesis merges them)
branch: design/chat-ledger @ 22727d24 (Ledger 8bb71f4f below it, both unpushed)
updated: 2026-10-19
---

# Decision register, engineering risk + sequencing lens

Inputs: maps 01-12 (`docs/design/halo/map/`). Cites = `map NN §S`. Owner quotes only from map 12 / task CONTEXT.
Verified myself this session (not just map claims):
- `ui/recipes.ts:20,58,60` animate on `data-open:`/`data-closed:`; only `MessageBubble.svelte:474` sets a `data-open` attr in `src/`. Tailwind v4 `data-open:` = `[data-open]` presence; bits-ui sets `data-state` (map 10 §5.2 #2) => primitive enter/exit motion probably never fires. Verify in P0.
- `app.css:281-282` `--z-dialog 4000` / `--z-popover 9000`; `app.css:244` `--dur-page-out 140ms` drives `--rail-w` tween (`:353`) vs rail width 240ms (map 01 §4) = confirmed desync.
- `node_modules/@fontsource-variable/` = inter, jetbrains-mono, lexend only (Geist not installed).
- `docs/design/halo/spec/` did not exist before this file. `docs/design/` is gitignored (untracked by design).

How to read: X0 = sequencing spine (this lens's main output). T1-T26 = task topics. X1-X6 = cross-cutting topics the maps raise that T1-T26 miss. Per topic: Options · DEFAULT · Why · Reversibility · OWNER-GATE · Unblocks. "Proceed on default" = work does not wait for the owner.

Guiding rule for the whole port: **make every taste value cheap to change later.** That means tokens for every size/time/colour, ratchets that stop new literals, one home per motion pattern. If that holds, most taste calls stop being gates and become felt tweaks after the fact.

---

### X0 Sequencing spine (phases, gates, what each unblocks)

| Phase | Scope | Visible change | Gate to merge (local) | Reversible by |
|---|---|---|---|---|
| P0 Baseline + spikes | `c.sh baseline` on `tour home chat settings ai-health` + `/dev/ui` + `/dev/gallery` at fixed viewport (audit used 2048x1104 @ DPR 1.25, map 08 §7.4); `c.sh measure` 8 probes; spikes X1 | none | n/a (no code) or spike notes | n/a |
| P1 Tokens + fonts + theme | vendor Halo `tokens.css`; alias layer in `app.css`; `data-theme="blue"`; Geist via fontsource; `--info` override; z ladder; native colours (X4); undefined vars (X5); `check:tokens` repointed; token tests | whole app re-colours + re-fonts, no layout/anatomy change | `npm run check` + `npm test` green, `c.sh diff` per surface reviewed, `inspect` 0 console errors | revert one commit (alias layer = one file + 2 imports) |
| P2 Motion + perf foundation | `styles/motion.css` (one home for keyframes), global reduced-motion net + essential exemption + reactive probe, 1 Spinner / 1 LiveDot / 2 rises, kill list (map 09 §4.2), zero `backdrop-filter`, layout-prop -> transform sweep, entrance-replay gate (X2) | calmer, fewer loops; motion values = tokens | check + test + gallery replay + reduced-motion emulation (CDP `Emulation.setEmulatedMedia` via `eval`) | per-commit revert (one pattern per commit) |
| P3 Primitives | restyle `ui/recipes.ts`; add bits-ui wrappers under `components/ui/` (tooltip, popover, dialog, tabs, switch, checkbox, radio-group, accordion, progress); CSS-only atoms (Badge, Kbd, LiveDot, Spinner, SectionLabel, EmptyState, Banner, Card, DenseTile, Skeleton); `/dev/ui` extended | primitives in `/dev/ui` only (consumers still old) | check (incl. check:ui class-generation), `/dev/ui` shots vs canvas | additive, unused until P4 |
| P4a..f Surfaces | 4a shell · 4b chat stream + pane · 4c composer · 4d home + onboarding · 4e settings · 4f health/diag/dialogs/browser chrome. Each: canvas board -> owner plays (when gated) -> port -> baseline/diff -> local merge | per surface | check + test + `c.sh tour/diff/measure` + 0 console errors + reduced-motion pass | per-surface branch |
| P5 Cleanup | drop Inter/JetBrains deps; `check:ui --update` lock-ins; DESIGN.md final; approved deletions | none | full `npm run verify` on Windows (Rust touched in P1/P4f) | n/a |

Order rationale: P1 before anything because 17 colliding token names silently retune the app (map 08 §1.3); P2 before surfaces because every surface would otherwise re-invent spinners/pulses/rises (25 spinner keyframes, 9 rise keyframes today, map 09 §4.1); P3 before surfaces because hand-rolled overlays raise `check:ui` ratchets and fail the gate (map 04 §9). Surface order: shell first (frame decision T1 colours everything), chat next (owner's main complaint), composer after chat (FLIP contract with `AssistantPane`), then dashboards (least risk, most Halo-native).

Owner-free path while away: P0-P3 + 4d/4e/4f fully on defaults. 4a/4b/4c proceed on defaults but their merge waits on the gated calls (T1, T11 pacing, T22), each isolated to 1-3 files so a later flip is cheap.

---

### T1 Material / frame

- Options: (a) Halo flat docked frame (sidebar border-right, sticky topbar, no gutters); (b) keep island STRUCTURE (12px gutters, radius 16) with flat Halo fills (canvas bg-000, islands bg-100, hairline line-100), drop gradient/grain/cavity/bevel, seam as 1px line; (c) keep onyx slab as is, recoloured; (d) hybrid per page.
- DEFAULT: (b). Canvas `bg-000`, islands `bg-100` + 1px `line-100`, radius 16 (`radius-lg`), gutters 12. Delete: 2 vertical gradients, 4-layer shadow, 4 SVG-noise layers, cavity inset shadows, `.app` 1s bg transition. Keep `.rift-seam` as a static 1px `line-100` (no gradient). Keep contact AO only if it reads (one `0 1px 0` shadow max).
- Why: CSS-only change in 2 files (`AppShell.svelte:339-512`, `Sidebar.svelte:255-284`), no markup/z/drag-region churn; drops the costly full-window layers (70px inset shadow, tiled noise; map 01 §9 Perf); kills the duplicated "lockstep" recipe (map 01 §8.7). (a) means restructuring the shell DOM, `--rail-w`, toast anchoring, Topbar drag regions, all at once. Owner built the chassis over 5 sessions (map 12 L6, S4) and Halo says "no colour washes" (map 12 X2): (b) satisfies both literally. Grain existed only to kill banding of gradients (map 12 X2) so it goes with them.
- Reversibility: (b)->(a) later = shell CSS + Topbar markup, ~1 day, isolated. (b)->(c) = git revert of 2 files.
- OWNER-GATE: yes, non-blocking. Canvas board shows (a) vs (b) side by side (rule C6 "show beside before committing", map 12 Q1). P4a builds (b); merge of P4a waits for the verdict.
- Unblocks: P4a shell, toast anchor (T20), page entrance target element (T10), first-paint colours (X4).

### T2 Accent model + saved prefs

- Options: (a) keep hue/vividness dials, default re-anchored to Electric blue; (b) fixed Halo presets only, delete dials; (c) presets picker (4 Halo accents) + "custom" dial as advanced.
- DEFAULT: (a) with an exact default: when no custom accent is stored, `--accent: #6aa8ff` literally (owner: "Use it as is"); the oklch dial derivation (`app.css:171-180`) applies only under `[data-accent="custom"]`. All accent derivatives via `color-mix(in srgb, var(--accent) N%, transparent)` (`accent-soft` 14%, `glow-strong` 28%, ring), never rgb triplets (CX light-theme blocker, map 11 §1.2). Redefine `--shadow-focus: 0 0 0 2px var(--bg-100), 0 0 0 4px var(--accent)` (Halo's is hex-hard-coded, map 10 §1.1). Migration: bump keys `rift.ui.accent.v1`/`vividness.v1` -> `.v2`; v1 ignored (owner chose blue globally). Settings swatch row: first swatch = Electric blue (exact), rest unchanged for now. 13 direct `--accent-h` literal consumers (map 08 §4.1) -> `color-mix` of `--accent` in P1.
- Why: zero consumer edits for 617 `var(--accent)` refs; dials are owner-built (map 12 C17, S9); deleting them is a feature removal not a restyle. Exact hex honours "as is"; oklch derivation cannot hit #6aa8ff exactly.
- Reversibility: cheap (one token block + storage key).
- OWNER-GATE: no (flag: dial UI fate is a later taste call; deleting it would need a yes).
- Unblocks: P1, Settings Appearance card (4e), onboarding step 1 swatches (4d).

### T3 Semantic colours on blue

- Options for each: follow Halo literally / CX precedent / keep Rift.
- DEFAULT:
  - `--info` = `#9cc6ff`, `--info-soft` = `color-mix(#9cc6ff 14%)`, declared after tokens.css (`:root[data-theme="blue"]` specificity, CX `app.css:62-67`). Verify `--chart-2` follows (map 11 Risks: blue block incomplete); if not, set `--chart-2` explicitly.
  - ok/success: NO green. Keep the Rift NAME `--ok`/`--ok-soft` (101 ext refs) aliased to `--accent`/`--accent-soft`. One-line flip back if the owner wants green. Evidence: owner's newest colour statement is "no green anywhere" (ConsoleX, 2026-10-17, map 11 §5) + Halo "positive = accent" (map 10 §4.4). Status always carries icon/word (Halo rule) so blue success stays legible.
  - `--status-busy #22c55e` -> `--accent` (live = accent, LiveDot). `--status-pending` -> `--info`. `--status-warn` -> `--warn`.
  - diff add = `accent-soft` line wash + accent `+` sign; del = `danger-soft` + danger `-`. Word emphasis classes `dem-add/dem-del` unchanged (test-coupled, map 08 §6).
  - warn vs cost: one `--warn` = Halo `#ff9f5a` for caution AND cost-money (Rift rule "amber = costs money", map 12 C3); same hue family, keep the at-control disclosure.
  - Plan-mode tone (was info) = `--info` light blue; Auto-edit tone (was ok) = accent.
  - `--dc-*` private palette in DiagnosticsConsole + off-palette orange -> status tokens (map 07 §8.8).
- Why: lime info on blue breaks log levels, CLI banners, plan mode, notification default tone (maps 01, 04, 07). Aliasing keeps the change one line per meaning.
- Reversibility: cheap (alias lines). Flag: "Use it as is" literally includes lime info; override is CX precedent.
- OWNER-GATE: no (flag both: info override, no-green).
- Unblocks: P1, every status surface.

### T4 Light themes

- Options: in scope now / later wave / never.
- DEFAULT: out of scope. Vendor `tokens.css` whole (light blocks harmless, unused); keep `color-scheme: dark`; no theme switch UI.
- Why: 131 oklch literals, 93 gradients, black-based shadows, Shiki dark theme, warm-tint, `#000` masks all assume dark (map 08 R9); Rift rule "dark only" (map 12 C13). P1 colour-mix discipline (T2) keeps the door open.
- Reversibility: n/a (additive later).
- OWNER-GATE: no.
- Unblocks: removes ~30% of P1 scope risk.

### T5 Fonts

- Options: Geist + Geist Mono everywhere; Geist UI + keep JetBrains for code; keep Inter/JetBrains on Halo scale.
- DEFAULT: `@fontsource-variable/geist` + `geist-mono` (CX uses ^5.3.0, map 08 §3) imported in `+layout.svelte` before `app.css`; ALSO import the italic css (`wght-italic.css`) IF present (map 10 §6 says it exists; Geist historically shipped no italics; X1 spike) so markdown emphasis is not synthesized. `--font-ui`/`--font-sans` = `"Geist Variable", ui-sans-serif, system-ui, "Segoe UI", sans-serif`; `--font-mono` = `"Geist Mono Variable", ui-monospace, "Cascadia Mono", monospace` (fontsource family names, not Halo's "Geist"). Drop Inter `font-feature-settings` (`app.css:306`) and body `-0.005em` (`:314`, `AppShell:358`). Geist Mono also for code (one mono face); `--code-liga` pref stays wired (may no-op; verify ligature support in P0). Lexend stays: dyslexia mode (`app.css:747-756`) + splash wordmark only; greeting moves to Geist. Remove Inter/JetBrains deps in P5, not P1 (cheap rollback window).
- Why: CSP `font-src 'self' data:` blocks Halo's Google import (`tauri.conf.json:28`); offline-first. Two imports = trivial revert.
- Risks: reflow of 34px rows, 10px tiles, status bar truncation (map 08 R5): P1 includes `c.sh measure` text widths on the 8 probes + tour truncation check. Dyslexia override targets `body, .app, textarea, input, button, select` (map 06 §9) and must still win over any Halo `font:` shorthand: keep it last in cascade.
- Reversibility: cheap (imports + 2 vars) until P5.
- OWNER-GATE: no (owner chose Halo; Geist is Halo's face).
- Unblocks: P1, every text measurement.

### T6 Type scale + density

- Options: Halo scale unchanged (15 body, 44 controls); Rift compact unchanged; a "Halo compact" scale with density tiers.
- DEFAULT: Halo-compact scale as tokens, density modes survive (owner-built, map 12 D2), compact stays default (map 12 D1 "denser and quieter").
  - Type tokens (new `--t-*` family; Halo has no type vars, map 10 §1.1): label 11/14 mono 500 caps .12em (SectionLabel, Kbd only) · caption 12/16 (meta, timestamps, badges) · small 13/18 (UI default, menu rows, descriptions) · heading 15/20 600 · subtitle 18/24 600 · title 24/30 600 -0.02em · metric 26/30 mono 600 tabular. Display unused.
  - Chat prose: compact 14/22, regular 15/22 (Halo body), comfy 16/24; column `--chat-col` 760px shared by messages, composer, alerts, turn-rail and jump-pill offsets (fixes 844/880/1100 mismatch, map 03 §2).
  - Controls: compact 30 / regular 36 (Halo sm) / comfy 44 (Halo md). Menu/list rows 30/36/40. Sidebar conversation rows 32/36/44. Stream tile head 30 compact.
  - Text floor: 12px sans; 11px only for mono label/kbd. Icon-button hit floor 24px (WCAG 2.5.8 AA) even in dense rows.
  - Enforce: add `font-size:Npx` literal ratchet to `check-ui.mjs` (baseline = today's 380, map 08 §1.5) so no surface port adds new literals; extend scan to `src/lib/styles/*.css` for this metric.
- Why: Halo 44/15 at a 900x600 min window wrecks the dense desktop tool (map 08 R6, map 06 §9); 261 sub-12px declarations must move anyway. Tokens + ratchet make the exact numbers a cheap later tweak.
- Reversibility: cheap IF the ratchet holds (values in one block). Costly without it.
- OWNER-GATE: no, flag as felt-tune (owner sees the scale on the first canvas board; tweak tokens, not surfaces).
- Unblocks: P3 primitives sizing, every surface.

### T7 Radius ladder, pills, control heights

- Options: Halo 8/12/16/pill as is; keep Rift 4-16 ladder; Halo + one small step.
- DEFAULT: `--radius-sm 8 / --radius-md 12 / --radius-lg 16 / --radius-pill 999` + Rift extension `--radius-xs 6` (kbd, inline code, checkbox: Halo uses 6 there, map 10 §1.3). Mapping: controls <= 30px tall -> 8 (12 on a 26-30px button reads pill-like); inputs/popups/menus/DenseTile -> 12; cards, islands, dialogs, drawers, palette -> 16 (island 14 -> 16); concentric rule kept (16 panel / 4 pad -> 12 rows). Pill only for badges, status/count chips, toggles, `.jump-latest`. Send button, `.chip-btn`, `.ps-chip`, `.char-count` stop being pills (DESIGN "no pill buttons" = Halo, map 04 §8.13).
- Collision handling: Halo `--radius-lg` 16 vs Rift `--radius-lg` 10 (15 ext refs that meant "medium"): rename those 15 sites to `--radius-md` in P1 (one commit, visual diff), let Halo 16 win. `--radius-sm` 6->8 retunes Tailwind `rounded-sm` in `ui/` (intended).
- Why: 13+ resolved radii per surface today (maps 05, 06); 3 steps is a big entropy cut with little risk.
- Reversibility: cheap (tokens).
- OWNER-GATE: no.
- Unblocks: P1/P3.

### T8 Token architecture + gates + design-system/

- Options: (a) ConsoleX alias layer (Halo first, Rift names aliased); (b) rename consumers to Halo names; (c) alias now, migrate to Halo names per surface later.
- DEFAULT: (a), permanent like CX, with an explicit collision policy:
  - Load order (contract): fontsource -> `src/lib/styles/halo/tokens.css` (vendored, generated, never hand-edited) -> `app.css`. `:root` and `[data-theme]` are equal specificity (0,1,0); source order decides; Rift overrides needing to beat `[data-theme="blue"]` use `:root[data-theme="blue"]`.
  - 17 collided names (map 08 §1.3): Halo value wins for `--accent*`(default path), `--warn*`, `--danger*`, `--scrim`, `--radius-sm`, `--radius-lg` (after the 15-site rename, T7), `--dur-fast`, `--dur-slow`, `--stagger`, `--ease-spring` (identical). Rift override wins for `--info*` (T3), `--dur-base` (T10), `--font-mono` (fontsource family name). Each override is a commented line in one "Rift deviations" block.
  - Rift-only names alias to Halo (`--bg: var(--bg-000)`, `--surface: var(--bg-100)`, `--fg: var(--ink)`, `--fg-2/--fg-muted: var(--ink-muted)`, `--fg-subtle/--fg-faint: var(--ink-faint)`, `--border: var(--line-100)`, `--border-strong: var(--line-200)`, `--field-border: var(--line-300)`, `--bg-inset: var(--bg-000)` (CX mapped wells to 000), `--ring` -> focus ring, `--shadow-lg/--shadow-float: var(--shadow-pop)`, `--ease-page/--ease-soft: var(--ease-out)`, `--dur-rise: var(--dur-slow)`).
  - Keep every Tailwind `@theme inline` bridge KEY name; only right-hand `var()`s change (keys are what `ui/*`, `recipes.ts`, `utils.ts:13-17`, `utils.test.ts`, `check:ui` speak, map 08 §5). Any NEW token consumed as a Tailwind class inside `components/ui/` (`--t-*` type, `--radius-xs`, z names) needs an `@theme inline` entry + `cn` registration in `utils.ts:13-17`, or `check:ui` hard-fails on class generation.
  - z ladder (Halo names where meaning matches; deviation documented): raised 1 · sticky 10 · hud 20 · overlay (dialogs, palette) 50 · toast 60 · dropdown/popover 65 (menus inside dialogs must work, DESIGN §10) · tooltip 70 · splash 100. Two deliberate changes: (1) toasts now sit ABOVE dialogs, reversing DESIGN §10 "dialogs above every surface incl. toasts", so non-modal feedback ("Copied", "Saved") stays visible while a dialog is up (Halo order); (2) Halo's `--z-dropdown` value is redefined 20 -> 65 (listed in the deviations block). Rift `--z-dialog`/`--z-popover` alias to overlay/dropdown. Tokenise the 25 raw z literals in P1 (sweep).
  - `check:tokens`: it parses the FIRST `:root` of `app.css` and cannot resolve `var(--bg-000)` (map 08 §5) -> mass MISMATCH. In P1 change the script (not delete) to load vendored `tokens.css` as the resolution source and regenerate `design-system/styles.css` `:root` + `_ds_manifest.json` tokens from the resolved alias layer. Never green it by hand-editing the stale mirror.
  - `check:ui`: unchanged contract; P1 lowers oklch/hex counts -> `--update` after each phase. Add metrics: `backdrop-filter` (target 0, T15), `font-size:Npx` (T6), scan `.css` for both.
  - New vitest (node, cheap): parse `app.css` alias block -> every Rift token resolves to a Halo var or a declared deviation; no raw hex outside the deviation block; z ladder strictly increasing in the documented order.
  - `design-system/` cards (15 HTML) + Rift Style DS artifact: leave untouched in P1 (deletion/publish need a yes). DESIGN.md: rewrite as "Halo for Rift" (deviations + preserved rejection log §8) in P1 because it's gate-adjacent.
- Why: 4,806 external `var()` refs (map 08 §0.1) => (b) touches ~63 files for zero visual gain and fails `check:ui`/`utils.test.ts` if bridge keys move. (a) is proven (CX).
- Reversibility: cheap (one file + one import). Collision policy is the risk; the token test guards it.
- OWNER-GATE: no (deleting `design-system/` cards / artifact supersede = deletion approval, see owner_gates).
- Unblocks: everything.

### T9 Component strategy

- Options: (a) restyle `components/ui` + `recipes.ts`, add missing bits-ui wrappers there; (b) new `src/lib/components/halo` kit hand-ported; (c) both.
- DEFAULT: (a). bits-ui-backed parts (Menu, ContextMenu, Select, Combobox, Dialog, Drawer, Popover, Tooltip, Accordion, Tabs, Switch, Checkbox, RadioGroup, Progress, Slider) = `components/ui/<name>` via `npx shadcn-svelte@latest add <name>` (NEVER `init`, it rewrites `app.css`, DESIGN §10) then Halo look in `recipes.ts`. CSS-only atoms (Badge, Kbd, LiveDot, Spinner, SectionLabel, EmptyState, Banner, Card, DenseTile, Skeleton, IconClose) = small `components/ui/<name>.svelte` with scoped CSS on tokens. Behaviour parts (Toast stack, RollingNumber, count-up, FlipList, sliding indicator, SplitPane bar) = runes classes / actions in `src/lib/motion/` + `src/lib/actions/`. Rift-specific parts live with their surface: `assistant/stream/` (DenseTile usage, DiffBlock, LogStream-like terminal body), `assistant/` (ChatColumn, UserMessage, TurnRail, JumpToLatest), `shared/` (PageHeader, SettingsRow, PreviewWell, StatTile, ShareBar, SecretField).
- Fix first in P3: `recipes.ts` `data-open:`/`data-closed:` variants -> `data-[state=open]:`/`data-[state=closed]:` (or `[data-starting-style]`/`[data-ending-style]`, map 10 §5.2) after the P0 spike confirms they are dead today.
- Why: `check:ui` exempts `components/ui/` from portal/role ratchets and demands its classes generate CSS (map 08 §5); a parallel halo kit would be the 5th button dialect (`.btn`, `.st-*`, `.ob-*`, `.sxa-btn`, `ui/button`, map 08 §2.3) and re-implement keyboard/focus that bits-ui already gives. Halo's React bundle can't mount (CX decision, map 11 §1.1).
- Reversibility: medium (primitives are shared; per-primitive commits keep it cheap).
- OWNER-GATE: no.
- Unblocks: P3, all overlay work (T20), Settings controls (4e).

### T10 Motion durations, easings, rise distances

- Options: (a) Halo values globally; (b) Rift-tuned values under Halo names; (c) two scales (chat vs pages).
- DEFAULT: (b), one scale. `--dur-fast 150` · `--dur-base 240` (Halo 300 = felt toggle, see below) · `--dur-slow 600` · `--dur-page 520` + 60ms delay (enter only; leave = opacity 140ms on the outgoing stacked page) · `--dur-leave 140` (popups) / `200` (modal, drawer, banner) · `--stagger 60`, cap 12 · `--ease-out cubic-bezier(.2,.8,.2,1)` with `--ease-page`/`--ease-soft` aliased to it · `--ease-spring` unchanged, scoped to flip/grab (toggle knob, checkbox/radio pop, slider thumb, detent effort knob, tag chip, badge pop). Rise: `--rise-page 10px` (page/card/dialog content, = Halo page-in) and `--rise-dense 3px` (in-stream blocks, owner-tuned, `stream.css:25-26`). Loop periods: ping 1.8s, spin 900ms, indeterminate 1.3s.
- Why: one line retimes ~570 token sites (map 09 §4.1); `--dur-base` 240->300 alone slows 111 sites by 25% in an app the owner just called "laggy" (map 12 M5). Halo's 14px rise was already rejected in the stream as "lurching" (map 02 §10 Q1). The bulk of the work is the ~150 literals + ~104 bare `ease` keywords: P2 sweep, then a lint (ratchet `cubic-bezier(` literal count in check-ui).
- Reversibility: cheap (tokens) once the literal sweep is done.
- OWNER-GATE: no; flag base 240 vs 300 as a single felt toggle in the motion lab (T11).
- Unblocks: P2.

### T11 Streaming reveal + scroll follow

Split into STRUCTURE (engineering, ship on default) and PACING (taste, felt, one value per commit).

- Structure (default, P4b, no owner needed):
  1. Extract the reveal cursor + follow math from `Markdown.svelte:216-252` and `AssistantPane.svelte:169-191` into pure `src/lib/motion/reveal.ts` / `follow.ts` with vitest tests. First automated net for motion: assert identical cadence at 60/120/144/180Hz.
  2. Carry the remainder: `last += due*WORD_MS` instead of `last = t` (today cadence is quantised to frames and drifts 33.3-34.7ms/word, map 03 §8.2).
  3. dt-normalise the lerp: `k = 1 - (1 - 0.35)^(dt/16.67)` (same feel at 60Hz as today, no longer 3x faster at 180Hz).
  4. Fix the follow quirks: any user wheel-up un-latches immediately (no 80px magnet eating small scrolls); `|delta| < 1` stop (1px stall, map 03 §4.2); rAF-throttle / IntersectionObserver for the turn navigator (no per-scroll O(n) `getBoundingClientRect`); Jump-to-latest uses the follow engine, not native smooth scroll (they contend).
  5. Shiki: don't highlight an open (still-growing) fence; plain mono until the fence closes (growing block misses the cache every frame, map 03 §4.1 S4).
  6. End-of-turn swap must not destroy the live `<Markdown>` (T12).
  7. Defer (measure first): incremental markdown parse (only the trailing block). Dev frames were fine (p99 5.7ms, map 09 §5.1); not worth the risk until a long real turn is measured.
- Pacing (felt; DEFAULT = keep 22727d24 numbers except blur):
  - `WORD_MS 30`, `CATCHUP_LAG 11 words`, lerp 0.35@60Hz: keep (owner hasn't judged them, map 12 0.3).
  - Per-word `filter: blur(3px)` -> opacity-only 320ms (default). Reason: ~18 concurrent filter animations on inline spans (map 09 §6.2), Halo "blur small and once", and blur delays readability = part of the "laggy" feel. Ships as its own commit, revertable.
  - Follow: lerp (default) vs instant pin while streaming = toggle.
- How the owner tunes it: (1) Canvas "Rift · Halo / Motion lab" board runs the extracted `reveal.ts` logic (pure JS) on canned text with sliders (word ms, catch-up, fade ms, blur on/off, lerp/pin, dur-base 240/300); owner picks numbers. (2) Numbers then confirmed in the real WebView2 via `/dev/gallery` (add a param panel; canvas perf != WebView2). (3) `pinToBottom` is NOT exercised by the gallery (scrolls its own `.stage`, map 08 §7.1): final scroll-follow check needs one short billed turn (owner approval, see gates) or a fake-stream dev harness in `AssistantPane` (DEFAULT: build the harness, `import.meta.env.DEV` only, replays gallery fixtures through the real pane).
- Reversibility: structure = cheap per commit; pacing = one constant each.
- OWNER-GATE: yes for pacing numbers (felt), non-blocking: structure ships; pacing commits merge after the owner plays the lab/gallery.
- Unblocks: P4b chat, the owner's #1 complaint.

### T12 Turn lifecycle motion

- Options: hard swap (today); crossfade; same-node fold.
- DEFAULT: same-node fold. One template for both states, no `{#if streaming}/{:else}` branch swap (that swap is what destroys the Markdown instance): flex column `[activity groups before answerStartIndex][answer slot = trailing say groups][ledger]`. Live mode is already in that DOM order; settled mode only flips a class: activity node collapses (grid-rows) and visual order is set with CSS `order`, so the live `<Markdown>` survives (no re-mount, no `blockIn` replay, no snap of held words). Caveat: while live, a say group that stops being "trailing" moves slots and remounts (same as today, acceptable). Activity groups above it collapse via grid-rows 0fr (`dur-base`, ease-out); the ledger line fades in (opacity, `dur-fast`, 60ms delay). Group keys: first block id, not array index (`StreamTurn.svelte:418,426,487`, map 02 §8.13). Block entrances: `--rise-dense` 3px + opacity at `dur-base`, gated by X2 so history hydration doesn't replay. Every disclosure (10 `transition:slide` sites + `workOpen`/`gcReveal`/`bodyFadeIn`, map 09 §4.1 rule 7) -> one Accordion grid-rows pattern (`dur-base`, content fade +60ms, chevron 180deg; pick ONE disclosure glyph app-wide: Halo chevron-down 180deg).
- Why: the hard swap is the most visible "glitch" candidate (map 03 §4.1, map 09 §5.4); engineering constraint is "no node destruction", the look is a bespoke design item. Ledger IA untouched (owner pick).
- Reversibility: medium (StreamTurn template restructure); keep `streamModel.ts` untouched (1011-line test file).
- OWNER-GATE: no (the fold look goes on the chat canvas board; IA is already owner-picked).
- Unblocks: P4b.

### T13 Loops + reduced motion

- Options: keep all 115 loops; Halo "real activity only" + Halo blanket reduced-motion freeze; real-activity only + net with essential exemption + reactive probe.
- DEFAULT survivors (real activity only): LiveDot (Halo ping, transform/opacity, replaces 5 box-shadow pulses + pane/CI/provider/agent dots) · Spinner (one, 900ms; replaces 25 keyframes) · indeterminate Progress (browser loading, long jobs; replaces the comet) · `.shim` on the ONE footer live verb (owner-approved, map 12 M16; killed on `.wb-label`, `.splan-text`, `.wa-text`, `.sh-label`) · terminal cursors (idle shell, exit-plan caret) · dictation caret + polish pulse + mic waveform (real signal, `height` -> `scaleY`) · ConversationList working indicator (keep 9-dot grid or LiveDot: perf of 9 nodes x N rows -> default LiveDot + elapsed text).
- KILL: welcome mark/dot breathe, `pill-ultra`, `.pill.ok` breathe, `up-pulse`/LimitBar sheen + brightness, enhance "magic" (shimmer + aura + 4 stars + wand pulse -> Spinner in the wand + static dim draft), drop-dot pulse, splash caret blink, orb breathe, legacy rail/avatar glows, hot-verdict pulse (static danger Badge). Onboarding mini-demos: play once on step enter, end on the composed static frame (no loop) (map 05 §10 Q6; teaching value kept).
- Reduced motion: (1) one global net in `app.css` = Halo's `*` rule (map 10 §2); (2) exemption `[data-essential-motion]` on Spinner only (keeps rotating; Rift's PermissionBar stance "state indicator, not decoration", map 09 §7.5); LiveDot ping stops, dot stays solid; skeleton static; (3) `prefersReducedMotion()` becomes a reactive `$state` fed by a `matchMedia` change listener (today one-shot snapshot, map 01 §3); (4) a `rm()` helper wraps every Svelte `transition:`/`in:`/`out:` param (10 unguarded sites, map 09 §7.3) because CSS nets don't reach JS transitions; (5) rAF engines (reveal, follow, count-up, roll) read the reactive flag and jump to end state; (6) delete per-selector blocks only as the net lands.
- Why: 115 infinite declarations, 13 motion files with zero reduced-motion handling (map 09 §7.2); Halo rule + owner's "most-rejected category".
- Reversibility: cheap per loop (one commit per kill group).
- OWNER-GATE: no (flag onboarding play-once and the enhance replacement on the canvas).
- Unblocks: P2.

### T14 Skeleton

- Options: Halo shimmer; static bones + Halo hand-off; Rift idle cursor/timer only.
- DEFAULT: one `ui/Skeleton` with static bones (`bg-200`, radius 6), NO travelling sweep (owner banned "banded sweeps", map 12 M2), Halo hand-off kept: bones `opacity 0` 360ms + content `halo-reveal` (opacity + 6px rise, blur 6px allowed: small area, once). Under reduced motion: instant swap. Replaces `Skeleton.svelte` sweep, `ws-load-shimmer`, `skel-shimmer`, `mcp-shimmer`, AI Health pulse (4 keyframe families, 2 implementations; 3 animate `background-position` = paint per frame).
- Reversibility: cheap (adding `::after` sweep later = 3 lines).
- OWNER-GATE: no (prior explicit ban honoured; flag Halo shimmer as available).
- Unblocks: P3, 4d/4f loading states.

### T15 Blur / backdrop-filter / glass

- Options: keep current blurs; blur only on non-fixed layers; zero backdrop-filter + opaque floating recipe.
- DEFAULT: zero `backdrop-filter` in the app (16 declarations, 3 on `position:fixed` layers that WebView2 mis-composites: `SettingsMenu:519`, `ConversationList:512`, `ProjectSwitcher:266`; map 09 §6.1). Floating surfaces = opaque `bg-200` + `line-200` + `shadow-pop`. `filter: blur` allowed only on: splash mark/letters (boot only), skeleton hand-off, nothing continuous. `.cm-fade` code collapse = gradient only. New check-ui metric `backdrop-filter` (scan `.svelte` + `.css`), baseline today's count, ratchet to 0.
- Why: project's own §10 ban + Halo product category has none (map 10 §5.2 #5). Composer well blur re-samples a streaming transcript every frame.
- Reversibility: n/a (removing a defect).
- OWNER-GATE: no.
- Unblocks: P2.

### T16 Halo extras

- Options: adopt all Halo extras; adopt none; per-extra (orientation yes, delight no unless felt).
- DEFAULT per extra:
  - Card spotlight: OFF. Not built until the owner feels it on a canvas board (owner retired spotlight cards 2026-07-16 and rejected travelling light twice, map 12 C2/M3). If wanted: hero cards only (Home overview, AI Health summary), rAF-throttled action.
  - Button magnetic: OFF. FocusGroup dim: OFF (built-then-rejected risk class, map 12 §1.8).
  - glow-strong: ON only under hovered solid primary (Send + <=1 CTA/screen).
  - Press: `scale: .97` via the individual `scale` property on buttons only (not rows, not tiles), `dur-fast`. Replaces translateY(1px)/scale(.96)/80ms variants.
  - Numbers: count-up (easeOutCubic, Halo 1100ms; DEFAULT 700ms = Halo Progress count) on load-once and range-change only, never on the 60s `statsNow`/30s ticks (map 05 §9). RollingNumber (900ms, right-most first) for live token + diff `+N/-M` counts (replaces `AnimatedCount`, keeps monotonic guard + 0.6 opacity floor, map 12 X42). Live elapsed seconds = plain tabular text (a roll every second = permanent motion).
  - Hover lift: 3px only on whole-card-clickable cards (project cards, starters); never stream tiles or rows.
- Reversibility: cheap (all opt-in).
- OWNER-GATE: no.
- Unblocks: P3, 4d/4f.

### T17 Tooltips

- Options: revive Halo Tooltip; keep aria-only; delete call sites + visible hints.
- DEFAULT: revive behind the EXISTING `use:tooltip` action (174 call sites / 37 files unchanged): one singleton tooltip layer (single DOM node, positioned with floating-ui: bits-ui's dep or bits-ui Tooltip; verify import path in P0), inverted bubble (bg `ink`, text `bg-000`), caption 12/16, 4px rise, `dur-fast`. Calm timing: 400ms cold show, 120ms warm (moving between tooltipped targets), hide immediate. Keep the aria-label promotion for icon-only buttons; add `aria-describedby` when the trigger has visible text. `z-tooltip` top. Suppressed while the pointer is over the native browser stage (can't paint over it). Native `title=` in WebBrowserPage (16) + McpServersDialog (5) -> the action (one policy).
- Why: pulled 2026-06-15 as "visual noise" (`tooltip.ts:1-8`), but exact numbers on AI Health/Home are unreachable today and copy says "hover to see" (maps 05 §8, 06 §8.4, 07 §8.1). Singleton = 1 node, not 174 components. Halo requires tooltips on icon-only buttons.
- Reversibility: cheap (one file; flip back to no-op).
- OWNER-GATE: no (flag: reverses a prior owner call; the 400ms cold delay is the calm answer).
- Unblocks: icon-only toolbars (composer, diagnostics, browser), stat values.

### T18 Syntax colour

- Options: keep Shiki everywhere; Halo monochrome CodeBlock everywhere; Shiki in bodies, mono in chrome.
- DEFAULT: keep Shiki for code bodies (Markdown fences, Read, diff lines), swap `github-dark-dimmed` for a custom muted theme generated from tokens (ink / ink-muted / chart-1..3 desaturated; accent reserved for nothing semantic). Shell command heads + terminal prompts = monochrome (fixes kind-coloured chrome, map 02 §8.5). Code-block CHROME = Halo CodeBlock recipe (bg-100, line-100, radius 12, compact 32px head, language label, copy "Copied" accent / "Copy failed" danger; Rift swallows copy errors today, map 03 §8.13). DOMPurify allow-list unchanged (security-adjacent; going monochrome later could shrink it). Growing-fence deferral in T11.
- Why: detail-first transcript is an owner pillar (map 12 D3, S1); dropping colour is a larger UX regression than a token swap; Shiki stays off the hot path with the deferral.
- Reversibility: cheap (theme JSON).
- OWNER-GATE: no (flag: deviates from Halo's "never colours syntax").
- Unblocks: 4b.

### T19 Navigation

- Options: add open-chat Tabs strip (mock); keep sidebar list as tab list; collapse to 72px icon rail vs collapse + peek + resize.
- DEFAULT: no tab strip. Sidebar ConversationList stays the tab list; keep collapse-to-nothing + mini peek + resize (owner iterated 3 sessions, map 12 L2, Q19); 5-tile footer dock re-skinned with NavItem tokens (bg-200 active + accent icon, `aria-current`, focus ring). Fix: one coordinated collapse move (`--rail-w` tween duration = rail width duration, peek animates transform not `top`, reduced-motion covers width/margin). Naming: tile "Home" opens the Workspace page today with aria-label "Workspace" -> label + aria "Home" everywhere; Ctrl+1 opens Home (by order) instead of `goHome()` new chat (Ctrl+N stays new chat) -- small behaviour change, flagged. `registry.kbd` dead field: flag (no consumer, wrong values). Diagnostics: no tile; Settings > System + palette (keep). Browser stays a dock.
- Why: a tab strip is a new stateful surface with zero tests, conflicts with split-pane heads, and the owner removed a tabs pill before ("saw it live, wanted it gone", map 12 L10). Halo 72px rail would drop owner-built peek/resize.
- Reversibility: tab strip later = additive; costly to remove once users rely on it -> that's why it's gated.
- OWNER-GATE: yes (tab strip, because the Halo mock added one and a prior call rejected it). Non-blocking: default = none.
- Unblocks: 4a.

### T20 Overlays

- Options: keep hand-rolled dialogs/menus re-skinned; all onto `ui/` bits-ui primitives; Halo overlay components ported standalone.
- DEFAULT:
  - Dialogs: UpdateDialog, McpServersDialog, CommandPalette move onto `ui/dialog` (bits-ui): fixes z bug (hand-rolled 200 < toasts 2000, map 07 §2), Svelte transitions that ignore reduced motion, hand-rolled Tab traps; lowers `check:ui` portal/role ratchets. Motion: Halo `halo-zoom` (scale .94 + 8px, `dur-base`), leave scale .96 200ms via `[data-state=closed]`. Opaque scrim. Keep MCP's capture-phase Esc (composer swallows Esc, map 07 §9).
  - Palette: bits-ui Dialog + Rift item model/scorer; Halo look + Rift extras (group headings, sub meta, footer legend), row 36 compact.
  - Menus/popovers: `halo-pop-in` (scale .96, 4px, `dur-fast`), origin from `--bits-floating-transform-origin`, rows stagger `min(i,12)*18ms` except Slash/Mention (enter together, owner/code intent "immediate", map 09 D13). Three popover recipes in the shell + `.rift-menu` + SettingsMenu custom -> one.
  - Drawer: Output inspector -> `ui` Drawer (right, 760px). NotificationCenter stays an upward popover from the sidebar bell (a right drawer lands on the native webview), re-skinned with Halo NotificationPanel rows + focus move/trap/return (a11y bug today).
  - Toasts: keep top-center-of-main-island anchor (owner 07-16 choice, map 12 X43) with Halo stack mechanics (collapsed stack, fan + pause on hover, enter from -16px since top-anchored, leave 240ms). Swipe-to-dismiss deferred (desktop, low value, gesture complexity). Tones: accent/info/warn/danger + keep Rift `muted`. Fix `--rail-w` desync so the stack doesn't arrive early.
  - Native webview contract (prerequisite for any overlay near the dock): overlay registry store; while the dock is visible: modals (dialog, palette, drawer) always -> `browser_hide`, restore when none remain; menus/popovers only when their rect intersects `.wb-stage` (browser overflow menu always does); toasts NEVER trigger it (a hide per toast = flicker) -- instead the toast stack is constrained to the main column left of the dock. Existing IPC (`WebBrowserPage.svelte:371-379`). Dock width slide kept (bounds sync), 240ms ease-out, reduced-motion = instant; nothing transformed on an ancestor of `.wb-stage`.
- Why: three scrim recipes, three panel recipes, inconsistent z; native child webview paints over HTML (`browser/mod.rs:8-11`).
- Reversibility: medium (shared primitives); per-dialog commits.
- OWNER-GATE: no (toast anchor flagged).
- Unblocks: 4f, palette, every popover.

### T21 Chat anatomy

- Options: Ledger IA + DenseTile on Halo tokens; Halo Card/CodeBlock anatomy for tool blocks; ConsoleX steps-rail; Document band (option C, rejected 2026-10-19).
- DEFAULT: Ledger information architecture frozen (owner pick 2026-10-19: answer first, one ruled receipt, hover Copy/Retry; map 12 L16); restyle only. Receipt facts stay sans `caption` tabular (owner C13 mono-in-chrome defect beats Halo mono labels). User message: right bubble, `bg-200` + `line-100`, radius 16 16 6 16, max `min(100%, 560px)`, no gradient/shadow, timestamp caption `ink-faint`. Tool blocks: Rift `DenseTile` (bg-100, line-100 -> line-200 hover, radius 12, head 30px, pad 6x10, no lift) not Halo 24px Card (map 02 G1). State edges: drop the 2px inset edge; failed = danger icon + Badge only (3 signals for 1 fact today, map 02 §3); ask/live = `line-200` + accent LiveDot. HUDs: keep both (owner asks), opaque `bg-200` + `shadow-pop`, `halo-pop-in`; IO selectors untouched. ConsoleX steps-rail: not adopted now (WorkLine already groups; revisit with the owner). Stream gap: `max_tokens` "Continue" + refusal notices exist only in the legacy tree (map 02 §2) -> add as inline Banner in P4b (behaviour gap, separate commit). Legacy `MessageBubble` assistant branch + `streamMode` toggle: freeze (no restyle); shared parts restyle hits both trees, acceptable; deletion = owner approval, dedicated commit after P4b.
- Why: the Ledger is HEAD~1 and explicitly chosen; ~700 dormant lines aren't worth restyling.
- Reversibility: cheap (CSS) except IA (not changing).
- OWNER-GATE: deletion of the legacy tree only.
- Unblocks: 4b.

### T22 Composer anatomy

- Options: controls inside the card (mock, ConsoleX one-row); bar below the well (Rift v4).
- DEFAULT: bar below the well (owner iterated 4x, map 12 L15/S5; hero/docked FLIP + `composerH` RO depend on it, map 04 §9). Well = Halo textarea recipe explicitly (bg-100, line-300, focus-within accent + focus ring around the well), NOT `--bg-inset` (which aliases to bg-000 for terminal/code wells, T8), no blur, radius 12 (hero 16). Controls compact 30px ghost buttons. Send: rounded-square radius 8, accent fill when ready, glyph swap send/stop/queue kept, single press `scale .97`; ripple = one ring once (or none, felt). Stop: danger-soft. Esc-to-stop: implement as the LAST Esc layer (streaming && draft empty && no menu/enhance/mention open) so the shipped hint is true (map 04 §7.20). SettingsMenu: keep virtual-listbox-from-textarea logic; replace per-frame rAF follow with RO + FLIP end event; portal stays (composer-host is a containing block). `will-change: transform` only during FLIP. Re-verify pixel couplings after Geist: `dict-ghost`/`magic-text` padding, mic-ring `rx`.
- Reversibility: costly (moving controls inside changes FLIP heights, focus ring extent, hero bar logic).
- OWNER-GATE: yes (mock had controls inside; owner called the mock "rough", which part is unknown, map 12 Q21). Non-blocking: P4c builds on default, merge after the composer canvas board.
- Unblocks: 4c.

### T23 Scrollbars

- Options: hidden everywhere (CX Oct 10); Halo thin bars; Halo thin + keep Rift's existing hidden sites.
- DEFAULT: Halo thin bars (10px, no arrows, thumb `line-200`, hover `ink-faint`, transparent track) via token swap of `app.css:320-337`; keep `scrollbar-width:none` where Rift already hides (chat `.stream`, welcome/home column, sideways strips). The task premise "hidden" is not Rift's current state (map 12 X29).
- Reversibility: cheap. OWNER-GATE: no. Unblocks: P1.

### T24 Data viz

- Options: Halo Stat/BarChart/LineChart as is; Rift wrappers on Halo tokens; keep current hub viz re-coloured.
- DEFAULT: `StatTile` wrapper (Halo Stat anatomy: label -> metric 26 mono -> detail; delta chip with arrow + sign; `peek` for exact values) with Rift `format(fn)` (compact `32k`, `$8.2k`, durations, text values) because Halo Stat is number + en-US/$ only (map 10 §5.2 #9). Count-up rules per T16. Daily chart: Rift dense bar chart (14/30/60 bars, `bg-300` bars, peak accent, hover value + sibling dim .45, bars grow via `scaleY` on mount and range change, accessible value list / `aria-label`), not BarChart (4-12 categories). Model mix: `ShareBar` top 3 = accent, chart-3 violet `#b79cff`, `ink-muted` grey; rest `bg-300` "Other" (two blues side by side are unreadable once info is light blue). Per-project hues: neutral initials on `bg-300`, accent ring on active only; keep `projectHue` util (tested) unused-flag, don't delete. LimitBar 3 variants -> one Halo Progress (scaleX, tones accent/warn/danger, no pulse/sheen).
- Why: Halo palette has 4 series; one-signal colour; hubs must expose values without tooltips (a11y).
- Reversibility: cheap. OWNER-GATE: no (flag project hues). Unblocks: 4d, 4f.

### T25 Process

- Options: port on `design/chat-ledger`; phase branches off `main`; one long `design/halo` branch; code-first without canvas.
- DEFAULT:
  - Branch: merge `design/chat-ledger` into `main` locally first (standing authorization covers local git; push still asks). Phase branches `design/halo-p<N>-<slug>` off `main`, local merge after gates. CI only runs on push to `main` (map 08 §5, check.yml) -> every merge needs `npm run verify:frontend` locally; phases touching Rust (P1 browser bg colour, P4f overlay/occlusion) need full `npm run verify` on Windows.
  - Canvas-first: Claude Design canvas project "Rift · Halo" with Halo DS installed. Boards: Tokens/type/density sampler (feeds T6), Shell frame A/B (T1), Chat turn live->settled (T12) + Motion lab (T11, T10 base toggle), Composer (T22), Home, Settings, Health. Owner plays boards in Play mode (CX precedent, map 11 §4.15). The canvas is a numbers contract, not markup: every board exports px/ms/hex into `docs/design/halo/spec/<surface>.md`; port reads that file. P0-P3 need no canvas.
  - Commit granularity: one concern per commit; token VALUE changes separate from token PLUMBING; each pacing constant its own commit; each kill-group its own commit; rename-with-consumer in the same commit (X3).
  - Verification per phase: `npm run check` (svelte-check 0/0 by convention + check:tokens + check:ui), `npm test`, `c.sh tour` + `c.sh diff` vs P0 baselines, `c.sh measure` probes, `inspect` 0 console errors, `/dev/ui` + `/dev/gallery` shots, reduced-motion emulation pass. No DOM tests exist (31 node-env test files, map 08 §6): new pure tests = token resolution (T8), reveal/follow cadence (T11).
  - Stale docs: add a one-line "superseded by 22727d24" note at the pacing sections of `docs/HANDOFF.md` and `docs/design/redesign-plan.md` (don't rewrite the audit). DESIGN.md rewrite in P1 (T8).
- OWNER-GATE: no (push/merge-to-remote always asks).
- Unblocks: everything.

### T26 Logo / splash / brand

- Options: keep Rift mark + seam splash; Halo-style fade/rise splash; drop splash.
- DEFAULT: keep Rift mark + seam-parting splash + Lexend wordmark (brand). Retune only: kill caret blink, drop lockup exit blur, curves to tokens (keep the panel split `(.65,0,.35,1)` as the ONE named exception curve), panel colour from `--bg` (= bg-000). First paint `app.html:9` -> `#08090c`; Rust browser child bg `browser/mod.rs:218` -> `Color(8,9,12,255)` (Windows-verify). Check the dark squircle PNG edge on bg-000 in P1 shots.
- Reversibility: cheap. OWNER-GATE: no (map 12 Q18 default keep). Unblocks: P1.

---

### X1 Phase-0 verification spikes (build nothing on these until checked)

1. `floatingMotion` dead? (`data-open:` vs bits-ui `data-state`): open a dropdown under CDP, read computed `animation-name`. Decides T9 fix.
2. `@fontsource-variable/geist{,-mono}` install + italic css + Geist Mono ligatures (`--code-liga`).
3. `interpolate-size` support in the installed WebView2 (`.wline` morph) -> if absent, Accordion grid-rows anyway.
4. Drag regions: `data-tauri-drag-region` vs `-webkit-app-region` CSS (which works); double-click maximize; snap layouts.
5. Lucide `optimizeDeps` double-runtime trap (map 11 §4.13) in Rift's Vite config.
6. floating-ui import path for the tooltip singleton (T17).
7. Whether settled-turn history hydration replays `blockIn` (map 09 §5.3) -> sizes X2.

### X2 Entrance replay on remount / hydration

- DEFAULT: one convention. Entrance animations run only on elements that mount while their container is `data-live`; history hydration, tab switch, split swap and re-parent set `data-settled` on the container so `animation-duration: 0s` for the first frame window (CX `.panes.swap` fix "duration 0, not none", map 11 §4.1). Applies to stream `blockIn` (~18 classes), `.bubble` enter, `.crow` rowIn, settings cards, diff rows.
- Why: recurring bug class in CX; Rift renders whole conversations without virtualization (map 03 §9).
- Reversibility: cheap. OWNER-GATE: no. Unblocks: P2 (must precede P4b).

### X3 Markup contracts outside tests

- `.sturn`, `.bubble[data-role=user]`, `.stream`, `.csurf-col`, `.splan`, `#sacard-<id>`, `.pane-shell textarea` (HUD IntersectionObservers), `.composer-wrap`, `.ctxring`, `data-composer-popover`, `[data-qedit]`, `#model-effort-menu`, `.sidebar`, `.ob-host`, `#card-*` + `.card` + `.sflash` (settings jumpTo), `settings-tab-*`/`settings-panel-*`, `#actnode-*` + `.tid-flash`, `.ah-wrap`, `window.__riftDiag`, `[data-a11y-*] .bubble/.markdown-body`, sidebar aria-labels used by `c.sh nav`, `dem-add/dem-del` (test).
- DEFAULT: keep these names; any rename lands in the same commit as every consumer (CDP scripts, rift-ui skill docs, HUDs). Fix the a11y reading-comfort selectors to also match stream prose (`.sturn .md`, map 03 §3) in P4b.
- OWNER-GATE: no.

### X4 Native-side and pre-JS colour

- `app.html:9` inline bg, `#first-paint`, `browser/mod.rs:218`, splash panels, `Markdown.svelte:1205 #22272e`, warm-tint literals. Move in P1 with the tokens. Rust edit unverifiable off Windows (say so if done from cloud).

### X5 Latent undefined vars

- `--text-primary/--text-secondary/--text-muted/--border-subtle/--bg-raised` (WorkspaceShell load-error panel renders unstyled today), `--bg-0` (StreamAskUser), `--font-sans` (SettingsPage:1707, would silently start resolving to Halo's Geist). Fix explicitly in P1 with real tokens; token test flags undefined `var()` names.

### X6 Geometry couplings that break when sizes change

- `sliderBubble.ts` `thumbW=18` (Halo thumb 20), Diagnostics `ROW_H=26` must equal `.dc-row` min-height (one constant shared CSS/JS), mic-ring `rx=10`/viewBox, `.dict-ghost` + `.magic-text` padding = textarea padding (already off by 1-2px, map 04 §8.5), composer FLIP `composerH`, `--chat-col` offsets for turn rail/jump pill, Settings `measureInk` + `onRadioKey` (role=radio; Halo Segmented uses aria-pressed: DEFAULT keep radio semantics + Rift thumb, map 06 §10 Q9).
- DEFAULT: convert each to a CSS var read by JS (or a shared TS const) in the phase that changes its size; checklist item per surface PR.

---

### Owner gates (taste-critical AND costly; everything else proceeds on default)

1. T1 frame: islands-flat (default) vs Halo flat docked frame. Canvas A/B. Blocks merge of P4a only.
2. T11 pacing numbers (blur vs opacity reveal, lerp vs pin, word cadence, dur-base 240/300): felt in Motion lab + dev gallery. Structure fixes ship regardless.
3. T22 composer controls below (default) vs inside the card. Blocks merge of P4c only.
4. T19 open-chat tab strip (default: none).
5. Deletions (global rule): legacy MessageBubble assistant branch + streamMode toggle; design-system static cards / mirror; Rift Style DS artifact supersede; Diagnostics dead modal branch; registry `kbd`; dead CSS listed in maps 04-07.
6. Spend: one short billed real turn to verify scroll-follow (or accept the dev fake-stream harness as proof).

### Expected sharpest disagreements with other lenses

- Flat Halo docked frame vs kept island structure (T1).
- `--dur-base` 240 (Rift-tuned) vs Halo 300 (T10); 14px vs 10px/3px rises.
- No-green (ok -> accent) vs keep green for transcript outcomes; lime info "as is" vs override (T3).
- Compact 30px / 14px prose vs Halo 44px / 15px (T6).
- Alias layer forever vs Halo-native names in markup (T8).
- Restyle `ui/` vs a fresh `components/halo` kit (T9).
- Tab strip from the mock vs no strip (T19); controls inside vs below (T22).
- Skeleton shimmer (Halo) vs static bones (Rift ban) (T14); reviving tooltips the owner pulled (T17).
- Keep Shiki colour vs Halo monochrome (T18); delete the legacy render tree now vs later (T21).

