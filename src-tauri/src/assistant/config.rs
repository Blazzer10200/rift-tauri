//! Assistant config — `AssistantConfig` + all get/set command pairs.
//! Extracted from `mod.rs` as R2 of the hot-file split (see `docs/design/assistant-mod-split.md`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct AssistantConfig {
    /// Legacy plaintext slot — Phase 6 (#37) moved the API key to the OS
    /// keychain. Still parsed (so old on-disk configs can be migrated) but
    /// never written: `skip_serializing_if` drops it once cleared, and
    /// `load_config()` runs a one-shot migration on read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) api_key: Option<String>,
    /// Currently-open project folder for the Assistant. None = no folder open
    /// → workspace tools are unavailable. Matches VS Code's "open folder"
    /// model — one root at a time.
    #[serde(default)]
    pub(super) current_root: Option<PathBuf>,
    /// Last ~10 folders the user opened. Newest first. Surfaced in EmptyState
    /// so they can jump back.
    #[serde(default)]
    pub(super) recent_roots: Vec<PathBuf>,
    /// When true (the default), spawn the CLI without `--strict-mcp-config`
    /// and `--disable-slash-commands` so user MCP servers + slash commands
    /// layer alongside Rift's. CLAUDE.md / hooks always load via the CLI's
    /// own resolution; the `Skill` tool is explicitly added to the
    /// `--allowed-tools` allowlist so `/handoff`, `/check`, `/plan`, etc.
    /// can invoke. No opt-out short of `--bare`, which fires automatically
    /// in API-key mode.
    /// `None` = default (true). Switch off for a sandboxed Assistant.
    #[serde(default)]
    pub(super) use_full_config: Option<bool>,
    /// Hard dollar cap per turn, passed as `--max-budget-usd <amount>`. The
    /// CLI exits non-zero if exceeded — we surface the failure as a chat
    /// notice. `None` or `<= 0` = no cap.
    #[serde(default)]
    pub(super) max_budget_usd: Option<f64>,
    /// Effort tier for extended thinking on non-Haiku models, mapped to the
    /// CLI's `--effort` flag in turn.rs: `"none"`→low · `"quick"`→medium ·
    /// `"smart"`→high (API default) · `"deep"`→xhigh · `"ultra"`→xhigh +
    /// ultracode workflows. Haiku rejects effort server-side and is skipped.
    /// Per-turn override rides the `assistant_send` arg; this is the default.
    #[serde(default)]
    pub(super) thinking_effort: Option<String>,
    /// Permission mode passed to the CLI's `--permission-mode`. One of
    /// `default` / `acceptEdits` / `plan` / `auto` / `bypassPermissions`.
    /// `None` resolves to `bypassPermissions` (Rift's historical behavior).
    /// Per-turn override rides the `assistant_send` arg; this is the default.
    #[serde(default)]
    pub(super) permission_mode: Option<String>,
    /// Assistant trust level gating the local git tools. One of `readonly` /
    /// `standard` / `full`. `None` = `readonly`.
    #[serde(default)]
    pub(super) trust_level: Option<String>,
    /// Experimental local-LLM mode. When true, `turn.rs` points the spawned CLI
    /// at a local Anthropic-Messages-compatible endpoint (LiteLLM/Ollama) via
    /// `ANTHROPIC_BASE_URL` + the keychain `LOCAL_LLM_API_KEY`, forces `--bare`,
    /// overrides `--model` with `local_llm_model`, and skips the cloud model-pin
    /// + `--effort`. Purely additive + flag-gated — off = byte-identical to the
    /// cloud path. Testing/experiment only for now.
    #[serde(default)]
    pub(super) local_llm_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) local_llm_base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) local_llm_model: Option<String>,
}

pub(super) const RECENT_ROOTS_MAX: usize = 10;

pub(super) fn config_path() -> Result<PathBuf, String> {
    let home = super::dirs_home()?;
    let dir = home.join(".rift").join("assistant");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir ~/.rift/assistant: {e}"))?;
    Ok(dir.join("config.json"))
}


