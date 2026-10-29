#!/usr/bin/env python3
"""Static reachability of FFX.exe's functions (docs/rewrite-plan.md, M5
stage 1): which functions the 64-bit program still needs, which of those
are not ported yet, and which can be dropped.

  FFX_GAME_DIR=... tools/reach.py [--out DIR] [--lists DIR] [--dropped FILE]
                                  [--no-cache] [--jobs N] [--why ADDR|NAME]...

Nodes are the dump's functions ($FFX_WORK/decomp/functions.tsv), the
function lines of symbols/ffx.sym (some are hidden from the dump), and every
call target or data-pointer target that lies outside every dump body (code
Ghidra never made a function of). A node covers its bytes up to the next
node's start, so code past a dump body (Ghidra's label chunks, jump tables)
belongs to the function before it.

Edges: the direct calls, jumps and conditional jumps of a linear
disassembly of .text (objdump), each kept only when no base relocation lies
inside the instruction (a decoded jump table or address is not a branch),
and every function start checked to be an instruction boundary (the range
is disassembled again from the start when it is not).

Roots, in this order (each kind's count is what it adds to the ones before):
  entry       the PE entry point
  export      the export directory's functions
  tls         TLS callbacks (FFX.exe has no TLS directory: 0)
  safeseh     the load configuration's SafeSEH handler table
  crt-init    the __xi/__xc initialiser tables the CRT start-up hands to
              _initterm/_initterm_e (push offset end, push offset start)
  c++eh       C++ EH FuncInfo (magic 0x1993052x): unwind actions and catch
              handlers
  seh         __SEH_prolog4 scope tables: filters and handlers
  vtable      pointer runs in .rdata after an RTTI complete-object-locator
              pointer
  native      the Atel native tables (AtelSetUpCallFunc's tables, 16-byte
              records)
  magic       g_MagicImports (0x00c64ce8, 744 slots), the table every
              magic_*.dll gets; the overlays reach FFX.exe only through it
  rdata/data  any other relocated pointer into .text in a data section
              (callback tables, function pointers in .data)
  code-imm    a relocated immediate in .text pointing into another node
              (push offset f, mov [x], offset f): an address taken in code

FFX.exe has a base relocation table, so the data and code pointers are
exact. Everything is conservative: a pointer into the middle of a function
reaches the whole function. `strict` in the report counts what is left when
an address taken in code only counts when the code taking it is reachable,
and an EH funclet only when the function installing its __ehhandler stub is
(push offset stub; the stub's mov eax, offset FuncInfo).

Outputs (default $FFX_WORK/m5/reach and $FFX_WORK/m5/port):
  report.md                counts per module and library, root kinds, the
                           library functions game code reaches, sanity checks
  reachable-unported.tsv   reachable functions no Rust replaces yet
  nodes.tsv                every node with its verdict and why
  <module>[-a..].list      leaf-first work lists of the reachable-unported
                           game functions (callees before callers, 300-500
                           each); stubs.list and m4-not-portable.list apart
  symbols/m5-dropped.tsv   (--dropped) unreachable game functions with the
                           evidence (refs.py's raw refs and the graph's)

The disassembly is cached in $FFX_WORK/cache/reach-<hash of FFX.exe>.pickle.
"""
import argparse
import bisect
import hashlib
import math
import os
import pathlib
import pickle
import re
import struct
import subprocess
import sys
import tempfile
import time
from array import array
from collections import Counter, defaultdict, deque
from concurrent.futures import ProcessPoolExecutor

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import disasm_cache  # noqa: E402
import progress  # noqa: E402
import refs  # noqa: E402

WORK = progress.WORK
REPO = progress.REPO
VERSION = 1
MAGIC_IMPORTS, MAGIC_SLOTS = 0x00C64CE8, 744
# from the first __xc_a initialiser to the end of .text: only the compiler's
# dynamic initialisers of globals (game and library alike) and the atexit
# destructors they register (unnamed but for FFXApplication_atexit_dtor; all
# reached through the crt-init tables or from the initialisers), a module of
# their own ("init" kind) instead of the neighbours' guess
STATIC_INIT = 0x00AE03B0
KIND = dict(progress.KIND, **{"static-init": "init", "import": "library"})
ROOT_ORDER = ("entry", "export", "tls", "safeseh", "crt-init", "c++eh", "seh", "vtable", "native", "magic",
              "rdata", "data", "code-imm")
EH_MAGIC = (0x19930520, 0x19930521, 0x19930522)
# alignment padding MSVC puts between functions (besides int3)
PADDING = {bytes.fromhex(h) for h in ("8bff", "8d4900", "8d642400", "8da42400000000", "8d9b00000000",
                                       "8d4000", "8d7600", "8dbf00000000", "0f1f00", "0f1f4000",
                                       "0f1f440000", "0f1f8000000000", "0f1f840000000000")}
# instructions compiled code never has: decoded data
DATA_OPS = {"daa", "das", "aaa", "aas", "aam", "aad", "bswap", "in", "out", "les", "lds", "into",
            "bound", "arpl", "es", "ss", "cs", "ds", "fs", "gs", "adc", "sbb", "salc", "lahf", "sahf"}
INSN = re.compile(r"^\s*([0-9a-f]+):\t(.*)$")
DIRECT = re.compile(r"^(?:(?:bnd|notrack)\s+)?(call|jmp|j[a-z]+|loop[a-z]*)\s+0x([0-9a-f]+)$")
INDIRECT = re.compile(r"^(?:(?:bnd|notrack)\s+)?(call|jmp)\s+DWORD PTR ds:0x([0-9a-f]+)$")


# ---------------------------------------------------------------- the image

