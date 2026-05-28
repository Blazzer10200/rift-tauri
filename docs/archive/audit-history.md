# Audit history — 2026-05-20 Wave 1/2/3 shipped findings

> Verification trail for audit items now closed. Moved out of `docs/ISSUES.md`
> on 2026-05-26 to keep the live tracker scoped to open work. Full audit reports
> (per-agent shards + synthesis docs) still live at
> `state/audit-2026-05-20/{A..AA}-*.md` + `SYNTHESIS-wave[1-3].md`.
>
> Format: each block carries the original `## NN. ~~Title~~` strikethrough header
> + the verification citation. Re-grep file:line refs before quoting — many have
> drifted since the original audit (especially in `auto_sync.rs` and
> `assistant.svelte.ts` which have grown 20-50% since 2026-05-20).

---

## Closed during tauri-updater migration (2026-05-26)

### ~~16.~~ — Stale; `update_service.rs` + custom `GithubSource` deleted in tauri-updater migration; see `docs/design/updater-migration.md`.
### ~~19.~~ — Was already RESOLVED v0.4.12; now doubly stale — `update_service.rs` deleted, new `commands/update.rs::apply_pending_update` explicitly stops engine + tunnel before `update.install(bytes)`; verified by runtime recon.
### ~~25.~~ — Stale; velopack-rust dropped from `Cargo.toml` in tauri-updater migration.
### ~~28.~~ — Stale; `ureq` dropped from `Cargo.toml` in tauri-updater migration; single reqwest stack remains.
### ~~33.~~ — SHIPPED 2026-05-26. `commands/sftp.rs::local_list_dir` now takes `server_key` + validates via new `path_guard::validate_local_listable` (root-allowing variant of `validate_local_child`). FE caller `LocalPane.svelte::load()` passes `connection.selectedKey`. CDP-verified: 11 local items render against the active profile's local_root.
### ~~252.~~ — Stale post tauri-updater migration; `update_service.rs` deleted; new path uses `tauri-plugin-updater` polling `latest.json`.

---

## Wave 1 (backend deep audit) — SHIPPED blocks

### ~~96. `apply_selected` bypasses buffered feed~~ — VERIFIED SHIPPED
`auto_sync.rs:1547-1550` routes the WARN row through `engine.log_activity(...)` w/ `#96` cited inline.

### ~~99. `flush_batch dispatched` count includes Requeued~~ — VERIFIED SHIPPED
`flush_batch` now returns `(dispatched, ok, fail)` (`auto_sync/flush.rs:141-143`); `force_push_now` cache-clear gates on `ok > 0` (`auto_sync.rs:901`).

### ~~107. `start_autosync` status sampled before prev engine fully stopped~~ — VERIFIED SHIPPED
`commands/sync.rs:473-476` — `engine.status()` now sampled AFTER `prev.stop().await` + slot replacement, w/ `#107` cited inline.

### ~~109. `bootstrap_list_files` accepts dead `_local_root` IPC param~~ — SHIPPED 2026-05-25
Backend signature already pruned (cmd now at `commands/sftp.rs:590` post-split, only `server_key`). FE caller in `Bootstrap.svelte:94` updated to drop the `localRoot` IPC field.

### ~~115. `session-lost` re-broadcasts full prompt~~ — VERIFIED SHIPPED
`assistant/mod.rs:2241-2248` — emits only `{ session_id }`; `#115` cited inline.

### ~~135. `force_push_now` promotion log out-of-order~~ — VERIFIED SHIPPED
Post-refactor ordering: promotion `log::debug!` lands at `auto_sync.rs:842`, `flush_all_now` runs at L890. Correct order.

---

## Wave 2 (frontend deep audit) — SHIPPED blocks

### ~~146. `mutateStreaming` rebuilds full messages array~~ — VERIFIED SHIPPED
`assistant.svelte.ts:743-753` — caches `streamingMsgIdx`; direct index write when index matches, full `.map` only as fallback. `#146/#234` cited inline. Also closes #234.

