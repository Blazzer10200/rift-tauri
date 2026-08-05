//! Diagnostics — process-global log/event bus.
//!
//! Any subsystem can publish into the bus without dep-injection; a pump
//! forwards events to the frontend over the `diag://event` channel.
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

use std::collections::VecDeque;
use std::io::Write as _;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub mod metrics;
pub mod perf;

const BUS_CAPACITY: usize = 4096;
/// Late-subscriber backlog — broadcast has no replay, so a console (or the
/// `diag_backlog` command) attaching mid-session would otherwise start blank.
const BACKLOG_CAP: usize = 500;
const FRONTEND_RATE_PER_SEC: u32 = 200;
/// #246: secondary ceiling on critical-bypass events. Pathological loops
/// (e.g. a System/Error event emitted in a hot retry) could otherwise flood
/// Svelte reactivity without limit. 50/s leaves head-room for normal bursty
/// activity (drift result + reconnect + bridge ack arriving in the same
/// second) while bounding pathological cases.
const FRONTEND_CRITICAL_RATE_PER_SEC: u32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagStage {
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
    /// Correlation id tying every event of one assistant turn together —
    /// `"<session_id>#<turn_epoch>"`. First-class (not buried in `fields`) so
    /// `read_events` / the console can filter a whole turn by it. Pairing
    /// `serde(default)` with skip-when-none keeps historical NDJSON lines
    /// readable and tool-less events lean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub file: Option<String>,
    pub message: String,
    pub fields: serde_json::Value,
}

pub struct DiagBus {
    tx: broadcast::Sender<DiagEvent>,
    seq: AtomicU64,
    backlog: Mutex<VecDeque<DiagEvent>>,
}

fn basename_only(path: &str) -> String {
    let norm = path.replace('\\', "/");
    norm.rsplit('/').find(|s| !s.is_empty()).unwrap_or("").to_string()
}

impl DiagBus {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(BUS_CAPACITY);
        Self {
            tx,
            seq: AtomicU64::new(0),
            backlog: Mutex::new(VecDeque::with_capacity(BACKLOG_CAP)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DiagEvent> {
        self.tx.subscribe()
    }

    pub fn publish(&self, mut event: DiagEvent) {
        // Renderer-bound: trailing basename only. Absolute paths leak the
        // user's directory structure (and occasionally embedded credentials)
        // to the webview. The basename keeps the signal — which file moved —
        // without the noise.
        event.file = event.file.as_deref().map(basename_only);
        // #238 / completes #8: scrub home-dir prefixes + private-key bodies
        // on every bus-bound message. LogForwarder already scrubs; the direct
        // `emit*` helpers bypassed scrub entirely. Idempotent — homedir
        // replacement is a no-op on already-scrubbed strings, key-body
        // redaction is a no-op on `[REDACTED ...]`.
        event.message = scrub_log_message(&event.message);
        if !event.fields.is_null() {
            // RR8: scrub the string LEAVES, not the serialized blob. `to_string()`
            // JSON-escapes `\` → `\\`, so a single-backslash USERPROFILE prefix
            // (`C:\Users\foo`) never matched in structured fields — Windows paths
            // leaked through. Walking the tree scrubs each String at its real,
            // unescaped value.
            scrub_value(&mut event.fields);
        }
        event.seq = self.seq.fetch_add(1, Ordering::Relaxed);
        {
            // Poison-recover like the file sink — a panic mid-push must not
            // blackout the backlog for the rest of the session.
            let mut bl = match self.backlog.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if bl.len() >= BACKLOG_CAP {
                bl.pop_front();
            }
            bl.push_back(event.clone());
        }
        // send() returns Err only when there are zero subscribers — that's
        // fine, we drop on the floor. Lag is reported through Receiver::recv
        // returning RecvError::Lagged on the consumer side.
        let _ = self.tx.send(event);
    }

    pub fn backlog_snapshot(&self) -> Vec<DiagEvent> {
        let bl = match self.backlog.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        bl.iter().cloned().collect()
    }

}

static BUS: OnceLock<DiagBus> = OnceLock::new();

pub fn bus() -> &'static DiagBus {
    BUS.get_or_init(DiagBus::new)
}

// ─── Convenience emit helpers ───────────────────────────────────────────────

pub fn emit_with_fields(
    stage: DiagStage,
    level: DiagLevel,
    resource: Option<&str>,
    file: Option<&str>,
    message: impl Into<String>,
    fields: serde_json::Value,
) {
    emit_scoped(stage, level, resource, file, message, fields, None);
}

/// `emit_with_fields` + a turn correlation id. Callers on the turn hot path
/// (dispatch outcomes, per-turn perf) pass `Some("<session>#<epoch>")` so the
/// whole turn's events share a filterable key; everything else uses the plain
/// helper above (turn_id defaults to None).
#[allow(clippy::too_many_arguments)]
pub fn emit_scoped(
    stage: DiagStage,
    level: DiagLevel,
    resource: Option<&str>,
    file: Option<&str>,
    message: impl Into<String>,
    fields: serde_json::Value,
    turn_id: Option<&str>,
) {
    bus().publish(DiagEvent {
        at: Utc::now(),
        seq: 0,
        stage,
        level,
        resource: resource.map(|s| s.to_string()),
        turn_id: turn_id.map(|s| s.to_string()),
        file: file.map(|s| s.to_string()),
        message: message.into(),
        fields,
    });
}

// ─── Persistent file sink ───────────────────────────────────────────────────

/// Rotating file sink for every `log` record. env_logger's stderr is /dev/null
/// in a GUI prod build, so in-process Velopack check/download/apply logs (and
/// their failures) vanished — the root cause of undiagnosable "update won't
/// install" reports. This persists them to `<appLogDir>/rift.log` so the next
/// failure is always traceable. Resolved lazily on the first record.
static FILE_LOG: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();

/// Mirror Tauri's `appLogDir()` for identifier `com.blazzer.rift`. Computed
/// from env (no AppHandle needed — `LogForwarder::install()` runs before the
/// Tauri app is built).
pub(crate) fn app_log_path() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share"));
    let dir = base?.join("com.blazzer.rift").join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("rift.log"))
}

