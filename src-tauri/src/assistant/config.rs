//! Assistant config — `AssistantConfig` + provider CRUD + all get/set command pairs.
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
    /// Currently-open project folder for the Assistant. None = no folder open;
    /// Assistant falls back to AutoSync's server folders if any, else no-tools.
    /// Matches VS Code's "open folder" model — one root at a time.
    #[serde(default)]
    pub(super) current_root: Option<PathBuf>,
    /// Last ~10 folders the user opened. Newest first. Surfaced in EmptyState
    /// so they can jump back. AutoSync folders are NOT mirrored here; they're
    /// a separate source the picker shows as a "Synced servers" group.
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
    /// Auto-compact threshold as fraction of context window (0.0-1.0). `None` =
    /// disabled (manual only). User has `DISABLE_AUTO_COMPACT=1` set globally
    /// so default to None — opt-in, not opt-out. See `docs/design/assistant-compaction.md`.
    #[serde(default)]
    pub(super) auto_compact_threshold: Option<f32>,
    /// Model alias for the one-shot summarize call. `None` = "haiku" (cheap +
    /// fast; sufficient for prose summarization w/ explicit preservation prompt).
    /// $0.91 at 900K vs $2.73 on sonnet.
    #[serde(default)]
    pub(super) compact_model: Option<String>,
    /// LEGACY single-provider escape hatch (pre-2a). Migrated into `providers`
    /// on first `load_config()` and then cleared. Still parsed so old on-disk
    /// configs migrate cleanly; never re-written once `providers` is populated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) base_url: Option<String>,
    /// LEGACY companion to `base_url`. Migrated + cleared alongside it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) provider_model: Option<String>,
    /// 2a multi-provider list (cc-switch pattern). Each profile is an
    /// Anthropic-compatible endpoint; the active one routes `assistant_send`.
    /// Empty = Anthropic only. Secrets live in the keychain (`key_ref`), never here.
    #[serde(default)]
    pub(super) providers: Vec<ProviderProfile>,
    /// `id` of the provider that routes turns. `None` = Anthropic (no custom endpoint).
    #[serde(default)]
    pub(super) active_provider_id: Option<String>,
    /// 3c compression toggle (headroom-style local proxy). When `true`,
    /// `assistant_send` points `ANTHROPIC_BASE_URL` at a local compression proxy
    /// that deterministically shrinks context before forwarding upstream. Opt-in,
    /// OFF by default. The Python proxy runtime is a SOFT dependency Rift never
    /// bundles or spawns — the user runs it (e.g. `headroom serve`); Rift only
    /// owns the env seam + a reachability check. An active custom provider wins
    /// the same seam, so compression is bypassed for custom-provider turns.
    #[serde(default)]
    pub(super) compression_enabled: Option<bool>,
    /// Local compression proxy URL. `None` = the headroom default
    /// (`http://127.0.0.1:8787`). Only consulted when `compression_enabled`.
    #[serde(default)]
    pub(super) compression_proxy_url: Option<String>,
}

/// One saved custom-provider endpoint. `key_ref` is the keychain entry name
/// holding this provider's API key — the secret itself is never serialized to
/// `config.json`. `model` (optional) overrides Rift's tier via `--model`;
/// gateways that map Rift's tiers can leave it blank.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub key_ref: String,
}

/// Outward shape for `assistant_list_providers` — the profile plus derived
/// flags the UI needs (`has_key` so it can show "key set" without exposing it,
/// `active` for the one-click toggle). The secret + raw `key_ref` are still
/// withheld from the renderer beyond what it needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: Option<String>,
    pub has_key: bool,
    pub active: bool,
}

/// Renderer-supplied profile for `assistant_save_provider`. `id` empty/None on
/// create (server mints a stable slug); set on edit. The API key rides a
/// separate arg so it can be omitted on edit without clobbering the stored one.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub model: Option<String>,
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

/// Claude Fable 5 — limited run Rift offers only through 2026-06-22. Past
/// sunset a stale pref/session pin falls back to `opus` instead of firing at
/// a retired model id.
pub(super) const FABLE_MODEL: &str = "claude-fable-5";
pub(super) const FABLE_SUNSET_EPOCH_SECS: u64 = 1_782_172_800; // 2026-06-23T00:00:00Z

pub(super) fn fable_sunset_passed() -> bool {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() >= FABLE_SUNSET_EPOCH_SECS)
        .unwrap_or(false)
}

