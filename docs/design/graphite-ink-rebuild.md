# Graphite Ink rebuild — execution map

> **RESUME HERE (2026-06-02):** ALL phases P1–P10 done. P4/P7/P8 took real patches; P5/P6/P9/P10
> were already landed by prior passes (verified, no patch). `npm run check` 0/0. **NOT shipped** —
> no version bump / changelog / build. Next = user eyeball pass, then ship decision.
> Cadence: one phase at a time, ground each against live `src/` first (prior pass landed more
> than the gap brief implied — most phases are verify+small-patch, not full builds), `npm run
> check` per phase, user eyeballs in their own running instance. Dev + CDP are already up
> (`bash scripts/cdp/c.sh health`). Do NOT `cargo check` while dev runs. No Rust in any phase.

> Grounded against live `src/` 2026-06-02 (not the handoff snapshot). Source design:
> `Downloads/.../design_handoff_rift_redesign/` (README + COMPONENT_MAP + DESIGN_TOKENS +
> GAP_REMEDIATION + spec/00-09). **Key finding:** the prior pass landed nearly all
> *functional* wiring. Remaining work is **visual restyle + Home/Onboarding layout** — not
> new plumbing. Trust `src/` for behavior, mockup for visuals.

## Already done (verified live, do NOT rebuild)

OKLCH tokens + emerald accent + neutral graphite + softened radii + motion vars (`app.css`) ·
accent engine + accent/presence/code/density/railPinned (`ui-prefs.svelte.ts`) · `home` added +
`activity` demoted, IA consolidated (`workspace.svelte.ts`, `workspaces/index.ts`) ·
`WorkspaceShell` crossfade + staggered rise + reduced-motion gate · `SettingsPage` all 7
sections (incl. Accessibility + Speech) + scroll-spy + portal `Select.svelte` · `AppShell`
3-row grid + `StatusBar` mounted · TOFU fingerprint prompt · `UpdateDialog` + `checkOnLaunch` ·
`ChatTabsBar` multi-tab + drag-reorder + Ctrl+T/W/Tab · ActivityBar drag-to-reorder.

## Invariants (every phase)

OKLCH not hex · keep Tailwind · extend global primitives (`.btn`/`.card`/`.pill`) don't reinvent ·
no React · no native `<select>` (use `Select.svelte`) · solid accent ONLY Send + 1 CTA/screen,
rest ghost · status LEDs hue-independent (`--ok/--warn/--danger/--info`, never `--accent`) ·
entrances rest at `opacity:1` behind transient class + `prefers-reduced-motion` gate · never
transition `grid-template-columns` fr/minmax (animate child `width`) · extend tokens only,
never rename/remove. **Verify per phase:** `npm run check` (no Rust changes in any phase).

## Phases (dependency order)

