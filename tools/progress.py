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
  stub      functions under five bytes (classes.tsv category `stub`: a bare
            ret, ret N, a constant or field return) that are too short for
            the hook's jmp; counted apart, never in to port

Sizes: the dump's, except where symbols/sizes.tsv has the length measured
in FFX.exe (the dump sizes some functions, most of them Atel natives, as
one byte). `--measure-sizes` regenerates it (FFX_GAME_DIR or the default
game dir): every function the dump sizes under five bytes, disassembled to
its last ret/jmp; run it after a new dump.

Usage: tools/progress.py [--json] [--unknown] [--measure-sizes]
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
    ("vpx", "library", r"^(vpx_|vp8_|vp9_)"),
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
# unnamed library code by address: libvpx's SIMD routines (the movie
# decoder's DSP functions that vpx_setup_rtcd_0040d2c0 installs), between
# TkMenuStopScene and the CRT
RANGES = [("vpx", 0x00A7C920, 0x00A8D87A)]
KIND = {m: k for m, k, _ in MODULES}


class WorkTree:
    """The files the counts come from, read from the checkout. tools/status.py
    passes a git commit's tree instead."""

    def read(self, rel):
        path = REPO / rel
        return path.read_text(encoding="utf-8") if path.exists() else None

    def ls(self, rel, suffix):
        return sorted(f"{rel}/{f.name}" for f in (REPO / rel).glob("*" + suffix))


WORKTREE = WorkTree()


def module_of(name):
    for m, _, rx in RULES:
        if rx.search(name):
            return m
    return None


def symbols(tree=WORKTREE):
    out = {}
    for line in tree.read("symbols/ffx.sym").splitlines():
        p = line.split("#", 1)[0].split()
        if len(p) == 3 and p[1] == "function":
            out[int(p[0], 16)] = p[2]
    return out


def hooks(tree=WORKTREE):
    """address -> 'replaced' | 'wrapped': the hook table (crates/ffx-hook/src/lib.rs)
    and the ported replacements (crates/ffx-hook/src/ported/*.rs)."""
    text = tree.read("crates/ffx-hook/src/lib.rs")
    out = {}
    for block in re.findall(r"Hook \{(.*?)\n\s*\},", text, re.S):
        addr = re.search(r"address:\s*0x([0-9a-f_]+)", block)
        if addr:
            wrapped = re.search(r"original:\s*Some", block)
            out[int(addr[1].replace("_", ""), 16)] = "wrapped" if wrapped else "replaced"
    for a in ported(tree):
        out[a] = "replaced"
    return out


def classes(tree=WORKTREE):
    """address -> category from symbols/classes.tsv (empty when missing)."""
    text = tree.read("symbols/classes.tsv")
    out = {}
    if text:
        for line in text.splitlines():
            if line and not line.startswith("#"):
                a, c = line.split("\t")[:2]
                out[int(a, 16)] = c
    return out


def ported(tree=WORKTREE):
    """name by address of every `ported!` entry."""
    out = {}
    for f in tree.ls("crates/ffx-hook/src/ported", ".rs"):
        for name, addr in re.findall(r"^\s*(\w+) @ 0x([0-9a-f_]+) =>", tree.read(f), re.M):
            out[int(addr.replace("_", ""), 16)] = name
    return out


def duplicate_hooks(repo=None):
    """Problem lines for functions hooked in two places: an address with two
    `ported!` entries (in one file or in two module tables), or ported and
    also hooked by hand in crates/ffx-hook/src (wrappers, mods, the
    harness). check-ported.py and verify-batch.py (before the build) use it."""
    repo = pathlib.Path(repo or REPO)
    hand = {}
    for f in sorted((repo / "crates/ffx-hook/src").glob("*.rs")):
        for name, addr in re.findall(r'name: "(\w+)",\s*address: 0x([0-9a-f_]+),', f.read_text(encoding="utf-8")):
            hand[int(addr.replace("_", ""), 16)] = f"{f.name}: {name}"
    seen, out = {}, []
    for f in sorted((repo / "crates/ffx-hook/src/ported").glob("*.rs")):
        for name, addr in re.findall(r"^\s*(\w+) @ 0x([0-9a-f_]+) =>", f.read_text(encoding="utf-8"), re.M):
            a = int(addr.replace("_", ""), 16)
            where = f"{f.name}: {name} @ {a:#010x}"
            if a in seen:
                out.append(f"{where}: also ported as {seen[a]}")
            if a in hand:
                out.append(f"{where}: also hooked by hand ({hand[a]})")
            seen.setdefault(a, f"{name} ({f.name})")
    return out


