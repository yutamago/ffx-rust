#!/usr/bin/env python3
"""Finds calls into original code wider than the game's host passes
(crates/ffx-bridge/src/lib.rs, `Host`, "Call width").

  tools/check-callwidth.py [-v] [--root DIR]

In the game, `ffx_bridge::Host::call` (`RawMem`) passes at most 6 words and
refuses a longer list without calling (the result is 0). A bridge that
calls an original with 7 or more words through it does nothing in the game
while its parity test can still pass on a host that takes any width
(Atel_TurnUpdate's 8-word Math_YawPitchTo call, event-cliff, 2026-10-24).
The oracle's test hosts now refuse such calls, but only on the paths the
tests reach; this scan reads every call statically.

Limits (what each in-game host passes):

  ffx_bridge::Host::call(f, args)                  6  (refused: 0)
  NativeHost::call_float(f, args)                  6  (refused: NaN)
  NativeHost::call_n(f, args)                     12  (refused: 0)
  ffx_graphic Host::call/call_sym(f, conv, args)   8; 12 for Conv::Thiscall
  ffx_graphic Host::call_st0/call_sym_st0          8
  ffx_graphic Host::call_virtual(obj, slot, args) 12
  ffx-hook call_cdecl / _f64 / _f32 / _u64        12  (logged, cut short)
  ffx-hook call_cdecl_words                       32  (refused: 0)
  ffx-hook call_thiscall(f, this, args)            8  (refused: 0)

For every call of these in crates/*/src (ffx-oracle and ffx-studio left
out) the width of the argument slice is worked out: a literal (`&[a, b]`,
`&[0; N]`, `vec![..]`), a local or const array, a sub-slice of one (bounded
by the array), or a parameter of the enclosing function. A function that
passes its own slice parameter on (`fn call(h, sym, args: &[u32]) {
h.call(h.rebase(sym), args) }`, also with words put in front:
`let mut v = vec![fmt]; v.extend_from_slice(args)`) becomes a limited
callee itself, and its callers are checked the same way, to a fixed point;
so does a local closure that forwards its slice parameter. Loop tables
(`for (f, args) in [(F, &[..][..]), ..]` or a const of such tuples), array
`.map(..)`, `[..][..n]`, `[..].concat()`, `if .. { [..] } else { [..] }`
and functions returning `[u32; N]` are followed too. A slice whose width
can't be worked out is reported as dynamic; when a human has bounded it,
a comment `call width: N` (N the most words it can hold) on the call's
line or one of the three lines above, or an entry in REVIEWED below,
settles it. A REVIEWED entry that no longer matches is reported.

Calls through closures the shims hand the bridges (`call: &mut impl
FnMut(&mut M, u32, &[u32]) -> u32`) are checked against what the shims
pass for that bridge: the forwarding closure's own limit (`|_, f, a|
call_thiscall(f, this, a)`: 8; `call_cdecl_words`: 32), the narrowest when
several shims call it, else 12. Inner helpers the bridge hands its closure
on to get 12.

Prints one line per problem and "N problems"; exits 1 when there are any.
-v also lists every resolved call with its width.
"""
import argparse
import bisect
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SKIP_CRATES = {'ffx-oracle', 'ffx-studio'}

