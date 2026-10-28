#!/usr/bin/env python3
"""Finds hooked functions whose shim declares fewer stack argument words
than the original reads, counting the words it only passes on through a
tail jump (check-tailargs, 2026-10-26).

  FFX_GAME_DIR=... tools/check-tailargs.py [-v] [--stats] [--self-test]
  FFX_GAME_DIR=... tools/check-tailargs.py --explain NAME|0xADDR [--code]...

MsMusicReadStart (0x0079fff0) ends in `jmp SndMusicRead` with the stack as
it came in: SndMusicRead reads MsMusicReadStart's own argument. The port
declared no argument and called SndMusicRead with none, so the victory
fanfare never loaded; its oracle stub logged 0 arguments and passed. The
other checks miss it: check-ported compares `ret n` (nothing for cdecl),
and callarity's callee check skips callees that end in a tail jump.

For every `ported!` shim in crates/ffx-hook/src/ported/*.rs:

1. The words the original reads. Its stack is followed from the entry
   (check-argslots.py's `Frame`: push/pop, `sub/add esp` with signed
   immediates, ebp and other frame registers, aligned frames). Each
   operand at [esp+4+k] or [ebp+8+k] from the entry esp that is read (a
   load, a `push`, a compare, a read-modify-write) counts word k, except:
   - a store alone (MSVC reuses argument slots as scratch);
   - a `push` whose word goes, on every path, to a direct call whose
     callee doesn't read that argument (MenuMn2_StepDrawY80 pushes its
     four words for MenuMn2_Plus80, which reads none of them).
   `lea` of a slot counts the word holding that byte (its address is
   passed on). A `jmp` to another function (a tail jump) with d bytes
   pushed counts each word the target reads, less d/4, unless the straight
   run of code before the jump stores that word whole (pppDrawMatrix sets
   two of its target's words itself). Recursive, so a chain of tail jumps
   is measured to its end; a target the dump has no function for is read
   up to the next function.
2. The words the shim declares: its parameters before `;`, less `this`
   (thiscall) or ecx/edx (fastcall).

Problems:
- the shim declares fewer words than (1), unless it takes the caller's
  argument area (`; slots: u32`), which it reads itself;
- the shim takes a word (1) counts as `_name` and its body never mentions
  it (a dropped argument with the right count);
- for each tail-jump target of a hooked original that reads some of the
  original's words: a port call to that target, as tools/callarity.py
  resolves them (direct calls, a file's forwarding wrappers such as
  ffx-battle's `game(mem, call, SYM, &[..])`), passing fewer words than the
  target reads. Targets the ports reach only through the target's own port
  are counted, not checked (their hook is checked by the first rule).

Not seen: [esp+..] reads after an indirect call that pops its arguments
(a stdcall or thiscall through a pointer: the depth is taken as unchanged;
[ebp+..] reads are exact), and callees reading past the words pushed for
them. ALLOW names checked exceptions with their reasons; an entry that
matches nothing any more is reported, so the table stays true.

Prints one line per problem and "N problems (...)"; exits 1 when there are
any. -v lists every port call checked; --explain prints the instruction
that makes each word count (--code adds the disassembly); --stats tallies
measured against declared words per convention; --self-test checks the
measure on the originals the rules were written for.
"""
import argparse
import bisect
import importlib.util
import os
import pathlib
import re
import sys
from collections import defaultdict

TOOLS = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import progress  # noqa: E402
import timing_log  # noqa: E402

timing_log.install()


def _load(name, file):
    spec = importlib.util.spec_from_file_location(name, TOOLS / file)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


AS = _load("check_argslots", "check-argslots.py")  # Frame, accesses, the cache
SHIM = re.compile(r'^\s*(\w+) @ 0x([0-9a-f_]+) =>\s*extern "([\w-]+)" fn \w+\(([^)]*)\)', re.M)
SKIP = {"C": 0, "cdecl": 0, "stdcall": 0, "thiscall": 1, "fastcall": 2,
        # ecx, edx, then stack words the caller pops (ported!, check-callconv)
        "fastcall-caller-pops": 2}
TAIL = re.compile(r"jmp\s+0x([0-9a-f]+)$")
BRANCH = re.compile(r"j\w*\s+0x([0-9a-f]+)$")
STARTS = []
LIMIT = 200  # instructions followed from a push to its call

