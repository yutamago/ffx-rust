//! `magic_0387.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Frame, Game, Image, Ret, Sig};

/// A sprite node's record set-up (0x10003c70, internal: `ecx` = `[node,
/// parent, a, b]`; no frame): the node under its parent
/// ([`node_under`](crate::m0186::node_under)); `+0x3dc` = a, `+0x3ec` =
/// b, the byte `+0x3e8` 0, `+0x3e0`/`+0x3e4` 1.0 and `+0x3f0..+0x3f8` 0.
/// Returns b.
pub fn sprite_record<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let r = fr.ecx;
    let node = g.read::<u32>(r);
    crate::m0186::node_under(g, node, g.read::<u32>(r + 4));
    g.write::<u32>(node + 0x3dc, g.read::<u32>(r + 8));
    let b = g.read::<u32>(r + 0xc);
    g.write::<u8>(node + 0x3e8, 0);
    g.write::<u32>(node + 0x3e0, 0x3f80_0000);
    g.write::<u32>(node + 0x3e4, 0x3f80_0000);
    for off in [0x3f0, 0x3f4, 0x3f8] {
        g.write::<u32>(node + off, 0);
    }
    g.write::<u32>(node + 0x3ec, b);
    Ret::eax(b)
}

/// [`sprite_record`] (133 bytes, 1 overlay).
pub const SPRITE_RECORD: Sig = Sig {
    len: 133,
    hash: 0x37a4_a88f_c48f_e633,
    calls: &[],
    imports: &[],
};

/// A strip node's record set-up (0x100048b0, internal: `ecx` = `[node,
/// parent, a, (u16 b, u16 c), d]`; no frame): the node under its parent
/// ([`node_under`](crate::m0186::node_under)); `+0x1c` = a, the
/// halfwords `+0x20` = b and `+0x22` = c, `+0x24..+0x29` and
/// `+0x4c..+0x54` 0, `+0x6c` = d, `+0x38`/`+0x70` 1.0, `+0x78`/`+0x74` 0
/// and `+0x7c` 0x10001. Returns 0.
pub fn strip_record<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let r = fr.ecx;
    let node = g.read::<u32>(r);
    crate::m0186::node_under(g, node, g.read::<u32>(r + 4));
    g.write::<u32>(node + 0x1c, g.read::<u32>(r + 8));
    g.write::<u16>(node + 0x20, g.read::<u16>(r + 0xc));
    g.write::<u16>(node + 0x22, g.read::<u16>(r + 0xe));
    g.write::<u32>(node + 0x26, 0);
    g.write::<u32>(node + 0x24, 0);
    for off in [0x4c, 0x50, 0x54] {
        g.write::<u32>(node + off, 0);
    }
    g.write::<u32>(node + 0x6c, g.read::<u32>(r + 0x10));
    g.write::<u32>(node + 0x38, 0x3f80_0000);
    g.write::<u32>(node + 0x70, 0x3f80_0000);
    g.write::<u16>(node + 0x28, 0);
    g.write::<u32>(node + 0x78, 0);
    g.write::<u32>(node + 0x7c, 0x0001_0001);
    g.write::<u32>(node + 0x74, 0);
    Ret::eax(0)
}

/// [`strip_record`] (136 bytes, 1 overlay).
pub const STRIP_RECORD: Sig = Sig {
    len: 136,
    hash: 0x77e9_bd11_c62a_55c7,
    calls: &[],
    imports: &[],
};
