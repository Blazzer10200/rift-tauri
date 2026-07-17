//! GitHub remote tools, riding the user's own `gh` CLI.
//!
//! Same execution posture as `git_local.rs` (no shell, pre-split args, pinned
//! env, 30s timeout + image-scoped kill, capped output), plus two GitHub-
//! specific rules:
//!   * **Tokenless** — Rift never reads or stores a credential. Every call
//!     shells to the user's `gh`, so auth/scopes are exactly what the user set
//!     up with `gh auth login`. No gh / not logged in → tools degrade to a
//!     clear error, the UI chip degrades to a plain branch label.
//!   * **Repo is always pinned** — `-R owner/repo` is derived from the
//!     workspace's `origin` remote on every call. Neither the model nor the
//!     UI can point these tools at an arbitrary repository.
//!
//! Trust gating (mirrors git tools): read set (`gh_checks`, `gh_run_view`,
//! `gh_pr_list`, `gh_pr_view`, `gh_pr_diff`) is always available; the single
//! write tool `gh_pr_create` needs `standard` trust like `git_push`.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

use super::git_local::{
    current_branch, drain_child_capped, run_git, truncate_bytes, validate_ref, workspace_root,
};

/// Cap on JSON/diff payloads returned to the model.
const MAX_GH_BYTES: usize = 64 * 1024;
/// PR title: single line, GitHub caps at 256 — stay under it.
const MAX_TITLE_CHARS: usize = 250;
/// PR body cap. Windows command lines top out ~32 KB; 8 KB of body plus args
/// leaves comfortable head-room.
const MAX_BODY_BYTES: usize = 8 * 1024;

// ─── gh invocation ──────────────────────────────────────────────────────────

struct GhOut {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

impl GhOut {
    fn ok(&self) -> bool {
        self.code == Some(0)
    }
    fn err_text(&self) -> String {
        let s = if !self.stderr.trim().is_empty() { &self.stderr } else { &self.stdout };
        let s = s.trim();
        if s.is_empty() {
            format!("gh exited with code {}", self.code.map(|c| c.to_string()).unwrap_or_else(|| "?".into()))
        } else {
            truncate_bytes(s, 8 * 1024).into_owned()
        }
    }
}

const GH_MISSING: &str = "GitHub CLI (gh) is not installed or not on PATH. Install it from https://cli.github.com and sign in with `gh auth login` to enable GitHub features.";

/// Run `gh <args>` in `root`. Mirrors `git_local::run_git` hardening: no
/// shell, stdin detached, non-interactive env, 30s timeout with an
/// image-scoped kill, output drained through the capped reader.
fn run_gh(root: &Path, args: &[&str]) -> Result<GhOut, String> {
    let mut cmd = Command::new("gh");
    cmd.current_dir(root)
        .args(args)
        .stdin(std::process::Stdio::null())
        // Never prompt, never self-update-nag, never color/page output.
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0")
        .env("GH_PAGER", "cat")
        .env("PAGER", "cat")
        // Strip repo/host redirection + debug spew: every call passes an
        // explicit `-R owner/repo` derived from origin, and GH_HOST/GH_REPO
        // env could silently retarget it. GH_DEBUG dumps API traffic (incl.
        // request headers) to stderr, which flows into model-facing errors.
        .env_remove("GH_REPO")
        .env_remove("GH_HOST")
        .env_remove("GH_DEBUG")
        .env_remove("GH_FORCE_TTY")
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
            GH_MISSING.to_string()
        } else {
            format!("failed to run gh: {e}")
        }
    })?;
    let pid = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(drain_child_capped(child));
    });
    let out = match rx.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(result) => result.map_err(|e| format!("failed to run gh: {e}"))?,
        Err(_) => {
            // Image-scoped kill (same PID-recycle guard as run_git).
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/FI", "IMAGENAME eq gh.exe", "/T", "/F"])
                    .creation_flags(0x08000000)
                    .status();
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).status();
            }
            return Err("gh timed out after 30s and was terminated".into());
        }
    };
    Ok(GhOut {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.code,
    })
}

