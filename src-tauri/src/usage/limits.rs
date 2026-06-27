//! Live subscription rate-limit gauge — the same data Claude Code's `/usage`
//! screen shows (5-hour window %, weekly %, reset times), fetched from the
//! undocumented OAuth usage endpoint with the CLI's own token from
//! `~/.claude/.credentials.json`. Read-only on the token: we NEVER refresh it
//! (refresh tokens are one-time-use; an external refresh would break the
//! CLI's own auth loop). 60s in-process cache keeps polling polite.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const CACHE_TTL: Duration = Duration::from_secs(60);

/// One rolling rate-limit window. `utilization` is 0–100.
#[derive(Clone, Serialize, Deserialize)]
pub struct LimitWindow {
    pub utilization: f64,
    #[serde(rename = "resetsAt", alias = "resets_at")]
    pub resets_at: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ExtraUsage {
    #[serde(rename = "isEnabled", alias = "is_enabled", default)]
    pub is_enabled: bool,
    /// Spend cap + spend-to-date in MINOR currency units (e.g. cents). The
    /// endpoint returns integers scaled by `decimal_places` — 8000 w/ dp=2 is
    /// $80.00, NOT $8000. The frontend divides by 10^decimal_places to format.
    #[serde(rename = "monthlyLimit", alias = "monthly_limit")]
    pub monthly_limit: Option<f64>,
    #[serde(rename = "usedCredits", alias = "used_credits")]
    pub used_credits: Option<f64>,
    pub utilization: Option<f64>,
    pub currency: Option<String>,
    /// Minor-unit exponent for monthly_limit/used_credits (2 ⇒ divide by 100).
    /// Defaults to 2 when the endpoint omits it (the observed USD default).
    #[serde(rename = "decimalPlaces", alias = "decimal_places", default = "default_decimal_places")]
    pub decimal_places: u32,
}

fn default_decimal_places() -> u32 {
    2
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
    /// Set by us, not the API — ms epoch of the fetch (drives "as of" UI).
    #[serde(rename = "fetchedAt", default)]
    pub fetched_at: i64,
}

static CACHE: Mutex<Option<(Instant, RateLimits)>> = Mutex::new(None);

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
    tauri::async_runtime::spawn(async {
        if let Err(e) = usage_rate_limits(None).await {
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
#[tauri::command]
pub async fn usage_rate_limits(cli_version: Option<String>) -> Result<RateLimits, String> {
    if let Some((at, cached)) = CACHE.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
        if at.elapsed() < CACHE_TTL {
            return Ok(cached.clone());
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
    limits.fetched_at = now_ms();
    *CACHE.lock().unwrap_or_else(|p| p.into_inner()) = Some((Instant::now(), limits.clone()));
    Ok(limits)
}
