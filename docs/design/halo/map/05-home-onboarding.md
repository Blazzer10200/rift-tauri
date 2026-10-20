# 05 — Home / workspace hub / news / onboarding (map, read-only)

Branch `design/chat-ledger`. Scope: `WorkspacePage`, `NewsFeed`, `PageHero`, `LimitBar`, `onboarding/*`, `styles/onboarding.css`. Nothing run, nothing edited.

Notation: paths are under `C:/AI Workflow/projects/rift-tauri/` unless prefixed `halo/` = `C:/Users/BLAZZER/AppData/Local/Temp/claude/C--AI-Workflow-projects-rift-tauri/3cd5d05a-bb71-48d8-8d23-bc53e60827d8/scratchpad/artifact-files/705c5425-86e3-4761-a421-4ee89499e5c6/project/`. `bundle.css` = `halo/components/bundle.css`. `tokens.css` = `C:/AI Workflow/projects/consolex/panel/design-systems/halo/tokens.css`. Screenshot of current hub: `docs/design/redesign-audit/07-home-workspace.jpeg` (viewed; 6 project cards 4-up, 5,914 hero numeral, green bars, 5-hue model mix, amber "Summarize" CTA, expanded AI updates).

---

## 1 Files & roles

| File | Lines | Role |
|---|---|---|
| `src/lib/components/workspace/WorkspacePage.svelte` | 1206 (script 1-393, markup 408-801, style 803-1206) | The "Workspace"/home hub. One page: header (folder + branch), inline project editor, project grid, Activity band (stats + chart + model mix), collapsible AI-updates strip. Mounted by registry as both `home` and legacy `projects` (`src/lib/components/shell/workspaceRegistry.ts:31,34`). |
| `src/lib/components/workspace/NewsFeed.svelte` | 337 (script 1-88, markup 90-215, style 217-337) | Body of the AI-updates strip, always `embedded` (`WorkspacePage.svelte:794`). Tier 1 Claude Code release cards, Tier 2 opt-in AI digest (costs a Claude turn). |
| `src/lib/components/shared/PageHero.svelte` | 48 | Eyebrow/title/desc header, **not used by this surface** (WorkspacePage rolls its own `.head`). Used by Settings (`settings/SettingsPage.svelte:626`), AI Health (`ai-health/AiHealthPage.svelte:721,824`), Diagnostics (`diagnostics/DiagnosticsPage.svelte:13`). Listed in scope because it is the shared page-header primitive. |
| `src/lib/components/shared/LimitBar.svelte` | 82 | Shared plan-limit bar, 3 variants (`panel`/`card`/`compact`). **Not used on the hub.** Consumers: `UsagePanel.svelte:242` (panel), `AiHealthPage.svelte:774,781,1097,1122` (card), `StatusBar.svelte:206` (compact). |
| `src/lib/components/onboarding/OnboardingFlow.svelte` | 392 (script 1-165, markup 167-392; zero `<style>`) | First-run 4-step flow: Welcome(+accent picker) / Connect AI / Open a project / Defaults. |
| `src/lib/components/onboarding/ObStage.svelte` | 67 | Per-step looping "product miniature" (220px stage). Markup only; all CSS in onboarding.css. |
| `src/lib/components/onboarding/ClaudeConnect.svelte` | 226 | Guided Claude connect (probe, install copy-blocks, sign-in, API key). Also mounted **outside onboarding** with `standalone` in `assistant/AssistantWelcome.svelte:158`. |
| `src/lib/components/onboarding/OpenAiConnect.svelte` | 78 | ChatGPT/Codex connect. Same dual use (`AssistantWelcome.svelte:158`). |
| `src/lib/styles/onboarding.css` | 548 | The whole `.ob-*` namespace + 23 keyframes. Imported as a global side-effect by OnboardingFlow (`:7`), ClaudeConnect (`:14`), OpenAiConnect (`:5`). |

Mount points / owners:
- Hub: `WorkspaceShell` keeps every once-opened page mounted, inactive ones `opacity:0; pointer-events:none; inert` (`src/lib/components/shell/WorkspaceShell.svelte:119-148`, `inert` at `:90`). First open gets a transient `.rising` child stagger (6px, `--stagger`) (`WorkspaceShell.svelte:196-209`) — but WorkspacePage's root is a single `.sb-main` child, so the whole page rises as one block.
- Onboarding: `AppShell.svelte:305-309` renders `<Titlebar/> + .ob-host > OnboardingFlow` INSTEAD OF sidebar/topbar/workspace when `showOnboarding` (`AppShell.svelte:48-52`: not dismissed, config loaded, and (no auth at all OR beta notice unacknowledged)). `.ob-host` styled at `AppShell.svelte:503-510`. Dismiss state: `src/lib/state/onboarding.svelte.ts` (localStorage `rift.onboarding.dismissed`).

Supporting (not in scope, logic the page imports): `src/lib/utils/usageStats.ts` (pure aggregation; types `ConvoStat/Totals/ModelSlice/DayCell` at `:6-48`), `workspace/hubHelpers.ts` (`pulseByRoot`, `relTime`, `fmtAgo`), `workspace/globPreview.ts`, `state/news.svelte.ts` (`NewsStore`, `:89-190`), `utils/projectHue.ts` (name -> 0-359 hue), `actions/tooltip.ts` (see §8, it is a no-op).

---

## 2 Layout & anatomy

### 2.1 Hub (`WorkspacePage`)
Single vertical scroll: `.sb-main` (flex col, h100%, transparent so the app dot-field shows, `:807`) > `.sb-scroll` (the ONE scroll viewport, `:808`) > `.sb-wrap` (max-width **1340px**, centered, padding `14px 40px`, gap 10px, `:817`). Design intent in comments: default window 1600x1000 fits with no scrollbar (`:812-814`); window min 900x600 (`src-tauri/tauri.conf.json:18-19`).

Z-order: all in normal flow, no overlays except the editor's recent-folders popover (`.recent-pop` abs, `z-index:20`, `:1030`). Card actions (`.pcard-actions`) are absolutely positioned over the card's top-right (`:1141`), siblings of the card's open-button (avoids nested buttons, `:594-629`).

Top to bottom:
1. **Header `.head`** (`:413-432`): eyebrow "Workspace" (10px caps faint), `h1.greet-line` = folder leaf name or "Your projects" (23px/600), and when a root is set a `.band-row`: `.cue` pill (FolderGit2 icon + `.branch-pill` + "N files") and `.chip-btn` "Switch folder" (`assistant.pickTabFolder(null)`). No "New project" here (it is in the Projects header). Does NOT use PageHero.
2. **Inline editor `section.editor`** (`:435-546`, only when `editing`): full-width card, head (icon tile, title/sub, X), 2-col `.id-grid` Folder (input + Recent popover + Browse) / Name, "File scope" dashed disclosure (`.adv-toggle`) revealing 3 textareas (Include/Exclude/Extra folders, `.pat-grid`), footer (Delete | Cancel, Create/Save).
3. **Projects `section.col.projects-col`** (`:558-650`): `.section-h-row` ("Projects" + count badge + `mini-new` "New"), then one of: load-error empty (`:568-573`), true-empty (`:574-584`), or **`.proj-grid`** (`:589-631`): `repeat(auto-fill, minmax(290px,1fr))`, gap 8px (`:1106`; 1-up under 680px; `.solo` 1 col). Each `article.pcard`: monogram tile (32px, per-project hue `--ph`), name + path (mono), active pill ("Active" + pulsing dot) on the active project, footer = chats / last-active / cost metas or "No chats yet", optional scope chip (`N include · M exclude`), and a "Continue" / arrow go-affordance. Top-right `.pcard-actions` (Open in split if `assistant.canAddPane`, Edit). Sort: active first, then last activity (`:372-380`). Below the grid, if the active folder isn't a project yet: `.save-current` CTA "Save <folder> as a project" (`:640-649`).
4. **Activity band `section.act-band`** (`:656-767`): header "Activity" + 7d/30d/All range group (`.range`, `:659-663`), then one of: skeleton (`:670-680`), error w/ Retry (`:681-689`), empty text (`:690-691`), or **`.act-card`** (`:693-765`) — 2-col grid `1.45fr / 1fr`, gap 20px, padding `13px 16px`, island fill + `--radius-2xl` (`:852-853`); under 920px it stacks (`:862-865`).
   - Left `.act-main`: `.hero` (38px gradient-clipped number = total messages + "messages exchanged" + `statSummary` sentence) and `.chart` (caption "Per day · last N" + "peak X", then `.chart-plot` = CSS-grid of N `span.ch-col` bars, N = 14/30/60 for 7d/30d/all, `:110`, plot `min-height:140px` flex-stretched, `:857-858`).
   - Right `.act-side` (left rule, `:859-860`): `.strip` 2x2 of stat tiles (sessions, tool calls, est. spend [link tile -> AI Health `:729`], streak/best), each with icon + number + optional trend chip (`trendChip` snippet `:398-406`); `.mix` model-mix (stacked 14px flex bar, <=5 hue-coded segments + muted "Other" tail, wrapped legend, `:741-761`); `.sig` italic "fun fact" line pinned to bottom (`:763`, `:851`).
