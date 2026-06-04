# Backend — Correctness

_41 confirmed findings._ [← back to index](README.md)

## Backend — Correctness

### Severity-Sorted Finding Table

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| medium | Stdout drain task panic silently replaced with zero defaults | `assistant/mod.rs:1789` | Mirror stderr pattern: `unwrap_or_else(\|e\| { log::error!(...); Default::default() })` |
| medium | UTF-8 line truncation at byte offset 200 can panic on multi-byte codepoints | `assistant/mcp_server.rs:302` | Walk back to char boundary: `while !line.is_char_boundary(idx) { idx -= 1; }` or `line.chars().take(200).collect()` |
| medium | Absolute paths with `..` bypass traversal guard | `assistant/git_local.rs:72` | Canonicalize or normalize `..` before `starts_with(root)` check |
| medium | Auto-detect language path silently clobbered in Whisper | `stt/whisper.rs:65` | Pass `language` directly; remove `.or(Some("en"))` |
| low | Negative CLI resolution cached forever — re-install not detected | `assistant/mod.rs:203` | Change `None => return None` to `None => {}` so not-found falls through to re-resolve |
| low | Sidecar cleanup swallows `remove_file` errors — leaks `.cwd`/`.model` files | `assistant/mod.rs:492, 530` | Add `log::warn!` on removal failure, matching save counterparts |
| low | Silent swallow of chmod failure on MCP config file | `assistant/mod.rs:776` | `if let Err(e) = set_permissions(...) { log::warn!(...) }` |
| low | Dead `"full"` trust-level variant tied to stripped RCON feature | `assistant/mod.rs:1140` | Remove `"full"` from `is_valid_trust_level` matches arms |
| low | I/O error on stdout silently terminates loop — surfaces as empty output | `assistant/mod.rs:1337, 1475` | `while let Some(line) = lines.next_line().await.map_err(...)? { ... }` |
| low | SAFE_MCP allowlist contains four stripped tools (dead post-conversion refs) | `assistant/mod.rs:2425` | Remove `sync_status`, `drift_snapshot`, `reconcile_preview`, `ask_user` from constant |
| low | Unreachable `None` arm in status match emits false error event | `assistant/mod.rs:2856` | Replace with `.expect("invariant violated")` to document and panic loudly |
| low | `FinalPayload.cleaned` flag wrong when scrubbed output is empty | `stt/mod.rs:500` | Track `did_clean: bool` through the conditional; emit that instead of `cleanup_enabled` |
| low | `load_config` silently swallows all I/O errors | `stt/mod.rs:139` | Match on `error.kind()` — log warn for unexpected variants, silently default only for `NotFound` |
| low | `tool_grep` off-by-one lets one extra file scan past `MAX_GREP_FILES` | `assistant/mcp_server.rs:291` | Change `>` to `>=` or move increment before the guard |
| low | Progress-event emit error silently dropped in diagnostics pump | `diagnostics/mod.rs:474` | `if let Err(e) = app.emit(...) { log::warn!(...) }` |
| low | Dead `DiagStage` variants from stripped SFTP/sync/bridge pipeline | `diagnostics/mod.rs:41` | Remove ~22 dead variants; trim module doc comment; shrink `is_critical` match |
| low | Dead unreachable early-return in `tool_git_status` masks clean-tree logic | `assistant/git_local.rs:194` | Remove `lines.is_empty()` guard — clean-tree path already correct at line 210 |
| low | Pre-release label comparison is lexicographic, not semver-ordered | `commands/update.rs:248` | Parse trailing numeric suffixes or use `semver` crate; at minimum document the constraint |
| low | Progress-event emit error silently dropped in update download | `commands/update.rs:206` | `if let Err(e) = app.emit(...) { log::warn!(...) }` |
| low | SHA-256 hasher silently skipped on resume — corrupt partial not integrity-checked | `stt/model_manager.rs:198` | Re-hash already-written bytes before resuming, or full-file re-hash after rename |
| low | `emit_progress` swallows Tauri emit errors via `let _ =` | `stt/model_manager.rs:267` | `if let Err(e) = app.emit(...) { log::warn!(...) }` |
| low | U16-to-f32 normalization uses wrong divisor (32767 instead of 32768) | `stt/audio.rs:146` | `(*s as f32 - 32768.0) / 32768.0` |
| low | Resampler error logged at debug level — audio silently dropped | `stt/audio.rs:215` | Upgrade to `log::warn!`; consider exposing error counter to orchestrator |
| low | Dead no-op constructor in stub `WhisperEngine` kept alive to silence lint | `stt/whisper.rs:139` | Delete `_new_unused()`; apply `#[allow(dead_code)]` to fields directly if needed |
| low | `show()` / `set_focus()` results silently discarded — window may start invisible | `lib.rs:116` | Log or propagate via `?` — setup closure already returns `Result` |
| low | `set_position` result silently discarded after Win32 work-area calculation | `lib.rs:91` | `if let Err(e) = window.set_position(...) { log::warn!(...) }` |
| low | Silent fallback on missing filename produces wrong tmp path | `state/paths.rs:82` | Replace `.unwrap_or("snapshot")` with `.ok_or_else(\|\| io::Error::new(InvalidInput, ...))?` |
| low | Stale comment references removed `SyncSnapshot` module | `state/paths.rs:69` | Trim comment to describe only the current atomic-write concern |
| low | `stdin` write errors silently swallowed — child receives no input | `stt/cleanup.rs:76` | Log `write_all`/`shutdown` errors at warn level before falling back to raw transcript |
| low | Timeout does not kill child process — leaks stalled subprocess | `stt/cleanup.rs:86` | Add `let _ = child.kill().await;` before `return Ok(raw.to_string())` in timeout arm |
| low | VAD errors silently treated as 'no speech' via `.unwrap_or(false)` | `stt/vad.rs:40` | Log at warn level before defaulting; consider propagating via `?` |
| low | No-op match arm on VAD constructor obscures infallibility assumption | `stt/vad.rs:25` | Replace identity match with direct binding: `let mut vad = Vad::new_with_rate_and_mode(...)` |
| low | `set_position`/`set_size`/`show` errors silently discarded in browser `open()` reopen path | `browser/mod.rs:31` | Propagate via `.map_err(...)? ` matching the create path |
| low | `set_bounds()` swallows `set_position`/`set_size` errors — `Result<(),String>` is decorative | `browser/mod.rs:64` | Propagate: `wv.set_position(...).map_err(\|e\| format!(...))?` |
| low | `register()` silently drops sender on mutex poison — receiver hangs | `assistant/permission.rs:41` | Use `.expect("PermissionRegistry mutex poisoned")` for fail-loud on an impossible-in-practice path |
| low | `register()` silently discards sender on poisoned mutex in `ask_user` | `assistant/ask_user.rs:36` | Same: `.expect(...)` or `unwrap_or_else(\|e\| e.into_inner())` with logging |
| low | Stale doc-comment references removed `remote_bridge` module | `assistant/ask_user.rs:8` | Update step 3 of flow comment to name the real current dispatcher |
| low | Dead `bridge_token_key` and `rcon_password_key` in `secrets.rs` | `secrets.rs:53–61` | Delete both functions; update module doc comment to reflect api-key-only purpose |
| low | `set()` accepts empty-string values that `get()` silently discards | `secrets.rs:34` | Early guard: `if value.is_empty() { return delete(key); }` |
| low | `spawn()` silently succeeds when VS Code is not installed | `commands/mod.rs:36` | Await child exit status via `.wait()` and return `Err` on non-zero |
| low | Stale module-level doc references stripped systems (`SyncSnapshot`, `EditTrail`, `SftpClient`) | `state/mod.rs:3` | Replace Phase 1a/1b migration block with a one-line current-reality comment |

