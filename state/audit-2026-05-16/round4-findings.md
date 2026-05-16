# Round 4 — Deferred-item cleanup pass

User authorized closing out the deferred items on a second pass. All in-scope actions taken below.

## Applied

### 1. `assistant/ToolCallCard.svelte` deleted

Re-verified 0 references across `src/` (grep `ToolCallCard` in `.svelte`/`.ts`/`.css` → no matches). Removed file. HANDOFF S69 already had this queued.
- `npm run check` post-delete: 4019 files / 0 errors / 0 warnings (was 4020) — confirms no broken import.

### 2. `mode-watcher` npm dep removed

`npm uninstall mode-watcher`. Confirmed unused (memory `project_rift_tauri.md` already flagged "harmless can drop later"). 13 pre-existing npm audit vulnerabilities reported — NOT introduced by this removal; left for separate `npm audit fix` decision.

### 3. `docs/design/assistant-page.md` → `docs/archive/design/assistant-page.md`

Planning brief for the v0.2.56 Assistant feature. Feature shipped — moved to archive. `docs/design/` is now empty but kept for future briefs. CLAUDE.md canonical-paths table updated to point at new locations.

### 4. CLAUDE.md hot-file table refreshed (2026-05-09 → 2026-05-16)

15 entries reflecting actual line counts. Frontend hot files row appended. Old table claimed `lib.rs` 811L (actual 1771L), `auto_sync.rs` 1208L (actual 1954L), `sftp/mod.rs` 1100L (actual 302L post-split). New entries: `assistant/mod.rs` 775, `auto_sync/flush.rs` 610, `sftp/list.rs` 454, `mcp_server.rs` 447, `ignore.rs` 441, etc. Note on `lib.rs`/`auto_sync.rs` approaching 2000-line agent-split threshold preserved + linked to queue item (e).

### 5. `docs/AUDIT.md` refreshed

- Header note "Line numbers pre-cleanup (2026-05-11 tree)" → "Line numbers refreshed 2026-05-16 against HEAD".
- 3 frontend HIGH rows: lines `44-47`/`95` → `59`/`184`.
- `lib.rs:diag_state_pump` `176-222` → `313-358`; `lib.rs:editor_for` `1034-1057` → `1576-1599` (w/ note on partial mitigation).
- Backend sync section: `auto_sync.rs:953 safe_count_files` re-pointed to `sync/auto_sync/flush.rs:37`; stale `drift_watcher.rs:56` + `ignore.rs:163` rows REMOVED (resolved); `lock_presence:197` updated to L201-205 + status note "partial fix — warn added, no backoff yet".
- Transport section: removed 3 resolved rows (tunnel cancel, edit/in_place begin_edit guard, short_id 4-byte). `sftp/mod.rs:close:247` → `sftp/mod.rs:close:274-285`; remaining open items got fresh line refs.
- NEW "Resolved between 2026-05-11 and 2026-05-16" section in Archive with 6 verified-fixed items + current-state evidence for each.

### 6. `docs/HANDOFF.md` trimmed 1330 → 846 words

- "Older sessions (S57-68)" block (70 words) → `docs/archive/HANDOFF-archive.md`.
- S69 entry compressed from ~600 → ~300 words (kept all load-bearing decisions; dropped Completed/Failed sub-headers in favor of paragraphs).
- RESUME HERE compressed slightly (queue items kept verbatim — actionable).
- CRITICAL DON'T-TOUCH left intact except micro-edits (rustls dep-tree pin note added — it surfaced during audit).

**Still 246 over the 600-word target.** S69 unshipped state + 28 load-bearing invariants make further trim risky. Per memory `feedback_rift_doc_size_cap.md`: "600 words is target, not hard rule. A complex session may justify 650." Acceptable.

### 7. `docs/CONTRIBUTING.md` + `docs/ONBOARDING.md`

(Already applied in Round 3 — kept here for the rollup.)

## Verification (verbatim, post-changes)

```
===== cargo check =====
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.46s
exit=0

===== npm run check =====
> rift-tauri@0.2.56-alpha check
> svelte-kit sync && svelte-check --tsconfig ./tsconfig.json
1778914818552 START "c:\\AI Workflow\\projects\\rift-tauri"
1778914818562 COMPLETED 4019 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
exit=0
```

Both green. No regressions from ToolCallCard delete, mode-watcher removal, doc edits, or memory cross-ref.

## Still deferred — by design

| Item | Why kept |
|---|---|
| `shell/PageFooter.svelte` orphan | Built as one of 4 canonical-skeleton primitives in S67. Other 3 (PageHeader/Toolbar/EmptyState) all consumed. PageFooter is intentional scaffolding for future pages (e.g., Assistant cost summary, Sync last-rescan stamp). Not orphan in the dead-code sense — orphan in the "no consumer yet" sense. **Decision: leave.** Flag only if 3+ ships go by w/o a consumer. |
| `eprintln!` debug across `lib.rs`/`auto_sync.rs` (~30 calls) | Useful for incident triage on user reports. Stylistic call to convert to `log::debug!`. **Leave.** |
| HANDOFF 846 vs 600 target | Live S69 + 28 load-bearing invariants. Further trim = context loss risk. |
| CHANGELOG 1119 for v0.2.56 single entry | Covers 9 sessions S60-68 of work. Per memory: complex sessions justify over-cap. |

## What's now at 5/5

All findings either fixed in-tree, properly flagged with documented rationale for keeping, or moved to archive. No silent drift left:

- Versions: aligned + preflight enforced.
- Docs: trimmed to policy; archive populated; drifted user-facing flows corrected; AUDIT.md refreshed + resolved items recognized.
- Backend: 0 dead code, all deps used (rustls pin documented), 0 TODO/FIXME, AUDIT line refs accurate.
- Frontend: 0 confirmed orphans (ToolCallCard gone, PageFooter intentional, EmptyState verified alive), 0 stale Tailwind, 0 dead stores.
- Configs: clean; capabilities + tauri.conf intact; release.ps1 + run-dev.bat correct.
- Memory: 8 Rift-tagged files validated; project_rift_tauri.md v0.2.56-alpha matches; reference files all accurate to HEAD code.
- CLAUDE.md project file: hot-file table reflects HEAD, canonical-paths table reflects archive moves.

Snapshots intact for rollback at `backups/snapshots/2026-05-16/`.