5. **AI updates strip `section.news-strip`** (`:772-797`): hairline-topped, one header row (`.news-strip-h`: Newspaper tile + "AI updates" + hint when closed / "last checked" + refresh when open + chevron). Body (`{#if newsOpen}`, no height anim) = `<NewsFeed embedded/>`. Collapsed by default; state in `localStorage rift.workspace.newsOpen` (`:54-61`).

### 2.2 NewsFeed (`NewsFeed.svelte:90-215`)
`.news` column gap 12: (a) `.news-coverage` slim bar "Source coverage · official Claude Code release notes" + Settings pointer (`:115-119`); (b) `.digest` block (`:122-165`): loading row (orb + cycling step copy + sliding bar) / digest list (`.dg-item` buttons: tag pill, title+ext icon, summary, date) / the **warn-tinted CTA** "Summarize recent Claude news" with cost line (`:155-162`); (c) release list (`:179-213`): `.news-sub` label, `article.rel` cards (version button -> opens URL, date, <=4 bullets, "N more notes"), and "Show N more releases" (only first release visible by default, `RELEASE_PREVIEW=1` `:51`). `embedded` hides its own title row (`:94`).

### 2.3 Onboarding (`OnboardingFlow` + `onboarding.css`)
`.ob-overlay` (grid center, `clamp(24px,3.5vw,56px)` padding, radial accent glow on `--bg`, `:10-21`) > `.ob-card` (max 1180px, `height:min(820px, 100vh-96px)`, `--radius-2xl`, `--shadow-lg`, entrance `ob-card-in` `--dur-page` (400ms; 360 is only the var fallback), `:24-31`) = flex row:
- **Left `.ob-rail`** 312px (`:35-43`): brand (RiftLogo 30px + "Rift" + "Setup" tag), vertical stepper `.ob-steps` (4 `button.ob-step`: 26px numbered node, title, subtitle; absolute 2px `.ob-spine` whose inner `<i>` height = `(step-1)/(last-1)*100%` `OnboardingFlow.svelte:180`, transition `--dur-slow`), "Skip setup" link in rail foot. Hidden under 720px (`:531-532`) -> a `.ob-skip-mobile` link appears in the footer.
- **Right `.ob-main`**: `.ob-body` (scroll, centered column max 660px, gap 26px, `margin:auto 0` vertical centering `:104-108`) keyed by `{#key step}` (`OnboardingFlow.svelte:207`) so each step remounts and replays the `ob-rise` stagger; pinned `.ob-foot` (progress dots + Skip(mobile) + Back + Next/"Start working", `:365-389`, blurred bar `:151-155`).
- Per-step content: 
  1. Welcome: ObStage(chat), header, 2-row value list (`.ob-vlist`, one warn row = beta notice), **accent swatch strip** (8 `ACCENTS`, `OnboardingFlow.svelte:232-249`).
  2. Connect AI: ObStage(provider), provider segmented radiogroup (ChatGPT | Claude, 46px two-line buttons), then `ClaudeConnect` or `OpenAiConnect` (status card, install copy-blocks / sign-in button / API-key input, connected bar, re-check), soft-block warn hint (`:265-267`).
  3. Open a project: ObStage(tree), header, status card + "Change folder", or "Choose a folder" + up to 3 recents + skip hint.
  4. Defaults: ObStage(defaults, shows live model label), model segmented control, 3 preset cards (Cautious/Balanced[Recommended]/Fast), hint.

ObStage scenes (all `aria-hidden`, 220px tall, dotted bg): welcome = mini chat turn (user bubble, 2 tool chips, reply skeleton lines); claude = terminal typing `claude` then "Connected" pill; openai = ChatGPT -> Codex CLI -> App Server route; project = file tree + scan sweep; defaults = model pill + 4-dot dial + "Balanced" pill. Note the `defaults` scene hard-codes the pill text "Balanced" (`ObStage.svelte:63`) regardless of the real preset.

---

## 3 States

### Hub
| State | Where | Behavior |
|---|---|---|
| Stats loading | `statsLoading` initial `true` (`WorkspacePage.svelte:65`) | Skeleton: ghost number + 2 bars + 16 pseudo-random bars (`:670-680`), `.skel` shimmer 1.5s loop (`:873-880`). `role="status" aria-label`. |
| Stats error | `:681-689` | Centered danger panel + message + Retry (`loadStats`). |
| Stats empty (0 convos) | `statsEmpty` `:114`, text `:690-691` | One plain sentence in a bordered box; no illustration/action. |
| Range change | `range` `:67` | Instant swap of all numbers/chart/mix. No transition, no count-up. Trend chips only for 7d/30d (`trends` null for "all", `:90-91`); delta hidden when prior window is 0 (`:98-99`) or pct===0 (`:399`). |
| Projects: load error | `!projects.loaded && projects.lastError` `:568` | Dashed empty + Retry. |
| Projects: none, no active folder | `:574-584` | "No projects yet" lean empty + New project button. |
| Projects: none but a folder IS open | falls through; only `.save-current` shows (`:574`,`:640`) | |
| Active project | `isActive` `:346-349` | Accent-cooled border + 5% wash (`:1118-1120`), solid hue monogram w/ glow (`:1127-1129`), "Active" pill with pulsing dot (`:1088-1092`), "Continue" shown at rest (`:1158`). Click = `goHome()` (no reload). |
| Card hover | `:1112-1113` | border-strong, bg wash, shadow, **translateY(-1px)**; go-affordance fades in + slides 4px (`:1154-1157`); actions 0.4 -> 1 opacity (`:1137-1140`). |
| Card focus | `.pcard-open:focus-visible` ring `0 0 0 2px var(--ring)` w/ 7px radius (`:1115`) — ring hugs the inner button, not the card edge. |
| Card with no chats | `:612-613` "No chats yet" muted italic. | |
| Scope set | chip only when include/exclude non-empty (`:387-392`). | |
| Editor open | `editing != null`; entrance `edIn` 5px/`--dur-base` (`:990-992`); Escape closes (`:438`); focus lands on Folder or Name (`:194-196`); "+ New project" intent from sidebar opens it (`:337-339`). | |
| Editor: invalid glob | `advForced` forces the scope panel open (`:203,507`); textarea `.bad` danger border (`:1018-1019`), error text. | |
| Editor: saving / disabled | Create/Save disabled when `!canSave||saving`, `opacity:.5` (`:1077`); spinner via `:global(.editor .spin)` (`:1004`). | |
| Recent popover | `recentOpen`; per-row forget (x) fades in on row hover only (`:1043-1045`) — hover-only, invisible to touch/keyboard until focus? (button is focusable but `opacity:0` until row hover; no `:focus-visible` rule). | |
| News collapsed/open | `newsOpen` persisted (`:54-61`); chevron flips 180deg `--dur-fast` (`:1200-1202`); body mounts instantly (no height animation). | |
| News: checking | refresh icon spins (`nspin`, `:1193`); disabled while `news.status==='checking'` (`:786`). | |
| News: digest loading / ok / error / consent-needed | `NewsFeed.svelte:123-165`; CTA disabled+0.68 opacity and cost line changes to "Connect Claude to run this optional digest." when `!claudeReady` (`:155,160,241`). | |
| News: items loading / error / none | `NewsFeed.svelte:168-177`: spinner row / danger + Retry / "No updates to show yet." Auto-retry w/ backoff lives in the store (`news.svelte.ts:150-163`). | |
| Maintenance release | `.rel.maint` 0.78 opacity, italic line (`:296,193`). | |
| Narrow window | 920px: act-card stacks (`:862`); 760px: id-grid 1-col (`:1003`); 680px: pat-grid + proj-grid 1-col (`:1062,1108`); 640px: coverage note wraps (`NewsFeed.svelte:245`). Min window 900 so <900 paths are mostly dead. | |
| Hidden-but-mounted | Page stays mounted at `opacity:0` when another page is active (`WorkspaceShell.svelte:128-139`); the 60s `statsNow` interval (`WorkspacePage.svelte:78-83`) and infinite CSS loops keep running (whether the compositor skips painting them at opacity 0 is **unverified**). | |

