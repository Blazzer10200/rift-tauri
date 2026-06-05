# Rift — Issue Tracker

> Single source of truth for **open work only**. When something ships, **delete the block** — `git log -- docs/ISSUES.md` preserves history. Each block carries `Where` (file:line, may have drifted — re-grep before acting), `Symptom`, optional `Fix sketch`. Issue IDs are durable — never re-number, only append.
>
> Shipped Wave-1/2/3 audit blocks + clippy tables live in `docs/archive/audit-history.md`. Pruned 2026-06-04: the pure-assistant conversion (2026-06-03) removed the SFTP/sync/server/RCON stack, so every issue scoped to those subsystems was deleted here (history via `git log`).

---

## Active work — current sprint

> Live queue. HANDOFF.md = session state; this section = what's queued.

- **✅ RESOLVED in-tree — unshipped, delete on `/git-ship` (code re-verified 2026-06-05):** #31 (fast-mode hidden until wired, cont.57), #32 (`\\?\` strip, cont.54), #33 (Harness avg-dead-wait recompute on load, cont.54), #34 (palette "Go to Home", cont.54), #35 (per-recent "Forget" button, cont.54). Each carries a `✅` tag + verified file:line on its block below. Plus a **Harness no-scroll redesign** (KPI rail + collapsible "Show details"; not an ISSUES item — detail in HANDOFF cont.54). Blocks kept until ship so `git log` preserves them.
- **Steer feature — live-verify on a tool-using turn.** Mid-turn message injection shipped end-to-end (`assistant_steer` command, `STEER_TX` registry, `tokio::select!` reader, Alt+Enter trigger; brief in `docs/design/steer-and-queue.md`). Verified: compiles, `npm run check` clean, live CDP test accepted a mid-stream steer (`steer=steered`). Remaining: confirm a *visible* mid-turn redirect on a multi-step tool turn through the UI (pure-text turns complete before the steer lands — by design).
- **Permission round-trip — code-complete, needs live-verify.** Wired end-to-end: `--permission-prompt-tool stdio` (mod.rs) → `can_use_tool` handler → control-response write → `PermissionBar.svelte` Allow/Deny UI → `submitPermissionDecision()`. Remaining: live-verify with a throwaway repo — a git-write op in default/acceptEdits/plan mode should surface the Allow/Deny bar.
- **#30 Update UI redesign (queued — tomorrow).** The update toast + dialog look dated and under-organized — visual + IA refresh wanted. See #30 block below.
- **CR-UX (DECISION PENDING — user)** Trust segment is binary (Read-only/Standard) over a **ternary** backend enum (`readonly/standard/full`). Once clicked, `trust_level` pins and can't return to the derived state via UI; "full" (rank 2) is functionally identical to "standard" — only `"standard"` is gated for git writes. **Recommendation: collapse to a true 2-level enum** (drop dead "full"). Touches `mcp_server::trust_rank`/`trust_level`, `mod.rs::is_valid_trust_level`/`effective_trust_level`/git-write gate, serde, + persisted config migration. Held for user sign-off — security-relevant + persisted-config change.

