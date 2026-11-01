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

/// Copies the camera's key words (0x100015f0, internal, no arguments;
/// `G_CAM` operand `+0x15`; a frame with the count at `ebp - 8`): for
/// each of the camera's slot 586 `(cam)` tracks `i` (signed), slot 584
/// `(cam, i)` gives the source keys and slot 585 `(cam, i, &n)` the
/// destination and their byte size `n`; the first word of every 16-byte
/// key is copied. Then slot 346 `(cam)` and slot 399 `(cam, 0)` (the
/// camera read again, `+0x93`; 346's word left under it). Returns slot
/// 399's `eax`.
pub fn copy_keys<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let n_at = ebp.wrapping_sub(8);
    let cam = g.read::<u32>(w(0x15));
    g.write::<u32>(ebp.wrapping_sub(4), cam);
    let t = slot_target(g, w(0x7), 586);
    let tracks = g.call(t, 0, &[cam]).eax.cast_signed();
    // ecx: the caller's, then 585's target, then the last copied word
    let mut ecx = fr.ecx;
    let mut i = 0i32;
    while i < tracks {
        let iu = i.cast_unsigned();
        let t = slot_target(g, w(0x32), 584);
        let src = g.call(t, 0, &[cam, iu]).eax;
        let t = slot_target(g, w(0x42), 585);
        let dst = g.call(t, t, &[cam, iu, n_at, cam, iu]).eax;
        ecx = t;
        let mut k = 0i32;
        while k < g.read::<i32>(n_at) {
            let ku = k.cast_unsigned();
            let v = g.read::<u32>(src.wrapping_add(ku));
            g.write::<u32>(dst.wrapping_add(ku), v);
            ecx = v;
            k += 0x10;
        }
        i += 1;
    }
    let t = slot_target(g, w(0x7d), 346);
    g.call(t, ecx, &[cam]);
    let c = g.read::<u32>(w(0x93));
    let t = slot_target(g, w(0x8b), 399);
    g.call(t, ecx, &[c, 0, cam]).eax
}

/// [`copy_keys`] (169 bytes, 1 overlay).
pub const COPY_KEYS: Sig = Sig {
    len: 169,
    hash: 0x43a6_d394_897e_9a23,
    calls: &[],
    imports: &[],
};