---

### Per-Finding Details

**[medium] Stdout drain task panic silently replaced with zero defaults** — `assistant/mod.rs:1789`
`stdout_task.await.unwrap_or_default()` discards a `JoinError` (task panic) entirely, substituting empty string and zero tokens. The accumulator lands at the `is_empty()` guard and surfaces as the generic "summarize call returned empty text" error — the real panic cause is lost. The `stderr` task at line 1791 correctly logs its `JoinError` via `unwrap_or_else`. Fix: mirror the stderr pattern on stdout, or map the `JoinError` to `Err` so callers see the actual failure.

**[medium] UTF-8 line truncation at byte offset 200 can panic on multi-byte codepoints** — `assistant/mcp_server.rs:302`
`&line[..200]` is a raw byte-offset slice on a valid UTF-8 `&str`. Any 2–4 byte codepoint that starts at byte 199 or straddles byte 200 triggers a Rust panic ("byte index 200 is not a char boundary"), crashing the MCP server subprocess. The `line.len() > 200` check gates entry to the branch but does not align to a char boundary. Fix: walk back to a valid boundary (`while !line.is_char_boundary(idx) { idx -= 1; }`) or use `line.chars().take(200).collect::<String>()`.

**[medium] Absolute paths with `..` components bypass traversal guard** — `assistant/git_local.rs:72`
The `ParentDir` component check lives inside the `else if` branch that only executes for relative paths. Absolute paths go directly to `starts_with(root)`, which is a lexical prefix comparison that does not normalize `..`. An input like `/workspace/../workspace/../../etc/passwd` satisfies the prefix test and is accepted. Fix: canonicalize `p` (or perform a pure `..`-collapsing normalization pass) before the `starts_with` check so path-traversal via an absolute input with `..` segments is rejected.

