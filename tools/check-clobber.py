#!/usr/bin/env python3
"""Finds callers that keep eax, ecx or edx live across a call to a ported
function (docs/porting.md). MSVC's whole-program optimisation knows when a
leaf leaves a scratch register alone and lets the caller rely on it; a Rust
replacement follows the ABI and clobbers all three.

  FFX_GAME_DIR=... tools/check-clobber.py [--jobs N] [--no-cache] [NAME...]

For every ported function (or the ones named) whose original never writes
one of the three, every call site is followed from the return address
(both ways at each branch, up to a call, ret or 64 instructions): a read of
that register before a write is reported. The shims' thunks keep those
registers (crates/ffx-hook/src/ported/mod.rs), so these are notes. Also
reported, and a bug: a shim returning nothing where the original sets eax
and a caller reads it. Exits 1 when it finds one of those.

Also a bug: a shim returning nothing whose original is stored as a
pointer (a vtable or table entry in .rdata/.data, or an immediate in code:
the base relocations, tools/refs.py) and leaves a result in eax at its
ret (EaxResult). Callers through a vtable are invisible to call_sites.tsv,
so they count as reading it (pc round 2: 0x6a4800 and 0x6a48d0 return
0x6a46e0's eax and their virtual callers read it). g_MagicImports
entries are decided by the magic DLLs' call sites instead (magic_slots,
cached in $FFX_WORK/cache/magic-<key>.pickle, about a minute to build
once), particle program record slots are exempt (ppp_record_slot: their
dispatchers drop eax), and checked-by-hand exceptions go into VOID_OK.

Disassembly (objdump, one run per function) is cached in
$FFX_WORK/cache/clobber-<key>.pickle (tools/disasm_cache.py, shared with
check-ported.py), the key a hash of FFX.exe and the objdump version, and
the missing functions are disassembled in parallel (--jobs, default every
CPU). --no-cache neither reads nor writes the cache.
The output does not depend on either.
"""
import argparse
import bisect
import os
import pathlib
import re
import subprocess
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import disasm_cache  # noqa: E402
import progress  # noqa: E402
import timing_log  # noqa: E402

timing_log.install()

WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))
EXE = pathlib.Path(os.environ["FFX_GAME_DIR"]).expanduser() / "FFX.exe"
SCRATCH = {
    "eax": {"eax", "ax", "al", "ah"},
    "ecx": {"ecx", "cx", "cl", "ch"},
    "edx": {"edx", "dx", "dl", "dh"},
}
WRITE_ONLY = {"mov", "movzx", "movsx", "lea", "pop", "movd", "cvttss2si", "cvtss2si",
              "cvttsd2si", "cvtsd2si", "fnstsw", "lahf"}
READ_ONLY = {"cmp", "test", "push", "bt"}
CACHE = disasm_cache.for_exe(EXE)  # shared with check-ported.py and callarity.py


def disasm(a, size):
    """Address -> instruction text of `size` bytes from `a` (a copy: callers may change it)."""
    return dict(CACHE.get(a, size))


def regs(text):
    return {r for r, names in SCRATCH.items() if any(re.search(rf"\b{n}\b", text) for n in names)}


def effect(insn):
    """(registers read, registers written) of one instruction."""
    parts = insn.split(None, 1)
    if parts and parts[0] in ("rep", "repz", "repnz", "repe", "repne", "lock"):
        prefix, parts = parts[0], parts[1].split(None, 1) if len(parts) > 1 else []
    else:
        prefix = ""
    if not parts:
        return set(), set()
    op, rest = parts[0], parts[1] if len(parts) > 1 else ""
    ops = [o.strip() for o in re.split(r",(?![^\[]*\])", rest)] if rest else []
    read, write = set(), set()
    if prefix.startswith("rep"):
        read.add("ecx")
        write.add("ecx")
    if op == "lahf":  # writes ah
        return set(), {"eax"}
    if op == "sahf":
        return {"eax"}, set()
    if op in ("cdq", "cwde", "cbw"):
        return {"eax"}, {"edx"} if op == "cdq" else {"eax"}
    if op in ("mul", "div", "idiv") or (op == "imul" and len(ops) == 1):
        return regs(rest) | {"eax"} | ({"edx"} if op in ("div", "idiv") else set()), {"eax", "edx"}
    if op.startswith("stos") or op.startswith("scas"):
        read.add("eax")
    if op.startswith("lods"):
        write.add("eax")
    if op == "xchg" and len(ops) == 2:
        return regs(rest), regs(rest)
    if not ops:
        return read, write
    if len(ops) == 2 and ((op == "or" and ops[1] in ("0xffffffff", "-1")) or (op == "and" and ops[1] == "0x0")):
        return set(), regs(ops[0])  # a constant whatever the register held
    if op in ("xor", "sub", "sbb") and len(ops) == 2 and ops[0] == ops[1]:
        return read, regs(ops[0])
    dest, src = ops[0], ",".join(ops[1:])
    in_mem = "[" in dest
    dest_regs = set() if in_mem else regs(dest)
    read |= regs(src) | (regs(dest) if in_mem else set())
    if op in READ_ONLY:
        read |= dest_regs
    elif op in WRITE_ONLY or op.startswith("set"):
        write |= dest_regs
    else:
        read |= dest_regs
        write |= dest_regs
    return read, write


