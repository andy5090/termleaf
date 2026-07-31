//! Low-latency, non-blocking typewriter audio.
//!
//! The output stream is opened once at startup and kept alive for the whole
//! session. Keystrokes add an in-memory PCM buffer directly to Rodio's mixer,
//! avoiding the process startup and file decoding delay of command-line
//! players.

use rodio::buffer::SamplesBuffer;
use rodio::cpal::StreamError;
use rodio::{DeviceSinkBuilder, MixerDeviceSink};
use std::cell::{Cell, RefCell};
use std::num::NonZero;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const KEY_VARIANT_COUNT: usize = 4;
const TYPEWRITER_WAVS: [&[u8]; KEY_VARIANT_COUNT] = [
    include_bytes!("../assets/typewriter-key.wav"),
    include_bytes!("../assets/typewriter-key-2.wav"),
    include_bytes!("../assets/typewriter-key-3.wav"),
    include_bytes!("../assets/typewriter-key-4.wav"),
];
const TYPEWRITER_DEEP_WAVS: [&[u8]; KEY_VARIANT_COUNT] = [
    include_bytes!("../assets/typewriter-key-deep.wav"),
    include_bytes!("../assets/typewriter-key-deep-2.wav"),
    include_bytes!("../assets/typewriter-key-deep-3.wav"),
    include_bytes!("../assets/typewriter-key-deep-4.wav"),
];
const TYPEWRITER_SOFT_WAVS: [&[u8]; KEY_VARIANT_COUNT] = [
    include_bytes!("../assets/typewriter-key-soft.wav"),
    include_bytes!("../assets/typewriter-key-soft-2.wav"),
    include_bytes!("../assets/typewriter-key-soft-3.wav"),
    include_bytes!("../assets/typewriter-key-soft-4.wav"),
];
const BACKSPACE_WAV: &[u8] = include_bytes!("../assets/typewriter-backspace.wav");
const RETURN_WAV: &[u8] = include_bytes!("../assets/typewriter-return.wav");
const BACKSPACE_MIN_INTERVAL: Duration = Duration::from_millis(55);
const STREAM_RETRY_INTERVAL: Duration = Duration::from_secs(2);
#[cfg(all(target_os = "linux", target_arch = "x86"))]
const I686_STABILITY_BUFFER_FRAMES: u32 = 4_096;

struct Clip {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

/// Best-effort typewriter sound player backed by a persistent audio stream.
pub struct SoundPlayer {
    stream: RefCell<Option<MixerDeviceSink>>,
    stream_healthy: Arc<AtomicBool>,
    classic_keys: [Clip; KEY_VARIANT_COUNT],
    deep_keys: [Clip; KEY_VARIANT_COUNT],
    soft_keys: [Clip; KEY_VARIANT_COUNT],
    backspace: Clip,
    carriage_return: Clip,
    key_variant_state: Cell<u32>,
    last_key_variant: Cell<Option<usize>>,
    last_backspace: Cell<Option<Instant>>,
    last_stream_retry: Cell<Option<Instant>>,
}

impl SoundPlayer {
    pub fn new() -> Self {
        let stream_healthy = Arc::new(AtomicBool::new(true));
        let stream = open_stream(Arc::clone(&stream_healthy));

        Self {
            stream: RefCell::new(stream),
            stream_healthy,
            classic_keys: TYPEWRITER_WAVS.map(Clip::decode),
            deep_keys: TYPEWRITER_DEEP_WAVS.map(Clip::decode),
            soft_keys: TYPEWRITER_SOFT_WAVS.map(Clip::decode),
            backspace: Clip::decode(BACKSPACE_WAV),
            carriage_return: Clip::decode(RETURN_WAV),
            key_variant_state: Cell::new(0x7A_DA_C0_DE),
            last_key_variant: Cell::new(None),
            last_backspace: Cell::new(None),
            last_stream_retry: Cell::new(None),
        }
    }

    /// Mix one printing-key strike immediately.
    pub fn play_key(&self, profile: &str) {
        let clips = match profile {
            "deep" => &self.deep_keys,
            "soft" => &self.soft_keys,
            _ => &self.classic_keys,
        };
        let (state, variant) =
            next_key_variant(self.key_variant_state.get(), self.last_key_variant.get());
        self.key_variant_state.set(state);
        self.last_key_variant.set(Some(variant));
        self.play(&clips[variant]);
    }

