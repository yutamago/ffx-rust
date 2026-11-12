#!/usr/bin/env python3
"""Which of the C runtime DLLs' own imports `FFX.exe` can reach through the
CRT functions it calls (docs/host.md, "Imports": the triage of the imports
with no Rust side).

  FFX_GAME_DIR=... tools/crt-reach.py [--import DLL!NAME]... [--why NAME]...
                                       [--all] [--tsv FILE]

ffx-host runs the player's MSVCR110.dll and MSVCP110.dll in Unicorn
(docs/host.md, "The C runtime DLLs"); their KERNEL32 imports go to the
Rust stand-ins, and the ones without a stand-in stop the run when called.
This tool answers, statically, which of those the game can get to.

Roots: every MSVCR110/MSVCP110 import of FFX.exe that ffx-host routes to
the DLL (all but `RUST_WITH_DLLS` in crates/ffx-host/src/exec.rs, read
from the source), and each DLL's entry point (DllMain). From there a
recursive descent over `objdump -d` of each DLL's code follows direct
calls, jumps and conditional jumps; every IAT slot an instruction names
(`call [slot]`, `mov esi, [slot]`; `call esi`) is an import use; an
MSVCP110 call of an MSVCR110 import continues at that export. Address
constants (a relocated dword inside a reached instruction) pointing into
code are reached too (callbacks, `_onexit` handlers, SEH filters), and one
pointing into data starts a pointer run (vtables, switch and callback
tables): every relocated dword from there on that points into code, up to
the first one that doesn't. Calls through registers or memory that no
reached instruction names are not followed: a function only a run-time
pointer reaches (GetProcAddress, a table built at run time) is missed, so
"unreachable" here still needs the reason the pointer can't be made.

Prints, per missing import (the DLLs' imports ffx-host has no Rust side
for, from --import or by default the list a `ffx-rust --routes` file
gives with --routes), whether a root reaches it and one chain
(root > ... > the function using it); --why NAME lists every root that
reaches import NAME (one descent per root); --all prints every reachable
import.
"""
import argparse
import collections
import os
import pathlib
import re
import struct
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
INSN = re.compile(r"^\s*([0-9a-f]+):\t(\S+)\s*(.*)$")
TARGET = re.compile(r"^(?:0x)?([0-9a-f]+)$")


