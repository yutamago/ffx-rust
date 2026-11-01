//! `magic_0610.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Sig};

/// Checks the targets (0x1000a610, internal, no arguments): slots 4
/// `(0x3018)` and 27 `(0x3018, 1)` (the first call's word under the
/// second's); `*G_P` (operand `+0x2c`) the immediate `+0x30`. A target
/// (of `*G_N`, `+0x3b`, signed; the list `*G_L` `+0x54`, the objects at
/// `+0x5f`; target -1 is `G_ME` `+0x48` with `+0x4d`) whose object's kind
/// halfword is one of the seven words at `+0x6a`, or which slot 580
/// `(id)` rejects (non-zero), stops the scan with the result 1. Then
/// `*G_OK` (`+0xa9`) = no such target and slot 523 `(0, result)`.
/// Returns slot 523's `eax`.
pub fn check_targets<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let t = slot_target(g, w(0x1), 4);
    g.call(t, 0, &[0x3018]);
    let t = slot_target(g, w(0x13), 27);
    g.call(t, 0, &[0x3018, 1, 0x3018]);
    g.write::<u32>(w(0x2c), w(0x30));
    let mut found = 0u32;
    let mut i = 0u32;
    'scan: while i.cast_signed() < g.read::<i32>(w(if i == 0 { 0x3b } else { 0x93 })) {
        let (id, o) = if i == u32::MAX {
            (g.read::<u32>(w(0x48)), g.read::<u32>(w(0x4d)))
        } else {
            let list = g.read::<u32>(w(0x54));
            let id = u32::from(g.read::<u8>(list.wrapping_add(i)));
            (id, g.read::<u32>(w(0x5f).wrapping_add(4 * id)))
        };
        let kind = u32::from(g.read::<u16>(o));
        for k in 0..7 {
            if kind == g.read::<u32>(w(0x6a) + 4 * k) {
                found = 1;
                break 'scan;
            }
        }
        let t = slot_target(g, w(0x79), 580);
        if g.call(t, 0, &[id]).eax != 0 {
            found = 1;
            break;
        }
        i = i.wrapping_add(1);
    }
    g.write::<u32>(w(0xa9), 1 - found);
    let t = slot_target(g, w(0xa3), 523);
    g.call(t, 0, &[0, found]).eax
}

/// [`check_targets`] (191 bytes, 1 overlay).
pub const CHECK_TARGETS: Sig = Sig {
    len: 191,
    hash: 0xcec2_9e39_c7bb_6202,
    calls: &[],
    imports: &[],
};

/// Spawns a spark every second call (0x1000bb00, cdecl `(rec)`): the
/// record's count `+0x2c` goes up; on an odd count nothing more (the
/// record returned). Else a pool record
/// ([`pool_take_140`](crate::sb_misc::pool_take_140); none: 0) starts
/// as an object ([`start_object_plain`](crate::s4_misc::start_object_plain):
/// index 3, the scale `(slot 404 (1.0) + K1) * K2` (`+0x4d`, `+0x5c`;
/// the ST0 stored as `f32` into the argument word, as the original's
/// `fstp [ebp + 8]`)); slot 268 `(A[b[0]], 12, P)` (`b = *G_B` `+0x66`,
/// `A` the table `+0x79`) places its point `P = +0xd0` (`w` 1.0); its
/// `+0xc4` = `*G_FX + 0x1a68` (`+0x16`) and its `y` slot 12 `(*G_M)`'s
/// ST0 (`+0xa4`). Returns
/// slot 12's `eax`.
pub fn spark<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.esp.wrapping_add(4);
    let rec = g.read::<u32>(arg);
    let n = g.read::<u32>(rec + 0x2c).wrapping_add(1);
    g.write::<u32>(rec + 0x2c, n);
    if n & 1 != 0 {
        return crate::Ret::eax(rec);
    }
    let fx = g.read::<u32>(w(0x16));
    let obj = crate::sb_misc::pool_take_140(g, img, img.rel32_target(f + 0x1c));
    if obj == 0 {
        return crate::Ret::eax(0);
    }
    let t = slot_target(g, w(0x2e), 404);
    let x = crate::ob_model::st0_f32(g.call(t, t, &[0x3f80_0000]).st0);
    g.write::<f32>(arg, x);
    let scale = crate::sb_sse::mul(
        crate::sb_sse::add(x, g.read::<f32>(w(0x4d))),
        g.read::<f32>(w(0x5c)),
    );
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x61), obj, scale, 3);
    let p = obj.wrapping_add(0xd0);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x66))));
    let a = g.read::<u32>(w(0x79).wrapping_add(4 * b));
    let t = slot_target(g, w(0x7e), 268);
    // the words under these two calls (the 1.0, two reserved words, the
    // index) are left out: the slots read only their own
    g.call(t, 0, &[a, 0xc, p]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let v = g.read::<u32>(fx + 0x1a68);
    g.write::<u32>(obj + 0xc4, v);
    let t = slot_target(g, w(0x9e), 12);
    let m = g.read::<u32>(w(0xa4));
    let o = g.call(t, 0, &[m, a, 0xc, p]);
    g.write::<f32>(obj + 0xd4, crate::ob_model::st0_f32(o.st0));
    crate::Ret::eax(o.eax)
}

