# 09 - Motion inventory (whole front end, `src/**`) vs Halo doctrine

Read-only map. Branch `design/chat-ledger`. Sources: Grep over `src/**` (svelte/css/ts) + targeted Reads. All paths relative to `C:/AI Workflow/projects/rift-tauri/src/` unless prefixed `halo:` (= Halo brand-book dir) . Line numbers = this working tree on 2026-10-19. Anything not read is marked `(body not read)` or `unknown`.

Headline numbers (grep, first-line-of-declaration counts, so multi-line `transition:` lists count once):
- 60 files carry motion CSS/JS. 173 `@keyframes` definitions / ~155 distinct names (real name clashes: `blockIn` stream.css:27 + SettingsPage:1590, `workOpen` stream.css:33 + MessageBubble:1006, `acRoll`/`acRollDown` stream.css:38-39 + AnimatedCount:102-109, `drop-in` AssistantPane:1556 (opacity only) vs Composer:1897 (scale .94), `slash-in` x3 (EnhanceBar:215, MentionPopover:56, SlashMenu:152 - bodies differ), `slash-item-in` x2, `spin` x3 (app.css:625, onboarding.css:362, UpdateDialog:557), `chip-spin` x4).
- 225 non-`none` `animation` declarations, 115 of them `infinite`. 328 non-`none` `transition` declarations.
- 25 separate "rotate 360deg" spinner keyframes (16 single-line + `chip-spin` x4 + `perm-spin`, `bh-spin`, `sask-spin`, `sxa-spin`, UpdateDialog `spin`) at 0.8s / 0.9s / 1s / 1.1s. Halo has one: `halo-spin` 900ms.
- Durations: ~570 sites already go through `--dur-fast` (453) / `--dur-base` (117) / `--dur-slow` (5) / `--dur-rise` (8) / `--pulse-live` (19). The remainder is ~150 raw literals (70+ distinct values) and raw easings. **Re-pointing 3 tokens retunes ~570 sites at once; the work is in the literals, the easing keywords, the 115 loops and the structural gaps (section 5).**
- Reduced motion: 48 files mention `prefers-reduced-motion` (CSS media block or the JS probe); 13 files with motion CSS/directives have no handling at all; coverage inside the other files is partial (7.4); and the JS guard is a one-time snapshot (section 7).
- 16 `backdrop-filter` declarations, 3 of them on `position:fixed` layers in direct contradiction of the project's own WebView2 ban (section 6).

## Legend

Token shorthand (Rift today -> Halo target):

| Rift token (`app.css:239-250`) | Today | Halo token (`halo:tokens.json:532-570`, `tokens.css:43-48`) |
|---|---|---|
| `--dur-fast` (d-f) | 140ms | `dur-fast` 150ms |
| `--dur-base` (d-b) | 240ms | `dur-base` 300ms |
| `--dur-slow` (d-s) | 520ms | `dur-slow` 600ms |
| `--dur-rise` (d-r) | 460ms | none (Halo rise = `dur-slow` 600ms, 14px) |
| `--dur-page` / `--dur-page-out` | 400ms / 140ms | page = 520ms + 60ms delay, no exit (`halo:bundle.css:24`) |
| `--stagger` | 62ms | `stagger` 60ms, capped ~12 items |
| `--pulse-live` | 1.6s | LiveDot ping 1.8s (`bundle.css:80`) |
| `--ease-page` (e-p) `cubic-bezier(0.22,1,0.36,1)` | decelerate, "expo-ish" | `ease-out` `cubic-bezier(0.2,0.8,0.2,1)` (softer landing) |
| `--ease-soft` (e-s) `cubic-bezier(0.4,0,0.2,1)` | standard | folds into `ease-out` |
| `--ease-spring` (e-sp) `cubic-bezier(0.34,1.56,0.64,1)` | soft overshoot | `ease-spring` **identical value** - keep |

Row columns: `ID | path:line | what (selector) | moves (props) | dur · ease · delay | trigger · loop | RM | verdict`.

- **RM** (reduced motion): `Y` handled by an in-file `@media (prefers-reduced-motion: reduce)` or `*:none`; `J` handled by JS `prefersReducedMotion()` snapshot; `G` caught by the global `[data-slot]` rule (`app.css:120-122`, bits-ui primitives only); `NP` the animation is only *defined* inside `@media (prefers-reduced-motion: no-preference)`; `N` no handling found in that file; `-` not motion that needs one (pure colour hover).
- **Verdict**: `K` keep. `Tk` token-follow: re-pointing the three duration tokens + switching `ease`/`ease-out`/`ease-in-out` keywords to Halo `ease-out` retunes it with no per-site edit. `Te` per-site retune of a raw literal to a token. `H:<pattern>` replace with a Halo pattern (key below). `X` kill. `D<n>` needs an owner call (section 4 list).
- Halo pattern key: `RISE` halo-rise (translateY 14px, d-s, stagger<=12 x 25-60ms, `bundle.css:22-23,247,370,679`) · `PAGE` halo-page-in 520ms/60ms delay/10px (`:24-25`) · `POP` halo-pop-in (d-f, scale .96, y -4px; `-up` +4px; `:406-413`) · `MENUITEM` halo-menu-item 220ms, `min(i,12)*18ms`, y -3px (`:419-420`) · `ZOOM` dialog halo-zoom d-b scale .94 y 8px, leave 200ms scale .96 (`:285-292`) · `DRAWER` halo-drawer-in d-b translateX 40px, leave 200ms (`:362-364`) · `TOAST` halo-toast-in d-b y16 scale .96 + stack/fan/drag, leave 240ms (`:318-322`) · `SKEL` shimmer 1.4s linear translateX + `halo-reveal` blur 6px->0 d-s + `halo-skel-out` 360ms (`:100-107`) · `PING` halo-ping 1.8s scale 1->3 opacity .7->0 (`:80-82`) · `SPIN` halo-spin 900ms linear (`:448`) · `INDET` halo-indet 1.3s (`:460`) · `THUMB` sliding underline/thumb transform+width d-b (`:92,266`) · `ACC` accordion grid-template-rows 0fr->1fr d-b + content fade, y -4px, 60ms delay (`:499-504`) · `CHK` check draw d-b + halo-pop spring (`:154-156`) · `ROLL` RollingNumber 900ms per-digit strip (`:737`) · `COUNT` Stat count-up 1100ms ease-out (`halo:components/Stat/README.md`) · `FLIP` FlipList 460ms ease-out (`FlipList/README.md`) · `HOV` hover/press d-f ease-out (colour, border, 3px lift, 0.97 press) · `FADE` halo-fade.

---

## 1. Full inventory

Rows are per declaration. Consecutive identical hover/colour declarations in one file are merged into one row with all `selector:line` pairs listed (every site still cited). `none` resets are folded into the RM column of the row they reset.

### 1A. Tokens + globals (`app.css`, `styles/*.css`)