    /// Mix the separate, gentle delete-key release effect.
    pub fn play_backspace(&self) {
        let now = Instant::now();
        if !backspace_playback_allowed(self.last_backspace.get(), now) {
            return;
        }
        self.last_backspace.set(Some(now));
        self.play(&self.backspace);
    }

    /// Mix the margin bell and carriage-return travel effect.
    pub fn play_return(&self) {
        self.play(&self.carriage_return);
    }

    /// Rodio performs playback on its existing audio thread, so this call does
    /// not block the editor event loop.
    fn play(&self, clip: &Clip) {
        self.recover_stream_if_needed();
        if clip.samples.is_empty() {
            return;
        }
        let Some(channels) = NonZero::new(clip.channels) else {
            return;
        };
        let Some(sample_rate) = NonZero::new(clip.sample_rate) else {
            return;
        };
        let stream = self.stream.borrow();
        let Some(stream) = stream.as_ref() else {
            return;
        };
        stream.mixer().add(SamplesBuffer::new(
            channels,
            sample_rate,
            clip.samples.clone(),
        ));
    }

    fn recover_stream_if_needed(&self) {
        let healthy = self.stream_healthy.load(Ordering::Acquire);
        if healthy && self.stream.borrow().is_some() {
            return;
        }

        if !healthy {
            self.stream.borrow_mut().take();
        }

        let now = Instant::now();
        if !stream_retry_allowed(self.last_stream_retry.get(), now) {
            return;
        }

        self.last_stream_retry.set(Some(now));
        self.stream_healthy.store(true, Ordering::Release);
        *self.stream.borrow_mut() = open_stream(Arc::clone(&self.stream_healthy));
    }
}

fn open_stream(stream_healthy: Arc<AtomicBool>) -> Option<MixerDeviceSink> {
    let callback_state = Arc::clone(&stream_healthy);
    let builder = DeviceSinkBuilder::from_default_device().ok()?;

    // Older 32-bit x86 machines have less scheduling headroom. Rodio documents
    // 2048-4096 frames as its stability-focused range, so use the upper bound
    // there while retaining the lower-latency default on other targets.
    #[cfg(all(target_os = "linux", target_arch = "x86"))]
    let builder =
        builder.with_buffer_size(rodio::cpal::BufferSize::Fixed(I686_STABILITY_BUFFER_FRAMES));

    builder
        .with_error_callback(move |error| {
            if stream_error_requires_rebuild(&error) {
                callback_state.store(false, Ordering::Release);
            }
        })
        .open_sink_or_fallback()
        .ok()
        .map(|mut stream| {
            // The stream intentionally lives until normal application
            // shutdown, so Rodio's development-only drop warning is noise.
            stream.log_on_drop(false);
            stream
        })
}

fn stream_error_requires_rebuild(error: &StreamError) -> bool {
    matches!(
        error,
        StreamError::DeviceNotAvailable | StreamError::StreamInvalidated
    )
}

fn backspace_playback_allowed(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|last| now.duration_since(last) >= BACKSPACE_MIN_INTERVAL)
}

fn next_key_variant(state: u32, last: Option<usize>) -> (u32, usize) {
    let state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    let mut variant = ((state >> 16) as usize) % KEY_VARIANT_COUNT;
    if last == Some(variant) {
        variant = (variant + 1 + (state as usize & 1)) % KEY_VARIANT_COUNT;
    }
    (state, variant)
}

fn stream_retry_allowed(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|last| now.duration_since(last) >= STREAM_RETRY_INTERVAL)
}

impl Clip {
    fn decode(wav: &[u8]) -> Self {
        let (channels, sample_rate, samples) =
            decode_pcm_wave(wav).unwrap_or_else(|| (1, 44_100, Vec::new()));
        Self {
            samples,
            channels,
            sample_rate,
        }
    }
}

