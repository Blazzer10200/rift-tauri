# Backend — Concurrency & Resources

_30 confirmed findings._ [← back to index](README.md)

## Backend — Concurrency & Resources

| # | Severity | Title | Location | Fix-gist |
|---|---|---|---|---|
| 1 | HIGH | Permission-request inline await deadlocks stdout pipe | `assistant/mod.rs:2143` | Spawn waiter task; keep drain loop running |
| 2 | MEDIUM | stderr drain exits early — `child.wait()` hangs on full pipe | `assistant/mod.rs:1326` (×3 helpers) | Read-and-discard past cap; never `break` the drain |
| 3 | MEDIUM | `stdout_task` / `stderr_task` leaked on `child.wait()` error | `assistant/mod.rs:2790` | `abort()` both tasks before the early `return` |
| 4 | MEDIUM | Blocking `std::sync::mpsc::recv()` on tokio worker in `stt_start_recording` | `stt/mod.rs:388` | Wrap in `spawn_blocking` or use `tokio::sync::oneshot` |
| 5 | MEDIUM | `rolling_window_loop` JoinHandle dropped — task races with next session | `stt/mod.rs:417` | Store handle; `.abort()` + await before final transcription |
| 6 | MEDIUM | `tool_grep` reads whole file w/ no size cap | `assistant/mcp_server.rs:286` | Stat and skip files above `MAX_READ_BYTES` before `fs::read` |
| 7 | MEDIUM | `run_git` inherits MCP stdin pipe — can corrupt JSON-RPC stream | `assistant/git_local.rs:153` | Add `.stdin(Stdio::null())` |
| 8 | MEDIUM | STT cleanup child orphaned on timeout | `stt/cleanup.rs:86` | `child.kill().await; child.wait().await` before `return` |
| 9 | LOW | Blocking `where.exe`/`which` on cache-miss in `resolve_claude_exe` | `assistant/mod.rs:209` | Pre-resolve at startup in `spawn_blocking` |
| 10 | LOW | `is_file()` stat held under `std::sync::Mutex` on async thread | `assistant/mod.rs:202` | Clone `PathBuf` under lock; drop lock before stat |
| 11 | LOW | Blocking keychain + file I/O in `assistant_auth_probe` | `assistant/mod.rs:937` | Wrap both calls in `spawn_blocking` |
| 12 | LOW | Unguarded read-modify-write on `config.json` across concurrent setters | `assistant/mod.rs:1078` (×8 setters) | Gate all RMW cycles on a `static Mutex<()>` |
| 13 | LOW | Multiple blocking `std::fs` calls in `assistant_send` | `assistant/mod.rs:2193` | Batch into a `spawn_blocking` closure |
| 14 | LOW | Blocking `std::process::Command::status()` in `assistant_stop` | `assistant/mod.rs:2971` | Use `tokio::process::Command` + `.await` |
| 15 | LOW | TOCTOU double-open in `tool_grep` binary probe | `assistant/mcp_server.rs:278` | Reuse probe `File`; seek to 0 for full read |
| 16 | LOW | Blocking `std::sync::Mutex` acquired inside async task (diag bus) | `diagnostics/mod.rs:477` | Use `tokio::sync::Mutex` or `try_lock` with discard |
| 17 | LOW | `spawn_frontend_pump` JoinHandle discarded — no graceful shutdown | `diagnostics/mod.rs:424` | Return/store handle; cancel via `CancellationToken` |
| 18 | LOW | `git diff` output fully buffered before truncation check | `assistant/git_local.rs:243` | Stream + early-exit read; kill child at byte cap |
| 19 | LOW | Blocking `std::fs` writes in `download_update` async loop | `commands/update.rs:194` | Replace with `tokio::fs::File` + `AsyncWriteExt` |
| 20 | LOW | Concurrent `download_update` calls race on same temp file | `commands/update.rs:195` | Guard with `AtomicBool`; write to unique path, rename on complete |
| 21 | LOW | Blocking `std::io::Write` per chunk in model download loop | `stt/model_manager.rs:217` | `tokio::fs::File` + `write_all(...).await` |
| 22 | LOW | Blocking `std::fs` open/stat/rename at `download` fn entry | `stt/model_manager.rs:175` | Replace with `tokio::fs` equivalents |
| 23 | LOW | Two mutexes held simultaneously across resampler CPU work | `stt/audio.rs:201` | Drain leftover into local `Vec` under one lock; release; then acquire resampler lock |
| 24 | LOW | Per-callback `Vec<f32>` allocation on real-time audio thread | `stt/audio.rs:127` | Pre-allocate scratch buffer; reuse via `resize` + copy |
| 25 | LOW | `spawn_blocking` JoinHandle dropped — startup JSONL sweep panics silently | `lib.rs:126` | Spawn an async wrapper; log `Err` from `spawn_blocking(...).await` |
| 26 | LOW | 20 regexes compiled on every `strip_hallucinations` call | `stt/vad.rs:82` | `LazyLock<Vec<Regex>>` static, compiled once |
| 27 | LOW | Mutex poison in `register()` silently closes permission receiver | `assistant/permission.rs:41` | `unwrap_or_else(|e| e.into_inner())` or return `Result` |
| 28 | LOW | `browser_open` blocks tokio worker on main-thread `add_child` round-trip | `commands/browser.rs:21` | `spawn_blocking` around `crate::browser::open()` |
| 29 | LOW | TOCTOU in `browser::open` — concurrent calls can double-create webview | `commands/browser.rs:21` / `browser/mod.rs:29` | Serialize with `Mutex<()>` in Tauri state |
| 30 | LOW | VS Code child handle dropped without wait — zombie on Unix | `commands/mod.rs:36` | `Stdio::null()` on all streams; `std::mem::forget` for intentional detach |