- [x] **P1 · `app.css` primitive polish** — confirmed `.btn.primary`/`--accent-fg`, `[data-presence="bold"]`, `::selection` all correct. Fixed dead code-preview controls: wired `--code-fs`/`--code-tab`/new `--code-liga` consumers in `Markdown.svelte` shiki rule, aligned `DEFAULT_CODE.fontSize` 13→12 (zero resting change). `npm run check` clean.
- [x] **P2 · `ActivityBar.svelte` restyle** — active state already correct (verified via CDP: accent icon + 14% ghost fill + thin 3px left bar w/ glow, both collapsed + expanded). Width 48↔220 left as-is (satisfies spec intent). FIXED missing rail animation: registered `@property --rail-w` `<length>` + `html` transition in `app.css` so the value tweens (grid track follows, never transitioned). Verified mid-tween 187px@120ms→220px. `npm run check` clean.
- [x] **P3 · `Titlebar.svelte` restyle** — ⌘K center trigger already live (`.cmdk` → `commandPalette.show()`, verified). Replaced `<img src="/favicon.png">` brand with inline RiftMark SVG (rounded-square `.rm-frame` in fg-55% + two offset `.rm-fault` strokes in `--accent` — the slip btw them reads as the rift). Converted shell flex → `grid-template-columns: auto 1fr auto` (left/right `auto`, center `1fr`). `npm run check` 0/0. CDP-verified glyph + centered pill.
- [x] **P4 · `StatusBar.svelte` restyle** — DONE. Hairline `--border` top (was `--border-strong`), added colored glow to LED data-states (ok/info/danger/stale breathe+glow), update pill → ghost (`--ghost-border` + pill radius + `--accent-soft` hover). CDP-verified, `npm run check` 0/0.
- [x] **P5 · `home/HomePage.svelte` layout** — VERIFIED DONE (prior pass landed it whole). CDP shot confirms honest state-machine hero (offline/attention/clear) w/ 1 solid CTA, Ask-Claude bar, 2-card grid (borderless stat rail + meter + divider + feed / recents), text-link footers. No patch.
- [x] **P6 · `sync/SyncPage.svelte` tabs** — VERIFIED DONE. Already `syncPage.tab` (not `activeTab`); hero hoisted to persistent header (L402) ABOVE 3 tabs (L422); ConflictsPage+ActivityFeed embedded; accent underline is instant (no transition); MIRROR typed-confirm intact. Dated `06-02` comment confirms prior pass. No patch.
- [x] **P7 · `settings/SettingsPage.svelte` polish** — accessibility.set* + uiPrefs.setCode all wired; accent swatch ring-on-select present; fixed-width sticky index (232px) + scroll-spy correct. PATCH: active-row bar 2px→3px + accent glow (spec parity w/ ActivityBar). CDP-verified.
- [x] **P8 · `browser/TwoPane.svelte` restyle** — conflict banner→`setActive("sync")` already wired. PATCH: 3 status LEDs (`.led-modified`/`.sp-modified` in LocalPane/RemotePane/FbDetail) `--accent`→`--info` for hue-independence. NOTE: `280px 1fr` + code-preview are N/A — live Files is a Local↔Remote dual-pane SFTP browser (real behavior), no tree+detail/code-preview surface; forcing them = behavioral rewrite, skipped per "trust src for behavior."
- [x] **P9 · Chat surface** — DEEP-AUDITED vs actual mockup JSX (not just spec md) + patched. Prior passes landed most; re-grounding against `chat.jsx`/`chatthread.jsx` found the live app already had improve-prompt (wand/`enhancePrompt`), word-reveal (`.ew`), context gauge, perm/model pill, thinking, compaction, tool-grouping, Bash-shell renderer, **cross-link** (`jumpTo` scroll+`act-flash`), **dock Plan** (ActivityPanel `tab.tasks` + Running + Outputs + Tool-mix) — often richer than mockup. BUILT the 3 genuine gaps: (1) **Composer Preview (eye)** — toggle + Markdown panel; (2) **TurnSummary** card in MessageBubble — real per-turn files/+adds/−dels (diffArrays on Edit/Write/MultiEdit) + duration + cost + perm-adaptive "Applied automatically"/mode + Review-diff→scroll; (3) **dock context meter** in ActivityPanel — tokens/window bar (warn≥75/crit≥92) off the same `ctxPctFor` getters as the composer gauge. SKIPPED w/ cause: AskUserQuestion **range-ring** (real options are `{label,description}` — no numeric range to bind; would be fake) + **PreviewConsole** (no real Preview/console-stream tool; Bash already renders as shell). All `npm run check` 0/0; Preview + TurnSummary CDP-verified live, meter verified consistent w/ composer gauge.
- [x] **P10 · `onboarding/OnboardingFlow.svelte` overlay** — VERIFIED DONE. onboarding.css header confirms full mockup port: left rail stepper (312px deliberate port width vs mockup 240) + right pane + footer dots, 5 steps, Welcome live accent picker (`uiPrefs.setAccentHue` + ring), reduced-motion gates. No patch.

## Deferrals — need user keep/cut

| Item | Recommendation |
|---|---|
| Web browser dock (`Ctrl+Shift+B`) | Keep deferred, don't break keybind |
| Reupload / suspicious-shrink rebaseline | Keep deferred (functional, restyle later) |
| Bootstrap first-sync detection | Keep deferred (functional, restyle later) |
| Composer dictate → `stt.startListening()` | Keep deferred (visual-only today) — separate wiring task |
| Per-model hue tweak | **Cut** — prototype-only, no `src/` backing |
| Files tree drag-resize | Keep deferred (follow-on) |
| Activity-bar workspace reorder | Already done — no action |

## Risks

