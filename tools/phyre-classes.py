#!/usr/bin/env python3
"""Which PhyreEngine code M5 needs, given the classes the game's data and
code use (docs/rewrite-plan.md, M5 stage 1).

  FFX_GAME_DIR=... tools/phyre-classes.py [--census TSV] [--out DIR] [--jobs N]

tools/reach.py counts every vtable in .rdata as a root, so every class
descriptor PhyreEngine links in looks reachable. But PhyreEngine objects
mostly have no vtables of their own: what a class does at run time
(construct, destroy, fix up after loading, script accessors) sits in the
vtables of its descriptor templates (PClassDescriptorForType<T>,
PClassDescriptorConcrete<T>, PClassDataMemberPhyreArray<T>,
PMethodCallerConcrete<T,...>; found by their RTTI names). A cluster names
the classes of its objects in its namespace; the loader looks each name up
(PNamespace_getClassDescriptor) and calls the descriptor's slots. So a
descriptor's slots run only for classes the data names or the code uses.

The model, on reach.py's graph:
  - roots: reach.py's, without `vtable`, and `strict` (an address taken in
    code counts only when that code is reachable; EH funclets through the
    function that installs their handler);
  - a function whose code holds the address of a vtable (a constructor:
    mov [obj], offset vtable) reaches all of its slots; a vtable a data
    section points at is a root;
  - a descriptor-template vtable of class T only counts when T is used:
    T is named in a cluster namespace (--census, from
    `cargo run --release -p ffx-phyre --example class-census`), or
    reachable code outside T's own descriptor code takes the address of
    T's descriptor object (code use: new objects of T, type checks,
    GetClassDescriptor<T>). Iterated until nothing changes.

Check: every hooked PhyreEngine function (M4 ported the ones that ran) and
every coverage hit must stay reachable; the misses are listed.

A registered class that is not used still keeps the descriptor slots the
start-up calls for every class (--always, see ALWAYS_SLOTS). Without the
script layer (a second solution in the report) the slots that move values
to and from a Phyre script context and the method/function caller vtables
drop out; none of the latter ran in any M4 run.

Evidence helpers: PHYRE_SLOTS=<k> prints, per class-specific function in
descriptor slot k, whether it ran (is hooked) by named/code-use class
(PHYRE_SLOTS_LIST=1 per class); PHYRE_PROBE=<class>,... prints the
functions taking a class's descriptor.

Outputs (default $FFX_WORK/m5/scope):
  phyre-needed.tsv    reachable, not hooked PhyreEngine functions with the
                      class (or vtable) they are reached through
  phyre-classes.tsv   every descriptor class: named / code use / kept,
                      its vtables and the functions only it reaches
  phyre-classes.md    the counts (report section)
"""
import argparse
import bisect
import os
import pathlib
import re
import sys
from collections import Counter, defaultdict, deque

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import progress  # noqa: E402
import reach  # noqa: E402
import refs  # noqa: E402

WORK = progress.WORK
# descriptor-template families: their vtables belong to their first template
# argument's class
GATED = ("PClassDescriptorForType", "PClassDescriptorAbstract", "PClassDescriptorConcrete",
         "PClassDescriptorWithoutDefaultConstructor", "PClassDescriptorWrapper", "PClassDescriptorHeader",
         "PClassDescriptorHeaderWithoutDefaultConstructor", "PClassDataMemberArrayDynamic",
         "PClassDataMemberPhyreArray", "PClassDataMemberPhyreSharray", "PClassDataMemberDynamicType",
         "PNamedSemanticDescriptorForType", "PNamedSemanticDescriptorForDynamicType",
         "PMethodCallerConcrete", "PFunctionCallerConcrete")
SCRIPT = ("PMethodCallerConcrete", "PFunctionCallerConcrete")
# PType/PClassDescriptor vtable slots: 3-6 move values to and from a script
# context (getValueFromScriptContext, putValueToScriptContext, ...); 10-11
# build and destroy instances (constructInstance, destroyInstance), which the
# default-buffer set-up calls for every registered class (375 of the 400
# class-specific slot-10 functions ran in the M4 runs, 73 of them for classes
# neither named nor used); 12-14 (validation, after-load fixups) ran for a
# few unused classes too, so they count as well: with 10-11 only, 4 hooked
# functions fall out
SCRIPT_SLOTS = (3, 4, 5, 6)
ALWAYS_SLOTS = (10, 11, 12, 13, 14)
# functions that register classes (their members' and derived classes'
# descriptors), by their recovered names
REGISTRAR_NAME = re.compile(r"_Bind(_|$)|BindAll|RegisterClass|RegisterType|DeclareClassDescriptor|"
                            r"PTypeDispenser_GetType|PClassBind_|_initialize(_|$)|PhyreInit")
