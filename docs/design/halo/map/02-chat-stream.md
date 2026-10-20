---
title: Halo map 02 — chat stream (assistant turn tool/activity blocks)
status: map only, read-only wave; no source touched
updated: 2026-10-19
branch: design/chat-ledger (HEAD 22727d24; Ledger = 8bb71f4f)
---

# 02 · Chat stream — tool / activity blocks

Path abbreviations (all under `C:/AI Workflow/projects/rift-tauri/`):
`S/` = `src/lib/components/assistant/stream/` · `A/` = `src/lib/components/assistant/` · `CSS` = `src/lib/styles/stream.css` · `APP` = `src/app.css` · `H/` = Halo brand book root (scratchpad `…/artifact-files/705c5425…/project/`) · `HTOK` = `C:/AI Workflow/projects/consolex/panel/design-systems/halo/tokens.css` · `HCSS` = `H/components/bundle.css`.
"Measured" = counted from source this session. "Unknown" = not verified.

---

## 1 Files & roles

| File | Lines | Role |
|---|---|---|
| `S/StreamTurn.svelte` | 554 | Turn orchestrator. Live: chronological blocks + footer. Done: answer, then Ledger receipt (L425-493). Owns live status (verb, secs, tokens, stall, compaction). |
| `S/streamModel.ts` | 1159 | Pure model: `messageToTurn` L724, `groupBlocks` L846, `segmentWork` L898, `adaptTool` L552, `answerStartIndex` L876, `classifySay` L1086, ANSI L244-298, reveal tiers L673-721, `VERB_PAST/ING` L1115-1125. Tested by `streamModel.test.ts` (1011 L, pure-logic; no markup selectors). |
| `S/StreamShell.svelte` | 299 | Shell/terminal block (IN row + OUT well), typewriter command, cursor, peek/full/minimal modes. |
| `S/WorkLine.svelte` | 113 | Grouped quiet tool rows: read/grep/glob/list_dir/mcp (+ shell with no output). Live = single compact "active" tile. |
| `S/BlockHeader.svelte` | 118 | Shared head contract `[lead][title][meta: pill·dur·copy·chev]`; used by Shell, Result, **and** history ToolChip. |
| `S/OutputBlock.svelte` | 300 | Shared output body (both trees): tiers collapsed(12)/expanded(40)/all, head-tail fold, ANSI per-line, cursor, full-output **inspector drawer** (L169-190). |
| `S/ReadResult.svelte` | 95 | Shiki-highlighted file body + CSS-counter gutter. |
| `S/GrepResult.svelte` | 139 | Clickable `path:line: text` rows, match `<mark>`, cap 12, FilePathMenu. |
| `S/WriteBatch.svelte` | 133 | Edit/Create/Delete batch: pill + path + animated +N/-M + live dot; per-row inline `EditDiff`. |
| `S/StreamThinking.svelte` | 27 | One-row collapsible reasoning ("Thinking…" / "Thought for Ns"). |
| `S/StreamPlan.svelte` | 49 | Inline plan card (progress + checklist, fold completed ≥8 items). |
| `S/PlanHud.svelte` | 329 | Floating pinned plan bar (only while inline card off-screen); linger 4s. |
| `S/ActivityHud.svelte` | 459 | Floating pinned agents+shells bar; per-PID kill; whole-turn Stop. |
| `S/StreamAgent.svelte` | 328 | Inline sub-agent card: head, live "now" line, expandable step timeline, nested child rail, result. |
| `S/StreamAskUser.svelte` | 427 | Interactive ask_user card (radio/checkbox options, Other, submit/dismiss, answered chips, expired, failed). |
| `S/StreamExitPlan.svelte` | 180 | Plan-mode proposal + approval gate (Approve/Ask/Edit/Refine/Discard). |
| `S/StreamResult.svelte` | 55 | Test/lint run: pill (N passed/failed), auto-open on failure. |
| `S/StreamWeb.svelte` | 32 | Web search/fetch compact tile + source chips. |
| `S/AnimatedCount.svelte` | 113 | rAF odometer (tween 420ms + glyph roll) for tokens/secs/+N/-M. |
| `CSS` | 555 | Shared `.sturn*`, ledger, footer, wline, wbatch, splan, sxplan, sweb, sresult, shimmer, keyframes. Loaded globally by `S/StreamTurn.svelte:2`. |
| `A/ToolChip.svelte` | 773 | **History / non-stream** tool block (per-tool icon, kv input, result by style). Routes Agent/Todo/Ask to `toolchip/*`. |
| `A/toolchip/AgentCard.svelte` | 159 | History agent card (prompt blockquote + result md). |
| `A/toolchip/TodoCard.svelte` | 125 | History TodoWrite checklist card. |
| `A/toolchip/AskUserCard.svelte` | 503 | History ask card (near-duplicate of StreamAskUser; own CSS). |
| `A/EditDiff.svelte` | 682 | Unified line diff (word emphasis, Shiki, gap collapsing, line cap, file crumb → FilePathMenu). Used by WriteBatch (`compact hideHead defaultExpanded`, `S/WriteBatch.svelte:91`) and MessageBubble (`A/MessageBubble.svelte:415,418`). |
| `A/bubble/TurnSummary.svelte` | 141 | History-tree turn receipt tile (files, +/-, dur, cost, fast, auto/bypass chip). Non-stream only (`A/MessageBubble.svelte:544-546`). |
| `A/bubble/BoundaryBlock.svelte` | 149 | Compaction divider pill (cli-source vs summary-source; streaming/expanded body). |
| `A/bubble/ModelSwitchBlock.svelte` | 76 | "Model switched A → B" divider pill. |
| `A/bubble/helpers.ts` | 379 | `isCardTool`, `isInlineDiffTool`, `coalesceToolGroups`, etc. (history tree). |

Mount points: `A/AssistantPane.svelte:716-738` (StreamTurn when `uiPrefs.streamMode && role==="assistant"`, else MessageBubble); HUD stack `A/AssistantPane.svelte:748-751`, CSS `:1166-1177`; scroll region `.stream` `:1181-1195`. Dev harness: `src/routes/dev/gallery/+page.svelte` + `src/lib/dev/galleryFixtures.ts` (16 scripted sections, L56-194 — the existing state catalogue / visual-QA rig).

**Two render trees.** `uiPrefs.streamMode` default `true` (`src/lib/state/ui-prefs.svelte.ts:105,161`), togglable (`:235`). Stream tree = everything above except ToolChip/TurnSummary. History tree (ToolChip, TurnSummary, MessageBubble thinking `tn-think` `A/MessageBubble.svelte:386`, toolgroup `:460-496`) still ships and shares BlockHeader/OutputBlock/ReadResult/GrepResult/EditDiff. Any Halo restyle of those shared parts hits both.

### Ledger receipt (8bb71f4f) — what it is, where
Commit touched `StreamTurn.svelte`, `stream.css`, `MessageBubble`, `AssistantPane`, `Composer`. Present state:
- Answer first: `.sanswer` L426-431 renders only trailing `say` groups (`answerStartIndex`, `S/streamModel.ts:876`).
- ONE ruled line `.sledger` (`CSS:393-394`, `.ruled` = 1px top border at 60% `--border`) with `data-outcome` attr (`S/StreamTurn.svelte:434`; attr has no CSS consumer in `CSS` — unknown elsewhere).
- Row L435-472: toggle button (glyph ✓ / ⚠, title, facts) + hover-only tokens + hover-only Copy/Retry.
- Title map (L63-75): `bgAgents>0` "Agent working in background" · `applied` "Changes applied" · `failed` "Changes failed" · `planned` "Plan prepared" · actions>0 "Work completed" · else "Reasoning".
- Facts (L443-449): `N file(s)` if applied else `N action(s)`; `fmtDur`; cost `$x.xx` if >0; `fast` zap.
- No-activity variant: facts only (L452-458). Whole ledger omitted if nothing to say; whole turn `display:none` when empty (`.sturn.is-empty`, `CSS:390`, `isEmpty` L260-262).
- Expanded: `.sactivity-body` on `--tile-well` (`CSS:417`), `slide 180ms` (L474), optional "N terminal results · Expand/Collapse output" bulk bar (L475-482 → `bulkOpen/bulkVersion` into every StreamShell).
- Facts typeset Inter tabular `--fs-xs`, `--fg-subtle` (`CSS:406-407`). Pointer: owner rejected mono chrome meta (C13, `docs/design/redesign-plan.md:60`).
- Not rendered at all in stream tree: `turn.meta` (computed `S/streamModel.ts:790`, pinned by tests `streamModel.test.ts:18,26,85`, no read in StreamTurn). Candidate write-only state; do not delete without test edit.

