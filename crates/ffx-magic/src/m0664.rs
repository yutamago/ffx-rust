//! `magic_0664.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts a ring part (0x10001810, an overlay table entry, `(obj)`; two
/// table entries): [`start_pair`](crate::part_draw::start_pair) with
/// program 4; `+0x44` 0; the first part `+0x40` 78, `+0x48` 7, `+0x4c`
/// 0, any other 85, 8, 1; `+0x50..+0x58` 0, `+0x5c` 1, the colour
/// 0x3e21611c, `+0x6c` 1.0. Returns 1.
pub fn ring_part<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = 0x3e21_611c;
    crate::part_draw::start_pair(
        g,
        img,
        fr,
        (0x36, 0x10),
        &[(0x44, 0)],
        &[(0x40, 0x429c_0000), (0x48, 0x40e0_0000), (0x4c, 0)],
        &[
            (0x40, 0x42aa_0000),
            (0x48, 0x4100_0000),
            (0x4c, 0x3f80_0000),
        ],
        &[
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x60, c),
            (0x64, c),
            (0x68, c),
            (0x6c, 0x3f80_0000),
        ],
    )
}

/// [`ring_part`] (190 bytes, 1 overlay).
pub const RING_PART: Sig = Sig {
    len: 190,
    hash: 0x09c2_7ac2_ea1e_c7b0,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's frame (0x100012f0, an overlay table entry, `(obj)`): slot
/// 237 `(G_A, 3, 0, obj + 0x40)` (`+0x1d`), `+0x44` -0.01; slot 258
/// `(obj)`, slot 231 with the globals at `+0x51`, `+0x45`, `+0x3a` and
/// slot 230 with `+0x64`, `+0x59` (the earlier words left on the stack);
/// `*G_P = ITEM` (`+0x71`, `+0x79`), slot 712 `(obj, 0x30)` with the game
/// flag `*G_LOCK` (`+0x7e`) 1 and restored after (`+0x9a`). When 712
/// says nonzero, `*G_P = ITEM` again, slot 713 `(obj)` removes it and the
/// part starts again with program 6
/// ([`start_part`](crate::part_draw::start_part)). Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    frame_with(g, img, fr, (3, 0x18))
}

/// [`frame`]'s shape with slot 237's mode `k` and the restart's program
/// `index / 4` (also `magic_0665` 0x10001970).
pub(crate) fn frame_with<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    (k, index): (u32, u32),
) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0xd), 237);
    let a = g.read::<u32>(w(0x1d));
    let mut stack = vec![a, k, 0, obj + 0x40];
    g.call(t, 0, &stack);
    g.write::<u32>(obj + 0x44, 0xbc23_d70a);
    let t = slot_target(g, w(0x24), 258);
    stack.insert(0, obj);
    // call width: 5 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for op in [0x3a, 0x45, 0x51] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x3f), 231);
    // call width: 8 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    for op in [0x59, 0x64] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x5e), 230);
    // call width: 10 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x71));
    g.write::<u32>(gp, w(0x79));
    let lock = g.read::<u32>(w(0x7e));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x8c), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 12 (the earlier calls' words stay on the stack)
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x9a));
    g.write::<u32>(lock, saved);
    if r == 0 {
        return Ret::eax(1);
    }
    let gp = g.read::<u32>(w(0xa8));
    g.write::<u32>(gp, w(0xaf));
    let t = slot_target(g, w(0xb4), 713);
    g.call(t, 0, &[obj]);
    crate::part_draw::start_part(
        g,
        img,
        f,
        obj,
        [0xc1, 0xc7, 0xcc, 0xd4, 0xef],
        index,
        &[obj],
    );
    Ret::eax(1)
}

