//! Phase 3b — edit-applying swarm orchestrator.
//!
//! Drives a confirmed-findings list into worktree-isolated, gate-verified,
//! adversarially-reviewed edits that cherry-pick back to main only if they pass.
//! The spine is the harness proven in `scripts/proto/swarm-harness.ps1` and
//! `docs/design/edit-swarm-safety-layer.md`. Decisions from §7 of that doc:
//!
//!  (a) cargo gate uses a DEDICATED, persistent `CARGO_TARGET_DIR` disjoint from
//!      the dev/main target dir, so a worktree `cargo check` never collides with
//!      a running `tauri dev` — no quit-dev, no fragile process detection.
//!  (b) merge-on-accept = `git cherry-pick` the worktree commit.
//!  (c) lives in its own module (assistant/mod.rs is already the largest file).
//!  (d) adversarial review = a dedicated diff-vs-finding prompt.
//!
//! Safety invariant (design §4.2): the node_modules junction is removed with
//! `rmdir` (reparse-point only) BEFORE the worktree is removed — never a
//! recursive delete that could follow the junction into the real node_modules.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::process::Command as TokioCommand;

const PROGRESS_EVENT: &str = "swarm://progress";
const DONE_EVENT: &str = "swarm://done";

/// Per-agent budget + turn caps. An edit pass is bounded; a swarm shouldn't be
/// able to spiral on one file.
const EDIT_MAX_TURNS: &str = "40";
const EDIT_MAX_BUDGET_USD: &str = "2.00";
const REVIEW_MAX_BUDGET_USD: &str = "0.50";

// ─── public types ─────────────────────────────────────────────────────────

/// A confirmed finding to fix. Mirrors the audit-swarm's output row.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Workspace-relative path of the file to fix.
    pub file: String,
    #[serde(default)]
    pub line: Option<u32>,
    pub evidence: String,
    pub suggested_fix: String,
}

/// What kind of automated gate applies to a file, by extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gate {
    Frontend, // npm run check (svelte-kit sync && svelte-check)
    Rust,     // cargo check
    None,     // no automated gate for this file type
}

fn gate_for(file: &str) -> Gate {
    let lower = file.to_ascii_lowercase();
    if lower.ends_with(".rs") {
        Gate::Rust
    } else if lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with(".jsx")
        || lower.ends_with(".svelte")
    {
        Gate::Frontend
    } else {
        Gate::None
    }
}

/// Reject any finding path that isn't a safe workspace-relative path. Guards
/// the edit agent (runs under bypassPermissions) and the git ops from escaping
/// the worktree via `..`, absolute/drive-relative paths, or embedded newlines.
fn validate_rel_path(file: &str) -> Result<(), String> {
    if file.is_empty() {
        return Err("finding has an empty file path".into());
    }
    if file.contains('\n') || file.contains('\r') || file.contains('\0') {
        return Err(format!("finding file path contains control characters: {file:?}"));
    }
    let p = Path::new(file);
    if p.is_absolute() {
        return Err(format!("finding file path must be workspace-relative: {file}"));
    }
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                return Err(format!("finding file path escapes the workspace with '..': {file}"));
            }
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                return Err(format!("finding file path must be workspace-relative: {file}"));
            }
            _ => {}
        }
    }
    Ok(())
}

/// The outcome for one file (= one agent).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentOutcome {
    pub agent: String,
    pub file: String,
    pub findings: usize,
    /// Last stage reached: isolate / edit / gate / review / merge / cleanup.
    pub stage: String,
    /// "pass" | "fail" | "skipped" | "n/a"
    pub gate: String,
    pub gate_detail: Option<String>,
    /// "accept" | "reject" | "n/a"
    pub review: String,
    pub review_detail: Option<String>,
    pub merged: bool,
    pub diff: Option<String>,
    pub error: Option<String>,
}

