#!/usr/bin/env python3
"""M6 proof of concept (docs/m6.md): the game's own D3D11 draws, replayed
on wgpu with the DXBC shaders translated to WGSL in pure Rust (ffx-dxbc,
ffx-render's replay), against what Direct3D 11 drew.

Input: a full-mode capture (crates/ffx-hook gfxcapture without `hash`):
capture.ffxgfx, frame.txt, objects.txt and rt/NNN_c_<view>.raw, the render
target the original's D3D11 device held each time the game moved off it.
For each such dump the script replays the capture with `ffx-render replay
--until N` (N = the draws before the dump) and compares the target pixel by
pixel.

A dump is compared only when everything that made it is in the capture.
A capture holds no contents for a dynamic buffer the game wrote before the
capture started: the replay draws zeros there. `capture_gaps` lists the
draws that read one; they are "incomplete", and so is a draw sampling a
texture an incomplete draw wrote; a clear makes a target complete again.
Incomplete dumps are listed, not counted.

usage: wgpu-parity.py [--backend vulkan|dx12|gl] [--exe PATH] [--gaps-exe PATH]
                      <capture dir> <work dir>
    (defaults: target/release/ffx-render and the capture_gaps example,
     built by `cargo build --release -p ffx-render --bin ffx-render
     --example capture_gaps`; on Windows hardware through WSL interop:
     --exe target/x86_64-pc-windows-msvc/release/ffx-render.exe, built by
     tools/build-renderer.sh, which runs no game)
"""
import argparse
import os
import re
import subprocess
import sys
import zlib
from pathlib import Path

import numpy as np

REPO = Path(__file__).resolve().parents[2]


def read_png(path):
    """RGBA8 PNGs as ffx-render writes them (filter 0 on every row)."""
    data = Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    i, idat, w, h = 8, b"", 0, 0
    while i < len(data):
        n = int.from_bytes(data[i:i + 4], "big")
        tag, body = data[i + 4:i + 8], data[i + 8:i + 8 + n]
        if tag == b"IHDR":
            w, h = int.from_bytes(body[0:4], "big"), int.from_bytes(body[4:8], "big")
            assert body[8] == 8 and body[9] == 6, "RGBA8 only"
        elif tag == b"IDAT":
            idat += body
        i += 12 + n
    raw = np.frombuffer(zlib.decompress(idat), dtype=np.uint8).reshape(h, 1 + w * 4)
    assert (raw[:, 0] == 0).all(), "filter 0 only"
    return raw[:, 1:].reshape(h, w, 4)


def winpath(p):
    return subprocess.run(["wslpath", "-w", str(p)], check=True, capture_output=True,
                          text=True).stdout.strip()


def ids(field, line):
    """The hex ids after `<field> ` in a draw line (`0:0x..,1:0x..`)."""
    m = re.search(rf"\b{field} ([^ ]*)", line)
    if not m:
        return []
    return [int(x, 16) for x in re.findall(r"0x([0-9a-f]+)", m.group(1))]


