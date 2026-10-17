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

CRT intrinsics take their operands on the x87 stack: an St0 shape for
a two-operand intrinsic (or the reverse) is a mismatch too.
"""
import re
import subprocess
from pathlib import Path

# helper name -> (most words it passes, file, regex its source must still match)
HELPERS = {
    "call_cdecl": (12, "crates/ffx-hook/src/ported/mod.rs", r"fn call_cdecl\(f: u32, a: &\[u32\]\)[\s\S]*?a\.len\(\)\.min\(12\)"),
    "call_ptr_f64": (12, "crates/ffx-hook/src/ported/vfx.rs", r"fn call_ptr_f64[\s\S]*?a\.len\(\)\.min\(12\)"),
    "harness_call": (8, "crates/ffx-hook/src/harness.rs", r"type F8 = unsafe extern \"C\" fn\(u32, u32, u32, u32, u32, u32, u32, u32\)"),
    "bridge_call": (6, "crates/ffx-bridge/src/lib.rs", r"impl Host for RawMem \{\s*fn call\(&mut self, addr: u32, args: &\[u32\]\) -> u32 \{\s*type F6"),
    "native_call_float": (6, "crates/ffx-script/src/vm/bridge/run.rs", r"fn call_float\(&mut self, addr: u32, args: &\[u32\]\) -> f64 \{\s*type F6"),
    "native_call_n": (12, "crates/ffx-script/src/vm/bridge/run.rs", r"fn call_n[\s\S]*?args\.len\(\)\.min\(12\)"),
    "graphic_call": (8, "crates/ffx-graphic/src/host.rs", r"fn call\(&mut self, addr: u32, conv: Conv, args: &\[u32\]\) -> u32 \{\s*type C8"),
    "graphic_call_st0": (8, "crates/ffx-graphic/src/host.rs", r"fn call_st0\(&mut self, addr: u32, conv: Conv, args: &\[u32\]\) -> f32 \{\s*let mut a = \[0u32; 8\]"),
}
# helpers whose word counts are match arms: (file, function) -> parsed set
ARM_HELPERS = {
    "st0_cdecl": ("crates/ffx-hook/src/ported/chara.rs", "st0_cdecl"),
    "ci_call": ("crates/ffx-hook/src/ported/chara.rs", "ci_call"),
    "map_float_stack": ("crates/ffx-hook/src/ported/map.rs", "call_float"),
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
    0x007AC9C0: "Battle_QueueCommand reads 5 words; its original callers (natives) push 3",
    0x00861160: "AtelBattleEndCallback reads 4 words; its original caller pushes 3",
}


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
            member = re.match(r"^\t[0-9a-f]+\s+\d+\s+(\S+)", line)
            if member and slot is not None:
                names[slot] = member.group(1)
                slot += 4
        return names

    def lines(self, a, n):
        out = subprocess.run(["objdump", "-d", "--no-show-raw-insn", "-M", "intel",
                              f"--start-address={a:#x}", f"--stop-address={a + n:#x}", str(self.exe)],
                             capture_output=True, text=True).stdout
        return [l.split(":", 1)[1].strip() for l in out.splitlines() if re.match(r"^\s+[0-9a-f]+:", l)]

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

    def scope(code, at):
        """The enclosing function's text before `at`."""
        starts = [m.start() for m in re.finditer(r"\bfn \w+", code[:at])]
        return code[starts[-1]:at] if starts else code[:at]

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
    for f in sorted(repo.glob("crates/*/src/**/*.rs")):
        if "ffx-oracle" in f.parts:
            continue
        code = f.read_text(encoding="utf-8", errors="replace")
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
            conv = "cdecl"
            if name in ("scene_call", "this_call", "call_virtual") or any("Conv::Thiscall" in p for p in parts):
                conv = "thiscall"
            elif any("Conv::Stdcall" in p for p in parts):
                conv = "stdcall"
            elif any("Conv::Fastcall" in p for p in parts):
                conv = "fastcall"
            if name == "call_virtual":
                continue
            k = next((i for i, p in enumerate(parts) if re.fullmatch(r"&\[.*\]|&\w+", p, re.S)), None)
            if k is None or k == 0:
                continue
            count = slice_words(parts[k], code, m.start())
            tgt_i = k - 1
            while tgt_i >= 0 and parts[tgt_i].startswith("Conv::"):
                tgt_i -= 1
            if name == "sfx_call" and k >= 3:
                tgt_i, conv = k - 1, "thiscall"
            if name == "this_call" and k >= 3:
                tgt_i = 1
            a = resolve(parts[tgt_i], code, m.start()) if tgt_i >= 0 else None
            if count is None or a is None:
                continue
            cap = None
            method = "." in full.replace("(c.", "")
            has_conv = any(p.startswith("Conv::") for p in parts)
            if method and name in ("call", "call_sym", "call_st0", "call_sym_st0"):
                cap = 8 if has_conv else 6
            elif name in ("call_cdecl", "call_ptr", "call"):
                cap = 8 if f.name == "loadsave.rs" else 12
            elif name == "st0_cdecl" or (name == "fcall" and "ffx-chara" in f.parts):
                if count not in arm_sets["st0_cdecl"]:
                    problems.append(f"{where}: {name} passes {count} words; st0_cdecl takes {sorted(arm_sets['st0_cdecl'])}")
            elif name == "call_n":
                cap = 12
            elif name == "call_float" and "ffx-script" in f.parts:
                cap = 6
            if cap is not None and count > cap:
                problems.append(f"{where}: {name} passes {count} words; its helper passes at most {cap}")
            reads, pops, forwards = exe.facts(a)
            checked += 1
            stack = count - (2 if conv == "fastcall" else 0)
            if conv != "cdecl" and pops is not None and pops != 4 * max(stack, 0):
                problems.append(f"{where}: {name} passes {stack} stack words ({conv}) to 0x{a:08x}, whose ret pops {pops // 4}")
            elif conv == "cdecl" and pops and pops != 0:
                problems.append(f"{where}: {name} calls 0x{a:08x} as cdecl, but it pops {pops // 4} words itself")
            elif conv == "cdecl" and reads is not None and not forwards and stack < reads and a not in INHERITED:
                problems.append(f"{where}: {name} passes {stack} words to 0x{a:08x}, which reads {reads}")
    return problems, checked
