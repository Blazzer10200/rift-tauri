//! R5 (per `docs/design/assistant-mod-split.md`) — environment observation:
//! optional host-tool PATH checks. Lifted verbatim from `assistant/mod.rs`
//! 2026-06-09. Config load/save stays on the parent (R2) — reached via `super::`.

use std::process::Stdio;

use serde::Serialize;

/// `true` if `program` resolves via `where`/`which` (PATHEXT-aware on Windows).
fn which_on_path(program: &str) -> bool {
    let (cmd_name, args): (&str, &[&str]) = if cfg!(windows) {
        ("where.exe", &[program])
    } else {
        ("which", &[program])
    };
    let mut cmd = std::process::Command::new(cmd_name);
    cmd.args(args).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

/// Which optional host tools resolve on PATH. Rift works without these, but
/// individual features need them: the git tools need `git`; the "Open in VS
/// Code" affordance needs `code`. Surfaced in Settings → About → Local tools
/// and used to hide dead affordances.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInfo {
    pub git: bool,
    pub node: bool,
    pub npm: bool,
    pub cargo: bool,
    pub code: bool,
}

/// Probe optional host tools. Pure observation — never spawns a tool, only asks
/// `where`/`which` whether each is resolvable. Blocking probes are offloaded so
/// the UI thread never stalls on a slow PATH scan.
#[tauri::command]
pub async fn environment_check() -> EnvironmentInfo {
    tokio::task::spawn_blocking(|| EnvironmentInfo {
        git: which_on_path("git"),
        node: which_on_path("node"),
        npm: which_on_path("npm"),
        cargo: which_on_path("cargo"),
        code: which_on_path("code"),
    })
    .await
    .unwrap_or(EnvironmentInfo { git: false, node: false, npm: false, cargo: false, code: false })
}
