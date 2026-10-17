#!/usr/bin/env python3
"""x87-lift: lift straight-line x87 code into Rust statements, as a first
draft for porting float-heavy functions exactly.

Usage:
    tools/x87-lift.py CONFIG.json > draft.rs

The input is a disassembly listing, one instruction per line in Intel
syntax as `  <hex address>: <mnemonic> <operands>` (capstone's text form;
anything after `;` is ignored). CONFIG.json names it and describes the
function:

    {
      "asm": "plate.asm",
      "entries": {"1": "0x8f5f9f", "4 | 5": "0x8f6ccd"},
      "frame": {"8": ["f32", "x"], "24": ["u32", "kind"]},
      "fpu": ["f64::from(h)", "img32(mem, 0x00b5_4f84)"],
      "calls": {"0x903ee0": 11, "0x641060": 0},
      "st0_calls": {"0x6447e0": "remap(mem, false, {0})"},
      "eax_calls": ["0x641060"]
    }

- `entries`: the start addresses to lift, keyed by the match-arm label
  printed before each (for a switch, the case labels).
- `frame`: the `[ebp + off]` slots at entry (decimal offsets) with their
  type (`f32`, `f64` or `u32`) and Rust name.
- `fpu`: the x87 stack at entry, bottom first, as Rust f64 expressions.
- `calls`: the argument count of every called function; `st0_calls` give a
  Rust template (`{0}` is the first argument, an f32) for functions that
  return in ST0, and `eax_calls` those whose eax is tested afterwards.

The output models the x87 at the game's 53-bit precision: register values
are `f64` expressions, `fst/fstp dword` stores round to `f32` (`let vN`),
qword stores keep `f64`. Conditional jumps become `if`/`else` (the code
must be loop-free; shared tails are duplicated). Image constants come out
as `img32(mem, addr)`/`img64(mem, addr)`/`image_f32(mem, addr)` calls.

The draft is exact but not readable. Before it goes into a crate, find
the structure (shared formulas, per-case tables), rewrite it by hand and
keep the rounding points. `crates/ffx-system/src/plate.rs` is the worked
example (TODrawMenuPlateXYWHType).

Supported: fld/fst/fstp (dword/qword memory, st(i)), fldz, fld1,
fadd/fsub/fsubr/fmul/fdiv/fdivr (memory or st(i) into st0), fxch, fcomp +
fnstsw + test ah,0x41 + je/jne, push imm/ecx/esi, sub/add esp, call,
cmp esi/test eax + je/jne, jmp, pop/mov epilogues and ret. Anything else
stops with an error naming the instruction.
"""

import sys, re
import json, os
cfg = json.load(open(sys.argv[1]))
base = os.path.dirname(os.path.abspath(sys.argv[1]))
lines = open(os.path.join(base, cfg['asm'])).read().splitlines()
ins = {}
order = []
for l in lines:
    m = re.match(r'\s+([0-9a-f]+): (\S+)\s*(.*?)(\s+;.*)?$', l)
    if not m: continue
    a = int(m.group(1), 16); ins[a] = (m.group(2), m.group(3).strip()); order.append(a)
nxt = {order[i]: order[i+1] for i in range(len(order)-1)}
ARITY = {int(k, 16): v for k, v in cfg.get('calls', {}).items()}
ST0 = {int(k, 16): v for k, v in cfg.get('st0_calls', {}).items()}
EAX = {int(k, 16) for k in cfg.get('eax_calls', [])}
def hx(v): 
    s = f'{v:08x}'; return f'0x{s[:4]}_{s[4:]}'
class St:
    def __init__(s):
        s.fpu = []; s.esp = 0; s.stack = {}; s.frame = {}; s.esi = 'kind'; s.eax = None
    def copy(s):
        n = St(); n.fpu = list(s.fpu); n.esp = s.esp; n.stack = dict(s.stack); n.frame = dict(s.frame); n.esi = s.esi; n.eax = s.eax; return n
counter = [0]
def fresh(p):
    counter[0] += 1; return f'{p}{counter[0]}'
