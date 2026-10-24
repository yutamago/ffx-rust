#!/usr/bin/env python3
"""Compares two replays by what happened, not when (docs/rewrite-plan.md,
Determinism): loading takes a varying number of frames, so frame-by-frame
traces drift even when the logic agrees.

- Traces: per range, the sequence of distinct consecutive values.
- Call logs (`FFX_HARNESS_CALL_LOG`): per (tag, value) — for `brnd` the
  generator index — the sequence of first callers. Independent consumers
  (a battle's draws and an event script's) interleave differently between
  runs; per generator their order is fixed.

Usage: tools/replay-compare.py A.trace B.trace [A.calls B.calls] [--also A2.trace] [--ignore N,...]
Exits 1 when anything differs. With `--also`, a second run of the original:
a range passes when B matches either original run (the originals themselves
can differ by an event where a press lands a frame apart). `--ignore`
leaves trace ranges out by index: the pad word (0x00ccb170) records when
the harness's presses land, which varies by a frame between any two runs
of the original, and is no game result.
"""
import sys
from collections import defaultdict


def events(path):
    rows = [line.split()[1:] for line in open(path, encoding="utf-8") if line.strip()]
    if not rows:
        return []
    # a run that ended early (a timed-out `waitfor`) can leave a cut last line
    rows = [row for row in rows if len(row) == len(rows[0])]
    out = []
    for r in range(len(rows[0])):
        seq = []
        for row in rows:
            if not seq or seq[-1] != row[r]:
                seq.append(row[r])
        out.append(seq)
    return out


def calls(path):
    per = defaultdict(list)
    for line in open(path, encoding="utf-8"):
        p = line.split()
        if len(p) >= 4:
            per[(p[2], p[3])].append(p[4].split(",")[0] if len(p) > 4 else "")
    return per


def main():
    argv = sys.argv[1:]
    also = None
    if "--also" in argv:
        k = argv.index("--also")
        also = events(argv[k + 1])
        argv = argv[:k] + argv[k + 2:]
    ignore = set()
    if "--ignore" in argv:
        k = argv.index("--ignore")
        ignore = {int(x) for x in argv[k + 1].split(",")}
        argv = argv[:k] + argv[k + 2:]
    sys.argv = sys.argv[:1] + argv
    if len(sys.argv) not in (3, 5):
        sys.exit(__doc__)
    bad = 0
    ea, eb = events(sys.argv[1]), events(sys.argv[2])
    for i, (x, y) in enumerate(zip(ea, eb)):
        if i in ignore:
            continue
        if x != y and also is not None and i < len(also) and also[i] == y:
            print(f"range {i}: matches the second original run")
            continue
        if x != y:
            n = next((k for k, (u, v) in enumerate(zip(x, y)) if u != v), min(len(x), len(y)))
            print(f"range {i}: differs at event {n} ({len(x)} and {len(y)} events)")
            bad += 1
    print(f"traces: {len(ea)} ranges, {'same events' if not bad else f'{bad} differ'}")
    if len(sys.argv) == 5:
        ca, cb = calls(sys.argv[3]), calls(sys.argv[4])
        diff = 0
        for key in sorted(set(ca) | set(cb)):
            if ca.get(key) != cb.get(key):
                print(f"calls {key[0]} {key[1]}: {len(ca.get(key, []))} and {len(cb.get(key, []))} calls")
                diff += 1
        total = sum(len(v) for v in ca.values())
        print(f"calls: {total} in {len(ca)} streams, {'same' if not diff else f'{diff} streams differ'}")
        bad += diff
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
