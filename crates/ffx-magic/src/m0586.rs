//! `magic_0586.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The summon's objects started (0x10002ba0, no arguments): with the
/// record `R = *G_R`, the object `O = R + 0x390` placed
/// ([`crate::s4_misc::start_object_plain`], scale `K`, part 3; three
/// never-written stack words under it, which it does not read), its
/// `+0xd0..+0xdc` from `R + 0x980..`, `+0xc4` from `R + 0x8568`, `+0xd4
/// -= D`; the timer block `T = R + 0x8330` (0, 80, 0, 20, 60, 60, a
/// halfword 0) on a task node (kind 1, callback `CB`, `+0x14` = `T`)
/// appended to the task list; the fader `R + 0x8368`
/// ([`crate::s4_small::fader_task`]) with flags 3, its scales and count
/// 20. Returns the fader task's node.
pub fn objects_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let scale = g.read::<f32>(w(0x04));
    let r = g.read::<u32>(w(0x0c));
    let obj = r.wrapping_add(w(0x15));
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x1e), obj, scale, 3);
    for (i, op) in (0u32..).zip([0x24u32, 0x30, 0x3c, 0x48]) {
        let v = g.read::<u32>(r.wrapping_add(w(op)));
        g.write::<u32>(obj + 0xd0 + 4 * i, v);
    }
    let x = sub(g.read::<f32>(obj + 0xd4), g.read::<f32>(w(0x64)));
    let v = g.read::<u32>(r.wrapping_add(w(0x5c)));
    g.write::<u32>(obj + 0xc4, v);
    g.write::<f32>(obj + 0xd4, x);
    let timer = r.wrapping_add(w(0x82));
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x50),
        (8, 0),
        (0xc, 0x14),
        (0x14, 0x3c),
        (0x10, 0x3c),
    ] {
        g.write::<u32>(timer + o, v);
    }
    g.write::<u16>(timer + 0x18, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xb4), w(0x74));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(0x14), timer);
    crate::r12h_start::append(g, img, f, node, [0xba, 0xd1, 0xf3, 0xe3, 0xf3]);
    let fader = r.wrapping_add(w(0xe9));
    // push esi; push edi; the return address
    let callee = Frame {
        at: img.rel32_target(f + 0xf8),
        esp: fr.esp.wrapping_sub(0xc),
        ecx: fader,
        ..Frame::default()
    };
    let eax = crate::s4_small::fader_task(g, img, &callee).eax;
    let v = g.read::<u16>(fader + 0x2c);
    g.write::<u16>(fader + 0x2c, v | 3);
    for (o, v) in [
        (4u32, 0x3f80_0000u32),
        (8, 0x3f80_0000),
        (0xc, 0),
        (0x10, 0x4080_0000),
        (0x14, 0x4080_0000),
        (0x18, 0),
        (0x24, 0x14),
    ] {
        g.write::<u32>(fader + o, v);
    }
    Ret::eax(eax)
}

/// [`objects_start`] (309 bytes, 1 overlay).
pub const OBJECTS_START: Sig = Sig {
    len: 309,
    hash: 0x93ce_da9d_0e84_1fba,
    calls: &[
        (0x1e, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0xb4, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xf8, Callee::Sig(&crate::s4_small::FADER_TASK_40)),
    ],
    imports: &[],
};
