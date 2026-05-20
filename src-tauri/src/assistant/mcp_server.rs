//! Rift MCP server — stdio JSON-RPC 2.0 endpoint exposing read-only
//! workspace tools to the Claude CLI.
//!
//! Spawned as a subprocess of `claude` itself via `--mcp-config`. The parent
//! Rift Tauri instance writes a temp MCP config that points the CLI at
//! `current_exe()` with `RIFT_MCP_SERVER=1` set; `lib.rs::run()` checks the
//! env at startup and branches here instead of launching the Tauri loop.
//!
//! Tools exposed:
//!   * `read_file(path)` — UTF-8 text, capped at 500 KB.
//!   * `list_dir(path)` — non-recursive directory listing.
//!   * `grep(pattern, path?, glob?)` — regex over the workspace,
//!      walkdir+regex (no ripgrep dep, works on Trey's box).
//!
//! Path safety: every requested path is canonicalized and checked to live
//! under one of the workspace roots passed in via `RIFT_MCP_ROOTS` (newline-
//! separated). The CLI's `--strict-mcp-config` plus our `--allowed-tools
//! mcp__rift__*` together guarantee these are the only tools Claude can call.

use std::io::{self, BufRead, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const PROTOCOL_VERSION: &str = "2025-03-26";
const MAX_READ_BYTES: u64 = 500 * 1024;
const MAX_LIST_ENTRIES: usize = 500;
const MAX_GREP_MATCHES: usize = 200;
const MAX_GREP_FILES: usize = 5000;
const SKIP_DIRS: &[&str] = &[
    "node_modules", ".git", ".svelte-kit", "build", "dist", "target",
    ".rift-trail", ".rift-tmp", "__pycache__", ".venv", ".next",
];

#[derive(Debug, Deserialize)]
struct RpcRequest {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

fn load_roots() -> Vec<PathBuf> {
    std::env::var("RIFT_MCP_ROOTS")
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| dunce::canonicalize(s).ok().or_else(|| Some(PathBuf::from(s))))
        .collect()
}

/// Resolve `path` (which may be absolute or relative) to an absolute path that
/// MUST live under one of the workspace roots. Returns canonicalized
/// `PathBuf` on success, error string otherwise.
fn resolve_under_roots(path: &str, roots: &[PathBuf]) -> Result<PathBuf, String> {
    if roots.is_empty() {
        return Err("no workspace root configured (start a server connection in Rift first)".into());
    }
    let raw = PathBuf::from(path);
    // For relative paths, anchor to the first root.
    let candidate = if raw.is_absolute() {
        raw
    } else {
        roots[0].join(&raw)
    };
    // Canonicalize so `..` segments are resolved before the root check.
    let canon = std::fs::canonicalize(&candidate)
        .map_err(|e| format!("canonicalize {}: {e}", candidate.display()))?;
    // Windows-friendly canonical (strip UNC prefix).
    let canon = strip_unc(&canon);
    for root in roots {
        let root = strip_unc(root);
        if canon.starts_with(&root) {
            return Ok(canon);
        }
    }
    Err(format!(
        "{} is outside the workspace root(s)",
        canon.display()
    ))
}

#[cfg(windows)]
fn strip_unc(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        p.to_path_buf()
    }
}

#[cfg(not(windows))]
fn strip_unc(p: &Path) -> PathBuf {
    p.to_path_buf()
}

// Tiny path canonicalize wrapper that works through symlinks the same way
// std does. Kept as a module-level fn so tests / future-deferred logic can
// override; for now it's `std::fs::canonicalize`. The `dunce` crate would
// be nicer (strips UNC) but we already handle that in `strip_unc`.
mod dunce {
    use std::io;
    use std::path::{Path, PathBuf};
    pub fn canonicalize<P: AsRef<Path>>(p: P) -> io::Result<PathBuf> {
        std::fs::canonicalize(p)
    }
}

// ─── tool implementations ──────────────────────────────────────────────────

fn tool_read_file(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let path = args.get("path").and_then(|v| v.as_str()).ok_or("missing `path`")?;
    let resolved = resolve_under_roots(path, roots)?;
    let meta = std::fs::metadata(&resolved).map_err(|e| format!("stat: {e}"))?;
    if !meta.is_file() {
        return Err(format!("{} is not a regular file", resolved.display()));
    }
    if meta.len() > MAX_READ_BYTES {
        return Err(format!(
            "file is {} bytes; limit is {} bytes — paste a smaller excerpt or grep for the specific section",
            meta.len(),
            MAX_READ_BYTES
        ));
    }
    let bytes = std::fs::read(&resolved).map_err(|e| format!("read: {e}"))?;
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(_) => Err("file is not valid UTF-8 (binary or non-UTF8 encoding not supported)".into()),
    }
}

