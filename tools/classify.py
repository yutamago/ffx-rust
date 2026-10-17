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

Runtime coverage: the addresses in symbols/coverage.txt (tools/coverage.py,
from ffx-hook's coverage mode) ran in a session; they count as roots, and
one no static reference reaches is logic (debug when it looks like debug).

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
COVERAGE = REPO / "symbols/coverage.txt"
WORK = progress.WORK
BASE = 0x400000
CATS = ("logic", "glue", "platform", "debug", "dead")

# verdicts from porters who read the function and found no rule matched it
# (address: (category, evidence)); applied last, over every rule
REVIEWED = {
    0x007eb760: ("glue", "op_gs1_init: fixed GS register-setup packets (effects round 13)"),
    0x00775330: ("glue", "builds a GS packet (system round 19)"),
    0x0082bb90: ("glue", "builds a GS sprite packet (main round 5)"),
    0x008432e0: ("glue", "waits on the GS register flag, then sets a register (main round 5)"),
    0x00a5e7b0: ("glue", "GS packet setup (effects round 14)"),
    0x0082baf0: ("glue", "writes GS packets (chara round 11)"),
    0x00a3b590: ("platform", "Direct3D device vtable calls only (effects round 16)"),
    0x00a3c280: ("platform", "sets Direct3D device state through its vtable (effects round 16)"),
    0x008217d0: ("glue", "builds the GS display and draw environments (main round 8)"),
    0x0082ce90: ("glue", "sg_AccSetCurrent: GS packet capturing the frame (main round 8)"),
    0x0082be20: ("glue", "Sg_AccSetFrameChr: GS packet writer (main round 8)"),
    0x0082be80: ("glue", "Sg_AccDrawBG: GS packet writer (main round 8)"),
    0x00843360: ("glue", "Sg_MFIFOSync: PS2 DMA (main round 8)"),
    0x00829c00: ("glue", "forwards its arguments to the renderer at 0x6440e0, writes no globals (main round 8)"),
    0x00943e30: ("platform", "CRT small-block free (main round 8)"),
    0x0083f090: ("platform", "Sg_FreeA (main round 8)"),
    0x0083b1d0: ("platform", "Sg_PcRead: file read (main round 8)"),
    0x0083c200: ("platform", "Sg_Mem heap setup (main round 8)"),
    0x007ff670: ("debug", "op_loop_end_ffx: jumps to the debug routine 0x00a5bd90 under g_OpDebugMode (effects round 17)"),
    0x0082a1c0: ("platform", "Ch_LoadHdRenderData: builds the path, loads the render data, starts an async task (chara round 12)"),
    0x0082a140: ("platform", "Ch_LoadClothFileTask: the async cloth file task (chara round 12)"),
    0x00851b20: ("debug", "debug code (main round 8)"),
    0x00843560: ("debug", "debug code (main round 8)"),
    0x008bedd0: ("glue", "TOMenuGetNameMes: glue (system round 21)"),
    0x008bee70: ("glue", "TOMenuGetStatusMes: glue (system round 21)"),
    0x008b7be0: ("glue", "TOGetFontWidthInterMode: glue (system round 21)"),
    0x008abb00: ("glue", "vtable call wrapper (system round 21)"),
    0x008af590: ("glue", "TOSetEnvMovieMessage: GS register write (system round 21)"),
    0x00885b00: ("debug", "AtelSetUpSourceDebug: line table for the script debugger (script rounds 11-15)"),
    0x006f29f0: ("glue", "thiscall trampoline: calls [ecx+0x10]([ecx+0x14]), returns 0 (script rounds 11-15)"),
    0x00858520: ("debug", "debug native 0x00f8 (script rounds 11-15)"),
    0x007a4a60: ("debug", "debug native 0x70b1 (script rounds 11-15)"),
    0x00878690: ("debug", "debug native 0xc00b (script rounds 11-15)"),
    0x00878650: ("debug", "debug native 0xc00c (script rounds 11-15)"),
    0x008788a0: ("debug", "debug native 0xc018 (script rounds 11-15)"),
    0x00878b00: ("debug", "debug native 0xc022 (script rounds 11-15)"),
    0x00878e10: ("debug", "debug native 0xc02a (script rounds 11-15)"),
    0x00878230: ("debug", "debug native 0xc056 (script rounds 11-15)"),
    0x008781f0: ("debug", "debug native 0xc057 (script rounds 11-15)"),
    0x00878330: ("debug", "debug native 0xc05b (script rounds 11-15)"),
    0x00757250: ("glue", "pppDrawFilterDes: only releases the renderer instance (graphicVFXDestroy) (effects round 19)"),
    0x00901cb0: ("glue", "wraps 0x8ed0a0 and TOCtrlPkt: packet emission (system round 22)"),
    0x00903a10: ("glue", "TOMkpShapeXY: packet emission (system round 22)"),
    0x00903b70: ("glue", "TOMkpShapeXYWHC2: packet emission (system round 22)"),
    0x006ef310: ("debug", "a single ret (no-op stub) (system round 22)"),
}

# (category, name regex, evidence, highest address or None); first match
# wins. Debug first: a debug window of the battle code is debug, not logic.
# The PC port's own code (PhyreScene, Iggy menus, FMOD, the loaders) sits
# below PC_END; above it the PS2 game code, where porters reuse words like
# Menu*, Save* or Kernel_* for game logic, so the PC-only rules stop there.
PC_END = 0x00760000
NAME_RULES = [
    ("debug", r"(Debug|debug|Dbg|dbg|DBG|Tweak|Deci2|_Gui_|^Ms_Gui|^gui_|^Gu_|^Gu[A-Z]|^rcui|Profil|"
              r"^Sg_Db|DebugFont|^eidprintf|^printf|Printf$|Dump|^Assert|"
              r"^SgGu|FpMem|host0|^TkPutDebug|DispDebug|^Sg_Id_coltest|^Gui_|^yiGui|^chr_anim_win|^FreeCamera)",
     "debug name", None),
    # scene state ffx-render needs as input, kept exactly ("original look
    # first"): camera view and projection, fog/ambient/background colours,
    # screen fades, mesh and skeleton matrices, character lighting, 2D-layer
    # and mesh visibility, the minimap and area-map camera. Only code that
    # passes data to PhyreEngine/D3D11 (render targets, shaders, buffers,
    # loading, drawing) stays glue.
    ("logic", r"^(PhyreScene_|graphic|cgraphic|Graphic)"
              r"(?!.*(Shader|Buffer|RenderTarget|Load(?!ed)|Unload|Create|Destroy|Initiali|Draw(?!Focus|Filter)|"
              r"Texture(?!VideoInfo)|Pipeline|Cluster|Remap))"
              r".*(Camera|FOV|AspectRatio|ViewProjection|Fog|Ambient|Background|ScreenFade|Fading|CrossFade|"
              r"Skeleton|WorldMatrix|Matrices|Lighting|LightDir|ShadeColor|ShadeCoeff|2DLayer|2DHideLayer|"
              r"2DShowLayer|2DStartLayer|2DSceneNum|Cut2DScene|3DScene|NonUI2DScene|Guidemap|Minimap|MiniMap|"
              r"SetVisible|MeshVisible|MeshSegment(Alpha|Color|Hide)|[Oo]bject(Set)?(Rotation|Translation)|"
              r"Project3DPoint|Screen(Get|Set)(Dtt)?Offset|LevelName|NameHash|KeyframeAnim|DttPageFlag|"
              r"UiShadowType|TextureVideoInfo|Magic(Get|Set)(ToBeDelete)?Path|(Current|ToBeDelete)MagicPath|"
              r"PushTip|AllSceneAssetsLoaded|HasRunDraw|Focus|BattleMap|Has[23]dMap|NowUIType|MiniMapType|"
              r"FadeCenter)",
     "scene state the renderer needs (original look first)", PC_END),
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
                 r"i?(Sleep|Wakeup)Thread$|i?(Signal|Wait|Create|Delete|Poll)Sema$|Throw[A-Za-z]*Exception|CxxThrow|str_replace$)",
     "platform name (files, CD/DVD emulation, allocators, threads, std::, exceptions)", None),
    ("platform", r"^(Input|input|Virtuos|virtuos|Async|JobSchedule(?!_ppp)|SaveData|Save[A-Z]|getPlatform|"
                 r"Game_InitFileSystem|FFXApplication|ResetToLauncher|Fixup|AssetListReader|MagicFile|Lang|"
                 r"Cloud|getIsConsoleMode|PArray|Alloc|Malloc|SetUpDefault|VbfStream|Vbf)",
     "platform name (PC port: input, saves, loaders, app lifecycle)", PC_END),
]
NAME_RULES = [(c, re.compile(p), e, hi) for c, p, e, hi in NAME_RULES]
DEBUG_NAME = NAME_RULES[0][1]
# the PS2 SDK stubs the PC build keeps (sce*, the Sif/IOP calls) run: they are
# platform by name, not dead

