#!/usr/bin/env python3
"""check-licences.py: the release builds link no GPL code (docs/rewrite-plan.md,
M5 stage 5 gate).

  tools/check-licences.py [--crate NAME[:TARGET[:FEATURES]]]... [--features F]

For each release crate (default: ffx-dinput8 for i686-pc-windows-msvc and
ffx-host for x86_64-pc-windows-msvc with `swf`, the release package's
features: tools/package-release.sh), lists its normal (non-dev, non-build)
dependency tree with default features off (`cargo tree -e normal
--no-default-features`) and reports every package whose licence expression
binds it to a GPL family licence (GPL, LGPL, AGPL): one where every `OR`
alternative names one (`Apache-2.0 OR GPL-2.0-only`, Ruffle's `self_cell`,
offers Apache-2.0 and passes). Unicorn (`unicorn-engine`,
`unicorn-engine-sys`, GPL-2.0) is the one expected today: it is behind the
`dev-unicorn` feature of ffx-hook/ffx-dinput8 and must stay out of the
release tree. Exit status 1 when any release tree has a GPL package, 2
when cargo tree fails.
"""
import argparse
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
DEFAULT = ["ffx-dinput8:i686-pc-windows-msvc", "ffx-host:x86_64-pc-windows-msvc:swf"]
GPL = re.compile(r"\b(A|L)?GPL")


def _split(expr, op):
    """`expr`'s operands of `op` (" OR " / " AND ") outside parentheses."""
    out, depth, start, i = [], 0, 0, 0
    while i < len(expr):
        c = expr[i]
        depth += (c == "(") - (c == ")")
        if depth == 0 and expr.startswith(op, i):
            out.append(expr[start:i])
            i += len(op)
            start = i
            continue
        i += 1
    out.append(expr[start:])
    return [o.strip() for o in out]


def gpl_bound(expr):
    """Whether a licence expression leaves no choice but a GPL-family
    licence (SPDX: AND binds tighter than OR; the old `MIT/Apache-2.0`
    form is an OR)."""
    expr = expr.strip()
    while expr.startswith("(") and expr.endswith(")") and _balanced(expr[1:-1]):
        expr = expr[1:-1].strip()
    alts = _split(expr, " OR ")
    if len(alts) == 1 and "/" in expr and "(" not in expr:
        alts = [a.strip() for a in expr.split("/")]
    if len(alts) > 1:
        return all(gpl_bound(a) for a in alts)
    parts = _split(expr, " AND ")
    if len(parts) > 1:
        return any(gpl_bound(p) for p in parts)
    return bool(GPL.search(expr))


def _balanced(expr):
    depth = 0
    for c in expr:
        depth += (c == "(") - (c == ")")
        if depth < 0:
            return False
    return depth == 0


def tree(crate, target, features=""):
    cmd = ["cargo", "tree", "-q", "-e", "normal", "--no-default-features", "-p", crate,
           "--prefix", "none", "--format", "{p}\t{l}"]
    if target:
        cmd += ["--target", target]
    if features:
        cmd += ["--features", features]
    r = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True)
    if r.returncode != 0:
        sys.stderr.write(r.stderr)
        return None
    seen = {}
    for line in r.stdout.splitlines():
        pkg, _, lic = line.partition("\t")
        pkg = pkg.replace(" (*)", "").strip()
        lic = lic.replace(" (*)", "")
        if pkg:
            seen[pkg] = lic.strip()
    return seen


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--crate", action="append", help="crate[:target] (repeatable)")
    ap.add_argument("--features", default="")
    args = ap.parse_args()
    bad = 0
    for spec in args.crate or DEFAULT:
        crate, _, rest = spec.partition(":")
        target, _, features = rest.partition(":")
        pkgs = tree(crate, target, ",".join(f for f in (features, args.features) if f))
        if pkgs is None:
            print(f"{spec}: cargo tree failed")
            sys.exit(2)
        gpl = sorted((p, l) for p, l in pkgs.items() if gpl_bound(l))
        if gpl:
            bad += 1
            print(f"{spec}: {len(gpl)} GPL package(s) in the release tree ({len(pkgs)} packages):")
            for p, l in gpl:
                print(f"  {p}: {l}")
        else:
            print(f"{spec}: no GPL package ({len(pkgs)} packages)")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
