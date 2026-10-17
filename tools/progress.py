#!/usr/bin/env python3
"""How far the rewrite is, per module (docs/rewrite-plan.md).

Reads the Ghidra dump ($FFX_WORK/decomp/functions.tsv, tools/ghidra/dump.sh),
symbols/ffx.sym and the hook table (crates/ffx-hook/src/lib.rs).

A function's module comes from its name (the original PS2 prefixes, which
the Switch build's symbols carry); an unnamed function takes the module of
its named neighbours (both sides when they agree, else the nearer one), so
the sizes of modules are estimates until most of the code is named.

Counts:
  named     the function has a name in symbols/ffx.sym
  replaced  a Rust function runs instead of it (hook without a trampoline)
  wrapped   a Rust function runs in front of it and calls the original
  to port   logic functions (symbols/classes.tsv, tools/classify.py) not
            replaced yet: what is left to port exactly; glue, platform,
            debug and dead code is replaced or dropped instead

Usage: tools/progress.py [--json] [--unknown]
  --unknown  list the named functions no module rule matches
"""
import argparse
import bisect
import json
import os
import pathlib
import re
import sys
from collections import defaultdict

REPO = pathlib.Path(__file__).resolve().parent.parent
WORK = pathlib.Path(os.environ.get("FFX_WORK", pathlib.Path.home() / "ffx-work"))

# (module, kind, name pattern); first match wins. Library modules are
# replaced at the boundary the game uses, not rewritten function by function.
MODULES = [
    ("phyre", "library", r"^(Phyre_|PCluster|PRendering|Vectormath|PhyreOS)"),
    ("bullet", "library", r"^bt[A-Z]"),
    ("lua", "library", r"^(lua[A-Z_]|luaL_|lua_)"),
    ("zlib", "library", r"^(zlib_|inflate|deflate|adler32|crc32)"),
    ("crt", "library", r"^(_|std_|operator|Unwind|Catch|ATL_|CMFC|D2D1|`|mem(cpy|set|move|cmp|chr)$|str[a-z]+$|"
                       r"[a-z]?printf$|f?open$|f?close$|malloc$|free$|calloc$|realloc$|qsort$)"),
    ("battle", "game", r"^(Ms|ms)[A-Z_]|^(brnd|rnd)$|^(DmgCalc|Battle|Mon|Enc|Dmg)_"),
    ("script", "game", r"^(Atel|CT_|Native_|Script_|Worker_|Event_)"),
    # own names of the PS2 menu code (MenuItem_*, MenuSummon_*, ...); the PC
    # port's MenuManager/MenuTextureManager/MenuDynamicGeometry are pc-ui
    ("menu", "game", r"^(Tk|TK|tk)[A-Z_]|^(Gu_|Msg_|Text_)|"
                     r"^Menu(?!Manager|TextureManager|DynamicGeometry)[A-Za-z0-9]+_"),
    ("vfx", "game", r"^(ppp|ClassVFX|VFX|graphicVFX|op[a-z]*_|BrokenScreenEffect|VfxShader)"),
    ("render", "game", r"^(graphic|Graphic|PhyreScene|Shadow|PostProcess|Distortion|TextureAnimation|DynGeo|"
                       r"RadialLine|tex_|Cache_|Render_)"),
    ("chara", "game", r"^(Ch_|ChN_|ch_|chr|Chr_)"),
    ("main", "game", r"^(Sg_|sg_|SG_|FFXApplication|Cam_|Screen_|ei|sa_|Game_)"),
    ("map", "game", r"^(rc|Yn_|yi[A-Z_]|Gu[A-Z]|Ke[A-Z]|Sqs|Nkz|Map|abmap|Guide_|Minimap_)"),
    ("sound", "game", r"^(Snd|snd|Fmod|FfxFmod|FMOD|iMusic|Music_|Sfx_|Voice_)"),
    ("system", "game", r"^(Save|Trophy|TO[A-Z]|Cloud|Localization|SetUpDefault|CD|cdc|BigFile|fios|sce|hdd|get_hdd|"
                       r"Settings|Path_|File|AsyncLoad|Job|VFile|Fixup|MagicFile|AssetListReader|ClusterManager|"
                       r"ModelBuffer|Virtuos|Input|SyncManager|LoadingScreen|getPlatform|user_|Kernel_|Lang|Vbf_)"),
    ("pc-ui", "game", r"^(Menu|FFEscMenu|Iggy|Flash|Gui|RemapButton|MenuTexture)"),
    ("movie", "game", r"^(movie|Movie|PhyFMV)"),
    ("ps2-emu", "game", r"^(AsmEmu|FFXVu|KeVu|restoreVf|set[SZ]SFlag)"),
]
RULES = [(m, k, re.compile(p)) for m, k, p in MODULES]
KIND = {m: k for m, k, _ in MODULES}


def module_of(name):
    for m, _, rx in RULES:
        if rx.search(name):
            return m
    return None


def symbols():
    out = {}
    for line in (REPO / "symbols/ffx.sym").read_text(encoding="utf-8").splitlines():
        p = line.split("#", 1)[0].split()
        if len(p) == 3 and p[1] == "function":
            out[int(p[0], 16)] = p[2]
    return out


