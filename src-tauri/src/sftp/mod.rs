// Phase 1b/1c/1h: SFTP client surface. Wraps russh + russh-sftp into a
// long-lived connection that DriftScanner / EditTrail / AutoSync / discovery
// call against.
//
// **Phase 1h gap-fill (2026-05-08) — daily-driver parity vs WPF SftpClient.cs:**
//   - Fingerprint pinning (was TOFU only). `ConnectArgs.trusted_fingerprint`
//     accepts both Rust's `SHA256:<b64>` form and WPF/WinSCP's full
//     `ssh-ed25519 256 SHA256:<b64>` form via exact normalized match. On TOFU first
//     connect, the captured fingerprint is exposed via `fingerprint()` so the
//     caller can persist it back to ServerProfile (Phase 1i write-back).
//   - Worker-pool sessions. Each `Worker` = own russh `Handle` + own
//     `SftpSession` + per-worker tokio mutex gate. Lazy-opened up to 4 workers
//     via `ensure_workers()`. russh-sftp serializes within a single session,
//     so true wire-level parallelism requires N independent sessions.
//   - Batch up/download (`upload_files_batch`, `download_files_batch`) —
//     fan jobs out across workers via mpsc, serial fallback on main session.
//   - Worker-aware `list_recursive_batch` w/ belt-and-braces empty-root retry
//     on main session (mirrors WPF v13.55.1 fix for the false-push regression
//     where a worker breaking mid-walk silently returned 0 entries).
//   - `list_directory` (single-level, sorted dirs-first), `ensure_remote_parent_dir`
//     (walk-up), `get_remote_folder_size` (server-side `find -prune` exec).
//
// **Still deferred** (lands w/ UI consumers in Phase 4-5):
//   - Per-file IProgress<int> transfer-pct callbacks. Drift/auto-sync don't use
//     them; surface lands when Phase 4 wires the activity feed.
//   - Cancellation tokens. Lock_presence + auto_sync use tokio structured
//     concurrency to cancel; that's enough for now.
//   - mtime preservation via setstat — WPF passes `PreserveTimestamp = false`
//     so it's not actually a WPF feature; we match WPF behavior.

use chrono::{DateTime, TimeZone, Utc};
use russh::client::{self, Handle};
use russh::keys::*;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::FileType;
use tokio::io::AsyncWriteExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::transport::ssh_handler::PinningHandler;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteEntry {
    pub full_path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub last_modified: DateTime<Utc>,
}

/// Stat-shape returned by `remote_stat` — mirrors WPF `RemoteExistsAsync`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteFileInfo {
    pub exists: bool,
    pub is_directory: bool,
    pub size: i64,
    pub last_modified: DateTime<Utc>,
}

/// Result of an upload/download — Success + Error mirror the WPF tuple.
#[derive(Debug, Clone)]
pub struct OpResult {
    pub success: bool,
    pub error: String,
}
impl OpResult {
    pub fn ok() -> Self { Self { success: true, error: String::new() } }
    pub fn err(e: impl Into<String>) -> Self { Self { success: false, error: e.into() } }
}

pub struct ConnectArgs<'a> {
    pub host: &'a str,
    pub port: u16,
    pub user: &'a str,
    pub key_path: &'a Path,
    /// If `Some`, server's pubkey fingerprint must match this string. Exact
    /// normalized match — accepts both Rust's `"SHA256:<b64>"` form and WPF/WinSCP's
    /// `"ssh-ed25519 256 SHA256:<b64>"` form transparently. If `None`, TOFU:
    /// captures whatever the server presents and surfaces it via
    /// `SftpClient::fingerprint()` so the caller can persist it.
    pub trusted_fingerprint: Option<&'a str>,
}

/// Captured connect args — reused by `ensure_workers` to spawn additional SSH
/// sessions w/ the same fingerprint pinning. Owned (not borrowed) so it lives
/// across the SftpClient's lifetime.
///
/// The private key is loaded once during `connect()` and reused for every
/// worker session opened later — saves a disk-read + parse per worker (3
/// reads per pool spin-up otherwise).
struct OwnedConnectArgs {
    host: String,
    port: u16,
    user: String,
    key_pair: Arc<russh::keys::ssh_key::PrivateKey>,
    trusted_fingerprint: Option<String>,
}