class Image:
    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.data = data = self.path.read_bytes()
        self.hash = hashlib.sha256(data).hexdigest()[:16]
        self.base, self.secs, relocs = refs.read_pe(data)
        self.relocs = sorted(relocs)
        self.reloc_set = set(relocs)
        pe = struct.unpack_from("<I", data, 0x3c)[0]
        opt = pe + 24
        self.entry = self.base + struct.unpack_from("<I", data, opt + 16)[0]
        ndirs = struct.unpack_from("<I", data, opt + 92)[0]
        self.dirs = [struct.unpack_from("<II", data, opt + 96 + 8 * i) for i in range(ndirs)]
        text = [(va, blob) for _, va, blob, code in self.secs if code]
        self.text_lo, tb = text[0]
        self.text_hi = self.text_lo + len(tb)

    def section(self, va):
        for name, sva, blob, code in self.secs:
            if sva <= va < sva + len(blob):
                return name
        return None

    def dword(self, va):
        for _, sva, blob, _ in self.secs:
            if sva <= va <= sva + len(blob) - 4:
                return struct.unpack_from("<I", blob, va - sva)[0]
        return None

    def cstr(self, va):
        for _, sva, blob, _ in self.secs:
            if sva <= va < sva + len(blob):
                o = va - sva
                return blob[o:blob.index(b"\0", o)].decode("latin-1")
        return ""

    def in_text(self, v):
        return v is not None and self.text_lo <= v < self.text_hi

    def imports(self):
        """IAT address -> 'dll!name'."""
        rva, _ = self.dirs[1]
        out = {}
        d = self.base + rva
        while True:
            ilt, _, _, name, iat = (self.dword(d + 4 * i) for i in range(5))
            if not (ilt or iat):
                break
            dll = self.cstr(self.base + name).lower()
            k = 0
            while True:
                t = self.dword(self.base + (ilt or iat) + 4 * k)
                if not t:
                    break
                fn = f"#{t & 0xffff}" if t & 0x80000000 else self.cstr(self.base + t + 2)
                out[self.base + iat + 4 * k] = f"{dll}!{fn}"
                k += 1
            d += 20
        return out

    def exports(self):
        rva, size = self.dirs[0]
        if not size:
            return []
        d = self.base + rva
        n, funcs = self.dword(d + 20), self.dword(d + 28)
        return [self.base + self.dword(self.base + funcs + 4 * i) for i in range(n)]

    def tls_callbacks(self):
        rva, size = self.dirs[9] if len(self.dirs) > 9 else (0, 0)
        if not size:
            return []
        cb = self.dword(self.base + rva + 12)
        out = []
        while cb and self.dword(cb):
            out.append(self.dword(cb))
            cb += 4
        return out

    def safeseh(self):
        rva, size = self.dirs[10] if len(self.dirs) > 10 else (0, 0)
        if not size:
            return []
        lc = self.base + rva
        if self.dword(lc) < 0x48:
            return []
        table, count = self.dword(lc + 0x40), self.dword(lc + 0x44)
        return [self.base + self.dword(table + 4 * i) for i in range(count)]


# ---------------------------------------------------------- the disassembly

def disasm_range(args):
    """(instruction starts, [(pc, end, op, target)], [(pc, op, iat)]) of
    objdump's linear disassembly of [a, b)."""
    exe, a, b = args
    out = subprocess.run(["objdump", "-d", "--no-show-raw-insn", "-M", "intel", f"--start-address={a:#x}",
                          f"--stop-address={b:#x}", str(exe)], capture_output=True, text=True,
                         check=True).stdout
    starts = array("I")
    branches, indirect = [], []
    pending = None  # a branch waiting for the next instruction's address (its end)
    for line in out.splitlines():
        m = INSN.match(line)
        if not m:
            continue
        pc = int(m.group(1), 16)
        starts.append(pc)
        if pending:
            branches.append((pending[0], pc, pending[1], pending[2]))
            pending = None
        text = m.group(2).split("#")[0].strip()
        if not text or text[0] not in "cjlbn":
            continue
        d = DIRECT.match(text)
        if d:
            pending = (pc, d.group(1), int(d.group(2), 16))
            continue
        d = INDIRECT.match(text)
        if d:
            indirect.append((pc, d.group(1), int(d.group(2), 16)))
    if pending:
        branches.append((pending[0], b, pending[1], pending[2]))
    return starts, branches, indirect


class Disasm:
    def __init__(self, img, node_starts, jobs, use_cache):
        path = WORK / "cache" / f"reach-{img.hash}-v{VERSION}.pickle"
        cached = None
        if use_cache:
            try:
                with open(path, "rb") as f:
                    cached = pickle.load(f)
            except (OSError, EOFError, pickle.UnpicklingError):
                cached = None
        if cached and cached["starts_key"] == len(node_starts) and not cached.get("partial"):
            self.starts, self.branches, self.indirect, self.redone = (
                cached["starts"], cached["branches"], cached["indirect"], cached["redone"])
            return
        t0 = time.time()
        exe = img.path
        inside = [a for a in node_starts if img.text_lo <= a < img.text_hi]
        # chunks of about 64 KB, split at function starts
        cuts = [img.text_lo]
        for a in inside:
            if a - cuts[-1] >= 0x10000:
                cuts.append(a)
        cuts.append(img.text_hi)
        ranges = [(exe, cuts[i], cuts[i + 1]) for i in range(len(cuts) - 1)]
        starts = array("I")
        branches, indirect = [], []
        with ProcessPoolExecutor(max_workers=jobs) as pool:
            for s, br, ind in pool.map(disasm_range, ranges, chunksize=4):
                starts.extend(s)
                branches.extend(br)
                indirect.extend(ind)
        self.starts, self.branches, self.indirect = starts, branches, indirect
        self.redone = []
        self.ensure(img, inside, jobs)
        print(f"reach: disassembled .text in {time.time() - t0:.0f} s ({len(self.starts)} instructions, "
              f"{len(self.redone)} ranges again from their start)", file=sys.stderr)
        if use_cache:
            path.parent.mkdir(parents=True, exist_ok=True)
            fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".")
            with os.fdopen(fd, "wb") as f:
                pickle.dump({"starts_key": len(node_starts), "starts": self.starts, "branches": self.branches,
                             "indirect": self.indirect, "redone": self.redone}, f,
                            protocol=pickle.HIGHEST_PROTOCOL)
            os.replace(tmp, path)

    def ensure(self, img, starts, jobs):
        """Disassembles again from each of `starts` (sorted) that linear
        disassembly ran past, up to the next one, replacing what it had."""
        have = set(self.starts)
        missing = [(a, starts[i + 1] if i + 1 < len(starts) else img.text_hi)
                   for i, a in enumerate(starts) if a not in have]
        missing = [(a, b) for a, b in missing if b > a]
        if missing:
            exe = img.path
            with ProcessPoolExecutor(max_workers=jobs) as pool:
                fixes = list(pool.map(disasm_range, [(exe, a, b) for a, b in missing], chunksize=16))
            keys = [a for a, _ in missing]

            def dropped(pc):
                i = bisect.bisect_right(keys, pc) - 1
                return i >= 0 and pc < missing[i][1]

            st = array("I", (p for p in self.starts if not dropped(p)))
            self.branches = [r for r in self.branches if not dropped(r[0])]
            self.indirect = [r for r in self.indirect if not dropped(r[0])]
            for s, br, ind in fixes:
                st.extend(s)
                self.branches.extend(br)
                self.indirect.extend(ind)
            self.starts = st
            self.redone = sorted(set(self.redone) | set(keys))
        self.starts = array("I", sorted(set(self.starts)))
        return len(missing)


