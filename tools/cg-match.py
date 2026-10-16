#!/usr/bin/env python3
"""Carries names from another build of the game (the Switch build, the older
Steam build) to ours through the call graph.

Anchors are functions both sides already agree on: ours named in
symbols/ffx.sym with the other build's name (by from-address in the
Fahrenheit leads, or by equal name). For an anchor pair, both sides' call
sites in order are cut at the calls to other anchors; between two such cuts,
an equal number of not-yet-matched callees are paired in order. The same is
done backwards, from callees to their callers. Each pairing is a vote (two
when it is the only unknown between two known calls); a pair is accepted
when it has at least --min-votes votes, it is the best for both
functions, and the runner-up has at most half its votes. Accepted pairs
become anchors and the search repeats until nothing new is found.

Inputs:
  ours    $FFX_WORK/decomp/call_sites.tsv, functions.tsv (tools/ghidra/dump.sh)
  theirs  <prefix>.calls.tsv and <prefix>.funcs.tsv from the Fahrenheit
          server (~/ffx-work/ghidra-remote/scripts/RemoteCallGraph.java)
  leads   ~/ffx-work/ghidra-remote/export/leads.tsv (for from-addresses)

Writes <out> (default <prefix>.cg_leads.tsv): our address, their address,
their name, votes, round. tools/fh-import.py merges it as "<origin>:cg".

Usage: tools/cg-match.py --theirs PREFIX --origin switch|v3 [--min-votes 2] [--out PATH]
"""
import argparse
import csv
import os
import pathlib
import re
import sys
from collections import Counter, defaultdict

REPO = pathlib.Path(__file__).resolve().parent.parent
WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))
LEADS = WORK / "ghidra-remote/export/leads.tsv"
NOISE = re.compile(r"^(FUN_|thunk_|LAB_|switchD|caseD|Unwind@|Catch@|entry$)")


def token(name):
    name = name.replace("::~", "::dtor_").replace("operator=", "operator_assign").replace("::", "_")
    return re.sub(r"[^A-Za-z0-9_]+", "_", name).replace("___", "_").strip("_")


def hexaddr(s):
    return int(s, 16)


def read_ours():
    calls = defaultdict(list)
    with open(WORK / "decomp/call_sites.tsv", encoding="utf-8") as f:
        next(f)
        for line in f:
            caller, site, callee = line.rstrip("\n").split("\t")
            if "EXTERNAL" in callee:
                continue
            calls[hexaddr(caller)].append((hexaddr(site), hexaddr(callee)))
    sizes = {}
    with open(WORK / "decomp/functions.tsv", encoding="utf-8") as f:
        next(f)
        for line in f:
            p = line.split("\t")
            sizes[hexaddr(p[0])] = int(p[2])
    names = {}
    for line in (REPO / "symbols/ffx.sym").read_text(encoding="utf-8").splitlines():
        p = line.split("#", 1)[0].split()
        if len(p) == 3 and p[1] == "function":
            names[hexaddr(p[0])] = p[2]
    return calls, sizes, names


def read_theirs(prefix):
    calls = defaultdict(list)
    names = {}
    with open(f"{prefix}.calls.tsv", encoding="utf-8") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            try:
                caller, site, callee = hexaddr(r["caller"]), hexaddr(r["site"]), hexaddr(r["callee"])
            except ValueError:
                continue
            calls[caller].append((site, callee))
    sizes = {}
    with open(f"{prefix}.funcs.tsv", encoding="utf-8") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            a = hexaddr(r["address"])
            sizes[a] = int(r["instructions"])
            if not NOISE.search(r["name"].rsplit("::", 1)[-1]):
                names[a] = r["name"]
    return calls, sizes, names


def ordered(calls):
    return {f: [c for _, c in sorted(v)] for f, v in calls.items()}


def callers_of(calls):
    rev = defaultdict(list)
    for f, sites in calls.items():
        for site, c in sites:
            rev[c].append((f, site))
    return rev


def read_strings(path):
    """value -> list of (string address, [referencing functions])."""
    out = defaultdict(list)
    with open(path, encoding="utf-8", errors="replace") as f:
        next(f)
        for line in f:
            p = line.rstrip("\n").split("\t", 2)
            if len(p) < 3 or len(p[2]) < 8:
                continue
            fns = [hexaddr(x if x.startswith("0x") else "0x" + x) for x in p[1].split(",") if x]
            out[p[2]].append(fns)
    return out


