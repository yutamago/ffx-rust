#!/usr/bin/env python3
"""Duplicate families across FFX.exe: functions with the same code.

Every function of the Ghidra dump ($FFX_WORK/decomp/functions.tsv) is
normalised from objdump's disassembly of FFX.exe and hashed:

  exact   the bytes are equal once every rel32/rel8 branch that leaves the
          function is replaced by its resolved target (through one-jump
          thunks); jumps inside the function stay relative. A copy runs
          the same code on the same globals: the port of one member is
          the port of all of them.
  masked  equal once the operands the relocation table (.reloc) patches are
          masked as well: the same code on other globals (or other
          function pointers). The port needs those addresses as inputs.
  near    instruction sequences with every number in the operands masked
          and the general registers renamed by first use (addressing kept;
          catches copies the compiler gave other registers) are >= --near (default 95 %)
          similar (difflib ratio), found through min-hash candidates. A
          near copy is a lead, not a port: read the difference.

Each family lists its members with name, module, size, class
(symbols/classes.tsv) and port status (the `ported!` tables, as
tools/check-ported.py reads them). For a family with a ported member it
can print the `ported!` entries for the other members, copied from the
ported member's entry, and the oracle test that covers the port with the
copy's address (--shims): text for a porting round to paste, after it has
checked the copy against its own original. Nothing is hooked by this tool.

  FFX_GAME_DIR=... tools/dupes.py [--out DIR] [--shims FILE] [--near 0.95]
                                  [--min-bytes 5] [--min-insns 12]
  tools/dupes.py --family 0x0074bff0      # the families of one address

Writes DIR/families.tsv (default $FFX_WORK/speedup/dupes) and prints a
summary: families, and the unported members of families with a ported
member (the immediate yield).
"""
import argparse
import bisect
import difflib
import hashlib
import os
import pathlib
import re
import struct
import subprocess
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

BASE = 0x400000
INSN = re.compile(r"^\s*([0-9a-f]+):\t([0-9a-f ]+?)\s*\t(\S+)\s*(.*)$")
TARGET = re.compile(r"^0x([0-9a-f]+)$")
NUMBER = re.compile(r"0x[0-9a-f]+|\b\d+\b")
REG = re.compile(r"\b(e?(ax|bx|cx|dx|si|di)|[abcd][lh])\b")
FAMILY = {"a": "ax", "b": "bx", "c": "cx", "d": "dx"}


def renamer():
    """Renames general registers by first use (esp/ebp kept), so that a copy
    the compiler gave other registers reads the same."""
    seen = {}

    def one(m):
        r = m[1]
        if len(r) == 2 and r[1] in "lh":  # al, ah, ...: the low/high byte of eax, ...
            fam, part = FAMILY[r[0]], "." + r[1]
        else:
            fam, part = r[-2:], "" if r.startswith("e") else ".w"
        return seen.setdefault(fam, f"g{len(seen)}") + part
    return lambda text: REG.sub(one, text)


REL32 = {0xE8, 0xE9}
REL8 = {0xEB, 0xE3, 0xE0, 0xE1, 0xE2} | set(range(0x70, 0x80))


def read_pe(path):
    """(sections [(va, blob)], addresses the relocation table patches)."""
    data = pathlib.Path(path).read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    nsec = struct.unpack_from("<H", data, pe + 6)[0]
    opt = struct.unpack_from("<H", data, pe + 20)[0]
    secs, reloc = [], None
    for i in range(nsec):
        o = pe + 24 + opt + 40 * i
        name = data[o:o + 8].rstrip(b"\0").decode("latin-1")
        vsize, va, rsize, raw = struct.unpack_from("<IIII", data, o + 8)
        blob = data[raw:raw + min(rsize, vsize or rsize)]
        if name == ".reloc":
            reloc = blob
        secs.append((va + BASE, blob))
    fixups = set()
    k = 0
    while reloc and k + 8 <= len(reloc):
        page, size = struct.unpack_from("<II", reloc, k)
        if size < 8:
            break
        for (e,) in struct.iter_unpack("<H", reloc[k + 8:k + size]):
            if e >> 12 == 3:  # IMAGE_REL_BASED_HIGHLOW
                fixups.add(BASE + page + (e & 0xFFF))
        k += size
    return secs, fixups


