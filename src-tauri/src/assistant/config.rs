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
    /// User-defined projects — named aliases over a workspace folder + per-project
    /// file-pattern config. Additive: empty = the app behaves exactly as before
    /// (folder picker + recent_roots only). See `projects.rs`.
    #[serde(default)]
    pub(super) projects: Vec<super::projects::Project>,
    /// When true (the default), Rift runs as a faithful reskin of the user's
    /// Claude Code setup: the CLI spawns with `--setting-sources
    /// user,project,local` (inheriting the global ~/.claude CLAUDE.md,
    /// settings.json, and hooks) and WITHOUT `--strict-mcp-config` /
    /// `--disable-slash-commands` (so user MCP servers + slash commands layer
    /// alongside Rift's). The `Skill` tool is in the `--allowed-tools`
    /// allowlist so `/handoff`, `/check`, `/plan`, etc. invoke. When false,
    /// Rift is a clean sandbox: the `user` setting source is dropped (no global
    /// CLAUDE.md/hooks) and only Rift's own MCP tools are exposed. Forced off
    /// in API-key + local-LLM modes (both fire `--bare`, which suppresses user
    /// config wholesale).
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
    /// `"smart"`→medium (responsive interactive default) · `"deep"`→high ·
    /// `"ultra"`→xhigh + ultracode. Haiku rejects effort server-side and is skipped.
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
    /// overrides `--model` with `local_llm_model`, and skips the cloud
    /// model-pin and `--effort`. Purely additive + flag-gated — off =
    /// byte-identical to the cloud path. Testing/experiment only for now.
    #[serde(default)]
    pub(super) local_llm_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) local_llm_base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) local_llm_model: Option<String>,
}

pub(super) const RECENT_ROOTS_MAX: usize = 10;

pub(super) fn config_path() -> Result<PathBuf, String> {
    let home = super::dirs_home()
        .map_err(|e| format!("cannot locate home dir (USERPROFILE/HOME unset): {e}"))?;
    let dir = home.join(".rift").join("assistant");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    Ok(dir.join("config.json"))
}

/// Best-effort DACL lockdown for a file that may hold a plaintext secret
/// (config.json when keychain migration fails). Mirrors the mcp-config icacls
/// pattern: strip inherited ACEs, grant the current user Full Control only.
/// No-op on non-Windows / empty username. Detached thread — icacls can block
/// for seconds under AV, and callers run on the load path.
fn harden_config_acl(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let user = std::env::var("USERNAME").unwrap_or_default();
        if user.is_empty() {
            return;
        }
        let principal = acl_principal(&user);
        let path_for_acl = path.to_path_buf();
        std::thread::spawn(move || {
            let status = std::process::Command::new("icacls")
                .arg(&path_for_acl)
                .args(["/inheritance:r", "/grant:r", &format!("{principal}:(F)")])
                .creation_flags(CREATE_NO_WINDOW)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            if !matches!(status, Ok(s) if s.success()) {
                log::warn!("icacls failed to lock down {} for {principal}", path_for_acl.display());
            }
        });
    }
}

/// Windows icacls principal for the current user. On a domain-joined machine the
/// bare `USERNAME` (e.g. `jsmith`) is NOT a resolvable SID for icacls — it needs
/// the `DOMAIN\user` form. USERDOMAIN holds the AD domain on a joined box and the
/// machine name on a standalone box; only prefix when it differs from
/// COMPUTERNAME, so standalone machines keep the exact bare-username behavior.
#[cfg(windows)]
pub(super) fn acl_principal(user: &str) -> String {
    let domain = std::env::var("USERDOMAIN").unwrap_or_default();
    let computer = std::env::var("COMPUTERNAME").unwrap_or_default();
    if !domain.is_empty() && !domain.eq_ignore_ascii_case(&computer) {
        format!("{domain}\\{user}")
    } else {
        user.to_string()
    }
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

/// Default model when the renderer sends none. Mirrors the `loadModel` default
/// in state/assistant/helpers.ts.
pub(super) const DEFAULT_MODEL: &str = "sonnet";

/// Substitute when a selected/pinned Fable session is unavailable. Fable is an
/// Opus-tier model, so Opus is the closest match. MUST match the frontend Fable
/// fallback in state/assistant/helpers.ts (`loadModel`) — a divergence here
/// runs one model while the UI shows another.
pub(super) const FABLE_FALLBACK_MODEL: &str = "opus";

/// Effort tiers low→high. Mirrors `EFFORT_ORDER` in state/assistant/helpers.ts.
pub(super) const EFFORT_ORDER: [&str; 5] = ["none", "quick", "smart", "deep", "ultra"];

/// Reject an effort tier the ladder doesn't define (renderer-supplied).
pub(super) fn is_valid_effort_tier(s: &str) -> bool {
    EFFORT_ORDER.contains(&s)
}

/// Highest effort tier a model honors server-side. Mirrors `MODEL_MAX_EFFORT`
/// in state/assistant/helpers.ts. Unknown ids default to the top (no clamp).
pub(super) fn model_max_effort(model: &str) -> &'static str {
    match model {
        "haiku" => "none",
        _ => "ultra", // opus / sonnet / claude-opus-4-7 / claude-fable-5 / unknown
    }
}

