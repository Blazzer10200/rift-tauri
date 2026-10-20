---
title: Halo map 01 — app shell, frame, navigation chrome
status: map only (read-only wave); no source edited
branch: design/chat-ledger
updated: 2026-10-19
---

# 01 · Shell map (Rift → Halo, Electric blue)

Scope: `app.html`, root layout, `AppShell`, `shell/*`, splash, toasts, tab/view switching, window chrome.
Path shorthand: `S/` = `src/lib/components/shell/`, `C/` = `src/lib/components/`, `ST/` = `src/lib/state/`. Halo refs: `HB/` = brand-book `project/`, `bundle.css:N` = `HB/components/bundle.css`, `halo-tokens.css` = `C:/AI Workflow/projects/consolex/panel/design-systems/halo/tokens.css`.
Not read this wave (unknown): `src/routes/+layout.ts`, `src/lib/state/ui-prefs.svelte.ts`, `design-system/DESIGN.md` (307 lines; only quoted via redesign-plan), `design-system/styles.css`, `ui/alert-dialog/*` internals beyond `recipes.ts`, `CommandPalette.svelte`, `UpdateDialog.svelte`, `McpServersDialog.svelte`, `OnboardingFlow.svelte`.

---

## 1 Files & roles

| File | Lines | Role |
|---|---|---|
| `src/app.html` | 22 | HTML shell. `<html data-direction="linear" data-mode="dark" data-density="compact">` (:2). Inline pre-JS bg `oklch(0.142 0.003 250)` (:9) + static `#first-paint` favicon at 45% opacity (:13-14,:19), removed in `+layout` onMount. `data-sveltekit-preload-data="hover"` (:18). |
| `src/routes/+layout.svelte` | 67 | Imports 3 font packages (Inter, JetBrains Mono, Lexend :2-4) + `app.css` (:5). Splash gate via `sessionStorage "rift.splash.seen"` (:38-41), arms `intro` on cold boot (:43). onMount: removes first-paint, `uiPrefs.init`, `workspace.init`, `projects.refresh`, `accessibility.init` (:45-55). Mounts global `ContextMenuHost` (:63) and `SplashOverlay` (:65-67). Dev-only Ctrl+Alt+G / Ctrl+Alt+U route to `/dev/gallery`, `/dev/ui` (:21-31, :59). |
| `src/routes/+page.svelte` | 5 | Renders `<AppShell />` only. Single real route; no router-driven views. |
| `C/AppShell.svelte` | 513 | The frame. Boot effects (:72-107, heartbeat 3s :100-102), CLI-update sync (:113-116), focus re-probe (:124-142), responsive rail (:155-169), window file-drop guard (:176-199), **all global shortcuts** (:201-301), template (:304-337), chassis CSS (:339-512). |
| `S/WorkspaceShell.svelte` | 212 | View host: lazy-loads + keeps every once-opened workspace mounted, cross-fades, staggered child rise, load-error + skeleton states. |
| `S/workspaceRegistry.ts` | 38 | `WORKSPACES` record: id → `{load(), title, icon, kbd, disabled?}`. `kbd` field is never read (grep: no `.kbd` consumer) and its values disagree with real Ctrl+N order. |
| `S/Sidebar.svelte` | 352 | Left "island": brand head, ProjectSwitcher, New chat + search, Conversations header, ConversationList, footer 5-tile nav. Collapse / peek / resize logic. Publishes `--rail-w` (:45-47). |
| `S/ConversationList.svelte` | 544 | The de-facto **tab list**. Time-bucket grouping, pinned, stable sort, working indicator, queued badge, elapsed timer, rename, row menu, drag-to-pane, "Show earlier", mini peek flavor, boot skeleton. |
| `S/ProjectSwitcher.svelte` | 286 | Project identity row + portaled dropdown (No project / projects / New project), project right-click menu, delete confirm, branch pill latch. |
| `S/Topbar.svelte` | 77 | 40px strip atop the main island: collapsed-rail pin/peek trigger (left), custom window controls (right). Whole bar is a drag region. |
| `S/Titlebar.svelte` | 76 | **Onboarding-only** 34px titlebar (brand + window controls). Duplicates Topbar's window-control logic. |
| `S/StatusBar.svelte` | 279 | Footer inside main island: provider connection, project/branch (+GitHub popover), AI-mistakes note, Split, Admin, date, 5h/7d usage pills + UsagePanel. |
| `S/NotificationCenter.svelte` | 421 | Bell tile (sidebar footer) + upward-opening portaled history panel. |
| `S/UpdateBanner.svelte` | 332 | Full-window strip above the body: Rift app update + 4 Claude-CLI states. |
| `S/CloseConfirm.svelte` | 170 | bits-ui AlertDialog: "Close Rift?" → 3-step verified close-out checklist (invokes `app_close_dismissed/_reap/_verify`, `app_exit_now`). |
| `S/DeleteProjectDialog.svelte` | 61 | AlertDialog confirm for removing a project (config-only). |
| `S/ContextMenuHost.svelte` | 103 | Single global bits-ui DropdownMenu anchored at a point; focus-restore logic; driven by `ST/contextMenu.svelte.ts` (212). |
| `S/Skeleton.svelte` | 57 | Shimmer block (transform sweep, accent-tinted, reduced-motion breathe). |
| `S/RiftLogo.svelte` | 29 | `<img>` of `$lib/assets/rift-logo.png` (dark squircle PNG). |
| `C/SplashOverlay.svelte` | 293 | Power-on veil: two chassis half-panels that split, lockup (mark + RIFT letters), late-disclosed boot readout. |
| `C/ToastHost.svelte` | 212 | Single stack renderer; top-center of main island. Store: `ST/toast.svelte.ts` (286; MAX_VISIBLE 3 :55, history cap 50, 7-day age-out, timeouts danger 8s / warn 6s / else 4s :57-63). |
| State used by shell | — | `ST/workspace.svelte.ts` (182), `ST/shell.svelte.ts` (207), `ST/intro.svelte.ts` (26), `ST/nav.ts` (12), `ST/bootLoad.svelte.ts` (25). |
| Tab state | — | `TabState` `ST/assistant.svelte.ts:207`; `openTabs` :971; `panes` :976; `currentConvoId` :969; `focusedPaneIdx` :977. |

---

## 2 Layout & anatomy

### Window and root
- Frameless window: `decorations:false` (`src-tauri/tauri.conf.json:20`), default 1600×1000, min 900×600 (:16-19). Capabilities grant start-dragging, minimize, toggle-maximize, close (`src-tauri/capabilities/default.json:11-15`). OS close is intercepted in Rust (`lib.rs:180-191`) and re-emitted as `rift://close-requested`, caught at `AppShell.svelte:73-75`, which opens CloseConfirm.
- Stack (back → front), with literal z-indexes: `.app` fixed inset 0 (`AppShell.svelte:344-346`) → `.app-body` z1 (:368) → `.side-rail` z30 (`Sidebar.svelte:218`) → toasts z2000 (`ToastHost.svelte:98`) → NotificationCenter panel z2100 (`NotificationCenter.svelte:270`) → conv menu z51 / project menu z52 (`ConversationList.svelte:509`, `ProjectSwitcher.svelte:264`) → dialogs `--z-dialog` 4000 / popovers `--z-popover` 9000 (`app.css:281-282`) → splash z9999 (`SplashOverlay.svelte:130`). Legacy `.dialog-overlay` z200 (`app.css:616`). Only 2 of ~9 layers use tokens; Halo says never raw numbers (`HB/02-type-space-and-layout.md`: layers).