def hooks():
    """address -> 'replaced' | 'wrapped': the hook table (crates/ffx-hook/src/lib.rs)
    and the ported replacements (crates/ffx-hook/src/ported/*.rs)."""
    text = (REPO / "crates/ffx-hook/src/lib.rs").read_text(encoding="utf-8")
    out = {}
    for block in re.findall(r"Hook \{(.*?)\n\s*\},", text, re.S):
        addr = re.search(r"address:\s*0x([0-9a-f_]+)", block)
        if addr:
            wrapped = re.search(r"original:\s*Some", block)
            out[int(addr[1].replace("_", ""), 16)] = "wrapped" if wrapped else "replaced"
    for a in ported():
        out[a] = "replaced"
    return out


def classes():
    """address -> category from symbols/classes.tsv (empty when missing)."""
    path = REPO / "symbols/classes.tsv"
    out = {}
    if path.exists():
        for line in path.read_text(encoding="utf-8").splitlines():
            if line and not line.startswith("#"):
                a, c = line.split("\t")[:2]
                out[int(a, 16)] = c
    return out


def ported():
    """name by address of every `ported!` entry."""
    out = {}
    for f in sorted((REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
        for name, addr in re.findall(r"^\s*(\w+) @ 0x([0-9a-f_]+) =>", f.read_text(encoding="utf-8"), re.M):
            out[int(addr.replace("_", ""), 16)] = name
    return out


def functions():
    path = WORK / "decomp/functions.tsv"
    if not path.exists():
        sys.exit(f"{path} missing: run tools/ghidra/dump.sh")
    rows = []
    with open(path, encoding="utf-8") as f:
        next(f)
        for line in f:
            a, name, size, _ = line.rstrip("\n").split("\t")
            rows.append((int(a, 16), name, int(size)))
    return rows


def placement(rows, names):
    """Function address -> module, for every function of the dump."""
    # Ghidra's own names (library signatures) place a function in a module
    # but don't count as named by us.
    placed = {a: module_of(names.get(a) or n) for a, n, _ in rows}
    known = sorted((a, m) for a, m in placed.items() if m)
    keys = [a for a, _ in known]

    def guess(a):
        i = bisect.bisect(keys, a)
        left = known[i - 1] if i > 0 else None
        right = known[i] if i < len(known) else None
        if left and right:
            if left[1] == right[1]:
                return left[1]
            return left[1] if a - left[0] <= right[0] - a else right[1]
        return (left or right)[1]

    return {a: m or guess(a) for a, m in placed.items()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--unknown", action="store_true")
    args = ap.parse_args()

    names = symbols()
    hooked = hooks()
    rows = functions()

    if args.unknown:
        for a, n in sorted(names.items()):
            if not module_of(n):
                print(f"0x{a:08x} {n}")
        return
    module = placement(rows, names)
    category = classes()

    stats = defaultdict(lambda: defaultdict(int))
    for a, _, size in rows:
        name = names.get(a)
        m = module[a]
        s = stats[m]
        s["functions"] += 1
        s["bytes"] += size
        if name:
            s["named"] += 1
            s["named_bytes"] += size
        state = hooked.get(a)
        if state:
            s[state] += 1
            s[state + "_bytes"] += size
        if category.get(a) == "logic":
            s["logic"] += 1
            s["logic_bytes"] += size
            if state != "replaced":
                s["to_port"] += 1
                s["to_port_bytes"] += size

    def total(kind):
        t = defaultdict(int)
        for m, s in stats.items():
            if KIND[m] == kind:
                for k, v in s.items():
                    t[k] += v
        return t

    game, lib = total("game"), total("library")
    if args.json:
        print(json.dumps({"modules": stats, "game": game, "library": lib}, indent=1))
        return

    def pct(a, b):
        return f"{100 * a / b:5.1f}%" if b else "    -"

    print(f"{'module':10} {'kind':8} {'funcs':>6} {'KB':>6} {'named':>7} {'by size':>7} "
          f"{'replaced':>8} {'by size':>7} {'wrapped':>7} {'logic KB':>8} {'to port':>7} {'KB':>5}")
    order = sorted(stats, key=lambda m: (KIND[m] != "game", -stats[m]["bytes"]))
    for m in order + ["GAME", "LIBRARY"]:
        s = game if m == "GAME" else lib if m == "LIBRARY" else stats[m]
        kind = "" if m in ("GAME", "LIBRARY") else KIND[m]
        if m == "GAME":
            print("-" * 98)
        print(f"{m:10} {kind:8} {s['functions']:6} {s['bytes'] / 1024:6.0f} "
              f"{pct(s['named'], s['functions'])} {pct(s['named_bytes'], s['bytes'])} "
              f"{s['replaced']:8} {pct(s['replaced_bytes'], s['bytes'])} {s['wrapped']:7}"
              + (f" {s['logic_bytes'] / 1024:8.0f} {s['to_port']:7} {s['to_port_bytes'] / 1024:5.0f}"
                 if kind != "library" else ""))


if __name__ == "__main__":
    main()
