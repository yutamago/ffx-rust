#!/usr/bin/env python3
"""Names functions from the alphabetical layout and their callees.

Like bracket.py, but for gaps whose number of unnamed functions differs from
the number of unused Switch names sorting there (the PC build splits or
merges some functions, and keeps statics the Switch build inlined). Between
the nearest Switch-named neighbours on either side (sorted, Switch addresses
within --file of each other), every unnamed function is compared with every
unused Switch name sorting between them (within --span of the neighbours'
Switch addresses) by their named callees (Jaccard over names, generic
helpers left out); an order-preserving alignment keeps the pairs with
similarity >= --min and at least --shared common callees, and a pair is
proposed only when neither side has another candidate within --margin of
its score. Holdout precision on this build (0.2 of the anchors hidden, three
seeds): 98-99.5% with the defaults.

  tools/naming/calleematch.py --out PROPOSALS.tsv [--min 0.6] [--shared 1]
  tools/naming/calleematch.py --holdout 0.2 [--seed 1]   # measure precision
  wider: --span 0x3000 --file 0x20000 (holdout 97-99.3%, about twice the names);
  --callers compares named callers too (holdout 99%, a few more)
  then: tools/naming/apply.py PROPOSALS.tsv  (repeat: new names make new gaps)

Inputs: $FFX_WORK/decomp/{functions,call_sites}.tsv and
$FFX_WORK/ghidra-remote/export/cg_switch.{funcs,calls}.tsv.
"""
import argparse
import bisect
import csv
import pathlib
import random
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import symlib  # noqa: E402

GENERIC = {"security_check_cookie", "get_ptr32_func", "get_ptr32", "set_ptr32", "init_ptr32", "ftol2_sse",
           "memcpy", "memset", "memmove", "strlen", "strcpy", "sprintf", "dbgPrintf", "printf", "sqrtf", "cosf",
           "sinf", "crt_abs_thunk", "operator_new", "operator_delete", "Phyre_PhyreFree", "Phyre_PhyreMalloc"}


def our_callees(path, names, callers=False):
    out = defaultdict(set)
    with open(path, encoding="utf-8") as f:
        next(f)
        for line in f:
            a, _, c = line.rstrip("\n").split("\t")
            if "EXTERNAL" in c:
                continue
            n = names.get(int(c, 16))
            if n and n not in GENERIC:
                out[int(a, 16)].add(n)
            m = names.get(int(a, 16))
            if callers and m and int(a, 16) != int(c, 16):
                out[int(c, 16)].add("<" + m)
    return out


def their_callees(path, callers=False):
    out = defaultdict(set)
    with open(path, encoding="utf-8") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            n = r["callee_name"]
            if n.startswith("FUN_") or "EXTERNAL" in n:
                continue
            t = symlib.token(n)
            if t not in GENERIC and r["callee_name"] != r["caller_name"]:
                out[int(r["caller"], 16)].add(t)
                if callers and not r["caller_name"].startswith("FUN_"):
                    out[int(r["callee"], 16)].add("<" + symlib.token(r["caller_name"]))
    return out


