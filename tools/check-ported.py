#!/usr/bin/env python3
"""Checks the ported replacements (crates/ffx-hook/src/ported/*.rs) against
the Ghidra dump: every address is a function start, of at least five bytes
(the jump patch), not ported twice nor already hooked by hand in
crates/ffx-hook/src (wrappers, mods, the harness), and named alike in symbols/ffx.sym
(which must name it).
With FFX_GAME_DIR set, also that the shim pops what the original's `ret`
pops (cdecl nothing, stdcall every argument, thiscall all but `this`,
fastcall all but the first two): a mismatch unbalances the caller's stack.
Exits 1 on problems."""
import os
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import disasm_cache  # noqa: E402
import progress  # noqa: E402
import timing_log  # noqa: E402

timing_log.install()

rows = {a: (n, size) for a, n, size in progress.functions()}
names = progress.symbols()
bad = 0
seen = {}
game = os.environ.get("FFX_GAME_DIR")
exe = pathlib.Path(game).expanduser() / "FFX.exe" if game else None
SHIM = re.compile(r'^\s*(\w+) @ 0x([0-9a-f_]+) =>\s*extern "([\w-]+)" fn \w+\(([^)]*)\)', re.M)


def popped(abi, args):
    """Bytes the callee pops for a shim's signature."""
    sizes = [8 if t.split(":", 1)[1].strip() in ("u64", "i64", "f64") else 4
             for t in args.split(",") if ":" in t]
    skip = {"C": len(sizes), "stdcall": 0, "thiscall": 1, "fastcall": 2}[abi]
    return sum(sizes[skip:])


RET = re.compile(r"ret(?!f)(?:\s+0x([0-9a-f]+)\b)?")


def rets(a, size):
    """The byte counts the original's `ret` instructions pop (a near ret
    without an operand pops 0)."""
    found = set()
    for insn in DIS.get(a, size).values():
        m = RET.match(insn)
        if m:
            found.add(int(m.group(1) or "0", 16))
    return found


DIS = None
if exe:
    # Disassembly is cached per FFX.exe (tools/disasm_cache.py, shared with
    # check-clobber.py); the missing originals are disassembled in parallel.
    DIS = disasm_cache.for_exe(exe)
    DIS.load()
    DIS.prefetch({(a, rows[a][1])
                  for f in (progress.REPO / "crates/ffx-hook/src/ported").glob("*.rs")
                  for a in (int(addr.replace("_", ""), 16) for _, addr, _, _ in SHIM.findall(f.read_text(encoding="utf-8")))
                  if a in rows})


# ported twice, or ported and hooked by hand (wrappers, mods, the harness)
for line in progress.duplicate_hooks():
    print(line)
    bad += 1

