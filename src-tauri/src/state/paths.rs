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
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}
