#!/usr/bin/env python3
"""magic-worklist.py: the work list for porting the spell overlays
(docs/formats/magic.md, "Porting the overlays").

  FFX_GAME_DIR=... tools/magic-worklist.py [--census DIR] [--lists 15] [--out FILE]

Input: tools/overlay-census.py's overlay-dll-functions.tsv (one row per
function per DLL: address, size, hash, ops group, how many DLLs share the
hash, slots) in --census (default $FFX_WORK/m5/magic/census), and what the
ffx-magic ports cover today (`magic-sig --covered`, built from this
checkout: cargo build -p ffx-magic --examples): an entry a port binds to,
a function a port reproduces inside another (`ffx_magic::INTERNAL`), or
the CRT's internals (the DllMain port; functions of the CRT's mnemonic
groups, taken from magic_0481's CRT tail).

Units are distinct functions (census hashes). Each is ported once, by its
owner: the list of the lowest-numbered DLL that uses it. The runtime
(hashes in 100+ DLLs) is ported first, in the pilot; the common pool
(10-99 DLLs) next, each by the list holding most of its DLLs. DLLs are cut
into --lists contiguous number ranges of about equal bytes (neighbouring
numbers are often the same spell family, so most shared code stays in one
list). Within a list: the DLLs a replay reaches first, then by bytes left
(small first, so DLLs finish early).

Output (tab-separated, one row per DLL): dll, list, order, functions,
own (1 DLL), shared (2-99), runtime (100+), bytes left (owned by this
DLL), functions covered now, shared functions it waits for from other
lists, replays, status (done when every function is covered).
"""
import argparse
import collections
import os
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent
WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))

# What the replays reach (overlay-emu logs, 2026-10-29)
REPLAYS = {
    481: "battle-magic",
    390: "battle-overdrive", 595: "battle-overdrive", 604: "battle-overdrive",
    309: "battle-ifrit", 596: "battle-ifrit",
}
CRT_FROM = (481, 0x100033b8)  # magic_0481's CRT starts at __security_check_cookie