/// Clamp an effort tier to a model's ceiling. Mirrors `clampEffort` in
/// state/assistant/helpers.ts. An unknown tier is left untouched (the flag
/// match treats it as the default).
pub(super) fn clamp_effort<'a>(effort: &'a str, model: &str) -> &'a str {
    let cap = model_max_effort(model);
    match (
        EFFORT_ORDER.iter().position(|&t| t == effort),
        EFFORT_ORDER.iter().position(|&t| t == cap),
    ) {
        (Some(e), Some(c)) if e > c => cap,
        _ => effort,
    }
}

/// Map a clamped effort tier to the CLI `--effort` flag value. Mirrors
/// `effortToFlag` in state/assistant/helpers.ts. Unknown/out-of-range tiers fall
/// through to `high` (the "deep" default) — `clamp_effort` runs first, so this
/// only sees a valid tier or a stale string the ladder doesn't define.
pub(super) fn effort_tier_to_flag(tier: &str) -> &'static str {
    match tier {
        "none" => "low",
        // "smart" = the responsive interactive default (Anthropic's recommended medium)
        "quick" | "smart" => "medium",
        "ultra" => "xhigh",
        _ /* "deep" or unknown */ => "high",
    }
}

/// The effort value actually sent to the CLI for a turn. #68: thinking-OFF must
/// still send `--effort low` (the CLI floor) — the CLI has no thinking-disable
/// flag and the no-think shim is bypassed on the OAuth path, so `low` is the only
/// real "minimal reasoning" lever. When thinking is on, send the tier's mapped
/// flag. Returns the flag string; the caller decides whether `--effort` is
/// emitted at all (local-LLM / haiku / old-CLI gates live at the call site).
pub(super) fn send_effort_flag(thinking_on: bool, effort_flag: &'static str) -> &'static str {
    if thinking_on {
        effort_flag
    } else {
        "low"
    }
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

/// The base URL is injected verbatim as `ANTHROPIC_BASE_URL`, so only allow
/// `http`/`https` (no `file:`/`javascript:`/etc.) with a non-empty host.
pub(super) fn is_valid_local_base_url(s: &str) -> bool {
    let rest = match s.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") => rest,
        _ => return false,
    };
    // Host is everything up to the first path/query/fragment delimiter.
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    !host.is_empty()
}

/// Claude Fable 5 — limited run Rift offers only through 2026-06-22. Past
/// sunset a stale pref/session pin falls back to `opus` instead of firing at
/// a retired model id.
pub(super) const FABLE_MODEL: &str = "claude-fable-5";
pub(super) const FABLE_SUNSET_EPOCH_SECS: u64 = 4_070_908_800; // 2099-01-01T00:00:00Z
/// Manual kill-switch. Owner call 2026-07-01: keep Fable ALWAYS VISIBLE (flag
/// `false`) even while the upstream Fable/Mythos access gate is up — so the row
/// is live and rolls the instant access returns, no code change needed. The
/// tradeoff is accepted: while the gate holds, a Fable turn returns "Claude
/// Fable 5 is currently unavailable" (anthropic.com/news/fable-mythos-access),
/// which Rift renders gracefully as a normal reply — this is EXPECTED, not a bug
/// to "fix" by re-gating. Mirrors the frontend `FABLE_DISABLED` (helpers.ts) —
/// mirror any change on both sides. Set `true` only to hard-pull Fable entirely.
pub(super) const FABLE_DISABLED: bool = false;

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

/// Haiku 4.5 — pulled by Anthropic 2026-06-26 (model removed). Kill-switch mirror
/// of Fable (frontend `HAIKU_DISABLED` in helpers.ts). A pinned/stale Haiku
/// session falls back to sonnet before the id can reach the API. Flip false to
/// restore if it returns; mirror on both sides.
pub(super) const HAIKU_MODEL: &str = "haiku";
pub(super) const HAIKU_FALLBACK_MODEL: &str = "sonnet";
pub(super) const HAIKU_DISABLED: bool = true;
pub(super) fn haiku_unavailable() -> bool {
    HAIKU_DISABLED
}

/// Claude Sonnet 5 — released 2026-06-09. The bare CLI alias `sonnet` still
/// resolves to `claude-sonnet-4-6` on shipped CLIs (the alias table lags the
/// release), so a turn sent as `sonnet` silently runs the previous generation.
/// `opus`/`haiku` aliases already resolve to their newest snapshot, so only
/// `sonnet` needs pinning. Mirror of `SONNET_MODEL` in helpers.ts.
pub(super) const SONNET_MODEL: &str = "claude-sonnet-5";

/// Resolve a renderer model selection to the explicit id sent to the CLI. Maps
/// the lagging `sonnet` alias → the pinned `claude-sonnet-5`; every other value
/// (opus/haiku/fable aliases + already-explicit ids) passes through unchanged.
/// Mirrors `canonicalModelAlias` in state/assistant/helpers.ts. Run AFTER the
/// per-conversation pin + Fable/Haiku guards so a new chat pins the canonical id.
pub(super) fn canonical_model_alias(model: &str) -> &str {
    if model == "sonnet" {
        SONNET_MODEL
    } else {
        model
    }
}

/// Sonnet ids the CLI gates to a 200K context window unless the `[1m]` window-
/// selector suffix is appended. The shipped CLI's model table treats bare
/// `claude-sonnet-5` (and `claude-sonnet-4-6`) as 200K-native, so it auto-
/// compacts at ~92% of 200K (~184K) — which read as "compacting at ~14%" against
/// Rift's correct 1M gauge (the reported bug). Verified empirically: bare
/// `--model claude-sonnet-5` → `modelUsage.contextWindow:200000`, while
/// `claude-sonnet-5[1m]` → `1000000`. Opus 4.x / Fable already default to 1M, and
/// Haiku is 200K-only — none of them need (or accept) the suffix here. Sonnet 4.5
/// is intentionally excluded: it's not in Rift's picker or resume path and its
/// `[1m]` support is flaky.
const SONNET_1M_GATED: [&str; 2] = ["claude-sonnet-5", "claude-sonnet-4-6"];

/// The exact string to pass to the CLI's `--model` arg for a fully-resolved model
/// id. Appends the `[1m]` window-selector for the Sonnet ids the CLI otherwise
/// gates at 200K, so the CLI's auto-compaction threshold matches Rift's 1M gauge.
/// LOAD-BEARING: this must be applied ONLY when building the live CLI arg — the
/// suffix must NEVER reach `save_session_model` (the pin), because
/// `is_valid_model_name` rejects `[`/`]` and a bracketed pin silently fails to
/// load on resume, un-pinning the session and breaking thinking-signature replay.
pub(super) fn cli_model_arg(model: &str) -> String {
    if SONNET_1M_GATED.contains(&model) {
        format!("{model}[1m]")
    } else {
        model.to_string()
    }
}

/// Read config.json with NO side effects — does not run the keychain
/// migration. Used by setters that need to inspect/clear the legacy plaintext
/// field before performing their own keychain op, where letting `load_config`'s
/// migration fire would re-write a stale value over the caller's change (RR7).
fn load_config_raw() -> AssistantConfig {
    match config_path().and_then(|p| std::fs::read_to_string(&p).map_err(|e| e.to_string())) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            log::warn!("assistant config unreadable — falling back to defaults: {e}");
            AssistantConfig::default()
        }),
        Err(_) => AssistantConfig::default(),
    }
}