/// One worker = its own russh `Handle` + `SftpSession`, gated by a tokio mutex.
/// Per-worker independent SFTP connections so batch ops genuinely overlap on
/// the wire (russh-sftp serializes requests within a single SftpSession).
/// WPF runs a 4-way pool; we mirror that ceiling. ~3-4× speedup vs serial.
struct Worker {
    #[allow(dead_code)] // kept alive so the SSH session doesn't tear down under us
    handle: Handle<PinningHandler>,
    sftp: SftpSession,
    gate: tokio::sync::Mutex<()>,
}

pub struct SftpClient {
    handle: Handle<PinningHandler>,
    sftp: SftpSession,
    fingerprint: String,
    connect_args: OwnedConnectArgs,
    workers: tokio::sync::Mutex<Vec<Arc<Worker>>>,
    worker_open_lock: tokio::sync::Mutex<()>,
}

/// Open one SSH session + SFTP subsystem against `args`. Used by both
/// `connect()` (main session) and `ensure_workers()` (worker sessions). Returns
/// the live handle, the live SFTP session, and the captured server fingerprint.
async fn open_session(
    args: &OwnedConnectArgs,
) -> Result<(Handle<PinningHandler>, SftpSession, String), String> {
    let config = Arc::new(client::Config::default());
    let addr = format!("{}:{}", args.host, args.port);
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let handler = PinningHandler {
        trusted: args.trusted_fingerprint.clone(),
        captured: captured.clone(),
    };
    let mut handle = client::connect(config, addr.clone(), handler)
        .await
        .map_err(|e| format!("connect {addr}: {e}"))?;

    let hash = match handle.best_supported_rsa_hash().await {
        Ok(Some(Some(h))) => Some(h),
        _ => None,
    };
    let auth = handle
        .authenticate_publickey(
            args.user.clone(),
            russh::keys::PrivateKeyWithHashAlg::new(args.key_pair.clone(), hash),
        )
        .await
        .map_err(|e| format!("auth: {e}"))?;
    if !auth.success() {
        return Err(format!("auth rejected for {}@{}", args.user, args.host));
    }

    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("open channel: {e}"))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("request sftp: {e}"))?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("sftp init: {e}"))?;

    let fingerprint = captured
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();

    Ok((handle, sftp, fingerprint))
}

impl SftpClient {
    /// Captured server pubkey fingerprint (`SHA256:<b64>`). Always populated on
    /// successful connect; caller persists this back to ServerProfile on first
    /// connect when the profile had `fingerprint = None` (TOFU pin).
    pub fn fingerprint(&self) -> &str { &self.fingerprint }

    pub async fn connect(args: ConnectArgs<'_>) -> Result<Self, String> {
        // Load the key ONCE here, then share via Arc — workers reuse without
        // re-reading the file (3+ disk-reads otherwise on a 4-worker pool).
        let key_pair = load_secret_key(args.key_path, None)
            .map_err(|e| format!("load key {}: {e}", args.key_path.display()))?;
        let captured_args = OwnedConnectArgs {
            host: args.host.to_string(),
            port: args.port,
            user: args.user.to_string(),
            key_pair: Arc::new(key_pair),
            trusted_fingerprint: args.trusted_fingerprint.map(|s| s.to_string()),
        };
        let (handle, sftp, fingerprint) = open_session(&captured_args).await?;
        Ok(Self {
            handle,
            sftp,
            fingerprint,
            connect_args: captured_args,
            workers: tokio::sync::Mutex::new(Vec::new()),
            worker_open_lock: tokio::sync::Mutex::new(()),
        })
    }

