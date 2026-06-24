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
//!     walkdir+regex (no ripgrep dep).
//!   * `git_*` — local git read/write set, gated by `RIFT_TRUST_LEVEL`.
//!   * `ask_user` / `open_browser` / `notify` — UI ops round-tripped to the
//!     parent Rift process over the loopback bridge (`assistant/bridge.rs`),
//!     listed only when `RIFT_BRIDGE_PORT`+`RIFT_BRIDGE_TOKEN` are set.
//!
//! Path safety: every requested path is canonicalized and checked to live
//! under one of the workspace roots passed in via `RIFT_MCP_ROOTS` (newline-
//! separated). The CLI's `--strict-mcp-config` plus our `--allowed-tools
//! mcp__rift__*` together guarantee these are the only tools Claude can call.

use std::fmt::Write as _;
use std::io::{self, BufRead, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::assistant::git_local;

const PROTOCOL_VERSION: &str = "2025-03-26";
const MAX_READ_BYTES: u64 = 500 * 1024;
const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;
const MAX_LIST_ENTRIES: usize = 500;
const MAX_GREP_MATCHES: usize = 200;
const MAX_GREP_FILES: usize = 5000;
/// Per-file byte cap for grep: never load more than this from any one file
/// (F10 — bound the full read after the binary probe).
const MAX_GREP_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// Shared with `workspace::assistant_list_workspace_files` — keep one copy.
pub(crate) const SKIP_DIRS: &[&str] = &[
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

/// Per-project file-pattern filter, compiled from `RIFT_MCP_INCLUDE` /
/// `RIFT_MCP_EXCLUDE` (newline-separated globs, injected by
/// `mod::write_mcp_config`). Applied ON TOP of the always-on SKIP_DIRS baseline
/// — it scopes the user-facing project, never the hardcoded safety exclusions.
///
/// Matching is against the path RELATIVE to the workspace root, forward-slash
/// normalized — the same target shape `tool_grep`'s glob already uses. A glob
/// with no `/` matches the basename (so `*.rs` matches `src/main.rs`), mirroring
/// the grep `glob_filename_only` rule; a glob with `/` matches the full relpath.
#[derive(Default)]
pub(crate) struct PathFilter {
    include: Vec<(regex::Regex, bool)>, // (matcher, filename_only)
    exclude: Vec<(regex::Regex, bool)>,
}

fn compile_globs(raw: &str) -> Vec<(regex::Regex, bool)> {
    raw.lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|g| glob_to_regex(g).ok().map(|re| (re, !g.contains('/'))))
        .collect()
}

impl PathFilter {
    fn from_env() -> Self {
        PathFilter {
            include: compile_globs(&std::env::var("RIFT_MCP_INCLUDE").unwrap_or_default()),
            exclude: compile_globs(&std::env::var("RIFT_MCP_EXCLUDE").unwrap_or_default()),
        }
    }

    /// Build from explicit glob lists (newline-joined). Used by the `@`-mention
    /// file walk (parent process — no MCP env) and by tests, which both need a
    /// filter from patterns rather than the process-global env `from_env` reads.
    pub(crate) fn from_globs(include: &str, exclude: &str) -> Self {
        PathFilter {
            include: compile_globs(include),
            exclude: compile_globs(exclude),
        }
    }

    /// Visibility for a precomputed root-relative, `/`-normalized path. Public
    /// for the mention-picker walk, which already has relpaths in hand.
    pub(crate) fn allows_rel(&self, rel: &str) -> bool {
        self.allows(rel)
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.is_empty()
    }

    fn is_empty(&self) -> bool {
        self.include.is_empty() && self.exclude.is_empty()
    }

    /// Match a (filename_only, relpath) glob pair against a relative path.
    fn matches_any(set: &[(regex::Regex, bool)], rel: &str, file_name: &str) -> bool {
        set.iter().any(|(re, fname_only)| re.is_match(if *fname_only { file_name } else { rel }))
    }

    /// Whether a file at `rel` (root-relative, `/`-normalized) is visible:
    /// excluded paths are hidden; if any include globs exist, only matching
    /// paths are visible. Empty filter → everything visible.
    fn allows(&self, rel: &str) -> bool {
        let file_name = rel.rsplit('/').next().unwrap_or(rel);
        if Self::matches_any(&self.exclude, rel, file_name) {
            return false;
        }
        if self.include.is_empty() {
            return true;
        }
        Self::matches_any(&self.include, rel, file_name)
    }

    /// Compute the root-relative, `/`-normalized path of `p` under `root`, then
    /// test visibility. A path outside the root (shouldn't happen post-resolve)
    /// is allowed — the root-containment check is the real boundary.
    fn allows_path(&self, p: &Path, root: &Path) -> bool {
        if self.is_empty() {
            return true;
        }
        match p.strip_prefix(root) {
            Ok(rel) => self.allows(&rel.to_string_lossy().replace('\\', "/")),
            Err(_) => true,
        }
    }

    /// Directory visibility — only the EXCLUDE rule applies. Include globs must
    /// NOT hide a directory: an include like `src/**` needs `src/` to stay
    /// listed so the model can drill into it. An explicit exclude (`docs`,
    /// `vendor/**`) does hide the directory.
    fn allows_dir(&self, p: &Path, root: &Path) -> bool {
        if self.exclude.is_empty() {
            return true;
        }
        match p.strip_prefix(root) {
            Ok(rel) => {
                let rel = rel.to_string_lossy().replace('\\', "/");
                let file_name = rel.rsplit('/').next().unwrap_or(&rel);
                !Self::matches_any(&self.exclude, &rel, file_name)
            }
            Err(_) => true,
        }
    }
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
    // Canonicalize so `..` segments are resolved before the root check. Don't
    // echo the raw OS error (it discloses internal filesystem layout / existence
    // of sibling paths); a path that won't canonicalize is reported as not-found
    // relative to the workspace, which is all the caller needs.
    let canon = std::fs::canonicalize(&candidate)
        .map_err(|_| format!("path not found or unreadable: {}", path))?;
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

/// Reject a canonical path that lands inside any SKIP_DIRS segment. Enforced on
/// the post-canonicalize path so a workspace-internal symlink (or passing a
/// skipped dir directly as the arg, e.g. `.git`) can't bypass the exclusion.
/// Shared by read_file / list_dir / grep so all three agree on the boundary.
fn reject_skipped(resolved: &std::path::Path) -> Result<(), String> {
    if resolved
        .components()
        .any(|c| SKIP_DIRS.contains(&c.as_os_str().to_string_lossy().as_ref()))
    {
        return Err(format!(
            "{} resolves into an excluded directory (node_modules/.git/target/…)",
            resolved.display()
        ));
    }
    Ok(())
}

fn tool_read_file(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let path = args.get("path").and_then(|v| v.as_str()).ok_or("missing `path`")?;
    let resolved = resolve_under_roots(path, roots)?;
    reject_skipped(&resolved)?;
    // Per-project file-pattern scoping — a file the project excludes (or that
    // falls outside a non-empty include set) is reported as out-of-scope.
    let filter = PathFilter::from_env();
    if let Some(ws_root) = roots.first() {
        if !filter.allows_path(&resolved, ws_root) {
            return Err(format!(
                "{} is outside this project's file patterns",
                resolved.display()
            ));
        }
    }
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
    // Capped read: open + take to avoid TOCTOU races where the file grows
    // between the metadata check and the read.
    use std::io::Read as _;
    let f = std::fs::File::open(&resolved).map_err(|e| format!("read: {e}"))?;
    let mut bytes = Vec::new();
    f.take(MAX_READ_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read: {e}"))?;
    if bytes.len() > MAX_READ_BYTES as usize {
        return Err(format!(
            "file is {} bytes; limit is {} bytes — paste a smaller excerpt or grep for the specific section",
            meta.len(),
            MAX_READ_BYTES
        ));
    }
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(_) => Err("file is not valid UTF-8 (binary or non-UTF8 encoding not supported)".into()),
    }
}

fn tool_list_dir(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let path = args.get("path").and_then(|v| v.as_str()).ok_or("missing `path`")?;
    let resolved = resolve_under_roots(path, roots)?;
    reject_skipped(&resolved)?;
    let meta = std::fs::metadata(&resolved).map_err(|e| format!("stat: {e}"))?;
    if !meta.is_dir() {
        return Err(format!("{} is not a directory", resolved.display()));
    }
    // Per-project file-pattern scoping, relative to the workspace root.
    let filter = PathFilter::from_env();
    let ws_root = roots.first().cloned().unwrap_or_else(|| resolved.clone());
    // (name, is_dir, is_symlink, size)
    let mut entries: Vec<(String, bool, bool, u64)> = Vec::new();
    let iter = std::fs::read_dir(&resolved).map_err(|e| format!("read_dir: {e}"))?;
    for de in iter {
        let de = match de {
            Ok(d) => d,
            Err(_) => continue,
        };
        let name = de.file_name().to_string_lossy().to_string();
        // file_type() does NOT follow symlinks — accurate for symlink detection.
        let ft = match de.file_type() {
            Ok(f) => f,
            Err(_) => continue,
        };
        // Hide files the project filter excludes; keep directories navigable
        // (a parent dir of an included path must stay listed so the model can
        // drill down to it) — only an explicit exclude hides a directory.
        if !ft.is_dir() && !filter.allows_path(&de.path(), &ws_root) {
            continue;
        }
        if ft.is_dir() && !filter.allows_dir(&de.path(), &ws_root) {
            continue;
        }
        let is_symlink = ft.is_symlink();
        let size = if ft.is_file() {
            de.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        entries.push((name, ft.is_dir(), is_symlink, size));
        if entries.len() >= MAX_LIST_ENTRIES {
            break;
        }
    }
    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut out = String::new();
    // #258: `write!` directly into the String — skips the intermediate
    // allocation `push_str(&format!(...))` would create for each line.
    let _ = writeln!(out, "{}", resolved.display());
    for (name, is_dir, is_symlink, size) in &entries {
        if *is_symlink {
            let _ = writeln!(out, "  {} -> (symlink)", name);
        } else if *is_dir {
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
    if pattern.len() > 4096 {
        return Err("grep pattern too long (max 4096 bytes)".into());
    }
    let path_arg = args.get("path").and_then(|v| v.as_str());
    let glob_arg = args.get("glob").and_then(|v| v.as_str());

    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(args.get("case_insensitive").and_then(|v| v.as_bool()).unwrap_or(false))
        .multi_line(true)
        .size_limit(1 << 20)
        .dfa_size_limit(1 << 20)
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
        Some(g) => {
            if g.len() > 512 {
                return Err("glob too long (max 512 bytes)".into());
            }
            Some(glob_to_regex(g)?)
        }
        None => None,
    };

    let search_root = match path_arg {
        Some(p) => resolve_under_roots(p, roots)?,
        None => roots
            .first()
            .cloned()
            .ok_or("no workspace root configured")?,
    };
    // The walkdir filter below only prunes SKIP_DIRS at depth>0; guard the root
    // itself so grep(path:".git") can't read .git/config etc. at walk-depth 1.
    reject_skipped(&search_root)?;

    // Per-project include/exclude scoping — relative to the WORKSPACE root
    // (roots[0]), not the search subdir, so a project glob like `src/**` means
    // the same thing regardless of where grep starts walking.
    let filter = PathFilter::from_env();
    let ws_root = roots.first().cloned().unwrap_or_else(|| search_root.clone());

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
        // Per-project file-pattern scoping (skips quietly, like SKIP_DIRS).
        if !filter.allows_path(p, &ws_root) {
            continue;
        }
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
    // Frozen at first read: the env var is set once by the parent at spawn, and
    // re-reading per call would let any in-process env mutation escalate trust
    // mid-session without a restart.
    static LEVEL: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    LEVEL.get_or_init(|| match std::env::var("RIFT_TRUST_LEVEL").as_deref() {
        // CR-UX: legacy `full` (pre-collapse) maps to `standard`, not the floor.
        Ok("full") | Ok("standard") => "standard",
        _ => "readonly",
    })
}

fn trust_rank(level: &str) -> u8 {
    match level {
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

// ─── UI-bridge tools (ask_user / open_browser / notify) ────────────────────

/// Whether the loopback bridge to the parent Rift process is reachable from
/// this MCP-child spawn (env injected by `mod::write_mcp_config`).
fn bridge_enabled() -> bool {
    std::env::var("RIFT_BRIDGE_PORT").is_ok() && std::env::var("RIFT_BRIDGE_TOKEN").is_ok()
}

/// Socket timeout setters return Err on platforms / states where the option
/// can't be applied. Dropping that silently turns a misbehaving bridge socket
/// into an indefinite blocked read on the stdio thread (one MCP request stalls
/// every subsequent tool call) — log the breadcrumb.
fn apply_bridge_timeouts(stream: &TcpStream, read: Duration, write: Duration, label: &str) {
    if let Err(e) = stream.set_read_timeout(Some(read)) {
        log::warn!("{label}: set_read_timeout failed: {e}");
    }
    if let Err(e) = stream.set_write_timeout(Some(write)) {
        log::warn!("{label}: set_write_timeout failed: {e}");
    }
}

/// Single round-trip to the parent's loopback bridge: one NDJSON request line
/// out, one response line back. `extra` fields are merged over the base
/// `{op, token, session_id}` envelope. `read_timeout` is per-op — `ask_user`
/// parks for minutes while the user decides; the fire-and-forget ops use
/// seconds.
fn bridge_call(op: &str, extra: Value, read_timeout: Duration) -> Result<Value, String> {
    let port_s = std::env::var("RIFT_BRIDGE_PORT")
        .map_err(|_| "RIFT_BRIDGE_PORT not set on this MCP child".to_string())?;
    let token = std::env::var("RIFT_BRIDGE_TOKEN")
        .map_err(|_| "RIFT_BRIDGE_TOKEN not set on this MCP child".to_string())?;
    let port: u16 = port_s
        .parse()
        .map_err(|e| format!("invalid RIFT_BRIDGE_PORT `{port_s}`: {e}"))?;

    let mut req = json!({
        "op": op,
        "token": token,
        "session_id": std::env::var("RIFT_SESSION_ID").unwrap_or_default(),
        // #37: originating window so the bridge emits_to it, not app-wide.
        "window_label": std::env::var("RIFT_BRIDGE_WINDOW").unwrap_or_default(),
    });
    if let (Value::Object(base), Value::Object(extra)) = (&mut req, extra) {
        for (k, v) in extra {
            base.insert(k, v);
        }
    }

    let addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|e| format!("bridge addr parse: {e}"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))
        .map_err(|e| format!("bridge connect: {e}"))?;
    apply_bridge_timeouts(&stream, read_timeout, Duration::from_secs(5), op);

    let payload = format!("{}\n", req);
    stream
        .write_all(payload.as_bytes())
        .map_err(|e| format!("bridge write: {e}"))?;
    stream.flush().map_err(|e| format!("bridge flush: {e}"))?;

    // Cap the response read — a bridge response (ask_user answer, ack) is small;
    // a Take guard bounds the allocation if bridge.rs ever emits a huge line.
    use std::io::Read as _;
    let mut reader = io::BufReader::new((&stream).take(64 * 1024));
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
    Ok(resp.get("data").cloned().unwrap_or(Value::Null))
}

/// Interactive question to the user via the chat UI. The parent emits an event
/// to the frontend and holds the TCP connection open until the user clicks an
/// answer (or 10-min timeout). 11-min read timeout here — 1 min headroom over
/// the parent's 10-min await so the parent times out first with a clean error.
fn tool_ask_user(args: &Value) -> Result<String, String> {
    let questions = args
        .get("questions")
        .and_then(|v| v.as_array())
        .ok_or("missing `questions` array")?;
    if questions.is_empty() {
        return Err("`questions` must contain at least one question".into());
    }
    // RR10: bound the model-supplied payload before it crosses the loopback
    // bridge (the parent's read path buffers a whole line). The schema says
    // maxItems:4 but MCP does not enforce JSON Schema, so cap here.
    if questions.len() > 4 {
        return Err("too many questions (max 4)".into());
    }
    if serde_json::to_string(questions).map(|s| s.len()).unwrap_or(usize::MAX) > 16 * 1024 {
        return Err("questions payload too large (max 16 KiB)".into());
    }

    // 16 random bytes → 22-char base64url pending-request key. Collision
    // space is the in-flight set, never more than ~1 at a time per session.
    let mut id_bytes = [0u8; 16];
    rand::fill(&mut id_bytes);
    let request_id = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id_bytes);

    let data = bridge_call(
        "ask_user",
        json!({ "request_id": request_id, "questions": questions }),
        Duration::from_secs(660),
    )?;
    Ok(format_ask_user_result(&data))
}

/// Turn the answer envelope from the frontend into a plain-text tool_result
/// Claude can read. Three shapes:
///   1. `{cancelled: true}` — user dismissed.
///   2. `{answers: [{question, answer}, ...]}` — normal path.
///   3. Anything else — serialize verbatim as JSON.
fn format_ask_user_result(data: &Value) -> String {
    if data.get("cancelled").and_then(|v| v.as_bool()) == Some(true) {
        return "User dismissed the question without answering. Fall back to asking in plain text, or proceed with the most-likely-correct default and note your assumption.".into();
    }
    let Some(answers) = data.get("answers").and_then(|v| v.as_array()) else {
        return data.to_string();
    };
    if answers.is_empty() {
        return "User submitted an empty answer set.".into();
    }
    let mut out = String::new();
    for (i, a) in answers.iter().enumerate() {
        let q = a.get("question").and_then(|v| v.as_str()).unwrap_or("?");
        let ans_val = a.get("answer");
        let label = match ans_val {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(", "),
            Some(other) => other.to_string(),
            None => "(no answer)".into(),
        };
        if i > 0 {
            out.push('\n');
        }
        let _ = write!(out, "Q: {q}\nA: {label}");
    }
    out
}

/// Show an http/https URL in Rift's in-app browser dock (the embedded webview
/// next to the chat). The parent validates the scheme and the frontend opens
/// the dock — this call returns as soon as the parent accepted the request.
fn tool_open_browser(args: &Value) -> Result<String, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .ok_or("missing `url`")?;
    bridge_call("open_browser", json!({ "url": url }), Duration::from_secs(10))?;
    Ok(format!(
        "Opened {url} in Rift's in-app browser dock — the page is now visible to the user next to the chat."
    ))
}

/// Pop a toast notification in Rift's corner. Fire-and-forget presentation —
/// no user response comes back.
fn tool_notify(args: &Value) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or("missing `title`")?;
    // RR10: a toast is bounded UI — cap the model-supplied strings (char
    // boundary safe) before they cross the bridge into the parent heap.
    let title: String = title.chars().take(200).collect();
    let mut extra = json!({ "title": title });
    if let Some(d) = args.get("detail").and_then(|v| v.as_str()) {
        let detail: String = d.chars().take(2000).collect();
        extra["detail"] = Value::from(detail);
    }
    if let Some(s) = args.get("severity").and_then(|v| v.as_str()) {
        extra["severity"] = Value::from(s);
    }
    bridge_call("notify", extra, Duration::from_secs(10))?;
    Ok("Notification shown to the user.".into())
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
    // UI-bridge tools: only listed when the parent's loopback bridge env is
    // present (env-stripped MCP launchers degrade to the file/git set).
    if bridge_enabled() {
        tools.push(json!({
            "name": "ask_user",
            "description": "Ask the user a multiple-choice question and wait for their answer. Use this when you need to make a decision the user should weigh in on — picking between approaches, confirming a destructive action, clarifying an ambiguous request, narrowing scope. The standard Anthropic `AskUserQuestion` tool is unavailable in this environment; this is the Rift-native replacement and renders as an interactive card in the chat. Each question gets a list of options the user clicks to answer; set `multiSelect: true` if more than one option can apply. Keep questions short (≤120 chars) and labels concise (≤5 words). Returns the user's selections as the tool result; if they dismiss without picking, returns a `cancelled` marker so you can fall back to asking in plain text.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "description": "1-4 questions to present. Most calls have exactly one.",
                        "minItems": 1,
                        "maxItems": 4,
                        "items": {
                            "type": "object",
                            "properties": {
                                "question": { "type": "string", "description": "Full question text. Ends with '?'." },
                                "header": { "type": "string", "description": "Short chip-label shown above the question (≤12 chars). Examples: 'Library', 'Approach', 'Confirm'." },
                                "multiSelect": { "type": "boolean", "description": "True if multiple options can be selected. Defaults to false (single-select)." },
                                "options": {
                                    "type": "array",
                                    "description": "2-4 distinct, mutually-exclusive choices (unless multiSelect). An 'Other' freeform option is always added automatically — do not include it manually.",
                                    "minItems": 2,
                                    "maxItems": 4,
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "label": { "type": "string", "description": "Display text (1-5 words)." },
                                            "description": { "type": "string", "description": "Optional 1-line explanation of what this option means." }
                                        },
                                        "required": ["label"]
                                    }
                                }
                            },
                            "required": ["question", "header", "options"]
                        }
                    }
                },
                "required": ["questions"]
            }
        }));
        tools.push(json!({
            "name": "open_browser",
            "description": "Open an http/https URL in Rift's in-app browser dock — an embedded webview the user sees right next to the chat. ALWAYS use this instead of just printing a link when you start a dev server or want to show the user a local preview (e.g. http://localhost:3000), a docs page, or any page worth looking at together. The dock opens automatically if it's closed; the user keeps full control (address bar, back/forward, close).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "Absolute http:// or https:// URL. Other schemes are rejected." }
                },
                "required": ["url"]
            }
        }));
        tools.push(json!({
            "name": "notify",
            "description": "Show a brief toast notification in the corner of the Rift window. Use it to flag that long-running work finished, that something needs the user's attention, or that an important milestone landed — especially when the user may be looking at another part of the app. Fire-and-forget: no response comes back. Don't spam it — one toast per noteworthy event.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Short headline (≤200 chars)." },
                    "detail": { "type": "string", "description": "Optional one-line detail under the title (≤500 chars)." },
                    "severity": { "type": "string", "enum": ["info", "ok", "warn", "danger"], "description": "Visual tone. Defaults to info." }
                },
                "required": ["title"]
            }
        }));
    }
    json!({ "tools": tools })
}