/// #221: reject any model value that would be parsed as a flag by the CLI.
/// Allowlist `[A-Za-z0-9._-]+` w/ NO leading dash. Covers short aliases
/// (`sonnet`/`opus`/`haiku`) and full ids (`claude-opus-4-7`).
pub(super) fn is_valid_model_name(s: &str) -> bool {
    if s.is_empty() || s.starts_with('-') {
        return false;
    }
    s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Local-LLM model names carry provider prefixes + tags the cloud allowlist
/// rejects (`ollama/llama3`, `ollama_chat/qwen2.5:7b`). Same anti-flag-injection
/// guard (no leading dash, no empty) but also allows `/` and `:`.
pub(super) fn is_valid_local_model_name(s: &str) -> bool {
    if s.is_empty() || s.starts_with('-') {
        return false;
    }
    s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/' | b':'))
}

/// Claude Fable 5 — limited run Rift offers only through 2026-06-22. Past
/// sunset a stale pref/session pin falls back to `opus` instead of firing at
/// a retired model id.
pub(super) const FABLE_MODEL: &str = "claude-fable-5";
pub(super) const FABLE_SUNSET_EPOCH_SECS: u64 = 1_782_172_800; // 2026-06-23T00:00:00Z
/// Manual kill-switch — Fable pulled 2026-06-14 (US-gov disablement, temporary).
/// Mirrors the frontend `FABLE_DISABLED` (state/assistant/helpers.ts). While
/// true a pinned/stale Fable session falls back to opus even before the date
/// sunset, so a gov-disabled model id never reaches the API. Flip back to false
/// (both sides) the moment it's re-enabled.
pub(super) const FABLE_DISABLED: bool = true;

pub(super) fn fable_sunset_passed() -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() >= FABLE_SUNSET_EPOCH_SECS)
        .unwrap_or(false)
}

/// Fable can't be used right now — either manually killed or past its sunset.
pub(super) fn fable_unavailable() -> bool {
    FABLE_DISABLED || fable_sunset_passed()
}

pub(super) fn load_config() -> AssistantConfig {
    // Missing file = normal first run (silent default); a file that exists
    // but fails to parse means settings are being dropped — warn so it's
    // traceable instead of "my settings vanished".
    let mut cfg: AssistantConfig = match config_path()
        .and_then(|p| std::fs::read_to_string(&p).map_err(|e| e.to_string()))
    {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            log::warn!("assistant config unreadable — falling back to defaults: {e}");
            AssistantConfig::default()
        }),
        Err(_) => AssistantConfig::default(),
    };
    // Phase 6 (#37): one-shot migration of any plaintext api_key into the
    // OS keychain. Failure is non-fatal — the field stays in JSON for a
    // future attempt, and runtime reads still see it via legacy fallback in
    // current_api_key().
    if let Some(k) = cfg.api_key.as_deref().filter(|s| !s.is_empty()) {
        match crate::secrets::set(crate::secrets::ASSISTANT_API_KEY, k) {
            Ok(()) => {
                cfg.api_key = None;
                if let Err(e) = save_config(&cfg) {
                    log::warn!("assistant: post-migration save_config failed: {e}");
                } else {
                    log::info!("assistant: migrated api_key to keychain");
                }
            }
            Err(e) => log::warn!("assistant: keychain migration for api_key failed: {e}"),
        }
    }
    cfg
}

/// Phase 6 (#37): the live API key. Reads the keychain first; falls back to
/// any (un-migrated) plaintext value still in `config.json`. Returns None
/// when both are empty/absent.
pub(super) fn current_api_key() -> Option<String> {
    crate::secrets::get(crate::secrets::ASSISTANT_API_KEY)
        .or_else(|| load_config().api_key.filter(|s| !s.is_empty()))
}

pub(super) fn save_config(cfg: &AssistantConfig) -> Result<(), String> {
    let p = config_path()?;
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    // #65: tmp+rename to match assistant_save_conversation. Two Tauri-command
    // setters racing on read-modify-write (e.g. set_api_key + set_max_budget
    // back-to-back) previously produced a torn or empty config.json under a
    // direct std::fs::write — the second writer truncated mid-flight.
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, s).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("rename {}: {e}", p.display()))?;
    Ok(())
}

/// Phase 6 (#37): renderer must never see the secret value — only whether
/// one is configured. Replaces the legacy `assistant_get_api_key` cmd which
/// returned the plaintext value to JS.
#[tauri::command]
pub fn assistant_get_api_key_present() -> Result<bool, String> {
    Ok(current_api_key().is_some())
}

#[tauri::command]
pub fn assistant_get_use_full_config() -> Result<bool, String> {
    Ok(load_config().use_full_config.unwrap_or(true))
}

/// Serializes every `assistant_set_*` config write. Each setter does a
/// load→modify→save read-modify-write; without this, two concurrent setters
/// (the Settings page can fire several `invoke`s in one render) read the same
/// on-disk state and the second save silently clobbers the first's change.
pub(super) static CONFIG_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tauri::command]
pub fn assistant_set_use_full_config(value: bool) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    cfg.use_full_config = Some(value);
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_max_budget_usd() -> Result<Option<f64>, String> {
    Ok(load_config().max_budget_usd.filter(|v| v.is_finite() && *v > 0.0))
}

#[tauri::command]
pub fn assistant_set_max_budget_usd(value: Option<f64>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    cfg.max_budget_usd = value.filter(|v| v.is_finite() && *v > 0.0);
    save_config(&cfg)
}

