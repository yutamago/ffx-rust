#!/usr/bin/env python3
"""What to port next (docs/porting.md): game-code functions not ported yet
whose callees are all ported already, or library code (left to the
original for now), or nothing at all. Leaves first, then by callers (the
functions most used), then smaller first.

  tools/worklist.py [--module M | --global] [--unclaimed] [--limit N] [--named] [--max-bytes B]

Columns: address, module, bytes, callers, name (and calls into imports,
which need a platform equivalent). Functions too short to hook that only
return (dbgPrintf, FlushCache) block nobody.

Only logic is listed (symbols/classes.tsv, tools/classify.py), and only
logic callees block: glue, platform, debug and dead functions are not
ported exactly, so a port keeps calling the original for them (through
the bridge). --all lists every category.

--global is the default made explicit: every game module in one list (an
error with --module). --unclaimed leaves out functions another agent has
claimed (tools/claim.py); stale claims (over 12 hours) are listed again,
marked.
"""
import argparse
import os
import pathlib
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))


def is_stub(a, size):
    """A function that only returns (`ret`, `ret n`, `xor eax,eax; ret`, ...)."""
    exe = os.environ.get("FFX_GAME_DIR")
    if not exe:
        return size == 1
    import subprocess
    out = subprocess.run(["objdump", "-d", "--no-show-raw-insn", f"--start-address={a:#x}",
                          f"--stop-address={a + size:#x}", str(pathlib.Path(exe).expanduser() / "FFX.exe")],
                         capture_output=True, text=True, check=False).stdout
    ops = [line.split("\t", 1)[1].split()[0] for line in out.splitlines() if ":\t" in line]
    return bool(ops) and ops[-1] == "ret" and all(o in ("ret", "xor", "mov", "or") for o in ops)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--module")
    ap.add_argument("--limit", type=int, default=50)
    ap.add_argument("--named", action="store_true", help="only functions with a name")
    ap.add_argument("--max-bytes", type=int, default=0)
    ap.add_argument("--min-bytes", type=int, default=5)
    ap.add_argument("--all", action="store_true", help="every category, not only logic")
    ap.add_argument("--global", dest="glob", action="store_true", help="every game module (the default)")
    ap.add_argument("--unclaimed", action="store_true", help="skip functions claimed with tools/claim.py")
    args = ap.parse_args()
    if args.glob and args.module:
        ap.error("--global and --module exclude each other")
    taken = {}
    if args.unclaimed:
        import claim
        taken = claim.claims()

    names = progress.symbols()
    rows = progress.functions()
    module = progress.placement(rows, names)
    size = {a: s for a, _, s in rows}
    ported = set(progress.ported())
    category = progress.classes()

    def logic(a):
        return category.get(a, "logic") == "logic"

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

    # functions too short to hook (a bare `ret` such as dbgPrintf, FlushCache)
    # do nothing a port has to replace: they block no caller
    stub = {a for a, _, s in rows if s < 5 and is_stub(a, s)}

    def blocking(a):
        return [c for c in callees[a]
                if c != a and c not in ported and c not in stub
                and progress.KIND.get(module.get(c)) == "game" and logic(c)]

    out = []
    for a, _, s in rows:
        m = module.get(a)
        if progress.KIND.get(m) != "game" or a in ported or s < args.min_bytes:
            continue
        if args.module and m != args.module:
            continue
        if not args.all and not logic(a):
            continue
        if args.named and a not in names:
            continue
        if args.max_bytes and s > args.max_bytes:
            continue
        if a in taken and not taken[a]["stale"]:
            continue
        b = blocking(a)
        if b:
            continue
        out.append((len(callees[a] - {a}) > 0 or imports[a] > 0, -len(callers[a]), s, a, m))
    out.sort()
    for _, neg_callers, s, a, m in out[: args.limit]:
        imp = f" (+{imports[a]} imports)" if imports[a] else ""
        stale = f" (stale claim: {taken[a].get('agent')})" if a in taken else ""
        print(f"0x{a:08x} {m:8} {s:6} {-neg_callers:5}  {names.get(a, '')}{imp}{stale}")
    print(f"{len(out)} ready", file=sys.stderr)


if __name__ == "__main__":
    main()
