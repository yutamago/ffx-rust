#!/usr/bin/env python3
"""Merges name proposals into symbols/ffx.sym.

Input: TSV files of `address<TAB>name<TAB>comment` (what bracket.py,
strmatch.py and import_self.py write, or hand-written). Addresses that
already have a name and names already used elsewhere are skipped and listed;
nothing existing is renamed. Keeps ffx.sym sorted.

  tools/naming/apply.py PROPOSALS.tsv... [--dry-run]
"""
import argparse
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import symlib  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("tsv", nargs="+")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()
    rows = []
    for path in args.tsv:
        with open(path, encoding="utf-8") as f:
            for line in f:
                if line.strip() and not line.startswith("#"):
                    a, n, c = line.rstrip("\n").split("\t", 2)
                    rows.append((int(a, 16), "function", n, c))
    if args.dry_run:
        _, ent = symlib.load()
        used = {v[1] for v in ent.values()}
        for a, _, n, _ in rows:
            state = "named" if a in ent else "name taken" if n in used else "new"
            print(f"0x{a:08x} {n}: {state}")
        return
    added, skipped = symlib.add(rows)
    symlib.check_sorted()
    print(f"added {len(added)}, skipped {len(skipped)}")
    for a, n, why in skipped:
        print(f"  skip 0x{a:08x} {n}: {why}", file=sys.stderr)


if __name__ == "__main__":
    main()
