#!/usr/bin/env python3
"""Checks the ported replacements (crates/ffx-hook/src/ported/*.rs) against
the Ghidra dump: every address is a function start, of at least five bytes
(the jump patch), not ported twice, and named alike in symbols/ffx.sym.
Exits 1 on problems."""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

rows = {a: (n, size) for a, n, size in progress.functions()}
names = progress.symbols()
bad = 0
seen = {}
for f in sorted((progress.REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
    import re
    for name, addr in re.findall(r"^\s*(\w+) @ 0x([0-9a-f_]+) =>", f.read_text(encoding="utf-8"), re.M):
        a = int(addr.replace("_", ""), 16)
        where = f"{f.name}: {name} @ {a:#010x}"
        if a in seen:
            print(f"{where}: also ported as {seen[a]}")
            bad += 1
        seen[a] = name
        if a not in rows:
            print(f"{where}: not a function start in the dump")
            bad += 1
            continue
        if rows[a][1] < 5:
            print(f"{where}: only {rows[a][1]} bytes, too small for the jump")
            bad += 1
        if names.get(a) and names[a] != name:
            print(f"{where}: ffx.sym calls it {names[a]}")
            bad += 1
print(f"{len(seen)} ported, {bad} problems")
sys.exit(1 if bad else 0)
