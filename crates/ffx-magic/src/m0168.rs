//! `magic_0168.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The camera's frame (0x10003890, no arguments, an aligned /GS frame):
/// the countdown `P + 0x3158` (`P` = `*` operand `+0x1d`) drops while
/// positive; slot 562, slot 723 `(a)` and 724 `(b, a)` fill two local
/// matrices (`a` at `esp + 0x44`, `b` at `esp + 4` of the aligned frame),
/// slot 231 `(c, b, P + 0x9c0)` combines them into `c` (`esp + 0x84`).
/// Returns slot 231's `eax`.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn camera_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xcc);
    let (a, b, c) = (e + 0x44, e + 4, e + 0x84);
    let p = g.read::<u32>(w(0x1d));
    let n = g.read::<i32>(p + 0x3158);
    if n > 0 {
        g.write::<i32>(p + 0x3158, n - 1);
    }
    let s = slot_target(g, w(0x33), 562);
    g.call(s, 0, &[]);
    let s = slot_target(g, w(0x45), 723);
    g.call(s, 0, &[a]);
    let s = slot_target(g, w(0x57), 724);
    g.call(s, 0, &[b, a]);
    let s = slot_target(g, w(0x78), 231);
    Ret::eax(g.call(s, 0, &[c, b, p + 0x9c0, b, a]).eax)
}

/// [`camera_frame`] (154 bytes, 1 overlay; the `rel32` at `+0x92` is
/// `__security_check_cookie`).
pub const CAMERA_FRAME: Sig = Sig {
    len: 154,
    hash: 0xf299_e16d_f015_cdb0,
    calls: &[(0x92, Callee::Any)],
    imports: &[],
};

/// Gives an object its counters and a task node (0x10007b50, internal:
/// `ecx` = the object): `+0x104`/`+0x108` 0x6e, `+0x10c` 8, the halfword
/// `+0x126` 0; a pool node ([`take`](crate::sa_pool::take), owner the
/// immediate `+0x06`) holds the object (`+0x14`, `+0x4` 0) at the end of
/// the task list (first `+0x36`/`+0x63`, last `+0x4a`/`+0x54`/`+0x68`).
/// Returns the node.
pub fn object_task(g: &mut impl crate::Mem, img: &Image, f: u32, obj: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(obj + 0x108, 0x6e);
    g.write::<u32>(obj + 0x104, 0x6e);
    g.write::<u32>(obj + 0x10c, 8);
    g.write::<u16>(obj + 0x126, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x30), w(0x6));
    let empty = g.read::<u32>(w(0x36)) == 0;
    g.write::<u32>(node + 0x14, obj);
    g.write::<u32>(node + 4, 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x63), node);
        g.write::<u32>(w(0x68), node);
    } else {
        let last = g.read::<u32>(w(0x4a));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(w(0x54), node);
    }
    g.write::<u32>(node + 8, node);
    node
}

/// [`object_task`] (112 bytes, 1 overlay).
pub const OBJECT_TASK: Sig = Sig {
    len: 112,
    hash: 0xc3bb_9a1d_b325_0c90,
    calls: &[(0x30, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// Starts the block's object (0x10002f40, a table entry, no arguments;
/// the block `S = *G_S`, operand `+0x0c`): the object `O = S + 0x4c0`
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale the
/// float at `+0x04`, index 4, value 0); `O + 0xd0..0xdc` = `S +
/// 0xac0..`, `O + 0xc4` = `S + 0x3150`. Returns that word.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x4));
    let s = g.read::<u32>(w(0xc));
    let o = s + 0x4c0;
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x20), o, scale, 4, 0);
    for k in 0..4u32 {
        let v = g.read::<u32>(s + 0xac0 + 4 * k);
        g.write::<u32>(o + 0xd0 + 4 * k, v);
    }
    let v = g.read::<u32>(s + 0x3150);
    g.write::<u32>(o + 0xc4, v);
    Ret::eax(v)
}

/// [`object_start`] (102 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 102,
    hash: 0xbff1_e65f_73b9_29a4,
    calls: &[(0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
    imports: &[],
};