**[medium] Auto-detect language path silently clobbered in Whisper** — `stt/whisper.rs:65`
The documented API contract is `language: None` → whisper.cpp auto-detect. `.or(Some("en"))` unconditionally substitutes `"en"` for `None`, making auto-detect permanently unreachable. Non-English audio passed with `None` is silently forced to English. Fix: `params.set_language(language)` — if a hard English default is intentional, remove the `None`-means-auto-detect doc comment and update the signature accordingly.

**[low] Negative CLI resolution cached forever** — `assistant/mod.rs:203`
`None => return None` in the `CLAUDE_EXE` fast-path permanently caches a "CLI not found" result for the process lifetime. Unlike the stale-path arm (which falls through to re-resolve), the `None` arm never retries. A user who installs the Claude CLI after Rift starts will get "CLI not found" on every spawn attempt until full app restart. Fix: change `None => return None` to `None => {}` so both stale-path and not-found cases fall through to re-resolution.

**[low] Sidecar cleanup swallows `remove_file` errors** — `assistant/mod.rs:492, 530`
`delete_session_cwd` and `delete_session_model` both use `let _ = std::fs::remove_file(&p)` with no logging, while their save counterparts emit `log::warn!` on failure. A leaked `.cwd` sidecar would route a re-created same-UUID session to a stale workspace path. Fix: add `log::warn!` on removal failure in both delete functions, consistent with save behavior.

**[low] Silent swallow of chmod failure on MCP config file** — `assistant/mod.rs:776`
`let _ = std::fs::set_permissions(...)` discards a chmod error silently. The MCP config file holds a session-scoped bridge token; if the chmod fails the file is left world-readable for the session lifetime with no log entry. The token is ephemeral, limiting real exposure, but the fail-loud rule still applies. Fix: `if let Err(e) = set_permissions(...) { log::warn!(...) }`.

**[low] Dead `"full"` trust-level variant tied to stripped RCON feature** — `assistant/mod.rs:1140`
`is_valid_trust_level` accepts `"full"` with a doc comment citing "RCON raw passthrough (phase 2)" — a stripped feature. In `mcp_server.rs`, `"full"` maps to trust rank 2 but `trust_at_least` is only ever called with `"standard"`, so rank 2 grants identical access to rank 1. The variant is inert but can be written to config and injected via `RIFT_TRUST_LEVEL`, confusing the permission model. Fix: remove `"full"` from the `matches!` arms and update the doc comment.

