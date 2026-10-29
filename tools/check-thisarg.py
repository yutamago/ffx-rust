#!/usr/bin/env python3
"""Finds calls into original code that leave out `ecx` where the callee
reads it: a method called as a plain function (m4-pilot, 2026-10-25).

  tools/check-thisarg.py [-v] [--root DIR] [--exe FFX.exe]
  tools/check-thisarg.py --self-test

Ghidra shows some methods as plain functions; their callers leave the
object in `ecx` (`Phyre_PGeometry_PDynamicMesh_getDataBlockCount`
0x00498770: `mov ecx, [edi+4]` ... `push eax; call 0x498770`). A port that
calls such a callee with `Conv::Cdecl` or `Conv::Stdcall` (or through
`ffx_bridge::Host::call`, `call_cdecl`, `call0`) hands it whatever `ecx`
held: the m4-pilot's handler crashed the game that way while its oracle
test, whose stub did not log `ecx`, passed.

Two halves:

1. Which originals take `ecx`. For a function, every path from its entry
   is followed (jumps and both sides of branches, up to a few hundred
   instructions) until `ecx` is written. It takes `ecx` when an
   instruction on some path reads `ecx`, `cx`, `cl` or `ch` before that:
   as a source operand, in an address, as a `rep`/`loop`/`jecxz` count,
   or as the destination of a read-modify-write (`add ecx, 4`, `cmp`,
   `test`, `inc`). `push ecx` is not counted (MSVC's prologues reserve a
   slot with it). A `call` or `jmp` to another function with `ecx` still
   unwritten takes it when the callee does (thunks, wrappers); a call
   otherwise ends the path (the callee may clobber `ecx`).
2. Which calls in crates/*/src (ffx-oracle and ffx-studio left out) pass
   no `ecx`: `.call(target, Conv::Cdecl|Conv::Stdcall, ..)`,
   `.call_st0(..)`/`.call_st0_f64(..)`/`.call_u64(..)`, `.call_sym(SYM, Conv::Cdecl|Conv::Stdcall, ..)`,
   `.call_wide(..)`/`.call_sym_wide(..)` (cdecl),
   `ffx_bridge` `.call(target, &[..])` (cdecl), and ffx-hook's
   `call_cdecl*`/`call0`. The target is resolved when it is a literal, a
   `const NAME: u32 = 0x..` (the same file first, then the crate, then
   anywhere), `x.rebase(..)`/`rebase(..)` of one, or a local assigned one
   of those earlier in the same function (`let f = h.rebase(NAME);`).
   Targets it can't resolve (pointers read from memory, parameters) are
   counted, not reported. A function of the file passing a parameter on
   as such a target is a forwarder: its calls are checked with their
   argument, a call reaching it when its path resolves to it
   (tools/rustpath.py; `other::name(..)` is another module's `name`).
   --self-test checks that on fixtures.

Every resolved call to an original that takes `ecx` is a problem unless
REVIEWED below names it (file, callee, why the port is right). A REVIEWED
entry that no longer matches is reported, so the table stays true.

Prints one line per problem and "N problems"; exits 1 when there are any.
-v also lists every resolved call.

  tools/check-thisarg.py --magic-slots [--write]

prints the `g_MagicImports` slots (0x00c64ce8, the first 1024 words that
point into code) whose function takes `ecx`, and with --write puts them
into crates/ffx-oracle/tests/magic/this_slots.rs: the overlay parity
harness compares `ecx` at a call-out to those slots (elsewhere it is
whatever the overlay code left there). Without --write it fails when that
file is out of date.
"""
import argparse
import collections
import os
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import disasm_cache  # noqa: E402
import rustpath  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
SKIP_CRATES = {"ffx-oracle", "ffx-studio"}
IMAGE_LO, IMAGE_HI = 0x00401000, 0x00b00000  # .text

