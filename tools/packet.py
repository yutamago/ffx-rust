#!/usr/bin/env python3
"""A work packet for porting one function: everything a fresh agent needs to
start, in one compact markdown file (docs/workflow.md, "Work packets").

  FFX_GAME_DIR=... tools/packet.py ADDR|NAME [--out FILE]
  FFX_GAME_DIR=... tools/packet.py --batch "WORKLIST ARGS" [--dir DIR]

--batch runs tools/worklist.py with the quoted arguments (e.g.
"--module battle --limit 10 --unclaimed") and writes one packet per listed
function into DIR (default ~/ffx-work/packets), named <addr>_<name>.md.

A packet holds: identity (module, category, size, callers, claim); the
Ghidra C from the dump; the Intel asm with names on addresses; every callee
with its convention and stack words (tools/callarity.py's measure), x87
result, and ported state (the Rust path of its shim) or "call through the
game"; the globals and labels it touches (relocations + data_refs.tsv +
ffx.sym); a heuristic trap scan; for x87-heavy code a tools/x87-lift.py
draft (reference only); a test skeleton for the module's oracle file.

Facts measured from machine code are heuristics where marked: a linear
scan of the function, not a data-flow analysis. Check them against the asm.
"""
import argparse
import bisect
import importlib.util
import json
import os
import pathlib
import re
import shlex
import struct
import subprocess
import sys
import tempfile
from collections import defaultdict

TOOLS = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import callarity  # noqa: E402
import claim  # noqa: E402
import progress  # noqa: E402

REPO = progress.REPO
WORK = progress.WORK
DECOMP = WORK / "decomp"
if not os.environ.get("FFX_GAME_DIR"):
    sys.exit("set FFX_GAME_DIR (the asm comes from FFX.exe)")
EXE = pathlib.Path(os.environ["FFX_GAME_DIR"]).expanduser() / "FFX.exe"
_spec = importlib.util.spec_from_file_location("check_clobber", TOOLS / "check-clobber.py")
clobber = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(clobber)

SECURITY_COOKIE = 0x00C613D8  # tools/ownership.py
# module -> oracle test file stem (the rest are the module name)
TEST_STEM = {"map": "field", "ps2-emu": "ps2", "pc-ui": "pc_ui"}
X87_LOADS = {"fld", "fild", "fldz", "fld1", "fldpi", "fldl2e", "fldl2t", "fldlg2", "fldln2"}
X87_POP1 = ("fstp", "fistp", "fisttp", "fcomp", "ficomp", "fucomp", "fcomip", "fucomip",
            "faddp", "fsubp", "fsubrp", "fmulp", "fdivp", "fdivrp", "fbstp")
X87_POP2 = ("fcompp", "fucompp")
WRITES = {"mov", "movss", "movsd", "movaps", "movups", "movq", "movd", "movdqa", "movdqu", "movlps",
          "movhps", "fst", "fstp", "fist", "fistp", "fisttp", "fnstcw", "fstcw", "fnstsw", "fstsw",
          "seta", "setae", "setb", "setbe", "sete", "setne", "setg", "setge", "setl", "setle", "sets",
          "setns", "setnp", "setp", "cvttss2si"}


class Image:
    """FFX.exe's sections and base relocations, read from the PE headers."""

    def __init__(self, path):
        self.data = path.read_bytes()
        d = self.data
        pe = struct.unpack_from("<I", d, 0x3C)[0]
        nsec, opt = struct.unpack_from("<H", d, pe + 6)[0], struct.unpack_from("<H", d, pe + 20)[0]
        o = pe + 24
        self.base = struct.unpack_from("<I", d, o + 28)[0]
        self.end = self.base + struct.unpack_from("<I", d, o + 56)[0]
        reloc_rva, reloc_size = struct.unpack_from("<II", d, o + 96 + 5 * 8)
        self.sections = []
        for i in range(nsec):
            s = pe + 24 + opt + 40 * i
            name = d[s:s + 8].rstrip(b"\0").decode()
            vsize, rva, rsize, raw = struct.unpack_from("<IIII", d, s + 8)
            self.sections.append((name, self.base + rva, self.base + rva + max(vsize, rsize), raw, rsize))
        sites = []
        p, end = self.offset(self.base + reloc_rva), self.offset(self.base + reloc_rva) + reloc_size
        while p < end:
            page, size = struct.unpack_from("<II", d, p)
            if size < 8:
                break
            for k in range(8, size, 2):
                e = struct.unpack_from("<H", d, p + k)[0]
                if e >> 12 == 3:
                    sites.append(self.base + page + (e & 0xFFF))
            p += size
        self.relocs = sorted(sites)

    def offset(self, va):
        for _, lo, hi, raw, rsize in self.sections:
            if lo <= va < hi and va - lo < rsize:
                return raw + va - lo
        return None

    def section(self, va):
        return next((n for n, lo, hi, _, _ in self.sections if lo <= va < hi), None)

    def dword(self, va):
        off = self.offset(va)
        return struct.unpack_from("<I", self.data, off)[0] if off is not None else None

    def relocs_in(self, lo, hi):
        i = bisect.bisect_left(self.relocs, lo)
        out = []
        while i < len(self.relocs) and self.relocs[i] < hi:
            out.append(self.relocs[i])
            i += 1
        return out