// F48: assistant_{get,set}_thinking_effort and _permission_mode commands removed
// — the frontend persists both via localStorage and passes them per-send through
// assistant_send's args; the config-file round-trip these commands wrote was a
// dead second store the UI never read. The `thinking_effort`/`permission_mode`
// config fields + `is_valid_permission_mode` are still used as per-send fallbacks.

/// The CLI's `--permission-mode` values Rift exposes. `dontAsk` (the CLI's
/// auto-DENY mode) is intentionally excluded — there's no Rift surface to
/// approve, so it would silently block everything (see the S92 note below).
pub(super) fn is_valid_permission_mode(v: &str) -> bool {
    matches!(v, "default" | "acceptEdits" | "plan" | "auto" | "bypassPermissions")
}

/// The Assistant trust levels Rift exposes. Gates the local git tools in the
/// MCP server (`git_local.rs`): `readonly` → status/diff/log; `standard` →
/// adds pull/commit/push. (CR-UX: the dead third level `full` — functionally
/// identical to `standard` — was collapsed 2026-06-11.)
pub(super) fn is_valid_trust_level(v: &str) -> bool {
    matches!(v, "readonly" | "standard")
}

/// Resolve the effective trust level. Explicit setting wins; when unset →
/// `readonly`. Persisted configs from the ternary era map `full` → `standard`
/// (read-side migration — no disk rewrite needed).
pub(super) fn effective_trust_level(trust_level: &Option<String>) -> String {
    match trust_level.as_deref() {
        Some("full") | Some("standard") => "standard".into(),
        Some("readonly") => "readonly".into(),
        _ => "readonly".into(),
    }
}

#[tauri::command]
pub fn assistant_get_trust_level() -> Result<String, String> {
    let cfg = load_config();
    Ok(effective_trust_level(&cfg.trust_level))
}

#[tauri::command]
pub fn assistant_set_trust_level(value: String) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    if !is_valid_trust_level(&value) {
        return Err(format!("invalid trust_level: {value}"));
    }
    let mut cfg = load_config();
    cfg.trust_level = Some(value);
    save_config(&cfg)
}

/// Renderer-facing view of local-LLM config. Never includes the key value —
/// only whether one is set (mirrors `assistant_get_api_key_present`).
#[derive(Serialize)]
pub struct LocalLlmDto {
    enabled: bool,
    base_url: Option<String>,
    model: Option<String>,
    has_key: bool,
}

#[tauri::command]
pub fn assistant_get_local_llm_config() -> Result<LocalLlmDto, String> {
    let cfg = load_config();
    Ok(LocalLlmDto {
        enabled: cfg.local_llm_enabled,
        base_url: cfg.local_llm_base_url,
        model: cfg.local_llm_model,
        has_key: crate::secrets::get(crate::secrets::LOCAL_LLM_API_KEY).is_some(),
    })
}

#[tauri::command]
pub fn assistant_set_local_llm_enabled(value: bool) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    cfg.local_llm_enabled = value;
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_set_local_llm_base_url(value: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // Normalize: a bare `localhost:4000` (no scheme) would reach the CLI as a
    // malformed ANTHROPIC_BASE_URL and fail confusingly — prepend http://.
    let normalized = value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|s| if s.contains("://") { s } else { format!("http://{s}") });
    let mut cfg = load_config();
    cfg.local_llm_base_url = normalized;
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_set_local_llm_model(value: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let trimmed = value.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref m) = trimmed {
        if !is_valid_local_model_name(m) {
            return Err(format!("invalid local model name: {m}"));
        }
    }
    let mut cfg = load_config();
    cfg.local_llm_model = trimmed;
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_set_local_llm_key(key: Option<String>) -> Result<(), String> {
    match key.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(k) => crate::secrets::set(crate::secrets::LOCAL_LLM_API_KEY, k),
        None => crate::secrets::delete(crate::secrets::LOCAL_LLM_API_KEY),
    }
}

#[tauri::command]
pub fn assistant_set_api_key(api_key: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // Phase 6 (#37): write the API key to the OS keychain, not config.json.
    // Empty/None → delete the keychain entry. Also clears any lingering
    // legacy plaintext field (load_config's migration handles the read side,
    // but a fresh set after a failed migration leaves the legacy slot stale).
    match api_key.as_deref().filter(|s| !s.is_empty()) {
        Some(k) => crate::secrets::set(crate::secrets::ASSISTANT_API_KEY, k)?,
        None => crate::secrets::delete(crate::secrets::ASSISTANT_API_KEY)?,
    }
    let mut cfg = load_config();
    if cfg.api_key.is_some() {
        cfg.api_key = None;
        save_config(&cfg)?;
    }
    Ok(())
}