# (file, callee address) -> why the port's call without ecx is right
REVIEWED = {
    # Ch_EnMapID's first ecx use is `or ecx, 0xffffffff`, which sets it
    # whatever it held (m5 map b3, rcbgIDLoad)
    ("crates/ffx-prender/src/m5m_c.rs", 0x0082_AA70): "or ecx, -1 overwrites ecx",
}

# --- 1. which originals take ecx ---

ECX = re.compile(r"\b(ecx|cx|cl|ch)\b")
WRITES = {"mov", "lea", "movzx", "movsx", "pop", "movd", "cvttss2si", "cvttsd2si", "cvtss2si", "setb"}
ZEROING = {"xor", "sub", "sbb"}
BRANCH = re.compile(r"^(j[a-z]+)\s+0x([0-9a-f]+)$")
CALL = re.compile(r"^call\s+0x([0-9a-f]+)$")
JMP = re.compile(r"^jmp\s+0x([0-9a-f]+)$")
END = re.compile(r"^(ret|retn|int3|ud2|hlt)\b|^jmp\s+(DWORD PTR|e[a-z]x)|^\(bad\)|^\.byte")
LIMIT = 400


def split_insn(text):
    """(mnemonic, [operands]) of an objdump Intel line (prefixes kept on the
    mnemonic: `rep movs ...`)."""
    parts = text.split(None, 1)
    if not parts:
        return "", []
    op = parts[0]
    rest = parts[1] if len(parts) > 1 else ""
    if op in ("rep", "repz", "repnz", "repe", "repne", "lock") and rest:
        sub = rest.split(None, 1)
        op = op + " " + sub[0]
        rest = sub[1] if len(sub) > 1 else ""
    ops = [o.strip() for o in re.split(r",(?![^\[]*\])", rest)] if rest else []
    return op, ops


def effect(text):
    """'read', 'write' or None for `ecx` in one instruction."""
    op, ops = split_insn(text)
    base = op.split()[-1] if op else ""
    if op.startswith(("rep", "repz", "repnz", "repe", "repne")) and op.split()[0] != "lock":
        return "read"  # the count
    if base.startswith("loop") or base == "jecxz":
        return "read"
    if base == "push" and ops == ["ecx"]:
        return None  # a reserved slot in MSVC prologues
    if base == "call":
        return None  # handled by the walker
    if not ops:
        return None
    dst, srcs = ops[0], ops[1:]
    if base == "lea" and len(ops) == 2 and re.fullmatch(r"(\w+),?", dst) and \
            re.fullmatch(r"\[" + re.escape(dst) + r"\+0x0\]", srcs[0].replace(" ", "")):
        return None  # `lea ecx, [ecx+0x0]`: alignment padding
    if any(ECX.search(s) for s in srcs):
        if base in ZEROING and len(ops) == 2 and ops[0] == ops[1] == "ecx":
            return "write"
        return "read"
    if ECX.search(dst):
        if dst == "ecx" and base in WRITES:
            return "write"
        if dst == "ecx" and base == "imul" and len(ops) == 3:
            return "write"
        if dst in ("cl", "ch", "cx") and (base in WRITES or base.startswith("set")):
            # a partial write; taken as the end of the caller's ecx (code
            # that keeps the rest of a `this` and writes cl is not MSVC's)
            return "write"
        if "[" in dst:
            return "read"  # an address
        if base == "xchg":
            return "read"
        return "read"  # read-modify-write, cmp, test
    return None