def objdump(exe, *extra):
    out = subprocess.run(["objdump", "-d", "-M", "intel", "--insn-width=16", "-j", ".text", *extra, str(exe)],
                         capture_output=True, text=True, check=True).stdout
    insns = []
    for line in out.splitlines():
        m = INSN.match(line)
        if m:
            ops = m[4].split("<", 1)[0].strip()
            insns.append((int(m[1], 16), bytes.fromhex(m[2]), m[3], ops))
    return insns


def disassemble(exe, starts):
    """[(address, bytes, mnemonic, operands)] for .text, in address order.
    objdump's linear sweep can run through a function start out of step
    (after a jump table); those functions are disassembled again from
    their own start."""
    insns = objdump(exe)
    have = {i[0] for i in insns}
    lo, hi = insns[0][0], insns[-1][0]
    fixed = []
    for k, a in enumerate(starts):
        if lo <= a <= hi and a not in have:
            nxt = starts[k + 1] if k + 1 < len(starts) else a + 0x1000
            fixed.append((a, nxt, objdump(exe, f"--start-address={a:#x}", f"--stop-address={nxt:#x}")))
    if fixed:
        spans = sorted((a, nxt) for a, nxt, _ in fixed)
        keys = [a for a, _ in spans]

        def outside(at):
            i = bisect.bisect_right(keys, at) - 1
            return i < 0 or at >= spans[i][1]
        insns = [x for x in insns if outside(x[0])] + [x for _, _, got in fixed for x in got]
        insns.sort(key=lambda x: x[0])
    return insns


def extent(insns, i, size, nxt, starts):
    """Index past a function's last instruction: Ghidra's body size, grown
    while a branch inside it targets code after it (some body sizes leave
    out tails), each tail run up to its ret/jmp; never past the next
    function start or into int3 padding."""
    body = end = insns[i][0] + size
    j, terminated = i, True
    while j < len(insns):
        at, raw, mn, ops = insns[j]
        if at >= nxt or mn == "int3" or (at >= end and (terminated or end == body)):
            break
        j += 1
        terminated = mn.startswith(("ret", "jmp"))
        t = TARGET.match(ops) if mn.startswith(("j", "loop")) else None
        if t and end <= int(t[1], 16) < nxt and int(t[1], 16) not in starts:
            end = int(t[1], 16) + 1
    return j


def normalise(insns, lo, starts, fixups, thunk):
    """(exact key, masked key, operand-masked token list) of a function's
    instructions: branches to other functions by target, the rest relative."""
    exact, masked, tokens = [], [], []
    rename = renamer()
    for at, raw, mn, ops in insns:
        b = bytearray(raw)
        mb = bytearray(raw)
        op = b[1] if b[0] == 0x0F and len(b) > 1 else None
        rel = 0
        if b[0] in REL32 or (b[0] == 0x0F and op is not None and 0x80 <= op <= 0x8F):
            rel = 4
        elif b[0] in REL8:
            rel = 1
        t = TARGET.match(ops)
        if rel and t and len(b) > rel:
            target = int(t[1], 16)
            if target not in starts and target not in thunk:
                ident = b"r" + (target - lo).to_bytes(4, "little", signed=True)
            else:
                ident = b"x" + thunk.get(target, target).to_bytes(4, "little")
            b = b[:-rel] + ident
            mb = mb[:-rel] + ident
        else:
            for p in range(at, at + len(raw)):
                if p in fixups and p + 4 <= at + len(raw):
                    k = p - at
                    mb[k:k + 4] = b"\xff\xff\xff\xff"
        exact.append(bytes(b))
        masked.append(bytes(mb))
        tokens.append(mn + " " + rename(NUMBER.sub("#", ops)))
    key = hashlib.blake2b(b"|".join(exact), digest_size=16).digest()
    mkey = hashlib.blake2b(b"|".join(masked), digest_size=16).digest()
    return key, mkey, tokens


