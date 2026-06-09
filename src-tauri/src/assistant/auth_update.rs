//! R4 (per `docs/design/assistant-mod-split.md`) — auth probe, in-app sign-in
//! spawn, and the multi-install CLI updater. Lifted verbatim from
//! `assistant/mod.rs` 2026-06-09. The `AuthStatus` DTO + config/key access
//! stay on the parent; CLI discovery comes from the sibling `cli_install`.

use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use super::cli_install::{
    claude_command, enumerate_claude_installs, resolve_claude_exe, select_active_index,
    ClaudeInstall, CLAUDE_EXE,
};
use super::{current_api_key, load_config, AuthStatus};

/// CLI auth-status JSON shape from `claude auth status`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliAuthStatus {
    logged_in: Option<bool>,
    auth_method: Option<String>,
    api_provider: Option<String>,
    email: Option<String>,
    subscription_type: Option<String>,
}

#[tauri::command]
pub async fn assistant_auth_probe() -> Result<AuthStatus, String> {
    let mut out = AuthStatus::default();
    let _cfg = load_config(); // run keychain migration / surface any stale legacy field
    out.api_key_configured = current_api_key().is_some();
    // A stray system `ANTHROPIC_API_KEY` is the classic silent-401 trap: it
    // bypasses Rift's keychain/login model entirely and was inherited by the
    // spawned CLI while staying invisible to the probe. `claude_command()` now
    // strips it from every spawn (so `claude auth status` below reports the
    // real OAuth/login state); we still detect it here purely to warn the user.
    out.env_api_key_present = std::env::var("ANTHROPIC_API_KEY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    // #134 + multi-install: enumerate EVERY Claude CLI on the box (npm, native,
    // …) and run `auth status` on the newest one, in parallel. Enumeration runs
    // `--version` on each candidate, so it's offloaded to a blocking task; auth
    // status runs concurrently on the active exe (`claude_command()` resolves
    // the same newest pick). The prior sequential layout opened a small TOCTOU
    // window where the CLI could be swapped between the two resolutions.
    let installs_fut = tokio::task::spawn_blocking(enumerate_claude_installs);
    let auth_fut = async {
        match claude_command() {
            Some(mut c) => c.args(["auth", "status"]).stdout(Stdio::piped()).stderr(Stdio::null()).output().await.ok(),
            None => None,
        }
    };
    let (installs_res, auth_opt) = tokio::join!(installs_fut, auth_fut);
    let mut installs = installs_res.unwrap_or_default();
    let active = select_active_index(&installs);
    if let Some(i) = active {
        installs[i].active = true;
    }
    log::info!(
        "auth-probe: {} claude install(s){}",
        installs.len(),
        active
            .map(|i| format!(
                ", active = {} ({}, v{})",
                installs[i].path,
                installs[i].method,
                installs[i].version.as_deref().unwrap_or("?")
            ))
            .unwrap_or_default()
    );

    match active.and_then(|i| installs[i].version.clone()) {
        Some(v) => {
            out.cli_present = true;
            out.cli_version = Some(v);
            out.install_method = active.map(|i| installs[i].method.clone());
            out.installs = installs;
        }
        None => {
            out.installs = installs;
            out.cli_present = false;
            out.pill = if out.api_key_configured { "yellow".into() } else { "red".into() };
            out.summary = if out.api_key_configured {
                "API key configured — install Claude Code CLI for piggyback session".into()
            } else {
                "Not configured — install Claude Code CLI or add an API key in Settings".into()
            };
            return Ok(out);
        }
    }

    // `claude auth status` — JSON when stdout isn't a TTY (which it isn't from spawn).
    if let Some(auth) = auth_opt {
        if auth.status.success() {
            let text = String::from_utf8_lossy(&auth.stdout);
            if let Ok(parsed) = serde_json::from_str::<CliAuthStatus>(text.trim()) {
                out.logged_in = parsed.logged_in.unwrap_or(false);
                out.auth_method = parsed.auth_method;
                out.api_provider = parsed.api_provider;
                out.email = parsed.email;
                out.subscription_type = parsed.subscription_type;
            }
        }
    }

    // Priority: explicit API key shadows the OAuth session (env-var precedence).
    if out.api_key_configured {
        out.pill = "yellow".into();
        out.summary = "Using API key".into();
    } else if out.logged_in {
        out.pill = "green".into();
        let who = out.email.as_deref().unwrap_or("Claude account");
        let sub = out.subscription_type.as_deref().unwrap_or("").trim();
        // Positively distinguish a subscription session (claude.ai OAuth, Pro/
        // Max) from a logged-in Console/API account — the latter still reports
        // loggedIn:true but bills per-token against no plan. `authMethod` ==
        // "claude.ai" is the subscription signal; `apiProvider` == "firstParty"
        // rules out Bedrock/Vertex third-party routing. A Console login reports
        // a different authMethod and no subscriptionType, so it must NOT read as
        // a subscription. (Real shapes: subscription → authMethod "claude.ai",
        // apiProvider "firstParty", subscriptionType "max"/"pro".)
        let claude_ai = out
            .auth_method
            .as_deref()
            .map(|m| m.eq_ignore_ascii_case("claude.ai"))
            .unwrap_or(false);
        let first_party = out
            .api_provider
            .as_deref()
            .map(|p| p.eq_ignore_ascii_case("firstParty"))
            .unwrap_or(false);
        let is_subscription = claude_ai && !sub.is_empty();
        out.summary = if is_subscription {
            let plan: &str = match sub.to_ascii_lowercase().as_str() {
                "max" => "Max",
                "pro" => "Pro",
                "team" => "Team",
                "enterprise" => "Enterprise",
                _ => sub, // unknown tier — surface the raw label rather than drop it
            };
            format!("Claude {plan} subscription · {who}")
        } else if claude_ai || first_party {
            // claude.ai login but no tier string yet — still a subscription
            // session, just without a reported plan. Don't mislabel it API.
            format!("Claude subscription · {who}")
        } else {
            // Logged in via a Console/API account → per-token billing, no plan.
            format!("Claude API account · {who} (per-token billing)")
        };
        // Login works and is what we'll use — but flag the ignored env key so a
        // user who *thinks* they're on a key isn't surprised by which identity
        // (and bill) turns actually run under.
        if out.env_api_key_present {
            out.summary
                .push_str(" · ignoring a system ANTHROPIC_API_KEY (Rift uses your login)");
        }
    } else if out.env_api_key_present {
        // No login, no Rift key, but a system env key exists. Before the strip
        // it silently authed the CLI; now Rift ignores it on purpose. Point the
        // user at the supported path rather than leaving them with a bare "401".
        out.pill = "red".into();
        out.summary = "A system ANTHROPIC_API_KEY is set but Rift ignores env keys — paste it into the API-key field below (stored in the keychain) or run `claude login`.".into();
    } else {
        out.pill = "red".into();
        out.summary = "Claude CLI found but not logged in — run `claude login` or add an API key".into();
    }
    Ok(out)
}

/// Open the Claude CLI's interactive sign-in in its OWN console window so a user
/// who just hit a 401 can re-authenticate without leaving Rift. Spawns
/// `<active claude> auth login` — subscription by default, `--console` for an
/// Anthropic API (per-token) account — which opens the browser OAuth flow and
/// prints progress in that console. Returns as soon as it's spawned; the
/// frontend then polls `assistant_auth_probe` until the session flips. The new
/// credentials land in the CLI's own store — the same one the probe reads and
/// every turn spawns against — so a successful login clears the failure for
/// real, not just in the UI. This is the in-app replacement for "go open a
/// terminal and run `claude login`".
#[tauri::command]
pub fn assistant_open_login(console: bool) -> Result<(), String> {
    let exe = resolve_claude_exe().ok_or(
        "No Claude CLI found on this machine. Install Claude Code first, or add an API key in Settings → CLI session.",
    )?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("auth").arg("login");
    cmd.arg(if console { "--console" } else { "--claudeai" });
    // Real OAuth flow — strip any inherited system key so login can't be
    // shadowed into a different identity than the one turns run under (the same
    // trap `claude_command()` guards against).
    cmd.env_remove("ANTHROPIC_API_KEY");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Give the child its OWN console: the OAuth flow prints a URL + status
        // the user may need to see. CREATE_NO_WINDOW (used for headless turns)
        // would hide it with no way to follow along.
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        cmd.creation_flags(CREATE_NEW_CONSOLE);
    }
    // Detached on purpose: drop the Child handle. std's `Child::drop` does NOT
    // kill the process (unlike tokio's kill_on_drop), so the sign-in keeps
    // running in its console while Rift polls auth status.
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to start sign-in: {e}"))
}

