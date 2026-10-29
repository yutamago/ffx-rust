#!/usr/bin/env python3
"""Census of the spell overlays (magicFiles/FFX/magic_*.dll) for sizing
their port (docs/rewrite-plan.md, M5): functions, how many are distinct
after masking what only the link moved, how widely each is shared, which
g_MagicImports slots each calls, and which are copies of FFX.exe functions
(ported or not).

  FFX_GAME_DIR=... tools/overlay-census.py [--out DIR] [--jobs N]

Functions: a DLL's .text is cut at the exports, the direct call
targets, the pointers into .text from data sections (function tables) and
every 16-byte aligned address after int3 padding; a function runs to the
next start, trailing int3 dropped.

Masking: a relocated dword becomes a token for what it points at: a
function (by its hash), an import (by name: the overlays import only
MSVCR110 and KERNEL32), a read-only constant (the 8 bytes there) or a
writable global (masked). A rel32 call or jump leaving the function becomes
the callee's token. Then
  shape = sha1(bytes with every token zeroed, the token kinds)
  hash  = sha1(bytes, the tokens with callees by shape)
so two copies match when their code and constants are the same and they
call the same-shaped functions; `hash` is the identity, distinct functions
are distinct hashes. `ops` (the mnemonic sequence) groups the functions
that differ only in immediates, offsets and constants.

g_MagicImports slots: per function, the [reg + off] operands through a
register loaded from the DLL's table global (InitMagicPRX stores the
table there; the most-loaded global), as in tools/check-clobber.py's
magic_slots; `off / 4` is the slot.

FFX.exe: the same hashes for every function of tools/reach.py's graph, so
an overlay function equal to an FFX.exe one (hash or shape) is a copy; the
report says whether that copy is hooked (ported).

Outputs (default $FFX_WORK/m5/scope):
  overlay-functions.tsv  one row per distinct function: hash, size, DLL
                         count, first DLL and address, shape, ops group
                         size, slots, FFX.exe copy (address, name, hooked)
  overlay-dll-functions.tsv  one row per function per DLL: address, size,
                         hash, ops group, DLLs sharing the hash, slots
                         (input of tools/magic-worklist.py)
  overlays-census.md     the counts
"""
import argparse
import bisect
import hashlib
import os
import pathlib
import re
import struct
import subprocess
import sys
from collections import Counter, defaultdict
from concurrent.futures import ProcessPoolExecutor

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402
import reach  # noqa: E402
import refs  # noqa: E402

WORK = progress.WORK
INSN = re.compile(r"^\s*([0-9a-f]+):\t(.*)$")
TABLE_LOAD = re.compile(r"mov\s+(e[a-ds][xip]),(?:DWORD PTR )?ds:0x([0-9a-f]+)$")
MEM = re.compile(r"\[(e[a-ds][xip])(?:\+0x([0-9a-f]+))?\]")


def objdump(path, lo=None, hi=None):
    """[(address, instruction text)] of objdump -d (intel)."""
    cmd = ["objdump", "-d", "--no-show-raw-insn", "-M", "intel"]
    if lo is not None:
        cmd += [f"--start-address={lo:#x}", f"--stop-address={hi:#x}"]
    out = subprocess.run(cmd + [str(path)], capture_output=True, text=True, check=True).stdout
    res = []
    for line in out.splitlines():
        m = INSN.match(line)
        if m:
            res.append((int(m.group(1), 16), m.group(2).split("#")[0].strip()))
    return res