### Active design briefs
- `docs/design/assistant-svelte-split.md` (#20 — M0-M7 shipped; M8 streaming + M9 send open)
- `docs/design/steer-and-queue.md` (steer/queue three-tier model — steer shipped; queue improvements + inline-bubble follow-ups open)

---

## 4. UI/UX consistency + navigability sweep (app-wide)

- **Scope:** Not a single bug — tracking the user's stated goal of an app-wide consistency pass. The Settings page is the densest control surface and the natural starting point.
- **Goal:** Every visible control is wired, every section is necessary, terminology + styling consistent. Navigation is intuitive.
- **Approach when actioned:** Per-page audit checklist (control → wired? necessary? consistent?). [src/lib/components/settings/SettingsPage.svelte](../src/lib/components/settings/SettingsPage.svelte) ~1064L (gutted of the old Server/RCON/SSH sections in the pure-assistant conversion) — audit still non-trivial.

## 14. No CI — release path local-only (CLOSED — by choice)

- `.github/workflows/check.yml` SHIPPED (cargo + svelte-check on PR). Release CI is **not being pursued** — it only made sense bundled with code-signing, which was **declined 2026-05-29** (SmartScreen friction not worth a recurring fee for a self-distributed alpha). Releases stay local via `scripts/release.ps1`. Reopen only if signing is reconsidered.

## 17. Two-repo split — historic, low-priority collapse

- **Where:** [scripts/release.ps1](../scripts/release.ps1) publishes to `Blazzer10200/rift-releases`; [src-tauri/src/update_service.rs](../src-tauri/src/update_service.rs) points Velopack's `GithubSource` at the same public repo.
- **Symptom:** Every release requires manual sync between the private source repo and the public releases repo. Forks/contributors can't test the update path against the real source.
- **Fix sketch:** Collapse to a single repo if the source repo goes public — a small change in `release.ps1` + the update source constant.

## 20. Hot files exceeding the 2000-line agent-split threshold

- **Where:** Per CLAUDE.md agent-routing guidance, files >2000 lines are agent-bail risks. Open targets (re-measured 2026-06-04):
  - [src-tauri/src/assistant/mod.rs](../src-tauri/src/assistant/mod.rs) — **~3331L (worst)**: Claude CLI spawn + auth + workspace + config + per-turn stream. Next backend split candidate.
  - [src/lib/state/assistant.svelte.ts](../src/lib/state/assistant.svelte.ts) — ~2479L (M0-M7 carved from 3356L; M8/M9 open).
- **Symptom:** Targeted edits become brittle, LSP slows, agents bail mid-emit on audit-shaped prompts.
- **Fix sketch:** `assistant.svelte.ts` next — design brief in [docs/design/assistant-svelte-split.md](design/assistant-svelte-split.md) (9-module extraction, ranked by blast radius). Then continue `assistant/mod.rs` extraction.
- **Status:** M0-M7 SHIPPED (`assistant.svelte.ts` 3356L → ~2479L). M8 (streaming pump) + M9 (send orchestrator) still open — the two highest-blast-radius extractions; deferred until a conversation-playback test harness exists.

## 21. Test coverage — thin after the pure-assistant rip

- **Where (2026-06-04):** ~9 Rust tests remain (`assistant/git_local.rs`, `stt/vad.rs`, `stt/whisper.rs`) + 1 vitest file (`src/lib/state/assistant.test.ts`, mocks Tauri IPC over the assistant store). The former 115-test lib suite + 7 live-SFTP `#[ignore]` integration tests + the `DriftScanner`/`SftpOps` mock layer were all removed with the sync engine.
- **Symptom:** The surviving high-risk surface — the per-turn stream/reader in `assistant/mod.rs` and the store orchestrator in `assistant.svelte.ts` — has no end-to-end coverage. A regression in the stream pump or the send/queue/steer path can break a turn silently.
- **Fix sketch:** Build a conversation-playback harness (feed recorded NDJSON frames through the reader + store) — also the unblocker for the #20 M8/M9 extractions. Then cover the git_local MCP tools against a throwaway repo.

## 30. Update UI — visual + organizational redesign

- **Where (re-grep — may have drifted):** the update toast/notification, [UpdateDialog.svelte](../src/lib/components/dialogs/UpdateDialog.svelte), and [updates.svelte.ts](../src/lib/state/updates.svelte.ts). The auto-update *flow* (Velopack check → download → apply-on-exit → relaunch) works correctly and is verified live (v0.4.48 → v0.5.0); this is **presentation only**, not the update mechanism.
- **Symptom:** The "Update available" toast + the dialog look dated and feel under-organized — spacing, hierarchy, and styling don't match the current emerald/bento design language of the rest of the app (Home, Harness, Settings). The toast shows `0.4.48 → 0.5.0 · 9.5 MB`; the layout reads as legacy.
- **Fix sketch:** Visual + IA refresh pass over the toast and the available/downloading/installing/error states of `UpdateDialog`. Align to the app's surface tiers + accent (`--accent`, never hardcoded), tighten the version/size/progress hierarchy, and make the available→downloading→installing progression read smoothly (the installing-state height-jump was partly addressed in v0.4.47 — finish the job). Keep the "View release on GitHub" fallback. Pure CSS/markup + state-presentation; don't touch the Velopack invoke contract.

## 29. CSP allows `style-src 'unsafe-inline'` (LOW)

- **Where:** [src-tauri/tauri.conf.json](../src-tauri/tauri.conf.json) `csp`.
- **Symptom:** Inline styles permitted — required by current Tailwind output, weakens CSP.
- **Fix sketch:** Switch to nonce/strict-dynamic once Tailwind supports hashed inline styles end-to-end.

## 31. Fast-mode toggle is cosmetic — not plumbed to CLI spawn (LOW)

- **✅ RESOLVED in-tree (2026-06-05, cont.57) — unshipped, delete on ship:** Chose **hide-until-wired** (the "don't ship a lying control" half of the fix sketch). The toggle is now gated behind `FAST_MODE_WIRED = false` **and** the per-model `fastMode` capability flag in [Composer.svelte](../src/lib/components/assistant/Composer.svelte) — so it renders nowhere today, and reappears **Opus-only** the moment the CLI side is wired (flip one const). `uiPrefs.fastMode` state + persistence kept dormant for that wiring. Block kept until `/git-ship`.
- **Where:** [src/lib/state/ui-prefs.svelte.ts:33-35](../src/lib/state/ui-prefs.svelte.ts) (`fastMode` $state + its own TODO), toggle UI in [src/lib/components/assistant/Composer.svelte:~1175](../src/lib/components/assistant/Composer.svelte). Persisted under `rift.ui.fast-mode.v1`.
- **Symptom (was):** The Composer model-menu showed a working fast-mode toggle that persists its on/off intent to localStorage, but the value is never read by the CLI-spawn path in `assistant.svelte.ts` / `assistant/mod.rs` — flipping it had zero effect on the turn. A visible control that did nothing.
- **Remaining (to fully close on a future arc):** Plumb `uiPrefs.fastMode` into the spawn envelope alongside the effort flag (CC's `/fast` = Opus-with-faster-output, valid on Opus 4.6/4.7/4.8 only), then flip `FAST_MODE_WIRED` to surface it Opus-only. Surfaced during the 2026-06-04 harness/audit pass.

## 32. `\\?\` extended-length path prefix leaks into UI (LOW — cosmetic)

- **✅ RESOLVED in-tree (cont.54) — unshipped, delete on ship:** [HomePage.svelte:50](../src/lib/components/home/HomePage.svelte) strips `^\\\\\?\\UNC\\` → `\\\\` and `^\\\\\?\\` → `` for display only (inline regex, not the `cleanPath()` helper — that one lives only in HarnessPage). Verified 2026-06-05.
- **Where:** [HomePage.svelte](../src/lib/components/home/HomePage.svelte) Workspace card path display (showed `\\?\C:\AI Workflow\projects\remotion-playground`). Harness chips use `cleanPath()`; Home now strips inline.
- **Symptom:** The raw Win32 extended-length prefix `\\?\` is shown verbatim in the Home workspace path. Ugly; reads as a bug to users.
- **Fix sketch:** Strip a leading `\\?\` (and `\\?\UNC\`) for display only — never on the value passed to the backend. Centralize in the existing `cleanPath` helper and apply on the Home card. Surfaced during the 2026-06-05 live-UI test pass (~3 AM session).

## 33. Harness "avg dead wait" shows "—" for archived sessions (LOW-MED — legacy data)

- **✅ RESOLVED in-tree (cont.54) — unshipped, delete on ship:** [sessionLog.ts:50](../src/lib/state/assistant/sessionLog.ts) recomputes `snap.summary = summarizeSession(snap.turns, snap.events)` on load — the frozen on-disk summary is discarded, so new `summarize()` fields backfill for old logs. Verified 2026-06-05.
- **Where:** [src/lib/components/workspaces/HarnessPage.svelte:237](../src/lib/components/workspaces/HarnessPage.svelte) (binds `fmtMs(sum.avgDeadWaitMs)`), `sum = source.summary` where `source` comes from [loadSessionLog](../src/lib/state/assistant/sessionLog.ts) → `assistant_load_session_log` returning the **persisted** `SessionSnapshot` incl. its frozen `summary` (serialized at save via [telemetry.ts:35](../src/lib/state/assistant/telemetry.ts)).
- **Symptom:** For sessions logged **before** cont.53 added `avgDeadWaitMs` to `summarize()`, the persisted summary lacks the field → `fmtMs(undefined)` → "—". Meanwhile the per-turn `.tl-dead` timeline markers ([HarnessPage.svelte:203,660](../src/lib/components/workspaces/HarnessPage.svelte)) recompute live from raw `source.turns` and DO render. So the aggregate stat and the timeline permanently disagree for any pre-field session. **Live/new sessions compute correctly** (verified `avg dead wait 6.2s` on a fresh turn) — this is purely stale persisted-summary data.
- **Fix sketch:** Don't trust the frozen summary on load — recompute `summary` from the persisted `turns[]` in `loadSessionLog` (or in the `source` derived) so new derived fields backfill for old logs. Same pattern would future-proof any later `summarize()` additions. Surfaced 2026-06-05 live-UI test (this was the cont.53 RESUME-HERE verify item).

## 34. Command palette omits "Go to Home" (LOW — consistency)

- **✅ RESOLVED in-tree (cont.54) — unshipped, delete on ship:** [CommandPalette.svelte:40](../src/lib/components/dialogs/CommandPalette.svelte) now has `{ id: "home", label: "Home", icon: Home, sub: "Ctrl+1" }` as the first `navs` entry. Verified 2026-06-05.
- **Where:** [src/lib/components/dialogs/CommandPalette.svelte:39-42](../src/lib/components/dialogs/CommandPalette.svelte) — the `navs` array lists only `chat`/`harness`/`settings`.
- **Symptom:** Home (Ctrl+1) is a primary workspace but has no "Go to Home" entry in the palette's GO TO group, while every other workspace does. Inconsistent.
- **Fix sketch:** Add `{ id: "home", label: "Home", icon: Home, sub: "Ctrl+1" }` as the first `navs` entry. One-line addition. Surfaced 2026-06-05 live-UI test.

## 35. Deleted project still in Home "Recent folders" (LOW — stale recents)

- **✅ RESOLVED in-tree (cont.54) — unshipped, delete on ship:** chose the **manual-prune** half of the fix — [HomePage.svelte:208](../src/lib/components/home/HomePage.svelte) renders a per-row "Forget" (×) button → `assistant.removeRecentRoot(r)` (→ `wsRemoveRecentRoot`, [assistant.svelte.ts:1671](../src/lib/state/assistant.svelte.ts)), so a dead recent can be dismissed. Verified 2026-06-05.
- **Where:** Home "Recent folders" list, [HomePage.svelte:197-209](../src/lib/components/home/HomePage.svelte) (`recents = assistant.workspace.recent`).
- **Symptom:** `vein-modding` (retired + deleted 2026-06-04 per workspace CLAUDE.md) still appeared in Recent Folders; selecting it pointed at a non-existent dir. Also `Coinsmith` listed.
- **Remaining (optional, future):** no auto-validation against existence on mount — dead entries still appear until manually Forgotten, and a missing-dir click isn't yet proven to fail gracefully. Auto-grey/prune-on-mount would fully close it.

---

## Priority tiers

**Tier 1 — ship blockers / data safety**
- #21 Test coverage — surviving stream/store paths uncovered after the rip.

**Tier 2 — needs live-verify (code-complete)**
- Steer mid-turn redirect on a tool turn · Permission round-trip Allow/Deny bar.

**Tier 3 — strategic / longer-term**
- **#30 Update UI redesign (next up — queued for tomorrow)** · #4 App-wide UX consistency sweep · #20 hot-file split M8-M9 · #17 two-repo collapse · CR-UX trust-enum decision (user sign-off).

**Tier 4 — backend LOW (opportunistic)**
- #29 CSP `style-src 'unsafe-inline'` (Tailwind-blocked) · ~~#31 fast-mode toggle cosmetic~~ (✅ resolved in-tree cont.57 — hidden until wired) · Wave-1 LOWs #91-#134 — clippy/doc/perf nits (see `docs/archive/audit-history.md`).
