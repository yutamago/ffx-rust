//! `magic_0447.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Frame, Game, Image, Ret, Sig};

/// The camera's copy (0x10005430, no arguments): with `E` = `*G`
/// (`+0x02`) the quads `E + 0x600` and `E + 0x5f0` go to `E + 0x610` and
/// `E + 0x620`; `E + 0x7cc` and `E + 0x7c8` become 30. Returns `E +
/// 0x5fc`'s word (the last one copied).
pub fn camera_copy<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let e = g.read::<u32>(img.word(fr.at + 0x02));
    let mut v = 0;
    for (src, dst) in [(0x600, 0x610), (0x5f0, 0x620)] {
        for k in 0..4 {
            v = g.read::<u32>(e + src + 4 * k);
            g.write::<u32>(e + dst + 4 * k, v);
        }
    }
    g.write::<u32>(e + 0x7cc, 0x1e);
    g.write::<u32>(e + 0x7c8, 0x1e);
    Ret::eax(v)
}

/// [`camera_copy`] (123 bytes, 1 overlay).
pub const CAMERA_COPY: Sig = Sig {
    len: 123,
    hash: 0xd094_08de_3142_fae2,
    calls: &[],
    imports: &[],
};
