//! `magic_0245.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x100016e0, cdecl `(task)`): each of the `*N`
/// timers (`N` `+0x05`, read again after each sound; the table `T`
/// `+0x13`) that is positive counts down, and one reaching 0 plays its
/// sound: slot 239 `(*H, B[i], 0)` (`H` `+0x27`, the byte list `*B`
/// `+0x35`; the code's `i == -1` case, which would pass `*H` as the
/// byte, never runs). Then the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both answer 1
/// the task ends: `+0x14` = 3 and its record's `+0x10` = the callback
/// `+0x81`. Returns the record, else the objects' answer.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut n = g.read::<i32>(w(0x05));
    let mut i = 0i32;
    while i < n {
        let at = |op: u32| w(op).wrapping_add(4 * i.cast_unsigned());
        let t = g.read::<i32>(at(0x13));
        if t > 0 {
            let t = t - 1;
            g.write::<i32>(at(0x1f), t);
            if t == 0 {
                let h = g.read::<u32>(w(0x27));
                let b = g.read::<u32>(w(0x35));
                let byte = u32::from(g.read::<u8>(b.wrapping_add(i.cast_unsigned())));
                let s = slot_target(g, w(0x41), 0x3bc / 4);
                g.call(s, 0, &[h, byte, 0]);
                n = g.read::<i32>(w(0x50));
            }
        }
        i += 1;
    }
    let step = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x5d));
    // push ebp; push esi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x64),
        fr.esp.wrapping_sub(0xc),
    );
    if r != 1 || step != 1 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x81));
    Ret::eax(rec)
}

/// [`frame`] (136 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 136,
    hash: 0x10a1_33fc_9196_38d5,
    calls: &[
        (0x5d, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x64, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
