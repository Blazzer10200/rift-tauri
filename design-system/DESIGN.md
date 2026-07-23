# Rift — DESIGN.md (taste layer)

Judgment rules for anyone — human or agent — touching Rift's UI. Token *values* live in
`styles.css` / `src/app.css` (drift-guarded by `check-tokens.mjs`); this file holds the
*decisions*: when to use what, and what's been rejected. Component reference = the cards
in `_ds_manifest.json` (rendered by the in-app Design System pane). Rules here reference
tokens by name only — if a value below ever contradicts `styles.css`, the CSS wins.

## 1. Atmosphere

Rift is a local-first coding assistant — a quiet, precise instrument, not a dashboard.
The look is **Graphite Ink**: near-black blue-gray surfaces (hue ~250–260, barely
chromatic), one themeable accent (emerald by default), dense but breathable. The chrome
is furniture; the conversation stream is the protagonist. The test for any change:
**does the chrome get louder than the content?** If yes, it's wrong.

## 2. Color — roles and discipline

- **Surface ladder carries hierarchy:** `--bg` (window) → `--bg-inset` (wells: composer,
  code, popovers) → `--bg-elev-1` (cards) → `--bg-elev-2` (raised/hover) → `--bg-elev-3`
  (highest/active). Pick by function, don't skip two levels, never invent an off-ladder gray.
- **Lift + hairline, not shadow.** Cards read as elevated via surface step + `--border`.
  Shadows (`--shadow*`) are reserved for genuinely floating layers: menus, popovers, dialogs.
- **Text ladder** = information priority: `--fg` → `--fg-2` → `--fg-muted` → `--fg-subtle`
  → `--fg-faint`. Demote before you shrink — a quieter color beats a smaller size.
- **Accent is scarce and themeable.** The whole ramp derives from `--accent-h/c/a`
  (user-adjustable in Settings). **Solid accent is RARE — Send plus at most one CTA per
  screen**; everything else is ghost (`--accent-soft` fill + `--ghost-border`). Never
  hard-code an accent-ish hex; a hue change in Settings must restyle it for free.
- **Status ≠ activity ≠ accent.** Status (`--ok/warn/danger/info` + `*-soft`) is semantic
  meaning, accent-independent, never decorative. Activity (`--status-busy/pending/warn`)
  means "the system is doing something" — status dots and stream indicators only.
- **One chromatic voice per view:** accent for interaction, status for meaning.
  A second decorative hue is a bug, not a choice.

## 3. Type

- `--font-ui` (Inter, optical features on, −0.005em) for all chrome; `--font-mono`
  (JetBrains Mono) for code, paths, IDs — never for chrome labels. **Lexend is brand +
  accessibility only** (greeting title, splash, dyslexia-friendly mode) — not general UI.
- Scale is small and tight: `--fs-xs` 11 → `--fs-2xl` 22. Hierarchy comes from **weight
  (400/500/600) + text-ladder color**, not size jumps. 600 is the weight ceiling in-app.
- Section labels: 10px / 600 / uppercase / 0.08em / `--fg-faint` + fading rule — the one
  sanctioned place for positive tracking.

## 4. Space, shape, density

- 4px base grid; `--gap` 8px default. Radius ladder: 4 (chips) · 6 · 8 (buttons, inputs)
  · 10 · 12 (`--r-card`, cards) · 16 (rare, large panels). Pills only where established
  (badges, toggles).
- **Density is user-set** (compact/regular/comfy via `--row-h/--gap/--fs-md`). New
  components consume the vars — hard-coded row heights break the setting.
- Compact is the default posture. When unsure: denser and quieter, not bigger and louder.

## 5. Depth

| Level | Treatment | Use |
|---|---|---|
| flat | none | body text, stream content |
| card | elev-1 + `--border` | cards, panels |
| raised | elev-2 + `--border-strong` | hover, featured |
| floating | elev-3 + `--shadow`/`--shadow-lg` | menus, popovers, dialogs |

Focus is always `--ring`, always visible for keyboard. Atmosphere doctrine
(settled 2026-07-16, four owner calls): the CANVAS is plain — flat `--bg`, no
glow, no texture, matching the sidebar's calm. The MAIN ISLAND is an ONYX SLAB —
a machined material surface, not a lit stage — and owns ALL atmosphere (AppShell
`.main` + its pseudos, clipped by the island radius): the SAME opaque gradient
fill as the sidebar island (fg 4%→1.8% over 280px — the two docked islands are
panels cut from one slab, owner call 2026-07-16) + a grounded foot,
a machined bevel (inset dark 1px seam + a 1px accent-cooled top-edge catch,
both box-shadows), and static SVG film grain at ≤3% (banding-killer). Tonal
gradients are strictly VERTICAL — a diagonal satin sheen was tried and rejected
same day (owner: "sideways glares"); angled washes read as glare on a screen,
not finish. Quality reads as CRAFT — edges, tonal depth, finish — never
as illumination or pattern. NO other page, pane, or component adds its own
wash/glow/grain behind content — local atmosphere layers stack into blotches
(see §8). Tight component effects (a button aura, a dialog head-glow) are fine;
surface-scale washes are not.

