# Round 3 — Configs + scripts + memory cross-ref + verification

## Surface 5 — Configs / capabilities / scripts

### Configs (all clean — no changes)

| File | Lines | Status |
|---|---|---|
| `svelte.config.js` | 19 | clean (adapter-static, SPA fallback for Tauri) |
| `vite.config.js` | 33 | clean (port 1420 strict, src-tauri ignored from watch) |
| `vitest.config.ts` | 32 | clean (node env, src-tauri excluded, $lib alias) |
| `tsconfig.json` | 20 | clean (extends `.svelte-kit/tsconfig.json`, strict mode on) |
| `src-tauri/capabilities/default.json` | 16 | clean — grants `core:window:allow-start-dragging` (gotcha #1) + `dialog:default` (S60 dialog plugin) + `opener:default` |
| `src-tauri/tauri.conf.json` | 45 | clean — alpha-safe `bundle.targets: ["nsis"]`, CSP locked-down |
| `scripts/release.ps1` | 139 | clean — 3-file version-sync preflight, vpk pack + GitHub upload, NSIS-only |
| `scripts/run-dev.bat` | 12 | clean — bare `npm run tauri dev` wrapper |

No drift on configs. `scripts/release.ps1` comment block accurately describes "bumping is manual or via `/git-ship`" (line 14-15) — `CONTRIBUTING.md` had the inaccurate "drives bump" line; fixed.

### Dependency findings

- `package.json`: **`mode-watcher`** dep is declared (`^1.1.0`) but **0 references in `src/`** (verified via grep `mode-watcher|mode_watcher|modeWatcher`). Memory `project_rift_tauri.md` already flags "Stale `mode-watcher` dep in `package.json` is harmless (can drop later)." Confirmed still unused. **FLAG** for future cleanup — not removing per "redundancy = FLAG, not delete" rule.
- `src-tauri/Cargo.toml`: 26 deps, 25 directly used. `rustls` (line 32) declared w/o direct `use` — intentional dep-tree pin to force `ring` crypto backend. **FLAG only — leave.**

## Memory cross-reference

All 8 Rift-tagged memory files validated against current code:

| File | Status | Notes |
|---|---|---|
| `project_rift_tauri.md` | ✅ accurate | v0.2.56-alpha matches. mode-watcher note still applies. All "Stack don't-touch" items verified against current code. |
| `feedback_rift_doc_size_cap.md` | ✅ accurate | ≤600 word cap. `docs/archive/` now exists (created this audit). CHANGELOG trimmed accordingly. |
| `feedback_rift_no_codesign.md` | ✅ accurate | `release.ps1:5` "signing deferred" comment confirmed historical/intentional. |
| `feedback_rift_tauri_build_full.md` | ✅ accurate | Build-batch policy matches current scripts/release.ps1 flow. |
| `reference_rift_atomic_replace_loop.md` | ✅ accurate | `recently_written: DashMap` lives at `auto_sync.rs:266`, `mark_recently_written` callsites confirmed in `drift_watcher.rs` (pull_one + delete_local_one). |
| `reference_russh_sftp_write_quirk.md` | ✅ accurate | `session.create()` (not `.write()`) confirmed at `sftp/transfer.rs:214,293,322`. Quirk fix held. |
| `reference_tauri2_drag_region.md` | ✅ accurate | `core:window:allow-start-dragging` present in capabilities/default.json. |
| `reference_self_replace_dance.md` | ✅ accurate | Generic Windows pattern, applies. |

**No memory updates required.** Salience levels all appropriate.

## In-scope doc fixes APPLIED this audit

| File | Change | Reason |
|---|---|---|
| `docs/CHANGELOG.md` | Trimmed 2501 → 1119 words. Removed v0.2.55/.54/.53 entries (1404 words). Header now points at archive + git log. | Per memory `feedback_rift_doc_size_cap.md` policy. Still over 600 cap because v0.2.56 entry alone covers 9 sessions (S60-68) of work — memory feedback explicit: "complex session may justify >600". |
| `docs/archive/CHANGELOG-archive.md` | Created (1404 words). Holds v0.2.55/.54/.53. | Belt-and-suspenders w/ git log. Per memory policy. |
| `docs/archive/` | Created (empty directory existed beforehand only as a documented intent). | Per memory `feedback_rift_doc_size_cap.md`. |
| `docs/ONBOARDING.md:28-32` | Updated "First sync" section. Renamed "Drift tab" → "Sync tab (Ctrl+2)", "AutoSync tab" → "Sync tab + Auto-rescan toggle", "Pull all"/"Push all" → "Sync button (pull-then-push)" + kebab note. Conflicts moved to Ctrl+4 reference. | Drift vs current v0.2.55 UI. Onboarding doc must match what new users actually see. |
| `docs/CONTRIBUTING.md:52` | Fixed `release.ps1 drives bump → build → vpk pack → vpk upload` → accurate version-bumped-manually-OR-by-/git-ship + script-does-build-pack-upload sequence. | Script doesn't bump; verified by reading source. |

## Deferred (FLAG, no action)

| Item | Why deferred |
|---|---|
| `CLAUDE.md` hot-file table | Stale by ~750 lines for `lib.rs`, ~750 for `auto_sync.rs`, omits `assistant/mod.rs`, `auto_sync/flush.rs`, `sftp/list.rs`. User-maintained file — flag for user-side refresh. |
| `docs/HANDOFF.md` over 600-word cap (1330) | All content is live S69 in-flight work + RESUME HERE + load-bearing CRITICAL DON'T-TOUCH invariants. Trimming risks losing context for next session. Acceptable per "complex session" exception. |
| `docs/design/assistant-page.md` (2846 words) | Planning brief for v0.2.56 Assistant feature. Feature shipped — could move to `docs/archive/`. Not blocking; user may want to preserve in-tree for design retrospectives. |
| `docs/AUDIT.md` line-anchor refresh | All open findings still valid in code; line numbers across the 4 frontend HIGH/MED + 16 backend rows are stale (verified `lib.rs:diag_state_pump` 176→313, `editor_for` 1034→1576, `RemotePane.svelte:44`→59, `AppShell.svelte:95`→184). Would need a dedicated pass; outside this audit's scope. |
| `assistant/ToolCallCard.svelte` orphan | HANDOFF S69 already queued for confirm-delete; deferred per "don't delete without confirmation" rule. |
| `shell/PageFooter.svelte` unused-built primitive | Implemented in S67 but no consumer page yet. Could prune or queue for a use. FLAG. |
| `mode-watcher` npm dep unused | Memory already flagged as "harmless can drop later". Not removing in this audit. |
| `eprintln!` debug instrumentation across `lib.rs` + `auto_sync.rs` (~30 calls) | Useful for incident triage; could convert to `log::debug!` macros. Stylistic — leave. |
| AUDIT.md items now RESOLVED (4 items) | Should be moved from Open to Archive section: tunnel cancel-on-stop, short_id 4-byte risk, delete_recursive_via is_symlink dep, lock_presence stale-delete log. Punt to dedicated AUDIT.md refresh. |

## Verification

```
===== cargo check =====
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.42s
exit=0

===== npm run check =====
> rift-tauri@0.2.56-alpha check
> svelte-kit sync && svelte-check --tsconfig ./tsconfig.json

1778914246779 START "c:\\AI Workflow\\projects\\rift-tauri"
1778914246789 COMPLETED 4020 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
exit=0
```

Both green. No edits introduced regressions.

## Audit complete

Snapshots intact at `backups/snapshots/2026-05-16/{audit-baseline.tar.gz,memory-rift-baseline.tar.gz}`. Round findings in `state/audit-2026-05-16/round[1-3]-findings.md`. Report card next.
