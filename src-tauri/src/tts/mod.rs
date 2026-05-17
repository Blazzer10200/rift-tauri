//! Text-to-speech service.
//!
//! Wraps `msedge-tts` (Microsoft Edge read-aloud endpoint — free, no API key,
//! Azure Neural voices). Synthesises text to MP3 bytes server-side, emits
//! base64 chunks to the frontend, which plays them through HTMLAudioElement.
//!
//! Concurrency model: a single tokio task drains a request queue serially so
//! audio output is in the right order. `tts_speak` enqueues + returns
//! immediately. `tts_cancel` bumps a generation counter — any in-flight
//! synthesis emits nothing once the generation has moved past it, and the
//! queue is drained.
//!
//! Persistence: tiny JSON at `~/.rift/tts-config.json`. Defaults to disabled.

use base64::Engine;
use msedge_tts::tts::client::connect as tts_connect;
use msedge_tts::tts::SpeechConfig;
use msedge_tts::voice::{get_voices_list, Voice};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, Mutex};

/// Persisted user settings. Matches the same `~/.rift/*.json` ergonomics
/// as `assistant::AssistantConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// Master switch — when false, `tts_speak` is a no-op.
    #[serde(default)]
    pub enabled: bool,
    /// When true, the assistant pipeline auto-speaks each completed sentence
    /// as the model streams it. Independent of `enabled` — flipping enabled
    /// off mutes auto-speak too.
    #[serde(default)]
    pub auto_speak: bool,
    /// Edge voice name, e.g. `Microsoft Server Speech Text to Speech Voice (en-US, AriaNeural)`.
    /// Empty = use the bundled default in `default_voice_name()`.
    #[serde(default)]
    pub voice: String,
    /// Speech rate adjustment, percent. Range -100..+100; 0 = natural.
    #[serde(default)]
    pub rate: i32,
    /// Pitch adjustment, percent. Range -100..+100.
    #[serde(default)]
    pub pitch: i32,
    /// Output gain at synthesis time, percent. Range -100..+100. The frontend
    /// also has its own HTMLAudioElement.volume for live mute, applied
    /// multiplicatively on top.
    #[serde(default)]
    pub volume: i32,
    /// Forward-compat slot.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_speak: false,
            voice: String::new(),
            rate: 0,
            pitch: 0,
            volume: 0,
            extra: serde_json::Map::new(),
        }
    }
}

fn default_voice_name() -> &'static str {
    "Microsoft Server Speech Text to Speech Voice (en-US, AriaNeural)"
}

fn config_path() -> PathBuf {
    let home = dirs_home();
    home.join(".rift").join("tts-config.json")
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

fn load_config() -> TtsConfig {
    let path = config_path();
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return TtsConfig::default(),
    };
    if bytes.len() > 64 * 1024 {
        log::warn!("tts: config file unexpectedly large, ignoring");
        return TtsConfig::default();
    }
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn save_config(cfg: &TtsConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir ~/.rift: {e}"))?;
    }
    let json = serde_json::to_vec_pretty(cfg).map_err(|e| format!("serialise tts config: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write tts config: {e}"))
}

/// Cached voice list — Edge's `voices` endpoint returns ~500 voices; we fetch
/// once per process and serve subsequent requests from memory. The list is
/// stable per Edge release so cache-forever is fine.
static VOICE_CACHE: OnceLock<Mutex<Option<Vec<Voice>>>> = OnceLock::new();

fn voice_cache() -> &'static Mutex<Option<Vec<Voice>>> {
    VOICE_CACHE.get_or_init(|| Mutex::new(None))
}

#[derive(Debug)]
struct SpeakRequest {
    text: String,
    request_id: String,
    generation: u64,
}

/// Service handle stored in tauri::State. Holds the sender into the worker
/// task plus the generation counter used by cancel.
pub struct TtsService {
    tx: mpsc::UnboundedSender<SpeakRequest>,
    generation: Arc<AtomicU64>,
}

impl TtsService {
    pub fn start(app: AppHandle) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<SpeakRequest>();
        let generation = Arc::new(AtomicU64::new(0));
        let gen_for_worker = generation.clone();

