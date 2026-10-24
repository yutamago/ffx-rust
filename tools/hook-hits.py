#!/usr/bin/env python3
"""Which hooked ports the game runs (hook-hit mode, crates/ffx-hook/src/hook_hits.rs).

  FFX_HOOK_HITS=OUT <launch the game>      # tools/run-game.sh passes it on
  tools/hook-hits.py merge OUT...          # adds the hits to symbols/hook-hits.txt
  tools/hook-hits.py report [OUT...] [--out FILE] [--only]
  tools/replays/run-all.sh --hits          # every replay + the coverage session

A hits file has one `0x<address> <name>` line per hook that ran (first call
only). symbols/hook-hits.txt keeps the addresses that ran in any run so far
(the union, one per line, sorted; addresses only, no game data).

`report` counts the ported hooks (`ported!` lines in
crates/ffx-hook/src/ported/*.rs) that ran in the union plus the given
files (--only: in the given files alone), per hook file and per module
(tools/progress.py's), in functions and bytes (body sizes from the Ghidra
dump, $FFX_WORK/decomp/functions.tsv), then the biggest areas that never
ran (name prefix within a hook file) and, per hook file, every port that
never ran, biggest first.
"""
import argparse
import pathlib
import re
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

REPO = progress.REPO
HITS = REPO / "symbols/hook-hits.txt"


def read_hits(path):
    out = set()
    for line in pathlib.Path(path).read_text(encoding="utf-8", errors="replace").splitlines():
        word = line.split("#", 1)[0].split()
        if word:
            try:
                out.add(int(word[0], 16))
            except ValueError:
                pass  # a line cut short by a crash
    return out


def union():
    return read_hits(HITS) if HITS.exists() else set()


def merge(paths):
    seen = union()
    before = len(seen)
    for p in paths:
        seen |= read_hits(p)
    HITS.write_text("".join(f"0x{a:08x}\n" for a in sorted(seen)), encoding="utf-8")
    print(f"{len(seen) - before} new, {len(seen)} in {HITS.relative_to(REPO)}")


def ported_by_file():
    """address -> (hook file stem, name) of every `ported!` entry."""
    out = {}
    for f in sorted((REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
        for name, addr in re.findall(r"^\s*(\w+) @ 0x([0-9a-f_]+) =>", f.read_text(encoding="utf-8"), re.M):
            out[int(addr.replace("_", ""), 16)] = (f.stem, name)
    return out


def area(name):
    """A name's prefix: up to the first `_`, else its first two camel-case words."""
    if "_" in name.strip("_"):
        return name.strip("_").split("_")[0] + "_"
    words = re.findall(r"[A-Z]+(?![a-z])|[A-Z]?[a-z0-9]+", name)
    return "".join(words[:2]) or name


def pct(a, b):
    return f"{100 * a / b:5.1f}%" if b else "    -"


def table(title, groups, out):
    """groups: key -> (hooked fns, ran fns, hooked bytes, ran bytes)."""
    out.append(f"{title:<14} {'ran':>6} / {'hooked':<6} {'fn %':>6}   {'bytes ran':>9} / {'hooked':<9} {'byte %':>6}   never ran (bytes)")
    tot = [0, 0, 0, 0]
    for key in sorted(groups, key=lambda k: -groups[k][2]):
        h, r, hb, rb = groups[key]
        tot = [t + v for t, v in zip(tot, (h, r, hb, rb))]
        out.append(f"{key:<14} {r:>6} / {h:<6} {pct(r, h):>6}   {rb:>9} / {hb:<9} {pct(rb, hb):>6}   {h - r} ({hb - rb})")
    h, r, hb, rb = tot
    out.append(f"{'total':<14} {r:>6} / {h:<6} {pct(r, h):>6}   {rb:>9} / {hb:<9} {pct(rb, hb):>6}   {h - r} ({hb - rb})")
    out.append("")


def report(paths, only, dest, top):
    hits = set() if only else union()
    for p in paths:
        hits |= read_hits(p)
    hooked = ported_by_file()
    rows = progress.functions()
    size = {a: s for a, _, s in rows}
    module = progress.placement(rows, progress.symbols())
    by_file, by_module, areas = defaultdict(lambda: [0, 0, 0, 0]), defaultdict(lambda: [0, 0, 0, 0]), defaultdict(lambda: [0, 0])
    never = defaultdict(list)
    for a, (f, name) in hooked.items():
        s = size.get(a, 0)
        ran = a in hits
        for g in (by_file[f], by_module[module.get(a, "?")]):
            g[0] += 1
            g[2] += s
            if ran:
                g[1] += 1
                g[3] += s
        if not ran:
            never[f].append((s, a, name))
            k = f"{f}: {area(name)}"
            areas[k][0] += 1
            areas[k][1] += s
    out = [f"Hook hits: {len(hits & hooked.keys())} of {len(hooked)} ported hooks ran "
           f"({'given files only' if only else 'union of symbols/hook-hits.txt and the given files'}; "
           f"{len(hits - hooked.keys())} other hits: hand-written hooks or ports since removed)", ""]
    table("hook file", by_file, out)
    table("module", by_module, out)
    out.append(f"The {top} biggest areas that never ran (hook file: name prefix; functions, bytes):")
    for k, (n, b) in sorted(areas.items(), key=lambda kv: -kv[1][1])[:top]:
        out.append(f"  {k:<40} {n:>5} fns {b:>8} bytes")
    out.append("")
    for f in sorted(never, key=lambda f: -sum(s for s, _, _ in never[f])):
        lst = sorted(never[f], reverse=True)
        out.append(f"== {f}.rs: {len(lst)} never ran, {sum(s for s, _, _ in lst)} bytes")
        out.extend(f"  0x{a:08x} {s:>7} {name}" for s, a, name in lst)
        out.append("")
    text = "\n".join(out)
    if dest:
        pathlib.Path(dest).parent.mkdir(parents=True, exist_ok=True)
        pathlib.Path(dest).write_text(text, encoding="utf-8")
        # the summary on stdout, the lists in the file
        print("\n".join(out[:out.index(next(l for l in out if l.startswith("==")))] if never else out))
        print(f"report -> {dest}")
    else:
        print(text)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    m = sub.add_parser("merge")
    m.add_argument("files", nargs="+")
    r = sub.add_parser("report")
    r.add_argument("files", nargs="*")
    r.add_argument("--only", action="store_true", help="the given files only, not the union")
    r.add_argument("--out", help="write the whole report here (stdout gets the summary)")
    r.add_argument("--top", type=int, default=20)
    args = ap.parse_args()
    if args.cmd == "merge":
        merge(args.files)
    else:
        report(args.files, args.only, args.out, args.top)


if __name__ == "__main__":
    main()
