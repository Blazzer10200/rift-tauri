//! Live subscription rate-limit gauge — the same data Claude Code's `/usage`
//! screen shows (5-hour window %, weekly %, reset times), fetched from the
//! undocumented OAuth usage endpoint with the CLI's own token from
//! `~/.claude/.credentials.json`. Read-only on the token: we NEVER refresh it
//! (refresh tokens are one-time-use; an external refresh would break the
//! CLI's own auth loop). 60s in-process cache keeps polling polite.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const CACHE_TTL: Duration = Duration::from_secs(60);

/// One rolling rate-limit window. `utilization` is 0–100.
#[derive(Clone, Serialize, Deserialize)]
pub struct LimitWindow {
    // Lenient like ScopedLimit.percent below: this undocumented endpoint has
    // been observed returning `null` for a utilization field, and a present-but-
    // wrong-typed value would otherwise fail the WHOLE RateLimits parse — not
    // just this window. That's the higher-consequence case here: `five_hour`/
    // `seven_day` utilization drives turn.rs's 90/95% plan-usage hot-gate every
    // turn, so a blanked parse silently masks a near-limit user. Coerce to 0.0.
    #[serde(default, deserialize_with = "de_percent_lenient")]
    pub utilization: f64,
    #[serde(rename = "resetsAt", alias = "resets_at")]
    pub resets_at: Option<String>,
}

/// All fields lenient (see `de_*_lenient` below): the endpoint is undocumented,
/// and `ExtraUsage` rides inside the same `RateLimits` deserialize as the
/// rate-limit windows — so a single wrong-typed extra-usage sub-field would
/// otherwise fail the WHOLE parse and blank five_hour/seven_day/limits[] too,
/// not just this widget. Each bad value degrades to a benign default instead.
#[derive(Clone, Serialize, Deserialize)]
pub struct ExtraUsage {
    #[serde(rename = "isEnabled", alias = "is_enabled", default, deserialize_with = "de_bool_lenient")]
    pub is_enabled: bool,
    /// Spend cap + spend-to-date in MINOR currency units (e.g. cents). The
    /// endpoint returns integers scaled by `decimal_places` — 8000 w/ dp=2 is
    /// $80.00, NOT $8000. The frontend divides by 10^decimal_places to format.
    #[serde(rename = "monthlyLimit", alias = "monthly_limit", default, deserialize_with = "de_opt_f64_lenient")]
    pub monthly_limit: Option<f64>,
    #[serde(rename = "usedCredits", alias = "used_credits", default, deserialize_with = "de_opt_f64_lenient")]
    pub used_credits: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_f64_lenient")]
    pub utilization: Option<f64>,
    #[serde(default, deserialize_with = "de_opt_string_lenient")]
    pub currency: Option<String>,
    /// Minor-unit exponent for monthly_limit/used_credits (2 ⇒ divide by 100).
    /// Defaults to 2 when the endpoint omits it (the observed USD default).
    #[serde(rename = "decimalPlaces", alias = "decimal_places", default = "default_decimal_places", deserialize_with = "de_decimal_places_lenient")]
    pub decimal_places: u32,
}

fn default_decimal_places() -> u32 {
    2
}

/// Coerce a wrong-typed optional number to `None` instead of failing the parse.
fn de_opt_f64_lenient<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(serde_json::Value::deserialize(deserializer)?.as_f64())
}

/// Coerce a wrong-typed optional string to `None` (keeps a real string).
fn de_opt_string_lenient<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(s) => Some(s),
        _ => None,
    })
}

/// Coerce a wrong-typed bool to `false` instead of failing the parse.
fn de_bool_lenient<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(serde_json::Value::deserialize(deserializer)?.as_bool().unwrap_or(false))
}

/// Coerce a wrong-typed / out-of-range `decimal_places` to the USD default (2).
fn de_decimal_places_lenient<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(serde_json::Value::deserialize(deserializer)?
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or_else(default_decimal_places))
}

