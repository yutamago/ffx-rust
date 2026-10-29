#!/usr/bin/env python3
"""Git merge driver for the ported module list (crates/ffx-hook/src/ported/mod.rs).

Branches only add lines there (`mod x;`, `all.extend(x::hooks());`), so a
union merge keeps both sides. Plain `merge=union` left two kinds of damage
(2026-10-29): a line both sides added twice (`mod m5p_registry_load_a;`, a
build error), and the `mod` lines out of order (`cargo fmt --check` fails).
This driver does the union, drops the second copy of a repeated `mod` or
`all.extend` line, then runs rustfmt on the result.

  git config merge.ffx-modlist.driver 'python3 -I <clone>/tools/merge-modlist.py %O %A %B %P'
  .gitattributes: crates/ffx-hook/src/ported/mod.rs merge=ffx-modlist

Exit 0; 1 only when git merge-file itself fails.
"""
import re
import subprocess
import sys

LISTED = re.compile(r"^\s*((pub(\(crate\))? )?mod \w+;|all\.extend\(\w+::hooks\(\)\);)\s*$")


def main():
    base, ours, theirs = sys.argv[1:4]
    if subprocess.run(["git", "merge-file", "--union", ours, base, theirs]).returncode < 0:
        return 1
    with open(ours, encoding="utf-8", newline="") as f:
        lines = f.read().splitlines(keepends=True)
    seen = set()
    out = []
    for line in lines:
        if LISTED.match(line):
            key = line.strip()
            if key in seen:
                continue
            seen.add(key)
        out.append(line)
    text = "".join(out)
    # rustfmt sorts the `mod` lines (from stdin, so it doesn't follow them to files);
    # a file it can't parse is written as is, for the build to report
    fmt = subprocess.run(["rustfmt", "--edition", "2024"], input=text, capture_output=True, text=True)
    if fmt.returncode == 0:
        text = fmt.stdout
    with open(ours, "w", encoding="utf-8", newline="") as f:
        f.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