### Onboarding
| State | Where | Behavior |
|---|---|---|
| Step enter | `{#key step}` `OnboardingFlow.svelte:207` | Children rise 8px/440ms staggered 70/130/185ms (only first 4 children) (`onboarding.css:109-115`). |
| Rail node | `.ob-step` `:60-85` | done = ghost fill + check, cursor pointer, hover bg; active = solid accent node + 5px soft ring; future = inert (`cursor:default`, `goto` only goes backward `:66-71`). |
| Spine | `:58-59` | Height transition `--dur-slow`. |
| Soft-block | `warnSkipConnect` (`:36-41,72-76`) | First Next on step 2 without connection shows a warn hint; second Next proceeds. |
| Provider flip | `selectProvider` `:43-53` | Mutates `assistant.model` as a side effect of the UI toggle. |
| Claude probing / not found / not signed in / connected | `ClaudeConnect.svelte:136-214` | Checking row; install copy-blocks + Re-check; Sign in button (disabled+spinner while `assistant.loginInProgress`) + hint to console/API-key; API-key input (password); connected bar with pulsing dot. Polls every 4s until connected (`:72-77`). |
| OpenAI connected / no CLI | `OpenAiConnect.svelte:42-65` | |
| Copy feedback | `.ob-copy-btn.ok`, 1.6s timeout (`ClaudeConnect.svelte:91`; css `:281`). | |
| Esc | `OnboardingFlow.svelte:99-107` | Step 1: dismiss entire flow. Later: back one step. |
| Narrow (<720) | `onboarding.css:531-540` | Rail hidden, card full-bleed, stage min 140px. <440: provider seg stacks, input rows stack. |
| Reduced motion | card entrance off (`:32`), stage loops off (all inside `no-preference` `:480-508`), connbar pulse off (`:323-325`). Rise stagger gated (`:109`). |
| Short windows | `.ob-card` height `min(820px, 100vh-96px)`; `.ob-body` scrolls; at 600px window height card = 504px: footer + padding leave little room (step 4 presets are the tallest). Unverified visually. |

---

## 4 Motion present

Rift tokens: `--dur-fast 140`, `--dur-base 240`, `--dur-slow 520`, `--dur-page 400`, `--dur-rise 460`, `--stagger 62ms`, `--pulse-live 1.6s`, `--ease-page cubic-bezier(.22,1,.36,1)`, `--ease-soft (.4,0,.2,1)`, `--ease-spring` (`src/app.css:240-250`). Halo: `fast 150 / base 300 / slow 600 / stagger 60`, one curve `ease-out (.2,.8,.2,1)`, `ease-spring` for flipped/grabbed things (`tokens.css`; `halo/03-motion.md`). Rift's tokens are close enough that a straight remap is nearly free; the durations differ by <=60ms.

Halo rules being judged against (`halo/README.md` principle 4, `halo/03-motion.md`): three speeds on one curve; **things enter once, in a stagger, never loop for attention; only real-time state pulses**; content *arrives* (skeletons fade, numbers count/roll, lists glide), never pops; only `opacity`/`transform` run continuously; blur only on small areas; everything honors `prefers-reduced-motion`; Cards rise 14px + fade (`bundle.css:22-23`); page change ~520ms glide (`bundle.css:24-25`).

### 4.1 Hub
| Where | What moves | Property / dur / easing | Trigger | Verdict |
|---|---|---|---|---|
| `WorkspaceShell.svelte:136-147` | whole page cross-fade | opacity+3px translate, 400ms in / 140ms out, page/soft | route change | KEEP concept (Halo `halo-page` 520ms/10px); retune to Halo `ease-out` + `AppShell pageKey`-style. |
| `WorkspaceShell.svelte:196-209` | page rise | opacity+6px, 460ms, stagger 62 | first open only | RETUNE: Halo rises 14px with per-card stagger; hub's one-root-child means no stagger today. Needs cards as direct stagger children or `halo-rise` per card. |
| `WorkspacePage.svelte:990-992` `edIn` | editor open | opacity + translateY(-5px), `--dur-base`, page ease | editor mount | KEEP (entrance once). No exit animation (unmount is instant). |
| `:1111,1113` `.pcard` | hover lift | translateY(-1px), border, shadow, bg; `--dur-fast` | hover | RETUNE: Halo `Card lift` = 3px, border line-100->200, **no shadow**, no bg wash (`bundle.css:30-32`). |
| `:1156-1158` `.pcard-go` | Continue arrow | opacity + translateX(-4px->0) `--dur-fast` | card hover | KEEP (a nice peek); transform/opacity only. |
| `:1137-1140` `.pcard-act` | action buttons | opacity .4 -> 1 | card hover | KEEP the hint-at-rest rule (discoverability); Halo has no equivalent. |
| `:1090-1092` `livePulse` | active dot | **box-shadow** ring expand, 2.2s infinite ease-out | always on active card | RETUNE: legit live-state loop but animates `box-shadow` (paint). Halo `LiveDot` = `::after` scale 1->3 + fade 1.8s, transform/opacity only (`bundle.css:75-82`). Period 2.2s vs `--pulse-live 1.6s` vs Halo 1.8s: pick one. Note "Active" project is not strictly real-time state; Halo says only real-time state pulses -> owner call. |
| `:873-880` `skel-shimmer` | skeleton | **background-position** 1.5s ease-in-out infinite | stats loading | RETUNE: animates background-position (repaints). Halo shimmer = `translateX` on `::after`, 1.4s linear (`bundle.css:100-102`) + 360ms blur-out reveal into content (`:104-107`). Also today the skeleton -> content swap pops (`{#if}`), no crossfade. |
| `:918-921` `.ch-col` | bar hover | filter brightness 1.18 + scaleY(1.03), `--dur-fast` | hover | KILL/RETUNE: `filter` hover + 1.03 scale is jitter; Halo bars dim siblings + show value (`bundle.css:209-218`). |
| (absent) | chart bars growing in | — | mount / range change | ADD: Halo BarChart bars grow in one after another, heights re-animate on data change (`halo/components/BarChart/README.md`); LineChart draws in (`halo-draw`). Today the chart pops. |
| (absent) | hero + stat numbers | — | load / range change | ADD: Stat count-up (default 1100ms ease-out) or `RollingNumber` (`halo/components/Stat/README.md`). Today numbers jump. |
| `:921` / `:947` `.mseg` hover | filter brightness | `--dur-fast` | hover | RETUNE (model mix widths also snap on range change; Halo has no stacked bar). |
| `:840-844` `.range` | range toggle | bg/color `--dur-fast`, no sliding thumb | click | RETUNE: Halo `Segmented` accent thumb slides `--dur-base` (`bundle.css:91-96`). |
| `:893,1076` `.act-retry`/`.save-btn` | press | translateY(1px) 80ms / `--dur-fast` | active | RETUNE: Halo press = `scale(.97)` (`bundle.css:54` `.halo-btn:active{scale:.97}`). |
| `:958-960,1004,1193,NewsFeed:331` | spinners | rotate 0.9s linear infinite (3 separate copies of the same keyframe: `wsActSpin`, `nspin`, `news-spin`; app.css has a 4th `.spin` 0.8s `src/app.css:625,628`) | pending | KEEP loop (real activity) but DEDUPE to one Spinner. |
| `:1054-1055,1200-1202` | chevrons | rotate 180deg `--dur-fast` | toggle | KEEP; Halo Accordion flips chevron + eases **height** at `--dur-base` (the strip body and file-scope panel just mount — no height ease). |
| `:1168-1169,1176` `.save-current` | hover | translateY(-1px), arrow translateX(2px) | hover | KEEP-ish; lift -> Halo 3px only if clickable Card. |
| `NewsFeed.svelte:239-240,271-272` | digest CTA / item hover | translateY(-1px), border | hover | RETUNE as Card lift; CTA carries a warn **gradient wash** (`:238`) which Halo forbids. |
| `NewsFeed.svelte:255-256` `orb-breathe` | loading orb | scale 1 -> 1.08, 2.4s infinite | digest loading | OK as real-activity loop; or replace with Halo Spinner. |
| `NewsFeed.svelte:259-260` `dl-slide` | indeterminate bar | translateX 1.3s ease-in-out infinite | digest loading | KEEP-> Halo `Progress` indeterminate (`bundle.css:460-461`, 1.3s ease-out). |
| `NewsFeed.svelte:70-74` | step copy | text swap every 2600ms | digest loading | KEEP (real stages); text swaps instantly (no fade). |
| `NewsFeed.svelte:312-313,321` | "more notes" chevron | rotate; bullet list expands instantly | click | RETUNE: add height ease or FlipList-style glide. |
| `NewsFeed.svelte:333-336` | reduced-motion | zeroes hover transforms + loops | media query | KEEP pattern; Halo does it globally (`bundle.css:901-903`: animation/transition duration 0.01ms). |

