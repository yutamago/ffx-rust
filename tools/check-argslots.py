#!/usr/bin/env python3
"""Finds hooked functions whose original stores into its own incoming
stack-argument slots, and the original call sites that read such a slot
after the call returns (docs/workflow.md, "Argument slots").

  FFX_GAME_DIR=... tools/check-argslots.py [--module M] [--all] [NAME...]

cdecl leaves the argument words to the caller, and MSVC reuses them as
scratch: op_random (0x7e6700) leaves its result in its argument's slot. A
caller that reads that word after the call (directly, or as a stale word of
a later call's arguments) sees the original's store; behind a shim it sees
the operand, because the thunk hands the shim a copy of the arguments.

For every shim in crates/ffx-hook/src/ported/*.rs the original is scanned
for writes to [ebp+8+k] (an ebp frame) or [esp+4+k] at its entry depth
(else), and for the address of such a slot taken with `lea`. Then every
call site from the dump's call_sites.tsv is followed from the return
address (both ways at each branch, up to a ret, a tail jump or 200
instructions), with the caller's stack depth tracked, until the slot is
overwritten (dead), popped, or read:

- a memory operand covering it ([esp+X] at the tracked depth, or [ebp+D]
  in an ebp frame), read or read-modify-write;
- a later call whose callee reads that stack word as an argument
  (measured from the callee's machine code, as tools/callarity.py does);
- its address taken with `lea`.

A shim reproduces the stores when it takes the caller's argument area
(`fn shim(args...; slots: u32)`, crates/ffx-hook/src/ported/mod.rs,
`ported!`); those are listed as fixed. Callers that are hooked themselves
never run their original code and are listed as notes. Exits 1 when an
unhooked caller reads a slot whose shim does not reproduce the store.

Depth tracking assumes 0 bytes popped by indirect calls and imports
(stdcall API calls and vtable calls pop theirs), so [esp+X] reads after
one may be misplaced; [ebp+D] reads are exact. Uses the disassembly cache
of check-clobber.py (tools/disasm_cache.py).
"""
import argparse
import os
import pathlib
import re
import struct
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import disasm_cache  # noqa: E402
import progress  # noqa: E402
import timing_log  # noqa: E402

timing_log.install()

WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))
EXE = pathlib.Path(os.environ["FFX_GAME_DIR"]).expanduser() / "FFX.exe"
CACHE = disasm_cache.for_exe(EXE)
SHIM = re.compile(r'^\s*(\w+) @ 0x([0-9a-f_]+) =>\s*extern "([\w-]+)" fn \w+\(([^)]*)\)', re.M)
MODULE = {"vfx_hi": "vfx"}  # shim file -> module
LIMIT = 200

# Ops that only read their memory operand(s).
READS = {"cmp", "test", "push", "bt", "fld", "fild", "fbld", "fadd", "fsub", "fsubr", "fmul",
         "fdiv", "fdivr", "fcom", "fcomp", "ficom", "ficomp", "fiadd", "fisub", "fisubr",
         "fimul", "fidiv", "fidivr", "fldcw", "fldenv", "frstor", "movzx", "movsx", "lea",
         "imul", "mul", "div", "idiv", "comiss", "ucomiss", "comisd", "ucomisd", "jmp", "call",
         "cvtss2sd", "cvtsd2ss", "cvttss2si", "cvtss2si", "cvttsd2si", "cvtsd2si", "cvtsi2ss",
         "cvtsi2sd", "addss", "subss", "mulss", "divss", "minss", "maxss", "sqrtss", "andps",
         "orps", "xorps", "addps", "mulps", "subps", "divps", "shufps", "unpcklps", "pshufd"}
# Ops that write their first operand without reading it.
STORES = {"mov", "movss", "movsd", "movaps", "movups", "movdqa", "movdqu", "movq", "movd",
          "movlps", "movhps", "pop", "fst", "fstp", "fist", "fistp", "fisttp", "fbstp",
          "fnstsw", "fnstcw", "fstsw", "fstcw", "fnstenv", "fnsave"}
