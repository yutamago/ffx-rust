#!/usr/bin/env python3
"""overlay-pfamilies.py: open overlay functions that are the same code up
to parameters (docs/formats/magic.md, "Parameter families").

  FFX_GAME_DIR=... tools/overlay-pfamilies.py [--worklist FILE] [--census DIR] [--out FILE]

The work list (tools/magic-worklist.py) has one row per code identity:
its hash keeps every operand that the link does not move, so two
functions that differ only in a displacement, an immediate, a global or
which (equal) callee they call are two rows, and so far two ports. This
tool groups the open rows by their code with those parameters masked, so
one port that reads its parameters from each member's code (or from a
generated per-member table) can bind them all.

Rows: every row of --worklist that no port covers now (`magic-sig
--covered` at its first DLL's address, built from this checkout: cargo
build -p ffx-magic --examples). Functions: the census's recursive cut
(tools/overlay-census.py `cut_dll`: instructions reached from the start,
plus the jump and byte tables after the code), cached per DLL in
$FFX_WORK/cache/overlay-pfamilies.

Each instruction is normalised to its length and its objdump text with:
- relocated words: an import (kept by name), an address inside the
  function (kept as its offset), a code pointer (param `fnptr`, kept as
  the target's shape), a data address (param `abs`);
- `rel32`/`rel8` branches inside the function: the offset from the start
  (kept); calls and tail jumps out of it: the callee's shape (param
  `call`; a slot thunk is `slot N`, kept);
- `[reg+disp]` where reg holds the `g_MagicImports` pointer: `slot N`
  (kept: a different slot is a different game function);
- every other number: param `disp` (inside brackets, signed) or `imm`.
A callee's shape is its own normalised stream's hash (recursively; a
cycle hashes as `cycle`), so callees that are themselves the same code
up to parameters match. Bytes of the function that are not instructions
(the tables): `exact-params` keeps them (table words as offsets into the
function), `loose` keeps only their length and also masks which
function a call, tail jump or code pointer reaches (slots and imports
are still kept), as `r9_load`'s load step, whose copies differ in the
DLL's start-up they call.

Output (--out, default $FFX_WORK/m5/magic/r4/pfamilies.tsv): one row
per family of 2 or more rows, ranked by members x bytes: id, level,
members, bytes (the first member's size), members x bytes, how many
members have every callee covered, taken (members already in
families.tsv as taken/done or in done.tsv), the differing parameters
(`insn index:offset in the function:kind=v1/v2/...` with values per
member in member order; `call`/`fnptr` values are the callee's port name
or `?addr` when uncovered), and the members (`dll@addr:order`). `loose`
families are listed only where they join more than one exact family.
"""
import argparse
import bisect
import collections
import concurrent.futures
import hashlib
import importlib.util
import os
import pathlib
import pickle
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE))
import reach  # noqa: E402

_spec = importlib.util.spec_from_file_location("overlay_census", HERE / "overlay-census.py")
census = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(census)

WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))
CACHE = WORK / "cache" / "overlay-pfamilies"
VERSION = b"overlay-pfamilies 1"  # the cut cache
GAME = pathlib.Path(os.environ.get("FFX_GAME_DIR", pathlib.Path.home() / "steam-games/FFX-X2-HD"))

HEX = re.compile(r"([+-]?)0x([0-9a-f]+)")
MEMREF = re.compile(r"\[([a-z]{3})([+-]0x[0-9a-f]+)?\]")
LOADREG = re.compile(r"^mov\s+(e[a-ds][xip]),(?:DWORD PTR )?ds:0x([0-9a-f]+)$")


def field_at(ib, v, end):
    """Offset in the instruction bytes `ib` of the field holding `v`, the
    rightmost before `end` (an immediate ends the instruction, a
    displacement comes before it): 4, 1 or 2 bytes, little-endian."""
    best = None
    for w in (4, 1, 2):
        lo, hi = -(1 << (8 * w - 1)), 1 << (8 * w)
        if not lo <= v < hi:
            continue
        enc = (v & ((1 << (8 * w)) - 1)).to_bytes(w, "little")
        if end >= w + 1 and ib[end - w:end] == enc:
            return end - w
        k = ib.rfind(enc, 1, end)
        if k >= 1 and best is None:
            best = k
    return best