def string_anchors(ours_path, theirs_path):
    """Function pairs that are the only users of a string unique to both sides."""
    ours, theirs = read_strings(ours_path), read_strings(theirs_path)
    votes = defaultdict(Counter)
    for value, defs in ours.items():
        other = theirs.get(value)
        if len(defs) != 1 or not other or len(other) != 1:
            continue
        fa, fb = defs[0], other[0]
        if len(fa) == 1 and len(fb) == 1:
            votes[fa[0]][fb[0]] += 1
    out = {}
    taken = Counter()
    for a, cs in votes.items():
        (b, _), *rest = cs.most_common(2) + [(None, 0)]
        if not rest or rest[0][1] == 0:
            out[a] = b
            taken[b] += 1
    return {a: b for a, b in out.items() if taken[b] == 1}


def lcs(xs, ys):
    """Index pairs of a longest common subsequence of xs and ys."""
    n, m = len(xs), len(ys)
    if not n or not m:
        return []
    # rows of lengths, backwards so the walk below goes forwards
    table = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(n - 1, -1, -1):
        row, below = table[i], table[i + 1]
        x = xs[i]
        for j in range(m - 1, -1, -1):
            row[j] = below[j + 1] + 1 if x == ys[j] else max(below[j], row[j + 1])
    out, i, j = [], 0, 0
    while i < n and j < m:
        if xs[i] == ys[j]:
            out.append((i, j))
            i += 1
            j += 1
        elif table[i + 1][j] >= table[i][j + 1]:
            i += 1
        else:
            j += 1
    return out