/// [`frame`] (255 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 255,
    hash: 0x1a6a_5225_df7f_809a,
    calls: &[(0xef, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's frame (0x10002200, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else `*G_P =
/// ITEM` (`+0x0d`, `+0x19`), slots 723 `(*G_A)` and 724 `(*G_B)`, `*G_V =
/// *G_W` (`+0x3f`, `+0x44`); the timelines `A` (`+0x61`) and `B`
/// (`+0x81`) run while slot 261 says so for 0 and 2
/// ([`timeline_run`](crate::sb_misc::timeline_run)); when it says so
/// for 1 both stop ([`timeline_stop`](crate::sb_misc::timeline_stop))
/// and slot 214 `(*G_E)` (`+0xbb`) runs, else the frame count `+0xd2`
/// goes up. Returns 1.
pub fn effect_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let gp = g.read::<u32>(w(0xd));
    let a = g.read::<u32>(w(0x13));
    g.write::<u32>(gp, w(0x19));
    let t = slot_target(g, w(0x1e), 723);
    g.call(t, 0, &[a]);
    let b = g.read::<u32>(w(0x31));
    let t = slot_target(g, w(0x2b), 724);
    g.call(t, 0, &[b, a]);
    let v = g.read::<u32>(w(0x3f));
    let x = g.read::<u32>(w(0x44));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x4b), 261);
    if g.call(t, 0, &[0, b, a]).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x66), w(0x61));
    }
    let t = slot_target(g, w(0x6b), 261);
    if g.call(t, 0, &[2]).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x86), w(0x81));
    }
    let t = slot_target(g, w(0x8b), 261);
    if g.call(t, 0, &[1]).eax == 0 {
        let n = w(0xd2);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop(g, w(0xa1));
    crate::sb_misc::timeline_stop(g, w(0xab));
    let t = slot_target(g, w(0xb5), 214);
    let e = g.read::<u32>(w(0xbb));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// [`effect_frame`] (220 bytes, 1 overlay).
pub const EFFECT_FRAME: Sig = Sig {
    len: 220,
    hash: 0xe003_90e3_2768_ff5b,
    calls: &[
        (0x66, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x86, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xa6, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xb0, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};

/// An object's frame at a bone (0x10001c60, an overlay table entry,
/// `(obj)`): slot 237 `(*G_H, BONE, 0, obj + 0x40)` with bone 0xba for
/// the array's first object (within 0x80 bytes of operand `+0x0a`), 0xbe
/// for the others; `+0x44` -0.01; the object drawn (slot 258, slot 231
/// `(*G_A, *G_B, *G_C)`, slot 230 `(*G_D, *G_A)`, slot 712 `(obj, 0x30)`
/// under the game flag with `**G_P = ITEM`; the earlier words left on the
/// stack); when 712 says nonzero, slot 713 `(obj)` removes it and the
/// part starts again with program 3
/// ([`start_part`](crate::part_draw::start_part)). Returns 1.
pub fn frame_bone<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, op: u32| g.read::<u32>(w(op));
    let obj = fr.arg(g, 0);
    let bone = if obj.wrapping_sub(w(0xa)) & 0xffff_ff80 == 0 {
        0xba
    } else {
        0xbe
    };
    let t = slot_target(g, w(0x18), 237);
    let h = r(g, 0x34);
    g.call(t, 0, &[h, bone, 0, obj + 0x40]);
    let t = slot_target(g, w(0x3b), 258);
    g.write::<u32>(obj + 0x44, 0xbc23_d70a);
    let mut stack = vec![obj];
    g.call(t, 0, &stack);
    let a = r(g, 0x55);
    let t = slot_target(g, w(0x5a), 231);
    let b = r(g, 0x60);
    let c = r(g, 0x6c);
    stack.splice(0..0, [c, b, a]);
    // call width: 4
    g.call(t, 0, &stack);
    let a = r(g, 0x74);
    let t = slot_target(g, w(0x79), 230);
    let d = r(g, 0x7f);
    stack.splice(0..0, [d, a]);
    // call width: 6
    g.call(t, 0, &stack);
    let gp = r(g, 0x8c);
    g.write::<u32>(gp, w(0x94));
    let lock = r(g, 0x99);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0xa7), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 8
    let res = g.call(t, 0, &stack).eax;
    let lock = r(g, 0xb5);
    g.write::<u32>(lock, saved);
    if res != 0 {
        let gp = r(g, 0xc4);
        g.write::<u32>(gp, w(0xcb));
        let t = slot_target(g, w(0xd0), 713);
        g.call(t, 0, &[obj]);
        crate::part_draw::start_part(g, img, f, obj, [0xdd, 0xe3, 0xe8, 0xf0, 0x10b], 0xc, &[obj]);
    }
    Ret::eax(1)
}

/// [`frame_bone`] (282 bytes, 1 overlay).
pub const FRAME_BONE: Sig = Sig {
    len: 282,
    hash: 0xfacd_7641_12d5_9aef,
    calls: &[(0x10b, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The look-matrix draw's tail after the matrix `M` is made (operands at
/// `magic_0664` 0x10001e20's offsets plus `b`, those after slot 712
/// plus `b2`): slot 230 `(*G_A, M)`, slot 231 `(*G_C, *G_B, *G_A)`, slot
/// 230 `(*G_D, *G_C)`; `*G_P = ITEM`; slot 712 `(obj, 0x30)` with the game
/// flag `*G_LOCK` 1 around it; when that is nonzero, `*G_P = ITEM`, slot
/// 713 `(obj)`, `*G_P = ITEM` again and part 13 of the list slot 941
/// `((*G_Q)[0x10])` gives started on `obj`
/// ([`crate::opu_parts::start_part`], `0x1000`, `(entry, 1)`). Each call
/// sees the words the earlier ones left (the saved register under them
/// is left out: no callee reads that deep). Returns 1.
fn look_tail<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (obj, m): (u32, u32),
    (b, b2): (u32, u32),
) -> Ret {
    let w = |o: u32| img.word(f + b + o);
    let r = |g: &G, o: u32| g.read::<u32>(w(o));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[m, r(g, 0x3b)]);
    let t = slot_target(g, w(0x35), 230);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, 0);
    s.push_all(&[r(g, 0x49), r(g, 0x54), r(g, 0x60)]);
    let t = slot_target(g, w(0x4e), 231);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, 0);
    s.push_all(&[r(g, 0x68), r(g, 0x73)]);
    let t = slot_target(g, w(0x6d), 230);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, t, 0);
    let p = r(g, 0x80);
    s.push(0x30);
    g.write::<u32>(p, w(0x88));
    let lock = r(g, 0x8d);
    s.push(obj);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x9b), 712);
    // call width: 12 (Pushes caps the words at 12)
    let done = s.call(g, t, 0).eax;
    let lock = r(g, 0xa9);
    g.write::<u32>(lock, saved);
    if done == 0 {
        return Ret::eax(1);
    }
    let w2 = |o: u32| img.word(f + b2 + o);
    let p = g.read::<u32>(w2(0xb7));
    g.write::<u32>(p, w2(0xbe));
    let t = slot_target(g, w2(0xc3), 713);
    // call width: 1
    g.call(t, lock, &[obj]);
    let p = g.read::<u32>(w2(0xd0));
    g.write::<u32>(p, w2(0xd6));
    let q = g.read::<u32>(w2(0xdb));
    let d = g.read::<u32>(q.wrapping_add(0x10));
    let t = slot_target(g, w2(0xe3), 941);
    // call width: 2 (slot 713's word under it)
    let list = g.call(t, lock, &[d, obj]).eax;
    let entry = g.read::<u32>(list.wrapping_add(0x34));
    crate::opu_parts::start_part(
        g,
        img,
        img.rel32_target(f + b2 + 0xfe),
        obj,
        0x1000,
        entry,
        1,
    );
    Ret::eax(1)
}

