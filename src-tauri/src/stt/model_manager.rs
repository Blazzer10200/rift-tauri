//! STT model registry + downloader. Models live in `~/.rift/models/`, pulled
//! from HuggingFace via `reqwest` streaming. Resumable via HTTP `Range`;
//! atomic via `.partial`+rename; sha256 verified against hashes baked in below.
//!
//! Two engine families share the registry:
//!   * Whisper GGML — single `.bin` at the models root (paths predate the
//!     multi-file support; keeping them there keeps old downloads valid).
//!     Hashes sourced from upstream `download-ggml-model.sh`.
//!   * Parakeet ONNX — a per-model subdirectory holding the encoder/decoder
//!     ONNX pair + vocab (filenames match the HF repo verbatim; parakeet-rs
//!     probes the `.int8.onnx` names natively). Hashes from the HF LFS API.

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

const WHISPER_HF_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
const PARAKEET_HF_BASE: &str =
    "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Whisper,
    Parakeet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub engine: EngineKind,
    /// Primary on-disk name shown in the UI — the single GGML file for
    /// Whisper, the model subdirectory for Parakeet.
    pub filename: String,
    /// Approximate display size in bytes (sum over files, for UI labels only —
    /// the real size is whatever HF returned). Don't compare against
    /// `metadata().len()` for completion-check.
    pub approx_size_bytes: u64,
    /// On-disk size (None if nothing downloaded yet).
    pub on_disk_bytes: Option<u64>,
    pub downloaded: bool,
    /// Load path handed to the engine: GGML file (Whisper) or dir (Parakeet).
    pub path: Option<PathBuf>,
}

struct FileSpec {
    /// Filename on HF *and* on disk (identical, keeps resume/verify 1:1).
    name: &'static str,
    approx_size_bytes: u64,
    sha256: Option<&'static str>,
}

struct ModelEntry {
    id: &'static str,
    display_name: &'static str,
    engine: EngineKind,
    hf_base: &'static str,
    /// Subdirectory under the models root (multi-file models). None = root.
    subdir: Option<&'static str>,
    files: &'static [FileSpec],
}

/// On-disk catalogue of every model Rift knows how to fetch. To add another:
/// append a row here and the Settings UI picks it up via `stt_list_models`.
fn catalogue() -> Vec<ModelEntry> {
    vec![
        ModelEntry {
            id: "parakeet-tdt-0.6b-v3-int8",
            display_name: "Parakeet TDT 0.6B v3 (int8, ~670 MB) — recommended",
            engine: EngineKind::Parakeet,
            hf_base: PARAKEET_HF_BASE,
            subdir: Some("parakeet-tdt-0.6b-v3-int8"),
            files: &[
                FileSpec {
                    name: "encoder-model.int8.onnx",
                    approx_size_bytes: 652_183_999,
                    sha256: Some("6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09"),
                },
                FileSpec {
                    name: "decoder_joint-model.int8.onnx",
                    approx_size_bytes: 18_202_004,
                    sha256: Some("eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70"),
                },
                FileSpec {
                    name: "vocab.txt",
                    approx_size_bytes: 93_939,
                    sha256: Some("d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d"),
                },
            ],
        },
        ModelEntry {
            id: "large-v3-turbo-q5_0",
            display_name: "Whisper Large v3 Turbo (Q5, ~574 MB) — multilingual",
            engine: EngineKind::Whisper,
            hf_base: WHISPER_HF_BASE,
            subdir: None,
            files: &[FileSpec {
                name: "ggml-large-v3-turbo-q5_0.bin",
                approx_size_bytes: 574_041_195,
                sha256: Some("394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2"),
            }],
        },
        ModelEntry {
            id: "large-v3-turbo-q8_0",
            display_name: "Whisper Large v3 Turbo (Q8, ~874 MB) — sharper vocab, RTX 8 GB+",
            engine: EngineKind::Whisper,
            hf_base: WHISPER_HF_BASE,
            subdir: None,
            files: &[FileSpec {
                name: "ggml-large-v3-turbo-q8_0.bin",
                approx_size_bytes: 874_188_075,
                sha256: Some("317eb69c11673c9de1e1f0d459b253999804ec71ac4c23c17ecf5fbe24e259a1"),
            }],
        },
        ModelEntry {
            id: "large-v3-turbo-f16",
            display_name: "Whisper Large v3 Turbo (f16, ~1.6 GB) — reference quality, RTX 8 GB+",
            engine: EngineKind::Whisper,
            hf_base: WHISPER_HF_BASE,
            subdir: None,
            files: &[FileSpec {
                name: "ggml-large-v3-turbo.bin",
                approx_size_bytes: 1_624_555_275,
                sha256: Some("1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69"),
            }],
        },
        ModelEntry {
            id: "medium-q5_0",
            display_name: "Whisper Medium (Q5, ~514 MB) — CPU fallback",
            engine: EngineKind::Whisper,
            hf_base: WHISPER_HF_BASE,
            subdir: None,
            files: &[FileSpec {
                name: "ggml-medium-q5_0.bin",
                approx_size_bytes: 539_212_467,
                sha256: Some("19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f"),
            }],
        },
    ]
}