/// Result of an in-app Claude CLI update attempt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliUpdateResult {
    /// The active install's method ("npm" | "native" | "unknown") — back-compat.
    pub method: String,
    /// Human-readable per-install summary (one line each).
    pub output: String,
    /// Freshly re-enumerated installs after the update, with new versions.
    pub installs: Vec<ClaudeInstall>,
}

/// Update EVERY Claude Code CLI on this machine, each by the command that
/// matches how it was installed:
///   * npm     → `npm install -g @anthropic-ai/claude-code@latest` (once)
///   * native  → `<exe> update` against that exact binary
///   * unknown → best-effort `<exe> update`
///
/// Updating all of them (not just the one Rift happens to spawn) is the whole
/// point: on a box with BOTH an npm and a native install, the version Rift
/// reads and the version the user's shell reads can drift apart, so updating a
/// single copy leaves the banner stuck "out of date". Buffered (no streaming) —
/// npm -g can take ~30-60s; the frontend shows a spinner and re-probes on
/// return. Returns a per-install summary + the post-update enumeration. Errors
/// only if EVERY install's update failed (partial failures surface in `output`).
#[tauri::command]
pub async fn assistant_update_cli() -> Result<CliUpdateResult, String> {
    let installs = tokio::task::spawn_blocking(enumerate_claude_installs)
        .await
        .map_err(|e| format!("install scan failed: {e}"))?;
    if installs.is_empty() {
        return Err("Claude CLI not found on PATH.".into());
    }
    let active_method = select_active_index(&installs)
        .map(|i| installs[i].method.clone())
        .unwrap_or_else(|| "unknown".into());
    log::info!("cli-update: updating {} install(s)", installs.len());

    let mut reports: Vec<String> = Vec::new();
    let mut any_ok = false;
    let mut any_err = false;
    let mut npm_done = false;

    for inst in &installs {
        let (label, res) = if inst.method == "npm" {
            // npm-global is location-independent (npm resolves its own prefix),
            // so one run covers every npm copy.
            if npm_done {
                continue;
            }
            npm_done = true;
            ("npm".to_string(), run_npm_update().await)
        } else {
            // native / unknown → update THIS binary specifically.
            (
                format!("{} ({})", inst.method, inst.path),
                run_exe_update(&inst.path).await,
            )
        };
        match res {
            Ok(o) => {
                any_ok = true;
                let line = o.lines().last().map(str::trim).filter(|l| !l.is_empty()).unwrap_or("updated");
                reports.push(format!("{label}: {line}"));
                log::info!("cli-update: {label} OK");
            }
            Err(e) => {
                any_err = true;
                reports.push(format!("{label}: FAILED — {e}"));
                log::warn!("cli-update: {label} FAILED — {e}");
            }
        }
    }

    // Drop the cached active exe so the next resolve re-stats the relinked
    // binaries, then re-enumerate to report fresh versions back to the UI.
    if let Ok(mut g) = CLAUDE_EXE.lock() {
        *g = None;
    }
    let mut after = tokio::task::spawn_blocking(enumerate_claude_installs)
        .await
        .unwrap_or_default();
    if let Some(i) = select_active_index(&after) {
        after[i].active = true;
    }
    log::info!(
        "cli-update: post-update = {:?}",
        after.iter().map(|i| (i.method.as_str(), i.version.as_deref())).collect::<Vec<_>>()
    );

    let summary = if reports.is_empty() {
        "Update complete.".to_string()
    } else {
        reports.join("\n")
    };
    if any_err && !any_ok {
        Err(summary)
    } else {
        Ok(CliUpdateResult {
            method: active_method,
            output: summary,
            installs: after,
        })
    }
}

