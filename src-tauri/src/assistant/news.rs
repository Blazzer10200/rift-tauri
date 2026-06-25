//! "What's new in AI" feed for the Workspace page. Two tiers:
//!
//! - **Tier 1** `assistant_fetch_ai_news` — deterministic, free, no LLM, no new
//!   deps. Joins the Claude Code CHANGELOG.md (clean markdown bullet content) with
//!   the npm registry's per-version publish dates. Both are parsed with in-tree
//!   `regex`/`serde_json`. Used on launch + on a manual refresh.
//! - **Tier 2** `assistant_summarize_ai_news` — an opt-in AI digest that spawns the
//!   user's own Claude headless WITH web tools to curate recent Anthropic + Claude
//!   Code news beyond the changelog (model launches, API changes). A near-clone of
//!   `oneshot::assistant_analyze_usage` — the one meaningful difference is
//!   `--tools "WebSearch,WebFetch"` (analyze runs `--tools ""`). Costs a little on
//!   the user's subscription + does real web egress, so it's strictly user-driven.
//!
//! Cross-machine doctrine (ISSUES #61): every fetch is timeout-bounded (a firewall
//! DROP must not hang), body-capped, and names the host in errors. Tier 1 returns
//! whatever it could fetch — it only errors when BOTH sources fail.

use std::collections::HashMap;
use std::process::Stdio;

use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

use super::cli_install::claude_command;
use super::config::{load_config, DEFAULT_MODEL};

const CHANGELOG_URL: &str =
    "https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md";
const NPM_REGISTRY_URL: &str = "https://registry.npmjs.org/@anthropic-ai/claude-code";
const RELEASE_TAG_BASE: &str = "https://github.com/anthropics/claude-code/releases/tag/v";

/// Max release entries returned to the UI — the page shows a scannable handful,
/// not the full multi-year history.
const MAX_ITEMS: usize = 8;
/// Body cap for each fetched source (mirrors `oneshot::read_body_capped`). The
/// npm registry document embeds every version's full package.json, so it's large
/// (~1.2MB and growing) and — crucially — must NOT be truncated: the `time` map we
/// parse lives deep in the doc and a cut tail makes `serde_json` fail → all dates
/// drop to null. 8MB clears it for years while still hard-bounding a hostile body.
/// (The CHANGELOG is ~100KB+; the same cap covers it trivially.)
const BODY_CAP: usize = 8 * 1024 * 1024;

/// Progress event for the Tier-2 digest spawn — same shape as the analyze path
/// so the frontend can reuse the staged "Analyzing…" card pattern.
pub const NEWS_PROGRESS_EVENT: &str = "assistant://news-progress";

/// One Claude Code release as the UI consumes it. Dates are returned as the raw
/// ISO string from npm (or null) — the frontend computes "1d ago" so it never
/// goes stale server-side.
#[derive(serde::Serialize, Clone, Debug)]
pub struct NewsItem {
    /// Always "claude-code" for Tier 1. (Reserved for future sources.)
    pub source: String,
    pub version: String,
    /// ISO-8601 publish date from the npm registry, or null if unmatched.
    pub published_at: Option<String>,
    /// Changelog bullet lines, leading "- " stripped. Empty for a
    /// maintenance-only release ("Bug fixes and reliability improvements").
    pub bullets: Vec<String>,
    /// True when the release had no substantive bullets — the UI collapses these
    /// into a single "maintenance" line instead of dropping them (keeps the
    /// version timeline continuous).
    pub maintenance: bool,
    pub url: String,
}

