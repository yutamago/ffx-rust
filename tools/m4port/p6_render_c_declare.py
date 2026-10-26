"""Reads the per-copy constants of `PClassDescriptorAbstract<T>`'s
constructor and the `PArray<T>` descriptor's `DeclareClassDescriptor`
(ffx_prender::declare) out of FFX.exe, for the p6-render-c copies.

Usage: FFX_GAME_DIR=... tools/m4port/p6_render_c_declare.py <addr>...
Prints one `kind addr rust-expression` line per copy; a copy that does
not follow the template is reported and skipped (the oracle tests
compare each copy with its own machine code anyway).
"""
import os
import re
import subprocess
import sys

EXE = os.path.join(os.environ['FFX_GAME_DIR'], 'FFX.exe')


def h(v):
    return f'0x{v >> 16:04x}_{v & 0xffff:04x}' if v > 0xffff else f'{v:#x}'


def disasm(a):
    out = subprocess.run(['objdump', '-d', '-M', 'intel', '--no-show-raw-insn',
                          f'--start-address={a:#x}', f'--stop-address={a + 0x400:#x}', EXE],
                         capture_output=True, text=True, check=True).stdout
    ins = []
    for l in out.splitlines():
        m = re.match(r'\s+([0-9a-f]+):\s+(.*)$', l)
        if not m:
            continue
        t = re.sub(r'\s+', ' ', m.group(2).strip())
        ins.append((int(m.group(1), 16), t))
        if t == 'ret' or t.startswith('ret '):
            break
    return ins


def imm(t):
    return int(t.split()[-1].split(',')[-1], 16)


def abstract(a):
    ins = disasm(a)
    calls = [i for i, (_, t) in enumerate(ins) if t.startswith('call')]
    tgt = [int(ins[i][1].split()[1], 16) for i in calls]
    if tgt[:3] != [0x435740, 0x43e2f0, 0x43b0a0]:
        raise ValueError(f'calls {[hex(x) for x in tgt]}')
    k = calls[1]
    name_t = ins[k - 1][1]
    size = imm(ins[k - 2][1])
    if ins[k - 3][1] != 'push 0x4':
        raise ValueError('alignment')
    if name_t.startswith('push DWORD PTR ds:'):
        name = f'dc::Name::Global({h(int(name_t.split(":")[1], 16))})'
    else:
        name = f'dc::Name::Image({h(imm(name_t))})'
    vt = [imm(t) for _, t in ins if t.startswith('mov DWORD PTR [esi],0x')]
    bufs = []
    for f in (0x43d510, 0x43d5b0, 0x43d5c0):
        i = calls[tgt.index(f)]
        j = i - 1
        while not ins[j][1].startswith('push 0x'):
            j -= 1
        bufs.append(imm(ins[j][1]))
    if len(vt) != 1 or len(tgt) != 6:
        raise ValueError('shape')
    return (f'dc::Abstract {{ name: {name}, size: {h(size)}, vtable: {h(vt[0])}, '
            f'default_buffer: {h(bufs[0])}, validation_buffer: {h(bufs[1])}, '
            f'write_mask_buffer: {h(bufs[2])} }}')


def declare(a):
    ins = disasm(a)
    text = [t for _, t in ins]
    guard = int(re.match(r'mov eax,ds:0x([0-9a-f]+)', text[11]).group(1), 16)
    desc = guard - 0x94
    i = text.index(f'mov ecx,{desc:#x}')
    ctor = int(text[i + 2].split()[1], 16)
    pushes_atexit = []
    for j, t in enumerate(text):
        if t == 'call 0x949207':
            k = j - 1
            while not text[k].startswith('push 0x'):
                k -= 1
            pushes_atexit.append(imm(text[k]))
    vt = imm(next(t for t in text if t.startswith(f'mov DWORD PTR ds:{desc:#x},')))
    name = int(next(t for t in text if t.startswith('mov ecx,DWORD PTR ds:')).split(':')[1], 16)
    j = text.index(f'push {0xb0fd78:#x}')
    flags = imm(text[j - 2])
    if text[j + 1].startswith('push 0x'):
        if text[j + 2] != f'push {desc:#x}':
            raise ValueError('element')
        elem = f'dc::Element::Descriptor({h(imm(text[j + 1]))})'
    elif text[j + 1] == 'push eax' and text[j - 5].startswith('call 0x'):
        elem = f'dc::Element::Getter({h(int(text[j - 5].split()[1], 16))})'
    else:
        raise ValueError('element ' + text[j + 1])
    mvt = imm(next(t for t in text if t.startswith(f'mov DWORD PTR ds:{desc + 0xe0:#x},')))
    st = imm(next(t for t in text if t.startswith(f'mov DWORD PTR ds:{desc + 0x11c:#x},')))
    if len(pushes_atexit) != 4:
        raise ValueError('atexit')
    ax = ', '.join(h(x) for x in pushes_atexit)
    return (ctor, f'dc::Declare {{ desc: {h(desc)}, ctor: {h(ctor)}, vtable: {h(vt)}, '
            f'name: {h(name)}, element: {elem}, flags: {flags}, member_vtable: {h(mvt)}, '
            f'set_storage: {h(st)}, atexit: [{ax}] }}')


for s in sys.argv[1:]:
    a = int(s, 16)
    first = disasm(a)
    try:
        if any(t == 'call 0x43b0a0' for _, t in first):
            print('abstract', hex(a), abstract(a))
        else:
            ctor, d = declare(a)
            print('declare', hex(a), hex(ctor), d)
    except (ValueError, StopIteration, IndexError, AttributeError) as e:
        print('SKIP', hex(a), e, file=sys.stderr)