/// One entry of the endpoint's newer generic `limits[]` array — the legacy
/// model buckets (`seven_day_opus`/`seven_day_sonnet`) now come back null and
/// model-scoped windows (e.g. Fable) only exist here.
#[derive(Clone, Serialize, Deserialize)]
pub struct ScopedLimit {
    pub kind: Option<String>,
    pub group: Option<String>,
    // `f64` (not Option) but via a lenient deserializer: the endpoint is
    // undocumented, so a present-but-wrong-typed value (string/null/bool) must
    // NOT fail the whole `limits[]` array — that would blank the entire usage
    // panel over one bad element. `#[serde(default)]` alone only covers a
    // MISSING key, not a wrong-typed one.
    #[serde(default, deserialize_with = "de_percent_lenient")]
    pub percent: f64,
    pub severity: Option<String>,
    #[serde(rename = "resetsAt", alias = "resets_at")]
    pub resets_at: Option<String>,
    pub scope: Option<LimitScope>,
    #[serde(rename = "isActive", alias = "is_active", default)]
    pub is_active: bool,
}

/// Coerce a wrong-typed `percent` (string/null/bool/object) to 0.0 instead of
/// failing the whole `ScopedLimit` (and thus the whole `limits[]` Vec).
fn de_percent_lenient<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(serde_json::Value::deserialize(deserializer)?.as_f64().unwrap_or(0.0))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LimitScope {
    pub model: Option<ScopedModel>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ScopedModel {
    #[serde(rename = "displayName", alias = "display_name")]
    pub display_name: Option<String>,
}

/// Deserializes the endpoint's snake_case body (via `alias`), serializes
/// camelCase to the frontend. Unknown buckets in the response are ignored.
#[derive(Clone, Serialize, Deserialize)]
pub struct RateLimits {
    #[serde(rename = "fiveHour", alias = "five_hour")]
    pub five_hour: Option<LimitWindow>,
    #[serde(rename = "sevenDay", alias = "seven_day")]
    pub seven_day: Option<LimitWindow>,
    #[serde(rename = "sevenDayOpus", alias = "seven_day_opus")]
    pub seven_day_opus: Option<LimitWindow>,
    #[serde(rename = "sevenDaySonnet", alias = "seven_day_sonnet")]
    pub seven_day_sonnet: Option<LimitWindow>,
    #[serde(rename = "extraUsage", alias = "extra_usage")]
    pub extra_usage: Option<ExtraUsage>,
    /// Newer generic window list (session / weekly_all / weekly_scoped per
    /// model). Preferred by the frontend when non-empty.
    #[serde(default)]
    pub limits: Vec<ScopedLimit>,
    /// Set by us, not the API — ms epoch of the fetch (drives "as of" UI).
    #[serde(rename = "fetchedAt", default)]
    pub fetched_at: i64,
}

static CACHE: Mutex<Option<(Instant, RateLimits)>> = Mutex::new(None);
/// Last failed-attempt timestamp — lets a permanently-failing user (API-key /
/// signed-out / expired) back off on the same TTL as a success, instead of
/// hot-looping a fetch+fail every single turn for the app's lifetime.
static LAST_FAILURE: Mutex<Option<Instant>> = Mutex::new(None);

/// Non-blocking cache read for the per-turn "Rift environment snapshot"
/// (`turn.rs`). Accepts a slightly stale value (≤5 min) — utilization moves
/// slowly and the snapshot must never add an HTTP round-trip to turn start.
pub fn cached_snapshot() -> Option<RateLimits> {
    const SNAPSHOT_TTL: Duration = Duration::from_secs(300);
    CACHE
        .lock()
        .ok()?
        .as_ref()
        .and_then(|(at, l)| (at.elapsed() < SNAPSHOT_TTL).then(|| l.clone()))
}

/// Fire-and-forget refresh from turn start: when the 60s cache is stale,
/// refetch in the background so the NEXT turn's snapshot is warm. Errors are
/// expected for whole user classes (API-key users have no OAuth creds) — the
/// gauge is optional enrichment, so they log at debug instead of surfacing.
pub fn spawn_background_refresh() {
    {
        let Ok(guard) = CACHE.lock() else { return };
        if let Some((at, _)) = guard.as_ref() {
            if at.elapsed() < CACHE_TTL {
                return;
            }
        }
    }
    {
        let Ok(guard) = LAST_FAILURE.lock() else { return };
        if let Some(at) = guard.as_ref() {
            if at.elapsed() < CACHE_TTL {
                return;
            }
        }
    }
    // Cap concurrent refreshes at one: a message queue draining back-to-back hits
    // spawn_background_refresh once per turn, all seeing the same stale cache —
    // without this each would spawn its own HTTP task. Cleared when the task ends.
    static REFRESH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
    if REFRESH_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    tauri::async_runtime::spawn(async {
        if let Err(e) = usage_rate_limits(None, None).await {
            // Record the failed attempt so the TTL gate above backs off a
            // permanently-failing user (API-key/signed-out/expired) instead of
            // re-fetching every turn for the app's lifetime.
            if let Ok(mut guard) = LAST_FAILURE.lock() {
                *guard = Some(Instant::now());
            }
            // The three swallowed states (missing creds / API-key user / expired
            // token) look identical at debug level — classify so the diagnostics
            // console can distinguish "needs login" from "broken" at a glance.
            let reason = classify_usage_error(&e);
            crate::diagnostics::emit_with_fields(
                crate::diagnostics::DiagStage::Log,
                crate::diagnostics::DiagLevel::Debug,
                Some("usage"),
                Some(file!()),
                "usage limits background refresh skipped",
                serde_json::json!({ "reason": reason, "detail": e }),
            );
        }
        REFRESH_IN_FLIGHT.store(false, Ordering::Release);
    });
}

/// Map a `read_oauth_token`/refresh error string to a stable reason enum so the
/// diagnostics console + a future health roll-up can branch on it instead of the
/// free-text message. Order matters: expired is checked before the generic
/// missing-login fallback.
fn classify_usage_error(e: &str) -> &'static str {
    if e.contains("not available for API-key users") {
        "api_key_user"
    } else if e.contains("token expired") {
        "expired"
    } else if e.contains("could not parse") {
        "parse_error"
    } else if e.contains("needs a Claude subscription login") {
        "missing_login"
    } else {
        "other"
    }
}

