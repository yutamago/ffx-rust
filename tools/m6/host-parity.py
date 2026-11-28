#!/usr/bin/env python3
"""Pixel parity between ffx-host's graphics back ends (docs/host.md, "The
wgpu back end"; docs/m6.md).

One back-end trace (`FFX_HOST_GFX_TRACE`, recorded on any back end, the null
device included) is replayed into each back end by `gfx-replay`
(crates/ffx-host/examples/gfx-replay.rs), which saves the back buffer at the
same Presents; this script compares the pictures pixel by pixel against the
reference back end's.

usage:
  host-parity.py <reference dir> <dir>...          # compare saved pictures
  host-parity.py --run <trace> <out dir> [--every N] [--exe gfx-replay.exe]
      # replays <trace> on this machine's Windows GPU into d3d11, wgpu on
      # Vulkan and wgpu on D3D12 (through WSL interop: no window is made),
      # then compares the two wgpu runs with d3d11

Per picture: pixels that differ at all, pixels more than 2 steps off in a
channel, the largest step. A total line per directory.
"""
import argparse
import os
import subprocess
import sys
import zlib
from pathlib import Path

import numpy as np


def read_png(path):
    """8-bit RGB or RGBA PNGs with filter 0 on every row (gfx::trace's)."""
    data = Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    i, idat, w, h, ch = 8, b"", 0, 0, 3
    while i < len(data):
        n = int.from_bytes(data[i:i + 4], "big")
        tag, body = data[i + 4:i + 8], data[i + 8:i + 8 + n]
        if tag == b"IHDR":
            w, h = int.from_bytes(body[0:4], "big"), int.from_bytes(body[4:8], "big")
            assert body[8] == 8 and body[9] in (2, 6), "RGB(A)8 only"
            ch = 3 if body[9] == 2 else 4
        elif tag == b"IDAT":
            idat += body
        i += 12 + n
    raw = np.frombuffer(zlib.decompress(idat), dtype=np.uint8).reshape(h, 1 + w * ch)
    assert (raw[:, 0] == 0).all(), "filter 0 only"
    return raw[:, 1:].reshape(h, w, ch)[:, :, :3]


def compare(ref_dir, dirs):
    ref_dir = Path(ref_dir)
    names = sorted(p.name for p in ref_dir.glob("present-*.png"))
    if not names:
        print(f"{ref_dir}: no pictures")
        return 1
    worst = 0
    for d in map(Path, dirs):
        total = px = big = 0
        top = 0
        print(f"== {d} against {ref_dir}")
        for n in names:
            other = d / n
            if not other.exists():
                print(f"  {n}: missing")
                continue
            a, b = read_png(ref_dir / n), read_png(other)
            if a.shape != b.shape:
                print(f"  {n}: size {b.shape[1]}x{b.shape[0]} vs {a.shape[1]}x{a.shape[0]}")
                continue
            diff = np.abs(a.astype(np.int16) - b.astype(np.int16)).max(axis=2)
            npx = int((diff > 0).sum())
            nbig = int((diff > 2).sum())
            m = int(diff.max())
            total += diff.size
            px += npx
            big += nbig
            top = max(top, m)
            print(f"  {n}: {npx} px differ ({100 * npx / diff.size:.3f} %), {nbig} by more than 2, largest {m}")
        if total:
            print(f"  total: {px} of {total} px differ ({100 * px / total:.4f} %), "
                  f"{big} by more than 2 ({100 * big / total:.4f} %), largest {top}")
        worst = max(worst, big)
    return 0


def winpath(p):
    return subprocess.run(["wslpath", "-w", str(p)], check=True, capture_output=True,
                          text=True).stdout.strip()


def run(trace, out, every, exe):
    out = Path(out)
    runs = [("d3d11", "d3d11", ""), ("wgpu-vulkan", "wgpu", "vulkan"), ("wgpu-dx12", "wgpu", "dx12")]
    for name, backend, api in runs:
        d = out / name
        d.mkdir(parents=True, exist_ok=True)
        env = dict(os.environ, FFX_HOST_WGPU_BACKEND=api, WSLENV="FFX_HOST_WGPU_BACKEND")
        r = subprocess.run([exe, winpath(trace), backend, winpath(d), "--every", str(every)],
                           env=env, capture_output=True, text=True)
        print(f"{name}: {(r.stdout or r.stderr).strip()}")
        problems = d / "problems.txt"
        if problems.exists():
            for line in problems.read_text().splitlines()[:20]:
                print(f"  {line}")
    return compare(out / "d3d11", [out / "wgpu-vulkan", out / "wgpu-dx12"])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", nargs=2, metavar=("TRACE", "OUT"))
    ap.add_argument("--every", type=int, default=100)
    ap.add_argument("--exe", default=str(Path(__file__).resolve().parents[2]
                                         / "target/x86_64-pc-windows-msvc/release/examples/gfx-replay.exe"))
    ap.add_argument("dirs", nargs="*")
    a = ap.parse_args()
    if a.run:
        return run(a.run[0], a.run[1], a.every, a.exe)
    if len(a.dirs) < 2:
        ap.error("a reference directory and at least one to compare")
    return compare(a.dirs[0], a.dirs[1:])


if __name__ == "__main__":
    sys.exit(main())
