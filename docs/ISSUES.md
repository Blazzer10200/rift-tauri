# Rift — Issue Tracker

> **Single source of truth for open work only.** When something ships, **delete its block** — `git log -- docs/ISSUES.md` preserves history. Issue IDs are **durable**: never re-number, only append. Each block carries `Where` (file:line — may have drifted, re-grep before acting), `Symptom`, and an optional `Fix sketch`.
>
> Shipped Wave-1/2/3 audit blocks + clippy tables live in `docs/archive/audit-history.md`. Pruned 2026-06-04: the pure-assistant conversion (2026-06-03) removed the SFTP/sync/server/RCON stack, so every issue scoped to those subsystems was deleted here (history via `git log`).

### Conventions

- **Status** — `✅ resolved in-tree` (fixed, unshipped — block stays until `/git-ship` so `git log` keeps it) · `🚧 open` · `👤 needs your call` · `🧪 live-verify` (code-complete, needs runtime confirmation) · `🔒 blocked` (external dep) · `🗄 closed` (decided, kept for the record).
- **Tier** — `T1` ship-blocker / data-safety · `T2` code-complete needs live-verify · `T3` strategic / longer-term · `T4` LOW / cosmetic.

### Index

| ID | Title | Tier | Status |
|----|-------|------|--------|
| #31 | Fast-mode toggle cosmetic → hidden until wired | T4 | ✅ resolved (ship) |
| #32 | `\\?\` path prefix leaks into Home UI | T4 | ✅ resolved (ship) |
| #33 | Harness "avg dead wait" "—" for archived sessions | T4 | ✅ resolved (ship) |
| #34 | Command palette omits "Go to Home" | T4 | ✅ resolved (ship) |
| #35 | Deleted project lingers in Home "Recent folders" | T4 | ✅ resolved (ship) |
| #36 | Settings scroll-spy skips last section ("About") | T4 | ✅ resolved (ship) |
| #21 | Test coverage thin after the pure-assistant rip | T1 | 🚧 open |
| Steer | Mid-turn redirect on a tool-using turn | T2 | 🧪 live-verify |
| Permission | Allow/Deny round-trip bar | T2 | 🧪 live-verify |
| #30 | Update UI — visual + IA redesign | T3 | 🚧 open |
| #4 | App-wide UX consistency + navigability sweep | T3 | 🚧 open |
| #20 | Hot files over the 2000-line split threshold | T3 | 🚧 open |
| #17 | Two-repo split → collapse | T3 | 🔒 blocked |
| CR-UX | Trust segment binary-vs-ternary enum | T3 | 👤 needs your call |
| #29 | CSP allows `style-src 'unsafe-inline'` | T4 | 🔒 blocked |
| #14 | No release CI — local-only path | — | 🗄 closed |

---

## ✅ Resolved in-tree — delete on `/git-ship`

> Fixed in earlier conts, unshipped. Blocks ride the next ship commit, then get deleted. Each was code-re-verified 2026-06-05. A non-ISSUES change shipping alongside: the **Harness no-scroll redesign** (KPI rail + collapsible "Show details" — detail in HANDOFF cont.54).

### 31. Fast-mode toggle is cosmetic — not plumbed to CLI spawn (T4)