### 4.2 LimitBar (shared; not on hub but in scope)
| Where | What | Detail | Verdict |
|---|---|---|---|
| `LimitBar.svelte:38-40` | panel fill | `transition: width --dur-slow`; `limitbar-grow` width 0->pct 480ms | animates `width` (layout). Halo `Progress` = `scaleX` with `transform-origin:left` (`bundle.css:459`) and a counting % label. RETUNE. |
| `:43-45` | hot zone | `limitbar-pulse` `filter:brightness` 2.2s infinite | **KILL/RETUNE**: looping `filter` for "attention" violates "never loop for attention"; severity should be color + label, not motion. Owner call (it encodes urgency on purpose, `:56-57` comment). |
| `:59-63` | card hot sheen | translateX sweep 2.6s infinite | same; transform-only so compositor-safe, but still a decorative loop. |
| `:47,70` | glow | static box-shadow (live / compact hot) | Halo: glow only under hovered primary button -> KILL on bars. |
| `:75-81` | reduced motion | `animation:none`; compact also kills transition | KEEP. |
| Three visual variants (`:34-70`) with **three different fills** (gradient oklch literals `:39`, flat accent, accent) | | Halo has one `Progress` (8px, sm 4px, tones accent/info/warn/danger). Collapse. |

### 4.3 PageHero (`PageHero.svelte`)
No motion. 24px/760 title, 10px eyebrow with 16px accent rule (`:43-44`), `--hero-w` 820 column. Halo: `display/title` + `SectionLabel` pattern; page entrance via `halo-page`.

### 4.4 Onboarding (`onboarding.css`)
| Where | What | Detail | Verdict |
|---|---|---|---|
| `:29-31` | card entrance | opacity + 10px + scale .99, `--dur-page` | KEEP (once); Halo `halo-zoom`-like. |
| `:109-115` | step content rise | 8px, 440ms, delays 70/130/185 (children 2-4 only; 5+ appear with no delay but also animate) | KEEP/RETUNE to Halo 14px/600ms/stagger 60 capped. |
| `:59` | spine fill | height, `--dur-slow` | RETUNE: Halo Stepper fill = `scaleY` transform (`bundle.css:542,558`). |
| `:74,158` | node/dot transitions | bg/border/color `--dur-base`, dot width | OK; Halo adds a **spring pop + drawn check** on completion (`bundle.css:545-547`) — currently the check just swaps in. |
| `:138,185,218,245,278,300` | button/swatch/seg/preset hovers | 140ms various, `translateY(.5px)` press, swatch `translateY(-2px)` | RETUNE press to 0.97 scale; unify. |
| `:482-507` + `:510-528` | **ObStage loops**: `ob-loop-out` 7s/5.5s/6.5s infinite on 5 scenes, plus per-child keyframes, `ob-mc-dot` 1.4s infinite, caret `ob-blink` 1.1s infinite, dial 5s infinite | the whole illustration replays forever while the user reads the step | **KILL or play-once** (Halo: "never loop for attention"). Strongest conflict on this surface. Owner call (§10). Loops all gated by reduced-motion `no-preference`; base state = composed static scene (good — keep that property). |
| `:323-326` `ob-live` | connected-bar dot | box-shadow ring, `--pulse-live` infinite | Real "connected" state -> LiveDot; Halo says pulse only real-time; a static "Connected" badge is arguably enough post-connect. |
| `:361-362` | spinner `spin` 1s | duplicates app.css `spin` keyframes (re-declared `@keyframes spin` globally!) | DEDUPE. |
| `:151-155` | footer blur | `backdrop-filter: blur(8px)` on a static bar | Halo: blur only on small areas, one at a time; WebView2 cost. Likely KILL (no content scrolls beneath it except `.ob-body` which is a sibling above). |
| (absent) | step exit | `{#key}` swap is instant out, rise in | optional: Halo has no step-transition; fine. |

### 4.5 Gaps vs Halo (nothing here yet)
Number count-up/roll; chart draw/grow; skeleton -> content crossfade; sliding Segmented thumb; accordion height ease; card spotlight; list glide (`FlipList`) when projects re-sort after activity; toast-style feedback is handled by `notify` elsewhere.

---

## 5 Styling wiring

- **Surface is 100% scoped CSS + one global CSS file.** Zero Tailwind utility classes in any of the 9 files (no `class="..."` utilities; everything is semantic class + `<style>`). `design-system/check-ui.mjs` Tailwind-arbitrary-value metric is therefore 0 here.
- **Tokens used (Rift names):** `--bg`, `--bg-inset`, `--bg-elev-1/2/3`, `--surface`, `--surface-hover`, `--surface-active`, `--island-fill/-border`, `--border`, `--border-strong`, `--ghost-border`, `--fg`, `--fg-2`, `--fg-muted`, `--fg-subtle`, `--fg-faint`, `--accent`, `--accent-h`, `--accent-soft`, `--accent-fg`, `--accent-hover`, `--ring`, `--ok/-soft`, `--warn/-soft`, `--danger/-soft`, `--field`, `--code-bg/-border/-fg`, `--radius-xs..2xl`, `--r-card`, `--fs-xs..lg`, `--dur-*`, `--ease-page/-soft`, `--pulse-live`, `--shadow-lg`, `--font-mono`. Defined `src/app.css:133-265`.
- **Off-token literals (counts via grep this session):**

| File | `px` literals | `font-size:Npx` | oklch literals | hex | rgb/hsl | `border-radius:Npx` | keyframes | reduced-motion blocks |
|---|---|---|---|---|---|---|---|---|
| WorkspacePage.svelte | 308 | 25 | **16** | 0 | 0 | 22 | 5 | 5 |
| NewsFeed.svelte | 90 | 4 | 0 | 0 | 0 | 5 | 3 | 2 |
| PageHero.svelte | 16 | 2 | 0 | 0 | 0 | 1 | 0 | 0 |
| LimitBar.svelte | 10 | 0 | **2** | 0 | 0 | 2 | 3 | 1 |
| OnboardingFlow.svelte | 0 | 0 | **1** (`:240` swatch `oklch(0.78 0.15 {hue})`) | 0 | 0 | 0 | 0 | 0 |
| ObStage / ClaudeConnect / OpenAiConnect | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| onboarding.css | 328 | 29 | 4 | 0 | 0 | 15 | 23 | 4 |

- **Font-size spread** (px literals only; tokens add 11/12/13/15/18/22):
  - WorkspacePage: 9.5, 10, 10.5, 11, 11.5, 12, 12.5, 13, 17, 23, 38 (**11 distinct** off-scale values; the type ladder is 11/12/13/15/18/22, `app.css:260-265`).
  - onboarding.css: 9, 9.5, 10, 10.5, 11, 11.5, 12, 12.5, 13, 15, 25, 29 (12 distinct).
  - NewsFeed: 9.5, 10, 10.5. PageHero: 10, 24.
  - Halo is a 6-style scale (`caption/small/body/heading/title/display` + mono `metric/data/label`, `halo/02-type-space-and-layout.md`): essentially everything above 12px-ish collapses to 5-6 sizes.
