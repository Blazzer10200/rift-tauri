//! Multi-model provider registry — named endpoint profiles (Kimi / DeepSeek /
//! GLM / OpenRouter / local) over the local-LLM wire mechanism. Activating a
//! profile copies its base_url/model/key into the proven `local_llm_*` config
//! fields + `LOCAL_LLM_API_KEY`, so the per-turn spawn path in turn.rs needs
//! zero provider awareness. Design: docs/design/multi-model-providers.md.

use serde::{Deserialize, Serialize};

use super::config::{
    is_valid_local_base_url, is_valid_local_model_name, load_config, save_config, AssistantConfig,
    CONFIG_WRITE_LOCK,
};

/// One saved endpoint profile. Presets (frontend-defined) prefill everything
/// but the key; `custom` entries are user-built. Models are NOT a catalog —
/// preset defaults + /v1/models detection hits + free-text, per the
/// providers-not-models doctrine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) base_url: String,
    #[serde(default)]
    pub(super) model: Option<String>,
    #[serde(default)]
    pub(super) models: Vec<String>,
    /// Preset slug ("kimi" / "deepseek" / "glm" / "openrouter" / "ollama") or
    /// None = custom. Display-only — the backend treats all kinds identically.
    #[serde(default)]
    pub(super) preset: Option<String>,
    /// CLAUDE_CODE_MAX_OUTPUT_TOKENS for this provider's turns (None = 8192).
    #[serde(default)]
    pub(super) max_output_tokens: Option<u32>,
    /// Endpoint honors Anthropic extended-thinking (`--effort` + native
    /// `thinking` blocks). Preset default: true for the reasoning clouds
    /// (Kimi/DeepSeek/GLM), false for Ollama/LiteLLM/OpenRouter. Conservative
    /// default false — unknown capability stays hidden (design doc §capability
    /// drift). Gates the composer effort ladder + the direct (shim-less) route.
    #[serde(default)]
    pub(super) effort: bool,
}

impl ProviderProfile {
    /// The pre-registry local-LLM config as a provider entry. Id "local" is
    /// load-bearing: its keychain slot IS `LOCAL_LLM_API_KEY`, so the
    /// migration never has to move a secret.
    pub(super) fn migrated_local(base_url: String, model: Option<String>) -> Self {
        Self {
            id: "local".to_string(),
            name: "Local (Ollama/LiteLLM)".to_string(),
            base_url,
            model,
            models: Vec::new(),
            preset: Some("ollama".to_string()),
            max_output_tokens: None,
            effort: false,
        }
    }
}

/// Keychain slot per provider. "local" aliases the pre-registry key.
pub(super) fn provider_key_name(id: &str) -> String {
    if id == "local" {
        crate::secrets::LOCAL_LLM_API_KEY.to_string()
    } else {
        format!("provider.{id}.api_key")
    }
}

/// Ids land in keychain entry names + config lookups — slug-only.
fn is_valid_provider_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 40
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn validate(p: &ProviderProfile) -> Result<(), String> {
    if !is_valid_provider_id(&p.id) {
        return Err(format!("invalid provider id (lowercase slug only): {}", p.id));
    }
    if p.name.trim().is_empty() {
        return Err("provider name is required".to_string());
    }
    if !is_valid_local_base_url(&p.base_url) {
        return Err(format!("invalid base URL (need http(s):// + host): {}", p.base_url));
    }
    if let Some(m) = p.model.as_deref() {
        if !is_valid_local_model_name(m) {
            return Err(format!("invalid model name: {m}"));
        }
    }
    if p.models.len() > 100 {
        return Err("too many models (max 100)".to_string());
    }
    if let Some(bad) = p.models.iter().find(|m| !is_valid_local_model_name(m)) {
        return Err(format!("invalid model name in list: {bad}"));
    }
    if let Some(cap) = p.max_output_tokens {
        if !(1024..=131_072).contains(&cap) {
            return Err(format!("max_output_tokens out of range (1024..=131072): {cap}"));
        }
    }
    Ok(())
}

/// Copy a profile into the wire fields turn.rs/nothink/oneshot actually read.
/// The key is COPIED (not moved) into `LOCAL_LLM_API_KEY` so switching back
/// and forth never loses a provider's stored key.
fn sync_wire(cfg: &mut AssistantConfig, p: &ProviderProfile) -> Result<(), String> {
    cfg.local_llm_base_url = Some(p.base_url.trim_end_matches('/').to_string());
    cfg.local_llm_model = p.model.clone();
    cfg.local_llm_max_output = p.max_output_tokens;
    cfg.local_llm_effort = p.effort;
    if p.id != "local" {
        match crate::secrets::get(&provider_key_name(&p.id)) {
            Some(k) => crate::secrets::set(crate::secrets::LOCAL_LLM_API_KEY, &k)?,
            None => {
                let _ = crate::secrets::delete(crate::secrets::LOCAL_LLM_API_KEY);
            }
        }
    }
    Ok(())
}

/// Renderer-facing view. Never carries key material — only `has_key`.
#[derive(Serialize)]
pub struct ProviderDto {
    id: String,
    name: String,
    base_url: String,
    model: Option<String>,
    models: Vec<String>,
    preset: Option<String>,
    max_output_tokens: Option<u32>,
    effort: bool,
    has_key: bool,
    active: bool,
}

