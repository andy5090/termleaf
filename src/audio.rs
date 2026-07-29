//! Low-latency, non-blocking typewriter audio.
//!
//! The output stream is opened once at startup and kept alive for the whole
//! session. Keystrokes add an in-memory PCM buffer directly to Rodio's mixer,
//! avoiding the process startup and file decoding delay of command-line
//! players.

use rodio::buffer::SamplesBuffer;
use rodio::cpal::BufferSize;
use rodio::{DeviceSinkBuilder, MixerDeviceSink};
use std::cell::Cell;
use std::num::NonZero;
use std::time::{Duration, Instant};

const TYPEWRITER_WAV: &[u8] = include_bytes!("../assets/typewriter-key.wav");
const TYPEWRITER_DEEP_WAV: &[u8] = include_bytes!("../assets/typewriter-key-deep.wav");
const TYPEWRITER_SOFT_WAV: &[u8] = include_bytes!("../assets/typewriter-key-soft.wav");
const BACKSPACE_WAV: &[u8] = include_bytes!("../assets/typewriter-backspace.wav");
const RETURN_WAV: &[u8] = include_bytes!("../assets/typewriter-return.wav");
const LOW_LATENCY_BUFFER_FRAMES: u32 = 512;
const BACKSPACE_MIN_INTERVAL: Duration = Duration::from_millis(55);

struct Clip {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

/// Best-effort typewriter sound player backed by a persistent audio stream.
pub struct SoundPlayer {
    stream: Option<MixerDeviceSink>,
    classic_key: Clip,
    deep_key: Clip,
    soft_key: Clip,
    backspace: Clip,
    carriage_return: Clip,
    last_backspace: Cell<Option<Instant>>,
}

impl SoundPlayer {
    pub fn new() -> Self {
        let stream = DeviceSinkBuilder::from_default_device()
            .and_then(|builder| {
                builder
                    .with_buffer_size(BufferSize::Fixed(LOW_LATENCY_BUFFER_FRAMES))
                    .open_sink_or_fallback()
            })
            .ok()
            .map(|mut stream| {
                // The stream intentionally lives until normal application
                // shutdown, so Rodio's development-only drop warning is noise.
                stream.log_on_drop(false);
                stream
            });

        Self {
            stream,
            classic_key: Clip::decode(TYPEWRITER_WAV),
            deep_key: Clip::decode(TYPEWRITER_DEEP_WAV),
            soft_key: Clip::decode(TYPEWRITER_SOFT_WAV),
            backspace: Clip::decode(BACKSPACE_WAV),
            carriage_return: Clip::decode(RETURN_WAV),
            last_backspace: Cell::new(None),
        }
    }

    /// Mix one printing-key strike immediately.
    pub fn play_key(&self, profile: &str) {
        let clip = match profile {
            "deep" => &self.deep_key,
            "soft" => &self.soft_key,
            _ => &self.classic_key,
        };
        self.play(clip);
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
        let Some(stream) = &self.stream else {
            return;
        };
        if clip.samples.is_empty() {
            return;
        }
        let Some(channels) = NonZero::new(clip.channels) else {
            return;
        };
        let Some(sample_rate) = NonZero::new(clip.sample_rate) else {
            return;
        };
        stream.mixer().add(SamplesBuffer::new(
            channels,
            sample_rate,
            clip.samples.clone(),
        ));
    }
}

fn backspace_playback_allowed(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|last| now.duration_since(last) >= BACKSPACE_MIN_INTERVAL)
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

    #[test]
    fn embedded_typewriter_sound_is_valid_pcm() {
        for wav in [
            TYPEWRITER_WAV,
            TYPEWRITER_DEEP_WAV,
            TYPEWRITER_SOFT_WAV,
            BACKSPACE_WAV,
            RETURN_WAV,
        ] {
            let (channels, sample_rate, samples) =
                decode_pcm_wave(wav).expect("embedded WAV should decode");
            assert_eq!(channels, 1);
            assert_eq!(sample_rate, 44_100);
            assert!(samples.len() > 3_000);
            assert!(samples.iter().any(|sample| sample.abs() > 0.25));
        }
    }

    #[test]
    fn key_and_backspace_use_distinct_recordings() {
        let (_, _, key) = decode_pcm_wave(TYPEWRITER_WAV).expect("key WAV should decode");
        let (_, _, deep) =
            decode_pcm_wave(TYPEWRITER_DEEP_WAV).expect("deep key WAV should decode");
        let (_, _, soft) =
            decode_pcm_wave(TYPEWRITER_SOFT_WAV).expect("soft key WAV should decode");
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
}
