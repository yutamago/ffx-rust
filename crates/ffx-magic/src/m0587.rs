//! `magic_0587.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::m0018::st0_f32;
use crate::o4_more::push_head;
use crate::runtime::slot_target;
use crate::sa_misc::scale_rotate;
use crate::sa_pool::take;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Resets the effect's camera (0x10001a90, no arguments; `G_CAM` operand
/// `+0x08`): slots 357 `(cam, -1, 0x3c00)`, 356 `(cam, -1, 0x2ee0)`, 562,
/// 362 `(G_A, cam, 2, slot 562's answer)`, 357 and 356 `(cam, -1, -1)`
/// (the camera read again), each seeing the earlier calls' words.
/// Returns the last call's `eax`.
pub fn camera_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let cam = g.read::<u32>(w(0x8));
    let t = slot_target(g, w(0x1), 357);
    let mut stack = vec![cam, u32::MAX, 0x3c00];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x1d), 356);
    stack.splice(0..0, [cam, u32::MAX, 0x2ee0]);
    // call width: 6 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x32), 562);
    // call width: 6 (the earlier calls' words stay on the stack)
    let x = g.call(t, 0, &stack).eax;
    let t = slot_target(g, w(0x40), 362);
    let a = g.read::<u32>(w(0x4f));
    stack.splice(0..0, [a, cam, 2, x]);
    // call width: 10 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x56), 357);
    let cam = g.read::<u32>(w(0x5c));
    stack.splice(0..0, [cam, u32::MAX, u32::MAX]);
    // call width: 13 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x6e), 356);
    stack.splice(0..0, [cam, u32::MAX, u32::MAX]);
    // call width: 16 (the earlier calls' words stay on the stack)
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// [`camera_reset`] (132 bytes, 1 overlay).
pub const CAMERA_RESET: Sig = Sig {
    len: 132,
    hash: 0x2b22_75fb_c161_9de4,
    calls: &[],
    imports: &[],
};

/// Loads the four groups' entries (0x100021f0, no arguments; the
/// record `R = *G_R`, operand `+0x08`): group `k` (the count word at `R +
/// 0x166c + 0x8a8 k`, signed) gets, for each entry `j`, slot 941
/// `(R[0xe48 + 0x8a8 k])`, slot 772 `(that, 0x10000 k + j)` and slot 779 with
/// slot 772's `eax`/`edx`, the word at `*G_F` (operand `+0x5b`) 1 around
/// the call. Returns `*G_R` as last read.
pub fn load_groups<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut r = g.read::<u32>(w(0x8));
    let mut base = 0u32;
    let mut off = 0x166cu32;
    while off < 0x390c {
        let mut j = 0i32;
        while j < g.read::<i32>(off.wrapping_add(r)) {
            let x = g.read::<u32>((off - 0x824).wrapping_add(r));
            let t = slot_target(g, w(0x38), 941);
            let a = g.call(t, 0, &[x]).eax;
            let t = slot_target(g, w(0x4d), 772);
            let b = base.wrapping_add(j.cast_unsigned());
            let o = g.call(t, 0, &[a, b]);
            let fl = g.read::<u32>(w(0x5b));
            let saved = g.read::<u32>(fl);
            g.write::<u32>(fl, 1);
            let t = slot_target(g, w(0x64), 779);
            g.call(t, 0, &[o.eax, o.edx, a, b]);
            let fl = g.read::<u32>(w(0x77));
            g.write::<u32>(fl, saved);
            r = g.read::<u32>(w(0x7f));
            j += 1;
        }
        off += 0x8a8;
        base = base.wrapping_add(0x1_0000);
    }
    Ret::eax(r)
}

/// [`load_groups`] (178 bytes, 1 overlay).
pub const LOAD_GROUPS: Sig = Sig {
    len: 178,
    hash: 0xc7c3_44dc_f5be_19cf,
    calls: &[],
    imports: &[],
};

