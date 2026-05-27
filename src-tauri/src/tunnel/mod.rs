// Phase 1g: SSH local-port-forward (`direct-tcpip`).
//
// Replacement for WPF `Services/Transport/SshTunnel.cs` (398L). The WPF version
// shells out to Win11's bundled `ssh.exe -N -L`; we open a russh `direct-tcpip`
// channel per inbound TCP connection ourselves so there's no external process,
// no JobObject orphan-recovery dance, no OpenSSH dependency.
//
// Lifecycle:
//   - `SshTunnel::start(args)` opens an SSH session (fingerprint-pinned via the
//     shared `transport::ssh_handler::PinningHandler`, same as the SFTP client),
//     binds a tokio TcpListener on 127.0.0.1:0, captures the kernel-assigned
//     port into `local_port`, and spawns an accept loop that fans each inbound
//     conn into a per-conn task.
//   - Each per-conn task opens a `channel_open_direct_tcpip(remote_host,
//     remote_port, "127.0.0.1", local_port)` channel and `copy_bidirectional`s
//     bytes between the TCP socket and the channel stream until either side EOFs.
//   - `stop(self)` drops the oneshot sender → accept loop's `select!` picks the
//     stop branch, breaks out, drops the listener + the russh `Handle` (the SSH
//     session tears down). In-flight conn tasks finish on their own (the channel
//     closes when the Handle drops, surfacing as EOF on the per-conn copy).

use russh::client;
use russh::keys::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::transport::ssh_handler::PinningHandler;

pub struct TunnelArgs<'a> {
    pub host: &'a str,
    pub port: u16,
    pub user: &'a str,
    pub key_path: &'a Path,
    /// Substring-matched against `SHA256:<b64>`. Same semantics as
    /// `sftp::ConnectArgs::trusted_fingerprint` — accepts WPF/WinSCP's
    /// `ssh-ed25519 256 SHA256:<b64>` form transparently.
    pub trusted_fingerprint: Option<&'a str>,
    /// Remote endpoint to forward to. Typically `"127.0.0.1"`.
    pub remote_host: &'a str,
    pub remote_port: u16,
}

/// Live SSH local-port-forward. Drop or call `.stop()` to tear down.
pub struct SshTunnel {
    pub local_port: u16,
    stop_tx: Option<oneshot::Sender<()>>,
    accept_task: Option<JoinHandle<()>>,
    /// Cancels in-flight per-conn `copy_bidirectional` tasks on stop. Without
    /// this, a stalled copy holds an `Arc<Handle>` clone indefinitely and the
    /// russh session can't tear down (audit scan-transport-2026-05-11.md).
    conn_cancel: CancellationToken,
}

impl SshTunnel {
    pub async fn start(args: TunnelArgs<'_>) -> Result<Self, String> {
        // ── SSH session open ────────────────────────────────────────────────
        let key_path: PathBuf = args.key_path.to_path_buf();
        let key_pair = load_secret_key(&key_path, None)
            .map_err(|e| format!("load key {}: {e}", key_path.display()))?;
        // Keepalive parity w/ sftp::open_session — 20s x 3 = ~60s to detect
        // a stalled server side instead of indefinite half-dead socket.
        // Window/packet parity w/ sftp::open_session — see that comment for
        // the v0.2.45 truncation post-mortem.
        let config = Arc::new(client::Config {
            keepalive_interval: Some(std::time::Duration::from_secs(20)),
            keepalive_max: 3,
            window_size: 2 * 1024 * 1024,
            maximum_packet_size: 32 * 1024,
            preferred: crate::sftp::rift_preferred(),
            ..client::Config::default()
        });
        let addr = format!("{}:{}", args.host, args.port);
        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let handler = PinningHandler {
            trusted: args.trusted_fingerprint.map(|s| s.to_string()),
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
                args.user.to_string(),
                russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key_pair), hash),
            )
            .await
            .map_err(|e| format!("auth: {e}"))?;
        if !auth.success() {
            return Err(format!("auth rejected for {}@{}", args.user, args.host));
        }

        // ── Local listener ──────────────────────────────────────────────────
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("bind 127.0.0.1:0: {e}"))?;
        let local_addr = listener
            .local_addr()
            .map_err(|e| format!("local_addr: {e}"))?;
        let local_port = local_addr.port();

        // ── Accept loop ─────────────────────────────────────────────────────
        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
        let handle_arc = Arc::new(handle);
        let remote_host = args.remote_host.to_string();
        let remote_port = args.remote_port as u32;
        let conn_cancel = CancellationToken::new();
        let conn_cancel_for_loop = conn_cancel.clone();

        let accept_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        log::debug!("ssh tunnel: stop signal received, exiting accept loop");
                        break;
                    }
                    accepted = listener.accept() => {
                        let (mut tcp, peer) = match accepted {
                            Ok(p) => p,
                            Err(e) => {
                                log::warn!("ssh tunnel: accept error: {e}");
                                continue;
                            }
                        };
                        let handle = handle_arc.clone();
                        let remote_host = remote_host.clone();
                        let conn_cancel = conn_cancel_for_loop.clone();
                        tokio::spawn(async move {
                            let channel = match handle
                                .channel_open_direct_tcpip(
                                    remote_host.as_str(),
                                    remote_port,
                                    "127.0.0.1",
                                    local_port as u32,
                                )
                                .await
                            {
                                Ok(c) => c,
                                Err(e) => {
                                    log::warn!(
                                        "ssh tunnel: open direct-tcpip from {peer}: {e}"
                                    );
                                    return;
                                }
                            };
                            let mut stream = channel.into_stream();
                            tokio::select! {
                                _ = conn_cancel.cancelled() => {
                                    log::debug!("ssh tunnel: conn for {peer} cancelled on stop");
                                }
                                r = tokio::io::copy_bidirectional(&mut tcp, &mut stream) => {
                                    if let Err(e) = r {
                                        log::debug!("ssh tunnel: copy ended for {peer}: {e}");
                                    }
                                }
                            }
                        });
                    }
                }
            }
            // Drop listener + handle_arc; when the last clone of handle_arc drops
            // (after in-flight per-conn tasks finish), the russh session tears down.
            drop(listener);
            drop(handle_arc);
        });

        Ok(Self {
            local_port,
            stop_tx: Some(stop_tx),
            accept_task: Some(accept_task),
            conn_cancel,
        })
    }

    /// Stop accepting + tear down the SSH session. Cancels in-flight per-conn
    /// `copy_bidirectional` tasks so a stalled copy can't pin the russh Handle.
    pub async fn stop(mut self) {
        self.conn_cancel.cancel();
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.accept_task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        // Best-effort cleanup if the caller forgot `.stop().await`.
        self.conn_cancel.cancel();
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.accept_task.take() {
            task.abort();
        }
    }
}
