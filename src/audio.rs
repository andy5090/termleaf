//! Low-latency, non-blocking typewriter audio.
//!
//! The output stream is opened once at startup and kept alive for the whole
//! session. Keystrokes add an in-memory PCM buffer directly to Rodio's mixer,
//! avoiding the process startup and file decoding delay of command-line
//! players.

use rodio::buffer::SamplesBuffer;
use rodio::cpal::BufferSize;
use rodio::{DeviceSinkBuilder, MixerDeviceSink};
use std::num::NonZero;

const TYPEWRITER_WAV: &[u8] = include_bytes!("../assets/typewriter-key.wav");
const LOW_LATENCY_BUFFER_FRAMES: u32 = 512;

/// Best-effort typewriter sound player backed by a persistent audio stream.
pub struct SoundPlayer {
    stream: Option<MixerDeviceSink>,
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

impl SoundPlayer {
    pub fn new() -> Self {
        let decoded = decode_pcm_wave(TYPEWRITER_WAV);
        let stream = DeviceSinkBuilder::from_default_device()
            .and_then(|builder| {
                builder
                    .with_buffer_size(BufferSize::Fixed(LOW_LATENCY_BUFFER_FRAMES))
                    .open_sink_or_fallback()
            })
            .ok();

        let (channels, sample_rate, samples) = decoded.unwrap_or_else(|| (1, 44_100, Vec::new()));
        Self {
            stream,
            samples,
            channels,
            sample_rate,
        }
    }

    /// Mix one key sound immediately. Rodio performs playback on its existing
    /// audio thread, so this call does not block the editor event loop.
    pub fn play(&self) {
        let Some(stream) = &self.stream else {
            return;
        };
        if self.samples.is_empty() {
            return;
        }
        let Some(channels) = NonZero::new(self.channels) else {
            return;
        };
        let Some(sample_rate) = NonZero::new(self.sample_rate) else {
            return;
        };
        stream.mixer().add(SamplesBuffer::new(
            channels,
            sample_rate,
            self.samples.clone(),
        ));
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
        let (channels, sample_rate, samples) =
            decode_pcm_wave(TYPEWRITER_WAV).expect("embedded WAV should decode");
        assert_eq!(channels, 1);
        assert_eq!(sample_rate, 44_100);
        assert!(samples.len() > 2_000);
        assert!(samples.iter().any(|sample| sample.abs() > 0.5));
    }

    #[test]
    fn invalid_wave_data_is_rejected() {
        assert!(decode_pcm_wave(b"not a wave").is_none());
    }
}
