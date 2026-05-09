// Phase 1j — port of Rift Project/Services/UpdateService.cs.
//
// Wraps `velopack::UpdateManager`. Source resolution mirrors WPF:
//   1. RIFT_UPDATE_FEED env var → local FileSource (offline dev / testing)
//   2. GITHUB_REPO_URL constant → AutoSource (resolves Github releases.json)
//   3. Neither → no-op (CheckForUpdates returns Ok(None))
//
// AutoSource uses the URL to detect the host (github.com → Github API). The
// 0.0.x velopack-rust crate doesn't expose a dedicated GithubSource type, so
// AutoSource is the canonical hook for github.com hosts.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const GITHUB_REPO_URL: &str = "https://github.com/Blazzer10200/rift-tauri";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub release_name: String,
}

pub struct UpdateService {
    mgr: Option<velopack::UpdateManager>,
}

impl UpdateService {
    /// Construct w/ source-resolution priority matching WPF:
    /// `RIFT_UPDATE_FEED` (local dir) > `GITHUB_REPO_URL` constant > none.
    pub fn new() -> Self {
        let mgr = match resolve_manager() {
            Ok(m) => m,
            Err(e) => {
                log::warn!("UpdateService init: {e}");
                None
            }
        };
        Self { mgr }
    }

    pub fn current_version(&self) -> Option<String> {
        self.mgr.as_ref().map(|m| m.get_current_version_as_string())
    }

    /// Check for an update. Returns Ok(None) when no source is configured or
    /// no update is available. Network / auth errors get logged + swallowed
    /// so the UI banner stays hidden on dev boxes.
    pub fn check(&self) -> Result<Option<UpdateInfoDto>, String> {
        let Some(mgr) = self.mgr.as_ref() else { return Ok(None) };
        match mgr.check_for_updates() {
            Ok(velopack::UpdateCheck::UpdateAvailable(info)) => {
                let asset = AsRef::<velopack::VelopackAsset>::as_ref(&info);
                Ok(Some(UpdateInfoDto {
                    version: asset.Version.clone(),
                    release_name: asset.FileName.clone(),
                }))
            }
            Ok(_) => Ok(None),
            Err(e) => {
                log::warn!("UpdateService.check: {e}");
                Ok(None)
            }
        }
    }
}

impl Default for UpdateService {
    fn default() -> Self { Self::new() }
}

fn resolve_manager() -> Result<Option<velopack::UpdateManager>, String> {
    if let Ok(local) = std::env::var("RIFT_UPDATE_FEED") {
        let p = Path::new(&local);
        if p.is_dir() {
            let src = velopack::sources::FileSource::new(p);
            return velopack::UpdateManager::new(src, None, None)
                .map(Some)
                .map_err(|e| format!("UpdateManager(local feed): {e}"));
        }
    }
    if !GITHUB_REPO_URL.is_empty() {
        let src = velopack::sources::AutoSource::new(GITHUB_REPO_URL);
        return velopack::UpdateManager::new(src, None, None)
            .map(Some)
            .map_err(|e| format!("UpdateManager(github): {e}"));
    }
    Ok(None)
}