# method name -> (args before the slice, limit, kind); limit None: by conv
METHODS = {
    'call': None,  # 2 args: ffx_bridge (6); 3 args: ffx_graphic (by conv)
    'call_sym': (2, 'conv', 'graphic Host::call_sym'),
    'call_st0': (2, 8, 'graphic Host::call_st0'),
    'call_sym_st0': (2, 8, 'graphic Host::call_sym_st0'),
    'call_virtual': (2, 12, 'graphic Host::call_virtual'),
    'call_float': (1, 6, 'NativeHost::call_float'),
    'call_n': (1, 12, 'NativeHost::call_n'),
}
# free functions of ffx-hook's ported/mod.rs
FREE = {
    'call_cdecl': (1, 12, 'call_cdecl'),
    'call_cdecl_f64': (1, 12, 'call_cdecl_f64'),
    'call_cdecl_f32': (1, 12, 'call_cdecl_f32'),
    'call_cdecl_u64': (1, 12, 'call_cdecl_u64'),
    'call_cdecl_words': (1, 32, 'call_cdecl_words'),
    'call_thiscall': (2, 8, 'call_thiscall'),
}
# where the hosts themselves are defined: their bodies are the limits
HOST_FILES = {
    'crates/ffx-bridge/src/lib.rs',
    'crates/ffx-graphic/src/host.rs',
    'crates/ffx-script/src/vm/bridge/run.rs',
    'crates/ffx-sg/src/scratch.rs',
}
CLOSURE_LIMIT = 12
# dynamic slices a human has bounded: (file, enclosing fn ('?' outside
# one), the slice as written) -> (most words, why). An entry no dynamic
# call matches any more is reported, so the table can't go stale.
REVIEWED = {
    ('crates/ffx-hook/src/ported/map.rs', 'rcbg_guide_update_ex', 'words'):
        (6, 'FArgs::Stack slices are literals of at most 6 words (ffx-field)'),
    ('crates/ffx-menu/src/tk.rs', 'tk_mn2_buy_sell_window', '&[&[0xc8], &rect[..], &arrow_uv].concat()'):
        (9, '1 + rect [u32; 4] + uv [u32; 4]'),
    ('crates/ffx-menu/src/tk.rs', 'tk_mn2_buy_sell_window', '&[&[0xc8], &rect[..], &uv].concat()'):
        (9, '1 + rect [u32; 4] + uv [u32; 4]'),
    ('crates/ffx-menu/src/tk.rs', 'tk_mn2_item_require_window', '&[&[u32::MAX], &rect[..], &uv].concat()'):
        (9, '1 + rect [u32; 4] + uv [u32; 4]'),
    ('crates/ffx-menu/src/tk.rs', 'tk_mn2_item_require_window', '&[&[200], &rect[..], &uv].concat()'):
        (9, '1 + rect [u32; 4] + uv [u32; 4]'),
    ('crates/ffx-menu/src/tk.rs', 'scroll_list_rows', '&args'):
        (6, 'vec of 5 + an Option'),
    ('crates/ffx-pcui/src/menus.rs', 'show_quit_pop_up', '&args'):
        (14, '2 + three [u32; 4] values (call_cdecl_words, 32)'),
    ('crates/ffx-pcui/src/menus.rs', 'show_incomplete_pop_up', '&args'):
        (22, '2 + five [u32; 4] values (call_cdecl_words, 32)'),
    ('crates/ffx-script/src/vm/bridge/native_map.rs', 'pop_call', '&v'):
        (4, 'n from pop_calls!, at most 4'),
    ('crates/ffx-script/src/vm/bridge/native_map.rs', 'set_fog_param', '&p'):
        (3, 'fog_params returns [u32; 3]'),
    ('crates/ffx-script/src/vm/bridge/native_move.rs', 'native_get_pitch_to_move_target', '&args'):
        (8, '2 + 3 + 3 pushes'),
    ('crates/ffx-script/src/vm/bridge/native_sys.rs', 'pop_mixed', '&v'):
        (4, 'one word per kind; mixed_calls! kinds have at most 4'),
    ('crates/ffx-sound/src/ffxfmod.rs', 'sfx_start_with', 'start.1'):
        (3, 'callers pass literals of at most 3 words'),
    ('crates/ffx-system/src/cmdwin.rs', 'to_btl_draw_command_window', '&piece'):
        (12, 'ripple_row returns [[u32; 12]; 8]'),
    ('crates/ffx-system/src/magic.rs', 'magic_file_preload_prx', '&args'):
        (5, 'stream + print words, at most 4'),
    ('crates/ffx-system/src/tolib.rs', 'btl_get_change_pad', '&a'):
        (2, 'the args closure returns [u32; 2]'),
    ('crates/ffx-system/src/tolib.rs', 'to_draw_scale_button_icon_rgba', '&with(if c & 1 != 0 { 0x259 } else { 0x258 })'):
        (6, 'with() returns icon, [u32; 6]'),
    ('crates/ffx-system/src/tolib.rs', 'to_draw_scale_button_icon_rgba', '&with(if c & 2 != 0 { 0x25b } else { 0x25a })'):
        (6, 'with() returns icon, [u32; 6]'),
    ('crates/ffx-system/src/tolib.rs', 'to_draw_scale_button_icon_rgba', '&with(if c & 4 != 0 { 0x25d } else { 0x25c })'):
        (6, 'with() returns icon, [u32; 6]'),
    ('crates/ffx-system/src/tolib.rs', 'to_draw_scale_button_icon_rgba', '&with(if c & 8 != 0 { 0x25f } else { 0x25e })'):
        (6, 'with() returns icon, [u32; 6]'),
    ('crates/ffx-system/src/tolib.rs', 'to_draw_scale_button_icon_rgba', '&with(id)'):
        (6, 'with() returns icon, [u32; 6]'),
    ('crates/ffx-system/src/tolib.rs', 'to_draw_scale_button_icon_rgba', '&with(c + 0x230)'):
        (6, 'with() returns icon, [u32; 6]'),
    ('crates/ffx-system/src/tolib.rs', 'to_draw_scale_button_icon_rgba', '&with(0x270)'):
        (6, 'with() returns icon, [u32; 6]'),
    ('crates/ffx-vfx/src/mdl_cam.rs', 'bmp_triangle', '&args'):
        (7, 'format + three f64 as two words each'),
    ('crates/ffx-vfx/src/op.rs', 'op_segment_circle_cross', '&args'):
        (9, 'p [f32; 7] + 2'),
    ('crates/ffx-vfx/src/ppp.rs', 'ppp_data_rcv', '&a'):
        (5, 'format + print words, at most 4'),
}
ANNOT = re.compile(r'call width:\s*(\d+)')


