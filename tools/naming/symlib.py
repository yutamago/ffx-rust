"""Shared helpers for the naming tools: read symbols/ffx.sym, the Ghidra dump
and a Fahrenheit export, and merge new names into ffx.sym in address order."""
import csv
import os
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parent.parent.parent
SYM = REPO / "symbols/ffx.sym"
WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))
DUMP = WORK / "decomp"
EXPORT = WORK / "ghidra-remote/export"
ENTRY = re.compile(r"^(0x[0-9a-fA-F]+)\s+(\S+)\s+(\S+)")
NOISE = re.compile(r"^(FUN_|thunk_|LAB_|switchD|caseD|Unwind@|Catch@)")


def token(name):
    """Fahrenheit's convention (Class::method -> Class_method), as one token."""
    name = name.replace("::~", "::dtor_").replace("operator=", "operator_assign").replace("::", "_")
    return re.sub(r"[^A-Za-z0-9_]+", "_", name).replace("___", "_").strip("_")


def load(sym=SYM):
    """(header lines, {address: (kind, name, line)})."""
    head, ent = [], {}
    for line in pathlib.Path(sym).read_text(encoding="utf-8").splitlines():
        m = ENTRY.match(line)
        if m:
            ent[int(m[1], 16)] = (m[2], m[3], line)
        else:
            head.append(line)
    return head, ent


def add(rows, sym=SYM):
    """Merges (address, kind, name, comment) rows that name new addresses.
    A name already used elsewhere is skipped and reported, never suffixed.
    Returns (added, skipped)."""
    head, ent = load(sym)
    taken = {v[1]: a for a, v in ent.items()}
    added, skipped = [], []
    for a, kind, name, comment in rows:
        if a in ent:
            skipped.append((a, name, f"address already named {ent[a][1]}"))
        elif name in taken:
            skipped.append((a, name, f"name already at 0x{taken[name]:08x}"))
        else:
            taken[name] = a
            ent[a] = (kind, name, f"0x{a:08x} {kind} {name:<28} # {comment}")
            added.append((a, name))
    pathlib.Path(sym).write_text("\n".join(head + [ent[a][2] for a in sorted(ent)]) + "\n",
                                 encoding="utf-8")
    return added, skipped


def check_sorted(sym=SYM):
    prev = -1
    for line in pathlib.Path(sym).read_text(encoding="utf-8").splitlines():
        m = ENTRY.match(line)
        if m:
            a = int(m[1], 16)
            if a <= prev:
                raise SystemExit(f"ffx.sym not sorted at {line!r}")
            prev = a


def read_funcs(path):
    """Ghidra dump functions.tsv -> sorted [address], {address: bytes}."""
    sizes = {}
    with open(path, encoding="utf-8") as f:
        next(f)
        for line in f:
            p = line.split("\t")
            sizes[int(p[0], 16)] = int(p[2])
    return sorted(sizes), sizes


def read_theirs(path):
    """Fahrenheit export <prefix>.funcs.tsv -> [(address, name, bytes)],
    without PLT stubs (16 bytes from 0x7100870000) and unnamed functions."""
    out = []
    with open(path, encoding="utf-8") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            a, n, size = int(r["address"], 16), r["name"], int(r["body_bytes"])
            if n.startswith("<EXTERNAL>") or NOISE.search(n.rsplit("::", 1)[-1]):
                continue
            if size == 16 and a >= 0x7100870000:
                continue
            out.append((a, n, size))
    return out


def read_strings(path):
    """strings.tsv (address, comma-separated function addresses, value) ->
    {value: [[function, ...] per occurrence]}."""
    out = {}
    with open(path, encoding="utf-8", errors="replace") as f:
        next(f)
        for line in f:
            p = line.rstrip("\n").split("\t", 2)
            if len(p) < 3:
                continue
            fns = [int(x, 16) for x in p[1].split(",") if x]
            out.setdefault(p[2], []).append(fns)
    return out


def write_tsv(path, rows):
    """rows of (address, name, comment) -> TSV that apply.py reads."""
    with open(path, "w", encoding="utf-8") as f:
        for a, n, c in rows:
            f.write(f"0x{a:08x}\t{n}\t{c}\n")
