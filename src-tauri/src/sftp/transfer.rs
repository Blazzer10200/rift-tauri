use super::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

impl SftpClient {
    pub async fn upload_file_atomic(
        &self,
        local_path: &Path,
        remote_path: &str,
    ) -> OpResult {
        upload_atomic_via(&self.sftp, local_path, remote_path).await
    }


    pub async fn download_file_atomic(
        &self,
        remote_path: &str,
        local_path: &Path,
    ) -> OpResult {
        download_atomic_via(&self.sftp, remote_path, local_path).await
    }


    pub async fn download_files_batch(
        &self,
        jobs: &[(String, PathBuf)],
        parallelism: usize,
        ct: tokio_util::sync::CancellationToken,
    ) -> Vec<bool> {
        let mut results = vec![false; jobs.len()];
        if jobs.is_empty() {
            return results;
        }

        self.ensure_workers(parallelism).await;
        let live = self.live_worker_count().await;
        if live == 0 {
            // Fallback: serial atomic download on the main session.
            for (i, (r, l)) in jobs.iter().enumerate() {
                if ct.is_cancelled() {
                    break;
                }
                let res = self.download_file_atomic(r, l).await;
                results[i] = res.success;
            }
            return results;
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<(usize, String, PathBuf)>();
        for (i, (r, l)) in jobs.iter().enumerate() {
            let _ = tx.send((i, r.clone(), l.clone()));
        }
        drop(tx);
        let rx = Arc::new(tokio::sync::Mutex::new(rx));

        let workers: Vec<Arc<Worker>> = {
            let g = self.workers.lock().await;
            g.iter().take(live).cloned().collect()
        };

        let results_arc = Arc::new(tokio::sync::Mutex::new(results));
        let mut tasks = Vec::with_capacity(workers.len());
        for worker in workers {
            let rx = rx.clone();
            let results_arc = results_arc.clone();
            let ct = ct.clone();
            tasks.push(tokio::spawn(async move {
                loop {
                    if ct.is_cancelled() {
                        break;
                    }
                    let job = {
                        let mut rxg = rx.lock().await;
                        rxg.recv().await
                    };
                    let Some((idx, remote, local)) = job else { break };
                    let _g = worker.gate.lock().await;
                    let res = tokio::select! {
                        r = download_atomic_via(&worker.sftp, &remote, &local) => r,
                        _ = ct.cancelled() => OpResult::err("cancelled"),
                    };
                    let mut r = results_arc.lock().await;
                    r[idx] = res.success;
                }
            }));
        }
        let _ = futures::future::join_all(tasks).await;
        Arc::try_unwrap(results_arc)
            .map(|m| m.into_inner())
            .unwrap_or_default()
    }


    pub async fn upload_files_batch(
        &self,
        jobs: &[(PathBuf, String)],
        parallelism: usize,
    ) -> Vec<bool> {
        let mut results = vec![false; jobs.len()];
        if jobs.is_empty() {
            return results;
        }

        self.ensure_workers(parallelism).await;
        let live = self.live_worker_count().await;
        if live == 0 {
            for (i, (l, r)) in jobs.iter().enumerate() {
                let res = self.upload_file_atomic(l, r).await;
                results[i] = res.success;
            }
            return results;
        }

        // Pre-create unique parent dirs on main session — workers won't race
        // on overlapping mkdir paths.
        let mut unique_parents: std::collections::HashSet<&str> =
            std::collections::HashSet::new();
        for (_, r) in jobs.iter() {
            if let Some(p) = super::ops::remote_parent(r) {
                if p != "/" {
                    unique_parents.insert(p);
                }
            }
        }
        for parent in unique_parents {
            let _ = self.mkdir_p(parent).await;
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<(usize, PathBuf, String)>();
        for (i, (l, r)) in jobs.iter().enumerate() {
            let _ = tx.send((i, l.clone(), r.clone()));
        }
        drop(tx);
        let rx = Arc::new(tokio::sync::Mutex::new(rx));

        let workers: Vec<Arc<Worker>> = {
            let g = self.workers.lock().await;
            g.iter().take(live).cloned().collect()
        };

        let results_arc = Arc::new(tokio::sync::Mutex::new(results));
        let mut tasks = Vec::with_capacity(workers.len());
        for worker in workers {
            let rx = rx.clone();
            let results_arc = results_arc.clone();
            tasks.push(tokio::spawn(async move {
                loop {
                    let job = {
                        let mut rxg = rx.lock().await;
                        rxg.recv().await
                    };
                    let Some((idx, local, remote)) = job else { break };
                    let _g = worker.gate.lock().await;
                    let res = upload_atomic_via(&worker.sftp, &local, &remote).await;
                    let mut r = results_arc.lock().await;
                    r[idx] = res.success;
                }
            }));
        }
        let _ = futures::future::join_all(tasks).await;
        Arc::try_unwrap(results_arc)
            .map(|m| m.into_inner())
            .unwrap_or_default()
    }


    pub async fn upload_bytes(&self, bytes: &[u8], remote_path: &str) -> Result<(), String> {
        // russh-sftp's `write()` is WRITE-only (no CREATE/TRUNCATE) — fails on
        // first creation and leaves trailing garbage if the new payload is
        // shorter than the existing file. `create()` is WRITE|CREATE|TRUNCATE.
        // shutdown() flushes pending writes + closes the SFTP file handle
        // server-side. Without it the close races with subsequent ops —
        // probe_write_access hit "remove_file: No such file" because the file
        // hadn't materialized server-side before cleanup ran.
        let mut f = self
            .sftp
            .create(remote_path)
            .await
            .map_err(|e| format!("create {remote_path}: {e}"))?;
        f.write_all(bytes)
            .await
            .map_err(|e| format!("write {remote_path}: {e}"))?;
        f.shutdown()
            .await
            .map_err(|e| format!("close {remote_path}: {e}"))
    }


    pub async fn download_file(
        &self,
        remote_path: &str,
        local_dir: &Path,
    ) -> Result<PathBuf, String> {
        std::fs::create_dir_all(local_dir).map_err(|e| format!("mkdir: {e}"))?;
        let name = remote_path
            .rsplit_once('/')
            .map(|(_, n)| n)
            .unwrap_or(remote_path);
        let local_path = local_dir.join(name);
        let buf = self
            .sftp
            .read(remote_path)
            .await
            .map_err(|e| format!("read {remote_path}: {e}"))?;
        std::fs::write(&local_path, &buf).map_err(|e| format!("write local: {e}"))?;
        Ok(local_path)
    }}



fn format_sftp_err(op: &str, target: &str, err: impl std::fmt::Display) -> String {
    let raw = err.to_string();
    // Collapse "X: X" duplicates russh-sftp emits.
    let cleaned = match raw.split_once(": ") {
        Some((a, b)) if a == b => a.to_string(),
        _ => raw.clone(),
    };
    if cleaned.contains("Permission denied") || cleaned.contains("permission denied") {
        format!(
            "{op} {target}: permission denied — your SSH user can't write here. \
Server admin: sudo chgrp -R <shared-group> <parent dir> && sudo chmod -R g+w <parent dir> \
&& sudo find <parent dir> -type d -exec chmod g+s {{}} \\;"
        )
    } else if cleaned.contains("No such file") || cleaned.contains("no such file") {
        format!("{op} {target}: remote path missing — parent dir may not exist")
    } else {
        format!("{op} {target}: {cleaned}")
    }
}


async fn upload_atomic_via(
    sftp: &SftpSession,
    local_path: &Path,
    remote_path: &str,
) -> OpResult {
    let bytes = match std::fs::read(local_path) {
        Ok(b) => b,
        Err(e) => return OpResult::err(format!("read local {}: {e}", local_path.display())),
    };
    if let Some(parent) = super::ops::remote_parent(remote_path) {
        // Strict mkdir — surface failure instead of silently swallowing.
        // Without this, a failed parent-dir create caused `sftp.create(&tmp)`
        // to return "No such file" downstream, but only as one of many fail
        // counts in a batch — files appeared "dropped" with no actionable
        // error. Strict mode disambiguates "exists as dir" (idempotent ok)
        // from "real failure" via a metadata probe on each create_dir miss.
        if let Err(e) = super::ops::mkdir_p_strict_via(sftp, parent).await {
            return OpResult::err(format_sftp_err("mkdir parent", parent, e));
        }
    }
    let tmp = format!("{remote_path}.rift-tmp");
    // Best-effort cleanup of any abandoned tmp from a prior crashed upload
    // (possibly owned by another user). POSIX unlink perm comes from parent-dir
    // write, so this clears foreign-owned tmps when the dir is group-writable.
    // Ignore the result — if it doesn't exist, or we can't unlink, `create()`
    // below will surface the real error.
    let _ = sftp.remove_file(&tmp).await;
    // russh-sftp's `write()` uses OpenFlags::WRITE only — no CREATE/TRUNCATE,
    // so it fails NO_SUCH_FILE on a fresh tmp. `create()` is WRITE|CREATE|TRUNCATE.
    // Wrap in a scope so the file handle drops (and the SFTP close packet
    // is sent) before we attempt the rename.
    {
        let mut f = match sftp.create(&tmp).await {
            Ok(f) => f,
            Err(e) => return OpResult::err(format_sftp_err("create tmp", &tmp, e)),
        };
        if let Err(e) = f.write_all(&bytes).await {
            drop(f);
            let _ = sftp.remove_file(&tmp).await;
            return OpResult::err(format_sftp_err("write tmp", &tmp, e));
        }
    }
    if let Err(e) = super::ops::rename_overwriting_via(sftp, &tmp, remote_path).await {
        let _ = sftp.remove_file(&tmp).await;
        return OpResult::err(format_sftp_err("rename", remote_path, e));
    }
    // Force mode 0664 on the uploaded file so other users in the shared
    // group can overwrite-rename it on their next push. Default umask 0022
    // would leave new files at 0644 and re-trigger the EACCES tmp-rename
    // failure that drove the v0.2.25 error-message work.
    //
    // CRITICAL: `FileAttributes::default()` is NOT what you want for a
    // partial SETSTAT — it sets `size: Some(0)` which truncates the file
    // to zero bytes server-side, plus `mtime/atime: Some(0)` which clobbers
    // timestamps to epoch 1970. That bug shipped in v0.2.26 and destroyed
    // every Trey-uploaded file (server.lua, client.lua, fxmanifest.lua all
    // went to 0 bytes + Jan 1 1970 mtime). Use `empty()` (all `None`) so
    // the SETSTAT packet only carries the fields we explicitly set —
    // SFTP spec semantics are "update only present fields".
    let mut attrs = russh_sftp::protocol::FileAttributes::empty();
    attrs.permissions = Some(0o664);
    let _ = sftp.set_metadata(remote_path, attrs).await;
    OpResult::ok()
}


async fn download_atomic_via(
    sftp: &SftpSession,
    remote_path: &str,
    local_path: &Path,
) -> OpResult {
    let bytes = match sftp.read(remote_path).await {
        Ok(b) => b,
        Err(e) => return OpResult::err(format!("read {remote_path}: {e}")),
    };
    if let Some(parent) = local_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return OpResult::err(format!("mkdir {}: {e}", parent.display()));
        }
    }
    let tmp_name = format!(
        "{}.rift-tmp",
        local_path.file_name().and_then(|s| s.to_str()).unwrap_or("download")
    );
    let tmp = local_path
        .parent()
        .map(|p| p.join(&tmp_name))
        .unwrap_or_else(|| std::path::PathBuf::from(&tmp_name));
    if let Err(e) = std::fs::write(&tmp, &bytes) {
        return OpResult::err(format!("write tmp {}: {e}", tmp.display()));
    }
    if let Err(e) = std::fs::rename(&tmp, local_path) {
        let _ = std::fs::remove_file(&tmp);
        return OpResult::err(format!(
            "rename {} -> {}: {e}",
            tmp.display(),
            local_path.display()
        ));
    }
    OpResult::ok()
}


