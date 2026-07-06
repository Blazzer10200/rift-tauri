//! Speech-to-text — engine routing + config persistence.
//!
//! Two engines coexist:
//!   * `web_speech` — legacy. Recognition runs in the WebView via the Web
//!     Speech API (Edge's Azure-backed recogniser). No Rust audio path; Rust
//!     only persists settings.
//!   * `whisper`    — local Whisper Large v3 Turbo via whisper-rs on CUDA,
//!     gated by webrtc-vad, optionally polished by Claude Haiku. Rust owns
//!     mic capture (cpal), inference, and event emission. Default after the
//!     user's first successful Whisper transcription.
//!
//! Sub-modules ([`audio`], [`vad`], [`whisper`], [`cleanup`], [`model_manager`])
//! are scaffolded as stubs in Phase 1 and wired in subsequent phases.

pub mod audio;
pub mod cleanup;
pub mod model_manager;
pub mod vad;
pub mod whisper;

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex as AsyncMutex;
use tokio_util::sync::CancellationToken;

const DEFAULT_INITIAL_PROMPT: &str =
    "Casual Southern American English. Informal contractions, dropped word \
     endings, and slurred consonants are normal. Profanity is fine and is \
     transcribed verbatim — fuck, shit, damn, ass — never masked or asterisked. \
     Transcribe what was meant, not a literal phonetic reading.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// Master switch. When false the composer's mic button is hidden.
    #[serde(default)]
    pub enabled: bool,
    /// BCP-47 language tag (e.g. `"en-US"`, `"es-ES"`). Empty = browser default.
    /// Only used by the `web_speech` engine.
    #[serde(default = "default_lang")]
    pub language: String,
    /// When true the transcript appends to the existing draft; false replaces it.
    #[serde(default = "default_true")]
    pub append_to_draft: bool,
    /// When true the recogniser keeps running until the user clicks stop;
    /// false stops after the first final result. Only used by `web_speech`.
    #[serde(default = "default_true")]
    pub continuous: bool,
    /// When true, partial in-progress text appears in the composer as you
    /// speak; the final committed text replaces it on each segment.
    #[serde(default = "default_true")]
    pub show_interim: bool,

    /// Engine selector. `"web_speech"` (default, legacy) or `"whisper"` (local).
    #[serde(default = "default_engine")]
    pub engine: String,
    /// Whisper model id. See `model_manager::known_models()`.
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,
    /// Input device name (cpal). `None` = system default.
    #[serde(default)]
    pub input_device: Option<String>,
    /// Whisper `initial_prompt` — biases the decoder toward the user's
    /// vocabulary + speaking style. Capped at 224 tokens at use-time.
    #[serde(default = "default_initial_prompt")]
    pub initial_prompt: String,
    /// User vocabulary (newline-separated). Concatenated onto `initial_prompt`
    /// before being passed to Whisper.
    #[serde(default)]
    pub vocab_text: String,
    /// When true, final transcripts are polished by Claude Haiku before being
    /// written into the composer draft.
    #[serde(default = "default_true")]
    pub cleanup_enabled: bool,
    /// Whisper decode strategy. `None`/`Some(1)` = greedy (fast). `Some(n>1)`
    /// = beam search with that width (slower, sharper on technical vocab — GPU
    /// only in practice). Whisper engine only; ignored by `web_speech`.
    #[serde(default)]
    pub beam_size: Option<u8>,
    /// Spoken commands ("send it", "new line", "scratch that"). Interpreted
    /// frontend-side; stored here so the preference rides the same config file.
    #[serde(default = "default_true")]
    pub voice_commands: bool,
    /// End recording after this many seconds of silence. 0 = off. Enforced
    /// frontend-side off the speech-event stream.
    #[serde(default)]
    pub auto_stop_secs: u16,

    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn default_lang() -> String {
    "en-US".to_string()
}
fn default_true() -> bool {
    true
}
fn default_engine() -> String {
    "web_speech".to_string()
}
fn default_whisper_model() -> String {
    "large-v3-turbo-q5_0".to_string()
}
fn default_initial_prompt() -> String {
    DEFAULT_INITIAL_PROMPT.to_string()
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            language: default_lang(),
            append_to_draft: true,
            continuous: true,
            show_interim: true,
            engine: default_engine(),
            whisper_model: default_whisper_model(),
            input_device: None,
            initial_prompt: default_initial_prompt(),
            vocab_text: String::new(),
            cleanup_enabled: true,
            beam_size: None,
            voice_commands: true,
            auto_stop_secs: 0,
            extra: serde_json::Map::new(),
        }
    }
}

