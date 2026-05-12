use super::*;
use std::path::Path;

pub(super) fn classify_action(a: &str) -> ActivityKind {
    let lower = a.to_ascii_lowercase();
    if lower.contains("blocked") {
        ActivityKind::Block
    } else if lower.contains("conflict\u{2192}") || lower.contains("conflict skipped") {
        ActivityKind::ConflictResolved
    } else if lower.contains("conflict") {
        ActivityKind::Conflict
    } else if lower.contains("drift") || lower.contains("scan") {
        ActivityKind::Drift
    } else if lower.contains("delete") || lower.contains("removed locally") {
        ActivityKind::Delete
    } else if lower.contains("synced") {
        ActivityKind::Sync
    } else if lower.contains("pull") {
        ActivityKind::Pull
    } else if lower.contains("[bridge]") || lower.contains("restart") || lower.contains("watching") {
        ActivityKind::Bridge
    } else if lower.contains("fail") || lower.contains("error") || lower.contains("rejected") {
        ActivityKind::Error
    } else {
        ActivityKind::System
    }
}


pub(super) fn map_local_to_remote(local: &Path, fw: &FolderWatch) -> Option<String> {
    let rel = local.strip_prefix(&fw.local_root).ok()?;
    let rel_s = rel.to_string_lossy().replace('\\', "/");
    if rel_s == "." || rel_s.is_empty() {
        return Some(fw.remote_root.clone());
    }
    if rel_s.starts_with("../") || rel_s.starts_with('/') {
        return None;
    }
    Some(format!("{}/{}", fw.remote_root.trim_end_matches('/'), rel_s))
}


pub(super) fn rel_of(fw: &FolderWatch, local: &Path) -> String {
    local
        .strip_prefix(&fw.local_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}


pub(super) fn file_name(p: &Path) -> &str {
    p.file_name().and_then(|s| s.to_str()).unwrap_or("?")
}


pub(super) fn stat_local(p: &Path) -> Option<(i64, DateTime<Utc>)> {
    let m = std::fs::metadata(p).ok()?;
    let size = m.len() as i64;
    let mtime = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
        .unwrap_or_else(Utc::now);
    Some((size, mtime))
}


pub(super) async fn wait_for_readable(path: &Path) -> bool {
    for _ in 0..4 {
        if tokio::fs::File::open(path).await.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}


pub(super) async fn safe_count_files(root: &Path) -> usize {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut count = 0usize;
        for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                count += 1;
                if count >= 5000 {
                    break;
                }
            }
        }
        count
    })
    .await
    .unwrap_or(0)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn merge_kind_delete_wins() {
        assert_eq!(merge_kind(ChangeKind::Created, ChangeKind::Deleted), ChangeKind::Deleted);
        assert_eq!(merge_kind(ChangeKind::Modified, ChangeKind::Deleted), ChangeKind::Deleted);
    }

    #[test]
    fn merge_kind_create_sticks() {
        assert_eq!(merge_kind(ChangeKind::Created, ChangeKind::Modified), ChangeKind::Created);
    }

    #[test]
    fn merge_kind_default_takes_b() {
        assert_eq!(merge_kind(ChangeKind::Modified, ChangeKind::Created), ChangeKind::Created);
    }

    #[test]
    pub(super) fn map_local_to_remote_basic() {
        let fw = FolderWatch {
            local_root: PathBuf::from("/srv/local/qbx_core"),
            remote_root: "/opt/server/[qbx]/qbx_core".into(),
            resource_name: "qbx_core".into(),
        };
        let out = map_local_to_remote(Path::new("/srv/local/qbx_core/server/main.lua"), &fw);
        assert_eq!(out.as_deref(), Some("/opt/server/[qbx]/qbx_core/server/main.lua"));
    }

    #[test]
    pub(super) fn map_local_to_remote_outside_returns_none() {
        let fw = FolderWatch {
            local_root: PathBuf::from("/srv/local/qbx_core"),
            remote_root: "/opt/server/[qbx]/qbx_core".into(),
            resource_name: "qbx_core".into(),
        };
        let out = map_local_to_remote(Path::new("/srv/other/main.lua"), &fw);
        assert!(out.is_none());
    }

    #[test]
    pub(super) fn classify_action_buckets() {
        assert_eq!(classify_action("synced"), ActivityKind::Sync);
        assert_eq!(classify_action("deleted"), ActivityKind::Delete);
        assert_eq!(classify_action("BLOCKED — 30 deletes"), ActivityKind::Block);
        assert_eq!(classify_action("CONFLICT — remote changed"), ActivityKind::Conflict);
        assert_eq!(classify_action("conflict→accept-remote"), ActivityKind::ConflictResolved);
        assert_eq!(classify_action("[bridge] restart triggered"), ActivityKind::Bridge);
        assert_eq!(classify_action("sync failed: timeout"), ActivityKind::Error);
    }
}
