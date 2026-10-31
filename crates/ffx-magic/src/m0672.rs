//! `magic_0672.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Sig};

/// Resets the camera (0x100016a0, internal, no arguments; `G_CAM`
/// operand `+0x08`): slots 357 `(cam, -1, 0x3c00)`, 356 `(cam, -1,
/// 0x2ee0)`, 357 and 356 `(cam, -1, -1)`, 399 `(cam, 1)` (the camera read
/// again, `+0x5e`), each seeing the earlier calls' words. Returns slot
/// 399's `eax`.
pub fn camera_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let cam = g.read::<u32>(w(0x8));
    let mut stack: Vec<u32> = Vec::new();
    for (op, slot, k) in [
        (0x1, 357, 0x3c00),
        (0x1d, 356, 0x2ee0),
        (0x32, 357, u32::MAX),
        (0x44, 356, u32::MAX),
    ] {
        let t = slot_target(g, w(op), slot);
        stack.splice(0..0, [cam, u32::MAX, k]);
        // call width: 12 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    let cam = g.read::<u32>(w(0x5e));
    let t = slot_target(g, w(0x56), 399);
    stack.splice(0..0, [cam, 1]);
    // call width: 14 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack).eax
}

/// [`camera_reset`] (111 bytes, 1 overlay).
pub const CAMERA_RESET: Sig = Sig {
    len: 111,
    hash: 0xf03e_2c3d_b987_4225,
    calls: &[],
    imports: &[],
};