pub(super) fn load_config() -> AssistantConfig {
    let mut cfg: AssistantConfig = config_path()
        .and_then(|p| std::fs::read_to_string(&p).map_err(|e| e.to_string()))
        .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        .unwrap_or_default();
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
    // 2a: migrate the legacy single `base_url`/`provider_model` into the
    // `providers` list once. The legacy escape hatch reused the main Anthropic
    // key, so the migrated profile points `key_ref` at ASSISTANT_API_KEY — the
    // user keeps routing without re-entering anything. Gated on empty list so it
    // runs at most once; clears the legacy fields after so it never re-fires.
    if cfg.providers.is_empty() {
        if let Some(b) = cfg.base_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            cfg.providers.push(ProviderProfile {
                id: "legacy".into(),
                name: "Custom".into(),
                base_url: b.to_string(),
                model: cfg.provider_model.clone().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
                key_ref: crate::secrets::ASSISTANT_API_KEY.into(),
            });
            cfg.active_provider_id = Some("legacy".into());
            cfg.base_url = None;
            cfg.provider_model = None;
            if let Err(e) = save_config(&cfg) {
                log::warn!("assistant: provider migration save_config failed: {e}");
            } else {
                log::info!("assistant: migrated legacy base_url into providers list");
            }
        }
    }
    cfg
}

/// Keychain entry name for a provider's API key. Stable across renames since
/// it's keyed on the immutable `id`. The legacy migrated provider is the lone
/// exception — it reuses `ASSISTANT_API_KEY`.
pub(super) fn provider_key_ref(id: &str) -> String {
    format!("assistant.provider.{id}")
}

/// The provider that should route turns, or `None` for plain Anthropic.
pub(super) fn resolve_active_provider(cfg: &AssistantConfig) -> Option<ProviderProfile> {
    let id = cfg.active_provider_id.as_deref()?;
    cfg.providers.iter().find(|p| p.id == id).cloned()
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
/// adds pull/commit/push; `full` → reserved for RCON raw passthrough (phase 2).
pub(super) fn is_valid_trust_level(v: &str) -> bool {
    matches!(v, "readonly" | "standard" | "full")
}

/// Resolve the effective trust level. Explicit setting wins; when unset → `readonly`.
pub(super) fn effective_trust_level(trust_level: &Option<String>) -> String {
    trust_level
        .clone()
        .filter(|v| is_valid_trust_level(v))
        .unwrap_or_else(|| "readonly".into())
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

#[tauri::command]
pub fn assistant_get_auto_compact_threshold() -> Result<Option<f32>, String> {
    Ok(load_config()
        .auto_compact_threshold
        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 1.0))
}

#[tauri::command]
pub fn assistant_set_auto_compact_threshold(value: Option<f32>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    cfg.auto_compact_threshold = value.filter(|v| v.is_finite() && *v > 0.0 && *v <= 1.0);
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_compact_model() -> Result<String, String> {
    Ok(load_config()
        .compact_model
        .filter(|v| matches!(v.as_str(), "haiku" | "sonnet" | "opus"))
        .unwrap_or_else(|| "haiku".to_string()))
}

#[tauri::command]
pub fn assistant_set_compact_model(value: String) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    if !matches!(value.as_str(), "haiku" | "sonnet" | "opus") {
        return Err(format!("invalid compact_model: {value}"));
    }
    let mut cfg = load_config();
    cfg.compact_model = Some(value);
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_base_url() -> Result<Option<String>, String> {
    Ok(load_config().base_url.filter(|s| !s.trim().is_empty()))
}

/// Custom Anthropic-compatible endpoint; routes headless `-p` turns off the metered pool (June-15).
#[tauri::command]
pub fn assistant_set_base_url(value: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let v = value.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref u) = v {
        if !(u.starts_with("http://") || u.starts_with("https://")) {
            return Err("base_url must start with http:// or https://".into());
        }
    }
    let mut cfg = load_config();
    cfg.base_url = v;
    save_config(&cfg)
}

#[tauri::command]
pub fn assistant_get_provider_model() -> Result<Option<String>, String> {
    Ok(load_config().provider_model.filter(|s| !s.trim().is_empty()))
}

/// Model id passed to `--model` when a custom base_url is set (e.g. "deepseek-chat").
#[tauri::command]
pub fn assistant_set_provider_model(value: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let v = value.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref m) = v {
        if !is_valid_model_name(m) {
            return Err(format!("invalid provider_model: {m}"));
        }
    }
    let mut cfg = load_config();
    cfg.provider_model = v;
    save_config(&cfg)
}

