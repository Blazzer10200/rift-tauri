---
title: Rift · Halo — spec 13 · Home + onboarding (P4d)
status: draft
updated: 2026-10-19
phase: P4d
depends_on:
  - spec/00-decisions.md (D-01 D-02 D-03 D-05 D-06 D-07 D-10 D-12 D-13 D-14 D-15 D-16 D-17 D-24 D-25 D-29 D-30 D-31 D-32 D-33)
  - spec/01-foundation.md (P1 tokens, --t-*-size/--t-*-line names, density vars, C12 accent literals, C19 isAccent/pickAccent)
  - spec/02-kit.md (P3 primitives K10-K30, motion helpers K04*)
  - spec/03-motion.md (P2 motion.css, M12 M14 M16 M17 M20 M23 M26 M30 M33 M34 M37, transitions.ts accordion carrier B)
  - P4a (tooltip layer + Toaster mounted app-wide, "Home" naming) and P4b (AssistantWelcome port) merged first (D-27 order)
  - map/05-home-onboarding.md, map/09-motion-inventory.md, map/12-owner-preferences.md
tags: [rift, halo, spec, p4d, home, onboarding]
---

# Spec 13 — Home + onboarding (P4d)

Paths are relative to `C:/AI Workflow/projects/rift-tauri`. Every `path:line` was opened on `design/chat-ledger @ 22727d24` on 2026-10-19. Line numbers are pre-P1; P1-P3 shift some of them, so each commit re-anchors by the quoted snippet, not the number. `c.sh` = `bash scripts/cdp/c.sh`. Type tokens use the P1 names (`--t-small-size` / `--t-small-line`, spec 01 §2.6); where spec 02 writes `var(--t-small)` / `--t-small-lh` it means the same pair.

## 0. Summary

- Home (`WorkspacePage`) moves onto the kit: one `PageHeader` + one 1080 column, numbered SectionLabels `01 / PROJECTS · 02 / ACTIVITY · 03 / AI UPDATES`, flat project Cards with neutral initials, five StatTiles on a Rift formatter, Rift dense `DailyBars`, a `ShareBar` model mix, a Segmented range control. The 38px gradient numeral, per-project hues, 5-hue mix, filter hovers and box-shadow pulse go.
- Motion arrives only when data does: count-up and bar growth on first data and on range change, never on the 60s tick; idle Home runs 0 animations.
- NewsFeed loses the amber wash and becomes a Halo accordion card (carrier B, Rift header with a sibling refresh button). `LimitBar` becomes a thin adapter over `ui/progress` (API unchanged, 6 call sites untouched).
- Onboarding keeps its owner-built structure (rail + stage + 4 steps) on flat Halo material: Halo vertical Stepper, kit buttons/RadioGroup/Segmented/Field, demos that play once and hold a composed final frame with a Replay button, no dialog role, Esc on step 1 no longer ends setup.
- The connect cards (`ClaudeConnect`, `OpenAiConnect`) are shared with the chat welcome; their commits land first so the welcome changes once. 36 commits + 1 gated (09g), L, 3 sessions. Board 05 decides 8 open taste numbers.

## 1. Scope + files

### 1.1 Edited in P4d

| File | Lines | What changes |
|---|---|---|
| `src/lib/components/workspace/WorkspacePage.svelte` | 1206 (script 1-393, markup 395-801, style 803-1206) | markup `:408-801` rebuilt on kit parts; style `:803-1206` replaced (~260 lines); script: `SEG_HUES`/`segs`/`mix` `:115-131` deleted (presentation, see §6 commit 30), `projectHue` import `:24` and `monogram` kept, `trendChip` snippet `:398-406` deleted; every other script line untouched |
| `src/lib/components/workspace/NewsFeed.svelte` | 337 (script 1-88, markup 90-215, style 217-337) | markup + style; script untouched except `stepIdx` label gets a `{#key}` crossfade wrapper in markup |
| `src/lib/components/workspace/DailyBars.svelte` | new (~120) | Rift dense bar chart (§5.2) |
| `src/lib/components/shared/Sparkline.svelte` | new (~70) | Halo Sparkline port, shared with 4f (§5.2) |
| `src/lib/components/shared/LimitBar.svelte` | 82 | body → `ui/progress` adapter; API kept (§5.2) |
| `src/lib/components/shared/PageHero.svelte` | 48 | one `@deprecated` comment line only (consumers swap in 4e/4f) |
| `src/lib/utils/usageStats.ts` | 280 | 3 formatters added after `fmtCost` `:243-248` |
| `src/lib/utils/usageStats.test.ts` | 159 | tests for the 3 formatters |
| `src/lib/components/onboarding/OnboardingFlow.svelte` | 392 (script 1-165, markup 167-392) | markup on kit parts; `onEscape` `:99-107` (commit 18); no other script change |
| `src/lib/components/onboarding/Stepper.svelte` | new (~90) | Halo vertical Stepper (§5.2, kit gap) |
| `src/lib/components/onboarding/ObStage.svelte` | 67 | Replay control, `presetLabel` prop, aria split |
| `src/lib/components/onboarding/ClaudeConnect.svelte` | 226 (script 1-128, markup 130-226) | markup on kit parts; copy reset 1600 → 1800 (`:91,101`) |
| `src/lib/components/onboarding/OpenAiConnect.svelte` | 78 | markup on kit parts |
| `src/lib/styles/onboarding.css` | 548 | rewritten on tokens; retired rules moved to an end block (§8) |
| `src/routes/dev/ui/+page.svelte` | 445 | exhibits for DailyBars, Sparkline, Stepper |
| `design-system/ui-baseline.json` | 8 | `check:ui --update` after counts drop |

### 1.2 Read, not edited (blast radius)

- `src/lib/components/assistant/AssistantWelcome.svelte:148-159,707-715` mounts both connect cards with `standalone` inside a 440px `.auth-connect` column. 4d changes how it looks; the file is not touched (owned by 4b).
- LimitBar call sites: `ai-health/AiHealthPage.svelte:774,781,1097,1122` (`variant="card"`), `shell/StatusBar.svelte:206` (`compact`), `assistant/composer/UsagePanel.svelte:242` (`panel`, `live`). Zone strings come from `AiHealthPage.svelte:204-206` and `state/usage.svelte.ts:46-50`: `ok | warn | hot`.
- PageHero call sites: `settings/SettingsPage.svelte:626`, `ai-health/AiHealthPage.svelte:721,824`, `diagnostics/DiagnosticsPage.svelte:13`.
- `src/lib/components/AppShell.svelte:48-59,304-309,503-512` (gate, `.ob-host`), `scripts/cdp/c.sh:548` (`.ob-host` probe), `src/lib/state/onboarding.svelte.ts` (`rift.onboarding.dismissed`), `src/lib/state/betaNotice.svelte.ts:7` (`rift.betaNotice.ack`).
- `projectHue` other consumers: `shell/ConversationList.svelte:297`, `shell/ProjectSwitcher.svelte:28,212`, `assistant/AssistantPane.svelte:673` (owned by 4a/4b).
- `.branch-pill` at `src/app.css:682`: only consumer is `WorkspacePage.svelte:423`.

### 1.3 Preconditions (stop and land in the owning phase if missing)

| Need | Owner | Check |
|---|---|---|
| Halo tokens, `--t-*`, density vars incl. `--card-pad`, `--control-h`, `--control-dense-h` | P1 / K01 | Grep `--card-pad` in `src/app.css` |
| `motion.css`: `.page-sections`, `.rise-page`, `halo-rise`, `halo-draw`, net, `.ws-page[data-active=false]` pause | P2 M04, M30 | Grep `page-sections` in `src/lib/styles/motion.css` |
| `transitions.ts` `accordion` carrier B, `rm()`, `prefersReducedMotion()` reactive | P2 M10, M11 | Grep `export const accordion` |
| P2 already did on these files: spinners → `.halo-spin` (M12), `ob-live` → `.live-dot` (M14), `orb-breathe` gone (M16), LimitBar pulse/sheen gone (M17), active-card pulse gone (M20), static skeleton (M23), demos play once (M26, see §13 S-1), `class="page-sections"` on the Home column (M30), fills `scaleX` (M33), spine `scaleY` (M34), footer blur gone (M37) | P2 | Grep `livePulse\|orb-breathe\|limitbar-pulse` = 0 |
| Kit: Button, IconButton, Card, Badge, Spinner, SectionLabel, EmptyState, Banner, Skeleton, Segmented, Field/Input/Textarea, StatTile, ShareBar, Progress, PageHeader, Collapse, RadioGroup, Popover, `shared/Select`, `countUp`, `haloFlip` | P3 K10-K30 | `ls src/lib/components/ui` |
| Tooltip layer mounted app-wide | P4a (spec 02 A-5) | without it tooltips stay a11y-only; exact values still reach `aria-label` |
| P1 C19 `uiPrefs.isAccent/pickAccent` and the Electric blue swatch | P1 | Grep `pickAccent` in `OnboardingFlow.svelte` |

### 1.4 Out of scope

