//! `magic_0354.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step with a countdown (0x10004710, cdecl `(task)`): while the
/// count `*G_C` (`+0x04`) is above 0 it goes down; reaching 0, the actor
/// block ([`me_block`](crate::sa_misc::me_block), the immediate `+0x15`),
/// slot 537 `(*G_A, 2, 0, *(*G_B), 10, 20)` (operands `+0x3c`, `+0x1f`;
/// the block's word under it) and `*G_D` (`+0x47`) 1. Then the motion
/// script of `+0x51` ([`motion_script`](crate::s4_misc::motion_script))
/// and the objects run ([`run_objects`](crate::l5_objhost::run_objects));
/// when both give 1 and `*G_D` is 1, the task's state is 3, its record's
/// `+0x10` the immediate `+0x83`, and a tail jump into the part
/// registration ([`register_parts`](crate::sa_misc::register_parts),
/// `eax` the record). Returns its result, else the objects'.
pub fn countdown_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = g.read::<u32>(w(0x4));
    if c.cast_signed() > 0 {
        let c = c - 1;
        g.write::<u32>(w(0xe), c);
        if c == 0 {
            crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x1a), w(0x15));
            let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x1f))));
            let a = g.read::<u32>(w(0x3c));
            let t = slot_target(g, w(0x2c), 537);
            g.call(t, 0, &[a, 2, 0, b, 0xa, 0x14, w(0x15)]);
            g.write::<u32>(w(0x47), 1);
        }
    }
    let m = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x56), w(0x51));
    // push ebp; push esi; the call
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x5d), fr.esp.wrapping_sub(12));
    if r != 1 || g.read::<u32>(w(0x68)) != 1 || m != 1 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0x83));
    Ret::eax(crate::sa_misc::register_parts(
        g,
        img,
        img.rel32_target(f + 0x89),
        rec,
    ))
}

/// [`countdown_step`] (144 bytes, 1 overlay).
pub const COUNTDOWN_STEP: Sig = Sig {
    len: 144,
    hash: 0xc9c8_7a3e_4779_a559,
    calls: &[
        (0x1a, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x56, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x5d, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x89, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};