pub(super) fn load_config() -> AssistantConfig {
    // Missing file = normal first run (silent default); a file that exists
    // but fails to parse means settings are being dropped — warn so it's
    // traceable instead of "my settings vanished".
    let mut cfg = load_config_raw();
    // Phase 6 (#37): one-shot migration of any plaintext api_key into the
    // OS keychain. Failure is non-fatal — the field stays in JSON for a
    // future attempt, and runtime reads still see it via legacy fallback in
    // current_api_key().
    if let Some(k) = cfg.api_key.as_deref().filter(|s| !s.is_empty()) {
        match crate::secrets::set(crate::secrets::ASSISTANT_API_KEY, k) {
            Ok(()) => {
                cfg.api_key = None;
                // Only persist the migration when the config lock is free.
                // load_config runs both unlocked (getters) and locked (setters,
                // same thread). A bare save here could land its tmp-rename after
                // a concurrent setter's, clobbering the setting. try_lock skips
                // the save when a setter already holds the lock (non-reentrant
                // std Mutex → WouldBlock on the same thread); the field stays in
                // JSON and re-migrates on the next cold, uncontended load.
                match CONFIG_WRITE_LOCK.try_lock() {
                    Ok(_guard) => {
                        if let Err(e) = save_config(&cfg) {
                            log::warn!("assistant: post-migration save_config failed: {e}");
                        } else {
                            log::info!("assistant: migrated api_key to keychain");
                        }
                    }
                    Err(_) => {
                        log::debug!("assistant: api_key migration save deferred — config lock busy");
                    }
                }
            }
            Err(e) => {
                log::warn!("assistant: keychain migration for api_key failed: {e}");
                // The plaintext api_key now stays in config.json indefinitely.
                // Harden the file's DACL so a domain-joined/shared profile's
                // inherited SYSTEM/Administrators read ACEs can't expose it —
                // mirrors the mcp-config lockdown. Best-effort. (RR9)
                if let Ok(p) = config_path() {
                    harden_config_acl(&p);
                }
            }
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

/// Same as `current_api_key` but reuses an already-loaded config — avoids a
/// second `load_config()` disk read on the per-turn hot path.
pub(super) fn current_api_key_with(cfg: &AssistantConfig) -> Option<String> {
    crate::secrets::get(crate::secrets::ASSISTANT_API_KEY)
        .or_else(|| cfg.api_key.clone().filter(|s| !s.is_empty()))
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
    if let Some(ref url) = normalized {
        if !is_valid_local_base_url(url) {
            return Err(format!("invalid base URL (need http(s):// + host): {url}"));
        }
    }
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
    // Empty/None → delete the keychain entry.
    //
    // CRITICAL ordering (RR7): clear the legacy plaintext field FIRST via a raw
    // (migration-free) load+save, THEN do the keychain op. If we did the keychain
    // op first and then called `load_config()`, its migration would fire against
    // the stale plaintext value and `secrets::set(old_key)` would overwrite the
    // key we just set (or re-create the entry we just deleted). The migration's
    // own save is gated on `try_lock` which we already hold here, so it would
    // also leave the plaintext in config.json — perpetuating the corruption on
    // every call. Using the raw loader avoids triggering migration at all.
    let mut cfg = load_config_raw();
    if cfg.api_key.is_some() {
        cfg.api_key = None;
        save_config(&cfg)?;
    }
    match api_key.as_deref().filter(|s| !s.is_empty()) {
        Some(k) => crate::secrets::set(crate::secrets::ASSISTANT_API_KEY, k)?,
        None => crate::secrets::delete(crate::secrets::ASSISTANT_API_KEY)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_model_alias, clamp_effort, cli_model_arg, effort_tier_to_flag,
        is_valid_effort_tier, is_valid_local_base_url, is_valid_model_name, model_max_effort,
        send_effort_flag, DEFAULT_MODEL, FABLE_FALLBACK_MODEL, SONNET_MODEL,
    };

    #[test]
    fn accepts_http_and_https_with_host() {
        assert!(is_valid_local_base_url("http://localhost:4000"));
        assert!(is_valid_local_base_url("https://127.0.0.1:4000/v1"));
        assert!(is_valid_local_base_url("HTTP://box.lan:8080"));
    }

    #[test]
    fn rejects_other_schemes_and_empty_host() {
        assert!(!is_valid_local_base_url("file:///etc/passwd"));
        assert!(!is_valid_local_base_url("ftp://host"));
        assert!(!is_valid_local_base_url("localhost:4000")); // no scheme
        assert!(!is_valid_local_base_url("http:///nohost"));
        assert!(!is_valid_local_base_url(""));
    }

    // The effort-ceiling contract — MUST stay in lockstep with the vitest cases
    // in src/lib/state/assistant/helpers.test.ts (clampEffort + effortToFlag).
    // Two independent suites encoding the same mappings surfaces one-sided drift
    // across the Rust/TS boundary in review.
    #[test]
    fn model_ceilings_match_capability_matrix() {
        assert_eq!(model_max_effort("opus"), "ultra");
        assert_eq!(model_max_effort("claude-opus-4-7"), "ultra");
        assert_eq!(model_max_effort("claude-fable-5"), "ultra");
        assert_eq!(model_max_effort("sonnet"), "ultra"); // Sonnet 5 honors xhigh
        assert_eq!(model_max_effort("haiku"), "none");
        assert_eq!(model_max_effort("some-future-model"), "ultra"); // unknown → top
    }

    #[test]
    fn clamp_caps_haiku_and_leaves_full_effort_models_untouched() {
        // Haiku floors to none; Sonnet 5/Opus/Fable all reach ultra now, so an
        // ultra(xhigh) pref passes through untouched.
        assert_eq!(clamp_effort("ultra", "haiku"), "none"); // floored
        assert_eq!(clamp_effort("ultra", "sonnet"), "ultra"); // Sonnet 5 in range
        assert_eq!(clamp_effort("deep", "sonnet"), "deep"); // in range
        assert_eq!(clamp_effort("quick", "sonnet"), "quick"); // already in range
        assert_eq!(clamp_effort("ultra", "opus"), "ultra");
        assert_eq!(clamp_effort("ultra", "claude-fable-5"), "ultra");
    }

    #[test]
    fn clamp_leaves_unknown_tier_untouched() {
        // An undefined tier isn't in EFFORT_ORDER; clamp passes it through so the
        // flag match downstream applies its "smart"/high default.
        assert_eq!(clamp_effort("bogus", "sonnet"), "bogus");
        assert!(!is_valid_effort_tier("bogus"));
        assert!(is_valid_effort_tier("ultra"));
    }

    // TC-001 (mega-audit cont.228): the tier→flag mapping was an untested inline
    // match in turn.rs — a wrong CLI `--effort` arg would ship silently. MUST stay
    // in lockstep with `effortToFlag` in helpers.ts (the 3-way effort invariant).
    #[test]
    fn effort_tier_maps_to_correct_cli_flag() {
        assert_eq!(effort_tier_to_flag("none"), "low");
        assert_eq!(effort_tier_to_flag("quick"), "medium");
        assert_eq!(effort_tier_to_flag("smart"), "medium");
        assert_eq!(effort_tier_to_flag("deep"), "high");
        assert_eq!(effort_tier_to_flag("ultra"), "xhigh");
        // A stale/unknown tier (clamp passes it through) falls back to high(deep).
        assert_eq!(effort_tier_to_flag("bogus"), "high");
    }

    // TC-002 (mega-audit cont.228): thinking-OFF MUST send `--effort low` (#68).
    // Untested → a silent regression reinstates the ~12s "slow hello" TTFT. This
    // is the v0.67.0 fast-default win; guard it.
    #[test]
    fn thinking_off_forces_low_regardless_of_tier() {
        // thinking on → the tier's own flag passes through unchanged.
        assert_eq!(send_effort_flag(true, "xhigh"), "xhigh");
        assert_eq!(send_effort_flag(true, "high"), "high");
        assert_eq!(send_effort_flag(true, "medium"), "medium");
        assert_eq!(send_effort_flag(true, "low"), "low");
        // thinking off → ALWAYS low, even when the tier maps higher.
        assert_eq!(send_effort_flag(false, "xhigh"), "low");
        assert_eq!(send_effort_flag(false, "high"), "low");
        assert_eq!(send_effort_flag(false, "medium"), "low");
    }

    #[test]
    fn fallback_consts_are_sane() {
        // Fable substitute must be a real non-Fable model; default must be set.
        assert_eq!(FABLE_FALLBACK_MODEL, "opus");
        assert_eq!(DEFAULT_MODEL, "sonnet");
        assert_ne!(FABLE_FALLBACK_MODEL, "claude-fable-5");
    }

    // The shipped CLI alias `sonnet` resolves to claude-sonnet-4-6, so the bare
    // alias silently ran the previous generation — the "still detected as 4.6"
    // bug. canonical_model_alias pins it to the explicit Sonnet 5 id; everything
    // else (opus/haiku/fable aliases + already-explicit ids) passes through. The
    // `sonnet`→claude-sonnet-5 mapping MUST mirror canonicalModelAlias in
    // helpers.ts. (The 4.6 RESUME-pin path is asserted by turn.rs's own logic,
    // not here — this helper only handles the forward `sonnet`→5 direction.)
    #[test]
    fn canonical_alias_pins_sonnet_and_passes_others() {
        assert_eq!(canonical_model_alias("sonnet"), SONNET_MODEL);
        assert_eq!(SONNET_MODEL, "claude-sonnet-5");
        // Other aliases resolve correctly in the CLI already — leave untouched.
        assert_eq!(canonical_model_alias("opus"), "opus");
        assert_eq!(canonical_model_alias("haiku"), "haiku");
        assert_eq!(canonical_model_alias("claude-fable-5"), "claude-fable-5");
        // Already-explicit ids (incl. an explicit Sonnet pin) pass through.
        assert_eq!(canonical_model_alias("claude-sonnet-5"), "claude-sonnet-5");
        assert_eq!(canonical_model_alias("claude-sonnet-4-6"), "claude-sonnet-4-6");
        assert_eq!(canonical_model_alias("claude-opus-4-8"), "claude-opus-4-8");
    }

    // `cli_model_arg` appends the `[1m]` window-selector ONLY for the Sonnet ids
    // the CLI gates at 200K, so its auto-compaction threshold matches Rift's 1M
    // gauge (the "compacting at 14%" bug). Opus/Fable already default to 1M and
    // Haiku is 200K-only — none get the suffix. The bare ids (what
    // save_session_model pins) must round-trip is_valid_model_name; the `[1m]`
    // arg must NOT (it's never persisted) — both asserted here so a future
    // refactor can't collapse the arg + pin into one value.
    #[test]
    fn cli_model_arg_adds_1m_suffix_for_gated_sonnets_only() {
        // 1M-gated Sonnets get the suffix.
        assert_eq!(cli_model_arg("claude-sonnet-5"), "claude-sonnet-5[1m]");
        assert_eq!(cli_model_arg("claude-sonnet-4-6"), "claude-sonnet-4-6[1m]");
        // 1M-native or 200K-only models pass through unchanged.
        assert_eq!(cli_model_arg("claude-opus-4-8"), "claude-opus-4-8");
        assert_eq!(cli_model_arg("claude-opus-4-7"), "claude-opus-4-7");
        assert_eq!(cli_model_arg("claude-fable-5"), "claude-fable-5");
        assert_eq!(cli_model_arg("haiku"), "haiku");
        // Sonnet 4.5 is excluded (flaky [1m] support, not in Rift's resume path).
        assert_eq!(cli_model_arg("claude-sonnet-4-5"), "claude-sonnet-4-5");
        // The suffixed arg is NOT a valid model name (so it can never be pinned),
        // while every bare id the pin sees IS valid.
        assert!(!is_valid_model_name(&cli_model_arg("claude-sonnet-5")));
        assert!(is_valid_model_name("claude-sonnet-5"));
        assert!(is_valid_model_name("claude-sonnet-4-6"));
    }
}
