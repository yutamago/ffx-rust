#!/usr/bin/env python3
"""Lists the `ported!` shims ffx-host can't run from its table yet
(docs/host.md, "One source of truth"), into
crates/ffx-host-ports/src/held.rs.

  tools/host-ports.py            # rewrite held.rs
  tools/host-ports.py --check    # exit 1 when held.rs is out of date
  tools/host-ports.py -v         # also print why each shim is held

ffx-host compiles the hook's shims a second time (crates/ffx-host-ports).
Calls into original code go through ffx_bridge::route and the per-call
frames through StackFrame, so both work there. What doesn't: a shim that
hands the game the address of a Rust local or a thread-local (`&raw mut
x`, `buf.as_mut_ptr()`, `ptr::from_mut(&mut x)`, `expose_provenance`, `addr_of_mut!`, a
`thread_local!` buffer's `.get()`/`.as_ptr()`): on x86_64 that address is
outside the 32-bit arena, and even on i686 Unicorn doesn't map the host
stack. Such a shim stays out of the table (its original runs in Unicorn)
until the local becomes a `StackFrame`.

A shim is held when its body, or a function of the same file its body
names (transitively), takes such an address. Rerun after merging new
`ported!` lines; check-ported runs --check.
"""
import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
PORTED = ROOT / 'crates/ffx-hook/src/ported'
OUT = ROOT / 'crates/ffx-host-ports/src/held.rs'

HOST_ADDR = re.compile(
    r'\.as_mut_ptr\(\)|\.as_ptr\(\)|&raw\s+(?:mut|const)\b|\baddr_of(?:_mut)?!|'
    r'expose_provenance|\bthread_local!|\.get\(\)\s+as\s+usize|\bptr::from_(?:ref|mut)\b|'
    r'\bas\s+\*(?:const|mut)\b')
ENTRY = re.compile(
    r'(\w+)\s*@\s*(0x[0-9a-fA-F_]+)\s*=>\s*extern\s+"[^"]+"\s+fn\s+(\w+)\s*\(')
FN = re.compile(r'(?<![\w.])fn\s+(\w+)\s*(?:<[^>{]*>)?\s*\(')
MACRO = re.compile(r'\bmacro_rules!\s*(\w+)\s*')
IDENT = re.compile(r'\b[a-z_][a-z0-9_]*\b')


def strip(code):
    """Comments and string/char literals blanked (same length)."""
    out = []
    i, n = 0, len(code)
    while i < n:
        c = code[i]
        if code.startswith('//', i):
            j = code.find('\n', i)
            j = n if j < 0 else j
            out.append(' ' * (j - i))
            i = j
        elif code.startswith('/*', i):
            j = code.find('*/', i + 2)
            j = n if j < 0 else j + 2
            out.append(re.sub(r'[^\n]', ' ', code[i:j]))
            i = j
        elif c == '"' or code.startswith('r"', i) or code.startswith('r#"', i):
            if c == 'r':
                hashes = 1 if code.startswith('r#"', i) else 0
                end = '"' + '#' * hashes
                j = code.find(end, i + 2 + hashes)
                j = n if j < 0 else j + len(end)
            else:
                j = i + 1
                while j < n and code[j] != '"':
                    j += 2 if code[j] == '\\' else 1
                j += 1
            out.append('"' + ' ' * (j - i - 2) + '"')
            i = j
        elif c == "'" and re.match(r"'(?:\\.|[^\\'])'", code[i:i + 4]):
            m = re.match(r"'(?:\\.|[^\\'])'", code[i:i + 4])
            out.append("' '".ljust(len(m.group(0))))
            i += len(m.group(0))
        else:
            out.append(c)
            i += 1
    return ''.join(out)


def block(code, at):
    """The span of the `{...}` block opening at or after `at`."""
    i = code.index('{', at)
    depth = 0
    for j in range(i, len(code)):
        if code[j] == '{':
            depth += 1
        elif code[j] == '}':
            depth -= 1
            if depth == 0:
                return i, j + 1
    return i, len(code)


