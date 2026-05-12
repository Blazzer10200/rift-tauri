//! Embedded terminal — S37 (Path A: generic PTY-backed shell, user runs
//! `claude` themselves to launch Claude Code; no SDK / API-key handling).
//!
//! Design: single global session per Rift instance (MVP). Tauri commands open
//! / write / resize / kill the PTY; a blocking reader thread streams bytes to
//! the frontend via `term:data` events. xterm.js renders.
//!
//! Shell autodetect order on Windows: Git Bash → pwsh → powershell → cmd.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;

use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellInfo {
    pub id: String,
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TermDataPayload {
    pub id: String,
    pub chunk: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TermExitPayload {
    pub id: String,
}

/// Returned from term_spawn so the frontend can label the tab w/o a second
/// round-trip (avoids a flash of "Terminal" before the shell name resolves).
#[derive(Debug, Clone, Serialize)]
pub struct SessionStartInfo {
    pub id: String,
    pub shell_id: String,
    pub shell_label: String,
}

/// One live PTY session.
pub struct TermSession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

/// Tauri-managed registry of live PTY sessions. MVP keeps it simple: a single
/// global "main" session keyed by id; future multi-tab can grow this map.
#[derive(Default)]
pub struct TerminalState {
    pub sessions: Mutex<HashMap<String, TermSession>>,
}

// ─── Shell autodetect ────────────────────────────────────────────────────────

fn known_shells() -> Vec<ShellInfo> {
    let mut out = Vec::new();

    // Git Bash — Blazzer's default. Check both 64-bit and 32-bit install dirs.
    let git_bash_candidates = [
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ];
    let git_bash = git_bash_candidates.iter().find(|p| std::path::Path::new(p).exists());
    out.push(ShellInfo {
        id: "git-bash".into(),
        label: "Git Bash".into(),
        program: git_bash.map(|s| s.to_string()).unwrap_or_default(),
        args: vec!["--login".into(), "-i".into()],
        available: git_bash.is_some(),
    });

    // PowerShell 7+ (pwsh) — modern, cross-platform.
    let pwsh = which_exe("pwsh.exe");
    out.push(ShellInfo {
        id: "pwsh".into(),
        label: "PowerShell 7".into(),
        program: pwsh.clone().unwrap_or_default(),
        args: vec!["-NoLogo".into()],
        available: pwsh.is_some(),
    });

    // Windows PowerShell 5.1 — always present on Win10+.
    let ps5 = which_exe("powershell.exe").or_else(|| {
        let p = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";
        if std::path::Path::new(p).exists() { Some(p.into()) } else { None }
    });
    out.push(ShellInfo {
        id: "powershell".into(),
        label: "Windows PowerShell".into(),
        program: ps5.clone().unwrap_or_default(),
        args: vec!["-NoLogo".into()],
        available: ps5.is_some(),
    });

    // cmd.exe — last-resort fallback. Always present.
    let cmd = r"C:\Windows\System32\cmd.exe".to_string();
    out.push(ShellInfo {
        id: "cmd".into(),
        label: "Command Prompt".into(),
        program: cmd.clone(),
        args: vec![],
        available: std::path::Path::new(&cmd).exists(),
    });

    out
}

/// Walk PATH for an executable. Returns the first hit, or None.
fn which_exe(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn default_shell() -> Option<ShellInfo> {
    known_shells().into_iter().find(|s| s.available)
}

// ─── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn term_list_shells() -> Vec<ShellInfo> {
    known_shells()
}

#[tauri::command]
pub async fn term_spawn(
    app: AppHandle,
    shell_id: Option<String>,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
) -> Result<SessionStartInfo, String> {
    let shells = known_shells();
    let shell = match shell_id {
        Some(id) => shells.into_iter().find(|s| s.id == id && s.available)
            .ok_or_else(|| format!("shell '{}' not available", id))?,
        None => default_shell().ok_or_else(|| "no shell available".to_string())?,
    };

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| format!("openpty failed: {e}"))?;

    let mut builder = CommandBuilder::new(&shell.program);
    for a in &shell.args {
        builder.arg(a);
    }
    if let Some(c) = cwd.as_ref().filter(|s| !s.is_empty()) {
        if std::path::Path::new(c).is_dir() {
            builder.cwd(c);
        }
    }
    // Pass through HOME / USERPROFILE so the shell finds the user's profile.
    if let Some(home) = std::env::var_os("USERPROFILE") {
        builder.env("HOME", &home);
        builder.env("USERPROFILE", &home);
    }
    builder.env("TERM", "xterm-256color");

    let mut child = pair.slave.spawn_command(builder)
        .map_err(|e| format!("spawn failed: {e}"))?;
    let killer = child.clone_killer();

    let reader = pair.master.try_clone_reader()
        .map_err(|e| format!("reader clone failed: {e}"))?;
    let writer = pair.master.take_writer()
        .map_err(|e| format!("writer take failed: {e}"))?;

    // ID: timestamp-based. Single MVP session typically named "main" but we
    // keep the map keyed by id for future multi-tab.
    let id = format!("term-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));

    // Output reader thread. portable-pty's reader is sync `Read`, so we use
    // std::thread, not tokio. Emit raw chunks; xterm batches internally.
    let app_clone = app.clone();
    let id_for_thread = id.clone();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app_clone.emit(
                        "term:data",
                        TermDataPayload { id: id_for_thread.clone(), chunk },
                    );
                }
                Err(_) => break,
            }
        }
        // Child exited or reader closed. Notify frontend and clean up the
        // session entry so a fresh spawn re-creates it.
        let _ = app_clone.emit(
            "term:exit",
            TermExitPayload { id: id_for_thread.clone() },
        );
        if let Some(state) = app_clone.try_state::<TerminalState>() {
            if let Ok(mut map) = state.sessions.lock() {
                map.remove(&id_for_thread);
            }
        }
        // Reap the child to avoid zombies on Unix; harmless on Windows.
        let _ = child.wait();
    });

    let session = TermSession {
        master: pair.master,
        writer,
        killer,
    };
    let state = app.state::<TerminalState>();
    let mut map = state.sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    map.insert(id.clone(), session);

    Ok(SessionStartInfo {
        id,
        shell_id: shell.id,
        shell_label: shell.label,
    })
}