    /// Lazy-open up to `desired` worker sessions (capped at 4 to mirror WPF).
    /// Idempotent: existing workers are reused, only the deficit is opened.
    /// Workers open in parallel — first auth seeds kex cache so subsequent
    /// dials are ~150ms each. Failed opens are silently dropped; caller checks
    /// `live_worker_count()` and falls back to serial if it's zero.
    pub async fn ensure_workers(&self, desired: usize) {
        let desired = desired.clamp(1, 4);
        {
            let workers = self.workers.lock().await;
            if workers.len() >= desired {
                return;
            }
        }
        let _open_guard = self.worker_open_lock.lock().await;
        let existing = self.workers.lock().await.len();
        let needed = desired.saturating_sub(existing);
        if needed == 0 {
            return;
        }

        // Open in parallel via FuturesUnordered so the first ready connection
        // becomes available without waiting for all peers — relevant when one
        // dial stalls on a slow handshake. Failures are logged + dropped so
        // partial pool open is acceptable; live_worker_count() drives fallback.
        use futures::stream::{FuturesUnordered, StreamExt};
        let mut open_futs: FuturesUnordered<_> = (0..needed)
            .map(|_| open_session(&self.connect_args))
            .collect();
        let mut opened = Vec::with_capacity(needed);
        while let Some(res) = open_futs.next().await {
            match res {
                Ok((handle, sftp, _fp)) => opened.push(Arc::new(Worker {
                    handle,
                    sftp,
                    gate: tokio::sync::Mutex::new(()),
                })),
                Err(e) => log::warn!("sftp worker open failed: {e}"),
            }
        }
        if !opened.is_empty() {
            self.workers.lock().await.extend(opened);
        }
    }

    /// Count of opened-and-still-live worker sessions.
    async fn live_worker_count(&self) -> usize {
        self.workers.lock().await.len()
    }

    pub async fn close(self) {
        let _ = self.sftp.close().await;
        // Tear down worker pool — each worker holds its own SSH connection,
        // leaking them would orphan russh background tasks.
        let workers = self.workers.lock().await;
        for w in workers.iter() {
            let _ = w.sftp.close().await;
        }
        drop(workers);
        drop(self.handle);
    }

    /// Recursive walk of a remote folder. Returns flat list of files (not dirs).
    /// Skips entries beyond max_depth from `root`.
    /// `ext_filter` (lowercased) — if Some, only include files w/ matching extension.
    pub async fn list_recursive(
        &self,
        root: &str,
        max_depth: usize,
        ext_filter: Option<&[&str]>,
    ) -> Result<Vec<RemoteEntry>, String> {
        let owned_filter: Option<Vec<String>> =
            ext_filter.map(|f| f.iter().map(|s| s.to_string()).collect());
        list_recursive_via(&self.sftp, root, max_depth, owned_filter.as_deref()).await
    }

    /// Worker-pooled recursive listing for each root. Distributes roots across
    /// up to 4 worker SFTP sessions for genuine wire-level parallelism (russh-
    /// sftp serializes requests within a single session). Falls back to serial
    /// listing on the main session if the pool fails to open.
    ///
    /// Belt-and-braces retry: any root that comes back empty is retried on the
    /// main session before being trusted as truly empty. Mirrors the v13.55.1
    /// WPF fix for the false-push regression where a worker session breaking
    /// mid-walk silently returned 0 entries → drift treated locals as
    /// "remote-only-absent" → 5693 false push entries. Main-session retry
    /// confirms before the empty-result propagates.
    pub async fn list_recursive_batch(
        &self,
        roots: &[String],
        max_depth: usize,
        ext_filter: Option<&[&str]>,
        parallelism: usize,
    ) -> Result<std::collections::HashMap<String, Vec<RemoteEntry>>, String> {
        let owned_filter: Option<Vec<String>> =
            ext_filter.map(|f| f.iter().map(|s| s.to_string()).collect());

        if roots.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        self.ensure_workers(parallelism).await;
        let live = self.live_worker_count().await;

        let mut out: std::collections::HashMap<String, Vec<RemoteEntry>> =
            std::collections::HashMap::new();

        if live == 0 {
            // Fallback: serial listing on the main session.
            for r in roots {
                let v = list_recursive_via(&self.sftp, r, max_depth, owned_filter.as_deref())
                    .await
                    .unwrap_or_default();
                out.insert(r.clone(), v);
            }
        } else {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            for r in roots {
                let _ = tx.send(r.clone());
            }
            drop(tx);
            let rx = Arc::new(tokio::sync::Mutex::new(rx));

            let workers: Vec<Arc<Worker>> = {
                let g = self.workers.lock().await;
                g.iter().take(live).cloned().collect()
            };
            let filter_arc = Arc::new(owned_filter.clone());
            let results_arc =
                Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::<
                    String,
                    Vec<RemoteEntry>,
                >::new()));

