#!/usr/bin/env python3
"""Ports trivial FFX.exe functions automatically (docs/porting.md, "Trivial
functions").

Every unported game function of the dump is disassembled (objdump, Intel
syntax) and run through a symbolic evaluator that knows a small, exact set of
integer instructions (mov/movzx/movsx/lea, add/sub/and/or/xor/not/neg/inc/dec,
shifts and multiplies by constants, cmp/test followed by setcc, push/pop and
the MSVC frame, ret). The function must be straight-line code made of those
instructions only, tile its whole body (every byte covered, ending in its one
`ret`), take its inputs only from its stack arguments, `this` (ecx, thiscall)
or ecx/edx (fastcall), touch the stack only for its frame, and restore every
callee-saved register. What it leaves in eax must be defined in all 32 bits
(a `mov al, [x]` getter keeps the caller's upper bits: skipped), or eax must
be untouched (the shim then returns nothing and its thunk keeps eax).

The result of a match is a row: the loads in program order (temporaries),
the stores in program order, and the eax expression. Image addresses are told
from constants by the base relocations (a relocated field is an address, and
is rebased in the Rust interpreter). Rows are classified into patterns
(return constant, return global, store argument to field, bit set, ...) for
the report; anything else is left to the module agents.

Only named functions (a `function` line in symbols/ffx.sym) are ported;
unnamed matches are listed with --unnamed for the naming work.

Outputs (generated, do not edit by hand):
  crates/ffx-auto/src/table.rs      the rows (ffx_auto::ROWS)
  crates/ffx-hook/src/ported/auto.rs the `ported!` shims

A row whose parity test fails (crates/ffx-oracle/tests/auto.rs) is dropped:
tools/autoport-dropped.tsv lists it with the reason, and later runs skip it.

Usage:
  FFX_GAME_DIR=... tools/autoport.py            regenerate the outputs, print the report
  FFX_GAME_DIR=... tools/autoport.py --verify   also run the parity test (image and the
                                                snapshots in FFX_SNAPSHOTS, ':'-separated,
                                                default ~/ffx-work/snapshots/{field,battle}.snap),
                                                drop failing rows and regenerate until clean
  tools/autoport.py --unnamed                    list unnamed matches (address, pattern, suggestion)
  tools/autoport.py --rejects                    histogram of why candidates were skipped
"""
import argparse
import bisect
import collections
import os
import pathlib
import re
import struct
import subprocess
import sys
import tempfile

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

REPO = progress.REPO
WORK = progress.WORK
TABLE_RS = REPO / "crates/ffx-auto/src/table.rs"
AUTO_RS = REPO / "crates/ffx-hook/src/ported/auto.rs"
DROPPED = REPO / "tools/autoport-dropped.tsv"
MAX_BODY = 160  # bytes; the evaluator does not care, the patterns are small
MAX_INSNS = 40
BASE = 0x0040_0000
# Offsets from a pointer the parity test can serve (its blocks are 0x40000
# bytes with the pointer in the middle).
MAX_OFFSET = 0x1_f000


class Reject(Exception):
    pass


# ---------------------------------------------------------------- the image

def pe_relocations(exe):
    """Addresses (preferred base) of every HIGHLOW-relocated dword, and the
    image addresses those dwords hold."""
    data = exe.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3c)[0]
    nsec = struct.unpack_from("<H", data, pe + 6)[0]
    opt = pe + 24
    optsize = struct.unpack_from("<H", data, pe + 20)[0]
    reloc_rva, reloc_size = struct.unpack_from("<II", data, opt + 96 + 5 * 8)
    secs = []
    for i in range(nsec):
        s = pe + 24 + optsize + 40 * i
        vsize, va, rawsize, raw = struct.unpack_from("<IIII", data, s + 8)
        secs.append((va, max(vsize, rawsize), raw))

    def off(rva):
        for va, size, raw in secs:
            if va <= rva < va + size:
                return raw + rva - va
        raise ValueError(hex(rva))

    fields = {}
    p, end = off(reloc_rva), off(reloc_rva) + reloc_size
    while p + 8 <= end:
        page, size = struct.unpack_from("<II", data, p)
        if size < 8:
            break
        for k in range(8, size, 2):
            e = struct.unpack_from("<H", data, p + k)[0]
            if e >> 12 == 3:
                rva = page + (e & 0xfff)
                fields[BASE + rva] = struct.unpack_from("<I", data, off(rva))[0]
        p += size
    return fields


INSN = re.compile(r"^\s*([0-9a-f]+):\t([0-9a-f ]+?)\s*\t(.*)$")
BRANCH = re.compile(r"^(j\w+|call|loop\w*) (?:0x)?([0-9a-f]+)\b")


def disassemble(exe):
    """address -> (length, bytes, text) for the whole .text (one objdump run),
    and every direct branch as (target, source)."""
    out = subprocess.run(["objdump", "-d", "-w", "-M", "intel", str(exe)],
                         capture_output=True, text=True, check=True).stdout
    code, targets = {}, []
    for line in out.splitlines():
        m = INSN.match(line)
        if not m:
            continue
        raw = bytes.fromhex(m[2].replace(" ", ""))
        text = re.sub(r"\s+", " ", m[3].split("#")[0].strip())
        code[int(m[1], 16)] = (len(raw), raw, text)
        b = BRANCH.match(text)
        if b:
            targets.append((int(b[2], 16), int(m[1], 16)))
    return code, targets


# ------------------------------------------------------------- expressions
# Values are tuples:
#   ('c', v)            constant (u32)
#   ('arg', j)          stack argument slot j (dword at entry esp + 4 + 4j)
#   ('reg', r)          a register's value at entry (eax..edi, ebp)
#   ('img', a)          image address a (preferred base)
#   ('t', k, size)      the k-th load (zero-extended, size bytes)
#   ('bin', op, a, b)   op in BINOPS
#   ('un', op, a)       not, neg, sx8, sx16
#   ('sp', k)           entry esp + k (only in esp/ebp; never escapes)

