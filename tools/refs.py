#!/usr/bin/env python3
"""Every reference to an address that FFX.exe's bytes show, whether or not
Ghidra made a function of the code holding it.

  FFX_GAME_DIR=... tools/refs.py <addr|name>... [--no-verify] [--no-cache]

The dump's calls.tsv / call_sites.tsv only list calls from code Ghidra made
a function of; code it left as a label (0xa44ef5, 0xa45012, 0xa452d5 calling
0xa58ff0) is missing there. This tool scans the image itself:

  call   E8 rel32 in .text
  jmp    E9 rel32 in .text (tail calls, thunks)
  jcc    0F 80..8F rel32 in .text
  data   a relocated 4-byte pointer in a data section (vtables, native and
         callback tables, function pointers in .data)
  code   a relocated 4-byte immediate in .text (push offset f, mov [x], f,
         jump tables in .text)

FFX.exe has a base relocation table, so `data` and `code` are exact. The
rel32 scan is raw: an E8/E9 byte inside another instruction can look like a
call. Hits whose opcode byte lies inside a relocated dword are dropped, and
each remaining branch is checked by disassembling the code around it (the
function holding it, or from the end of the function before a label);
"(raw, not decoded)" marks a hit that is not an instruction there, which is
noise. "ghidra" marks a site call_sites.tsv lists too.

A dead verdict (tools/classify.py OVERRIDES, a "no callers" claim) must cite
this tool's output: "tools/refs.py: 0 refs" or the refs it found.

The index is cached in $FFX_WORK/cache/rawrefs-<hash of FFX.exe>.pickle
(about 1 s to build, 0.1 s to load). Importable: `refs.index()` returns
an Index; `Index.to(addr)` gives [(site, kind)], `Index.callers()` the raw
call/jmp sites by target (worklist.py uses it).
"""
import argparse
import bisect
import hashlib
import os
import pathlib
import pickle
import re
import struct
import sys
import tempfile
from array import array
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

WORK = progress.WORK
VERSION = 1  # bump when the index format or the scan changes
KINDS = ("call", "jmp", "jcc", "data", "code")
BRANCH = re.compile(rb"\xe8|\xe9|\x0f[\x80-\x8f]", re.S)


def default_exe():
    return pathlib.Path(os.environ.get("FFX_GAME_DIR", "~/steam-games/FFX-X2-HD")).expanduser() / "FFX.exe"


def read_pe(data):
    """(image base, [(name, va, bytes, is code)], [relocated va])."""
    pe = struct.unpack_from("<I", data, 0x3c)[0]
    nsec, = struct.unpack_from("<H", data, pe + 6)
    opt_size, = struct.unpack_from("<H", data, pe + 20)
    opt = pe + 24
    base, = struct.unpack_from("<I", data, opt + 28)
    ndirs, = struct.unpack_from("<I", data, opt + 92)
    reloc_rva, reloc_size = struct.unpack_from("<II", data, opt + 96 + 8 * 5) if ndirs > 5 else (0, 0)
    secs = []
    for i in range(nsec):
        o = opt + opt_size + 40 * i
        name = data[o:o + 8].rstrip(b"\0").decode("latin-1")
        vsize, va, rsize, raw = struct.unpack_from("<IIII", data, o + 8)
        chars, = struct.unpack_from("<I", data, o + 36)
        secs.append((name, va + base, data[raw:raw + min(rsize, vsize or rsize)], bool(chars & 0x20000000), raw, va))

    def file_off(rva):
        for _, _, blob, _, raw, va in secs:
            if va <= rva < va + len(blob):
                return raw + rva - va
        return None

    relocs = []
    o = file_off(reloc_rva) if reloc_size else None
    end = (o or 0) + reloc_size
    while o is not None and o + 8 <= end:
        page, size = struct.unpack_from("<II", data, o)
        if size < 8:
            break
        for (e,) in struct.iter_unpack("<H", data[o + 8:o + size]):
            if e >> 12 == 3:  # IMAGE_REL_BASED_HIGHLOW
                relocs.append(base + page + (e & 0xfff))
        o += size
    return base, [s[:4] for s in secs], relocs


class Index:
    """Sorted (target, site, kind) triples of one FFX.exe."""

    def __init__(self, targets, sites, kinds):
        self.targets, self.sites, self.kinds = targets, sites, kinds

    def to(self, addr):
        lo = bisect.bisect_left(self.targets, addr)
        hi = bisect.bisect_right(self.targets, addr, lo)
        return [(self.sites[i], KINDS[self.kinds[i]]) for i in range(lo, hi)]

    def callers(self, kinds=("call", "jmp")):
        """{target: [site]} of the rel32 branches of these kinds."""
        want = {KINDS.index(k) for k in kinds}
        out = defaultdict(list)
        for t, s, k in zip(self.targets, self.sites, self.kinds):
            if k in want:
                out[t].append(s)
        return out


