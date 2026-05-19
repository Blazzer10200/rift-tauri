// Auto-updater service. Wraps `velopack::UpdateManager` w/ a custom
// `GithubSource` implementation since velopack-rust 0.0.1298 has no
// GithubSource type — `AutoSource` for an HTTP URL just delegates to
// `HttpSource`, which probes `<url>/releases.<channel>.json` as a flat
// static file. That's a 404 against a real GitHub release page.
//
// `GithubSource` mirrors what the .NET `Velopack.Sources.GithubSource` does:
// hit the GitHub REST API, find the newest release that matches the
// `allow_prerelease` policy, look up `releases.<channel>.json` in the
// release's asset list, download it, and cache asset URLs by filename so
// `download_release_entry` can resolve them later.
//
// Source resolution priority:
//   1. RIFT_UPDATE_FEED env var → local FileSource (offline dev / testing)
//   2. GitHub release-repo via GithubSource (production path)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use velopack::sources::{FileSource, UpdateSource};
use velopack::{VelopackAsset, VelopackAssetFeed, Error as VeloError};
use velopack::bundle::Manifest;

/// Public release repo. Source repo is private; releases publish here so
/// unauthenticated GithubSource fetches succeed without exposing source.
const GITHUB_OWNER: &str = "Blazzer10200";
const GITHUB_REPO: &str = "rift-releases";
/// Set true so alpha/beta tags are eligible for the "newest" pick. Match the
/// WPF GithubSource(prerelease:true) call site.
const ALLOW_PRERELEASE: bool = true;

const USER_AGENT: &str = concat!("Rift/", env!("CARGO_PKG_VERSION"));

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

    /// Check for an update. Returns Ok(None) when no source is configured or
    /// no update is available. Network errors get logged + swallowed so the
    /// UI banner stays hidden on dev boxes. Velopack's `check_for_updates`
    /// is blocking I/O — call from `spawn_blocking` context.
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

    /// Re-check, download, then apply + restart. Blocking I/O -- must be
    /// called from a `spawn_blocking` context. `apply_updates_and_restart`
    /// `exit(0)`s on success, so this only returns on error. Caller MUST
    /// stop autosync + tunnel BEFORE invoking -- in-flight uploads die when
    /// the process exits. The `apply_updates` Tauri command in `lib.rs`
    /// already handles this; direct callers of this method must do it
    /// themselves.
    pub fn apply(&self) -> Result<(), String> {
        let Some(mgr) = self.mgr.as_ref() else {
            return Err("no update source configured".into());
        };
        let info = match mgr.check_for_updates() {
            Ok(velopack::UpdateCheck::UpdateAvailable(info)) => info,
            Ok(_) => return Err("no update available".into()),
            Err(e) => return Err(format!("check_for_updates: {e}")),
        };
        mgr.download_updates(&info, None)
            .map_err(|e| format!("download_updates: {e}"))?;
        mgr.apply_updates_and_restart(&info)
            .map_err(|e| format!("apply_updates_and_restart: {e}"))?;
        Ok(())
    }
}

impl Default for UpdateService {
    fn default() -> Self { Self::new() }
}

fn resolve_manager() -> Result<Option<velopack::UpdateManager>, String> {
    // Local FileSource (RIFT_UPDATE_FEED) is dev-only — gated behind
    // `debug_assertions` so a release-build binary can't be tricked into
    // pointing at an attacker-controlled local update feed via env var.
    #[cfg(debug_assertions)]
    if let Ok(local) = std::env::var("RIFT_UPDATE_FEED") {
        let p = Path::new(&local);
        if p.is_dir() {
            let src = FileSource::new(p);
            return velopack::UpdateManager::new(src, None, None)
                .map(Some)
                .map_err(|e| format!("UpdateManager(local feed): {e}"));
        }
    }
    let src = GithubSource::new(GITHUB_OWNER, GITHUB_REPO, ALLOW_PRERELEASE);
    velopack::UpdateManager::new(src, None, None)
        .map(Some)
        .map_err(|e| format!("UpdateManager(github): {e}"))
}

// ─── GithubSource ──────────────────────────────────────────────────────────
//
// Implements `velopack::sources::UpdateSource` against the GitHub REST API.
// Fetches `/repos/{owner}/{repo}/releases?per_page=10`, picks the newest
// non-draft release whose `prerelease` flag matches `allow_prerelease`,
// downloads the `releases.{channel}.json` asset, and caches every asset's
// `browser_download_url` so `download_release_entry` can resolve nupkgs by
// filename later.

