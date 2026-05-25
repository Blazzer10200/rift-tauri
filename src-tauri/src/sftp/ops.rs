use super::*;

impl SftpClient {
    pub async fn remote_exists(&self, path: &str) -> bool {
        self.sftp.metadata(path).await.is_ok()
    }


    pub async fn remote_stat(&self, path: &str) -> RemoteFileInfo {
        match self.sftp.metadata(path).await {
            Ok(m) => RemoteFileInfo {
                exists: true,
                is_directory: matches!(m.file_type(), FileType::Dir),
                size: m.size.unwrap_or(0) as i64,
                last_modified: mtime_to_utc(m.mtime),
            },
            Err(_) => RemoteFileInfo {
                exists: false,
                is_directory: false,
                size: 0,
                last_modified: Utc::now(),
            },
        }
    }


    pub async fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        rename_via(&self.sftp, from, to).await
    }


    pub async fn delete(&self, path: &str) -> OpResult {
        let info = self.remote_stat(path).await;
        if !info.exists {
            // Nothing to delete server-side — treat as success so the local
            // delete is reconciled (otherwise drift would re-queue forever).
            return OpResult::ok();
        }
        if info.is_directory {
            return match delete_recursive_via(&self.sftp, path).await {
                Ok(_) => OpResult::ok(),
                Err(e) => OpResult::err(format!("delete dir {path}: {e}")),
            };
        }
        match self.sftp.remove_file(path).await {
            Ok(_) => OpResult::ok(),
            Err(e) => OpResult::err(format!("delete {path}: {e}")),
        }
    }


    pub async fn delete_recursive(&self, path: &str) -> Result<(), String> {
        delete_recursive_via(&self.sftp, path).await
    }


    pub async fn mkdir_p(&self, path: &str) -> Result<(), String> {
        mkdir_p_via(&self.sftp, path).await
    }


    /// Strict variant — every `create_dir` failure is verified against a
    /// metadata probe. If the path doesn't exist as a directory after the
    /// failed create, the error is surfaced instead of silently swallowed.
    pub async fn mkdir_p_strict(&self, path: &str) -> Result<(), String> {
        mkdir_p_strict_via(&self.sftp, path).await
    }


    pub async fn probe_write_access(&self, remote_root: &str) -> Result<(), String> {
        let root = remote_root.trim().trim_end_matches('/');
        if root.is_empty() || root == "/" {
            return Err("write probe refused empty/root remote_root".into());
        }
        let probe = format!(
            "{root}/.rift-write-probe-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_millis()
        );
        // Create proves write access. Cleanup is best-effort: if it fails
        // (ENOENT from a sweep, or transient SFTP error) we'd rather declare
        // the session healthy than reject the connection. A stray probe file
        // is harmless; failing to connect because of one is not.
        self.upload_bytes(b"rift-probe\n", &probe)
            .await
            .map_err(|e| format!("write probe create failed under {root}: {e}"))?;
        if let Err(e) = self.sftp.remove_file(&probe).await {
            log::warn!("write probe cleanup left {probe} on server (non-fatal): {e}");
        }
        Ok(())
    }}




// #86: per-op timeouts on the recursive-delete path. A wedged russh session
// during a multi-file delete (drift_watcher delete sweep) used to block the
// whole delete-drift path with no surfacing — now each SFTP op caps at
// T_QUICK or T_NORMAL (mirrors transfer.rs `with_t` discipline, kept local
// here to avoid promoting the helper across modules).
const OPS_T_QUICK: u64 = 10;
const OPS_T_NORMAL: u64 = 30;

async fn ops_with_t<F, T, E>(secs: u64, op: &str, target: &str, fut: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    match tokio::time::timeout(std::time::Duration::from_secs(secs), fut).await {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(format!("{op} {target}: {e}")),
        Err(_) => Err(format!("{op} {target}: timeout after {secs}s — connection wedged")),
    }
}

pub(super) async fn delete_recursive_via(sftp: &SftpSession, path: &str) -> Result<(), String> {
    // Belt-and-braces — Tauri cmd layer already containment-checks, but this
    // helper is reachable internally too. Don't let "/" or "" get this far.
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == "/" {
        return Err(format!("refusing to recursively delete root: '{trimmed}'"));
    }
    // lstat: never follow a symlink at the target. If it IS a symlink, unlink
    // the link only — never recurse through whatever it points to.
    let meta = ops_with_t(OPS_T_QUICK, "lstat", path, sftp.symlink_metadata(path)).await?;
    if meta.file_type().is_symlink() || !meta.file_type().is_dir() {
        return ops_with_t(OPS_T_NORMAL, "delete", path, sftp.remove_file(path)).await;
    }
    let mut to_visit: Vec<String> = vec![path.to_string()];
    let mut to_rmdir: Vec<String> = Vec::new();
    while let Some(dir) = to_visit.pop() {
        to_rmdir.push(dir.clone());
        let entries = ops_with_t(OPS_T_NORMAL, "readdir", &dir, sftp.read_dir(&dir)).await?;
        for entry in entries {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let full = format!("{}/{}", dir.trim_end_matches('/'), name);
            // Per-child lstat — entry.metadata() may report followed-target
            // attrs depending on server, which could push a symlinked dir
            // onto the visit stack and walk out of tree.
            let child_meta = ops_with_t(OPS_T_QUICK, "lstat", &full, sftp.symlink_metadata(&full)).await?;
            let ft = child_meta.file_type();
            if ft.is_dir() && !ft.is_symlink() {
                to_visit.push(full);
            } else {
                ops_with_t(OPS_T_NORMAL, "delete", &full, sftp.remove_file(&full)).await?;
            }
        }
    }
    while let Some(d) = to_rmdir.pop() {
        ops_with_t(OPS_T_NORMAL, "rmdir", &d, sftp.remove_dir(&d)).await?;
    }
    Ok(())
}