M32 = 0xffff_ffff
MASK = {1: 0xff, 2: 0xffff, 4: M32}
BINOPS = {"add", "sub", "mul", "and", "or", "xor", "shl", "shr", "sar",
          "eq", "ne", "ltu", "leu", "gtu", "geu", "lt", "le", "gt", "ge"}


def s32(v):
    return v - (1 << 32) if v & 0x8000_0000 else v


def const_fold(op, a, b):
    if op == "add":
        return (a + b) & M32
    if op == "sub":
        return (a - b) & M32
    if op == "mul":
        return (a * b) & M32
    if op == "and":
        return a & b
    if op == "or":
        return a | b
    if op == "xor":
        return a ^ b
    if op == "shl":
        return (a << (b & 31)) & M32
    if op == "shr":
        return a >> (b & 31)
    if op == "sar":
        return (s32(a) >> (b & 31)) & M32
    cmp = {"eq": a == b, "ne": a != b, "ltu": a < b, "leu": a <= b, "gtu": a > b, "geu": a >= b,
           "lt": s32(a) < s32(b), "le": s32(a) <= s32(b), "gt": s32(a) > s32(b), "ge": s32(a) >= s32(b)}
    return int(cmp[op])


def bound(x):
    """An upper bound of the bits x can have set."""
    if x[0] == "c":
        return x[1]
    if x[0] == "t":
        return MASK[x[2]]
    if x[0] == "bin":
        op, a, b = x[1], x[2], x[3]
        if op == "and":
            return bound(a) & bound(b)
        if op in ("or", "xor"):
            return bound(a) | bound(b)
        if op == "shr" and b[0] == "c":
            return bound(a) >> (b[1] & 31)
        if op in ("eq", "ne", "ltu", "leu", "gtu", "geu", "lt", "le", "gt", "ge"):
            return 1
    return M32


def B(op, a, b):
    if a[0] == "sp" or b[0] == "sp":
        raise Reject("stack address escapes")
    if a[0] == "c" and b[0] == "c":
        return ("c", const_fold(op, a[1], b[1]))
    if op in ("add", "or", "xor", "mul", "and", "eq", "ne") and a[0] == "c":
        a, b = b, a  # constants on the right
    if b[0] == "c":
        v = b[1]
        if op in ("add", "sub", "or", "xor", "shl", "shr", "sar") and v == 0:
            return a
        if op == "mul" and v == 1:
            return a
        if op == "mul" and v == 0:
            return ("c", 0)
        if op == "and":
            if v == 0:
                return ("c", 0)
            if bound(a) & ~v & M32 == 0:
                return a
            if a[0] == "bin" and a[1] == "and" and a[3][0] == "c":
                return B("and", a[2], ("c", a[3][1] & v))
            if a[0] == "bin" and a[1] == "or":
                # (x | y) & m with one side already inside or outside m
                x, y = a[2], a[3]
                if bound(x) & v == 0:
                    return B("and", y, b)
                if bound(y) & v == 0:
                    return B("and", x, b)
        if op == "or" and v == M32:
            return ("c", M32)
        if op == "add" and a[0] == "bin" and a[1] == "add" and a[3][0] == "c":
            return B("add", a[2], ("c", (a[3][1] + v) & M32))
        if op == "sub":
            return B("add", a, ("c", (-v) & M32))
    if op == "or" and a == b:
        return a
    if op == "and" and a == b:
        return a
    return ("bin", op, a, b)


def U(op, a):
    if a[0] == "sp":
        raise Reject("stack address escapes")
    if a[0] == "c":
        v = a[1]
        if op == "not":
            return ("c", ~v & M32)
        if op == "neg":
            return ("c", -v & M32)
        if op == "sx8":
            return ("c", (v & 0xff) - 0x100 & M32 if v & 0x80 else v & 0xff)
        if op == "sx16":
            return ("c", (v & 0xffff) - 0x10000 & M32 if v & 0x8000 else v & 0xffff)
    return ("un", op, a)


def atoms(x, kind):
    out = set()

    def walk(v):
        if v[0] == kind:
            out.add(v)
        elif v[0] == "bin":
            walk(v[2])
            walk(v[3])
        elif v[0] == "un":
            walk(v[2])
    walk(x)
    return out


def has(x, pred):
    if pred(x):
        return True
    if x[0] == "bin":
        return has(x[2], pred) or has(x[3], pred)
    if x[0] == "un":
        return has(x[2], pred)
    return False


# ---------------------------------------------------------------- operands

R32 = ["eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi"]
SUB = {"al": ("eax", 0, 1), "cl": ("ecx", 0, 1), "dl": ("edx", 0, 1), "bl": ("ebx", 0, 1),
       "ah": ("eax", 8, 1), "ch": ("ecx", 8, 1), "dh": ("edx", 8, 1), "bh": ("ebx", 8, 1),
       "ax": ("eax", 0, 2), "cx": ("ecx", 0, 2), "dx": ("edx", 0, 2), "bx": ("ebx", 0, 2),
       "si": ("esi", 0, 2), "di": ("edi", 0, 2), "bp": ("ebp", 0, 2), "sp": ("esp", 0, 2)}
for r in R32:
    SUB[r] = (r, 0, 4)
PTR = {"BYTE": 1, "WORD": 2, "DWORD": 4}
MEM = re.compile(r"^(?:(BYTE|WORD|DWORD|QWORD|TBYTE|XMMWORD|FWORD) PTR )?(?:(ds|es|ss|cs|fs|gs):)?(\[[^\]]*\]|0x[0-9a-f]+)$")


def num(s):
    s = s.strip()
    neg = s.startswith("-")
    s = s.lstrip("+-")
    v = int(s, 16) if s.startswith("0x") else int(s)
    return -v if neg else v


