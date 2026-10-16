#!/usr/bin/env python3
"""Resolves merge conflicts in symbols/ffx.sym (two branches naming things
in the same address range).

  tools/sym-resolve.py [path]

Each conflict keeps the lines of both sides, sorted by address. Then, for
every address and kind named twice in the file (one side renamed or
re-commented a line the other kept), the longer line stays: the one with
more evidence. Prints what it dropped, names given to two addresses, and
lines out of order; review those before committing.
"""
import re
import sys

path = sys.argv[1] if len(sys.argv) > 1 else "symbols/ffx.sym"
ADDR = re.compile(r"0x[0-9a-fA-F]+")


def key(line):
    return int(line.split()[0], 16) if ADDR.match(line) else -1


out, side, a, b = [], None, [], []
for line in open(path, encoding="utf-8"):
    if line.startswith("<<<<<<<"):
        side, a, b = "a", [], []
    elif line.startswith("=======") and side == "a":
        side = "b"
    elif line.startswith(">>>>>>>") and side == "b":
        chunk = list(dict.fromkeys(a + b))
        chunk.sort(key=key)
        out += chunk
        side = None
    else:
        (a if side == "a" else b if side == "b" else out).append(line)

best = {}
for i, line in enumerate(out):
    if ADDR.match(line):
        k = tuple(line.split()[:2])
        if k not in best or len(line) > len(out[best[k]]):
            best[k] = i
keep = set(best.values())
for i, line in enumerate(out):
    if ADDR.match(line) and i not in keep:
        print("dropped:", line.rstrip()[:120])
out = [line for i, line in enumerate(out) if not ADDR.match(line) or i in keep]

prev = -1
for n, line in enumerate(out, 1):
    if ADDR.match(line):
        if key(line) < prev:
            print(f"out of order at line {n}: {line.rstrip()[:80]}")
        prev = key(line)

# one name at two addresses: two branches named different things alike
seen = {}
for line in out:
    p = line.split("#", 1)[0].split()
    if len(p) == 3 and p[1] != "fh":
        if p[2] in seen and seen[p[2]] != p[0]:
            print(f"{p[2]} names both {seen[p[2]]} and {p[0]}")
        seen.setdefault(p[2], p[0])

with open(path, "w", encoding="utf-8") as f:
    f.writelines(out)
