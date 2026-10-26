#!/usr/bin/env python3
"""Platform functions a coverage run reached that nobody has ported or listed yet.

  tools/coverage-new.py DIR TAG... [--lists 'GLOB'] [--out FILE]

DIR holds a COVERAGE_ALL run's per-run hits (`tools/coverage-run.sh` writes
`<tag>.hits` for each replay and session); TAG... are the runs in the
order they ran (first reach is the first TAG whose hits hold the
function). Keeps the functions of the platform modules (docs/rewrite-plan.md
M4): the `phyre` module (tools/progress.py's placement), and the functions
symbols/classes.tsv calls `glue` or `platform` in any module. Leaves out
what a `ported!` line hooks and what the M4 lists hold (--lists, default
~/ffx-work/m4/port/*.list, one address per line). Writes them grouped by
module (phyre by subsystem, as tools/coverage-phyre.py), biggest group
first, each line `0x<address> <size> <name> <first run>`; `#` lines head
the groups and sum them up. With --pilot HITS, the functions that run
reached too (the M4 pilot's, from which the lists were cut: what is
missing from the lists among them was left out on purpose, the movie
libraries and the `platform` class for M5) get ` pilot` at the end of
their line, and the group headers count the others as new.
"""
import argparse
import collections
import glob
import importlib.util
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import progress  # noqa: E402

spec = importlib.util.spec_from_file_location("coverage_phyre", HERE / "coverage-phyre.py")
cphyre = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cphyre)


def read_hits(path):
    out = set()
    for w in pathlib.Path(path).read_text().split():
        try:
            out.add(int(w, 16))
        except ValueError:
            pass
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dir")
    ap.add_argument("tags", nargs="+")
    ap.add_argument("--lists", default=str(pathlib.Path.home() / "ffx-work/m4/port/*.list"))
    ap.add_argument("--out")
    ap.add_argument("--pilot")
    args = ap.parse_args()
    first = {}
    for tag in args.tags:
        p = pathlib.Path(args.dir) / f"{tag}.hits"
        if not p.exists():
            print(f"missing {p}", file=sys.stderr)
            continue
        for a in read_hits(p):
            first.setdefault(a, tag)
    rows = progress.functions()
    names = progress.symbols()
    ghidra = {a: n for a, n, _ in rows}
    size = {a: s for a, _, s in rows}
    module = progress.placement(rows, names)
    cls = progress.classes()
    ported = progress.ported()
    listed = set()
    for f in glob.glob(args.lists):
        # not the output itself (coverage2.list sits beside the lists)
        if args.out and pathlib.Path(f).resolve() == pathlib.Path(args.out).resolve():
            continue
        listed |= read_hits(f)

    pilot = read_hits(args.pilot) if args.pilot else set()
    groups = collections.defaultdict(list)
    for a, tag in first.items():
        if a not in size or a in ported or a in listed:
            continue
        m = module.get(a)
        c = cls.get(a)
        name = names.get(a) or ghidra.get(a, "")
        if m == "phyre":
            g = cphyre.group_of(name) if not name.startswith("FUN_") else None
            group = f"phyre/{g or 'unnamed'}"
        elif c in ("glue", "platform"):
            group = f"{c}/{m}"
        else:
            continue
        groups[group].append((a, size[a], name, tag))

    out = []
    total = sum(len(v) for v in groups.values())
    total_b = sum(s for v in groups.values() for _, s, _, _ in v)
    out.append(f"# M4 coverage 2: {total} reached platform functions not ported and not on"
               f" an M4 list ({total_b} bytes), by module")
    out.append(f"# runs, in order: {' '.join(args.tags)}")
    by_run = collections.Counter(t for v in groups.values() for *_, t in v)
    out.append("# first reached by: " + ", ".join(f"{t} {by_run[t]}" for t in args.tags if by_run[t]))
    new = [a for v in groups.values() for a, *_ in v if a not in pilot]
    if args.pilot:
        out.append(f"# not reached by the pilot's run: {len(new)}")
    for g, items in sorted(groups.items(), key=lambda kv: -sum(s for _, s, _, _ in kv[1])):
        fresh = [x for x in items if x[0] not in pilot]
        out.append("")
        out.append(f"# {g}: {len(items)} functions, {sum(s for _, s, _, _ in items)} bytes"
                   + (f"; new: {len(fresh)}, {sum(x[1] for x in fresh)} bytes" if args.pilot else ""))
        for a, s, n, t in sorted(items):
            out.append(f"0x{a:08x} {s:6d} {n} {t}" + (" pilot" if a in pilot else ""))
    text = "\n".join(out) + "\n"
    if args.out:
        pathlib.Path(args.out).write_text(text)
    print("\n".join(line for line in out if line.startswith("#")))


if __name__ == "__main__":
    main()