def build(data):
    base, secs, relocs = read_pe(data)
    reloc_set = set(relocs)
    text = [(va, blob) for _, va, blob, code in secs if code]
    lo = min(va for va, _ in text)
    hi = max(va + len(blob) for va, blob in text)
    triples = []
    for va, blob in text:
        for m in BRANCH.finditer(blob):
            i = m.start()
            n = len(m.group())
            if i + n + 4 > len(blob):
                continue
            site = va + i
            # an opcode byte inside a relocated dword is an operand byte
            if any(site - k in reloc_set for k in range(4)):
                continue
            t = (site + n + 4 + struct.unpack_from("<i", blob, i + n)[0]) & 0xffffffff
            if lo <= t < hi:
                triples.append((t, site, 0 if n == 1 and blob[i] == 0xe8 else 1 if n == 1 else 2))
    for r in relocs:
        for _, va, blob, code in secs:
            if va <= r <= va + len(blob) - 4:
                triples.append((struct.unpack_from("<I", blob, r - va)[0], r, 4 if code else 3))
                break
    triples.sort()
    return Index(array("I", (t for t, _, _ in triples)), array("I", (s for _, s, _ in triples)),
                 bytes(k for _, _, k in triples))


_INDEX = {}


def index(exe=None, use_cache=True):
    exe = pathlib.Path(exe or default_exe()).expanduser().resolve()
    if exe in _INDEX:
        return _INDEX[exe]
    data = exe.read_bytes()
    key = hashlib.sha256(data).hexdigest()[:16]
    path = WORK / "cache" / f"rawrefs-{key}-v{VERSION}.pickle"
    idx = None
    if use_cache:
        try:
            with open(path, "rb") as f:
                t, s, k = pickle.load(f)
            idx = Index(array("I", t), array("I", s), k)
        except (OSError, EOFError, pickle.UnpicklingError, ValueError):
            idx = None
    if idx is None:
        idx = build(data)
        if use_cache:
            path.parent.mkdir(parents=True, exist_ok=True)
            fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".")
            with os.fdopen(fd, "wb") as f:
                pickle.dump((idx.targets.tobytes(), idx.sites.tobytes(), idx.kinds), f,
                            protocol=pickle.HIGHEST_PROTOCOL)
            os.replace(tmp, path)
    _INDEX[exe] = idx
    return idx


def sym_names():
    """Address -> name of every function and label line in ffx.sym."""
    out = {}
    for line in (progress.REPO / "symbols/ffx.sym").read_text(encoding="utf-8").splitlines():
        p = line.split("#", 1)[0].split()
        if len(p) == 3 and p[1] in ("function", "label"):
            out[int(p[0], 16)] = p[2]
    return out


def parse_addr(text, by_name):
    try:
        return int(text, 16)
    except ValueError:
        if text in by_name:
            return by_name[text]
        sys.exit(f"refs.py: {text}: not an address or a name in ffx.sym / functions.tsv")


def decoded(exe, start, site):
    """Whether linear disassembly from `start` has a branch at `site`."""
    import disasm_cache
    if not start or site - start > 0x8000:
        start = site - 0x40  # objdump resynchronises within a few instructions
    code = disasm_cache.objdump(exe, start, site + 8 - start)
    return code.get(site, "").startswith(("call", "j"))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("addrs", nargs="+", metavar="ADDR|NAME")
    ap.add_argument("--no-verify", action="store_true", help="do not disassemble around raw branch sites")
    ap.add_argument("--no-cache", action="store_true", help="rebuild the index and do not write it")
    args = ap.parse_args()
    exe = default_exe()
    idx = index(exe, use_cache=not args.no_cache)
    rows = progress.functions()
    starts = [a for a, _, _ in rows]
    size = {a: s for a, _, s in rows}
    names = {a: n for a, n, _ in rows}
    names.update(sym_names())
    by_name = {n: a for a, n in names.items()}
    ghidra = set()
    with open(WORK / "decomp/call_sites.tsv", encoding="utf-8") as f:
        next(f)
        for line in f:
            _, site, _ = line.rstrip("\n").split("\t")
            ghidra.add(int(site, 16))

    labels = sorted(names)

    def where(site, kind):
        if kind == "data":
            i = bisect.bisect_right(labels, site) - 1
            if i >= 0 and site - labels[i] < 0x1000:
                return f"{names[labels[i]]}+{site - labels[i]:#x}", None
            return "", None
        i = bisect.bisect_right(starts, site) - 1
        if i < 0:
            return "?", None
        a = starts[i]
        if site < a + size[a]:
            return f"{names.get(a, hex(a))}+{site - a:#x}", a
        # label code past the end of a dump function: name the nearest label
        j = bisect.bisect_right(labels, site) - 1
        lab = labels[j] if j >= 0 and labels[j] >= a + size[a] else None
        if lab is not None:
            return f"{names[lab]}+{site - lab:#x} (outside every dump function body)", a + size[a]
        return f"after {names.get(a, hex(a))} (outside every dump function body)", a + size[a]

    for text in args.addrs:
        addr = parse_addr(text, by_name)
        found = idx.to(addr)
        print(f"{addr:#010x} {names.get(addr, '')}: {len(found)} refs")
        for site, kind in found:
            place, start = where(site, kind)
            note = ""
            if kind in ("call", "jmp", "jcc"):
                if site in ghidra:
                    note = " ghidra"
                elif args.no_verify:
                    note = " raw"
                elif decoded(exe, start, site):
                    note = " raw (decoded)"
                else:
                    note = " (raw, not decoded)"
            print(f"  {site:#010x} {kind:4} {place}{note}")


if __name__ == "__main__":
    main()
