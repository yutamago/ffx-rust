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

/// Slot 371 `(0, 0x10000)` and `(1, 0x20000)` (the first call's words
/// left under the second's), then the block's scale task on `*G_B +
/// 0xe3c` (0x10002280, a table entry, no arguments; `*G_B` operand
/// `+0x0e`; [`scale_task_edx`](crate::s4_small::scale_task_edx), `edx`
/// 30, `xmm2`/`xmm1` the floats at `+0x33`/`+0x3b`, the word 0x10000
/// still pushed). Returns the task's node.
pub fn scale_e3c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let b = g.read::<u32>(w(0xe));
    let t = slot_target(g, w(0x1), 371);
    g.call(t, 0, &[0, 0x1_0000]);
    let t = slot_target(g, w(0x1c), 371);
    g.call(t, 0, &[1, 0x2_0000, 0, 0x1_0000]);
    let esp = fr.esp.wrapping_sub(12);
    g.write::<u32>(esp + 4, 0x1_0000);
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(g.read::<u32>(w(0x33)));
    xmm[1] = u128::from(g.read::<u32>(w(0x3b)));
    let callee = Frame {
        at: img.rel32_target(f + 0x4e),
        esp,
        ecx: b.wrapping_add(0xe3c),
        edx: 0x1e,
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_edx(g, img, &callee)
}

/// [`scale_e3c`] (87 bytes, 1 overlay).
pub const SCALE_E3C: Sig = Sig {
    len: 87,
    hash: 0x2076_a523_74e6_9599,
    calls: &[(0x4e, Callee::Sig(&crate::s4_small::SCALE_TASK_EDX))],
    imports: &[],
};

/// Starts three objects (0x100024a0, a table entry, no arguments; the
/// block `S = *G_S`, operand `+0x0c`;
/// [`start_object_anim`](crate::s4_misc::start_object_anim) each): `O =
/// S + 0x4c0` (scale the float at `+0x04`, index 4, value 0) with `O +
/// 0xd0..0xdc` = `S + 0xcc4..`, `O + 0xc4` = `S + 0xeb4` and `O + 0xd4`
/// less `K` (`+0x68`); `S + 0x390` (scale `+0x2e`, index 3, value 0x3a)
/// with `+0x104` the immediate `+0xa3` and `+0xc4` = `S + 0xeb4`; `S +
/// 0x260` (scale `+0x96`, index 2, value 0x3a) likewise (`+0xc8`).
/// Returns `S + 0xeb4`'s word.
pub fn three_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0xc));
    let o = s + 0x4c0;
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x20), o, k(g, 0x4), 4, 0);
    for i in 0..4u32 {
        let v = g.read::<u32>(s + 0xcc4 + 4 * i);
        g.write::<u32>(o + 0xd0 + 4 * i, v);
    }
    let y = crate::sb_sse::sub(g.read::<f32>(o + 0xd4), k(g, 0x68));
    let v = g.read::<u32>(s + 0xeb4);
    g.write::<u32>(o + 0xc4, v);
    g.write::<f32>(o + 0xd4, y);
    crate::s4_misc::start_object_anim(
        g,
        img,
        img.rel32_target(f + 0x8e),
        s + 0x390,
        k(g, 0x2e),
        3,
        0x3a,
    );
    g.write::<u32>(s + 0x494, w(0xa3));
    let v = g.read::<u32>(s + 0xeb4);
    g.write::<u32>(s + 0x454, v);
    crate::s4_misc::start_object_anim(
        g,
        img,
        img.rel32_target(f + 0xbe),
        s + 0x260,
        k(g, 0x96),
        2,
        0x3a,
    );
    g.write::<u32>(s + 0x364, w(0xc8));
    let v = g.read::<u32>(s + 0xeb4);
    g.write::<u32>(s + 0x324, v);
    Ret::eax(v)
}

/// [`three_start`] (222 bytes, 1 overlay).
pub const THREE_START: Sig = Sig {
    len: 222,
    hash: 0xc1de_57b1_60ac_8e67,
    calls: &[
        (0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x8e, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xbe, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};
