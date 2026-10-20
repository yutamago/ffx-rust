#!/usr/bin/env python3
"""Logic functions that only build GS/VIF/DMA packets: the PS2 packet layers
the original-look renderer (M4) could replace instead of a port.

A logic function (symbols/classes.tsv) is listed when
  - every global it writes (data_refs.tsv) is packet state: the packet
    buffers and cursors named in symbols/ffx.sym (g_TOPktCursor,
    g_PppPacketPos, g_GuPacket*, g_ChrPacket*, g_YiPolyBuf, ...), every
    global a packet-API function (tools/classify.py's PACKET_API names)
    writes, or draw-private state: a global only listed functions, glue,
    dead and debug code touch (VU0/EE emulation scratch, a draw's colour
    shift);
  - everything it calls is a packet writer (the packet API, functions
    classify.py already made glue as packet writers, or functions listed
    here), other glue, a side-effect-free helper (no global writes, no
    stores through pointers, calls only such helpers, or a getter/VU0
    scratch name classify.py trusts) or a math/format CRT routine;
  - it makes no indirect call;
  - it touches packet state or calls a packet writer (otherwise it is a
    pure helper, not a packet layer).

Confidence, from the pseudocode ($FFX_WORK/decomp/code):
  high    every store through a pointer goes through a value derived from
          packet state (a cursor, a packet writer's result), into a local,
          or nowhere: the function writes nothing but packets
  medium  it also stores through a parameter or another pointer (often the
          packet pointer handed in, sometimes an out-parameter or an
          object): read it before treating it as packet code

This changes nothing: the user decides whether M4 replaces these layers
(docs/rewrite-plan.md); then classify.py would class them glue.

  tools/packet-layers.py [--out FILE.md] [--tsv FILE.tsv] [--explain 0xADDR]
"""
import argparse
import bisect
import os
import pathlib
import re
import sys
from collections import defaultdict

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import classify  # noqa: E402
import dupes  # noqa: E402
import progress  # noqa: E402

PACKET_LABEL = re.compile(r"(Pkt|Packet|PolyBuf|ChrStrip)(?!Strides|TagSizes)")
# the emulated EE/VU0 registers: scratch, not game state
SCRATCH_LABEL = re.compile(r"^g_(Ee_Gpr|Vu0_|Vu0Acc)")
CRT_OK = re.compile(r"^(sqrt|sqrtf|sin|cos|tan|atan|atan2|asin|acos|floor|ceil|fabs|fmod|pow|exp|log|_CI\w+|"
                    r"_ftol\w*|ftol\w*|sprintf|_sprintf|snprintf|vsprintf|strlen|strcpy|strcat|memcpy|memset|"
                    r"memmove|security_check_cookie|__security_check_cookie|_alloca_probe\w*|_chkstk|abs|labs|"
                    r"rand|crt_\w+)$")
LOCAL = re.compile(r"^&?\(?(local_|auStack|acStack|afStack|aiStack|auVar|abStack|uStack|fStack|iStack|stack0x)")
SECTION = re.compile(r"^// ==== 0x([0-9a-f]+) ====", re.M)
STORE = re.compile(r"^\s*([^=;{}]*?[\]\w)])\s*=(?!=)\s*(.*);\s*$")
ASSIGN = re.compile(r"^\s*(\w+)\s*=(?!=)\s*(.*);\s*$")
IDENT = re.compile(r"[A-Za-z_]\w*")
TYPES = {"undefined", "undefined1", "undefined2", "undefined4", "undefined8", "uint", "int", "short", "ushort",
         "char", "byte", "float", "double", "float10", "longlong", "ulonglong", "bool", "code", "void", "long",
         "ulong", "uchar", "word", "dword", "sbyte", "unaff_EBP", "unaff_ESI", "unaff_EDI", "in_EAX", "CONCAT31",
         "CONCAT22", "CONCAT44", "SUB41", "SUB42", "ZEXT14", "ZEXT24", "SEXT14", "SEXT24"}


def tsv(path):
    with open(path, encoding="utf-8") as f:
        next(f)
        for line in f:
            yield line.rstrip("\n").split("\t")


def pseudocode(want):
    """address -> pseudocode text, for the addresses in want."""
    out = {}
    for f in sorted((progress.WORK / "decomp/code").glob("*.c")):
        text = f.read_text(encoding="utf-8", errors="replace")
        marks = list(SECTION.finditer(text))
        for i, m in enumerate(marks):
            a = int(m[1], 16)
            if a in want:
                out[a] = text[m.end():marks[i + 1].start() if i + 1 < len(marks) else len(text)]
    return out


def base_of(lhs):
    """The variable a store's target is computed from."""
    for w in IDENT.findall(lhs):
        if w not in TYPES:
            return w
    return None


