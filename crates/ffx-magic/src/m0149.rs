//! `magic_0149.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's step (0x10003950, cdecl `(task)`): two countdowns
/// (`+0x04`, `+0x40`) that, when they reach 0 from above, each give slot
/// 306 `(7, 0 / 1, h)` the handle `h` of slot 719 `(NAME, 9, 0)`
/// (`NAME` the immediates `+0x24`, `+0x60`); the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); once both say 1,
/// the task's state `+0x14` becomes 3 and its record (`task[8]`) gets
/// the callback `+0xa1`. Returns the record, else `run_objects`' answer.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn countdown_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    for (count, name, t719, t306, k) in [
        (0x04u32, 0x24u32, 0x15u32, 0x2cu32, 0u32),
        (0x40, 0x60, 0x51, 0x68, 1),
    ] {
        let c = g.read::<i32>(w(count));
        if c > 0 {
            let c = c - 1;
            g.write::<i32>(w(count), c);
            if c == 0 {
                let s = slot_target(g, w(t719), 719);
                let h = g.call(s, 0, &[w(name), 9, 0]).eax;
                let s = slot_target(g, w(t306), 306);
                // call width: 6 (slot 719's words left on the stack)
                g.call(s, 0, &[7, k, h, w(name), 9, 0]);
            }
        }
    }
    let motion = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x7d));
    // push ebp; push esi; call: run_objects' esp
    let ran = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x84),
        fr.esp.wrapping_sub(0xc),
    );
    if ran != 1 || motion != 1 {
        return Ret::eax(ran);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0xa1));
    Ret::eax(rec)
}

/// [`countdown_step`] (168 bytes, 1 overlay).
pub const COUNTDOWN_STEP: Sig = Sig {
    len: 168,
    hash: 0xf7e5_474c_97b6_a58c,
    calls: &[
        (0x7d, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x84, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