for f in sorted((progress.REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
    for name, addr, abi, args in SHIM.findall(f.read_text(encoding="utf-8")):
        a = int(addr.replace("_", ""), 16)
        where = f"{f.name}: {name} @ {a:#010x}"
        seen[a] = name
        if a not in rows:
            print(f"{where}: not a function start in the dump")
            bad += 1
            continue
        if rows[a][1] < 5:
            print(f"{where}: only {rows[a][1]} bytes, too small for the jump")
            bad += 1
        if exe:
            r = rets(a, rows[a][1])
            if r and r != {popped(abi, args)}:
                print(f"{where}: the shim pops {popped(abi, args)} bytes, the original's ret {sorted(r)}")
                bad += 1
        if names.get(a) and names[a] != name:
            print(f"{where}: ffx.sym calls it {names[a]}")
            bad += 1
        elif not names.get(a):
            print(f"{where}: no function line in symbols/ffx.sym")
            bad += 1
# A panic in a function the game calls cannot unwind: it ends the process.
# Shims and their helpers must not panic (the trigonometry stand-ins built
# only off the game's target are the exception).
PANICS = re.compile(r"\b(unreachable!|panic!|todo!|unimplemented!)|\.unwrap\(\)|\.expect\(")
for f in sorted((progress.REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
    lines = f.read_text(encoding="utf-8").splitlines()
    for n, line in enumerate(lines, 1):
        if PANICS.search(line) and "runs in the 32-bit game" not in line and "\"32-bit\"" not in line:
            print(f"{f.name}:{n}: may panic in the game: {line.strip()[:80]}")
            bad += 1
# The same holds for the game-side Host implementations other crates give
# RawMem (the bridges' calls run through them inside the game): no panics
# and no asserts in those impl blocks.
GAME_SIDE = re.compile(r"\bassert(_eq|_ne)?!")
for f in sorted(progress.REPO.glob("crates/*/src/**/*.rs")):
    text = f.read_text(encoding="utf-8")
    for m in re.finditer(r"^impl [^\n]*\bfor (?:ffx_bridge::)?RawMem\b[^\n]*\{\n(.*?)^\}", text, re.S | re.M):
        start = text[: m.start(1)].count("\n") + 1
        for k, line in enumerate(m.group(1).splitlines()):
            if line.strip().startswith("//"):
                continue
            if PANICS.search(line) or GAME_SIDE.search(line):
                print(f"{f.relative_to(progress.REPO)}:{start + k}: may panic in the game: {line.strip()[:80]}")
                bad += 1
# An asm! block that calls the game must not use an `in(reg)` operand after
# the call: the register allocator may have put it in eax/ecx/edx, which the
# callee overwrites (an output pointer in edx crashed the field replays).
for f in sorted((progress.REPO / "crates/ffx-hook/src").rglob("*.rs")):
    text = f.read_text(encoding="utf-8")
    for m in re.finditer(r"asm!\((.*?)\);", text, re.S):
        body = m.group(1)
        lines = body.splitlines()
        call = next((i for i, l in enumerate(lines) if re.search(r'"\s*call\b', l)), None)
        if call is None:
            continue
        after = " ".join(l for l in lines[call + 1:] if l.strip().startswith('"'))
        for name in set(re.findall(r"\{(\w+)\}", after)):
            if re.search(rf"\b{name}\s*=\s*in\(reg\)", body):
                n = text[: m.start()].count("\n") + 1
                print(f"{f.name}:{n}: asm uses {{{name}}} after a call, in a register the callee may clobber")
                bad += 1

# Scratch a shim hands to a bridge or to the game's code must be per call
# (ported::StackFrame, a local array) or per thread (thread_local!), never a
# process-wide static: the game runs ports on several threads at once
# (docs/hooking.md, "Threads"). A shared Ch_CalcElement frame garbled every
# battle model. This rejects a static outside thread_local! whose address the
# same file takes (as_ptr/as_mut_ptr/get()/addr_of!/&raw) when it is a buffer
# (an array or an UnsafeCell), in the hook and in
# the game-side RawMem impls of other crates.
# a buffer: an array (of words, atomics, ...) or a cell holding one
STATIC = re.compile(
    r"^\s*(?:pub(?:\([\w:]+\))?\s+)?static\s+(?:mut\s+)?([A-Z_][A-Z0-9_]*)\s*:\s*"
    r"(?:\[|(?:[\w]+::)*(?:Sync)?UnsafeCell\s*<)",
    re.M,
)


def thread_local_spans(text):
    spans = []
    for m in re.finditer(r"thread_local!\s*\{", text):
        depth, i = 1, m.end()
        while i < len(text) and depth:
            depth += {"{": 1, "}": -1}.get(text[i], 0)
            i += 1
        spans.append((m.start(), i))
    return spans


def shared_scratch(path, text):
    spans = thread_local_spans(text)
    found = []
    for m in STATIC.finditer(text):
        if any(a <= m.start() < b for a, b in spans):
            continue
        name = re.escape(m.group(1))
        took = re.search(rf"\b{name}\s*\.\s*(?:as_ptr|as_mut_ptr|get)\s*\(|addr_of(?:_mut)?!\s*\(\s*{name}\b|&raw\s+(?:const|mut)\s+{name}\b", text)
        if took:
            n = text[: m.start()].count("\n") + 1
            found.append(f"{path}:{n}: process-wide static {m.group(1)} handed out by address (make it per call or thread_local!)")
    return found


hook_side = list((progress.REPO / "crates/ffx-hook/src").rglob("*.rs"))
for f in sorted(progress.REPO.glob("crates/*/src/**/*.rs")):
    if f not in hook_side and re.search(r"^impl [^\n]*\bfor (?:ffx_bridge::)?RawMem\b", f.read_text(encoding="utf-8"), re.M):
        hook_side.append(f)
for f in sorted(hook_side):
    for line in shared_scratch(f.relative_to(progress.REPO), f.read_text(encoding="utf-8")):
        print(line)
        bad += 1

# Argument counts at the bridges' calls into original code (tools/callarity.py).
if exe:
    import callarity  # noqa: E402

    found, checked = callarity.check(progress.REPO, exe, {a: size for a, (_, size) in rows.items()})
    for line in found:
        print(line)
    bad += len(found)
    print(f"{checked} calls checked for argument counts")
    DIS.save()
print(f"{len(seen)} ported, {bad} problems")
sys.exit(1 if bad else 0)