fn config_path() -> PathBuf {
    crate::state::paths::dirs_home_or_temp().join(".rift").join("stt-config.json")
}

/// Serializes config disk access — `stt_set_config` is a sync command on the
/// thread pool, so concurrent saves could otherwise tear the file (truncate
/// races overwrite). Mirrors `assistant::config::CONFIG_WRITE_LOCK`.
static STT_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn load_config() -> SttConfig {
    let _g = STT_WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = config_path();
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return SttConfig::default(),
    };
    if bytes.len() > 64 * 1024 {
        return SttConfig::default();
    }
    serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        log::warn!("stt-config parse failed ({e}), using defaults");
        SttConfig::default()
    })
}

fn save_config(cfg: &SttConfig) -> Result<(), String> {
    let _g = STT_WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir ~/.rift: {e}"))?;
    }
    let json = serde_json::to_vec_pretty(cfg).map_err(|e| format!("serialise: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write: {e}"))
}

#[tauri::command]
pub fn stt_get_config() -> Result<SttConfig, String> {
    Ok(load_config())
}

#[tauri::command]
pub fn stt_set_config(config: SttConfig) -> Result<(), String> {
    save_config(&config)
}

// ============================================================================
// Whisper session orchestration
// ============================================================================
//
// Two managed Tauri state slots:
//   * `WhisperCache` — Option<WhisperEngine>. The loaded model survives across
//     record sessions; only swapped when the user picks a different model.
//   * `WhisperSession` — Option<ActiveSession>. The currently-recording
//     session. Holding the mic stream + rolling-window task.
//
// `cpal::Stream` is `!Send`, so it lives on a dedicated `std::thread` that
// blocks on a `mpsc` channel. Dropping the sender (or sending `()`) drops the
// Stream, which stops mic capture. Tokio sees none of this.

pub struct WhisperCache(pub AsyncMutex<Option<whisper::WhisperEngine>>);
pub struct WhisperSession(pub AsyncMutex<Option<ActiveSession>>);

pub struct ActiveSession {
    ring: audio::AudioRing,
    /// The engine THIS session loaded/resolved at start. Stored so the final
    /// transcribe uses the session's own model, not whatever is in the shared
    /// cache at stop-time — a second start racing during this session's model
    /// load can overwrite that cache with a DIFFERENT model/quantization, and
    /// re-reading it at stop would silently finalize with the wrong engine.
    engine: whisper::WhisperEngine,
    initial_prompt: String,
    language: Option<String>,
    beam_size: Option<u8>,
    /// Label of the window that started this session — all stt:// events
    /// emit_to it only, so a second window's mic UI never mirrors a recording
    /// it doesn't own (multi-window rule, same as turn events).
    window_label: String,
    /// Workspace-context string (project/branch/files) reused for the Haiku
    /// cleanup pass at stop-time so it shares Whisper's domain vocabulary.
    workspace_ctx: String,
    cleanup_enabled: bool,
    rolling_cancel: CancellationToken,
    /// Dropping sends a stop signal; capture thread drops cpal::Stream, mic
    /// closes. Sender is `Option` so `stop` can take + drop it explicitly.
    capture_stop: Option<std::sync::mpsc::Sender<()>>,
    capture_handle: Option<std::thread::JoinHandle<()>>,
}

impl ActiveSession {
    /// Signal capture to stop; returns the thread handle so the caller can
    /// join it without blocking a Tokio worker (see `stt_stop_recording`).
    fn shutdown_capture(&mut self) -> Option<std::thread::JoinHandle<()>> {
        self.rolling_cancel.cancel();
        if let Some(tx) = self.capture_stop.take() {
            let _ = tx.send(());
            drop(tx);
        }
        self.capture_handle.take()
    }
}

