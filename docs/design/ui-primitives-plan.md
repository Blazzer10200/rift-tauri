---
title: UI primitives pilot — plan
tags: [rift, design, plan]
status: shipped v0.160.0 — remaining swaps in docs/HANDOFF.md
updated: 2026-10-18
---

# UI primitives pilot

## Why

Owner goal: Claude produces consistent, higher-quality UI. Electron conversion
rejected 2026-09-25 — the shell (Tauri vs Electron) doesn't touch the UI layer;
same Svelte in a webview either way.

Diagnosis (measured 2026-09-25):

- **Look is well-specified — not the problem.** `design-system/DESIGN.md` taste
  layer + rejection log, ~80 OKLCH tokens in `src/app.css`, drift guard
  `check-tokens.mjs`, shared recipes (`.btn*`, `.pill*`, `.rift-toggle`,
  `.rift-menu*`, `.dialog-overlay`, `.tile*`).
- **Behavior + structure is hand-rolled per component.** 14 overlays
  (menus/popovers/dialogs/select/toast), 28 `getBoundingClientRect` calls in 14
  files. Keyboard support inconsistent: PermMenu none; FilePathMenu / GhPopover
  Esc only; ContextMenuHost / Select full arrows + focus return.
  `src/lib/actions/portal.ts` only moves the node (no dismiss/focus logic).
- 3 shared components total (`src/lib/components/shared/`). Styling = scoped
  `<style>` blocks (Composer 2661 ln / ~770 CSS, SettingsMenu 972 / ~470).
  Tailwind v4 imported, ~0 utility classes used, 0 `@apply`/`@theme`.
- 0 component tests. No ESLint / Stylelint.
- Net: every new UI reinvents a primitive → drift.

## Decision

**shadcn-svelte 1.7.0 (Bits UI 2.19.3 underneath), manual setup, restyled to Rift.**

- Why: Svelte 5 + Tailwind 4 native; code is copied into the repo (we own and
  restyle it); Bits UI owns positioning / focus / keyboard / a11y; publishes
  `https://shadcn-svelte.com/llms.txt`; mirrors React shadcn — the component
  vocabulary Claude knows best.
- Rejected: Skeleton v3 (ships its own visual language — fights Graphite Ink);
  Melt next-gen, Flowbite-next (pre-1.0); Ark UI (Svelte secondary to React);
  raw Bits UI (same engine, no scaffold/registry); React swap (113K-line rewrite —
  revisit only if pilot fails).
- **The look stays Rift.** Primitives reuse existing recipes (`.rift-menu`,
  `.btn` variants, floating tier, density vars). The pilot changes behavior and
  structure, not the brand.

Verified 2026-09-25: `npm view` shadcn-svelte 1.7.0 (2026-09-16), bits-ui 2.19.3
(peers `svelte ^5.33`, `@internationalized/date ^3.8.1`), `cn` 0.4.0
(shadcn-ui/cn, replaces clsx+tailwind-merge in 1.7), tailwind-variants 3.3.1,
tw-animate-css 1.4.0. No `src/lib/utils.ts`, `components/ui/`, `hooks/`,
`components.json` exist → no collisions.

## Principles

1. **One vocabulary.** Tailwind theme names = Rift token names. Wipe the default
   palette (`@theme { --color-*: initial; }`) so off-palette utilities can't
   exist. Never add shadcn's raw `:root` vars — shadcn `--accent` means hover
   gray, Rift `--accent` means emerald brand; "accent" must mean one thing.
2. **No global reset changes.** Skip shadcn's `@layer base` (border/body rules).
   Tailwind preflight already active via `@import "tailwindcss"`.
3. **Manual setup, not `init`** — `init` rewrites the CSS entry. Use CLI `add`
   only, with a hand-written `components.json`.
4. Copied components restyled on arrival; the file is the source of truth.
5. Feature components keep scoped CSS (no mass rewrite). New overlays / buttons /
   inputs must compose `ui/`.
6. Additive, one commit per swap → any swap reverts alone.

## Phases

