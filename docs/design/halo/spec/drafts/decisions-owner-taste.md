---
title: Halo decision register — owner-taste + calm lens (draft)
tags: [rift, halo, redesign, spec, draft]
status: draft (one of several lens drafts; synthesis merges)
updated: 2026-10-19
lens: owner taste + calm (ADHD readability, every fact once)
inputs: docs/design/halo/map/01..12 (all read in full this session)
---

# Halo decision register — owner-taste + calm lens

Map cites: `M01`..`M12` = `docs/design/halo/map/NN-*.md` + section. Owner quotes ≤15 words. "OB" = owner-built feature (M12 §4 proud list). Bare owner IDs (`L#` layout, `D#` density, `C#` colour, `T#` type, `M#` motion, `K#` chrome, `S#` shipped/owner-built, `P#` copy, `X#` cross-cutting, `Q#` open question) = rows in `12-owner-preferences.md` §1.1-§1.x / §3-§5, each with source path + date. `T1..T31` as headings = this register's topics.

## 0. Ground rules this lens applied

- **Owner's binding words:** "Electric blue is the one. Use it as is." · "find a happy medium for literally everything" · "proper motion and animations of everything" · first Halo mock "rough" (which parts: UNKNOWN, M12 Q21). "Use it as is" = the PRESET (colour). It does not by itself carry Halo's 44px / 15px / AppShell.
- **Owner's own Halo density precedent = ConsoleX after the 2026-10-17 calm pass** (M11 §1, §5): body 14, `.btn` 40, iconbtn 32, toolbar 52→44, sidebar 232→208 / 72→56, column 780. Owner quotes there: "cleaner and more clear … no clutter", "No green anywhere", rejected ring gauges, hates visible/white scrollbars, "restyle, don't rebuild", "really see the pages" before porting.
- **NOT owner approval** (M11 §5 last row): ConsoleX page glide, per-word blur fade, shimmer, nav plate = "AI-chosen, owner-tolerated". Not cited as approval below.
- **Pacing `22727d24` verdict = UNKNOWN** (M12 §0.2-0.3). Nothing below treats 30 ms/word, 320 ms catch-up, 0.35 lerp, 3 px as owner-approved.
- **Delegated "surprise me" additions get rejected** (comet arc, per-project hues; M12 §1.8 #2). Halo delight layer (spotlight, magnetic, dim, shimmer skeleton, bell swing) is the same risk class → default OFF.
- **Gate test:** OWNER-GATE = yes only if taste-critical AND costly to reverse. Precondition: T8 lands as an alias layer. After T8, every colour / radius / duration / semantic default is a one-declaration revert → NOT gated, shown on canvas boards instead. Gated = removes an owner-built feature, deletes code/assets, crosses into Rust/IPC, ships pacing (owner's own rule `redesign-plan.md:209-215`), or pushes.
- Any default that changes an OB item says so in **Owner-built touched**.

## 1. Owner sign-off list (short)

| # | Gate | Default work proceeds on |
|---|---|---|
| G1 | Canvas boards 00 (type + density specimen) and 01 (frame A vs B) signed off before any surface port. Board 00 opens with Q21: "what felt rough in the first mock?" | Compact Rift-Halo scale (T6) + islands-flat frame (T1) |
| G2 | Retire accent picker (8 swatches + hue + vividness, Settings Appearance; swatch strip onboarding step 1) → fixed Electric blue | Tokens fixed blue from day 1; Settings/onboarding controls removed in their phases; stored prefs ignored, not deleted |
| G3 | Delete legacy assistant render tree (MessageBubble assistant branch + ToolChip/TurnSummary/AskUserCard/AgentCard/TodoCard + Stream-view toggle, ~2k lines) | Not ported; left untouched (old look) until yes |
| G4 | Ship streaming pacing numbers (word rate, fade, lag cap, follow τ) | Mechanism fixes ship; numbers stay at `22727d24` values, flagged unconfirmed, until owner picks in motion lab |
| G5 | `design-system/` static cards + `_ds_manifest.json` + published Rift Style artifact: retire / supersede | Left in place, README marked deprecated; `check:tokens` repointed (T8) |
| G6 | Native browser overlay lock (Rust/IPC hides child webview while an HTML overlay is open) | Build it in overlay phase; until then overlays never overlap the dock |
| G7 | Push / merge `design/halo` | Never without asking |

---

## 2. Register

### T1 · Material / frame
- **Options:** A islands-flat: Rift docked two-island structure, Halo flat material · B Halo AppShell: flat frame, sidebar on `bg-000` + `border-right line-100`, sticky top bar · C keep onyx slab (gradients, grain, bevel, cavity, AO, seam) recoloured.
- **Default:** A. Canvas `bg-000` #08090c; sidebar + main islands `bg-100` #0e1014, 1px `line-100`, radius 16, gutter 12, ONE level deep (canvas → island → content). Delete: vertical gradient fills, SVG grain ×4, bevel top catch, cavity insets, contact AO, shell `.rift-seam`, `.app` 1s bg transition. Depth by lightness only. Wells (composer, code) = `bg-100`/`bg-000` per Halo input/terminal recipe, not "darker inset".
- **Why:** Owner iterated island structure 5 sessions (L1 docked islands, L4 "one material", L5 one level deep, L2/L3 peek). Owner C6 "still not impressed… something more immersed" = stage must be visible — `bg-000`→`bg-100` step does that without washes. Owner C7 "shit stacked on top of shit" + C8 dots/grain retired + Halo "no colour washes" (M10 §4 r2) all point to flat. ConsoleX shipped flat frame and owner called result "cleaner". M12 X2 rule C6: don't ship flat without owner seeing it beside the slab → board 01 shows A vs B (and a C thumbnail).
- **Owner-built touched:** recessed chassis + seam (L6, 2026-07-23, newest canvas call) removed from shell; seam survives as splash motif (T26).
- **Reversibility:** cheap-moderate (AppShell/Sidebar CSS only; logic untouched).
- **OWNER-GATE:** via G1 (board 01). Not separate.
- **Unblocks:** shell phase, every surface's ground colour, native pre-JS colour (→ T8 step 6).

### T2 · Accent model
- **Options:** a keep hue/vividness dials, re-anchored to blue default · b fixed Halo `blue` preset, picker removed · c Halo preset picker (lime/blue/ember/violet) replaces dials.
- **Default:** b. `data-theme="blue"` on `<html>`; accent `#6aa8ff`, accent-ink `#08090c`, accent-soft `rgba(106,168,255,.14)`, glow-strong `.28`. `--accent-hover/-active` alias to accent (Halo has no shades; hover = surface step or glow). `--accent-h/-c` consumers (13 ext, M08 §4.1) → `color-mix(in srgb, var(--accent) N%, transparent)`. Persisted `rift.ui.accent.v1` / `vividness.v1` ignored (keys left). Accent scarcity rule kept: solid accent = Send + ≤1 CTA per view; rest ghost.
- **Why:** "Electric blue is the one. Use it as is." is explicit. Option c reopens a choice the owner closed (ConsoleX: chose blue, no picker). Fixed hex keeps Halo's measured contrast table (M10 §1.1) valid. Scarcity = owner C1 + Halo "one primary per view".
- **Owner-built touched:** S9 accent swatches + vividness (owner-built, persisted) + onboarding step-1 swatch strip. Removal = G2.
- **Reversibility:** token side cheap; UI deletion costly-ish (Settings card, onboarding step, `/dev/ui` accent bar).
- **OWNER-GATE:** yes (G2) for removing the controls; tokens go blue immediately.
- **Unblocks:** T8 alias values, Settings Appearance rebuild, onboarding step 1 rework, `/dev/ui`.

### T3 · Semantic colours on blue
- **Options:** keep Rift greens (`--ok`, `--status-busy`) · Halo literal (positive = accent; `info` = lime) · ConsoleX "no green" (positive = accent, `info` overridden light blue).
- **Default:** ConsoleX model, all as alias tokens:
  - `--info` → `#9cc6ff`, `--info-soft` → `rgba(156,198,255,.14)` (override Halo blue's lime, verified `consolex/panel/src/app.css:62-67`).
  - `--ok` / success / connected / passed → `accent` (+ ✓ icon + word; never colour alone). `--ok-soft` → `accent-soft`.
  - `--status-busy` (working) → accent via `LiveDot`; `--status-pending` → `ink-muted` idle dot.
  - `--warn` → Halo `#ff9f5a`; keeps BOTH meanings owner set: attention AND "costs money" (C3/P5 amber-at-the-control). Bypass / pay-per-use / Fast-mode disclosure stay warn.
  - `--danger` → `#ff6b6b`; destructive + errors + failed.
  - Plan mode tone → `info` (light blue). Auto-edit → accent.
  - Diff add/del: add = `accent-soft` row wash + accent `+`; del = `danger-soft` + danger `−`. **Confirm on chat board** (diff colour is outside Halo's model).
  - ANSI green in tool output = tool meaning, not chrome → map ANSI green to `accent`, red → danger, yellow → warn, blue/cyan → info, magenta → chart-3 `#b79cff`. **Flag separately** on chat board.
  - Log level `info` → neutral badge (not light blue) to keep logs quiet.
- **Why:** owner "No green anywhere" (2026-10-17, ConsoleX, verified this session) is the newest colour call and was made inside a Halo-blue app. Lime info on a blue app reads as a second accent (owner C2 "a second decorative hue is a bug"). Owner C3 cost-amber has "huge gap" stakes → must survive; Halo warn is same family.
- **Owner-built touched:** none removed; green status vocabulary the owner trained on (M12 Q20) changes meaning → shown on boards.
- **Reversibility:** cheap after T8 (alias values).
- **OWNER-GATE:** no (canvas FYI).
- **Unblocks:** every badge/pill/banner/toast tone mapper (`ok→accent`, `hot→danger`), diff + ANSI renderers, Settings provider status.

### T4 · Light themes
- **Options:** in scope · out.
- **Default:** out of the Halo port. Dark `blue` only. Use `color-mix` (not rgb triplets) so light stays possible later.
- **Why:** owner C13 "Dark direction only" (perpetual). Light is not free (131 oklch literals, 93 gradients, shiki dark, warm tint; M08 R9).
- **Reversibility:** cheap to keep out; costly to add later (separate wave).
- **OWNER-GATE:** no.
- **Unblocks:** token phase scope; drops 4 Halo light blocks from the vendor import.

### T5 · Fonts
- **Options:** Geist + Geist Mono everywhere · keep Inter/JetBrains on Halo scale · Geist UI + JetBrains code.
- **Default:** `@fontsource-variable/geist` + `/geist-mono` (^5.3.0, ConsoleX pin) incl. the italic CSS file (else synthesized slant in markdown). Override `--font-sans: 'Geist Variable', ui-sans-serif, system-ui, sans-serif` and `--font-mono: 'Geist Mono Variable', ui-monospace, monospace` (fontsource names ≠ Halo's `"Geist"`; M10 §6). Retire Inter + JetBrains packages + Inter `font-feature-settings` + body `-0.005em`. Geist Mono also for code. Lexend kept for: dyslexia mode (owner T7 a11y feature) + splash wordmark only. Greeting moves to Geist (Halo `title`). Settings font radio "Inter/Lexend" → "Geist/Lexend". Code-ligature pref: keep only if Geist Mono ships ligatures (UNKNOWN; verify, else hide toggle).
- **Why:** Halo identity is Geist; CSP `font-src 'self' data:` blocks Google import (`tauri.conf.json:28`) → fontsource only. Owner never stated a preference for Inter itself (it was a Graphite rule). Lexend brand use is kept where it's a boot "brand moment"; the greeting is everyday chrome.
- **Owner-built touched:** greeting Lexend (font bug fixed today, T5 in M12) → Geist; dyslexia mode untouched.
- **Reversibility:** cheap (two vars + imports).
- **OWNER-GATE:** no (board 00 shows it).
- **Unblocks:** board 00 specimen, text-width sweep (truncation in 34px rows, dock labels).

### T6 · Type scale + density
- **Options:** Halo native (15 body / 44 controls) · Rift today (12.5-13 / 26) · compact Rift-Halo tier anchored on owner's ConsoleX precedent.
- **Default:** compact Rift-Halo scale as default, density modes kept:
  - Sans: `caption` 12/16 (floor) · `small` 13/18 (UI chrome text, labels, settings rows) · `chat` 14/22 (assistant prose, user message, composer input) · `heading` 16/22 (card/panel titles) · `subtitle` 18/26 (dialog titles) · `title` 24/30 (page title) · no `display`.
  - Mono: `label` 11/14 500 .12em UPPERCASE (SectionLabel + Kbd only: the ONE sub-12 exception) · `data` 13/18 500 tabular (numbers in tables/stats) · `metric` 28/32 600 −.03em (Stat value only, ≤4 per screen).
  - Weights 400/500/600 only (owner 600 ceiling kept).
  - Controls compact: button/input/select/segmented 32 (sm 28 for dense toolbars: pane head, tile actions, browser bar), icon button 32 (sm 28), list/nav/menu rows 32, settings row min 44 incl. description, toggle 36×20, checkbox/radio 16 in a 32 row.
  - Density tiers via existing `--row-h/--gap/--fs-*` contract: compact (default, above) · regular (+4 px controls, chat 15/24) · comfy = Halo native (44/36 controls, body 15).
  - Chat column 760 px shared by messages, composer, alerts, receipt; jump pill + turn rail anchored to it.
  - Floor: lift all 261 sub-12 px declarations to ≥12 (except mono label/kbd).
- **Why:** owner D1 "denser and quieter, not bigger and louder" + D5 rejected BOTH "way too small" and "big and funky" → middle. ConsoleX calm pass (owner "cleaner… no clutter") landed at 14 body / 32-40 controls; this is that, one notch denser for a coding tool. Owner C14 lifted faint text for legibility → floor 12 is consistent. 760 = Halo mock + ConsoleX 780; 14 px Geist at 844 gives ~115 chars/line, too long for ADHD reading. Owner D2 density pref is OB → kept.
- **Owner-built touched:** density tiers kept; compact gets ~6 px taller controls than today.
- **Reversibility:** cheap as tokens; layout fits (hub no-scroll, Settings rows) cost a re-tune → why it's on board 00.
- **OWNER-GATE:** via G1 (board 00).
- **Unblocks:** every surface port; `ROW_H` constant in DiagnosticsConsole (must equal the chosen row height); composer FLIP `dy`.

### T7 · Radius ladder + pills + control heights
- **Options:** Rift 4/6/8/10/12/14/16 · Halo 8/12/16/pill · Halo + one-offs.
- **Default:** Halo ladder: `sm` 8 (controls ≤32 px, menu rows, inner tiles, thumbnails) · `md` 12 (stream tiles, popovers/menus, inputs ≥36, segmented track) · `lg` 16 (islands, cards, dialogs, palette, user bubble with `16 16 4 16`) · `pill` (status/count badges, toggle track, `.jump-latest` float). One-offs allowed (Halo has them): 6 for inline code + kbd + checkbox. Buttons NEVER pill (send = 32 rounded square, radius 8). `.chip-btn` 999 → 8. Concentric rule: 16 panel → 12 → 8.
- **Why:** owner K1 "no pill buttons/inputs; pills for badges/toggles" == Halo rule. 12 radius on a 28-32 button reads pill-like (M12 X25) → 8.
- **Reversibility:** cheap (tokens).
- **OWNER-GATE:** no.
- **Unblocks:** primitive restyle, `recipes.ts`.

### T8 · Token architecture + gates + design-system/
- **Options:** ConsoleX alias layer · rename consumers to Halo names · hybrid.
- **Default:** alias layer (ConsoleX `main.ts:1-4` + `app.css:1-67` pattern):
  1. Vendor Halo `tokens.json` + generated `tokens.css` at `src/lib/styles/halo/` (dark + blue blocks only), imported BEFORE `app.css`.
  2. `app.css` `:root` becomes the alias block: every Rift name → Halo var (`--surface: var(--bg-100)`, `--fg: var(--ink)` …). The 17 colliding names (M08 §1.3) re-declared here with the decided values. This block is the contract; order is load-bearing.
  3. Tailwind `@theme inline` bridge KEYS unchanged (`bg-surface`, `text-fg-2`, `rounded-card`, `ease-page`, `h-row` …); only right-hand values follow aliases. Keeps `check:ui` + `utils.test.ts` green.
  4. Net-new Rift tokens Halo lacks: compact type scale (T6), density vars, tile tokens, `--z-*` (below), `--ease-stream`, chat column.
  5. z ladder (Rift order kept, Halo names): sticky 10 · overlay/dialog 50 · toast 60 · dropdown/popover 65 (deviation from Halo 20, documented: menus inside dialogs must work, owner K7) · tooltip 70 · splash 100. Replace 25 raw z literals.
  6. Pre-JS + native colours: `app.html:9` → `#08090c`; `src-tauri/src/browser/mod.rs:218` `Color(8,9,12,255)`.
  7. `check:tokens`: repoint to assert alias block resolves to vendored Halo `tokens.css` + an explicit Rift-extension allow-list (strongest guard). Never "fix" by editing the stale Graphite mirror.
  8. `check:ui`: keep + `--update` after each phase that lowers counts.
  9. `DESIGN.md` rewritten as "Rift on Halo": taste rules + §8 rejection log kept verbatim (it's the owner's taste memory), Graphite-only rules marked superseded.
- **Why:** M08 §0.1: 4,806 `var()` refs → zero consumer edits; proven in owner's other app. Makes T2/T3/T7/T10 one-line reversible — that's what keeps the gate list short. "Restyle, don't rebuild" (owner, ConsoleX).
- **Reversibility:** cheap (one file + vendor folder); costly if consumers are renamed instead.
- **OWNER-GATE:** no; G5 covers the `design-system/` cards/artifact fate.
- **Unblocks:** everything. Must land first.

### T9 · Component strategy
- **Options:** restyle bits-ui `components/ui/` + `recipes.ts` · new `components/halo/` kit · both.
- **Default:** ONE kit in `components/ui/` (ratchet-exempt home): (a) bits-ui primitives restyled to Halo CSS (dropdown, context, select, alert-dialog) + new ones added via `shadcn-svelte add` (never `init`): dialog, popover, tooltip, tabs, accordion, switch, checkbox, radio-group, progress, combobox, command; (b) CSS-only Halo parts as Svelte components there too: Button, Card, Badge, LiveDot, Spinner, SectionLabel, Kbd, Divider, EmptyState, Banner, Stat, RollingNumber, Skeleton, Segmented, SearchField, Toast. Rift-specific composites (Tile, SettingsRow, DiffBlock, LogStream, TurnRail, PreviewWell, SecretField, ProviderCard, ShareBar, BrowserBar) live beside their surface, built only from `ui/` parts + tokens. Retire `.st-*`, `.ob-*` button/pill dialects, `.btn/.pill/.input` as surfaces port.
- **Why:** M08 §2: Rift already enforces "compose from ui/" (`AGENTS.md:45-49`, `check-ui`); a second kit = third system (M07 §9). bits-ui supplies focus/keyboard the composer's virtual listbox needs (M04 §6). Owner S10 "steadier menus" came from this primitives layer → keep it.
- **Reversibility:** moderate.
- **OWNER-GATE:** no.
- **Unblocks:** all surface ports; `/dev/ui` becomes the Halo specimen page.

### T10 · Motion durations, easings, rise distances
- **Options:** Halo as-is 150/300/600 · Rift-tuned 140/240/520 · Halo + named chat exceptions.
- **Default:** Halo tokens as-is: `dur-fast` 150, `dur-base` 300, `dur-slow` 600, `stagger` 60 (cap 6 per entrance), `ease-out` (.2,.8,.2,1) everywhere; `ease-page`/`ease-soft` alias to it; `ease-spring` only for flip/grab/add (toggle knob, check pop 1.18, slider thumb, owner's effort detent snap). Literal sweep (~150 literals, 36 raw beziers, 104 bare `ease`) → tokens. Named exceptions (only these):
  - `--ease-stream`/stream rise: opacity + 3 px, 200 ms (live blocks only).
  - page change: ONE orchestrated entrance — sections rise 10 px, 520 ms, 60 ms stagger, cap 6, on first open per session; later activations = 520 ms glide of the content area only. Exit 140 ms opacity (old page never ghosts). Replaces today's 3 stacked layers (wsIn + ws-page + rising).
  - menus `halo-pop-in` 150 ms; dialogs `halo-zoom` 300 ms, leave 200 ms; drawers 300/200; toasts 300/240.
  - Three rise distances only: 10 px (pages, cards, empty states), 6 px (skeleton→content hand-off, T14), 3 px (stream). Not Halo's 14 px.
  - Splash doors `(.65,0,.35,1)` kept as the one brand curve (T26).
- **Why:** owner M24 "One orchestrated entrance beats scattered effects" → one entrance, not page-glide + card-rise. Owner M6 "lurching downward" when stream blocks rose 8 px → stream stays tiny. 14 px is Halo-tolerated, never owner-reacted. Owner's "laggy" was streaming pacing (M12 M5), not overlays → Halo durations elsewhere are fine; +60 ms on `dur-base` is a calmer, felt-on-board change.
- **Reversibility:** cheap after T8.
- **OWNER-GATE:** no (motion lab board shows page + overlay motion too).
- **Unblocks:** every motion sweep; kills keyframe duplication (~155 names → ~35).

### T11 · Streaming reveal + scroll follow
- **Options:** keep `22727d24` as-is · mechanism fixes + motion-lab tuning · instant text + trailing fade · line/sentence chunks.
- **Default:** mechanism fixes now; numbers via motion lab (G4):
  - Cadence dt-based with remainder carry (today `last = t` drops it → 29-30 w/s ceiling, uneven ticks; M03 §8 #2). Frame-rate independent.
  - Catch-up: rate scales with lag so lag never exceeds a cap (lab knob, start 8 words).
  - Word appear: opacity only, fade (lab knob, start 180 ms), NO blur, no transform (inline spans). Drops ~18 concurrent filter layers (Halo blur rule + perf).
  - Chrome-hold kept (owner-screenshotted bug fix, M12 S3).
  - Follow: dt-normalised exponential, `k = 1 − exp(−dt/τ)` (lab knob, start τ 80 ms), snap if gap > 1 viewport; instant under reduced motion. Fix 80 px magnet eating small upward scrolls (un-latch on any user-initiated upward wheel/key) and the possible 1 px stall.
  - Kill per-frame full re-parse replays of `code-rise` (fresh nodes each delta).
  - **Motion lab** = Claude Design canvas board 04: canned transcripts (prose / code-heavy / tool-heavy), sliders: word rate 20-60 w/s, fade 0-300 ms, lag cap 4-16, follow τ 0-150 ms, blur on/off (to A/B against today), reveal mode word/line. Owner plays, picks.
  - Canvas runs in a browser, not WebView2: I confirm picked values in-app via CDP + `/dev/gallery` (owner does not have to run a dev server; he finds it "irritating"). One real paid turn for `pinToBottom` only with owner OK (M12 Q16).
- **Why:** owner "a little laggy, could be better" was pacing not frames (p95 5.6 ms, M12 M5). Owner liked the paced word reveal idea (M20) — keep the idea, fix the mechanics. His rule: no speculative pacing ships, one toggle at a time, felt live.
- **Owner-built touched:** S3 paced word reveal kept; blur removed.
- **Reversibility:** cheap (constants).
- **OWNER-GATE:** yes, G4 (shipping numbers only).
- **Unblocks:** chat phase; T12.

### T12 · Turn lifecycle motion
- **Options:** hard swap (today) · fold-to-receipt · crossfade.
- **Default:** fold-to-receipt, no remount: answer `Markdown` instances persist across live→settled (same keyed component; no `blockIn` replay, no snap of held words → let reveal finish). Live activity body collapses via accordion `grid-template-rows 1fr→0fr` 300 ms ease-out; receipt line fades in 150 ms after 60 ms delay. Blocks born live: stream rise (3 px/200 ms); history hydration renders static (no entrance on remount, tab switch, split swap — ConsoleX swap-guard pattern, M11 §4 #1). Disclosure everywhere = Halo accordion (rows 0fr→1fr 300 ms, content fade with 60 ms delay); one chevron glyph app-wide: ChevronDown rotating 180° (300 ms). Replaces 6 different `slide` durations.
- **Why:** M03 §4.1 suspects a blink at turn end (unverified, high suspicion) — the single most visible glitch at the owner's Ledger moment. Owner picked Ledger today; motion must make the receipt arrive, not pop. Halo "content arrives, never pops".
- **Reversibility:** moderate (StreamTurn render structure).
- **OWNER-GATE:** no.
- **Unblocks:** chat phase; Ledger receipt polish.

### T13 · Loops + reduced motion
- **Options:** Halo strict (LiveDot/Spinner/indeterminate only) · keep owner-approved exceptions · keep all.
- **Default:** survive (real activity only): `LiveDot` (Halo ping 1.8 s, transform/opacity; replaces 20+ box-shadow pulses) · ONE `Spinner` (900 ms; replaces 25 keyframes) · indeterminate `Progress` (long jobs, browser load) · terminal/dictation cursors (real cursors) · mic waveform (real signal) · `.shim` on the ONE live footer verb (owner-approved M16; max 1 per view; killed on plan text, write-batch label, work line). Kill: enhance "magic" (shimmer + aura + 4 stars + wand pulse → static dim text + Spinner in wand) · ultra pulse · drop-dot pulse · welcome mark/greeting-dot breathe · `pill.ok` breathe · active-project pulse (selected ≠ live → static accent dot) · LimitBar hot pulse + sheen · AI Health verdict pulse · vitals ×N breathes (one LiveDot for live state) · splash caret blink · `up-pulse` · 9-dot working grid → `LiveDot` + `m:ss` elapsed (owner likes ticking timer, M17). Onboarding miniatures: play once on step enter, rest on composed final frame; small "Replay" text button.
  - Reduced motion: Halo global net (`*` durations .01 ms) + reactive probe (`matchMedia` change → `$state`; today a one-shot snapshot in ~6 components) + explicit static forms: Spinner → static arc + text label; LiveDot → solid dot; shimmer → plain `ink-muted`; cursors solid; Svelte `transition:` sites read the reactive probe (10 unguarded today). No in-app motion setting this wave (offer later if owner wants).
- **Why:** owner M17 "honest signals, no light show" + M3 "disturbing" + Halo "never loop for attention". Shim kept because owner approved it explicitly — the only Halo deviation in loops.
- **Owner-built touched:** S13 onboarding looping miniatures → play-once (owner proud of them; delight kept, endless loop removed) · 9-dot grid (not on proud list) · enhance magic (OB? built under composer revamp; not listed as proud).
- **Reversibility:** cheap (iteration counts / CSS).
- **OWNER-GATE:** no (onboarding + composer boards show it).
- **Unblocks:** motion sweep; perf (paint loops → compositor).

### T14 · Skeleton
- **Options:** Halo shimmer · static bones + Halo hand-off · Rift idiom only (idle cursor + timer).
- **Default:** static bones (`bg-200`, radius 6, no sweep, no breathing loop) shown only after 300 ms (no flash on fast loads) + Halo hand-off (bones fade 360 ms → content `opacity` + 6 px rise 600 ms; blur ≤4 px on bones only, one region at a time). One Skeleton component replaces 2 implementations + 5 shimmer variants.
- **Why:** owner M2 "that little loading blurry block… looks old school" bans traveling sweeps (2026-07-21). Hand-off gives Halo's "skeletons fade, never pop" without the banned band.
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** Hub/Health/MCP/Workspace loading states.

### T15 · Blur / backdrop-filter / glass
- **Options:** keep 16 backdrop-filters · Halo "small areas, once at a time" · none except bones/splash.
- **Default:** zero `backdrop-filter` in the app (16 declarations removed, incl. 3 on fixed layers). Floats = solid `bg-200` + `line-200` + `shadow-pop` (Halo menu recipe). `filter: blur` only: skeleton hand-off (≤4 px), splash mark settle (boot, brand). No per-word blur, no exit blur, no page blur. `will-change` only during FLIP.
- **Why:** WebView2 mis-composites (own rule, 4 comments) + Halo "blur on small areas, once at a time" + owner C7 no stacked atmosphere.
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** composer, menus, HUDs, settings search.

### T16 · Halo extras
- **Options:** Halo delight layer on (spotlight, magnetic, dim, glow, lift, count-up) · all off · selective (signal-bearing only).
- **Default:** Card spotlight OFF everywhere (canvas board 05 shows ONE hub hero card A/B only if owner asks) · Button magnetic OFF · FocusGroup dim OFF · `glow-strong` only under hovered Send + the one primary CTA in dialogs · press `scale .97` on buttons only (not rows/cards/tiles) · hover lift 3 px only on whole-clickable hub cards (project, resume, starter); no lift on rows/tiles/stream · Stat count-up on first data arrival only (600 ms `dur-slow`, not on 60 s window slides) · `RollingNumber` only for the live token counter + diff `+N/−M` (owner-requested odometer M21; monotonic guard + .6 opacity floor kept) · elapsed seconds = plain tabular swap (no roll every second).
- **Why:** owner traveling-light bans (M3 comet "disturbing", C8 accent spotlight retired) + built-then-rejected lesson (M12 §1.8 #2). Rolling digits every second = constant motion in the eye line (ADHD).
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** Hub, Health, composer Send.

### T17 · Tooltips
- **Options:** keep aria-only shim · revive Halo Tooltip as-is (120 ms, inverted bubble) · revive, calmer.
- **Default:** revive via bits-ui Tooltip (collision-aware, portal), calmer than Halo: first-show delay 500 ms, skip-delay 300 ms, hide immediate; surface `bg-200` + `line-200` + `ink` 12/16 + `shadow-pop` (not Halo's inverted white bubble); `z-tooltip` 70. Scope: icon-only buttons, abbreviated numbers (exact value), chart marks, truncated paths. Remove `use:tooltip` where the element has visible text (dead today anyway). Native `title=` in Browser/MCP converge on it. Keep aria-label promotion for icon-only buttons.
- **Why:** pull reason (`tooltip.ts:1-2`, verified) = "themed glass popover read as visual noise" — authorship UNKNOWN. Owner D6 fix (2026-09-18) "abbreviate + exact on tooltip" assumes tooltips paint; AiHealth promises exact values via tooltip; Settings copy says "hover to see" → today the owner's own fix is broken. Slow delay + quiet surface answers "noise".
- **Reversibility:** cheap (one action/component).
- **OWNER-GATE:** no (FYI: reverses a 2026-06-15 pull of unclear authorship).
- **Unblocks:** 174 call-site audit, AiHealth numbers, composer shortcut hints.

### T18 · Syntax colour
- **Options:** keep Shiki github-dark-dimmed · Halo monochrome CodeBlock · muted Halo-palette Shiki theme.
- **Default:** custom Shiki theme "rift-halo" (≤4 hues, low chroma, no green): base `ink-muted`, keywords `accent`, strings `#9cc6ff` (info), numbers/consts `#b79cff` (chart-3), comments `ink-faint` italic, errors danger. Used in Markdown fences, Read results, diffs. Shell command heads = monochrome mono `ink` (kills kind-coloured chrome, owner C9 / DESIGN §7). Config/console/JSON detail blocks = Halo CodeBlock monochrome. CodeBlock chrome (header, copy "Copied"/"Copy failed") for all code surfaces.
- **Why:** owner D3 "show as much detail as possible — that's what tools lack" → syntax colour is detail. Halo's one-signal rule honoured by muting to its own palette; "no green" honoured.
- **Reversibility:** cheap (one theme file).
- **OWNER-GATE:** no (chat board FYI).
- **Unblocks:** chat code, diffs, Read.

### T19 · Navigation
- **Options:** open-chat Tabs strip · sidebar list = tabs (today) · Halo 72 px icon rail vs collapse-to-nothing + mini peek.
- **Default:** no tab strip — sidebar ConversationList stays the tab list (owner L10 "saw it live, wanted it gone", 2026-07-17; zero tab UI accepted). Conflict named: ConsoleX shipped ChatTabs and owner said "cleaner" there — Rift-specific rejection wins for Rift. Sidebar keeps Rift behaviour: resize, collapse-to-nothing, mini peek, pin (owner iterated 3 sessions, L2/L3); peek animates transform, not `top`; collapse = one coordinated move (width + margin + `--rail-w` same 300 ms). Borrow Halo NavItem anatomy for rows (icon 18, active `bg-200` + accent icon, mono count). Footer dock: 5 tiles icon 18 + 12 px label. Naming: "Home" everywhere (tile, registry title, aria-label, Ctrl+1 opens Home page; new chat = Ctrl+N). Diagnostics: link from AI Health header + palette + Settings→System (not in dock). Browser stays a chat dock (Ctrl+Shift+B, palette).
- **Why:** owner calls on tabs + peek are direct and Rift-specific. Naming fix is pure clarity (one name per place, M01 §8 #4).
- **Owner-built touched:** none removed.
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** shell phase.

### T20 · Overlays
- **Options:** Halo overlays as-is (z order, right drawer, bottom-right toasts) · Halo motion/look on Rift placement + z order · keep Rift overlays restyled only.
- **Default:**
  - Dialog: bits-ui Dialog/AlertDialog, Halo `halo-zoom` (scale .94 + 8 px, 300 ms; leave .96 200 ms), width 480, `bg-100` + `line-200` + `shadow-pop` + radius 16, opaque Halo scrim `rgba(4,5,7,.72)`, Cancel-default on destructive (owner K12). Update + MCP + reset confirms (native `confirm()` → Dialog).
  - Command palette: Halo look + `halo-zoom`; keep Rift scorer, group headings, sub-meta, ↵ hint (extend Halo API).
  - Menus/popovers/select: `halo-pop-in` 150 ms from trigger corner, item stagger 18 ms cap 12 (composer mention list stays all-at-once: owner-feel "immediate", M04 #28).
  - Notifications: keep bell + upward popover anchored in sidebar footer (not Halo right drawer); Halo row motion, age buckets kept, add focus trap/return, bell count pop; no bell swing.
  - Toasts: anchor top-centre of main island (owner 2026-07-16 choice, M12 Q14) with Halo stack mechanics (collapsed stack, fan on hover pauses timers, horizontal swipe to dismiss), tones accent/info/warn/danger + neutral.
  - Native webview: G6 — when dock shows a page, opening dialog/palette/menu over it hides the child webview (Rust/IPC lock); toasts constrained to the chat area left of the dock; dock-slide keeps real-width tween (webview sync), 300 ms ease-out.
- **Why:** Rift's popover > dialog order is deliberate (`DESIGN.md:296-298`). Toast centre = owner choice; bottom-right collides with composer (`ToastHost:3-6`). Drawer + scrim for notifications = heavier than the owner's calm chrome.
- **Reversibility:** moderate (overlay lock is Rust).
- **OWNER-GATE:** G6 only.
- **Unblocks:** overlay phase.

### T21 · Chat anatomy
- **Options:** Ledger on Halo tokens + Rift dense tiles · Halo Card/Message components as-is (24 px padding) · Ledger + keep legacy tree alive.
- **Default:**
  - Ledger receipt kept exactly (answer first, one ruled line, hover/focus Copy/Retry, tokens on hover) — owner pick 2026-10-19 (L16). Receipt meta: sans 12 tabular `ink-muted` (owner C13 mono-chrome defect; Halo `caption` is sans for timestamps).
  - User message: right-aligned bubble, `bg-200` + `line-100`, radius `16 16 4 16`, max 560 px / 80 %, 14/22, timestamp + Copy below on hover; no gradient/inset/shadow.
  - Tool blocks: Rift dense Tile on Halo tokens (`bg-100`, `line-100` → `line-200` hover, radius 12, head 32 px, pad 8×12), NOT Halo Card 24 px padding. State: live = LiveDot + accent hairline; failed = danger icon + word (one signal, not border+edge+pill+glyph); ask = accent hairline + "Needs you" badge. Pills → Badge. Glyph colour = status, never kind.
  - Expanded activity (inside receipt): ConsoleX-style hairline rail rows (owner liked that cleanup, M11 §5) for WorkLine groups.
  - HUDs (Plan/Agent): keep both (owner asked, S6), solid `bg-200` float, no blur, `halo-pop-in`.
  - Legacy assistant tree: G3.
  - Stream-mode gap: add `max_tokens` / refusal notice as inline Banner (behaviour exists only in legacy tree).
- **Why:** owner picked Ledger today; D3 detail-first via hierarchy, not hiding; K4 one tile family; K5 one fact one home.
- **Reversibility:** moderate.
- **OWNER-GATE:** G3 only.
- **Unblocks:** chat board 02 + chat phase.

### T22 · Composer anatomy
- **Options:** v4 flat bar BELOW the well (today) · controls INSIDE one card (Halo mock, ConsoleX one-row, Claude Desktop).
- **Default:** controls inside one card: textarea top, one control row bottom (perm · attach · mic · wand | model · ctx ring · send); focus ring + `line-300`→accent border on the card. Single settings popover (model + effort + Fast) kept as Rift composition (owner detent slider "slot-machine feel" kept, K14). Hero: keep FLIP descent and controls materialising mid-flight (owner liked, M19). Send = 32 rounded square radius 8, accent fill, `glow-strong` hover, press .97; Stop = same slot, `danger-soft` fill + square glyph; Queue mode kept. One send ripple ring max (or none; board A/B). Esc-to-stop: implement (Esc stops when streaming AND no menu/enhance/mention/slash layer open) so the existing hint becomes true. Drop copy "Drop image to attach" → "Drop files to attach". Placeholder: one quiet phrase (owner K8).
- **Why:** owner L15 north star = Claude Desktop screenshots (controls inside card); owner's own Halo app shipped one-row inside card → "cleaner". Bar-below was "Claude-Code flat bar" (`Composer.svelte:1906`, verified) — authorship UNKNOWN; board 03 shows both.
- **Owner-built touched:** S5 composer v4 layout (layout only; FLIP, queue, dictation, detent slider, cost disclosure kept).
- **Reversibility:** moderate (markup + FLIP `dy`).
- **OWNER-GATE:** no (board 03 decides; default proceeds).
- **Unblocks:** composer phase.

### T23 · Scrollbars
- **Options:** hidden in stream (today) · Halo thin everywhere · native visible.
- **Default:** Halo thin everywhere (10 px, transparent track, `line-200` thumb with 3 px transparent border, hover `ink-faint`, no arrows), incl. chat stream (today hidden). Hidden only on sideways strips.
- **Why:** owner hates visible/WHITE scrollbars (ConsoleX) — Halo thumb is dark and thin; hidden scrollbars remove orientation (ADHD). ConsoleX restored thin ones; no recorded objection.
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** scroll-region sweep (stream, Settings, Diagnostics, menus).

### T24 · Data viz
- **Options:** Halo Stat/chart kit as-is (en-US formatter, multi-hue) · Halo anatomy + Rift formatter + accent-only series · keep Rift charts recoloured.
- **Default:** Stat anatomy (label → value → detail, delta chip with arrow + sign) on Home + AI Health; Rift formatter (compact `32k`, `$8.2k`, durations) not Halo's `en-US`/`$`. Spend delta up = warn (cost inversion kept), decline = neutral. Hero numeral 38 px gradient → metric 28 px, no gradient text. Daily chart: Rift dense bars (14/30/60), all `bg-300`, peak `accent`, hover tooltip + sibling dim .45, bars grow on first load. Model mix: `ShareBar` top 3 in accent / `#b79cff` / `#9cc6ff` + neutral "Other"; legend with values. Per-project hues → neutral initials on `bg-300`, accent only on active project. Spend sparkline → Halo Sparkline draw-in. Plan limits → Progress (scaleX), tone by zone.
- **Why:** owner C2 "a second decorative hue is a bug"; per-project hues PENDING (C15, built under delegation, accepted once) → neutral default, board 05 A/B. D7 oversized numerals = AI-slop tell.
- **Owner-built touched:** per-project hue monograms (C15) → neutral.
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** Home + AI Health ports (board 05/07).

### T25 · Process
- **Options:** canvas boards then port · port then show in-app · parallel (board + port same phase).
- **Default:**
  - Canvas project "Rift · Halo" (Claude Design, Halo installed), boards: 00 Foundations (type/density specimen; opens with Q21) · 01 Shell frame A/B · 02 Chat (Ledger, tiles, user bubble, diff/ANSI colour) · 03 Composer (inside vs below) · 04 Motion lab (T11 + page/overlay motion) · 05 Home · 06 Settings · 07 Health/Diagnostics · 08 Overlays (dialog, palette, menu, toast, tooltip). Boards = interactive Play mode (ConsoleX `renderVals` precedent). Plain markup + Halo tokens, not mounted React components.
  - Order: T8 tokens/fonts → `ui/` kit + `/dev/ui` specimen → shell → chat (stream, pane, composer) → overlays → Home/onboarding → Settings → Health/Diagnostics/browser. Board N signed before port N (00+01 = G1).
  - Branch: `design/halo` cut from `design/chat-ledger` HEAD `22727d24` (keeps Ledger `8bb71f4f`); pacing constants from `22727d24` marked unconfirmed (G4). No push/merge without asking (G7). CI only runs on `main` → `npm run verify:frontend` locally before any merge.
  - Commits: one per phase step, motion changes in separate revertable commits.
  - Verify per phase: `npm run check` + `npm test`; CDP `baseline` before / `diff` after on `tour home chat settings ai-health`, `measure` probes for exact token values, `/dev/gallery` for stream, `/dev/ui` for primitives; one console-error check. No DOM tests exist → CDP is the net.
  - Review loop for owner: a no-server demo build published as a private artifact (ConsoleX `build:demo` precedent) instead of asking him to run a dev server.
  - Docs: mark `redesign-plan.md` + `HANDOFF.md` pacing sections superseded by this spec at session close; rejection → DESIGN §8 same session.
- **Why:** owner designs in canvas, finds dev server "irritating", wants to "really see pages" before porting (ConsoleX: "or it gets done twice").
- **Reversibility:** n/a.
- **OWNER-GATE:** G1, G7.
- **Unblocks:** sequencing of every phase.

### T26 · Logo / splash / brand
- **Options:** keep Rift mark + seam splash recoloured · Halo Logo + no splash · new Halo-native splash.
- **Default:** keep Rift mark (PNG) everywhere (not Halo Logo); brand name Geist 600 −.03em in sidebar head. Splash: keep seam-parting doors + real boot stages (owner-requested rework, M22; honest signals S13); recolour to `bg-000`; keep Lexend RIFT letters; kill caret blink + exit blur; doors curve kept as the one non-Halo curve. Seam motif lives only here.
- **Why:** owner "needs a whole rework" → delivered seam splash (verdict pending, not rejected). Halo has no splash and says nothing about replacing a product's logo.
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** boot/splash pass, sidebar brand head.

### T27 · Text ladder, contrast, focus ring (added)
- **Options:** keep 5 Rift text steps · Halo 3 steps · 3 steps + faint restricted to meta.
- **Default:** 5 text steps → 3 + one bridge: `--fg`/`--fg-2` → `ink` #eceff3 · `--fg-muted`/`--fg-subtle` → `ink-muted` #9aa2af · `--fg-faint` → `ink-faint` #7d8592, used ONLY for timestamps, section labels, axis ticks, never on `bg-300`, never body copy (247 refs triage; descriptive copy → `ink-muted`). Control borders `line-300` (3:1). Focus = Halo `shadow-focus` (2 px gap + 2 px solid accent) on every control, focus-visible only; card focus ring hugs the card radius. Fix invisible focus (hue slider, `.seg`, `.sw`, AiHealth buttons).
- **Why:** owner C6 "quiet ≠ invisible" + C14 lifted faint for legibility; Halo floor 4.5:1 fixes Rift's 1.57:1 meta (M12 X15).
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** 247-ref `fg-faint` triage sweep + focus-ring sweep.

### T28 · Page shell layout (added)
- **Options:** per-page headers/widths (today) · Halo PageHeader + stacked Field rows · shared PageHeader + one column + horizontal SettingsRow.
- **Default:** one shared PageHeader (Rift part: title 24 + sub 13 `ink-muted`, optional SectionLabel eyebrow) and ONE column per page with one left edge (fixes 4-edge finding L18): Settings 820, Health/Home 1080 (2-col ≥1160 inside same edge), Diagnostics 1080. Numbered SectionLabels (`01 / PROJECTS`) on Home/Health/Settings with index in `ink-faint` (not accent: accent-count rule), never in chat. Settings keeps horizontal `SettingsRow` (title + description left, control right; Halo has no equivalent; stacked Field would ~2× page height). Fit-one-screen: Home + Health aim to fit 1600×1000 with lists scrolling inside cards; Settings tabs scroll (accordions for advanced). Live preview wells: drop fake window dots, max one level of nesting.
- **Why:** owner L18 four-edges imprecision; L5 one level deep; K13 fake chrome flagged.
- **Reversibility:** cheap-moderate.
- **OWNER-GATE:** no.
- **Unblocks:** Settings, Home, Health, Diagnostics page ports.

### T29 · Copy, icons, hit targets, hover-only (added)
- **Options:** leave copy/icons as-is · Halo content rules + lucide 1.8 + ≥28 px targets · Halo rules incl. 44 px targets.
- **Default:** sentence case everywhere ("Claude usage & health", drop "Experimental" eyebrow); no emoji/glyph text in UI (`📋`, `✓ done`, `ℹ`, `⚠` → lucide icons); copy failures shown ("Copy failed", danger) not swallowed. Icons lucide stroke 1.8, 16 px beside text, 18 px in nav. Hit targets ≥28 px (compact) — 18-20 px copy/kill buttons grow. Hover-only actions allowed only where owner chose them (Ledger Copy/Retry) and always also on `:focus-within`; card actions visible at rest at reduced opacity (existing hint-at-rest rule). Native colour → T8 step 6.
- **Why:** owner P1/P3 honest, plain copy; T8 sentence case; Halo content rules agree.
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** copy/emoji sweep, icon-size + hit-target sweep.

### T30 · Accessibility carry-overs (added)
- **Options:** port selectors as-is (bug stays) · re-point reading-comfort to stream prose · drop reading-comfort modes.
- **Default:** reading-comfort selectors (`[data-a11y-*] .bubble/.markdown-body`) re-pointed to stream prose (`.sturn .md`) — today dyslexia line-height/warm tint never reach real chat prose (M03 §3, a real bug); Settings preview must use the same selectors. Lexend override keeps priority over Halo `font:` shorthands on buttons/segmented. Reduced-motion probe reactive (T13). Onboarding overlay gets real focus trap or loses `aria-modal`; Esc on step 1 goes back/does nothing instead of dismissing setup + acknowledging beta (owner can override).
- **Why:** dyslexia mode is an owner feature (T7 in M12); fixing reach is restoring his feature, not adding one.
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** dyslexia/reading-comfort in real chat, onboarding overlay a11y fix.

### T31 · Entrance replay + keep-alive pages (added)
- **Options:** replay entrance on every mount (today) · once per mount, new content only + swap guard · no entrances.
- **Default:** entrances run once per mount AND only for genuinely new content; remounts from tab switch, split swap, history hydration, `{#key}` redraws, 60 s data ticks never replay (swap guard: zero durations ~450 ms around re-parenting, ConsoleX `51f4d02`). Keep-alive pages: first open = staggered section rise; later activations = 520 ms content glide; hidden pages pause loops (`animation-play-state: paused` / `content-visibility` on inactive `.ws-page`).
- **Why:** M09 §6.6 + M11 §4 #1 recurring bug class; replaying motion on every switch = motion for attention (Halo ban, owner M24).
- **Reversibility:** cheap.
- **OWNER-GATE:** no.
- **Unblocks:** tab/split/page-switch motion; T12 history hydration.

---

## 3. Dependencies (read before sequencing)

- T8 first. T2/T3/T7/T10/T27 are "cheap to reverse" only after T8's alias layer exists.
- T5 + T6 before any surface measurement (text widths, `ROW_H`, FLIP `dy`).
- T9 `ui/` kit before overlays (T20) and Settings (T28).
- T11 mechanism + T12 before chat sign-off; G4 before pacing numbers ship.
- G6 Rust lock before any overlay that can cover the browser dock.

## 4. Open questions to put on canvas boards (not gates)

1. Q21 (board 00, first): which parts of the first Halo mock felt "rough"? Shapes T1/T6/T22.
2. Board 01: frame A islands-flat vs B Halo AppShell (C slab thumbnail for reference).
3. Board 02: diff add blue vs green; ANSI green mapping; receipt meta sans vs mono.
4. Board 03: controls inside card vs bar below; send ripple single ring vs none.
5. Board 05: per-project initials neutral vs hue; spotlight on one hero card yes/no.
6. Board 08: tooltip quiet surface vs Halo inverted bubble.
