//! Non-blocking typewriter audio.
//!
//! The output stream is opened once at startup and kept alive for the whole
//! session. Keystrokes add an in-memory PCM buffer directly to Rodio's mixer,
//! avoiding process startup on desktop systems. Termux uses its `play-audio`
//! command on a dedicated worker because terminal processes have no Android
//! JVM context for CPAL's normal device discovery.

#[cfg(not(target_os = "android"))]
use rodio::buffer::SamplesBuffer;
#[cfg(not(target_os = "android"))]
use rodio::cpal::StreamError;
#[cfg(not(target_os = "android"))]
use rodio::DeviceSinkBuilder;
#[cfg(not(target_os = "android"))]
use rodio::MixerDeviceSink;
use std::cell::Cell;
#[cfg(not(target_os = "android"))]
use std::cell::RefCell;
#[cfg(target_os = "android")]
use std::collections::VecDeque;
#[cfg(target_os = "android")]
use std::fs;
#[cfg(not(target_os = "android"))]
use std::num::NonZero;
#[cfg(target_os = "android")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "android")]
use std::process::{Child, Command, Stdio};
#[cfg(not(target_os = "android"))]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "android")]
use std::sync::mpsc::{sync_channel, SyncSender};
#[cfg(not(target_os = "android"))]
use std::sync::Arc;
#[cfg(target_os = "android")]
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
#[cfg(target_os = "android")]
use std::time::{SystemTime, UNIX_EPOCH};

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
#[cfg(target_os = "android")]
const EXTERNAL_SOUND_QUEUE_CAPACITY: usize = 8;
#[cfg(target_os = "android")]
const MAX_CONCURRENT_EXTERNAL_SOUNDS: usize = 4;
#[cfg(not(target_os = "android"))]
const STREAM_RETRY_INTERVAL: Duration = Duration::from_secs(2);

#[cfg(not(target_os = "android"))]
struct Clip {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
}

/// Best-effort typewriter sound player backed by a mixer or Termux worker.
pub struct SoundPlayer {
    #[cfg(not(target_os = "android"))]
    stream: RefCell<Option<MixerDeviceSink>>,
    #[cfg(not(target_os = "android"))]
    stream_healthy: Arc<AtomicBool>,
    #[cfg(not(target_os = "android"))]
    classic_keys: [Clip; KEY_VARIANT_COUNT],
    #[cfg(not(target_os = "android"))]
    deep_keys: [Clip; KEY_VARIANT_COUNT],
    #[cfg(not(target_os = "android"))]
    soft_keys: [Clip; KEY_VARIANT_COUNT],
    #[cfg(not(target_os = "android"))]
    backspace: Clip,
    #[cfg(not(target_os = "android"))]
    carriage_return: Clip,
    #[cfg(target_os = "android")]
    external: Option<ExternalSoundPlayer>,
    key_variant_state: Cell<u32>,
    last_key_variant: Cell<Option<usize>>,
    last_backspace: Cell<Option<Instant>>,
    #[cfg(not(target_os = "android"))]
    last_stream_retry: Cell<Option<Instant>>,
}

impl SoundPlayer {
    pub fn new() -> Self {
        #[cfg(not(target_os = "android"))]
        let stream_healthy = Arc::new(AtomicBool::new(true));
        #[cfg(not(target_os = "android"))]
        let stream = open_stream(Arc::clone(&stream_healthy));

        Self {
            #[cfg(not(target_os = "android"))]
            stream: RefCell::new(stream),
            #[cfg(not(target_os = "android"))]
            stream_healthy,
            #[cfg(not(target_os = "android"))]
            classic_keys: TYPEWRITER_WAVS.map(Clip::decode),
            #[cfg(not(target_os = "android"))]
            deep_keys: TYPEWRITER_DEEP_WAVS.map(Clip::decode),
            #[cfg(not(target_os = "android"))]
            soft_keys: TYPEWRITER_SOFT_WAVS.map(Clip::decode),
            #[cfg(not(target_os = "android"))]
            backspace: Clip::decode(BACKSPACE_WAV),
            #[cfg(not(target_os = "android"))]
            carriage_return: Clip::decode(RETURN_WAV),
            #[cfg(target_os = "android")]
            external: ExternalSoundPlayer::new(),
            key_variant_state: Cell::new(0x7A_DA_C0_DE),
            last_key_variant: Cell::new(None),
            last_backspace: Cell::new(None),
            #[cfg(not(target_os = "android"))]
            last_stream_retry: Cell::new(None),
        }
    }

    /// Mix one printing-key strike immediately.
    pub fn play_key(&self, profile: &str) {
        #[cfg(not(target_os = "android"))]
        let clips = match profile {
            "deep" => &self.deep_keys,
            "soft" => &self.soft_keys,
            _ => &self.classic_keys,
        };
        let (state, variant) =
            next_key_variant(self.key_variant_state.get(), self.last_key_variant.get());
        self.key_variant_state.set(state);
        self.last_key_variant.set(Some(variant));

        #[cfg(target_os = "android")]
        if let Some(external) = &self.external {
            external.play_key(profile, variant);
        }
        #[cfg(not(target_os = "android"))]
        self.play(&clips[variant]);
    }

