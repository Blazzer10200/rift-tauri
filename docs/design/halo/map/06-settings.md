# 06 — Settings surface map (Halo redesign, MAP ONLY)

Scope: `SettingsPage.svelte` + its control kit. Read-only survey, branch `design/chat-ledger`. All cites `path:line`; `SP` = `src/lib/components/settings/SettingsPage.svelte`; `HB` = Halo `components/bundle.css`; `HD` = Halo `components/index.d.ts`; `HT` = Halo `tokens.css` (consolex copy). Anything not read = "unknown".

Evidence base: SP read in full (1-2023). Not run, not screenshotted live; only visual evidence is the pre-Halo audit shot `docs/design/redesign-audit/05-settings.jpeg` (Graphite Ink era).

---

## 1 Files & roles

| File | Lines | Role |
|---|---|---|
| `SP` | 2023 | Whole surface. Script 1-620 (state, search index, handlers), template 622-1582, scoped `<style>` 1584-2022 (439 lines of CSS). Five tabs rendered with `{#if activeSec === …}` (714, 811, 941, 1226, 1429). |
| `src/lib/styles/settings-controls.css` | 37 | Global (unscoped) `.st-*` kit: `.st-input/.st-secret/.st-eye` (7-13), `.st-btn` (+`.primary/.icon/.danger-btn`, `.st-spin`) (16-27), `.st-pill` (+`.ok/.warn/.accent`) (30-37). Imported by `SP:2`. Header comment says SP + "LocalLlmPage" share it (settings-controls.css:2); grep finds no LocalLlmPage anywhere in `src/` — stale comment, SP is the only consumer of `.st-*` (only other hits: unrelated `.cost-pill` in MessageBubble). |
| `src/lib/components/shared/Select.svelte` | 56 | Thin wrapper over `ui/select/*` (bits-ui). Maps `""` to sentinel `__rift_empty__` (22-30). Used ONCE in SP (1356, mic device). |
| `src/lib/actions/sliderBubble.ts` | 81 | Svelte action: floating value bubble on `document.body` while dragging/keyboard-nudging a range; hard-codes `thumbW = 18` (28). Used on 3 sliders (SP 729, 736, 761). Its CSS lives in `src/app.css:369-392` (`.slider-bubble`, `input.slider-zoom::-webkit-slider-thumb`). |
| `src/lib/components/shared/PageHero.svelte` | 48 | Eyebrow/title/sub header + `children` snippet used for the tab row (SP 626). Scoped CSS: `.sb-topbar` pad `30px 40px 0` (PageHero:37), `.sb-hero` `max-width: var(--hero-w)` (38), title 24px/760 (45). Shared with other pages (not mapped here). |
| `src/lib/components/ui/button/button.svelte` | 85 | tv() variants secondary/ghost/soft/primary/danger/warn/info/link, sizes h-6.5…size-8 (button.svelte:6-35). **SP does not use it** — SP rolls its own `.st-btn` and bare `<button>`s. |
| `src/lib/components/ui/select/*` (12 files, 7-44 lines each) | ~290 | bits-ui Select, styled by `recipes.ts`. Reached only via `shared/Select.svelte`. |
| `ui/separator/separator.svelte` | 21 | `bg-border h-px`. **Unused by SP** (SP draws dividers with `border-bottom` on `.ctl-row`, `.ap-divider`, `.card-divider`). |
| `ui/dropdown-menu/*`, `ui/context-menu/*`, `ui/alert-dialog/*` | 54+52+40 idx | **Not used by SP.** SP's destructive confirms use the native Tauri `confirm()` (SP 337, 354) not `alert-dialog`. Listed only because the owner scoped them in; no Settings coupling. |
| `ui/recipes.ts` | 60 | Single source of class strings for floating panel/menu/field/dialog. `fieldTrigger` = `h-7 … rounded-sm border-border-strong bg-bg-elev-1 text-base …` (recipes.ts:52-53). Header says DESIGN.md §10 documents the rules (recipes.ts:5). |
| `src/app.css` (shared deps) | — | `.rift-toggle` (476-493, 16 uses in SP), `.slider-bubble` (371-392), global `.card` (496-500, shadowed by SP's scoped `.card` at SP:1689), `.pill` (449-470, unused by SP; SP uses `.st-pill`), `.btn` (394-421, unused by SP), `.input` (423-434, unused by SP; SP uses `.st-input`), density vars (289-291), `[data-slot]` reduced-motion (120-122), a11y `data-a11y-*` (739-777), `.spin` (628). |
| `src/lib/state/ui-prefs.svelte.ts` | — | Source of ACCENTS (8 hues, 84-93), VIVIDNESS 0.05-0.26 (78-79), UI_SCALE 0.8-1.5 (19-20), DENSITY_PRESETS (65-69), TOOL_DETAILS (54-58); writes `--accent-h/--accent-c` on root (293-294). |
| `design-system/DESIGN.md` | — | Current taste rulebook (Graphite Ink, "solid accent is RARE", motion 140/240/460/62 — DESIGN.md:97-102). Halo supersedes it; Settings code cites it indirectly via recipes.ts. |
| `design-system/check-tokens.mjs`, `check-ui.mjs`, `ui-baseline.json` | — | Gates; see §9. |

Surface inventory (counts from grep of SP): 16 `.rift-toggle` (+1 decorative demo at 746), 10 `.seg` groups / 14 `role="radiogroup"`, 22 `.st-btn`, 10 `.st-pill`, 4 `.st-input`, 3 `type="range"`, 2 `<textarea>`, 3 `<details>`, 1 `<Select>`, 5 `.code-preview` wells, 42 `.ctl-row`, 19 `use:tooltip` lines, ~60 `class="card…"` greps (many are `card-tt`/`card-sub`; actual card containers ≈ 20).

Settings groups (tab → card id → controls):

| Tab | Card (id) | Controls |
|---|---|---|
| Appearance (714-809) | Accent color (`card-accent` 717) | 8 swatch buttons (721-727), hue range 0-360 w/ rainbow track (729), vividness range (736), live "accent in action" well (742-752) |
| | Interface & code (`card-interface` 755) | UI scale range (761), density seg ×3 (767), code font size seg ×4 (775), tab width seg ×2 (781), ligatures toggle (787), live density+code preview (793-804) |
| | — | "Reset Appearance" dashed button (807) |
| Chat (811-939) | Chat rendering (`card-rendering` 815) | Stream view toggle (820); if ON: Process detail seg (825), Tool detail seg (833), Narration seg (841), Command output seg (849, disabled when toolDetail="detailed"), live stream preview (858-883); if OFF: classic bubbles preview (887-894) |
| | Reading comfort (`card-comfort` 898) | Dyslexia toggle (903), Font seg Inter/Lexend (907, gated), Wider spacing toggle (914, gated), Warm tint toggle (918), Autocorrect toggle (922), reply preview (927-934) |
| | — | Reset Chat (937) |
| Providers (941-1224) | Provider switcher (947-958) | 2 card-tabs ChatGPT / Claude each w/ status pill; `providerFocus` state (68) |
| | Claude: status banner (`card-claude` 966-1041) | CircleCheck/Loader icon, summary, CLI warn/note lines, CLI chip, update/check pills, "checked Ns ago", Check again btn, multi-install list (999-1019), Update now + copyable command + err/ok/stuck lines (1020-1040) |
| | Claude session (`card-session` 1048) | Full-config toggle (1053), Git tools seg ×2 (1057), Output style text input + datalist + Save/Clear (1063-1072), Context limit seg 200K/1M (1076), plan gauge (1082-1088) |
| | Optional Claude API (`card-api` 1091, `<details>`) | route strip, API key secret field + Save / Configured pill + Clear (1102-1116), env-key warning, per-turn cost cap number + Save/Clear (only when key set, 1125-1137) |
| | — | Reset Claude preferences (1140) |
| | ChatGPT account (`card-codex` 1147) | route strip, plan label/email, status pill, Connect, Check again, rate-limit snapshot grid, notes |
| | Optional ChatGPT API (`card-chatgpt` 1185, `<details>`) | route strip + model count, key secret field + Connect / pill + Clear, notes |
| Voice (1226-1427) | Engine (`card-engine` 1228) | STT master toggle (1233), engine 3-up pick grid (1237), "Whisper not in build" info + nested `<details>` (1246-1260) |
| | Web Speech (1265, cond) | language pick grid (3 shown / 12 via `set-expand`) (1270-1281), continuous toggle |
| | Parakeet / Whisper model (1329 / 1339, cond) | `modelRows` snippet (1290-1326): name, filename, size, download progress bar, Use / Active pill / Delete / Download / Cancel |
| | Capture & cleanup (1349, cond local engine) | `<Select>` mic (1356) + refresh icon btn, cleanup toggle, beam-search toggle (whisper) |
| | Vocabulary priming (1381, cond whisper) | 2 textareas (1386, 1390) |
| | warnings (1395-1398) | `.st-warn` blocks |
| | Composer integration (`card-composer` 1400) | 3 toggles (1405-1413), auto-stop pick grid Off/3/5/10s (1417) |
| | — | Reset Speech (1425) |
| System (1429-1579) | Administrator access (`card-admin` 1432, cond `elevation.supported`) | status + Relaunch btn / "Administrator" live label, always-elevated toggle |
| | Build & install (`card-build` 1456) | version identity block, Check for updates, stack chips (1478), Config/Logs path rows + Open, Source row, "Works with Claude + ChatGPT" |
| | Keyboard shortcuts (`card-shortcuts` 1501) | 4-of-9 rows + expander (1504-1516) |
| | Local tools (`card-tools` 1519) | installed chips strip + Re-check, missing tool rows + Install, install notes |
| | Help & diagnostics (`card-help` 1554) | 5 `.st-about-row` buttons (diagnostics, copy diag, report bug, replay walkthrough, repair) |

Cross-cutting: settings search (combobox + listbox, 656-701; index of 43 entries at 106-150); per-tab factory reset via native confirm (331-351; Providers/Voice/Chat/Appearance only, System has none).

---

## 2 Layout & anatomy

Regions (top → bottom, SP):
1. `.sb-main` (1587) flex column, transparent bg so the app dot-field shows through; fills the main island.
2. `PageHero` (626): eyebrow "SETTINGS" + 24px title (= active tab label) + sub (per-tab, 59-65). `padBottom=false`, `maxWidth=820`.
3. `.tabrow` (628-702, CSS 1593): `.tabnav` role=tablist (5 buttons w/ 15px icon + label + optional status dot) + one sliding `.tab-ink` (630, CSS 1595) + search pill (`.sset-search` 210px → 250px on focus, 1617-1618). Hairline `border-bottom` on the row.
4. `.surface-body` (706-712, CSS 1642): the single scroll container (`overflow-y:auto`, `scroll-behavior:smooth`), `role=tabpanel`.
5. `.set-surface` (CSS 1650): `max-width:820; margin:0 auto; padding:26px 40px 48px` → `.set-col` → stacked cards.

Column geometry: hero = `max-width:820` with **no** inner padding (PageHero:38) while body = `max-width:820` **plus** `padding: 0 40px` (SP:1650) → body cards are 80px narrower than the hero/tab row. This is the likely source of the "header x=727 vs cards x=765 (38px offset)" in `docs/design/redesign-plan.md:83-84` — likely, verify in app. (The audit screenshot agrees: header left edge ≈727, card left edge ≈765.)

Card anatomy (`.card` SP:1689): `padding:16px 18px`, `border-radius:12px`, `background: var(--island-fill)` (2.8% fg tint, app.css:212), `1px solid var(--island-border)`, hover brightens border. Inner: `.card-tt` 14px/650 (1798), `.card-sub` 11.5px (1799), then `.ctl-row`s (1806): **horizontal rows** — label block left (`.ctl-t` 13px/500 + `.ctl-s` 11.5px), control right, `padding:9px 0` (`.tight`), hairline between rows. `.stack` variant puts control under label (1809). Wells (`.code-preview`, 1769) nested inside cards: card → well → inner stage = 3 surface levels (the audit already flagged this, redesign-plan.md:86-88).

Z-order: `.sset-results` `z-index:30` raw number (SP:1624); `.slider-bubble` `z-index:9999` (app.css:372); Select content `z-(--z-popover)` 9000 (recipes.ts:14); no other stacking in SP. Halo bans raw z numbers (`02-type-space-and-layout.md`, Layers).

Breakpoints: `@media (max-width: 900px)` (SP:2000) — hides tab icons, tab row wraps, search goes full-width, provider switcher 1-col, ctl-rows wrap; `@media (max-width: 560px)` (2015) — hides provider pills, pick grids 1-col, `.ctl-actions` full width. `tauri.conf.json:18-19` minWidth 900 / minHeight 600. **The 900px block matches at the minimum window width** (inclusive) so the min-size window always renders the "narrow" layout; the **560px block is unreachable** with the 1.5× UI-scale cap (900/1.5 = 600 CSS px > 560) unless the window is allowed below 900 by something I did not find (unknown). Both depend on zoom via `webview.setZoom` (ui-prefs.svelte.ts:17-19), so UI scale directly toggles which layout you get.

Provider tab layout: two `.provider-switch` cards (CSS 1657-1665) then runtime section heads (`.provider-section-head`, 1652-1655: 11px/700 uppercase 0.09em) — an ad-hoc SectionLabel.

---

## 3 States

| State | Where | Notes |
|---|---|---|
| Empty | `stt.models.length===0` → "Loading model catalog…" (1325); `configDir/logDir` → "—" (1484, 1488); no installed tools → strip hidden (1526); search with no hits → **nothing renders** (676, no "No matches" row) |
| Loading | Claude banner: `Loader2 class="spin"` + "Checking session…" (969-972); `cliUpdate.status==="checking"` pill (985); "Checking account…" in switcher subs (950, 955); Install "Installing…" spinner (1539); `st-btn` Saving…/Updating…/Relaunching… (1023, 1067, 1113, 1130, 1210, 1444). **Tone-while-loading bug:** `.sb-status {assistantDot ?? 'ok'}` (966) paints the green/ok banner before auth resolves (assistantDot is undefined until `auth.pill` exists, 559-562). |
| Live / streaming | Download progress: `set-progress-fill` width % + mono label (1303-1306, CSS 1986-1988); pill dot one-shot ping when probe lands (`{#key authPulseKey}` 956; CSS 1602); relative "checked Ns ago" tick every 30s (610-611) |
| Error | `st-warn` (1396, 1398), `st-note.warn` (1181-1182, 1215), `st-cli-err` red pre-wrap (1033), `.st-cli-warn` (975, 1038), `aria`-less save-failure strings (493, 508, 522, 543) shown via `.st-note` |
| Hover | `.card:hover` border→strong (1690); `.sw:hover` translateY(-2px) (1836); `.set-pick:hover`, `.st-about-row:hover`, `.provider-switch:hover`, `.set-expand:hover` (warn tint for reset, 1788) |
| Focus | `.st-input:focus` 3px `--ring` (settings-controls.css:8); `.rift-toggle:focus-visible` ring (app.css:484); `.set-range:focus-visible` thumb ring (1853); **`.hue-range:focus {outline:none}` (1843) with no `:focus-visible` replacement → keyboard focus on the hue slider is invisible**; `.seg button` and `.sw`, `.set-pick`, `.snav`, `.provider-switch`, `.st-about-row` have **no focus-visible rule in SP** (rely on UA default outline — unverified how it renders on the dark bg); search results use `data-active` highlight + aria-activedescendant (679-697) |
| Disabled | `.st-btn:disabled` .45 (settings-controls.css:18); `.rift-toggle:disabled` .5 (app.css:483); `.seg button:disabled` .5 (1829); `.set-pick:disabled` .5 (1978); `.ctl-row[data-disabled]` .5 (1810, sub-rows under dyslexia toggle 905, 912); `.ctl-row.overridden` dims label side (1813, command output when toolDetail=detailed 847); textarea .55 (1993). Three different disabled opacities (.45 / .5 / .55) vs Halo single `--opacity-disabled` .45 |
| Collapsed / expanded | `<details>` ×3 (1091, 1185, 1250) native, **no height animation**; summary "Show/Hide" via `::after` content (1669-1670); `set-expand` list expanders w/ chevron flip (1278, 1513; CSS 1781-1784); search-jump into closed `<details>` does **not** open it (`jumpTo` 184-209 never touches `open`) so "API-key fallback" / "Per-turn cost cap" / "ChatGPT API access" search hits (SP 129-132) land on a collapsed summary |
| Conditional content | Stream view ON/OFF swaps 4 rows + preview (822-895); `providerFocus` swaps whole Claude vs ChatGPT stack (960/1141); `hasApiKey` swaps key field vs Configured pill (1105) and shows cost cap (1125); engine-dependent Voice cards (1264, 1328, 1338, 1348, 1380); admin card only when `elevation.supported` (1431) |
| Narrow window | see §2 breakpoints |
| Reduced motion | handled per-rule in SP (1607-1610, 1643-1647, 1684, 1797) and `[data-slot]` kill (app.css:120-122); JS paths use `prefersReducedMotion()` (194); **sliderBubble.ts has no reduced-motion branch of its own** (CSS reduces it, app.css:389-392) |
| Dyslexia/warm tint a11y | `data-a11y-*` rules target `.bubble/.markdown-body` (app.css:739-777); the Reading-comfort preview reuses them (930) — coupling to keep |

---

## 4 Motion present

Rift tokens (app.css:240-250): `--dur-fast 140`, `--dur-base 240`, `--dur-slow 520`, `--dur-rise 460`, `--stagger 62`, `--ease-page cubic-bezier(.22,1,.36,1)`, `--ease-soft`, `--ease-spring`. Halo (HT): `--dur-fast 150`, `--dur-base 300`, `--dur-slow 600`, `--stagger 60`, `--ease-out cubic-bezier(.2,.8,.2,1)`, `--ease-spring cubic-bezier(.34,1.56,.64,1)`. Different curve + base 240→300 + slow 520→600; spring identical.

| Cite | What moves | Property | Duration · easing | Trigger | Verdict vs Halo |
|---|---|---|---|---|---|
| SP:1595-1596 | `.tab-ink` underline slides between tabs | transform + width | base · ease-page | tab change / ResizeObserver | **KEEP pattern** (Halo `.halo-tabs__bar` same: transform+width, dur-base ease-out, HB:266). Retune tokens only. JS measure `measureInk` 78-95 |
| SP:1676, 1590 | cards enter on every tab mount: opacity 0→1, translateY 8px | opacity+transform | base (240) · ease-page; delays 0/50/100/150/190 ms (1680-1683) | tab switch (`{#if}` remount) | **RETUNE**: Halo rise = 14px, dur-slow, stagger 60 (`@keyframes halo-rise` HB:23; 03-motion "Entrance"). Cap stagger ~12 items. Note `nth-child(n+5)` clamps at 190ms |
| SP:1792-1796 | revealed list rows cascade (`anim-reveal`) opacity + translateY 7px + scale .985 | opacity+transform | base · ease-page, 22ms × min(ti,16) | expanding language/shortcut lists | **RETUNE/KEEP**: Halo caps stagger; fine. Currently runs on **initial** render of shortcuts/languages too (applied unconditionally 1272, 1505) |
| SP:1635-1640 | search-jump landing flash on target card | **box-shadow** (ring + glow) | 1600ms · ease-page, removed by JS timer 1700ms (203-207) | `jumpTo` | **KILL or REPLACE**: animates box-shadow (not compositor-only; Halo rule "only opacity and transform run continuously", 03-motion). It is one-shot, so low perf risk, but Halo has no equivalent; candidate: border-color/outline fade or `shadow-focus` pulse. Also `scrollIntoView({behavior:"smooth"})` 195 |
| SP:1602-1606 | status-pill dot "ping" once | **box-shadow** | 900ms · ease-page, 1 iteration | pill mount / `authPulseKey` re-key | **RETUNE → Halo LiveDot ping** (`halo-ping`: transform scale 1→3 + opacity, compositor-safe, HB:82). Today it is a box-shadow animation. Applied to `.st-pill.ok .dot` including model/CLI pills |
| SP:1733 | context gauge fill | **width** (layout) | 420ms · ease-page | plan 200K↔1M | **RETUNE → Halo Progress** (bar uses transform scaleX, HB:459) |
| SP:1987 | download progress fill | **width** | fast · linear | progress events | **RETUNE → Halo Progress** (eases each value + mono % counts) |
| SP:1762 | density preview rows | **height** (`--row-h` var) | base · ease-page | density change | KEEP (a preview of real density); layout-property but tiny; revisit if density survives (see §10 Q2) |
| SP:1835-1836 | swatch hover lift | transform translateY(-2px) | fast | hover | KEEP (Halo cards lift 3px on hover, 03-motion row 1) |
| SP:1841-1842, 1850-1851 | slider thumb scale on hover | transform 1.12 / 1.14 | fast · ease-page | hover | **RETUNE → Halo thumb**: spring ease, hover 1.15, active 1.3 (HB:181-184) |
| app.css:371-392 + sliderBubble.ts | floating value bubble + thumb zoom 1.35 while dragging | opacity+transform (bubble); transform (thumb) | fast · ease-out/ease-page | pointerdown/input/keyboard | **OWNER CALL (kill vs keep)**: Halo Slider shows value inline beside label (HB:175-176) and has no bubble. Bubble is a real "focus mode" affordance (fine adjust on a 150px track). Also `.slider-zoom` (1.35) conflicts with `.set-range:hover` 1.14 / Halo's 1.3 press scale — specificity winner unverified |
| app.css:476-493 | toggle knob | transform translateX(17px), bg | base · ease-page; track bg/border base | click | **RETUNE → Halo Toggle**: ease-spring + knob widens to 24px while pressed (HB:142, 145) |
| SP:1826 (`.seg button`) | active pill appears | background snaps with fast transition; **no sliding thumb** | fast | click | **RETUNE → Halo Segmented**: accent thumb slides, dur-base (HB:92). Needs JS measure like `.tab-ink` |
| SP:1972-1974 | pick-grid tile active | bg/border | fast | click | → RadioGroup cards (HB radio dot springs, 566-570) |
| SP:1783-1784 | expander chevron flip | transform rotate(180) | fast · ease-page | click | KEEP |
| SP:1895-1896 | `.st-dev` summary chevron | transform rotate(90) | fast · ease-out | open | → Accordion (chev rotate 180, HB:497-498) |
| `<details>` ×3 | open/close | **none** (instant) | — | click | **ADD**: Halo Accordion = grid-template-rows 0fr→1fr dur-base + content fades up with 60ms delay (HB:499-504) |
| SP:1624, 1617 | results popup (no enter motion); search width 210→250 | width (layout) | 180ms · ease-page | focus | search width: KEEP-ish (small area); popup: **ADD** Halo `halo-pop-in` (HB:411, scale .96 + 4px) |
| settings-controls.css:25-27 | `.st-spin` loader | rotate | 0.8s linear ∞ | busy | KEEP (Halo: loops allowed only for real activity; Spinner). `Loader2.spin` (SP 969, 1539; app.css:628) same |
| SP:1611, 1597 etc. | tab icon/label colour | color | fast | hover/select | KEEP |
| SP:1642 | scroll container | `scroll-behavior:smooth` | UA | programmatic scroll (tab change uses `scrollTo({top:0})` 225 = smooth!) | **RETUNE**: tab switch smooth-scrolls to top while content remounts/animates → double motion. Reduced-motion override present (1646) |
| tab remount | tab content swap | no exit; entrance only | — | `{#if}` | **GAP vs Halo `halo-page` page-change** (halo-page-in 10px/520ms, HB:25) — decide if tab swap gets one container-level entrance instead of per-card |
| ui/recipes.ts:19-20, 58-60 | Select popup + dialogs: tw-animate fade/zoom-95/slide-2, dur-fast | opacity+transform | dur-fast · `ease-page` | open/close | → Halo `halo-pop-in` 4px/.96 on `dur-base`; menu items stagger (HB:420). Reduced-motion via `[data-slot]` (app.css:120) |

Count of keyframes in SP: 4 (`blockIn`, `pill-ping`, `sflash`, `reveal-in`) + `.st-spin` + `.spin`. Reduced-motion coverage is good in SP; note the dead `.bg-opt` selector inside the reduce block (1797).

---

## 5 Styling wiring

Dialects on ONE page (the headline finding):
1. **Scoped Svelte `<style>`** in SP — 439 lines, ~all layout/skin (1584-2022), with 9 `:global()` escapes (e.g. 1611, 1619, 1632, 1635, 1645, 1783-1784, 1998).
2. **Global shared CSS** `settings-controls.css` (`.st-*`) + `app.css` (`.rift-toggle`, `.slider-bubble`).
3. **Tailwind v4 via `ui/recipes.ts` + tailwind-variants** (Select + button), vocabulary from the `@theme` bridge (app.css:70-104).
4. **Inline `style=`** (13 occurrences: 630, 718, 723, 736, 761, 1065, 1085, 1110, 1129, 1207, 1272, 1304, 1505) — dynamic values (ink offset, fill %, `--ti`) plus three hard-coded `max-width` on inputs (1065/1110/1129/1207).
5. Rift tokens everywhere; **zero Halo tokens**.

Token usage (SP template+style): `var(--accent*)` 73 refs, `color-mix(` 69, `var(--warn*)` 26, `var(--ok)` 13 + `--ok-soft` 4, plus fg/border/surface ladder. `color-mix` is the main way tints are made (Halo's answer: `*-soft` tokens + one `line-*` ladder).

Hard-coded literals (SP): hex — real: `#d97757` (1944, Claude-brand tint on the Terminal icon); regex false-positives `&#123; &#125;` (803) count against the `color:hex` ratchet. `rgba(0,0,0,0.82)` (724, check-mark on swatch). `oklch(<number>` literals: **14** (718, 723, 1624, 1801, 1840 ×9 hue-spectrum stops, 1841) — of the repo's 131 baseline (`design-system/ui-baseline.json`). 0 arbitrary Tailwind values in SP.

Font sizes in SP `<style>` (counted by grep, 1584-2023):
- px literals: **10 distinct** — 9 (×3), 10 (×12), 10.5 (×5), 11 (×12), 11.5 (×7), 12 (×9), 12.5 (×1), 13 (×5), 14 (×2), 16 (×1) = 57 declarations
- `var(--fs-*)`: xs ×13, sm ×4, md ×2 = 19; `0.92em` ×1
- plus 3 `var(--fs-*)` in settings-controls.css (7, 16, 30), `slider-bubble` 14px (app.css:379), PageHero 24px/10px/`--fs-sm` (PageHero:43, 45, 46), `recipes.ts` `text-base/xs/2xs`
- Font weights used: 400, 500, 550, 600 (×12), 640, 650, 680, 700 → **8 distinct** (Halo: 400/500/600 only per `font:` shorthands, HB:43, 63, 85, 93, 124, 261)
- Letter-spacing: -0.01em, 0.03, 0.04, 0.05, 0.09em (5 values) + `0.005em` body (app.css:314); Halo `.halo-section` 0.12em (HB:85).
- Result matches the pre-Halo audit (`redesign-plan.md:79-82`: "10 font sizes… 12 radii"): **the type scale has not been repaired since**.

Radius spread in SP `<style>` (65 declarations): 999px ×14, 4px ×8, 50% ×7, `var(--radius)` (8px) ×6, 12px ×5, `var(--r-card)` (12px) ×4, 8px ×4, 9px ×3, 7px ×3, 10px ×3, 6px ×2, 5px/3px/2px/11px ×1 each, plus corner shorthands `2px 2px 0 0` and `0 6px 6px 0`. → 17 distinct forms / **13 distinct resolved values** (2,3,4,5,6,7,8,9,10,11,12,pill,50%). settings-controls.css adds 6px, 999 ×2, `var(--radius)` ×2. Halo ladder = `sm 8 / md 12 / lg 16 / pill` (HT). Mapping suggestion: 12 (card)→`lg`? Halo card is `radius-lg` 16, controls/popups `md` 12, small parts `sm` 8; so Rift 12px cards → 16, 8-9px controls → 12, 4-7 → 8. (Flag: 9px/7px/11px/5px/3px/2px are off any ladder.)

Control sizing (px, SP+css): `.st-btn`/`.st-input` 32; `.seg` buttons 26 in a ~32 track; `.rift-toggle` 40×23; search 30; `.snav` 42; `.sw` aspect 1.4 (~46×33); `.st-eye` 24; `.st-cli-copy` 24-26×20-24; `.set-pick` ≈ 35; slider track 6 / thumb 15 (`.set-range`) and 12 / 20 (`.hue-range`). Halo: Button 44 (sm 36), Toggle 44×26, Select 44, TextField 52, Segmented btn 36, Tab 44, Check/Radio 20 inside a 44px row, slider thumb 20/track 6. **Rift Settings is built to a 26-32px desktop-dense target; Halo's rule "touch targets at least 44px" (02-type…, Space) is the central mismatch** (§10 Q2).

Palette reality: semantic **`--ok` (green, accent-independent, app.css:188-190)** drives connected pills (`.st-pill.ok` settings-controls.css:32-33), status banner (1863), env-stat (1876), build tag (1937), `cp-str`. Halo has **no ok token**: "positive = accent on accent-soft" (01-themes…, Status). In the Electric-blue preset `info` becomes lime `#c6f35a` (HT `[data-theme="blue"]`), `warn` stays `#ff9f5a`, `danger` `#ff6b6b`.

Dead / duplicate CSS found in SP: `.bg-opt` (1797, no markup), `.card-tt-act` (1750), `.card-divider` (1800), `.set-pick-grid-2` (1970, only in a media query 2021), `.st-cli-warn` defined twice (1920, 1926), `.st-powered` styled as an interactive button (`cursor:pointer`, hover colour, 1942-1946) but rendered as a plain `<div>` with no handler (1495). `.card` exists globally (app.css:496) AND scoped (SP:1689) — scoped wins; `:global(.card.sflash)` (1635) depends on it.

Zero use of: `ui/button`, `ui/separator`, `ui/alert-dialog`, the global `.btn/.input/.pill` — i.e. the "composed from ui/ primitives" direction of `check-ui.mjs` isn't applied to Settings; it hand-rolls ~every control.

---

## 6 Restyle boundary

### LOGIC to keep (do not rewrite in the restyle wave)

| Concern | Cite | Keep notes |
|---|---|---|
| Section/tab model | `Section` type 51, `ST_SECTIONS` 59-65, `activeSec` 67, `selectSec` 223, `focusTab`/`onTabKey` roving 227-241, `secDomId()` 56-58 (DOM ids `settings-tab-*`/`settings-panel-*`, `providers` alias — comment says CDP/tests may target them) | Keep ids. Halo Tabs is value/label/count only (HD:300-306) — icon/dot must go inside `label` ReactNode-equivalent snippet |
| Command-palette deep link | effect 264-272 (`commandPalette.targetSettingsSection`, `clearSettingsSection` untracked) | keep verbatim |
| Settings search | `SEARCH_INDEX` 106-150, `searchResults` scoring 167-182, `jumpTo` 184-209 (uses `#card-*` ids + `.sflash` + `requestAnimationFrame` ×2), `onSearchKey` 210-219, `onGlobalKey` ("/", Ctrl+F) 158-166 | keep; **`#card-*` anchors and `.card` + `.sflash` class are coupled** — if Card markup changes, update jumpTo or keep ids. Add: open closed `<details>`/Accordion before scrolling (currently missing) |
| Ink measure | `measureInk` 78-95 reads `offsetLeft/offsetWidth` of tab buttons, `ResizeObserver` on host | markup-coupled: any new tab markup must expose measurable buttons; same logic reusable for a Segmented thumb |
| Radio roving | `onRadioKey` 242-257 queries `button[role="radio"]` inside `[role="radiogroup"]` and `.click()`s | markup-coupled; **dies if Segmented becomes `aria-pressed` buttons** (Halo, README Segmented) or is swapped for native radios (Halo RadioGroup). Choose semantics first (§10 Q9) |
| Stores & IPC | `uiPrefs` (setAccentHue/Vividness/UiScale/Density/Code/…), `accessibility`, `assistantStore` (setUseFullConfig, setTrustLevel, setPlan, setApiKey, setOpenAiApiKey, setOutputStyle, setMaxBudgetUsd, startCodexLogin, refresh*), `stt` (config, models, download/delete/cancel, inputDevices), `cliUpdate`, `updates`, `environment`, `elevation`, `onboarding/betaNotice`, `workspace.setActive("diagnostics")`, `diagnostics.init`; Tauri: `invoke("app_version")` 596, `appConfigDir/appLogDir` 12, `confirm()` 337/354, `openPath/openUrl` 14, `navigator.clipboard` 312 | all in script; untouched by a skin change |
| Drafts & dirty/saving/msg triplets | `asstApiKey*`, `openAiApiKey*`, `asstMaxBudget*`, `asstOutputStyle*` 398-417, savers 483-547 | keep; the **msg strings** are plain `.st-note` text, candidates for Halo Field `hint`/`error` |
| Derived provider state | `claudeConnected`, `codexConnected`, `openAiConnected`, `providerNeedsAttention`, `cliNewer/Target/Summary`, `cliVersionUnknown/BelowFeatureFloor` 420-480 | keep |
| Reset flow | `resetTab` w/ in-flight latch 330-351, `RESET_COPY` 322-327 | keep; dialog presentation is a separate decision (native `confirm` vs Halo Dialog) |
| One-time mount loads + 30s tick | `onMount` 594-619 | keep |
| Slider action | `sliderBubble` (thumbW=18 constant, `.slider-bubble/.slider-zoom` class names, body-append) | keep logic if bubble survives; thumb width constant must match new thumb (Halo 20) |
| Progressive disclosure | `headWithSelected` 580-585, `langsExpanded`, `shortcutsExpanded` 586-591 | keep |

### MARKUP / CSS to change
Everything in 622-1582 template skin classes and 1584-2022 style; `settings-controls.css`; `.rift-toggle` global (shared with other pages? grep shows only SP uses it in `src/`: SettingsMenu `.tt-switch` mentioned in app.css:473 comment but no hit — so SP is currently the only `.rift-toggle` consumer; unknown whether other files use `.sm`); `Select.svelte` + `ui/select/*` look; `recipes.ts` tokens; hero/tab row; preview wells; swatch/hue/vividness widgets.

Four places where logic is **entangled with markup** and must be re-wired together: (1) `measureInk`+tab buttons, (2) `jumpTo`+`#card-*`/`.card`/`.sflash`, (3) `onRadioKey`+`role="radio"`, (4) `sliderBubble`+`thumbW`+CSS classes.

---

## 7 Halo mapping table

Legend: **Direct** = Halo component fits; **Adapt** = fits with a wrapper/changes; **GAP** = Rift-specific part needed.

| Rift element (cite) | Halo target | Fit | Notes |
|---|---|---|---|
| `.snav` tab row + sliding `.tab-ink` (628-655) | `Tabs` (HB:259-266; props value/label/count, HD:300) | Adapt | Halo Tabs has no icon/status-dot props: put icon + dot inside the label node, or Rift wrapper. Halo underline bar = same transform+width trick. Keep role=tab + arrows (already native in Halo). Halo gap `space-5` 24px vs Rift ~20px (4px gap + 2×8px margin, SP:1594, 1597). Tab is 44px vs 42 |
| `PageHero` eyebrow/title/sub | `SectionLabel`+Halo `title` text style; no hero component | GAP | Halo has no page-header part. Rift keeps `PageHero` re-skinned (Geist title style `title`, eyebrow like `label` style). Page-level decision, not Settings-only |
| (no Rift element) Checkbox | `Checkbox` (README: confirmed by a button / multi-pick; box pops on spring, check draws in) | N/A | Settings has **no checkbox use case**: every boolean applies instantly, which Halo assigns to `Toggle` ("If the change needs a Save button, use Checkbox", Toggle README). Only text inputs wait on Save (output style, keys, budget) — see Toggle and `.st-input` rows. |
| `.sset-search` (+results listbox 676-700) | `SearchField` (README SearchField: icon, clear button pops in, shortcut hint, Esc clears, `loading`) for the input; results = `CommandPalette` pattern or `Menu` surface | Adapt | SearchField has `shortcut` chips ("/" fits) but no results popup. Results listbox must be Rift-built on the Menu surface (HB `halo-pop-in`, 411). **Hand-rolling bumps `check-ui` `overlay:aria-role` ratchet** (check-ui.mjs:97) unless built from `ui/` primitives |
| `.card` containers (1689) | `Card` (HB:30: bg-100, line-100→200 hover, radius-lg, padding space-5 24px) | Direct | Rift padding 16/18 → 24; island tint → `bg-100` solid step. `Card spotlight` optional; Halo says few hero cards only. Keep `id="card-*"` for jumpTo |
| `.card-tt` + `.card-sub` | Halo `heading`/`subtitle` type styles (02-type…: "subtitle: dialog titles and settings-group titles") | Direct | exact doc line: settings-group titles = `subtitle` |
| `.provider-section-head` (1652) + section blocks | `SectionLabel` (`01 / CLAUDE SETUP`, mono, accent index, hairline, HB:85-88) | Direct | Number sections per tab; also candidate for the five tab sub-groupings |
| `.ctl-row` horizontal label-left/control-right ×42 | **none** — Halo `Field`/`Toggle`/`Slider`/`Select`/`TextField` stack **label above control** (Field README; Slider label+value top row HB:175; TextField floating label) | **GAP (structural)** | Halo `Toggle` takes `label` (row clickable) but **no description prop** (HD:110-122); no key/description/control row part exists. Either (a) adopt vertical stacking (page grows ~2×; conflicts w/ Halo "settings fit one screen") or (b) Rift-specific `SettingsRow` (title + description + trailing control) built from Halo tokens. Largest decision for this surface (§10 Q4) |
| `.rift-toggle` ×16 (+1 demo) | `Toggle` (HB:141-148: 44×26, knob 18, ease-spring, stretch on press 24px, `accent` track / `bg-300` off, `line-300` border) | Direct | Apply-instantly semantics match Rift (all Rift toggles save immediately). Halo `label` makes whole row a target — only if SettingsRow wraps. Gated toggles use `disabled` (opacity .45). ARIA role=switch already matches |
| `.seg` ×10 (density, font size, tab width, process detail, tool detail, narration, command output, UI font, git tools, context limit) | `Segmented` (HB:91-96; 2-5 short options, sliding accent thumb, `aria-pressed`, 36px btns, mono 13px labels) | Adapt | Fits (all ≤4 options). Differences: Halo uses `aria-pressed` not `radio` (breaks `onRadioKey`); **no per-option `disabled`** in HD:41-48 (Rift disables whole group for command output; works with `aria-disabled` wrapper or a prop addition); thumb = **solid accent fill** where Rift active is a neutral raised pill → visual change; `activePreset===null` "Custom mix" state (no active option, 827) needs "no selection" support (unknown whether Halo permits value undefined; `value?` optional in HD:41-48 suggests yes) |
| `.set-pick-grid` engine ×3 (1237) + language ×3/12 (1270) + auto-stop ×4 (1417) | `RadioGroup variant="cards"` (HB:561-574, HD:541) with `description`/`meta`; Auto-stop = `Segmented` | Adapt | Engine tiles have label + mono sub ("Edge · Azure when online") = RadioGroup `description`/`meta`. Language list 12 → `Select` or `Combobox` (README: >~15 → Combobox; 12 → Select) instead of a tile grid + "Show all 12" expander. Disabled-per-option (whisper/parakeet backend absent) supported by RadioGroup `disabled` |
| `.st-input` ×4 (output style, API keys ×2 w/ show/hide, cost cap) | `TextField` (floating label, 52px, hint/error, `type`, icon) or `Field`+native `halo-input` | Adapt | `TextField` has no trailing-button slot → show/hide eye (1111, 1208) and inline Save/Clear (1065-1071) are Rift-specific compositions. Monospace values need class. Saved/err messages (`st-note`, e.g. 1117, 1136, 1214) → Field `hint`/`error` (error says what to do). Output-style `<datalist>` → `Combobox` (README: suggestions; also custom value allowed? unknown) |
| `.st-secret` password+eye | none | GAP | Rift-specific `SecretField` (TextField type=password + trailing icon button) |
| `<textarea .set-textarea>` ×2 (vocab/prompt) | `Textarea` (HB; `maxLength` counter ink-faint→warn→danger, auto-grow) | Direct | Vocabulary has a stated ~800-char budget (1389) → use `maxLength=800` counter. Mono font vs Halo sans: class override |
| `Select` mic device (1356) + `ui/select` | `Select` (README: Halo list, same surface/rows/motion as Menu, `description`, `meta`, typeahead, `native`) | Direct (visual) | Keep bits-ui behavior + restyle via recipes.ts to Halo (44px field, `line-300`, chevron→accent, `halo-pop-in`). `EMPTY` sentinel logic stays. No size prop in HD:147-165 → Rift's 28px h-7 (`fieldTrigger`) goes to 44 or needs `sm` |
| Sliders ×3: hue (729), vividness (736), UI scale (761) | `Slider` (HB:174-186: label + mono value top row, 6px track, 20px ring-thumb, spring grow 1.15/1.3, `format`) | Adapt | Direct for vividness + UI scale (`format` = `v => v+'%'`). Hue slider needs rainbow track → custom (GAP). Bubble: keep/kill (§10 Q5) |
| Accent card: swatches ×8 + hue + vividness + "in action" well (717-752) | **none** — Halo has 4 fixed accent presets via `data-theme` (01-themes…); no hue/chroma dial | **GAP (conceptual)** | See §10 Q1. Options: delete card; replace with a Halo **theme picker** (RadioGroup cards / Segmented of Lime/Electric blue/Ember/Violet ± light); or keep hue dial as an override that rewrites `--accent` over Halo's blue |
| Interface density seg (767) + UI scale (761) | none (Halo has no density mode; fixed 44px targets, 8px grid) | GAP | §10 Q2 |
| `.code-preview` ×5 wells w/ fake window dots (742, 793, 858, 887, 927) | `CodeBlock` (README: "never colours syntax", for config/commands/console) — wrong fit; or `Card` + `Divider` | **GAP (Rift-specific `PreviewWell`)** | Previews are live renderings (accent demo, density rows, stream preview, bubble, reply with a11y pipeline). The audit already says drop the ●●● chrome (`redesign-plan.md:86-88`). Rebuild as a `bg-000` inset well with a `SectionLabel`-style caption ("PREVIEW · updates live") |
| Status banner `.sb-status` (966-1041) | `Banner` (HB:387-396; tone info/accent/warn/danger; **one** action; `onDismiss`) | Adapt / GAP | Rift banner holds icon, summary, CLI chip, 1-2 pills, timestamp, Check again, install list, update CTA + copy cmd, err/ok output. Halo `Banner` = title + description + one action (HD:405-412) → use Banner for the headline status only; chips/list/update row = Rift-specific `ProviderStatus` card below it. Note default tone is `info` which is **lime in blue preset** |
| `.st-pill` ×10 (Connected/Set up/Active/Configured/Update →/Check failed/Checking…) | `Badge` (neutral/accent/info/warn/danger + dot, HB:229-235) and `LiveDot` for live state | Adapt | No `ok` tone: Connected → `accent`; Set up/needs attention → `warn`. Needs a label always (Halo: status carries word/icon, not colour alone — already true here). Dot ping → `LiveDot` only when truly live |
| `.provider-switcher` card-tabs w/ icon, 2-line copy, status pill (947-958) | `RadioGroup variant="cards"` (label+description+meta) *or* `Tabs` | GAP/Adapt | Semantically a 2-way view switch (not a selection) → closest is cards-style RadioGroup/Tabs, but it embeds a status Badge; Rift-specific `ProviderCard` likely. Today it uses `role=tab` buttons with **no tabpanel linkage or arrow-key handling** (947-957) |
| `<details class="card provider-advanced">` ×2, `.st-dev` | `Accordion` (`variant="cards"`, `type=multiple`; animated grid-rows; `aria-expanded`; `meta`) | Direct | Gain animation + `inert` closed content. `summary::after "Show/Hide"` text → chevron. jumpTo should open the item |
| `.route` strip (1095, 1150, 1188) "A → B → note" | none (closest: `Stepper`? no) | GAP (Rift-specific) | Billing-route diagram chip row; could become Badge + arrow glyph row |
| `.plan-gauge` (1082-1088) | `Progress` (`value` 0-1, label, `format`; eases) | Direct | 200K/1M gauge with a tick at 20%; tick is extra |
| `.set-progress` (1303) | `Progress size="sm"` (4px, HB:458) | Direct | Matches exactly (4px) |
| `.keys b` / `<kbd>` (1508, 759) | `Kbd` (`keys` combo, HB:305) | Direct | |
| `.kbd-row` shortcuts list | `DataTable`? overkill → `Card` list rows + `Kbd` | Adapt | 9 rows; Halo `Divider` between |
| `.st-stack-chip` (1479) | `Badge neutral` (mono?) | Direct | |
| `.env-stat` chips (1529) | `Badge` accent/warn + `LiveDot`/icon | Adapt | |
| `.st-build` identity block (1459-1474) | `Card` + `Logo` (Halo Logo is brand-specific; Rift logo ≠ Halo) | GAP (Rift identity) | keep Rift mark |
| `.st-btn` ×22 (`.primary`, `.danger-btn`, `.icon`) | `Button` (primary/secondary/ghost/danger; 44/36px; icon 16px; `glow-strong` on primary hover; 0.97 press) | Direct | Rift's secondary→secondary, primary→primary (Halo: **one primary per view**; Settings today shows up to 6 Save/Connect/Update primaries on one tab), danger-btn→danger, `.icon`→icon. Spinner inside button (`Spinner`). Use `sm` (36px) given density of tab |
| `.set-expand` dashed full-width button (1781) & `.set-reset` | `Button ghost`; reset = ghost/danger text | Adapt | dashed-border pattern isn't Halo; reset could be `Button variant="ghost"` or sit in a card footer |
| `.st-about-row` ×7 (1491, 1557-1573) | `Menu`-like list rows / `NavItem` / ghost `Button` rows | Adapt/GAP | Icon + title + sub + (implied chevron) clickable rows have no Halo component (NavItem is sidebar link w/ icon, count). Rift-specific `ActionRow` |
| `.st-note` / `.st-warn` / `.st-info` | `Banner` (page-level) / `Field` hint+error (inline) | Adapt | Inline per-control notes → Field hint/error; block warnings (`st-warn` 1396, 1398) → `Banner tone=warn` |
| `.link-btn` (1954) | `Button variant="ghost"` sm or Field `action` link | Adapt | |
| Native `confirm()` for reset/repair (337, 354) | `Dialog` (title required, description, footer secondary-first, focus trap, scale-in) | Adapt | Currently OS dialog ≠ themed. Keep latch logic `resetBusy` (330) when async Dialog. Possibly use existing `ui/alert-dialog` (unused here) restyled to Halo (`dialogPanel`, recipes.ts:59-60) |
| `use:tooltip` ×19 | `Tooltip` (120ms delay, rises 4px, inverted bubble, `aria-describedby`) | Adapt | **Today a no-op** (tooltip.ts:1-8: paints nothing, only promotes text to aria-label). Visible consequences below (§8) |
| `.snav-dot` status dot (652) | `Badge dot` / `LiveDot tone=warn` | Adapt | |
| `.card.sflash` search flash | none | GAP | §4 |
| Elevation card live label "Administrator" (1441) | `Badge accent` + shield icon | Direct | |

Tokens to adopt for this surface (Electric blue = `data-theme="blue"`): bg-000 `#08090c`, bg-100 `#0e1014`, bg-200 `#14171c`, bg-300 `#1b1f26`, line-100 `#1f242c`, line-200 `#2c323c`, **line-300 `#5f6774` (control borders, 3:1)**, ink `#eceff3`, ink-muted `#9aa2af`, ink-faint `#7d8592`, accent `#6aa8ff`, accent-ink `#08090c`, accent-soft rgba(106,168,255,.14), shadow-focus `0 0 0 2px #0e1014, 0 0 0 4px #6aa8ff`, radius sm8/md12/lg16/pill, space 4-step ladder, dur 150/300/600, stagger 60, ease-out `.2,.8,.2,1` (HT). Rift ↔ Halo name collisions in `:root`: both define `--accent`, `--accent-soft`, `--warn(-soft)`, `--danger(-soft)`, `--info(-soft)`, `--scrim`, `--shadow-*`, `--radius-sm/lg/xl`, `--dur-fast/base/slow`, `--stagger`, `--ease-spring`, `--font-mono`, `--z-*` with **different values** (e.g. Rift `--radius-sm 6` vs Halo 8; Rift `--z-popover 9000`/`--z-dialog 4000` vs Halo `--z-dropdown 20`/`--z-overlay 50`; `--font-mono` JetBrains vs Geist Mono) → cannot simply load `tokens.css` alongside `app.css`.

---

## 8 Pain points (evidence)

1. **Type scale unrepaired**: 10 px sizes + 8 weights + 5 tracking values in one file (§5); same as pre-Halo audit finding (`redesign-plan.md:79-82`).
2. **Radius sprawl**: 13 resolved radii across 65 declarations (§5) — 9px/7px/11px/5px/3px sit off any ladder.
3. **Two column edges**: hero 820 flush vs body 820-minus-80 padding (SP:1650 vs PageHero:38) — 38-40px offset (§2).
4. **Tooltips are dead but referenced**: `use:tooltip` ×19 is a no-op shim (tooltip.ts:1-8). Information that now exists only as `aria-label`: CLI version-unreadable explanation (975), feature-floor note (977), multi-install explainer (1000), per-install "version?" (1009), update-command copy tips, "Re-check" tip (1531), env-stat "what it does" (1529). Copy even instructs a hover that does nothing: "…hover one to see what it does" (1522). `cursor:help` remains on `.st-cli-chip`, `.st-cli-warn`, `.st-cli-note` (1693, 1920, 1927) — affordance with no result.
5. **Search lands on hidden content**: entries for API key / cost cap / ChatGPT API (SP:129-132) anchor to closed `<details>`; `jumpTo` doesn't open them (184-209). No-results state shows nothing (676).
6. **Search is the only dropdown with blur**: `.sset-results` `backdrop-filter: blur(14px)` + 92% mix (1624), contradicting the WebView2 rule in `ui/recipes.ts:55-56` ("no backdrop-filter — WebView2 mis-composites"). It is absolutely-positioned not fixed (unverified whether it misbehaves).
7. **Hue slider keyboard focus invisible**: `.hue-range:focus {outline:none}` (1843), only `.set-range` gets a `:focus-visible` thumb ring (1853). `.seg button`, `.sw`, `.set-pick`, `.snav`, `.provider-switch`, `.st-about-row`, `.set-expand` have no explicit focus style in SP (UA default or none; unverified rendering).
8. **Hand-rolled everything**: ~22 buttons, 16 toggles, 10 segmented groups built by hand vs available `ui/button`; `ui/separator`/`alert-dialog` unused; native `confirm()` for reset/repair.
9. **Provider switcher semantics**: `role="tablist"/tab` without `aria-controls`, roving tabindex or arrow keys (947-957), unlike the main tabs (636-647).
10. **Primary-button overload**: solid accent `.st-btn.primary` appears on Save/Connect/Update/Install/Download simultaneously (1022, 1067, 1113, 1130, 1165, 1210, 1320, 1541); Rift's own DESIGN.md §2 says solid accent is rare (DESIGN.md:28-31); Halo: one primary per view.
11. **Status tone while loading**: green `ok` banner before auth resolves (966, 559-562).
12. **Long single-scroll tabs w/ conditional cards**: Providers/Voice change length by state (960-1224, 1264-1393); Halo guidance "settings fit one screen" (02-type…, Scrolling and fit).
13. **Fake window chrome + 3-level nesting** in 5 previews (742, 793, 858, 887, 927) — flagged by the audit (`redesign-plan.md:86-88`); still present.
14. **Dead/duplicate CSS** (§5: `.bg-opt`, `.card-tt-act`, `.card-divider`, `.set-pick-grid-2`, duplicate `.st-cli-warn`, fake-button `.st-powered`) and a stale comment claiming a LocalLlmPage consumer (settings-controls.css:2).
15. **Disabled opacity ×3** (.45 / .5 / .55 — Halo single .45); **tab switch double motion**: smooth `scrollTo` (225) + per-card rise (1676).
16. **Accent card shows the accent three ways** (header dot 718, swatch ring, "in action" well) — flagged in audit (`redesign-plan.md:88`).
17. **Slider bubble vs thumb zoom/hover** triple-specified scale (app.css:388 → 1.35; SP:1851 → 1.14) — winner unverified.
18. **Inline max-widths on inputs** (1065, 1110, 1129, 1207) bypass the grid.

---

## 9 Risks

| Risk | Evidence | Detail |
|---|---|---|
| **`npm run check` includes `check:tokens` + `check:ui`** | package.json `check` script | `check-tokens.mjs` diffs the **first `:root` block of `src/app.css`** against `design-system/styles.css` (check-tokens.mjs:1-80) — **any changed shared token value (`--dur-*`, `--radius-*`, `--accent`, …) MISMATCHes unless the mirror is updated in the same change; new Halo-only names are "app-only", advisory (no fail)**. Adopting Halo values means touching both files + `_ds_manifest.json` |
| `check-ui` ratchet (`ui-baseline.json`: use-portal 11, aria-role 17, hex 21, rgb/hsl 16, oklch-literal 131, tailwind-arbitrary 2) | check-ui.mjs:94-105, 165-176 | Counts may only go **down**. SP contributes hex 3 (`#d97757`, 2 × `&#123;/&#125;`), rgb 1, oklch 14. Hand-rolling a search listbox/dialog adds `role="listbox"/"dialog"` (aria-role +1/+N) and fails. Replacing the 9-stop oklch hue gradient with tokens/hsl would move metrics between buckets. Fewer = ask `--update` |
| `ui/` classes must generate CSS | check-ui.mjs:48-91 | Any class added in `ui/*` recipes (Halo tokens like `bg-bg-100`) must exist in the `@theme` bridge (app.css:70-104) or hard-fail |
| Tests referencing Settings markup | grep of `src/**/*.test.ts`, `scripts/cdp`, `tests` | **None found** reference `.card`, `.seg`, `.snav`, `settings-tab/panel`, `card-*` ids (only unrelated `streamModel.test.ts` "seg", `modelMatrix.test.ts` comment). CDP tooling only needs sidebar `[aria-label="Settings"]` (`scripts/cdp/c.sh:16, 483`). The SP comment at 52-55 still asks to keep `providers` DOM ids for "a CDP/test selector". Missing component-level tests = **no safety net for a restyle** |
| Reduced motion | SP 1607-1610, 1643-1647, 1684, 1797; app.css:120-122 | New Halo animations (spring, stagger, accordion, slide thumbs) each need explicit `prefers-reduced-motion` handling; `[data-slot] {animation:none!important}` only covers primitives with `data-slot` |
| WebView2/Tauri | recipes.ts:55-56; app.css:274-276; ui-prefs.svelte.ts:17-19 | `backdrop-filter` on fixed layers mis-composites (Halo uses blur only on small areas; keep it off). UI scale is **page zoom** (`setZoom`): px-based Halo tokens scale fine but `position:fixed` bubble math (`getBoundingClientRect`, sliderBubble.ts:22-32) under zoom is unverified. `::-webkit-slider-thumb` styling is WebKit-only — fine in WebView2 (Chromium); Halo also supplies `-moz-` rules (unneeded) |
| Fonts | Halo = Geist/Geist Mono from Google Fonts (HB loads them; `02-type…`), Rift = `@fontsource` Inter/JetBrains/Lexend bundled offline | Tauri app must not depend on Google Fonts at runtime; Lexend a11y override (app.css:747-752 targets `body, .app, textarea, input, button, select`) vs Halo's `font:` shorthand on `.halo-btn`/`.halo-seg__btn` (HB:43, 93) — specificity/order unverified |
| a11y | HB radio/segmented semantics; SP 636-647, 947-957 | Segmented `aria-pressed` vs Rift radiogroup (changes SR announcement + `onRadioKey`); Halo requires 3:1 control borders (`line-300`) — Rift `--field-border` = `--border-strong` (app.css:152, 157) likely below 3:1 on card (unmeasured); `ink-faint` `#7d8592` allowed only for section labels, not body — SP uses `--fg-faint`/`--fg-subtle` for much descriptive copy (`.ctl-s`, `.card-sub`) which in Halo would map to `ink-muted` |
| Perf | SP 1676-1684, 1624, 1733, 1987 | Settings mounts 2-7 cards/tab, each with entrance animation + `color-mix` heavy styles; width animations (gauge, progress) are layout-bound; `backdrop-filter` + large box-shadow in results panel; `ResizeObserver` + 30s tick are negligible. Replacing with Halo transform/opacity versions is a net win. Avoid `Card spotlight` on all cards (HB README: hero only) |
| Layout/zoom | tauri.conf.json:18-19; SP 2000, 2015 | 900px block hit at min window; Halo 44px targets + `space-5` card padding will grow every row → density loss at 900×600 |
| Scope bleed | app.css `.rift-toggle`, `.slider-bubble`, `.card`, `.btn`, `.input`, `.pill` are global | Restyling `.rift-toggle` or `.card` in `app.css` can reach other surfaces; grep shows SP is the only `.rift-toggle` consumer in `src/` today but `.card` global + `AssistantPage`-style `.island` are shared (full-repo `.card` usage = unknown, not grepped) |
| Hand-rolled search listbox | SP 676-700 | If rebuilt on bits-ui Combobox/Popover primitives the aria-role ratchet stays flat |
| Data loss on tab remount | SP 714-1579 `{#if}` | Drafts (`asst*Draft`) live at component scope so survive tab switches; if Settings becomes Accordion/Tabs with separate components, keep them at page level |
| Git/branch | task rules | branch `design/chat-ledger`; no commit this wave |

---

## 10 Open owner questions (ordered by how much they change the plan)

1. **Accent model.** Halo accent is fixed per preset (Electric blue `#6aa8ff`); Rift's whole Accent card (8 swatches, 0-360 hue, vividness, live preview — SP 717-753, ui-prefs.svelte.ts:84-93, 293-294) assumes a dynamic OKLCH hue that restyles the entire app. Delete it, keep as an "advanced override" over the Halo preset, or turn it into a **Halo preset picker** (Lime / Electric blue / Ember / Violet)? And is light mode (Halo's 4 light themes) in scope — Rift is dark-only today (app.css:125-126, app.html:2 `data-mode="dark"`)?
2. **Density / target size.** Rift Settings is 26-32px controls with a compact/regular/comfy density pref (SP 767; app.css:289-291). Halo mandates 44px targets and an 8px grid (02-type…). Keep a density setting (Halo has none), adopt Halo sizing with a "compact" opt-down, or drop the pref? Window min is 900×600.
3. **One screen vs long scroll.** Halo: settings should fit one screen (`AppShell` scrolls internally). Rift tabs scroll long (Providers, Voice). Accept long tabs, or split further / use Accordion cards so each tab fits?
4. **Row anatomy.** Keep the horizontal `title + description … control` row (needs a Rift-specific `SettingsRow` since Halo has none) or move to Halo's stacked Field layout (taller, no description on Toggle)?
5. **Slider bubble.** Keep `sliderBubble` (floating value + thumb zoom) as a Rift extra over Halo's inline mono value, or drop it for Halo Slider as-is?
6. **Fonts.** Adopt Geist/Geist Mono (needs bundling; Google Fonts not an option offline) or keep Inter/JetBrains Mono under Halo tokens? What happens to the Lexend dyslexia option?
7. **Semantic "ok".** Halo has no green: positive = accent. In Electric blue, "Connected/CLI up to date/Active" turn blue and `info` turns lime. Accept, or keep a Rift-only `--ok` green?
8. **Build approach.** Restyle the existing bits-ui `ui/` primitives with Halo tokens (ratchet-safe, keeps a11y/keyboard) vs hand-porting Halo's React components to Svelte (ConsoleX precedent: its own `Select.svelte`, `SegBar.svelte`, `Tip.svelte` hand-rolled over `design-systems/halo/`)? Which wins for Select, Tabs, Segmented, Accordion, Dialog?
9. **Segmented semantics.** Halo Segmented = `aria-pressed` buttons; Rift uses `role="radio"` radiogroups with arrow-key roving. Keep radio semantics (own thumb implementation) or follow Halo?
10. **Tooltips.** They were pulled 2026-06-15 as visual noise (tooltip.ts:1-8) yet 19 call sites and "hover to see" copy remain. Bring Halo `Tooltip` back (for icon-only buttons at least) or delete the call sites and move the text to visible hints/Field help?
11. **Reset/repair confirmations.** Keep native OS `confirm()` or move to a Halo `Dialog` (with the `resetBusy` latch)?
12. **Previews.** Keep all 5 live preview wells (accent, interface+density, stream, classic bubbles, reply) as a Rift-specific `PreviewWell`, trim some, or merge into one sticky preview?
13. **Search jump flash.** Replace the box-shadow `sflash` with a Halo-safe transform/opacity or border cue, or drop it?
14. **Visible provider status.** Is the green `ok` banner state meant to show while checking (966), or should loading be neutral/`info`?
15. **System tab reset.** Intentionally no reset on System (SP 319-321 comment); confirm stays.

---

## Appendix — numeric summary (for the cross-surface merge)

- SP: 2023 lines; template 961 lines; scoped CSS 439 lines.
- Distinct px font sizes in SP CSS: 10 (+3 `--fs-*` + 1 em); weights 8; letter-spacing values 5.
- Distinct resolved radii: 13 (65 declarations); Halo ladder: 4.
- Hard-coded colour literals in SP: 1 real hex, 1 rgba, 14 oklch literals.
- Controls: 16 toggles, 10 segmented groups, 3 sliders (1 gradient-track), 22 `st-btn`, 10 pills, 4 text inputs, 2 textareas, 1 Select, 3 `<details>`, 3 pick grids, 5 preview wells, 43 search-index entries.
- Halo parts with direct equivalents: Tabs, Toggle, Segmented, Select, Slider, Textarea, TextField(+Field), RadioGroup(cards), Accordion, SectionLabel, Banner (partial), Badge/LiveDot, Progress, Kbd, Button, Card, Dialog, Tooltip, SearchField.
- Rift-specific parts still needed: `SettingsRow`, `PreviewWell`, `SecretField`, `ProviderCard/Switcher` + `ProviderStatus`, `RouteStrip`, `ActionRow` (about rows), hue-spectrum slider (if the accent dial survives), search-results popup, PageHero re-skin.
