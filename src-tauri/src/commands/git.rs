//! Frontend-facing git commands for the Environment / Source Control panel.
//!
//! The `assistant::git_local` tools serve the Claude CLI over MCP and return
//! plain strings; the Environment panel needs *typed* working-tree state it can
//! render. These `#[tauri::command]` wrappers reuse `git_local`'s hardened
//! `run_git` + validators (no shell, no credential prompts, env stripped) and
//! parse the output into serializable structs. Read-only except
//! `git_commit_and_push`.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::assistant::git_local::{run_git, validate_message, validate_path};

/// Mirror of git_local's diff cap — truncate before handing a diff to the UI.
const MAX_DIFF_BYTES: usize = 64 * 1024;

#[derive(Serialize)]
pub struct GitFile {
    pub path: String,
    /// Porcelain X column (staged state): one of `MADRC?! ` or space.
    pub staged: char,
    /// Porcelain Y column (unstaged state).
    pub unstaged: char,
    pub adds: i32,
    pub dels: i32,
}

#[derive(Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<GitFile>,
    pub total_adds: i32,
    pub total_dels: i32,
    /// `origin` remote URL, for the panel's "Sources" section.
    pub remote: Option<String>,
    /// Resolved workspace root (also shown under "Sources").
    pub root: String,
}

fn resolve_root(root: Option<String>) -> Result<PathBuf, String> {
    let r = root.unwrap_or_default();
    if r.trim().is_empty() {
        return Err("no workspace folder open".into());
    }
    let p = PathBuf::from(r);
    if !p.is_dir() {
        return Err(format!("workspace path is not a directory: {}", p.display()));
    }
    Ok(p)
}

/// Parse the porcelain `-b` header (the text after `## `) into
/// `(branch, upstream, ahead, behind)`. Tolerant of detached HEAD, fresh repos,
/// and missing upstreams.
fn parse_branch_line(s: &str) -> (String, Option<String>, u32, u32) {
    if s.starts_with("HEAD (no branch)") {
        return ("HEAD".to_string(), None, 0, 0);
    }
    if let Some(rest) = s.strip_prefix("No commits yet on ") {
        return (rest.trim().to_string(), None, 0, 0);
    }
    // Split off a trailing " [ahead N, behind M]" / "[gone]" bracket.
    let (head, bracket) = match s.split_once(" [") {
        Some((h, b)) => (h, Some(b.trim_end_matches(']'))),
        None => (s, None),
    };
    let (branch, upstream) = match head.split_once("...") {
        Some((b, up)) => (b.to_string(), Some(up.to_string())),
        None => (head.to_string(), None),
    };
    let (mut ahead, mut behind) = (0u32, 0u32);
    if let Some(b) = bracket {
        for part in b.split(',') {
            let part = part.trim();
            if let Some(n) = part.strip_prefix("ahead ") {
                ahead = n.trim().parse().unwrap_or(0);
            } else if let Some(n) = part.strip_prefix("behind ") {
                behind = n.trim().parse().unwrap_or(0);
            }
        }
    }
    (branch, upstream, ahead, behind)
}

/// Accumulate `git diff --numstat` output into `path -> (adds, dels)`. Binary
/// files report `-` for both counts and fold to 0.
fn parse_numstat(stdout: &str, map: &mut HashMap<String, (i32, i32)>) {
    for line in stdout.lines() {
        let mut it = line.splitn(3, '\t');
        let adds = it.next().unwrap_or("0");
        let dels = it.next().unwrap_or("0");
        let path = match it.next() {
            Some(p) => p,
            None => continue,
        };
        let a = adds.parse::<i32>().unwrap_or(0);
        let d = dels.parse::<i32>().unwrap_or(0);
        let entry = map.entry(path.to_string()).or_insert((0, 0));
        entry.0 += a;
        entry.1 += d;
    }
}

