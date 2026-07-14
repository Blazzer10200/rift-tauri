//! Microphone capture via `cpal`. Whisper expects 16 kHz mono f32 — we resample
//! on the audio thread w/ `rubato::FastFixedIn` to keep the inference hot path
//! free of conversion overhead. Captured samples land in a shared ring buffer
//! (capped at `MAX_BUFFER_SECS` of 16 kHz mono) that the whisper orchestrator
//! drains in rolling windows.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Whisper's native sample rate.
const TARGET_HZ: u32 = 16_000;
/// Cap the ring buffer at 5 min of 16 kHz mono (~19 MB worst case). The ring
/// serves BOTH the 3s rolling-partial peeks AND the finalise-on-stop drain —
/// the old 30s cap silently dropped everything before the last 30s of a long
/// dictation from the FINAL transcript (the rolling partials are preview-only,
/// never concatenated). 5 min covers any realistic dictation; past it the
/// oldest audio still falls off (warned at drain).
const MAX_BUFFER_SECS: usize = 300;

/// Ring storage + absolute-offset bookkeeping. `start` is the absolute sample
/// index (since capture start) of `buf[0]` — it advances as the cap evicts old
/// samples, so segment boundaries recorded as absolute offsets stay valid even
/// after eviction.
pub struct RingBuf {
    buf: VecDeque<f32>,
    start: u64,
}

impl RingBuf {
    fn new(capacity: usize) -> Self {
        Self { buf: VecDeque::with_capacity(capacity), start: 0 }
    }
    /// Absolute sample index one past the newest sample.
    fn end(&self) -> u64 {
        self.start + self.buf.len() as u64
    }
}

/// Shared 16 kHz mono f32 ring buffer. Cheap to clone (Arc).
pub type AudioRing = Arc<Mutex<RingBuf>>;

/// Live capture session. Dropping it stops the cpal stream.
pub struct AudioCapture {
    _stream: Stream,
    pub ring: AudioRing,
}

pub fn list_input_devices() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    for d in host
        .input_devices()
        .map_err(|e| format!("enumerate input devices: {e}"))?
    {
        if let Ok(name) = d.name() {
            out.push(name);
        }
    }
    Ok(out)
}

fn pick_device(name: Option<&str>) -> Result<Device, String> {
    let host = cpal::default_host();
    if let Some(n) = name {
        for d in host
            .input_devices()
            .map_err(|e| format!("enumerate input devices: {e}"))?
        {
            if d.name().map(|x| x == n).unwrap_or(false) {
                return Ok(d);
            }
        }
        return Err(format!("input device '{n}' not found"));
    }
    host.default_input_device().ok_or_else(|| {
        "No microphone found or access denied — check Windows mic permissions or that a mic is connected".into()
    })
}