fn tool_list_dir(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let path = args.get("path").and_then(|v| v.as_str()).ok_or("missing `path`")?;
    let resolved = resolve_under_roots(path, roots)?;
    let meta = std::fs::metadata(&resolved).map_err(|e| format!("stat: {e}"))?;
    if !meta.is_dir() {
        return Err(format!("{} is not a directory", resolved.display()));
    }
    let mut entries: Vec<(String, bool, u64)> = Vec::new();
    let iter = std::fs::read_dir(&resolved).map_err(|e| format!("read_dir: {e}"))?;
    for de in iter {
        let de = match de {
            Ok(d) => d,
            Err(_) => continue,
        };
        let name = de.file_name().to_string_lossy().to_string();
        let ft = match de.file_type() {
            Ok(f) => f,
            Err(_) => continue,
        };
        let size = if ft.is_file() {
            de.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        entries.push((name, ft.is_dir(), size));
        if entries.len() >= MAX_LIST_ENTRIES {
            break;
        }
    }
    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut out = String::new();
    out.push_str(&format!("{}\n", resolved.display()));
    for (name, is_dir, size) in &entries {
        if *is_dir {
            out.push_str(&format!("  {}/\n", name));
        } else {
            out.push_str(&format!("  {} ({} bytes)\n", name, size));
        }
    }
    if entries.len() >= MAX_LIST_ENTRIES {
        out.push_str(&format!("  (truncated at {} entries)\n", MAX_LIST_ENTRIES));
    }
    Ok(out)
}

fn tool_grep(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let pattern = args.get("pattern").and_then(|v| v.as_str()).ok_or("missing `pattern`")?;
    let path_arg = args.get("path").and_then(|v| v.as_str());
    let glob_arg = args.get("glob").and_then(|v| v.as_str());

    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(args.get("case_insensitive").and_then(|v| v.as_bool()).unwrap_or(false))
        .multi_line(true)
        .build()
        .map_err(|e| format!("invalid regex `{pattern}`: {e}"))?;

    let glob_matcher = match glob_arg {
        Some(g) => Some(glob_to_regex(g)?),
        None => None,
    };

    let search_root = match path_arg {
        Some(p) => resolve_under_roots(p, roots)?,
        None => roots
            .first()
            .cloned()
            .ok_or("no workspace root configured")?,
    };

    let mut files_scanned = 0usize;
    let mut matches: Vec<String> = Vec::new();

    'walk: for entry in walkdir::WalkDir::new(&search_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.depth() > 0 && e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()) {
                return false;
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if let Some(ref gm) = glob_matcher {
            let rel = p.strip_prefix(&search_root).unwrap_or(p);
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            if !gm.is_match(&rel_s) {
                continue;
            }
        }
        // #71: Cheap binary skip — actually read only the first 8 KiB for the
        // NUL probe. Prior impl called `std::fs::read(p)` (loads entire file)
        // then `.iter().take(8192)`; under a 5000-file scan that loaded ~5000
        // full files into stdio-process memory. Now: open + take(8192) for the
        // probe, full read only after the file passes.
        {
            use std::io::Read;
            let mut probe = Vec::with_capacity(8192);
            let Ok(f) = std::fs::File::open(p) else { continue };
            if f.take(8192).read_to_end(&mut probe).is_err() {
                continue;
            }
            if probe.iter().any(|&b| b == 0) {
                continue;
            }
        }
        let bytes = match std::fs::read(p) {
            Ok(b) => b,
            Err(_) => continue,
        };
        files_scanned += 1;
        if files_scanned > MAX_GREP_FILES {
            matches.push(format!("(scan stopped at {} files)", MAX_GREP_FILES));
            break 'walk;
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for (lineno, line) in text.lines().enumerate() {
            if re.is_match(line) {
                let rel = p.strip_prefix(&search_root).unwrap_or(p);
                let truncated = if line.len() > 200 { &line[..200] } else { line };
                matches.push(format!("{}:{}: {}", rel.display(), lineno + 1, truncated));
                if matches.len() >= MAX_GREP_MATCHES {
                    matches.push(format!("(truncated at {} matches)", MAX_GREP_MATCHES));
                    break 'walk;
                }
            }
        }
    }

    if matches.is_empty() {
        Ok(format!("(no matches for `{}` in {} files under {})", pattern, files_scanned, search_root.display()))
    } else {
        Ok(matches.join("\n"))
    }
}

/// Whether the parent Rift process has enabled the remote-Bash tool for this
/// MCP-child spawn. Set via `RIFT_REMOTE_SHELL_ENABLED=1` in the MCP env
/// stanza written by `assistant::mod::write_mcp_config`. Gates both tool
/// listing AND tool dispatch — so even if the model name-collides with a
/// disabled tool, the call is rejected at the server side.
fn remote_shell_enabled() -> bool {
    std::env::var("RIFT_REMOTE_SHELL_ENABLED").as_deref() == Ok("1")
}

/// Whether the loopback bridge is reachable from this MCP-child spawn.
/// True whenever `RIFT_BRIDGE_PORT` + a read-only-or-write token are set.
/// `sync_status` uses the bridge without needing the user to opt into
/// remote-bash. #62: read-only token gates read ops; write token (only injected
/// when `remote_shell_enabled`) gates `remote_bash`.
fn bridge_enabled() -> bool {
    std::env::var("RIFT_BRIDGE_PORT").is_ok()
        && (std::env::var("RIFT_BRIDGE_READONLY_TOKEN").is_ok()
            || std::env::var("RIFT_BRIDGE_TOKEN").is_ok())
}

/// Dial the parent Tauri's loopback bridge for a single read-only op (`sync_status`).
/// Reuses the same TCP+NDJSON pattern as `tool_remote_bash` but is always
/// available when the bridge is running — no remote-shell gate.
fn tool_sync_status() -> Result<String, String> {
    if !bridge_enabled() {
        return Err("sync_status requires an active Rift server connection (no bridge env vars set)".into());
    }
    let port_s = std::env::var("RIFT_BRIDGE_PORT").map_err(|_| "RIFT_BRIDGE_PORT not set".to_string())?;
    // #62: read-only token first; fall back to the write-scoped token when
    // the readonly env var is missing (older MCP child spawns, or paranoid
    // ops where only the write token is around). Either authorizes `sync_status`.
    let token = std::env::var("RIFT_BRIDGE_READONLY_TOKEN")
        .or_else(|_| std::env::var("RIFT_BRIDGE_TOKEN"))
        .map_err(|_| "no RIFT_BRIDGE_*_TOKEN set".to_string())?;
    let port: u16 = port_s.parse().map_err(|e| format!("invalid RIFT_BRIDGE_PORT `{port_s}`: {e}"))?;

    let req = json!({ "op": "sync_status", "token": token });
    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|e| format!("bridge addr parse: {e}"))?;
    let mut stream = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|e| format!("bridge connect: {e}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let payload = format!("{}\n", req);
    stream.write_all(payload.as_bytes()).map_err(|e| format!("bridge write: {e}"))?;
    stream.flush().ok();

    let mut reader = io::BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| format!("bridge read: {e}"))?;
    if line.trim().is_empty() {
        return Err("bridge closed connection without a response".into());
    }
    let resp: Value = serde_json::from_str(line.trim())
        .map_err(|e| format!("bridge parse: {e} (raw: {})", line.trim()))?;

    if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        let msg = resp.get("error").and_then(|v| v.as_str()).unwrap_or("unknown bridge error");
        return Err(msg.to_string());
    }

    let data = resp.get("data").cloned().unwrap_or(Value::Null);
    if data.get("connected").and_then(|v| v.as_bool()) == Some(false) {
        return Ok("Sync engine: not connected (no active server configured in Rift)".into());
    }

    let state   = data.get("state").and_then(|v| v.as_str()).unwrap_or("unknown");
    let pending  = data.get("pending").and_then(|v| v.as_u64()).unwrap_or(0);
    let failed   = data.get("failed").and_then(|v| v.as_u64()).unwrap_or(0);
    let conflicts = data.get("conflicts").and_then(|v| v.as_u64()).unwrap_or(0);
    let watches  = data.get("watches").and_then(|v| v.as_u64()).unwrap_or(0);
    let detail   = data.get("detail").and_then(|v| v.as_str()).unwrap_or("");

    let mut out = format!("Sync engine: {state}");
    if !detail.is_empty() && detail != state {
        out.push_str(&format!(" — {detail}"));
    }
    out.push('\n');
    out.push_str(&format!(
        "  pending: {pending}  failed: {failed}  conflicts: {conflicts}  watches: {watches}"
    ));
    Ok(out)
}