- **Radius spread:** WorkspacePage px radii 2,3,4,6,7,8,10,12,999 plus token radii (`--radius`, `-lg`, `-xl`, `-2xl`); onboarding.css 1,2,3,6,7,9,10,12,999 + tokens. Halo: sm 8 / md 12 / lg 16 / pill (`tokens.css`).
- **Colour-per-feature, not tokens:** chart gradient `oklch(0.82 0.15 var(--accent-h))` -> `oklch(0.66 0.13 ...)` (`WorkspacePage.svelte:919`), model-mix per-segment hue gradients (`:947`, hues `[163,220,285,35,130]` `:115`), project monogram per-hue `oklch(... var(--ph))` (`:1125-1129`), "Other" grays `oklch(0.55 0 0)` etc. (`:949-950`), hero-number gradient text (`:907-909`), LimitBar gradient (`LimitBar.svelte:39`).
- **Shared-global CSS dependencies:** `.branch-pill` lives in `app.css` (comment `WorkspacePage.svelte:966`); `.lucide` utility selectors in onboarding.css (`:189,303,307,317,329,336,340`); global `.spin` + `@keyframes spin` in both `app.css:625,628` and `onboarding.css:361-362`; `.rift-input` is defined **inside** WorkspacePage's scoped style (`:1012`) despite the global-sounding name.
- **Import-time global side effect:** `onboarding.css` is a plain global stylesheet; importing it from the 3 components means `.ob-*` rules apply app-wide the moment any of them mounts (needed because AssistantWelcome reuses the connect cards, `ClaudeConnect.svelte:11-14`).
- **`check:ui` ratchet** (`design-system/check-ui.mjs:94-105`, baseline `design-system/ui-baseline.json`): counts oklch literals/hex/rgb/arbitrary values in **`.svelte` only**; baseline `oklch-literal:131`, `overlay:aria-role:17`, `use-portal:11`. WorkspacePage contributes 16 oklch, LimitBar 2, OnboardingFlow 1 (~19 of 131). `onboarding.css` is **invisible** to the ratchet — moving its rules into `<style>` blocks would count them. Restyling that deletes literals only lowers counts (then run `--update`); adding any new literal in a `.svelte` fails.

---

## 6 Restyle boundary

### LOGIC to keep (do not touch while restyling)
**WorkspacePage script (`:1-393`)** — all of it is data/behavior:
- Stats pipeline: `loadStats` -> `invoke("assistant_stats")` (`:69-77`), `range`/`statsNow` + 60s tick (`:67-83`), `filterRange/summarize/streaks/peakHour/perModel/topModel/dailySeries/summaryLine/funFact` derivations (`:84-114`), trend math (`:89-105`), `segs`/`mix` with the <1% "Other" fold (`:115-131`). Pure helpers stay in `utils/usageStats.ts` (tested, `usageStats.test.ts`).
- Hub context: `paneRoot`, `hubBranch`/`hubFileCount` with the stale-response guard (`:33-43,133-160`) — invokes `assistant_workspace_branch`, `assistant_list_workspace_files`.
- Scroll-to-top on activation (`:46-49`), `newsOpen` persistence (`:54-61`).
- Project CRUD + editor state machine (`:162-366`): `startNew/startEdit/cancelEdit/save/confirmDelete/browse/pickRecentInEditor/forgetRecent/autoName`, `DEFAULT_EXCLUDE`, glob validation (`globSummary`), `scopeSummary`, `newProjectIntent` consume effect (`:337-339`), focus-on-open effect (`:194-196`), Escape handler (markup `:438`).
- Card logic: `pulseOf`, `isActive`, `sortedProjects`, `monogram`, `scopeLabel`, `openProject`, `openInSplit` (`:168-171,345-392`), store calls `projects.*`, `assistant.setRoot/openProjectInPane/pickTabFolder/removeRecentRoot/canAddPane`, `workspace.setActive("ai-health"|"chat")`, `goHome`, `notify.*`.
- `news.*` store usage and the "Last checked" tick.

**NewsFeed script (`:1-88`):** `openExternal` https-only guard (`:34-43`), `expanded` Set, `RELEASE_PREVIEW`/`showAllReleases`, digest step-copy effects (`:62-80`), `tagMeta`, `claudeReady` gate, `news.maybeFetch/summarize`.

**Onboarding script:** step machine + soft-block (`OnboardingFlow.svelte:26-107`), preset fan-out to `assistant.setPermissionMode/setThinkingDial/setTrustLevel` + `activePreset` derivation (`:128-164`), `seedFirstPrompt` (`:87-93`), model filtering (`:55-64`), folder picking (`:109-120`), Escape semantics, `uiPrefs.setAccentHue`. ClaudeConnect: probe/poll/copy/signIn/saveKey (`:38-127`), `onConnectedChange` contract (`:49-55`), `standalone` prop. OpenAiConnect: `refresh`, `onConnectedChange`.
- AppShell's gate + `.ob-host` + `finishOnboarding` (`AppShell.svelte:48-60,305-309`) and `scripts/cdp/c.sh:548` (`document.querySelector(".ob-host")`) — selector must survive.

**LimitBar contract:** props `pct/zone/variant/live/class` and the `data-zone` attribute are consumed by 3 other pages — change visuals, keep (or migrate) API with those callers.

### MARKUP / CSS to change
- Everything inside `<style>` of WorkspacePage/NewsFeed/PageHero/LimitBar and all of `onboarding.css`.
- Hub template regions: header, project cards, Activity band (hero number, chart, stat strip, model mix, fun fact), news strip, empty/error/skeleton blocks, editor chrome (keep fields + validation; restyle container/buttons/inputs — Halo `TextField`/`Textarea`/`Field`/`Button`).
- Onboarding: rail + stepper, step frame, ObStage scenes, provider/model segmented controls, preset cards, copy blocks, status cards, connected bar, footer.
- Class names that other code/tests might depend on: only `.ob-host` (AppShell-owned) was found in scripts (`c.sh:548`). Everything else is component-private. `ClaudeConnect`/`OpenAiConnect` markup (`.ob-*`) is **shared** with AssistantWelcome (`AssistantWelcome.svelte:158`), so restyling `.ob-*` restyles the signed-out chat welcome too (see §9).

---

## 7 Halo mapping table

Halo CSS refs are `bundle.css` lines; component docs in `halo/components/<Name>/README.md`.

