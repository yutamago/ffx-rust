#!/usr/bin/env python3
"""Merges the Fahrenheit team's names into symbols/ffx.sym.

peppy (Fahrenheit lead, owner of their Ghidra server) gave written permission
on 2026-10-16 to use their names and findings. Only names, addresses and
calling conventions are taken; none of their code.

Sources:
  leads.tsv   ~/ffx-work/ghidra-remote/export/leads.tsv, written by merge.py
              from their Ghidra server (their names for our build, names
              carried over from their older Steam build and from the Switch
              build's symbols)
  call.cs     their hand-checked FhCall tables (src/core/call.cs,
              src/core/ffx/call.cs) at a pinned commit

Lines this script writes have a comment starting with "fh " and are rewritten
on every run; edit the comment (drop the "fh " prefix) to keep a line as our
own. Lines of ours are never renamed by the script; we took Fahrenheit's
names for ours on 2026-10-16 (the old name follows "was" in the comment).
Where Fahrenheit names one of our addresses differently, their name is
appended to our comment as "fh: <name>".

Usage: tools/fh-import.py [--leads PATH] [--calls DIR] [--dry-run]
"""
import argparse
import csv
import pathlib
import re
import sys
from collections import Counter

REPO = pathlib.Path(__file__).resolve().parent.parent
SYM = REPO / "symbols/ffx.sym"
WORK = pathlib.Path.home() / "ffx-work"
COMMIT = "ecb6702abdc4573011163b22d943e9ef6761ca5c"
BASE = 0x400000  # their databases load FFX.exe at 0

# Ghidra's function-ID guesses (std::, MFC, D2D1 in a game) say nothing about
# their findings; RTTI records and vftable meta pointers come from Ghidra's own
# RTTI analysis, which our database can run as well, and Rsrc_ labels name
# PE resources.
SKIP_ORIGINS = {"v4:analysis"}
SKIP_LABEL = re.compile(r"(RTTI_|vftable_meta_ptr$|^Rsrc_)")
ENTRY = re.compile(r"^(0x[0-9a-f]{8})\s+(function|label)\s+(\S+)\s*(?:#\s?(.*))?$")
CALL = re.compile(
    r"FhCallHandle<[^>]*>\s+(\w+)\s*=>\s*new\(\s*new FhCallLocation\((?:\"FFX\.exe\",\s*)?0x([0-9A-Fa-f]+)"
)
CONVENTION = re.compile(r"CallingConvention\.(\w+)")


def token(name):
    """Fahrenheit's own convention (Class::method -> Class_method), as one token."""
    name = name.replace("::~", "::dtor_").replace("operator=", "operator_assign")
    name = name.replace("::", "_")
    return re.sub(r"[^A-Za-z0-9_]+", "_", name).replace("___", "_").strip("_")


def read_leads(path):
    out = {}
    with open(path, encoding="utf-8") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            if r["origin"] in SKIP_ORIGINS:
                continue
            leaf = r["name"].rsplit("::", 1)[-1]
            if r["kind"] == "label" and (SKIP_LABEL.search(leaf) or r["name"] == "vftable"):
                continue
            out[int(r["address"], 16)] = dict(kind=r["kind"], name=r["name"], origin=r["origin"],
                                              from_addr=r["from_addr"], cc="")
    return out


def read_calls(root):
    """Named FFX.exe entries of their FhCall tables (FUN_ placeholders skipped)."""
    out = {}
    for rel in ("src/core/call.cs", "src/core/ffx/call.cs"):
        text = (root / rel).read_text(encoding="utf-8")
        prev = 0
        for m in CALL.finditer(text):
            name, off = m[1], m[2]
            cc = ([""] + CONVENTION.findall(text, prev, m.start()))[-1]
            prev = m.end()
            if not name.startswith("FUN_"):
                out[int(off, 16) + BASE] = dict(kind="function", name=name, origin="call.cs",
                                                from_addr="", cc=cc.lower())
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--leads", type=pathlib.Path, default=WORK / "ghidra-remote/export/leads.tsv")
    ap.add_argument("--calls", type=pathlib.Path, default=WORK / "fahrenheit" / COMMIT)
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    fh = read_leads(args.leads)
    for addr, c in read_calls(args.calls).items():
        if addr in fh:  # their hand-checked name wins; keep the convention
            fh[addr].update(name=c["name"] if fh[addr]["name"].replace("::", "_") != c["name"]
                            else fh[addr]["name"], cc=c["cc"], origin=fh[addr]["origin"] + "+call.cs")
        else:
            fh[addr] = c

    lines = SYM.read_text(encoding="utf-8").splitlines()
    head = []
    ours = {}
    for line in lines:
        m = ENTRY.match(line)
        if not m:
            if ours:
                sys.exit(f"comment line among entries, keep them in the header: {line!r}")
            head.append(line)
            continue
        addr, kind, name, comment = int(m[1], 16), m[2], m[3], m[4] or ""
        if comment.startswith("fh "):
            continue  # ours to rewrite
        ours[addr] = (kind, name, re.sub(r"(\s*#)?\s*;?\s*fh: \S+$", "", line))

    taken = Counter(n for _, n, _ in ours.values())
    entries = {}
    for addr, (kind, name, line) in ours.items():
        if addr in fh and token(fh[addr]["name"]) != name:
            line += f"; fh: {fh[addr]['name']}" if "#" in line else f"  # fh: {fh[addr]['name']}"
        entries[addr] = line

    renamed = 0
    for addr in sorted(fh):
        if addr in ours:
            continue
        r = fh[addr]
        name = token(r["name"])
        if not name:
            continue
        if taken[name]:
            name = f"{name}_{addr:08x}"
            renamed += 1
        taken[name] += 1
        parts = ["fh", r["origin"]]
        if r["from_addr"]:
            parts.append(r["from_addr"])
        if r["cc"]:
            parts.append(r["cc"])
        if token(r["name"]) != r["name"]:
            parts.append(r["name"].replace("#", "_"))
        entries[addr] = f"{f'0x{addr:08x} {r["kind"]} {name}':<45} # {' '.join(parts)}"

    body = [entries[a] for a in sorted(entries)]
    counts = Counter(entries[a].split("# fh ")[1].split(":")[0].split()[0] for a in entries if a not in ours)
    print(f"ours {len(ours)} (with a Fahrenheit name {sum(a in fh for a in ours)}), "
          f"imported {len(entries) - len(ours)} {dict(counts)}, suffixed duplicates {renamed}")
    if not args.dry_run:
        SYM.write_text("\n".join(head + body) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
