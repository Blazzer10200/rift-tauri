//! Sync Inspector — observability surface for the autosync pipeline.
//!
//! Provides a process-global event bus that any subsystem (watcher, debounce,
//! sftp pool, drift, bridge) can publish into without dep-injection. The Tauri
//! command `diag_subscribe` forwards bus events to the frontend over the
//! `diag://event` channel; pipeline state snapshots are emitted on
//! `diag://state` every 500ms.
//!
//! Design notes:
//! * `DiagBus` wraps `tokio::sync::broadcast` (cap 4096, drop-oldest on lag).
//!   Hot-path callers fire-and-forget via `publish()` — never blocks the
//!   producer.
//! * `LogForwarder` impls `log::Log` and chains every existing
//!   `log::info!/warn!/error!` call into the bus AS WELL AS env_logger, so
//!   instrumentation we already have shows up for free without rewriting call
//!   sites.
//! * Frontend rate-limits emits at 200/sec to avoid overwhelming Svelte
//!   reactivity on a webpack-rebuild-style burst.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

const BUS_CAPACITY: usize = 4096;
const FRONTEND_RATE_PER_SEC: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagStage {
    FsEvent,
    Ignored,
    Debounced,
    Queued,
    QueueDropped,
    UploadStart,
    UploadDone,
    UploadFail,
    AtomicRename,
    LockAcquired,
    LockReleased,
    LockHeldByOther,
    DriftScanStart,
    DriftScanResult,
    BridgePing,
    BridgeAck,
    RescanSignal,
    SftpConnect,
    SftpDisconnect,
    RemoteScanStart,
    RemoteScanResult,
    RemotePullStart,
    RemotePullDone,
    RemotePullFail,
    Log,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<log::Level> for DiagLevel {
    fn from(l: log::Level) -> Self {
        match l {
            log::Level::Error => DiagLevel::Error,
            log::Level::Warn => DiagLevel::Warn,
            log::Level::Info => DiagLevel::Info,
            log::Level::Debug => DiagLevel::Debug,
            log::Level::Trace => DiagLevel::Trace,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagEvent {
    pub at: DateTime<Utc>,
    pub seq: u64,
    pub stage: DiagStage,
    pub level: DiagLevel,
    pub resource: Option<String>,
    pub file: Option<String>,
    pub message: String,
    pub fields: serde_json::Value,
}

/// Snapshot of pipeline state, emitted periodically. Read-mostly aggregate —
/// individual events still flow through the bus.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagState {
    pub at: Option<DateTime<Utc>>,
    pub watcher_count: usize,
    pub queue_pending: usize,
    pub queue_failed: usize,
    pub queue_dropped_total: u64,
    pub ignored_total: u64,
    pub conflicts: usize,
    pub locks_count: usize,
    pub last_drift_scan_at: Option<DateTime<Utc>>,
    pub last_rescan_signal_at: Option<DateTime<Utc>>,
    pub bus_lag_total: u64,
    pub events_emitted_total: u64,
}

pub struct DiagBus {
    tx: broadcast::Sender<DiagEvent>,
    seq: AtomicU64,
    queue_dropped_total: AtomicU64,
    last_rescan_signal_at: std::sync::Mutex<Option<DateTime<Utc>>>,
    last_drift_scan_at: std::sync::Mutex<Option<DateTime<Utc>>>,
    bus_lag_total: AtomicU64,
    events_emitted_total: AtomicU64,
    enabled: AtomicBool,
}

impl DiagBus {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(BUS_CAPACITY);
        Self {
            tx,
            seq: AtomicU64::new(0),
            queue_dropped_total: AtomicU64::new(0),
            last_rescan_signal_at: std::sync::Mutex::new(None),
            last_drift_scan_at: std::sync::Mutex::new(None),
            bus_lag_total: AtomicU64::new(0),
            events_emitted_total: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DiagEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, mut event: DiagEvent) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        event.seq = self.seq.fetch_add(1, Ordering::Relaxed);
        self.events_emitted_total.fetch_add(1, Ordering::Relaxed);
        match event.stage {
            DiagStage::QueueDropped => {
                self.queue_dropped_total.fetch_add(1, Ordering::Relaxed);
            }
            DiagStage::RescanSignal => {
                if let Ok(mut g) = self.last_rescan_signal_at.lock() {
                    *g = Some(event.at);
                }
            }
            DiagStage::DriftScanStart | DiagStage::DriftScanResult => {
                if let Ok(mut g) = self.last_drift_scan_at.lock() {
                    *g = Some(event.at);
                }
            }
            _ => {}
        }
        // send() returns Err only when there are zero subscribers — that's
        // fine, we drop on the floor. Lag is reported through Receiver::recv
        // returning RecvError::Lagged on the consumer side.
        let _ = self.tx.send(event);
    }

    pub fn record_bus_lag(&self, n: u64) {
        self.bus_lag_total.fetch_add(n, Ordering::Relaxed);
    }

    pub fn queue_dropped_total(&self) -> u64 {
        self.queue_dropped_total.load(Ordering::Relaxed)
    }

    pub fn last_rescan_signal_at(&self) -> Option<DateTime<Utc>> {
        self.last_rescan_signal_at.lock().ok().and_then(|g| *g)
    }

    pub fn last_drift_scan_at(&self) -> Option<DateTime<Utc>> {
        self.last_drift_scan_at.lock().ok().and_then(|g| *g)
    }

    pub fn bus_lag_total(&self) -> u64 {
        self.bus_lag_total.load(Ordering::Relaxed)
    }

    pub fn events_emitted_total(&self) -> u64 {
        self.events_emitted_total.load(Ordering::Relaxed)
    }
}

static BUS: OnceLock<DiagBus> = OnceLock::new();

pub fn bus() -> &'static DiagBus {
    BUS.get_or_init(DiagBus::new)
}

// ─── Convenience emit helpers ───────────────────────────────────────────────

pub fn emit(stage: DiagStage, level: DiagLevel, message: impl Into<String>) {
    bus().publish(DiagEvent {
        at: Utc::now(),
        seq: 0,
        stage,
        level,
        resource: None,
        file: None,
        message: message.into(),
        fields: serde_json::Value::Null,
    });
}

pub fn emit_for(
    stage: DiagStage,
    level: DiagLevel,
    resource: Option<&str>,
    file: Option<&str>,
    message: impl Into<String>,
) {
    bus().publish(DiagEvent {
        at: Utc::now(),
        seq: 0,
        stage,
        level,
        resource: resource.map(|s| s.to_string()),
        file: file.map(|s| s.to_string()),
        message: message.into(),
        fields: serde_json::Value::Null,
    });
}

pub fn emit_with_fields(
    stage: DiagStage,
    level: DiagLevel,
    resource: Option<&str>,
    file: Option<&str>,
    message: impl Into<String>,
    fields: serde_json::Value,
) {
    bus().publish(DiagEvent {
        at: Utc::now(),
        seq: 0,
        stage,
        level,
        resource: resource.map(|s| s.to_string()),
        file: file.map(|s| s.to_string()),
        message: message.into(),
        fields,
    });
}

// ─── Log forwarder ──────────────────────────────────────────────────────────

/// `log::Log` impl that mirrors every log macro into the diagnostics bus AND
/// delegates to env_logger for stderr. Installed once during `lib::run()`.
pub struct LogForwarder {
    inner: env_logger::Logger,
}

impl LogForwarder {
    pub fn install() {
        let inner = env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("info"),
        )
        .build();
        let max_level = inner.filter();
        let forwarder = Self { inner };
        if log::set_boxed_logger(Box::new(forwarder)).is_ok() {
            log::set_max_level(max_level);
        }
    }
}

