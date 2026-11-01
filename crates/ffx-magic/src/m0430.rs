//! `magic_0430.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Puts a fresh pool node last on the task list: first `F`, last `L`
/// (`+0x0c` is `F`'s operand for the test, `L` read at `l_read`; the
/// stores go to `l_write`, and into `f_write`/`l_empty` for an empty
/// list). The node holds itself (`+8`); `extra` is `+0xeb0`'s object or
/// 0 for none (stored after `+8`).
struct Last {
    first: u32,
    l_read: u32,
    l_write: u32,
    f_write: u32,
    l_empty: u32,
}

fn put_last<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, at: &Last, extra: u32) {
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(at.first)) == 0 {
        g.write::<u32>(node, 0);
        if extra == 0 {
            g.write::<u32>(w(at.f_write), node);
            g.write::<u32>(w(at.l_empty), node);
            g.write::<u32>(node + 8, node);
        } else {
            g.write::<u32>(node + 8, node);
            g.write::<u32>(extra + 0xeb0, node);
            g.write::<u32>(w(at.f_write), node);
            g.write::<u32>(w(at.l_empty), node);
        }
    } else {
        let last = g.read::<u32>(w(at.l_read));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        if extra == 0 {
            g.write::<u32>(w(at.l_write), node);
            g.write::<u32>(node + 8, node);
        } else {
            g.write::<u32>(node + 8, node);
            g.write::<u32>(extra + 0xeb0, node);
            g.write::<u32>(w(at.l_write), node);
        }
    }
}

/// The effect's start (0x100030d0, no arguments): with `E` = `*G`
/// (`+0x03`) two colour quads `E + 0xbf4` and `E + 0xc34` become `(0, 0,
/// 0, 1.0)` and `E + 0xec4` 100; a pool node
/// ([`take`](crate::sa_pool::take), owner `+0x08`) goes last on the task
/// list (first `+0x6d`, last `+0x7d`) and into `E + 0xeb0`. Returns the
/// node.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    const ONE: u32 = 0x3f80_0000;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x03));
    for (off, v) in [
        (0xbf4, 0),
        (0xbf8, 0),
        (0xbfc, 0),
        (0xc00, ONE),
        (0xc34, 0),
        (0xc38, 0),
        (0xc3c, 0),
        (0xc40, ONE),
        (0xec4, 0x64),
    ] {
        g.write::<u32>(e + off, v);
    }
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x67), w(0x08));
    g.write::<u32>(node + 4, 0);
    let at = Last {
        first: 0x6d,
        l_read: 0x7d,
        l_write: 0x90,
        f_write: 0xa6,
        l_empty: 0xab,
    };
    put_last(g, img, f, node, &at, e);
    Ret::eax(node)
}

/// [`start`] (177 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 177,
    hash: 0xd7e0_3dc1_e4df_80b3,
    calls: &[(0x67, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// The fade's step (0x10003cf0, cdecl `(node)`): with the count `n`
/// (`*C`, `+0x04`) slot 38 `(*V, x, 0)` (`V` `+0x2a`), `x` = `(n - 7) /
/// D` (`D` `+0x1e`) above 7, else 0; the count goes up and from 20 on slot
/// 38 `(*V, 1.0, 0)` and the node goes back to the pool
/// ([`give_back`](crate::sa_pool::give_back), a tail jump, `+0x73`).
/// Returns the new count below 20. Each slot call passes the words the
/// original leaves on the stack.
pub fn fade_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = g.read::<u32>(w(0x04));
    let x = if n.cast_signed() > 7 {
        crate::sb_sse::div(
            crate::sb_sse::cvt(n.wrapping_sub(7)),
            g.read::<f32>(w(0x1e)),
        )
    } else {
        0.0
    };
    let s = slot_target(g, w(0x23), 0x98 / 4);
    let v = g.read::<u32>(w(0x2a));
    g.call(s, 0, &[v, x.to_bits(), 0]);
    let n = g.read::<u32>(w(0x40)).wrapping_add(1);
    g.write::<u32>(w(0x49), n);
    if n.cast_signed() < 0x14 {
        return Ret::eax(n);
    }
    let s = slot_target(g, w(0x53), 0x98 / 4);
    g.call(s, 0, &[v, 0x3f80_0000, 0]);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x73),
        node,
    ))
}

/// [`fade_step`] (122 bytes, 1 overlay).
pub const FADE_STEP: Sig = Sig {
    len: 122,
    hash: 0xec7f_549d_e3b9_c249,
    calls: &[(0x73, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// The fade's start (0x10003d70, no arguments): slot 38 `(*V, 0, 0)`
/// (`V` `+0x17`), then a pool node ([`take`](crate::sa_pool::take),
/// owner the fade's step `+0x21`) with the byte `+0x18` = 1 goes last on
/// the task list (first `+0x2c`, last `+0x40`). Returns the node.
pub fn fade_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x01), 0x98 / 4);
    let v = g.read::<u32>(w(0x17));
    g.call(s, 0, &[v, 0, 0]);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x26), w(0x21));
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 4, 0);
    let at = Last {
        first: 0x2c,
        l_read: 0x40,
        l_write: 0x4a,
        f_write: 0x59,
        l_empty: 0x5e,
    };
    put_last(g, img, f, node, &at, 0);
    Ret::eax(node)
}

/// [`fade_start`] (102 bytes, 1 overlay).
pub const FADE_START: Sig = Sig {
    len: 102,
    hash: 0x23bd_4634_6f69_225f,
    calls: &[(0x26, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};
