#!/usr/bin/env python3
"""Build Termleaf's English core and optional language glyph packs.

The core stores dense printable ASCII records. Language packs use a sparse
format so Japanese can include every CJK glyph available in Galmuri without
pretending the font covers the entire Unicode CJK block.
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
CORE_OUTPUT = ROOT / "assets" / f"galmuri9-{VERSION}-core.bin"
PACK_ROOT = ROOT / "language-packs"
CANVAS_TOP = 8
CANVAS_BOTTOM = -1
CANVAS_HEIGHT = CANVAS_TOP - CANVAS_BOTTOM + 1
RECORD_SIZE = 1 + CANVAS_HEIGHT * 2
PACK_MAGIC = b"TLGP1"

ASCII = range(0x0020, 0x007F)
KOREAN_RANGES = ((0x3131, 0x3164), (0xAC00, 0xD7A4))
JAPANESE_RANGES = (
    (0x3000, 0x3040),
    (0x3040, 0x30A0),
    (0x30A0, 0x3100),
    (0x4E00, 0xA000),
    (0xFF01, 0xFFA0),
)


def in_ranges(codepoint: int, ranges: tuple[tuple[int, int], ...]) -> bool:
    return any(start <= codepoint < end for start, end in ranges)


def wanted(codepoint: int) -> bool:
    return (
        codepoint in ASCII
        or in_ranges(codepoint, KOREAN_RANGES)
        or in_ranges(codepoint, JAPANESE_RANGES)
    )


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
        # Combining marks with zero advance are rendered by the terminal's IME
        # path; the enlarged bitmap view only stores standalone glyph cells.
        if advance == 0:
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


def write_dense(
    path: pathlib.Path,
    codepoints: list[int],
    glyphs: dict[int, tuple[int, list[int]]],
) -> None:
    missing = [codepoint for codepoint in codepoints if codepoint not in glyphs]
    if missing:
        raise RuntimeError(f"missing dense glyph U+{missing[0]:04X}")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as output:
        for codepoint in codepoints:
            advance, rows = glyphs[codepoint]
            output.write(bytes([advance]))
            for row in rows:
                output.write(struct.pack(">H", row))
    expected_size = len(codepoints) * RECORD_SIZE
    if path.stat().st_size != expected_size:
        raise RuntimeError(f"expected {expected_size} bytes at {path}")


def write_sparse(
    path: pathlib.Path,
    codepoints: list[int],
    glyphs: dict[int, tuple[int, list[int]]],
) -> None:
    available = [codepoint for codepoint in codepoints if codepoint in glyphs]
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as output:
        output.write(PACK_MAGIC)
        output.write(struct.pack(">I", len(available)))
        for codepoint in available:
            advance, rows = glyphs[codepoint]
            output.write(struct.pack(">I", codepoint))
            output.write(bytes([advance]))
            for row in rows:
                output.write(struct.pack(">H", row))
    print(f"wrote {len(available)} sparse glyphs to {path}")


def expand(ranges: tuple[tuple[int, int], ...]) -> list[int]:
    return [codepoint for start, end in ranges for codepoint in range(start, end)]


def main() -> None:
    with urllib.request.urlopen(URL) as response:
        glyphs = parse_bdf(response.read().decode("utf-8"))

    write_dense(CORE_OUTPUT, list(ASCII), glyphs)
    print(f"wrote {len(ASCII)} core glyphs to {CORE_OUTPUT}")
    write_sparse(PACK_ROOT / "ko" / "glyphs.bin", expand(KOREAN_RANGES), glyphs)
    write_sparse(PACK_ROOT / "ja" / "glyphs.bin", expand(JAPANESE_RANGES), glyphs)


if __name__ == "__main__":
    main()
