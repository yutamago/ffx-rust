#!/usr/bin/env python3
"""magic-worklist.py: the work list for porting the spell overlays
(docs/formats/magic.md, "Porting the overlays").

  FFX_GAME_DIR=... tools/magic-worklist.py [--census DIR] [--lists FILE] [--out FILE] [--replace]

Units are distinct functions by code identity, the identity `ffx_magic`
binds by (`magic-sig`): the length, the masked hash (relocated words and
the `rel32` fields leaving the function zeroed) and, recursively, what
those `rel32` fields reach (another function's identity, or an import
thunk) and the imports the code names. The census hash
(tools/overlay-census.py) also keeps read-only constants, so it splits
functions the ports treat as one; this tool regroups the census rows by
identity. Each unit is ported once; its owner is the lowest-numbered DLL
using it.

Input: the census's overlay-dll-functions.tsv (one row per function per
DLL: address, size) in --census (default $FFX_WORK/m5/magic/census2, the
recursive-descent census; the first census, `census`, cut functions at
`int3`-aligned addresses and has rows inside functions); each
DLL's code (objdump, for the `rel32` fields; cached in
$FFX_WORK/cache/magic-worklist); what the ports cover now
(`magic-sig --covered`/`--batch`, built from this checkout: cargo build
-p ffx-magic --examples); and the list each DLL belongs to, from --lists
(default $FFX_WORK/m5/magic/worklist.tsv, the first work list). List
membership is never recomputed: a rerun writes a new file (--out, default
worklist2.tsv next to the lists; an existing file is kept unless
--replace) and the lists file is only read.

The CRT is never ported: the functions of the CRT's mnemonic groups
(magic_0481's CRT tail, and magic_0267's stack probe at 0x10005c80) and
every function using the CRT's failure imports (`__report_gsfailure`,
`__raise_securityfailure`: IsDebuggerPresent, UnhandledExceptionFilter,
...) count as covered.

Order ("shared first"): every unit used by 2 or more DLLs and not covered,
callees before callers (a unit's uncovered callees come earlier; a call
cycle is kept together), then each DLL's own code, the DLLs the replays
reach first, then by number, again callees first.

Output: --out, one row per uncovered unit: order, identity, magic-sig
length and hash (the port's `Sig`), kind (shared/own), size, how many DLLs
use it, the first DLL and its address there, the owner's list, how many
of its callees are uncovered and which (identities), how many DLLs wait
on it, and the DLLs. Next to it, <out>-dlls.tsv: one row per DLL with its
list, functions, CRT, covered, own and shared functions left, the replays
reaching it, and the shared units it waits on (identities; a DLL is done
when both counts are 0).
"""
import argparse
import collections
import concurrent.futures
import hashlib
import os
import pathlib
import pickle
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import reach  # noqa: E402
REPO = HERE.parent
WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))
CACHE = WORK / "cache" / "magic-worklist"
VERSION = b"magic-worklist 2"

# What the replays reach (overlay-emu logs, 2026-10-29)
REPLAYS = {
    481: "battle-magic",
    390: "battle-overdrive", 595: "battle-overdrive", 604: "battle-overdrive",
    309: "battle-ifrit", 596: "battle-ifrit",
}
CRT_FROM = (481, 0x100033b8)  # magic_0481's CRT starts at __security_check_cookie
# CRT functions outside that tail: magic_0267's stack probe (_chkstk /
# _alloca_probe: the page-touching loop below esp)
CRT_ALSO = [(267, 0x10005c80)]
# __report_gsfailure / __raise_securityfailure and their kin
CRT_IMPORTS = {"IsDebuggerPresent", "UnhandledExceptionFilter", "SetUnhandledExceptionFilter",
               "IsProcessorFeaturePresent", "TerminateProcess", "GetCurrentProcess",
               "_crt_debugger_hook"}

INSN = re.compile(r"^\s*([0-9a-f]+):\t((?:[0-9a-f]{2} )+)\s*\t?(.*)$")
TARGET = re.compile(r"^(call|jmp|j[a-z]+)\s+0x([0-9a-f]+)\b")


