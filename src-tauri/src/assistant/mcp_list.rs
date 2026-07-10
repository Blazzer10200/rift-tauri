//! `/mcp` harness view — shells out to `claude mcp list` (the CLI's own
//! config walk + live health check) so Rift reports the SAME servers a
//! terminal `/mcp` would: user scope (`~/.claude.json`), project `.mcp.json`,
//! whatever THIS user configured — not just what a past init frame happened
//! to mention. Session init frames (streaming.ts) stay the live per-chat
//! overlay; this is the config-level truth that works before the first turn.
//!
//! No `--json` flag exists on `mcp list` (checked v2.1.206), so the text
//! output is parsed. Format per server line:
//!   `<name>: <target>[ (HTTP|SSE)] - <glyph> <status text>`
//! Older CLIs may print bare `<name>: <target>` (no health check) — those
//! rows parse with status "unknown" rather than being dropped.

use serde::Serialize;
use super::cli_install::claude_command;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct McpListRow {
    pub name: String,
    /// URL for http/sse servers; the spawn command line for stdio ones.
    pub target: String,
    /// "HTTP" / "SSE" when the CLI printed a transport suffix; stdio has none.
    pub transport: Option<String>,
    /// Normalized: connected · needs-auth · needs-approval · failed · unknown.
    pub status: String,
    /// Raw status text after the glyph, e.g. "Pending approval (run `claude` to approve)".
    pub detail: Option<String>,
}

/// Status glyphs the CLI puts in front of the health text. Used to anchor the
/// `" - "` split so a stdio command line containing `" - "` can't be mistaken
/// for a status separator. MUST include the legacy-Windows fallbacks (`√` for
/// `✔`, `×` for `✖`): the CLI's glyph lib downgrades when it detects no
/// modern terminal env (no TERM/WT_SESSION), which is exactly how Rift's
/// GUI-spawned child runs — a terminal never sees those, live-caught 2026-07-10.
const STATUS_GLYPHS: [char; 11] = ['✔', '✓', '√', '✗', '✘', '✖', '×', '!', '⏸', '…', '○'];

fn normalize_status(raw: &str) -> &'static str {
    let t = raw.to_lowercase();
    // Order matters: "failed to connect" / "disconnected" both contain
    // "connect(ed)", so the bad states must match first.
    if t.contains("failed") || t.contains("disconnected") || t.contains("error") {
        "failed"
    } else if t.contains("needs auth") {
        "needs-auth"
    } else if t.contains("pending approval") {
        "needs-approval"
    } else if t.contains("connected") {
        "connected"
    } else {
        "unknown"
    }
}

/// Drop ANSI SGR/cursor sequences — a user's FORCE_COLOR can survive into the
/// child even though we set NO_COLOR (belt and braces, both cheap).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for e in chars.by_ref() {
                    if e.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn parse_line(line: &str) -> Option<McpListRow> {
    let line = line.trim();
    // Noise ("Checking MCP server health…", blanks, hints) has no ": ".
    let (name, rest) = line.split_once(": ")?;
    let name = name.trim();
    if name.is_empty() || rest.is_empty() {
        return None;
    }

    // Split target from status on the LAST " - " whose tail starts with a
    // status glyph (glyph-anchored: " - " inside a stdio command line loses).
    let mut target = rest;
    let mut raw_status: Option<&str> = None;
    let mut search_end = rest.len();
    while let Some(i) = rest[..search_end].rfind(" - ") {
        let tail = &rest[i + 3..];
        if tail.chars().next().is_some_and(|c| STATUS_GLYPHS.contains(&c)) {
            target = &rest[..i];
            raw_status = Some(tail.trim());
            break;
        }
        search_end = i;
    }

    // Trailing transport marker: "<url> (HTTP)" / "(SSE)". Anything else in
    // parens is part of the target (stdio args can contain parens).
    let target = target.trim();
    let (target, transport) = match target.rfind(" (") {
        Some(i) if target.ends_with(')') => {
            let t = &target[i + 2..target.len() - 1];
            if t.eq_ignore_ascii_case("http") || t.eq_ignore_ascii_case("sse") {
                (target[..i].to_string(), Some(t.to_string()))
            } else {
                (target.to_string(), None)
            }
        }
        _ => (target.to_string(), None),
    };

    let status = raw_status.map_or("unknown", normalize_status).to_string();
    let detail = raw_status.map(|s| {
        s.trim_start_matches(|c| STATUS_GLYPHS.contains(&c))
            .trim_start()
            .to_string()
    });

    Some(McpListRow { name: name.to_string(), target, transport, status, detail })
}

pub(crate) fn parse_mcp_list(out: &str) -> Vec<McpListRow> {
    strip_ansi(out).lines().filter_map(parse_line).collect()
}

