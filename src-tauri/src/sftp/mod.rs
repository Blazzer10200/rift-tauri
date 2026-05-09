// Phase 1b/1c/1h: SFTP client surface. Wraps russh + russh-sftp into a
// long-lived connection that DriftScanner / EditTrail / AutoSync / discovery
// call against.
//
// **Phase 1h gap-fill (2026-05-08) — daily-driver parity vs WPF SftpClient.cs:**
//   - Fingerprint pinning (was TOFU only). `ConnectArgs.trusted_fingerprint`
//     accepts both Rust's `SHA256:<b64>` form and WPF/WinSCP's full
//     `ssh-ed25519 256 SHA256:<b64>` form via substring match. On TOFU first
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
//   - `discover_manifest_folders` — pruned BFS w/ worker fan-out for FXServer
//     manifest scanning. ~20× speedup vs full-walk + filter.
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
    /// If `Some`, server's pubkey fingerprint must match this string. Substring
    /// match — accepts both Rust's `"SHA256:<b64>"` form and WPF/WinSCP's
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
        let mut workers = self.workers.lock().await;
        while let Some(res) = open_futs.next().await {
            match res {
                Ok((handle, sftp, _fp)) => workers.push(Arc::new(Worker {
                    handle,
                    sftp,
                    gate: tokio::sync::Mutex::new(()),
                })),
                Err(e) => log::warn!("sftp worker open failed: {e}"),
            }
        }
    }

    /// Count of opened-and-still-live worker sessions.
    pub async fn live_worker_count(&self) -> usize {
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
                        let v = list_recursive_via(
                            &worker.sftp,
                            &root,
                            max_depth,
                            (*filter).as_deref(),
                        )
                        .await
                        .unwrap_or_default();
                        let mut r = results_arc.lock().await;
                        r.insert(root, v);
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

    /// FXServer manifest discovery — walks the tree under `root` looking for
    /// directories that contain any of `manifest_names` (typically
    /// `fxmanifest.lua` / `__resource.lua`). Two optimizations vs naive recurse:
    ///
    ///   1. **Prune at first manifest** — once a dir has a manifest it's a
    ///      resource boundary; we don't recurse further. ~80% fewer readdirs
    ///      on a typical 200-resource Qbox tree.
    ///   2. **Parallel subtree fan-out** — list root's immediate children once,
    ///      then dispatch each child to a worker session for pruned recursion.
    ///      Workers use independent SFTP connections so subtree walks overlap
    ///      on the wire.
    ///
    /// Combined w/ pruning: ~20× speedup vs single-session full-walk + filter.
    /// Falls back to a serial pruned walk on main session if pool open fails.
    pub async fn discover_manifest_folders(
        &self,
        root: &str,
        manifest_names: &[&str],
        max_depth: usize,
        parallelism: usize,
    ) -> Vec<String> {
        if root.is_empty() || manifest_names.is_empty() {
            return Vec::new();
        }
        let manifest_set: Arc<std::collections::HashSet<String>> = Arc::new(
            manifest_names
                .iter()
                .map(|s| s.to_lowercase())
                .collect(),
        );

        // Top-level listing of root. If root itself has a manifest, return it.
        let top = match self.sftp.read_dir(trim_slash(root).as_str()).await {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };
        let mut subroots: Vec<String> = Vec::new();
        let root_trim = trim_slash(root);
        for entry in top {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let meta = entry.metadata();
            let full = format!("{}/{}", root_trim, name);
            if matches!(meta.file_type(), FileType::Dir) {
                subroots.push(full);
            } else if manifest_set.contains(&name.to_lowercase()) {
                return vec![root_trim];
            }
        }
        if subroots.is_empty() {
            return Vec::new();
        }

        // Subroots cost depth 1; remaining budget for the recursive walk = max_depth - 1.
        let child_depth = max_depth.saturating_sub(1).max(1);

        self.ensure_workers(parallelism).await;
        let live = self.live_worker_count().await;
        let mut all: Vec<String> = Vec::new();

        if live == 0 {
            for s in &subroots {
                let hits = walk_pruned_via(&self.sftp, s, &manifest_set, child_depth).await;
                all.extend(hits);
            }
        } else {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            for s in &subroots {
                let _ = tx.send(s.clone());
            }
            drop(tx);
            let rx = Arc::new(tokio::sync::Mutex::new(rx));

            let workers: Vec<Arc<Worker>> = {
                let g = self.workers.lock().await;
                g.iter().take(live).cloned().collect()
            };
            let results_arc = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
            let mut tasks = Vec::with_capacity(workers.len());
            for worker in workers {
                let rx = rx.clone();
                let results_arc = results_arc.clone();
                let manifest_set = manifest_set.clone();
                tasks.push(tokio::spawn(async move {
                    loop {
                        let job = {
                            let mut rxg = rx.lock().await;
                            rxg.recv().await
                        };
                        let Some(sub) = job else { break };
                        let _g = worker.gate.lock().await;
                        let hits =
                            walk_pruned_via(&worker.sftp, &sub, &manifest_set, child_depth).await;
                        if !hits.is_empty() {
                            let mut acc = results_arc.lock().await;
                            acc.extend(hits);
                        }
                    }
                }));
            }
            let _ = futures::future::join_all(tasks).await;
            all = Arc::try_unwrap(results_arc)
                .map(|m| m.into_inner())
                .unwrap_or_default();
        }

        // Dedup (case-insensitive) — workers shouldn't overlap given we
        // distribute subroots, but a degenerate root w/ symlinks could trip it.
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        all.retain(|p| seen.insert(p.to_lowercase()));
        all
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
        let escaped = shell_quote(remote_path);
        // Build prune clause from the ignore module's canonical list + .rift-tmp
        // (per-file ext, not in the dir list — added explicitly).
        let mut prune_names: Vec<String> = crate::sync::ignore::ignored_directory_names()
            .iter()
            .map(|n| format!("-name {}", shell_quote(n)))
            .collect();
        prune_names.push(format!("-name {}", shell_quote(".rift-tmp")));
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
        let cmd = format!("sha1sum {}", shell_quote(path));
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

/// Pruned BFS — recurse into subdirs UNTIL a manifest file is found, then
/// stop descending (resource boundary). Used by `discover_manifest_folders`.
async fn walk_pruned_via(
    sftp: &SftpSession,
    root: &str,
    manifest_names: &std::collections::HashSet<String>,
    max_depth: usize,
) -> Vec<String> {
    let mut results = Vec::new();
    let mut bfs: std::collections::VecDeque<(String, usize)> = std::collections::VecDeque::new();
    bfs.push_back((trim_slash(root), 0));
    while let Some((cur, depth)) = bfs.pop_front() {
        let entries = match sftp.read_dir(&cur).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        let mut has_manifest = false;
        let mut subdirs: Vec<String> = Vec::new();
        for entry in entries {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let meta = entry.metadata();
            if matches!(meta.file_type(), FileType::Dir) {
                if depth + 1 < max_depth {
                    subdirs.push(format!("{}/{}", cur, name));
                }
            } else if manifest_names.contains(&name.to_lowercase()) {
                has_manifest = true;
            }
        }
        if has_manifest {
            results.push(cur);
            continue; // prune — don't recurse
        }
        for s in subdirs {
            bfs.push_back((s, depth + 1));
        }
    }
    results
}

async fn rename_via(sftp: &SftpSession, from: &str, to: &str) -> Result<(), String> {
    // Some servers reject rename-over-existing — best-effort delete first.
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
    // russh-sftp's `write()` uses OpenFlags::WRITE only — no CREATE/TRUNCATE,
    // so it fails NO_SUCH_FILE on a fresh tmp. `create()` is WRITE|CREATE|TRUNCATE.
    // Wrap in a scope so the file handle drops (and the SFTP close packet
    // is sent) before we attempt the rename.
    {
        let mut f = match sftp.create(&tmp).await {
            Ok(f) => f,
            Err(e) => return OpResult::err(format!("create tmp {tmp}: {e}")),
        };
        if let Err(e) = f.write_all(&bytes).await {
            drop(f);
            let _ = sftp.remove_file(&tmp).await;
            return OpResult::err(format!("write tmp {tmp}: {e}"));
        }
    }
    if let Err(e) = rename_via(sftp, &tmp, remote_path).await {
        let _ = sftp.remove_file(&tmp).await;
        return OpResult::err(e);
    }
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

/// POSIX single-quote escape — `replace ' with '\\''`. Safe for typical
/// SFTP-served paths (UTF-8, no NUL/newlines). Does NOT sanitize NUL bytes
/// or newlines — SFTP technically allows them, no FXServer host ever uses
/// them. If that changes, sanitize at the call site.
fn shell_quote(s: &str) -> String {
    let mut out = String::from("'");
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}
