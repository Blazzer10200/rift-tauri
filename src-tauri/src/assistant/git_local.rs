//! Local git operations exposed to the Assistant via the MCP server.
//!
//! These tools run *inside the MCP child* (the `RIFT_MCP_SERVER=1` process),
//! same as `tool_read_file` / `tool_grep` — git is a LOCAL operation against
//! the user's workspace, so no loopback bridge is involved. The working
//! directory is always the first workspace root (`RIFT_MCP_ROOTS[0]`).
//!
//! Trust gating (see `mcp_server::trust_level`):
//!   * read-only  → `git_status`, `git_diff`, `git_log`
//!   * standard+  → adds `git_pull`, `git_commit`, `git_push`
//!
//! Security model (design brief §11 — written after the Jan-2026 MCP-git
//! argument-injection CVEs): the trust setting decides *whether* a tool runs;
//! every string param is still validated on *every* call before it reaches
//! `Command`. The posture is "trust the trust setting, distrust the params."
//!   * No shell — always `Command::new("git").args([...])`, never `sh -c`.
//!   * Refs / remotes / branches: strict `^[A-Za-z0-9._/-]{1,200}$`, no
//!     leading `-` (blocks `--upload-pack=` style flag injection).
//!   * Paths: no `..`, no NUL, no leading `-`, must stay lexically under the
//!     workspace root, always passed after a `--` separator.
//!   * Commit messages: ≤4 KiB, no embedded NUL.
//!   * `force` / `-f` is never emitted; a `force: true` arg is rejected.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Max bytes of a diff we hand back to the model before truncating.
const MAX_DIFF_BYTES: usize = 64 * 1024;
/// Hard ceiling on `git log -n`.
const MAX_LOG_COUNT: u64 = 100;
/// Commit-message cap.
const MAX_MSG_BYTES: usize = 4 * 1024;

// ─── validation ─────────────────────────────────────────────────────────────

/// Validate a ref / branch / remote name. Strict allowlist; rejects anything
/// that could be parsed as a git flag.
fn validate_ref(kind: &str, s: &str) -> Result<String, String> {
    if s.is_empty() || s.len() > 200 {
        return Err(format!("invalid {kind}: must be 1-200 chars"));
    }
    if s.starts_with('-') {
        return Err(format!("invalid {kind}: must not start with '-'"));
    }
    if !s.bytes().all(|b| {
        b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'/' | b'-')
    }) {
        return Err(format!(
            "invalid {kind} `{s}`: only letters, digits, and . _ / - are allowed"
        ));
    }
    Ok(s.to_string())
}

/// Validate a workspace-relative path param. Rejects traversal, NUL, and
/// leading-dash flag injection; confirms the joined path stays under `root`.
/// Does NOT require the path to exist (a freshly-created file being `git add`ed
/// may have just appeared on disk; existence is git's job to report).
fn validate_path(root: &Path, raw: &str) -> Result<String, String> {
    if raw.is_empty() || raw.len() > 1024 {
        return Err("invalid path: must be 1-1024 chars".into());
    }
    if raw.contains('\0') {
        return Err("invalid path: contains NUL".into());
    }
    if raw.starts_with('-') {
        return Err(format!("invalid path `{raw}`: must not start with '-'"));
    }
    let p = PathBuf::from(raw);
    // `..` is rejected for BOTH absolute and relative paths. `starts_with(root)`
    // is purely lexical, so an absolute `<root>/../../etc/passwd` would otherwise
    // pass the prefix check yet resolve outside the workspace.
    if p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(format!("path `{raw}` escapes the workspace root (`..` not allowed)"));
    }
    if p.is_absolute() {
        // Absolute paths must already live under the root.
        if !p.starts_with(root) {
            return Err(format!("path `{raw}` is outside the workspace root"));
        }
    } else if p.components().any(|c| matches!(c, std::path::Component::Prefix(_))) {
        // Windows drive-relative (`C:foo`) and UNC (`\\server\share`) paths
        // report is_absolute()==false yet carry a Prefix component, so they'd
        // otherwise slip past the checks and git would resolve them relative to
        // the process CWD on that drive — outside the workspace.
        return Err(format!("path `{raw}` must be workspace-relative (no drive or UNC prefix)"));
    }
    // Symlink guard: if the path already exists, canonicalize and confirm it
    // remains under root. Non-existent paths (new files) skip this block.
    let joined = root.join(&p);
    if joined.symlink_metadata().is_ok() {
        match std::fs::canonicalize(&joined) {
            Ok(real) if real.starts_with(root) => {}
            Ok(_) => return Err(format!("path `{raw}` resolves outside the workspace root (symlink)")),
            Err(e) => return Err(format!("path `{raw}`: canonicalize failed: {e}")),
        }
    }
    Ok(raw.to_string())
}

