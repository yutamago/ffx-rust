//! `magic_0183.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::add;
use crate::{Image, Sig};

/// A point by the camera (0x100012e0, internal: `ecx` = the point): slot
/// 233 `(v, M, K)` (`M` the immediate `+0x1d`, the float `K` from
/// `+0x04`), then `v.x += A`, `v.y = (B + C) + v.y`, `v.z += D` (`+0x2c`,
/// `+0x3b`, `+0x43`, `+0x5a`). Returns slot 233's `eax`.
pub fn camera_point<G: crate::Game>(g: &mut G, img: &Image, f: u32, v: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let fl = |g: &G, at: u32| g.read::<f32>(at);
    let k = g.read::<u32>(w(0x04));
    let s = slot_target(g, w(0x09), 233);
    let eax = g.call(s, v, &[v, w(0x1d), k]).eax;
    let x = add(fl(g, v), fl(g, w(0x2c)));
    g.write::<u32>(v, x.to_bits());
    let y = add(add(fl(g, w(0x3b)), fl(g, w(0x43))), fl(g, v + 4));
    g.write::<u32>(v + 4, y.to_bits());
    let z = add(fl(g, v + 8), fl(g, w(0x5a)));
    g.write::<u32>(v + 8, z.to_bits());
    eax
}

/// [`camera_point`] (101 bytes, 1 overlay).
pub const CAMERA_POINT: Sig = Sig {
    len: 101,
    hash: 0x80bc_98f4_4f6f_d7fb,
    calls: &[],
    imports: &[],
};