def jaccard(a, b):
    return len(a & b) / len(a | b) if a and b else 0.0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--funcs", default=str(symlib.DUMP / "functions.tsv"))
    ap.add_argument("--calls", default=str(symlib.DUMP / "call_sites.tsv"))
    ap.add_argument("--theirs", default=str(symlib.EXPORT / "cg_switch.funcs.tsv"))
    ap.add_argument("--their-calls", default=str(symlib.EXPORT / "cg_switch.calls.tsv"))
    ap.add_argument("--out")
    ap.add_argument("--min", type=float, default=0.6, help="minimum callee similarity")
    ap.add_argument("--shared", type=int, default=1, help="minimum common named callees")
    ap.add_argument("--margin", type=float, default=0.15, help="lead over the next candidate")
    ap.add_argument("--file", type=lambda s: int(s, 0), default=0x8000,
                    help="largest Switch distance between the two neighbours")
    ap.add_argument("--span", type=lambda s: int(s, 0), default=0x800)
    ap.add_argument("--callers", action="store_true", help="compare named callers too")
    ap.add_argument("--holdout", type=float, default=0)
    ap.add_argument("--seed", type=int, default=1)
    args = ap.parse_args()
    if not args.out and not args.holdout:
        ap.error("--out or --holdout")

    funcs, osize = symlib.read_funcs(args.funcs)
    _, ent = symlib.load()
    names = {a: v[1] for a, v in ent.items() if v[0] == "function"}
    theirs = defaultdict(list)
    for a, n, s in symlib.read_theirs(args.theirs):
        theirs[symlib.token(n)].append((a, s))
    tcallees = their_callees(args.their_calls, args.callers)
    anchors = {a for a, n in names.items() if n in theirs}
    hidden = {}
    if args.holdout:
        rng = random.Random(args.seed)
        for a in rng.sample(sorted(anchors), int(len(anchors) * args.holdout)):
            hidden[a] = names.pop(a)
            anchors.discard(a)
    ocallees = our_callees(args.calls, names, args.callers)
    used = set(names.values())
    tsorted = sorted(t for t in theirs if t not in used)
    order = [a for a in funcs if osize[a] > 0]
    anc_idx = [i for i, a in enumerate(order) if a in anchors]

    def their_callee_set(t):
        s = set()
        for x, _ in theirs[t]:
            s |= tcallees.get(x, set())
        return s

    results = []
    for k in range(len(anc_idx) - 1):
        i, j = anc_idx[k], anc_idx[k + 1]
        gap = [order[x] for x in range(i + 1, j) if order[x] not in names]
        if not gap:
            continue
        before, after = names[order[i]], names[order[j]]
        if before >= after:
            continue
        lo_locs = [x for x, _ in theirs[before]]
        hi_locs = [x for x, _ in theirs[after]]
        pair = min(((p, q) for p in lo_locs for q in hi_locs), key=lambda pq: abs(pq[0] - pq[1]))
        if abs(pair[0] - pair[1]) > args.file:
            continue
        smin, smax = min(pair) - args.span, max(pair) + args.span
        lo = bisect.bisect_right(tsorted, before)
        hi = bisect.bisect_left(tsorted, after)
        cands = [t for t in tsorted[lo:hi] if any(smin <= x <= smax for x, _ in theirs[t])]
        if not cands:
            continue
        tc = {t: their_callee_set(t) for t in cands}
        sim = [[0.0] * len(cands) for _ in gap]
        for gi, a in enumerate(gap):
            for ci, t in enumerate(cands):
                common = ocallees.get(a, set()) & tc[t]
                if len(common) >= args.shared:
                    sim[gi][ci] = jaccard(ocallees[a], tc[t])
        # order-preserving alignment maximising the summed similarity
        n, m = len(gap), len(cands)
        best = [[0.0] * (m + 1) for _ in range(n + 1)]
        for gi in range(n - 1, -1, -1):
            for ci in range(m - 1, -1, -1):
                take = sim[gi][ci] + best[gi + 1][ci + 1] if sim[gi][ci] >= args.min else -1
                best[gi][ci] = max(take, best[gi + 1][ci], best[gi][ci + 1])
        gi = ci = 0
        while gi < n and ci < m:
            s = sim[gi][ci]
            if s >= args.min and best[gi][ci] == s + best[gi + 1][ci + 1]:
                row = sorted((v for x, v in enumerate(sim[gi]) if x != ci), reverse=True)
                col = sorted((sim[y][ci] for y in range(n) if y != gi), reverse=True)
                if (not row or row[0] <= s - args.margin) and (not col or col[0] <= s - args.margin):
                    t = cands[ci]
                    x = min(theirs[t], key=lambda xs: abs(xs[0] - pair[0]))[0]
                    common = sorted(ocallees[gap[gi]] & tc[t])
                    results.append((gap[gi], t, x, before, after, s, common))
                gi += 1
                ci += 1
            elif best[gi][ci] == best[gi + 1][ci]:
                gi += 1
            else:
                ci += 1

    # a name two gaps propose (their spans overlap) is ambiguous: drop both
    count = defaultdict(int)
    for r in results:
        count[r[1]] += 1
    results = [r for r in results if count[r[1]] == 1]
    if args.holdout:
        got = [r for r in results if r[0] in hidden]
        right = sum(1 for r in got if r[1] == hidden[r[0]])
        for r in got:
            if r[1] != hidden[r[0]]:
                print(f"wrong 0x{r[0]:08x}: {r[1]}, was {hidden[r[0]]}", file=sys.stderr)
        print(f"holdout {len(hidden)}: {len(got)} recovered, {right} right, {len(results) - len(got)} others")
        return
    symlib.write_tsv(args.out, [
        (a, t, f"layout+callees: sorts between {p} and {q}, and its named callees match the Switch "
               f"0x{x:x} {t}'s (similarity {s:.2f}: {', '.join(c[:5])}); symbols from Fahrenheit's server export")
        for a, t, x, p, q, s, c in results])
    print(f"{len(results)} proposals -> {args.out}")


if __name__ == "__main__":
    main()
