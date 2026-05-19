# Round 2 — Backend + Frontend dead-code sweep

## Backend (src-tauri/src/) — 30 files, 10,806 lines

### Dead code findings

**Nothing meaningful to remove.** The codebase is well-maintained. Net findings:

| Status | Item | Reason |
|---|---|---|
| ⚠️ INTENTIONAL | `rustls` dep in `Cargo.toml:32` | 0 direct `use rustls::` refs, but declared to FORCE `ring` crypto provider into the dep tree (blocks `aws-lc-rs` which needs NASM). Comment on the line documents intent. NOT a real dead-dep — leave. |
| LEAVE | `edit_trail::Entry` pub struct | Used internally only via `serde_json` round-trip. Could downgrade to `pub(crate)`, but Entry models the on-disk trail-file row schema — exposing as pub documents the file format. Leave. |
| LEAVE | `sync_snapshot::Entry` pub struct | Same pattern — accessed via fields by `drift_scanner.rs:381` (`snap_e.sha1`, `.local_size`, etc.). NOT dead. |

### Cargo.toml deps (26 listed) — all used

26/26 deps have at least one direct `use` or qualified path. `rustls` is the only zero-`use` case — intentional pin (see above).

### `eprintln!` debug noise — non-blocking FLAG

30 `eprintln!` calls instrument the slow paths (force_push_now, sync_pull_pending, sync_reconcile, force_pull_now, list_via_exec, sftp::ops probe-write cleanup). Tagged `[rift]` prefix. Useful for triage on user reports. Could be converted to `log::debug!` since `env_logger` is wired through `LogForwarder`, but that's a stylistic call — leaving as-is per "FLAG, not fix" doctrine.

Files w/ `eprintln!`: `lib.rs` (12), `auto_sync.rs` (13), `sftp/{list,ops}.rs` (2), `state/sync_snapshot.rs` (2), `drift_scanner.rs` (1), `profile/mod.rs` (1, behind `#[test] #[ignore]`).

### Backend AUDIT.md re-verify — what was resolved since 2026-05-11

These open items in `docs/AUDIT.md` were silently RESOLVED in the current code and can be archived:

| Original AUDIT row | Current state |
|---|---|
| `tunnel/mod.rs:117` per-conn `tokio::spawn` outlives `stop()` — CancellationToken needed | ✅ FIXED — `conn_cancel: CancellationToken` per `SshTunnel`, cancelled in `stop()` + Drop (tunnel/mod.rs:55, 184-191) |
| `edit/in_place.rs:short_id:126` 4-byte collision risk | ✅ FIXED — `short_id()` now 16-hex / 8 bytes via `rand::fill` (transport/env.rs:30) |
| `sftp/mod.rs:880 delete_recursive_via` relies on version-dependent `is_symlink()` method | ✅ FIXED — uses `meta.file_type().is_symlink()` + `ft.is_symlink()` + symlink_metadata for lstat (sftp/ops.rs:107-145) |
| `lock_presence.rs:stale-delete:197` silent retry-loop on failed delete | ✅ PARTIAL — now logs `log::warn!` on delete failure (lock_presence.rs:204) but no backoff counter. Improvement, not full fix. |

**Recommendation:** Move first 3 to AUDIT.md Archive (`Codex Fix-Pass 2026-05-16` or similar header). The remaining open items in AUDIT.md still apply against current source — just w/ stale line numbers.

### Hot file structural notes

- `sftp/mod.rs` (302L) — clean session wrapper. Submodules (list/ops/transfer/remote_exec) each are 246-454L. Split was clean.
- `auto_sync.rs` (1954L) — orchestrator. Submodules `auto_sync/{path,watch,flush}.rs` (160/341/610L) handle ingestion, FS event, flush pipeline. Approaching 2000-line agent-split threshold — when it crosses, the natural next split would be the drift-reconcile cluster (`kick_drift_reconcile`, `force_push_now`, `force_pull_now` — large public methods).
- `lib.rs` (1771L, 52 Tauri commands) — at the AGENT-split threshold. HANDOFF queue item (e) calls this out: "`lib.rs` split (1747 L, 52 cmds) — needs per-domain `commands/*.rs` design." Confirmed live concern.
- `assistant/mod.rs` (775L) — clean, w/ S69 dirty work in flight (HANDOFF S69 entry). Probe of `.exe` direct-path before `.cmd` fallback (lines 82-126) is the chronic blank-bubble fix.