/// #84: KNOWN TOCTOU — SFTP has no atomic conditional-rename primitive, so
/// the exists-check + rename below has an inherent race. If two writers race
/// on the same target, both can pass the `try_exists` probe and one will
/// silently clobber the other. This helper is the right call ONLY when the
/// caller already holds an advisory lock on `to` (e.g. via `LockPresence`)
/// or when overwrite-loss is acceptable. For authoritative overwrite paths
/// (atomic-replace tmp swap), use `rename_overwriting_via` instead — it
/// declares intent and skips the racing exists-check entirely.
async fn rename_via(sftp: &SftpSession, from: &str, to: &str) -> Result<(), String> {
    match sftp.try_exists(to).await {
        Ok(true) => return Err(format!("target already exists: {to}")),
        Ok(false) => {}
        Err(e) => return Err(format!("exists check {to}: {e}")),
    }
    sftp.rename(from, to)
        .await
        .map_err(|e| format!("rename {from} -> {to}: {e}"))
}


pub(super) async fn rename_overwriting_via(sftp: &SftpSession, from: &str, to: &str) -> Result<(), String> {
    let _ = sftp.remove_file(to).await;
    sftp.rename(from, to)
        .await
        .map_err(|e| format!("rename {from} -> {to}: {e}"))
}


pub(super) async fn mkdir_p_via(sftp: &SftpSession, path: &str) -> Result<(), String> {
    mkdir_p_inner(sftp, path, false).await
}


/// Strict mkdir — propagates errors when `create_dir` fails AND a metadata
/// probe confirms the path is not a directory. Use when a silent mkdir miss
/// would later cause "No such file" on file create and a phantom Push success.
pub(crate) async fn mkdir_p_strict_via(sftp: &SftpSession, path: &str) -> Result<(), String> {
    mkdir_p_inner(sftp, path, true).await
}


async fn mkdir_p_inner(sftp: &SftpSession, path: &str, strict: bool) -> Result<(), String> {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let leading_slash = path.starts_with('/');
    let mut cur = if leading_slash { String::new() } else { String::from(".") };
    for p in parts {
        if leading_slash || cur != "." {
            cur.push('/');
        } else {
            cur.clear();
        }
        cur.push_str(p);
        let create_err = sftp.create_dir(&cur).await.err();
        if strict {
            if let Some(err) = create_err {
                // create_dir failed — could be "already exists" (idempotent
                // success) or a real error. Probe metadata to disambiguate.
                match sftp.metadata(&cur).await {
                    Ok(m) if matches!(m.file_type(), FileType::Dir) => {
                        // Exists as dir — race won by another worker / pre-existing.
                    }
                    Ok(_) => {
                        return Err(format!(
                            "mkdir {cur}: path exists but is not a directory ({err})"
                        ));
                    }
                    Err(probe_err) => {
                        return Err(format!("mkdir {cur}: {err} (probe: {probe_err})"));
                    }
                }
            }
        }
        // Force mode 2775 (setgid + group-writable). Default umask 0022 leaves
        // new dirs at 0755 → only the creator can write inside, so a teammate
        // pushing into a dir the other person created hits EACCES on the
        // tmp-file create. v0.2.26 fixed the file case (0664); this is the
        // directory case. Setgid (the leading `2`) makes new files inside
        // inherit the parent's group, which keeps the shared-group model
        // working without per-file `chown`. Best-effort: if we don't own
        // this segment the SETSTAT fails silently, no harm — the next push
        // by someone who DOES own it heals the perms. Combined w/ runs on
        // every upload's `ensure_remote_parent_dir`, the tree converges
        // toward correct perms over time without any one-shot SSH cleanup.
        let mut attrs = russh_sftp::protocol::FileAttributes::empty();
        attrs.permissions = Some(0o2775);
        let _ = sftp.set_metadata(&cur, attrs).await;
    }
    Ok(())
}


pub(crate) fn remote_parent(path: &str) -> Option<&str> {
    let trimmed = path.trim_end_matches('/');
    let idx = trimmed.rfind('/')?;
    if idx == 0 { Some("/") } else { Some(&trimmed[..idx]) }
}