// ─── repo pinning ───────────────────────────────────────────────────────────

/// Parse a github.com remote URL into `owner/repo`. Whitelisted prefixes only
/// (no substring matching — `https://evil.com/github.com/x/y` must not parse).
pub(crate) fn parse_github_repo(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = url.strip_prefix("git@github.com:").or_else(|| {
        ["https://github.com/", "http://github.com/", "ssh://git@github.com/", "git://github.com/", "ssh://github.com/"]
            .iter()
            .find_map(|p| url.strip_prefix(p))
    })?;
    let rest = rest.trim_end_matches('/');
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let mut parts = rest.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let seg_ok = |s: &str| {
        !s.is_empty()
            && s.len() <= 100
            && !s.starts_with('-')
            && !s.starts_with('.')
            && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    };
    if seg_ok(owner) && seg_ok(repo) {
        Some(format!("{owner}/{repo}"))
    } else {
        None
    }
}

/// The workspace's pinned GitHub repo (`owner/repo`) from its `origin` remote.
fn origin_repo(root: &Path) -> Result<String, String> {
    let out = run_git(root, &["remote", "get-url", "origin"])?;
    if !out.ok() {
        return Err("this workspace has no `origin` remote — GitHub tools need one.".into());
    }
    parse_github_repo(out.stdout.trim())
        .ok_or_else(|| "the `origin` remote does not point at github.com — GitHub tools only work with GitHub-hosted repos.".into())
}

/// Keep the END of an oversized string (CI logs: the failure is at the tail).
fn tail_bytes(s: &str, limit: usize) -> Cow<'_, str> {
    if s.len() <= limit {
        return Cow::Borrowed(s);
    }
    let mut start = s.len() - limit;
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    Cow::Owned(format!("… (log truncated — showing the last {} KB)\n{}", limit / 1024, &s[start..]))
}

