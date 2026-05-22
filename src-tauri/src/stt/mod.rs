//! Speech-to-text — engine routing + config persistence.
//!
//! Two engines coexist:
//!   * `web_speech` — legacy. Recognition runs in the WebView via the Web
//!     Speech API (Edge's Azure-backed recogniser). No Rust audio path; Rust
//!     only persists settings.
//!   * `whisper`    — local Whisper Large v3 Turbo via whisper-rs on CUDA,
//!     gated by Silero VAD, optionally polished by Claude Haiku. Rust owns
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
     endings, and slurred consonants are normal. Transcribe what was meant, \
     not a literal phonetic reading.";

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
            extra: serde_json::Map::new(),
        }
    }
}

fn dirs_home() -> PathBuf {
    if let Some(p) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        return p;
    }
    if let Some(p) = std::env::var_os("HOME").map(PathBuf::from) {
        return p;
    }
    PathBuf::from(".")
}

fn config_path() -> PathBuf {
    dirs_home().join(".rift").join("stt-config.json")
}

fn load_config() -> SttConfig {
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

#[tauri::command]
pub fn stt_set_engine(engine: String) -> Result<(), String> {
    if engine != "web_speech" && engine != "whisper" {
        return Err(format!("unknown engine: {engine}"));
    }
    let mut cfg = load_config();
    cfg.engine = engine;
    save_config(&cfg)
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
    initial_prompt: String,
    cleanup_enabled: bool,
    rolling_cancel: CancellationToken,
    /// Dropping sends a stop signal; capture thread drops cpal::Stream, mic
    /// closes. Sender is `Option` so `stop` can take + drop it explicitly.
    capture_stop: Option<std::sync::mpsc::Sender<()>>,
    capture_handle: Option<std::thread::JoinHandle<()>>,
}

impl ActiveSession {
    fn shutdown_capture(&mut self) {
        self.rolling_cancel.cancel();
        if let Some(tx) = self.capture_stop.take() {
            let _ = tx.send(());
            drop(tx);
        }
        if let Some(h) = self.capture_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for ActiveSession {
    fn drop(&mut self) {
        self.shutdown_capture();
    }
}

#[derive(Clone, Serialize)]
struct PartialPayload {
    text: String,
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

fn emit_state(app: &AppHandle, state: &'static str, msg: Option<String>) {
    let _ = app.emit("stt://state", StatePayload { state, message: msg });
}

fn emit_error(app: &AppHandle, code: &str, message: &str) {
    let _ = app.emit(
        "stt://error",
        json!({ "code": code, "message": message }),
    );
}

#[tauri::command]
pub async fn stt_start_recording(
    app: AppHandle,
    cache: tauri::State<'_, WhisperCache>,
    session: tauri::State<'_, WhisperSession>,
    model: Option<String>,
) -> Result<(), String> {
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
    let engine = {
        let mut slot = cache.0.lock().await;
        let need_reload = match slot.as_ref() {
            Some(e) => e.model_id != model_id,
            None => true,
        };
        if need_reload {
            emit_state(&app, "loading_model", Some(model_id.clone()));
            let model_id_owned = model_id.clone();
            let loaded = tokio::task::spawn_blocking(move || {
                whisper::WhisperEngine::load(&model_path, &model_id_owned)
            })
            .await
            .map_err(|e| format!("model load task join: {e}"))??;
            *slot = Some(loaded);
        }
        slot.as_ref().expect("engine just loaded").clone()
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

    let ring = cap_ready_rx
        .recv()
        .map_err(|e| format!("capture init channel: {e}"))??;

    let initial_prompt = whisper::compose_prompt(&cfg.initial_prompt, &cfg.vocab_text);
    let cleanup_enabled = cfg.cleanup_enabled;
    let rolling_cancel = CancellationToken::new();

    // Spawn rolling-window transcribe task.
    if cfg.show_interim {
        let task_app = app.clone();
        let task_ring = ring.clone();
        let task_engine = engine.clone();
        let task_prompt = initial_prompt.clone();
        let task_cancel = rolling_cancel.clone();
        tokio::spawn(async move {
            rolling_window_loop(task_app, task_ring, task_engine, task_prompt, task_cancel).await;
        });
    }

    {
        let mut slot = session.0.lock().await;
        *slot = Some(ActiveSession {
            ring,
            initial_prompt,
            cleanup_enabled,
            rolling_cancel,
            capture_stop: Some(cap_stop_tx),
            capture_handle: Some(capture_handle),
        });
    }
    emit_state(&app, "recording", None);
    Ok(())
}

#[tauri::command]
pub async fn stt_stop_recording(
    app: AppHandle,
    cache: tauri::State<'_, WhisperCache>,
    session: tauri::State<'_, WhisperSession>,
) -> Result<String, String> {
    let mut active = {
        let mut slot = session.0.lock().await;
        slot.take()
    }
    .ok_or_else(|| "no stt session active".to_string())?;

    active.shutdown_capture();
    emit_state(&app, "transcribing", None);

    // Drain final buffer, transcribe.
    let samples = audio::drain_all(&active.ring);
    let engine = {
        let slot = cache.0.lock().await;
        slot.as_ref()
            .cloned()
            .ok_or_else(|| "whisper engine missing — model unloaded mid-session".to_string())?
    };
    let prompt = active.initial_prompt.clone();
    let raw = tokio::task::spawn_blocking(move || engine.transcribe(&samples, &prompt))
        .await
        .map_err(|e| format!("final transcribe join: {e}"))??;

    let scrubbed = vad::strip_hallucinations(&raw);

    let final_text = if active.cleanup_enabled && !scrubbed.is_empty() {
        match cleanup::polish(&scrubbed).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("[stt] cleanup hop failed, returning raw: {e}");
                scrubbed.clone()
            }
        }
    } else {
        scrubbed.clone()
    };

    let _ = app.emit(
        "stt://final",
        FinalPayload {
            text: final_text.clone(),
            raw: scrubbed,
            cleaned: active.cleanup_enabled,
        },
    );
    emit_state(&app, "idle", None);
    Ok(final_text)
}

/// Rolling-window partial-emit loop. Ticks every 1 s; peeks the most-recent
/// 3 s of audio, runs VAD, transcribes if speech is present, emits the result
/// as `stt://partial` only when it differs from the previous emission (avoids
/// flooding the frontend w/ identical text).
async fn rolling_window_loop(
    app: AppHandle,
    ring: audio::AudioRing,
    engine: whisper::WhisperEngine,
    initial_prompt: String,
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
        let raw = match tokio::task::spawn_blocking(move || {
            engine_c.transcribe(&samples, &prompt_c)
        })
        .await
        {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => {
                emit_error(&app, "transcribe_failed", &e);
                continue;
            }
            Err(e) => {
                emit_error(&app, "task_join_failed", &e.to_string());
                continue;
            }
        };
        let scrubbed = vad::strip_hallucinations(&raw);
        if scrubbed.is_empty() || scrubbed == last_emitted {
            continue;
        }
        last_emitted = scrubbed.clone();
        let _ = app.emit("stt://partial", PartialPayload { text: scrubbed });
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
    let res = model_manager::download(app, &model_id, cancel).await;
    {
        let mut slot = state.0.lock().map_err(|e| format!("cancel slot lock: {e}"))?;
        *slot = None;
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