class Walker:
    def __init__(self, exe, starts):
        self.cache = disasm_cache.for_exe(exe)
        self.cache.load()
        self.starts = sorted(starts)
        self.memo = {}

    def code(self, a):
        return self.cache.get(a, 0x400)

    def takes(self, f, depth=0):
        """Whether the function at `f` reads `ecx` before writing it."""
        if f in self.memo:
            return self.memo[f]
        self.memo[f] = False  # recursion guard
        if depth > 6 or not IMAGE_LO <= f < IMAGE_HI:
            return False
        code = self.code(f)
        addrs = sorted(code)
        index = {a: i for i, a in enumerate(addrs)}
        seen = set()
        todo = [f]
        steps = 0
        result = False
        while todo and not result:
            pc = todo.pop()
            while pc in index and pc not in seen and steps < LIMIT:
                seen.add(pc)
                steps += 1
                text = code[pc]
                m = CALL.match(text)
                if m:
                    if self.takes(int(m[1], 16), depth + 1):
                        result = True
                    break  # ecx clobbered from here
                m = JMP.match(text)
                if m:
                    t = int(m[1], 16)
                    if t in index and f <= t:
                        pc = t
                        continue
                    if self.takes(t, depth + 1):
                        result = True
                    break
                if END.match(text):
                    break
                e = effect(text)
                if e == "read":
                    result = True
                    break
                if e == "write":
                    break
                m = BRANCH.match(text)
                if m:
                    t = int(m[2], 16)
                    if t in index:
                        todo.append(t)
                i = index[pc] + 1
                pc = addrs[i] if i < len(addrs) else None
        self.memo[f] = result
        return result


# --- 2. calls without ecx in the ports ---

TOKEN = re.compile(r'//[^\n]*|/\*.*?\*/|b?"(?:\\.|[^"\\])*"', re.S)
CONST = re.compile(r"\bconst\s+([A-Z][A-Z0-9_]*)\s*:\s*u32\s*=\s*(0x[0-9a-fA-F_]+)\s*;")
FN = re.compile(r"\bfn\s+([a-zA-Z_][a-zA-Z0-9_]*)")
LET = re.compile(r"\blet\s+(?:mut\s+)?([a-z_][a-z0-9_]*)\s*(?::\s*u32\s*)?=\s*([^;]+);")
NO_ECX = re.compile(r"Conv::(Cdecl|Stdcall)\b")


def blank(src):
    return TOKEN.sub(lambda m: re.sub(r"[^\n]", " ", m.group(0)), src)


def args_of(code, i):
    """The top-level arguments of the call whose `(` is at `i`."""
    depth, start, out = 0, i + 1, []
    for j in range(i, len(code)):
        c = code[j]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
            if depth == 0:
                out.append(code[start:j].strip())
                return out
        elif c == "," and depth == 1:
            out.append(code[start:j].strip())
            start = j + 1
    return out


def number(s):
    try:
        return int(s.replace("_", ""), 16) if s.lower().startswith("0x") else None
    except ValueError:
        return None


class Resolver:
    def __init__(self, files):
        self.by_file = {}
        self.by_crate = collections.defaultdict(dict)
        self.anywhere = collections.defaultdict(set)
        for f, code in files.items():
            consts = {n: number(v) for n, v in CONST.findall(code)}
            self.by_file[f] = consts
            crate = f.split("/")[1]
            self.by_crate[crate].update(consts)
            for n, v in consts.items():
                self.anywhere[n].add(v)

    def const(self, f, name):
        if name in self.by_file.get(f, {}):
            return self.by_file[f][name]
        crate = f.split("/")[1]
        if name in self.by_crate[crate]:
            return self.by_crate[crate][name]
        vals = self.anywhere.get(name, set())
        return next(iter(vals)) if len(vals) == 1 else None

    def expr(self, f, e, lets, depth=0):
        """The address an expression names, or None."""
        e = e.strip().lstrip("&").strip()
        if depth > 4:
            return None
        m = re.fullmatch(r"(?:[\w.()]+\.)?rebase\((.+)\)", e) or re.fullmatch(r"crate::rebase\((.+)\)", e)
        if m:
            return self.expr(f, m[1], lets, depth + 1)
        m = re.fullmatch(r"(?:\w+::)*([A-Z][A-Z0-9_]*)", e)
        if m:
            return self.const(f, m[1])
        n = number(e)
        if n is not None:
            return n
        if re.fullmatch(r"[a-z_][a-z0-9_]*", e) and e in lets:
            return self.expr(f, lets[e], {k: v for k, v in lets.items() if k != e}, depth + 1)
        return None