/// Starts the glow (0x100022b0, no arguments): slot 371 `(0, 0x10000)`
/// and `(1, 0x20000)` (the first call's words under the second's); the
/// record `R = *G_R + 0xaa0` (operand `+0x0e`) gets its colours `+0x80`
/// 0x80ffffff, `+0x84` 0xffffff, `+0x88` 0, kind `+0x98` 0x48, sizes
/// `+0x8c`/`+0x90` 40, `+0x94` 0; a pool node ([`take`], the owner the
/// immediate at `+0x39`) with `+0x14` = `R`, `+0x4` 0, put at the head of
/// the list ([`push_head`]: head `+0x9d`/`+0xa7`, empty test `+0x89`, set
/// `+0xb6`/`+0xbb`). Returns the node.
pub fn glow_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 371);
    let r = g.read::<u32>(w(0xe));
    g.call(t, 0, &[0, 0x1_0000]);
    let t = slot_target(g, w(0x1c), 371);
    g.call(t, 0, &[1, 0x2_0000, 0, 0x1_0000]);
    let r = r.wrapping_add(0xaa0);
    g.write::<u32>(r + 0x80, 0x80ff_ffff);
    g.write::<u32>(r + 0x84, 0x00ff_ffff);
    g.write::<u32>(r + 0x88, 0);
    g.write::<u16>(r + 0x98, 0x48);
    g.write::<u32>(r + 0x90, 0x28);
    g.write::<u32>(r + 0x8c, 0x28);
    g.write::<u32>(r + 0x94, 0);
    let node = take(g, img, img.rel32_target(f + 0x83), w(0x39));
    let empty = g.read::<u32>(w(0x89)) == 0;
    g.write::<u32>(node + 0x14, r);
    g.write::<u32>(node + 4, 0);
    push_head(g, node, empty, (w(0x9d), w(0xa7)), (w(0xb6), w(0xbb)));
    Ret::eax(node)
}

