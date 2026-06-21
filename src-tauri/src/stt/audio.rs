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
    host.default_input_device()
        .ok_or_else(|| "no default input device".into())
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
            device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let f: Vec<f32> =
                            data.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
                        push_samples(&ring_cb, &f, source_channels, rs.as_ref(), lo.as_ref());
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
            device
                .build_input_stream(
                    &config,
                    move |data: &[u16], _| {
                        let f: Vec<f32> = data
                            .iter()
                            .map(|s| (*s as f32 - i16::MAX as f32 - 1.0) / i16::MAX as f32)
                            .collect();
                        push_samples(&ring_cb, &f, source_channels, rs.as_ref(), lo.as_ref());
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