def memref(st, op):
    m = re.match(r'(dword|qword) ptr \[(.*)\]', op)
    size, inner = m.group(1), m.group(2).replace(' ', '')
    if inner.startswith('esp'):
        off = int(inner[3:], 16) if len(inner) > 3 else 0
        return size, ('stack', st.esp + off)
    if inner.startswith('ebp'):
        off = int(inner[3:], 16) if len(inner) > 3 else 0
        return size, ('frame', off)
    return size, ('abs', int(inner, 16))
def load(st, out, size, ref):
    kind, k = ref
    if kind == 'abs':
        return f'{"img32" if size=="dword" else "img64"}(mem, {hx(k)})'
    slot = (st.stack if kind == 'stack' else st.frame).get(k)
    if slot is None: raise Exception(f'undefined {ref}')
    t, v = slot
    if size == 'dword':
        if t == 'f32': return f'f64::from({v})'
        if t == 'u32': return f'f64::from(f32::from_bits({v}))'
        raise Exception(f'size mismatch {ref} {slot}')
    if t == 'f64': return v
    raise Exception(f'qword of {slot}')
def store(st, out, size, ref, expr, ind):
    kind, k = ref
    if size == 'dword':
        if re.fullmatch(r'f64::from\((\w+)\)', expr):
            v = re.fullmatch(r'f64::from\((\w+)\)', expr).group(1)
        else:
            v = fresh('v')
            r = re.fullmatch(r'img32\(mem, (\w+)\)', expr)
            if expr == '0.0': out.append(f'{ind}let {v} = 0.0f32;')
            elif expr == '1.0': out.append(f'{ind}let {v} = 1.0f32;')
            elif r: out.append(f'{ind}let {v} = image_f32(mem, {r.group(1)});')
            else: out.append(f'{ind}let {v} = {expr} as f32;' if re.fullmatch(r'\w+', expr) else f'{ind}let {v} = ({expr}) as f32;')
        val = ('f32', v)
    else:
        if re.fullmatch(r'[a-z]\d+', expr): v = expr
        else:
            v = fresh('d'); out.append(f'{ind}let {v}: f64 = {expr};')
        val = ('f64', v)
    (st.stack if kind == 'stack' else st.frame)[k] = val
def arg(st, off):
    t, v = st.stack.get(st.esp + off, (None, None))
    if t is None: raise Exception(f'undefined arg {off}')
    if t == 'f32': return f'{v}.to_bits()'
    return v
