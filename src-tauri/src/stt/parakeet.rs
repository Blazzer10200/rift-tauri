//! Parakeet TDT inference. Built on `parakeet-rs` (ONNX Runtime via `ort`,
//! prebuilt binaries — no LLVM) when the `parakeet` Cargo feature is enabled;
//! otherwise a stub that returns a clear error. Public surface mirrors
//! `whisper.rs` so `mod.rs` orchestration treats both engines alike.
//!
//! Unlike Whisper there is no `initial_prompt` / beam-search knob — vocabulary
//! correction is carried by the Claude cleanup pass instead.

use std::path::Path;
use std::sync::Arc;

// ----------------------------------------------------------------------------
// Real implementation — parakeet-rs (DirectML EP, automatic CPU fallback)
// ----------------------------------------------------------------------------
#[cfg(feature = "parakeet")]
mod real {
    use super::*;
    use parakeet_rs::{ExecutionConfig, ExecutionProvider, ParakeetTDT, Transcriber};
    use std::sync::Mutex;

    // `transcribe_samples` takes `&mut self`, so the loaded model sits behind a
    // Mutex; the rolling-partial loop and the final transcribe serialize on it
    // (they'd contend for the same GPU anyway).
    #[derive(Clone)]
    pub struct ParakeetEngine {
        inner: Arc<Mutex<ParakeetTDT>>,
        pub model_id: String,
    }

    impl ParakeetEngine {
        /// Load the ONNX model set from `model_dir`. Blocking — call from
        /// spawn_blocking. DirectML EP with parakeet-rs's automatic CPU
        /// fallback when no compatible GPU is present.
        pub fn load(model_dir: &Path, model_id: &str) -> Result<Self, String> {
            let config =
                ExecutionConfig::new().with_execution_provider(ExecutionProvider::DirectML);
            let load_t0 = std::time::Instant::now();
            let model = ParakeetTDT::from_pretrained(model_dir, Some(config))
                .map_err(|e| format!("load parakeet model '{}': {e}", model_dir.display()))?;
            let load_ms = load_t0.elapsed().as_millis() as u64;
            log::info!(
                "[stt] parakeet model loaded: id={model_id} dir={}",
                model_dir.display()
            );
            crate::diagnostics::emit_with_fields(
                crate::diagnostics::DiagStage::Log,
                crate::diagnostics::DiagLevel::Info,
                Some("stt"), Some(file!()),
                "parakeet model loaded",
                serde_json::json!({
                    "event": "model_load",
                    "model": model_id,
                    "load_ms": load_ms,
                    "ok": true,
                    "backend": "directml"
                }),
            );
            Ok(Self {
                inner: Arc::new(Mutex::new(model)),
                model_id: model_id.to_string(),
            })
        }

        /// Transcribe 16 kHz mono f32 samples. Blocking.
        pub fn transcribe(&self, samples: &[f32]) -> Result<String, String> {
            if samples.is_empty() {
                return Ok(String::new());
            }
            let mut model = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let infer_t0 = std::time::Instant::now();
            let infer_res = model
                .transcribe_samples(samples.to_vec(), 16_000, 1, None)
                .map_err(|e| format!("parakeet transcribe: {e}"));
            let infer_ms = infer_t0.elapsed().as_millis() as u64;
            crate::diagnostics::emit_with_fields(
                crate::diagnostics::DiagStage::Log,
                crate::diagnostics::DiagLevel::Debug,
                Some("stt"), Some(file!()),
                "parakeet inference",
                serde_json::json!({ "event": "inference", "infer_ms": infer_ms, "ok": infer_res.is_ok() }),
            );
            infer_res.map(|r| r.text.trim().to_string())
        }
    }
}

// ----------------------------------------------------------------------------
// Stub — used when the `parakeet` Cargo feature is off
// ----------------------------------------------------------------------------
#[cfg(not(feature = "parakeet"))]
mod stub {
    use super::*;

    const NOT_BUILT_MSG: &str = "Parakeet backend not built. Rebuild Rift with \
the default feature set (`cargo build --release`) or add `--features parakeet`. \
The Web Speech engine remains available in Settings → Speech.";

    #[derive(Clone)]
    pub struct ParakeetEngine {
        #[allow(dead_code)]
        pub model_id: String,
        #[allow(dead_code)]
        _marker: Arc<()>,
    }

    impl ParakeetEngine {
        pub fn load(_model_dir: &Path, _model_id: &str) -> Result<Self, String> {
            Err(NOT_BUILT_MSG.into())
        }

        pub fn transcribe(&self, _samples: &[f32]) -> Result<String, String> {
            Err(NOT_BUILT_MSG.into())
        }
    }
}

#[cfg(feature = "parakeet")]
pub use real::ParakeetEngine;
#[cfg(not(feature = "parakeet"))]
pub use stub::ParakeetEngine;

/// Reports whether the build includes the Parakeet backend.
pub fn backend_available() -> bool {
    cfg!(feature = "parakeet")
}
