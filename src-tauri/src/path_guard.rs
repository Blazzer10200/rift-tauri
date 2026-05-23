// Path-containment guards for destructive Tauri commands (rename/delete).
//
// Threat model (audit codex-2026-05-11, Bucket A):
//   - Frontend or devtools can invoke commands w/ arbitrary paths.
//   - Without containment, a crafted `..`, absolute path, or root reference
//     can step outside the mapped resource tree → cross-resource data loss
//     or worse (drive-root SFTP delete under account permissions).
//
// Policy: hybrid (decided 2026-05-11). Browser may navigate anywhere; only
// destructive ops are gated to the configured `remote_root` / `local_root`
// for the active server profile.

use std::path::{Component, Path, PathBuf};

use crate::profile::ServerProfile;

/// Validate a remote SFTP path stays strictly under `profile.remote_root`.
/// Normalizes POSIX separators, rejects empty / `..` / backslash, and refuses
/// to operate on the root itself.
///
/// Containment is case-sensitive and assumes the remote is a Linux-style SFTP
/// filesystem. Case-insensitive remotes (Samba/macOS) need a case-folded guard.
pub fn validate_remote_child(profile: &ServerProfile, path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("remote path is empty".into());
    }
    if trimmed.contains('\\') {
        return Err("remote path contains backslash".into());
    }
    let mut clean: Vec<&str> = Vec::new();
    for seg in trimmed.split('/') {
        match seg {
            "" | "." => continue,
            ".." => return Err("remote path contains '..'".into()),
            s => clean.push(s),
        }
    }
    if clean.is_empty() {
        return Err("remote path normalizes to root".into());
    }
    let normalized = format!("/{}", clean.join("/"));

    let root_raw = profile.remote_root.trim();
    if root_raw.is_empty() {
        return Err("profile.remote_root is empty".into());
    }
    let root_trim = root_raw.trim_end_matches('/');
    let root_norm = if root_trim.starts_with('/') {
        root_trim.to_string()
    } else {
        format!("/{root_trim}")
    };
    if root_norm == "/" {
        return Err("profile.remote_root '/' is not allowed for destructive ops".into());
    }

    if normalized == root_norm {
        return Err(format!("refusing to operate on remote_root itself: {root_norm}"));
    }
    if !normalized.starts_with(&format!("{root_norm}/")) {
        return Err(format!(
            "remote path '{normalized}' escapes remote_root '{root_norm}'"
        ));
    }
    Ok(normalized)
}

/// Permissive variant for read-only navigation (list/browse). Allows `path ==
/// remote_root` since the browser's entry point IS the root. Still rejects
/// `..` / backslash / drive-root escapes. Use `validate_remote_child` for
/// destructive ops (rename/delete/upload/edit) where root must be refused.
pub fn validate_remote_listable(profile: &ServerProfile, path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("remote path is empty".into());
    }
    if trimmed.contains('\\') {
        return Err("remote path contains backslash".into());
    }
    let mut clean: Vec<&str> = Vec::new();
    for seg in trimmed.split('/') {
        match seg {
            "" | "." => continue,
            ".." => return Err("remote path contains '..'".into()),
            s => clean.push(s),
        }
    }
    let normalized = if clean.is_empty() { "/".to_string() } else { format!("/{}", clean.join("/")) };

    let root_raw = profile.remote_root.trim();
    if root_raw.is_empty() {
        return Err("profile.remote_root is empty".into());
    }
    let root_trim = root_raw.trim_end_matches('/');
    let root_norm = if root_trim.starts_with('/') {
        root_trim.to_string()
    } else {
        format!("/{root_trim}")
    };
    if normalized == root_norm {
        return Ok(normalized);
    }
    if !normalized.starts_with(&format!("{root_norm}/")) {
        return Err(format!(
            "remote path '{normalized}' escapes remote_root '{root_norm}'"
        ));
    }
    Ok(normalized)
}