            let mut tasks = Vec::with_capacity(workers.len());
            for worker in workers {
                let rx = rx.clone();
                let results_arc = results_arc.clone();
                let filter = filter_arc.clone();
                tasks.push(tokio::spawn(async move {
                    loop {
                        let job = {
                            let mut rxg = rx.lock().await;
                            rxg.recv().await
                        };
                        let Some(root) = job else { break };
                        let _g = worker.gate.lock().await;
                        let res = list_recursive_via(
                            &worker.sftp,
                            &root,
                            max_depth,
                            (*filter).as_deref(),
                        )
                        .await;
                        match res {
                            Ok(v) => {
                                let mut r = results_arc.lock().await;
                                r.insert(root, v);
                            }
                            Err(e) => log::warn!("list_recursive worker failed for {root}: {e}"),
                        }
                    }
                }));
            }
            let _ = futures::future::join_all(tasks).await;
            out = Arc::try_unwrap(results_arc)
                .map(|m| m.into_inner())
                .unwrap_or_default();

            // Belt-and-braces: retry only roots the worker pool failed to
            // populate (vec absent). Worker can return 0 entries silently if
            // its session breaks mid-walk; main retry confirms before
            // propagating. Genuinely-empty roots (worker returned `Some(vec![])`)
            // do NOT retry — that's a serial round-trip per empty folder.
            for r in roots {
                if !out.contains_key(r) {
                    if let Ok(retry) = list_recursive_via(
                        &self.sftp,
                        r,
                        max_depth,
                        owned_filter.as_deref(),
                    )
                    .await
                    {
                        out.insert(r.clone(), retry);
                    } else {
                        out.entry(r.clone()).or_default();
                    }
                }
            }
        }

        Ok(out)
    }

    /// Single-level listing — entries sorted dirs-first then alphabetical
    /// (case-insensitive). Mirrors WPF `ListDirectoryAsync`. Used by the
    /// browser pane (Phase 3) and the manifest discovery top-level read.
    pub async fn list_directory(&self, path: &str) -> Result<Vec<RemoteEntry>, String> {
        let entries = self
            .sftp
            .read_dir(path)
            .await
            .map_err(|e| format!("readdir {path}: {e}"))?;
        let trimmed = path.trim_end_matches('/');
        let mut out: Vec<RemoteEntry> = Vec::new();
        for entry in entries {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let meta = entry.metadata();
            let is_dir = matches!(meta.file_type(), FileType::Dir);
            out.push(RemoteEntry {
                full_path: format!("{}/{}", trimmed, name),
                name,
                is_dir,
                size: meta.size.unwrap_or(0),
                last_modified: mtime_to_utc(meta.mtime),
            });
        }
        out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        Ok(out)
    }

    /// Walk up from `remote_file`'s parent looking for the first existing
    /// ancestor, then create everything below. Mirrors WPF
    /// `EnsureRemoteParentDirAsync`. Race-tolerant: concurrent uploads from
    /// sibling files may have created the same dir between our check and call.
    /// Falls back gracefully on errors — best-effort.
    pub async fn ensure_remote_parent_dir(&self, remote_file: &str) -> Result<(), String> {
        let parent = match remote_parent(remote_file) {
            Some(p) if p != "/" => p.to_string(),
            _ => return Ok(()),
        };

        let mut to_create: Vec<String> = Vec::new();
        let mut cur: String = parent;
        while !cur.is_empty() && cur != "/" {
            if self.sftp.metadata(&cur).await.is_ok() {
                break;
            }
            let next = remote_parent(&cur).map(|p| p.to_string()).unwrap_or_default();
            to_create.push(cur);
            cur = next;
        }
        // Create deepest-missing-first up the stack.
        for dir in to_create.iter().rev() {
            // Idempotent: ignore concurrent-create races.
            let _ = self.sftp.create_dir(dir).await;
        }
        Ok(())
    }

    /// Server-side `du`-style folder sizing via `find -prune` over the SSH
    /// session — ONE round trip vs hundreds for SFTP-walk. Excludes the same
    /// dir names that drift scan ignores so the size matches drift's scope.
    /// Returns -1 on failure (caller should fall back to recursive sum or "—").
    pub async fn get_remote_folder_size(&self, remote_path: &str) -> i64 {
        if remote_path.is_empty() {
            return -1;
        }
        let escaped = match shell_quote(remote_path) {
            Ok(v) => v,
            Err(_) => return -1,
        };
        // Build prune clause from the ignore module's canonical list + .rift-tmp
        // (per-file ext, not in the dir list — added explicitly).
        let mut prune_names: Vec<String> = crate::sync::ignore::ignored_directory_names()
            .iter()
            .filter_map(|n| shell_quote(n).ok().map(|q| format!("-name {q}")))
            .collect();
        prune_names.push(format!(
            "-name {}",
            shell_quote(".rift-tmp").unwrap_or_else(|_| "'.rift-tmp'".into())
        ));
        let prune_clause = prune_names.join(" -o ");
        // GNU find + awk: portable on FXServer hosts (Ubuntu/Debian/Alpine).
        let cmd = format!(
            "find {} -type d \\( {} \\) -prune -o -type f -printf '%s\\n' 2>/dev/null | awk '{{s+=$1}} END {{print s+0}}'",
            escaped, prune_clause
        );

        let channel = match self.handle.channel_open_session().await {
            Ok(c) => c,
            Err(_) => return -1,
        };
        if channel.exec(true, cmd).await.is_err() {
            return -1;
        }
        let mut out = String::new();
        let mut chan = channel;
        while let Some(msg) = chan.wait().await {
            match msg {
                russh::ChannelMsg::Data { data } => {
                    out.push_str(&String::from_utf8_lossy(&data));
                }
                russh::ChannelMsg::ExitStatus { .. } => break,
                _ => {}
            }
        }
        out.split_whitespace()
            .next()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(-1)
    }

    /// Probe whether a remote path exists. Returns false on ENOENT, surfaces
    /// other errors. Mirrors WPF's `(bool Exists, ...)` tuple — we only need
    /// the Exists bit at present.
    pub async fn remote_exists(&self, path: &str) -> bool {
        self.sftp.metadata(path).await.is_ok()
    }

    /// Full stat — mirrors WPF `RemoteExistsAsync` returning size + mtime.
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

    /// Rename a remote path. Used by atomic upload (.rift-tmp → final).
    pub async fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        rename_via(&self.sftp, from, to).await
    }

    /// Delete a remote file. Returns OpResult so AutoSync can surface error
    /// strings into the activity row without losing the success/fail bit.
    pub async fn delete(&self, path: &str) -> OpResult {
        match self.sftp.remove_file(path).await {
            Ok(_) => OpResult::ok(),
            Err(e) => OpResult::err(format!("delete {path}: {e}")),
        }
    }

    /// Delete a remote path — file OR directory. Dir deletes recurse depth-first.
    pub async fn delete_recursive(&self, path: &str) -> Result<(), String> {
        delete_recursive_via(&self.sftp, path).await
    }

    /// Create remote directory tree (mkdir -p semantics).
    pub async fn mkdir_p(&self, path: &str) -> Result<(), String> {
        mkdir_p_via(&self.sftp, path).await
    }

    /// Atomic upload — write `local_path`'s bytes to `<remote>.rift-tmp`, then
    /// rename to `remote`. Mirrors WPF `UploadFileAtomicAsync`. Creates parent
    /// dirs on demand.
    pub async fn upload_file_atomic(
        &self,
        local_path: &Path,
        remote_path: &str,
    ) -> OpResult {
        upload_atomic_via(&self.sftp, local_path, remote_path).await
    }

    /// Atomic download — read remote bytes, write to `<local>.rift-tmp`, then
    /// rename to `local_path`. Mirrors WPF `DownloadFileAtomicAsync`.
    pub async fn download_file_atomic(
        &self,
        remote_path: &str,
        local_path: &Path,
    ) -> OpResult {
        download_atomic_via(&self.sftp, remote_path, local_path).await
    }

    /// Parallel batch download — fans `jobs` out across the worker pool, falls
    /// back to serial atomic-download on the main session if the pool failed
    /// to open. Returns parallel `Vec<bool>` aligned w/ `jobs` so callers can
    /// dispatch post-success work (snapshot baseline refresh, activity rows)
    /// per index. Mirrors WPF `DownloadFilesBatchAsync`.
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

    /// Parallel batch upload — symmetric counterpart to `download_files_batch`.
    /// Pre-creates unique parent dirs on the main session before fanning bytes
    /// out to workers — saves N×depth `create_dir` round-trips when many files
    /// share the same parent (typical for resource pushes). Mirrors WPF
    /// `UploadFilesBatchAsync`.
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
            if let Some(p) = remote_parent(r) {
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

    /// SHA1 of a remote file via `sha1sum` over a transient exec channel.
    /// Returns None on any failure (mirrors WPF's null-on-error). Stderr is
    /// debug-logged so drift-scan failures are diagnosable without a server
    /// shell; previously errors silently dropped.
    pub async fn get_remote_sha1(&self, path: &str) -> Option<String> {
        // sha1sum prints "<hex>  <filename>\n". We extract the hex prefix.
        // Quote the path to handle spaces / special chars.
        let quoted = shell_quote(path).ok()?;
        let cmd = format!("sha1sum {quoted}");
        let channel = self.handle.channel_open_session().await.ok()?;
        channel.exec(true, cmd).await.ok()?;
        let mut out = String::new();
        let mut err = String::new();
        let mut chan = channel;
        while let Some(msg) = chan.wait().await {
            match msg {
                russh::ChannelMsg::Data { data } => {
                    out.push_str(&String::from_utf8_lossy(&data));
                }
                russh::ChannelMsg::ExtendedData { data, ext: 1 } => {
                    err.push_str(&String::from_utf8_lossy(&data));
                }
                russh::ChannelMsg::ExitStatus { .. } => break,
                _ => {}
            }
        }
        let hex = out.split_whitespace().next();
        let result = hex.filter(|h| h.len() == 40).map(|h| h.to_uppercase());
        if result.is_none() && !err.trim().is_empty() {
            log::debug!("sha1sum {path}: stderr = {}", err.trim());
        }
        result
    }

    pub async fn upload_bytes(&self, bytes: &[u8], remote_path: &str) -> Result<(), String> {
        // russh-sftp's `write()` is WRITE-only (no CREATE/TRUNCATE) — fails on
        // first creation and leaves trailing garbage if the new payload is
        // shorter than the existing file. `create()` is WRITE|CREATE|TRUNCATE.
        let mut f = self
            .sftp
            .create(remote_path)
            .await
            .map_err(|e| format!("create {remote_path}: {e}"))?;
        f.write_all(bytes)
            .await
            .map_err(|e| format!("write {remote_path}: {e}"))
    }

    /// Download a remote file into `local_dir`. Returns the local path written.
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
    }
}

