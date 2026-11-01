//! `magic_0199.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (0x10001960, cdecl `(task)`): the motion script of the
/// record at `+0x05` ([`motion_script`](crate::s4_misc::motion_script))
/// and the objects run ([`run_objects`](crate::l5_objhost::run_objects));
/// when both give 1 and `*G_F` (`+0x20`) has bit 15, the task's state is
/// 3 and its record's `+0x10` the immediate `+0x3a`. Returns the record,
/// else the objects' result.
pub fn script_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let m = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0xa), w(0x5));
    // push ebp; push esi; call
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x11), fr.esp.wrapping_sub(12));
    if r != 1 || m != 1 || g.read::<u32>(w(0x20)) & 0x8000 == 0 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0x3a));
    Ret::eax(rec)
}

/// [`script_step`] (65 bytes, 1 overlay).
pub const SCRIPT_STEP: Sig = Sig {
    len: 65,
    hash: 0x6849_63b5_0c94_52d6,
    calls: &[
        (0xa, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x11, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
