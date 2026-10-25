#!/usr/bin/env python3
"""Runtime coverage of the functions tools/classify.py calls dead or debug.

  tools/coverage.py candidates OUT     # the list ffx-hook's coverage mode arms
  tools/coverage.py candidates --all OUT  # every function of the dump instead
                                       # (game, PhyreEngine, CRT: m4-pilot)
  FFX_COVERAGE=OUT <launch the game>   # tools/run-game.sh passes it on
  tools/coverage.py merge OUT.hits...  # adds the hits to symbols/coverage.txt
  tools/classify.py                    # functions that ran leave "dead"

OUT is best on the Windows drive (/mnt/c/...): the game appends one line
per first hit to OUT.hits, and the WSL share is slow for that.
symbols/coverage.txt holds the addresses that ran in any session so far
(one per line, sorted; addresses only, no game data).
"""
import argparse
import os
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import classify  # noqa: E402

REPO = classify.REPO
COVERAGE = REPO / "symbols/coverage.txt"


def exe_path():
    return pathlib.Path(os.environ.get("FFX_GAME_DIR", "~/steam-games/FFX-X2-HD")).expanduser() / "FFX.exe"


def candidates(out, everything=False):
    """`<address> <first byte>` lines: the dead and debug functions of
    symbols/classes.tsv, or with `everything` every function of the Ghidra
    dump (tools/progress.py), the statically linked libraries included."""
    cls = {}
    for line in (REPO / "symbols/classes.tsv").read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            a, c = line.split("\t")[:2]
            cls[int(a, 16)] = c
    _, secs = classify.read_pe(exe_path())
    text = [(va, blob) for name, va, blob, code in secs if code]
    lines = []
    if everything:
        import progress
        wanted = sorted(a for a, _, _ in progress.functions())
    else:
        wanted = sorted(a for a, c in cls.items() if c in ("dead", "debug"))
    for a in wanted:
        for va, blob in text:
            if va <= a < va + len(blob):
                lines.append(f"{a:08x} {blob[a - va]:02x}")
    pathlib.Path(out).write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{len(lines)} candidates -> {out}")


def merge(paths):
    seen = set()
    if COVERAGE.exists():
        seen = {int(x, 16) for x in COVERAGE.read_text(encoding="utf-8").split()}
    before = len(seen)
    for p in paths:
        for x in pathlib.Path(p).read_text(encoding="utf-8").split():
            seen.add(int(x, 16))
    COVERAGE.write_text("".join(f"0x{a:08x}\n" for a in sorted(seen)), encoding="utf-8")
    print(f"{len(seen) - before} new, {len(seen)} in {COVERAGE.relative_to(REPO)}")


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("candidates")
    c.add_argument("--all", action="store_true", help="every function of the dump")
    c.add_argument("out")
    sub.add_parser("merge").add_argument("hits", nargs="+")
    args = ap.parse_args()
    if args.cmd == "candidates":
        candidates(args.out, args.all)
    else:
        merge(args.hits)


if __name__ == "__main__":
    main()
