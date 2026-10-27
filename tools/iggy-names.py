#!/usr/bin/env python3
"""The names FFX.exe hands Iggy, found statically (docs/formats/pc-ui.md, "Iggy calls").

For every call of the game's Iggy wrappers (AS3 function calls:
IggyMenu_callIggyAS3External and the IggyMenu_callFlashFunction_<n> family;
external callbacks: IggyMenu_addCallback / IggyCallbackTable_add; movies:
IggyMenu_ctor, IggyPlayerOpen*), prints the call site, its function and the
strings pushed or stored since the previous call. Heuristic: a name built at
run time (sprintf) shows as no string.

  FFX_GAME_DIR=~/steam-games/FFX-X2-HD python3 -I tools/iggy-names.py [--tsv]
"""
import os
import pathlib
import re
import struct
import subprocess
import sys

TARGETS = {
    0x00a28ba0: "callIggyAS3External",
    0x0068dd40: "callFlashFunction_1",
    0x0068dd80: "callFlashFunction_2",
    0x0068ddd0: "callFlashFunction_3",
    0x0068de50: "callFlashFunction_4",
    0x0068def0: "callFlashFunction_5",
    0x006fcaf0: "callFlashFunction_6",
    0x006ffaa0: "callFlashFunction_8",
    0x00a28e90: "addCallback",
    0x00a266b0: "IggyCallbackTable_add",
    0x00a288f0: "IggyMenu_ctor",
    0x006e9250: "IggyPlayerOpenScaled",
    0x006e93f0: "IggyPlayerOpen",
}
INSN = re.compile(r"^\s*([0-9a-f]+):\t(.*)$")
IMM = re.compile(r"0x([0-9a-f]{6,8})\b")


def sections(exe):
    data = exe.read_bytes()
    nt = struct.unpack_from("<I", data, 0x3c)[0]
    count = struct.unpack_from("<H", data, nt + 6)[0]
    opt = struct.unpack_from("<H", data, nt + 20)[0]
    base = struct.unpack_from("<I", data, nt + 24 + 28)[0]
    out = []
    at = nt + 24 + opt
    for _ in range(count):
        vsize, va, rsize, raw = struct.unpack_from("<IIII", data, at + 8)
        out.append((base + va, base + va + max(vsize, rsize), raw))
        at += 40
    return data, out


def string_at(data, secs, addr):
    for start, end, raw in secs:
        if start <= addr < end:
            off = raw + addr - start
            s = data[off:off + 200].split(b"\0", 1)[0]
            if 2 <= len(s) and all(32 <= c < 127 for c in s):
                return s.decode()
    return None


def symbols(repo):
    names = []
    for line in (repo / "symbols" / "ffx.sym").read_text().splitlines():
        p = line.split()
        if len(p) >= 3 and p[1] == "function":
            names.append((int(p[0], 16), p[2]))
    names.sort()
    return names


def owner(names, addr):
    lo, hi = 0, len(names)
    while lo < hi:
        mid = (lo + hi) // 2
        if names[mid][0] <= addr:
            lo = mid + 1
        else:
            hi = mid
    return names[lo - 1][1] if lo else "?"


def main():
    repo = pathlib.Path(__file__).resolve().parent.parent
    exe = pathlib.Path(os.environ.get("FFX_GAME_DIR", str(pathlib.Path.home() / "steam-games/FFX-X2-HD"))) / "FFX.exe"
    data, secs = sections(exe)
    names = symbols(repo)
    asm = subprocess.run(["objdump", "-d", "--no-show-raw-insn", "-M", "intel", str(exe)],
                         capture_output=True, text=True, check=True).stdout
    window = []
    tsv = "--tsv" in sys.argv
    for line in asm.splitlines():
        m = INSN.match(line)
        if not m:
            continue
        addr, text = int(m.group(1), 16), m.group(2)
        if text.startswith("call"):
            t = IMM.search(text)
            target = int(t.group(1), 16) if t else None
            if target in TARGETS:
                strings = []
                for w in window:
                    for imm in IMM.findall(w):
                        s = string_at(data, secs, int(imm, 16))
                        if s:
                            strings.append(s)
                where = owner(names, addr)
                if tsv:
                    print(f"{addr:#010x}\t{where}\t{TARGETS[target]}\t" + "\t".join(strings))
                else:
                    print(f"{addr:#010x} {where:50} {TARGETS[target]:22} {strings}")
            window = []
            continue
        if text.startswith("ret") or text.startswith("jmp"):
            window = []
            continue
        window.append(text)


if __name__ == "__main__":
    main()
