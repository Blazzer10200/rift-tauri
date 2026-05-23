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
//
// v0.4.13: UpdateService becomes a managed singleton (Tauri State) so the
// pending `UpdateInfo` survives between `download_update` and
// `apply_pending_update` cmds. Progress events stream during download.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use velopack::sources::{FileSource, UpdateSource};
use velopack::{UpdateInfo, VelopackAsset, VelopackAssetFeed, Error as VeloError};
use velopack::bundle::Manifest;

/// Public release repo. Source repo is private; releases publish here so
/// unauthenticated GithubSource fetches succeed without exposing source.
const GITHUB_OWNER: &str = "Blazzer10200";
const GITHUB_REPO: &str = "rift-releases";
/// Set true so alpha/beta tags are eligible for the "newest" pick. Match the
/// WPF GithubSource(prerelease:true) call site.
const ALLOW_PRERELEASE: bool = true;
/// Velopack channel identifier — matches the `--channel win` arg in
/// scripts/release.ps1. Currently informational (the value flows through the
/// installed manifest, not this constant), but kept here as the single source
/// of truth for "what channel does Rift ship on" on the client side.
#[allow(dead_code)]
const UPDATE_CHANNEL: &str = "win";

const USER_AGENT: &str = concat!("Rift/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub version: String,
    pub release_name: String,
    pub size_bytes: u64,
    pub notes_markdown: String,
    pub release_url: String,
    pub published_at: String,
}

/// GitHub-side release metadata not present on `VelopackAsset`. Cached during
/// `get_release_feed` so `UpdateService::check` can hand it back to the UI.
#[derive(Debug, Clone, Default)]
struct ReleaseMeta {
    html_url: String,
    published_at: String,
}

type ReleaseMetaCache = Arc<Mutex<Option<ReleaseMeta>>>;

struct Inner {
    mgr: Option<velopack::UpdateManager>,
    /// Shared w/ `GithubSource`. Populated when the source fetches a release.
    meta: ReleaseMetaCache,
    /// Held between `check`/`download` and `apply` so the same update plan is
    /// reused without a second roundtrip + re-download.
    pending: Option<UpdateInfo>,
}

pub struct UpdateService {
    inner: Mutex<Inner>,
}

impl UpdateService {
    pub fn new() -> Self {
        let meta: ReleaseMetaCache = Arc::new(Mutex::new(None));
        let mgr = match resolve_manager(meta.clone()) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("UpdateService init: {e}");
                None
            }
        };
        Self {
            inner: Mutex::new(Inner { mgr, meta, pending: None }),
        }
    }

    /// Check for an update. Returns Ok(None) when no source is configured or
    /// no update is available. Network errors get logged + swallowed so the
    /// UI banner stays hidden on dev boxes. Velopack's `check_for_updates`
    /// is blocking I/O — call from `spawn_blocking` context.
    pub fn check(&self) -> Result<Option<UpdateInfoDto>, String> {
        let mut g = self.inner.lock().map_err(|_| "update mutex poisoned".to_string())?;
        let Some(mgr) = g.mgr.as_ref() else { return Ok(None) };
        match mgr.check_for_updates() {
            Ok(velopack::UpdateCheck::UpdateAvailable(info)) => {
                let asset: &VelopackAsset = info.as_ref();
                let meta = g.meta.lock().ok().and_then(|m| m.clone()).unwrap_or_default();
                let dto = UpdateInfoDto {
                    version: asset.Version.clone(),
                    release_name: asset.FileName.clone(),
                    size_bytes: asset.Size,
                    notes_markdown: asset.NotesMarkdown.clone(),
                    release_url: meta.html_url,
                    published_at: meta.published_at,
                };
                g.pending = Some(info);
                Ok(Some(dto))
            }
            Ok(_) => {
                g.pending = None;
                Ok(None)
            }
            Err(e) => {
                log::warn!("UpdateService.check: {e}");
                Ok(None)
            }
        }
    }

    /// Download the pending update package, streaming `0..=100` progress
    /// ticks through `progress`. Call `check` first to populate the pending
    /// plan. Blocking I/O — must run on `spawn_blocking`.
    pub fn download(&self, progress: Sender<i16>) -> Result<(), String> {
        // Clone out under lock so we don't hold the mutex across blocking I/O.
        let (mgr, info) = {
            let g = self.inner.lock().map_err(|_| "update mutex poisoned".to_string())?;
            let mgr = g.mgr.clone().ok_or_else(|| "no update source configured".to_string())?;
            let info = g.pending.clone().ok_or_else(|| "no pending update — call check first".to_string())?;
            (mgr, info)
        };
        mgr.download_updates(&info, Some(progress))
            .map_err(|e| format!("download_updates: {e}"))
    }

    /// Apply the previously-downloaded update + relaunch. `exit(0)`s on
    /// success, so only returns on error. Caller MUST stop autosync + tunnel
    /// before invoking — in-flight uploads die when the process exits.
    pub fn apply(&self) -> Result<(), String> {
        let (mgr, info) = {
            let g = self.inner.lock().map_err(|_| "update mutex poisoned".to_string())?;
            let mgr = g.mgr.clone().ok_or_else(|| "no update source configured".to_string())?;
            let info = g.pending.clone().ok_or_else(|| "no pending update — call check first".to_string())?;
            (mgr, info)
        };
        mgr.apply_updates_and_restart(&info)
            .map_err(|e| format!("apply_updates_and_restart: {e}"))?;
        Ok(())
    }
}

