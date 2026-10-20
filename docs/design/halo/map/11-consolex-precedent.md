# 11 - ConsoleX precedent: how the owner's other Svelte app adopted Halo

Map only. Everything below was read this session (2026-10-19); a line cite means I read that line. "Unknown" = not read / not verifiable from disk.

## 0. Key + sources

- `CX` = `C:/AI Workflow/projects/consolex`; `PANEL` = `CX/panel` (the Svelte app).
- `CL` = canvas `Claude.dc.html`, `MN` = canvas `Main.dc.html` (Overview board, NOT chat), both under `C:/Users/BLAZZER/AppData/Local/Temp/claude/C--AI-Workflow-projects-rift-tauri/3cd5d05a-bb71-48d8-8d23-bc53e60827d8/scratchpad/artifact-files/9177ef0f-35ed-49aa-a627-411c79ca8017/project/`. Scratchpad copies die with the session (CX/docs/archive/handoff-history.md:395, :422) so section 2 pastes the canvas CSS verbatim. `canvas.json` and `support.js` (the `x-dc` runtime): not read, unknown.
- Plan: `CX/docs/archive/halo-port-plan.md` (A-F phases). Roadmap row 6 says "Done 2026-10-10 ... looked at by the user, merged to main, released as v0.1.0" (`CX/docs/roadmap.md:13`).
- Stack delta vs Rift: ConsoleX = plain Svelte 5 + Vite 8, **no SvelteKit, no Tailwind, no bits-ui** (`PANEL/package.json` deps: svelte, vite, @lucide/svelte, fontsource Geist). Rift = SvelteKit 2 + Tailwind 4 + bits-ui + Inter/JetBrains Mono/Lexend (`C:/AI Workflow/projects/rift-tauri/package.json:41-74`). So the CSS/motion/anatomy transfers; the alias-layer wiring needs a Tailwind decision (see Owner questions).
- ConsoleX also ships in a Chrome 103 FiveM overlay (`PANEL/vite.config.ts` `build: mode==='nui' ? {target:'chrome103'}`); Rift is a Tauri/WebView2 app, so every "Chrome 103" caveat below is informational only.

### Commit trail (git, `git -C CX log`)
| Date | Commit | What |
|---|---|---|
| 2026-10-08 | `e1bc33f`, `c520013` | Halo tokens + restyle of 21 files (styles only, 944+/1055-), `app.css` rewritten; Dock height tweak |
| 2026-10-10 | `d936ec7` A | hidden scrollbars, page glide, motion helpers (`app.css`, `App.svelte`) |
| 2026-10-10 | `1f0244a` B | Sidebar to canvas design (Sidebar.svelte +204/-97) |
| 2026-10-10 | `fc81b99` C | Claude page to canvas design (Claude.svelte +326/-234, `ui.sideOpen`) |
| 2026-10-10 | `03164dd` D | per-view motion (LogView, Trace, Overview) |
| 2026-10-10 | `595b688` | merged to main, v0.1.0 |
| 2026-10-14 | `0eabd84`, `cfc5e3f` | Halo v2.3 vendored; panel moved onto generated `tokens.css`; `app.css` = aliases only; blur dropped; thin scrollbars back; `z-*` tokens |
| 2026-10-17 | `1aebd1e` | "No green": blue preset's lime `--info` replaced by light blue |
| 2026-10-17 | `70e7e72`, `0ba531d` | compact header 76->52, Claude toolbar 60->44, sidebar 232->208 / 72->56 |
| 2026-10-17 | `a532426`, `aca15a1` | composer to one row + one settings menu; session panel shut by default; reply numbers on hover |
| 2026-10-17 | `82b394f`, `51f4d02` | split view; swap-animation fix |

Halo-era timeline reads: canvas v16-v18 (Oct 8, interactive Claude board) -> port plan (Oct 10) -> A-F same day -> Halo v2.3 (Oct 14) -> "calmer page" pass (Oct 17) (`CX/docs/archive/handoff-history.md:392-470`, `CX/HANDOFF.md:17-29`).

---

## 1. Port mechanics

### 1.1 Token load order (the actual wiring)
- `PANEL/src/main.ts:1-4`: `import '@fontsource-variable/geist'` -> `'@fontsource-variable/geist-mono'` -> `'../design-systems/halo/tokens.css'` -> `'./app.css'`. Order matters: Halo vars first, ConsoleX aliases second.
- Halo's React `bundle.css/bundle.js` are vendored (`PANEL/design-systems/halo/`: README, bundle.css, bundle.js, index.d.ts, react.js, react-dom.js, tokens.css, tokens.json) but **never mounted by the app**; they are served at runtime to the design-studio canvas, and knip ignores them (`CX/CLAUDE.md:31`). All Halo markup in the app is hand-written Svelte (decision: "Canvas built as plain markup with Halo tokens, not mounted window.Halo components ... the Svelte port can't use its React bundle", `handoff-history.md:458`).
- Preset: `PANEL/index.html:2` `<html lang="en" data-theme="blue">`. Plus a belt-and-braces JS guard because claude.ai's artifact pane overwrites `data-theme` with dark/light: `main.ts:7-14` `MutationObserver` on `<html>` re-sets `data-theme='blue'` whenever it changes (`HANDOFF.md:36`). Irrelevant to Rift unless Rift is ever rendered inside an artifact.
- `tokens.css` is **generated**: `tokens.json` -> `PANEL/src/host/design-tokens.ts:20-45` `tokensCss()` (resolves `{alias}` -> `var(--x)`, re-declares aliases per theme block so `data-theme` below `:root` works) via `PANEL/design-systems/build-tokens.ts` (`npm run tokens`, `PANEL/package.json` scripts). A test fails if tokens.css is stale (`PANEL/README.md:58`). Header of generated file: `PANEL/design-systems/halo/tokens.css:1`.
- tokens.css shape: `:root, [data-theme="dark"]` holds lime/default values and every non-colour token (spacing 1-8, radii sm8/md12/lg16/pill, `--shadow-pop`, `--shadow-focus`, `--dur-fast 150ms`, `--dur-base 300ms`, `--dur-slow 600ms`, `--stagger 60ms`, `--ease-out cubic-bezier(.2,.8,.2,1)`, `--ease-spring cubic-bezier(.34,1.56,.64,1)`, `--opacity-disabled .45`, `--z-raised 1 / sticky 10 / dropdown 20 / overlay 50 / toast 60 / tooltip 70`, `--bp-*`, `--font-sans/--font-mono`) (`tokens.css:2-62`); `[data-theme="blue"]` overrides only 8 vars: accent `#6aa8ff`, accent-ink `#08090c`, accent-soft, glow-strong, **`--info:#c6f35a` (lime!)**, info-soft, chart-*, shadow-focus (`tokens.css:63-74`). Light themes swap surfaces/lines/ink too (`:100-219`).