// ── 2a multi-provider escape hatch (cc-switch pattern) ──

/// List saved custom providers with derived flags. Secrets are never returned —
/// only `has_key` so the UI can show "key set" without exposing the value.
#[tauri::command]
pub fn assistant_list_providers() -> Result<Vec<ProviderDto>, String> {
    let cfg = load_config();
    let active = cfg.active_provider_id.as_deref();
    Ok(cfg
        .providers
        .iter()
        .map(|p| ProviderDto {
            id: p.id.clone(),
            name: p.name.clone(),
            base_url: p.base_url.clone(),
            model: p.model.clone(),
            has_key: crate::secrets::get(&p.key_ref).is_some(),
            active: active == Some(p.id.as_str()),
        })
        .collect())
}

/// Create or update a provider. Empty/None `id` → create (server mints a stable
/// slug from the name); otherwise update in place. `api_key`, when present and
/// non-empty, is written to the keychain under the profile's `key_ref`; omit it
/// on edit to leave the stored key untouched. Returns the profile id.
#[tauri::command]
pub fn assistant_save_provider(profile: ProviderInput, api_key: Option<String>) -> Result<String, String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let name = profile.name.trim().to_string();
    if name.is_empty() {
        return Err("provider name is required".into());
    }
    let base_url = profile.base_url.trim().to_string();
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err("base_url must start with http:// or https://".into());
    }
    let model = profile.model.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref m) = model {
        if !is_valid_model_name(m) {
            return Err(format!("invalid provider model: {m}"));
        }
    }

    let mut cfg = load_config();
    let id = profile
        .id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| mint_provider_id(&name, &cfg.providers));

    // key_ref is stable per id; the legacy migrated provider keeps reusing the
    // main key, so don't reassign its key_ref to the provider-scoped slot.
    let key_ref = cfg
        .providers
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.key_ref.clone())
        .unwrap_or_else(|| provider_key_ref(&id));

    if let Some(k) = api_key.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        crate::secrets::set(&key_ref, k)?;
    }

    let prof = ProviderProfile { id: id.clone(), name, base_url, model, key_ref };
    match cfg.providers.iter_mut().find(|p| p.id == id) {
        Some(existing) => *existing = prof,
        None => cfg.providers.push(prof),
    }
    save_config(&cfg)?;
    Ok(id)
}

/// Delete a provider + its keychain entry. Clears `active_provider_id` if it
/// pointed here. The legacy provider's shared `ASSISTANT_API_KEY` is preserved
/// (it's also the Anthropic key) — only provider-scoped keys are removed.
#[tauri::command]
pub fn assistant_delete_provider(id: String) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    if let Some(pos) = cfg.providers.iter().position(|p| p.id == id) {
        let removed = cfg.providers.remove(pos);
        if removed.key_ref != crate::secrets::ASSISTANT_API_KEY {
            if let Err(e) = crate::secrets::delete(&removed.key_ref) {
                log::warn!("assistant: delete provider key {} failed: {e}", removed.key_ref);
            }
        }
        if cfg.active_provider_id.as_deref() == Some(id.as_str()) {
            cfg.active_provider_id = None;
        }
        save_config(&cfg)?;
    }
    Ok(())
}

/// Set the active provider (`None` = Anthropic). Rejects an unknown id.
#[tauri::command]
pub fn assistant_set_active_provider(id: Option<String>) -> Result<(), String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut cfg = load_config();
    let id = id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if let Some(ref want) = id {
        if !cfg.providers.iter().any(|p| &p.id == want) {
            return Err(format!("unknown provider id: {want}"));
        }
    }
    cfg.active_provider_id = id;
    save_config(&cfg)
}


/// Mint a stable, filesystem/keychain-safe id from the provider name, deduped
/// against existing ids. No uuid dep — a slug + numeric suffix is stable enough
/// (ids are immutable once created).
pub(super) fn mint_provider_id(name: &str, existing: &[ProviderProfile]) -> String {
    let base: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() { "provider".to_string() } else { base };
    if !existing.iter().any(|p| p.id == base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|cand| !existing.iter().any(|p| &p.id == cand))
        .unwrap_or(base)
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
