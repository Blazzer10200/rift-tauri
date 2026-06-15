# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-14 (cont.133) — Home stats dashboard (Claude-Code-style) → COMMITTED, live-verified Overview

Filled the empty lower-center of Home with a Rift-native stats dashboard (user req: "make it blend in, quality over quantity"). **Honest data only** — per-message tokens/timestamps aren't persisted, so NO fabricated token totals.
- **Backend:** new `assistant_stats` cmd (`convo_store.rs`) scans every saved transcript once → one lightweight `ConvoStat` row/convo (messages, userMessages, toolCalls, words, costUsd, model, created/updatedAt). Registered in `lib.rs`. All day/hour bucketing is FRONTEND so it lands in local tz.
- **Frontend:** `statsHelpers.ts` (pure aggregation — range filter, summarize, streaks, peakHour, perModel, heatmap, funFact, formatters; +21 vitest) · `homeStats.svelte.ts` (cached store) · `HomeStats.svelte` (Overview = 8 KPI tiles [Sessions·Messages·Tool calls·Spend / Active days·Streak·Peak hour·Top model] + 18-week GitHub-style heatmap + Moby-Dick fun-fact; Models = daily message bars + per-model breakdown w/ colored share bars; All/30d/7d range chips; empty + skeleton states). Center column restructured to a flex stack (jump tile on top, stats fills rest) in `HomePage.svelte`.
- **Verified:** svelte-check 0/0 · vitest 154/154 · cargo check clean · live CDP (Overview): real data (138 sessions · 1,841 msgs · 6,794 tools · $936 · 18d streak · 3 PM peak · Opus 4.8), 0 console errors. **Untested:** Models tab pixels (static-green only). No version bump.

## Session 2026-06-14 (cont.132) — split-pane header w/ convo title (#36 UX) → COMMITTED `3b28567`