fn init_file_log() -> Option<Mutex<std::fs::File>> {
    let path = app_log_path()?;
    // Size-based rotation so a long-lived install can't grow the file unbounded.
    const MAX_BYTES: u64 = 5 * 1024 * 1024;
    let oversized = std::fs::metadata(&path).map(|m| m.len() > MAX_BYTES).unwrap_or(false);
    // B11: if rotation fails (e.g. .log.old is locked), truncate on open instead
    // of appending — otherwise a stuck rename reopens the full 5 MB log in append
    // mode and it grows without bound. No logging here: we ARE the log sink.
    let rotate_failed =
        oversized && std::fs::rename(&path, path.with_extension("log.old")).is_err();
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true);
    if rotate_failed {
        opts.write(true).truncate(true);
    } else {
        opts.append(true);
    }
    let file = opts.open(&path).ok()?;
    Some(Mutex::new(file))
}

fn file_log_write(level: log::Level, target: &str, msg: &str) {
    if let Some(m) = FILE_LOG.get_or_init(init_file_log) {
        // Recover from poison: a panic mid-write must not blackout all future
        // logging (this is the only persistent sink in a GUI prod build — stderr
        // is /dev/null). Matches CONFIG_WRITE_LOCK / turn.rs into_inner() pattern.
        let mut f = match m.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let _ = writeln!(
            f,
            "{} [{:<5}] {} — {}",
            Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            level,
            target,
            msg
        );
        let _ = f.flush();
    }
}