CONTAINERS = {"PArray", "PSharray"}
# MSVC primitive codes, spelled as Phyre's clusters spell them
PRIM = {"C": "PInt8", "D": "char", "E": "PUInt8", "F": "PInt16", "G": "PUInt16", "H": "PInt32",
        "I": "PUInt32", "J": "long", "K": "unsigned long", "M": "float", "N": "double", "X": "void",
        "_N": "bool", "_J": "PInt64", "_K": "PUInt64", "_W": "wchar_t"}


# ---------------------------------------------------------- MSVC demangling

class Demangle:
    """Enough of the MSVC type grammar for RTTI names: classes with scopes,
    templates, pointers, primitives and integer arguments. Names drop their
    namespaces (clusters name classes without them)."""

    def __init__(self, s):
        self.s, self.i, self.names = s, 0, []

    def peek(self, n=1):
        return self.s[self.i:self.i + n]

    def ident(self):
        """A simple name or a back reference; the scopes that follow are
        read and dropped."""
        if self.peek().isdigit():
            k = int(self.peek())
            self.i += 1
            return self.names[k] if k < len(self.names) else "?"
        if self.peek(2) == "?$":
            self.i += 2
            j = self.s.index("@", self.i)
            base = self.s[self.i:j]
            self.i = j + 1
            saved = self.names
            self.names = [base]
            args = []
            while self.peek() != "@":
                a = self.typ()
                if a is not None:
                    args.append(a)
            self.i += 1
            self.names = saved
            name = f"{base}<{','.join(args)}>"
            self.names.append(name)
            return name
        j = self.s.index("@", self.i)
        name = self.s[self.i:j]
        self.i = j + 1
        if name not in self.names:
            self.names.append(name)
        return name

    def qualified(self):
        first = self.ident()
        while self.peek() != "@":
            self.ident()  # scope
        self.i += 1
        return first

    def number(self):
        if self.peek().isdigit():
            v = int(self.peek()) + 1
            self.i += 1
            return v
        j = self.s.index("@", self.i)
        v = int("".join("0123456789ABCDEF"["ABCDEFGHIJKLMNOP".index(c)] for c in self.s[self.i:j]) or "0", 16)
        self.i = j + 1
        return v

    def typ(self):
        c = self.peek()
        if c in "VUT":
            self.i += 1
            return self.qualified()
        if self.peek(2) == "W4":
            self.i += 2
            return self.qualified()
        if c in "PQAR":  # pointer/reference: storage class then the type
            self.i += 1
            if self.peek() == "E":
                self.i += 1
            self.i += 1  # A, B, C, D (cv)
            return f"{self.typ()} *"
        if c == "$":
            self.i += 1
            k = self.peek()
            self.i += 1
            if k == "0":
                self.number()  # an integer argument: dropped
                return None
            if k == "$":  # $$Q rvalue ref, $$C cv
                self.i += 1
                return self.typ()
            return None
        if c == "_":
            self.i += 2
            return PRIM.get(self.s[self.i - 2:self.i], self.s[self.i - 2:self.i])
        if c == "?":
            self.i += 1
            return self.typ()
        self.i += 1
        return PRIM.get(c, c)


def template_of(rtti):
    """('Family', 'first argument') of an RTTI name like
    .?AV?$PClassDescriptorForType@VPMesh@PGeometry@Phyre@@@Phyre@@, or None."""
    m = re.match(r"\.\?A[VU]\?\$(\w+)@", rtti)
    if not m:
        return None
    d = Demangle(rtti)
    d.i = m.end()
    d.names = [m[1]]
    try:
        arg = d.typ()
    except (ValueError, IndexError):
        return m[1], "?"
    return m[1], arg


def plain_of(rtti):
    """Simple class name of a non-template RTTI name, with its scopes."""
    m = re.match(r"\.\?A[VU](.*)@@$", rtti)
    return "::".join(reversed(m[1].split("@"))) if m else rtti


