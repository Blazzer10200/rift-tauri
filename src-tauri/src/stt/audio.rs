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
/// Cap the ring buffer at 30s of 16 kHz mono = 480k samples (~1.9 MB). More
/// than enough for the 3s rolling window + finalise-on-stop.
const MAX_BUFFER_SECS: usize = 30;

/// Shared 16 kHz mono f32 ring buffer. Cheap to clone (Arc).
pub type AudioRing = Arc<Mutex<VecDeque<f32>>>;

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

    let ring: AudioRing = Arc::new(Mutex::new(VecDeque::with_capacity(
        TARGET_HZ as usize * MAX_BUFFER_SECS,
    )));

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
            q.extend(interleaved.iter().copied());
            while q.len() > cap {
                q.pop_front();
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
        q.extend(resampled);
        while q.len() > cap {
            q.pop_front();
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
    let start = q.len().saturating_sub(n);
    q.iter().copied().skip(start).collect()
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
    let start = q.len().saturating_sub(n);
    let mut sum_sq = 0.0f32;
    let mut count = 0usize;
    for &s in q.iter().skip(start) {
        sum_sq += s * s;
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    (sum_sq / count as f32).sqrt()
}

/// Drain the entire buffer (used at finalise-on-stop).
pub fn drain_all(ring: &AudioRing) -> Vec<f32> {
    let mut q = match ring.lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    let out: Vec<f32> = q.iter().copied().collect();
    q.clear();
    out
}
