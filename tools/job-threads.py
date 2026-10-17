#!/usr/bin/env python3
"""Which hooked ports the game runs off its main thread.

FFX.exe runs work on other threads: job-scheduler jobs (JobWorker threads,
several at once) and a few dedicated threads (vsync emulation, loaders,
sound, rendering). A port reached from one of them must be reentrant: no
process-wide scratch, and nothing it shares with another thread without the
original's locking (docs/hooking.md, "Threads").

This walks the decompilation dump's direct calls from each thread entry and
prints the hooked ports (`Name @ 0x...` in crates/ffx-hook/src/ported/*.rs)
it reaches. Calls through pointers are not followed, so the lists are lower
bounds.

    python3 -I tools/job-threads.py [--decomp DIR] [--all] [entry ...]

Without entries it walks the known ones (ENTRIES). --all prints every
reached port for the broad entries too (otherwise only their count).
"""

import argparse
import collections
import glob
import os
import re

# (entry, thread, how the game starts it, broad: reaches most of the game)
ENTRIES = [
    (0x00833530, "JobWorker", "Ch_CalcMiddle_Batch, queued by Ch_CalcMain and "
     "Ch_CalcMiddle through 0x00a44b90, one job per ~200 elements", False),
    (0x0082E390, "JobWorker", "Ch_Cloth_PerCharacter (Apply_Envelope, "
     "Sg_ClothStep, Sg_ClothPlaceAnchors), queued by Ch_DrawMain through "
     "0x00a44b40", False),
    (0x0071ED10, "JobWorker", "JobSchedule_pppPart_fillBuffer (vfx draw "
     "fills), queued by JobSchedule_pppPart_fillBuffer_Kick (updateFFX)", False),
    # eiAbmDraw's four jobs (0x00a8ed00..0x00a8ed60 queue the stubs
    # 0x00a53450, 0x00a534b0, 0x00a534c0, 0x00a53510); their direct callees:
    (0x006390D0, "JobWorker", "eiAbmDraw job (sphere grid draw)", False),
    (0x00A4B4B0, "JobWorker", "eiAbmDraw job (sphere grid draw)", False),
    (0x00A50290, "JobWorker", "eiAbmDraw job (sphere grid draw)", False),
    (0x00A4C8D0, "JobWorker", "eiAbmDraw job (sphere grid draw)", False),
    (0x00A50ED0, "JobWorker", "eiAbmDraw job (sphere grid draw)", False),
    (0x00A51560, "JobWorker", "eiAbmDraw job (sphere grid draw)", False),
    (0x00A51340, "JobWorker", "eiAbmDraw job (sphere grid draw)", False),
    (0x00636410, "JobWorker", "asyncFFXInitialize, queued by the loader "
     "through 0x00633670 (runs beside the main thread while loading)", True),
    # VsyncHandler (0x00887d60) calls g_TkSyncVCallbackPtrs[0..2] through
    # pointers; the one registered callback (TkSetSyncVCallbackPtr(2, ...)):
    (0x0086CA20, "VsyncEmulationThread (32 KB stack)", "the vsync callback "
     "VsyncHandler calls (TkSetVsyncHandler -> createVsyncEmuThread)", False),
    (0x00616010, "render thread", "Phyre_PRendering_PRenderThread_threadEntry",
     True),
    (0x00691360, "AsyncLoadManager (32 KB stack)", "AsyncLoadManager_createThread",
     False),
    (0x0068D740, "AsyncControllerTaskManager (32 KB stack)",
     "AsyncControllerTaskManager constructor", False),
    (0x006BD1E0, "Read command (32 KB stack)", "0x006bca90", False),
    # the FmodManager thread body (0x006fa1b0) loops over these two:
    (0x006FA700, "FmodManager (64 KB stack)", "FmodManager_createThread", False),
    (0x006FA7E0, "FmodManager (64 KB stack)", "FmodManager_createThread", False),
]


def load(decomp):
    body = {}
    for f in glob.glob(os.path.join(decomp, "code", "*.c")):
        cur, buf = None, []
        with open(f, errors="replace") as fh:
            for line in fh:
                m = re.match(r"// ==== 0x([0-9a-f]+) ====", line)
                if m:
                    if cur is not None:
                        body[cur] = "".join(buf)
                    cur, buf = int(m.group(1), 16), []
                    continue
                buf.append(line)
        if cur is not None:
            body[cur] = "".join(buf)
    names = {}
    for a, b in body.items():
        m = re.search(r"^[A-Za-z_][\w \*]*?\b([A-Za-z_]\w*)\s*\(", b, re.M)
        if m:
            names.setdefault(m.group(1), a)
    return body, names


def hooked_ports(repo):
    out = {}
    for f in glob.glob(os.path.join(repo, "crates/ffx-hook/src/ported/*.rs")):
        with open(f) as fh:
            text = fh.read()
        for m in re.finditer(r"^\s*(\w+) @ 0x([0-9a-f_]+) =>", text, re.M):
            out[int(m.group(2).replace("_", ""), 16)] = (m.group(1), os.path.basename(f))
    return out


def callees(body, names, a):
    out = set()
    text = body.get(a, "")
    for n in re.findall(r"\b([A-Za-z_]\w*)\s*\(", text):
        m = re.fullmatch(r"(?:thunk_)?FUN_([0-9a-f]{8})", n)
        t = int(m.group(1), 16) if m else names.get(n)
        if t is not None and t != a:
            out.add(t)
    return out


def reach(body, names, root):
    seen, q = {root}, collections.deque([root])
    while q:
        for t in callees(body, names, q.popleft()):
            if t not in seen:
                seen.add(t)
                q.append(t)
    return seen


def main():
    repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--decomp", default=os.path.expanduser("~/ffx-work/decomp"))
    ap.add_argument("--all", action="store_true")
    ap.add_argument("entry", nargs="*")
    args = ap.parse_args()
    body, names = load(args.decomp)
    hooked = hooked_ports(repo)
    entries = ENTRIES
    if args.entry:
        entries = [(int(e, 16), "?", "given", False) for e in args.entry]
    for root, thread, how, broad in entries:
        seen = reach(body, names, root)
        hits = sorted((hooked[a][1], hooked[a][0], a) for a in seen if a in hooked)
        print(f"== 0x{root:08x} [{thread}] {how}: {len(seen)} functions, "
              f"{len(hits)} hooked")
        if broad and not args.all:
            continue
        for f, n, a in hits:
            print(f"   {f:12} {n} 0x{a:08x}")


if __name__ == "__main__":
    main()