    /// Mix the separate, gentle delete-key release effect.
    pub fn play_backspace(&self) {
        let now = Instant::now();
        if !backspace_playback_allowed(self.last_backspace.get(), now) {
            return;
        }
        self.last_backspace.set(Some(now));

        #[cfg(target_os = "android")]
        if let Some(external) = &self.external {
            external.play(&external.paths.backspace);
        }
        #[cfg(not(target_os = "android"))]
        self.play(&self.backspace);
    }

    /// Mix the margin bell and carriage-return travel effect.
    pub fn play_return(&self) {
        #[cfg(target_os = "android")]
        if let Some(external) = &self.external {
            external.play(&external.paths.carriage_return);
        }
        #[cfg(not(target_os = "android"))]
        self.play(&self.carriage_return);
    }

    /// Rodio performs playback on its existing audio thread, so this call does
    /// not block the editor event loop.
    #[cfg(not(target_os = "android"))]
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

    #[cfg(not(target_os = "android"))]
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

#[cfg(not(target_os = "android"))]
fn open_stream(stream_healthy: Arc<AtomicBool>) -> Option<MixerDeviceSink> {
    let callback_state = Arc::clone(&stream_healthy);
    let builder = DeviceSinkBuilder::from_default_device().ok()?;

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

#[cfg(not(target_os = "android"))]
fn stream_error_requires_rebuild(error: &StreamError) -> bool {
    !matches!(error, StreamError::BufferUnderrun)
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

#[cfg(not(target_os = "android"))]
fn stream_retry_allowed(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|last| now.duration_since(last) >= STREAM_RETRY_INTERVAL)
}

#[cfg(not(target_os = "android"))]
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

#[cfg(target_os = "android")]
struct ExternalClipPaths {
    classic_keys: [PathBuf; KEY_VARIANT_COUNT],
    deep_keys: [PathBuf; KEY_VARIANT_COUNT],
    soft_keys: [PathBuf; KEY_VARIANT_COUNT],
    backspace: PathBuf,
    carriage_return: PathBuf,
}

#[cfg(target_os = "android")]
struct ExternalSoundPlayer {
    sender: Option<SyncSender<PathBuf>>,
    worker: Option<JoinHandle<()>>,
    paths: ExternalClipPaths,
    directory: PathBuf,
}

#[cfg(target_os = "android")]
impl ExternalSoundPlayer {
    fn new() -> Option<Self> {
        Self::new_with_command(find_in_path("play-audio")?)
    }

    fn new_with_command(command: PathBuf) -> Option<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("termleaf-audio-{}-{nonce}", std::process::id()));
        fs::create_dir(&directory).ok()?;

        let paths = write_external_clips(&directory);
        let Some(paths) = paths else {
            let _ = fs::remove_dir_all(&directory);
            return None;
        };

        // Android's player blocks for the full clip (roughly 230 ms for a key
        // strike). A small queue and a bounded set of overlapping players keep
        // normal typing responsive without building a long tail of stale
        // sounds after an extreme burst.
        let (sender, receiver) = sync_channel::<PathBuf>(EXTERNAL_SOUND_QUEUE_CAPACITY);
        let worker_directory = directory.clone();
        let worker = thread::Builder::new()
            .name("termleaf-play-audio".into())
            .spawn(move || {
                let mut children = VecDeque::new();
                while let Ok(path) = receiver.recv() {
                    reap_finished_children(&mut children);
                    if children.len() >= MAX_CONCURRENT_EXTERNAL_SOUNDS {
                        if let Some(mut oldest) = children.pop_front() {
                            let _ = oldest.wait();
                        }
                        reap_finished_children(&mut children);
                    }

                    if let Ok(child) = Command::new(&command)
                        .args(["-s", "media"])
                        .arg(path)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                    {
                        children.push_back(child);
                    }
                }
                for mut child in children {
                    let _ = child.wait();
                }
                let _ = fs::remove_dir_all(worker_directory);
            });
        let Ok(worker) = worker else {
            let _ = fs::remove_dir_all(&directory);
            return None;
        };

        Some(Self {
            sender: Some(sender),
            worker: Some(worker),
            paths,
            directory,
        })
    }

    fn play_key(&self, profile: &str, variant: usize) {
        let path = match profile {
            "deep" => &self.paths.deep_keys[variant],
            "soft" => &self.paths.soft_keys[variant],
            _ => &self.paths.classic_keys[variant],
        };
        self.play(path);
    }

    fn play(&self, path: &Path) {
        if let Some(sender) = &self.sender {
            let _ = sender.try_send(path.to_path_buf());
        }
    }
}

#[cfg(target_os = "android")]
fn reap_finished_children(children: &mut VecDeque<Child>) {
    children.retain_mut(|child| matches!(child.try_wait(), Ok(None)));
}