### ~~147. `ensureThinkingFromEnvelope` reference-equality on `$state` proxies~~ — VERIFIED SHIPPED
`assistant.svelte.ts:846-852` — match by `startedAt` stable key; `#147` cited inline.

### ~~148. `handleTurnComplete` microtask races tab switch~~ — VERIFIED SHIPPED
`assistant.svelte.ts:2907-2917` — captures `capturedConvoId`; re-queues onto original tab if convo changed before microtask fires. `#148` cited inline.

### ~~149. `openTab` race against `deleteConversation`~~ — VERIFIED SHIPPED
`assistant.svelte.ts:2322-2327` — after `refreshConversations`, explicitly closes any tab pointing at the deleted id. `#149` cited inline.

---

## Wave 3 (cross-cutting) — SHIPPED blocks

### Wave 1 #42 cross-verification — NOT A REAL BUG
Agent T verified via `auto_sync/watch.rs:245` + `sync/ignore.rs:91-97,157`. `classify()` extracts basename via `rsplit('/').next()` before `.rift-conflict.` substring check. Absolute vs relative path makes no difference for the conflict-copy marker rule. **#42 is closed/INFO.** Frontend safety chip from #158 is still useful but not security-critical.

### ~~222. `stderr_task.await.unwrap_or_default()` drops JoinError~~ — VERIFIED SHIPPED
`assistant/mod.rs:1339-1342, 2204+` — both sites now `log::error!` + surface `(stderr drain task panicked: {e})`.

### ~~223. `create_dir_all` for download staging silently ignored~~ — VERIFIED SHIPPED
All 3 sites (now `commands/sftp.rs:191, 211, 219` post-split) wrap in `if let Err(e)` + `log::warn!("download mkdir ...")`.

### ~~224. `try_read_lock` `.ok()?` conflates absent-lock vs SFTP-error~~ — VERIFIED SHIPPED
`lock_presence.rs:48-54` — `LockReadOutcome` enum (`Present`/`Absent`/`Error`) distinguishes the three. Doc comment cites `#224`.

### ~~225. `eprintln!` in sync handlers + drift scanner~~ — SHIPPED 2026-05-25
14 `eprintln!` in `sync/auto_sync.rs` (force_push_now / force_pull_now / reconcile) → `log::debug!` (most) + `log::info!` (reconcile summary) + `log::warn!` (no-watched-folders). Drift-scanner duplicate `eprintln!` deleted (kept the `emit_with_fields` follower). Verify: `Grep eprintln! src-tauri/src/sync/` returns zero. Remaining eprintlns audited 2026-05-25: `sftp/list.rs:389`, `sftp/ops.rs:88` already cleaned. `profile/mod.rs:248` + `state/sync_snapshot.rs:370,377` are inside `#[test] #[ignore]` blocks — test diagnostic output, not production code paths. Leave as-is.

### ~~226. Broadcast bus lag silently counted~~ — VERIFIED SHIPPED
`diagnostics/mod.rs:481` — `log::warn!("diag bus lagged: {n} events dropped")` lands after `record_bus_lag(n)`.

### ~~228. `dialog:default`~~ — closed 2026-05-25 (plugin is genuinely required)
Audit: 4 unique callers (ProfileSetup, ServerAdd, SSHKeySetup, assistant.svelte.ts) all use `openDialog` for OS-native file/folder pickers. Cannot be replaced w/ Svelte dialogs — needs OS-level chooser. Plugin stays.

### ~~229. `opener:default` too broad~~ — VERIFIED SHIPPED 2026-05-25
`capabilities/default.json` already lists the 3 explicit `opener:allow-*` perms; no `opener:default`. Closes #31 + #229.

### ~~230. `core:default` bundles unused~~ — VERIFIED SHIPPED 2026-05-25
`capabilities/default.json` pinned to `core:event:default` + `core:path:default` + `core:webview:default` + `core:window:default` + 4 explicit `core:window:allow-*`. `core:app`/`core:menu`/`core:resources` excluded. `core:path:default` retained — `Settings.svelte` uses `appConfigDir`/`appLogDir`. Closes #30 + #230.