# GS/DMA packet building: kicks and the helpers that append to a packet
PACKET_API = re.compile(r"^(sceDma|sceGs|rcPacketData|TOAdp|TOGs[A-Z]|TOCtrlPkt|TODrawKick|"
                        r"TO(Open|Close)Pktbuff|TOSetPktBuffAdrs|Gu_Set|Gu_Packet|yiPk|Sg_Pk|rcbg(Set|Restore)Reg$|"
                        # the menu/battle 2D shape writers append GIF primitives to the packet
                        r"TOMkp|TODraw|TOMakePkt|"
                        # the PC port's stand-in for those GIF primitives
                        r"graphicDrawUI|"
                        # own names of packet builders found by reading
                        r"GsPkt_)")
# the appending part of the packet API (no DMA kicks)
APPEND_API = re.compile(r"^(TOAdp|TOGs[A-Z]|TOMkp|TODraw(?!Kick)|TOMakePkt|graphicDrawUI|GsPkt_|rcPacketData)")
# read-only helpers of the 2D shape writers: the menu scissor clip (writes
# only through its pointer arguments) and the shape texture lookup (its
# cache is a texture name, not game state)
DRAW_HELPER = re.compile(r"^(BeginScissor_|TOGetShapTextureName$|TOGetImageWH$|TOCopyWindowLayout)")
# named read-only helpers a packet writer may call (no global writes either)
GETTER = re.compile(r"(Get|[Ii]s[A-Z0-9]|get_ptr32|Calc|ScaleMatrix|PersMatrix|Copy16|CopyQword|ToFixed|Mul[0-9A-Z]|Apply)")
# the PC build's PS2 SDK stand-ins (sce*, FlushCache, threads, semaphores)
PS2_STUB = re.compile(r"^(sce[A-Z]|FlushCache$|i?(Sleep|Wakeup)Thread$|i?(Signal|Wait|Create|Delete|Poll)Sema$|"
                      r"FFXVpu0Reset|SetDebugHandler)")