impl Default for UpdateService {
    fn default() -> Self { Self::new() }
}

fn resolve_manager(meta: ReleaseMetaCache) -> Result<Option<velopack::UpdateManager>, String> {
    // Local FileSource (RIFT_UPDATE_FEED) is dev-only — gated behind
    // `debug_assertions` so a release-build binary can't be tricked into
    // pointing at an attacker-controlled local update feed via env var.
    // Channel is bound at vpk-pack time via `--channel win` in
    // scripts/release.ps1 (#232). The string lands in the installed
    // VelopackLocator manifest, gets passed back to
    // `GithubSource::get_release_feed(channel, ...)` as the `channel` arg,
    // and drives the `releases.{channel}.json` asset lookup.
    //
    // We deliberately pass `None` for UpdateOptions here:
    //   - `UpdateOptions::ExplicitChannel = Some("win")` would force-override
    //     the manifest channel, which defeats any future channel-switch UX.
    //   - Leaving it None lets the manifest decide. The single-channel
    //     coupling is now documented at both ends — `--channel win` (vpk) and
    //     `UPDATE_CHANNEL` here, kept as the source-of-truth identifier even
    //     though the value flows through the manifest, not this call site.
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
    let src = GithubSource::new(GITHUB_OWNER, GITHUB_REPO, ALLOW_PRERELEASE, meta);
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
    /// Outer cache shared w/ UpdateService — populated each feed fetch.
    meta: ReleaseMetaCache,
}

impl GithubSource {
    pub fn new(owner: &str, repo: &str, allow_prerelease: bool, meta: ReleaseMetaCache) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            allow_prerelease,
            asset_urls: Arc::new(Mutex::new(HashMap::new())),
            meta,
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

    /// Variant that also returns the `Link: <url>; rel="next"` follow-on URL
    /// when GitHub paginates. Used by `get_release_feed` to walk multiple
    /// pages when the newest eligible release doesn't carry the channel feed
    /// asset (#252).
    fn http_get_with_link(url: &str) -> Result<(String, Option<String>), VeloError> {
        let mut resp = ureq::get(url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| VeloError::Generic(format!("GET {url}: {e}")))?;
        let next_url = resp
            .headers()
            .get("link")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_link_next);
        let body = resp
            .body_mut()
            .read_to_string()
            .map_err(|e| VeloError::Generic(format!("read {url}: {e}")))?;
        Ok((body, next_url))
    }
}

/// Parse the GitHub `Link` response header and return the URL for the `next`
/// page if present. Header format: `<url1>; rel="next", <url2>; rel="last"`.
fn parse_link_next(header: &str) -> Option<String> {
    for part in header.split(',') {
        let part = part.trim();
        if !part.contains("rel=\"next\"") { continue; }
        let start = part.find('<')?;
        let end = part[start + 1..].find('>')?;
        return Some(part[start + 1..start + 1 + end].to_string());
    }
    None
}

impl UpdateSource for GithubSource {
    fn get_release_feed(
        &self,
        channel: &str,
        _app: &Manifest,
        _staged_user_id: &str,
    ) -> Result<VelopackAssetFeed, VeloError> {
        let releases_name = format!("releases.{channel}.json");
        let initial_url = format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=50",
            self.owner, self.repo
        );

        // Page walker (#252). Per page, examine each eligible release for the
        // channel feed asset. First match wins; otherwise follow Link: rel="next".
        let mut next_url: Option<String> = Some(initial_url);
        let mut pages_walked = 0;
        let mut eligible_examined = 0;

        while let Some(api_url) = next_url {
            pages_walked += 1;
            log::info!("GithubSource: querying {api_url}");
            let (body, link_next) = Self::http_get_with_link(&api_url)?;
            let releases: Vec<serde_json::Value> = serde_json::from_str(&body)
                .map_err(|e| VeloError::Generic(format!("parse releases JSON: {e}")))?;

            for r in &releases {
                let is_draft = r.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);
                let is_pre = r.get("prerelease").and_then(|v| v.as_bool()).unwrap_or(false);
                if is_draft || (!self.allow_prerelease && is_pre) {
                    continue;
                }
                eligible_examined += 1;

                let Some(assets) = r.get("assets").and_then(|v| v.as_array()) else {
                    continue;
                };

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
                let Some(feed_url) = feed_url else { continue; };

                // Match found. Stash release metadata + asset URL cache, then
                // fetch the feed JSON and return. This is the only success path.
                let html_url = r.get("html_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let published_at = r.get("published_at").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if let Ok(mut g) = self.meta.lock() {
                    *g = Some(ReleaseMeta { html_url, published_at });
                }
                if let Ok(mut g) = self.asset_urls.lock() {
                    *g = url_map;
                }

                log::info!(
                    "GithubSource: downloading feed from {feed_url} (page {pages_walked}, eligible examined {eligible_examined})"
                );
                let feed_json = Self::http_get_string(&feed_url)?;
                let feed: VelopackAssetFeed = serde_json::from_str(&feed_json)
                    .map_err(|e| VeloError::Generic(format!("parse {releases_name}: {e}")))?;
                return Ok(feed);
            }

            next_url = link_next;
        }

        Err(VeloError::Generic(format!(
            "no eligible release w/ {releases_name} in {}/{} across {pages_walked} page(s) (eligible examined: {eligible_examined}, allow_prerelease={})",
            self.owner, self.repo, self.allow_prerelease
        )))
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