        tauri::async_runtime::spawn(async move {
            while let Some(req) = rx.recv().await {
                // Drop stale requests cheaply before doing real work.
                if req.generation < gen_for_worker.load(Ordering::Acquire) {
                    let _ = app.emit(
                        "tts://cancelled",
                        serde_json::json!({ "request_id": req.request_id }),
                    );
                    continue;
                }

                let cfg = load_config();
                if !cfg.enabled {
                    let _ = app.emit(
                        "tts://cancelled",
                        serde_json::json!({ "request_id": req.request_id }),
                    );
                    continue;
                }

                let app_for_blocking = app.clone();
                let gen_at_dispatch = req.generation;
                let gen_check = gen_for_worker.clone();
                let join = tauri::async_runtime::spawn_blocking(move || {
                    synthesise_blocking(&cfg, &req.text)
                })
                .await;

                // Generation may have advanced while we were synthesising —
                // suppress the emit so cancel actually mutes the output.
                if gen_check.load(Ordering::Acquire) > gen_at_dispatch {
                    let _ = app_for_blocking.emit(
                        "tts://cancelled",
                        serde_json::json!({ "request_id": req.request_id }),
                    );
                    continue;
                }

                let err: Option<String> = match join {
                    Ok(Ok(bytes)) => {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        let _ = app_for_blocking.emit(
                            "tts://audio",
                            serde_json::json!({
                                "request_id": req.request_id,
                                "audio_b64": b64,
                                "mime": "audio/mpeg",
                            }),
                        );
                        None
                    }
                    Ok(Err(e)) => Some(e),
                    Err(e) => Some(format!("join: {e}")),
                };
                if let Some(e) = err {
                    log::warn!("tts: synthesis failed: {e}");
                    let _ = app_for_blocking.emit(
                        "tts://error",
                        serde_json::json!({
                            "request_id": req.request_id,
                            "error": e,
                        }),
                    );
                }
            }
        });

        Self { tx, generation }
    }
}

fn synthesise_blocking(cfg: &TtsConfig, text: &str) -> Result<Vec<u8>, String> {
    let mut client = tts_connect().map_err(|e| format!("connect to edge tts: {e}"))?;
    let speech = SpeechConfig {
        voice_name: if cfg.voice.is_empty() {
            default_voice_name().to_string()
        } else {
            cfg.voice.clone()
        },
        audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
        pitch: cfg.pitch.clamp(-100, 100),
        rate: cfg.rate.clamp(-100, 100),
        volume: cfg.volume.clamp(-100, 100),
    };
    let audio = client
        .synthesize(text, &speech)
        .map_err(|e| format!("synthesize: {e}"))?;
    Ok(audio.audio_bytes)
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn tts_get_config() -> Result<TtsConfig, String> {
    Ok(load_config())
}

#[tauri::command]
pub fn tts_set_config(config: TtsConfig) -> Result<(), String> {
    save_config(&config)
}

/// Curated `(name, label, locale)` tuples surfaced in Settings. Filtered down
/// from the ~500 Edge voices to the conversational-English Neural set most
/// users want. Frontend can still pick an arbitrary voice via `tts_list_voices`.
#[derive(Debug, Serialize)]
pub struct VoiceSummary {
    pub name: String,
    pub short_name: String,
    pub locale: String,
    pub gender: Option<String>,
}

#[tauri::command]
pub async fn tts_list_voices() -> Result<Vec<VoiceSummary>, String> {
    let cache = voice_cache();
    {
        let guard = cache.lock().await;
        if let Some(v) = guard.as_ref() {
            return Ok(summarise(v));
        }
    }
    let fetched = tauri::async_runtime::spawn_blocking(get_voices_list)
        .await
        .map_err(|e| format!("voice list join: {e}"))?
        .map_err(|e| format!("voice list: {e}"))?;
    let summary = summarise(&fetched);
    let mut guard = cache.lock().await;
    *guard = Some(fetched);
    Ok(summary)
}

fn summarise(voices: &[Voice]) -> Vec<VoiceSummary> {
    let mut out: Vec<VoiceSummary> = voices
        .iter()
        .map(|v| {
            let locale = v.locale.clone().unwrap_or_default();
            let short = v
                .short_name
                .clone()
                .unwrap_or_else(|| v.name.clone());
            let gender = v.gender.clone();
            VoiceSummary {
                name: v.name.clone(),
                short_name: short,
                locale,
                gender,
            }
        })
        .collect();
    // Surface English locales first, then alphabetical within each group.
    out.sort_by(|a, b| {
        let ae = a.locale.starts_with("en-");
        let be = b.locale.starts_with("en-");
        match (ae, be) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.short_name.cmp(&b.short_name),
        }
    });
    out
}

#[tauri::command]
pub fn tts_speak(
    text: String,
    request_id: String,
    state: tauri::State<'_, TtsService>,
) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let generation = state.generation.load(Ordering::Acquire);
    state
        .tx
        .send(SpeakRequest {
            text: trimmed.to_string(),
            request_id,
            generation,
        })
        .map_err(|e| format!("tts queue closed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn tts_cancel(state: tauri::State<'_, TtsService>) -> Result<(), String> {
    // Bump generation — drains queued items + suppresses in-flight emit.
    state.generation.fetch_add(1, Ordering::AcqRel);
    Ok(())
}
