"""Argument counts at the bridges' calls into original code (check-ported).

The hook-side call helpers take word slices, and a count they lack is
silently dropped or answered with NaN (chara's st0_cdecl took one word, so
Ch_AutoMotion's three-word Sg_AngleApproach call got NaN in the game);
the oracle's own closures never run them. For every call whose target
address and literal argument list can be read from the source, this checks:

- the count fits the helper (HELPERS below, each verified against its
  source, so a changed helper fails loudly instead of going stale);
- a callee-pops convention (stdcall, thiscall, fastcall) passes exactly
  the words the callee's `ret n` pops;
- a cdecl call passes at least the stack words the callee reads (measured
  from its machine code, which the dump's signatures do not always match).

Calls through a file's own wrapper functions (any name, such as
chr_plate.rs's `Plate::run`) are checked as the call each forwards to: a
wrapper whose body passes a parameter target (or a fixed one) and a
parameter word slice to a recognised call inherits its convention and word
capacity; a wrapper around a closure field gets the callee-side checks only.

CRT intrinsics take their operands on the x87 stack: an St0 shape for
a two-operand intrinsic (or the reverse) is a mismatch too.
"""
import bisect
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import disasm_cache  # noqa: E402

# helper name -> (most words it passes, file, regex its source must still match)
HELPERS = {
    "call_cdecl": (12, "crates/ffx-hook/src/ported/mod.rs", r"fn call_cdecl\(f: u32, a: &\[u32\]\)[\s\S]*?a\.len\(\)\.min\(12\)"),
    "call_ptr_f64": (12, "crates/ffx-hook/src/ported/vfx.rs", r"fn call_ptr_f64[\s\S]*?a\.len\(\)\.min\(12\)"),
    "harness_call": (8, "crates/ffx-hook/src/harness.rs", r"type F8 = unsafe extern \"C\" fn\(u32, u32, u32, u32, u32, u32, u32, u32\)"),
    "bridge_call": (6, "crates/ffx-bridge/src/lib.rs", r"impl Host for RawMem \{\s*fn call\(&mut self, addr: u32, args: &\[u32\]\) -> u32 \{\s*type F6"),
    "native_call_float": (6, "crates/ffx-script/src/vm/bridge/run.rs", r"fn call_float\(&mut self, addr: u32, args: &\[u32\]\) -> f64 \{\s*type F6"),
    "native_call_n": (12, "crates/ffx-script/src/vm/bridge/run.rs", r"pub const CALL_N_WORDS: usize = 12;[\s\S]*?fn call_n\(&mut self[\s\S]*?args\.len\(\) > CALL_N_WORDS"),
    "graphic_call": (8, "crates/ffx-graphic/src/host.rs", r"fn call\(&mut self, addr: u32, conv: Conv, args: &\[u32\]\) -> u32 \{\s*type C8"),
    "graphic_call_st0": (8, "crates/ffx-graphic/src/host.rs", r"fn call_st0\(&mut self, addr: u32, conv: Conv, args: &\[u32\]\) -> f32 \{\s*let mut a = \[0u32; 8\]"),
}
# helpers whose word counts are match arms: (file, function) -> parsed set
ARM_HELPERS = {
    "st0_cdecl": ("crates/ffx-hook/src/ported/chara.rs", "st0_cdecl"),
    "ci_call": ("crates/ffx-hook/src/ported/chara.rs", "ci_call"),
    "map_float_stack": ("crates/ffx-hook/src/ported/map.rs", "call_float"),
    # ffx-system's `fcall` callbacks; the 0x00901700 port passed
    # AdjustButtonIconY five words when it took four (NaN: no button icon)
    "system_fcall": ("crates/ffx-hook/src/ported/system.rs", "fcall"),
}
TWO_OPERAND = {"_CIatan2", "_CIfmod", "_CIpow"}
# The crt_trig helpers' slots (one-operand intrinsics), checked by name.
TRIG_SLOTS = {
    "crates/ffx-hook/src/ported/battle.rs": [("Cos", 0xc3b4), ("Sin", 0xc3b8), ("Atan", 0xc450)],
    "crates/ffx-hook/src/ported/ps2_emu.rs": [("Cos", 0xc3b4), ("Sin", 0xc3b8)],
    "crates/ffx-hook/src/ported/vfx.rs": [("cos", 0xc3b4)],
}
# Callees that read more words than their original callers push: the rest is
# whatever the caller's stack holds there, which a bridge cannot reproduce.
# Reported to the coordinator (chara round 14) for the owning module.
INHERITED = {
}


