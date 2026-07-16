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

Focus is always `--ring`, always visible for keyboard. No decorative gradients —
at most a single faint radial page glow behind everything.

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
  surface. Intentional divergences, don't "fix" them: sidebar keeps its opaque
  gradient fill (list rows must not fight the dotted canvas); HUDs keep frosted
  `--surface` 84% + `--border-strong` (floating layers earn a firmer edge).
- DON'T ship a control that spends the user's money without saying so AT the control
  (2026-07-14, fast-mode incident — owner: "it's charging me, that is a huge gap").
  Porting an upstream feature means porting its cost disclosure too (the CLI showed
  "Draws from usage credits"; Rift's toggle didn't). Generalized: anything pay-per-use
  gets the WARN treatment at the point of consent — amber state (never accent), a cost
  line in the row itself (not just a tooltip), and a first-enable toast. Accent = free;
  amber = costs money. Same family as the bypass pill.

## 9. Agent guide — process over palette

1. **Intent first:** before pixels, state in ≤2 sentences what the element's job is and
   how it should feel ("quiet furniture" vs "the protagonist").
2. **Build from tokens;** verify token names against `styles.css` before using them.
3. **Look, then judge:** after any visual change, `bash scripts/cdp/c.sh look "<selector>"`
   and critique before claiming done — spacing rhythm, competing weights, accent overuse.
4. **Named-lens critique:** ask "what would a Linear designer flag?" (ladder discipline,
   accent scarcity) or "a Raycast designer?" (chrome polish, keyboard-first affordances).
5. **Rejection → §8, same session.** That's how this file gets smarter.
