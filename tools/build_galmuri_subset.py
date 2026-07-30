#!/usr/bin/env python3
"""Build Termleaf's fixed-width Galmuri9 bitmap subset.

Each output record contains one advance-width byte followed by ten big-endian
u16 bitmap rows. Records are ordered as printable ASCII U+0020..U+007E,
Hangul Compatibility Jamo U+3131..U+3163, and Hangul U+AC00..U+D7A3.
"""

from __future__ import annotations

import pathlib
import struct
import urllib.request


VERSION = "2.40.4"
URL = (
    "https://raw.githubusercontent.com/quiple/galmuri/"
    f"v{VERSION}/dist/Galmuri9.bdf"
)
ROOT = pathlib.Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / f"galmuri9-{VERSION}-termleaf.bin"
RANGES = ((0x0020, 0x007E), (0x3131, 0x3163), (0xAC00, 0xD7A3))
CANVAS_TOP = 8
CANVAS_BOTTOM = -1
CANVAS_HEIGHT = CANVAS_TOP - CANVAS_BOTTOM + 1


def wanted(codepoint: int) -> bool:
    return any(start <= codepoint <= end for start, end in RANGES)


def parse_bdf(text: str) -> dict[int, tuple[int, list[int]]]:
    glyphs: dict[int, tuple[int, list[int]]] = {}
    lines = iter(text.splitlines())
    for line in lines:
        if not line.startswith("STARTCHAR"):
            continue

        encoding = -1
        advance = 0
        bbx = (0, 0, 0, 0)
        bitmap: list[str] = []
        for field in lines:
            if field.startswith("ENCODING "):
                encoding = int(field.split()[1])
            elif field.startswith("DWIDTH "):
                advance = int(field.split()[1])
            elif field.startswith("BBX "):
                _, width, height, x_offset, y_offset = field.split()
                bbx = tuple(map(int, (width, height, x_offset, y_offset)))
            elif field == "BITMAP":
                for row in lines:
                    if row == "ENDCHAR":
                        break
                    bitmap.append(row)
                break
            elif field == "ENDCHAR":
                break

        if encoding < 0 or not wanted(encoding):
            continue
        if not 1 <= advance <= 16:
            raise ValueError(f"U+{encoding:04X} has unsupported advance {advance}")

        width, height, x_offset, y_offset = bbx
        rows = [0] * CANVAS_HEIGHT
        for source_row, encoded_row in enumerate(bitmap):
            y = y_offset + height - 1 - source_row
            if not CANVAS_BOTTOM <= y <= CANVAS_TOP:
                continue
            target_row = CANVAS_TOP - y
            bits = int(encoded_row, 16)
            storage_width = len(encoded_row) * 4
            for source_x in range(width):
                if bits & (1 << (storage_width - 1 - source_x)):
                    x = x_offset + source_x
                    if 0 <= x < advance:
                        rows[target_row] |= 1 << (advance - 1 - x)
        glyphs[encoding] = (advance, rows)
    return glyphs


def main() -> None:
    with urllib.request.urlopen(URL) as response:
        glyphs = parse_bdf(response.read().decode("utf-8"))

    expected = [cp for start, end in RANGES for cp in range(start, end + 1)]
    missing = [cp for cp in expected if cp not in glyphs]
    if missing:
        raise RuntimeError(f"missing {len(missing)} glyphs; first is U+{missing[0]:04X}")

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT.open("wb") as output:
        for codepoint in expected:
            advance, rows = glyphs[codepoint]
            output.write(bytes([advance]))
            for row in rows:
                output.write(struct.pack(">H", row))

    record_size = 1 + CANVAS_HEIGHT * 2
    expected_size = len(expected) * record_size
    actual_size = OUTPUT.stat().st_size
    if actual_size != expected_size:
        raise RuntimeError(f"expected {expected_size} bytes, wrote {actual_size}")
    print(f"wrote {len(expected)} glyphs ({actual_size} bytes) to {OUTPUT}")


if __name__ == "__main__":
    main()