/// Write a dedicated, non-rotating crash report on panic. The rotating
/// `rift.log` keeps only one `.log.old` backup — a second crash in a session
/// overwrites it before the user can grab it, and a startup panic that fires
/// before the frontend pump is wired never reaches the UI at all. Each panic
/// instead gets its own `crash-<ts>.txt` in the log dir so a field user can
/// always find and send the exact crash. Best-effort: a write failure here is
/// itself swallowed (we're already unwinding — nothing useful to do with it).
///
/// `location` + `payload` are expected pre-scrubbed by the caller; the
/// force-captured backtrace is scrubbed here.
pub fn write_crash_report(location: &str, payload: &str) {
    let Some(log) = app_log_path() else { return };
    let Some(dir) = log.parent() else { return };
    // `%.3fZ` timestamp has no `:` — safe as a Windows filename.
    let ts = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let path = dir.join(format!("crash-{ts}.txt"));
    let backtrace = scrub_log_message(&std::backtrace::Backtrace::force_capture().to_string());
    let body = format!(
        "Rift crash report\nversion:  {}\ntime:     {}\nlocation: {}\npayload:  {}\n\n--- backtrace ---\n{}\n",
        env!("CARGO_PKG_VERSION"),
        Utc::now().to_rfc3339(),
        location,
        payload,
        backtrace,
    );
    let _ = std::fs::write(&path, body);
}

// ─── Structured event sink (events.ndjson) ──────────────────────────────────

/// NDJSON mirror of every bus event — the structured counterpart of `rift.log`
/// (which only receives `log::` macro text). The MCP server (separate process,
/// same binary) tails it for `read_events`, giving the assistant the same view
/// the Diagnostics console renders — structured fields intact, including
/// webview errors forwarded through `diag_frontend_event`.
static EVENTS_LOG: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
static EVENTS_LOG_BYTES: AtomicU64 = AtomicU64::new(0);
const EVENTS_LOG_MAX_BYTES: u64 = 5 * 1024 * 1024;
/// events.ndjson lines dropped because the disk-writer channel was full (the
/// writer fell behind a burst). Bounded backpressure — the live console + bus
/// stay complete; only the on-disk NDJSON mirror skips lines. A one-time warn
/// keeps a lagging disk from becoming silently lossy.
static DISK_SINK_DROPPED: AtomicU64 = AtomicU64::new(0);
const DISK_SINK_CAP: usize = 4096;

/// `<appLogDir>/events.ndjson` — beside `rift.log` (mirrors `turns_log_path`).
pub fn events_log_path() -> Option<std::path::PathBuf> {
    let log = app_log_path()?;
    Some(log.parent()?.join("events.ndjson"))
}

fn init_events_log() -> Option<Mutex<std::fs::File>> {
    let path = events_log_path()?;
    let start_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let oversized = start_len > EVENTS_LOG_MAX_BYTES;
    let rotate_failed =
        oversized && std::fs::rename(&path, path.with_extension("ndjson.old")).is_err();
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true);
    if rotate_failed {
        opts.write(true).truncate(true);
    } else {
        opts.append(true);
    }
    let file = opts.open(&path).ok()?;
    let seed = if rotate_failed || oversized { 0 } else { start_len };
    EVENTS_LOG_BYTES.store(seed, Ordering::Relaxed);
    Some(Mutex::new(file))
}

fn events_log_write(ev: &DiagEvent) {
    let Some(cell) = EVENTS_LOG.get_or_init(init_events_log).as_ref() else { return };
    let mut f = match cell.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let Ok(mut line) = serde_json::to_string(ev) else { return };
    line.push('\n');
    use std::io::Seek as _;
    // In-place periodic rotation — same counter trick as `append_turn_perf`:
    // the OnceLock size check only ever runs once per process.
    let written =
        EVENTS_LOG_BYTES.fetch_add(line.len() as u64, Ordering::Relaxed) + line.len() as u64;
    if written > EVENTS_LOG_MAX_BYTES
        && f.set_len(0).is_ok()
        && f.seek(std::io::SeekFrom::Start(0)).is_ok()
    {
        EVENTS_LOG_BYTES.store(0, Ordering::Relaxed);
    }
    let _ = f.write_all(line.as_bytes());
    let _ = f.flush();
}