### Normal mode (not onboarding), 1600×1000 reference
```
.app (flex column, chassis gradient)                       AppShell:343
├─ UpdateBanner (0px when empty; 38px/row)                 AppShell:313
└─ .app-body (flex row, 12px gutters, grain+cavity)        AppShell:314
   ├─ Sidebar .side-rail  width=248 (208–380; 0 collapsed) Sidebar:104
   │  └─ .sidebar island (abs; top/bottom/left 12px; w=rail-24)
   │     ├─ .side-head 40px: RiftLogo 22 + "Rift" 14px + collapse btn 28px   :123
   │     ├─ ProjectSwitcher 43px (28px monogram tile + name 13px + sub 10.5px)
   │     ├─ .action-row 36px: New chat (flex) + search icon 36px
   │     ├─ .history-head 28px: CONVERSATIONS + count pill
   │     ├─ ConversationList (flex 1, scroll; rows 34px)
   │     └─ .foot-nav: 5 equal tiles, 44px: Home · Chat · Health · Alerts · Settings
   ├─ .rift-seam: 0-width spacer, 1px accent gradient line in gutter  AppShell:449
   └─ .main island (margin 12px 12px 12px 0, radius 14)
      ├─ Topbar 40px: [pin trigger when collapsed] … [min/max/close 36×30]
      ├─ <main.workspace> → WorkspaceShell → .ws-page × N (absolute stack)
      └─ StatusBar footer, min-height 32px, hairline top
```
- Sidebar island is `position:absolute` inside a width-only spacer (`Sidebar.svelte:255-256`); island width = `shell.width - 24` (:118), because the 12px gutters sit inside the rail.
- Resize handle: 8px, `top:48px; right:4px` (`Sidebar.svelte:246`), `role=separator`, tabindex 0, arrow/Home/End keys (:84-95).
- Peek flyout (collapsed + hover on Topbar trigger): `.sidebar.mini` width 240 (`Sidebar.svelte:64`), `top:54px`, `max-height: calc(100% - 62px)` (:288), no head, no scope header, list capped to 8 rows + "Pin sidebar" seam (`ConversationList.svelte:20, 340-347`).
- Onboarding mode: `Titlebar` (34px, `--titlebar-h` `app.css:259`) + `.ob-host` full surface (`AppShell.svelte:305-309`); no sidebar/topbar/statusbar. Gate: `!onboarding.dismissed && configLoaded && (no provider auth || beta not acked)` (:48-52).
- Always mounted after body: UpdateDialog, CloseConfirm, ToastHost, CommandPalette, McpServersDialog (:329-336); ContextMenuHost + Splash from root layout.
- Material: onyx-slab islands = 2 vertical gradients + 4-layer box-shadow + SVG-noise grain `::after` (`AppShell:400-411, 422-431`; `Sidebar:263-284`); recessed chassis gradient (`AppShell:352-354`); grain on `.app-body::before` (:372-381); cavity inset shadows (`:441-443`: `inset 0 2px 22px` + `inset 0 0 70px`); accent seam (:456-471).

### How views switch (chat / home / settings / health / diagnostics / browser)
- State: `workspace.activeId` (`ST/workspace.svelte.ts:72`), type `WorkspaceId = home|chat|projects|settings|ai-health|diagnostics` (:5-6). Persisted `rift.ui.workspace.v1` (:20, :164). `projects` is a legacy alias folded to `home` (:89-91, :158). `order` (:73) = [home, chat, settings, ai-health, diagnostics] (:16-18).
- No URL routing: `+page.svelte` is the only app route; `setActive()` swaps what `WorkspaceShell` shows. Dev showrooms are the only other routes (`/dev/gallery`, `/dev/ui`).
- `WorkspaceShell` renders one `.ws-page` per id in `workspace.everOpened` (`WorkspaceShell.svelte:79-80`); inactive ones stay mounted at `opacity:0; pointer-events:none; transform:translateY(3px)` plus `inert` + `aria-hidden` (:84-91, :128-139). Components lazy-imported on first open (`ensureLoaded` :14-34).
- Entry points: footer nav tiles (`Sidebar.svelte:165-194`), Ctrl+1…9 by `workspace.order` index (`AppShell:284-291`), Ctrl+0 → chat (:277-281), Ctrl+, → settings (:298), palette (Ctrl+P/K :300), StatusBar chips (`StatusBar:104-109, 143`), `goHome()` verb (`ST/nav.ts:8-12`: go to Chat, mint new tab if current has messages).
- **Diagnostics has no nav tile**: filtered out of footer (`Sidebar.svelte:24`); reachable by palette / Settings→System / Ctrl+5.
- **"Browser" is not a view**: it is a right-docked panel inside Chat (`C/assistant/AssistantPage.svelte:301-328`, `WebBrowserPage`), toggled by `browserDock` (Ctrl+Shift+B `AppShell:275`, Ctrl+L :294, Ctrl+F :297), width-animated via custom `dockSlide` (220ms `cubicOut`, `AssistantPage:15-22`). Out of this map's styling scope but it consumes shell width (`shell.maxPanesForWidth` `ST/shell.svelte.ts:188-194`).

### How tabs are presented (TabState)
- **There is no tab strip in the DOM today.** `role="tab"` / `tablist` appear only in `SettingsMenu.svelte` and `SettingsPage.svelte` (grep). `tabsbar` survives only in comments (`AssistantPane.svelte:410,414`) and a stale doc-comment (`assistant.svelte.ts:317` "tabs bar badges…").
- A tab = one `TabState` per conversation (`assistant.svelte.ts:207-`), keyed by convoId; `openTabs` is the ordered id list (:971); each pane holds a `tabId` (:976); `currentConvoId` mirrors the focused pane (:969-975).
- Presentation = **sidebar ConversationList rows** (click → `workspace.setActive("chat")` + `assistant.openTab(id)` `ConversationList.svelte:183-187`), live state per row (`assistant.liveTabs`), plus keyboard (Alt+1…9 jump `AppShell:213-220`, Ctrl+T/W/Tab `:243-258`).
- Split mode (≤`MAX_PANES`): each pane gets a 28px `.pane-head` (index chip, title, folder chip, working/done, ctx chip, maximize, close) only when `splitActive` (`AssistantPane.svelte:585-` , CSS :925-952); pane limit by width `shell.maxPanesForWidth` (360px/pane). StatusBar "Split" chip is the discoverable entry (`StatusBar:172-186`); Alt+←/→, Alt+Enter, Ctrl+\ (`AppShell:223-268`).
- Drag: row → pane (`ConversationList:193-204` sets `assistant.draggingTabId`), project → pane (`ProjectSwitcher:106-114`).
- Halo's first chat mock *added* a `Tabs` strip for open chats in a 48px header (`halo-canvas/project/Main.dc.html:60-61`). That is a new surface for Rift (see Q1).

### Window chrome and drag regions
- Custom controls: Topbar 36×30 buttons, CSS-drawn min/max/restore glyphs, lucide `X` (`Topbar.svelte:45-49, 67-76`); Titlebar variant 38px-wide with lucide icons (`Titlebar.svelte:30-40`). Both track `isMaximized` via `win.onResized` (:14-18 / :15-21) — duplicated logic.
- `data-tauri-drag-region` on: Topbar root (`Topbar:22`), Sidebar head (`Sidebar:123`), **StatusBar footer** (`StatusBar:123`), Titlebar + brand (`Titlebar:24-25`), Splash (`Splash:95`). `-webkit-app-region: drag/no-drag` is also set in CSS (`Sidebar:246, 289, 291`; `Topbar:58`; `StatusBar:234, 261`). Whether the CSS property has any effect in Tauri/WebView2 is **unknown** (not tested here); Tauri's documented mechanism is the attribute.
- Not verified: double-click-to-maximize on drag regions; Win11 snap-layout flyout over a custom maximize button (a known WebView2/frameless gap — would need a live test).

---

## 3 States

