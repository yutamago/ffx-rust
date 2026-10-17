#!/usr/bin/env python3
"""Sorts every game function (tools/progress.py's game modules) into what
the rewrite does with it, and writes symbols/classes.tsv.

  logic     decides game state or the original look: ported exactly
  glue      only drives a library that is replaced (PhyreEngine/D3D11,
            Iggy, FMOD, the .webm player): replaced in M4, not ported
  platform  files, allocators, threads/locks, settings, Steam, window and
            input setup, std:: copies: replaced by the host and Rust std
  debug     debug and development code: kept as mod-tool candidates
  dead      PS2-only paths that cannot run on PC, and code nothing reaches

Evidence, in order (the first rule that fires decides; one line of it goes
into the TSV):
  1. reachability: a call graph from the PE entry point and every function
     address found in the image outside code (vtables, native and overlay
     tables, callbacks in data, exception tables) and in code as an
     immediate (taken as an edge from the function holding it), with
     Ghidra's calls and objdump's direct call/jmp/jcc edges. A function
     no root reaches is dead (debug when it looks like debug code).
  2. names (ffx.sym), by prefix rules below.
  3. imports and library calls: Iggy imports and calls into PhyreEngine
     mark glue, Steam/window/file/thread imports platform, console and
     debugger imports debug.
  4. referenced strings: "host0:", "DEBUG" and the like turn an
     unreachable function into debug rather than dead.
  5. propagation: an unnamed or default function whose callers are all
     debug is debug; all glue (in a glue-side module), glue; all platform,
     platform.
  Anything left is logic: skipping something necessary breaks parity,
  porting something unnecessary only costs time.

Runtime coverage is not part of this yet (no hook-side coverage exists);
see docs/rewrite-plan.md.

  tools/classify.py [--out symbols/classes.tsv] [--exe FFX.exe] [--summary]
"""
import argparse
import bisect
import os
import pathlib
import re
import struct
import sys
from array import array
from collections import Counter, defaultdict, deque

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402

REPO = progress.REPO
WORK = progress.WORK
BASE = 0x400000
CATS = ("logic", "glue", "platform", "debug", "dead")

