#!/usr/bin/env python3
"""Generate the TermZip app icon (1024x1024 PNG) without external libraries.

The mark is the TermZip chevron prompt on a dark rounded square, with a
zinc-to-blue gradient on the glyph.
"""

import math
import struct
import sys
import zlib

SIZE = 1024
RADIUS = 232
BG = (0x09, 0x09, 0x0B, 0xFF)
BG_TOP = (0x16, 0x16, 0x1A, 0xFF)
BORDER = (0x3F, 0x3F, 0x46, 0xFF)
BLUE_START = (0x3B, 0x82, 0xF6, 0xFF)
BLUE_END = (0x93, 0xC5, 0xFD, 0xFF)
FG = (0xFA, 0xFA, 0xFA, 0xFF)


def rounded_square_mask(x: int, y: int) -> bool:
    """Inside the 1024 rounded square with an 8px transparent margin?"""
    margin = 8
    inner = SIZE - 2 * margin
    lx = x - margin
    ly = y - margin
    if lx < 0 or ly < 0 or lx >= inner or ly >= inner:
        return False
    r = RADIUS
    cx = min(max(lx, r), inner - r)
    cy = min(max(ly, r), inner - r)
    dx = lx - cx
    dy = ly - cy
    return dx * dx + dy * dy <= r * r


def edge_distance(x: int, y: int) -> float:
    margin = 8
    inner = SIZE - 2 * margin
    lx = x - margin
    ly = y - margin
    r = RADIUS
    cx = min(max(lx, r), inner - r)
    cy = min(max(ly, r), inner - r)
    dx = lx - cx
    dy = ly - cy
    return math.sqrt(dx * dx + dy * dy)


def segment_distance(px: float, py: float, ax: float, ay: float, bx: float, by: float) -> float:
    abx = bx - ax
    aby = by - ay
    apx = px - ax
    apy = py - ay
    length_sq = abx * abx + aby * aby
    if length_sq == 0:
        return math.hypot(apx, apy)
    t = max(0.0, min(1.0, (apx * abx + apy * aby) / length_sq))
    return math.hypot(px - (ax + abx * t), py - (ay + aby * t))


THICKNESS = 66
HALF = THICKNESS / 2
SEGMENTS = [
    ((338, 312), (562, 512)),
    ((562, 512), (338, 712)),
    ((612, 712), (792, 712)),
]


def glyph_coverage(x: int, y: int) -> tuple[float, float]:
    """(coverage 0..1, gradient t 0..1) for the mark at pixel (x, y)."""
    best = 1e9
    grad_x = 0.0
    for (ax, ay), (bx, by) in SEGMENTS:
        distance = segment_distance(x + 0.5, y + 0.5, ax, ay, bx, by)
        if distance < best:
            best = distance
            grad_x = (x + 0.5) / SIZE
    if best >= HALF:
        return 0.0, 0.0
    return 1.0, grad_x


def pixel(x: int, y: int) -> tuple[int, int, int, int]:
    if not rounded_square_mask(x, y):
        return (0, 0, 0, 0)

    top = BG if y < SIZE / 2 else BG_TOP
    # Vertical two-tone background blend near the middle.
    blend = max(0.0, min(1.0, (y - SIZE * 0.42) / (SIZE * 0.16)))
    color = [
        round(top[0] + (BG_TOP[i] - top[0]) * (1 - blend) * 0 + 0) for i in range(3)
    ]
    # Simpler: base color is BG with subtle top highlight.
    highlight = max(0.0, 1.0 - y / (SIZE * 0.35)) * 0.05
    color = [round(BG[i] + (BG_TOP[i] - BG[i]) * highlight) for i in range(3)]

    distance_to_edge = edge_distance(x, y)
    if distance_to_edge > RADIUS - 3:
        # Anti-aliased outer border ring.
        color = [round(BORDER[i] * 0.55 + color[i] * 0.45) for i in range(3)]

    coverage, grad_t = glyph_coverage(x, y)
    if coverage > 0:
        gradient = [
            round(BLUE_START[i] + (BLUE_END[i] - BLUE_START[i]) * grad_t) for i in range(3)
        ]
        color = [round(gradient[i] * coverage + color[i] * (1 - coverage)) for i in range(3)]

    return (color[0], color[1], color[2], 0xFF)


def png_chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def main() -> None:
    out_path = sys.argv[1] if len(sys.argv) > 1 else "assets/icon.png"
    raw = bytearray()
    for y in range(SIZE):
        raw.append(0)  # filter type 0
        for x in range(SIZE):
            raw.extend(pixel(x, y))

    ihdr = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", ihdr)
        + png_chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + png_chunk(b"IEND", b"")
    )

    with open(out_path, "wb") as handle:
        handle.write(png)
    print(f"wrote {out_path} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()