class Module:
    """A PE image cut into functions, with tokens for hashing."""

    def __init__(self, img, insns, starts, name=""):
        self.img, self.name = img, name
        self.iat = img.imports()
        self.text = next(blob for _, va, blob, code in img.secs if code)
        self.lo, self.hi = img.text_lo, img.text_hi
        self.insns = insns
        self.pcs = [a for a, _ in insns]
        self.starts = sorted(s for s in starts if self.lo <= s < self.hi)
        self.writable = {name for name, *_ in img.secs if name in (".data", ".bss", ".tls")}

    def fn_of(self, a):
        i = bisect.bisect_right(self.starts, a) - 1
        return self.starts[i] if i >= 0 else None

    def end_of(self, s):
        i = bisect.bisect_right(self.starts, s)
        e = self.starts[i] if i < len(self.starts) else self.hi
        while e > s + 1 and self.text[e - 1 - self.lo] == 0xCC:
            e -= 1
        return e

    def functions(self):
        """{start: (end, masked bytes, [(kind, value)], mnemonics, insn texts)}."""
        out = {}
        relocs = self.img.relocs
        for s in self.starts:
            e = self.end_of(s)
            b = bytearray(self.text[s - self.lo:e - self.lo])
            toks = []
            i = bisect.bisect_left(relocs, s)
            masked = set()
            while i < len(relocs) and relocs[i] < e:
                r = relocs[i]
                i += 1
                if r + 4 > e:
                    continue
                v = struct.unpack_from("<I", self.text, r - self.lo)[0]
                b[r - s:r - s + 4] = b"\0\0\0\0"
                masked.add(r)
                if v in self.iat:
                    toks.append((r - s, "I", self.iat[v].split("!")[-1]))
                elif self.lo <= v < self.hi:
                    toks.append((r - s, "F", self.fn_of(v), v - self.fn_of(v)))
                else:
                    sec = self.img.section(v)
                    if sec in (".rdata", ".rodata"):
                        d0, d1 = self.img.dword(v), self.img.dword(v + 4)
                        toks.append((r - s, "R", d0, d1))
                    else:
                        toks.append((r - s, "D"))
            j = bisect.bisect_left(self.pcs, s)
            mn = []
            texts = []
            while j < len(self.pcs) and self.pcs[j] < e:
                pc, t = self.insns[j]
                j += 1
                mn.append(t.split(None, 1)[0] if t else "")
                texts.append(t)
                m = reach.DIRECT.match(t)
                if not m or m.group(1).startswith("loop"):
                    continue
                tgt = int(m.group(2), 16)
                nxt = self.pcs[j] if j < len(self.pcs) else e
                if s <= tgt < e or nxt - pc < 5:
                    continue  # internal, or a short (rel8) jump
                rel = nxt - 4
                if rel in masked or not s <= rel < e:
                    continue
                b[rel - s:rel - s + 4] = b"\0\0\0\0"
                toks.append((rel - s, "F", self.fn_of(tgt) if self.lo <= tgt < self.hi else None, 0))
            toks.sort(key=lambda t: t[0])
            out[s] = (e, bytes(b), toks, mn, texts)
        return out


def shape_of(b, toks):
    h = hashlib.sha1(b)
    for t in toks:
        h.update(f"{t[0]}:{t[1]};".encode())
        if t[1] in ("I", "R"):
            h.update(repr(t[2:]).encode())
    return h.hexdigest()[:16]


def hash_of(b, toks, shapes):
    h = hashlib.sha1(b)
    for t in toks:
        if t[1] == "F":
            h.update(f"{t[0]}:F{shapes.get(t[2], '?')}+{t[3]};".encode())
        elif t[1] == "D":
            h.update(f"{t[0]}:D;".encode())
        else:
            h.update(f"{t[0]}:{t[1]}{t[2:]!r};".encode())
    return h.hexdigest()[:16]


def ops_of(mn):
    return hashlib.sha1(" ".join(mn).encode()).hexdigest()[:16]


def table_global(texts_all):
    loads = Counter()
    for t in texts_all:
        m = TABLE_LOAD.match(t)
        if m:
            loads[m.group(2)] += 1
    return loads.most_common(1)[0][0] if loads else None


