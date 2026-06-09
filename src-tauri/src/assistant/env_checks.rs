//! R5 (per `docs/design/assistant-mod-split.md`) — environment observation:
//! the compression-proxy toggle + reachability probe and the optional host-tool
//! PATH checks. Lifted verbatim from `assistant/mod.rs` 2026-06-09. Config
//! load/save stays on the parent (R2) — reached via `super::`.

use std::process::Stdio;

use serde::Serialize;

use super::{load_config, save_config, AssistantConfig, CONFIG_WRITE_LOCK};

/// Default local compression proxy URL — headroom's `serve` port. The user can
/// override it (any Anthropic-compatible compressing proxy works on this seam).
const DEFAULT_COMPRESSION_PROXY: &str = "http://127.0.0.1:8787";

/// The effective proxy URL when the compression toggle is on, else `None`.
/// Resolution: explicit `compression_proxy_url` (trimmed, non-empty) → default.
pub(super) fn resolve_compression(cfg: &AssistantConfig) -> Option<String> {
    if cfg.compression_enabled != Some(true) {
        return None;
    }
    Some(
        cfg.compression_proxy_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_COMPRESSION_PROXY)
            .to_string(),
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionConfig {
    pub enabled: bool,
    /// `None` = use the default proxy URL (surfaced separately as `default_url`).
    pub proxy_url: Option<String>,
    pub default_url: String,
}

#[tauri::command]
pub fn assistant_get_compression() -> Result<CompressionConfig, String> {
    let cfg = load_config();
    Ok(CompressionConfig {
        enabled: cfg.compression_enabled == Some(true),
        proxy_url: cfg.compression_proxy_url.filter(|s| !s.trim().is_empty()),
        default_url: DEFAULT_COMPRESSION_PROXY.to_string(),
    })
}

/// Persist the compression toggle + optional custom proxy URL. Empty/blank
/// `proxy_url` clears the override (falls back to the default at send time).
#[tauri::command]
pub fn assistant_set_compression(enabled: bool, proxy_url: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let v = proxy_url.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref u) = v {
        if !(u.starts_with("http://") || u.starts_with("https://")) {
            return Err("proxy_url must start with http:// or https://".into());
        }
    }
    let mut cfg = load_config();
    cfg.compression_enabled = Some(enabled);
    cfg.compression_proxy_url = v;
    save_config(&cfg)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionEnv {
    /// The URL that was probed (resolved override → default).
    pub proxy_url: String,
    /// A TCP listener answered at the proxy host:port.
    pub proxy_reachable: bool,
    /// `headroom` resolves on PATH (the reference compressor runtime).
    pub headroom_present: bool,
    /// A Python interpreter resolves on PATH (headroom's soft runtime dep).
    pub python_present: bool,
}

/// Probe whether a local compression proxy is reachable + whether the headroom
/// runtime looks installed. Pure observation — never spawns or installs anything.
#[tauri::command]
pub async fn compression_env_check(proxy_url: Option<String>) -> CompressionEnv {
    let url = proxy_url
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_COMPRESSION_PROXY.to_string());
    let probe_url = url.clone();
    let reachable = tokio::task::spawn_blocking(move || probe_tcp(&probe_url))
        .await
        .unwrap_or(false);
    CompressionEnv {
        proxy_reachable: reachable,
        headroom_present: which_on_path("headroom"),
        python_present: which_on_path("python") || which_on_path("python3"),
        proxy_url: url,
    }
}

/// Best-effort TCP connect to the host:port of an http(s) URL with a short
/// timeout. Used only as a reachability hint for the compression proxy.
fn probe_tcp(url: &str) -> bool {
    use std::net::ToSocketAddrs;
    let https = url.starts_with("https://");
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);
    let authority = rest.split('/').next().unwrap_or(rest);
    if authority.is_empty() {
        return false;
    }
    let addr = if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{authority}:{}", if https { 443 } else { 80 })
    };
    match addr.to_socket_addrs() {
        Ok(mut addrs) => addrs.next().is_some_and(|a| {
            std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_millis(600)).is_ok()
        }),
        Err(_) => false,
    }
}

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
/// individual features need them: git tools + edit-swarm need `git`; the swarm
/// gates need `npm`/`cargo`; the "Open in VS Code" affordance needs `code`.
/// Surfaced in Settings → About → Local tools and used to hide dead affordances.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_resolution() {
        // Off (default) → None, regardless of any stored URL.
        let mut cfg = AssistantConfig::default();
        assert_eq!(resolve_compression(&cfg), None);
        // On, no custom URL → the default proxy.
        cfg.compression_enabled = Some(true);
        assert_eq!(resolve_compression(&cfg).as_deref(), Some(DEFAULT_COMPRESSION_PROXY));
        // On, custom URL → trimmed custom URL wins.
        cfg.compression_proxy_url = Some("  http://127.0.0.1:9999  ".into());
        assert_eq!(resolve_compression(&cfg).as_deref(), Some("http://127.0.0.1:9999"));
        // On, blank URL → falls back to default (not the empty string).
        cfg.compression_proxy_url = Some("   ".into());
        assert_eq!(resolve_compression(&cfg).as_deref(), Some(DEFAULT_COMPRESSION_PROXY));
    }
}