| Surface | States found (with cite) |
|---|---|
| Boot | `#first-paint` static mark (`app.html:19`) → splash veil opaque until `configLoaded && workspaceReady` or 10s cap (`Splash:18-19, 33-34`); min hold 900ms; slow-boot readout after 1400ms (:22, :87). `bootLoad.showSkeleton` = `!configLoaded` after 120ms grace (`ST/bootLoad.svelte.ts:7-22`) → Skeletons in ProjectSwitcher (`:146-153`), ConversationList (`:323-331`), StatusBar (`:124-127`). |
| Sidebar | expanded · collapsed (island `translateX(-16px)` opacity 0 `inert`, `Sidebar:223`) · peeking (floating, shadowed `:227-232`) · mini flavor · resizing (transitions off `:222`) · narrow auto-collapse ≤1100 / reopen ≥1180 (`shell.svelte.ts:26-27, 169-182`) · persisted width/collapse/pins/scope/history-expanded (`:7-12`). |
| ProjectSwitcher | "No project" (chat icon tile, "All chats · no folder") · project (hue monogram + branch pill) · "Unsaved folder" · "Local workspace" fallback · branch latched across surface changes (`:35-42`) · menu open (`aria-expanded`) · active ✓ row · delete-confirm. |
| ConversationList | empty ("No conversations yet" `:334`) · skeleton · mini · Pinned/Today flat/bucketed older behind "Show earlier" with batches of 40 (`:123-146`) · row: hover (time→actions swap), `.on` active (accent 3px bar), `.pinned` wash, `.menu-open`, `.pinned`, working (9-dot grid + live elapsed `m:ss`), queued badge, rename input, project tag in All scope, drag source. No disabled state. |
| Footer nav | default · hover (icon lifts 1px) · `.on` (accent text + 9% wash) · `:active` scale .96 · NotificationCenter tile hover/focus-within; unread badge `9+`. No focus-visible style defined on `.fnav-item` (grep of `Sidebar.svelte` CSS: none) — relies on UA/global. |
| Topbar | rail expanded (controls only) · rail collapsed (pin trigger fades in 240ms `:57`) · maximized vs restored glyph · `.tb-ic:disabled` style exists but no disabled button renders (:65). |
| StatusBar | skeleton · connected / offline dot (`:138-139`) · repo name / "Local" / "No folder" (`:35-39`) · branch static vs GH-popover button + CI dot (`:146-158`) · Split (hidden off-chat; disabled at pane limit `:174-186`) · Admin chip (elevated) · usage pills `data-zone` ok/warn/hot (`:264-265`) · UsagePanel open · responsive hides: note ≤920, date ≤720, branch ≤540 (`:266-277`) — **min window is 900px** (`tauri.conf.json:18`) so the ≤720/≤540 rules are unreachable at 100% UI scale (does `uiPrefs.stepUiScale` change `innerWidth`? unknown). |
| Workspace host | loading skeleton (`ws-load-skeleton` aria-busy `WorkspaceShell:103-107`) · load error + "Try again" (`:96-101`) · active/inactive pages · `.rising` transient (810ms). A `def.disabled` branch (:92-93) is dead: `DISABLED` is an empty Set (`workspace.svelte.ts:58`). |
| UpdateBanner | hidden (0 height) · app update available · downloading/installing with progress fill (`:162-164`) · CLI update / restart-ready / version-unreadable / check-failing (`:71-155`) · two rows stacked 1px apart · per-row dismiss (snooze). CLI rows suppressed when ChatGPT route active (`:44`). |
| Toast | 5 severities (ok/warn/danger/info/muted) · plain `status` vs clickable `button` role · sticky · hover pauses timer (`:50-51`) · detail clamped to 2 lines, click un-clamps (`:22-31, 160-174`) · close only visible on hover/focus (`:199-203`) · up to 3 visible (`toast.svelte.ts:55`). |
| NotificationCenter | closed · open · empty ("You're all caught up" `:156-161`) · groups Now/Past hour/Today/Older · unread (full dot + brighter title) vs read · ×N count chip · row action button · hover swaps time→close (`:375, :403`) · header buttons disabled when nothing to mark/clear (`:136, 146`). |
| CloseConfirm | confirm · closing with 3 steps pending/running/done/warn + notes (`:15-26, 139-162`); Esc/outside ignored while closing (`:114-115`). |
| ContextMenuHost | open at (x,y) · closes on window blur/resize (`:25-31`) · focus restored to invoker (`:43-51`). |
| Reduced motion | per-component `@media` blocks (see §4). OS `prefers-reduced-motion` only; JS probe `prefersReducedMotion()` is a **one-shot, non-reactive** `matchMedia` read at component init (`ST/accessibility.svelte.ts:103-106`), used by Splash, NotificationCenter, ToastHost, UpdateBanner, intro. |
| Narrow window | auto-collapse rail at ≤1100 (hysteresis 1180) `shell.svelte.ts:26-27`; panes limited by 360px each; no Halo-style "<760 sidebar becomes top strip" behavior (min window 900 anyway). |

---

## 4 Motion present

Rift tokens: `--dur-fast 140 / base 240 / slow 520 / page 400 / page-out 140 / rise 460 / stagger 62`, eases `page (.22,1,.36,1)`, `soft (.4,0,.2,1)`, `spring (.34,1.56,.64,1)` (`app.css:240-250`). Halo: `fast 150 / base 300 / slow 600 / stagger 60`, `ease-out (.2,.8,.2,1)`, `ease-spring` same as Rift (`halo-tokens.css`; `HB/03-motion.md`). Halo page change = 520ms ease-out, 10px, 60ms delay, **no blur** (`bundle.css:24-25`); entrance rise = 14px at `dur-slow`+stagger cap ≈ a dozen (`:22-23`, `03-motion.md`); loops only for real activity; only opacity/transform continuous; blur only small areas.

Verdict key: KEEP · RETUNE (change numbers/curve/property) · KILL · GAP (Halo pattern absent in Rift).

