//! `magic_0244.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's start (0x10001a10, internal: `ecx` = the context, `edx` =
/// the object, `xmm3` = a scale, two stack words of which the second is
/// the animation's): the object's position `+0x40..+0x48` 0 and `+0x4c`
/// 1.0, `+0x50..+0x5c` 0, the scale `+0x60..+0x6c` the `xmm3` float;
/// `**G` = the context (`G` `+0x04`); the program `list[0]` of slot 941
/// `(context +0x2c's +0x10)` started on the object
/// ([`start_part`](crate::opu_parts::start_part) `(obj, 0x1000, P,
/// 1)`), the animation block reset
/// ([`anim_reset`](crate::sb_misc::anim_reset)); a pool node
/// ([`pool_alloc`](crate::l5_objhost::pool_alloc)), when there is one,
/// goes last on the context's list (first `+0x30`, last `+0x34`, count
/// `+0x3c`) pointing at the object (`+8`). Returns the object.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (ctx, obj) = (fr.ecx, fr.edx);
    let gp = g.read::<u32>(w(0x04));
    for off in [0x48, 0x44, 0x40] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    for off in [0x5c, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + off, 0);
    }
    let scale = fr.xmm[3] as u32;
    for off in [0x6c, 0x68, 0x64, 0x60] {
        g.write::<u32>(obj + off, scale);
    }
    g.write::<u32>(gp, ctx);
    let mut p = Pushes::default();
    p.push(g.read::<u32>(g.read::<u32>(ctx + 0x2c) + 0x10));
    let s = slot_target(g, w(0x63), 0xeb4 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let programs = p.call(g, s, 0).eax;
    let prog = g.read::<u32>(programs);
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x7d), obj, 0x1000, prog, 1);
    crate::sb_misc::anim_reset(g, obj, fr.arg(g, 1));
    let node = crate::l5_objhost::pool_alloc(g, img, img.rel32_target(f + 0x92));
    if node == 0 {
        return Ret::eax(obj);
    }
    let last = g.read::<u32>(ctx + 0x34);
    g.write::<u32>(node + 4, 0);
    g.write::<u32>(node, last);
    if last == 0 {
        g.write::<u32>(ctx + 0x30, node);
    } else {
        let last = g.read::<u32>(ctx + 0x34);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(ctx + 0x34, node);
    g.write::<u32>(node + 8, obj);
    let n = g.read::<u32>(ctx + 0x3c);
    g.write::<u32>(ctx + 0x3c, n.wrapping_add(1));
    Ret::eax(obj)
}

/// [`object_start`] (213 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 213,
    hash: 0x055d_7118_7853_5769,
    calls: &[
        (0x7d, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x8a, Callee::Sig(&crate::sb_misc::ANIM_RESET)),
        (0x92, Callee::Sig(&crate::l5_objhost::POOL_ALLOC)),
    ],
    imports: &[],
};