TOKEN = re.compile(
    r'//[^\n]*'
    r'|/\*.*?\*/'
    r'|\br(#*)"(?:.|\n)*?"\1'
    r'|b?"(?:\\.|[^"\\])*"'
    r"|b?'(?:\\.|\\u\{[0-9a-fA-F]+\}|[^\\'\n])'",
    re.S,
)


def blank(src):
    """`src` with comments, strings and char literals turned to spaces
    (newlines kept, so offsets and lines stay)."""
    return TOKEN.sub(lambda m: re.sub(r'[^\n]', ' ', m.group(0)), src)


def bracket_map(code):
    """For each opening bracket's offset, its closing bracket's."""
    pairs = {'(': ')', '[': ']', '{': '}'}
    close, stack = {}, []
    for m in re.finditer(r'[()\[\]{}]', code):
        c, k = m.group(0), m.start()
        if c in pairs:
            stack.append((pairs[c], k))
        elif stack and stack[-1][0] == c:
            close[stack.pop()[1]] = k
        else:
            stack = []  # unbalanced (a macro): start over
    return close


def match_close(code, i):
    """The index of the bracket closing the one at `i`."""
    pairs = {'(': ')', '[': ']', '{': '}'}
    stack = [pairs[code[i]]]
    j = i + 1
    while j < len(code):
        c = code[j]
        if c in pairs:
            stack.append(pairs[c])
        elif c in ')]}':
            if not stack or c != stack[-1]:
                return -1
            stack.pop()
            if not stack:
                return j
        j += 1
    return -1


def split_top(s):
    """`s` split at the commas outside brackets (and closure bars)."""
    parts, depth, cur = [], 0, []
    for c in s:
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
        if c == ',' and depth == 0:
            parts.append(''.join(cur))
            cur = []
        else:
            cur.append(c)
    last = ''.join(cur)
    if last.strip():
        parts.append(last)
    return [p.strip() for p in parts]


def eval_int(s, consts):
    s = s.strip().replace('_', '')
    if re.fullmatch(r'\d+(usize|u32)?', s):
        return int(re.match(r'\d+', s).group(0))
    if re.fullmatch(r'0x[0-9a-fA-F]+(usize|u32)?', s):
        return int(re.match(r'0x[0-9a-fA-F]+', s).group(0), 16)
    name = s.split('::')[-1]
    if name in consts:
        return consts[name]
    return None


class Fn:
    def __init__(self, name, start, body_start, end, params, line):
        self.name, self.start, self.body_start, self.end = name, start, body_start, end
        self.params, self.line = params, line  # params: [(name, type)], self left out


