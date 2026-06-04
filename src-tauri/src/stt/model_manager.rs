//! Whisper GGML model registry + downloader. Models live in `~/.rift/models/`,
//! pulled from `huggingface.co/ggerganov/whisper.cpp` via `reqwest` streaming.
//! Resumable via HTTP `Range`; atomic via `.partial`+rename; sha256 verified
//! against a hash table baked in below (sourced from the upstream
//! `download-ggml-model.sh` manifest).

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

const HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub filename: String,
    /// Approximate display size in bytes (for UI labels only — the real size
    /// of the downloaded file is whatever HF returned). Don't compare against
    /// `metadata().len()` for completion-check.
    pub approx_size_bytes: u64,
    /// On-disk size (None if not downloaded).
    pub on_disk_bytes: Option<u64>,
    pub downloaded: bool,
    pub path: Option<PathBuf>,
    /// Expected sha256 (lowercase hex). `None` means "skip verify" — true for
    /// every entry today until we pin upstream hashes.
    pub sha256: Option<String>,
}

/// On-disk catalogue of every model Rift knows how to fetch. To add another:
/// append a row here and the Settings UI picks it up via `stt_list_models`.
fn catalogue() -> Vec<ModelEntry> {
    vec![
        ModelEntry {
            id: "large-v3-turbo-q5_0",
            display_name: "Large v3 Turbo (Q5, ~574 MB) — recommended for RTX GPUs",
            filename: "ggml-large-v3-turbo-q5_0.bin",
            approx_size_bytes: 574_041_195,
            sha256: Some("394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2"),
        },
        ModelEntry {
            id: "large-v3-turbo-q8_0",
            display_name: "Large v3 Turbo (Q8, ~874 MB) — sharper vocab, RTX 8 GB+",
            filename: "ggml-large-v3-turbo-q8_0.bin",
            approx_size_bytes: 874_188_075,
            sha256: Some("317eb69c11673c9de1e1f0d459b253999804ec71ac4c23c17ecf5fbe24e259a1"),
        },
        ModelEntry {
            id: "large-v3-turbo-f16",
            display_name: "Large v3 Turbo (f16, ~1.6 GB) — reference quality, RTX 8 GB+ GPU",
            filename: "ggml-large-v3-turbo.bin",
            approx_size_bytes: 1_624_555_275,
            sha256: Some("1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69"),
        },
        ModelEntry {
            id: "medium-q5_0",
            display_name: "Medium (Q5, ~514 MB) — CPU fallback",
            filename: "ggml-medium-q5_0.bin",
            approx_size_bytes: 539_212_467,
            sha256: Some("19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f"),
        },
    ]
}

struct ModelEntry {
    id: &'static str,
    display_name: &'static str,
    filename: &'static str,
    approx_size_bytes: u64,
    sha256: Option<&'static str>,
}

pub fn models_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".rift").join("models")
}

pub fn known_models() -> Vec<ModelInfo> {
    let dir = models_dir();
    catalogue()
        .into_iter()
        .map(|m| {
            let path = dir.join(m.filename);
            let md = path.metadata().ok();
            let on_disk_bytes = md.as_ref().map(|x| x.len());
            // Completion-check is "the final filename exists and is non-empty"
            // — the downloader only renames `.partial → final` on success, so
            // file presence implies a clean download. Strict integrity will
            // come once we pin sha256 (catalogue entries are `None` today).
            let downloaded = on_disk_bytes.map(|b| b > 0).unwrap_or(false);
            ModelInfo {
                id: m.id.to_string(),
                display_name: m.display_name.to_string(),
                filename: m.filename.to_string(),
                approx_size_bytes: m.approx_size_bytes,
                on_disk_bytes,
                downloaded,
                path: Some(path),
                sha256: m.sha256.map(String::from),
            }
        })
        .collect()
}

fn entry_for(id: &str) -> Option<ModelEntry> {
    catalogue().into_iter().find(|m| m.id == id)
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    model: String,
    downloaded: u64,
    total: u64,
    /// `"start" | "progress" | "done" | "error"`.
    phase: &'static str,
    message: Option<String>,
}

