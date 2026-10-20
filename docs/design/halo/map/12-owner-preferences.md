# 12 — Owner preferences + rejections (map only, read-only wave)

Scope: taste constraints the Halo plan must honour or explicitly renegotiate. Status of this map: sources read in full = `feedback_rift.md`, `project_rift_tauri.md`, `reference_tauri_svelte_frontend.md`, `feedback_ux_preferences.md`, `design-system/DESIGN.md`, `docs/design/redesign-plan.md`, `docs/HANDOFF.md`, `docs/design/claude-code-ui-reference.md`, `docs/CHANGELOG.md`, Halo brand book (README, 01-08, Card/Button/Badge/Skeleton/LiveDot/CodeBlock/Tooltip/Toast/Dialog/SectionLabel/Stat/NavItem/AppShell/Motion READMEs), Halo `bundle.css` + `tokens.css` greps. Daily notes read = 06-09, 06-21, 06-30, 07-02, 07-06, 07-08, 07-09, 07-10, 07-11, 07-14, 07-15, 07-16, 07-23, 10-19 (targeted line ranges, not whole-file) + grep sweeps of 06-xx..09-xx. Daily sweeps: 06-xx..09-xx via glob `2026-0[6-9]*.md` (regex over owner/tacky/laggy/etc.) and 10-01..10-19 via glob `2026-10-*.md` (regex `owner|rift.*(ui|design|motion|…)`); the October sweep found NO Rift taste statements for 10-01..10-18 (hits = crumbtrail/migration notes only, `Daily/2026-10-01.md:20,34`), so the newest taste data is 10-19 (`Daily/2026-10-19.md:20,31,34`). Keyword greps can miss unquoted preferences. NOT read: git history before `f2f4a82d`, Settings/AI Health source (only grep-cited), the first Halo mock `Main.dc.html` (not in this wave's list; Q21).

## 0. Read this first (staleness + framing)

| # | Fact | Cite |
|---|---|---|
| 0.1 | Brief of 2026-10-19 AM was "EVOLVE Graphite Ink — not a new look, not a layout overhaul". The Halo move supersedes it (owner message this session). Every Rift taste rule below was written FOR Graphite Ink; treat each as "honour or renegotiate", not "void". | `docs/design/redesign-plan.md:9-12` |
| 0.2 | Motion pacing already CHANGED in commit `22727d24` (HEAD of `design/chat-ledger`): `WORD_MS` 42→30, catch-up 520→320ms, follow lerp 0.2→0.35, `blockIn` 8→3px, footer springs removed, railGlint deleted, held-word blur removed, jump pill backdrop-filter removed. `docs/HANDOFF.md:26-36` and `redesign-plan.md:178-216` still describe these as TODO/"toggles" = STALE. | `src/lib/components/assistant/Markdown.svelte:169-172`; `AssistantPane.svelte:183-185`; `stream.css:25-27,65-68`; `git show 22727d24` message |
| 0.3 | Owner verdict on the 22727d24 feel is UNKNOWN. Plan promised "Never ship a pacing/reveal change speculatively" + one toggle at a time, felt live, but the commit shipped all together. | `redesign-plan.md:209-214` vs commit msg ("Owner 2026-10-19: streaming felt laggy") |
| 0.4 | `docs/design/PREFERENCES.md` ("living taste memo", Daily 07-23) NO LONGER EXISTS (not in repo, not in archive). `DESIGN.md` §8 is the surviving taste log; `docs/design/README.md:10` points there. | `Daily/2026-07-23.md:145`; `docs/design/README.md:10`; glob = no file |
| 0.5 | Motion/atmosphere = "the owner's most-rejected category". | `Daily/2026-10-19.md:20`; `redesign-plan.md:209` |
| 0.6 | Working tree clean at map time; branch `design/chat-ledger`; HEAD `22727d24`. | `git status --short` empty, `git log` |
| 0.7 | CC UI reference §2 says a blinking tail caret is correct; owner DELETED the green blinking caret ("absolutely hate it"). Owner wins; that ref section is stale on this point. | `claude-code-ui-reference.md:55-58` vs `Daily/2026-07-02.md:107` |
| 0.8 | Owner session message (this session): Rift front end "needs an update"; wants "proper motion and animations"; "find a happy medium for literally everything"; first Halo mock "rough". | session user message 2026-10-19 (no file) |

## 1. Every stated preference / rejection

Legend: **HOLD** = standing rule in DESIGN.md/code, **DONE** = implemented fix, **PENDING** = owner verdict outstanding, **SUPER** = superseded by a later owner call. Quotes ≤15 words.

### 1.1 Layout

| ID | Preference / rejection | Source | Date | Status |
|---|---|---|---|---|
| L1 | Docked "island" shell, Claude-Desktop-inspired: sidebar + main as rounded cards on a canvas. | `Daily/2026-07-15.md:12` | 2026-07-15 | HOLD |
| L2 | Hover-peek must NOT be the full sidebar over content: "overlapping everything… not framed correctly". Peek = minimized sibling (240px, content-hugging height, "All chats · N" seam). | `DESIGN.md:167-172`; `Daily/2026-07-15.md:21` | 2026-07-15 | HOLD |
| L3 | Peek→pin morph is a SNAP; "ease if abrupt". Not yet felt by owner. | `Daily/2026-07-15.md:25` | 2026-07-15 | PENDING |
| L4 | Two docked islands = ONE material: "blended in with the sidebar"; "the sidebar does not match the other pages". Grep every page-conditional variant after a material change. | `AppShell.svelte:397`; `DESIGN.md:212-218` | 2026-07-16 | HOLD |
| L5 | Islands speak ONE dialect, ONE level deep: canvas → island → content; inside an island use hairlines / tint tiles, never a second island. Status bar = main island's footer. | `DESIGN.md:173-185` | 2026-07-15 | HOLD |
| L6 | Canvas is a RECESSED CHASSIS ("seems off" → "I want something more different"): darker canvas, 12px gutters, contact AO, cavity shading, accent `.rift-seam` hairline between islands. | `DESIGN.md:81-95`; `Daily/2026-07-23.md:122-126` | 2026-07-23 | HOLD (newest canvas call) |
| L7 | Welcome: "one column, nothing floating"; greeting/context/resume/composer share left+right edge (844px). | `AssistantWelcome.svelte:457-460` | 2026-09-18 | HOLD |
| L8 | Welcome must not scroll (tool-less no-folder case). | `AssistantWelcome.svelte:277` | 2026-09-22 | HOLD |
| L9 | Chat message column and composer share edges (finding C6); picked in Ledger. | `redesign-plan.md:53,136-139` | 2026-10-19 | DONE (`8bb71f4f`) |
| L10 | Tabs pill: "saw it live, wanted it gone"; zero tab UI = accepted state. | `Daily/2026-07-17.md:16` | 2026-07-17 | HOLD |
| L11 | Quick-start chips on welcome: "do away with those" (files deleted). | `Daily/2026-07-02.md:18,25` | 2026-07-02 | HOLD |
| L12 | Floating Plan/Agent HUDs must not duplicate inline cards: "feels duplicated" → show only while inline card is off-screen. | `Daily/2026-07-08.md:31` | 2026-07-08 | HOLD |
| L13 | Old agents/shells/tools count pills near composer = "old displays", ripped out. | `Daily/2026-06-21.md:180,183` | 2026-06-21 | HOLD |
| L14 | Hub: bottom-pinned footer docked under Activity band; resume strip cut ("nobody resumes old"). | `WorkspacePage.svelte:551,1179` | undated | HOLD |
| L15 | Composer revamp: "feels outdated"; Claude Desktop screenshots = north star (3-layer stack, home launcher, FLIP descent). | `Daily/2026-07-14.md:96-98` | 2026-07-14 | HOLD |
| L16 | Ledger: reply first, "one quiet receipt line under it". Chosen over Spine (B) and Document (C band). | `redesign-plan.md:135-139` | 2026-10-19 | HOLD |
| L17 | `/mcp` as notice rejected → centered glass dialog (palette chrome). | `Daily/2026-07-10.md:95` | 2026-07-10 | HOLD |
| L18 | Page shell: one column + one left edge per page (4 edges today = imprecision). | `redesign-plan.md:80-84` (Settings/Health/Home edges; plan Phase 5 :170-172) | 2026-10-19 | audit finding, not yet fixed |

### 1.2 Density

| ID | Preference / rejection | Source | Date | Status |
|---|---|---|---|---|
| D1 | Compact is the default: "denser and quieter, not bigger and louder". | `DESIGN.md:52-53` | 2026-07 | HOLD |
| D2 | Density is USER-SET (compact/regular/comfy via `--row-h/--gap/--fs-md`); new components must consume vars, no hard-coded row heights. Default `"compact"`, plus `uiScale`. | `DESIGN.md:48-51`; `ui-prefs.svelte.ts:98,114` | — | HOLD |
| D3 | Detail-first transcript: "show as much detail as possible — that's what tools lack". | `Daily/2026-07-02.md:44` | 2026-07-02 | HOLD |
| D4 | But narration must not out-weigh work: "feels more chatty, less working"; "I love to see what's going on" → Narration pref (Focused/Balanced/Chatty). | `Daily/2026-06-30.md:49,56`; `ui-prefs.svelte.ts:107` | 2026-06-30 | HOLD |
| D5 | Undersized floating UI is also wrong: onboarding "way too small, needs to be expanded". Overscaled: composer text "big and funky". Both directions rejected. | `Daily/2026-06-20.md:74`; `Daily/2026-06-21.md:182` | 2026-06 | HOLD |
| D6 | "dislocated numbers" (clipped/wrapped metrics) → abbreviate + exact on tooltip, min-widths. | `Daily/2026-09-18.md:54` | 2026-09-18 | DONE |
| D7 | No oversized hero numerals (AI-slop tell); the `5,914` hub numeral flagged. | `DESIGN.md:146`; `redesign-plan.md:71` | 2026-10-19 | audit finding |
| D8 | Tool-run bars: full-width bars "90% empty chrome — owner: tacky" → content-hugging island chip. | `stream.css:252-259` | 2026-07-22 | HOLD |
| D9 | Reading is ADHD-sensitive (dense report walls hard to read). Inference for UI: fewer simultaneous competing signals is a first-order requirement, not polish. | `feedback_ux_preferences.md:10-20` | 2026-09 | HOLD (inference, flagged) |

### 1.3 Colour

| ID | Preference / rejection | Source | Date | Status |
|---|---|---|---|---|
| C1 | Accent is scarce + themeable. Solid accent only on Send + at most one CTA per screen; everything else ghost (`accent-soft` + `ghost-border`). | `DESIGN.md:26-29` | 2026-07 | HOLD |
| C2 | One chromatic voice per view; "A second decorative hue is a bug"; no second accent, no spotlight cards, no purple/lavender defaults. | `DESIGN.md:33-34,145-146` | 2026-07 | HOLD |
| C3 | Accent = free, amber = costs money (pay-per-use shows warn treatment AT the control). Owner: "it's charging me, that is a huge gap". | `DESIGN.md:219-225` | 2026-07-14 | HOLD (high stakes) |
| C4 | "tacky blue" inline code/links → calm neutral chip, restrained links. | `Daily/2026-06-21.md:180,185` | 2026-06-21 | DONE |
| C5 | Accent wash on chat atmos = "owner pet peeve" (neutralized). | `Daily/2026-07-02.md:20` | 2026-07-02 | HOLD |
| C6 | Quiet ≠ invisible: "still not impressed… something more immersed" — the STAGE must be plainly visible; restraint applies to chrome. | `DESIGN.md:186-189` | 2026-07-16 | HOLD |
| C7 | Atmosphere is ONE lighting model: "shit stacked on top of shit" → delete all page-local washes/glows/grain. | `DESIGN.md:190-196`; `AssistantWelcome.svelte:462-463` ("read as blotches") | 2026-07-16 | HOLD |
| C8 | Dots/specks banned ("I'm talking about the dotted background"); pattern stays INSIDE its frame ("going outside of the frame"); grid + accent spotlight retired: "easy on the eyes". Surviving language = MATERIAL (satin, bevel, tonal crown/foot, grain ≤3%). | `DESIGN.md:197-211` | 2026-07-16 | HOLD |
| C9 | No diagonal sheen: "sideways glares" — tonal gradients strictly VERTICAL. | `DESIGN.md:73-76` | 2026-07-16 | HOLD |
| C10 | 12-texture picker "being extra" → removed (later all textures retired per C8). | `Daily/2026-07-11.md:14` | 2026-07-11 | DONE |
| C11 | Don't drop a sunken near-black well into a flat menu: "looks like shit compared to the rest of the menu". Borrow a sibling control's recipe; additive ask = additive change. | `DESIGN.md:236-243` | 2026-07-23 | HOLD |
| C12 | Active project card: "calm it" — cooled border, no glow halo. | `WorkspacePage.svelte:1116-1117` | 2026-07-16 | HOLD |
| C13 | Dark direction only. (Halo ships 4 light themes.) | `project_rift_tauri.md:14` | perpetual | HOLD → see Q3 |
| C14 | `--fg-subtle` lifted L .545→.600 (a "taste call"; owner approved push). `fg-faint` meta near-invisible (1.57:1) flagged. | `HANDOFF.md:64-67,70-71`; `redesign-plan.md:48` | 2026-10-19 | DONE / DONE |
| C15 | Per-project hues + 5-hue model-mix bar: built under delegated "decisions on you", later audited as second-accent noise; model-mix legit dataviz? UNDECIDED. | `Daily/2026-07-10.md:111`; `redesign-plan.md:72,223` | 2026-07-10 / 10-19 | PENDING |
| C16 | Lift + hairline, not shadow; shadows only for floating layers. Contact AO on docked islands is allowed ("occlusion, not a float shadow"). | `DESIGN.md:23-25,94-95` | 2026-07-23 | HOLD |
| C17 | User-adjustable accent: 8 curated hues + vividness slider drive whole ramp via `--accent-h/c/a`; default emerald hue 163. | `ui-prefs.svelte.ts:82-93,99-100`; `app.css:167-180` | — | HOLD (owner-built) |

### 1.4 Type

| ID | Preference / rejection | Source | Date | Status |
|---|---|---|---|---|
| T1 | UI = Inter (optical features, −0.005em); mono = JetBrains Mono for code/paths/IDs, "never for chrome labels"; Lexend = brand (greeting, splash) + dyslexia mode only. | `DESIGN.md:38-40`; `app.css:257-258,304-314` | 2026-07 | HOLD |
| T2 | Scale small + tight (11→22); hierarchy by weight 400/500/600 + colour ladder, not size jumps; 600 ceiling; "Demote before you shrink". | `DESIGN.md:25-26,41-43`; `app.css:260-265` | — | HOLD |
| T3 | Section labels 10px/600/UPPERCASE/0.08em/`fg-faint` + fading rule — the one place for positive tracking. | `DESIGN.md:44-45` | — | HOLD |
| T4 | Mono used for chrome meta (timestamps, durations) is a defect (C13 finding); receipt in `fg-subtle` Inter tabular-nums. | `redesign-plan.md:60,112-114` | 2026-10-19 | DONE in Ledger |
| T5 | Greeting font bug fixed: `"Lexend Variable"` is the registered face. | `HANDOFF.md:61-63` | 2026-10-19 | DONE |
| T6 | 388 hard-coded `font-size` literals / 10 sizes in Settings: unacceptable imprecision; wants scale tokens ("every number tabular"). | `redesign-plan.md:80-83,103` | 2026-10-19 | audit finding |
| T7 | Dyslexia-friendly mode (Lexend, line-height boost, warm tint) is an owner feature. | `src/lib/state/accessibility.svelte.ts:1,9,18-30` | — | HOLD |
| T8 | Sentence case, not Title Case. | `redesign-plan.md:93` | 2026-10-19 | audit finding |

### 1.5 Motion (summary rows; full likes/dislikes in §3)

| ID | Preference / rejection | Source | Date | Status |
|---|---|---|---|---|
| M* | See §3 table M1-M24. Cross-ref only here (one fact, one home). | §3 | — | — |

### 1.6 Chrome (controls, tiles, overlays)

| ID | Preference / rejection | Source | Date | Status |
|---|---|---|---|---|
| K1 | Buttons ghost-first; no pill-round buttons/inputs; pills only for badges/toggles. Existing exceptions: `.jump-latest` (999px, badge-like float), `.chip-btn` 999px (flagged to fix). | `DESIGN.md:107-108,147`; `AssistantPane.svelte:1375-1385`; `redesign-plan.md:69-70` | — | HOLD |
| K2 | Chrome quieter than content ("does the chrome get louder than the content?"). | `DESIGN.md:14-15,135` | — | HOLD |
| K3 | Tool activity = islands (hairline + faint fill), owner "too transparent"; prose boxless. | `DESIGN.md:159-165` | 2026-07-15 | HOLD |
| K4 | Stream tiles = one family (Card/Row/Prompt/Float); state is stationary class; glyph colour = status never tool kind; no bordered well inside a tile. | `DESIGN.md:119-128`; `redesign-plan.md:55-56` | 2026-09/10 | HOLD |
| K5 | One fact gets one home (provider/model/plan/project/branch not repeated). Plan usage shown 3× and project/branch 3× are defects. Live tool caption in footer removed in `22727d24`. | `DESIGN.md:141-142`; `redesign-plan.md:52,89`; commit msg | 2026-10-19 | HOLD / DONE |
| K6 | Menus/selects/dialogs are `components/ui/` primitives (shadcn-svelte over bits-ui), recipes in `ui/recipes.ts`; hand-rolled overlays ratchet only DOWN (`check-ui.mjs`). | `DESIGN.md:267-306` | 2026-09-25 | HOLD |
| K7 | Floating tier = `--surface` + `--border-strong` + `--shadow-lg`, 12px radius over 4px padding; dialog dim = opaque `--scrim`, NEVER backdrop-filter (WebView2); z: dialog 4000 < popover 9000 < tooltip/splash 9999. | `DESIGN.md:283-299`; `app.css:274` | 2026-09-25 | HOLD |
| K8 | Kbd-soup placeholder: "looks like ass" → one quiet rotating phrase. Picker hotkey strip = "owner pet peeve", ⇧Tab chip removed. | `DESIGN.md:154-158`; `Daily/2026-07-02.md:38`; `Daily/2026-07-14.md:98` | 2026-07-14 | HOLD |
| K9 | "Tacky UX": launch toast "MCP not connected", never-ending shell dropdowns. | `Daily/2026-07-10.md:61` | 2026-07-10 | DONE |
| K10 | Stock WebView2 right-click menu = "a bit stock" → custom menu. Owner dislikes default/stock chrome. | `Daily/2026-06-10.md:57` | 2026-06-10 | DONE |
| K11 | Mic: "calm down" — waveform is the ONLY motion; countdown = thin ring. | `Daily/2026-07-15.md:12` | 2026-07-15 | HOLD |
| K12 | Dialog focus/Cancel-default for destructive; confirm dialogs not modal-stealing in transcript (permission prompts inline, never modal). | `CHANGELOG.md:8-12`; `claude-code-ui-reference.md:194-199` | 2026-09 | HOLD |
| K13 | Fake window chrome (macOS dots) on previews and left-border "Healthy" card and accent spotlight stat tile = defects. | `redesign-plan.md:86-93` | 2026-10-19 | audit finding |
| K14 | Effort control: Faster↔Smarter detent slider, "slot-machine feel" (free-follow, spring-snap on release), Claude-Desktop anatomy; sibling-recipe track. | `SettingsMenu.svelte:205-208,443-446`; `DESIGN.md:236-243` | 2026-07-22/23 | HOLD (owner-specified) |

### 1.7 Copy

| ID | Preference / rejection | Source | Date | Status |
|---|---|---|---|---|
| P1 | Name what the user controls, active voice, same verb through a flow; errors = what happened + what to do; empty state invites ONE action. | `DESIGN.md:109-112` | — | HOLD |
| P2 | One user-facing "ChatGPT" identity; provider cards same anatomy; show Codex/OpenAI only where setup/billing needs it. | `DESIGN.md:136-140` | — | HOLD |
| P3 | Honest signals only: fake progress bar "killed on principle"; whimsy idle verbs (Mapping/Pondering) retired as fabricated; "Worked for 0s" → real wall clock; token meter monotonic. | `Daily/2026-06-09.md:282`; `Daily/2026-07-06.md:93`; `Daily/2026-07-10.md:82`; `Daily/2026-07-15.md:40` | 2026-06..07 | HOLD |
| P4 | Plain professional voice, jargon out (onboarding copy pass). | `Daily/2026-07-09.md:66` | 2026-07-09 | HOLD |
| P5 | Cost disclosure copy at the control, never only a tooltip. | `DESIGN.md:219-225` | 2026-07-14 | HOLD |
| P6 | Tooltips: Rift tooltips "never paint" (`actions/tooltip.ts`) — known gap; receipt tokens therefore on hover-title. | `HANDOFF.md:40` | 2026-10-19 | OPEN |

### 1.8 Taste tensions the plan must design INSIDE (owner contradicts himself, legitimately)

1. Immersive stage vs restraint: "still not impressed" (C6) AND "shit stacked on top of shit" (C7). Resolution so far = ONE lighting model + material finish (C8). Halo's "no colour washes" sits on the restraint side; Halo has no equivalent of the slab/seam.
2. Built-then-rejected pattern: comet arc built under "make it look cooler" delegation (`Daily/2026-07-10.md:106,110`), rejected 12 days later "I like it but it's disturbing" (`DESIGN.md:245-249`). Also per-project hues (C15). Lesson: delegated "surprise me" additions have a high rejection rate; Halo extras (spotlight, magnetic, FocusGroup dim) are the same risk class.
3. Detail-first (D3) vs clutter (audit C1-C14). Resolved by hierarchy (Ledger), not by hiding.
4. Want visible liveness ("I love to see what's going on", D4) vs no traveling motion (M2-M4). Resolved by STATIONARY signals (breathing dot, tinted border, ticking timer).

## 2. Conflict matrix: Rift rule × Halo rule

Verdict key: **AGREE** / **CONFLICT** / **SILENT** (Halo says nothing) / **PARTIAL**. "Resolution" = recommended happy medium; **[OWNER]** = needs a Q in §5 before it's locked. Halo cites: `HALO` = `…/scratchpad/artifact-files/705c5425-86e3-4761-a421-4ee89499e5c6/project`; `H-CSS` = `HALO/components/bundle.css`; `H-TOK` = `C:/AI Workflow/projects/consolex/panel/design-systems/halo/tokens.css`.

### 2.1 Surfaces, material, depth

| # | Rift rule (cite) | Halo rule (cite) | Verdict | Resolution |
|---|---|---|---|---|
| X1 | Surface ladder `bg`→`bg-inset`→`elev-1/2/3` (5 rungs); "content sinks, chrome lifts": wells (composer, code) on DARKER `bg-inset` (`DESIGN.md:19-22,108-109`) | 4 rungs `bg-000/100/200/300`; Inset `bg-300` = tracks/skeletons, LIGHTER than surface (`HALO/01-themes-and-colour.md:24-28`) | CONFLICT (direction of "inset") | Map `bg`→000, `elev-1`→100, `elev-2`→200. Composer/code wells: use `bg-100` + `line-200/300` border (Halo input recipe) instead of darker-than-page; keep `bg-300` for tracks only. Retire `bg-inset` and `elev-3` or alias them. Keep one-step rule. |
| X2 | Onyx SLAB islands: vertical gradient fill, bevel (inset 1px accent-cooled top catch), grain ≤3%, recessed chassis, contact AO, `.rift-seam` (`DESIGN.md:64-96`; `AppShell.svelte:348-413`) | "No colour washes; the one gradient allowed is Card spotlight"; Card: "No shadows, no colour washes" (`HALO/README.md:7`; `HALO/components/Card/README.md:5`) | CONFLICT (owner-mandated material vs Halo flat steps) | **[OWNER] Q1.** Recommended: keep the island/chassis STRUCTURE (it answers "immersed", C6) but express it in Halo steps: canvas `bg-000` (recessed), islands `bg-100`, hairline `line-100`; keep contact AO + 1px top catch (hairlines, not washes); keep `.rift-seam`; drop grain only if banding is absent on the flatter ramp (grain exists solely as banding-killer, `AppShell.svelte:370-371`). Do NOT ship a flat Halo look without the owner seeing it beside the slab (rule C6). |
| X3 | Lift + hairline, shadows only for floating layers (`DESIGN.md:23-25`) | `shadow-pop` only for popovers/menus/dialogs/drawers/toasts (`HALO/01:34`) | AGREE | Adopt `shadow-pop` (`0 12px 32px .45`, `H-TOK:41`) as `--shadow-float`/`--shadow-lg`. |
| X4 | Card hairline `--border`; hover `border-strong` (`DESIGN.md:61`); hub card hover `translateY(-1px)` (`WorkspacePage.svelte:1112-1113`) | `line-100` → `line-200` on hover; `lift` = `translateY(-3px)` only when whole card clickable (`HALO/Card/README.md:3,7`; `H-CSS:32`) | PARTIAL | Hairline-brighten: adopt. Lift: only clickable cards (project cards, mock tiles) and use ≤2px for calm; 3px is Halo spec — felt test **[OWNER]**. Stream tiles never lift. |
| X5 | NO spotlight cards (accent spotlight retired 2026-07-16) (`DESIGN.md:145,206-211`) | `Card spotlight`: cursor-following light along hairline + 4% sheen, hover-only, "hero and overview cards, not every card" (`HALO/Card/README.md:8`; `HALO/04-performance.md:7`) | CONFLICT (nominal) but different class: interaction-driven, fades on leave, no idle motion | Default OFF in first build. Candidate: ≤3 hero cards (Home hub overview, AI Health summary), never stream tiles/lists/chat. Owner must FEEL it live before it ships (his rejections target ambient/idle light). **[OWNER] Q7.** |
| X6 | Focus = `--ring` (translucent accent, α=`accent-a*3.75`), always visible (`DESIGN.md:64`; `app.css:180`) | `shadow-focus`: 2px gap + solid 2px accent ring on every control (`HALO/01:34`; `H-TOK:42`) | PARTIAL (both visible; Halo solid + gap) | Adopt Halo's solid ring + gap; passes 3:1 more reliably. Keep Rift focus-visible-only behaviour. |
| X7 | Z-stack: dialog 4000 < popover 9000 < tooltip/splash 9999, so a menu inside a dialog works (`DESIGN.md:296-298`) | Layers: dropdown < overlay (dialogs) < toast < tooltip (`HALO/02:17`) | CONFLICT (menu-in-dialog order) | Keep Rift order; map names onto Halo's token names only. |
| X8 | Dialog dim opaque `--scrim`, never backdrop-filter; dialogs 95% zoom + 8px slide, `--dur-fast` (`DESIGN.md:297-299`) | Dialog "scales in over a dimmed backdrop" (`HALO/Dialog/README.md:3`); `halo-zoom` .94 + 8px (`H-CSS:287`) | AGREE | Adopt Halo `halo-zoom`; keep opaque scrim. |
| X9 | Rift is "a quiet, precise instrument, not a dashboard"; chat is protagonist (`DESIGN.md:11-12`) | "Organised like an instrument panel": bento, numbered SectionLabels, card anatomy label→value→detail (`HALO/README.md:9`) | PARTIAL (philosophy) | Chat/transcript = Rift/Ledger rules (no bento). Bento + Halo card anatomy ONLY on Home/Workspace hub, AI Health, Settings. |
| X10 | Page-conditional material variants forbidden (grep after any material change) (`DESIGN.md:212-218`) | Same surfaces everywhere (themes differ only in colour) (`HALO/01:5`) | AGREE | Carry the rule over: grep for stale variants when switching tokens (`.sidebar.home`-class bugs). |

### 2.2 Colour

| # | Rift rule | Halo rule | Verdict | Resolution |
|---|---|---|---|---|
| X11 | Accent count: solid accent ≤ Send + 1 CTA/screen; all else ghost; accent derived from user hue+vividness (`DESIGN.md:26-29`; `ui-prefs.svelte.ts:82-100`) | "One signal colour": accent marks primary action, live state, positive change, focus, main data series; chart-1..4 for series (`HALO/README.md:8`; `HALO/01:33`) | AGREE on count; CONFLICT on source (Halo = 4 fixed presets, Rift = continuous hue/vividness) | Keep Rift's hue+vividness engine, re-anchored so default = Halo Electric blue `#6aa8ff` (`H-TOK:64`); expose Halo's 4 presets as named swatches among the 8. **[OWNER] Q2** (default flip emerald→blue). Preserve ghost-first rule; Halo SectionLabel's accent index (X27) must not inflate accent count. |
| X12 | Dark only (`project_rift_tauri.md:14`) | 8 themes (4 light) (`HALO/01:5-16`) | CONFLICT | Dark `blue` only for Phase 1; leave light tokens out of the build. **[OWNER] Q3.** |
| X13 | Status ≠ activity ≠ accent; status tokens `ok/warn/danger/info` + `-soft`; amber = costs money (`DESIGN.md:30-32,219-225`) | Positive = accent/accent-soft; `warn` = negative (amber), `danger`, `info`; status always carries arrow/icon/word (`HALO/01:32`) | PARTIAL: Halo makes positive = accent (blue) not green | Keep green `--ok` for pass/success glyphs in the transcript (activity/outcome split, `claude-code-ui-reference.md:201-212`) — or accept blue-for-positive on dashboards only **[OWNER]**. Preserve amber=money; Halo `warn` amber is the same hue family (good). |
| X14 | Per-project hues + multi-hue model-mix bar (`Daily/2026-07-10.md:111`; `redesign-plan.md:72,223`) | One accent per chart; chart-2..4 allowed for series (`HALO/01:33`) | AGREE with the audit, CONFLICT with shipped code | Project avatars → neutral initials on `bg-200`, accent only for active; model-mix = legit multi-series → chart-1..4. **[OWNER] Q11.** |
| X15 | Text ladder 5 rungs `fg`→`fg-faint`; `fg-faint` ≈ 1.57-2.3:1 in use (`DESIGN.md:24-26`; `redesign-plan.md:48`) | `ink/ink-muted/ink-faint`; faint "only for section labels, axis ticks, timestamps (never on bg-300)", floor 4.5:1 (`HALO/01:30`; `HALO/06:3`) | AGREE direction; Halo stricter | Collapse to 3 rungs + faint-floor ≥4.5:1; kills the "quiet = invisible" class (C1). |
| X16 | Syntax colour: Shiki per-line in diffs/Read/shell commands; ANSI colour + semantic tone overlay (`claude-code-ui-reference.md:160-161`; `Daily/2026-07-11.md:39`) | CodeBlock: "No syntax colouring: Halo keeps one signal colour" (`HALO/CodeBlock/README.md:10`) | CONFLICT (strong) | **[OWNER] Q6.** Recommended: keep syntax colour inside code/diff/Read bodies using a muted palette derived from `chart-1..4` + ink ladder (a declared Halo extension), keep ANSI tone; Halo CodeBlock rule governs config/console blocks only. Dropping it contradicts "detail-first" (D3). |

### 2.3 Type

| # | Rift rule | Halo rule | Verdict | Resolution |
|---|---|---|---|---|
| X17 | Inter Variable + JetBrains Mono Variable + Lexend (`app.css:257-258`; `package.json:41-43`) | Geist (words) + Geist Mono (numbers, labels); loaded from Google Fonts by `bundle.css` (`HALO/02:5`; `H-TOK:60-61`) | CONFLICT | Adopt Geist + Geist Mono, BUNDLED locally via fontsource (Rift is offline-first; do not link Google Fonts). Availability of `@fontsource-variable/geist{,-mono}` on npm is UNVERIFIED here (no network step in this wave) — verify before planning. Keep Lexend ONLY for dyslexia mode (T7) and, if owner agrees, the greeting. **[OWNER] Q4.** |
| X18 | Mono never for chrome labels (`DESIGN.md:38-39`; C13 finding `redesign-plan.md:60`) | Mono for `metric` numbers, `data` amounts, `label` uppercase section labels (`HALO/02:8`); Segmented buttons mono 13px (`H-CSS:93`) | CONFLICT | Take Halo for numerics + `metric/data` (tabular figures kill jiggle). Keep Rift rule for prose chrome: timestamps/receipt stay SANS `caption` with `tabular-nums` (Halo's own `caption` = timestamps, `HALO/02:7`). Section labels: mono uppercase OK on page headers; not in chat. |
| X19 | Type scale 11→22: xs 11 / sm 12 / md 13 / lg 15 / xl 18 / 2xl 22; body 13 (`app.css:260-265`) | caption 12 / small 13 / body 15 / heading 16 / subtitle 20 / metric 30 / title 32 / display 48; label 11 (`tokens.json:329-440`) | CONFLICT (sizes + ceiling) | Halo scale as the COMFY tier; Rift compact (13 body) stays default (D1/D2). Cap `metric` 30px to Stat cards only; never `display` 48 for stats (D7 "oversized hero numerals"). Replace 388 px literals with these tokens. **[OWNER] Q5, Q12.** |
| X20 | Weight ceiling 600 (`DESIGN.md:42-43`) | display/title/heading/subtitle/metric all 600; label 500 (`tokens.json`) | AGREE | — |
| X21 | Section label 10px/600/0.08em uppercase `fg-faint` + fading rule (`DESIGN.md:44-45`) | `SectionLabel` `01 / OVERVIEW`: 11px/500/0.12em, index in ACCENT, hairline, optional count (`HALO/SectionLabel/README.md:3-6`) | PARTIAL | Use Halo form on page-level screens (Home, Settings, AI Health); keep index `ink-faint` unless owner approves accent index (accent-count rule, X11). Not in chat. **[OWNER] Q12.** |
| X22 | Sentence case (`redesign-plan.md:93`) | "Short, plain labels in sentence case" (`HALO/05:5`) | AGREE | Note: SectionLabel is uppercase by style, content stays sentence-case source text. |
| X23 | Tabular numbers everywhere ("every number tabular") (`redesign-plan.md:103`) | "Mono styles use tabular figures" (`HALO/02:8`) | AGREE | Sans numerics get `font-variant-numeric: tabular-nums` (Geist supports). |

### 2.4 Shape, density, controls

| # | Rift rule | Halo rule | Verdict | Resolution |
|---|---|---|---|---|
| X24 | Pills: badges/toggles only; buttons/inputs never pill; chips radius 4 (`DESIGN.md:47-49,109,147`) | `radius-pill` for badges, chips; Button `radius-md`(12), inputs `radius-md` (`HALO/02:14`; `H-CSS:229,43`) | AGREE on buttons; PARTIAL on chips | Status/count badges → pill (matches Halo Badge, `H-CSS:229`; Rift `.tile-pill`). Inline code chips + ctx chips stay `radius-xs/sm` (code tokens ≠ status). `.jump-latest` pill stays (badge-like float, already documented). `.chip-btn` 999px → button radius (already an audit item). **[OWNER] Q10.** |
| X25 | Radius ladder 4/6/8/10/12; card 12; island 14; hero 16 (`app.css:200-206,210`) | sm 8 / md 12 / lg 16 / pill (`H-TOK:37-40`) | CONFLICT (finer ladder vs 3 steps) | Adopt Halo 8/12/16 + pill. Concentric rule from `DESIGN.md:285` (12px panel over 4px padding → 8px rows) still holds: 16 panel → 12 → 8. Island 14 → 16. Controls ≤28px high take `radius-sm` (8) because 12 on a 26px button reads pill-like. |
| X26 | Button heights 26 / 22 / 20 + lg 32; row-h 28; compact default (`DESIGN.md:294-295`; `app.css:253`) | Button 44px (`sm` 36); touch ≥44px; toggle 44×26 (`HALO/Button/README.md:6`; `HALO/02:15`) | CONFLICT (density) | **[OWNER] Q5.** Windows pointer app → touch-target rationale doesn't apply. Recommended: Rift heights remain the default (compact); Halo 36/44 become `regular/comfy` tiers via the existing density var. Never hard-code. |
| X27 | `Button` variants `secondary/ghost/soft/primary/danger/warn/info/link` (`DESIGN.md:292-295`) | `primary/secondary/ghost/danger`; one primary per view (`HALO/Button/README.md:3-5`) | PARTIAL | Keep `soft/warn/info/link` as Rift extensions (cost disclosure uses `warn`). |
| X28 | `glow-strong` n/a; "Tight component effects (a button aura…) are fine" (`DESIGN.md:77-79`) | `glow-strong` only under a hovered primary button (`HALO/01:34`; `HALO/Button/README.md:8`; `H-CSS:47`) | AGREE | Adopt on the single solid primary (Send, one CTA). Blue 0.28 α (`H-TOK:67`). Never on ghost/secondary. |
| X29 | Scrollbars: 10px, neutral thumb `--border-strong`, hover `fg-faint`, transparent track; 5 sites use `scrollbar-width:none` (`app.css:318-337`; grep) | Thin 10px, no arrows, `line-200` thumb, brighten on hover; sideways-only strips hide (`H-CSS:7-12`; `HALO/02:31`) | AGREE (NOT hidden in either) | Port tokens only (`border-strong`→`line-200`, hover→`ink-faint`). Keep hide-on-horizontal-strips. Prompt's "scrollbars hidden" is not Rift's current behaviour. |
| X30 | Disabled = `fg-faint`; no global opacity (`DESIGN.md:289`) | `opacity-disabled` 0.45 everywhere (`HALO/02:16`) | CONFLICT (minor) | Adopt Halo 0.45 token for controls. |
| X31 | Hover/press: Rift hover = surface step; durations 140/240 (`app.css:247-248`); press unspecified | Hover/press `dur-fast`: colour, border, 3px lift, 0.97 press (`HALO/03:5`) | PARTIAL / press SILENT in Rift | Adopt press 0.97 on buttons only if felt-test passes; skip lift on dense rows. |
| X32 | Hub cards, tiles, receipts are compact; "no hero numerals" | Halo `Stat`: count-up 1100ms, delta chip `↑ 12.4%`, peek rows (`HALO/Stat/README.md:5-8`) | PARTIAL | Adopt Stat anatomy + delta chip with arrow (Halo `05:6`) on AI Health/Home. Keep Rift monotonic guard on live token meter (P3). |

### 2.5 Motion tokens and behaviours

| # | Rift rule | Halo rule | Verdict | Resolution |
|---|---|---|---|---|
| X33 | Durations: fast 140 / base 240 / rise 460 / slow 520 / page 400 / page-out 140 / stagger 62 (`app.css:243-249`) | fast 150 / base 300 / slow 600 / stagger 60 (`H-TOK:43-46`) | PARTIAL (≈ same shape, Halo slower) | Owner said "laggy" (Chat motion). Keep Halo token NAMES; set values to the Rift-tuned set for chat/overlays (150/240/460–520). Halo's 300/600 only for dashboard page entrances. One scale, not two. **[OWNER] Q13** (felt). |
| X34 | `--ease-page` (0.22,1,0.36,1) entrances; `--ease-soft` standard; `--ease-spring` (0.34,1.56,0.64,1) (`app.css:240-242`) | `ease-out` (0.2,0.8,0.2,1); `ease-spring` identical curve (`H-TOK:47-48`) | AGREE | Merge `ease-page`→`ease-out` (Δ imperceptible); keep `ease-soft` for layout shifts only if Halo has no equivalent (Halo silent). |
| X35 | Spring used on status dot pop, sturn-head label rise, footer verb (removed) (`stream.css:77-82`; commit `22727d24`) | `ease-spring` ONLY for things you flip/grab/add: toggle knob, checkbox/radio pop, slider thumb, tag chips, calendar day, drop-zone icon (`HALO/03:16`) | CONFLICT (scope) | Follow Halo's scope. Keep: detent-slider spring-snap (owner-specified, `SettingsMenu.svelte:205-208`), toggle/checkbox pop. Move status dot/label/verb to `ease-out`. Per-word verb spring was removed in `22727d24` after the "laggy" complaint; owner verdict pending (M13). |
| X36 | Entrance travel: `blockIn` 3px (was 8px — "lurching downward"), `workOpen` -4px (`stream.css:25-27,33`) | Cards rise 14px + fade, 600ms; page change 10px/520ms (`HALO/03:7,11`; `H-CSS:23,25`) | CONFLICT (travel) | Two contexts: PAGE/CARD entrances (Home, Settings, Health) = Halo rise but 8–10px not 14; STREAM blocks keep 3px (follow-scroll chases growth). Never mix. |
| X37 | Page: enter 400ms, exit 140ms ("old page never ghosts") (`app.css:243-244`; `Daily/2026-07-16.md:21`) | Page change 520ms glide-up, no blur, sidebar stays; no exit (`HALO/03:11`) | PARTIAL | Halo `pageKey` glide for enter; keep the 140ms exit. |
| X38 | Skeleton sweep banned ("old school"); idle-cursor + ticking timer + breathing dot instead (`DESIGN.md:227-234`); AI Health skeleton + Workspace activity skeleton exist (`Daily/2026-07-16.md:21`, `07-10.md:112`) | Skeleton: bones with traveling `halo-shimmer` (1.4s linear infinite), fade-out + blur-sharpen handoff (~360ms) (`H-CSS:101-107`; `HALO/Skeleton/README.md:3`) | CONFLICT (hard) | Use Halo's skeleton HANDOFF (fade + sharpen) but replace the sweep with static bones (or very slow opacity breathing, compositor-only). Do not ship `halo-shimmer`. **[OWNER] Q8.** |
| X39 | Allowed loops: breathing live dot (`--pulse-live` 1.6s box-shadow), approved `.shim` accent sweep on single live VERB text (1.5s infinite, background-position), spinner (`stream.css:40-44,328-342`; `DESIGN.md:231-234`) | Loops only for real activity: `LiveDot` (transform/opacity ping 1.8s), `Spinner`, indeterminate `Progress` (`HALO/03:17`; `H-CSS:80-82,448-461`) | PARTIAL | Live dot: switch to Halo's transform/opacity ping (compositor) — Rift's box-shadow pulse repaints. `.shim` = owner-approved exception: KEEP, 1 element per view max. `WebBrowserPage.svelte:735` "accent comet sweeping the bar's bottom edge" (traveling light on chrome) → replace with indeterminate `Progress` or static bar. **[OWNER] Q8b** whether indeterminate-bar counts as the banned class. |
| X40 | Blur: no backdrop-filter on dialog scrim; JS "held" words unblurred (22727d24); but word reveal still blurs 3px × 500ms/word (`Markdown.svelte:692-695`); `backdrop-filter` in 23 files (grep: `app.css` + `lib`) incl. `SettingsMenu.svelte:519-520` blur(20px) | Blur only on small areas, one at a time: skeleton→content (4–6px), countdown, game glass; "a full-area blur costs frames" (`HALO/03:11,19`; `HALO/04:5`) | PARTIAL (Rift drifted from its own §10) | Policy: ZERO backdrop-filter in new build (opaque `.hud-float` recipe, surface 94% + `border-strong`). Per-word reveal: keep ≤3px blur OR opacity + 2px rise — the pending Reveal toggle (`redesign-plan.md:211`); decide by feel. Delete `AssistantWelcome.svelte:567-578` blurred aura (page-local wash, violates C7). **[OWNER] Q9.** |
| X41 | Prefers-reduced-motion honoured; `[data-slot]` animations dropped; caret exempt (`DESIGN.md:299`; `claude-code-ui-reference.md:68-74`) | Everything respects reduced-motion (`HALO/03:18`) | AGREE | — |
| X42 | Number change: token odometer `acRoll` with opacity floor 0.6 (blurred digits complaint) (`stream.css:36-39`; `Daily/2026-07-15.md:40`) | `RollingNumber` digit wheels right-most first; Stat count-up (`HALO/03:8`; `HALO/Stat/README.md:7-8`) | AGREE | Port as `RollingNumber`; keep floor 0.6 and the monotonic guard. |
| X43 | Toasts centered on main island (`--rail-w`) (`Daily/2026-07-16.md:19`); toasts z below dialog (`DESIGN.md:296`) | Bottom-right stack, fan-out on hover (pauses timers), swipe to dismiss (`HALO/Toast/README.md:3-8`) | PARTIAL | Adopt Halo stack mechanics; anchor choice **[OWNER] Q14**. |
| X44 | Tooltips never paint (bug) (`HANDOFF.md:40`) | `Tooltip` 120ms delay, inverted bubble, `aria-describedby` (`HALO/Tooltip/README.md:3-7`) | Halo SOLVES a Rift defect | Port Halo Tooltip; retire `actions/tooltip.ts` path. |
| X45 | Hover lift / FocusGroup dim / Button magnetic: no Rift stance; kbd-first power-user tool | `magnetic`, `FocusGroup dim` opt-in, "one hero moment per page" (`HALO/README.md:11`) | SILENT in Rift | Omit both. Fits the built-then-rejected lesson (§1.8 #2). |
| X46 | AppShell: floating island sidebar w/ hover mini-peek, pin, collapsed topbar cluster; `position:fixed; inset:0` mandatory (`Daily/2026-07-15.md:13,21`; `reference_tauri_svelte_frontend.md:39-60`) | AppShell: sidebar + sticky topbar, 72px icon-rail collapse, container-query reflow, `pageKey` (`HALO/AppShell/README.md:3-9`) | CONFLICT (structure) | **[OWNER] Q19.** Recommended: Rift shell behaviour wins (owner iterated it over 3 sessions); borrow Halo NavItem anatomy (icon 18, active on `bg-200` w/ accent icon, mono count) and pageKey glide. Do NOT use Halo AppShell as the shell (Tauri constraints: fixed positioning, drag-region capability grants, `data-tauri-drag-region`). |
| X47 | Logo: Rift mark + splash seam-parting (`Daily/2026-07-23.md:127-129`; `Daily/2026-06-09.md:278`) | Halo `Logo` = solid core + two rings, takes accent; "never glow/recolour" (`HALO/05:16`) | CONFLICT (brand) | **[OWNER] Q18.** Keep Rift mark; Halo says nothing about replacing another product's logo. Splash seam is the brand motif; don't regress. |
| X48 | Halo page-fit: dashboards/settings fit ONE screen, lists scroll in-card (`HALO/02:22-31`) | Rift: welcome no-scroll (`AssistantWelcome.svelte:277`); Settings scroll unknown | PARTIAL | Apply Halo fit-one-screen to Home/AI Health; chat + Settings tabs scroll in their own region. **[OWNER] Q15.** |

## 3. Motion likes / dislikes (specifically)

### 3.1 Dislikes / rejections (ordered by strength of evidence)

| ID | Item | Evidence (quote ≤15w) | Source | Date | Status |
|---|---|---|---|---|---|
| M1 | Blinking stream caret | "absolutely hate it" | `Daily/2026-07-02.md:107` | 2026-07-02 | DELETED |
| M2 | Traveling skeleton/loading sweep | "that little loading blurry block… looks old school" | `DESIGN.md:227-231` | 2026-07-21 | BANNED (block/row scope) |
| M3 | Orbiting light around container (composer comet) | "I like it but it's disturbing… too distracting" | `DESIGN.md:245-251` | 2026-07-22 | BANNED |
| M4 | Traveling glint on live rail | assistant-extended §8 (no owner quote) | `stream.css:65-68`; `22727d24` | 2026-10-19 | REMOVED; owner not yet asked |
| M5 | Streaming "a little laggy, could be better" = deliberate pacing (reveal + scroll lerp), NOT frame drops (p95 5.6ms) | `redesign-plan.md:178,181-195` | 2026-10-19 | pacing CHANGED, verdict PENDING |
| M6 | Content "lurching downward" while follow-scroll chases 8px block rises | `stream.css:25-26` | 2026-10-19 | 8→3px |
| M7 | Streaming screen-recording "wonky": ~3 of 6.5 min visually frozen, token counter ticking backward, status flashes, blurred digits mid-roll | `Daily/2026-07-06.md:88`; `Daily/2026-07-15.md:39-40`; `stream.css:36-37` | 2026-07-06/15 | DONE |
| M8 | Empty gray code chips racing ahead of word reveal (chrome painting before text) → chrome-hold | `Daily/2026-07-02.md:59` | 2026-07-02 | DONE (CSS `:has()`) |
| M9 | Mic halo pulse / busy STT display | "calm-down" (summary) | `Daily/2026-07-15.md:12` | 2026-07-15 | DONE |
| M10 | Splash "funky" / "needs a whole rework"; fake progress bar killed; burst/flare/letter-cascade removed for "one continuous motion… quality over quantity" | `Daily/2026-06-09.md:278,282`; `Daily/2026-07-09.md:63`; `Daily/2026-07-23.md:127` | 2026-06..07 | seam-parting current |
| M11 | Hover-peek flyout (jarring full-sidebar overlay) | "full sidebar overlapping everything looks off" | `Daily/2026-07-15.md:21` | 2026-07-15 | DONE (mini peek) |
| M12 | Stock/ambient decoration "being extra" (12 textures) | `Daily/2026-07-11.md:14` | 2026-07-11 | REMOVED |
| M13 | Per-word 320ms spring on footer verb swap; cluster spring-in; dot pop on live dot | removed in `22727d24` (assistant, "calmer live turn") | commit msg | 2026-10-19 | owner verdict PENDING |
| M14 | Unspecified: the "120/130/200ms crowd" hover timings left "a felt-motion design call… for owner" after token consolidation (#96); "hover-feel eyeball (revert-per-site)" requested | `Daily/2026-07-14.md:32,36` | 2026-07-14 | UNKNOWN verdict |
| M15 | Backdrop blur / full-area blur | no owner quote; WebView2 cost; plan item to drop | `DESIGN.md:297-299`; `redesign-plan.md:204-205` | 2026-09/10 | engineering rule, not taste |

### 3.2 Likes / explicit approvals

| ID | Item | Evidence | Source | Date |
|---|---|---|---|---|
| M16 | `.shim` accent sweep on a single live VERB (text only) | "owner-approved accent `.shim`" | `DESIGN.md:231-234` | 2026-07-21 |
| M17 | Breathing live dot; stationary signals (tinted border/halo, ticking timer, idle cursor) | "honest signals, no light show" | `DESIGN.md:230-234,251-252` | 2026-07-21/22 |
| M18 | Detent slider: free-follow drag, spring-snap on release ("slot-machine feel") — owner-specified spring | `SettingsMenu.svelte:205-208` | 2026-07-22 |
| M19 | Composer FLIP descent 460ms, controls materialize mid-flight, blur reverses; home launcher | `Daily/2026-07-14.md:98` | 2026-07-14 |
| M20 | Paced word reveal (mock `splitReveal`) — came from an approved design mock; owner's complaint was LAG not the idea | `Markdown.svelte:160-165`; `redesign-plan.md:178-189` | 2026-06→10 |
| M21 | Animated token odometer (live-turn polish, shipped on owner request) | `Daily/2026-06-21.md:189` | 2026-06-21 |
| M22 | Seam-parting splash and `.rift-seam` motif (owner calls: "needs a whole rework" → delivered, verdict pending) | `Daily/2026-07-23.md:127-133` | 2026-07-23 |
| M23 | HUD boundary polish: 350ms debounced appear, instant yield, 160ms fade-out; Plan HUD all-done 4s linger | `Daily/2026-07-08.md:33`; `Daily/2026-07-02.md:106` | 2026-07-02/08 |
| M24 | Orientation-first doctrine: "Motion exists for orientation, not delight. One orchestrated entrance beats scattered effects." | `DESIGN.md:100-102`; `claude-code-ui-reference.md:245-248` | 2026-07 / 2026-06-30 |

### 3.3 Net motion stance to design to

- Keep: orientation motion, stationary liveness, one-shot entrances, compositor-only continuous motion (opacity/transform), reduced-motion fallbacks.
- Halo agrees on most of it ("Motion with manners", loops only for real activity, `HALO/README.md:10`; `HALO/03:17-19`). The real collision is Halo's DELIGHT layer: spotlight, count-up on every change, FlipList glides, magnetic, FocusGroup dim, skeleton shimmer, staggered 14px rises, bell swing. Gate each by "orientation or delight?" and by the owner's built-then-rejected history (§1.8 #2).
- Pacing numbers are a feel call, not a spec: every number goes through a felt toggle in the dev app ("Say 'measured in dev'", `redesign-plan.md:214-215`). The release build is faster than dev.

## 4. Things the owner built and is proud of — must survive the Halo port

Evidence of pride = repeated iteration sessions, explicit "owner call", or shipped as a named release. Port target: Svelte components/behaviour, not just look.

| # | Feature / interaction | Why it matters (owner signal) | Source |
|---|---|---|---|
| S1 | Block-ordered stream: tool calls inline in arrival order; Tile family (Card/Row/Prompt/Float); mini-terminal shell IN/OUT; Shiki diffs w/ unified view + `SMALL_DIFF` auto-expand; ANSI colour; head+tail output folding; honest exit receipt; clickable `path:line` grep rows; test/lint → pass/fail pills | "the transcript IS the reasoning trace"; detail-first | `claude-code-ui-reference.md:38-47,153-165`; `Daily/2026-07-11.md:39`; `DESIGN.md:119-128` |
| S2 | Ledger turn: answer first, one ruled receipt line | owner pick | `redesign-plan.md:135-139`; `8bb71f4f` |
| S3 | Paced word reveal + chrome-hold + progressive markdown | owner-screenshotted bugs, long iteration | `Markdown.svelte:160-195`; `Daily/2026-07-02.md:59-62` |
| S4 | Floating island shell, recessed chassis, `.rift-seam`, mini peek sidebar, pin, collapsed topbar cluster | 5+ owner calls 07-15..07-23 | `DESIGN.md:64-96`; `Daily/2026-07-15.md:13,21` |
| S5 | Composer: 3-layer stack, home-launcher FLIP, send button morph (accent/stop/queue), QueueRail, enhance wand, slash/mention menus, dictation (VAD segments, ghost text, spring), detent effort slider, Fast-mode amber disclosure | "revamp the composer, it feels outdated" → iterated live 4× | `Daily/2026-07-14.md:96-98`; `Daily/2026-07-15.md:28-34`; `SettingsMenu.svelte:443` |
| S6 | Pinned Plan HUD + Agent HUD with visibility-aware yield | owner asks ("agents get left behind", "feels duplicated") | `Daily/2026-07-02.md:106`; `Daily/2026-07-08.md:28-33` |
| S7 | Status bar: plan-limits popover (5h/7d), branch chip + live CI dot + PR popover; honest footer (verb + elapsed + tokens) | shipped arcs 07-02, 07-16 | `Daily/2026-07-02.md:16`; `Daily/2026-07-16.md:52-56` |
| S8 | AI Health dashboard (range picker, spend-by-model, latency per model, plan pace forecast) | "rework AI Health — looks, accuracy, features" | `Daily/2026-07-10.md:75-86` |
| S9 | User-owned theming and prefs: 8 accent hues + vividness, density (compact default), `uiScale`, narration, toolDetail, commandOutput, code font prefs; dyslexia mode (Lexend, line-height, warm tint) | built features, persisted | `ui-prefs.svelte.ts:82-114`; `accessibility.svelte.ts:1-30` |
| S10 | Primitives layer (bits-ui keyboard/focus/dismiss), command palette, `/mcp` dialog, context menu with fade-out | v0.160.0 "steadier menus" | `CHANGELOG.md:13-25` |
| S11 | Split panes + pane identity | owner ask 06-14 | `Daily/2026-06-14.md:154` |
| S12 | Cost honesty (amber pay-per-use at the control) | born from a real incident | `DESIGN.md:219-225` |
| S13 | Splash with REAL boot stages (no fake bar) and onboarding with looping product miniatures | principle: honest signals | `Daily/2026-07-09.md:63-64`; `SplashOverlay.svelte:223-225` |
| S14 | Guardrails: `check-tokens.mjs` drift guard (`styles.css` + `_ds_manifest.json` lockstep), `check-ui.mjs` ratchet, Rift Style DS artifact, `/dev/ui` + `/dev/gallery` catalogs | the migration must update ALL of these or `npm run check` fails | `DESIGN.md:3-6,300-303`; `redesign-plan.md:142-148`; `HANDOFF.md:128-137` |
| S15 | Tauri shell constraints (not taste but fatal if lost): `position:fixed; inset:0` shell, `data-tauri-drag-region` + capability grants, no percentage-height chains | prod-only failures | `reference_tauri_svelte_frontend.md:16-60` |

Process preferences that shape HOW the redesign ships (not visuals): dev-server iteration, no per-change full build, release only when a big chunk is done (`feedback_rift.md:14-16`); HANDOFF/CHANGELOG ≤600 words, trim as you add (`feedback_rift.md:18-19`); each phase own branch/commit(s), no push/merge without asking (`redesign-plan.md:176`); plan/verify via `scripts/cdp/c.sh look`, named-lens critique, rejection → DESIGN.md §8 same session (`DESIGN.md:253-265`); code-signing permanently off (`feedback_rift.md:21-22`); explain to owner plain-language, short paragraphs, ADHD-readable (`feedback_ux_preferences.md:10-20`).

## 5. Questions only the owner can answer

Ordered by blocking power. "Default if unanswered" = what the plan should assume and flag.

| Q | Question | Why only he can answer | Default if unanswered |
|---|---|---|---|
| Q1 | Keep the onyx slab material (bevel, contact AO, grain, recessed chassis, `.rift-seam`) on Halo, or go Halo-flat (`bg-000/100` steps + hairlines only)? | He called the slab/chassis on 5 separate days; Halo says "no colour washes" | Keep structure + hairlines + seam, drop gradient fill/grain; show side-by-side before committing (rule C6) |
| Q2 | Default accent becomes Electric blue `#6aa8ff`? Does the 8-hue + vividness picker survive (re-anchored on Halo tokens)? | Emerald default + picker are his | Default blue, picker survives |
| Q3 | Halo ships 4 light themes — in scope or dark only? | `project_rift_tauri.md:14` says "Dark direction only" | Dark only |
| Q4 | Geist/Geist Mono replace Inter/JetBrains Mono? Keep Lexend for the greeting (brand) and dyslexia mode? Code-font pref stays? | Font identity; dyslexia mode is a stated accessibility feature | Geist + Geist Mono; Lexend only for dyslexia mode |
| Q5 | Density: keep compact (28px rows, 13px body, 26px buttons) as default and treat Halo's 44px/15px as the comfy tier? | He set "denser and quieter" as the default posture | Compact default |
| Q6 | Syntax colour in diffs/Read/shell: keep (muted, chart-derived) or go monochrome like Halo CodeBlock? | Directly trades his "detail-first" against Halo's one-signal rule | Keep, muted |
| Q7 | Card spotlight / glow-strong / press scale: try on which cards? Spotlight on ≤3 hero cards only? Reject `magnetic` + `FocusGroup dim`? | Motion = most-rejected category; he must feel it | Spotlight OFF; glow on primary only; magnetic/dim omitted |
| Q8 | Skeleton shimmer is a traveling light-band (banned 2026-07-21). Static bones + fade/sharpen handoff OK? Does an indeterminate Progress bar (and the WebBrowser loading comet) fall in the ban? | Scope of his own ban | Static bones; comet replaced by static/indeterminate bar, flag |
| Q9 | Verdict on the 22727d24 feel (30ms/word, catch-up 320ms, lerp 0.35, 3px entrances)? Word reveal: keep ≤3px blur or opacity + 2px rise? Follow: lerp or instant pin while streaming? | Only felt in the live dev app; he asked for toggles one at a time | Keep current numbers, mark "owner has not signed off" |
| Q10 | Pills: badges/status = pill (Halo); inline code + ctx chips stay rectangular; `.jump-latest` stays pill. OK? | Reconciles his "no pill buttons" with Halo's pill badges | As stated |
| Q11 | Per-project hues and the multi-hue model-mix bar: keep, or neutral initials + chart-1..4? | He accepted hues once, audit flagged them | Neutral initials; model-mix = chart series |
| Q12 | Allow numbered `01 / OVERVIEW` SectionLabels (accent index?) on page screens and a 30px `metric` numeral inside Stat cards, despite "oversized hero numerals"? | Collides with two of his rules | Numbers yes, index `ink-faint`, metric 30px cap |
| Q13 | Durations: Rift-tuned (150/240/460) vs Halo (150/300/600) for chat and overlays? | "Laggy" is his word; Halo is slower | Rift-tuned for chat/overlays; Halo timings for dashboard page entrances only |
| Q14 | Toast anchor: bottom-right Halo stack or centered on the main island (07-16 choice)? | Placement taste | Halo stack mechanics, centered-on-island anchor |
| Q15 | Hover-feel timings after the #96 token consolidation (120/130/200ms crowd): any sites to revert? Halo page-fit rule ("dashboards fit one screen") for Settings? | Never answered (`Daily/2026-07-14.md:32,36`) | Treat as accepted |
| Q16 | Allow ONE short billed real turn to measure scroll-follow lag (gallery can't exercise `pinToBottom`)? | Costs money; plan said ask first (`redesign-plan.md:192-194`) | Don't run |
| Q17 | Retire `design-system/` static cards, and supersede the Rift Style DS artifact by Halo? Is the Halo DS the new single source of truth (DESIGN.md §8 rejection log kept)? | His standing flag (`redesign-plan.md:221-222`) | Flag only, delete nothing |
| Q18 | Logo/splash: keep Rift mark + seam-parting splash under Halo? | Brand | Keep |
| Q19 | Shell: keep floating-island sidebar with hover mini-peek/pin/collapse as built, borrowing only Halo's NavItem anatomy + page glide (not Halo AppShell)? | He iterated this over three sessions | Keep Rift shell |
| Q20 | Halo's "positive change = accent" vs Rift green `ok` for pass/success glyphs in the transcript? | Changes the meaning of colour he trained himself on | Keep green for outcomes in transcript; blue positives on dashboards |
| Q21 | Which parts of the first Halo mock felt "rough" (layout, density, type, motion, the chrome)? | Only he knows; mock file not diffed in this wave | UNKNOWN |
| Q22 | Does the 2026-10-19 brief ("not a layout overhaul") still apply to Chat (Ledger stays) while Home/Settings/Health get full Halo layouts? | Scope of "redesign" | Ledger stays; other screens free |

## 6. Known drift between doctrine and code (flag only, nothing edited)

| # | Drift | Cite |
|---|---|---|
| F1 | `backdrop-filter` in 23 files despite blur policy (incl. `blur(20px)` in `SettingsMenu`) | grep `backdrop-filter`; `SettingsMenu.svelte:519-520` |
| F2 | Welcome hero blurred accent aura + blurred seam = page-local atmosphere | `AssistantWelcome.svelte:567-578` vs `:462-463` |
| F3 | `WebBrowserPage` loading "comet" is traveling light on chrome | `WebBrowserPage.svelte:735` |
| F4 | `WorkspacePage` active-card gradient wash (accent 5%) | `WorkspacePage.svelte:1118-1119`; audit flagged amber wash at `redesign-plan.md:71-75` |
| F5 | `.sturn-head` / live dot use `--ease-spring` + box-shadow pulse (non-compositor) | `stream.css:40-44,77-82` |
| F6 | Tooltips never paint | `HANDOFF.md:40` |
| F7 | `HANDOFF.md` / `redesign-plan.md` pacing notes stale vs `22727d24` | §0.2 |
| F8 | `.chip-btn` 999px | `redesign-plan.md:69-70` |
| F9 | 388 hard-coded `font-size`, 12 radii on Settings | `redesign-plan.md:80-83` |
