---
title: "Halo P4a — Shell"
status: draft
updated: 2026-10-19
phase: P4a
depends_on: [00-decisions, 01-foundation, 02-kit, 03-motion]
tags: [rift, halo, spec, shell]
---

# P4a · Shell

## 0. Summary

- P4a ports the frame to Halo with the Electric blue preset. Two flat islands (`bg-100`, 1px `line-100`, r16, 12px gutters) sit on a `bg-000` canvas. Gradients, grain, bevel, cavity, AO, `.rift-seam` and the 1s `.app` transition are deleted (D-01).
- The sidebar keeps Rift's behaviour (resize, collapse-to-nothing, peek, pin) and takes Halo NavItem anatomy. Rows become 32px buttons with one sliding active plate. The dock goes icon-only and says "Home" everywhere (D-19).
- Every bespoke overlay in the shell moves onto the kit. That covers 3 portals, 2 hand-rolled `role=menu`/`dialog` panels, ToastHost → Toaster, and UpdateBanner → Banner inside the main island (D-20).
- The work is 22 ordered commits. The merge waits on shortlist #1 and board 01. Effort is L, 2-3 sessions.
- Nothing here re-does P2. Live dots (M14), static bones (M23), splash retune (M25), page entrance (M30), the sidebar one-move (M31), rows first-mount (M32), zero backdrop-filter (M37), banner motion (M38) and `rm()` (M11) are prerequisites.

## 1. Scope + files

All lines were verified at `design/chat-ledger` @ `22727d24`. "P1/P2 touched" means an earlier phase edits those lines first. Anchor on the snippet, not the number.

### 1.1 Files this spec edits

| File | Lines | What P4a touches |
|---|---|---|
| `src/lib/components/AppShell.svelte` | 513 | keymap `:288` (Ctrl+1); template `:304-337` (UpdateBanner `:313` moves, seam `:316` deleted, ToastHost `:332` swapped); chassis CSS `:343-474`; intro `:493-501` |
| `src/lib/components/shell/Sidebar.svelte` | 352 | `NAV_LABEL` `:32-35`; `--rail-w` publish `:45-47` (kept); head `:122-138`; action row `:142-150`; history head `:154-159`; dock `:165-194`; CSS `:215-351` |
| `src/lib/components/shell/ConversationList.svelte` | 544 | imports `:8,12-14`; skeleton `:24-31,323-331`; row snippet `:253-320`; group labels `:354-355,381`; menu `:398-412`; CSS `:414-543` |
| `src/lib/components/shell/ProjectSwitcher.svelte` | 286 | hue `:16,28,212`; skeleton `:17,19,146`; context items `:119-143` (`:131` label); trigger `:155-181`; `.sw-menu` portal `:184-224`; CSS incl. `:264` z, `:266` blur |
| `src/lib/components/shell/Topbar.svelte` | 77 | window controls `:45-49` → shared part; `.tb-ic` `:27-37`; CSS `:53-77` |
| `src/lib/components/shell/Titlebar.svelte` | 76 | window controls `:30-40` → shared part; CSS `:43-76` |
| `src/lib/components/shell/StatusBar.svelte` | 279 | skeleton `:14-15,124`; drag region `:123` (kept); LimitBar compact `:206`; `.sb-dot` `--ok` `:240`; media queries `:266-278` (kept) |
| `src/lib/components/shell/NotificationCenter.svelte` | 421 | `severity-tone.css` import `:14`; badge `:112-116`; `role=dialog` panel with no trap `:119-203`; panel CSS `:255-290` (z 2100 at `:270`) |
| `src/lib/components/ToastHost.svelte` | 212 | DEV hook `__riftToast` `:18-20` moves to Toaster; component unmounted |
| `src/lib/components/shell/UpdateBanner.svelte` | 332 | rows `:71-155` (logic kept); markup `:158-197` → Banner; CSS |
| `src/lib/components/shell/CloseConfirm.svelte` | 170 | manual Cancel focus `:116-119` → kit `destructive` |
| `src/lib/components/shell/DeleteProjectDialog.svelte` | 61 | AlertDialog → `destructive` |
| `src/lib/components/shell/ContextMenuHost.svelte` | 103 | drop local surface CSS so the K05 recipes apply |
| `src/lib/components/shell/WorkspaceShell.svelte` | 212 | load state `:96-108` + CSS `:149-191` → kit Skeleton + Button |
| `src/lib/components/shell/workspaceRegistry.ts` | 38 | titles `:31,34` "Workspace" → "Home" |
| `src/lib/components/dialogs/CommandPalette.svelte` | — | `:60` "Go to Workspace" → "Go to Home" |
| `src/routes/+layout.svelte` | 67 | tooltip-layer mount beside `<ContextMenuHost />` `:63` |
| `src/routes/dev/ui/+page.svelte` | — | its local tooltip-layer mount (added in K37) is removed in S03 |
| `scripts/cdp/c.sh` | — | nav/tour labels `:481,:514` "Workspace" → "Home" |
| new `src/lib/components/shell/WindowControls.svelte` | ~70 | shared min/max/close |

### 1.2 Read, not edited (verify only)

| File | Lines | Why |
|---|---|---|
| `src/lib/components/SplashOverlay.svelte` | 293 | D-26 retune is P2 M25. Colours are P1 D-34. P4a only checks the hand-off onto the flat canvas |
| `src/app.html` | 22 | first paint `#08090c` is P1 |
| `src/lib/state/intro.svelte.ts` | 26 | 700ms settle ≥ 600 + 60 coupling (03 §7) |
| `src/lib/state/bootLoad.svelte.ts` | 25 | consumers: ConversationList `:14`, ProjectSwitcher `:19`, StatusBar `:15`, AssistantWelcome `:20` |
| `src/lib/state/shell.svelte.ts` | 207 | collapse, peek, width, pins. Logic untouched |
| `src/lib/state/workspace.svelte.ts` | — | `order` `:16-18` drives Ctrl+1..9 |
| `src/lib/state/nav.ts` | 12 | `goHome()` = new chat; Ctrl+N keeps it |
| `src/app.css` | — | `--rail-w` `:342-354`, `--titlebar-h: 34px` `:259`, `.island` `:502-510` (no markup consumer), `--island-*` `:210-212` |

**Size of the set.** The literal counts in §10 cover 17 files, 4017 lines: AppShell, ToastHost, SplashOverlay and the 14 `shell/*.svelte`, RiftLogo included. The 4092 `wc` total in my notes is a different set: it adds the `.ts` state files, `+layout` and `app.html`, and leaves SplashOverlay out.

## 2. Target design

### 2.1 Wireframe (1440 × 900, compact, sidebar pinned at 264)