class Fn:
    """The symbolic state of one function."""

    def __init__(self, relocs):
        self.regs = {r: ("reg", r) for r in R32}
        self.regs["esp"] = ("sp", 0)
        self.stack = {}
        self.flags = None
        self.steps = []
        self.tmps = 0
        self.relocs = relocs  # relocated values within this instruction
        self.used_relocs = set()

    # registers
    def get(self, name):
        r, shift, size = SUB[name]
        v = self.regs[r]
        if size == 4:
            return v
        if v[0] == "sp":
            raise Reject("partial stack register")
        if shift:
            v = B("shr", v, ("c", shift))
        return B("and", v, ("c", MASK[size]))

    def put(self, name, v):
        r, shift, size = SUB[name]
        if size == 4:
            self.regs[r] = v
            return
        old = self.regs[r]
        if old[0] == "sp":
            raise Reject("partial stack register")
        m = MASK[size] << shift
        lo = B("and", B("shl", v, ("c", shift)), ("c", m))
        self.regs[r] = B("or", B("and", old, ("c", ~m & M32)), lo)

    # immediates and displacements: relocated fields are image addresses
    def absolute(self, v):
        v &= M32
        if v in self.relocs:
            self.used_relocs.add(v)
            return ("img", v)
        return ("c", v)

    def address(self, inner):
        """The address expression of `[...]` (or of a bare ds:0x...)."""
        if inner.startswith("0x"):
            a = self.absolute(num(inner))
            if a[0] != "img":
                raise Reject("absolute address not relocated")
            return a
        terms = re.findall(r"([+-]?)([^+-]+)", inner[1:-1])
        base, idx, disp = [], [], 0
        for sign, t in terms:
            t = t.strip()
            if "*" in t:
                r, s = t.split("*")
                idx.append((r, int(s)))
            elif t in SUB:
                base.append(t)
            else:
                disp += -num(t) if sign == "-" else num(t)
        vals = [self.get(r) for r in base]
        sp = [v for v in vals if v[0] == "sp"]
        if sp:
            if len(vals) != 1 or idx:
                raise Reject("indexed stack access")
            return ("sp", sp[0][1] + disp)
        acc = None
        for v in vals:
            acc = v if acc is None else B("add", acc, v)
        for r, s in idx:
            v = B("mul", self.get(r), ("c", s))
            acc = v if acc is None else B("add", acc, v)
        d = self.absolute(disp) if (disp & M32) in self.relocs else ("c", disp & M32)
        return d if acc is None else B("add", acc, d)

    def load(self, addr, size):
        if addr[0] == "sp":
            k = addr[1]
            if k in self.stack and size == 4:
                return self.stack[k]
            if k in self.stack:
                raise Reject("narrow read of a stack local")
            if k < 4:
                raise Reject("reads the return address or an unwritten local")
            j, b = divmod(k - 4, 4)
            if b + size > 4:
                raise Reject("unaligned argument read")
            v = ("arg", j)
            if b:
                v = B("shr", v, ("c", 8 * b))
            return B("and", v, ("c", MASK[size])) if size < 4 or b else v
        k = self.tmps
        self.tmps += 1
        self.steps.append(("load", k, size, addr))
        return ("t", k, size)

    def store(self, addr, size, v):
        if addr[0] == "sp":
            k = addr[1]
            if size != 4 or k % 4:
                raise Reject("narrow stack write")
            if k == 0:
                raise Reject("writes the return address")
            self.stack[k] = v
            return
        if v[0] == "sp":
            raise Reject("stack address escapes")
        self.steps.append(("store", size, addr, v))

    # operands: ('reg', name) | ('mem', addr, size) | ('imm', value)
    def operand(self, text, size_hint=None):
        text = text.strip()
        if text in SUB:
            return ("r", text, SUB[text][2])
        m = MEM.match(text)
        if m and (m[1] or m[2] or m[3].startswith("[")):
            if m[2] in ("fs", "gs") or (m[1] and m[1] not in PTR):
                raise Reject(f"operand {text}")
            size = PTR[m[1]] if m[1] else size_hint
            return ("m", self.address(m[3]), size)
        if re.match(r"^-?(0x[0-9a-f]+|\d+)$", text):
            return ("i", num(text) & M32, size_hint)
        raise Reject(f"operand {text}")

    def read(self, op, size=None):
        kind = op[0]
        if kind == "r":
            return self.get(op[1])
        if kind == "m":
            return self.load(op[1], op[2] or size)
        return self.absolute(op[1])

    def write(self, op, v):
        if op[0] == "r":
            self.put(op[1], v)
        elif op[0] == "m":
            self.store(op[1], op[2], v)
        else:
            raise Reject("write to an immediate")


SETCC = {"e": "e", "z": "e", "ne": "ne", "nz": "ne", "b": "b", "c": "b", "nae": "b", "ae": "ae",
         "nb": "ae", "nc": "ae", "a": "a", "nbe": "a", "be": "be", "na": "be", "l": "l", "nge": "l",
         "ge": "ge", "nl": "ge", "g": "g", "nle": "g", "le": "le", "ng": "le", "s": "s", "ns": "ns"}
CMP = {"e": "eq", "ne": "ne", "b": "ltu", "ae": "geu", "a": "gtu", "be": "leu", "l": "lt", "ge": "ge",
       "g": "gt", "le": "le"}
NOPS = {"nop", "xchg ax,ax", "lea esi,[esi+0x0]", "lea edi,[edi+0x0]", "lea esi,[esi+eiz*1+0x0]",
        "lea edi,[edi+eiz*1+0x0]", "lea esp,[esp+0x0]", "mov edi,edi"}


def condition(fn, cc):
    f = fn.flags
    if f is None:
        raise Reject("setcc without cmp/test")
    kind, a, b, size = f
    if kind == "cmp":
        if cc in ("s", "ns"):
            raise Reject("sign of a compare")
        op = CMP[cc]
        if op in ("lt", "le", "gt", "ge") and size < 4:
            sx = "sx8" if size == 1 else "sx16"
            a, b = U(sx, a), U(sx, b)
        return B(op, a, b)
    v = B("and", a, b)
    if cc == "e":
        return B("eq", v, ("c", 0))
    if cc == "ne":
        return B("ne", v, ("c", 0))
    if cc in ("s", "ns"):
        bit = B("and", v, ("c", 0x80 << 8 * (size - 1)))
        return B("ne" if cc == "s" else "eq", bit, ("c", 0))
    raise Reject(f"set{cc} after test")