# (category, name regex, evidence, highest address or None); first match
# wins. Debug first: a debug window of the battle code is debug, not logic.
# The PC port's own code (PhyreScene, Iggy menus, FMOD, the loaders) sits
# below PC_END; above it the PS2 game code, where porters reuse words like
# Menu*, Save* or Kernel_* for game logic, so the PC-only rules stop there.
PC_END = 0x00760000
NAME_RULES = [
    ("debug", r"(Debug|debug|Dbg|dbg|DBG|Tweak|Deci2|_Gui_|^Ms_Gui|^gui_|^Gu_|^Gu[A-Z]|^rcui|Profil|"
              r"^Sg_Db|DebugFont|^eidprintf|^printf|Printf$|Dump|^Assert|"
              r"^SgGu|FpMem|host0|^TkPutDebug|DispDebug|^Sg_Id_coltest|^Gui_|^yiGui|^chr_anim_win)",
     "debug name", None),
    ("glue", r"^(Fmod|FfxFmod|FMOD|fmod|SfxBank|SfxEvent|iMusic|iSfx|iVoice|movie|Movie|PhyFMV|PVideoPlayback|"
             r"createU16Mesh)", "glue name (FMOD or the movie player)", None),
    ("glue", r"^(PhyreScene|graphic|cgraphic|Graphic|PostProcess|Shadow|Distortion|TextureAnimation|"
             r"DynGeo|RadialLine|RadialBlur|MotionBlur|BlurEffect|SpecularMap|BrokenScreenEffect|"
             r"Specular|Class[A-Z]\w*(Render|Geometry|Shader)|ClassChr|ClassLoaded|ClassCharacter|VFXDynamic|"
             r"VfxShader|InitVFXRenderData|InitMapRenderData|updateVFXRenderData|createVFXRenderData|"
             r"createMapRenderData|calculateVFX|MagicVFXHook|RenderDelegation|ChrResourceCache|"
             r"CharacterLighting|Phyre_CharacterLighting|AbmapManager|Abmap(State)?_|WaterWave|WaterDistortion|"
             r"PhyreCursor|Shader|Render_|IconPanel|Iggy|iggy|Flash|FFEscMenu|MenuManager|"
             r"MenuTexture|PageManager|PageHash|RemapButton|TextUtil|Gui[A-Z]|Menu(?!Draw_)[A-Z]|"
             r"eventcallback|ERRCHECK|printToOutput|mpeg|SyncManager|ModelBuffer|"
             r"ClusterManager|LoadingScreen|DepthCopy|Water|JobSchedule_ppp)",
     "glue name (PC port code driving PhyreEngine/D3D11 or Iggy)", PC_END),
    ("platform", r"^(fios|CD|Cd|dvd|hdd|get_hdd|BigFile|VFile|Vbf_|Path_|File|FileSys|Settings|user_|"
                 r"UserHeap|Sg_Mem|mspace|sa_|Sa_|task_|sce[A-Z]|FFXSif|Sif|rpc_|Trophy|trophy|achievement|"
                 r"Steam|std_|Std|Array_|CriticalSection|VirtualFree|Virtual|MemSystem|AsyncLoad|FlushCache$|"
                 r"i?(Sleep|Wakeup)Thread$|i?(Signal|Wait|Create|Delete|Poll)Sema$)",
     "platform name (files, CD/DVD emulation, allocators, threads, std::)", None),
    ("platform", r"^(Input|input|Virtuos|virtuos|Async|JobSchedule(?!_ppp)|SaveData|Save[A-Z]|getPlatform|"
                 r"Game_InitFileSystem|FFXApplication|ResetToLauncher|Fixup|AssetListReader|MagicFile|Lang|"
                 r"Cloud|getIsConsoleMode|PArray|Alloc|Malloc)",
     "platform name (PC port: input, saves, loaders, app lifecycle)", PC_END),
]
NAME_RULES = [(c, re.compile(p), e, hi) for c, p, e, hi in NAME_RULES]
DEBUG_NAME = NAME_RULES[0][1]
# sce* stubs the PC build keeps for the PS2 code: platform unless PS2-only
DEAD_FIRST = re.compile(r"^(sceMc|sceSif|SqsIop|Iop)")

GLUE_IMPORTS = re.compile(r"^_?Iggy|IggyGDraw")
PLATFORM_IMPORTS = re.compile(
    r"^(Steam|CreateFile|ReadFile|WriteFile|SetFilePointer|SetEndOfFile|GetFile|CreateDirectory|_wfopen|"
    r"CreateThread|_beginthreadex|ResumeThread|SuspendThread|TerminateThread|ExitThread|SetThreadPriority|"
    r"(Enter|Leave|Delete|Initialize)CriticalSection|CreateMutex|ReleaseMutex|CreateSemaphore|"
    r"ReleaseSemaphore|CreateEvent|SetEvent|ResetEvent|WaitForSingleObject|Sleep|Virtual(Alloc|Free|Query)|"
    r"Heap|SHGetFolderPath|CreateWindow|RegisterClass|ShowWindow|SetWindow|GetWindow|AdjustWindowRect|"
    r"PeekMessage|DispatchMessage|TranslateMessage|DefWindowProc|ClipCursor|SetCursorPos|GetCursorPos|"
    r"ShowCursor|ChangeDisplaySettings|GetSystemMetrics|RegisterDeviceNotification|LoadLibrary|"
    r"GetProcAddress|FreeLibrary|Crypt|CoInitialize|CoCreateInstance|ShellExecute|GetModule|"
    r"QueryPerformance|timeGetTime|GetTickCount|SetThreadExecutionState|GetUserName|Interlocked)")
