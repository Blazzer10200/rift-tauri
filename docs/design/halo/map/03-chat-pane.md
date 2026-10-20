# 03 Chat pane, messages, markdown (map only, read-only)

Branch `design/chat-ledger`, HEAD `22727d24` ("faster reveal"), tree clean when mapped. Nothing run; one static lint (`node design-system/check-ui.mjs`, PASS, baseline unchanged) was the only command that executed project code. Numbers marked **(derived)** are computed from constants in source, **not measured**. Anything marked **(unverified)** is an inference from reading code and needs a live look before it is treated as fact.

Paths are relative to `C:/AI Workflow/projects/rift-tauri/` unless stated. `A` = `src/lib/components/assistant/`.

## KEY FINDING FIRST: which markup is live

`uiPrefs.streamMode` defaults **true** (`src/lib/state/ui-prefs.svelte.ts:105`, read back `:161` as "on unless localStorage is `"0"`"). `AssistantPane.svelte:717` therefore renders every **assistant** message through `stream/StreamTurn.svelte`, and `MessageBubble.svelte` only for **user** messages and **system** boundary/model-switch rows. `MessageBubble`'s whole assistant branch (turn-rail, avatar, head, `.tl-node` timeline, `.work` groups, thinking tiles, steer, stop-notice, TurnSummary) is the legacy path, reachable only if the owner flips "Stream view" off in Settings (`SettingsPage.svelte:820`). `StreamTurn` + `stream.css` (`src/lib/styles/stream.css`, 555 lines) are owned by another map; this file maps how they host `Markdown` and which `AssistantPane` / `MessageBubble` bits still render.

Decision the redesign must make (Q1): port the legacy assistant branch of `MessageBubble` (about 700 of its 1410 lines, incl. ~20 keyframes) or delete it. Not mine to delete; flagged.

---

## 1 Files & roles

| File | Lines | Role |
|---|---|---|
| `A/AssistantPage.svelte` | 450 (script 1-235, markup 237-331, style 333-450) | Chat page shell: init, split layout (grid of panes + draggable dividers, per-count persisted fractions), maximized-pane mode, browser-dock slide + resize. `workspaceRegistry.ts:32` lazy-loads it as the "Chat" page. |
| `A/AssistantPane.svelte` | 1563 (script 1-565, markup 567-863, style 865-1563) | One pane: split card chrome + header chips, empty-pane card, merged home/convo column, message scroller (`.stream`), scroll-follow (`pinToBottom`), jump-latest pill, turn rail, alerts/errors, composer host + FLIP, drag-drop zones. |
| `A/MessageBubble.svelte` | 1410 (script 1-275, markup 277-582, style 584-1409) | User bubble (live path) + system rows; legacy assistant timeline (dormant, see above); image lightbox; copy/retry; hover timestamps. |
| `A/Markdown.svelte` | 1477 (script 1-662, markup 664-667, style 669-1477) | marked + marked-alert + DOMPurify + Shiki renderer, the paced word-reveal, code-block chrome (copy, collapse), diff blocks, checklist-to-Tasks sync, link/path click routing. Used by StreamTurn, MessageBubble, BoundaryBlock, AgentCard, StreamAgent, StreamExitPlan, StreamThinking (imports: grep of `import Markdown`). |
| `A/AssistantWelcome.svelte` | 716 (script 1-146, markup 148-403, style 405-716) | Four welcome variants: needsAuth (hosts `ClaudeConnect`/`OpenAiConnect`), warm launchpad (has folder), local-mode launchpad, cold first-run (no folder). Another map (05-home-onboarding) may overlap; this one covers it as the chat empty state. |
| `A/FilePathMenu.svelte` | 97 | Point-anchored file-actions popover (VS Code / default app / reveal / copy path / copy name). Used by Markdown, EditDiff (`EditDiff.svelte:17`), GrepResult (`stream/GrepResult.svelte:10`). |
| Coupled, not mine | | `stream/StreamTurn.svelte` 554, `src/lib/styles/stream.css` 555, `Composer.svelte`, `stream/PlanHud.svelte` + `ActivityHud.svelte` (both `querySelector(".stream")`: `PlanHud.svelte:111`, `ActivityHud.svelte:177`), `src/lib/state/assistant/streaming.ts` (char pacer), `src/app.css` (tokens, `.hud-float`, `.rift-menu*`, `.btn`, `.ctx-chip`, `@keyframes enter`). |

---

## 2 Layout & anatomy

### Page (`AssistantPage`)
`.assistant` (flex col, transparent so the app dot-field shows, `:334-344`) > `.workbench` (flex row, `overflow:hidden`, `:345`) > `.layout` (`:351`) + optional `.dock-wrap` (browser dock, right).
- 1 pane: `AssistantPane` direct (`:292-297`). 2-4 panes: `.split` CSS grid, template `minmax(0, {frac}fr) 6px minmax(0, …)` (`:121-133`); divider = 6px track, 1px visible hairline via `::before` (`:409-426`), grip 2x24px at 0.7 opacity on hover/drag (`:436-449`). `SPLIT_MIN` 0.15 (`:44`). Fractions persisted in `localStorage rift.assistant.splitFracs.{n}` (`:45`, try/catch).
- Maximize: one pane, siblings unmount (`:244-253`), restore keeps `fracs`.
- Browser dock: `dock-wrap` (animated width) > `dock-inner` (inline `width: dock+3px`) > 3px `.dock-divider` + `.browser-dock` (`:301-328`).

### Pane (`AssistantPane`)
`.pane-shell` (`min-width:320px`, `:866`) > `.pane` (`role=region`, `tabindex 0|-1`, drag target, `:570-584`).
Split-mode only: `.pane.split` = rounded 14px card, 1px `--border`, margin 6px, bg `--bg-inset` 26%, `isolation:isolate`, unfocused opacity .84 (`:887-897`); focus rail 2px accent via `::before` (`:901-914`); focused border accent 30% + 22px accent-tinted drop (`:1106-1109`).
Pane head (split only, 28px, `:925-939`): index pill (16px) · title (ellipsis) · folder chip (max 130px) · working chip OR done chip · ctx chip (28x3 bar + % + cost) · maximize 18px · close 18px. Dbl-click head = maximize (`:445-448`).

Inside `.pane`, z-order low to high (all `position` verified in CSS):
1. `.csurf-col` z1 (flex col). Home: `overflow-y:auto; padding 30px 32px; scrollbar hidden`, children capped `max-width:940px` (`:1129-1145`), weighted spacers `54 / 46` flex (`:1138-1139`). Engaged: `justify-content:flex-end`, spacers `1 / 0` (`:1152-1154`). Convo: `.stream` + `.hud-stack` + `.alerts` + `.composer-host`.
2. `.stream` z1: the scroller. `flex:1; overflow-y:auto; padding 16px 18px 8px; scroll-padding-bottom:28px; overflow-anchor:none; scrollbar-width:none` (`:1181-1195`). Contains `.messages` (flex col, gap 18px, `max-width: calc(min(var(--chat-col-max),880px) - 36px)` = **844px at most**, `:1233-1239`; +18px extra margin-top before every non-first user bubble `:1245-1247`).
3. `.hud-stack` z4: absolute, top 10px, centered, `width: min(560px, 100% - 36px)`, holds PlanHud + ActivityHud (`:1166-1177`).
4. `.scroll-fade-top` z2: 44px gradient `--bg → transparent`, opacity 0/1 on `scrolledTop` (scrollTop > 8) (`:1212-1227`).
5. `.turnrail` z4: absolute, `right: max(12px, calc((100% - var(--chat-col-max))/2 - 38px)); top:50%`, chevron(34x30) + ticks(34x24, bar 2x12 → 2x20) + chevron, opacity .68 (`:1409-1435`). Shown when `exchangeCount >= 2` (`:825`).
6. `.jump-latest.hud-float` z3: absolute, `bottom: composerH + 12px` (inline, `:819`), `right: max(24px, calc((100% - var(--chat-col-max))/2 + 18px))`, 26px pill (`:1380-1394`). Shown when `!stickToBottom`.
7. `.alerts` (in `.csurf-col`, z1): notice + error/recovery banner, `max-width: var(--chat-col-max)` (up to **1100px**) (`:1437-1447`).
8. `.composer-host` z1 (`will-change: transform`, `:1156-1161`) wraps `Composer` (`.composer-wrap`: `max-width min(--chat-col-max, 880px)`, padding 6/18/10, `Composer.svelte:1846-1853`).
9. `.drop-zone` z5: tab/project drag overlay, `full` or `half left/right`.