PRIMITIVE = r"((?:ffx_bridge::)?Host::call\b|\.call|\.call_st0(?:_f64)?|\.call_u64|\.call_sym|\.call_sym_st0(?:_f64)?|\.call_sym_u64|\.call_wide|\.call_sym_wide|\bcall_cdecl(?:_f64|_f32|_u64|_words)?|\bcall0)"


def params_of(code, fn_start):
    """The parameter names of the `fn` whose keyword is at `fn_start`."""
    i = code.find("(", fn_start)
    if i < 0:
        return []
    names = []
    for p in args_of(code, i):
        m = re.match(r"(?:mut\s+)?([a-z_][a-z0-9_]*)\s*:", p)
        names.append(m[1] if m else None)
    return names


CLOSURE = re.compile(r"\blet\s+(?:mut\s+)?([a-z_]\w*)\s*=\s*(?:move\s+)?\|([^|]*)\|")


def closure_param(rf, code, lo, off, name):
    """(closure's `let` name, index of `name` among its parameters, offset
    of the `let`) when `name` is a parameter of a `let x = |..| ..`
    closure whose body holds `off`, else None."""
    best = None
    for m in CLOSURE.finditer(code, lo, off):
        params = [re.sub(r"^\s*(?:mut\s+)?", "", x.split(":")[0]).strip() for x in m[2].split(",")]
        if name in params and rf.closure_holds(m.end(), off):
            best = (m[1], params.index(name), m.start())
    return best


def reaches(ws, f, code, at, name, home):
    """Whether the call of `name` at `at` reaches the forwarder `home`: a
    fn's offset (by the call's path, rustpath) or ("closure", lo, hi), a
    local closure seen from its `let` to the end of its function."""
    qual = re.search(r"((?:\w+::)*)$", code[max(0, at - 200):at]).group(1)
    if isinstance(home, tuple):
        return not qual and home[1] < at < home[2]
    if ws is None:
        return True
    return ws.reaches(f, at - len(qual), qual, name, (f, home))


