# Minimal-core strip — execution brief (Phase 2 of release campaign)

> Authored 2026-06-12 (planning session). User decision: **minimal core** for the buddy release, decisions delegated. Execute slices IN ORDER, verify green between slices, then Phase 3 (#34 fix) → Phase 4 (walkthrough) → Phase 5 (ship). All line numbers measured 2026-06-12 — re-anchor by snippet if the file was touched since.

## Decisions (locked)

| Feature | Verdict | Why |
|---|---|---|
| Harness workspace (telemetry + Cost + Swarm sub-tabs) | **CUT** | Niche diagnostics surface; nav drops to 3 workspaces |
| Edit swarm (UI + `swarm/` 824L backend) | **CUT** | Self-contained power feature |
| Cost cockpit (SQLite aggregate/budget/insights/pricing/store) | **CUT, keep plan-limit gauges** | `limits.rs` is independent (verified: zero refs to UsageDb/store); gauges = composer UsagePanel + Home t-limits tile |
| Session-log subsystem | **CUT** | Only consumers were Harness (read) + usage backfill (cut). In-memory `telemetry.turns` STAYS (ActivityPanel + healthAlerts) |
| Compaction (#33) | **CUT** | Broken per user; removal closes #33 without a /diagnose arc. Long chats → new tab |
| Custom providers + compression proxy | **CUT** | First-party Anthropic only. **API-key override STAYS** (`assistant_get/set_api_key`) |
| Per-turn cost cap (`max_budget_usd`) | KEEP | Safety guard, tiny, works |
| Speech/STT + enhance wand | KEEP | User's daily drivers, verified working |
| Browser dock | KEEP | `open_browser` MCP tool depends on it |
| SessionDiff (#34) | KEEP + FIX (Phase 3) | Trust feature; fix sketch in ISSUES |

## Slice order + exact surface

### S3 — Harness workspace cut (FIRST: kills the last UI consumers of S1/S4/S5 targets)
- DELETE `src/lib/components/workspaces/HarnessPage.svelte`, `CostPage.svelte`, `SwarmPage.svelte`, `workspaces/helpers.ts` (modelHue/shortModel — consumers are Harness+Cost only; re-grep first).
- `workspaces/index.ts`: drop harness entry + `HarnessPage` import; settings `kbd: "4"` → `"3"`. KEEP `Activity` lucide import (`WorkspaceIcon = typeof Activity` type anchor).
- `state/workspace.svelte.ts`: drop `"harness"` from `WorkspaceId` + `WORKSPACE_IDS`; delete `HarnessSubtab`, `targetHarnessSubtab`, `openHarness()`, `clearHarnessSubtab()`. Stale persisted `activeId="harness"` already falls back to chat via `isWorkspaceId`; stored order filtered the same way — no sweep needed (no Harness-owned localStorage keys; verified).
- `dialogs/CommandPalette.svelte:43`: remove Harness row; Settings sub → Ctrl+3. Check the `sections` deep-link list (line ~62) for harness anchors.
- `state/assistant/healthAlerts.ts:99,125`: reword two detail strings that point at "the Harness page / reliability card" (keep the alerts).
- Verify Ctrl+1..N is positional (WorkspaceShell/Titlebar) — expected generic via registry order.

### S1 — Swarm backend cut
- DELETE `src-tauri/src/swarm/` + `src/lib/state/swarm.svelte.ts`.
- `lib.rs`: drop `pub mod swarm` (L23) + `swarm::swarm_env_check` / `swarm::swarm_run` (L240-241).
- `SettingsPage.svelte:104-107` localTools copy: reword npm/cargo descriptions ("edit swarm's gate…").

### S4 — Session-log subsystem cut
- DELETE `src-tauri/src/assistant/session_log.rs` + `src/lib/state/assistant/sessionLog.ts`.
- `lib.rs` L216-220: drop `assistant_save/list/load/delete/prune_session_log(s)`; remove `commands/*.rs` + `assistant/mod.rs` re-exports.
- `assistant.svelte.ts`: L107 import (`saveSessionLog`, `pruneSessionLogs`, `ingestUsage`) + the debounced write timer (L605-632) + all call sites.

### S5 — Usage slim to plan gauges + Home rework
- Backend: DELETE `usage/aggregate.rs`, `budget.rs`, `insights.rs`, `pricing.rs`, `store.rs`; slim `usage/mod.rs` to `limits` only. `lib.rs`: drop `UsageDb` managed state (L123), the backfill setup block (L158-168), registrations L227-238. KEEP `usage::limits::usage_rate_limits` (L239).
- `state/usage.svelte.ts`: slim to `LimitWindow`/`ExtraUsage`/`RateLimits` types + `rateLimits`/`rateLimitsError` + `refreshRateLimits()`. Delete everything else (daily/monthly/byModel/byWorkspace/blocks/insights/budget/config/refresh/setBudget/allTime*).
- `home/HomePage.svelte`: keep tiles `t-ws`, `t-jump`, `t-limits`; DELETE `t-kpi` ×3 (today/month/burn), `t-spark` (+ its `openHarness("cost")` link), `t-insight`; drop `usage.refresh()` (L21) + cost deriveds (~L111-156); keep `usage.refreshRateLimits` (L22). Re-balance the bento grid CSS for 3 tiles (small design pass — keep the mission-control feel).
- `composer/UsagePanel.svelte`: unchanged (already gauges-only).

### S2 — Compaction cut (closes #33 as REMOVED)
- Frontend: DELETE `state/assistant/compaction.ts`. In `assistant.svelte.ts`: imports L132-133, tab fields `compactingNow`/`pendingSummary`/`forceNextFirstTurn` (~L233-257), auto-fire check (~L467), `autoCompactThreshold`/`compactModel` $state (L645-650), init loads (L945-953), setters (L1409-1420), `summarizeCurrentSession`/`compactConversation` wrappers (L1700-1730).
- `send.ts`: remove `forceNextFirstTurn`/`pendingSummary` consumption (first-turn seeding) — preserve plain `isFirstTurn` semantics.
- `persistence.ts` + `types.ts`: drop persisted compaction fields from save/load records (tolerate old records w/ extra keys).
- `ChatTabsBar.svelte`: remove both compact buttons (L602, L738) + `autoCompactThreshold` deriveds (L385-424); REWORK the ≥70% ctx nudge copy to suggest Ctrl+T only (keep the nudge!).
- `composer/SlashMenu.svelte`: remove `/compact` (+ `/summarize` debug) entries; check `Composer.svelte` dispatch arms for both.
- `SettingsPage.svelte`: Assistant-tab compaction rows (threshold slider + compact-model picker) + tab `sub` copy (L31).
- Check `HistoryDrawer`/`bubble/helpers` "compact" hits — most are CSS density false positives; the retired-session badge (if any) stays harmless or goes.
- Backend: `oneshot.rs` summarize/remint fns + `assistant://summarize-progress` emit; `commands` re-exports; `config.rs` `auto_compact_threshold`/`compact_model` fields + get/set; `lib.rs` L188-191 + L200-201. **KEEP `cleanup_retired_jsonls`** (L152-157 setup) — housekeeping for installs that already have retired JSONLs.
- Tests: `assistant.playback.test.ts` + `assistant.test.ts` reference compaction fields — update; expect vitest count to drop from 122.

### S6 — Providers + compression cut (first-party only)
- `SettingsPage.svelte`: provider list/form/presets + compression section (script ~L129-230, L304; markup ~L648-790; routing copy L652) + tab `sub` (L31).
- `assistant.svelte.ts`: `ProviderDto` import + providers/activeProvider/compression state, `saveProvider`/`deleteProvider`/`setActiveProvider`/`setCompression`/`compressionEnvCheck` + init loads.
- Backend: `config.rs` — provider CRUD commands (L389+), `ProviderProfile`/`ProviderDto`/`ProviderInput`, `resolve_active_provider` (L240), `provider_key_ref`, legacy migration block (L205-225), struct fields `base_url`/`provider_model`/`providers`/`active_provider_id`/`compression_enabled`/`compression_proxy_url` (serde ignores stale JSON keys — safe). DELETE `env_checks.rs` compression fns + tests (keep `environment_check` if it lives there — re-check; it's a separate general probe, KEEP it).
- `turn.rs`: L435 `resolve_active_provider` + L739-744 `ANTHROPIC_BASE_URL` seam — remove both arms; check `oneshot.rs` for the same seam (enhance/title paths).
- `lib.rs`: L192-198 (`assistant_list/save/delete_provider`, `assistant_set_active_provider`, `assistant_get/set_compression`, `compression_env_check`). KEEP L199 `environment_check`, L180-185 (api key, use_full_config, budget cap), L186-187 (trust).
- Note: orphaned `assistant.provider.*` keychain entries are left behind — acceptable, invisible.

### S7 — Docs + tracker sweep
- `docs/ISSUES.md`: #33 → closed (feature removed); fold #31's provider/Fable notes (provider remainder superseded by S6; Fable sweep still dated post-Jun 22); re-index.
- Project `CLAUDE.md`: hot-files table (drop swarm/usage rows, HarnessPage; re-measure assistant.svelte.ts/ChatTabsBar/SettingsPage after slices), backend dir list, frontend notes.
- CHANGELOG only at ship (/git-ship territory).

## Verification gates (every slice)
- Frontend slice → `npm run check` + `npx vitest run` green.
- Backend slice → `cargo check --manifest-path src-tauri/Cargo.toml` + `cargo test` green (NOT while tauri dev runs).
- After S7: full pass — svelte-check 0/0, vitest green, cargo check + tests clean.

## No-mistakes watchpoints
- **Untouchable:** trust enum 2-level migration arms · effort lockstep ×3 · MCP server + bridge env injection · `kill_all_session_children` re-export · Velopack chain · TabState-authoritative-over-disk · onboarding gate.
- localStorage `rift.ui.workspace*` self-heal already; no new sweeps needed.
- `telemetry.turns` (in-memory) is NOT the session-log subsystem — keep it.
- Removing serde fields is deserialize-safe; never add `deny_unknown_fields`.
- Stale `RIFT_TRUST_LEVEL`/bridge envs untouched by all slices.

## After Phase 2
- **Phase 3:** #34 SessionDiff — collapse groups >N files by default, memoize per-edit diff (header count + body share one), cap rendered lines w/ "show more". Repro w/ synthetic 20-file session first.
- **Phase 4:** CDP sweep per workspace (3 now) + fresh-machine pass: clean-profile install → onboarding → logged-out sign-in (closes Auth-Rec) → updater round-trip → #29 CSP prod verify (transitions + progress fill, zero violations).
- **Phase 5:** bump → CHANGELOG → tag-driven CI → install from real feed → smoke → distribute (warn buddies: SmartScreen "More info → Run anyway", unsigned).