def minhash_pairs(seqs, ratio, bands=16, rows=4):
    """Candidate pairs (i, j) of token sequences for a >= ratio match:
    min-hashes over 3-gram shingles, banded; then the length bound."""
    import numpy as np

    n = bands * rows
    rng = np.random.default_rng(0x5EED)
    a = rng.integers(1, 2**61 - 1, size=n, dtype=np.uint64) | np.uint64(1)
    b = rng.integers(0, 2**61 - 1, size=n, dtype=np.uint64)
    buckets = defaultdict(list)
    for i, s in enumerate(seqs):
        sh = {hash((s[k], s[k + 1], s[k + 2])) & 0xFFFFFFFFFFFF for k in range(len(s) - 2)}
        h = np.fromiter(sh, dtype=np.uint64)
        sig = (h[:, None] * a[None, :] + b[None, :]).min(axis=0)
        for band in range(bands):
            buckets[(band, sig[band * rows:(band + 1) * rows].tobytes())].append(i)
    pairs = set()
    for members in buckets.values():
        if len(members) > 1:
            for x in range(len(members)):
                for y in range(x + 1, len(members)):
                    i, j = members[x], members[y]
                    li, lj = len(seqs[i]), len(seqs[j])
                    if 2 * min(li, lj) / (li + lj) >= ratio:
                        pairs.add((i, j))
    return pairs


class Union:
    def __init__(self):
        self.parent = {}

    def find(self, x):
        self.parent.setdefault(x, x)
        while self.parent[x] != x:
            self.parent[x] = self.parent[self.parent[x]]
            x = self.parent[x]
        return x

    def join(self, x, y):
        self.parent[self.find(x)] = self.find(y)


ENTRY = re.compile(r"^(\s*)((?:///[^\n]*\n\s*)*)(\w+) @ 0x([0-9a-f_]+) =>\s*extern \"([\w-]+)\" fn (\w+)", re.M)