/// Open the device, pick the best supported f32 input config, build the cpal
/// stream w/ a callback that resamples to 16 kHz and pushes into the ring.
pub fn start_capture(device_name: Option<&str>) -> Result<AudioCapture, String> {
    let device = pick_device(device_name)?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("default input config: {e}"))?;
    let source_hz = supported.sample_rate().0;
    let source_channels = supported.channels();
    let config: StreamConfig = supported.clone().into();
    let sample_format = supported.sample_format();

    // Pre-alloc 1 min; grows amortized toward MAX_BUFFER_SECS only when a
    // dictation actually runs long (don't commit 19 MB per mic start).
    let ring: AudioRing = Arc::new(Mutex::new(RingBuf::new(TARGET_HZ as usize * 60)));

    let ring_cb = ring.clone();
    let needs_resample = source_hz != TARGET_HZ;
    // Polynomial resampler — fast, low-CPU, audible-quality-sufficient for
    // ASR. Whisper doesn't care about anti-alias purity past 8 kHz.
    // Chunk size 1024 = ~21ms at 48k input; tail-leftover state below means
    // callbacks of any size (cpal often delivers ~10ms = 480 samples) still
    // produce output without losing the unaligned tail.
    let (resampler, leftover) = if needs_resample {
        let r = FastFixedIn::<f32>::new(
            TARGET_HZ as f64 / source_hz as f64,
            1.0,
            PolynomialDegree::Linear,
            1024,
            1,
        )
        .map_err(|e| format!("rubato init: {e}"))?;
        (
            Some(Arc::new(Mutex::new(r))),
            Some(Arc::new(Mutex::new(Vec::<f32>::with_capacity(4096)))),
        )
    } else {
        (None, None)
    };

    let err_cb = |e| log::warn!("[stt] cpal stream error: {e}");

    let stream = match sample_format {
        SampleFormat::F32 => {
            let ring_cb = ring_cb.clone();
            let rs = resampler.clone();
            let lo = leftover.clone();
            device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        push_samples(&ring_cb, data, source_channels, rs.as_ref(), lo.as_ref());
                    },
                    err_cb,
                    None,
                )
                .map_err(|e| format!("build f32 input stream: {e}"))?
        }
        SampleFormat::I16 => {
            let ring_cb = ring_cb.clone();
            let rs = resampler.clone();
            let lo = leftover.clone();
            let mut conv_buf: Vec<f32> = Vec::with_capacity(4096);
            device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        // /32768 (not i16::MAX=32767): the i16 range is asymmetric
                        // (-32768..=32767), so dividing by 32767 pushes the minimum
                        // sample to -1.00003 — past the ±1.0 rail. 32768 keeps every
                        // sample in range (max maps to +0.99997, which is correct).
                        conv_buf.clear();
                        conv_buf.extend(data.iter().map(|s| *s as f32 / 32768.0));
                        push_samples(&ring_cb, &conv_buf, source_channels, rs.as_ref(), lo.as_ref());
                    },
                    err_cb,
                    None,
                )
                .map_err(|e| format!("build i16 input stream: {e}"))?
        }
        SampleFormat::U16 => {
            let ring_cb = ring_cb.clone();
            let rs = resampler.clone();
            let lo = leftover.clone();
            let mut conv_buf: Vec<f32> = Vec::with_capacity(4096);
            device
                .build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        // u16 (0..=65535) centers at 32768; subtract it to center,
                        // divide by 32768 so the rails land in ±1.0. The old divisor
                        // i16::MAX (32767) pushed s=0 to -1.00003 — past the rail.
                        conv_buf.clear();
                        conv_buf.extend(data.iter().map(|s| (*s as f32 - 32768.0) / 32768.0));
                        push_samples(&ring_cb, &conv_buf, source_channels, rs.as_ref(), lo.as_ref());
                    },
                    err_cb,
                    None,
                )
                .map_err(|e| format!("build u16 input stream: {e}"))?
        }
        other => return Err(format!("unsupported sample format: {other:?}")),
    };
    stream.play().map_err(|e| format!("stream play: {e}"))?;

    log::info!(
        "[stt] mic capture started: device={:?} hz={} ch={} fmt={:?}",
        device_name,
        source_hz,
        source_channels,
        sample_format,
    );

    Ok(AudioCapture {
        _stream: stream,
        ring,
    })
}

/// Downmix to mono (avg of all channels), optionally resample to 16 kHz, push
/// into the ring. Trims oldest samples if the ring exceeds its cap.
///
/// `leftover` is the per-stream tail buffer — cpal callbacks deliver
/// arbitrary frame counts (typically ~10ms = 480 samples at 48k), but
/// `FastFixedIn` needs exactly `chunk_in` (1024) input frames per `process`
/// call. We accumulate across callbacks; the unaligned tail rolls forward.
fn push_samples(
    ring: &AudioRing,
    interleaved: &[f32],
    channels: u16,
    resampler: Option<&Arc<Mutex<FastFixedIn<f32>>>>,
    leftover: Option<&Arc<Mutex<Vec<f32>>>>,
) {
    // Fast path: already mono and no resampling needed — push directly.
    if channels <= 1 && resampler.is_none() {
        let cap = TARGET_HZ as usize * MAX_BUFFER_SECS;
        if let Ok(mut q) = ring.lock() {
            q.buf.extend(interleaved.iter().copied());
            while q.buf.len() > cap {
                q.buf.pop_front();
                q.start += 1;
            }
        }
        return;
    }

    let mono: Vec<f32> = if channels <= 1 {
        interleaved.to_vec()
    } else {
        let ch = channels as usize;
        interleaved
            .chunks_exact(ch)
            .map(|frame| frame.iter().sum::<f32>() / ch as f32)
            .collect()
    };

    let resampled: Vec<f32> = if let (Some(r), Some(lo)) = (resampler, leftover) {
        let mut out = Vec::with_capacity(mono.len() / 2);
        let mut buf = match lo.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        buf.extend_from_slice(&mono);
        let mut guard = match r.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let chunk_in = guard.input_frames_next();
        while buf.len() >= chunk_in {
            let chunk: Vec<f32> = buf.drain(..chunk_in).collect();
            match guard.process(&[chunk], None) {
                Ok(o) => out.extend_from_slice(&o[0]),
                Err(e) => {
                    log::debug!("[stt] resample skip: {e}");
                    break;
                }
            }
        }
        out
    } else {
        mono
    };

    let cap = TARGET_HZ as usize * MAX_BUFFER_SECS;
    if let Ok(mut q) = ring.lock() {
        q.buf.extend(resampled);
        while q.buf.len() > cap {
            q.buf.pop_front();
            q.start += 1;
        }
    }
}