/// Run `claude mcp list` and return the parsed rows. `root` = the open
/// workspace folder (project-scope `.mcp.json` servers and their per-project
/// approval state resolve exactly like the user's turns do); falls back to a
/// neutral cwd so a no-folder chat still answers from user scope.
#[tauri::command]
pub async fn list_mcp_servers(root: Option<String>) -> Result<Vec<McpListRow>, String> {
    let mut cmd = claude_command().ok_or_else(|| {
        "Claude CLI not found on this machine — install Claude Code, then retry /mcp.".to_string()
    })?;
    cmd.arg("mcp").arg("list");
    // Status read, not a turn: skip autoupdate/telemetry startup work, and
    // force plain output so the parser never sees color codes.
    cmd.env("DISABLE_AUTOUPDATER", "1")
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .env("NO_COLOR", "1");
    let cwd = root
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_dir())
        .or_else(|| {
            // Mirror turn.rs's fallback: never inherit Rift's install dir (a
            // live cwd handle on `current\` blocks Velopack's update apply).
            std::env::var_os("LOCALAPPDATA")
                .map(std::path::PathBuf::from)
                .filter(|p| p.is_dir())
        })
        .unwrap_or_else(std::env::temp_dir);
    cmd.current_dir(cwd);
    cmd.stdin(std::process::Stdio::null());
    let out = tokio::time::timeout(std::time::Duration::from_secs(30), cmd.output())
        .await
        .map_err(|_| "`claude mcp list` timed out after 30s — a server health check hung.".to_string())?
        .map_err(|e| format!("run `claude mcp list`: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.trim();
        return Err(if err.is_empty() {
            format!("`claude mcp list` exited with {}", out.status)
        } else {
            format!("`claude mcp list` failed: {err}")
        });
    }
    Ok(parse_mcp_list(&String::from_utf8_lossy(&out.stdout)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verbatim capture from CLI v2.1.206 on 2026-07-10 (names/hosts real).
    const REAL_OUTPUT: &str = "Checking MCP server health…\n\n\
claude.ai Gmail: https://gmailmcp.googleapis.com/mcp/v1 - ✔ Connected\n\
claude.ai Google Calendar: https://calendarmcp.googleapis.com/mcp/v1 - ! Needs authentication\n\
claude.ai Stripe: https://mcp.stripe.com - ! Needs authentication\n\
blazzer-search: http://192.168.1.172:8080/mcp (HTTP) - ⏸ Pending approval (run `claude` to approve)\n";

    #[test]
    fn parses_real_cli_output() {
        let rows = parse_mcp_list(REAL_OUTPUT);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].name, "claude.ai Gmail");
        assert_eq!(rows[0].target, "https://gmailmcp.googleapis.com/mcp/v1");
        assert_eq!(rows[0].transport, None);
        assert_eq!(rows[0].status, "connected");
        assert_eq!(rows[0].detail.as_deref(), Some("Connected"));
        assert_eq!(rows[1].status, "needs-auth");
        let bz = &rows[3];
        assert_eq!(bz.name, "blazzer-search");
        assert_eq!(bz.target, "http://192.168.1.172:8080/mcp");
        assert_eq!(bz.transport.as_deref(), Some("HTTP"));
        assert_eq!(bz.status, "needs-approval");
        assert_eq!(bz.detail.as_deref(), Some("Pending approval (run `claude` to approve)"));
    }

    #[test]
    fn legacy_windows_fallback_glyphs_parse() {
        // GUI-spawned child (no TERM/WT_SESSION) → the CLI's glyph lib
        // downgrades ✔→√ and ✖→×. Regression: Gmail read "unknown" in-app
        // while a terminal read "connected" (2026-07-10).
        let rows = parse_mcp_list(
            "claude.ai Gmail: https://gmailmcp.googleapis.com/mcp/v1 - √ Connected\n\
             dead: http://x - × Failed to connect\n",
        );
        assert_eq!(rows[0].status, "connected");
        assert_eq!(rows[0].detail.as_deref(), Some("Connected"));
        assert_eq!(rows[1].status, "failed");
    }

    #[test]
    fn failed_and_disconnected_normalize_to_failed() {
        let rows = parse_mcp_list(
            "a: npx -y foo - ✘ Failed to connect\nb: http://x (SSE) - ✗ Disconnected\n",
        );
        assert_eq!(rows[0].status, "failed");
        assert_eq!(rows[1].status, "failed");
        assert_eq!(rows[1].transport.as_deref(), Some("SSE"));
    }

    #[test]
    fn old_format_line_without_health_check_is_kept_as_unknown() {
        let rows = parse_mcp_list("legacy: node server.js --port 3000\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].target, "node server.js --port 3000");
        assert_eq!(rows[0].status, "unknown");
        assert_eq!(rows[0].detail, None);
    }

    #[test]
    fn glyph_anchor_survives_dash_separator_inside_stdio_target() {
        let rows = parse_mcp_list(r"weird: node C:\my - tools\srv.js - ✓ Connected");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].target, r"node C:\my - tools\srv.js");
        assert_eq!(rows[0].status, "connected");
    }

    #[test]
    fn non_transport_parens_stay_in_target() {
        let rows = parse_mcp_list("s: cmd /c run.bat (fast) - ✓ Connected");
        assert_eq!(rows[0].target, "cmd /c run.bat (fast)");
        assert_eq!(rows[0].transport, None);
    }

    #[test]
    fn noise_lines_and_empty_output_yield_no_rows() {
        assert!(parse_mcp_list("").is_empty());
        assert!(parse_mcp_list("Checking MCP server health…\n\n").is_empty());
        assert!(parse_mcp_list("No MCP servers configured. Run `claude mcp add`.\n").is_empty());
    }

    #[test]
    fn ansi_codes_are_stripped() {
        let rows = parse_mcp_list("\u{1b}[32ma\u{1b}[0m: http://x - \u{1b}[32m✓\u{1b}[0m Connected\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "a");
        assert_eq!(rows[0].status, "connected");
    }
}