fn arg_u64(args: &Value, key: &str) -> Option<u64> {
    match args.get(key) {
        Some(Value::Number(n)) => n.as_u64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
}

// ─── MCP tools ──────────────────────────────────────────────────────────────

pub fn tool_gh_checks(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let repo = origin_repo(root)?;
    let branch = match args.get("branch").and_then(|v| v.as_str()) {
        Some(b) => validate_ref("branch", b)?,
        None => current_branch(root)?,
    };
    if branch == "HEAD" {
        return Err("detached HEAD — pass an explicit `branch`.".into());
    }
    let limit = arg_u64(args, "limit").unwrap_or(5).clamp(1, 20).to_string();
    let out = run_gh(root, &[
        "run", "list", "-R", &repo, "--branch", &branch, "--limit", &limit,
        "--json", "databaseId,workflowName,displayTitle,status,conclusion,event,createdAt,url",
    ])?;
    if !out.ok() {
        return Err(format!("gh run list failed: {}", out.err_text()));
    }
    if out.stdout.trim() == "[]" {
        return Ok(format!("No workflow runs found for branch `{branch}` on {repo}."));
    }
    Ok(truncate_bytes(out.stdout.trim(), MAX_GH_BYTES).into_owned())
}

pub fn tool_gh_run_view(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let repo = origin_repo(root)?;
    let run_id = arg_u64(args, "run_id").ok_or("missing `run_id` (a number)")?.to_string();
    let failed_logs = args.get("failed_logs").and_then(|v| v.as_bool()).unwrap_or(false);
    if failed_logs {
        let out = run_gh(root, &["run", "view", &run_id, "-R", &repo, "--log-failed"])?;
        if !out.ok() {
            return Err(format!("gh run view failed: {}", out.err_text()));
        }
        if out.stdout.trim().is_empty() {
            return Ok("No failed-job logs (the run may still be in progress, or nothing failed).".into());
        }
        return Ok(tail_bytes(out.stdout.trim(), MAX_GH_BYTES).into_owned());
    }
    let out = run_gh(root, &[
        "run", "view", &run_id, "-R", &repo,
        "--json", "databaseId,workflowName,displayTitle,status,conclusion,event,createdAt,url,jobs",
    ])?;
    if !out.ok() {
        return Err(format!("gh run view failed: {}", out.err_text()));
    }
    Ok(truncate_bytes(out.stdout.trim(), MAX_GH_BYTES).into_owned())
}

pub fn tool_gh_pr_list(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let repo = origin_repo(root)?;
    let state = match args.get("state").and_then(|v| v.as_str()).unwrap_or("open") {
        s @ ("open" | "closed" | "merged" | "all") => s,
        other => return Err(format!("invalid state `{other}` — use open, closed, merged, or all")),
    };
    let limit = arg_u64(args, "limit").unwrap_or(10).clamp(1, 20).to_string();
    let out = run_gh(root, &[
        "pr", "list", "-R", &repo, "--state", state, "--limit", &limit,
        "--json", "number,title,state,isDraft,headRefName,baseRefName,author,reviewDecision,url,updatedAt",
    ])?;
    if !out.ok() {
        return Err(format!("gh pr list failed: {}", out.err_text()));
    }
    if out.stdout.trim() == "[]" {
        return Ok(format!("No {state} pull requests on {repo}."));
    }
    Ok(truncate_bytes(out.stdout.trim(), MAX_GH_BYTES).into_owned())
}

pub fn tool_gh_pr_view(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let repo = origin_repo(root)?;
    let number = arg_u64(args, "number").ok_or("missing `number` (the PR number)")?.to_string();
    let out = run_gh(root, &[
        "pr", "view", &number, "-R", &repo,
        "--json", "number,title,state,isDraft,body,author,headRefName,baseRefName,reviewDecision,mergeable,statusCheckRollup,additions,deletions,changedFiles,url",
    ])?;
    if !out.ok() {
        return Err(format!("gh pr view failed: {}", out.err_text()));
    }
    Ok(truncate_bytes(out.stdout.trim(), MAX_GH_BYTES).into_owned())
}

pub fn tool_gh_pr_diff(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let repo = origin_repo(root)?;
    let number = arg_u64(args, "number").ok_or("missing `number` (the PR number)")?.to_string();
    let out = run_gh(root, &["pr", "diff", &number, "-R", &repo])?;
    if !out.ok() {
        return Err(format!("gh pr diff failed: {}", out.err_text()));
    }
    if out.stdout.trim().is_empty() {
        return Ok("Empty diff.".into());
    }
    Ok(truncate_bytes(&out.stdout, MAX_GH_BYTES).into_owned())
}

pub(crate) fn validate_pr_title(s: &str) -> Result<String, String> {
    let t = s.trim();
    if t.is_empty() {
        return Err("PR title is empty".into());
    }
    if t.chars().count() > MAX_TITLE_CHARS {
        return Err(format!("PR title too long (max {MAX_TITLE_CHARS} chars)"));
    }
    if t.contains('\0') || t.contains('\n') || t.contains('\r') {
        return Err("PR title must be a single line".into());
    }
    Ok(t.to_string())
}

pub(crate) fn validate_pr_body(s: &str) -> Result<String, String> {
    if s.len() > MAX_BODY_BYTES {
        return Err(format!("PR body too long ({} bytes; max {MAX_BODY_BYTES})", s.len()));
    }
    if s.contains('\0') {
        return Err("PR body contains NUL".into());
    }
    Ok(s.to_string())
}

pub fn tool_gh_pr_create(args: &Value, roots: &[PathBuf]) -> Result<String, String> {
    let root = workspace_root(roots)?;
    let repo = origin_repo(root)?;
    let title = validate_pr_title(args.get("title").and_then(|v| v.as_str()).ok_or("missing `title`")?)?;
    let body = validate_pr_body(args.get("body").and_then(|v| v.as_str()).unwrap_or(""))?;
    let head = current_branch(root)?;
    if head == "HEAD" {
        return Err("detached HEAD — checkout a branch before creating a PR.".into());
    }
    let base = match args.get("base").and_then(|v| v.as_str()) {
        Some(b) => Some(validate_ref("base branch", b)?),
        None => None,
    };
    let draft = args.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
    let mut gh_args: Vec<&str> = vec![
        "pr", "create", "-R", &repo, "--head", &head, "--title", &title, "--body", &body,
    ];
    if let Some(b) = base.as_deref() {
        gh_args.push("--base");
        gh_args.push(b);
    }
    if draft {
        gh_args.push("--draft");
    }
    let out = run_gh(root, &gh_args)?;
    if !out.ok() {
        return Err(format!("gh pr create failed: {}", out.err_text()));
    }
    let url = out.stdout.trim();
    Ok(if url.is_empty() {
        format!("Pull request created for {head}.")
    } else {
        format!("Pull request created: {url}")
    })
}

// ─── UI aggregate (branch chip + popover) ───────────────────────────────────

const FAIL_CONCLUSIONS: [&str; 3] = ["failure", "startup_failure", "timed_out"];

/// Red completed run → add `failedJob`/`failedStep` (first failing job + its
/// first failing step) so the popover can say WHAT broke. One extra `gh run
/// view`, spent only on failures; any miss leaves the run untouched.
fn attach_failure_detail(root: &Path, repo: &str, mut run: Value) -> Value {
    let is_red = run.get("status").and_then(Value::as_str) == Some("completed")
        && run
            .get("conclusion")
            .and_then(Value::as_str)
            .is_some_and(|c| FAIL_CONCLUSIONS.contains(&c));
    let Some(id) = run.get("databaseId").and_then(Value::as_u64) else { return run };
    if !is_red {
        return run;
    }
    let Ok(out) = run_gh(root, &["run", "view", &id.to_string(), "-R", repo, "--json", "jobs"]) else {
        return run;
    };
    if !out.ok() {
        return run;
    }
    let Ok(v) = serde_json::from_str::<Value>(&out.stdout) else { return run };
    let failed_job = v
        .get("jobs")
        .and_then(Value::as_array)
        .and_then(|jobs| {
            jobs.iter().find(|j| {
                j.get("conclusion")
                    .and_then(Value::as_str)
                    .is_some_and(|c| FAIL_CONCLUSIONS.contains(&c))
            })
        });
    if let Some(job) = failed_job {
        if let Some(obj) = run.as_object_mut() {
            if let Some(name) = job.get("name").and_then(Value::as_str) {
                obj.insert("failedJob".into(), json!(name));
            }
            let step = job.get("steps").and_then(Value::as_array).and_then(|steps| {
                steps
                    .iter()
                    .find(|s| {
                        s.get("conclusion")
                            .and_then(Value::as_str)
                            .is_some_and(|c| FAIL_CONCLUSIONS.contains(&c))
                    })
                    .and_then(|s| s.get("name").and_then(Value::as_str))
            });
            if let Some(step) = step {
                obj.insert("failedStep".into(), json!(step));
            }
        }
    }
    run
}

/// One-call status snapshot for the branch chip. Never errors — every failure
/// mode collapses into a `state` the UI can render (missing gh, no auth,
/// non-GitHub origin, plain non-repo folder).
pub(crate) fn branch_status_sync(root: &Path) -> Value {
    // Not a git repo (or no commits) → chip stays a plain label.
    let branch = match current_branch(root) {
        Ok(b) => b,
        Err(_) => return json!({ "state": "no_repo" }),
    };
    let repo = match run_git(root, &["remote", "get-url", "origin"]) {
        Ok(out) if out.ok() => match parse_github_repo(out.stdout.trim()) {
            Some(r) => r,
            None => return json!({ "state": "not_github", "branch": branch }),
        },
        _ => return json!({ "state": "not_github", "branch": branch }),
    };
    let url = format!("https://github.com/{repo}");

    // gh availability + auth, distinguished so the popover can say what to fix.
    match run_gh(root, &["auth", "status", "--hostname", "github.com"]) {
        Ok(out) if out.ok() => {}
        Ok(_) => return json!({ "state": "no_auth", "branch": branch, "repo": repo, "url": url }),
        Err(e) if e == GH_MISSING => {
            return json!({ "state": "no_gh", "branch": branch, "repo": repo, "url": url })
        }
        Err(e) => return json!({ "state": "error", "branch": branch, "repo": repo, "url": url, "detail": e }),
    }

    // Ahead/behind vs upstream (null when no upstream is set).
    let (mut ahead, mut behind) = (Value::Null, Value::Null);
    if branch != "HEAD" {
        if let Ok(out) = run_git(root, &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"]) {
            if out.ok() {
                let mut it = out.stdout.split_whitespace();
                if let (Some(a), Some(b)) = (it.next(), it.next()) {
                    if let (Ok(a), Ok(b)) = (a.parse::<u64>(), b.parse::<u64>()) {
                        ahead = json!(a);
                        behind = json!(b);
                    }
                }
            }
        }
    }

    // Latest workflow run + open PR for this branch — the two slow network
    // calls, run IN PARALLEL (scoped threads; run_gh is already blocking +
    // self-contained). Partial failures degrade to nulls with a detail note —
    // a flaky network must not blank the chip.
    fn first_of_json_array(out: &GhOut) -> Value {
        serde_json::from_str::<Value>(&out.stdout)
            .ok()
            .and_then(|v| v.as_array().and_then(|a| a.first().cloned()))
            .unwrap_or(Value::Null)
    }
    // Local HEAD sha: lets the chip claim tag-triggered runs (release builds
    // have head_branch = the TAG, so a branch-scoped list misses them; the
    // commit sha still matches).
    let head_sha = run_git(root, &["rev-parse", "HEAD"])
        .ok()
        .filter(|o| o.ok())
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default();
    let (run, detail, pr) = if branch == "HEAD" {
        (Value::Null, Value::Null, Value::Null)
    } else {
        std::thread::scope(|scope| {
            let run_h = scope.spawn(|| {
                // Unscoped newest-first window, filtered locally: first run on
                // this branch OR on the local HEAD commit (tag runs). Nothing
                // in the window → null ("no recent runs on this branch").
                match run_gh(root, &[
                    "run", "list", "-R", &repo, "--limit", "20",
                    "--json", "databaseId,workflowName,displayTitle,status,conclusion,event,createdAt,url,headBranch,headSha",
                ]) {
                    Ok(out) if out.ok() => {
                        let picked = serde_json::from_str::<Value>(&out.stdout)
                            .ok()
                            .and_then(|v| v.as_array().and_then(|runs| {
                                runs.iter()
                                    .find(|r| {
                                        r.get("headBranch").and_then(Value::as_str) == Some(branch.as_str())
                                            || (!head_sha.is_empty()
                                                && r.get("headSha").and_then(Value::as_str)
                                                    == Some(head_sha.as_str()))
                                    })
                                    .cloned()
                            }))
                            .unwrap_or(Value::Null);
                        (attach_failure_detail(root, &repo, picked), Value::Null)
                    }
                    Ok(out) => (Value::Null, json!(out.err_text())),
                    Err(e) => (Value::Null, json!(e)),
                }
            });
            let pr_h = scope.spawn(|| {
                match run_gh(root, &[
                    "pr", "list", "-R", &repo, "--head", &branch, "--state", "open", "--limit", "1",
                    "--json", "number,title,isDraft,reviewDecision,url",
                ]) {
                    Ok(out) if out.ok() => first_of_json_array(&out),
                    _ => Value::Null,
                }
            });
            let (run, detail) = run_h.join().unwrap_or((Value::Null, json!("run fetch panicked")));
            let pr = pr_h.join().unwrap_or(Value::Null);
            (run, detail, pr)
        })
    };

    json!({
        "state": "ok",
        "branch": branch,
        "repo": repo,
        "url": url,
        "ahead": ahead,
        "behind": behind,
        "run": run,
        "pr": pr,
        "detail": detail,
    })
}

/// UI entry: aggregated GitHub status for the workspace's branch chip.
/// Read-only; ~2-4 subprocess calls, so it runs on the blocking pool.
#[tauri::command]
pub async fn gh_branch_status(root: Option<String>) -> Result<Value, String> {
    let Some(root) = super::workspace::resolve_root(root) else {
        return Ok(json!({ "state": "no_root" }));
    };
    tokio::task::spawn_blocking(move || branch_status_sync(&root))
        .await
        .map_err(|e| format!("gh status task failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_github_remotes() {
        for (url, want) in [
            ("https://github.com/Blazzer10200/rift-tauri.git", "Blazzer10200/rift-tauri"),
            ("https://github.com/owner/repo", "owner/repo"),
            ("https://github.com/owner/repo/", "owner/repo"),
            ("git@github.com:owner/repo.git", "owner/repo"),
            ("ssh://git@github.com/owner/repo.git", "owner/repo"),
            ("git://github.com/o-w.n_er/re.po", "o-w.n_er/re.po"),
        ] {
            assert_eq!(parse_github_repo(url).as_deref(), Some(want), "url: {url}");
        }
    }

    #[test]
    fn rejects_non_github_and_lookalike_remotes() {
        for url in [
            "https://gitlab.com/owner/repo.git",
            "https://github.com.evil.com/owner/repo",
            "https://evil.com/github.com/owner/repo",
            "git@github.com:owner",                 // missing repo
            "https://github.com/owner/repo/extra", // path too deep
            "https://github.com/-owner/repo",      // flag-shaped owner
            "https://github.com/owner/.repo",      // dot-leading repo
            "",
        ] {
            assert_eq!(parse_github_repo(url), None, "url: {url}");
        }
    }

    #[test]
    fn pr_title_bounds() {
        assert!(validate_pr_title("").is_err());
        assert!(validate_pr_title("   ").is_err());
        assert!(validate_pr_title("fix: two\nlines").is_err());
        assert!(validate_pr_title(&"x".repeat(MAX_TITLE_CHARS + 1)).is_err());
        assert!(validate_pr_title("feat: add GitHub integration").is_ok());
    }

    #[test]
    fn pr_body_bounds() {
        assert!(validate_pr_body("").is_ok());
        assert!(validate_pr_body("multi\nline\nbody").is_ok());
        assert!(validate_pr_body("nul\0byte").is_err());
        assert!(validate_pr_body(&"x".repeat(MAX_BODY_BYTES + 1)).is_err());
    }

    #[test]
    fn tail_keeps_the_end() {
        let s = format!("{}THE-END", "a".repeat(100_000));
        let t = tail_bytes(&s, 1024);
        assert!(t.ends_with("THE-END"));
        assert!(t.starts_with("… (log truncated"));
        let small = tail_bytes("short", 1024);
        assert_eq!(small, "short");
    }

    #[test]
    fn arg_u64_accepts_number_and_string() {
        assert_eq!(arg_u64(&serde_json::json!({"run_id": 42}), "run_id"), Some(42));
        assert_eq!(arg_u64(&serde_json::json!({"run_id": "42"}), "run_id"), Some(42));
        assert_eq!(arg_u64(&serde_json::json!({"run_id": "-1"}), "run_id"), None);
        assert_eq!(arg_u64(&serde_json::json!({}), "run_id"), None);
    }

    #[test]
    fn no_workspace_root_errors() {
        assert!(tool_gh_checks(&serde_json::json!({}), &[]).is_err());
    }
}