SIZE = {"BYTE": 1, "WORD": 2, "DWORD": 4, "QWORD": 8, "TBYTE": 10, "XMMWORD": 16}
MEM = re.compile(r"(?:(\w+) PTR )?(?:\w+:)?\[(e[a-z]{2})(?:([+-])0x([0-9a-f]+))?\]")


def op_of(insn):
    parts = insn.split(None, 1)
    if parts and parts[0] in ("rep", "repz", "repnz", "repe", "repne", "lock"):
        parts = parts[1].split(None, 1) if len(parts) > 1 else []
    return (parts[0], parts[1] if len(parts) > 1 else "") if parts else ("", "")


def operands(rest):
    return [o.strip() for o in re.split(r",(?![^\[]*\])", rest)] if rest else []


def accesses(insn):
    """[(base register, displacement, size, kind)] of the instruction's
    register-based memory operands (no index register); kind 'r' (read),
    'w' (written, not read), 'rw' or 'addr' (lea)."""
    op, rest = op_of(insn)
    out = []
    for i, o in enumerate(operands(rest)):
        m = MEM.search(o)
        if not m:
            continue
        size = SIZE.get(m.group(1) or "", 4)
        disp = int(m.group(4), 16) * (-1 if m.group(3) == "-" else 1) if m.group(4) else 0
        if op == "lea":
            kind = "addr"
        elif i > 0 or op in READS or op.startswith(("j", "cmov")):
            kind = "r"
        elif op in STORES or op.startswith("set"):
            kind = "w"
        else:
            kind = "rw"
        out.append((m.group(2), disp, size, kind))
    return out


class Image:
    """FFX.exe's section table, for reading jump tables."""

    def __init__(self, path):
        d = self.data = path.read_bytes()
        pe = struct.unpack_from("<I", d, 0x3C)[0]
        nsec, opt = struct.unpack_from("<HxxxxxxxxxxxxH", d, pe + 6)
        base = struct.unpack_from("<I", d, pe + 24 + 28)[0]
        self.sections = []
        for i in range(nsec):
            s = pe + 24 + opt + 40 * i
            vsize, rva, rsize, raw = struct.unpack_from("<IIII", d, s + 8)
            self.sections.append((base + rva, base + rva + min(vsize, rsize), raw))

    def dword(self, va):
        for lo, hi, raw in self.sections:
            if lo <= va <= hi - 4:
                return struct.unpack_from("<I", self.data, raw + va - lo)[0]
        return None


IMAGE = Image(EXE)


def table(insn, code):
    """The targets of a jump table `jmp DWORD PTR [r*4+0xT]` inside `code`."""
    m = re.match(r"jmp\s+DWORD PTR (?:ds:)?\[e\w\w\*4\+0x([0-9a-f]+)\]$", insn)
    out = []
    if m:
        at = int(m.group(1), 16)
        while (t := IMAGE.dword(at)) in code:
            out.append(t)
            at += 4
    return out


def ret_pops(a):
    """Bytes the function at `a` pops (its `ret n`), 0 if unknown."""
    row = ROWS.get(a)
    if not row:
        return 0
    for insn in CACHE.get(a, row[1]).values():
        m = re.match(r"ret\s+0x([0-9a-f]+)", insn)
        if m:
            return int(m.group(1), 16)
    return 0


def call_target(insn):
    m = re.match(r"call\s+0x([0-9a-f]+)$", insn)
    return int(m.group(1), 16) if m else None


IMM = re.compile(r"0x([0-9a-f]+)")


def signed(v):
    """A 32-bit immediate as objdump prints it, read as signed."""
    return v - (1 << 32) if v >= 1 << 31 else v


