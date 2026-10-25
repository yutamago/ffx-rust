#!/usr/bin/env python3
"""What the game reaches of the statically linked libraries (M4 pilot).

  tools/coverage-phyre.py HITS [--list GROUP] [--json]

HITS: the hits of a coverage run that armed every function
(`COVERAGE_ALL=1 tools/coverage-run.sh`, tools/coverage.py candidates --all),
one address per line. Counts per module (tools/progress.py's placement:
names first, unnamed functions by their named neighbours), then splits the
`phyre` module's reached functions into subsystems:

  render    scene traversal, culling, sort, constant fill, render commands,
            render thread, D3D11 wrappers (PRendering, PRenderer, PShader,
            PTexture, PEffect, PMaterial, PMesh, PDataBlock, PRender*, D3D11)
  load      serialization and PBinary/cluster loading (PSerialization,
            PCluster, PBinary, PClassDescriptor, PNamespace, PAsset, fixups)
  math      Vectormath, PMatrix, PVector, PQuaternion, bounds and frustum math
  jobs      PThreadPool, jobs, semaphores, threads, the ring allocators
  anim      PAnimation (clips, networks)
  media     the movie player's libraries (vorbis, mkvparser, PVideo)
  containers PArray, PSharray, PHashTable, PString, PList, the heap
  other     the rest (animation, physics bindings, input, file IO, ...)

An unnamed library function takes the subsystem its callers give it when
they agree (the dump's call graph, reached callers only), else its named
neighbours'; such counts are estimates and are reported apart.
Also counts the reached functions symbols/classes.tsv calls `glue`.
"""
import argparse
import bisect
import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

RULES = [
    ("render", r"PRendering|PRenderer|Render|PShader|PTexture|PEffect|PMaterial|PMesh|PDataBlock|"
               r"PVertex|PIndex|PSampler|PBlend|PDepth|PRaster|D3D11|Dxgi|DXGI|PCamera|PLight|"
               r"PNode|PWorld|PInstance|PScene|PSkin|Shadow|PParameter|PConstant|Visib|Cull|"
               r"PCheckVisible|BoundingBox|PGpu|PDraw|PSurface|PViewport|PFramework|PDynamicGeometry|"
               r"PGeometry|PSprite|PText|PPostProcessing|ShaderParams"),
    ("load", r"PSerialization|PCluster|PBinary|PClassDescriptor|PNamespace|PAsset|Fixup|PFixup|"
             r"PObjectSet|PClusterRead|loadCluster|PDatabase|PChunk|PInstanceList|PClass|PType"),
    ("math", r"Vectormath|PMatrix|PVector|PQuaternion|PMath|Matrix|Vector|Quat|Frustum|PAabb|"
             r"PSphere|PPlane|Transform"),
    ("jobs", r"PThread|Job|Semaphore|PMutex|PCritical|PEvent|PAtomic|RingBuffer|PSpu|Worker|"
             r"PSync|PLock"),
    ("anim", r"PAnimation"),
    ("media", r"vorbis|ogg_|mkvparser|PVideo|PAudio"),
    ("containers", r"PArray|PSharray|PHash|PString|PList|PMap|PSet|Heap|Malloc|Free|PAlloc|"
                   r"PMemory|PStack|PQueue|PBitArray|PStrCmp|PStrlen|PStr"),
]
RX = [(g, re.compile(p)) for g, p in RULES]
GROUPS = [g for g, _ in RULES] + ["other"]
WORK = progress.WORK


def group_of(name):
    for g, rx in RX:
        if rx.search(name):
            return g
    return None


