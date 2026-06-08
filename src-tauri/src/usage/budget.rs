//! Credit-pool fuel gauge (idea-phase-plan §1d). The user picks a plan tier
//! ($20 Pro / $100 Max-5x / $200 Max-20x, or a custom cap) and a reset cadence
//! (monthly / weekly / daily). `usage_budget_status` reads current-window spend
//! from the durable `turns` store, projects a dry-out date from the burn rate,
//! and hands the cockpit a single render-ready struct. Config persists to
//! `~/.rift/usage-budget.json` (atomic temp+rename), kept out of the assistant
//! config so the usage domain stays self-contained.

use super::aggregate::now_ms;
use super::UsageDb;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted budget config. `plan` drives `limit` unless it's `"custom"`, in
/// which case `custom_limit_usd` is used. `cadence` is the reset window.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetConfig {
    /// "pro" | "max5x" | "max20x" | "custom". Default "max5x".
    #[serde(default = "default_plan")]
    pub plan: String,
    /// Used only when `plan == "custom"`.
    #[serde(default)]
    pub custom_limit_usd: Option<f64>,
    /// "monthly" | "weekly" | "daily". Default "monthly".
    #[serde(default = "default_cadence")]
    pub cadence: String,
}

fn default_plan() -> String {
    "max5x".into()
}
fn default_cadence() -> String {
    "monthly".into()
}

impl Default for BudgetConfig {
    fn default() -> Self {
        BudgetConfig {
            plan: default_plan(),
            custom_limit_usd: None,
            cadence: default_cadence(),
        }
    }
}

impl BudgetConfig {
    /// The dollar pool the plan grants per reset window.
    fn limit(&self) -> f64 {
        match self.plan.as_str() {
            "pro" => 20.0,
            "max5x" => 100.0,
            "max20x" => 200.0,
            "custom" => self.custom_limit_usd.unwrap_or(0.0).max(0.0),
            _ => 100.0,
        }
    }
}

/// Render-ready fuel-gauge state for the cockpit.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStatus {
    pub plan: String,
    pub cadence: String,
    pub limit: f64,
    pub spent: f64,
    /// 0–100. Clamped; 0 when over budget.
    pub pct_remaining: f64,
    /// Current reset-window bounds, epoch ms.
    pub window_start: i64,
    pub window_end: i64,
    /// Average $/day so far this window (spend ÷ elapsed days).
    pub burn_per_day: f64,
    /// When spend is projected to hit `limit` at the current burn, epoch ms.
    /// None when burn is ~0 (never) or already over budget.
    pub projected_exhaustion_date: Option<i64>,
    /// Days until the projected dry-out (None mirrors the field above).
    pub days_remaining: Option<f64>,
}

fn config_path() -> Result<PathBuf, String> {
    let dir = crate::state::paths::rift_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("usage-budget.json"))
}

fn load_config() -> BudgetConfig {
    let Ok(p) = config_path() else {
        return BudgetConfig::default();
    };
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Current reset-window bounds in epoch ms, derived from the cadence. Uses
/// chrono local time so "start of month/week/day" align to the user's clock.
fn window_bounds(cadence: &str) -> (i64, i64) {
    use chrono::{Datelike, Duration, Local, TimeZone};
    let now = Local::now();
    let day_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|d| Local.from_local_datetime(&d).earliest().unwrap_or(now))
        .unwrap_or(now);
    let (start, end) = match cadence {
        "daily" => (day_start, day_start + Duration::days(1)),
        "weekly" => {
            // Monday-anchored week.
            let dow = now.weekday().num_days_from_monday() as i64;
            let ws = day_start - Duration::days(dow);
            (ws, ws + Duration::days(7))
        }
        _ => {
            // Monthly: first of this month → first of next month.
            let first = now
                .date_naive()
                .with_day(1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|d| Local.from_local_datetime(&d).earliest().unwrap_or(now))
                .unwrap_or(now);
            let (ny, nm) = if now.month() == 12 {
                (now.year() + 1, 1)
            } else {
                (now.year(), now.month() + 1)
            };
            let next = chrono::NaiveDate::from_ymd_opt(ny, nm, 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|d| Local.from_local_datetime(&d).earliest().unwrap_or(now))
                .unwrap_or(now);
            (first, next)
        }
    };
    (start.timestamp_millis(), end.timestamp_millis())
}

/// Sum of trustworthy cost for turns at/after `since_ms`.
fn spend_since(db: &UsageDb, since_ms: i64) -> Result<f64, String> {
    let conn = db.0.lock().map_err(|e| format!("usage db lock: {e}"))?;
    conn.query_row(
        "SELECT COALESCE(SUM(COALESCE(cost_usd_calc, cost_usd_cli, 0)), 0)
         FROM turns WHERE ts >= ?1",
        [since_ms],
        |r| r.get(0),
    )
    .map_err(|e| format!("usage spend query: {e}"))
}