def step(fn, text):
    """One instruction; returns the bytes `ret` pops when it is a ret."""
    if text in NOPS:
        return None
    parts = text.split(" ", 1)
    mn = parts[0]
    ops = [o for o in re.split(r",(?![^\[]*\])", parts[1])] if len(parts) > 1 else []
    if mn == "ret":
        return num(ops[0]) if ops else 0
    if mn == "push":
        v = fn.read(fn.operand(ops[0], 4), 4)
        sp = fn.regs["esp"]
        fn.regs["esp"] = ("sp", sp[1] - 4)
        fn.store(("sp", sp[1] - 4), 4, v)
        return None
    if mn == "pop":
        sp = fn.regs["esp"]
        if sp[1] not in fn.stack:
            raise Reject("pop of an unpushed slot")
        v = fn.stack.pop(sp[1])
        fn.regs["esp"] = ("sp", sp[1] + 4)
        dst = fn.operand(ops[0], 4)
        if dst[0] != "r" or dst[2] != 4:
            raise Reject("pop to memory")
        fn.regs[dst[1]] = v  # restoring a callee-saved register is fine
        return None
    if mn == "leave":
        fn.regs["esp"] = fn.regs["ebp"]
        return step(fn, "pop ebp")
    if mn in ("mov", "movzx", "movsx"):
        dst = fn.operand(ops[0])
        src = fn.operand(ops[1], dst[2] if mn == "mov" else None)
        if dst[0] == "r" and dst[1] == "esp":
            v = fn.read(src)
            if v[0] != "sp":
                raise Reject("esp from a non-stack value")
            # drop locals below the new esp
            fn.stack = {k: x for k, x in fn.stack.items() if k >= v[1]}
            fn.regs["esp"] = v
            return None
        if dst[0] == "r" and dst[1] == "ebp" and fn.regs["esp"][0] == "sp" and src == ("r", "esp", 4):
            fn.regs["ebp"] = fn.regs["esp"]
            return None
        if mn == "mov" and dst[0] == "m" and dst[2] is None:
            dst = ("m", dst[1], src[2])
        size = dst[2] if mn == "mov" else src[2]
        if size is None:
            raise Reject("operand size")
        v = fn.read(src, size)
        if mn == "movsx":
            v = U("sx8" if size == 1 else "sx16", v)
        fn.write(dst, v)
        return None
    if mn == "lea":
        dst = fn.operand(ops[0])
        m = re.match(r"^\[.*\]$", ops[1].strip())
        if not m or dst[0] != "r":
            raise Reject("lea form")
        a = fn.address(ops[1].strip())
        if dst[1] == "esp":
            if a[0] != "sp":
                raise Reject("esp from a non-stack value")
            fn.regs["esp"] = a
            return None
        if a[0] == "sp":
            if dst[1] == "ebp" or dst[1] == "esp":
                fn.regs[dst[1]] = a
                return None
            raise Reject("address of a stack slot")
        if dst[2] != 4:
            raise Reject("narrow lea")
        fn.write(dst, a)
        return None
    if mn in ("add", "sub", "and", "or", "xor", "cmp", "test"):
        dst = fn.operand(ops[0])
        src = fn.operand(ops[1], dst[2])
        if dst[0] == "m" and dst[2] is None:
            dst = ("m", dst[1], src[2])
        size = dst[2]
        if mn in ("xor", "sub") and dst == src and dst[0] == "r":
            v = ("c", 0)
            fn.flags = ("cmp", v, v, size) if mn == "sub" else ("test", v, v, size)
            fn.write(dst, v)
            return None
        if dst[0] == "r" and dst[1] == "esp" and mn in ("add", "sub") and src[0] == "i":
            k = fn.regs["esp"][1] + (src[1] if mn == "add" else -src[1])
            fn.stack = {x: y for x, y in fn.stack.items() if x >= k}
            fn.regs["esp"] = ("sp", s32(k & M32))
            fn.flags = None
            return None
        a, b = fn.read(dst, size), fn.read(src, size)
        if mn == "cmp":
            fn.flags = ("cmp", a, b, size)
            return None
        if mn == "test":
            fn.flags = ("test", a, b, size)
            return None
        v = B("and", B(mn, a, b), ("c", MASK[size]))
        if mn == "sub":
            fn.flags = ("cmp", a, b, size)
        elif mn in ("and", "or", "xor"):
            fn.flags = ("test", v, ("c", MASK[size]), size)
        else:
            fn.flags = None
        fn.write(dst, v)
        return None
    if mn in ("inc", "dec", "not", "neg"):
        dst = fn.operand(ops[0])
        size = dst[2]
        if size is None:
            raise Reject("operand size")
        a = fn.read(dst, size)
        if mn == "inc":
            v = B("add", a, ("c", 1))
        elif mn == "dec":
            v = B("sub", a, ("c", 1))
        else:
            v = U(mn, a)
        v = B("and", v, ("c", MASK[size]))
        if mn != "not":  # not leaves the flags alone
            fn.flags = None
        fn.write(dst, v)
        return None
    if mn in ("shl", "sal", "shr", "sar"):
        dst = fn.operand(ops[0])
        size = dst[2]
        if size is None or len(ops) < 2:
            raise Reject("shift form")
        cnt = fn.operand(ops[1], 1)
        if cnt[0] != "i":
            raise Reject("shift by a register")
        n = cnt[1] & 31
        a = fn.read(dst, size)
        if mn == "sar" and size < 4:
            a = U("sx8" if size == 1 else "sx16", a)
        op = {"shl": "shl", "sal": "shl", "shr": "shr", "sar": "sar"}[mn]
        v = B("and", B(op, a, ("c", n)), ("c", MASK[size]))
        fn.flags = None
        fn.write(dst, v)
        return None
    if mn == "imul":
        if len(ops) == 3:
            dst = fn.operand(ops[0])
            a = fn.read(fn.operand(ops[1], dst[2]), dst[2])
            b = fn.read(fn.operand(ops[2], dst[2]), dst[2])
        elif len(ops) == 2:
            dst = fn.operand(ops[0])
            a = fn.read(dst, dst[2])
            b = fn.read(fn.operand(ops[1], dst[2]), dst[2])
        else:
            raise Reject("one-operand imul")
        if dst[0] != "r" or dst[2] != 4:
            raise Reject("imul form")
        fn.flags = None
        fn.write(dst, B("mul", a, b))
        return None
    if mn.startswith("set") and mn[3:] in SETCC:
        dst = fn.operand(ops[0], 1)
        if dst[2] != 1:
            raise Reject("setcc size")
        fn.write(dst, condition(fn, SETCC[mn[3:]]))
        return None
    raise Reject(f"instruction {mn}")


