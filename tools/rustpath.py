"""Resolves the function a Rust call names, through its path, to the
definition it reaches (check-callwidth, check-callconv, check-thisarg).

The checks follow forwarding helpers to their callers. Taken by bare name,
a call reached every function of that name: when a second crate added a
`pub fn forward(h, target, conv, args)`, ffx-hook's calls of
`ffx_pcore::small::bridge::forward(h, sym, p, checked)` were checked as
calls of the new one (5 false problems, 2026-10-29), and a second and third
`fn this_call` made a right call path disappear in check-callconv
(b7efbac17). So a call is resolved the way rustc does, far enough for the
ports:

- the module tree: `crates/<crate>/src/a/b.rs` is `<crate>::a::b`
  (`mod.rs`, `lib.rs`, `main.rs` and `src/bin/*.rs` as Cargo lays them
  out; the `[lib] name` of Cargo.toml), plus inline `mod x { .. }`;
- a path's first segment: `crate`, `self`, `super`, `Self` (the
  enclosing impl's type), a `use` binding visible at the call (aliases,
  `{..}` groups, `self`, globs), a child module, a workspace crate, or a
  type (`Type::f`: the `impl Type` blocks' functions);
- the segments after it through child modules and re-exports (`pub use`,
  globs) of the module reached;
- a bare name: a function nested in an enclosing block, a function of
  the calling module, a `use` binding, then the glob imports.

`targets()` returns the definitions a call reaches as `(file, offset of its
fn keyword)`, an empty set when the path leaves the workspace (`std::..`,
another crate), or None when it can't tell (a macro-made module, a path it
doesn't follow); `reaches()` falls back to the bare name only then, and only
when one function in the workspace has it.
"""
import bisect
import os
import re

TOKEN = re.compile(
    r'//[^\n]*'
    r'|/\*.*?\*/'
    r'|\br(#*)"(?:.|\n)*?"\1'
    r'|b?"(?:\\.|[^"\\])*"'
    r"|b?'(?:\\.|\\u\{[0-9a-fA-F]+\}|[^\\'\n])'",
    re.S,
)
FN = re.compile(r'\bfn\s+(\w+)')
EXTERNAL = frozenset()


def blank(src):
    return TOKEN.sub(lambda m: re.sub(r'[^\n]', ' ', m.group(0)), src)


def is_type(seg):
    return seg[:1].isupper() and not seg.isupper()


def brace_pairs(code):
    """The `{..}` pairs sorted by their opening offset, and the index of
    each one's parent (-1 at the top)."""
    pairs, stack, parent = [], [], {}
    for m in re.finditer(r'[{}]', code):
        if m.group(0) == '{':
            parent[m.start()] = stack[-1] if stack else -1
            stack.append(m.start())
        elif stack:
            pairs.append((stack.pop(), m.start()))
    pairs.sort()
    index = {lo: i for i, (lo, _) in enumerate(pairs)}
    up = [index.get(parent.get(lo, -1), -1) for lo, _ in pairs]
    return pairs, up


def parse_use(text):
    """[(alias, [segments], glob)] of a use tree (`a::{b as c, d::*}`)."""
    toks = re.findall(r'\w+|::|[{},*]', text)
    out = []

    def tree(i, prefix):
        path = list(prefix)
        while i < len(toks):
            t = toks[i]
            if t == '::':
                i += 1
                continue
            if t == '{':
                i += 1
                while i < len(toks) and toks[i] != '}':
                    if toks[i] == ',':
                        i += 1
                        continue
                    i = tree(i, path)
                return i + 1
            if t == '*':
                out.append((None, path, True))
                return i + 1
            if t in ',}':
                break
            path.append(t)
            i += 1
            if i < len(toks) and toks[i] == '::':
                continue
            alias = None
            if i < len(toks) and toks[i] == 'as' and i + 1 < len(toks):
                alias = toks[i + 1]
                i += 2
            if path[-1] == 'self' and len(path) > 1:
                path = path[:-1]
            alias = alias or path[-1]
            if alias != '_':
                out.append((alias, path, False))
            return i
        return i

    tree(0, [])
    return out