/// Validate a local path stays strictly under `profile.local_root`. Canonicalizes
/// the target (or its parent, for not-yet-existing rename targets) before the
/// containment check so symlinks can't smuggle escapes.
pub fn validate_local_child(profile: &ServerProfile, path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("local path is empty".into());
    }
    let p = Path::new(trimmed);
    for c in p.components() {
        if matches!(c, Component::ParentDir) {
            return Err("local path contains '..'".into());
        }
    }

    let canon = if p.exists() {
        p.canonicalize()
            .map_err(|e| format!("canonicalize '{}': {e}", p.display()))?
    } else {
        let parent = p
            .parent()
            .ok_or_else(|| format!("'{}' has no parent", p.display()))?;
        let name = p
            .file_name()
            .ok_or_else(|| format!("'{}' has no filename", p.display()))?;
        let canon_parent = parent
            .canonicalize()
            .map_err(|e| format!("canonicalize parent '{}': {e}", parent.display()))?;
        let joined = canon_parent.join(name);
        if joined.parent() != Some(canon_parent.as_path()) {
            return Err(format!("local path '{}' escapes parent", p.display()));
        }
        joined
    };

    let local_root = profile.local_root.trim();
    if local_root.is_empty() {
        return Err("profile.local_root is empty".into());
    }
    let root_canon = Path::new(local_root)
        .canonicalize()
        .map_err(|e| format!("canonicalize local_root '{local_root}': {e}"))?;
    if canon == root_canon {
        return Err(format!(
            "refusing to operate on local_root itself: {}",
            root_canon.display()
        ));
    }
    if !canon.starts_with(&root_canon) {
        return Err(format!(
            "local path '{}' escapes local_root '{}'",
            canon.display(),
            root_canon.display()
        ));
    }
    Ok(canon)
}

/// Structured per-path result for delete commands. Replaces the old `Vec<bool>`
/// shape so the UI can surface per-item reasons instead of a bare count.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpStatus {
    pub ok: bool,
    pub error: Option<String>,
}

impl OpStatus {
    pub fn ok() -> Self {
        Self { ok: true, error: None }
    }
    pub fn err(e: impl Into<String>) -> Self {
        Self { ok: false, error: Some(e.into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ServerProfile;

    fn profile_with_roots(remote_root: &str, local_root: &str) -> ServerProfile {
        ServerProfile {
            key: "test".into(),
            name: "Test".into(),
            host: "10.0.0.1".into(),
            port: 22,
            user: "root".into(),
            key_path: String::new(),
            remote_root: remote_root.into(),
            local_root: local_root.into(),
            fingerprint: None,
            tx_admin_url: None,
            added_at: None,
            bridge_token: None,
            bridge_port: None,
        }
    }

    // ── validate_remote_child ─────────────────────────────────────────────

    #[test]
    fn remote_child_accepts_path_under_root() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        let r = validate_remote_child(&p, "/opt/fxserver/resources/mod/init.lua").unwrap();
        assert_eq!(r, "/opt/fxserver/resources/mod/init.lua");
    }

    #[test]
    fn remote_child_normalizes_dot_and_doubleslash() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        let r = validate_remote_child(&p, "/opt/fxserver/./resources//mod/init.lua").unwrap();
        assert_eq!(r, "/opt/fxserver/resources/mod/init.lua");
    }

    #[test]
    fn remote_child_rejects_dotdot() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        let err = validate_remote_child(&p, "/opt/fxserver/../etc/passwd").unwrap_err();
        assert!(err.contains(".."), "err = {err}");
    }

