---
title: "P4f — AI Health, Diagnostics, overlays (palette, update, MCP, browser)"
status: draft
updated: 2026-10-19
phase: P4f
depends_on: [00-decisions, 01-foundation, 02-kit, 03-motion, 10-shell, 11-chat, 12-composer, 13-home-onboarding, "P2 (M11 M12 M14 M20 M21 M23 M30 M33 M34)", "P3 (K05-K09 K23c K24 K25 K26 K27 K28 K31 K32)", "P4a (Toaster + NotificationCenter)", "P4b (ui/code-block)"]
---

# 15 — AI Health, Diagnostics and overlays (P4f)

## 0. Summary

P4f moves the last bespoke surfaces onto the kit: AI Health (Claude + ChatGPT), the Diagnostics console, the Command palette, the Update dialog, the MCP servers dialog and the browser dock.
The one real engineering piece is the **overlay registry wiring**: the native WebView2 child (`browser_show`/`browser_hide`) must hide whenever any modal or floating layer overlaps its stage, decided by one `$derived` instead of the current ad-hoc effect, and it must not cancel an in-flight `open_probed` probe.
Everything else is token/primitive substitution with logic frozen: no new IPC, no Rust change, no store-shape change.
Native `confirm()` from `@tauri-apps/plugin-dialog` is replaced by a kit `AlertDialog` host (`state/confirm.svelte.ts` + `shell/ConfirmHost.svelte`), owned by whichever of 4e/4f lands first.
~40 commits (F00–F40), effort L, 5–6 sessions. Seven amendments to 00-decisions/02-kit/03-motion, listed in §13.

## 1. Scope and files

Line counts measured 2026-10-19 on `design/chat-ledger` (22727d24). `path:line` anchors are pre-P2; P2/P3 commits shift them — re-anchor by symbol, never by number.

| File | Lines | What changes in 4f |
|---|---|---|
| `src/lib/components/dialogs/CommandPalette.svelte` | 529 | Kit `Dialog size="palette"`; drop Tab trap :248-265; rows on `--list-row-h`; group headings `menuLabel`; ARIA combobox/listbox |
| `src/lib/state/command-palette.svelte.ts` | 35 | Unchanged API; nav entries read titles from `WORKSPACES` |
| `src/lib/components/dialogs/UpdateDialog.svelte` | 562 | Kit `Dialog` 480; `Progress`; `Banner`; `Accordion`+`CodeBlock` tech details; drop Tab trap :39-55; busy ⇒ Esc/outside ignored |
| `src/lib/state/updates.svelte.ts` | ~310 | `:287` native `confirm` → `state/confirm.svelte.ts` (logic identical) |
| `src/lib/state/cliUpdate.svelte.ts` | ~450 | `:432` native `confirm` → `state/confirm.svelte.ts` |
| `src/lib/components/dialogs/McpServersDialog.svelte` | 541 | Kit `Dialog size="lg"`; `Field`/`Input`/`Select`; `Badge dot`; `Banner` remove-confirm; `EmptyState`; `Skeleton`; window capture Esc handler :111-129 kept but gated on `overlays.floating.size===0` |
| `src/lib/components/diagnostics/DiagnosticsConsole.svelte` | 512 | `--dc-*` (:326-331) → tokens; `ROW_H` from `--list-row-h`; source filter → `Combobox`; actions → `PageHeader`; errors → toast; modal branch frozen |
| `src/lib/components/diagnostics/DiagnosticsPage.svelte` | 29 | `PageHeader` wrapper; `.page-sections` |
| `src/lib/components/ai-health/AiHealthPage.svelte` | 1520 | `PageHeader` 1080; grid areas; `StatTile`/`Progress`/`ShareBar`; tone mappers; dead CSS flagged |
| `src/lib/components/shared/LimitBar.svelte` | 82 | **Untouched.** AI Health stops importing it (kit `Progress` instead); `UsagePanel:10` and `StatusBar:11` still import it (their phases migrate them) |
| `src/lib/components/shared/PageHero.svelte` | 48 | **Untouched.** AI Health (:10) and DiagnosticsPage (:5) stop importing it; `SettingsPage:33` still does until 4e |
| `src/lib/components/ai-health/healthFormat.ts` | new | Tone mappers `zone/mcpTint/healthScore/statusMeta/HealthLevel → Tone` + test |
| `src/lib/components/webview/WebBrowserPage.svelte` | 979 | `nativeVisible` `$derived` + one `$effect` (replaces :371-379); veil; `BrowserBar` extraction; find bar `SearchField`; 16 `title=` → tooltip |
| `src/lib/components/webview/BrowserBar.svelte` | new | URL bar + nav buttons + overflow `DropdownMenu` extracted from WebBrowserPage |
| `src/lib/state/browserDock.svelte.ts` | 100 | Unchanged API |
| `src/lib/state/overlays.svelte.ts` (02-kit §2.4b, K08) | — | Adds `moved` counter, `notifyMoved()`, `overlapsRect(rect)` |
| `src/lib/actions/overlay.ts` | new | `use:overlay` action for bespoke floating sites |
| `src/lib/state/confirm.svelte.ts` | new | Promise-returning `confirm(message, opts)` FIFO |
| `src/lib/components/shell/ConfirmHost.svelte` | new | One kit `AlertDialog`, mounted in `AppShell` |
| `src/lib/components/assistant/AssistantPage.svelte` | 450 | Publishes `--dock-w` on `document.documentElement` (px; `0px` when closed / on destroy) |
| `src/lib/components/AppShell.svelte` | ~340 | Mount `ConfirmHost`; Toaster `left` calc reads `--dock-w`. The toast host (:332) is a sibling of `WorkspaceShell` (:320), not inside `.assistant-page`, so the var must live on `:root` to inherit |
| `src/lib/components/shell/WorkspaceShell.svelte` | ~150 | `.ws-page[data-workspace="chat"]` glide = opacity only (amend 03 §2.4) |
| `src-tauri/src/browser/mod.rs` | — | **Read-only.** OPEN_GEN :294-313, `open_probed` :151-196 — reason for `navPending` deferral |
| `scripts/check-ui.mjs` / `scripts/ui-baseline.json` | — | Baseline counts down (§10) |
| `scripts/cdp/c.sh` | — | `nav diagnostics` case (:472-521 block) |

Out of scope: Settings `confirm` sites (`Settings:337,354` → 4e), the Toaster itself and `ToastHost` swap (4a), `ui/code-block` (4b), Rust.

### 1.1 Preconditions (must have landed before F00)

