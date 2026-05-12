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
