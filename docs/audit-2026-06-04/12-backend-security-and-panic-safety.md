# Backend — Security & Panic-safety

_24 confirmed findings._ [← back to index](README.md)

## Backend — Security & Panic-safety Findings

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| **high** | Panic on non-ASCII grep match: byte-index slice mid-char-boundary | `mcp_server.rs:302` | Use `line.chars().take(200).collect::<String>()` instead of `&line[..200]` |
| **medium** | `bypassPermissions` on STT cleanup subprocess | `stt/cleanup.rs:58-59`, `stt/mod.rs:625` | Remove `--permission-mode bypassPermissions`; cleanup needs zero tool access |
| **medium** | SHA-256 integrity check skipped on resumed downloads | `stt/model_manager.rs:198-202` | On sha256-pinned entry with `resume_from > 0`, force fresh download (`resume_from = 0`, truncate partial) |
| **medium** | No URL scheme allowlist — `file://`, `data:` accepted | `browser/mod.rs:21`, `commands/browser.rs:15` | After `Url::parse`, reject any scheme other than `http`/`https` |
| **medium** | Command injection via `cmd /C code <path>` | `commands/mod.rs:27` | Resolve `code.cmd` explicitly and invoke directly via `Command::new(code_cmd).arg(&path)`; avoid `cmd /C` |
| **medium** | `stdout_task.await.unwrap_or_default()` silently swallows panic | `mod.rs:1789` | Mirror stderr pattern: `.unwrap_or_else(\|e\| { log::error!("summarize stdout task panicked: {e}"); Default::default() })` |
| **low** | Unsanitized path as CLI working directory | `mod.rs:479` | `canonicalize()` + `is_dir()` + cross-check against known workspace roots |
| **low** | Full filesystem path in IPC error return | `mod.rs:594` | Use `p.file_name()` or generic message; log full path via `log::error!` only |
| **low** | Unbounded message payload serialized without size guard | `mod.rs:601` | Guard before `to_string`: reject payload exceeding ~50 MB |
| **low** | Workspace roots joined with `\n` — no embedded-newline stripping | `mod.rs:737-741` | Strip `\n`/`\r` from each path component, or encode `RIFT_MCP_ROOTS` as JSON array |
| **low** | Raw stderr forwarded to frontend/IPC — may contain credential fragments | `mod.rs:1384, 1509, 2939-2944` | Redact `sk-ant-*` patterns; cap to ~500 chars; log full raw internally only |
| **low** | Panic on non-ASCII stderr: `buf[STDERR_TRIM..]` mid-char-boundary | `mod.rs:2766` | Use `char_indices` to find first safe boundary ≥ `STDERR_TRIM` before slicing |
| **low** | `current_branch()` output used as git arg without `validate_ref` | `git_local.rs:347` | `None => validate_ref("branch", &current_branch(root)?)?` |
| **low** | Raw git stderr may leak credential fragments in remote URLs | `git_local.rs:122` | Redact `https?://[^@]*@` → `https://<redacted>@` before returning from `err_text()` |
| **low** | `fields` (`serde_json::Value`) forwarded to frontend without scrubbing | `diagnostics/mod.rs:286` | Walk JSON in `publish()` and apply `scrub_log_message` to every string leaf |
| **low** | Panic payload forwarded verbatim to frontend diagnostic stream | `lib.rs:47` | Truncate to 256 chars; strip known secret patterns; or emit fixed-text event to frontend |
| **low** | Unchecked i32 cast in window centering — window can be placed off-screen | `lib.rs:89-90` | Clamp `x`/`y` to work-area bounds; guard `size.width > i32::MAX as u32` |
| **low** | User-supplied URL reflected verbatim into IPC error string | `commands/update.rs:170` | Embed parsed host component only, not raw caller-supplied string |
| **low** | `models_dir()` silently falls back to CWD when home env vars absent | `stt/model_manager.rs:80` | Return `Result`; use `dirs::home_dir()` and propagate `None` as an error |
| **low** | Claude CLI stderr logged verbatim in cleanup — may contain auth diagnostics | `stt/cleanup.rs:96` | Cap to ~200 chars; strip bearer/token patterns before `log::warn!` |
| **low** | Download error masked by cleanup-lock `?` — wrong error returned to caller | `stt/mod.rs:604` | `if let Ok(mut slot) = state.0.lock() { *slot = None; }` then unconditionally `return res` |
| **low** | Unsanitized workspace context injected into Claude system prompt | `stt/cleanup.rs:119` | Wrap `capped` in XML delimiters (`<project_terms>…</project_terms>`); allowlist identifiers only |
| **low** | `stdin.write_all()` and `shutdown()` errors silently discarded | `stt/cleanup.rs:76-77` | Log write errors at `warn!` and return raw transcript immediately on failure |
| **low** | `vad.is_voice_segment().unwrap_or(false)` silently drops VAD errors | `stt/vad.rs:40` | `.unwrap_or_else(\|e\| { log::warn!("VAD error: {e}"); false })` |

