#!/usr/bin/env python3
"""Resolves a merge conflict in symbols/ffx.sym (two branches naming things
in the same address range), during `git merge`.

  tools/sym-resolve.py

A three-way merge per symbol, from git's stages of the file (1 the common
ancestor, 2 ours, 3 theirs), keyed by address and kind: a symbol only one
side changed (added, renamed, re-commented, removed) takes that side's
line; one both changed alike takes it; one both changed differently keeps
ours and is reported. The result is written in address order. Also reports
a name given to two addresses. Review what it prints before committing.
"""
import re
import subprocess
import sys

PATH = "symbols/ffx.sym"
ADDR = re.compile(r"0x[0-9a-fA-F]+\s")


def stage(n):
    out = subprocess.run(["git", "show", f":{n}:{PATH}"], capture_output=True, text=True, check=False)
    if out.returncode:
        sys.exit(f"no stage {n} of {PATH}: run this during a merge with a conflict in it")
    return out.stdout.splitlines(keepends=True)


def symbols(lines):
    """Header lines, and {(address, kind): line}; a key named twice keeps both lines in order."""
    head, syms = [], {}
    for line in lines:
        if ADDR.match(line):
            p = line.split()
            syms.setdefault((int(p[0], 16), p[1]), []).append(line)
        elif not syms:
            head.append(line)
    return head, syms


base_head, base = symbols(stage(1))
ours_head, ours = symbols(stage(2))
_, theirs = symbols(stage(3))

merged = {}
for key in sorted(set(base) | set(ours) | set(theirs)):
    b, o, t = base.get(key), ours.get(key), theirs.get(key)
    if o == t or t == b:
        pick = o
    elif o == b:
        pick = t
    else:
        pick = o
        print(f"both changed {key[0]:#010x} {key[1]}; kept ours:")
        print("  ours:  ", "".join(o or ["(removed)\n"]).rstrip()[:150])
        print("  theirs:", "".join(t or ["(removed)\n"]).rstrip()[:150])
    if pick:
        merged[key] = pick

out = list(ours_head)
for key in sorted(merged):
    out += merged[key]

seen = {}
for line in out:
    p = line.split("#", 1)[0].split()
    if len(p) == 3 and ADDR.match(line) and p[1] != "fh":
        if p[2] in seen and seen[p[2]] != p[0]:
            print(f"{p[2]} names both {seen[p[2]]} and {p[0]}")
        seen.setdefault(p[2], p[0])

with open(PATH, "w", encoding="utf-8") as f:
    f.writelines(out)