def evaluate(insns, relocs_at):
    """The row of a function's instruction list [(addr, len, raw, text)]."""
    fn = Fn(set())
    pops = None
    for i, (a, n, raw, text) in enumerate(insns):
        if pops is not None:
            raise Reject("code after ret")
        fn.relocs = {v for f, v in relocs_at.items() if a <= f <= a + n - 4}
        fn.used_relocs = set()
        pops = step(fn, text)
        if fn.relocs - fn.used_relocs:
            raise Reject("relocated field not understood")
    if pops is None:
        raise Reject("no ret at the end")
    if fn.regs["esp"] != ("sp", 0):
        raise Reject("unbalanced stack")
    for r in ("ebx", "esi", "edi", "ebp"):
        if fn.regs[r] != ("reg", r):
            raise Reject(f"{r} not restored")
    return fn, pops


def row_of(fn, pops):
    """Convention, arguments, steps and result; raises Reject."""
    eax = fn.regs["eax"]
    result = None if eax == ("reg", "eax") else eax
    exprs = [s[3] for s in fn.steps] + [s[2] for s in fn.steps if s[0] == "store"]
    if result is not None:
        exprs.append(result)
    inputs = set()
    for e in exprs:
        inputs |= {x[1] for x in atoms(e, "reg")}
    if result is not None and "eax" in {x[1] for x in atoms(result, "reg")}:
        raise Reject("eax only partly defined at ret")
    if has(result or ("c", 0), lambda x: x[0] == "sp"):
        raise Reject("returns a stack address")
    if inputs - {"ecx", "edx"}:
        raise Reject(f"reads {'/'.join(sorted(inputs - {'ecx', 'edx'}))} at entry")
    slots = set()
    for e in exprs:
        slots |= {x[1] for x in atoms(e, "arg")}
    if pops % 4:
        raise Reject("odd ret")
    if "edx" in inputs:
        conv, regs = "fastcall", ["ecx", "edx"]
    elif "ecx" in inputs:
        conv, regs = "thiscall", ["ecx"]
    else:
        conv, regs = ("stdcall" if pops else "cdecl"), []
    if pops:
        nstack = pops // 4
        if slots and max(slots) >= nstack:
            raise Reject("reads beyond the popped arguments")
    else:
        if regs and slots:
            raise Reject("register arguments with caller-popped stack arguments")
        nstack = max(slots) + 1 if slots else 0
    if fn.tmps > 64:
        raise Reject("too many loads")
    params = [("reg", r) for r in regs] + [("arg", j) for j in range(nstack)]
    index = {p: i for i, p in enumerate(params)}

    def rename(x):
        if x[0] in ("reg", "arg"):
            return ("p", index[x])
        if x[0] == "bin":
            return ("bin", x[1], rename(x[2]), rename(x[3]))
        if x[0] == "un":
            return ("un", x[1], rename(x[2]))
        return x
    steps = []
    for s in fn.steps:
        if s[0] == "load":
            steps.append(("load", s[1], s[2], rename(s[3])))
        else:
            steps.append(("store", s[1], rename(s[2]), rename(s[3])))
    res = rename(result) if result is not None else None
    return {"conv": conv, "args": len(params), "pops": pops, "steps": steps, "result": res}


# ----------------------------------------------------- test-input analysis

def linear(x):
    """x as [(term, coefficient)] + constant: terms are p/t/img leaves or
    any other subexpression, taken whole."""
    if x[0] == "c":
        return [], x[1]
    if x[0] in ("p", "t", "img"):
        return [(x, 1)], 0
    if x[0] == "bin" and x[1] == "add":
        a, b = linear(x[2]), linear(x[3])
        if a is None or b is None:
            return None
        return a[0] + b[0], (a[1] + b[1]) & M32
    if x[0] == "bin" and x[1] == "sub":
        a, b = linear(x[2]), linear(x[3])
        return a[0] + [(t, -c & M32) for t, c in b[0]], (a[1] - b[1]) & M32
    if x[0] == "bin" and x[1] in ("mul", "shl") and x[3][0] == "c":
        a = linear(x[2])
        k = x[3][1] if x[1] == "mul" else 1 << x[3][1]
        return [(t, c * k & M32) for t, c in a[0]], a[1] * k & M32
    return [(x, 1)], 0  # anything else is one opaque term


def kinds(row):
    """How the parity test must choose each argument and load: 'value'
    (anything), 'pointer' (to a block it allocates) or 'index' (small).
    Raises Reject when an address is not a pointer or image address plus
    small indices and offsets."""
    want = {}

    def mark(atom, k):
        if atom[0] == "img":
            return
        old = want.get(atom)
        if old and old != k:
            raise Reject("an input is both a pointer and an index")
        want[atom] = k

    def leaves(x, out):
        if x[0] in ("p", "t"):
            out.add(x)
        elif x[0] == "bin":
            leaves(x[2], out)
            leaves(x[3], out)
        elif x[0] == "un":
            leaves(x[2], out)
        return out

    addrs = [s[3] for s in row["steps"] if s[0] == "load"] + \
            [s[2] for s in row["steps"] if s[0] == "store"]
    for a in addrs:
        terms, off = linear(a)
        bases = [t for t, c in terms if c == 1 and t[0] in ("p", "t")]
        imgs = [t for t, c in terms if t[0] == "img"]
        if len(imgs) > 1:
            raise Reject("two image addresses in one address")
        if imgs:
            ptr = None
            if any(c != 1 for t, c in terms if t[0] == "img"):
                raise Reject("scaled image address")
        elif len(bases) == 1:
            ptr = bases[0]
        elif not bases and not terms:
            raise Reject("absolute non-image address")
        else:
            raise Reject("no single base pointer")
        if ptr is not None:
            mark(ptr, "pointer")
            if abs(s32(off)) > MAX_OFFSET:
                raise Reject("offset too large for the test blocks")
        for t, c in terms:
            if t == ptr or t[0] == "img":
                continue
            if abs(s32(c)) * 8 > MAX_OFFSET:
                raise Reject("index scale too large")
            for leaf in leaves(t, set()):
                mark(leaf, "index")
    for s in row["steps"]:
        if s[0] == "load":
            t = ("t", s[1], s[2])
            if want.get(t) == "pointer" and s[2] != 4:
                raise Reject("narrow pointer")
    args = [want.get(("p", i), "value") for i in range(row["args"])]
    tmps = {s[1]: want.get(("t", s[1], s[2]), "value") for s in row["steps"] if s[0] == "load"}
    return args, tmps