/// Stream-downloads the model w/ `Range`-based resume. Emits
/// `stt://download_progress` events ~10x/s throttled. Atomic via .partial +
/// rename on success. Aborts on `cancel` flag; partial file is preserved so a
/// later `download` call can resume.
pub async fn download(
    app: AppHandle,
    model_id: &str,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let entry = entry_for(model_id).ok_or_else(|| format!("unknown model id: {model_id}"))?;
    let dir = models_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    let final_path = dir.join(entry.filename);
    let partial_path = dir.join(format!("{}.partial", entry.filename));

    // Already complete? Final-filename presence is the completion marker —
    // the downloader only renames `.partial → final` after a clean stream end.
    if final_path.exists() {
        if let Ok(md) = final_path.metadata() {
            if md.len() > 0 {
                emit_progress(&app, entry.id, md.len(), md.len(), "done", None);
                return Ok(());
            }
        }
    }

    // Resume offset = current size of .partial (0 if absent).
    let resume_from = partial_path.metadata().map(|m| m.len()).unwrap_or(0);

    let url = format!("{HF_BASE}/{}", entry.filename);
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;
    let mut req = client.get(&url);
    if resume_from > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("download request failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() && status.as_u16() != 206 {
        return Err(format!("HF returned HTTP {status} for {url}"));
    }

    // Append-mode if resuming, create-truncate otherwise.
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(resume_from > 0)
        .write(true)
        .truncate(resume_from == 0)
        .open(&partial_path)
        .map_err(|e| format!("open {}: {e}", partial_path.display()))?;

    // Total bytes — Content-Length on a 206 is the REMAINING bytes; on a 200
    // it's the FULL size. Combine to derive the canonical total for the
    // progress bar; fall back to approx if absent.
    let remote_len = resp.content_length();
    let total = match (status.as_u16(), remote_len) {
        (206, Some(remaining)) => resume_from.saturating_add(remaining),
        (_, Some(full)) => full,
        _ => entry.approx_size_bytes,
    };

    emit_progress(&app, entry.id, resume_from, total, "start", None);

    let mut downloaded = resume_from;
    let mut last_emit = std::time::Instant::now();
    let mut stream = resp.bytes_stream();
    let mut hasher = if entry.sha256.is_some() && resume_from == 0 {
        Some(Sha256::new())
    } else {
        None
    };

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            emit_progress(
                &app,
                entry.id,
                downloaded,
                total,
                "error",
                Some("cancelled".into()),
            );
            return Err("download cancelled".into());
        }
        let bytes = chunk.map_err(|e| format!("stream read failed: {e}"))?;
        file.write_all(&bytes)
            .map_err(|e| format!("write {}: {e}", partial_path.display()))?;
        if let Some(h) = hasher.as_mut() {
            h.update(&bytes);
        }
        downloaded = downloaded.saturating_add(bytes.len() as u64);
        if last_emit.elapsed() >= std::time::Duration::from_millis(100) {
            emit_progress(&app, entry.id, downloaded, total, "progress", None);
            last_emit = std::time::Instant::now();
        }
    }
    file.flush().map_err(|e| format!("flush: {e}"))?;
    drop(file);

    let on_disk = partial_path
        .metadata()
        .map(|m| m.len())
        .map_err(|e| format!("stat partial: {e}"))?;

    // SHA256 verify (only when we hashed the whole stream — fresh download,
    // not a resume). Catalogue entries have `sha256: None` today, so this
    // branch is dormant until we pin upstream hashes.
    if let (Some(expected), Some(h)) = (entry.sha256, hasher) {
        let got = format!("{:x}", h.finalize());
        if got != expected {
            let _ = std::fs::rename(
                &partial_path,
                dir.join(format!("{}.badhash", entry.filename)),
            );
            return Err(format!(
                "sha256 mismatch for {}: got {got}, expected {expected}",
                entry.filename
            ));
        }
    }

    std::fs::rename(&partial_path, &final_path)
        .map_err(|e| format!("promote partial → final: {e}"))?;
    emit_progress(&app, entry.id, on_disk, on_disk, "done", None);
    Ok(())
}

fn emit_progress(
    app: &AppHandle,
    model_id: &str,
    downloaded: u64,
    total: u64,
    phase: &'static str,
    message: Option<String>,
) {
    let _ = app.emit(
        "stt://download_progress",
        ProgressPayload {
            model: model_id.to_string(),
            downloaded,
            total,
            phase,
            message,
        },
    );
}

pub fn delete(model_id: &str) -> Result<(), String> {
    let entry = entry_for(model_id).ok_or_else(|| format!("unknown model id: {model_id}"))?;
    let path = models_dir().join(entry.filename);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("delete failed: {e}"))?;
    }
    Ok(())
}