class Pe:
    """One PE image as the file holds it (preferred base, no relocation)."""

    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.name = self.path.name
        b = self.data = self.path.read_bytes()
        pe = struct.unpack_from("<I", b, 0x3c)[0]
        nsec, = struct.unpack_from("<H", b, pe + 6)
        optsz, = struct.unpack_from("<H", b, pe + 20)
        opt = pe + 24
        self.entry = struct.unpack_from("<I", b, opt + 16)[0]
        self.base = struct.unpack_from("<I", b, opt + 28)[0]
        self.entry += self.base
        dirs = opt + 96
        self.sections = []
        for k in range(nsec):
            o = opt + optsz + 40 * k
            name = b[o:o + 8].rstrip(b"\0").decode()
            vsz, va, rsz, raw, = struct.unpack_from("<IIII", b, o + 8)
            chars, = struct.unpack_from("<I", b, o + 36)
            self.sections.append((name, self.base + va, max(vsz, rsz), raw, rsz, bool(chars & 0x20)))
        self.exports = {}
        erva, _ = struct.unpack_from("<II", b, dirs)
        if erva:
            e = self.off(self.base + erva)
            nfun, nnames, afun, aname, aord = struct.unpack_from("<IIIII", b, e + 20)
            for i in range(nnames):
                nrva, = struct.unpack_from("<I", b, self.off(self.base + aname) + 4 * i)
                o, = struct.unpack_from("<H", b, self.off(self.base + aord) + 2 * i)
                frva, = struct.unpack_from("<I", b, self.off(self.base + afun) + 4 * o)
                self.exports[self.cstr(self.base + nrva)] = self.base + frva
        self.imports = {}  # IAT slot -> (dll, name)
        irva, _ = struct.unpack_from("<II", b, dirs + 8)
        d = self.off(self.base + irva)
        while True:
            ilt, _, _, name, iat = struct.unpack_from("<IIIII", b, d)
            if not name:
                break
            dll = self.cstr(self.base + name)
            k = 0
            lookup = ilt or iat
            while True:
                v, = struct.unpack_from("<I", b, self.off(self.base + lookup) + 4 * k)
                if not v:
                    break
                fn = f"ORDINAL {v & 0xffff}" if v & 0x8000_0000 else self.cstr(self.base + v + 2)
                self.imports[self.base + iat + 4 * k] = (dll, fn)
                k += 1
            d += 20
        self.relocs = set()
        rrva, rsize = struct.unpack_from("<II", b, dirs + 40)
        r, end = self.off(self.base + rrva), self.off(self.base + rrva) + rsize
        while r < end:
            page, size = struct.unpack_from("<II", b, r)
            if size < 8:
                break
            for k in range((size - 8) // 2):
                e, = struct.unpack_from("<H", b, r + 8 + 2 * k)
                if e >> 12 == 3:  # IMAGE_REL_BASED_HIGHLOW
                    self.relocs.add(self.base + page + (e & 0xfff))
            r += size

    def off(self, va):
        for _, start, size, raw, rsz, _ in self.sections:
            if start <= va < start + size:
                return va - start + raw if va - start < rsz else -1
        return -1

    def cstr(self, va):
        o = self.off(va)
        return self.data[o:self.data.index(b"\0", o)].decode("latin-1")

    def dword(self, va):
        o = self.off(va)
        return struct.unpack_from("<I", self.data, o)[0] if o >= 0 else None

    def is_code(self, va):
        return any(c and s <= va < s + z for _, s, z, _, _, c in self.sections)

    def is_data(self, va):
        return any(not c and s <= va < s + z for _, s, z, _, _, c in self.sections)


class Code:
    """`objdump -d` of one DLL: address -> (mnemonic, operands, next)."""

    def __init__(self, pe):
        out = subprocess.run(["objdump", "-d", "--no-show-raw-insn", "-M", "intel", str(pe.path)],
                             capture_output=True, text=True, check=True).stdout
        rows = []
        for line in out.splitlines():
            m = INSN.match(line)
            if m:
                rows.append((int(m.group(1), 16), m.group(2), m.group(3).split("#")[0].strip()))
        self.insn = {}
        for i, (a, mn, ops) in enumerate(rows):
            nxt = rows[i + 1][0] if i + 1 < len(rows) else a + 1
            self.insn[a] = (mn, ops, nxt)


def rust_with_dlls():
    src = (ROOT / "crates/ffx-host/src/exec.rs").read_text()
    m = re.search(r"RUST_WITH_DLLS: \[&str; \d+\] = \[(.*?)\];", src, re.S)
    return set(re.findall(r'"([^"]+)"', m.group(1))) if m else set()


class Graph:
    def __init__(self, game):
        self.exe = Pe(game / "FFX.exe")
        self.mods = {}
        for n in ("MSVCR110.dll", "MSVCP110.dll"):
            p = Pe(game / n)
            self.mods[n.lower()] = (p, Code(p))
        keep = rust_with_dlls()
        self.roots = []
        for dll, fn in self.exe.imports.values():
            if dll.lower() in self.mods and fn not in keep and fn in self.mods[dll.lower()][0].exports:
                self.roots.append((dll.lower(), fn))
        self.roots.sort()

    def descend(self, starts):
        """(module, address, label) starts -> {(dll, name): chain} of every
        import use reached, and the parents."""
        parent = {}
        uses = {}
        seen = set()
        work = collections.deque()
        for mod, at, label in starts:
            key = (mod, at)
            if key not in parent:
                parent[key] = (None, label)
                work.append((mod, at, key))
        while work:
            mod, at, fn = work.popleft()
            pe, code = self.mods[mod]
            while (mod, at) not in seen:
                seen.add((mod, at))
                ins = code.insn.get(at)
                if ins is None:
                    break
                mn, ops, nxt = ins

                def reach(target, key_mod=mod):
                    k = (key_mod, target)
                    if k not in parent:
                        parent[k] = (fn, None)
                        work.append((key_mod, target, k))
                # address constants inside this instruction
                for r in range(at, nxt):
                    if r in pe.relocs and r + 4 <= nxt:
                        v = pe.dword(r)
                        if v in pe.imports:
                            dll, name = pe.imports[v]
                            uses.setdefault((dll.lower(), name), fn)
                            if dll.lower() in self.mods:
                                x = self.mods[dll.lower()][0].exports.get(name)
                                if x is not None:
                                    reach(x, dll.lower())
                        elif v is not None and pe.is_code(v):
                            reach(v)
                        elif v is not None and pe.is_data(v):
                            p = v
                            for _ in range(4096):
                                w = pe.dword(p)
                                if p not in pe.relocs or w is None:
                                    break
                                if pe.is_code(w):
                                    reach(w)
                                elif w in pe.imports:
                                    pass
                                elif not pe.is_data(w):
                                    break
                                p += 4
                branch = mn.startswith("j") or mn == "call" or mn.startswith("loop")
                t = TARGET.match(ops) if branch else None
                if t:
                    reach(int(t.group(1), 16))
                if mn in ("ret", "retn", "iret", "int3", "hlt", "ud2") or mn.startswith("jmp"):
                    break
                at = nxt
        return uses, parent

    def chain(self, parent, key):
        out = []
        while key is not None and len(out) < 40:
            up, label = parent[key]
            mod, at = key
            pe = self.mods[mod][0]
            out.append(label or self.label(pe, at))
            key = up
        return " < ".join(out)

    def label(self, pe, at):
        best = None
        for n, a in pe.exports.items():
            if a <= at and (best is None or a > best[1]):
                best = (n, a)
        if best and at - best[1] < 0x400:
            off = at - best[1]
            return f"{pe.name}!{best[0]}" + (f"+{off:#x}" if off else "")
        return f"{pe.name}@{at:#010x}"

    def starts(self, roots):
        s = []
        for mod, fn in roots:
            s.append((mod, self.mods[mod][0].exports[fn], f"FFX.exe -> {self.mods[mod][0].name}!{fn}"))
        return s

    def all_starts(self):
        s = self.starts(self.roots)
        for mod, (pe, _) in self.mods.items():
            s.append((mod, pe.entry, f"{pe.name} DllMain"))
        return s


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--import", dest="imports", action="append", default=[],
                    help="DLL!NAME as the DLL imports it (KERNEL32.dll!FindFirstFileExW)")
    ap.add_argument("--routes", help="a `ffx-rust --routes` file: its DLL imports marked missing")
    ap.add_argument("--why", action="append", default=[], help="every root that reaches import NAME")
    ap.add_argument("--all", action="store_true", help="every import the roots reach")
    args = ap.parse_args()
    game = pathlib.Path(os.environ.get("FFX_GAME_DIR", "")).expanduser()
    if not (game / "FFX.exe").is_file():
        sys.exit("FFX_GAME_DIR must name the game folder")
    g = Graph(game)
    uses, parent = g.descend(g.all_starts())
    wanted = []
    for i in args.imports:
        dll, _, name = i.partition("!")
        wanted.append((dll.lower(), name))
    if args.routes:
        for line in open(args.routes, encoding="utf-8"):
            m = re.match(r"^(\S+?)!(.+) \(from (\S+)\)\tmissing$", line.rstrip("\n"))
            if m:
                wanted.append((m.group(1).lower(), m.group(2)))
    print(f"roots: {len(g.roots)} CRT imports of FFX.exe routed to the DLLs, and the DLLs' entry points")
    if args.all:
        for k in sorted(uses):
            print(f"{k[0]}!{k[1]}\t{g.chain(parent, uses[k])}")
    seen = set()
    for k in wanted:
        if k in seen:
            continue
        seen.add(k)
        if k in uses:
            print(f"REACHED\t{k[0]}!{k[1]}\t{g.chain(parent, uses[k])}")
        else:
            print(f"unreached\t{k[0]}!{k[1]}")
    for name in args.why:
        hits = []
        for r in g.roots:
            u, _ = g.descend(g.starts([r]))
            if any(n == name for (_, n) in u):
                hits.append(f"{r[1]}")
        for mod, (pe, _) in g.mods.items():
            u, _ = g.descend([(mod, pe.entry, "DllMain")])
            if any(n == name for (_, n) in u):
                hits.append(f"{pe.name} DllMain")
        print(f"{name}: {len(hits)} roots: {', '.join(hits)}")


if __name__ == "__main__":
    main()