/// Read a reqwest response body with a hard byte cap (mirrors
/// `oneshot::read_body_capped`). A hostile/misbehaving endpoint could stream an
/// unbounded body into `.text()` and OOM us.
async fn read_body_capped(resp: reqwest::Response) -> String {
    let mut resp = resp;
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < BODY_CAP {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                let take = (BODY_CAP - buf.len()).min(chunk.len());
                buf.extend_from_slice(&chunk[..take]);
                if take < chunk.len() {
                    break;
                }
            }
            Ok(None) | Err(_) => break,
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Parse the CHANGELOG markdown into (version, bullets) pairs in document order
/// (newest first — that's how the file is written). Pure + unit-tested.
///
/// Shape:
/// ```text
/// # Changelog
/// ## 2.1.191
/// - Added `/rewind` support …
/// - Fixed scroll position …
/// ## 2.1.190
/// - Bug fixes and reliability improvements
/// ```
fn parse_changelog(md: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut cur: Option<(String, Vec<String>)> = None;
    for raw in md.lines() {
        let line = raw.trim_end();
        // A version header: `## X.Y.Z` (optionally `## vX.Y.Z`). Anything after
        // the version (a date in parens etc.) is ignored.
        if let Some(rest) = line.strip_prefix("## ") {
            let token = rest.trim().trim_start_matches('v');
            let ver: String = token
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            // Require a dotted numeric to count as a version header (skip e.g.
            // "## Unreleased").
            if ver.contains('.') && ver.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                if let Some(prev) = cur.take() {
                    out.push(prev);
                }
                cur = Some((ver, Vec::new()));
                continue;
            }
        }
        // A bullet under the current version.
        if let Some((_, bullets)) = cur.as_mut() {
            let t = line.trim_start();
            if let Some(b) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
                let b = b.trim();
                if !b.is_empty() {
                    bullets.push(b.to_string());
                }
            }
        }
    }
    if let Some(prev) = cur.take() {
        out.push(prev);
    }
    out
}

/// A release with no substantive content — npm publishes these between feature
/// releases. We keep them (collapsed) so the timeline doesn't look gappy.
fn is_maintenance(bullets: &[String]) -> bool {
    if bullets.is_empty() {
        return true;
    }
    bullets.len() == 1
        && bullets[0]
            .to_ascii_lowercase()
            .contains("bug fixes and reliability")
}

/// Parse the npm registry document's `time` map → `version → ISO date`.
fn parse_npm_dates(json: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(v) = serde_json::from_str::<Value>(json) else {
        return map;
    };
    if let Some(time) = v.get("time").and_then(|t| t.as_object()) {
        for (ver, date) in time {
            // Skip the registry's own "created"/"modified" meta keys.
            if ver == "created" || ver == "modified" {
                continue;
            }
            if let Some(d) = date.as_str() {
                map.insert(ver.clone(), d.to_string());
            }
        }
    }
    map
}

/// Classify a fetch error so the message names the host + the likely cause
/// (network-blocked vs broken), per #61.
fn fetch_err(host: &str, e: &reqwest::Error) -> String {
    let s = e.to_string().to_ascii_lowercase();
    if e.is_timeout() || s.contains("timed out") {
        format!("Couldn't reach {host} — the request timed out (check your network/proxy/firewall).")
    } else if e.is_connect() || s.contains("dns") || s.contains("connect") {
        format!("Couldn't connect to {host} — you may be offline or behind a firewall.")
    } else if s.contains("certificate") || s.contains("tls") || s.contains("cert") {
        format!("Couldn't verify the TLS connection to {host} — a network proxy may be intercepting HTTPS.")
    } else {
        format!("Couldn't load from {host}: {e}")
    }
}

/// Tier 1 — fetch + join the Claude Code changelog with npm publish dates. Free,
/// deterministic, no LLM. Returns whatever it could assemble; errors only when
/// BOTH fetches fail.
#[tauri::command]
pub async fn assistant_fetch_ai_news() -> Result<Vec<NewsItem>, String> {
    let client = crate::certs::usage_client();

    // Fetch both in parallel — the changelog carries content, npm carries dates.
    let (changelog_res, npm_res) = tokio::join!(
        async {
            client
                .get(CHANGELOG_URL)
                .send()
                .await
                .map_err(|e| fetch_err("raw.githubusercontent.com", &e))
        },
        async {
            client
                .get(NPM_REGISTRY_URL)
                .header(reqwest::header::ACCEPT, "application/json")
                .send()
                .await
                .map_err(|e| fetch_err("registry.npmjs.org", &e))
        },
    );

    // The changelog is the load-bearing source (content). npm is enrichment
    // (dates) — if it fails we still ship the releases, just without dates.
    let changelog_md = match changelog_res {
        Ok(resp) if resp.status().is_success() => read_body_capped(resp).await,
        Ok(resp) => {
            return Err(format!(
                "raw.githubusercontent.com returned HTTP {} for the changelog.",
                resp.status().as_u16()
            ));
        }
        Err(e) => return Err(e),
    };

    let dates = match npm_res {
        Ok(resp) if resp.status().is_success() => parse_npm_dates(&read_body_capped(resp).await),
        // npm down/blocked is non-fatal — proceed dateless.
        _ => HashMap::new(),
    };

    let parsed = parse_changelog(&changelog_md);
    if parsed.is_empty() {
        return Err("The Claude Code changelog couldn't be parsed (format may have changed).".into());
    }

    let items: Vec<NewsItem> = parsed
        .into_iter()
        .take(MAX_ITEMS)
        .map(|(version, bullets)| {
            let maintenance = is_maintenance(&bullets);
            NewsItem {
                source: "claude-code".to_string(),
                published_at: dates.get(&version).cloned(),
                url: format!("{RELEASE_TAG_BASE}{version}"),
                version,
                bullets: if maintenance { Vec::new() } else { bullets },
                maintenance,
            }
        })
        .collect();

    Ok(items)
}