Column widths (the inconsistency): text column 844, composer 880 (box 844 after its 18px padding), alerts up to 1100, jump pill and turn rail offsets computed from the **1100** `--chat-col-max` not the 880 cap. Screenshot `docs/design/redesign-audit/A1-ledger-chat.jpeg`: message column right edge at x~1538, turn-rail ticks at x~1680 (display px), i.e. the rail floats ~140px right of the text it indexes (matches audit item C14, `redesign-plan.md:61`).

### Message anatomy
- **User** (`MessageBubble` `data-role=user`): right-aligned text bubble `max-width min(100%,62ch)`, `padding 10/15`, radius `16 16 5 16`, accent-tinted gradient + inset highlight + soft drop shadow (`:1206-1223`). Optional image thumbs above (`:1251-1280`). Below: `.user-actions` row = Copy (opacity 0 until hover/focus-within, `:1244-1245`) + timestamp (11px tabular `--fg-subtle`, `:1238-1243`). No avatar/rail.
- **System**: `BoundaryBlock` / `ModelSwitchBlock` (other maps).
- **Assistant, live path**: `StreamTurn` (`sturn` > `sturn-body` > `snarr` > `<Markdown>`; ledger line under it when settled, `StreamTurn.svelte:426-492`).
- **Assistant, legacy path**: `.bubble` 2-col grid (26px lane + 12px gap) with avatar 22px, 2px gradient rail, header (provider · model · hover time · live dot/heartbeat · cost · copy/retry), `.tl-node` blocks each with a 9px status bullet at `left:-30px` (`:808-961`), grouped `.work` tile (`:970-1023`).

### Markdown typographic scale (`.md`, `Markdown.svelte:670-679`)
`.md` 13px (`--fs-md`) / 1.68, `--fg-2`, `padding-left:10px`. p margin-bottom 9px; h1 20, h2 16 (+border-bottom, 3px accent bar at `left:-10px`), h3 14 (bar uses `--info`), h4 13, h5/h6 12 uppercase; inline code 0.86em mono in a 6% chip, radius 5, `white-space:nowrap`; code blocks 12px/1.72 Shiki github-dark-dimmed in a `.tile` shell; tables 12px with uppercase 11.5px headers; lists 22px indent, custom 5px bullet; blockquote 2px accent edge on `--accent-soft`; `kbd`, `details`, `mark`, GFM alerts, task-list checkboxes (14px custom). Legacy bubble prose is `.text` 13px/1.55-1.62 (`MessageBubble.svelte:1195-1224`, assistant `max-width:78ch`).

---

## 3 States

| State | Where | Behaviour |
|---|---|---|
| Empty split slot | `AssistantPane:643-702` | Card: mark, "Empty pane", hint, New chat (primary) + Close pane, up to 4 project rows (hue monogram via `projectHue`), up to 3 resume rows. |
| Home / welcome (empty chat) | `:709-712`, `AssistantWelcome` | `showEmpty && !engaged`: welcome-wrap + hero composer. Variants chosen at `AssistantWelcome:149/160/267/272/321`: needsAuth, hasRoot launchpad, boot-hold (renders nothing branded, `:267-271`), local mode, cold. |
| Engaged home | `setEngaged` `:291-298`, effects `:301-324` | Focusing composer or starting dictation unmounts the welcome (fade+drift 200ms) and FLIPs composer down; blur with empty draft and no dictation floats back (welcome re-enters 220ms). Popover focus-bounce guarded (`:311-320`). |
| First send | `handleSend` `:270-278` + effect `:386-404` | Snapshots composer rect and welcome rect, FLIP composer to docked slot. |
| Conversation | `:713-751` | `.stream` + HUDs. |
| Streaming | `tab.streaming` | Last assistant message gets `streaming`; Markdown reveal active; follow-scroll active; per-pane "working" chip in split; legacy path adds rail glow + avatar halo + bullet pulses + stage dots. |
| Streaming, pre-first-block | legacy only `MessageBubble:348-356` | Three bouncing dots (`stage-strip`), fades out 220ms. StreamTurn has its own footer/thinking row (other map). |
| Background pane finished | `AssistantPane:450-466` | `doneFlash` "done" chip 6s, only when split and unfocused; cleared on focus. |
| Error | `:754-799` | Danger banner; auth-class errors (regex `:60-69`) become recovery banner with Open Settings / Sign in / Re-check. Renders in any pane that owns the erroring tab (`:55-56`). |
| Notice | `:757-761` | Info banner, click anywhere to dismiss, only on focused pane (`:53`). |
| Stop notices | `MessageBubble:524-542` (legacy) | max_tokens (warn + Continue button) / refusal. |
| Hover | `.bubble:hover` reveals model, cost, copy, timestamp, rail glow; `.md` code-block Copy on hover; table row hover tint | |
| Focus | `.pane.split.focused` rail + border + shadow; `:focus-visible` rings `0 0 0 2px var(--ring)` on pill, rail, copy; dividers 2px outline | |
| Disabled | `.tr-chev:disabled` opacity .3 (`:1424`); `.recovery-btn:disabled` .6 (`:1511`) | Halo's single disabled opacity is 0.45. |
| Scrolled / not at bottom | `scrolledTop` fade; `!stickToBottom` jump pill | Latch threshold 80px (`:195-202`). |
| Collapsed | Tall code blocks (> 24 lines) clamp to `max-height:23rem` with fade + "Show N more lines" pill (one-way expand, `Markdown:495-508`, `1165-1248`); tool groups; thinking; "New to Rift?" (`AssistantWelcome:240-265`). | |
| Narrow window | `.pane-shell min-width:320px`; welcome `container-type:inline-size` but **no `@container`/`@media` width rules in the file** (grepped via reading: none); `nr-grid auto-fit minmax(180px,1fr)`; Markdown `flat-short` lists use `columns:22ch`. No breakpoint logic for pane header (chips wrap by `min-width:0` ellipsis only). | |
| Reduced motion | Mixed: CSS `@media (prefers-reduced-motion: reduce)` blocks everywhere, **and** JS snapshots `prefersReducedMotion()` once at component init (`AssistantPane:167`, `Markdown:158`, `MessageBubble:6`). Snapshot is not reactive; an OS toggle mid-session needs a remount. Global `[data-slot]{animation:none!important}` at `app.css:120-122` covers only ui/ primitives. | |
| Reading-comfort a11y toggles | `app.css:758-780` | `[data-a11y-line-height]` and `[data-a11y-warm-tint]` target `.bubble` and `.markdown-body` only. **Assistant prose in stream mode is `.md` inside `.sturn`, matched by neither** (unverified at runtime, certain by selector). Settings preview (`SettingsPage.svelte:925,1737`) uses bare `.bubble/.markdown-body`, so it will look correct there and not in the real chat. |

---

## 4 Motion present

Legend: KEEP / KILL / RETUNE vs Halo rules (`03-motion.md`, `04-performance.md`): three speeds on one curve (fast 150 / base 300 / slow 600, `ease-out` cubic-bezier(0.2,0.8,0.2,1)), entrance once with stagger capped near 12, loops only for real activity, only opacity/transform continuous, blur only on small areas, reduced motion respected. Rift tokens today: fast 140, base 240, rise 460, slow 520, page 400, stagger 62, ease-page cubic-bezier(0.22,1,0.36,1), pulse-live 1.6s (`app.css:240-250`).