### 1.2 Aliasing old names -> Halo (`PANEL/src/app.css:1-67`)
- Strategy: every old ConsoleX name is kept as a one-line alias of a Halo var so no component had to be rewritten: `--bg: var(--bg-000)`, `--rail`, `--panel/--panel-hi/--dock: var(--bg-100)`, `--well/--input: var(--bg-000)`, `--raised: var(--bg-200)`, `--track: var(--bg-300)`, `--line: var(--line-100)`, `--line-2/--line-3: line-200`, `--line-4/--line-hover: line-300`, `--text: var(--ink)`, `--muted: var(--ink-muted)`, `--dim/--faint: var(--ink-faint)` (`app.css:13-47`).
- Hand-set non-Halo values: `--text-2 #dde1e7`, `--text-3 #c3c9d2`, `--text-4 #aab1bc`, `--ghost #737b88`, `--row #171a20`, `--amber-hi #9cc6ff`, `--err-text #ff9c9c` (`:29-47`) - these have no Halo equivalent and are the places a Rift port will need a decision.
- `--amber: var(--accent)` (old signal name = accent), every warning-meaning site was hand-moved to `--warn` (`handoff-history.md:459`).
- **rgb triplets** Halo does not ship are hand-set to blue: `--accent-rgb: 106,168,255`, `--info-rgb`, `--warn-rgb`, `--danger-rgb` (`app.css:6-9`) because the code uses `rgba(var(--accent-rgb), a)`. Roadmap flags this as the blocker for light themes: "move to color-mix()" (`CX/docs/roadmap.md:27`). Rift tip: use `color-mix(in srgb, var(--accent) N%, transparent)` from day one.
- `--info` override: `:root[data-theme='blue'] { --info:#9cc6ff; --info-soft: rgba(156,198,255,.14) }` because blue's lime info = green in memory chart/scanner notes; owner wants "no green anywhere" 2026-10-17 (`app.css:62-67`, commit `1aebd1e`). Specificity note in file: `:root[...]` outranks tokens.css's `[data-theme]`.
- Fonts: **not** Halo's Google Fonts; self-hosted fontsource variable Geist: `--sans:'Geist Variable'`, `--mono:'Geist Mono Variable'` (`app.css:57-59`). Halo's `--font-sans` names "Geist" (Google build) so they are separate vars.
- Preset change after the fact = the 6 accent values (`--accent, --accent-rgb, --accent-soft, --accent-ink, --glow-strong, --shadow-focus`) in the pre-v2.3 world (`halo-port-plan.md:5`); after v2.3 it is just `data-theme`.
- Global rules adopted: body `14px/1.5 var(--sans)` (`app.css:91-95`); solid focus ring on `button/a/input/textarea/[tabindex]:focus-visible { outline:none; box-shadow: var(--shadow-focus) }` (`:114-121`); `::selection` accent 30% (`:163-165`); `.lucide-icon` 1em, stroke 1.8 (`:197-203`); `.legend` = mono 11px, .12em, uppercase, ink-faint (`:210-218`); `.panel` card = bg-100, line-100 1px, radius-lg, hover line-200 (`:229-245`).

### 1.3 Layout coupling worth copying/avoiding
- App shell: grid `208px | 1fr` open, `56px` collapsed, `transition: grid-template-columns .5s var(--ease-out)`; widths **duplicated** in `App.svelte:50,97` and `Sidebar.svelte:33` ("widths are mirrored", `Sidebar.svelte:31`).
- Page area padding `28px 40px 40px` (`App.svelte:123`); the Claude page escapes it with `margin:-28px -40px -40px; height:calc(100% + 68px)` (`Claude.svelte:1249-1250`). Must change both together (`halo-port-plan.md:39`). Pitfall: fragile coupling; a Rift port should give the chat route its own layout rather than negative margins.

---

## 2. Motion classes they built (exact CSS)