/// Dial the parent Tauri's loopback bridge and run a single `remote_bash`
/// round-trip. Returns the formatted human-readable output (stdout + stderr +
/// exit code) ready to hand to the model, or an error string. Blocking I/O
/// because `run_stdio` is itself synchronous (one thread per MCP child).
fn tool_remote_bash(args: &Value) -> Result<String, String> {
    if !remote_shell_enabled() {
        return Err("remote_bash is disabled — toggle 'Allow remote shell' on the Rift Assistant Settings page and restart the conversation".into());
    }
    let port_s = std::env::var("RIFT_BRIDGE_PORT")
        .map_err(|_| "RIFT_BRIDGE_PORT not set on this MCP child".to_string())?;
    let token = std::env::var("RIFT_BRIDGE_TOKEN")
        .map_err(|_| "RIFT_BRIDGE_TOKEN not set on this MCP child".to_string())?;
    let port: u16 = port_s
        .parse()
        .map_err(|e| format!("invalid RIFT_BRIDGE_PORT `{port_s}`: {e}"))?;

    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("missing `command`")?;
    if command.trim().is_empty() {
        return Err("`command` is empty".into());
    }
    let timeout_secs: u64 = args
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(60)
        .clamp(1, 600);

    let req = json!({
        "op": "remote_bash",
        "token": token,
        "command": command,
        "timeout_secs": timeout_secs,
    });

    let addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|e| format!("bridge addr parse: {e}"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|e| format!("bridge connect: {e}"))?;
    // The bridge holds the connection open for the whole exec — read timeout
    // covers the worst case (full exec duration + bridge overhead).
    let read_to = Duration::from_secs(timeout_secs + 15);
    let _ = stream.set_read_timeout(Some(read_to));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let payload = format!("{}\n", req);
    stream
        .write_all(payload.as_bytes())
        .map_err(|e| format!("bridge write: {e}"))?;
    stream.flush().ok();

    let mut reader = io::BufReader::new(&stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("bridge read: {e}"))?;

    if line.trim().is_empty() {
        return Err("bridge closed connection without a response".into());
    }
    let resp: Value = serde_json::from_str(line.trim())
        .map_err(|e| format!("bridge parse: {e} (raw: {})", line.trim()))?;

    if resp.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        let msg = resp
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown bridge error");
        return Err(msg.to_string());
    }

    let stdout_s = resp.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
    let stderr_s = resp.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
    let exit_code = resp.get("exit_code").and_then(|v| v.as_i64());
    let truncated = resp
        .get("truncated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let cmd_preview = command.lines().next().unwrap_or(command);
    let cmd_preview = if cmd_preview.len() > 200 {
        &cmd_preview[..200]
    } else {
        cmd_preview
    };
    let mut out = String::new();
    out.push_str(&format!("$ {cmd_preview}\n"));
    if !stdout_s.is_empty() {
        out.push_str("--- stdout ---\n");
        out.push_str(stdout_s);
        if !stdout_s.ends_with('\n') {
            out.push('\n');
        }
    }
    if !stderr_s.is_empty() {
        out.push_str("--- stderr ---\n");
        out.push_str(stderr_s);
        if !stderr_s.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str(&format!(
        "--- exit: {} ---\n",
        exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".into())
    ));
    if truncated {
        out.push_str("(output truncated at 256 KB)\n");
    }
    Ok(out)
}

/// Tiny glob → regex compiler. Supports `*` (any, including `/`), `?` (one
/// non-`/`), `**` (also any), and literal everything else. Sufficient for
/// `*.rs`, `src/**/*.svelte` style patterns.
fn glob_to_regex(glob: &str) -> Result<regex::Regex, String> {
    let mut out = String::from("^");
    let mut chars = glob.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    out.push_str(".*");
                } else {
                    out.push_str("[^/]*");
                }
            }
            '?' => out.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('$');
    regex::Regex::new(&out).map_err(|e| format!("glob compile: {e}"))
}