fn to_dto(p: &ProviderProfile, active: Option<&str>) -> ProviderDto {
    ProviderDto {
        id: p.id.clone(),
        name: p.name.clone(),
        base_url: p.base_url.clone(),
        model: p.model.clone(),
        models: p.models.clone(),
        preset: p.preset.clone(),
        max_output_tokens: p.max_output_tokens,
        effort: p.effort,
        has_key: crate::secrets::get(&provider_key_name(&p.id)).is_some(),
        active: active == Some(p.id.as_str()),
    }
}

#[tauri::command]
pub fn assistant_list_providers() -> Result<Vec<ProviderDto>, String> {
    let cfg = load_config();
    let active = cfg.active_provider.as_deref();
    Ok(cfg.providers.iter().map(|p| to_dto(p, active)).collect())
}

#[tauri::command]
pub fn assistant_upsert_provider(profile: ProviderProfile) -> Result<(), String> {
    let _g = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut profile = profile;
    // Same scheme-normalization as the legacy base-url setter.
    let trimmed = profile.base_url.trim().to_string();
    profile.base_url = if trimmed.contains("://") { trimmed } else { format!("http://{trimmed}") };
    profile.name = profile.name.trim().to_string();
    profile.model = profile.model.map(|m| m.trim().to_string()).filter(|m| !m.is_empty());
    validate(&profile)?;
    let mut cfg = load_config();
    match cfg.providers.iter_mut().find(|p| p.id == profile.id) {
        Some(slot) => *slot = profile.clone(),
        None => cfg.providers.push(profile.clone()),
    }
    // Editing the live provider re-syncs the wire fields the next turn reads.
    if cfg.active_provider.as_deref() == Some(profile.id.as_str()) {
        sync_wire(&mut cfg, &profile)?;
    }
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_delete_provider(id: String) -> Result<(), String> {
    let _g = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    if !cfg.providers.iter().any(|p| p.id == id) {
        return Ok(());
    }
    let was_active = cfg.active_provider.as_deref() == Some(id.as_str());
    cfg.providers.retain(|p| p.id != id);
    // Clear the wire fields when the deleted entry owned them — also prevents
    // the empty-registry migration in load_config from resurrecting "local".
    if was_active || (id == "local" && cfg.active_provider.is_none()) {
        cfg.active_provider = None;
        cfg.local_llm_enabled = false;
        cfg.local_llm_base_url = None;
        cfg.local_llm_model = None;
        cfg.local_llm_max_output = None;
        cfg.local_llm_effort = false;
    }
    // "local"'s key slot is the shared wire slot — only clear it when no OTHER
    // provider's copied key is live in there.
    if id == "local" {
        if was_active || cfg.active_provider.is_none() {
            let _ = crate::secrets::delete(crate::secrets::LOCAL_LLM_API_KEY);
        }
    } else {
        let _ = crate::secrets::delete(&provider_key_name(&id));
    }
    save_config(&cfg)
}

/// Switch the assistant onto a provider (Some) or back to Claude (None).
#[tauri::command]
pub fn assistant_activate_provider(id: Option<String>) -> Result<(), String> {
    let _g = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    match id {
        None => {
            cfg.active_provider = None;
            cfg.local_llm_enabled = false;
        }
        Some(id) => {
            let p = cfg
                .providers
                .iter()
                .find(|p| p.id == id)
                .cloned()
                .ok_or_else(|| format!("unknown provider: {id}"))?;
            sync_wire(&mut cfg, &p)?;
            cfg.active_provider = Some(id);
            cfg.local_llm_enabled = true;
        }
    }
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_set_provider_key(id: String, key: Option<String>) -> Result<(), String> {
    let cfg = load_config();
    if !cfg.providers.iter().any(|p| p.id == id) {
        return Err(format!("unknown provider: {id}"));
    }
    let slot = provider_key_name(&id);
    let trimmed = key.as_deref().map(str::trim).filter(|s| !s.is_empty());
    match trimmed {
        Some(k) => crate::secrets::set(&slot, k)?,
        None => crate::secrets::delete(&slot)?,
    }
    // Keep the wire copy current when this provider is the live one.
    if cfg.active_provider.as_deref() == Some(id.as_str()) && id != "local" {
        match trimmed {
            Some(k) => crate::secrets::set(crate::secrets::LOCAL_LLM_API_KEY, k)?,
            None => {
                let _ = crate::secrets::delete(crate::secrets::LOCAL_LLM_API_KEY);
            }
        }
    }
    Ok(())
}

fn probe_params(id: &str) -> Result<(String, String, Option<String>), String> {
    let cfg = load_config();
    let p = cfg
        .providers
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("unknown provider: {id}"))?;
    let base = p.base_url.trim().trim_end_matches('/').to_string();
    if !is_valid_local_base_url(&base) {
        return Err(format!("invalid base URL: {base}"));
    }
    let key = crate::secrets::get(&provider_key_name(id)).unwrap_or_else(|| "local".to_string());
    Ok((base, key, p.model.clone()))
}

/// Round-trip one prompt through a profile (active or not) — the Models page's
/// per-card "Test". Key stays backend-side.
#[tauri::command]
pub async fn assistant_test_provider(id: String) -> Result<super::local_llm::LocalTestResult, String> {
    let (base, key, model) = probe_params(&id)?;
    let model = model.ok_or("No model configured — pick or type one first")?;
    super::local_llm::probe_messages(&base, &key, &model).await
}

/// Best-effort `/v1/models` listing for a profile. Errors surface (the page
/// shows them beside Detect); cloud Anthropic-compat bases often lack this.
#[tauri::command]
pub async fn assistant_list_provider_models(id: String) -> Result<Vec<String>, String> {
    let (base, key, _) = probe_params(&id)?;
    super::local_llm::probe_models(&base, &key).await
}