class File:
    def __init__(self, rel, src):
        self.rel, self.src = rel, src
        self.code = blank(src)
        self.close = bracket_map(self.code)
        self.lines = src.split('\n')
        self.line_starts = [0] + [m.end() for m in re.finditer('\n', src)]
        self.fns = self.find_fns()
        self.fns.sort(key=lambda f: f.body_start)
        self.words = set(re.findall(r'\w+', self.code))
        self.fn_names = {f.name for f in self.fns}
        # a nested fn is seen only inside its parent; the rest file-wide
        for fn in self.fns:
            parent = self.enclosing(fn.start)
            fn.scope = (parent.body_start, parent.end) if parent else (0, len(self.code))
        self.top_names = {f.name for f in self.fns if f.scope[0] == 0}
        self.stem = os.path.splitext(os.path.basename(rel))[0]
        if self.stem in ('mod', 'lib'):
            self.stem = os.path.basename(os.path.dirname(rel))
        self.mods = {self.stem} | set(re.findall(r'\bmod\s+(\w+)\s*\{', self.code))
        self.uses = ' '.join(re.findall(r'\buse\s+[^;]+;', self.code))
        self.consts = {}
        for m in re.finditer(r'\bconst\s+(\w+)\s*:\s*usize\s*=\s*([^;]+);', self.code):
            v = eval_int(m.group(2), {})
            if v is not None:
                self.consts[m.group(1)] = v
        self.arrays = {}
        for m in re.finditer(r'\b(?:const|static)\s+(\w+)\s*:\s*&?\[\s*u32\s*;\s*([^\]]+)\]', self.code):
            v = eval_int(m.group(2), self.consts)
            if v is not None:
                self.arrays[m.group(1)] = v
        # functions returning `[u32; N]`
        self.rets = {}
        for fn in self.fns:
            if fn.ret is not None:
                v = eval_int(fn.ret, self.consts)
                if v is not None:
                    self.rets[fn.name] = v

    def line(self, off):
        return bisect.bisect_right(self.line_starts, off)

    def match(self, i):
        return self.close.get(i, -1)

    def find_fns(self):
        fns = []
        code = self.code
        for m in re.finditer(r'\bfn\s+(\w+)\s*', code):
            j = m.end()
            if j < len(code) and code[j] == '<':
                depth = 0
                while j < len(code):
                    if code[j] == '<':
                        depth += 1
                    elif code[j] == '>' and code[j - 1] != '-':
                        depth -= 1
                        if depth == 0:
                            j += 1
                            break
                    j += 1
                while j < len(code) and code[j].isspace():
                    j += 1
            if j >= len(code) or code[j] != '(':
                continue
            pe = self.match(j)
            if pe < 0:
                continue
            params = []
            has_self = False
            for p in split_top(code[j + 1:pe]):
                if ':' not in p or p.replace('mut ', '').strip().startswith('self'):
                    has_self = has_self or 'self' in p
                    continue  # self, &mut self
                name, ty = p.split(':', 1)
                name = name.replace('mut ', '').strip()
                params.append((name, ty.strip()))
            # body: the first '{' at depth 0 before a ';'
            k = pe + 1
            depth = 0
            body = -1
            while k < len(code):
                c = code[k]
                if c in '([<':
                    depth += 1
                elif c in ')]>' and not (c == '>' and code[k - 1] == '-'):
                    depth -= 1
                elif c == ';' and depth <= 0:
                    break
                elif c == '{' and depth <= 0:
                    body = k
                    break
                k += 1
            if body < 0:
                continue
            end = self.match(body)
            if end < 0:
                continue
            fn = Fn(m.group(1), m.start(), body, end, params, self.line(m.start()))
            fn.has_self = has_self
            rm = re.search(r'->\s*\[\s*u32\s*;\s*([^\]]+)\]', code[pe:body])
            fn.ret = rm.group(1) if rm else None
            fns.append(fn)
        return fns

    def visible(self, name, off):
        """The definition of `name` a call at `off` in this file reaches."""
        best = None
        for fn in self.fns:
            if fn.name == name and fn.scope[0] <= off < fn.scope[1]:
                if best is None or fn.scope[0] > best.scope[0]:
                    best = fn
        return best

    def enclosing(self, off):
        best = None
        for f in self.fns:
            if f.body_start < off < f.end and (best is None or f.body_start > best.body_start):
                best = f
        return best

    def annotation(self, off):
        ln = self.line(off)
        for k in range(max(0, ln - 4), ln):
            m = ANNOT.search(self.lines[k])
            if m:
                return int(m.group(1))
        return None


def literal_width(e, consts):
    """Words in an array literal `[..]` / `vec![..]`, or None."""
    e = e.strip()
    if e.startswith('vec!'):
        e = e[4:].strip()
    if not e.startswith('[') or match_close(e, 0) != len(e) - 1:
        return None
    inner = e[1:-1]
    parts = split_top(inner)
    # `[x; N]`
    depth = 0
    for k, c in enumerate(inner):
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
        elif c == ';' and depth == 0:
            return eval_int(inner[k + 1:], consts)
    return len(parts)


RETS = {}
# widths of the enclosing function's locals, set by width_of for concat
LOCALS = {}


def expr_width(e, consts, rets=None):
    w = expr_width1(e, consts)
    if w is not None:
        return w
    e = e.strip()
    if e.startswith('&'):
        return expr_width(e[1:], consts, rets)
    # `[..][..n]`: at most the literal
    if e.startswith('['):
        q = match_close(e, 0)
        if q > 0 and re.fullmatch(r'\s*\[[^\]]*\.\.[^\]]*\]', e[q + 1:]):
            return literal_width(e[:q + 1], consts)
        # `[&a, &b[..], &[c]].concat()`: the parts' sum
        if q > 0 and re.fullmatch(r'\s*\.\s*concat\s*\(\s*\)', e[q + 1:]):
            total = 0
            for part in split_top(e[1:q]):
                part = re.sub(r'^&\s*', '', part.strip())
                part = re.sub(r'\[\s*\.\.\s*\]$', '', part)
                w = expr_width(part, consts, rets)
                if w is None and re.fullmatch(r'\w+', part) and part in LOCALS:
                    w = LOCALS[part]
                if w is None:
                    return None
                total += w
            return total
    if e.startswith('{') and match_close(e, 0) == len(e) - 1:
        last = split_stmts(e[1:-1])[-1]
        return expr_width(last, consts, rets)
    m = re.match(r'(?:\w+::)*(\w+)\s*\(', e)
    if m and match_close(e, m.end() - 1) == len(e) - 1:
        name = m.group(1)
        if rets and name in rets:
            return rets[name]
        if name in RETS:
            return RETS[name]
    return None


def split_stmts(s):
    """`s` split at the semicolons outside brackets."""
    out, depth, cur = [], 0, []
    for c in s:
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
        if c == ';' and depth == 0:
            out.append(''.join(cur))
            cur = []
        else:
            cur.append(c)
    out.append(''.join(cur))
    return [x.strip() for x in out]