def covered(dll_path, addrs, sig):
    """Addresses of `addrs` the ffx-magic ports cover."""
    out = set()
    for i in range(0, len(addrs), 400):
        chunk = addrs[i:i + 400]
        r = subprocess.run([str(sig), "--covered", str(dll_path), *[f"{a:#x}" for a in chunk]],
                           capture_output=True, text=True, check=True)
        for line in r.stdout.splitlines():
            a, _, name = line.partition(" ")
            if name.strip() != "-":
                out.add(int(a, 16))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--census", type=pathlib.Path, default=WORK / "m5/magic/census")
    ap.add_argument("--lists", type=int, default=15)
    ap.add_argument("--out", type=pathlib.Path, default=WORK / "m5/magic/worklist.tsv")
    ap.add_argument("--sig", type=pathlib.Path, default=REPO / "target/debug/examples/magic-sig")
    args = ap.parse_args()
    game = pathlib.Path(os.environ.get("FFX_GAME_DIR", pathlib.Path.home() / "steam-games/FFX-X2-HD"))
    rows = collections.defaultdict(list)  # dll -> [(addr, size, hash, ops, shared)]
    with open(args.census / "overlay-dll-functions.tsv", encoding="utf-8") as f:
        for line in f:
            if line.startswith("#"):
                continue
            d, a, size, h, ops, n, _slots = line.rstrip("\n").split("\t")
            rows[int(d[6:10])].append((int(a, 16), int(size), h, ops, int(n)))
    crt_ops = {r[3] for r in rows[CRT_FROM[0]] if r[0] >= CRT_FROM[1]}
    if not args.sig.exists():
        sys.exit(f"{args.sig} missing: cargo build -p ffx-magic --examples")
    cov = {}
    for d, fs in rows.items():
        c = covered(game / f"magicFiles/FFX/magic_{d:04}.dll", [r[0] for r in fs], args.sig)
        cov[d] = c | {r[0] for r in fs if r[3] in crt_ops}
    done_h = {r[2] for d, fs in rows.items() for r in fs if r[0] in cov[d]}
    size_of, dlls_of = {}, collections.defaultdict(set)
    for d, fs in rows.items():
        for r in fs:
            size_of[r[2]] = max(size_of.get(r[2], 0), r[1])
            dlls_of[r[2]].add(d)
    todo = {h for h in dlls_of if h not in done_h}
    # owner DLL of each unit: the lowest number using it
    owner = {h: min(ds) for h, ds in dlls_of.items()}
    dll_bytes = collections.Counter()
    for h in todo:
        if len(dlls_of[h]) < 10:
            dll_bytes[owner[h]] += size_of[h]
    # contiguous ranges of about equal bytes
    order = sorted(rows)
    total = sum(dll_bytes[d] for d in order) + sum(size_of[h] for h in todo if len(dlls_of[h]) >= 10)
    per = total / args.lists
    list_of, acc, i = {}, 0, 0
    for d in order:
        if acc >= per * (i + 1) and i < args.lists - 1:
            i += 1
        list_of[d] = i
        acc += dll_bytes[d]
    load = collections.Counter()
    for d in order:
        load[list_of[d]] += dll_bytes[d]
    # the common pool: to the list with most of its DLLs
    list_of_h = {}
    for h in todo:
        if len(dlls_of[h]) >= 10:
            c = collections.Counter(list_of[d] for d in dlls_of[h])
            list_of_h[h] = max(c, key=lambda k: (c[k], -k))
            load[list_of_h[h]] += size_of[h]
        else:
            list_of_h[h] = list_of[owner[h]]
    out = []
    for d, fs in sorted(rows.items()):
        own = sum(1 for r in fs if r[4] == 1)
        shared = sum(1 for r in fs if 2 <= r[4] < 100)
        runtime = sum(1 for r in fs if r[4] >= 100)
        waits = len({r[2] for r in fs if r[2] in todo and list_of_h[r[2]] != list_of[d]})
        ncov = len(cov[d])
        status = "done" if ncov == len(fs) else ("started" if any(
            r[4] < 100 and r[0] in cov[d] and r[3] not in crt_ops for r in fs) else "")
        out.append([d, list_of[d] + 1, 0, len(fs), own, shared, runtime, dll_bytes[d], ncov, waits,
                    REPLAYS.get(d, ""), status])
    for k in range(1, args.lists + 1):
        mine = [r for r in out if r[1] == k]
        mine.sort(key=lambda r: (r[11] == "done", r[10] == "", r[7], r[0]))
        for n, r in enumerate(mine, 1):
            r[2] = n
    out.sort(key=lambda r: (r[1], r[2]))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as f:
        f.write("# dll\tlist\torder\tfunctions\town (1 DLL)\tshared (2-99)\truntime (100+)\t"
                "bytes left (owned)\tcovered now\twaits on other lists\treplays\tstatus\n")
        for r in out:
            f.write("\t".join(f"magic_{x:04}" if j == 0 else str(x) for j, x in enumerate(r)) + "\n")
    runtime_h = {h for h, ds in dlls_of.items() if len(ds) >= 100}
    print(f"{len(rows)} DLLs, {len(dlls_of)} distinct functions ({sum(size_of.values()) // 1024} KB); "
          f"covered now {len(done_h)} ({len(done_h & runtime_h)} of the {len(runtime_h)} runtime), "
          f"{len(todo)} left ({sum(size_of[h] for h in todo) // 1024} KB)")
    for k in range(args.lists):
        ds = [r for r in out if r[1] == k + 1]
        units = sum(1 for h in todo if list_of_h[h] == k)
        nums = [r[0] for r in ds]
        print(f"list {k + 1:2}: magic_{min(nums):04}..magic_{max(nums):04}, {len(ds):3} DLLs, "
              f"{units:4} functions, {load[k] // 1024:4} KB")
    done = [f"magic_{r[0]:04}" for r in out if r[11] == "done"]
    print(f"DLLs fully covered: {len(done)}: {' '.join(done[:30])}{' ...' if len(done) > 30 else ''}")


if __name__ == "__main__":
    sys.exit(main())
