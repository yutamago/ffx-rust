#!/usr/bin/env python3
"""Which functions and modules touch which globals (docs/rewrite-plan.md, M0).

Reads $FFX_WORK/decomp/data_refs.tsv (tools/ghidra/DumpDataRefs.java) and
places functions in modules the way tools/progress.py does. Only absolute
references into .data count: a global reached through a pointer someone else
passed in does not show up here, and constants in .rdata are left out.

  tools/ownership.py summary            per game module: globals it touches,
                                        how many only it touches, and which
                                        modules share the rest
  tools/ownership.py shared [N]         the N globals touched by most modules
  tools/ownership.py global ADDR        every function touching ADDR (and the
                                        16 bytes after it, for struct fields)
  tools/ownership.py function NAME|ADDR the globals a function touches and
                                        who else touches them
  tools/ownership.py module MODULE      the globals MODULE shares, with the
                                        other modules' functions on them
"""
import os
import pathlib
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))
# .data and its uninitialized tail (FFX.exe section table): game state. .rdata
# holds constants (float literals, vftables, strings), which nobody owns.
STATE = (0x00C0A000, 0x025D7000)
SECURITY_COOKIE = 0x00C613D8  # read by the prologue of 5,959 functions


def load():
    names = progress.symbols()
    rows = progress.functions()
    module = progress.placement(rows, names)
    dump_names = {a: n for a, n, _ in rows}
    refs = defaultdict(lambda: defaultdict(set))  # global -> function -> accesses
    path = WORK / "decomp/data_refs.tsv"
    if not path.exists():
        sys.exit(f"{path} missing: run tools/ghidra/dump.sh")
    with open(path, encoding="utf-8") as f:
        next(f)
        for line in f:
            fn, addr, access, _ = line.rstrip("\n").split("\t")
            g = int(addr, 16)
            if STATE[0] <= g < STATE[1] and g != SECURITY_COOKIE:
                refs[g][int(fn, 16)].add(access)
    labels = {}
    for line in (progress.REPO / "symbols/ffx.sym").read_text(encoding="utf-8").splitlines():
        p = line.split("#", 1)[0].split()
        if len(p) == 3 and p[1] == "label":
            labels[int(p[0], 16)] = p[2]
    return names, dump_names, module, refs, labels


def fname(a, names, dump_names):
    return names.get(a) or dump_names.get(a) or f"0x{a:08x}"


def game(m):
    return progress.KIND.get(m) == "game"


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    cmd, rest = sys.argv[1], sys.argv[2:]
    names, dump_names, module, refs, labels = load()

    def modules_of(g):
        return {module.get(f, "?") for f in refs[g]}

    if cmd == "summary":
        touched = defaultdict(set)
        for g in refs:
            for m in modules_of(g):
                touched[m].add(g)
        print(f"{'module':10} {'globals':>7} {'only it':>7} {'shared':>6}  shared with (globals)")
        for m in sorted((m for m in touched if game(m)), key=lambda m: -len(touched[m])):
            only = sum(1 for g in touched[m] if modules_of(g) == {m})
            partners = defaultdict(int)
            for g in touched[m]:
                for o in modules_of(g) - {m}:
                    partners[o] += 1
            top = ", ".join(f"{o} {n}" for o, n in sorted(partners.items(), key=lambda x: -x[1])[:6])
            print(f"{m:10} {len(touched[m]):7} {only:7} {len(touched[m]) - only:6}  {top}")
    elif cmd == "shared":
        n = int(rest[0]) if rest else 40
        ranked = sorted(refs, key=lambda g: (-len({m for m in modules_of(g) if game(m)}), -len(refs[g])))
        for g in ranked[:n]:
            ms = sorted(m for m in modules_of(g) if game(m))
            w = sum(1 for acc in refs[g].values() if "W" in acc)
            print(f"0x{g:08x} {labels.get(g, ''):28} {len(refs[g]):4} functions ({w} write)  {' '.join(ms)}")
    elif cmd == "global":
        base = int(rest[0], 16)
        for g in sorted(a for a in refs if base <= a < base + 16):
            print(f"0x{g:08x} (+{g - base:#x}) {labels.get(g, '')}")
            for f, acc in sorted(refs[g].items()):
                print(f"  {''.join(sorted(acc)):3} {module.get(f, '?'):8} 0x{f:08x} {fname(f, names, dump_names)}")
    elif cmd == "function":
        key = rest[0]
        if key.startswith("0x"):
            fa = int(key, 16)
        else:
            by_name = {n: a for a, n in names.items()} | {n: a for a, n in dump_names.items()}
            if key not in by_name:
                sys.exit(f"no function named {key}")
            fa = by_name[key]
        print(f"{fname(fa, names, dump_names)} (0x{fa:08x}, {module.get(fa)})")
        for g in sorted(a for a in refs if fa in refs[a]):
            others = [f for f in refs[g] if f != fa]
            ms = sorted({module.get(f, "?") for f in others})
            print(f"  {''.join(sorted(refs[g][fa])):3} 0x{g:08x} {labels.get(g, ''):24} "
                  f"{len(others):4} others  {' '.join(ms)}")
    elif cmd == "module":
        m = rest[0]
        for g in sorted(refs):
            ms = modules_of(g)
            if m in ms and len(ms) > 1:
                others = [(f, acc) for f, acc in refs[g].items() if module.get(f) != m]
                print(f"0x{g:08x} {labels.get(g, '')}  ({len(refs[g]) - len(others)} in {m})")
                for f, acc in sorted(others)[:8]:
                    print(f"  {''.join(sorted(acc)):3} {module.get(f, '?'):8} {fname(f, names, dump_names)}")
                if len(others) > 8:
                    print(f"  ... {len(others) - 8} more")
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