def slots_of(texts, table):
    """g_MagicImports slots this function reaches through the table."""
    held = set()
    out = set()
    for t in texts:
        m = TABLE_LOAD.match(t)
        if m and m.group(2) == table:
            held.add(m.group(1))
            continue
        for mm in MEM.finditer(t):
            off = int(mm.group(2) or "0", 16)
            if mm.group(1) in held and off < 4 * reach.MAGIC_SLOTS:
                out.add(off // 4)
        parts = t.split(None, 1)
        if parts and parts[0] in ("call", "ret", "jmp"):
            held = set()
        elif len(parts) > 1:
            dest = parts[1].split(",")[0]
            if dest in held and parts[0] not in ("push", "cmp", "test"):
                held.discard(dest)
    return out


def census_dll(path):
    img = reach.Image(path)
    insns = objdump(path)
    starts = {img.text_lo, *img.exports()}
    for a, t in insns:
        m = reach.DIRECT.match(t)
        if m and m.group(1) == "call":  # jmp targets are mostly loop entries
            starts.add(int(m.group(2), 16))
    for r in img.relocs:
        v = img.dword(r)
        # pointers in .text are mostly switch tables and their case labels
        if img.in_text(v) and not img.in_text(r):
            starts.add(v)
    text = next(blob for _, va, blob, code in img.secs if code)
    for k in range(1, len(text)):
        a = img.text_lo + k
        if a % 16 == 0 and text[k - 1] == 0xCC and text[k] != 0xCC:
            starts.add(a)
    pcs = {a for a, _ in insns}
    starts = {s for s in starts if s in pcs}
    mod = Module(img, insns, starts, path.name)
    fns = mod.functions()
    table = table_global([t for _, t in insns])
    rows = {}
    for s, (e, b, toks, mn, texts) in fns.items():
        rows[s] = (e - s, b, toks, ops_of(mn), sorted(slots_of(texts, table)) if table else [], mn)
    return path.name, len(text), img.exports(), rows


def exe_chunk(args):
    exe, lo, hi = args
    return objdump(exe, lo, hi)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--out", type=pathlib.Path, default=WORK / "m5/scope")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    args = ap.parse_args()
    exe = refs.default_exe()
    dlls = sorted((pathlib.Path(exe).parent / "magicFiles/FFX").glob("magic_*.dll"))

    with ProcessPoolExecutor(args.jobs) as pool:
        results = list(pool.map(census_dll, dlls, chunksize=4))

    # ---- shapes then hashes, per DLL (callees are in the same DLL)
    per_dll = {}
    for name, text_size, exports, rows in results:
        shapes = {s: shape_of(r[1], r[2]) for s, r in rows.items()}
        hashes = {s: hash_of(r[1], r[2], shapes) for s, r in rows.items()}
        per_dll[name] = (text_size, exports, rows, shapes, hashes)

    distinct = {}  # hash -> dict
    slot_dlls = {}  # slot -> DLLs using it
    for name in sorted(per_dll):
        text_size, exports, rows, shapes, hashes = per_dll[name]
        for s, (size, _, _, ops, slots, mn) in rows.items():
            h = hashes[s]
            d = distinct.get(h)
            if d is None:
                d = distinct[h] = {"size": size, "dlls": set(), "first": (name, s), "shape": shapes[s],
                                   "ops": ops, "slots": set(slots), "copies": 0, "mn": mn}
            for sl in slots:
                slot_dlls.setdefault(sl, set()).add(name)
            d["dlls"].add(name)
            d["copies"] += 1
            d["slots"] |= set(slots)

    # ---- FFX.exe: the same hashes
    img = reach.Image(exe)
    g = reach.Graph(img, args.jobs, True)
    bounds = g.starts
    step = max(1, len(bounds) // (args.jobs * 4))
    chunks = [(exe, bounds[i], bounds[i + step] if i + step < len(bounds) else img.text_hi)
              for i in range(0, len(bounds), step)]
    with ProcessPoolExecutor(args.jobs) as pool:
        insns = [x for part in pool.map(exe_chunk, chunks) for x in part]
    insns.sort()
    emod = Module(img, insns, bounds, "FFX.exe")
    efns = emod.functions()
    eshapes = {s: shape_of(r[1], r[2]) for s, r in efns.items()}
    ehashes = {s: hash_of(r[1], r[2], eshapes) for s, r in efns.items()}
    eops = {s: ops_of(r[3]) for s, r in efns.items()}
    by_hash, by_shape, by_ops = defaultdict(list), defaultdict(list), defaultdict(list)
    for s in efns:
        by_hash[ehashes[s]].append(s)
        by_shape[eshapes[s]].append(s)
        by_ops[eops[s]].append(s)
    hooked = progress.hooks()
    replaced = {a for a, st in hooked.items() if st == "replaced"}

    def exe_copy(d):
        for kind, table, key in (("shape", by_shape, d["shape"]), ("ops", by_ops, d["ops"])):
            cands = table.get(key, ())
            if cands:
                return kind, min(cands, key=lambda a: (a not in replaced, a))
        return None, None

    # ---- near copies: mnemonic 3-gram Jaccard against FFX.exe's game
    # functions of similar length (one representative per ops group)
    def grams(mn):
        return {" ".join(mn[i:i + 3]) for i in range(max(1, len(mn) - 2))}

    game = [s for s in efns if reach.KIND.get(g.module.get(s), "") == "game" and len(efns[s][3]) >= 8]
    egrams = {s: grams(efns[s][3]) for s in game}
    by_len = defaultdict(list)
    for s in game:
        by_len[len(efns[s][3]) // 8].append(s)
    near = {}  # ops group -> (FFX.exe function, Jaccard)
    best_score = {}
    rep_of_ops = {}
    for h, d in distinct.items():
        rep_of_ops.setdefault(d["ops"], h)
    for ops, h in rep_of_ops.items():
        mn = distinct[h]["mn"]
        if len(mn) < 8:
            continue
        gs = grams(mn)
        n = len(mn)
        best, score = None, 0.0
        for k in range(int(n * 0.8) // 8, int(n * 1.25) // 8 + 1):
            for c in by_len.get(k, ()):
                eg = egrams[c]
                j = len(gs & eg) / len(gs | eg)
                if j > score:
                    best, score = c, j
        best_score[ops] = score
        if best is not None and score >= 0.7:
            near[ops] = (best, score)

    ops_groups = Counter(d["ops"] for d in distinct.values())
    n_dlls = len(per_dll)
    args.out.mkdir(parents=True, exist_ok=True)
    stats = Counter()
    sizes = Counter()
    with open(args.out / "overlay-functions.tsv", "w", encoding="utf-8") as f:
        f.write("# hash\tsize\tdlls\tcopies\tfirst dll\taddress there\tshape\tops group (distinct functions "
                "with this mnemonic sequence)\tslots\tFFX.exe match\tFFX.exe address\tname\tmodule\thooked\t"
                "nearest FFX.exe function (mnemonic 3-gram Jaccard >= 0.7)\tjaccard\tits module\tit is hooked\n")
        for h, d in sorted(distinct.items(), key=lambda kv: (-len(kv[1]["dlls"]), -kv[1]["size"], kv[0])):
            first, addr = d["first"]
            # an identical function in FFX.exe: same hash (code, constants and
            # callee shapes), else same shape (callees differ), else same ops
            exact = by_hash.get(h, ())
            if exact:
                kind, ea = "hash", min(exact, key=lambda a: (a not in replaced, a))
            else:
                kind, ea = exe_copy(d)
            d["exe"] = (kind, ea)
            ename = g.name.get(ea, "") if ea else ""
            emodule = g.module.get(ea, "") if ea else ""
            hk = "yes" if ea in replaced else ("no" if ea else "")
            f.write(f"{h}\t{d['size']}\t{len(d['dlls'])}\t{d['copies']}\t{first}\t{addr:#010x}\t{d['shape']}\t"
                    f"{ops_groups[d['ops']]}\t{','.join(map(str, sorted(d['slots'])))}\t{kind or ''}\t"
                    f"{f'0x{ea:08x}' if ea else ''}\t{ename}\t{emodule}\t{hk}\t")
            nb, sc = near.get(d["ops"], (None, 0.0))
            if nb is None:
                f.write("\t\t\t\n")
            else:
                f.write(f"0x{nb:08x} {g.name.get(nb, '')}\t{sc:.2f}\t{g.module.get(nb, '')}\t"
                        f"{'yes' if nb in replaced else 'no'}\n")

    # ---- per DLL: every function with its identity (tools/magic-worklist.py)
    with open(args.out / "overlay-dll-functions.tsv", "w", encoding="utf-8") as f:
        f.write("# dll\taddress\tsize\thash\tops\tdlls sharing the hash\tslots\n")
        for name in sorted(per_dll):
            _, _, rows, _, hashes = per_dll[name]
            for s in sorted(rows):
                size, _, _, ops, slots, _ = rows[s]
                h = hashes[s]
                f.write(f"{name}\t{s:#010x}\t{size}\t{h}\t{ops}\t{len(distinct[h]['dlls'])}\t"
                        f"{','.join(map(str, slots))}\n")

    # ---- counts
    total_fns =sum(len(v[2]) for v in per_dll.values())
    total_code = sum(sum(r[0] for r in v[2].values()) for v in per_dll.values())
    text_total = sum(v[0] for v in per_dll.values())
    dist_size = sum(d["size"] for d in distinct.values())

    def bucket(n):
        if n == 1:
            return "1 DLL"
        if n < 10:
            return "2-9 DLLs"
        if n < 100:
            return "10-99 DLLs"
        if n < n_dlls:
            return f"100-{n_dlls - 1} DLLs"
        return f"all {n_dlls} DLLs"

    for d in distinct.values():
        b = bucket(len(d["dlls"]))
        stats[b] += 1
        sizes[b] += d["size"]
    shapes_distinct = len({d["shape"] for d in distinct.values()})
    ops_distinct = len(ops_groups)
    match = Counter()
    match_size = Counter()
    for d in distinct.values():
        kind, ea = d["exe"]
        key = (kind or "none") + ("" if not ea else (" hooked" if ea in replaced else " not hooked"))
        match[key] += 1
        match_size[key] += d["size"]
    size_hist = Counter()
    for d in distinct.values():
        s = d["size"]
        size_hist["<32" if s < 32 else "32-127" if s < 128 else "128-511" if s < 512 else
                  "512-2047" if s < 2048 else ">=2048"] += 1
    slot_use = Counter()
    for d in distinct.values():
        for sl in d["slots"]:
            slot_use[sl] += 1
    L = ["# Spell overlay census", ""]
    L.append(f"tools/overlay-census.py over {n_dlls} magicFiles/FFX/magic_*.dll; FFX.exe {img.hash}.")
    L.append("")
    L.append("| measure | value |")
    L.append("| --- | --- |")
    L.append(f"| .text total | {text_total // 1024} KB |")
    L.append(f"| functions (all DLLs) | {total_fns} ({total_code // 1024} KB without padding) |")
    L.append(f"| distinct functions (hash) | {len(distinct)} ({dist_size // 1024} KB) |")
    L.append(f"| distinct shapes (callees may differ) | {shapes_distinct} |")
    L.append(f"| distinct mnemonic sequences | {ops_distinct} |")
    L.append("")
    L.append("| shared by | distinct functions | KB |")
    L.append("| --- | --- | --- |")
    order = ["1 DLL", "2-9 DLLs", "10-99 DLLs", f"100-{n_dlls - 1} DLLs", f"all {n_dlls} DLLs"]
    for b in order:
        L.append(f"| {b} | {stats[b]} | {sizes[b] // 1024} |")
    L.append("")
    L.append("| size (bytes) | distinct functions |")
    L.append("| --- | --- |")
    for k in ("<32", "32-127", "128-511", "512-2047", ">=2048"):
        L.append(f"| {k} | {size_hist[k]} |")
    L.append("")
    L.append("| FFX.exe copy | distinct functions | KB |")
    L.append("| --- | --- | --- |")
    for k in sorted(match):
        L.append(f"| {k} | {match[k]} | {match_size[k] // 1024} |")
    L.append("")
    # ---- porting units: drop the runtime every DLL links (>= 100 DLLs:
    # CRT start-up, slot wrappers), then count one port per mnemonic group
    # (copies that differ only in immediates, offsets and constants)
    work = {h: d for h, d in distinct.items() if len(d["dlls"]) < 100}
    groups = defaultdict(list)
    for h, d in work.items():
        groups[d["ops"]].append(d)
    rep_kb = sum(max(x["size"] for x in ds) for ds in groups.values()) // 1024
    multi = sum(1 for ds in groups.values() if len(ds) > 1)
    L.append("| porting units (functions in fewer than 100 DLLs) | count | KB |")
    L.append("| --- | --- | --- |")
    L.append(f"| distinct functions (upper bound) | {len(work)} | {sum(d['size'] for d in work.values()) // 1024} |")
    L.append(f"| distinct shapes | {len({d['shape'] for d in work.values()})} | |")
    L.append(f"| mnemonic groups (lower bound: one port per group; largest member's size) | {len(groups)} | "
             f"{rep_kb} |")
    L.append(f"| of them groups with 2+ distinct members | {multi} | |")
    near_hits = Counter()
    near_kb = Counter()
    for ops, ds in groups.items():
        if ops in near:
            nb, _ = near[ops]
            k = f"{g.module.get(nb, '?')} {'hooked' if nb in replaced else 'not hooked'}"
            near_hits[k] += 1
            near_kb[k] += max(x["size"] for x in ds)
    L.append("")
    L.append(f"Near copies in FFX.exe (a game function with mnemonic 3-gram Jaccard >= 0.7 and 0.8-1.25x the "
             f"length), per mnemonic group: {sum(near_hits.values())} of {len(groups)} groups.")
    L.append("")
    hist = Counter()
    for ops in groups:
        sc = best_score.get(ops)
        hist["under 8 instructions" if sc is None else ">= 0.9" if sc >= 0.9 else "0.7-0.9" if sc >= 0.7
             else "0.5-0.7" if sc >= 0.5 else "0.3-0.5" if sc >= 0.3 else "< 0.3"] += 1
    L.append("Best Jaccard per group: " + ", ".join(f"{k}: {hist[k]}" for k in
                                                     (">= 0.9", "0.7-0.9", "0.5-0.7", "0.3-0.5", "< 0.3",
                                                      "under 8 instructions")))
    L.append("")
    L.append("| FFX.exe module of the nearest function | groups | KB |")
    L.append("| --- | --- | --- |")
    for k, n in near_hits.most_common():
        L.append(f"| {k} | {n} | {near_kb[k] // 1024} |")
    L.append("")
    fp = Counter()
    fp_kb = Counter()
    for d in work.values():
        x87 = any(m.startswith("f") and m not in ("fwait",) for m in d["mn"])
        sse = any(m.endswith(("ss", "ps", "sd", "pd")) or m.startswith("cvt") for m in d["mn"])
        k = "both" if x87 and sse else "x87" if x87 else "SSE" if sse else "no floats"
        fp[k] += 1
        fp_kb[k] += d["size"]
    L.append("Floating point in the porting units: " + ", ".join(
        f"{k} {fp[k]} ({fp_kb[k] // 1024} KB)" for k in ("SSE", "x87", "both", "no floats")))
    thunks = sum(1 for d in work.values() if d["size"] <= 20 and d["mn"] and d["mn"][-1] == "jmp"
                 and d["slots"])
    L.append(f"Slot thunks (at most 20 bytes, a jump through g_MagicImports): {thunks}.")
    targets = {}
    for sl in slot_use:
        v = img.dword(reach.MAGIC_IMPORTS + 4 * sl)
        if img.in_text(v):
            targets[sl] = g.node_of(v)
    tset = set(targets.values())
    L.append(f"The slots the overlays use point at {len(tset)} FFX.exe functions; hooked (ported): "
             f"{len(tset & replaced)}; not: " + ", ".join(
                 f"{g.name[a]} [{g.module[a]}]" for a in sorted(tset - replaced)[:60])
             + (" ..." if len(tset - replaced) > 60 else "") + ".")
    L.append("")
    L.append(f"Slots: {len(slot_use)} g_MagicImports slots used by distinct functions; the most used: "
             + ", ".join(f"{s} ({n})" for s, n in slot_use.most_common(15)))
    # check against docs/formats/magic.md's per-slot overlay counts
    doc = {}
    for line in (progress.REPO / "docs/formats/magic.md").read_text(encoding="utf-8").splitlines():
        m = re.match(r"\|\s*(\d+)\s*\|\s*`0x[0-9a-f]+`\s*\|.*\|\s*(\d+)\s*\|\s*(\d+)\s*\|\s*$", line)
        if m:
            doc[int(m[1])] = int(m[2])
    same = sum(1 for s, n in doc.items() if len(slot_dlls.get(s, ())) == n)
    L.append("")
    L.append(f"Slot check: the overlay count per slot equals docs/formats/magic.md's for {same} of {len(doc)} "
             f"slots" + ("; differ: " + ", ".join(f"{s} ({len(slot_dlls.get(s, ()))} vs {n})" for s, n in
                                                  sorted(doc.items()) if len(slot_dlls.get(s, ())) != n)[:600]
                         if same != len(doc) else "") + ".")
    (args.out / "overlays-census.md").write_text("\n".join(L) + "\n", encoding="utf-8")
    print("\n".join(L))


if __name__ == "__main__":
    main()