## 6. Motion

- Shared vocabulary only: `--ease-page` (entrances) / `--ease-soft` (layout shifts);
  `--dur-fast` 140 hover · `--dur-base` 240 standard · `--dur-rise` 460 + `--stagger` 62
  for content entering. `--pulse-live` breathing = live status dots only.
- Motion exists for **orientation** (what appeared, from where), not delight. One
  orchestrated entrance beats scattered effects. Respect reduced-motion.

## 7. Component idioms

- **Buttons:** ghost-first. Solid accent = Send + ≤1 CTA/screen. Secondary = surface +
  border. Destructive = `--danger` and asks first.
- **Chips/pills:** caption-size type, `*-soft` fills — never solid status fills on labels.
- **Wells** (composer, code blocks) sit on `--bg-inset`: content sinks, chrome lifts.
- **Copy:** name what the user controls, active voice, same verb through the whole flow.
  Errors say what happened + what to do — no apology, no vagueness. Empty states invite
  one action.

## 8. Do's & Don'ts — the rejection log

**This section grows.** Every real "that looks off" from Blazzer becomes a line here,
same session — dated, with the rule generalized. This is the app's accumulated taste;
seed rules below.

- DO keep chrome quieter than content — stream text is the brightest thing on screen.
- DO derive every new color from an existing token; extend the ladder in `app.css` +
  `styles.css` together if one is genuinely missing.
- DON'T introduce a second chromatic accent, atmospheric gradients, or spotlight cards.
- DON'T use purple/lavender-on-dark defaults or oversized hero numerals (AI-slop tells).
- DON'T pill-round buttons or inputs; pills are for badges/toggles only.
- DON'T hard-code hex, row heights, or durations where a token exists.
- DON'T write `var(--token, #hexFallback)` with a divergent hex (2026-07-14): the
  ok/warn/danger tokens are `:root`-global so the fallback is dead code, and the ones
  found (Tailwind green/red/amber) had drifted from the tokens' real oklch values —
  a wrong color waiting for a scoping accident. Use the bare `var(--token)`; if a
  fallback is genuinely needed, it must equal the token's current value.
- DON'T stack multiple kbd hints into one placeholder/ghost line (2026-07-14,
  composer idle ghost "Ask · / · @ · Ctrl+D" — owner: "looks like ass"). A
  placeholder is an invitation, not documentation: one quiet plain-text phrase
  at a time; if more must be taught, rotate phrases slowly instead of joining
  them with separators and keycaps.
