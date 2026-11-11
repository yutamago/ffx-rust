#!/usr/bin/env python3
"""Runtime coverage of the functions tools/classify.py calls dead or debug.

  tools/coverage.py candidates OUT     # the list ffx-hook's coverage mode arms
  tools/coverage.py candidates --all OUT  # every function of the dump instead
                                       # (game, PhyreEngine, CRT: m4-pilot)
  tools/coverage.py candidates --rust OUT # the same without the movie
                                       # libraries (stage 2): the 32-bit
                                       # stage 1 gate (coverage-run.sh --rust)
  FFX_COVERAGE=OUT <launch the game>   # tools/run-game.sh passes it on
  tools/coverage.py merge OUT.hits...  # adds the hits to symbols/coverage.txt
  tools/coverage.py report HITS... [--out FILE]  # the hits by module, named
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


def movie_library(addr):
    """The movie library (stage 2) holding `addr`, else None: the ranges of
    docs/formats/platform.md "Code", kept in tools/unicorn-report.py."""
    return __import__("unicorn-report").movie_library(addr)


def candidates(out, everything=False, rust=False):
    """`<address> <first byte>` lines: the dead and debug functions of
    symbols/classes.tsv, or with `everything` every function of the Ghidra
    dump (tools/progress.py), the statically linked libraries included;
    with `rust` every function but those in the movie libraries."""
    cls = {}
    for line in (REPO / "symbols/classes.tsv").read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            a, c = line.split("\t")[:2]
            cls[int(a, 16)] = c
    _, secs = classify.read_pe(exe_path())
    text = [(va, blob) for name, va, blob, code in secs if code]
    lines = []
    if everything or rust:
        import progress
        wanted = sorted(a for a, _, _ in progress.functions())
        if rust:
            wanted = [a for a in wanted if not movie_library(a)]
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


def report(paths, out=None):
    """The functions the hits files name (one hex address a line, any
    number of files; each file's name is its run), by module
    (tools/progress.py), with their ffx.sym names and the runs that hit
    them. In a run with every hook on (coverage-run.sh --rust) each hit is
    an FFX.exe function that ran its original code: the stage 1 gate wants
    none."""
    import progress
    from collections import defaultdict
    names = progress.symbols()
    dump = {a: n for a, n, _ in progress.functions()}
    runs = defaultdict(list)
    for p in paths:
        tag = pathlib.Path(p).name
        tag = tag[:-5] if tag.endswith(".hits") else tag
        for x in pathlib.Path(p).read_text(encoding="utf-8").split():
            a = int(x, 16)
            if tag not in runs[a]:
                runs[a].append(tag)
    by_module = defaultdict(list)
    for a in sorted(runs):
        name = names.get(a) or dump.get(a) or f"FUN_{a:08x}"
        mod = "movie libraries" if movie_library(a) else (progress.module_of(name) or "other")
        by_module[mod].append((a, name))
    lines = [f"{len(runs)} functions ran their original code ({len(paths)} hits files)"]
    for mod in sorted(by_module, key=lambda m: (-len(by_module[m]), m)):
        lines.append(f"  {mod}: {len(by_module[mod])}")
    for mod in sorted(by_module, key=lambda m: (-len(by_module[m]), m)):
        lines.append("")
        lines.append(f"{mod}:")
        for a, name in by_module[mod]:
            r = runs[a]
            lines.append(f"  0x{a:08x} {name}: {', '.join(r[:4])}{f' +{len(r) - 4}' if len(r) > 4 else ''}")
    text = "\n".join(lines) + "\n"
    if out:
        pathlib.Path(out).write_text(text, encoding="utf-8")
    return text


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("candidates")
    c.add_argument("--all", action="store_true", help="every function of the dump")
    c.add_argument("--rust", action="store_true",
                   help="every function of the dump but the movie libraries (stage 2)")
    c.add_argument("out")
    sub.add_parser("merge").add_argument("hits", nargs="+")
    r = sub.add_parser("report")
    r.add_argument("hits", nargs="+")
    r.add_argument("--out")
    args = ap.parse_args()
    if args.cmd == "candidates":
        candidates(args.out, args.all, args.rust)
    elif args.cmd == "report":
        sys.stdout.write(report(args.hits, args.out))
    else:
        merge(args.hits)


if __name__ == "__main__":
    main()
