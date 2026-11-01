//! `magic_0325.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10001910, cdecl `(task)`): the countdown `*C`
/// (`+0x04`), when positive, drops; reaching 0 it plays the sound (slot
/// 296 `(*S, 0)` while the flag `+0x16` is clear and `*S` (`+0x1e`) is
/// below 0x20, else slot 257 `(*T, 1)`, `T` `+0x40`) and stops it again
/// (slot 525 `(*T)`, `+0x5c`). Then the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)) and the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)); when both answer 1
/// the task gets state 3 and its record the callback `+0x8b`. Returns the
/// record, else the motion step's answer.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let c = g.read::<i32>(w(0x04));
    if c > 0 {
        g.write::<i32>(w(0x0e), c - 1);
        if c == 1 {
            let sound = g.read::<i32>(w(0x1e));
            if g.read::<u32>(w(0x16)) == 0 && sound < 0x20 {
                let s = slot_target(g, w(0x2b), 0x4a0 / 4);
                g.call(s, 0, &[sound.cast_unsigned(), 0]);
            } else {
                let s = slot_target(g, w(0x38), 0x404 / 4);
                let t = g.read::<u32>(w(0x40));
                g.call(s, 0, &[t, 1]);
            }
            let s = slot_target(g, w(0x4d), 0x834 / 4);
            let t = g.read::<u32>(w(0x5c));
            g.call(s, 0, &[t]);
        }
    }
    // push ebp; push esi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x67),
        fr.esp.wrapping_sub(0xc),
    );
    let step = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x6e));
    if step != 1 || r != 1 {
        return Ret::eax(step);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x8b));
    Ret::eax(rec)
}

/// [`frame`] (146 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 146,
    hash: 0xed37_fa18_6c94_e014,
    calls: &[
        (0x67, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x6e, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
    ],
    imports: &[],
};