/// Read the persisted budget config.
#[tauri::command]
pub fn usage_get_budget() -> Result<BudgetConfig, String> {
    Ok(load_config())
}

/// Persist budget config (atomic temp+rename, matching the assistant config
/// pattern). Validates the enums so a bad value can't poison later reads.
#[tauri::command]
pub fn usage_set_budget(config: BudgetConfig) -> Result<(), String> {
    let plan = match config.plan.as_str() {
        "pro" | "max5x" | "max20x" | "custom" => config.plan.clone(),
        _ => return Err(format!("unknown plan: {}", config.plan)),
    };
    let cadence = match config.cadence.as_str() {
        "monthly" | "weekly" | "daily" => config.cadence.clone(),
        _ => return Err(format!("unknown cadence: {}", config.cadence)),
    };
    let clean = BudgetConfig {
        plan,
        cadence,
        custom_limit_usd: config.custom_limit_usd,
    };
    let p = config_path()?;
    let body = serde_json::to_vec_pretty(&clean).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, &body).map_err(|e| format!("write budget tmp: {e}"))?;
    std::fs::rename(&tmp, &p).map_err(|e| format!("rename budget: {e}"))
}

/// Fuel-gauge status: current-window spend, % remaining, and a burn-rate
/// dry-out projection. The killer post-June-15 view.
#[tauri::command]
pub fn usage_budget_status(db: tauri::State<UsageDb>) -> Result<BudgetStatus, String> {
    let cfg = load_config();
    let limit = cfg.limit();
    let (window_start, window_end) = window_bounds(&cfg.cadence);
    let spent = spend_since(&db, window_start)?;
    let now = now_ms();

    let pct_remaining = if limit > 0.0 {
        ((1.0 - spent / limit) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    let elapsed_days = ((now - window_start) as f64 / 86_400_000.0).max(1.0 / 24.0); // ≥1h floor
    let burn_per_day = spent / elapsed_days;

    let (projected_exhaustion_date, days_remaining) =
        if limit > 0.0 && spent < limit && burn_per_day > 1e-9 {
            let days = (limit - spent) / burn_per_day;
            let when = now + (days * 86_400_000.0) as i64;
            (Some(when), Some(days))
        } else {
            (None, None)
        };

    Ok(BudgetStatus {
        plan: cfg.plan,
        cadence: cfg.cadence,
        limit,
        spent,
        pct_remaining,
        window_start,
        window_end,
        burn_per_day,
        projected_exhaustion_date,
        days_remaining,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(plan: &str, custom: Option<f64>) -> BudgetConfig {
        BudgetConfig { plan: plan.into(), custom_limit_usd: custom, cadence: "monthly".into() }
    }

    #[test]
    fn plan_limits_map_to_caps() {
        assert_eq!(cfg("pro", None).limit(), 20.0);
        assert_eq!(cfg("max5x", None).limit(), 100.0);
        assert_eq!(cfg("max20x", None).limit(), 200.0);
        assert_eq!(cfg("custom", Some(42.0)).limit(), 42.0);
        // Unknown plan → safe default (max5x cap).
        assert_eq!(cfg("garbage", None).limit(), 100.0);
        // Custom with no/blank value → 0; negative is clamped up to 0.
        assert_eq!(cfg("custom", None).limit(), 0.0);
        assert_eq!(cfg("custom", Some(-5.0)).limit(), 0.0);
    }

    #[test]
    fn window_bounds_ordered_and_contain_now() {
        let now = chrono::Local::now().timestamp_millis();
        for cadence in ["daily", "weekly", "monthly"] {
            let (start, end) = window_bounds(cadence);
            assert!(start < end, "{cadence}: start !< end");
            assert!(start <= now && now < end, "{cadence}: now outside window");
        }
    }

    #[test]
    fn window_sizes_grow_with_cadence() {
        let len = |c| { let (s, e) = window_bounds(c); e - s };
        let (day, week, month) = (len("daily"), len("weekly"), len("monthly"));
        // Robust against DST (a local day may be 23-25h): just assert ordering.
        assert!(day < week, "daily {day} !< weekly {week}");
        assert!(week < month, "weekly {week} !< monthly {month}");
        // Sanity bands: daily ~24h, monthly ≥ 28 days.
        assert!((23..=25).contains(&(day / 3_600_000)), "daily hours = {}", day / 3_600_000);
        assert!(month / 86_400_000 >= 28, "monthly days = {}", month / 86_400_000);
    }

    #[test]
    fn unknown_cadence_falls_back_to_monthly() {
        // The `_` arm handles "monthly" and anything unrecognized identically.
        assert_eq!(window_bounds("garbage"), window_bounds("monthly"));
    }
}