impl Drop for ActiveSession {
    fn drop(&mut self) {
        // Drop-path is off the async runtime; blocking join is safe here.
        // B10: surface a panicked capture thread instead of swallowing it — a
        // silent panic here leaks the mic handle to process exit with no signal.
        if let Some(h) = self.shutdown_capture() {
            if let Err(e) = h.join() {
                let msg = e
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| e.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("<non-string panic payload>");
                log::warn!("stt: capture thread panicked on shutdown: {msg}");
            }
        }
    }
}

#[derive(Clone, Serialize)]
struct PartialPayload {
    text: String,
}

#[derive(Clone, Serialize)]
struct LevelPayload {
    /// RMS amplitude 0.0..~1.0 of the live mic input, for the meter.
    rms: f32,
}

#[derive(Clone, Serialize)]
struct FinalPayload {
    text: String,
    raw: String,
    cleaned: bool,
}

#[derive(Clone, Serialize)]
struct StatePayload {
    state: &'static str,
    message: Option<String>,
}

fn emit_state(app: &AppHandle, win: &str, state: &'static str, msg: Option<String>) {
    let _ = app.emit_to(win, "stt://state", StatePayload { state, message: msg });
}

fn emit_error(app: &AppHandle, win: &str, code: &str, message: &str) {
    let _ = app.emit_to(
        win,
        "stt://error",
        json!({ "code": code, "message": message }),
    );
}

/// BCP-47 (`"en-US"`) → ISO 639-1 (`"en"`) for whisper.cpp's `set_language`.
/// Empty/whitespace → `None`, which lets whisper.cpp auto-detect.
fn lang_code(bcp47: &str) -> Option<String> {
    let t = bcp47.trim();
    if t.is_empty() {
        return None;
    }
    Some(
        t.split(['-', '_'])
            .next()
            .unwrap_or(t)
            .to_ascii_lowercase(),
    )
}

/// A short workspace-context string — project name, git branch, and a sample
/// of open filenames — used to bias Whisper (and the Haiku cleanup pass)
/// toward the user's project vocabulary. Empty when no folder is open.
fn workspace_context() -> String {
    use std::fmt::Write as _;
    let root = match crate::assistant::current_root() {
        Some(r) => r,
        None => return String::new(),
    };
    let mut ctx = String::new();
    if let Some(name) = root.file_name().and_then(|n| n.to_str()) {
        let _ = write!(ctx, "Project: {name}.");
    }
    if let Some(branch) = crate::assistant::workspace::workspace_branch_sync(&root) {
        let _ = write!(ctx, " Branch: {branch}.");
    }
    {
        let files = crate::assistant::workspace::list_workspace_files_sync(&root);
        // Distinct basenames, first ~30. compose_prompt's 800-char cap is the
        // final backstop; this keeps the file list from dominating it.
        let mut names: Vec<&str> = Vec::new();
        for p in &files {
            let n = p.rsplit('/').next().unwrap_or(p.as_str());
            if n.starts_with('.') || names.contains(&n) {
                continue;
            }
            names.push(n);
            if names.len() >= 30 {
                break;
            }
        }
        if !names.is_empty() {
            let _ = write!(ctx, " Files: {}.", names.join(", "));
        }
    }
    ctx
}

