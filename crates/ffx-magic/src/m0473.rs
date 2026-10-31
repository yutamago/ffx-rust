//! `magic_0473.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::sa_pool::take;
use crate::{Callee, Image, Mem, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Starts a scaling object (0x10003c20, internal: `ecx` = the object,
/// `xmm1` and `xmm2` two scales): a scale that is not above 0 becomes
/// `K` (operand `+0x04`; a NaN stays); the two go to the globals at
/// `+0x27` and `+0x2f`, the object's `+0x0..+0x8` (the first, `+0xc`
/// 1.0) and `+0x10..+0x18` (the second, `+0x1c` 1.0); `+0x28` and
/// `+0x34` 0, `+0x2c` and `+0x30` 60. A pool node ([`take`], owner the
/// callback at `+0x1f`) holds the object (`+0x14`, byte `+0x18` 1, `+0x4`
/// 0) at the head of the list (`G` at `+0x81` 0: empty). Returns the
/// node.
pub fn scale_start(g: &mut impl Mem, img: &Image, f: u32, obj: u32, a: f32, b: f32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let k = g.read::<f32>(w(0x04));
    // comiss 0, x; jb: kept when above 0 or unordered
    let fix = |x: f32| if 0.0 < x || x.is_nan() { x } else { k };
    let (a, b) = (fix(a), fix(b));
    g.write::<f32>(w(0x27), a);
    g.write::<f32>(w(0x2f), b);
    for off in [8, 4, 0] {
        g.write::<f32>(obj + off, a);
    }
    g.write::<u32>(obj + 0xc, ONE);
    for off in [0x18, 0x14, 0x10] {
        g.write::<f32>(obj + off, b);
    }
    g.write::<u32>(obj + 0x1c, ONE);
    g.write::<u32>(obj + 0x28, 0);
    g.write::<u32>(obj + 0x34, 0);
    g.write::<u32>(obj + 0x30, 0x3c);
    g.write::<u32>(obj + 0x2c, 0x3c);
    let node = take(g, img, img.rel32_target(f + 0x7b), w(0x1f));
    let empty = g.read::<u32>(w(0x81)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), obj);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    push_head(g, node, empty, (w(0x99), w(0xa3)), (w(0xb2), w(0xb7)));
    node
}

/// [`scale_start`] (191 bytes, 1 overlay).
pub const SCALE_START: Sig = Sig {
    len: 191,
    hash: 0x264a_f6eb_2f5e_63d5,
    calls: &[(0x7b, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};