| Source | Item | Why 4f needs it |
|---|---|---|
| P2 M11 | `rm()` + zoom carriers on Palette/Update/MCP/Diag; `dockSlide` → `dockWidth` 300 | Motion tokens these files consume |
| P2 M12 / M14 | Spinner unification; live-dot | Browser load, MCP status |
| P2 M20 / M21 | Shimmer killed, `.indet-bar` | Browser load bar, Diag refresh |
| P2 M23 | Skeletons | MCP list loading, AI Health first load |
| P2 M30 | `.page-sections`, `.ws-page` glide | AI Health / Diagnostics page entry |
| P2 M33 / M34 | Transform cleanup | Browser dock |
| P3 K05–K09 | recipe state fix (K05), menus/Select, Dialog sizes sm/default/lg/palette/drawer (K07), overlay registry `state/overlays.svelte.ts` (K08), Popover (K09) | Every dialog in this phase |
| P3 K10–K23 | Button (`soft`), IconButton (required `label` → tooltip), Card, Badge (tones neutral/accent/ok/warn/danger/info, `dot`), Kbd, Spinner, EmptyState (`compact`), Banner, Skeleton, SearchField (K22), Field/Input | Every surface |
| P3 K23c | Combobox | Diagnostics source filter |
| P3 K24 / K30b | AlertDialog `destructive` → focus Cancel (K24); Accordion (K30b) | ConfirmHost, MCP remove; Update tech details, AI Health advice |
| P3 K25 / K26 / K27 / K28 | StatTile, ShareBar, Progress, PageHeader | AI Health, Diagnostics |
| P3 K31–K32 | Tooltip | 21 `title=` replacements |
| P4a | Toaster mount + NotificationCenter, dock-tile removal | Action errors → toast; `severity-tone.css` importers drop |
| P4b | `ui/code-block` (D-18) | Update tech details. **Not in 02-kit** — 4f does not build a parallel one |

If any precondition is missing at F00, stop and land it in its own phase; 4f does not backfill kit parts.

## 2. Target design

Values resolve at **compact** (the default density); regular / comfy follow in brackets where they differ. Density vars (01 §2.7, 02-kit §1.5 A): `--list-row-h` 32 (36/40), `--control-h` 32 (36/44), `--control-dense-h` 28 (32/36), `--control-r` 8 (12/12), `--card-pad` 16 (20/24), `--palette-input-h` 44 (48/52), `--badge-h` 20 (22/24). Dialog widths: `--dialog-w` 480, `--dialog-w-sm` 400, `--dialog-w-lg` 560, `--palette-top` 12vh, `--dialog-max-h` 100vh−32. Radii: `--radius-sm` 8; dialog/card surfaces use the Tailwind `rounded-lg` key from the 02-kit recipes (02-kit A-6: the key differs from the CSS var `--radius-lg` 16, unused here). Type: `--t-label` 11/14 500 caps, `--t-small` 13/18, `--t-body` 15/24, `--t-title` 24/30, `--t-metric` 28/32. Surfaces `--bg-000` (canvas), `--bg-100` (card/dialog), `--bg-200` (hover/active row), `--bg-300` (kbd chip). Lines `--line-100` / `--line-200`. Text `--fg` / `--fg-muted` / `--fg-subtle` (L 0.600).

### 2.1 Command palette

```
            ┌──────────────── 560 ────────────────┐   top: 12vh
            │ ⌕  Go to a page, run a command…  Esc │   --palette-input-h 44 (48/52)
            ├──────────────────────────────────────┤   --line-100
            │ NAVIGATE                              │   menuLabel (--t-label 11/14 500 caps, fg-subtle)
            │ ▣  Chat                       ⌘1     │   row --list-row-h 32 (36/40)
            │█▣  AI Health                  ⌘4     │   active: --bg-200, icon --accent
            │ ▣  Go to Diagnostics                 │
            │ ACTIONS                               │
            │ ↻  Check for updates                  │
            ├──────────────────────────────────────┤
            │ ↑↓ move   ↵ run   esc close          │   footer 36, Kbd chips --bg-300
            └──────────────────────────────────────┘   max-height 64vh, rounded-lg, w --dialog-w-lg
```

| Element | Token | Compact | Regular | Comfy |
|---|---|---|---|---|
| Dialog width | `size="palette"` → `--dialog-w-lg` | 560 | 560 | 560 |
| Top offset | `--palette-top` | 12vh | 12vh | 12vh |
| Input height | `--palette-input-h` (`paletteInput` recipe) | 44 | 48 | 52 |
| Input text | `--t-body` | 15/24 | 15/24 | 15/24 |
| Row height | `--list-row-h` (`paletteRow` recipe) | 32 | 36 | 40 |
| Row radius | `--control-r` | 8 | 12 | 12 |
| Active row | `--bg-200` | — | — | — |
| Group heading | `menuLabel` recipe | 11/14 500 .12em caps, `fg-subtle` | same | same |
| Footer | `paletteFooter` recipe, `Kbd` (20) | 36 | 36 | 36 |
| Scrim | `--scrim` (Halo, `rgba(4,5,7,.72)`) | — | — | — |

Behaviour preserved: fuzzy filter, arrow/Enter on the input, pointer-move sets active, scroll-into-view, focus restore (kit Dialog does it — drop the bespoke restore). Navigation rows are **not** built from `workspace.order`; labels read `WORKSPACES[id].title` so a rename lands once. "Go to Diagnostics" stays (it is the only route to that page besides AI Health).

### 2.2 Update dialog

```
        ┌─────────────── 480 ───────────────┐
        │ Update available              ✕   │  Dialog title (K07 dialogSurface recipe)
        │                                    │
        │ 0.42.0            (was 0.41.3)     │  --t-metric 28/32 tabular, fg; old fg-subtle
        │ Released 2 days ago                │  --fg-muted
        │ ▓▓▓▓▓▓▓▓▓▓▓░░░░░░░  58 %          │  Progress md (6px), only while downloading
        │ ⚠ Download failed: …   [Retry]     │  Banner tone=danger, only on error
        │ ▸ Technical details                │  Accordion → CodeBlock (release notes / log)
        ├────────────────────────────────────┤
        │                 [Later] [Install]  │  secondary first, primary last
        └────────────────────────────────────┘
```

Busy (downloading/installing): `escapeKeydownBehavior="ignore"`, `interactOutsideBehavior="ignore"`, close ✕ disabled. CLI updates (`cliUpdate`) reuse the same layout with "Claude CLI" / "Codex CLI" eyebrow.

### 2.3 MCP servers dialog