# ------------------------------------------------------------ the patterns

def pattern(row):
    """A short pattern name for the report."""
    steps, res = row["steps"], row["result"]
    loads = [s for s in steps if s[0] == "load"]
    stores = [s for s in steps if s[0] == "store"]

    def where(a):
        terms = linear(a)[0]
        if any(t[0] == "img" for t, _ in terms):
            return "table" if len(terms) > 1 else "global"
        if any(t[0] == "t" for t, _ in terms):
            return "deref"
        return "field"

    def plain(x):
        return x is None or x[0] in ("c", "p", "img") or (x[0] == "bin" and x[1] == "add" and
                                                           x[2][0] in ("p", "img") and x[3][0] == "c")
    if not stores:
        if not loads:
            if res is None:
                return "nothing"
            if res[0] == "c":
                return "return constant"
            if res[0] == "img" or (res[0] == "bin" and res[2][0] == "img"):
                return "return address of global"
            if res[0] == "p":
                return "pass-through"
            if plain(res):
                return "return argument + offset"
            return "arithmetic on arguments"
        last = loads[-1]
        w = {1: "u8", 2: "u16", 4: "u32"}[last[2]]
        if res is not None and res[0] == "t" and len(loads) == 1:
            return f"return {where(last[3])} {w}"
        if res is not None and res[0] == "un" and res[2][0] == "t" and len(loads) == 1:
            return f"return {where(last[3])} {res[1]}"
        if res is not None and res[0] == "t":
            return f"return {where(last[3])} {w} (pointer chain)"
        if res is not None and res[1] in ("eq", "ne", "ltu", "leu", "gtu", "geu", "lt", "le", "gt", "ge"):
            return "compare"
        if res is not None and res[0] == "bin" and res[1] in ("and", "shr") and not has(res, lambda x: x[0] == "p"):
            return f"bit test of {where(last[3])}"
        if res is not None and res[0] == "bin" and res[1] == "add" and where(last[3]) in ("field", "deref") \
                and res[3][0] in ("c", "p", "t"):
            return "return address of a field's target"
        return "load and compute"
    value_kinds = set()
    for s in stores:
        v = s[3]
        loaded = [x for x in loads if ("t", x[1], x[2]) == v]
        rmw = loaded and loaded[0][3] == s[2]
        if v[0] == "c":
            value_kinds.add("constant")
        elif v[0] == "p" or (v[0] == "bin" and v[1] == "and" and v[2][0] == "p"):
            value_kinds.add("argument")
        elif v[0] == "t":
            value_kinds.add("copy")
        elif v[0] == "bin" and v[1] in ("or", "and") and v[2][0] == "t" and v[3][0] == "c" and \
                any(x[3] == s[2] for x in loads if ("t", x[1], x[2]) == v[2]):
            value_kinds.add("bit set" if v[1] == "or" else "bit clear")
        elif v[0] == "img":
            value_kinds.add("address")
        elif rmw:
            value_kinds.add("update")
        else:
            value_kinds.add("computed")
    targets = "/".join(sorted({where(s[2]) for s in stores}))
    kinds_ = "/".join(sorted(value_kinds))
    if len(stores) == 1:
        return f"store {kinds_} to {targets}"
    if value_kinds <= {"constant", "argument", "address"}:
        return f"several stores: set {targets} (constants, arguments, addresses)"
    if value_kinds <= {"constant", "argument", "address", "copy"}:
        return f"several stores: copy into {targets}"
    return f"several stores: computed, into {targets}"


# --------------------------------------------------------------- matching

def hand_hooked():
    out = {}
    for f in sorted((REPO / "crates/ffx-hook/src").glob("*.rs")):
        for name, addr in re.findall(r'name: "(\w+)",\s*address: 0x([0-9a-f_]+),', f.read_text(encoding="utf-8")):
            out[int(addr.replace("_", ""), 16)] = name
    return out


def dropped():
    out = {}
    if DROPPED.exists():
        for line in DROPPED.read_text(encoding="utf-8").splitlines():
            if line and not line.startswith("#"):
                a, reason = line.split("\t", 1)
                out[int(a, 16)] = reason
    return out