| Rift element (cite) | Halo component / token | Notes / gaps |
|---|---|---|
| Page title block `.head` (`WorkspacePage.svelte:413-432`) + `PageHero` | Halo `display`/`title` type + `SectionLabel` for sections; Halo has **no PageHero** (AppShell/`halo-page` for the entrance) | Keep a Rift `PageHeader` part (shared by Settings/Health/Diagnostics). Folder name + branch chip + file count = Halo `Badge`/`Breadcrumbs`-ish; "Switch folder" = `Button ghost sm`. Halo buttons are 44px (36 sm) vs Rift 26-38px compact density (`app.css:289`) -> **owner call**. |
| Section headers `.section-h` ("Projects", "Activity") (`:823-825`) | `SectionLabel` `01 / PROJECTS` + hairline + count (`bundle.css:85-88`; README) | Direct fit. Numbered `01 / 02 / 03` for Projects / Activity / AI updates. Halo mono caps 11px, index in accent, `.12em` tracking. |
| Hub grid wrapper `.proj-grid` (`:1106`) | `FocusGroup` (grid, `minColumnWidth` default 220, gap `space-4`; `dim` opt-in) (`bundle.css:38-40`) | Rift floor 290px/gap 8px. `dim` is "one hero moment per page"; with 6+ project cards it is plausible but owner choice. |
| Project card `.pcard` (`:1109-1158`) | `Card` (`lift` if whole card clickable; hairline line-100 -> line-200; pad `space-5`=24px; radius 16) (`bundle.css:30-35`) | **Density change**: Rift cards are 11x13px padded, ~105px tall, 4-up at 1600. Halo 24px pad -> 3-up or fewer / more rows -> fold pressure. Keep the "real `<button>` inside + sibling actions" structure (Card README). Active-accent frame: Halo allows accent only for primary/live — use `spotlight` on the active card or a 1px accent hairline, not a wash. |
| Project monogram `.pcard-mono` (per-name hue, `:1124-1129`; `projectHue.ts`) | `Avatar` (initials, tint from the 4 chart colours, deterministic by name) (`bundle.css:464-479`) | **Conflict**: Rift = free 360-hue identity per project across card/switcher/chips; Halo = 4 tints. Prior audit asked for neutral avatars w/ accent only on active (`docs/design/redesign-plan.md:63-76,163-166`). Decide; switcher + sidebar chips share `projectHue`. |
| "Active" pill + dot (`:1088-1092`) | `Badge tone=accent` + `LiveDot` | Dot is a loop; "Active" = selected, not live data (§4). |
| Meta row chats/ago/cost (`:607-617`) | Halo `caption` text + 11-12px icons; `Badge neutral` for scope chip | No dedicated "meta row"; Rift-specific small composition. Cost shown as plain text (Halo: mono numbers). |
| "Continue ->" affordance (`:616`) | `Button ghost sm` or text link in Card; `lift` | keep hover-reveal. |
| Open-in-split / Edit (`:619-628`) | `Button ghost` icon-only w/ `ariaLabel` (+ Tooltip) or `Menu` | **Tooltip gap**: Rift's `tooltip` action is a no-op (§8); Halo `Tooltip` exists — re-introducing it is "a single-file change" (`tooltip.ts:6-7`). |
| New project button `.mini-new`, `.save-btn` (`:978-983,1072-1077`) | `Button secondary sm` / `primary` | One primary per view (README rule): hub has up to 3 solid-accent calls (editor Create, empty New project, Retry). |
| Save-current CTA (`:1164-1176`) | `Banner tone=accent` w/ action OR `Card lift` with accent icon | Halo `Banner` = persistent page message with one action — good fit semantically. Remove gradient (`:1167`). |
| Empty "No projects yet" (`:578-584`), "No conversations yet" (`:691`), "No updates" (`NewsFeed:177`) | `EmptyState` (accent icon in concentric rings, title, one line, **one action**) (`bundle.css:338-343`) | Halo's is centered, padded `space-7`/`space-5` — bigger than Rift's deliberately "slim inline panel" (`:1079`). Pick a compact variant. |
| Load errors (`:568-573`, `:681-689`, `NewsFeed:170-175`) | `Banner tone=danger` (announced as alert) w/ Retry action, or `EmptyState` | Halo Banner dismiss/keep rules fit. |
| Range toggle `.range` (`:659-663`) | `Segmented` (2-5 short options, sliding accent thumb, `aria-pressed`) (`bundle.css:91-96`) | Exact fit (7D / 30D / ALL). Halo min 52x36 vs Rift 22px — compact. |
| Hero number "5,914 messages" (`:696-702`, gradient text `:907-909`) | `Stat` / `RollingNumber` `metric` 30/34 mono (`bundle.css:64`, roll `:740`) | Gradient-clip text is a "colour wash" Halo bans; old audit flagged oversized numeral as AI-slop (`redesign-plan.md:71`). 38px -> 30px. Count-up on range change = Halo behavior to gain. |
| Stat tiles 2x2 (`:720-739`, CSS `:924-933`) | 4x `Stat` inside `Card`/`FocusGroup` (label -> big value -> detail), `delta` chip, optional `peek` (best streak / breakdown), optional `Sparkline` child | **Gaps:** `StatProps.value: number`, `format: 'number'|'currency'|'percent'` only (`halo/components/index.d.ts:50-63`) — Rift needs compact ("32k"), "$8.2k", "1d" with a unit suffix, and "streak · best 37d"; use `RollingNumber` with function `format`, or Rift wrapper. `delta` sign picks tone, down = `warn` (no cost-inversion; Rift inverts cost, shows neutral grey for down, hides at 0/null — `:398-406`). Tile that is a link (est. spend -> AI Health) has no Halo "linked stat" idiom: wrap in `Card lift` + `<a>/<button>`. |
| Trend chips `.delta` (`:936-942`) | `Stat delta` -> `.halo-chip--up/--down` (`bundle.css:66-68`): "↑ 12.4%" accent-soft / "↓" warn-soft | Halo mandates arrow + sign (README content rules); Rift already has arrows. Down in warn (amber) instead of neutral gray — decision (is a decline a "warning"?). |
| Daily chart `.chart-plot` 14/30/60 bars (`:703-716`, `:916-922`) | `BarChart` (4-12 categories; peak bar accent, rest `bg-300`, hover value + dim others; grows in) **or** `LineChart` (trends, draws in, gliding tooltip, `compare` dashed) | **Gap:** 14-60 points exceeds BarChart's 4-12 guidance; LineChart fits trends but its tooltip shows only value (no "N sessions"). Two candidates: LineChart with 30/60 points (loses zero-day texture) or Rift dense-bar variant. Per-bar tooltip text today (`:712`) is dead (§8) — Halo gives a working one. |
| Model mix stacked bar + legend (`:741-761`) | **No equivalent.** Closest: `Progress` x N, or `BarChart` (loses share-of-whole), `Ring` (one goal only) | **Rift-specific part needed ("ShareBar")**: single stacked bar, <=6 segments. Halo palette = `chart-1..4` only (4 series, "one accent per chart") vs Rift 5 hues + Other. Proposal: top 3 by share in chart-1..3, rest folded into neutral `bg-300`/`ink-faint` "Other". Prior audit also asks "keep multi-hue here?" (`redesign-plan.md:223`). |
| Fun-fact line `.sig` (`:763`) | `caption` text / `Divider` | trivial. |
| Streak (`:735-738`) | `Stat` w/ icon, or `Ring` (progress to "best")? | **No natural Ring on this surface** — `Ring` = "one goal"; nothing here is a goal. Do not force it. |
| Activity skeleton (`:670-680`) | `Skeleton variant=stat` + custom chart skeleton; crossfade via `loading` prop (`halo/components/Skeleton/README.md`; `bundle.css:98-111`) | Chart-shaped skeleton isn't a Halo variant — compose from `text` bars. Gains the 360ms blur-out reveal. |
| Per-card `use:tooltip` hover details | `Tooltip` | see gap above |
| Editor card `.editor` (`:986-992`) | `Card` (maybe) or `Drawer` ("edits beside the page", Halo README rules) or `Dialog` ("asks") | Inline edit-in-place is a Rift choice; Halo's rule of thumb says `Drawer` edits beside the page. Owner call. |
| Editor fields `.rift-input`, `.fld` (`:1009-1020`) | `TextField` (carries label/hint/error) / `Textarea` / `Field` wrapper; Folder row = `TextField` + `Button`s; globs hint+count = Field hint | Halo TextField is 52px tall with reserved label space (padding 20px top, inferred floating label) (`bundle.css:124` `.halo-field__input{height:52px}`) vs Rift 38px — big density jump. Recent-folders popover = `Popover`/`Menu` (Rift hand-rolled abs popover, `:1030`; also counted by check:ui? no—only portal/role). |
| "File scope" disclosure `.adv-toggle` (`:1049-1058`) | `Accordion` (height ease + fade, chevron flip) | clean fit; forced-open when invalid = controlled `value`. |
| Delete button | `Button danger` | |
| News strip `.news-strip` (`:1181-1203`) | `Accordion` (single item, `variant=cards`) w/ `meta` slot for "last checked" | The refresh button lives in the header row beside the toggle (a button inside an accordion header is invalid nesting -> header is a Rift composition, body = Accordion panel). |
| Release cards `.rel` (`NewsFeed.svelte:292-321`) | `Card` + bullets; "show more" = `Accordion` or `FlipList` | Version button -> `<a>`/Button link. |
| AI digest CTA (warn gradient) (`:236-249`) | `Card` + `Button secondary` + warn **text** cost line (keep) | Keep cost-in-amber language (`redesign-plan.md:73-74` agrees); drop gradient/wash. Not a `Banner` (not a page-problem). |
| Digest tag pills (`:273-277`) | `Badge` (neutral; `model` -> accent) | Halo status tones are semantic (accent/info/warn/danger) — a 4-category tag (Model/Claude Code/API/Anthropic) has no tone mapping; use neutral for all but optionally accent for Model (as today). |
| Digest items list | `Card` rows / `FlipList` (new items "fade up") | |
| Loading orb/bar (`:251-260`) | `Spinner` + `Progress` indeterminate + `Skeleton variant=list` | |
| Last-checked / refresh | `LiveDot idle` + `Button ghost` icon | |
| `LimitBar` (3 variants) | `Progress` (tones accent/warn/danger; `sm` 4px; eases to value, % counts) (`bundle.css:454-461`) | Hot pulse/sheen/glow have no Halo analogue (§4.2). Compact StatusBar bar is 46x4 fixed width. `zone` strings -> tone mapping layer. Consumers on other pages (UsagePanel, AiHealth, StatusBar) — coordinate with those maps. |
| Onboarding overlay/card (`onboarding.css:10-31`) | `halo-screen` page + `Card` (or Halo `Dialog contained`?) | It is a takeover screen, not a dialog; `role="dialog" aria-modal` is applied (`OnboardingFlow.svelte:169`) without focus-trap/inert (a11y gap vs Halo dialog behavior; counted in check:ui `overlay:aria-role`). |
| Rail stepper (`:55-85`, markup `:179-197`) | `Stepper orientation='vertical'` (steps `{label, description}`, `current` index, `onStepClick` for finished steps, `aria-current="step"`, accent fill track, pop+drawn check) (`bundle.css:535-558`; README) | **Near-exact fit**: same label+description, same backward-only click. Halo mark 32px mono numbers, current = accent ring + `accent-soft` halo; done = filled accent w/ drawn check. Dots in footer become redundant. |
| Brand + "Setup" tag | `Logo` + `Badge` | Rift `RiftLogo` is the product mark, keep. |
| ObStage miniatures (`ObStage.svelte`, css `:365-528`) | **No equivalent** (Halo `EmptyState` rings + `Motion` showcase + `halo-draw` are the nearest vocabulary) | **Rift-specific illustrations stay.** Restyle with Halo tokens, drop loops (play once/static). |
| Provider seg (ChatGPT/Claude two-line) (`:209-234`) | `RadioGroup` (pick one with explanations) | Halo rule: `Segmented` = short labels only; two-line w/ subtitles => RadioGroup. |
| Model seg (Defaults step) (`OnboardingFlow.svelte:316-334`) | `Segmented` (<=5 short) or `Select` | Count depends on provider/plan (`defaultModels`). |
| Preset cards (`:238-261`) | `RadioGroup` with descriptions; `Badge accent` "Recommended" | Direct fit. |
| Accent swatches (`:177-198`, markup `:232-249`) | Halo theme presets (4 accents x dark/light) | **Conflict**: Rift = continuous hue picker over 8 swatches (`ui-prefs.svelte.ts:84-93`), dark only. Halo = fixed presets `dark/blue/ember/violet`. Open question #3. |
| Copy block (`:263-281`) | `CodeBlock` + copy button | "never colours syntax" — fits. |
| Status card rows (`:331-344`) | `Card` + rows w/ `Badge` ok/warn + icon | |
| Connected bar `.ob-connbar` (`:320-326`) | `Banner tone=accent` or `Badge`+`LiveDot` | Rift uses `--ok` (green) for success; Halo success = `accent` (not a green). In Halo-blue, "connected/ok" becomes blue; `--ok` green usage across Rift needs a decision (global; see token map). |
| Warn hint (`.ob-hint--warn`) | `Banner tone=warn` (inline) | |
| API-key input, recents list | `TextField type=password`, list of `Button ghost` rows / `Menu` | key input lacks label/aria today (`ClaudeConnect.svelte:198-204` placeholder only). |
| Skip/Back/Next | `Button ghost` / `secondary` / `primary` | |

