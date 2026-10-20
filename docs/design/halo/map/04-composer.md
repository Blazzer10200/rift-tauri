# 04 · Composer map (Halo redesign, MAP wave, read-only)

Scope: composer + everything attached. Branch `design/chat-ledger`. All cites are `path:line` relative to `src/lib/components/assistant/` unless prefixed. Counts were taken from the files this session (`grep -o`; "line-count" noted where a tool counted lines). Unknown = stated. Nothing run (no app, no build, no tests).

Notation: C = `Composer.svelte`; AP = `AssistantPane.svelte`; HALO-CSS = `…/artifact-files/…/project/components/bundle.css`; HT = `consolex/panel/design-systems/halo/tokens.css`.

---

## 1 Files & roles

### Composer proper (this wave owns these)

| File | Lines | Role |
|---|---|---|
| `Composer.svelte` | 2604 (script 1-1416 · template 1418-1843 · style 1845-2604) | God component. Draft/queue/streaming binding per tab, send/queue/stop modes, key orchestration, slash+mention state, enhance state machine, dictation (PTT + Ctrl+D + mic), paste/drop/pick attach, drag overlay, hero vs docked posture, model/perm pill triggers. |
| `composer/SettingsMenu.svelte` | 970 (script 1-271 · tpl 273-500 · style 502-970) | Model + provider tabs + legacy flyout + Fast toggle + effort detent slider + caption. Portaled `position:fixed`, rAF follow loop (74-81). 14 `oklch()` literals. |
| `composer/SlashMenu.svelte` | 454 (tpl 80-128 · style 130-453) | `/command` palette. CSS is TWO stacked layers: glass palette 133-346, then "compact command rail" override 348-453 that undoes most of it (see §8). |
| `composer/EnhanceBar.svelte` | 403 | Prompt-enhance preview panel (stream-in words, Ground/Edit/Diff toggles, refine chips, steer input, undo-mini pill). |
| `composer/UsagePanel.svelte` | 353 | `mode="ctx"` card hanging over the ring; `mode="full"` plan-limit panel (also mounted by `shell/StatusBar.svelte:210`, outside this wave's markup). |
| `composer/QueueRail.svelte` | 206 | "Next up" disclosure above the well: ordered rows, inline edit, drag reorder, Clear all. Uses svelte `fly`/`slide` transitions (99,115,125-126). |
| `composer/AttachmentsRow.svelte` | 147 | Image/text-file chips + attach-error pill. Render + click only. |
| `composer/MentionPopover.svelte` | 110 | `@file` popover. Render + click only. |
| `composer/PermMenu.svelte` | 119 | Permission-mode radio menu on bits-ui `DropdownMenu` (`ui/dropdown-menu`), Tailwind classes via `ui/recipes.ts`. ZERO own CSS. |
| `composer/CtxRing.svelte` | 84 | 16px context-window gauge button (R=6 ring, tone ok/warn/danger at 75/90, `CtxRing.svelte:23-27`). |
| `composer/modelMatrix.ts` | 607 | Pure model/effort/perm tables + helpers (`MODEL_OPTIONS` 79, `dialStopsFor` 516, `settingsRowsFor` 578, `MODE_OPTIONS` 590, `permToneFor` 602). Logic-only, tested. |
| `composer/helpers.ts` | 275 | Pure: `fuzzyScore` 8, `slashCatalogScore` 62, `slashMatchSegments` 81, `isFileDrag` 127, `attachImageFiles` 187, `attachTextFiles` 211, `queueChipLabel` 253. Tested. |
| `composer/helpers.test.ts` / `modelMatrix.test.ts` | 196 / 270 | Logic tests. No markup. |

### Composer-ADJACENT (mapped briefly; owning wave should be named by the orchestrator so they are mapped exactly once)

| File | Lines | Actually lives where | Composer link |
|---|---|---|---|
| `GhPopover.svelte` | 285 | NOT in the composer any more. Mounted by `AssistantWelcome.svelte:199` and `shell/StatusBar.svelte:162`. The composer's branch crumb was removed 2026-10-19 (C:93-97). Hand-rolled `.rift-menu gh-pop`, `use:portal`, `role="dialog"` (GhPopover:91-98). | `inject()` writes `tab.draft` (GhPopover:83-88). Belongs to the welcome/shell waves. |
| `PermissionBar.svelte` | 147 | In-stream, under the gated tool block: `MessageBubble.svelte:420,423`, `stream/StreamTurn.svelte:383-393`. `.tile.is-ask perm-bar` + Allow/Deny `.perm-btn`. | None to the composer; it is a stream tile. Belongs to the stream/tiles wave. |
| `toolchip/AskUserCard.svelte` | 503 | Stream-owned (`ToolChip.svelte:448`; stream variant `stream/StreamAskUser.svelte`). | Composer link = `askTarget` (C:694, 727-737 reroutes Enter to answer the card; placeholder C:1598-1599) + the card hint "Or just type in the composer below" (AskUserCard:283). Own styles: `.ask-option`, `.ask-btn` (duplicates PermissionBar's `.perm-btn` idiom). Belongs to the stream wave. |
| `AssistantPane.svelte` | (host) | Hosts `<Composer>` at AP:809 inside `.composer-host` (AP:802-810). Owns the hero↔docked FLIP, `engaged` posture, `composerH` ResizeObserver. Not mine to restyle but the composer's motion contract is defined here (see §3/§4). | |
| `shell/StatusBar.svelte` | (host) | Second consumer of `UsagePanel` (`anchor="statusbar"`). | |

Import map (C:2-32): lucide `Send, Square, X, Mic, Loader2, Wand2, Paperclip, Sparkles, ChevronUp`; bits `DropdownMenu` (22); stores `assistant`, `github`, `notify`, `stt`; actions `tooltip` (31).

---

## 2 Layout & anatomy

DOM tree (template line refs, C unless noted):

```
.composer-host                                (AP:802, z1, will-change:transform AP:1156-1161)
 └ .composer-wrap [data-model]                (1420; padding 6px 18px 10px; max-width min(--chat-col-max,880px) 1846-1853; sets --model-color:var(--accent) 1857)
    ├ QueueRail  .pending-rail                (1421; margin 0 2px 7px, QueueRail:157)  — above the shell, in flow
    └ .composer-shell  role=region            (1426; position:relative z1 1860) — drag handlers here
       ├ .drop-overlay > .drop-pill            (1436-1443; abs inset0 z8)   "Drop image to attach"
       ├ EnhanceBar .enhance-panel             (1444-1462; ABS bottom:calc(100%+8px) z10) — floats ABOVE shell, overlaps stream
       ├ SlashMenu                             (1464; ABS bottom:100%+8px; width min(440px,100%) z10)
       ├ UsagePanel                            (1474; ABS bottom:100%+8px; ctx = right-anchored 290px z10)
       ├ MentionPopover                        (1478; ABS bottom:100%+8px; full width, max-h 280 z10)
       └ .composer [hero|streaming|enchanting] [data-mode=send|queue|stop]   (1487)
          ├ .composer-box (THE WELL, overflow:hidden, radius 12, glass+blur 14px)  (1490; css 1916-1930)
          │   ├ .ps-row ghost-suggestion chip + dismiss      (1491-1522; only when promptSuggestion && !draft && !streaming)
          │   ├ AttachmentsRow                                (1523-1530)
          │   └ .cbox-row > .textarea-wrap                    (1531-1621)
          │        textarea[rows=1, placeholder=""]           (1533-1591) — native placeholder is transparent (1986-1988)
          │        .placeholder-ghost.static × 5 variants     (1592-1604)  — fake placeholder span
          │        .dict-ghost (mirror div)                   (1605-1609)  — dictation tail
          │        .magic-aura + .magic-text + .magic-stars   (1610-1619)  — enchanting overlay
          └ .composer-bar  (control deck, FLAT, BELOW the well; `{#if !hero}` 1624-1840; container-type:inline-size 2041)
              ├ .cbar-l (1629-1752): perm pill [DropdownMenu] · sep · attach · mic · wand(reveal) · char-count(>500)
              └ .cbar-r (1754-1838): model-pill → SettingsMenu(portal) · CtxRing(if ctx>0) · send-btn
```

Sizes (css): textarea min 28 / max 340 (1975), padding 9px 12px (1976), font `--fs-md` (13px; compact 12.5, comfy 14 via `app.css:289,291`), line 1.5; hero textarea 14.5px/1.55, pad 11px 12px 11px 14px (1948). Well radius 12 (hero 14, pad 3), pad 2 (1919). Every bar control height 34px: `.cbtn` 2072, `.send-btn` 34×34 pill (2220-2225), `.model-pill` 34 (2474), CtxRing 32 (CtxRing:65-66), `.cbtn.ic` 34 wide (2086). Bar gaps 3/2/5px (2035, 2049, 2050).

Z-order (low→high): textarea-wrap z1 → textarea z1 / ghost z0 / dict-ghost z2 / magic-text z2 / magic-stars z3 (see 1962-1971, 2004, 2399-2401, 2307, 2356) → drop-overlay z8 (1873) → in-shell popovers z10 → `composer-bar` z1 (2034). Floating, portaled: SettingsMenu `position:fixed z-index:9998` (SettingsMenu:510-515), PermMenu `--z-popover` (recipes.ts), GhPopover z950 (GhPopover:236). Halo layer tokens are `z-dropdown 20 / overlay 50 / toast 60 / tooltip 70` (HT:52-57) and say "never raw z-index" — Rift currently has 6 different raw values here.

Containment traps (all verified in source): `.composer-box{overflow:hidden}` (1929) clips anything inside the well; `.composer-bar{container-type:inline-size}` (2041) is only safe because both bar popovers portal to body (comment 2038-2040); `.composer-host{will-change:transform}` (AP:1160) plus the inline FLIP transform makes `.composer-host` a containing block for `position:fixed` descendants, so ANY Halo popover must portal to `<body>` (as SettingsMenu does via `use:portal`) rather than rely on `fixed` in place.

What the user sees at rest (from `docs/design/redesign-audit/A1-ledger-chat.jpeg`, observed this session): a single 1-line well (placeholder "Ask anything about this project", faint), below it a flat row: `∞ Bypass` (amber) | paperclip, mic | right: `● Fable 5.1 [High]`, ctx ring, send arrow. Controls are quiet text/icon, no surfaces until hover.

Narrow-window ladder (container queries 2055-2067): ≤440 hides `.pill-effort`, `.char-count`, pill-label→64px; ≤380 hides `.perm-label`, `.cbtn.reveal`(wand), pill-label→56px; ≤330 hides mic + attach (`.cbar-l .micbtn, .cbar-l .attachbtn`).

---

## 3 States

| State | Trigger / where | Visual today | Evidence |
|---|---|---|---|
| Hero (home, empty) | `hero` prop (`showEmpty && !engaged`, AP:809) | bigger well (r14), 14.5px text, placeholder "What are we working on today?", **control bar not rendered** | C:1624, 1594-1595, 1947-1951 |
| Engaged / docked | focus or dictation on home (AP:285-324), or any convo | bar mounts with `enter` (4px rise, 240ms) | C:2043; AP FLIP 359-374 |
| Idle | draft empty | placeholder IDLE_HINT, perm label quiet, chevrons hidden (opacity 0) | C:195, 1597, 2092 |
| Typing | draft | wand appears (`.reveal` `enter` 140ms) if Claude model; char count at >500 (warn >4000) | C:1730, 1743-1751, 2462 |
| Multiline / max | h>40 → `.multiline` flips row to flex-end; `atMaxHeight` → scrollbar | well grows to 340 then scrolls | C:216-219, 1933, 1985 |
| Focus | `:focus-within` on well | border → `--model-color 38%`, 1px ring tint; textarea itself `outline:0` | C:1938-1943, 1978 |
| Streaming, empty draft | `streaming && !hasDraft` | send btn → **Stop** (danger-soft); placeholder "Enter queues… /stop halts" (or "Rift asked a question…") | C:1335-1339, 1598-1601, 2245-2250 |
| Streaming + draft | | send → **Queue** (same Send glyph, `data-mode=queue`) + QueueRail appears | C:1337, 1421 |
| Streaming border | `.composer.streaming` | static 28% accent border; "stream owns liveness", no pulse | C:1956-1960 |
| ask_user pending | `askTarget` | Enter answers the card instead of queueing | C:694, 727-737 |
| Send disabled | `!canFire` (empty, or provider not ready) | opacity .55; `.ready` = accent fill when canFire | C:1340-1343, 2233-2243 |
| Provider locked / checking | `currentModelAccess` | model-pill amber border + amber dot; checking = dot pulse 1.2s | C:1762-1763, 2528-2534 |
| Drag-over | file drag depth>0 | dashed accent border, 4px glow ring, lift -1px, centered pill overlay | C:1861-1869, 1436-1443 |
| Enhancing | `enhancing` | textarea readonly + transparent text; shimmer clone, aura, 4 stars, wand pulses; EnhanceBar shows streamed words | C:1590, 2305-2391, EnhanceBar:142 |
| Enhance preview / error / undo-mini | `enhancedPreview`/`enhanceError`/`undoDraft` | panel above shell; undo pill auto-clears 12s | C:873-890, EnhanceBar:80-189 |
| Dictation: recording | `sttMine && stt.recording` | mic btn danger-soft + 5-bar level waveform; silent → idle breathing; auto-stop ring drains | C:1680-1727, 2114-2186 |
| Dictation: transcribing / polishing | `stt.transcribing` / `stt.polishing` | spinner (`mic-spin`); tail `dictate-polish` opacity pulse; text pulses | C:1702-1703, 2419, 2440-2446 |
| Dictation ghost | `dictGhost` | dim tail after committed text + blinking caret | C:1605-1609, 2398-2438 |
| mic error / other pane owns mic | | mic danger outline / disabled | C:1684, 1687, 2125 |
| Attach error / staged | | chips above input; red pill w/ dismiss | AttachmentsRow:28-72 |
| Queue present | `queue.length>0` | "Next up N" rail, expanded by default, rows draggable | QueueRail:98-151 |
| ctx ring tones | pct ≥75 warn, ≥90 danger | stroke colour + dashoffset tween `--dur-slow` | CtxRing:27, 78-83 |
| `usageOpen` | ring click | UsagePanel ctx card; closes on Esc/outside (ignores `.ctxring`) | C:1474-1476, UsagePanel:182-191 |
| ultra tier | effort id `xhigh` | pill border accent, sparkles glyph pulsing 2.6s | C:1761, 1782-1784, 2587-2599 |
| Reduced motion | `prefers-reduced-motion` | 10 `@media` blocks in C; every keyframe has a guard EXCEPT: `drop-in`/`drop-dot-pulse` (1875, 1895), `provider-pulse` (2533), `.char-count` `enter` (2209; 2463 only guards `.cbtn.reveal`), SettingsMenu `provider-spin` is covered by neither list (SettingsMenu:729 vs 962-969 omits it) | grep'd |
| Disabled | `.cbtn:disabled` | opacity .4 (Halo uses one `--opacity-disabled` .45) | C:2084 |
| Narrow / split pane | container <440/380/330 | see §2 ladder | C:2055-2067 |
| Hero + SettingsMenu open | | menu follows anchor rect every rAF so it survives the FLIP | SettingsMenu:68-81 |

Not present / unknown: a loading skeleton state (none); an offline state (only via provider access tags). Light theme: unknown whether composer is verified in light, Rift `app.css` theme list not read.

---

## 4 Motion present

Legend: KEEP = real activity, passes Halo "loops only for real state"; KILL = decorative/loop/blur-heavy against Halo 03-motion + owner brief (redesign-plan.md:9-12: no traveling shimmer, no specks, no stacked atmosphere); RETUNE = right idea, move to Halo token/curve. Halo curve `--ease-out cubic-bezier(.2,.8,.2,1)`, `--dur-fast 150 / base 300 / slow 600`, `--stagger 60` (HT:43-48); Rift curve `--ease-page .22,1,.36,1`, dur 140/240/520 (`app.css:240-250`). 7 raw `cubic-bezier(.22,1,.36,1)` literals + 7 raw `ms` values in C alone (grep).

| # | Where | What moves · property | Duration · easing | Trigger | Verdict |
|---|---|---|---|---|---|
| 1 | C:2043 `.composer-bar` `enter` | bar rises 4px + fade | 240ms, `.22,1,.36,1` | bar mount (hero→engaged / convo) | RETUNE → Halo `halo-rise`-lite / `dur-base` `ease-out`; keep (it is the "materialize" half of the FLIP) |
| 2 | AP:359-374 composer FLIP | `translateY` of host | 460ms `.22,1,.36,1` | engage/send on home | KEEP concept (Halo FLIP 460ms ease-out = same); retune curve only. Not in my files; host-owned |
| 3 | C:1926-1928 `.composer-box` | border-color, box-shadow, transform | 240/240/140ms | focus, drag, stream | RETUNE → `border-color/box-shadow dur-fast ease-out` (Halo input rule HALO-CSS:842) |
| 4 | C:1861-1869 drag-over | border dashed, ring, `translateY(-1px)` | via #3 | file drag | RETUNE → Halo FileDrop `.is-over`: accent border solid + 7% accent wash + `scale(1.01)` (HALO-CSS:670-671) |
| 5 | C:1875, 1897-1900 `drop-in` | overlay scale .94→1 + fade | 140ms | drag | RETUNE → `halo-pop-in` |
| 6 | C:1895, 1901-1904 `drop-dot-pulse` | dot scale 1→1.3 loop | 1.2s ∞ | while dragging | KILL (attention loop; not real activity) |
| 7 | C:1886, 1921, SlashMenu:143, SettingsMenu:520, EnhanceBar:203 `backdrop-filter` | blur 12/14/18/20/14px + saturate | static, repaint cost | always on well/menus | KILL ×5. Halo: "blur only on small areas"; DESIGN.md:297 already bans it for WebView2; slash already sets `none` in its override (SlashMenu:354) |
| 8 | C:2003, 2009-2012 `placeholder-fade` | blur(3px)+8px rise | 700ms | **dead**: every placeholder uses `.static` (2006) | KILL (dead code) |
| 9 | C:2080-2085 `.cbtn` | bg/color/border, `scale(.96)` press | 140ms | hover/press | RETUNE → Halo button: `dur-fast ease-out`, press `scale(.97)` (HALO-CSS:54) |
| 10 | C:2092, 2108, 2536-2541 chevrons | opacity 0→1, rotate 180° | 140ms | hover/open | KEEP/RETUNE → Halo menu chevron rotates `dur-base` (HALO-CSS:416-417) |
| 11 | C:2130-2131 `mic-spin` | rotate loop | .9s linear ∞ | busy | KEEP → Halo `Spinner` (900ms, HALO-CSS:448) |
| 12 | C:2148-2150 waveform bars | height by live `--lvl` | 80ms ease-out | mic level | KEEP (real signal). Halo has no equivalent |
| 13 | C:2158-2168 `mic-idle` | bar breathe 22→38% ∞ | `--pulse-live` 1.6s | recording & silent | KEEP (real activity) |
| 14 | C:2180-2187 `mic-ring` | stroke-dashoffset | 260ms linear | silence countdown | KEEP (real countdown). Geometry hard-coded `rx=10`, `viewBox 38×36` (1720-1722) |
| 15 | C:2209 / 2462 `enter` on char-count + wand | 4px rise | 140ms | appear | KEEP/RETUNE |
| 16 | C:2228-2233 `.send-btn` | colours, `scale(.9)` press, `translateY(-1px)` hover | 140ms | hover/press | RETUNE → Halo primary: hover glow (`--glow-strong`), press .97, NO lift (Halo lift is 3px only on cards) |
| 17 | C:2254-2270 `send-ripple` ×2 | two rings scale .6→2.2, fade | 620ms, 90ms stagger | every send | RETUNE → one `halo-ping`-style single ring, or drop to the Halo Button feedback; two stacked rings is the "stacked atmosphere" the brief rejects. Owner call (Q6) |
| 18 | C:2279-2290 `.icon-slot` | send/stop glyph swap: scale .6→1 + rotate -30°→0 | 140/240ms `.22,1,.36,1` | mode change | KEEP (small, state-flip, matches "things you flip" → could use `ease-spring`) |
| 19 | C:2306-2338 `magic-shimmer` | gradient sweeps across draft text (background-position) | 1.15s linear ∞ | enhancing | KILL: traveling shimmer (DESIGN.md §8 + brief). Replace with static dim text + Halo `Spinner`/LiveDot "Enhancing…" |
| 20 | C:2339-2355 `magic-aura` | radial wash opacity .35↔.85 | 1.6s ∞ | enhancing | KILL (atmosphere) |
| 21 | C:2357-2375 `magic-twinkle` ×4 stars | scale/rotate/opacity, clip-path sparkle, drop-shadow glow | 1.6s ∞ | enhancing | KILL (specks; also `filter: drop-shadow` ×4 continuous) |
| 22 | C:2379-2388 `wand-pulse` | glow + scale 1.08 loop | 1.3s ∞ | enhancing | RETUNE → Halo `Spinner` inside the button (real wait of seconds) |
| 23 | C:2419, 2441-2446 `dictate-polish` | opacity 1↔.55 | 1.2s ∞ | polishing | KEEP (real activity; opacity only) |
| 24 | C:2422-2435 `dict-caret` | caret blink `steps(2)` | 1s ∞ | dictating | KEEP (real) |
| 25 | C:2533-2534 `provider-pulse` | dot opacity | 1.2s ∞ | checking | KEEP → Halo `LiveDot tone="info"` |
| 26 | C:2587-2599 `ultra-pulse` + drop-shadow | sparkles opacity 1↔.62 | 2.6s ∞ | tier = xhigh | KILL (decorative loop; static glyph is enough) |
| 27 | EnhanceBar:213-218 / SlashMenu:152 / MentionPopover:56 / UsagePanel:286 `slash-in`/`usage-in` | popover rise 4–6px (+.985 scale in slash) | 140ms | open | RETUNE ×4 → ONE Halo `halo-pop-in-up` (scale .96 + 4px, `dur-fast ease-out`, HALO-CSS:411-413), origin bottom-left. Same keyframe is declared 3× with 3 different offsets today |
| 28 | MentionPopover:63-68 `slash-item-in` (all rows together) | row 6px rise | 140ms | open | RETUNE → Halo `halo-menu-item` 220ms, 18ms stagger capped at 12 (HALO-CSS:419-420). SlashMenu already killed its stagger (SlashMenu:405) |
| 29 | SlashMenu:259-269, 413-421 `.sm-bar` | accent bar height 0→48% | 80ms | active row | RETUNE/KILL → Halo uses `bg-200` row lift only; bar optional |
| 30 | EnhanceBar:386-399 `word-materialize` | each word blurs 7px→0 + fade, stagger 14ms cap 650ms | 420ms | streamed text | RETUNE: Halo `halo-reveal` (6px blur+6px rise, HALO-CSS:104) is the same idea; keep but cap blur at 6px, keep `.live` no-delay. Blur on a ≤220px text region = "small area" OK |
| 31 | EnhanceBar:301-310 `status-pulse` | title opacity | 1.4s ∞ | grounded lookup | KEEP (real) |
| 32 | QueueRail:99,115,125-126 svelte `fly/slide` | rail y7/170ms; list slide 160ms; rows y5 stagger 20ms/150ms | quintOut | queue change | KEEP/RETUNE → Halo `FlipList` glide 460ms for reorder (rows reorder live on dragover today with no glide, QueueRail:84-94); entrance → `halo-rise` stagger |
| 33 | QueueRail:174-175 chevron | rotate | 140ms | toggle | KEEP |
| 34 | CtxRing:78-81 | stroke-dashoffset + stroke colour | 520ms `--ease-soft` | ctx change | KEEP → matches Halo Ring "arc sweeps from old value" |
| 35 | UsagePanel:308-313 `up-fill`/`up-grow`/`up-pulse` | bar width reveal 480ms; **hot zone brightness pulse 2.2s ∞** | | open / ≥90% | width reveal KEEP (Halo Progress eases); `up-pulse` KILL (attention loop on a number) |
| 36 | UsagePanel:347-348 `up-spin` | refresh icon | .9s | refreshing | KEEP |
| 37 | SettingsMenu:526 `pop-in` (app.css:640) | 8px rise + scale .96 | 240ms `--ease-page`, origin bottom-right | open | RETUNE → `halo-pop-in-up` `dur-fast` |
| 38 | SettingsMenu:589-594 `flyout-open` | max-height 0→200 + 4px | 460ms (`--dur-rise`) | Other models | RETUNE (max-height animation = layout; use Halo accordion pattern) |
| 39 | SettingsMenu:578-580, 735-742 | chevron rotate 240ms; check pops scale .4→1 rotate | | pick | RETUNE → Halo check pop = `halo-pop` scale 1.18 `ease-spring` (HALO-CSS:160) |
| 40 | SettingsMenu:772-789 fast switch | knob translate 240ms | | toggle | RETUNE → Halo `Toggle` (spring, stretches on press) |
| 41 | SettingsMenu:866-943 effort slider | fill width spring (`--ease-spring`), knob left spring, drag free-follow, release spring-snap, hover scale 1.05, drag 1.12 | `--dur-base` | drag/pick | KEEP the **behaviour** (owner call 2026-07-22/23, comments 205-209, 857-861); retoken to Halo `Slider` look/spring. Real gesture feedback |
| 42 | SettingsMenu:729 `provider-spin` | loader | .9s ∞ | checking | KEEP → `Spinner` |
| 43 | PermMenu (bits) | `floatingMotion` = fade+zoom95+slide from side, 140ms `ease-page` (`recipes.ts` floatingMotion) | | open | RETUNE → same Halo pop-in; recipes.ts is the single place to retune |
| 44 | AttachmentsRow / chips | none (no enter, no exit) | | add/remove | GAP → Halo `tag-in` (scale .6→1, `ease-spring`, HALO-CSS:621) for chips; remove = fade |
| 45 | C:1493 `ps-chip` | colour only | 140ms | hover | KEEP |

Net: of ~45 motion sites, KILL ≈ 11 (all the enhance "magic", ultra pulse, drop dot, dead placeholder fade, up-pulse, 5 backdrop-blurs counted as one row), RETUNE ≈ 20 (mostly curve/duration/keyframe consolidation), KEEP ≈ 14 (every state-bound loop). Loop inventory vs Halo "loops only for real activity": real = mic-spin, mic-idle, dictate-polish, dict-caret, provider-pulse/spin, status-pulse, up-spin; decorative = drop-dot, magic ×4, wand-pulse, ultra-pulse, up-pulse.

Reduced-motion coverage gaps are in §3 (row "Reduced motion").

---

## 5 Styling wiring

- **Mechanism:** 100% scoped `<style>` in each component + `:global()` for portaled/child-targeted selectors. **No Tailwind in composer markup** except `PermMenu.svelte` (bits classes: `data-[tone=ok]:data-[state=checked]:not-data-highlighted:bg-ok-soft` strings, PermMenu:41-61) and the `ui/` primitives. App-level shared CSS used: `.rift-menu`, `.rift-menu-row`, `.rift-menu-divider` (`app.css:695-736`), `@keyframes enter` / `pop-in` (`app.css:634-643`), `.tile*`, `.ctx-chip`.
- **Tokens used (Rift names):** `--surface`, `--surface-hover`, `--bg-elev-1/2/3`, `--bg-inset`, `--border`, `--border-strong`, `--fg`, `--fg-2`, `--fg-muted`, `--fg-subtle`, `--fg-faint`, `--accent`, `--accent-soft`, `--accent-hover`, `--accent-fg`, `--ok/warn/danger/info` (+`-soft`), `--ring`, `--model-color` (local alias of `--accent`, C:1857, used at ~25 sites), `--dur-fast/base/slow/rise`, `--ease-page/soft/spring`, `--pulse-live`, `--fs-xs/sm/md`, `--radius-xs/sm/(none)`, `--font-mono`, `--chat-col-max`, `--accent-h` (UsagePanel gradient, :309), `--shadow-lg`.
- **Per-file counts (grep -o this session):**

| File | `var(--` uses | `color-mix(` | `oklch(` literal | hex | rgb/hsl | raw `Npx` | `@keyframes` | `prefers-reduced-motion` | `use:tooltip` |
|---|---|---|---|---|---|---|---|---|---|
| Composer | 165 | 47 | 2 (2238, 2323) | 0 | 0 | 161 | 14 | 10 | 10 |
| SettingsMenu | 154 | 46 | 15 | 0 | 0 | 174 | 3 | 1 | 7 |
| SlashMenu | 98 | 34 | 2 | 0 | 0 | 121 | 2 | 1 | 0 |
| EnhanceBar | 68 | 22 | 1 | 0 | 0 | 67 | 3 | 1 | 8 |
| UsagePanel | 53 | 9 | 2 | 0 | 0 | 54 | 4 | 1 | 1 |
| QueueRail | 37 | 9 | 0 | 0 | 0 | 47 | 0 | 0 | 4 |
| AttachmentsRow | 21 | 2 | 0 | 0 | 0 | 30 | 0 | 0 | 2 |
| MentionPopover | 11 | 1 | 0 | 0 | 0 | 15 | 2 | 1 | 0 |
| CtxRing | 11 | 2 | 0 | 0 | 0 | 5 | 0 | 0 | 1 |
| PermMenu | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| (adjacent) PermissionBar | 29 | 6 | 0 | 0 | 0 | 9 | 2 | 1 | 0 |
| (adjacent) GhPopover | 30 | 0 | 0 | 0 | 0 | 27 | 2 | 1 | 0 |
| (adjacent) AskUserCard | 68 | 20 | 0 | 0 | 0 | 53 | 1 | 0 | 0 |

  Composer-set `oklch(` literals = 22 of the `check:ui` baseline 131 (`design-system/ui-baseline.json`). Tailwind arbitrary values: 0 in all of these.
- **Spread:** distinct `border-radius` forms across the composer set = 17 (999px ×26, 8px ×9, 6px ×9, 50% ×8, `--radius-sm` ×5, 7px ×5, 10px ×5, 5px ×4, 12px ×4, 9px ×2, 4px ×2, 14px ×2, `--radius-xs`, `--radius`, 16px, `0 2px 2px 0`, `0`). Distinct `font-size` forms ≈ 18 (10px ×20, `--fs-sm` ×14, `--fs-xs` ×13, 9px ×10, 11px ×9, `--fs-md` ×6, 10.5px ×6, 9.5px ×5, 12px ×5, 11.5px ×5, 14.5px ×3, 13px ×2, 15px, 12.5px, plus SettingsMenu-local `--sm-eyebrow/title/sub/meta/mono` 9.5/12.5/11/10/10px, SettingsMenu:531-537). Halo has a fixed type ladder (display/title/subtitle/heading/body 15/small 13/caption 12/label 11 mono).
- **Duplication:** 3 popover chromes coexist: (a) bits `DropdownMenu` via `ui/recipes.ts` (PermMenu), (b) `.rift-menu` global (Slash/Mention/Usage/GhPopover, `app.css:695-704`, then overridden to near-opaque), (c) SettingsMenu's own `:global(.rift-menu.settings-menu)` (blur 20px, 276px). Keyframe `slash-in` declared 3× (SlashMenu:152, MentionPopover:56, EnhanceBar:215) with different distances. Two copies of the "Glanceable pill marker" comment (C:2585-2586).
- **Stale/dead CSS (evidence):** `.placeholder-ghost` non-static animation (C:2003) never fires (all uses `.static`); `.cbtn.ic.active` (2088) has no `class:active` producer; SettingsMenu `.ac-glyph/.ac-switch/.ac-dict*/.ac-chip*` (791-815) and `.model-row.soon` (732-734) have no markup in the template (read 273-500); `.pop-item.rich` is set in markup (426) but has no CSS; SlashMenu layer-1 rules 133-346 are mostly overridden by 348-453 (e.g. `.sm-item animation: none`, `.slash-menu background:var(--surface)`).
- **Token collisions with Halo (names equal, values different):** `--dur-fast` 140→150, `--dur-base` 240→300, `--dur-slow` 520→600 (`app.css:247-249` vs HT:43-45); `--radius-sm` 6→8 (`app.css:201` vs HT:37), Halo adds `--radius-md 12`, `--radius-lg 16` where Rift's `--radius` is 8 and `--radius-lg` is 10 (`app.css:202-203`); `--ease-spring` identical (`.34,1.56,.64,1`); `--accent/--warn/--danger/--info` and the whole `*-soft` family (`--accent-soft` etc., `app.css:178` oklch vs HT:15-24 rgba) are redefined with different values; `--font-mono` = JetBrains Mono in Rift (`package.json` deps) vs Geist Mono in Halo (HT:61) — matters for the ghost-mirror pixel match (§9 Fonts). `--ring` is Rift-only (Halo has `--shadow-focus`, no `--ring`) → needs an alias decision, not a collision. `--surface`, `--bg-elev-*`, `--fg-*`, `--border*` are Rift-only names with no Halo twin. **No `--ok` in Halo** (brand book: positive = accent on accent-soft, `01-themes-and-colour.md`): Rift's Auto-edit perm tone (C:2100, 2105; PermMenu `tone-ok`), PermissionBar Allow (PermissionBar:122-130) and the ctx/usage "ok" zone all paint `--ok` green → under Halo either they become accent-blue (Auto-edit and Send then share one colour) or a Rift-local green stays outside the "one signal colour" rule (Q16). Dropping HT into `app.css` retunes every existing transition in the app silently. **Known-good bridge:** ConsoleX loads Halo `tokens.css` FIRST and keeps its old names as aliases (`consolex/panel/src/app.css:1-40`, e.g. `--panel: var(--bg-100)`, `--text: var(--ink)`). That is the pattern to copy.
- **Blue preset hazard:** in `data-theme="blue"` Halo swaps `--info` to lime `#c6f35a` (HT:64-66). Rift paints "Plan mode" tone with `--info` (C:2101, 2107; PermMenu `info` tone). Plan mode would turn lime on Electric blue.
- **Shadow/focus:** Rift focus = `0 0 0 2px var(--ring)` on controls (C:2085), 3px on send (2234), accent 2px outline on CtxRing (CtxRing:76), well uses a tinted border + 1px ring (1938-1943). Halo = one `--shadow-focus` (2px gap + solid 2px accent) on every focused control (HALO-CSS:45). Halo's gap colour is the surface colour baked into the token (`#0e1014`, HT:42/73).

---

## 6 Restyle boundary

### LOGIC to keep verbatim (state, handlers, IPC, stores)

- **Per-tab binding** (C:86-123): `tab`, `draft`, `attachments`, `queue`, `streaming`, `paneEffectiveModel`, `paneChatGptRoute`, ctx getters. Reset-on-tab-change effect (C:272-301). Pre-warm effect (311-323). `wsRoot`/GitHub refresh effects (99-105).
- **Send path** `fire()` (696-758): ghost-tail fold, autocorrect, ChatGPT slash gate, ask_user reroute, auth gate (`notify.danger` + refresh), `setDraft("")`, `stt.consume()`, `fireKey++`, `onsubmit`. `onBtnClick`/`mode`/`canFire` (1321-1343).
- **Key ownership `onKey` (1142-1319):** Shift+Tab perm cycle, Esc layers (enhance → undo → mention → settings → slash), Ctrl/Cmd+E enhance, Ctrl+D tap/hold, hold-Space PTT, mention nav, ↑/↓ prompt recall, settings nav (↑↓, ←→ effort, digits 1-9, Enter/Tab, any char closes), slash nav, Enter send, Shift+Enter newline. **Constraint for the redesign:** the settings menu is a *virtual listbox driven from the textarea* (`settingsIdx`, `activeSettingsRow`, `onmousedown preventDefault` to keep focus, SettingsMenu:273-287, 301-309, 340). A Halo `Menu`/`Select` that moves focus into its own list or owns ↑↓/typeahead will break this and will also trip the hero disengage (AP:304-324, which special-cases `[data-composer-popover]` at AP:316 for the bits perm menu only). Swap markup/CSS, keep `settingsRows`, `settingsIdx`, `onKey`, `data-composer-popover`.
- **Autosize** (197-224) incl. the measurement hacks (wrap freeze, `alignSelf`), `multiline`, `atMaxHeight`, ghost scroll sync.
- **Slash** catalog/lanes/filter (135-183, 375-496), `pickSlash`, `fillSlash`, `handleChatGptSlash` (602-643). **Mention** detect/pick (245-364).
- **Enhance state machine** (760-921, enhanceSeq token, request id, undo timer, ground toggle in localStorage `rift.enhanceGround`, C:782-789 — no try/catch; read at component init (instance script), see §8 #16).
- **Dictation** (923-1098): PTT timers, Ctrl+D classification, window blur/keyup, `stt.*` calls, voice "send it" effect (1033-1042), `micBusy`, `silenceFrac`, `dictGhost`.
- **Attach** paste/drop/pick/`stageFiles`, `dragDepth` + document `dragend` reset (1099-1415).
- **Model/effort/perm:** `composerModels`, `providerAccess`, `modelAccessFor`, `effortStops`, `setEffortByIdx`, clamp effect (541-547), `pickModel`, `pickMode`, `cyclePerm`, `settingsRowsFor` (modelMatrix). SettingsMenu's `alignLiveEffort`, `providerView` cursor-crossing logic (143-158), `modelLimitFor` (255-263), pointer-capture slider math (209-233).
- **IPC/store calls inside:** `invoke("spell_check_words")` (41), `assistant.selectModel/setPermissionMode/setThinkingDial/setFastMode/enhancePrompt/cancelEnhance/submitAskUserAnswer/stop/newChatWithModel`, `usage.refreshRateLimits`, `assistant.send("/compact")`, `stt.*`, `github.maybeRefresh`.
- **Hero/FLIP host contract (AP):** `hero` prop semantic, `composerSlotEl` ref, `onfocusin/out`, `ResizeObserver → composerH` (used for jump-latest offset, AP:819). Composer height must stay measurable and stable across hero↔docked.
- **Hooks that MUST survive markup rewrite:** `class="composer-wrap"` (used by `scripts/cdp/c.sh shot-sel ".composer-wrap"` and README, `scripts/cdp/README.md:133-134`, `c.sh:28`), `.ctxring` (UsagePanel `ignoreSel` default, UsagePanel:15), `data-composer-popover` (AP:316), `[data-qedit]` (QueueRail:32), `id="model-effort-menu"` (aria-controls, C:1769).

### MARKUP/CSS to change (everything else)

All `<style>` blocks; button anatomy (`.cbtn`, `.send-btn`, `.model-pill`), well chrome, popover chrome ×3, pill/effort chips, drag overlay, enhance "magic" overlay, queue rail chrome, attachment chips, CtxRing chrome, UsagePanel chrome/bars, SettingsMenu rows/tabs/switch/slider visuals, placeholder-ghost, keyboard-hint footers (`<kbd class="sm-kbd">`, SlashMenu:112,124-126 → Halo `Kbd`).

---

## 7 Halo mapping table

Halo specifics cited: `Textarea` = bg-100 ground, `line-300` border, focus accent border + `shadow-focus`, auto-grow to `maxHeight` 360 (README; HALO-CSS:585-595). `Menu` = `halo-pop` (radius-md, bg-100, line-200, shadow-pop, scale-in from corner), items min-h 40, active = bg-200 + accent icon, stagger 18ms cap 12 (HALO-CSS:406-430). `Popover` = same shell, padding space-4. `Button` 44/36px, radius-md; primary hover = `--glow-strong`. `Kbd` = 22px mono keycap bg-200. `Ring` = card-size metric with centred % (HALO-CSS:221-226). `Progress` 8px/4px track, eases, indeterminate sweep.

| # | Rift element | Halo component / token | Notes · gap flag |
|---|---|---|---|
| 1 | Well + textarea (`.composer-box`, `textarea`) | `Textarea` surface recipe (`bg-100`, `line-300→accent` focus, `shadow-focus`, radius-lg/md) | Halo Textarea is a labelled form field with hint/counter; composer needs the *recipe* not the component: no label, `rows=1`, 340px cap (Halo default 360), custom ghost/dictation/enhance overlays. Owner's rough mock (`Main.dc.html:126-138`) puts the control row INSIDE the card; Rift v4 deliberately put it BELOW (C:1906-1909, 2028-2031) → Q1 |
| 2 | Placeholder ghost | Halo placeholder = `ink-faint` native `::placeholder` | Rift fakes placeholder with a span (needed for hero vs convo variants, `Enter`/`/stop` kbd styling). Halo `Kbd` for `.ph-k` (C:2007). DESIGN.md:154-157 "one quiet phrase" rule still applies |
| 3 | Send / Stop / Queue | `Button` primary `sm` icon-only (mock uses exactly this, `Main.dc.html:137`) | Halo = radius-md square, Rift = 34px circle (999px). DESIGN.md:147 bans pill buttons — consistent with Halo. Three modes + glyph swap + ripple are Rift-specific. **Stop** = Halo `danger` variant (transparent+danger border) or soft-danger — Halo has no "soft danger" button variant (gap) |
| 4 | `.cbtn` flat controls (attach, mic, wand) | `Button variant="ghost" size="sm"` | Halo sm = 36px (touch ≥44 rule); Rift = 34. Flag density Q3. Halo `ghost` hover = `bg-200` + `ink` — matches Rift's `surface-hover` |
| 5 | Mic (recording/transcribing/error) + waveform + silence ring | `Button ghost` + `Spinner` for transcribing; **waveform + countdown ring = Rift-specific part** | Halo has no level meter. Use `danger-soft`/`danger` tokens; `LiveDot` only if a dot is wanted. `mic-ring` rect must be re-derived from new button radius (hard-coded `rx=10`) |
| 6 | Perm pill + menu (`PermMenu`) | `Menu` (`triggerVariant ghost`, `size sm`, `side top`, `chevron`) — the mock already does this (`Main.dc.html:133`) | **Gap:** Halo `MenuItem` = `{label, icon?, hint?, onSelect, danger?, disabled?}` (`index.d.ts:474`); no radio/checked state, no per-row tone (ok/warn/info), no checkmark. Rift needs checked semantics (bits `RadioItem`). Keep bits menu, restyle via `recipes.ts` to Halo `halo-pop` look. Perm tone glyphs map: ok→`accent`, warn→`warn`, info→`info` (**lime on blue preset!**) |
| 7 | Model pill | `Button ghost sm` / `Menu` trigger w/ chevron | `LiveDot` for the leading dot (provider ready/checking/locked → accent/info/warn; "idle" no pulse). Effort micro-chip → `Badge neutral` (caption). Keep amber for locked state. `ultra` marker → static glyph/`Badge accent` |
| 8 | SettingsMenu (model list + provider tabs + flyout + Fast + effort slider + caption) | Shell = `Popover` (width ~280, `side top`, `align end`); tabs = `Segmented` (thumb slides `dur-base`); rows = `Menu` item recipe; Fast = `Toggle`; effort = `Slider` | **Gap/compose:** Halo `Menu` cannot host tabs/toggle/slider; this is a `Popover` composed of `Segmented`+list+`Toggle`+`Slider`. Halo `Slider` is a native range with value readout; Rift's is a 4-detent snapping control with free-follow + spring release (owner-specified) → keep Rift slider logic, take Halo slider *visuals/spring*. Digit hotkey badge → `Kbd` |
| 9 | SlashMenu | `CommandPalette` pattern / `Menu` rows + `Kbd` + `SectionLabel` group headers (`halo-menu__heading`) | Halo CommandPalette is a modal dialog near top of window — wrong placement; use `Menu` list recipe anchored above composer. Highlighted-match `<b>` ↔ Halo Combobox match = `ink` + accent underline (HALO-CSS combobox). Source badges → `Badge`. Footer hints → `Kbd` |
| 10 | MentionPopover | `Menu` list / `Combobox` listbox recipe | mono path rows; "N files" footer |
| 11 | EnhanceBar | `Popover` shell + `Button` primary/ghost + `Toggle`-like chip (`Segmented`/Badge pressed) + `TextField` (steer) + `Spinner` | **Rift-specific:** word-by-word reveal, diff view (reuses `EditDiff`), undo-mini pill. Halo `Banner`? No — transient panel. Accent colour for the header glyph currently `--model-color` |
| 12 | Drag overlay + drag-over | `FileDrop` `.is-over` recipe (accent border, 7% accent wash, icon lift on `ease-spring`) | Halo FileDrop is a standalone dropzone + file list w/ `Progress`; composer needs only the overlay *look*. Copy says "Drop image" but non-image text files are also accepted (`stageFiles` C:1381-1393) → copy bug (§8) |
| 13 | Attachment chips | `TagInput` chip recipe (pop `ease-spring`, × button) / `FileDrop` file row | Thumbnails (40px image) have no Halo equivalent — Rift-specific. Error pill → `Banner danger` compact or `Badge danger` |
| 14 | QueueRail | Rift-specific list; rows → `FlipList` for glide on reorder; header → `SectionLabel` + `Badge` count; Clear all → `Button ghost sm danger`-on-hover | Inline edit input → `TextField sm` |
| 15 | CtxRing | `Ring` (arc sweep from previous value) | **Gap:** Halo Ring is card-scale with centred % label and `size/stroke` props; Rift's is a 16px icon-only gauge inside a 32px hit area. Keep geometry; retoken (`bg-300` track, tone → `accent/warn/danger`) and keep the 520ms dashoffset ease (matches Halo "arc sweeps") |
| 16 | UsagePanel (ctx + full) | `Popover` + `Progress` (`size` default, tone accent/warn/danger) ; `RollingNumber` for % | `LimitBar` (shared) also needs this mapping (other wave). Drop `up-pulse` |
| 17 | `.ps-chip` suggestion | `Badge neutral` w/ dashed border → Halo has no dashed chip; use `Button ghost sm` + sparkles | |
| 18 | Char counter | `Textarea` `maxLength` counter style: `ink-faint` → `warn` within 10% → `danger` full (HALO-CSS:593-595) | Rift thresholds are 500/4000 absolute, no hard limit → keep logic, take colour ladder |
| 19 | Tooltips (`use:tooltip`) | `Tooltip` (120ms delay, rises 4px, inverted bubble) | **Policy conflict:** Rift's tooltip is a deliberate **no-op** since 2026-06-15 (`actions/tooltip.ts:1-8`; promotes text to `aria-label` only). 34 call sites in the composer set carry the ONLY hint for icon buttons + 4 `kbd:` hints (C:1642, 1822-1825). Halo ships Tooltip → Q2 |
| 20 | Kbd hints (`.sm-kbd`, `.ph-k`, `.model-num`, `.qnum`) | `Kbd` | Halo rule "show only shortcuts the app really binds". Check: send-button tooltip claims `Esc` halts the turn (C:1822) but no Escape→stop route exists: every `assistant.stop(` caller (grep, all of `src`) is a click path — Composer:1325 (button), `stream/ActivityHud.svelte:151` (mouse handler), `shell/CloseConfirm.svelte:55` — and Composer `onKey` only uses Esc to close menus/enhance → claim is false as shipped (`/stop` slash text is separate); drop or implement |
| 21 | Dividers / hairlines (`.cbar-sep`, `.sm-group i`, menu dividers) | `Divider` / `line-100` | |
| 22 | Z-layers (9998, 950, 10, 8) | `--z-dropdown`, `--z-overlay`, `--z-tooltip` | Rift has `--z-popover`/`--z-dialog` in `app.css` (value not read this session; DESIGN.md:296 says menu inside dialog works, tooltips/splash 9999) → reconcile at token-bridge time |
| 23 | Hero placeholder, hero well size | Halo `Textarea` + `display` type | Hero is a Rift posture; Halo has no "hero input". Keep |
| 24 | Provider status dots (SettingsMenu tabs `i.state-*`) | `LiveDot` tones / `Badge` | ready=accent, checking=info, setup=warn, error=danger. Blue preset: `info` is lime (collision) |

Gaps needing Rift-specific parts (summary): waveform meter, dictation ghost mirror, enhance preview (stream/diff/undo), queue drag-reorder + inline edit, hero↔docked FLIP, effort detent slider logic, radio/checked/tone menu rows, 16px ring gauge, image-thumb chips, soft-danger stop button.

---

## 8 Pain points (evidence)

1. **Monolith.** C = 2604 lines: 1416 script, 760 CSS, 14 keyframes, 10 reduced-motion blocks; SettingsMenu 970 with 174 raw px. Hard to restyle safely; all visual rules are interleaved with ~40 `$state`s.
2. **Three popover languages** (bits/Tailwind, `.rift-menu`, SettingsMenu custom) with 3 duplicate `slash-in` keyframes and 5 `backdrop-filter` blurs (C:1886,1921; SlashMenu:143; SettingsMenu:520; EnhanceBar:203) in a WebView2 app whose own rules say no blur (DESIGN.md:297).
3. **Dead / layered CSS** (see §5): SlashMenu is effectively styled twice; dead `ac-*`, `.soon`, `.active`, `placeholder-fade`.
4. **Type + radius sprawl:** 17 radii, ~18 font sizes (9px–14.5px) in one surface; 34px controls under Halo's 36/44.
5. **Hard-coded geometry coupling** (restyle landmines): `mic-ring` `rx=10`/viewBox 38×36 (C:1720-1722); `magic-aura` radius 12 (2344) = well radius; `.dict-ghost` padding must equal textarea padding "to the pixel" (2401-2405) and hero re-states it (2414); **`magic-text` padding is `8px 10px 6px` (2310) vs textarea `9px 12px` (1976)** → on enhance start the text jumps ~1-2px and wraps differently (hero is worse: 11/12/11/14). Evidence is in-source; not visually verified.
6. **Hard-coded colours/curves:** send-ready text `oklch(0.16 0.01 250)` (C:2238) instead of `--accent-fg` (EnhanceBar uses `--accent-fg`, EnhanceBar:249); shimmer highlight `oklch(0.98 0.02 250)` (2323); 22 `oklch` literals total; 7 raw cubic-beziers.
7. **Shortcuts are undiscoverable.** Tooltip no-op (`tooltip.ts:1-8`) means ⇧Tab, Ctrl+E, Ctrl+D, hold-Space, Enter/⇧Enter hints only exist as `aria-label` text; and the send tooltip advertises `Esc` for Stop with no handler found (§7 #20).
8. **Chevrons hidden until hover** (C:2092, 2537) → pickers look like labels at rest; owner already noted reading effort chip as uneven (C:2505-2507).
9. **Decorative loops contradict the brief.** Enhance "magic" = 4 simultaneous infinite animations (shimmer, aura, 4 stars, wand) on a task that takes seconds; `ultra-pulse`, `drop-dot-pulse`, `up-pulse` loops. DESIGN.md:246 already records the owner rejecting a composer comet arc as "disturbing".
10. **Perf:** SettingsMenu runs a `requestAnimationFrame` follow loop for as long as it is open (SettingsMenu:74-81) (no-ops state but still reads `getBoundingClientRect` each frame, forcing layout); `autosize()` runs from two effects (draft + dictGhost, C:226-239, 1065-1069) and does a wrap-freeze / height:auto / re-measure sequence per keystroke (197-224) = forced sync layout each time; `backdrop-filter` blur on a surface that sits over a scrolling stream; `filter: drop-shadow` on twinkling stars.
11. **Copy accuracy:** drag overlay says "Drop image to attach" (C:1440) while text/code files are accepted (`stageFiles` 1381-1393) and the paperclip tooltip says "file or image" (1674).
12. **Reduced-motion holes** (§3).
13. **Pill shapes against house rule:** send button 999px, `.ps-chip` 999px, `.char-count` 999px, `.enhance-toggle` 999px, `.rail-count` 999px — DESIGN.md:147 says pills for badges/toggles only (send btn is a button). Halo permits `radius-pill` for chips/pills, not buttons (README: "chips and pills radius-pill").
14. **Idle chrome.** Control bar is not rendered in hero (C:1624) so the home composer has no model/perm visibility until engaged — layout shift of the whole deck on focus (hidden by the FLIP).
15. **Column alignment** was an audit finding pre-Halo (`redesign-plan.md:53`: composer 880 vs messages 1100); composer still `min(var(--chat-col-max), 880px)` (C:1849). Whether the ledger branch fixed the stream side is UNKNOWN (not in my scope's files).
16. **`localStorage` access unguarded** at component init (C:782-788) — platform rule is try/catch; WebView2 normally fine, unverified failure mode.
17. **Ctx% wording:** `UsagePanel` ctx mode and `CtxRing` use independent derivations of tone thresholds (UsagePanel:42-44 `ok/warn/hot` vs CtxRing:27 `ok/warn/danger`) → same numbers, two vocabularies.

---

## 9 Risks

- **Token collision (headline).** Same variable names, different values (§5). Importing Halo tokens verbatim retunes ALL existing timings/radii app-wide, not just the composer. Adopt the ConsoleX bridge (Halo first, Rift names aliased) and decide per-name; do it as a deliberate global step, not inside this wave.
- **Blue-preset `--info` = lime**: Plan-mode glyph/wash (C:2101/2107, PermMenu info tone) and any "info" use (provider checking dot, session note) change colour. Needs an alias or a Rift-local `--info`.
- **`npm run check` = svelte-check + `check:tokens` + `check:ui`.** `check:ui` baselines (`design-system/ui-baseline.json`): `overlay:use-portal 11`, `overlay:aria-role 17`, `color:hex 21`, `color:rgb-hsl 16`, `color:oklch-literal 131`, `tailwind:arbitrary-value 2`; each may only GO DOWN (check-ui.mjs:169-173). Composer set contributes: portal ×2 (SettingsMenu, GhPopover), `role=menu|dialog` ×5 (SlashMenu, MentionPopover, UsagePanel, SettingsMenu, GhPopover) outside `components/ui/`, `oklch` ×22. A hand-rolled Halo Menu/Popover port in `assistant/composer/` that re-adds `role="menu"`/`use:portal` raises the counts and FAILS; build it under `components/ui/` (exempt) and consume it. Raw hex/rgba is scanned in `.svelte` only (`check-ui.mjs:108`): Halo's hex/rgba tokens are safe in `app.css`, fatal if pasted into a component. `ui/` must stay at zero raw colours/arbitrary values (uiDirty, :122-129). Also every class in `ui/` must generate CSS (Tailwind bridge) or hard fail.
- **Tests are logic-only → markup/class churn is test-safe.** `vitest` `environment:"node"` (vitest.config), no testing-library/mount found; composer-touching tests: `composer/helpers.test.ts`, `composer/modelMatrix.test.ts` (`scripts/verify-openai.ps1:11`), `state/assistant.playback.test.ts` (queue/attachment state, not DOM). No test references composer class names. **Therefore regressions in this wave are caught by nothing automatic** — only `svelte-check` types + CDP/visual checks. Plan visual verification via `scripts/cdp/c.sh` (`.composer-wrap` selector, `rift-ui` skill, owner's rules say CDP bridge, not stealing focus).
- **CDP/skills selectors:** `.composer-wrap` is referenced in `scripts/cdp/README.md:133-134` and `c.sh:28`; keep the class or update docs. `.agents/skills/rift-ui` had no composer selector match (grep).
- **A11y.** Keep: `role=region` + `aria-label` on shell (C:1428-1430), `aria-haspopup/expanded/controls` on model pill (1767-1769), `role=menuitemradio/tab/radiogroup/radio/note` in SettingsMenu, `role=alert` on errors, radiogroup keyboard on effort rail (SettingsMenu:458-466). Risks: Halo Menu/Select own focus (see §6); tooltip no-op means icon-button names depend on `aria-label`; Halo `shadow-focus` must reach every control incl. textarea focus-within on the well (only the well, since textarea has outline:0); min target 44px vs 34px; colour-only state cues on mic/perm tones (Halo: icon+word too).
- **Perf.** Remove backdrop-blur; remove rAF loop in favour of a single `ResizeObserver`/scroll listener or Floating-UI-style autoUpdate; keep `autosize` out of per-frame work; only `opacity/transform` for continuous motion (Halo rule). `will-change: transform` is permanently on `.composer-host` (AP:1160) → a permanent compositor layer + fixed-containing-block; consider setting it only during FLIP.
- **WebView2/Tauri.** `backdrop-filter` flagged in DESIGN.md:297; `container-type: inline-size` on the bar (2041) is safe only while popovers portal; `clip-path` + `drop-shadow` stars; `:focus-visible` ring colour; paste of images relies on `ClipboardEvent` file items (WebView2 supports); drag/drop `dragend` quirk already patched (C:1353-1362). `-webkit-box` line-clamp used in SettingsMenu (713, 955). `field-sizing` not used (Halo Textarea autoGrow is JS).
- **FLIP coupling.** Any height change between hero and docked (e.g. Halo 44px controls, 22px line-height) shifts `dy` and `composerH`; AP's `ResizeObserver` (AP:34-38) pins stream scroll on each change. Changing composer height mid-redesign will move the whole stream's bottom padding.
- **Fonts.** Halo = Geist/Geist Mono (Google Fonts, HT:60-61); Rift = Inter Variable + JetBrains Mono (`package.json` deps). Composer text metrics (autosize line-height 1.5, hero 1.55) and the ghost-mirror exact-match requirement depend on the font; a font swap requires re-verifying `dict-ghost`/`magic-text` alignment.
- **Reduced-motion + loops:** Halo says everything respects it; fix the holes listed in §3.
- **Svelte specifics:** keyframes are component-scoped (same-name keyframes in three files are not shared; consolidating to `app.css` or `ui/` is a real change). `{#key fireKey}` ripple relies on unmount for cleanup (C:1831-1836).
- **Unverified (not read / not run):** actual rendered look of SettingsMenu/UsagePanel/EnhanceBar (only A1 screenshot of the docked bar seen); light theme; `app.css` `--z-popover` value; whether `data-model` attrs are consumed anywhere (no selector in the composer files; not grepped in `app.css`).

---

## 10 Open owner questions

1. **Anatomy.** Keep Rift v4 (flat control bar BELOW the well, C:1906-1909) or move to the Halo mock's single card with controls INSIDE (`Main.dc.html:126-138`)? This decides well height, focus-ring extent (ring around card vs well), hero→docked FLIP, and whether the bar can hide in hero.
2. **Tooltips.** Tooltips were deliberately removed 2026-06-15 (`tooltip.ts:1-8`). Halo has `Tooltip` (120ms, rises 4px). Re-enable for icon-only buttons + shortcut `Kbd` hints in the composer, or stay silent and put shortcuts elsewhere (menus/help)?
3. **Control density.** Halo `sm` Button = 36px, `md` = 44px; Rift bar = 34px. Accept 36px (and a taller bar), add a Rift `xs` 32/34px size, or keep 34px as a documented exception for dense toolbars?
4. **Enhance "magic".** Plan: kill shimmer/aura/stars/wand-pulse and show a quiet "Enhancing…" with Spinner (matches brief "no traveling shimmer/specks"). Any element you want kept?
5. **Ultra marker.** The sparkles pill lights only at `xhigh` (C:1761, 1782) but the ladder also has `max` and `ultra` above it (`modelMatrix.ts:504-505`). Bug or intent? And keep any pulse at all (plan says static)?
6. **Send feedback.** Keep a one-shot launch ripple (Halo `halo-ping` style, single ring) or drop it for plain Halo button press + the user message appearing?
7. **Send button shape.** 34px circle today; Halo icon buttons are rounded squares (radius-md). Follow Halo (and DESIGN.md:147 no-pill-buttons) or keep the circle as the one signature element?
8. **Stop shortcut.** Tooltip claims `Esc` stops the turn; grep shows every `assistant.stop(` caller is a click, none on Esc. Implement Esc-to-stop (Halo `Kbd` then true) or remove the hint?
9. **Placeholder policy.** Keep one phrase ("Ask anything about this project"; hero "What are we working on today?") per DESIGN.md:154-157, or allow a rotating teach-line now that Halo `Kbd` exists?
10. **Plan-mode colour.** On Electric blue `--info` becomes lime. Plan mode glyph should be: Halo `info` (lime), `accent` (blue), or a Rift-local blue?
11. **Popover surface.** Halo pop = `bg-100` + `line-200` + `shadow-pop`, no blur. OK to drop all glass/blur and the layered inner highlights across Slash/Settings/Enhance/Usage?
12. **Model picker complexity.** Keep provider tabs + "Other models" flyout + Fast toggle + effort slider in one popover (Halo can compose it but not as a single `Menu`), or split model and effort into two controls?
13. **Hero control bar.** Keep hiding the deck while hero idles (C:1624) or show perm/model at rest so nothing materializes on focus?
14. **Drag copy.** "Drop image to attach" vs files generally ("Drop files to attach")?
16. **Green (`--ok`).** Halo has no green; positive = accent. Auto-edit mode, Allow button, ctx/usage "ok" zone currently use `--ok`. Fold into accent, or keep a Rift-local green as a documented exception to "one signal colour"?
15. **Ownership of adjacent components.** GhPopover, PermissionBar, AskUserCard are not in the composer; confirm they go to the Welcome/shell and stream/tiles waves.
