//! Updater — GH-release-API path (v0.4.34+).
//!
//! Replaced `tauri-plugin-updater` (had a brief 2026-05-26 → 2026-05-27 run).
//! That plugin's ed25519 signature check cannot be disabled per its docs —
//! losing the signing key bricks every installed client forever, which is what
//! prompted the v0.4.33 key rotation + manual reinstall. This module hits the
//! public GitHub Releases API instead, semver-compares the latest tag against
//! the running build, and (on user confirm) opens the Setup.exe asset URL in
//! the user's browser via `tauri-plugin-opener`. Install = standard NSIS
//! wizard; Tauri's NSIS template handles "close running app" prompt + relaunch.
//!
//! No signing key. No `latest.json`. No `*.sig` files. Lose the build machine
//! and the next release just ships from a different one.

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

/// Check GitHub for a newer release.
///
/// Return contract — the frontend distinguishes these three outcomes:
///   * `Ok(Some(info))` — a newer release exists and has an installer asset.
///   * `Ok(None)`       — genuinely nothing to offer: up to date, no published
///                        release yet (404), or a newer release with no
///                        `*-setup.exe` asset. Frontend shows "up to date".
///   * `Err(msg)`       — the check itself FAILED (offline, timed out, rate-
///                        limited, server error, unparseable). Frontend shows
///                        the error card. Previously every one of these
///                        returned `Ok(None)`, so a failed check looked
///                        identical to "you're current" — the core reason
///                        updates felt silently broken.
#[tauri::command]
pub async fn check_for_updates() -> Result<Option<UpdateInfoDto>, String> {
    let url = format!("https://api.github.com/repos/{RELEASES_REPO}/releases/latest");
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| {
            log::warn!("update check (client): {e}");
            format!("Couldn't initialize the update client: {e}")
        })?;
    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| {
            log::warn!("update check (network): {e}");
            // reqwest's Display is noisy — give the user a short, actionable reason.
            if e.is_timeout() {
                "Update check timed out — GitHub didn't respond in time.".to_string()
            } else if e.is_connect() {
                "Couldn't reach GitHub — check your internet connection.".to_string()
            } else {
                format!("Network error while checking for updates: {e}")
            }
        })?;
    let status = resp.status();
    // 404 = the releases repo has no published `latest` release. That's "nothing
    // to update to", not a failure — treat as up to date.
    if status == reqwest::StatusCode::NOT_FOUND {
        log::info!("update check: no published release (404)");
        return Ok(None);
    }
    if !status.is_success() {
        log::warn!("update check: HTTP {status}");
        // 403 on the unauthenticated API is almost always the rate limit.
        let hint = if status == reqwest::StatusCode::FORBIDDEN {
            " — GitHub API rate limit, try again later"
        } else {
            ""
        };
        return Err(format!("GitHub returned HTTP {status}{hint}."));
    }
    let release: GhRelease = resp.json().await.map_err(|e| {
        log::warn!("update check (parse): {e}");
        format!("Couldn't read the release data from GitHub: {e}")
    })?;
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

/// Progress payload for the `update://download-progress` event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
}

/// Stream the installer to a temp file, emitting `update://download-progress`
/// as bytes arrive, then return the local path. The frontend launches it via
/// the opener plugin — from that point the install is byte-identical to the
/// user running the browser download (NSIS closes Rift, installs, relaunches).
/// The frontend falls back to opening the URL in the browser on any failure,
/// so this path never regresses below the v0.4.36 browser handoff.
#[tauri::command]
pub async fn download_update(app: tauri::AppHandle, url: String) -> Result<String, String> {
    use futures::StreamExt;
    use std::io::Write;
    use tauri::Emitter;

    // Only ever download from the release host we control.
    if !(url.starts_with("https://github.com/")
        || url.starts_with("https://objects.githubusercontent.com/"))
    {
        return Err(format!("Refusing to download from an unexpected host: {url}"));
    }
    // Derive a safe filename from the URL's last segment.
    let fname = url.rsplit('/').next().unwrap_or("");
    if !fname.ends_with(".exe") || fname.contains('\\') || fname.contains(':') {
        return Err("Download URL does not end in a plain .exe filename.".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| format!("Couldn't initialize the download client: {e}"))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed to start: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);

    let dir = std::env::temp_dir().join("rift-update");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create temp dir: {e}"))?;
    let path = dir.join(fname);
    let mut file =
        std::fs::File::create(&path).map_err(|e| format!("Couldn't create installer file: {e}"))?;

    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download interrupted: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Couldn't write installer: {e}"))?;
        downloaded += chunk.len() as u64;
        let _ = app.emit(
            "update://download-progress",
            DownloadProgress { downloaded, total },
        );
    }
    file.flush().map_err(|e| format!("Couldn't finalize installer: {e}"))?;
    drop(file);

    if total > 0 && downloaded != total {
        return Err(format!(
            "Download incomplete: got {downloaded} of {total} bytes."
        ));
    }
    Ok(path.to_string_lossy().into_owned())
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
