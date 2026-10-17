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
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

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


def rets(a, size):
    """The byte counts the original's `ret` instructions pop."""
    out = subprocess.run(["objdump", "-d", "--no-show-raw-insn", f"--start-address={a:#x}",
                          f"--stop-address={a + size:#x}", str(exe)],
                         capture_output=True, text=True, check=True).stdout
    return {int(m.group(1) or "0", 16) for m in re.finditer(r"\tret\s*(?:\$0x([0-9a-f]+))?", out)}


# hooks written by hand (wrappers, mods, the harness) outside the ported tables
hand = {}
for f in sorted((progress.REPO / "crates/ffx-hook/src").glob("*.rs")):
    for name, addr in re.findall(r'name: "(\w+)",\s*address: 0x([0-9a-f_]+),', f.read_text(encoding="utf-8")):
        hand[int(addr.replace("_", ""), 16)] = f"{f.name}: {name}"

for f in sorted((progress.REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
    for name, addr, abi, args in SHIM.findall(f.read_text(encoding="utf-8")):
        a = int(addr.replace("_", ""), 16)
        where = f"{f.name}: {name} @ {a:#010x}"
        if a in seen:
            print(f"{where}: also ported as {seen[a]}")
            bad += 1
        if a in hand:
            print(f"{where}: also hooked by hand ({hand[a]})")
            bad += 1
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

# Argument counts at the bridges' calls into original code (tools/callarity.py).
if exe:
    import callarity  # noqa: E402

    found, checked = callarity.check(progress.REPO, exe, {a: size for a, (_, size) in rows.items()})
    for line in found:
        print(line)
    bad += len(found)
    print(f"{checked} calls checked for argument counts")
print(f"{len(seen)} ported, {bad} problems")
sys.exit(1 if bad else 0)