// ─── JSON-RPC dispatch ─────────────────────────────────────────────────────

fn tools_list_payload() -> Value {
    let mut tools = vec![
        json!({
            "name": "read_file",
            "description": "Read a UTF-8 text file from the user's Rift workspace. Path is absolute, or relative to the workspace root. Files larger than 500 KB are rejected — use grep to locate the specific section instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path (absolute or workspace-relative)." }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "list_dir",
            "description": "List the immediate (non-recursive) contents of a directory in the user's Rift workspace. Returns one entry per line: directories suffixed with `/`, files with their byte size.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path (absolute or workspace-relative)." }
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "grep",
            "description": "Search the user's Rift workspace for a regex pattern. Skips node_modules, .git, build/dist/target, and binary files. Returns up to 200 matches as `path:line: snippet`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Regex pattern (Rust regex flavor, multiline mode on)." },
                    "path": { "type": "string", "description": "Optional subdirectory to scope the search to. Defaults to the full workspace root." },
                    "glob": { "type": "string", "description": "Optional glob to filter files, e.g. `*.rs` or `src/**/*.svelte`." },
                    "case_insensitive": { "type": "boolean", "description": "Case-insensitive match. Defaults to false." }
                },
                "required": ["pattern"]
            }
        }),
    ];
    if bridge_enabled() {
        tools.push(json!({
            "name": "sync_status",
            "description": "Get a live snapshot of the Rift sync engine state — pending uploads, failed uploads, conflict count, watches, and engine state (idle/syncing/error/watching). Call this when the user asks whether files are synced, how many are queued, or whether a push completed. More accurate than the per-turn <system-reminder> snapshot because it queries the engine at call time.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }));
    }
    if remote_shell_enabled() {
        tools.push(json!({
            "name": "remote_bash",
            "description": "Run a shell command on the Rift-connected remote SSH server, reusing the auto-sync engine's live russh session. Output is bounded to 256 KB per stream; commands time out after `timeout_secs` (default 60, max 600). A workspace-scoped advisory lock serializes calls across users — if another user is mid-exec, the call returns an error and you should retry shortly. Use for ops work (server status, pm2 restart, git pull on the remote, log inspection); prefer the file-edit tools for content changes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command line. Runs in the remote user's default shell (bash). Quote arguments yourself." },
                    "timeout_secs": { "type": "integer", "description": "Per-call timeout. Default 60, max 600.", "minimum": 1, "maximum": 600 }
                },
                "required": ["command"]
            }
        }));
    }
    json!({ "tools": tools })
}