def writes(code):
    """Scratch registers an original may change (a call: all of them)."""
    out = set()
    for insn in code.values():
        if insn.startswith("call"):
            return set(SCRATCH)
        out |= effect(insn)[1]
    return out


def reserves(code, nxt, a):
    """`push reg` that only makes room: the next instructions store to [esp]."""
    b = nxt.get(a)
    for _ in range(2):
        if b is None:
            return False
        if re.match(r"(fstp|fst|mov|movss|movsd)\s+\w+ PTR \[esp\](,|$)", code[b]):
            return True
        b = nxt.get(b)
    return False


def next_map(code):
    """Address -> the next instruction's address."""
    addrs = sorted(code)
    return {a: b for a, b in zip(addrs, addrs[1:])}


# objdump prints "jne    0x6a486e" (older binutils: "jne    6a486e <sym>")
DIRECT_JMP = re.compile(r"jmp\s+(?:0x)?[0-9a-f]+(?:\s*<.*>)?$")
BRANCH = re.compile(r"j\w*\s+(?:0x)?([0-9a-f]+)(?:\s*<.*>)?$")


def live_after(code, site, keep, nxt=None):
    """Registers of `keep` read before written on some path from after the call at `site`."""
    if nxt is None:
        nxt = next_map(code)
    found = set()
    todo = [(nxt.get(site), frozenset(keep), 0)]
    seen = set()
    while todo:
        a, live, n = todo.pop()
        if a is None or a not in code or not live or n > 64 or (a, live) in seen:
            continue
        seen.add((a, live))
        insn = code[a]
        if insn.startswith(("ret", "call", "jmp")) and not DIRECT_JMP.match(insn):
            if insn.startswith("ret") and "eax" in live:
                found.add("eax (returned)")
            continue
        r, w = effect(insn)
        if insn.startswith("push") and reserves(code, nxt, a):
            r = set()
        found |= live & r
        live = live - w
        target = BRANCH.match(insn)
        if target:
            todo.append((int(target.group(1), 16), live, n + 1))
            if insn.startswith("jmp"):
                continue
        todo.append((nxt.get(a), live, n + 1))
    return found


X87_LOADS = ("fld", "fild", "fldz", "fld1", "fldpi", "fldl2e", "fldln2", "fldlg2", "fldl2t",
             "fnstcw", "fldcw", "fninit", "fwait", "fnclex", "fnstsw", "ffree")


# Originals whose x87 code is balanced (every load stored back), checked by
# hand: their callers keep their own floats across the call.
X87_BALANCED = {
    "useExtendedAlpha",  # 0x0071e460: fld constant, fstp g_ExtendedAlpha
}


def reads_st0(code, site, addrs=None):
    """Whether the first x87 instruction after the call at `site` reads the
    x87 stack it left (a float result), before any load, jump or call.
    `addrs`: `sorted(code)`, when the caller has it."""
    if addrs is None:
        addrs = sorted(code)
    i = bisect.bisect_right(addrs, site)
    after = addrs[i:i + 16]
    for a in after:
        insn = code[a]
        op = insn.split()[0] if insn else ""
        if op.startswith(("j", "call", "ret")):
            return False
        if op.startswith("f"):
            return not op.startswith(X87_LOADS)
    return False


# Shims reached through a vtable or a stored pointer that return nothing
# although the original's eax looks like a result, checked by hand: the
# eax is garbage to every indirect caller. address -> reason.
MENU_CTRL = ("menu window +0xc control: its dispatchers TkMenuMainActiveMenu 0x8aa153 and "
             "TOMenuCloseMainWindows_008a9230 0x8a927c overwrite eax after the call (2026-10-23)")
