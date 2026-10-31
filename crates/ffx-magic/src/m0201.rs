//! `magic_0201.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A node handler (0x100042e0, `ecx` = the cursor, an aligned /GS
/// frame): the walk (each child of the cursor's node gets its method
/// `+0x1c` with the cursor; then the node takes the pass); when the
/// cursor moved, slot 231 `(m, M, node[0x1c] + 0x9c)` (`M` the immediate
/// `+0x72`) into a local `m` (`esp + 8` of the aligned frame) and slot
/// 697 `(node[0x20], m, **P)` (`P` at `+0x85`). Returns slot 697's
/// `eax`, else the walk's (the low byte the pass, the rest what the
/// frame's cookie or the last method left).
pub fn matrix_visit<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let cur = fr.ecx;
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x50);
    let mut eax = g.read::<u32>(w(0x0a)) ^ e;
    let node = g.read::<u32>(cur);
    g.write::<u32>(e + 4, node);
    let pass = g.read::<u8>(node + 0x1a);
    if pass == g.read::<u8>(cur + 5) {
        g.write::<u8>(cur + 4, 0);
        eax = (eax & !0xff) | u32::from(pass);
    } else {
        let mut child = g.read::<u32>(node + 0x10);
        while child != node {
            g.write::<u32>(cur, child);
            let c = g.read::<u32>(cur);
            child = g.read::<u32>(child + 8);
            let m = g.read::<u32>(g.read::<u32>(c) + 0x1c);
            eax = g.call(m, node, &[cur]).eax;
        }
        let pass = g.read::<u8>(cur + 5);
        g.write::<u32>(cur, node);
        g.write::<u8>(node + 0x1a, pass);
        g.write::<u8>(cur + 4, 1);
        eax = (eax & !0xff) | u32::from(pass);
    }
    if g.read::<u8>(cur + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let node = g.read::<u32>(cur);
    let sub = g.read::<u32>(node + 0x20);
    let src = g.read::<u32>(node + 0x1c).wrapping_add(0x9c);
    let m = e + 8;
    let s = slot_target(g, w(0x78), 231);
    g.call(s, 0, &[m, w(0x72), src]);
    let p = g.read::<u32>(g.read::<u32>(w(0x85)));
    let s = slot_target(g, w(0x91), 697);
    Ret::eax(g.call(s, 0, &[sub, m, p, m, w(0x72), src]).eax)
}

/// [`matrix_visit`] (178 bytes, 1 overlay; the `rel32` at `+0xaa` is
/// `__security_check_cookie`).
pub const MATRIX_VISIT: Sig = Sig {
    len: 178,
    hash: 0x5a20_4171_71b3_54a4,
    calls: &[(0xaa, Callee::Any)],
    imports: &[],
};
