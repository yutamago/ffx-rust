//! `magic_0072.dll`'s own functions (besides the runtime and the shared
//! code): the sprite pool and the task list.

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Sets the sprite pool up and takes the first task (0x10002610,
/// internal: `ecx` = the pool's buffer): 0x200 records of 0x6c bytes,
/// each marked free (`+0x68` 0), the rover and the count reset; then a
/// node from the free task list (`G_FREE`, `G_NFREE` u16 nodes; the free
/// list's links joined around it) cleared, its step `STEP` (operand
/// `+0x95`), appended to the task list (`G_HEAD`, `G_TAIL`). With no
/// free node the original goes on with node 0 (`[4]` written: it
/// faults); the port does the same. Returns the node.
pub fn pool_setup<G: Game>(g: &mut G, img: &Image, f: u32, buf: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(w(0x02), buf);
    for i in 0..0x200u32 {
        g.write::<u32>(buf + 0x68 + 0x6c * i, 0);
    }
    let n = g.read::<u16>(w(0x20));
    g.write::<u32>(w(0x25), 0);
    g.write::<u32>(w(0x2b), 0x200);
    g.write::<u32>(w(0x34), 0);
    let mut node = 0;
    if n != 0 {
        node = g.read::<u32>(w(0x3e));
        let (next, prev) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(next + 4, prev);
        let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node));
        g.write::<u32>(prev, next);
        if n > 1 {
            let v = g.read::<u32>(node + 4);
            g.write::<u32>(w(0x5c), v);
        } else {
            g.write::<u32>(w(0x64), 0);
        }
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node + o, 0);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(0x95));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(0xa0), n.wrapping_sub(1));
    }
    let head = g.read::<u32>(w(0xa6));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head != 0 {
        let tail = g.read::<u32>(w(0xb7));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(0xc4), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0xd0), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0xd8), node);
    }
    node
}

/// `pool_setup` (221 bytes, 1 overlay).
pub const POOL_SETUP: Sig = Sig {
    len: 221,
    hash: 0x5489_3d32_45d6_e643,
    calls: &[],
    imports: &[],
};

/// A free sprite record (0x100026f0, internal, no arguments): from the
/// rover on (wrapping at the count), the first with `+0x68` 0, taken
/// (`+0x68` 1, `+0x60`/`+0x64` 0, colours `+0x3c`/`+0x40` 0x80808080)
/// and the rover moved past it. Returns it, or 0 when all are taken.
pub fn pool_take<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut i = g.read::<u32>(w(0x02));
    let n = g.read::<u32>(w(0x0a)).cast_signed();
    if n <= 0 {
        return 0;
    }
    let buf = g.read::<u32>(w(0x17));
    for _ in 0..n {
        let rec = i.wrapping_mul(0x6c).wrapping_add(buf);
        i = i.wrapping_add(1);
        if i.cast_signed() >= n {
            i = 0;
        }
        if g.read::<u32>(rec + 0x68) == 0 {
            g.write::<u32>(rec + 0x64, 0);
            g.write::<u32>(rec + 0x60, 0);
            g.write::<u32>(rec + 0x3c, 0x8080_8080);
            g.write::<u32>(rec + 0x40, 0x8080_8080);
            g.write::<u32>(rec + 0x68, 1);
            g.write::<u32>(w(0x5e), i);
            return rec;
        }
    }
    0
}

/// `pool_take` (100 bytes, 1 overlay).
pub const POOL_TAKE: Sig = Sig {
    len: 100,
    hash: 0x9443_dda2_af82_32b0,
    calls: &[],
    imports: &[],
};

/// An object callback (stored at 0x1000154f, `(obj)`): the camera turned
/// (slot 235 `(&M)`, slots 328 `(&M, &M, A)`, 329 `(&M, &M, B)`), slot
/// 236 `(G_H, 0x10, &C)` with its 3x3 part scaled by `K / s` (`s` slot
/// 277's ST0 as `f32`, `K` operand `+0x9c`) and slot 231 `(&C, &C, &M)`;
/// then `C`'s translation kept, cleared, and slot 229 `(&V, &C, &V)` with
/// `V = (0, 2, -2, 1)` gives the offset added to it into the object's
/// `+0xd0..+0xdc`. Locals: `M` `ebp - 0x94`, `C` `- 0x54`, `V` `- 0x14`.
/// Each call gets the words the earlier ones left on the stack. Returns
/// slot 229's result.
pub fn place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp - 4;
    let (m, c, v) = (ebp - 0x94, ebp - 0x54, ebp - 0x14);
    let ecx = fr.ecx;
    let s235 = slot_target(g, w(0x1f), 235);
    g.call(s235, ecx, &[m]);
    let s328 = slot_target(g, w(0x3b), 328);
    g.call(s328, ecx, &[m, m, 0x3f5f_66f3]);
    let s329 = slot_target(g, w(0x5a), 329);
    let mut stack = vec![m, m, 0xbfc9_0fdb];
    g.call(s329, ecx, &stack);
    let s236 = slot_target(g, w(0x6b), 236);
    let h = g.read::<u32>(w(0x73));
    stack.splice(0..0, [h, 0x10, c]);
    // call width: 6
    g.call(s236, ecx, &stack);
    let s277 = slot_target(g, w(0x80), 277);
    let h = g.read::<u32>(w(0x86));
    stack.insert(0, h);
    // call width: 7
    let s = st0_f32(g.call(s277, ecx, &stack).st0);
    let k = div(g.read::<f32>(w(0x9c)), s);
    for i in [0u32, 1, 2, 4, 5, 6, 8, 9, 10] {
        let x = mul(g.read::<f32>(c + 4 * i), k);
        g.write::<f32>(c + 4 * i, x);
    }
    let s231 = slot_target(g, w(0xda), 231);
    stack.splice(0..0, [c, c, m]);
    // call width: 10
    g.call(s231, ecx, &stack);
    let t = [0x30u32, 0x34, 0x38, 0x3c].map(|o| g.read::<f32>(c + o));
    for (o, x) in [
        (0u32, 0u32),
        (4, 0x4000_0000),
        (8, 0xc000_0000),
        (0xc, 0x3f80_0000),
    ] {
        g.write::<u32>(v + o, x);
    }
    for o in [0x38u32, 0x34, 0x30] {
        g.write::<u32>(c + o, 0);
    }
    let s229 = slot_target(g, w(0x18e), 229);
    stack.splice(0..0, [v, c, v]);
    // call width: 13
    let r = g.call(s229, ecx, &stack).eax;
    for (i, x) in (0u32..).zip(&t[..3]) {
        let y = add(*x, g.read::<f32>(v + 4 * i));
        g.write::<f32>(obj + 0xd0 + 4 * i, y);
    }
    g.write::<f32>(obj + 0xdc, t[3]);
    Ret::eax(r)
}

/// 0x10002af0 (542 bytes, 1 overlay).
pub const PLACE: Sig = Sig {
    len: 542,
    hash: 0x4a25_461f_0efd_ea4e,
    // __security_check_cookie
    calls: &[(0x216, Callee::Any)],
    imports: &[],
};