def dll_path(d):
    return GAME / f"magicFiles/FFX/magic_{d:04}.dll"


def cut(d):
    """{start: (end, [(pc, length, text)])}, the g_MagicImports global, and
    the import table; cached per DLL file."""
    path = dll_path(d)
    data = path.read_bytes()
    key = hashlib.sha256(VERSION + data + (HERE / "overlay-census.py").read_bytes()).hexdigest()[:20]
    cached = CACHE / f"{key}.pickle"
    if cached.exists():
        with open(cached, "rb") as f:
            return pickle.load(f)
    img = reach.Image(path)
    code = census.Code(img, path)
    fns, _ = census.cut_dll(img, code)
    fns = {s: (e, ins) for s, (e, ins) in fns.items()}
    table = census.table_global([t for _, ins in fns.values() for _, _, t in ins])
    res = (fns, int(table, 16) if table else None)
    CACHE.mkdir(parents=True, exist_ok=True)
    tmp = cached.with_suffix(f".{os.getpid()}.tmp")
    with open(tmp, "wb") as f:
        pickle.dump(res, f)
    os.replace(tmp, cached)
    return res


def covered_names(sig, d, addrs):
    out = {}
    addrs = sorted(addrs)
    for k in range(0, len(addrs), 400):
        r = subprocess.run([str(sig), "--covered", str(dll_path(d)), *[f"{x:#x}" for x in addrs[k:k + 400]]],
                           capture_output=True, text=True, check=True)
        for line in r.stdout.splitlines():
            x, _, name = line.partition(" ")
            name = name.strip()
            if name != "-":
                out[int(x, 16)] = name
    return out