#[derive(Deserialize)]
struct CredsFile {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OauthCreds>,
}

#[derive(Deserialize)]
struct OauthCreds {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: Option<i64>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        // Safe-fail on a broken clock (pre-1970): treat every token as expired
        // so the user sees the clear "expired" message, not a confusing 401.
        // `unwrap_or(0)` would do the reverse — mark every token unexpired.
        .unwrap_or(i64::MAX)
}

/// Real-or-zero epoch ms for stamping (NOT expiry checks). now_ms()'s
/// fail-EXPIRED `i64::MAX` sentinel is correct for the token check but wrong as
/// a `fetched_at` timestamp — it would render an "as of year 292-million" label
/// on a broken clock. 0 is a recognizable "unknown" the FE can treat as such.
fn stamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn read_oauth_token() -> Result<String, String> {
    let home = crate::state::paths::dirs_home().map_err(|e| e.to_string())?;
    let path = home.join(".claude").join(".credentials.json");
    let bytes = std::fs::read(&path)
        .map_err(|e| format!("usage gauge needs a Claude subscription login (checked {}): {e}", path.display()))?;
    let creds: CredsFile = serde_json::from_slice(&bytes)
        .map_err(|_| "could not parse Claude credentials".to_string())?;
    let oauth = creds
        .claude_ai_oauth
        .ok_or_else(|| "usage gauge requires a Claude subscription (not available for API-key users)".to_string())?;
    if let Some(exp) = oauth.expires_at {
        if exp < now_ms() {
            return Err("Claude login token expired — it refreshes next time the CLI runs".into());
        }
    }
    Ok(oauth.access_token)
}

