#!/usr/bin/env python3
"""Build Termleaf's optimized sounds from CC0 typewriter recordings."""

from __future__ import annotations

import hashlib
import math
import pathlib
import struct
import tempfile
import urllib.request
import wave


SAMPLE_RATE = 44_100
ASSETS = pathlib.Path(__file__).resolve().parents[1] / "assets"
USER_AGENT = "termleaf-audio-builder/1.0"

# Hermes Precisa 305 recordings by Joseph SARDIN, dedicated to the public
# domain under CC0 1.0. The hashes pin the exact masters used for the assets.
SOURCES = {
    "typing": (
        "https://bigsoundbank.com/UPLOAD/bwf-en/2841.wav",
        "ebde693739503bc1b213c97faa746bffeaf9f1a1239c047bc035d3469eed67e3",
    ),
    "space": (
        "https://bigsoundbank.com/UPLOAD/bwf-en/2843.wav",
        "f681dda9b53feaf75540ab3310398b37855173d694b55cb183c846e076fafe9e",
    ),
    "bell": (
        "https://bigsoundbank.com/UPLOAD/bwf-en/2844.wav",
        "deb29f0d2729450e0b2b0f7d3f3259811579c30e688822acec06e519372b0661",
    ),
}

# Per-strike adjustments stay below the threshold where they read as an
# obvious effect: speed changes pitch by roughly one percent, while the filter,
# level, and tail offsets reinforce the differences already in the recordings.
VARIANT_TUNING = [
    (0.990, 0.94, 0.92, 0.97, 0.008),
    (0.997, 1.00, 1.00, 1.00, 0.000),
    (1.008, 1.04, 1.07, 0.95, -0.006),
    (1.015, 1.08, 1.12, 1.02, -0.010),
]


def download_source(name: str, directory: pathlib.Path) -> pathlib.Path:
    url, expected_digest = SOURCES[name]
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request) as response:
        data = response.read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != expected_digest:
        raise ValueError(f"unexpected SHA-256 for {name}: {digest}")
    path = directory / f"{name}.wav"
    path.write_bytes(data)
    return path


def read_wave(path: pathlib.Path) -> tuple[int, list[float]]:
    with wave.open(str(path), "rb") as source:
        if source.getnchannels() != 1:
            raise ValueError(f"{path} must be mono")
        sample_rate = source.getframerate()
        sample_width = source.getsampwidth()
        frames = source.readframes(source.getnframes())

    if sample_width == 2:
        samples = [
            value[0] / 32_768.0 for value in struct.iter_unpack("<h", frames)
        ]
    elif sample_width == 3:
        samples = [
            int.from_bytes(frames[index : index + 3], "little", signed=True)
            / 8_388_608.0
            for index in range(0, len(frames), 3)
        ]
    else:
        raise ValueError(f"unsupported {sample_width * 8}-bit PCM in {path}")
    return sample_rate, samples


def trim(
    samples: list[float], sample_rate: int, start: float, end: float
) -> list[float]:
    return samples[round(start * sample_rate) : round(end * sample_rate)]


def resample(
    samples: list[float], source_rate: int, speed: float = 1.0
) -> list[float]:
    output_length = round(len(samples) * SAMPLE_RATE / source_rate / speed)
    output: list[float] = []
    for index in range(output_length):
        position = index * source_rate * speed / SAMPLE_RATE
        left = min(int(position), len(samples) - 1)
        right = min(left + 1, len(samples) - 1)
        fraction = position - left
        output.append(samples[left] * (1.0 - fraction) + samples[right] * fraction)
    return output


def high_pass(samples: list[float], cutoff: float) -> list[float]:
    time_step = 1.0 / SAMPLE_RATE
    resistance = 1.0 / (math.tau * cutoff)
    alpha = resistance / (resistance + time_step)
    output: list[float] = []
    previous_input = 0.0
    previous_output = 0.0
    for sample in samples:
        filtered = alpha * (previous_output + sample - previous_input)
        output.append(filtered)
        previous_input = sample
        previous_output = filtered
    return output


def remove_rumble(samples: list[float], cutoff: float) -> list[float]:
    """Apply a gentle two-pole high-pass without an external DSP dependency."""
    return high_pass(high_pass(samples, cutoff), cutoff)


def low_pass(samples: list[float], cutoff: float) -> list[float]:
    time_step = 1.0 / SAMPLE_RATE
    resistance = 1.0 / (math.tau * cutoff)
    alpha = time_step / (resistance + time_step)
    output: list[float] = []
    previous = 0.0
    for sample in samples:
        previous += alpha * (sample - previous)
        output.append(previous)
    return output


def fade(samples: list[float], fade_in: float, fade_out: float) -> list[float]:
    output = samples.copy()
    fade_in_samples = min(round(fade_in * SAMPLE_RATE), len(output))
    fade_out_samples = min(round(fade_out * SAMPLE_RATE), len(output))
    for index in range(fade_in_samples):
        output[index] *= index / max(1, fade_in_samples)
    for age in range(fade_out_samples):
        output[-1 - age] *= age / max(1, fade_out_samples)
    return output