SIZES = "symbols/sizes.tsv"


def dump_functions():
    """(address, Ghidra name, Ghidra body size) of the dump, sizes as they are."""
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


def size_fixes(tree=WORKTREE):
    """address -> size measured in FFX.exe, where the dump's is wrong
    (symbols/sizes.tsv, `tools/progress.py --measure-sizes`)."""
    out = {}
    for line in (tree.read(SIZES) or "").splitlines():
        if line and not line.startswith("#"):
            a, s = line.split("\t")[:2]
            out[int(a, 16)] = int(s)
    return out


def functions():
    """The dump's functions with corrected sizes: Ghidra gives some
    functions (most of them Atel natives) a one-byte body although the code
    runs on to a `ret` 5-45 bytes later; symbols/sizes.tsv holds the length
    measured in FFX.exe for those. It also holds the functions of
    symbols/ffx.sym the dump has no start for (Ghidra left them as labels
    or never decoded them: most are Atel natives reached only through
    g_NativeTables), measured the same way; they are added here with an
    empty Ghidra name."""
    fix = size_fixes()
    rows = [(a, n, fix.get(a, s)) for a, n, s in dump_functions()]
    have = {a for a, _, _ in rows}
    rows += [(a, "", s) for a, s in fix.items() if a not in have]
    return sorted(rows)


_PE = {}


def jmp_thunk(exe, a, rows):
    """The target of the 5-byte `jmp rel32` at `a` in `exe`, when `a` is
    no dump function start nor inside a dump function's body (`rows`:
    address -> (name, size), from functions()) and the jump goes to a dump
    function start; else None. Ghidra folds such lone jump thunks into
    their callers or leaves them as labels (user_free_thunk 0x00687010,
    code_006f2740), so the dump has no function at them; the machine code
    confirms them, and a hook's 5-byte patch covers them exactly
    (check-ported, check-clobber)."""
    import struct
    key = str(exe)
    if key not in _PE:
        d = pathlib.Path(exe).read_bytes()
        pe = struct.unpack_from("<I", d, 0x3c)[0]
        nsec, opt = struct.unpack_from("<H", d, pe + 6)[0], struct.unpack_from("<H", d, pe + 20)[0]
        base = struct.unpack_from("<I", d, pe + 24 + 28)[0]
        secs = []
        for i in range(nsec):
            o = pe + 24 + opt + 40 * i
            vsize, rva, rsize, raw = struct.unpack_from("<IIII", d, o + 8)
            chars = struct.unpack_from("<I", d, o + 36)[0]
            secs.append((base + rva, base + rva + min(vsize, rsize), raw, chars))
        _PE[key] = (d, secs, sorted(rows))
    d, secs, starts = _PE[key]
    if a in rows:
        return None
    # executable code only (IMAGE_SCN_MEM_EXECUTE)
    sec = next((x for x in secs if x[0] <= a and a + 5 <= x[1] and x[3] & 0x20000000), None)
    if sec is None:
        return None
    off = sec[2] + a - sec[0]
    if d[off] != 0xE9:
        return None
    target = (a + 5 + struct.unpack_from("<i", d, off + 1)[0]) & 0xFFFFFFFF
    if target not in rows:
        return None
    i = bisect.bisect_right(starts, a) - 1
    if i >= 0 and starts[i] + rows[starts[i]][1] > a:
        return None  # inside a function's body: not a thunk of its own
    return target


# a body the dump sizes under this many bytes is measured (a hook needs 5)
MEASURE_BELOW = 5
JUMP = re.compile(r"^(j[a-z]+|loop[a-z]*)\s+0x([0-9a-f]+)$")


def measure(code, a, limit):
    """Length of the function at `a` from its disassembly `code` ({address:
    text}, up to `limit`): instructions in order until a ret or jmp that no
    earlier forward branch jumps past. None when the code stops first
    (int3, bad bytes, the limit) or flow leaves through an indirect jmp
    with code after it (a jump table), where the length is a guess."""
    reach = a
    addrs = sorted(code)
    for i, pc in enumerate(addrs):
        text = code[pc]
        nxt = addrs[i + 1] if i + 1 < len(addrs) else limit
        op = text.split()[0] if text else ""
        if op in ("int3", "(bad)") or text.startswith(".byte"):
            return None
        m = JUMP.match(text)
        if m:
            t = int(m[2], 16)
            if pc < t < limit:
                reach = max(reach, t)
        if op in ("ret", "jmp") and nxt > reach:
            if op == "jmp" and not m and nxt < limit and code.get(nxt, "").split()[:1] != ["int3"]:
                return None
            return nxt - a
    return None


