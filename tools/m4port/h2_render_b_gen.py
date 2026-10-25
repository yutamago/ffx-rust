"""Generates the h2-render-b shims (crates/ffx-hook/src/ported/h2_render_b.rs) and
oracle cases (crates/ffx-oracle/tests/h2_render_b/cases.rs) from the spec files
(tools/m4port/h2_render_b_spec*.py); run `cargo fmt` afterwards.

Usage: tools/m4port/h2_render_b_gen.py tools/m4port/h2_render_b_spec*.py

The bridges themselves (crates/ffx-prender/src/descriptor.rs, misc.rs, ...) are
hand-written; this only emits the glue: one `ported!` line and one test case
per function, both making the same bridge call.

Each spec row: (addr, abi, kinds, ret, body, callees, prep)
  abi: 'C' cdecl, 'S' stdcall, 'T' thiscall (first kind is `this`)
  kinds: one char per argument: p pointer, n nullable pointer, v value,
         f float bits, c small count, b byte/flags, i small index
  ret: 'u32', 'u8' or '' (eax untouched / not read)
  body: Rust expression over `M` (the host) and a0..aN
  callees: [(addr, stack words, pops)] stubbed with recording stubs
  prep: name of a prep fn in the test file, or None
  callees: (addr, words, pops[, k]): k = the stub stores a word through
         stack argument k (an out-pointer)
  gap (optional, after frame): uncompared bytes between the frame and ebp
  frame (optional): bytes of the original's frame below ebp that callees
         see; the body names the port's frame FRAME (a StackFrame in the
         shim, the original's frame address in the test)
"""
import re, sys, importlib.util, pathlib

R = pathlib.Path(__file__).resolve().parent.parent.parent
SYM = {}
for l in open(R / 'symbols/ffx.sym'):
    p = l.split()
    if len(p) >= 3 and p[0].startswith('0x'):
        SYM[int(p[0], 16)] = p[2]

spec_files = sys.argv[1:]
rows = []
for f in spec_files:
    s = importlib.util.spec_from_file_location('spec', f)
    m = importlib.util.module_from_spec(s)
    s.loader.exec_module(m)
    rows += m.SPEC

CDECL = {0x0042fb20, 0x004e4740, 0x004e4ec0, 0x004dd770, 0x004e4df0, 0x004f4fc0, 0x00540b80,
         0x0056a700, 0x00588190, 0x004395b0, 0x004dd910, 0x0058a9a0, 0x004ec310, 0x004dd860,
         0x00949256, 0x00949207, 0x00437fc0, 0x00435740, 0x0043e2f0, 0x004d0e20, 0x0049d050,
         0x004c8a70, 0x0058ab60, 0x004ffe40, 0x004352f0, 0x0058a8f0, 0x00560e90}
ABI = {'C': '"C"', 'S': '"stdcall"', 'T': '"thiscall"'}
CONV = {'C': 'K::C', 'S': 'K::S', 'T': 'K::T'}
OUT = {'u32': 'O::U32', 'u8': 'O::U8', '': 'O::None'}


def hexu(a):
    return f"0x{a >> 16:04x}_{a & 0xffff:04x}"


shims = []
cases = []
seen = set()
for row in rows:
    (addr, abi, kinds, ret, body, callees, prep) = row[:7]
    frame = row[7] if len(row) > 7 else 0
    gap = row[8] if len(row) > 8 else 0
    assert addr not in seen, hex(addr)
    seen.add(addr)
    name = SYM[addr]
    args = [f"a{i}" for i in range(len(kinds))]
    rt = f" -> {ret}" if ret else ''
    b = body.replace('FRAME', 'fr.at()').replace('M', '&mut mem()')
    stmt = b if ret else b + ';'
    if frame:
        assert frame % 4 == 0
        stmt = f"let mut fr = super::StackFrame::<{frame // 4}>::new();\n        {stmt}"
    unused = [a for a in args if not re.search(r'\b' + a + r'\b', body)]
    params = ', '.join((f"_{a}" if a in unused else a) + ": u32" for a in args)
    shims.append(f"    /// `{name}`.\n    {name} @ {hexu(addr)} => extern {ABI[abi]} fn s_{addr:08x}({params}){rt} {{\n        {stmt}\n    }}")
    pb = body.replace('FRAME', 'g.frame()').replace('M', 'g')
    for i, a in enumerate(args):
        pb = re.sub(r'\b' + a + r'\b', f'a[{i}]', pb)
    gv = 'g' if re.search(r'\bg\b', pb) else '_'
    av = 'a' if 'a[' in pb else '_'
    if ret == 'u32':
        port = f"|{gv}, {av}| {pb}"
    elif ret == 'u8':
        port = f"|{gv}, {av}| u32::from({pb})"
    else:
        port = f"|{gv}, {av}| {{\n            {pb};\n            0\n        }}"
    cl = ', '.join(f"({hexu(c[0])}, {c[1]}, {c[2]}, {'false' if c[0] in CDECL else 'true'})" for c in callees)
    wr = ', '.join(f"({hexu(c[0])}, {c[3]})" for c in callees if len(c) > 3)
    cases.append(f"    Case {{\n        name: \"{name}\",\n        addr: {hexu(addr)},\n        conv: {CONV[abi]},\n        kinds: \"{kinds}\",\n        out: {OUT[ret]},\n        callees: &[{cl}],\n        prep: {prep or 'no_prep'},\n        port: {port},\n        frame: {frame:#x},\n        gap: {gap},\n        writes: &[{wr}],\n    }},")

open(R / 'crates/ffx-hook/src/ported/h2_render_b.rs', 'w').write(
    "//! `ffx-prender`, M4 port round 1, list h2-render-b: `PhyreEngine`'s\n"
    "//! render code (the type system's descriptor and array-member slots of\n"
    "//! the render classes, small render-interface, material and shader\n"
    "//! methods). Checked by `crates/ffx-oracle/tests/h2_render_b.rs`.\n"
    "//!\n"
    "//! Threads: the descriptor slots run where the serializer loads (the main\n"
    "//! and loader threads), the render-interface setters on the render\n"
    "//! thread. No shim keeps state; nothing here has a frame.\n"
    "//!\n"
    "//! Round 2 adds the `PArray<T>` class descriptors' constructors and\n"
    "//! `DeclareClassDescriptor`s (`ffx_prender::declare`), which run where a\n"
    "//! class is first used (the main and loader threads).\n\n"
    "use super::{mem, ported};\n"
    "use ffx_prender::array::bridge as a;\n"
    "use ffx_prender::declare as dc;\n"
    "use ffx_prender::dtor::bridge as dt;\n"
    "use ffx_prender::small::bridge as sm;\n"
    "use ffx_prender::descriptor::{Init, bridge as d};\n"
    "use ffx_prender::misc::bridge as x;\n\n"
    "ported! {\n" + '\n'.join(shims) + "\n}\n")
(R / 'crates/ffx-oracle/tests/h2_render_b').mkdir(exist_ok=True)
open(R / 'crates/ffx-oracle/tests/h2_render_b/cases.rs', 'w').write(
    "//! Generated by `tools/m4port/h2_render_b_gen.py` from its spec files: one case\n"
    "//! per port, the same call as its shim.\n\n"
    "use super::*;\n\n"
    "pub const CASES: &[Case] = &[\n" + '\n'.join(cases) + "\n];\n")
print(len(rows), 'rows')
