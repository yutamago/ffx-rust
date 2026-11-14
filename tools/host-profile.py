#!/usr/bin/env python3
"""Names the CPU samples of ffx-host's sampler (FFX_HOST_SAMPLE, src/sample.rs).

Usage: tools/host-profile.py <prefix>.pc.tsv [--top N] [--raw] [--callers TEXT]

Each row of the .pc.tsv file is an object, an offset into it and a sample
count. The offsets of every object are named with `nm -C -n` (a function's
samples are summed; --raw keeps the addresses), and the functions are
printed with their share of the samples, most first. Generic Rust names are
shortened (the hash suffix goes). --callers TEXT prints, for the functions
whose name contains TEXT, what the word at rsp named when they were sampled:
for a leaf function (libc's memcpy and memset) its caller.
"""

import argparse
import bisect
import collections
import os
import re
import subprocess
import sys


def symbols(path):
    """Sorted (address, name) of the object's functions."""
    try:
        out = subprocess.run(
            ["nm", "-C", "-n", "--defined-only", path],
            capture_output=True, text=True, check=False,
        ).stdout
    except OSError:
        return [], []
    addrs, names = [], []
    for line in out.splitlines():
        parts = line.split(" ", 2)
        if len(parts) < 3 or parts[1] not in "tTwW":
            continue
        try:
            a = int(parts[0], 16)
        except ValueError:
            continue
        addrs.append(a)
        names.append(re.sub(r"::h[0-9a-f]{16}$", "", parts[2]))
    return addrs, names


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pc")
    ap.add_argument("--top", type=int, default=60)
    ap.add_argument("--raw", action="store_true")
    ap.add_argument("--callers")
    a = ap.parse_args()
    rows, pairs = [], []
    with open(a.pc) as f:
        in_pairs = False
        for line in f:
            if line.startswith("# pairs"):
                in_pairs = True
                continue
            if line.startswith("#"):
                continue
            p = line.rstrip("\n").split("\t")
            if in_pairs:
                pairs.append((p[0], int(p[1], 16), p[2], p[3], int(p[4], 16), int(p[5])))
            else:
                rows.append((p[0], int(p[1], 16), p[2], int(p[3])))
    tables = {}

    def name_of(obj, off, sym):
        if obj not in tables:
            tables[obj] = symbols(obj) if os.path.exists(obj) else ([], [])
        addrs, names = tables[obj]
        i = bisect.bisect_right(addrs, off) - 1
        name = names[i] if i >= 0 else (sym or "?")
        if obj.endswith(".so") or ".so." in obj:
            name = f"{os.path.basename(obj)}!{sym or name}"
        return name

    if a.callers:
        by = collections.Counter()
        total = 0
        for obj, off, sym, robj, roff, n in pairs:
            name = name_of(obj, off, sym)
            if a.callers not in name:
                continue
            total += n
            by[(name, name_of(robj, roff, ""))] += n
        print(f"{total} samples in functions matching {a.callers!r}")
        for (name, caller), n in by.most_common(a.top):
            print(f"{100.0 * n / max(total, 1):6.2f}%  {n:7}  {name}  <-  {caller}")
        return 0
    total = sum(r[3] for r in rows)
    by = collections.Counter()
    for obj, off, sym, n in rows:
        name = name_of(obj, off, sym)
        key = f"{name} +{off:#x}" if a.raw else name
        by[key] += n
    print(f"{total} samples")
    for name, n in by.most_common(a.top):
        print(f"{100.0 * n / total:6.2f}%  {n:7}  {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
