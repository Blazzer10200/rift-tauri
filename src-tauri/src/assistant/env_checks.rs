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

/// winget package id for an optional host tool. `npm` ships with Node, so it maps
/// to the Node package; `cargo` comes from rustup. Returns `None` for unknown keys.
#[cfg(windows)]
fn winget_id(key: &str) -> Option<&'static str> {
    Some(match key {
        "git" => "Git.Git",
        "node" | "npm" => "OpenJS.NodeJS.LTS",
        "cargo" => "Rustlang.Rustup",
        "code" => "Microsoft.VisualStudioCode",
        _ => return None,
    })
}

/// Install a missing optional host tool via winget, in a VISIBLE console so the
/// user sees the UAC prompt + download progress and can react if winget asks for
/// agreements. We deliberately do NOT install silently/in-process: silent winget
/// can stall on a hidden prompt, and a fresh install needs a new shell to pick up
/// PATH changes anyway. After this returns the frontend re-probes `environment_check`
/// (the user reopens Settings / clicks re-probe once the console finishes).
///
/// winget is built into Windows 11; if it's absent we surface an actionable error.
#[tauri::command]
pub async fn install_local_tool(key: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let id = winget_id(&key).ok_or_else(|| format!("unknown tool '{key}'"))?;
        if !which_on_path("winget") {
            return Err(
                "Windows Package Manager (winget) isn't available. Update \"App Installer\" \
                 from the Microsoft Store, then try again — or install the tool manually."
                    .to_string(),
            );
        }
        // Launch winget in its own console window via PowerShell Start-Process so
        // the user sees progress/UAC. `-e` exact-id match; accept source+package
        // agreements so it doesn't block on the first-run EULA prompt.
        let inner = format!(
            "winget install --id {id} -e --accept-source-agreements --accept-package-agreements"
        );
        let arg = format!(
            "Start-Process -FilePath 'powershell' -ArgumentList @('-NoLogo','-NoProfile','-Command',\"{inner}; Write-Host ''; Write-Host 'Done — you can close this window.' -ForegroundColor Green; Read-Host 'Press Enter to close'\")"
        );
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &arg])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Couldn't launch installer: {e}"))
    }
    #[cfg(not(windows))]
    {
        let _ = key;
        Err("Automatic install is only supported on Windows.".to_string())
    }
}