/// Drain bus events onto the NDJSON sink from one dedicated OS thread — the
/// broadcast receiver never blocks on disk I/O. The writer channel is BOUNDED
/// (`DISK_SINK_CAP`): if the disk falls behind a burst, lines are dropped (and
/// counted in `DISK_SINK_DROPPED`) rather than growing memory without limit.
/// Only the on-disk NDJSON mirror skips lines — the live console + bus, fed from
/// the broadcast directly, stay complete.
pub fn spawn_event_sink() {
    use tokio::sync::broadcast::error::RecvError;
    use std::sync::mpsc::TrySendError;
    let mut rx = bus().subscribe();
    let (tx, file_rx) = std::sync::mpsc::sync_channel::<DiagEvent>(DISK_SINK_CAP);
    std::thread::spawn(move || {
        while let Ok(ev) = file_rx.recv() {
            events_log_write(&ev);
        }
    });
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(ev) => match tx.try_send(ev) {
                    Ok(()) => {}
                    // Writer fell behind — drop this line (bounded backpressure),
                    // count it, and warn once so a lagging disk isn't silent.
                    Err(TrySendError::Full(_)) => {
                        if DISK_SINK_DROPPED.fetch_add(1, Ordering::Relaxed) == 0 {
                            log::warn!("diagnostics disk sink lagging — dropping events.ndjson lines under load (console + bus stay complete)");
                        }
                    }
                    Err(TrySendError::Disconnected(_)) => break,
                },
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });
}

// ─── Log forwarder ──────────────────────────────────────────────────────────

/// `log::Log` impl that mirrors every log macro into the diagnostics bus AND
/// a persistent file sink, and delegates to env_logger for stderr. Installed
/// once during `lib::run()`.
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

/// Scrub log messages of homedir prefixes + obvious key/secret markers
/// before they're broadcast to the diag bus (and onward to the renderer).
/// Disable via `RIFT_LOG_SCRUB=0` for dev/debugging only.
///
/// Scrubs applied (order matters — homedir first so it doesn't undo
/// secret-body redaction):
///   1. `$USERPROFILE` / `$HOME` (both backslash + forward-slash forms) → `~`
///   2. Lines containing OpenSSH/RSA `BEGIN ... PRIVATE KEY` markers →
///      full-message redaction (safer than per-line — a single leaked body
///      line is enough to compromise the key, so drop the whole message).
///
/// RR8: recursively scrub every String leaf of a JSON value in place. Used on
/// structured `event.fields` so home-dir prefixes / key bodies are redacted at
/// their real (unescaped) values — serializing first would `\`-escape Windows
/// paths and defeat the literal match in `scrub_log_message`.
pub fn scrub_value(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) => {
            let scrubbed = scrub_log_message(s);
            if scrubbed != *s {
                *s = scrubbed;
            }
        }
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(scrub_value),
        serde_json::Value::Object(map) => map.values_mut().for_each(scrub_value),
        _ => {}
    }
}

pub fn scrub_log_message(msg: &str) -> String {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    let enabled = *ENABLED
        .get_or_init(|| !matches!(std::env::var("RIFT_LOG_SCRUB").as_deref(), Ok("0")));
    if !enabled {
        return msg.to_string();
    }
    let mut out = msg.to_string();
    for var in ["USERPROFILE", "HOME"] {
        if let Ok(home) = std::env::var(var) {
            if !home.is_empty() {
                let fwd = home.replace('\\', "/");
                // Guard each replace with `contains` — `str::replace` scans + allocates
                // a fresh String unconditionally, but most log lines hold no home-dir.
                if home.len() >= 3 && out.contains(&home) {
                    out = out.replace(&home, "~");
                }
                if fwd != home && fwd.len() >= 3 && out.contains(&fwd) {
                    out = out.replace(&fwd, "~");
                }
            }
        }
    }
    // #227: Ed25519 (PKCS#8) is the most common SSH key type now; DSA still
    // appears on legacy systems. Original guard missed both — they passed
    // through to the renderer unredacted. Generic `BEGIN ... PRIVATE KEY`
    // catch-all covers any future format too.
    if out.contains("BEGIN OPENSSH PRIVATE KEY")
        || out.contains("BEGIN RSA PRIVATE KEY")
        || out.contains("BEGIN EC PRIVATE KEY")
        || out.contains("BEGIN ED25519 PRIVATE KEY")
        || out.contains("BEGIN DSA PRIVATE KEY")
        || out.contains("BEGIN PRIVATE KEY")
        || out.contains("BEGIN ENCRYPTED PRIVATE KEY")
    {
        return "[REDACTED — log line contained private-key body]".to_string();
    }
    out
}