| Cite | What moves · property | Duration · easing · trigger | Verdict vs Halo |
|---|---|---|---|
| `Splash:149,165-166` | two half-panels `transform: translateX(±101%)` | 520ms `cubic-bezier(.65,0,.35,1)` +140ms delay · on exit | **KEEP as Rift-specific** (no Halo splash). Retune curve to Halo `ease-out` for consistency or keep in-out as an intentional "doors"; hard-coded bezier, not a token (also :177-179,:189,:215,:236,:269). |
| `Splash:189-194` | mark: opacity, translateY 8, scale .94, **blur 8→0** | 560ms `(.22,1,.36,1)` +140ms · mount | RETUNE: Halo allows blur on small areas (skeleton reveal 6px, `bundle.css:104`); 64px mark OK. Use `dur-slow` token. |
| `Splash:213-221` | RIFT letters opacity/translateY 10/**blur 4→0**, stagger 55ms from +300ms | 460ms · mount | KEEP (brand moment), blur is 4 small elements; stagger 55 → Halo 60. |
| `Splash:177-184` | lockup exit opacity→0 + blur 3 | 240/280ms · exit | RETUNE: drop blur on exit (Halo: no blur on exits; page change explicitly "no blur"). |
| `Splash:226-243` | boot readout fade/translate 4 | 400ms +80ms | KEEP. |
| `Splash:268-280` | stage line rise 260ms; **caret `_` blink 1.1s steps infinite** | loop | KILL caret blink: decorative loop (Halo: loops only for real activity; DESIGN §8 cited in comment `:224-225` also bans glow/shimmer). Keep stage-in. |
| `Splash:113` + `:47-63` | progress `scaleX` driven by rAF ease + slow creep to 96% | per frame | KEEP (compositor-safe transform); honest stages. Note: estimated pacing, labeled in comment :43-45. |
| `Splash:79-80, 18-22` | exit timing 680ms (120 reduced); MIN 900 / CAP 10000 / SLOW 1400 | JS timers | KEEP; magic numbers should reference one choreography constant. |
| `ST/intro.svelte.ts:11-23`, `AppShell:493-501` | `.main` opacity 0→1, translateY 12→0; delay 60ms | 460ms `ease-page` · handoff phase (80ms after exit starts; settles +700ms) | RETUNE to Halo page-glide values (10px, 520ms, 60ms delay). Choreography with sidebar (below). |
| `Sidebar:238-243` | island `translateX(-16px)`+opacity during veil → none | 460ms `ease-page` | KEEP concept (assemble under the doors). Halo AppShell has no such intro: Rift-specific. |
| `AppShell:483-486` | `.workspace` `translateY(7px)→0` (`wsIn`) | 400ms `ease-page` · **first mount only** (element persists) | KILL: redundant with `ws-page` transition + `.rising`; suppressed on cold boot (`:493`). |
| `WorkspaceShell:128-148` | `.ws-page` opacity + translateY(3px) | enter 400ms `ease-page`, leave 140ms `ease-soft` · `data-active` flip | RETUNE → Halo page change: one glide, 520ms `ease-out`, 10px, 60ms delay, entering page only; leaving page instant-hide (Halo's `pageKey` swaps content). Note enter(400) and leave(140) overlap → brief double-exposure (both visible t<140ms). |
| `WorkspaceShell:198-211, 47-75` | `.rising > *` children `ws-child-rise` opacity/translateY 6, nth-child 1–5 stagger 62ms | 460ms · transient class 810ms (**JS constant `SETTLE_MS=810` :48 duplicates CSS numbers**) | KEEP idea (Halo stagger entrance) but retune to `halo-rise` (14px, `dur-slow`, 60ms, cap ~12) and derive timing from one source; third stacked page-motion layer today (wsIn + ws-page + rising). |
| `WorkspaceShell:164-166` | skeleton `background-position` shimmer | **1.4s ease-in-out infinite** | RETUNE: animates `background-position` (not compositor-only; Halo: opacity/transform only). Replace with the Skeleton.svelte transform sweep or Halo `halo-shimmer` (1.4s linear `translateX`, `bundle.css:101-102`). Two skeleton implementations coexist. |
| `Skeleton:42-48` | sweep `translateX(-100%→100%)`, gradient with **accent 14%** center | 1.6s `ease-soft` infinite, per-instance delay | RETUNE: Halo bar = `bg-200`, sheen `bg-300`, linear, 1.4s, no accent tint (`bundle.css:100-102`). Keep stagger-phase idea. Add Halo crossfade (skeleton out 360ms + blur 4; content in 600ms blur 6→0, `:104-107`) = GAP. |
| `Sidebar:218-221` | rail `width` (layout prop) | `dur-base` 240ms `ease-soft` | KEEP approach — Halo also transitions `width/flex-basis` at `dur-base` (`bundle.css:703`). Code comment (:219-220) records that long durations read as lag; stay ≤300. |
| `Sidebar:274` | island `transform`, `top`, `opacity`, `box-shadow` | 240ms soft/page · collapse/peek | RETUNE ease to single `ease-out`; **peek animates `top` (layout prop)** (:228, :274) — convert to transform. |
| `AppShell:417` | `.main` `margin-left` | 240ms soft · collapse | KEEP (lockstep with rail, comment :415-416). |
| `app.css:347-354` | `--rail-w` registered `<length>` tween on `<html>` | **140ms** `ease-soft` (uses `--dur-page-out`) | **BUG-LEVEL RETUNE**: rail width animates 240ms (`Sidebar:221`) but the toast anchor tweens 140ms → toast stack arrives early then waits; match to 240/300ms. |
| `Sidebar:300,328,326,341` | nav icon `translateY(-1px)` on hover; `.fnav-item:active scale(.96)`; new-chat `translateY(1px)` active | 140ms | RETUNE: Halo hover = color/bg only on nav (`bundle.css:270`), press = `scale .97` (`:54`); drop the icon lift or keep as subtle "answer the cursor". |
| `Sidebar:247-250` | resize handle line bg | 140ms | KEEP. |
| `ConversationList:479,525` | every row `rowIn` opacity/translateY 3, stagger 14ms (cap 12) | 220ms `ease-page` · **every row mount** (incl. `{#key showAll}` remount `:352`, "Show earlier" expand) | RETUNE: Halo menu-item rise = 220ms / 18ms stagger / cap 12 (`bundle.css:419-420`) — nearly identical; but Halo scopes it to menus, not persistent nav lists. Decide whether history rows should rise on every remount. |
| `ConversationList:484,523` | active bar `scaleY(.25→1)` | 280ms `ease-page` | RETUNE → Halo has no sliding nav indicator for NavItem; Tabs underline *slides* (`bundle.css:266`, `dur-base`). Candidate: one sliding active marker across rows (GAP) instead of per-row pop. |
| `ConversationList:502,527` | hover actions `display:none→flex` + `actsIn` translateX(5) opacity | 140ms | KEEP idea; display-swap retrigger is fragile; Halo `halo-notif__x` uses opacity transition (`bundle.css:382-383`). |
| `ConversationList:423-424` | "Show earlier" chevron rotate 180° | 220ms `ease-page` | RETUNE to `dur-base` + `ease-out` (Halo `halo-menu__chev` 300ms, `bundle.css:416`). |
| `ConversationList:533-543` | **working dot-grid: 9 `<i>` scale/opacity, infinite, diagonal wave** | 1.05s `ease-soft` infinite per working row | KEEP semantics (real activity = allowed loop) but retune visual to Halo `LiveDot` (ping 1.8s, `bundle.css:75-82`) or keep as Rift signature; perf: 9 animated nodes × N working chats. |
| `ConversationList:515,530`; `ProjectSwitcher:269,284` | popover `scale(.97) translateY(-2/-4)` | 140ms `ease-page` | RETUNE → Halo popup `halo-pop-in` `scale(.96) translateY(-4px)` (`bundle.css:411`) at `dur-base`; both menus bypass the ui/ DropdownMenu recipe. |
| `ConversationList:470-475`; `ProjectSwitcher:238-241` | skeleton container fade-in | 240ms | KEEP. |
| `NotificationCenter:125` | panel Svelte `fly y:6` | 160ms (0 when reduced at init) | RETUNE → GAP: Halo = right-side drawer, 40px slide, `dur-base`, scrim, rows rise (50ms stagger cap 8), dismissed rows slide out + collapse (`bundle.css:358-383`). Current = small popover opening upward. |
| `NotificationCenter:113,170` | badge/row `fade` | 120ms | RETUNE → Halo badge spring pop (`bundle.css:356`, `halo-pop` w/ `ease-spring`) + bell swing 900ms + ping once on count increase (`:353-357`) = GAP. |
| `UpdateBanner:159` | Svelte `slide` of whole host (**reflows entire app body**) | 200ms (0 reduced) | RETUNE: Halo banner = rise at `dur-base`, leave fade + translateY(-6) 200ms (`bundle.css:387-391`); reflow of full window on update arrival is the real cost (see §8). |
| `UpdateBanner:238, 258` | progress fill `width` transition; spinner rotate | `dur-base ease-out`; **0.8s linear infinite** | RETUNE: width transition → `scaleX` (Halo `Progress`); spinner 900ms (`bundle.css:448`). |
| `ToastHost:53-54,52` | in `fly y:-10` 220ms · out `fly y:-6` 140ms · `flip` 180ms | Svelte transitions · push/dismiss | RETUNE/GAP: Halo = bottom-right stack, `translateY(16) scale(.96)` in at `dur-base`, collapsed stack that **fans on hover and pauses**, **drag-right-to-dismiss** with spring-back, leave slides right 32px/240ms (`bundle.css:314-324`). Rift pauses on hover already (`ToastHost:50-51`) but doesn't fan or swipe. |
| `ToastHost:130,133-136` | hover `translateY(1px)` + border tint | 140ms | KEEP/minor. |
| `CloseConfirm` via `ui/recipes.ts:58-60` | overlay fade + content `zoom-in-95` (tw-animate) | `--dur-fast` 140ms `ease-page` | RETUNE → Halo dialog `scale(.94) translateY(8)` at `dur-base`, leave `scale(.96)` 200ms (`bundle.css:285-287`). (Shared `ui/` recipe — owned by dialog map.) |
| `Topbar:57` | `.tb-left` enter opacity/translateY 4 | 240ms `ease-page` | KEEP. |
| `StatusBar:233-237` | `.sb-btn` hover | **no `transition` declared** → snaps | RETUNE: add `dur-fast` bg/color. |
| `app.css:676-677` | `.gh-dot.busy` opacity breathe (`--pulse-live` 1.6s) | infinite while CI pending | KEEP (real activity). |
| `AppShell:360` | `.app` `transition: background 1s` | 1s | KILL: gradients (`background-image`) are not interpolated by CSS transitions, so this is likely a no-op; it also exceeds `dur-slow`. Unverified in browser. |
| `app.css:120-122` | `[data-slot]{animation:none!important}` under reduced | global for ui/ primitives only | Halo applies one global reduce rule to everything (`bundle.css:901-903`); Rift's is per-component → gaps below. |

**Reduced-motion coverage gaps in shell** (CSS that still moves under `prefers-reduced-motion`): `.side-rail` width + `.sidebar` transform/opacity/top transition (`Sidebar:218-221, 274`; its reduce block `:349-351` covers only `.snav-ic/.fnav-item`); `.main` margin (`AppShell:417`); `.app` bg (:360); `.workdots` ok (`ConversationList:543`); `ProjectSwitcher` skeleton ok; `NotificationCenter` hover transitions ok (:418-420); StatusBar none needed. `rift-seam`, intro: `intro.arm()` early-returns when reduced (`intro.svelte.ts:11`).

**Perf notes tied to motion**: `filter: blur` animates on splash only (small); no `backdrop-filter` on fixed layers except the two portaled menus (`ConversationList:512`, `ProjectSwitcher:266`) — contradicting the WebView2 ban documented 4 times (`NotificationCenter:257-259`, `ToastHost:109-111`, `UpdateBanner:219-220`, `app.css:609-611`).

---

## 5 Styling wiring

- **Mechanism**: Svelte scoped `<style>` blocks + `:global()` for portaled bits. **Zero Tailwind utility classes** in markup across all shell files except `CloseConfirm.svelte` (5 class attrs :140-155, e.g. `data-[state=done]:border-accent/35`, `size-4.5`) and the bits-ui `ui/` primitives it wraps. Shared CSS: `app.css` (780; tokens :129-285, base :293-336, `.pill/.btn/.input/.tile*/.rift-menu*` :394-736) and `styles/severity-tone.css` (14; `.toast`/`.row` `--tone` map, imported by ToastHost + NotificationCenter).
- **Token surface used**: `--bg --bg-inset --bg-elev-1/2/3 --surface(-hover/-active) --border --border-strong --border-focus --fg --fg-2 --fg-muted --fg-subtle --fg-faint --accent(-soft) --ring --ok --warn --danger(-fg/-soft) --info --status-busy --island-radius --island-border --island-fill --shadow-float --shadow-lg --dur-* --ease-* --stagger --font-ui --font-mono --fs-xs/sm/md --titlebar-h --rail-w`. 5 text steps (`fg`, `fg-2`, `fg-muted`, `fg-subtle`, `fg-faint`) vs Halo's 3 (`ink`, `ink-muted`, `ink-faint`).
- **Accent is runtime-themeable**: `--accent-h/-c/-a` dials (`app.css:171-173`) drive `--accent`, `-hover`, `-active`, `-soft`, `--ghost-border`, `--ring`; Settings exposes them (comment :167-169). `app.html` carries `data-direction="linear" data-mode="dark"`; theme tokens are keyed on that pair (`app.css:129-130`) — **dark only**.
- **Hard-coded counts (my grep, approximate; px counts include every `Npx` token)**: px literals — AppShell 23, Sidebar 78, ConversationList 103, ProjectSwitcher 58, NotificationCenter 70, StatusBar 35, Topbar 32, UpdateBanner 22, WorkspaceShell 22, ToastHost 24, Splash 28, Titlebar 7. Literal `oklch()` colors (check:ui metric): AppShell 9, Sidebar 7, ConversationList 5, ProjectSwitcher 9, Splash 5, Titlebar 1 = **36**. Hex 0, `rgb/hsl()` 0 in shell. `color-mix(` ×84 across shell files (most tinting is mix-based, i.e. derived from tokens).
- **font-size spread (px literals in shell files)**: 9, 9.5(×4), 10(×6), 10.5(×4), 11(×3), 11.5, 12(×2), 12.5(×5), 13(×3), 14, 40 (wordmark) = **11 distinct** sizes, half-pixel heavy. Token sizes used besides: `--fs-xs/sm/md` (NotificationCenter, UpdateBanner, ToastHost, Titlebar). Halo has a named scale (display…caption: 14/20 nav, 13/18 crumb, 11/16 mono count).
- **radius spread**: 5, 6, 7(×9), 8(×8), 9(×6), 12, 14 (`--island-radius`), 999(×5), 50%, plus 1.5/2/3. Halo ladder: 8 sm / 12 md / 16 lg / 999 pill (cards lg, controls & popups md, small parts sm).
- **Durations/eases**: tokens mostly, but literals remain: `240ms` ×3, `0.22s`, `0.36s`, `1.4s`, `1.6s`, `1.05s`, splash set; 3 literal beziers incl. `(.65,0,.35,1)` (non-token).
- **Layering of surface material**: opaque gradients + noise + inset shadows (non-Halo). Menus/panels: `color-mix(--bg-elev-2 90%)` + blur (`ConversationList:512`) vs opaque (`NotificationCenter:265`) vs `bg-surface` — three different popover recipes in the shell alone.
- **check:ui ratchet contribution** (`design-system/ui-baseline.json`: portal 11, aria-role 17, hex 21, rgb-hsl 16, oklch-literal 131, arbitrary-value 2): shell holds portal 3 (ConversationList, NotificationCenter, ProjectSwitcher), aria-role 3, oklch-literal 36 (27% of the app's 131). The arbitrary-value hit in StatusBar:65 is a false positive (regex literal `/^GPT-[^-]+-Codex-/`).
- **Fonts**: Inter Variable (UI), JetBrains Mono, Lexend (splash wordmark only `Splash:201`) via `@fontsource-variable/*` (`+layout.svelte:2-4`). Geist/Geist Mono are **not installed** (`node_modules/@fontsource-variable` = inter, jetbrains-mono, lexend). CSP `font-src 'self' data:` (`tauri.conf.json:28`) blocks Halo's Google Fonts `@import` (`bundle.css:1`); precedent: ConsoleX self-hosts `@fontsource-variable/geist` + `geist-mono` (`consolex/panel/package.json:32-33`, `app.css:57-59`).

---

## 6 Restyle boundary

### LOGIC to keep (do not rewrite when reskinning)
- **Global key map & boot** — `AppShell.svelte:72-107, 113-199, 201-301` (shortcuts incl. Alt+1-9, Alt+arrows, Ctrl+T/W/Tab/\\, Ctrl+N→`goHome`, Ctrl+1-9 by order, Ctrl+, Ctrl+P/K, UI zoom Ctrl+=/-/0; heartbeat; focus re-probe 60s min-gap; file-drop guard; `listen("rift://close-requested")`; `invoke("ui_heartbeat")`).
- **Onboarding gate** — `showOnboarding` derived (`:48-52`), `finishOnboarding` (:56-59).
- **View host** — `ensureLoaded/retryLoad`, `everOpened` lazy-mount latch, `inert/aria-hidden` (`WorkspaceShell:14-45, 84-91`), registry lazy imports (`workspaceRegistry.ts:29-38`). Keep "all opened workspaces stay mounted" (scroll/focus/in-flight state).
- **State classes** — `workspace` (init/migrate/setActive/order persistence), `shell` (collapse/peek/auto-collapse hysteresis/width/pins/scope/maxPanesForWidth), `intro`, `bootLoad`, `toast` (queue, history, pause/resume, unread), `contextMenu`.
- **Sidebar behaviors** — resize pointer-capture + keyboard (`Sidebar:67-95`), peek hover timing (`shell.svelte.ts:136-164`, 260ms grace), `--rail-w` publish (:45-47), footer nav derivation (:19-26), scope counts (:52-58).
- **ConversationList logic** — grouping/stable sort comment & comparator (`:36-146`), pin/rename/delete/menu/drag handlers (`:181-250`), live elapsed ticker (`:164-180`), `queuedCounts`, `liveTabs`.
- **ProjectSwitcher logic** — branch latch (`:35-42`), `openFocused / openInSplit / chooseNoProject / newProject` (`:75-103`), drag source (:106-114), context-menu items (:119-143), `rootKey` compare.
- **StatusBar logic** — provider/limits derivations (`:19-75`), GH popover wiring (:79-88), 5-min refresh interval (`:111-120`), `fmtReset`, settings deep links.
- **UpdateBanner `rows` derived** (`:71-155`) incl. suppression rules and handlers; **CloseConfirm** checklist + IPC (`:36-109`); **DeleteProjectDialog** confirm; **ContextMenuHost** focus-restore (`:34-59`); **Splash** readiness logic (`:18-90`) and session-seen gate; Titlebar/Topbar `win.*` calls + `onResized` tracking.
- **A11y semantics to preserve**: `aria-current` on nav, `role=separator` resize, `inert` on hidden pages/collapsed rail, `aria-live` on toast host and splash `role=status`, Escape/outside-click closers, focus-to-Cancel on destructive dialogs.

### MARKUP / CSS to change (the restyle surface)
- All `<style>` blocks in the shell files; chassis/island recipe (`AppShell:339-512`, `Sidebar:255-284`); grain/cavity/seam layers; foot-nav tile grid; conversation row anatomy; switcher row; statusbar layout; banner row; toast & notification markup; skeleton markup; splash visual choreography (keep logic).
- Hover/active micro-motion rules and the page-transition stack (wsIn + ws-page + rising) → one Halo page glide.
- Window-control markup duplicated in Topbar/Titlebar → single component (markup change only; handlers unchanged).
- `app.html` `data-*` attrs → `data-theme="blue"` mapping (needs the `app.css:129-130` selector change too).
- Token layer: either (a) alias Rift names → Halo values in `:root` (leaves ~44 files' `var(--fg…)` untouched) or (b) rename to Halo names; (a) keeps `check:tokens` mirror (`design-system/styles.css`, `_ds_manifest.json`) updatable in one place, (b) touches every component. Boundary decision for the plan wave, not made here.

---

## 7 Halo mapping table

Halo CSS values from `bundle.css`; Rift cites above.

| Rift element | Halo component / pattern / token | Notes / gap |
|---|---|---|
| `.app` chassis gradient + grain + cavity + `.rift-seam` (`AppShell:343-471`) | `.halo-root` flat `bg-000` ground (`bundle.css:3`) | Halo forbids washes/gradients ("depth by lightness, not boxes", README principle 1). **Kill** gradient, grain, cavity, seam, or demote seam to a 1px `line-100`. Biggest visual delta. |
| Floating islands with 12px gutters, 14px radius (`AppShell:386-411`, `Sidebar:255`) | `AppShell` flat frame: side `bg-000` + `border-right line-100`, top bar sticky `border-bottom line-100`, content padded `space-6` (`bundle.css:703-721`) | Structural fork (Q2). Halo's mock (`Main.dc.html:27-60`) used the flat frame. Keeping islands = a Rift-specific layer re-skinned with `bg-100` + `line-100`. |
| `Sidebar` rail 208–380 resizable, collapse-to-hidden + hover peek | `AppShell` side `--side-w`, collapse → **72px icon rail** (`bundle.css:703-718`); no resize, no peek | **Gaps**: resize handle and peek flyout are Rift-only (keep). Halo's icon rail replaces "collapse to nothing" (Q3). |
| Brand head `RiftLogo` 22 + "Rift" 14px/650 (`Sidebar:123-127`) | `Logo` 32px mark + title 600 15/20, −0.03em (`bundle.css:705-707, 788`) | Halo `Logo` is **Halo's** mark (README "Logo"); Rift keeps its own PNG (Q4). Brand-name type spec reusable. |
| `fnav-item` tile (icon over 10px label, 44px, 5 columns) (`Sidebar:320-330`) | `NavItem` row: 44px, 18px icon, 14/20 label, active = `bg-200` + accent icon + `aria-current` (`bundle.css:269-278`) | Different anatomy (Halo = horizontal row in a column, optional mono count). Rift's 5-across dock at 248px can't hold Halo rows. Decide: vertical NavItem list vs keep tile dock. Halo `halo-nav-in` 600ms active fade (`:271-272`). |
| Conversations (`ConversationList`) | `SectionLabel` (`01 / TODAY`, mono 11px caps +.12em `bundle.css:369`) + `NavItem` rows; order changes → `FlipList` (460ms) | **Gap**: Halo has no grouped sticky list, pin, working indicator, queued badge, elapsed, drag, rename, row menu → Rift-specific part built from `NavItem` + `Menu` + `LiveDot` + `Badge`. Halo row 44px vs Rift 34px. |
| Working dot-grid / elapsed (`ConversationList:268-271, 533-543, 300`) | `LiveDot` (ping 1.8s, `bundle.css:75-82`) + mono tabular time | Mapping is token-level; the 9-dot grid is Rift signature (keep or replace, Q). |
| `ProjectSwitcher` trigger + dropdown | `Menu` (trigger + `halo-menu__list`, item 40px, stagger 18ms `bundle.css:416-430`) or `Select`/`Combobox` w/ avatar tile (`Avatar`) | Dropdown today is a bespoke portaled menu; Halo `Menu` provides surface/motion. Project hue monograms conflict with "one signal colour" (project hue = per-project oklch hue `ProjectSwitcher:244`, `ConversationList:444`) → owner call (Q5). |
| New chat button + search icon (`Sidebar:142-150`) | `Button variant=secondary` + `SearchField size=sm shortcut` (Halo mock `Main.dc.html:33-34`) | Halo SearchField is an inline input with kbd chip; Rift opens the command palette instead (keep behavior, restyle trigger as SearchField look-alike). |
| Conversations header + count (`Sidebar:155-159`) | `SectionLabel index` | direct. |
| `Topbar` (40px: pin trigger + window controls) | `AppShell` header: `Breadcrumbs` + `SearchField` + `NotificationBell` + `Avatar`, 64px min, sticky (`bundle.css:720`) | Rift's top band is intentionally empty ("calm chrome", `Topbar:42-44`). Breadcrumbs could carry project / chat title (Q6). Window controls: **no Halo equivalent** (web-first) → Rift-specific part (mock drew 40×36 `.winbtn`, `Main.dc.html:22`). |
| `Titlebar` (onboarding) | none | Merge into one window-chrome component. |
| Open-chat tabs (do not exist) | `Tabs`: sliding 2px accent underline `dur-base`, optional count pill, 44px min (`bundle.css:259-266`) | New surface; mock used it. Needs decision (Q1). Not for filters (use `Segmented`). |
| Split pane header (`AssistantPane:587-`) | `SplitPane` (resizable) + small `Badge`/`Ring` for ctx | other map; noted for boundary. |
| `StatusBar` footer | **none** | Halo has no footer/status bar. Rift-specific; build from `LiveDot`, `Badge`, `Kbd`, `Progress`/`Ring`(ctx), `Popover` (UsagePanel/GH). Gap: "ok" green dot has no Halo token (Halo status = accent/warn/info/danger). |
| `UpdateBanner` rows | `Banner` (`tone info|accent|warn|danger`, `title`, `description`, one `action`, `onDismiss`; `bundle.css:387-396`) + `Progress` for download | Halo Banner sits *in the layout above the content it is about*; Rift's is full-window above both islands. App update = `accent`, CLI = `info` (**note**: in `blue` theme Halo `info` = lime `#c6f35a`, `halo-tokens.css [data-theme="blue"]`). Multiple stacked rows OK (gap 1px → `space-2`). |
| `NotificationCenter` bell tile + upward panel | `NotificationBell` (44px bordered button, accent count pill, swing+ping on increase, `bundle.css:346-357`) + `NotificationPanel` (right drawer 400px, scrim, groups, tone dots, row dismiss slide-out, focus trap) | Behavior change popover→drawer (Q7). Bell lives in sidebar footer today, Halo puts it top-right. Halo groups by caller-provided `group`; Rift buckets by age (keep). |
| `ToastHost` (top-center, island-centered via `--rail-w`) | `Toast` bottom-right stack, fan on hover, swipe, `tone accent|info|warn|danger`, `action` ghost Button (`bundle.css:314-335`) | Placement flips (Q8). Halo tone set lacks "muted" and "ok" (ok→accent). Rift's detail-expand-on-click and per-toast pause are extras to keep. |
| `Skeleton` | `Skeleton` (+ `halo-skel-wrap` crossfade) | direct; add crossfade. |
| `CloseConfirm` / `DeleteProjectDialog` | `Dialog` (scale .94, 480px, `bundle.css:285-291`) | Rift uses bits-ui AlertDialog (focus/escape semantics) — restyle recipe, keep primitive. |
| `ContextMenuHost` | `ContextMenu` | restyle `ui/dropdown-menu` recipe; keep point-anchored host. |
| `SplashOverlay` | none (Halo `Logo`/`Countdown` only) | Rift-specific; restyle tokens only. |
| `WorkspaceShell` page transitions | `AppShell pageKey` / `.halo-page` (`bundle.css:24-25`) + `.halo-rise` | direct (see §4). |
| Typography (Inter 12.5/13 compact) | Geist 14/20 body, Geist Mono labels (`HB/02`) | Density mismatch; Halo "touch targets ≥44px" vs Rift 26–36px desktop rows (Q9). |
| Color roles | `bg-000..300`, `line-100/200/300`, `ink/-muted/-faint`, `accent`, `accent-ink`, `*-soft`, `glow-strong`, `shadow-pop`, `shadow-focus`, `scrim` | Rift `--bg→bg-000`, `bg-elev-1→bg-100`, `bg-elev-2→bg-200`, `bg-elev-3→bg-300`, `--border→line-100`, `--border-strong→line-200` (or `line-300` for controls), `--fg→ink`, `--fg-2/muted→ink-muted`, `--fg-subtle/faint→ink-faint`. Rift's extra step and `--ok`, `--status-busy/pending/warn`, `--ring`, `--bg-inset`, `--tile-*` have no direct Halo name. |
| Focus ring | `shadow-focus`: 2px gap + solid 2px accent | Rift uses `--ring` glow (`StatusBar:235`, `Sidebar:249`); unify. |

---

## 8 Pain points (evidence)

1. **Three stacked page-motion layers** (`wsIn` `AppShell:483-485`, `.ws-page` fade `WorkspaceShell:128-148`, `.rising` stagger `:198-211`) plus a JS timer re-encoding CSS numbers (`SETTLE_MS` :47-48).
2. **Rail/toast desync**: `--rail-w` tweens 140ms (`app.css:353`) while the rail it tracks animates 240ms (`Sidebar:221`).
3. **Layout-affecting motion**: rail `width` and `.main` `margin-left` (`Sidebar:221`, `AppShell:417`), peek `top` (`Sidebar:228,274`); banner `slide` reflows the entire body (`UpdateBanner:159`, host sits above `.app-body` `AppShell:313`) — a late update check shoves the whole window down 38px+.
4. **Navigation inconsistency**: sidebar tile labeled "Home" (`Sidebar:33`) opens the *Workspace* page (`registry:31`, aria-label "Workspace" `Sidebar:174`), while Ctrl+1 on the same slot runs `goHome()` = new chat (`AppShell:288`). Registry `kbd` hints are unused and wrong (`registry:31-37` vs real order `workspace.svelte.ts:16-18`: settings is Ctrl+3, health Ctrl+4).
5. **Diagnostics unreachable from chrome** (filtered out `Sidebar:24`); Browser is a dock not a view; neither is discoverable as navigation.
6. **No tab strip**: open tabs, panes and history are one conflated list; "which chats are open" isn't visible; split pane header only appears in split (`AssistantPane:585`).
7. **Heavy surface material vs Halo**: gradients, 4 noise layers, cavity shadows, bevel (`AppShell:352-354, 372-381, 400-411, 441-443`); `Sidebar` and `.main` must be kept "in lockstep" by hand (comments `Sidebar:260-262`, `AppShell:404-406`) — duplicated recipe.
8. **Duplicate implementations**: window controls ×2 (Topbar/Titlebar); skeletons ×2 (`WorkspaceShell:159-176` vs `Skeleton.svelte`); popover recipes ×3 (`ConversationList:509-515`, `ProjectSwitcher:264-269`, `NotificationCenter:254-271`) plus `ui/` DropdownMenu; noise data-URI copy-pasted ×4 (`AppShell:378,428`, `Sidebar:281`, `Splash:159`).
9. **WebView2 ban violated twice**: `backdrop-filter` on `position:fixed` portaled menus (`ConversationList:512`, `ProjectSwitcher:266`) although four comments forbid it.
10. **Type/radius entropy**: 11 font sizes incl. 9, 9.5, 10.5, 11.5; 9 radii (6/7/8/9/12/14/999/50%…). Sidebar footer labels at 10px (`Sidebar:323`), count pill 9.5px (`:315`).
11. **Stale/lying comments**: UpdateBanner "sits at the top of .main" (`:205`) but is above both islands; ToastHost says "drop in top-center" and `toast.svelte.ts:2-4` says "bottom-right"; NotificationCenter header says Topbar bell (`:2-3`); `ConversationList` mini-peek comment "conv-preview" (:506-508) refers to a removed element.
12. **Notification panel a11y**: `role="dialog"` with no focus move/trap/return (`NotificationCenter:120-127`; no focus code in script :1-97); Esc handled. Halo's panel takes and returns focus (`HB/06-accessibility.md`).
13. **Hover-only affordances**: row pin/more actions only on hover (`ConversationList:497-502`), toast close hidden until hover (`ToastHost:199`) — keyboard users get them via focus only for toast (`:203`); conversation row `.crow` actions lack a `:focus-within` reveal (grep of `ConversationList` CSS: none).
14. **Status bar discoverability**: `.sb-btn` has no hover transition (`StatusBar:233-237`); AI-mistake disclaimer is permanent chrome (`:165`); media queries for ≤720/≤540 can't fire at min window 900.
15. **Hidden pages stay composited**: every opened workspace remains an absolutely-positioned opacity-0 layer with a transform transition (`WorkspaceShell:128-139`) — no `visibility:hidden`/`content-visibility` after the fade (cost unmeasured here).
16. **Accessibility of reduced motion** is a patchwork (see §4 gaps) and the JS probe is non-reactive (`accessibility.svelte.ts:103-106`).
17. **`rift-seam` is invisible-by-design** (`AppShell:449-471`) yet participates in layout/z-order; collapses via `display:none` (:472).

---

## 9 Risks

- **Tests**: no unit test references shell markup or classes (vitest files listed are state/util/helper tests; `workspace.test.ts` covers `init/setActive/order` only — keep those APIs stable). Only external selector users: `scripts/cdp/c.sh` examples and `.agents/skills/rift-ui/SKILL.md:56` use `.sidebar`; `docs/HANDOFF.md:48` mentions `div.crow`. Renaming `.sidebar/.crow/.main/.statusbar/.topbar` breaks CDP recipes and the rift-ui skill (update docs/scripts with the restyle).
- **`npm run check`** = svelte-check + `check:tokens` + `check:ui` (`package.json` scripts). `check:tokens` mirrors `app.css` `:root` against `design-system/styles.css` + `_ds_manifest.json` — *any token add/rename/value change must be mirrored* or it exits 1 (`design-system/check-tokens.mjs`). `check:ui` ratchets only go **down** (`ui-baseline.json`); porting Halo should *drop* the 36 shell `oklch` literals and 3 portal/role hits; new hex/rgb/oklch literals or arbitrary Tailwind values fail the gate. Also validates every class in `components/ui/` generates CSS under the `@theme` bridge — new Halo tokens used in `ui/` need `@theme inline` entries (`app.css:29-115`, `cn` registration note :16-17).
- **Tailwind bridge wipes defaults** (`app.css:18-28`): Halo's `--radius-md: 12px` etc. collide by name with Rift's `--radius-md→var(--radius)` mapping (:84); `--shadow-lg`, `--ease-*` names overlap — porting verbatim token names will silently change every `rounded-md/shadow-lg` utility across `ui/` and CloseConfirm.
- **Halo `blue` theme swaps `info` → lime** (`halo-tokens.css [data-theme="blue"]`, `--info:#c6f35a`): Rift's info toasts, CLI-update banner tone, notification default row tone (`NotificationCenter:308 --tone: var(--info)`) turn lime; and `ok`/`status-busy` greens have no Halo token. Needs an explicit status mapping before restyle.
- **Light theme**: Halo ships light presets; Rift is dark-only (`app.html:2`, `app.css:125-126`). Out of scope unless owner asks.
- **Dials**: Rift's runtime accent hue/chroma/strength (`app.css:171-173`) vs Halo's 4 fixed accent presets — keeping the dials means deriving Halo `accent-soft/glow/focus` from oklch at runtime (feasible, today's pattern); fixing to `#6aa8ff` means retiring Settings controls (other map).
- **Fonts**: Geist requires bundling (not installed; CSP blocks remote). Changing UI font changes text metrics → truncation/wrapping in 34px rows, 10px tiles, status bar; `font-feature-settings` for Inter (`app.css:306`) wouldn't apply. Lexend a11y font override targets `body, .app` (`app.css:747-755`).
- **Perf**: width/margin transitions reflow the whole chat stream (documented lag `Sidebar:219-220`). Halo's AppShell also animates width (`bundle.css:703`), so parity isn't a regression, but Rift's chat-heavy content makes each frame costlier. Keep ≤300ms; consider transform-based collapse. Constant-time costs to drop with the slab: 4 tiled SVG-noise layers, 70px inset shadow on full-window pseudo.
- **WebView2 / Tauri quirks**: no `backdrop-filter` on fixed layers; frameless drag regions (attribute vs CSS property uncertainty); native child webview (browser dock) tracks bounds via `ResizeObserver` during `dockSlide` — any new layout animation near the dock can desync it; `html` `@property --rail-w` transition requires Chromium registered-property support (works in WebView2, comment `app.css:342-346`). `inert` support fine. `prefers-reduced-motion` follows OS only.
- **Dev showroom routes** (`/dev/gallery`, `/dev/ui`) also mount under root layout and use shell tokens; they'd need token continuity.
- **Splash/first-paint colors** are literal (`app.html:9` bg, `Splash:146-148` panel gradient uses `--bg`) — theme change must update `app.html` inline bg or the first frame flashes the old ground.
- **Keyboard/regression surface**: all shortcuts live in `AppShell.svelte` (one function) — safe if the markup is replaced but the `<script>` kept.
- **Unverified**: nothing here was run; all runtime behaviours (blur costs, maximize double-click, snap layouts, SvelteKit transition locality) are from source reading only.

---

## 10 Open owner questions

1. **Tabs**: add a Halo `Tabs` strip for open chats (as the first mock did) or keep "sidebar list = tabs"? If added: where (topbar band vs per-pane), and does it replace the 28px split pane header?
2. **Frame**: keep floating islands (12px gutters, 14px radius) re-skinned with Halo surfaces/lines, or switch to Halo's flat docked frame (sidebar border-right, sticky top bar)? This decides whether chassis gradient/grain/cavity/seam are deleted or restyled.
3. **Sidebar collapse**: keep "collapse to nothing + hover peek + resize", or adopt Halo's 72px icon rail (no peek)? Keep the resize handle either way?
4. **Brand**: Rift keeps its own logo PNG and "Rift" wordmark (Halo `Logo` is Halo's mark) — confirm; splash wordmark stays Lexend or move to Geist 600 (-0.03em)?
5. **Project hues**: per-project identity colors (monograms, tags) vs Halo's "one signal colour" — neutral `bg-300` tiles with initials, or keep hue?
6. **Top bar content**: keep it empty (calm) or add Breadcrumbs (project / chat) and/or a SearchField that opens the palette? Should the Alerts bell move from the sidebar footer to the top-right like Halo?
7. **Notifications**: popover (today) vs Halo right-side drawer with scrim + bell swing — and keep age buckets?
8. **Toasts**: top-center of the island (today) vs Halo bottom-right fan stack with swipe-to-dismiss? (Bottom-right collides with the composer/status bar per the old code comment, `ToastHost:3-6`.)
9. **Density**: Halo is 14px/44px touch targets; Rift is 12.5px compact. Pick the "happy medium": Halo scale unchanged, or a compact variant (e.g. 13px / 36px rows) as a documented Rift density?
10. **Status colors under `blue`**: map Rift `ok`/`busy`/`pending` and `info` (Halo swaps info to lime) — accept lime info, or override `info` for Rift so blue accent + blue info don't collide differently?
11. **Accent dials**: keep Settings hue/chroma/strength sliders on top of Halo, or lock to the 8 Halo presets (and light mode)?
12. **Fonts**: approve bundling Geist + Geist Mono (self-hosted `@fontsource-variable`, as ConsoleX did) and retiring Inter/JetBrains Mono, or keep Inter/JetBrains with Halo's scale?
13. **Navigation**: fix "Home" vs Workspace naming and Ctrl+1 semantics; surface Diagnostics (and Browser) in nav or leave palette-only?
14. **Update banner**: keep full-window strip above everything or move inside the content island (Halo: banner belongs to the page, above the content it is about)?
15. **Status bar**: keep as is (Halo has none) — and keep the permanent "AI can make mistakes" note?
16. **Splash**: keep the two-doors reveal (non-Halo) as Rift's signature, simplify to a Halo-style fade/rise, or drop the caret blink and blur only?
17. **Motion scope**: ok to remove the per-row `rowIn` cascade on history remounts and the per-row active-bar pop in favor of a single sliding marker?