def calls(f, code, resolver, forwarders, ws=None):
    """(line, callee, how) of the calls in `code` that pass no ecx, the
    count of unresolved ones, and the forwarders found: functions and
    local closures passing a parameter of theirs on as such a call's
    target, `(name, home) -> (index, how)` (their callers are then checked
    with that argument). A call reaches a forwarder when its path resolves
    to it (`ws`, rustpath): `other::name(..)` is not this file's `name`,
    nor is a closure `name` in another function."""
    out, unresolved, found = [], 0, {}
    rf = ws.files[f] if ws is not None else None
    fns = [(m.start(), m[1]) for m in FN.finditer(code)]
    names = "|".join(re.escape(n) for n in sorted({k[0] for k in forwarders}))
    pattern = "(?:" + PRIMITIVE + rf"|(?<![\w.])({names or '(?!x)x'}))"
    for m in re.finditer(pattern + r"\s*\(", code):
        name = (m.group(1) or m.group(2) or "").lstrip(".")
        ufcs = name.endswith("Host::call")  # `ffx_bridge::Host::call(h, f, args)`
        i = m.end() - 1
        a = args_of(code, i)
        if not a:
            continue
        start, fname = max(((s, n) for s, n in fns if s < m.start()), default=(0, None))
        if code[start:m.start()].rstrip().endswith("fn " + name) or re.search(r"\bfn\s+" + re.escape(name) + r"\s*$", code[max(0, m.start() - 40):m.start() + len(name)]):
            continue  # the definition itself
        how, at = None, 0
        if m.group(2):
            hit = [v for k, v in sorted(forwarders.items(), key=repr)
                   if k[0] == name and reaches(ws, f, code, m.start(), name, k[1])]
            if not hit or hit[0][0] >= len(a):
                continue
            idx, how = hit[0]
            at = idx
            how = f"{name} -> {how}"
        elif ufcs:
            if len(a) != 3:
                continue
            how, at = "ffx_bridge Host::call (cdecl)", 1
        elif name in ("call", "call_st0", "call_st0_f64", "call_u64", "call_sym", "call_sym_st0", "call_sym_st0_f64", "call_sym_u64"):
            if len(a) >= 3 and NO_ECX.search(a[1]):
                how = f"{name}({a[1]})"
            elif name == "call" and len(a) == 2 and (a[1].startswith("&") or re.search(
                    r"\b" + re.escape(a[1]) + r"\s*:\s*&\s*(?:'\w+\s+)?\[\s*u32\s*\]", code[start:m.start()])):
                how = "ffx_bridge Host::call (cdecl)"
        else:
            how = name
        if how is None:
            continue
        body = code[start:m.start()]
        lets = {}
        for lm in LET.finditer(body):
            lets[lm[1]] = lm[2]
        if rf is not None:
            # inside a `let x = |..| ..` closure its parameters shadow the
            # function's lets, and its own lets come after them
            inner = None
            for cm in CLOSURE.finditer(code, start, m.start()):
                if rf.closure_holds(cm.end(), m.start()):
                    inner = cm
            if inner is not None:
                for x in inner[2].split(","):
                    lets.pop(re.sub(r"^\s*(?:mut\s+)?", "", x.split(":")[0]).strip(), None)
                for lm in LET.finditer(code, inner.end(), m.start()):
                    lets[lm[1]] = lm[2]
        target = resolver.expr(f, a[at], lets)
        line = code.count("\n", 0, m.start()) + 1
        if target is None:
            # a parameter passed on: the function is a forwarder
            e = a[at].strip().lstrip("&")
            for _ in range(4):
                if e in lets:
                    e = lets[e].strip()
            mm = re.fullmatch(r"(?:[\w.()]+\.|crate::)?rebase\((\w+)\)(?:\s+as\s+u32)?", e) \
                or re.fullmatch(r"(?:\w+::)*Mem::rebase\(\s*\w+\s*,\s*(\w+)\s*\)", e)  # `Mem::rebase(h, f)`
            p = mm[1] if mm else e
            ps = params_of(code, start) if fname else []
            cp = closure_param(rf, code, start, m.start(), p) if rf is not None else None
            if cp is not None:
                body = rf.enclosing_fn(m.start())
                found[(cp[0], ("closure", cp[2], body[1] if body else len(code)))] = (cp[1], how)
            elif fname and p in ps:
                found[(fname, start)] = (ps.index(p) - (1 if ps and ps[0] is None else 0), how)
            else:
                unresolved += 1
            continue
        out.append((line, target, how))
    return out, unresolved, found


MAGIC_IMPORTS, MAGIC_WORDS = 0x00C64CE8, 1024
THIS_SLOTS = ROOT / "crates/ffx-oracle/tests/magic/this_slots.rs"


def magic_table(exe):
    """The first MAGIC_WORDS words of g_MagicImports in the file."""
    import struct
    d = pathlib.Path(exe).read_bytes()
    pe = struct.unpack_from("<I", d, 0x3C)[0]
    nsec = struct.unpack_from("<H", d, pe + 6)[0]
    opt = struct.unpack_from("<H", d, pe + 20)[0]
    base = struct.unpack_from("<I", d, pe + 24 + 28)[0]
    rva = MAGIC_IMPORTS - base
    for i in range(nsec):
        sec = pe + 24 + opt + 40 * i
        vsize, va, _rawsize, raw = struct.unpack_from("<IIII", d, sec + 8)
        if va <= rva < va + vsize:
            return struct.unpack_from(f"<{MAGIC_WORDS}I", d, raw + rva - va)
    sys.exit("g_MagicImports is in no section")