impl log::Log for LogForwarder {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if !self.inner.enabled(record.metadata()) {
            return;
        }
        self.inner.log(record);
        // Mirror into the bus. Skip our own log-forwarder records (would loop
        // if any subscriber path called log::*). We tag stage=Log so the UI
        // can filter generic log lines vs structured pipeline events.
        let target = record.target();
        if target.starts_with("rift_tauri_lib::diagnostics") {
            return;
        }
        let message = format!("{}", record.args());
        bus().publish(DiagEvent {
            at: Utc::now(),
            seq: 0,
            stage: DiagStage::Log,
            level: DiagLevel::from(record.level()),
            resource: None,
            file: None,
            message,
            fields: serde_json::json!({ "target": target }),
        });
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

// ─── Frontend forwarder ─────────────────────────────────────────────────────

/// Pumps bus events to the frontend over `diag://event`. Rate-limited at
/// FRONTEND_RATE_PER_SEC to stay under Svelte reactivity overhead during
/// FS-event bursts; overflow is dropped (the bus retains the canonical
/// history for export).
///
/// Started once on app boot; runs for the life of the process.
pub fn spawn_frontend_pump(app: tauri::AppHandle) {
    use tauri::Emitter;
    use tokio::sync::broadcast::error::RecvError;

    let mut rx = bus().subscribe();
    tauri::async_runtime::spawn(async move {
        let mut window_start = std::time::Instant::now();
        let mut window_emitted: u32 = 0;
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let now = std::time::Instant::now();
                    if now.duration_since(window_start).as_millis() >= 1000 {
                        window_start = now;
                        window_emitted = 0;
                    }
                    if window_emitted >= FRONTEND_RATE_PER_SEC {
                        continue;
                    }
                    window_emitted += 1;
                    let _ = app.emit("diag://event", &ev);
                }
                Err(RecvError::Lagged(n)) => {
                    bus().record_bus_lag(n);
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}
