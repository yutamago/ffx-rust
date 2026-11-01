//! `magic_0318.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (0x100014d0, cdecl `(task)`): the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)) and the objects run
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both give 1:
/// slot 296 `(n, 0)` while `*G_DONE` (`+0x1b`) is 0 and the count `n =
/// *G_N` (`+0x23`) below 32 (signed), else slot 257 `(*G_P, 1)` (`+0x45`);
/// then the task's state is 3 and its record's `+0x10` the immediate
/// `+0x64`. Returns the record, else the objects' result.
pub fn count_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x5));
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0xc), fr.esp.wrapping_sub(12));
    if r != 1 || m != 1 {
        return Ret::eax(r);
    }
    let n = g.read::<u32>(w(0x23));
    if g.read::<u32>(w(0x1b)) == 0 && n.cast_signed() < 0x20 {
        let t = slot_target(g, w(0x30), 296);
        g.call(t, 0, &[n, 0]);
    } else {
        let t = slot_target(g, w(0x3d), 257);
        let p = g.read::<u32>(w(0x45));
        g.call(t, 0, &[p, 1]);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0x64));
    Ret::eax(rec)
}

/// [`count_step`] (107 bytes, 1 overlay).
pub const COUNT_STEP: Sig = Sig {
    len: 107,
    hash: 0x58b9_1ea5_8453_d738,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