/// [`glow_start`] (195 bytes, 1 overlay).
pub const GLOW_START: Sig = Sig {
    len: 195,
    hash: 0x08db_5fb2_4385_8296,
    calls: &[(0x83, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// Sets the glow's colour (0x10002380, cdecl `(src)`; a frame with a
/// stack cookie): in the record `R = *G_R` (operand `+0x33`), `+0xe20..
/// +0xe2c` = `+0xe10..+0xe1c`, `+0xe30..+0xe38` = `src[0x2c..0x34]`,
/// `+0xe3c` 0x80, `+0xe44` and `+0xe40` = `src[0x38]`. Returns `R[0xe1c]`.
pub fn glow_colour<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let src = fr.arg(g, 0);
    let c: [u32; 3] = std::array::from_fn(|i| g.read::<u32>(src + 0x2c + 4 * i as u32));
    let d = g.read::<u32>(src + 0x38);
    let r = g.read::<u32>(w(0x33));
    let mut eax = 0;
    for i in 0..4 {
        eax = g.read::<u32>(r + 0xe10 + 4 * i);
        g.write::<u32>(r + 0xe20 + 4 * i, eax);
    }
    for (i, v) in (0u32..).zip(c) {
        g.write::<u32>(r + 0xe30 + 4 * i, v);
    }
    g.write::<u32>(r + 0xe3c, 0x80);
    g.write::<u32>(r + 0xe44, d);
    g.write::<u32>(r + 0xe40, d);
    Ret::eax(eax)
}

/// [`glow_colour`] (179 bytes, 1 overlay).
pub const GLOW_COLOUR: Sig = Sig {
    len: 179,
    hash: 0x9c4f_80f8_5294_e635,
    // __security_check_cookie
    calls: &[(0xab, Callee::Any)],
    imports: &[],
};

/// Turns the glow (0x100031f0, no arguments; the record `R = *G_R`,
/// operand `+0x03`): the speed `+0xa84` drops by `+0xa94`; the angle
/// `+0xa44` grows by it, wrapped once by `W` (`+0x38`) above `H`
/// (`+0x2e`) or below `L` (`+0x42`); the matrix `+0x980` is scaled and
/// turned by the vector `+0xa40` ([`scale_rotate`]); `+0xa54` grows by
/// `S` (`+0x79`); the four counters (bytes `+0x1657 + 0x8a8 k`) count up
/// to 24. Returns the scale-rotate's `eax` with the last counter in its
/// low byte.
pub fn glow_turn<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &mut G, off: u32| g.read::<f32>(w(off));
    let r = g.read::<u32>(w(0x3));
    let x = sub(g.read::<f32>(r + 0xa84), g.read::<f32>(r + 0xa94));
    g.write::<f32>(r + 0xa84, x);
    let mut y = add(g.read::<f32>(r + 0xa44), x);
    let hi = k(g, 0x2e);
    // comiss y, H; jbe: below, equal or unordered
    if y > hi {
        y = sub(y, k(g, 0x38));
    } else {
        let lo = k(g, 0x42);
        // comiss L, y; jbe
        if lo > y {
            y = add(y, k(g, 0x4f));
        }
    }
    g.write::<f32>(r + 0xa44, y);
    let eax = scale_rotate(g, img, img.rel32_target(f + 0x69), r + 0x980, r + 0xa40);
    let s = add(g.read::<f32>(r + 0xa54), k(g, 0x79));
    g.write::<f32>(r + 0xa54, s);
    let mut low = 0u8;
    for i in 0..4 {
        let at = r + 0x1657 + 0x8a8 * i;
        low = g.read::<u8>(at);
        if low < 0x18 {
            low += 1;
            g.write::<u8>(at, low);
        }
    }
    Ret::eax(eax & !0xff | u32::from(low))
}

/// [`glow_turn`] (210 bytes, 1 overlay).
pub const GLOW_TURN: Sig = Sig {
    len: 210,
    hash: 0xcecd_208a_6876_e285,
    calls: &[(0x69, Callee::Sig(&crate::sa_misc::SCALE_ROTATE))],
    imports: &[],
};

/// The summon's sway frame (0x100029b0, an overlay table entry, no
/// arguments; a /GS frame) on the summon block `S = *G_S`: the two
/// offsets `+0xa90`/`+0xa98` lose their steps `+0xa80`/`+0xa88`, each
/// plus a random float (slot 314 `(0.1)`, ST0) into `+0xa50`/`+0xa58`;
/// the matrix `S + 0x980` scaled and rotated by `S + 0xa40`
/// ([`scale_rotate`](crate::sa_misc::scale_rotate)); the angle `+0xa84`
/// steps by `K1`, wrapped by `K3` above `K2` or below `K4`, and turns the
/// matrix (slot 235 `(&L)`, slot 329 `(&L, &L, angle)`, slot 231 `(M, M,
/// &L)`; `L` at `ebp - 0x44`); the three u16 levels `+0x660..+0x664`
/// become `+0x664 - 17`, and `+0x6c0`/`+0x6c8` lose `+0xa94`/`+0xa9c`.
/// Returns the level (`eax`, 32 bits).
pub fn sway_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let fl = |g: &G, a: u32| g.read::<f32>(a);
    let t = slot_target(g, w(0x11), 314);
    let s = g.read::<u32>(w(0x18));
    let x = sub(fl(g, s + 0xa90), fl(g, s + 0xa80));
    g.write::<f32>(s + 0xa90, x);
    let y = sub(fl(g, s + 0xa98), fl(g, s + 0xa88));
    g.write::<f32>(s + 0xa98, y);
    let r = st0_f32(g.call(t, 0, &[0x3dcc_cccd]).st0);
    let v = add(fl(g, s + 0xa90), r);
    let t = slot_target(g, w(0x6e), 314);
    g.write::<f32>(s + 0xa50, v);
    let r = st0_f32(g.call(t, 0, &[0x3dcc_cccd]).st0);
    let v = add(fl(g, s + 0xa98), r);
    let m = s + 0x980;
    g.write::<f32>(s + 0xa58, v);
    scale_rotate(g, img, img.rel32_target(f + 0xb1), m, s + 0xa40);
    let mut a = add(fl(g, s + 0xa84), k(g, 0xc1));
    if a > k(g, 0xcb) {
        a = sub(a, k(g, 0xd5));
    } else if k(g, 0xdf) > a {
        a = add(a, k(g, 0xec));
    }
    let l = fr.esp.wrapping_sub(4).wrapping_sub(0x44);
    let t = slot_target(g, w(0xf5), 235);
    g.write::<f32>(s + 0xa84, a);
    g.call(t, 0, &[l]);
    let a = g.read::<u32>(s + 0xa84);
    let t = slot_target(g, w(0x11c), 329);
    let mut stack = vec![l, l, a];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x12d), 231);
    stack.splice(0..0, [m, m, l]);
    // call width: 6
    g.call(t, 0, &stack);
    let level = u32::from(g.read::<u16>(s + 0x664)).wrapping_sub(0x11);
    let z = fl(g, s + 0x6c0);
    #[allow(clippy::cast_possible_truncation)] // the low 16 bits
    for o in [0x664u32, 0x662, 0x660] {
        g.write::<u16>(s + o, level as u16);
    }
    let z = sub(z, fl(g, s + 0xa94));
    g.write::<f32>(s + 0x6c0, z);
    let z = sub(fl(g, s + 0x6c8), fl(g, s + 0xa9c));
    g.write::<f32>(s + 0x6c8, z);
    Ret::eax(level)
}