fn validate_message(s: &str) -> Result<String, String> {
    if s.trim().is_empty() {
        return Err("commit message is empty".into());
    }
    if s.len() > MAX_MSG_BYTES {
        return Err(format!("commit message too long ({} bytes; max {MAX_MSG_BYTES})", s.len()));
    }
    if s.contains('\0') {
        return Err("commit message contains NUL".into());
    }
    Ok(s.to_string())
}

// ─── git invocation ─────────────────────────────────────────────────────────

fn workspace_root(roots: &[PathBuf]) -> Result<&PathBuf, String> {
    roots
        .first()
        .ok_or_else(|| "no workspace root configured (open a folder in Rift first)".to_string())
}

/// Output of a single git invocation.
struct GitOut {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

impl GitOut {
    fn ok(&self) -> bool {
        self.code == Some(0)
    }
    /// stderr (or stdout) trimmed for surfacing as an error message.
    fn err_text(&self) -> String {
        let s = if !self.stderr.trim().is_empty() { &self.stderr } else { &self.stdout };
        let s = s.trim();
        if s.is_empty() {
            format!("git exited with code {}", self.code.map(|c| c.to_string()).unwrap_or_else(|| "?".into()))
        } else {
            s.to_string()
        }
    }
}

/// Run `git <args>` in `root` with the hardened env from brief §11. `args` are
/// pre-split (no shell). Never includes `force`/`-f`.
fn run_git(root: &Path, args: &[&str]) -> Result<GitOut, String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(root)
        .args(args)
        // Detach git's stdin from the MCP server's stdin pipe — a git subprocess
        // that reads stdin (credential helper, hook) must never drain bytes out
        // of the JSON-RPC request stream. `output()` already closes stdin, but
        // be explicit so a future switch to `spawn()` keeps the guarantee.
        .stdin(std::process::Stdio::null())
        // Fail fast instead of blocking on a credential prompt; suppress GUI
        // credential-manager popups; strip all git-override and layout env vars
        // that could cause git to operate outside the workspace. PATH / HOME /
        // SSH_AUTH_SOCK / GIT_SSH are intentionally preserved — Windows
        // credential helpers are shell scripts that need `sh.exe` on PATH.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_ASKPASS", "")
        // Force non-interactive SSH and suppress pager output.
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=yes")
        .env("GIT_PAGER", "cat")
        // Strip git layout/namespace overrides that could redirect object storage
        // or index outside the workspace root.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_NAMESPACE")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_EXEC_PATH")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "git is not installed or not on PATH. Install Git for Windows (or your OS package) and relaunch Rift from a shell that has it.".to_string()
        } else {
            format!("failed to run git: {e}")
        }
    })?;
    let pid = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    let out = match rx.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(result) => result.map_err(|e| format!("failed to run git: {e}"))?,
        Err(_) => {
            // Kill the whole process tree; ignore result — already timed out.
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .creation_flags(0x08000000)
                    .status();
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("kill")
                    .args(["-9", &pid.to_string()])
                    .status();
            }
            return Err("git timed out after 30s and was terminated".into());
        }
    };
    Ok(GitOut {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code(),
    })
}

/// Current branch name, or "HEAD" (detached) — used as the default push target.
fn current_branch(root: &Path) -> Result<String, String> {
    let out = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if !out.ok() {
        return Err(format!("not a git repository (or no commits yet): {}", out.err_text()));
    }
    // Re-validate before this value is reused as a `git push` argument: a branch
    // name containing odd bytes (or a leading `-`) must never reach the command
    // line. "HEAD" (detached) passes the allowlist and is handled by the caller.
    validate_ref("branch", out.stdout.trim())
}

/// True if the working tree has uncommitted changes (staged or unstaged).
fn is_dirty(root: &Path) -> Result<bool, String> {
    let out = run_git(root, &["status", "--porcelain"])?;
    if !out.ok() {
        return Err(out.err_text());
    }
    Ok(!out.stdout.trim().is_empty())
}

// ─── tools ──────────────────────────────────────────────────────────────────

