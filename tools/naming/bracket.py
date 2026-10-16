#!/usr/bin/env python3
"""Names functions from the alphabetical layout of FFX.exe.

Inside a source file this build lays functions out in name order (the
Switch build keeps definition order). For each gap of unnamed functions
between two named ones, where the named neighbours on both sides form a
sorted run of Switch names (--run on each side) whose Switch addresses lie
within one source file, the candidates are that file's Switch names (within
--span of the run's Switch addresses) that sort between the two neighbours
and are not used yet. When there are exactly as many candidates as gap
functions and every size ratio is within 0.25..4, they are paired in order.
Holdout precision on this build: 98-99% with --run 3, ~98% with --run 2.

  tools/naming/bracket.py --out PROPOSALS.tsv [--run 3] [--span 0x800]
  tools/naming/bracket.py --holdout 0.1 [--seed 1]      # measure precision
  then: tools/naming/apply.py PROPOSALS.tsv  (repeat until nothing is new)

Inputs default to $FFX_WORK/decomp/functions.tsv and
$FFX_WORK/ghidra-remote/export/cg_switch.funcs.tsv (--funcs, --theirs).
"""
import argparse
import pathlib
import random
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import symlib  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--funcs", default=str(symlib.DUMP / "functions.tsv"))
    ap.add_argument("--theirs", default=str(symlib.EXPORT / "cg_switch.funcs.tsv"))
    ap.add_argument("--out")
    ap.add_argument("--run", type=int, default=3, help="sorted named neighbours needed on each side")
    ap.add_argument("--span", type=lambda s: int(s, 0), default=0x800,
                    help="margin around the run's Switch addresses")
    ap.add_argument("--holdout", type=float, default=0)
    ap.add_argument("--seed", type=int, default=1)
    args = ap.parse_args()
    if not args.out and not args.holdout:
        ap.error("--out or --holdout")

    funcs, osize = symlib.read_funcs(args.funcs)
    _, ent = symlib.load()
    names = {a: v[1] for a, v in ent.items() if v[0] == "function"}
    t_by_tok = defaultdict(list)
    t_name = {}
    for a, n, s in symlib.read_theirs(args.theirs):
        t_by_tok[symlib.token(n)].append((a, s))
        t_name[a] = n

    anchor = {a: n for a, n in names.items() if n in t_by_tok}
    hidden = {}
    if args.holdout:
        rng = random.Random(args.seed)
        for a in rng.sample(sorted(anchor), int(len(anchor) * args.holdout)):
            hidden[a] = names.pop(a)
            anchor.pop(a)
    used = set(names.values())
    named_idx = [i for i, a in enumerate(funcs) if a in names]

    results = []
    for k in range(len(named_idx) - 1):
        i, j = named_idx[k], named_idx[k + 1]
        gap = [funcs[x] for x in range(i + 1, j) if osize[funcs[x]] > 0]
        lo, hi = k - args.run + 1, k + args.run
        if not gap or lo < 0 or hi >= len(named_idx):
            continue
        run = [funcs[named_idx[x]] for x in range(lo, hi + 1)]
        rn = [names[a] for a in run]
        if any(rn[x] >= rn[x + 1] for x in range(len(rn) - 1)) or not all(a in anchor for a in run):
            continue
        before, after = names[funcs[i]], names[funcs[j]]
        insts = [[x for x, _ in t_by_tok[anchor[a]]] for a in run]
        flat = sorted(x for s in insts for x in s)
        med = flat[len(flat) // 2]
        pts = [min(s, key=lambda x: abs(x - med)) for s in insts]
        if max(pts) - min(pts) > 0x8000:
            continue
        smin, smax = min(pts) - args.span, max(pts) + args.span
        cands = sorted((t, loc[0]) for t, inst in t_by_tok.items()
                       if before < t < after and t not in used
                       for loc in [[(x, s) for x, s in inst if smin <= x <= smax]] if loc)
        if len(cands) != len(gap):
            continue
        if not all(0.25 < osize[a] / max(s, 1) < 4 for a, (_, (_, s)) in zip(gap, cands)):
            continue
        for a, (t, (x, _)) in zip(gap, cands):
            results.append((a, t, x, before, after, len(gap)))

    if args.holdout:
        got = [r for r in results if r[0] in hidden]
        right = sum(1 for r in got if r[1] == hidden[r[0]])
        for r in got:
            if r[1] != hidden[r[0]]:
                print(f"wrong 0x{r[0]:08x}: {r[1]}, was {hidden[r[0]]}", file=sys.stderr)
        print(f"holdout {len(hidden)}: {len(got)} recovered, {right} right, {len(results) - len(got)} others")
        return
    symlib.write_tsv(args.out, [
        (a, t, f"layout: alphabetical run between {p} and {n} ({g} unnamed in the gap); Switch 0x{x:x} "
               f"{t_name[x]} is the only unused name of that source file sorting there (symbols from "
               f"Fahrenheit's server export)")
        for a, t, x, p, n, g in results])
    print(f"{len(results)} proposals -> {args.out}")


if __name__ == "__main__":
    main()
