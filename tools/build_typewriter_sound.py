#!/usr/bin/env python3
"""Generate Tadak's short mechanical typewriter key sound."""

from __future__ import annotations

import math
import pathlib
import random
import struct
import wave


SAMPLE_RATE = 44_100
DURATION = 0.065
OUTPUT = pathlib.Path(__file__).resolve().parents[1] / "assets" / "typewriter-key.wav"


def burst(t: float, start: float, decay: float, noise: float, tone: float) -> float:
    if t < start:
        return 0.0
    age = t - start
    envelope = math.exp(-age / decay)
    metallic = math.sin(math.tau * 2_350 * age) + 0.45 * math.sin(
        math.tau * 3_850 * age
    )
    return envelope * (noise + tone * metallic)


def main() -> None:
    rng = random.Random(0x7ADAC)
    samples: list[float] = []
    previous_noise = 0.0

    for index in range(round(SAMPLE_RATE * DURATION)):
        t = index / SAMPLE_RATE
        noise = rng.uniform(-1.0, 1.0)
        bright_noise = noise - previous_noise * 0.72
        previous_noise = noise

        # Hard key impact, a low mechanical body resonance, then the typebar
        # and key-cap return striking a fraction later.
        impact = burst(t, 0.0, 0.0045, bright_noise * 0.9, 0.34)
        body = 0.22 * math.sin(math.tau * 165 * t) * math.exp(-t / 0.022)
        return_click = burst(t, 0.021, 0.006, bright_noise * 0.48, 0.22)
        sample = impact + body + return_click

        # Avoid a discontinuity at the end of the file.
        if t > DURATION - 0.008:
            sample *= (DURATION - t) / 0.008
        samples.append(sample)

    peak = max(abs(sample) for sample in samples)
    pcm = b"".join(
        struct.pack("<h", round(max(-1.0, min(1.0, sample / peak * 0.82)) * 32767))
        for sample in samples
    )

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(OUTPUT), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(SAMPLE_RATE)
        output.writeframes(pcm)
    print(f"wrote {len(samples)} samples to {OUTPUT}")


if __name__ == "__main__":
    main()
