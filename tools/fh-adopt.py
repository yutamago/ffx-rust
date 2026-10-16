#!/usr/bin/env python3
"""Takes Fahrenheit's name for every function of ours that has a different
one ("fh: <name>" in its symbols/ffx.sym comment, written by
tools/fh-import.py): renames it in ffx.sym (the old name follows "was"), and
everywhere the old name appears as a whole word in tracked files (code,
comments, docs). Run tools/fh-import.py afterwards.

Usage: tools/fh-adopt.py [--dry-run]
"""
import pathlib
import re
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import importlib.util  # noqa: E402

REPO = pathlib.Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("fh_import", REPO / "tools/fh-import.py")
fh = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fh)

dry = "--dry-run" in sys.argv
sym = REPO / "symbols/ffx.sym"
lines = sym.read_text(encoding="utf-8").splitlines()
names = {fh.ENTRY.match(l)[3] for l in lines if fh.ENTRY.match(l)}
renames = {}
out = []
for line in lines:
    m = fh.ENTRY.match(line)
    if not m or (m[4] or "").startswith("fh "):
        out.append(line)
        continue
    theirs = re.search(r"fh: (\S+)$", m[4] or "")
    if not theirs:
        out.append(line)
        continue
    new = fh.token(theirs[1])
    old = m[3]
    if new == old:
        out.append(line)
        continue
    if new in names:
        print(f"skip {old}: {new} already names another address")
        out.append(line)
        continue
    renames[old] = new
    names.add(new)
    comment = re.sub(r"\s*;?\s*fh: \S+$", "", m[4] or "")
    comment = f"{comment}; was {old}" if comment else f"was {old}"
    out.append(f"{f'{m[1]} {m[2]} {new}':<45} # {comment}")

for old, new in renames.items():
    print(f"{old} -> {new}")
if dry or not renames:
    sys.exit(0)
sym.write_text("\n".join(out) + "\n", encoding="utf-8")
pat = re.compile(r"(?<![A-Za-z0-9_])(" + "|".join(map(re.escape, sorted(renames, key=len, reverse=True))) + r")(?![A-Za-z0-9_])")
files = subprocess.run(["git", "ls-files"], cwd=REPO, capture_output=True, text=True).stdout.split()
changed = 0
for f in files:
    if f == "symbols/ffx.sym":
        continue
    p = REPO / f
    try:
        s = p.read_text(encoding="utf-8")
    except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
        continue
    t = pat.sub(lambda m: renames[m[1]], s)
    if t != s:
        p.write_text(t, encoding="utf-8")
        changed += 1
print(f"{len(renames)} renamed, {changed} files changed")