class Dll:
    def __init__(self, d, sig):
        self.d = d
        self.fns, self.table = cut(d)
        self.img = reach.Image(dll_path(d))
        self.iat = self.img.imports()
        self.text = next(blob for _, va, blob, c in self.img.secs if c)
        self.starts = sorted(self.fns)
        self.cov = covered_names(sig, d, self.starts)
        self.memo = {}

    def fn_at(self, a):
        return a if a in self.fns else None

    def slot_of_thunk(self, a):
        """The slot of a slot thunk (`mov eax, ds:G; jmp [eax+4N]`), else None."""
        if a not in self.fns:
            return None
        ins = self.fns[a][1]
        if len(ins) > 4:
            return None
        held = set()
        for _, _, t in ins:
            m = LOADREG.match(t)
            if m and self.table is not None and int(m.group(2), 16) == self.table:
                held.add(m.group(1))
                continue
            if t.startswith("jmp"):
                m = MEMREF.search(t)
                if m and m.group(1) in held:
                    return int(m.group(2) or "0", 16) // 4
        return None

    def callee(self, t, stack):
        """(token, port name or ?addr) of a call/jump/pointer target."""
        slot = self.slot_of_thunk(t)
        if slot is not None:
            return f"slot{slot}", None
        name = self.cov.get(t, f"?{t:#x}")
        if t not in self.fns:
            return f"code?{t:#x}", name
        if t in stack:
            return "cycle", name
        return "fn:" + self.shape(t, stack)[0], name

    def shape(self, s, stack=()):
        """(exact hash, loose hash, [(index, offset, kind, value)], bytes)."""
        if s in self.memo:
            return self.memo[s]
        stack = stack + (s,)
        e, ins = self.fns[s]
        img = self.img
        toks, ltoks, params = [], [], []
        held = set()
        rel32 = []  # (field offset, callee port name) of the rel32 fields leaving the function
        prev_end = s
        for idx, (pc, n, t) in enumerate(ins):
            if pc > prev_end:  # bytes between instructions (tables)
                toks.append(self.data_tok(s, prev_end, pc))
                ltoks.append(f"data{pc - prev_end}")
            prev_end = max(prev_end, pc + n)
            norm = t
            lsubs = []  # (exact token, loose token)
            # relocated words
            i = bisect.bisect_left(img.relocs, pc)
            relocated = {}
            while i < len(img.relocs) and img.relocs[i] < pc + n:
                r = img.relocs[i]
                i += 1
                v = img.dword(r)
                relocated[v] = r
            for v, r in relocated.items():
                pat = f"0x{v:x}"
                if v in self.iat:
                    rep = "imp:" + self.iat[v].split("!")[-1]
                elif v == self.table:
                    rep = "TABLE"
                elif s <= v < e:
                    rep = f"L{v - s:x}"
                elif img.in_text(v):
                    tok, name = self.callee(v, stack)
                    rep = "P:" + tok
                    lrep = "P:fn" if tok.startswith(("fn:", "cycle")) else rep
                    params.append((idx, r - s, "fnptr", name or tok))
                else:
                    rep = "ABS"
                    params.append((idx, r - s, "abs", f"{v:#x}"))
                if rep.startswith("P:"):
                    lsubs.append((rep, lrep))
                norm = re.sub(rf"{pat}\b", rep, norm)
            m = reach.DIRECT.match(t)
            if m:
                tgt = int(m.group(2), 16)
                if s <= tgt < e and m.group(1) != "call":
                    norm = f"{m.group(1)} L{tgt - s:x}"
                else:
                    tok, name = self.callee(tgt, stack)
                    norm = f"{m.group(1)} {tok}"
                    if tok.startswith(("fn:", "cycle")):
                        lsubs.append((tok, "fn"))
                    if not tok.startswith("slot"):
                        params.append((idx, pc + n - 4 - s, "call", name or tok))
                    if n >= 5:
                        rel32.append((pc + n - 4 - s, "slot thunk" if tok.startswith("slot") else name or tok))
            else:
                # slots through the held table pointer
                def slotrep(mm):
                    if mm.group(1) in held:
                        return f"[SLOT{int(mm.group(2) or '0', 16) // 4}]"
                    return mm.group(0)
                norm = MEMREF.sub(slotrep, norm)
                # every other number is a parameter
                out, last, nums = [], 0, []
                for mm in HEX.finditer(norm):
                    out.append(norm[last:mm.start()])
                    last = mm.end()
                    v = int(mm.group(2), 16)
                    if norm.rfind("[", 0, mm.start()) > norm.rfind("]", 0, mm.start()):
                        v = -v if mm.group(1) == "-" else v
                        out.append("+#")
                        nums.append(("disp", v))
                    else:
                        out.append(mm.group(1) + "#")
                        nums.append(("imm", v))
                out.append(norm[last:])
                norm = "".join(out)
                ib = self.text[pc - img.text_lo:pc - img.text_lo + n]
                offs, end = [], n
                for kind, v in reversed(nums):
                    at = field_at(ib, v, end)
                    offs.append(at)
                    if at is not None:
                        end = at
                for (kind, v), at in zip(nums, reversed(offs)):
                    params.append((idx, None if at is None else pc - s + at, kind,
                                   f"{v:#x}" if v >= 0 else f"-{-v:#x}"))
            # track registers holding the g_MagicImports pointer
            lm = LOADREG.match(t)
            parts = t.split(None, 1)
            if lm and self.table is not None and int(lm.group(2), 16) == self.table:
                held.add(lm.group(1))
            elif parts and parts[0] in ("call", "ret", "jmp"):
                held = set()
            elif len(parts) > 1:
                dest = parts[1].split(",")[0]
                if dest in held and parts[0] not in ("push", "cmp", "test"):
                    held.discard(dest)
            tok = f"{n}|{norm}"
            toks.append(tok)
            for a, b in lsubs:
                tok = tok.replace(a, b)
            ltoks.append(tok)
        if e > prev_end:
            toks.append(self.data_tok(s, prev_end, e))
            ltoks.append(f"data{e - prev_end}")
        res = (hashlib.sha1("\n".join(toks).encode()).hexdigest()[:16],
               hashlib.sha1("\n".join(ltoks).encode()).hexdigest()[:16],
               params, e - s, len(ins), rel32)
        self.memo[s] = res
        return res

    def data_tok(self, s, lo, hi):
        img = self.img
        out = []
        k = lo
        while k < hi:
            if k in img.reloc_set and k + 4 <= hi:
                v = img.dword(k)
                out.append(f"T{v - s:x}" if s <= v < hi else f"T?{v:x}")
                k += 4
            else:
                out.append(f"{self.text[k - img.text_lo]:02x}")
                k += 1
        return "data:" + " ".join(out)


