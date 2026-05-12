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
//   - `list_directory` (single-level, sorted dirs-first).
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
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::transport::ssh_handler::PinningHandler;

mod list;
mod ops;
mod remote_exec;
mod transfer;

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
    pub write_probe_root: Option<&'a str>,
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
    // Keepalive + inactivity timeout — without these russh sits on a half-dead
    // TCP socket indefinitely (Windows OS-level keepalive is ~2hr). 20s ping
    // x 3 unanswered = ~60s to detect a stalled server, then session closes
    // cleanly w/ an error instead of hanging the push or remote-list panel.
    // window_size + maximum_packet_size bumped from russh defaults (32 KiB)
    // to OpenSSH-equivalent 2 MiB / 32 KiB. Default channel-window pressure
    // caused trailing Data chunks on `find` exec listings to back up against
    // the channel buffer, which the v0.2.45 drain fix already addresses for
    // exec, but the same pressure can truncate SFTP `read_dir` responses on
    // the worker fallback path — bigger window = no truncation under load.
    let config = Arc::new(client::Config {
        keepalive_interval: Some(std::time::Duration::from_secs(20)),
        keepalive_max: 3,
        window_size: 2 * 1024 * 1024,
        maximum_packet_size: 32 * 1024,
        ..client::Config::default()
    });
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
        let client = Self {
            handle,
            sftp,
            fingerprint,
            connect_args: captured_args,
            workers: tokio::sync::Mutex::new(Vec::new()),
            worker_open_lock: tokio::sync::Mutex::new(()),
        };
        if let Some(root) = args.write_probe_root {
            client.probe_write_access(root).await?;
        }
        Ok(client)
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
}

// ─── Session-agnostic helpers (callable on main + worker SftpSessions) ──────

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