---

### Finding details

**1 — HIGH: Permission-request inline await deadlocks stdout pipe**
`assistant/mod.rs:2143`. The stdout-reader loop is the sole consumer of the child CLI's stdout pipe. When a `can_use_tool` frame arrives, the loop calls `handle_permission_request(...).await` inline, which blocks on a oneshot `rx` for up to 1800 s waiting for the user. The Claude CLI, launched with `--include-partial-messages`, continues streaming content frames to stdout while waiting. Once the ~4–64 KB kernel pipe buffer fills, the child blocks in `write()`; the reader is also blocked; neither can proceed. The turn is silently wedged until the timeout fires. Fix: spawn a separate tokio task to own the 1800 s wait; keep the reader loop draining stdout into a side buffer; hand the `stdin` write-back to the waiter task via an internal channel.

**2 — MEDIUM: stderr drain exits early — `child.wait()` hangs on full pipe**
`assistant/mod.rs:1326` (identical pattern in `enhance_prompt`, `generate_title`, `summarize_session`). Each helper spawns a tokio task to drain stderr but `break`s out of the loop once the buffer exceeds 8 KiB / 32 KiB, dropping the `BufReader` and closing the read end. If the child subsequently writes more stderr than the OS pipe buffer (~64 KB) can hold, it blocks in `write()` and never exits. The parent's `child.wait().await` then hangs indefinitely. Fix: after the cap, read-and-discard all remaining bytes rather than breaking; never drop the read end before the child exits.

**3 — MEDIUM: `stdout_task` / `stderr_task` leaked on `child.wait()` error**
`assistant/mod.rs:2790`. The `Ok(Err(e)) =>` arm returns early before the abort/timeout block at lines 2827–2845. `stdout_task` holds stdin open and keeps reading stdout; `stderr_task` reads stderr. Both continue running until the child's pipe ends close, which may never happen in an error state. Fix: call `stdout_task.abort(); stderr_task.abort();` before the early return; a RAII drop-guard is more robust against future early-return additions.

**4 — MEDIUM: Blocking `mpsc::recv()` on tokio worker in `stt_start_recording`**
`stt/mod.rs:388`. The async command calls `cap_ready_rx.recv()` (a blocking `std::sync::mpsc` receive) directly on the tokio worker thread with no `spawn_blocking` wrapper. If WASAPI device negotiation is slow, the thread parks and starves co-scheduled tasks. Fix: `tokio::task::spawn_blocking(move || cap_ready_rx.recv()).await`.