### 2.1 Canvas source of truth - `CL:12-66` `<helmet><style>` (verbatim, motion/transition rules; `MN:12-52` is the same minus menu/rail/cx-* rules, plus `.hx-card:hover{border-color:#2c323c}` (`MN:17`) and `.hx-row:hover{background:#14171c;border-color:#2c323c}` (`MN:18`))
```css
body{margin:0;background:#08090c}
button:focus-visible,a:focus-visible,textarea:focus-visible{outline:none;box-shadow:0 0 0 3px rgba(255,255,255,.35)}
@keyframes hx-page{from{opacity:0;transform:translateY(10px);filter:blur(6px)}}
@keyframes hx-rise{from{opacity:0;transform:translateY(14px)}}
@keyframes hx-slide{from{opacity:0;transform:translateX(-10px)}}
@keyframes hx-open{from{opacity:0;transform:translateY(8px) scale(.98)}}
@keyframes hx-draw{from{clip-path:inset(0 100% 0 0)}}
@keyframes hx-ring{from{stroke-dasharray:0 100}}
@keyframes hx-pop{from{opacity:0;transform:scale(0)}}
@keyframes hx-pulse{from{transform:scale(.7);opacity:1}to{transform:scale(1.9);opacity:0}}
@keyframes hx-breathe{50%{opacity:.35}}
@keyframes hx-blink{50%{opacity:0}}
.hx-page{animation:hx-page 520ms cubic-bezier(.2,.8,.2,1) backwards}
.hx-open{animation:hx-open 520ms cubic-bezier(.2,.8,.2,1) 80ms backwards}
.hx-rise>*{--hx-i:0;animation:hx-rise 300ms cubic-bezier(.2,.8,.2,1) backwards;animation-delay:calc(var(--hx-b,140ms) + var(--hx-i) * 70ms)}
.hx-late>*{--hx-b:300ms}
.hx-stream>*{--hx-i:0;animation:hx-slide 300ms cubic-bezier(.2,.8,.2,1) backwards;animation-delay:calc(260ms + var(--hx-i) * 45ms)}
:is(.hx-rise,.hx-stream)>:nth-child(2){--hx-i:1} ... :nth-child(14){--hx-i:13}
button,a,.hx-card,.hx-row,.hx-res,.hx-log{transition:translate 150ms cubic-bezier(.2,.8,.2,1),scale 150ms cubic-bezier(.2,.8,.2,1),background-color 150ms,border-color 150ms,color 150ms}
.hx-card:hover{translate:0 -3px}
button:not(:disabled):active,a:active{scale:.97}
.hx-draw{animation:hx-draw 700ms cubic-bezier(.2,.8,.2,1) 480ms backwards}
.hx-ring{animation:hx-ring 700ms cubic-bezier(.2,.8,.2,1) 480ms backwards}
.hx-pop{animation:hx-pop 300ms cubic-bezier(.2,.8,.2,1) 1100ms backwards}
.hx-pulse{animation:hx-pulse 2s cubic-bezier(.2,.8,.2,1) infinite}
.hx-breathe{animation:hx-breathe 1.6s ease-in-out infinite}
.hx-blink{animation:hx-blink 1s steps(1) infinite}
*{scrollbar-width:none}*::-webkit-scrollbar{display:none}
.hx-row:hover{background:#14171c;border-color:#1f242c}
.hx-chev{opacity:0;translate:-4px 0;transition:opacity 150ms,translate 150ms cubic-bezier(.2,.8,.2,1)}
.hx-row:hover .hx-chev{opacity:1;translate:0}
.cx-side nav a>span,.cx-side nav>span,.cx-brand>div:last-child,.cx-status{transition:opacity 150ms}
.cx-min nav a>span,.cx-min nav>span,.cx-min .cx-brand>div:last-child,.cx-min .cx-status{opacity:0;pointer-events:none}
.cx-min .cx-brand{padding:0 4px!important}
.cx-min nav a{padding:0 8px!important}
@keyframes hx-ping{from{box-shadow:0 0 0 0 currentColor}to{box-shadow:0 0 0 5px transparent}}
.hx-ping{animation:hx-ping 2s cubic-bezier(.2,.8,.2,1) infinite}
.hx-ico{transition:background-color 150ms,color 150ms}
.hx-nav:hover .hx-ico{background:#1b1f26;color:#eceff3}
@keyframes hx-menu{from{opacity:0;scale:.96}}
@keyframes hx-spin{to{rotate:360deg}}
.hx-menu{animation:hx-menu 180ms cubic-bezier(.2,.8,.2,1) backwards}
.hx-spin{animation:hx-spin .8s linear infinite}
.hx-opt:hover{background:#14171c!important}
.hx-sug:hover{background:#14171c;border-color:#2c323c}
.cx-comp{transition:border-color 150ms,box-shadow 150ms}
.cx-comp:focus-within{border-color:#3a414c;box-shadow:0 0 0 4px rgba(255,255,255,.04)}
.cx-comp textarea:focus-visible{box-shadow:none}
@media (prefers-reduced-motion:reduce){*,*::before,*::after{animation:none!important;transition:none!important}}
```
Design rules baked into it (from the session logs): fill mode `backwards` + hover/press through the **individual** `translate`/`scale` properties so a finished keyframe's `transform` never blocks hover (`handoff-history.md:433`); canvas panel collapse = `flex-basis 300ms ease-out` + label opacity 150ms (`CL:71`, `CL:324`).

### 2.2 Shipped global helpers - `PANEL/src/app.css`
```css
/* app.css:247-299 */
.enter { animation: panelIn var(--dur-slow) var(--ease-out) both; animation-delay: var(--d, 0s); }
.rise > *, .stream > * { --i: 0; }
:is(.rise, .stream) > :nth-child(2) { --i: 1; }  /* ... up to nth-child(n + 11) { --i: 10; } */
.rise > *   { animation: panelIn var(--dur-base) var(--ease-out) backwards;
              animation-delay: calc(var(--rise-from, 120ms) + var(--i) * var(--stagger)); }
.stream > * { animation: rowIn var(--dur-base) var(--ease-out) backwards;
              animation-delay: calc(var(--rise-from, 240ms) + var(--i) * 45ms); }
.draw { animation: draw var(--dur-slow) var(--ease-out) 400ms backwards; }
.ping { animation: ping 2s var(--ease-out) infinite; }
.lift { transition: translate var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out); }
.lift:hover { translate: 0 -3px; }
/* app.css:500-606 keyframes */
@keyframes panelIn { from { opacity:0; transform: translateY(14px) } }
@keyframes pageOut { to { opacity:0 } }
@keyframes pageIn  { from { opacity:0; transform: translateY(10px) } }   /* blur dropped, see 3.3 */
@keyframes draw    { from { clip-path: inset(0 100% 0 0) } }
@keyframes ping    { from { box-shadow: 0 0 0 0 currentColor } to { box-shadow: 0 0 0 5px transparent } }
@keyframes ringOut { 0%{transform:scale(1);opacity:.75} 100%{transform:scale(2.7);opacity:0} }
@keyframes blink   { 50% { opacity:.2 } }
@keyframes sweepBar{ from{transform:translateX(-100%)} to{transform:translateX(340%)} }
@keyframes badgePop{ from { transform: scale(0) } }
@keyframes popIn   { from { opacity:0; transform: translateY(-10px) scale(.97) } }
@keyframes fadeIn  { from { opacity:0 } }
@keyframes grow    { from { transform: scaleX(0) } }
@keyframes rowIn   { from { opacity:0; transform: translateX(-10px) } }
@keyframes stampIn { from { opacity:0; transform: translateX(-18px); clip-path: inset(0 100% 0 0) } }
@keyframes spin    { to { transform: rotate(360deg) } }
@media (prefers-reduced-motion: reduce) { *,*::before,*::after { animation:none!important; transition:none!important } }
```
- Shell hooks: `.page { animation: pageIn 520ms var(--ease-out) backwards }`, `.page.exiting { animation: pageOut .17s ease-in forwards }` (`App.svelte:118-128`); page re-created via `{#key ui.page}` (`App.svelte:56-66`); `ui.nav()` sets `exiting=true`, swaps `page` after **170ms** (`PANEL/src/lib/ui.svelte.ts:105-129`); console-dock navigation special-cased with a 650ms hold (`:114-118`).
- Interaction primitives (`app.css`): `.btn` 40px, radius-md, `transition: bg/border/color/box-shadow/transform var(--dur-fast)`, `.btn:active{transform:scale(.97)}` (:326-356) - note this uses `transform`, not the individual `scale` property the plan prescribes; `.btn.primary:hover{box-shadow:0 8px 24px -6px var(--glow-strong)}` (:373-376); `.iconbtn` 32px, `:active{scale:.94}` (:387-410); `.field:focus-within` accent border + `--shadow-focus` (:413-431); `.led.busy{animation:blink .5s steps(2) infinite}` (:445-464); `.trough>span` = accent bar, `transition:width .9s; animation: grow 1.1s` (:477-496).
- JS motion helpers `PANEL/src/lib/motion.ts`: `reducedMotion()` (:1), `tween(node,{value,dec})` 750ms cubic-out count-up written straight to `textContent` (:5-47), `decode(node,text)` 560ms glyph-scramble page title (:49-72). Used: `TopStrip.svelte:47`, `Overview.svelte:138`, `Diagnostics.svelte:186`, `InGame.svelte:112`.
- Usage census (grep this session): `.rise` is used once (Claude `.side-in`, `Claude.svelte:1161`); `.stream` once (`Overview.svelte:112`); `.draw` on meters (`Claude.svelte:1169-1177`), Trace sparkline (`Trace.svelte:30`), Boot logo paths (`Boot.svelte:19-21`). `.enter`/`.panel` carry most per-view entrance (`handoff-history.md`/plan phase D: "most per-view motion already existed").

