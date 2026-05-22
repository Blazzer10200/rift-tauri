//! Final-transcript polish via Claude Haiku 4.5 — fixes slur-to-word
//! substitutions, adds punctuation, preserves intent. Shells out to `claude
//! -p` (pipe mode) so we reuse the user's existing Claude Code auth — no API
//! key plumbing on Rift's side. Reference pattern: dev.to/auratech MumbleFlow
//! (Feb 2026) which uses the same `claude -p` shell-out for OCR cleanup.

use tokio::io::AsyncWriteExt;
use tokio::time::{timeout, Duration};

const CLEANUP_PROMPT: &str = "You are a dictation cleanup tool. Clean this \
transcribed speech from a Southern US English speaker. Fix obvious slur-to-word \
substitutions, add punctuation and proper capitalisation, normalise spacing, \
and preserve the speaker's intent, tone, and word choice. Do NOT rephrase, \
summarise, or add content. Do NOT add quotes or markdown. Output only the \
cleaned transcript text, nothing else.";

const HAIKU_MODEL: &str = "claude-haiku-4-5";
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(15);

/// Polish a raw Whisper transcript via Claude Haiku. Returns the cleaned text
/// on success; on any failure (CLI missing, timeout, non-zero exit) returns
/// the raw input unchanged so a transient cleanup outage never costs the user
/// their transcript.
pub async fn polish(raw: &str) -> Result<String, String> {
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
    cmd.arg("-p")
        .arg("--model")
        .arg(HAIKU_MODEL)
        .arg("--permission-mode")
        .arg("bypassPermissions")
        .arg("--append-system-prompt")
        .arg(CLEANUP_PROMPT)
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
        let _ = stdin.write_all(raw_owned.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    let output = match timeout(CLEANUP_TIMEOUT, child.wait_with_output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            log::warn!("[stt] cleanup wait failed: {e}");
            return Ok(raw.to_string());
        }
        Err(_) => {
            log::warn!("[stt] cleanup timed out after {}s", CLEANUP_TIMEOUT.as_secs());
            return Ok(raw.to_string());
        }
    };

    if !output.status.success() {
        log::warn!(
            "[stt] cleanup exited non-zero: status={:?} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return Ok(raw.to_string());
    }

    let cleaned = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if cleaned.is_empty() {
        Ok(raw.to_string())
    } else {
        Ok(cleaned)
    }
}