```
   ┌──────────────────────── 560 (size lg) ────────────────────────┐
   │ MCP servers                                            ✕      │
   │ ┌───────────────────────────────────────────────────────────┐ │
   │ │ ● filesystem      stdio   Connected            [⋯]        │ │ rows --list-row-h, Badge dot tone
   │ │ ● github          http    Needs auth           [⋯]        │ │
   │ │ ○ blazzer-search  http    Failed               [⋯]        │ │
   │ └───────────────────────────────────────────────────────────┘ │
   │ Add server                                                     │
   │ Name [__________]  Transport [stdio ▾]                         │ Field + Input sm + Select
   │ Command [______________________________]                       │
   │ ⚠ Remove "github"? This edits ~/.claude.json  [Cancel][Remove]│ Banner danger, focus Cancel
   ├────────────────────────────────────────────────────────────────┤
   │                                        [Close]  [Add server]   │
   └────────────────────────────────────────────────────────────────┘
```

Status dot tones (K13 Badge tones): connected `ok`, needs-auth `warn`, failed `danger`, disabled `neutral`. Dialog `size="lg"` → `--dialog-w-lg` 560. Empty: `EmptyState` "No MCP servers yet". Loading: 3 `Skeleton` rows at `--list-row-h`. Five `title=` → `Tooltip`.

### 2.4 Diagnostics page

```
 ┌ PageHeader ─────────────────────────────────────────────────────────────┐
 │ Diagnostics                    [Source: All ▾] [⌕ filter] [Copy] [Clear]│
 ├─────────────────────────────────────────────────────────────────────────┤
 │ 12:04:11  warn   claude.stream   slow first token (4.2 s)              │ row --list-row-h 32 (36/40)
 │ 12:04:09  info   mcp             connected filesystem                  │ info = neutral Badge
 │ 12:03:58  error  browser         probe failed: ERR_CONNECTION_REFUSED  │ error Badge danger
 │ …virtualised…                                                          │
 └─────────────────────────────────────────────────────────────────────────┘
```