def expr_width1(e, consts):
    """Words of an array-valued expression: a literal, a literal's
    `.map(..)`, `if .. { A } else { B }` (the wider), `pops::<N>(..)`;
    None when unknown."""
    e = e.strip()
    n = literal_width(e, consts)
    if n is not None:
        return n
    if e.startswith('['):
        q = match_close(e, 0)
        if q > 0 and re.match(r'\s*\.\s*map\s*\(', e[q + 1:]) and match_close(e, e.index('(', q)) == len(e) - 1:
            return literal_width(e[:q + 1], consts)
    m = re.match(r'(?:\w+::)*\w+\s*::\s*<\s*(\w+)\s*>\s*\(', e)
    if m and match_close(e, m.end() - 1) == len(e) - 1:
        return eval_int(m.group(1), consts)
    if e.startswith('if '):
        widths = []
        k = 0
        while True:
            b = e.find('{', k)
            if b < 0:
                return None
            q = match_close(e, b)
            if q < 0:
                return None
            inner = e[b + 1:q].strip()
            last = inner.rsplit(';', 1)[-1].strip()
            w = expr_width(last, consts)
            if w is None:
                return None
            widths.append(w)
            rest = e[q + 1:].strip()
            if not rest:
                return max(widths)
            if not rest.startswith('else'):
                return None
            k = q + 1
    return None


def closure_param(f, fn, name, off):
    """(closure's `let` name or None, index of `name` among its parameters)
    when `name` is a parameter of a closure around `off`."""
    lo = fn.body_start if fn else 0
    lo = max(lo, off - 2000)
    body = f.code[lo:off]
    # every `|..|` pair, overlapping ones too (`a | b` is no closure): a
    # closure's bars hold only patterns, and the first follows `(`, `,`,
    # `=`, `move` or the start of an expression
    for m in reversed(list(re.finditer(r'(?=\|([^|]*)\|)', body))):
        if not re.search(r'(^|[(,=]|\bmove|&mut|&|\{|;)\s*$', body[:m.start()]):
            continue
        parts = split_top(re.sub(r'<[^<>|]*>', '', re.sub(r'<[^<>|]*>', '', m.group(1))))
        params = [x.split(':')[0].replace('mut ', '').replace('&', '').strip() for x in parts]
        if not all(re.fullmatch(r'\w+|\(.*\)|\[.*\]', x) for x in params):
            continue
        if name in params:
            before = body[:m.start()]
            lm = re.search(r'\blet\s+(?:mut\s+)?(\w+)\s*=\s*(move\s+)?$', before)
            return (lm.group(1) if lm else None, params.index(name), lo + m.start())
        if off - (lo + m.start()) > 600:
            break
    return None


def table_init(files_by_name, f, ident):
    """The initializer of `const`/`static` `ident` (this file first)."""
    pat = re.compile(r'\b(?:const|static)\s+' + re.escape(ident) + r'\s*:[^=]*=\s*')
    for g in [f] + files_by_name.get(ident, []):
        m = pat.search(g.code)
        if m:
            k = m.end()
            while k < len(g.code) and g.code[k] in '& ':
                k += 1
            if k < len(g.code) and g.code[k] == '[':
                q = g.match(k)
                if q > 0:
                    return g.code[k:q + 1], g.consts
    return None


TABLES = {}  # const name -> files defining it


def for_binding(f, fn, name, off):
    """The widest slice `name` takes in `for (.., name, ..) in TABLE`, where
    TABLE is an array literal of tuples or a const/static one."""
    lo = fn.body_start if fn else max(0, off - 4000)
    body = f.code[lo:off]
    for m in reversed(list(re.finditer(r'\bfor\s+(\(.*?\))\s+in\s+', body))):
        pat = [x.replace('&', '').replace('mut ', '').strip() for x in split_top(m.group(1)[1:-1])]
        if name not in pat:
            continue
        # the loop must still be open at `off`
        k = lo + m.end()
        expr_end = f.code.find('{', k)
        if expr_end < 0 or f.match(expr_end) < off:
            continue
        expr = f.code[k:expr_end].strip()
        expr = re.sub(r'^&\s*', '', expr)
        expr = re.sub(r'\.(iter|into_iter)\(\)$', '', expr)
        consts = f.consts
        if not expr.startswith('['):
            ident = expr.split('::')[-1]
            if not re.fullmatch(r'\w+', ident):
                return None
            got = table_init(TABLES, f, ident)
            if got is None:
                return None
            expr, consts = got
        idx = pat.index(name)
        widest = 0
        for el in split_top(expr[1:-1]):
            el = el.strip()
            if not (el.startswith('(') and match_close(el, 0) == len(el) - 1):
                return None
            parts = split_top(el[1:-1])
            if idx >= len(parts):
                return None
            w = expr_width(parts[idx], consts)
            if w is None:
                return None
            widest = max(widest, w)
        return Width(widest)
    return None


class Width:
    """A slice's width: `n` words, or `param` (index, offset) of the
    enclosing function, or dynamic (`why`)."""

    def __init__(self, n=None, param=None, why=None, closure=None):
        self.n, self.param, self.why, self.closure = n, param, why, closure