### ~~234. Re-cite of #146~~ — VERIFIED SHIPPED (see #146).

### ~~240. `aborted_shrunk()` mutex-poison silently returns empty vec~~ — VERIFIED SHIPPED
`auto_sync.rs:1328-1334` — explicit `Err(p)` arm logs + recovers via `p.into_inner().clone()`.

### ~~241. MCP bridge socket timeouts swallowed~~ — VERIFIED SHIPPED
`mcp_server.rs:35-40` — `set_read_timeout`/`set_write_timeout` failures now `log::warn!` with label.

### ~~242. MCP bridge `stream.flush().ok()` drops errors~~ — VERIFIED SHIPPED
4 bridge flush sites (`mcp_server.rs:407, 590, 676, 774`) all now `.map_err(|e| format!("bridge flush: {e}"))?`.

### ~~243. STT `from_slice().unwrap_or_default()` accepts corrupt config~~ — VERIFIED SHIPPED
`stt/mod.rs:138-141` — parse failure now logs `stt-config parse failed ({e}), using defaults` before defaulting.

### ~~244. `edit_trail.rs` destroys trail on SFTP error~~ — VERIFIED SHIPPED
`edit_trail.rs:56-67` — `ReadOutcome` enum (`Present`/`Absent`/`Error`); error arm logs + early-returns to preserve remote history.

### ~~250. STT console.debug calls~~ — VERIFIED SHIPPED 2026-05-25
Re-grep of `src/lib/state/stt.svelte.ts` returns zero `console.*` matches. Both #22 and #250 closed.

---

## Wave 1 #81 — PARTIAL (kept in live tracker as historical-context only)

### 81. `SyncSnapshot::set`/`forget` silently discard save errors
- **Where:** `state/sync_snapshot.rs:74,80`
- **Symptom:** Both methods originally `let _ = self.save_locked(&g)` — disk-write fail silently leaves in-memory state diverged from on-disk. Next restart loads stale data → phantom drift / false ToDelete/ToPull. `replace_under` at L125 correctly propagates.
- **Status:** PARTIAL v0.4.16-alpha S119 — `set` + `forget` now match the save_locked Result; failures emit `log::error!` with remote_path context. Signatures kept `-> ()` to avoid touching every caller (hot path: flush). Full `Result<(), io::Error>` propagation + DiagBus surface deferred — log is enough to diagnose; the silent-divergence case is closed.

---

## AA — top-10 clippy perf lints (Wave 3 triage table)

> Moved out of live tracker 2026-05-26. Snapshot from 2026-05-20 audit; file:line refs likely drifted. Re-grep before acting.

| # | Rule | Count | Representative | Fix |
|---|------|-------|----------------|-----|
| 1 | `format_push_string` | 39 | `mcp_server.rs:185` | `write!(out, ...)` (see #258) |
| 2 | `redundant_closure_for_method_calls` | ~12 | `mcp_server.rs:205` | Method-ref instead of closure |
| 3 | `map_unwrap_or` | 3 | `mcp_server.rs:174` | `map_or(default, f)` |
| 4 | `needless_pass_by_value` | 3 | `assistant/mod.rs:447` | `&str` over `String` |
| 5 | `uninlined_format_args` | ~8 | `mcp_server.rs:188` | Inline vars in format str |
| 6 | `unnecessary_sort_by` | 1 | `assistant/mod.rs:442` | `sort_by_key(\|b\| Reverse(...))` |
| 7 | `map_unwrap_or` (Option) | 1 | `assistant/mod.rs:428` | `map_or` |
| 8 | `redundant_else` | 1 | `drift_scanner.rs:245` | Flatten else |
| 9 | `manual_let_else` | ~8 | `mcp_server.rs:164` | `let Ok(x) = ... else { ... }` |
| 10 | `match_same_arms` | 1 | `mcp_server.rs:642` | Merge arms |