**5 — MEDIUM: `rolling_window_loop` JoinHandle dropped — races with next session**
`stt/mod.rs:417`. The JoinHandle from `tokio::spawn` is immediately discarded. `shutdown_capture()` cancels the token but never awaits the task. The task may still be inside `spawn_blocking(transcribe)` when `stt_stop_recording` starts draining the same `AudioRing` and calling `engine.transcribe()`. Concurrent inference on the same `WhisperContext` (shared via `Arc`) is not thread-safe in whisper.cpp. A rapid stop+start sequence can spawn a second rolling task before the first has exited. Fix: store the handle in `ActiveSession`; `.abort()` + `.await` it before the final drain.

**6 — MEDIUM: `tool_grep` reads whole file with no size cap**
`assistant/mcp_server.rs:286`. After the 8 KiB NUL binary probe, `std::fs::read(p)` loads the entire file into a `Vec<u8>` with no size guard. `MAX_READ_BYTES` (500 KB) is enforced only in `tool_read_file`. With `MAX_GREP_FILES=5000`, a directory of large non-binary log files exhausts process memory in a single grep call. Fix: stat the file after the probe and skip if size exceeds `MAX_READ_BYTES`; or reuse the probe `File` (seek to 0, read up to cap).

**7 — MEDIUM: `run_git` inherits MCP stdin pipe**
`assistant/git_local.rs:153`. `cmd.output()` is called with no `.stdin(Stdio::null())`. The parent's stdin is the live JSON-RPC pipe from the Claude CLI. Any git operation that reads stdin (e.g. a git hook, a credential helper not fully suppressed by `GIT_ASKPASS`) silently consumes bytes from that pipe, causing protocol desync or a hung MCP server. The env guards (`GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`) reduce the exposure but don't cover all credential paths. Fix: one line — `.stdin(std::process::Stdio::null())`.

**8 — MEDIUM: STT cleanup child orphaned on timeout**
`stt/cleanup.rs:86`. The timeout arm returns `Ok(raw.to_string())` without calling `child.kill()` or `child.wait()`. `kill_on_drop(true)` is not set. On Windows the claude subprocess continues consuming resources and holding pipe handles; on Unix it becomes a zombie. Repeated timeouts accumulate orphaned `claude -p` processes. Fix: `child.kill().await; child.wait().await;` before `return`.

**9 — LOW: Blocking `where.exe`/`which` on cache-miss in `resolve_claude_exe`**
`assistant/mod.rs:209`. `resolve_claude_exe_uncached()` calls `std::process::Command::output()` (blocking) on the slow path. Called from multiple async Tauri commands on cache miss or when the cached path has been moved. Impact is a one-time sub-20 ms stall at first call; not ongoing starvation. Fix: pre-resolve at app startup inside `spawn_blocking`; async callers always hit the cached fast path.

**10 — LOW: `is_file()` stat held under `std::sync::Mutex` on async thread**
`assistant/mod.rs:202`. The `CLAUDE_EXE` global `Mutex` guard is held while `p.is_file()` (a blocking stat syscall) runs. A concurrent async call parks trying to acquire the same mutex. Practical impact is sub-millisecond given local path stats. Fix: clone the `PathBuf` under the lock, drop the lock, then call `is_file()` outside the critical section.

**11 — LOW: Blocking keychain + file I/O in `assistant_auth_probe`**
`assistant/mod.rs:937`. `load_config()` (`std::fs::read_to_string`) and `current_api_key()` (`keyring::Entry::get_password()`) are called without `spawn_blocking` in an async command. The keychain call can block tens of ms on domain-joined/roaming-profile machines. Fix: wrap both in a single `spawn_blocking` closure.

**12 — LOW: Unguarded RMW on `config.json` across concurrent setters**
`assistant/mod.rs:1078` (×8 setter commands). Every setter follows `load_config() → mutate → save_config()` with no mutex guarding the cycle. Tauri 2 dispatches sync commands on a blocking thread pool, making concurrent execution possible. The `tmp+rename` in `save_config()` prevents torn bytes but not logical lost updates. Impact is limited to a stale config value that self-heals on the next save in this single-user app. Fix: a `static Mutex<()>` guarding all RMW cycles.

