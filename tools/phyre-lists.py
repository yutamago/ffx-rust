#!/usr/bin/env python3
"""M5 stage 1 PhyreEngine port lists from tools/phyre-classes.py's scope.

  FFX_GAME_DIR=... tools/phyre-lists.py [--needed TSV] [--with-script]
                                        [--out DIR] [--min N] [--max N]

Reads phyre-needed.tsv (tools/phyre-classes.py: reachable, not hooked
PhyreEngine functions; column 5 marks the ones only the script layer
reaches, left out unless --with-script) and splits it into port lists of
--min..--max functions (default 250..400), one family per list, leaf-first
inside a list (callees before callers, cycles kept together; tarjan order
over the needed functions' own call graph).

A function's family, first match wins:
  1. template: it is a slot of a descriptor-template vtable
     (PClassDescriptor*<T>, PClassDataMember*<T>, PNamedSemantic*<T>):
     family "registry-<template family>" -- the same code shape for every T,
     so one agent ports the template once and instantiates it;
  2. its own name (tools/coverage-phyre.py's subsystem rules, plus physics,
     input, framework, gameplay, scripting, media);
  3. its named neighbours by address in the phyre module (object files are
     contiguous; nearest named function within --near bytes);
  4. its callers in the needed set when they agree, else its callees';
  5. the class or utility table it is reached through (phyre-classes.py's
     label), by name;
  6. the nearest named phyre function at any distance.
Under 5 bytes (not hookable in-process) go to stubs.list; the movie
libraries (mkvparser, vorbis, libvpx: stage 2 replaces them) to media.list,
not ported in stage 1.

Writes DIR/<list>.list (one address per line), DIR/lists.tsv (address,
list, family, how placed, size, name, slots) and prints the summary table.
"""
import argparse
import bisect
import math
import os
import pathlib
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402
import reach  # noqa: E402
import refs  # noqa: E402

pc = __import__("phyre-classes")

WORK = progress.WORK
# family by name; order matters (first match)
RULES = [
    ("media", r"mkvparser|vorbis|ogg_|vpx|vp8|vp9|libwebm|PVideo"),
    ("physics", r"PPhysics|^bt[A-Z]|_bt[A-Z]|Bullet|btCollision|btDiscrete|btSequential"),
    ("input", r"PInput|Joypad|Keyboard|PMouse|Pad(XInput|DirectInput)|PTouch|PMotion"),
    ("scripting", r"PScript|PScheduler|Lua|lua_|PCallerScript|PMethodCaller|PFunctionCaller"),
    ("anim", r"PAnimation|PTimeController|PTimeInterval|PTimeScale|PSkeleton|PSkin"),
    ("postfx", r"PPostProcessing|PDeferredLighting|PDepthOfField|PGlow|PFXAA|PMLAA|PMotionBlur|"
               r"PScreenSpace|PPostEffect|PLegacyGlow"),
    ("scene", r"PNode|PWorld|PInstance|PMeshInstance|PLight|PCamera|PLocator|PEntity|PComponent|"
              r"PScene|PLOD|PSpline|PTrigger|PGameplay|PWorldMatrix|PContextSwitch|POccluder|PShadowCaster|"
              r"PVisib|Cull|PCheckVisible"),
    ("render", r"PRendering|PRenderer|Render|PShader|PTexture|PEffect|PMaterial|PMesh|PDataBlock|"
               r"PVertex|PIndex|PSampler|PBlend|PDepth|PRaster|D3D11|Dxgi|DXGI|PParameter|PConstant|"
               r"PGpu|PDraw|PSurface|PViewport|PDynamicGeometry|PGeometry|PSprite|PText|ShaderParams|"
               r"PStructuredBuffer|PIndirectArgs|PFont|FogEffect|PS2Lighting|CharacterShader|"
               r"CharacterLighting"),
    ("framework", r"PFramework|PApplication|PWindow|PTimer|PhyreOS|PhyreInit|PhyreExit|PGameSettings|"
                  r"PProfiling|PFile|PStream|PUtilityFramework"),
    ("load", r"PSerialization|PCluster|PBinary|PAsset|Fixup|PFixup|PObjectSet|PClusterRead|loadCluster|"
             r"PDatabase|PChunk|PInstanceList"),
    ("registry", r"PClassDescriptor|PNamespace|PClassDataMember|PClassMember|PType|PNamedSemantic|"
                 r"PEnumDescriptor|PClassBind|PTypedObject"),
    ("math", r"Vectormath|PMatrix|PVector|PQuaternion|PMath|Matrix|Vector|Quat|Frustum|PAabb|"
             r"PSphere|PPlane|Transform|maxFloat|minFloat|Lerp|PRandom"),
    ("jobs", r"PThread|Job|Semaphore|PMutex|PCritical|PEvent|PAtomic|RingBuffer|PSpu|Worker|"
             r"PSync|PLock"),
    ("containers", r"PArray|PSharray|PHash|PString|PList|PMap|PSet|Heap|Malloc|Free|PAlloc|"
                   r"PMemory|PStack|PQueue|PBitArray|PStr|PRedBlackTree|PFreeList|MemoryAllocator"),
]
RX = [(f, re.compile(p)) for f, p in RULES]
TEMPLATE_FAMILIES = [f for f in pc.GATED if f not in pc.SCRIPT]
# families that share a list (one owner): small template families, the
# loader with the class registry's own code, the core libraries
LIST_OF = {
    "registry-PClassDescriptorAbstract": "tmpl-abstract",
    "registry-PClassDescriptorForType": "tmpl-fortype",
    "registry-PClassDescriptorConcrete": "tmpl-concrete",
    "registry-PClassDescriptorWithoutDefaultConstructor": "tmpl-concrete",
    "registry-PClassDescriptorWrapper": "tmpl-concrete",
    "registry-PClassDescriptorHeader": "tmpl-concrete",
    "registry-PClassDescriptorHeaderWithoutDefaultConstructor": "tmpl-concrete",
    "registry-PNamedSemanticDescriptorForType": "tmpl-concrete",
    "registry-PNamedSemanticDescriptorForDynamicType": "tmpl-concrete",
    "registry-PClassDataMemberPhyreArray": "tmpl-members-phyrearray",
    "registry-PClassDataMemberPhyreSharray": "tmpl-members-phyrearray",
    "registry-PClassDataMemberDynamicType": "tmpl-members-phyrearray",
    "registry-PClassDataMemberArrayDynamic": "tmpl-members-dynamic",
    "registry": "registry-load",
    "load": "registry-load",
    "containers": "core",
    "math": "core",
    "jobs": "core",
    "framework": "framework-input",
    "input": "framework-input",
    "postfx": "render",
}