def uses(text):
    """alias -> full path, from a file's `use ffx_*` lines (one brace level)."""
    out = {}
    for m in re.finditer(r"^use (ffx_\w+(?:::\w+)*)(?:::\{([^}]*)\}|\s+as\s+(\w+))?;", text, re.M):
        base, group, alias = m.groups()
        if group is None:
            out[alias or base.rsplit("::", 1)[-1]] = base
            continue
        for item in group.split(","):
            item = item.strip()
            if not item:
                continue
            name, _, al = (x.strip() for x in item.partition(" as "))
            full = base if name == "self" else f"{base}::{name}"
            out[al or (base.rsplit("::", 1)[-1] if name == "self" else name.rsplit("::", 1)[-1])] = full
    return out


class World:
    """Everything loaded once per run (a batch shares it)."""

    def __init__(self):
        self.rows = {a: (n, s) for a, n, s in progress.functions()}
        self.starts = sorted(self.rows)
        self.names = progress.symbols()
        self.module = progress.placement(progress.functions(), self.names)
        self.category = progress.classes()
        self.ported = progress.ported()
        self.hooks = progress.hooks()
        self.claims = claim.claims()
        self.sym = {}  # address -> (kind, name, comment)
        for line in (REPO / "symbols/ffx.sym").read_text(encoding="utf-8").splitlines():
            body, _, comment = line.partition("#")
            p = body.split()
            if len(p) == 3 and p[1] in ("function", "label"):
                self.sym[int(p[0], 16)] = (p[1], p[2], comment.strip())
        self.labels = sorted(a for a, (k, _, _) in self.sym.items() if k == "label")
        self.callers = defaultdict(set)
        self.sites = defaultdict(list)
        with open(DECOMP / "call_sites.tsv", encoding="utf-8") as f:
            next(f)
            for line in f:
                a, site, c = line.rstrip("\n").split("\t")
                if "EXTERNAL" not in c:
                    self.callers[int(c, 16)].add(int(a, 16))
                    self.sites[int(c, 16)].append((int(a, 16), int(site, 16)))
        self.data_refs = defaultdict(dict)
        with open(DECOMP / "data_refs.tsv", encoding="utf-8") as f:
            next(f)
            for line in f:
                fn, addr, acc, _ = line.rstrip("\n").split("\t")
                d = self.data_refs[int(fn, 16)]
                d[int(addr, 16)] = d.get(int(addr, 16), "") + acc
        self.image = Image(EXE)
        self.exe = callarity.Exe(EXE, {a: s for a, (_, s) in self.rows.items()})
        self.shims = self._shims()
        self._files = {}
        self._float10 = None

    def _shims(self):
        """address -> (file, signature, Rust path the shim calls)."""
        out = {}
        sig = re.compile(r'^\s*(\w+) @ 0x([0-9a-f_]+) =>\s*(extern "[\w-]+" fn \w+\([^)]*\)(?:\s*->\s*[\w:]+)?)', re.M)
        for f in sorted((REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
            text = f.read_text(encoding="utf-8")
            alias = uses(text)
            ms = list(sig.finditer(text))
            for i, m in enumerate(ms):
                body = text[m.end(): ms[i + 1].start() if i + 1 < len(ms) else len(text)]
                path = None
                for pm in re.finditer(r"\b([a-z_]\w*(?:::\w+)+)\s*\(", body):
                    head, _, rest = pm.group(1).partition("::")
                    if head in ("super", "self", "std", "core"):
                        continue
                    if head in alias or head.startswith("ffx_"):
                        path = f"{alias.get(head, head)}::{rest}"
                        break
                out[int(m.group(2).replace("_", ""), 16)] = (f.name, " ".join(m.group(3).split()), path)
        return out

    def name(self, a):
        return self.names.get(a) or (self.rows[a][0] if a in self.rows else f"0x{a:08x}")

    def function_at(self, a):
        i = bisect.bisect_right(self.starts, a) - 1
        if i >= 0 and self.starts[i] <= a < self.starts[i] + max(self.rows[self.starts[i]][1], 1):
            return self.starts[i]
        return None

    def describe(self, v):
        """A short name for an absolute address, or None."""
        if v in self.rows:
            return self.name(v)
        if v == SECURITY_COOKIE:
            return "__security_cookie"
        if v in self.sym:
            return self.sym[v][1]
        i = bisect.bisect_right(self.labels, v) - 1
        near = [x for x in self.labels[max(i - 1, 0): i + 1] if 0 <= v - x < 0x40] if i >= 0 else []
        if near:
            # inside a labelled struct or array (which one is a guess)
            return " or ".join(f"{self.sym[x][1]}+{v - x:#x}?" for x in reversed(near))
        f = self.function_at(v)
        if f is not None:
            return f"{self.name(f)}+{v - f:#x}"
        if v in self.exe.imports:
            return self.exe.imports[v]
        if self.image.section(v) == ".text":
            return "in .text outside every function (a jump table?)"
        return None

    def decomp(self, a):
        """(Ghidra's C for the function, its signature line)."""
        key = a & ~0xFFFF
        if key not in self._files:
            p = DECOMP / f"code/0x{key:08x}.c"
            text = p.read_text(encoding="utf-8", errors="replace") if p.exists() else ""
            blocks = {}
            parts = re.split(r"^// ==== 0x([0-9a-f]+) ====\n", text, flags=re.M)
            for i in range(1, len(parts) - 1, 2):
                blocks[int(parts[i], 16)] = parts[i + 1].strip("\n")
            self._files[key] = blocks
        c = self._files[key].get(a, "")
        bare = re.sub(r"/\*.*?\*/", "", c, flags=re.S)
        lines = bare.splitlines()
        k = next((i for i, l in enumerate(lines) if l.strip() and "(" in l), None)
        sig = ""
        if k is not None:
            for l in lines[k:k + 8]:
                sig += l.strip()
                if ")" in l:
                    break
        return c, sig.strip()

    def float10(self):
        """Names Ghidra casts to float10 at some call site: an ST0 result."""
        if self._float10 is None:
            found = set()
            for p in (DECOMP / "code").glob("*.c"):
                found.update(re.findall(r"\(float10\)(\w+)\(", p.read_text(encoding="utf-8", errors="replace")))
            self._float10 = found
        return self._float10


def body(w, a):
    """The function's asm. The dump's body size can stop short of the final
    epilogue (a shared `pop; ret` tail): extend up to the next ret or jmp,
    not into the next function."""
    size = max(w.rows[a][1], 1)
    code = clobber.disasm(a, size)
    end = a + size
    for _ in range(16):
        last = code[max(code)] if code else ""
        if last.startswith(("ret", "jmp")) or end in w.rows:
            break
        more = clobber.disasm(end, 16)
        if not more:
            break
        for x in sorted(more):
            if x in w.rows:
                break
            code[x] = more[x]
            if more[x].startswith(("ret", "jmp")):
                return code
        end += 16
    return code


def is_ftol(w, c):
    """The CRT's statically linked float-to-int helpers (ftol2_sse, ...)."""
    return "ftol" in w.name(c).lower()


def x87_depth_at_ret(w, code):
    """Heuristic: the largest x87 stack depth at a `ret` over the function's
    branches (positive: it leaves a float in ST0). A call counts as pushing
    ST0 when the instruction after it consumes ST0 (check-clobber's
    reads_st0); _ftol pops ST0 into eax; a two-operand _CI intrinsic pops one."""
    order = sorted(code)
    nxt = {x: y for x, y in zip(order, order[1:])}
    best, seen, todo = 0, {}, [(order[0], 0)] if order else []
    while todo:
        a, depth = todo.pop()
        while a is not None:
            if seen.get(a, -1) >= depth:
                break
            seen[a] = depth
            insn = code[a]
            op = insn.split()[0] if insn else ""
            if op in X87_LOADS:
                depth += 1
            elif op in X87_POP2:
                depth -= 2
            elif op in X87_POP1:
                depth -= 1
            elif op == "ret":
                best = max(best, depth)
                break
            elif op == "call":
                m = re.match(r"call\s+0x([0-9a-f]+)$", insn)
                t = int(m.group(1), 16) if m else None
                intr = w.exe.intrinsic(t) if m else None
                if (intr and intr.startswith("_ftol")) or (t and is_ftol(w, t)):
                    depth -= 1
                elif intr in callarity.TWO_OPERAND:
                    depth -= 1
                elif not (intr and intr.startswith("_CI")) and clobber.reads_st0(code, a):
                    depth += 1
            depth = max(depth, 0)
            if op.startswith("j"):
                m = re.search(r"0x([0-9a-f]+)$", insn)
                if m and int(m.group(1), 16) in code:
                    todo.append((int(m.group(1), 16), depth))
                if op == "jmp":
                    break
            a = nxt.get(a)
    return best


def reads_this(code):
    """Registers among ecx/edx read before written, scanning linearly up to
    the first call, jmp or ret (thiscall/fastcall arguments)."""
    out, written = set(), set()
    for a in sorted(code):
        insn = code[a]
        op = insn.split()[0] if insn else ""
        if op.startswith(("call", "jmp", "ret")):
            break
        r, w = clobber.effect(insn)
        out |= {x for x in r & {"ecx", "edx"} if x not in written}
        written |= w
    return out


def convention(w, c):
    """(text, stack words, pops, float) for a callee."""
    if c not in w.rows:
        return "unknown", None, None, False
    reads, pops, forwards = w.exe.facts(c)
    code = body(w, c)
    regs = reads_this(code)
    first = [code[x] for x in sorted(code)[:2]]
    if first == ["push   ebx", "mov    ebx,esp"]:
        # an aligned frame (and esp,-16): arguments at [ebx+8...], which
        # callarity's ebp/esp measure does not see
        offs = [int(m.group(1), 16) for t in code.values() for m in re.finditer(r"\[ebx\+0x([0-9a-f]+)\]", t)]
        reads = max([(o - 8) // 4 + 1 for o in offs if o >= 8], default=0)
    if {"ecx", "edx"} <= regs:
        kind = "fastcall"
        words = pops // 4 if pops else 0
    elif "ecx" in regs:
        kind = "thiscall" + ("" if pops else " (or a cdecl/fastcall taking ecx)")
        words = pops // 4 if pops else reads
    elif pops:
        kind = "stdcall (or a thiscall not reading ecx)"
        words = pops // 4
    else:
        kind = "cdecl"
        words = reads
    if forwards:
        kind += ", tail-jumps on"
    _, sig = w.decomp(c)
    fl = sig.startswith(("float10", "float ", "double ")) or w.name(c) in w.float10() \
        or x87_depth_at_ret(w, code) > 0
    return kind, words, pops, fl


def callee_state(w, c):
    if is_ftol(w, c):
        return "CRT float-to-int (pops ST0, result in eax): ffx_bridge::x87::ftol"
    if w.name(c) == "security_check_cookie":
        return "stack cookie check: nothing to port"
    m = w.module.get(c)
    cat = w.category.get(c, "logic")
    if c in w.shims:
        f, sig, path = w.shims[c]
        return f"PORTED -> `{path or '?'}` (ported/{f}: `{sig}`)"
    if w.hooks.get(c) == "wrapped":
        return "wrapped by a hand hook; call through the game"
    if progress.KIND.get(m) == "library":
        return f"library ({m}): call through the game"
    if cat != "logic":
        return f"{cat}: not ported exactly, call through the game"
    return "not ported: call through the game"


def scan_frame(code):
    """Heuristic stack-slot scan. Per instruction: the entry-relative offsets
    (0 = return address) of local slots it reads and writes; `lea` of a
    slot counts as writing the 0x40 bytes from it (a buffer handed out).
    esp offsets follow a linear push/pop/sub/add count, reset at jump
    targets after a ret or jmp."""
    order = sorted(code)
    delta, frame, saved = 0, None, {}
    reads, writes, taken = defaultdict(set), defaultdict(set), defaultdict(set)
    prev = ""
    for a in order:
        if a in saved and prev.split()[:1] and prev.split()[0] in ("ret", "jmp"):
            delta = saved[a]
        insn = code[a]
        parts = insn.split(None, 1)
        op, ops = (parts[0], parts[1] if len(parts) > 1 else "") if parts else ("", "")
        if op.startswith("j") and (m := re.match(r"0x([0-9a-f]+)$", ops.strip())):
            saved.setdefault(int(m.group(1), 16), delta)
        if op == "mov" and ops == "ebp,esp":
            frame = -delta
        refs = []
        for m in re.finditer(r"\[(esp|ebp)([+-]0x[0-9a-f]+)?\]", ops):
            off = int(m.group(2), 16) if m.group(2) else 0
            if m.group(1) == "esp":
                refs.append(off - delta)
            elif frame is not None:
                refs.append(frame + off)
        first_mem = ops.split(",")[0].strip().endswith("]") if ops else False
        for k, rel in enumerate(refs):
            if rel >= 0:
                continue
            if op == "lea":
                taken[a].add(rel)
            elif op in WRITES and k == 0 and first_mem:
                writes[a].add(rel)
            else:
                reads[a].add(rel)
        if op == "push":
            delta += 4
            writes[a].add(-delta)
        elif op == "pop":
            delta -= 4
        elif m := re.match(r"sub\s*esp,0x([0-9a-f]+)", insn):
            delta += int(m.group(1), 16)
        elif m := re.match(r"add\s*esp,0x([0-9a-f]+)", insn):
            delta -= int(m.group(1), 16)
        prev = insn
    return reads, writes, taken


def unwritten_reads(code):
    """(slot, read site) for local slots read where some path from the entry
    reaches the read without writing them (a must-write data flow over the
    function's branches; heuristic, see scan_frame)."""
    reads, writes, taken = scan_frame(code)
    order = sorted(code)
    slots = set().union(*reads.values()) if reads else set()
    if not slots:
        return []
    gen = {a: set(writes.get(a, ())) | {s for t in taken.get(a, ()) for s in slots if t <= s < t + 0x40}
           for a in order}
    succ = {}
    for i, a in enumerate(order):
        insn = code[a]
        op = insn.split()[0] if insn else ""
        nxt = [order[i + 1]] if i + 1 < len(order) and op not in ("ret", "jmp") else []
        if op.startswith("j") and (m := re.search(r"0x([0-9a-f]+)$", insn)) and int(m.group(1), 16) in code:
            nxt.append(int(m.group(1), 16))
        succ[a] = nxt
    full = slots
    inn = {a: set(full) for a in order}
    inn[order[0]] = set()
    changed = True
    while changed:
        changed = False
        out = {a: inn[a] | gen[a] for a in order}
        new = {a: set(full) for a in order}
        new[order[0]] = set()
        for a in order:
            for b in succ[a]:
                if b != order[0]:
                    new[b] &= out[a]
        if new != inn:
            inn, changed = new, True
    found = {}
    for a in order:
        for r in reads.get(a, ()):
            if r not in inn[a] and r not in found:
                found[r] = a
    return sorted(found.items(), key=lambda x: x[1])


def traps(w, a, code, calls):
    """Heuristic trap scan of the function's asm (see the module docstring)."""
    out = []
    ops = [code[x].split()[0] for x in sorted(code) if code[x]]
    if any(o.startswith("rep") or o in ("stos", "movs", "stosd", "stosb", "movsb") for o in ops):
        out.append("block copy/fill (rep/stos/movs) on the frame: the stack-slot check below may miss writes")
    for rel, site in unwritten_reads(code):
        out.append(f"{site:#x} `{code[site]}` reads a stack slot that some path to it never writes: stack "
                   "garbage? (zero the stack in tests; reproduce what the caller leaves there)")
    _, sig = w.decomp(a)
    if sig.startswith("void") and "eax" in clobber.writes(code):
        readers = []
        for caller, site in w.sites.get(a, [])[:40]:
            cc = body(w, caller) if caller in w.rows else {}
            if cc.get(site, "").startswith("call") and "eax" in clobber.live_after(cc, site, {"eax"}):
                readers.append(f"{w.name(caller)} ({site:#x})")
        if readers:
            out.append(f"Ghidra says void, but eax is set and callers read it: {', '.join(readers[:4])}"
                       f"{' ...' if len(readers) > 4 else ''}: the shim must return it")
    x87 = [o for o in ops if o.startswith("f") and o != "fs"]
    if x87:
        extra = []
        if "fldcw" in ops:
            extra.append("fldcw (rounding/precision change)")
        if any(o.startswith(("fcom", "fucom")) for o in ops):
            extra.append("x87 compares (NaN rules: ffx_bridge::x87)")
        targets = [int(code[x].split("0x")[1], 16) for x in code if re.match(r"call\s+0x[0-9a-f]+$", code[x])]
        if any(is_ftol(w, t) or (w.exe.intrinsic(t) or "").startswith(("_CI", "_ftol")) for t in targets):
            extra.append("CRT intrinsics (_CI*/_ftol: ffx_bridge::x87)")
        if x87_depth_at_ret(w, code) > 0:
            extra.append("leaves a float in ST0 at ret: the shim must return f32/f64")
        out.append(f"x87: {len(x87)} instructions" + (f"; {', '.join(extra)}" if extra else "")
                   + ". Compute in f64 in the original's order, round to f32 at each store")
    for site, c, pushed in calls:
        if c not in w.rows or pushed is None:
            continue
        reads_n, pops, forwards = w.exe.facts(c)
        if forwards or reads_n is None:
            continue
        need = pops // 4 if pops else reads_n
        if pushed < need:
            out.append(f"call {w.name(c)} at {site:#x}: about {pushed} words pushed, the callee reads {need}: "
                       "it sees the caller's stack beyond them")
    # pushed immediates that look like image addresses but carry no
    # relocation: constants (colours, masks), not addresses to rebase
    order = sorted(code)
    plain = []
    for i, x in enumerate(order):
        m = re.match(r"push\s+0x([0-9a-f]{6,8})$", code[x])
        if not m:
            continue
        v = int(m.group(1), 16)
        nxt = order[i + 1] if i + 1 < len(order) else x + 5
        if w.image.base <= v < w.image.end and not any(w.image.dword(r) == v for r in w.image.relocs_in(x, nxt)):
            plain.append(f"{v:#x} at {x:#x}")
    if plain:
        out.append(f"pushed constants in the image range without a relocation: {', '.join(plain[:6])}"
                   f"{' ...' if len(plain) > 6 else ''}: plain numbers, don't rebase them")
    return out


def call_sites(w, a, code):
    """(site, callee, stack words set up for it; heuristic) for every direct
    call: the most of (a) the caller's own `add esp,N` right after it and
    (b) a walk back from the call over push/pop/`sub esp` and stores to
    [esp+k] (slots reserved earlier) up to a jmp, ret, `add esp` or the
    prologue. An earlier call leaves its words for this one (a cdecl call
    the caller does not clean up at once) unless it pops them itself.
    None when the walk reaches a jump target, a ret or a jmp before the
    words add up: they may come from another path."""
    order = sorted(code)
    prologue = 0
    for x in order:
        if not re.match(r"push   (ebp|ebx|esi|edi)$|mov    ebp,esp$|sub    esp,", code[x]):
            break
        prologue = x
    targets = {int(t.group(1), 16) for v in code.values() if v.startswith("j")
               for t in [re.search(r"0x([0-9a-f]+)$", v)] if t}
    out = []
    for i, x in enumerate(order):
        m = re.match(r"call\s+0x([0-9a-f]+)$", code[x])
        if not m:
            continue
        after = code[order[i + 1]] if i + 1 < len(order) else ""
        cleanup = re.match(r"add\s+esp,0x([0-9a-f]+)$", after)
        words = most = debt = 0
        open_path = False
        for y in reversed(order[:i]):
            t = code[y]
            if t.startswith(("jmp", "ret")):
                open_path = True
                break
            if y <= prologue or t.startswith("add    esp,"):
                break
            if t.startswith("call"):
                mc = re.match(r"call\s+0x([0-9a-f]+)$", t)
                if not mc:
                    break
                _, pops, _ = w.exe.facts(int(mc.group(1), 16))
                debt += (pops or 0) // 4
                continue
            if t.startswith("push") and debt:
                debt -= 1
            elif t.startswith("push"):
                words += 1
            elif t.startswith("pop"):
                words -= 1
            elif mm := re.match(r"sub\s+esp,0x([0-9a-f]+)$", t):
                words += int(mm.group(1), 16) // 4
            elif mm := re.match(r"(?:mov|fstp|fst|movss|fistp)\s+\w+ PTR \[esp(?:\+0x([0-9a-f]+))?\]", t):
                most = max(most, words + int(mm.group(1) or "0", 16) // 4 + 1)
            if y in targets:
                open_path = True
                break
        words = max(words, most, int(cleanup.group(1), 16) // 4 if cleanup else 0)
        out.append((x, int(m.group(1), 16), None if open_path and not cleanup else words))
    return out


def lift(w, a, code, callees):
    """A tools/x87-lift.py draft, or the reason it stopped."""
    lines, frame, stack = [], {}, {}
    ebp = len(code) > 1 and list(code.values())[:2] == ["push   ebp", "mov    ebp,esp"]
    for x in sorted(code):
        t = code[x].lower().replace("ds:0x", "[0x") if code[x] else ""
        t = re.sub(r"\[0x([0-9a-f]+)(?!\])", r"[0x\1]", t)
        t = re.sub(r",(?! )", ", ", t)
        t = re.sub(r"\s+", " ", t, count=1)
        lines.append(f"  {x:x}: {t}")
        for m in re.finditer(r"(dword|qword) ptr \[(esp|ebp)\+0x([0-9a-f]+)\]", t):
            off = int(m.group(3), 16)
            ty = ("f32" if m.group(1) == "dword" else "f64") if t.startswith("fld ") else "u32"
            if m.group(2) == "ebp" and ebp and off >= 8:
                frame.setdefault(str(off), [ty, f"arg{(off - 8) // 4}"])
    if not ebp:
        # esp frames: arguments only before the first push/sub (entry esp)
        for x in sorted(code):
            t = code[x]
            if t.startswith(("push", "sub", "call")):
                break
            for m in re.finditer(r"(DWORD|QWORD) PTR \[esp\+0x([0-9a-f]+)\]", t):
                off = int(m.group(2), 16)
                ty = ("f32" if m.group(1) == "DWORD" else "f64") if t.startswith("fld ") else "u32"
                stack.setdefault(str(off), [ty, f"arg{(off - 4) // 4}"])
    entry = a
    if ebp:
        entry = sorted(code)[2]  # after push ebp; mov ebp,esp (the lifter's [ebp+off] are frame slots)
    calls, st0, eax = {}, {}, []
    order = sorted(code)
    for i, x in enumerate(order):
        m = re.match(r"call\s+0x([0-9a-f]+)$", code[x])
        if not m:
            continue
        c = int(m.group(1), 16)
        conv = callees.get(c)
        if conv and conv[3]:
            st0[f"{c:#x}"] = f"st0_call(mem, {c:#x}, {{0}})"
        calls[f"{c:#x}"] = (conv[1] if conv and conv[1] is not None else 0)
        if i + 1 < len(order) and code[order[i + 1]].startswith("test   eax,eax"):
            eax.append(f"{c:#x}")
    # The lifter takes straight-line x87 code over frame and image operands.
    # Lift section by section: from an entry until the instruction it
    # rejects (made a stop), then on from the next x87 instruction.
    order = sorted(code)
    x87_at = [x for x in order if code[x] and code[x].split()[0].startswith("f")]
    drafts, first_err, at, stops = [], None, entry, []
    with tempfile.TemporaryDirectory() as d:
        (pathlib.Path(d) / "f.asm").write_text("\n".join(lines) + "\n")
        for _ in range(8):
            cfg = {"asm": "f.asm", "entries": {f"{at:#x}": f"{at:#x}"}, "frame": frame, "stack": stack,
                   "fpu": [], "calls": calls, "st0_calls": st0, "eax_calls": eax,
                   "stops": [f"{x:#x}" for x in stops],
                   "regs": {r: r for r in ("eax", "ebx", "edi", "edx", "ebp")}}
            (pathlib.Path(d) / "f.json").write_text(json.dumps(cfg))
            r = subprocess.run([sys.executable, "-I", str(TOOLS / "x87-lift.py"), str(pathlib.Path(d) / "f.json")],
                               capture_output=True, text=True)
            nxt_from = None
            if r.returncode:
                err = (r.stderr.strip().splitlines() or ["?"])[-1]
                first_err = first_err or err
                m = re.search(r"Exception: ([0-9a-f]+): ", err)
                bad = int(m.group(1), 16) if m else None
                if bad is not None and bad != at and bad not in stops:
                    stops.append(bad)
                    continue  # rerun the section up to the rejected instruction
                nxt_from = bad if bad is not None else at
            else:
                body_lines = [l for l in r.stdout.splitlines() if l.strip().startswith("let ")]
                if len(body_lines) >= 2:
                    drafts.append(r.stdout.rstrip())
                nxt_from = min([x for x in stops if x > at], default=order[-1])
            later = [x for x in x87_at if x > nxt_from]
            if not later:
                break
            # restart at the first x87 instruction after the stop
            at = later[0]
    if drafts:
        return "\n".join(drafts), None
    return None, first_err


def test_skeleton(w, a, conv):
    m = w.module.get(a, "?")
    stem = TEST_STEM.get(m, m.replace("-", "_"))
    tests = REPO / "crates/ffx-oracle/tests"
    files = sorted(p for p in tests.glob("*.rs") if p.stem == stem or p.stem.startswith(stem + "_"))
    if not files:
        return f"No oracle test file for module {m}: start crates/ffx-oracle/tests/{stem}.rs."
    # the file that names the most of this function's neighbours and callees
    near = [x for x in w.ported if w.module.get(x) == m]
    near.sort(key=lambda x: abs(x - a))
    keys = {f"0x{x >> 16:04x}_{x & 0xFFFF:04x}" for x in near[:40]}
    best = max(files, key=lambda p: sum(k in p.read_text(encoding="utf-8") for k in keys))
    text = best.read_text(encoding="utf-8")
    crates = re.findall(r"^use (ffx_\w+)", text, re.M)
    crate = next((c for c in crates if c not in ("ffx_bridge", "ffx_oracle")), "ffx_<module>")
    helpers = [h for h in re.findall(r"^(?:pub )?fn (\w+\([^)]*\)(?:\s*->\s*[^{]+)?)", text, re.M)
               if not re.fullmatch(r"\w+\(\)\s*", h)][:12]
    for ty, block in re.findall(r"^impl (\w+) \{\n(.*?)^\}", text, re.M | re.S):
        helpers += [f"{ty}::{n}() -> Option<{ty}>" for n in re.findall(r"^\s+(?:pub )?fn (\w+)\(\) -> Option<Self>", block, re.M)]
    if (tests / f"{best.stem}.rs").exists() and "mod battle_common;" in text:
        helpers += [f"battle_common::{h}" for h in re.findall(
            r"^pub fn (\w+\([^)]*\)(?:\s*->\s*[^{]+)?)", (tests / "battle_common/mod.rs").read_text(), re.M)][:6]
    have = {re.match(r"(?:battle_common::)?(\w+)", h).group(1) for h in helpers}
    snake = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", w.name(a)).lower()
    kind, words, pops, fl = conv
    n = words or 0
    args = ", ".join(f"a{i}" for i in range(n))
    cv = {"thiscall": "Conv::Thiscall(this)", "stdcall": "Conv::Stdcall", "fastcall": "Conv::Fastcall"}.get(
        kind.split()[0], "Conv::Cdecl")
    if "synthetic" in have and "snapshot" in have:
        head = "    for (mut a, mut b) in [synthetic(), snapshot()].into_iter().flatten() {"
        tail = "    }"
        ind = "        "
    elif "twins" in have:
        head = "    for mut t in twins() {\n        let (a, b) = (&mut t.a, &mut t.b);"
        tail, ind = "    }", "        "
    elif "Twin::image" in " ".join(helpers) and "Twin::snapshot" in " ".join(helpers):
        head = ("    for mut t in [Twin::image(), Twin::snapshot()].into_iter().flatten() {\n"
                "        let (a, b) = (&mut t.a, &mut t.b);")
        tail, ind = "    }", "        "
    elif "setup" in have:
        head = "    let Some((mut a, mut b)) = setup() else {\n        return;\n    };"
        tail, ind = "", "    "
    else:
        head = "    // build the two oracles the way this file's other tests do (helpers above)"
        tail, ind = "", "    "
    _, sig = w.decomp(a)
    void = sig.startswith("void") and not fl
    bref = "b" if "(&mut t.a, &mut t.b)" in head else "&mut b"
    body = [
        "#[test]",
        f"fn {snake}_matches_the_original() {{",
        head,
        f"{ind}// {kind}, {n} stack word(s){'' if not pops else f', ret {pops}'}",
        f"{ind}let ra = a.call(a.rebase(0x{a >> 16:04x}_{a & 0xffff:04x}), {cv}, &[{args}]).unwrap();",
        f"{ind}let rb = {crate}::<path>::bridge::{snake}({bref}{', ' + args if args else ''});",
        f"{ind}// Ghidra says void: compare eax only if the trap scan says callers read it" if void else
        f"{ind}assert_eq!(ra.st0, Some(f64::from(rb)));  // ST0: compare bits as the file's float helper does"
        if fl else f"{ind}assert_eq!(ra.eax, rb);",
        f"{ind}assert_eq!(a.diff({bref}).unwrap(), []);",
    ]
    if tail:
        body.append(tail)
    body.append("}")
    out = [f"File: `crates/ffx-oracle/tests/{best.name}` (best match for the module; "
           f"run on the image and both snapshots). Helpers there:", ""]
    out += [f"- `{h.strip()}`" for h in helpers]
    out += ["", "```rust", *body, "```"]
    return "\n".join(out)


def annotate(w, insn):
    names = []
    for m in re.finditer(r"0x([0-9a-f]{6,8})\b", insn):
        d = w.describe(int(m.group(1), 16))
        if d and d not in names:
            names.append(d)
    return f"{insn:44}; {', '.join(names)}" if names else insn


def packet(w, a):
    name, size = w.rows[a]
    name = w.name(a)
    m = w.module.get(a, "?")
    code = body(w, a)
    c_text, sig = w.decomp(a)
    out = [f"# {name} @ 0x{a:08x}", ""]
    callers = sorted(w.callers.get(a, ()))
    cl = w.claims.get(a)
    sym = w.sym.get(a)
    out.append(f"- module **{m}**, category {w.category.get(a, 'logic')}, {size} bytes, "
               f"{len(callers)} caller(s)")
    if a in w.ported:
        out.append(f"- **already ported** ({w.shims.get(a, ('?',))[0]})")
    if cl:
        out.append(f"- claimed by {cl.get('agent')}{' (stale)' if cl['stale'] else ''}")
    if sym and sym[2]:
        out.append(f"- ffx.sym: {sym[2][:700]}")
    conv = convention(w, a)
    out.append(f"- measured convention: {conv[0]}, {conv[1]} stack word(s)"
               + (f", ret {conv[2]}" if conv[2] else "") + (", returns a float in ST0" if conv[3] else "")
               + f" (Ghidra: `{sig}`)")
    if callers:
        cs = ", ".join(f"{w.name(x)}" for x in callers[:12])
        out.append(f"- callers: {cs}{f' (+{len(callers) - 12})' if len(callers) > 12 else ''}")

    # callees
    calls = call_sites(w, a, code)
    seen, rows = {}, []
    for site, c, _ in calls:
        if c in seen:
            continue
        seen[c] = convention(w, c)
        intr = w.exe.intrinsic(c)
        kind, words, pops, fl = seen[c]
        floats = fl or bool(intr and intr.startswith("_CI")) or clobber.reads_st0(code, site)
        seen[c] = (kind, words, pops, floats)
        if intr:
            x87_op = intr.startswith(("_CI", "_ftol"))
            kind = "x87 stack operands" if x87_op else "import thunk (cdecl)"
            words = 0 if x87_op else "?"
            floats = intr.startswith("_CI")
            what = f"CRT `{intr}`: ffx_bridge::x87" if x87_op else f"import `{intr}`: platform equivalent"
        else:
            what = callee_state(w, c)
        label = intr if intr and c not in w.names else w.name(c)
        rows.append(f"| {label} | {c:#x} | {kind} | {words} | {'ST0' if floats else ''} | {what} |")
    indirect = [x for x in code if code[x].startswith("call") and not re.match(r"call\s+0x[0-9a-f]+$", code[x])]
    out += ["", "## Callees", ""]
    if rows:
        out += ["| name | addr | convention | stack words | float | state |", "|---|---|---|---|---|---|", *rows]
    else:
        out.append("none (a leaf)")
    for x in indirect:
        out.append(f"- indirect: `{annotate(w, code[x])}` at {x:#x}")

    # globals: relocations in the body and the dump's data refs
    targets = {}
    for r in w.image.relocs_in(a, a + size):
        v = w.image.dword(r)
        if v is not None and not a <= v < a + size and v not in seen:
            targets[v] = ""
    for g, acc in w.data_refs.get(a, {}).items():
        targets[g] = "".join(sorted(set(acc)))
    cookie = targets.pop(SECURITY_COOKIE, None) is not None
    if targets or cookie:
        out += ["", "## Globals and labels", ""]
    if cookie:
        out.append("- stack cookie (`__security_cookie`, `security_check_cookie`): nothing to port")
    if targets:
        for v in sorted(targets):
            d = w.describe(v) or ""
            sec = w.image.section(v) or "?"
            note = ""
            if v in w.sym and w.sym[v][2]:
                note = " — " + w.sym[v][2][:160]
            elif v in w.rows:
                note = f" — function pointer ({callee_state(w, v)})"
            elif sec == ".rdata" and (x := w.image.dword(v)) is not None:
                f32 = struct.unpack("<f", struct.pack("<I", x))[0]
                note = f" — const {x:#010x} = {f32:.9g}f"
                hi = w.image.dword(v + 4)
                if hi is not None and any(f"QWORD PTR ds:{v:#x}" in t for t in code.values()):
                    f64 = struct.unpack("<d", struct.pack("<II", x, hi))[0]
                    note = f" — const f64 {f64:.17g}"
            out.append(f"- `{v:#010x}` {sec} {targets[v] or '-'} {d}{note}")

    t = traps(w, a, code, calls)
    out += ["", "## Traps (heuristic linear scan: check against the asm)", ""]
    out += [f"- {x}" for x in t] or ["- none found"]

    out += ["", "## Ghidra C (~/ffx-work/decomp)", "", "```c", c_text.strip() or "(not in the dump)", "```"]
    out += ["", "## asm (objdump -M intel)", "", "```"]
    out += [f"{x:x}: {annotate(w, code[x])}" for x in sorted(code)]
    out.append("```")

    x87n = sum(1 for x in code.values() if x and x.split()[0].startswith("f"))
    if x87n >= 6:
        draft, err = lift(w, a, code, seen)
        out += ["", "## x87-lift draft: REFERENCE ONLY, rewrite by hand (brief: lifted code is never a port)", ""]
        if draft:
            out += ["```rust", draft.rstrip(), "```"]
        else:
            out.append(f"x87-lift lifted no section (first stop: `{err}`). It takes straight-line x87 code over "
                       "frame arguments, image constants and pushed words; lift a section by hand with "
                       "`stops` (tools/x87-lift.py docstring) if a draft helps.")

    out += ["", "## Test skeleton", "", test_skeleton(w, a, conv), ""]
    return "\n".join(out)


def resolve(w, key):
    try:
        a = int(key, 16)
    except ValueError:
        by = {n: x for x, (n, _) in w.rows.items()} | {n: x for x, n in w.names.items()}
        a = by.get(key)
    if a is None:
        sys.exit(f"{key}: no such function")
    if a not in w.rows:
        f = w.function_at(a)
        sys.exit(f"{key}: not a function start" + (f" (inside {w.name(f)} @ {f:#x})" if f else ""))
    return a


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("function", nargs="?")
    ap.add_argument("--out")
    ap.add_argument("--batch", metavar="WORKLIST_ARGS")
    ap.add_argument("--dir", default=str(WORK / "packets"))
    args = ap.parse_args()
    w = World()
    if args.batch is not None:
        r = subprocess.run([sys.executable, "-I", str(TOOLS / "worklist.py"), *shlex.split(args.batch)],
                           capture_output=True, text=True, check=True)
        d = pathlib.Path(args.dir).expanduser()
        d.mkdir(parents=True, exist_ok=True)
        for line in r.stdout.splitlines():
            if not line.startswith("0x"):
                continue
            a = int(line.split()[0], 16)
            p = d / f"0x{a:08x}_{w.name(a)}.md"
            p.write_text(packet(w, a), encoding="utf-8")
            print(p)
        return
    if not args.function:
        ap.error("give a function or --batch")
    text = packet(w, resolve(w, args.function))
    if args.out:
        pathlib.Path(args.out).write_text(text, encoding="utf-8")
    else:
        print(text)


if __name__ == "__main__":
    main()