### 4.1 THE STREAMING TEXT REVEAL PIPELINE (end to end, current numbers)

Stages in the order a token travels. Latency column = added delay at 60Hz **(derived)**.

| # | Stage | File:line | Mechanism and exact numbers |
|---|---|---|---|
| S0 | Backend event arrives | `src/lib/state/assistant/streaming.ts:1319-1327` | `content_block_delta/text_delta` calls `enqueueText`. Burstiness is upstream (Rust stream); not mapped here. |
| S1 | **Char pacer** (rAF) | `streaming.ts:393-446` | `pendingText` buffer drained once per frame: `rate = max(360, pending/0.1)` chars/s, `n = max(1, round(rate*dt/1000))`, `dt` clamped to 100ms. 360 c/s floor = 6 chars/frame at 60Hz; a 200-char backlog drains at 2000 c/s. Hidden page flushes at once (`:401-404`). Comment `:436-439` says it was already tightened (was 0.25s/180 c/s) because it "compounds" with the word reveal. ~+1 frame (17ms). |
| S2 | Message mutation | `streaming.ts:176-187`, `379-391` | `appendText` builds a new blocks array and a new `{type:"text", text: last.text+chunk}` each frame; `tab.messages[idx] = …`. Re-runs `AssistantPane` `messages` derived and `StreamTurn` `messageToTurn` + `groupBlocks` every frame (`StreamTurn.svelte:40-41`). |
| S3 | Markdown parse throttle | `Markdown.svelte:186-203` | While `streaming`, `renderText` updates via one `requestAnimationFrame` (coalesces many deltas to 1/frame). Not when `reduceMotion`. +1 frame (17ms). |
| S4 | Full re-parse | `Markdown.svelte:574-607` | **Entire accumulated text** every frame: `marked.parse` (gfm, breaks) > `DOMPurify.sanitize` (allow-listed tags/attrs, style-hook `:25-33`) > `extractAndStripChecklists` (template DOM parse, only if checkbox present) > `tagFlatShortLists` (template parse if `<ul`) > `annotateCodeBlocks` (template parse if `<pre`). Cap 500,000 chars (`:156`). Shiki `highlightSync` inside the code renderer: memo keyed on full code string, only cached if ≤2000 chars, 400 entries (`highlighter.svelte.ts:113-146`); a **growing** fenced block misses the cache every frame and a >2000-char block never caches. |
| S5 | DOM inject + word wrap | `Markdown.svelte:619-641`, `260-293` | `container.innerHTML = html` (**replaces every node in the message every frame**), `wrapWords` walks all text nodes (skips `PRE`/`.shiki-block`), splits on whitespace, makes one `<span class="md-w-hold">` per word, resets `revealedUpTo = solidUpTo = 0`, then `applyReveal(shownCount)` re-stamps every already-revealed span (`className="md-w"` + `animationDelay = -(age*WORD_MS)ms`) and re-solidifies old ones (`md-w-shown`). O(total words) per frame. |
| S6 | **Reveal cursor** (rAF) | `Markdown.svelte:169-176`, `216-252` | `WORD_MS = 30`, `REVEAL_MS = 500`, `REVEAL_WINDOW = ceil(500/30)+1 = 18`, `CATCHUP_LAG = ceil(320/30) = 11`. Loop: `due = floor((t - last)/30)`; if `due>0`: `lag = totalWords - shownCount`; if `lag > 11` then `due = ceil(due*lag/11)`; `shownCount = min(shownCount+due, totalWords+18)`; `last = t` (**remainder is discarded, not carried**). Parks when caught up; `wakeReveal()` restarts on new text. Tail: target = `totalWords + 18`, so after the stream ends the cursor runs 18 more ticks (~0.55s) before parking. Held spans are `opacity:0` (`:689-691`, blur dropped today in `22727d24`). |
| S7 | Per-word CSS | `Markdown.svelte:683-695` | `.md-w { animation: md-word 0.5s cubic-bezier(0.22,1,0.36,1) both }`, keyframes `opacity 0→1` and `filter: blur(3px)→0`. `md-w-shown` has no rule (plain). Up to 18 words are mid-animation at once = up to 18 concurrent filter animations. Inline spans, so `transform` cannot be used on them as-is (no effect on non-replaced inline boxes), which is why the animation is opacity+blur only. |
| S8 | Chrome hold | `Markdown.svelte:708-728` | `:has()`-based: while every word inside `code/kbd/mark/a/h1-6/li/blockquote/table/tr` is still held, the element itself is `opacity:0` (transition `--dur-base` 240ms); `hr` held before an all-held next block and while last in a streaming message; `pre/.shiki-block/img` held while any earlier sibling has held words. Selectors like `*:has(.md-w-hold) ~ :is(pre, .shiki-block, img)` re-evaluate on every class flip. |
| S9 | Block entrances | `stream.css:90` | `.snarr { animation: blockIn var(--dur-base) var(--ease-page) both }` (opacity + translateY 3px, 240ms) on each narration block when it mounts. Code blocks: `code-rise` `--dur-rise` 460ms, translateY 6px + scale .992 (`Markdown:777, 1088-1093`). |
| S10 | Scroll follow | `AssistantPane.svelte:158-227` | See 4.2. |

**Effective cadence (derived).** Because `last = t` throws away the remainder, the base rate is `ceil(30/frame_ms) * frame_ms` per word: 33.3ms at 60Hz and 120Hz, 34.7ms at 144Hz, 33.3ms at 180Hz, i.e. about 29-30 words/s regardless of display. A model emitting about 45 words/s outruns it; lag climbs until it crosses 11 words, then `ceil(lag/11)` ticks 2 words per step (~60 w/s). The reveal therefore sits **about 11 words (~330-370ms) behind delivered text** in steady state, then each word needs ~100-150ms of its 500ms ease-out to become comfortably readable, so worst steady-state token-to-readable is about **0.5s**, plus the scroll lag in 4.2. Prior owner audit (`docs/design/redesign-plan.md:183-191`) measured the same story with the old numbers (42ms/520ms/0.2): "lag is deliberate pacing", gallery replay p95 5.6ms frames, 2 frames > 20ms of 12,569 (dev build). Today's commit cut it to 30 / 320 / 0.35; owner still calls it laggy (this task's brief).

**End-of-turn re-render (unverified, high suspicion).** When `streaming` flips false, `StreamTurn` swaps `{#if streaming}` to the settled branch (`StreamTurn.svelte:412-493`): the live `<Markdown streaming>` instances are destroyed and fresh `<Markdown text>` inside `.sanswer > .snarr` (`:426-430`) mount. A fresh Markdown has `everStreamed=false`, so it injects plain HTML instantly: the up-to-~11 words still held and any mid-blur words snap to solid, **and** the new `.snarr` plays `blockIn` again (opacity 0→1 over 240ms), so the finished answer can blink out and fade back in at the moment the receipt line appears and the activity group collapses. Needs a CDP recording to confirm; if true it is the most visible "glitch" at turn end.

**Per-delta restarts (unverified).** Because the whole container is rebuilt every frame, `.shiki-block`/`pre` (`animation: code-rise` 460ms, `:777,1088`) are new elements each frame and should replay their rise from opacity 0 on every delta while a fenced block is streaming; and chrome-hold `transition: opacity` cannot run on freshly created nodes. Word spans avoid this only because the negative `animationDelay` re-phases them.

### 4.2 Scroll follow and navigation