#[cfg(target_os = "android")]
impl Drop for ExternalSoundPlayer {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // The worker normally removes this. Also clean it here if the worker
        // exited before reaching its final cleanup operation.
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(target_os = "android")]
fn write_external_clips(directory: &Path) -> Option<ExternalClipPaths> {
    fn write_group<const N: usize>(
        directory: &Path,
        prefix: &str,
        wavs: [&[u8]; N],
    ) -> Option<[PathBuf; N]> {
        let paths =
            std::array::from_fn(|index| directory.join(format!("{prefix}-{}.wav", index + 1)));
        for (path, wav) in paths.iter().zip(wavs) {
            fs::write(path, wav).ok()?;
        }
        Some(paths)
    }

    let classic_keys = write_group(directory, "classic", TYPEWRITER_WAVS)?;
    let deep_keys = write_group(directory, "deep", TYPEWRITER_DEEP_WAVS)?;
    let soft_keys = write_group(directory, "soft", TYPEWRITER_SOFT_WAVS)?;
    let backspace = directory.join("backspace.wav");
    let carriage_return = directory.join("return.wav");
    fs::write(&backspace, BACKSPACE_WAV).ok()?;
    fs::write(&carriage_return, RETURN_WAV).ok()?;

    Some(ExternalClipPaths {
        classic_keys,
        deep_keys,
        soft_keys,
        backspace,
        carriage_return,
    })
}

#[cfg(target_os = "android")]
fn find_in_path(command: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(command))
        .find(|candidate| candidate.is_file())
}

/// Decode the generator's standard 16-bit little-endian PCM WAV.
#[cfg(any(test, not(target_os = "android")))]
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
    let (sample_bytes, remainder) = data.as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }

    let samples = sample_bytes
        .iter()
        .map(|bytes| i16::from_le_bytes(*bytes) as f32 / i16::MAX as f32)
        .collect();
    Some((channels, sample_rate, samples))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "android")]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(not(target_os = "android"))]
    use std::thread;

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

    #[cfg(not(target_os = "android"))]
    #[test]
    fn backend_failures_require_a_rebuild_but_transient_underruns_do_not() {
        assert!(!stream_error_requires_rebuild(&StreamError::BufferUnderrun));
        assert!(stream_error_requires_rebuild(
            &StreamError::DeviceNotAvailable
        ));
        assert!(stream_error_requires_rebuild(
            &StreamError::StreamInvalidated
        ));
        assert!(stream_error_requires_rebuild(
            &StreamError::BackendSpecific {
                err: rodio::cpal::BackendSpecificError {
                    description: "`alsa::poll()` returned POLLERR".into(),
                },
            }
        ));
    }

    #[cfg(not(target_os = "android"))]
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

    #[cfg(target_os = "android")]
    #[test]
    fn terminal_android_process_prepares_external_audio() {
        let player = ExternalSoundPlayer::new_with_command(PathBuf::from("/system/bin/true"))
            .expect("the external audio worker should start");
        let directory = player.directory.clone();
        assert!(player.paths.classic_keys.iter().all(|path| path.is_file()));
        assert!(player.paths.deep_keys.iter().all(|path| path.is_file()));
        assert!(player.paths.soft_keys.iter().all(|path| path.is_file()));
        assert!(player.paths.backspace.is_file());
        assert!(player.paths.carriage_return.is_file());

        player.play_key("classic", 0);
        drop(player);
        assert!(!directory.exists());
    }

    #[cfg(target_os = "android")]
    #[test]
    fn termux_audio_keeps_normal_typing_bursts_and_overlaps_decay() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "termleaf-audio-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let command = root.join("fake-play-audio");
        let log = root.join("starts");
        fs::write(
            &command,
            format!(
                "#!/system/bin/sh\nprintf x >> '{}'\n/system/bin/sleep 1\n",
                log.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&command, fs::Permissions::from_mode(0o755)).unwrap();

        let player = ExternalSoundPlayer::new_with_command(command)
            .expect("the external audio worker should start");
        let start = Instant::now();
        for variant in 0..KEY_VARIANT_COUNT {
            player.play_key("classic", variant);
        }
        drop(player);

        assert_eq!(fs::read_to_string(log).unwrap(), "xxxx");
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "four one-second clips should overlap instead of playing serially"
        );
        fs::remove_dir_all(root).unwrap();
    }

    /// Exercise the real output callback when diagnosing a supported machine.
    /// This stays ignored because CI and cross-build hosts may have no speaker.
    #[cfg(not(target_os = "android"))]
    #[test]
    #[ignore = "requires a working default audio output device"]
    fn hardware_audio_callback_stays_alive() {
        let player = SoundPlayer::new();
        assert!(player.stream.borrow().is_some(), "audio stream should open");
        player.play_key("classic");
        thread::sleep(Duration::from_millis(500));
        assert!(player.stream_healthy.load(Ordering::Acquire));
    }
}
