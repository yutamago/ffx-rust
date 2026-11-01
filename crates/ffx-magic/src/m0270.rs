//! `magic_0270.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's facing (0x10001b10, cdecl `(obj)`; `m0243::object_facing`'s
/// kin): `K - f` with `f` slot 333 `(*X)`'s float (`X` `+0x0b`, `K`
/// `+0x1e`), wrapped once (above `HI` `+0x2d` less `SPAN` `+0x37`, below
/// `LO` `+0x4e` plus `SPAN` `+0x5b`) into `+0xc4`. Returns the object.
pub fn object_facing<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = slot_target(g, w(0x05), 0x534 / 4);
    let x = g.read::<u32>(w(0x0b));
    let f = g.call(s, 0, &[x]).st0.unwrap_or(0.0) as f32;
    let a = sub(g.read::<f32>(w(0x1e)), f);
    let a = if a > g.read::<f32>(w(0x2d)) {
        sub(a, g.read::<f32>(w(0x37)))
    } else if g.read::<f32>(w(0x4e)) > a {
        add(a, g.read::<f32>(w(0x5b)))
    } else {
        a
    };
    let obj = fr.arg(g, 0);
    g.write::<f32>(obj + 0xc4, a);
    Ret::eax(obj)
}

/// [`object_facing`] (110 bytes, 1 overlay).
pub const OBJECT_FACING: Sig = Sig {
    len: 110,
    hash: 0xba0b_1b9a_5332_9b4b,
    calls: &[],
    imports: &[],
};

/// A sound task (0x10001bd0, cdecl `(rec)`): slot 239 `(*H, B[i], 0)`
/// for the record's sound `i` (`+0x2c`; `H` `+0x05`, the byte list `*B`
/// `+0x1c`; `i == -1` passes `*H` as the byte); the pair `(i, rec +
/// 0x30)` into the table `+0x38` at `i`; a pool node
/// ([`take`](crate::sa_pool::take), owner the callback `+0x45`) holds the
/// pair (`+0x14`) and goes last on the task list (first `+0x53`, last
/// `+0x69`). Returns the node.
pub fn sound_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let h = g.read::<u32>(w(0x05));
    let rec = fr.arg(g, 0);
    let i = g.read::<u32>(rec + 0x2c);
    let byte = if i == u32::MAX {
        h
    } else {
        let b = g.read::<u32>(w(0x1c));
        u32::from(g.read::<u8>(b.wrapping_add(i)))
    };
    let s = slot_target(g, w(0x28), 0x3bc / 4);
    g.call(s, 0, &[h, byte, 0]);
    let pair = i.wrapping_mul(8).wrapping_add(w(0x38));
    g.write::<u32>(pair, i);
    g.write::<u32>(pair + 4, g.read::<u32>(rec + 0x30));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x4d), w(0x45));
    let head = g.read::<u32>(w(0x53));
    g.write::<u32>(node + 0x14, pair);
    g.write::<u32>(node + 4, 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x83), node);
        g.write::<u32>(w(0x88), node);
    } else {
        let last = g.read::<u32>(w(0x69));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(w(0x73), node);
    }
    g.write::<u32>(node + 8, node);
    Ret::eax(node)
}

/// [`sound_task`] (145 bytes, 1 overlay).
pub const SOUND_TASK: Sig = Sig {
    len: 145,
    hash: 0x1e60_f1e4_2a3c_d1f8,
    calls: &[(0x4d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};