# hook name (or "call <file> -> 0x<target>" for check 3) -> why the shim's
# or the call's word count is right although the original reads more
ALLOW = {
    "Vec3AddScalars_00735100": "the sixth word (w) is copied to [ebp-0x8] and never read again; "
                               "only x, y and z are added (0x00735125)",
    "Phyre_CharacterLighting_setShadeCoeff": "`coeff`'s slot is scratch: the original stores its own "
                                             "float there (fstp [ebp+0xc] at 0x006aaf19) before it "
                                             "passes the slot's address to memcpy (0x006aaf6a)",
    "AtelPopStackFloat": "the worker (word 0) is read only on the empty-stack path (0x0086de67), "
                         "for the arguments of dbgPrintf (0x0062f500, a bare `ret`); the port "
                         "returns 0.0 and clears the error word there as the original does",
}


class Reads:
    """The stack argument words functions read, with the evidence."""

    def __init__(self):
        self.memo = {}

    def of(self, a):
        """({word: why}, {word: why} for words whose address is taken,
        [(tail jump address, target, bytes pushed)], unknown depth) of the
        function at `a`; `why` is (address, instruction text)."""
        if a in self.memo:
            return self.memo[a]
        self.memo[a] = ({}, {}, [], False)  # recursion through tail jumps
        size = self.size(a)
        if not size:
            return self.memo[a]
        code = AS.CACHE.get(a, size)
        fr = AS.Frame(code, a)
        reads, addrs, tails, unknown = {}, {}, [], False
        # branch targets: where a straight run of code starts
        labels = {int(m.group(1), 16) for i in code.values() if (m := BRANCH.match(i))}
        labels |= {t for i in code.values() for t in AS.table(i, code)}
        for at, insn in sorted(code.items()):
            if at not in fr.depth:
                continue  # not reached from the entry (padding, other code)
            for base, disp, size, kind in AS.accesses(insn):
                if base != "esp" and base not in fr.bases:
                    continue
                p = fr.position(at, base, disp)
                if p is None:
                    unknown = True
                    continue
                if p[0] != 0 or p[1] < 4 or kind == "w":
                    continue
                off = p[1] - 4
                if insn.startswith("push") and self.unread_push(code, fr, at):
                    continue
                into = addrs if kind == "addr" else reads
                for k in range(off // 4, (off + (1 if kind == "addr" else size) + 3) // 4):
                    into.setdefault(k, (at, insn))
            m = TAIL.fullmatch(insn)
            d = fr.depth.get(at)
            if m and int(m.group(1), 16) not in code:
                if d is None:
                    unknown = True
                elif d[0] == 0:
                    t = int(m.group(1), 16)
                    tails.append((at, t, d[1]))
                    own = self.stored_before(code, fr, labels, at)
                    for k, why in self.of(t)[0].items():
                        w = k - d[1] // 4
                        if w >= 0 and w not in own:
                            reads.setdefault(w, (at, f"{insn} ({name(t)} reads its word {k}: "
                                                     f"{why[1]} at {why[0]:#x})"))
        self.memo[a] = (reads, addrs, tails, unknown)
        return self.memo[a]

    @staticmethod
    def size(a):
        """The function's size in the dump, or for a tail-jump target the
        dump has no function at (pppDrawMatrix's 0x006ed5c0), the bytes up
        to the next function (at most 0x1000; the walk stops at its ret)."""
        if a in AS.ROWS:
            return AS.ROWS[a][1]
        if not STARTS or not STARTS[0] <= a < STARTS[-1]:
            return 0
        nxt = STARTS[bisect.bisect_right(STARTS, a)]
        return min(nxt - a, 0x1000)

    def unread_push(self, code, fr, at):
        """Whether the word `push`ed at `at` goes, on every path, to a direct
        call whose callee never reads it (followed through branches, at most
        LIMIT instructions): MenuMn2_StepDrawY80 passes its four arguments
        on to MenuMn2_Plus80, which reads none of them; SG_ApplyBoneSel
        pushes its first word, then calls SG_ApplyBone or SG_ApplyBoneDamped
        (a branch), and neither reads it."""
        d = fr.depth.get(at)
        if d is None:
            return False
        addrs = sorted(code)
        nxt = dict(zip(addrs, addrs[1:]))
        todo, seen, steps = [nxt.get(at)], set(), 0
        while todo:
            a = todo.pop()
            if a in seen:
                continue
            seen.add(a)
            steps += 1
            insn = code.get(a)
            dc = fr.depth.get(a)
            if insn is None or dc is None or dc[0] != d[0] or steps > LIMIT:
                return False
            if insn.startswith("call"):
                t = AS.call_target(insn)
                if t is None:
                    return False
                j = (dc[1] - (d[1] + 4)) // 4  # the pushed word's argument index
                r, ad, _, _ = self.of(t)
                if j < 0 or j in r or j in ad:
                    return False
                continue  # this path's call ignores it
            if dc[1] < d[1] + 4 or insn.startswith("ret"):
                return False  # popped unread? read by something else, or returned
            m = BRANCH.fullmatch(insn)
            if insn.startswith("j") and not m:
                return False  # an indirect jump
            if m:
                todo.append(int(m.group(1), 16))
                if insn.startswith("jmp"):
                    continue
            todo.append(nxt.get(a))
        return True

    @staticmethod
    def stored_before(code, fr, labels, at):
        """Argument words (from the entry) the straight run of code ending
        at the tail jump `at` stores whole: the function's own values for
        the target, not its caller's (pppDrawMatrix sets two words of its
        target's arguments and passes on one of its own)."""
        addrs = sorted(a for a in code if a <= at)
        own = set()
        for a in reversed(addrs[:-1]):
            if a not in fr.depth or re.match(r"(j\w*|ret\w*|call)\b", code[a]):
                break
            for base, disp, size, kind in AS.accesses(code[a]):
                p = fr.position(a, base, disp) if base == "esp" or base in fr.bases else None
                if p is not None and p[0] == 0 and kind == "w" and size >= 4 and (p[1] - 4) % 4 == 0 \
                        and p[1] >= 4:
                    own.update(range((p[1] - 4) // 4, (p[1] - 4 + size) // 4))
            if a in labels:
                break
        return own

    def words(self, a, with_addr=True):
        r, ad, _, _ = self.of(a)
        keys = set(r) | (set(ad) if with_addr else set())
        return max(keys) + 1 if keys else 0


def name(a):
    return AS.ROWS.get(a, (f"{a:#x}",))[0]


def shims():
    out = []
    for f in sorted((progress.REPO / "crates/ffx-hook/src/ported").glob("*.rs")):
        text = f.read_text(encoding="utf-8")
        for m in SHIM.finditer(text):
            nm, addr, abi, params = m.groups()
            a = int(addr.replace("_", ""), 16)
            main, semi, _ = params.partition(";")
            names = [p.split(":")[0].strip() for p in main.split(",") if ":" in p][SKIP[abi]:]
            # stack words the shim takes and ignores (`_name: u32` its body
            # never mentions)
            body = text[m.end():text.find("\n    }", m.end())]
            ignored = {k for k, n in enumerate(names)
                       if n.startswith("_") and not re.search(rf"\b{n}\b", body)}
            out.append((f.name, nm, a, abi, len(names), bool(semi), ignored))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("-v", action="store_true", help="list every tail jump checked")
    ap.add_argument("--explain", action="append", default=[], metavar="NAME",
                    help="print why each word of this hooked function (or 0xADDR) counts")
    ap.add_argument("--code", action="store_true", help="with --explain: print the disassembly too")
    ap.add_argument("--self-test", action="store_true", help="check the measure on known originals")
    ap.add_argument("--stats", action="store_true", help="tally measured against declared words")
    ap.add_argument("--jobs", "-j", type=int, default=os.cpu_count() or 1)
    args = ap.parse_args()
    AS.CACHE.load()
    AS.ROWS = {a: (n, s) for a, n, s in progress.functions()}
    STARTS.extend(sorted(AS.ROWS))
    hooks = [h for h in shims() if h[2] in AS.ROWS]
    AS.CACHE.prefetch(((h[2], AS.ROWS[h[2]][1]) for h in hooks), args.jobs)
    R = Reads()
    # tail-jump targets (and theirs) are disassembled on demand
    for h in hooks:
        R.of(h[2])
    AS.CACHE.save()

    if args.self_test:
        # (address, the words it reads): the cases the rules above were written for
        cases = [
            (0x0079FFF0, {0}),  # MsMusicReadStart: its word only through the tail jump
            (0x00734B70, {0}),  # pppDrawMatrix: stores words 0-2 of its target's, reads its word 0
            (0x008DCA30, {0, 1}),  # TkMenuDrawSp3R0: word 2 stored, 0-1 passed through
            (0x008B7C00, {0, 1, 2}),  # TOGetFontWidthForLang: two tail jumps, word 3 stored on both
            (0x0081CF90, {0}),  # SqsBattleWaveDataSend: tail jump on one branch, stores 0-1
            (0x008C0DD0, set()),  # MenuMn2_StepDrawY80: MenuMn2_Plus80 reads none of the 4 it pushes
            (0x00838240, {1, 2, 3}),  # SG_ApplyBoneSel: word 0 to SG_ApplyBone(Damped), unread
            (0x0088A040, {0, 1, 2}),  # Math_SegmentNearestT: `add esp, 0xfffffff8` (signed)
            (0x007099A0, {0}),  # FmodMusic_stop: `lea eax,[ebp+0xb]` is the address of word 0
        ]

        def got(a):
            r, ad, _, _ = R.of(a)
            return set(r) | set(ad)

        bad = [(a, want, got(a)) for a, want in cases if got(a) != want]
        for a, want, have in bad:
            print(f"self-test: {name(a)} ({a:#x}) reads words {sorted(have)}, expected {sorted(want)}")
        print(f"self-test: {len(cases) - len(bad)} of {len(cases)} passed")
        sys.exit(1 if bad else 0)

    if args.explain:
        by = {h[1]: h for h in hooks}
        for e in args.explain:
            a = by[e][2] if e in by else int(e, 16)
            r, ad, tails, unknown = R.of(a)
            print(f"{name(a)} ({a:#x}): reads {R.words(a, False)} words, "
                  f"{R.words(a)} with taken addresses{'; some depths unknown' if unknown else ''}")
            for k in sorted(r):
                print(f"  word {k}: {r[k][1]} at {r[k][0]:#x}")
            for k in sorted(ad):
                print(f"  word {k} (address): {ad[k][1]} at {ad[k][0]:#x}")
            for at, t, d in tails:
                print(f"  tail jump at {at:#x} to {name(t)} ({t:#x}), {d} bytes pushed: "
                      f"it reads {R.words(t, False)} words")
            if args.code:
                for at, insn in sorted(AS.CACHE.get(a, R.size(a)).items()):
                    print(f"    {at:08x}  {insn}")
        return

    if args.stats:
        # measured words against declared ones, per convention; for the
        # callee-pops ones the declared count is the `ret n` (check-ported)
        tally = defaultdict(int)
        for _, _, a, abi, declared, _, _ in hooks:
            w = R.words(a, False)
            tally[(abi, "as many stack words as" if w == declared else
                   "fewer stack words than" if w < declared else "more stack words than")] += 1
        for (abi, how), n in sorted(tally.items()):
            print(f"{abi}: {n} originals read {how} the shim declares")
    used = set()
    problems = []
    unknown = 0
    for fname, nm, a, abi, declared, slots, ignored in hooks:
        r, ad, _, unk = R.of(a)
        unknown += unk
        need = R.words(a, with_addr=not slots)
        # a shim with the caller's argument area reads its words through it
        dropped = [] if slots else sorted(k for k in ignored if k in r or k in ad)
        if not slots and need > declared:
            k = declared  # the first word the shim lacks
            what = f"declares {declared} stack words, the original reads {need}"
        elif dropped:
            k = dropped[0]
            what = f"ignores its stack word {k} (`_` parameter), which the original reads"
        else:
            continue
        why = r.get(k) or ad.get(k)
        if nm in ALLOW:
            used.add(nm)
            continue
        problems.append(f"{fname}: {nm} ({a:#x}, {abi}) {what} (word {k}: {why[1]} at {why[0]:#x})")

    # 3. the ports' calls to the hooked originals' tail-jump targets
    import callarity  # noqa: E402

    exe = pathlib.Path(os.environ["FFX_GAME_DIR"]).expanduser() / "FFX.exe"
    callarity.check(progress.REPO, exe, {a: s for a, (_, s) in AS.ROWS.items()})
    calls = defaultdict(list)
    for d in callarity.check.calls:
        calls[d.a].append(d)
    targets = defaultdict(list)
    for fname, nm, a, *_ in hooks:
        for at, t, d in R.of(a)[2]:
            if R.words(t, False) > d // 4:  # it reads some of the caller's words
                targets[t].append(nm)
    checked = unresolved = 0
    for t, by in sorted(targets.items()):
        need = R.words(t, False)
        if not calls.get(t):
            # its port calls the target's port, or a call callarity can't read
            unresolved += 1
            if args.v:
                print(f"{name(t)} ({t:#x}, reads {need}; tail target of {', '.join(sorted(set(by)))}): "
                      f"no resolved port call")
        for d in calls.get(t, []):
            checked += 1
            if args.v:
                print(f"{d.where}: {d.label} passes {d.count} words to {name(t)} ({t:#x}, reads {need}; "
                      f"tail target of {', '.join(sorted(set(by)))})")
            key = f"call {d.where.split(':')[0]} -> {t:#010x}"
            if d.count >= need:
                continue
            if key in ALLOW:
                used.add(key)
                continue
            problems.append(f"{d.where}: {d.label} passes {d.count} words to {name(t)} ({t:#x}), which reads "
                            f"{need} (a tail-jump target of {', '.join(sorted(set(by)))})")
    for k in sorted(set(ALLOW) - used):
        problems.append(f"ALLOW entry {k!r} matches nothing any more")
    for p in problems:
        print(p)
    print(f"{len(problems)} problems ({len(hooks)} hooks measured, {unknown} with code at an unknown "
          f"stack depth; {len(targets)} tail-jump targets that read arguments, {checked} port calls to them "
          f"checked, {unresolved} targets without a resolved call; {len(ALLOW)} allowed)")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