def plan(cap, gaps):
    """The dumps with the draws before them, each marked complete or not;
    `gaps`: the draw numbers (from 1) capture_gaps names."""
    view_res = {}
    for line in (cap / "objects.txt").read_text().splitlines():
        m = re.match(r"view 0x([0-9a-f]+) \w+ resource 0x([0-9a-f]+)", line)
        if m:
            view_res[int(m.group(1), 16)] = int(m.group(2), 16)
    tainted = set()  # resources an incomplete draw wrote
    draws = incomplete = 0
    dumps = []
    for line in (cap / "frame.txt").read_text().splitlines():
        if line.startswith("draw "):
            draws += 1
            srv = [view_res.get(v, v) for v in ids("pssrv", line) + ids("vssrv", line)]
            bad = draws in gaps or any(r in tainted for r in srv)
            incomplete += bad
            for v in ids("rt", line) + ids("ds", line):
                r = view_res.get(v, v)
                if bad:
                    tainted.add(r)
        elif line.startswith("clear_rtv ") or line.startswith("clear_dsv "):
            v = int(line.split()[1], 16)
            tainted.discard(view_res.get(v, v))
        m = re.match(r"dump rt/(\d+)_c_([0-9a-f]+) (\d+)x(\d+) format (\d+) pitch (\d+)", line)
        if m:
            view = int(m.group(2), 16)
            dumps.append({
                "until": draws, "name": m.group(1), "view": view,
                "w": int(m.group(3)), "h": int(m.group(4)),
                "format": int(m.group(5)), "pitch": int(m.group(6)),
                "complete": view_res.get(view, view) not in tainted,
            })
    return draws, incomplete, dumps


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("capture", type=Path)
    ap.add_argument("work", type=Path)
    ap.add_argument("--backend", default="vulkan")
    ap.add_argument("--exe", default=str(REPO / "target/release/ffx-render"))
    ap.add_argument("--gaps-exe", default=str(REPO / "target/release/examples/capture_gaps"))
    a = ap.parse_args()
    cap, work = a.capture, a.work
    work.mkdir(parents=True, exist_ok=True)
    stream = cap / "capture.ffxgfx"
    listing = subprocess.run([a.gaps_exe, str(stream)], check=True, capture_output=True,
                         text=True).stdout
    gaps = {int(line.split()[0]) for line in listing.splitlines() if line.strip()}
    draws, incomplete, dumps = plan(cap, gaps)
    wslenv = ":".join(x for x in (os.environ.get("WSLENV", ""), "FFX_RENDER_BACKEND") if x)
    env = dict(os.environ, FFX_RENDER_BACKEND=a.backend, WSLENV=wslenv)
    adapter = "?"
    print(f"{cap}: {draws} draws ({incomplete} incomplete: {len(gaps)} read dynamic data "
          f"the capture lacks), {len(dumps)} colour dumps")
    tot_px = tot_diff = tot_big = 0
    for d in dumps:
        tag = f"{d['name']} view {d['view']:x} {d['w']}x{d['h']} after draw {d['until']}"
        if not d["complete"]:
            print(f"{tag}: not compared (made from incomplete draws)")
            continue
        if d["format"] not in (28, 29, 87, 91):
            print(f"{tag}: format {d['format']} not compared")
            continue
        out = work / f"u{d['until']:04d}"
        png = out / f"{d['view']:x}.png"
        if not png.exists():
            # A Windows build (ffx-render.exe from tools/build-renderer.sh)
            # runs through WSL interop with Windows paths.
            win = a.exe.endswith(".exe")
            paths = [winpath(p) if win else str(p) for p in (stream, out)]
            r = subprocess.run([a.exe, "replay", *paths, "--until", str(d["until"])],
                               check=False, capture_output=True, text=True, env=env)
            if r.returncode != 0:
                last = (r.stdout + r.stderr).strip().splitlines()[-1:] or ["?"]
                print(f"{tag}: the replay failed: {last[0]}")
                continue
            adapter = r.stdout.splitlines()[0] if r.stdout else adapter
        if not png.exists():
            print(f"{tag}: wgpu wrote no image")
            continue
        got = read_png(png).astype(np.int16)
        raw = np.frombuffer((cap / "rt" / f"{d['name']}_c_{d['view']:x}.raw").read_bytes(), np.uint8)
        ref = raw.reshape(d["h"], d["pitch"])[:, :d["w"] * 4].reshape(d["h"], d["w"], 4)
        if d["format"] in (87, 91):
            ref = ref[:, :, [2, 1, 0, 3]]
        diff = np.abs(got - ref.astype(np.int16)).max(axis=2)
        n, big = int((diff > 0).sum()), int((diff > 1).sum())
        tot_px += d["w"] * d["h"]
        tot_diff += n
        tot_big += big
        print(f"{tag}: {n} px differ ({100.0 * n / (d['w'] * d['h']):.3f} %), {big} by more "
              f"than 1 ({100.0 * big / (d['w'] * d['h']):.4f} %), largest {int(diff.max())}")
    print(f"adapter: {adapter}")
    if tot_px:
        print(f"compared: {tot_diff} of {tot_px} px differ ({100.0 * tot_diff / tot_px:.3f} %), "
              f"{tot_big} by more than 1 ({100.0 * tot_big / tot_px:.4f} %)")
    return 0


sys.exit(main())