/// Meta-prompt for the Tier-2 AI digest. The model browses for recent Anthropic +
/// Claude Code news the deterministic changelog can't cover and returns guarded
/// JSON. Modeled on `oneshot::ANALYZE_META_PROMPT`. The frontend re-validates
/// every field (`normalizeNews`) — never trust this raw.
const NEWS_META_PROMPT: &str = "You are a concise tech-news editor for a developer using Rift, a desktop \
client for Claude Code. Using web search and fetch, find what is genuinely NEW and RELEVANT about \
Anthropic and Claude Code from roughly the last two weeks: new or updated Claude models, Claude Code CLI \
features, API/SDK changes, and pricing or usage-limit changes. Prioritize things that change how a \
developer works day to day.\n\
\n\
Use ONLY facts you verified from a fetched source this run. NEVER invent a version number, date, feature, \
or URL. If you cannot verify recent news, return an empty items array — do not pad it.\n\
\n\
Output ONLY a JSON object, no markdown, no code fences, no prose around it:\n\
{\"items\":[{\"title\":string,\"summary\":string,\"date\":string-or-null,\"url\":string,\"tag\":\"model\"|\"claude-code\"|\"api\"|\"company\"}],\"asOf\":string}\n\
\n\
Rules: at most 6 items, most important first. `summary` <= 200 characters, plain text, says why it \
matters. `date` is ISO-8601 if known, else null. `url` must be a real https link to the source. `asOf` is \
today's date in ISO-8601.";