def magic_slots(exe, write):
    words = magic_table(exe)
    walker = Walker(exe, [])
    fns = [(i, w) for i, w in enumerate(words) if IMAGE_LO <= w < IMAGE_HI]
    walker.cache.prefetch([(w, 0x400) for _, w in fns])
    # REVIEWED callees overwrite ecx before using it
    reviewed = {t for _, t in REVIEWED}
    this = [i for i, w in fns if walker.takes(w) and w not in reviewed]
    walker.cache.save()
    lines = ["//! `g_MagicImports` slots whose function reads `ecx` before writing it",
             "//! (a method's `this`): generated by `tools/check-thisarg.py",
             "//! --magic-slots --write` from the player's FFX.exe; the harness compares",
             "//! `ecx` at call-outs to these.", "",
             f"/// {len(this)} of the {len(fns)} function slots.",
             "pub const THIS_SLOTS: &[u32] = &["]
    for k in range(0, len(this), 12):
        lines.append("    " + ", ".join(str(x) for x in this[k:k + 12]) + ",")
    lines += ["];", ""]
    text = "\n".join(lines)
    print(f"{len(this)} of {len(fns)} function slots take ecx")
    if write:
        THIS_SLOTS.write_text(text, encoding="utf-8")
        return 0
    # (rustfmt may lay the list out differently)
    def squash(t):
        return re.sub(r"\s+", "", t).replace(",]", "]")

    if not THIS_SLOTS.exists() or squash(THIS_SLOTS.read_text(encoding="utf-8")) != squash(text):
        print(f"{THIS_SLOTS.relative_to(ROOT)} is out of date: run with --write")
        return 1
    return 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-v", action="store_true")
    ap.add_argument("--magic-slots", action="store_true")
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--root", default=str(ROOT))
    ap.add_argument("--exe", default=os.path.join(os.environ.get("FFX_GAME_DIR", os.path.expanduser("~/steam-games/FFX-X2-HD")), "FFX.exe"))
    args = ap.parse_args()
    if args.magic_slots:
        return magic_slots(args.exe, args.write)
    if args.self_test:
        return self_test(args.exe)
    problems, found, unresolved = analyze(args.root, args.exe, REVIEWED, args.v)
    for p in problems:
        print(p)
    print(f"{len(problems)} problems ({len(found)} calls resolved, {unresolved} not resolved)")
    return 1 if problems else 0


def analyze(root, exe, reviewed, verbose=False):
    """(problems, resolved calls, count of unresolved ones)."""
    root = pathlib.Path(root)
    files, sources = {}, {}
    for p in sorted(root.glob("crates/*/src/**/*.rs")):
        rel = p.relative_to(root).as_posix()
        if rel.split("/")[1] in SKIP_CRATES:
            continue
        sources[rel] = p.read_text(encoding="utf-8")
        files[rel] = blank(sources[rel])
    resolver = Resolver(files)
    ws = rustpath.Workspace(sources, str(root))
    found, unresolved = [], 0
    for f, code in files.items():
        # forwarders are per file (private `fn call(h, sym, args)` helpers),
        # found to a fixed point
        fwd = {}
        for _ in range(6):
            c, u, new = calls(f, code, resolver, fwd, ws)
            if set(new) <= set(fwd):
                break
            for k, v in new.items():
                fwd.setdefault(k, v)
        unresolved += u
        found += [(f, *x) for x in c]
    walker = Walker(exe, [])
    walker.cache.prefetch([(t, 0x400) for t in sorted({t for _, _, t, _ in found if IMAGE_LO <= t < IMAGE_HI})])
    problems, used = [], set()
    for f, line, target, how in found:
        takes = walker.takes(target)
        if verbose:
            print(f"{f}:{line}: {target:#010x} {how}{'  TAKES ECX' if takes else ''}")
        if not takes:
            continue
        if (f, target) in reviewed:
            used.add((f, target))
            continue
        problems.append(f"{f}:{line}: {target:#010x} reads ecx on entry, called through {how} (no ecx)")
    walker.cache.save()
    for k in sorted(set(reviewed) - used):
        problems.append(f"REVIEWED entry {k} matches no call any more")
    return problems, found, unresolved


