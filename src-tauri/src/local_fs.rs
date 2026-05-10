// Phase 1j — port of Rift Project/Services/State/LocalFs.cs.
//
// Local filesystem helpers used by AutoSync, drift scan, and (Phase 3) the
// Browse-tab LocalPane. Faithful port — most calls are thin wrappers over
// std::fs that swallow per-entry IO errors so one bad junction doesn't kill
// a whole listing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalEntry {
    pub name: String,
    pub full_path: String,
    pub is_directory: bool,
    pub size: u64,
    pub last_modified: DateTime<Utc>,
}

/// Mirrors `LocalFs.ListDirectoryAsync`. Returns dirs-first, then files,
/// each group case-insensitively sorted by name. Missing / permission-denied
/// roots return an empty list (not an error) — matches the WPF surface so
/// callers don't have to special-case "not yet downloaded".
pub fn list_directory(local_path: &Path) -> Vec<LocalEntry> {
    if local_path.as_os_str().is_empty() || !local_path.is_dir() {
        return Vec::new();
    }

    let mut dirs: Vec<LocalEntry> = Vec::new();
    let mut files: Vec<LocalEntry> = Vec::new();

    let it = match std::fs::read_dir(local_path) {
        Ok(it) => it,
        Err(_) => return Vec::new(),
    };

    for entry_res in it {
        let Ok(entry) = entry_res else { continue };
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();
        let mtime: DateTime<Utc> = meta
            .modified()
            .ok()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(Utc::now);

        let row = LocalEntry {
            name,
            full_path: path.to_string_lossy().to_string(),
            is_directory: meta.is_dir(),
            size: if meta.is_dir() { 0 } else { meta.len() },
            last_modified: mtime,
        };
        if meta.is_dir() { dirs.push(row); } else { files.push(row); }
    }

    dirs.sort_by_key(|a| a.name.to_ascii_lowercase());
    files.sort_by_key(|a| a.name.to_ascii_lowercase());
    dirs.extend(files);
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_missing_dir_is_empty() {
        let p = Path::new("/this/path/does/not/exist/__rift_tj");
        assert!(list_directory(p).is_empty());
    }
}