### 2.3 Page-local motion on the Claude page (scoped in `Claude.svelte`, keyframes at :3248-3352)
| Name | CSS | Used for |
|---|---|---|
| `rise` | `from{opacity:0;transform:translateY(8px)}` .36s ease-out both (:3248, :1522) | every `.turn` (user + bot), hero (.5s :1432), peer pane (.36s :1342), jump pill (.25s :2297) |
| `tick` | `from{opacity:0;transform:translateX(-4px)}` .28s (:3268, :1726) | each step row |
| `ticker` | `from{opacity:0;transform:translateY(6px)}` .3s (:3258, :1792) | live-thought one-liner when a new sentence starts (`{#key tick.n}` :700) |
| `stamp` | `from{opacity:0;transform:scale(.985)}` .45s (:3278, :2153) | in-chat one-line "changed file" row |
| `pop` | `from{opacity:0;transform:scale(.96)}` (:3288) | menus .18s (:2647), dropzone .15s (:2797), context ring .2s (:2960) |
| `turn` | rotate 360 (:3298) | spinner .9s linear (:1752), sparkle mark while live 2.4s (:1594) |
| `dot` | 0/80/100%{opacity:.3} 40%{opacity:1;translateY(-2px)} (:3303), 1.2s, delays .15/.3 (:1624-1636) | "working" three-dot indicator |
| `line-in` | `from{opacity:0;transform:translateY(4px)}` .24s (:3315, :1942) | each new line of a live Edit/Write draft |
| `caret` | `50%{opacity:0}`, 1s `steps(2, jump-none)` (:3325, :1966) | caret on last draft line only |
| `glow` | `50%{opacity:.35}` 1.4s (:3330) | mic-recording icon (:2396), rec dot (:2434), background-chat dot (:2866) |
| `wfade` | `from{opacity:0;filter:blur(3px)} to{opacity:1;filter:none}` .32s (:3335-3344, :1837-1839) | **each streamed word** |
| `shimmer` | bg-position sweep, 2.2s linear infinite, `background-clip:text` (:3345, :1637-1643) | label of the running step ("Reading qbx_core/...") |
- Hover reveals: `.det{opacity:0;transition:opacity .25s}` -> 1 on `.turn.bot:hover/:focus-within` (:1658-1665); `.hint` fades in on composer focus (:2623-2630); `.pane.peer .col{opacity:.7}` -> 1 on hover (:1355-1361).
- Panel open/close: `.cw{display:grid;grid-template-columns:minmax(0,1fr) 340px;transition:grid-template-columns var(--dur-base) var(--ease-out)}`, `.cw.side-off{...0px}` (:1230-1260); `.side.shut{opacity:0;border-color:transparent}` (:2812-2815); inner `.side-in{width:340px}` so content does not reflow while the track shrinks (:2816-2823); `.cw.resizing{transition:none}` so a drag is not animated (:1274-1276). Narrow (<960px container): panel becomes an absolute drawer with `translateX(24px)` + opacity (`:3221-3237`).
- Reduced motion on this page, enforced three ways: global kill (`app.css:608-615`), page-scoped (`Claude.svelte:3353-3362`, also un-clips `.shimmer`), and JS `reducedMotion()` in `pump()` (:391), `follow()` (:277), `jump()` (:456).
- Svelte detail: `@keyframes` inside a component `<style>` are name-scoped, so the page's local `rise` does not collide with global `panelIn`.

---

## 3. Chat page: anatomy + motion, canvas vs shipped