# ------------------------------------------------------------------- graph

class Graph:
    def __init__(self, img, jobs, use_cache):
        self.img = img
        rows = progress.functions()
        names = progress.symbols()
        self.sym = names
        dump = {a: (n, s) for a, n, s in rows}
        for a in names:
            if a not in dump and img.in_text(a):
                dump[a] = (names[a], None)
        starts = sorted(dump)
        # call targets and data-pointer targets outside every dump body
        # become nodes of their own; a first disassembly finds the calls
        dis = Disasm(img, starts, jobs, use_cache)

        def outside(t):
            i = bisect.bisect_right(starts, t) - 1
            if i < 0:
                return True
            a = starts[i]
            return a != t and dump[a][1] is not None and t >= a + max(dump[a][1], 1)

        reloc_bad = img.reloc_set

        def clean(pc, end):
            return not any(p in reloc_bad for p in range(pc - 3, end))

        extra = set()
        for pc, end, op, t in dis.branches:
            if op == "call" and img.in_text(t) and t not in dump and outside(t) and clean(pc, end):
                extra.add(t)
        for r in img.relocs:
            sec = img.section(r)
            if sec and sec != ".text" and sec != ".reloc":
                v = img.dword(r)
                if img.in_text(v) and v not in dump and outside(v):
                    extra.add(v)
        # the image's own roots (the SafeSEH handlers are __ehhandler stubs
        # Ghidra left inside the Unwind@ funclets)
        for v in [img.entry, *img.exports(), *img.tls_callbacks(), *img.safeseh()]:
            if img.in_text(v) and v not in dump:
                extra.add(v)
        for a in extra:
            dump[a] = ("", None)
        self.extra = extra
        self.starts = sorted(dump)
        # an extra start inside a range decoded out of step: from the start
        dis.ensure(img, self.starts, jobs)
        self.dis = dis
        seh = set(img.safeseh())
        self.name = {a: names.get(a) or dump[a][0] or (f"ehhandler_{a:08x}" if a in seh else f"code_{a:08x}")
                     for a in self.starts}
        self.ghidra = {a: dump[a][0] for a in self.starts}
        fix = {a: s for a, _, s in rows}
        self.size = {}
        for i, a in enumerate(self.starts):
            nxt = self.starts[i + 1] if i + 1 < len(self.starts) else img.text_hi
            self.size[a] = fix.get(a) or (nxt - a)
        self.span = {a: (self.starts[i + 1] if i + 1 < len(self.starts) else img.text_hi) - a
                     for i, a in enumerate(self.starts)}
        prow = [(a, self.ghidra[a] or "", self.size[a]) for a in self.starts]
        self.module = progress.placement(prow, names)
        self.iat = img.imports()
        # import thunks (a node that is one `jmp [IAT]`): module "import"
        self.thunk = {}
        for pc, op, slot in dis.indirect:
            if op == "jmp" and slot in self.iat:
                self.thunk[pc] = self.iat[slot]
        for a in self.starts:
            if a >= STATIC_INIT:
                self.module[a] = "static-init"
            elif a in self.thunk:
                self.module[a] = "import"
                if self.name[a].startswith("code_"):
                    fn = self.thunk[a].split("!")[1]
                    m = re.match(r"\?(\w+)@(\w+)@(\w+)@@", fn)  # ?method@Class@NS@@... -> NS_Class_method
                    self.name[a] = "imp_" + (f"{m[3]}_{m[2]}_{m[1]}" if m else fn)

        node_of = self.node_of
        self.out = defaultdict(set)      # caller -> callees
        self.inn = defaultdict(set)      # callee -> callers
        self.edge_kind = {}
        self.imports_of = defaultdict(set)
        decoded = set()
        for pc, end, op, t in dis.branches:
            if not img.in_text(t) or not clean(pc, end):
                continue
            decoded.add(pc)
            s, d = node_of(pc), node_of(t)
            if s is None or d is None or s == d:
                continue
            self.out[s].add(d)
            self.inn[d].add(s)
            k = "call" if op == "call" else "jmp"
            if self.edge_kind.get((s, d)) != "call":
                self.edge_kind[(s, d)] = k
        self.decoded = decoded
        # fall-through: code that runs on into the next node (Ghidra split a
        # function at a label, or a call that never returns comes last)
        self.text = next(blob for _, va, blob, code in img.secs if code)
        self.fall = {}
        for b in self.starts[1:]:
            a = self.falls_into(b)
            if a is not None:
                a = self.confirm_fall(b)
            if a is not None:
                s = node_of(a)
                self.out[s].add(b)
                self.inn[b].add(s)
                self.edge_kind.setdefault((s, b), "fall")
                self.fall[b] = s
        for pc, op, slot in dis.indirect:
            if slot in self.iat:
                s = node_of(pc)
                if s is not None:
                    self.imports_of[s].add(self.iat[slot])

    def falls_into(self, b):
        """The address of the instruction that runs on into `b`, or None when
        the code before `b` ends in a ret, jmp, int3, ud2 or hlt, or is data
        (a relocated jump table), skipping alignment padding (nop forms)."""
        starts = self.dis.starts
        lo = self.img.text_lo
        i = bisect.bisect_left(starts, b) - 1
        end = b
        for _ in range(16):
            if i < 0:
                return None
            p = starts[i]
            if end - p > 15:
                return None  # the bytes before b are no instruction there
            if any(r in self.img.reloc_set for r in range(p - 3, end)):
                return None
            code = self.text[p - lo:end - lo]
            op = code[0]
            while op in (0x66, 0xf2, 0xf3, 0x2e, 0x3e) and len(code) > 1:
                code = code[1:]
                op = code[0]
            if op in (0x90, 0xcc) or code in PADDING:
                if op == 0xcc:
                    return None
                end = p
                i -= 1
                continue
            if op in (0xc3, 0xc2, 0xcb, 0xca, 0xe9, 0xeb, 0xf4) or code[:2] == b"\x0f\x0b":
                return None
            if op == 0xff and len(code) > 1 and (code[1] >> 3) & 7 in (4, 5):
                return None
            return p
        return None

    def confirm_fall(self, b):
        """falls_into's candidate checked by disassembling the node before `b`
        from its start: `b` must be an instruction boundary there, and the
        instruction before it no terminator or padding."""
        s = self.node_of(b - 1)
        if s is None or b - s > 0x20000:
            return None
        code = disasm_cache.objdump(self.img.path, s, b + 16 - s)
        pcs = sorted(code)
        if b not in code:
            return None
        i = pcs.index(b) - 1
        while i >= 0:
            text = code[pcs[i]]
            op = text.split()[0] if text else ""
            if op in ("int3", "nop", "xchg") or text.startswith(("lea    esp,[esp", "lea    ecx,[ecx", "mov    edi,edi",
                                                                   "lea    ebx,[ebx", "lea    esi,[esi")):
                if op == "int3":
                    return None
                i -= 1
                continue
            if op in ("ret", "jmp", "hlt", "ud2", "(bad)") or text.startswith((".byte", "repz ret", "bnd jmp")):
                return None
            # bytes that are no code: int3 padding swallowed by the decoder,
            # switch index tables decoded as rare instructions
            if self.text[b - 1 - self.img.text_lo] == 0xcc or op in DATA_OPS or \
                    re.match(r"(push|pop)\s+[c-gs]s$", text):
                return None
            return pcs[i]
        return None

    def node_of(self, a):
        i = bisect.bisect_right(self.starts, a) - 1
        if i < 0 or not self.img.text_lo <= a < self.img.text_hi:
            return None
        return self.starts[i]


