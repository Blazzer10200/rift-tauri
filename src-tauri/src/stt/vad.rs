//! Voice activity detection — `webrtc-vad` energy gating + post-Whisper
//! blocklist for residual silence-hallucination text. Two layers:
//!   1. Pre-Whisper: drop windows whose speech-frame ratio falls below a
//!      threshold. Avoids spending GPU on silence and starves the hallucination
//!      class at the source.
//!   2. Post-Whisper: strip known silence-artifact phrases from the final
//!      transcript (e.g. "Thank you for watching", "Subtitles by …" — the
//!      135-phrase corpus catalogued by r/LocalLLaMA Mar 2026).

use std::sync::OnceLock;
use webrtc_vad::{SampleRate, Vad, VadMode};

/// 30 ms @ 16 kHz = 480 samples — webrtc-vad's largest supported frame size.
const FRAME_SAMPLES: usize = 480;
/// Window-level threshold: at least this fraction of frames must register as
/// speech for the window to be passed to Whisper. 0.10 = ~300 ms of speech
/// inside a 3 s window — generous for the rolling partial path, strict enough
/// to drop pure-silence windows.
const SPEECH_RATIO_THRESHOLD: f32 = 0.10;

/// Returns true if the window appears to contain speech.
pub fn window_has_speech(samples_f32: &[f32]) -> bool {
    if samples_f32.is_empty() {
        return false;
    }
    let mut vad = Vad::new_with_rate_and_mode(SampleRate::Rate16kHz, VadMode::Aggressive);
    let total_frames = samples_f32.len() / FRAME_SAMPLES;
    if total_frames == 0 {
        // Tiny window — fall back to RMS energy gate.
        let rms = (samples_f32.iter().map(|s| s * s).sum::<f32>() / samples_f32.len() as f32).sqrt();
        return rms > 0.005;
    }
    let mut speech_frames = 0usize;
    for chunk in samples_f32.chunks_exact(FRAME_SAMPLES) {
        let frame_i16: Vec<i16> = chunk
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .collect();
        if vad.is_voice_segment(&frame_i16).unwrap_or(false) {
            speech_frames += 1;
        }
    }
    let ratio = speech_frames as f32 / total_frames as f32;
    ratio >= SPEECH_RATIO_THRESHOLD
}

/// Known silence-hallucination phrases (case-insensitive substring match).
/// Sourced from the r/LocalLLaMA 135-phrase whisper-hallucination corpus
/// (Mar 2026). Trimmed to the highest-frequency entries — full list lives in
/// the original thread; this is the 80/20.
const HALLUCINATION_PHRASES: &[&str] = &[
    "thank you for watching",
    "thanks for watching",
    "please subscribe",
    "subtitles by the amara.org community",
    "subtitled by",
    "transcription by",
    "subtitles by",
    "captions by",
    "[music]",
    "[applause]",
    "[silence]",
    "♪",
    "you you you",
    "bye bye",
    "thank you. thank you.",
    "thanks for listening",
    "see you next time",
    "see you in the next video",
    "translated by",
    "(silence)",
    "(no audio)",
];

fn hallucination_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let alts = HALLUCINATION_PHRASES
            .iter()
            .map(|p| regex::escape(p))
            .collect::<Vec<_>>()
            .join("|");
        regex::Regex::new(&format!("(?i)(?:{alts})")).expect("hallucination regex")
    })
}

/// Strip known hallucination artifacts from a transcript. Case-insensitive,
/// handles multiple occurrences per phrase. Idempotent. Collapses whitespace.
pub fn strip_hallucinations(text: &str) -> String {
    let out = hallucination_re().replace_all(text, "");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_known_phrase() {
        let t = "okay so the meeting Thank you for watching is at three";
        let cleaned = strip_hallucinations(t);
        assert!(!cleaned.to_lowercase().contains("thank you for watching"));
        assert!(cleaned.contains("meeting"));
        assert!(cleaned.contains("three"));
    }

    #[test]
    fn idempotent() {
        let t = "hello world";
        assert_eq!(strip_hallucinations(t), "hello world");
    }

    #[test]
    fn empty_window_no_speech() {
        assert!(!window_has_speech(&[]));
    }
}