impl AgentOutcome {
    fn new(agent: String, file: String, findings: usize) -> Self {
        AgentOutcome {
            agent,
            file,
            findings,
            stage: "isolate".into(),
            gate: "n/a".into(),
            gate_detail: None,
            review: "n/a".into(),
            review_detail: None,
            merged: false,
            diff: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmReport {
    pub agents: Vec<AgentOutcome>,
    pub merged_count: usize,
    pub main_tree_intact: bool,
    pub root: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmEnv {
    pub is_git_repo: bool,
    pub clean_tree: bool,
    pub claude_present: bool,
    pub node_modules_present: bool,
    pub head_short: Option<String>,
}

// ─── git helpers (hardened, no shell) ───────────────────────────────────────

/// Run a git command, returning (ok, stdout, stderr). Never uses a shell; never
/// emits `--force`. Mirrors the hardening posture in `assistant/git_local.rs`.
async fn git(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let mut cmd = TokioCommand::new("git");
    cmd.current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    match cmd.output().await {
        Ok(o) => (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        ),
        Err(e) => (false, String::new(), format!("failed to run git: {e}")),
    }
}

async fn is_git_repo(root: &Path) -> bool {
    git(root, &["rev-parse", "--is-inside-work-tree"]).await.0
}

async fn is_clean(root: &Path) -> bool {
    let (ok, out, _) = git(root, &["status", "--porcelain"]).await;
    ok && out.trim().is_empty()
}

async fn head_sha(root: &Path) -> Option<String> {
    let (ok, out, _) = git(root, &["rev-parse", "HEAD"]).await;
    if ok {
        Some(out.trim().to_string())
    } else {
        None
    }
}

// ─── junction / worktree lifecycle ──────────────────────────────────────────

/// Create a `node_modules` junction (Windows) / symlink (unix) in the worktree
/// pointing at the main checkout's node_modules, so the frontend gate can run
/// without a reinstall. Returns the junction path if created.
fn junction_node_modules(root: &Path, wt: &Path) -> Option<PathBuf> {
    let target = root.join("node_modules");
    if !target.is_dir() {
        return None;
    }
    let link = wt.join("node_modules");
    if link.exists() {
        return Some(link); // worktree already had one (shouldn't, it's gitignored)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .creation_flags(0x08000000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if matches!(status, Ok(s) if s.success()) {
            return Some(link);
        }
        None
    }
    #[cfg(not(windows))]
    {
        match std::os::unix::fs::symlink(&target, &link) {
            Ok(_) => Some(link),
            Err(_) => None,
        }
    }
}

/// Remove a junction/symlink WITHOUT recursing into its target. On Windows
/// `rmdir` deletes the reparse point only; a recursive delete would follow the
/// link and wipe the real node_modules (design §4.2 — the load-bearing detail).
fn remove_junction(link: &Path) {
    if !link.exists() && link.symlink_metadata().is_err() {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd")
            .args(["/c", "rmdir"])
            .arg(link)
            .creation_flags(0x08000000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::fs::remove_file(link); // unix symlink: unlink, never follows
    }
}

/// Always-run cleanup: junction first, THEN worktree, THEN prune. Best-effort;
/// each step is independent so a failure in one still attempts the rest.
async fn cleanup(root: &Path, wt: &Path, junction: Option<&Path>) {
    if let Some(j) = junction {
        remove_junction(j);
    }
    let _ = git(root, &["worktree", "remove", "--force", &wt.to_string_lossy()]).await;
    let _ = git(root, &["worktree", "prune"]).await;
}

// ─── gate ───────────────────────────────────────────────────────────────────

struct GateResult {
    /// "pass" | "fail" | "skipped"
    status: &'static str,
    detail: String,
}

/// Run the verify gate for a file inside its worktree. Frontend = `npm run
/// check` against the junctioned node_modules; Rust = `cargo check` with a
/// dedicated CARGO_TARGET_DIR (collision-free with dev). `None` → skipped.
async fn run_gate(wt: &Path, gate: Gate, swarm_target: &Path) -> GateResult {
    match gate {
        Gate::None => GateResult {
            status: "skipped",
            detail: "no automated gate for this file type".into(),
        },
        Gate::Frontend => {
            let mut cmd = TokioCommand::new(if cfg!(windows) { "npm.cmd" } else { "npm" });
            cmd.current_dir(wt)
                .args(["run", "check"])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            #[cfg(windows)]
            cmd.creation_flags(0x08000000);
            run_gate_cmd(cmd, "npm run check").await
        }
        Gate::Rust => {
            let mut cmd = TokioCommand::new("cargo");
            cmd.current_dir(wt)
                .args(["check", "--manifest-path", "src-tauri/Cargo.toml"])
                .env("CARGO_TARGET_DIR", swarm_target)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            #[cfg(windows)]
            cmd.creation_flags(0x08000000);
            run_gate_cmd(cmd, "cargo check").await
        }
    }
}

async fn run_gate_cmd(mut cmd: TokioCommand, label: &str) -> GateResult {
    match cmd.output().await {
        Ok(o) if o.status.success() => GateResult {
            status: "pass",
            detail: format!("{label} passed"),
        },
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            let out = String::from_utf8_lossy(&o.stdout);
            // Surface the most informative tail (errors land in either stream).
            let tail: String = err
                .lines()
                .chain(out.lines())
                .filter(|l| {
                    let t = l.to_ascii_lowercase();
                    t.contains("error") || t.contains("svelte-check") || t.contains("found ")
                })
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            GateResult {
                status: "fail",
                detail: if tail.is_empty() {
                    format!("{label} failed (exit {:?})", o.status.code())
                } else {
                    tail
                },
            }
        }
        Err(e) => GateResult {
            status: "fail",
            detail: format!("{label} could not run: {e}"),
        },
    }
}

// ─── claude agents ──────────────────────────────────────────────────────────

fn edit_prompt(file: &str, findings: &[&Finding]) -> String {
    let mut body = String::new();
    for (i, f) in findings.iter().enumerate() {
        let loc = f.line.map(|l| format!(" (line {l})")).unwrap_or_default();
        body.push_str(&format!(
            "{}.{loc} {}\n   Suggested fix: {}\n",
            i + 1,
            f.evidence.trim(),
            f.suggested_fix.trim()
        ));
    }
    format!(
        "You are fixing confirmed issues in EXACTLY ONE file: `{file}`.\n\n\
         Apply minimal, targeted edits that resolve each finding below. Edit the file in place \
         with your file-editing tools. Do NOT touch any other file. Do NOT reformat unrelated code. \
         Do NOT create new files. Do NOT run git. Make only the changes needed to fix the findings, \
         and keep the file compiling / type-checking.\n\n\
         Findings:\n{body}"
    )
}

/// Spawn the edit agent inside the worktree. Built-in Read/Write/Edit tools
/// under bypassPermissions; no external MCP, no user slash-commands/hooks, no
/// dynamic system-prompt sections (deterministic, scoped to the prompt).
async fn run_edit_agent(wt: &Path, file: &str, findings: &[&Finding]) -> Result<(), String> {
    let mut cmd = crate::assistant::claude_command()
        .ok_or("claude CLI not on PATH — install Claude Code")?;
    cmd.arg("-p")
        .arg(edit_prompt(file, findings))
        .arg("--permission-mode")
        .arg("bypassPermissions")
        .arg("--strict-mcp-config") // no filesystem MCP servers
        .arg("--disable-slash-commands")
        .arg("--exclude-dynamic-system-prompt-sections")
        .arg("--max-turns")
        .arg(EDIT_MAX_TURNS)
        .arg("--max-budget-usd")
        .arg(EDIT_MAX_BUDGET_USD)
        .arg("--output-format")
        .arg("json")
        .env("CLAUDE_DISABLE_HOOKS", "1")
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("DISABLE_AUTOUPDATER", "1")
        .current_dir(wt)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = cmd
        .output()
        .await
        .map_err(|e| format!("spawn edit agent: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "edit agent exited {:?}: {}",
            out.status.code(),
            err.lines().last().unwrap_or("").trim()
        ));
    }
    Ok(())
}

/// Adversarial diff-vs-finding review. Pure reasoning over the diff text — no
/// tools, neutral cwd. Returns (accept, reason). Unparseable output → reject.
async fn run_review_agent(file: &str, findings: &[&Finding], diff: &str) -> (bool, String) {
    let mut body = String::new();
    for (i, f) in findings.iter().enumerate() {
        body.push_str(&format!("{}. {}\n", i + 1, f.evidence.trim()));
    }
    let prompt = format!(
        "A code-fixing agent produced the diff below to resolve findings in `{file}`.\n\n\
         Findings:\n{body}\n\
         Diff:\n```diff\n{diff}\n```\n\n\
         Does this diff (1) actually resolve the findings, (2) introduce no regression or new bug, \
         and (3) stay strictly in scope (only the necessary change)? Accept ONLY if all three hold.\n\
         Respond with ONLY a single-line JSON object: {{\"accept\": <bool>, \"reason\": \"<one sentence>\"}}"
    );
    let cmd = crate::assistant::claude_command();
    let Some(mut cmd) = cmd else {
        return (false, "claude CLI not available for review".into());
    };
    cmd.arg("-p")
        .arg(prompt)
        .arg("--strict-mcp-config")
        .arg("--tools")
        .arg("") // pure reasoning, no file access
        .arg("--disable-slash-commands")
        .arg("--exclude-dynamic-system-prompt-sections")
        .arg("--max-budget-usd")
        .arg(REVIEW_MAX_BUDGET_USD)
        .arg("--output-format")
        .arg("json")
        .env("CLAUDE_DISABLE_HOOKS", "1")
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("DISABLE_AUTOUPDATER", "1")
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = match cmd.output().await {
        Ok(o) => o,
        Err(e) => return (false, format!("review agent failed to spawn: {e}")),
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    // `--output-format json` wraps the result; the model's text is in `.result`.
    let result_text = serde_json::from_str::<Value>(stdout.trim())
        .ok()
        .and_then(|v| v.get("result").and_then(|r| r.as_str()).map(String::from))
        .unwrap_or_else(|| stdout.trim().to_string());
    // Extract the first {...} JSON object from the model's text.
    match extract_verdict(&result_text) {
        Some((accept, reason)) => (accept, reason),
        None => (
            false,
            "review verdict unparseable — rejecting to stay safe".into(),
        ),
    }
}

/// Pull `{"accept":bool,"reason":string}` out of arbitrary model text.
fn extract_verdict(text: &str) -> Option<(bool, String)> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    let v: Value = serde_json::from_str(&text[start..=end]).ok()?;
    let accept = v.get("accept").and_then(|a| a.as_bool())?;
    let reason = v
        .get("reason")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string();
    Some((accept, reason))
}

// ─── orchestration ──────────────────────────────────────────────────────────

fn emit_progress(app: &AppHandle, agent: &str, stage: &str, detail: &str) {
    let _ = app.emit(
        PROGRESS_EVENT,
        serde_json::json!({ "agent": agent, "stage": stage, "detail": detail }),
    );
}

/// Process one file end-to-end. Captures all errors into the returned outcome;
/// ALWAYS cleans up its worktree + junction before returning.
async fn process_file(
    app: &AppHandle,
    root: &Path,
    proto_root: &Path,
    swarm_target: &Path,
    agent_name: String,
    file: String,
    findings: Vec<&Finding>,
) -> AgentOutcome {
    let mut outcome = AgentOutcome::new(agent_name.clone(), file.clone(), findings.len());
    let wt = proto_root.join(&agent_name);
    let mut junction: Option<PathBuf> = None;

    let inner = async {
        // 1. isolate
        emit_progress(app, &agent_name, "isolate", "git worktree add --detach");
        let (ok, _, err) = git(
            root,
            &["worktree", "add", "--detach", &wt.to_string_lossy(), "HEAD"],
        )
        .await;
        if !ok {
            return Err(format!("worktree add failed: {}", err.trim()));
        }

        // 2. junction node_modules (frontend gate needs it; harmless otherwise)
        junction = junction_node_modules(root, &wt);

        // 3. edit
        outcome.stage = "edit".into();
        emit_progress(app, &agent_name, "edit", "claude applies hash-anchored edits");
        run_edit_agent(&wt, &file, &findings).await?;

        // Did the agent change the target file?
        let (_, status_out, _) = git(&wt, &["status", "--porcelain", "--", &file]).await;
        if status_out.trim().is_empty() {
            return Err("agent produced no change to the target file".into());
        }
        // Capture the diff (target file only) for the UI + review.
        let (_, diff, _) = git(&wt, &["diff", "--", &file]).await;
        outcome.diff = Some(diff.clone());

        // 4. verify gate
        outcome.stage = "gate".into();
        let gate = gate_for(&file);
        emit_progress(app, &agent_name, "gate", "verify gate");
        let gr = run_gate(&wt, gate, swarm_target).await;
        outcome.gate = gr.status.into();
        outcome.gate_detail = Some(gr.detail.clone());
        if gr.status == "fail" {
            return Err(format!("gate FAIL → auto-revert. {}", gr.detail));
        }

        // 5. adversarial diff-vs-finding review
        outcome.stage = "review".into();
        emit_progress(app, &agent_name, "review", "adversarial diff-vs-finding review");
        let (accept, reason) = run_review_agent(&file, &findings, &diff).await;
        outcome.review = if accept { "accept" } else { "reject" }.into();
        outcome.review_detail = Some(reason.clone());
        if !accept {
            return Err(format!("review REJECT → discard. {reason}"));
        }

        // 6. commit the single-file diff in the worktree, then cherry-pick to main
        outcome.stage = "merge".into();
        emit_progress(app, &agent_name, "merge", "cherry-pick to main");
        let (ok, _, err) = git(&wt, &["add", "--", &file]).await;
        if !ok {
            return Err(format!("git add failed in worktree: {}", err.trim()));
        }
        let msg = format!("swarm: fix {file}");
        let (ok, _, err) = git(
            &wt,
            &[
                "-c",
                "user.email=swarm@rift.local",
                "-c",
                "user.name=Rift Swarm",
                "commit",
                "--only",
                "-m",
                &msg,
                "--",
                &file,
            ],
        )
        .await;
        if !ok {
            return Err(format!("worktree commit failed: {}", err.trim()));
        }
        let (ok, sha, _) = git(&wt, &["rev-parse", "HEAD"]).await;
        if !ok {
            return Err("could not read worktree commit sha".into());
        }
        let sha = sha.trim().to_string();
        let (ok, _, err) = git(root, &["cherry-pick", &sha]).await;
        if !ok {
            let _ = git(root, &["cherry-pick", "--abort"]).await;
            return Err(format!("cherry-pick to main failed: {}", err.trim()));
        }
        outcome.merged = true;
        Ok(())
    }
    .await;

    if let Err(e) = inner {
        outcome.error = Some(e);
    }
    // ALWAYS clean up — junction first, then worktree (design §4.2).
    outcome.stage = "cleanup".into();
    cleanup(root, &wt, junction.as_deref()).await;
    outcome
}

/// Probe whether the environment can run a swarm against `root`.
#[tauri::command]
pub async fn swarm_env_check(root: String) -> SwarmEnv {
    let root_path = PathBuf::from(&root);
    let is_repo = root_path.is_dir() && is_git_repo(&root_path).await;
    SwarmEnv {
        is_git_repo: is_repo,
        clean_tree: is_repo && is_clean(&root_path).await,
        claude_present: crate::assistant::claude_command().is_some(),
        node_modules_present: root_path.join("node_modules").is_dir(),
        head_short: if is_repo {
            head_sha(&root_path).await.map(|s| s[..s.len().min(7)].to_string())
        } else {
            None
        },
    }
}

/// Run the edit-applying swarm over a confirmed-findings list. Groups findings
/// by file (one-file-one-agent), processes each through worktree → edit → gate
/// → review → merge, and returns a per-agent report. Refuses to start on a
/// non-repo or dirty tree (a dirty tree makes "main tree intact" unverifiable).
#[tauri::command]
pub async fn swarm_run(
    app: AppHandle,
    root: String,
    findings: Vec<Finding>,
) -> Result<SwarmReport, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("workspace root does not exist: {root}"));
    }
    if !is_git_repo(&root_path).await {
        return Err("workspace is not a git repository".into());
    }
    if !is_clean(&root_path).await {
        return Err(
            "working tree is dirty — commit or stash first so the swarm can verify main stays intact"
                .into(),
        );
    }
    if findings.is_empty() {
        return Err("no findings to fix".into());
    }
    // Reject path-traversal before any finding reaches the edit agent or git.
    for f in &findings {
        validate_rel_path(&f.file)?;
    }

    let before_head = head_sha(&root_path).await;

    // Group findings by file, preserving first-seen order.
    let mut files: Vec<String> = Vec::new();
    for f in &findings {
        if !files.contains(&f.file) {
            files.push(f.file.clone());
        }
    }

    // Isolated scratch areas (unique per run via the current HEAD).
    let run_id = before_head
        .as_deref()
        .map(|s| &s[..s.len().min(8)])
        .unwrap_or("run");
    let proto_root = std::env::temp_dir().join(format!("rift-swarm-{run_id}"));
    let _ = std::fs::create_dir_all(&proto_root);
    // Dedicated, persistent cargo target dir — disjoint from dev/main (§7a).
    let swarm_target = std::env::temp_dir().join("rift-swarm-target");

    // Serialize files for v1 (deterministic; cherry-pick to main must serialize
    // anyway). Parallelizing the edit+gate stages is a future optimization.
    let mut agents: Vec<AgentOutcome> = Vec::new();
    for (i, file) in files.iter().enumerate() {
        let group: Vec<&Finding> = findings.iter().filter(|f| &f.file == file).collect();
        let agent_name = format!("agent-{:02}", i + 1);
        let outcome = process_file(
            &app,
            &root_path,
            &proto_root,
            &swarm_target,
            agent_name,
            file.clone(),
            group,
        )
        .await;
        agents.push(outcome);
    }

    // Best-effort: remove the (now-empty) per-run scratch dir.
    let _ = std::fs::remove_dir_all(&proto_root);

    let merged_count = agents.iter().filter(|a| a.merged).count();
    // main tree intact ⇔ no stray uncommitted changes AND no orphan worktrees.
    let clean_after = is_clean(&root_path).await;
    let (_, wt_list, _) = git(&root_path, &["worktree", "list"]).await;
    let no_orphan_worktrees = wt_list.lines().count() <= 1;
    let main_tree_intact = clean_after && no_orphan_worktrees;

    let report = SwarmReport {
        agents,
        merged_count,
        main_tree_intact,
        root,
    };
    let _ = app.emit(DONE_EVENT, &report);
    Ok(report)
}

