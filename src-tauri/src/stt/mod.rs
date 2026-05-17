//! Speech-to-text configuration persistence.
//!
//! The actual recognition runs in the WebView via the Web Speech API (Edge's
//! built-in recogniser, Azure-backed when online). No Rust-side audio capture
//! or model — this module only owns the persisted user settings so they
//! survive restarts.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// Master switch. When false the composer's mic button is hidden.
    #[serde(default)]
    pub enabled: bool,
    /// BCP-47 language tag (e.g. `"en-US"`, `"es-ES"`). Empty = browser default.
    #[serde(default = "default_lang")]
    pub language: String,
    /// When true the transcript appends to the existing draft; false replaces it.
    #[serde(default = "default_true")]
    pub append_to_draft: bool,
    /// When true the recogniser keeps running until the user clicks stop;
    /// false stops after the first final result.
    #[serde(default = "default_true")]
    pub continuous: bool,
    /// When true, partial in-progress text appears in the composer as you
    /// speak; the final committed text replaces it on each segment.
    #[serde(default = "default_true")]
    pub show_interim: bool,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn default_lang() -> String {
    "en-US".to_string()
}
fn default_true() -> bool {
    true
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            language: default_lang(),
            append_to_draft: true,
            continuous: true,
            show_interim: true,
            extra: serde_json::Map::new(),
        }
    }
}

fn dirs_home() -> PathBuf {
    if let Some(p) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        return p;
    }
    if let Some(p) = std::env::var_os("HOME").map(PathBuf::from) {
        return p;
    }
    PathBuf::from(".")
}

fn config_path() -> PathBuf {
    dirs_home().join(".rift").join("stt-config.json")
}

fn load_config() -> SttConfig {
    let path = config_path();
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return SttConfig::default(),
    };
    if bytes.len() > 64 * 1024 {
        return SttConfig::default();
    }
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn save_config(cfg: &SttConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir ~/.rift: {e}"))?;
    }
    let json = serde_json::to_vec_pretty(cfg).map_err(|e| format!("serialise: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write: {e}"))
}

#[tauri::command]
pub fn stt_get_config() -> Result<SttConfig, String> {
    Ok(load_config())
}

#[tauri::command]
pub fn stt_set_config(config: SttConfig) -> Result<(), String> {
    save_config(&config)
}