def identifiers(arg):
    return {t for t in re.findall(r"[A-Za-z_]\w*", arg) if t not in CONTAINERS}


# ------------------------------------------------------------------- model

def vtables(img):
    """[(start, slot count, RTTI name)]: reach.py's vtable runs."""
    data_relocs = [r for r in img.relocs if img.section(r) not in (".text", ".reloc", None)]
    ds = set(data_relocs)
    out = []
    for r in data_relocs:
        if img.section(r) != ".rdata" or not img.in_text(img.dword(r)):
            continue
        prev = r - 4
        if prev in ds and img.in_text(img.dword(prev)):
            continue
        col = img.dword(prev) if prev in ds else None
        if col is None or img.section(col) != ".rdata" or img.dword(col) != 0:
            continue
        td = img.dword(col + 12)
        name = img.cstr(td + 8) if td else ""
        n, a = 0, r
        while a in ds and img.in_text(img.dword(a)):
            n, a = n + 1, a + 4
        out.append((r, n, name))
    return out


def read_census(path):
    named, fixups = {}, set()
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        f = line.split("\t")
        if f[0] == "class":
            named[f[1]] = (int(f[3]), int(f[4]), int(f[5]))
        elif f[0] == "fixup":
            fixups.add(f[1])
    return named, fixups


def norm(name):
    """Cluster and demangled spellings to one form."""
    return re.sub(r"\s+", "", name).replace("Phyre::", "")


