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

/// Links the effect's object into the ring (0x10009a00, an overlay
/// table entry, no arguments): the object `O` (the immediate `+0xb`'s
/// home, operand `+0x7`) gets its vtable, the method `+0x15`, itself at
/// `+0x14` (`+0x1b`/`+0x1f`), the old tail `*G_T` (`+0x1`) as its prev
/// and the ring `+0x2e` as its next; the old tail's next and `G_T`
/// become `O`; bit 0 of `+0x3b` cleared, bit 1 of the byte `+0x42` set;
/// `+0x53` the immediate `+0x57`, `+0x5d` and `+0x67` 0. Returns the old
/// tail.
pub fn link_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    let w = |off: u32| img.word(fr.at + off);
    let tail = g.read::<u32>(w(0x1));
    g.write::<u32>(w(0x7), w(0xb));
    g.write::<u32>(w(0x11), w(0x15));
    g.write::<u32>(w(0x1b), w(0x1f));
    g.write::<u32>(w(0x24), tail);
    g.write::<u32>(w(0x2a), w(0x2e));
    g.write::<u32>(tail.wrapping_add(4), w(0x35));
    let v = g.read::<u32>(w(0x3b));
    g.write::<u32>(w(0x3b), v & !1);
    let b = g.read::<u8>(w(0x42));
    g.write::<u8>(w(0x42), b | 2);
    g.write::<u32>(w(0x49), w(0x4d));
    g.write::<u32>(w(0x53), w(0x57));
    g.write::<u32>(w(0x5d), 0);
    g.write::<u32>(w(0x67), 0);
    crate::Ret::eax(tail)
}

/// [`link_object`] (112 bytes, 1 overlay).
pub const LINK_OBJECT: Sig = Sig {
    len: 112,
    hash: 0x31bb_a0b9_45e5_0f24,
    calls: &[],
    imports: &[],
};