DEBUG_IMPORTS = re.compile(r"^(AllocConsole|IsDebuggerPresent|DebugBreak|SetConsoleScreenBufferSize|"
                           r"GetConsoleScreenBufferInfo|_wassert|OutputDebugString)$")
PRINT = re.compile(r"(dbgPrintf|^printf|Printf|^printf2)")
DEBUG_STRINGS = re.compile(r"host0:|DEBUG|[Dd]ebug|\bdump\b|Deci2|assert", re.I)
GLUE_SIDE = {"render", "pc-ui", "movie", "sound", "vfx", "system", "main"}
# where a direct PhyreEngine call makes an unnamed function glue (effect
# drawing in vfx draws the original look: logic)
GLUE_LIB_SIDE = {"render", "pc-ui", "movie", "sound", "system", "main"}
# PhyreEngine's allocator, containers and printf say nothing about glue
# outside the PC-only modules: game code allocates through them too
PHYRE_UTIL = re.compile(r"^Phyre_(PhyreFree|PhyreMalloc|PFreeList|PRedBlackTree|PhyrePrintf|PString)")
PC_SIDE = {"render", "pc-ui", "movie", "sound"}


def read_pe(path):
    data = pathlib.Path(path).read_bytes()
    pe = struct.unpack_from("<I", data, 0x3c)[0]
    nsec = struct.unpack_from("<H", data, pe + 6)[0]
    opt = struct.unpack_from("<H", data, pe + 20)[0]
    entry = struct.unpack_from("<I", data, pe + 24 + 16)[0] + BASE
    secs = []
    for i in range(nsec):
        o = pe + 24 + opt + 40 * i
        name = data[o:o + 8].rstrip(b"\0").decode("latin-1")
        vsize, va, rsize, raw = struct.unpack_from("<IIII", data, o + 8)
        chars = struct.unpack_from("<I", data, o + 36)[0]
        secs.append((name, va + BASE, data[raw:raw + min(rsize, vsize or rsize)], bool(chars & 0x20000000)))
    return entry, secs


def scan_pointers(secs, starts):
    """(data roots, code edges {holder: targets}) from every 4-byte value
    equal to a function start, at every byte offset."""
    fset = set(starts)
    lo, hi = min(starts), max(starts)
    roots, at = set(), []
    for name, va, blob, code in secs:
        for k in range(4):
            n = (len(blob) - k) // 4
            if n <= 0:
                continue
            words = array("I")
            words.frombytes(blob[k:k + 4 * n])
            for i, w in enumerate(words):
                if lo <= w <= hi and w in fset:
                    if code:
                        at.append((va + k + 4 * i, w))
                    else:
                        roots.add(w)
    edges = defaultdict(set)
    for where, w in at:
        i = bisect.bisect_right(starts, where) - 1
        if i >= 0:
            edges[starts[i]].add(w)
    return roots, edges


BRANCH = re.compile(r"^\s*([0-9a-f]+):\s+(call|jmp|j[a-z]+)\s+0x([0-9a-f]+)\s*$")