def rel32_insns(path):
    """[(address, field offset in the instruction, target)] of every rel32
    call, jmp and jcc in the DLL's code (objdump -d, cached)."""
    data = pathlib.Path(path).read_bytes()
    ver = subprocess.run(["objdump", "--version"], capture_output=True, text=True, check=True).stdout
    key = hashlib.sha256(VERSION + data + ver.splitlines()[0].encode()).hexdigest()[:20]
    cached = CACHE / f"{key}.pickle"
    if cached.exists():
        with open(cached, "rb") as f:
            return pickle.load(f)
    out = subprocess.run(["objdump", "-d", "-M", "intel", "-w", str(path)],
                         capture_output=True, text=True, check=True).stdout
    found = []
    for line in out.splitlines():
        m = INSN.match(line)
        if not m:
            continue
        b = m.group(2).split()
        t = TARGET.match(m.group(3).strip())
        if not t:
            continue
        if b[0] in ("e8", "e9") and len(b) == 5:
            found.append((int(m.group(1), 16), 1, int(t.group(2), 16)))
        elif b[0] == "0f" and len(b) == 6 and b[1][0] == "8":
            found.append((int(m.group(1), 16), 2, int(t.group(2), 16)))
    CACHE.mkdir(parents=True, exist_ok=True)
    tmp = cached.with_suffix(f".{os.getpid()}.tmp")
    with open(tmp, "wb") as f:
        pickle.dump(found, f)
    os.replace(tmp, cached)
    return found


def run_sig(sig, mode, dll_path, lines):
    r = subprocess.run([str(sig), mode, str(dll_path)], input="".join(lines),
                       capture_output=True, text=True, check=True)
    return r.stdout.splitlines()


def scan_dll(args):
    """One DLL: per function (address, size): magic-sig hash, imports,
    the rel32 fields' targets, and whether a port covers it."""
    d, fs, path, sig = args
    insns = rel32_insns(path)
    starts = sorted(insns)
    import bisect
    lines, fields = [], {}
    for a, size, *_ in fs:
        i = bisect.bisect_left(starts, (a, 0, 0))
        offs = []
        while i < len(starts) and starts[i][0] < a + size:
            at, k, t = starts[i]
            if at + k + 4 <= a + size and not a <= t < a + size:
                offs.append(at + k - a)
            i += 1
        fields[a] = offs
        lines.append(f"{a:#x} {size} {' '.join(f'{o:#x}' for o in offs)}\n")
    info = {}
    for line in run_sig(sig, "--batch", path, lines):
        parts = line.split(" ")
        a = int(parts[0], 16)
        imports = [] if parts[2] == "-" else parts[2].split(",")
        callees = [p for p in parts[3:] if p]
        info[a] = (int(parts[1], 16), imports, callees)
    cov, bound = set(), {}
    addrs = [r[0] for r in fs]
    for k in range(0, len(addrs), 400):
        r = subprocess.run([str(sig), "--covered-len", str(path), *[f"{x:#x}" for x in addrs[k:k + 400]]],
                           capture_output=True, text=True, check=True)
        for line in r.stdout.splitlines():
            x, n, name = line.split(" ", 2)
            if name.strip() != "-":
                cov.add(int(x, 16))
                if n != "-":
                    bound[int(x, 16)] = int(n)
    return d, info, cov, fragments(path, fs, cov, bound, insns)


def fragments(path, fs, cov, bound, insns):
    """Rows that are pieces of a ported function rather than functions:
    an uncovered row starting inside the range a port binds at a covered
    row (its `Sig` length) that nothing enters on its own: no call
    targets it, no jump from outside that range, no relocated word or
    export points at it, and it is not the entry point. The first
    census cut such pieces after `int3`-aligned addresses (an epilogue
    `call __security_check_cookie; mov esp, ebp; pop ebp; ret` after
    `xor ecx, esp`, 0x33 0xcc); a port covers them, so they are no
    rows. A real separate entry inside a port's range stays a row."""
    import bisect
    spans = sorted((a, a + n) for a, n in bound.items())
    lo = [s for s, _ in spans]
    pieces, img, entries = {}, None, set()
    for a, _size, *_ in fs:
        if a in cov:
            continue
        i = bisect.bisect_right(lo, a) - 1
        if i < 0 or not spans[i][0] < a < spans[i][1]:
            continue
        if img is None:
            img = reach.Image(path)
            entries = {img.entry, *img.exports()} | {img.dword(r) for r in img.relocs}
        s, e = spans[i]

        def enters(at, t):  # a call to a, or a jump to a from outside the port's range
            return t == a and (img.dword(at) & 0xff == 0xe8 or not s <= at < e)

        if a in entries or any(enters(at, t) for at, _k, t in insns):
            continue
        pieces[a] = s
    return pieces