def ported_entries():
    """address -> (file, entry text) for every `ported!` entry."""
    out = {}
    for f in sorted((progress.REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
        text = f.read_text(encoding="utf-8")
        for m in ENTRY.finditer(text):
            k = text.index("{", m.end())
            depth, e = 0, k
            while e < len(text):
                depth += {"{": 1, "}": -1}.get(text[e], 0)
                e += 1
                if depth == 0:
                    break
            out[int(m[4].replace("_", ""), 16)] = (f, m[1] + text[m.start(3):e], m[3], m[6])
    return out


def underscore(a):
    s = f"{a:08x}"
    return f"0x{s[:4]}_{s[4:]}"


def oracle_uses(a):
    """(file, line number, line) of oracle test lines naming address a."""
    pats = (underscore(a), f"0x{a:08x}", f"0x{a:x}")
    out = []
    for f in sorted((progress.REPO / "crates/ffx-oracle/tests").glob("*.rs")):
        for i, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
            if any(re.search(re.escape(p) + r"\b", line) for p in pats):
                out.append((f, i, line.strip()))
    return out


def shim_text(fam, info, entries, want):
    """The `ported!` entries and test hints for a family's unported members
    that `want` accepts."""
    ported = [a for a in fam if a in entries]
    if not ported:
        return ""
    src = ported[0]
    f, entry, name, fn = entries[src]
    lines = [f"// family of {name} @ {src:#010x} ({f.relative_to(progress.REPO)})"]
    uses = oracle_uses(src)
    for a in fam:
        if a in entries or not want(a):
            continue
        new = info[a]["name"]
        body = entry.replace(f"{name} @ {underscore(src)}", f"{new} @ {underscore(a)}", 1)
        body = re.sub(rf"\bfn {fn}\b", f"fn {fn}_{a:08x}", body, count=1)
        indent = re.match(r"\s*", body)[0]
        lines.append(f"// paste into {f.name}" + (" (name it in symbols/ffx.sym first):" if new.startswith("FUN_") else ":"))
        lines.append(f"{indent}/// `{new}`: the same code as `{name}` (tools/dupes.py).")
        lines.append(body)
        if uses:
            tf, ln, text = uses[0]
            lines.append(f"// test: {tf.relative_to(progress.REPO)}:{ln} tests {name} as `{text}`;")
            lines.append(f"//   run that test against {underscore(a)} too (add it to the address list,")
            lines.append("//   or copy the test with the address swapped): each copy is checked")
            lines.append("//   against its own original.")
        else:
            lines.append(f"// test: no oracle test names {underscore(src)}; write one for both addresses.")
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("--exe", default=None)
    ap.add_argument("--out", default=str(progress.WORK / "speedup/dupes"))
    ap.add_argument("--shims", help="write ported! entries and test hints for copies of ports here")
    ap.add_argument("--shims-all", action="store_true", help="for every class, not only logic copies")
    ap.add_argument("--near", type=float, default=0.95)
    ap.add_argument("--no-near", action="store_true")
    ap.add_argument("--min-bytes", type=int, default=5, help="smallest function (the jump patch needs 5)")
    ap.add_argument("--min-insns", type=int, default=12, help="smallest function for near copies")
    ap.add_argument("--family", help="print only the families of this address")
    args = ap.parse_args()

    exe = pathlib.Path(args.exe) if args.exe else (
        pathlib.Path(os.environ.get("FFX_GAME_DIR", "~/steam-games/FFX-X2-HD")).expanduser() / "FFX.exe")
    if not exe.exists():
        sys.exit(f"{exe} missing: set FFX_GAME_DIR or --exe")
    _, fixups = read_pe(exe)
    rows = progress.functions()
    insns = disassemble(exe, sorted(a for a, _, _ in rows))
    addrs = [i[0] for i in insns]
    names = progress.symbols()
    module = progress.placement(rows, names)
    category = progress.classes()
    ported = progress.ported()
    starts = sorted(a for a, _, _ in rows)
    start_set = set(starts)

    # one-jump thunks: a function that is only `jmp rel32` stands for its target
    thunk = {}
    for a, _, size in rows:
        i = bisect.bisect_left(addrs, a)
        if size == 5 and i < len(insns) and insns[i][0] == a and insns[i][1][0] == 0xE9:
            t = TARGET.match(insns[i][3])
            if t:
                thunk[a] = int(t[1], 16)

    info = {}
    by_exact, by_masked = defaultdict(list), defaultdict(list)
    for a, n, size in rows:
        if size < args.min_bytes:
            continue
        k = bisect.bisect_right(starts, a)
        nxt = starts[k] if k < len(starts) else a + size
        i = bisect.bisect_left(addrs, a)
        if i >= len(insns) or insns[i][0] != a:
            continue  # not in .text, or not on an instruction boundary
        j = extent(insns, i, size, nxt, start_set)
        key, mkey, tokens = normalise(insns[i:j], a, start_set, fixups, thunk)
        info[a] = {"name": names.get(a) or n, "size": size, "module": module.get(a) or "?",
                   "class": category.get(a, "?"), "ported": a in ported, "tokens": tokens}
        by_exact[key].append(a)
        by_masked[mkey].append(a)

    fams = []  # (kind, members, similarity by member)
    exact_of = {}
    for members in by_exact.values():
        if len(members) > 1:
            fams.append(("exact", sorted(members), {}))
        for m in members:
            exact_of[m] = members[0]
    masked_of = {}
    for members in by_masked.values():
        reps = sorted({exact_of[m] for m in members})
        if len(reps) > 1:
            fams.append(("masked", sorted(members), {}))
        for m in members:
            masked_of[m] = min(members)

    if not args.no_near:
        reps = sorted({masked_of[a] for a in info if len(info[a]["tokens"]) >= args.min_insns})
        seqs = [info[a]["tokens"] for a in reps]
        uf = Union()
        sims = {}
        for i, j in minhash_pairs(seqs, args.near):
            sm = difflib.SequenceMatcher(None, seqs[i], seqs[j], autojunk=False)
            if sm.real_quick_ratio() >= args.near and sm.quick_ratio() >= args.near:
                r = sm.ratio()
                if r >= args.near:
                    uf.join(reps[i], reps[j])
                    sims[(reps[i], reps[j])] = r
        groups = defaultdict(set)
        for x, y in sims:
            groups[uf.find(x)].update((x, y))
        members_of = defaultdict(list)
        for a in info:
            members_of[masked_of[a]].append(a)
        for g in groups.values():
            mem = sorted(m for r in g for m in members_of[r])
            anchor = next((m for m in mem if info[m]["ported"]), mem[0])
            sim = {}
            for m in mem:
                if masked_of[m] == masked_of[anchor]:
                    sim[m] = 1.0
                else:
                    sim[m] = difflib.SequenceMatcher(None, info[anchor]["tokens"], info[m]["tokens"],
                                                     autojunk=False).ratio()
            fams.append(("near", mem, sim))

    if args.family:
        want = int(args.family, 16)
        for kind, mem, sim in fams:
            if want in mem:
                print(f"{kind} family of {len(mem)}:")
                for m in mem:
                    x = info[m]
                    s = f" {sim[m]:.3f}" if m in sim else ""
                    print(f"  {m:#010x} {x['name']:<40} {x['module']:<8} {x['size']:>6} {x['class']:<8}"
                          f" {'ported' if x['ported'] else '-'}{s}")
        return

    out = pathlib.Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    with open(out / "families.tsv", "w", encoding="utf-8") as f:
        f.write("# family\tkind\taddress\tname\tmodule\tsize\tclass\tported\tsimilarity\n")
        for n, (kind, mem, sim) in enumerate(fams):
            for m in mem:
                x = info[m]
                f.write(f"{n}\t{kind}\t{m:#010x}\t{x['name']}\t{x['module']}\t{x['size']}\t{x['class']}\t"
                        f"{'ported' if x['ported'] else '-'}\t{sim.get(m, 1.0):.3f}\n")

    for kind in ("exact", "masked", "near"):
        fs = [mem for k, mem, _ in fams if k == kind]
        print(f"{kind:<6} {len(fs):>5} families, {sum(len(m) for m in fs):>6} functions")
    for kind in ("exact", "masked", "near"):
        yield_ = set()
        for k, mem, sim in fams:
            if k == kind and any(info[m]["ported"] for m in mem):
                yield_ |= {m for m in mem if not info[m]["ported"]}
        logic = [m for m in yield_ if info[m]["class"] == "logic"]
        kb = sum(info[m]["size"] for m in yield_) / 1024
        lkb = sum(info[m]["size"] for m in logic) / 1024
        print(f"unported {kind} copies of ported functions: {len(yield_)} ({kb:.1f} KB), "
              f"logic {len(logic)} ({lkb:.1f} KB)")
    spare = [mem for k, mem, _ in fams if k == "exact" and not any(info[m]["ported"] for m in mem)]
    spare = [[m for m in mem if info[m]["class"] == "logic"] for mem in spare]
    spare = [mem for mem in spare if len(mem) > 1]
    extra = [m for mem in spare for m in mem[1:]]
    print(f"unported logic in exact families without a port: {len(spare)} families; porting one member "
          f"covers {len(extra)} more ({sum(info[m]['size'] for m in extra) / 1024:.1f} KB)")
    print(f"wrote {out / 'families.tsv'}")

    if args.shims:
        entries = ported_entries()
        with open(args.shims, "w", encoding="utf-8") as f:
            f.write("// tools/dupes.py: ported! entries for exact copies of ported functions.\n"
                    "// Check each against its own original (oracle test) before hooking it.\n\n")
            def want(a):
                return args.shims_all or info[a]["class"] == "logic"
            for kind, mem, _ in fams:
                if kind == "exact" and any(m in entries for m in mem) and any(
                        not info[m]["ported"] and want(m) for m in mem):
                    f.write(shim_text(mem, info, entries, want) + "\n")
        print(f"wrote {args.shims}")


if __name__ == "__main__":
    main()