- DO frame tool activity as islands (2026-07-15, owner: work rows "too
  transparent"). Every tool block in the stream — work-line groups, the
  in-flight row, edit batches — sits on the same card shell as shell/result
  blocks (hairline `--border` + `--radius-lg` + faint fg-2.5% fill), sibling
  language to the sidebar island. Detail stays fully visible; only the chrome
  got a frame. Narration/prose stays boxless — content is the protagonist,
  the frames are for the MACHINERY.
- DON'T let a transient hover-peek be the full persistent surface teleported over
  the content (2026-07-15, sidebar peek — owner: "overlapping everything… not framed
  correctly"). A peek/flyout is a MINIMIZED sibling of the pinned surface: same
  language (radius, hairline, gradient), compact fixed width, height that hugs its
  content, only the high-frequency items — with a quiet seam back to the full
  version. Full-height + full-width floating over content reads as a glitch, not
  an affordance.
- DO speak island as ONE dialect, ONE level deep (2026-07-15, app-wide rollout).
  The recipe lives in tokens — `--island-radius` / `--island-border` /
  `--island-fill` (+ `.island` / `.island-float` utilities in app.css) — never
  re-roll the literals. Two tiers only: docked (border + tint, no shadow:
  sidebar, main card) and floating (+ `--shadow-float`: peek, HUDs, palette,
  popovers). Nesting ceiling is canvas → island → content: INSIDE an island,
  differentiate with hairline separators or tint tiles (fill/border, smaller
  radius, no shadow — settings cards, jump-back-in, stream tool rows), never a
  second full island. The status bar is the main island's footer, not its own
  surface. Both docked islands (sidebar, main) share the same opaque gradient
  fill since 2026-07-16 (see §5) — `--island-fill` remains the recipe for the
  floating tier. Intentional divergence, don't "fix" it: HUDs keep frosted
  `--surface` 84% + `--border-strong` (floating layers earn a firmer edge).
- DON'T equate "quiet" with invisible (2026-07-16, canvas — owner: "still not
  impressed… something more immersed"). A whisper-level glow read as no background
  at all. The canvas light must be plainly visible in a side-by-side — restraint
  applies to chrome and components, not to the stage they sit on.
- DON'T stack atmosphere layers (2026-07-16, canvas — owner: "shit stacked on top
  of shit"). Drifting aurora blobs over glow over dots over grain — PLUS pane- and
  page-local washes from earlier eras — composited into murky blotches. The fix was
  subtraction: ONE light source + ONE vignette on the canvas, and every page-local
  wash/glow/grain deleted (AssistantPane .atmos, AssistantWelcome launchpad aurora).
  Atmosphere is a single lighting model, not a collection of effects; before adding
  any background treatment, grep for the ones already there.
- DON'T scatter specks on the canvas (2026-07-16, owner: "I'm talking about the
  dotted background — that's what I'm absolutely getting at"). The 30px dot grid
  read as visual noise, not craft. The brief that replaced it: "more of a
  development space, professional grade" — answered with RULED structure (the
  blueprint grid: continuous 1px lines, minor+major cells, masked into the light),
  not scattered points. Generalized: canvas texture must read as engineered
  structure (lines, alignment) or be invisible (grain); dots/specks/particles are
  noise and stay banned. Second refinement same day (owner: "going outside of the
  frame"): pattern textures stay INSIDE the frame of the surface they belong to.
  Final call same day (owner: "something different other than the grid… and this
  brightness effect… easy on the eyes"): the grid AND the accent spotlight both
  retired. The surviving language is MATERIAL — satin luster, machined bevel,
  tonal crown/foot — quality shown through finish and edges, not through
  illumination or pattern. Reach for craft cues first when asked for "quality."
- DON'T leave page-conditional material variants behind after a material change
  (2026-07-16, owner: "the sidebar does not match the other pages… this is some
  stuff you need to catch"). The sidebar had a `.home`-only frosted-glass flavor
  from the glow-canvas era — invisible in code reviews of the new slab, obvious
  the moment the hub opened. When a surface's material changes, grep for EVERY
  conditional variant of that surface (`class:x` + its style overrides) and
  either port it to the new material or delete it. The sidebar now carries the
  full slab recipe (fill + foot + bevel + grain) identical to AppShell `.main`.
- DON'T ship a control that spends the user's money without saying so AT the control
  (2026-07-14, fast-mode incident — owner: "it's charging me, that is a huge gap").
  Porting an upstream feature means porting its cost disclosure too (the CLI showed
  "Draws from usage credits"; Rift's toggle didn't). Generalized: anything pay-per-use
  gets the WARN treatment at the point of consent — amber state (never accent), a cost
  line in the row itself (not just a tooltip), and a first-enable toast. Accent = free;
  amber = costs money. Same family as the bypass pill.

- DON'T use a traveling light-band shimmer (skeleton-loader sweep) as a "working"
  state (2026-07-21, stream-blocks comp — owner: "that little loading blurry block…
  looks old school"). A blurry gradient sweeping across content is 2015 skeleton-UI
  vocabulary and reads as fake progress. Show waiting in the surface's OWN idiom:
  a terminal waits with an idle cursor where output will appear + a ticking elapsed
  timer + the breathing live dot — honest signals, no light show. (Scope: banded
  sweeps over blocks/rows; the owner-approved accent `.shim` on single live VERB
  text is a different, settled treatment — don't extend it to blocks either.)

- DON'T drop a sunken panel into a flat surface (2026-07-23, effort-slider well —
  owner: "looks like shit compared to the rest of the menu"). A recessed near-black
  well behind the effort rail clashed with a menu whose every other row is flat.
  When a control inside a popover/menu needs a background, borrow the recipe of a
  sibling control in the SAME surface (the slider now wears the .fast-switch pill
  track + switch-ON accent fill) instead of inventing a new depth treatment. Same
  session, first pass: replacing an owner-called anatomy (bare detents) when the ask
  was additive ("add a background") also got rejected — additive ask, additive change.

- DON'T orbit a light around a container as a "working" state (2026-07-22,
  composer comet arc — owner: "I like it but it's disturbing… too distracting").
  Same family as the banned skeleton sweep: TRAVELING motion pinned to a fixed
  chrome edge competes with the stream for the eye, and it never stops while a
  turn runs. Liveness on chrome = stationary signals only (breathing opacity,
  tinted border/halo, a live dot); motion may travel WITH content as it enters,
  never around a frame.

## 9. Agent guide — process over palette

1. **Intent first:** before pixels, state in ≤2 sentences what the element's job is and
   how it should feel ("quiet furniture" vs "the protagonist").
2. **Build from tokens;** verify token names against `styles.css` before using them.
3. **Look, then judge:** after any visual change, `bash scripts/cdp/c.sh look "<selector>"`
   and critique before claiming done — spacing rhythm, competing weights, accent overuse.
4. **Named-lens critique:** ask "what would a Linear designer flag?" (ladder discipline,
   accent scarcity) or "a Raycast designer?" (chrome polish, keyboard-first affordances).
5. **Rejection → §8, same session.** That's how this file gets smarter.
