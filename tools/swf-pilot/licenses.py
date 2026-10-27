#!/usr/bin/env python3
"""Licences of a crate's normal and build dependencies, from `cargo metadata`.

  cargo metadata --format-version 1 --filter-platform <target> > meta.json
  python3 -I tools/swf-pilot/licenses.py meta.json [crate]

Prints the count per licence expression and lists every crate whose
expression offers neither MIT nor Apache-2.0 (those need a look).
"""
import collections
import json
import sys


def main():
    meta = json.load(open(sys.argv[1]))
    name = sys.argv[2] if len(sys.argv) > 2 else "ffx-swf"
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    root = next(p["id"] for p in meta["packages"] if p["name"] == name)
    seen, stack = set(), [root]
    while stack:
        i = stack.pop()
        if i in seen:
            continue
        seen.add(i)
        for d in nodes[i]["deps"]:
            if any(k["kind"] in (None, "build") for k in d["dep_kinds"]):
                stack.append(d["pkg"])
    seen.discard(root)
    count = collections.Counter()
    crates = collections.defaultdict(list)
    for i in seen:
        p = packages[i]
        lic = p.get("license") or f"(file {p.get('license_file')})"
        count[lic] += 1
        crates[lic].append(f"{p['name']} {p['version']}")
    print(f"{len(seen)} crates")
    for lic, n in count.most_common():
        print(f"{n:5} {lic}")
    print("without MIT or Apache-2.0:")
    for lic, names in sorted(crates.items()):
        if "MIT" not in lic and "Apache-2.0" not in lic:
            print(f"  {lic}: {', '.join(sorted(names))}")


if __name__ == "__main__":
    main()
