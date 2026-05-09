// Phase 1j — port of Rift Project/Services/State/LocalFs.cs.
//
// Local filesystem helpers used by AutoSync, drift scan, and (Phase 3) the
// Browse-tab LocalPane. Faithful port — most calls are thin wrappers over
// std::fs that swallow per-entry IO errors so one bad junction doesn't kill
// a whole listing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

/// Mirrors `LocalFs.GetParent`. None when at filesystem root.
pub fn get_parent(path: &Path) -> Option<PathBuf> {
    path.parent().map(|p| p.to_path_buf())
}

/// Human-readable byte count (KB/MB/GB) — matches the WPF activity-feed format.
pub fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Make a string safe to use as a directory name. Replaces filesystem-reserved
/// characters w/ `_`. Mirrors WPF's `Path.GetInvalidFileNameChars` filter.
pub fn safe_dir_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'..='\x1f' => out.push('_'),
            _ => out.push(c),
        }
    }
    let trimmed = out.trim_matches(|c: char| c == ' ' || c == '.').to_string();
    if trimmed.is_empty() { "_".to_string() } else { trimmed }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.00 MB");
        assert_eq!(human_size(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn safe_dir_name_strips_reserved() {
        assert_eq!(safe_dir_name("foo/bar"), "foo_bar");
        assert_eq!(safe_dir_name("a:b*c?"), "a_b_c_");
        assert_eq!(safe_dir_name("  ..hi.."), "hi");
        assert_eq!(safe_dir_name(""), "_");
    }

    #[test]
    fn list_missing_dir_is_empty() {
        let p = Path::new("/this/path/does/not/exist/__rift_tj");
        assert!(list_directory(p).is_empty());
    }
}
