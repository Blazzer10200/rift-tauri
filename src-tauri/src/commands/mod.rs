//! Tauri command surface — split per domain (#20).
//!
//! lib.rs's `invoke_handler!` references these via `commands::*`. Each domain
//! file owns its #[tauri::command] fns + private helpers. Cross-domain helpers
//! (path-traversal guard, fingerprint pin guard, log-basename) live here.

pub mod assistant;
pub mod browser;
pub mod update;

pub use assistant::*;
pub use browser::*;
pub use update::*;

/// #248: surface frontend-side failures (wire/runtime errors) to the DiagBus so
/// the panic-hook log path + any diagnostics consumers see them. Pre-fix these
/// only `console.error`'d in the WebView console. Emits as System/Error. (Moved
/// here from the deleted `commands::sync` module during the pure-assistant rip.)
/// Open a path in VS Code. The opener plugin's `openWith` can't launch VS Code
/// on Windows (its CLI is `code.cmd`, which `Command::new("code")` won't resolve
/// without a shell) — so spawn through `cmd /C` on Windows, direct `code` elsewhere.
#[tauri::command]
pub fn open_in_vscode(path: String) -> Result<(), String> {
    // The Windows path runs through `cmd /C`, which re-parses metacharacters
    // *after* Rust's arg quoting — so `&`/`|`/`<`/`>`/`^`/`%`/`"` (and control
    // chars) could chain a second command. None are needed to open a file; reject
    // them rather than try to escape cmd.exe's quoting (F19).
    #[cfg(windows)]
    if path.bytes().any(|b| matches!(b, b'&' | b'|' | b'<' | b'>' | b'^' | b'%' | b'"') || b < 0x20) {
        return Err("path contains characters that can't be passed safely to the shell".into());
    }
    #[cfg(windows)]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "code", &path]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut c = std::process::Command::new("code");
        c.arg(&path);
        c
    };
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't launch VS Code (is `code` on PATH?): {e}"))
}
