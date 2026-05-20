// Phase 1b: load ~/.rift/rift.json server profiles. Read-only this session;
// writing/editing profiles lands w/ Phase 2's server-picker UI.
//
// Format compat: WPF Rift's `rift.json` uses camelCase fields. We preserve
// unknown fields via `serde(flatten)` into a JSON Value bag so a write-back
// pass in Phase 2 doesn't lose new fields the WPF app may add.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::state::paths::{atomic_write_json, rift_dir};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfile {
    pub key: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: String,
    pub remote_root: String,
    pub local_root: String,
    #[serde(default)]
    pub fingerprint: Option<String>,
    #[serde(default)]
    pub tx_admin_url: Option<String>,
    #[serde(default)]
    pub added_at: Option<String>,
    // Bridge token. Phase 6 (#9.3) migrated this to the OS keychain — the JSON
    // field is now legacy: parsed for one-time migration into the keychain on
    // `RiftConfig::load()`, never written back. Skip-serialize keeps it absent
    // from disk after migration. Runtime callers use `secrets::get(bridge_token_key(&server.key))`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_token: Option<String>,
    #[serde(default)]
    pub bridge_port: Option<u16>,
}

/// IPC-safe view of `ServerProfile`. Omits `bridge_token` (secret) and
/// replaces it with `has_bridge_token: bool` so the UI can still render
/// "bridge configured" state without seeing the token value. Used as the
/// return type of every Tauri command that surfaces a server to the
/// renderer (#9.1 — keep secrets off the IPC boundary).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfilePublic {
    pub key: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: String,
    pub remote_root: String,
    pub local_root: String,
    pub fingerprint: Option<String>,
    pub tx_admin_url: Option<String>,
    pub added_at: Option<String>,
    pub bridge_port: Option<u16>,
    pub has_bridge_token: bool,
}

