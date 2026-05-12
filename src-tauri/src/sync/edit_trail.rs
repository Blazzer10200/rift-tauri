// Per-watched-folder edit trail — `.rift-trail.jsonl` lives at the root of each
// watched folder on the remote. Append-only line-delimited JSON. AutoSync (Phase
// 1c) will write one batch per FlushBatchAsync success. Capped at MaxLines.
//
// Compat w/ WPF: same filename, same JSON shape per entry. Read/append uses the
// SftpClient surface from Phase 1b.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::sftp::SftpClient;
use crate::transport::env::{current_user, hostname, short_id};

const MAX_LINES: usize = 500;
const TRAIL_FILE_NAME: &str = ".rift-trail.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Entry {
    pub path: String,
    pub user: String,
    pub host: String,
    pub ts: DateTime<Utc>,
    pub action: String,
}

pub struct EditTrail<'a> {
    sftp: &'a SftpClient,
    user: String,
    host: String,
}

impl<'a> EditTrail<'a> {
    pub fn new(sftp: &'a SftpClient) -> Self {
        let user = current_user();
        let host = hostname().unwrap_or_else(|| "unknown".into());
        Self { sftp, user, host }
    }

    fn trail_path(watched_root: &str) -> String {
        let trimmed = watched_root.trim_end_matches('/');
        format!("{trimmed}/{TRAIL_FILE_NAME}")
    }

    /// Best-effort append. Read existing → append → trim → re-upload. One round
    /// trip per batch (not per file).
    pub async fn append(
        &self,
        watched_root: &str,
        items: &[(String, String)], // (rel_path, action)
    ) -> Result<(), String> {
        if items.is_empty() || watched_root.is_empty() {
            return Ok(());
        }
        let trail = Self::trail_path(watched_root);
        let existing = self.read_raw(&trail).await.unwrap_or_default();
        let mut buf = existing;
        let now = Utc::now();
        for (rel, action) in items {
            let line = serde_json::to_string(&Entry {
                path: rel.clone(),
                user: self.user.clone(),
                host: self.host.clone(),
                ts: now,
                action: action.clone(),
            })
            .map_err(|e| format!("serialize: {e}"))?;
            buf.push_str(&line);
            buf.push('\n');
        }
        let trimmed = trim_to_tail(&buf, MAX_LINES);
        self.sftp.upload_bytes(trimmed.as_bytes(), &trail).await
    }

    async fn read_raw(&self, remote_path: &str) -> Option<String> {
        let sandbox = std::env::temp_dir().join(format!(
            "rift-trail-{}-{}",
            std::process::id(),
            short_id()
        ));
        let local = self.sftp.download_file(remote_path, &sandbox).await.ok()?;
        let text = std::fs::read_to_string(&local).ok();
        let _ = std::fs::remove_dir_all(&sandbox);
        text
    }
}

fn trim_to_tail(content: &str, max_lines: usize) -> String {
    // Preserve any trailing newline so the JSONL file stays well-formed across
    // append cycles (consumers split on '\n' and an absent terminator can
    // confuse line-based tooling).
    let had_trailing_newline = content.ends_with('\n');
    let body = content.trim_end_matches(['\r', '\n']);
    let lines: Vec<&str> = body.lines().collect();
    let mut out = if lines.len() <= max_lines {
        body.to_string()
    } else {
        lines[lines.len() - max_lines..].join("\n")
    };
    if had_trailing_newline && !out.is_empty() {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_to_tail_keeps_last_n() {
        // Content w/ no trailing newline (split on '\n' = exact line count).
        let content = "a\nb\nc\nd\ne";
        assert_eq!(trim_to_tail(content, 10), content);
        assert_eq!(trim_to_tail(content, 3), "c\nd\ne");
    }

    #[test]
    fn trail_path_handles_trailing_slash() {
        assert_eq!(
            EditTrail::trail_path("/opt/foo/"),
            "/opt/foo/.rift-trail.jsonl"
        );
        assert_eq!(
            EditTrail::trail_path("/opt/foo"),
            "/opt/foo/.rift-trail.jsonl"
        );
    }
}