## Frontend (src/) — top components + state

### Dead components — 2 confirmed orphans

| File | Status | Notes |
|---|---|---|
| `src/lib/components/assistant/ToolCallCard.svelte` | ❌ ORPHAN | 0 references across `src/`. HANDOFF S69 already flagged "still pending from S68: confirm-delete orphan `assistant/ToolCallCard.svelte`". Confirmed dead since S63. **FLAG for deletion in next ship.** |
| `src/lib/components/shell/PageFooter.svelte` | ❌ UNUSED | Built as one of 4 canonical-skeleton primitives (S67 per CHANGELOG: "PageHeader, PageToolbar, PageFooter, EmptyState"). PageHeader/Toolbar/EmptyState consumed; **PageFooter is implemented but no page imports it yet**. Either prune or queue an issue for a page that needs it (e.g., Assistant cost summary, Sync last-rescan stamp). |

### False alarm — agent missed import path

| File | Status | Why |
|---|---|---|
| `src/lib/components/assistant/EmptyState.svelte` | ✅ ALIVE | Imported by `AssistantPage.svelte:6` as `./EmptyState.svelte` (sibling relative). Frontend recon agent grepped only for path-qualified imports w/ `assistant/EmptyState` literal and missed the sibling-relative form. |

### Dead stores/utils — none

All `src/lib/state/*.ts` files have at least one external importer. `connection.test.ts` is a test file (no importer expected). `diagnostics.svelte.ts` has only `Diagnostics.svelte` as importer — single-consumer but legit (component-scoped store).

### Stale Tailwind classes — none

Grepped HANDOFF "Don't reintroduce" markers (`bg-backlog`, `pill-warn`, `btn-lg`, `op-rail`, `top-bar`, `status-hero`) across `src/`. Zero matches. Cleanup landed cleanly.

### `console.*` usage — all legitimate

64 console calls across 24 files. Verified pattern: all are inside `.catch()` handlers, `try/catch` blocks, or explicit telemetry for diagnostics + auto-reconnect. None are leftover debug. Two `console.debug` calls in `assistant.svelte.ts:726,807` are S69 diagnostic plumbing (intentional per HANDOFF S69 entry).

### AUDIT.md open frontend items — re-verify

All 7 frontend HIGH/MED rows in `docs/AUDIT.md` still apply to current source. Line numbers stale (verified samples):

| AUDIT row | New line |
|---|---|
| `RemotePane.svelte:44-47` async load swallow | line 59 |
| `LocalPane.svelte:44-47` same | line 59 |
| `AppShell.svelte:95` addEventListener leak | line 184 |
| `connection.svelte.ts:249-321` wireEvents no retry surface | partially fixed — there IS a `wireEventsFailed` banner + retry path (AppShell.svelte:132, 173-176) but the underlying `wireEvents()` failure handling per AUDIT might still need work; needs deeper read. |

**Recommendation:** AUDIT.md should get a line-anchor refresh pass. Findings still valid; refs need updating.

## Round 2 tally

- Backend: 0 dead exports, 0 dead deps, 4 AUDIT items now-resolved (candidates for Archive move).
- Frontend: 2 orphans (`ToolCallCard.svelte`, `PageFooter.svelte` — FLAG only, per global "redundancy = FLAG, not delete").
- AUDIT.md line refs stale across all open items.

## Next: Round 3

Configs + scripts + svelte.config / vite.config / tsconfig audit. Then memory cross-ref. Then verification (`cargo check` + `npm run check`).