- **✅ Resolved (2026-06-05, cont.57):** chose **hide-until-wired** (the "don't ship a lying control" half of the fix). The toggle is gated behind `FAST_MODE_WIRED = false` **and** the per-model `fastMode` capability flag in [Composer.svelte](../src/lib/components/assistant/Composer.svelte) — renders nowhere today, reappears **Opus-only** the moment the CLI side is wired (flip one const). `uiPrefs.fastMode` state + persistence kept dormant for that wiring.
- **Where:** [src/lib/state/ui-prefs.svelte.ts:33-35](../src/lib/state/ui-prefs.svelte.ts) (`fastMode` $state + its own TODO); toggle UI in [src/lib/components/assistant/Composer.svelte:~1175](../src/lib/components/assistant/Composer.svelte). Persisted under `rift.ui.fast-mode.v1`.
- **Symptom (was):** the Composer model-menu showed a working fast-mode toggle that persisted on/off to localStorage, but the value was never read by the CLI-spawn path (`assistant.svelte.ts` / `assistant/mod.rs`) — flipping it had zero effect. A visible control that did nothing.
- **Remaining (future arc, full close):** plumb `uiPrefs.fastMode` into the spawn envelope alongside the effort flag (CC's `/fast` = Opus-with-faster-output, Opus 4.6/4.7/4.8 only), then flip `FAST_MODE_WIRED` to surface it Opus-only.

### 32. `\\?\` extended-length path prefix leaks into UI (T4 — cosmetic)

- **✅ Resolved (cont.54):** [HomePage.svelte:50](../src/lib/components/home/HomePage.svelte) strips `^\\\\\?\\UNC\\` → `\\\\` and `^\\\\\?\\` → `` for display only (inline regex, not the `cleanPath()` helper — that one lives only in HarnessPage).
- **Where:** [HomePage.svelte](../src/lib/components/home/HomePage.svelte) Workspace card path display (showed `\\?\C:\AI Workflow\projects\remotion-playground`). Harness chips use `cleanPath()`; Home now strips inline.
- **Symptom:** the raw Win32 extended-length prefix `\\?\` shown verbatim in the Home workspace path. Ugly; reads as a bug.

### 33. Harness "avg dead wait" shows "—" for archived sessions (T4 — legacy data)

- **✅ Resolved (cont.54):** [sessionLog.ts:50](../src/lib/state/assistant/sessionLog.ts) recomputes `snap.summary = summarizeSession(snap.turns, snap.events)` on load — the frozen on-disk summary is discarded, so new `summarize()` fields backfill for old logs.
- **Where:** [HarnessPage.svelte:237](../src/lib/components/workspaces/HarnessPage.svelte) binds `fmtMs(sum.avgDeadWaitMs)`, `sum = source.summary`; `source` from [loadSessionLog](../src/lib/state/assistant/sessionLog.ts) → `assistant_load_session_log` returning the **persisted** `SessionSnapshot` incl. its frozen `summary` (serialized at save via [telemetry.ts:35](../src/lib/state/assistant/telemetry.ts)).
- **Symptom:** sessions logged before cont.53 added `avgDeadWaitMs` lacked the field → `fmtMs(undefined)` → "—", while the per-turn `.tl-dead` timeline markers recomputed live and DID render — so aggregate and timeline permanently disagreed for any pre-field session. Live/new sessions always computed correctly; purely stale persisted-summary data.

### 34. Command palette omits "Go to Home" (T4 — consistency)

- **✅ Resolved (cont.54):** [CommandPalette.svelte:40](../src/lib/components/dialogs/CommandPalette.svelte) now has `{ id: "home", label: "Home", icon: Home, sub: "Ctrl+1" }` as the first `navs` entry.
- **Where:** [CommandPalette.svelte:39-42](../src/lib/components/dialogs/CommandPalette.svelte) — the `navs` array previously listed only `chat`/`harness`/`settings`.
- **Symptom:** Home (Ctrl+1) is a primary workspace but had no "Go to Home" entry in the palette's GO TO group, while every other workspace did.

### 35. Deleted project still in Home "Recent folders" (T4 — stale recents)

- **✅ Resolved (cont.54):** chose the **manual-prune** half — [HomePage.svelte:208](../src/lib/components/home/HomePage.svelte) renders a per-row "Forget" (×) button → `assistant.removeRecentRoot(r)` (→ `wsRemoveRecentRoot`, [assistant.svelte.ts:1671](../src/lib/state/assistant.svelte.ts)), so a dead recent can be dismissed.
- **Where:** Home "Recent folders" list, [HomePage.svelte:197-209](../src/lib/components/home/HomePage.svelte) (`recents = assistant.workspace.recent`).
- **Symptom:** `vein-modding` (retired + deleted 2026-06-04) still appeared; selecting it pointed at a non-existent dir. `Coinsmith` also listed.
- **Remaining (optional, future):** no auto-validation against existence on mount — dead entries appear until manually Forgotten, and a missing-dir click isn't yet proven to fail gracefully. Auto-grey/prune-on-mount would fully close it. Live-confirmed 2026-06-05 (cont.58): both still visible, Forget (×) present + wired.

### 36. Settings sidebar scroll-spy never activates the last section ("About") (T4 — cosmetic)

- **✅ Resolved (2026-06-05, cont.58):** [SettingsPage.svelte:45-49](../src/lib/components/settings/SettingsPage.svelte) `onScroll()` now bottom-detects (`scrollTop + clientHeight >= scrollHeight - 2`) and explicitly spies the last `ST_SECTIONS` entry — so reaching the bottom (incl. via the About nav click → `jump()` smooth-scroll) lights "About" instead of leaving "Speech" stuck. `npm run check` 0/0.
- **Where:** [SettingsPage.svelte](../src/lib/components/settings/SettingsPage.svelte) — one scrolling column with a scroll-spy sidebar (Appearance/Accessibility/Assistant/Speech/About). Active-highlight driven by a positional 140px-from-top threshold in `onScroll`, not by the click.
- **Symptom:** clicking About (last item) scrolled it fully into view but the sidebar kept Speech lit. Root cause: About is the last/short section and the container can't scroll it to the top, so the threshold never fired for it. Affected any section that can't reach the top on click.

---

## 🚧 Open issues

### Tier 1 — ship-blocker / data-safety

#### 21. Test coverage — thin after the pure-assistant rip

- **Where (2026-06-04):** ~9 Rust tests remain (`assistant/git_local.rs`, `stt/vad.rs`, `stt/whisper.rs`) + 1 vitest file (`src/lib/state/assistant.test.ts`, mocks Tauri IPC over the assistant store). The former 115-test lib suite + 7 live-SFTP `#[ignore]` integration tests + the `DriftScanner`/`SftpOps` mock layer were all removed with the sync engine.
- **Symptom:** the surviving high-risk surface — the per-turn stream/reader in `assistant/mod.rs` and the store orchestrator in `assistant.svelte.ts` — has no end-to-end coverage. A regression in the stream pump or the send/queue/steer path can break a turn silently.
- **Fix sketch:** build a conversation-playback harness (feed recorded NDJSON frames through the reader + store) — also the unblocker for the #20 M8/M9 extractions. Then cover the git_local MCP tools against a throwaway repo.

### Tier 2 — code-complete, needs live-verify

#### Steer — mid-turn redirect on a tool-using turn

- **Status:** mid-turn message injection shipped end-to-end (`assistant_steer` command, `STEER_TX` registry, `tokio::select!` reader, Alt+Enter trigger; brief in `docs/design/steer-and-queue.md`). Verified: compiles, `npm run check` clean, live CDP test accepted a mid-stream steer (`steer=steered`).
- **Remaining:** confirm a *visible* mid-turn redirect on a multi-step tool turn through the UI (pure-text turns complete before the steer lands — by design).

#### Permission — Allow/Deny round-trip bar

- **Status:** wired end-to-end — `--permission-prompt-tool stdio` (mod.rs) → `can_use_tool` handler → control-response write → `PermissionBar.svelte` Allow/Deny UI → `submitPermissionDecision()`.
- **Remaining:** live-verify with a throwaway repo — a git-write op in default/acceptEdits/plan mode should surface the Allow/Deny bar.

### Tier 3 — strategic / longer-term

#### 30. Update UI — visual + organizational redesign

- **Where (re-grep — may have drifted):** the update toast/notification, [UpdateDialog.svelte](../src/lib/components/dialogs/UpdateDialog.svelte), and [updates.svelte.ts](../src/lib/state/updates.svelte.ts). The auto-update *flow* (Velopack check → download → apply-on-exit → relaunch) works and is verified live (v0.4.48 → v0.5.0); this is **presentation only**, not the mechanism.
- **Symptom:** the "Update available" toast + dialog look dated and under-organized — spacing, hierarchy, and styling don't match the current emerald/bento design language (Home, Harness, Settings). The toast shows `0.4.48 → 0.5.0 · 9.5 MB`; the layout reads as legacy.
- **Fix sketch:** visual + IA refresh over the toast and the available/downloading/installing/error states of `UpdateDialog`. Align to the app's surface tiers + accent (`--accent`, never hardcoded), tighten the version/size/progress hierarchy, smooth the available→downloading→installing progression (the installing-state height-jump was partly addressed in v0.4.47 — finish it). Keep the "View release on GitHub" fallback. Pure CSS/markup + state-presentation; don't touch the Velopack invoke contract.
- **Note:** taste-driven on an already-polished dialog → best done with the live states visible (CDP) or under review, not blind.

#### 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** not a single bug — tracks the stated goal of an app-wide consistency pass. The Settings page is the densest control surface and the natural starting point.
- **Goal:** every visible control is wired, every section is necessary, terminology + styling consistent, navigation intuitive.
- **Approach when actioned:** per-page audit checklist (control → wired? necessary? consistent?). [SettingsPage.svelte](../src/lib/components/settings/SettingsPage.svelte) ~1064L (gutted of the old Server/RCON/SSH sections in the pure-assistant conversion) — audit still non-trivial.

#### 20. Hot files exceeding the 2000-line agent-split threshold

- **Where:** per CLAUDE.md agent-routing guidance, files >2000 lines are agent-bail risks. Open targets (re-measured 2026-06-04):
  - [src-tauri/src/assistant/mod.rs](../src-tauri/src/assistant/mod.rs) — **~3331L (worst)**: Claude CLI spawn + auth + workspace + config + per-turn stream. Next backend split candidate.
  - [src/lib/state/assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) — ~2479L (M0-M7 carved from 3356L; M8/M9 open).
- **Symptom:** targeted edits become brittle, LSP slows, agents bail mid-emit on audit-shaped prompts.
- **Fix sketch:** `assistant.svelte.ts` next — design brief in [docs/design/assistant-svelte-split.md](design/assistant-svelte-split.md) (9-module extraction, ranked by blast radius). Then continue `assistant/mod.rs` extraction.
- **Status:** M0-M7 SHIPPED (`assistant.svelte.ts` 3356L → ~2479L). M8 (streaming pump) + M9 (send orchestrator) open — the two highest-blast-radius extractions; deferred until a conversation-playback test harness exists (see #21).

#### 17. Two-repo split — historic, low-priority collapse (🔒 blocked)

- **Where:** [scripts/release.ps1](../scripts/release.ps1) publishes to `Blazzer10200/rift-releases`; [src-tauri/src/update_service.rs](../src-tauri/src/update_service.rs) points Velopack's `GithubSource` at the same public repo.
- **Symptom:** every release requires manual sync between the private source repo and the public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** collapse to a single repo **if the source repo goes public** — a small change in `release.ps1` + the update source constant. Blocked on that decision.

#### CR-UX. Trust segment binary-vs-ternary enum (👤 needs your sign-off)

- **Symptom:** the trust segment is binary (Read-only/Standard) over a **ternary** backend enum (`readonly/standard/full`). Once clicked, `trust_level` pins and can't return to the derived state via UI; "full" (rank 2) is functionally identical to "standard" — only `"standard"` is gated for git writes.
- **Recommendation:** collapse to a true 2-level enum (drop the dead "full"). Touches `mcp_server::trust_rank`/`trust_level`, `mod.rs::is_valid_trust_level`/`effective_trust_level`/git-write gate, serde, + persisted-config migration.
- **Held for sign-off:** security-relevant + persisted-config change.

### Tier 4 — LOW / cosmetic

#### 29. CSP allows `style-src 'unsafe-inline'` (🔒 blocked)

- **Where:** [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) `csp`.
- **Symptom:** inline styles permitted — required by current Tailwind output, weakens CSP.
- **Fix sketch:** switch to nonce/strict-dynamic once Tailwind supports hashed inline styles end-to-end. Blocked on Tailwind.

> Also parked: **Wave-1 LOWs #91–#134** — clippy/doc/perf nits, in `docs/archive/audit-history.md` (not tracked live here).

---

## 🗄 Closed — kept for the record

### 14. No CI — release path local-only (closed by choice)

- `.github/workflows/check.yml` SHIPPED (cargo + svelte-check on PR). Release CI is **not being pursued** — it only made sense bundled with code-signing, which was **declined 2026-05-29** (SmartScreen friction not worth a recurring fee for a self-distributed alpha). Releases stay local via `scripts/release.ps1`. Reopen only if signing is reconsidered.

---

## Investigated 2026-06-05 — NOT bugs (don't re-chase)

- **`ReferenceError: MessageCircle is not defined` in the console ring** — STALE. The symbol IS imported ([HarnessPage.svelte:4](../src/lib/components/workspaces/HarnessPage.svelte)) and used (~line 537); the error was a transient intermediate-HMR artifact from ~50min before the pass. Navigating to Harness throws nothing now.
- **Model-menu rows "don't switch on click"** — NOT a bug. Rows use `onmousedown` ([Composer.svelte:1209](../src/lib/components/assistant/Composer.svelte)) (fires before blur so the menu doesn't close first); a synthetic `click` simply doesn't trigger them. Real pointer + keyboard both work.

---

## Active design briefs

- `docs/design/assistant-svelte-split.md` (#20 — M0-M7 shipped; M8 streaming + M9 send open)
- `docs/design/steer-and-queue.md` (steer/queue three-tier model — steer shipped; queue improvements + inline-bubble follow-ups open)

---

## Last full-app verification

- **🔍 Full-app CDP stress pass 2026-06-05 (cont.58) — app healthy.** Walked every workspace + dialog live (Home · Chat · Harness · Settings · command palette · History drawer · Web-browser panel · Panels menu). Ran a real read-only backend turn end-to-end (CLI spawn → MCP `grep`/`glob`/`list_dir`/`read_file` → stream → cost/context/activity render — all correct). Stress: 12 rapid workspace switches + a 14.8K-char emoji/unicode/`<script>` composer paste (auto-grew to the 340px cap, inert, no XSS). **Console: 0 errors / 0 warnings the whole session.** Verified live: cont.57 model/effort capability matrix (Haiku hides slider + shows the no-effort caption), #31–#35 fixes, themeable accent incl. amber warm-hue with no oklch purple-wrap, Harness no-scroll + trust-gated git tools. One new defect found → #36 (now resolved). Could NOT live-exercise: #30 update toast/dialog (app up-to-date on v0.5.0 → state never renders) and first-run onboarding (next-launch only).