def match_all(exe):
    rows = progress.functions()
    names = progress.symbols()
    # ported by the module agents or hooked by hand (never by our own auto.rs)
    taken = (set(progress.hooks()) - set(progress.ported())) | others_ported() | set(hand_hooked())
    module = progress.placement(rows, names)
    relocs = pe_relocations(exe)
    code, targets = disassemble(exe)
    # (address referred to, where from): branches, relocated pointers, Ghidra's data references
    interior = targets + [(v, f) for f, v in relocs.items()]
    for line in (WORK / "decomp/data_refs.tsv").read_text(encoding="utf-8").splitlines()[1:]:
        f, x = line.split("\t")[:2]
        interior.append((int(x, 16), int(f, 16)))
    interior.sort()
    interior_keys = [t for t, _ in interior]
    reloc_fields = sorted(relocs)

    def within(keys, lo, hi):
        """Keys k with lo <= k < hi."""
        return keys[bisect.bisect_left(keys, lo):bisect.bisect_left(keys, hi)]
    drop = dropped()
    matched, rejects, examples = [], collections.Counter(), {}
    for a, gname, size in rows:
        if progress.KIND[module[a]] != "game" or a in taken:
            continue
        if size > MAX_BODY:
            continue
        why = None
        try:
            if size < 5:
                raise Reject("under five bytes (no room for the jump)")
            lo, hi = bisect.bisect_left(interior_keys, a + 1), bisect.bisect_left(interior_keys, a + size)
            if any(not a <= src < a + size for _, src in interior[lo:hi]):
                raise Reject("something jumps or points into its body")
            insns, p = [], a
            while p < a + size:
                if p not in code:
                    raise Reject("disassembly does not tile the body")
                n, raw, text = code[p]
                insns.append((p, n, raw, text))
                p += n
            if p != a + size:
                raise Reject("disassembly does not tile the body")
            if len(insns) > MAX_INSNS:
                raise Reject("too many instructions")
            local_relocs = {f: relocs[f] for f in within(reloc_fields, a, a + size)}
            fn, pops = evaluate(insns, local_relocs)
            row = row_of(fn, pops)
            row["kinds"] = kinds(row)
            if a in drop:
                raise Reject(f"dropped by the parity test: {drop[a]}")
        except Reject as e:
            why = str(e)
        if why:
            key = re.sub(r"0x[0-9a-f]+", "N", why)
            rejects[key] += 1
            examples.setdefault(key, a)
            continue
        row.update(address=a, size=size, name=names.get(a), module=module[a], pattern=pattern(row),
                   text=[t for _, _, _, t in insns])
        matched.append(row)
    return matched, rejects, examples