    #[test]
    fn remote_child_rejects_backslash() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        let err = validate_remote_child(&p, "/opt/fxserver\\resources").unwrap_err();
        assert!(err.contains("backslash"), "err = {err}");
    }

    #[test]
    fn remote_child_rejects_empty() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        assert!(validate_remote_child(&p, "").is_err());
        assert!(validate_remote_child(&p, "   ").is_err());
    }

    #[test]
    fn remote_child_rejects_root_itself() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        let err = validate_remote_child(&p, "/opt/fxserver").unwrap_err();
        assert!(err.contains("remote_root itself"), "err = {err}");
    }

    #[test]
    fn remote_child_rejects_sibling_prefix_escape() {
        // /opt/fxserver-evil starts with the textual prefix "/opt/fxserver"
        // but is NOT under it. validate_remote_child must reject.
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        let err = validate_remote_child(&p, "/opt/fxserver-evil/x").unwrap_err();
        assert!(err.contains("escapes remote_root"), "err = {err}");
    }

    #[test]
    fn remote_child_rejects_drive_root_remote_root() {
        let p = profile_with_roots("/", "C:/local");
        let err = validate_remote_child(&p, "/anything").unwrap_err();
        assert!(err.contains("'/' is not allowed"), "err = {err}");
    }

    #[test]
    fn remote_child_trims_trailing_slash_on_root() {
        let p = profile_with_roots("/opt/fxserver/", "C:/local");
        let r = validate_remote_child(&p, "/opt/fxserver/a.lua").unwrap();
        assert_eq!(r, "/opt/fxserver/a.lua");
    }

    #[test]
    fn remote_child_adds_leading_slash_to_relative_root() {
        let p = profile_with_roots("opt/fxserver", "C:/local");
        let r = validate_remote_child(&p, "/opt/fxserver/a.lua").unwrap();
        assert_eq!(r, "/opt/fxserver/a.lua");
    }

    #[test]
    fn remote_child_rejects_empty_remote_root() {
        let p = profile_with_roots("", "C:/local");
        let err = validate_remote_child(&p, "/opt/anything").unwrap_err();
        assert!(err.contains("remote_root is empty"), "err = {err}");
    }

    // ── validate_remote_listable (permissive: root itself OK) ────────────

    #[test]
    fn remote_listable_accepts_root_itself() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        let r = validate_remote_listable(&p, "/opt/fxserver").unwrap();
        assert_eq!(r, "/opt/fxserver");
    }

    #[test]
    fn remote_listable_accepts_child() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        let r = validate_remote_listable(&p, "/opt/fxserver/resources").unwrap();
        assert_eq!(r, "/opt/fxserver/resources");
    }

    #[test]
    fn remote_listable_rejects_dotdot() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        assert!(validate_remote_listable(&p, "/opt/fxserver/../etc").is_err());
    }

    #[test]
    fn remote_listable_rejects_sibling_prefix() {
        let p = profile_with_roots("/opt/fxserver", "C:/local");
        assert!(validate_remote_listable(&p, "/opt/fxserver-evil/x").is_err());
    }

    // ── validate_local_child ─────────────────────────────────────────────

    #[test]
    fn local_child_rejects_empty() {
        let p = profile_with_roots("/r", "C:/some/root");
        assert!(validate_local_child(&p, "").is_err());
        assert!(validate_local_child(&p, "   ").is_err());
    }

    #[test]
    fn local_child_rejects_dotdot_component() {
        let p = profile_with_roots("/r", "C:/some/root");
        let err = validate_local_child(&p, "C:/some/root/../other/file.lua").unwrap_err();
        assert!(err.contains(".."), "err = {err}");
    }

    #[test]
    fn local_child_accepts_real_path_under_root() {
        // Build a real temp dir as the local_root, then validate a child path
        // inside it. Canonicalize-based containment requires real fs entries.
        let root = std::env::temp_dir().join(format!(
            "rift-pathguard-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let child = root.join("nested");
        std::fs::create_dir_all(&child).unwrap();
        let target = child.join("file.lua");
        std::fs::write(&target, "x").unwrap();

        let p = profile_with_roots("/r", root.to_str().unwrap());
        let canon = validate_local_child(&p, target.to_str().unwrap()).unwrap();
        // canonicalized path must start w/ canonicalized root
        let canon_root = root.canonicalize().unwrap();
        assert!(
            canon.starts_with(&canon_root),
            "canon {} not under {}",
            canon.display(),
            canon_root.display()
        );

        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn local_child_rejects_root_itself() {
        let root = std::env::temp_dir().join(format!(
            "rift-pathguard-rooitself-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();

        let p = profile_with_roots("/r", root.to_str().unwrap());
        let err = validate_local_child(&p, root.to_str().unwrap()).unwrap_err();
        assert!(err.contains("local_root itself"), "err = {err}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn local_child_rejects_empty_local_root() {
        // local_root empty — must error before canonicalize attempts.
        let p = profile_with_roots("/r", "");
        // Use a path that doesn't exist; the function checks for empty
        // local_root after canonicalize'ing the target.
        let nonexistent = std::env::temp_dir().join("rift-pathguard-no-such-file.lua");
        let res = validate_local_child(&p, nonexistent.to_str().unwrap());
        assert!(res.is_err());
    }
}