**[low] I/O error on stdout silently terminates loop** — `assistant/mod.rs:1337, 1475`
`while let Ok(Some(line)) = lines.next_line().await` treats `Err(_)` identically to `Ok(None)` (end of stream). A mid-stream pipe I/O error silently breaks the loop; the partial or empty accumulator surfaces as "enhancer returned empty output" / "title generation returned empty output" with the real I/O error discarded. Fix: `while let Some(line) = lines.next_line().await.map_err(|e| format!("read stdout: {e}"))?`.

**[low] SAFE_MCP allowlist contains four stripped tools** — `assistant/mod.rs:2425` *(deduped from findings 11 & 12)*
`SAFE_MCP` includes `mcp__rift__sync_status`, `mcp__rift__drift_snapshot`, `mcp__rift__reconcile_preview`, and `mcp__rift__ask_user` — none appear anywhere in `mcp_server.rs` post-conversion. These dead names are passed verbatim as `--allowed-tools` to the CLI. The Claude CLI ignores allowlist entries for non-existent tools so no crash or permission escalation occurs, but the entries mislead readers of the permission model. Fix: trim `SAFE_MCP` to `"mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep"`.

**[low] Unreachable `None` arm in status match emits false error event** — `assistant/mod.rs:2856`
The only `break None` in the wait loop (line 2802) is gated on `result_seen == true`, but the guard at line 2850 returns `Ok(())` before the match at line 2856 is reached when `result_seen` is true — making the `None` arm permanently dead. If a future refactor makes it reachable, a spurious error event fires for an already-successful turn. Fix: replace with `.expect("status is always Some — invariant violated")` to document and loudly catch any future regression.

**[low] `FinalPayload.cleaned` flag wrong when scrubbed output is empty** — `stt/mod.rs:500`
`cleaned: active.cleanup_enabled` is emitted unconditionally, but the cleanup branch only executes when `cleanup_enabled && !scrubbed.is_empty()`. When `scrubbed` is empty and cleanup is enabled, the else branch runs with no cleanup yet `cleaned: true` is reported to the frontend. Currently the frontend ignores the flag, so there is no behavioral impact, but it is a latent lie. Fix: track `did_clean: bool` through the conditional and emit that value.

**[low] `load_config` silently swallows all I/O errors** — `stt/mod.rs:139`
`Err(_) => return SttConfig::default()` discards all error variants — `NotFound`, `PermissionDenied`, disk errors — identically and silently. The parse-error path at lines 144–147 does emit `log::warn!`. Fix: match on `e.kind()` — silently default only for `NotFound`, log warn for everything else.

**[low] `tool_grep` off-by-one lets one extra file scan past `MAX_GREP_FILES`** — `assistant/mcp_server.rs:291`
`files_scanned` is incremented then checked with `>` (strictly-greater). File number `MAX_GREP_FILES+1` (5001) is fully read and UTF-8-decoded before the break fires — the regex loop at line 299 is skipped for it, so no extra search occurs, but one extra read+decode crosses the intended cap. Fix: change `>` to `>=` or move the increment before the guard.

**[low] Progress-event emit error silently dropped in diagnostics pump** — `diagnostics/mod.rs:474`
`let _ = app.emit("diag://event", &ev)` discards the Result. The `RecvError::Lagged` and `Closed` arms log, but IPC-level emit failures (webview torn down, serialization error) are invisible. Diagnostic events are non-critical, but the pattern violates fail-loud. Fix: `if let Err(e) = app.emit(...) { log::warn!("diag emit error: {e}"); }`.

