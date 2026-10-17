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

# A shim helper that dispatches on the argument count (chara's `st0_cdecl`,
# `ci_call`) answers NaN for a count it lacks, and the oracle's own
# closures never run it: Ch_AutoMotion's three-word Sg_AngleApproach call
# got NaN in the game. Every `fcall`/`ci` call a bridge makes must pass a
# count its helper supports and, for a cdecl `fcall`, the callee's arity
# (the dump's signature).
def helper_arities(text, fn):
    m = re.search(rf"fn {fn}\(.*?\n}}\n", text, re.S)
    if not m:
        return None
    return {len([x for x in arm.split(",") if x.strip()])
            for arm in re.findall(r"^\s*\[([\w\s,]*)\]\s*=>", m.group(0), re.M)}


def dump_arity(addr):
    """The parameter count of the dump's signature for `addr`, or None."""
    chunk = WORK_DECOMP / f"code/0x{addr & ~0xffff:08x}.c"
    if not chunk.exists():
        return None
    text = chunk.read_text(encoding="utf-8", errors="replace")
    i = text.find(f"// ==== 0x{addr:08x} ====")
    if i < 0:
        return None
    for line in text[i:].splitlines()[1:40]:
        m = re.match(r"^[A-Za-z_].*?\b\w+\((.*)\)\s*$", line)
        if m:
            params = m.group(1).strip()
            return 0 if params in ("", "void") else params.count(",") + 1
    return None


def slice_len(code, arg):
    """Words in a call's slice argument: a literal `&[..]`, or `&name` bound
    to a literal array (`let name = [..]`, also `[..].map(..)`)."""
    arg = arg.strip()
    lit = re.fullmatch(r"&\[(.*)\]", arg, re.S)
    if not lit:
        name = re.fullmatch(r"&(\w+)", arg)
        if not name:
            return None
        bound = list(re.finditer(rf"let {name.group(1)}\s*=\s*\[", code))
        if not bound:
            return None
        start = bound[-1].end() - 1
        body = balanced(code, start)
        return top_level_items(body[1:-1]) if body else None
    return top_level_items(lit.group(1))


def balanced(code, start):
    depth = 0
    for j in range(start, len(code)):
        depth += code[j] in "([{" and 1 or (-1 if code[j] in ")]}" else 0)
        if depth == 0:
            return code[start:j + 1]
    return None


def top_level_items(body):
    depth, items, cur = 0, 0, ""
    for ch in body:
        depth += 1 if ch in "([{" else -1 if ch in ")]}" else 0
        if ch == "," and depth == 0:
            items += cur.strip() != ""
            cur = ""
        else:
            cur += ch
    return items + (cur.strip() != "")


WORK_DECOMP = progress.WORK / "decomp"
shim = (progress.REPO / "crates/ffx-hook/src/ported/chara.rs").read_text(encoding="utf-8")
helpers = {"fcall": helper_arities(shim, "st0_cdecl"), "ci": helper_arities(shim, "ci_call")}
consts = {}
for f in sorted((progress.REPO / "crates/ffx-chara/src").glob("*.rs")):
    for m in re.finditer(r"pub const (\w+): u32 = (0x[0-9a-f_]+);", f.read_text(encoding="utf-8")):
        consts[m.group(1)] = int(m.group(2).replace("_", ""), 16)
checked = 0
for f in sorted((progress.REPO / "crates/ffx-chara/src").glob("*.rs")):
    code = f.read_text(encoding="utf-8")
    for m in re.finditer(r"\((?:c\.)?(fcall|ci)\)\(mem,\s*", code):
        args = balanced(code, m.start() + m.group(0).index(")(") + 1)
        n = code[: m.start()].count("\n") + 1
        parts = []
        depth, cur = 0, ""
        for ch in args[1:-1]:
            depth += 1 if ch in "([{" else -1 if ch in ")]}" else 0
            if ch == "," and depth == 0:
                parts.append(cur)
                cur = ""
            else:
                cur += ch
        parts.append(cur)
        target, arg = parts[1].strip(), ",".join(parts[2:])
        count = slice_len(code, arg)
        kind = m.group(1)
        if count is None:
            print(f"{f.name}:{n}: {kind} call with an argument list this check cannot count")
            bad += 1
            continue
        if helpers[kind] is not None and count not in helpers[kind]:
            print(f"{f.name}:{n}: {kind} passes {count} words; the shim helper takes {sorted(helpers[kind])}")
            bad += 1
        name = re.fullmatch(r"r\(mem,\s*(\w+)\)", target)
        addr = None
        if name:
            v = name.group(1)
            addr = int(v.replace("_", ""), 16) if v.startswith("0x") else consts.get(v)
        if kind == "fcall" and addr is not None:
            arity = dump_arity(addr)
            if arity is not None and arity != count:
                print(f"{f.name}:{n}: fcall passes {count} words to 0x{addr:08x}, which takes {arity}")
                bad += 1
        checked += 1
print(f"{checked} float helper calls checked")
print(f"{len(seen)} ported, {bad} problems")
sys.exit(1 if bad else 0)
