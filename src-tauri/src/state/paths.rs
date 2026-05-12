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

pub fn safe_profile_key(key: &str) -> String {
    key.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

pub fn cache_path(prefix: &str, profile_key: &str) -> std::io::Result<PathBuf> {
    let dir = rift_dir()?;
    let safe = safe_profile_key(profile_key);
    Ok(dir.join(format!("{prefix}-{safe}.json")))
}

pub fn atomic_write_json(path: &std::path::Path, json: &str) -> std::io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let tmp = path.with_extension("json.tmp");
    {
        let mut f = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        f.write_all(json.as_bytes())?;
        // M4: fsync the temp file before the rename so a crash post-rename
        // can't leave us with a 0-byte target. Cheap on local SSDs.
        f.sync_all()?;
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