**[low] Dead `DiagStage` variants from stripped pipeline** — `diagnostics/mod.rs:41` *(deduped from findings 18 & 44)*
~22 variants (`FsEvent`, `Debounced`, `Queued`, `UploadStart/Done/Fail`, `AtomicRename`, `Lock*`, `DriftScanProgress`, `BridgePing/Ack`, `RescanSignal`, `SftpConnect/Disconnect`, `ConnectionWedged`, `RemoteScan*`, `RemotePull*`, `Baseline*`) have zero emit sites outside `diagnostics/mod.rs` itself after the 2026-06-03 pure-assistant conversion. They are only referenced in the dead arms of `is_critical`'s `matches!` guard. The module doc comment (lines 1–18) still describes the full "autosync pipeline." Fix: trim enum to `Log` and `System`; delete dead match arms; rewrite module doc.

**[low] Dead unreachable early-return in `tool_git_status`** — `assistant/git_local.rs:194`
`git status --porcelain=v1 -b` always emits at least the `## <branch>` header when it exits 0, so `lines.is_empty()` is permanently false after `out.ok()` passes. The "Working tree clean." message it would return is already correctly emitted at line 210 via `changes.is_empty()`. Fix: remove the `lines.is_empty()` guard (lines 194–196) entirely.

**[low] Pre-release label comparison is lexicographic, not semver-ordered** — `commands/update.rs:248`
`(Some(rp), Some(lp)) => rp > lp` uses plain string comparison. For numbered suffixes (`alpha2` vs `alpha10`) char-order gives `"alpha2" > "alpha10"`, incorrectly suppressing updates. Current release history uses bare labels (alpha, beta, rc) with no numeric suffix, so the path is latent. Fix: use the `semver` crate or parse trailing numeric portions; at minimum document the numeric-suffix constraint.

**[low] Progress-event emit error silently dropped in update download** — `commands/update.rs:206`
`let _ = app.emit("update://download-progress", ...)` discards the Result on every download loop iteration. Download correctness is independently validated by byte-count comparison at lines 214–218, so a silent emit failure cannot corrupt the download — but UI progress feedback stops with no log. Fix: `if let Err(e) = app.emit(...) { log::warn!(...) }`.

**[low] SHA-256 hasher silently skipped on resume** — `stt/model_manager.rs:198`
`hasher` is `None` when `resume_from != 0`, and the integrity check at line 239 gates on `hasher.is_some()`. All four catalogue entries already have `sha256: Some(...)`, so a resumed download on a pre-corrupted partial file passes verification silently. Fix: re-hash the already-written bytes before resuming the stream, or perform a full-file hash after the rename.

**[low] `emit_progress` swallows Tauri emit errors** — `stt/model_manager.rs:267`
`emit_progress` discards `app.emit()` Results via `let _ =` with no logging. `ProgressPayload` contains only primitives, making serialization failure essentially impossible, but the pattern still violates fail-loud. Fix: `if let Err(e) = app.emit(...) { log::warn!(...) }`.

**[low] U16-to-f32 normalization uses wrong divisor** — `stt/audio.rs:146`
`(*s as f32 - i16::MAX as f32 - 1.0) / i16::MAX as f32` correctly centers at 32768.0 but divides by 32767.0. Extreme u16 values map to ±1.000031, marginally outside `[-1, 1]`. Whisper's f32 pipeline handles this without crashing but with minor theoretical ASR degradation. Fix: `(*s as f32 - 32768.0) / 32768.0`.

**[low] Resampler error logged at debug level — audio silently dropped** — `stt/audio.rs:215`
A `guard.process()` `Err` logs at `log::debug!` and breaks the chunk loop; at default log levels (info/warn) the failure is completely invisible. `push_samples` returns `()` so the caller cannot observe the failure. A persistent resampler fault causes the STT ring to receive no audio while the capture stream appears healthy. Fix: upgrade to `log::warn!`.

**[low] Dead no-op constructor in stub `WhisperEngine`** — `stt/whisper.rs:139`
`_new_unused()` is decorated with `#[allow(dead_code)]` and its own comment admits "there is no construction site." It exists solely to suppress field-unused warnings in a stub where `load` always returns `Err`. Fix: delete the function; apply `#[allow(dead_code)]` directly to the struct fields.

