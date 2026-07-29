#!/usr/bin/env python3
"""Generate Tadak's mechanical key, delete, and carriage-return sounds."""

from __future__ import annotations

import math
import pathlib
import random
import struct
import wave


SAMPLE_RATE = 44_100
ASSETS = pathlib.Path(__file__).resolve().parents[1] / "assets"


def burst(
    t: float,
    start: float,
    decay: float,
    noise: float,
    tone: float,
    frequencies: tuple[float, float],
) -> float:
    if t < start:
        return 0.0
    age = t - start
    envelope = math.exp(-age / decay)
    metallic = math.sin(math.tau * frequencies[0] * age) + 0.45 * math.sin(
        math.tau * frequencies[1] * age
    )
    return envelope * (noise + tone * metallic)


def write_wave(name: str, samples: list[float], peak_level: float = 0.88) -> None:
    # Normalize both effects to the same conservative peak so overlapping
    # keystrokes remain punchy without clipping Rodio's mixer too easily.
    peak = max(abs(sample) for sample in samples)
    pcm = b"".join(
        struct.pack(
            "<h", round(max(-1.0, min(1.0, sample / peak * peak_level)) * 32767)
        )
        for sample in samples
    )
    output_path = ASSETS / name
    ASSETS.mkdir(parents=True, exist_ok=True)
    with wave.open(str(output_path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(SAMPLE_RATE)
        output.writeframes(pcm)
    print(f"wrote {len(samples)} samples to {output_path}")


def key_strike() -> list[float]:
    """A weighty typebar impact followed by a short key-cap return."""
    duration = 0.095
    rng = random.Random(0x7ADAC)
    samples: list[float] = []
    smoothed_noise = 0.0

    for index in range(round(SAMPLE_RATE * duration)):
        t = index / SAMPLE_RATE
        noise = rng.uniform(-1.0, 1.0)
        smoothed_noise = smoothed_noise * 0.76 + noise * 0.24

        # The old effect emphasized 2–4 kHz noise, which read as a light
        # computer-key click. Most energy now sits in the wooden/mechanical
        # body, with just enough metal at the leading edge to identify a
        # typebar strike.
        impact = burst(
            t, 0.0, 0.0085, smoothed_noise * 0.82, 0.42, (760.0, 1_280.0)
        )
        body = (
            0.72 * math.sin(math.tau * 112 * t)
            + 0.34 * math.sin(math.tau * 238 * t)
            + 0.16 * math.sin(math.tau * 475 * t)
        ) * math.exp(-t / 0.038)
        return_click = burst(
            t, 0.027, 0.007, smoothed_noise * 0.34, 0.25, (980.0, 1_620.0)
        )
        sample = impact + body + return_click

        # Avoid a discontinuity at the end of the file.
        if t > duration - 0.012:
            sample *= (duration - t) / 0.012
        samples.append(sample)
    return samples


def deep_strike() -> list[float]:
    """A lower, cabinet-heavy typewriter with a restrained metal edge."""
    duration = 0.11
    rng = random.Random(0xDEE7)
    samples: list[float] = []
    smoothed_noise = 0.0

    for index in range(round(SAMPLE_RATE * duration)):
        t = index / SAMPLE_RATE
        noise = rng.uniform(-1.0, 1.0)
        smoothed_noise = smoothed_noise * 0.82 + noise * 0.18
        impact = burst(
            t, 0.0, 0.011, smoothed_noise * 0.55, 0.34, (540.0, 860.0)
        )
        body = (
            0.9 * math.sin(math.tau * 82 * t)
            + 0.45 * math.sin(math.tau * 164 * t)
            + 0.18 * math.sin(math.tau * 328 * t)
        ) * math.exp(-t / 0.052)
        return_click = burst(
            t, 0.032, 0.009, smoothed_noise * 0.2, 0.16, (720.0, 1_080.0)
        )
        sample = impact + body + return_click
        if t > duration - 0.014:
            sample *= (duration - t) / 0.014
        samples.append(sample)
    return samples


def soft_strike() -> list[float]:
    """A damped office typewriter with felted mechanics."""
    duration = 0.085
    rng = random.Random(0x50F7)
    samples: list[float] = []
    smoothed_noise = 0.0

    for index in range(round(SAMPLE_RATE * duration)):
        t = index / SAMPLE_RATE
        noise = rng.uniform(-1.0, 1.0)
        smoothed_noise = smoothed_noise * 0.88 + noise * 0.12
        impact = burst(
            t, 0.0, 0.013, smoothed_noise * 0.38, 0.24, (620.0, 980.0)
        )
        body = (
            0.44 * math.sin(math.tau * 126 * t)
            + 0.18 * math.sin(math.tau * 252 * t)
        ) * math.exp(-t / 0.035)
        sample = impact + body
        if t > duration - 0.012:
            sample *= (duration - t) / 0.012
        samples.append(sample)
    return samples


def soft_burst(
    t: float,
    start: float,
    attack: float,
    decay: float,
    noise: float,
    tone: float,
    frequencies: tuple[float, float],
) -> float:
    if t < start:
        return 0.0
    age = t - start
    envelope = (1.0 - math.exp(-age / attack)) * math.exp(-age / decay)
    resonance = math.sin(math.tau * frequencies[0] * age) + 0.3 * math.sin(
        math.tau * frequencies[1] * age
    )
    return envelope * (noise + tone * resonance)


def backspace_release() -> list[float]:
    """A gentle felted key release, distinct from a printing strike."""
    duration = 0.075
    rng = random.Random(0xBAC5ACE)
    samples: list[float] = []
    smoothed_noise = 0.0

    for index in range(round(SAMPLE_RATE * duration)):
        t = index / SAMPLE_RATE
        noise = rng.uniform(-1.0, 1.0)
        smoothed_noise = smoothed_noise * 0.86 + noise * 0.14

        # Rounded attacks and a reduced peak keep deletion audible without the
        # sharp ratchet sound of a printing strike.
        release = soft_burst(
            t, 0.0, 0.0035, 0.016, smoothed_noise * 0.16, 0.11, (250.0, 390.0)
        )
        settle = soft_burst(
            t, 0.025, 0.004, 0.012, smoothed_noise * 0.1, 0.07, (310.0, 470.0)
        )
        spring = (
            0.2 * math.sin(math.tau * 68 * t)
            + 0.07 * math.sin(math.tau * 136 * t)
        ) * math.exp(-t / 0.032)
        sample = release + settle + spring
        if t > duration - 0.01:
            sample *= (duration - t) / 0.01
        samples.append(sample)
    return samples


def carriage_return() -> list[float]:
    """A typewriter margin bell followed by carriage travel and a soft stop."""
    duration = 0.48
    rng = random.Random(0xCA771A6E)
    samples: list[float] = []
    smoothed_noise = 0.0

    for index in range(round(SAMPLE_RATE * duration)):
        t = index / SAMPLE_RATE
        noise = rng.uniform(-1.0, 1.0)
        smoothed_noise = smoothed_noise * 0.9 + noise * 0.1

        bell_attack = 1.0 - math.exp(-t / 0.0015)
        bell = bell_attack * math.exp(-t / 0.24) * (
            0.72 * math.sin(math.tau * 1_320 * t)
            + 0.28 * math.sin(math.tau * 2_640 * t)
            + 0.12 * math.sin(math.tau * 3_960 * t)
        )

        travel_age = max(0.0, t - 0.045)
        travelling = 0.045 <= t < 0.32
        travel_envelope = (
            min(1.0, travel_age / 0.025)
            * min(1.0, max(0.0, 0.32 - t) / 0.04)
            if travelling
            else 0.0
        )
        carriage = travel_envelope * (
            smoothed_noise * 0.3
            + 0.12 * math.sin(math.tau * 92 * travel_age)
            + 0.07 * math.sin(math.tau * 184 * travel_age)
        )
        stop = soft_burst(
            t, 0.32, 0.002, 0.038, smoothed_noise * 0.22, 0.2, (110.0, 260.0)
        )
        sample = bell + carriage + stop
        if t > duration - 0.025:
            sample *= (duration - t) / 0.025
        samples.append(sample)
    return samples


def main() -> None:
    write_wave("typewriter-key.wav", key_strike())
    write_wave("typewriter-key-deep.wav", deep_strike())
    write_wave("typewriter-key-soft.wav", soft_strike(), peak_level=0.72)
    write_wave("typewriter-backspace.wav", backspace_release(), peak_level=0.32)
    write_wave("typewriter-return.wav", carriage_return(), peak_level=0.68)


if __name__ == "__main__":
    main()
