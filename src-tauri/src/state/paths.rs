use std::path::PathBuf;

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