/// [`sway_frame`] (413 bytes, 1 overlay).
pub const SWAY_FRAME: Sig = Sig {
    len: 413,
    hash: 0xb5f1_e038_cbec_0b60,
    calls: &[
        (0xb1, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        // __security_check_cookie
        (0x195, Callee::Any),
    ],
    imports: &[],
};

/// The record's `+0x30fc` cleared (`R = *G_R`, operand `+0x1`), then
/// [`load_groups`] (0x10002f20, no arguments; a tail jump). Returns its
/// result.
pub fn reload_groups<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let r = g.read::<u32>(img.word(fr.at + 0x1));
    g.write::<u32>(r.wrapping_add(0x30fc), 0);
    let callee = Frame {
        at: img.rel32_target(fr.at + 0x10),
        ..*fr
    };
    load_groups(g, img, &callee)
}

/// [`reload_groups`] (20 bytes, 1 overlay).
pub const RELOAD_GROUPS: Sig = Sig {
    len: 20,
    hash: 0x9176_b07c_9612_1f52,
    calls: &[(0x10, Callee::Sig(&LOAD_GROUPS))],
    imports: &[],
};

/// Ends the effect's models (0x10001920, an overlay table entry, no
/// arguments): slots 6 `(m, 0)` and 547 `(m)` on the model `*G_M`
/// (operand `+0x8`), slot 295 `(*G_A, -1)` (`+0x2b`); then unless the
/// flag `+0x3c` is set and while the actor `*G_A` is below 0x20 slot 541
/// `(a, 1.0, 1)`, else slot 38 `(m, 1.0, 1)`. The node `*G_N` (`+0x85`),
/// when there is one, goes back to the pool
/// ([`give_back`](crate::sa_pool::give_back)) and `G_N` = 0; slots 568
/// `(1)`, 566 `(1)`, 546 `(m)`, 271 `(1)`; then the groups load
/// ([`load_groups`], a tail jump), whose answer it returns. Each slot
/// call sees the words the earlier ones left on the stack.
pub fn models_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = g.read::<u32>(w(0x8));
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[0, m]);
    let t = slot_target(g, w(0x1), 6);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, fr.ecx);
    p.push(m);
    let t = slot_target(g, w(0x15), 547);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, fr.ecx);
    let a = g.read::<u32>(w(0x2b));
    p.push_all(&[u32::MAX, a]);
    let t = slot_target(g, w(0x23), 295);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, fr.ecx);
    p.pop(5);
    let a = g.read::<u32>(w(0x45));
    let mut q = crate::pushes::Pushes::default();
    let t = if g.read::<u32>(w(0x3c)) == 0 && a.cast_signed() < 0x20 {
        q.push_all(&[1, 0x3f80_0000, a]);
        slot_target(g, w(0x5a), 541)
    } else {
        let m = g.read::<u32>(w(0x7d));
        q.push_all(&[1, 0x3f80_0000, m]);
        slot_target(g, w(0x67), 38)
    };
    // call width: 12 (Pushes caps the words at 12)
    q.call(g, t, fr.ecx);
    let node = g.read::<u32>(w(0x85));
    if node != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x91), node);
        g.write::<u32>(w(0x97), 0);
    }
    let mut r = crate::pushes::Pushes::default();
    for (op, slot) in [(0xa0, 568), (0xaf, 566)] {
        r.push(1);
        let t = slot_target(g, w(op), slot);
        // call width: 12 (Pushes caps the words at 12)
        r.call(g, t, 0);
    }
    r.push(g.read::<u32>(w(0xc4)));
    let t = slot_target(g, w(0xbe), 546);
    // call width: 12 (Pushes caps the words at 12)
    r.call(g, t, 0);
    r.push(1);
    let t = slot_target(g, w(0xd1), 271);
    // call width: 12 (Pushes caps the words at 12)
    r.call(g, t, 0);
    let load = Frame {
        at: img.rel32_target(f + 0xe3),
        ..*fr
    };
    load_groups(g, img, &load)
}

