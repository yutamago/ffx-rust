#!/usr/bin/env python3
"""Compares two FMOD captures (FFX_AUDIO_WAV, docs/harness.md "Audio capture").

    tools/wav-compare.py A.wav B.wav [--from start] [--to end] [--tolerance T]

Each WAV has a marks file beside it (`<wav>.marks`, written by the hook:
`mark <name> frame <n> sample <n> ...`). The samples from mark `--from` to
mark `--to` of A are compared with the same marks' samples of B. Without
the marks file (or with `--from 0`) the comparison starts at sample 0 and
runs to the shorter file's end.

Sample-exact by default (the capture holds FMOD's 32-bit float mix: every
bit counts); `--tolerance T` allows |a - b| <= T (in full-scale units for
float, in steps for integer PCM). Prints the length, the level (peak and
RMS: a silent capture compares equal trivially) and either "identical" or
the first differing sample (its offset from the start mark, its time, the
FMOD tick it fell in, the channel and both values), how many sample frames
differ and the largest difference. Exit 0 when they match, 1 when they
differ, 2 on bad input. The header's sizes are not trusted: the harness's
`quit` ends the game before FMOD closes the file, and its last buffer
(about 4 KB, ~500 frames) is lost: frames past a file's end are left out
and noted.
"""

import argparse
import struct
import sys
from pathlib import Path

try:
    import numpy as np
except ImportError:  # pragma: no cover - the slow path
    np = None


class Wav:
    def __init__(self, path):
        self.path = Path(path)
        data = self.path.read_bytes()
        if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
            raise ValueError(f"{path}: not a RIFF WAVE file")
        at = 12
        self.fmt = None
        body = None
        while at + 8 <= len(data):
            cid, size = data[at:at + 4], struct.unpack_from("<I", data, at + 4)[0]
            start = at + 8
            if cid == b"fmt ":
                tag, channels, rate, _, align, bits = struct.unpack_from("<HHIIHH", data, start)
                if tag == 0xFFFE and size >= 40:
                    tag = struct.unpack_from("<H", data, start + 24)[0]
                self.fmt = (tag, channels, rate, align, bits)
            elif cid == b"data":
                end = len(data) if size == 0 or start + size > len(data) else start + size
                body = data[start:end]
                break
            at = start + size + (size & 1)
        if self.fmt is None or body is None:
            raise ValueError(f"{path}: no fmt or data chunk")
        tag, self.channels, self.rate, align, self.bits = self.fmt
        self.float = tag == 3
        if not self.float and tag != 1:
            raise ValueError(f"{path}: format tag {tag} (want PCM or IEEE float)")
        self.align = align or self.channels * self.bits // 8
        self.frames = len(body) // self.align
        self.body = body[: self.frames * self.align]
        self.marks = {}
        self.notes = []
        self.block = None
        marks = Path(str(self.path) + ".marks")
        if marks.exists():
            for line in marks.read_text(errors="replace").splitlines():
                w = line.split()
                if not w or w[0].startswith("#"):
                    continue
                kv = dict(zip(w[2::2], w[3::2])) if w[0] == "mark" else dict(zip(w[1::2], w[2::2]))
                if w[0] == "mark" and len(w) > 1:
                    self.marks[w[1]] = {k: int(v) if v.lstrip("-").isdigit() else v for k, v in kv.items()}
                elif w[0] == "format":
                    self.block = int(kv.get("block", 0)) or None
                elif w[0] in ("free", "late"):
                    self.notes.append(line)

    def samples(self, start, end):
        """Interleaved samples of frames [start, end) as a list or array."""
        raw = self.body[start * self.align:end * self.align]
        code = {32: "f" if self.float else "i", 16: "h", 8: "B"}.get(self.bits)
        if code is None:
            raise ValueError(f"{self.path}: {self.bits}-bit samples")
        if np is not None:
            dtype = {"f": "<f4", "i": "<i4", "h": "<i2", "B": "u1"}[code]
            return np.frombuffer(raw, dtype=dtype)
        return list(struct.unpack(f"<{len(raw) * 8 // self.bits}{code}", raw))

    def span(self, first, last):
        def at(name, default):
            if name in (None, ""):
                return default
            if name.isdigit():
                return int(name)
            if name not in self.marks:
                raise ValueError(f"{self.path}: no mark {name!r} (marks: {', '.join(self.marks) or 'none'})")
            return self.marks[name]["sample"]
        return at(first, 0), at(last, self.frames)