**13 — LOW: Multiple blocking `std::fs` calls in `assistant_send`**
`assistant/mod.rs:2193`. The async command calls six or more synchronous `std::fs` helpers (`load_config`, `current_api_key`, `load_session_cwd`, `save_session_cwd`, `load_session_model`, `save_session_model`, `write_mcp_config`) before spawning the child. Local SSD makes this sub-millisecond in normal use. Fix: batch into one `spawn_blocking` closure returning all required values.

**14 — LOW: Blocking `std::process::Command::status()` in `assistant_stop`**
`assistant/mod.rs:2971`. `taskkill` (Windows) and `kill` (Unix) are invoked via blocking `std::process::Command::status()` inside an async command. `taskkill` completes in <100 ms; thread pool exhaustion is not realistic at desktop scale. Fix: `tokio::process::Command` + `.status().await` on both platform branches.

**15 — LOW: TOCTOU double-open in `tool_grep` binary probe**
`assistant/mcp_server.rs:278`. The probe `File` is opened at line 278, dropped at line 285, then `std::fs::read(p)` opens the same path again at line 286. Between the two opens the file can be replaced or deleted. The `Err(_) => continue` guard means a deleted file is silently skipped rather than causing a panic; worst case is one stale grep result. Fix: seek the probe `File` back to 0 and read the remainder from the same handle.

**16 — LOW: Blocking `std::sync::Mutex` acquired inside async task (diag bus)**
`diagnostics/mod.rs:477`. The `log::warn!` macro on the `Lagged` branch of the frontend pump routes through `DiagBus::publish()` → `self.recent.lock()` (a `std::sync::Mutex`). The critical section is a trivial `VecDeque` push never held across `.await`, so deadlock is not possible, but it is a Tokio best-practice violation. Fix: use `tokio::sync::Mutex` for `DiagBus::recent`, or `try_lock` with discard on contention.

**17 — LOW: `spawn_frontend_pump` JoinHandle discarded**
`diagnostics/mod.rs:424`. The task is explicitly documented as running for the life of the process; the only exit is `RecvError::Closed` at shutdown, and `app.emit()` already uses `let _ = ...`. No observable leak exists in normal operation. The legitimate gap is the absence of an abort handle for graceful shutdown. Fix: return the `JoinHandle` from `spawn_frontend_pump` and store it in app state, or accept a `CancellationToken`.

**18 — LOW: `git diff` output fully buffered before truncation**
`assistant/git_local.rs:243`. `cmd.output()` collects the entire diff into a `Vec<u8>` before the `MAX_DIFF_BYTES` (64 KB) truncation check at line 243. A pathological repo (large binaries, lock-file churn) can OOM the MCP child. Impact is contained to the MCP subprocess. Fix: use `Child` + incremental read that kills the process at the byte cap.

**19 — LOW: Blocking `std::fs` writes in `download_update`**
`commands/update.rs:194`. `std::fs::create_dir_all`, `File::create`, `write_all` per chunk, and `flush` are all synchronous inside an async command. Per-chunk writes on a local SSD complete in microseconds; no practical starvation at desktop scale. Fix: `tokio::fs::File` + `AsyncWriteExt::write_all(...).await`.

**20 — LOW: Concurrent `download_update` calls race on same temp path**
`commands/update.rs:195`. The temp path is deterministic (`%TEMP%/rift-update/<fname>`). Two concurrent calls both truncate via `File::create` and interleave chunk writes, producing a silently corrupted installer. Trigger requires a double-click or retry race in the single-user UI. Fix: `AtomicBool` in-progress guard; write to a unique temp path, rename atomically on flush success.

**21 — LOW: Blocking `std::io::Write` per chunk in model download loop**
`stt/model_manager.rs:217`. `file.write_all(&bytes)` and `file.flush()` are blocking calls inside the `stream.next().await` loop in `download()`. With multi-threaded Tokio and kernel-buffered I/O, actual starvation is unlikely. Fix: `tokio::fs::File` + `write_all(...).await`; can be done in the same pass as finding 22.

