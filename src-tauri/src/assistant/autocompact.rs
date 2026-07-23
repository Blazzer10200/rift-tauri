//! CLI auto-compact configuration probe. The FE's compaction detection (the
//! "Auto-compacting conversation…" card + the context health nudge) used to
//! assume the CLI compacts near window-full — wrong the moment the user tunes
//! auto-compaction (e.g. `CLAUDE_CODE_AUTO_COMPACT_WINDOW=312500` +
//! `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE=80` fires at 250K = 25% of a 1M window,
//! so an 80%-of-window gate never trips). This mirrors the CLI's own
//! resolution (verified against the 2.1.217 binary):
//!   enabled — `DISABLE_AUTO_COMPACT` env kills it, else `autoCompactEnabled`
//!             from `~/.claude.json` (default true);
//!   window  — `CLAUDE_CODE_AUTO_COMPACT_WINDOW` env, else settings key
//!             `autoCompactWindow` (local > project > user), else model default;
//!   pct     — `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` env only.
//! Env vars resolve process-env first, then settings-file `env` blocks. The
//! `user` settings source only applies in full-config mode (mirrors turn.rs's
//! `--setting-sources` choice); project/local always apply.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCompactCfg {
    pub enabled: bool,
    pub window_tokens: Option<u64>,
    pub pct: Option<f64>,
}

fn truthy(v: &str) -> bool {
    !matches!(v.trim().to_ascii_lowercase().as_str(), "" | "0" | "false" | "no" | "off")
}

fn read_json(path: &std::path::Path) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// Pure resolution over already-loaded sources. `settings` is ordered
/// highest-precedence first (local, project, user); `proc_env` abstracts
/// `std::env::var` for tests.
fn resolve(
    settings: &[serde_json::Value],
    legacy_global: Option<&serde_json::Value>,
    proc_env: impl Fn(&str) -> Option<String>,
) -> AutoCompactCfg {
    let env_of = |key: &str| -> Option<String> {
        if let Some(v) = proc_env(key) {
            return Some(v);
        }
        for s in settings {
            match s.get("env").and_then(|e| e.get(key)) {
                Some(serde_json::Value::String(x)) => return Some(x.clone()),
                Some(serde_json::Value::Number(n)) => return Some(n.to_string()),
                _ => {}
            }
        }
        None
    };
    let enabled = if env_of("DISABLE_AUTO_COMPACT").map(|v| truthy(&v)).unwrap_or(false) {
        false
    } else {
        legacy_global
            .and_then(|g| g.get("autoCompactEnabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    };
    let window_tokens = env_of("CLAUDE_CODE_AUTO_COMPACT_WINDOW")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .or_else(|| settings.iter().find_map(|s| s.get("autoCompactWindow").and_then(|v| v.as_u64())))
        .filter(|w| *w > 0);
    let pct = env_of("CLAUDE_AUTOCOMPACT_PCT_OVERRIDE")
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|p| p.is_finite() && *p > 0.0 && *p <= 100.0);
    AutoCompactCfg { enabled, window_tokens, pct }
}

/// Probed once at FE boot + re-probed when the full-config toggle flips (that
/// changes whether the user settings source applies). Never errors — an
/// unreadable file just drops out of the resolution.
#[tauri::command]
pub fn assistant_autocompact_config() -> AutoCompactCfg {
    let cfg = super::config::load_config();
    let full = cfg.use_full_config.unwrap_or(true)
        && super::config::current_api_key_with(&cfg).is_none();
    let home = super::dirs_home().ok();
    let mut files: Vec<PathBuf> = Vec::new();
    if let Some(root) = &cfg.current_root {
        files.push(root.join(".claude").join("settings.local.json"));
        files.push(root.join(".claude").join("settings.json"));
    }
    if full {
        if let Some(h) = &home {
            files.push(h.join(".claude").join("settings.json"));
        }
    }
    let settings: Vec<serde_json::Value> = files.iter().filter_map(|p| read_json(p)).collect();
    let legacy = home.as_ref().and_then(|h| read_json(&h.join(".claude.json")));
    resolve(&settings, legacy.as_ref(), |k| std::env::var(k).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn defaults_when_nothing_configured() {
        let got = resolve(&[], None, no_env);
        assert_eq!(got, AutoCompactCfg { enabled: true, window_tokens: None, pct: None });
    }

    #[test]
    fn settings_env_block_supplies_window_and_pct() {
        // The real repro: user settings.json env block tunes both knobs.
        let user = json!({ "env": {
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "312500",
            "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "80"
        }});
        let got = resolve(&[user], None, no_env);
        assert_eq!(got, AutoCompactCfg { enabled: true, window_tokens: Some(312_500), pct: Some(80.0) });
    }

    #[test]
    fn process_env_beats_settings_env() {
        let user = json!({ "env": { "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "312500" } });
        let got = resolve(&[user], None, |k| {
            (k == "CLAUDE_CODE_AUTO_COMPACT_WINDOW").then(|| "100000".to_string())
        });
        assert_eq!(got.window_tokens, Some(100_000));
    }

    #[test]
    fn settings_key_and_source_order() {
        // Bare `autoCompactWindow` settings key works; first (higher-precedence)
        // source wins over later ones.
        let local = json!({ "autoCompactWindow": 150000 });
        let user = json!({ "autoCompactWindow": 312500 });
        assert_eq!(resolve(&[local, user], None, no_env).window_tokens, Some(150_000));
    }

    #[test]
    fn disabled_paths() {
        let legacy = json!({ "autoCompactEnabled": false });
        assert!(!resolve(&[], Some(&legacy), no_env).enabled);
        let got = resolve(&[], None, |k| (k == "DISABLE_AUTO_COMPACT").then(|| "1".to_string()));
        assert!(!got.enabled);
        // "false"/"0" strings are NOT truthy — still enabled.
        let got = resolve(&[], None, |k| (k == "DISABLE_AUTO_COMPACT").then(|| "false".to_string()));
        assert!(got.enabled);
    }

    #[test]
    fn garbage_values_drop_out() {
        let user = json!({ "env": {
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "not-a-number",
            "CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "250"
        }});
        let got = resolve(&[user], None, no_env);
        assert_eq!(got.window_tokens, None);
        assert_eq!(got.pct, None);
    }
}
