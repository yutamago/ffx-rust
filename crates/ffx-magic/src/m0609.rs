//! `magic_0609.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::{Callee, Frame, Game, Image, Sig};

/// Starts the DLL's static task `T` (0x100095d0, internal: `edx` = an
/// effect object): `T`'s vtable, callback and own word (the operand
/// pairs at `+0x0a`, `+0x14`, `+0x1e`) set, linked at the end of the task
/// ring (the last word `+0x01`/`+0x27`, the sentinel `+0x31`, the flag
/// word `+0x3e` bit 0 cleared, the last word written `+0x4c`); the object
/// gets the value 0xbc ([`set_value`](crate::m0042::set_value)); `T`'s
/// object word `+0x5b` = it, `+0x61` and `+0x6b` 0. Returns the
/// set-value's `eax`.
pub fn task_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.edx;
    let tail = g.read::<u32>(w(0x1));
    for (at, v) in [(0xa, 0xe), (0x14, 0x18), (0x1e, 0x22)] {
        g.write::<u32>(w(at), w(v));
    }
    g.write::<u32>(w(0x27), tail);
    g.write::<u32>(w(0x2d), w(0x31));
    g.write::<u32>(tail.wrapping_add(4), w(0x38));
    let fl = g.read::<u32>(w(0x3e));
    g.write::<u32>(w(0x3e), fl & !1);
    g.write::<u32>(w(0x4c), w(0x50));
    let eax = crate::m0042::set_value(g, img, img.rel32_target(f + 0x55), obj, 0xbc);
    g.write::<u32>(w(0x5b), obj);
    g.write::<u32>(w(0x61), 0);
    g.write::<u32>(w(0x6b), 0);
    eax
}

/// [`task_start`] (117 bytes, 1 overlay).
pub const TASK_START: Sig = Sig {
    len: 117,
    hash: 0xc651_5745_8d91_3171,
    calls: &[(0x55, Callee::Sig(&crate::m0042::SET_VALUE))],
    imports: &[],
};
