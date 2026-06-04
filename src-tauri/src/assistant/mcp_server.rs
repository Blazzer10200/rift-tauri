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

use std::fmt::Write as _;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::assistant::git_local;

const PROTOCOL_VERSION: &str = "2025-03-26";
const MAX_READ_BYTES: u64 = 500 * 1024;
const MAX_LIST_ENTRIES: usize = 500;
const MAX_GREP_MATCHES: usize = 200;
const MAX_GREP_FILES: usize = 5000;
/// Per-file byte cap for grep: never load more than this from any one file
/// (F10 — bound the full read after the binary probe).
const MAX_GREP_FILE_BYTES: u64 = 4 * 1024 * 1024;
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
        // Fail-closed: a root we can't canonicalize can't serve as a reliable
        // containment boundary (the later starts_with check compares canonical
        // candidates), so drop it rather than store the raw path (F240/F244).
        .filter_map(|s| dunce::canonicalize(s).ok())
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
    // #258: `write!` directly into the String — skips the intermediate
    // allocation `push_str(&format!(...))` would create for each line.
    let _ = writeln!(out, "{}", resolved.display());
    for (name, is_dir, size) in &entries {
        if *is_dir {
            let _ = writeln!(out, "  {}/", name);
        } else {
            let _ = writeln!(out, "  {} ({} bytes)", name, size);
        }
    }
    if entries.len() >= MAX_LIST_ENTRIES {
        let _ = writeln!(out, "  (truncated at {} entries)", MAX_LIST_ENTRIES);
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

    // #120: when the glob has no path separator (`*.rs`, `*.svelte`), it's a
    // filename-only pattern in user intent. The raw regex compiled from
    // `*.rs` only matches at the path root (`foo.rs`, not `src/foo.rs`)
    // because `*` stops at `/`. Match on just the filename component in
    // that case instead. Globs containing `/` (e.g. `src/**/*.svelte`)
    // keep the relpath-match semantics.
    let glob_filename_only = glob_arg.is_some_and(|g| !g.contains('/'));
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
            // #120: filename-only globs (no `/`) match the basename; otherwise
            // match the full relpath like before.
            let target_s = if glob_filename_only {
                p.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                let rel = p.strip_prefix(&search_root).unwrap_or(p);
                rel.to_string_lossy().replace('\\', "/")
            };
            if !gm.is_match(&target_s) {
                continue;
            }
        }
        // F81: enforce the file-count cap BEFORE the expensive read, so the
        // (MAX+1)th file is never loaded just to be discarded.
        files_scanned += 1;
        if files_scanned > MAX_GREP_FILES {
            matches.push(format!("(scan stopped at {} files)", MAX_GREP_FILES));
            break 'walk;
        }
        // #71 + F80 + F10: open the file ONCE. Read up to the per-file byte cap
        // from a single handle (no second open for the full read), then NUL-probe
        // the head of that same buffer. Bounds memory at MAX_GREP_FILE_BYTES even
        // for a pathologically large text file.
        use std::io::Read;
        let Ok(f) = std::fs::File::open(p) else { continue };
        let mut bytes = Vec::new();
        if f.take(MAX_GREP_FILE_BYTES).read_to_end(&mut bytes).is_err() {
            continue;
        }
        if bytes.iter().take(8192).any(|&b| b == 0) {
            continue;
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for (lineno, line) in text.lines().enumerate() {
            if re.is_match(line) {
                let rel = p.strip_prefix(&search_root).unwrap_or(p);
                // F11: back off to a char boundary — `&line[..200]` panics when
                // byte 200 lands inside a multi-byte UTF-8 codepoint.
                let truncated = if line.len() > 200 {
                    let mut end = 200;
                    while end > 0 && !line.is_char_boundary(end) {
                        end -= 1;
                    }
                    &line[..end]
                } else {
                    line
                };
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

/// Assistant trust level for this MCP child, from `RIFT_TRUST_LEVEL` (injected
/// by `mod::write_mcp_config`). Gates the local git tools. Unknown/unset →
/// `readonly` (safe floor).
fn trust_level() -> &'static str {
    match std::env::var("RIFT_TRUST_LEVEL").as_deref() {
        Ok("full") => "full",
        Ok("standard") => "standard",
        _ => "readonly",
    }
}

fn trust_rank(level: &str) -> u8 {
    match level {
        "full" => 2,
        "standard" => 1,
        _ => 0,
    }
}

/// True when the current trust level is at least `min`. Gates both the
/// git-tool listing AND dispatch.
fn trust_at_least(min: &str) -> bool {
    trust_rank(trust_level()) >= trust_rank(min)
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
    // Local git tools (git_local.rs). Read set always listed; write set needs
    // Standard trust. Dispatch enforces the same gate server-side.
    tools.push(json!({
        "name": "git_status",
        "description": "Show the git working-tree status of the user's Rift workspace (branch + changed files, porcelain). Read-only. Use before committing or to see what changed.",
        "inputSchema": { "type": "object", "properties": {}, "required": [] }
    }));
    tools.push(json!({
        "name": "git_diff",
        "description": "Show a unified git diff for the user's Rift workspace. Read-only. `cached: true` shows staged changes; optional `path` scopes to one file. Output truncated at 64 KB.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "cached": { "type": "boolean", "description": "Show staged (index) changes instead of working-tree changes." },
                "path": { "type": "string", "description": "Optional workspace-relative file to scope the diff to." }
            },
            "required": []
        }
    }));
    tools.push(json!({
        "name": "git_log",
        "description": "Show recent commits (oneline) for the user's Rift workspace. Read-only. `max` caps the count (default 20, hard max 100).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "max": { "type": "integer", "description": "Number of commits (1-100). Default 20.", "minimum": 1, "maximum": 100 }
            },
            "required": []
        }
    }));
    if trust_at_least("standard") {
        tools.push(json!({
            "name": "git_pull",
            "description": "Pull the current branch from upstream in the user's Rift workspace. Fast-forward only by default; `rebase: true` rebases. Refuses on a dirty working tree (stash/commit first). Surfaces merge errors verbatim — never silently merges.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "rebase": { "type": "boolean", "description": "Use --rebase instead of --ff-only." }
                },
                "required": []
            }
        }));
        tools.push(json!({
            "name": "git_commit",
            "description": "Stage and commit changes in the user's Rift workspace. `message` is required. Provide `paths` (workspace-relative) to stage specific files, or `all: true` to stage everything; omit both to commit only what's already staged. Refuses an empty message or an empty index.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "message": { "type": "string", "description": "Commit message (1-4096 bytes)." },
                    "paths": { "type": "array", "items": { "type": "string" }, "description": "Workspace-relative paths to stage before committing." },
                    "all": { "type": "boolean", "description": "Stage all tracked+untracked changes (git add -A) before committing." }
                },
                "required": ["message"]
            }
        }));
        tools.push(json!({
            "name": "git_push",
            "description": "Push the current branch to its remote in the user's Rift workspace. Defaults to `origin` + current branch. Force push is NOT permitted. Auth uses the user's system git/SSH config.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "remote": { "type": "string", "description": "Remote name. Default `origin`." },
                    "branch": { "type": "string", "description": "Branch to push. Default = current branch." }
                },
                "required": []
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
                "git_status" => git_local::tool_git_status(&args, roots),
                "git_diff" => git_local::tool_git_diff(&args, roots),
                "git_log" => git_local::tool_git_log(&args, roots),
                "git_pull" if trust_at_least("standard") => git_local::tool_git_pull(&args, roots),
                "git_commit" if trust_at_least("standard") => git_local::tool_git_commit(&args, roots),
                "git_push" if trust_at_least("standard") => git_local::tool_git_push(&args, roots),
                // #72: gate call-path the same way the list-path gates the
                // tool declaration. Env-stripped MCP launchers see "unknown
                // tool" instead of a silent ignore + no response.
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
            // #68: MCP 2025-03-26 spec requires -32700 Parse error reply
            // when an id is derivable from the malformed payload. Try a
            // minimal `{id: ...}` parse before discarding.
            Err(_) => {
                #[derive(Deserialize)]
                struct IdOnly { id: Option<Value> }
                if let Ok(probe) = serde_json::from_str::<IdOnly>(line) {
                    if let Some(id) = probe.id {
                        let err_resp = RpcResponse {
                            jsonrpc: "2.0",
                            id,
                            result: None,
                            error: Some(RpcError { code: -32700, message: "parse error".into() }),
                        };
                        if let Ok(s) = serde_json::to_string(&err_resp) {
                            if writeln!(out, "{}", s).is_err() { return; }
                            if out.flush().is_err() { return; }
                        }
                    }
                }
                continue;
            }
        };
        let resp = handle_request(req, &roots);
        if let Some(r) = resp {
            let s = match serde_json::to_string(&r) {
                Ok(s) => s,
                // #70: a serialize failure means the client is waiting for a
                // response that will never arrive. `continue` would hang the
                // peer; bail the loop so the stdio child exits and the parent
                // sees the disconnect.
                Err(_) => return,
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