#[tauri::command]
pub async fn stt_start_recording(
    app: AppHandle,
    window: tauri::Window,
    cache: tauri::State<'_, WhisperCache>,
    session: tauri::State<'_, WhisperSession>,
    model: Option<String>,
) -> Result<(), String> {
    let win = window.label().to_string();
    let cfg = load_config();
    let model_id = model.unwrap_or(cfg.whisper_model.clone());

    // Refuse if already recording — caller must stop first.
    {
        let s = session.0.lock().await;
        if s.is_some() {
            return Err("stt session already active".into());
        }
    }

    // Resolve model path.
    let models = model_manager::known_models();
    let model_info = models
        .iter()
        .find(|m| m.id == model_id)
        .ok_or_else(|| format!("unknown model id: {model_id}"))?;
    if !model_info.downloaded {
        return Err(format!(
            "model '{}' not downloaded — fetch it in Settings first",
            model_id
        ));
    }
    let model_path = model_info
        .path
        .clone()
        .ok_or_else(|| "model path missing".to_string())?;

    // Load or reuse engine. Reload only when the requested model differs.
    // Load happens OUTSIDE the cache lock so stt_stop_recording isn't blocked
    // for the whole multi-second model load.
    let engine = {
        let cached = {
            let slot = cache.0.lock().await;
            slot.as_ref().filter(|e| e.model_id == model_id).cloned()
        };
        match cached {
            Some(e) => e,
            None => {
                emit_state(&app, &win, "loading_model", Some(model_id.clone()));
                let model_id_owned = model_id.clone();
                let loaded = tokio::task::spawn_blocking(move || {
                    whisper::WhisperEngine::load(&model_path, &model_id_owned)
                })
                .await
                .map_err(|e| format!("model load task join: {e}"))??;
                let engine = loaded.clone();
                *cache.0.lock().await = Some(loaded);
                engine
            }
        }
    };

    // Start mic capture (off the tokio runtime — cpal::Stream is !Send).
    let device_name = cfg.input_device.clone();
    let (cap_ready_tx, cap_ready_rx) = std::sync::mpsc::channel::<Result<audio::AudioRing, String>>();
    let (cap_stop_tx, cap_stop_rx) = std::sync::mpsc::channel::<()>();
    let capture_handle = std::thread::spawn(move || {
        let capture = match audio::start_capture(device_name.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                let _ = cap_ready_tx.send(Err(e));
                return;
            }
        };
        let ring = capture.ring.clone();
        if cap_ready_tx.send(Ok(ring)).is_err() {
            return;
        }
        // Hold the Stream alive until told to stop (or sender dropped).
        let _ = cap_stop_rx.recv();
        drop(capture);
    });

    // recv() would block the Tokio worker — offload to a blocking thread.
    // Bounded wait: a stalled audio subsystem (Bluetooth/WASAPI device-enum hang
    // on Windows) must not wedge stt_start_recording forever and burn a blocking
    // thread-pool slot. On timeout we surface an error instead of hanging; the
    // capture thread is reaped when stop_recording drops the session's stop tx.
    let ring = tokio::task::spawn_blocking(move || {
        cap_ready_rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|e| format!("capture init channel: {e}"))
            .and_then(|r| r)
    })
    .await
    .map_err(|e| format!("capture init join: {e}"))??;

    // Fold the workspace context into the vocab slot so it biases the decoder
    // toward project filenames/symbols; user vocab follows (trimmed first if
    // the 800-char prompt budget is exceeded).
    // RR11: workspace_context() spawns `git rev-parse` + walks up to 4000 files
    // synchronously (workspace.rs mandates spawn_blocking for Tokio callers);
    // running it inline here stalled a worker for seconds on record-start.
    let workspace_ctx = tokio::task::spawn_blocking(workspace_context)
        .await
        .unwrap_or_default();
    let vocab = if workspace_ctx.is_empty() {
        cfg.vocab_text.clone()
    } else if cfg.vocab_text.trim().is_empty() {
        workspace_ctx.clone()
    } else {
        format!("{workspace_ctx} {}", cfg.vocab_text)
    };
    let initial_prompt = whisper::compose_prompt(&cfg.initial_prompt, &vocab);
    let language = lang_code(&cfg.language);
    let beam_size = cfg.beam_size;
    let cleanup_enabled = cfg.cleanup_enabled;
    let rolling_cancel = CancellationToken::new();
    // Clone for the spawn before moving rolling_cancel into the session slot.
    let task_cancel = rolling_cancel.clone();

    // Store session BEFORE spawning — any racing stop call can now cancel.
    {
        let mut slot = session.0.lock().await;
        // RR7: the is_some() guard above dropped the lock before the multi-second
        // model load + capture init, so two concurrent stt_start_recording calls
        // can both pass it and reach here. Re-check under THIS lock: if a racing
        // call already stored a session, bail rather than silently overwrite it
        // (which would orphan the first session's capture thread + engine).
        // Dropping cap_stop_tx here signals our just-started capture thread to exit.
        if slot.is_some() {
            return Err("stt session already active (raced concurrent start)".into());
        }
        *slot = Some(ActiveSession {
            ring: ring.clone(),
            engine: engine.clone(),
            initial_prompt: initial_prompt.clone(),
            language: language.clone(),
            beam_size,
            window_label: win.clone(),
            workspace_ctx,
            cleanup_enabled,
            rolling_cancel,
            capture_stop: Some(cap_stop_tx),
            capture_handle: Some(capture_handle),
        });
    }

    // Spawn the live-level meter loop (independent of show_interim) before the
    // ring is moved into the rolling loop below.
    {
        let level_app = app.clone();
        let level_win = win.clone();
        let level_ring = ring.clone();
        let level_cancel = task_cancel.clone();
        tokio::spawn(async move {
            level_loop(level_app, level_win, level_ring, level_cancel).await;
        });
    }

    // Spawn rolling-window transcribe task after session is visible.
    if cfg.show_interim {
        let task_app = app.clone();
        let task_win = win.clone();
        let task_ring = ring;
        let task_engine = engine.clone();
        let task_prompt = initial_prompt.clone();
        let task_lang = language.clone();
        tokio::spawn(async move {
            rolling_window_loop(
                task_app,
                task_win,
                task_ring,
                task_engine,
                task_prompt,
                task_lang,
                beam_size,
                task_cancel,
            )
            .await;
        });
    }

    emit_state(&app, &win, "recording", None);
    Ok(())
}