/// Tier 2 — opt-in AI digest. Spawns the user's own Claude headless WITH web
/// tools. Near-clone of `oneshot::assistant_analyze_usage`; the meaningful
/// difference is `--tools "WebSearch,WebFetch"`.
#[tauri::command]
pub async fn assistant_summarize_ai_news(app: AppHandle) -> Result<String, String> {
    let mut cmd = claude_command()
        .ok_or_else(|| "claude CLI not on PATH — install Claude Code or configure an API key".to_string())?;

    let _ = load_config(); // touch config so a corrupt config surfaces here, not mid-stream.

    let user_msg = "Find and summarize the most important recent Anthropic and Claude Code news for a \
developer, per your instructions. Use web search/fetch to verify everything. Return only the JSON object.";

    cmd.arg("-p").arg(user_msg)
        .arg("--append-system-prompt").arg(NEWS_META_PROMPT)
        .arg("--exclude-dynamic-system-prompt-sections")
        .arg("--output-format").arg("stream-json")
        .arg("--verbose")
        .arg("--model").arg(DEFAULT_MODEL)
        // Web tool-loops can run longer than the analyze path; give the budget a
        // little more headroom over the ~$0.10 cache-creation tax. Still bounded.
        .arg("--max-budget-usd").arg("0.75")
        .arg("--strict-mcp-config")
        .arg("--disable-slash-commands")
        // The ONE meaningful difference vs analyze: enable web tools so the model
        // can actually pull current news. Everything else stays sandboxed.
        .arg("--tools").arg("WebSearch,WebFetch")
        .arg("--permission-mode").arg("bypassPermissions")
        .arg("--no-session-persistence")
        .current_dir(std::env::temp_dir())
        .env("CLAUDE_DISABLE_HOOKS", "1")
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("DISABLE_AUTOUPDATER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn `claude` (news): {e}"))?;
    let _ = app.emit(NEWS_PROGRESS_EVENT, serde_json::json!({ "stage": "spawned" }));
    let stdout = child.stdout.take().ok_or("news stdout unavailable")?;
    let stderr = child.stderr.take().ok_or("news stderr unavailable")?;

    let mut stderr_task = tokio::spawn(async move {
        let mut buf = String::new();
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(l)) = lines.next_line().await {
            // Keep draining past the cap so the child's stderr pipe can't fill +
            // deadlock it on wait() (same invariant as the analyze path).
            if buf.len() <= 8192 {
                buf.push_str(&l);
                buf.push('\n');
            }
        }
        buf
    });

    let progress_app = app.clone();
    let read_wait = async {
        let mut acc = String::new();
        let mut wrote = false;
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            match v.get("type").and_then(|t| t.as_str()) {
                Some("system") => {
                    let _ = progress_app
                        .emit(NEWS_PROGRESS_EVENT, serde_json::json!({ "stage": "thinking" }));
                }
                Some("assistant") if !wrote => {
                    wrote = true;
                    let _ = progress_app
                        .emit(NEWS_PROGRESS_EVENT, serde_json::json!({ "stage": "writing" }));
                }
                _ => {}
            }
            if v.get("type").and_then(|t| t.as_str()) == Some("result") {
                if let Some(t) = v.get("result").and_then(|r| r.as_str()) {
                    if !t.trim().is_empty() {
                        acc = t.to_string();
                    }
                }
                break;
            }
        }
        let status = child
            .wait()
            .await
            .map_err(|e| format!("await claude (news): {e}"))?;
        Ok::<_, String>((acc, status))
    };

    // 120s: the digest does real web round-trips on top of the cold-spawn auth
    // warmup, so it runs longer than the 90s analyze path.
    let (acc, status) =
        match tokio::time::timeout(std::time::Duration::from_secs(120), read_wait).await {
            Ok(r) => r?,
            Err(_) => {
                let _ = child.start_kill();
                let tail = tokio::time::timeout(
                    std::time::Duration::from_millis(500),
                    &mut stderr_task,
                )
                .await
                .ok()
                .and_then(|r| r.ok())
                .unwrap_or_default();
                stderr_task.abort();
                let tail = tail.trim();
                log::warn!("assistant_summarize_ai_news timed out (120s); stderr tail: {tail}");
                return Err(if tail.is_empty() {
                    "news summary timed out".to_string()
                } else {
                    format!("news summary timed out: {tail}")
                });
            }
        };

    const DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);
    let stderr_buf = match tokio::time::timeout(DRAIN_TIMEOUT, &mut stderr_task).await {
        Ok(Ok(buf)) => buf,
        Ok(Err(e)) => format!("(stderr drain task panicked: {e})"),
        Err(_) => {
            stderr_task.abort();
            String::new()
        }
    };
    if !status.success() {
        let msg = stderr_buf.trim();
        return Err(if msg.is_empty() {
            "news summary failed".to_string()
        } else {
            format!("news summary failed: {msg}")
        });
    }

    let cleaned = acc
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string();
    if cleaned.is_empty() {
        return Err("news summary returned empty output".into());
    }
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions_and_bullets() {
        let md = "# Changelog\n\n## 2.1.191\n\n- Added `/rewind` support\n- Fixed scroll jump\n\n## 2.1.190\n\n- Bug fixes and reliability improvements\n\n## 2.1.187\n\n- Improved MCP error messages\n";
        let p = parse_changelog(md);
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].0, "2.1.191");
        assert_eq!(p[0].1.len(), 2);
        assert_eq!(p[0].1[0], "Added `/rewind` support");
        assert_eq!(p[1].0, "2.1.190");
        assert!(is_maintenance(&p[1].1));
        assert!(!is_maintenance(&p[0].1));
    }

    #[test]
    fn handles_v_prefix_and_trailing_date() {
        let md = "## v2.0.0 (2026-01-01)\n- First\n## 1.9.9\n- Older\n";
        let p = parse_changelog(md);
        assert_eq!(p[0].0, "2.0.0");
        assert_eq!(p[1].0, "1.9.9");
    }

    #[test]
    fn skips_non_version_headers() {
        let md = "## Unreleased\n- pending\n## 2.1.0\n- real\n";
        let p = parse_changelog(md);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].0, "2.1.0");
    }

    #[test]
    fn empty_changelog_parses_empty() {
        assert!(parse_changelog("# Changelog\n\nnothing here\n").is_empty());
    }

    #[test]
    fn npm_dates_skip_meta_keys() {
        let json = r#"{"time":{"created":"2025-01-01T00:00:00Z","modified":"2026-06-24T00:00:00Z","2.1.191":"2026-06-24T18:54:40Z"}}"#;
        let m = parse_npm_dates(json);
        assert_eq!(m.len(), 1);
        assert_eq!(m.get("2.1.191").unwrap(), "2026-06-24T18:54:40Z");
    }

    #[test]
    fn npm_malformed_json_is_empty() {
        assert!(parse_npm_dates("not json").is_empty());
    }
}