def params_end(code, at):
    """Just past the `)` closing the parameter list whose `(` ends at `at`."""
    depth = 1
    for j in range(at, len(code)):
        if code[j] == '(':
            depth += 1
        elif code[j] == ')':
            depth -= 1
            if depth == 0:
                return j + 1
    return len(code)


def scan(path):
    """[(addr, name, reasons)] for the file's held shims."""
    code = strip(path.read_text())
    entries = []
    spans = []
    for m in ENTRY.finditer(code):
        b0, b1 = block(code, m.end())
        entries.append((int(m.group(2).replace('_', ''), 16), m.group(1), b0, b1))
        spans.append((m.start(), b1))
    fns = {}
    for m in FN.finditer(code):
        if any(a <= m.start() < b for a, b in spans):
            continue
        try:
            b0, b1 = block(code, m.end())
        except ValueError:
            continue
        # a `;` between the parameter list and the body: a declaration (the
        # parameters' own `[u32; 8]` don't count)
        semi = code.find(';', params_end(code, m.end()))
        if 0 <= semi < b0:
            continue  # a declaration (trait method) without a body
        fns.setdefault(m.group(1), []).append(code[b0:b1])
    # a `macro_rules!` body counts as a function of the macro's name: a shim
    # that invokes `name!` names it (battle.rs's `script!` hands every shim
    # `regist_signal`, which took a local's address and slipped through)
    for m in MACRO.finditer(code):
        b0, b1 = block(code, m.end())
        fns.setdefault(m.group(1), []).append(code[b0:b1])
    direct = {name for name, bodies in fns.items() if any(HOST_ADDR.search(b) for b in bodies)}
    tainted = set(direct)
    changed = True
    while changed:
        changed = False
        for name, bodies in fns.items():
            if name in tainted:
                continue
            if any(set(IDENT.findall(b)) & tainted for b in bodies):
                tainted.add(name)
                changed = True
    held = []
    for addr, name, b0, b1 in entries:
        body = code[b0:b1]
        why = []
        m = HOST_ADDR.search(body)
        if m:
            why.append(m.group(0))
        why += sorted(set(IDENT.findall(body)) & tainted)
        if why:
            held.append((addr, name, why))
    return held, len(entries)


def render(held):
    lines = [
        '//! The `ported!` shims ffx-host leaves out of its table: each hands the',
        "//! game the address of a Rust local or thread-local, which the arena and",
        '//! Unicorn can\'t reach. Generated by `tools/host-ports.py`; rerun it after',
        '//! merging `ported!` lines (check-ported checks it is current).',
        '',
        '/// Their addresses, sorted.',
        'pub const HELD: &[u32] = &[',
    ]
    for addr, name, _ in sorted(held):
        lines.append(f'    0x{addr >> 16:04x}_{addr & 0xffff:04x}, // {name}')
    lines.append('];')
    return '\n'.join(lines) + '\n'


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--check', action='store_true')
    ap.add_argument('-v', '--verbose', action='store_true')
    a = ap.parse_args()
    held, total = [], 0
    for path in sorted(PORTED.glob('*.rs')):
        if path.name == 'mod.rs':
            continue
        h, n = scan(path)
        total += n
        held += h
        if a.verbose:
            for addr, name, why in h:
                print(f'{path.name}: {name} @ {addr:#010x}: {", ".join(why)}')
    text = render(held)
    if a.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f'{OUT.relative_to(ROOT)} is out of date: run tools/host-ports.py')
            return 1
        print(f'held.rs current: {len(held)} of {total} shims held')
        return 0
    OUT.write_text(text)
    print(f'{len(held)} of {total} shims held, {total - len(held)} in the table')
    return 0


if __name__ == '__main__':
    sys.exit(main())
