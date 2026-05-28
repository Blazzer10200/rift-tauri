//! Updater — v0.4.34+ GH-release-API path.
//!
//! Replaced `tauri-plugin-updater` (2026-05-26 → 2026-05-27 brief lifetime).
//! That plugin's ed25519 signature check cannot be disabled per its docs —
//! losing the signing key bricks every installed client forever, which is what
//! prompted the v0.4.33 key rotation + buddy reinstall. This module hits the
//! public GitHub Releases API instead, semver-compares the latest tag against
//! the running build, and (on user confirm) opens the Setup.exe asset URL in
//! the user's browser via `tauri-plugin-opener`. Install = standard NSIS
//! wizard; Tauri's NSIS template handles "close running app" prompt + relaunch.
//!
//! No signing key. No `latest.json`. No `*.sig` files (v0.4.34 release.ps1
//! still publishes them one last time as a bridge so v0.4.33 clients can
//! receive v0.4.34 via the OLD path; v0.4.35+ release pipeline can drop them).

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const RELEASES_REPO: &str = "Blazzer10200/rift-releases";
const USER_AGENT: &str = concat!("rift-tauri/", env!("CARGO_PKG_VERSION"));
const SETUP_ASSET_SUFFIX: &str = "-setup.exe";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub release_name: String,
    pub size_bytes: u64,
    pub notes_markdown: String,
    pub release_url: String,
    pub download_url: String,
    pub published_at: String,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    published_at: Option<String>,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn check_for_updates() -> Result<Option<UpdateInfoDto>, String> {
    let url = format!("https://api.github.com/repos/{RELEASES_REPO}/releases/latest");
    let client = match reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("update check (client): {e}");
            return Ok(None);
        }
    };
    let resp = match client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("update check (network): {e}");
            return Ok(None);
        }
    };
    if !resp.status().is_success() {
        log::warn!("update check: HTTP {}", resp.status());
        return Ok(None);
    }
    let release: GhRelease = match resp.json().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("update check (parse): {e}");
            return Ok(None);
        }
    };
    let remote = release.tag_name.trim_start_matches('v').to_string();
    let local = env!("CARGO_PKG_VERSION");
    if !is_newer(&remote, local) {
        return Ok(None);
    }
    let setup = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(SETUP_ASSET_SUFFIX));
    let (download_url, size_bytes, release_name) = match setup {
        Some(a) => (a.browser_download_url.clone(), a.size, a.name.clone()),
        None => {
            log::warn!(
                "update check: latest release {} has no *-setup.exe asset",
                release.tag_name
            );
            return Ok(None);
        }
    };
    Ok(Some(UpdateInfoDto {
        version: remote,
        release_name,
        size_bytes,
        notes_markdown: release.body.unwrap_or_default(),
        release_url: release.html_url,
        download_url,
        published_at: release.published_at.unwrap_or_default(),
    }))
}

/// Parse `"0.4.33"` / `"0.4.33-alpha"` / `"v0.4.33"` into a comparable tuple.
fn parse_version(s: &str) -> Option<(u32, u32, u32, Option<String>)> {
    let s = s.trim().trim_start_matches('v');
    let (core, pre) = match s.find('-') {
        Some(i) => (&s[..i], Some(s[i + 1..].to_string())),
        None => (s, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch, pre))
}

fn is_newer(remote: &str, local: &str) -> bool {
    let (Some(r), Some(l)) = (parse_version(remote), parse_version(local)) else {
        return false;
    };
    match (r.0, r.1, r.2).cmp(&(l.0, l.1, l.2)) {
        Ordering::Greater => true,
        Ordering::Less => false,
        // X.Y.Z equal → release outranks any pre-release; otherwise lex pre.
        Ordering::Equal => match (&r.3, &l.3) {
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (None, None) => false,
            (Some(rp), Some(lp)) => rp > lp,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newer_patch_minor_major() {
        assert!(is_newer("0.4.34", "0.4.33"));
        assert!(is_newer("0.5.0", "0.4.99"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.4.33", "0.4.33"));
        assert!(!is_newer("0.4.32", "0.4.33"));
    }
    #[test]
    fn double_digit_patch() {
        assert!(is_newer("0.4.10", "0.4.9"));
        assert!(!is_newer("0.4.9", "0.4.10"));
    }
    #[test]
    fn prerelease_rules() {
        assert!(is_newer("0.4.33", "0.4.33-alpha"));
        assert!(!is_newer("0.4.33-alpha", "0.4.33"));
        assert!(is_newer("0.4.33-beta", "0.4.33-alpha"));
        assert!(!is_newer("0.4.33-alpha", "0.4.33-alpha"));
    }
    #[test]
    fn v_prefix_stripped() {
        assert!(is_newer("v0.4.34", "0.4.33"));
        assert!(is_newer("0.4.34", "v0.4.33"));
    }
    #[test]
    fn malformed_returns_false() {
        assert!(!is_newer("notaversion", "0.4.33"));
        assert!(!is_newer("0.4.33", "notaversion"));
        assert!(!is_newer("0.4", "0.4.33"));
    }
}