### Live footer (verb · secs · tokens)
`S/StreamTurn.svelte:495-517` `.sfooter` = `[dot] [verb] [· secs · ↑tokens]`.
- Verb (L302-314): compacting → "Summarizing older messages"; live tool → `VERB_ING[kind]`; else stall tiers (≥20s "Waiting on the model", ≥60s "Still waiting for a response", ≥150s "No response — ending soon", L286-301); else "Thinking…" (reasoning live) / "Working…".
- Awaiting ask: clock + tokens suppressed, hint "— choose an option above to continue" (L501-504); dot accent steady, verb accent 600 (`CSS:191,247-250`).
- Secs/tokens via 1s ticker (L187-197) + `AnimatedCount` (L509,513; secs `durationMs=300`).
- Hidden while a pure reasoning pass owns status (`thinkingNow` L244-246 → footer gated L495).
- Compaction: head row L405-410 + progress card L518-539 (scaleX bar, ~% / messages / tokens / ETA, hedged copy L528-538). Auto-compaction is **inferred** (L203-226: near trigger ×0.9 + 12s silence). Stall notes L540-551.

---

## 2 Layout & anatomy

Column: `.stream` scroll (`A/AssistantPane.svelte:1181`, padding 16/18/8, hidden scrollbar) → `.messages` → per-turn `.sturn` (`CSS:46`, margin `--stream-gap-between-turns` 22px (18 compact / 27 comfy, `CSS:20-23`)). Message column shares composer edges (844px per commit msg 8bb71f4f; width not re-verified in source).

Within a turn, vertical rhythm = `--stream-gap` 12px (10/15 by `data-density`), turn chrome `--stream-gap-turn` 16px.

**Live turn** (`S/StreamTurn.svelte:412-419`): `.sturn-body.railed.live` (13px left pad + 1.5px gradient rail, accent-tinted tail `CSS:51-64`; rail only if any work group or thinking, L86) → [StreamThinking] → groups in arrival order → `.sfooter` → (compaction card | stall note). Pinned HUDs float above (see below).

**Done turn**: `.sanswer` → `.sledger` (rule + row) → [`.sactivity-body` expanded: bulk bar + `.sturn-body` (thinking + activity groups)].

**Group dispatch** (`renderGroup`, L327-398):

| Group / seg | Component | Condition |
|---|---|---|
| `say` | `.snarr` (Markdown) / `.snarr-beat` (muted dot line) / hidden | `sayMode` L134-143: live-last or latched → full; `prose` full; pref chatty/focused/balanced (filler hidden, connective muted). Latch `liveShown` L123-128 so watched text never collapses. |
| `steer` | `.ssteer` dashed right-aligned stub + thumbs + "mid-turn" tag | mid-turn user message |
| rich `plan` | StreamPlan (items from block or live aggregate; last turn only, L96-97) | coalesced consecutive plan blocks (`segmentWork` L909-912) |
| rich `web`/`fetch` | StreamWeb | |
| rich `test`/`lint` | StreamResult | shell cmd matching runners (`shellCheckKind` L75) |
| rich `agent` | StreamAgent (+spawn, +childSpawns from `liveTab.agentSpawns`) | |
| rich `ask` | StreamAskUser | |
| rich `exitplan` | StreamExitPlan | |
| rich `shell` | StreamShell (+`poll` count) | shell pending OR has output (`isRich` L891-894); 3+ identical → one card (`coalescePolls` L933) |
| `edit` seg | WriteBatch | edit/create/delete runs |
| `other` seg | WorkLine | read/grep/mcp/finished no-output shell |
| per tool w/ pending ask | `PermissionBar` under it | `pendingPerms` L107-110 |

`TKind` set: read, grep, edit, create, delete, shell, agent, web, fetch, test, lint, mcp, plan, ask, exitplan (`S/streamModel.ts:18-20`). `nameToKind` L321-347 (Glob/list_dir fold into grep/read; TaskOutput/TaskStop → shell; Task*/TodoWrite → plan; all else → mcp).

### Block catalogue — anatomy (live vs done)

| Block | Anatomy | Live | Done |
|---|---|---|---|
| **StreamThinking** | `Brain` 13px + label + chevron (`S/StreamThinking.svelte:16-19`). Boxless. Body = Markdown italic faint 64ch (`CSS:178`). | label "Thinking…", icon accent, text muted (`CSS:170-171`); static (comment `CSS:168`) | "Thought for Ns"/"Thought"; disabled button when no text (encrypted-only) |
| **WorkLine active** (1 tool, pending) | `.tile.is-compact.is-live` `[dot][VERB][dir/cap]` (`S/WorkLine.svelte:55-60`) | dot pulses, verb shimmer (`CSS:328-342`), accent border | — (becomes the grouped row) |
| **WorkLine group** | `.tile.wline` fit-content → head `[glyph][summary][failed pill][count pill][dur][chev]`; open → 30px-indented ledger rows `verb · dir/cap → meta  dur`, rows w/ body expand to ReadResult/GrepResult/OutputBlock (`S/WorkLine.svelte:62-111`) | glyph live tint if any active | frame red only if ALL failed; partial = "N failed" pill (L33-38); width morph fit→100% on open (`CSS:260-264`, `interpolate-size`) |
| **StreamShell** | `.tile.sshell`: head = `[prompt glyph $/PS>/>][typed Shiki cmd][poll ×N][cursor]` + `[pill ok/failed/secs][dur][copy][chev]`; well: "output" label + line count + OutputBlock (`S/StreamShell.svelte:168-217`) | typewriter 120-320ms (L90), blinking cursor in head → "tty-wait" cursor in empty well (L111), elapsed pill ticking, prompt glyph pulses+glows (L232-236), output lands whole → per-line cascade + parked cursor 750ms (L116-127) | pill ok/failed, dur if ≥1s, copy shows after run |
| Shell modes | `commandOutput` pref: minimal (no body) · peek (last 3 lines, `outputPeek` L649) · full (auto-open live, `S/StreamShell.svelte:29,132-144`); `toolDetail==="detailed"` forces full | | `historical` receipt opens collapsed (L143) |
| **StreamResult** (test/lint) | `.tile.sresult`: BlockHeader `[flask/shield glyph][cmd][pill "651 passed"/"3 failed"][dur][copy][chev]`; body OutputBlock head-tail (`S/StreamResult.svelte:34-54`) | no pill while running (comment L24); glyph live | failures auto-open (L19-21) |
| **WriteBatch** | `.wbatch` framed card (`CSS:347-351`); rows `[chev][pill new/edit/deleted][dir/name][+N −M][dot]`; open → `.wb-diffwrap` well w/ EditDiff (`S/WriteBatch.svelte:61-96`) | live dot (accent pulse), `+N` rolls (`AnimatedCount live`), label shimmer (`CSS:332`) | green dot w/ glow (`CSS:368-369`); ≤14 changed lines (60 for create) auto-open (L18-23,28-32); plan artifacts folded (L29-31) |
| **EditDiff** (inside) | optional crumb head (chev · file crumb → FilePathMenu · op pill · LANG · +N −M · copy); body = unified lines `[num?][+/−][code]`, word-level emphasis, gap rows, line-cap button (`A/EditDiff.svelte:371-430`) | rows slide in staggered 16ms×min(i,14) (L570-575) | |
| **StreamPlan** | `.tile.splan`: `[ListChecks glyph]Plan [track] n/m` + list `[mark][text]` (`S/StreamPlan.svelte:21-49`) | active row ring spin + shimmer text; fill width transition | done rows struck + `dotPop`; ≥8 items & >2 done → "N completed" fold (L11-13) |
| **PlanHud** | `.phud.hud-float` bar 32px `[ring/✓/list][active text][72px track][n/m][chev]`; expand → list (`S/PlanHud.svelte:172-209`) | only `streaming && total>0 && !allDone && !planCardInView` (L99) | all done → green "Plan complete" linger 4s once (L85-89) |
| **ActivityHud** | `.ahud.hud-float` bar `[Bot/Terminal/✓][headline · n shells][done/total][chev][Stop]`; expand rows per spawn (click → jump) + per shell PID (hover kill) (`S/ActivityHud.svelte:229-314`) | visible while spawn/shell live and inline cards off-screen (L110-111) | linger 4s |
| **StreamAgent** | `.tile.sacard` head `[Bot][type pill][desc][N steps][tok][dur][status glyph][chev]`; "now" line (`CornerDownRight`/`Brain` + label + italic snip); body = timeline (steps w/ status glyph + tool icon + caption + beat dot + dur; thinking italic; prose; depth-2 `.sa-kid` rail; step errors) + result Markdown (`S/StreamAgent.svelte:102-193`) | live border, spinner glyph, bouncing dots (thinking), heartbeat dot, tail-follow scroll (L88-99) | ✓/⚠ glyph |
| **StreamAskUser** | `.tile.sask` states: failed (compact "couldn't be shown"), live (eyebrow + dot, options as radio/checkbox tiles, Other + input, Dismiss/Submit, hints), forming ("Preparing…"), answered (chips), dismissed, expired (inert) (`S/StreamAskUser.svelte:148-314`) | `.is-ask` accent edge; dot pulse | plain tile |
| **StreamExitPlan** | `.tile.sxplan`: head `[ScrollText]Proposed plan[chev]`, Markdown body, action bar (`S/StreamExitPlan.svelte:117-180`) | typewriter reveal 20 chars/30ms (L33), caret blink, "Drafting plan…", icon pulse | live ask (`.is-ask`) or settled-actionable fallback (L50-54) |
| **StreamWeb** | `.tile.is-compact.sweb`: `[Globe]Searched/Fetched <query>` + ≤5 source chips w/ hue dot (`S/StreamWeb.svelte:17-31`) | italic faint query while pending | **`sources` is always `[]`** — `S/streamModel.ts:583` — chips never render live today (count also null) |
| **Steer** | `.ssteer` dashed stub | | |
| **Permission** | `PermissionBar` (`A/PermissionBar.svelte`, not in this scope) | | |
| **Boundary / ModelSwitch** | divider line + pill; boundary has streaming/expanded body | `Compacting · n messages` live dot | expandable summary Markdown |
| **TurnSummary** (history only) | `.tile` row: files · +/- · clock · cost · fast · auto/bypass chip | | |
| **ToolChip** (history only) | `.tile.is-compact` BlockHeader + body: kv input + result by `resultStyle` (terminal/code/list/plain) | pill "running…" | opacity .85 when done, 1 on hover (`A/ToolChip.svelte:537-538`) |

