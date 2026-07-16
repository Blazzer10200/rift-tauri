//! Shell-descendant enumeration for the ActivityHud — lists the live shell
//! processes (cmd/powershell/pwsh/bash) running under a streaming `claude`
//! child so the frontend can show them and offer a per-PID kill. Also the
//! safety gate for that kill: `is_descendant` verifies a client-supplied PID
//! actually belongs to the tracked CLI subtree before anything is killed
//! (never trust a bare PID, never kill by image name).
//!
//! Snapshotting is whole-system (that's how process tables work on every OS
//! sysinfo supports) but refresh is narrowed to pid/ppid/cmd/start-time only —
//! no per-process CPU/memory/disk probes. Callers run this on the blocking
//! pool (`spawn_blocking`); a snapshot is milliseconds, not free.

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// One live shell under the CLI child, as shown in the HUD.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ShellRow {
    pub pid: u32,
    /// Executable name (`pwsh.exe`), display fallback when `cmd` is empty.
    pub exe: String,
    /// Full command line joined with spaces (frontend trims for display).
    pub cmd: String,
    /// Process start, seconds since UNIX epoch (sysinfo granularity).
    pub started_at: u64,
}

/// Only TOPMOST shells become rows: a `cmd.exe` that npm spawned under an
/// already-listed `bash.exe` is an implementation detail of the listed row,
/// and killing the topmost shell tree-kills it anyway.
fn is_shell_exe(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "cmd.exe" | "powershell.exe" | "pwsh.exe" | "bash.exe" | "sh.exe" | "zsh.exe"
            | "cmd" | "powershell" | "pwsh" | "bash" | "sh" | "zsh"
    )
}

/// The CLI's own `rift-tauri --mcp` grandchild (and anything under it) is
/// Rift infrastructure, not user work — never a row, never descended into.
fn is_rift_infra(name: &str) -> bool {
    name.to_ascii_lowercase().starts_with("rift-tauri")
}

struct Snapshot {
    /// pid → (ppid, exe name, cmdline, start epoch secs)
    procs: std::collections::HashMap<u32, (Option<u32>, String, String, u64)>,
    /// ppid → child pids
    children: std::collections::HashMap<u32, Vec<u32>>,
}

fn snapshot() -> Snapshot {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
    );
    let mut procs = std::collections::HashMap::new();
    let mut children: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for (pid, p) in sys.processes() {
        let pid = pid.as_u32();
        let ppid = p.parent().map(|pp| pp.as_u32());
        let name = p.name().to_string_lossy().into_owned();
        let cmd = p
            .cmd()
            .iter()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        procs.insert(pid, (ppid, name, cmd, p.start_time()));
        if let Some(pp) = ppid {
            children.entry(pp).or_default().push(pid);
        }
    }
    Snapshot { procs, children }
}

/// Walk the subtree under `root_pid`, collecting the topmost shell processes.
/// Recursion stops at a shell (its descendants belong to its row) and at Rift
/// infrastructure. Rows are sorted oldest-first for stable HUD order.
pub(crate) fn shell_rows(root_pid: u32) -> Vec<ShellRow> {
    let snap = snapshot();
    let mut rows = Vec::new();
    let mut stack: Vec<u32> = snap.children.get(&root_pid).cloned().unwrap_or_default();
    while let Some(pid) = stack.pop() {
        let Some((_, name, cmd, started_at)) = snap.procs.get(&pid) else { continue };
        if is_rift_infra(name) {
            continue;
        }
        if is_shell_exe(name) {
            rows.push(ShellRow {
                pid,
                exe: name.clone(),
                cmd: if cmd.trim().is_empty() { name.clone() } else { cmd.clone() },
                started_at: *started_at,
            });
            continue; // topmost shell — don't descend into its children
        }
        if let Some(kids) = snap.children.get(&pid) {
            stack.extend(kids.iter().copied());
        }
    }
    rows.sort_by_key(|r| (r.started_at, r.pid));
    rows
}

/// Kill-gate: is `pid` anywhere in the live subtree under `root_pid`?
/// A fresh snapshot per call — the answer must reflect NOW, not the last poll
/// (PIDs recycle; a stale map could bless an unrelated process).
pub(crate) fn is_descendant(root_pid: u32, pid: u32) -> bool {
    if pid == root_pid || pid == 0 {
        return false; // the CLI child itself is assistant_stop's job, not ours
    }
    let snap = snapshot();
    let mut stack: Vec<u32> = snap.children.get(&root_pid).cloned().unwrap_or_default();
    while let Some(p) = stack.pop() {
        if p == pid {
            return true;
        }
        if let Some(kids) = snap.children.get(&p) {
            stack.extend(kids.iter().copied());
        }
    }
    false
}
