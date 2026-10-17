#!/usr/bin/env python3
"""Compares screenshots of a replay (docs/harness.md): two runs of the
original and one with the Rust replacements.

  tools/shot-compare.py ORIG_DIR ORIG2_DIR RUST_DIR [--coarse PREFIX...]

For each .bmp in ORIG_DIR, counts the pixels where the Rust run differs
from the original although the two original runs agree there: animations
driven by the wall clock (the menu's character panels shimmer) differ
between any two runs, so only pixels the original draws the same twice
count, and not their neighbours within RADIUS pixels either (an animated
pixel can match twice by chance). Exits 1 when a screenshot has such
pixels.

Screenshots whose name starts with a `--coarse` prefix are compared by the
mean brightness of BLOCK x BLOCK blocks instead: where the whole picture
moves between runs of the original (the field's camera sways with a timer
that the replay does not pin down), a block may differ by COARSE levels, or
twice what the two original runs differ by, whichever is more, leaving out
the top SKY rows (the field's searchlight beams sweep on the wall clock;
one run measured 38 there). A missing character model changes its blocks
by about 50.
"""
import pathlib
import struct
import sys

RADIUS = 12
# a pixel counts only when a channel differs by more than this: the menu
# cursor's glow pulses on the wall clock, and two original runs can agree on
# a glow pixel by chance (seen: 8 to 15 levels); a missing or moved element
# differs by far more
TOLERANCE = 24
# regions left out per screenshot (x0, y0, x1, y1, top-left origin): the
# main menu's play time blinks its colon once a second of wall time, so two
# original runs can both show it while the port's run catches it off
MASKS = {
    "menu-browse-2.bmp": [(140, 575, 165, 605)],
}
BLOCK = 40
COARSE = 25
SKY = 160  # coarse shots: rows above this are left out (the beams reach the y 120 block)


def block_means(w, h, px):
    """Mean brightness (R+G+B)/3 per BLOCK x BLOCK block, rows of blocks."""
    out = {}
    for i in range(0, w * h * 4, 4):
        x, y = (i // 4) % w, h - 1 - (i // 4) // w
        k = (x // BLOCK, y // BLOCK)
        out[k] = out.get(k, 0) + (px[i] + px[i + 1] + px[i + 2]) / 3
    return {k: v / (BLOCK * BLOCK) for k, v in out.items()}


def pixels(path):
    d = path.read_bytes()
    off = struct.unpack_from("<I", d, 10)[0]
    w, h = struct.unpack_from("<ii", d, 18)
    return w, abs(h), d[off:off + w * abs(h) * 4]


a_dir, b_dir, r_dir = (pathlib.Path(p) for p in sys.argv[1:4])
coarse = tuple(sys.argv[sys.argv.index("--coarse") + 1:]) if "--coarse" in sys.argv else ()
bad = 0
for a_path in sorted(a_dir.glob("*.bmp")):
    w, h, a = pixels(a_path)
    _, _, b = pixels(b_dir / a_path.name)
    _, _, r = pixels(r_dir / a_path.name)
    if coarse and a_path.name.startswith(coarse):
        ma, mb, mr = block_means(w, h, a), block_means(w, h, b), block_means(w, h, r)
        # the top band: the deck's searchlight beams sweep on the wall clock
        for m in (ma, mb, mr):
            for k in [k for k in m if k[1] * BLOCK < SKY]:
                del m[k]
        limit = max(COARSE, 2 * max(abs(ma[k] - mb[k]) for k in ma))
        worst = max(ma, key=lambda k: abs(ma[k] - mr[k]))
        d = abs(ma[worst] - mr[worst])
        print(f"{a_path.name}: largest block difference {d:.1f} at x {worst[0] * BLOCK}, y {worst[1] * BLOCK} (limit {limit:.1f})")
        bad += d > limit
        continue
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
        if not noise[i // 4] and max(abs(a[i + c] - r[i + c]) for c in range(3)) > TOLERANCE:
            mx, my = (i // 4) % w, h - 1 - (i // 4) // w
            if any(x0 <= mx < x1 and y0 <= my < y1 for x0, y0, x1, y1 in MASKS.get(a_path.name, ())):
                continue
            stable_diff += 1
            x, y = (i // 4) % w, h - 1 - (i // 4) // w
            box = [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
    where = f" in x {box[0]}..{box[2]}, y {box[1]}..{box[3]}" if stable_diff else ""
    print(f"{a_path.name}: {stable_diff} pixels differ from the original{where} ({noisy} differ between the original runs)")
    bad += stable_diff > 0
sys.exit(1 if bad else 0)
