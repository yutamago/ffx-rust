#!/usr/bin/env python3
"""Names functions through strings shared with the Switch build.

A pair is our function and a Switch function that are each the only user of
a string both builds hold exactly once. Our function takes the Switch name
when every such string points at the same Switch function, no other of our
functions points at it, and the Switch function has a real name. Pairs that
contradict an existing name are listed on stderr (not applied).

  tools/naming/strmatch.py --out PROPOSALS.tsv [--min-len 6]
  then: tools/naming/apply.py PROPOSALS.tsv

Inputs default to $FFX_WORK/decomp/strings.tsv and the export's
str_switch.strings.tsv / cg_switch.funcs.tsv (--ours, --theirs, --theirs-funcs).
"""
import argparse
import pathlib
import sys
from collections import Counter, defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import symlib  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ours", default=str(symlib.DUMP / "strings.tsv"))
    ap.add_argument("--theirs", default=str(symlib.EXPORT / "str_switch.strings.tsv"))
    ap.add_argument("--theirs-funcs", default=str(symlib.EXPORT / "cg_switch.funcs.tsv"))
    ap.add_argument("--min-len", type=int, default=6)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    ours, theirs = symlib.read_strings(args.ours), symlib.read_strings(args.theirs)
    tn = {a: n for a, n, _ in symlib.read_theirs(args.theirs_funcs)}
    votes = defaultdict(Counter)
    example = {}
    for value, defs in ours.items():
        other = theirs.get(value)
        if len(value) < args.min_len or len(defs) != 1 or not other or len(other) != 1:
            continue
        fa, fb = defs[0], other[0]
        if len(fa) == 1 and len(fb) == 1:
            votes[fa[0]][fb[0]] += 1
            example.setdefault((fa[0], fb[0]), value)
    best = {a: cs.most_common(1)[0] for a, cs in votes.items() if len(cs) == 1}
    users = Counter(b for b, _ in best.values())

    _, ent = symlib.load()
    rows = []
    for a, (b, n) in sorted(best.items()):
        if users[b] != 1 or b not in tn:
            continue
        name = symlib.token(tn[b])
        if a in ent:
            if not ent[a][1].startswith(name):
                print(f"differs 0x{a:08x} {ent[a][1]}: strings say {tn[b]} ({n})", file=sys.stderr)
            continue
        ex = repr(example[(a, b)][:60]).replace("#", "%23")
        rows.append((a, name, f"str-match switch: only user of {n} string(s) held once by both builds, "
                              f"e.g. {ex}; Switch 0x{b:x} {tn[b]} (symbols from Fahrenheit's server export)"))
    symlib.write_tsv(args.out, rows)
    print(f"{len(rows)} proposals -> {args.out}")


if __name__ == "__main__":
    main()