RCBG_DRAW = ("rcbg object +0xc draw: its dispatcher 0x92075e (rcbgScene draw loop) and the +4 "
             "ones 0x920695/0x9208cf/0x920abe overwrite eax after the call (2026-10-23)")
VOID_OK = {
    0x00922100: RCBG_DRAW,  # rcbgTrxTexDraw
    0x0092d470: RCBG_DRAW,  # rcbgTrxDraw_0092d470
    0x008d9e20: MENU_CTRL,  # MenuShop_SellWindowCtrl
    0x008d9bd0: MENU_CTRL,  # MenuShop_BuyWindowCtrl
    0x008d7690: MENU_CTRL,  # MenuWShop_BuyConfirmCtrl
    0x008c3580: MENU_CTRL,  # MenuNameEntry_WindowCtrl_008c3580
    # TOFuncPadMessageInputValue: the +0xfc pad handler; TOGetControlPadStatus
    # 0x8b6ba7 tail-jumps to it, and both its callers (0x8ab969, 0x8ab9f9)
    # overwrite eax (2026-10-23)
    0x008b62b0: "+0xfc pad handler: TOGetControlPadStatus's callers drop its eax",
}

# g_MagicImports (ffx.sym): the function table MagicFile_Start hands to
# every magic_*.dll's InitMagicPRX; the DLLs call through it
MAGIC_IMPORTS = 0x00c64ce8
MAGIC_SPAN = 0x1000
MAGIC_VERSION = 3


def magic_slots(jobs):
    """{offset into g_MagicImports: (call sites, sites reading eax, an
    example "dll@site")} over $FFX_GAME_DIR/magicFiles/FFX/*.dll: a load
    of the DLL's table global, then `mov r, [table + off]` and `call r`
    (or `call [table + off]`), each followed like a call site (live_after).
    Cached in $FFX_WORK/cache/magic-<key>.pickle (DLL names, sizes, mtimes)."""
    import hashlib
    import pickle
    from concurrent.futures import ThreadPoolExecutor
    dlls = sorted((EXE.parent / "magicFiles/FFX").glob("magic_*.dll"))
    if not dlls:
        return None
    h = hashlib.sha256(f"v{MAGIC_VERSION}".encode())
    for d in dlls:
        st = d.stat()
        h.update(f"{d.name}:{st.st_size}:{st.st_mtime_ns};".encode())
    path = WORK / "cache" / f"magic-{h.hexdigest()[:16]}.pickle"
    try:
        if CACHE.use_disk:
            with open(path, "rb") as f:
                return pickle.load(f)
    except (OSError, EOFError, pickle.UnpicklingError):
        pass
    out = {}
    with ThreadPoolExecutor(max_workers=max(1, jobs)) as pool:
        for name, found in pool.map(lambda d: (d.name, magic_dll_calls(d)), dlls):
            for off, site, reads in found:
                n, r, ex = out.get(off, (0, 0, ""))
                out[off] = (n + 1, r + reads, ex if ex or not reads else f"{name}@{site:#x}")
    if not CACHE.use_disk:
        return out
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(f".{os.getpid()}.tmp")
    with open(tmp, "wb") as f:
        pickle.dump(out, f)
    os.replace(tmp, path)
    return out