def scan(args):
    d, addrs, sig = args
    dll = Dll(d, sig)
    out = {}
    for a in addrs:
        if a not in dll.fns:
            out[a] = None
            continue
        ex, lo, params, size, n, rel32 = dll.shape(a)
        callees = [p for p in params if p[2] in ("call", "fnptr")]
        out[a] = (ex, lo, params, size, n, dll.cov.get(a), all(not p[3].startswith("?") for p in callees), rel32)
    return d, out


def read_worklist(path):
    rows = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.startswith("#"):
                continue
            c = line.rstrip("\n").split("\t")
            rows.append({"order": int(c[0]), "id": c[1], "kind": c[4], "size": int(c[5]),
                         "dll": int(c[7][6:10]), "addr": int(c[8], 16), "ndlls": int(c[6])})
    return rows


def read_taken(families, done):
    """dll@addr (lower hex, 4-digit dll, address - 0x10000000) and identities
    already taken."""
    keys = set()
    if families.exists():
        for line in families.read_text(encoding="utf-8").splitlines():
            if line.startswith("#"):
                continue
            c = line.split("\t")
            if len(c) < 10 or c[1] not in ("taken", "done"):
                continue
            for m in c[9].split():
                p = m.split(":")
                if len(p) >= 4:
                    dll, _, addr = p[2].partition("@")
                    keys.add(f"{int(dll):04}@{int(addr, 16) & 0xffffff:x}")
                    keys.add(p[3])
    if done.exists():
        for line in done.read_text(encoding="utf-8").splitlines():
            if line.startswith("#") or not line.strip():
                continue
            k = line.split("\t")[0].strip()
            if "@" in k:
                dll, _, addr = k.partition("@")
                try:
                    keys.add(f"{int(dll):04}@{int(addr, 16) & 0xffffff:x}")
                except ValueError:
                    pass
            else:
                keys.add(k)
    return keys


