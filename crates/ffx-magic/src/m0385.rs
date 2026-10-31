//! `magic_0385.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (0x10004aa0, cdecl `(task)`): the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); once both say 1,
/// the count `*C` (`+0x1b`) is 0 or below and bit 15 of both halfwords
/// `(*P)[0x2c]` and `(*Q)[0x18]` (`+0x23`, `+0x33`) is set, the task's
/// state `+0x14` becomes 3 and its record (`task[8]`) gets the callback
/// `+0x4d`. Returns the record, else `run_objects`' answer (or `*P`/`*Q`
/// when its bit is clear).
pub fn step_ready<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let motion = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x05));
    // push ebp; push esi; call: run_objects' esp
    let ran = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x0c),
        fr.esp.wrapping_sub(0xc),
    );
    if ran != 1 || motion != 1 || g.read::<i32>(w(0x1b)) > 0 {
        return Ret::eax(ran);
    }
    for (at, off) in [(0x23, 0x2c), (0x33, 0x18)] {
        let p = g.read::<u32>(w(at));
        if g.read::<u16>(p + off) & 0x8000 == 0 {
            return Ret::eax(p);
        }
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x4d));
    Ret::eax(rec)
}

/// [`step_ready`] (84 bytes, 1 overlay).
pub const STEP_READY: Sig = Sig {
    len: 84,
    hash: 0x7543_2a57_5339_d3a9,
    calls: &[
        (0x05, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x0c, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
