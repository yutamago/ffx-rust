#!/usr/bin/env python3
"""What to port next (docs/porting.md): game-code functions not ported yet
whose callees are all ported already, or library code (left to the
original for now), or nothing at all. Leaves first, then by callers (the
functions most used), then smaller first.

  tools/worklist.py [--module M | --global] [--unclaimed] [--limit N] [--named] [--max-bytes B]

Columns: address, module, bytes, callers, name (and calls into imports,
which need a platform equivalent). Callers and callees are the dump's
call_sites.tsv plus the direct calls and jumps FFX.exe's bytes show
(tools/refs.py: calls from code Ghidra left as a label, tail jumps).
Functions too short to hook (under five bytes, whatever they do: a bare
`ret`, `ret 4`, `xor eax, eax; ret`, a field getter) block nobody: no port
can replace them, so their callers' ports reproduce them or call them.

Mutual recursion: unported functions that block each other in a cycle
(a calls b, b calls a, through any number of steps) are listed together
once nothing outside the cycle blocks any of them, each marked with the
cycle's size and its first member; port and hook them in one go.

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
import bisect
import os
import pathlib
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))


def cycles(nodes, edges):
    """Strongly connected components (Tarjan, iterative) of the graph
    `edges` over `nodes`: {node: its component (a frozenset)}."""
    index, low, comp = {}, {}, {}
    stack, on = [], set()
    counter = 0
    for root in nodes:
        if root in index:
            continue
        work = [(root, iter(edges.get(root, ())))]
        index[root] = low[root] = counter
        counter += 1
        stack.append(root)
        on.add(root)
        while work:
            v, it = work[-1]
            for w in it:
                if w not in index:
                    index[w] = low[w] = counter
                    counter += 1
                    stack.append(w)
                    on.add(w)
                    work.append((w, iter(edges.get(w, ()))))
                    break
                if w in on:
                    low[v] = min(low[v], index[w])
            else:
                work.pop()
                if work:
                    low[work[-1][0]] = min(low[work[-1][0]], low[v])
                if low[v] == index[v]:
                    members = []
                    while True:
                        w = stack.pop()
                        on.discard(w)
                        members.append(w)
                        if w == v:
                            break
                    group = frozenset(members)
                    for w in members:
                        comp[w] = group
    return comp


def raw_edges(size, callees, callers):
    """Adds the direct call/jmp edges FFX.exe's bytes show that
    call_sites.tsv misses (tools/refs.py): calls from code Ghidra left as a
    label count as callers (keyed by the site), tail jumps and calls inside
    a function body as its callees."""
    import refs
    exe = refs.default_exe()
    if not exe.exists():
        return
    starts = sorted(size)
    for t, sites in refs.index(exe).callers().items():
        if t not in size:
            continue
        for s in sites:
            i = bisect.bisect_right(starts, s) - 1
            a = starts[i] if i >= 0 else None
            if a is not None and s < a + size[a]:
                if a != t:
                    callees[a].add(t)
                    callers[t].add(a)
            else:
                callers[t].add(s)


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
    # replaced: the ported! tables and the hand-written hooks without a
    # trampoline (crates/ffx-hook/src/lib.rs), as tools/progress.py counts
    ported = {a for a, state in progress.hooks().items() if state == "replaced"}
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
    raw_edges(size, callees, callers)

    # functions too short to hook (under five bytes: a bare `ret` such as
    # dbgPrintf, `ret 4`, a constant or field return) cannot be replaced by
    # any port: they block no caller, whatever their category
    short = {a for a, _, s in rows if s < 5}

    def blocking(a):
        return [c for c in callees[a]
                if c != a and c not in ported and c not in short
                and progress.KIND.get(module.get(c)) == "game" and logic(c)]

    # cycles among the unported: the members are ready together when
    # everything that blocks any of them is in the cycle
    block = {a: blocking(a) for a, _, s in rows
             if progress.KIND.get(module.get(a)) == "game" and a not in ported}
    comp = cycles(sorted(block), block)

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
        group = comp.get(a, frozenset((a,)))
        if any(c not in group for w in group for c in block.get(w, ())):
            continue
        out.append((len(callees[a] - {a}) > 0 or imports[a] > 0, -len(callers[a]), s, a, m,
                    group if len(group) > 1 else None))
    out.sort(key=lambda r: r[:5])
    for _, neg_callers, s, a, m, group in out[: args.limit]:
        imp = f" (+{imports[a]} imports)" if imports[a] else ""
        stale = f" (stale claim: {taken[a].get('agent')})" if a in taken else ""
        cyc = f" (cycle of {len(group)} with 0x{min(group):08x}: port together)" if group else ""
        print(f"0x{a:08x} {m:8} {s:6} {-neg_callers:5}  {names.get(a, '')}{imp}{stale}{cyc}")
    print(f"{len(out)} ready", file=sys.stderr)


if __name__ == "__main__":
    main()