def measure_sizes(exe):
    """Rows of symbols/sizes.tsv: every function the dump sizes under five
    bytes, measured in `exe`, where the measured length differs, and every
    symbols/ffx.sym function the dump has no start for (dump size 0)."""
    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
    import disasm_cache
    rows = sorted(dump_functions())
    starts = [a for a, _, _ in rows]
    cache = disasm_cache.for_exe(exe)
    cache.load()
    todo = []
    for i, (a, name, s) in enumerate(rows):
        if s < MEASURE_BELOW:
            limit = min(starts[i + 1] if i + 1 < len(rows) else a + 0x1000, a + 0x1000)
            todo.append((a, name, s, limit))
    # ffx.sym functions without a dump start, outside every dump body:
    # measured to the next start of either (dump size 0)
    have = set(starts)
    sym = symbols()
    named = sorted(a for a in sym if a not in have)
    every = sorted(have | set(named))
    for a in named:
        i = bisect.bisect_right(starts, a) - 1
        if i >= 0 and starts[i] + rows[i][2] > a:
            continue
        j = bisect.bisect_right(every, a)
        limit = min(every[j] if j < len(every) else a + 0x1000, a + 0x1000)
        todo.append((a, sym[a], 0, limit))
    cache.prefetch([(a, limit - a) for a, _, _, limit in todo])
    out, unsure = [], []
    for a, name, s, limit in todo:
        n = measure(cache.get(a, limit - a), a, limit)
        if n is None:
            unsure.append((a, name, s))
        elif n != s:
            out.append((a, n, s, name))
    cache.save()
    return out, unsure


def placement(rows, names):
    """Function address -> module, for every function of the dump."""
    # Ghidra's own names (library signatures) place a function in a module
    # but don't count as named by us.
    def by_range(a):
        return next((m for m, lo, hi in RANGES if lo <= a < hi), None)

    placed = {a: module_of(names.get(a) or n) or (None if a in names else by_range(a)) for a, n, _ in rows}
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


def count(rows, module, names, hooked, category):
    """module -> counters (the columns of the table) for the dump's rows."""
    stats = defaultdict(lambda: defaultdict(int))
    for a, _, size in rows:
        name = names.get(a)
        s = stats[module[a]]
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
        elif category.get(a) == "stub":
            s["stub"] += 1
            s["stub_bytes"] += size
    return stats


def total(stats, kind):
    t = defaultdict(int)
    for m, s in stats.items():
        if KIND[m] == kind:
            for k, v in s.items():
                t[k] += v
    return t


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--unknown", action="store_true")
    ap.add_argument("--measure-sizes", action="store_true")
    args = ap.parse_args()

    if args.measure_sizes:
        exe = pathlib.Path(os.environ.get("FFX_GAME_DIR", "~/steam-games/FFX-X2-HD")).expanduser() / "FFX.exe"
        fixed, unsure = measure_sizes(exe)
        # rows measured by hand (a dump body that runs into the next
        # function, 0x00432820) stay
        done = {a for a, _, _, _ in fixed} | {a for a, _, _ in unsure}
        dump = {a: s for a, _, s in dump_functions()}
        for line in (WORKTREE.read(SIZES) or "").splitlines():
            p = line.split("\t")
            if line and not line.startswith("#") and int(p[0], 16) not in done and dump.get(int(p[0], 16), 0) >= MEASURE_BELOW:
                fixed.append((int(p[0], 16), int(p[1]), int(p[2]), p[3] if len(p) > 3 else ""))
        fixed.sort()
        with open(REPO / SIZES, "w", encoding="utf-8") as f:
            f.write("# address\tsize\tdump size\tGhidra name (tools/progress.py --measure-sizes: functions the "
                    "dump sizes under five bytes or lacks (dump size 0), measured in FFX.exe to the last "
                    "ret/jmp)\n")
            for a, n, s, name in fixed:
                f.write(f"0x{a:08x}\t{n}\t{s}\t{name}\n")
        print(f"{len(fixed)} sizes corrected; {len(unsure)} left as the dump has them (no clear end):",
              file=sys.stderr)
        for a, name, s in unsure:
            print(f"  0x{a:08x} {name} ({s} bytes)", file=sys.stderr)
        return

    names = symbols()
    hooked = hooks()
    rows = functions()

    if args.unknown:
        for a, n in sorted(names.items()):
            if not module_of(n):
                print(f"0x{a:08x} {n}")
        return
    stats = count(rows, placement(rows, names), names, hooked, classes())
    game, lib = total(stats, "game"), total(stats, "library")
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