def magic_dll_calls(dll):
    """[(offset, call site, reads eax)] of one magic DLL."""
    code = {}
    text = subprocess.run(["objdump", "-d", "--no-show-raw-insn", "-M", "intel", "-j", ".text", str(dll)],
                          capture_output=True, text=True, check=False).stdout
    for line in text.splitlines():
        m = disasm_cache.INSN.match(line)
        if m:
            code[int(m.group(1), 16)] = m.group(2).split("#")[0].strip()
    # the table global: InitMagicPRX stores its argument there, and every
    # import call loads it first; the most loaded global is it
    loads = defaultdict(int)
    for insn in code.values():
        m = re.match(r"mov\s+(e[a-ds][xip]),(?:DWORD PTR )?ds:0x([0-9a-f]+)$", insn)
        if m:
            loads[m.group(2)] += 1
    if not loads:
        return []
    table = max(loads, key=loads.get)
    nxt = next_map(code)
    found = []
    regs_state = {}
    for a in sorted(code):
        insn = code[a]
        m = re.match(r"mov\s+(e[a-ds][xip]),(?:DWORD PTR )?ds:0x([0-9a-f]+)$", insn)
        if m and m.group(2) == table:
            regs_state[m.group(1)] = ("table", 0)
            continue
        m = re.match(r"(mov|call)\s+(?:(e[a-ds][xip]),)?DWORD PTR \[(e[a-ds][xip])(?:\+0x([0-9a-f]+))?\]$", insn)
        if m and regs_state.get(m.group(3), (None,))[0] == "table":
            off = int(m.group(4) or "0", 16)
            if m.group(1) == "call":
                found.append((off, a, "eax" in live_after(code, a, {"eax"}, nxt)))
                regs_state = {}
            else:
                regs_state[m.group(2)] = ("slot", off)
            continue
        m = re.match(r"call\s+(e[a-ds][xip])$", insn)
        if m and regs_state.get(m.group(1), (None,))[0] == "slot":
            found.append((regs_state[m.group(1)][1], a, "eax" in live_after(code, a, {"eax"}, nxt)))
            regs_state = {}
            continue
        if insn.startswith(("ret", "call", "jmp")):
            regs_state = {}
            continue
        parts = insn.split(None, 1)
        dest = parts[1].split(",")[0] if len(parts) > 1 else ""
        _, w = effect(insn)
        for r in list(regs_state):
            if r in w or (dest == r and parts[0] not in ("push", "cmp", "test")):
                regs_state.pop(r, None)
    return found


# eax writes that are never a result: x87 status words and flags, the
# eax half of a multiply or divide (a remainder in edx is the usual use)
TEMP_OPS = {"fnstsw", "fstsw", "lahf", "mul", "div", "idiv", "cdq"}
TARGET = re.compile(r"(?:call|jmp)\s+(?:0x)?([0-9a-f]+)$")


def bare(insn):
    """The instruction without a rep/repz prefix (`repz ret`)."""
    parts = insn.split(None, 1)
    if parts and parts[0] in ("rep", "repz", "repnz", "repe", "repne") and len(parts) > 1:
        return parts[1]
    return insn


class EaxResult:
    """Whether an original leaves a result in eax at its ret: the last write
    to eax on some path to a ret is a computed value (mov/xor/lea/setcc...)
    that nothing reads after it, or the eax of a direct callee
    that does so itself (followed through callees that keep eax, such as
    __security_check_cookie). A write the function then reads (stores,
    pushes, compares) is a temporary; so are a loop counter, x87 status
    words, a multiply's or divide's eax, the eax it came in with and an
    indirect call's."""

    def __init__(self, rows):
        self.rows = rows
        self.memo = {}

    def keeps_eax(self, a):
        return a in self.rows and "eax" not in writes(CACHE.get(a, self.rows[a][1]))

    def __call__(self, a, depth=0):
        """None, or a short reason the original's eax is a result."""
        if a in self.memo:
            return self.memo[a]
        self.memo[a] = None  # recursion: no result until shown
        if a not in self.rows or depth > 4:
            return None
        code = CACHE.get(a, self.rows[a][1])
        end = a + self.rows[a][1]
        nxt = next_map(code)
        state = defaultdict(set)  # address -> {(origin, used)}
        origins = set()
        todo = []

        def flow(x, s):
            if x is None or x not in code:
                return
            if not s <= state[x]:
                state[x] |= s
                todo.append(x)

        unreached = sorted(code)
        loop = set()
        flow(a, {("entry", False)})
        while True:
            while todo:
                x = todo.pop()
                s = frozenset(state[x])
                insn = bare(code[x])
                if insn.startswith("ret"):
                    origins |= s
                    continue
                m = TARGET.match(insn)
                if insn.startswith("call"):
                    if m and self.keeps_eax(int(m.group(1), 16)):
                        out = s
                    else:
                        out = {(("call", int(m.group(1), 16) if m else None), False)}
                    flow(nxt.get(x), out)
                    continue
                if insn.startswith("jmp"):
                    if m:
                        t = int(m.group(1), 16)
                        if a <= t < end and t in code:
                            flow(t, s)
                        else:  # a tail call
                            origins |= s if self.keeps_eax(t) else {(("call", t), False)}
                    continue  # an indirect jmp: its targets are seeded below
                r, w = effect(insn)
                op = insn.split()[0] if insn else ""
                if "eax" in w:
                    if op in TEMP_OPS or (op == "imul" and "," not in insn):
                        out = {("temp", False)}
                    else:
                        if "eax" in r and any(o == ("set", x) for o, _ in s):
                            loop.add(x)  # eax += step around a loop: a counter
                        if "eax" in r and op in ("dec", "inc", "sub", "add", "and", "or", "xor", "neg") \
                                and code.get(nxt.get(x), "").startswith("j") \
                                and not code.get(nxt.get(x), "").startswith("jmp"):
                            loop.add(x)  # a switch on eax (sub eax, n; je ...): flags only
                        out = {(("set", x), False)}
                elif "eax" in r:
                    out = {(o, True) for o, _ in s}
                else:
                    out = s
                if op.startswith("j") and m is None:
                    t = re.match(r"j\w+\s+(?:0x)?([0-9a-f]+)$", insn)
                    if t:
                        flow(int(t.group(1), 16), out)
                flow(nxt.get(x), out)
            # code only an indirect jump (a jump table) reaches: unknown eax
            unreached = [x for x in unreached if x not in state]
            if not unreached:
                break
            flow(unreached[0], {("unknown", False)})
        for origin, used in sorted(origins, key=str):
            if used or origin in ("entry", "unknown", "temp"):
                continue
            kind, where = origin
            if kind == "set" and where not in loop:
                self.memo[a] = f"{code[where]} at {where:#x}"
                break
            if where is not None:
                inner = self(where, depth + 1)
                if inner:
                    self.memo[a] = f"{where:#x}'s eax ({inner})"
                    break
        return self.memo[a]


