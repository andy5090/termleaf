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
    """A lever touch, quiet carriage travel, stop clack, and high margin bell."""
    duration = 0.48
    rng = random.Random(0xCA771A6E)
    samples: list[float] = []
    smoothed_noise = 0.0
    rack_times = [0.058, 0.087, 0.119, 0.154, 0.183]

    for index in range(round(SAMPLE_RATE * duration)):
        t = index / SAMPLE_RATE
        noise = rng.uniform(-1.0, 1.0)
        smoothed_noise = smoothed_noise * 0.78 + noise * 0.22

        # The reference recording is transient-first: a short lever contact,
        # followed by a mostly quiet mechanical travel interval. Avoiding a
        # sustained low tone keeps this from sounding electronically looped.
        lever = burst(t, 0.0, 0.006, noise * 0.34, 0.13, (290.0, 880.0))
        lever_body = 0.08 * math.sin(math.tau * 170.0 * t) * math.exp(-t / 0.018)

        if 0.035 <= t < 0.205:
            travel_age = t - 0.035
            travel = (
                smoothed_noise * 0.018
                + 0.009 * math.sin(math.tau * 71.0 * travel_age)
            ) * math.sin(math.pi * travel_age / 0.17)
        else:
            travel = 0.0

        rack = 0.0
        for click_time in rack_times:
            click_age = t - click_time
            if 0.0 <= click_age < 0.006:
                rack += math.exp(-click_age / 0.0015) * (
                    noise * 0.055
                    + 0.025 * math.sin(math.tau * 980.0 * click_age)
                )

        # The real acoustic signature arrives at the end of travel: one broad
        # stop transient excites a small, high bell. Sparse inharmonic partials
        # resemble metal without turning into a clean electronic chord.
        stop = burst(t, 0.213, 0.012, noise * 0.5, 0.16, (240.0, 1_180.0))
        bell_age = t - 0.222
        if bell_age >= 0.0:
            bell_attack = 1.0 - math.exp(-bell_age / 0.00045)
            bell = bell_attack * (
                0.5
                * math.sin(math.tau * 2_620.0 * bell_age)
                * math.exp(-bell_age / 0.16)
                + 0.14
                * math.sin(math.tau * 5_730.0 * bell_age + 0.45)
                * math.exp(-bell_age / 0.075)
                + 0.055
                * math.sin(math.tau * 10_180.0 * bell_age + 1.1)
                * math.exp(-bell_age / 0.038)
            )
            bell_strike = noise * 0.22 * math.exp(-bell_age / 0.006)
        else:
            bell = 0.0
            bell_strike = 0.0

        sample = lever + lever_body + travel + rack + stop + bell_strike + bell
        if t > duration - 0.025:
            sample *= (duration - t) / 0.025
        samples.append(sample)
    return samples


def main() -> None:
    write_wave("typewriter-key.wav", key_strike())
    write_wave("typewriter-key-deep.wav", deep_strike())
    write_wave("typewriter-key-soft.wav", soft_strike(), peak_level=0.72)
    write_wave("typewriter-backspace.wav", backspace_release(), peak_level=0.32)
    write_wave("typewriter-return.wav", carriage_return(), peak_level=0.58)


if __name__ == "__main__":
    main()