`ROW_H` reads `--list-row-h` via `getComputedStyle(listEl)` and re-runs on `uiPrefs.density` (ui-prefs :98, :279). 26 → 32 at compact = ~19% fewer visible rows (36/40 at regular/comfy cost more; board lever: pin the console to `--control-dense-h` 28/32/36). `--dc-*` vars (:326-331) map: `--dc-bg`→`--bg-000`, `--dc-row-hover`→`--bg-200`, `--dc-line`→`--line-100`, `--dc-warn`→`--warn`, `--dc-error`→`--danger`, `--dc-muted`→`--fg-subtle`. Level badges: error `danger`, warn `warn`, info/debug `neutral`. Action errors (copy/clear/export) → `toast({tone:'danger'})`. The modal-dialog branch of the console is frozen (gated #6, §8).

### 2.5 AI Health (Claude and ChatGPT)

Fit-one-screen target: **≥1160×860 at compact, advice collapsed**, no page scroll; lists scroll inside their card.

```
 ┌ PageHeader (max 1080) ──────────────────────────────────────────────────┐
 │ Claude usage & health                              [Refresh] Diagnostics→│ ghost link
 ├─────────────────────────────────────────────────────────────────────────┤
 │ status  ● Healthy · last check 12 s ago · 3 MCP connected               │ Banner-less status row
 ├──────────────────────────────┬──────────────────────────────────────────┤
 │ plan   [Max 20x] [Opus 4.x]  │ speed   TTFT 1.2 s  ▁▂▃▅▃▂  tok/s 84     │ StatTile ×3 + Sparkline
 ├──────────────────────────────┤                                          │
 │ usage  5h  ▓▓▓▓▓░░ 62 %      │                                          │ Progress (warn ≥75, danger ≥90)
 │        7d  ▓▓░░░░░ 28 %      │                                          │
 ├──────────────────────────────┼──────────────────────────────────────────┤
 │ spend  ▇▅▃▆▂▇▅  ShareBar     │ mcp   ● fs ● github ○ search  (scroll)   │ DayBars + ShareBar · list in card
 └──────────────────────────────┴──────────────────────────────────────────┘
   ▸ Advice (2)   — inline Accordion under the grid (board lever: drawer)
```

| Element | Token / part | Value |
|---|---|---|
| Page max width | `PageHeader maxWidth` | 1080 |
| Grid | `grid-template-areas` | `"status status" "plan speed" "usage speed" "spend mcp"`, cols `1fr 1fr`, gap 16 (`gap-4`) |
| Card (K12) | `--bg-100`, `--line-100`, `rounded-lg`, `--card-pad` | pad 16 (20/24) |
| Title | sentence case | "Claude usage & health" / "ChatGPT health"; "Experimental" eyebrow removed |
| StatTile | K25 + amendment `text?` | value `--t-metric` 28/32 tabular; text tiles (plan, top model, codex version) use `text` |
| Limits | K27 `Progress` | 6px; tone from `zone()` mapper |
| Spend split | K26 `ShareBar` | model share |
| Day bars / sparkline | `shared/DayBars`, `shared/Sparkline` | owned by first of 4d/4f |
| Count-up | 03 count-up | only when `rangeKey === perfWindow` changes, never on poll |

Tone mapping lives in `healthFormat.ts` at the boundary: `zone`/`mcpTint`/`healthScore` (`ok|warn|hot`) → `accent|warn|danger`; `statusMeta` (`ok|warn|danger|muted`) → `ok|warn|danger|neutral`; `HealthLevel` (`ok|warn|bad|idle`) → `ok|warn|danger|neutral`. Target names are the K13 Badge tones (`neutral|accent|ok|warn|danger|info`) and the K27 Progress tones. Dead `.ah-trend*` and `.ah-chip.*` rules are moved into one block marked `/* DEAD — delete P5 (#6) */`, not deleted.

### 2.6 Browser dock

```
 ┌ BrowserBar (h 40) ─────────────────────────────────────────────────────┐
 │ [←][→][↻]  [ example.com/path___________________ ]  ◌  [Add to chat] [⋯]│ IconButton/Input at --control-dense-h 28, Spinner
 ├────────────────────────────────────────────────────────────────────────┤
 │ ▔▔▔▔▔▔▔▔▔ indeterminate Progress (2px) while phase=loading             │
 │                                                                        │
 │        .wb-stage — native WebView2 child sits here                     │
 │        veil (when hidden by overlay or loading first paint):           │
 │           [favicon]  example.com                                       │ bg --bg-000, hostLabel --fg-muted
 │                                                                        │
 │ ┌ find ─────────────────────────────┐                                  │ SearchField, no animation
 └────────────────────────────────────────────────────────────────────────┘
```

Overflow `DropdownMenu`: Zoom in / Zoom out / Reset zoom (`closeOnSelect={false}`), Open externally, Copy URL, Find in page. "Add to chat" = `Button variant="soft" size="sm"`. Empty (no URL yet): `EmptyState` "Open a page" with the URL input focused. 16 `title=` → `Tooltip`. The veil shows host label + favicon only: the `browser://load` payload carries `{phase, url}` and no page title (D-20 correction, §13).

## 3. States

| Surface | State | Visual | Source of truth |
|---|---|---|---|
| Palette | empty query | all groups, headings visible | `q === ''` |
| Palette | filtered | headings hidden, flat list | `needle !== ''` |
| Palette | no match | `EmptyState compact` "Nothing matches that." | `items.length === 0` |
| Update | idle/available | version metric + notes accordion | `updates.status` |
| Update | downloading | `Progress` value, Esc/outside ignored, ✕ disabled | `status === 'downloading'` |
| Update | installing | indeterminate `Progress`, same locks | `status === 'installing'` |
| Update | error | `Banner tone="danger"` + Retry | `updates.error` |
| Update | up to date | `EmptyState` "You're on the latest version" | `status === 'current'` |
| MCP | loading | 3 `Skeleton` rows | `mcpPanel.loading` |
| MCP | empty | `EmptyState` | `servers.length === 0` |
| MCP | row connected / needs-auth / failed / disabled | `Badge dot` ok / warn / danger / neutral | server `status` |
| MCP | confirm remove | inline `Banner danger`, focus Cancel | `pendingRemove` |
| MCP | form invalid | `Field error` text, Add disabled | existing validators |
| Diagnostics | empty | `EmptyState` "No events yet" | `rows.length === 0` |
| Diagnostics | filtered empty | `EmptyState compact` "No events match" + Clear filter | filter active |
| Diagnostics | action failed | `toast tone="danger"` | catch in action |
| AI Health | first load | `Skeleton` per card | no snapshot |
| AI Health | stale (> poll × 2) | status row `Badge warn` "Stale" | `lastCheckedAt` |
| AI Health | provider unavailable | card `EmptyState` with reason | probe error |
| AI Health | limit zones | `Progress` tone accent / warn ≥75 / danger ≥90 | `zone()` |
| Browser | no URL | `EmptyState` "Open a page" | `!browserDock.url` |
| Browser | loading | indeterminate bar + Spinner in bar | `phase === 'loading'` |
| Browser | loaded | native child visible | `nativeVisible` |
| Browser | covered by overlay | native hidden, veil shown | `!nativeVisible && opened` |
| Browser | probe failed | `Banner tone="danger"` + Retry on the veil (EmptyState has no tone prop) | `phase === 'error'` |
| Confirm | open | `AlertDialog`; `destructive` (K24, focus Cancel) when `kind==='warning'` | `confirmQueue[0]` |

## 4. Motion

All durations/easings are 03-motion tokens. RM = behaviour under `prefers-reduced-motion` / `uiPrefs.reducedMotion`.

| Element | Trigger | Property | From → To | Duration | Easing | Delay | RM |
|---|---|---|---|---|---|---|---|
| Palette dialog | open | opacity, scale | kit `animate-zoom-in` (02 K07 palette recipe) | `--dur-base` 240 | `--ease-out` | 0 | opacity only (P2 net) |
| Palette dialog | close | opacity | kit `animate-zoom-out` | `--dur-pop-out` 140 | `--ease-out` | 0 | instant |
| Palette rows | filter | — | none (`animate-none`, no stagger) | — | — | — | — |
| Palette active row | arrow / pointer | background | transparent → `--bg-200` | `--dur-fast` 150 | `--ease-out` | 0 | instant |
| Update / MCP / Confirm dialog | open / close | opacity, scale | kit Dialog (02 K07) `animate-zoom-in` / `-out` | `--dur-base` 240 / `--dur-pop-out` 140 | `--ease-out` | 0 | opacity only |
| Update Progress | value change | width (transform scaleX) | prev → next (K27 scaleX) | `--dur-base` 240 | `--ease-out` | 0 | instant |
| Indeterminate bar | loading | `.indet-bar` (M21) / K27 indeterminate sweep | — | `--dur-indet` 1.3s loop | linear | 0 | static 40% bar (K27) |
| Accordion (tech details, advice) | toggle | height, opacity | kit K30b (Collapse grid-rows) | `--dur-base` 240 | `--ease-out` | 0 | instant |
| MCP remove Banner | appear | opacity | 0 → 1 | `--dur-fast` 150 | `--ease-out` | 0 | instant |
| Diagnostics rows | append | — | none (virtualised; no entrance) | — | — | — | — |
| AI Health page | workspace enter | `.ws-page` glide (M30) | translateY `--rise-page` 10 → 0, opacity 0 → 1 | `--dur-page` 520 (+ `--dur-page-delay` 60) | `--ease-out` | 0 | opacity only |
| AI Health cards | first data | `.page-sections` reveal | opacity 0 → 1 | `--dur-base` 240 | `--ease-out` | `--stagger` 60 per section, max 3 | instant |
| StatTile value | `rangeKey === perfWindow` change | count-up | prev → next (K04c `countUp`) | `--dur-count` 700 | easeOutCubic | 0 | instant |
| Chat `.ws-page` | workspace enter | opacity only (**amend 03 §2.4**) | 0 → 1 | `--dur-page` 520 | `--ease-out` | 0 | instant |
| Browser dock | open / close | width (`dockWidth` 300, M11) | 0 → w | 300 (`dockWidth`, 03 M11) | `--ease-out` | 0 | instant |
| Browser veil | native hide | opacity | 0 → 1 | 0 (instant, must cover before hide) | — | — | instant |
| Browser veil | native show | opacity | 1 → 0 | `--dur-fast` 150 | `--ease-out` | 1 frame after `browser_show` resolves | instant |
| Find bar | open | — | none | — | — | — | — |
| Toaster offset | dock width change | `left` | follows `--dock-w` | none (jumps with dock frame) | — | — | — |

Why chat `.ws-page` is opacity-only: `.wb-stage` lives inside `.ws-page` (WorkspaceShell :128-148, `data-workspace` :87). A `translateY` on an ancestor moves the stage without firing `ResizeObserver`, so the native child is positioned 10 px off for the whole glide. Opacity does not move geometry.

## 5. Parts

### 5.1 Kit parts used

| Part (02-kit id) | Where |
|---|---|
| `Dialog` (K07) sizes `palette` / default (`--dialog-w` 480) / `lg` (`--dialog-w-lg` 560) | Palette, Update, MCP |
| `AlertDialog` (K07 surface + K24 `destructive`) | ConfirmHost |
| overlay registry (K08, `state/overlays.svelte.ts`, §2.4b) | all overlays, browser visibility |
| `DropdownMenu` (§2.4b, K05 recipes) | MCP row ⋯, Browser overflow |
| `Select` (§2.4b, K23 `fieldTrigger`) | MCP transport |
| `Combobox` (K23c) | Diagnostics source filter |
| `Accordion` (K30b) | Update tech details, AI Health advice |
| `StatTile` (K25, + `text`) | AI Health |
| `ShareBar` (K26) | AI Health spend |
| `Progress` (K27) | Update download, limits, browser load |
| `PageHeader` (K28) | AI Health, Diagnostics |
| `Tooltip` (K31–32) | 21 `title=` sites |
| `Button`, `IconButton`, `Input`, `Field`, `Badge`, `Banner`, `EmptyState`, `Skeleton`, `Spinner`, `Kbd`, `SearchField` | throughout |
| `CodeBlock` (P4b, D-18) | Update tech details |
| `toast()` (P4a) | Diagnostics/MCP action errors |

### 5.2 Rift parts API (new or changed)

```ts
// state/overlays.svelte.ts (K08) — 02-kit §2.4b class kept, three additions
export const overlays: {
  modal: number;                       // $state(0) — existing
  floating: Set<HTMLElement>;          // existing ($state Set, immutable add/remove)
  moved: number;                       // NEW $state(0): bumps when a floating el repositions
  openModal(): () => void;             // existing — returns release fn
  trackFloating(el: HTMLElement): () => void; // existing
  notifyMoved(): void;                 // NEW
  overlapsRect(r: DOMRectReadOnly): boolean;  // NEW: any floating el intersects r (reads `moved`)
};
```

Supersedes the 02-kit §6 K08 row (`open(id)/close(id)/top/Esc`): there is no id stack; Esc is owned by bits-ui's escape layer. Tooltips and toasts never register.

```ts
// actions/overlay.ts — for bespoke floating sites that are not kit content
export function overlay(node: HTMLElement, opts?: { modal?: boolean }): ActionReturn;
// modal → openModal(); else trackFloating(node) + ResizeObserver/position watch → notifyMoved()
```

Bespoke sites that get `use:overlay`: image lightbox, OutputBlock inspector, FilePathMenu, GhPopover, SettingsMenu, UsagePanel, SlashMenu, MentionPopover, EnhanceBar. Kit content components (Dialog/AlertDialog/Popover/DropdownMenu/Select/Combobox content) register themselves in a `$effect`.

```ts
// state/confirm.svelte.ts — drop-in for plugin-dialog confirm
export function confirm(message: string, opts?: {
  title?: string; kind?: 'info' | 'warning'; okLabel?: string; cancelLabel?: string;
}): Promise<boolean>;            // FIFO queue; resolves false on Esc/outside
export const confirmQueue: ConfirmRequest[]; // read by ConfirmHost
```

```ts
// WebBrowserPage.svelte — replaces the effect at :371-379
// stageRect is $state, written by the existing .wb-stage ResizeObserver AND on dock-width
// change and window resize; without these writes the derived never re-evaluates on geometry.
let stageRect = $state<DOMRectReadOnly | null>(null);
const nativeVisible = $derived(
  opened && activeWorkspace === 'chat' && overlays.modal === 0 &&
  !(stageRect && overlays.overlapsRect(stageRect))
);
$effect(() => {
  if (navPending) return;                // browser_hide bumps OPEN_GEN (mod.rs:301-312) → would cancel open_probed
  nativeVisible ? show() : (veilOn = true, hide());
});
```

`BrowserBar.svelte` props: `{ url, phase, canBack, canForward, zoom, onNavigate, onBack, onForward, onReload, onAddToChat, onFind, onZoom }`. `healthFormat.ts` exports `zoneTone`, `mcpTone`, `scoreTone`, `statusTone`, `levelTone` (all `→ Tone`).

`window.__riftUi` (DEV only) gains `updates`, `mcpPanel`, `commandPalette`, `browserDock`, `confirm`, and `browserVis()` returning `{ opened, modal, floating, nativeVisible, navPending }` for CDP checks.

## 6. Ordered commits

Verify column: `chk` = `npm run check`, `test` = `npm test`, `ui` = `npm run check:ui`, `cdp` = the c.sh steps in §10. Every commit runs `chk`; listed extras only.

### 6.1 Group A — overlay infrastructure (F00–F07)

| # | Title | Files | Change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| F00 | chore(ui): record 4f baseline | `ui-baseline.json` | Measure use-portal / aria-role / rgb-hsl / oklch on HEAD; note numbers in commit body | — | none | `ui` |
| F01 | feat(overlay): moved counter + overlapsRect | `state/overlays.svelte.ts` + test | Add `moved`, `notifyMoved()`, `overlapsRect()` | `modal`, `floating`, `openModal`, `trackFloating` unchanged | low | `test` |
| F02 | feat(overlay): use:overlay action | `actions/overlay.ts` + test | Action wraps registry; RO → `notifyMoved` | — | low | `test` |
| F03 | feat(overlay): register kit floating content | kit Popover/DropdownMenu/Select/Combobox content | `$effect` → `trackFloating(el)`; Dialog/AlertDialog → `openModal()` | bits-ui behaviour untouched | med: double registration if a kit part already does it — grep first | `test` |
| F04 | feat(overlay): bespoke floating sites | lightbox, OutputBlock inspector, FilePathMenu, GhPopover, SettingsMenu, UsagePanel, SlashMenu, MentionPopover, EnhanceBar | add `use:overlay` on the root floating element | open/close conditions unchanged | med: a site that is always mounted but hidden would count as floating — register only inside the `{#if open}` | `cdp` B1 |
| F05 | feat(browser): nativeVisible derived | `WebBrowserPage.svelte` | One `$derived` + one `$effect` replace :371-379; `navPending` defers hide | `browser_show/hide` IPC args, `open_probed` path | **high** (OPEN_GEN cancel, flash) | `cdp` B1–B4 |
| F06 | fix(browser): veil before hide, fade after show | `WebBrowserPage.svelte` | `veilOn=true` synchronously before `hide()`; clear one rAF after `show()` resolves; covers `open()`'s unconditional show at :207 | — | med | `cdp` B2 |
| F07 | style(motion): chat ws-page opacity-only | `WorkspaceShell.svelte` | `.ws-page[data-workspace="chat"]` drops translateY | other workspaces keep 10 px glide | low | `cdp` B5 |

### 6.2 Group B — confirm host and dialogs (F08–F17)

| # | Title | Files | Change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| F08 | feat(confirm): promise confirm + host | `state/confirm.svelte.ts`, `shell/ConfirmHost.svelte`, `AppShell.svelte`, test | FIFO queue; AlertDialog; mounted once. **Skip if 4e landed it** | — | low | `test` |
| F09 | refactor(updates): use kit confirm | `updates.svelte.ts:287`, `cliUpdate.svelte.ts:432` | swap import; same message, `kind:'warning'` | branch taken on true/false identical | low: async signature already | `test` |
| F10 | refactor(palette): kit Dialog shell | `CommandPalette.svelte` | `Dialog size="palette"`; drop scrim/wrap CSS, Tab trap :248-265, bespoke focus restore | open state from `command-palette` store | med: Esc double-close — bits-ui owns Esc | `cdp` P1 |
| F11 | style(palette): rows, headings, footer | `CommandPalette.svelte` | `--list-row-h`, `--bg-200`, `menuLabel`, `Kbd` legend, `--palette-input-h`; `animate-none` | filter, active index, scrollIntoView | low | `cdp` P2 |
| F12 | a11y(palette): combobox/listbox roles | `CommandPalette.svelte` | input `role=combobox aria-expanded aria-controls aria-activedescendant`; list `listbox`; rows `option aria-selected` | keys stay on input | low (aria-role +1) | `ui` |
| F13 | refactor(palette): nav titles from WORKSPACES | `command-palette.svelte.ts`, `CommandPalette.svelte` | labels read `WORKSPACES[id].title`; keep "Go to Diagnostics" | order not from `workspace.order` | low | `test` |
| F14 | refactor(update): kit Dialog + footer | `UpdateDialog.svelte` | `Dialog` 480; drop Tab trap :39-55; busy ⇒ `escapeKeydownBehavior`/`interactOutsideBehavior="ignore"` | download/install/relaunch calls | med: Esc closing mid-download | `cdp` U1 |
| F15 | style(update): metric, Progress, Banner | `UpdateDialog.svelte` | version metric 28; `Progress`; error `Banner` + Retry | progress math | low | `cdp` U2 |
| F16 | feat(update): tech details accordion | `UpdateDialog.svelte` | `Accordion` → `CodeBlock` (P4b) for notes/log | content unchanged | low | `cdp` U3 |
| F17 | refactor(mcp): kit Dialog lg + Esc gate | `McpServersDialog.svelte` | `Dialog size="lg" escapeKeydownBehavior="ignore"`; window capture handler :111-129 kept, returns early when `overlays.floating.size>0` | dirty-form guard in that handler | **high**: nested Select Esc closes dialog | `cdp` M1 |
| F18 | style(mcp): rows, Badge dot, form fields | `McpServersDialog.svelte` | `--list-row-h` rows; `Badge dot`; `Field`/`Input sm`/`Select`; row ⋯ `DropdownMenu` | add/edit/remove/toggle calls | low | `cdp` M2 |
| F19 | feat(mcp): inline remove Banner | `McpServersDialog.svelte` | `Banner danger` replaces bespoke confirm; focus Cancel | remove call + reload | low | `cdp` M3 |
| F20 | feat(mcp): empty, skeleton, tooltips | `McpServersDialog.svelte` | `EmptyState`; 3 `Skeleton`; 5 `title=` → `Tooltip` | — | low | `ui` |
| F21 | fix(mcp): errors → toast | `McpServersDialog.svelte` | action catches call `toast({tone:'danger'})` | error text | low | `cdp` M4 |

### 6.3 Group C — Diagnostics (F22–F27)

| # | Title | Files | Change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| F22 | refactor(diag): tokens replace --dc-* | `DiagnosticsConsole.svelte` | map 6 vars (§2.4); delete their `:root`-like block :326-331 | — | low (rgb-hsl/oklch down) | `ui` |
| F23 | feat(diag): ROW_H from --list-row-h | `DiagnosticsConsole.svelte` | `getComputedStyle` read; `$effect` on `uiPrefs.density` re-measures and resets virtual window | virtual-scroll math | med: stale height after density switch | `cdp` D2 |
| F24 | refactor(diag): PageHeader + actions | `DiagnosticsPage.svelte`, `DiagnosticsConsole.svelte` | Copy/Clear/Export into `PageHeader actions`; `.page-sections`; drop `PageHero` import (:5) | handlers | low | `cdp` D1 |
| F25 | feat(diag): source Combobox | `DiagnosticsConsole.svelte` | replaces bespoke source select | filter predicate | low | `cdp` D3 |
| F26 | style(diag): level badges, empty states | `DiagnosticsConsole.svelte` | `Badge` danger/warn/neutral; two `EmptyState`s | — | low | `cdp` D1 |
| F27 | feat(cdp): nav diagnostics target | `AiHealthPage.svelte` (header link `data-goto="diagnostics"`), `scripts/cdp/c.sh` | add `diagnostics)` case in nav block (:472-521) | — | low | `cdp` D1 |

### 6.4 Group D — AI Health (F28–F33)

| # | Title | Files | Change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| F28 | feat(health): tone mappers | `ai-health/healthFormat.ts` + test | 5 mappers (§5.2) | source enums untouched | low | `test` |
| F29 | refactor(health): PageHeader + titles | `AiHealthPage.svelte` | drop `PageHero` import (:10); `PageHeader maxWidth=1080`; sentence-case titles; drop Experimental eyebrow; Diagnostics ghost link | refresh handler | low | `cdp` H1 |
| F30 | refactor(health): grid areas, card scroll | `AiHealthPage.svelte` | `grid-template-areas`; lists `overflow:auto` inside card | — | med: fit-one-screen at 1160×860 | `cdp` H2 |
| F31 | feat(health): StatTile, Progress, ShareBar | `AiHealthPage.svelte` | tiles (+`text`); limits → kit `Progress` (drop `LimitBar` import :11; `LimitBar` itself untouched — UsagePanel/StatusBar use it); `ShareBar` | numbers, zones | med: StatTile `text` amendment must land in kit first | `cdp` H3 |
| F32 | feat(health): DayBars + Sparkline | `shared/DayBars.svelte`, `shared/Sparkline.svelte` (create **only if 4d has not**) | extract from AiHealthPage | data shaping | low | `test` |
| F33 | style(health): advice accordion, dead CSS block | `AiHealthPage.svelte` | advice → inline `Accordion`; `.ah-trend*`/`.ah-chip.*` moved under `/* DEAD — delete P5 (#6) */`; count-up gated on `rangeKey===perfWindow` | advice content | low | `cdp` H4 |

### 6.5 Group E — Browser dock (F34–F38)

| # | Title | Files | Change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| F34 | refactor(browser): extract BrowserBar | `webview/BrowserBar.svelte`, `WebBrowserPage.svelte` | move URL bar markup; props in §5.2 | navigate/back/forward/reload IPC | med: focus/selection on URL input | `cdp` B6 |
| F35 | style(browser): kit controls in bar | `BrowserBar.svelte` | `IconButton sm`, `Input sm`, `Spinner`, `Badge`s, "Add to chat" soft; 16 `title=` → `Tooltip` | — | low | `ui` |
| F36 | feat(browser): overflow DropdownMenu | `BrowserBar.svelte` | zoom items `closeOnSelect={false}`; open external; copy URL; find | zoom step values | med: menu over stage hides native (expected — verify veil) | `cdp` B3 |
| F37 | style(browser): Progress, find SearchField, EmptyState | `WebBrowserPage.svelte` | kit indeterminate `Progress`; find bar `SearchField` (no animation); empty/error `EmptyState` | find next/prev IPC | low | `cdp` B7 |
| F38 | feat(toast): dock-aware Toaster offset | `assistant/AssistantPage.svelte`, `AppShell.svelte` | `document.documentElement.style.setProperty('--dock-w', w+'px')` on dock resize, `0px` on close/destroy; Toaster `right`/`left` offset adds `var(--dock-w,0px)` (the toast host :332 sits outside `.assistant-page`, so a var on that element would never inherit) | dock resize logic | low | `cdp` B8 |

### 6.6 Group F — close-out (F39–F40)

| # | Title | Files | Change | Logic preserved | Risk | Verify |
|---|---|---|---|---|---|---|
| F39 | feat(dev): __riftUi probes | `AppShell.svelte` (DEV block) | expose stores + `browserVis()` | DEV-only, tree-shaken | low | `cdp` all |
| F40 | chore(ui): lower 4f baseline | `ui-baseline.json` | write new counts (§10) | — | none | `ui`, `npm run verify` |

41 commits counting F00. F08 and F32 collapse to no-ops when 4e / 4d already landed them (→ 39).

## 7. Contracts (D-33)

What other phases and code may rely on after 4f:

- **Overlay registry**: `overlays.modal`, `overlays.floating`, `overlays.moved`, `openModal()`, `trackFloating()`, `notifyMoved()`, `overlapsRect()`. Any new floating layer **must** register (kit content does it; bespoke uses `use:overlay`). Tooltips and toasts must not.
- **Native browser visibility** is decided only by `nativeVisible` in `WebBrowserPage`. No other file calls `browser_show`/`browser_hide`.
- **`confirm()`** from `state/confirm.svelte.ts` is the only confirm in the frontend; `@tauri-apps/plugin-dialog` `confirm/ask` imports = 0 after 4e+4f.
- **Tone mapping** happens in `healthFormat.ts`; components receive `Tone`, never `ok|warn|hot|bad`.
- **`--dock-w`** is published on `:root` (`document.documentElement`, px, `0px` when closed); Toaster reads it.
- **`--list-row-h`** drives Diagnostics virtual-row height; nothing hard-codes 26/32.
- **`data-goto="diagnostics"`** and the `c.sh nav diagnostics` case are stable selectors for CDP.
- **`window.__riftUi`** keys added here are DEV-only and may change without notice.
- IPC, Rust, store shapes, persisted prefs: **unchanged** by 4f.

## 8. Deletions (gated #6)

Nothing below is deleted inside 4f. Each is flagged in code with `/* DEAD — delete P5 (#6) */` or listed for the P5 sweep, and needs the owner's explicit go.

| Item | Where | Why dead | Gate |
|---|---|---|---|
| `PageHero.svelte` | `shared/` (48 lines) | AI Health + Diagnostics drop it (F24, F29); `SettingsPage:33` remains → 0 importers only after 4e | #6, P5 (after 4e) |
| `LimitBar.svelte` | `shared/` (82) | AI Health drops it (F31); `UsagePanel:10`, `StatusBar:11` remain until their phases adopt `Progress` | #6, P5 (after those phases) |
| `.ah-trend*`, `.ah-chip.*` rules | `AiHealthPage.svelte` | no markup uses them (verified by Grep at F33) | #6, P5 |
| `severity-tone.css` | `src/lib/styles/` | importers `ToastHost:12`, `NotificationCenter:14` → 0 after 4a | #6, P5 |
| Diagnostics modal branch | `DiagnosticsConsole.svelte` | no caller opens it as a modal; frozen, not restyled | #6, P5 |
| Palette/Update Tab-trap helpers | `CommandPalette:248-265`, `UpdateDialog:39-55` | kit Dialog traps focus — these are removed in F10/F14 (logic replacement, not a dead-code delete) | none (part of the refactor) |
| `--dc-*` vars | `DiagnosticsConsole:326-331` | mapped to tokens (F22) — removed in the same commit | none (part of the refactor) |

## 9. Canvas boards

Boards live in the halo canvas file (07 and 08 are this phase's numbers in the board index).

### Board 07 — "Overlays" (frame 1440×900, plus a 375×812 phone frame for the palette)

Contents: palette (empty, filtered, no-match), Update (available, downloading, error, up-to-date), MCP (list, loading, empty, remove-confirm, nested Select open), ConfirmHost, and the browser dock with a DropdownMenu over the stage showing the veil.

Play interactions: ⌘K opens palette, arrows move, Enter runs; Update "Install" walks available → downloading (fake 0→100 over 3 s) → installing; Esc during download does nothing (visible shake-free no-op); MCP "Remove" reveals Banner with focus ring on Cancel; opening the browser overflow menu swaps the stage for the veil.

`data-props` levers: `palette.rowH` 28 | **32** | 36 · `palette.top` 10vh | **12vh** | 16vh · `palette.inputH` 44 | **52** · `update.width` 440 | **480** | 520 · `mcp.width` 520 | **560** | 640 · `veil.content` none | **host+favicon** | spinner.

Owner decisions on this board: palette row height; whether "Go to Diagnostics" stays in the palette; veil content.

Exported numbers (consumed by F11/F14/F17/F06): palette row h, top offset, input h; Update width; MCP width; veil variant.

### Board 08 — "Health & Diagnostics" (frames 1160×860 compact and 1440×900 comfortable)

Contents: Claude AI Health, ChatGPT health, Diagnostics with 200 fake rows, each in default / stale / provider-unavailable / first-load.

Play interactions: density toggle re-flows both pages; Diagnostics source Combobox filters; AI Health advice expands inline; range switch triggers count-up on tiles only.

`data-props` levers: `diag.rowH` **32** | 28 · `health.advice` **inline** | drawer · `health.grid` **2-col areas** | 3-col · `health.maxWidth` 960 | **1080** | 1200 · `limit.warnAt` 70 | **75** · `limit.dangerAt` **90** | 95.

Owner decisions: Diagnostics row height (density cost: 19% fewer rows at 32); advice inline vs drawer; limit thresholds.

Exported numbers (consumed by F23/F30/F31/F33): row h, max width, grid variant, warn/danger thresholds, fit-one-screen pass/fail at 1160×860 compact.

## 10. Verification

Commands (Windows dev machine): `npm run check`, `npm test`, `npm run check:ui`; at F40 `npm run verify` (complete gate). No cargo run is needed — no Rust changes; CI still runs clippy.

CDP steps (`scripts/cdp/c.sh`, running app via the rift-ui skill):

| Id | Steps | Pass |
|---|---|---|
| B1 | `nav chat`; open browser dock on a URL; open Settings menu over stage; `eval __riftUi.browserVis()` | `nativeVisible:false`, `floating≥1`; close menu → `true` |
| B2 | open palette over dock; `look .wb-stage` | veil visible, no native frame; close → native back, no flash frame in a 3-shot burst |
| B3 | open overflow, click Zoom in ×2 | menu stays open; zoom applied; native hidden while menu open |
| B4 | navigate to slow URL, immediately open palette | probe completes (phase → loaded), page shown after palette closes; no `ERR` toast |
| B5 | switch workspace into chat with dock open | stage does not jump (compare `look .wb-stage` rect at t0 and t+120ms) |
| B6–B8 | URL input focus/select-all; find bar next/prev; toast appears left of dock | as described |
| P1–P2 | ⌘K; type "dia"; ↓ Enter; Esc closes | lands on Diagnostics; one Esc closes |
| U1–U3 | `eval __riftUi.updates.simulate('downloading')`; press Esc | dialog stays; Progress visible; accordion opens CodeBlock |
| M1–M4 | open MCP; open Transport Select; Esc | Select closes, dialog stays; second Esc closes dialog (or dirty-guard prompt) |
| D1–D3 | `nav diagnostics`; toggle density | visible row count changes; no blank band in virtual list |
| H1–H4 | `nav health` at 1160×860 compact | no page scrollbar (`document.scrollingElement.scrollHeight <= innerHeight`) |

Counts that must go down (F00 measures, F40 records; expected deltas):

| Check | Expected |
|---|---|
| `use-portal` | 11 → 9 |
| `aria-role` | 17 → ~14 (palette adds listbox/option, dialogs drop bespoke `role=dialog`) |
| `rgb-hsl` literals | −9 |
| `oklch` literals | −8 |
| `title=` in touched files | 21 → 0 |
| plugin-dialog `confirm` imports | 2 → 0 (Settings' 2 owned by 4e) |

## 11. Risks

| # | Risk | Likelihood | Mitigation |
|---|---|---|---|
| R1 | Hiding the native child during navigation bumps `OPEN_GEN` (mod.rs:301-312) and cancels the `open_probed` loopback probe (:151-196, gen check :188) → page never loads | high without the guard | `navPending` defers hide; effect re-runs when it clears; B4 |
| R2 | One-frame flash: `open()` shows unconditionally (:207) before the derived can veto | med | veil set synchronously; `show()` call routed through the effect only; B2 burst |
| R3 | `.ws-page` translateY moves `.wb-stage` without firing ResizeObserver → native child 10 px off during glide | high (today) | chat glide opacity-only (F07, amend 03); B5 |
| R4 | Nested Select inside MCP: window-capture Esc handler closes dialog before bits-ui closes the Select | high | gate on `overlays.floating.size===0`; M1 |
| R5 | A bespoke floating site missed in F04 → native browser paints over it | med | Grep for `position: fixed` / `use-portal` sites at F04; list in commit body |
| R6 | Fit-one-screen at 1160×860 fails once real MCP lists are long | med | lists scroll in card; board 08 pass/fail exported |
| R7 | StatTile `text` amendment not accepted → plan/model/version tiles have no part | low | fallback: plain `Field`-style label/value inside the card |
| R8 | 02-kit internal contradiction on ToastHost swap owner (§2.3 vs §8.2/K34b) leaves two toasters | med | 4f assumes 4a did the swap; F38 aborts if `ToastHost` still mounted |
| R9 | Density switch leaves Diagnostics virtual list with stale ROW_H | med | `$effect` on `uiPrefs.density` re-measures; D-steps |

## 12. Effort

**L — 5 to 6 sessions.** Group A (overlay + browser visibility) 1.5 sessions, the riskiest; Group B (dialogs + confirm) 1.5; Group C (Diagnostics) 0.5; Group D (AI Health) 1–1.5; Group E (browser bar) 0.5–1; close-out 0.25. Each group ends in a commit that passes `chk` and its CDP steps, so a session boundary can fall after any group.

## 13. Amendments to 00-decisions (and 02-kit / 03-motion)

| # | Amends | Change | Reason |
|---|---|---|---|
| A1 | 02-kit §6 K08 row (registry `open(id)/close(id)/top/Esc`) | Superseded by 02-kit §2.4b API plus `moved`, `notifyMoved()`, `overlapsRect()` | Esc already owned by bits-ui escape layer (`use-escape-layer.svelte.js:29`); id stack unused; browser needs geometry, not order |
| A2 | 03-motion §2.4 `.ws-page` glide | `[data-workspace="chat"]` is opacity-only (no translateY) | translate on ancestor of `.wb-stage` misplaces the native child; RO does not fire on transform |
| A3 | 02-kit K25 StatTile | add `text?: string` (renders instead of numeric value, no count-up) | plan name, top model, codex version tiles |
| A4 | D-20 (browser veil shows page title) | veil shows favicon + host label only | `browser://load` payload is `{phase, url}`; no title is available without new IPC (out of scope) |
| A5 | 02-kit §2.3 vs §8.2/K34b (ToastHost swap owner) | owner = **P4a**; 4f only offsets for the dock | contradiction in 02-kit |
| A6 | 02-kit kit inventory | `ui/code-block` (D-18) is a P4b precondition, not a 4f build | not listed in 02-kit; avoid a parallel component |
| A7 | 00-decisions (confirm) | `state/confirm.svelte.ts` + `ConfirmHost` replace `@tauri-apps/plugin-dialog` confirm; owned by the first of 4e/4f | 4 native sites split across two phases |

## 14. Open questions (with defaults)

| # | Question | Default if unanswered |
|---|---|---|
| Q1 | Diagnostics row height 32 (comfortable) costs ~19% visible rows — accept, or pin Diagnostics to 28? | 32 comfortable / 28 compact via `--list-row-h` (board 08 lever) |
| Q2 | AI Health advice inline accordion or side drawer? | inline accordion |
| Q3 | Keep "Go to Diagnostics" in the palette? | keep |
| Q4 | Browser veil: host+favicon, plain bg, or spinner? | host+favicon |
| Q5 | Limit warn/danger thresholds 75/90? | 75 / 90 |
| Q6 | Should `shared/DayBars` + `shared/Sparkline` live in 4d or 4f? | first phase to land owns them; the other imports |
| Q7 | Delete `PageHero`/`LimitBar`/dead `.ah-*` CSS in P5? | yes, after owner #6 approval |
| Q8 | MCP dirty-form guard: keep window capture handler, or move into `onEscapeKeydown`? | keep handler, gated on floating (lower risk) |