#[tauri::command]
pub fn term_write(
    state: tauri::State<'_, TerminalState>,
    id: String,
    data: String,
) -> Result<(), String> {
    let mut map = state.sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    let session = map.get_mut(&id).ok_or_else(|| format!("session '{}' not found", id))?;
    session.writer.write_all(data.as_bytes()).map_err(|e| format!("write failed: {e}"))?;
    session.writer.flush().map_err(|e| format!("flush failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn term_resize(
    state: tauri::State<'_, TerminalState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let map = state.sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    let session = map.get(&id).ok_or_else(|| format!("session '{}' not found", id))?;
    session.master
        .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| format!("resize failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn term_kill(state: tauri::State<'_, TerminalState>, id: String) -> Result<(), String> {
    let mut map = state.sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;
    if let Some(mut session) = map.remove(&id) {
        let _ = session.killer.kill();
    }
    Ok(())
}

#[tauri::command]
pub fn term_default_cwd(profile_local_root: Option<String>) -> String {
    if let Some(p) = profile_local_root.as_ref().filter(|s| !s.is_empty()) {
        if std::path::Path::new(p).is_dir() { return p.clone(); }
    }
    std::env::var("USERPROFILE")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_else(|| "C:\\".into())
}

/// Kill all live sessions. Called from the window close hook so child shells
/// don't outlive the app.
pub fn kill_all(state: &TerminalState) {
    if let Ok(mut map) = state.sessions.lock() {
        for (_id, mut session) in map.drain() {
            let _ = session.killer.kill();
        }
    }
}