### 3.1 Canvas anatomy (`CL`, 1440x1000 board, "Play" mode, all state-driven in `renderVals()` `CL:399-495`)
- Left nav 232px (`navW` :485) / 76px collapsed (:485), `flex-basis 300ms`, labels fade 150ms (`CL:71`, :48-51). Brand 36px tile; nav rows 40px with 28px icon tile (active = accent-soft tile + `inset 0 0 0 1px #1f242c`) (`CL:81-110`); status card = ONLINE pill + `hx-ping` dot + port + Players/Uptime 2-col grid (`CL:111-121`).
- Header: nav toggle 44px, **h1 "Claude" 32px/600/-.03em**, access pill (Read-only / Full access), spacer, cost mono 13px, History (menu 340px), New chat, rail toggle (`CL:127-151`).
- Message column: `max-width:820px`, padding 32/48, gap 24 (`CL:154`). User bubble: right, 80% max, `#14171c` + `#1f242c` border, radius 16 (`CL:159`), context chips below (`#1b1f26` pills 11px mono, `CL:160`). Assistant: 28px accent-soft tile + "Claude" 14/600 + mono meta `16:02 · 38s · $0.11` + model pill (`CL:164-169`).
- **Steps** (`CL:170-181`): a 36px ghost summary button "5 steps · 31s  read 2 files, edited 1, restarted ..." with a chevron `rotate(90deg)` (200ms ease-out). Expanded = vertical timeline, each row `[26px tile + 1px connector line] [verb + mono code target ... time]`. Tile: neutral `#14171c` border for read/run; **warn-soft** for Edit; **accent-soft** + check for the final "Watched the console, clean for 20s"; Edit row shows `+2 −1` mono. List gets `.hx-stream` (slide-in, 45ms stagger). Per-step time right-aligned mono 12px `#7d8592` (4s, 3s, 6s, 2s, 16s).
- Prose 15/24 `#c3c9d2`; inline `code` chips (`#1b1f26`, radius 6); **diff block**: card `#0e1014`, 36px header (mono "diff" + Copy ghost -> "Copied" 1.5s), body mono 13/22 with `rgba(255,107,107,.08)` removed and accent-soft added lines (`CL:184-193`).
- Thinking: three 6px accent dots with `hx-breathe` staggered .2s (`CL:246`). Streaming: text revealed 3 chars / 24ms after a 1200ms "thinking" phase, trailing 8x17 accent caret with `hx-blink` (`CL:541-549`, :249). Stop = filled `#eceff3` 36px button (`CL:313-315`); Send = accent 36px, opacity .4 when empty (`CL:316-318`, :478).
- Empty state: 56px accent-soft tile, "What should we work on?" 28/600, 4 suggestion cards in auto-fit 240px grid, each with a mono tag chip (FIX = warn-soft, others neutral), `.hx-card .hx-sug` hover lift (`CL:214-230`, :452-457).
- **Composer** (`CL:258-321`): radius 16, `#0e1014`, border `#2c323c`; textarea 15/22 rows=2; toolbar row: attach (36px ghost), Context menu button (+count pill), spacer, Read-only/Full-access segmented (3px pad, radius 10), Model menu, Effort menu, Send/Stop. Focus: border `#3a414c` + `0 0 0 4px rgba(255,255,255,.04)` (`CL:62-64`). Menus: 14px radius, `#0e1014`, border `#2c323c`, `0 18px 48px rgba(0,0,0,.55)`, mono 11px .12em section label, option rows radius 10 with 18px checkbox/check, `transform-origin` pinned to the trigger corner, 180ms `hx-menu` (`CL:267-311`).
- **Right rail** 340px, `hx-rise hx-late` (`CL:324-325`): Usage card (this-chat cost; two straight 8px meters "5-hour limit 34%", "Weekly limit 12%" using `.hx-draw`; access/model/effort chips), Changed files card (M tile warn-soft 30px, mono name + dir, `+2 −1`, 4px +/- stat bar drawn with `.hx-draw`, Restart [spins `hx-spin`, label Restarting -> Restarted] + Undo<->Redo with `scale:-1 1` icon flip, dims file to .45), Recent chats card (34px icon tile rows, hover chevron `.hx-chev`, "View all 6") (`CL:327-390`, :369-373). Both sidebars collapsible.
- History menu: row has 6px state dot (accent if current), title + mono meta (`CL:136-146`).
- Backdrop for menus is a transparent fixed `inset:0` layer, `z-index:20` (`CL:69`).

### 3.2 Shipped anatomy (`PANEL/src/views/Claude.svelte`, 3363 lines; script :1-617, markup :619-1226, style :1228-3363)
| Region | Shipped | Cites |
|---|---|---|
| Root | `.cw` 2-3 col grid (conv / canvas dock / session panel), container `conv / inline-size`, breakpoints by **container** not window | :897-911, :1262-1268, :1298-1304, :3238 |
| Topbar | **no h1**; `ChatTabs` + History menu + New chat + split toggle + design-canvas toggle + panel toggle; 44px high | :915-929, :1306-1316 |
| Log | scroll box with top fade `mask-image: linear-gradient(180deg,transparent 0,#000 22px)`; `.col` width `min(780px, 100% - 48px)`, gap 26, padding 34/0/28; `{#key claude.session}` redraws fresh on chat switch | :936-967, :1404-1423 |
| Hero | 44px mark (accent-soft + `--clay-line` border), h2 22px, 2-col starters (collapse at 560px container), rise .5s | :941-962, :1426-1515 |
| User turn | right-aligned, `.ubody` radius `16 16 4 16`, max 82%, chips as outlined pills; images as 84px thumbs | :806-812, :1524-1566 |
| Bot header | 22px mark (spins 2.4s while live), "Claude", clock + duration + cost + tokens, model pill - **details are `.det` (hidden until hover/focus)**; read-only tag | :816-828, :1568-1665 |
| Steps | live: one running row (spinner, shimmering verb, command `code`, own timer; live Edit/Write shows a growing line-numbered draft box with caret); finished: collapsed pill "N steps" + summary line, opens to a **hairline rail** (1px left border, rows 34px, mono target chips, `x3` fold counts, thought chip, Diff chip) | :672-777, :1674-1967 |
| Changes | one-line per turn (status pill, file(+N more), `+/-`, **Undo**); callouts for shared/undo error | :779-799, :2141-2208 |
| Live footer | only between steps: three-dot indicator + "Writing/Thinking · 12s · 3 steps"; sr-only status | :840-850, :1607-1636 |
| Composer | one row: attach, mic (+engine menu), Context menu, Looking-at eye chip, spacer, context **ring** (only while panel shut), **single settings menu** ("Full access · Model" -> segmented access, 3-col Model + Effort pickers, Helpers switch), Send/Stop; "Sends next:" queue line; `/` command list; drag/paste images; textarea auto-grow to 220px | :970-1132, :2303-2630 |
| Session panel | 340px, **shut by default**; **two** cards: Usage (Context/5h/weekly straight meters with `.draw`, per-model token rows, chips) + Changed files (grouped by resource, restart/open-folder icons). **History moved out** to a topbar menu | :1160-1225, :2800-3030 |
| Menus | `.menu` radius 14, bg-100, border line-200, `0 18px 48px rgba(0,0,0,.55)`, `pop .18s`, origin by side (`.right`, `.down`), `z-dropdown`; closes on outside click via `.menu-wrap` check + Esc | :2632-2665, :519-521, :255-256 |
| Split view | second chat pane (`order`), dimmed .7 until hover, click focuses it | :932-1155, :1322-1403 |

Not on the canvas (added later, listed not mapped): tabs, split view, dictation, slash commands, queue, image attach, canvas dock, Looking-at chip, thinking summaries/ticker, per-step Diff, helpers switch.

