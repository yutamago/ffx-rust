#!/usr/bin/env python3
"""Compares two replays by what happened, not when (docs/rewrite-plan.md,
Determinism): loading takes a varying number of frames, so frame-by-frame
traces drift even when the logic agrees.

- Traces: per range, the sequence of distinct consecutive values.
- Call logs (`FFX_HARNESS_CALL_LOG`): per (tag, value) — for `brnd` the
  generator index — the sequence of first callers. Independent consumers
  (a battle's draws and an event script's) interleave differently between
  runs; per generator their order is fixed.

Usage: tools/replay-compare.py A.trace B.trace [A.calls B.calls] [--also A2.trace] [--ignore N,...] [--sampled N,...] [--null-first N,...]
Exits 1 when anything differs. With `--also`, a second run of the original:
a range passes when B matches either original run (the originals themselves
can differ by an event where a press lands a frame apart). `--ignore`
leaves trace ranges out by index: the pad word (0x00ccb170) records when
the harness's presses land, which varies by a frame between any two runs
of the original, and is no game result. `--sampled` compares ranges by
order only: a range passes when either run's sequence is the other's with
values left out. For ranges that change every frame in a replay at the
game's own frame rate (`realtime`): when a frame falls behind, the game
runs more 30 Hz ticks in it (`Sg_FrameSync`'s hold), and the trace, taken
once a frame, misses the values in between (docs/harness.md, event-cliff);
and for event-agency's player position, which skipped a value in one of six
original runs in lockstep.
`--null-first` leaves out a range's first event when one run read it as
`-` (unreadable) and the other did not: ranges read through a pointer that
is still 0 there (the developer-save replays' monster HP,
`*0x011334d4+0x13d10..`, before the battle) land in the 32-bit process's
low memory, which `FFX.exe`'s process maps (Windows' own blocks at
0x10000) and `ffx-rust.exe`'s arena does not. The rest must agree.
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


def within(short, long):
    """Whether `short` is `long` with values left out (in order)."""
    it = iter(long)
    return all(v in it for v in short)


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
    sampled = set()
    if "--sampled" in argv:
        k = argv.index("--sampled")
        sampled = {int(x) for x in argv[k + 1].split(",")}
        argv = argv[:k] + argv[k + 2:]
    null_first = set()
    if "--null-first" in argv:
        k = argv.index("--null-first")
        null_first = {int(x) for x in argv[k + 1].split(",")}
        argv = argv[:k] + argv[k + 2:]
    sys.argv = sys.argv[:1] + argv
    if len(sys.argv) not in (3, 5):
        sys.exit(__doc__)
    bad = 0
    ea, eb = events(sys.argv[1]), events(sys.argv[2])
    for i, (x, y) in enumerate(zip(ea, eb)):
        if i in ignore:
            continue
        if (
            i in null_first
            and x[:1] != y[:1]
            and "-" in (x[:1] + y[:1])
            and (x[1:] == y[1:] or (also is not None and i < len(also) and also[i][1:] == y[1:]))
        ):
            print(f"range {i}: the same events after the first (read through a null pointer)")
            continue
        if x != y and also is not None and i < len(also) and also[i] == y:
            print(f"range {i}: matches the second original run")
            continue
        if x != y and i in sampled and (within(x, y) or within(y, x)):
            print(f"range {i}: the same values in order, {abs(len(x) - len(y))} left out")
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
