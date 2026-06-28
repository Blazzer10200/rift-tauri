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

/// `dirs_home` with an absolute-temp fallback. Use when a missing home must NOT
/// abort (e.g. writing a config file): falling back to `temp_dir()` keeps the path
/// absolute so it never lands CWD-relative next to the exe (which a Velopack
/// install update-wipes). Prefer the `Result` form when a missing home is a real
/// error to surface. (cont.228: was inline in stt::dirs_home.)
pub fn dirs_home_or_temp() -> PathBuf {
    dirs_home().unwrap_or_else(|_| std::env::temp_dir())
}

