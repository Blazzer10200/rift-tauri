# Backend Audit — rift-tauri v0.1.5-alpha

Generated 2026-05-09. Scope: all `src-tauri/src/` (~6200 LOC, 23 files). Frontend `src/` excluded — UI redesign in progress.

Verification: clippy clean, 47/47 tests passing, zero TODOs in source. This audit catches what passes the compiler but is suspect.

---

## CRITICAL (panic risk / data-loss path)

1. **`src-tauri/src/state/sync_snapshot.rs:49,62,79,88,111`** — `Mutex::lock().unwrap()` on every accessor. Poisoned mutex (any panic-while-holding) cascades into every subsequent caller including the autosync flush loop. Use `.lock().unwrap_or_else(\|e\| e.into_inner())` or surface as `Result`.
2. **`src-tauri/src/state/remote_state.rs:42,47,55,64`** — same Mutex-poison cascade in `RemoteStateCache`.
3. **`src-tauri/src/state/discovery.rs:40,52`** — same in `ResourceDiscoveryCache` (lower blast radius — see #61).
4. **`src-tauri/src/sync/auto_sync.rs:266`** — `mpsc::unbounded_channel` for FS events has no backpressure. Stalled flush + Webpack rebuild = unbounded growth.
5. **`src-tauri/src/sync/auto_sync.rs:535`** — `let _ = std::fs::rename(local_path, &aside)` in `SaveLocalCopy` discards rename error, then downloads remote over the still-present local. Silent loss of user's local copy.
6. **`src-tauri/src/sftp/mod.rs:384-409`** — `Arc::try_unwrap(...).unwrap_or_default()` in `list_recursive_batch` returns empty map on failure. Downstream sees "remote is empty" → could trip mass-delete circuit breaker or false-push regression. Same pattern at line 518, 763, 838.
7. **`src-tauri/src/state/paths.rs:39-43`** — `atomic_write_json` uses `fs::rename` which on Windows can fail if target is open; tmp file stranded, no callback.
8. **`src-tauri/src/sync/auto_sync.rs:634`** — fire-and-forget `tokio::spawn` for lock acquire/release in `queue_path`. No JoinHandle, not cancelled on `stop()`.
9. **`src-tauri/src/sync/auto_sync.rs:791-810`** — fire-and-forget bridge `sync_done` task. Same JoinHandle leak.
10. **`src-tauri/src/sync/auto_sync.rs:814-822`** — fire-and-forget edit-trail append. Same.

---

## HIGH (correctness / hygiene that bites later)

11. **`src-tauri/src/sftp/mod.rs:276-279`** — `close()` doesn't await all worker shutdowns under error.
12. **`src-tauri/src/sftp/mod.rs:339-344`** — worker pool serializes job acquisition through one shared Mutex-wrapped mpsc receiver. Contention point at high job count.
13. **`src-tauri/src/sync/auto_sync.rs:386,983-989`** — explicit `drop(g)` before `fire_status()` is correct but fragile; tokio Mutex held across `.await` is a refactor away from latent bug.
14. **`src-tauri/src/sync/auto_sync.rs:607-626`** — DashMap `entry().and_modify().or_insert()` in hot callback holds shard lock through closure that allocates non-trivially.
15. **`src-tauri/src/sftp/mod.rs:858-868`** — `get_remote_sha1` discards stderr (`ChannelMsg::ExtendedData`), no debug log on sha1sum failure.
16. **`src-tauri/src/sftp/mod.rs:616-637`** — `get_remote_folder_size` opens exec channel on the SFTP-loaded main session; HOL-blocking on TCP layer.
17. **`src-tauri/src/sync/lock_presence.rs:233-245`** — temp dir keyed by `timestamp_nanos`; collision risk under concurrent `poll_once`. Use `short_id()` from `edit/in_place.rs`.
18. **`src-tauri/src/sync/edit_trail.rs:105-113`** — same nanosecond-only temp dir uniqueness.
19. **`src-tauri/src/lib.rs:201-208`** — `start_autosync` cleanup flow with partial-move tunnel_handle; works today by Rust enforcement but fragile. Wrap in scopeguard / RAII.
20. **`src-tauri/src/sync/auto_sync.rs:1047-1057`** — `stat_local` returns `(0, Utc::now())` on metadata error, then writes that as snapshot baseline. Permanently wrong entry.
21. **`src-tauri/src/sftp/mod.rs:995-1001`** — `rename_via` blindly `let _ = sftp.remove_file(to)` before rename creates TOCTOU window.
22. **`src-tauri/src/sync/auto_sync.rs:709-711`** — `safe_count_files` walkdir runs synchronously on tokio thread. Stalls flush cycle on big folders.
23. **`src-tauri/src/sync/drift_scanner.rs:133-134`** — `walk_local` synchronous read_dir on tokio thread.
24. **`src-tauri/src/sync/auto_sync.rs:1062`** — `wait_for_readable` uses `std::fs::File::open` blocking on tokio thread.

---

## MEDIUM (improvements w/ moderate payoff)

25. `sftp/mod.rs:1025` — `upload_atomic_via` reads whole file into Vec<u8> (64MB worst case).
26. `sftp/mod.rs:1052` — `download_atomic_via` same whole-file-in-memory.
27. `sync/auto_sync.rs:42-48` — debounce/ceiling/concurrency are compile-time constants; should be per-server config.
28. `sync/auto_sync.rs:166-188` — `classify_action` mixes case-sensitive and lowercased checks; "blocked" lowercase falls through to Error.
29. `sync/auto_sync.rs:575` — `path.to_string_lossy().to_string()` allocates per FS event for nothing.
30. `sync/auto_sync.rs:605` — `chrono::Duration::milliseconds` constructed in hot loop; should be const-extracted.
31. `sftp/mod.rs:256-265` — `ensure_workers` uses `join_all` (no short-circuit on first failure).
32. `sftp/mod.rs:383-409` — belt-and-braces retry fires for legitimately empty folders (N round-trips per scan).
33. `sync/auto_sync.rs:980` — file failing 3 times is silently dropped from retry queue with no log/UI surface.
34. `sftp/mod.rs:126-131` — private key re-loaded from disk on every `ensure_workers` call (3x per pool init).
35. **`state/sync_snapshot.rs:36`** — `pub const SHA1_MAX_BYTES: i64 = 5 * 1024 * 1024` is dead (never referenced outside definition) and its comment is wrong (claims it matches DriftScanner — DriftScanner uses 64MB). Two other modules redefine 64MB locally. Consolidate to one constant.
36. **`profile/mod.rs:72`** — `RiftConfig::save()` uses plain `fs::write`, NOT atomic. Crash mid-write corrupts `~/.rift/rift.json` (sole source of server profiles). Use `atomic_write_json`.
37. `sync/auto_sync.rs:319-326` — `stop()` awaits flush_task but aborts event_task; document why abort is safe here.
38. `sftp/mod.rs:447,463` — `discover_manifest_folders` early-return invariant (root must be resources parent, not a resource itself) not documented or validated.
39. `lib.rs:562-570` — `bootstrap_list_files` byte-slices original-case path using lowercased prefix length; misalignment risk on multi-byte UTF-8.
40. `sync/lock_presence.rs:225-229` — `active_by_path.clear()` + re-insert is not atomic; concurrent reads see empty map.
41. `lib.rs:458-477` — `open_sftp_for` opens fresh SSH+SFTP per browser op (300-500ms per click). Persistent browser-session pool would help.
42. `edit/in_place.rs:150-199` — sync notify callback calls `tokio::spawn`; can panic on shutdown if runtime gone. Use `Handle::try_current()`.
43. `sync/auto_sync.rs:267-272` — `let _ = tx.send(ev)` is fine for unbounded but becomes silent-drop point if #4 is bounded.
44. `sync/ignore.rs:125-138` — needle is lowercased per-iteration inside loop.
45. `bridge/mod.rs:54-59` — `text().await.unwrap_or_default()` benign but worth noting on redirect responses.
46. `state/sync_snapshot.rs:110-115` — `save()` is last-write-wins under concurrent `set()`; document.
47. `sync/auto_sync.rs:1044-1057` — `m.len() as i64` truncates on >9.2EB files (theoretical only).
48. `sftp/mod.rs:1092-1094` — SFTP u32 mtime wraps in 2106 (protocol limitation).

---

## LOW (nitpicks, modernization, polish)

49. `tunnel/mod.rs:51-78` ↔ `sftp/mod.rs:96-120` — `Handler` struct + fingerprint check duplicated verbatim.
50. `sftp/mod.rs:87` — `pubkey_fingerprint` not callable cross-module; tunnel re-implements inline.
51. `local_fs.rs:26` ↔ `lib.rs:425` — `local_fs::list_directory` and `lib::local_list_dir` are duplicate impls.
52. `lib.rs:414-455` — `LocalEntry` in `lib.rs` duplicates `local_fs::LocalEntry` (different field names, same shape).
53. `edit/in_place.rs:335-343` — `_unused_export_warning_silencer` test exists only to suppress dead-code warning.
54. `sync/auto_sync.rs:207` — `state: Mutex<(AutoSyncState, String)>` should be a named struct.
55. `sftp/mod.rs:295-296` — `owned_filter: Option<Vec<String>>` allocates just to extend lifetime.
56. `lib.rs:39-40` — sync vs async commands inconsistent (probably fine, no rule).
57. `profile/mod.rs:82-103` — `slugify` uses Unicode-aware lowercase; FiveM names are ASCII.
58. `sync/auto_sync.rs:431-438` — `sort_by_key` uses `OsStr::len()` (WTF-16 code units on Win); use `.components().count()`.
59. `sync/ignore.rs:59` — `path.replace('\\', "/")` allocates always; `Cow<str>` would skip on POSIX.
60. `sync/edit_trail.rs:117-123` — `trim_to_tail` may drop trailing newline inconsistently.
61. **`state/discovery.rs`** — `ResourceDiscoveryCache` is fully implemented + tested but wired to ZERO commands. Either wire it (faster reconnect) or `#[allow(dead_code)]` w/ phase-N TODO.
62. `local_fs.rs:69-87` — `get_parent`, `human_size`, `safe_dir_name` defined but unused.
63. `update_service.rs:42` — `current_version()` defined but never exposed via invoke_handler.
64. `transport/ssh_keygen.rs:109-113` — fourth independent `home_dir()` impl. Consolidate.
65. `sync/lock_presence.rs:62-74` ↔ `edit_trail.rs:125-133` — `hostname()` shells out to a process; duplicated.
66. `sync/auto_sync.rs:354,365` — `MAIN_SEPARATOR` probe path coupling fragile.
67. `sftp/mod.rs:1104-1116` — `shell_quote` doesn't handle NUL/newline; document limitation.
68. `lib.rs:677-712` — `run()` and managed-state types have no rustdoc.
69. `sftp/mod.rs:1-32` — module headers use `//` not `//!`; not picked up by `cargo doc`.
70. `sync/auto_sync.rs:44` — 700ms+150ms = 850ms worst-case flush latency; document as known UX trade.
71. `bootstrap/mod.rs:40-56` — `BAD_REMOTE_ROOT` ratio threshold calibrated for Qbox-style; stock FXServer may false-positive.
72. `sftp/mod.rs:273-283` — `close()` may run while fire-and-forget tasks still hold `Arc<Worker>` clones (see #8).
73. `lib.rs:619-621` — `editor_for` TOCTOU: concurrent `begin_edit_in_place` for same server opens 2 SFTP connections, leaks one.
74. **`Cargo.toml:16`** — `tokio = features = ["full"]` pulls everything (process, signal, tracing, …); narrow to actually-used.
75. **`Cargo.toml:43`** — `anyhow = "1"` and `thiserror = "2"` declared, ZERO usages in source. Remove.
76. **`Cargo.toml:49`** — `notify-debouncer-full = "0.7.0"` declared, ZERO usages (custom debounce in autosync). Remove.
77. **`Cargo.toml:41`** — `async-trait = "0.1"` declared; only needed transitively via russh. Verify + remove from direct deps.
78. `update_service.rs:50-64` — `check_for_updates` is sync, calls Velopack which does network I/O. May or may not be on a Tauri thread pool.
79. `sync/auto_sync.rs:888-890` — `Entry { ... }` re-construction from already-cloned `snap`.
80. `state/sync_snapshot.rs:125-131` — `hex_upper` uses `format!("{:02X}", b)` per byte (20 micro-allocs per SHA1).
81. `sync/drift_scanner.rs:294` — magic `2` for mtime tolerance; should reference `MTIME_TOLERANCE_SECS` constant.
82. `sftp/mod.rs:596-614` — `get_remote_folder_size` shell command builds w/ `shell_quote`; defensive const-assert that IGNORE_SEGMENTS contains no `'`.
83. `lib.rs:264` — `enqueue_for_flush_batch` no input validation; relies on watch-root ownership as security boundary (undocumented).
84. `lib.rs:277` — `resolve_conflict` no input validation; path traversal via `local_path` could escape watch root.
85. `sync/auto_sync.rs:1027-1028` — `rel_s.starts_with("../")` check is misleading (impossible after `strip_prefix`).

---

## Notes for the human

- **SHA1_MAX_BYTES (#35):** the 5MB constant in sync_snapshot.rs is verifiably dead code w/ a wrong comment. Need intentional decision on which ceiling is correct.
- **ResourceDiscoveryCache wiring (#61):** fully implemented + tested + never used. Either wire for fast-reconnect or mark dead code.
- **Worker pool liveness after close() (#8/#72):** fire-and-forget tasks may outlive `SftpClient::close()`; russh shutdown semantics determine visible impact.
- **EditInPlaceManager TOCTOU (#73):** low-probability connection leak under rapid clicks. Backend per-server init lock or UI-side guarantee needed.
- **Bridge token plaintext (`profile/mod.rs:33`):** WPF DPAPI-encrypted; Tauri stores plaintext. Intentional or gap?
- **No SSH/SFTP integration tests:** all tests are pure-logic units. Dockerized OpenSSH fixture in CI would catch worker-pool empty-listing class bugs.
