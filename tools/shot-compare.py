#!/usr/bin/env python3
"""Compares screenshots of a replay (docs/harness.md): two runs of the
original and one with the Rust replacements.

  tools/shot-compare.py ORIG_DIR ORIG2_DIR RUST_DIR [--only REPLAY,...] [--coarse PREFIX...]

`--only` compares only the named replays' shots (`<replay>-<n>.bmp`), for
a run of some replays of a list (`run-all.sh <replay>...`).

For each .bmp in ORIG_DIR, counts the pixels where the Rust run differs
from the original although the two original runs agree there: animations
driven by the wall clock (the menu's character panels shimmer) differ
between any two runs, so only pixels the original draws the same twice
count, and not their neighbours within RADIUS pixels either (an animated
pixel can match twice by chance). Exits 1 when a screenshot has such
pixels.

Screenshots whose name starts with a `--coarse` prefix (the field) are
compared by BLOCK x BLOCK block means instead (`compare_coarse`): the deck sways
and its light changes between runs, original or not, because the asset
loader's timing moves the map's animation phase. The scene below the SKY
rows is aligned to the original first (one shift for the whole picture,
up to SWAY_X, SWAY_Y pixels: runs measured 0 across and -9 to +6 down),
each block may then match within LOCAL_X, LOCAL_Y pixels of that (the
railing and the near deck move more than the far deck), and each colour
channel is compared after removing its median difference (the light
step). A block's limit grows with the edges in it, BLOCK_LIMIT + EDGE x
its mean gradient: edges are what a leftover sub-pixel or parallax shift
turns into a difference, while a missing model on the flat deck shows
against a low limit; the limit is also at least twice the difference
between the two original runs. The UI (the minimap, which does not sway)
is compared unshifted against UI_LIMIT: a picture shifted as a whole
moves it.
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
# a screenshot fails only with more than this many differing pixels in one
# 2-pixel-wide column band: the menu cursor's glow leaves thin vertical
# streaks (2 px wide, up to ~20 tall) that two original runs can share by
# chance; anything real (a missing or moved element) is wider than 2 px
STREAK_WIDTH = 2
# regions left out per screenshot (x0, y0, x1, y1, top-left origin): the
# main menu's play time blinks its colon once a second of wall time, so two
# original runs can both show it while the port's run catches it off
MASKS = {
    # the play-time clock (x 149-181, y 582-597): its colon blinks and its
    # minutes can tick between recordings
    "menu-browse-1.bmp": [(140, 575, 190, 605)],
    "menu-browse-2.bmp": [(140, 575, 190, 605)],
    "menu-browse-3.bmp": [(140, 575, 190, 605)],
    "menu-browse-4.bmp": [(140, 575, 190, 605)],
    "menu-browse-5.bmp": [(140, 575, 190, 605)],
    # the searchlight beams below the SKY rows, where they cross the sky
    # behind the crane (x 880-1280, y 240-400 in field-walk-1, up to 31
    # seen; x 600-700, y 240-320 in field-walk-2, 12 seen), the beam's
    # lower end (blocks at y 240, x 1000-1120; up to 41 seen) and the save
    # sphere's pulsing glow (field-walk-1 only; 21.5 seen)
    "field-walk-1.bmp": [(880, 240, 1280, 400), (480, 300, 580, 420)],
    "field-walk-2.bmp": [(1000, 240, 1120, 280), (600, 240, 700, 320)],
    # the chocobo icon at the bottom right of the Besaid path maps (0x14,
    # 0x29) animates by the wall clock (tools/replays/event-besaid.txt)
    "event-besaid-2.bmp": [(1200, 640, 1280, 720)],
    "event-besaid-3.bmp": [(1200, 640, 1280, 720)],
    "event-besaid-4.bmp": [(1200, 640, 1280, 720)],
    "event-besaid-5.bmp": [(1200, 640, 1280, 720)],
    # the same icon on the Thunder Plains outside the agency (event-agency 4-6)
    # and at the counter's choice (7: 834 pixels between two original runs
    # on 2026-10-24)
    "event-agency-4.bmp": [(1200, 640, 1280, 720)],
    "event-agency-5.bmp": [(1200, 640, 1280, 720)],
    "event-agency-6.bmp": [(1200, 640, 1280, 720)],
    "event-agency-7.bmp": [(1200, 640, 1280, 720)],
    # and on Besaid beach (903 pixels, its other running frame, in a Rust
    # run; the two recordings agreed) and in the agency (49 pixels between
    # the recordings) of settings-shadow (2026-10-26)
    "settings-shadow-3.bmp": [(1200, 640, 1280, 720)],
    "settings-shadow-4.bmp": [(1200, 640, 1280, 720)],
    # the developer-save replays' main menu (shot 3): the play-time clock, as
    # in menu-browse (x 136-180 here; "62:45" is wider)
    "devsave-12-liki-3.bmp": [(125, 575, 200, 605)],
    "devsave-51-thunder-3.bmp": [(125, 575, 200, 605)],
    "devsave-73-bevelle-3.bmp": [(125, 575, 200, 605)],
    "devsave-92-gagazet-3.bmp": [(125, 575, 200, 605)],
    "devsave-109-sin-3.bmp": [(125, 575, 200, 605)],
}
BLOCK = 20
SKY = 240  # coarse shots: rows above this are left out (the beams reach the y 160 blocks; characters stand lower)
# coarse shots (`compare_coarse`), measured on 95 field-walk runs (2026-10-17 to
# 2026-10-20) against synthetic breakage made from them
SWAY_X, SWAY_Y = 4, 16  # the whole-scene shift searched; at its edge the camera is wrong
LOCAL_X, LOCAL_Y = 3, 6  # per block, around that shift
# the scene's left and right halves are lined up on their own (QUAD pixels
# around the whole-scene shift); the deck's sway turns them by at most 3
# pixels against each other (roll: right half down minus left half down)
# and spreads them by at most 2.5 (zoom: right half across minus left);
# a camera rotated by 1 degree gives 7 to 14, zoomed by 2 % 7 to 12
QUAD = 8
CAMERA_LIMIT = 4.5
BLOCK_LIMIT, EDGE = 13.0, 1.4  # a block's limit: 13 + 1.4 x its mean gradient (levels per pixel)
UI = {  # (x0, y0, x1, y1) compared unshifted: the minimap
    "field-walk-1.bmp": [(100, 60, 320, 220)],
    "field-walk-2.bmp": [(100, 60, 320, 220)],
}
UI_LIMIT = 6.0  # runs: up to 5.4; the picture shifted by 2 pixels: 6.9, by 4: 14


def compare_coarse(name, w, h, a, b, r):
    """Field shots: returns (failures, report lines) for the Rust run r
    against the original a, with the second original run b for the noise
    (module docstring)."""
    import numpy as np

    def img(px):  # rows top-down, channels as stored (B, G, R)
        return np.frombuffer(px, np.uint8).reshape(h, w, 4)[::-1, :, :3].astype(np.float64)

    def grad(lum):
        g = np.zeros_like(lum)
        g[:, 1:] += np.abs(np.diff(lum, axis=1))
        g[1:] += np.abs(np.diff(lum, axis=0))
        return g

    def blocks(x):  # BLOCK x BLOCK means of an (h, w, ...) array
        return x[:h // BLOCK * BLOCK, :w // BLOCK * BLOCK].reshape(
            h // BLOCK, BLOCK, w // BLOCK, BLOCK, *x.shape[2:]).mean((1, 3))

    def box_means(x):  # mean of x[y:y + BLOCK, x:x + BLOCK] at every (y, x)
        c = np.zeros((h + 1, w + 1, 3))
        c[1:, 1:] = x.cumsum(0).cumsum(1)
        return (c[BLOCK:, BLOCK:] - c[:-BLOCK, BLOCK:] - c[BLOCK:, :-BLOCK] + c[:-BLOCK, :-BLOCK]) / BLOCK ** 2

    hb, wb = h // BLOCK, w // BLOCK
    keep = np.zeros((hb, wb), bool)
    keep[SKY // BLOCK:, :] = True
    for x0, y0, x1, y1 in MASKS.get(name, ()):
        keep[y0 // BLOCK:-(-y1 // BLOCK), x0 // BLOCK:-(-x1 // BLOCK)] = False

    def best_shift(go, gq, rows, cols, us, vs):
        """(u, v) with gq[y + v, x + u] closest to go[y, x] over rows x cols."""
        return min((np.abs(go[rows, cols] - gq[rows.start + v:rows.stop + v, cols.start + u:cols.stop + u]).mean(), u, v)
                   for v in vs for u in us)[1:]

    def scene(o, q):
        """Per-block difference of q from o after the alignment, and the shift."""
        go, gq = grad(o.mean(2)), grad(q.mean(2))
        rows, cols = slice(SKY + SWAY_Y, h - SWAY_Y), slice(SWAY_X, w - SWAY_X)
        dx, dy = best_shift(go, gq, rows, cols, range(-SWAY_X, SWAY_X + 1), range(-SWAY_Y, SWAY_Y + 1))
        mo, mq = blocks(o), box_means(q)
        by, bx = np.mgrid[0:hb, 0:wb]
        cand = np.stack([mq[np.clip(by * BLOCK + dy + v, 0, h - BLOCK), np.clip(bx * BLOCK + dx + u, 0, w - BLOCK)]
                         for v in range(-LOCAL_Y, LOCAL_Y + 1) for u in range(-LOCAL_X, LOCAL_X + 1)])
        k = keep.copy()
        k[hb - (abs(dy) + LOCAL_Y) // BLOCK - 1:, :] = False  # rows the shift pushes off the picture
        shift = np.median((mo - cand[len(cand) // 2])[k], 0)
        d = np.abs(mo[None] - cand - shift).max(-1).min(0)
        d[~k] = 0
        return d, dx, dy

    def camera(o, q, dx, dy):
        """Roll and zoom of q against o (QUAD): the shifts of the scene's
        upper and lower, left and right quarters."""
        go, gq = grad(o.mean(2)), grad(q.mean(2))
        m = SWAY_Y + QUAD
        mid_y, mid_x = (SKY + h) // 2, w // 2
        quarter = {}
        for ys in (slice(SKY + m, mid_y), slice(mid_y, h - m)):
            for side, xs in (("left", slice(m, mid_x)), ("right", slice(mid_x, w - m))):
                quarter.setdefault(side, []).append(best_shift(
                    go, gq, ys, xs, range(dx - QUAD, dx + QUAD + 1), range(dy - QUAD, dy + QUAD + 1)))
        mean = {side: np.mean(v, 0) for side, v in quarter.items()}
        return mean["right"][1] - mean["left"][1], mean["right"][0] - mean["left"][0]

    a, b, r = img(a), img(b), img(r)
    d, dx, dy = scene(a, r)
    roll, zoom = camera(a, r, dx, dy)
    noise, _, _ = scene(a, b)
    limit = np.maximum(BLOCK_LIMIT + EDGE * blocks(grad(a.mean(2))), 2 * noise)
    by, bx = np.unravel_index((d / limit).argmax(), d.shape)
    bad = 0
    lines = [f"{name}: largest block difference {d[by, bx]:.1f} at x {bx * BLOCK}, y {by * BLOCK} "
             f"(limit {limit[by, bx]:.1f}; scene shifted by {dx}, {dy})"]
    if d[by, bx] > limit[by, bx]:
        bad += 1
    if abs(dx) == SWAY_X or abs(dy) == SWAY_Y:
        lines.append(f"  the scene is shifted by {dx}, {dy}, beyond the deck's sway: wrong camera?")
        bad += 1
    lines.append(f"  camera: roll {roll:.1f}, zoom {zoom:.1f} (limit {CAMERA_LIMIT:.1f})")
    if max(abs(roll), abs(zoom)) > CAMERA_LIMIT:
        bad += 1
    for x0, y0, x1, y1 in UI.get(name, ()):
        ma = a[y0:y1, x0:x1].reshape((y1 - y0) // BLOCK, BLOCK, (x1 - x0) // BLOCK, BLOCK, 3).mean((1, 3))
        mr = r[y0:y1, x0:x1].reshape((y1 - y0) // BLOCK, BLOCK, (x1 - x0) // BLOCK, BLOCK, 3).mean((1, 3))
        ui = np.abs(ma - mr).max()
        lines.append(f"  UI x {x0}..{x1}, y {y0}..{y1}: largest block difference {ui:.1f} (limit {UI_LIMIT:.1f})")
        bad += ui > UI_LIMIT
    return bad, lines


def pixels(path):
    d = path.read_bytes()
    off = struct.unpack_from("<I", d, 10)[0]
    w, h = struct.unpack_from("<ii", d, 18)
    return w, abs(h), d[off:off + w * abs(h) * 4]


a_dir, b_dir, r_dir = (pathlib.Path(p) for p in sys.argv[1:4])
coarse = tuple(sys.argv[sys.argv.index("--coarse") + 1:]) if "--coarse" in sys.argv else ()
only = set(sys.argv[sys.argv.index("--only") + 1].split(",")) if "--only" in sys.argv else None


def wanted(name):
    """Whether NAME (`<replay>-<n>.bmp`) belongs to a replay `--only` names."""
    return only is None or name.rsplit("-", 1)[0] in only


bad = 0
for a_path in sorted(a_dir.glob("*.bmp")):
    if not wanted(a_path.name):
        continue
    if not (r_dir / a_path.name).exists():
        # the run stopped before this shot (a replay whose `waitfor` timed
        # out): report it and go on with the other replays' shots
        print(f"{a_path.name}: missing in {r_dir}")
        bad += 1
        continue
    w, h, a = pixels(a_path)
    _, _, b = pixels(b_dir / a_path.name)
    _, _, r = pixels(r_dir / a_path.name)
    if coarse and a_path.name.startswith(coarse):
        n, lines = compare_coarse(a_path.name, w, h, a, b, r)
        print("\n".join(lines))
        bad += n
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
    columns = set()
    box = [w, h, -1, -1]
    for i in range(0, w * h * 4, 4):
        if not noise[i // 4] and max(abs(a[i + c] - r[i + c]) for c in range(3)) > TOLERANCE:
            mx, my = (i // 4) % w, h - 1 - (i // 4) // w
            if any(x0 <= mx < x1 and y0 <= my < y1 for x0, y0, x1, y1 in MASKS.get(a_path.name, ())):
                continue
            stable_diff += 1
            columns.add((i // 4) % w)
            x, y = (i // 4) % w, h - 1 - (i // 4) // w
            box = [min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y)]
    where = f" in x {box[0]}..{box[2]}, y {box[1]}..{box[3]}" if stable_diff else ""
    print(f"{a_path.name}: {stable_diff} pixels differ from the original{where} ({noisy} differ between the original runs)")
    if noisy > w * h // 2:
        # the battle scene's water shimmers everywhere, so nearly every pixel
        # is noise and this comparison sees nothing (a broken skeleton passed
        # it on 2026-10-17): look at the screenshot instead
        print(f"  BLIND: the original runs differ in {100 * noisy // (w * h)}% of the picture; check {r_dir / a_path.name} by eye")
    # only thin streaks (every differing column within STREAK_WIDTH of
    # another, at most STREAK_WIDTH wide in total per cluster) are noise
    clusters, cur = [], []
    for x in sorted(columns):
        if cur and x - cur[-1] > 1:
            clusters.append(cur)
            cur = []
        cur.append(x)
    if cur:
        clusters.append(cur)
    streaks_only = stable_diff and all(len(c) <= STREAK_WIDTH for c in clusters) and stable_diff <= 60
    if streaks_only:
        print("  (thin cursor-glow streaks only: passes)")
    bad += stable_diff > 0 and not streaks_only
sys.exit(1 if bad else 0)