# ------------------------------------------------------------------- roots

def find_roots(g, by_name):
    """[(kind, node, site, detail)]."""
    img = g.img
    roots = []
    roots.append(("entry", g.node_of(img.entry), img.entry, "PE entry point"))
    for a in img.exports():
        if img.in_text(a):
            roots.append(("export", g.node_of(a), a, "export directory"))
    for a in img.tls_callbacks():
        roots.append(("tls", g.node_of(a), a, "TLS callback"))
    for a in img.safeseh():
        if img.in_text(a):
            roots.append(("safeseh", g.node_of(a), a, "SafeSEH handler table"))

    # classify data sites: {site: kind}
    site_kind = {}
    # g_MagicImports
    for i in range(MAGIC_SLOTS):
        site_kind[MAGIC_IMPORTS + 4 * i] = "magic"
    # code relocs, sorted, for "push offset X" patterns
    code_relocs = [r for r in img.relocs if img.text_lo <= r < img.text_hi]
    branches_by_pc = sorted(g.dis.branches)
    bpcs = [b[0] for b in branches_by_pc]
    ind_by_pc = sorted(g.dis.indirect)
    ipcs = [b[0] for b in ind_by_pc]

    def next_branch(site, within):
        i = bisect.bisect_right(bpcs, site)
        if i < len(bpcs) and bpcs[i] - site <= within:
            return branches_by_pc[i]
        return None

    def next_indirect(site, within):
        i = bisect.bisect_right(ipcs, site)
        if i < len(ipcs) and ipcs[i] - site <= within:
            return ind_by_pc[i]
        return None

    # native tables: the pushed table of each AtelSetUpCallFunc call, plus
    # its default table (the immediate inside AtelSetUpCallFunc)
    native_starts = set()
    setup = by_name.get("AtelSetUpCallFunc")
    if setup:
        for r in code_relocs:
            v = img.dword(r)
            if img.section(v) in (".rdata", ".data"):
                b = next_branch(r, 16)
                if (b and b[2] == "call" and b[3] == setup) or g.node_of(r) == setup:
                    native_starts.add(v)
    for t in sorted(native_starts):
        a = t
        while True:
            words = [img.dword(a + 4 * k) for k in range(4)]
            if any(w is None for w in words) or not any(img.in_text(w) for w in words):
                break
            if any(w and not img.in_text(w) for w in words):
                break
            for k in range(4):
                site_kind.setdefault(a + 4 * k, "native")
            a += 16
    # CRT initialiser tables: two pushed .rdata/.data addresses before a call
    # of _initterm/_initterm_e (imported or local)
    initterm_iat = {s for s, n in g.iat.items() if n.split("!")[1] in ("_initterm", "_initterm_e")}
    initterm_fn = {a for a, n in g.name.items() if n.lstrip("_") in ("initterm", "initterm_e")}
    initterm_fn |= {a for a, n in g.thunk.items() if n.split("!")[1] in ("_initterm", "_initterm_e")}
    crt_tables = []
    for i, r in enumerate(code_relocs[:-1]):
        r2 = code_relocs[i + 1]
        if r2 - r > 8:
            continue
        lo, hi = img.dword(r2), img.dword(r)  # push end; push start
        if img.section(lo) not in (".rdata", ".data") or not lo < hi <= lo + 0x1000:
            continue
        ind = next_indirect(r2, 12)
        br = next_branch(r2, 12)
        if (ind and ind[2] in initterm_iat) or (br and br[2] == "call" and br[3] in initterm_fn):
            crt_tables.append((lo, hi))
            for a in range(lo, hi, 4):
                site_kind[a] = "crt-init"
    # C++ EH FuncInfo; eh_sites: FuncInfo -> its funclet pointer sites
    eh_sites = defaultdict(list)
    for name, sva, blob, code in img.secs:
        if code or name in (".reloc", ".rsrc"):
            continue
        for m in re.finditer(rb"(?=[\x20\x21\x22]\x05\x93\x19)", blob):
            fi = sva + m.start()
            if fi % 4:
                continue
            max_state, unwind, ntry, trymap = (img.dword(fi + 4 * k) for k in (1, 2, 3, 4))
            if max_state is None or max_state > 0x10000:
                continue
            if unwind and max_state:
                for s in range(max_state):
                    site_kind[unwind + 8 * s + 4] = "c++eh"
                    eh_sites[fi].append(unwind + 8 * s + 4)
            if trymap and ntry and ntry < 0x1000:
                for t in range(ntry):
                    nc, harr = img.dword(trymap + 20 * t + 12), img.dword(trymap + 20 * t + 16)
                    if nc and harr and nc < 0x100:
                        for c in range(nc):
                            site_kind[harr + 16 * c + 12] = "c++eh"
                            eh_sites[fi].append(harr + 16 * c + 12)
    # __SEH_prolog4 / _except_handler4 scope tables
    seh_fns = {a for a, n in g.name.items() if re.search(r"SEH_prolog4|except_handler[34]", n)}
    for r in code_relocs:
        v = img.dword(r)
        if img.section(v) != ".rdata":
            continue
        br = next_branch(r, 12)
        nxt_reloc_val = None
        j = bisect.bisect_right(code_relocs, r)
        if j < len(code_relocs) and code_relocs[j] - r <= 6:
            nxt_reloc_val = img.dword(code_relocs[j])
        if not ((br and br[2] == "call" and br[3] in seh_fns) or (nxt_reloc_val in seh_fns)):
            continue
        rec = v + 16
        for _ in range(256):
            enc, flt, hnd = (img.dword(rec + 4 * k) for k in range(3))
            if enc is None or not (enc == 0xfffffffe or enc < 256):
                break
            if not img.in_text(hnd) or not (flt == 0 or img.in_text(flt)):
                break
            site_kind.setdefault(rec + 4, "seh")
            site_kind.setdefault(rec + 8, "seh")
            rec += 12

    data_relocs = [r for r in img.relocs if img.section(r) not in (".text", ".reloc", None)]
    data_set = set(data_relocs)
    # vtables: a run of .text pointers after an RTTI complete-object-locator
    # pointer (a relocated .rdata word whose target starts with signature 0)
    for r in data_relocs:
        if r in site_kind or img.section(r) != ".rdata" or not img.in_text(img.dword(r)):
            continue
        prev = r - 4
        if prev in data_set and img.in_text(img.dword(prev)):
            continue  # not the run's first word
        col = img.dword(prev) if prev in data_set else None
        if col is None or img.section(col) != ".rdata" or img.dword(col) != 0:
            continue
        a = r
        while a in data_set and img.in_text(img.dword(a)) and a not in site_kind:
            site_kind[a] = "vtable"
            a += 4

    for r in data_relocs:
        v = img.dword(r)
        if not img.in_text(v):
            continue
        sec = img.section(r)
        kind = site_kind.get(r) or ("rdata" if sec in (".rdata", ".rodata", "_RDATA") else "data")
        roots.append((kind, g.node_of(v), r, f"{sec} {r:#010x} -> {v:#010x}"))
    # addresses taken in code
    code_imm = []
    for r in code_relocs:
        v = img.dword(r)
        if not img.in_text(v):
            continue
        s, d = g.node_of(r), g.node_of(v)
        if s != d:
            roots.append(("code-imm", d, r, f"immediate at {r:#010x} in {g.name[s]}"))
            code_imm.append((s, d))
    # strict: a funclet is reached from the __ehhandler stub that loads its
    # FuncInfo (mov eax, offset FuncInfo; jmp __CxxFrameHandler3), and the
    # stub from the function that installs it (push offset stub)
    eh_edges = defaultdict(set)
    for r in code_relocs:
        fi = img.dword(r)
        if fi in eh_sites:
            for site in eh_sites[fi]:
                v = img.dword(site)
                if img.in_text(v):
                    eh_edges[g.node_of(r)].add(g.node_of(v))
    return roots, code_imm, eh_edges, crt_tables, len(native_starts)