def callers():
    """callee -> callers (the dump's calls.tsv)."""
    out = collections.defaultdict(set)
    path = WORK / "decomp/calls.tsv"
    if not path.exists():
        return out
    with open(path, encoding="utf-8") as f:
        next(f)
        for line in f:
            p = line.rstrip("\n").split("\t")
            if len(p) >= 2:
                try:
                    out[int(p[1], 16)].add(int(p[0], 16))
                except ValueError:
                    pass
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("hits")
    ap.add_argument("--list", metavar="GROUP")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    hits = {int(x, 16) for x in pathlib.Path(args.hits).read_text().split()}
    rows = progress.functions()
    names = progress.symbols()
    ghidra = {a: n for a, n, _ in rows}
    size = {a: s for a, _, s in rows}
    module = progress.placement(rows, names)
    cls = progress.classes()

    per_mod = collections.defaultdict(lambda: [0, 0, 0, 0])  # all n, all bytes, hit n, hit bytes
    for a, _, s in rows:
        m = per_mod[module[a]]
        m[0] += 1
        m[1] += s
        if a in hits:
            m[2] += 1
            m[3] += s

    # the phyre module's reached functions by subsystem
    phyre = sorted(a for a in hits if module.get(a) == "phyre")
    direct = {}
    for a in phyre:
        n = names.get(a) or ghidra.get(a, "")
        g = group_of(n) if not n.startswith("FUN_") else None
        if g is None and n and not n.startswith("FUN_") and (a in names or not n.startswith("FUN_")):
            g = "other" if a in names else None
        direct[a] = g
    cl = callers()
    guessed = {}
    named_keys = sorted(a for a, g in direct.items() if g)
    for _ in range(4):  # propagate from reached callers
        for a in phyre:
            if direct.get(a) or a in guessed:
                continue
            gs = {direct.get(c) or guessed.get(c) for c in cl.get(a, ()) if c in hits}
            gs.discard(None)
            if len(gs) == 1:
                guessed[a] = gs.pop()
    for a in phyre:
        if direct.get(a) or a in guessed:
            continue
        i = bisect.bisect(named_keys, a)
        near = [named_keys[j] for j in (i - 1, i) if 0 <= j < len(named_keys)]
        guessed[a] = direct[min(near, key=lambda k: abs(k - a))] if near else "other"
    stat = {g: [0, 0, 0, 0] for g in GROUPS}  # named n, bytes, guessed n, bytes
    for a in phyre:
        if direct.get(a):
            stat[direct[a]][0] += 1
            stat[direct[a]][1] += size[a]
        else:
            stat[guessed[a]][2] += 1
            stat[guessed[a]][3] += size[a]
    glue = [a for a in hits if cls.get(a) == "glue"]
    glue_all = [a for a, c in cls.items() if c == "glue"]

    if args.list:
        for a in phyre:
            g = direct.get(a) or guessed.get(a)
            if g == args.list:
                tag = "" if direct.get(a) else " (guessed)"
                print(f"0x{a:08x} {size[a]:6d} {names.get(a) or ghidra.get(a)}{tag}")
        return
    report = {
        "modules": {m: dict(zip(["funcs", "bytes", "reached", "reached_bytes"], v)) for m, v in per_mod.items()},
        "phyre_groups": {g: dict(zip(["named", "named_bytes", "guessed", "guessed_bytes"], v)) for g, v in stat.items()},
        "glue": {"reached": len(glue), "reached_bytes": sum(size.get(a, 0) for a in glue),
                 "all": len(glue_all), "all_bytes": sum(size.get(a, 0) for a in glue_all)},
        "hits": len(hits),
    }
    if args.json:
        print(json.dumps(report, indent=1))
        return
    print(f"{len(hits)} functions reached of {len(rows)}")
    print(f"{'module':10} {'funcs':>7} {'KB':>7} {'reached':>8} {'KB':>7} {'%fn':>6}")
    for m, (n, b, h, hb) in sorted(per_mod.items(), key=lambda kv: -kv[1][3]):
        print(f"{m:10} {n:7d} {b / 1024:7.0f} {h:8d} {hb / 1024:7.0f} {100 * h / max(n, 1):6.1f}")
    print("\nphyre, reached, by subsystem (named | placed by callers/neighbours)")
    print(f"{'group':11} {'named':>6} {'KB':>6} {'placed':>7} {'KB':>6} {'total':>6} {'KB':>6}")
    for g in GROUPS:
        n, b, gn, gb = stat[g]
        print(f"{g:11} {n:6d} {b / 1024:6.1f} {gn:7d} {gb / 1024:6.1f} {n + gn:6d} {(b + gb) / 1024:6.1f}")
    t = [sum(stat[g][i] for g in GROUPS) for i in range(4)]
    print(f"{'total':11} {t[0]:6d} {t[1] / 1024:6.1f} {t[2]:7d} {t[3] / 1024:6.1f} {t[0] + t[2]:6d} {(t[1] + t[3]) / 1024:6.1f}")
    g = report["glue"]
    print(f"\nglue (symbols/classes.tsv): {g['reached']} reached ({g['reached_bytes'] / 1024:.1f} KB) "
          f"of {g['all']} ({g['all_bytes'] / 1024:.1f} KB)")


if __name__ == "__main__":
    main()