| What | File:line | Numbers |
|---|---|---|
| Follow lerp | `AssistantPane:169-190` | rAF loop, `scrollTop += delta*0.35` per frame; **snaps** (`scrollTop = scrollHeight`) if `delta > 900`; stops at `|delta| < 0.5`; skipped (instant write) when reduced-motion or `instant=true`. Frame-rate dependent (no dt normalisation): 90% of the gap closes in 6 frames = 100ms@60Hz, 50ms@120Hz, 33ms@180Hz **(derived)**. Was 0.2/frame (comment `:183-184`). |
| Triggers | `:208-227`, `:38` | `ResizeObserver` on `.messages` (any growth) when latched; effect on `messages.length`/`streaming` after `tick()`; composer `ResizeObserver` forces an **instant** pin (`:38`). |
| Un-latch | `:193-206` | Scroll event: ignore if `|scrollTop - pinEcho| < 2` (own echo); ignore latch change inside `glideUntil` window; else `stickToBottom = gap < 80`. A deliberate upward scroll of < 80px is pulled straight back by the next follow frame (**magnet, by design, plausible annoyance**). `onwheel` clears `glideUntil` (`:714`). |
| Tab switch | `:229-250` | Restore cached scrollTop instantly or pin instantly; never glide. |
| Jump to latest | `:252-257` | `stickToBottom = true; glideUntil = now+700; scrollTo({behavior:"smooth"})` (native smooth, browser-chosen easing/duration). If content is still growing the follow lerp starts too and the two write the same property (unverified interaction). |
| Turn navigator | `:121-149` | `scrollTo({behavior:"smooth", top: bubbleTop - 16})`; active tick = last user bubble whose top is within 48px of the scroller top; **querySelectorAll + one `getBoundingClientRect` per user bubble on every scroll event** (`:124-138`), and scroll events fire every frame during the lerp. |
| Pin echo bug candidate | `:181-187` | If `scrollTop` assignments are integer-snapped at DPR 1, `delta = 1` gives `+0.35`, rounds to no movement, `|delta| < 0.5` never true, so the rAF loop re-arms forever with a 1px gap and swallows later `pinToBottom()` calls (`if (smoothRaf) return`). (unverified, needs a CDP look). |

### 4.3 Full motion inventory

Durations in ms; "tok" = token. Rows are grouped by file.