def branch_edges(exe, starts):
    """Direct call/jmp/jcc edges from objdump's disassembly: Ghidra's call
    sites leave out tail jumps into other functions."""
    import subprocess
    fset = set(starts)
    out = subprocess.run(["objdump", "-d", "-M", "intel", "--no-show-raw-insn", "-j", ".text", exe],
                         capture_output=True, text=True, check=True).stdout
    edges = defaultdict(set)
    for line in out.splitlines():
        m = BRANCH.match(line)
        if not m:
            continue
        t = int(m[3], 16)
        if t not in fset:
            continue
        src = int(m[1], 16)
        i = bisect.bisect_right(starts, src) - 1
        if i >= 0 and starts[i] != t:
            edges[starts[i]].add(t)
    return edges


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(REPO / "symbols/classes.tsv"))
    ap.add_argument("--exe", default=str(pathlib.Path(os.environ.get("FFX_GAME_DIR", "~/steam-games/FFX-X2-HD"))
                                         .expanduser() / "FFX.exe"))
    ap.add_argument("--summary", action="store_true", help="print the table only")
    args = ap.parse_args()

    names = progress.symbols()
    rows = progress.functions()
    module = progress.placement(rows, names)
    size = {a: s for a, _, s in rows}
    starts = sorted(size)
    game = {a for a in starts if progress.KIND.get(module.get(a)) == "game"}

    callees, callers, imports = defaultdict(set), defaultdict(set), defaultdict(set)
    with open(WORK / "decomp/calls.tsv", encoding="utf-8") as f:
        next(f)
        for line in f:
            a, c, cname = line.rstrip("\n").split("\t")
            a = int(a, 16)
            if "EXTERNAL" in c:
                imports[a].add(cname)
            else:
                c = int(c, 16)
                callees[a].add(c)
                callers[c].add(a)
    strings = defaultdict(list)
    with open(WORK / "decomp/strings.tsv", encoding="utf-8", errors="replace") as f:
        next(f)
        for line in f:
            p = line.rstrip("\n").split("\t", 2)
            if len(p) == 3:
                for x in p[1].split(","):
                    if x:
                        strings[int(x, 16)].append(p[2])

    entry, secs = read_pe(args.exe)
    roots, edges = scan_pointers(secs, starts)
    roots.add(entry)
    for extra in (edges, branch_edges(args.exe, starts)):
        for a, ts in extra.items():
            for t in ts:
                callees[a].add(t)
                callers[t].add(a)
    reach = set()
    todo = deque(r for r in roots if r in size)
    while todo:
        a = todo.popleft()
        if a in reach:
            continue
        reach.add(a)
        todo.extend(c for c in callees[a] if c not in reach)

    phyre_lib = {a for a in starts if module.get(a) == "phyre"}
    cls, why, decided = {}, {}, set()

    def name_rule(a, n):
        for c, rx, e, hi in NAME_RULES:
            if (hi is None or a < hi) and rx.search(n):
                return c, e
        return None

    for a in game:
        n = names.get(a)
        m = module[a]
        imp = imports[a]
        strs = strings[a]
        debugish = (n and DEBUG_NAME.search(n)) or any(DEBUG_IMPORTS.search(i) for i in imp)
        if a not in reach:
            if debugish or any(DEBUG_STRINGS.search(s) for s in strs):
                cls[a], why[a] = "debug", "unreachable from the entry point and every pointer table; debug name or strings"
            else:
                cls[a], why[a] = "dead", "unreachable from the entry point and every pointer table"
            decided.add(a)
            continue
        if n and DEAD_FIRST.search(n):
            cls[a], why[a] = "dead", f"PS2-only API ({n}): memory card or IOP"
            decided.add(a)
            continue
        r = name_rule(a, n) if n else None
        if r:
            cls[a], why[a] = r[0], f"{r[1]}: {n}"
            decided.add(a)
            continue
        dbg_imp = sorted(i for i in imp if DEBUG_IMPORTS.search(i))
        if dbg_imp:
            cls[a], why[a] = "debug", "imports " + ", ".join(dbg_imp)
            decided.add(a)
            continue
        glue_imp = sorted(i for i in imp if GLUE_IMPORTS.search(i))
        if glue_imp:
            cls[a], why[a] = "glue", "calls Iggy " + ", ".join(glue_imp[:3])
            decided.add(a)
            continue
        lib = [names[c] for c in callees[a] if c in phyre_lib and names.get(c, "").startswith("Phyre")]
        real = [x for x in lib if not PHYRE_UTIL.search(x)]
        if (real and m in GLUE_LIB_SIDE) or (lib and m in PC_SIDE):
            cls[a], why[a] = "glue", "calls PhyreEngine " + ", ".join(sorted(real or lib)[:3])
            decided.add(a)
            continue
        plat_imp = sorted(i for i in imp if PLATFORM_IMPORTS.search(i))
        if plat_imp and m in GLUE_SIDE and not n:
            cls[a], why[a] = "platform", "imports " + ", ".join(plat_imp[:3])
            decided.add(a)
            continue
        cls[a], why[a] = "logic", "default (no rule: ported to be safe)"

    def game_callees(a):
        return {c for c in callees[a] if c != a and c in cls and not PRINT.search(names.get(c, ""))}

    # propagation for functions no rule decided: an unnamed function that
    # only calls debug code (a debug window's handler, reached through a
    # table) is debug; one only debug/glue/platform code calls is that
    changed = True
    while changed:
        changed = False
        for a in game:
            if a in decided or cls[a] != "logic":
                continue
            ce = game_callees(a)
            if not names.get(a) and len(ce) >= 2 and all(cls[c] == "debug" for c in ce):
                cls[a] = "debug"
                why[a] = "only calls debug code (" + ", ".join(sorted(names.get(c, hex(c)) for c in ce)[:3]) + ")"
                changed = True
                continue
            cs = [c for c in callers[a] if c != a and c in reach]
            if not cs:
                continue
            kinds = {cls.get(c, "library") for c in cs}
            for k in ("debug", "glue", "platform"):
                # glue and platform pass one hop, to unnamed functions whose
                # callers a rule decided (a main loop that the PC update
                # function calls stays logic); debug-only trees go all the way
                if kinds == {k} and (k == "debug" or (module[a] in GLUE_SIDE and not names.get(a)
                                                      and all(c in decided for c in cs))):
                    cls[a] = k
                    why[a] = f"only called from {k} code ({names.get(next(iter(cs)), hex(next(iter(cs))))}" + \
                             (f" and {len(cs) - 1} more)" if len(cs) > 1 else ")")
                    changed = True
                    break

    # a debug-sounding name the game logic depends on (a math helper of the
    # rc tools, an "is debug" flag the battle code reads) is logic when most
    # callers are logic and it calls no debug code itself
    for a in game:
        if cls[a] != "debug" or a not in reach or not names.get(a):
            continue
        cs = [c for c in callers[a] if c != a and c in cls]
        lc = [c for c in cs if cls[c] == "logic"]
        if cs and 2 * len(lc) >= len(cs) and not any(cls[c] == "debug" for c in game_callees(a)):
            cls[a] = "logic"
            why[a] = ("debug-sounding name, but used by logic: " +
                      ", ".join(sorted(names.get(c, hex(c)) for c in lc)[:3]))

    stats = defaultdict(Counter)
    for a in game:
        stats[module[a]][cls[a]] += 1
        stats[module[a]][cls[a] + "_bytes"] += size[a]
    if not args.summary:
        with open(args.out, "w", encoding="utf-8") as f:
            f.write("# address\tcategory\tevidence (tools/classify.py; categories in docs/rewrite-plan.md)\n")
            for a in sorted(game):
                f.write(f"0x{a:08x}\t{cls[a]}\t{why[a]}\n")
    print(f"{'module':9}" + "".join(f"{c:>15}" for c in CATS))
    tot = Counter()
    for m in sorted(stats, key=lambda m: -sum(stats[m][c + '_bytes'] for c in CATS)):
        s = stats[m]
        tot.update(s)
        print(f"{m:9}" + "".join(f"{s[c]:6} {s[c + '_bytes'] / 1024:6.0f}K" for c in CATS))
    print(f"{'GAME':9}" + "".join(f"{tot[c]:6} {tot[c + '_bytes'] / 1024:6.0f}K" for c in CATS))
    print(f"reachable {len(reach)} of {len(size)} functions; {len(roots)} pointer roots", file=sys.stderr)


if __name__ == "__main__":
    main()