def write_script_traps(out, img, g, vinfo, seen, seen_ns, used):
    """The script layer's entry points as coverage-mode candidates.

    Entries: every slot of every PMethodCallerConcrete/PFunctionCallerConcrete
    vtable and slots 3-6 (SCRIPT_SLOTS) of every PClassDescriptor* vtable,
    used class or not, at the slot's own target address (Ghidra does not
    always start a function there, so COVERAGE_ALL may not have armed it).
    Inner: the function starts only the script layer reaches (seen minus
    seen_ns), any module. Writes
      cand.txt     `<address> <first byte>` (tools/coverage.py's format)
      entries.tsv  address, kind (entry/inner), family, class, slot, class used,
                   in the dump, name
    """
    out.mkdir(parents=True, exist_ok=True)
    starts = set(g.starts)
    rows = {}  # address -> [kind, family, class, slot, used]

    def matches(cls):
        return norm(cls) in used or bool(identifiers(cls) & used)

    for v, (n, _, fam, cls) in sorted(vinfo.items()):
        if not fam:
            continue
        if fam in SCRIPT:
            ks = range(n)
        elif fam.startswith("PClassDescriptor"):
            ks = [k for k in SCRIPT_SLOTS if k < n]
        else:
            continue
        for k in ks:
            a = img.dword(v + 4 * k)
            if not img.in_text(a):
                continue
            r = rows.setdefault(a, ["entry", set(), set(), set(), False])
            r[1].add(fam)
            r[2].add(cls)
            r[3].add(k)
            r[4] |= matches(cls)
    for a in sorted(set(seen) - set(seen_ns)):
        rows.setdefault(a, ["inner", set(), set(), set(), False])
    text = [(va, blob) for _, va, blob, code in img.secs if code]

    def first_byte(a):
        for va, blob in text:
            if va <= a < va + len(blob):
                return blob[a - va]
        return None

    cand, tsv = [], ["# address\tkind\tfamilies\tclasses\tslots\tclass used\tin dump\tname"]
    for a in sorted(rows):
        kind, fams, clss, ks, use = rows[a]
        b = first_byte(a)
        if b is None or b == 0xCC:
            continue
        cand.append(f"{a:08x} {b:02x}")
        node = g.node_of(a)
        name = g.name.get(a) or (f"{g.name[node]}+0x{a - node:x}" if node is not None else "")
        tsv.append(f"0x{a:08x}\t{kind}\t{','.join(sorted(fams))}\t{';'.join(sorted(clss))[:300]}\t"
                   f"{','.join(map(str, sorted(ks)))}\t{int(use)}\t{int(a in starts)}\t{name}")
    (out / "cand.txt").write_text("\n".join(cand) + "\n", encoding="utf-8")
    (out / "entries.tsv").write_text("\n".join(tsv) + "\n", encoding="utf-8")
    n_entry = sum(1 for r in rows.values() if r[0] == "entry")
    print(f"script traps: {len(cand)} candidates ({n_entry} entries, {len(rows) - n_entry} inner) -> {out}")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--census", type=pathlib.Path, default=WORK / "m5/scope/cluster-classes.tsv")
    ap.add_argument("--out", type=pathlib.Path, default=WORK / "m5/scope")
    ap.add_argument("--registrar", type=int, default=4,
                    help="a function taking this many classes' descriptors registers them (default 4)")
    ap.add_argument("--jobs",type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--always", default=",".join(map(str, ALWAYS_SLOTS)),
                    help="descriptor slots every registered class keeps (default %(default)s)")
    ap.add_argument("--script-traps", type=pathlib.Path, metavar="DIR",
                    help="also write the script layer's coverage traps to DIR (see write_script_traps)")
    args = ap.parse_args()
    always = tuple(int(k) for k in args.always.split(",") if k)

    img = reach.Image(refs.default_exe())
    g = reach.Graph(img, args.jobs, True)
    by_name = {n: a for a, n in g.name.items()}
    roots, code_imm, eh_edges, _, _ = reach.find_roots(g, by_name)
    hooked = progress.hooks()
    replaced = {g.node_of(a) for a, s in hooked.items() if s == "replaced" and g.node_of(a) is not None}
    named, _ = read_census(args.census)

    # ---- vtables and their classes
    vts = vtables(img)
    vinfo = {}  # start -> (slots, rtti, family or None, class)
    for start, n, rtti in vts:
        t = template_of(rtti)
        if t and t[0] in GATED:
            vinfo[start] = (n, rtti, t[0], t[1])
        else:
            vinfo[start] = (n, rtti, None, plain_of(rtti) if not t else f"{t[0]}<{t[1]}>")
    vstarts = sorted(vinfo)

    def vt_of(v):
        i = bisect.bisect_right(vstarts, v) - 1
        if i >= 0 and v < vstarts[i] + 4 * vinfo[vstarts[i]][0]:
            return vstarts[i]
        return None

    def slots(v):
        return {g.node_of(img.dword(v + 4 * k)) for k in range(vinfo[v][0])} - {None}

    code_relocs = [r for r in img.relocs if img.text_lo <= r < img.text_hi]
    data_relocs = [r for r in img.relocs if img.section(r) not in (".text", ".reloc", None)]
    vt_refs = defaultdict(set)      # function -> vtables whose address its code holds
    vt_users = defaultdict(set)     # vtable -> those functions
    for r in code_relocs:
        v = vt_of(img.dword(r))
        if v is not None:
            f = g.node_of(r)
            vt_refs[f].add(v)
            vt_users[v].add(f)
    vt_data_roots = {vt_of(img.dword(r)) for r in data_relocs} - {None}

    # ---- descriptor objects: the global a descriptor constructor of T (a
    # function holding T's descriptor vtable) is called on (mov ecx, offset
    # desc; call ctor)
    cls_vts = defaultdict(set)      # class -> gated vtables
    for v, (_, _, fam, cls) in vinfo.items():
        if fam:
            cls_vts[cls].add(v)
    ctor_cls = defaultdict(set)     # descriptor constructor -> classes
    for v, (_, _, fam, cls) in vinfo.items():
        if fam and fam.startswith("PClassDescriptor"):
            for f in vt_users[v]:
                ctor_cls[f].add(cls)
    relocs_sorted = img.relocs
    data_lo = min(va for name, va, *_ in img.secs if name == ".data")

    def is_global(v):
        return v is not None and v >= data_lo and img.section(v) in (".data", None)

    def text_byte(a):
        return g.text[a - img.text_lo]

    desc_obj = defaultdict(set)     # class -> descriptor object addresses
    for pc, _, op, t in g.dis.branches:
        if op not in ("call", "jmp") or t not in ctor_cls:
            continue
        i = bisect.bisect_left(relocs_sorted, pc - 16)
        while i < len(relocs_sorted) and relocs_sorted[i] < pc:
            r = relocs_sorted[i]
            if text_byte(r - 1) == 0xB9 and is_global(img.dword(r)):  # mov ecx, offset desc
                for c in ctor_cls[t]:
                    desc_obj[c].add(img.dword(r))
            i += 1
    obj_starts = sorted((o, c) for c, os_ in desc_obj.items() for o in os_)
    ostarts = [o for o, _ in obj_starts]

    def obj_class(a):
        i = bisect.bisect_right(ostarts, a) - 1
        if i >= 0 and a < ostarts[i] + 0xa0:
            return obj_starts[i][1]
        return None

    # function -> classes whose descriptor object its code takes, or whose
    # descriptor constructor or getter it calls
    desc_takers = defaultdict(set)
    for r in code_relocs:
        c = obj_class(img.dword(r))
        if c is not None:
            desc_takers[g.node_of(r)].add(c)
    for f, ds in g.out.items():
        for d in ds:
            if d in ctor_cls:
                desc_takers[f] |= ctor_cls[d]
    # a class's own descriptor code: the slots of its gated vtables and the
    # functions holding their addresses (descriptor constructors, Bind)
    own = defaultdict(set)
    for c, vs in cls_vts.items():
        for v in vs:
            own[c] |= slots(v) | vt_users[v]

    # ---- roots and edges
    loose = ("code-imm", "safeseh", "c++eh", "vtable")
    base_roots = [(k, n, s) for k, n, s, _ in roots if k not in loose and n is not None]
    extra = defaultdict(set)
    for s, d in code_imm:
        extra[s].add(d)
    for s, ds in eh_edges.items():
        extra[s] |= ds

    def matches(cls, used_names):
        return norm(cls) in used_names or bool(identifiers(cls) & used_names)

    def kept_slots(v, used, script):
        """The slot indices of vtable v that count, given the used classes."""
        n, _, fam, cls = vinfo[v]
        if not fam:
            return range(n)
        use = matches(cls, used)
        if fam in SCRIPT:
            return range(n) if use and script else ()
        if fam.startswith("PClassDescriptor"):
            if use:
                return [k for k in range(n) if script or k not in SCRIPT_SLOTS]
            # registered but unused: what registration itself calls
            return [k for k in always if k < n]
        return range(n) if use else ()

    def run(used, script):
        """BFS; node -> label (the class or vtable that brought it in)."""
        seen = {}
        q = deque()
        for kind, n, _ in base_roots:
            if n not in seen:
                seen[n] = f"root:{kind}"
                q.append(n)

        def take(v, lab):
            for k in kept_slots(v, used, script):
                w = g.node_of(img.dword(v + 4 * k))
                if w is not None and w not in seen:
                    seen[w] = lab
                    q.append(w)

        for v in sorted(vt_data_roots):
            take(v, vinfo[v][3])
        while q:
            a = q.popleft()
            lab = seen[a]
            for w in sorted(g.out.get(a, ())) + sorted(extra.get(a, ())):
                if w not in seen:
                    seen[w] = lab
                    q.append(w)
            for v in sorted(vt_refs.get(a, ())):
                fam, cls = vinfo[v][2], vinfo[v][3]
                take(v, f"{fam}<{cls}>" if fam else cls)
        return seen

    data_used = {norm(n) for n in named} | {t for n in named for t in identifiers(n)}
    # registration: code that builds or tears down descriptors rather than
    # using one: static initialisers and atexit destructors, EH unwind
    # funclets (they destroy what a registration built), and functions
    # taking the descriptors of REGISTRAR or more classes (BindAll,
    # PUtility*::initialize, the script accessor tables). Their own calls
    # and pointers still count.
    registrars = {f for f, cs in desc_takers.items()
                  if len(cs) >= args.registrar or g.module[f] == "static-init" or g.name[f].startswith("Unwind@")
                  or REGISTRAR_NAME.search(g.name[f])}

    def solve(script):
        used = set(data_used)
        code_used = {}              # class -> the function that uses it
        for _ in range(50):
            seen = run(used, script)
            new = {}
            for f, cs in sorted(desc_takers.items()):
                if f in seen and f not in registrars:
                    for c in cs:
                        if f not in own[c] and not matches(c, used):
                            new.setdefault(c, f)
            if not new:
                break
            code_used.update(new)
            used |= {norm(c) for c in new}
        return run(used, script), used, code_used

    seen, used, code_used = solve(True)
    seen_ns, used_ns, code_used_ns = solve(False)
    if args.script_traps:
        write_script_traps(args.script_traps, img, g, vinfo, seen, seen_ns, used)
    if os.environ.get("PHYRE_SLOTS"):
        k = int(os.environ["PHYRE_SLOTS"])
        tally = Counter()
        for c, vs in sorted(cls_vts.items()):
            fns = {g.node_of(img.dword(v + 4 * k)) for v in vs if vinfo[v][0] > k
                   and vinfo[v][2].startswith("PClassDescriptor")}
            fns = {f for f in fns if f is not None and len({cc for cc, vv in cls_vts.items()
                                                             for v2 in vv if vinfo[v2][0] > k
                                                             and g.node_of(img.dword(v2 + 4 * k)) == f}) == 1}
            if not fns:
                continue
            ran = any(f in replaced for f in fns)
            tally[(ran, matches(c, data_used), c in code_used)] += 1
            if os.environ.get("PHYRE_SLOTS_LIST"):
                print(f"   slot{k} {c}: ran={ran} named={matches(c, data_used)} code={c in code_used}")
        print("slot", k, "(ran, named, code use):", sorted(tally.items()))
    for probe in os.environ.get("PHYRE_PROBE", "").split(","):
        if probe:
            print(f"== {probe}: desc objects {[hex(o) for o in desc_obj.get(probe, ())]}")
            for f, cs in sorted(desc_takers.items()):
                if probe in cs:
                    print(f"   taker {f:#x} {g.name[f]} [{g.module[f]}] reach={f in seen} own={f in own[probe]} "
                          f"registrar={f in registrars} n={len(cs)} callers={[g.name[c] for c in g.inn.get(f, ())][:4]}")

    # ---- reference: reach.py's own count (every vtable a root)
    full, _ = reach.bfs(g, roots, reach.ROOT_ORDER)
    lib = "phyre"
    phyre_nodes = [a for a in g.starts if g.module[a] == lib]
    full_unhooked = {a for a in phyre_nodes if a in full and a not in replaced}
    need = {a for a in phyre_nodes if a in seen}
    need_unhooked = need - replaced
    misses = sorted(a for a in replaced if g.module.get(a) == lib and a not in seen and a in full)
    cov = set()
    for s in reach.coverage_hits().values():
        for h in s:
            n = g.node_of(h)
            if n is not None:
                cov.add(n)
    cov_miss = sorted(a for a in cov if g.module.get(a) == lib and a not in seen)

    # ---- per class
    all_classes = sorted(cls_vts)
    named_hit = {n for n in named if any(matches(c, {norm(n)}) or norm(c) == norm(n) for c in all_classes)}
    no_desc = sorted(set(named) - named_hit)
    kept = [c for c in all_classes if matches(c, used)]
    never = [c for c in all_classes if c not in kept]
    label_count = Counter(seen[a] for a in need_unhooked)

    args.out.mkdir(parents=True, exist_ok=True)
    with open(args.out / "phyre-needed.tsv", "w", encoding="utf-8") as f:
        f.write("# address\tname\tsize\treached through (class/vtable or root kind)\tscript layer only\t"
                "category\n")
        cat = progress.classes()
        for a in sorted(need_unhooked):
            f.write(f"0x{a:08x}\t{g.name[a]}\t{g.size[a]}\t{seen[a]}\t{int(a not in seen_ns)}\t{cat.get(a, '')}\n")
    with open(args.out / "phyre-classes.tsv", "w", encoding="utf-8") as f:
        f.write("# class\tnamed in data\tcode use\tkept\tdescriptor vtables\tfamilies\t"
                "phyre functions not hooked that it brings in\n")
        for c in all_classes:
            fams = sorted({vinfo[v][2] for v in cls_vts[c]})
            n_in = sum(1 for a in need_unhooked if seen[a].endswith(f"<{c}>") and "<" in seen[a])
            f.write(f"{c}\t{int(matches(c, data_used))}\t{int(c in code_used)}\t{int(c in kept)}\t"
                    f"{len(cls_vts[c])}\t{','.join(fams)}\t{n_in}\n")

    def kb(addrs):
        return sum(g.size[a] for a in addrs) // 1024

    need_ns = {a for a in phyre_nodes if a in seen_ns}
    need_ns_unhooked = need_ns - replaced
    misses_ns = sorted(a for a in replaced if g.module.get(a) == lib and a not in seen_ns and a in full)
    script_fns = need_unhooked - need_ns_unhooked
    L = ["# PhyreEngine classes and the code they need", ""]
    L.append(f"tools/phyre-classes.py on FFX.exe {img.hash}; census {args.census.name}: "
             f"{len(named)} class names in the cluster namespaces.")
    L.append("")
    L.append("| set | count |")
    L.append("| --- | --- |")
    L.append(f"| RTTI vtables | {len(vinfo)} ({sum(1 for v in vinfo.values() if v[2])} descriptor-template) |")
    L.append(f"| classes with descriptor-template vtables | {len(all_classes)} |")
    L.append(f"| named in data (cluster namespaces) | {len(named)} |")
    L.append(f"| of them matched to a descriptor class | {len(named_hit)} |")
    L.append(f"| descriptor classes kept: named | {sum(1 for c in all_classes if matches(c, data_used))} |")
    L.append(f"| descriptor classes kept: code use only | {len(code_used)} |")
    L.append(f"| descriptor classes kept (union) | {len(kept)} |")
    L.append(f"| descriptor classes never named nor used | {len(never)} |")
    registered = [c for c in all_classes if any(f in seen for v in cls_vts[c] for f in vt_users[v])]
    L.append(f"| descriptor classes registered (a descriptor constructor reachable) | {len(registered)} |")
    L.append("")
    L.append("| phyre functions | count | KB |")
    L.append("| --- | --- | --- |")
    L.append(f"| all | {len(phyre_nodes)} | {kb(phyre_nodes)} |")
    L.append(f"| reach.py reachable, not hooked | {len(full_unhooked)} | {kb(full_unhooked)} |")
    L.append(f"| this model: reachable | {len(need)} | {kb(need)} |")
    L.append(f"| this model: reachable, hooked | {len(need & replaced)} | {kb(need & replaced)} |")
    L.append(f"| this model: reachable, not hooked | {len(need_unhooked)} | {kb(need_unhooked)} |")
    L.append(f"| dropped from reach.py's list | {len(full_unhooked - need_unhooked)} | "
             f"{kb(full_unhooked - need_unhooked)} |")
    L.append(f"| without the script layer (slots {SCRIPT_SLOTS}, method/function callers): reachable | "
             f"{len(need_ns)} | {kb(need_ns)} |")
    L.append(f"| without the script layer: reachable, not hooked | {len(need_ns_unhooked)} | "
             f"{kb(need_ns_unhooked)} |")
    L.append(f"| only the script layer reaches | {len(script_fns)} | {kb(script_fns)} |")
    L.append("")
    L.append(f"Without the script layer: {len(used_ns)} used names, {len(code_used_ns)} classes by code use; "
             f"hooked phyre functions it misses: {len(misses_ns)}.")
    for a in misses_ns[:20]:
        L.append(f"- miss (no script) `0x{a:08x}` {g.name[a]}")
    L.append("")
    L.append(f"Check: hooked phyre functions this model misses: {len(misses)}; "
             f"coverage hits it misses: {len(cov_miss)}.")
    for a in misses[:40]:
        L.append(f"- miss `0x{a:08x}` {g.name[a]} (reach.py: {reach.chain(g, full, a) if a in full else 'unreachable'})")
    for a in cov_miss[:20]:
        L.append(f"- coverage miss `0x{a:08x}` {g.name[a]}")
    L.append("")
    L.append("Not hooked, by what brings them in (top 40):")
    L.append("")
    for lab, n in label_count.most_common(40):
        L.append(f"- {n}: {lab}")
    L.append("")
    L.append(f"Classes the data names that have no descriptor in FFX.exe ({len(no_desc)}): "
             + ", ".join(no_desc))
    L.append("")
    L.append(f"Classes kept only for code use ({len(code_used)}; the first user found): "
             + ", ".join(f"{c} ({g.name[f]} [{g.module[f]}])" for c, f in sorted(code_used.items())))
    L.append("")
    L.append(f"Descriptor objects found for {len(desc_obj)} of {len(all_classes)} classes.")
    L.append("")
    L.append(f"Descriptor classes never named nor used ({len(never)}): " + ", ".join(never))
    (args.out / "phyre-classes.md").write_text("\n".join(L) + "\n", encoding="utf-8")
    print("\n".join(L[:40]))


if __name__ == "__main__":
    main()
