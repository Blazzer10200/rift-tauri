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
    /// so the UI banner stays hidden on dev boxes. Velopack's `check_for_updates`
    /// is blocking I/O — the wrapper at the Tauri command layer should call
    /// this from `spawn_blocking` so the runtime isn't stalled on a slow
    /// network probe.
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
    // SECURITY TODO (audit H4) — before public v14 ship:
    //   1. Get a code-signing cert (P12/PFX). Per CA/Browser Forum 2023-06,
    //      certs must live in HSM (USB or Azure Key Vault). For SmallTeam
    //      use Azure Artifact Signing (AAS, ~$120/yr) — eliminates on-disk key.
    //   2. CI: store P12 base64 in GH secret `CERT_P12_BASE64` + `CERT_PASSWORD`.
    //      Decode at job start, pass to `vpk pack --signTemplate
    //      "/td sha256 /fd sha256 /f cert.p12 /p {password}
    //       /tr http://timestamp.comodoca.com"`. Add `permissions: contents: write`.
    //   3. Embed Velopack-issued public key here via `UpdateOptions { public_key: Some("...") }`
    //      (third arg below). Without it, a compromised release host can ship
    //      a tampered binary and the auto-updater installs it silently.
    //   Rotation: revoke old cert at CA, update both GH secrets, retrigger release.
    //   SmartScreen reputation resets on cert change — AAS mitigates.
    //   Local FileSource path is dev-only (RIFT_UPDATE_FEED) — unsigned is OK there.
    //   Refs: docs.velopack.io/packaging/signing + docs.velopack.io/distributing/github-actions
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
