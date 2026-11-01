//! `magic_0375.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A part start by slot (0x10001bc0, cdecl `(obj)`; `m0351`'s kin):
/// [`part_head`](crate::m0351::part_head) with the list's word 0; with
/// `i` the object's slot in the block `+0x3c` (128 bytes each), slot 0
/// and 1 get their positions `+0x40..+0x48` and the motion words
/// (`+0x4c` 1.0, `+0x50..+0x58` 0, `+0x5c` 1, the scale `+0x60..+0x68`
/// 0.0686 and `+0x6c` 1.0); other slots nothing more. Returns `i`.
pub fn part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (obj, _) = crate::m0351::part_head(g, img, fr, 0, 0x35);
    let i = (obj.wrapping_sub(w(0x3c)).cast_signed() >> 7).cast_unsigned();
    let pos: [u32; 3] = match i {
        0 => [0x4140_0000, 0xc138_0000, 0x4208_0000],
        1 => [0xc110_0000, 0xc148_0000, 0xc1d0_0000],
        _ => return Ret::eax(i),
    };
    for (k, v) in (0u32..).zip(pos) {
        g.write::<u32>(obj + 0x40 + 4 * k, v);
    }
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
    for off in [0x68, 0x64, 0x60] {
        g.write::<u32>(obj + off, 0x3d8c_4828);
    }
    g.write::<u32>(obj + 0x5c, 1);
    for off in [0x58, 0x54, 0x50] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    Ret::eax(i)
}

/// [`part_start`] (189 bytes, 1 overlay).
pub const PART_START: Sig = Sig {
    len: 189,
    hash: 0xbed2_3b0e_3d83_d76f,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