Token-level notes for this surface: Rift uses `--fg-faint` heavily for labels/meta (`redesign-plan.md:96` counts 261x app-wide); Halo says `ink-faint` only for section labels/axis ticks/timestamps, never on `bg-300`, and 4.5:1 floor. Halo fonts: Geist/Geist Mono loaded from Google Fonts (`halo/components/bundle.css:1`); Rift ships Inter + JetBrains Mono + Lexend via `@fontsource` (`src/routes/+layout.svelte:2-4`) under a CSP `font-src 'self' data:` (`src-tauri/tauri.conf.json:28`) — **remote Google Fonts would be blocked**; Geist must be bundled (e.g. @fontsource) or Halo type mapped onto Inter/JetBrains.

---

## 8 Pain points (evidence)

1. **Chart/model-mix data is invisible.** `use:tooltip` on every bar and mix segment (`WorkspacePage.svelte:712,746,750`) and on stat/links is a **no-op** by design since 2026-06-15 (`src/lib/actions/tooltip.ts:1-8,24-41`: paints nothing, strips `title`). The bars are `aria-hidden` (`:713`) so there is also no screen-reader path. Per-day values, session counts, model names/shares for tiny slices ("<1%" Other labels), and path full text (`:604`) are unreachable. Only "peak N" survives (`:706`).
2. **Static, popping dashboard.** Numbers/bars/mix change instantly on range switch (`:660-662`), skeleton -> content is a hard `{#if}` swap (`:666-692`), news body mounts instantly (`:792-796`). No count-up, draw-in, or crossfade. Reads as a screenshot, not "alive".
3. **Looping decoration contradicts the calm brief:** 5 ObStage loops 5-7s forever (`onboarding.css:482-507`), LimitBar hot pulse/sheen (`LimitBar.svelte:43-45,59-63`), orb-breathe (`NewsFeed:255`), active-card dot at 2.2s (`WorkspacePage:1090`). Loop periods are inconsistent (2.2 / 2.4 / 1.6 `--pulse-live` / 1.8 Halo).
4. **Four copies of the same spinner keyframe** (`wsActSpin :959`, `nspin :1194`, `news-spin NewsFeed:332`, `.spin app.css:625,628` + `spin onboarding.css:362`) and two different `.spin` durations (0.9s vs 0.8s vs 1s).
5. **Off-scale typography/radii:** 11 distinct font sizes in WorkspacePage incl. 9.5/10/10.5/11.5/12.5/17/23/38 (`:823-1203`), 12 in onboarding.css, plus 9+ radii. The audit already measured this class of problem app-wide (388 hard-coded font-sizes, `redesign-plan.md:94-95`).
6. **Hero numeral + gradient text** = flagged AI-slop tell (`redesign-plan.md:71`) still present (`:907-909`, 38px/760).
7. **Colour noise:** per-project hue avatars (`:1124-1129`), 5-hue model mix (`:115,947`), amber gradient wash CTA (`NewsFeed.svelte:238`), accent-washed active card/save-current/editor (`:1118-1119,1167,988`), gradient bars (`:919`). The Halo brief is "one signal colour, no washes".
8. **Nested-surface depth:** `.act-card` (island) > `.chart-plot` (inset bordered) and `.strip` (bordered grid in a bordered card) = 2-3 levels of box (`:852,916,924`), the "box in card in island" problem from the audit (`redesign-plan.md:74-75`).
9. **Four left edges / mixed widths:** hub column 1340px (`:817`) vs PageHero 820px (`PageHero.svelte:15,18`) vs onboarding 660px (`onboarding.css:105`) — the audit's "four left edges" finding (`redesign-plan.md:97-99`) is unresolved; hub doesn't use the shared PageHero, so Settings/Health/Hub headers diverge.
10. **Dense = small hit targets:** `.pcard-act` 26px at 0.4 opacity (`:1137-1138`), `.range button` 22px (`:841`), `.mini-new` 24px (`:978`), `.nsh-refresh` 26px (`:1189`), `.recent-forget` 22px **opacity 0 until row hover** (`:1042-1045`). Halo asks >=44px targets (`halo/02-type-space-and-layout.md`); desktop density means this needs a deliberate rule.
11. **Weak a11y in chart + focus:** `.ch-col` bars `aria-hidden`; focus ring on a card is drawn on the inner button with a 7px radius (`:1115`), not matching the card's 12px radius; onboarding overlay claims `aria-modal` without trap/inert (`OnboardingFlow.svelte:169`); API-key input has no label (`ClaudeConnect.svelte:198`); provider seg uses `role="radio"` on buttons without arrow-key handling (`OnboardingFlow.svelte:252-258`; unverified keyboard behavior).
12. **Rail footer duplicate progress** (footer dots + rail spine) and the `defaults` stage hard-codes "Balanced" (`ObStage.svelte:63`) even if the user picks Fast.
13. **Escape semantics trap:** Esc on step 1 skips the whole setup and sets dismissal + beta acknowledgement (`OnboardingFlow.svelte:99-107`; `AppShell.svelte:56-59`) — a reflexive Esc ends first-run (design comment acknowledges it for later steps only).
14. **Hidden page keeps running:** hub stays mounted at `opacity:0` (`WorkspaceShell.svelte:128-135`) with infinite loops + a 60s interval (`WorkspacePage.svelte:78-83`) — perf/battery smell (unmeasured).
15. **Skeleton animates `background-position`** (`:877-879`) = repaint every frame; Halo's is transform-only (`bundle.css:100-102`).
16. **Hover-only discovery:** recent-forget x, go-arrow, news ext icons (`NewsFeed.svelte:280,300`).