fn handle_request(req: RpcRequest, roots: &[PathBuf]) -> Option<RpcResponse> {
    // Notifications (no id) get no response.
    let id = match req.id {
        Some(v) => v,
        None => return None,
    };

    let result = match req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "rift", "version": env!("CARGO_PKG_VERSION") }
        })),
        "tools/list" => Ok(tools_list_payload()),
        "tools/call" => {
            let name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
            let res = match name {
                "read_file" => tool_read_file(&args, roots),
                "list_dir" => tool_list_dir(&args, roots),
                "grep" => tool_grep(&args, roots),
                "sync_status" => tool_sync_status(),
                "remote_bash" => tool_remote_bash(&args),
                other => Err(format!("unknown tool: {other}")),
            };
            match res {
                Ok(text) => Ok(json!({
                    "content": [{ "type": "text", "text": text }]
                })),
                Err(msg) => Ok(json!({
                    "content": [{ "type": "text", "text": msg }],
                    "isError": true
                })),
            }
        }
        // Anything else: method-not-found.
        other => Err(format!("method `{other}` not supported")),
    };

    match result {
        Ok(v) => Some(RpcResponse { jsonrpc: "2.0", id, result: Some(v), error: None }),
        Err(msg) => Some(RpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError { code: -32601, message: msg }),
        }),
    }
}

/// Blocking stdio loop. Reads one JSON-RPC request per stdin line, writes one
/// response per stdout line. Newline-delimited NDJSON. Returns when stdin
/// closes (CLI parent exits) or on any fatal stdout write error.
pub fn run_stdio() {
    let roots = load_roots();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut reader = stdin.lock();
    let mut buf = String::new();
    loop {
        buf.clear();
        match reader.read_line(&mut buf) {
            Ok(0) => return,
            Ok(_) => {}
            Err(_) => return,
        }
        let line = buf.trim();
        if line.is_empty() {
            continue;
        }
        let req: RpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(_) => continue, // ignore garbage lines
        };
        let resp = handle_request(req, &roots);
        if let Some(r) = resp {
            let s = match serde_json::to_string(&r) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if writeln!(out, "{}", s).is_err() {
                return;
            }
            if out.flush().is_err() {
                return;
            }
        }
    }
}
