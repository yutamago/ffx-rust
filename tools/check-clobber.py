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

Disassembly (objdump, one run per function) is cached in
$FFX_WORK/cache/clobber-<key>.pickle, the key a hash of FFX.exe and the
objdump version, and the missing functions are disassembled in parallel
(--jobs, default every CPU). --no-cache neither reads nor writes the cache.
The output does not depend on either.
"""
import argparse
import bisect
import hashlib
import os
import pathlib
import pickle
import re
import subprocess
import sys
import tempfile
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
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
INSN = re.compile(r"^\s*([0-9a-f]+):\t(.*)$")


def _objdump(a, size):
    out = subprocess.run(["objdump", "-d", "--no-show-raw-insn", "-M", "intel", f"--start-address={a:#x}",
                          f"--stop-address={a + size:#x}", str(EXE)], capture_output=True, text=True,
                         check=True).stdout
    code = {}
    for line in out.splitlines():
        m = INSN.match(line)
        if m:
            code[int(m.group(1), 16)] = m.group(2).split("#")[0].strip()
    return code


class _Cache:
    """(address, size) -> disassembly, in memory and on disk per FFX.exe."""

    def __init__(self):
        self.code = {}
        self.added = False
        self.path = None
        self.use_disk = True

    def _file(self):
        if self.path is None:
            h = hashlib.sha256(EXE.read_bytes())
            ver = subprocess.run(["objdump", "--version"], capture_output=True, text=True, check=True).stdout
            h.update(ver.splitlines()[0].encode() if ver else b"")
            h.update(INSN.pattern.encode())
            self.path = WORK / "cache" / f"clobber-{h.hexdigest()[:16]}.pickle"
        return self.path

    def load(self):
        if not self.use_disk:
            return
        try:
            with open(self._file(), "rb") as f:
                self.code.update(pickle.load(f))
        except (OSError, EOFError, pickle.UnpicklingError):
            pass

    def save(self):
        if not self.use_disk or not self.added:
            return
        path = self._file()
        path.parent.mkdir(parents=True, exist_ok=True)
        merged = {}
        try:  # keep what a concurrent run added
            with open(path, "rb") as f:
                merged = pickle.load(f)
        except (OSError, EOFError, pickle.UnpicklingError):
            pass
        merged.update(self.code)
        fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".")
        with os.fdopen(fd, "wb") as f:
            pickle.dump(merged, f, protocol=pickle.HIGHEST_PROTOCOL)
        os.replace(tmp, path)
        self.added = False

    def get(self, a, size):
        key = (a, size)
        code = self.code.get(key)
        if code is None:
            code = self.code[key] = _objdump(a, size)
            self.added = True
        return code

    def prefetch(self, ranges, jobs):
        todo = sorted({r for r in ranges if r not in self.code})
        if not todo:
            return
        with ThreadPoolExecutor(max_workers=max(1, jobs)) as pool:
            for key, code in zip(todo, pool.map(lambda r: _objdump(*r), todo, chunksize=16)):
                self.code[key] = code
        self.added = True


CACHE = _Cache()


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


def live_after(code, site, keep, nxt=None):
    """Registers of `keep` read before written on some path from after the call at `site`."""
    if nxt is None:
        nxt = next_map(code)
    found = set()
    todo = [(nxt.get(site), frozenset(keep), 0)]
    seen = set()
    while todo:
        a, live, n = todo.pop()
        if a is None or not live or n > 64 or (a, live) in seen:
            continue
        seen.add((a, live))
        insn = code[a]
        if insn.startswith(("ret", "call", "jmp")) and not re.match(r"jmp\s+[0-9a-f]+\s*<", insn):
            if insn.startswith("ret") and "eax" in live:
                found.add("eax (returned)")
            continue
        r, w = effect(insn)
        if insn.startswith("push") and reserves(code, nxt, a):
            r = set()
        found |= live & r
        live = live - w
        target = re.match(r"j\w*\s+([0-9a-f]+)\b", insn)
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
    CACHE.save()
    print(f"{bad} call sites rely on a register the original leaves alone, {dropped_n} read a dropped result")
    sys.exit(1 if dropped_n else 0)


if __name__ == "__main__":
    main()
