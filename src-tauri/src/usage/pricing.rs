//! Static model price table + per-turn cost recompute (idea-phase-plan §1b / D2).
//!
//! The bundled `assets/model-prices.json` is embedded at compile time so cost
//! recompute never depends on runtime asset resolution. A user override at
//! `~/.rift/model-prices.json` is merged on top (user entries win) so a
//! custom-provider's rates (e.g. DeepSeek) can be supplied — the CLI's
//! `total_cost_usd` is wrong/absent for those, so the table is the only
//! trustworthy source. A turn whose model isn't in the table gets a NULL
//! `cost_usd_calc` (the "estimated/unknown pricing" flag the UI surfaces).

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

const BUNDLED: &str = include_str!("../../assets/model-prices.json");

/// USD per million tokens for one model. `cache_write` is the 5-minute-TTL
/// write rate; `cache_read` the cache-hit rate.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelPrice {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default)]
    pub cache_write: f64,
    #[serde(default)]
    pub cache_read: f64,
}

pub struct PriceTable {
    map: HashMap<String, ModelPrice>,
}

impl PriceTable {
    /// Load bundled prices, then merge the optional `~/.rift/model-prices.json`
    /// override on top. Tolerant: non-object entries (e.g. the `_comment` key)
    /// and malformed override files are skipped, never fatal.
    pub fn load() -> Self {
        let mut map = parse_table(BUNDLED);
        if let Ok(home) = crate::state::paths::dirs_home() {
            let p = home.join(".rift").join("model-prices.json");
            if let Ok(s) = std::fs::read_to_string(&p) {
                for (k, v) in parse_table(&s) {
                    map.insert(k, v);
                }
            }
        }
        PriceTable { map }
    }

    /// Cost in USD for one turn's token counts. `None` when the model has no
    /// price entry — caller stores NULL so the UI can flag it as estimated.
    pub fn cost_for(
        &self,
        model_id: &str,
        input: i64,
        output: i64,
        cache_read: i64,
        cache_write: i64,
    ) -> Option<f64> {
        let p = self.lookup(model_id)?;
        let m = 1_000_000.0;
        Some(
            (input as f64) * p.input / m
                + (output as f64) * p.output / m
                + (cache_read as f64) * p.cache_read / m
                + (cache_write as f64) * p.cache_write / m,
        )
    }

    /// Exact match, else the longest table key that is a prefix of `model_id`
    /// (CLI model ids carry date suffixes, e.g. `claude-opus-4-8-20260101`).
    fn lookup(&self, model_id: &str) -> Option<&ModelPrice> {
        if let Some(p) = self.map.get(model_id) {
            return Some(p);
        }
        self.map
            .iter()
            .filter(|(k, _)| model_id.starts_with(k.as_str()))
            .max_by_key(|(k, _)| k.len())
            .map(|(_, v)| v)
    }
}

/// Parse a `{ model_id: {input,output,...} }` JSON blob into prices, skipping
/// any entry whose value isn't a well-formed price object.
fn parse_table(s: &str) -> HashMap<String, ModelPrice> {
    let raw: HashMap<String, Value> = serde_json::from_str(s).unwrap_or_default();
    raw.into_iter()
        .filter_map(|(k, v)| serde_json::from_value::<ModelPrice>(v).ok().map(|p| (k, p)))
        .collect()
}

/// Best-effort provider tag from the model id. Anthropic ids start with
/// `claude`; everything else is a custom/escape-hatch provider.
pub fn provider_for_model(model_id: &str) -> &'static str {
    if model_id.starts_with("claude") {
        "anthropic"
    } else {
        "custom"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_from(extra: &str) -> PriceTable {
        let mut map = parse_table(BUNDLED);
        for (k, v) in parse_table(extra) {
            map.insert(k, v);
        }
        PriceTable { map }
    }

    #[test]
    fn comment_key_is_skipped() {
        assert!(!parse_table(BUNDLED).contains_key("_comment"));
    }

    #[test]
    fn dated_opus_id_resolves_via_prefix() {
        // 1M input @ $5/Mtok = $5.00 — exercises longest-prefix lookup on a
        // date-suffixed CLI model id.
        let c = table_from("{}")
            .cost_for("claude-opus-4-8-20260101", 1_000_000, 0, 0, 0)
            .unwrap();
        assert!((c - 5.0).abs() < 1e-9, "got {c}");
    }

    #[test]
    fn unknown_custom_model_is_unpriced() {
        // No entry → None → row stores NULL cost_usd_calc (estimated flag).
        assert!(table_from("{}")
            .cost_for("deepseek-chat", 1000, 1000, 0, 0)
            .is_none());
    }

    #[test]
    fn user_priced_custom_model_computes_cost() {
        // DeepSeek-style entry the user adds to ~/.rift/model-prices.json. The
        // CLI's total_cost_usd is wrong for this provider; the table is correct.
        let t = table_from(
            r#"{ "deepseek-chat": { "input": 0.27, "output": 1.10, "cache_write": 0.27, "cache_read": 0.07 } }"#,
        );
        // 2M in @0.27 + 1M out @1.10 = 0.54 + 1.10 = 1.64
        let c = t.cost_for("deepseek-chat", 2_000_000, 1_000_000, 0, 0).unwrap();
        assert!((c - 1.64).abs() < 1e-9, "got {c}");
    }

    #[test]
    fn provider_tag() {
        assert_eq!(provider_for_model("claude-opus-4-8"), "anthropic");
        assert_eq!(provider_for_model("deepseek-chat"), "custom");
    }
}
