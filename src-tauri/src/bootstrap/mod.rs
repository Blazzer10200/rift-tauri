// Phase 1j — port of Rift Project/Services/Bootstrap/{BootstrapDetection,BootstrapDetector}.cs.
//
// Pure classifier (Classify) + async SFTP wrapper (detect). The classifier
// reads the local filesystem; does NOT touch SFTP — testable directly via
// real temp dirs. The async wrapper fetches the remote top-level dir list,
// filters [disabled], then hands off to the classifier.

use serde::{Deserialize, Serialize};

use crate::sftp::SftpClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BootstrapState {
    Synced,
    MissingLocalRoot,
    Empty,
    Uninitialized,
    Partial,
    BadRemoteRoot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapDetection {
    pub state: BootstrapState,
    pub remote_resource_count: u32,
    pub local_present_count: u32,
    pub missing_count: u32,
    pub remote_top_level_dirs: Vec<String>,
}

pub const SYNCED_THRESHOLD: f64 = 0.80;
pub const UNINITIALIZED_THRESHOLD: f64 = 0.10;
pub const BAD_REMOTE_ROOT_BRACKETED_RATIO: f64 = 0.50;
pub const BAD_REMOTE_ROOT_MIN_DIRS: usize = 3;
const DISABLED_MARKER: &str = "[disabled]";

pub fn classify(remote_dirs: &[String], local_root: &str) -> BootstrapDetection {
    let remote_count = remote_dirs.len();

    if remote_count >= BAD_REMOTE_ROOT_MIN_DIRS {
        let bracketed = remote_dirs
            .iter()
            .filter(|n| n.len() >= 3 && n.starts_with('[') && n.ends_with(']'))
            .count();
        let ratio = bracketed as f64 / remote_count as f64;
        if ratio < BAD_REMOTE_ROOT_BRACKETED_RATIO {
            return BootstrapDetection {
                state: BootstrapState::BadRemoteRoot,
                remote_resource_count: remote_count as u32,
                local_present_count: 0,
                missing_count: 0,
                remote_top_level_dirs: remote_dirs.to_vec(),
            };
        }
    }

    let local_path = std::path::Path::new(local_root);
    if local_root.trim().is_empty() || !local_path.is_dir() {
        return BootstrapDetection {
            state: BootstrapState::MissingLocalRoot,
            remote_resource_count: remote_count as u32,
            local_present_count: 0,
            missing_count: remote_count as u32,
            remote_top_level_dirs: remote_dirs.to_vec(),
        };
    }

    let mut local_dir_names: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut has_any_local = false;
    match std::fs::read_dir(local_path) {
        Ok(it) => {
            for entry in it.flatten() {
                has_any_local = true;
                let Ok(meta) = entry.metadata() else { continue };
                if meta.is_dir() {
                    local_dir_names
                        .insert(entry.file_name().to_string_lossy().to_ascii_lowercase());
                }
            }
        }
        Err(_) => {
            return BootstrapDetection {
                state: BootstrapState::MissingLocalRoot,
                remote_resource_count: remote_count as u32,
                local_present_count: 0,
                missing_count: remote_count as u32,
                remote_top_level_dirs: remote_dirs.to_vec(),
            };
        }
    }

    if !has_any_local {
        return BootstrapDetection {
            state: BootstrapState::Empty,
            remote_resource_count: remote_count as u32,
            local_present_count: 0,
            missing_count: remote_count as u32,
            remote_top_level_dirs: remote_dirs.to_vec(),
        };
    }

    let present = remote_dirs
        .iter()
        .filter(|d| local_dir_names.contains(&d.to_ascii_lowercase()))
        .count();
    let missing = remote_count.saturating_sub(present);
    let ratio = if remote_count == 0 { 1.0 } else { present as f64 / remote_count as f64 };

    let state = if ratio >= SYNCED_THRESHOLD {
        BootstrapState::Synced
    } else if ratio >= UNINITIALIZED_THRESHOLD {
        BootstrapState::Partial
    } else {
        BootstrapState::Uninitialized
    };

    BootstrapDetection {
        state,
        remote_resource_count: remote_count as u32,
        local_present_count: present as u32,
        missing_count: missing as u32,
        remote_top_level_dirs: remote_dirs.to_vec(),
    }
}

/// Async wrapper — fetches remote top-level dirs (skipping `[disabled]`) +
/// hands off to the pure classifier.
pub async fn detect(
    sftp: &SftpClient,
    remote_root: &str,
    local_root: &str,
) -> Result<BootstrapDetection, String> {
    let entries = sftp.list_directory(remote_root).await?;
    let remote_dirs: Vec<String> = entries
        .into_iter()
        .filter(|e| e.is_dir && !e.name.eq_ignore_ascii_case(DISABLED_MARKER))
        .map(|e| e.name)
        .collect();
    Ok(classify(&remote_dirs, local_root))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn bad_remote_root_when_few_brackets() {
        let r = classify(&dirs(&["server", "txData", "logs", "cache"]), "");
        assert_eq!(r.state, BootstrapState::BadRemoteRoot);
    }

    #[test]
    fn missing_local_when_path_absent() {
        let r = classify(
            &dirs(&["[qbx]", "[standalone]", "[voice]"]),
            "/no/such/path/__rift_tj",
        );
        assert_eq!(r.state, BootstrapState::MissingLocalRoot);
    }

    #[test]
    fn empty_when_local_has_no_content() {
        let tmp = std::env::temp_dir().join(format!("rift-bs-empty-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let r = classify(
            &dirs(&["[qbx]", "[standalone]", "[voice]"]),
            tmp.to_str().unwrap(),
        );
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(r.state, BootstrapState::Empty);
    }

    #[test]
    fn synced_when_above_threshold() {
        let tmp = std::env::temp_dir().join(format!("rift-bs-sync-{}", std::process::id()));
        let _ = std::fs::create_dir_all(tmp.join("[qbx]"));
        let _ = std::fs::create_dir_all(tmp.join("[standalone]"));
        let _ = std::fs::create_dir_all(tmp.join("[voice]"));
        let _ = std::fs::create_dir_all(tmp.join("[ox]"));
        let r = classify(
            &dirs(&["[qbx]", "[standalone]", "[voice]", "[ox]"]),
            tmp.to_str().unwrap(),
        );
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(r.state, BootstrapState::Synced);
        assert_eq!(r.local_present_count, 4);
    }
}