impl log::Log for LogForwarder {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if !self.inner.enabled(record.metadata()) {
            return;
        }
        let target = record.target();
        let message = scrub_log_message(&format!("{}", record.args()));
        // Forward to env_logger (dev stderr) with the SCRUBBED message, not the
        // raw record — else home-dir paths leak to the dev terminal before scrub.
        self.inner.log(
            &log::Record::builder()
                .args(format_args!("{message}"))
                .metadata(record.metadata().clone())
                .level(record.level())
                .target(target)
                .module_path(record.module_path())
                .file(record.file())
                .line(record.line())
                .build(),
        );
        // Persist to the rotating file sink (captures Velopack's in-process
        // check/download/apply logs that stderr discards in GUI prod). Done for
        // every record, including our own forwarder target — no loop risk here.
        file_log_write(record.level(), target, &message);
        // Mirror into the bus. Skip our own log-forwarder records (would loop
        // if any subscriber path called log::*). We tag stage=Log so the UI
        // can filter generic log lines vs structured pipeline events.
        if target.starts_with("rift_tauri_lib::diagnostics") {
            return;
        }
        bus().publish(DiagEvent {
            at: Utc::now(),
            seq: 0,
            stage: DiagStage::Log,
            level: DiagLevel::from(record.level()),
            resource: None,
            turn_id: None,
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
        let mut crit_window_start = std::time::Instant::now();
        let mut crit_window_emitted: u32 = 0;
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    // System events (panics, command errors) bypass the 200/s
                    // cap so they can't be dropped under a Log-event burst.
                    let is_critical = matches!(ev.stage, DiagStage::System);
                    if is_critical {
                        // #246: secondary ceiling on the critical bypass —
                        // pathological loops (e.g. a System/Error event fired
                        // in tight retry) could otherwise flood Svelte
                        // reactivity. 50/s is well above any legitimate
                        // burst we've seen in production.
                        let now = std::time::Instant::now();
                        if now.duration_since(crit_window_start).as_millis() >= 1000 {
                            crit_window_start = now;
                            crit_window_emitted = 0;
                        }
                        if crit_window_emitted >= FRONTEND_CRITICAL_RATE_PER_SEC {
                            continue;
                        }
                        crit_window_emitted += 1;
                    } else {
                        let now = std::time::Instant::now();
                        if now.duration_since(window_start).as_millis() >= 1000 {
                            window_start = now;
                            window_emitted = 0;
                        }
                        if window_emitted >= FRONTEND_RATE_PER_SEC {
                            continue;
                        }
                        window_emitted += 1;
                    }
                    let _ = app.emit("diag://event", &ev);
                }
                Err(RecvError::Lagged(n)) => {
                    // Direct file write, NOT log::warn! — the latter re-enters
                    // LogForwarder (file mutex + flush) on this tokio task and
                    // re-publishes onto the very bus that just lagged.
                    // RR8: off the async worker — file_log_write grabs a blocking
                    // mutex + does sync I/O, which would stall the emit loop (and
                    // every other task on this worker) under disk contention.
                    let msg = format!("diag bus lagged: {n} events dropped");
                    tokio::task::spawn_blocking(move || {
                        file_log_write(
                            log::Level::Warn,
                            "rift_tauri_lib::diagnostics",
                            &msg,
                        );
                    });
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    /// `app_log_path()` mirrors the bundle identifier as a string literal
    /// (it runs before the AppHandle exists, so it can't query Tauri). This
    /// pins the two copies together — if `tauri.conf.json` ever changes its
    /// identifier, this fails instead of logs silently diverging.
    #[test]
    fn bundle_identifier_matches_tauri_conf() {
        let conf = include_str!("../../tauri.conf.json");
        assert!(
            conf.contains("\"identifier\": \"com.blazzer.rift\""),
            "tauri.conf.json identifier changed — update app_log_path() in diagnostics/mod.rs to match"
        );
    }
}