def others_ported():
    """Addresses the other `ported!` files (not auto.rs) replace."""
    out = set()
    for f in sorted((REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
        if f != AUTO_RS:
            out |= {int(a.replace("_", ""), 16) for a in
                    re.findall(r"^\s*\w+ @ 0x([0-9a-f_]+) =>", f.read_text(encoding="utf-8"), re.M)}
    return out


# ------------------------------------------------------------- generation

def lit(v, width=0):
    """A hex literal with `_` every four digits (clippy's unreadable_literal)."""
    h = f"{v:0{width}x}"
    groups = []
    while len(h) > 4:
        groups.insert(0, h[-4:])
        h = h[:-4]
    groups.insert(0, h)
    return "0x" + "_".join(groups)


def rs_val(x):
    k = x[0]
    if k == "c":
        return f"Val::Const({lit(x[1])})"
    if k == "p":
        return f"Val::Arg({x[1]})"
    if k == "img":
        return f"Val::Image({lit(x[1], 8)})"
    if k == "t":
        return f"Val::Tmp({x[1]})"
    if k == "bin":
        return f"Val::Bin(Bin::{x[1].capitalize()}, &{rs_val(x[2])}, &{rs_val(x[3])})"
    if k == "un":
        return f"Val::Un(Un::{x[1].capitalize()}, &{rs_val(x[2])})"
    raise ValueError(x)


def c_val(x):
    """The row's expression in C-like notation, for the doc comments."""
    k = x[0]
    if k == "c":
        return f"{x[1]:#x}"
    if k == "p":
        return f"a{x[1]}"
    if k == "img":
        return f"&{x[1]:#010x}"
    if k == "t":
        return f"t{x[1]}"
    sym = {"add": "+", "sub": "-", "mul": "*", "and": "&", "or": "|", "xor": "^", "shl": "<<", "shr": ">>",
           "sar": ">>s", "eq": "==", "ne": "!=", "ltu": "<u", "leu": "<=u", "gtu": ">u", "geu": ">=u",
           "lt": "<s", "le": "<=s", "gt": ">s", "ge": ">=s"}
    if k == "bin":
        return f"({c_val(x[2])} {sym[x[1]]} {c_val(x[3])})"
    return f"{x[1]}({c_val(x[2])})"


def describe(row):
    parts = []
    for s in row["steps"]:
        if s[0] == "load":
            parts.append(f"t{s[1]} = u{8 * s[2]}[{c_val(s[3])}]")
        else:
            parts.append(f"u{8 * s[1]}[{c_val(s[2])}] = {c_val(s[3])}")
    if row["result"] is not None:
        parts.append(f"return {c_val(row['result'])}")
    return "; ".join(parts) or "nothing"


CONV_RS = {"cdecl": "Cdecl", "stdcall": "Stdcall", "thiscall": "Thiscall", "fastcall": "Fastcall"}
KIND_RS = {"value": "Kind::Value", "pointer": "Kind::Pointer", "index": "Kind::Index"}


def generate(rows, everything=False):
    """Writes the table and the shims. With `everything`, the table also has
    the unnamed matches (to test the matcher itself; not for committing)."""
    named = sorted((r for r in rows if r["name"]), key=lambda r: r["address"])
    tabled = sorted(rows, key=lambda r: r["address"]) if everything else named
    t = ["// Generated by tools/autoport.py: do not edit. One row per trivial function",
         "// ported automatically (docs/porting.md, \"Trivial functions\").",
         "",
         "use crate::{Bin, Conv, Kind, Row, Step, Un, Val};",
         ""]
    for r in tabled:
        a = r["address"]
        t.append(f"/// `{r['name'] or 'unnamed'}` ({r['pattern']}): `{describe(r)}`.")
        t.append(f"pub static R_{a:08X}: Row = Row {{")
        t.append(f"    address: {lit(a, 8)},")
        t.append(f"    conv: Conv::{CONV_RS[r['conv']]},")
        t.append(f"    args: &[{', '.join(KIND_RS[k] for k in r['kinds'][0])}],")
        steps = []
        for s in r["steps"]:
            if s[0] == "load":
                steps.append(f"Step::Load {{ tmp: {s[1]}, size: {s[2]}, addr: {rs_val(s[3])}, "
                             f"kind: {KIND_RS[r['kinds'][1][s[1]]]} }}")
            else:
                steps.append(f"Step::Store {{ size: {s[1]}, addr: {rs_val(s[2])}, value: {rs_val(s[3])} }}")
        t.append("    steps: &[" + "".join(f"\n        {s}," for s in steps) + ("\n    ]," if steps else "],"))
        res = "None" if r["result"] is None else f"Some({rs_val(r['result'])})"
        t.append(f"    result: {res},")
        t.append("};")
    t.append("")
    t.append("/// Every row, by address.")
    t.append(f"pub static ROWS: [&Row; {len(tabled)}] = [")
    t.extend(f"    &R_{r['address']:08X}," for r in tabled)
    t.append("];")
    TABLE_RS.write_text("\n".join(t) + "\n", encoding="utf-8")

    h = ["//! Generated by tools/autoport.py: do not edit. Trivial functions ported",
         "//! automatically: each shim runs its row of `ffx_auto::ROWS` (the",
         "//! interpreter in `ffx_auto`), checked against the original by",
         "//! `crates/ffx-oracle/tests/auto.rs`.",
         "",
         "use ffx_auto::table as t;",
         "",
         "use super::{mem, ported};",
         "",
         "ported! {"]
    abi = {"cdecl": "C", "stdcall": "stdcall", "thiscall": "thiscall", "fastcall": "fastcall"}
    for r in named:
        a = r["address"]
        args = ", ".join(f"a{i}: u32" for i in range(r["args"]))
        call_args = ", ".join(f"a{i}" for i in range(r["args"]))
        ret = " -> u32" if r["result"] is not None else ""
        f = "run" if r["result"] is None else "call"
        body = f"ffx_auto::{f}(&mut mem(), &t::R_{a:08X}, &[{call_args}])"
        if r["result"] is None:
            body += ";"
        h.append(f"    /// {r['pattern']}: `{describe(r)}`.")
        h.append(f"    {r['name']} @ {lit(a, 8)} => extern \"{abi[r['conv']]}\" fn "
                 f"auto_{a:08x}({args}){ret} {{")
        h.append(f"        {body}")
        h.append("    }")
    h.append("}")
    AUTO_RS.write_text("\n".join(h) + "\n", encoding="utf-8")
    subprocess.run(["rustfmt", "--edition", "2024", str(TABLE_RS), str(AUTO_RS)], check=True)
    return named


def report(rows):
    named = [r for r in rows if r["name"]]
    by = collections.defaultdict(lambda: [0, 0, 0, 0])
    for r in rows:
        e = by[r["pattern"]]
        e[0 if r["name"] else 1] += 1
        if r["name"]:
            e[2] += r["size"]
        else:
            e[3] += r["size"]
    print(f"{'pattern':58} {'named':>6} {'unnamed':>8} {'bytes':>7}")
    for p, (n, u, b, _) in sorted(by.items(), key=lambda kv: -(kv[1][0] + kv[1][1])):
        print(f"{p:58} {n:6} {u:8} {b:7}")
    print(f"matched {len(rows)}: {len(named)} named (ported, {sum(r['size'] for r in named)} bytes), "
          f"{len(rows) - len(named)} unnamed ({sum(r['size'] for r in rows if not r['name'])} bytes)")


def run_parity(snapshots):
    """Runs the parity test on the image and each snapshot; returns
    {address: reason} of the failing rows."""
    fails = {}
    env = dict(os.environ)
    for snap in [None] + snapshots:
        with tempfile.NamedTemporaryFile("r", suffix=".tsv", delete=False) as f:
            path = f.name
        e = dict(env, AUTOPORT_FAILURES=path)
        e.pop("FFX_SNAPSHOT", None)
        if snap:
            e["FFX_SNAPSHOT"] = str(snap)
        r = subprocess.run(["cargo", "test", "--release", "-p", "ffx-oracle", "--test", "auto"],
                           cwd=REPO, env=e, capture_output=True, text=True)
        text = pathlib.Path(path).read_text(encoding="utf-8")
        os.unlink(path)
        for line in text.splitlines():
            a, why = line.split("\t", 1)
            fails.setdefault(int(a, 16), f"{why} ({snap.name if snap else 'image'})")
        if r.returncode and not text:
            sys.stderr.write(r.stdout[-4000:] + r.stderr[-4000:])
            sys.exit("the parity test failed without reporting a row")
    return fails


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--verify", action="store_true")
    ap.add_argument("--unnamed", action="store_true")
    ap.add_argument("--rejects", action="store_true")
    ap.add_argument("--all", action="store_true",
                    help="put the unnamed matches in the table too (testing the matcher; do not commit)")
    args = ap.parse_args()
    game = os.environ.get("FFX_GAME_DIR")
    if not game:
        sys.exit("FFX_GAME_DIR is not set")
    exe = pathlib.Path(game).expanduser() / "FFX.exe"
    rows, rejects, examples = match_all(exe)
    if args.rejects:
        for why, n in rejects.most_common():
            print(f"{n:6} {why} (e.g. {examples[why]:#010x})")
        return
    if args.unnamed:
        for r in sorted((r for r in rows if not r["name"]), key=lambda r: r["address"]):
            print(f"{r['address']:#010x}\t{r['module']}\t{r['conv']}\t{r['pattern']}\t{describe(r)}")
        return
    generate(rows, args.all)
    if args.verify:
        snaps = os.environ.get("FFX_SNAPSHOTS")
        snaps = [pathlib.Path(s) for s in snaps.split(":")] if snaps else \
            [WORK / "snapshots/field.snap", WORK / "snapshots/battle.snap"]
        snaps = [s for s in snaps if s.exists()]
        while True:
            fails = run_parity(snaps)
            if not fails:
                break
            with open(DROPPED, "a", encoding="utf-8") as f:
                if DROPPED.stat().st_size == 0:
                    f.write("# address\treason (rows tools/autoport.py dropped after a parity failure)\n")
                for a, why in sorted(fails.items()):
                    print(f"dropped {a:#010x}: {why}")
                    f.write(f"{a:#010x}\t{why}\n")
            rows, rejects, examples = match_all(exe)
            generate(rows, args.all)
    report(rows)


if __name__ == "__main__":
    main()
