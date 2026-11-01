//! `magic_0162.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the block's scale task and its record (0x10002e40, a table
/// entry, no arguments; the block `S = *G_S`, operand `+0x13`): the scale
/// task at `S + 0x8428` ([`scale_task`](crate::s4_misc::scale_task),
/// `xmm2`/`xmm1` the floats at `+0x04`/`+0x0c`, the count 0); the record
/// `R = S + 0x8390` gets `+0x0..+0xc` `[0, 100, 0, 0]`, `+0x10`/`+0x14`
/// 150 and the halfword `+0x18` 0; a pool node
/// ([`take`](crate::sa_pool::take), owner the immediate `+0x30`) holds it
/// (`+0x14`, byte `+0x18` 1, `+0x4` 0) at the end of the task list (first
/// `+0x68`/`+0x92`, last `+0x80`/`+0x9a`). Then slot 371 `(0,
/// 0x10000)` and `(1, 0x8000)` (the first call's words left under the
/// second's). Returns the last call's `eax`.
pub fn scale_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x13));
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(g.read::<u32>(w(0x4)));
    xmm[1] = u128::from(g.read::<u32>(w(0xc)));
    // push esi; push 0; the call
    let task = Frame {
        at: img.rel32_target(f + 0x20),
        esp: fr.esp.wrapping_sub(12),
        ecx: s.wrapping_add(0x8428),
        xmm,
        ..*fr
    };
    g.write::<u32>(task.esp + 4, 0);
    crate::s4_misc::scale_task(g, img, &task);
    let r = s.wrapping_add(0x8390);
    for (o, v) in [
        (0u32, 0u32),
        (4, 100),
        (8, 0),
        (0xc, 0),
        (0x14, 150),
        (0x10, 150),
    ] {
        g.write::<u32>(r + o, v);
    }
    g.write::<u16>(r + 0x18, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x62), w(0x30));
    let empty = g.read::<u32>(w(0x68)) == 0;
    g.write::<u32>(node + 0x14, r);
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 4, 0);
    crate::s3_misc::append_task(g, node, w(0x92), w(0x9a), empty);
    let t = slot_target(g, w(0x9f), 371);
    g.call(t, 0, &[0, 0x1_0000]);
    let t = slot_target(g, w(0xb3), 371);
    Ret::eax(g.call(t, 0, &[1, 0x8000, 0, 0x1_0000]).eax)
}

/// [`scale_start`] (202 bytes, 1 overlay).
pub const SCALE_START: Sig = Sig {
    len: 202,
    hash: 0x4b10_559b_e174_efbb,
    calls: &[
        (0x20, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        (0x62, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// Starts the block's object (0x10002f10, a table entry, no arguments;
/// the block `S = *G_S`, operand `+0x0b`): the object `O = S + 0x130`
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale
/// the float at `+0x04`, index 1); `O + 0xd0..0xdc` = `S + 0x960..`,
/// `O + 0xc4` = `S + 0x85c8`, and `O + 0xd4` less the float at `+0x5e`.
/// Returns the word copied to `+0xc4`.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x4));
    let s = g.read::<u32>(w(0xb));
    let o = s.wrapping_add(0x130);
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x1e), o, scale, 1);
    for k in 0..4u32 {
        let v = g.read::<u32>(s + 0x960 + 4 * k);
        g.write::<u32>(o + 0xd0 + 4 * k, v);
    }
    let y = crate::sb_sse::sub(g.read::<f32>(o + 0xd4), g.read::<f32>(w(0x5e)));
    let v = g.read::<u32>(s + 0x85c8);
    g.write::<u32>(o + 0xc4, v);
    g.write::<f32>(o + 0xd4, y);
    Ret::eax(v)
}

/// [`object_start`] (124 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 124,
    hash: 0xa898_c6c5_6b36_efc6,
    calls: &[(0x1e, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN))],
    imports: &[],
};
