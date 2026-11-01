//! `magic_0672.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

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

/// The fade-out task (0x10001b80, `(task)`, a task step): the countdown
/// `n = S[0x5b8]` (`S = *G_S`) down by one while above 0; slot 348
/// `(*G_B, *G_H, (80 - n) * 256 / 80, (n - 80) * 64 / 80, (n - 80) * 40 /
/// 80, 0)` (signed divisions); once the countdown is out the task goes
/// back to its pool (a tail jump into
/// [`give_back`](crate::sa_pool::give_back)). Returns slot 348's `eax` or
/// the give-back's.
pub fn fade_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x6));
    let c = g.read::<i32>(s + 0x5b8);
    if c > 0 {
        g.write::<i32>(s + 0x5b8, c - 1);
    }
    let n = g.read::<i32>(s + 0x5b8);
    let d = n.wrapping_sub(0x50);
    let a = d.wrapping_mul(40) / 80;
    let b = d.wrapping_shl(6) / 80;
    let e = 0x50i32.wrapping_sub(n).wrapping_shl(8) / 80;
    let h = g.read::<u32>(w(0x73));
    let t = slot_target(g, w(0x78), 348);
    let bg = g.read::<u32>(w(0x7e));
    let r = g
        .call(
            t,
            0,
            &[
                bg,
                h,
                e.cast_unsigned(),
                b.cast_unsigned(),
                a.cast_unsigned(),
                0,
            ],
        )
        .eax;
    if g.read::<i32>(s + 0x5b8) > 0 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x9e),
        task,
    ))
}

/// [`fade_task`] (164 bytes, 1 overlay).
pub const FADE_TASK: Sig = Sig {
    len: 164,
    hash: 0x0982_6d64_dfc8_364e,
    calls: &[(0x9e, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// Starts the fade (0x10001c30, no arguments): the colour block `E =
/// S + 0x280` (`S = *G_S`) set (`+0x80` 0xff, `+0x81` 0xff80ffff, `+0x85`
/// 0xffff, `+0x87` 0, `+0x88` 0, `+0x8c`/`+0x90` 30, `+0x94` 0, `+0x98`
/// 0x48); a pool node ([`take`](crate::sa_pool::take)) with `+0x14` =
/// `E` and `+0x4` 0 put at the head of the DLL's list
/// ([`push_head`](crate::o4_more::push_head)); slot 587 `(*G_B, 0)`.
/// Returns slot 587's `eax`.
pub fn fade_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x3)) + 0x280;
    g.write::<u32>(e + 0x81, 0xff80_ffff);
    g.write::<u8>(e + 0x80, 0xff);
    g.write::<u16>(e + 0x85, 0xffff);
    g.write::<u8>(e + 0x87, 0);
    g.write::<u32>(e + 0x88, 0);
    g.write::<u16>(e + 0x98, 0x48);
    g.write::<u32>(e + 0x90, 0x1e);
    g.write::<u32>(e + 0x8c, 0x1e);
    g.write::<u32>(e + 0x94, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x65), w(0xe));
    let empty = g.read::<u32>(w(0x6b)) == 0;
    g.write::<u32>(node + 0x14, e);
    g.write::<u32>(node + 4, 0);
    crate::o4_more::push_head(g, node, empty, (w(0x7f), w(0x99)), (w(0x91), w(0x99)));
    let t = slot_target(g, w(0x9e), 587);
    let b = g.read::<u32>(w(0xa6));
    Ret::eax(g.call(t, 0, &[b, 0]).eax)
}

/// [`fade_start`] (182 bytes, 1 overlay).
pub const FADE_START: Sig = Sig {
    len: 182,
    hash: 0xfa52_48bd_5b17_2637,
    calls: &[(0x65, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};