// ─── tests ──────────────────────────────────────────────────────────────────
//
// `swarm_mechanics_*` exercise the SAFETY-CRITICAL harness (worktree + junction
// + gate + cleanup + main-tree isolation) deterministically, without spawning an
// LLM — the edit step is a direct file write. `#[ignore]` by default because
// they shell out to git/npm and create worktrees; run on-demand with:
//   cargo test --manifest-path src-tauri/Cargo.toml swarm -- --ignored --nocapture
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_selection_by_extension() {
        assert_eq!(gate_for("src/lib/x.svelte"), Gate::Frontend);
        assert_eq!(gate_for("src/lib/x.ts"), Gate::Frontend);
        assert_eq!(gate_for("src-tauri/src/lib.rs"), Gate::Rust);
        assert_eq!(gate_for("docs/README.md"), Gate::None);
    }

    #[test]
    fn verdict_parsing() {
        assert_eq!(
            extract_verdict("sure: {\"accept\": true, \"reason\": \"ok\"}"),
            Some((true, "ok".to_string()))
        );
        assert_eq!(
            extract_verdict("{\"accept\":false,\"reason\":\"regression\"} trailing"),
            Some((false, "regression".to_string()))
        );
        assert_eq!(extract_verdict("no json here"), None);
    }

    /// End-to-end harness proof with synthetic edits (no LLM): a clean edit
    /// passes the frontend gate; a type-error edit fails it; the main checkout's
    /// porcelain count is unchanged throughout; no worktree leaks.
    #[tokio::test]
    #[ignore]
    async fn swarm_mechanics_discriminates_and_isolates() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        assert!(is_git_repo(&root).await, "test must run inside the rift repo");

        let target_rel = "src/lib/state/assistant/sessionLog.ts";
        let abs = root.join(target_rel);
        assert!(abs.is_file(), "target file missing: {target_rel}");

        let (_, before, _) = git(&root, &["status", "--porcelain"]).await;
        let before_count = before.lines().count();

        let proto = std::env::temp_dir().join("rift-swarm-test");
        let _ = std::fs::create_dir_all(&proto);
        let dummy_target = std::env::temp_dir().join("rift-swarm-test-target");

        // helper: isolate → write edit → gate → cleanup, return gate status.
        async fn one(
            root: &Path,
            proto: &Path,
            dummy_target: &Path,
            name: &str,
            target_rel: &str,
            append: &str,
        ) -> String {
            let wt = proto.join(name);
            let (ok, _, err) =
                git(root, &["worktree", "add", "--detach", &wt.to_string_lossy(), "HEAD"]).await;
            assert!(ok, "worktree add: {err}");
            let junction = junction_node_modules(root, &wt);
            // synthetic "agent": append directly to the worktree copy.
            let p = wt.join(target_rel);
            let mut content = std::fs::read_to_string(&p).unwrap();
            content.push_str(append);
            std::fs::write(&p, content).unwrap();
            let gr = run_gate(&wt, gate_for(target_rel), dummy_target).await;
            cleanup(root, &wt, junction.as_deref()).await;
            gr.status.to_string()
        }

        let clean = one(&root, &proto, &dummy_target, "t-clean", target_rel,
            "\n// [swarm-test] no-op comment\n").await;
        let broken = one(&root, &proto, &dummy_target, "t-broken", target_rel,
            "\nexport const __swarmTestErr: number = 'not a number';\n").await;

        let _ = std::fs::remove_dir_all(&proto);
        let _ = git(&root, &["worktree", "prune"]).await;

        let (_, after, _) = git(&root, &["status", "--porcelain"]).await;
        let after_count = after.lines().count();
        let (_, wt_list, _) = git(&root, &["worktree", "list"]).await;

        assert_eq!(clean, "pass", "clean edit should pass the frontend gate");
        assert_eq!(broken, "fail", "type-error edit should fail the frontend gate");
        assert_eq!(before_count, after_count, "main checkout must be unchanged");
        assert_eq!(wt_list.lines().count(), 1, "no worktree may leak");
    }
}