def print_sigs(args, fams):
    """The Sig of each member of one family, for a family port's table."""
    for n, (level, k, v) in enumerate(fams, 1):
        fid = f"P{n:03}-{k[:8]}"
        if args.sigs not in (fid, fid[:4]):
            continue
        print(f"// family {fid} ({level}, {len(v)} members; tools/overlay-pfamilies.py --sigs {fid[:4]})")
        for r in v:
            d, a = r["dll"], r["addr"]
            size, rel32 = r["shape"][3], r["shape"][7]
            out = subprocess.run([str(args.sig), str(dll_path(d)), f"{a:#x}", str(size),
                                  *[f"{o:#x}" for o, _ in rel32]], capture_output=True, text=True, check=True)
            print(f"// magic_{d:04} {a:#x} (row {r['order']}): len {size}, hash {out.stdout.strip()}")
            for o, name in rel32:
                print(f"//     ({o:#x}, {name})")
        return 0
    sys.exit(f"no family {args.sigs}")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--worklist", type=pathlib.Path, default=WORK / "m5/magic/worklist5.tsv")
    ap.add_argument("--families", type=pathlib.Path, default=WORK / "m5/magic/r4/families.tsv")
    ap.add_argument("--done", type=pathlib.Path, default=WORK / "m5/magic/r4/done.tsv")
    ap.add_argument("--out", type=pathlib.Path, default=WORK / "m5/magic/r4/pfamilies.tsv")
    ap.add_argument("--sig", type=pathlib.Path, default=REPO / "target/debug/examples/magic-sig")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--top", type=int, default=30, help="families printed")
    ap.add_argument("--sigs", metavar="FAMILY", help="print each member's Sig (magic-sig's length and hash, "
                    "the rel32 callees by port name) for a family id of the output (P003 or P003-601ac5bc)")
    args = ap.parse_args()
    if not args.sig.exists():
        sys.exit(f"{args.sig} missing: cargo build -p ffx-magic --examples")
    rows = read_worklist(args.worklist)
    by_dll = collections.defaultdict(set)
    for r in rows:
        by_dll[r["dll"]].add(r["addr"])
    res = {}
    with concurrent.futures.ProcessPoolExecutor(args.jobs) as ex:
        for d, out in ex.map(scan, [(d, sorted(a), args.sig) for d, a in sorted(by_dll.items())]):
            for a, v in out.items():
                res[(d, a)] = v
    taken = read_taken(args.families, args.done)
    open_rows = []
    nocut = 0
    for r in rows:
        v = res.get((r["dll"], r["addr"]))
        if v is None:
            nocut += 1
            continue
        if v[5] is not None:  # covered now
            continue
        r["shape"] = v
        r["key"] = f"{r['dll']:04}@{r['addr'] & 0xffffff:x}"
        r["taken"] = r["key"] in taken or r["id"] in taken
        open_rows.append(r)

    def group(level):
        g = collections.defaultdict(list)
        for r in open_rows:
            g[r["shape"][level]].append(r)
        return {k: v for k, v in g.items() if len(v) >= 2}

    exact = group(0)
    loose = group(1)
    exact_sets = {frozenset(id(r) for r in v) for v in exact.values()}
    fams = [("exact-params", k, v) for k, v in exact.items()]
    for k, v in loose.items():
        if frozenset(id(r) for r in v) in exact_sets:
            continue
        joined = {r["shape"][0] for r in v}
        if len(joined) > 1:
            fams.append(("loose", k, v))
    fams.sort(key=lambda f: (-len(f[2]) * f[2][0]["shape"][3], f[0], f[2][0]["order"]))

    def diffs(members, level):
        out = []
        ps = [m["shape"][2] for m in members]
        if len({len(p) for p in ps}) != 1:
            return "-"
        for j in range(len(ps[0])):
            vals = [p[j][3] for p in ps]
            if len(set(vals)) > 1:
                idx, off, kind, _ = ps[0][j]
                where = f"{idx}" + (f"@+{off:#x}" if off is not None else "")
                if any(p[j][1] != off for p in ps):
                    where += "(offsets differ)"
                out.append(f"{where}:{kind}=" + "/".join(vals))
        return "; ".join(out) or "none"

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as f:
        f.write("# overlay-pfamilies.py: open work-list rows that are the same code up to parameters "
                f"(from {args.worklist.name}; docs/formats/magic.md, \"Parameter families\")\n")
        f.write("# family\tlevel\tmembers\tbytes\tmembers x bytes\tinsns\tcallees covered\ttaken\t"
                "differing params (insn[@+offset]:kind=values per member)\tmembers (dll@addr:order)\n")
        for n, (level, k, v) in enumerate(fams, 1):
            v.sort(key=lambda r: r["order"])
            fid = f"P{n:03}-{k[:8]}"
            sh = v[0]["shape"]
            cov = sum(1 for r in v if r["shape"][6])
            tk = sum(1 for r in v if r["taken"])
            f.write("\t".join(str(x) for x in [
                fid, level, len(v), sh[3], len(v) * sh[3], sh[4], f"{cov}/{len(v)}", tk, diffs(v, level),
                " ".join(f"{r['key']}:{r['order']}" for r in v)]) + "\n")
            v[0]["fid"] = fid
    in_exact = sum(len(v) for v in exact.values())
    if args.sigs:
        return print_sigs(args, fams)

    def at_least(k, groups):
        return sum(len(v) for v in groups.values() if len(v) >= k)
    print(f"{len(rows)} work-list rows; {len(open_rows)} open ({nocut} not a census function start)")
    for lvl, gs in (("exact-params", exact), ("loose", loose)):
        print(f"{lvl}: {len(gs)} families; rows in families of 2+ {at_least(2, gs)}, 5+ {at_least(5, gs)}, "
              f"10+ {at_least(10, gs)}")
    print(f"exact-params rows in families: {in_exact} ({sum(len(v) * v[0]['shape'][3] for v in exact.values())} bytes)")
    print(f"wrote {args.out}")
    for level, k, v in fams[:args.top]:
        sh = v[0]["shape"]
        print(f"  {level:12} {len(v):3} x {sh[3]:5} = {len(v) * sh[3]:6}  callees covered "
              f"{sum(1 for r in v if r['shape'][6])}/{len(v)}  first {v[0]['key']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