**AssistantPage.svelte**
| Line | What | Prop | Dur / ease | Trigger | Verdict |
|---|---|---|---|---|---|
| 15-22 | `dockSlide` (transition:dockSlide `:307`) | `width` px + opacity | 220, Svelte `cubicOut` (not the brand curve) | dock open/close | KEEP the width tween (it drives the native child webview's `syncBounds`, see `:302-306`); RETUNE to `dur-base` 300 on `ease-out` (Halo allows layout-prop motion only where justified; this is the one). |
| 391, 425, 444 | divider bg / width / grip opacity | bg, width, opacity | `--dur-fast` 140 ease-out | hover/drag | KEEP, snap to 150. |

**AssistantPane.svelte** (CSS unless "JS")
| Line | What | Prop | Dur / ease | Trigger | Verdict |
|---|---|---|---|---|---|
| 879, 894 | pane border/bg/shadow/opacity | those | fast ease-out; base `--ease-soft` | focus/hover | RETUNE to fast/base `ease-out`. |
| 895, 916 | `paneIn` | opacity, translateY 4px | `--dur-base` `--ease-page` `backwards` | pane mount | KEEP (entrance once), Halo rise is 14px/600: decide. |
| 901-914 | focus rail | opacity, `scaleX(.6→1)` | base `--ease-soft` | focus | KEEP; Halo SplitPane focus = accent line + grip. |
| 1022 | ctx fill | `width` | base `ease-out` | ctx % change | RETUNE to transform scaleX (Halo Progress eases value). |
| 1042-1047 | `pane-live-dot` pulse | box-shadow | `--pulse-live` 1.6s infinite | streaming in split | RETUNE: real activity, becomes `LiveDot` (1.8s ping, transform/opacity, not box-shadow). |
| 1078-1080 | `pane-done-in` | opacity, translateY -2px | base `--ease-soft` | background finish | KEEP as Badge entrance. |
| 1092, 994, 1328, 1363 | hover bg/color/border; empty-row chevron slide-in | colors, opacity, translateX(-3px) | fast | hover | KEEP, 150. |
| JS 344/355 | `welcomeOut` / `welcomeIn` | opacity, translateY ∓10px | 200 / 220, **linear** (no `easing`) | engage/send | RETUNE to `ease-out`, `dur-base`. |
| JS 373-382 | composer FLIP | transform translateY | 460 `cubic-bezier(0.22,1,0.36,1)`, `transitionend` + 540ms timer | engage/send | KEEP the technique (transform only); RETUNE 460 to 300 or 600 (Halo has no 460). |
| 1148 | composer-host `enter` on home | opacity, translateY 4px | base `--ease-page`, 200ms delay | home mount | KEEP as stagger beat 4. |
| 1225 | top fade | opacity | base ease-out | scrollTop > 8 | KEEP. |
| 1263-1271 | empty-card `pane-empty-in` | opacity, translateY 6px + scale .985 | 320 ease-page | mount | RETUNE (Halo EmptyState has no scale; use rise or `halo-zoom`-like). |
| 1390, 1402-1405 | jump pill `jump-in` | opacity, translateY 8px | base ease-page | `!stickToBottom` | KEEP; add leave transition (none today: pill vanishes). |
| 1413, 1432 | turn rail hover + tick width | opacity, bg, shadow; width 12→20 | base / fast | hover/active | RETUNE: tick width is a layout prop, use scaleX. |
| 1457, 1513 | alerts `enter` | opacity, translateY 4px | base cubic-bezier(.22,1,.36,1) | mount | KEEP; add leave (none today). |
| 1535, 1556-1559 | drop zone `drop-in` + bg/border | opacity; colors | fast | drag | KEEP. |
| JS 145, 252-257 | smooth native scrolls | scrollTop | browser default | click | RETUNE: custom or accept native; unify with follow. |

**MessageBubble.svelte** (live: user bubble, lightbox. Legacy: everything else)
| Line | What | Prop | Dur / ease | Verdict |
|---|---|---|---|---|
| 626 | `.bubble` `enter` (applies to user bubbles in live path too) | opacity, translateY 4px | base `cubic-bezier(.22,1,.36,1)` | KEEP, once. |
| 810-811 | `.tl-node` `enter` + `--idx*35ms` | same | base + stagger (idx cap 6 = 210ms) | legacy; if kept, stagger cap fine. |
| 817-823 | `node-stream-enter` on streaming `.tl-node:not([data-kind="text"])` | opacity, translateY 3px, **blur 4px** | 480 | legacy. **Bug:** `nodeKind` returns `"prose"` for text (`bubble/helpers.ts:193`), never `"text"`, so the `:not` also matches prose nodes and blur-ins them on top of the word reveal, contradicting the comment `:813-816` (legacy path only). KILL blur. |
| 668-686 | `rail-stream` glow | box-shadow | 2400 infinite | legacy; KILL (decorative loop over chrome, DESIGN §8 `DESIGN.md:247-250`). |
| 705-711 | `avatar-halo` | box-shadow | 1800 infinite | legacy; KILL. |
| 774-781 | `stage-bounce` dots | opacity, transform, scale | 1100 infinite | real activity; KEEP idea as `LiveDot`/Spinner. |
| 859-869, 1052 | `bullet-pop` (spring overshoot) | scale 0→1.3→1 | 340 `cubic-bezier(.34,1.56,.64,1)` per node, staggered | legacy; Halo reserves spring for things you flip/grab: RETUNE/KILL. |
| 905-912, 989-993 | `tl-bullet-pulse`, `nodePulse` | box-shadow | `--pulse-live` infinite | real activity; collapse to one LiveDot. |
| 1004-1006, JS 408/488 | `workOpen`, `slide` | translateY; height | 340 ease-page; 180/200 ease (Svelte default `cubicInOut`) | Halo Accordion = height `dur-base` + content fade; RETUNE. |
| 1317, 1339 | thinking `dots`, `live-dot` | opacity, scale, box-shadow | 1100 / 1400 infinite | legacy. |
| 1363-1366 | streaming `.content` **mask-image** gradient | mask | static | legacy; a mask on a streaming container forces offscreen compositing every repaint: KILL. |
| 1369-1374 | `tid-flash` | box-shadow | 1400 | cross-link highlight (used by dock Steps); KEEP behaviour, class is `:global`. |
| 1262-1268 | image thumb hover lift | translateY -1px, shadow | fast | Halo hover lift is 3px; RETUNE. |
| JS 349, 525, 538, 574 | `fade` out 220 / in 160 / lightbox 140 | opacity | Svelte linear | RETUNE to tok. |
| JS 74 | `setInterval` 500ms `tickNow` | reactive state | — | drives heartbeat label while streaming (legacy head); cheap. |

**Markdown.svelte** (CSS): 684 `md-word` (see 4.1); 709-727 chrome-hold opacity (240); 777/1088 `code-rise` 460 (**RETUNE**, replays per delta); 809/1132 copy opacity fast; 968 link underline-colour fast + `opacity 220ms`; 1233 `.cm-pill` hover lift; 1380 details chevron rotate fast; 1469 `md-path` hover `text-shadow` glow (Halo: only glow-strong under primary button, KILL); 1004 table row hover tint.

**AssistantWelcome.svelte**: 427-430 children `enter` 320ms, delays 0/80/140 (+ `.newrift` 200 override at `:464`; comment `AssistantPane:1146-1147` still says 140: doc drift); 448 `glyph-breathe` 4.2s infinite scale (auth mark, decorative loop: KILL); 471-473 `ge-breathe` 3.4s infinite opacity on the greeting dot (decorative loop: KILL); 509-510 resume tile hover `translateY(-1px)` base ease-page; 522 chevron `translateX(3px)` + spring token; 544/557 `gcReveal` 300; 599-605 open-folder target hover lift/`scale(.997)` + glow shadow `0 0 58px accent 60%` (Halo: glow only under primary button hover, RETUNE); 619/621 badge scale 1.04; `Skeleton` used for boot (`:206-216`, Halo Skeleton is the right match).

**FilePathMenu.svelte**: no motion at all (pops in), no leave. Halo Menu: scale-in 140ms (dur-fast) from the corner (`halo-pop-in`), rows stagger 18ms capped at 12 (`bundle.css:411-420`).

---

## 5 Styling wiring

- **Tailwind: none** in these six files (no utility-class attributes, 0 arbitrary values). **All styling is scoped `<style>` blocks** (page 117 lines, pane 698, bubble 826, markdown 808, welcome 311, menu 9) plus `:global()` reaches into child markup (Markdown is entirely `.md :global(...)`) and shared `.css`: `app.css` (`.btn`, `.hud-float`, `.rift-menu*`, `.ctx-chip`, `.tile*`, `@keyframes enter`), `stream.css` (`.snarr` etc., imported by `StreamTurn.svelte:2`).
- **Primitives**: `bits-ui` / `components/ui/` not used here. Hand-rolled: the file menu (`use:portal` + `role="menu"`, `FilePathMenu:57`), lightbox (`use:portal` + `role="dialog"`, `MessageBubble:567-571`), the pane/dock dividers, jump pill, turn rail. These count toward `check-ui` ratchets (`overlay:use-portal 11`, `overlay:aria-role 17`; baseline `design-system/ui-baseline.json`: color:hex 21, rgb-hsl 16, oklch-literal 131, arbitrary 2, all currently PASS, may only go down).
- **Tokens used**: surfaces `--bg --bg-inset --bg-elev-1/2/3 --surface --surface-hover`; text `--fg --fg-2 --fg-muted --fg-subtle --fg-faint`; lines `--border --border-strong`; brand `--accent --accent-soft --accent-fg --ring`; status `--ok --warn --danger --info (+ -soft)`; shape `--radius-sm/lg/xl/2xl --r-card --tile-radius --tile-radius-inner --tile-edge --tile-well --tile-border --island-* --ghost-border`; type `--fs-xs/sm/md`, `--font-ui --font-mono`, `--code-fs/--code-tab/--code-liga` (user code prefs); motion `--dur-fast/base/rise --ease-page/soft/spring --pulse-live`; layout `--chat-col-max`; shadow `--shadow --shadow-float`.
- **Hard-coded counts per file** (my tallies by regex; `px` = every `Npx` literal incl. comments):

| File | px literals | hex/rgba (real) | oklch literals | font-size px literals | distinct radius values |
|---|---|---|---|---|---|
| AssistantPage | 19 | 0 | 0 | 0 | 2 (1px, 2px) |
| AssistantPane | 186 | 1 rgba | 7 | 7 (all `10px`) + 9 token uses | 12 (999, 8, 6, 2, 14, 13, 12, 5, 4, 50%, var x2) |
| MessageBubble | 222 | 4 `#000` (mask) + 2 rgba | 4 | 20 (9.5/10/10.5/11/11.5/12) + 7 token uses | 10 (50%, 8, 999, 6, 10, 7, 4, 2, `16 16 5 16`, token) |
| Markdown | 150 | 2 `#000` + 1 rgba | 13 | 14 (9.5, 10, 10.5, 11, 11.5, 13, 14, 16, 20) + 7 token uses | 10 (4, 0, 999, 8, 3, 5, 50%, 2, tokens) |
| AssistantWelcome | 226 | 0 | 5 | 26 (9, 9.5, 10, 10.5, 11, 11.5, 12, 12.5, 13, 13.5, 16, 27, 28) + 0 token | 13 (999, 8, 12, 7, 6, 5, 50%, 18, 17, 14, 11, 0 3px 3px 0, var 2xl) |
| FilePathMenu | 2 | 0 | 0 | 0 | 0 (inherits `.rift-menu` radius 14) |

  Distinct sub-12px font sizes in one component family: 9, 9.5, 10, 10.5, 11, 11.5, 12.5, 13.5, 27 (non-token). Halo type is a 7-step sans scale 12/13/15/16/20/32/48 plus mono 11/13/30, with `caption` 12 the **smallest sans size** (`tokens.json` type group). Rift's 9-11.5px chrome has no Halo home.
- **Oklch literals** worth knowing: `oklch(0.92 0.05 22)` / `0.99 0.01 22` error text (`AssistantPane:1462,1496,1506`), project monogram `oklch(0.78 0.14 var(--ph))` (`:1342-1344`, intentionally a per-project hue, `projectHue`), diff add/del tints `oklch(.76 .18 152)` / `oklch(.68 .20 22)` (`Markdown:1303-1320`), `oklch(0.18 0.012 245)` code-fade base (`:1208-1209`, hard-wired to a block fill and **stale against the Halo surface**).
- **Fonts today**: Inter Variable (UI), JetBrains Mono (code), Lexend Variable (welcome greeting `AssistantWelcome:474`, and the reading-comfort font). All via `@fontsource-variable/*` (`package.json:41-43`). Halo wants Geist + Geist Mono and its `bundle.css` does `@import url(fonts.googleapis.com…)` (`bundle.css:1`); Tauri CSP (`src-tauri/tauri.conf.json:28`) is `font-src 'self' data:` and `style-src 'self' 'unsafe-inline'`, so the remote import **will be blocked**; Geist must be self-hosted.
- **Tokens that do not exist in Halo** (used here): `--ok` (green done/checked states), `--bg-elev-1/2/3` (4 steps vs Halo's bg-000..300 = 4, mapping ok), `--fg-faint` (Rift L .43 `#4B5056`, far darker than Halo `ink-faint` `#7d8592`), `--tile-*`/`--island-*`, `--accent-h/c/a` user-tunable hue/vividness/strength (Halo accent is a fixed per-theme hex), `--stagger` 62 vs 60, `--dur-rise`/`--dur-page` (Halo: slow/none).

---

## 6 Restyle boundary

### LOGIC to keep (do not rewrite while restyling)
**AssistantPage**: init + new-tab seeding (`:23-39`), split fractions + removal/add adaptation + persistence (`:41-111`), grid-template builder (`:121-133`), divider pointer/keyboard handlers + ARIA (`:135-196`, `:271-287`), `pendingBrowserUrl` handoff (`:198-211`), dock resize (`:213-233`), `dockSlide` width-tween logic.
**AssistantPane**: all `assistant.*` store calls (send, setFocusedPaneById, closePane, toggleMaximizePane, pickTabFolder, newTab, openTab, openProjectInPane, dropTabIntoPane, dismissNotice, startLogin, recheckAuth, setTabScroll/getTabScroll), derived tab state (`:43-110`), auth-error regexes (`:60-69`), the whole scroll engine (`pinToBottom`, `pinEcho`, `glideUntil`, `onScroll`, tab-switch restore, composer-height ResizeObserver, `:112-257`), turn navigator math (`:121-149`), composer FLIP + engaged-posture state machine incl. focusout/popover guard (`:259-404`), drag-drop half detection (`:410-564`), doneFlash timer (`:450-466`), empty-pane recents/projects (`:477-513`).
**MessageBubble**: copy, context menu (`:43-56`), expand state (`:60`, `:150-155`), lightbox state + data-URL MIME allow-list (`:361`, `:432`), `grouped` timeline build (`:199-264`), heartbeat tick (`:64-91`), stop-notice Continue/Retry handlers, `PermissionBar`/`EditDiff`/`ToolChip` mounting. Props contract: `message, streaming, isLast, tab, tabId, workspaceRoot`.
**Markdown**: sanitisation config + DOMPurify style hook (`:18-34`, `:587-604`, security-sensitive: touch only with `/quick-review`), marked renderers (diff, codespan path detection `pathish` `:140-145`), `fireCodeCopy` incl. diff-row rebuild (`:316-347`), click routing (copy / expand / path menu / links: scheme allow-list `:395-412`, localhost to browser dock), checklist to Tasks sync (`:460-487`, `:654-661`), `annotateCodeBlocks`, `tagFlatShortLists`, `markCollapsible` thresholds (18 + 6), `expandedBlocks` persistence, 500k cap, reveal engine **numbers and structure** (decide per Section 8; behaviour is the owner's complaint).
**AssistantWelcome**: `needsAuth` mounting of `ClaudeConnect/OpenAiConnect`, lazy loaders and `sharedRootMatches` guard (`:62-92`), resumables filter (`:128-144`), localStorage dismissal (`:125-126`), greeting hour tick (`:114-119`), `bootLoad.showSkeleton`, all `assistant.*`/`github`/`workspace` calls.
**FilePathMenu**: `invoke("open_in_vscode")`, `openPath`, `revealItemInDir`, clipboard calls, viewport clamping rAF (`:22-31`), outside-click/Esc wiring, `environment.code` gating. **Note** `catch` only `console.warn`s (`:47`) while Halo CodeBlock says "Copy failed" in danger, "never pretend": fail-loud candidate.

### MARKUP / CSS to change
Everything in the `<style>` blocks; class names on `.pane*`, `.csurf-col`, `.welcome*`, `.bubble`, `.md` content classes; the pane header chips; empty-pane card; alerts; jump pill; turn rail; user bubble; code-block head/`.shiki-head` string built in `codeHead()` (`Markdown:53-55`, **markup is a JS string**, so restyling it means editing TS, not CSS); diff-row string builder (`:99-105`); `.cm-pill`/`.cm-fade` string (`:506`); `.tasks-migrated` string (`:482`, contains an emoji, which Halo bans in UI: `05-content-and-icons.md`); `✓ done`, `ℹ`, `⚠` text glyphs in the pane (`:611`, `:758`, `:764`) that should become icons.

**Hard couplings that survive a restyle** (selectors outside these files): `.bubble[data-role="user"]` (turn navigator `AssistantPane:126`, `:1245`), `.stream` (`PlanHud.svelte:111`, `ActivityHud.svelte:177`), `.bubble`/`.sturn` counts (`scripts/cdp/serve.cjs:876`, `c.sh:18`, `cdp/README.md:120`), `.composer-wrap` (`c.sh:28`), `#actnode-{id}` ids and `.tid-flash` (cross-link flash from dock Steps/Review diff, `MessageBubble:510,1369`), `[data-a11y-*] .bubble/.markdown-body` (`app.css:758-780`), `.rift-menu*` (FilePathMenu), `.btn.primary/.ghost/.sm` (empty pane), `.hud-float`, `.ctx-chip`, `@keyframes enter`, `.tile/.tile-head/.tile-chev` (code blocks via `codeHead` classes `shiki-head tile-head`, `tile-label`, `tile-dur`).

---

## 7 Halo mapping

Halo has **no chat, message, prose, markdown, turn, composer or scroll-follow component** (component list: `components/` dir and `README.md:20-33`). Chat is therefore a set of **Rift-specific parts built from Halo tokens**; Halo components cover chrome around it. Target theme: `data-theme="blue"`: `accent #6aa8ff`, `accent-ink #08090c`, `accent-soft rgba(106,168,255,.14)`, `glow-strong rgba(106,168,255,.28)`, and **`info` becomes lime `#c6f35a`** (swapped in the blue preset, `tokens.json` info), which affects Rift's `h3` bar and anything using `--info`.

| Rift element | Halo component / pattern / token | Notes and gaps |
|---|---|---|
| Page bg, pane card | `bg-000` ground; pane = `Card`-like `bg-100` + `line-100`, `radius-lg` 16 | Halo "no shadows on cards" (Rift's focused-pane accent drop shadow goes away); Rift card radius 14 vs 16; dot-field texture has no Halo analogue (`README.md:8`: "no colour washes"). |
| Split dividers (2-4 panes) | `SplitPane` (`README` in components/SplitPane) | **Gap:** Halo takes exactly two children (nest for more); Rift supports up to 4 with N-fraction persistence and per-count storage. Keep Rift's grid, restyle to Halo bar: 1px `line-100`, hover `line-300`, drag/focus `accent`, grip 4x28, 13px hit area, arrow step 16/Shift 64 (Rift 2%/5%), Home/dblclick reset. |
| Browser dock divider | same SplitPane bar style | width tween = native-webview constraint; Halo has no equivalent. |
| Pane head | hairline row, `caption` 12px | Rift chips are 16px high with 10px text: below Halo minimums (caption 12, targets 36-44). |
| index pill / done chip | `Badge` (`neutral` / `accent`) | `Badge` is 12px/18px with 3px 10px padding. |
| "working" chip | `LiveDot tone=accent` + label | LiveDot ping 1.8s infinite, transform/opacity (Rift pulses box-shadow). |
| ctx chip bar | `Progress size=sm` (4px bar) with `tone` mapping 70/90 to `warn`/`danger`; cost in `halo-mono` | Halo has no ctx "ring in a chip"; `Ring` exists for one goal. |
| maximize / close | `Button variant=ghost size=sm` icon-only, `ariaLabel`, `Tooltip` | **Size conflict:** Halo ghost sm = 36px min-height (`bundle.css:56`); Rift 18px in a 28px head. Needs a "dense toolbar" decision (Q2). |
| Empty split pane | `EmptyState` (concentric rings, 56px mark, title 16/22, desc 14/21 ≤340px, one primary action) | **Gap:** Rift card carries 2 actions + project list + resume list; EmptyState "one action at most". Compose `EmptyState` + `NavItem`/list rows. |
| Welcome greeting | `title` (32/38) or `display` (48/52) type, `SectionLabel` (`01 / CONTINUE LATEST`), `Card` tile, `Badge` chips, `Button secondary` | Greeting font Lexend vs Halo Geist; hero logo `Logo`. "Still up" eyebrow = `SectionLabel`-like (mono, `.12em`, uppercase 11px). |
| Resume tile | `Card lift` (clickable) | Rift hover lift 1px, Halo `lift` 3px. |
| Cold welcome viewfinder (open folder) | `FileDrop` (accent border on drag, Browse button) | Rift accepts a click-to-pick + "drop a folder anywhere"; FileDrop emits `File` objects, not folder paths (**Tauri folder drop needs Rift handling**). Corner-bracket decoration is a Rift flourish with no Halo equivalent (kill or keep as Rift-specific). |
| "New to Rift?" | `Accordion` (single, height `dur-base`, content fade) | direct match; Rift's `gcReveal` goes. |
| Primer steps (numbered 1-3) | `Stepper` visual or `SectionLabel` index pattern | `Stepper` is interactive wizard UI; likely token-only reuse. |
| Alerts: error | `Banner tone=danger` (title + description, one action, `onDismiss`, announced as alert) | Auth recovery has 2-3 buttons: Banner allows action as children (small secondary Button). |
| Alerts: notice | `Banner tone=info` with `onDismiss` | Rift is click-anywhere-to-dismiss; Halo has an explicit close. |
| Jump to latest | `Button secondary size=sm` + `ChevronDown` | **Gap:** Halo restricts `shadow-pop` to popovers/menus/dialogs/drawers/toasts (`01-themes-and-colour.md:89`); a floating pill is not listed. Use `line-200` outline on `bg-200`, no shadow, or add a rule. |
| Turn rail | no match (`Stepper`, `Pagination` are page/wizard widgets) | **Rift-specific part.** Use `line-300` ticks, `accent` active, `dur-fast`, `Tooltip`; fix the column-edge offset. |
| User bubble | no component; first mock used `bg-200`, `radius-lg` 16, padding 12/16, max 560px, text 15/22, timestamp below in `caption` `ink-faint` (`Main.dc.html:75-78`) | Rift-specific `UserMessage`. Rift today is gradient + inset + shadow (all banned by Halo "no colour washes / shadows"). |
| Assistant prose | type `body` 15/22 (mock used 15/24, `Main.dc.html:17`) in `ink`; secondary `ink-muted` | Rift today is **13px**/1.68 `--fg-2`. Halo body is 15: a global density decision (Q3). Measure target ≈ 760px (mock `max-width:760px`) vs Rift 844. |
| Inline code | mock: mono 13px, `bg-200`, `radius` 6, padding 1px 6px (`Main.dc.html:18`) | Rift: 0.86em, 6% chip with 1px border, `nowrap`. |
| Code blocks | `CodeBlock` (`bg-100`, `line-100`, `radius-md`, head row 40px with title / `language` label `label` style, copy button, `lineNumbers`, `highlight`, `maxHeight`, `kind=terminal` on `bg-000`) | **Gaps (Q4):** (1) Halo **never colours syntax**; Rift ships Shiki `github-dark-dimmed` inside `codeHead` shells, (2) no diff rows (+/- gutters, hunks), (3) no "Show N more lines" fade+expand (Halo scrolls inside `maxHeight`), (4) no line-count label, (5) Halo copy shows **"Copy failed" in danger**, Rift swallows copy errors (`Markdown:346`). Since `codeHead()` and rows are TS strings, class-level reuse of `.halo-code__*` is feasible, the React component itself is not. |
| Tables | `DataTable` tokens (`bg-100`, `line-100`, sticky head) | CSS-only port; DataTable is interactive (sort), Markdown tables are static. |
| Blockquote / GFM alerts | `Banner` tone recipe (`color-mix tone 8% / border 32%`, `bundle.css:387`) | Rift uses a 2px left edge (`--tile-edge`), Halo uses full border. |
| Task checkboxes, `kbd`, `details` | `Checkbox` (spring pop), `Kbd` (22px keycap, mono 11px), `Accordion` | CSS ports. `.tasks-migrated` emoji must go. |
| Links | `accent` text | Halo `accent` on `bg-000` ≥4.5:1. |
| Hover timestamps / model / cost / copy | `caption` + `ink-faint` (timestamps), `Badge neutral` (cost), `Button ghost sm` (copy/retry) | Halo says `ink-faint` is for timestamps only and never on `bg-300`. |
| FilePathMenu | `ContextMenu` / `Menu` surface (`bg-100`, `line-200`, `radius-md`, `shadow-pop`, rows min-height 40px / 14px text, item stagger 18ms ≤12, scale-in `dur-fast`) | **Gap:** `Menu` is trigger-anchored, `ContextMenu` is right-click-anchored; FilePathMenu opens at an arbitrary (x,y) under a clicked span. `ContextMenu`'s fixed layer (`.halo-ctx__layer`) is the closest. Also needs `Kbd` hints and `danger`/`disabled` rows only if added. Row 40px vs Rift row ~30px. |
| Image lightbox | `Dialog` (`scrim`, focus trap, Esc) | Rift's is click-anywhere + opaque scrim already (no backdrop-filter). |
| Boot skeleton | `Skeleton variant=list` | exists in Welcome already (shell `Skeleton.svelte`). |
| Streaming indicator / elapsed / tokens | `LiveDot` + `RollingNumber` | in `StreamTurn` (other map). |
| Toasts for copy feedback | `Toast` | Rift uses inline "Copied" label; Halo CodeBlock does the same inline in `accent`. |

Gaps needing Rift-specific parts: ChatColumn (shared 760/844 measure for messages, composer, alerts), UserMessage, AssistantProse/Markdown theme, TurnRail, JumpToLatest, ScrollFollow engine, WordReveal, DiffBlock, CollapsibleCode, PaneHeader, AnchoredFileMenu.

---

## 8 Pain points (evidence)

1. **Reveal is stacked pacing, not jank.** Char pacer (S1) + rAF parse throttle (S3) + word cursor at ~30 w/s with an 11-word allowed lag (S6) + 500ms fade (S7) + follow lerp (4.2): about 0.5-0.6s worst steady token-to-settled **(derived)**. Owner measured the previous constants as pacing, not frame drops (`redesign-plan.md:183-191`). Still the owner's complaint after today's `22727d24`.
2. **Cadence quantised to frame time and drops remainder** (`Markdown:229-237`): 29-30 w/s ceiling, uneven steps (a "word" every 2 frames at 60Hz), no carry. The reveal moves in visible ticks rather than a smooth flow.
3. **Whole-message rebuild every frame** (S4+S5): `innerHTML`, full parse, full wrap, O(words) restamp. Costs: selection and hover reset, text selection lost mid-stream (unverified), code-block animations replaying, `expandedBlocks` rehydrate hack (`:626-629`), Shiki re-tokenize on every frame for a growing block (cache miss), and checklist JSON stringify each frame (`:655-660`). Gallery measurement (`redesign-plan.md:183`) did not exercise a long real turn.
4. **Per-word `filter: blur(3px)` on up to 18 concurrent spans** (S7) vs Halo "only opacity and transform run continuously; blur on small areas, once at a time" (`03-motion.md:141`, `04-performance.md:148`). Plus the `:has()` chrome-hold selector set (S8) with a universal sibling combinator re-evaluated on each class flip.
5. **End-of-turn swap re-mounts the answer and replays `blockIn`** (4.1, unverified): possible blink + snap + layout jump when the live body collapses into the ledger (`StreamTurn:412-493`).
6. **Follow engine quirks**: frame-rate-dependent lerp, 900px snap, 80px magnet that eats small upward scrolls, native smooth `scrollTo` contending with the lerp on Jump, O(user-bubbles) layout reads per scroll event, possible 1px stall (4.2).
7. **Column mismatch**: text 844, composer 880, alerts up to 1100, rail/pill offset from 1100; rail ~140px off the text edge in the audit screenshot. Prior audit C6/C14 only partly fixed.
8. **Decorative loops the Halo rules forbid**: greeting dot breathe 3.4s, auth mark breathe 4.2s, legacy rail glow 2.4s, avatar halo 1.8s; many simultaneous pulses in the legacy path (rail + avatar + bullet + stage dots + live dot).
9. **Mask-image on streaming `.content` (legacy)** and `.cm-fade` `backdrop-filter: blur(1.5px)` on every collapsed code block (`Markdown:1212-1213`): DESIGN §10 / `app.css:609-611` ban backdrop-filter on WebView2 ("garbage rects"); this one is not on a fixed layer, but it is the same class of risk.
10. **Dead/dormant weight**: ~700 lines of legacy assistant markup in `MessageBubble` plus `node-stream-enter` prose bug (`nodeKind` "prose" vs `:not([data-kind="text"])`, `MessageBubble:817` vs `bubble/helpers.ts:193`).
11. **Type scale sprawl**: 9, 9.5, 10, 10.5, 11, 11.5, 12.5, 13.5 px literals and 27/28px greeting across 5 files (Section 5), versus 3 `--fs-*` tokens used inconsistently; 10px text is everywhere in the pane head.
12. **Reading-comfort toggles don't reach stream-mode prose** (selector mismatch, Section 3).
13. **Silent copy failure** (`Markdown:346`, `MessageBubble:165`, `FilePathMenu:47`): `console.warn` only, no UI.
14. **Emoji / glyphs in UI**: `📋` (`Markdown:482`), `✓ done`, `ℹ`, `⚠` (`AssistantPane:611,758,764`).
15. **Doc/stagger drift**: welcome launchpad stagger comment says 140ms, code sets 200ms (`AssistantPane:1146` vs `AssistantWelcome:464`).
16. **No leave transitions** on jump pill, alerts, pane head chips (pop out).
17. `prefersReducedMotion()` is a one-time snapshot in three components (not reactive).

---

## 9 Risks

- **Tests**: no test imports or queries these components' classes/markup. `grep` of `*.test.ts` for `.bubble`, `.md`, `data-role`, `MessageBubble`, `Markdown`, `pinToBottom`, `turnrail` found only a comment in `stream/streamModel.test.ts:593`. `bubble/helpers.test.ts` (244 lines) and `streamModel.test.ts` (1011 lines) test pure logic, so a markup restyle is test-safe; a **logic** change to `bubble/helpers.ts` or `streamModel.ts` is not. Result: **no automated net** for the visual/motion work. It will need the `/dev/gallery` route (`src/routes/dev/gallery`, `src/lib/dev/galleryFixtures.ts`) and CDP (`scripts/cdp/c.sh`, `rift-ui` skill) for before/after.
- **`check:ui` ratchet** (`design-system/ui-baseline.json`: use-portal 11, aria-role 17, hex 21, rgb 16, oklch-literal 131, arbitrary 2; run today: PASS). A port that hand-writes new menus/dialogs, raw hex or oklch literals will fail the gate. Moving to Halo tokens should drop counts: re-baseline with `--update`. `check:tokens` (`design-system/check-tokens.mjs`) also gates `npm run check` and was not read: unknown how it treats a new token namespace (`--bg-000`, `--ink`, `--line-*`); it will need a map-aware pass.
- **a11y**: keep `role=region` + `aria-label`, divider `role=separator` with `aria-valuenow/min/max`, `aria-label` on all rail/chevron buttons, `aria-current` on active tick, `aria-expanded` on toggles, lightbox `role=dialog aria-modal` + Esc, keyboard activation for `<span role="button">` copy and `code.md-path role="link"` (`Markdown:443-458`). Halo requires 4.5:1 text; `ink-faint` `#7d8592` passes only on bg-000..200, **not bg-300**. Rift's `--fg-faint` (L .43) already fails body-text contrast, so swapping tokens improves it. Reading-comfort selectors (Section 3) must be re-pointed. Touch targets: Halo 44/36px vs Rift 18-30px controls (Q2). Focus ring: Halo `shadow-focus` = 2px surface gap + 2px solid accent; Rift `0 0 0 2px var(--ring)` (translucent).
- **Perf**: reveal pipeline is O(doc) per frame; a Halo-style restyle must not add per-word filters or `:has()` chains; the Halo rule "stagger capped at about a dozen items" is the right guard for tl-node/`--idx`. Long conversations: `.messages` renders all messages (no virtualization), each with a ResizeObserver-tracked parent.
- **WebView2/Tauri**: `backdrop-filter` mis-composites on fixed layers (`app.css:609-611`, Halo `GameFrame` also warns); mask-image on a streaming container; `will-change: transform` on `.composer-host` permanently promotes a layer (`:1160`); inline-span transforms don't work (reveal rise needs `display:inline-block`, which changes wrapping for long tokens/URLs); `ResizeObserver` loop warnings possible when pin writes scrollTop inside RO (`:38`); native child webview bounds sync depends on `.dock-wrap` width tweening (don't convert to transform); CSP blocks Google Fonts import (Geist must be bundled); `localStorage` access wrapped in try/catch in Page but **not** at `AssistantWelcome:125` (read at component init; throws in blocked-storage contexts, only a risk outside WebView2).
- **Security**: Markdown sanitiser (`:587-604`) and the link scheme allow-list (`:395-412`) are load-bearing; the Halo restyle should not touch ALLOWED_TAGS/ATTR or the style hook (only `color|background-color|font-weight|font-style|text-decoration` survive). Shiki inline colours depend on that allow-list; if Halo's "no syntax colour" is chosen the allow-list can shrink.
- **Coupling**: changing `.bubble[data-role]`, `.stream`, `.composer-wrap` class names breaks turn rail, Plan/Activity HUDs, CDP tooling (Section 6). Gallery `/dev/gallery` replays `StreamTurn` (the real surface); it scrolls its own `.stage`, so **`pinToBottom` is not exercised there** (`redesign-plan.md:192`).
- **Parity with ConsoleX port**: ConsoleX already ported Halo to Svelte (`C:/AI Workflow/projects/consolex/panel/design-systems/halo/` holds `bundle.css`, `tokens.css`, `bundle.js`, `index.d.ts`, `react*.js`). Whether its Svelte parts are reusable here was **not inspected** (out of my scope; unknown).

---

## 10 Open owner questions

1. **Legacy assistant bubble**: delete the dormant `MessageBubble` assistant branch (and Stream-view toggle) or port it? Default recommendation from this map: treat stream view as the only chat render and port only the user bubble + system rows.
2. **Density**: Halo controls are 36-44px with 12px minimum sans; Rift's pane chrome is 16-28px with 9-11.5px text. Adopt Halo's sizes in chat chrome, or define a Rift "compact toolbar" tier (and keep the 10px mono chips)?
3. **Prose size/measure**: Halo `body` is 15/22 (mock 15/24, 760px measure). Rift chat is 13/1.68 at 844px. Move to 15px? It changes line counts, code-block scale (Halo code 13/20), and the whole transcript's density.
4. **Syntax highlighting**: keep Shiki github-dark-dimmed (conflicts with "one signal colour", `README.md:10`) or go monochrome like Halo `CodeBlock`? And diffs: need an accent/danger-soft Halo-compatible diff recipe (Halo has none).
5. **Reveal motion** ("the motion the owner called laggy"): the honest levers, one at a time, felt live: (a) keep word-by-word vs reveal by line/sentence chunk vs instant text with a trailing fade only, (b) drop the 500ms blur for opacity-only (compositor safe), (c) remove the extra char pacer, (d) cut the 11-word lag allowance, (e) carry remainder time so cadence is smooth, (f) instant follow pin during streaming vs 0.35 lerp (frame-rate dependent). Not guessing a winner.
6. **Turn rail**: keep (needs Rift-specific part, and a column-edge fix), or drop in favour of Halo-style `FlipList`-free simplicity?
7. **Pane count**: keep up to 4 panes (Halo SplitPane is two-pane, nested), or cap at 2 for the redesign?
8. **User bubble form**: right-aligned bubble (mock) vs full-width band (audit option C, `redesign-plan.md:123-126`)?
9. **Accent control**: Halo accent is a fixed preset; Rift's Settings exposes hue and vividness (`ui-prefs.svelte.ts:99-100`). Keep user-tunable accent (breaks Halo's fixed hex contrast guarantees) or lock to Electric blue?
10. **Fonts**: bundle Geist + Geist Mono (needed under CSP) and drop Inter/JetBrains/Lexend, or keep JetBrains Mono for code?
11. **Semantic green**: Halo has no `ok` token (positive = accent). Done/checked states turn blue; acceptable, or add a Rift `ok` token?
12. **Float material**: jump pill and turn-rail hover use `--shadow-float`; Halo allows `shadow-pop` only on popovers/menus/dialogs/drawers/toasts. Amend the rule or flatten these?