#[tauri::command]
pub async fn stt_stop_recording(
    app: AppHandle,
    // Retained for the command's DI signature but no longer read: the final
    // transcribe uses the session's own stored engine, not the shared cache.
    _cache: tauri::State<'_, WhisperCache>,
    session: tauri::State<'_, WhisperSession>,
) -> Result<String, String> {
    let mut active = {
        let mut slot = session.0.lock().await;
        slot.take()
    }
    .ok_or_else(|| "no stt session active".to_string())?;

    let win = active.window_label.clone();
    // Join the capture thread off the Tokio runtime to avoid blocking a worker.
    if let Some(h) = active.shutdown_capture() {
        tokio::task::spawn_blocking(move || { let _ = h.join(); }).await.ok();
    }
    emit_state(&app, &win, "transcribing", None);

    // Drain final buffer, transcribe with the SESSION'S OWN engine (stored at
    // start), NOT the shared cache — a start racing during this session's model
    // load could have overwritten the cache with a different model, and the
    // rolling partials all ran on this engine, so the final block must match.
    let samples = audio::drain_all(&active.ring);
    let engine = active.engine.clone();
    let prompt = active.initial_prompt.clone();
    let language = active.language.clone();
    let beam_size = active.beam_size;
    let raw = tokio::task::spawn_blocking(move || {
        engine.transcribe(&samples, &prompt, language.as_deref(), beam_size)
    })
    .await
    .map_err(|e| format!("final transcribe join: {e}"))??;

    let scrubbed = vad::strip_hallucinations(&raw);

    let final_text = if active.cleanup_enabled && !scrubbed.is_empty() {
        match cleanup::polish_with_ctx(&scrubbed, &active.workspace_ctx).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("[stt] cleanup hop failed, returning raw: {e}");
                // Surface the failure so the user knows the transcript is the
                // unpolished raw text (token expiry / network), not silent.
                emit_error(&app, &win, "cleanup_failed", &e);
                scrubbed.clone()
            }
        }
    } else {
        scrubbed.clone()
    };

    let _ = app.emit_to(
        &win,
        "stt://final",
        FinalPayload {
            text: final_text.clone(),
            raw: scrubbed,
            cleaned: active.cleanup_enabled,
        },
    );
    emit_state(&app, &win, "idle", None);
    Ok(final_text)
}

