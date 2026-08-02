//! Local Codex CLI discovery and ChatGPT sign-in bridge.
//!
//! Rift never reads Codex's auth cache. The official CLI/App Server owns both
//! credentials and token refresh; this module only runs its public commands.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatus {
    pub cli_present: bool,
    pub cli_version: Option<String>,
    pub logged_in: bool,
    pub ready: bool,
    pub summary: String,
}

fn is_windowsapps_path(path: &Path) -> bool {
    let normalized = path.to_string_lossy().to_ascii_lowercase().replace('/', "\\");
    normalized.contains("\\windowsapps\\")
}

fn is_runnable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    // The Desktop app's packaged helper is not a supported standalone CLI and
    // Windows blocks direct launches from WindowsApps. Do not imply it works.
    if is_windowsapps_path(path) {
        return false;
    }
    matches!(
        path.extension().and_then(|ext| ext.to_str()).map(str::to_ascii_lowercase).as_deref(),
        Some("exe") | Some("cmd") | Some("bat")
    )
}

fn codex_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("CODEX_CLI_PATH").map(PathBuf::from) {
        candidates.push(path);
    }
    #[cfg(windows)]
    {
        if let Ok(output) = std::process::Command::new("where.exe").arg("codex").output() {
            candidates.extend(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(PathBuf::from),
            );
        }
        if let Some(app_data) = std::env::var_os("APPDATA") {
            candidates.push(PathBuf::from(app_data).join("npm").join("codex.cmd"));
        }
    }
    #[cfg(not(windows))]
    {
        candidates.push(PathBuf::from("/usr/local/bin/codex"));
    }
    candidates.into_iter().find(|path| is_runnable(path)).into_iter().collect()
}

pub(super) fn resolve_codex_cli() -> Option<PathBuf> {
    codex_candidates().into_iter().next()
}

pub(super) fn command_for(exe: &Path, args: &[&str]) -> Command {
    #[cfg(windows)]
    if matches!(exe.extension().and_then(|ext| ext.to_str()), Some("cmd") | Some("bat")) {
        let mut command = Command::new("cmd.exe");
        command.args(["/d", "/s", "/c"]);
        // Keep the command path and its arguments separate. Pre-quoting this
        // into one `/c` string makes Windows escape the inner quotes, so a
        // space-free `codex.cmd --version` is treated as a literal command.
        command.arg(exe).args(args);
        command.kill_on_drop(true);
        return command;
    }

    let mut command = Command::new(exe);
    command.args(args).kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

async fn output_for(exe: &Path, args: &[&str]) -> Option<std::process::Output> {
    let mut command = command_for(exe, args);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    tokio::time::timeout(std::time::Duration::from_secs(10), command.output())
        .await
        .ok()
        .and_then(Result::ok)
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[tauri::command]
pub async fn assistant_codex_status() -> Result<CodexStatus, String> {
    let Some(exe) = resolve_codex_cli() else {
        return Ok(CodexStatus {
            cli_present: false,
            cli_version: None,
            logged_in: false,
            ready: false,
            summary: "Install the standalone Codex CLI to connect a ChatGPT account. The Windows Desktop bundle is not a runnable CLI.".into(),
        });
    };
    let version = output_for(&exe, &["--version"])
        .await
        .filter(|output| output.status.success())
        .map(|output| output_text(&output).trim().lines().next().unwrap_or_default().to_string())
        .filter(|version| !version.is_empty());
    let auth = output_for(&exe, &["login", "status"]).await;
    let auth_text = auth.as_ref().map(output_text).unwrap_or_default().to_ascii_lowercase();
    let logged_in = auth.as_ref().is_some_and(|output| output.status.success())
        && (auth_text.contains("logged in") || auth_text.contains("authenticated") || auth_text.contains("chatgpt"));
    let summary = if logged_in {
        "Codex CLI is signed in. Rift can use its local App Server without copying credentials.".into()
    } else {
        "Codex CLI is installed but not signed in. Choose Connect to complete ChatGPT sign-in in the official CLI.".into()
    };
    Ok(CodexStatus {
        cli_present: true,
        cli_version: version,
        logged_in,
        ready: logged_in,
        summary,
    })
}

/// Start the official interactive Codex login. Its browser flow and credential
/// storage remain entirely owned by Codex, then the frontend re-probes status.
#[tauri::command]
pub fn assistant_codex_open_login() -> Result<(), String> {
    let exe = resolve_codex_cli().ok_or_else(|| {
        "No runnable Codex CLI was found. Install the standalone Codex CLI first; the Windows Desktop bundle cannot host Rift turns.".to_string()
    })?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        if matches!(exe.extension().and_then(|ext| ext.to_str()), Some("cmd") | Some("bat")) {
            std::process::Command::new("cmd.exe")
                .args(["/d", "/s", "/c"])
                .arg(format!("\"\"{}\" login\"", exe.display()))
                .creation_flags(CREATE_NEW_CONSOLE)
                .spawn()
                .map(|_| ())
                .map_err(|error| format!("failed to start Codex sign-in: {error}"))
        } else {
            std::process::Command::new(exe)
                .arg("login")
                .creation_flags(CREATE_NEW_CONSOLE)
                .spawn()
                .map(|_| ())
                .map_err(|error| format!("failed to start Codex sign-in: {error}"))
        }
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(exe)
            .arg("login")
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to start Codex sign-in: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::is_windowsapps_path;
    use std::path::Path;

    #[test]
    fn packaged_windowsapps_helpers_are_never_accepted() {
        assert!(is_windowsapps_path(Path::new("C:\\Program Files\\WindowsApps\\OpenAI.Codex\\codex.exe")));
    }
}
