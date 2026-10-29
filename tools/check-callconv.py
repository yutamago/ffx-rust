#!/usr/bin/env python3
"""Finds port calls into original code or imports made with the wrong
calling convention (check-callconv, 2026-10-26).

  tools/check-callconv.py [-v] [--root DIR] [--exe FFX.exe] [--decomp DIR]
  tools/check-callconv.py --self-test

p6's createWindow called ImmAssociateContext's import thunk (0x009da254,
`jmp [IAT]`, a stdcall Win32 API) through `Conv::Cdecl`: the callee popped
its 8 bytes and the caller popped them again, and every replay crashed at
startup. Its oracle test passed because the test stub did not pop.

Three halves:

1. The ports' calls (crates/*/src, ffx-oracle and ffx-studio left out):
   ffx_bridge `Host::call(f, &[..])` (cdecl), ffx_graphic
   `.call/.call_sym/.call_st0/.call_st0_f64/.call_sym_st0/.call_sym_st0_f64
   (f, Conv::X, &[..])`, `NativeHost::call_n/call_float` (cdecl), ffx-hook's
   `call_cdecl*`, `call0` (cdecl) and `call_thiscall(f, this, &[..])`, and
   `let g: unsafe extern "ABI" fn(..) = transmute(EXPR)`. The target is
   resolved as check-thisarg does (literals, consts, `rebase(..)`, locals),
   but consts by path and `use` aliases first (`Consts`: two files define
   `SURFACE_CTOR` differently), plus import slots read through
   `h.read(..)`/`import(..)`; the width as check-callwidth does. A function
   passing its own parameter on as the target, the convention or the slice
   is a forwarder: its callers are checked with their arguments (same file;
   other files when the name is unique), to a fixed point. A closure the
   shims hand a bridge (`&mut |_, f, a| call_cdecl(f, a)`) or bind with
   `let` gives its convention to the calls made through it in the bridge
   (and the functions the bridge hands it on to). When a forwarder makes
   several calls of its parameter target (bind_tables: 6 words when
   `with_tables`, else 5), a caller is right when one of them fits.
2. The callee's convention from FFX.exe: every path from its entry is
   followed (both sides of branches, jumps, tail jumps into other
   functions; calls fall through) to its `ret`/`ret n`. An import thunk or
   a tail jump through an import slot takes the import's convention: the
   CRT (MSVCR/MSVCP's C functions) and steam_api are cdecl, decorated
   names say theirs (`_Name@N` stdcall N bytes, C++ `@@..A/E/G/I`), every
   other DLL export is stdcall (Win32 `WINAPI`, FMOD `F_API`, RAD
   `RADLINK`) unless CDECL_EXPORTS names it; STDCALL_BYTES holds the
   stdcall imports' argument bytes the ports need.
3. Oracle test stubs (crates/ffx-oracle/tests): a test helper with
   parameters named `sym` and `pops` (words its stub pops), or `sym`,
   `conv: Conv` and `n`, called with a resolvable `sym` and a literal
   `pops` must pop what the original pops, or a port calling with the
   wrong convention still passes its test (the oracle's `Oracle::call`
   checks `esp` after each call against the convention, but only a stub
   that pops like the original lets that check see the port's mistake).

--self-test checks fixtures of the createWindow case: the call made
directly, through a helper function, through a closure handed to a bridge,
and a stub that doesn't pop.

`Conv::FastcallCallerPops` (the LTCG custom convention of 0x00401690 and
Settings_ValidateBindings: `ecx`, `edx`, then stack words the caller pops)
is checked like cdecl, with its first two words in registers; --self-test
has a call of 0x00401690 made right, one made as fastcall and one of a
popping import made with it.

Problems: a cdecl helper against a callee that pops; a callee-pops helper
(stdcall, thiscall, fastcall) against a plain `ret` or a cdecl import when
it passes stack words; a word count that doesn't match `ret n`/4; a stub
popping other than the original. Calls whose target, convention or width
can't be resolved, and callees whose `ret`s disagree or can't be found,
are counted, not checked. REVIEWED names checked exceptions; an entry that
matches nothing any more is reported.

Prints one line per problem and "N problems (...)"; exits 1 when there are
any. -v lists every checked call.
"""
import argparse
import importlib.util
import os
import pathlib
import re
import subprocess
import sys
import tempfile

TOOLS = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import disasm_cache  # noqa: E402


