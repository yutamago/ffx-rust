#!/usr/bin/env python3
"""Git merge driver for the address-sorted lists (symbols/ffx.sym,
symbols/sizes.tsv, symbols/m5-not-portable.tsv, ...): a three-way merge by
line set, then sorted by address. Branches that each add their own names
conflicted on every merge (neighbouring addresses land in one hunk);
nearly all of those were "keep both, keep it sorted".

  git config merge.ffx-sorted.driver 'python3 -I tools/merge-sorted.py %O %A %B %P'
  .gitattributes: symbols/ffx.sym merge=ffx-sorted

The leading lines before the first address line (the header) are ours
(%A). A line is in the result when it is in ours or theirs and neither side
removed it from the base. Two different lines for the same key (address,
plus the kind column in ffx.sym) where both sides changed it are a real
conflict: ours is kept, theirs is reported, and the driver exits 1 so git
marks the file conflicted (the merged file is still written, sorted, for a
look). Exit 0 otherwise.
"""
import re
import sys

ADDR = re.compile(r"0x[0-9a-fA-F]+")


def split(path):
    with open(path, encoding="utf-8", newline="") as f:
        lines = f.read().splitlines(keepends=True)
    head = []
    for i, line in enumerate(lines):
        if ADDR.match(line):
            return head, lines[i:]
        head.append(line)
    return head, []


def key(line, path):
    parts = line.split()
    if path.endswith(".sym") and len(parts) > 1:
        return (parts[0].lower(), parts[1])
    return (parts[0].lower(),)


def main():
    base, ours, theirs, path = sys.argv[1:5]
    _, b = split(base)
    head, a = split(ours)
    _, t = split(theirs)
    norm = lambda ls: [l if l.endswith("\n") else l + "\n" for l in ls if l.strip()]
    b, a, t = norm(b), norm(a), norm(t)
    bs, as_, ts = set(b), set(a), set(t)
    removed = (bs - as_) | (bs - ts)
    merged = [l for l in a if l not in removed]
    have = set(merged)
    merged += [l for l in t if l not in have and l not in removed]
    # one line per key: both sides changing one entry differently conflicts
    by_key, out, clash = {}, [], []
    base_keys = {key(l, path): l for l in b}
    for l in merged:
        k = key(l, path)
        if k in by_key:
            first = by_key[k]
            if k in base_keys and base_keys[k] in (first, l):
                # one side kept the base line: take the changed one
                keep = l if first == base_keys[k] else first
                out[out.index(first)] = keep
                by_key[k] = keep
            else:
                clash.append((first, l))
            continue
        by_key[k] = l
        out.append(l)
    out.sort(key=lambda l: int(ADDR.match(l).group(0), 16))
    with open(ours, "w", encoding="utf-8", newline="") as f:
        f.writelines(head + out)
    for mine, other in clash:
        sys.stderr.write(f"{path}: both sides changed {key(mine, path)}; kept ours\n  ours:   {mine}  theirs: {other}")
    return 1 if clash else 0


if __name__ == "__main__":
    sys.exit(main())
