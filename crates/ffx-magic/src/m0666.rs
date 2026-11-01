//! `magic_0666.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::start_pair;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Starts a spark part (0x10001bd0, an overlay table entry, `(obj)`;
/// five table entries): [`start_pair`] with program 10; `+0x40`/`+0x44`
/// 0, `+0x4c`/`+0x6c` 1.0; the first part `+0x48` 2.0 and the colour
/// 0x3c4d5b68, any other -3.0000262 and 0x3b75b1c8; `+0x50..+0x58` 0,
/// `+0x5c` 1. Returns 1.
pub fn spark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = 0x3c4d_5b68;
    let d = 0x3b75_b1c8;
    start_pair(
        g,
        img,
        fr,
        (0x36, 0x28),
        &[(0x40, 0), (0x44, 0), (0x4c, ONE), (0x6c, ONE)],
        &[(0x48, 0x4000_0000), (0x60, c), (0x64, c), (0x68, c)],
        &[(0x48, 0xc040_006d), (0x60, d), (0x64, d), (0x68, d)],
        &[(0x50, 0), (0x54, 0), (0x58, 0), (0x5c, 1)],
    )
}

/// [`spark`] (197 bytes, 1 overlay).
pub const SPARK: Sig = Sig {
    len: 197,
    hash: 0x649d_324e_f27b_55ac,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// Starts a ring part (0x10002370, an overlay table entry, `(obj)`; two
/// table entries): [`start_pair`] with program 3; `+0x44` -0.01, `+0x4c`
/// 1.0; the first part `+0x40`/`+0x48` 2.83 and -3.53, any other -4.27
/// and 1.67; `+0x50..+0x58` 0, `+0x5c` 1, the colour 0x3e018ca0,
/// `+0x6c` 1.0. Returns 1.
pub fn ring<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = 0x3e01_8ca0;
    start_pair(
        g,
        img,
        fr,
        (0x36, 0xc),
        &[(0x44, 0xbc23_d70a), (0x4c, ONE)],
        &[(0x40, 0x4035_1eb0), (0x48, 0xc061_eb81)],
        &[(0x40, 0xc088_a3d5), (0x48, 0x3fd5_c287)],
        &[
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x60, c),
            (0x64, c),
            (0x68, c),
            (0x6c, ONE),
        ],
    )
}

/// [`ring`] (183 bytes, 1 overlay).
pub const RING: Sig = Sig {
    len: 183,
    hash: 0xf951_860b_69ec_7b2a,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// Starts a flare part (0x10002430, an overlay table entry, `(obj)`;
/// two table entries): [`start_pair`] with program 11; `+0x44` -0.01,
/// `+0x4c`/`+0x6c` 1.0, the colour 0x3c01cf78, `+0x58` 0, `+0x5c` 1; the
/// first part `+0x40`/`+0x48` -17.4 and -21.14, `+0x50` 0, `+0x54`
/// 0x7054, any other -12.4, -28.04, -910 and -10920. Returns 1.
pub fn flare<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = 0x3c01_cf78;
    start_pair(
        g,
        img,
        fr,
        (0x36, 0x2c),
        &[
            (0x44, 0xbc23_d70a),
            (0x4c, ONE),
            (0x60, c),
            (0x64, c),
            (0x68, c),
            (0x6c, ONE),
            (0x58, 0),
            (0x5c, 1),
        ],
        &[
            (0x40, 0xc18b_331e),
            (0x48, 0xc1a9_1eaf),
            (0x50, 0),
            (0x54, 0x7054),
        ],
        &[
            (0x40, 0xc146_6638),
            (0x48, 0xc1e0_51c5),
            (0x50, 0xffff_fc72),
            (0x54, 0xffff_d558),
        ],
        &[],
    )
}

/// [`flare`] (198 bytes, 1 overlay).
pub const FLARE: Sig = Sig {
    len: 198,
    hash: 0xccac_7fab_d9de_24cf,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