Settings/Health/Diagnostics headers (4e/4f swap PageHero themselves), AssistantWelcome layout (4b), StatusBar/UsagePanel/AiHealth markup (they only re-render through LimitBar), `projectHue` on sidebar/switcher/pane (4a/4b), sidebar "Home" naming and `c.sh nav` labels (4a), dial removal (gated #5, commit O11g only), every deletion (§8).

## 2. Target design

All values are compact (default). Rows marked † change with density: `--card-pad` 16/20/24, `--control-h` 32/36/44, `--control-dense-h` 28/32/36.

### 2.1 Home page shell

```
.ws-page[data-workspace=home]
└ .home  (container: home / inline-size; --page-col: 1080px)         bg transparent → island bg-100 shows
  └ .home-scroll  (the ONE scroll viewport; scrollbar Halo thin, D-23)
    ├ PageHeader maxWidth=1080                                        padding 24 32 0 (inside the scroll, so the
    │   eyebrow  HOME                                                 scrollbar never shifts header vs body)
    │   title    rift-tauri                          [Switch folder]  ← actions: Button ghost sm, Folder 16
    │   children ⎇ design/halo  ·  4,812 files                        ← meta row (see below)
    └ .home-col.page-sections   max-width var(--page-col); margin 0 auto; padding 20 32 24; gap 20
        ├ section.projects   (child 1)
        ├ section.activity   (child 2)
        └ section.news       (child 3)
```

| Element | Spec | Resolved |
|---|---|---|
| Column | `max-width: var(--page-col)`; header and body share it (D-31, one edge) | 1080; edge = (container − 1080)/2 when container ≥ 1144 |
| PageHeader eyebrow | `eyebrow={{ label: "Home" }}` (no index; indices belong to sections) | mono 11/14 500 .12em caps `--ink-faint` #7d8592 |
| PageHeader title | `hasRoot ? ctxName : "Your projects"` (logic `:418` unchanged) | 24/30 600 −.02em `--ink` #eceff3 |
| Lede | none (the meta row carries the context) | — |
| Meta row (children) | flex, gap 8, height 20, `margin-top: 8px`; only when `hasRoot` | — |
| ↳ branch | `<Badge mono>` + `GitBranch` 12 + `{branch}` | bg-300 #1b1f26, ink-muted #9aa2af, h20, pill |
| ↳ files | `small` 13/18 `--ink-muted`, number `tabular-nums` `--ink`: "4,812 files" | — |
| ↳ separator | none; gap 8 only | — |
| Switch folder | `Button variant="ghost" size="sm"` + `Folder` 16, label "Switch folder", `onclick` `:426` unchanged | h28 r8, ink-muted → hover bg-200/ink |
| Section gap | 20 | — |
| Section head | `div.sec-head` flex, align center, gap 12, height 28, margin-bottom 12: `<SectionLabel index count>` (flex 1, its rule fills) + trailing control | label mono 11/14; rule `--line-100` #1f242c; index + count per spec 02 K17 |

**Fit budget** (D-31 "fit one screen"; checked at the audit viewport 1638×883 CSS px, which is stricter than the 1600×1000 window; compact; 6 projects; news collapsed):

| Block | Height |
|---|---|
| PageHeader: 24 top + eyebrow 14 + 6 + title 30 + 8 + meta 20 | 102 |
| column top padding | 20 |
| 01 Projects: head 28 + 12 + 2 rows × 96 + 12 | 244 |
| gap | 20 |
| 02 Activity: head 28 + 12 + card (16 + stats 76 + divider 33 + viz 128 + 16 = 269) | 309 |
| gap | 20 |
| 03 AI updates: head 28 + 12 + closed accordion 44 | 84 |
| column bottom padding | 24 |
| **Total** | **823** |

Commit 23 (the shell commit) first records the measured `.home-scroll` `clientHeight` at the audit viewport into this table. If it is below 823, apply levers in this order, one commit each, and stop when it fits: DailyBars plot 104 → 88 (−16); section gap 20 → 16 (−8); PageHeader top 24 → 16 (−8); column bottom 24 → 16 (−8). If it still overflows, report it; do not shrink type.

**Overflow.** The page scrolls as a whole (as today, `:809-811`) when news is open or the window is short. Projects beyond 3 rows scroll inside the grid region (D-31 "lists scroll inside their cards"): `.proj-scroll { max-height: calc(3 * var(--pcard-h) + 2 * 12px + 4px); overflow-y: auto; padding-top: 4px; margin-top: -4px; }` (4px = lift room). Board 05 lever `data-prop-overflow` decides between this and page scroll (§14 Q-3).

**Width.** At ≥1160px container width (`@container home (min-width: 1160px)`) the Activity viz row is two columns; below it stacks. D-31's "two columns at ≥1160 inside the same edge" is read as the page container width, since the column itself never exceeds 1080 (§13 A-6). Window min 900 (`tauri.conf.json:18`) → container ≈ 620: everything single column, stat grid `repeat(auto-fit, minmax(150px, 1fr))`.

### 2.2 01 / Projects

```
01 / PROJECTS ───────────────────────────────────────────── 6   [+ New project]
┌─────────────────────────────┐ ┌─────────────────────────────┐ ┌───────────────────────────┐
│ (RT) rift-tauri  ●ACTIVE  ⧉ ✎│ │ (CX) consolex            ⧉ ✎│ │ (CP) cuepilot          ⧉ ✎│
│      …/projects/rift-tauri  │ │      …/projects/consolex    │ │      …/projects/cuepilot  │
│ ▢ 42 chats  ◷ 2h  $ $18.40  Continue → │ ▢ 9 chats ◷ 3d  $2.10      │ No chats yet              │
└─────────────────────────────┘ └─────────────────────────────┘ └───────────────────────────┘
```

| Element | Spec | Resolved |
|---|---|---|
| Head control | `Button variant="ghost" size="sm"` + `Plus` 16, "New project" (D-32 sentence case; was "New"), `onclick={() => startNew()}`; tooltip removed (repeats text, D-17) | h28 r8 |
| SectionLabel | `index={1} count={projects.items.length || undefined}` "Projects" | `01 / PROJECTS ─── 6` |
| Grid | `.proj-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); grid-auto-rows: var(--pcard-h); gap: 12px }`; `.solo` → 1 column (`:589` logic kept); 1 column under 680 container | 3-up at 1080 (3 × 352) |
| `--pcard-h` † | `calc(2 * var(--card-pad) + 64px)` (32 avatar row + 12 gap + 20 foot) | 96 / 104 / 112 |
| Card | `<Card as="article" interactive pad="default">` + inner `<button class="pcard-open">` (structure `:594-629` kept: actions are siblings, no nested buttons) | bg-100 #0e1014, 1px line-100 #1f242c, r16, pad 16†; hover border line-200 #2c323c (300ms), lift `translate: 0 -3px` (150ms, D-16; lever 2/3 on board 05) |
| Focus | `:focus-within` on the Card draws `--shadow-focus` around the whole card (fixes the 7px inner ring, map 05 §8 #11); `.pcard-open:focus-visible` itself gets `outline: none` | 2px bg-100 + 2px accent |
| Active card | border stays line-100 → **line-200 at rest** (C12 "calm it"); no wash, no gradient | accent appears only on avatar ring, Badge, Continue |
| Avatar | 32×32 circle, `background: var(--bg-300)`, `color: var(--ink-muted)`, 600 13/1 sans, initial from `monogram()` `:383` | #1b1f26 / #9aa2af |
| Avatar active | `box-shadow: 0 0 0 2px var(--bg-100), 0 0 0 3px var(--accent)`; colour `--ink` | 1px accent ring outside a 2px gap |
| Name | `small` 13/18 600 `--ink`, ellipsis | — |
| Active badge | `<Badge tone="accent" dot>Active</Badge>` beside the name; dot static (D-13, 03 #125) | accent-soft rgba(106,168,255,.14) / #6aa8ff, h20 |
| Path | mono 12/16 (`--t-caption-*`, `--font-mono`) `--ink-faint`, ellipsis, `use:tooltip={prettyPath(p.root)}` (truncated path, D-17) | — |
| Actions | `.pcard-actions` absolute top 12 right 12, gap 4: `<IconButton size="sm" label="Open in split">` (when `assistant.canAddPane`) + `<IconButton size="sm" label="Edit project">`; icons 16; at rest `opacity: .6`, `1` on card `:hover`/`:focus-within` (D-32) | 28×28 r8 ghost |
| Foot | flex wrap, gap 4 12; metas `caption` 12/16 `--ink-muted` `tabular-nums`, icons 14 `--ink-faint` (`MessageSquare`, `Clock`, `DollarSign`); "No chats yet" `--ink-faint`, not italic | — |
| Scope chip | `<Badge mono>` + `Filter` 12 + `scopeLabel(p)` | neutral |
| Continue | `span.pcard-go` `margin-left: auto`, `small` 13/18 500; idle card: hidden, revealed on hover/focus-within (opacity 0 → 1, `translateX(-4px)` → 0, 150ms); active card: visible, text "Continue", `--accent` | ArrowRight 16 |
| Re-sort | `animate:haloFlip` on `article` inside `{#each sortedProjects as p (p.id)}` (460ms, `easeOut`, 0 under reduced motion) | — |
| Save current | `<Banner tone="accent" title={`Save ${ctxName} as a project`} description="Pins it to the sidebar with a sensible file scope.">` + `action`: `<Button variant="secondary" size="sm" onclick={adoptActive}>Save project</Button>`; icon `FolderPlus`; condition `:640` unchanged; margin-top 12 | bg = accent 8% over bg-100, border accent 32% over line-100, r12 |
| Load error | `<Banner tone="danger" title="Couldn't load projects" description={projects.lastError}>` + action `Button secondary sm` "Retry" → `projects.refresh()` | role=alert |
| Empty | `<EmptyState compact title="No projects yet" description="Point Rift at a folder. It names the project and skips build output for you.">` icon `FolderGit2`, action `<Button variant="primary">New project</Button>` (the only solid accent on the page in that state) | 56 mark, 1 ring |

### 2.3 Project editor (inline, inside 01 / Projects)

The editor moves from a direct child of the column (`:435`) to the first element after the Projects section head. Reason: `.page-sections > *` would otherwise give it the 520ms page entrance on every open (§13 A-7). Logic `:173-324,437-438` unchanged.

| Element | Spec |
|---|---|
| Container | `<Card as="section" role="form" aria-label=…>`; margin-bottom 12; entrance `--rise: var(--rise-page); animation: halo-rise var(--dur-base) var(--ease-out) both` (10px, 300ms); unmount instant |
| Head | title `heading` 16/22 600 ("New project" / "Edit project"); description `small` `--ink-muted` (copy `:443-445` kept); close `<IconButton label="Cancel">` X 16. The accent icon tile `.ed-ic` goes (accent scarcity) |
| Grid | `.id-grid` 2 columns `minmax(0,1.9fr) minmax(200px,1fr)` gap 12; 1 column under 760 container |
| Folder | `<Field label="Folder">` + `<Input class="font-mono">` bound to `dRoot` / `folderEl`, handlers `:456-458`; trailing `<IconButton variant="secondary" size="default" label="Recent folders">History</IconButton>` as a `ui/popover` trigger, then `<Button variant="secondary">` FolderOpen "Browse" |
| Recents popover | `ui/popover` content (portalled, `--z-dropdown` 65, pop-in 150 / out 140); heading `menuLabel` "Recent folders"; rows 32 (`--list-row-h`): pick button (Folder 16, name `small` 500, path mono caption `--ink-faint`) + `<IconButton size="sm" label="Forget folder">` visible on row `:hover` **and** `:focus-within` (D-30). `recentOpen` stays the bound open state |
| Name | `<Field label="Name" hint="Filled in from the folder">` + `<Input>` (`:493-495`) |
| File scope | trigger: full-width ghost row, `ChevronDown` 16 `.chev` (rotates 180°), "File scope" `small` 500, summary `caption` `--ink-muted` (`--danger` when `advForced`); `aria-expanded`/`aria-controls="ed-adv"` kept. Body: `<Collapse open={advOpen || advForced} id="ed-adv">` always mounted, `inert` closed; replaces `{#if}` `:507` |
| Globs | three `<Field>` + `<Textarea class="font-mono" rows=4>`; invalid → Field `error` (`--danger`, CircleAlert 14) replacing `.glob-err`; count chip → Field `action` snippet `caption` mono `--ink-muted` |
| Footer | left `<Button variant="danger">` Trash2 "Delete" (edit only); right `<Button variant="ghost">Cancel</Button>` + `<Button variant="primary" loading={saving} disabled={!canSave}>` Check "Create"/"Save" |

### 2.4 02 / Activity

```
02 / ACTIVITY ──────────────────────────────────────────────── [ 7D │ 30D │ ALL ]
┌──────────────────────────────────────────────────────────────────────────────┐
│ Messages          Sessions   ↑ +12%  Tool calls  ↓ −8%  Est. spend ↑ +4%  Streak │
│ 12.4k             86                 1.9k                $8.2k  ╱╲╱    12d    │
│ 31 active days    busiest around 3 pm                    API-equivalent  best 37d │
│──────────────────────────────────────────────────────────────────────────────│
│ Messages per day · last 30 days      Peak 412 │ Model mix                    │
│ ▁ ▃ ▅ █ ▂ ▁ ▃ … (DailyBars, plot 104)          │ ████████████▆▆▆▆▃▃▂ 8px      │
│                                                │ ● Sonnet 5 61%  ● Opus 5.5 27% │
│                                                │ ● Haiku 4.5 9%  ● Other 3%    │
│                                                │ ≈ 412k words exchanged …      │
└──────────────────────────────────────────────────────────────────────────────┘
```

| Element | Spec | Resolved |
|---|---|---|
| Range | `<Segmented ariaLabel="Time range" size="sm" mono options={[{value:"7d",label:"7D"},{value:"30d",label:"30D"},{value:"all",label:"All"}]} bind:value={range}>` neutral thumb (spec 02 A-6) | h28, thumb bg-300 + inset line-200, slide 300ms |
| SectionLabel | `index={2}` "Activity", no count | — |
| Card | `<Card as="div">` pad 16†, `.act-card` grid rows: stats / divider / viz | flat, no island fill, no `--radius-2xl` gradient |
| Stat row | `display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 16px`; under 1160 container `repeat(auto-fit, minmax(150px, 1fr))` | 5 × ~196 |
| Divider | `<Separator>` margin 16 0 | 1px line-100 |
| Viz row | ≥1160: `grid-template-columns: minmax(0,1.45fr) minmax(0,1fr); gap: 24px`; the mix column gets `border-left: 1px solid var(--line-100); padding-left: 24px`; below: stacked, gap 16, rule on top instead | — |
| Bars caption | `caption` 12/16 `--ink-muted` "Messages per day · last {windowDays} days"; right `caption` `tabular-nums` "Peak {fmtInt(series.max)}" when `series.max > 0` | sans (D-05), not mono |
| DailyBars | §5.2; plot height 104 (lever) | — |
| Mix caption | `caption` `--ink-muted` "Model mix" | the "· mostly X" suffix goes (K5: the legend's first row says it) |
| ShareBar | §5.2 adoption; legend 2 columns (`grid-template-columns: repeat(2, minmax(0,1fr)); gap: 6px 16px`) | chart-1 #6aa8ff, chart-2 #b79cff, chart-3 #9cc6ff, Other line-200 |
| Fun fact | `caption` 12/16 `--ink-muted`, not italic, margin-top auto (pinned to the column bottom), `{#if fact}` `:763` | — |

**StatTiles** (`ui/stat`, spec 02 K25). Common props: `rangeKey={range}`, `loading={statsLoading}`, `value={statsLoading ? null : <v>}`. Never key them on `statsNow` (the 60s tick re-derives `totals`, `:78-84`; StatTile snaps on non-range changes, D-16).

| # | label | value | format / exact | delta | detail |
|---|---|---|---|---|---|
| 1 | Messages | `totals.messages` | `fmtCompact` / `n => fmtInt(n) + " messages"` | — | `` `${fmtInt(totals.activeDays)} active day${…}` `` |
| 2 | Sessions | `totals.sessions` | `fmtInt` / — | `trends?.sessions` | `peak != null ? "busiest around " + hourLabel(peak) : undefined` |
| 3 | Tool calls | `totals.toolCalls` | `fmtCompact` / `fmtInt` | `trends?.toolCalls` | — |
| 4 | Est. spend | `totals.cost` | `fmtCost` / `n => "$" + n.toFixed(2)` | `trends?.cost`, `goodWhen: "down"` | "API-equivalent value"; children: `<Sparkline values={series.cells.map(c => c.cost)} rangeKey={range} />` (commit 31, droppable) |
| 5 | Streak | `strk.current` | `fmtDays` / — | — | `` `best ${fmtDays(strk.longest)}` `` |

- Delta: pass `delta={pct == null || pct === 0 ? undefined : { value: pct, goodWhen, format: fmtSignedPct }}` (today's hide rules `:399`, `:98-99`). Good = accent-soft/accent, bad = warn-soft/warn (#ff9f5a), arrow + sign (D-24). This changes today's neutral-grey "down" to warn, as D-03 decides.
- Tile 4 is the one destination (`:729`): wrap the StatTile in `<button type="button" class="stat-link" onclick={() => workspace.setActive("ai-health")}>` styled as a hover surface (r8, padding 8, margin −8, hover bg-200 150ms, focus `--shadow-focus`, `-webkit-app-region: no-drag` kept from `:931`), `aria-label="Est. spend {fmtCost(totals.cost)}, open AI Health"`.
- Values: metric 28/32 mono 600 −.03em `--ink`; label `small` `--ink-muted`; detail `caption` `--ink-muted`. No icons on tiles (Halo anatomy; the 4 lucide icons `:722-736` go).

**States of the Activity block** (replaces `:666-692`): loading → the card renders with StatTiles `loading` (Skeleton `stat` bones after 300ms) and `<Skeleton loading variant="text" lines={4}>` in each viz column; error → `<Banner tone="danger" title="Couldn't load activity" description={statsError}>` + Retry `Button secondary sm` → `loadStats`; empty → `<EmptyState compact title="No activity yet" description="Messages, sessions and tool calls show up here after your first chat.">` + `<Button variant="secondary" size="sm" onclick={() => workspace.setActive("chat")}>Open chat</Button>` (D-32 one action; §14 Q-6).

### 2.5 03 / AI updates + NewsFeed

The strip keeps `{#if newsOpen}` (`:792`) so NewsFeed's 60s tick and digest-step interval (`NewsFeed.svelte:23-26,70-80`) do not run while closed. Motion comes from P2's carrier B: `<div transition:accordion>` (spec 03 §5.2, `transitions.ts`). A bits-ui Accordion is not used: its `forceMount` would keep NewsFeed mounted, and the refresh button must sit beside the trigger, not inside it.

```
03 / AI UPDATES ────────────────────────────────────────────────────────────────
┌──────────────────────────────────────────────────────────────────────────────┐
│ [▤] AI updates   Provider release notes and optional digests     2h ago [↻] ⌄ │  44 high
├──────────────────────────────────────────────────────────────────────────────┤  (open)
│ ⌨ Source: official Claude Code release notes · Connections live in Settings → Providers │
│ ✦ Summarize recent Claude news                                   [Summarize] │
│   Claude searches official sources for recent Anthropic and Claude Code news. │
│   $ Runs one Claude turn, billed like a chat message.  (warn)                 │
│ CLAUDE CODE RELEASES                                                          │
│ v2.1.84 ↗                                                          3d ago     │
│ • …  • …  • …  • …                       ⌄ 6 more notes                       │
│ [ ⌄ Show 11 more releases ]                                                   │
└──────────────────────────────────────────────────────────────────────────────┘
```

| Element | Spec |
|---|---|
| Item | `.news-item`: bg-100, 1px line-100 → line-200 when open (300ms), r12 (Halo accordion `cards`, `bundle.css:485-486`) |
| Header row | flex, min-height 44, padding 0 8 0 16, gap 12. Toggle `<button class="news-toggle" aria-expanded aria-controls="ws-news-body">` (flex 1): `Newspaper` 18 `--ink-muted` (→ `--accent` when open, Halo `:494`), "AI updates" `small` 500 `--ink`, hint `caption` `--ink-faint` when closed (copy `:780`, sentence case), `ChevronDown` 18 `.chev` (180°, 300ms) at the end of the row |
| When open, siblings of the toggle | "2h ago" `caption` `tabular-nums` `--ink-faint` with `use:tooltip={"Last checked"}`; `<IconButton size="sm" label="Refresh feed">` whose icon is `RefreshCw` 16, or `<Spinner size={16} decorative>` while `news.status === "checking"` (`loading` prop) |
| Body | `<div id="ws-news-body" class="acc-inner-host" transition:accordion>` padding 0 16 16; `<NewsFeed embedded />` |
| Coverage | one line, `caption` `--ink-faint`, `Terminal` 14: "Source: official Claude Code release notes · Connections live in Settings → Providers." No box (the 3% wash `NewsFeed:229` goes) |
| Digest CTA (no amber wash) | row: `Sparkles` 16 `--ink-muted`; text column: title `small` 500 `--ink`, description `caption` `--ink-muted`, cost line `caption` `--warn` #ff9f5a with `CircleDollarSign` 14 ("Runs one Claude turn, billed like a chat message." / "Connect Claude to run this optional digest." `:160` logic kept); trailing `<Button variant="secondary" size="sm" disabled={!claudeReady} onclick={() => void news.summarize()}>Summarize</Button>`. The gradient `:238`, warn border `:237`, warn icon tile `:243-244` and hover lift `:240` go. Cost stays amber at the control (C3, §8) |
| Digest loading | row: `<Spinner size={16}>`, step text `small` `--ink-muted` wrapped in `{#key stepIdx}<span class="step-label">` (opacity 0 → 1, 150ms; 03 #129), `<Progress value={null} size="sm" class="w-24">` (96px, indeterminate 1.3s). Orb + `dl-slide` gone |
| Digest head | `Sparkles` 14 + "Claude digest" `caption` 500 `--ink`, "· 2h ago" `--ink-faint`, `<Button variant="ghost" size="sm">Refresh</Button>` with `use:tooltip={"Runs one Claude turn"}` |
| Digest items | list, rows separated by hairlines (no per-item box): padding 10 8, r8, hover bg-200 150ms; `<Badge tone={d.tag === "model" ? "accent" : "neutral"}>` icon 12 + label; title `small` 500 + `ExternalLink` 14 `--ink-faint` **visible at rest** (D-32); summary `caption` `--ink-muted`; date `caption` `--ink-faint` |
| Releases sub-head | `caption` 500 `--ink-muted` + `Terminal` 14: "Claude Code releases" (sentence case, not caps) |
| Release | rows with a hairline between, padding 12 0; version `<button class="rel-ver">` mono 13/18 (`--t-data-*`) 500 `--ink`, `ExternalLink` 14 `--ink-faint` at rest, hover `--accent`; date `caption` `--ink-faint` right; bullets `small` 13/18 `--ink-muted`, 6px dot `--line-300`; maintenance line `small` `--ink-faint` (opacity .78 rule goes) |
| More notes | bullets beyond `BULLET_PREVIEW` render inside `<Collapse open={isOpen}>`; trigger `<Button variant="ghost" size="sm">` `ChevronDown` `.chev` + "6 more notes" / "Show less" |
| More releases | releases beyond `RELEASE_PREVIEW` inside `<Collapse open={showAllReleases}>`; trigger full-width `<Button variant="secondary" size="sm">` "Show 11 more releases" / "Show fewer" |
| States | checking + empty list → `<Skeleton loading variant="list" lines={2}>`; error → `<Banner tone="danger" title="Couldn't load updates" description={news.error}>` + Retry (secondary sm); none → `<EmptyState compact ground="100" title="No updates yet" description="Claude Code release notes appear here when they're published.">` (no action: Refresh already sits in the header) |
| `embedded={false}` header `:94-113` | kept for API; restyled to the same header row look (no consumer passes `false` today) |

### 2.6 LimitBar → Progress adapter

```svelte
<div class="limitbar {className}" data-zone={zone} data-variant={variant} aria-hidden="true">
  <Progress value={pct / 100} tone={TONE[zone] ?? "accent"} size={variant === "compact" ? "sm" : "default"} showValue={false} />
</div>
```

| Variant | Track | Width | Fill | Notes |
|---|---|---|---|---|
| panel (UsagePanel) | 8px bg-300 pill | parent | scaleX, 700ms ease-out | `live` accepted, ignored (glow killed, D-16); flagged §8 |
| card (AiHealth) | 8px (was 7) | parent | same | sheen gone (P2 M17) |
| compact (StatusBar) | 4px | 46px; 34px under 720 (`:71-73` kept) | same | hot glow `:70` gone |

`TONE = { ok: "accent", warn: "warn", hot: "danger" }`. `aria-hidden` keeps today's decorative semantics (callers render the % text and labels themselves). Gradient fill `:39`, three fill recipes, `limitbar-grow` and the per-variant reduced-motion block go; the P2 net covers motion.

### 2.7 Onboarding

```
bg-000 canvas, padding clamp(24px, 3vw, 48px)
┌ .ob-card  max 1180 × min(820px, 100vh − 96px) · bg-100 · 1px line-100 · r16 · no shadow ─────────────┐
│ .ob-rail 312 · bg-100 · border-right line-100 │ .ob-main                                              │
│ pad 32 28 24                                  │  .ob-body (scroll) → .ob-step-wrap max 640, gap 24,   │
│ [mark 28] Rift  (Setup)                       │   padding 48 0 40, class="page-sections"              │
│                                               │   ┌ ObStage well 220 · bg-000 · line-100 · r12 ──────┐ │
│ (✓)─ Welcome                                  │   │       miniature scene          [↻ Replay]  CAPTION│ │
│  │   What Rift is                             │   └──────────────────────────────────────────────────┘ │
│ (2)─ Connect AI           ← current           │   01 / WELCOME ───────────────────────── 1 of 4        │
│  │   Claude or ChatGPT                        │   Meet Rift                         (title 24/30)      │
│ (3)  Open a project                           │   lede body 15/24 ink-muted                            │
│ (4)  Defaults                                 │   …step content…                                       │
│                                               ├───────────────────────────────────────────────────────┤
│ [Skip setup]                                  │  .ob-foot 64 · bg-100 · border-top line-100 [Back][Next →]│
└───────────────────────────────────────────────┴───────────────────────────────────────────────────────┘
```

| Element | Spec | Resolved |
|---|---|---|
| Overlay `.ob-overlay` | `background: var(--bg-000)`; radial wash `:18-20` and `--gi-ghost-*` aliases `:12-14` go; padding `clamp(24px, 3vw, 48px)` | #08090c |
| Card | bg-100, 1px line-100, `--radius-lg`, no `--shadow-lg` (flat islands D-01: shadows only on floats); entrance `.rise-page` (10px, 600ms, 03 #131) replacing `ob-card-in` | #0e1014 / #1f242c / 16 |
| Rail | bg-100 (no sunken `--bg-inset`, §8 "no sunken panel"), border-right line-100, padding 32 28 24; radial `:39-40` goes | — |
| Brand | `RiftLogo` 28; "Rift" `heading` 16/22 600 −.03em (D-26 brand head); `<Badge>Setup</Badge>` neutral | — |
| Stepper | `Stepper` vertical (§5.2), margin-top 32 | marks 32, gap 24 |
| Skip | `<Button variant="ghost" size="sm">Skip setup</Button>` at the rail foot | h28 |
| Body column | `.ob-step-wrap` max 640 (660 → 640, 8px grid), gap 24, padding 48 0 40, `class="page-sections"` inside `{#key step}` (intended replay, §7) | cascade 520 + 60i |
| Step head | `<SectionLabel index={step} count={`${step} of ${last}`}>{steps[step-1].t}</SectionLabel>` replaces the accent eyebrow `:212` etc.; title `<h1>` `--t-title-*` 24/30 600 −.02em `--ink`; lede `<p>` `--t-body-*` 15/24 `--ink-muted`, max 64ch | index ink-faint (D-31) |
| Value rows (step 1) | rows separated by hairlines, padding 12 0; icon chip 32 r8 bg-200 icon 18 `--ink-muted`; warn row chip `--warn-soft` / `--warn`; title `small` 500 `--ink`; text `small` `--ink-muted` (13, was 11) | — |
| Accent strip (step 1, while #5 pending) | row, gap 12: "Accent colour" `small` `--ink-muted` + swatches 24×24 circles (a swatch is a colour sample, not a pill button); selected `box-shadow: 0 0 0 2px var(--bg-100), 0 0 0 4px var(--sw)` + `Check` 12 in `--bg-000`; inset ring `color-mix(in srgb, var(--ink) 16%, transparent)`; hover: no lift. Selection logic is P1's `isAccent/pickAccent`. Whole strip removed by O11g if #5 = yes | first swatch Electric blue |
| Provider (step 2) | `<RadioGroup variant="cards" ariaLabel="AI provider">` 2 items, value `provider`, `onValueChange={selectProvider}` (side effects `:43-53` unchanged); item: icon 18 (`Orbit`/`Terminal`), title `small` 600, description `caption` `--ink-muted` ("Account through the Codex App Server" / "Command-line tool or API key") | checked: accent border + accent 6% over bg-100 |
| Connect card | §2.8 |
| Soft-block | `<Banner tone="warn" title="{name} isn't connected yet" description="That service can't reply until it is. Press Next again to continue anyway.">` | — |
| Step 3 status | `.ob-statlist` (§2.8) one row: "Workspace" + mono path `--ink` + `<Badge tone="accent">` Check "Selected"; `<Button variant="secondary" size="sm" loading={picking}>` FolderOpen "Change folder" | — |
| Step 3 empty | `<Button variant="primary" loading={picking}>` FolderOpen "Choose a folder"; "Recent" `caption` 500 `--ink-muted` + rows: `<Button variant="ghost" class="w-full justify-start">` History 16 + mono path (rtl ellipsis kept, `:304`); hint `small` `--ink-muted` | rows h32 |
| Model (step 4) | `defaultModels.length <= 4` → `<Segmented ariaLabel="Default model" options={defaultModels.map(m => ({ value: m.id, label: available ? `${m.label} ${m.version}` : `${m.label} · unavailable`, disabled: !available }))} value={assistant.model} onValueChange={id => pickModel(byId(id))}>`; more than 4 (ChatGPT lists 9, `modelMatrix.ts:80-88`) → `shared/Select` with the same options (§14 Q-8) | — |
| Presets | `<RadioGroup variant="cards" ariaLabel="Working style preset">` 3 items stacked (one column), value `activePreset`, `onValueChange={id => pickPreset(byPreset(id))}`; title + `<Badge tone="accent">Recommended</Badge>` on Balanced; blurb `small` `--ink-muted`; field hint `caption` `--ink-muted` | — |
| Footer | height 64, padding 16 28, bg-100 opaque (blur gone P2 M37), border-top line-100; left: mobile "Skip setup" (`<720` only); right: `<Button variant="ghost">` ChevronLeft "Back" (step > 1) + `<Button variant="primary">` "Next" ChevronRight / Check "Start working". Progress dots `:368-373` removed (the Stepper and "1 of 4" carry progress; map 05 §8 #12) | h32 r8 |
| Narrow <720 | rail hidden (`:531-540` kept), card full-bleed r12, body padding 0 20; stage 160; <440 provider cards stack (RadioGroup auto-fit does it), input rows stack | — |
| Semantics | `.ob-overlay` loses `role="dialog" aria-modal` (§13 A-1); `<main class="ob-main" aria-labelledby="ob-step-title">`; `<h1 id="ob-step-title">` per step | — |

### 2.8 Connect cards (shared with the chat welcome)

Global `onboarding.css` stays global: AssistantWelcome mounts these without OnboardingFlow, and scoped styles would strip there (`ClaudeConnect.svelte:11-14`). Only kit parts replace bespoke rules.

| Element | Spec |
|---|---|
| Head `.ob-head` | eyebrow (`ClaudeConnect.svelte:131`, `OpenAiConnect.svelte:34`, non-standalone only, as today) → `<SectionLabel index={2} count="2 of 4">Connect Claude</SectionLabel>` / "Connect ChatGPT". Step 2 in OnboardingFlow therefore renders no SectionLabel of its own and keeps today's order (stage, provider, card head); title `--t-title-*` 24/30 (standalone: `--t-subtitle-*` 18/24, the welcome column is 440); lede `--t-body-*` 15/24 `--ink-muted` (standalone `small`); inline `code` uses `--code-bg`/`--code-border`, r6, mono 12 |
| Status list `.ob-statlist` (was `.ob-statcard`) | 1px line-100, r12, no fill (sits on bg-100), rows min-height 44, padding 0 16, hairline between; key `small` `--ink-muted` + icon 16 `--ink-faint`; value `<Badge tone="accent">` CheckCircle2 12 + text (installed version / email / "API key" / "Connected") or `<Badge tone="warn">` AlertCircle 12 ("Not found" / "Not signed in" / "Sign-in needed"); summary row `small` `--ink-muted` |
| Checking | row with `<Spinner size={16}>` + "Checking the Claude CLI…" |
| Copy block | `.ob-copy-block`: bg-000 (terminal surface D-18), 1px line-100, r12, padding 8 8 8 16; command mono 13/18 `--ink`, `overflow-x: auto` with `scrollbar-width: none`; `<Button variant="ghost" size="sm">` Copy 16 "Copy" → Check 16 "Copied" in `--accent` for 1800ms; failure keeps today's `error` path (shown, D-32) |
| Labels | `caption` 500 `--ink-muted`, sentence case (was 10px caps) |
| Sign in | `<Button variant="primary" loading={assistant.loginInProgress}>` LogIn "Sign in with Claude" / "Waiting for sign-in…" |
| Hints `.ob-hint` | `small` 13/18 `--ink-muted`; `.ob-link` = accent text, underline offset 2, `:focus-visible` `--shadow-focus` r6 |
| API key | `<Field label="Anthropic API key" hint="Billed per use through your Anthropic Console account.">` + `<Input type="password" placeholder="sk-ant-…" autocomplete="off">` (adds the missing label, map 05 §8 #11) + `<Button variant="secondary" loading={savingKey}>` KeyRound "Save key"; the separate hint `:209` folds into the Field hint |
| Connected | `<Banner tone="accent" title="Claude is connected and ready to work.">` icon Check (static; no LiveDot: "connected" is a state, not activity, D-13) |
| OpenAI install | `<Banner tone="warn" title="Install the Codex CLI first" description="Run npm install -g @openai/codex, then Re-check.">` |
| Route note | row, no box: KeyRound 16 `--ink-faint` + `small` `--ink-muted`, strong `--ink` |
| Error | `small` `--danger` + CircleAlert 16 |
| Re-check | `<Button variant="secondary" size="sm" loading={probing|checking}>` RefreshCw "Re-check" |

### 2.9 ObStage (preview well)

| Element | Spec |
|---|---|
| Well | height 220 (narrow 160), bg-000, 1px line-100, `--radius-md` 12; radial `:369` and dot grid `:373-377` go (C7, §8 "no specks or dots"; D-31 preview wells are bg-000, no fake chrome) |
| Caption | absolute left 16 bottom 12, mono 11/14 500 .12em caps `--ink-faint` |
| Replay | absolute right 8 bottom 8, `<Button variant="ghost" size="sm">` RotateCcw 16 "Replay"; hidden when `prefersReducedMotion()` |
| Scenes | text 12/16 (`--t-caption-*`; mono for chips and paths); user bubble bg-200 + line-100, r `12 12 4 12`; chips bg-100 + line-100 r8; reply lines bg-300 (last line accent 34% → `--accent-soft`); terminal bg-100 line-100 r8, `$` `--ink-muted` (no accent prompt, D-18), caret 7×14 `--ink` solid; "Connected" pills = Badge look (accent-soft/accent); tree rows mono 12 `--ink-muted`, root `--ink` 500, root icon `--ink-muted`; scan band accent 8%; dial dots 8px accent, no glow; all glows/box-shadows gone |

## 3. States

| Surface | State | Trigger (code) | Renders |
|---|---|---|---|
| Home | first open | workspace mounts | sections cascade (P2 `.page-sections`), then idle: 0 animations |
| Home | later activation | `data-active` true | `.ws-page` glide (P2); nothing inside replays |
| Home | hidden keep-alive | `data-active` false | loops/entrances paused (P2); CountUp rAF only runs if a value changes (risk R-7) |
| Header | no root | `!hasRoot` | title "Your projects", no meta row, no Switch folder |
| Header | root, branch loading | `branch == null` | meta row shows files only; no badge placeholder |
| Projects | load error | `!projects.loaded && projects.lastError` `:568` | danger Banner + Retry |
| Projects | none, no folder | `:574` | EmptyState + primary "New project" |
| Projects | none, folder open | `:574` false branch | save-current Banner only |
| Projects | 1 project | `.solo` `:589` | one full-width card |
| Projects | 4-9 | — | 3-up grid, 2-3 rows |
| Projects | >9 | — | grid region scrolls after 3 rows (board lever) |
| Card | rest / hover / focus-within / active | — | §2.2 rows |
| Card | no chats | `pulse.chats === 0` | "No chats yet" `--ink-faint` |
| Card | scoped | `scopeLabel(p)` non-empty | neutral mono Badge |
| Card | split unavailable | `!assistant.canAddPane` | Edit only |
| Editor | new / edit | `isNew` | Delete only in edit |
| Editor | recents open | `recentOpen` | Popover |
| Editor | invalid glob | `advForced` | scope Collapse forced open, Field error, summary `--danger` |
| Editor | saving | `saving` | primary Button `loading` (Spinner + aria-busy), width stable |
| Editor | can't save | `!canSave` | primary disabled .45 |
| Activity | loading <300ms | `statsLoading` | nothing (Skeleton waiting) |
| Activity | loading ≥300ms | — | stat + text bones, static |
| Activity | first data | `statsLoading` → false | bones out 360 + content reveal 600; StatTiles count from 0; bars grow |
| Activity | range change | `range` | Segmented thumb slides; StatTiles count old → new; bars and Sparkline re-grow; ShareBar segments re-scale |
| Activity | 60s tick | `statsNow` | values/bars snap silently |
| Activity | `all` range | `trends === null` | no delta chips |
| Activity | delta 0 or no baseline | `:98-99,399` | no chip |
| Activity | error / empty | `:681,690` | danger Banner + Retry / EmptyState + Open chat |
| News | closed / open | `newsOpen` (persisted `:54-61`) | header only / body via carrier B |
| News | checking | `news.status === "checking"` | refresh icon → Spinner, button disabled |
| News | digest consent (no Claude) | `!claudeReady` | Summarize disabled, cost line "Connect Claude…" |
| News | digest loading / ok / error | `news.digestStatus` | Spinner + crossfading step + indeterminate bar / list / danger text |
| News | items loading / error / none | `NewsFeed:168-177` | list bones / danger Banner + Retry / EmptyState |
| News | maintenance release | `it.maintenance` | `--ink-faint` line |
| Onboarding | step 1-4 | `step` | §2.7 |
| Onboarding | demo playing / held / replay | mount, `run` | scene animates once, holds composed frame; Replay restarts |
| Onboarding | reduced motion | OS | composed static scene, no Replay button |
| Onboarding | step 2 soft-block | `warnSkipConnect` | warn Banner |
| Onboarding | Esc | `onEscape` | step 1: nothing; step >1: back one step (commit 18) |
| Onboarding | narrow <720 / <440 | media | rail hidden, mobile Skip / cards stack |
| Connect | probing / not found / not signed in / key form / connected / error | `ClaudeConnect:136-218` | §2.8 |
| Connect | copied / copy failed | `copied`, `error` | "Copied" accent 1.8s / danger error line |
| Connect | standalone (welcome) | `standalone` | no eyebrow, 18/24 title, `small` lede |
| LimitBar | ok / warn / hot | `zone` | accent / warn / danger fill |

## 4. Motion

Durations: f = `--dur-fast` 150, b = `--dur-base` (240 until shortlist #3, then 300), s = `--dur-slow` 600. eo = `--ease-out`, sp = `--ease-spring`. "net" = P2 reduced-motion net makes it instant.

| Element | Trigger | Property | From → to | Dur | Easing | Delay / stagger | Reduced motion |
|---|---|---|---|---|---|---|---|
| Home sections | first open | opacity, transform | 0 / +10px → 1 / 0 | 520 (`--dur-page`) | eo | 60 + 60·i, cap 6 (P2) | net |
| Project card | hover | `translate` | 0 → 0 −3px | f (kit Card) | eo | — | net (colour only) |
| Project card | hover | border-color | line-100 → line-200 | b | eo | — | net |
| Card actions | card hover / focus-within | opacity | .6 → 1 | f | eo | — | net |
| Continue | card hover / focus-within | opacity, transform | 0 / −4px → 1 / 0 | f | eo | — | net |
| Card grid | re-sort | transform (FLIP) | old → new slot | 460 | easeOut | — | duration 0 (`haloFlip`) |
| Buttons | press | `scale` | 1 → .97 | f | eo | — | none |
| Editor | open | opacity, transform | 0 / +10px → 1 / 0 | b | eo | — | net |
| Scope Collapse | toggle | grid-template-rows; content opacity + y | 0fr → 1fr; 0 / −4px → 1 / 0 | b | eo | content +60 | net |
| Chevrons | toggle | transform | 0 → 180° | b | eo | — | net |
| Recents popover | open / close | opacity, scale, y | .96 / −4px → 1 / 0; leave scale .97 | f / 140 | eo | — | net |
| Range Segmented thumb | pick | translate, width | old → new option | b | eo | — | net |
| StatTile value | first data, range change | number (CountUp) | 0 or old → new | 700 (`--dur-count`) | easeOutCubic | — | snaps (`motion.reduced`) |
| StatTile value | 60s tick | — | snap | 0 | — | — | — |
| DailyBars bars | mount, rangeKey | `transform: scaleY` origin bottom | 0 → v | s | eo | none (03 #122) | net |
| DailyBars | hover a bar | sibling opacity | 1 → .45 | b | eo | — | net |
| ShareBar segments | mount, rangeKey | `scale` x | 0 → 1 | s | eo | 60 per segment | net |
| ShareBar | hover segment | sibling opacity | 1 → .45 | b | eo | — | net |
| Sparkline | mount, rangeKey | stroke-dashoffset | 1 → 0 (`halo-draw`) | s | eo | — | drawn |
| Skeleton | loading ≥300ms → data | bones opacity in; out opacity + blur 4; content opacity + blur 6 + y 6 | — | 150 / 360 / 600 | eo | bones after 300 | instant swap |
| Banner (save current, errors) | mount | opacity, transform | 0 / +10px → 1 / 0 | b | eo | — | net |
| EmptyState | mount | opacity, transform | 0 / +10px → 1 / 0 | s | eo | — | net |
| News body | open / close | grid-template-rows + content opacity (carrier B) | 0fr ↔ 1fr | b | eo | content +60 in | `rm()` → 0 |
| News refresh | checking | Spinner rotate | 0 → 360° ∞ | 900 | linear | — | **keeps turning** (essential) |
| Digest loading | loading | indeterminate bar translateX | −100% → 250% ∞ | 1.3s | eo | — | static 40% bar |
| Digest step label | 2.6s interval | opacity | 0 → 1 | f | eo | — | net |
| More notes / releases | toggle | Collapse | 0fr ↔ 1fr | b | eo | +60 | net |
| Onboarding card | first show | opacity, transform | 0 / +10px → 1 / 0 | s | eo | — | net |
| Onboarding step | `{#key step}` | `.page-sections` cascade | 0 / +10px → 1 / 0 | 520 | eo | 60 + 60·i, cap 6 | net |
| Stepper fill | step | `transform: scaleY` origin top | p → p' | s | eo | — | net |
| Stepper mark (done) | step advances past it | scale pop 1 → 1.18 → 1; check stroke-dashoffset 1 → 0 | b; b | sp; eo | check +120 | net (check drawn) |
| Stepper mark (current) | step | bg, border, colour, ring | — | b | eo | — | net |
| Demo scenes | step open, Replay | per scene keyframes (opacity, transform, `clip-path`, scaleX) | composed final frame held | 5.5 / 6.5 / 7 / 5 s (today's timelines, `motion-literal: onboarding demo`) | eo | today's % windows | composed static scene |
| RadioGroup card / dot | pick | border, bg / dot `scale` 0 → 1 | f / b | eo / sp | — | net |
| Model Segmented thumb | pick | translate, width | — | b | eo | — | net |
| Connected Banner | connected | opacity, transform | 0 / +10px → 1 / 0 | b | eo | — | net |
| LimitBar fill | value change | `transform: scaleX` | p → p' | 700 | eo | — | net |

Idle budget: Home 2s after load → `document.getAnimations()` running = 0; onboarding step 8s after open → 0 (03 §4.7).

## 5. Parts

### 5.1 Kit primitives used (P3, no changes to them in 4d)

PageHeader (K28), SectionLabel (K17), Card (K12), Button + IconButton (K10-K11), Badge (K13), Banner (K19), EmptyState (K18), Skeleton (K20), Segmented (K21), Field/Input/Textarea (K23), Popover (K09), Collapse (K30), RadioGroup `cards` (K22d), StatTile (K25), ShareBar (K26), Progress (K27), Spinner (K15), Separator (K16), `shared/Select`, motion helpers `haloFlip` (K04d), `countUp` (K04c, inside StatTile), P2 `transitions.ts` `accordion`, `.page-sections`, `.rise-page`, `.chev`.

If a kit part's API differs from spec 02 when 4d starts, the kit wins and this spec's call sites adapt; never fork a parallel part.

### 5.2 Rift parts built in 4d

**`workspace/DailyBars.svelte`** (D-24 "Rift dense bars"; spec 02 §5.3 places it here)

```ts
type Props = {
  cells: Array<{ key: number; value: number; label: string }>; // oldest → newest
  max: number;
  rangeKey: unknown;               // change → bars re-grow from 0
  format: (n: number) => string;   // value text
  ariaLabel: string;               // e.g. "Messages per day, last 30 days"
  height?: number;                 // plot px, default 104 (board lever)
};
```

- Markup: `<figure aria-label={ariaLabel}>` → `<div class="plot" aria-hidden="true">` with one `<span class="bar" data-peak data-zero>` per cell inside `{#key rangeKey}`; then `<ul class="sr-only">` with one `<li>{label}: {format(value)}</li>` per non-zero cell plus "{n} days with no activity". The list is the accessible value path (bars were `aria-hidden` with dead tooltips, `WorkspacePage.svelte:712-713`).
- CSS: plot `display: grid; grid-template-columns: repeat(var(--cols), minmax(0, 1fr)); gap: 2px; align-items: end; height: var(--plot-h)`; no border, no inset well (box-in-card, map 05 §8 #8). Bar `height: 100%; transform-origin: bottom; transform: scaleY(var(--v)); background: var(--bg-300); border-radius: 2px 2px 0 0 /* data-mark radius, §13 A-4 */`; `[data-peak] { background: var(--accent) }`; `[data-zero] { --v: .02; background: var(--line-100) }`; non-zero minimum `--v` .06 (today's 6% `:711`). Mount/rangeKey: `animation: db-grow var(--dur-slow) var(--ease-out) both` with `@keyframes db-grow { from { transform: scaleY(0) } }` — the only new keyframe; it reads the element's own `--v` end state. Value changes outside a key change (60s tick) set `--v` with no transition (snap).
- Hover: `.plot:has(.bar:hover) .bar:not(:hover) { opacity: .45 }`, `transition: opacity var(--dur-base) var(--ease-out)`. Each bar `use:tooltip={`${label} · ${format(value)}`}` (the bar carries pointer events; `aria-hidden` parent keeps it out of the a11y tree).
- Peak: every cell with `value === max && max > 0`.
- Home call: `cells={series.cells.map(c => ({ key: c.day, value: c.messages, label: `${dayLabel(c.ms)} · ${fmtInt(c.messages)} msg · ${c.sessions} session${c.sessions === 1 ? "" : "s"}` }))}` (today's tooltip text `:712`), `max={series.max}`, `rangeKey={range}`, `format={fmtInt}`.

**`shared/Sparkline.svelte`** (D-24 "Spend: a Halo Sparkline drawn in"; Halo `components/Sparkline/README.md`; shared with 4f)

```ts
type Props = { values: number[]; width?: number /* 96 */; height?: number /* 36 */; tone?: "muted" | "accent" | "chart-2" | "chart-3" /* "muted" */; rangeKey?: unknown; ariaHidden?: boolean /* true */ };
```

- One `<svg viewBox="0 0 {width} {height}">` `<path pathLength="1">` smoothed (monotone cubic through points scaled to 2px inset), stroke 2, round caps, no fill, `stroke: var(--ink-muted)` for `muted` (default: spend rising is not "good", and accent stays scarce; §14 Q-5), else the named token. Draw: `stroke-dasharray: 1; stroke-dashoffset: 1; animation: halo-draw var(--dur-slow) var(--ease-out) forwards` inside `{#key rangeKey}`. All values 0 → flat line at mid height, no animation. Fewer than 2 points → render nothing.
- Decorative on Home (the tile states the value), so `aria-hidden="true"`.

**`onboarding/Stepper.svelte`** (Halo Stepper `bundle.css:535-558`, vertical; spec 02 has no Stepper, §13 A-2)

```ts
type Props = {
  steps: Array<{ label: string; description?: string }>;
  current: number;                       // 0-based
  onStepClick?: (index: number) => void; // only finished steps are buttons
  ariaLabel?: string;                    // default "Setup progress"
};
```

- Markup: `<nav aria-label>` → `<ol>` with a track span (`aria-hidden`) and one `<li data-state="done|current|upcoming">` per step. Done steps render `<button type="button" onclick={() => onStepClick?.(i)}>`; others a `<div>`. Current carries `aria-current="step"`; sr text " (completed)" / " (current step)".
- CSS (scoped, attribute selectors): list `flex-direction: column; gap: 24px; position: relative`; track `position: absolute; left: 15px; top: 16px; bottom: 16px; width: 2px; border-radius: 2px; background: var(--bg-300)`; fill `inset: 0; background: var(--accent); transform-origin: top; transform: scaleY(var(--progress)); transition: transform var(--dur-slow) var(--ease-out)` with `--progress = current / (steps.length − 1)` (replaces `OnboardingFlow.svelte:180` inline height); mark 32×32 circle, bg-300, 1px line-300, mono 600 13/1 `--ink-muted`; current: bg-100, border accent, colour accent, `box-shadow: 0 0 0 4px var(--accent-soft)`; done: bg accent, border accent, check svg 16 `stroke: var(--accent-ink)` 2.6 drawn (`stroke-dashoffset` 1 → 0, b, +120ms); pop `halo-pop` b sp only on the transition to done (set `data-touched` after the first change so a mount on step 3 does not pop marks 1-2); text column padding-top 6, label 13/18 500 (`--ink-muted` upcoming, `--ink` current/done), description 12/16 `--ink-faint`; done button hover bg-200 r8, focus `--shadow-focus`.
- Call: `<Stepper steps={steps.map(s => ({ label: s.t, description: s.s }))} current={step - 1} onStepClick={(i) => goto(i + 1)} />` — `goto` `:66-71` unchanged (backward only).

**`ObStage.svelte` API change**

```ts
let { kind, caption, modelLabel, presetLabel }: { kind: Kind; caption?: string; modelLabel?: string; presetLabel?: string } = $props();
let run = $state(0);
```

- Structure: `<div class="ob-stage">` (no `aria-hidden`) → `{#key run}<div class="ob-mini …" aria-hidden="true">…</div>{/key}` → caption (`aria-hidden`) → `{#if !prefersReducedMotion()}<Button variant="ghost" size="sm" class="ob-replay" onclick={() => run++} aria-label="Replay the animation">`RotateCcw 16 "Replay"`</Button>{/if}`. The key on `run` is an intended replay (§7).
- `defaults` scene: `<ShieldCheck size={12} /> {presetLabel ?? "Your style"}` replaces the hard-coded "Balanced" (`:63`). OnboardingFlow passes `presetLabel={PRESETS.find((p) => p.id === activePreset)?.label}`.

**One-shot demo keyframes** (onboarding.css; fixes the blank-final-frame trap, §13 S-1). Rule: every scene's last keyframe equals its base (no-animation) style, so the held frame and the reduced-motion frame are the same picture.

| Today | 4d |
|---|---|
| `ob-loop-out` `:510` (ends opacity 0) on `.ob-mini-chat/-cli/-tree` `:482,492,497` | deleted; the three rules lose it |
| `ob-mc-dot` `:514` (1.4s loop) | deleted; chip dots static `--accent` |
| `ob-dial` `:528` (ends .22, box-shadow) | `@keyframes ob-dial { 0%, 8% { opacity: .22 } 16%, 100% { opacity: 1 } }`; delays `:505-507` kept |
| `ob-mcl-type` `:519` (`max-width`, layout) | `@keyframes ob-mcl-type { 0%, 6% { clip-path: inset(0 100% 0 0) } 26%, 100% { clip-path: inset(0 0 0 0) } }`, `steps(6, end)` kept; `.cmd` gets a fixed `width: 6ch` |
| `ob-mcl-caret` + `ob-blink` `:520-521` | deleted; caret solid (no blink loop, D-13) |
| `ob-tree-scan` `:527` (`top`) | `.scan { inset: 0; height: auto; background: linear-gradient(180deg, transparent 40%, var(--accent-soft) 50%, transparent 60%) }`; `@keyframes ob-tree-scan { 0%, 48% { transform: translateY(-60%); opacity: 0 } 54% { opacity: 1 } 74% { transform: translateY(60%); opacity: 1 } 80%, 100% { transform: translateY(60%); opacity: 0 } }` |
| every `infinite` in `:482-507` (24 declarations) | iteration count 1, `animation-fill-mode: both`; the `prefers-reduced-motion: no-preference` wrapper `:480` stays |
| `ob-mc-*`, `ob-tree-row*`, `ob-mcl-pill`, `ob-mc-l1..3` | kept as written (they already end on the composed state); `var(--ease-soft, ease-out)` `:488-490` → `var(--ease-out)` |

**`LimitBar.svelte`** — adapter, §2.6. Props unchanged (`pct`, `zone`, `variant`, `live`, `class`); `data-zone` kept on the root.

**`usageStats.ts` formatters** (after `fmtCost` `:243-248`)

```ts
/** "12d" — streak lengths. */
export function fmtDays(n: number): string { return `${Math.max(0, Math.round(n))}d`; }
/** "+12%" / "−8%" (U+2212) / "0%" — delta chips; caller hides 0. */
export function fmtSignedPct(n: number): string {
  const r = Math.round(n);
  return r > 0 ? `+${r}%` : r < 0 ? `−${Math.abs(r)}%` : "0%";
}
/** Share 0..1 → "61%", "<1%" for (0, .005), "0%" for 0. */
export function fmtShare(s: number): string {
  if (s <= 0) return "0%";
  if (s < 0.005) return "<1%";
  return `${Math.round(s * 100)}%`;
}
```

**ShareBar adoption** (Home): `items={perModel(stats).filter((m) => m.messages > 0).map((m) => ({ label: m.label, value: m.messages }))}`, `top={3}`, `format={(n) => fmtShare(n / totals.messages)}`, `exact={(n) => `${fmtInt(n)} messages`}`, `ariaLabel="Model usage share"`, `rangeKey={range}`. ShareBar folds everything past the top 3 into "Other" itself, so `SEG_HUES`, `segs`, `mix` (`:115-131`) and the slice-to-5 in `models` (`:108`) are deleted. They are presentation, not tested logic; map 05 §6 lists the whole script as logic, and this is a deliberate exception.

## 6. Ordered commit list

Branch `halo/p4d-home-onboarding` off `design/halo` after 4a-4c merge (D-27). Every commit: `npm run check` + `npm test` green before it is made. "Verify" is the cheapest proof for that commit; the full §10 pass runs at the end. Subjects use `style(home): …`, `style(onboarding): …`, `feat(home): …`. Order: connect cards first (the welcome shifts once, after 4b), then onboarding, then the shared LimitBar, then Home.

**Before commit 01 (no commit):** `c.sh baseline p4d-home ""` on Home with news collapsed; `c.sh baseline p4d-home-news ""` with news open; `c.sh nav ai-health` + `c.sh baseline p4d-health ""`; onboarding shown (§10 step 6) + `c.sh baseline p4d-ob1 ".ob-card"` … `p4d-ob4`. Record `.home` scroll/client heights.

### 6.1 Session A — connect cards + onboarding

| # | Title | Files (path:line) | Exact change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| 01 | `style(onboarding): ClaudeConnect on kit parts` | `onboarding/ClaudeConnect.svelte:130-226`, `:91,101` | §2.8: `.ob-head` gets `data-standalone={standalone || undefined}`; eyebrow → SectionLabel (non-standalone); statcard → `.ob-statlist` rows + Badge values; checking row Spinner; copy buttons → `Button ghost sm` (Copy/Check), timer 1600 → 1800; Sign in → `Button primary loading`; hints restyle only; API key → `Field` + `Input type=password` + `Button secondary loading`; connbar → `Banner accent`; error line; Re-check → `Button secondary sm loading={probing}` | `probe/poll/copyCmd/signIn/saveKey`, `onConnectedChange`, `standalone`, 4s poll `:72-77`, all copy text except label case | welcome needs-auth screen restyles in the same commit (intended) | `c.sh shot-sel '.ob-card' jpeg 70` on step 2; welcome: see §10 step 9 |
| 02 | `style(onboarding): OpenAiConnect on kit parts` | `onboarding/OpenAiConnect.svelte:33-78` | same vocabulary: statlist + Badge, `Banner accent` connected, `Banner warn` install, route note row, Sign in `Button primary` (disabled while `codexChecking`), Re-check `Button secondary sm loading={checking}` | `refresh`, `onConnectedChange`, `standalone` | — | shot step 2 with ChatGPT selected |
| 03 | `style(onboarding): connect-card CSS on tokens` | `styles/onboarding.css:117-131,201-206,263-345` | rewrite `.ob-head/.ob-eyebrow/.ob-title/.ob-sub/.ob-link/.ob-input-row/.ob-field/.ob-flabel/.ob-hint/.ob-route-note/.ob-error/.ob-statlist/.ob-copy-*` per §2.8 on tokens; standalone variants via `.auth-connect .ob-title` etc. are NOT added (the component passes `data-standalone` on its head and CSS keys off `[data-standalone]`); move `.ob-btn*` `:133-148`, `.ob-input*` `:284-291`, `.ob-connbar` + `ob-live` `:321-326`, `.ob-hint--warn` `:309-310`, `.ob-statcard` `:332` to a final `/* ── RETIRED in P4d — delete in P5 (shortlist #6) ── */` block, unchanged | class names used by the 2 components | a rule still referenced is retired: Grep each retired class across `src/` = 0 before moving | Grep; shot step 2 |
| 04 | `style(onboarding): flat frame — canvas, card, rail, footer` | `styles/onboarding.css:10-32,35-53,87-103,150-161`; `OnboardingFlow.svelte:169-177` | §2.7 frame rows: overlay bg-000, card bg-100/line-100/r16/no shadow, `.rise-page` replaces `ob-card-in`; rail flat; brand → RiftLogo 28 + heading + `Badge`; footer opaque 64; `--gi-ghost-*` aliases deleted once their last user (`:78,80,291,470`) is migrated in 06/11/15 (leave until then) | `.ob-host` untouched (AppShell) | darker first impression (canvas vs today's radial) | `c.sh shot-sel '.ob-card'` vs `p4d-ob1` |
| 05 | `feat(onboarding): Halo vertical Stepper` | new `onboarding/Stepper.svelte`; `routes/dev/ui/+page.svelte` exhibit (4 steps, current 1) | §5.2 Stepper | — | `check:ui` rule 1 does not apply (not under `ui/`), but keep attribute selectors anyway | `npm run check`; dev/ui look |
| 06 | `style(onboarding): rail uses Stepper; one progress signal` | `OnboardingFlow.svelte:179-197,368-373`; `onboarding.css:55-85,157-160` | rail `.ob-steps` block → `<Stepper>`; footer dots deleted; `.ob-steps/.ob-spine/.ob-step*/.ob-node/.ob-fdot/.ob-foot-dots` → RETIRED block | `goto` backward-only | narrow <720 loses footer dots: "1 of 4" in the SectionLabel (07) replaces them, so land 06 and 07 back to back | shot rail; `c.sh measure '.ob-rail'` |
| 07 | `style(onboarding): step head + page-sections cascade` | `OnboardingFlow.svelte:208-215,270-274,310-314`; `onboarding.css:104-121` | `.ob-step-wrap` gets `page-sections`, max 640, gap 24, padding 48 0 40; steps 1/3/4 heads → SectionLabel + `h1#ob-step-title` + lede (§2.7); delete `ob-rise` `:109-115` | copy text | `{#key step}` replay is intended (§7) | `c.sh click` Next; `getAnimations().length` ≤ 7 during, 0 after 1s |
| 08 | `style(onboarding): step 1 value rows` | `OnboardingFlow.svelte:216-231`; `onboarding.css:163-175` | hairline rows, 32 chip, `small` text (§2.7) | copy | — | shot step 1 |
| 09 | `style(onboarding): accent strip on tokens (shortlist #5 pending)` | `OnboardingFlow.svelte:232-249`; `onboarding.css:177-198` | label `small`; swatches 24, ring per §2.7, `oklch(0 0 0/.82)` → `var(--bg-000)`, `oklch(1 0 0/.16)` → `color-mix(in srgb, var(--ink) 16%, transparent)`; hover lift removed | P1 `isAccent/pickAccent` | — | shot step 1; click blue + one other swatch |
| 09g | **gated #5** `feat(onboarding): drop the accent strip` | `OnboardingFlow.svelte:232-249` (+ `ACCENTS`, `uiPrefs` imports `:12`); `onboarding.css:177-198` → RETIRED | delete the strip; step 1 ends after the value rows | dial code in ui-prefs untouched (4e owns it) | owner-visible | shot step 1 |
| 10 | `style(onboarding): provider as RadioGroup cards + warn Banner` | `OnboardingFlow.svelte:252-259,265-267`; `onboarding.css:208-236` | §2.7 provider + soft-block rows; `.ob-seg*`/`.ob-provider*` → RETIRED (after 12 also drops the model seg) | `selectProvider` side effects, `warnSkipConnect` | arrow keys now move selection (bits-ui) where `:252-258` had none: improvement, note in body | keyboard: Tab to group, ArrowRight → ChatGPT selected |
| 11 | `style(onboarding): step 3 folder status + recents` | `OnboardingFlow.svelte:275-307`; `onboarding.css:293-304` | §2.7 step 3 rows | `chooseFolder`, `assistant.setRoot(r)`, `displayPath` | — | shot step 3 (with and without a folder) |
| 12 | `style(onboarding): step 4 model + presets` | `OnboardingFlow.svelte:315-357`; `onboarding.css:238-261` | Segmented (≤4) / `shared/Select` (>4) for model; RadioGroup cards for presets; Badge "Recommended"; `.ob-preset*`, `.ob-seg-ver` → RETIRED | `pickModel`, `pickPreset` fan-out `:157-164`, `activePreset` | Select inside a takeover: z 65 > card, fine | shot step 4 with Claude and with ChatGPT |
| 13 | `style(onboarding): footer buttons on kit` | `OnboardingFlow.svelte:365-389`; `onboarding.css:88-95,151-161` | `Button ghost` Back, `Button primary` Next/Start working, `Button ghost sm` Skip (rail + mobile) | `next/back/onDone` | — | shot footer |
| 14 | `style(onboarding): ObStage well + scene colours` | `onboarding.css:346-359,364-478` | §2.9 well and scene rows; `--gi-ghost-*` now unused → delete the 3 alias lines `:12-14` | scene markup | — | shot each stage, reduced motion on (static composed) |
| 15 | `fix(onboarding): demos play once, hold the composed frame, Replay` | `onboarding.css:480-528`; `ObStage.svelte:14-67` | §5.2 one-shot table + Replay + aria split | scene content | if P2 M26 already landed differently, this commit supersedes it; diff must leave 0 `infinite` in the file | 8s after each step opens: `getAnimations().length === 0` and `getComputedStyle(document.querySelector('.ob-mini')).opacity === "1"` |
| 16 | `fix(onboarding): defaults scene shows the chosen preset` | `ObStage.svelte:11,63`; `OnboardingFlow.svelte:309` | `presetLabel` prop (§5.2) | — | — | pick Fast on step 4 → stage pill reads "Fast" |
| 17 | `a11y(onboarding): takeover is a page, not a modal (A-1)` | `OnboardingFlow.svelte:169,205,213,272,312`; `design-system/ui-baseline.json` | drop `role="dialog" aria-modal="true"`; keep `aria-label` on `.ob-overlay` as `<section>`; `<main aria-labelledby="ob-step-title">`; `check:ui -- --update` (`overlay:aria-role` 17 → 16) | focus order unchanged | if the owner keeps D-30's trap instead, replace with 17-alt (§13 A-1) | `npm run check:ui`; `c.sh ax` shows no dialog role |
| 18 | `fix(onboarding): Esc on step 1 no longer ends setup (D-30)` | `OnboardingFlow.svelte:99-107` | `if (step === 1) return;` replaces `onDone()`; comment updated; `preventDefault` only when acting | Esc on steps 2-4 goes back | behaviour change: release note | `c.sh key Escape` on step 1 → `.ob-host` still present |

### 6.2 Session B — LimitBar + Home

| # | Title | Files (path:line) | Exact change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| 19 | `style(shared): LimitBar renders ui/progress` | `shared/LimitBar.svelte:1-82` | §2.6 adapter; style block reduced to the compact width rules | props + `data-zone`; callers untouched | restyles StatusBar (4a), UsagePanel (4c), AiHealth (4f) after their ports: card track 7 → 8px | `c.sh diff` vs `p4d-health`; shot status bar + usage panel open |
| 20 | `feat(stats): fmtDays, fmtSignedPct, fmtShare` | `utils/usageStats.ts` after `:248`; `utils/usageStats.test.ts` | §5.2 code; tests: `fmtDays(12.4)` "12d", `fmtSignedPct(12)` "+12%", `(-8)` "−8%" (U+2212), `(0)` "0%", `fmtShare(0)` "0%", `(0.004)` "<1%", `(0.614)` "61%" | existing exports | — | `npm test -- usageStats` |
| 21 | `feat(home): DailyBars` | new `workspace/DailyBars.svelte`; dev/ui exhibit (14/30/60 synthetic cells) | §5.2 | — | 60 bars × tooltips: action is cheap (singleton) | dev/ui look + hover |
| 22 | `feat(shared): Sparkline` | new `shared/Sparkline.svelte`; dev/ui exhibit | §5.2 | — | — | dev/ui look |
| 23 | `style(home): one column + PageHeader` | `WorkspacePage.svelte:408-432,799-801`, style `:804-830` | root `div.home` (`container: home / inline-size; --page-col: 1080px; height: 100%`) → `.home-scroll` (overflow-y auto) → `<PageHeader maxWidth={1080} eyebrow={{label:"Home"}} title=…>` with `actions` (Switch folder) and `children` (meta row) → `div.home-col.page-sections` (keeps P2's class; the 3 sections become its direct children); delete `.sb-main/.sb-scroll/.sb-wrap/.head*/.greet-line/.band-row/.cue/.chip-btn/.dash/.col`; `bind:this={scrollEl}` moves to `.home-scroll` (`:46-49` scroll-to-top kept) | `paneRoot/hubBranch/hubFileCount`, `pickTabFolder(null)` | header inside the scroller, never sticky (scrollbar shift) | measure `.home-scroll` clientHeight at audit viewport → write into §2.1; `c.sh measure` header inner left == `.home-col` content left ±0.5 |
| 24 | `style(home): numbered section heads` | `WorkspacePage.svelte:559-566,657-664,775-791`; style `:823-844,974-983` | `.sec-head` + SectionLabel 01/02/03; "New project" `Button ghost sm`; range → `Segmented` bound to `range` | `startNew()`, `range` | Segmented emits on keyboard arrows (selection follows focus): 3 quick key presses fire 3 range changes, each a count-up: acceptable, it is the user's input | keyboard on range: values update, thumb slides |
| 25 | `style(home): project cards on Card + neutral avatars` | `WorkspacePage.svelte:24,585-631`, style `:1087-1158` | §2.2 card rows; `{#each}` gets `animate:haloFlip`; `projectHue` import removed; `.live-dot` + `livePulse` remnants gone | `openProject/goHome/openInSplit/startEdit`, `sortedProjects`, `pulseOf`, `monogram`, `scopeLabel`, aria-labels `:595,621,625` | 4-up → 3-up at fullscreen (D-31 1080 column) | hover card: `getComputedStyle(card).translate` "0px -3px"; focus via Tab shows ring on the card edge |
| 26 | `style(home): save-current Banner, projects empty + error` | `WorkspacePage.svelte:568-584,640-649`, style `:1079-1085,1160-1176` | §2.2 rows | `adoptActive`, `projects.refresh` | — | three states via §10 step 4 eval toggles |
| 27 | `style(home): project editor on kit fields` | `WorkspacePage.svelte:434-546` → moved inside the Projects section after its head; style `:985-1077` | §2.3; `.rift-input` local class deleted (the global-sounding scoped class, map 05 §5) | every handler `:437-543`, focus effect `:194-196`, `recentOpen`, `advOpen/advForced` | Popover is portalled: Escape inside it must close the popover, not the editor — bits-ui stops propagation; the editor's Escape handler `:438` still fires when focus is in a field | open editor, Esc in Name closes; open recents, Esc closes recents only |
| 28 | `style(home): Activity card + StatTiles` | `WorkspacePage.svelte:693-739`, snippet `:395-406`, style `:846-865,905-942` | §2.4 card frame + 5 StatTiles + spend link button; delete `trendChip`, hero `.hn-v` gradient numeral, `.strip/.st/.delta` | `totals`, `trends`, `strk`, `peak`, `windowDays`, `workspace.setActive("ai-health")` | Geist widths: "$12.4k" + chip in 196px: verify no wrap | range click: first sample 100ms after < final; 900ms later == final |
| 29 | `style(home): daily chart → DailyBars` | `WorkspacePage.svelte:703-716`, style `:913-922` | §5.2 Home call; caption + peak line | `series`, `dayLabel` | — | hover a bar: siblings .45; sr list count == non-zero days |
| 30 | `style(home): model mix → ShareBar` | `WorkspacePage.svelte:108-131,740-762`, style `:944-956` | §5.2 ShareBar adoption; delete `models`, `topMdl`, `SEG_HUES`, `segs`, `mix`, `statSummary` (`:108-109,112,115-131`) and the now-unused imports `topModel`, `summaryLine`; fun fact restyle | `perModel`, `funFact` | — | legend shows ≤ 4 rows; "Other" present when > 3 models |
| 31 | `feat(home): spend sparkline` (droppable, board 05 `data-prop-spark`) | `WorkspacePage.svelte` tile 4 children | `<Sparkline values={series.cells.map(c => c.cost)} rangeKey={range} />` | — | — | tile shows a line; reduced motion: drawn, no animation |
| 32 | `style(home): Activity loading, empty, error` | `WorkspacePage.svelte:666-692`, style `:866-897,958-960` | §2.4 states | `loadStats`, `statsEmpty` | Skeleton waits 300ms: fast loads never flash | §10 step 4 |
| 33 | `style(home): AI updates as an accordion card` | `WorkspacePage.svelte:769-797`, style `:1178-1203` | §2.5 header + `{#if newsOpen}<div transition:accordion>`; refresh → IconButton/Spinner | `toggleNews`, `rift.workspace.newsOpen`, `news.maybeFetch(true)`, `aria-controls="ws-news-body"` | — | open/close: one 300ms move; refresh shows Spinner |
| 34 | `style(news): NewsFeed on kit, no amber wash` | `workspace/NewsFeed.svelte:90-337` | §2.5 body rows | script `:1-88` untouched (`openExternal` https guard, `expanded`, previews, step effects) | — | open news: shot vs `p4d-home-news`; Summarize disabled when Claude not ready |
| 35 | `docs(shared): PageHero is deprecated` | `shared/PageHero.svelte:1` | `<!-- @deprecated: use ui/page-header (PageHeader). Settings, AI Health, Diagnostics swap in 4e/4f; delete under shortlist #6. -->` | — | — | none (comment) |
| 36 | `chore(ui): lock P4d ratchet counts` | `design-system/ui-baseline.json` | `npm run check:ui -- --update` after diffing the JSON: every key ≤ before | — | never commit a risen count | `npm run check:ui` |

Conditional (only if §2.1's measured height < 823): `style(home): fit lever N`, one per lever in the §2.1 order.

**Totals:** 36 default commits (01-36) + 1 gated (09g). Session A 01-18, Session B 19-36.

Import changes in `WorkspacePage.svelte:4-29` across 23-33: add `hourLabel`, `fmtDays`, `fmtSignedPct`, `fmtShare` (usageStats), kit parts, `DailyBars`, `Sparkline`, `ShareBar`, `haloFlip`, `accordion`; remove `projectHue`, `topModel`, `summaryLine` and the lucide icons no longer rendered (`Sparkles`, `History` stays for the editor, `ActivityIcon`, `Loader2`, `Flame`, `Cpu`, `Wrench`, `TrendingUp`, `TrendingDown`, `AlertTriangle`, `RotateCw`, `Newspaper` stays). svelte-check flags any leftover.

## 7. Contracts + couplings touched (D-33)

| Contract / coupling | Where | 4d handling |
|---|---|---|
| `.ob-host` (CDP onboarding probe) | `AppShell.svelte:307,504`; `scripts/cdp/c.sh:548` | untouched |
| Connect-card props `onConnectedChange`, `standalone` | `ClaudeConnect.svelte:49-50`, `OpenAiConnect.svelte:7-8`; consumers `OnboardingFlow.svelte:261,263`, `AssistantWelcome.svelte:158` | unchanged; `standalone` now also sets `data-standalone` on the head |
| Global `onboarding.css` side-effect import | `OnboardingFlow:7`, `ClaudeConnect:14`, `OpenAiConnect:5` | kept global (scoping would strip the welcome); retired rules parked at file end |
| `.auth-connect` 440 column | `AssistantWelcome.svelte:710-715` (4b) | not touched; 4d's standalone sizes (18/24 title, `small` lede) are chosen to fit it |
| LimitBar API `pct/zone/variant/live/class` + `data-zone` | 6 call sites (§1.2) | unchanged; `live` ignored (flagged §8) |
| `rift.workspace.newsOpen`, `rift.onboarding.dismissed`, `rift.betaNotice.ack` | `WorkspacePage:54`, `onboarding.svelte.ts:6`, `betaNotice.svelte.ts:7` | unchanged |
| `aria-controls` ids `ws-news-body`, `ed-adv` | `WorkspacePage:777,502` | kept on the new bodies |
| `usageStats` exports (tested, `usageStats.test.ts`) | `utils/usageStats.ts` | additions only; `summaryLine`, `topModel` lose their only consumer (flag §8) |
| `.ws-page[data-workspace="home"]`, `.page-sections` direct-children rule | `WorkspaceShell.svelte:84-91`; P2 motion.css | the three sections are the direct children; the editor moved inside Projects so it never inherits the page entrance |
| Intended replays (no `settleGuard`, spec 03 §5.2 exemption list) | `{#key step}` `OnboardingFlow:207`; `{#key run}` ObStage; `{#key rangeKey}` DailyBars + Sparkline; `{#key stepIdx}` digest label | add these four to spec 03's exemption list |
| Geometry coupling: `--pcard-h` | grid `grid-auto-rows` and `.proj-scroll` max-height | one var feeds both |
| Geometry coupling: Stepper track inset = mark radius | Stepper scoped CSS | one `--mark: 32px` var feeds mark size and track `left: calc(var(--mark) / 2 - 1px)`, `top/bottom: calc(var(--mark) / 2)` |
| Geometry coupling: DailyBars plot height | `height` prop → `--plot-h` | board lever writes the default prop value only |
| D-33 listed couplings (`ROW_H`, `thumbW`, mic `rx`, `composerH`, `measureInk`, `--chat-col`) | — | none touched |
| `projectHue` | ConversationList, ProjectSwitcher, AssistantPane (4a/4b) | Home stops importing it; util and test untouched (D-24) |
| `c.sh nav home` label | `c.sh:481,514` | owned by 4a's rename; 4d uses `c.sh nav home` either way |

## 8. Deletions (flagged)

Not gated (restyle in place inside the file being ported, same commit): WorkspacePage scoped rules and markup replaced in 23-34 (incl. its local `.rift-input`, `wsActSpin`/`nspin`/`skel-shimmer` remnants, `trendChip`), NewsFeed scoped rules, LimitBar's three fill recipes, the ObStage loop keyframes listed in §5.2, OnboardingFlow footer dots, the presentation consts `SEG_HUES/segs/mix/models/topMdl/statSummary` (`WorkspacePage.svelte:108-131`).

Gated by shortlist #6 (left in place, listed here):

| # | Item | Why it is dead after 4d | Precondition |
|---|---|---|---|
| 1 | `onboarding.css` RETIRED block (`.ob-btn*`, `.ob-input*`, `.ob-seg*`, `.ob-provider*`, `.ob-preset*`, `.ob-seg-ver`, `.ob-connbar` + `ob-live`, `.ob-hint--warn`, `.ob-statcard`, `.ob-steps/.ob-spine/.ob-step*/.ob-node`, `.ob-fdot/.ob-foot-dots`, `.ob-accent*` if 09g) | no markup uses them (Grep 0 checked before each move) | P5 |
| 2 | `.branch-pill` `src/app.css:682-…` | only consumer `WorkspacePage.svelte:423` becomes a Badge | Grep `branch-pill` in `src/` = 0 |
| 3 | `shared/PageHero.svelte` | deprecated in 35 | 4e/4f swap their 4 call sites |
| 4 | `shared/LimitBar.svelte` and its `live` prop | thin adapter | 4c/4f callers use `ui/progress` directly |
| 5 | `summaryLine`, `topModel` in `usageStats.ts` (+ their tests) | no consumer after 30 | owner yes; or keep as tested utilities |
| 6 | `.ob-accent*` swatch strip + `ACCENTS[1..]` | only with 09g | shortlist #5 = yes |

## 9. Canvas board 05 — Home + onboarding

- **Board id:** `05` (D-25), Claude Design project "Rift · Halo", Halo installed (Electric blue), plain markup on Halo tokens, played in Play mode. No dev server.
- **Frames** (all 1638 × 883 = the audit viewport in CSS px, so specimens read at real size):
  - **05a Home · data.** Real-size Home in the 1080 column inside the flat main island (board 01 frame A): header (title "rift-tauri", branch Badge "design/halo", "4,812 files", Switch folder), 6 project cards (one active), Activity card with 5 StatTiles (12.4k · 86 ↑+12% · 1.9k ↓−8% · $8.2k ↑+4% · 12d best 37d), 30 daily bars, ShareBar (Sonnet 5 61% · Opus 5.5 27% · Haiku 4.5 9% · Other 3%), fun fact, AI updates closed. A dashed 883 fold line marks the fit.
  - **05b Home · states.** 2 × 3 grid at 50%: loading bones; activity empty; activity error; 0 projects (EmptyState); folder open but not a project (save Banner); editor open with invalid glob + recents popover.
  - **05c Home · news open.** Accordion open: coverage line, digest CTA (warn cost line), one release with "6 more notes", "Show 11 more releases"; second variant with digest loading.
  - **05d Onboarding.** The full takeover at real size with a step switcher (1-4) above the frame; ChatGPT/Claude toggle on step 2.
  - **05e Connect cards.** Onboarding step 2 (Claude: not found; not signed in + key form; connected) beside the chat welcome needs-auth column (440) showing the same card standalone.
- **Play-mode interactions:** range Segmented → StatTiles count-up 700ms, bars re-grow 600ms, ShareBar re-scales, sparkline redraws; hover a bar → siblings .45 + tooltip after 400ms; hover a card → lift + Continue reveal; New project → editor opens (300ms rise), File scope toggles; AI updates toggles (300ms); onboarding Next/Back → step cascade + Stepper fill + done-mark pop; Replay → demo restarts; provider/preset/model picks; "Copy" → "Copied" 1.8s.
- **`data-prop-*` levers** on the board root (spec 03 §14 Q7 mechanism): `data-prop-density="compact|regular|comfy"` · `data-prop-projects="0|1|6|12"` · `data-prop-lift="0|2|3"` · `data-prop-overflow="in-grid|page"` · `data-prop-stats="loading|empty|error|data"` · `data-prop-plot="88|104|120"` · `data-prop-series2="violet|ink-muted"` · `data-prop-spark="on|off"` · `data-prop-summary="tiles|sentence"` (today's summary sentence vs K5 tile details) · `data-prop-demo="once|static|loop"` · `data-prop-title="24|28"` (onboarding title) · `data-prop-accent-strip="on|off"` · `data-prop-tone="neutral|accent"` (range thumb).
- **Owner decides here:**
  1. Violet #b79cff as series 2 of the model mix (D-24 flags it against §8 "no purple defaults") or `ink-muted`.
  2. Card lift 3px (Halo, D-16) or 2px (map 12 X4 felt test).
  3. More than 9 projects: scroll inside the grid (D-31) or the page scrolls (today).
  4. Onboarding demos: play once + Replay (D-13 default), static, or loop (the owner built them looping, S13).
  5. Onboarding title 24 (D-06 ceiling) or 28 (D5 "way too small" history).
  6. Spend sparkline on or off.
  7. Summary sentence vs per-tile details (K5).
  8. Shortlist #5 preview: accent strip on step 1 on/off.
- **Exports back into this spec** (one commit per changed number): lift → Card hover translate override on `.pcard` (§2.2); overflow → §2.1 rule or its removal; plot → DailyBars default `height` (§5.2); series2 → ShareBar colour for Home (`--chart-2` local override only if `ink-muted`); spark → keep/drop commit 31; summary → §2.4 detail column; demo → §5.2 one-shot table (static = no keyframes; loop = owner overrides D-13, register amendment needed); title → §2.7 title token (28 would need a new `--t-*` rung: register amendment); accent strip → 09g.

## 10. Verification + acceptance

Windows dev machine. App via `npm run cdp:dev` + `cdp:serve` (background); audit viewport 2048×1104 @ DPR 1.25 (= 1638×883 CSS px); compact density.

1. `npm run check` → svelte-check 0 errors, `check:tokens` PASS, `check:ui` PASS. `npm test` → green incl. the 7 new formatter cases.
2. `c.sh ready`; `c.sh nav home`; `c.sh errors` → 0; `c.sh console error 20 1` → 0.
3. **Fit.** With 6 projects and news closed: `c.sh eval "(()=>{const s=document.querySelector('.ws-page[data-workspace=home] .home-scroll');return [s.scrollHeight,s.clientHeight]})()"` → first ≤ second. Repeat at regular density and record (no pass/fail; D-31 fit is compact).
4. **States.** Loading: `c.sh reload` and `c.sh shot-sel '.act-card' jpeg 70` within 1s (bones static, no sweep). Error/empty: in DEV, `c.sh eval` the store toggles used by spec 03 M23 (a 1s fetch delay / a rejected `assistant_stats`), screenshot each. Projects empty: shot from board-equivalent state only if a test profile exists; otherwise "not verified live".
5. **Motion budget.** 2s after Home load: `c.sh eval "document.getAnimations().filter(a=>a.playState==='running').length"` → 0. Click a range option (`c.sh click '[data-slot=segmented] [role=radio][aria-checked=false]'`), then the same eval within 300ms → ≥ 1, after 1s → 0. StatTile text sampled at 100ms ≠ final, at 900ms == final. Wait 61s on Home: eval → 0 the whole time (60s tick snaps).
6. **Onboarding.** Read and save `localStorage['rift.onboarding.dismissed']` and `['rift.betaNotice.ack']`, then `c.sh eval "localStorage.removeItem('rift.onboarding.dismissed');localStorage.removeItem('rift.betaNotice.ack');location.reload()"`; `c.sh ready` → "ONBOARDING visible". For steps 1-4: `c.sh shot-sel '.ob-card' jpeg 70`; 8s after each step opens: running animations 0 and `.ob-mini` opacity "1". Replay click → ≥ 1 running. `c.sh key Escape` on step 1 → `.ob-host` still present; on step 3 → step 2. Restore both keys to the saved values.
7. **Reduced motion.** `c.sh media reduce` (P2 M01), reload, repeat 5-6: no running animations except a visible Spinner; demos show the composed scene; Replay hidden. `c.sh media ''` after.
8. **Column edge.** `c.sh measure` the PageHeader inner block and `.home-col` content box: left edges equal ±0.5px.
9. **Blast radius.** `c.sh diff p4d-health ""` (LimitBar card bars only); status bar compact bars and the usage panel (open it) by shot; the chat welcome needs-auth screen only if a signed-out profile is available, else report "welcome connect card not verified live; same component verified in onboarding step 2".
10. **Keyboard.** Tab order on Home: Switch folder → New project → each card (whole-card ring) → its actions → range → bars skipped → spend tile → news toggle → refresh. Onboarding: Stepper done steps reachable; provider RadioGroup arrows.
11. **Screenshots** to `docs/design/halo/baselines/p4d/`: `home-data`, `home-loading`, `home-empty`, `home-error`, `home-editor`, `home-news`, `ob-1`…`ob-4`, `ob-2-chatgpt`, `ob-reduced`.

**Counts that must go down** (Grep per file; `check:ui` totals locked in 36):

| Metric | Before 4d (after P1-P3, est.) | After 4d |
|---|---|---|
| `oklch(` literals: WorkspacePage / LimitBar / OnboardingFlow / onboarding.css live rules | 14 / 0 / 1 / 4 | 0 / 0 / 1 (swatch fill, kept by P1; 0 with 09g) / 0 (the 2 in the RETIRED block go in P5) |
| `font-size:Npx`: WorkspacePage / NewsFeed / onboarding.css live rules | 25 / 4 / 29 | 0 / 0 / 0 |
| `border-radius:Npx` off-ladder: same files | 22 / 5 / 15 | 0 / 0 / 0 (DailyBars 2px mark radius is the one documented exception) |
| `infinite`: onboarding.css / WorkspacePage / NewsFeed / LimitBar | 26 / 5 / 3 / 2 (P2 lowers some) | 0 / 0 / 0 / 0 |
| `@keyframes`: WorkspacePage / NewsFeed / LimitBar / onboarding.css | 5 / 3 / 3 / 23 | 0 / 0 / 0 / 15 one-shot demo keyframes (+1 `db-grow` in DailyBars) |
| `overlay:aria-role` (`check:ui`) | 17 | 16 |
| `projectHue` imports in `components/workspace/` | 1 | 0 |

**Acceptance:** steps 1-10 pass; every count row met; no file outside §1.1 has a diff; Home idle runs 0 animations; onboarding idle runs 0 after 8s; welcome either verified or reported as not verified.

## 11. Risks + mitigations

| # | Risk | Mitigation |
|---|---|---|
| R-1 | Restyling the connect cards changes the chat welcome needs-auth screen (`AssistantWelcome:158`) outside 4d | commits 01-03 land first; standalone sizes target the 440 column; §10 step 9 |
| R-2 | LimitBar restyle shows up on StatusBar/UsagePanel/AiHealth (4a/4c/4f) | API frozen; `aria-hidden` kept; diff the three surfaces in 19 |
| R-3 | Fit budget (823) is computed, not measured | commit 23 measures first; ordered levers; never shrink type |
| R-4 | 1080 column drops today's 4-up fullscreen grid (`:815-817`) | D-31 decision; board 05a shows it; owner sees it before merge |
| R-5 | In-grid scroll beyond 9 projects reintroduces a "twin scroll wheel" the code comment warns about (`:809-811`) | board lever `data-prop-overflow`; D-31 default |
| R-6 | Demo play-once ends on a blank frame if P2 M26 landed as written (§13 S-1) | commit 15 owns the final frames; §10 step 6 checks opacity 1 |
| R-7 | StatTile CountUp is a JS rAF and is not paused by P2's keep-alive CSS pause; a first load while Home is hidden counts invisibly | harmless (700ms, once); never keyed on `statsNow` |
| R-8 | Range clicks in quick succession restart count-ups | CountUp `to()` starts from the displayed value (K04c) |
| R-9 | Onboarding title 24 is smaller than today's 29; the owner once called onboarding "way too small" (D5) | lede moves 12.5 → 15/24 and value text 11 → 13; board lever 24/28 |
| R-10 | Dropping `role="dialog"` reads as ignoring D-30 | §13 A-1 states it; 17-alt spec ready |
| R-11 | Esc on step 1 no longer exits; users who relied on it must use Skip | Skip stays visible on the rail and in the mobile footer |
| R-12 | Model Segmented vs Select switch at >4 options changes control type between providers | intended (Halo Segmented ≤5); Select is the same `shared/Select` Settings uses |
| R-13 | Delta "down" turns from grey to warn orange (D-03) and may read as an alarm on sessions/tool calls | D-24 decided; board 05a shows ↓−8% in warn |
| R-14 | Geist width growth wraps StatTile values ("$12.4k" + chip) | 196px per tile at 1080; `white-space: nowrap`; measure in 28 |
| R-15 | ShareBar violet series vs §8 "no purple defaults" | board 05 decision 1; `ink-muted` fallback is one line |

## 12. Effort + dependencies

- **Effort:** L, 3 sessions. Session A (01-18, onboarding + connect) 1.25; Session B (19-36, LimitBar + Home + news) 1.5; verification, screenshots and fixes 0.25.
- **Depends on:** P1, P2 (incl. M26, M30), P3 (K04-K30), P4a (tooltip layer, Toaster, "Home" naming), P4b (AssistantWelcome). Board 05 does not gate the build (D-27: 4d runs on defaults); its exports are follow-up one-number commits.
- **Unblocks:** 4f reuses `shared/Sparkline`, the LimitBar → Progress path and the StatTile/ShareBar call patterns; 4e/4f retire PageHero.

## 13. Amendments proposed

To the register (00-decisions):

- **A-1 (D-30, onboarding modal).** D-30 says the onboarding `aria-modal` gets a real trap. AppShell renders onboarding *instead of* the shell (`AppShell.svelte:305-309`): only the Titlebar sits beside it, so a trap would only block keyboard access to the window controls. Proposed: it is a page, not a dialog. Drop `role="dialog" aria-modal` (commit 17), label the `<main>` by the step title. Alternative 17-alt if rejected: a focus trap over `.ob-overlay` focusables with initial focus on the step title and Shift+Tab wrap, Titlebar excluded.
- **A-2 (D-09 kit list).** Halo's Stepper is not in spec 02's kit. Proposed: build it as `onboarding/Stepper.svelte` (one consumer); promote to `ui/stepper/` if a second consumer appears.
- **A-3 (D-24 projectHue).** "The `projectHue` util stays but unused" is not true at 4d: ConversationList, ProjectSwitcher and AssistantPane use it. Proposed: those neutralise in 4a/4b (they own the files); the register line reads "unused on Home".
- **A-4 (D-07 ladder).** Chart marks in DailyBars use a 2px top radius: 14-60 bars are 8-40px wide, and 6px would round thin bars into pills. Proposed: data marks are outside the UI radius ladder (one documented literal in DailyBars).
- **A-5 (D-31 PageHeader).** D-31 calls PageHeader "re-skinned PageHero". Spec 02 built a new part. Proposed: Home adopts PageHeader in 4d; PageHero is deprecated (commit 35) and its 4 call sites swap in 4e/4f, so no other page's edge moves before its own port.
- **A-6 (D-31 "two columns at ≥1160").** The column never exceeds 1080, so the breakpoint is read as the page container width (`@container home (min-width: 1160px)`).
- **A-7 (D-10 page entrance).** The inline editor moves inside the Projects section so `.page-sections > *` never gives it the 520ms page entrance on each open; it gets a 300ms 10px rise instead.
- **A-8 (D-32 "empty states offer one action").** The Activity empty state's action is "Open chat" (`workspace.setActive("chat")`), a new affordance on Home.

To sibling specs:

- **S-1 (spec 03 M26).** "`infinite` → `1` + `forwards`" ends three scenes at opacity 0 (`ob-loop-out`, `onboarding.css:510`) and the dial at .22 (`:528`). Fix: delete `ob-loop-out`, end every one-shot on its composed state; also convert `ob-tree-scan` (`top`) to `translateY` and `ob-mcl-type` (`max-width`) to `clip-path` (§5.2). Commit 15 carries it.
- **S-2 (spec 03 #124 vs spec 02 Card).** 03 gives the hub card lift `--dur-base`; the kit Card transitions `translate` at `--dur-fast`. 4d follows the kit (150ms).
- **S-3 (spec 02 PageHeader `--page-col`).** A var published on the header host cannot reach a sibling body. Home sets `--page-col` on the common root `.home`.
- **S-4 (spec 03 §5.2 exemptions).** Add `{#key step}`, `{#key run}`, DailyBars/Sparkline `{#key rangeKey}` and `{#key stepIdx}` to the intended-replay list.
- **S-5 (spec 02 migration table `:1485`, `.ob-seg*` → Segmented).** 4d maps the provider control to RadioGroup cards (each option carries a description line) and the model control to Segmented only at ≤4 options, `shared/Select` above that (ChatGPT has 9, M-5). Segmented stays the target for every `.ob-seg` use that fits it.

**Map / spec corrections found while verifying:**

- **M-1.** The onboarding demo block has 24 `infinite` declarations (`onboarding.css:482-507`; 26 in the file with `:324,:361`), not 22 (map 05 §4.4, map 09 I29, spec 03 M26/§4.7).
- **M-2.** `WorkspacePage.svelte:966` says `.branch-pill` is shared with AssistantWelcome; the only consumer is `WorkspacePage.svelte:423` (`app.css:682`).
- **M-3.** `WorkspacePage.svelte:1104-1105` says "Default window = 3-up"; at the 1340 column the audit screenshot shows 4-up (map 05 header). Stale comment, deleted with the rule.
- **M-4.** Spec 02 §14 Q5 says the accent Segmented thumb is "used by onboarding step 1". Step 1 has no Segmented (provider is step 2 and becomes a RadioGroup; the model control is step 4, neutral).
- **M-5.** The ChatGPT default-model list has 9 entries (`modelMatrix.ts:80-88`); today's `.ob-seg` (`onboarding.css:209-212`, no wrap) cannot hold them. Map 05 §2.3 does not mention it.
- **M-6.** Map 05 §1/§7 confirmed: LimitBar has 6 call sites in 3 files; PageHero 4 call sites in 3 files; `.ob-host` is the only class any script reads (`c.sh:548`).

## 14. Open questions not settled by the register (default in **bold**)

- **Q-1.** Keep the inline project editor or move it to a Drawer/Dialog (map 05 §10 Q8)? **Inline Card inside Projects.**
- **Q-2.** Page eyebrow on Home: "Home" or none? **"Home".** It tells the reader the title is a folder name.
- **Q-3.** More than 9 projects: in-grid scroll (D-31) or page scroll? **In-grid after 3 rows**, board lever.
- **Q-4.** Drop the "mostly X" caption and the summary sentence (K5)? **Drop; details move into tiles**, board lever `data-prop-summary`.
- **Q-5.** Sparkline tone on the spend tile? **`ink-muted`** (spend rising is not "good"; accent stays scarce).
- **Q-6.** Activity empty-state action? **"Open chat"** (A-8).
- **Q-7.** Copy feedback 1.6s → 1.8s in the connect cards to match D-18 CodeBlock? **Yes (commit 01).**
- **Q-8.** Model picker when more than 4 options? **`shared/Select`; Segmented at ≤4** (§13 S-5).
- **Q-9.** Should the "connected" state carry a LiveDot? **No.** Connected is a state, not activity (D-13); a static accent Banner with a check.
- **Q-10.** Onboarding title 24 or 28? **24** (D-06 ceiling), board lever.
- **Q-11.** Should DailyBars stagger its bars like Halo BarChart? **No** (spec 03 #122; 60 bars would take seconds).
- **Q-12.** Card hover lift 3px or 2px? **3px** (D-16), board lever.
- **Q-13.** Range Segmented thumb neutral or accent? **Neutral** (spec 02 A-6).
- **Q-14.** Show the onboarding Replay button before the demo ends? **Yes, always visible** (D-32 actions visible), hidden only under reduced motion.