def width_of(f, fn, expr, off):
    e = expr.strip()
    e = re.sub(r'^&\s*(mut\s+)?', '', e)
    e = re.sub(r'\.as_slice\(\)$|\.as_ref\(\)$|\.as_mut_slice\(\)$', '', e)
    consts = f.consts
    LOCALS.clear()
    if e.startswith('[') and '.concat()' in e:
        for part in re.findall(r'&\s*(\w+)\s*(?:\[\s*\.\.\s*\])?\s*[,\]]', e):
            w = width_of(f, fn, part, off)
            if w.n is not None:
                LOCALS[part] = w.n
    n = expr_width(e, consts, f.rets)
    if n is not None:
        return Width(n)
    sub = False
    m = re.fullmatch(r'(\w+)\s*\[([^\]]*)\]', e)
    if m and '..' in m.group(2):
        lo, _, hi = m.group(2).partition('..')
        hi = hi.lstrip('=')
        a0 = eval_int(lo, consts) if lo.strip() else 0
        b0 = eval_int(hi, consts) if hi.strip() else None
        if a0 is not None and b0 is not None:
            return Width(b0 - a0 + (1 if '..=' in m.group(2) else 0))
        e, sub = m.group(1), True
    if not re.fullmatch(r'\w+', e):
        return Width(why=f'slice `{expr.strip()}`')
    name = e
    if fn is None:
        # outside any fn (a macro's arms): the text before the call
        lo = max(0, off - 3000)
        fn = Fn('?', lo, lo, off + 1, [], f.line(lo))
    cp = closure_param(f, fn, name, off)
    if fn is not None:
        body = f.code[fn.body_start:off]
        # the last `let name ... = ...;` before the call
        lets = list(re.finditer(r'\blet\s+(?:mut\s+)?' + re.escape(name) + r'\s*(:\s*(?:\[[^\]]*\]|[^=;\[])+)?=(?!=)', body))
        if cp is not None and (not lets or fn.body_start + lets[-1].start() < cp[2]):
            return Width(closure=cp)
        if lets:
            lm = lets[-1]
            ty = lm.group(1) or ''
            tm = re.search(r'\[\s*u32\s*;\s*([^\]]+)\]', ty)
            if tm and eval_int(tm.group(1), consts) is not None:
                return Width(eval_int(tm.group(1), consts))
            rhs_start = lm.end()
            # the statement's end: the ';' at depth 0
            k, depth = rhs_start, 0
            while k < len(body):
                c = body[k]
                if c in '([{':
                    depth += 1
                elif c in ')]}':
                    depth -= 1
                elif c == ';' and depth == 0:
                    break
                k += 1
            rhs = body[rhs_start:k].strip()
            base = expr_width(rhs, consts, f.rets)
            mm = re.fullmatch(r'(\w+)\s*(?:\.\s*map\s*\((.*)\))?', rhs, re.S)
            if base is None and mm and mm.group(1) != name:
                w = width_of(f, fn, mm.group(1), fn.body_start + lm.start())
                base = w.n
            if base is None and rhs in ('Vec::new()', 'vec![]'):
                base = 0
            if base is None:
                return Width(why=f'`{name}` = `{" ".join(rhs.split())[:60]}`')
            rest = body[k:]
            grows = list(re.finditer(re.escape(name) + r'\.(push|extend_from_slice|extend|insert|resize|truncate)\s*\(', rest))
            if not grows:
                return Width(base)
            if len(grows) == 1 and grows[0].group(1) == 'extend_from_slice':
                a = rest[grows[0].end():]
                am = re.match(r'\s*&?\s*(\w+)\s*\)', a)
                if am:
                    w = width_of(f, fn, am.group(1), off)
                    if w.n is not None:
                        return Width(base + w.n)
                    if w.param is not None:
                        return Width(param=(w.param[0], w.param[1] + base))
            return Width(why=f'`{name}` grows at run time')
        for k, (pname, ty) in enumerate(fn.params):
            if pname == name:
                tm = re.search(r'\[\s*u32\s*;\s*([^\]]+)\]', ty)
                if tm and eval_int(tm.group(1), consts) is not None:
                    return Width(eval_int(tm.group(1), consts))
                if re.search(r'&\s*(\'\w+\s+)?(mut\s+)?\[\s*u32\s*\]', ty):
                    if sub:
                        return Width(why=f'sub-slice of parameter `{name}`')
                    return Width(param=(k, 0))
                return Width(why=f'parameter `{name}: {ty}`')
    if name in f.arrays:
        return Width(f.arrays[name])
    w = for_binding(f, fn, name, off)
    if w is not None:
        return w
    if cp is not None:
        return Width(closure=cp)
    return Width(why=f'`{name}` not found')