/// Typed working-tree status for the Environment panel: branch + tracking,
/// per-file XY state with numstat, and the aggregate +/- totals.
#[tauri::command]
pub fn git_working_status(root: Option<String>) -> Result<GitStatus, String> {
    let root = resolve_root(root)?;
    let st = run_git(&root, &["status", "--porcelain=v1", "-b"])?;
    if !st.ok() {
        return Err(format!("git status failed: {}", st.err_text()));
    }

    let mut branch = String::new();
    let mut upstream = None;
    let mut ahead = 0;
    let mut behind = 0;
    let mut raw_files: Vec<(char, char, String)> = Vec::new();

    for line in st.stdout.lines() {
        if let Some(b) = line.strip_prefix("## ") {
            let (br, up, a, be) = parse_branch_line(b);
            branch = br;
            upstream = up;
            ahead = a;
            behind = be;
        } else if line.len() >= 3 {
            let bytes = line.as_bytes();
            let x = bytes[0] as char;
            let y = bytes[1] as char;
            let mut path = line[3..].to_string();
            // Rename lines read `old -> new`; keep the destination path.
            if let Some(idx) = path.find(" -> ") {
                path = path[idx + 4..].to_string();
            }
            // Porcelain quotes paths containing unusual bytes; unwrap them.
            if path.len() >= 2 && path.starts_with('"') && path.ends_with('"') {
                path = path[1..path.len() - 1].to_string();
            }
            raw_files.push((x, y, path));
        }
    }

    // numstat for both the staged and unstaged diffs; `--no-renames` keeps paths
    // plain so they line up with the porcelain entries above.
    let mut stats: HashMap<String, (i32, i32)> = HashMap::new();
    if let Ok(d) = run_git(&root, &["diff", "--numstat", "--no-renames"]) {
        if d.ok() {
            parse_numstat(&d.stdout, &mut stats);
        }
    }
    if let Ok(d) = run_git(&root, &["diff", "--numstat", "--no-renames", "--cached"]) {
        if d.ok() {
            parse_numstat(&d.stdout, &mut stats);
        }
    }
    let total_adds: i32 = stats.values().map(|(a, _)| *a).sum();
    let total_dels: i32 = stats.values().map(|(_, d)| *d).sum();

    let files = raw_files
        .into_iter()
        .map(|(x, y, path)| {
            let (adds, dels) = stats.get(&path).copied().unwrap_or((0, 0));
            GitFile { path, staged: x, unstaged: y, adds, dels }
        })
        .collect();

    let remote = run_git(&root, &["remote", "get-url", "origin"])
        .ok()
        .filter(|o| o.ok())
        .map(|o| o.stdout.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(GitStatus {
        branch,
        upstream,
        ahead,
        behind,
        files,
        total_adds,
        total_dels,
        remote,
        root: root.display().to_string(),
    })
}

/// Raw unified diff for one file (staged or working-tree), truncated at 64 KB.
#[tauri::command]
pub fn git_file_diff(root: Option<String>, path: String, cached: bool) -> Result<String, String> {
    let root = resolve_root(root)?;
    let safe = validate_path(&root, &path)?;
    let mut args: Vec<&str> = vec!["diff"];
    if cached {
        args.push("--cached");
    }
    args.push("--");
    args.push(&safe);
    let out = run_git(&root, &args)?;
    if !out.ok() {
        return Err(format!("git diff failed: {}", out.err_text()));
    }
    let mut s = out.stdout;
    // Untracked/new files aren't in the index, so plain `git diff` is empty for
    // them. Fall back to `--no-index` against the null device, which emits the
    // whole file as an add-diff (it exits 1 when the files differ — expected, so
    // we read stdout regardless of the code).
    if s.trim().is_empty() && !cached {
        if let Ok(alt) = run_git(&root, &["diff", "--no-index", "--", "/dev/null", &safe]) {
            if !alt.stdout.trim().is_empty() {
                s = alt.stdout;
            }
        }
    }
    if s.len() > MAX_DIFF_BYTES {
        let mut cut = MAX_DIFF_BYTES;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s.truncate(cut);
        s.push_str("\n… diff truncated (exceeded 64 KB) …\n");
    }
    Ok(s)
}

/// Stage all changes (`git add -A`), commit, and optionally push. v1 is
/// commit-all; per-file staging is a future addition. Push is no-arg so it
/// follows the branch's configured upstream; a missing upstream surfaces as a
/// clear error rather than a prompt (credentials are non-interactive).
#[tauri::command]
pub fn git_commit_and_push(
    root: Option<String>,
    message: String,
    push: bool,
) -> Result<String, String> {
    let root = resolve_root(root)?;
    let msg = validate_message(&message)?;

    let add = run_git(&root, &["add", "-A"])?;
    if !add.ok() {
        return Err(format!("git add failed: {}", add.err_text()));
    }
    let commit = run_git(&root, &["commit", "-m", &msg])?;
    if !commit.ok() {
        return Err(format!("git commit failed: {}", commit.err_text()));
    }
    let mut summary = commit.stdout.trim().to_string();
    if push {
        let p = run_git(&root, &["push"])?;
        if !p.ok() {
            return Err(format!("commit succeeded, but push failed: {}", p.err_text()));
        }
        summary.push_str("\nPushed.");
    }
    Ok(summary)
}
