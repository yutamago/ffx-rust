//! `magic_0419.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10001a70, cdecl `(task)`): the countdown `*C`
/// (`+0x04`), when positive, drops; reaching 0 it plays the sound (slot
/// 296 `(*S, 0)` while the flag `+0x1e` is clear and `*S` (`+0x26`) is
/// below 0x20, else slot 257 `(*T, 1)`, `T` `+0x48`), sets the model's
/// colour (slot 256 `(*T, 1, 1, 1, 1)`, the floats 1.0) and, unless the
/// halfword `+0x8c` is set, slot 213 `(*X, 1)` (`+0x9c`) with `+0xad`
/// set to 1. Then the objects ([`run_objects`](crate::l5_objhost::run_objects));
/// when they answer 1, bit 15 of `+0xc1` is set and `*N` (`+0xcd`) is not
/// positive, the task gets state 3 and its record the callback `+0xe4`.
/// Either way a tail jump into the parts' registration
/// ([`register_parts`](crate::sa_misc::register_parts)).
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let c = g.read::<i32>(w(0x04));
    if c > 0 {
        g.write::<i32>(w(0x12), c - 1);
        if c == 1 {
            let sound = g.read::<i32>(w(0x26));
            if g.read::<u32>(w(0x1e)) == 0 && sound < 0x20 {
                let s = slot_target(g, w(0x33), 0x4a0 / 4);
                g.call(s, 0, &[sound.cast_unsigned(), 0]);
            } else {
                let s = slot_target(g, w(0x40), 0x404 / 4);
                let t = g.read::<u32>(w(0x48));
                g.call(s, 0, &[t, 1]);
            }
            let s = slot_target(g, w(0x55), 0x400 / 4);
            let t = g.read::<u32>(w(0x80));
            let one = 0x3f80_0000;
            g.call(s, 0, &[t, one, one, one, 1]);
            if g.read::<u16>(w(0x8c)) == 0 {
                let s = slot_target(g, w(0x94), 0x354 / 4);
                let x = g.read::<u32>(w(0x9c));
                g.call(s, 0, &[x, 1]);
                g.write::<u32>(w(0xad), 1);
            }
        }
    }
    // push ebp; the call's return address
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0xb6), fr.esp.wrapping_sub(8));
    let mut eax = r;
    if r == 1 && g.read::<u32>(w(0xc1)) & 0x8000 != 0 && g.read::<i32>(w(0xcd)) <= 0 {
        let task = fr.arg(g, 0);
        g.write::<u32>(task + 0x14, 3);
        eax = g.read::<u32>(task + 8);
        g.write::<u32>(eax + 0x10, w(0xe4));
    }
    Ret::eax(crate::sa_misc::register_parts(
        g,
        img,
        img.rel32_target(f + 0xea),
        eax,
    ))
}

/// [`frame`] (238 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 238,
    hash: 0xbc61_40d8_c189_fc74,
    calls: &[
        (0xb6, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0xea, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};
