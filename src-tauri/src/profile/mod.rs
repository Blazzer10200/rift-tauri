// Phase 1b: load ~/.rift/rift.json server profiles. Read-only this session;
// writing/editing profiles lands w/ Phase 2's server-picker UI.
//
// Format compat: WPF Rift's `rift.json` uses camelCase fields. We preserve
// unknown fields via `serde(flatten)` into a JSON Value bag so a write-back
// pass in Phase 2 doesn't lose new fields the WPF app may add.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::state::paths::rift_dir;

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
    // Bridge token (DPAPI-encrypted on WPF side) + bridge port — preserved on
    // round-trip for Phase 2 write-back, not used by Phase 1b drift scanning.
    #[serde(default)]
    pub bridge_token: Option<String>,
    #[serde(default)]
    pub bridge_port: Option<u16>,
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

impl RiftConfig {
    pub fn load() -> Result<Self, String> {
        let path = config_path().map_err(|e| format!("rift dir: {e}"))?;
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("parse rift.json: {e}"))
    }

    pub fn find(&self, key: &str) -> Option<&ServerProfile> {
        self.servers.iter().find(|s| s.key == key)
    }

    /// Atomic-ish write to `~/.rift/rift.json`. Pretty JSON; preserves the
    /// `serde(flatten) extra` bag for any unknown WPF fields.
    pub fn save(&self) -> Result<(), String> {
        let path = config_path().map_err(|e| format!("rift dir: {e}"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| format!("serialize rift.json: {e}"))?;
        std::fs::write(&path, text).map_err(|e| format!("write {}: {e}", path.display()))
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
