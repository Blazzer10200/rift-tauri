//! Tauri command surface — split per domain (#20).
//!
//! lib.rs's `invoke_handler!` references these via `commands::*`. Each domain
//! file owns its #[tauri::command] fns + private helpers. Cross-domain helpers
//! (path-traversal guard, fingerprint pin guard, log-basename) live here.

pub mod assistant;
pub mod browser;
pub mod profile;
pub mod sftp;
pub mod sync;
pub mod update;

pub use assistant::*;
pub use browser::*;
pub use profile::*;
pub use sftp::*;
pub use sync::*;
pub use update::*;

/// Audit H11: reject paths that can escape via `..`. Rift's identity files
/// and key-output dirs come from JS; without this, a malicious profile
/// could point key reads at arbitrary filesystem locations.
pub(crate) fn reject_path_traversal(p: &std::path::Path, label: &str) -> Result<(), String> {
    use std::path::Component;
    for c in p.components() {
        if matches!(c, Component::ParentDir) {
            return Err(format!("{label}: '..' components not allowed"));
        }
    }
    Ok(())
}

/// #10: defense-in-depth guard against silent TOFU. Sync entry paths
/// (`scan_drift`, `start_autosync`) + every SFTP-touching command must refuse
/// to connect when no fingerprint is pinned in the profile -- the frontend's
/// `probe_server_fingerprint` + user-confirm flow is the only sanctioned
/// way to capture a host key.
pub(crate) fn require_pinned_fingerprint(server_key: &str, fingerprint: Option<&str>) -> Result<(), String> {
    if fingerprint.unwrap_or("").trim().is_empty() {
        return Err(format!(
            "server '{server_key}' has no pinned fingerprint -- run probe_server_fingerprint + \
             set_server_fingerprint (AddServer dialog) to capture and confirm the host key first"
        ));
    }
    Ok(())
}

pub(crate) fn basename_for_log(p: &str) -> String {
    let norm = p.replace('\\', "/");
    norm.rsplit('/').next().unwrap_or(p).to_string()
}