class Use:
    __slots__ = ('alias', 'segs', 'glob', 'lo', 'hi', 'module', 'rel', 'off')

    def __init__(self, alias, segs, glob, lo, hi, module, rel, off):
        self.alias, self.segs, self.glob = alias, segs, glob
        self.lo, self.hi, self.module, self.rel, self.off = lo, hi, module, rel, off


class RFile:
    def __init__(self, rel, src, crate, base):
        self.rel = rel
        self.code = code = blank(src)
        self.crate = crate
        self.base = base
        self.pairs, self.up = brace_pairs(code)
        self.los = [lo for lo, _ in self.pairs]
        self.close = dict(self.pairs)
        # inline modules: (body lo, body hi, name)
        self.mods = []
        for m in re.finditer(r'\bmod\s+(\w+)\s*\{', code):
            lo = m.end() - 1
            if lo in self.close:
                self.mods.append((lo, self.close[lo], m.group(1)))
        self.mods.sort()
        # impl / trait items (not `impl Trait` types): body lo -> type name
        self.impls = {}
        for m in re.finditer(r'\b(impl|trait)\b', code):
            before = code[max(0, m.start() - 40):m.start()]
            if not re.search(r'(?:^|[;{}\]]|\bunsafe|\bpub(?:\s*\([\w\s:]*\))?|\bdefault|\bauto)\s*$', before):
                continue
            j = code.find('{', m.end())
            if j < 0 or j not in self.close:
                continue
            head = code[m.end():j]
            if ';' in head:
                continue
            self.impls[j] = self.impl_type(m.group(1), head)
        # functions: (name, start, body lo, body hi)
        self.fns = []
        for m in FN.finditer(code):
            j = m.end()
            depth = 0
            body = -1
            while j < len(code):
                c = code[j]
                if c in '(<[':
                    depth += 1
                elif c in ')]' or (c == '>' and code[j - 1] != '-'):
                    depth -= 1
                elif c == ';' and depth <= 0:
                    break
                elif c == '{' and depth <= 0:
                    body = j
                    break
                j += 1
            if body >= 0 and body in self.close:
                self.fns.append((m.group(1), m.start(), body, self.close[body]))
        self.fn_bodies = {lo for _, _, lo, _ in self.fns}
        self.uses = []
        for m in re.finditer(r'\buse\s+', code):
            if re.search(r'[\w.]$', code[max(0, m.start() - 1):m.start()]):
                continue
            j, depth = m.end(), 0
            while j < len(code):
                c = code[j]
                if c == '{':
                    depth += 1
                elif c == '}':
                    depth -= 1
                    if depth < 0:
                        break
                elif c == ';' and depth == 0:
                    break
                j += 1
            if j >= len(code) or code[j] != ';':
                continue
            lo, hi = self.block(m.start())
            module = self.module_at(m.start())
            for alias, segs, glob in parse_use(code[m.end():j]):
                if segs:
                    self.uses.append(Use(alias, segs, glob, lo, hi, module, rel, m.start()))

    @staticmethod
    def impl_type(kind, head):
        h = re.sub(r'\s+', ' ', head)
        h = re.split(r'\bwhere\b', h)[0]
        # drop generic arguments
        while True:
            g = re.sub(r'<[^<>]*>', '', h)
            if g == h:
                break
            h = g
        if kind == 'impl' and re.search(r'\bfor\b', h):
            h = re.split(r'\bfor\b', h)[-1]
        names = re.findall(r'\w+', h.replace('dyn ', '').replace('mut ', ''))
        names = [n for n in names if n not in ('unsafe', 'pub', 'crate', 'dyn', 'mut')]
        return names[-1] if names else None

    def blocks(self, off):
        """Indices of the `{..}` pairs holding `off`, innermost first."""
        i = bisect.bisect_right(self.los, off) - 1
        while i >= 0 and not (self.pairs[i][0] <= off < self.pairs[i][1]):
            i = self.up[i]
        while i >= 0:
            yield i
            i = self.up[i]

    def block(self, off):
        """The innermost `{..}` holding `off` (the file when none)."""
        for i in self.blocks(off):
            return self.pairs[i]
        return (0, len(self.code))

    def module_at(self, off):
        path = list(self.base)
        for lo, hi, name in self.mods:
            if lo < off < hi:
                path.append(name)
        return tuple(path)

    def module_scope(self, module):
        """The body of the inline module `module`, or the file."""
        if module == self.base:
            return (0, len(self.code))
        best = None
        for lo, hi, name in self.mods:
            if self.module_at(lo + 1) == module:
                best = (lo, hi)
                break
        return best

    def enclosing_fn(self, off):
        """The body (lo, hi) of the innermost fn holding `off`, or None."""
        for i in self.blocks(off):
            if self.pairs[i][0] in self.fn_bodies:
                return self.pairs[i]
        return None

    def in_fn_body(self, off):
        return self.enclosing_fn(off) is not None

    def impl_at(self, off):
        """(lo, hi, type) of the innermost impl/trait block holding `off`."""
        for i in self.blocks(off):
            lo, hi = self.pairs[i]
            if lo in self.impls:
                return (lo, hi, self.impls[lo])
        return None

    def is_local(self, off, name):
        """Whether `name` at `off` is a local value of the enclosing
        function (a parameter, `let`, closure parameter, `for` or
        `Some(..)` binding): a closure or fn pointer, not a fn item."""
        body = self.enclosing_fn(off)
        if body is None:
            return False
        lo = body[0]
        n = re.escape(name)
        sig = self.code[max(0, lo - 2000):lo]
        k = sig.rfind('fn ')
        if k >= 0 and re.search(r'(?<![\w.])(?:mut\s+)?' + n + r'\s*:(?!:)', sig[k:]):
            return True
        text = self.code[lo:off]
        if re.search(r'\blet\s+(?:mut\s+)?' + n + r'\b'
                     r'|\b(?:Some|Ok|Err)\s*\(\s*(?:mut\s+)?' + n + r'\s*\)'
                     r'|\bfor\s+(?:mut\s+)?' + n + r'\s+in\b', text):
            return True
        # closure parameters `|a, b: u32|` (not the `||` operator)
        if name not in text:
            return False
        for m in re.finditer(r'(?=\|([\w\s,:&\'*<>\[\];]{0,200})\|)', text):
            for p in m.group(1).split(','):
                p = re.sub(r'^\s*(?:&\s*)?(?:mut\s+)?', '', p.split(':')[0]).strip()
                if p == name and self.closure_holds(lo + m.start() + len(m.group(1)) + 2, off):
                    return True
        return False

    def closure_holds(self, k, off):
        """Whether the closure body starting at `k` (after its `|..|`)
        reaches `off`: a block to its `}`, else an expression to the `,`
        `;` or unmatched bracket ending it."""
        code = self.code
        m = re.match(r'\s*(?:->\s*[^{]*?)?\s*\{', code[k:k + 200])
        if m:
            hi = self.close.get(k + m.end() - 1, len(code))
            return off < hi
        depth = 0
        for j in range(k, off):
            c = code[j]
            if c in '([{':
                depth += 1
            elif c in ')]}':
                depth -= 1
                if depth < 0:
                    return False
            elif c in ',;' and depth == 0:
                return False
        return True


