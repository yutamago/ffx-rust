#!/usr/bin/env python3
"""Which of a list's functions ffx-rust still reaches once the movie
libraries are replaced (M5 stage 2; symbols/m5-dropped.tsv evidence, tail9).

  FFX_GAME_DIR=... tools/reach-replaced.py <tsv: address in column 1>

Reachability in ffx-rust, where the 35 movie
library hooks (ported/movie.rs from "libvpx 1.4.0" on: ffx_movie's pure-Rust
decoders, which never call the original library code) are leaves.

Strict pointer roots: a vtable / data table / code-immediate / EH root counts
only once code referencing it is reachable (the table holding the pointer,
followed through data-to-data references up to depth 4; the instruction
taking the address; the function installing the EH funclet). Fixpoint.
Per dead row: which single leaf, expanded again, makes it reachable.

The strict rule treats a pointer table with holes (zero words between its
entries) as several tables, so a row it calls dead whose pointer sits in
such a table (the opus handler table 0x00c48ec8) must be checked by hand
with tools/refs.py; the same goes for addresses a ported function takes
(the strict rule follows the original's code, not the port's)."""
import sys, pathlib, collections, re
REPO = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "tools"))
import reach, progress, refs


def main():
    rows = [l.split("\t") for l in open(sys.argv[1]).read().splitlines() if l.strip()]
    targets = [int(r[0], 16) for r in rows]
    img = reach.Image(refs.default_exe())
    g = reach.Graph(img, 8, True)
    by_name = {n: a for a, n in g.name.items()}
    roots, code_imm, eh_edges, crt, nn = reach.find_roots(g, by_name)
    idx = refs.index()
    hooked = progress.hooks()
    src = (REPO / "crates/ffx-hook/src/ported/movie.rs").read_text()
    lib = src[src.index("---- libvpx 1.4.0"):]
    leaf = [int(m.replace("_", ""), 16) for m in re.findall(r"@ 0x([0-9a-f_]+) =>", lib)]
    leaves = {g.node_of(a) for a in leaf if hooked.get(a) == "replaced"}
    print("leaves", len(leaves), file=sys.stderr)
    strong = {"entry", "export", "tls", "crt-init", "native", "magic"}

    def table_extent(site):
        lo = site
        while (lo - 4) in img.reloc_set:
            lo -= 4
        hi = site
        while (hi + 4) in img.reloc_set:
            hi += 4
        return lo, hi

    memo = {}

    def code_enablers(site, depth=0):
        """nodes whose reachability makes the pointer at `site` live."""
        if img.in_text(site):
            n = g.node_of(site)
            return frozenset([n]) if n is not None else frozenset()
        lo, hi = table_extent(site)
        key = (lo, hi)
        if key in memo:
            return memo[key]
        memo[key] = frozenset()
        out = set()
        if depth < 4:
            for a in range(lo, hi + 4):
                for s, k in idx.to(a):
                    if img.in_text(s):
                        n = g.node_of(s)
                        if n is not None:
                            out.add(n)
                    else:
                        out |= code_enablers(s, depth + 1)
        memo[key] = frozenset(out)
        return memo[key]

    eh_owner = collections.defaultdict(set)
    for s, ds in eh_edges.items():
        for d in ds:
            eh_owner[d].add(s)
    pending = []
    for kind, node, site, _ in roots:
        if node is None:
            continue
        if kind in strong or site is None:
            pending.append((kind, node, site, None))
        elif kind in ("c++eh", "seh", "safeseh"):
            pending.append((kind, node, site, frozenset(eh_owner.get(node, ()))))
        else:
            pending.append((kind, node, site, code_enablers(site)))
    print("roots prepared", len(pending), file=sys.stderr)

    def run(stop):
        seen = {}
        q = collections.deque()

        def add(n, why):
            if n not in seen:
                seen[n] = why
                q.append(n)

        changed = True
        while changed:
            changed = False
            for kind, node, site, e in pending:
                if node in seen:
                    continue
                if e is None or any(x in seen and x not in stop for x in e):
                    add(node, (kind, site, None))
                    changed = True
            while q:
                v = q.popleft()
                if v in stop:
                    continue
                for w in g.out.get(v, ()):
                    add(w, ("call", None, v))
                for w in eh_edges.get(v, ()):
                    add(w, ("eh", None, v))
        return seen

    seen = run(leaves)
    full = run(set())
    print("seen", len(seen), "full", len(full), file=sys.stderr)
    dead = {g.node_of(a) for a in targets if g.node_of(a) not in seen}
    unlock = collections.defaultdict(list)
    for lf in sorted(leaves):
        s2 = run(leaves - {lf})
        for n in dead:
            if n in s2:
                unlock[n].append(lf)
    for a in targets:
        n = g.node_of(a)
        if n is None:
            print(f"{a:#010x}\t-\t-\t-\tNONODE")
            continue
        if n in seen:
            path = []
            v = n
            while v is not None and len(path) < 14:
                path.append(g.name[v])
                k, s_, p = seen[v]
                if p is None:
                    path.append(f"[{k}@{s_:#x}]" if s_ is not None else f"[{k}]")
                    break
                v = p
            print(f"{a:#010x}\t{g.name[n]}\t{g.module[n]}\t{g.size[n]}\tREACH\t" + " < ".join(path))
        else:
            callers = sorted(g.inn.get(n, ()))
            cs = ", ".join(g.name[c] for c in callers)
            ul = ", ".join(g.name[x] for x in unlock.get(n, []))
            print(f"{a:#010x}\t{g.name[n]}\t{g.module[n]}\t{g.size[n]}\tDEAD\t{n in full}\tunlock[{ul}]\tcallers[{cs}]")


if __name__ == '__main__':
    main()