fn handle_request(req: RpcRequest, roots: &[PathBuf]) -> Option<RpcResponse> {
    // Notifications (no id) get no response.
    let id = req.id?;

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
                "ask_user" if bridge_enabled() => tool_ask_user(&args),
                "open_browser" if bridge_enabled() => tool_open_browser(&args),
                "notify" if bridge_enabled() => tool_notify(&args),
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
    let mut buf = Vec::new();
    loop {
        buf.clear();
        // Bound the read at the source: read_line would allocate an arbitrarily
        // large line into memory BEFORE any length check. Cap the reader to
        // MAX_LINE_BYTES+1 per line so a runaway/hostile producer can't OOM the
        // child; an over-cap line is detected + skipped without buffering it
        // whole. (RR3)
        let read = io::Read::take(&mut reader, MAX_LINE_BYTES as u64 + 1)
            .read_until(b'\n', &mut buf);
        match read {
            Ok(0) => return,
            Ok(_) => {}
            Err(_) => return,
        }
        if buf.len() > MAX_LINE_BYTES {
            // Drain the rest of the over-cap line so the next iteration starts at
            // a real line boundary, not mid-record. The naive
            // `reader.read_until('\n', &mut sink)` would buffer the ENTIRE
            // remainder of a multi-GB no-newline line into `sink`, defeating the
            // MAX_LINE_BYTES cap above and OOMing the child. Drain in bounded
            // chunks and discard instead — constant memory regardless of line size.
            let mut skip = [0u8; 8192];
            loop {
                match io::Read::read(&mut reader, &mut skip) {
                    Ok(0) => return, // EOF before newline → stream closed
                    Ok(n) => {
                        if let Some(nl) = skip[..n].iter().position(|&b| b == b'\n') {
                            // Found the line boundary. Anything after it in this
                            // chunk belongs to the next record — but BufRead has
                            // no unread, and a producer interleaving a giant line
                            // with a valid record on the same chunk is not a real
                            // case (one JSON-RPC request per line). Resume at next
                            // read; the rare lost-tail is acceptable for a line we
                            // already rejected as oversized.
                            let _ = nl;
                            break;
                        }
                    }
                    Err(_) => return,
                }
            }
            continue;
        }
        let line = match std::str::from_utf8(&buf) {
            Ok(s) => s.trim(),
            Err(_) => continue,
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Temp workspace with a small fixture tree:
    ///   a.txt ("hello world\n"), sub/b.txt ("nested\n"),
    ///   node_modules/skip.txt ("hello\n"), bin (NUL byte + "hello").
    /// Returns the guard (drop = cleanup) and the canonicalized root — matching
    /// the form `load_roots` produces.
    fn workspace() -> (tempfile::TempDir, PathBuf) {
        let td = tempfile::tempdir().expect("tempdir");
        let root = std::fs::canonicalize(td.path()).expect("canonicalize");
        std::fs::write(root.join("a.txt"), "hello world\n").unwrap();
        std::fs::create_dir(root.join("sub")).unwrap();
        std::fs::write(root.join("sub").join("b.txt"), "nested\n").unwrap();
        std::fs::create_dir(root.join("node_modules")).unwrap();
        std::fs::write(root.join("node_modules").join("skip.txt"), "hello\n").unwrap();
        std::fs::write(root.join("bin"), b"\0\0hello").unwrap();
        (td, root)
    }

    // ─── resolve_under_roots — the path-containment boundary ──────────────────

    #[test]
    fn resolve_relative_and_absolute_under_root() {
        let (_td, root) = workspace();
        let roots = vec![root.clone()];
        assert!(resolve_under_roots("a.txt", &roots).is_ok());
        assert!(resolve_under_roots("sub/b.txt", &roots).is_ok());
        let abs = root.join("a.txt");
        assert!(resolve_under_roots(abs.to_str().unwrap(), &roots).is_ok());
    }

    #[test]
    fn resolve_rejects_outside_root() {
        let (_td, root) = workspace();
        let roots = vec![root.clone()];
        // Parent-dir escape: `..` resolves to the temp parent, which is not under
        // root — the canonicalize-then-starts_with guard must reject it.
        assert!(resolve_under_roots("..", &roots).is_err());
        // An absolute path in a *different* temp dir is outside every root.
        let (_other_td, other) = workspace();
        let outside = other.join("a.txt");
        assert!(resolve_under_roots(outside.to_str().unwrap(), &roots).is_err());
    }

    #[test]
    fn resolve_errors_on_empty_roots_and_missing_path() {
        let (_td, root) = workspace();
        assert!(resolve_under_roots("a.txt", &[]).is_err());
        // Non-existent path can't be canonicalized → error (never silently OK).
        assert!(resolve_under_roots("nope.txt", &[root]).is_err());
    }

    // ─── read_file ────────────────────────────────────────────────────────────

    #[test]
    fn read_file_returns_contents() {
        let (_td, root) = workspace();
        let out = tool_read_file(&json!({ "path": "a.txt" }), &[root]).unwrap();
        assert_eq!(out, "hello world\n");
    }

    #[test]
    fn read_file_rejects_directory_and_oversize() {
        let (_td, root) = workspace();
        let roots = vec![root.clone()];
        assert!(tool_read_file(&json!({ "path": "sub" }), &roots).unwrap_err().contains("not a regular file"));
        // > 500 KB → rejected with a size message.
        std::fs::write(root.join("big.bin"), vec![b'x'; (MAX_READ_BYTES as usize) + 10]).unwrap();
        let err = tool_read_file(&json!({ "path": "big.bin" }), &roots).unwrap_err();
        assert!(err.contains("limit"), "got: {err}");
    }

    #[test]
    fn read_file_rejects_outside_root() {
        let (_td, root) = workspace();
        assert!(tool_read_file(&json!({ "path": ".." }), &[root]).is_err());
    }

    // ─── list_dir ─────────────────────────────────────────────────────────────

    #[test]
    fn list_dir_lists_entries() {
        let (_td, root) = workspace();
        let out = tool_list_dir(&json!({ "path": "." }), &[root]).unwrap();
        assert!(out.contains("a.txt"), "got: {out}");
        assert!(out.contains("sub/"), "dirs should carry a trailing slash: {out}");
    }

    #[test]
    fn list_dir_rejects_file() {
        let (_td, root) = workspace();
        let err = tool_list_dir(&json!({ "path": "a.txt" }), &[root]).unwrap_err();
        assert!(err.contains("not a directory"), "got: {err}");
    }

    // ─── grep ─────────────────────────────────────────────────────────────────

    #[test]
    fn grep_finds_match_and_reports_path() {
        let (_td, root) = workspace();
        let out = tool_grep(&json!({ "pattern": "hello world" }), &[root]).unwrap();
        assert!(out.contains("a.txt"), "got: {out}");
    }

    #[test]
    fn grep_skips_skip_dirs_and_binary() {
        let (_td, root) = workspace();
        // "hello" lives in a.txt, node_modules/skip.txt, and the binary `bin`.
        // Only a.txt should match: node_modules is skipped, bin is NUL-probed out.
        let out = tool_grep(&json!({ "pattern": "hello" }), &[root]).unwrap();
        assert!(out.contains("a.txt"), "got: {out}");
        assert!(!out.contains("node_modules"), "SKIP_DIRS leaked: {out}");
        assert!(!out.contains("bin:"), "binary file matched: {out}");
    }

    #[test]
    fn grep_glob_filters_by_extension() {
        let (_td, root) = workspace();
        let roots = vec![root];
        // `*.txt` keeps a.txt; `*.rs` matches nothing in the fixture.
        assert!(tool_grep(&json!({ "pattern": "hello", "glob": "*.txt" }), &roots).unwrap().contains("a.txt"));
        let none = tool_grep(&json!({ "pattern": "hello", "glob": "*.rs" }), &roots).unwrap();
        assert!(none.contains("no matches"), "got: {none}");
    }

    #[test]
    fn grep_rejects_bad_regex_and_long_pattern() {
        let (_td, root) = workspace();
        let roots = vec![root];
        assert!(tool_grep(&json!({ "pattern": "(" }), &roots).is_err());
        let long = "a".repeat(4097);
        assert!(tool_grep(&json!({ "pattern": long }), &roots).unwrap_err().contains("too long"));
    }

    #[test]
    fn grep_rejects_oversize_glob() {
        // The glob length cap (512) is a memory guard on the glob→regex compile.
        let (_td, root) = workspace();
        let big_glob = format!("{}/*.rs", "a".repeat(520));
        let err = tool_grep(&json!({ "pattern": "x", "glob": big_glob }), &[root]).unwrap_err();
        assert!(err.contains("glob too long"), "got: {err}");
    }

    #[test]
    fn grep_rejects_skip_dir_passed_directly_as_path() {
        // The walkdir filter only prunes SKIP_DIRS at depth>0; reject_skipped on
        // the search_root itself stops grep(path:"node_modules") from reading an
        // excluded tree wholesale. Without it, .git/config etc. would be greppable.
        let (_td, root) = workspace();
        let err = tool_grep(&json!({ "pattern": "hello", "path": "node_modules" }), &[root]).unwrap_err();
        assert!(err.contains("excluded directory"), "got: {err}");
    }

    // ─── reject_skipped — the excluded-tree boundary (read/list, not just grep) ──

    #[test]
    fn read_and_list_reject_files_inside_skip_dirs() {
        // A direct read/list INTO an excluded dir must be refused even though the
        // path is technically under the workspace root — node_modules/.git/target
        // are never legitimate read targets and can hide secrets/large blobs.
        let (_td, root) = workspace();
        let roots = vec![root];
        let read_err = tool_read_file(&json!({ "path": "node_modules/skip.txt" }), &roots).unwrap_err();
        assert!(read_err.contains("excluded directory"), "read got: {read_err}");
        let list_err = tool_list_dir(&json!({ "path": "node_modules" }), &roots).unwrap_err();
        assert!(list_err.contains("excluded directory"), "list got: {list_err}");
    }

    // ─── symlink escape — canonicalize must follow the link OUT of root ─────────

    #[cfg(unix)]
    #[test]
    fn resolve_rejects_symlink_escaping_root() {
        // A symlink living inside the workspace but pointing at a sibling outside
        // it: canonicalize resolves to the real (outside) target, so the
        // starts_with(root) check rejects it. This is THE classic sandbox escape.
        let (_td, root) = workspace();
        let (_outside_td, outside) = workspace(); // a separate temp tree
        let secret = outside.join("a.txt");
        let link = root.join("escape");
        std::os::unix::fs::symlink(&secret, &link).unwrap();
        let roots = vec![root];
        // Resolving the symlink path must land outside → rejected.
        assert!(resolve_under_roots("escape", &roots).is_err());
        assert!(tool_read_file(&json!({ "path": "escape" }), &roots).is_err());
    }

    // ─── glob_to_regex (pure) ─────────────────────────────────────────────────

    #[test]
    fn glob_compiles_expected_semantics() {
        let star = glob_to_regex("*.rs").unwrap();
        assert!(star.is_match("foo.rs"));
        assert!(!star.is_match("foo.txt"));
        assert!(!star.is_match("a/foo.rs"), "single * must not cross '/'");
        let double = glob_to_regex("src/**/*.svelte").unwrap();
        assert!(double.is_match("src/lib/x.svelte"), "** must cross '/'");
        let q = glob_to_regex("?.rs").unwrap();
        assert!(q.is_match("a.rs"));
        assert!(!q.is_match("ab.rs"));
        // `.` is a literal, not a wildcard.
        let dot = glob_to_regex("a.txt").unwrap();
        assert!(!dot.is_match("axtxt"), "'.' must be escaped to a literal");
    }

    // ─── PathFilter (per-project include/exclude scoping) ─────────────────────

    #[test]
    fn pathfilter_empty_allows_everything() {
        let f = PathFilter::from_globs("", "");
        assert!(f.is_empty());
        assert!(f.allows("src/main.rs"));
        assert!(f.allows("anything/at/all.bin"));
    }

    #[test]
    fn pathfilter_exclude_hides_matching() {
        let f = PathFilter::from_globs("", "*.log\nvendor/**");
        assert!(f.allows("src/main.rs"));
        assert!(!f.allows("app.log"), "filename-only glob matches basename anywhere");
        assert!(!f.allows("logs/app.log"));
        assert!(!f.allows("vendor/dep/x.js"), "relpath glob matches under dir");
        // exclude hides the dir itself too
        assert!(!f.allows_dir(std::path::Path::new("/ws/vendor"), std::path::Path::new("/ws")));
        assert!(f.allows_dir(std::path::Path::new("/ws/src"), std::path::Path::new("/ws")));
    }

    #[test]
    fn pathfilter_include_keeps_only_matching_files_but_dirs_stay() {
        let f = PathFilter::from_globs("src/**\n*.md", "");
        assert!(f.allows("src/lib/x.rs"));
        assert!(f.allows("README.md"), "*.md basename match");
        assert!(!f.allows("scripts/build.sh"), "outside include set → hidden");
        // a non-excluded directory stays navigable even with includes set
        assert!(f.allows_dir(std::path::Path::new("/ws/scripts"), std::path::Path::new("/ws")));
    }

    #[test]
    fn pathfilter_exclude_wins_over_include() {
        let f = PathFilter::from_globs("src/**", "src/**/*.test.rs");
        assert!(f.allows("src/main.rs"));
        assert!(!f.allows("src/foo.test.rs"), "exclude takes precedence over include");
    }

    // ─── trust_rank (pure, security-relevant ordering) ────────────────────────

    #[test]
    fn trust_rank_orders_and_floors() {
        // CR-UX: "full" is no longer a rank of its own — trust_level() maps the
        // legacy env value to "standard" before ranking; raw "full" floors.
        assert_eq!(trust_rank("full"), trust_rank("readonly"));
        assert!(trust_rank("standard") > trust_rank("readonly"));
        // Unknown / unset must floor to readonly (0), never escalate.
        assert_eq!(trust_rank("garbage"), trust_rank("readonly"));
        assert_eq!(trust_rank("readonly"), 0);
    }
}
