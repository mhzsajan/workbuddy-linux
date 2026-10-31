#!/usr/bin/env python3
"""Generate the Kestrel app icon.

Pure standard library on purpose: contributors should be able to regenerate the
icon without installing Pillow or ImageMagick.

    python scripts/make_icon.py            # writes assets/icon.png (1024x1024)
    npx tauri icon assets/icon.png         # derives every platform size
"""

import os
import struct
import zlib

SIZE = 1024
SS = 2  # supersampling factor for anti-aliasing

# Brand colours (match --accent in src/styles.css).
TOP = (0xB0, 0x53, 0x1F)
BOTTOM = (0x7A, 0x3A, 0x12)
MARK = (0xFA, 0xF8, 0xF4)

# Normalised chevron, same silhouette as the CSS brand mark.
CHEVRON = [
    (0.50, 0.00),
    (1.00, 0.78),
    (0.72, 0.78),
    (0.50, 0.44),
    (0.28, 0.78),
    (0.00, 0.78),
]


def in_polygon(x, y, poly):
    inside = False
    n = len(poly)
    j = n - 1
    for i in range(n):
        xi, yi = poly[i]
        xj, yj = poly[j]
        if (yi > y) != (yj > y):
            xint = (xj - xi) * (y - yi) / (yj - yi) + xi
            if x < xint:
                inside = not inside
        j = i
    return inside


def rounded_rect_alpha(x, y, size, radius):
    """Coverage of a rounded square, in normalised 0..1 coordinates."""
    cx = min(max(x, radius), 1 - radius)
    cy = min(max(y, radius), 1 - radius)
    dx = x - cx
    dy = y - cy
    dist = (dx * dx + dy * dy) ** 0.5
    if dist <= radius:
        return 1.0
    return 0.0


def build():
    hi = SIZE * SS
    radius = 0.22

    # Scale the chevron into the middle 62% of the canvas.
    scale = 0.62
    offset = (1 - scale) / 2
    chevron = [
        (offset + px * scale, offset + py * scale) for px, py in CHEVRON
    ]

    rows = []
    for py in range(SIZE):
        row = bytearray()
        for px in range(SIZE):
            r = g = b = a = 0
            for sy in range(SS):
                for sx in range(SS):
                    x = (px * SS + sx + 0.5) / hi
                    y = (py * SS + sy + 0.5) / hi
                    if rounded_rect_alpha(x, y, 1.0, radius) == 0.0:
                        continue
                    if in_polygon(x, y, chevron):
                        cr, cg, cb = MARK
                    else:
                        t = y
                        cr = int(TOP[0] + (BOTTOM[0] - TOP[0]) * t)
                        cg = int(TOP[1] + (BOTTOM[1] - TOP[1]) * t)
                        cb = int(TOP[2] + (BOTTOM[2] - TOP[2]) * t)
                    r += cr
                    g += cg
                    b += cb
                    a += 255
            samples = SS * SS
            if a == 0:
                row += bytes((0, 0, 0, 0))
            else:
                covered = a // 255
                row += bytes(
                    (r // covered, g // covered, b // covered, a // samples)
                )
        rows.append(bytes(row))
    return rows


def write_png(path, width, height, rows):
    raw = b"".join(b"\x00" + row for row in rows)

    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    blob = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    with open(path, "wb") as fh:
        fh.write(blob)


if __name__ == "__main__":
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out = os.path.join(root, "assets", "icon.png")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    write_png(out, SIZE, SIZE, build())
    print(f"wrote {out}")