**Stream-tree gap (verified):** the `max_tokens` ("Response cut off — Continue") and `refusal` notices exist only in `A/MessageBubble.svelte:524-542`; `grep` for those strings in `src/` finds only MessageBubble. Stream mode has no equivalent (unless surfaced via a path not grepped, e.g. AssistantPane error banner — unknown).

HUD stack: `.hud-stack` absolute top 10px centred, `min(560px, 100%-36px)`, z-index 4, gap 8 (`A/AssistantPane.svelte:1166-1177`). z-order on screen: composer/alerts < HUD stack (4) < OutputBlock inspector scrim (z 1800, portalled `S/OutputBlock.svelte:172,264`).

---

## 3 States

| State | Where | Evidence |
|---|---|---|
| Empty turn | `display:none` | `CSS:390`, `S/StreamTurn.svelte:260` |
| Forming (input streaming) | caption provisional ("running…", "searching…"), diff counts deferred, ask "Preparing…", plan "Drafting…" | `S/streamModel.ts:364-370,567-577`; `S/StreamAskUser.svelte:49,196`; `S/StreamExitPlan.svelte:47` |
| Pending/live | `is-live` border tint + glyph live + dot/verb/shimmer | `app.css:534`; `CSS:328-342` |
| Done ok | pill ok/passed; green dot on WriteBatch; ✓ glyph | |
| Error | `.tile.is-bad` red border + 2px inset edge (`app.css:535-538`) + red pill + red glyph (3 signals for 1 fact: C10 in plan) | |
| Partial failure | WorkLine count pill only (`S/WorkLine.svelte:33-38,67`) | |
| Awaiting user | `.is-ask` accent edge; footer clock frozen/hidden | `app.css:539-543`; `S/StreamTurn.svelte:178,501` |
| Expired ask | inert tile, "turn ended before answered" | `S/StreamAskUser.svelte:46,168-176` |
| Stall tiers | footer verb warn + note ≥20/60/150s | `S/StreamTurn.svelte:286-301`, `CSS:209-214` |
| Compacting (manual/auto) | head row + progress card; stall suppressed | `S/StreamTurn.svelte:182-239,405,518` |
| Hover | tile border → `--tile-border-hover`; chev recolor; copy icon opacity 0→1 (BlockHeader `.bh:hover .bh-copy`, `S/BlockHeader.svelte:112`); ledger tokens+Copy/Retry on `.sturn:hover`/`:focus-within` (`CSS:412-416`) | hover-only actions: **not discoverable on touch/keyboard-less**; keyboard path via `:focus-within` ok |
| Focus | `.tile-head:focus-visible` 2px ring (`app.css:595`); `.sledger-toggle`, `.msg-act`, `.sactivity-tools button` ring `--ring` (`CSS:402,428,442`); many bare buttons rely on default/outline (e.g. `.wr-click` has none, `CSS:281-285`) | |
| Disabled | `BlockHeader` `disabled={!expandable}` (cursor default), StreamThinking bare, ask options `opacity:.55` | |
| Collapsed/open | Shell `open/touched` + bulk (`S/StreamShell.svelte:132-144`); WorkLine `userOpen` + per-row `bodyOpen` map; WriteBatch `openMap`; ledger `activityOpen`; Output tier | **no persistence** across remount; keyed `{#each gi}` index keys (L418,426) |
| Reduced motion | `prefersReducedMotion()` one-shot probe (`src/lib/state/accessibility.svelte.ts:103`; not reactive to OS change) + CSS `@media` blocks everywhere (`CSS:372-384,445`) | typewriter/ride/AnimatedCount/slide honour it; Svelte `slide` in StreamTurn L474, StreamShell L211, StreamResult L51 does **not** (no `reduced ? 0` guard) — measured; WriteBatch/EditDiff/ToolChip do guard |
| Narrow window | only `@media (max-width:720px)` hides ledger tokens (`CSS:434-436`) and inspector `<=620px` full width (`S/OutputBlock.svelte:297`); WorkLine/Shell truncate with ellipsis; `.hud-stack` clamps to `100%-36px`. No stacking behaviour for ledger row, wbatch row, agent head (flex-nowrap + ellipsis). Unknown real behaviour <500px. | |
| Density | `data-density` compact/comfy scale 3 gap vars only (`CSS:22-23`); other paddings fixed | |
| Narration pref | focused/balanced/chatty; toolDetail minimal/balanced/detailed; commandOutput minimal/peek/full; presets calm/standard/verbose (`ui-prefs.svelte.ts:70-73`). **9 prefs-driven variants multiply the QA matrix.** | |

---

## 4 Motion present

Token vocabulary today (`APP:239-250`): `--ease-page` cubic-bezier(.22,1,.36,1) · `--ease-soft` (.4,0,.2,1) · `--ease-spring` (.34,1.56,.64,1) · `--dur-fast` 140 · `--dur-base` 240 · `--dur-rise` 460 · `--dur-slow` 520 · `--pulse-live` 1.6s · `--stagger` 62.
Halo (`HTOK:43-48`): `--dur-fast` 150 · `--dur-base` 300 · `--dur-slow` 600 · `--stagger` 60 · `--ease-out` (.2,.8,.2,1) · `--ease-spring` identical (.34,1.56,.64,1). Halo page change 520ms ease-out, order change 460ms, skeleton→content ~360ms (`H/03-motion.md`). Halo rule: loops only for LiveDot, Spinner, indeterminate Progress; continuous motion = opacity/transform only (`03-motion.md:17,19`; `04-performance.md:5`).

Verdict key: **K** keep · **T** retune to Halo token · **X** kill · **P** port to Halo component.

### 4a Entrances / reveals