**22 — LOW: Blocking `std::fs` open/stat/rename at `download` fn entry**
`stt/model_manager.rs:175`. `create_dir_all`, `metadata()` (×2), `OpenOptions::open()`, and `rename()` are all synchronous inside `download()`. Models dir is local SSD; syscalls are sub-millisecond in practice. Fix: replace with `tokio::fs` equivalents in the same migration pass as finding 21.

**23 — LOW: Two mutexes held simultaneously across resampler CPU work**
`stt/audio.rs:201`. `leftover` lock acquired at line 201; `resampler` lock acquired at line 206 while `leftover` is still held; both held across the `while buf.len() >= chunk_in` loop including `guard.process(...)`. No current deadlock (ordering is consistent), but future callers inverting the order would deadlock. Fix: drain `buf` into a local `Vec<f32>` under the `leftover` lock only, release it, then acquire the `resampler` lock alone for the process loop.

**24 — LOW: Per-callback `Vec<f32>` allocation on real-time audio thread**
`stt/audio.rs:127`. The I16 and U16 stream callbacks allocate a new `Vec<f32>` (~2–4 KB) on every invocation (~100 Hz) via `.collect()`. The F32 path passes the raw slice directly. Windows allocator handles this without contention in practice; xruns are theoretically possible on constrained platforms. Fix: pre-allocate a conversion scratch buffer alongside `leftover`; reuse via `resize` + in-place copy.

**25 — LOW: `spawn_blocking` JoinHandle dropped — startup JSONL sweep panics silently**
`lib.rs:126`. The best-effort startup sweep explicitly fire-and-forgets; a panic mid-walk leaves some old JSONL files on disk until next startup — no data loss, no corrupt state. The defect is that panics are unobservable. Fix: wrap in `spawn(async { if let Err(e) = spawn_blocking(|| ...).await { log::warn!(...) } })`.

**26 — LOW: 20 regexes compiled on every `strip_hallucinations` call**
`stt/vad.rs:82`. `Regex::new` is called 20 times per transcript segment inside a loop with no caching. Dominant cost is Whisper inference; regex compilation overhead is measurable but minor. Fix: `static HALLUCINATION_RES: LazyLock<Vec<Regex>>` initialised once at startup.

**27 — LOW: Mutex poison in `register()` silently closes permission receiver**
`assistant/permission.rs:41`. On Mutex poison, `tx` is dropped without insertion; the returned `rx` immediately resolves to `Err(RecvError)`, which the caller treats identically to a 1800 s timeout — a synthetic deny with no diagnostic. Mutex poisoning requires a thread panic inside a trivial `HashMap::insert` critical section, making the trigger near-impossible in practice. Same pattern exists in `ask_user.rs:36–39`. Fix: `self.inner.lock().unwrap_or_else(|e| e.into_inner())`.

**28 — LOW: `browser_open` blocks tokio worker on main-thread `add_child` round-trip**
`commands/browser.rs:21`. The blocking path (first open only) posts `add_child` to the main thread and waits; subsequent opens take the `get_webview` fast path with no blocking. One-shot initialization cost; pool exhaustion is not realistic. Fix: `spawn_blocking(|| crate::browser::open(...)).await`.

**29 — LOW: TOCTOU in `browser::open` — concurrent calls can double-create webview**
`commands/browser.rs:21` / `browser/mod.rs:29`. `get_webview(LABEL)` (check) and `add_child(...)` (create) are unsynchronized. A second concurrent call that passes the existence check before the first completes will call `add_child` with the same label; Tauri returns `Err` rather than panicking. Trigger requires concurrent frontend calls to an infrequently-called command in a single-user app. Fix: `Mutex<()>` in Tauri state guarding the check-then-create window.

**30 — LOW: VS Code child handle dropped without wait — zombie on Unix**
`commands/mod.rs:36`. `cmd.spawn().map(|_| ())` discards the `Child` handle. On Unix this leaves a zombie until Rift exits. No `Stdio::null()` configured on any stream, leaving Rift's descriptors open in the child. VS Code self-daemonizes so the zombie is transient. Fix: `.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()` + `std::mem::forget(child)` to signal intentional detach.