# what copies of std:: and PhyreEngine container code call: allocators,
# trees, free lists, vectors, static-object registration
CONTAINER_API = re.compile(r"^(Phyre_(PRedBlackTree|PFreeList|PhyreFree|PhyreMalloc|PString|PArray|PSharray)|"
                           r"operator_(new|delete)|user_(malloc|free|aligned)|UserHeap|eh_vector_|atexit|onexit|"
                           r"std_|Std|PArray|Array_Reserve)")
# calls such code may make that are no evidence of it by themselves
CONTAINER_OK = re.compile(r"^(memcpy|memmove|memset|crt_mem|_Xlength|_Xout|_Xbad|security_check_cookie|"
                          r"CriticalSection|strlen)")
# VU0 emulation: scratch registers, not game state
VU_SCRATCH = re.compile(r"^(FFXVu0|AsmEmu|restoreVf00|set[SZ]SFlag|cdc_|rcVectorToFixed|rcCopyQword)")

GLUE_IMPORTS = re.compile(r"^_?Iggy|IggyGDraw")
PLATFORM_IMPORTS = re.compile(
    r"^(Steam|CreateFile|ReadFile|WriteFile|CloseHandle|SetFilePointer|SetEndOfFile|GetFile|CreateDirectory|_wfopen|"
    r"CreateThread|_beginthreadex|ResumeThread|SuspendThread|TerminateThread|ExitThread|SetThreadPriority|"
    r"(Enter|Leave|Delete|Initialize)CriticalSection|CreateMutex|ReleaseMutex|CreateSemaphore|"
    r"ReleaseSemaphore|CreateEvent|SetEvent|ResetEvent|WaitForSingleObject|Sleep|Virtual(Alloc|Free|Query)|"
    r"Heap|SHGetFolderPath|CreateWindow|RegisterClass|ShowWindow|SetWindow|GetWindow|AdjustWindowRect|"
    r"PeekMessage|DispatchMessage|TranslateMessage|DefWindowProc|ClipCursor|SetCursorPos|GetCursorPos|"
    r"ShowCursor|ChangeDisplaySettings|GetSystemMetrics|RegisterDeviceNotification|LoadLibrary|"
    r"GetProcAddress|FreeLibrary|Crypt|CoInitialize|CoCreateInstance|ShellExecute|GetModule|"
    r"QueryPerformance|timeGetTime|GetTickCount|SetThreadExecutionState|GetUserName|Interlocked)")