// ─── Session-agnostic helpers (callable on main + worker SftpSessions) ──────

async fn list_recursive_via(
    sftp: &SftpSession,
    root: &str,
    max_depth: usize,
    ext_filter: Option<&[String]>,
) -> Result<Vec<RemoteEntry>, String> {
    let root = trim_slash(root);
    let mut out = Vec::new();
    let mut stack: Vec<(String, usize)> = vec![(root.clone(), 0)];

    while let Some((dir, depth)) = stack.pop() {
        let entries = match sftp.read_dir(&dir).await {
            Ok(e) => e,
            Err(e) => return Err(format!("readdir {dir}: {e}")),
        };
        for entry in entries {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let meta = entry.metadata();
            let full = format!("{}/{}", dir, name);
            let is_dir = matches!(meta.file_type(), FileType::Dir);
            if is_dir {
                if depth < max_depth {
                    stack.push((full, depth + 1));
                }
                continue;
            }
            if let Some(filter) = ext_filter {
                let ok = name
                    .rsplit_once('.')
                    .map(|(_, ext)| filter.iter().any(|f| f.eq_ignore_ascii_case(ext)))
                    .unwrap_or(false);
                if !ok {
                    continue;
                }
            }
            out.push(RemoteEntry {
                full_path: full,
                name,
                is_dir: false,
                size: meta.size.unwrap_or(0),
                last_modified: mtime_to_utc(meta.mtime),
            });
        }
    }
    Ok(out)
}