/// [`models_end`] (231 bytes, 1 overlay).
pub const MODELS_END: Sig = Sig {
    len: 231,
    hash: 0x4f5f_ecf1_4ff8_8fb0,
    calls: &[
        (0x91, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xe3, Callee::Sig(&LOAD_GROUPS)),
    ],
    imports: &[],
};

/// Starts the glow scale (0x10003a20, an overlay table entry, no
/// arguments): `*G_FX`'s point `+0xe10` copied to `+0xe20`, the colour
/// `+0xe30..` the four globals at operands `+0x1a`, `+0xa`, `+0x12`,
/// `+0x22`, the counts `+0xe44`/`+0xe40` 20; the scale task of `+0xc78`
/// ([`scale_task_fixed`](crate::s4_small::scale_task_fixed): `xmm1` 0,
/// `xmm2` = `*K` `+0x6a`, `*G_FX` pushed under it); then slot 560 `(slot
/// 557 () & !4)`, whose `eax` it returns.
pub fn glow_scale_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x2));
    let c = [w(0x1a), w(0xa), w(0x12), w(0x22)].map(|a| g.read::<u32>(a));
    let mut last = 0;
    for i in 0..4 {
        last = g.read::<u32>(fx + 0xe10 + 4 * i);
        g.write::<u32>(fx + 0xe20 + 4 * i, last);
    }
    for (o, v) in [(0xe34, c[1]), (0xe38, c[2]), (0xe30, c[0]), (0xe3c, c[3])] {
        g.write::<u32>(fx + o, v);
    }
    g.write::<u32>(fx + 0xe44, 0x14);
    g.write::<u32>(fx + 0xe40, 0x14);
    // push ecx (*G_FX), the return address
    let esp = fr.esp.wrapping_sub(8);
    g.write::<u32>(esp + 4, fx);
    let mut xmm = fr.xmm;
    xmm[0] = u128::from(c[0]);
    xmm[1] = 0;
    xmm[2] = u128::from(g.read::<u32>(w(0x6a)));
    xmm[3] = u128::from(c[3]);
    let task = Frame {
        at: img.rel32_target(f + 0x9d),
        esp,
        eax: last,
        ecx: fx.wrapping_add(0xc78),
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_fixed(g, img, &task);
    let t = slot_target(g, w(0xa2), 557);
    let v = g.call(t, 0, &[fx]).eax & !4;
    let t = slot_target(g, w(0xb3), 560);
    Ret::eax(g.call(t, 0, &[v, fx]).eax)
}

/// [`glow_scale_start`] (195 bytes, 1 overlay).
pub const GLOW_SCALE_START: Sig = Sig {
    len: 195,
    hash: 0x82b6_0f10_de2d_7c82,
    calls: &[(0x9d, Callee::Sig(&crate::s4_small::SCALE_TASK_20))],
    imports: &[],
};
