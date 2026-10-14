"""Converts the render-target and texture dumps of a `gfxcapture` to PNG.

Usage: uv run --with numpy --with pillow tools/gfx/dumps2png.py <capture dir>
Writes <dir>/png/<name>.png for every `dump` line of frame.txt and every
`texdata` line of objects.txt in an uncompressed format it knows.
"""
import re
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def to_image(raw, w, h, fmt, pitch, rows):
    a = np.frombuffer(raw, np.uint8)[: pitch * rows].reshape(rows, pitch)
    if fmt in (28, 29, 87, 91):  # RGBA8 / BGRA8
        px = a[:h, : 4 * w].reshape(h, w, 4).copy()
        if fmt in (87, 91):
            px = px[..., [2, 1, 0, 3]]
        px[..., 3] = 255
        return Image.fromarray(px, "RGBA")
    if fmt in (44, 45, 46):  # R24G8 depth: 24 bits of depth, view as grey
        d = a[:h, : 4 * w].reshape(h, w, 4).astype(np.uint32)
        z = (d[..., 0] | (d[..., 1] << 8) | (d[..., 2] << 16)) / float(1 << 24)
        lo, hi = np.percentile(z, 1), np.percentile(z, 99)
        g = np.clip((z - lo) / max(hi - lo, 1e-9), 0, 1)
        return Image.fromarray((g * 255).astype(np.uint8), "L")
    if fmt in (61, 65):  # R8 / A8
        return Image.fromarray(a[:h, :w].copy(), "L")
    if fmt == 10:  # RGBA16F
        px = a[:h, : 8 * w].copy().view(np.float16).reshape(h, w, 4).astype(np.float32)
        return Image.fromarray((np.clip(px[..., :3], 0, 1) ** (1 / 2.2) * 255).astype(np.uint8), "RGB")
    return None


def main():
    d = Path(sys.argv[1])
    out = d / "png"
    out.mkdir(exist_ok=True)
    for line in (d / "frame.txt").read_text().splitlines():
        m = re.match(r"dump (\S+)(?: 0x\w+)? (\d+)x(\d+) format (\d+) pitch (\d+) rows (\d+)", line)
        if m:
            name = m.group(1)
            w, h, fmt, pitch, rows = map(int, m.groups()[1:])
            img = to_image((d / f"{name}.raw").read_bytes(), w, h, fmt, pitch, rows)
            if img:
                img.save(out / (Path(name).name + ".png"))
    for line in (d / "objects.txt").read_text().splitlines():
        m = re.match(r"texdata 0x(\w+) (\d+)x(\d+) format (\d+) pitch (\d+) rows (\d+)", line)
        if m:
            w, h, fmt, pitch, rows = map(int, m.groups()[1:])
            img = to_image((d / f"tex/{m.group(1)}.raw").read_bytes(), w, h, fmt, pitch, rows)
            if img:
                img.save(out / f"tex_{m.group(1)}.png")


main()