User dir: "do whatever's best for user-friendly + UI, make it look good"; **keybinds explicitly off the table** (rejected #36 kbd-nav). Picked the clearest split-pane UX gap: panes were identifiable only by a tiny 50%-opacity floating number — couldn't tell *which chat* was in *which pane*.
- **Fix:** floating `.pane-chrome` → always-legible in-flow `.pane-head` strip (split mode only). Shows pane index + **conversation title** (`tab.convoTitle` / first-user-msg fallback, mirrors `healthAlerts.tabTitle`) + ctx chip + close. Focused pane = accent wash + brighter title. Single-pane unchanged (no header). Also fixed a pre-existing tooltip bug (pane-label was a plain string w/ literal `{braces}` → template literal). `AssistantPane.svelte` only.
- **Verified:** svelte-check 0/0 · vitest 132/132 · live CDP (forced 2-pane split: both headers titled, focused pane tinted, restored to single → no header). **No version bump.**
- **Note for next:** resizable pane dividers ALREADY EXIST (`AssistantPage.svelte:29-132` — drag/dbl-click-reset/arrow-keys). #37 multi-window Route A + vertical/grid splits still open if user wants more.

## Session 2026-06-14 (cont.131) — per-pane STT routing + per-tab workspace root (#38) → COMMITTED `ca5db9d`, live-verified

Fixed the two concrete split-pane pains the user reported (the #36 specifics). **Committed, not shipped** — no version bump; ready to `/git-ship` when wanted.
- **STT wrong-pane (a):** dictation wrote `assistant.composerDraft` (focused-pane shim); mic `onclick` beats the pane-focus bubble so text landed in the old pane. Fix: `stt.targetTabId` bound at `start(tabId)`, draft I/O via `readDraft/writeDraft`, composer passes `tabId` (mic + PTT), `sendRequested` gated on target. (`stt.svelte.ts`, `Composer.svelte`)
- **Per-tab root (b):** `cfg.current_root` was one global → switching a pane's folder leaked (esp. after `/clear`). Added `TabState.workspaceRoot` + `effectiveRoot/activeRoot`; new backend `assistant_set_tab_root` (canonicalize + record recent, **no `current_root` mutation**); `assistant_send` takes optional `root` (first-turn only, then per-session pinned as before); `newTab` snapshots focused root, `clearConversation` preserves the pane's own, disk-load hydrates from `sessionCwd`; @-mention/branch scope to `activeRoot`, caches dropped on focus change. (`turn.rs`, `workspace.rs`, `tabs/workspace/persistence.ts`, `ChatTabsBar`, `AssistantWelcome`) **3 files lockstep N/A (no version bump).**
- **Verified:** svelte-check 0/0 · vitest 132/132 · Rust recompiled (CDP-invoked new cmd → canonical) · live CDP: set focused→exfil-v1, new tab inherited it, restore→default, 0 errors. **Untested:** real mic; true cross-pane leak (live was single-pane — design-guaranteed). Detail: `docs/ISSUES.md` #38.

## Session 2026-06-14 (cont.130) — v0.9.5 R2 ship VERIFIED + windowing ideas logged

- **v0.9.5 SHIPPED + R2 verified live.** Empty-bucket root cause: v0.9.4's `release.yml` got the R2 secrets wired into the Release `env:` *after* the v0.9.4 tag, so v0.9.4's own CI run never reached `vpk upload s3`. v0.9.5 is the first tag through the corrected workflow. CI green (3m51s, run `27511422298`). **Verified:** `releases.win.json` → HTTP 200 (v0.9.5, SHA256 present) + `Rift-win-Setup.exe` → HTTP 200 (15MB) on the R2 public URL → auto-update feed + site download CTA both live. **Closes the cont.126 R2 RESUME item.** The 5 cont.129 local-llm commits rode along in the binary (gated off, inert).
- **Windowing ideas in `docs/ISSUES.md`** (T3, 🚧): #36 split-pane overhaul (2 concrete pains gathered + fixed in cont.131 #38; bigger "better" still un-prioritized) · #37 multi-window (Tauri 2 multi-`WebviewWindow`, backend per-session keyed; Route A MVP gotchas documented).

## Sessions cont.127–129 — Local-LLM (thinking-shim + probe + model picker) → WORKING, COMMITTED, gated/unshipped. Stack must be up: litellm `:4001` + shim `:4000` (`strip_thinking_proxy.py`); copies in `C:/AI Workflow/tools/litellm/`. Detail: `docs/design/local-llm.md` + git log.

## Open tails (v0.9.4–0.9.5 arc — detail in git log + `docs/design/self-hosted-distribution.md`)
- **Roll 2 exposed tokens** (R2 S3 + `cfut_` Pages) — optional, still pending.
- **RR-5** CSP prod-verify · **RR-8** Allow/Deny needs `trust_level=standard` · RR-11 code-signing? · RR-12 repo collapse (#17). Locked: D1 R2+Pages · D2 domain DEFERRED · D5 single `win` channel.

## Prior arcs — detail in git log + CHANGELOG
cont.123 ship-blockers + robustness → **v0.9.3**. cont.122 Activity dock removal. cont.121 tool-group cards → v0.9.2. cont.120 → v0.9.1. cont.119 minimal-core strip (3 workspaces) → v0.9.0. **§7 Harness rebuild OPEN**. cont.94 Fable 5 (Jun 22 sunset). PID-only kills, NEVER by image name.

## CRITICAL DON'T-TOUCH
- **Activity dock is GONE** (cont.122) — don't reintroduce `assistant.ui.dockOpen`/`dockWidth`. Live readout = composer LivePills; context = composer gauge + tabsbar ctx-pill; diff = Ctrl+Shift+D.
- **Tool-group grouping (cont.121):** `coalesceToolGroups` absorbs quick thoughts; threshold = TOOL count. Open = `expandedGroups.has(key) !== defaultOpen` (XOR), stores FLIPPED-from-default keys. Card + left status-rail (`::after`), NOT spine bullet — don't re-add a spine bullet to groups (steps-numbering unify kept this).
- **Live TabState authoritative over disk** — never re-add `stop()` to `loadConversation`.
- **Trust enum 2-level** — `full` rejected for new writes, MIGRATE read-side.
- **Effort mapping lockstep** (`effortToFlag` ↔ `turn.rs` ↔ `modelMatrix.ts` + vitest). **Right-click ownership** (`preventDefault()`).
- **Accent via `--accent-h`** (emerald 163); tint `in oklab`. Surface tiers: page .142 · card .215 · wells .178 · field .25 · track .175.
- **IA: 3 core workspaces** (Home·Chat·Settings) + **experimental Local LLM** (cont.127–129, kbd 4, yankable, COMMITTED but gated — no version bump, not shipped; needs the `scripts/local-llm/` proxy stack running for non-thinking models). **Versions lockstep ×3 + Cargo.lock** — only at ship. **v0.9.2 stands.**