def normalize(samples: list[float], peak_level: float) -> list[float]:
    peak = max(abs(sample) for sample in samples)
    return [sample / peak * peak_level for sample in samples]


def prepare_key_variants(
    source: list[float],
    source_rate: int,
    *,
    speed: float,
    high: float,
    low: float,
    peak: float,
) -> list[list[float]]:
    # Four separated strikes from the slow-typing master preserve differences
    # in typebar position and touch. Their relative levels remain intact.
    onsets = [0.24, 0.63, 1.055, 1.29]
    variants: list[list[float]] = []
    levels: list[float] = []
    for onset, tuning in zip(onsets, VARIANT_TUNING, strict=True):
        speed_scale, high_scale, low_scale, level, tail_offset = tuning
        samples = trim(
            source, source_rate, onset - 0.012, onset + 0.216 + tail_offset
        )
        samples = resample(samples, source_rate, speed * speed_scale)
        samples = remove_rumble(samples, high * high_scale)
        samples = low_pass(samples, low * low_scale)
        variants.append(fade(samples, 0.004, 0.025))
        levels.append(level)

    loudest_peak = max(
        max(abs(sample) for sample in samples) * level
        for samples, level in zip(variants, levels, strict=True)
    )
    return [
        [sample / loudest_peak * peak * level for sample in samples]
        for samples, level in zip(variants, levels, strict=True)
    ]


def prepare_backspace(source: list[float], source_rate: int) -> list[float]:
    # The recorded space bar has a natural two-stage press/release action that
    # remains recognizably different from a printing-key strike.
    samples = trim(source, source_rate, 0.068, 0.31)
    samples = resample(samples, source_rate)
    samples = remove_rumble(samples, 380.0)
    samples = low_pass(samples, 7_000.0)
    samples = fade(samples, 0.004, 0.03)
    return normalize(samples, 0.23)


def mix(tracks: list[tuple[list[float], float]], duration: float) -> list[float]:
    output = [0.0] * round(duration * SAMPLE_RATE)
    for samples, start in tracks:
        offset = round(start * SAMPLE_RATE)
        for index, sample in enumerate(samples[: len(output) - offset]):
            output[offset + index] += sample
    return output


def prepare_return(
    space_source: list[float], space_rate: int, bell_source: list[float], bell_rate: int
) -> list[float]:
    mechanism = trim(space_source, space_rate, 0.068, 0.34)
    mechanism = resample(mechanism, space_rate, 0.94)
    mechanism = remove_rumble(mechanism, 300.0)
    mechanism = low_pass(mechanism, 7_500.0)
    mechanism = fade(mechanism, 0.004, 0.035)
    mechanism = normalize(mechanism, 0.24)

    bell = trim(bell_source, bell_rate, 0.03, 0.70)
    bell = resample(bell, bell_rate)
    bell = remove_rumble(bell, 420.0)
    bell = low_pass(bell, 12_000.0)
    bell = fade(bell, 0.003, 0.06)
    bell = normalize(bell, 0.3)

    combined = mix([(mechanism, 0.0), (bell, 0.13)], 0.8)
    combined = fade(combined, 0.002, 0.04)
    return normalize(combined, 0.42)


def write_wave(name: str, samples: list[float]) -> None:
    pcm = b"".join(
        struct.pack("<h", round(max(-1.0, min(1.0, sample)) * 32_767))
        for sample in samples
    )
    output_path = ASSETS / name
    with wave.open(str(output_path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(SAMPLE_RATE)
        output.writeframes(pcm)
    print(f"wrote {len(samples)} samples to {output_path}")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="termleaf-audio-") as temporary:
        directory = pathlib.Path(temporary)
        typing_rate, typing_source = read_wave(download_source("typing", directory))
        space_rate, space_source = read_wave(download_source("space", directory))
        bell_rate, bell_source = read_wave(download_source("bell", directory))

    profiles = {
        "typewriter-key": prepare_key_variants(
            typing_source,
            typing_rate,
            speed=1.0,
            high=300.0,
            low=10_500.0,
            peak=0.46,
        ),
        "typewriter-key-deep": prepare_key_variants(
            typing_source,
            typing_rate,
            speed=0.9,
            high=280.0,
            low=6_000.0,
            peak=0.42,
        ),
        "typewriter-key-soft": prepare_key_variants(
            typing_source,
            typing_rate,
            speed=0.97,
            high=390.0,
            low=3_600.0,
            peak=0.28,
        ),
    }
    for base_name, variants in profiles.items():
        for index, samples in enumerate(variants, start=1):
            suffix = "" if index == 1 else f"-{index}"
            write_wave(f"{base_name}{suffix}.wav", samples)
    write_wave("typewriter-backspace.wav", prepare_backspace(space_source, space_rate))
    write_wave(
        "typewriter-return.wav",
        prepare_return(space_source, space_rate, bell_source, bell_rate),
    )


if __name__ == "__main__":
    main()