def crate_names(root, rel_dirs):
    """Crate directory -> the name code uses for it (`[lib] name`, else
    the package directory with `-` as `_`)."""
    out = {}
    for d in rel_dirs:
        name = d
        toml = os.path.join(root, 'crates', d, 'Cargo.toml') if root else None
        if toml and os.path.exists(toml):
            with open(toml, encoding='utf-8') as fh:
                text = fh.read()
            m = re.search(r'^\[lib\][^\[]*?^name\s*=\s*"([^"]+)"', text, re.M | re.S)
            if m:
                name = m.group(1)
        out[d] = name.replace('-', '_')
    return out


def file_base(rel, crate, has_lib):
    """The module path of `crates/<crate>/src/...`."""
    parts = rel.split('/')
    if len(parts) < 4 or parts[2] != 'src':
        return (crate, '<' + rel + '>')
    sub = parts[3:]
    sub[-1] = sub[-1][:-3] if sub[-1].endswith('.rs') else sub[-1]
    if sub[0] == 'bin':
        if len(sub) == 2:
            return ('<bin>' + sub[1],)
        rest = sub[2:]
        if rest[-1] in ('main', 'mod'):
            rest = rest[:-1]
        return ('<bin>' + sub[1],) + tuple(rest)
    if sub == ['lib']:
        return (crate,)
    if sub == ['main']:
        return ('<main>' + crate,) if has_lib else (crate,)
    if sub[-1] == 'mod':
        sub = sub[:-1]
    return (crate,) + tuple(sub)