# --self-test: a file's own forwarder and another module's of the same
# name, the call naming that one by its path (2026-10-29: a new
# `bridge::forward` was taken for ffx-pcore's `small::bridge::forward`).
# 0x00806d10 (a thiscall tree rotation, `ret 4`) reads ecx.
SELF_TEST = {
    "crates/ffx-pcore/src/small.rs": """
pub mod bridge {
    use ffx_graphic::{Conv, Host};
    pub fn forward(h: &mut impl Host, sym: u32, p: u32, checked: bool) -> u8 {
        if !checked || p != 0 {
            h.call_sym(sym, Conv::Thiscall(p), &[]);
        }
        1
    }
}
""",
    "crates/ffx-prender/src/m5r_l.rs": """
pub const ROTATE: u32 = 0x0080_6d10;
pub mod bridge {
    use ffx_graphic::{Conv, Host};
    use ffx_pcore::small::bridge as small;
    pub fn forward(h: &mut impl Host, target: u32, conv: Conv, args: &[u32]) -> u32 {
        h.call_sym(target, Conv::Cdecl, args)
    }
    pub fn here(h: &mut impl Host, p: u32) {
        ffx_pcore::small::bridge::forward(h, 0x0080_6d10, p, false);
        small::forward(h, 0x0080_6d10, p, true);
        forward(h, 0x0080_6d10, Conv::Cdecl, &[p]);
        self::forward(h, 0x0080_6d10, Conv::Cdecl, &[p]);
    }
}
""",
    # a local closure `call` and another function's nested `fn call`:
    # each call reaches the one in its own function
    "crates/ffx-sg/src/state.rs": """
pub mod bridge {
    pub fn one<H: ffx_bridge::Host>(h: &mut H) {
        let call = |h: &mut H, f: u32, args: &[u32]| {
            let f = h.rebase(f);
            h.call(f, args)
        };
        call(h, 0x0080_6d10, &[]);
    }
    pub fn two(h: &mut impl ffx_bridge::Host) {
        fn call(h: &mut impl ffx_bridge::Host, f: u32) -> u32 {
            h.call(f, &[])
        }
        call(h, 0x0080_6d10);
    }
    pub fn three<H: ffx_bridge::Host>(h: &mut H) {
        let call = |h: &mut _, f: u32, args: &[u32]| ffx_bridge::Host::call(h, f, args);
        call(h, 0x0080_6d10, &[1]);
    }
}
""",
}
SELF_EXPECT = [
    "m5r_l.rs:12: 0x00806d10 reads ecx on entry, called through forward -> call_sym",
    "m5r_l.rs:13: 0x00806d10 reads ecx on entry, called through forward -> call_sym",
    "state.rs:8: 0x00806d10 reads ecx on entry, called through call -> ffx_bridge Host::call",
    "state.rs:14: 0x00806d10 reads ecx on entry, called through call -> ffx_bridge Host::call",
    "state.rs:18: 0x00806d10 reads ecx on entry, called through call -> ffx_bridge Host::call",
]


def self_test(exe):
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        for rel, text in SELF_TEST.items():
            p = pathlib.Path(d) / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text)
        problems, found, unresolved = analyze(d, exe, {})
    bad = [f"missed: {e}" for e in SELF_EXPECT if not any(e in p for p in problems)]
    bad += [f"unexpected: {p}" for p in problems if not any(e in p for e in SELF_EXPECT)]
    for b in bad:
        print(b)
    print("self-test", "FAILED" if bad else "passed", f"({len(problems)} problems, {len(found)} calls resolved)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