/// `npm install -g @anthropic-ai/claude-code@latest`. Static args (no user
/// input, no newlines) so the Rust 1.77 batch-arg validator + `cmd /C` are safe.
async fn run_npm_update() -> Result<String, String> {
    let mut cmd;
    #[cfg(windows)]
    {
        cmd = Command::new("cmd");
        cmd.args(["/C", "npm", "install", "-g", "@anthropic-ai/claude-code@latest"]);
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        cmd = Command::new("npm");
        cmd.args(["install", "-g", "@anthropic-ai/claude-code@latest"]);
    }
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Couldn't launch npm — is Node.js/npm installed and on PATH? ({e})"))?;
    finish_update(output)
}

/// `<exe> update` for a native/script install, run against that exact binary so
/// the right copy is bumped on a multi-install box. Hides the window + strips a
/// stray ANTHROPIC_API_KEY like every other spawn.
async fn run_exe_update(exe: &str) -> Result<String, String> {
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.env_remove("ANTHROPIC_API_KEY");
    let output = cmd
        .arg("update")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Couldn't run `claude update`: {e}"))?;
    finish_update(output)
}

/// Shared success/failure shaping for an updater's buffered output.
fn finish_update(output: std::process::Output) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        let tail = tail_lines(&stdout, 6);
        Ok(if tail.is_empty() { "Update complete.".into() } else { tail })
    } else {
        let msg = tail_lines(&stderr, 8);
        Err(if msg.is_empty() {
            format!("Update failed (exit {:?}).", output.status.code())
        } else {
            msg
        })
    }
}

/// Last `n` non-blank lines of `s`, trimmed — keeps updater output digestible.
fn tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().filter(|l| !l.trim().is_empty()).collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n").trim().to_string()
}