/// Fetch live plan-limit utilization. `cli_version` feeds the User-Agent —
/// without a `claude-code/<ver>` UA the endpoint throttles aggressively.
/// `force` skips the 60s cache read (manual refresh button); the fresh result
/// still lands in the cache for everyone else.
#[tauri::command]
pub async fn usage_rate_limits(cli_version: Option<String>, force: Option<bool>) -> Result<RateLimits, String> {
    if !force.unwrap_or(false) {
        if let Some((at, cached)) = CACHE.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
            if at.elapsed() < CACHE_TTL {
                return Ok(cached.clone());
            }
        }
    }

    let token = tauri::async_runtime::spawn_blocking(read_oauth_token)
        .await
        .map_err(|e| format!("spawn_blocking failed: {e}"))??;
    // Sanitize the frontend-supplied version (header value, digits/dots only).
    let ver: String = cli_version
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let ua = format!(
        "claude-code/{}",
        if ver.is_empty() { env!("CARGO_PKG_VERSION") } else { ver.as_str() }
    );

    let resp = crate::certs::usage_client()
        .get(USAGE_URL)
        .header("Authorization", format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("User-Agent", ua)
        .send()
        .await
        .map_err(|e| format!("usage endpoint unreachable: {e}"))?;

    match resp.status().as_u16() {
        401 => return Err("Claude session expired — open Settings and use Sign In to re-authenticate".into()),
        429 => return Err("usage endpoint rate-limited — retry in a few minutes".into()),
        s if !(200..300).contains(&s) => {
            return Err(format!("usage endpoint returned HTTP {s}"));
        }
        _ => {}
    }

    let mut limits: RateLimits = resp
        .json()
        .await
        .map_err(|e| format!("unexpected usage response shape: {e}"))?;
    limits.fetched_at = stamp_ms();
    *CACHE.lock().unwrap_or_else(|p| p.into_inner()) = Some((Instant::now(), limits.clone()));
    Ok(limits)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed copy of a real 2026-07 endpoint response: legacy model buckets
    /// null, generic limits[] carrying the model-scoped (Fable) window, plus
    /// unknown future buckets that must parse-tolerate.
    const FIXTURE: &str = r#"{
      "five_hour": {"utilization": 16, "resets_at": "2026-07-02T07:09:59+00:00", "limit_dollars": null},
      "seven_day": {"utilization": 10, "resets_at": "2026-07-07T14:59:59+00:00"},
      "seven_day_opus": null, "seven_day_sonnet": null,
      "seven_day_cowork": null, "tangelo": null, "iguana_necktie": null,
      "extra_usage": {"is_enabled": true, "monthly_limit": 8000, "used_credits": 0, "utilization": null, "currency": "USD", "decimal_places": 2, "daily": null},
      "limits": [
        {"kind": "session", "group": "session", "percent": 16, "severity": "normal", "resets_at": "2026-07-02T07:09:59+00:00", "scope": null, "is_active": true},
        {"kind": "weekly_all", "group": "weekly", "percent": 10, "severity": "normal", "resets_at": "2026-07-07T14:59:59+00:00", "scope": null, "is_active": false},
        {"kind": "weekly_scoped", "group": "weekly", "percent": 16, "severity": "normal", "resets_at": "2026-07-07T14:59:59+00:00", "scope": {"model": {"id": null, "display_name": "Fable"}, "surface": null}, "is_active": false}
      ],
      "spend": {"percent": 0}
    }"#;

    #[test]
    fn parses_generic_limits_and_extra_usage() {
        let l: RateLimits = serde_json::from_str(FIXTURE).expect("fixture parses");
        assert!(l.seven_day_opus.is_none());
        assert_eq!(l.limits.len(), 3);
        assert!(l.limits[0].is_active);
        let fable = &l.limits[2];
        assert_eq!(fable.kind.as_deref(), Some("weekly_scoped"));
        assert_eq!(
            fable.scope.as_ref().and_then(|s| s.model.as_ref()).and_then(|m| m.display_name.as_deref()),
            Some("Fable")
        );
        assert!((fable.percent - 16.0).abs() < f64::EPSILON);
        let x = l.extra_usage.expect("extra_usage present");
        assert!(x.is_enabled);
        assert_eq!(x.monthly_limit, Some(8000.0));
        assert_eq!(x.decimal_places, 2);
    }

    /// The frontend contract is camelCase — a serde rename regression here
    /// silently blanks the panel, so pin the wire shape.
    #[test]
    fn serializes_camelcase_for_frontend() {
        let l: RateLimits = serde_json::from_str(FIXTURE).expect("fixture parses");
        let out = serde_json::to_string(&l).expect("serializes");
        for key in ["\"fiveHour\"", "\"resetsAt\"", "\"isActive\"", "\"displayName\"", "\"extraUsage\"", "\"limits\""] {
            assert!(out.contains(key), "missing {key} in serialized output");
        }
        assert!(!out.contains("resets_at"), "snake_case leaked to frontend");
    }

    /// A body without the newer array (older endpoint) must still parse —
    /// `limits` defaults empty and the frontend falls back to legacy buckets.
    #[test]
    fn tolerates_missing_limits_array() {
        let l: RateLimits =
            serde_json::from_str(r#"{"five_hour": {"utilization": 5, "resets_at": null}}"#).expect("minimal body parses");
        assert!(l.limits.is_empty());
        assert!((l.five_hour.expect("five_hour").utilization - 5.0).abs() < f64::EPSILON);
    }

    /// r4 regression: a wrong-typed window `utilization` (this undocumented
    /// endpoint has returned null; a string is equally possible) must NOT fail
    /// the WHOLE parse — that would blank seven_day + the limits[] array and
    /// silently disable turn.rs's plan-usage hot-gate. It coerces to 0.0 and the
    /// rest of the response survives.
    #[test]
    fn wrong_typed_window_utilization_does_not_blank_response() {
        let body = r#"{
          "five_hour": {"utilization": null, "resets_at": "x"},
          "seven_day": {"utilization": "42", "resets_at": "y"},
          "limits": [{"kind": "session", "percent": 30, "is_active": true}]
        }"#;
        let l: RateLimits = serde_json::from_str(body).expect("wrong-typed utilization must not fail the parse");
        assert_eq!(l.five_hour.expect("five_hour present").utilization, 0.0);
        assert_eq!(l.seven_day.expect("seven_day present").utilization, 0.0);
        assert_eq!(l.limits.len(), 1, "limits[] survives a bad sibling window");
    }

    /// r4 regression: a wrong-typed extra_usage sub-field must degrade that
    /// widget only, not fail the whole RateLimits deserialize.
    #[test]
    fn wrong_typed_extra_usage_field_does_not_blank_response() {
        let body = r#"{
          "five_hour": {"utilization": 12, "resets_at": "x"},
          "extra_usage": {"is_enabled": "yes", "monthly_limit": "8000", "decimal_places": "2", "currency": 5}
        }"#;
        let l: RateLimits = serde_json::from_str(body).expect("bad extra_usage must not fail the parse");
        assert!((l.five_hour.expect("five_hour").utilization - 12.0).abs() < f64::EPSILON);
        let x = l.extra_usage.expect("extra_usage still present");
        assert!(!x.is_enabled, "non-bool is_enabled → false");
        assert_eq!(x.monthly_limit, None, "string monthly_limit → None");
        assert_eq!(x.decimal_places, 2, "string decimal_places → USD default");
        assert_eq!(x.currency, None, "numeric currency → None");
    }
}