### 3.3 Canvas -> shipped deltas (the "what changed when it met the real app" table)
| Thing | Canvas | Shipped (and why) |
|---|---|---|
| Page glide | `hx-page` 10px rise **+ blur(6px)**, 520ms (`CL:19,29`) | `pageIn` rise only, 520ms; blur removed 2026-10-14 ("no page/NUI blur", `Daily/2026-10-14.md:11`; Halo README:138 "a full-area blur costs frames"); an earlier audit saw the page "look blurred for a frame" (`CX/docs/archive/full-audit-plan.md:44`) |
| Rise stagger | 70ms, base 140ms, `.hx-late` 300ms | `var(--stagger)` 60ms, base 120ms (`app.css:272-275`) |
| Stream | 300ms, base 260ms, 45ms step | `--dur-base` 300ms, base 240ms, 45ms (`app.css:277-280`) |
| Draw | 700ms, delay 480ms | `--dur-slow` 600ms, delay 400ms (`app.css:282-284`) |
| Menu open | `hx-menu` opacity + `scale:.96` 180ms, backwards | `pop` opacity + `transform:scale(.96)` .18s, `both` (:2647, :3288) |
| Press | `scale:.97` on all buttons | only `.btn`/`.iconbtn`; `.btn` uses `transform`, `.starter` hover lifts via `transform:translateY(-1px)` (:1483-1491) - **inconsistent with the "individual props" rule** |
| Streaming text | char-wise reveal + blinking block caret | rAF-paced **word-wise** reveal; each word its own span with `wfade` (blur 3px -> sharp, .32s); no caret on prose (caret only on live Edit/Write line, :1957-1967). Commit `b1d0b2f` "timeline layout with word-fade typing" (before Halo port) |
| Thinking | 3 `breathe` dots | `dot` bounce dots (:1624), shimmering verb label (:1637), spinner in the running row |
| Steps | 36px summary button; 26px icon **tiles** + connector line; per-step time on every row; tile tint by kind (warn=edit, accent=done) | pill summary; hairline rail, 15px bare icons, 34px rows, time only on the running row (`.secs`, :1813); no kind-tinted tiles. Rows fold repeats (`x3`) and attach thoughts as chips (`runs()`, `claude-steps.ts:130-150`) |
| Header | h1 32px + pill + cost | compact 44px tab strip (user 2026-10-17 "cleaner ... no clutter", `70e7e72`, `docs/ai-assist.md:59`) |
| Right rail | Usage, Changed files, **Recent chats** | Usage, Changed files; History is a topbar menu (320px). The plan still called for a History card in the rail (`halo-port-plan.md:35`); when/why it moved to the menu is unrecorded |
| Changed files in chat | removed (duplicate) | kept as one-line summary **with Undo** because the sidebar lacks it (`halo-port-plan.md:33`, :779-799) |
| Rail default | open | shut (`ui.sideOpen`, `ui.svelte.ts:68`, `aca15a1`); composer ring stands in |
| Composer | access segmented + model + effort as separate buttons | one "settings" button/menu (`a532426`) |
| Column width | 820px | 780px; hero 44/22px vs 56/28px |
| Scrollbars | hidden everywhere | hidden Oct 10, **thin Halo scrollbars restored Oct 14** (`app.css:137-161`); `.noscroll` retained for sideways strips (`:220-226`); Claude `.log` uses `scrollbar-width: thin` (:1409-1410) |
| Focus ring | white 3px (`CL:18`) | solid Halo `--shadow-focus` (2px gap + 2px accent) (`app.css:114-121`); composer box shows its own white 4px .04 halo and suppresses the textarea ring (:2334-2342) |
| Nav | fixed groups | SERVER group drag-reorderable (pointer drag + Alt+Arrow), Claude/Design pinned under ASSIST; a sliding `.plate` (translateY, .3s) behind the active row; 40px pitch, 36px items; collapsed = 56px strip with corner badges (`Sidebar.svelte:156,344-360,59-61,434-472`) |

### 3.4 What transfers to Rift directly (CSS/markup/logic with no ConsoleX store dependency)
1. The motion vocabulary in 2.1/2.2 (class names can be renamed; Rift can put keyframes in `app.css` or a Tailwind layer). Constants worth keeping: 150/300/600ms, ease-out `(.2,.8,.2,1)`, stagger 60ms capped ~10-14 children, page glide 520ms / exit 170ms.
2. Page transition mechanics: `{#key page}` wrapper + `exiting` class + 170ms timeout swap (`App.svelte:56-66`, `ui.svelte.ts:105-129`). **Rift caveat:** Rift's unit is per-tab (`TabState`, `src/lib/state/assistant.svelte.ts` per Rift CLAUDE.md), not a global view; key the glide on the route/tab, not on a global.
3. Streaming engine: `pump()` rAF word pacing with catch-up (`Claude.svelte:378-411`), `words()` per-word spans (`claude-text.ts:78-81`), per-word `wfade` (:1837, :3335), `view(m)` slice (:411), `stick`/`atBottom` follow logic with wheel/PageUp/scrollbar-grab unsticking (:435-463), `jump()` pill (:2280-2301), `follow()` smooth-scroll action with `.over` fade mask (:274-281, :1908-1915).
4. Live-turn UI: steps snippet (running row with shimmer + own timer; finished = pill -> rail), `runs()` folding, `ticker()` one-sentence thought line, `timeline()` interleaving text and step groups at reply offsets (`claude-steps.ts:130-182`). The data shape (`at` offsets, `Step{name,state,target,desc,detail,thought,draft,t0,t1}`) is ConsoleX/Claude-CLI specific; Rift's provider events would need an adapter - unknown what Rift's event model offers.
5. Composer anatomy and focus behaviour (box owns focus ring, textarea auto-height 220px, hint fades on focus, single settings menu, outside-click + Esc close) (:2303-2630, :495-501, :519-521).
6. Panel open/close pattern: animate grid track, keep inner fixed width, fade, `inert` when shut (`Claude.svelte:1160`, :1256-1260, :2801-2823), and the container-query layout (`container: conv / inline-size`, :1304).
7. `.det` hover-reveal for metadata, `stamp` for one-line change rows, `tick`/`ticker`/`line-in` micro-entrances (section 2.3).
8. Split-pane swap guard: when remounted rows would replay entrance animation, zero durations for ~450ms (`.panes.swap`, :1349-1354, :85-96). Needed whenever Rift re-parents message lists (tab drag, split).
9. Sliding-plate nav indicator (`Sidebar.svelte:156,344-360`) and badge pop (:434-452).
10. Meter/ring primitives: straight meter with `.draw` wipe (:2894-2915); composer context ring `stroke-dasharray` with `pathLength=100`, transition .6s (:1092, :2976-2992).