---

### Finding Detail

**[high] mcp_server.rs:302 — Byte-index slice panics on non-ASCII**
`&line[..200]` indexes by byte offset into a validated UTF-8 `&str`. Any source file line longer than 200 bytes containing a multi-byte codepoint (Cyrillic, CJK, emoji) whose encoding straddles byte 200 causes a `'byte index 200 is not a char boundary'` panic. The panic kills the MCP stdio child process, severing the assistant connection. Fix: `line.chars().take(200).collect::<String>()`.

---

**[medium] stt/cleanup.rs:58-59 + stt/mod.rs:625 — bypassPermissions on transcript subprocess**
Every STT cleanup invocation spawns `claude -p --permission-mode bypassPermissions`. The raw Whisper transcript is piped to stdin unsanitized. `bypassPermissions` disables all tool-use gates — a prompt-injection payload in transcribed speech (or Whisper hallucination) that persuades the model to invoke a tool proceeds with no confirmation. The flag is entirely unnecessary for text-in/text-out cleanup. Remove it; `--print` mode suffices.

**[medium] stt/model_manager.rs:198-202 — SHA-256 skipped on resume**
`hasher` is set to `None` whenever `resume_from > 0`. All four catalogue entries have `sha256: Some(...)`. The `if let (Some(expected), Some(h))` integrity gate therefore short-circuits on any resumed download, unconditionally renaming the partial to the final path. A crafted or corrupted `.partial` in `~/.rift/models/` passes silently. Fix: force `resume_from = 0` and truncate any partial when a sha256-pinned entry is requested.

**[medium] browser/mod.rs:21 + commands/browser.rs:15 — No URL scheme allowlist**
`parse_url` does only `Url::parse(raw)` — no scheme check. Both `browser_open` and `browser_navigate` pass the result directly to `wv.navigate()`. A `file://` URL loads arbitrary local filesystem paths in the embedded child WebView2. Fix: after parsing, reject any scheme other than `http`/`https`.

**[medium] commands/mod.rs:27 — cmd /C code \<path\> command injection**
`c.args(["/C", "code", &path])` where `path` is raw frontend input. `cmd.exe /C` reconstructs all trailing arguments into a single command line, so metacharacters (`&`, `|`, `>`) in `path` are interpreted by the shell. A path like `C:\foo & calc` executes `calc`. Fix: resolve `code.cmd` via PATH and invoke it directly with `.arg(&path)` — no shell intermediary.

**[medium] mod.rs:1789 — stdout_task panic swallowed via unwrap_or_default**
`stdout_task.await.unwrap_or_default()` silently discards a `JoinError`. All output fields default to zero/empty; the caller receives `Err("summarize call returned empty text")` with no indication of the panic. The sibling `stderr_task` at line 1791 correctly uses `unwrap_or_else` with logging. Mirror that pattern for the stdout side.

---

**[low] mod.rs:479 — Unsanitized path from disk as CLI working directory**
`load_session_cwd` returns the raw sidecar path with no canonicalization or workspace-root cross-check. A tampered sidecar (same-user write access) can steer the CLI spawn into a malicious directory with a crafted `CLAUDE.md`/`.mcp.json`. Add `std::fs::canonicalize` + `is_dir()` + optional check against known `recent_roots`.

**[low] mod.rs:594 — Full filesystem path in IPC error return**
`format!("read {}: {e}", p.display())` embeds the absolute path (including home dir) in the Tauri IPC error string, which propagates to the frontend. Use `p.file_name()` or a generic message for the IPC return; log the full path with `log::error!` only.

**[low] mod.rs:601 — Unbounded payload serialized without size guard**
`assistant_save_conversation` accepts `Conversation.messages: serde_json::Value` from IPC with no size limit. `serde_json::to_string` allocates the entire value in memory before writing. A large blob causes OOM or disk exhaustion. Add a size check before serialization (e.g., reject if estimated size exceeds 50 MB).

**[low] mod.rs:737-741 — Workspace roots joined with `\n`, no newline stripping**
`RIFT_MCP_ROOTS` is constructed via `.join("\n")` and parsed in `mcp_server.rs` via `.lines()`. A path component containing a literal newline (illegal on NTFS but a latent portability hazard) would split into two roots. Strip `\n`/`\r` from each component before joining, or encode as a JSON array.

**[low] mod.rs:1384, 1509, 2939-2944 — Raw stderr forwarded to frontend/IPC**
The enhance (1384), title (1509), and main turn exit (2939) error paths all format raw `stderr_buf` into the IPC error string and/or emit it via `ERROR_EVENT`. The subprocess runs with `ANTHROPIC_API_KEY` in its environment. While the CLI does not echo the key itself, error messages, stack traces, or MCP subprocess output are forwarded unscrubbed. Redact `sk-ant-*` patterns, cap to ~500 chars, and keep the full raw value in local logs only.

