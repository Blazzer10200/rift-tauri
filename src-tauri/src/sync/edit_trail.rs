// Per-watched-folder edit trail — `.rift-trail.jsonl` lives at the root of each
// watched folder on the remote. Append-only line-delimited JSON. AutoSync (Phase
// 1c) will write one batch per FlushBatchAsync success. Capped at MaxLines.
//
// Compat w/ WPF: same filename, same JSON shape per entry. Read/append uses the
// SftpClient surface from Phase 1b.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::sftp::SftpClient;

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
        let user = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "unknown".into());
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

    /// Read trail and return the latest entry per path.
    pub async fn read_latest_per_path(
        &self,
        watched_root: &str,
    ) -> Result<HashMap<String, Entry>, String> {
        let trail = Self::trail_path(watched_root);
        let raw = self.read_raw(&trail).await.unwrap_or_default();
        let mut out: HashMap<String, Entry> = HashMap::new();
        for line in raw.split('\n') {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(e) = serde_json::from_str::<Entry>(line) else {
                continue;
            };
            if e.path.is_empty() {
                continue;
            }
            match out.get(&e.path) {
                Some(prior) if prior.ts >= e.ts => {}
                _ => {
                    out.insert(e.path.clone(), e);
                }
            }
        }
        Ok(out)
    }

    async fn read_raw(&self, remote_path: &str) -> Option<String> {
        let sandbox = std::env::temp_dir().join(format!(
            "rift-trail-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let local = self.sftp.download_file(remote_path, &sandbox).await.ok()?;
        let text = std::fs::read_to_string(&local).ok();
        let _ = std::fs::remove_dir_all(&sandbox);
        text
    }
}

fn trim_to_tail(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    if lines.len() <= max_lines {
        return content.to_string();
    }
    lines[lines.len() - max_lines..].join("\n")
}

fn hostname() -> Option<String> {
    std::env::var("COMPUTERNAME").ok().or_else(|| {
        std::process::Command::new("hostname")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    })
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