Does NOT transfer: ConsoleX store/`claude.svelte.ts` plumbing, Lua/NUI/dev-host, resource-restart/changed-files-by-resource grouping, Design/canvas dock, Chrome-103 build target.

---

## 4. Lessons / pitfalls

1. **Entrance animations replay on every remount.** Split-view pane swap made rows "shuffle"; fixed by `.panes.swap :global(.turn *) {animation-duration:0s !important}` for 450ms, "duration 0 (not none): rows finish at once and nothing replays" (`Claude.svelte:1349-1354`, `HANDOFF.md:7-9`, commit `51f4d02`). Also `{#key claude.session}` intentionally redraws a switched-to chat fresh (:937-938). Plan for it before any tab/split work in Rift.
2. **Blur cost.** Full-area blur on page change dropped (`Daily/2026-10-14.md:11`); only one `backdrop-filter` remains, `blur(6px)` in `Palette.svelte:185` (the command palette; I did not read what element it sits on); only other blur is the per-word `wfade` 3px (small, transient) (`Claude.svelte:3338`). Halo rule: continuous motion = opacity + transform only; blur only small/once (Halo README:146, :155). Earlier audit flagged `TopStrip` blur over drifting glows as MED perf (unmeasured) (`archive/audit-2026-10-05.md:87`) - glows were removed in the Halo restyle.
3. **No infinite animations at idle** (audit rec, `audit-2026-10-05.md:115`); remaining loops are all live-state: ping dot, spinner, shimmer, dots, caret, glow on recording. Halo: "Loops only for real activity" (README:144).
4. **Individual `translate`/`scale`/`rotate` + `backwards` fill** so keyframes never block hover (`handoff-history.md:433`). Chrome 103 (FiveM CEF) ignores the individual properties (plan:30) - hover lift/press simply do nothing in game. Same era gaps in 103: container queries/`:has()` (Chrome 105) are used in `Claude.svelte` (:3238, :1833) but that page is not in the in-game view, `oklch`/`color-mix` (Chrome 111) are used in the app (`app.css:50-55`, `Claude.svelte:1934`, `Diagnostics.svelte:618`); whether `--src-*` oklch renders in game is **unknown** (not verified). **Rift is WebView2: none of this applies** unless Rift targets an old runtime (unknown).
5. **Scrollbars.** Owner "hates visible/white scrollbars" -> hidden everywhere on Oct 10 (`halo-port-plan.md:19`, `handoff-history.md:426`). Then Halo v2.3 (Oct 14) put thin Halo scrollbars back (thumb `--line-200`, 10px, transparent 3px border, no arrow buttons, hover `--ink-faint`) with `.noscroll` only for sideways-overflow strips (`app.css:137-161`, `Halo README:124`). Claude's `.log` is `scrollbar-width: thin`. Whether the owner objected after v2.3 is **unrecorded**.
6. **Negative-margin full-bleed page** tied to the shell's page padding (see 1.3); also `.root` is ~65px taller than the window so a bottom input scrolls into view (`HANDOFF.md:21`).
7. **Duplicated width constants** between grid (`App.svelte`) and component (`Sidebar.svelte`); one source (CSS var) would avoid drift.
8. **Class-name collision:** `.ln` is both the live-draft line-number cell (`text-align:right; opacity:.7; user-select:none`, :1944-1950) and the diff line span (`<span class="ln add">`, :658, :2106-2114) - diff lines inherit `opacity:.7`. Derived from reading, not rendered. Avoid reusing short class names inside a 3k-line scoped stylesheet.
9. **Theme clobbering:** claude.ai artifact pane rewrites `<html data-theme>` -> MutationObserver guard (`main.ts:7-14`). If Rift ever previews inside an artifact, same trap.
10. **`--info` pairing:** Halo's blue preset pairs accent with lime info (`tokens.css:68-69`); ConsoleX overrode to light blue (`app.css:62-67`). If Rift charts use `chart-2`/`--info` they will go green on blue.
11. **Hand-set rgb triplets** block light themes (see 1.2).
12. **Reduced motion** is belt+braces: global kill, page-scoped kill, and JS branches (`pump` jumps to full text, `follow/jump` use `auto`) - global CSS alone does not stop rAF-driven streaming/tweens (`motion.ts:22-26`).
13. **Svelte 5 + Vite prebundle trap:** a lucide icon first imported mid-session caused two Svelte runtimes and a blank Claude page; fixed with `optimizeDeps.include: ['@lucide/svelte/icons/*']` (`vite.config.ts` comment, 2026-10-16). Rift also uses `@lucide/svelte`; check its Vite config.
14. **Size:** `Claude.svelte` is 3363 lines (2219 at port time, `Daily/2026-10-10.md:31`) and flagged as an open issue (`roadmap.md`/`HANDOFF.md:22`). Rift should split page into subcomponents (steps, composer, side panel, menus) from the start.
15. **Process:** the canvas was the design source of truth and the owner clicked through it before any port ("really see the pages", `Daily/2026-10-08.md:58`; plan: "User plays canvas v18 ... and signs off. Phase C waits on that, or it gets done twice", `halo-port-plan.md:11`). A later `npm run build:demo` produced a no-server clickable build of the real panel published as a claude.ai artifact for review (`HANDOFF.md:25,37`) - a reusable review loop.
16. Agent checks only prove it builds, not that it looks right: "every phase ends with the user opening localhost:5173" (`halo-port-plan.md:16`); the in-app browser pane at fixed 1440x900 was used for self-review (`handoff-history.md:364,376`).
17. Gate used per phase: `npm run check` (0/0), `npm run test`, `npm run build` (`halo-port-plan.md:16`); later `verify` adds knip.

---

## 5. Owner reactions recorded