class Image:
    """FFX.exe's sections and relocations (tools/refs.py's reader)."""

    def __init__(self):
        import refs
        _, self.secs, relocs = refs.read_pe(EXE.read_bytes())
        self.relocs = set(relocs)

    def word(self, v):
        for _, va, blob, _ in self.secs:
            if va <= v <= va + len(blob) - 4:
                return int.from_bytes(blob[v - va:v - va + 4], "little")
        return None

    def string(self, v, n=8):
        for _, va, blob, code in self.secs:
            if not code and va <= v < va + len(blob):
                return blob[v - va:v - va + n]
        return b""


def ppp_record_slot(img, site):
    """Whether `site` is a function slot of a particle program table record
    (0x28 bytes: +0 the name, "ppp...", +8 step, +0xc draw, +0x1c/+0x20
    set-ups; ffx.sym "ppp program table record"). Every dispatcher through
    those slots overwrites eax right after the call, checked by hand
    (pppRunPart's 0x712d72..0x712ea6, 0x716756, 0x71693a, 0x716b28,
    0x716d00, 0x7170bd), so their eax is no result."""
    for k in range(4, 0x28, 4):
        r = site - k
        if r in img.relocs and img.string(img.word(r) or 0, 3) == b"ppp":
            return True
    return False


def indirect_drops(shims, rows, jobs):
    """Shims returning nothing whose original is stored as a pointer (a
    vtable or table entry, a callback immediate) and leaves a result in
    eax: callers through the pointer are invisible to call_sites.tsv, so
    assume they read it. Entries of g_MagicImports are decided by the
    magic DLLs' own call sites instead (magic_slots): a problem when one
    of them reads eax. [(file, name, address, pointer sites, reason)]."""
    import refs
    idx = refs.index(EXE)
    result = EaxResult(rows)
    magic = None
    img = Image()
    out = []
    for f, name, a, ret in shims:
        if ret or a in VOID_OK or a not in rows:
            continue
        ptrs = [(s, k) for s, k in idx.to(a) if k in ("data", "code")]
        if not ptrs:
            continue
        other = [(s, k) for s, k in ptrs if not (k == "data" and 0 <= s - MAGIC_IMPORTS < MAGIC_SPAN)
                 and not (k == "data" and ppp_record_slot(img, s))]
        why = None
        for s, _ in ptrs:
            if (s, "data") in other or not 0 <= s - MAGIC_IMPORTS < MAGIC_SPAN:
                continue
            if magic is None:
                magic = magic_slots(jobs) or {}
            n, reads, ex = magic.get(s - MAGIC_IMPORTS, (0, 0, ""))
            if reads:
                why = f"{reads} of {n} calls in magic_*.dll through g_MagicImports+{s - MAGIC_IMPORTS:#x} read it, e.g. {ex}"
                break
        if why is None and other:
            why = result(a)
            ptrs = other
        if why:
            out.append((f, name, a, ptrs, why))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("names", nargs="*", metavar="NAME", help="only these ported functions")
    ap.add_argument("--jobs", "-j", type=int, default=os.cpu_count() or 1,
                    help="parallel objdump runs for uncached functions (default: every CPU)")
    ap.add_argument("--no-cache", action="store_true", help="neither read nor write the disassembly cache")
    args = ap.parse_args()
    CACHE.use_disk = not args.no_cache
    CACHE.load()
    rows = {a: (n, s) for a, n, s in progress.functions()}
    sites = defaultdict(list)
    with open(WORK / "decomp/call_sites.tsv", encoding="utf-8") as f:
        next(f)
        for line in f:
            caller, site, callee = line.rstrip("\n").split("\t")
            if "EXTERNAL" not in callee:
                sites[int(callee, 16)].append((int(caller, 16), int(site, 16)))
    shim = re.compile(r"^\s*(\w+) @ 0x([0-9a-f_]+) =>\s*extern \"([\w-]+)\" fn \w+\([^)]*\)\s*(->\s*[\w:]+)?", re.M)
    shims = []
    for f in sorted((progress.REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
        for name, addr, _abi, ret in shim.findall(f.read_text(encoding="utf-8")):
            if args.names and name not in args.names:
                continue
            shims.append((f, name, int(addr.replace("_", ""), 16), ret))
    # disassemble what is not cached yet in parallel: the originals, then
    # the callers of those that need their call sites checked
    CACHE.prefetch(((a, rows[a][1]) for _, _, a, _ in shims), args.jobs)
    checks = []
    for f, name, a, ret in shims:
        code = CACHE.get(a, rows[a][1])
        changed = writes(code)
        kept = set(SCRATCH) - changed
        if ret:
            kept.discard("eax")
        # a shim that returns nothing where the original leaves a value
        # in eax its callers read (Ghidra calls many of those void)
        dropped = {"eax"} if not ret and "eax" in changed else set()
        floats = ret.replace("->", "").strip() in ("f32", "f64")
        # without x87 code of its own the original leaves the x87 stack
        # alone, and callers may keep a float on it across the call
        # (whole-program optimisation); a shim without x87 code does too
        x87 = any(i.split()[0].startswith("f") for i in code.values() if i)
        if not kept and not dropped and (floats or not x87):
            continue
        checks.append((f, name, a, kept, dropped, floats, x87))
    CACHE.prefetch(((c, rows[c][1]) for _, _, a, *_ in checks for c, _ in sites[a]), args.jobs)
    CACHE.save()
    # per caller: its instructions, their order and successor map
    order = {}
    bad = dropped_n = 0
    for f, name, a, kept, dropped, floats, x87 in checks:
        for caller, site in sites[a]:
            code = CACHE.get(caller, rows[caller][1])
            if not code.get(site, "").startswith("call"):
                continue  # a tail jump: the callee returns past this caller
            if caller not in order:
                addrs = sorted(code)
                order[caller] = (addrs, {x: y for x, y in zip(addrs, addrs[1:])})
            addrs, nxt = order[caller]
            live = live_after(code, site, kept, nxt)
            if live:
                print(f"{f.name}: {name}: {rows[caller][0]} ({site:#x}) keeps {', '.join(sorted(live))} across the call")
                bad += 1
            if not floats and x87 and name not in X87_BALANCED and reads_st0(code, site, addrs):
                print(f"{f.name}: {name}: {rows[caller][0]} ({site:#x}) reads the float the original returns in ST0; the shim returns none")
                dropped_n += 1
            used = {r for r in live_after(code, site, dropped, nxt) if r == "eax"} if dropped else set()
            if used:
                print(f"{f.name}: {name}: {rows[caller][0]} ({site:#x}) reads the eax the original returns; the shim returns nothing")
                dropped_n += 1
    for f, name, a, ptrs, why in indirect_drops(shims, rows, args.jobs):
        where = ", ".join(f"{s:#x} ({'vtable/data pointer' if k == 'data' else 'address taken in code'})"
                          for s, k in ptrs[:4])
        print(f"{f.name}: {name}: stored at {where}; its callers through the pointer may read the eax "
              f"the original returns ({why}); the shim returns nothing")
        dropped_n += 1
    CACHE.save()
    print(f"{bad} call sites rely on a register the original leaves alone, {dropped_n} read a dropped result")
    sys.exit(1 if dropped_n else 0)


if __name__ == "__main__":
    main()