def level(x):
    if np is not None:
        if len(x) == 0:
            return 0.0, 0.0
        f = x.astype(np.float64)
        return float(np.max(np.abs(f))), float(np.sqrt(np.mean(f * f)))
    if not x:
        return 0.0, 0.0
    return max(abs(v) for v in x), (sum(v * v for v in x) / len(x)) ** 0.5


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("a")
    ap.add_argument("b")
    ap.add_argument("--from", dest="first", default="start", help="mark (or sample) to start at (default start)")
    ap.add_argument("--to", dest="last", default="end", help="mark (or sample) to stop at (default end)")
    ap.add_argument("--tolerance", type=float, default=0.0)
    ap.add_argument("--quiet", action="store_true", help="one line")
    args = ap.parse_args()
    try:
        a, b = Wav(args.a), Wav(args.b)
        if (a.fmt[0], a.channels, a.rate, a.bits) != (b.fmt[0], b.channels, b.rate, b.bits):
            print(f"different formats: {a.fmt} against {b.fmt}")
            return 1
        # without marks files, compare whole files
        first_a = args.first if a.marks else None
        first_b = args.first if b.marks else None
        last_a = args.last if a.marks else None
        last_b = args.last if b.marks else None
        sa, ea = a.span(first_a, last_a)
        sb, eb = b.span(first_b, last_b)
    except ValueError as e:
        print(e)
        return 2
    # FMOD's writer buffers its file (about 4 KB) and `quit` ends the game
    # before it is flushed: a mark past a file's end is cut to it, and only
    # the frames both files hold are compared (noted, not a difference)
    want_a, want_b = ea - sa, eb - sb
    cut = max(0, ea - a.frames, eb - b.frames)
    ea, eb = min(ea, a.frames), min(eb, b.frames)
    n = min(ea - sa, eb - sb)
    rate, ch = a.rate, a.channels
    block = a.block or rate // 30
    head = (f"{n} frames ({n / rate:.3f} s, {ch} ch, {rate} Hz, {'float' if a.float else f'{a.bits}-bit'}),"
            f" A from {sa}, B from {sb}")
    out = [head]
    for w in (a, b):
        for note in w.notes:
            out.append(f"warning: {w.path.name}: {note} (the output may not repeat from there)")
    ok = True
    if want_a != want_b:
        out.append(f"lengths differ: A {want_a} frames, B {want_b} (compared: the first {n})")
        ok = False
    elif cut:
        out.append(f"note: the last {want_a - n} frames before the end mark were never written"
                   " (FMOD's file buffer at quit); not compared")
    xa, xb = a.samples(sa, sa + n), b.samples(sb, sb + n)
    peak, rms = level(xa)
    out.append(f"level A: peak {peak:.6g}, rms {rms:.6g}{' (silent)' if peak == 0 else ''}")
    if np is not None:
        if args.tolerance > 0:
            diff = np.abs(xa.astype(np.float64) - xb.astype(np.float64)) > args.tolerance
        else:
            diff = xa.view(np.uint8).reshape(-1, xa.itemsize) != xb.view(np.uint8).reshape(-1, xb.itemsize)
            diff = diff.any(axis=1)
        bad = np.flatnonzero(diff)
        count_frames = len(np.unique(bad // ch)) if len(bad) else 0
        largest = float(np.max(np.abs(xa.astype(np.float64) - xb.astype(np.float64)))) if len(bad) else 0.0
        first = int(bad[0]) if len(bad) else None
    else:
        bad = [i for i, (u, v) in enumerate(zip(xa, xb))
               if (abs(u - v) > args.tolerance if args.tolerance > 0 else struct.pack("<f", u) != struct.pack("<f", v))]
        count_frames = len({i // ch for i in bad})
        largest = max((abs(xa[i] - xb[i]) for i in bad), default=0.0)
        first = bad[0] if bad else None
    if first is None:
        out.append("identical" + (f" (tolerance {args.tolerance})" if args.tolerance else ""))
    else:
        ok = False
        f = first // ch
        out.append(
            f"first difference at frame {f} ({f / rate:.4f} s after the start mark, FMOD tick {f // block}),"
            f" channel {first % ch}: A {float(xa[first])!r}, B {float(xb[first])!r};"
            f" {count_frames} of {n} frames differ, largest difference {largest:.6g}")
    if args.quiet:
        print(out[-1] if ok else "; ".join(out[1:]))
    else:
        print("\n".join(out))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
