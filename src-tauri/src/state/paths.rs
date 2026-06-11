use std::path::PathBuf;

pub fn rift_dir() -> std::io::Result<PathBuf> {
    let home = dirs_home()?;
    let dir = home.join(".rift");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Single source of truth for "where is the user's home". Tries `USERPROFILE`
/// (Windows) then `HOME` (POSIX). Other modules import this — don't reinvent.
pub fn dirs_home() -> std::io::Result<PathBuf> {
    if let Some(p) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        if !p.as_os_str().is_empty() {
            return Ok(p);
        }
    }
    if let Some(p) = std::env::var_os("HOME").map(PathBuf::from) {
        if !p.as_os_str().is_empty() {
            return Ok(p);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no USERPROFILE/HOME",
    ))
}

/// **BLOCKING** — uses `std::fs` + `std::thread::sleep` for the Windows
/// rename-retry backoff (worst case ~500ms across 4 attempts). Callers on
/// the tokio runtime should wrap in `tokio::task::spawn_blocking` for paths
/// known to hit the retry window (concurrent indexer / AV).
pub fn atomic_write_json(path: &std::path::Path, json: &str) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    // #80: a deterministic `.json.tmp` name lets concurrent writers truncate
    // each other's in-flight write. Unique per-call suffix (pid + monotonic
    // counter) keeps every save isolated; the rename target is still the
    // canonical path so readers never see the temp name.
    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);
    let suffix = format!(
        "{}-{}.json.tmp",
        std::process::id(),
        TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let stem = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("snapshot");
    let tmp = path.with_file_name(format!("{stem}.{suffix}"));
    // #128: previously a write/sync failure orphaned the tmp file (only the
    // rename-retry path had cleanup). Capture errors from the inner block
    // and best-effort remove the tmp before returning.
    let write_result: std::io::Result<()> = (|| {
        let mut f = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        f.write_all(json.as_bytes())?;
        // M4: fsync the temp file before the rename so a crash post-rename
        // can't leave us with a 0-byte target. Cheap on local SSDs.
        f.sync_all()?;
        Ok(())
    })();
    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    // M4: on Windows, AV scanners / Search indexers / Explorer thumbnailers
    // briefly hold the destination open for reading. MoveFileExW returns
    // ERROR_SHARING_VIOLATION transiently; retry a few times before failing.
    let mut last_err: Option<std::io::Error> = None;
    for attempt in 0..5u32 {
        match std::fs::rename(&tmp, path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(
                        50 * u64::from(attempt + 1),
                    ));
                }
            }
        }
    }
    let _ = std::fs::remove_file(&tmp);
    Err(last_err.unwrap_or_else(|| {
        std::io::Error::other("atomic_write_json: rename failed")
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_json_round_trips_and_overwrites() {
        let td = tempfile::tempdir().unwrap();
        let path = td.path().join("snap.json");
        atomic_write_json(&path, "{\"a\":1}").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"a\":1}");
        // Overwrite in place — readers must see the new content, no temp leak.
        atomic_write_json(&path, "{\"b\":2}").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"b\":2}");
        // No orphaned *.json.tmp left behind in the dir.
        let leftover: Vec<_> = std::fs::read_dir(td.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(leftover.is_empty(), "temp files leaked: {leftover:?}");
    }
}
