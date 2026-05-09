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

/// Best-effort hostname. Tries `COMPUTERNAME` (Windows) then shells out to
/// `hostname` (POSIX). Returns `Some(name)` on success, `None` on miss so
/// callers can apply their own fallback string.
pub fn hostname() -> Option<String> {
    std::env::var("COMPUTERNAME").ok().or_else(|| {
        std::process::Command::new("hostname")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    })
}

/// 8-hex-char OS-randomness id. Used by edit/in-place + lock-presence +
/// edit-trail to disambiguate concurrent temp-dir names. Replaces
/// `chrono::Utc::now().timestamp_nanos_opt()` which can collide when two
/// callers fire inside the same nanosecond OR when nanos aren't supported.
pub fn short_id() -> String {
    let mut buf = [0u8; 4];
    rand::fill(&mut buf);
    format!("{:08x}", u32::from_le_bytes(buf))
}