pub fn tool_git_status(_args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let out = run_git(root, &["status", "--porcelain=v1", "-b"])?;
    if !out.ok() {
        return Err(format!("git status failed: {}", out.err_text()));
    }
    let lines: Vec<&str> = out.stdout.lines().collect();
    if lines.is_empty() {
        return Ok("Working tree clean.".into());
    }
    let mut branch = String::new();
    let mut changes: Vec<String> = Vec::new();
    for l in &lines {
        if let Some(rest) = l.strip_prefix("## ") {
            branch = rest.to_string();
        } else {
            changes.push((*l).to_string());
        }
    }
    let mut s = String::new();
    if !branch.is_empty() {
        s.push_str(&format!("Branch: {branch}\n"));
    }
    if changes.is_empty() {
        s.push_str("Working tree clean.");
    } else {
        s.push_str(&format!("{} changed file(s):\n", changes.len()));
        for c in changes.iter().take(200) {
            s.push_str(&format!("  {c}\n"));
        }
        if changes.len() > 200 {
            s.push_str(&format!("  … +{} more\n", changes.len() - 200));
        }
    }
    Ok(s)
}

pub fn tool_git_diff(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let cached = args.get("cached").and_then(|v| v.as_bool()).unwrap_or(false);
    let mut git_args: Vec<String> = vec!["diff".into()];
    if cached {
        git_args.push("--cached".into());
    }
    git_args.push("--".into());
    if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
        git_args.push(validate_path(root, path)?);
    }
    let arg_refs: Vec<&str> = git_args.iter().map(|s| s.as_str()).collect();
    let out = run_git(root, &arg_refs)?;
    if !out.ok() {
        return Err(format!("git diff failed: {}", out.err_text()));
    }
    if out.stdout.trim().is_empty() {
        return Ok(if cached { "No staged changes.".into() } else { "No unstaged changes.".into() });
    }
    if out.stdout.len() > MAX_DIFF_BYTES {
        // Back off to the last UTF-8 char boundary at or before the limit. The
        // naive `&out.stdout[..MAX_DIFF_BYTES]` would panic if that byte lands
        // inside a multibyte codepoint (from_utf8_lossy guarantees valid UTF-8,
        // not that index 65536 is a boundary).
        let mut end = MAX_DIFF_BYTES;
        while end > 0 && !out.stdout.is_char_boundary(end) {
            end -= 1;
        }
        Ok(format!("{}\n… (diff truncated at {} KB)", &out.stdout[..end], MAX_DIFF_BYTES / 1024))
    } else {
        Ok(out.stdout)
    }
}

pub fn tool_git_log(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let max = args.get("max").and_then(|v| v.as_u64()).unwrap_or(20);
    if max == 0 {
        return Err("max must be ≥1".into());
    }
    let n = max.min(MAX_LOG_COUNT).to_string();
    let out = run_git(root, &["log", "--oneline", "-n", &n])?;
    if !out.ok() {
        return Err(format!("git log failed: {}", out.err_text()));
    }
    if out.stdout.trim().is_empty() {
        return Ok("No commits.".into());
    }
    Ok(out.stdout)
}

pub fn tool_git_pull(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    if is_dirty(root)? {
        return Err("working tree is dirty — pull refused to avoid clobbering local changes. Stash first: `git stash` (then `git stash pop` after the pull), or commit your work.".into());
    }
    let rebase = args.get("rebase").and_then(|v| v.as_bool()).unwrap_or(false);
    let strategy = if rebase { "--rebase" } else { "--ff-only" };
    let out = run_git(root, &["pull", strategy])?;
    if !out.ok() {
        return Err(format!("git pull failed: {}", out.err_text()));
    }
    let mut s = out.stdout.trim().to_string();
    if !out.stderr.trim().is_empty() {
        if !s.is_empty() { s.push('\n'); }
        s.push_str(out.stderr.trim());
    }
    if s.is_empty() { s = "Already up to date.".into(); }
    Ok(s)
}

pub fn tool_git_commit(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or("missing `message`")?;
    let message = validate_message(message)?;

    // Staging: explicit paths win; else `all: true` stages everything; else
    // commit only what's already staged.
    if let Some(paths) = args.get("paths").and_then(|v| v.as_array()) {
        let mut add_args: Vec<String> = vec!["add".into(), "--".into()];
        for p in paths {
            let s = p.as_str().ok_or("`paths` entries must be strings")?;
            add_args.push(validate_path(root, s)?);
        }
        let refs: Vec<&str> = add_args.iter().map(|s| s.as_str()).collect();
        let add = run_git(root, &refs)?;
        if !add.ok() {
            return Err(format!("git add failed: {}", add.err_text()));
        }
    } else if args.get("all").and_then(|v| v.as_bool()).unwrap_or(false) {
        let add = run_git(root, &["add", "-A"])?;
        if !add.ok() {
            return Err(format!("git add -A failed: {}", add.err_text()));
        }
    }

    let out = run_git(root, &["commit", "-m", &message])?;
    if !out.ok() {
        let txt = out.err_text();
        // "nothing to commit" is git's normal message on an empty index.
        if txt.contains("nothing to commit") || out.stdout.contains("nothing to commit") {
            return Err("nothing to commit — stage changes first (pass `paths` or `all: true`).".into());
        }
        return Err(format!("git commit failed: {txt}"));
    }
    Ok(out.stdout.trim().to_string())
}

