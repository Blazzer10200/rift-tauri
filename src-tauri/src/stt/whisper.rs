//! Whisper inference. Built on `whisper-rs` (whisper.cpp FFI) when the
//! `whisper-rs` Cargo feature is enabled; otherwise a stub that returns a
//! clear "install LLVM and rebuild" error. The public surface is identical
//! across both branches so `mod.rs` orchestration doesn't need to fork.
//!
//! See `Cargo.toml` `[features]` header for the install + opt-in instructions.

use std::path::Path;
use std::sync::Arc;

// ----------------------------------------------------------------------------
// Real implementation — whisper-rs FFI (CPU; cuBLAS when whisper-cuda is on)
// ----------------------------------------------------------------------------
#[cfg(feature = "whisper-rs")]
mod real {
    use super::*;
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    #[derive(Clone)]
    pub struct WhisperEngine {
        ctx: Arc<WhisperContext>,
        pub model_id: String,
    }

    impl WhisperEngine {
        /// Load a GGML model from disk. Blocking — call from spawn_blocking.
        pub fn load(model_path: &Path, model_id: &str) -> Result<Self, String> {
            let params = WhisperContextParameters::default();
            let path_str = model_path.to_string_lossy().into_owned();
            let ctx = WhisperContext::new_with_params(&path_str, params)
                .map_err(|e| format!("load whisper model '{path_str}': {e}"))?;
            log::info!("[stt] whisper model loaded: id={model_id} path={path_str}");
            Ok(Self {
                ctx: Arc::new(ctx),
                model_id: model_id.to_string(),
            })
        }

        /// Transcribe 16 kHz mono f32 samples. Blocking. `language` is an ISO
        /// 639-1 code (`Some("en")`) or `None` for whisper.cpp auto-detect;
        /// `beam_size` of `Some(n)` with `n > 1` switches greedy → beam search.
        pub fn transcribe(
            &self,
            samples: &[f32],
            initial_prompt: &str,
            language: Option<&str>,
            beam_size: Option<u8>,
        ) -> Result<String, String> {
            if samples.is_empty() {
                return Ok(String::new());
            }
            let mut state = self
                .ctx
                .create_state()
                .map_err(|e| format!("whisper create_state: {e}"))?;

            let strategy = match beam_size {
                Some(n) if n > 1 => SamplingStrategy::BeamSearch {
                    beam_size: n as i32,
                    patience: 1.0,
                },
                _ => SamplingStrategy::Greedy { best_of: 1 },
            };
            let mut params = FullParams::new(strategy);
            params.set_language(language.or(Some("en")));
            params.set_translate(false);
            params.set_no_timestamps(true);
            params.set_suppress_blank(true);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_special(false);
            params.set_print_timestamps(false);
            params.set_single_segment(true);
            if !initial_prompt.is_empty() {
                params.set_initial_prompt(initial_prompt);
            }

            state
                .full(params, samples)
                .map_err(|e| format!("whisper full: {e}"))?;

            let n = state
                .full_n_segments()
                .map_err(|e| format!("whisper full_n_segments: {e}"))?;
            let mut out = String::new();
            for i in 0..n {
                let seg = state
                    .full_get_segment_text(i)
                    .map_err(|e| format!("whisper full_get_segment_text({i}): {e}"))?;
                out.push_str(&seg);
                out.push(' ');
            }
            Ok(out.trim().to_string())
        }
    }
}

// ----------------------------------------------------------------------------
// Stub — used when `whisper-rs` Cargo feature is off (the default build path)
// ----------------------------------------------------------------------------
#[cfg(not(feature = "whisper-rs"))]
mod stub {
    use super::*;

    const NOT_BUILT_MSG: &str = "Whisper backend not built. To enable on-device \
speech recognition: install LLVM (winget install LLVM.LLVM, needs admin), \
optionally install the NVIDIA CUDA Toolkit, then rebuild Rift with \
`cargo build --release --features whisper-rs` (or `whisper-rs,whisper-cuda` \
for GPU). The Web Speech engine remains available in Settings → Speech.";

    #[derive(Clone)]
    pub struct WhisperEngine {
        pub model_id: String,
        // Silence unused-field-on-feature-off lint by holding an Arc — also
        // gives WhisperEngine the same Send+Clone semantics across branches.
        _marker: Arc<()>,
    }

    impl WhisperEngine {
        pub fn load(_model_path: &Path, _model_id: &str) -> Result<Self, String> {
            Err(NOT_BUILT_MSG.into())
        }

        pub fn transcribe(
            &self,
            _samples: &[f32],
            _initial_prompt: &str,
            _language: Option<&str>,
            _beam_size: Option<u8>,
        ) -> Result<String, String> {
            Err(NOT_BUILT_MSG.into())
        }
    }

    // Keep the unused-field warning quiet at construction-time too — there is
    // no construction site (load always errs), but rustc still type-checks.
    impl WhisperEngine {
        #[allow(dead_code)]
        fn _new_unused() -> Self {
            Self {
                model_id: String::new(),
                _marker: Arc::new(()),
            }
        }
    }
}

#[cfg(feature = "whisper-rs")]
pub use real::WhisperEngine;
#[cfg(not(feature = "whisper-rs"))]
pub use stub::WhisperEngine;

// ----------------------------------------------------------------------------
// Shared helpers (feature-independent)
// ----------------------------------------------------------------------------

/// Compose the effective `initial_prompt` from the hard-coded preamble + the
/// user's vocab file. Whisper's prompt budget is 224 tokens (~896 chars) —
/// we cap to ~800 chars to leave headroom for tokenizer expansion, trimming
/// the vocab tail (preamble is non-negotiable).
pub fn compose_prompt(preamble: &str, vocab: &str) -> String {
    const MAX: usize = 800;
    let preamble = preamble.trim();
    let vocab = vocab.trim();
    if vocab.is_empty() {
        return truncate_at_char(preamble, MAX);
    }
    let combined = format!("{preamble} Vocabulary: {vocab}");
    truncate_at_char(&combined, MAX)
}

fn truncate_at_char(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// Reports whether the build includes a working Whisper backend. The frontend
/// uses this to decide whether to show "install LLVM and rebuild" hints in
/// the Speech settings panel.
pub fn backend_available() -> bool {
    cfg!(feature = "whisper-rs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_no_vocab() {
        let p = compose_prompt("hello world.", "");
        assert_eq!(p, "hello world.");
    }

    #[test]
    fn prompt_with_vocab() {
        let p = compose_prompt("hello.", "FiveM, Qbox, RedM");
        assert!(p.starts_with("hello."));
        assert!(p.contains("FiveM"));
    }

    #[test]
    fn prompt_truncates() {
        let long_vocab: String = "word ".repeat(500);
        let p = compose_prompt("preamble.", &long_vocab);
        assert!(p.len() <= 800);
        assert!(p.starts_with("preamble."));
    }
}