`ChatTabsBar` 1268L + `MessageBubble` touches every turn's visual contract → P9 iterative.
SyncPage hero-hoist must not break kebab/mirror flow. `app.css` edits hit everything — extend only.

---

# ROUND 2 — full-mock alignment (`design_handoff_rift_workspace`, 2026-06-02)

> New, fuller handoff bundle from Claude-designs: adds the **complete chat thread** mock
> (`mock/app/chatthread.jsx` 815L + `mock/styles/chat-thread.css` 551L), `MOTION.md` (24-beat
> streaming spec + route cross-fades + reduced-motion contract), 6 screenshots. Tokens =
> identical to round-1 (`rift-theme.css` ≡ app.css Graphite Ink, hex-exact). **Golden rule:
> port the timing/visual MODEL, NOT the React JSX or the 24-beat demo timer** (CLAUDE.md "Do
> NOT port the simulated chat-playback timer"). Drive all motion from the app's REAL streaming
> state (`tab.streaming`, block `status`, `tab.tasks`), which already exists + is wired.

## Round-2 key finding (mapped 2026-06-02, 3 Explore agents)

The chat is **already a wired timeline** — round-1 built parallel structures under different
class names. Remaining = **reskin + grid geometry**, NOT rebuild. Data pipelines all reused:
`MessageBubble.grouped: TimelineUnit[]` → `renderBlock` snippet; `ActivityPanel` reads
`tab.tasks` / `liveActivity()` / `ctxPctFor`; `AssistantPane` `.pane-shell`+`.dock-resize`.
Mock `--gi-*` tokens ≡ app tokens (`--gi-ghost-bg`≡`--accent-soft`, `--gi-ghost-border`≡`--ghost-border`).

### Mock → current class map (for reference while editing)
| Mock (`ct-*`) | Current | File |
|---|---|---|
| `.ct-turn` (grid 26px+1fr) | `.bubble` (grid 2px+1fr) | MessageBubble |
| `.ct-row`/`.ct-marker`/`.ct-cell` | `.tl-node` (no marker lane) | MessageBubble |
| `.ct-rail` (absolute, left:12px) | `.turn-rail` (2px grid col) | MessageBubble |
| `.ct-avatar` (22px, on rail) | `.avatar` (in turn-head) | MessageBubble |
| `.ct-stepdot`/`.ct-editdot` (18px, on rail) | `.tl-stepdot` (absolute) | MessageBubble |
| `.ct-prose` fg-2/1.68 · `.ct-usertext` (NO box) | `.text` · `.text` w/ fg-4% box | MessageBubble |
| `.ct-chip`/`.ct-group` (naked) | `.chip` variant="timeline" | ToolChip |
| `.ct-diff` (editor chrome) | `.edit-diff` | EditDiff |
| `.ct-dock`/`.ct-meter-bar`/`.ct-plan`/`.ct-stream` | `.activity`/`.cm-bar`/`.plan-card`/`.rows` | ActivityPanel |

## Data-model reference (VERIFIED 2026-06-02 — execute from this; do NOT re-map)

> Verifier-confirmed file:line anchors (9/11 ✅, C4 corrected, C11 re-✅). These are the
> load-bearing facts for R2-P1…P5. Source mock: `Downloads/design_handoff_rift_workspace/`
> (`mock/styles/chat-thread.css` 551L + `mock/app/chatthread.jsx` 815L = the design; read for
> exact px/hex. Tokens = hex-exact match to app.css, no `--gi-*` in app).

**ChatMessage / Block** (`src/lib/state/assistant/types.ts:94-96`):
`ChatMessage = { id; role:"user"|"assistant"|"system"; blocks: Block[]; costUsd?; model? }`.
`Block = TextBlock{type:"text";text} | ToolBlock{type:"tool";id;name;input;result;isError;status:"pending"|"done"|"error";startedAt?;durationMs?} | ThinkingBlock{type:"thinking";text;status:"active"|"done";durationMs} | BoundaryBlock{type:"boundary";summary;archivedCount;costUsd;...} | ImageBlock{type:"image";mime;dataBase64}`.

**MessageBubble.svelte** (props `{message, streaming=false, isLast=false}`, MB:154):
- Grid `.bubble` = `2px 1fr` @ **MB:911**; rail `.turn-rail` grid-col 1. → regrid to `26px 1fr`.
- Render pipeline (KEEP): `reconcileSplitHeaders` → `grouped: TimelineUnit[]` (`$derived.by` @ MB:470; kinds `divider`/`block`/`toolgroup`, step-numbered, ≥3 groupable chips coalesced) → main loop MB:731-795 → `renderBlock` snippet MB:666 (image / user-text raw / assistant-text `<Markdown streaming={revealing}>` / thinking `.tn-think` / Edit→`<EditDiff>` / other→`<ToolChip variant="timeline">`). All preserved; only DOM/CSS around it changes.
- Avatar in `.turn-head` @ MB:581 (move onto rail per-row); step nums `.tl-stepdot` `position:absolute;left:-25px` @ MB:1386.
- Assistant prose values (`--fg-2`/1.68) are in **Markdown.svelte** (cont.10, correct — leave). MB `.text` wrapper = `--fg`/1.55 @ MB:1491 (drop box only). USER box override @ **MB:1502-1512** (`--fg-2`+fg/4% bg+8/12 pad) → de-box to `--fg`/1.6.
- Word-reveal LIVE: `Markdown.wrapNewWords` @ Markdown:102, `.md-w` keyframe `md-word` (opacity+blur3→0) @ Markdown:341 ≡ mock `.ct-w`/`ct-word`. `revealing = streaming && !isUser && last text block` @ MB:792.
- Handlers (KEEP): copy MB:298, retry `assistant.retryLast()` MB:617, thinking/group toggles (local Sets), reviewDiff scroll+`tid-flash` MB:387 (tool node id `actnode-${b.id}`). TurnSummary @ MB:808 (shows when `!streaming && turnStats.files>0`).

**ActivityPanel.svelte** (`{tabId}`, AP:30; reads `assistant` singleton + `liveActivity`/`loadCollapsedSections` from `state/assistant/helpers`):
- `tab=assistant.activeTab|tabFor(tabId)`; `messages=tab.messages`; `streaming=tab.streaming`.
- Meter: `assistant.ctxTokensFor/ctxWindowFor/ctxPctFor(tab)`; tone crit≥92/warn≥75; foot = `% used` only @ AP:446 (R2-P2 add `$cost` from `tab.totalCostUsd`). Window 200K, 1M for sonnet-4-5/6+opus-4-7+ (`ctxWindowFor` assistant.svelte.ts:945).
- Plan: `tasks=tab.tasks` (`{id(content-keyed);content;status:"pending"|"in_progress"|"completed"}` assistant.svelte.ts:170) @ AP:257; `data-status` @ AP:516; bar `width:{taskPct}%` @ AP:512. Wired+live.
- Steps: `liveActivity(messages,agentSpawns,mountTs)` (helpers.ts:198 — only `status:"pending"` tools + `completedAt===null` agents) → `displayRunning` w/ 1200ms done-linger; **sorted oldest-first** @ AP:139, **no cap**. → R2-P2: reverse newest-first + 4-cap + "Show N more".
- Header AP:383: title "Activity" + `.dock-stop` Working/Stop (`assistant.stop(tabId)`) + `.dock-x` (`ui.dockOpen=false`). Quickbar AP:396: Copy(`copyTranscript`)/Compact(`assistant.compactConversation`,disabled `streaming||msgs<4`)/Latest(scroll).

**AssistantPane.svelte** (`{tabId,focused,paneIdx}`, AP:14): split `.pane-shell`>`.pane`+`.dock-resize`+`.pane-dock-slot.open`(width:300 @ line449; drag→`ui.dockWidth`, clamp 260-520, persist localStorage `rift.assistant.dockWidth`). `streaming` only to last assistant MessageBubble (line 332). `showEmpty=msgs.length===0`→`<AssistantWelcome>`. Composer always when tabId; `onsubmit`→`assistant.send`.

**ToolChip.svelte** (`{tool,variant="card"|"timeline",caption}`, TC:24): category→`data-category` (read/write/shell/agent/meta); status `data-status` pending(spin+pulse)/done(check)/error. Timeline variant = transparent/no-bar. AskUser options `{label,description}` only TC:51 (NO numeric range → range-ring stays SKIP). EditDiff inline for Edit/MultiEdit.

**EditDiff.svelte** (`{input,compact,defaultExpanded}`, ED:17): `input{old_string,new_string,file_path,replace_all}`. DiffPair ctx/del/add/mod/meta/gap; counts `+adds/−dels` (mod=both); grid `34px 14px 1fr`; has change-bar+pills+line-nums+gap-compaction. MISSING (R2-P3 add): copy btn + open-file affordance (ED:194-238 = breadcrumb+tooltip only). No "writing" state (renders only when strings present).

## Round-2 phases (one at a time; `npm run check` per phase; CDP-verify each vs screenshot)

- [x] **R2-P1 · MessageBubble timeline geometry** ✅ DONE 2026-06-02 (verified vs `02-chat.png`; DOM: rail=avatar=stepdot center all @ 13px). Regrid `2px 1fr`→`26px 1fr` col-gap 12; avatar pulled into lane col 1; rail `justify-self:center`+`margin-top:18px`; `::before`@−30px / `.tl-stepdot`@−33px re-anchored; user prose de-boxed (MB:1502 → `--fg`/1.6); turn gap 18+12px (AssistantPane:660). `npm run check` 0 err.
  Regrid `.bubble` `2px 1fr` → `26px 1fr` (`grid-template-columns: 26px 1fr; column-gap:12px;
  row-gap:13px`). Move `.ct-rail` to `position:absolute; left:12px; top:18px; bottom:4px; width:2px`
  w/ the fade-gradient (chat-thread.css L47-55). Give EVERY row a `.ct-marker` (grid-col 1,
  justify-self center): head row → avatar (22px circle, ghost-bg/accent for Claude, elev-2/muted
  for user); tool rows → `.ct-stepdot` (18px, step num or spinner); edit rows → `.ct-editdot`;
  divider/prose rows → empty marker. **De-box** (VERIFIED 2026-06-02): assistant prose values
  (`--fg-2`/1.68) already live in `Markdown.svelte` per cont.10 — LEAVE THEM; MessageBubble
  `.text` wrapper (MB:1491 `--fg`/1.55) is just a container, drop any box only. The USER box is
  the `.bubble[data-role="user"] .text` override at **MB:1502-1512** (`--fg-2` + `background:
  fg/4%` + `padding:8px 12px`) → change to mock `.ct-usertext`: `color:var(--fg)`, `line-height:1.6`,
  NO bg/padding/border. Turn gap 30px (`.ct-thread gap:30px`; current `.messages gap:14px` +22px
  pre-user — reconcile). Keep `grouped`/`renderBlock` pipeline + all handlers intact. Entrance
  anims scoped to `.streaming` only (resting = solid). Risk: HIGH (every turn). Needs window for verify.

- [x] **R2-P2 · ActivityPanel dock parity** ✅ DONE 2026-06-02. `displayRunning` newest-first + `STEP_CAP=4` + `.rows-more` "Show N more"; cost (`tab.totalCostUsd`) added to `.cm-foot` (flex justify-between + `.cm-cost`). `npm run check` 0 err. ~~Steps: reverse to **newest-first**~~
  + cap at **4 rows** w/ "Show N more" toggle (`.ct-more`); current shows all oldest-first. Verify
  meter (`82K/200K · 41% · $0.63` style — mock shows cost in foot: add `$cost` to `.cm-foot` from
  `tab.totalCostUsd`), plan-card (`Plan · complete 4/4` w/ ring-check icons + in_progress ghost+bar
  — already wired), Working/Stop merged control (already wired). Mostly value alignment. Risk: LOW.

- [x] **R2-P3 · ToolChip + EditDiff** ✅ DONE 2026-06-02. ToolChip timeline variant ALREADY matched `.ct-chip-head` (transparent + faint surface + hover inset accent bar) — no change. EditDiff: split head into chev/crumb/copy buttons, added **copy** (→`new_string`) + **open-file** (`openPath`, `.edit-open` corner-down-left on crumb hover). `npm run check` 0 err. ~~ToolChip timeline variant →~~
  match `.ct-chip-head` (transparent bg, hover `inset 2px 0 accent` bar, naked). EditDiff: add
  **copy** button (`.ct-diff-copy` → clipboard) + **open-file** affordance (`.ct-diff-crumb` hover
  `corner-down-left`) + confirm change-bar/+N−M pills/line-nums (already present). Diff syntax
  highlighting = DEFER (tokenizer, lang-agnostic build — not value-lift). Risk: LOW.

- [x] **R2-P4 · Motion pass** ✅ DONE 2026-06-02 (source-audit; live-loop visuals ride R2-P1 geometry). Tokens exact; word-reveal stagger patched `0.03`→`0.042s` (Markdown:132); route crossfade = faithful model port (`everOpened` snapshot + `.rising`≡`.route-enter` stripped @810ms + 62ms stagger, RM-gated); tid-flash cross-link (act-flash, `block:center`≈96px) + spinner 0.9s confirmed. `npm run check` 0 err. ~~Verify/port the *vocabulary*~~
  word-reveal blur (`md-w` ≡ `ct-w`, 0.042s stagger, 0.5s blur — LIVE, confirm), `ct-enter`
  cell entrance (blur4→0, scoped `.streaming`), `ct-pop`/`ct-halo` avatar, `ct-dotpulse` pending
  stepdot, `ct-ev-in` dock event entrance, `ct-rail-pulse` streaming rail, plan self-check
  (derives from `tab.tasks` — LIVE), dock↔thread `tid-flash` cross-link (≡ `act-flash`/`jumpTo` —
  LIVE, confirm 96px offset). **Route cross-fades (MOTION §1)**: confirm `WorkspaceShell` crossfade
  + `.route-enter` strip-after-820ms + per-screen stagger (round-1 has crossfade+stagger — verify
  vs spec). Reduced-motion: every entrance/loop gated `@media (prefers-reduced-motion: no-preference)`,
  resting DOM = solid end-state. Risk: LOW (mostly verify; small gaps patch).

- [x] **R2-P5 · Cross-screen pixel pass.** ✅ DONE 2026-06-03 (CDP-shot home/sync/files/settings @
  1600×1000, `npm run check` 0 err). **ONE true gap patched:** home decoy composer lacked the mock's
  model pill → added `.da-model` (● + `MODEL_LABELS[assistant.model]`, e.g. "Opus 4.8") before
  `.da-send` in `HomePage.svelte` (display-only mirror of default model). Verified vs `01-home.png`.
  **Sync** (`03`): faithful + superset via Design-preview fixture (kebab → "9 changes need a sync"):
  amber hero, Drift/Conflicts/Activity accent-underline tabs, color-coded PUSH/PULL/DELETE/CONFLICT
  chips, Sync now w/ counts, Apply selected + footer, grouped resource table w/ tone tags. Conflict
  card present in source (gated `connection.conflictCount`, fires on real conflicts; preview routes
  conflicts to `syncPage.entries` so absent in fixture — not a gap). Subline = arrow format (round-1
  choice, kept). No patch. **Files** (`04`): real SFTP dual-pane — mock tree+detail N/A per round-1
  P8, no match attempted. No patch. **Settings** (`05`): faithful superset (title + "Workspace ·
  {name}", nav, version card, identity card, on-machine toggles, accent swatches; live adds
  Accessibility/Speech/First-run). "Offline" vs mock "Connected" = state only. No patch. 3 deferrals
  stay skipped. Env note: live SFTP unreachable (Offline) → connected Sync verified via preview
  fixture, not a live handshake. Risk realized: LOW.

### Round-2 deferrals (real-data gaps — keep skipped w/ cause, from round-1 P9)
- **AskUserQuestion range-ring** — mock binds a numeric `dist`/`r` per option; real options are
  `{label, description}` only. No numeric range to bind → ring would be fabricated. SKIP.
- **PreviewConsole** (`ct-preview` live server console) — no real Preview/console-stream tool in
  the app; Bash already renders as `.terminal` shell. SKIP unless such a tool ships.
- **Diff syntax highlighting** — needs a real lang-agnostic tokenizer (mock `hiLua` is Lua-only
  demo). Build later; not a value-lift.

### Execution gate
Dev window currently **collapsed to 146×20px** — CDP `shot` unusable. R2-P1/P5 need pixel verify;
**restore window to normal size before executing**. R2-P2/P3/P4 are source-safe but should still
shot-confirm. Do NOT `cargo check` while `rift-tauri.exe` dev runs (no Rust in any R2 phase anyway).