```
bg-000 canvas #08090c ─────────────────────────────────────────────────────────────────────────┐
│12                                                                                            │
│  ┌ sidebar island ──────────┐12┌ main island ──────────────────────────────────────────────┐ │
│  │ bg-100 #0e1014 · 1px line-100 #1f242c · r16                                             │ │
│  │ [▣ Rift]            [⇤]  │  │ [⇥ peek/pin when collapsed]              [ – ][ □ ][ × ]  │ │  ← head 40 / topbar 40, drag regions
│  │ ┌──────────────────────┐ │  ├──────────────────────────────────────────────────────────┤ │
│  │ │[R] rift-tauri  main ▾│ │  │ ▸ Banner (accent) Update ready · Rift 0.9.3  [Restart] ×│ │  ← UpdateBanner, inside .main, Collapse
│  │ └──────────────────────┘ │  │                                                          │ │
│  │ [+ New chat        ][⌕]  │  │                 WorkspaceShell (.ws-page stack)          │ │
│  │ CONVERSATIONS ─────── 24 │  │                                                          │ │  ← SectionLabel mono 11 + count
│  │┌▒ Live chat title   ●  ┐│  │             ┌ Toaster (top 54, centred on main) ┐        │ │
│  ││  Pinned title      📌 ││  │             └───────────────────────────────────┘        │ │
│  │└▒ active plate ────────┘│  │                                                          │ │  ← plate: bg-200 + inset line-200
│  │  Another title    2h    │  │                                                          │ │
│  │ YESTERDAY ────────── 4  │  │                                                          │ │
│  │  …                      │  │                                                          │ │
│  │ ────────────────────────│  ├──────────────────────────────────────────────────────────┤ │
│  │ [⌂] [💬] [♥] [🔔3] [⚙]  │  │ StatusBar · t-caption ink-faint · ● accent              │ │  ← dock: 5 equal columns, icon tiles
│  └──────────────────────────┘  └──────────────────────────────────────────────────────────┘ │
│12                                                                                            │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

Collapsed: the rail is 0 wide and `.main` has `margin-left: 12px`. The peek is the same island in its mini form (240 wide, content height). It floats at `top: 54px` with `shadow-pop` and `z-index: var(--z-hud)` (20).

### 2.2 Tokens (names from 01 §2; values resolved for Halo blue + compact)

| Element | Property | Token | Resolved |
|---|---|---|---|
| `.app` | background | `--bg-000` | #08090c |
| `.app` | font | `--font-sans`, `--t-small-size` / `--t-small-line` | Geist 13/18 |
| `.main`, `.sidebar` | background | `--bg-100` | #0e1014 |
| `.main`, `.sidebar` | border | 1px `--line-100` | #1f242c |
| `.main`, `.sidebar` | radius | `--radius-lg` | 16 |
| gutters | margin / inset | `--space-3` | 12 |
| `.sidebar` peek | shadow | `--shadow-pop` | 0 12px 32px rgba(0,0,0,.45) |
| side-head brand | type | Geist 600, −0.03em, `--t-body-size` (A-9) | 15 |
| side-collapse, search, tb-ic | IconButton ghost/secondary | `--control-dense-h` / `--control-h` | 28 / 32 |
| New chat | Button secondary | `--control-h`, `--control-r` | 32, r8 |
| Rows | height | `--list-row-h` | 32 (36 / 40) |
| Row title | type | `--t-small-size/-line`, `--ink-muted` → `--ink` 500 active | 13/18 |
| Row meta (time) | type | `--t-caption-size/-line`, `--ink-faint`, tabular | 12/16 |
| Row hover | background | `--bg-200` (`surface-hover`) | #14171c |
| Active plate | background + ring | `--bg-200` + `inset 0 0 0 1px var(--line-200)` | #14171c / #2c323c |
| Row radius | radius | `--radius-sm` | 8 |
| Row actions | IconButton ghost sm | `--control-dense-h` | 28 |
| Group / history labels | SectionLabel | `--t-label-*`, mono, `--ink-faint`, rule `--line-100` | 11/14 500 .12em |
| Dock tile | size / radius | 1/5 column × `--control-h`, `--radius-sm` | ~31-39 × 32, r8 |
| Dock tile icon | size / colour | 18, `--ink-muted` → `--accent` active | #6aa8ff |
| Dock active | background | `--bg-200` | #14171c |
| Dock divider | border-top | 1px `--line-100` | #1f242c |
| Unread count | Badge accent mono | `--badge-h`, `--accent-soft`/`--accent` | 20 |
| Monogram | bg / ink | `--bg-300` / `--ink-muted`, `--radius-xs` | #1b1f26 / #9aa2af, r6 |
| Focus | ring | `--shadow-focus` | 0 0 0 2px #0e1014, 0 0 0 4px #6aa8ff |
| Resize handle | hover / focus bar | `--line-300` / `--accent` | #5f6774 / #6aa8ff |
| Floating (menus, bell) | recipe | `floatingPanel` (`bg-100`, `line-200`, r12, `shadow-pop`) | — |
| Window controls | hover / close | `--bg-200` / `--danger` + `--danger-fg` | — |
| StatusBar | type / border | `--t-caption-*`, `--ink-faint`; top 1px `--line-100` | 12/16 |
| Z | side-rail / menus / toasts / tooltip | `--z-hud` / `--z-dropdown` / `--z-toast` / `--z-tooltip` | 20 / 65 / 60 / 70 |

The brand-head row: D-26 fixes the weight and tracking, not the size. Today's 14px (`Sidebar:294`) is not a D-06 step, so it goes to `--t-body-size` 15 with weight 600 and −0.03em. Board 01 can pull it back to 13 (`--t-small-size`).

### 2.3 Density tiers

| Var | compact | regular | comfy | Shell consumers |
|---|---|---|---|---|
| `--list-row-h` | 32 | 36 | 40 | `.crow`, skeleton rows, menu rows |
| `--control-h` | 32 | 36 | 44 | New chat, search, dock tile **height** (width = one of 5 equal columns), switcher trigger (`+ 8px`) |
| `--control-dense-h` | 28 | 32 | 36 | row actions, side-collapse, `.tb-ic`, bell header actions |
| head / topbar | 40 | 40 | 40 | fixed. They are drag strips, not controls |
| StatusBar | today's height | same | same | not density-tied in P4a (§14 Q6) |

The rows fit: a 28px action inside a 32px row leaves 2px each side, and the same holds at 36/32 and 40/36.

## 3. States

| Part | rest | hover | focus-visible | active / current | open | disabled / loading | reduced motion |
|---|---|---|---|---|---|---|---|
| Island | `bg-100` + `line-100` | — | — | — | — | — | — |
| Peek island | hidden (`inert`, opacity 0, x −16) | opacity 1, `shadow-pop`, `z-hud` | inherits | click pins (`toggleCollapsed`) | — | — | instant (M31) |
| Row `.crow` | transparent, `ink-muted` | `bg-200`, `ink`; time hides, actions show | `.crow-main` ring `shadow-focus` (inset); actions show (`:focus-within`) | `data-state=active`: plate under it, title `ink` 500, `aria-current="true"` on `.crow-main` | menu open: `.menu-open` keeps actions shown | renaming: input replaces `.crow-main` | plate jumps (no slide) |
| Row live | `.live-dot` 8px accent in lead slot, elapsed `m:ss` `ink-muted` tabular | same | same | same | — | — | dot solid |
| Row pinned | pin glyph 14 `ink-faint` in lead slot, **no wash** | same | same | same | — | — | — |
| Row queued | Badge neutral mono `Clock 12` + n | stays visible | — | — | — | — | — |
| Row project tag (All scope) | Badge neutral, max 92px, ellipsis; hides on hover | hidden | hidden | — | — | — | — |
| Dock tile | `ink-muted` icon 18 | `bg-200`, `ink` | `shadow-focus` | `bg-200` + accent icon + `aria-current="page"` | bell: `aria-expanded` → `bg-200` | — | — |
| Bell count | Badge accent mono, hidden at 0 | — | — | — | — | — | no `count-pop` |
| New chat | Button secondary | `bg-300` | ring | scale .97 | — | — | no scale |
| Switcher trigger | `bg-100` row, 1px `line-100`, r12 | `bg-200` | ring | — | `aria-expanded`: `bg-200`, `.chev` 180° | skeleton bones | — |
| Menus (row, switcher) | — | row `data-highlighted`: `bg-200`, `ink`, accent icon | roving | — | `floatingPanel` + pop-in 150 | — | instant |
| Bell popover | — | rows `bg-200` | trapped, first row | — | upward pop-in-up 150, focus inside | empty: EmptyState compact | instant |
| Toast | `bg-200` + `line-200` + `shadow-pop` | stack fans, timers pause | × visible | — | — | danger sticky | instant |
| Banner | tone accent (app) / info (CLI) | — | action ring | — | Collapse open + `halo-rise` | download: Progress | instant |
| Resize handle | invisible | 2px `line-300` | 2px `accent` | dragging: 2px accent | — | hidden when collapsed | — |
| Window control | `ink-muted` | `bg-200` | ring | — | — | — | — |
| Close control | `ink-muted` | `--danger` bg, `--danger-fg` | ring | — | — | — | — |

The z rungs are named in P1 (01 §2 z table rows 15-18): `.side-rail` → `--z-hud`. `.sw-menu`, `.conv-menu` and the bell panel → `--z-dropdown`. All three panels become bits-ui content in P4a, so the recipe carries `z-(--z-dropdown)` and the local declarations go.

## 4. Motion

Every class and token is 03's. P4a adds no keyframe.

| # | Event | Element | Carrier | Duration / ease | Stagger | Reduced | Source today → after |
|---|---|---|---|---|---|---|---|
| 1 | Boot hand-off | `.main` y `--rise-page` → 0, opacity; sidebar x −16 → 0 | `data-intro=handoff` transitions | `--dur-slow` 600, `--ease-out`, `.main` delay 60 | — | skipped (`intro.svelte.ts:11`) | `AppShell:494-501`, `Sidebar:238-243` (12px, `--dur-rise`, `--ease-page`) → 03 row 9 values. **No M commit owns this; P4a S02 claims it** |
| 2 | Collapse / expand | rail width, `.main` margin-left, `--rail-w`, island x/opacity | transitions | `--dur-base`, `--ease-out` (one move) | — | instant | M31 (P2). P4a keeps it |
| 3 | Peek in / out | island opacity, x, `top` snap | transitions | `--dur-base`, `--ease-out`; `top 0s` delayed on leave | — | instant | M31 (P2) |
| 4 | Active row change | `.crow-plate` translateY + height | transition on `--ind-y`/`--ind-h` | `--dur-base`, `--ease-out` | — | jump | new (replaces `barPop` `ConversationList:483-484,523`) |
| 5 | Rows first mount | `.crow` | `.rise-row` under `[data-first]` | `--dur-base` | `min(i,12)×25ms` | none | M32 (P2) |
| 6 | Row actions reveal | `.crow-acts` | opacity | `--dur-fast` | — | instant | M32 (P2) |
| 7 | Live row | `.live-dot` | ping | `--dur-ping` 1.8s | — | solid | M14 (P2) |
| 8 | Row / switcher menu | DropdownMenu content | `floatingMotion` + `data-menu-stagger` | pop-in 150, rows 220 | `min(i,12)×18ms` | instant | `convPopIn` `:530` deleted |
| 9 | Bell popover | Popover content, `data-side=top` | `pop-in-up` | 150; leave `pop-out` 140 | rows `.menu-rows` 18ms | instant | NC `transition:` sites (M11-wrapped) deleted |
| 10 | New notification | bell icon | `.bell-swing` (one shot, re-keyed) | `--dur-roll` | — | none | new, D-16 vocabulary |
| 11 | Unread count change | Badge | `.count-pop` (re-keyed on value) | `--dur-base`, `--ease-spring` | — | none | new |
| 12 | Toast enter / leave / fan | Toaster | `.toast-motion` | 300 / 240 / 300 | stack math | instant | ToastHost `animate:flip` + fly |
| 13 | Banner enter / leave | Collapse + Banner | grid-rows + `.banner-motion`; `data-leaving` | 300 / 200 | — | instant | M38 (P2) banner motion; P4a moves the host |
| 14 | Chevron | switcher `.chev`, Show earlier | `.chev` 180° | `--dur-base` | — | instant | `.sm-ch` 0.22s literal `:423` |
| 15 | Dialog | CloseConfirm, DeleteProjectDialog | kit zoom-in / zoom-out | 300 / 200 | — | instant | K07 |
| 16 | Skeleton | ConversationList, ProjectSwitcher, StatusBar, WorkspaceShell | kit Skeleton (delay 300, hand-off 600) | — | — | instant | M23 bones (P2) → kit component |
| 17 | Tooltip | every `use:tooltip` in the app | singleton | 400 cold / 120 warm, `halo-tip-in` 150 | — | instant | the layer mounts in P4a (§6 S03) |

## 5. Parts

### 5.1 Kit primitives used

| Kit part | Where |
|---|---|
| Button (secondary, ghost sm) | New chat; WorkspaceShell "Try again"; bell header "Clear"; Banner actions |
| IconButton (ghost sm / secondary) | side-collapse, search, `.tb-ic`, row pin + more, Banner dismiss, toast × |
| Badge (neutral, accent, mono) | row queued, row project tag, bell count, switcher branch |
| SectionLabel (`count`) | history head, group labels |
| LiveDot | live row lead slot (P2 class, P3 part) |
| Skeleton (`variant="text"`) | list, switcher trigger, status bar, WorkspaceShell |
| Banner, Collapse, Progress | UpdateBanner |
| Popover (`data-side=top`) | bell |
| DropdownMenu + `menuItem`/`menuItemDanger`/`menuSeparator` | row menu, switcher menu |
| ContextMenu recipes | ContextMenuHost, switcher right-click |
| AlertDialog `destructive` (K24) | CloseConfirm, DeleteProjectDialog |
| Toaster (K34b) | AppShell |
| tooltip-layer (K32) | `+layout.svelte` |
| EmptyState (`compact`) | bell empty, list empty |
| `slidingIndicator` (`motion/indicator.ts`, axis y) | row plate |
| `overlays.trackFloating` | comes free with the bits-ui content parts |

The kit `list` Skeleton variant is avatar rows (a 36px circle plus two bars), which conversation rows are not. The shell uses `variant="text"` with `lines` and bars 13px tall at `--list-row-h` spacing. §13 A-6 proposes a `rows` variant if 4d needs the same.

### 5.2 Rift parts (built in P4a, live in `shell/`)

```ts
// shell/WindowControls.svelte — new; replaces Topbar:45-49 and Titlebar:30-40
interface Props { variant?: "island" | "bar" }   // island = 36×30 r8 (Topbar); bar = 38×100% square (Titlebar)
// owns: getCurrentWindow(), maximized $state + onResized listener (one copy, today duplicated at Topbar:11-18 / Titlebar:13-21)
// renders 3 <button>s: aria-label Minimize | Maximize/Restore | Close, use:tooltip, -webkit-app-region:no-drag
// glyphs: Topbar's CSS glyphs (.wc-min/.wc-max/.wc-restore, Topbar:70-76) for both variants
```

```svelte
<!-- ConversationList row (replaces :253-320). .crow is a plain div; the button is the row. -->
<div class="crow" data-state={active ? "active" : undefined} class:menu-open={menuOpen} style="--i:{i}"
     draggable={renaming === c.id ? "false" : "true"} ondragstart={(e) => onRowDragStart(e, c.id)} ondragend={onRowDragEnd}>
  {#if renaming === c.id}
    <input class="crow-rename" … />                       <!-- unchanged logic :278-286 -->
  {:else}
    <button class="crow-main" type="button" aria-current={active ? "true" : undefined} onclick={() => open(c.id)}>
      <span class="crow-lead">{#if live}<LiveDot />{:else if pinned}<Pin size={14} />{/if}</span>
      <span class="crow-title">{c.title || "Untitled"}</span>
      {#if qn}<Badge mono>…</Badge>{/if}
      {#if showAll}<Badge class="crow-proj">{lbl}</Badge>{/if}
      <span class="crow-time" class:busy={live}>…</span>
    </button>
    <span class="crow-acts">
      <IconButton label={pinned ? "Unpin conversation" : "Pin conversation"} onclick={() => shell.togglePin(c.id)}>…</IconButton>
      <DropdownMenu.Root bind:open={…}>
        <DropdownMenu.Trigger>{#snippet child({ props })}<IconButton {...props} label="Conversation actions">…</IconButton>{/snippet}</DropdownMenu.Trigger>
        <DropdownMenu.Content data-menu-stagger align="start" onCloseAutoFocus={(e) => { if (renaming) e.preventDefault(); }}>
          Pin/Unpin · Rename · separator · Delete (variant=danger)
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </span>
  {/if}
</div>
```

The plate:

```svelte
<div class="conv-list" use:slidingIndicator={{ selector: '.crow[data-state="active"]', axis: "y", target: plateHost }}>
  <span class="crow-plate" bind:this={plateHost} aria-hidden="true"></span>
  …rows…
</div>
```

```css
.conv-list { position: relative; }                       /* the scroll container: the plate scrolls with the rows */
.crow-plate { position: absolute; left: 0; right: 0; top: 0; z-index: 0; pointer-events: none;
  height: var(--ind-h, var(--list-row-h)); transform: translateY(var(--ind-y, 0));
  background: var(--bg-200); box-shadow: inset 0 0 0 1px var(--line-200); border-radius: var(--radius-sm);
  opacity: 0; }
.crow-plate[data-ready] { opacity: 1;
  transition: transform var(--dur-base) var(--ease-out), height var(--dur-base) var(--ease-out); }
.crow { position: relative; z-index: 1; }
.conv-group-label { z-index: 2; }                        /* sticky labels stay above the plate */
```

- **Plate behaviour.** When no row matches, the target gets no fresh write. The active chat may be scoped out, under "Show earlier" collapsed, or not loaded yet. For that case a `$derived` `hasActive` toggles `data-hidden` (opacity 0, no transition).
- **Lever.** `data-marker="plate-accent"` (board 01) adds `::before`, a 2px × 14px accent bar at left 0. It rides inside the plate, so it slides with it.

Dock tile:

```svelte
<button class="fnav-item" type="button" aria-label={def.title} aria-current={on ? "page" : undefined}
        use:tooltip={{ text: def.title, kbd: `Ctrl+${workspace.order.indexOf(id) + 1}` }} onclick={() => goto(id)}>
  <!-- the index comes from workspace.order (the Ctrl+N binding, AppShell:288-289), never the filtered footNav index -->
  <def.icon size={18} />
  {#if dockLabels}<span class="fnav-lbl">{def.title}</span>{/if}   <!-- board lever, off by default -->
</button>
```

**Bell** (NotificationCenter, footer mode):

- `Popover.Root bind:open={toast.centerOpen}`.
- Trigger = dock tile with `aria-label="Alerts"`, the bell icon `{#key bellSwingKey}<Bell class="bell-swing" />{/key}`, and the count Badge `{#key unread}<Badge tone="accent" mono class="count-pop">`.
- `Popover.Content side="top" align="start" sideOffset={8} trapFocus class="w-90 p-0"` holds:
  - a header: title "Notifications" plus a Button ghost sm "Clear" (today's action);
  - a `.menu-rows` list;
  - an EmptyState compact.

## 6. Ordered commits

Each commit is one concern and passes `npm run check` + `npm test` by itself. There is no splash or first-paint commit. D-34 (P1) and M25 (P2) cover both, and §10 #11 checks the hand-off onto the flat canvas. Commit titles follow the repo's `type(scope): summary`. "Logic preserved" lists what must not change.

| # | Title | Files | Change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| S01 | `style(shell): flat islands on a bg-000 canvas` | `AppShell.svelte:304-474`; `Sidebar.svelte:223-232,252-284` | `.app` bg → `--bg-000`, delete the gradient `:352-354` and the `transition: background 1s` `:360`. Delete `.app-body::before` grain `:370-381` and `::after` cavity `:432-444`. `.main` → `bg-100`/`line-100`/`--radius-lg`, delete the slab gradient and 4-layer shadow `:400-411` and grain `::after` `:419-431`. Delete the `.rift-seam` div `:316` and CSS `:445-472`. `.sidebar` same as `.main` (`:258-284`). Peek shadow `:229-232` → `var(--shadow-pop)`. Gutters → `var(--space-3)`. `--island-*` tokens are **not** touched (§13 A-1) | rail layout, `--rail-w` publish, `isolation`, `overflow:hidden`, `position:fixed` chain, peek geometry | merge gated by #1; a one-day flip to B (D-01) | `c.sh tour home chat settings ai-health` + `c.sh diff .sidebar` / `.main`; `measure .main` → bg `rgb(14,16,20)`, radius 16 |
| S02 | `style(shell): intro hand-off on motion tokens` | `AppShell.svelte:493-501`; `Sidebar.svelte:238-243` | 12px → `var(--rise-page)`; `--dur-rise`/`--ease-page` → `var(--dur-slow)`/`var(--ease-out)`; delay 60 → `var(--dur-page-delay)` | `data-intro` phases; 700ms settle (`intro.svelte.ts`) ≥ 600 + 60 | cold-boot feel | `c.sh reload` + shot sequence; `getAnimations()` empty after 700ms |
| S03 | `feat(ui): mount the tooltip layer app-wide` | `routes/+layout.svelte:63`; `routes/dev/ui/+page.svelte` (local mount) | `<TooltipLayer />` beside `<ContextMenuHost />`; remove the `/dev/ui` copy (kit §2.4) | the 174 `use:tooltip` call sites unchanged; aria promotion unchanged | first time tooltips render app-wide; over-tooltip noise | hover a dock tile → `#rift-tooltip` visible at 400ms; `c.sh eval "document.querySelectorAll('#rift-tooltip').length"` ≤ 1 |
| S04 | `refactor(shell): one WindowControls part` | new `shell/WindowControls.svelte`; `Topbar.svelte:2-18,45-49,66-76`; `Titlebar.svelte:6-21,30-40,66-75` | extract; both bars render `<WindowControls variant=…/>`; Titlebar flattens to `bg-000` + bottom `line-100`, drops `box-shadow` `:53` | min/max/close calls, maximized state, aria-labels, tooltips | the onResized listener now lives in one place; check unlisten | onboarding + normal: click each control; `c.sh measure .winctl` |
| S05 | `style(shell): topbar on kit parts` | `Topbar.svelte:22-39,53-65` | `.tb-ic` → IconButton ghost sm `label="Pin sidebar open"`; keep `onmouseenter`/`onmouseleave` peek handlers; `enter` keyframe on `.tb-left` `:57` → `.rise-row` (one shot) | `data-tauri-drag-region` on `.topbar`; `beginPeek`/`schedulePeekClose`/`toggleCollapsed` | the tooltip loses "— hover to peek" (§14 Q4) | collapse, hover `.tb-ic` → peek; click → pinned |
| S06 | `style(sidebar): head, actions, history label` | `Sidebar.svelte:120-159,289-347` | brand `.brand-name` Geist 600 −0.03em `--t-body-size`; `.side-collapse` → IconButton ghost sm; New chat → `<Button variant="secondary" class="flex-1 justify-start">` with Plus 16 `ink-muted`; search → IconButton secondary default `label="Search chats" kbd="Ctrl+K"`; history head → `<SectionLabel count={scopeCount}>Conversations</SectionLabel>` (the `count-note` aria-label `:157` moves onto the count span); resize-handle colours → `line-300`/`accent` | `goHome()`, `commandPalette.show()`, `toggleCollapsed()`, drag region `:123` + `:291`, `scopeCount` | the New chat CTA gets quieter (accent wash → secondary) | `c.sh shot-sel .sidebar` |
| S07 | `feat(nav): "Home" everywhere; Ctrl+1 opens Home` | `workspaceRegistry.ts:31,34`; `Sidebar.svelte:32-35` (`NAV_LABEL` deleted); `ProjectSwitcher.svelte:131`; `CommandPalette.svelte:60`; `AppShell.svelte:288`; `scripts/cdp/c.sh:481,514` | titles → "Home"; "Go to Workspace" → "Go to Home" ×2; Ctrl+1 → `workspace.setActive("home")` instead of `goHome()`; c.sh alias label → "Home" | Ctrl+N = new chat (`:273`); Ctrl+2..9 by `workspace.order`; aria-labels "Chat", "AI Health", "Settings" | **behaviour change** (D-19 flags it); D-33 contract: c.sh in the same commit | `c.sh nav home` lands on Home; Ctrl+1 → Home, Ctrl+N → new chat |
| S08 | `style(sidebar): NavItem dock` | `Sidebar.svelte:165-194,317-330,349-351` | keep `grid-template-columns: repeat(5, minmax(0, 1fr))` (`:320`); each tile fills its column (the hit target), height `var(--control-h)`, `--radius-sm`, icon 18 centred. A fixed `repeat(5, var(--control-h))` would overflow at regular/comfy (5×36 = 180, 5×44 = 220 > ~164px inner width at the 208 rail); active `bg-200` + accent icon; labels removed (lever `data-dock-labels`); `.fnav-item:active` scale → kit `.97` rule; `.snav-ic` hover nudge deleted | `footNav` filter `:19-26`; `aria-label={def.title}`; `aria-current="page"`; NotificationCenter in slot 4 | discoverability without labels (§13 A-2) | `c.sh act click '[aria-label="Settings"]'`; tab through dock → ring on each |
| S09 | `fix(a11y): conversation row is one button` | `ConversationList.svelte:253-320,477-504` | markup per §5.2; `role=button`/`tabindex`/`onkeydown` on the div deleted; `.crow` 32 (`--list-row-h`), padding `0 var(--space-1) 0 var(--space-2)`; title/time/hover tokens; actions → IconButton sm, shown on `:hover`, `:focus-within`, `.menu-open`; queued → Badge neutral mono (accent wash `:436-439` deleted); project tag → Badge neutral (`projectHue` import `:12` and `.crow-proj` hue CSS `:441-449` deleted); pin glyph `ink-faint`; pinned wash `:488-489` deleted; rename input → kit `fieldInput` recipe at 24px | `open()`, drag start/end, rename commit/cancel, `togglePin`, `workingIds`, `liveStarts`, `queuedCounts`, `showAll`, stable sort `:64-109` | Space now also opens a row (native button); drag on a div that holds a button (test it) | keyboard: Tab → row → Enter opens; Tab → pin → Space pins; axe: 0 `nested-interactive`; drag a row to the chat dock |
| S10 | `feat(sidebar): sliding active plate` | `ConversationList.svelte` (list root, CSS `:477-484,523`) | `.crow-plate` + `slidingIndicator` (§5.2); delete `.crow.on::before` and `barPop`; `.crow.on` → `[data-state=active]` (title `ink` 500) | active detection `isActive()`; `[data-first]` cascade (M32) | measuring inside a scrolling list; sticky labels overlap | click 3 rows → `c.sh eval` the plate transform equals the active row `offsetTop`; reduced motion → no transition |
| S11 | `refactor(sidebar): row menu on DropdownMenu` | `ConversationList.svelte:8,175-250` (menu state), `:398-412`, `:506-521,530` | delete `use:portal` menu, the window click/Esc `$effect` `:243-250`, `menuPos`, `:global(.conv-menu*)` CSS and `-global-convPopIn`; per-row DropdownMenu (§5.2) | Pin/Unpin, Rename (`startRename`), Delete (`del`, no confirm, as today) | Rename: the close-auto-focus would blur the input and commit at once. Prevent it while `renaming` is set | open menu → arrow keys → Rename → input keeps focus; Esc → focus back on the trigger |
| S12 | `style(sidebar): group labels + show earlier` | `ConversationList.svelte:333-395,414-433,451-466` | group labels → SectionLabel with `count`, sticky with `background: var(--bg-100)` (the gradient `:463` is deleted); show-more `.sm-ch` → `.chev`; `.sm-ct` → Badge neutral mono; empty → EmptyState compact "No conversations yet", description "Start one from Home or Chat." | `{#key showAll}`, `HISTORY_BATCH`, `aria-expanded`/`aria-controls` | sticky + plate z (S10 sets z 2) | expand / collapse earlier; scroll a long list, labels stick |
| S13 | `style(sidebar): project switcher trigger` | `ProjectSwitcher.svelte:16,28,155-181`, CSS | trigger `calc(var(--control-h) + 8px)`, `bg-100` + `line-100` r12 → hover `bg-200`; monogram 24 `bg-300`/`ink-muted` r6 (hue deleted); name `t-small` 500 `ink`; branch → Badge neutral mono; caret → `.chev` | project resolution, `activeProject`, branch read | the hue identity cue goes (D-02 one voice) | `c.sh shot-sel .sidebar`; switch projects |
| S14 | `refactor(sidebar): project menu on bits-ui` | `ProjectSwitcher.svelte:119-143,184-224`, CSS `:260-286`; `DeleteProjectDialog.svelte` | `.sw-menu` portal → DropdownMenu (items `menuItem`, monogram as icon slot, `menuLabel` heading); right-click items `:119-143` → ContextMenu on the trigger with the same items; **if spike 1 fails** keep the bespoke list inside `DropdownMenu.Content` with the HTML5 drag handlers on each item | project reorder by drag, open/delete/"Go to Home", DeleteProjectDialog trigger | bits-ui roving focus vs `draggable` items (spike 1, §14 Q2) | reorder 2 projects by drag; keyboard open → arrows → Enter; Esc returns focus |
| S15 | `refactor(shell): kit Skeleton in the shell` | `ConversationList.svelte:13-14,22-31,323-331,468-475`; `ProjectSwitcher.svelte:17,19,146-153`; `StatusBar.svelte:14-15,124-…`; `WorkspaceShell.svelte:96-108,149-191` | `<Skeleton loading={bootLoad.showSkeleton} variant="text" lines={6}>` around each region (the `loading` source stays `bootLoad`); WorkspaceShell load state → `<Skeleton loading variant="text" lines={4}>` + error copy `t-small`/`ink-muted` + `<Button size="sm">Try again</Button>`; `.ws-skel-card` (uses `--island-fill` `:171`) deleted | `bootLoad` timing, `retryLoad(id)`, `aria-busy`, `role="alert"` on the error | Skeleton's 300ms delay stacks on bootLoad's own gate (§14 Q5) | throttle the first load in DEV: bones at ≥300ms, hand-off 600ms, no sweep |
| S16 | `style(shell): status bar on tokens` | `StatusBar.svelte` CSS (`:206`, `:240`, type) | top border `line-100`; type `--t-caption-*` `ink-faint`; `.sb-dot` `--ok` → `--accent` (D-03); LimitBar compact keeps its own fills (M17 already de-pulsed) | drag region `:123`, media queries `:266-278`, every readout | none | `c.sh shot-sel` status bar at 1280 and 900 widths |
| S17 | `refactor(shell): bell on Popover with a focus trap` | `NotificationCenter.svelte` (all of `:100-421`); `styles/severity-tone.css` import `:14` | §5.2 bell; rows: tone dot 8 (`toneOf`), title `t-small` `ink`, detail `t-caption` `ink-muted` 2-line clamp, time `t-caption` `ink-faint`; `.menu-rows`; `bell-swing` re-keyed when `toast.history.length` grows while closed; `count-pop` re-keyed on `unreadCount` | `toast.history`, `centerOpen`, `unreadCount` reset-on-open, "Clear", `label`/`footer` props | upward popover from a bottom dock near the window edge: `collisionPadding` 12 | Tab into the bell → Enter → focus inside; Tab cycles inside; Esc → focus on the bell; axe dialog checks pass |
| S18 | `refactor(shell): Toaster replaces ToastHost` | `AppShell.svelte:332`; `ToastHost.svelte:18-20` → `ui/toast/toaster.svelte`; `styles/severity-tone.css` | mount `<Toaster />`; move the `__riftToast` DEV hook verbatim (behind `import.meta.env.DEV`); delete `severity-tone.css` (both consumers are gone after S17-S18; grep = 0) | `toast.push`/`notify.*` call sites; top 54; `--rail-w` centring | two hosts if a step is skipped; the K34 sticky-danger change is a separate kit commit | `c.sh eval "window.__riftToast?.('ok','Saved')"`; hover the stack → fans, timers pause |
| S19 | `refactor(shell): UpdateBanner inside the main island` | `AppShell.svelte:311-313` (move under `<Topbar />` at `:318`); `UpdateBanner.svelte:158-332` | rows `:71-155` render as `<Collapse open={rows.length > 0}><div class="ub-stack">{#each rows}<Banner tone={row.kind === "app" ? "accent" : "info"} …>`; Progress for download; dismiss → IconButton | row derivation, install/restart/dismiss handlers, CLI vs app split | layout: the banner now pushes the page inside the island, not the whole window | trigger the DEV update row; `measure .sidebar` unchanged while the banner shows |
| S20 | `refactor(shell): destructive dialogs default to Cancel` | `CloseConfirm.svelte:116-119`; `DeleteProjectDialog.svelte` | `<AlertDialog.Content destructive>`; delete the manual focus code; body copy `t-small` `ink-muted`; actions `buttonVariants` danger + secondary | close/minimise-to-tray choices, remember-choice, delete-project call | none (K24 lands first) | open each → focus on Cancel; Enter cancels |
| S21 | `style(shell): context menus on recipes` | `ContextMenuHost.svelte` | delete local surface/row CSS so `floatingPanel`/`menuItem` apply; danger items → `variant="danger"` | the registration API and item handlers | none | right-click a row and a project |
| S22 | `style(shell): literal sweep` | the 17-file set (§1.2) | remaining `font-size: Npx` → `--t-*`; px radii → `--radius-*`; `oklch(`/accent `color-mix(` → tokens; raw `cubic-bezier` → tokens (M39 leftovers) | — | missed values on rarely seen states | §10 counts |

S01 is the only commit gated by the owner. If the answer to #1 is B, S01 is replaced. In B the sidebar sits on `bg-000` with `border-right: 1px solid var(--line-100)` and no gutter, the main island loses its radius, and the Topbar gets a bottom border. S02-S22 survive unchanged.

## 7. Contracts (D-33)

| Contract | Consumers | P4a action |
|---|---|---|
| `.sidebar` | `c.sh measure/baseline/diff` (`c.sh:307,403-404`), `rift-ui/SKILL.md:56`, 01/03 verify lines | kept on the `<aside>` |
| `.app`, `.ob-host`, `document.documentElement.dataset.mode` | `c.sh ready` `:543-549` | kept |
| `[aria-label="Settings"]` | `SKILL.md:57`, `c.sh nav/tour` | kept on the dock tile. **Make sure nothing else in the shell gets `aria-label="Settings"`** |
| `[aria-label="Chat"]`, `[aria-label="AI Health"]` | `c.sh nav/tour` | kept (the registry titles) |
| `[aria-label="Workspace"]` → `"Home"` | `c.sh:481,514` | renamed in S07, the same commit as the registry |
| `window.__riftToast` (DEV) | manual QA, gallery | moved verbatim in S18 |
| `--rail-w` (registered `<length>`, published by `Sidebar:45-47`) | Toaster placement, ToastHost today | kept; M31 already ties its tween to the rail |
| `data-intro` on `.app` | `intro.svelte.ts`, 03 §7 | kept |
| `.side-rail.collapsed` / `.peeking` | 03 M31 CSS | kept |
| `.crow`, `.conv-list`, `.fnav-item` | none outside their own file (grep of `c.sh` and `.agents/skills/rift-ui`: 0 hits) | free to restructure, but the names are kept anyway |
| `.conv-menu`, `.sw-menu` | none outside their file | deleted |
| Peek `top: 54px` (`Sidebar:228`) and toast `top: calc(var(--titlebar-h) + 20px)` = 54 | geometry | **Coupling candidate.** `--titlebar-h` is 34 but the Topbar is 40 (`Topbar:54`), so the two 54s agree by coincidence. Not solved here (§14 Q3) |
| Resize maths `clientX + 12` (`Sidebar:76`) and `width - 24` (`:118`) | gutter | if a board moves the gutter, these become `var(--space-3)` readers (`getComputedStyle`) or a shared `GUTTER` const; P4a keeps 12 |

## 8. Deletions

### 8.1 Gated by shortlist #6 (owner sign-off; one commit each, after P4a lands)

- The registry `kbd` field (`workspaceRegistry.ts:25` type and `:31-37` values): no consumer, and the values are wrong (D-19). CommandPalette builds its own `Ctrl+${idx+1}` (`:72`).
- None else in the shell. The global `.island` / `.island-float` (`app.css:502-510`) has 0 markup consumers (grep `class="island` = 0), but no map lists it as dead (map 06 `:287` only calls it shared). So it goes under §8.2.

### 8.2 Grep-to-zero (deleted in the commit where the count reaches 0; not owner-gated)

| Target | Consumers today | Count after P4a | Lands |
|---|---|---|---|
| `styles/severity-tone.css` | ToastHost `:12`, NotificationCenter `:14` | 0 | S18 |
| `components/ToastHost.svelte` | AppShell `:332` | 0 | S18 unmounts it; the file follows kit §8.1 (P5). Delete it in S18 if the reviewer prefers. No other importer |
| `shell/Skeleton.svelte` | ConversationList `:13`, ProjectSwitcher `:17`, StatusBar `:14`, AssistantWelcome `:19` | 1 (AssistantWelcome, 4b/4d) | the phase that ports AssistantWelcome |
| `state/bootLoad.svelte.ts` | the same 4 files | 4 (still the `loading` source) | kept; the kit Skeleton consumes it |
| `utils/projectHue.ts` (+ test) | ConversationList, ProjectSwitcher, WorkspacePage `:24` | 1 (WorkspacePage, 4d) | 4d |
| `--island-radius/-border/-fill` (`app.css:210-212`) | Sidebar, AppShell, NotificationCenter, AssistantWelcome, SettingsPage, NewsFeed, WorkspacePage ×3, WorkspaceShell | 6 (non-shell) | 4d/4e |
| `.island` / `.island-float` (`app.css:502-510`) | 0 markup consumers | 0 | any commit; S01 is the natural one |
| `RiftLogo` | — | kept (D-26) | — |

## 9. Board 01 — Frame A vs B (+ tab strip)

| Field | Value |
|---|---|
| Board id | `01` "Frame", project "Rift · Halo" in Claude Design, Halo installed, Electric blue (D-25) |
| Frame | 1440 × 900 at compact, with a second 1280 × 800 artboard for the narrow check. The canvas is `bg-000` |
| Contents | **A** islands-flat: the full shell at true size with canned data. **B** Halo flat docked frame (sidebar on `bg-000` + `border-right line-100`, main flush, Topbar bottom border). **C** today's slab as a 25% thumbnail in the corner, reference only (map 12 X2/C6 "beside the old look") |
| Canned data | 14 conversations in Pinned (2) / Today (4) / Yesterday (4) / Previous 7 Days (4); one live (`.live-dot` + `1:42`), one with 2 queued, All-projects tags on 3; the switcher on `rift-tauri` · `main`; 3 unread; one app update banner; one toast; the status bar with model, branch and limit |
| Play-mode interactions | click rows (the plate slides); collapse/expand (one move); hover the topbar trigger (peek); open the bell (popover, focus trap); push a toast (stack, fan on hover); show/hide the banner; open the row ⋯ menu; switch density; A/B toggle (same data in both frames, one click) |
| Levers (data-props) | `data-prop-frame="A,B"` · `data-prop-gutter="8,16,12,2"` · `data-prop-island-r="12,20,16,2"` · `data-prop-row-h="28,40,32,2"` · `data-prop-marker="plate,plate-accent,bar"` · `data-prop-dock="icons,labels"` · `data-prop-collapse-ms="240,300"` (ties to #3) · `data-prop-brand-size="13,15,15,2"` · `data-prop-tabstrip="off,on"` · `data-prop-density="compact,regular,comfy"`. The board script builds its rail from these (the 03 §9 pattern) |
| Tab-strip question | `tabstrip=on` overlays a 36px open-chat strip at the top of the main island (the mock's look: `bg-100`, active tab `bg-200` + 2px accent underline, ×, max 6). It only draws, with no state. D-19 default: off (owner rejection L10). The board asks "still no?" |
| Decisions it closes | #1 frame A/B · tab strip yes/no · marker style · dock labels · brand size · collapse 240 vs 300 (recorded under #3) |
| Exported numbers | gutter → `--space-3` use in S01 (and the resize maths in §7) · island radius → `--radius-lg` vs a shell override · `--list-row-h` compact · marker variant → `data-marker` default · dock mode → `data-dock-labels` default · brand size → S06 · collapse ms → `--dur-base` (shortlist #3) |

## 10. Verification

**Commands** (Windows dev machine): `npm run check`, `npm test` and `npm run check:ui` on every commit. `npm run verify` before merge. No Rust changes, so cargo stays as P1 left it.

**c.sh verbs** (`bash scripts/cdp/c.sh …`, app running under the CDP bridge):

1. `ready` → `mounted:true`, `onboarding:false`.
2. `tour home chat settings ai-health` → 4 shots and 0 click failures. This proves the "Home" alias after S07.
3. `measure ".sidebar" nokids` and `measure ".main" nokids`:
   - background `rgb(14, 16, 20)`, radius 16px, border `1px rgb(31, 36, 44)`;
   - box-shadow `none` (pinned) and `shadow-pop` (peek).
4. `diff ".sidebar"` after each sidebar commit, against a P3-end `baseline ".sidebar"`.
5. `act click '[aria-label="Collapse sidebar"]'`, then `measure .sidebar`. `--rail-w` and the margin end together. There is no Ctrl+B; see §14 Q1.
6. `eval` counts:
   - `document.querySelectorAll('[role=button] button, [role=button] [role=button]').length` = 0;
   - `document.querySelectorAll('#rift-tooltip').length` ≤ 1;
   - `document.querySelectorAll('.rift-seam').length` = 0;
   - `getComputedStyle(document.querySelector('.app')).backgroundImage` = `none`.
7. `act key "Ctrl+1"` (c.sh's key syntax, `c.sh:17,285`) → `workspace.activeId === "home"`. `act key "Ctrl+N"` → new chat.
8. `media reduce`, then repeat 5 and S10. No transitions run. The plate jumps.
9. `inspect` → 0 console errors on every surface.
10. Keyboard pass (manual), as listed in S09, S11, S14, S17 and S20.
11. Splash and first paint: `reload`, then the cold-boot shot sequence (as in M25).
    - Splash panels, `.app` and the pre-JS `app.html` background all read `rgb(8, 9, 12)`, with no colour step at the hand-off.
    - The islands land with S02's values.
    - `document.getAnimations().length` = 0 by 700ms after `data-intro` clears.

**Counts that must go down** (the 17-file set, 4017 lines at `22727d24`; P1/P2 lower some first, and the targets are absolute):

| Pattern | Today | After P4a |
|---|---|---|
| `font-size: Npx` | 31 | 0 (SplashOverlay keeps its letter sizes under `/* motion-literal */`-style allow comments: ≤ 4) |
| `oklch(` | 36 | ≤ 6 (SplashOverlay) |
| `color-mix(` | 83 | ≤ 10 (Banner/Badge tone mixes live in the kit, not here) |
| `backdrop-filter` | 10 | 0 (M37 + S11/S14/S17) |
| `cubic-bezier` | 8 | 0 (SplashOverlay → `--ease-door`) |
| `z-index:` ≥ 10, raw | 6 | 0 (P1 tokens; bits-ui content via recipes) |
| `use:portal` | 3 | 0 |
| `role="menu"` / `role="dialog"` hand-rolled | 3 | 0 |
| `@keyframes` | 19 | ≤ 5 (SplashOverlay only) |
| `infinite` | 6 | 0 (LiveDot/Spinner are global classes) |
| `border-radius: Npx` | 43 | ≤ 6 (SplashOverlay, glyph pixels in WindowControls) |
| `transition:` | 57 | ≤ 25 (tokens only; `motion:raw-duration` clean) |

## 11. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| The owner picks B (or C) at board 01 | medium | S01 rewritten | S01 is isolated; S02-S22 do not depend on islands |
| The sliding plate mis-measures in a scrolled list, or under sticky labels | medium | plate off by the scroll offset | measure `offsetTop` relative to `.conv-list` (the positioned scroll container); a unit test in `indicator.test.ts` with a scrolled fixture; `data-hidden` when there is no active row |
| bits-ui DropdownMenu vs draggable project items | medium | reorder breaks | spike 1 before S14; the fallback is listed in S14 |
| Rename from the menu loses focus and commits an empty title | high if missed | data change | `onCloseAutoFocus` preventDefault while `renaming`; the S11 verify covers it |
| `--island-*` flip would restyle Settings, the Home cards and NewsFeed early | avoided | — | §13 A-1: the shell uses `--bg-100`/`--line-100`/`--radius-lg` directly; the tokens stay for 4d/4e |
| An icon-only dock reads as less discoverable (it used to be labelled) | medium | owner pushback | tooltips app-wide from S03; board lever; aria-labels unchanged |
| Upward bell popover is clipped by the window bottom or left edge | low | panel cut off | `collisionPadding={12}`, `avoidCollisions` |
| The tooltip layer goes live for 174 sites at once | medium | noisy hovers | D-17 skip rules (`repeatsVisibleText`, `.wb-stage` avoid); `tooltip={false}` on IconButtons whose label is visible text |
| Banner inside `.main` shifts the chat scroll anchor | low | jump on update | the chat pane re-pins to bottom on resize (FLIP code untouched); verify with a long chat |
| Drag regions: `data-tauri-drag-region` + `-webkit-app-region` both present (`Sidebar:123,291`) | low | dead drag strip or swallowed clicks | keep both as today until D-28 spike #4; every new button sets `no-drag` (IconButton inside `.sidebar button` rule `:289`) |
| Ctrl+1 change surprises muscle memory | low | wrong page | flagged in S07's message; Ctrl+N unchanged |

## 12. Effort

| Block | Commits | Size | Session |
|---|---|---|---|
| Frame, intro, tooltip mount | S01-S03 | S | 1 |
| Chrome + topbar | S04-S05 | S | 1 |
| Sidebar head, Home, dock | S06-S08 | M | 1 |
| Rows, plate, menu, labels | S09-S12 | L | 2 |
| Switcher | S13-S14 (+ spike 1) | M | 2 |
| Skeleton, status bar | S15-S16 | S | 2 |
| Bell, toasts, banner, dialogs, context | S17-S21 | M | 3 |
| Sweep + verification pass | S22 + §10 | S | 3 |

**Total: L, 2-3 sessions** (2 if spike 1 passes and S14 is a straight port). Prerequisites: P1, P2 (M11, M14, M23, M25, M30-M32, M37, M38) and P3 (K05-K11, K13, K17-K20, K24, K27, K30-K34b, K04b).

## 13. Amendments proposed to 00-decisions (and specs 01-03)

- **A-1 (01 A-7, D-01).** 01 says P4a flips `--island-radius` to 16 and `--island-fill` to `bg-100`. Instead the shell islands read `--bg-100`/`--line-100`/`--radius-lg` directly, and `--island-*` stays for 4d/4e to retire with their cards. Flipping the token would restyle six non-shell sites (AssistantWelcome `:500-510`, SettingsPage `:1666,1689`, NewsFeed `:270,293`, WorkspacePage `:853,1110,1183`) a phase early.
- **A-2 (D-19 "Home everywhere (tile, …)", 00 :173 board-00 dock-label check).** Dock tiles go icon-only by default, with D-17 tooltips and unchanged aria-labels. At the D-06 floor of 12px, five labelled tiles do not fit:
  - at rail 248 a tile is about 39px wide, and about 31px at the 208 minimum;
  - "Settings" alone needs about 45px.

  The "Home" name still lands everywhere: tooltip, aria-label, registry, palette and c.sh. Board 01 carries a `labels` lever. Labels would need a wider minimum rail or a two-row dock.
- **A-3 (D-19 marker).** The plate is `bg-200` + inset `line-200`, with no accent bar (ConsoleX pattern, `Sidebar.svelte:157-161,350-365` in ConsoleX). D-19 names "active = bg-200 + accent icon" for NavItems, and rows have no icon. `plate-accent` is the board lever.
- **A-4 (02 §2.3, §2.2 Banner).** The kit places the ToastHost → Toaster swap and the UpdateBanner chrome in 4f (`02-kit:886`, `:476`), but K34b and the D-20 scope put them in P4a. This spec does both in P4a (S18, S19).
- **A-5 (02 Popover width).** The bell uses `w-90` (360) over the kit's 300 (`w-75`). At 300, a notification row (dot, title, 2-line detail, time) wraps the detail to 3 lines. Halo's notification panel is wider than its generic popover (`bundle.css:345-383`). This is a per-instance class, not a kit change.
- **A-6 (02 Skeleton).** The shell uses `variant="text"`. If 4d needs title-only list bones too, add a `rows` variant (bars only, `--list-row-h` pitch) rather than reuse `list`.
- **A-7 (03 row 9).** Claim the intro hand-off retune (12px → `--rise-page`, `--ease-page` → `--ease-out`) for P4a S02. No M commit touches `AppShell:494-501` or `Sidebar:238-243`.
- **A-8 (03 M31 verify, §10.2 #9).** Replace `c.sh act key "Ctrl+B"` with `act click '[aria-label="Collapse sidebar"]'`. There is no Ctrl+B binding (§14 Q1).
- **A-10 (02 K04b / §2.5 `indicator.ts`).** `slidingIndicator` must measure the active child's `offsetTop`/`offsetHeight` against the positioned list (the scroll container), not `getBoundingClientRect` deltas. Rect deltas shift by `scrollTop`, which would put the sidebar plate off by the scroll offset. Add a scrolled-list fixture to `indicator.test.ts`.
- **A-9 (D-26 brand head).** The size is `--t-body-size` 15 at Geist 600 −0.03em (today 14/650). D-26 names the weight and tracking only.

## 14. Open questions (each has a default)

1. **Add a Ctrl+B sidebar toggle?** None exists; 03 M31 assumed one, and D-19 does not list it. **Default: no.** A new shortcut is a behaviour change. Offer it on board 01.
2. **Does bits-ui DropdownMenu tolerate HTML5 `draggable` items** (ProjectSwitcher reorder)? **Default: spike first (30 min). If it fails, keep the bespoke item list inside `DropdownMenu.Content`, with the drag handlers and `menuItem` classes.**
3. **One `--peek-top` var for the peek (`Sidebar:228`) and the toast stack (54px)?** **Default: no change in P4a. Record it as a D-33 coupling and solve it with Topbar height work.**
4. **The Topbar trigger's tooltip loses "— hover to peek".** **Default: accept.** Hovering already peeks, so the hint describes what the user is seeing.
5. **Kit Skeleton's 300ms delay on top of bootLoad's own gate.** **Default: pass `delay={0}` from shell consumers, so bootLoad stays the single timing source,** until bootLoad retires.
6. **Density-tie the StatusBar height?** **Default: no.** It is a readout strip and stays as is in P4a; 4f revisits it.
7. **Delete `ToastHost.svelte` in S18 or in P5?** **Default: P5, per kit §8.1.** S18 only unmounts it. The reviewer may fold the delete into S18, since there is no other importer.
8. **Row delete without confirm** (today's behaviour)? **Default: keep it.** Changing it is a behaviour change outside D-19/D-20.

## Map corrections

- 02-kit §2.2 Skeleton names one shell consumer for `shell/Skeleton.svelte`. There are 4: ConversationList `:13`, ProjectSwitcher `:17`, StatusBar `:14`, AssistantWelcome `:19`.
- 02-kit §2.4 "Mount points" puts `<ContextMenuHost />` at `+layout.svelte:62`. It is at `:63`.
- 03-motion M31 and §10.2 #9 verify with `Ctrl+B`. No such binding exists (grep for `key === "b"` / `KeyB` / `Ctrl+B` in `src`: 0).
- 03-motion row 9 (intro hand-off retune) has no owning M commit. M30 stops at `AppShell:493` and M31 touches `:221,274,417`.
- Keyframe names differ between specs. 00 D-20 and 02 §9 say `halo-zoom`; 03 §2.2 defines `halo-zoom-in`/`halo-zoom-out`. Use 03's.
- 02-kit's K04b row says `measureIndicator(list, active) → {x,w}`, while 02 §2.5 says `slidingIndicator(list, opts)` writes `--ind-*` vars. P4a uses §2.5.
- 02-kit uses type tokens like `var(--t-caption)`/`--t-label-lh`. 01 defines `--t-caption-size/-line` and `--t-label-size/-line`. P4a uses 01's names.
- `ConversationList.svelte:253-267`: `.crow` is a `div role="button" tabindex=0` that contains `<button>`s, a nested-interactive a11y defect not recorded in 00-03. It is fixed in S09.
- The registry titles both `home` and `projects` "Workspace" (`workspaceRegistry.ts:31,34`), while Sidebar's `NAV_LABEL` already says "Home" (`:33`). The aria-label and the visible label disagree today.
- `ProjectSwitcher.svelte:131`, the second "Go to Workspace" string, is in the right-click item list, not the menu.
- Ctrl+1 today runs `goHome()` (new chat, `AppShell:288`). It does not open the Home page.
