//! The effect wrapper of the functions whose lowest DLL is `magic_0194`
//! (docs/formats/magic.md, "Porting the overlays"; the shape is
//! [`crate::m0196`]'s). Ported from `magic_0194`'s 0x10001230, the lowest of
//! the seven overlays that carry it (`magic_0196` has it at 0x10001180).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// 0x10001230 (63 bytes, 7 overlays): slot 432 with no arguments; slot 450
/// with (`arg`, the callback whose address is the operand at `+0x16`);
/// slot 449 with (`arg`, the callback whose address is the operand at
/// `+0x2b`); returns slot 449's `eax`. The tables are the globals at `+0x04`,
/// `+0x11` and `+0x26`.
pub fn e1230<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let mut s = Pushes::default();
    let slot432 = slot_target(g, w(0x04), 432);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot432, fr.ecx);
    s.push(w(0x16));
    s.push(arg);
    let slot450 = slot_target(g, w(0x11), 450);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot450, fr.ecx);
    s.push(w(0x2b));
    s.push(arg);
    let slot449 = slot_target(g, w(0x26), 449);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot449, fr.ecx);
    Ret::eax(out.eax)
}

/// 0x10001230 (63 bytes, 7 overlays; `magic_0194` is the lowest).
pub const E1230: Sig = Sig {
    len: 63,
    hash: 0xb2f0_ced7_f7bc_19ad,
    calls: &[],
    imports: &[],
};

/// An object's set-up (0x10001280, cdecl `(obj)`): slot 441 `(obj, M,
/// 0x8c)` with `M` = `(*A)[0x198]` (`A` at `+0x04`), the object's words
/// `+0xd0..+0x13c` set, slot 440 `(*A, obj, 0, 1)`, slots 450 `(obj, F1)`
/// and 449 `(obj, F2)` (the immediates `+0xbb`, `+0xce`). Returns slot
/// 449's `eax`. Each slot call passes the words the original leaves on
/// the stack.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let m = g.read::<u32>(g.read::<u32>(w(0x04)) + 0x198);
    let s = slot_target(g, w(0x18), 441);
    g.call(s, 0, &[obj, m, 0x8c]);
    let a = g.read::<u32>(w(0x31));
    for (off, v) in [
        (0xd0, 0xbff3_3333),
        (0xd4, 0),
        (0xd8, 0),
        (0xdc, 0),
        (0x100, 0),
        (0x104, 0),
        (0x108, 0x42b4_0000),
        (0x10c, 0),
        (0x130, 0x3e4c_cccd),
        (0x134, 0x3e4c_cccd),
        (0x138, 0x3e4c_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    let s = slot_target(g, w(0x26), 440);
    g.call(s, 0, &[a, obj, 0, 1, obj, m, 0x8c]);
    let s = slot_target(g, w(0xb6), 450);
    g.call(s, 0, &[obj, w(0xbb), a, obj, 0, 1, obj, m, 0x8c]);
    let s = slot_target(g, w(0xc9), 449);
    // call width: 11 (the earlier calls' words left on the stack)
    let words = [obj, w(0xce), obj, w(0xbb), a, obj, 0, 1, obj, m, 0x8c];
    Ret::eax(g.call(s, 0, &words).eax)
}

/// [`object_start`] (225 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 225,
    hash: 0x9984_0dec_833c_3e29,
    calls: &[],
    imports: &[],
};