/// Rolling-window partial-emit loop. Ticks every 1 s; peeks the most-recent
/// 3 s of audio, runs VAD, transcribes if speech is present, emits the result
/// as `stt://partial` only when it differs from the previous emission (avoids
/// flooding the frontend w/ identical text).
async fn rolling_window_loop(
    app: AppHandle,
    win: String,
    ring: audio::AudioRing,
    engine: whisper::WhisperEngine,
    initial_prompt: String,
    language: Option<String>,
    beam_size: Option<u8>,
    cancel: CancellationToken,
) {
    const TICK_MS: u64 = 1000;
    const WINDOW_SECS: f32 = 3.0;
    let mut last_emitted = String::new();
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(TICK_MS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = ticker.tick() => {}
        }
        let samples = audio::peek_window(&ring, WINDOW_SECS);
        if samples.is_empty() {
            continue;
        }
        if !vad::window_has_speech(&samples) {
            continue;
        }
        let engine_c = engine.clone();
        let prompt_c = initial_prompt.clone();
        let lang_c = language.clone();
        let infer = tokio::task::spawn_blocking(move || {
            engine_c.transcribe(&samples, &prompt_c, lang_c.as_deref(), beam_size)
        });
        // RR10: race the (multi-second, CPU-bound) inference against cancel. If
        // stop fires mid-transcribe, return immediately — otherwise the stale
        // partial lands AFTER stt://final + stt://state:idle, overwriting the
        // Haiku-cleaned final transcript on the frontend.
        let raw = tokio::select! {
            _ = cancel.cancelled() => return,
            res = infer => match res {
                Ok(Ok(t)) => t,
                Ok(Err(e)) => {
                    emit_error(&app, &win, "transcribe_failed", &e);
                    continue;
                }
                Err(e) => {
                    emit_error(&app, &win, "task_join_failed", &e.to_string());
                    continue;
                }
            }
        };
        if cancel.is_cancelled() {
            return;
        }
        let scrubbed = vad::strip_hallucinations(&raw);
        if scrubbed.is_empty() || scrubbed == last_emitted {
            continue;
        }
        last_emitted = scrubbed.clone();
        let _ = app.emit_to(&win, "stt://partial", PartialPayload { text: scrubbed });
    }
}

/// Live input-level loop. Ticks ~12×/s, emits the RMS of the most-recent ~50ms
/// as `stt://level` so the frontend can drive a real meter. Independent of the
/// (1s, inference-bound) partial loop and of `show_interim` — the meter is
/// useful even with interim text off. Cheap (one short ring peek per tick), so
/// the fast cadence costs nothing on the inference hot path.
async fn level_loop(app: AppHandle, win: String, ring: audio::AudioRing, cancel: CancellationToken) {
    const TICK_MS: u64 = 80;
    const WINDOW_SECS: f32 = 0.05;
    let mut ticker = tokio::time::interval(std::time::Duration::from_millis(TICK_MS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = ticker.tick() => {}
        }
        let rms = audio::peek_level(&ring, WINDOW_SECS);
        let _ = app.emit_to(&win, "stt://level", LevelPayload { rms });
    }
}

#[tauri::command]
pub fn stt_get_input_devices() -> Result<Vec<String>, String> {
    audio::list_input_devices()
}

/// Build-time query — true if the Whisper FFI backend was compiled in. The
/// Settings UI uses this to gate the "engine: whisper" option and to show
/// the "install LLVM and rebuild" hint when it's false.
#[tauri::command]
pub fn stt_backend_available() -> bool {
    whisper::backend_available()
}

#[tauri::command]
pub fn stt_list_models() -> Result<Vec<model_manager::ModelInfo>, String> {
    Ok(model_manager::known_models())
}

/// Active download cancel flag — single-slot. Set true to abort the running
/// download; the streamer checks between chunks and aborts mid-flight. The
/// `.partial` file is preserved so the next `download` call resumes.
pub struct DownloadCancel(pub std::sync::Mutex<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>>);

#[tauri::command]
pub async fn stt_download_model(
    app: tauri::AppHandle,
    state: tauri::State<'_, DownloadCancel>,
    model_id: String,
) -> Result<(), String> {
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut slot = state.0.lock().map_err(|e| format!("cancel slot lock: {e}"))?;
        if let Some(prev) = slot.as_ref() {
            prev.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        *slot = Some(cancel.clone());
    }
    let res = model_manager::download(app, &model_id, cancel.clone()).await;
    {
        let mut slot = state.0.lock().map_err(|e| format!("cancel slot lock: {e}"))?;
        // Only clear if the slot still holds THIS call's cancel Arc — a newer
        // concurrent download may have already replaced it.
        if slot.as_ref().is_some_and(|s| std::sync::Arc::ptr_eq(s, &cancel)) {
            *slot = None;
        }
    }
    res
}

#[tauri::command]
pub fn stt_cancel_download(state: tauri::State<'_, DownloadCancel>) -> Result<(), String> {
    let slot = state.0.lock().map_err(|e| format!("cancel slot lock: {e}"))?;
    if let Some(c) = slot.as_ref() {
        c.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub fn stt_delete_model(model_id: String) -> Result<(), String> {
    model_manager::delete(&model_id)
}

#[tauri::command]
pub async fn stt_clean_transcript(text: String) -> Result<String, String> {
    cleanup::polish(&text).await
}
