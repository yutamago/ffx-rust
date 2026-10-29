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
