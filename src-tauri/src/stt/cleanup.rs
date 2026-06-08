//! Final-transcript polish via Claude Haiku 4.5 — fixes slur-to-word
//! substitutions, adds punctuation, preserves intent. Shells out to `claude
//! -p` (pipe mode) so we reuse the user's existing Claude Code auth — no API
//! key plumbing on Rift's side. Reference pattern: dev.to/auratech MumbleFlow
//! (Feb 2026) which uses the same `claude -p` shell-out for OCR cleanup.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};

const CLEANUP_PROMPT: &str = "You are a dictation cleanup tool. Clean this \
transcribed speech from a Southern US English speaker. Fix obvious slur-to-word \
substitutions, add punctuation and proper capitalisation, normalise spacing, \
and preserve the speaker's intent, tone, and word choice. Preserve profanity and \
swear words EXACTLY as spoken — never censor, mask, asterisk out, bleep, or \
soften them. Do NOT rephrase, summarise, or add content. Do NOT add quotes or \
markdown. Output only the cleaned transcript text, nothing else.";

const HAIKU_MODEL: &str = "claude-haiku-4-5";
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(15);

/// Polish a raw Whisper transcript via Claude Haiku. Returns the cleaned text
/// on success; on any failure (CLI missing, timeout, non-zero exit) returns
/// the raw input unchanged so a transient cleanup outage never costs the user
/// their transcript.
pub async fn polish(raw: &str) -> Result<String, String> {
    polish_with_ctx(raw, "").await
}

/// Like [`polish`], but injects a short workspace-context string (project,
/// branch, filenames) into the system prompt so Haiku keeps the speaker's
/// project terms verbatim instead of "correcting" symbols it doesn't know.
pub async fn polish_with_ctx(raw: &str, ctx: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }
    // Don't bother cleaning a single-word utterance — Haiku might over-edit it.
    if raw.split_whitespace().count() < 3 {
        return Ok(raw.to_string());
    }

    let mut cmd = match crate::assistant::claude_command() {
        Some(c) => c,
        None => {
            log::debug!("[stt] cleanup: claude CLI not found, returning raw transcript");
            return Ok(raw.to_string());
        }
    };
    // Pattern matches assistant::assistant_send — system instruction via
    // `--append-system-prompt`, user content (raw transcript) via stdin.
    // Note: `--bare` would cut ~500ms startup but skips OAuth + keychain
    // auth, which requires ANTHROPIC_API_KEY. Subscription Claude Code users
    // don't have one, so we eat the regular-startup cost.
    let system_prompt = build_system_prompt(ctx);
    cmd.arg("-p")
        .arg("--model")
        .arg(HAIKU_MODEL)
        // Pure text task: hand it an EMPTY tool allowlist so it can invoke
        // nothing. This is what makes the untrusted transcript (piped on stdin)
        // and the workspace-context system prompt safe — a prompt injection has
        // no tool to reach (F7/F79/F113). With no tools there's also nothing to
        // approve, so `--permission-mode bypassPermissions` is gone.
        .arg("--allowed-tools")
        .arg("")
        .arg("--append-system-prompt")
        .arg(&system_prompt)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("[stt] cleanup spawn failed: {e}");
            return Ok(raw.to_string());
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let raw_owned = raw.to_string();
        if let Err(e) = stdin.write_all(raw_owned.as_bytes()).await {
            log::warn!("[stt] cleanup stdin write failed: {e}");
        }
        let _ = stdin.shutdown().await;
    }

    // Read stdout/stderr concurrently while waiting, all under one timeout, so we
    // keep ownership of `child` and can kill it on timeout instead of leaking a
    // stalled subprocess (F16/F116 — `wait_with_output` consumed the child, so
    // the old code had no handle to kill).
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let drain = async {
        let read_out = async {
            let mut b = Vec::new();
            if let Some(s) = &mut stdout {
                let _ = s.read_to_end(&mut b).await;
            }
            b
        };
        let read_err = async {
            let mut b = Vec::new();
            if let Some(s) = &mut stderr {
                let _ = s.read_to_end(&mut b).await;
            }
            b
        };
        let (out, err) = tokio::join!(read_out, read_err);
        (child.wait().await, out, err)
    };

    let (status_res, out_buf, err_buf) = match timeout(CLEANUP_TIMEOUT, drain).await {
        Ok(t) => t,
        Err(_) => {
            log::warn!("[stt] cleanup timed out after {}s — killing subprocess", CLEANUP_TIMEOUT.as_secs());
            let _ = child.start_kill();
            return Ok(raw.to_string());
        }
    };

    let status = match status_res {
        Ok(s) => s,
        Err(e) => {
            log::warn!("[stt] cleanup wait failed: {e}");
            return Ok(raw.to_string());
        }
    };

    if !status.success() {
        log::warn!(
            "[stt] cleanup exited non-zero: status={:?} stderr={}",
            status.code(),
            String::from_utf8_lossy(&err_buf).trim()
        );
        return Ok(raw.to_string());
    }

    let cleaned = String::from_utf8_lossy(&out_buf).trim().to_string();
    if cleaned.is_empty() {
        Ok(raw.to_string())
    } else {
        Ok(cleaned)
    }
}

/// Append the workspace context (capped) to the cleanup instruction so Haiku
/// preserves project-specific terms. Empty context → the bare prompt.
fn build_system_prompt(ctx: &str) -> String {
    let ctx = ctx.trim();
    if ctx.is_empty() {
        return CLEANUP_PROMPT.to_string();
    }
    let capped: String = ctx.chars().take(300).collect();
    format!(
        "{CLEANUP_PROMPT} The speaker is working in this codebase; preserve \
         these project terms verbatim if they appear: {capped}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context_yields_bare_prompt() {
        assert_eq!(build_system_prompt(""), CLEANUP_PROMPT);
        assert_eq!(build_system_prompt("   \n  "), CLEANUP_PROMPT);
    }

    #[test]
    fn context_is_appended_and_capped_at_300() {
        let p = build_system_prompt("rift-tauri on branch main");
        assert!(p.starts_with(CLEANUP_PROMPT));
        assert!(p.contains("rift-tauri on branch main"));
        // Oversized context is truncated to 300 chars before injection.
        let long = "x".repeat(400);
        let capped = build_system_prompt(&long);
        assert!(capped.contains(&"x".repeat(300)));
        assert!(!capped.contains(&"x".repeat(301)), "context exceeded the 300-char cap");
    }
}