#[derive(Clone)]
pub struct GithubSource {
    owner: String,
    repo: String,
    allow_prerelease: bool,
    /// filename → browser_download_url, populated by get_release_feed.
    asset_urls: Arc<Mutex<HashMap<String, String>>>,
}

impl GithubSource {
    pub fn new(owner: &str, repo: &str, allow_prerelease: bool) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            allow_prerelease,
            asset_urls: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn http_get_string(url: &str) -> Result<String, VeloError> {
        ureq::get(url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| VeloError::Generic(format!("GET {url}: {e}")))?
            .body_mut()
            .read_to_string()
            .map_err(|e| VeloError::Generic(format!("read {url}: {e}")))
    }
}

impl UpdateSource for GithubSource {
    fn get_release_feed(
        &self,
        channel: &str,
        _app: &Manifest,
        _staged_user_id: &str,
    ) -> Result<VelopackAssetFeed, VeloError> {
        let api_url = format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=10",
            self.owner, self.repo
        );
        log::info!("GithubSource: querying {api_url}");
        let body = Self::http_get_string(&api_url)?;
        let releases: Vec<serde_json::Value> = serde_json::from_str(&body)
            .map_err(|e| VeloError::Generic(format!("parse releases JSON: {e}")))?;

        // Newest-first ordering is GitHub's default. Pick the first release
        // that's not a draft and matches the prerelease policy.
        let target = releases
            .iter()
            .find(|r| {
                let is_draft = r.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
                let is_pre = r.get("prerelease").and_then(|v| v.as_bool()).unwrap_or(false);
                !is_draft && (self.allow_prerelease || !is_pre)
            })
            .ok_or_else(|| {
                VeloError::Generic(format!(
                    "no eligible release in {}/{} (allow_prerelease={})",
                    self.owner, self.repo, self.allow_prerelease
                ))
            })?;

        let releases_name = format!("releases.{channel}.json");
        let assets = target
            .get("assets")
            .and_then(|v| v.as_array())
            .ok_or_else(|| VeloError::Generic("release has no assets array".into()))?;

        let mut url_map: HashMap<String, String> = HashMap::new();
        let mut feed_url: Option<String> = None;
        for a in assets {
            let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let url = a
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if name.is_empty() || url.is_empty() {
                continue;
            }
            if name.eq_ignore_ascii_case(&releases_name) {
                feed_url = Some(url.to_string());
            }
            url_map.insert(name.to_string(), url.to_string());
        }

        let feed_url = feed_url.ok_or_else(|| {
            VeloError::Generic(format!("{releases_name} not found in latest release assets"))
        })?;

        // Cache asset URLs so download_release_entry can resolve nupkgs by name.
        if let Ok(mut g) = self.asset_urls.lock() {
            *g = url_map;
        }

        log::info!("GithubSource: downloading feed from {feed_url}");
        let feed_json = Self::http_get_string(&feed_url)?;
        let feed: VelopackAssetFeed = serde_json::from_str(&feed_json)
            .map_err(|e| VeloError::Generic(format!("parse {releases_name}: {e}")))?;
        Ok(feed)
    }

    fn download_release_entry(
        &self,
        asset: &VelopackAsset,
        local_file: &str,
        progress_sender: Option<Sender<i16>>,
    ) -> Result<(), VeloError> {
        let url = self
            .asset_urls
            .lock()
            .ok()
            .and_then(|g| g.get(&asset.FileName).cloned())
            .ok_or_else(|| {
                VeloError::Generic(format!(
                    "asset '{}' not in cache — get_release_feed must be called first",
                    asset.FileName
                ))
            })?;

        log::info!("GithubSource: downloading {} from {url}", asset.FileName);
        velopack::download::download_url_to_file(&url, local_file, move |p| {
            if let Some(s) = &progress_sender {
                let _ = s.send(p);
            }
        })
        .map_err(|e| VeloError::Generic(format!("download {}: {e}", asset.FileName)))
    }

    fn clone_boxed(&self) -> Box<dyn UpdateSource> {
        Box::new(self.clone())
    }
}
