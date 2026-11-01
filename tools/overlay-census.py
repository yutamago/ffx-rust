#!/usr/bin/env python3
"""Census of the spell overlays (magicFiles/FFX/magic_*.dll) for sizing
their port (docs/rewrite-plan.md, M5): functions, how many are distinct
after masking what only the link moved, how widely each is shared, which
g_MagicImports slots each calls, and which are copies of FFX.exe functions
(ported or not).

  FFX_GAME_DIR=... tools/overlay-census.py [--out DIR] [--old DIR] [--jobs N]

Functions (docs/formats/magic.md, "The census's functions"): recursive
descent over a DLL's .text (objdump; an address the linear sweep did not
decode as an instruction start is disassembled again from there). A
function starts only at the entry point, an export, a pointer into .text
from a data section, a direct call target, a code pointer taken in
reachable code (a relocated immediate: push offset f, mov [x], offset f)
or a tail-call target (a jump out of a function's range), found to a
fixed point. It covers every instruction reachable from its start by
fall-through, jumps, conditional jumps and jump tables (not calls) up to
the next start, and its jump tables and byte index tables after its
code; it ends at the last of them. The __SEH_prolog4 scope tables'
filters and handlers are labels of their function, not starts.

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
  overlay-unreached.tsv  code (not int3 or zero padding) no function start
                         reaches: not in the census
  overlay-row-changes.tsv  with --old DIR (an earlier census; it is never
                         overwritten): per DLL, the rows that changed
                         (merged, split, resized, recut, dropped, new),
                         also printed per DLL
"""
import argparse
import bisect
import functools
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

    def __init__(self, img, insns, starts, name="", ends=None, fn_insns=None):
        """`ends` and `fn_insns` ({start: end}, {start: [(pc, length,
        text)]}) give each function's extent and instructions (the
        overlays' recursive cut); without them a function runs to the next
        start, trailing int3 dropped, over the linear `insns`."""
        self.img, self.name = img, name
        self.ends, self.fn_insns = ends, fn_insns
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
        if self.ends is not None:
            return self.ends[s]
        i = bisect.bisect_right(self.starts, s)
        e = self.starts[i] if i < len(self.starts) else self.hi
        while e > s + 1 and self.text[e - 1 - self.lo] == 0xCC:
            e -= 1
        return e

    def insns_of(self, s, e):
        """[(pc, next pc, text)] of the function at s (ending at e)."""
        if self.fn_insns is not None:
            return [(pc, pc + n, t) for pc, n, t in self.fn_insns[s]]
        j = bisect.bisect_left(self.pcs, s)
        out = []
        while j < len(self.pcs) and self.pcs[j] < e:
            pc, t = self.insns[j]
            j += 1
            out.append((pc, self.pcs[j] if j < len(self.pcs) else e, t))
        return out

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
            mn = []
            texts = []
            for pc, nxt, t in self.insns_of(s, e):
                mn.append(t.split(None, 1)[0] if t else "")
                texts.append(t)
                m = reach.DIRECT.match(t)
                if not m or m.group(1).startswith("loop"):
                    continue
                tgt = int(m.group(2), 16)
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