---

## 9 Risks

- **Shared `.ob-*` blast radius.** `ClaudeConnect`/`OpenAiConnect` render in `AssistantWelcome` when signed out (`AssistantWelcome.svelte:158`, comment `:155-157`) and import `onboarding.css` themselves. Restyling `.ob-*` (or relocating it into scoped `<style>`) changes the chat welcome needs-auth screen and, if rules go scoped, can strip styles there (Svelte scoping; the original reason for the global file, `onboarding.css:3-4`). Map file for the chat/welcome surface must coordinate.
- **No component tests reference these classes.** Found only: `hubHelpers.test.ts`, `usageStats.test.ts`, `globPreview.test.ts` (logic, pure) and a CDP helper `scripts/cdp/c.sh:548` using `.ob-host`. So markup is free to change; logic modules must stay API-stable (`usageStats` exports, `pulseByRoot`, `fmt*`).
- **`npm run check` includes `check:tokens` + `check:ui`** (`package.json` `check` script). `check:tokens` compares `design-system/styles.css` mirror to `src/app.css` (`design-system/check-tokens.mjs:1-28`): adopting Halo token names/values in `app.css` will MISMATCH against the DS mirror and manifest — a token-layer decision belongs to the global-tokens map, but this surface can't land first. `check:ui` ratchet only goes down; any new off-token oklch/hex/rgb in a `.svelte` fails. Putting Halo's literal hexes into `.svelte` instead of tokens will fail; `.css` files are not scanned (loophole, but don't rely on it).
- **`check:ui` hand-rolled overlay count** (`overlay:aria-role` baseline 17): OnboardingFlow's `role="dialog"` (`:169`) is one; replacing hand-rolled recent popover/overlays with bits-ui primitives (`src/lib/components/ui/*`) lowers counts (then `--update`).
- **WebView2/Tauri quirks:** `backdrop-filter` + gradient-text (`background-clip:text`) are used; `-webkit-app-region: no-drag` on `.st.link` (`:931`) is a Tauri drag-region guard — keep on any clickable that lands near the custom titlebar. Halo `Card spotlight` uses CSS mask-composite (`bundle.css:33`) + per-mousemove updates; on WebView2 verify (Halo promises once-per-frame). `color-mix(in srgb|oklab)` is used widely both sides. `:has()` (Halo FocusGroup dim) needs WebView2 >= Chromium 105 — Tauri 2 evergreen is fine; unverified here.
- **Fonts/CSP:** Geist from Google Fonts is blocked by CSP (`tauri.conf.json:28`) and offline use; Halo mono numerals (tabular figures) matter for roll/count animations — a mapped font must have tabular nums (JetBrains Mono does).
- **Motion perf:** Halo count-up/roll + chart draw + stagger at once on a keep-alive page: Halo caps stagger (~12) and uses only opacity/transform; the hub's 14-60 bars + 4 stats + N cards is within that, but rolling 4 numbers on every range click should be measured in the packaged app. Don't animate on every `statsNow` tick (60s) — `statsNow` feeds derived `stats`, so a naive `Stat` tied to derived totals could re-fire when the window slides.
- **Keep-alive + entrance animations:** Halo `halo-rise`/`halo-draw` run on mount only; the hub mounts once and is then toggled by opacity, so re-entry won't replay unless the redesign adds a key/`pageKey` on activation (WorkspaceShell `.rising` is first-open only, `:67-71`). Needs an explicit "re-arrive" hook if desired.
- **Layout fit goal:** current hub is tuned to avoid a scrollbar at 1600x1000 with tight 10px rhythm (`:812-817`). Halo defaults (24px card padding, 16/32 gaps, 44px controls) will push past it; README says dashboards should fit one screen and the list scroll inside the card (`halo/02-type-space-and-layout.md` "Scrolling and fit") — requires choosing what scrolls (projects grid in-card vs page).
- **Behavioral landmines:** `selectProvider` mutates `assistant.model` as a side effect (`OnboardingFlow.svelte:43-53`); preset buttons fan out to 3 setters with a swallowed rejection (`:157-164`); `seedFirstPrompt` on finish. Don't move these into presentational components.
- **Editor semantics:** Escape handler on a `role="form"` section with `svelte-ignore` (`WorkspacePage.svelte:436-438`) — if moved to `Drawer`/`Dialog`, Esc/focus handling comes from the primitive (Halo: traps focus, returns it); re-verify the "focus Folder vs Name" effect (`:194-196`).
- **a11y regressions to avoid:** chart currently has no text alternative (pain #1) — whatever replaces it must expose values (Halo charts take `ariaLabel`); status never color-only (Halo rule) — project "Active" and delta chips already carry word/arrow.
- **Light mode:** Rift is dark-only (`app.css:125-127` header comment "Dark mode only"); Halo ships light presets. Not in this wave; flag if the plan promises it.

---

## 10 Open owner questions

1. **Accent model.** Rift today: continuous hue (`--accent-h`) with 8 swatches picked in onboarding step 1 (`ui-prefs.svelte.ts:84-93`; `OnboardingFlow.svelte:232-249`). Halo: 4 fixed presets (lime/blue/ember/violet) — and the owner chose Electric blue. Keep a user-pickable accent (and which swatches), or fix to Halo blue and drop the picker from onboarding (and what does the user choose in step 1 instead)?
2. **Project identity colour.** Keep per-project hue monograms (`projectHue`, used app-wide), or switch to Halo `Avatar` 4-tint / neutral with accent only on the active project?
3. **Model-mix chart.** Accept "top 3 in chart-1..3 + neutral Other" (Halo palette) or build a Rift-specific multi-hue `ShareBar`? Is the 5-hue legend worth breaking "one signal colour"?
4. **Daily chart form.** `LineChart` (trend, draws in, gliding tooltip, but can't show zero-day texture or session counts) vs a Rift dense bar chart (14/30/60 bars) vs `BarChart` limited to 7-12 bars (e.g. weekly buckets for 30d/all)?
5. **Density vs Halo.** Halo = 24px card padding, 44px buttons, 52px fields; Rift = compact (11-13px padding, 26-38px controls, 1600x1000 no-scroll target, `data-density` toggle `app.css:289-291`). Adopt Halo sizes for this surface, add a compact Halo variant, or keep compact? Does the hub still have to fit without scrolling?
6. **Onboarding stage loops.** Kill them, play once then freeze, play on step-enter only, or keep (they are the one "product miniature" delight)? Same question for "Active" pulse dot and LimitBar hot pulse/sheen (urgency motion vs "never loop for attention").
7. **Tooltips.** The `tooltip` action is intentionally a no-op (2026-06-15, "visual noise"). Halo has a `Tooltip` and hover-driven chart tooltips. Re-introduce tooltips (at least on charts and icon-only buttons), or keep them off and show values another way (always-visible labels)?
8. **Project editor placement:** keep inline card, or Halo `Drawer` ("edits beside the page") / `Dialog`? Halo `TextField` (52px floating label) or a denser field for this form?
9. **Page header:** adopt one shared page header (Rift `PageHero` => Halo display title + `SectionLabel`) for Hub/Settings/Health at one column width, or keep the hub's own header + 1340px width?
10. **Section numbering:** `01 / PROJECTS`, `02 / ACTIVITY`, `03 / AI UPDATES` — OK for the hub? (Halo principle 3; the old audit said mono is for code/IDs only, `redesign-plan.md:60` C13. Halo overrides: mono for numbers + labels.)
11. **Success colour.** Rift's `--ok` is green (connected bars, copy-ok, delta-up). In Halo-blue, positive = accent. Replace greens with accent everywhere on this surface, or keep a green status?
12. **AI-updates strip:** keep collapsed-by-default under a hairline, make it a Halo `Accordion`, or demote/remove from the hub (it is the least "launch" content, and the Tier-2 digest spends money)?
13. **Esc on onboarding step 1** currently dismisses and acknowledges beta (`OnboardingFlow.svelte:105`): keep?
14. **Fonts:** bundle Geist + Geist Mono (Halo native; needs `@fontsource` + CSP-safe) or keep Inter + JetBrains Mono mapped onto Halo's type styles?
15. **Animated numbers:** count-up (loads once) vs odometer roll for these stats? Halo says count-up for load-once, roll for live-in-place values; hub stats change on range click and every 60s window slide.