/// Decode the generator's standard 16-bit little-endian PCM WAV.
fn decode_pcm_wave(wav: &[u8]) -> Option<(u16, u32, Vec<f32>)> {
    if wav.len() < 44 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return None;
    }
    let channels = u16::from_le_bytes([wav[22], wav[23]]);
    let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
    let bits_per_sample = u16::from_le_bytes([wav[34], wav[35]]);
    if channels == 0 || sample_rate == 0 || bits_per_sample != 16 {
        return None;
    }

    let data_position = wav.windows(4).position(|window| window == b"data")?;
    let length_position = data_position + 4;
    let samples_position = data_position + 8;
    if samples_position > wav.len() {
        return None;
    }
    let data_len = u32::from_le_bytes(
        wav.get(length_position..samples_position)?
            .try_into()
            .ok()?,
    ) as usize;
    let data = wav.get(samples_position..samples_position.checked_add(data_len)?)?;
    if data.len() % 2 != 0 {
        return None;
    }

    let samples = data
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]) as f32 / i16::MAX as f32)
        .collect();
    Some((channels, sample_rate, samples))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedded_wavs() -> Vec<&'static [u8]> {
        TYPEWRITER_WAVS
            .into_iter()
            .chain(TYPEWRITER_DEEP_WAVS)
            .chain(TYPEWRITER_SOFT_WAVS)
            .chain([BACKSPACE_WAV, RETURN_WAV])
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
    }

    fn tone_power(samples: &[f32], sample_rate: u32, frequency: f32) -> f64 {
        let (real, imaginary) =
            samples
                .iter()
                .enumerate()
                .fold((0.0, 0.0), |(real, imaginary), (index, sample)| {
                    let phase = std::f64::consts::TAU * frequency as f64 * index as f64
                        / sample_rate as f64;
                    (
                        real + *sample as f64 * phase.cos(),
                        imaginary + *sample as f64 * phase.sin(),
                    )
                });
        (real * real + imaginary * imaginary) / (samples.len() * samples.len()) as f64
    }

    fn band_power(samples: &[f32], sample_rate: u32, frequencies: &[f32]) -> f64 {
        frequencies
            .iter()
            .map(|frequency| tone_power(samples, sample_rate, *frequency))
            .sum()
    }

    #[test]
    fn embedded_typewriter_sound_is_valid_pcm() {
        for wav in embedded_wavs() {
            let (channels, sample_rate, samples) =
                decode_pcm_wave(wav).expect("embedded WAV should decode");
            assert_eq!(channels, 1);
            assert_eq!(sample_rate, 44_100);
            assert!(samples.len() > 2_500);
            assert!(samples.iter().any(|sample| sample.abs() > 0.2));
        }
    }

    #[test]
    fn classic_keys_keep_a_short_recorded_decay() {
        for wav in TYPEWRITER_WAVS {
            let (_, sample_rate, samples) = decode_pcm_wave(wav).expect("key WAV should decode");
            let duration = samples.len() as f32 / sample_rate as f32;
            let section = |start: f32, end: f32| {
                &samples[(start * sample_rate as f32) as usize..(end * sample_rate as f32) as usize]
            };

            assert!((0.2..=0.26).contains(&duration));
            assert!(
                rms(section(0.0, 0.08)) > rms(section(duration - 0.05, duration)) * 2.0,
                "the recorded strike should decay instead of ending abruptly"
            );
        }
    }

    #[test]
    fn built_in_effects_do_not_emphasize_drum_like_resonances() {
        let rumble_frequencies = [
            68.0, 82.0, 112.0, 126.0, 164.0, 170.0, 238.0, 252.0, 290.0, 328.0,
        ];
        let mechanical_frequencies = [
            360.0, 420.0, 480.0, 580.0, 620.0, 720.0, 850.0, 980.0, 1_150.0, 1_400.0, 1_650.0,
            1_800.0, 2_100.0, 2_400.0, 2_900.0, 3_400.0,
        ];

        for wav in embedded_wavs() {
            let (_, sample_rate, samples) =
                decode_pcm_wave(wav).expect("embedded WAV should decode");
            let rumble = band_power(&samples, sample_rate, &rumble_frequencies);
            let mechanical = band_power(&samples, sample_rate, &mechanical_frequencies);
            assert!(
                rumble < mechanical * 0.2,
                "typewriter effects should favor mechanical detail over low-frequency rumble"
            );
        }
    }

    #[test]
    fn key_variants_and_backspace_use_distinct_recordings() {
        let (_, _, key) = decode_pcm_wave(TYPEWRITER_WAVS[0]).expect("key WAV should decode");
        let (_, _, deep) =
            decode_pcm_wave(TYPEWRITER_DEEP_WAVS[0]).expect("deep key WAV should decode");
        let (_, _, soft) =
            decode_pcm_wave(TYPEWRITER_SOFT_WAVS[0]).expect("soft key WAV should decode");
        let (_, _, backspace) =
            decode_pcm_wave(BACKSPACE_WAV).expect("backspace WAV should decode");
        let (_, _, carriage_return) =
            decode_pcm_wave(RETURN_WAV).expect("return WAV should decode");
        assert_ne!(key.len(), deep.len());
        assert_ne!(key.len(), soft.len());
        assert_ne!(key.len(), backspace.len());
        assert_ne!(key.len(), carriage_return.len());
        assert_ne!(
            &key[..key.len().min(1_000)],
            &backspace[..key.len().min(1_000)]
        );
        for pair in TYPEWRITER_WAVS.windows(2) {
            assert_ne!(pair[0], pair[1], "adjacent key variants must differ");
        }
    }

    #[test]
    fn key_variant_selection_avoids_immediate_repetition() {
        let mut state = 0x7A_DA_C0_DE;
        let mut last = None;
        for _ in 0..64 {
            let (next_state, variant) = next_key_variant(state, last);
            assert!(variant < KEY_VARIANT_COUNT);
            assert_ne!(Some(variant), last);
            state = next_state;
            last = Some(variant);
        }
    }

    #[test]
    fn carriage_return_contains_each_mechanical_stage() {
        let (_, sample_rate, samples) =
            decode_pcm_wave(RETURN_WAV).expect("return WAV should decode");
        let section = |start: f32, end: f32| {
            &samples[(start * sample_rate as f32) as usize..(end * sample_rate as f32) as usize]
        };

        assert!((0.75..=0.85).contains(&(samples.len() as f32 / sample_rate as f32)));
        let lever = rms(section(0.0, 0.08));
        let quiet_travel = rms(section(0.08, 0.13));
        let stop_and_bell = rms(section(0.21, 0.31));
        assert!(lever > 0.02, "lever contact should lead");
        assert!(
            quiet_travel < stop_and_bell * 0.35,
            "travel should leave space before the final strike"
        );
        assert!(
            stop_and_bell > 0.08,
            "stop and high bell should be the main event"
        );
        assert!(
            rms(section(0.55, 0.75)) > 0.02,
            "bell should decay naturally"
        );
    }

    #[test]
    fn invalid_wave_data_is_rejected() {
        assert!(decode_pcm_wave(b"not a wave").is_none());
    }

    #[test]
    fn rapid_backspace_requests_are_rate_limited() {
        let start = Instant::now();
        assert!(backspace_playback_allowed(None, start));
        assert!(!backspace_playback_allowed(
            Some(start),
            start + BACKSPACE_MIN_INTERVAL - Duration::from_millis(1)
        ));
        assert!(backspace_playback_allowed(
            Some(start),
            start + BACKSPACE_MIN_INTERVAL
        ));
    }

    #[test]
    fn only_device_loss_and_invalidated_streams_require_a_rebuild() {
        assert!(!stream_error_requires_rebuild(&StreamError::BufferUnderrun));
        assert!(stream_error_requires_rebuild(
            &StreamError::DeviceNotAvailable
        ));
        assert!(stream_error_requires_rebuild(
            &StreamError::StreamInvalidated
        ));
        assert!(!stream_error_requires_rebuild(
            &StreamError::BackendSpecific {
                err: rodio::cpal::BackendSpecificError {
                    description: "`alsa::poll()` returned POLLERR".into(),
                },
            }
        ));
    }

    #[test]
    fn failed_stream_retries_are_rate_limited() {
        let start = Instant::now();
        assert!(stream_retry_allowed(None, start));
        assert!(!stream_retry_allowed(
            Some(start),
            start + STREAM_RETRY_INTERVAL - Duration::from_millis(1)
        ));
        assert!(stream_retry_allowed(
            Some(start),
            start + STREAM_RETRY_INTERVAL
        ));
    }
}