def bfs(g, roots, order, extra_edges=None):
    """node -> (kind, parent node or None, root site); and per-kind counts."""
    seen = {}
    added = {}
    by_kind = defaultdict(list)
    for kind, node, site, _ in roots:
        if node is not None:
            by_kind[kind].append((node, site))
    for kind in order:
        q = deque()
        n0 = len(seen)
        for node, site in by_kind.get(kind, ()):
            if node not in seen:
                seen[node] = (kind, None, site)
                q.append(node)
        while q:
            v = q.popleft()
            for w in sorted(g.out.get(v, ())) + sorted((extra_edges or {}).get(v, ())):
                if w not in seen:
                    seen[w] = (seen[v][0], v, None)
                    q.append(w)
        added[kind] = len(seen) - n0
    return seen, added


def chain(g, seen, a, limit=6):
    names = []
    v = a
    while v is not None:
        names.append(g.name[v])
        _, parent, site = seen[v]
        if parent is None:
            break
        v = parent
    names.reverse()
    root_kind, _, site = seen[v]
    if len(names) > limit:
        names = names[:2] + [f"...{len(names) - limit + 2}..."] + names[-(limit - 3):]
    head = f"{root_kind}@{site:#010x}" if site is not None else root_kind
    return f"{head}: " + " > ".join(names)


def tarjan_order(nodes, edges):
    """Strongly connected components in callee-first order (Tarjan emits a
    component after every component it reaches)."""
    index, low, on = {}, {}, set()
    stack, comps = [], []
    counter = 0
    for root in nodes:
        if root in index:
            continue
        work = [(root, iter(sorted(edges.get(root, ()))))]
        index[root] = low[root] = counter
        counter += 1
        stack.append(root)
        on.add(root)
        while work:
            v, it = work[-1]
            for w in it:
                if w not in index:
                    index[w] = low[w] = counter
                    counter += 1
                    stack.append(w)
                    on.add(w)
                    work.append((w, iter(sorted(edges.get(w, ())))))
                    break
                if w in on:
                    low[v] = min(low[v], index[w])
            else:
                work.pop()
                if work:
                    low[work[-1][0]] = min(low[work[-1][0]], low[v])
                if low[v] == index[v]:
                    comp = []
                    while True:
                        w = stack.pop()
                        on.discard(w)
                        comp.append(w)
                        if w == v:
                            break
                    comps.append(sorted(comp))
    return comps


def not_portable():
    out = {}
    p = REPO / "symbols/m4-not-portable.tsv"
    for line in p.read_text(encoding="utf-8").splitlines()[1:]:
        if line and not line.startswith("#"):
            f = line.split("\t")
            out[int(f[0], 16)] = f[2] if len(f) > 2 else ""
    return out


def coverage_hits():
    """{file: set of addresses} of the M4 and earlier coverage runs."""
    files = sorted((WORK / "m4").rglob("*.hits")) + sorted((WORK / "agents/coverage").rglob("*.hits"))
    files += [WORK / "agents/coverage/coverage.txt"]
    out = {}
    for f in files:
        if not f.is_file():
            continue
        s = set()
        for line in f.read_text(encoding="utf-8", errors="replace").split():
            try:
                s.add(int(line, 16))
            except ValueError:
                pass
        out[f] = s
    return out


# ------------------------------------------------------------------ report

