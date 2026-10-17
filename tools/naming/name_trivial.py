#!/usr/bin/env python3
"""Descriptive names for the trivial functions tools/autoport.py matches but
cannot port while they are unnamed (its --unnamed list).

Each name says what the code does: the shape of the autoport row (getter,
setter, copy, compare, ...), the field offset or the global (by its ffx.sym
label when it has one), and the owner when every named caller shares a
prefix (PhyreScene_, Ms, TO, ...). The address suffix keeps them apart and
marks them as ours. Functions symbols/classes.tsv calls dead are left out.
The comment carries the autoport expression and the callers, so a better
name can replace it later.

  tools/autoport.py --unnamed > /tmp/unnamed.tsv
  tools/naming/name_trivial.py /tmp/unnamed.tsv --out PROPOSALS.tsv
  tools/naming/apply.py PROPOSALS.tsv
"""
import argparse
import pathlib
import re
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import symlib  # noqa: E402

STEMS = {
    "store copy to field": "CopyField",
    "compare": "Compare",
    "arithmetic on arguments": "Compute",
    "several stores: set field (constants, arguments, addresses)": "SetFields",
    "pass-through": "Identity",
    "store argument to field": "SetField",
    "load and compute": "LoadCompute",
    "several stores: copy into field": "CopyFields",
    "return deref u8 (pointer chain)": "GetDerefU8",
    "return deref u16 (pointer chain)": "GetDerefU16",
    "return deref u32 (pointer chain)": "GetDerefU32",
    "return field u32": "GetField",
    "return field u16": "GetFieldU16",
    "return field u8": "GetFieldU8",
    "return constant": "ReturnConst",
    "return argument + offset": "FieldAddr",
    "several stores: computed, into field": "StoreComputed",
    "store computed to field": "SetFieldComputed",
    "return address of a field's target": "GetFieldPtr",
    "return global u32": "GetGlobal",
    "store argument to global": "SetGlobal",
    "store constant to field": "SetFieldConst",
    "several stores: set global (constants, arguments, addresses)": "SetGlobals",
    "several stores: copy into field/global": "CopyFields",
    "return address of global": "AddrOfGlobal",
    "several stores: copy into deref": "CopyDeref",
    "several stores: computed, into deref/field": "StoreComputed",
    "store constant to global": "SetGlobalConst",
    "store bit set to field": "SetFieldBits",
    "store argument to table": "SetTableEntry",
    "store argument to deref": "SetDeref",
}
# exact shapes that deserve a more specific word
SHAPES = [
    (r"^return \(a0 == a1\)$", "Equal"),
    (r"^t0 = u8\[a0\]; t1 = u8\[a1\]; return \(t0 == t1\)$", "EqualU8"),
    (r"^t0 = u32\[a0\]; t1 = u32\[a1\]; return \(t0 == t1\)$", "EqualU32"),
    (r"^t0 = u32\[a0\]; return \(t0 == a0\)$", "PointsToSelf"),
    (r"^return \(\(\(a0 \+ 0xffffffff\) \+ a1\) & not\(\(a1 \+ 0xffffffff\)\)\)$", "AlignUp"),
    (r"<< 0x8\) \| .*>> 0x8\)\)$", "ByteSwap32"),
    (r"^t0 = u32\[\(a0 \+ (0x[0-9a-f]+)\)\]; return \(t0 \+ \(a1 \* (0x[0-9a-f]+)\)\)$", "ElementAddr"),
    (r"^t0 = u32\[\(a0 \+ \(a1 \* 0x4\)\)\]; return t0$", "GetWordAt"),
]


def label_names(ent):
    return {a: v[1] for a, v in ent.items() if v[0] == "label"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("unnamed")
    ap.add_argument("--out", required=True)
    ap.add_argument("--classes", default=str(symlib.REPO / "symbols/classes.tsv"))
    args = ap.parse_args()
    _, ent = symlib.load()
    names = {a: v[1] for a, v in ent.items() if v[0] == "function"}
    labels = label_names(ent)
    cls = {}
    for line in open(args.classes, encoding="utf-8"):
        if line and not line.startswith("#"):
            a, c = line.split("\t")[:2]
            cls[int(a, 16)] = c
    callers = defaultdict(set)
    with open(symlib.DUMP / "call_sites.tsv", encoding="utf-8") as f:
        next(f)
        for line in f:
            a, _, c = line.rstrip("\n").split("\t")
            if "EXTERNAL" not in c:
                callers[int(c, 16)].add(int(a, 16))
    used = set(names.values())
    rows = []
    skipped = 0
    for line in open(args.unnamed, encoding="utf-8"):
        p = line.rstrip("\n").split("\t")
        if len(p) < 5:
            continue
        a, module, cc, pattern, expr = int(p[0], 16), p[1], p[2], p[3], p[4]
        if a in names or cls.get(a) == "dead":
            skipped += 1
            continue
        stem = STEMS.get(pattern, re.sub(r"[^A-Za-z0-9]+", " ", pattern).title().replace(" ", ""))
        for rx, word in SHAPES:
            if re.search(rx, expr):
                stem = word
                break
        detail = ""
        m = re.search(r"\[\(a[01] \+ (0x[0-9a-f]+)\)\]", expr) or re.search(r"\[a[01]\]()", expr)
        if stem in ("GetField", "GetFieldU16", "GetFieldU8", "SetField", "SetFieldConst", "SetFieldComputed",
                    "SetFieldBits", "CopyField", "GetFieldPtr") and m:
            detail = m[1][2:] or "0"
        elif stem == "FieldAddr":
            m = re.search(r"return \(a[0-3] \+ (0x[0-9a-f]+)\)", expr)
            detail = m[1][2:] if m else ""
        elif stem == "ReturnConst":
            m = re.search(r"return (0x[0-9a-f]+|-?\d+)$", expr)
            detail = m[1].replace("0x", "") if m else ""
        elif stem in ("GetGlobal", "SetGlobal", "SetGlobalConst", "AddrOfGlobal"):
            m = re.search(r"&(0x[0-9a-f]+)", expr)
            if m:
                g = int(m[1], 16)
                lab = labels.get(g)
                stem = {"GetGlobal": "Get", "SetGlobal": "Set", "SetGlobalConst": "Set",
                        "AddrOfGlobal": "AddrOf"}[stem] + ("_" + lab if lab else "Global")
                detail = "" if lab else f"{g:08x}"
        elif stem == "ElementAddr":
            m = re.search(SHAPES[6][0], expr)
            detail = f"{m[2][2:]}At{m[1][2:]}"
        cs = sorted(callers[a])
        cn = [names[c] for c in cs if c in names]
        prefix = ""
        if cn and len(cn) == len(cs):
            heads = {n.split("_")[0] + "_" if "_" in n[:20]
                     else re.match(r"^([A-Za-z][a-z0-9]*|[A-Z]+(?=[A-Z][a-z]))", n)[0] for n in cn}
            if len(heads) == 1:
                prefix = heads.pop()
        name = f"{prefix}{stem}{detail}_{a:08x}"
        if name in used:
            skipped += 1
            continue
        used.add(name)
        who = ", ".join(cn[:3]) + (f" and {len(cs) - 3} more" if len(cs) > 3 else "") if cs else "no direct caller"
        rows.append((a, name, f"auto-port shape ({pattern}, {cc}): {expr}; callers: {who}; name ours"))
    symlib.write_tsv(args.out, rows)
    print(f"{len(rows)} names, {skipped} skipped (named or dead) -> {args.out}")


if __name__ == "__main__":
    main()