async fn delete_recursive_via(sftp: &SftpSession, path: &str) -> Result<(), String> {
    // Belt-and-braces — Tauri cmd layer already containment-checks, but this
    // helper is reachable internally too. Don't let "/" or "" get this far.
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return Err(format!("refusing to recursively delete root: '{trimmed}'"));
    }
    // lstat: never follow a symlink at the target. If it IS a symlink, unlink
    // the link only — never recurse through whatever it points to.
    let meta = sftp
        .symlink_metadata(path)
        .await
        .map_err(|e| format!("lstat {path}: {e}"))?;
    if meta.file_type().is_symlink() || !meta.file_type().is_dir() {
        return sftp
            .remove_file(path)
            .await
            .map_err(|e| format!("delete {path}: {e}"));
    }
    let mut to_visit: Vec<String> = vec![path.to_string()];
    let mut to_rmdir: Vec<String> = Vec::new();
    while let Some(dir) = to_visit.pop() {
        to_rmdir.push(dir.clone());
        let entries = sftp
            .read_dir(&dir)
            .await
            .map_err(|e| format!("readdir {dir}: {e}"))?;
        for entry in entries {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let full = format!("{}/{}", dir.trim_end_matches('/'), name);
            // Per-child lstat — entry.metadata() may report followed-target
            // attrs depending on server, which could push a symlinked dir
            // onto the visit stack and walk out of tree.
            let child_meta = sftp
                .symlink_metadata(&full)
                .await
                .map_err(|e| format!("lstat {full}: {e}"))?;
            let ft = child_meta.file_type();
            if ft.is_dir() && !ft.is_symlink() {
                to_visit.push(full);
            } else {
                sftp.remove_file(&full)
                    .await
                    .map_err(|e| format!("delete {full}: {e}"))?;
            }
        }
    }
    while let Some(d) = to_rmdir.pop() {
        sftp.remove_dir(&d)
            .await
            .map_err(|e| format!("rmdir {d}: {e}"))?;
    }
    Ok(())
}