DEBUG_IMPORTS = re.compile(r"^(AllocConsole|IsDebuggerPresent|DebugBreak|SetConsoleScreenBufferSize|"
                           r"GetConsoleScreenBufferInfo|OutputDebugString)$")
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


def branches(exe):
    """Direct call/jmp/jcc instructions from objdump's disassembly of .text:
    (source, target, is_call). Ghidra's call sites leave out tail jumps,
    and calls into code Ghidra never made a function."""
    import subprocess
    out = subprocess.run(["objdump", "-d", "-M", "intel", "--no-show-raw-insn", "-j", ".text", exe],
                         capture_output=True, text=True, check=True).stdout
    found = []
    for line in out.splitlines():
        m = BRANCH.match(line)
        if m:
            found.append((int(m[1], 16), int(m[3], 16), m[2] == "call"))
    return found


def code_graph(exe, secs, starts, ghidra_calls):
    """Nodes: Ghidra's functions plus the code they miss that something
    calls, or jumps to from another function (thunks, tail-call targets).
    Returns (nodes, edges {node: targets}, data roots)."""
    lo, hi = min(starts), max(starts)
    br = [x for x in branches(exe) if lo <= x[1] <= hi]

    def ghidra_holder(x):
        return starts[bisect.bisect_right(starts, x) - 1]

    known = set(starts)
    extra = {t for src, t, call in br if t not in known and (call or ghidra_holder(t) != ghidra_holder(src))}
    nodes = sorted(known | extra)

    def holder(x):
        i = bisect.bisect_right(nodes, x) - 1
        return nodes[i] if i >= 0 else None

    edges = defaultdict(set)
    for a, cs in ghidra_calls.items():
        edges[a] |= cs
    for src, t, _ in br:
        h, th = holder(src), holder(t)
        if h is not None and th is not None and th != h:
            edges[h].add(t if t in extra or t in known else th)
    # code Ghidra counted as part of the function before it may fall into it
    for i, t in enumerate(nodes):
        if t in extra and i:
            edges[nodes[i - 1]].add(t)
    direct = {a: set(ts) for a, ts in edges.items()}
    roots, code_edges = scan_pointers(secs, nodes)
    for h, ts in code_edges.items():
        edges[h] |= ts
    return nodes, edges, roots, direct


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(REPO / "symbols/classes.tsv"))
    ap.add_argument("--exe", default=str(pathlib.Path(os.environ.get("FFX_GAME_DIR", "~/steam-games/FFX-X2-HD"))
                                         .expanduser() / "FFX.exe"))
    ap.add_argument("--summary", action="store_true", help="print the table only")
    ap.add_argument("--explain", type=lambda x: int(x, 16), action="append", default=[],
                    help="print why this address fails the packet-writer test")
    args = ap.parse_args()

    names = progress.symbols()
    # auto-port shape names (SetFields_*, Compare_*, ...) say what the code
    # does, not whose it is: the propagation rules treat them as unnamed
    shape = set()
    for line in (REPO / "symbols/ffx.sym").read_text(encoding="utf-8").splitlines():
        if "# auto-port shape" in line:
            shape.add(int(line.split()[0], 16))

    def meaningful(a):
        return bool(names.get(a)) and a not in shape
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
    nodes, edges, roots, direct = code_graph(args.exe, secs, starts, {a: set(c) for a, c in callees.items()})
    roots.add(entry)
    for a, ts in edges.items():
        for t in ts:
            if t != a:
                callees[a].add(t)
                callers[t].add(a)
    node_set = set(nodes)

    def reach_from(rs):
        seen = set()
        todo = deque(r for r in rs if r in node_set)
        while todo:
            a = todo.popleft()
            if a not in seen:
                seen.add(a)
                todo.extend(c for c in callees[a] if c not in seen)
        return seen

    static_reach = reach_from(roots)
    # functions seen running (tools/coverage.py, ffx-hook's coverage mode)
    # are roots too: something reached them that the static scan missed
    ran = set()
    if COVERAGE.exists():
        ran = {int(x, 16) for x in COVERAGE.read_text(encoding="utf-8").split()}
    reach = reach_from(roots | ran)

    phyre_lib = {a for a in starts if module.get(a) == "phyre"}

    # an unnamed function between two debug-named methods of one class (the
    # free debug camera's unnamed methods) is debug; callers outside the
    # game modules count as debug by the same name rules
    named = [a for a in starts if a in names]

    def bracket_debug(a):
        i = bisect.bisect_left(named, a)
        if a in names or i == 0 or i >= len(named):
            return None
        lo, hi = names[named[i - 1]], names[named[i]]
        if DEBUG_NAME.search(lo) and DEBUG_NAME.search(hi) and lo.split("_")[0] == hi.split("_")[0]:
            return lo, hi
        return None

    def debug_outside(c):
        n = names.get(c)
        return bool(DEBUG_NAME.search(n)) if n else bracket_debug(c) is not None

    writes = defaultdict(set)
    accessors = defaultdict(set)
    with open(WORK / "decomp/data_refs.tsv", encoding="utf-8") as f:
        next(f)
        for line in f:
            fn, at, acc = line.rstrip("\n").split("\t")[:3]
            fn, at = int(fn, 16), int(at, 16)
            if acc != "W":
                accessors[at].add(fn)
            else:
                writes[fn].add(at)
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
        if a in ran and a not in static_reach:
            if debugish or any(DEBUG_STRINGS.search(x) for x in strs):
                cls[a], why[a] = "debug", "ran in the coverage sessions (no static reference); debug name or strings"
            else:
                cls[a], why[a] = "logic", "ran in the coverage sessions (no static reference)"
            decided.add(a)
            continue
        if a not in reach:
            if debugish or any(DEBUG_STRINGS.search(s) for s in strs):
                cls[a], why[a] = "debug", "unreachable from the entry point and every pointer table; debug name or strings"
            else:
                cls[a], why[a] = "dead", "unreachable from the entry point and every pointer table"
            decided.add(a)
            continue
        r = name_rule(a, n) if n else None
        if r:
            cls[a], why[a] = r[0], f"{r[1]}: {n}"
            decided.add(a)
            continue
        i = bisect.bisect_left(named, a)
        if not n and 0 < i < len(named) and PS2_STUB.search(names[named[i - 1]]) \
                and PS2_STUB.search(names[named[i]]):
            cls[a], why[a] = "platform", (f"unnamed, in the PS2 SDK stub file between {names[named[i - 1]]} "
                                          f"and {names[named[i]]}")
            decided.add(a)
            continue
        br = bracket_debug(a)
        if br:
            cls[a], why[a] = "debug", f"unnamed, between the debug methods {br[0]} and {br[1]}"
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

    # exception unwinding funclets (the CRT part of the image)
    unwind = {a for a in starts if module.get(a) == "crt" and a >= 0x00aa0000}

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
            pc_part = a < PC_END and not meaningful(a)
            if pc_part and all(c in unwind for c in cs):
                cls[a], why[a] = "platform", "only exception unwinding reaches it (a release or destructor)"
                changed = True
                continue
            # in the PC port's part of the image, library code (std:: copies
            # placed in crt, PhyreEngine) calling a function makes it platform
            kinds = {cls.get(c) or ("debug" if debug_outside(c) else
                                    "platform" if pc_part and c < PC_END else "library") for c in cs}
            # the PC part's own helpers (allocator internals, container
            # copies, renderer set-up) pass any number of hops when they call
            # no game logic above PC_END
            pc_tree = pc_part and not any(c >= PC_END and cls.get(c) == "logic" for c in game_callees(a))
            for k in ("debug", "glue", "platform"):
                # glue and platform pass one hop, to unnamed functions whose
                # callers a rule decided (a main loop that the PC update
                # function calls stays logic); debug-only trees go all the way
                if kinds == {k} and (k == "debug" or pc_tree or (module[a] in GLUE_SIDE and not meaningful(a)
                                                                 and all(c in decided for c in cs))):
                    cls[a] = k
                    why[a] = f"only called from {k} code ({names.get(next(iter(cs)), hex(next(iter(cs))))}" + \
                             (f" and {len(cs) - 1} more)" if len(cs) > 1 else ")")
                    changed = True
                    break

    # copies of std:: and PhyreEngine container code in the PC port's part
    # of the image are platform: unnamed (or no rule) functions whose every
    # call goes to allocators, trees, vectors, std:: copies or platform code
    # (at least one such call); static initialisers (atexit registration);
    # and destructors only exception unwinding reaches. Game code above
    # PC_END keeps its default.

    def container_ok(c, depth):
        """c is an allocator/container/platform call; a code block Ghidra
        has no function for (an import thunk, a tail jump) counts by what
        it calls in turn."""
        n = names.get(c, "")
        if CONTAINER_API.search(n) or CONTAINER_OK.search(n) or cls.get(c) == "platform" \
                or (c not in cls and module.get(c) == "crt"):
            return True
        if c in size or depth == 0:
            return False
        return all(container_ok(x, depth - 1) for x in direct.get(c, ()) if x != c)
    changed = True
    while changed:
        changed = False
        for a in sorted(game):
            if cls[a] != "logic" or a in decided or a >= PC_END or a not in reach:
                continue
            ce = {c for c in direct.get(a, ()) if c != a}
            cnames = [names.get(c, "") for c in ce]
            imp = imports[a]
            if not ce and not imp:
                continue
            if a in args.explain:
                print(f"0x{a:08x}: container test, calls " + ", ".join(f"{n or hex(c)}:{cls.get(c)}:{module.get(c)}" for c, n in zip(ce, cnames)) + f"; imports {sorted(imp)}", file=sys.stderr)
            fine = all(container_ok(c, 3) for c in ce)
            fine = fine and all(PLATFORM_IMPORTS.search(i) or CONTAINER_API.search(i) or CONTAINER_OK.search(i)
                                or i in ("free", "malloc") for i in imp)
            cs = [c for c in callers[a] if c != a]
            if "atexit" in imp or any(n == "atexit" for n in cnames):
                cls[a], why[a] = "platform", "static initialiser (registers its destructor with atexit)"
            elif fine and any(CONTAINER_API.search(n) for n in cnames):
                cls[a], why[a] = "platform", ("std::/PhyreEngine container code: only allocator, tree, "
                                              "vector or platform calls (" +
                                              ", ".join(sorted({n for n in cnames if n})[:3]) + ")")
            elif fine and cs and all(c in unwind for c in cs):
                cls[a], why[a] = "platform", "destructor only exception unwinding reaches"
            else:
                continue
            decided.add(a)
            changed = True

    # pure GS/DMA packet writers are glue (ffx-render replaces them): they
    # kick a packet or append to one through the packet API, or only packet
    # writers call them; they call nothing but the packet API, glue,
    # platform, VU0 scratch, read-only getters and other packet writers; and
    # no logic reads a global they write (writers, an init that resets the
    # buffer index, do not matter)
    pkt_globals = set()
    for x in game:
        if PACKET_API.search(names.get(x, "")):
            pkt_globals |= writes[x]

    def gwrites(x):
        return writes[x] - pkt_globals

    def other(c):
        n = names.get(c, "")
        return cls.get(c) in ("glue", "platform") or bool(PACKET_API.search(n) or VU_SCRATCH.search(n)
                                                         or DRAW_HELPER.search(n)
                                                         or (GETTER.search(n) and not gwrites(c))
                                                         or (size.get(c, 99) <= 16 and not gwrites(c)
                                                             and not direct.get(c)))

    for a in game:
        n = names.get(a, "")
        if cls[a] == "logic" and a not in decided and PACKET_API.search(n):
            cls[a] = "glue"
            why[a] = f"GS packet API ({n}): appends to or kicks a packet; replaced by ffx-render"
            decided.add(a)
    # rounds: a writer found in one round counts as a packet call in the
    # next (a draw helper that only calls TOAdp* through another writer)
    confirmed = set()
    while True:
        pk = {a for a in game if cls[a] == "logic" and a not in decided and a in reach
              and (not confirmed or any(c in confirmed for c in direct.get(a, ())))}
        for a in args.explain:
            if a not in pk:
                print(f"0x{a:08x}: not a candidate ({cls.get(a)}, {why.get(a)})", file=sys.stderr)
        changed = True
        while changed:
            changed = False
            for a in sorted(pk):
                # real calls only: immediates in packet code (GIF tags) can look
                # like function addresses
                ce = {c for c in direct.get(a, ()) if c in cls and c != a}
                if not all(c in pk or other(c) for c in ce):
                    if a in args.explain:
                        print(f"0x{a:08x}: calls " + ", ".join(names.get(c, hex(c)) for c in ce
                                                           if not (c in pk or other(c))), file=sys.stderr)
                    pk.discard(a)
                    changed = True
                    continue
                if not all(fn in pk or other(fn) or fn not in reach for g in gwrites(a) for fn in accessors[g]):
                    if a in args.explain:
                        print(f"0x{a:08x}: writes globals read by " + ", ".join(sorted(
                            {names.get(fn, hex(fn)) for g in gwrites(a) for fn in accessors[g]
                             if not (fn in pk or other(fn) or fn not in reach)})[:6]), file=sys.stderr)
                    pk.discard(a)
                    changed = True
                    continue
                kicks = any(PACKET_API.search(names.get(c, "")) or c in confirmed for c in ce)
                cs = [c for c in callers[a] if c != a and c in reach]
                if not kicks and not (cs and not confirmed and all(c in pk for c in cs)):
                    if a in args.explain:
                        print(f"0x{a:08x}: no packet API call and callers not all packet writers: "
                              + ", ".join(names.get(c, hex(c)) for c in cs if c not in pk), file=sys.stderr)
                    pk.discard(a)
                    changed = True
        for a in pk:
            api = sorted({names[c] for c in callees[a] if PACKET_API.search(names.get(c, ""))})
            via = sorted({names.get(c, hex(c)) for c in direct.get(a, ()) if c in confirmed})
            cls[a] = "glue"
            why[a] = ("GS/DMA packet writer (" + (", ".join(api[:3]) if api else
                                                  "calls the packet writers " + ", ".join(via[:3]) if via else
                                                  "only packet writers call it") +
                      "; writes no game state): replaced by ffx-render")
        new = pk - confirmed
        # only appenders carry over (TOAdp*, TOGs*, the shape writers), not
        # kicks: an assert that DMAs an error screen does not make its callers
        # packet code
        confirmed |= {a for a in pk if any(APPEND_API.search(names.get(c, "")) or c in confirmed
                                           for c in direct.get(a, ()))}
        if not new:
            break

    # a debug-sounding name the game logic depends on (a math helper of the
    # rc tools, an "is debug" flag the battle code reads) is logic when most
    # callers are logic and it calls no debug code itself
    changed = True
    while changed:
        changed = False
        for a in game:
            if cls[a] != "debug" or a not in reach or not names.get(a) or PRINT.search(names[a]):
                continue
            cs = [c for c in callers[a] if c != a and c in cls]
            lc = [c for c in cs if cls[c] == "logic"]
            if cs and 2 * len(lc) >= len(cs) and not any(cls[c] == "debug" for c in game_callees(a)):
                cls[a] = "logic"
                changed = True
                why[a] = ("debug-sounding name, but used by logic: " +
                          ", ".join(sorted(names.get(c, hex(c)) for c in lc)[:3]))
            elif a in ran and lc and not any(cls[c] == "debug" and c not in ran for c in game_callees(a)):
                # seen running, and logic calls it: whatever its name, the
                # game needs it (an rc matrix helper the movie camera uses)
                cls[a] = "logic"
                changed = True
                why[a] = ("ran in the coverage sessions, called by logic: " +
                          ", ".join(sorted(names.get(c, hex(c)) for c in lc)[:3]))

    # static initialisers (they register a destructor with atexit), anywhere
    for a in game:
        if cls[a] == "logic" and (any(names.get(c) == "atexit" for c in direct.get(a, ())) or "atexit" in imports[a]):
            cls[a], why[a] = "platform", "static initialiser (registers its destructor with atexit)"

    # thin wrappers in the PC part: a small function that writes no global
    # and only forwards to glue or platform code (PhyFMVPlayerManager calls,
    # a renderer delegation, a PString setter) is that boundary, even when
    # its name sounds like scene state: the state lives on the PC side and
    # ffx-render takes the call's arguments as its input
    changed = True
    while changed:
        changed = False
        for a in sorted(game):
            if cls[a] != "logic" or a not in reach or writes[a] or size.get(a, 99) > 64:
                continue
            ce = {c for c in direct.get(a, ()) if c != a and not CONTAINER_OK.search(names.get(c, ""))}
            ks = {cls.get(c) for c in ce}
            if ce and (ks <= {"glue", "platform"} if a < PC_END else ks == {"platform"}):
                cls[a] = "glue" if "glue" in ks else "platform"
                why[a] = ("thin wrapper: only forwards to " + cls[a] + " code (" +
                          ", ".join(sorted(names.get(c, hex(c)) for c in ce)[:3]) + ")")
                changed = True

    for a in args.explain:
        if a in cls:
            cs = sorted(c for c in callers[a] if c != a)
            print(f"0x{a:08x}: {cls[a]} ({why[a]}); module {module.get(a)}; callers " +
                  ", ".join(f"{names.get(c, hex(c))}:{cls.get(c)}{'' if c in decided else '?'}" for c in cs[:8]) +
                  "; calls " + ", ".join(f"{names.get(c, hex(c))}:{cls.get(c)}"
                                         for c in sorted(direct.get(a, ())) if c != a)[:300], file=sys.stderr)

    for a, (c, w) in REVIEWED.items():
        if a in cls:
            cls[a], why[a] = c, "reviewed: " + w

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
    print(f"reachable {len(reach & set(size))} of {len(size)} functions ({len(static_reach & set(size))} statically); {len(roots)} roots, {len(nodes) - len(size)} code blocks Ghidra has no function for", file=sys.stderr)


if __name__ == "__main__":
    main()
