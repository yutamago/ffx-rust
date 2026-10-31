//! `magic_0351.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// 0x10002770 (no arguments): 0 when `*A` (`+0x02`) is set; 1 while the
/// clock `*T` (`+0x0b`) is below 0x28; else for the first three `i` below
/// the count `*N` (`+0x21`) whose flag `F[i]` is set (the table at
/// `+0x2d`), slot 239 `(*H, (*B)[i], 0)` (`H` at `+0x47`, the bytes `B`
/// at `+0x35`). Returns 0.
pub fn sounds_3<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    if g.read::<u32>(w(0x02)) != 0 {
        return Ret::eax(0);
    }
    if g.read::<i32>(w(0x0b)) < 0x28 {
        return Ret::eax(1);
    }
    for i in 0..3u32 {
        if i.cast_signed() > g.read::<i32>(w(0x21)).wrapping_sub(1) {
            break;
        }
        if g.read::<u32>(w(0x2d).wrapping_add(4 * i)) == 0 {
            continue;
        }
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x35)).wrapping_add(i)));
        let h = g.read::<u32>(w(0x47));
        let s = slot_target(g, w(0x41), 239);
        g.call(s, 0, &[h, b, 0]);
    }
    Ret::eax(0)
}

/// [`sounds_3`] (96 bytes, 1 overlay).
pub const SOUNDS_3: Sig = Sig {
    len: 96,
    hash: 0x3f08_74f1_0ded_cfec,
    calls: &[],
    imports: &[],
};