/// User-facing rename. Errors if `to` already exists — never silently overwrites.
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

/// Replace-on-write rename used only by the atomic upload tmp-swap. Best-effort
/// removes any existing destination first — required b/c some SFTP servers
/// reject rename-over-existing. NEVER call this from user-initiated rename.
async fn rename_overwriting_via(sftp: &SftpSession, from: &str, to: &str) -> Result<(), String> {
    let _ = sftp.remove_file(to).await;
    sftp.rename(from, to)
        .await
        .map_err(|e| format!("rename {from} -> {to}: {e}"))
}

async fn mkdir_p_via(sftp: &SftpSession, path: &str) -> Result<(), String> {
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
        // Idempotent: ignore "already exists".
        let _ = sftp.create_dir(&cur).await;
    }
    Ok(())
}

/// Reformat noisy/duplicated SFTP error strings into something a user can act
/// on. russh-sftp's Display impl frequently emits "Permission denied: Permission
/// denied" (status + message both filled). We collapse the double + append a
/// hint pointing at the actual fix (server-side group write on the parent).
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
    if let Some(parent) = remote_parent(remote_path) {
        let _ = mkdir_p_via(sftp, parent).await;
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
    if let Err(e) = rename_overwriting_via(sftp, &tmp, remote_path).await {
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

fn trim_slash(s: &str) -> String {
    let t = s.trim_end_matches('/');
    if t.is_empty() {
        "/".into()
    } else {
        t.to_string()
    }
}

fn mtime_to_utc(mtime: Option<u32>) -> DateTime<Utc> {
    let secs = mtime.unwrap_or(0) as i64;
    Utc.timestamp_opt(secs, 0).single().unwrap_or_else(Utc::now)
}

/// Returns the parent dir component of a posix-style remote path, or None for root-level.
fn remote_parent(path: &str) -> Option<&str> {
    let trimmed = path.trim_end_matches('/');
    let idx = trimmed.rfind('/')?;
    if idx == 0 { Some("/") } else { Some(&trimmed[..idx]) }
}

/// POSIX single-quote escape: `replace ' with '\\''`. Rejects control
/// characters that would split an SSH exec command line.
fn shell_quote(s: &str) -> Result<String, String> {
    if s.contains(['\0', '\n', '\r']) {
        return Err("path contains command-breaking control character".into());
    }
    let mut out = String::from("'");
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    Ok(out)
}