fn models_dir() -> PathBuf {
    // Resolve via the canonical USERPROFILE→HOME helper. Fallback is LOCALAPPDATA
    // (persistent + outside the Velopack `current/` dir, so not wiped on update),
    // then temp only as a true last resort. NOT a CWD-relative path (would land
    // next to the exe under a Velopack install and vanish on every update).
    let base = crate::state::paths::dirs_home()
        .ok()
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join(".rift").join("models")
}

impl ModelEntry {
    /// Directory the model's files land in.
    fn dir(&self) -> PathBuf {
        let root = models_dir();
        match self.subdir {
            Some(s) => root.join(s),
            None => root,
        }
    }

    fn total_size(&self) -> u64 {
        self.files.iter().map(|f| f.approx_size_bytes).sum()
    }
}

pub fn known_models() -> Vec<ModelInfo> {
    catalogue()
        .into_iter()
        .map(|m| {
            let dir = m.dir();
            let mut on_disk: u64 = 0;
            let mut all_present = true;
            for f in m.files {
                // Completion-check is "the final filename exists and is non-empty"
                // — the downloader only renames `.partial → final` on success, so
                // file presence implies a clean download.
                match dir.join(f.name).metadata().ok().map(|x| x.len()) {
                    Some(len) if len > 0 => on_disk += len,
                    _ => all_present = false,
                }
            }
            let path = match m.engine {
                EngineKind::Whisper => dir.join(m.files[0].name),
                EngineKind::Parakeet => dir.clone(),
            };
            ModelInfo {
                id: m.id.to_string(),
                display_name: m.display_name.to_string(),
                engine: m.engine,
                filename: match m.subdir {
                    Some(s) => s.to_string(),
                    None => m.files[0].name.to_string(),
                },
                approx_size_bytes: m.total_size(),
                on_disk_bytes: if on_disk > 0 { Some(on_disk) } else { None },
                downloaded: all_present,
                path: Some(path),
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

/// Hash an on-disk file with SHA256, hex-encoded.
fn file_sha256(path: &std::path::Path) -> std::io::Result<String> {
    let mut h = Sha256::new();
    let mut f = std::fs::File::open(path)?;
    std::io::copy(&mut f, &mut h)?;
    Ok(format!("{:x}", h.finalize()))
}

/// Stream-downloads every file of the model w/ `Range`-based resume. Emits
/// `stt://download_progress` events ~10x/s throttled with byte counts
/// aggregated across the whole file set. Atomic via .partial + rename on
/// success. Aborts on `cancel` flag; partial files are preserved so a later
/// `download` call can resume.
pub async fn download(
    app: AppHandle,
    model_id: &str,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    match download_inner(&app, model_id, cancel).await {
        Ok(()) => Ok(()),
        Err(e) => {
            // Every failure path below (network/write/hash/etc) must still
            // unstick the Settings progress bar — only the explicit cancel
            // branch inside download_file emits its own "error" phase.
            if e != "download cancelled" {
                emit_progress(&app, model_id, 0, 0, "error", Some(e.clone()));
            }
            Err(e)
        }
    }
}

async fn download_inner(
    app: &AppHandle,
    model_id: &str,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let entry = entry_for(model_id).ok_or_else(|| format!("unknown model id: {model_id}"))?;
    let dir = entry.dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;

    let total: u64 = entry.total_size();
    emit_progress(app, entry.id, 0, total, "start", None);

    // Bytes fully settled by prior files in this loop — per-file progress is
    // emitted as `base + file_progress` so the bar is monotonic across the set.
    let mut base: u64 = 0;
    for f in entry.files {
        download_file(app, entry.id, entry.hf_base, &dir, f, base, total, cancel.clone()).await?;
        base += f.approx_size_bytes;
    }

    emit_progress(app, entry.id, total, total, "done", None);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn download_file(
    app: &AppHandle,
    model_id: &str,
    hf_base: &str,
    dir: &std::path::Path,
    spec: &FileSpec,
    progress_base: u64,
    progress_total: u64,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let final_path = dir.join(spec.name);
    let partial_path = dir.join(format!("{}.partial", spec.name));

    // Already complete? Final-filename presence is the completion marker — the
    // downloader only renames `.partial → final` after a clean stream end. Still
    // re-verify SHA256 of the on-disk file before trusting it: a corrupted or
    // foreign file planted at this path would otherwise be served forever, since
    // the stream-hash only runs during an actual download.
    if final_path.exists() {
        if let Ok(md) = final_path.metadata() {
            if md.len() > 0 {
                let verified = match spec.sha256 {
                    Some(expected) => {
                        file_sha256(&final_path).map(|g| g == expected).unwrap_or(false)
                    }
                    None => true,
                };
                if verified {
                    return Ok(());
                }
                // Failed verification — quarantine and fall through to re-download.
                let _ = std::fs::rename(
                    &final_path,
                    dir.join(format!("{}.badhash", spec.name)),
                );
            }
        }
    }

    // Resume offset = current size of .partial (0 if absent).
    let resume_from = partial_path.metadata().map(|m| m.len()).unwrap_or(0);

    let url = format!("{hf_base}/{}", spec.name);
    let client = crate::certs::download_client();
    let mut req = client.get(&url);
    if resume_from > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("could not reach huggingface.co (check your network/proxy/firewall): {e}"))?;
    let status = resp.status();
    // 416 = Range Not Satisfiable: .partial already covers the full file.
    if status.as_u16() == 416 {
        // Verify the full .partial before promoting — a truncated/corrupt resume
        // must not be silently accepted (mirrors the post-rename verify below).
        if let Some(expected) = spec.sha256 {
            let got = file_sha256(&partial_path)
                .map_err(|e| format!("416 verify: {e}"))?;
            if got != expected {
                let _ = std::fs::rename(
                    &partial_path,
                    dir.join(format!("{}.badhash", spec.name)),
                );
                return Err(format!(
                    "sha256 mismatch (416 resume) for {}: got {got}, expected {expected}",
                    spec.name
                ));
            }
        }
        std::fs::rename(&partial_path, &final_path)
            .map_err(|e| format!("promote partial -> final (416 path): {e}"))?;
        return Ok(());
    }
    if !status.is_success() && status.as_u16() != 206 {
        return Err(format!("HF returned HTTP {status} for {url}"));
    }

    // Server ignored our Range header and sent the full body (200) instead of
    // a partial (206) — restart from scratch or the full stream gets appended
    // after the existing partial bytes, corrupting the file.
    let resume_from = if resume_from > 0 && status.as_u16() != 206 {
        0
    } else {
        resume_from
    };

    // Append-mode if resuming, create-truncate otherwise.
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(resume_from > 0)
        .write(true)
        .truncate(resume_from == 0)
        .open(&partial_path)
        .map_err(|e| format!("open {}: {e}", partial_path.display()))?;

    let mut downloaded = resume_from;
    let mut last_emit = std::time::Instant::now();
    let mut stream = resp.bytes_stream();
    let mut hasher = if spec.sha256.is_some() && resume_from == 0 {
        Some(Sha256::new())
    } else {
        None
    };
    // For resumed downloads sha256 will be verified post-stream (full-file pass below).

    // Stall guard (mirrors commands/update.rs): without it a half-open HF
    // connection hangs the download forever with no progress or error.
    const STALL_SECS: u64 = 90;
    loop {
        let next = match tokio::time::timeout(
            std::time::Duration::from_secs(STALL_SECS),
            stream.next(),
        )
        .await
        {
            Ok(n) => n,
            Err(_) => {
                emit_progress(
                    app,
                    model_id,
                    progress_base.saturating_add(downloaded),
                    progress_total,
                    "error",
                    Some(format!("stalled — no data for {STALL_SECS}s")),
                );
                return Err(format!(
                    "download stalled — no data received for {STALL_SECS}s"
                ));
            }
        };
        let Some(chunk) = next else { break };
        if cancel.load(Ordering::Relaxed) {
            emit_progress(
                app,
                model_id,
                progress_base.saturating_add(downloaded),
                progress_total,
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
            emit_progress(
                app,
                model_id,
                progress_base.saturating_add(downloaded),
                progress_total,
                "progress",
                None,
            );
            last_emit = std::time::Instant::now();
        }
    }
    file.flush().map_err(|e| format!("flush: {e}"))?;
    drop(file);

    // SHA256 verify (only when we hashed the whole stream — fresh download, not a resume).
    let was_hashed = hasher.is_some();
    if let (Some(expected), Some(h)) = (spec.sha256, hasher) {
        let got = format!("{:x}", h.finalize());
        if got != expected {
            let _ = std::fs::rename(
                &partial_path,
                dir.join(format!("{}.badhash", spec.name)),
            );
            return Err(format!(
                "sha256 mismatch for {}: got {got}, expected {expected}",
                spec.name
            ));
        }
    }

    // Verify resumed downloads (hasher was None — we didn't stream-hash the
    // pre-existing bytes) on the .partial BEFORE the rename. Verifying after the
    // rename leaves a corrupt file sitting at final_path as a valid completion
    // marker — known_models() trusts presence+size with no hash check, so a
    // crash/concurrent call in that window would serve a bad model. Mirror the
    // fresh-download path: verify, then promote.
    if !was_hashed {
        if let Some(expected) = spec.sha256 {
            let got = file_sha256(&partial_path)
                .map_err(|e| format!("pre-rename verify: {e}"))?;
            if got != expected {
                let _ = std::fs::rename(
                    &partial_path,
                    dir.join(format!("{}.badhash", spec.name)),
                );
                return Err(format!(
                    "sha256 mismatch (resumed) for {}: got {got}, expected {expected}",
                    spec.name
                ));
            }
        }
    }

    std::fs::rename(&partial_path, &final_path)
        .map_err(|e| format!("promote partial → final: {e}"))?;
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
    let dir = entry.dir();
    for f in entry.files {
        let path = dir.join(f.name);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("delete failed: {e}"))?;
        }
    }
    // Multi-file models own their subdir — remove it once emptied (leftover
    // .partial/.badhash files keep it alive on purpose).
    if entry.subdir.is_some() {
        let _ = std::fs::remove_dir(&dir);
    }
    Ok(())
}