class Workspace:
    def __init__(self, sources, root=None):
        """`sources`: {path relative to the root: source text}."""
        dirs = sorted({rel.split('/')[1] for rel in sources if rel.startswith('crates/')})
        names = crate_names(root, dirs)
        self.crates = set(names.values())
        libs = {rel.split('/')[1] for rel in sources if rel.endswith('/src/lib.rs')}
        self.files = {}
        for rel, src in sources.items():
            d = rel.split('/')[1] if rel.startswith('crates/') else rel
            crate = names.get(d, d.replace('-', '_'))
            self.files[rel] = RFile(rel, src, crate, file_base(rel, crate, d in libs))
        self.modules = set()
        self.mod_defs = {}    # module -> name -> [(rel, start)]
        self.mod_uses = {}    # module -> [Use] at its top level
        self.type_fns = {}    # type name -> name -> [(rel, start, impl lo)]
        self.by_name = {}     # name -> [(rel, start)]
        self.nested = {}      # rel -> [(block lo, block hi, name, start)]
        self.kind = {}        # (rel, start) -> 'mod' | 'impl' | 'nested'
        for f in self.files.values():
            for i in range(len(f.base)):
                self.modules.add(f.base[:i + 1])
            for lo, hi, name in f.mods:
                self.modules.add(f.module_at(lo + 1))
            for name, start, _, _ in f.fns:
                key = (f.rel, start)
                self.by_name.setdefault(name, []).append(key)
                blo, bhi = f.block(start)
                imp = f.impl_at(start)
                if imp is not None and imp[0] == blo:
                    self.kind[key] = 'impl'
                    self.type_fns.setdefault(imp[2], {}).setdefault(name, []).append((f.rel, start, imp[0]))
                elif f.in_fn_body(start):
                    self.kind[key] = 'nested'
                    self.nested.setdefault(f.rel, []).append((blo, bhi, name, start))
                else:
                    self.kind[key] = 'mod'
                    self.mod_defs.setdefault(f.module_at(start), {}).setdefault(name, []).append(key)
            for u in f.uses:
                scope = f.module_scope(u.module)
                if scope is not None and (u.lo, u.hi) == scope or u.lo == 0:
                    self.mod_uses.setdefault(u.module, []).append(u)
        self.memo = {}

    # --- bindings ---

    def visible_uses(self, rel, off):
        f = self.files[rel]
        module = f.module_at(off)
        return [u for u in f.uses if u.module == module and u.lo <= off < u.hi]

    def binding(self, uses, name):
        named = [u for u in uses if not u.glob and u.alias == name]
        globs = [u for u in uses if u.glob]
        return named, globs

    # --- paths ---

    def locate(self, segs, rel, off, module, uses, depth):
        """Where the module path `segs` leads from (rel, off) in `module`:
        ('mod', path), ('type', name, rel, impl lo or None), EXTERNAL or None."""
        if depth > 10:
            return None
        if not segs:
            return ('mod', module)
        first, rest = segs[0], segs[1:]
        if first == 'crate':
            cur = ('mod', module[:1])
        elif first == 'self':
            cur = ('mod', module)
        elif first == 'super':
            cur = ('mod', module[:-1] or module)
            while rest and rest[0] == 'super':
                cur = ('mod', cur[1][:-1] or cur[1])
                rest = rest[1:]
        elif first == 'Self':
            imp = self.files[rel].impl_at(off) if rel in self.files and off is not None else None
            cur = ('type', imp[2], rel, imp[0]) if imp and imp[2] else None
        else:
            cur = self.first(first, rel, module, uses, depth)
        for s in rest:
            if cur is None or cur is EXTERNAL:
                return cur
            if cur[0] == 'type':
                return None  # an associated item's item
            cur = self.child(cur[1], s, depth)
        return cur

    def first(self, name, rel, module, uses, depth):
        named, globs = self.binding(uses, name)
        for u in named:
            r = self.locate_use(u, depth + 1)
            if r is not None:
                return r
        if module + (name,) in self.modules:
            return ('mod', module + (name,))
        if (name,) in self.modules:
            return ('mod', (name,))
        for u in globs:
            g = self.locate_use(u, depth + 1, whole=True)
            if g and g is not EXTERNAL and g[0] == 'mod':
                r = self.child(g[1], name, depth + 1)
                if r is not None and r is not EXTERNAL:
                    return r
        if is_type(name):
            return ('type', name, rel, None)
        if name in ('std', 'core', 'alloc') or name not in self.crates and not named and not globs:
            return EXTERNAL
        return None

    def child(self, module, name, depth):
        if module + (name,) in self.modules:
            return ('mod', module + (name,))
        uses = self.mod_uses.get(module, [])
        named, globs = self.binding(uses, name)
        for u in named:
            r = self.locate_use(u, depth + 1)
            if r is not None:
                return r
        for u in globs:
            g = self.locate_use(u, depth + 1, whole=True)
            if g and g is not EXTERNAL and g[0] == 'mod':
                r = self.child(g[1], name, depth + 1)
                if r is not None and r is not EXTERNAL:
                    return r
        if is_type(name):
            return ('type', name, None, None)
        return None

    def locate_use(self, u, depth, whole=False):
        """The module or type a use binding's whole path names (a glob's
        module, or a module imported by name)."""
        key = ('use', u.rel, u.off, tuple(u.segs))
        if key in self.memo:
            return self.memo[key]
        self.memo[key] = None
        # a use path resolves in the use's own module (its sibling uses
        # included, but not itself)
        uses = [x for x in self.visible_uses(u.rel, u.off) if x is not u] or self.mod_uses.get(u.module, [])
        r = self.locate(u.segs, u.rel, u.off, u.module, uses, depth)
        self.memo[key] = r
        return r

    def fns_in(self, loc, name, depth, caller_rel=None):
        """The definitions `name` names in `loc`, EXTERNAL or None."""
        if loc is None or loc is EXTERNAL:
            return loc
        if loc[0] == 'mod':
            module = loc[1]
            defs = self.mod_defs.get(module, {}).get(name)
            if defs:
                return frozenset(defs)
            uses = self.mod_uses.get(module, [])
            named, globs = self.binding(uses, name)
            for u in named:
                r = self.fn_of_use(u, depth + 1)
                if r is not None:
                    return r
            for u in globs:
                g = self.locate_use(u, depth + 1, whole=True)
                r = self.fns_in(g, name, depth + 1) if g and g is not EXTERNAL else None
                if r:
                    return r
            # the module is known and has no such fn written out (a macro
            # makes it): no definition elsewhere is the one called
            return EXTERNAL
        _, t, rel, impl_lo = loc
        cands = self.type_fns.get(t, {}).get(name, [])
        if impl_lo is not None:
            # `Self::f`: the impl's own, else the type's other impls in the file
            own = [(r, s) for r, s, lo in cands if r == rel and lo == impl_lo]
            same = own or [(r, s) for r, s, lo in cands if r == rel]
            if same:
                return frozenset(same)
        if not cands:
            return None
        rel = rel or caller_rel
        here = [(r, s) for r, s, _ in cands if rel and r == rel]
        if here:
            return frozenset(here)
        crate = self.files[rel].crate if rel in self.files else None
        same = [(r, s) for r, s, _ in cands if crate and self.files[r].crate == crate]
        if same:
            return frozenset(same)
        if len({self.files[r].crate for r, _, _ in cands}) == 1:
            return frozenset((r, s) for r, s, _ in cands)
        return None

    def fn_of_use(self, u, depth):
        if depth > 10:
            return None
        if len(u.segs) == 1:
            return None
        key = ('fnuse', u.rel, u.off, tuple(u.segs))
        if key in self.memo:
            return self.memo[key]
        self.memo[key] = None
        uses = [x for x in self.visible_uses(u.rel, u.off) if x is not u] or self.mod_uses.get(u.module, [])
        loc = self.locate(u.segs[:-1], u.rel, u.off, u.module, uses, depth)
        r = self.fns_in(loc, u.segs[-1], depth, u.rel)
        self.memo[key] = r
        return r

    # --- calls ---

    def targets(self, rel, off, qual, name):
        """The definitions a call `qual name(..)` at `off` in `rel` reaches
        (`qual`: the path before the name, `a::b::` or ''): a frozenset of
        (rel, fn offset), EXTERNAL (empty) when it leaves the workspace, or
        None when it can't be told."""
        if rel not in self.files:
            return None
        segs = [s.strip() for s in qual.split('::') if s.strip()]
        key = ('call', rel, off, tuple(segs), name)
        if key in self.memo:
            return self.memo[key]
        f = self.files[rel]
        if re.search(r'::\s*$', f.code[max(0, off - 4):off]):
            return None  # `Type::<T>::f`, `<T as Tr>::f`: not followed
        module = f.module_at(off)
        uses = self.visible_uses(rel, off)
        r = None
        if segs:
            loc = self.locate(segs, rel, off, module, uses, 0)
            r = self.fns_in(loc, name, 0, rel)
        elif f.is_local(off, name):
            r = EXTERNAL  # a closure or fn pointer held in a local
        else:
            best = None
            for lo, hi, n, start in self.nested.get(rel, []):
                if n == name and lo <= off < hi and (best is None or lo > best[0]):
                    best = (lo, start)
            if best is not None:
                r = frozenset([(rel, best[1])])
            else:
                defs = self.mod_defs.get(module, {}).get(name)
                if defs:
                    r = frozenset(defs)
                else:
                    named, globs = self.binding(uses, name)
                    for u in named:
                        r = self.fn_of_use(u, 0)
                        if r is not None:
                            break
                    if r is None:
                        for u in globs:
                            g = self.locate_use(u, 0, whole=True)
                            x = self.fns_in(g, name, 0, rel) if g and g is not EXTERNAL else None
                            if x:
                                r = x
                                break
        self.memo[key] = r
        return r

    def aliases(self, name):
        """{file: {alias}} of the `use ..::name as alias` bindings."""
        if 'aliases' not in self.memo:
            out = {}
            for f in self.files.values():
                for u in f.uses:
                    if not u.glob and u.alias != u.segs[-1]:
                        out.setdefault(u.segs[-1], {}).setdefault(f.rel, set()).add(u.alias)
            self.memo['aliases'] = out
        return self.memo['aliases'].get(name, {})

    def known(self, rel, start):
        return (rel, start) in self.kind

    def reaches(self, rel, off, qual, name, target):
        """Whether the call reaches the definition `target` (rel, start):
        by its path; by the bare name only when the path can't be resolved
        and `target` is the workspace's only function of that name."""
        r = self.targets(rel, off, qual, name)
        if r is not None:
            return target in r
        return self.by_name.get(name) == [target]


def load(root, rels):
    sources = {}
    for rel in rels:
        with open(os.path.join(root, rel), encoding='utf-8') as fh:
            sources[rel] = fh.read()
    return Workspace(sources, root)