class Deferred:
    """A word call whose callee-side checks wait for the disassembly."""

    def __init__(self, where, label, conv, count, a):
        self.where, self.label, self.conv, self.count, self.a = where, label, conv, count, a


def balanced(code, start):
    depth = 0
    for j in range(start, len(code)):
        c = code[j]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
            if depth == 0:
                return code[start:j + 1]
    return None


def split_top(body):
    depth, out, cur = 0, [], ""
    for c in body:
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == "," and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += c
    if cur.strip():
        out.append(cur)
    return [p.strip() for p in out]


def arms(text, fn, pattern):
    m = re.search(rf"fn {fn}\b[\s\S]*?\n}}\n", text)
    if not m:
        return None
    return {len(split_top(a)) for a in re.findall(pattern, m.group(0))}


class Exe:
    """Stack-argument facts from FFX.exe's machine code."""

    def __init__(self, exe, sizes):
        self.exe, self.sizes, self.cache = exe, sizes, {}
        self.dis = disasm_cache.for_exe(exe)
        self.dis.load()
        self.imports = self._imports()

    def _imports(self):
        """IAT slot address -> imported name (each DLL's members in order
        from its First Thunk)."""
        out = subprocess.run(["objdump", "-p", str(self.exe)], capture_output=True, text=True).stdout
        base = int(re.search(r"ImageBase\s+([0-9a-f]+)", out).group(1), 16)
        names, slot = {}, None
        for line in out.splitlines():
            head = re.match(r"^ [0-9a-f]{8}\t[0-9a-f]{8} [0-9a-f]{8} [0-9a-f]{8} [0-9a-f]{8} ([0-9a-f]{8})$", line)
            if head:
                slot = base + int(head.group(1), 16)
                continue
            # binutils 2.42: "\t<vma> <hint> <name>"; 2.46 adds an ordinal
            # column ("\t<vma> <none> <hint> <name> [bound-to]", "<none>"
            # for each absent field). One IAT slot per member line in both.
            fields = line.split()
            if line.startswith("\t") and len(fields) >= 3 and re.fullmatch(r"[0-9a-f]+", fields[0]) \
                    and slot is not None:
                new_layout = len(fields) >= 4 and re.fullmatch(r"<none>|\d+", fields[1]) \
                    and re.fullmatch(r"<none>|[0-9a-f]+", fields[2])
                names[slot] = fields[3] if new_layout else fields[2]
                slot += 4
        return names

    def lines(self, a, n):
        """Instruction texts of `n` bytes from `a` (tools/disasm_cache.py)."""
        return list(self.dis.get(a, n).values())

    def prefetch(self, addrs, jobs=None):
        """Disassembles the functions at `addrs` in parallel (missing ones)."""
        self.dis.prefetch(((a, self.sizes[a]) for a in addrs if self.sizes.get(a)), jobs)

    def intrinsic(self, a):
        """The CRT intrinsic an import thunk at `a` jumps to, or None."""
        ls = self.lines(a, 6)
        m = ls and re.match(r"jmp\s+DWORD PTR ds:0x([0-9a-f]+)", ls[0])
        return self.imports.get(int(m.group(1), 16)) if m else None

    def facts(self, a):
        """(stack words read or None, ret pop bytes or None, forwards)."""
        if a in self.cache:
            return self.cache[a]
        size = self.sizes.get(a)
        if not size:
            self.cache[a] = (None, None, False)
            return self.cache[a]
        ls = self.lines(a, size)
        frame = len(ls) > 1 and ls[0] == "push   ebp" and ls[1] == "mov    ebp,esp"
        reads, pops, forwards, delta = -1, None, False, 0
        for l in ls:
            if frame and re.search(r"lea\s+\w+,\[ebp\+0x[0-9a-f]+\]", l):
                # the address of an argument: a varargs walk (printf-like)
                self.cache[a] = (None, None, True)
                return self.cache[a]
            if frame:
                for m in re.finditer(r"\[ebp\+0x([0-9a-f]+)\]", l):
                    off = int(m.group(1), 16)
                    if off >= 8:
                        reads = max(reads, (off - 8) // 4)
            else:
                for m in re.finditer(r"\[esp\+0x([0-9a-f]+)\]", l):
                    off = int(m.group(1), 16) - delta
                    if off >= 4:
                        reads = max(reads, (off - 4) // 4)
                if l.startswith("push"):
                    delta += 4
                elif l.startswith("pop"):
                    delta -= 4
                elif m := re.match(r"sub\s+esp,0x([0-9a-f]+)", l):
                    delta += int(m.group(1), 16)
                elif m := re.match(r"add\s+esp,0x([0-9a-f]+)", l):
                    delta -= int(m.group(1), 16)
            if m := re.match(r"ret\s+0x([0-9a-f]+)", l):
                pops = int(m.group(1), 16)
            elif l.startswith("ret"):
                pops = pops if pops is not None else 0
            if re.match(r"jmp\s+0x[0-9a-f]+$", l):
                target = int(l.split("0x")[1], 16)
                if not a <= target < a + size:
                    forwards = True
        self.cache[a] = (reads + 1, pops, forwards)
        return self.cache[a]


def check(repo, exe_path, sizes):
    """Yields problem lines; returns via StopIteration the count of checked calls."""
    repo = Path(repo)
    problems, checked = [], 0
    wrapped_sites = check.wrapped = []
    for name, (n, f, pat) in HELPERS.items():
        if not re.search(pat, (repo / f).read_text(encoding="utf-8")):
            problems.append(f"callarity: helper {name} ({f}) no longer passes {n} words as recorded; update tools/callarity.py")
    arm_sets = {}
    for name, (f, fn) in ARM_HELPERS.items():
        text = (repo / f).read_text(encoding="utf-8")
        pat = r"Stack\(&\[([\w\s,]*)\]\)" if name == "map_float_stack" else r"^\s*\[([\w\s,]*)\]\s*=>"
        got = arms(text, fn, pat if name == "map_float_stack" else re.compile(pat, re.M))
        if not got:
            problems.append(f"callarity: no argument arms found in {f} {fn}")
        arm_sets[name] = got or set()
    exe = Exe(exe_path, sizes)
    # The trig helpers that name their intrinsic by its import slot.
    for f, pairs in TRIG_SLOTS.items():
        text = (repo / f).read_text(encoding="utf-8")
        for word, slot in pairs:
            m = re.search(rf"{word}\s*=>\s*0x00b0_{slot:04x}|if cos \{{ 0x00b0_{slot:04x}", text) if word else None
            found = exe.imports.get(0x00B00000 + slot)
            want = {"Cos": "_CIcos", "Sin": "_CIsin", "Atan": "_CIatan", "cos": "_CIcos"}[word]
            if m is None or found != want:
                problems.append(f"callarity: {f} maps {word} to slot 0x00b0{slot:04x} ({found}), expected {want}")
    consts = {}
    for f in repo.glob("crates/*/src/**/*.rs"):
        for m in re.finditer(r"const (\w+): u32 = (0x[0-9a-fA-F_]+);", f.read_text(encoding="utf-8", errors="replace")):
            consts.setdefault(m.group(1), set()).add(int(m.group(2).replace("_", ""), 16))

    fn_starts = {}

    def scope(code, at):
        """The enclosing function's text before `at`."""
        # the last `fn name` match that fits wholly before `at` (matching
        # in code[:at], recomputed per call, took most of the run)
        starts = fn_starts.get(code)
        if starts is None:
            starts = fn_starts[code] = [m.start() for m in re.finditer(r"\bfn \w+", code)]
        i = bisect.bisect_right(starts, at - 4)
        return code[starts[i - 1]:at] if i else code[:at]

    def resolve(expr, code, at):
        expr = expr.strip()
        for pat in (r"(?:[\w.]+\.)?rebase\((.*)\)", r"r\(\s*\w+\s*,\s*(.*)\)"):
            m = re.fullmatch(pat, expr, re.S)
            if m:
                expr = m.group(1).strip()
        expr = expr.split("::")[-1]
        if re.fullmatch(r"0x[0-9a-fA-F_]+", expr):
            return int(expr.replace("_", ""), 16)
        if re.fullmatch(r"[A-Z][A-Z0-9_]*", expr):
            vals = consts.get(expr, set())
            if len(vals) > 1:
                # several files define it: the calling file's own one
                own = set(int(v.replace("_", ""), 16) for v in
                          re.findall(rf"const {expr}: u32 = (0x[0-9a-fA-F_]+);", code_file[0]))
                vals = own if len(own) == 1 else vals
            return next(iter(vals)) if len(vals) == 1 else None
        if re.fullmatch(r"[a-z_]\w*", expr):
            body = scope(code, at)
            lets = list(re.finditer(rf"let {expr}\s*=\s*([^;]+);", body))
            if lets:
                return resolve(lets[-1].group(1), body, lets[-1].start())
        return None

    def slice_words(arg, code, at):
        arg = arg.strip()
        m = re.fullmatch(r"&\[(.*)\]", arg, re.S)
        if m:
            return len(split_top(m.group(1)))
        m = re.fullmatch(r"&(\w+)", arg)
        if m:
            text = scope(code, at)
            lets = list(re.finditer(rf"let {m.group(1)}\s*=\s*\[", text))
            if lets:
                body = balanced(text, lets[-1].end() - 1)
                return len(split_top(body[1:-1])) if body else None
        return None

    sites = re.compile(r"(?<![\w.])((?:[\w]+\.)*(?:\(?c\.)?(call\w*|\w+_call\w*|callf|fcall|ci)\)?)\(")

    def classify(f, full, name, parts):
        """(conv, word cap or None, closure, arm set name or None) of a call
        through a named helper."""
        conv = "cdecl"
        if name in ("scene_call", "this_call", "call_virtual") or any("Conv::Thiscall" in p for p in parts):
            conv = "thiscall"
        elif any("Conv::Stdcall" in p for p in parts):
            conv = "stdcall"
        elif any("Conv::Fastcall" in p for p in parts):
            conv = "fastcall"
        cap, arms_of = None, None
        method = "." in full.replace("(c.", "")
        has_conv = any(p.startswith("Conv::") for p in parts)
        # `(self.call)(..)`: a closure field, whose helper the call site
        # doesn't name; only the callee side is checked
        closure = full.endswith(")") and "(c." not in full
        if closure:
            pass
        elif method and name in ("call", "call_sym", "call_st0", "call_sym_st0"):
            cap = 8 if has_conv else 6
        elif name in ("call_cdecl", "call_ptr", "call"):
            cap = 8 if f.name == "loadsave.rs" else 12
        elif name == "st0_cdecl" or (name == "fcall" and "ffx-chara" in f.parts):
            arms_of = "st0_cdecl"
        elif name == "fcall" and "ffx-system" in f.parts:
            arms_of = "system_fcall"
        elif name == "call_n":
            cap = 12
        elif name == "call_float" and "ffx-script" in f.parts:
            cap = 6
        return conv, cap, closure, arms_of

    def word_call(where, label, conv, cap, arms_of, count, a):
        """Checks one call of `count` words to `a`; returns 1 (checked)."""
        if arms_of == "st0_cdecl" and count not in arm_sets["st0_cdecl"]:
            problems.append(f"{where}: {label} passes {count} words; st0_cdecl takes {sorted(arm_sets['st0_cdecl'])}")
        elif arms_of == "system_fcall" and count not in arm_sets["system_fcall"]:
            problems.append(f"{where}: {label} passes {count} words; system.rs fcall takes {sorted(arm_sets['system_fcall'])}")
        if cap is not None and count > cap:
            problems.append(f"{where}: {label} passes {count} words; its helper passes at most {cap}")
        # the callee side needs its machine code: decided at the end, once
        # every callee is disassembled (in parallel), in this order
        problems.append(Deferred(where, label, conv, count, a))
        return 1

    def callee_side(d):
        reads, pops, forwards = exe.facts(d.a)
        where, label, conv, a = d.where, d.label, d.conv, d.a
        stack = d.count - (2 if conv == "fastcall" else 0)
        if conv != "cdecl" and pops is not None and pops != 4 * max(stack, 0):
            return f"{where}: {label} passes {stack} stack words ({conv}) to 0x{a:08x}, whose ret pops {pops // 4}"
        if conv == "cdecl" and pops and pops != 0:
            return f"{where}: {label} calls 0x{a:08x} as cdecl, but it pops {pops // 4} words itself"
        if conv == "cdecl" and reads is not None and not forwards and stack < reads and a not in INHERITED:
            return f"{where}: {label} passes {stack} words to 0x{a:08x}, which reads {reads}"
        return None

    def slice_index(parts, slice_params=()):
        return next((i for i, p in enumerate(parts)
                     if re.fullmatch(r"&\[.*\]|&\w+", p, re.S) or p in slice_params), None)

    def target_index(parts, k):
        i = k - 1
        while i >= 0 and parts[i].startswith("Conv::"):
            i -= 1
        return i

    def params(code, at):
        """Parameter names of the `fn` whose name ends at `at` (self
        excluded), their types, and whether it takes self."""
        sig = balanced(code, code.index("(", at))
        names, types, has_self = [], [], False
        for p in split_top(sig[1:-1]) if sig else []:
            if re.fullmatch(r"&?(?:'\w+\s+)?(?:mut\s+)?self", p):
                has_self = True
                continue
            m = re.fullmatch(r"(?:mut\s+)?(\w+)\s*:\s*(.+)", p, re.S)
            names.append(m.group(1) if m else None)
            types.append(m.group(2).strip() if m else "")
        return names, types, has_self

    def wrappers(code, f):
        """Helper functions named outside `sites` that forward a parameter
        target and a parameter word slice to one recognised call: name ->
        (target param index or fixed address, slice param index, conv, cap,
        closure, arm set, has self). Wrappers of wrappers resolve too."""
        defs = {}
        for m in re.finditer(r"\bfn (\w+)\s*(?:<[^>{]*>)?\s*\(", code):
            if sites.fullmatch(m.group(1) + "("):
                continue
            body_at = code.find("{", balanced(code, m.end() - 1) and m.end() - 1 + len(balanced(code, m.end() - 1)))
            body = balanced(code, body_at) if body_at >= 0 else None
            if body:
                defs.setdefault(m.group(1), []).append((m.start(), body_at, body))
        found = {}
        for _ in range(4):
            grew = False
            for fname, ds in defs.items():
                if fname in found:
                    continue
                got = set()
                for start, body_at, body in ds:
                    names, types, has_self = params(code, start)
                    slice_params = [n for n, t in zip(names, types) if n and re.fullmatch(r"&(?:'\w+\s+)?\[u32(?:;\s*\w+)?\]", t)]
                    inner = []
                    for im in sites.finditer(body):
                        if body[max(0, im.start() - 3):im.start()] == "fn ":
                            continue
                        inner.append(("site", im))
                    for wname in found:
                        for im in re.finditer(rf"(?<![\w:])((?:[\w]+\.)*)({wname})\(", body):
                            if body[max(0, im.start() - 3):im.start()] != "fn ":
                                inner.append(("wrap", im))
                    for kind, im in inner:
                        args = balanced(body, im.end() - 1)
                        if not args:
                            continue
                        parts = split_top(args[1:-1])
                        if kind == "site":
                            k = slice_index(parts, slice_params)
                            if k is None or k == 0 or parts[k] not in slice_params:
                                continue
                            conv, cap, closure, arms_of = classify(f, im.group(1), im.group(2), parts)
                            t = parts[target_index(parts, k)]
                        else:
                            ti, k, conv, cap, closure, arms_of, wself = found[im.group(2)]
                            if wself and not im.group(1):
                                continue
                            if k >= len(parts) or parts[k] not in slice_params:
                                continue
                            if isinstance(ti, tuple):
                                got.add((ti, names.index(parts[k]), conv, cap, closure, arms_of, has_self))
                                continue
                            if ti >= len(parts):
                                continue
                            t = parts[ti]
                        # the target: a parameter (through lets and rebase) or a constant
                        expr = t.strip()
                        local = body[:body.find(args) if args in body else len(body)]
                        for _ in range(4):
                            lets = list(re.finditer(rf"let {re.escape(expr)}\s*=\s*([^;]+);", local)) \
                                if re.fullmatch(r"[a-z_]\w*", expr) else []
                            if not lets:
                                break
                            expr = lets[-1].group(1).strip()
                        mm = re.fullmatch(r"(?:[\w.]+\.)?rebase\((.*)\)", expr, re.S)
                        if mm:
                            expr = mm.group(1).strip()
                        if expr in names:
                            got.add((names.index(expr), names.index(parts[k]), conv, cap, closure, arms_of, has_self))
                        else:
                            a = resolve(expr, code, body_at + body.find(args))
                            if a is not None:
                                got.add(((a,), names.index(parts[k]), conv, cap, closure, arms_of, has_self))
                if len(got) == 1:
                    found[fname] = got.pop()
                    grew = True
            if not grew:
                break
        return found

    code_file = [""]
    for f in sorted(repo.glob("crates/*/src/**/*.rs")):
        if "ffx-oracle" in f.parts:
            continue
        code = f.read_text(encoding="utf-8", errors="replace")
        code_file[0] = code
        for m in sites.finditer(code):
            if code[max(0, m.start() - 3):m.start()] == "fn ":
                continue
            full, name = m.group(1), m.group(2)
            args = balanced(code, m.end() - 1)
            if not args:
                continue
            parts = split_top(args[1:-1])
            line = code[:m.start()].count("\n") + 1
            where = f"{f.relative_to(repo)}:{line}"
            # intrinsic shapes
            shape = None
            if name == "ci" and len(parts) == 3:
                target, count = parts[1], slice_words(parts[2], code, m.start())
                shape = count
            elif name == "call_intrinsic" and len(parts) == 2:
                target, shape = parts[0], 1
            elif name == "call_intrinsic2" and len(parts) == 3:
                target, shape = parts[0], 2
            elif name == "fcall" and len(parts) == 3 and "FArgs::St0" in parts[2]:
                target, shape = parts[1], (2 if "FArgs::St01" in parts[2] else 1)
            if shape is not None:
                a = resolve(target, code, m.start())
                if a is None:
                    continue
                intr = exe.intrinsic(a)
                if intr and intr.startswith("_CI"):
                    want = 2 if intr in TWO_OPERAND else 1
                    checked += 1
                    if shape != want:
                        problems.append(f"{where}: {name} gives {intr} {shape} x87 operand(s); it takes {want}")
                continue
            # word calls: find the slice and the target before it
            if name == "call_virtual":
                continue
            conv, cap, closure, arms_of = classify(f, full, name, parts)
            k = slice_index(parts)
            if k is None or k == 0:
                continue
            count = slice_words(parts[k], code, m.start())
            tgt_i = target_index(parts, k)
            if name == "sfx_call" and k >= 3:
                tgt_i, conv = k - 1, "thiscall"
            if name == "this_call" and k >= 3:
                tgt_i = 1
            a = resolve(parts[tgt_i], code, m.start()) if tgt_i >= 0 else None
            if count is None or a is None:
                continue
            checked += word_call(where, name, conv, cap, arms_of, count, a)
        # calls through the file's own wrappers (any name), checked as the
        # call each forwards to
        for wname, (ti, k, conv, cap, closure, arms_of, wself) in wrappers(code, f).items():
            for m in re.finditer(rf"(?<![\w:])((?:[\w]+\.)*)({wname})\(", code):
                if code[max(0, m.start() - 3):m.start()] == "fn " or (wself and not m.group(1)):
                    continue
                args = balanced(code, m.end() - 1)
                parts = split_top(args[1:-1]) if args else []
                if k >= len(parts):
                    continue
                count = slice_words(parts[k], code, m.start())
                if isinstance(ti, tuple):
                    a = ti[0]
                else:
                    a = resolve(parts[ti], code, m.start()) if ti < len(parts) else None
                if count is None or a is None:
                    continue
                where = f"{f.relative_to(repo)}:{code[:m.start()].count(chr(10)) + 1}"
                checked += word_call(where, wname, conv, cap, arms_of, count, a)
                wrapped_sites.append(where)
    exe.prefetch({d.a for d in problems if isinstance(d, Deferred)})
    out = []
    for d in problems:
        if isinstance(d, Deferred):
            d = callee_side(d)
        if d is not None:
            out.append(d)
    return out, checked