def _load(name, file):
    spec = importlib.util.spec_from_file_location(name, TOOLS / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


cw = _load("check_callwidth", "check-callwidth.py")
ta = _load("check_thisarg", "check-thisarg.py")

ROOT = TOOLS.parent
SKIP_CRATES = {"ffx-oracle", "ffx-studio"}

# (file, callee address) -> why the port's convention is right anyway
REVIEWED = {
}

# --- imports ---

CRT_DLL = re.compile(r"^(msvcr|msvcp|vcruntime|ucrtbase|api-ms-win-crt)", re.I)
CDECL_DLLS = {"steam_api.dll"}  # S_API ... S_CALLTYPE (__cdecl)
# varargs exports: the originals pop their arguments after the call
# (IggyGDrawSendWarning: `call [0x00b0c874]; add esp, 0x10` at 0x00a3571a)
CDECL_EXPORTS = {"wsprintfA", "wsprintfW", "DbgPrint", "IggyGDrawSendWarning"}
# stdcall imports' argument bytes (their `ret n`), from the SDK prototypes
STDCALL_BYTES = {
    "ImmAssociateContext": 8,
    "Sleep": 4,
    "WaitForSingleObject": 8,
    "ReleaseSemaphore": 12,
    "CreateSemaphoreW": 16,
    "CreateSemaphoreA": 16,
    "CloseHandle": 4,
    "SetEvent": 4,
    "ResetEvent": 4,
    "CreateEventW": 16,
    "CreateEventA": 16,
    "InitializeCriticalSection": 4,
    "DeleteCriticalSection": 4,
    "EnterCriticalSection": 4,
    "LeaveCriticalSection": 4,
    "InitializeCriticalSectionAndSpinCount": 8,
    "GetCurrentThreadId": 0,
    "GetCurrentThread": 0,
    "GetTickCount": 0,
    "timeGetTime": 0,
    "QueryPerformanceCounter": 4,
    "QueryPerformanceFrequency": 4,
    "CreateThread": 24,
    "ResumeThread": 4,
    "SetThreadPriority": 8,
    "GetLastError": 0,
    "TlsAlloc": 0,
    "TlsGetValue": 4,
    "TlsSetValue": 8,
    "TlsFree": 4,
    "SwitchToThread": 0,
    "WaitForMultipleObjects": 16,
    "GetModuleHandleA": 4,
    "GetModuleHandleW": 4,
    "LoadCursorA": 8,
    "LoadCursorW": 8,
    "LoadIconA": 8,
    "LoadIconW": 8,
    "RegisterClassExA": 4,
    "RegisterClassExW": 4,
    "CreateWindowExA": 48,
    "CreateWindowExW": 48,
    "ShowWindow": 8,
    "UpdateWindow": 4,
    "DestroyWindow": 4,
    "DefWindowProcA": 16,
    "DefWindowProcW": 16,
    "RegisterDeviceNotificationA": 12,
    "RegisterDeviceNotificationW": 12,
    "GetClientRect": 8,
    "GetWindowRect": 8,
    "AdjustWindowRect": 12,
    "AdjustWindowRectEx": 16,
    "SetWindowPos": 28,
    "PeekMessageA": 20,
    "PeekMessageW": 20,
    "TranslateMessage": 4,
    "DispatchMessageA": 4,
    "DispatchMessageW": 4,
    "PostQuitMessage": 4,
    "SetCursor": 4,
    "ShowCursor": 4,
    "OutputDebugStringA": 4,
    "OutputDebugStringW": 4,
    "MultiByteToWideChar": 24,
    "WideCharToMultiByte": 32,
    "HeapAlloc": 12,
    "HeapFree": 12,
    "GetProcessHeap": 0,
    "VirtualAlloc": 16,
    "VirtualFree": 12,
    "CreateFileA": 28,
    "CreateFileW": 28,
    "ReadFile": 20,
    "WriteFile": 20,
    "SetFilePointer": 16,
    "GetFileSize": 8,
    "XInputGetState": 8,
    "XInputSetState": 8,
}


def demangled_conv(name):
    """The convention of a C++-decorated name, or None."""
    i = name.find("@@")
    # a template argument list closes with `@@@` (`?getline@?$basic_istream
    # @DU?$char_traits@D@std@@@std@@QAE...`): the name ends at the next `@@`
    while 0 <= i and name[i + 2:i + 3] == "@":
        i = name.find("@@", i + 3)
    if i < 0 or i + 2 >= len(name):
        return None
    s = name[i + 2:]
    if s[0] in "YSKC":  # global, static members
        c = s[1:2]
    elif s[0] in "QIAUMEBJ":  # members: a cv-qualifier first
        c = s[2:3]
    else:
        return None
    conv = {"A": "cdecl", "E": "thiscall", "G": "stdcall", "I": "fastcall"}.get(c)
    if conv == "stdcall" and s[0] not in "YSKC":
        conv = "stdcall-this"  # `this` pushed on the stack too
    return conv


def import_facts(dll, name):
    """(conv, argument bytes or None) of an import."""
    if name is None or name == "<none>":
        return "stdcall", None  # an ordinal of a Win32 DLL (OLEAUT32)
    if name.startswith("?"):
        conv = demangled_conv(name) or "cdecl"
        return conv, (0 if conv == "cdecl" else None)
    m = re.fullmatch(r"_?(\w+)@(\d+)", name)
    if m:
        return "stdcall", int(m.group(2))
    if CRT_DLL.match(dll) or dll.lower() in CDECL_DLLS or name in CDECL_EXPORTS:
        return "cdecl", 0
    return "stdcall", STDCALL_BYTES.get(name)


def read_imports(exe):
    """IAT slot -> (dll, name)."""
    out = subprocess.run(["objdump", "-p", str(exe)], capture_output=True, text=True).stdout
    base = int(re.search(r"ImageBase\s+([0-9a-f]+)", out).group(1), 16)
    slots, slot, dll = {}, None, None
    for line in out.splitlines():
        m = re.match(r"^\s*DLL Name: (\S+)", line)
        if m:
            dll = m.group(1)
            continue
        head = re.match(r"^ [0-9a-f]{8}\t[0-9a-f]{8} [0-9a-f]{8} [0-9a-f]{8} [0-9a-f]{8} ([0-9a-f]{8})$", line)
        if head:
            slot = base + int(head.group(1), 16)
            continue
        fields = line.split()
        if line.startswith("\t") and len(fields) >= 3 and re.fullmatch(r"[0-9a-f]+", fields[0]) \
                and slot is not None:
            # binutils 2.42 "\t<vma> <hint> <name>"; 2.46 adds an ordinal column
            new_layout = len(fields) >= 4 and re.fullmatch(r"<none>|\d+", fields[1]) \
                and re.fullmatch(r"<none>|[0-9a-f]+", fields[2])
            slots[slot] = (dll, fields[3] if new_layout else fields[2])
            slot += 4
    return slots


# --- 2. callees in FFX.exe ---

RET_N = re.compile(r"^(?:repz\s+|bnd\s+)?ret[nd]?\s+0x([0-9a-f]+)$")
RET = re.compile(r"^(?:repz\s+|bnd\s+)?ret[nd]?$")
JMP = re.compile(r"^jmp\s+0x([0-9a-f]+)$")
JMP_SLOT = re.compile(r"^jmp\s+DWORD PTR ds:0x([0-9a-f]+)$")
JCC = re.compile(r"^(j[a-z]+|loop[a-z]*)\s+0x([0-9a-f]+)$")
STOP = re.compile(r"^(int3|hlt|ud2|\(bad\)|\.byte|jmp\s)")
LIMIT = 20000


class Callee:
    """What a call target does with its stack arguments: `pops` (bytes,
    None when unknown), `conv` ('cdecl', 'stdcall', ..., 'code' for a
    function of the image), `what` (a description), `why` (None, or why
    it is unknown)."""

    def __init__(self, pops, conv, what, why=None):
        self.pops, self.conv, self.what, self.why = pops, conv, what, why


class Binary:
    def __init__(self, exe, decomp):
        self.exe = exe
        self.cache = disasm_cache.for_exe(exe)
        self.cache.load()
        self.imports = read_imports(exe)
        self.starts = {}
        tsv = pathlib.Path(decomp) / "functions.tsv"
        if tsv.exists():
            for line in tsv.read_text().splitlines()[1:]:
                f = line.split("\t")
                if len(f) >= 3:
                    self.starts[int(f[0], 16)] = int(f[2])
        self.text, self.next = {}, {}
        self.memo = {}

    def window(self, a):
        size = self.starts.get(a)
        return (a, size + 16 if size else 0x400)

    def prefetch(self, addrs):
        self.cache.prefetch([self.window(a) for a in addrs])

    def _merge(self, a, n):
        code = self.cache.get(a, n)
        addrs = sorted(code)
        for x, y in zip(addrs, addrs[1:]):
            if y <= a + n - 16:
                self.text.setdefault(x, code[x])
                self.next.setdefault(x, y)

    def insn(self, pc):
        if pc not in self.text:
            self._merge(*self.window(pc))
            if pc not in self.text:
                self._merge(pc, 0x400)
            self.text.setdefault(pc, None)
        return self.text[pc]

    def succ(self, pc):
        """The address of the instruction after the one at `pc`."""
        if pc not in self.next:
            self._merge(pc, 0x400)
        return self.next.get(pc)

    def import_callee(self, slot):
        dll, name = self.imports[slot]
        conv, n = import_facts(dll, name)
        pops = 0 if conv == "cdecl" else n
        return Callee(pops, conv, f"{dll}!{name}",
                      None if pops is not None else f"{conv} import {name}: argument bytes unknown (STDCALL_BYTES)")

    def callee(self, f, depth=0):
        if f in self.memo:
            return self.memo[f]
        self.memo[f] = Callee(None, "code", f"{f:#010x}", "recursive tail jump")
        if f in self.imports:
            self.memo[f] = self.import_callee(f)
            return self.memo[f]
        exits, why = [], None
        todo, seen, steps = [f], set(), 0
        while todo and steps < LIMIT:
            pc = todo.pop()
            while pc is not None and pc not in seen and steps < LIMIT:
                seen.add(pc)
                steps += 1
                t = self.insn(pc)
                if t is None:
                    why = f"no code at {pc:#x}"
                    break
                m = RET_N.match(t)
                if m:
                    exits.append(Callee(int(m.group(1), 16), "code", ""))
                    break
                if RET.match(t):
                    exits.append(Callee(0, "code", ""))
                    break
                m = JMP_SLOT.match(t)
                if m and int(m.group(1), 16) in self.imports:
                    exits.append(self.import_callee(int(m.group(1), 16)))
                    break
                m = JMP.match(t)
                if m:
                    tgt = int(m.group(1), 16)
                    if tgt != f and (tgt in self.starts or tgt < f - 0x10000 or tgt > f + 0x10000):
                        if depth < 8:
                            exits.append(self.callee(tgt, depth + 1))
                        break
                    pc = tgt
                    continue
                if STOP.match(t):
                    break
                m = JCC.match(t)
                if m:
                    todo.append(int(m.group(2), 16))
                pc = self.succ(pc)
                if pc in self.starts and pc != f:
                    break  # fell into the next function
        known = {e.pops for e in exits if e.pops is not None}
        imp = [e for e in exits if e.conv not in ("code",)]
        what = f"{f:#010x}" + (f" -> {imp[0].what}" if imp else "")
        conv = imp[0].conv if imp and all(e.conv == imp[0].conv for e in imp) else "code"
        if len(known) == 1 and not any(e.pops is None and e.why for e in exits if e.conv != "code"):
            res = Callee(known.pop(), conv, what)
        elif not exits:
            res = Callee(None, conv, what, why or "no ret found")
        elif len(known) > 1:
            res = Callee(None, conv, what, f"rets pop {sorted(known)}")
        else:
            # imports of unknown size only: the class is still known
            res = Callee(None, conv, what, next((e.why for e in exits if e.why), "unknown"))
        self.memo[f] = res
        return res


# --- 1. calls in the ports ---

POPPING = {"stdcall", "thiscall", "fastcall"}
# `ecx` and `edx` like fastcall, the stack words left to the caller like
# cdecl: MSVC LTCG's custom convention (0x00401690, Settings_ValidateBindings;
# the oracle's Conv::FastcallCallerPops, ported!'s "fastcall-caller-pops")
CALLER_POPS = {"cdecl", "fastcall-caller-pops"}
REG_WORDS = {"thiscall": 1, "fastcall": 2, "fastcall-caller-pops": 2}
CONV_NAMES = {"FastcallCallerPops": "fastcall-caller-pops"}
IMAGE_LO, IMAGE_HI = 0x00401000, 0x00b00000  # .text


def strip(e):
    e = e.strip()
    while True:
        old = e
        e = re.sub(r"^&\s*(mut\s+)?", "", e).strip()
        e = re.sub(r"\s+as\s+(u32|usize|i32|u64)\s*$", "", e).strip()
        if e.startswith("(") and cw.match_close(e, 0) == len(e) - 1:
            e = e[1:-1].strip()
        if e == old:
            return e


LET = re.compile(r"\blet\s+(?:mut\s+)?([a-z_]\w*)\s*(?::\s*[\w:<>\[\]; ]+?\s*)?=(?!=)\s*")


def lets_before(f, fn, off):
    """name -> right-hand side of the last `let name = ..;` before `off` in
    the enclosing function."""
    lo = fn.body_start if fn else max(0, off - 3000)
    body = f.code[lo:off]
    out = {}
    for m in LET.finditer(body):
        k, depth = m.end(), 0
        while k < len(body):
            c = body[k]
            if c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
                if depth < 0:
                    break
            elif c == ";" and depth == 0:
                break
            k += 1
        if k < len(body) and body[k] == ";":
            out[m.group(1)] = body[m.end():k]
    return out


DEREF = [
    re.compile(r"(?:[\w:]+::)?import\((.+)\)", re.S),
    re.compile(r"(?:[\w.()]+\.)?read(?:::<\s*u32\s*>)?\((.+)\)", re.S),
    re.compile(r"\*\s*\(?\s*(.+?)\s+as\s+\*const\s+(?:u32|usize)\s*\)?", re.S),
    re.compile(r"\(\s*(.+?)\s+as\s+\*const\s+(?:u32|usize)\s*\)\s*\.read(?:_unaligned)?\(\)", re.S),
]
REBASE = re.compile(r"(?:[\w.()]+\.|crate::|super::)?rebase\((.+)\)", re.S)


CONST_ANY = re.compile(r"\bconst\s+([A-Z][A-Z0-9_]*)\s*:\s*u32\s*=\s*([^;]+);")


class Consts:
    """`const NAME: u32 = ..;` values: literals, sums of a literal and a
    const, and aliases (`crate::h3_glue18::SURFACE_CTOR`), looked up by
    path (a module path names the file by its stem), else in the same file,
    else the crate's one value, else the only value anywhere. A name with
    different values and no path to tell them apart is not resolved."""

    def __init__(self, files):
        self.defs = {}
        self.stems = {}
        self.names = {}
        for f in files:
            d = {m.group(1): m.group(2).strip() for m in CONST_ANY.finditer(f.code)}
            self.defs[f.rel] = d
            self.stems.setdefault(f.stem, []).append(f.rel)
            for n in d:
                self.names.setdefault(n, []).append(f.rel)
        # `use a::b as c;`, `use a::{b as c, self as d, NAME}`: local name
        # -> the module (stem) it comes from or names
        self.uses = {}
        for f in files:
            u = {}
            for m in re.finditer(r"\buse\s+([\w:]+?)(?:::\{([^}]*)\}|\s+as\s+(\w+))?\s*;", f.code):
                path = [x for x in m.group(1).split("::") if x]
                if m.group(3):
                    u[m.group(3)] = ("mod", path[-1])
                elif m.group(2) is not None:
                    for item in m.group(2).split(","):
                        item = item.strip()
                        im = re.fullmatch(r"(\w+)(?:\s+as\s+(\w+))?", item)
                        if not im:
                            continue
                        name, alias = im.group(1), im.group(2) or im.group(1)
                        if name == "self":
                            u[alias] = ("mod", path[-1])
                        elif re.fullmatch(r"[A-Z][A-Z0-9_]*", name):
                            u[alias] = ("const", path[-1], name)
                        else:
                            u[alias] = ("mod", name)
                elif len(path) >= 2 and re.fullmatch(r"[A-Z][A-Z0-9_]*", path[-1]):
                    u[path[-1]] = ("const", path[-2], path[-1])
            self.uses[f.rel] = u
        self.memo = {}

    @staticmethod
    def crate(rel):
        return rel.split("/")[1]

    def lookup(self, rel, name, depth):
        return self.value(rel, self.defs[rel][name], depth + 1)

    def value(self, rel, expr, depth=0):
        if depth > 8:
            return None
        e = strip(expr)
        n = ta.number(e)
        if n is not None:
            return n
        if re.fullmatch(r"\d+", e):
            return int(e)
        m = re.fullmatch(r"(.+?)\s*\+\s*(.+)", e)
        if m and "(" not in e:
            a, b = self.value(rel, m.group(1), depth + 1), self.value(rel, m.group(2), depth + 1)
            return None if a is None or b is None else (a + b) & 0xFFFFFFFF
        m = re.fullmatch(r"((?:\w+::)*)([A-Z][A-Z0-9_]*)", e)
        if not m:
            return None
        name = m.group(2)
        segs = [x for x in m.group(1).split("::") if x and x not in ("crate", "super", "self", "Self")]
        key = (rel, e)
        if key in self.memo:
            return self.memo[key]
        self.memo[key] = None
        out = None
        used = self.uses.get(rel, {})
        if not segs and name in used and used[name][0] == "const" and name not in self.defs.get(rel, {}):
            segs, name = [used[name][1]], used[name][2]
        if segs and segs[-1] in used and used[segs[-1]][0] == "mod":
            segs = segs[:-1] + [used[segs[-1]][1]]
        if segs:
            mod = segs[-1]
            cands = [r for r in self.stems.get(mod, []) if name in self.defs[r]]
            same = [r for r in cands if self.crate(r) == self.crate(rel)]
            pick = same or cands
            vals = {self.lookup(r, name, depth) for r in pick}
            if len(vals) == 1:
                out = vals.pop()
        if out is None and name in self.defs.get(rel, {}):
            out = self.lookup(rel, name, depth)
        if out is None:
            where = self.names.get(name, [])
            same = [r for r in where if self.crate(r) == self.crate(rel) and r != rel]
            for group in (same, where):
                vals = {self.lookup(r, name, depth) for r in group} - {None}
                if group and len(vals) == 1:
                    out = vals.pop()
                    break
                if group and len(vals) > 1:
                    break
        self.memo[key] = out
        return out


class Ports:
    def __init__(self, root, files, imports):
        self.root, self.files, self.imports = root, files, imports
        self.consts = Consts(files)

    def target(self, f, fn, e, off, depth=0):
        """('addr', a) | ('import', slot) | ('param', index, deref) | None."""
        if depth > 6:
            return None
        e = strip(e)
        lets = lets_before(f, fn, off)
        for _ in range(6):
            if re.fullmatch(r"[a-z_]\w*", e) and e in lets:
                e2 = strip(lets.pop(e))
                if e2 == e:
                    break
                e = e2
            else:
                break
        for pat in DEREF:
            m = pat.fullmatch(e)
            if m:
                inner = self.target(f, fn, m.group(1), off, depth + 1)
                if inner and inner[0] == "addr" and inner[1] in self.imports:
                    return ("import", inner[1])
                if inner and inner[0] == "param" and not inner[2]:
                    return ("param", inner[1], True)
                return None
        m = REBASE.fullmatch(e)
        if m:
            return self.target(f, fn, m.group(1), off, depth + 1)
        a = self.consts.value(f.rel, e)
        if a is not None:
            return ("addr", a)
        if fn is not None and re.fullmatch(r"[a-z_]\w*", e):
            for i, (p, _) in enumerate(fn.params):
                if p == e:
                    return ("param", i, False)
        return None

    @staticmethod
    def conv(fn, text):
        """'cdecl'.. | ('param', index) | None."""
        m = re.search(r"\bConv::(Cdecl|Stdcall|FastcallCallerPops|Fastcall|Thiscall)\b", text)
        if m:
            return CONV_NAMES.get(m.group(1), m.group(1).lower())
        t = text.strip()
        if fn is not None and re.fullmatch(r"[a-z_]\w*", t):
            for i, (p, ty) in enumerate(fn.params):
                if p == t and "Conv" in ty:
                    return ("param", i)
        return None

    @staticmethod
    def words(f, fn, expr, off):
        """int | ('param', index, extra) | None."""
        m = re.fullmatch(r"&?\s*(\w+)\s*\[\s*(\d+)\s*\.\.\s*\]", expr.strip())
        if m:
            w = cw.width_of(f, fn, m.group(1), off)
            return w.n - int(m.group(2)) if w.n is not None and w.n >= int(m.group(2)) else None
        w = cw.width_of(f, fn, expr, off)
        if w.n is not None:
            return w.n
        if w.param is not None:
            return ("param", w.param[0], w.param[1])
        return None


class Site:
    def __init__(self, f, off, how, target, conv, words, t_expr=None, s_expr=None):
        self.f, self.off, self.how = f, off, how
        self.target, self.conv, self.words = target, conv, words
        self.t_expr, self.s_expr = t_expr, s_expr  # as written, for closures

    @property
    def where(self):
        return f"{self.f.rel}:{self.f.line(self.off)}"


METHODS = {
    "call_sym": (0, 1, 2),
    "call_st0": (0, 1, 2),
    "call_st0_f64": (0, 1, 2),
    "call_sym_st0": (0, 1, 2),
    "call_sym_st0_f64": (0, 1, 2),
    "call_n": (0, "cdecl", 1),
    "call_wide": (0, "cdecl", 1),
    "call_sym_wide": (0, "cdecl", 1),
    "call_float": (0, "cdecl", 1),
}
FREE = {
    "call_cdecl": (0, "cdecl", 1),
    "call_cdecl_f64": (0, "cdecl", 1),
    "call_cdecl_f32": (0, "cdecl", 1),
    "call_cdecl_u64": (0, "cdecl", 1),
    "call_cdecl_words": (0, "cdecl", 1),
    "call0": (0, "cdecl", None),
    "call_thiscall": (0, "thiscall", 2),
}
TRANSMUTE = re.compile(r'\blet\s+(?:mut\s+)?(\w+)\s*:\s*unsafe\s+extern\s+"(\w+)"\s+fn\s*\(')
ABI = {"C": "cdecl", "cdecl": "cdecl", "stdcall": "stdcall", "thiscall": "thiscall",
       "fastcall": "fastcall", "system": "stdcall"}


def raw_sites(ports):
    """Every primitive call in the ports, as Site with specs."""
    out = []
    for f in ports.files:
        in_host = f.rel in cw.HOST_FILES
        specs = []
        for name in ["call", *METHODS]:
            for off, args in cw.call_sites(f, name, True):
                if name == "call":
                    if len(args) == 2:
                        spec = (0, "cdecl", 1, "Host::call (cdecl)")
                    elif len(args) == 3:
                        spec = (0, 1, 2, "graphic Host::call")
                    else:
                        continue
                else:
                    t, c, s = METHODS[name]
                    spec = (t, c, s, name)
                specs.append((off, args, spec))
        if f.rel.startswith("crates/ffx-hook/"):
            for name, (t, c, s) in FREE.items():
                for off, args in cw.call_sites(f, name, False):
                    fn = f.enclosing(off)
                    if f.rel.endswith("ported/mod.rs") and fn is not None and fn.name in FREE:
                        continue
                    specs.append((off, args, (t, c, s, name)))
        for off, args, (ti, ci, si, how) in specs:
            fn = f.enclosing(off)
            if in_host and fn is not None and (fn.name in METHODS or fn.name.startswith("call")):
                continue  # the hosts' own forwarding
            if ti >= len(args) or (isinstance(ci, int) and ci >= len(args)) or (si is not None and si >= len(args)):
                continue
            target = ports.target(f, fn, args[ti], off)
            conv = ci if isinstance(ci, str) else ports.conv(fn, args[ci])
            if isinstance(ci, int) and conv is None:
                conv = None
            words = 0 if si is None else ports.words(f, fn, args[si], off)
            if conv in ("fastcall", "fastcall-caller-pops") and isinstance(words, int):
                words = max(words - 2, 0)
            out.append(Site(f, off, how, target, conv, words, args[ti], args[si] if si is not None else None))
        # `let g: unsafe extern "ABI" fn(..) = transmute(EXPR);`
        for m in TRANSMUTE.finditer(f.code):
            p = m.end() - 1
            q = f.match(p)
            if q < 0:
                continue
            types = cw.split_top(f.code[p + 1:q])
            rest = f.code[q + 1:q + 400]
            tm = re.match(r"\s*(?:->\s*[^=;]+?)?\s*=\s*(?:std::mem::|core::mem::|mem::)?transmute\s*(?:::<[^>]*>)?\s*\(", rest)
            if not tm:
                continue
            a = q + 1 + tm.end() - 1
            b = f.match(a)
            if b < 0:
                continue
            conv = ABI.get(m.group(2))
            n = sum(2 if re.search(r"\b(u64|i64|f64)\b", t) else 1 for t in types if t)
            n -= REG_WORDS.get(conv, 0)
            fn = f.enclosing(m.start())
            if in_host and fn is not None and (fn.name in METHODS or fn.name.startswith("call") or fn.params and fn.params[0][0] == "f"):
                continue
            target = ports.target(f, fn, f.code[a + 1:b], m.start())
            if target and target[0] == "param" and fn is not None and in_host:
                continue
            out.append(Site(f, m.start(), f'extern "{m.group(2)}" fn', target, conv, max(n, 0)))
    return out


IDENT = re.compile(r"[a-z_]\w*")


def enclosing_call(f, at):
    """(callee name, argument index) of the call whose arguments hold
    offset `at`, or None."""
    k = at
    while True:
        k = f.code.rfind("(", 0, k)
        if k < 0:
            return None
        q = f.match(k)
        if q > at:
            break
    m = re.search(r"(\w+)\s*(?:::<[^>]*>)?\s*$", f.code[max(0, k - 120):k])
    if not m:
        return None
    depth, j = 0, 0
    for c in f.code[k + 1:at]:
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif c == "," and depth == 0:
            j += 1
    return m.group(1), j


def closure_sites(ports, s, done):
    """The calls made through a closure that forwards to the primitive
    call `s` (`&mut |_, f, a| call_cdecl(f, a)` handed to a bridge, or a
    `let call = |..| ..;`): Sites in the bridges, with `s`'s convention."""
    if s.t_expr is None:
        return None
    t = strip(s.t_expr)
    if not IDENT.fullmatch(t):
        return None
    f = s.f
    fn = f.enclosing(s.off)
    cp = cw.closure_param(f, fn, t, s.off)
    if cp is None:
        return None
    let, k1, at = cp
    k2, fixed = None, s.words
    if s.s_expr is not None:
        se = strip(s.s_expr)
        cs = cw.closure_param(f, fn, se, s.off) if IDENT.fullmatch(se) else None
        if cs is not None and cs[2] == at:
            k2, fixed = cs[1], None
    how = f"closure {f.rel.split('/')[-1]}:{f.line(at)} -> {s.how}"
    carriers = []
    if let:
        carriers.append(("local", f, fn, let, at))
    else:
        ec = enclosing_call(f, at)
        if ec:
            carriers.append(("fn",) + ec)
    out = []
    while carriers:
        c = carriers.pop()
        key = (c[0], c[1] if c[0] == "fn" else c[1].rel, c[2] if c[0] == "fn" else c[3], k1, k2, s.conv, repr(fixed))
        if key in done:
            continue
        done.add(key)
        if c[0] == "local":
            _, g, gfn, name, lo = c
            scopes = [(g, gfn, name, lo, gfn.end if gfn else len(g.code))]
        else:
            _, name, j = c
            scopes = []
            for g in ports.files:
                if name not in g.words:
                    continue
                for d in g.fns:
                    if d.name == name and j < len(d.params) and d.body_start < d.end:
                        scopes.append((g, d, d.params[j][0], d.body_start, d.end))
        for g, gfn, pname, lo, hi in scopes:
            body = g.code[lo:hi]
            for m in re.finditer(r"(?<![\w.])\(?\s*\*?\s*" + re.escape(pname) + r"\s*\)?\s*\(", body):
                p = lo + m.end() - 1
                q = g.match(p)
                if q < 0:
                    continue
                args = cw.split_top(g.code[p + 1:q])
                if k1 >= len(args) or (k2 is not None and k2 >= len(args)):
                    continue
                at2 = lo + body.index(pname, m.start())
                inner = g.enclosing(at2)
                target = ports.target(g, inner, args[k1], at2)
                words = fixed if k2 is None else ports.words(g, inner, args[k2], at2)
                out.append(Site(g, at2, how, target, s.conv, words, args[k1],
                                args[k2] if k2 is not None else None))
            # handed on to another function
            for m in re.finditer(r"(?<![\w.])(?:&\s*mut\s*\*?\s*|&\s*)?" + re.escape(pname) + r"\b(?!\s*[(.:\[])", body):
                ec = enclosing_call(g, lo + m.start())
                if ec and ec[0] != pname:
                    carriers.append(("fn",) + ec)
    return out


def resolve_sites(ports, sites):
    """Follows forwarders to a fixed point; returns (checkable sites,
    unresolved sites, forwarders)."""
    final, unresolved, wrappers = [], [], {}
    names = {}
    for f in ports.files:
        for fn in f.fns:
            names.setdefault(fn.name, set()).add(f.rel)
    todo = list(sites)
    seen = set()
    closures = set()
    while todo:
        s = todo.pop()
        key = (s.f.rel, s.off, repr(s.target), repr(s.conv), repr(s.words))
        if key in seen or s.how.count(" -> ") > 8:
            continue
        seen.add(key)
        params = [x for x in (s.target, s.conv, s.words) if isinstance(x, tuple) and x[0] == "param"]
        if not params:
            if s.target is None and s.conv is not None:
                more = closure_sites(ports, s, closures)
                if more:
                    todo.extend(more)
                    continue
            if s.target is None or s.conv is None:
                unresolved.append(s)
            else:
                final.append(s)
            continue
        fn = s.f.enclosing(s.off)
        if fn is None:
            unresolved.append(s)
            continue
        wkey = (s.f.rel, fn.name, fn.start)
        wrappers.setdefault(wkey, []).append(s)
        # the forwarder's callers
        homes = names.get(fn.name, set())
        for g in ports.files:
            if fn.name not in g.words:
                continue
            if g is not s.f and len(homes) != 1:
                continue
            for method in (False, True):
                if method and not fn.has_self:
                    continue
                for off, args in cw.call_sites(g, fn.name, method):
                    if g is s.f and not method and s.f.visible(fn.name, off) is not fn:
                        continue
                    if g is not s.f and not method and (fn.scope[0] != 0 or fn.name in g.top_names):
                        continue
                    if g.enclosing(off) is fn and off < fn.body_start:
                        continue
                    caller = g.enclosing(off)
                    t, c, w = s.target, s.conv, s.words
                    ok = True
                    if isinstance(t, tuple) and t[0] == "param":
                        if t[1] >= len(args):
                            ok = False
                        else:
                            r = ports.target(g, caller, args[t[1]], off)
                            if t[2]:  # the parameter is an import slot read
                                if r and r[0] == "addr" and r[1] in ports.imports:
                                    r = ("import", r[1])
                                elif r and r[0] == "param" and not r[2]:
                                    r = ("param", r[1], True)
                                else:
                                    r = None
                            t = r
                    if isinstance(c, tuple) and c[0] == "param":
                        c = ports.conv(caller, args[c[1]]) if c[1] < len(args) else None
                    if isinstance(w, tuple) and w[0] == "param":
                        if w[1] < len(args):
                            x = ports.words(g, caller, args[w[1]], off)
                            if isinstance(x, int):
                                w = x + w[2]
                            elif isinstance(x, tuple):
                                w = ("param", x[1], x[2] + w[2])
                            else:
                                w = None
                        else:
                            w = None
                    if ok:
                        te = args[s.target[1]] if isinstance(s.target, tuple) and s.target[0] == "param" else s.t_expr
                        se = args[s.words[1]] if isinstance(s.words, tuple) and s.words[0] == "param" \
                            and s.words[1] < len(args) else s.s_expr
                        if isinstance(s.target, tuple) and s.target[0] == "param" and s.target[2]:
                            te = None  # an import slot read: no closure
                        todo.append(Site(g, off, f"{fn.name} -> {s.how}", t, c, w, te, se))
    return final, unresolved, wrappers


def judge(site, callee):
    """A problem line, or None."""
    conv, words = site.conv, site.words
    pops = callee.pops
    what = callee.what
    cls = callee.conv
    if conv in CALLER_POPS:
        if pops:
            return f"{site.where}: {site.how} calls {what} as {conv}, but it pops {pops // 4} words itself ({pops:#x} bytes)"
        if pops is None and cls in ("stdcall", "stdcall-this", "thiscall", "fastcall") and words:
            return f"{site.where}: {site.how} calls {what} ({cls}, pops its arguments) as {conv}"
        return None
    if conv in POPPING:
        if pops is None and cls == "cdecl" and words:
            return f"{site.where}: {site.how} calls {what} (cdecl, caller pops) as {conv}"
        if pops is not None and isinstance(words, int) and pops != 4 * words:
            if pops == 0:
                return f"{site.where}: {site.how} calls {what} as {conv} with {words} stack words, but it returns with a plain ret (caller pops)"
            return f"{site.where}: {site.how} passes {words} stack words ({conv}) to {what}, whose ret pops {pops // 4}"
    return None


# --- 3. oracle stubs ---

def stub_sites(ports, test_files):
    """(file, off, sym expr, pops literal) of calls to test helpers with
    parameters `sym` and `pops` (words), or `sym`, `conv: Conv` and `n`
    (render.rs `stub_sym`: `n` popped unless `Conv::Cdecl`)."""
    out = []
    helpers = {}
    for f in test_files:
        for fn in f.fns:
            names = [p for p, _ in fn.params]
            conv = any(p == "conv" and "Conv" in t for p, t in fn.params)
            if "sym" in names and ("pops" in names or conv and "n" in names):
                helpers.setdefault(f.rel, []).append(fn)
    for f in test_files:
        defs = helpers.get(f.rel, [])
        if not defs:
            # helpers of a parent test file (tests/x/y.rs uses tests/x.rs's)
            parent = re.sub(r"/[^/]+\.rs$", ".rs", f.rel)
            defs = helpers.get(parent, [])
        for fn in defs:
            names = [p for p, _ in fn.params]
            si = names.index("sym")
            pi = names.index("pops") if "pops" in names else names.index("n")
            ci = None if "pops" in names else names.index("conv")
            for method in (False, True):
                if method != fn.has_self:
                    continue
                for off, args in cw.call_sites(f, fn.name, method):
                    if max(si, pi, ci or 0) >= len(args):
                        continue
                    caller = f.enclosing(off)
                    if caller is fn:
                        continue
                    pops = cw.eval_int(args[pi], {})
                    if ci is not None:
                        c = Ports.conv(None, args[ci])
                        pops = None if c is None else 0 if c in CALLER_POPS else pops
                    out.append((f, caller, off, args[si], pops, fn.name))
    return out


def judge_site(binary, s, unknown, lines, verbose):
    """judge() of one site: None (fine), "unknown" or a problem line."""
    kind, a = s.target[0], s.target[1]
    callee = binary.import_callee(a) if kind == "import" else binary.callee(a)
    what = f"[{a:#010x}] {callee.what}" if kind == "import" else callee.what
    if callee.pops is None and callee.conv == "code":
        unknown.append(f"{s.where}: {what}: {callee.why}")
        return "unknown"
    if verbose:
        lines.append(f"{s.where}: {s.how} {s.conv} {s.words} -> {what} pops {callee.pops} ({callee.conv})")
    shown = Callee(callee.pops, callee.conv, what, callee.why)
    return judge(s, shown)


def analyze(root, exe, decomp, verbose=False):
    root = pathlib.Path(root)
    files, tests = [], []
    for p in sorted(root.glob("crates/*/src/**/*.rs")):
        rel = p.relative_to(root).as_posix()
        if rel.split("/")[1] in SKIP_CRATES:
            continue
        files.append(cw.File(rel, p.read_text(encoding="utf-8")))
    for p in sorted(root.glob("crates/ffx-oracle/tests/**/*.rs")):
        tests.append(cw.File(p.relative_to(root).as_posix(), p.read_text(encoding="utf-8")))
    binary = Binary(exe, decomp)
    for f in files:
        for m in re.finditer(r"\b(?:const|static)\s+(\w+)\s*:\s*&?\s*(?:'static\s+)?\[\s*\(", f.code):
            cw.TABLES.setdefault(m.group(1), []).append(f)
        for name, v in f.rets.items():
            cw.RETS[name] = max(v, cw.RETS.get(name, v))
    ports = Ports(root, files, binary.imports)
    final, unresolved, wrappers = resolve_sites(ports, raw_sites(ports))
    # the stubs resolve their syms against tests and sources together
    tports = Ports(root, files + tests, binary.imports)
    stubs = stub_sites(tports, tests)
    addrs = {s.target[1] for s in final if s.target[0] == "addr"}
    stub_targets = []
    for f, caller, off, sym, pops, how in stubs:
        t = tports.target(f, caller, sym, off)
        stub_targets.append(t)
        if t and t[0] == "addr":
            addrs.add(t[1])
    binary.prefetch(sorted(a for a in addrs if a not in binary.imports))
    problems, used, checked, unknown, lines = [], set(), 0, [], []
    # a forwarder with several calls of its parameter target (bind_tables:
    # 6 words when `with_tables`, else 5) gives each caller every one of
    # them: the caller is right when one of them fits
    groups = {}
    for s in list(final):
        if s.target[0] == "addr" and not (IMAGE_LO <= s.target[1] < IMAGE_HI):
            # data, or a misread closure call: not a function of the image
            final.remove(s)
            unresolved.append(s)
            continue
        groups.setdefault((s.f.rel, s.off, s.target), []).append(s)
    for key, group in sorted(groups.items(), key=lambda kv: (kv[0][0], kv[0][1])):
        verdicts = [judge_site(binary, s, unknown, lines, verbose) for s in group]
        if any(v is None for v in verdicts) or all(v == "unknown" for v in verdicts):
            if any(v is None for v in verdicts):
                checked += 1
            continue
        checked += 1
        a = key[2][1]
        if (key[0], a) in REVIEWED:
            used.add((key[0], a))
            continue
        for v in dict.fromkeys(v for v in verdicts if v != "unknown"):
            problems.append(v)
    stub_checked = 0
    for (f, caller, off, sym, pops, how), t in zip(stubs, stub_targets):
        if pops is None or t is None:
            continue
        callee = binary.import_callee(t[1]) if t[0] == "import" else binary.callee(t[1])
        if callee.pops is None:
            continue
        stub_checked += 1
        if verbose:
            lines.append(f"{f.rel}:{f.line(off)}: stub {how} pops {pops} -> {callee.what} pops {callee.pops}")
        if pops * 4 != callee.pops:
            if (f.rel, t[1]) in REVIEWED:
                used.add((f.rel, t[1]))
                continue
            problems.append(f"{f.rel}:{f.line(off)}: test stub {how} of {callee.what} pops {pops} words; the original pops {callee.pops // 4}")
    binary.cache.save()
    for k in sorted(set(REVIEWED) - used):
        problems.append(f"REVIEWED entry {k} matches no call any more")
    stats = {
        "checked": checked,
        "unresolved": len(unresolved),
        "callee_unknown": len(unknown),
        "forwarders": len(wrappers),
        "stubs": stub_checked,
    }
    if verbose:
        for x in lines:
            print("ok", x)
        for s in unresolved:
            print(f"unresolved {s.where}: {s.how} target={s.target} conv={s.conv}")
        for u in unknown:
            print("unknown callee", u)
    return problems, stats


SELF_TEST = {
    "crates/ffx-prender/src/lib.rs": """
use ffx_graphic::{Conv, Host};
pub const IME_DISASSOCIATE: u32 = 0x009d_a254;
// the thunk of IMM32!ImmAssociateContext (stdcall, 8 bytes)
pub fn direct(h: &mut impl Host, hwnd: u32) {
    h.call_sym(IME_DISASSOCIATE, Conv::Cdecl, &[hwnd, 0]);
}
pub fn direct_ok(h: &mut impl Host, hwnd: u32) {
    h.call_sym(IME_DISASSOCIATE, Conv::Stdcall, &[hwnd, 0]);
}
fn imm(h: &mut impl Host, f: u32, a: &[u32]) -> u32 {
    h.call_sym(f, Conv::Cdecl, a)
}
fn imm_conv(h: &mut impl Host, f: u32, conv: Conv, a: &[u32]) -> u32 {
    h.call_sym(f, conv, a)
}
// 0x00401690: ecx, edx and one stack word, plain ret (caller pops)
pub const PARSE_LIST: u32 = 0x0040_1690;
pub fn custom(h: &mut impl Host, s: u32, out: u32, hwnd: u32) {
    h.call_sym(PARSE_LIST, Conv::FastcallCallerPops, &[s, out, 0x51]);
    h.call_sym(PARSE_LIST, Conv::Fastcall, &[s, out, 0x51]);
    h.call_sym(IME_DISASSOCIATE, Conv::FastcallCallerPops, &[0, 0, hwnd, 0]);
}
pub fn through_helper(h: &mut impl Host, hwnd: u32) {
    imm(h, IME_DISASSOCIATE, &[hwnd, 0]);
    imm_conv(h, IME_DISASSOCIATE, Conv::Stdcall, &[hwnd, 0]);
    imm_conv(h, IME_DISASSOCIATE, Conv::Stdcall, &[hwnd]);
}
""",
    "crates/ffx-prender/src/closure.rs": """
pub const IME: u32 = 0x009d_a254;
pub fn bridge<M>(m: &mut M, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) {
    call(m, IME, &[1, 0]);
}
// a closure's parameter does not bind a later `let` of the same name:
// `f` below is no forward of `rotate`'s `f`, so the `rotate` calls (the
// thiscall 0x00806d10, `ret 4`) are not `call`'s (cdecl)
pub fn erase<M>(
    m: &mut M,
    call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    this_call: &mut impl FnMut(&mut M, u32, u32, &[u32]) -> u32,
) {
    const LEFT: u32 = 0x0080_6d10;
    let mut rotate = |m: &mut M, f: u32, n: u32| {
        this_call(m, f, 0, &[n]);
    };
    let f = load(m);
    call(m, f, &[2]);
    rotate(m, LEFT, 3);
}
""",
    "crates/ffx-hook/src/ported/fixture.rs": """
fn shim() {
    ffx_prender::closure::bridge(&mut mem(), &mut |_, f, a| super::call_cdecl(f, a));
}
fn shim_erase() {
    ffx_prender::closure::erase(
        &mut mem(),
        &mut |_, f, a| super::call_cdecl(f, a),
        &mut |_, f, this, a| super::call_thiscall(f, this, a),
    );
}
""",
    "crates/ffx-oracle/tests/fixture.rs": """
const IME: u32 = 0x009d_a254;
impl Twins {
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32, ret: u32) {}
    fn stub_conv(&mut self, sym: u32, conv: Conv, n: u32) {}
}
fn t(t: &mut Twins) {
    t.stub(IME, false, 2, 0, 1);
    t.stub(IME, false, 2, 2, 1);
    t.stub_conv(IME, Conv::FastcallCallerPops, 4);
}
""",
}
SELF_EXPECT = [
    ("lib.rs:6:", "as cdecl, but it pops 2 words"),
    ("lib.rs:21:", "as fastcall with 1 stack words, but it returns with a plain ret"),
    ("lib.rs:22:", "as fastcall-caller-pops, but it pops 2 words"),
    ("lib.rs:25:", "imm -> call_sym calls"),
    ("lib.rs:27:", "passes 1 stack words (stdcall)"),
    ("fixture.rs:8:", "stub of"),
    ("fixture.rs:10:", "stub_conv of"),
    ("closure.rs:4:", "closure fixture.rs:3 -> call_cdecl calls"),
]


def self_test(exe, decomp):
    with tempfile.TemporaryDirectory() as d:
        for rel, text in SELF_TEST.items():
            p = pathlib.Path(d) / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text)
        problems, stats = analyze(d, exe, decomp)
    bad = []
    for where, text in SELF_EXPECT:
        if not any(where in p and text in p for p in problems):
            bad.append(f"missed: {where} {text}")
    for p in problems:
        if not any(where in p and text in p for where, text in SELF_EXPECT):
            bad.append(f"unexpected: {p}")
    for b in bad:
        print(b)
    print("self-test", "FAILED" if bad else "passed", f"({len(problems)} problems, {stats})")
    return 1 if bad else 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-v", action="store_true")
    ap.add_argument("--root", default=str(ROOT))
    ap.add_argument("--exe", default=os.path.join(os.environ.get("FFX_GAME_DIR", os.path.expanduser("~/steam-games/FFX-X2-HD")), "FFX.exe"))
    ap.add_argument("--decomp", default=os.path.expanduser("~/ffx-work/decomp"))
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    if not os.path.exists(args.exe):
        print(f"no FFX.exe at {args.exe} (set FFX_GAME_DIR)")
        return 2
    if args.self_test:
        return self_test(args.exe, args.decomp)
    problems, stats = analyze(args.root, args.exe, args.decomp, args.v)
    for p in problems:
        print(p)
    print(f"{len(problems)} problems ({stats['checked']} calls checked, {stats['unresolved']} not resolved, "
          f"{stats['callee_unknown']} with an unknown callee convention, {stats['forwarders']} forwarders, "
          f"{stats['stubs']} test stubs checked)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
