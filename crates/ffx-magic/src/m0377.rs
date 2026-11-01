//! `magic_0377.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (0x10003ce0, cdecl `(task)`): the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)) and the objects run
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both give 1
/// and `*G_W` (`+0x17`) is 0, the task's state is 3, its record's
/// `+0x10` the immediate `+0x33`, and a tail jump into the part
/// registration ([`register_parts`](crate::sa_misc::register_parts),
/// `eax` the record). Returns its result, else the objects'.
pub fn parts_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x5));
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0xc), fr.esp.wrapping_sub(12));
    if r != 1 || g.read::<u32>(w(0x17)) != 0 || m != 1 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0x33));
    Ret::eax(crate::sa_misc::register_parts(
        g,
        img,
        img.rel32_target(f + 0x39),
        rec,
    ))
}

/// [`parts_step`] (64 bytes, 1 overlay).
pub const PARTS_STEP: Sig = Sig {
    len: 64,
    hash: 0x4102_229a_ec14_f80a,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x39, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};
