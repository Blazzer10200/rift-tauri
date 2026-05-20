// Shared environment lookups — single source of truth for the
// "who am I, where am I" probes that the lock-presence + edit-trail modules
// need. Avoids three hostname() impls across the crate.

/// Best-effort current username. Tries `USERNAME` (Windows) then `USER`
/// (POSIX). Returns "unknown" on miss.
pub fn current_user() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".into())
}

/// Best-effort hostname. Tries `COMPUTERNAME` (Windows) then `HOSTNAME`
/// env (POSIX shells), then reads `/proc/sys/kernel/hostname` on Linux,
/// then absolute-path `/bin/hostname` as a last resort. Returns
/// `Some(name)` on success, `None` on miss so callers can apply their own
/// fallback string. #32: switched off ambient-PATH `hostname` to remove
/// the `$PATH`-hijack vector on multi-user POSIX hosts.
pub fn hostname() -> Option<String> {
    if let Ok(v) = std::env::var("COMPUTERNAME") {
        return Some(v);
    }
    if let Ok(v) = std::env::var("HOSTNAME") {
        if !v.is_empty() {
            return Some(v);
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(s) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    // Absolute-path fallback. `/bin/hostname` on Linux/macOS; we accept the
    // file-not-found case as a miss rather than searching PATH.
    for candidate in ["/bin/hostname", "/usr/bin/hostname"] {
        if let Ok(o) = std::process::Command::new(candidate).output() {
            if let Ok(s) = String::from_utf8(o.stdout) {
                let t = s.trim();
                if !t.is_empty() {
                    return Some(t.to_string());
                }
            }
        }
    }
    None
}

/// 16-hex-char OS-randomness id. Used by edit/in-place + lock-presence +
/// edit-trail to disambiguate concurrent temp-dir names. Replaces
/// `chrono::Utc::now().timestamp_nanos_opt()` which can collide when two
/// callers fire inside the same nanosecond OR when nanos aren't supported.
pub fn short_id() -> String {
    let mut buf = [0u8; 8];
    rand::fill(&mut buf);
    format!("{:016x}", u64::from_le_bytes(buf))
}