/// [`spark`] (187 bytes, 1 overlay).
pub const SPARK: Sig = Sig {
    len: 187,
    hash: 0x58e4_21d7_af7b_082e,
    calls: &[
        (0x1c, crate::Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (
            0x61,
            crate::Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN),
        ),
    ],
    imports: &[],
};

/// Spawns a placed object for side `k` (0x1000ba00, cdecl `(-, k)`):
/// for `k` above 1 nothing (the caller's `eax` returned). Else a pool
/// record ([`pool_take_140`](crate::sb_misc::pool_take_140); none: 0)
/// starts as an object ([`start_object_plain`](crate::s4_misc::start_object_plain):
/// index 4, the scale `(slot 404 (1.0) + K1) * K2` (`+0x4c`, `+0x5b`;
/// the ST0 stored as `f32` into the `k` word, as the original's `fstp
/// [ebp + 0xc]`)); slot 237 `(*G_A, T[k], 0, P)` (`+0x81`, the table
/// `+0x7b`) fills its point `P = +0xd0` (`w` 1.0); its `+0xc4` =
/// `*G_FX + 0x1a68` (`+0x12`, read first) and its `y` slot 12 `(*G_A)`'s
/// ST0 (`+0xa4`). Returns slot 12's `eax`.
pub fn side_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let karg = fr.esp.wrapping_add(8);
    let k = g.read::<u32>(karg);
    if k.cast_signed() > 1 {
        return crate::Ret::eax(fr.eax);
    }
    let fx = g.read::<u32>(w(0x12));
    let obj = crate::sb_misc::pool_take_140(g, img, img.rel32_target(f + 0x1b));
    if obj == 0 {
        return crate::Ret::eax(0);
    }
    let t = slot_target(g, w(0x2d), 404);
    let x = crate::ob_model::st0_f32(g.call(t, t, &[0x3f80_0000]).st0);
    g.write::<f32>(karg, x);
    let scale = crate::sb_sse::mul(
        crate::sb_sse::add(x, g.read::<f32>(w(0x4c))),
        g.read::<f32>(w(0x5b)),
    );
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x60), obj, scale, 4);
    let p = obj.wrapping_add(0xd0);
    let a = g.read::<u32>(w(0x81));
    let tk = g.read::<u32>(w(0x7b).wrapping_add(k.wrapping_mul(4)));
    let t = slot_target(g, w(0x65), 237);
    // the words under these two calls (the 1.0, two reserved words, the
    // index) are left out: the slots read only their own
    g.call(t, 0, &[a, tk, 0, p]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let v = g.read::<u32>(fx + 0x1a68);
    g.write::<u32>(obj + 0xc4, v);
    let t = slot_target(g, w(0x9e), 12);
    let m = g.read::<u32>(w(0xa4));
    let o = g.call(t, 0, &[m, a, tk, 0, p]);
    g.write::<f32>(obj + 0xd4, crate::ob_model::st0_f32(o.st0));
    crate::Ret::eax(o.eax)
}

/// [`side_object`] (189 bytes, 1 overlay).
pub const SIDE_OBJECT: Sig = Sig {
    len: 189,
    hash: 0xb641_cfec_2654_afce,
    calls: &[
        (0x1b, crate::Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (
            0x60,
            crate::Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN),
        ),
    ],
    imports: &[],
};