def slots_of(texts, table, nslots):
    """g_MagicImports slots (below nslots) this function reaches through the
    table."""
    held = set()
    out = set()
    for t in texts:
        m = TABLE_LOAD.match(t)
        if m and m.group(2) == table:
            held.add(m.group(1))
            continue
        for mm in MEM.finditer(t):
            off = int(mm.group(2) or "0", 16)
            if mm.group(1) in held and off < 4 * nslots:
                out.add(off // 4)
        parts = t.split(None, 1)
        if parts and parts[0] in ("call", "ret", "jmp"):
            held = set()
        elif len(parts) > 1:
            dest = parts[1].split(",")[0]
            if dest in held and parts[0] not in ("push", "cmp", "test"):
                held.discard(dest)
    return out


WIDE = re.compile(r"^\s*([0-9a-f]+):\t((?:[0-9a-f]{2} )+)\s*\t?(.*)$")
JUMP_TABLE = re.compile(r"^jmp\s+DWORD PTR \[e[a-ds][xip]\*4\+0x([0-9a-f]+)\]$")
BYTE_TABLE = re.compile(r"^movzx\s+e[a-ds][xip],BYTE PTR \[e[a-ds][xip]\+0x([0-9a-f]+)\]$")
ENDS = ("ret", "int3", "hlt", "ud2", "iret")


class Code:
    """Recursive-descent instructions of one DLL's .text: {pc: (length,
    text)}. A linear objdump first; an address no decoding reached yet is
    disassembled again from there (a linear sweep desyncs on the jump
    tables MSVC puts after a function)."""

    def __init__(self, img, path):
        self.img, self.path = img, path
        self.at = {}
        self.tried = set()
        self._read(objdump_wide(path))

    def _read(self, lines):
        for pc, n, t in lines:
            self.at.setdefault(pc, (n, t))

    def get(self, pc):
        if pc not in self.at and self.img.in_text(pc) and pc not in self.tried:
            self.tried.add(pc)
            hi = min(pc + 0x100, self.img.text_hi)
            self._read(objdump_wide(self.path, pc, hi, keep_last=hi == self.img.text_hi))
        return self.at.get(pc)


def objdump_wide(path, lo=None, hi=None, keep_last=False):
    """[(address, length, instruction text)] of objdump -d -w (intel)."""
    cmd = ["objdump", "-d", "-w", "-M", "intel"]
    if lo is not None:
        cmd += [f"--start-address={lo:#x}", f"--stop-address={hi:#x}"]
    out = subprocess.run(cmd + [str(path)], capture_output=True, text=True, check=True).stdout
    res = []
    for line in out.splitlines():
        m = WIDE.match(line)
        if m:
            res.append((int(m.group(1), 16), len(m.group(2).split()), m.group(3).split("#")[0].strip()))
    if lo is not None and res and not keep_last:
        res.pop()  # the last one may be cut by the stop address
    return res


def walk(code, start, stops, bound, labels=()):
    """Every instruction reachable from `start` by fall-through, jumps,
    conditional jumps and jump tables (not calls) inside [start, bound)
    (the next start: MSVC lays a function out in one piece). A jump to
    another function's start (`stops`) or out of the range is a tail call
    and is not followed. `labels` are more entries of the same function
    (its SEH filters and handlers). Returns (pcs, call targets, code
    pointers taken, tail-call targets, tables [(address, kind)])."""
    img = code.img
    seen, calls, imms, tails, tables = set(), set(), set(), set(), []
    todo = [start, *labels]

    def inside(t):
        if start <= t < bound and t not in stops:
            return True
        if t != start:
            tails.add(t)
        return False

    while todo:
        pc = todo.pop()
        while pc not in seen and (pc == start or (start < pc < bound and pc not in stops)):
            ins = code.get(pc)
            if ins is None:
                break
            n, t = ins
            seen.add(pc)
            parts = t.split(None, 1)
            mn = parts[0] if parts else ""
            if mn in ("repz", "rep", "bnd") and len(parts) > 1:
                mn = parts[1].split(None, 1)[0]
            i = bisect.bisect_left(img.relocs, pc)
            while i < len(img.relocs) and img.relocs[i] < pc + n:
                v = img.dword(img.relocs[i])
                i += 1
                # a relocated immediate into .text is an address taken in
                # code (push offset f, mov [x], offset f); a displacement
                # (`[...+0x...]`, `ds:0x...`) is a table read
                if img.in_text(v) and f"+0x{v:x}]" not in t and f"ds:0x{v:x}" not in t:
                    imms.add(v)
            m = JUMP_TABLE.match(t)
            if m:
                base = int(m.group(1), 16)
                tables.append((base, "dword"))
                k = base
                while k in img.reloc_set and img.in_text(img.dword(k)) and k not in seen and k not in stops:
                    if start <= img.dword(k) < bound:
                        todo.append(img.dword(k))
                    k += 4
                break
            m = BYTE_TABLE.match(t)
            if m and img.in_text(int(m.group(1), 16)):
                tables.append((int(m.group(1), 16), "byte"))
            if mn in ENDS:
                break
            m = reach.DIRECT.match(t)
            if m:
                tgt = int(m.group(2), 16)
                if m.group(1) == "call":
                    calls.add(tgt)
                elif m.group(1) == "jmp":
                    if not inside(tgt):
                        break
                    pc = tgt
                    continue
                elif inside(tgt):
                    todo.append(tgt)
            elif mn == "jmp":  # through a register or memory
                break
            pc += n
    return seen, calls, imms, tails, tables


def seh_labels(img):
    """The words of the __SEH_prolog4 scope tables (in the CRT's
    __try/__except functions) that point at filters and handlers: labels
    inside the function, not functions. A scope table is a .rdata address
    a relocated word in .text names, with negative GS and EH cookie
    offsets at +0 and +8, followed at +16 by records (enclosing
    level -2 or below 256, filter 0 or a relocated .text pointer, handler a
    relocated .text pointer)."""
    out = set()
    for r in img.relocs:
        if not img.in_text(r):
            continue
        rec = img.dword(r)
        if img.section(rec) not in (".rdata", ".rodata"):
            continue
        # the header: GS and EH cookie offsets (negative frame offsets, -2
        # for none) and their XOR offsets
        gs, eh = img.dword(rec), img.dword(rec + 8)
        if (gs is None or eh is None or gs < 0xFFFF0000 or eh < 0xFFFF0000
                or rec in img.reloc_set or rec + 8 in img.reloc_set):
            continue
        rec += 16
        found = []
        for _ in range(256):
            enc, flt, hnd = (img.dword(rec + 4 * k) for k in range(3))
            if enc is None or not (enc == 0xFFFFFFFE or enc < 256):
                break
            if not (hnd and rec + 8 in img.reloc_set and img.in_text(hnd)):
                break
            if flt and not (rec + 4 in img.reloc_set and img.in_text(flt)):
                break
            found += [rec + 4, rec + 8]
            rec += 12
        out |= set(found)
    return out


def cut_dll(img, code):
    """The functions of one DLL: {start: (end, [(pc, length, text)])} and
    notes. Starts: the entry point, the exports, pointers into .text from
    the data sections, and, to a fixed point, the call targets and the
    code pointers taken in code reachable from them, and the targets of
    tail calls (jumps out of a function's range). A function covers every
    instruction reachable from its start by jumps inside its range (up to
    the next start; `walk`), its jump tables and byte index tables after
    its code; it ends at the last of them."""
    starts = {img.entry, *img.exports()}
    seh = seh_labels(img)
    for r in img.relocs:
        v = img.dword(r)
        if img.in_text(v) and not img.in_text(r) and r not in seh:
            starts.add(v)
    labels = sorted({img.dword(r) for r in seh} - starts)

    def labels_in(s, e):
        return labels[bisect.bisect_right(labels, s):bisect.bisect_left(labels, e)]

    starts = {s for s in starts if img.in_text(s) and code.get(s)}
    # to a fixed point: a new start shortens its neighbour's range, which
    # can turn more of the neighbour's jumps into tail calls
    while True:
        order = sorted(starts)
        new = set()
        for i, s in enumerate(order):
            e = order[i + 1] if i + 1 < len(order) else img.text_hi
            _, calls, imms, tails, _ = walk(code, s, starts, e, labels_in(s, e))
            new |= {c for c in calls | imms | tails if c not in starts and img.in_text(c) and code.get(c)}
        if not new:
            break
        starts |= new
    order = sorted(starts)
    text = next(blob for _, va, blob, c in img.secs if c)
    fns = {}
    covered = bytearray(len(text))
    for s in order:
        nxt = order[bisect.bisect_right(order, s)] if bisect.bisect_right(order, s) < len(order) else img.text_hi
        pcs, _, _, _, tables = walk(code, s, starts, nxt, labels_in(s, nxt))
        ins = sorted((pc, *code.at[pc]) for pc in pcs)
        end = max(pc + n for pc, n, _ in ins)
        cap = max(nxt, end)
        for base, kind in sorted(tables):
            if not end <= base < cap:
                continue
            if kind == "dword":
                k = base
                while k + 4 <= cap and k in img.reloc_set and img.in_text(img.dword(k)):
                    k += 4
                end = max(end, k)
            else:
                k = base
                while k < cap and text[k - img.text_lo] != 0xCC:
                    k += 1
                end = max(end, k)
        for pc, n, _ in ins:
            covered[pc - img.text_lo:pc - img.text_lo + n] = b"\1" * n
        fns[s] = (end, ins)
    for s, (e, _) in fns.items():
        covered[s - img.text_lo:e - img.text_lo] = b"\1" * (e - s)
    # code no start reaches (dead, or reached through something unseen),
    # without int3 and zero padding
    gaps = []
    k = 0
    while k < len(text):
        if covered[k] or text[k] in (0xCC, 0):
            k += 1
            continue
        j = k
        while j < len(text) and not covered[j]:
            j += 1
        e = j
        while e > k and text[e - 1] in (0xCC, 0):
            e -= 1
        gaps.append((img.text_lo + k, e - k))
        k = j
    overlaps = sum(1 for a, b in zip(order, order[1:]) if fns[a][0] > b)
    return fns, {"gaps": gaps, "overlaps": overlaps}


def census_dll(path, nslots):
    img = reach.Image(path)
    code = Code(img, path)
    fns, notes = cut_dll(img, code)
    insns = sorted((pc, t) for pc, (_, t) in code.at.items())
    mod = Module(img, insns, fns, path.name, ends={s: e for s, (e, _) in fns.items()},
                 fn_insns={s: ins for s, (_, ins) in fns.items()})
    table = table_global([t for _, ins in fns.values() for _, _, t in ins])
    rows = {}
    for s, (e, b, toks, mn, texts) in mod.functions().items():
        rows[s] = (e - s, b, toks, ops_of(mn), sorted(slots_of(texts, table, nslots)) if table else [], mn)
    text = next(blob for _, va, blob, c in img.secs if c)
    return path.name, len(text), img.exports(), rows, notes


def row_changes(old_tsv, new):
    """[(dll, kind, old rows, new rows)] between an earlier census's
    overlay-dll-functions.tsv and `new` ({dll: {address: size}}). Rows are
    grouped into clusters of overlapping old and new ranges; a cluster
    whose rows are not the same is one change: `resized` (one row each,
    same address), `merged` (several old rows, one new), `split` (one old,
    several new), `recut` (several each), `dropped` (old rows only: code no
    start reaches now), `new` (new rows only)."""
    old = defaultdict(dict)
    with open(old_tsv, encoding="utf-8") as f:
        for line in f:
            if line.startswith("#"):
                continue
            c = line.split("\t")
            old[c[0]][int(c[1], 16)] = int(c[2])
    out = []
    for name in sorted(set(old) | set(new)):
        o, n = old.get(name, {}), new.get(name, {})
        same = {a for a in o if n.get(a) == o[a]}
        ev = sorted([(a, a + max(sz, 1), 0) for a, sz in o.items() if a not in same]
                    + [(a, a + max(sz, 1), 1) for a, sz in n.items() if a not in same])
        cluster, hi = [], 0
        for item in ev + [(1 << 40, 0, 0)]:
            if cluster and item[0] >= hi:
                os_ = [f"{a:#x}+{o[a]}" for a, _, k in cluster if k == 0]
                ns = [f"{a:#x}+{n[a]}" for a, _, k in cluster if k == 1]
                kind = ("dropped" if not ns else "new" if not os_ else
                        "resized" if len(os_) == len(ns) == 1 else "merged" if len(ns) == 1 else
                        "split" if len(os_) == 1 else "recut")
                out.append((name, kind, os_, ns))
                cluster, hi = [], 0
            cluster.append(item)
            hi = max(hi, item[1])
    return out


def exe_chunk(args):
    exe, lo, hi = args
    return objdump(exe, lo, hi)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--out", type=pathlib.Path, default=WORK / "m5/scope")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--old", type=pathlib.Path, default=None,
                    help="an earlier census directory: print, per DLL, the rows that changed")
    args = ap.parse_args()
    if args.old and args.old.resolve() == args.out.resolve():
        sys.exit("--old and --out are the same directory: write the new census elsewhere")
    exe = refs.default_exe()
    dlls = sorted((pathlib.Path(exe).parent / "magicFiles/FFX").glob("magic_*.dll"))

    # g_MagicImports' length, from FFX.exe (tools/reach.py)
    nslots = reach.Image(exe).magic_imports()[0]
    with ProcessPoolExecutor(args.jobs) as pool:
        results = list(pool.map(functools.partial(census_dll, nslots=nslots), dlls, chunksize=4))

    # ---- shapes then hashes, per DLL (callees are in the same DLL)
    per_dll = {}
    notes = {}
    for name, text_size, exports, rows, note in results:
        notes[name] = note
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

    # ---- dead code, overlaps, and the rows that changed since --old
    gaps = [(name, a, n) for name in sorted(notes) for a, n in notes[name]["gaps"]]
    with open(args.out / "overlay-unreached.tsv", "w", encoding="utf-8") as f:
        f.write("# dll\taddress\tbytes (code no function start reaches; not a census row)\n")
        for name, a, n in gaps:
            f.write(f"{name}\t{a:#010x}\t{n}\n")
    if args.old:
        changes = row_changes(args.old / "overlay-dll-functions.tsv",
                              {name: {s: r[0] for s, r in v[2].items()} for name, v in per_dll.items()})
        with open(args.out / "overlay-row-changes.tsv", "w", encoding="utf-8") as f:
            f.write("# dll\tchange\told rows (address+size)\tnew rows (address+size)\n")
            for name, kind, old, new in changes:
                f.write(f"{name}\t{kind}\t{' '.join(old)}\t{' '.join(new)}\n")
        by_dll = defaultdict(list)
        for c in changes:
            by_dll[c[0]].append(c)
        for name in sorted(by_dll):
            cs = by_dll[name]
            kinds = Counter(c[1] for c in cs)
            print(f"{name}: " + ", ".join(f"{k} {kinds[k]}" for k in sorted(kinds)) + ": " + "; ".join(
                f"{k} {' '.join(o) or '-'} -> {' '.join(n) or '-'}" for _, k, o, n in cs))
        kinds = Counter(c[1] for c in changes)
        print(f"rows changed against {args.old}: {len(by_dll)} DLLs; " +
              ", ".join(f"{k} {kinds[k]}" for k in sorted(kinds)))

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