def vote(pairs, ours_seq, theirs_seq, ours_known, theirs_known, votes, cap=1500):
    """Aligns each pair's sequences on their known calls; in each gap between
    aligned calls with as many unknowns on both sides, pairs them in order."""
    for a, b in pairs:
        sa, sb = ours_seq.get(a), theirs_seq.get(b)
        if not sa or not sb or len(sa) > cap or len(sb) > cap:
            continue
        # our sequence in their addresses where known
        xa = [ours_known.get(c, ("?", c)) for c in sa]
        xb = [c if c in theirs_known else ("!", c) for c in sb]
        anchors = [(-1, -1), *lcs(xa, xb), (len(sa), len(sb))]
        for (i1, j1), (i2, j2) in zip(anchors, anchors[1:]):
            ga = list(dict.fromkeys(c for c in sa[i1 + 1:i2] if c not in ours_known))
            gb = list(dict.fromkeys(c for c in sb[j1 + 1:j2] if c not in theirs_known))
            if not ga or len(ga) != len(gb):
                continue
            inner = i1 >= 0 and i2 < len(sa)
            weight = 2 if len(ga) == 1 and inner else 1
            for x, y in zip(ga, gb):
                votes[x][y] += weight


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--theirs", required=True)
    ap.add_argument("--origin", required=True)
    ap.add_argument("--min-votes", type=int, default=2)
    ap.add_argument("--out", type=pathlib.Path)
    ap.add_argument("--rounds", type=int, default=20)
    ap.add_argument("--strings", action="store_true",
                    help="also anchor on unique strings (<prefix minus cg_>str_<origin>.strings.tsv"
                         " or --strings-file)")
    ap.add_argument("--strings-file", type=pathlib.Path)
    ap.add_argument("--holdout", type=float, default=0.0,
                    help="hide this share of the anchors and report how many come back right")
    args = ap.parse_args()
    out = args.out or pathlib.Path(f"{args.theirs}.cg_leads.tsv")

    o_calls, o_sizes, o_names = read_ours()
    t_calls, t_sizes, t_names = read_theirs(args.theirs)

    # anchors: from-addresses of this origin in the leads, then equal names
    m = {}  # ours -> theirs
    with open(LEADS, encoding="utf-8") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            if r["kind"] == "function" and r["origin"].startswith(args.origin + ":") and r["from_addr"]:
                a = hexaddr(r["address"])
                if a in o_names:
                    m[a] = hexaddr(r["from_addr"])
    by_token = defaultdict(list)
    for a, n in t_names.items():
        by_token[token(n)].append(a)
    # Equal names; the Switch binary holds FFX and FFX-2 (and two Luas), so
    # a name often has several candidates: take the one whose callees and
    # callers carry the most names our function's have too.
    o_rev_all = callers_of(o_calls)
    t_rev_all = callers_of(t_calls)

    def o_context(a):
        return {o_names[c] for _, c in o_calls.get(a, ()) if c in o_names} | {
            "<" + o_names[f] for f, _ in o_rev_all.get(a, ()) if f in o_names}

    def t_context(b):
        return {token(t_names[c]) for _, c in t_calls.get(b, ()) if c in t_names} | {
            "<" + token(t_names[f]) for f, _ in t_rev_all.get(b, ()) if f in t_names}

    for a, n in o_names.items():
        cands = by_token.get(n, ())
        if a in m or not cands:
            continue
        if len(cands) == 1:
            m[a] = cands[0]
            continue
        ctx = o_context(a)
        scored = sorted(((len(ctx & t_context(b)), b) for b in cands), reverse=True)
        if scored[0][0] >= 1 and scored[0][0] > scored[1][0]:
            m[a] = scored[0][1]
    if args.strings or args.strings_file:
        sf = args.strings_file or pathlib.Path(args.theirs).with_name(f"str_{args.origin}.strings.tsv")
        added = 0
        for a, b in string_anchors(WORK / "decomp/strings.tsv", sf).items():
            if a not in m and b not in set(m.values()):
                m[a] = b
                added += 1
        print(f"string anchors added {added}", file=sys.stderr)
    # one-to-one only
    back = Counter(m.values())
    m = {a: b for a, b in m.items() if back[b] == 1}
    hidden = {}
    if args.holdout:
        import random
        rng = random.Random(1)
        for a in rng.sample(sorted(m), int(len(m) * args.holdout)):
            hidden[a] = m.pop(a)
    print(f"anchors {len(m)}, hidden {len(hidden)}", file=sys.stderr)

    o_seq, t_seq = ordered(o_calls), ordered(t_calls)
    o_rev = {c: [f for f, _ in sorted(v, key=lambda x: x[1])] for c, v in callers_of(o_calls).items()}
    t_rev = {c: [f for f, _ in sorted(v, key=lambda x: x[1])] for c, v in callers_of(t_calls).items()}

    found = {}  # ours -> (theirs, votes, round)
    for rnd in range(1, args.rounds + 1):
        inv = {b: a for a, b in m.items()}
        votes = defaultdict(Counter)
        vote(m.items(), o_seq, t_seq, m, inv, votes)
        vote(m.items(), o_rev, t_rev, m, inv, votes)
        # best for both sides
        best_t = defaultdict(Counter)
        for x, cs in votes.items():
            for y, v in cs.items():
                best_t[y][x] += v
        new = 0
        for x, cs in votes.items():
            if x in m:
                continue
            (y, v), *rest = cs.most_common(2) + [(None, 0)]
            if y is None or y in inv or v < args.min_votes:
                continue
            if rest and rest[0][1] * 2 > v:
                continue
            (x2, v2), *rest2 = best_t[y].most_common(2) + [(None, 0)]
            if x2 != x or (rest2 and rest2[0][1] * 2 > v2):
                continue
            # sizes must be roughly alike (instructions there, bytes here)
            if t_sizes.get(y) and o_sizes.get(x):
                ratio = o_sizes[x] / (t_sizes[y] * 3.5)
                if not 0.2 < ratio < 5:
                    continue
            m[x] = y
            inv[y] = x
            found[x] = (y, v, rnd)
            new += 1
        print(f"round {rnd}: {new} new", file=sys.stderr)
        if not new:
            break

    if hidden:
        back_ = [x for x in hidden if x in found]
        def same(x):
            return token(t_names.get(found[x][0], "?")) == token(t_names.get(hidden[x], "!"))

        right = sum(1 for x in back_ if same(x))
        for x in back_:
            if not same(x):
                print(f"  0x{x:08x} {o_names.get(x)}: graph says {t_names.get(found[x][0])} "
                      f"(0x{found[x][0]:x}, {found[x][1]} votes), lead says {t_names.get(hidden[x])} "
                      f"(0x{hidden[x]:x})", file=sys.stderr)
        wrong_new = len(found) - len(back_)
        print(f"holdout: {len(back_)} of {len(hidden)} found again, {right} right "
              f"({100 * right / max(len(back_), 1):.1f}% precision), {wrong_new} others", file=sys.stderr)
        return

    rows = 0
    with open(out, "w", encoding="utf-8") as f:
        f.write("address\ttheir_address\tname\tvotes\tround\n")
        for x, (y, v, rnd) in sorted(found.items()):
            if y in t_names:
                f.write(f"0x{x:08x}\t0x{y:x}\t{t_names[y]}\t{v}\t{rnd}\n")
                rows += 1
    named_new = sum(1 for x in found if x not in o_names and found[x][0] in t_names)
    print(f"matched {len(found)}, with their name {rows}, new names for us {named_new} -> {out}",
          file=sys.stderr)


if __name__ == "__main__":
    main()