| ID | path:line | what | moves | dur · ease · delay | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| A1 | app.css:2 | `@import "tw-animate-css"` | provides `animate-in/out`, `fade-in-0`, `zoom-in-95`, `slide-in-from-*` utilities used by bits-ui primitives | lib defaults; recipes override to d-f | open/close of dropdown/select/context/alert-dialog | G | H:POP (retune recipe values, A2/H1) |
| A2 | app.css:117-122 | global `[data-slot]{animation:none!important}` | kills primitive enter/exit | - | prefers-reduced-motion | G (primitives only) | K, but add Halo-style global net (section 7) |
| A3 | app.css:239-250 | motion token block | defines e-p, e-s, e-sp, d-page 400, d-page-out 140, d-rise 460, stagger 62, d-f 140, d-b 240, d-s 520, pulse-live 1.6s | - | - | - | replace values per legend table |
| A4 | app.css:326 | `::-webkit-scrollbar-thumb` | background | d-f · ease-out | hover | - | Tk |
| A5 | app.css:347-354 | `@property --rail-w` + `html{transition:--rail-w}` | animates a registered `<length>` custom property on `<html>` | `var(--dur-page-out,240ms)` = **140ms** (token wins) · e-s | sidebar collapse; NP only | NP | **X** - see 5.6 + 6 (root custom-prop transition; duration disagrees with Sidebar's 240ms) |
| A6 | app.css:384,390 | `.slider-bubble` (+`.on`) | opacity, translate+scale .7->1 | d-f · ease-out / e-p; reduce: 80ms opacity | slider drag | Y (389-392) | Te: spring (slider thumb is a "grab" thing) |
| A7 | app.css:403 | `.btn` | background, border, shadow, colour d-f `ease`; transform 80ms `ease` (press) | d-f/80ms | hover/press | - | HOV (Te: press 80ms -> d-f, add 0.97 scale; `ease` -> ease-out) |
| A8 | app.css:431 | `.input` | border, shadow | **80ms** `ease` | focus | - | Te d-f |
| A9 | app.css:458-465 | `.pill.ok .dot` pill-breathe | box-shadow ring 2px<->4px | 2.6s · ease-in-out · infinite | status "ok" pill, always on; NP | NP | **X** - decorative loop on a non-activity state (Halo: loops only for real activity) |
| A10 | app.css:480,488 | `.rift-toggle` / `-knob` | bg+border d-b; knob transform d-b **e-p** | d-b | toggle flip | - | H:Toggle (knob d-b e-sp, width d-f, `bundle.css:141-142`) |
| A11 | app.css:531 | `.tile` | border, bg, shadow | d-b · e-s | hover | Y (604-606) | Tk |
| A12 | app.css:569,592 | `.tile-glyph` colour/bg d-f; `.tile-chev` transform+colour d-f e-s | chevron rotate | d-f | disclosure | Y | Tk (Halo chevron = transform d-b ease-out, `bundle.css:497`) |
| A13 | app.css:612-618 | `.dialog-overlay` overlay-in (keyframe at 626, inside NP block) | opacity 0->1 | d-f · ease-out · both | dialog mount (UpdateDialog only) | NP | H:ZOOM overlay |
| A14 | app.css:624,627 | `.pulse` pulse-soft | opacity 1<->.55 | 2s · ease-in-out · ∞ | utility class; NP | NP | X (unused-by-grep? usage not checked) |
| A15 | app.css:625,628 | `.spin` | rotate | 0.8s linear ∞ | spinner utility; NP | NP | H:SPIN (900ms) |
| A16 | app.css:634-650 | `.enter` + `@keyframes enter` | opacity + translateY 4px; `animation-delay: idx*35ms` | d-b · cubic-bezier literal (= e-p) · both | many mounts (composer-bar, alerts, ToolChip, bubble...) | Y (648-650) | H:RISE for blocks/cards; keep a small-y variant for dense chat rows (D8) |
| A17 | app.css:640-643 | `@keyframes pop-in` (y 8px scale .96) | SettingsMenu popover | d-b · e-p | menu open | Y (via SettingsMenu 962-969) | H:POP |
| A18 | app.css:676-678 | `.gh-dot.busy` gh-dot-breathe | opacity 1<->.35 | pulse-live · ease-in-out · ∞ | CI running (real activity) | Y | H:PING (warn tone) |
| A19 | app.css:719,724 | `.rift-menu-row` bg, `-ic` colour | colour | d-f · `ease` | hover | - | HOV Tk |
| A20 | styles/settings-controls.css:16,25-27 | `.st-btn` colours d-f; `.st-spin` | rotate | 0.8s linear ∞ | busy | Y (27) | Tk; spinner H:SPIN |

### 1B. App shell, chrome, splash, lists

| ID | path:line | what | moves | dur · ease · delay | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| B1 | components/AppShell.svelte:360 | `.app` background | gradient background colour | **1s** · e-s | background change (theme/accent; trigger not read) | N | X or Te d-s (**D9**) |
| B2 | AppShell.svelte:417 | `.main` margin-left | **layout** margin | d-b · e-s | rail collapse | N | H:AppShell-collapse (one coordinated width move) |
| B3 | AppShell.svelte:483-486 | `.workspace` wsIn | translateY 7px (no opacity) | d-page 400 · e-p | `.workspace` mount (boot; the element persists across page changes - inferred, not run) | Y | X (superseded by WorkspaceShell page motion; PAGE) |
| B4 | AppShell.svelte:493-501 | `[data-intro]` veil/handoff | `.main` opacity 0->1 + translateY 12->0 | d-r 460 · e-p · 60ms delay | cold boot only | J (intro.svelte.ts:11 skips arm) | K signature; Te d-s/ease-out |
| B5 | shell/WorkspaceShell.svelte:136-148 | `.ws-page` / `[data-active]` | opacity + translateY 3px; in d-page 400 e-p, out d-page-out 140 e-s | page switch | Y (190) | H:PAGE (in 520ms/10px/60ms; keep a short opacity-only out - pages are stacked) |
| B6 | WorkspaceShell.svelte:160-191 | `.ws-load-skeleton` ws-load-shimmer | background-position sweep on 3-4 bars | 1.4s · ease-in-out · ∞ | workspace chunk loading | Y (191) | H:SKEL **D1** |
| B7 | WorkspaceShell.svelte:198-211 | `.ws-page.rising > *` ws-child-rise | opacity + translateY 6px, first 5 children delayed `nth-child x --stagger(62)` | d-r 460 · e-p · 0-248ms | page switch, `.rising` class for 810ms (SETTLE_MS :48, timer :69-72); NP | NP | H:RISE (14px, d-s, stagger 60) |
| B8 | shell/Sidebar.svelte:221-222 | `.side-rail` | **width** (layout) | d-b · e-s; none while resizing | collapse/expand | N (349-351 omit it) | H:AppShell-collapse; add RM |
| B9 | Sidebar.svelte:223,227,274 | `.sidebar` collapsed/peek | translateX(-16px)+opacity, `top`, box-shadow | transform d-b e-s · top d-b e-s · opacity d-b e-p · shadow d-b e-p | collapse, hover-peek | N | Tk (single ease; drop `top`) |
| B10 | Sidebar.svelte:238-243 | intro veil/handoff | translateX(-16px)->0 + opacity | d-r · e-p | cold boot | J | K, Te |
| B11 | Sidebar.svelte:247,296,308,324,338 | resize handle, collapse btn, `.ic-btn`, `.fnav-item`, `.new-chat` | bg/colour/border; `:active` scale .96 / translateY(1px) | d-f | hover/press | Y for 324 only (349-351) | HOV Tk |
| B12 | Sidebar.svelte:300 (+328 translateY -1px) | `.snav-ic` | icon translate on hover | d-f · e-p | hover | Y | HOV Tk |
| B13 | shell/Topbar.svelte:57-59,63,67 | `.tb-left` enter; `.tb-ic`, `.wc` hovers | opacity+4px | d-b · e-p | mount | Y | Tk |
| B14 | shell/Titlebar.svelte:72 | `.wb` window buttons | bg, colour | d-f · e-s | hover | N (no media; colour only) | HOV Tk |
| B15 | SplashOverlay.svelte:149,165-166 | `.panel` halves | translateX(-/+101%) | **520ms** `cubic-bezier(0.65,0,0.35,1)` · 140ms delay | exit when ready && MIN_MS(900) | Y (284-288) | K signature (named exception **D10**) |
| B16 | SplashOverlay.svelte:177-184 | `.lockup` | opacity + **filter blur 3px** | 240ms/280ms `(0.4,0,0.2,1)` | exit | Y | K |
| B17 | SplashOverlay.svelte:189-194 | `.mark` settle-mark | opacity, y 8px, scale .94, blur 8px->0 | 560ms e-p · 140ms | boot | Y (290) | K (Halo SKEL-reveal analog) |
| B18 | SplashOverlay.svelte:215-221 | `.wl` letter-rise x4 | opacity, y 10px, blur 4px | 460ms e-p · `300+i*55ms` | boot | Y | K |
| B19 | SplashOverlay.svelte:236-243 | `.boot` readout | opacity, y 4px; exit fade 200ms | 400ms e-p · 80ms | slow boot (>1400ms) | Y | K |
| B20 | SplashOverlay.svelte:269-281 | `.bootline` stage-in; `::after` caret-blink | y 3px; caret | 260ms e-p; blink 1.1s `steps(1)` ∞ | stage change | Y | K / caret: X? (loop on a transient readout; low cost) |
| B21 | SplashOverlay.svelte:284 | `.splash` reduced | opacity | 100ms linear | reduce | Y | K |
| B22 | shell/ProjectSwitcher.svelte:233,251,271 | switcher/menu hovers | border/bg/colour | d-f | hover | - | HOV Tk |
| B23 | ProjectSwitcher.svelte:238-241 | `.sw-skel` sw-skel-in | opacity | 240ms e-p | loading | Y | Te d-b |
| B24 | ProjectSwitcher.svelte:269,284-285 | `.sw-menu` swPopIn (`-global-` keyframe) | opacity, scale .97, y -4px, origin top-left | d-f · e-p | menu open | Y | H:POP |
| B25 | shell/ConversationList.svelte:419-433 | `.showmore`, `.sm-rule`, `.sm-ct`, `.sm-ch` | colour/bg; chevron rotate 0.22s e-p | d-f; 220ms | hover/expand | Y (433 only chevron: `transform 0s`) | Tk |
| B26 | ConversationList.svelte:471-475 | `.cl-skel` cl-skel-in | opacity | 240ms e-p | loading | Y | Te |
| B27 | ConversationList.svelte:478-479 | `.crow` hover + rowIn | bg/colour; opacity + y 3px | d-f; 220ms e-p · `min(i,12)*14ms` | row mount (list render) | Y (528 `!important`) | H:RISE-row (Halo table rows: d-b, `min(i,12)*25ms`) |
| B28 | ConversationList.svelte:484 | `.crow.on::before` barPop | scaleY .25->1 | 280ms e-p | selection | **N** (528 matches `.crow`, not the pseudo) | Te d-b; add RM |
| B29 | ConversationList.svelte:492,499,502 | `.crow-time` opacity; `.crow-act` hover; `.crow-acts` actsIn | opacity/bg; translateX 5px on `display:none->flex` | d-f · e-p | hover | Y (528) | HOV Tk |
| B30 | ConversationList.svelte:515-530 | `.conv-menu` convPopIn | opacity, scale .97, y -2px | d-f · e-p | menu open | N (528 covers rows only) | H:POP |
| B31 | ConversationList.svelte:517 | `.pop-item` | bg, colour | d-f | hover | - | HOV |
| B32 | ConversationList.svelte:536-543 | `.workdots i` workPulse (3 dots) | opacity .16->1, scale .5->1 | 1.05s e-s ∞ | chat is working (real activity) | Y | K (Halo has no 3-dot; **keep as live indicator**, Te d-s) / or H:PING |
| B33 | shell/NotificationCenter.svelte:113,125,170 | svelte `fade` 120 (badge), `fly y:6` 160 (panel), `fade` 120 (rows) | opacity, y | 120/160 ms · cubicOut default | open/close | J (`reducedMotion?0:`) | H:DRAWER+NOTIF (bell swing 900ms, drawer 40px, rows `min(i,8)*50ms`) |
| B34 | NotificationCenter.svelte:221,288,317,329,373,387,401 | bell, `.ph-btn`, `.row`, `.row-dot`, `.row-time`, `.row-action`, `.row-close` | bg/colour/opacity/shadow | d-f | hover | Y (419) | HOV Tk |
| B35 | shell/UpdateBanner.svelte:159 | svelte `slide` 200 | height | 200ms cubicOut | banner mount | J | H:ACC (or Halo Banner `bundle.css:387` halo-rise d-b) |
| B36 | UpdateBanner.svelte:238,258,296,317 | progress width d-b ease-out; `.ub-ic .spin` ub-spin 0.8s; cta/x hovers | width, rotate, bg | d-b / 0.8s linear ∞ / d-f | download progress; busy | Y (328-331) | Te: progress = `transform:scaleX` (Halo stepper fill `bundle.css:542`); H:SPIN |
| B37 | shell/Skeleton.svelte:42-56 | `.sk::after` sk-sweep; reduce -> `.sk` sk-breathe | translateX -100->100 (transform: compositor-ok); reduce: opacity .55<->.85 ∞ | 1.6s e-s ∞ / 2.4s ease-in-out ∞ | skeleton visible | Y but **reduce still loops** | H:SKEL **D1** |
| B38 | ToastHost.svelte:52-54 | `animate:flip` 180; `in:fly y:-10` 220; `out:fly y:-6` 140 | position, opacity, y | 180/220/140ms cubicOut | toast push/dismiss | J | H:TOAST |
| B39 | ToastHost.svelte:130,200 | `.toast` border/transform/shadow d-f e-s; `.close` | hover | d-f | hover | Y (209-211) | HOV |

### 1C. Chat surface (pane, welcome, dock)

| ID | path:line | what | moves | dur · ease · delay | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| C1 | assistant/AssistantPage.svelte:15-22,307 | `transition:dockSlide` (custom) | **width** px + opacity; the native child webview tracks via ResizeObserver | 220ms `cubicOut` | browser dock open/close | N | H:DRAWER-ish but width-driven on purpose (webview sync) - K with Te ease-out; **add RM** |
| C2 | AssistantPage.svelte:391,425,444 | dock divider / `.divider::before` width / grip | bg, width, opacity | d-f · ease-out | hover/drag | N | HOV |
| C3 | assistant/AssistantPane.svelte:879,938,950,994,1092,1328,1363,1399,1420,1470,1497,1534 | pane/head/folder/max-close/empty-row/jl-ic/tr-chev/alert.notice/recovery-btn/drop-zone | bg/border/colour/opacity/transform on `.pane-empty-row-chev` | d-f · ease(-out) | hover | - (1534 drop-zone bg) | HOV Tk |
| C4 | AssistantPane.svelte:887-919 | `.pane.split` border/shadow/opacity d-b e-s; paneIn (opacity+y 4px, `backwards`); `::before` opacity+transform d-b e-s | split-pane focus + mount | d-b | split focus | Y (917-920) | Tk; paneIn H:RISE |
| C5 | AssistantPane.svelte:1022 | `.pane-ctx-fill` | **width** | d-b · ease-out | ctx % change | N | Te: `scaleX` (Halo Progress/stepper fill) |
| C6 | AssistantPane.svelte:1042-1049 | `.pane-live-dot` pane-live-pulse | **box-shadow** ring 0->4px | pulse-live · ease-out · ∞ | pane streaming | Y | H:PING (pseudo-element, transform+opacity) |
| C7 | AssistantPane.svelte:1078-1081 | `.pane-done` pane-done-in | opacity + y -2px | d-b · e-s · backwards | turn done badge | Y | Tk / H:POP |
| C8 | AssistantPane.svelte:1148-1149 | `.csurf-col.is-home .composer-host` enter | opacity + y 4px | d-b · e-p · **200ms delay** | home mount | Y | H:RISE |
| C9 | AssistantPane.svelte:1160 | `.composer-host{will-change:transform}` | permanent layer promotion | - | always (composer lives for the app's life) | - | X (set only during FLIP, see J5) |
| C10 | AssistantPane.svelte:1225-1230 | `.scroll-fade-top` | opacity | d-b ease-out | scrolled>8px | Y | Tk |
| C11 | AssistantPane.svelte:1254-1269 | `.pane-empty-card` pane-empty-in | opacity, y 6px, scale .985 | 320ms e-p (literal) | empty pane | Y | H:RISE |
| C12 | AssistantPane.svelte:1380-1407 | `.jump-latest` jump-in + colour/border | opacity + y 8px | d-b e-p | latch lost | Y | H:POP-up |
| C13 | AssistantPane.svelte:1413 | `.turnrail` | opacity, bg, shadow | d-b | hover/active exchange | N | Tk |
| C14 | AssistantPane.svelte:1432 | `.tr-tick::after` | **width**, bg, opacity | d-f | hover | N | Te: transform |
| C15 | AssistantPane.svelte:1448-1513 | `.alert` enter (opacity y 4px) | alert mount | d-b e-p | Y (1512) | H:RISE |
| C16 | AssistantPane.svelte:1520-1561 | `.drop-zone` drop-in (opacity) + bg/border | file drag | d-f e-p | Y | HOV/FADE |
| C17 | assistant/AssistantWelcome.svelte:427-432 | `.wel-inner > *` enter | opacity + y 4px, **320ms literal**, `both` (no stagger found - none set) | 320ms e-p | welcome mount | Y | H:RISE with stagger (Halo "things enter once, in a stagger") |
| C18 | AssistantWelcome.svelte:447-453 | `.wel-mark` glyph-breathe | scale 1<->1.04 | **4.2s** ease-in-out ∞; NP | idle welcome (decorative) | NP | **X** (loop for attention; Halo README principle 4) **D11** |
| C19 | AssistantWelcome.svelte:470-473 | `.ge-dot` ge-breathe | opacity 1<->.45 | 3.4s ∞; NP | greeting dot (decorative) | NP | X |
| C20 | AssistantWelcome.svelte:488,506,535,539,541,643,667,673,688 | greet-switch, resume-item, newrift-hide/toggle/chev, folder-row(+go,+x), primer-step | colour/bg/opacity/transform | d-f (667: + e-p) | hover | - | HOV Tk |
| C21 | AssistantWelcome.svelte:509,522 | `.resume` border d-b e-s + transform d-b e-p; `.ri-go` opacity d-f + transform d-b **e-sp** | hover lift / arrow nudge | d-b | hover | Y (524-527) | HOV (spring on a hover arrow: Halo reserves spring for flip/grab -> Te e-o) |
| C22 | AssistantWelcome.svelte:544,557 | `.newrift-body` gcReveal (opacity y -3px) | accordion body | 0.3s e-p | disclosure | **N** | H:ACC |
| C23 | AssistantWelcome.svelte:599,608,619 | `.of-target`, `.oft-corner`, `.oft-badge` | border, transform (hover lift), opacity | d-f e-p | hover | - | HOV (Halo lift 3px) |

### 1D. Stream renderer (`styles/stream.css` + `assistant/stream/*`) - the default live/settled turn (`uiPrefs.streamMode` defaults true, ui-prefs.svelte.ts:105; AssistantPane.svelte:717)

| ID | path:line | what | moves | dur · ease · delay | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| D1 | stream.css:27 (+ used 73,90,104,117,185,214,227,262,320,353,451,483,525,546; StreamAgent:204, StreamAskUser:324, StreamShell:227) | `blockIn` entrance on ~18 block classes | opacity + translateY **3px** | d-b · e-p · both | every block mount | Y (372-384 lists most; **missing**: `.ssteer`:117, `.wline-row`:279, `.wr-out`:304, `.sxa-*`; StreamShell `.sshell`:227, StreamAskUser `.sask`:324) | H:RISE-dense (keep small y for stream; owner cut 8px->3px 2026-10-19, stream.css:25-26) **D8** |
| D2 | stream.css:31 | `.ssh-outwrap > *, .sr-out > *` bodyFadeIn | opacity | 200ms e-s · 40ms | output body open | Y (32) | H:ACC content fade (d-b, 60ms) |
| D3 | stream.css:33 / 273,279,304 | `workOpen` on `.wline-list` 0.3s, `.wline-row` 0.24s, `.wr-out` 0.24s (+StreamAgent:250 `.sa-body` 0.3s) | opacity + y -4px (opens downward) | 240-300ms e-p · both | ledger expand | Y for list; **N** rows/out | H:ACC |
| D4 | stream.css:34,464-466 | `ringspin`, `.splan-ring` 0.9s, `.sa-spin` StreamAgent:219 1s, `.ahud-spin` ActivityHud:412 1s | rotate | 0.9-1s linear ∞ | live | Y (377; StreamAgent 325; ActivityHud 457) | H:SPIN |
| D5 | stream.css:35,341 | `shim` on live text: `.sturn-head.live .sh-label`, `.wline-active .wa-text`, `.sfooter .sf-verb`, `.wbrow.active .wb-label`, `.splan-item.is-active .splan-text` | `background-position` on `background-clip:text` | 1.5s linear ∞ (up to 5 simultaneous) | a live line | Y (372-375 -> static fg-muted) | K signature, owner-approved verb shimmer (DESIGN.md:233-234) - perf flag in 6.4 **D3** |
| D6 | stream.css:38-39,366-367; AnimatedCount.svelte:98-109 | `acRoll`/`acRollDown` glyph roll (duplicated twice) | translateY +/-0.62em, opacity .6->1, `clip-path` | d-f · e-p | each rAF settle of a live number | Y (376, AnimatedCount 111) | H:ROLL (Halo rolls per-digit strip 900ms) - see J7 |
| D7 | stream.css:40-44 | `streamLivePulse` (box-shadow 0->7px ring) used at **5 sites**: 189 `.sf-dot`, 321 `.wa-dot`, 370 `.wb-dot.live`, 80 `.sh-dot` (after dotPop, 420ms delay), StreamAskUser:342 `.sask-dot` | **box-shadow** | pulse-live 1.6s · ease-out ∞ | live dots | Y (383; StreamAskUser **N**) | H:PING (halo-ping on a pseudo-element: scale+opacity, compositor-only) |
| D8 | stream.css:78,80,82,83-88,460; StreamAskUser:390 | `dotPop` (scale 0 -> **1.55** -> 1), `headRise` (x -8px) | scale overshoot, translateX | 420ms / 380ms e-sp · 80ms | status dot appears; plan item done; ask option selected | Y (383, 380) except StreamAskUser:390 N | Te: halo-pop (scale 1.18, d-b) **D5** |
| D9 | stream.css:196-204 | `.sf-verb-wrap` verbIn | opacity | d-f e-s | verb swap | Y | K |
| D10 | stream.css:226,233 | `.scompact-card` border; `.scompact-fill` transform scaleX | compaction bar | 900ms e-s | compaction | Y (382) | Te d-s/ease-out (already transform: good) |
| D11 | stream.css:260-264 | `.wline` blockIn + **`transition: width`** (fit-content -> 100%) with `interpolate-size: allow-keywords` | **layout width** morph | d-b · e-p | tool-run open/close | **N** (not in 372-384) | H:ACC (height via grid rows) and drop width morph |
| D12 | stream.css:162,284,287,350,399,412,415,440,494,533 | hovers/colour/opacity: `.sthink-row`, `.wline-row.wr-click`, `.wr-chev` transform, `.wbatch`, `.sledger-toggle`, `.sledger-hover`, `.sledger-acts`, `.msg-act`, `.sxa-btn`, `.sweb-chip` | colour/bg/opacity/transform | d-f | hover | Y for 412/415/399 (445); rest - | HOV Tk |
| D13 | stream.css:455 | `.splan-fill` | **width** | d-b e-p | plan progress | N | Te scaleX |
| D14 | stream.css:472,476 | `.splan-fold` colour 160ms e-s; chevron transform 200ms e-s | disclosure | 160/200ms | hover/expand | N | Te d-f/d-b ease-out |
| D15 | stream.css:503-504,513-518 | `.sxa-spin` 1s; `.sxplan-write` opacity 1<->.45 1.5s ∞; `.sxplan-caret` 1s `steps(2,start)` ∞ | live plan write | | exit-plan drafting | Y (518) for pulse/caret; spin N | H:SPIN; pulse/caret K as real-activity cursor |
| D16 | stream/ActivityHud.svelte:329-330,398,447,366,408 | `.ahud` ahud-in (opacity, y -10px, scale .96); border d-b; `.ahud-list` ahud-open (y -3px); hovers | HUD mount / expand | d-b · e-p | agents running | Y (455-458) | H:POP / ZOOM |
| D17 | ActivityHud.svelte:231 | `out:fade` 160 (JS-guarded); line 163 `scrollIntoView({behavior:"smooth"})` | fade; smooth scroll to agent card | 160ms; browser smooth | agents end; row click | J for fade; **scroll N** | Te fade d-f; guard scroll |
| D18 | stream/PlanHud.svelte:227-228,252,265,283,300 | `.phud` phud-in (same as ahud-in); border; `.phud-ring` phud-spin 0.9s; `.phud-fill` **width** d-b e-p + bg 200ms; `.phud-list` phud-open | plan HUD | d-b | plan active | Y (325-328) | H:POP; fill -> scaleX |
| D19 | PlanHud.svelte:173 | `out:fade` 160 | | 160ms | plan done | J | Tk |
| D20 | stream/StreamAgent.svelte:204,237,250,271,288 | `.sacard` blockIn; `.sa-dots span` sa-dot 1.1s ∞; `.sa-body` workOpen; rows sa-row-in 0.22s y 3px; `.sa-beat` sa-beat pulse-live opacity .35<->1 | agent card live | 220-300ms; loops 1.1s/1.6s | agent running | Y (321-326) | Tk; loops H:PING |
| D21 | stream/StreamShell.svelte:211,227,232,248,266 | `transition:slide` 140 (output body); `.sshell` blockIn; `.ssh-prompt` sshPromptPulse opacity 1<->.45 1.4s ∞; `.bh` bg; `.ssh-cursor` sshBlink 1.06s `steps(1)` ∞ | terminal block | | live shell | slide **N**; prompt/cursor Y (237-240); `.sshell` entrance **N** | H:ACC for slide; cursor K (real cursor) |
| D22 | stream/StreamResult.svelte:51 | `transition:slide` 140 (`.sr-out`) | height | 140ms | result open | **N** | H:ACC |
| D23 | stream/OutputBlock.svelte:119,127-142,206-215 | `.term-line` termLineIn (opacity y 2px) delay `min(i*16,400)ms` (JS `lineDelay`, :119); `.term-cursor` termCurBlink 1.06s steps ∞ | per-line reveal on mount | d-b e-p; 0-400ms | shell output lines appear | Y (208; JS also zeroes delay :119, :141) | K (stagger-capped, matches Halo cap rule) |
| D24 | OutputBlock.svelte:238,253 | `.oblock-foldbtn`, `.oblock-btn` | colour/bg | d-f | hover | - | HOV |
| D25 | OutputBlock.svelte:267-295 | `.output-scrim` oiScrimIn 160ms ease-out; `.output-inspector` oiPanelIn translateX 18px 180ms e-p | side inspector drawer | 160/180ms | open | Y (294-296) | H:DRAWER (40px, d-b; leave 200ms) |
| D26 | stream/BlockHeader.svelte:107-117 | `.bh-spin svg` bh-spin 1s; `.bh-copy` d-f | spinner; hover | 1s ∞ | block running | Y (117) | H:SPIN |
| D27 | stream/WriteBatch.svelte:89,104,123-131 | `transition:slide` 200 (diff wrap, J `reduced?0`); `.wb-chev` transform d-f; `.wb-diffwrap` wb-rise d-r e-p (opacity y 5px) | diff expand | 200ms; 460ms | expand | J + Y | H:ACC (slide + rise double-animate the same node: pick one) |
| D28 | stream/GrepResult.svelte:118,133 | `.gres-row`, `.gres-more` | bg/colour | d-f | hover | N (file has no media; colour only) | HOV |
| D29 | stream/StreamAskUser.svelte:324,342,377,388,390,422-424 | `.sask` blockIn; `.sask-dot` streamLivePulse; option border/bg d-f; `.sask-marker` colour d-f + dotPop 420ms spring; submit spinner sask-spin 1s | question card | | ask pending | spinner Y; rest **N** | H:PING, H:CHK (selected marker) |
| D30 | stream/StreamTurn.svelte:474 | `.sactivity-body` `transition:slide` 180 | settled-turn "receipt" expand | 180ms cubicOut | click receipt toggle | **N** (file has no media/JS guard) | H:ACC (see 5.4) |
| D31 | stream/StreamExitPlan.svelte:33-42 | setInterval 30ms, +20 chars/tick typewriter (`REVEAL_STEP 20`, `REVEAL_TICK_MS 30`) | text length state | ~666 chars/s | exit-plan card pending | J (:35 `reveal = -1` when reduced) | K (Rift-specific, see 5.3) |

### 1E. Legacy bubble renderer (`MessageBubble.svelte`; used when `uiPrefs.streamMode` is false)

| ID | path:line | what | moves | dur · ease · delay | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| E1 | MessageBubble.svelte:614,1407 | `.stop-notice-btn`, `.lightbox-close` | bg/colour | d-f | hover | - | HOV |
| E2 | :626 | `.bubble` enter (opacity y 4px) | **every bubble on mount** (incl. history hydration) | d-b e-p | list render | **N** | H:RISE with cap, or gate to new messages |
| E3 | :668,677-684 | `.turn-rail` opacity/shadow d-b; rail-stream box-shadow glow | live rail | 2.4s ease-in-out ∞ | streaming | Y (683) | X (stationary tint is enough; box-shadow loop) |
| E4 | :703-713 | `.avatar` avatar-halo box-shadow 0->3px | | 1.8s ∞ | streaming | Y | H:PING |
| E5 | :741,749,799,1181,1232,1244,1262 | `.head-sep`, `.head-model`, `.cost-pill`, `.copybtn`, `.head-time`, `.user-copy`, `.user-image-thumb` | opacity/colour/border/transform | d-f | hover | cost-pill Y (1158); rest - | HOV |
| E6 | :349 | `out:fade` 220 (stage strip) | | 220ms | stage ends | J | Tk |
| E7 | :763-783 | `.stage-strip` enter (**N**); `.stage-dot` stage-bounce (y -2px scale 1.1) | thinking dots | 1.1s ∞ | stage | dot Y; strip N | H:PING / X |
| E8 | :810-825 | `.tl-node` enter (Y via 1158); streaming `node-stream-enter` opacity y 3px **blur 4px** 480ms | node mount | | Y (824) | X per redesign-plan.md:204-206 (likely dead: StreamTurn owns live) - verify then delete |
| E9 | :853-868 | `.tl-node::before` bg/border/shadow d-b; bullet-pop 340ms **e-sp** scale 0->1.3->1 | bullet | | Y | Te halo-pop 1.18 |
| E10 | :877-912,1066 | `tl-bullet-pulse` box-shadow ring 0->5px (x3 selectors) | pending nodes | pulse-live ∞ | | Y (1160 + 1157-1160) | H:PING |
| E11 | :947,1075 | hover lift: transform d-f e-p + colours d-b | | | hover | - | HOV |
| E12 | :980,989-990 | `.work-head` bg; `.work.live .work-spark` nodePulse box-shadow 0->4px | | pulse-live e-s ∞ | live | **N** | H:PING |
| E13 | :408,488 | `transition:slide` 180 (`.tn-think-body`), 200 (`.work-body`), J | height | | disclosure | J | H:ACC |
| E14 | :1002-1006 | `.work-body` workOpen (transform only) 0.34s e-p | | | **N** | H:ACC (+ svelte slide on same node = double animation) |
| E15 | :1052-1056,1088 | `.tl-stepdot` bullet-pop; `.tl-divider` enter | | | Y | Te |
| E16 | :525,538,574 | `in:fade` 160 (stop notices), `transition:fade` 140 (lightbox) | | | J | Tk |
| E17 | :1314-1318 | `.dots .dot` pulse 1.1s ∞ (opacity .3<->1, scale .85) | thinking dots | | **N** | H:PING/SPIN-dots |
| E18 | :1334-1344 | `.live-dot` live-pulse opacity .55<->1 + box-shadow ring | streaming dot | 1.4s ∞ | **N** | H:PING |
| E19 | :1369-1374 | `:global(.tid-flash)` tid-flash box-shadow glow flash | jump-to-tool highlight | 1.4s e-s | Y | K |

### 1F. Markdown, tool chips, diffs, permission

| ID | path:line | what | moves | dur · ease · delay | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| F1 | assistant/Markdown.svelte:684-699 | `.md-w` md-word per-word reveal | opacity 0->1 + **filter blur 3px->0** | 500ms e-p · negative delay set once per span (:307) | streaming prose; spans released after `REVEAL_WINDOW`=18 words | Y (696-699; JS bypass :198,:217,:622) | **D2** - see 5.1 + 6.2 |
| F2 | Markdown.svelte:689-691 | `.md-w-hold` | `opacity:0` (no filter - already fixed per plan item 1) | - | held words | Y | K |
| F3 | Markdown.svelte:708-727 | "chrome-hold" `opacity` d-b `ease` on code/kbd/mark/h1-6/li/blockquote/table/tr/hr/pre/shiki/img, driven by `:has(.md-w-hold)` | | d-b | during stream; NP | NP | K (but `:has()` selectors re-evaluate per class flip - perf watch 6.5) |
| F4 | Markdown.svelte:777,1088-1095 | `pre` / `.shiki-block` code-rise (opacity y 6px scale .992) | code block mount | d-r 460 e-p · both | block appears | Y for `.shiki-block` (1094); **N for `pre` :777** | H:RISE |
| F5 | Markdown.svelte:809,968,1132,1233,1360,1380,1469 | `.code-copy` opacity d-f + colour/bg **120ms**; link underline d-f + opacity **220ms**; `.shiki-head .code-copy`; `.cm-pill` (reduce Y 1247); `details` border; `summary::before` transform; `code.md-path` text-shadow | hover | d-f / 120 / 220 | hover | partly | HOV Tk (kill 120/220 literals) |
| F6 | Markdown.svelte:1212 | `.cm-fade` `backdrop-filter: blur(1.5px)` | | static | collapsed code | - | X (gradient+mask already does the job; 6.1) |
| F7 | assistant/ToolChip.svelte:535-548 | `.tile` enter (opacity y 4px) | chip mount | d-b e-p | **N** | H:RISE |
| F8 | ToolChip.svelte:626,704 | `chip-spin` 1s ∞ (also AgentCard:157-158, AskUserCard:437,501-502, TodoCard:118-122) | spinners x4 defs | 1-1.1s linear ∞ | pending | **N** (spinner; Halo override would stop it) | H:SPIN (dedupe 4 keyframes) |
| F9 | ToolChip.svelte:479 | `transition:slide` 180 | chip body | 180ms | J | H:ACC |
| F10 | ToolChip.svelte:711-724 | `.body-frame` tool-rise (opacity y 5px) d-r e-p | | Y | | H:RISE (double with slide on same subtree, F9) |
| F11 | ToolChip.svelte:730 | `.terminal-cmd` backdrop-filter blur 8px saturate 120% | | static | | - | X (6.1) |
| F12 | assistant/EditDiff.svelte:406,484,501,511,571-579,677 | slide 200 (J); crumb/copy/more hovers d-f; `.diff-line/.diff-meta/.diff-gap` diff-row-in (translateX -4px, d-b e-p) | diff expand | | J + Y | H:ACC; rows: cap stagger (none today) |
| F13 | assistant/PermissionBar.svelte:79,118,135-145 | `perm-enter` (y -3px d-b); `.perm-btn` colours d-f; `perm-spin` 1s | permission ask | | Y for enter/btn; **spinner intentionally kept in reduce** (comment :141-142) | H:POP-down; spinner K (state indicator) |
| F14 | toolchip/AgentCard.svelte:124-133 | `.agent-pending .dot` agent-dot 1.1s ∞ | | | Y | H:PING |
| F15 | toolchip/AskUserCard.svelte:346,398,420 | option bg/border d-f + transform **80ms**; input border/shadow; btn | hover | | - | HOV |
| F16 | toolchip/TodoCard.svelte:92,98,102 | `.todo-item/-box/-content` colour/opacity/text-decoration-color **d-b ease-out** | todo done strike | d-b | status flip | - | H:CHK (draw the check) |
| F17 | bubble/BoundaryBlock.svelte:113-129 | `.boundary-pill` bg/border/transform d-f + chevron; backdrop-filter blur 8px | | | **N** | HOV; X blur |
| F18 | bubble/ModelSwitchBlock.svelte:63 | backdrop-filter blur 8px | static | | | X |
| F19 | bubble/TurnSummary.svelte:64 | `in:fade` 160 | | 160ms | J | Tk |
| F20 | assistant/GhPopover.svelte:257-283 | `.gp-spin` gp-rotate 0.9s; `.gp-dot.busy` gp-breathe pulse-live | | | Y | H:SPIN / PING |

### 1G. Composer family

| ID | path:line | what | moves | dur · ease · delay | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| G1 | assistant/Composer.svelte:1875-1904 | `.drop-overlay` drop-in (scale .94); `.drop-dot` drop-dot-pulse scale 1<->1.3; pill `backdrop-filter: blur(12px)` (:1886) | file drag over | d-f · e-p; 1.2s ∞ | **N** | H:POP; X blur |
| G2 | Composer.svelte:1912-1930 | `.composer-box` `backdrop-filter: blur(14px) saturate(135%)` (:1921); border+shadow d-b **cubic literal**, transform d-f | focus | | **N** | Tk; X blur (6.1: always-on, full-width, over scrolling stream) |
| G3 | Composer.svelte:1993-2014 | `.placeholder-ghost` placeholder-fade (opacity, y 8px, **blur 3px**) | rotating placeholder | 700ms e-p | Y | K, consider opacity-only |
| G4 | Composer.svelte:2032-2045 | `.composer-bar` enter | opacity y 4px | d-b e-p | Y | H:RISE |
| G5 | Composer.svelte:2080,2092,2482,2526,2538,2559,2573 | `.cbtn` (+transform), `.cbtn-chev`, `.model-pill`, `.model-dot`, `.pill-chev`, `.ps-chip`, `.ps-dismiss` | hover | d-f ease-out | - | HOV |
| G6 | Composer.svelte:2130-2192 | `.mic-spin` 0.9s; `.mic-wave span` height 80ms (JS meter-driven, see J6); `.mic-wave.silent` mic-idle height 22<->38% pulse-live ∞; `.mic-ring rect` stroke-dashoffset 260ms linear | dictation | | Y (2188-2193) | height anim on bars: Te transform:scaleY |
| G7 | Composer.svelte:2209,2462-2464 | `.char-count` enter d-f (**N**); `.cbtn.reveal` enter (Y) | | | | H:RISE |
| G8 | Composer.svelte:2228,2261-2272,2285 | `.send-btn` colour/bg/border/shadow/transform d-f; `.send-ripple` scale .6->2.2 + fade 620ms e-p; `.icon-slot` opacity d-f + transform d-b e-p | send | | ripple Y | HOV; ripple K (one-shot, press feedback; Halo press = 0.97 scale - D12) |
| G9 | Composer.svelte:2333-2375,2454-2457 | `.magic-text` magic-shimmer 1.15s linear ∞ (background-position); `.magic-aura` opacity pulse pulse-live; `.magic-stars i` twinkle pulse-live (scale+rotate) | enhance-prompt "magic" | | Y | **D6** |
| G10 | Composer.svelte:2383-2390 | `.wandbtn.enhancing` wand-pulse **box-shadow + scale 1.08** 1.3s | | | Y | H:PING |
| G11 | Composer.svelte:2419-2450 | `.dg-tail.pending`/`.textarea-wrap.polishing textarea` dictate-polish opacity 1<->.55 1.2s ∞; `.dg-tail::after` dict-caret 1s `steps(2,jump-none)` ∞ | dictation tail | | Y | K (real activity) |
| G12 | Composer.svelte:2533-2534 | `.model-pill.provider-checking .model-dot` provider-pulse opacity .35 1.2s ∞ | provider check | | **N** | H:PING |
| G13 | Composer.svelte:2591-2601 | `.pill-ultra` ultra-pulse opacity 1<->.62 2.6s ∞ | "ultra" mode badge (decorative) | | Y | X (**D11**) |
| G14 | composer/CtxRing.svelte:72,80 | `.ctxring` bg d-b; `.fill` stroke-dashoffset **d-s e-s** + stroke d-b | context % ring | | **N** | K (Halo ring/progress animate dashoffset: `bundle.css:154`) Tk |
| G15 | composer/AttachmentsRow.svelte:86 | `.attach-chip` border/bg d-f | | | N (hover only) | HOV |
| G16 | composer/EnhanceBar.svelte:196-215 | `.enhance-panel` slash-in (opacity y 4px) d-f e-p; `backdrop-filter: blur(14px) saturate(135%)` (:203) | | | **N** | H:POP-up; X blur |
| G17 | EnhanceBar.svelte:243,271,296,341 | buttons/toggles/steer input | hover; transform 120ms | | - | HOV |
| G18 | EnhanceBar.svelte:305-310 | `.enhance-title.status` status-pulse opacity 1.4s ∞ | | | **N** | H:PING |
| G19 | EnhanceBar.svelte:386-401 | `.ew` word-materialize opacity + **blur 7px** 420ms e-p, per word | enhanced text reveal | | Y | D2 family (6.2) |
| G20 | composer/MentionPopover.svelte:54-70 | `.slash-menu` slash-in d-f; `.mention-item` slash-item-in (y 6px) **d-f** all rows together (comment :60-61: "rows enter together") | | | Y | H:POP; MENUITEM stagger intentionally off - D13 |
| G21 | composer/SlashMenu.svelte:134-155,240-248,358,405-416,451-452 | menu `backdrop-filter: blur(18px) saturate(1.25)` (:143, then `backdrop-filter:none` :354 in a variant); `.slash-menu` slash-in (y 6px scale .985) d-f; `.sm-item` bg **90ms/80ms** + slash-item-in d-b (per-row, delay unknown - not read); `.sm-bar` height d-f / 80ms; `.sm-icon` | | | Y | H:POP + MENUITEM (`min(i,12)*18ms`); X blur |
| G22 | composer/SettingsMenu.svelte:510-526 | `.settings-menu` **`position:fixed`**, `backdrop-filter: blur(20px) saturate(1.4)` (:519-520), pop-in d-b e-p, origin bottom-right | | | Y (962-969) | H:POP; **X blur (6.1)** |
| G23 | SettingsMenu.svelte:551,578,589-594,606,691,777,783,839,845,891,893,905,932,937,957 | tabs, more-chev 0.24s, legacy-flyout flyout-open (**max-height 0->200px**, y -4px, d-r), pop-item, fast-switch bg + knob transform d-b e-p, effort-rail shadow, `.er-track::after` **width** d-b **e-sp**, `.er-stop i`, `.er-knob` **left** d-b e-sp, caption colour | settings popover controls | | Y (962-969) | `left`/`width`/`max-height` -> transform/grid-rows; spring on a grabbed knob is correct per Halo |
| G24 | SettingsMenu.svelte:729,737-742 | `provider-spin` 0.9s; `.pop-ck` ck-in (scale .4 rotate -12deg) 0.28s e-p | check appears | | Y | H:CHK |
| G25 | composer/UsagePanel.svelte:280-289,308-313,338,347-351 | `.usage-pop` usage-in d-f; `.up-fill` **width** d-s e-p + up-grow `from{width:0}` 480ms; hot: + `up-pulse` filter brightness 1.3 2.2s ∞ (delay 600ms); `.up-compact`; `up-spin` 900ms | usage bars | | Y (349-352) | fill -> scaleX (Halo Progress); H:POP; pulse X (filter loop) |
| G26 | composer/QueueRail.svelte:99,115,125-126 | `fly y:7 170 quintOut`; `slide` 160; items `in:fly y:5 150 delay 20*i quintOut` / `out:fly y:5 120` | queue rail | | **N** | H:RISE + ACC (stagger cap) |
| G27 | QueueRail.svelte:174,193 | chevron transform d-f; `.qactions` opacity d-f | hover | | - | HOV |
| G28 | composer/PermMenu.svelte:87 | rAF (focus) - no visual motion | - | | | - | n/a |

### 1H. Overlays, dialogs, primitives

| ID | path:line | what | moves | dur · ease · delay | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| H1 | ui/recipes.ts:20 (popover content) | tw-animate: `duration d-f`, `ease-page`, `fade-in-0 zoom-in-95`, `slide-in-from-{top,bottom,left,right}-2` per side; closed: `fade-out-0 zoom-out-95` | dropdown/select/context menu | d-f · e-p | open/close | G | H:POP (+ per-side +/-4px); matches already |
| H2 | ui/recipes.ts:27,53 | (long lines; contain `transition-colors` for items - body not read in full) | item hover | d-f | | G | HOV |
| H3 | ui/recipes.ts:58 | overlay `duration d-f fade-in-0/out-0` | scrim | d-f | open | G | ZOOM overlay |
| H4 | ui/recipes.ts:60 | alert-dialog content `duration d-f ease-page fade zoom-95` | confirm dialog | d-f | open | G | H:ZOOM (d-b, scale .94, y 8px; leave 200ms) |
| H5 | ui/button/button.svelte:7 | `transition ease-soft duration d-f`; `active:translate-y-px` | press | d-f | | - | H:HOV press scale .97 |
| H6 | ui/select/select-trigger.svelte:31 | chevron `transition-transform duration d-f ease-soft in-data-[state=open]:rotate-180` | | d-f | | - | Tk |
| H7 | dialogs/CommandPalette.svelte:311,318 | scrim `fade` 130; panel `scale` start .97, 170ms, `quintOut` | palette | 130/170ms | open/close | **N** | H:ZOOM (palette = `halo-zoom`, `bundle.css:292`) |
| H8 | CommandPalette.svelte:465,481 | `.cp-item` bg **80ms**; icon colour/bg d-f | hover/selection | | N | HOV; item stagger MENUITEM |
| H9 | dialogs/McpServersDialog.svelte:133,141 | `fade` 130 / `scale` .97 170 quintOut | dialog | | **N** | H:ZOOM |
| H10 | McpServersDialog.svelte:321,326,369,439,514 | hovers; `mcp-spin` 900ms; `.mcp-skel` mcp-shimmer 1.1s ease-in-out ∞ with inline `animation-delay: i*120ms` (:213) | | | **N** (no media at all) | H:SPIN; H:SKEL **D1** |
| H11 | dialogs/UpdateDialog.svelte:95,103 | `fade` 120 / `fly y:8` 180 | | | partial (556-561: spinner+hero-fill only; svelte transitions unguarded) | H:ZOOM |
| H12 | UpdateDialog.svelte:328,430,531,556 | hovers; `.hero-fill` **width** d-f; `.spin` 1s | download | | Y | Te scaleX |
| H13 | diagnostics/DiagnosticsConsole.svelte:163,317 | panel `scale` 0.98 180; backdrop `fade` 140 | | | **N** for svelte transitions | H:ZOOM |
| H14 | DiagnosticsConsole.svelte:375,394,405-407,420-430,454,490,499-503 | hovers with **fallback copies** `var(--dur-fast,.14s)` (x11 at 375,394,420,425,430,490 / `.12s` 454); `dc-breathe` scale 1->1.5 + opacity (pulse-live ∞); `dc-blink` opacity (pulse-live) | | | Y (407,501) | Tk (drop fallbacks); H:PING |

### 1I. Pages (Settings, Workspace hub, News, AI Health, Web, Onboarding, LimitBar)

| ID | path:line | what | moves | dur · ease · delay | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| I1 | settings/SettingsPage.svelte:1596 (`.tab-ink`) | sliding underline | transform + **width** | d-b e-p | tab change | Y (1608) | H:THUMB (Halo = transform+width d-b, same idea) K, Tk |
| I2 | SettingsPage.svelte:1597,1611,1657,1689,1781,1783,1826,1835,1841,1850,1895,1908,1942,1945,1972 | `.snav` colour; svg colour; provider-switch; `.card` border; `.set-expand` (+icon rotate d-f e-p); `.seg button`; `.sw` transform; range thumbs transform d-f e-p (slider thumbs: spring in Halo); summary chevron; cli-copy; powered; `.set-pick` | hover/press | d-f | - | - | HOV Tk; thumbs H:spring |
| I3 | SettingsPage.svelte:1602-1609 | `.st-pill.ok .dot` pill-ping box-shadow 0->7px, **once** | | 900ms e-p x1 | status ok | Y | H:PING-once (halo-bell__ping 900ms) |
| I4 | SettingsPage.svelte:1617,1642-1647 | `.sset-search` width 180ms e-p (expands on focus); `.surface-body{scroll-behavior:smooth}` | | | Y (1643-1647) | width -> Te |
| I5 | SettingsPage.svelte:1635-1645 | `.card.sflash` sflash box-shadow glow (search-jump highlight) | | 1.6s e-p | Y | K |
| I6 | SettingsPage.svelte:1590,1676,1684 | `.set-col > .card` blockIn 8px (local keyframe **overrides stream.css's 3px** - same name) | cards enter | d-b e-p | Y | H:RISE (cap stagger) |
| I7 | SettingsPage.svelte:1733,1762,1987 | `.plan-gauge-fill` **width** 420ms e-p; `.iface-row` **height** d-b; `.set-progress-fill` **width** d-f linear | | | N | Te scaleX / ACC |
| I8 | SettingsPage.svelte:1792-1797 | `.anim-reveal` reveal-in (y 7px scale .985) d-b e-p, delay `min(--ti,16)*22ms` | search results/tab content stagger | | Y | H:RISE (cap 12) |
| I9 | SettingsPage.svelte:1624 | `.sset-results` backdrop-filter blur 14px | `position:absolute` dropdown | static | | X (6.1) |
| I10 | settings JS: 191-198,225,229,269 | `scrollIntoView({behavior: reduceMotion?"auto":"smooth"})`, rAF double for jump-flash | J | | | K (a correct guard example) |
| I11 | workspace/WorkspacePage.svelte:842,931,970,980,982,1006,1014,1026,1035,1044,1052,1054,1067,1070,1138,1156,1175,1184,1190,1200 | hover/colour/opacity/transform across hub (range tabs, chip-btn, mini-new, ico-btn, input, browse, recent-row/forget, adv-toggle/chev, del/ghost, pcard-act/go, save-current, news-strip, nsh-refresh/chev) | | d-f | | - (only skel/spin/editor/live-dot have media) | HOV Tk |
| I12 | WorkspacePage.svelte:893,1074,1111,1168 | `.act-retry` transform 80ms; `.save-btn` `filter: brightness` d-f + transform; `.pcard` border/shadow/**transform**/bg d-f; `.save-current` | hover lift/press | | N | HOV (3px lift = Halo); brightness X |
| I13 | WorkspacePage.svelte:920-921,947-948 | `.ch-col` `filter:brightness(1.18)` + `scaleY(1.03)` hover; `.mseg` filter brightness hover | chart bars | d-f | N | H:BarChart `is-dim` siblings (opacity .45, `bundle.css:210-211`) instead of filter |
| I14 | WorkspacePage.svelte:877-880 | `.skel` skel-shimmer background-position 1.5s ease-in-out ∞ | | | Y | H:SKEL **D1** |
| I15 | WorkspacePage.svelte:958-960,1004,1193-1195 | spinners wsActSpin x2 (0.9s), nspin | | | Y | H:SPIN |
| I16 | WorkspacePage.svelte:990-992 | `.editor` edIn (opacity y -5px) d-b e-p | | | Y | H:ACC/POP |
| I17 | WorkspacePage.svelte:1090-1092 | `.live-dot` livePulse box-shadow ring 5px 2.2s ease-out ∞ | | | Y | H:PING |
| I18 | workspace/NewsFeed.svelte:226,239,266,271,280,294,300,310,312,318,320 | hover/lift/chevron rotates (d-f; 239 transform + e-p) | | | N | HOV |
| I19 | NewsFeed.svelte:255-260,331-336 | `.dl-orb` orb-breathe scale 1<->1.08 2.4s ∞ (NP); `.dl-fill` dl-slide translateX 1.3s ease-in-out ∞; `.news .spin` | digest loading | | Y | H:INDET (1.3s) for dl-fill; orb X |
| I20 | NewsFeed.svelte:72 | setInterval 2600ms rotates `stepIdx` (digest step label) - visual text swap | state | | | - (no motion on swap) | add verbIn-style crossfade |
| I21 | ai-health/AiHealthPage.svelte:1253-1254 | `.ah-wrap > *` enter 360ms e-p, **NP-gated** | page entrance | | NP | H:PAGE/RISE |
| I22 | AiHealthPage.svelte:1282-1295 | `.ah-verdict-strip` ah-vs-in 0.42s (y -4px); hot `.ah-vs-dot` ah-vs-pulse **box-shadow** 2.4s e-p ∞ | | | Y | H:PING |
| I23 | AiHealthPage.svelte:1316-1323,1344,1348,1369,1394,1464,1484-1488 | `ah-spin` 0.9s; `ah-step-in` 0.4s (x -5px); hovers; `.ah-sc-barwrap` **height** d-b e-p; `.ah-fill` **width** 0.3s ease; `.ah-skel` ah-skel-pulse opacity .5<->1 1.4s e-s ∞ | | | Y except 1394/1464 | Te scaleX/height; H:SKEL **D1** |
| I24 | webview/WebBrowserPage.svelte:706-708,743-750,768-777 | `wb-spin` 0.9s; `.wb-progress::before` wb-sweep translateX 1.1s ease-in-out ∞ (reduce -> static 50% bar); `.wb-airead` wb-shimmer 1.5s linear ∞ + wb-airead-in d-f | page load | | Y | H:INDET; shimmer **D1** |
| I25 | WebBrowserPage.svelte:789,826,961 | hovers d-f ease | | | N | HOV |
| I26 | shared/LimitBar.svelte:38,40,45,48-49,53,62-64,68,76-80 | fill **width** d-s e-p + limitbar-grow `from{width:0}` 480ms; hot: limitbar-pulse `filter: brightness(1.3)` 2.2s ∞ (delay 600ms) + `::after` limitbar-sheen translateX 2.6s e-s ∞ (delay 900ms; **traveling light band**); card fill width 0.3s ease; compact width d-s e-s | usage meters | | Y for animations (`!important`), compact transition (80); panel/card width transitions **N** | Te scaleX; pulse/sheen X (DESIGN.md:227-251 bans traveling bands) |
| I27 | styles/onboarding.css:29-32,59,64,74,92,110-115,138,158,168,185,218,245,278,288,300 | `.ob-card` ob-card-in 360ms (opacity y 10px scale .99; Y); spine `height` d-s e-s; step/node/skip/btn/fdot(**width** d-b)/vrow/accent-dot/seg/preset/copy/input/recent transitions d-f/d-b `ease`; `.ob-step-wrap.ob-enter > *` ob-rise 440ms e-p (NP, **no stagger set**) | onboarding | | partly | HOV Tk; H:RISE with stagger |
| I28 | onboarding.css:323-326,361-362 | `.ob-connbar .dot` ob-live (box-shadow ring) pulse-live ∞ (NP); `:where(.ob-overlay) .spin` spin 1s (**unguarded**, keyframe unguarded) | | | partly | H:PING / H:SPIN |
| I29 | onboarding.css:480-528 | mini-demo loops (NP): `.ob-mini-chat` 7s, `.ob-mc-user/-chip c1 c2/-reply/-l1 l2 l3` 7s, `.ob-mc-chip i` 1.4s; `.ob-mini-cli` 5.5s (`ob-mcl-type` animates `max-width`, `steps(6,end)`; caret `ob-blink` 1.1s steps); `.ob-mini-tree` 6.5s (rows `translateX`; `.scan` animates `top`); `.ob-md-dial .d1-d4` 5s, delays 0/.55/1.1/1.65s (`box-shadow`) | 22 simultaneous infinite animations across onboarding demos | | NP | **D4** |
| I30 | onboarding/ClaudeConnect.svelte:71-74 | setInterval poll (not visual) | | | | n/a |
| I31 | routes/dev/gallery/+page.svelte:46,162 | rAF scroll; `scroll-behavior: smooth` | dev only | | N | n/a (dev route) |

### 1J. Programmatic / JS-driven motion (rAF, timers, FLIP, custom Svelte transitions)

| ID | path:line | what | moves | dur · ease | trigger · loop | RM | verdict |
|---|---|---|---|---|---|---|---|
| J1 | state/assistant/streaming.ts:393-446 (`enqueueText`/`drainTick`) | per-tab rAF char pacer: `rate = max(360, pending/0.1)` c/s, `dt<=100ms`; flush immediately if `document.hidden` (:401-404) | text length in model | per-frame | every stream delta | **N** (pacer is invisible-ish but adds ~<=100ms latency) | K; Halo-agnostic (5.1) |
| J2 | Markdown.svelte:196-202,216-252 | rAF coalesced parse; rAF word-reveal cursor `WORD_MS=30`, `CATCHUP_LAG=ceil(320/30)=11`, `REVEAL_WINDOW=ceil(500/30)+1=18`; parks when caught up | `shownCount` -> class flips | 30ms/word | streaming | J (`reduceMotion` returns early :198,:217) | D2 / 5.1 |
| J3 | Markdown.svelte:260-314 | `wrapWords` + `applyReveal` (negative `animationDelay`, set once) | DOM spans | | | J | K |
| J4 | AssistantPane.svelte:169-191 (`pinToBottom`) | rAF lerp `scrollTop += delta*0.35` **per frame**, snap if delta>900 | scroll | frame-rate dependent | stuck-to-bottom + content growth | J (:167,171) | 5.2 |
| J5 | AssistantPane.svelte:359-384 (`runComposerFlip`) | inline `transition: transform 460ms cubic-bezier(0.22,1,0.36,1)`; clears on `transitionend` + 540ms timer | composer translateY FLIP between hero and docked | 460ms e-p | focus/blur/send on empty pane | J (:364) | H:FLIP-ish; K, Te ease-out |
| J6 | AssistantPane.svelte:332-357 | custom `welcomeOut` (fixed-pos pin, opacity+translateY(-10px*(1-t)), 200ms) / `welcomeIn` (220ms) | welcome block | 200/220ms linear t | engage/disengage composer | J | K / Te |
| J7 | AnimatedCount.svelte:35-76 | rAF tween `easeOut = 1-(1-t)^3`, `durationMs 420`; roll key bump | live token/diff counters | 420ms | value change while `live` | J (:23,:51) | H:COUNT (Halo 1100ms) / ROLL |
| J8 | SplashOverlay.svelte:47-63 | rAF progress smoothing (`dt/240` pull, creep `0.00005*dt`, cap target+0.08 <= 0.96) | `shownP` -> `scaleX` | | boot readout | J (:50) | K |
| J9 | stt.svelte.ts:733-752 | rAF audio RMS -> waveform level push | mic bars (`height` 80ms transition at Composer:2149) | per-frame | dictating | CSS Y | Te transform |
| J10 | SettingsMenu.svelte:74-81 | **permanent rAF loop** calling `position()` every frame while the popover is open (to follow composer FLIP) | layout reads | per-frame | menu open | - | perf 6.5 |
| J11 | AppShell.svelte:155-169 | rAF-coalesced resize -> `shell.syncToViewport` | rail auto-collapse | | | - | n/a |
| J12 | diagnostics.svelte.ts:158,208 | rAF flush of log buffer | not visual | | | - | n/a |
| J13 | AssistantPane.svelte:145,255-257 | `scrollTo({behavior:"smooth"})` for exchange-jump and "jump to latest" | scroll | UA default | click | **N** | guard + Te |
| J14 | many | `setInterval` 1000ms clocks (ConversationList:171, ActivityHud:119, StreamAgent:71, StreamShell:50, StreamTurn:191), 500ms (MessageBubble:74), 10s/30s/60s/300s elsewhere - **drive text only**; ConversationList:171 re-renders list each second | state | | | - | 6.5 |
| J15 | SplashOverlay.svelte:78-80 | `setTimeout` 80ms handoff, 680ms/120ms exit | choreography | | | J | K |
| J16 | state/intro.svelte.ts:10-23 | phase veil -> handoff -> settled (700ms timer) | choreography | | cold boot | J (:11) | K |
| J17 | WorkspaceShell.svelte:57-73 | settle timer 810ms for `.rising` | | | | - | K (becomes obsolete if RISE moves to CSS-only) |
| J18 | actions/sliderBubble.ts:42-43 | double-rAF to run entrance transition | | | | - | K |
| J19 | svelte transition directive census | `transition:slide` x10, `fade` x7, `scale` x3, `fly` x3, `in/out:fade` x6, `in/out:fly` x4, `animate:flip` x1, custom `dockSlide`, `welcomeIn/Out`; **no** `crossfade`, `tweened`, `spring`, `Tween`, `Spring`, `svelte/motion`, `startViewTransition` anywhere | | | | | 4/§5 |

---

## 2. Duration and easing census

Method: `grep -oE` over every `transition|animation` declaration in `src/**/*.{svelte,css}` (first line of each declaration; multi-line `transition:` lists count once, so a few tokens are under-counted). Svelte transition directive durations are listed separately at the end of 2.1. Counts include `animation-delay` / `transition-delay` literals where they sit on the same line.

### 2.1 Durations

| Cluster | Values (count) | Where it lives | Halo equivalent |
|---|---|---|---|
| **Token: fast** | `var(--dur-fast)` x453, + `var(--dur-fast,.14s)` x11 and `,.12s` x1 (Diagnostics fallback copies) | hover/press/colour everywhere | `dur-fast` 150ms |
| **Token: base** | `var(--dur-base)` x117 | state flips, blockIn, pop-ins, menus | `dur-base` 300ms |
| **Token: rise** | `var(--dur-rise)` x8 | code-rise, tool-rise, wb-rise, flyout-open, ws-child-rise, intro handoff | none (-> `dur-slow`) |
| **Token: slow** | `var(--dur-slow)` x5 | LimitBar/UsagePanel fills, CtxRing, onboarding spine | `dur-slow` 600ms |
| **Token: page** | `--dur-page` x2, `--dur-page-out` x2 (one with a never-applied 240ms fallback) | WorkspaceShell, `.workspace`, `--rail-w` | PAGE 520ms |
| **Token: pulse** | `var(--pulse-live)` x16 + `,1.6s` x3 | live dots, breathing | LiveDot 1.8s |
| **Token: stagger** | `var(--stagger)` x5 | WorkspaceShell `.rising` | `stagger` 60ms |
| Literal <=100ms | 80ms x13, 90ms x3, 70ms x2, 100ms x2 | button press (`app.css:403`), input focus (`:431`), slider bubble (`:390`), CommandPalette item (`:465`), SlashMenu item/bar (`:240,406,416`), mic bars (`Composer:2149`), AskUserCard option press (`:346`) | none (press = d-f) |
| Literal 120-200ms | 120ms x4 (EnhanceBar press, Markdown copy), 140ms x3, 150ms x3, 160ms x2, 170, 180 x2, 185, 190, 200ms x6 | slides, fades, rows, splash fades | `dur-fast` / 200ms leave |
| Literal 220-300ms | 220 x2, 0.22s x3, 240ms x3, 0.24s x4, 260 x2, 0.27, 0.28s x2, 270, 0.3s x8 | rowIn, workOpen, swap/pop-ins, ConversationList chevron | `dur-base` |
| Literal 320-480ms | 320 x3, 340 x2, 360 x5, 380, 400, 420ms/0.42s x8, 440, 460, 480ms x5 | pane-empty-in, bullet-pop, ob-rise, dotPop, word-materialize, up-grow/limitbar-grow, node-stream-enter | between base and slow |
| Literal 500-700ms | 0.5s, 520 x1, 0.55, 560, 600 x2, 620, 650, 700 | md-word 0.5s, splash panels 520, send-ripple 620 | `dur-slow` |
| Literal 900ms-1.1s | 900ms x5 (incl. `scompact-fill` 900, `pill-ping` 900), 0.9s x11 (spinners), 1s x16, 1.05/1.06/1.1s x12 | spinners, cursor blink, pulses | spin 900ms |
| Literal loops 1.2-4.2s | 1.2s x4, 1.3s x2, 1.4s x7, 1.5s x4, 1.6s x5, 1.8s, 2s, 2.2s x3, 2.4s x4, 2.6s x3, 3.4s, 4.2s | pulses/breathes/shimmers | LiveDot 1.8s; shimmer 1.4s |
| Literal long loops | 5s x4, 5.5s x4, 6.5s x6, 7s x8 | onboarding mini-demos only (`onboarding.css:482-507`) | none |
| Delays | 14ms (conv rows), 22ms (settings reveal), 35ms (`.enter`), 16ms (term lines), 55ms (splash letters), `20*i` ms (QueueRail), `120*i` ms (McpServers skeleton), 0.55s steps (ob-dial), 140/200/300/600/900ms one-offs | | stagger 60 (cap 12) |

Distinct raw literal values (durations + delays): ~90. Halo ships 3 durations + 7 named exceptions (520 page, 360 skel-out, 220 menu-item, 240 toast-leave, 200 leave, 460 flip, 900/1100 roll/count) + 4 loop periods (900 spin, 1.3 indet, 1.4 shimmer, 1.8 ping).

Svelte transition directives (JS, not in the grep above): `fade` 120/130/140 (x6), `fly` 150-220 (y 5-10), `scale` start .97/.98 170/180, `slide` 140/160/180/200 (x10), `flip` 180, custom `dockSlide` 220, `welcomeOut` 200, `welcomeIn` 220. Easing is Svelte's default `cubicOut`, or `quintOut` at CommandPalette:318, McpServersDialog:141, QueueRail:99,125,126. None of them uses a Rift token.

### 2.2 Easings

| Family | Rift expression (count) | Notes |
|---|---|---|
| Decelerate "page" | `var(--ease-page)` x98 + literal `cubic-bezier(0.22,1,0.36,1)` x35 (same curve, un-tokenised) + `quintOut` x5 | largest family (~138). Stronger decel than Halo `ease-out` `(0.2,0.8,0.2,1)` -> landings are snappier today |
| Keyword `ease-out` | x130 | CSS default `(0,0,0.58,1)`; not in Halo |
| Keyword `ease` | x104 (mostly `transition: ... var(--dur-fast) ease`) | CSS default `(0.25,0.1,0.25,1)`; accidental, not a token |
| Standard "soft" | `var(--ease-soft)` x36 + literal `(0.4,0,0.2,1)` in SplashOverlay x2 + `var(--ease-soft,ease)` x1 | layout shifts; sidebar, tiles, scompact |
| `ease-in-out` | x46 (almost all infinite pulses/breathes) | |
| `linear` | x36 | spinners, shimmers, progress bars |
| Spring | `var(--ease-spring)` x7 + literal `(0.34,1.56,0.64,1)` x2 + one `var(--ease-spring,var(--ease-page))` | **identical curve to Halo `ease-spring`**; uses: dotPop (overshoot to 1.55), headRise, bullet-pop, `.ri-go` hover arrow, effort knob/track |
| Step | `steps(1)` x4, `steps(2,start)`, `steps(2,jump-none)`, `steps(6,end)` | cursor blinks, ob typing demo |
| One-offs | `(0.65,0,0.35,1)` (splash panels only) | |

So: 4 named tokens + 5 CSS keywords + 3 raw bezier curves + 5 step forms. Halo: 1 curve + 1 spring.

---

## 3. Halo motion doctrine (exact values)

Sources: `halo:03-motion.md`, `halo:04-performance.md`, `halo:README.md:10`, `halo:tokens.json:532-570`, `halo:components/bundle.css` (line numbers in col 3), component READMEs.

### 3.1 Rules

- **Three speeds, one curve.** `dur-fast` 150ms (hover/press: colour, border, 3px lift, 0.97 press, spotlight fade) · `dur-base` 300ms (state change: sliding thumbs/underlines, accordion height, focus dim, dialogs/popups scaling from their corner) · `dur-slow` 600ms (entrance: cards rise 14px + fade, lines draw, numbers count, Stepper fills) · `stagger` 60ms per item, capped ~12 so long lists never wait. (`03-motion.md:3-7`, `tokens.json:532-555`)
- **Curves.** `ease-out` = `cubic-bezier(0.2,0.8,0.2,1)` is the main curve. `ease-spring` = `cubic-bezier(0.34,1.56,0.64,1)` only for things you flip or grab: toggle knob, checkbox/radio pop, slider thumb, tag chips, chosen calendar day, drop-zone icon. (`03-motion.md:16`, `tokens.json:557-569`)
- **Loops only for real activity**: `LiveDot`, `Spinner`, indeterminate `Progress` (+ a game's own targets). "Things enter once, in a stagger, and never loop for attention. Only real-time state pulses." (`03-motion.md:17`, `README.md:10`)
- **Only `opacity` and `transform` run continuously** (shimmer, ping, spin, indeterminate bar). **Blur is reserved for small areas**: a skeleton fading into content, the countdown veil. Page change has **no blur** (a full-area blur costs frames). (`03-motion.md:11,19`, `04-performance.md`)
- **Reduced motion**: counts and rolls land instantly, lines appear drawn, glides and pulses stop. Implemented as a blanket override: `*,*::before,*::after{animation-duration:0.01ms!important;animation-iteration-count:1!important;transition-duration:0.01ms!important;scroll-behavior:auto!important}` (`bundle.css:901-902`).
- Performance habits: stagger cap ~12; hover effects update at most once a frame; lists re-measure for glide only when order/data changes; `Card spotlight` on a few hero cards only (`04-performance.md`).

### 3.2 Pattern table (exact)

| Pattern | Values | Source |
|---|---|---|
| `halo-rise` (card/list entrance) | `from{opacity:0;transform:translateY(14px)}`; `dur-slow` `ease-out` `both` | `bundle.css:22-23` |
| Table row entrance | `halo-rise dur-base ease-out both`, `animation-delay: min(var(--i),12)*25ms` | `:247` |
| Notification row entrance | `halo-rise 500ms ease-out backwards`, `min(i,8)*50ms` | `:370` |
| `halo-page` (page change / AppShell `pageKey`) | `halo-page-in 520ms ease-out 60ms both`, `from{opacity:0;translateY(10px)}`, sidebar stays put, no blur | `:24-25`, `03-motion.md:11` |
| Menu / popover enter | `halo-pop-in dur-fast ease-out`, `from{opacity:0;scale(.96) translateY(-4px)}`; opens-upward variant `translateY(4px)`; leave `140ms` opacity+transform | `:406-413` |
| Menu item stagger | `halo-menu-item 220ms ease-out backwards`, `from{opacity:0;translateY(-3px)}`, `min(i,12)*18ms` | `:419-420,801` |
| Dialog enter | `halo-zoom dur-base ease-out`, `from{opacity:0;scale(.94) translateY(8px)}`; overlay leave `opacity 200ms`; dialog leave `scale(.96) 200ms` | `:284-292` |
| Command palette | same `halo-zoom dur-base` | `:292` |
| Drawer | `halo-drawer-in dur-base ease-out`, `from{opacity:0;translateX(40px)}`; left variant `-40px`; leave `opacity+translateX(40px) 200ms` | `:362-364,691-692` |
| Toast | `halo-toast-in dur-base ease-out`, `from{opacity:0;translateY(16px) scale(.96)}`; stack container `height dur-base`; leave `transform+opacity 240ms`; hover fans + pauses timers; drag right to throw, short drags spring back | `:314-324`, `Toast/README.md` |
| Skeleton shimmer | `::after` gradient `translateX(-100%)->100%`, `1.4s linear infinite` | `:100-102` |
| Skeleton -> content | `halo-reveal dur-slow ease-out both` `from{opacity:0;filter:blur(6px);translateY(6px)}`; bones `halo-skel-out 360ms ease-out forwards` `to{opacity:0;filter:blur(4px)}` | `:103-107`, `Skeleton/README.md` |
| Tabs underline | `.halo-tabs__bar` `transform dur-base, width dur-base ease-out` | `:266` |
| Segmented thumb | `.halo-seg__thumb` `transform dur-base, width dur-base ease-out`; label colour `dur-base` | `:92-93` |
| Active nav item | `halo-nav-in dur-slow ease-out` `from{background:transparent;color:ink-muted}` | `:271-272` |
| Toggle | track `bg/border dur-base ease-out, shadow dur-fast`; knob `transform dur-base ease-spring, width dur-fast ease-out` | `:141-142` |
| Checkbox / radio | check path `stroke-dashoffset dur-base ease-out` (draw) + `halo-pop dur-base ease-spring`; `halo-pop{50%{scale(1.18)}}` | `:154-160,547,664` |
| Slider thumb | `transform dur-fast ease-spring, shadow dur-fast ease-out` | `:181` |
| Accordion | panel `grid-template-rows 0fr->1fr dur-base ease-out`; content `opacity+translateY(-4px) dur-base`, `transition-delay:60ms` when open | `:499-504` |
| Count-up (`Stat`) | counts from old to new value, `ease-out`, default **1100ms** | `Stat/README.md` |
| Rolling number | each digit on its own wheel, strip `transform 900ms ease-out`, right-most first | `:737`, `RollingNumber/README.md` |
| `FlipList` / sorted `DataTable` | **460ms** `ease-out`; new items fade up; reduced motion off | `FlipList/README.md` |
| `LiveDot` | `::after` `halo-ping 1.8s ease-out infinite`, `scale 1->3`, `opacity .7->0`; idle variant `animation:none` | `:80-82` |
| `Spinner` | `halo-spin 900ms linear infinite` | `:448` |
| Indeterminate `Progress` | `halo-indet 1.3s ease-out infinite`, `width:40%`, `translateX(-100%->250%)` | `:460-461` |
| Bell + count | `halo-bell-swing 900ms`; count `halo-pop dur-base ease-spring`; `halo-ping 900ms` once; drawer slides 40px; rows 50ms stagger | `:353-357`, `03-motion.md:14` |
| Tooltip | `opacity+transform dur-fast ease-out`, `transition-delay:120ms` on hover | `:309-311` |
| Hover (buttons/cards) | buttons `transform,bg,border,shadow,color dur-fast ease-out`; cards `transform,border,opacity dur-base ease-out` + 3px lift | `:30,43` |
| Bar chart | bar `height dur-slow ease-out var(--d)`, siblings `is-dim` opacity .45 `dur-base`, value label `opacity+translateY(4px) dur-fast` | `:210-215` |
| Stepper | fill `transform:scaleX(var(--progress)) dur-slow ease-out` | `:542` |
| Crumbs | `halo-crumb-in dur-base ease-out backwards` x-4px, `min(i,6)*40ms` | `:508` |
| Calendar month | slide `translateX(+/-16px) dur-base` from the side you head to | `:648-651` |

---

## 4. Per-row verdict vs Halo

Every inventory row in section 1 carries its own verdict in the last column. This section gives the rules behind the codes so each can be executed mechanically, then the cross-cutting lists.

### 4.1 Mapping rules (apply to every `Tk` row)

1. **Tokens (one edit, ~570 sites).** In `app.css:239-250` set `--dur-fast:150ms; --dur-base:300ms; --dur-slow:600ms; --stagger:60ms`; add `--ease-out:cubic-bezier(0.2,0.8,0.2,1)`; alias `--ease-page` and `--ease-soft` to it (they carry ~98 and ~36 var sites) so nothing needs a per-site edit; keep `--ease-spring` unchanged (already identical). Retire `--dur-rise` (8 sites -> `--dur-slow`), `--dur-page`, `--dur-page-out` (PAGE rule; B3/B5/A5). `--pulse-live` 1.6s -> 1.8s if PING lands (19 sites).
2. **Literal sweep.** ~35 raw `cubic-bezier(0.22,1,0.36,1)` -> `var(--ease-out)`; the ~104 bare `ease` and ~130 `ease-out` keywords on *transitions* -> `var(--ease-out)`; raw `80/90/120/220ms`-style literals that are really "fast" or "base" -> tokens (rows A6-A8, C21, F5, G5...); Diagnostics' 12 fallback copies deleted.
3. **Hover/press (HOV).** colour/bg/border at `dur-fast`; where Rift already lifts (WorkspacePage `.pcard` transform, Welcome `.of-target`, Sidebar `.fnav-item:hover .snav-ic`) use Halo's 3px lift; press = scale .97 (today a mix: `translateY(1px)` Sidebar:341 and ui/button `translate-y-px`, `scale(.96)` Sidebar:326, 80ms transforms at app.css:403, AskUserCard:346, WorkspacePage:893).
4. **Spinners (H:SPIN).** Collapse the 25 keyframes into one global `halo-spin 900ms linear infinite` (the `.spin` utility at `app.css:625-628` is the natural home); delete per-component copies. Spinners deliberately stay alive under reduced motion today (PermissionBar:141-142) - see 7.5.
5. **Pulses (H:PING).** Every `box-shadow` keyframe pulse (D7 x5 sites, C6, E4, E10, E12, E18, F14, G10, G12, G18, H14, I3, I17, I22, A18) -> one `.live-dot::after` ping (`scale 1->3`, `opacity .7->0`, 1.8s ease-out). Also the largest paint-cost win (6.3).
6. **Layout-property animations (Te).** `width`/`height`/`left`/`top`/`margin`/`max-height` -> `transform: scaleX/scaleY/translate` or `grid-template-rows` (list in 6.4). Progress/fill bars specifically: C5, C14, D13, D18, G25, H12, I7, I23, I26, B36 -> `transform: scaleX(var(--p))`, `transform-origin:left` (exactly Halo's Stepper fill, `bundle.css:542`).
7. **Disclosures (H:ACC).** Replace the 10 `transition:slide` directives + `workOpen`/`gcReveal`/`bodyFadeIn` keyframes with grid-rows height + content fade with 60ms delay. Sites: B35, D2, D3, D11, D21, D22, D27, D30, E13, E14, F9, F12, G26, C22.
8. **Overlays.** Three divergent implementations today - tw-animate recipes (`ui/recipes.ts`, H1-H4), Svelte `fade/scale/fly` (H7, H9, H11, H13, B38), and the `.dialog-overlay` CSS (A13) - -> ONE vocabulary: `halo-zoom` / `halo-pop-in` / `halo-drawer-in` / `halo-toast-in` with their leave variants. Svelte directives may remain the mount/unmount carrier if given matching `css` functions; visible values must match 3.2.
9. **Entrances (H:RISE).** Nine unrelated rise keyframes today: `blockIn` (3px), `enter` (4px), `reveal-in` (7px), `pane-empty-in` (6px), `ob-rise` (8px), `ws-child-rise` (6px), `code-rise` (6px), `tool-rise`/`wb-rise` (5px), `ah-vs-in` (4px). One `halo-rise` (14px, `dur-slow`) for cards/pages/settings/welcome/onboarding; ONE dense variant (<=4px, `dur-base`) for in-stream blocks (D8).

### 4.2 Kill list (`X` rows) with reason

| Row | What | Why (Halo / DESIGN.md) |
|---|---|---|
| A5 | `--rail-w` root transition | root custom-property transition; duration disagrees with the thing it follows (5.6) |
| A9, A14 | `.pill.ok .dot` breathe, `.pulse` | loop on a non-activity state ("never loop for attention", `README.md:10`) |
| B3 | `.workspace` wsIn | superseded by PAGE; stacks with `.ws-page` + `.rising` (5.5) |
| B1 | `.app` 1s background transition | 1.7x `dur-slow`; not a Halo pattern (D9) |
| C9 | permanent `will-change: transform` on composer-host | only needed during the FLIP |
| C18, C19, G13 | welcome mark/dot breathe 4.2s/3.4s, `pill-ultra` 2.6s | decorative ambient loops (D11) |
| E3, E8 | `.turn-rail` glow loop, `node-stream-enter` blur | stationary liveness is the rule (stream.css:65-68; DESIGN.md:245-251); E8 likely dead (redesign-plan.md:204-206) |
| F6, F11, F17, F18, G1, G2, G16, G21, G22, I9 | blur backdrops | 6.1: perf + WebView2 ban; use the existing opaque `.hud-float` recipe (app.css:598) |
| G25 `up-pulse`, I26 `limitbar-pulse`/`limitbar-sheen` | `filter: brightness` loops, traveling sheen | filter animation + traveling band (DESIGN.md:227-251) |
| I12 `.save-btn`, I13 | `filter: brightness()` on hover | use Halo sibling-dim (`is-dim` opacity .45) or lift |
| I19 `.dl-orb` | orb breathe | decorative loop |

### 4.3 Keep list (Rift signature, no Halo analog; retune curve/token only)

Splash split + intro handoff (B4, B10, B15-B21, J8, J15, J16) · composer FLIP + welcome out/in (J5, J6) · word-reveal cadence (J2/J3, pending D2) · live-line shimmer verb (D5/D3) · AnimatedCount tween (J7) · terminal cursors/blinks (D15, D21, D23, G11) · send ripple (G8) · tid-flash / sflash jump highlights (E19, I5) · ledger receipt (5.4) · dock width slide (C1, tied to native webview sync).

### 4.4 Owner decisions (flagged, not decided here)

| ID | Question | Context |
|---|---|---|
| D1 | Skeleton shimmer: Halo ships a travelling shimmer (`bundle.css:101-102`) but Rift DESIGN.md:227-234 bans "banded sweeps over blocks/rows" (2026-07-21 owner call). Sites: B6, B37, H10, I14, I23, I24 (+ `shim` text, D5). | pick Halo shimmer, Rift idiom (idle cursor + timer + live dot), or only Halo's `halo-skel-out`/`halo-reveal` hand-off without the sweep |
| D2 | Per-word blur-in while streaming (Markdown `md-word`, EnhanceBar `word-materialize` 7px, Composer `placeholder-fade` 3px) vs Halo "blur only on small areas, once at a time". Plan-doc options: keep blur / opacity + 2px rise / shorter window (`redesign-plan.md:209-215`). | owner called streaming laggy 2026-10-19; WORD_MS 42->30, CATCHUP 520->320ms, lerp 0.2->0.35 already applied (Markdown:169,172; AssistantPane:183-185) |
| D3 | Verb `shim` on up to 5 live text lines (background-clip:text): owner-approved (DESIGN.md:233-234) but not compositor-only. Keep as a named exception to "only opacity/transform loop"? | |
| D4 | Onboarding mini-demos (22 concurrent infinite loops, 5-7s): teaching, not attention-seeking, but break the loop rule | |
| D5 | `dotPop` overshoot to scale 1.55 (stream.css:83-87; bullet-pop 1.3) vs Halo spring pop 1.18; Halo reserves spring for flip/grab | |
| D6 | Composer "magic" enhance effect (shimmer + aura + twinkle stars + wand pulse; G9/G10): 4 simultaneous decorative loops | |
| D8 | Rise distance: Halo 14px/600ms vs the stream's deliberately tiny 3px/240ms (owner cut 8->3px because follow-scroll chased growth, stream.css:25-26) | one Halo rise for pages/cards, one dense variant for in-stream |
| D9 | `.app` background 1s transition (AppShell:360): what drives it? (trigger not read) | |
| D10 | Splash `(0.65,0,0.35,1)` panel split: keep as a named exception curve? | only non-Halo-family curve |
| D11 | Ambient "breathing" (welcome mark, ge-dot, pill-ultra, pill.ok): Halo forbids loops that are not real activity | |
| D12 | Press feedback: Halo `0.97 scale`; Rift has translateY(1px) / scale .96 / ripple (G8) | |
| D13 | MentionPopover rows enter together by design (MentionPopover:60-61 "autocomplete needs to feel immediate") vs Halo MENUITEM stagger `min(i,12)*18ms` | |
| D14 | Reduced motion and spinners: keep alive as essential state indicators (PermissionBar:141-142) or follow Halo's blanket stop (7.5) | |

---

## 5. Gaps: Rift-specific motion Halo does not define

### 5.1 Streaming word reveal (the signature interaction)

Pipeline, all from code (derived; **not run or measured in this wave**):

1. Backend deltas -> `enqueueText` (`state/assistant/streaming.ts:393-409`): `pendingText += chunk`; if `document.hidden` flush immediately (rAF never fires hidden, #100); else start a per-tab rAF `drainTick`.
2. `drainTick` (`streaming.ts:428-446`): `dt = min(now-last, 100ms)`; `rate = max(360, pending/0.1)` chars/s, i.e. the whole backlog drains in ~0.1s; slices `n` chars into the message.
3. `Markdown.svelte` coalesces re-parse to one per rAF while streaming (`:196-202`). `wrapWords` (`:260-293`) wraps every prose word (not inside `pre`/`.shiki-block`) in `<span class="md-w-hold">` once per markdown delta; held spans are `opacity:0`, reserve layout, and carry **no filter** (`:689-691`).
4. One lifetime rAF loop (`:216-252`) advances `shownCount` by `floor(elapsed/WORD_MS)`, `WORD_MS = 30` (`:169`) = ~33 words/s; when the reveal trails delivered words by more than `CATCHUP_LAG = ceil(320/30) = 11` (`:172`) the step is multiplied by `lag/CATCHUP_LAG` (`:231-236`). The target overshoots `totalWords` by `REVEAL_WINDOW` so the tail finishes its blur (`:226`). The loop parks when caught up; `wakeReveal` restarts it (`:241-250`).
5. `applyReveal` (`:301-314`): spans crossing the cursor become `.md-w` (keyframe `md-word` 0.5s `(0.22,1,0.36,1)`: `opacity 0->1`, `filter blur(3px)->0`; `animation-delay = -(ageWords*WORD_MS)ms` set once so a re-wrap resumes mid-blur and never restarts); spans `REVEAL_WINDOW = ceil(500/30)+1 = 18` words behind the cursor become plain `.md-w-shown`. So ~18 spans carry a live blur animation at steady state.
6. "Chrome-hold" (`:708-727`): an element (code chip, heading, list item, blockquote, table row, hr, pre, img) whose words are all still held is itself `opacity:0` (`:has(.md-w-hold):not(:has(.md-w,.md-w-shown))`) and fades in over `dur-base` when its first word reveals. Code blocks/images are additionally held while any earlier sibling still has held words (`:727`).
7. Reduced motion: the whole mechanism is bypassed (`:198,217,622`; CSS `:696-699` makes held spans visible).

Latency chain (derived from constants): char pacer <=~100ms -> reveal cadence 30ms/word -> 500ms blur-in -> follow lerp (5.2). `docs/design/redesign-plan.md:183-215` documents an earlier state (42ms / 520ms / 0.2 lerp; "measured" 12,569 frames, p99 5.7ms, ~180Hz display, dev build = NOT dropped frames, the lag was pacing). **Those numbers in the plan doc are now stale**; code comments at Markdown:169,172 and AssistantPane:183-185 record the 2026-10-19 changes.
Halo: no equivalent (`halo-reveal` is loading->content, not per-word). Halo's perf rule (blur small + once at a time) is bent by ~18 concurrent blurred spans; see D2 and 6.2.

### 5.2 Scroll follow

`AssistantPane.svelte:158-191`: while `stickToBottom`, `pinToBottom()` runs a rAF loop `scrollTop += delta * 0.35` **per frame**, snapping straight to the bottom when `delta > 900`; it re-reads the target every frame so bursty growth reads as one motion. `pinEcho` records every position we write so scroll echoes do not un-latch; only a position we did not write un-latches (`:193-206`, latch threshold gap < 80). A `ResizeObserver` on `.messages` calls `pinToBottom()` on every size change (`:208-218`); message-count / streaming flips re-pin after `tick()` (`:220-227`). Tab switch restores the cached scroll instantly (`:229-250`; comment: a glide would animate across the other tab's content). `jumpToLatest` and exchange-jump use native `scrollTo({behavior:"smooth"})` with a 700ms `glideUntil` latch window (`:139-148,252-257`); a wheel event clears `glideUntil` (`:714`).
- **Frame-rate dependent**: a fixed 0.35 per *frame* converges 90% in ~5 frames = ~83ms at 60Hz but ~28ms at 180Hz (derived arithmetic; the plan doc's display was ~180Hz).
- Reduced motion: instant pin (`:167,171`); the two native smooth scrolls are **not** guarded (J13). Settings' `scrollIntoView` is guarded correctly (I10).
- Halo: nothing (AppShell content "scrolls on its own"; no follow behaviour).

### 5.3 Live tool rows / live blocks

Current behaviour (default `streamMode`, `state/ui-prefs.svelte.ts:105`): each block mounts with `blockIn` (opacity + 3px, `dur-base`, `both`); the live line wears a text shimmer (`shim` 1.5s linear on five selectors, `stream.css:328-342`); status dots pulse via `streamLivePulse` (box-shadow ring 0->7px, `pulse-live`) and pop in via `dotPop` (scale 0->1.55->1, 420ms spring); spinners are `ringspin` 0.9-1s; a tool-run `.wline` mounts as a `width: fit-content` chip and morphs to `100%` on open (`interpolate-size`), its rows entering with `workOpen` 0.24-0.3s; shell output lines enter with `termLineIn` plus a JS stagger `min(i*16,400)ms` (OutputBlock:119); numbers (tokens, diff +N/-M) tween via rAF 420ms ease-out-cubic and roll a glyph (J7/D6); floating HUDs (plan/agents) enter with `ahud-in`/`phud-in` (scale .96, y -10px, `dur-base`); the exit-plan card typewrites the plan at 20 chars / 30ms (D31).
Unknown: whether history hydration (settled turns mounting for the first time) replays `blockIn` on all ~18 block classes - no settled-turn gating class was found in `stream.css:73-546`; only `StreamShell` reads `mountedLive` (`:75,122`, body not read).
Halo analog: `RISE`, `LiveDot/PING`, `Spinner`, `COUNT/ROLL` - but **no** pattern for "a row that is actively working, then settles". The chip->ledger width morph, the live shimmer line and the terminal cursor are Rift-only and should be specified as a short list of named exceptions.

### 5.4 Turn fold-to-receipt (live -> settled)

`stream/StreamTurn.svelte:412-420` vs `:420-489`: while `streaming` the turn renders `.sturn-body` (thinking + chronological groups). The moment streaming ends it is an `{#if}/{:else}` **hard swap**: the live body unmounts and the settled view mounts - answer text first (`.sanswer` > `.snarr`, whose `blockIn` replays because the nodes are new; derived) then ONE ruled `.sledger` line (outcome · count · duration · cost; tokens on hover; Copy/Retry). There is **no fold, collapse or crossfade between the two states**, and `.sledger` itself carries no entrance animation (`stream.css:394-445`: transitions on toggle/hover/acts only). Expanding the receipt uses `transition:slide` 180ms (`StreamTurn:474`, **no reduced-motion guard**) and the chevron rotates at `dur-fast` (`.tile-chev.open`, app.css:592). The legacy renderer does the analogous thing with `.work-body` `slide` 200 + `workOpen` 0.34s on the same node (E13/E14).
Halo analog: ACC for the expand. For the swap itself the nearest Halo idiom is the skeleton hand-off (`halo-skel-out` 360ms blur 4px + `halo-reveal` 600ms) or `FlipList`. This is the most visible "missing motion" in the chat and needs a bespoke short opacity/height crossfade (a design item, not a retune).

### 5.5 Tab / page switching

- **Workspace switch** (`shell/WorkspaceShell.svelte`): every once-opened workspace stays mounted in an absolute stack; inactive = `opacity:0; translateY(3px); pointer-events:none; inert`, transitioning out at `dur-page-out` 140ms `ease-soft`; active = `opacity 1; none` in at `dur-page` 400ms `ease-page` (`:128-148`). Additionally the first five direct children rise (`ws-child-rise`, 6px, `dur-rise` 460ms, delays `0..4 x 62ms`) for 810ms while `.rising` (`:47-73,198-211`); under reduced motion `.ws-page` transitions are `none` and `.rising` never fires (`:189-191,198`). `AppShell .workspace` has its own `wsIn` 400ms translateY(7px) (`AppShell:475-486`; runs at boot only - inferred from the persistent element, not run) and is suppressed during `data-intro` (`:493`). Net: up to three stacked motions on a page change (page opacity/translate + children rise, + shell wsIn at boot). Halo page change = ONE motion: `halo-page-in` 520ms/10px/60ms delay, sidebar static, no exit (`bundle.css:24-25`).
- **Chat tab switch** (assistant tabs): no tab-strip component exists (`Glob **/*[Tt]ab*.svelte` -> none); switching restores scroll instantly (`AssistantPane:229-250`) with no cross-fade. Split panes enter with `paneIn` (opacity + 4px, `dur-base`, `AssistantPane:894-895`).
- **Settings tabs**: sliding `.tab-ink` underline `transform+width dur-base ease-page` (`SettingsPage:1596`) - already the Halo THUMB pattern.
- No `startViewTransition`, `crossfade`, `tweened` or `spring` anywhere in `src/` (grep, J19).

### 5.6 Sidebar collapse / peek

`shell/Sidebar.svelte`: the rail is a layout spacer whose **width** transitions at `dur-base` `ease-soft` (`:218-221`; the file's own comment: "every frame reflows + rewraps the chat stream, so keep the animation SHORT", `:219-220`); the island inside translates `-16px` + fades (`:223,274`; `top` is also transitioned for the peek flyout, `:227-228,274`); resizing disables transitions (`:222`). `AppShell .main` animates `margin-left` at `dur-base` `ease-soft` "in lockstep" (`AppShell:415-417`). A registered `@property --rail-w <length>` on `<html>` transitions at `var(--dur-page-out, 240ms)` (`app.css:347-354`) so consumers (the top-center stack) glide with the island - **but `--dur-page-out` is defined as 140ms (`app.css:244`), so the fallback never applies and that tween ends ~100ms before the 240ms width/margin move** (derived from the token values; not observed running). Reduced motion: `--rail-w` sits inside `no-preference` (`app.css:352`); the width/margin/island transitions are **not** covered (`Sidebar:349-351` zeroes only `.snav-ic` and `.fnav-item`).
Halo: AppShell collapses to a 72px icon rail, labels fade, `flex-basis`/`width` at `dur-base ease-out` (`bundle.css:703`) - also a layout animation, so the per-frame cost is the same; the gain is one coordinated move instead of three (width + margin + custom property).

### 5.7 Splash and cold-boot choreography

`SplashOverlay.svelte`: an opaque veil holds until `assistant.configLoaded && workspaceReady` (and >= `MIN_MS` 900ms; hard cap 10s; boot readout only after `SLOW_MS` 1400ms) (`:18-33,64-90`). Entrance: mark `settle-mark` 560ms (opacity, y 8px, scale .94, **blur 8px->0**, 140ms delay), wordmark letters `letter-rise` 460ms staggered `300ms + i*55ms` (blur 4px->0). The readout bar is `transform: scaleX(shownP)` driven by an rAF smoother (pull `dt/240`, creep `0.00005*dt` capped at target+0.08 and 0.96). Exit: `exiting=true`; `intro.handoff()` at +80ms; the two chassis halves translate `-/+101%` over 520ms `(0.65,0,0.35,1)` after a 140ms delay while the lockup fades 240ms with blur 3px; `onComplete` at 680ms (120ms when reduced). `state/intro.svelte.ts`: `veil` -> `handoff` (islands rise under the lifting veil: `.main` opacity 0 + translateY(12px) -> rest over `dur-rise` with 60ms delay; sidebar slides in from -16px) -> `settled` after 700ms; skipped entirely under reduced motion (`:10-12`).
Halo has no splash. Everything here is Rift identity; only the curve/token vocabulary is retuned (D10). `will-change: transform` on the panels (`:150`) lives only while the splash is mounted.

### 5.8 Other Rift-only motion (no Halo pattern)

- **Composer FLIP** (J5): hero-centre <-> docked, inline `transition: transform 460ms cubic-bezier(0.22,1,0.36,1)`, with `welcomeOut` (fixed-pin, 200ms) / `welcomeIn` (220ms); the SettingsMenu follows it with a per-frame rAF (J10).
- **Browser dock slide** (C1): animates real `width` deliberately so the native child webview's `syncBounds` tracks (AssistantPage:11-14).
- **Context ring / usage bars** (G14, G25): dashoffset / width tweens at `dur-slow`.
- **ConversationList** row stagger + selection bar + working dots (B27-B32).
- **Dictation** waveform (J9, G6): rAF audio meter + 80ms CSS height transitions.
- **Toast systems**: three families (ToastHost fly+flip B38, UpdateBanner slide B35, NotificationCenter fly/fade B33) vs Halo's single Toast + NotificationPanel + Banner family.

---

## 6. Perf hazards

Ranked by expected cost under WebView2 (Chromium). **None measured in this wave** (no app run); the only measured fact is `redesign-plan.md:183-185` (dev build, rAF sampler, 12,569 frames, p99 5.7ms).

### 6.1 `backdrop-filter` / `filter: blur` (16 backdrop declarations + animated/static blur filters)

| Site | Kind | Layer | Note |
|---|---|---|---|
| `composer/SettingsMenu.svelte:519-520` | backdrop 20px + saturate 1.4 | **`position:fixed`**, z 9998 | violates `app.css:609-611` ("WebView2 mis-composites it on fixed elements") |
| `shell/ConversationList.svelte:512` (`.conv-menu`) | backdrop 12px + saturate 1.35 | **fixed** (portal) | same ban; its comment notes the older 60%/blur(26) was "expensive to composite" (`:510-511`) |
| `shell/ProjectSwitcher.svelte:266` (`.sw-menu`) | backdrop 12px + saturate 1.35 | **fixed** (portal) | same ban |
| `Composer.svelte:1921` (`.composer-box`) | backdrop 14px + saturate 135% | in-flow, **always on, full column width, over a scrolling/streaming transcript** | re-samples the backdrop while text streams |
| `Composer.svelte:1886`, `composer/EnhanceBar.svelte:203`, `composer/SlashMenu.svelte:143`, `stream/ActivityHud.svelte:326`, `stream/PlanHud.svelte:224` | backdrop 12/14/18/20/20px | absolute | the HUD pair is live during every agent/plan turn (comment "extra blur texture", ActivityHud:322-323) |
| `ToolChip.svelte:730`, `bubble/BoundaryBlock.svelte:113`, `bubble/ModelSwitchBlock.svelte:63`, `settings/SettingsPage.svelte:1624`, `styles/onboarding.css:154`, `Markdown.svelte:1212` | backdrop 8/8/8/14/8/1.5px | in-flow / absolute | ToolChip terminal-cmd repeats per tool chip |
| animated `filter: blur`: `md-word` (Markdown:693), `node-stream-enter` (MessageBubble:821), `word-materialize` 7px (EnhanceBar:397), `placeholder-fade` 3px (Composer:2010), splash x3 (B16-B19) | filter | | Halo allows blur only on small areas, once at a time |
| static `drop-shadow` filters | Composer:2365,2590; Splash:188; AssistantWelcome blur(20px)/(0.4px) decorative layers at `:571,577` | | cheap-ish, paint-time |

### 6.2 Many concurrent animated blur spans
`md-word` keeps ~18 spans animating `filter` at steady state (5.1). Chromium can composite filter animations but each animating inline span becomes its own layer/surface; held spans were already stripped of filter (`:689-691`, plan item 1). Dev frame times were fine; risk is lower-end GPUs and long bursts.

### 6.3 Paint-per-frame loops (not compositor-only)
- **box-shadow keyframe pulses** (repaint): `streamLivePulse` x5 declarations (stream.css:80,189,321,370; StreamAskUser:342), `pane-live-pulse`, `rail-stream`, `avatar-halo`, `tl-bullet-pulse` x3, `nodePulse`, `live-pulse`, `wand-pulse` (+scale), `dc-breathe`, `ah-vs-pulse`, `livePulse`, `pill-breathe`, `ob-live`, `ob-dial` x4. >= 20 declarations; during a streaming turn ~6 can run at once (header dot, footer dot, active-line dot, write-batch dot, pane dot, + avatar/rail on the legacy renderer).
- **`background-position` shimmers**: `shim` on `background-clip:text` x5 selectors (stream.css:329-342, up to 5 simultaneously, repaints glyphs each frame), `magic-shimmer` (Composer:2333), `mcp-shimmer`, `skel-shimmer`, `ws-load-shimmer` (3-4 bars), `wb-shimmer`. (`sk-sweep`, `limitbar-sheen`, `dl-slide`, `wb-sweep` are transform-based: fine.)
- **`filter: brightness()` animation**: `limitbar-pulse`, `up-pulse` (infinite while a usage zone is hot); hover filters on chart columns (WorkspacePage:920,947,1074).
- **`steps()` blinks** are cheap but each is a perpetual timeline: cursors (D15, D21, D23, B20, G11, I29).

### 6.4 Layout-thrashing animations (animating layout properties)
`Sidebar .side-rail width` (B8; reflows the whole chat each frame, per its own comment) · `AppShell .main margin-left` (B2) · `dockSlide width` (C1; also resizes a native webview every frame) · `.wline width` (D11) · `html --rail-w` registered property (A5; style recalc of its consumers each frame) · width fills: C5, D13, D18, G25 (`up-grow from{width:0}` + `transition:width`), H12, I7, I23, I26 (x3), B36, `SettingsPage .set-progress-fill`, `ob-fdot` · height: I7 `.iface-row`, I23 `.ah-sc-barwrap`, G21 `.sm-bar`, `mic-wave span` (J9), `SettingsMenu legacy-flyout max-height` (G23), the 10 `transition:slide` sites · left/top/width: `SettingsMenu .er-knob left`, `.er-track::after width` (G23), `ob-tree-scan top`, `ob-mcl-type max-width` (I29), `Sidebar .sidebar top` (B9), `.sset-search width` (I4).

### 6.5 Always-running work
- Permanent per-frame rAF while a popover is open: `SettingsMenu.svelte:74-81` calls `position()` (layout reads) every frame until closed.
- Per-instance rAF tweens: `AnimatedCount` (one per live counter; J7), Markdown reveal loop (one per streaming message; parks when caught up - good), stt meter (while dictating), splash smoother (boot only).
- `:has()` chrome-hold selectors in `Markdown.svelte:712,718,727` re-evaluate on every class flip (30ms word cadence) over the message subtree; probably fine at chat lengths, unmeasured.
- Timers that re-render text each second: `ConversationList.svelte:171` (`nowTick` 1s), `ActivityHud:119`, `StreamAgent:71`, `StreamShell:50`, `StreamTurn:191`, `MessageBubble:74` (500ms). Visual cost = text updates only.
- 115 `infinite` animation declarations in the tree; onboarding alone declares 22 concurrent mini-demo loops (I29) while that screen is up. Typical idle chat screen: 0-3 loops; streaming turn: ~6-10 (estimate from selectors, not counted live).

### 6.6 Entrance replay on mount
`.bubble` enter (E2), stream `blockIn` on ~18 classes (D1), `.crow` rowIn (B27; delay capped at 12 but the animation applies to every row), `.set-col > .card` (I6), per-row `.diff-line/.diff-meta/.diff-gap` (F12): on hydration of a long conversation all of these can start at once. Halo caps *delay* not *count*, but its lists are small. No "history vs live" gating found in the stream renderer (5.3).

### 6.7 Layer promotion
`will-change: transform` is permanent on `.composer-host` (`AssistantPane.svelte:1160`) and on the splash panels while mounted (`SplashOverlay:150`). Nothing else uses `will-change`.

---

## 7. Reduced-motion coverage gaps

### 7.1 Mechanism
- Rift handles reduced motion **per selector**: ~48 files with `@media (prefers-reduced-motion: reduce)` blocks or a JS probe; Halo uses one global net (`bundle.css:901-902`). Anything added later and forgotten is silently unprotected - 13 motion files have no handling today (7.2).
- `prefersReducedMotion()` (`state/accessibility.svelte.ts:103-106`) is a **one-time snapshot** called at component init (e.g. `Markdown:158`, `AssistantPane:167`, `ToastHost:14`, `SplashOverlay:31`, `AnimatedCount:23`): flipping the OS setting while the app is open does not reach already-mounted components, whereas the CSS media queries are live. No in-app "reduce motion" setting exists (the accessibility store only mirrors dyslexia/font/line-height/warm-tint to `data-a11y-*`, `:85-95`; no "motion" key in the file besides the probe).
- The global `[data-slot]{animation:none!important}` (`app.css:117-122`) covers only bits-ui primitives (dropdown/select/context/alert-dialog); their `transition-*` utilities (button press, select chevron) are not covered.
- Several animations are only *defined* inside `@media (prefers-reduced-motion: no-preference)` (NP) - correct but fragile: `.spin/.pulse/overlay-in` (`app.css:623-629`), `pill-breathe` (`:459`), `--rail-w` (`:352`), welcome breathes (`AssistantWelcome:447,470`), `.dl-orb` (`NewsFeed:255`), `ws-child-rise` activation (`WorkspaceShell:198`), `ah-wrap` (`AiHealthPage:1253`), onboarding `ob-rise`/`ob-live`/mini-demos (`onboarding.css:109,323,480`), Markdown chrome-hold (`:708`).

### 7.2 Files with motion and zero reduced-motion handling (13)
(grep: file contains `animation`/`@keyframes`/`transition` but none of `prefers-reduced-motion|reducedMotion|reduceMotion|prefersReducedMotion`)

| File | Unguarded motion | Severity |
|---|---|---|
| `assistant/AssistantPage.svelte` | `dockSlide` width+opacity 220ms (C1); divider hovers | movement |
| `assistant/stream/StreamTurn.svelte` | `slide` 180 receipt expand (D30) | movement |
| `assistant/stream/StreamResult.svelte` | `slide` 140 (D22) | movement |
| `assistant/composer/QueueRail.svelte` | `fly y:7/5`, `slide` 160, staggered `in:fly` (G26) | movement |
| `dialogs/CommandPalette.svelte` | `fade` 130 + `scale` .97 170 (H7) | movement (scale) |
| `dialogs/McpServersDialog.svelte` | `fade`/`scale` (H9), `mcp-spin`, **`mcp-shimmer` infinite**, staggered skeleton (H10) | movement + loop |
| `assistant/toolchip/AskUserCard.svelte` | `chip-spin` x2, press transform 80ms | spinner + minor |
| `assistant/toolchip/TodoCard.svelte` | `chip-spin` x2 (1.1s/1s), colour transitions d-b | spinner |
| `assistant/composer/CtxRing.svelte` | `stroke-dashoffset` d-s tween | small |
| `assistant/bubble/BoundaryBlock.svelte` | hover `transform` + chevron | minor |
| `assistant/composer/AttachmentsRow.svelte`, `assistant/stream/GrepResult.svelte`, `shell/Titlebar.svelte` | colour-only hovers | none needed |

### 7.3 Svelte transitions without a guard
`dialogs/UpdateDialog.svelte:95,103` (fade 120 + fly y8 180; the file's media block at `:558-561` covers spinner + hero fill only) · `diagnostics/DiagnosticsConsole.svelte:163,317` (scale .98 180 + fade 140) · `CommandPalette:311,318` · `McpServersDialog:133,141` · `QueueRail:99,115,125,126` · `StreamResult:51` · `StreamShell:211` · `StreamTurn:474` · custom `dockSlide` (`AssistantPage:307`). Guarded correctly (duration/y forced to 0): ToastHost, NotificationCenter, UpdateBanner, ActivityHud/PlanHud `out:fade`, MessageBubble, ToolChip, EditDiff, WriteBatch, TurnSummary, the welcome transitions, and the JS in Markdown / AnimatedCount / SplashOverlay.

### 7.4 Partial coverage inside otherwise-guarded files
- **MessageBubble**: `.bubble` enter (`:626`), `.stage-strip` enter (`:763`), `.work-body` workOpen (`:1004`), `.dots .dot` pulse (`:1317`), `.live-dot` live-pulse (`:1339`), `.work.live .work-spark` nodePulse (`:989`) have no reduce rule (reduce blocks at 683, 712, 782, 824, 867, 1056, 1157, 1374 cover rail, avatar, stage-dot, tl-node, bullets, divider, cost-pill, tid-flash only).
- **stream.css reduce list** (`:380-381`) omits `.ssteer` (`:117`), `.wline-row` (`:279`), `.wr-out` (`:304`), `.sxa-spin` (`:503`), and does not zero `.wline`'s `transition: width` (`:263`) or `.splan-fill` width (`:455`). `StreamShell` reduce (`:237-240`) covers prompt pulse + cursor only, not `.sshell` blockIn (`:227`). `StreamAskUser` reduce (`:424`) covers the spinner only, not `.sask` blockIn (`:324`), `.sask-dot` pulse (`:342`) or `dotPop` (`:390`).
- **Composer**: `.drop-overlay` drop-in (`:1875`), `.drop-dot` pulse (`:1895`), `.char-count` enter (`:2209`), `.model-dot` provider-pulse (`:2533`), `.composer-box` transitions (`:1926`), `.icon-slot` (`:2285`) unguarded. **EnhanceBar**: `slash-in` (`:213`), `status-pulse` (`:305`) unguarded (reduce at `:400` is `.ew` only). **ToolChip**: `.tile` enter (`:535`) and `chip-spin` unguarded (reduce `:724` is `.body-frame` only). **Markdown**: `.md pre` code-rise (`:777`) unguarded (reduce `:1094` is `.shiki-block`). **AssistantWelcome**: `gcReveal` (`:544`) unguarded. **ConversationList**: `.crow.on::before` barPop (`:484`) - `.crow{animation:none !important}` (`:528`) does not reach the pseudo-element; `.conv-menu` convPopIn (`:515`) unguarded.
- **Layout/size motion unguarded**: Sidebar rail width + island (`:221,274`), AppShell `.app` bg 1s + `.main` margin (`:360,417`), `LimitBar` panel/card width (`:38,53`; only compact is zeroed at `:80`), `SettingsPage` `.plan-gauge-fill` (`:1733`) and `.iface-row` height (`:1762`), `AiHealthPage .ah-sc-barwrap`/`.ah-fill` (`:1394,1464`), `AssistantPane .pane-ctx-fill` (`:1022`), `.tr-tick::after` (`:1432`), WorkspacePage hover lifts/filters (`:920,1074,1111`). (`UpdateDialog .hero-fill` is covered, `:560`.)
- **Native smooth scroll unguarded**: `AssistantPane:145,256`, `ActivityHud:163`; dev gallery `scroll-behavior:smooth` (`routes/dev/gallery/+page.svelte:162`, dev only).
- `onboarding.css:361-362` `.spin` spinner (and its keyframe) sits outside the `no-preference` blocks.

### 7.5 Under-reduction (still moving when reduced) - contradicts Halo's blanket stop
- `shell/Skeleton.svelte:51-56`: under reduce it swaps the sweep for an **infinite** `sk-breathe` opacity loop.
- Spinners kept on purpose: `PermissionBar:141-142` ("state indicator, not decoration"); `AskUserCard/TodoCard/AgentCard/ToolChip` `chip-spin` have no rule at all. Halo's override stops *all* iteration after one 0.01ms cycle, which freezes a spinner on one frame. Owner call D14: keep spinners alive as essential state indicators or follow the net. Recommendation shape: global net + an explicit `.halo-spin`/`[data-essential]` exemption.
- `StreamExitPlan` typewriter is skipped when reduced (good, `:35`); `NewsFeed` digest label rotates every 2.6s (`:72`) with no transition either way.

### 7.6 Shape of the fix (for the build plan)
One global safety net equal to Halo's (`bundle.css:901-902`) in `app.css`, with an opt-out for essential indicators; keep the JS probe but make it reactive (`matchMedia(...).addEventListener('change')` feeding a `$state`); delete the per-selector blocks as the net lands; add `reducedMotion` to the 10 Svelte transition sites in 7.3 or move them to CSS classes the net covers.

---

## Unknowns / not verified

- No app run: every duration/latency figure is read from code, none measured. Pixel behaviour of the stacked page-change motions (5.5), the `--rail-w` vs width mismatch (5.6) and the `interpolate-size` morph on `.wline` (D11) are unobserved.
- Not read: `SlashMenu .sm-item` stagger delay (`:240-248`), `ui/recipes.ts:27,53` and `ui/button/button.svelte:7` long lines (only grepped), `Composer .icon-slot` trigger (`:2279-2290`), what drives `AppShell .app` background transition (B1), whether the `.pulse` utility (A14) is used, whether history hydration replays stream `blockIn` (5.3).
- `tw-animate-css` default keyframe values are inferred from class names (`zoom-in-95` = scale .95, `slide-in-from-*-2`), not read from node_modules.
- Counts are first-line grep counts (multi-line `transition:` lists and `style:`-set values undercount). The selector-resolved scan behind section 1 covered `<style>` blocks and `.css` files only (675 declarations incl. `none` resets); Svelte directives, JS timers/rAF and inline `style.*` writes came from separate greps (`AssistantPane:370-379` is the only inline `style.transition` writer).