class Frame:
    """A function's stack, propagated along its control flow.

    `depth[a]` is esp before the instruction at `a` as (epoch, bytes
    below that epoch's origin), or None where unknown. Epoch 0's origin is
    the entry esp (the return address at [origin], the first argument at
    origin + 4); `and esp, -n` (an aligned frame) starts a new epoch at
    the aligned esp. `bases` holds the registers set from esp once
    (`mov ebp,esp`, `mov ebx,esp`), as the same pair."""

    def __init__(self, code, entry):
        self.code = code
        self.bases = {}
        self.depth = {}
        epochs = 0
        # frame registers: set from esp once and otherwise only popped
        # (`push ebp; mov ebp,esp`, an aligned frame's `mov ebx,esp`);
        # other copies of esp are followed per path ([`readers`])
        writes = defaultdict(int)
        for insn in code.values():
            op, rest = op_of(insn)
            ops = operands(rest)
            if ops and re.fullmatch(r"e[a-z]{2}", ops[0]) and op not in ("cmp", "test", "push", "pop"):
                writes[ops[0]] += 1 if op not in ("mov", "lea") or len(ops) < 2 or ops[1] != "esp" else 1000
        frames = {r for r, n in writes.items() if n == 1000}
        addrs = sorted(code)
        nxt = dict(zip(addrs, addrs[1:]))
        todo = [(entry, (0, 0))]
        while todo:
            a, d = todo.pop()
            while a is not None and a in code and a not in self.depth:
                self.depth[a] = d
                insn = code[a]
                op, rest = op_of(insn)
                ops = operands(rest)
                src = ops[1] if len(ops) > 1 else ""
                imm = IMM.fullmatch(src)
                if d is not None:
                    e, n = d
                    if op == "push":
                        d = (e, n + (2 if ops and len(ops[0]) == 2 else 4))
                    elif op == "pop":
                        d = (e, n - 4)
                        if ops and ops[0] == "esp":
                            d = None
                    elif op in ("pushad", "pushal"):
                        d = (e, n + 32)
                    elif op in ("popad", "popal"):
                        d = (e, n - 32)
                    elif op in ("sub", "add") and ops and ops[0] == "esp":
                        # `add esp, 0xfffffff8` reserves 8 bytes: a signed immediate
                        d = (e, n + (1 if op == "sub" else -1) * signed(int(imm.group(1), 16))) if imm else None
                    elif op == "and" and ops and ops[0] == "esp":
                        epochs += 1
                        d = (epochs, 0)
                    elif op == "mov" and len(ops) == 2 and ops[1] == "esp" and ops[0] in frames \
                            and ops[0] not in self.bases:
                        self.bases[ops[0]] = d
                    elif op == "mov" and ops and ops[0] == "esp":
                        d = self.bases.get(src)
                    elif op == "lea" and ops and ops[0] == "esp":
                        m = MEM.search(src)
                        b = (d if m and m.group(2) == "esp" else self.bases.get(m.group(2))) if m else None
                        if b is None:
                            d = None
                        else:
                            disp = int(m.group(4), 16) * (-1 if m.group(3) == "-" else 1) if m.group(4) else 0
                            d = (b[0], b[1] - disp)  # lea esp,[esp+0x0] is padding
                    elif op == "leave":
                        b = self.bases.get("ebp")
                        d = None if b is None else (b[0], b[1] - 4)
                    elif ops and ops[0] == "esp" and op not in ("cmp", "test"):
                        d = None
                    elif op == "call":
                        t = call_target(insn)
                        d = (e, n - (ret_pops(t) if t is not None else 0))
                if op.startswith("ret") or op == "jmp" and not IMM.fullmatch(rest.strip()):
                    todo.extend((t, d) for t in table(insn, code))
                    break
                m = re.match(r"j\w*\s+0x([0-9a-f]+)$", insn)
                if m:
                    t = int(m.group(1), 16)
                    if t in code:
                        todo.append((t, d))
                    if op == "jmp":
                        break
                a = nxt.get(a)

    def position(self, at, base, disp):
        """A memory operand's address as (epoch, offset from its origin), or None."""
        d = self.depth.get(at) if base == "esp" else self.bases.get(base)
        if d is None:
            return None
        # a base register holds esp as it was when set
        return (d[0], disp - d[1])


def stores(a):
    """{slot offset k: kind} of the function at `a`: 'w' for a store to its
    incoming argument word k (bytes from the first), 'addr' for its
    address taken."""
    fr = Frame(CACHE.get(a, ROWS[a][1]), a)
    found = {}
    for at, insn in fr.code.items():
        if at not in fr.depth:
            continue
        for base, disp, size, kind in accesses(insn):
            if kind == "r":
                continue
            p = fr.position(at, base, disp)
            # the incoming arguments start at entry esp + 4
            if p is not None and p[0] == 0 and p[1] >= 4:
                off = p[1] - 4
                for k in range(off & ~3, off + (size if kind != "addr" else 4), 4):
                    if found.get(k) != "w":
                        found[k] = "w" if kind in ("w", "rw") else "addr"
    return found


