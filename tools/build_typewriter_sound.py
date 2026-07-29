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
    """A lever clack, margin bell, ratcheting carriage sweep, and end stop."""
    duration = 0.56
    rng = random.Random(0xCA771A6E)
    samples: list[float] = []
    smoothed_noise = 0.0
    ratchet_times: list[float] = []
    ratchet_time = 0.067
    ratchet_interval = 0.012
    while ratchet_time < 0.305:
        ratchet_times.append(ratchet_time)
        # The rack slows slightly as the carriage reaches its stop.
        ratchet_interval += 0.00055
        ratchet_time += ratchet_interval

    for index in range(round(SAMPLE_RATE * duration)):
        t = index / SAMPLE_RATE
        noise = rng.uniform(-1.0, 1.0)
        smoothed_noise = smoothed_noise * 0.82 + noise * 0.18

        # A return begins with the lever and line-feed pawl engaging. Keeping
        # this very short prevents it from reading as a second key strike.
        lever = burst(t, 0.0, 0.008, smoothed_noise * 0.5, 0.25, (185.0, 620.0))
        feed_pawl = burst(
            t, 0.027, 0.0055, smoothed_noise * 0.28, 0.14, (410.0, 930.0)
        )

        # Real bells are inharmonic. The old exact 1x/2x/3x stack sounded like
        # an electronic chime, so these partials ring at independent ratios.
        bell_age = t - 0.014
        if bell_age >= 0.0:
            bell_attack = 1.0 - math.exp(-bell_age / 0.00065)
            bell = bell_attack * (
                0.52
                * math.sin(math.tau * 1_510 * bell_age)
                * math.exp(-bell_age / 0.25)
                + 0.23
                * math.sin(math.tau * 2_093 * bell_age + 0.3)
                * math.exp(-bell_age / 0.17)
                + 0.11
                * math.sin(math.tau * 3_427 * bell_age + 0.8)
                * math.exp(-bell_age / 0.09)
            )
        else:
            bell = 0.0

        travel_start = 0.052
        travel_end = 0.325
        travel_age = t - travel_start
        if travel_start <= t < travel_end:
            progress = travel_age / (travel_end - travel_start)
            travel_envelope = math.sin(math.pi * progress) ** 0.65
            slide = travel_envelope * (
                smoothed_noise * 0.095
                + 0.035
                * math.sin(math.tau * (58.0 * travel_age + 13.0 * travel_age**2))
            )
        else:
            slide = 0.0

        # Closely-spaced rack clicks turn the slide into a recognizable
        # carriage return instead of a featureless burst of noise.
        ratchet = 0.0
        for click_time in ratchet_times:
            click_age = t - click_time
            if 0.0 <= click_age < 0.009:
                click_envelope = math.exp(-click_age / 0.0022)
                ratchet += click_envelope * (
                    smoothed_noise * 0.19
                    + 0.075 * math.sin(math.tau * 760.0 * click_age)
                    + 0.035 * math.sin(math.tau * 1_260.0 * click_age)
                )

        # The stop is a damped cabinet thump, not a bright metal impact.
        stop = burst(
            t, 0.326, 0.022, smoothed_noise * 0.34, 0.2, (92.0, 235.0)
        )
        cabinet_age = t - 0.326
        cabinet = (
            (
                0.2 * math.sin(math.tau * 74.0 * cabinet_age)
                + 0.08 * math.sin(math.tau * 148.0 * cabinet_age)
            )
            * math.exp(-cabinet_age / 0.055)
            if cabinet_age >= 0.0
            else 0.0
        )

        sample = lever + feed_pawl + bell + slide + ratchet + stop + cabinet
        if t > duration - 0.035:
            sample *= (duration - t) / 0.035
        samples.append(sample)
    return samples


def main() -> None:
    write_wave("typewriter-key.wav", key_strike())
    write_wave("typewriter-key-deep.wav", deep_strike())
    write_wave("typewriter-key-soft.wav", soft_strike(), peak_level=0.72)
    write_wave("typewriter-backspace.wav", backspace_release(), peak_level=0.32)
    write_wave("typewriter-return.wav", carriage_return(), peak_level=0.62)


if __name__ == "__main__":
    main()