def call_sites(f, name, method):
    """(offset of the name, args list, args offsets) for each call."""
    code = f.code
    pat = (r'\.\s*' if method else r'(?<![\w.])(?:[\w]+::)*') + re.escape(name) + r'\s*(::<[^>]*>)?\s*\('
    for m in re.finditer(pat, code):
        if not method and re.search(r'\bfn\s+$', code[max(0, m.start() - 8):m.start()]):
            continue
        p = m.end() - 1
        q = f.match(p)
        if q < 0:
            continue
        yield m.start(), split_top(code[p + 1:q])


def graphic_limit(conv):
    c = conv.replace(' ', '')
    if re.search(r'Conv::Thiscall', c):
        return 12, 'thiscall'
    if re.search(r'Conv::(Cdecl|Stdcall|Fastcall)$', c):
        return 8, c.split('::')[-1].lower()
    return 12, f'conv `{conv}`'


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('-v', '--verbose', action='store_true')
    ap.add_argument('--root', default=ROOT)
    a = ap.parse_args()

    files = []
    crates = os.path.join(a.root, 'crates')
    for crate in sorted(os.listdir(crates)):
        if crate in SKIP_CRATES:
            continue
        src = os.path.join(crates, crate, 'src')
        for dp, _, fs in os.walk(src):
            for fn in sorted(fs):
                if fn.endswith('.rs'):
                    p = os.path.join(dp, fn)
                    rel = os.path.relpath(p, a.root)
                    with open(p, encoding='utf-8') as fh:
                        files.append(File(rel, fh.read()))

    by_rel = {f.rel: f for f in files}
    used_reviews = set()
    for f in files:
        for m in re.finditer(r'\b(?:const|static)\s+(\w+)\s*:\s*&?\s*(?:\'static\s+)?\[\s*\(', f.code):
            TABLES.setdefault(m.group(1), []).append(f)
    closure_limits = {}  # bridge fn name -> (limit, kind) of the closures shims pass
    for f in files:
        for name, v in f.rets.items():
            RETS[name] = max(v, RETS.get(name, v))
    problems = []
    seen = set()
    resolved = []
    wrappers = {}  # (file rel, fn name) -> (index, limit, kind)

    def check(f, off, args, idx, limit, kind):
        if idx >= len(args):
            return
        fn = f.enclosing(off)
        ann = f.annotation(off)
        key = (f.rel, off, kind)
        w = width_of(f, fn, args[idx], off)
        where = f'{f.rel}:{f.line(off)}'
        inside = fn.name if fn else '?'
        if w.param is not None and fn is not None:
            pidx, extra = w.param
            if fn.name in METHODS and f.rel in HOST_FILES:
                return
            k = (f.rel, fn.name, fn.start)
            new = (pidx, limit - extra, f'{fn.name} -> {kind}')
            old = wrappers.get(k)
            if old is None or new[1] < old[1]:
                wrappers[k] = new
            return
        if w.closure is not None:
            let, pidx, at = w.closure
            if let is not None and fn is not None:
                # a local forwarding closure: its calls in this function
                c = Fn(let, at, at, at, [(f'_{i}', '') for i in range(pidx + 1)], f.line(at))
                c.has_self, c.scope = False, (at, fn.end)
                f.fns.append(c)
                f.fn_names.add(let)
                f.words.add(let)
                k = (f.rel, let, at)
                if k not in wrappers:
                    wrappers[k] = (pidx, limit, f'{let} -> {kind}')
                return
            # a closure a shim hands to a bridge: the bridge's calls through
            # it get this limit
            k = at
            while k > 0:
                k = f.code.rfind('(', 0, k)
                if k < 0 or f.match(k) > at:
                    break
            bm = re.search(r'(\w+)\s*(::<[^>]*>)?\s*$', f.code[max(0, k - 120):k]) if k > 0 else None
            if bm:
                b = bm.group(1)
                if limit < closure_limits.get(b, (99, ''))[0]:
                    closure_limits[b] = (limit, kind)
            return
        if key in seen:
            return
        seen.add(key)
        if w.n is None and ann is not None:
            w = Width(ann)
        slice_text = ' '.join(args[idx].split())
        if w.n is None and (f.rel, inside, slice_text) in REVIEWED:
            w = Width(REVIEWED[(f.rel, inside, slice_text)][0])
            used_reviews.add((f.rel, inside, slice_text))
        if w.n is None:
            problems.append(f'{where}: in {inside}: {kind} with a dynamic width ({w.why}); '
                            f'the game passes at most {limit} (bound it with a `call width: N` comment, '
                            f'or REVIEWED[({f.rel!r}, {inside!r}, {slice_text!r})])')
        elif w.n > limit:
            problems.append(f'{where}: in {inside}: {kind} with {w.n} words; the game passes at most {limit}')
        else:
            resolved.append(f'{where}: in {inside}: {kind} {w.n}/{limit}')

    for f in files:
        in_host = f.rel in HOST_FILES
        for name, spec in METHODS.items():
            for off, args in call_sites(f, name, True):
                fn = f.enclosing(off)
                if in_host and fn is not None and (fn.name in METHODS or fn.name.startswith('call')):
                    # the hosts' own forwarding (call_sym -> call, ...)
                    w = width_of(f, fn, args[-1], off) if args else Width()
                    if w.param is not None:
                        continue
                if name == 'call':
                    if len(args) == 2:
                        check(f, off, args, 1, 6, 'Host::call')
                    elif len(args) == 3:
                        lim, conv = graphic_limit(args[1])
                        check(f, off, args, 2, lim, f'graphic Host::call ({conv})')
                    continue
                idx, lim, kind = spec
                if lim == 'conv':
                    if len(args) < 3:
                        continue
                    lim, conv = graphic_limit(args[1])
                    kind = f'{kind} ({conv})'
                check(f, off, args, idx, lim, kind)
        if f.rel.startswith('crates/ffx-hook/'):
            for name, (idx, lim, kind) in FREE.items():
                for off, args in call_sites(f, name, False):
                    fn = f.enclosing(off)
                    if f.rel.endswith('ported/mod.rs') and fn is not None and fn.name in FREE:
                        continue
                    check(f, off, args, idx, lim, kind)
    def settle():
        # wrappers to a fixed point
        done = set()
        while True:
            todo = [(k, v) for k, v in wrappers.items() if (k, v) not in done]
            if not todo:
                break
            for (rel, name, start), (idx, lim, kind) in todo:
                done.add(((rel, name, start), (idx, lim, kind)))
                home = by_rel[rel]
                wfn = next(fn for fn in home.fns if fn.start == start)
                unique = sum(name in g.fn_names for g in files) == 1
                for f in files:
                    if name not in f.words:
                        continue
                    for method in (False, True):
                        for off, args in call_sites(f, name, method):
                            if f is home:
                                target = f.visible(name, off)
                                if target is not wfn and not (method and wfn.has_self and unique):
                                    continue
                            elif method:
                                # a method elsewhere: only a name no other fn has
                                if not (wfn.has_self and unique):
                                    continue
                            else:
                                if wfn.scope[0] != 0 or wfn.has_self or name in f.top_names:
                                    continue
                                qual = f.code[off:f.code.index(name, off)]
                                segs = [q for q in qual.split('::') if q.strip()]
                                if segs:
                                    if segs[-1].strip() not in home.mods:
                                        continue
                                elif not re.search(r'\b(' + '|'.join(sorted(home.mods)) + r')\b[^;]*\b' + re.escape(name) + r'\b', f.uses):
                                    continue
                            check(f, off, args, idx, lim, kind)

    settle()

    # closures handed in by the shims: `call(h, f, &[..])` through a
    # parameter typed FnMut(.., &[u32]) -> .., checked against what the
    # shims pass for that bridge (CLOSURE_LIMIT when none is found)
    for f in files:
        for fn in f.fns:
            for pname, ty in fn.params:
                tm = re.search(r'Fn(Mut|Once)?\s*\(', ty)
                if tm:
                    tq = match_close(ty, tm.end() - 1)
                    tys = split_top(ty[tm.end():tq]) if tq > 0 else []
                    sidx = next((i for i, t in enumerate(tys)
                                 if re.fullmatch(r"&\s*('\w+\s+)?(mut\s+)?\[\s*u32\s*\]", t)), None)
                    if sidx is None:
                        tys = None
                elif re.search(r'\bCall\s*<', ty):
                    tys, sidx = [0, 0, 0], 2
                else:
                    tys = None
                if tys:
                    body_code = f.code[fn.body_start:fn.end]
                    for m in re.finditer(r'(?<![\w.])\(?\s*\*?\s*' + re.escape(pname) + r'\s*\)?\s*\(', body_code):
                        p = fn.body_start + m.end() - 1
                        q = f.match(p)
                        if q < 0:
                            continue
                        args = split_top(f.code[p + 1:q])
                        if len(args) != len(tys):
                            continue
                        inner = f.enclosing(fn.body_start + m.start())
                        if inner is not fn:
                            continue
                        lim, via = closure_limits.get(fn.name, (CLOSURE_LIMIT, 'call_cdecl'))
                        check(f, fn.body_start + m.start(), args, sidx, lim,
                              f'closure `{pname}` ({via})')
    settle()

    if a.verbose:
        for r in sorted(set(resolved)):
            print('ok', r)
        for (rel, name, _), (idx, lim, kind) in sorted(wrappers.items()):
            print(f'wrapper {rel}: {name} (slice argument {idx}) at most {lim}: {kind}')
    for k in sorted(set(REVIEWED) - used_reviews):
        problems.append(f'REVIEWED entry {k} matches no dynamic call: remove it or update it')
    for p in sorted(set(problems)):
        print(p)
    print(f'{len(set(problems))} problems ({len(set(resolved))} calls checked, '
          f'{len(wrappers)} forwarding functions)')
    return 1 if problems else 0


if __name__ == '__main__':
    sys.exit(main())