/// The level look matrix (`magic_0664`'s
/// [`crate::s6_misc::level_look_matrix`], the `rel32` at `call`) into
/// `M` (`ebp - 0x44`) from the source `src`, the height `K` (operand
/// `+0x14`) and the scale `obj + 0x60`; `ebp` = `esp - 4` at the entry,
/// `pushed` words under the 0x44-byte frame.
fn look_matrix<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    call: u32,
    src: u32,
    pushed: u32,
) -> (u32, u32) {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let m = ebp.wrapping_sub(0x44);
    let height = g.read::<u32>(img.word(f + 0x14));
    let scale = g.read::<u32>(obj.wrapping_add(0x60));
    let callee = Frame {
        at: img.rel32_target(f + call),
        esp: m.wrapping_sub(4 * pushed + 4),
        ecx: m,
        edx: src,
        xmm: [0, 0, u128::from(height), u128::from(scale)],
        ..Frame::default()
    };
    crate::s6_misc::level_look_matrix(g, img, &callee);
    (obj, m)
}

/// An object drawn on the level look matrix (0x10001e20, an overlay
/// table entry, `(obj)`; a /GS frame of 0x44 bytes): the matrix from
/// `*G_S` (`+0x1a`), then [`look_tail`]. Returns 1.
pub fn look_draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let src = g.read::<u32>(img.word(fr.at + 0x1a));
    // esi and edi pushed under the frame
    let (obj, m) = look_matrix(g, img, fr, 0x2c, src, 2);
    look_tail(g, img, fr.at, (obj, m), (0, 0))
}