pub fn tool_git_push(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    // Refuse force unconditionally — never emit `-f`/`--force`.
    if args.get("force").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Err("force push is not permitted by this tool.".into());
    }
    let remote = match args.get("remote").and_then(|v| v.as_str()) {
        Some(r) => validate_ref("remote", r)?,
        None => "origin".into(),
    };
    let branch = match args.get("branch").and_then(|v| v.as_str()) {
        Some(b) => validate_ref("branch", b)?,
        None => current_branch(root)?,
    };
    if branch == "HEAD" {
        return Err("detached HEAD — checkout a branch before pushing.".into());
    }

    let out = run_git(root, &["push", &remote, &branch])?;
    if !out.ok() {
        return Err(format!("git push failed: {}", out.err_text()));
    }
    let mut s = out.stdout.trim().to_string();
    if !out.stderr.trim().is_empty() {
        if !s.is_empty() { s.push('\n'); }
        s.push_str(out.stderr.trim());
    }
    if s.is_empty() { s = format!("Pushed to {remote}/{branch}."); }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_flag_injection_refs() {
        assert!(validate_ref("branch", "--upload-pack=evil").is_err());
        assert!(validate_ref("remote", "-x").is_err());
        assert!(validate_ref("branch", "main; rm -rf ~").is_err());
        assert!(validate_ref("branch", "feature/x_y-1.2").is_ok());
        assert!(validate_ref("branch", "main").is_ok());
    }

    #[test]
    fn rejects_path_traversal() {
        let root = PathBuf::from(if cfg!(windows) { r"C:\ws" } else { "/ws" });
        assert!(validate_path(&root, "../../etc/passwd").is_err());
        assert!(validate_path(&root, "-rf").is_err());
        assert!(validate_path(&root, "src/foo.rs").is_ok());
        assert!(validate_path(&root, "a/b/c.lua").is_ok());
        // Absolute path under root but with `..` must NOT pass the lexical
        // starts_with guard (F13/F50 — would resolve to /etc/passwd).
        let escape = if cfg!(windows) { r"C:\ws\..\Windows\x" } else { "/ws/../etc/passwd" };
        assert!(validate_path(&root, escape).is_err());
        // Windows drive-relative / UNC / cross-drive paths report
        // is_absolute()==false or fall outside root — all must be rejected.
        #[cfg(windows)]
        {
            assert!(validate_path(&root, "C:evil").is_err());
            assert!(validate_path(&root, r"\\server\share\x").is_err());
            assert!(validate_path(&root, r"D:\other\x").is_err());
        }
    }

    #[test]
    fn message_bounds() {
        assert!(validate_message("").is_err());
        assert!(validate_message("   ").is_err());
        assert!(validate_message("fix: thing").is_ok());
        let big = "x".repeat(MAX_MSG_BYTES + 1);
        assert!(validate_message(&big).is_err());
    }

    // ─── integration: real `git` against a throwaway repo ─────────────────────
    //
    // These exercise the tool entrypoints end-to-end (run_git, output parsing,
    // staging logic, dirty/branch helpers, and the security gates) — the surface
    // the validator unit tests above can't reach. All offline; each test owns its
    // own temp repo. Skipped if `git` isn't on PATH so the suite stays green on a
    // machine without it.

    use serde_json::json;

    fn git_available() -> bool {
        std::process::Command::new("git")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Init a temp repo with one commit (`README.md`). Returns the TempDir guard
    /// (drop = cleanup, must stay in scope) and the canonicalized root — matching
    /// the form `validate_path`'s canonicalize check produces on Windows.
    fn init_repo() -> (tempfile::TempDir, PathBuf) {
        let td = tempfile::tempdir().expect("tempdir");
        let root = std::fs::canonicalize(td.path()).expect("canonicalize");
        run_git(&root, &["init", "-q"]).expect("git init");
        // Local identity + no signing so commits succeed regardless of the host's
        // global git config (a machine that forces gpgsign would otherwise fail).
        run_git(&root, &["config", "user.email", "test@rift.local"]).unwrap();
        run_git(&root, &["config", "user.name", "Rift Test"]).unwrap();
        run_git(&root, &["config", "commit.gpgsign", "false"]).unwrap();
        std::fs::write(root.join("README.md"), "hello\n").unwrap();
        run_git(&root, &["add", "-A"]).unwrap();
        run_git(&root, &["commit", "-q", "-m", "init"]).unwrap();
        (td, root)
    }

    #[test]
    fn status_reports_clean_then_dirty() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        let roots = vec![root.clone()];
        let clean = tool_git_status(&json!({}), &roots).unwrap();
        assert!(clean.contains("clean"), "expected clean tree, got: {clean}");
        std::fs::write(root.join("new.txt"), "x").unwrap();
        let dirty = tool_git_status(&json!({}), &roots).unwrap();
        assert!(dirty.contains("changed file"), "expected change, got: {dirty}");
        assert!(dirty.contains("new.txt"), "expected new.txt, got: {dirty}");
    }

    #[test]
    fn log_lists_the_initial_commit() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        let out = tool_git_log(&json!({}), &[root]).unwrap();
        assert!(out.contains("init"), "log missing initial commit: {out}");
    }

    #[test]
    fn log_max_zero_rejected() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        let err = tool_git_log(&json!({ "max": 0 }), &[root]).unwrap_err();
        assert!(err.contains("max must be"), "got: {err}");
    }

    #[test]
    fn commit_all_stages_everything() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        let roots = vec![root.clone()];
        std::fs::write(root.join("f2.txt"), "two\n").unwrap();
        let out = tool_git_commit(&json!({ "all": true, "message": "add f2" }), &roots).unwrap();
        assert!(!out.is_empty());
        assert!(!is_dirty(&root).unwrap(), "tree should be clean after commit -A");
        let log = tool_git_log(&json!({}), &roots).unwrap();
        assert!(log.contains("add f2"), "log missing new commit: {log}");
    }

    #[test]
    fn commit_explicit_paths_leaves_others_unstaged() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        let roots = vec![root.clone()];
        std::fs::write(root.join("a.txt"), "a\n").unwrap();
        std::fs::write(root.join("b.txt"), "b\n").unwrap();
        // Commit only a.txt — exercises validate_path on a real (existing) file.
        tool_git_commit(&json!({ "paths": ["a.txt"], "message": "add a" }), &roots).unwrap();
        // b.txt is still untracked → tree dirty.
        assert!(is_dirty(&root).unwrap(), "b.txt should remain unstaged");
        let status = tool_git_status(&json!({}), &roots).unwrap();
        assert!(status.contains("b.txt"), "b.txt should still show: {status}");
    }

    #[test]
    fn commit_with_nothing_staged_errors() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        let err = tool_git_commit(&json!({ "all": true, "message": "noop" }), &[root]).unwrap_err();
        assert!(err.contains("nothing to commit"), "got: {err}");
    }

    #[test]
    fn diff_shows_unstaged_then_nothing() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        let roots = vec![root.clone()];
        let none = tool_git_diff(&json!({}), &roots).unwrap();
        assert!(none.contains("No unstaged"), "expected no diff, got: {none}");
        std::fs::write(root.join("README.md"), "hello\nworld\n").unwrap();
        let diff = tool_git_diff(&json!({}), &roots).unwrap();
        assert!(diff.contains("+world"), "diff missing addition: {diff}");
    }

    #[test]
    fn pull_refuses_dirty_tree() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        std::fs::write(root.join("dirty.txt"), "x").unwrap();
        let err = tool_git_pull(&json!({}), &[root]).unwrap_err();
        assert!(err.contains("dirty"), "expected dirty refusal, got: {err}");
    }

    #[test]
    fn push_force_is_refused() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        let err = tool_git_push(&json!({ "force": true }), &[root]).unwrap_err();
        assert!(err.contains("force push is not permitted"), "got: {err}");
    }

    #[test]
    fn push_without_remote_fails_cleanly() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        // No `origin` configured → push fails fast (GIT_TERMINAL_PROMPT=0, no
        // network hang). Exercises current_branch() + the push arg path.
        let err = tool_git_push(&json!({}), &[root]).unwrap_err();
        assert!(err.contains("git push failed"), "got: {err}");
    }

    #[test]
    fn current_branch_is_valid() {
        if !git_available() { return; }
        let (_td, root) = init_repo();
        let b = current_branch(&root).unwrap();
        assert!(!b.is_empty() && b != "HEAD", "unexpected branch: {b}");
    }

    #[test]
    fn no_workspace_root_errors() {
        let err = tool_git_status(&json!({}), &[]).unwrap_err();
        assert!(err.contains("no workspace root"), "got: {err}");
    }
}
