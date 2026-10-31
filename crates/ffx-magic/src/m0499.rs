//! `magic_0499.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The effect's start (0x100012b0, an overlay table entry, `(arg)`):
/// `arg` to the global at operand `+0x07`; slot 803 `(0x100000, 0x10)`
/// the buffer (`+0x25`), slot 861 `(buf, NAME, 0x21940)` loads the file
/// into it, slot 259; slot 260 `(0x14)` and `(0x15)` two handles
/// (`+0x59`, `+0x8d`), each followed by five halfword flags cleared;
/// slots 718 `(NAME)` and 222 `(NAME, 0, 1000, 0x100)` (each with the
/// earlier words on the stack); three state words 0. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x7), arg);
    let t = slot_target(g, w(0xc), 803);
    let mut stack = vec![0x10_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x25), buf);
    stack.splice(0..0, [buf, w(0x2a), 0x2_1940]);
    let t = slot_target(g, w(0x30), 861);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x3d), 259);
    g.call(t, 0, &stack);
    for (tab, n, dst, flags) in [
        (0x4au32, 0x14u32, 0x59u32, [0x61u32, 0x67, 0x6d, 0x73, 0x79]),
        (0x7e, 0x15, 0x8d, [0x95, 0x9b, 0xa1, 0xa7, 0xad]),
    ] {
        let t = slot_target(g, w(tab), 260);
        stack.insert(0, n);
        // call width: 12
        let h = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), h);
        for op in flags {
            g.write::<u16>(w(op), 0);
        }
    }
    let t = slot_target(g, w(0xb2), 718);
    stack.insert(0, w(0xb7));
    // call width: 12
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xc4), 222);
    stack.splice(0..0, [w(0xdb), 0, 0x3e8, 0x100]);
    // call width: 12
    g.call(t, 0, &stack);
    for op in [0xe6, 0xf0, 0xfa] {
        g.write::<u32>(w(op), 0);
    }
    Ret::eax(0)
}

/// 0x100012b0 (262 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 262,
    hash: 0x71f3_c43e_9518_338d,
    calls: &[],
    imports: &[],
};