def arity(a):
    """Stack words the function at `a` reads or writes, through its tail
    jumps too (an argument whose address it only takes is not counted:
    an out word, or a varargs walk)."""
    if a in ARITY:
        return ARITY[a]
    row = ROWS.get(a)
    if not row:
        ARITY[a] = 0
        return 0
    ARITY[a] = 0  # recursion through tail jumps
    fr = Frame(CACHE.get(a, row[1]), a)
    words = 0
    for at, insn in fr.code.items():
        for base, disp, size, kind in accesses(insn):
            p = fr.position(at, base, disp)
            if p is not None and p[0] == 0 and p[1] >= 4 and kind != "addr":
                words = max(words, (p[1] - 4 + size + 3) // 4)
        m = re.fullmatch(r"jmp\s+0x([0-9a-f]+)", insn)
        d = fr.depth.get(at)
        if m and int(m.group(1), 16) not in fr.code and d is not None and d[0] == 0:
            # a tail jump: the target reads this function's arguments
            words = max(words, (arity(int(m.group(1), 16)) or 0) - d[1] // 4)
    ARITY[a] = words
    return words


def readers(caller, site, k):
    """How the caller reads the slot (offset k of the arguments of the call
    at `site`) after the call returns: [(address, how)]."""
    if caller not in FRAMES:
        FRAMES[caller] = Frame(CACHE.get(caller, ROWS[caller][1]), caller)
    fr = FRAMES[caller]
    code = fr.code
    d = fr.depth.get(site)
    if d is None:
        return [(site, "unknown stack depth at the call")]
    slot = (d[0], k - d[1])
    addrs = sorted(code)
    nxt = dict(zip(addrs, addrs[1:]))
    found = []
    # copies of esp in other registers on this path: register -> depth
    todo = [(nxt.get(site), 0, ())]
    seen = set()
    while todo:
        a, n, copies = todo.pop()
        if a is None or a not in code or n > LIMIT or (a, copies) in seen:
            continue
        seen.add((a, copies))
        insn = code[a]
        op, rest = op_of(insn)
        da = fr.depth.get(a)
        if da is not None and da[0] == slot[0] and slot[1] < -da[1]:
            continue  # popped: below esp
        held = dict(copies)
        hit = None
        for base, disp, size, kind in accesses(insn):
            p = (held[base][0], disp - held[base][1]) if base in held else fr.position(a, base, disp)
            if p is None or p[0] != slot[0] or p[1] + size <= slot[1] or p[1] >= slot[1] + 4:
                continue
            if kind == "w" and p[1] <= slot[1] and p[1] + size >= slot[1] + 4:
                hit = hit or "dead"
            else:
                how = {"addr": "takes the address of"}.get(kind, "reads")
                hit = f"{how} it ({insn})"
                break
        if hit == "dead":
            continue
        if hit:
            found.append((a, hit))
            continue
        if op == "call" and da is not None and da[0] == slot[0]:
            t = call_target(insn)
            words = arity(t) if t is not None else 0  # indirect: assume none
            at = slot[1] + da[1]  # from esp at the call
            name = ROWS.get(t, ("?",))[0]
            if words and 0 <= at < 4 * words:
                found.append((a, f"passes it to {name} (argument word {at // 4})"))
                continue
        if op.startswith("ret"):
            continue
        ops = operands(rest)
        if ops and ops[0] in held and op not in ("cmp", "test", "push"):
            del held[ops[0]]
        if op == "mov" and len(ops) == 2 and ops[1] == "esp" and da is not None and ops[0] not in fr.bases:
            held[ops[0]] = da
        if op == "call":
            for r in ("eax", "ecx", "edx"):
                held.pop(r, None)
        copies = tuple(sorted(held.items()))
        if op == "jmp":
            m = IMM.fullmatch(rest.strip())
            if not m or int(m.group(1), 16) not in code:
                todo.extend((t, n + 1, copies) for t in table(insn, code))
                continue  # tail jump, or the table's targets
            todo.append((int(m.group(1), 16), n + 1, copies))
            continue
        m = re.match(r"j\w*\s+0x([0-9a-f]+)$", insn)
        if m:
            todo.append((int(m.group(1), 16), n + 1, copies))
        todo.append((nxt.get(a), n + 1, copies))
    return found


ROWS = {}
ARITY = {}
FRAMES = {}


def main():
    global ROWS
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("names", nargs="*", metavar="NAME", help="only these ported functions")
    ap.add_argument("--module", help="only shims of this module (vfx includes vfx_hi)")
    ap.add_argument("--all", action="store_true", help="also list stores no caller reads")
    ap.add_argument("--jobs", "-j", type=int, default=os.cpu_count() or 1)
    args = ap.parse_args()
    CACHE.load()
    ROWS = {a: (n, s) for a, n, s in progress.functions()}
    sites = defaultdict(list)
    with open(WORK / "decomp/call_sites.tsv", encoding="utf-8") as f:
        next(f)
        for line in f:
            caller, site, callee = line.rstrip("\n").split("\t")
            if "EXTERNAL" not in callee:
                sites[int(callee, 16)].append((int(caller, 16), int(site, 16)))
    shims, hooked = [], set()
    for f in sorted((progress.REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
        module = MODULE.get(f.stem, f.stem)
        for name, addr, abi, params in SHIM.findall(f.read_text(encoding="utf-8")):
            a = int(addr.replace("_", ""), 16)
            hooked.add(a)
            if a not in ROWS:
                continue
            if args.names and name not in args.names or args.module and module != args.module:
                continue
            shims.append((module, f.name, name, a, abi, ";" in params))
    CACHE.prefetch(((a, ROWS[a][1]) for *_, a, _, _ in shims), args.jobs)
    found = [(s, stores(s[3])) for s in shims]
    found = [(s, st) for s, st in found if st]
    CACHE.prefetch(((c, ROWS[c][1]) for (*_, a, _, _), _ in found for c, _ in sites[a] if c in ROWS),
                   args.jobs)
    callees = set()
    for (*_, a, _, _), _ in found:
        for c, _ in sites[a]:
            if c in ROWS:
                for insn in CACHE.get(c, ROWS[c][1]).values():
                    t = call_target(insn)
                    if t in ROWS:
                        callees.add(t)
    CACHE.prefetch(((t, ROWS[t][1]) for t in callees), args.jobs)
    CACHE.save()
    bad = defaultdict(int)
    fixed = notes = 0
    for (module, fname, name, a, abi, slots), st in found:
        lines = []
        unhooked = 0
        for caller, site in sites[a]:
            if caller not in ROWS:
                continue
            code = CACHE.get(caller, ROWS[caller][1])
            if not code.get(site, "").startswith("call"):
                continue  # a tail jump: the callee returns past this caller
            for k, kind in sorted(st.items()):
                for at, how in readers(caller, site, k):
                    note = " [caller hooked]" if caller in hooked else ""
                    unhooked += not note
                    lines.append(f"    {ROWS[caller][0]} ({site:#x}): slot +{k:#x} at {at:#x} {how}{note}")
        if not lines and not args.all:
            continue
        what = ", ".join(f"+{k:#x}{'' if v == 'w' else ' (address)'}" for k, v in sorted(st.items()))
        if slots:
            state = "shim reproduces the stores"
            fixed += 1
        elif unhooked:
            state = "SHIM DOES NOT reproduce the stores"
            bad[module] += 1
        else:
            state = "only hooked callers read" if lines else "no caller reads"
            notes += 1
        print(f"{fname}: {name} ({a:#x}) stores its argument slots {what}: {state}")
        for line in lines:
            print(line)
    total = sum(bad.values())
    by = ", ".join(f"{m} {n}" for m, n in sorted(bad.items())) or "none"
    print(f"{total} hooked functions store argument slots an unhooked caller reads and the shim "
          f"does not reproduce ({by}); {fixed} reproduced, {notes} notes")
    sys.exit(1 if total else 0)


if __name__ == "__main__":
    main()
