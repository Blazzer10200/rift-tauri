# Graphite-Ink Redesign — Verified Gap Audit + Plan

> Created 2026-06-02. Method: 4 parallel read-only agents cross-checked every `COMPONENT_MAP.md` row against `src/`. Status is evidence-based (path:line / class presence / `git log` recency), not narrative. Spec lives in `C:\Users\BLAZZER\Downloads\.rift-redesign-tmp\design_handoff_rift_redesign\` (README + COMPONENT_MAP + DESIGN_TOKENS + `mockup/`).
>
> **Why this exists:** the rift HANDOFF narrative over-claimed "done." This is the corrected, verified baseline. The redesign is **NOT** as complete as previously stated — Settings is missing a whole section + the RCON console, Chat History is half-built, Files detail pane + Onboarding restyle are unstarted.

---

## ✅ STATUS UPDATE — 2026-06-02 (cont.)
**Phases 1–3 + the P3 follow-up are now DONE, committed (`fff86a2`/`d34fe7f`/`cb0384c`/`d5e3eef`), and CDP-verified end-to-end this session.** Escape-to-close on the history modal also fixed (`77c8aab`). `npm run check` 0/0/4108.
- The **per-surface tables below are the ORIGINAL session-start baseline** (the pre-P1–3 snapshot) — left intact on purpose as the historical "before." Do **not** trust their MISSING/NOT-STARTED/PARTIAL markers as current; the verified-done detail lives in `docs/HANDOFF.md` + `git log`.
- **Remaining: Phase 4 (Files detail pane — metadata-only) + Phase 5 (Onboarding restyle), then Phase 6 ship.** See the Plan section (items 12–14) and HANDOFF "RESUME HERE."
- One scope correction since this doc was written: **P4 Files is metadata-only** (status pill + size/modified + status-LED pips + filter chips); the **Lua code-preview is DEFERRED** (no file-read path; user declined a 2nd backend exception).

## Token nuance (read before trusting "missing `--accent-h`" flags)
Agents flagged many components for "no `var(--accent-h)`." **That is a FALSE signal.** Per the foundation, only the `:root` ramp in `app.css` consumes `--accent-h`; components correctly use the hue-derived `var(--accent)`/`--accent-soft`/etc., which already retheme. Do **not** churn components to add `--accent-h`. The real signals are *structural* (missing layout/feature) and *motion-var adoption* (`--ease-page`/`--dur-*`), not token names.

---

## Verified status by surface

### Shell / chrome
| Item | File | Status | Note |
|---|---|---|---|
| Titlebar 44px / brand / controls | `shell/Titlebar.svelte` | PARTIAL | Structure fine; motion-var polish only |
| Rail calm active capsule | `shell/ActivityBar.svelte` | DONE | — |
| **Rail expand 44→220** | `shell/ActivityBar.svelte` | **MISSING** | `ui-prefs` writes `--rail-w` (220/48) + `railPinned`, but ActivityBar CSS hard-codes `44px` and ignores it. No expanded label state. |
| **⌘K centered top-bar search** | `dialogs/CommandPalette.svelte` | **MISSING** | Modal-only (`cp-scrim`). Spec wants an inline centered search field in the top bar. |
| StatusBar LEDs | `shell/StatusBar.svelte` | PARTIAL | LEDs work; motion-var polish only |
| Route crossfade / stagger | `shell/WorkspaceShell.svelte` | PARTIAL | Crossfade exists (inline cubic-bezier); no `--ease-page`/`--dur-*`, no staggered-rise |
| **PageHeader** | `shell/PageHeader.svelte` | **NOT-STARTED** | Pre-redesign (last touch 05-22), legacy styling |
| **PageToolbar** | `shell/PageToolbar.svelte` | **NOT-STARTED** | 26-line stub, no redesign markers |
| ChatTabsBar | `shell/ChatTabsBar.svelte` | PARTIAL | Model-aurora done; motion-var polish |

### Home — DONE (net-new "Focal", real-data). No gaps found.

### Chat / assistant
| Item | File | Status | Note |
|---|---|---|---|
| Flat-timeline turns | `assistant/MessageBubble.svelte` | DONE | `.turn-rail`/`.tl-node`/`.tl-stepdot`, no bubbles |
| Tool nodes + captions | `assistant/ToolChip.svelte`, `toolCaption.ts` | DONE | timeline variant, verb-led captions |
| Edit diff | `assistant/EditDiff.svelte` | DONE | side-by-side; no syntax-highlight (by design) |
| Composer | `assistant/Composer.svelte` | DONE | model/slash/@/attach/gauge/perm-pill all present |
| Activity dock | `assistant/ActivityPanel.svelte` | DONE | quick-actions→meter→tasks→review |
| Panels menu | `assistant/OpenInPaneMenu.svelte` | PARTIAL | functional; not in mockup; low priority |
| **Sync-in-chat banner** | `assistant/SyncActivityBanner.svelte` | **NOT-STARTED** | still sRGB `color-mix` + hex literals; needs oklch/token restyle |
| **History — detail pane** | `assistant/HistoryDrawer.svelte` | **PARTIAL — major gap** | List half only. Missing: master-detail split, per-convo stat strip (msgs/tokens/cost/dur), AI recap, changed-files diff-stat, resource chips, Preview transcript, Open/Branch/Export footer CTAs, pin/unpin. (`history.css` `.hp-detail`/`.hp-stats`/`.hp-block`/`.hpv-*`/`.hp-cta` have no Svelte counterpart.) |
| **ChatRail (left-of-chat archive)** | — | **MISSING (unbuilt)** | Mockup's separate collapsible `ChatRail` (`cr-*`, push/overlay modes) does not exist at all. |

### Sync — DONE (this session: tabbed hub + Activity demotion). No gaps.

### Files — two-pane browser
| Item | File | Status | Note |
|---|---|---|---|
| Two-pane local\|remote | `browser/TwoPane.svelte` | PARTIAL (deferred-by-decision) | App keeps the MORE-capable local\|remote split; mockup's tree+detail would regress it. **Decision: keep two-pane, ADD a detail pane — don't migrate to tree.** |
| Status LEDs + filter chips | `browser/{Local,Remote}Pane.svelte` | PARTIAL | conflict shown as text `▲`, not LED pip; filter is plain input, no chip row |
| Breadcrumb / LockBadge | `browser/PathBreadcrumbs.svelte`, `LockBadge.svelte` | PARTIAL | functional; class-namespace differs |
| **Right detail pane + Lua preview** | — | **MISSING (FbDetail)** | No detail panel anywhere: status pill, meta strip (size/lines/branch/mod), Lua highlight, action footer w/ "Ask Claude" |
| **Conflict banner → Sync/Conflicts** | — | **MISSING** | One-line wire (`syncPage.tab = "conflicts"`) once a small banner exists |

### Settings — single-scroll rebuilt
| Section | Status | Note |
|---|---|---|
| **General** | **MISSING (whole section)** | No `general` in `ST_SECTIONS`. Need: workspace-identity card (name + SFTP path + Connected pill + Switch) + on-machine toggles (launch-at-login / restore-session / confirm-on-quit) + Replay-first-run button |
| Appearance | DONE | 8 swatches, presence, density, code fontSize/tabWidth/ligatures all present + persisted |
| Accessibility | DONE | dyslexia font / line-height / warm tint preserved |
| **Server — RCON live console** | **MISSING** | Zero `rcon` in `settings/`. Stripped 2026-05-25, no successor. Spec: status + password + auto-reconnect + working command console |
| Server — SFTP hero + connection | DONE | — |
| Server — drift-detection card | PARTIAL | only inline fingerprint-unpin; no dedicated auto-rescan/interval/strategy/lock card |
| **Server — danger zone** | **MISSING** | no danger-zone block (Mirror mode lives in Sync kebab) |
| Assistant (session/budget/compaction) | DONE | — |
| About (build/paths/diag) | DONE | — |
| Legacy `settings/Settings.svelte` | CONFIRMED ORPHAN | zero importers → safe to delete |

### Onboarding — first-run overlay (verification-blocked: needs `servers=0` + no-ssh-key gate)
| Item | File | Status | Note |
|---|---|---|---|
| Gate logic | `state/onboarding.svelte.ts` | DONE | 4-condition gate intact |
| Flow shell | `onboarding/OnboardingFlow.svelte` | PARTIAL | centered card stack + top dots; mockup wants full-window left-rail vertical stepper (`ob-overlay`/`ob-rail`/`ob-steps`/`ob-foot`) |
| Welcome | `onboarding/Welcome.svelte` | PARTIAL | no `ObStage` animation, no accent picker; stale "NOT YET WIRED" comment to remove |
| SSHKeySetup | `onboarding/SSHKeySetup.svelte` | PARTIAL | functional; no `ob-choices` grid / ObStage |
| ServerAdd | `onboarding/ServerAdd.svelte` | PARTIAL | mockup folds ProfileSetup in here (5 steps, not 6) |
| ClaudeAuth | `onboarding/ClaudeAuth.svelte` | PARTIAL | no CTA card / statcard / ObStage |
| FirstSync | `onboarding/FirstSync.svelte` | PARTIAL | static prose; mockup wants animated scan (bar + log lines + summary pills) |
| ProfileSetup | `onboarding/ProfileSetup.svelte` | PARTIAL | should be eliminated → folded into ServerAdd |

### Do-NOT-build (prototype scaffolding): `tweaks-panel.jsx`, `icons.jsx`, `usePlayback`, `mix.jsx`.

---

## Plan for tomorrow (sequenced by risk × independence)

Realistically **multi-session** — RCON console, Chat History, Files detail, and Onboarding are each large. Suggested order:

### ✅ Phase 1 — quick structural wins (frontend-only, low-risk, high-visibility) — DONE `fff86a2`, CDP-verified
1. **Rail expand 44→220** — make `ActivityBar.svelte` CSS consume `--rail-w`; add expanded label state. (`railPinned`/`--rail-w` already wired in `ui-prefs`.)
2. **Files conflict banner + deeplink** — small banner in `TwoPane`/pane → `syncPage.tab = "conflicts"`. (deeplink is 1 line; tab state shipped this session.)
3. **SyncActivityBanner restyle** — sRGB→oklch, drop hex, adopt `var(--ok)`/`--danger`/`--bg-elev-2`.
4. **PageHeader + PageToolbar** — restyle to redesign (both pre-redesign).
5. **Delete orphaned `settings/Settings.svelte`** (confirm zero importers first — verified, but re-grep).
6. Motion-var polish pass: Titlebar/StatusBar/WorkspaceShell/ChatTabsBar adopt `--ease-page`/`--dur-*`; add staggered-rise in WorkspaceShell.

### ✅ Phase 2 — Settings completion — DONE `d34fe7f`, CDP-verified (RCON live round-trip untested-by-design)
7. **General section** — new `general` entry in `ST_SECTIONS` + workspace-identity card + on-machine toggles + Replay-first-run button.
8. **Server danger zone** + **drift-detection card** (auto-rescan/interval/strategy/lock — data already in `sync-page`/`connection`).
9. **RCON live console** ⚠️ **LARGEST backend-touching item** — needs bridge/RCON Tauri command wiring (the terminal was stripped, no successor) + a console UI in Settings→Server. **Likely its own session.** Scope the backend commands first (grep `bridge`/`rcon` in `src-tauri/`).

### ✅ Phase 3 — Chat History — DONE `cb0384c`+`d5e3eef`, CDP-verified (master-detail modal + ChatRail)
10. **HistoryDrawer detail pane** — master-detail split: detail pane + per-convo stats + AI recap + changed-files diff-stat + resource chips + Preview transcript + Open/Branch/Export CTAs.
11. **ChatRail** — net-new collapsible left-of-chat archive (push/overlay modes).

### ⏭️ Phase 4 — Files detail pane — NEXT (frontend, metadata-only)
12. **FbDetail** — right-side detail pane (keep local\|remote two-pane; add detail without regressing it): status pill + size/modified meta + status-LED pips (replace text `▲`) + filter chips (All/Lua/Conflicts/Modified) on the panes. **Lua code-preview DEFERRED** (no file-read path; user declined a 2nd backend exception). Files: `src/lib/components/browser/{TwoPane,LocalPane,RemotePane}.svelte`.

### Phase 5 — Onboarding restyle — verification-blocked
13. Full restyle to `ob-*` left-rail stepper + `ObStage` animations + Welcome accent picker + FirstSync animated scan + fold ProfileSetup→ServerAdd (6→5 steps). **Do when a new-user path can be exercised** (temporarily force the gate or use a clean profile). Remove stale Welcome comment.

### Phase 6 — Ship
14. `/git-ship` the whole arc (bump 3 version files + `Cargo.lock` → CHANGELOG → check → build → install).

## Effort ranking (largest first)
RCON console (backend+UI) ≫ Chat History detail+ChatRail ≈ Onboarding full restyle ≈ Files FbDetail > Settings General/danger/drift > Phase-1 quick wins.

## Verification discipline for each phase
Frontend: `npm run check` 0/0 + CDP shot per surface (dev on `localhost:1420`, `bash scripts/cdp/c.sh`). RCON: also `cargo check` + a live bridge probe. Pull exact values from `mockup/styles/*.css` + `DESIGN_TOKENS.md` — don't eyeball.