OPS = {'fadd': '{a} + {b}', 'fsub': '{a} - {b}', 'fsubr': '{b} - {a}', 'fmul': '{a} * {b}', 'fdiv': '{a} / {b}', 'fdivr': '{b} / {a}'}
def run(a, st, out, ind):
    while True:
        m, op = ins[a]
        n = nxt.get(a)
        if m == 'fld':
            r = re.match(r'st\((\d)\)', op)
            if r: st.fpu.append(st.fpu[-1-int(r.group(1))])
            else:
                size, ref = memref(st, op); st.fpu.append(load(st, out, size, ref))
        elif m == 'fldz': st.fpu.append('0.0')
        elif m == 'fld1': st.fpu.append('1.0')
        elif m in ('fst', 'fstp'):
            r = re.match(r'st\((\d)\)', op)
            if r:
                i = int(r.group(1)); st.fpu[-1-i] = st.fpu[-1]
            else:
                size, ref = memref(st, op); e = st.fpu[-1]
                store(st, out, size, ref, e, ind)
                # reuse the rounded value
                if size == 'dword':
                    t, v = (st.stack if ref[0]=='stack' else st.frame)[ref[1]]
                    if m == 'fst': st.fpu[-1] = e
            if m == 'fstp': st.fpu.pop()
        elif m in OPS:
            r = re.match(r'st\((\d)\)$', op)
            if r: b = st.fpu[-1-int(r.group(1))]
            else:
                size, ref = memref(st, op); b = load(st, out, size, ref)
            x = st.fpu[-1]
            par = lambda t: t if re.fullmatch(r'[\w.:]+(\([^()]*\))?', t) else f'({t})'
            e = OPS[m].format(a=par(x), b=par(b))
            v = fresh('x'); out.append(f'{ind}let {v}: f64 = {e};'); st.fpu[-1] = v
        elif m == 'fxch':
            i = int(re.match(r'st\((\d)\)', op).group(1)); st.fpu[-1], st.fpu[-1-i] = st.fpu[-1-i], st.fpu[-1]
        elif m == 'push':
            st.esp -= 4
            if re.fullmatch(r'-?0x[0-9a-f]+|-?\d+', op):
                st.stack[st.esp] = ('u32', hx(int(op, 0) & 0xffffffff))
            elif op == 'ecx': st.stack.pop(st.esp, None)
            elif op in ('esi',): st.stack[st.esp] = ('u32', st.esi)
            else: raise Exception(f'push {op}')
        elif m == 'sub' and op.startswith('esp'):
            k = int(op.split(',')[1], 0)
            for o in range(st.esp - k, st.esp, 4): st.stack.pop(o, None)
            st.esp -= k
        elif m == 'add' and op.startswith('esp'):
            st.esp += int(op.split(',')[1], 0)
        elif m == 'call':
            t = int(op, 16)
            if t in ST0:
                v = fresh('r'); a0 = arg(st, 0); a0 = a0[:-len('.to_bits()')] if a0.endswith('.to_bits()') else f'f32::from_bits({a0})'
                out.append(f'{ind}let {v} = {ST0[t].format(a0)};')
                st.fpu.append(f'f64::from({v})')
            else:
                args = ', '.join(arg(st, 4*i) for i in range(ARITY[t]))
                if t in EAX:
                    v = fresh('e'); out.append(f'{ind}let {v} = call(mem, {hx(t)}, &[{args}]);'); st.eax = v
                else:
                    out.append(f'{ind}call(mem, {hx(t)}, &[{args}]);')
        elif m == 'cmp' and op.startswith('esi'):
            st.cmp = ('esi', int(op.split(',')[1], 0))
        elif m == 'test' and op == 'eax, eax':
            st.cmp = ('eax',)
        elif m == 'fcomp':
            size, ref = memref(st, op); b = load(st, out, size, ref); x = st.fpu.pop()
            st.cmp = ('fcomp', x, b)
        elif m == 'fnstsw': pass
        elif m == 'test' and op == 'ah, 0x41': pass
        elif m in ('je', 'jne', 'ja'):
            tgt = int(op, 16)
            c = st.cmp
            if c[0] == 'esi': cond = f'kind == {c[1]}'
            elif c[0] == 'eax': cond = f'{st.eax} == 0'
            else: cond = f'!({c[1]} > {c[2]})'  # je taken when C0 and C3 clear: st0 > m
            if c[0] == 'fcomp': cond = f'({c[1]}) > ({c[2]})'
            taken, fall = [], []
            run(tgt, st.copy(), taken, ind + '    ')
            run(n, st.copy(), fall, ind + '    ')
            if m == 'jne':
                taken, fall = fall, taken
            if m == 'ja': raise Exception('ja')
            if not taken:
                neg = cond.replace(' == ', ' != ') if c[0] in ('esi', 'eax') else f'({c[1]}) <= ({c[2]}) || ({c[1]}).is_nan() || ({c[2]}).is_nan()'
                out.append(f'{ind}if {neg} {{'); out.extend(fall); out.append(f'{ind}}}')
            elif not fall:
                out.append(f'{ind}if {cond} {{'); out.extend(taken); out.append(f'{ind}}}')
            else:
                out.append(f'{ind}if {cond} {{'); out.extend(taken)
                out.append(f'{ind}}} else {{'); out.extend(fall); out.append(f'{ind}}}')
            return
        elif m == 'jmp':
            n = int(op, 16)
        elif m in ('pop', 'mov') and op in ('esi', 'ebp', 'esp, ebp'): pass
        elif m == 'ret':
            return
        else:
            raise Exception(f'{a:x}: {m} {op}')
        a = n
for label, entry in cfg['entries'].items():
    st = St()
    st.frame = {int(k): tuple(v) for k, v in cfg.get('frame', {}).items()}
    st.fpu = list(cfg.get('fpu', []))
    out = []
    run(int(entry, 16), st, out, '            ')
    while True:
        text = '\n'.join(out); changed = False
        for i, l in enumerate(out):
            mm = re.match(r'\s*let (\w+)(: f64)? = ', l)
            if mm and not l.strip().startswith('let r') and len(re.findall(r'\b' + mm.group(1) + r'\b', text)) == 1:
                out.pop(i); changed = True; break
        if not changed: break
    print(f'        {label} => {{')
    print('\n'.join(out))
    print('        }')