def family_of_name(name):
    if not name or re.match(r"(FUN|code|LAB|thunk_FUN)_[0-9a-f]+$", name):
        return None
    for f, rx in RX:
        if rx.search(name):
            return f
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--needed", type=pathlib.Path, default=WORK / "m5/scope/phyre-needed.tsv")
    ap.add_argument("--with-script", action="store_true", help="keep the script-layer-only functions")
    ap.add_argument("--out", type=pathlib.Path, default=WORK / "m5/phyre")
    ap.add_argument("--min", type=int, default=250)
    ap.add_argument("--max", type=int, default=400)
    ap.add_argument("--near", type=int, default=0x400,
                    help="a named neighbour counts within this many bytes (default 0x400)")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    args = ap.parse_args()

    rows = []
    for line in args.needed.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        f = line.split("\t")
        if f[4] == "1" and not args.with_script:
            continue
        rows.append((int(f[0], 16), f[1], int(f[2]), f[3]))
    need = {a for a, *_ in rows}
    size = {a: s for a, _, s, _ in rows}
    label = {a: lab for a, _, _, lab in rows}

    img = reach.Image(refs.default_exe())
    g = reach.Graph(img, args.jobs, True)
    name = {a: g.name.get(a, n) for a, n, *_ in rows}

    # ---- descriptor-template slots
    slot_of = defaultdict(set)  # function -> {(family, slot)}
    for start, n, rtti in pc.vtables(img):
        t = pc.template_of(rtti)
        if not t or t[0] not in pc.GATED:
            continue
        for k in range(n):
            w = g.node_of(img.dword(start + 4 * k))
            if w in need:
                slot_of[w].add((t[0], k))

    fam, how = {}, {}
    for a in need:
        fs = {f for f, _ in slot_of.get(a, ())}
        tf = [f for f in TEMPLATE_FAMILIES if f in fs]
        if tf:
            fam[a], how[a] = "registry-" + tf[0], "template"
            continue
        if fs & set(pc.SCRIPT):
            fam[a], how[a] = "scripting", "template"
            continue
        f = family_of_name(name[a])
        if f:
            fam[a], how[a] = f, "name"
    # named neighbours: every phyre-module function with a family by name
    named = sorted((a, f) for a, f in ((a, family_of_name(n)) for a, n in g.name.items()
                                       if g.module.get(a) == "phyre") if f)
    keys = [a for a, _ in named]
    for a in sorted(need - set(fam)):
        i = bisect.bisect(keys, a)
        near = [named[j] for j in (i - 1, i) if 0 <= j < len(named) and abs(named[j][0] - a) <= args.near]
        if near:
            fam[a], how[a] = min(near, key=lambda p: abs(p[0] - a))[1], "neighbour"
    callers = defaultdict(set)
    for s, ds in g.out.items():
        for d in ds:
            if d in need and s in need:
                callers[d].add(s)
    for _ in range(6):
        changed = False
        for a in sorted(need - set(fam)):
            for edges, tag in ((callers.get(a, ()), "callers"), (g.out.get(a, ()), "callees")):
                fs = {fam[c] for c in edges if c in fam}
                if len(fs) == 1:
                    fam[a], how[a] = fs.pop(), tag
                    changed = True
                    break
        if not changed:
            break
    for a in sorted(need - set(fam)):
        f = family_of_name(re.sub(r"^\w+<(.*)>$", r"\1", label[a]))
        if f:
            fam[a], how[a] = f, "reached-through"
            continue
        # the movie libraries only take their own neighbourhood
        i = bisect.bisect(keys, a)
        near = [named[j] for j in (i - 1, i) if 0 <= j < len(named)
                and (named[j][1] != "media" or abs(named[j][0] - a) <= 0x4000)]
        if not near:
            near = [p for p in named if p[1] != "media"]
        if near:
            fam[a], how[a] = min(near, key=lambda p: abs(p[0] - a))[1], "far-neighbour"
        else:
            fam[a], how[a] = "other", "none"

    # ---- leaf-first order over the needed graph
    sub = {a: [d for d in g.out.get(a, ()) if d in need] for a in sorted(need)}
    order = [a for comp in reach.tarjan_order(sorted(need), sub) for a in comp]
    stubs = [a for a in order if size[a] < 5 and fam[a] != "media"]
    media = [a for a in order if fam[a] == "media"]
    skip = set(stubs) | set(media)
    per = defaultdict(list)
    for a in order:
        if a not in skip:
            per[LIST_OF.get(fam[a], fam[a])].append(a)
    lists = []
    for f in sorted(per, key=lambda f: (-len(per[f]), f)):
        addrs = per[f]
        n = max(1, math.ceil(len(addrs) / args.max))
        if n > 1 and len(addrs) / n < 0.8 * args.min:  # rather one long list than two short ones
            n -= 1
        k = math.ceil(len(addrs) / n)
        for i in range(n):
            lists.append((f if n == 1 else f"{f}-{chr(ord('a') + i)}", f, addrs[i * k:(i + 1) * k]))
    if stubs:
        lists.append(("stubs", "stubs", stubs))
    if media:
        lists.append(("media", "media (stage 2)", media))

    args.out.mkdir(parents=True, exist_ok=True)
    for old in args.out.glob("phyre-*.list"):
        old.unlink()
    with open(args.out / "lists.tsv", "w", encoding="utf-8") as tsv:
        tsv.write("# address\tlist\tfamily\tplaced by\tsize\tname\ttemplate slots\treached through\n")
        for ln, f, addrs in lists:
            (args.out / f"phyre-{ln}.list").write_text("".join(f"0x{a:08x}\n" for a in addrs), encoding="utf-8")
            for a in addrs:
                sl = ",".join(f"{t}:{k}" for t, k in sorted(slot_of.get(a, ())))
                tsv.write(f"0x{a:08x}\t{ln}\t{fam[a]}\t{how[a]}\t{size[a]}\t{name[a]}\t{sl}\t{label[a]}\n")
    print(f"{len(need)} functions{' (script layer in)' if args.with_script else ''}; placed by "
          + ", ".join(f"{k} {v}" for k, v in Counter(how.values()).most_common()))
    print(f"{'list':34} {'funcs':>6} {'KB':>6}  placed by")
    for ln, f, addrs in lists:
        c = Counter(how[a] for a in addrs)
        print(f"{'phyre-' + ln:34} {len(addrs):6d} {sum(size[a] for a in addrs) / 1024:6.1f}  "
              + ", ".join(f"{k} {v}" for k, v in c.most_common()))


if __name__ == "__main__":
    main()