| Where | What / property | Dur · ease · trigger | Verdict |
|---|---|---|---|
| `CSS:27` `blockIn` (used by `.snarr`, `.snarr-beat`, `.ssteer`, `.sfooter`, `.sstall-note`, `.scompact-card`, `.wline`, `.wline-active`, `.wbrow`, `.splan`, `.sxplan`, `.sweb`, `.sresult`, `.sshell`, `.sacard`, `.sask`, `.sturn-head`) | opacity + translateY **3px** | `--dur-base` 240 · `--ease-page` · mount | **T** — Halo `halo-rise` is 14px/600ms/ease-out (`HCSS:23`). Rift cut 8→3px on purpose: follow-scroll chasing growth read as "lurching" (`CSS:25-26`, commit 22727d24). Recommend: stay ≤4px inside the live column; 14px rise only for the whole turn's first mount / welcome. Owner call Q1. |
| `CSS:33` `workOpen` (wline-list .3s, wline-row .24s, wr-out .24s, sa-body .3s) | opacity + translateY(-4px) | 240-300ms literal seconds · `--ease-page` · open | **T** → `dur-base` 300 + Halo accordion pattern (`grid-template-rows 0fr→1fr`, content fades up after 60ms delay, `HCSS:499-504`) |
| `CSS:30-31` `bodyFadeIn` on `.ssh-outwrap>*`, `.sr-out>*` | opacity | 200ms, 40ms delay · `--ease-soft` | **T** (fold into accordion content fade) |
| `S/StreamTurn.svelte:474`, `S/StreamShell.svelte:211`, `S/StreamResult.svelte:51`, `S/WriteBatch.svelte:89`, `A/EditDiff.svelte:406`, `A/ToolChip.svelte:479` | Svelte `slide` height | 180 / 140 / 140 / 200 / 200 / 180ms, default ease (cubic-out), WriteBatch/EditDiff/ToolChip reduced-guarded, **first three not** | **T/P** — six different durations for one gesture; unify to Accordion-style `dur-base`; add reduced guard |
| `CSS:88`+`dotPop` `CSS:83-87` | spring scale 0→1.55→1 | 420ms · `--ease-spring` · `.sh-dot` mount, `.splan-item.is-done .splan-mark`, `.sask-option.selected .sask-marker` (`S/StreamAskUser.svelte:390`) | **K/T** — Halo reserves spring for "things you flip, grab or add"; check-pop on done plan rows and option select fit; head dot pop is status → drop |
| `CSS:88` `headRise` | translateX(-8px) fade | 380ms spring · `.sh-label` (compaction head only) | **X** |
| `S/StreamShell.svelte:73-105` typewriter | text reveal char-by-char | clamp(len×14, 120, 320)ms, per-char jitter 0.6-1.5×, wall-clock catch-up, only when mounted pending | **K** (signature, tiny) but Halo has no typewriter; keep as Rift-specific; guard exists |
| `S/OutputBlock.svelte:119,205-207` `termLineIn` | per-line opacity + translateY(2px) | `--dur-base`, delay min(i×16, 400)ms · mount (per `.term-line`, up to ~40+ lines) | **T** — stagger 60ms cap ~dozen per Halo (`03-motion.md:7`); current 16ms×N up to 400ms ≈ 25 lines. Perf: N animated DOM nodes per block |
| `S/StreamShell.svelte:116-127` ride | cursor parked after output | 750ms timer | **K** |
| `A/EditDiff.svelte:570-575` `diff-row-in` | translateX(-4px) (deliberately no opacity, L563-569) | `--dur-base`, delay 16ms×min(i,14) | **T** (stagger 60 cap 12 → or drop for diffs; diff rows are content) |
| `S/WriteBatch.svelte:125` `wb-rise` | opacity + translateY(5px) | `--dur-rise` 460 | **T** |
| `S/OutputBlock.svelte:292-293` inspector | scrim fade 160ms ease-out; panel translateX(18px)+fade 180ms `--ease-page` | open | **P** → Halo `Drawer` (`halo-drawer-in` translateX 40px, `HCSS:363`) |
| `S/PlanHud.svelte:241-244`, `S/ActivityHud.svelte:343-346` | translateY(-10px) scale(.96) | `--dur-base`; exit `fade 160ms` (L173/231); `.hud-stack > .phud + .ahud` 90ms delay | **T** → `halo-pop-in` family (scale .96 translateY(-4px), `HCSS:411`) / Toast fan; keep exit fade |
| `phud-open`/`ahud-open` | list rises -3px | `--dur-base` | **T** → accordion |
| `S/StreamAgent.svelte:271-275` `sa-row-in` | opacity + translateY(3px) per new timeline row | .22s | **K/T** (Halo list-item stagger; no stagger needed live) |
| `S/StreamExitPlan.svelte:33` reveal | 20 chars/30ms (~660 chars/s) | | **K** (Rift-specific) |
| `CSS:233` `.scompact-fill` | scaleX | 900ms `--ease-soft` | **T** → Halo Progress (`scaleX` transform already; value ease) |
| `A/bubble/TurnSummary.svelte:64` `fade` in 160ms | | | **K** (history tree) |

### 4b Continuous / looping (27 `infinite` declarations in scope, measured)

| Where | What | Property | Verdict |
|---|---|---|---|
| `CSS:40-44` `streamLivePulse` on `.sh-dot` `CSS:80`, `.sf-dot` `CSS:188`, `.wa-dot` `CSS:321`, `.wb-dot.live` `CSS:370`, `.sask-dot` `S/StreamAskUser.svelte:342` | expanding ring | **box-shadow** (paint, not compositor) · `--pulse-live` 1.6s | **P** → `LiveDot` (`halo-ping` scale 1→3 + opacity, 1.8s, `::after`, `HCSS:80-82`) — transform/opacity only; one component replaces 5 hand-rolled dots |
| `CSS:341` `shim` text-gradient sweep on live verb/label (5 selectors L329-333) | `background-position` animation, clipped to text | paint-bound, travels | **Conflict.** Halo allows no text shimmer (loops = LiveDot/Spinner/Progress). DESIGN §8 explicitly grandfathers "owner-approved accent `.shim` on single live VERB text" (`DESIGN.md:233-234`). Owner Q4. Default recommendation: keep on footer verb only; kill on `.wbrow.active .wb-label` and `.splan-item.is-active .splan-text`, `.wa-text`. |
| Spinners: `ringspin` `CSS:34` (splan-ring, ahud-spin), `bh-spin` `S/BlockHeader.svelte:108`, `phud-spin` `S/PlanHud.svelte:268`, `sxa-spin` `CSS:504`, `sask-spin` `S/StreamAskUser.svelte:423`, `chip-spin` ×4 (`A/ToolChip.svelte:626`, `A/toolchip/AgentCard.svelte:158`, `TodoCard.svelte:122`, `AskUserCard.svelte:502`), `sa-spin` uses ringspin, `phud-ring`/`splan-ring` CSS ring + `Loader2` icon | rotate 0.9-1.1s linear | **P** → one `Spinner` (900ms, 3px track + 18/100 dash arc, `HCSS:447-450`). Measured: 9 separate rotate keyframes + 3 different visual spinners (CSS ring, Loader2, `phud-ring`). |
| `S/StreamShell.svelte:232,236` `sshPromptPulse` | prompt glyph opacity 1↔.45 + text-shadow glow 9px | 1.4s | **X** the glow (Halo: `glow-strong` only under hovered primary button, `01-themes-and-colour.md:34`); fold into LiveDot beside title |
| `S/StreamShell.svelte:266`, `S/OutputBlock.svelte:214` `sshBlink`/`termCurBlink` | 7×13 block cursor, steps(1) 1.06s | opacity | **K** (terminal idiom, DESIGN §8 endorses idle cursor `DESIGN.md:230-232`); Halo CodeBlock has no cursor → Rift-specific part |
| `S/StreamAgent.svelte:237` `sa-dot` (3-dot), `sa-beat` L288, `A/toolchip/AgentCard.svelte:124` `agent-dot` | opacity (agent-dot also scale) | 1.1-1.6s | **X/P** → one Spinner or LiveDot(info); 3-dot "typing" is a second vocabulary |
| `CSS:513-516` `sxplan-writepulse`, `sxplan-blink` caret | opacity | 1.5s/1s | **K** caret (real drafting), **X** icon pulse |
| `CSS:466` `.splan-ring` | spinner | | **P** Spinner |
| `S/ActivityHud.svelte:412` | row spinner | | **P** |

### 4c Hover / state transitions