/// Drain the most recent `secs` of audio without removing it from the ring
/// (peek-style — the rolling window needs overlap to produce sensible
/// partials, so we don't consume).
pub fn peek_window(ring: &AudioRing, secs: f32) -> Vec<f32> {
    let n = (TARGET_HZ as f32 * secs) as usize;
    let q = match ring.lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    let start = q.buf.len().saturating_sub(n);
    q.buf.iter().copied().skip(start).collect()
}

/// Peek samples from absolute offset `from` to the newest sample. When
/// `max_secs` is set and the span is longer, only the most-recent `max_secs`
/// are returned (sliding display window). Returns `(samples, end_offset)` —
/// `end_offset` is the absolute offset one past the last returned sample, for
/// recording a segment boundary. A `from` older than the ring's retention is
/// clamped to what's still buffered.
pub fn peek_from(ring: &AudioRing, from: u64, max_secs: Option<f32>) -> (Vec<f32>, u64) {
    let q = match ring.lock() {
        Ok(g) => g,
        Err(_) => return (Vec::new(), from),
    };
    let end = q.end();
    let mut lo = from.max(q.start).min(end);
    if let Some(secs) = max_secs {
        let cap = (TARGET_HZ as f32 * secs) as u64;
        lo = lo.max(end.saturating_sub(cap));
    }
    let skip = (lo - q.start) as usize;
    (q.buf.iter().copied().skip(skip).collect(), end)
}

/// Take everything from absolute offset `from` and clear the ring (finalise-on-
/// stop). Warns when `from` predates retention — uncommitted audio older than
/// the cap was evicted, so the tail transcript can't cover it.
pub fn take_from(ring: &AudioRing, from: u64) -> Vec<f32> {
    let mut q = match ring.lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    if from < q.start {
        log::warn!(
            "[stt] uncommitted dictation exceeded {MAX_BUFFER_SECS}s — final tail covers only the most recent {MAX_BUFFER_SECS}s"
        );
    }
    let skip = (from.max(q.start) - q.start) as usize;
    let out: Vec<f32> = q.buf.iter().copied().skip(skip).collect();
    let end = q.end();
    q.buf.clear();
    q.start = end;
    out
}

/// RMS amplitude (0.0..~1.0) of the most recent `secs` of audio, for driving a
/// live input-level meter. Peek-style — never consumes. Cheap: one pass over a
/// small tail (~50ms = 800 samples), so it's safe to poll at meter cadence.
pub fn peek_level(ring: &AudioRing, secs: f32) -> f32 {
    let n = (TARGET_HZ as f32 * secs) as usize;
    let q = match ring.lock() {
        Ok(g) => g,
        Err(_) => return 0.0,
    };
    let start = q.buf.len().saturating_sub(n);
    let mut sum_sq = 0.0f32;
    let mut count = 0usize;
    for &s in q.buf.iter().skip(start) {
        sum_sq += s * s;
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    (sum_sq / count as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring_with(samples: &[f32], start: u64) -> AudioRing {
        Arc::new(Mutex::new(RingBuf {
            buf: samples.iter().copied().collect(),
            start,
        }))
    }

    #[test]
    fn peek_from_respects_absolute_offsets() {
        let ring = ring_with(&[1.0, 2.0, 3.0, 4.0], 10);
        let (samples, end) = peek_from(&ring, 12, None);
        assert_eq!(samples, vec![3.0, 4.0]);
        assert_eq!(end, 14);
    }

    #[test]
    fn peek_from_clamps_evicted_offset() {
        // `from` predates retention — clamp to what's buffered.
        let ring = ring_with(&[5.0, 6.0], 100);
        let (samples, end) = peek_from(&ring, 3, None);
        assert_eq!(samples, vec![5.0, 6.0]);
        assert_eq!(end, 102);
    }

    #[test]
    fn peek_from_caps_to_max_secs() {
        let n = TARGET_HZ as usize; // 1s of audio
        let buf: Vec<f32> = (0..n * 2).map(|i| i as f32).collect();
        let ring = ring_with(&buf, 0);
        let (samples, end) = peek_from(&ring, 0, Some(1.0));
        assert_eq!(samples.len(), n);
        assert_eq!(samples[0], n as f32); // most-recent second only
        assert_eq!(end, n as u64 * 2);
    }

    #[test]
    fn take_from_drains_tail_and_clears() {
        let ring = ring_with(&[1.0, 2.0, 3.0], 5);
        let out = take_from(&ring, 6);
        assert_eq!(out, vec![2.0, 3.0]);
        // Ring is empty; offsets keep advancing from where it ended.
        let (rest, end) = peek_from(&ring, 0, None);
        assert!(rest.is_empty());
        assert_eq!(end, 8);
    }
}