**[low] mod.rs:2766 — Stderr trim slices mid-char-boundary**
`buf[STDERR_TRIM..]` slices a `String` at fixed byte offset 32768. If a multi-byte UTF-8 sequence straddles that boundary, Rust panics. The panic is caught at the `tokio::spawn` boundary and logged, so no silent failure occurs — but the panic is avoidable. Use `char_indices` to find the first safe char boundary at or after `STDERR_TRIM`.

**[low] git_local.rs:347 — current_branch() bypasses validate_ref**
The `None` branch of the `branch` match calls `current_branch(root)?` and returns raw trimmed stdout directly into `run_git(&["push", &remote, &branch])` without `validate_ref`. The `Some(b)` branch does call `validate_ref`. A crafted `.git/HEAD` containing a flag-shaped branch name (e.g. `--upload-pack=evil`) would bypass the guard. Fix: `None => validate_ref("branch", &current_branch(root)?)?`.

**[low] git_local.rs:122 — Raw git stderr may contain credential fragments**
`err_text()` returns git's raw stderr verbatim. Git can emit `fatal: Authentication failed for https://user:TOKEN@github.com/...` when a remote URL embeds credentials. These propagate through `tool_git_pull`, `tool_git_commit`, `tool_git_push` into MCP responses and logs. Apply a regex to redact `https?://[^@]*@` before returning.

**[low] diagnostics/mod.rs:286 — fields forwarded unscrubbed**
`publish()` scrubs `event.message` and `event.file` but forwards `event.fields` (`serde_json::Value`) unchanged. Any caller embedding sensitive data in `fields` bypasses `scrub_log_message`. Walk the JSON in `publish()` and apply scrubbing to every string leaf.

**[low] lib.rs:47 — Panic payload forwarded verbatim to frontend**
The global panic hook emits the raw payload string via `diag://event`. A panic inside code that formats sensitive data into its message would relay that to the frontend renderer. Truncate to 256 chars and strip known secret patterns, or emit a fixed-text event to the frontend and keep the full payload in stderr only.

**[low] lib.rs:89-90 — Unchecked i32 cast in window centering**
`size.width as i32` and `size.height as i32` are not bounds-checked. If the window is wider/taller than the work area, `x`/`y` goes negative and the window is placed partially off-screen. Clamp `x = x.max(wa.left)` and `y = y.max(wa.top)` before calling `set_position`.

**[low] commands/update.rs:170 — User-supplied URL reflected in IPC error**
`format!("Refusing to download from an unexpected host: {url}")` echoes the raw caller-supplied string back via IPC. Replace `{url}` with the parsed host component only (e.g., `url::Url::parse(&url).ok().and_then(|u| u.host_str().map(str::to_owned)).unwrap_or_default()`).

**[low] stt/model_manager.rs:80 — models_dir() falls back to CWD on missing home env**
When both `USERPROFILE` and `HOME` are unset, `models_dir()` returns `PathBuf::from(".")`, writing/deleting model files relative to CWD. Return `Result<PathBuf>` and use `dirs::home_dir()` to surface a clear error rather than silently using CWD.

**[low] stt/cleanup.rs:96 — Claude CLI stderr logged verbatim**
On non-zero exit, `String::from_utf8_lossy(&output.stderr).trim()` is passed verbatim to `log::warn!`. Auth failure messages are not typically credential-bearing, but the pattern is inconsistent with scrubbing elsewhere. Cap to ~200 chars and strip token-pattern lines before logging.

**[low] stt/mod.rs:604 — Download error masked by cleanup-lock `?`**
After `model_manager::download` completes, the post-download cleanup re-acquires the mutex with `?`. If the mutex is poisoned, the early return discards `res` and returns `Err("cancel slot lock: {e}")`. The caller receives the wrong error. Use `if let Ok(mut slot) = state.0.lock() { *slot = None; }` and unconditionally `return res`.

**[low] stt/cleanup.rs:119 — Workspace context injected into system prompt**
`workspace_context()` builds `ctx` from directory name, branch name, and file basenames — all attacker-controlled for a malicious repo. This is interpolated (only 300-char truncated) into `--append-system-prompt`. A crafted branch name can override cleanup instructions. Wrap in XML delimiters (`<project_terms>…</project_terms>`) and allowlist only safe identifier characters.

**[low] stt/cleanup.rs:76-77 — stdin write errors silently discarded**
Both `stdin.write_all()` and `stdin.shutdown()` errors are dropped with `let _ =`. If the write fails, the child receives empty input and the fallback returns the raw transcript with no log explaining why. Log write errors at `warn!` and return the raw transcript immediately on failure.

**[low] stt/vad.rs:40 — VAD errors silently treated as silence**
`vad.is_voice_segment(&frame_i16).unwrap_or(false)` treats every engine error as non-speech with no diagnostic surface. While the `chunks_exact(FRAME_SAMPLES)` guarantee makes the primary error path unreachable, a defensive `.unwrap_or_else(|e| { log::warn!("VAD error: {e}"); false })` should replace the silent fallback.
