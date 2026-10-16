#!/usr/bin/env python3
"""Compares screenshots of a replay (docs/harness.md): two runs of the
original and one with the Rust replacements.

  tools/shot-compare.py ORIG_DIR ORIG2_DIR RUST_DIR

For each .bmp in ORIG_DIR, counts the pixels where the Rust run differs
from the original although the two original runs agree there: animations
driven by the wall clock (the menu's character panels shimmer) differ
between any two runs, so only pixels the original draws the same twice
count, and not their neighbours within RADIUS pixels either (an animated
pixel can match twice by chance). Exits 1 when a screenshot has such
pixels.
"""
import pathlib
import struct
import sys

RADIUS = 12


def pixels(path):
    d = path.read_bytes()
    off = struct.unpack_from("<I", d, 10)[0]
    w, h = struct.unpack_from("<ii", d, 18)
    return w, abs(h), d[off:off + w * abs(h) * 4]


a_dir, b_dir, r_dir = (pathlib.Path(p) for p in sys.argv[1:4])
bad = 0
for a_path in sorted(a_dir.glob("*.bmp")):
    w, h, a = pixels(a_path)
    _, _, b = pixels(b_dir / a_path.name)
    _, _, r = pixels(r_dir / a_path.name)
    noise = bytearray(w * h)
    noisy = 0
    for i in range(0, w * h * 4, 4):
        if a[i:i + 3] != b[i:i + 3]:
            noisy += 1
            x, y = (i // 4) % w, (i // 4) // w
            for yy in range(max(0, y - RADIUS), min(h, y + RADIUS + 1)):
                noise[yy * w + max(0, x - RADIUS):yy * w + min(w, x + RADIUS + 1)] = b"\1" * (min(w, x + RADIUS + 1) - max(0, x - RADIUS))
    stable_diff = 0
    box = [w, h, -1, -1]
    for i in range(0, w * h * 4, 4):
        if not noise[i // 4] and a[i:i + 3] != r[i:i + 3]:
            stable_diff += 1
            x, y = (i // 4) % w, h - 1 - (i // 4) // w
            box = [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
    where = f" in x {box[0]}..{box[2]}, y {box[1]}..{box[3]}" if stable_diff else ""
    print(f"{a_path.name}: {stable_diff} pixels differ from the original{where} ({noisy} differ between the original runs)")
    bad += stable_diff > 0
sys.exit(1 if bad else 0)