def stores(text, pkt_names, writer_names):
    """(kinds of pointer-store bases, uses an indirect call): kinds are
    'packet', 'local', 'global', 'param', 'other'."""
    tainted = set()
    lines = text.splitlines()
    for _ in range(4):
        for line in lines:
            m = ASSIGN.match(line)
            if m:
                rhs = set(IDENT.findall(m[2]))
                if rhs & pkt_names or rhs & tainted or rhs & writer_names:
                    tainted.add(m[1])
    kinds = defaultdict(list)
    for line in lines:
        m = STORE.match(line)
        if not m or ASSIGN.match(line):
            continue
        lhs = m[1].strip()
        if not any(ch in lhs for ch in "*[") and "->" not in lhs:
            continue
        b = base_of(lhs)
        if b is None:
            continue
        if LOCAL.match(lhs.lstrip("*(").lstrip()) or b.startswith(("local_", "auStack", "acStack", "stack0x")):
            k = "local"
        elif b in pkt_names or b in tainted:
            k = "packet"
        elif b.startswith(("DAT_", "g_", "PTR_")):
            k = "global"
        elif b.startswith("param_"):
            k = "param"
        else:
            k = "other"
        kinds[k].append(line.strip())
    indirect = "(code *)" in text or "(code **)" in text
    return kinds, indirect


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("--out", help="write the list as markdown here")
    ap.add_argument("--tsv", help="write the list as TSV here")
    ap.add_argument("--explain", action="append", default=[], help="why an address is or is not listed")
    ap.add_argument("--reasons", action="store_true", help="what keeps packet-touching logic off the list")
    args = ap.parse_args()
    explain = {int(x, 16) for x in args.explain}

    rows = progress.functions()
    names = progress.symbols()
    module = progress.placement(rows, names)
    cls = progress.classes()
    ported = progress.ported()
    size = {a: s for a, _, s in rows}
    dname = {a: n for a, n, _ in rows}
    label = {}
    for line in (progress.REPO / "symbols/ffx.sym").read_text(encoding="utf-8").splitlines():
        p = line.split("#", 1)[0].split()
        if len(p) == 3 and p[1] == "label":
            label[int(p[0], 16)] = p[2]
    evidence = {}
    for parts in tsv(progress.REPO / "symbols/classes.tsv"):
        if len(parts) >= 3 and not parts[0].startswith("#"):
            evidence[int(parts[0], 16)] = parts[2]

    # calls into code the dump has no function for: a `jmp rel32` thunk
    # stands for its target (FFX.exe's bytes, with FFX_GAME_DIR set)
    game = os.environ.get("FFX_GAME_DIR")
    exe = pathlib.Path(game).expanduser() / "FFX.exe" if game else None
    secs = dupes.read_pe(exe)[0] if exe and exe.exists() else []

    def through_thunk(c):
        for va, blob in secs:
            k = c - va
            if 0 <= k <= len(blob) - 5 and blob[k] == 0xE9:
                return (c + 5 + int.from_bytes(blob[k + 1:k + 5], "little", signed=True)) & 0xFFFFFFFF
        return c

    callees, callers, ext = defaultdict(set), defaultdict(set), defaultdict(set)
    for caller, callee, cname in tsv(progress.WORK / "decomp/calls.tsv"):
        a = int(caller, 16)
        if callee.startswith("0xEXTERNAL"):
            ext[a].add(cname)
        else:
            c = int(callee, 16)
            if c not in size:
                c = through_thunk(c)
                if c not in size:
                    dname.setdefault(c, cname)
            if c != a:
                callees[a].add(c)
                callers[c].add(a)
    writes, touches = defaultdict(set), defaultdict(set)
    addr_taken = defaultdict(set)
    for fn, addr, access, _ in tsv(progress.WORK / "decomp/data_refs.tsv"):
        a, g = int(fn, 16), int(addr, 16)
        touches[a].add(g)
        if access == "W":
            writes[a].add(g)
        elif access == "A":
            addr_taken[a].add(g)

    def nm(a):
        return names.get(a) or dname.get(a) or f"FUN_{a:08x}"

    api = {a for a in size if classify.PACKET_API.search(nm(a))}
    # packet state: named packet buffers/cursors (each up to the next label)
    # and whatever the packet API writes
    labs = sorted(label)
    pkt_ranges, scratch_ranges = [], []
    for i, a in enumerate(labs):
        end = min(labs[i + 1] if i + 1 < len(labs) else a + 4, a + 0x5000)
        if PACKET_LABEL.search(label[a]):
            pkt_ranges.append((a, end))
        elif SCRATCH_LABEL.search(label[a]):
            scratch_ranges.append((a, end))
    def within(ranges):
        keys = [a for a, _ in ranges]

        def test(g):
            i = bisect.bisect_right(keys, g) - 1
            return i >= 0 and g < ranges[i][1]
        return test
    is_pkt, scratch = within(pkt_ranges), within(scratch_ranges)
    api_writes = set().union(*(writes[a] for a in api)) if api else set()

    def pkt(g):
        return is_pkt(g) or g in api_writes
    pkt_names = {label[a] for a in label if pkt(a)}
    every_global = set().union(*touches.values())
    pkt_names |= {f"DAT_{g:08x}" for g in every_global if pkt(g)}
    writer_glue = {a for a in size if cls.get(a) == "glue" and ("packet" in evidence.get(a, "") or a in api)}
    writer_names = {nm(a) for a in writer_glue | api}

    code = pseudocode(set(size))

    # side-effect-free helpers: greatest fixpoint
    pure = set()
    for a in size:
        if writes[a]:
            continue
        text = code.get(a, "")
        kinds, indirect = stores(text, set(), set())
        if indirect or any(k != "local" for k in kinds) or not all(CRT_OK.search(e) for e in ext[a]):
            continue
        pure.add(a)
    changed = True
    while changed:
        changed = False
        for a in sorted(pure):
            if not all(c in pure or classify.VU_SCRATCH.search(nm(c)) for c in callees[a]):
                pure.discard(a)
                changed = True

    def helper(c):
        n = nm(c)
        return (c in pure or CRT_OK.search(n) or progress.KIND.get(module.get(c)) == "library" and CRT_OK.search(n) or classify.VU_SCRATCH.search(n) or classify.DRAW_HELPER.search(n)
                or (classify.GETTER.search(n) and not writes[c]))

    info = {}
    cand = {a for a in size if cls.get(a) == "logic" and progress.KIND.get(module.get(a)) == "game"}
    for a in sorted(cand):
        kinds, indirect = stores(code.get(a, ""), pkt_names, writer_names)
        info[a] = (kinds, indirect)
    layer = set(cand)
    reason = {}
    accessors = defaultdict(set)
    for f, gs in touches.items():
        for g in gs | writes[f] | addr_taken[f]:
            accessors[g].add(f)
    for f, gs in addr_taken.items():
        for g in gs:
            accessors[g].add(f)

    def private(g):
        """Draw-private state: only listed functions, glue and dead or debug
        code touch it (the VU0/EE emulation scratch, a draw's colour shift)."""
        return all(f in layer or f in api or cls.get(f) in ("glue", "dead", "debug") for f in accessors[g])
    changed = True
    while changed:
        changed = False
        for a in sorted(layer):
            kinds, indirect = info[a]
            why = None
            other = sorted(g for g in writes[a] if not pkt(g) and not scratch(g) and not private(g))
            if other:
                why = "writes " + ", ".join(label.get(g, f"{g:#x}") for g in other[:4])
            elif indirect:
                why = "indirect call"
            elif "global" in kinds:
                why = "stores through a global pointer: " + kinds["global"][0]
            else:
                bad = [c for c in callees[a] if not (c in layer or c in api or c in writer_glue
                                                     or cls.get(c) == "glue" or helper(c))]
                if bad:
                    why = "calls " + ", ".join(nm(c) for c in sorted(bad)[:4])
                elif not all(CRT_OK.search(e) for e in ext[a]):
                    why = "calls " + ", ".join(sorted(e for e in ext[a] if not CRT_OK.search(e)))
            if why:
                layer.discard(a)
                reason[a] = why
                changed = True
    # of the functions that pass, the packet layers: those that touch packet
    # state or call a packet writer, and the helpers only they call
    ok = layer
    layer = {a for a in ok if any(pkt(g) for g in touches[a] | addr_taken[a])
             or any(c in api or c in writer_glue for c in callees[a])}
    changed = True
    while changed:
        changed = False
        for a in sorted(ok - layer):
            cs = callers[a] - {a}
            if any(c in layer for c in callees[a]) or (cs and all(c in layer for c in cs)):
                layer.add(a)
                changed = True
    for a in ok - layer:
        reason[a] = "touches no packet state and calls no packet writer (a helper others call too)"

    # what keeps packet-touching logic off the list
    hist = defaultdict(int)
    for a, why in reason.items():
        if any(pkt(g) for g in touches[a] | addr_taken[a]) or any(c in api or c in writer_glue for c in callees[a]):
            hist[" ".join(why.split(" ", 2)[:2]).rstrip(",:")] += 1
    blocked = sorted(hist.items(), key=lambda kv: -kv[1])
    if args.reasons:
        for k, v in blocked[:30]:
            print(f"{v:5} {k}")
    for a in sorted(explain):
        print(f"{a:#010x} {nm(a)}: " + ("listed" if a in layer else reason.get(a, "not a game logic function")))
    if explain:
        return

    out = []
    for a in sorted(layer):
        kinds, _ = info[a]
        conf = "medium" if ("param" in kinds or "other" in kinds) else "high"
        cs = sorted(callers[a])
        out.append({"a": a, "name": nm(a), "module": module.get(a), "size": size[a], "conf": conf,
                    "ported": a in ported, "callers": cs,
                    "callers_layer": all(c in layer or cls.get(c) == "glue" for c in cs) and bool(cs),
                    "why": ", ".join(f"{k} stores {len(v)}" for k, v in sorted(kinds.items()) if k != "local")})

    by_mod = defaultdict(list)
    for x in out:
        by_mod[x["module"]].append(x)
    total = sum(x["size"] for x in out)
    print(f"{len(out)} functions, {total / 1024:.1f} KB "
          f"(high {sum(x['conf'] == 'high' for x in out)}, medium {sum(x['conf'] == 'medium' for x in out)}; "
          f"ported already {sum(x['ported'] for x in out)})")
    for m, xs in sorted(by_mod.items(), key=lambda kv: -sum(x["size"] for x in kv[1])):
        hi = [x for x in xs if x["conf"] == "high"]
        print(f"  {m:<8} {len(xs):>4} functions {sum(x['size'] for x in xs) / 1024:7.1f} KB"
              f"  (high {len(hi)}, {sum(x['size'] for x in hi) / 1024:.1f} KB; "
              f"unported {sum(not x['ported'] for x in xs)}, "
              f"{sum(x['size'] for x in xs if not x['ported']) / 1024:.1f} KB)")

    if args.tsv:
        with open(args.tsv, "w", encoding="utf-8") as f:
            f.write("# address\tname\tmodule\tsize\tconfidence\tported\tcallers\tpointer stores\n")
            for x in out:
                f.write(f"{x['a']:#010x}\t{x['name']}\t{x['module']}\t{x['size']}\t{x['conf']}\t"
                        f"{'ported' if x['ported'] else '-'}\t"
                        f"{','.join(nm(c) for c in x['callers'][:6])}\t{x['why']}\n")
    if args.out:
        lines = ["# PS2 packet layers (tools/packet-layers.py)", "",
                 "Logic functions (symbols/classes.tsv) whose only side effects are writes",
                 "into GS/VIF/DMA packet buffers. Candidates for M4's renderer to replace",
                 "instead of a port: **a user decision** (speedup-plan item 6); nothing is",
                 "reclassified yet. Rule and confidence levels: the tool's docstring.", "",
                 f"Total: {len(out)} functions, {total / 1024:.1f} KB; "
                 f"high confidence {sum(x['conf'] == 'high' for x in out)} "
                 f"({sum(x['size'] for x in out if x['conf'] == 'high') / 1024:.1f} KB); "
                 f"already ported {sum(x['ported'] for x in out)} "
                 f"({sum(x['size'] for x in out if x['ported']) / 1024:.1f} KB).", "",
                 f"Not counted: the {len(writer_glue)} packet writers classify.py already classes as glue "
                 f"({sum(size[a] for a in writer_glue) / 1024:.1f} KB).", "",
                 "| module | functions | KB | high | high KB | unported | unported KB |",
                 "|---|---:|---:|---:|---:|---:|---:|"]
        for m, xs in sorted(by_mod.items(), key=lambda kv: -sum(x["size"] for x in kv[1])):
            hi = [x for x in xs if x["conf"] == "high"]
            un = [x for x in xs if not x["ported"]]
            lines.append(f"| {m} | {len(xs)} | {sum(x['size'] for x in xs) / 1024:.1f} | {len(hi)} | "
                         f"{sum(x['size'] for x in hi) / 1024:.1f} | {len(un)} | "
                         f"{sum(x['size'] for x in un) / 1024:.1f} |")
        lines += ["", "Packet-touching logic left off the list, by first reason (top 12):", ""]
        lines += [f"- {v} {k}" for k, v in blocked[:12]]
        for m, xs in sorted(by_mod.items(), key=lambda kv: -sum(x["size"] for x in kv[1])):
            lines += ["", f"## {m}", "",
                      "| address | name | bytes | conf. | ported | callers | pointer stores |",
                      "|---|---|---:|---|---|---|---|"]
            for x in sorted(xs, key=lambda x: -x["size"]):
                cs = ", ".join(nm(c) for c in x["callers"][:3]) + (
                    f" +{len(x['callers']) - 3}" if len(x["callers"]) > 3 else "")
                lines.append(f"| {x['a']:#010x} | {x['name']} | {x['size']} | {x['conf']} | "
                             f"{'yes' if x['ported'] else ''} | {cs} | {x['why']} |")
        pathlib.Path(args.out).expanduser().write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