0. **Baseline** — branch `ui/primitives` off `main` @ `44d054fc`. CDP
   before-screenshots + keyboard notes for the 4 pilot surfaces.
1. **Foundation** — deps: `bits-ui`, `tailwind-variants`, `cn`,
   `tw-animate-css`, `@internationalized/date`. `src/lib/utils.ts` (`cn` + the
   ref/child prop types components expect). `components.json` (aliases →
   `$lib/components/ui`, `$lib/utils`). `app.css`: palette reset + `@theme inline`
   bridge (colors, radius ladder, `--text-*` ← `--fs-*`, shadows, motion).
   Unit test: `cn` merge with Rift class names (color vs size never collapse).
2. **Primitives** — add + restyle: button, dropdown-menu, context-menu, select,
   alert-dialog. Button variants = `.btn` variants 1:1 (primary/ghost/danger/
   warn/info, sm/xs). Menus = `.rift-menu` recipe. Motion = `--dur-fast` +
   Rift eases, reduced-motion respected.
3. **Living catalog** — `/dev/ui` route (dev-only, like `/dev/gallery`): every
   primitive × variant × state (hover/focus/disabled) × density
   (compact/regular/comfy) × accent hue. Renders real components → can't drift
   like the static `design-system/components/*.html` cards.
4. **Pilot swaps** (1 commit each):
   - a. `composer/PermMenu.svelte` (180 ln, no keyboard) → DropdownMenu
   - b. `shell/ContextMenuHost.svelte` (right-click, app-wide) → ContextMenu
   - c. `shared/Select.svelte` (Settings) → Select
   - d. `shell/CloseConfirm.svelte` → AlertDialog
5. **Guardrail** — `design-system/check-ui.mjs` in `npm run check`. Ratchet
   baseline (JSON) per file: `use:portal` or `role="menu|dialog|listbox"`
   outside `ui/`, raw hex/rgb in `.svelte`, arbitrary Tailwind values (`-[`).
   Counts may only go down.
6. **Agent docs** — DESIGN.md §10 Primitives + §9 step "compose from `ui/`,
   check `/dev/ui`"; AGENTS.md pointer under Rift UI Workflow; llms.txt URL.

## Verification

- `npm run check` (svelte-check + tokens + ui ratchet), `npm test` (+ `cn`
  test). No Rust touched → cargo not required.
- CDP live per swap (`rift-ui` skill): open, arrows, Home/End, typeahead, Esc,
  focus returns to trigger, click-outside, 0 console errors, after-screenshot
  vs before.

## What Blazzer tests

1. `/dev/ui` catalog — does the system read as Rift?
2. The 4 swapped surfaces in the real app — same look, consistent keyboard +
   dismiss behavior.
3. **Acid test:** owner names any screen/feature; Claude builds it from
   primitives only; judge consistency.

Pass → rollout: SettingsMenu, GhPopover, FilePathMenu, NotificationCenter,
Update/MCP dialogs, CommandPalette → Command, settings toggles/radio grids →
Switch/ToggleGroup, `.btn` usages → `<Button>`. Fail → reassess (React inside
Tauri).

## Out of scope (pilot)

SlashMenu / MentionPopover (textarea-driven combobox — keys handled by
Composer, correct pattern already), toasts, tooltips (`use:tooltip` stays),
Composer / AssistantPane restructure, light mode.

## Risks

- `cn` / tailwind-merge unaware of custom names → silently drops a class.
  Mitigation: standard Tailwind namespaces + unit test. Fallback:
  clsx + `extendTailwindMerge`.
- Bits UI floating layers in WebView2 — no known Tauri reports; verify live.
  LIKELY: native child webviews (in-app browser) paint above all DOM, so a menu
  overlapping the browser pane gets covered — true of today's menus too.
- tw-animate-css exit animations vs Rift motion tokens — tune or drop.
- vite-plugin-svelte#1143 "don't deploy" warning was against the Vite 8 beta;
  Rift already ships on Vite 8.3.
- CDP scripts reference none of the pilot selectors (grep 0 hits).