| Reaction | Source | Result |
|---|---|---|
| Old sidebar status dot/blob = "sloppy" | `handoff-history.md:425` | ONLINE pill + box-shadow ping ring + port + Players/Uptime grid |
| Hates visible/white scrollbars | `halo-port-plan.md:19`; `handoff-history.md:426` | hidden everywhere (later thin Halo ones, section 4.5) |
| Rejected ring gauges ("wheels") on the Claude rail | `handoff-history.md:427` | straight 8px meters; (a small context ring later returned in the composer and was accepted - the owner "liked" the cleanup, `HANDOFF.md:20`) |
| Wanted to "really see" pages before porting | `Daily/2026-10-08.md:58` | motion on all 7 canvas boards, Play-mode click-through, interactive Claude board (v18) |
| Chose blue, not ember (ember warn = danger red) | `Daily/2026-10-08.md:52`, `handoff-history.md:457` | `data-theme="blue"` |
| Looked at it, then OK'd merge; confirmed in-game "no grey/black screen"; confirmed frameless window | `roadmap.md:13`, `handoff-history.md:363,365` | v0.1.0 |
| "cleaner and more clear ... no clutter" (2026-10-17) | `docs/ai-assist.md:59` | one-row composer, panel shut by default, hover-only reply metadata, 44px toolbar, compact sidebar |
| "No green anywhere" (2026-10-17) | `app.css:62-63` | `--info` light blue override |
| Liked split view + the cleanup on the canvas | `HANDOFF.md:20` | not yet tried with real turns in the app |
| "Don't rebuild" (session 36) | `HANDOFF.md:1` | panel served from working tree on relaunch |
| Restyle, don't rebuild the logic | `halo-port-plan.md:7`, `Daily/2026-10-10.md:17` | decision: keep menus/steps/streaming, change markup + styles |
| Nav: keep drag-reorder for server views, pin Claude | `halo-port-plan.md:26` | `Sidebar.svelte:28,41-95` |
| Burnt out, handed over full autonomy (Session 34, Oct 17) | `HANDOFF.md:51` | context only (the Oct 17 "calmer page" pass followed; owner feedback on it is the quote above and "liked") |
No recorded owner reaction to: per-word blur fade, page glide specifically, shimmer, nav plate sliding. Treat those as AI-chosen, owner-tolerated.

---

## 6. Reusable code (path:line, lift verbatim unless noted)

| What | Where | Notes |
|---|---|---|
| Token alias + override block | `PANEL/src/app.css:1-67` | replace hand-set rgb with `color-mix`; drop names Rift never had |
| Global reset + focus + selection + scrollbar | `app.css:69-165` | scrollbar block :137-161 |
| `.legend` section label | `app.css:209-218` | mono 11px/.12em/uppercase |
| `.panel` card, `.btn`/`.iconbtn`/`.field`/`.led`/`.trough` | `app.css:228-496` | `.btn:active` uses `transform`; switch to `scale:` if consistent behaviour wanted |
| Motion helper classes | `app.css:247-299` | `rise/stream/draw/ping/lift/enter` |
| Keyframes + reduced-motion | `app.css:500-615` | |
| `tween` / `decode` / `reducedMotion` | `PANEL/src/lib/motion.ts:1-72` | zero deps |
| Page-glide shell | `PANEL/src/App.svelte:55-66,118-128` + `PANEL/src/lib/ui.svelte.ts:105-129` | re-key per Rift route/tab |
| Token generator | `PANEL/src/host/design-tokens.ts:1-45`, `PANEL/design-systems/build-tokens.ts:1-11` | only needed if Rift wants `tokens.json` as source; otherwise vendor `tokens.css` |
| Sidebar (nav, plate, badge, status card, collapse) | `PANEL/src/components/Sidebar.svelte:139-232` markup, :234-685 style | drag-reorder script :41-103 optional; depends on `store`/`ui` |
| Streaming engine | `Claude.svelte:378-411` (pump/view), :435-463 (stick), :274-281 (follow) | replace `claude.*` with Rift's `TabState` |
| Word spans | `PANEL/src/lib/claude-text.ts:78-98` (`words`, `inline`) | |
| Step shaping | `PANEL/src/lib/claude-steps.ts:130-182` (`runs`, `ticker`, `timeline`) | Step type is ConsoleX's |
| Steps markup | `Claude.svelte:672-777` + style :1674-1967 | |
| Bot header / det / live footer | `Claude.svelte:816-850` + :1568-1672, :1607-1636 | |
| Prose / code / diff / table styling | `Claude.svelte:1978-2130` (`.prose`, `.code`, `.code-h`, `.copy`, `.ln.add/.del/.hunk`) | fix `.ln` collision (4.8) |
| One-line changes row | `Claude.svelte:779-799`, :2141-2208 | |
| Composer + menus | `Claude.svelte:970-1132`, style :2303-2780 (settings menu :1097-1117, `.seg` :2461-2491, `.pick` :2512-2539, `.sw` :2544-2571, `.send` :2572-2609, `.menu` :2636-2665) | |
| Session panel cards + meters + ring | `Claude.svelte:1160-1225`, style :2800-3030 | |
| Hero / starters | `Claude.svelte:941-962`, style :1426-1515 | |
| Container-query + narrow drawer | `Claude.svelte:73-78`, :1297-1304, :3212-3246 | |
| Canvas HTML for pixel reference | `CL:68-395` (static markup, inline styles with exact px/hex) | scratchpad path; copy out if needed |
| Exists but NOT read this wave | `PANEL/src/lib/tip.svelte.ts` (135 lines) + `components/Tip.svelte` (150), `Select.svelte` (263), `ChatTabs.svelte` (162), `Palette.svelte`, `Boot.svelte`, `Dock.svelte` | custom tooltip layer replaced all native `title`s (`HANDOFF-history` Session 30); unknown internals |

---

## Risks (carry to synthesis)
- Alias strategy kept the old vocabulary alive; Rift should decide up front whether to alias or rename (ConsoleX still has `--clay`, `--amber`, `--w-*` page-local layers over Halo vars, `Claude.svelte:1230-1246`).
- Halo's `[data-theme="blue"]` is incomplete by design (8 vars): any Rift code reading `--info` / `--chart-2` / `--chart-3` gets lime/violet-ish values unless overridden.
- Canvas was a 1440x1000 mock with fixed 1000px column and hidden scrollbars; real app needed container queries, narrow drawers, split panes, hover-hidden metadata, a queue/stop/follow model, none present on canvas.
- Streaming word-fade with blur is the one place blur is on a hot path (per word, 320ms); cost on long replies is **unmeasured** in the source.
- Entrance-animation replay on remount is a recurring bug class (split swap, chat switch).