impl From<&ServerProfile> for ServerProfilePublic {
    fn from(p: &ServerProfile) -> Self {
        // Phase 6 (#9.3): post-migration, `bridge_token` is always None on the
        // struct — the live value sits in the OS keychain. The "configured"
        // indicator the UI reads must come from there.
        let has_bridge_token = p
            .bridge_token
            .as_deref()
            .map(|s| !s.is_empty())
            .unwrap_or(false)
            || server_bridge_token(&p.key).is_some();
        Self {
            key: p.key.clone(),
            name: p.name.clone(),
            host: p.host.clone(),
            port: p.port,
            user: p.user.clone(),
            key_path: p.key_path.clone(),
            remote_root: p.remote_root.clone(),
            local_root: p.local_root.clone(),
            fingerprint: p.fingerprint.clone(),
            tx_admin_url: p.tx_admin_url.clone(),
            added_at: p.added_at.clone(),
            bridge_port: p.bridge_port,
            has_bridge_token,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RiftConfig {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub servers: Vec<ServerProfile>,
    #[serde(default)]
    pub last_selected: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Hard cap on `~/.rift/rift.json` parse size. A normal config is <10 KB;
/// 1 MiB is ~100× headroom. A crafted oversize file (e.g. attacker-supplied
/// w/ deeply-nested `extra` flatten payload) is rejected up-front before
/// `serde_json` can allocate or recurse into stack overflow.
const RIFT_CONFIG_MAX_BYTES: u64 = 1024 * 1024;

impl RiftConfig {
    pub fn load() -> Result<Self, String> {
        let path = config_path().map_err(|e| format!("rift dir: {e}"))?;
        let meta = std::fs::metadata(&path)
            .map_err(|e| format!("stat {}: {e}", path.display()))?;
        if meta.len() > RIFT_CONFIG_MAX_BYTES {
            return Err(format!(
                "rift.json oversized: {} bytes > cap {}",
                meta.len(),
                RIFT_CONFIG_MAX_BYTES
            ));
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        let mut cfg: RiftConfig = serde_json::from_str(&text)
            .map_err(|e| format!("parse rift.json: {e}"))?;
        // Phase 6 (#9.3) one-shot migration: lift any plaintext bridge_token
        // into the OS keychain, then strip it from the in-memory struct so the
        // next save() drops it from disk. Failure to write the keychain is
        // non-fatal — log + leave the JSON field intact for next attempt.
        let mut migrated = false;
        for s in cfg.servers.iter_mut() {
            if let Some(tok) = s.bridge_token.as_deref() {
                if !tok.is_empty() {
                    let key = crate::secrets::bridge_token_key(&s.key);
                    match crate::secrets::set(&key, tok) {
                        Ok(()) => {
                            s.bridge_token = None;
                            migrated = true;
                            log::info!("profile: migrated bridge_token for '{}' to keychain", s.key);
                        }
                        Err(e) => log::warn!("profile: keychain migration for '{}' failed: {e}", s.key),
                    }
                } else {
                    s.bridge_token = None;
                }
            }
        }
        if migrated {
            if let Err(e) = cfg.save() {
                log::warn!("profile: post-migration save failed: {e}");
            }
        }
        Ok(cfg)
    }

    pub fn find(&self, key: &str) -> Option<&ServerProfile> {
        self.servers.iter().find(|s| s.key == key)
    }

    /// Atomic write to `~/.rift/rift.json` via tmp-file + rename. Pretty JSON;
    /// preserves the `serde(flatten) extra` bag for any unknown WPF fields. A
    /// crash mid-write leaves the previous file intact (the rename is the
    /// commit point) — without this the only source of server profiles could
    /// be left half-written.
    pub fn save(&self) -> Result<(), String> {
        let path = config_path().map_err(|e| format!("rift dir: {e}"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| format!("serialize rift.json: {e}"))?;
        atomic_write_json(&path, &text).map_err(|e| format!("write {}: {e}", path.display()))
    }
}

/// Read the per-server bridge token from the OS keychain. Returns None if
/// unset, the backend is unavailable, or the entry is empty. Phase 6 (#9.3).
pub fn server_bridge_token(server_key: &str) -> Option<String> {
    crate::secrets::get(&crate::secrets::bridge_token_key(server_key))
}

/// Persist or clear the per-server bridge token in the OS keychain. Some →
/// set, None → delete. Phase 6 (#9.3).
pub fn set_server_bridge_token(server_key: &str, value: Option<&str>) -> Result<(), String> {
    let key = crate::secrets::bridge_token_key(server_key);
    match value {
        Some(v) if !v.is_empty() => crate::secrets::set(&key, v),
        _ => crate::secrets::delete(&key),
    }
}

pub fn config_path() -> std::io::Result<PathBuf> {
    Ok(rift_dir()?.join("rift.json"))
}

/// Mirrors WPF `ConfigStore.Slugify`. Lowercases, replaces non-alphanumeric
/// runs w/ a single hyphen, trims leading/trailing hyphens.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_hyphen = true;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            for lc in c.to_lowercase() {
                out.push(lc);
            }
            last_hyphen = false;
        } else if !last_hyphen {
            out.push('-');
            last_hyphen = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "server".into()
    } else {
        out
    }
}

/// Mirrors WPF `ConfigStore.UniqueKey`. Returns `base` if no collision, else
/// `base-2`, `base-3`, …
pub fn unique_key(base: &str, existing: &[String]) -> String {
    if !existing.iter().any(|k| k == base) {
        return base.to_string();
    }
    for n in 2..u32::MAX {
        let candidate = format!("{base}-{n}");
        if !existing.iter().any(|k| k == &candidate) {
            return candidate;
        }
    }
    base.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn loads_real_user_config() {
        let cfg = RiftConfig::load().expect("load");
        assert!(!cfg.servers.is_empty(), "user has at least one server");
        let first = &cfg.servers[0];
        eprintln!(
            "loaded {} servers; first = {} ({}@{}:{} → {})",
            cfg.servers.len(),
            first.name,
            first.user,
            first.host,
            first.port,
            first.remote_root
        );
    }
}