/// [`look_draw`] (282 bytes, 1 overlay).
pub const LOOK_DRAW: Sig = Sig {
    len: 282,
    hash: 0x3672_a1d3_072a_6bdb,
    calls: &[
        (0x2c, Callee::Sig(&crate::s6_misc::LEVEL_LOOK_MATRIX)),
        (0xfe, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x112, Callee::Any),
    ],
    imports: &[],
};

/// An object drawn on the level look matrix, fading out (0x100014e0, an
/// overlay table entry, `(obj)`; a /GS frame of 0x44 bytes): the object
/// within the 0x80 bytes from `BASE` (`+0x24`) takes the matrix from
/// `*G_S` (`+0x34`) and, once the frame counter `*G_N` (`+0x3f`) is past
/// 0xb90, its halfword `+0x76` drops by 409; below 1 the call ends there
/// (returns 0). Any other object takes the matrix from `*G_T` (`+0x6c`).
/// Then [`look_tail`] (the operands 0x46 and 0x47 bytes on). Returns 1.
pub fn look_draw_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let first = obj.wrapping_sub(img.word(f + 0x24)) & 0xffff_ff80 == 0;
    // edi pushed under the frame
    let (src, call) = if first { (0x34, 0x39) } else { (0x6c, 0x71) };
    let src = g.read::<u32>(img.word(f + src));
    let (obj, m) = look_matrix(g, img, fr, call, src, 1);
    if first && g.read::<i32>(img.word(f + 0x3f)) > 0xb90 {
        let v = g.read::<u16>(obj.wrapping_add(0x76)).wrapping_add(0xfe67);
        g.write::<u16>(obj.wrapping_add(0x76), v);
        if v.cast_signed() < 1 {
            return Ret::eax(0);
        }
    }
    look_tail(g, img, f, (obj, m), (0x46, 0x47))
}

/// [`look_draw_fade`] (352 bytes, 1 overlay).
pub const LOOK_DRAW_FADE: Sig = Sig {
    len: 352,
    hash: 0xace1_b6a7_4b11_12e5,
    calls: &[
        (0x39, Callee::Sig(&crate::s6_misc::LEVEL_LOOK_MATRIX)),
        // __security_check_cookie
        (0x62, Callee::Any),
        (0x71, Callee::Sig(&crate::s6_misc::LEVEL_LOOK_MATRIX)),
        (0x145, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x158, Callee::Any),
    ],
    imports: &[],
};