def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--out", type=pathlib.Path, default=WORK / "m5/reach")
    ap.add_argument("--lists", type=pathlib.Path, default=WORK / "m5/port")
    ap.add_argument("--dropped", type=pathlib.Path, default=REPO / "symbols/m5-dropped.tsv")
    ap.add_argument("--no-cache", action="store_true")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--why", action="append", default=[], metavar="ADDR|NAME",
                    help="print the verdict of these functions and stop")
    args = ap.parse_args()
    t0 = time.time()
    img = Image(refs.default_exe())
    g = Graph(img, args.jobs, not args.no_cache)
    by_name = {n: a for a, n in g.name.items()}
    roots, code_imm, eh_edges, crt_tables, n_native = find_roots(g, by_name)
    seen, added = bfs(g, roots, ROOT_ORDER)
    # strict: an address taken in code counts only when that code is reachable
    imm_edges = defaultdict(set)
    for s, d in code_imm:
        imm_edges[s].add(d)
    for s, ds in eh_edges.items():
        imm_edges[s] |= ds
    loose = ("code-imm", "safeseh", "c++eh")
    strict, _ = bfs(g, [r for r in roots if r[0] not in loose], [k for k in ROOT_ORDER if k not in loose],
                    imm_edges)

    if args.why:
        for text in args.why:
            a = int(text, 16) if re.fullmatch(r"(0x)?[0-9a-fA-F]+", text) else by_name.get(text)
            n = g.node_of(a) if a is not None else None
            if n is None:
                print(f"{text}: not a function")
                continue
            v = chain(g, seen, n, 40) if n in seen else "UNREACHABLE"
            print(f"{n:#010x} {g.name[n]} [{g.module[n]}]: {v}")
            print(f"  callers: {', '.join(g.name[c] for c in sorted(g.inn.get(n, ())))[:2000]}")
        return

    hooked = progress.hooks()
    category = progress.classes()
    np_ = not_portable()
    kind_of = KIND
    nodes = g.starts

    def state(a):
        return hooked.get(a)  # replaced | wrapped | None

    # ---- counts per module
    stats = defaultdict(lambda: defaultdict(int))
    for a in nodes:
        m = g.module[a]
        s = stats[m]
        s["functions"] += 1
        s["bytes"] += g.size[a]
        r = a in seen
        h = state(a) == "replaced"
        if a in g.extra:
            s["extra"] += 1
        if r and h:
            s["reach_hooked"] += 1
        elif r:
            s["reach_unhooked"] += 1
            s["reach_unhooked_bytes"] += g.size[a]
            if state(a) == "wrapped":
                s["reach_wrapped"] += 1
        else:
            s["unreach"] += 1
            s["unreach_bytes"] += g.size[a]
            if h:
                s["unreach_hooked"] += 1
        if a in strict:
            s["strict"] += 1

    # ---- sanity: hooked and covered functions must be reachable
    hooked_miss = sorted(a for a, st in hooked.items() if g.node_of(a) is not None and g.node_of(a) not in seen)
    hits = coverage_hits()
    cov_nodes = defaultdict(set)
    for f, s in hits.items():
        for h in s:
            n = g.node_of(h)
            if n is not None:
                cov_nodes[n].add(f.name)
    cov_miss = sorted(n for n in cov_nodes if n not in seen)

    # ---- library functions game code reaches
    lib_boundary = defaultdict(lambda: defaultdict(set))  # lib -> node -> game callers
    for a in nodes:
        if a not in seen or kind_of[g.module[a]] != "game":
            continue
        for d in g.out.get(a, ()):
            if kind_of[g.module[d]] == "library":
                lib_boundary[g.module[d]][d].add(a)
    for s, d in code_imm:
        if s in seen and kind_of[g.module[s]] == "game" and kind_of[g.module[d]] == "library":
            lib_boundary[g.module[d]][d].add(s)
    game_imports = defaultdict(set)  # import -> game callers (reachable)
    for a, imps in g.imports_of.items():
        if a in seen and kind_of[g.module[a]] == "game":
            for i in imps:
                game_imports[i].add(a)
    # thunks: a node whose only instruction is jmp [IAT]
    thunk_import = {a: imp for a, imp in g.thunk.items() if g.node_of(a) == a}

    # ---- reachable-unported
    unported = [a for a in nodes if a in seen and state(a) != "replaced"]
    args.out.mkdir(parents=True, exist_ok=True)

    def callers_text(a):
        return len(g.inn.get(a, ()))

    with open(args.out / "reachable-unported.tsv", "w", encoding="utf-8") as f:
        f.write("# address\tname\tmodule\tsize\tcallers\treached-from\tkind\tcategory\thook\tnote\n")
        for a in unported:
            m = g.module[a]
            note = []
            if a in np_:
                note.append("m4-not-portable")
            if a in g.extra:
                note.append("not in the dump")
            if a in thunk_import:
                note.append(f"thunk {thunk_import[a]}")
            if a not in strict:
                note.append("only an address taken in unreachable code")
            f.write(f"0x{a:08x}\t{g.name[a]}\t{m}\t{g.size[a]}\t{callers_text(a)}\t{chain(g, seen, a)}\t"
                    f"{kind_of[m]}\t{category.get(a, '')}\t{state(a) or ''}\t{'; '.join(note)}\n")
    with open(args.out / "nodes.tsv", "w", encoding="utf-8") as f:
        f.write("# address\tname\tmodule\tsize\treachable\tstrict\thook\tcategory\tcallers\tcovered\twhy\n")
        for a in nodes:
            f.write(f"0x{a:08x}\t{g.name[a]}\t{g.module[a]}\t{g.size[a]}\t{int(a in seen)}\t{int(a in strict)}\t"
                    f"{state(a) or ''}\t{category.get(a, '')}\t{callers_text(a)}\t{len(cov_nodes.get(a, ()))}\t"
                    f"{chain(g, seen, a) if a in seen else ''}\n")

    # ---- dropped: unreachable game functions with evidence
    idx = refs.index(img.path)
    dropped = []
    for a in nodes:
        if a in seen or kind_of[g.module[a]] != "game":
            continue
        raw = idx.to(a)
        callers = sorted(g.inn.get(a, ()))
        ev = []
        if not raw and not callers:
            ev.append("tools/refs.py: 0 refs")
        else:
            if callers:
                ev.append("callers only unreachable: " + ", ".join(
                    f"{g.name[c]} ({g.edge_kind.get((c, a), 'jmp')})" for c in callers[:6])
                    + (f" and {len(callers) - 6} more" if len(callers) > 6 else ""))
            noise = [s for s, k in raw if k in ("call", "jmp", "jcc") and s not in g.decoded]
            if noise:
                ev.append(f"{len(noise)} raw rel32 hit(s) that are no instruction "
                          f"({', '.join(f'{s:#x}' for s in noise[:3])})")
            if not callers and not noise:
                ev.append("tools/refs.py: only refs from its own body")
        ev.append("address never taken, in no table")
        if state(a) == "replaced":
            ev.append("ported anyway (no coverage run saw it)" if a not in cov_nodes else "ported; SEEN IN COVERAGE")
        dropped.append((a, g.name[a], g.module[a], g.size[a], "; ".join(ev), category.get(a, "")))
    # rows written by hand (a list owner's decision, e.g. phyre-physics' script-only
    # library code) are kept: only this script's own rows are regenerated
    ours = ("tools/refs.py:", "callers only unreachable:")
    kept = []
    if args.dropped.exists():
        mine = {a for a, *_ in dropped}
        for line in args.dropped.read_text(encoding="utf-8").splitlines():
            row = line.split("\t")
            if line.startswith("#") or len(row) < 5 or row[4].startswith(ours):
                continue
            if int(row[0], 16) not in mine:
                kept.append((int(row[0], 16), line))
    rows = [(a, f"0x{a:08x}\t{n}\t{m}\t{s}\t{ev}") for a, n, m, s, ev, _ in dropped] + kept
    with open(args.dropped, "w", encoding="utf-8") as f:
        f.write("# address\tname\tmodule\tsize\tevidence (tools/reach.py: unreachable from every root; "
                "docs/rewrite-plan.md M5, draft)\n")
        for _, line in sorted(rows):
            f.write(line + "\n")

    # ---- work lists
    args.lists.mkdir(parents=True, exist_ok=True)
    for old in args.lists.glob("*.list"):
        old.unlink()
    todo = [a for a in unported if kind_of[g.module[a]] in ("game", "init")]
    todo_set = set(todo)
    sub = {a: [d for d in g.out.get(a, ()) if d in todo_set] for a in todo}
    order = [a for comp in tarjan_order(todo, sub) for a in comp]
    stubs = [a for a in order if category.get(a) == "stub" or g.size[a] < 5]
    held = [a for a in order if a in np_ and a not in stubs]
    rest = [a for a in order if a not in set(stubs) | set(held)]
    per_mod = defaultdict(list)
    for a in rest:
        per_mod[g.module[a]].append(a)
    lists = []

    def write_list(name, addrs):
        with open(args.lists / f"{name}.list", "w", encoding="utf-8") as f:
            for a in addrs:
                f.write(f"0x{a:08x}\n")
        lists.append((name, len(addrs), sum(g.size[a] for a in addrs)))

    for m in sorted(per_mod, key=lambda m: -len(per_mod[m])):
        addrs = per_mod[m]
        n = max(1, math.ceil(len(addrs) / 500))
        size = math.ceil(len(addrs) / n)
        for i in range(n):
            part = addrs[i * size:(i + 1) * size]
            write_list(m if n == 1 else f"{m}-{chr(ord('a') + i)}", part)
    if stubs:
        write_list("stubs", stubs)
    if held:
        write_list("m4-not-portable", held)

    # ---- report.md
    def row(cells):
        return "| " + " | ".join(str(c) for c in cells) + " |"

    L = []
    L.append("# M5 static reachability of FFX.exe")
    L.append("")
    L.append(f"tools/reach.py on FFX.exe {img.hash} ({time.strftime('%Y-%m-%d %H:%M')}, {time.time() - t0:.0f} s). "
             f"{len(nodes)} nodes: {len(nodes) - len(g.extra)} dump/ffx.sym functions and {len(g.extra)} call or "
             f"pointer targets outside every dump body. {sum(len(v) for v in g.out.values())} "
             f"edges. Reachable means reachable from some root; `strict` drops the addresses taken only in "
             f"unreachable code and ties EH funclets to the function that installs their handler (SafeSEH and "
             f"FuncInfo entries are no roots there). Hooked = replaced by a Rust function (`ported!` or a hand "
             f"hook without a trampoline); wrapped hooks count as not hooked.")
    L.append("")
    L.append("## Per module")
    L.append("")
    L.append(row(["module", "kind", "functions", "reachable hooked", "reachable not hooked", "(KB)",
                  "unreachable", "(KB)", "unreachable but hooked", "strict reachable"]))
    L.append(row(["---"] * 10))
    rank = {"game": 0, "init": 1, "library": 2}
    order_m = sorted(stats, key=lambda m: (rank[kind_of[m]], -stats[m]["functions"]))
    tot = {"game": defaultdict(int), "library": defaultdict(int), "init": defaultdict(int)}
    for m in order_m:
        s = stats[m]
        for k, v in s.items():
            tot[kind_of[m]][k] += v
    totals = {"GAME": "game", "STATIC-INIT": "init", "LIBRARY": "library"}
    for m in order_m + list(totals):
        s = tot[totals[m]] if m in totals else stats[m]
        k = "" if m in totals else kind_of[m]
        L.append(row([f"**{m}**" if not k else m, k, s["functions"], s["reach_hooked"], s["reach_unhooked"],
                      f"{s['reach_unhooked_bytes'] / 1024:.0f}", s["unreach"], f"{s['unreach_bytes'] / 1024:.0f}",
                      s["unreach_hooked"], s["strict"]]))
    L.append("")
    L.append("## Root kinds")
    L.append("")
    L.append("Each kind adds the functions it reaches that the kinds above it did not.")
    L.append("")
    L.append(row(["kind", "roots (sites)", "root functions", "functions added"]))
    L.append(row(["---"] * 4))
    sites = defaultdict(int)
    rnodes = defaultdict(set)
    for kind, node, site, _ in roots:
        sites[kind] += 1
        if node is not None:
            rnodes[kind].add(node)
    for kind in ROOT_ORDER:
        L.append(row([kind, sites[kind], len(rnodes[kind]), added.get(kind, 0)]))
    L.append(row(["**total**", sum(sites.values()), len(set().union(*rnodes.values())), len(seen)]))
    L.append("")
    L.append(f"CRT initialiser tables found: {', '.join(f'{lo:#x}..{hi:#x}' for lo, hi in crt_tables) or 'none'}; "
             f"native tables: {n_native}; TLS callbacks: {len(img.tls_callbacks())}; "
             f"SafeSEH handlers: {len(img.safeseh())}; exports: {len(img.exports())}.")
    L.append("")
    L.append("## Sanity checks")
    L.append("")
    # the unreachable hooked functions' callers up to the ones nothing calls
    tops = defaultdict(set)
    for a in hooked_miss:
        a = g.node_of(a)
        stack, vis = [a], {a}
        while stack:
            v = stack.pop()
            if not g.inn.get(v):
                tops[v].add(a)
            for c in g.inn.get(v, ()):
                if c not in vis:
                    vis.add(c)
                    stack.append(c)
    top_refs = {t: len(idx.to(t)) for t in tops}
    # refs from the function's own body (a state machine storing its own address)
    own_only = [t for t in tops if top_refs[t] and all(g.node_of(x) == t for x, _ in idx.to(t))]
    seen_cov = [a for a in hooked_miss if g.node_of(a) in cov_nodes]
    cats = Counter(category.get(g.node_of(a), "") or "-" for a in hooked_miss)
    clean_tops = sum(1 for n in top_refs.values() if n == 0) + len(own_only) == len(tops)
    L.append(f"- Hooked functions (ported! and hand hooks): {len(hooked)}; unreachable: {len(hooked_miss)} "
             f"({sum(1 for a in hooked_miss if kind_of[g.module[g.node_of(a)]] == 'game')} game). Their callers, "
             f"followed up, end in {len(tops)} functions nothing calls; tools/refs.py finds "
             f"{sum(1 for n in top_refs.values() if n == 0)} of those with 0 refs (no call, jump, data or code "
             f"pointer anywhere in the image) and {len(own_only)} with refs only from their own body; "
             f"{len(seen_cov)} of the {len(hooked_miss)} were seen in a coverage run. "
             + ("So no root kind is missing for them: they were ported although dead"
                if clean_tops and not seen_cov else "SOME ARE REFERENCED OR WERE SEEN: a root kind may be missing")
             + f" (classes.tsv: {', '.join(f'{c} {n}' for c, n in cats.most_common())}). The tops covering most:")
    for t in sorted(tops, key=lambda t: -len(tops[t]))[:25]:
        L.append(f"  - `0x{t:08x}` {g.name[t]} ({g.module[t]}, {top_refs[t]} refs): {len(tops[t])} hooked below it")
    with_refs = [t for t in tops if top_refs[t] and t not in own_only]
    for t in own_only:
        L.append(f"  - refs only from its own body: `0x{t:08x}` {g.name[t]}: "
                 f"{', '.join(f'{x:#x} {k}' for x, k in idx.to(t)[:4])}")
    for t in with_refs[:20]:
        L.append(f"  - with refs: `0x{t:08x}` {g.name[t]}: {', '.join(f'{x:#x} {k}' for x, k in idx.to(t)[:4])}")
    L.append(f"- Coverage runs: {len(hits)} hit files ({', '.join(sorted({f.parent.name for f in hits}))}), "
             f"{len(cov_nodes)} functions seen; unreachable: {len(cov_miss)}.")
    for a in cov_miss[:50]:
        L.append(f"  - `0x{a:08x}` {g.name[a]} ({g.module[a]}), in {', '.join(sorted(cov_nodes[a])[:3])}")
    L.append(f"- Functions classify.py calls dead (symbols/classes.tsv) that are reachable: "
             f"{sum(1 for a in nodes if category.get(a) == 'dead' and a in seen)} of "
             f"{sum(1 for a in nodes if category.get(a) == 'dead')}.")
    L.append(f"- Linear-disassembly ranges done again from a function start: {len(g.dis.redone)}; "
             f"fall-through edges (code running on into the next node): {len(g.fall)}.")
    L.append("")
    L.append("## Libraries: the functions game code reaches")
    L.append("")
    L.append("Library functions with a direct call or jump from reachable game code, or whose address "
             "reachable game code takes. These are what stage 1 replaces (Rust `std` for the CRT/STL, "
             "the Lua functions the game calls). Callers = reachable game functions.")
    for lib in sorted(lib_boundary, key=lambda m: -len(lib_boundary[m])):
        fns = lib_boundary[lib]
        L.append("")
        L.append(f"### {lib}: {len(fns)} functions")
        L.append("")
        L.append(row(["address", "name", "game callers", "hooked", "note"]))
        L.append(row(["---"] * 5))
        for a in sorted(fns, key=lambda a: (-len(fns[a]), a)):
            L.append(row([f"`0x{a:08x}`", g.name[a], len(fns[a]), state(a) or "",
                          thunk_import.get(a, "")]))
    L.append("")
    L.append("### Imports reachable game code calls directly (call/jmp through the IAT)")
    L.append("")
    by_dll = defaultdict(list)
    for imp, cs in game_imports.items():
        by_dll[imp.split("!")[0]].append((imp.split("!")[1], len(cs)))
    for dll in sorted(by_dll):
        items = sorted(by_dll[dll], key=lambda x: (-x[1], x[0]))
        L.append(f"- **{dll}** ({len(items)}): " + ", ".join(f"{n} ({c})" for n, c in items))
    L.append("")
    L.append("## Outputs")
    L.append("")
    L.append(f"- reachable-unported.tsv: {len(unported)} functions "
             f"({sum(1 for a in unported if kind_of[g.module[a]] == 'game')} game, "
             f"{sum(1 for a in unported if kind_of[g.module[a]] == 'init')} static-init, "
             f"{sum(1 for a in unported if kind_of[g.module[a]] == 'library')} library).")
    L.append(f"- symbols/m5-dropped.tsv: {len(dropped)} unreachable game functions "
             f"({sum(s for _, _, _, s, _, _ in dropped) / 1024:.0f} KB).")
    L.append("- work lists (leaf-first):")
    for name, n, b in lists:
        L.append(f"  - {args.lists / (name + '.list')}: {n} functions, {b / 1024:.0f} KB")
    (args.out / "report.md").write_text("\n".join(L) + "\n", encoding="utf-8")
    print(f"reach: {len(seen)} of {len(nodes)} reachable, {len(unported)} reachable-unported, "
          f"{len(dropped)} dropped; hooked misses {len(hooked_miss)}, coverage misses {len(cov_miss)}; "
          f"{time.time() - t0:.0f} s -> {args.out / 'report.md'}", file=sys.stderr)


if __name__ == "__main__":
    main()