| Where | Property · dur | Verdict |
|---|---|---|
| `.tile` `APP:531` | border/bg/shadow `--dur-base` `--ease-soft` | **T** → Halo card hairline `line-100→line-200` dur-base ease-out (`HCSS:30-31`); Halo hover lift 3px only for clickable cards — not for blocks |
| `.tile-chev` `APP:592` | rotate 90° `--dur-fast` | **T** — Halo accordion chevron flips 180° dur-base (`HCSS:497-498`); Rift = ChevronRight→90°. Pick one disclosure glyph app-wide (Q8) |
| `.bh-copy` opacity 0→1 | `--dur-fast` | **K** |
| `.sledger-hover/.sledger-acts` opacity | `--dur-fast` | **K** (keyboard via focus-within) |
| `.sfooter .sf-verb-wrap` verbIn | opacity crossfade `--dur-fast` `--ease-soft` | **K** (Halo "content arrives; never pops") |
| `AnimatedCount` | rAF tween 420ms ease-out cubic + glyph roll `--dur-fast` (`S/AnimatedCount.svelte:98,35`); footer secs `durationMs=300` | **P** → `RollingNumber` (per-digit wheels, "right-most first", `H/components/RollingNumber/README.md`). Gap: AnimatedCount is a whole-number tween, not odometer; `format` fn (fmtTokens "1.2k", fmtDur "1m 4s") — Halo RollingNumber supports `format` fn |
| `.phud-fill`, `.splan-fill` width transition `--dur-base` | width (layout) | **P** → Halo Progress (`transform: scaleX`, `HCSS:459`) |
| `.wline` width morph | width `--dur-base` | **K?** layout animation of width; fine but unique. Keep or drop (Halo cards don't morph width). Q-low |
| Svelte `fade` HUD out | 160ms | **K** |
| ask option / button transitions | `--dur-fast` border+bg | **T** → Halo Button/RadioGroup |

Perf notes (measured counts): 27 infinite animations; 2 `backdrop-filter` in this scope's HUDs (`S/PlanHud.svelte:224-225`, `S/ActivityHud.svelte:326-327`) + ToolChip `.terminal-cmd` (`A/ToolChip.svelte:730`) + BoundaryBlock/ModelSwitch pills (`:113`,`:63`) — DESIGN §10 states backdrop-filter is banned (WebView2) (`DESIGN.md:297`, scrim rule; wording is about scrim — treat as project-wide per commit 22727d24 message "§10 bans it"). Halo itself uses `game-glass` blur only for Game category; Product category has none.

---

## 5 Styling wiring

**Mix.** Shared global CSS (`CSS` 555 L + `app.css` `.tile*` L525-606) + component-scoped `<style>` in 14 of 18 stream/toolchip/bubble svelte files + zero Tailwind utility classes in these files (measured: classes are hand-named `s*`/`wr-*`/`wb-*`/`sa-*`/`ssh-*`; Tailwind only supplies the `@theme` token bridge `APP:72-111`). Several components reach into others' markup via `:global` / descendant selectors: `.sshell :global(.bh)` (`S/StreamShell.svelte:245`), `.sresult .bh` (`CSS:547`), `.tile.is-compact :global(.bh)` (`A/ToolChip.svelte:581`), `.wr-out .oblock-out` (`CSS:305`), `.sactivity-body .sturn-body` (`CSS:429`). BlockHeader's `.bh` is not `.tile-head`, so every host re-applies tile padding.

**Token consumption (measured over stream/toolchip/bubble svelte + `CSS`):**
`--accent` 88 · `--fg-2` 55 · `--fg-muted` 48 · `--fg-faint` 74 · `--fg-subtle` 34 · `--ok` 35 · `--danger` 31 · `--warn` 13 · `--info` 6 · `color-mix` ≈ 160 · `tile-*` references 171.
Tile recipe tokens `APP:217-231`: `--tile-fill` = fg 2.8% mix (translucent, not a lightness step), `--tile-well` = `--bg-inset` 70% mix, `--tile-border` = `--border`, `-live` accent 30% mix, `-bad` danger 40%, `-ask` accent 42%, `--tile-edge` 2px.

**Hard-coded counts (this scope):**
- Colour literals in `.svelte` (check:ui metrics scan only `.svelte`, `check-ui.mjs:99-101`): hex-regex hits = 2 in `S/StreamShell.svelte` (both are `#100` comments, L81,97 — they count toward the `color:hex` ratchet!) ; `rgb(`: 2 in `S/OutputBlock.svelte` (L266 scrim `rgb(0 0 0 / .48)`, L273 shadow); oklch literals: `S/OutputBlock.svelte` 2 (magenta/cyan ANSI L225-226), `S/StreamWeb.svelte` 1 (favicon hue L11), `CSS` 1 (not scanned). Baseline `design-system/ui-baseline.json`: hex 21, rgb 16, oklch-literal 131, arbitrary 2, portal 11, aria-role 17.
- Raw `px` tokens: `CSS` 219, ToolChip 47, EditDiff 55, AskUserCard 53, StreamAgent 38, PlanHud 32, ActivityHud 29, StreamShell 27, StreamAskUser 26, BoundaryBlock 26.
- Raw font-size px literals in scope (non-token): 11px ×9, 10px ×7, 9.5px ×6, 11.5px ×6, 10.5px ×6, 12px ×3, 12.5px ×3, 13px ×2 = 42 vs token uses `--fs-xs` 43, `--fs-sm` 35, `--tile-meta-fs` 13. Size ladder in use: 9.5/10/10.5/11/11.5/12/12.5/13 + `--fs-xs` 11 + `--fs-sm` 12 + `--fs-md` 13 → ~11 distinct sizes in a 12-13px UI. Halo: `caption`/`small`/`body` ladder; no sub-11 sizes except mono `label`.
- Radii (measured): 50% ×14, `--radius-sm` ×12, 999px ×11, `--radius-xs` ×6, literal 6px ×6, `--tile-radius-inner` ×5, `--radius` ×5, 2px ×4, `--tile-radius` ×3, literal 8/5/4/3/1.5/7/14/10 ≈ 11 literals. Halo: sm 8 · md 12 · lg 16 · pill.
- Raw ms/s literals in stream scope: `CSS` 22, StreamTurn 14 (mostly `setTimeout`/delays), PlanHud 4, ToolChip 4, AgentCard 4, StreamAgent 6.
- Fonts: `--font-ui` Inter Variable + `--font-mono` JetBrains Mono Variable (`APP:257-258`); Halo = Geist + Geist Mono (`HTOK:60-61`). Font swap is a global decision (not stream-only).
- Mono usage: chrome meta (pills, dur, counts, plan count, ssh-poll, `wline-row` entire row in mono `CSS:277`) — Halo says mono for numbers/labels/data (`02-type-space-and-layout.md` §Type); plan C13 said mono-for-chrome was a defect. Tension, Q3.

**Sizes (px):** tile head 7×11 pad (`--tile-head-pad`), compact 5×10; glyph 18px; pill mono 10px; shell cmd 12px; output 11px/1.55; read gutter 4ch; HUD bar 32px; copy hit 20×20 (below Halo 44px touch target and any usable target); `.msg-act` min-height 26; `.sledger-toggle` min 28. Halo targets 44px (`02-type-space-and-layout.md:15`); desktop density wins, flag Q9.

---

## 6 Restyle boundary

### KEEP (logic — do not touch in restyle)
- `S/streamModel.ts` entirely (kinds, grouping, `answerStartIndex`, polls, `classifySay`, ANSI parse, reveal tiers, `diffCounts`, adapter memo `adaptCache` L543 — perf-critical, #100).
- StreamTurn script L1-324: `liveTab` (not global `assistant.*` — split-pane correctness L25-31), ticker, `liveTool`, `awaitingInput`, compaction inference, stall tiers, `liveShown` latch, `sayMode`, `pendingPerms`, `copy()`, `assistant.retryLast(tabId)` L467, `activityOpen`/`outputBulk*`.
- StreamShell: typewriter wall-clock catch-up (#100), `ride`, open/touched/bulk effect, `mode` from prefs, `copyText`, Shiki `cmdHtml` + `whenReady`.
- OutputBlock: tier state machine, inspector focus/Esc handling (`window keydown capture`), `lineDelay`, caps (`OUTPUT_CHAR_CAP`), index keys (`(i)`, comment L203 — duplicate-key crash history).
- GrepResult: `invoke("resolve_workspace_path")` L84, FilePathMenu flow, regex guard. ReadResult: Shiki guard + `parseReadOutput`.
- WriteBatch: `openMap`, `autoOpen`/`SMALL`/`SMALL_CREATE`, `diffInputs` (MultiEdit split), `isPlanArtifact`.
- PlanHud/ActivityHud: linger latches (`sawLive/lingered`), `$effect.pre` ordering (TDZ + focus-bounce comments L43-90, L146-167), IntersectionObserver duplication guard (selectors `.csurf-col`, `.stream`, `.splan`, `#sacard-<id>`, `.pane-shell textarea` — **markup contracts**: `S/ActivityHud.svelte:162,176-183,222`, `S/PlanHud.svelte:110-116,154`), `assistant_kill_shell` invoke L139, `assistant.stop()` L151.
- StreamAgent: `agentNowLine`, tail-follow, spawn/childSpawns wiring, `id="sacard-<tool.id>"` anchor (HUD jump target).
- StreamAskUser / StreamExitPlan: all state + `assistant.submitAskUserAnswer`, `answerPlan`, `approvePlanFallback`, `refinePlanPrefill`, `permissionPromptFor`, reveal timer, edit/refine flows.
- AnimatedCount tween math (or replace wholesale by RollingNumber port — logic is only the rAF tween).
- `data-density`, `data-outcome` hooks; `.sturn` class (CDP tooling selects `.sturn`: `scripts/cdp/serve.cjs:876-877`).

### CHANGE (markup / CSS)
- All of `CSS`; `.tile*` in `app.css` (shared with the rest of the app → see wave-wide decision); every scoped `<style>` block in the 18 stream/toolchip/bubble files; BlockHeader markup (pill/dur/copy/chev cluster).
- Replace hand-rolled primitives: 5 pulse dots, 9 spinners, 3 progress bars (`.phud-track`, `.splan-track`, `.scompact-track`), 2 hand-made drawers/scrims (OutputBlock inspector), kbd/pill/badge (`.tile-pill`), copy buttons (`.bh-copy`, `.msg-act`, `.edit-copy`, `.oi-btn`), collapsible chevrons (≥6 variants: `.tile-chev`, `.wb-chev`, `.wr-chev`, `.splan-fold-ch`, `.chev` in Boundary, `.sxplan-chev`).
- Ledger markup/CSS (Ledger *behaviour* stays).

Dual-tree rule: BlockHeader/OutputBlock/ReadResult/GrepResult/EditDiff restyle changes both trees; ToolChip/TurnSummary/AskUserCard/AgentCard/TodoCard only matter if non-stream mode survives (Q6).

---

## 7 Halo mapping table

Token translation (Rift → Halo, `HTOK`): `--bg`/island → `bg-000/100`; `--tile-fill` → `bg-100` (card) ; `--tile-well`/`--bg-inset` → `bg-300` (inset) or `bg-000` (terminal); `--border` → `line-100`; `--border-strong`/hover → `line-200`; control borders → `line-300`; `--fg`/`fg-2` → `ink`; `--fg-muted`/`fg-subtle` → `ink-muted`; `--fg-faint` → `ink-faint` (**floor; labels/timestamps only; never on `bg-300`** — Rift uses faint 74× in this scope incl. body-ish text like `.sa-now-snip`, `.sthink-text`, `.snarr-beat`, `.wline-row`); `--accent` → `accent`; `--accent-soft` → `accent-soft`; `--ok` → **no Halo equivalent** (positive = accent); `--warn/--danger` → same names; `--info` → in `blue` theme `info` = **lime #c6f35a** (`HTOK:64-69`) so blue-for-info assumptions break; `--radius-lg` tile → `radius-md` 12 (controls/popups) or `radius-lg` 16 (cards); `--ring` → `shadow-focus`.

| Rift element | Halo component / pattern / token | Notes / gap |
|---|---|---|
| `.tile` card shell (shell/result/agent/plan/ask/exitplan) | `Card` (bg-100, line-100→200 hover, radius-lg, **pad space-5 = 24px**) | Card pad 24px far too big for a 7×11 tool header; need a **Rift-specific dense tile** using Card's surface/hairline/hover only. Gap G1. |
| `.tile.is-compact` row | none (Card is not a row) | Build as dense Card variant. G1. |
| `.tile.is-live/.is-bad/.is-ask` edge | none | Halo says state = no heavy borders; closest: `Banner` tone tint (8% tone over bg-100) or `accent-soft`. 2px inset edge has **no Halo precedent**. G2 / Q5. |
| `.tile-pill` (ok/failed/passed/new/edit/deleted/agent type/poll/step) | `Badge` neutral/accent/info/warn/danger (+dot) (`HCSS:229-235`) | Halo Badge: sans 12/18, pad 3×10, pill radius. Rift pill is mono 10px 1×7. Map ok/passed→accent, failed/del→danger, edit→accent?, new→accent, neutral for agent type. **`ok`≠green in Halo.** Edit-vs-new both accent would collide: need `new`=accent, `edit`=neutral. |
| `.tile-glyph` 18px status tile | none; `Spinner` (live) / icon in tone colour | Glyph bg tint = colour wash on a tiny tile; Halo "no colour washes" → icon only. |
| Live dots (`.sf-dot`, `.sh-dot`, `.wa-dot`, `.wb-dot`, `.sask-dot`, plan active) | `LiveDot` (accent/info/warn/idle; label required) | Direct 1:1. Halo: "always pair with text label" — footer/verb satisfies; WriteBatch row dot needs label via row text. |
| Spinners (9) | `Spinner` (size 14 in mock) | 1:1. Replace `Loader2` + CSS ring. |
| Footer verb · secs · tokens | `LiveDot` + `RollingNumber` (+ mono per mock `Main.dc.html:119-123`) | 1:1 for dot/number. Shimmer on verb: no Halo equiv (Q4). |
| `AnimatedCount` | `RollingNumber` (`value`, `format` fn) | `format` accepts a function `(value:number)=>string` (`H/components/index.d.ts:756-762`), so `fmtTokens`/`fmtDur` plug in; `value` is a number only. Unknown: ≥1Hz live-update cost in Svelte (Halo is React; unknown whether the ConsoleX port includes RollingNumber). |
| Progress bars (`.phud-track`, `.splan-track`, `.scompact-track`) | `Progress size="sm"` (4px, scaleX, indeterminate sweep) | 1:1. Compaction card: indeterminate vs honest estimate — keep determinate+hedge. |
| Plan card checklist | `Stepper vertical` ? or Card + list | `Stepper` is "3–6 steps" with labels + track fill (`H/components/Stepper/README.md`); Rift plans are 1-30+ items with fold → **Stepper doesn't fit long lists** (G3). Use list rows w/ Halo check-draw (`halo-draw-check`, `HCSS:546`) + Progress header. Stepper only for PlanHud if ≤6 (no). |
| Terminal block (Shell) | `CodeBlock kind="terminal"` (bg-000, line-100, radius-md, `$ `/`> `/`# ` prompts ink, output ink-muted, copy w/ accent "Copied"/danger "Copy failed", `title`, `maxHeight`, `wrap`) | **Gaps (G4):** no `PS>` prompt recognition; no ANSI colour (Halo "never colours syntax" — ANSI is output colour; Rift currently paints ok/err/warn per line + 16 SGR colours); no collapse tiers / head-tail fold / "N hidden" divider; no status pill, duration, poll tag, live cursor, tty-wait, typewriter; copy always visible vs Rift hover. Keep Rift terminal body (OutputBlock) and re-skin onto CodeBlock tokens; use CodeBlock only for surface/typography/copy recipe. |
| Read result | `CodeBlock title language lineNumbers` (`highlight` for lines) | **Gap (G5): Halo CodeBlock has NO syntax colouring by design** (`CodeBlock/README.md`: "No syntax colouring: Halo keeps one signal colour") vs Rift ReadResult/EditDiff/Markdown Shiki. Q2. Gutter: Halo `3ch` + space-4, number `ink-faint`; Rift CSS-counter 4ch. Start-line offset (`parseReadOutput`) maps to `start` prop; `highlight: number[]`, `wrap`, `maxHeight`, `copy`, `onCopy` all exist (`H/components/index.d.ts:903-925`). |
| Grep results | none | **Gap G6**: no search-results list component. Build Rift-specific `ResultList` from `DataTable`-less rows: path (mono, ink) `:line` (ink-faint) + text, `<mark>` = `accent-soft` bg (Halo `highlight` uses accent-soft line wash — consistent). |
| Diff view | none (CodeBlock `highlight` = accent-soft line wash only) | **Gap G7 (largest)**: Halo has no diff component and no +/- tokens. Rift needs a Rift-specific `DiffBlock`: add/del line washes, word emphasis, gap rows, gutter. Colours: add = accent-soft (blue) or keep `ok` green? del = danger-soft. Q5. |
| WriteBatch row pill + counts | `Badge` + mono `+N −M` via `RollingNumber` | |
| Agent card | `Card` + `Badge neutral` + `Spinner/LiveDot` + `Accordion` for timeline (items with `meta` mono right: dur) | **Gap G8**: no "run/step timeline" or nested-run component; `Accordion` gives open/close motion only. Depth-2 child rail (`.sa-kid`) = Rift-specific. |
| WorkLine group | `Accordion` (`variant` hairlines or `cards`, `meta` for count, chevron) | Collapsed state is a fit-content chip (width morph) — Accordion is full width. Decide: keep chip (Rift-specific) or go full-width row (Halo-native). |
| Thinking row | `Accordion` single item, or bare row + `LiveDot` | Boxless today; Halo `Accordion` default = hairline rows → consistent. |
| Ledger receipt | `Divider` (hairline) + mono stats + `Button variant="ghost" size="sm"` Copy/Retry (exactly what `Main.dc.html:84-91` mocks) | Halo ghost Button height 44/36 (sm) vs Rift 26px `.msg-act` → density gap (Q9). `SectionLabel` not applicable. |
| HUD bars (Plan/Activity) | `Popover`/`Banner`/`Card spotlight`? none exact | Closest: floating `Card` with `shadow-pop` + `Progress sm`. Halo explicit: backdrop blur not in Product category; frosted 94% surface → solid `bg-200` (raised) + `shadow-pop`. Stack/entrance → `halo-pop-in`. G9 (floating status bars). |
| Output inspector drawer | `Drawer` (`side` right, width 420 → needs ~760, `title`, focus trap, `scrim`) | Replace hand-rolled `use:portal` + `role="dialog"` (2 check:ui ratchet hits). Drawer default 420 "caps it" → pass `width` 760. 1:1 + width. |
| Ask user card | `Card` + `RadioGroup`/`Checkbox` group + `TextField` + `Button primary/ghost` + `Badge` | Halo RadioGroup "one of a few with explanations" = exact fit (label+description). Gap: Halo Checkbox "waits for a button" fits multiSelect+Submit. Good native fit. Radius `--radius`→ md. |
| ExitPlan card | `Card` + Markdown + `Button` row (primary Approve, secondary, ghost) + `Textarea` (edit) + `TextField` (refine) | Native fit. One primary per view rule: "Approve & build" is the one. |
| Compaction card | `Banner` (info) + `Progress` | Banner = "belongs to the page, stays until fixed" ~ ok semantically; or Card+Progress. |
| Stall notes | `Banner warn` inline or plain muted text | Rift = muted note; keep calm. |
| Web tile + source chips | `Badge neutral` / `Kbd`-like chip; favicon hue dot | Hue-hashed favicon dots = second chromatic voice (DESIGN §8 bans, `CSS:536` uses oklch per domain); Halo one-signal → neutral dot. Also `sources` never populated (G10). |
| Steer stub | none; dashed border | Rift-specific, keep (neutral `line-300` dashed). |
| Boundary/ModelSwitch dividers | `Divider label=` ("Conversation compacted") | **1:1** — Halo Divider supports a centred uppercase caption (`Divider/README.md`). Pill chrome + glow + blur removed. |
| TurnSummary tile | ledger line / `Badge` for auto/bypass (`warn` tone for bypass) | |
| Copy buttons | CodeBlock copy recipe ("Copied" accent / "Copy failed" danger) | Rift already does failed state (BlockHeader L50-55). |
| Tooltips (`use:tooltip`) | `Tooltip` (120ms delay, rise 4px, inverted bubble) | Rift's own action; restyle to Halo. |
| Skeleton / loading | none in stream today | Halo `Skeleton` not applicable to streaming; do **not** add (Halo: "skeleton inside cards"). Gap noted: waiting is cursor/timers by DESIGN §8. |
| Entrance rise | `halo-rise` util (14px/600/stagger 60) | See 4a — tension. |
| Page change | n/a | |
| `Toast` | existing Rift `notify` | out of scope. |

Gaps needing Rift-specific parts (consolidated): **G1** dense Tile/row shell · **G2** state edge vocabulary · **G3** long checklist · **G4** terminal output (ANSI, folds, cursor, live meta) · **G5** syntax colouring policy · **G6** search-result list · **G7** diff block · **G8** agent timeline + nested rail · **G9** floating HUD bar · **G10** web sources data gap · **G11** ask/permission "needs you" emphasis without edge (Halo Banner-ish).

---

## 8 Pain points (evidence)

1. **Primitive duplication.** 9 rotate keyframes + 3 spinner looks, 5 hand-rolled live dots, 3 progress bars, ≥6 chevron variants, 4 near-copies of "copy" buttons (counts above; `S/BlockHeader.svelte:108`, `S/PlanHud.svelte:268`, `CSS:34,504`, `A/ToolChip.svelte:626`).
2. **Two trees, two vocabularies.** History ToolChip/TurnSummary/AskUserCard/AgentCard/TodoCard re-implement stream cards; `AskUserCard` (503 L) ≈ `StreamAskUser` (427 L). Stop-notice exists in one tree only (§2).
3. **Motion tuned by feel, not tokens.** 6 slide durations (140-200ms), workOpen at literal `.24s/.3s`, stagger 16ms vs token 62ms, HUD entrance 240ms scale .96 vs Halo pop-in. Last two commits are motion de-escalation (`22727d24` msg: "streaming felt laggy", blockIn 8→3px, glint removed) — direction = calmer.
4. **Loops everywhere.** 27 `infinite` declarations; box-shadow ring pulse (paint) ×5; text shimmer ×5 selectors; plan-doc screenshot `04-gallery-tiles.jpeg` shows glowing prompts + coloured dots + coloured pills in one frame.
5. **Kind-coloured chrome persists** (plan C9): `.ssh-prompt.bash/.pwsh/.cmd` = ok/info/warn colours + glow (`S/StreamShell.svelte:233-235,255-257`); commands Shiki-coloured in head (L60-66); contradicts DESIGN §7 "glyph colour = status, never kind" (`DESIGN.md:126`) and the owner's own Ledger rationale.
6. **Status vocabulary still drifts** (plan C10): shell "ok/failed", test "651 passed/3 failed", ask "answered/awaiting reply/connecting…" (history), agent glyph only, WriteBatch green dot + "new/edit/deleted"; failed tile = border + edge + pill + glyph.
7. **Type/size entropy:** ~11 font sizes, ~11 literal radii, 42 raw `font-size px` in scope; mono used for row text (`CSS:277`), pills, durations.
8. **Fact repetition** (DESIGN §8 "say each fact once"): ReadResult head comment admits "same facts three deep" (`S/ReadResult.svelte:9-13`); WorkLine row meta ("→ N lines") + group count + head summary; footer previously repeated tool caption (commit 22727d24).
9. **Dead/odd data:** `turn.meta` unused in stream render; `StreamWeb` sources always `[]` (`S/streamModel.ts:583`); `data-outcome` no CSS.
10. **Hover-only controls** (Copy/Retry/tokens/`.bh-copy`/`.ahud-kill`): invisible at rest by design; contrast of `fg-faint` meta is low-by-token (plan measured 1.57:1 earlier; Ledger fixed meta only).
11. **Hit targets 18-28px** (`.bh-copy` 20, `.ahud-kill` 18, `.wb-chev` ~12 no padding `S/WriteBatch.svelte:99-105`).
12. **HUD complexity**: two 300-450-line components built on IntersectionObserver + DOM queries into other components' markup; pinned over scroll region.
13. **Index-keyed `{#each gi}`** for groups (`S/StreamTurn.svelte:418,426,487`) — groups re-key as blocks regroup; Svelte transitions/`slide` on keyed index nodes can replay; intentional (comment L123-128) but fragile.

---

## 9 Risks

**Tests / tooling**
- `streamModel.test.ts` (1011 L) is pure-logic: no class/markup selectors (grep'd stream/ and `*.test.ts` for `sturn|sledger|sshell|wline|…` → no markup hits in tests). Restyle should not break vitest; any model change (e.g. dropping `turn.meta`) will (`:18,26,85`).
- Markup contracts outside tests: `scripts/cdp/serve.cjs:876-877` selects `.sturn` and `.bubble`; `scripts/cdp/c.sh look "<selector>"` pattern + `.claude/skills/rift-ui` / `.agents/skills/rift-ui` document selectors (not read: unknown which). HUD IntersectionObservers depend on `.csurf-col`, `.stream`, `.splan`, `#sacard-*`, `.pane-shell` (§6). **Keep these class/id hooks or update all consumers in the same change.**
- Visual QA rig: `/dev/gallery` replay (16 sections) is the only broad regression surface — `src/routes/dev/gallery/+page.svelte`, `src/lib/dev/galleryFixtures.ts`; fake tab omits fields (`agentSpawns: []`, `shellRows` absent) so **HUDs and StreamAgent spawn states are not exercised** by it (measured from `+page.svelte:11-18`).
- `check:ui` ratchet (`design-system/check-ui.mjs`): counts only go DOWN; baseline hex 21 / rgb 16 / oklch 131 / arbitrary 2 / portal 11 / aria-role 17. Raw literals in new CSS-in-svelte count; `.css` files not scanned (stream.css/app.css free). Dropping OutputBlock's `use:portal`+`role="dialog"` in favour of Drawer lowers portal/aria-role (needs `--update`). `#100` issue-number comments are counted as hex (StreamShell ×2) — harmless but don't add `#123`-style comments. `check:ui` also hard-fails if `components/ui/` classes generate no CSS.
- `npm run check` = svelte-check + `check:tokens` + `check:ui` (`package.json:10`). `check:tokens` mirrors `app.css` ↔ `design-system/styles.css` + `_ds_manifest.json` (plan Phase 1 note, `docs/design/redesign-plan.md:147`); new Halo tokens must be added in both or it fails.
- Cargo untouched; frontend-only (`CLAUDE.md` rift-tauri: `npm run check`/`npm test`).

**a11y**
- Many `div`/`span role="button"` copy affordances (`S/BlockHeader.svelte:74-84` span with `role="button"` **nested inside a `<button>`** — invalid interactive nesting) — Halo says every control is a real `<button>`; restructure meta cluster (needs sibling, not child).
- `ssteer`, `.ssh-prompt` are decorative; `aria-live="polite"` on thinking label (`S/StreamThinking.svelte:18`) re-announces every ticker? (label changes only on state flips) ok.
- Status via colour + word mostly present; `wb-dot` colour-only (live vs done) — Halo requires text/icon pair (`06-accessibility.md:8`; `01-themes-and-colour.md:32`).
- Focus ring styles inconsistent (§3 table); Halo `shadow-focus` = 2px gap + 2px accent.
- Contrast: `fg-faint` carries real content (74 uses) while Halo floor `ink-faint` = labels/timestamps only; Halo claims 4.5:1 for ink-faint on bg-000/100/200 but **not on bg-300** — Rift terminal wells (`bg-300`-like inset) must not carry faint text.
- Reduced-motion probe is one-shot, not live; Svelte `slide`/`fade` transitions bypass CSS `@media`.
- HUD focus-rescue logic (`rescueFocus`) depends on `.pane-shell textarea`.

**Perf**
- `messageToTurn` runs per stream delta; `adaptCache` WeakMap is the guard (L543). Any restyle that adds per-token reactive work, or replaces memoized props with fresh objects, re-introduces the #100 wedge.
- Per-line animated `.term-line` (up to dozens/block), per-diff-row animations, 27 infinite loops, `box-shadow` pulses ×5, backdrop-filter HUDs, `background-position` text shimmer: all paint-bound. Halo rule (compositor-only continuous motion) improves this if ported faithfully; React-origin `RollingNumber`/`FlipList` DOM-per-digit costs on 1Hz footer + diff counts are unknown in Svelte.
- Shiki highlight on read/diff/shell head (`{@html}` of Shiki output; `S/ReadResult.svelte:53`, `S/StreamShell.svelte:182`) — removing syntax colour (Halo CodeBlock) would also remove main-thread cost; keeping it needs the 500KB guard (`OUTPUT_CHAR_CAP` 60k).
- `interpolate-size: allow-keywords` on `.wline` (`CSS:261`) — Chromium-only; WebView2 version dependent (unknown support level; falls back to jump).
- Layout animations: Svelte `slide` (height) and `.wline` width morph; Halo accordion also animates `grid-template-rows` (layout) — same class of cost, fine at this scale.

**WebView2/Tauri**
- `backdrop-filter` known to mis-composite (DESIGN §10, `DESIGN.md:297`); HUDs/pills use it (§4 list) — Halo Product has none; removing is aligned.
- Fixed/portal layers: OutputBlock scrim z 1800 hand-set; Halo wants `z-overlay` 50 tokens; stacking vs app dialogs unknown.
- Pre-JS `#first-paint` mark fixed-position bug already hit gallery (22727d24) — any global background change needs `app.html` check.
- Fonts: Geist/Geist Mono must be bundled locally (Halo loads from Google Fonts `04-performance.md:9`); Tauri app offline-first → self-host (Rift already self-hosts Inter/JetBrains Variable; unknown font pipeline for Geist).

**Scope/regression**
- `.tile*` lives in `app.css` and is consumed app-wide (Settings, composer, Markdown, etc. — not scanned here); stream-only restyle needs a namespaced class (e.g. `.hx-tile`) or a coordinated wave.
- Reduced-motion + pref matrix (3×3×3 + density + streaming vs history) = ≥ 54 visual variants; gallery covers presets, not HUD/agent/permission/boundary.
- Ledger is brand new (HEAD~1, unpushed, branch `design/chat-ledger`): owner just picked it 2026-10-19; Halo redesign must **preserve its information architecture** (answer first, one rule, hover actions) and only re-skin it, otherwise it overrides an explicit pick.

---

## 10 Open owner questions

1. **Entrance travel.** Halo `rise` = 14px/600ms. Rift cut to 3px/240ms two commits ago because the live stream "lurched". Keep ≤4px inside the live column and use Halo's 14px only on first mount / welcome? (recommended)
2. **Syntax colour.** Halo CodeBlock "never colours syntax" (one signal colour). Keep Shiki in Read results, diffs, Markdown fences, shell commands — or go monochrome (+ accent highlight) in the stream and keep colour only in Markdown fences?
3. **Mono vs sans for meta.** Halo mock puts receipt stats in Geist Mono (`Main.dc.html:87`); the owner's Ledger decision moved chrome meta from mono to tabular Inter (C13). Which wins for receipt, durations, pills, counts?
4. **Live-text shimmer.** DESIGN §8 grandfathers `.shim` on the single live verb; Halo allows loops only for LiveDot/Spinner/Progress. Keep on footer verb only, or retire all text shimmer for `LiveDot`+label?
5. **Status colours + diff colours.** Halo has no green; positive = accent (blue in `blue` theme), negative = warn/danger, and `info` turns lime in `blue`. Do pass/ok/added become accent-blue? Do diff add/del keep green/red (extra tokens) or become accent-soft/danger-soft? Does the 2px inset left edge (is-bad/is-ask) survive or become a tone tint / Banner-style?
6. **Non-stream (history) tree.** `streamMode` toggle still exists (`ui-prefs.svelte.ts:235`). Retire it (delete ToolChip/TurnSummary/AskUserCard/AgentCard/TodoCard, ~2k lines) or restyle both trees?
7. **Tile density.** Halo Card is 24px padded; tool blocks are 5-7px rows. Accept a Rift-specific dense tile built from Halo hairline/surface tokens (recommended) — yes?
8. **Disclosure glyph.** Rift: ChevronRight→90°; Halo Accordion: chevron rotates 180° (down→up). Pick one app-wide.
9. **Hit targets.** Halo 44/36px vs Rift 20-28px desktop density; keep Rift densities for stream chrome (Halo `size="sm"` 36px ghost buttons in the ledger)?
10. **WorkLine shape.** Keep the fit-content chip that widens on open (unique, width morph) or become a full-width Accordion row (Halo-native)?
11. **Stop notices in stream mode.** `max_tokens` / refusal notices exist only in the history tree — should the stream tree get them (as Halo `Banner` inline) as part of this wave? (behaviour gap, not styling)
12. **Web source chips.** `sources` is never populated; show nothing (remove chips) or wire real data (backend/model work)?
13. **HUDs.** Keep both pinned bars (heavy IntersectionObserver logic) as Halo floating cards, merge into one status bar, or drop PlanHud (inline plan card already exists)?
14. **Hover-only actions.** Keep Copy/Retry/tokens hover-only (Ledger pick) given Halo's "no hidden-only" a11y stance, or show at `ink-faint` at rest?
15. **Geist font.** Is adopting Geist/Geist Mono app-wide decided (this wave assumes the shell wave owns it)?