def read_lists(path):
    lists = {}
    if path.exists():
        with open(path, encoding="utf-8") as f:
            for line in f:
                if line.startswith("#"):
                    continue
                c = line.rstrip("\n").split("\t")
                lists[int(c[0][6:10])] = int(c[1])
    return lists


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--census", type=pathlib.Path, default=WORK / "m5/magic/census2")
    ap.add_argument("--lists", type=pathlib.Path, default=WORK / "m5/magic/worklist.tsv")
    ap.add_argument("--out", type=pathlib.Path, default=None)
    ap.add_argument("--replace", action="store_true", help="overwrite --out if it exists")
    ap.add_argument("--sig", type=pathlib.Path, default=REPO / "target/debug/examples/magic-sig")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    args = ap.parse_args()
    out_path = args.out or args.lists.with_name("worklist2.tsv")
    if out_path.resolve() == args.lists.resolve():
        sys.exit(f"{out_path} is the lists file: list membership is never rewritten")
    if out_path.exists() and not args.replace:
        sys.exit(f"{out_path} exists (a work list in use): pick another --out or pass --replace")
    if not args.sig.exists():
        sys.exit(f"{args.sig} missing: cargo build -p ffx-magic --examples")
    game = pathlib.Path(os.environ.get("FFX_GAME_DIR", pathlib.Path.home() / "steam-games/FFX-X2-HD"))
    rows = collections.defaultdict(list)  # dll -> [(addr, size, census hash, ops)]
    with open(args.census / "overlay-dll-functions.tsv", encoding="utf-8") as f:
        for line in f:
            if line.startswith("#"):
                continue
            d, a, size, h, ops, _n, _slots = line.rstrip("\n").split("\t")
            rows[int(d[6:10])].append((int(a, 16), int(size), h, ops))
    lists = read_lists(args.lists)
    crt_ops = {r[3] for r in rows[CRT_FROM[0]] if r[0] >= CRT_FROM[1]}
    crt_ops |= {r[3] for d, a in CRT_ALSO for r in rows[d] if r[0] == a}
    jobs = [(d, fs, game / f"magicFiles/FFX/magic_{d:04}.dll", args.sig) for d, fs in sorted(rows.items())]
    info, cov, pieces = {}, {}, {}
    with concurrent.futures.ThreadPoolExecutor(args.jobs) as ex:
        for d, i, c, p in ex.map(scan_dll, jobs):
            info[d], cov[d], pieces[d] = i, c, p
    # pieces of a ported function are no rows (see fragments())
    for d in rows:
        if pieces[d]:
            rows[d] = [r for r in rows[d] if r[0] not in pieces[d]]
    # identities, recursively over the callees inside each DLL
    size_of = {(d, r[0]): r[1] for d, fs in rows.items() for r in fs}
    memo = {}

    def ident(d, a, stack=()):
        k = (d, a)
        if k in memo:
            return memo[k]
        h, imports, callees = info[d][a]
        if k in stack:  # a call cycle: the masked code alone
            return f"cycle:{size_of[k]}:{h:x}"
        parts = [str(size_of[k]), f"{h:x}", ",".join(imports)]
        for c in callees:
            if c.startswith("imp:"):
                parts.append(c)
            else:
                t = int(c, 16)
                parts.append(ident(d, t, stack + (k,)) if (d, t) in size_of else f"at:{t - a:x}")
        memo[k] = hashlib.sha1("|".join(parts).encode()).hexdigest()[:16]
        return memo[k]

    units = {}  # identity -> dict
    of = {}  # (dll, addr) -> identity
    for d, fs in sorted(rows.items()):
        for a, size, ch, ops in fs:
            u = ident(d, a)
            of[(d, a)] = u
            h, imports, callees = info[d][a]
            crt = ops in crt_ops or bool(CRT_IMPORTS & ({*imports} | {c[4:] for c in callees if c.startswith("imp:")}))
            x = units.setdefault(u, {"size": size, "sig": (size, h), "dlls": [], "census": set(),
                                     "covered": False, "crt": False, "at": (d, a), "callees": set()})
            x["dlls"].append(d)
            x["census"].add(ch)
            x["covered"] |= a in cov[d]
            x["crt"] |= crt
    for (d, a), u in of.items():
        for c in info[d][a][2]:
            if not c.startswith("imp:") and (d, int(c, 16)) in of:
                t = of[(d, int(c, 16))]
                if t != u:
                    units[u]["callees"].add(t)
    done = {u for u, x in units.items() if x["covered"] or x["crt"]}
    todo = {u for u in units if u not in done}
    shared = {u for u in todo if len(set(units[u]["dlls"])) >= 2}

    # callees first (iterative DFS post-order; cycles stay together)
    def dep_order(us, key):
        order, seen = [], set()
        for root in sorted(us, key=key):
            if root in seen:
                continue
            stack = [(root, iter(sorted(units[root]["callees"] & us, key=key)))]
            seen.add(root)
            while stack:
                u, it = stack[-1]
                nxt = next(it, None)
                if nxt is None:
                    order.append(u)
                    stack.pop()
                elif nxt not in seen:
                    seen.add(nxt)
                    stack.append((nxt, iter(sorted(units[nxt]["callees"] & us, key=key))))
        return order

    ordered = dep_order(shared, lambda u: (-len(set(units[u]["dlls"])), units[u]["size"], u))
    own_by_dll = collections.defaultdict(set)
    for u in todo - shared:
        own_by_dll[units[u]["dlls"][0]].add(u)
    for d in sorted(own_by_dll, key=lambda d: (d not in REPLAYS, d)):
        ordered += dep_order(own_by_dll[d], lambda u: (units[u]["at"][1], u))
    waiting = collections.Counter()
    waits_of = {}
    for d, fs in rows.items():
        w = sorted({of[(d, r[0])] for r in fs} & shared)
        waits_of[d] = w
        for u in w:
            waiting[u] += 1
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write("# order\tidentity\tsig len\tsig hash\tkind\tsize\tdlls\tfirst dll\taddress there\t"
                "owner's list\tuncovered callees\tcallees (identities)\tdlls waiting\tdlls\n")
        for n, u in enumerate(ordered, 1):
            x = units[u]
            ds = sorted(set(x["dlls"]))
            cal = sorted(x["callees"] & todo)
            f.write("\t".join(str(v) for v in [
                n, u, x["sig"][0], f"{x['sig'][1]:#018x}", "shared" if u in shared else "own", x["size"],
                len(ds), f"magic_{ds[0]:04}", f"{x['at'][1]:#010x}", lists.get(ds[0], ""), len(cal),
                ",".join(cal) or "-", waiting.get(u, 0), " ".join(f"{d:04}" for d in ds)]) + "\n")
    dll_path = out_path.with_name(out_path.stem + "-dlls.tsv")
    if dll_path.exists() and not args.replace:
        sys.exit(f"{dll_path} exists: pick another --out or pass --replace")
    with open(dll_path, "w", encoding="utf-8") as f:
        f.write("# dll\tlist\tfunctions\tcrt\tcovered\town left\tshared left\treplays\twaits on (shared identities)\n")
        for d, fs in sorted(rows.items()):
            us = [of[(d, r[0])] for r in fs]
            crt = sum(1 for u in us if units[u]["crt"])
            covd = sum(1 for u in us if units[u]["covered"] and not units[u]["crt"])
            own_left = sum(1 for u in us if u in todo and u not in shared)
            f.write("\t".join(str(v) for v in [
                f"magic_{d:04}", lists.get(d, ""), len(fs), crt, covd, own_left, len(waits_of[d]),
                REPLAYS.get(d, ""), ",".join(waits_of[d]) or "-"]) + "\n")
    census_n = len({r[2] for fs in rows.values() for r in fs})
    merged = sum(1 for x in units.values() if len(x["census"]) > 1)
    full = [d for d in rows if not waits_of[d] and not any(of[(d, r[0])] in todo for r in rows[d])]
    print(f"{len(rows)} DLLs; {len(units)} distinct functions by code identity ({census_n} census hashes; "
          f"{merged} identities join several census hashes); CRT {sum(1 for x in units.values() if x['crt'])}, "
          f"covered {sum(1 for x in units.values() if x['covered'] and not x['crt'])}, left {len(todo)} "
          f"({sum(units[u]['size'] for u in todo) // 1024} KB): shared first {len(shared)} "
          f"({sum(units[u]['size'] for u in shared) // 1024} KB), own {len(todo) - len(shared)}")
    print(f"rows inside a port's range, not functions: {sum(len(p) for p in pieces.values())} "
          f"in {sum(1 for p in pieces.values() if p)} DLLs")
    print(f"DLLs fully covered: {len(full)}; waiting on shared code: "
          f"{sum(1 for d in rows if waits_of[d])}")
    print(f"wrote {out_path} and {dll_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
