//! `magic_0311.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x100018a0, cdecl `(task)`): the countdown `*C`
/// (`+0x04`), when positive, drops; reaching 0 it plays the sound: slot
/// 296 `(*S, 0)` while the flag `+0x16` is clear and `*S` (`+0x1e`) is
/// below 0x20, else slot 257 `(*T, 1)` (`+0x40`). Then the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); when they answer 1
/// and bit 15 of `+0x5b` is set the task gets state 3 and its record the
/// callback `+0x75`. Returns the record, else the objects' answer.
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
        }
    }
    // push ebp; the call's return address
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x50), fr.esp.wrapping_sub(8));
    if r != 1 || g.read::<u32>(w(0x5b)) & 0x8000 == 0 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x75));
    Ret::eax(rec)
}

/// [`frame`] (123 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 123,
    hash: 0x04fc_1b34_f5ef_7f4d,
    calls: &[(0x50, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};