**[low] `show()`/`set_focus()` results silently discarded** — `lib.rs:116`
Both calls use `let _ =` inside the `setup` closure, which already returns `Result<(), Box<dyn Error>>`. A `show()` failure would leave the app permanently invisible with no log. The handle cannot realistically be invalidated between acquisition and use here, so practical risk is near-zero, but the pattern still violates fail-loud. Fix: log or propagate via `?`.

**[low] `set_position` result silently discarded after Win32 work-area calculation** — `lib.rs:91`
After successfully calling `SPI_GETWORKAREA` and computing coordinates, `window.set_position(...)` result is dropped via `let _ =`. The function early-returns on `SPI_GETWORKAREA` failure (line 85) and `outer_size` failure (line 86), making this arm inconsistent. A failure means the window appears at the default position — cosmetic, but should be logged. Fix: `if let Err(e) = window.set_position(...) { log::warn!(...) }`.

**[low] Silent fallback on missing filename produces wrong tmp path** — `state/paths.rs:82`
`.unwrap_or("snapshot")` substitutes `"snapshot"` when `path.file_name()` returns `None` (root dir or `..` terminal). `path.with_file_name(...)` then places the tmp file in the wrong parent, and the subsequent `rename` fails or touches a wrong path. `atomic_write_json` currently has zero call sites (dead function), so no live path triggers this. Fix: replace with an explicit early `Err` via `.ok_or_else(|| io::Error::new(InvalidInput, "path has no filename"))?`.

**[low] Stale comment references removed `SyncSnapshot` module** — `state/paths.rs:69`
Lines 68–72 describe a race between `SyncSnapshot::set` (flush loop) and `replace_under` (rebaseline) — both deleted in the 2026-06-03 conversion. The implementation (unique tmp suffix) remains correct; only the rationale comment is stale. Fix: trim to describe the general concurrent-writer concern without naming the removed modules.

**[low] `stdin` write errors silently swallowed** — `stt/cleanup.rs:76`
`let _ = stdin.write_all(...)` and `let _ = stdin.shutdown()` discard errors. A broken-pipe failure causes the Claude CLI subprocess to receive empty input; it either errors (logged at line 93–98) or emits empty stdout (caught at line 102–104), so no silent data loss occurs — but the root cause is never diagnosed. Fix: log `write_all`/`shutdown` errors at warn level.

**[low] Timeout does not kill child process** — `stt/cleanup.rs:86`
The timeout arm returns `Ok(raw.to_string())` without calling `child.kill().await`. Tokio's `Child::drop` on Windows closes handles but does not terminate the process. The spawned `claude -p` subprocess continues running until it completes naturally. STT cleanup is user-triggered (not a tight loop), so accumulation is slow, but repeated timeouts leak orphan processes. Fix: add `let _ = child.kill().await;` before the return.

**[low] VAD errors silently treated as 'no speech'** — `stt/vad.rs:40`
`vad.is_voice_segment(&frame_i16).unwrap_or(false)` maps any processing error to "not speech." Frame size is guaranteed correct by `chunks_exact(FRAME_SAMPLES)`, so the main error path is an internal C library fault — rare, but when it occurs, speech frames are silently discarded before Whisper. Fix: log at warn level before defaulting; optionally propagate via a `Result`-returning wrapper.

**[low] No-op match arm on VAD constructor** — `stt/vad.rs:25`
`match Vad::new_with_rate_and_mode(...) { v => v }` is a pure identity match — the constructor is infallible (`-> Self`). If it were ever changed to return a `Result`, this pattern would silently pass the `Result` through as `vad`, breaking all subsequent calls. Fix: replace with a direct `let mut vad = Vad::new_with_rate_and_mode(...)`.

**[low] `set_position`/`set_size`/`show` errors discarded in browser `open()` reopen path** — `browser/mod.rs:31`
The create path (`add_child`) correctly propagates errors via `?`, but the reopen path uses `let _ =` for all three geometry/visibility calls. Repositioning failures on an existing webview are swallowed; the browser dock may display misaligned with no feedback. Fix: propagate these via `.map_err(...)?` consistent with the create path.

