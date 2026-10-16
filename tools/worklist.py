#!/usr/bin/env python3
"""What to port next (docs/porting.md): game-code functions not ported yet
whose callees are all ported already, or library code (left to the
original for now), or nothing at all. Leaves first, then by callers (the
functions most used), then smaller first.

  tools/worklist.py [--module M] [--limit N] [--named] [--max-bytes B]

Columns: address, module, bytes, callers, name (and calls into imports,
which need a platform equivalent).
"""
import argparse
import os
import pathlib
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--module")
    ap.add_argument("--limit", type=int, default=50)
    ap.add_argument("--named", action="store_true", help="only functions with a name")
    ap.add_argument("--max-bytes", type=int, default=0)
    ap.add_argument("--min-bytes", type=int, default=5)
    args = ap.parse_args()

    names = progress.symbols()
    rows = progress.functions()
    module = progress.placement(rows, names)
    size = {a: s for a, _, s in rows}
    ported = set(progress.ported())
    callees = defaultdict(set)
    callers = defaultdict(set)
    imports = defaultdict(int)
    with open(WORK / "decomp/call_sites.tsv", encoding="utf-8") as f:
        next(f)
        for line in f:
            a, _, c = line.rstrip("\n").split("\t")
            if "EXTERNAL" in c:
                imports[int(a, 16)] += 1
                continue
            a, c = int(a, 16), int(c, 16)
            callees[a].add(c)
            callers[c].add(a)

    def blocking(a):
        return [c for c in callees[a]
                if c != a and c not in ported and progress.KIND.get(module.get(c)) == "game"]

    out = []
    for a, _, s in rows:
        m = module.get(a)
        if progress.KIND.get(m) != "game" or a in ported or s < args.min_bytes:
            continue
        if args.module and m != args.module:
            continue
        if args.named and a not in names:
            continue
        if args.max_bytes and s > args.max_bytes:
            continue
        b = blocking(a)
        if b:
            continue
        out.append((len(callees[a] - {a}) > 0 or imports[a] > 0, -len(callers[a]), s, a, m))
    out.sort()
    for _, neg_callers, s, a, m in out[: args.limit]:
        imp = f" (+{imports[a]} imports)" if imports[a] else ""
        print(f"0x{a:08x} {m:8} {s:6} {-neg_callers:5}  {names.get(a, '')}{imp}")
    print(f"{len(out)} ready", file=sys.stderr)


if __name__ == "__main__":
    main()