**[low] `set_bounds()` swallows `set_position`/`set_size` errors — return type is decorative** — `browser/mod.rs:64`
`set_bounds()` returns `Result<(), String>` but can never return `Err` — both geometry calls use `let _ =`. Layout failure is invisible to callers; `show()`, `hide()`, and `navigate()` in the same file all propagate their errors. Fix: `wv.set_position(...).map_err(|e| format!("set_position: {e}"))?`.

**[low] `register()` silently drops sender on poisoned mutex in `PermissionRegistry`** — `assistant/permission.rs:41`
`if let Ok(mut g) = self.inner.lock()` silently skips the insert on a poisoned mutex, dropping `tx` immediately. The returned `rx` is closed before the caller awaits it; `RecvError` is indistinguishable from user-deny or cancellation. Mutex poisoning requires a panic inside a `HashMap::insert`, which is effectively impossible in practice. Fix: `.expect("PermissionRegistry mutex poisoned")` to fail loudly rather than silently.

**[low] `register()` silently discards sender on poisoned mutex in `ask_user`** — `assistant/ask_user.rs:36`
Same pattern as `permission.rs`: `if let Ok` on `self.inner.lock()` silently drops `tx` if the mutex is poisoned, returning a closed `rx`. Any future call also silently fails while the mutex stays poisoned. Fix: `.expect("AskUserRegistry mutex poisoned")` or `unwrap_or_else(|e| e.into_inner())` with logging.

**[low] Stale doc-comment references removed `remote_bridge` module** — `assistant/ask_user.rs:8`
Module doc step 3 names `remote_bridge::ask_user_op` as the registrant. The `remote_bridge` module was deleted in the 2026-06-03 conversion — no file matching `remote_bridge*` exists under `src-tauri/src/`. Fix: update step 3 to name the actual current dispatcher in `assistant/mod.rs`, or remove the step until re-documented.

**[low] Dead `bridge_token_key` and `rcon_password_key` in `secrets.rs`** — `secrets.rs:53–61` *(deduped from findings 40 & 45)*
Both `pub fn`s have zero callers across all Rust source — the bridge and RCON subsystems were deleted in the 2026-06-03 conversion. The module doc comment (line 3) still leads with "Replaces plaintext storage of bridge_token (per server)" as the primary rationale. Fix: delete lines 53–61; update the module doc comment to reflect that the surviving purpose is `ASSISTANT_API_KEY` storage only.

**[low] `set()` accepts empty-string values that `get()` silently discards** — `secrets.rs:34`
`get` treats `Ok(v) if v.is_empty()` as absent; `set` has no such guard and writes empty strings to the keychain. A `set(key, "")` call creates a phantom entry invisible to `get`, silently breaking round-trip semantics. Current callers in `mod.rs` both pre-filter empty values, but the public API surface is unguarded. Fix: early guard in `set`: `if value.is_empty() { return delete(key); }`.

**[low] `spawn()` silently succeeds when VS Code is not installed** — `commands/mod.rs:36`
On Windows, `cmd /C code <path>` always spawns `cmd.exe` successfully. The child exit code (non-zero when `code` is absent from PATH) is discarded via `.map(|_| ())`. The frontend receives `Ok(())` and shows no error. Fix: await the child exit status via `.wait()` (or `.output()`) and return `Err` if `!status.success()`.

**[low] Stale module-level doc references stripped systems** — `state/mod.rs:3`
Lines 1–9 describe "Phase 1a port from C# Services/State/* + Services/Sync/SyncSnapshot.cs," WPF migration window byte-compatibility, `EditTrail` behind `SftpClient`, and Phase 1b. All referenced systems were removed in the 2026-06-03 conversion. The module now contains only `pub mod paths;`. Fix: replace the entire block with a one-line current-reality comment.
