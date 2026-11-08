//! Round 17's ports (magic-r17): shared rows and the own rows of overlays
//! below `magic_0450` with only a few functions left, written out for
//! their DLLs.

use crate::m0018::st0_f32;
use crate::r11_fades::{stores, zeros};
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `K - r` wrapped once (`comiss` + `jbe`: an unordered value stays):
/// above `HI` less `T`, below `LO` plus `T'` (the floats at the operands
/// `k`, `hi`, `t`, `lo`, `t2` of `f`).
fn wrap_once<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    y: f32,
    (hi, t, lo, t2): (u32, u32, u32, u32),
) -> f32 {
    let c = |o: u32| g.read::<f32>(img.word(f + o));
    if y > c(hi) {
        sub(y, c(t))
    } else if c(lo) > y {
        add(y, c(t2))
    } else {
        y
    }
}

/// `f32` of slot 314 (`op_random`) of `x` (one word, ST0 stored as
/// `fstp dword`).
fn random<G: Game>(g: &mut G, table: u32, x: f32) -> f32 {
    st0_f32(
        g.call(slot_target(g, table, 0x4e8 / 4), 0, &[x.to_bits()])
            .st0,
    )
}

/// The debris burst task of `magic_0018` (0x10001340) and `magic_0019`
/// (0x10003850), 1921 bytes (stored by their start-ups as the task step;
/// one argument, the task `T`; an 8-aligned /GS frame, `A` its 8-aligned
/// base). `T + 0x14` counts down; while it stays above 0 the step returns
/// `T`. At 0: the count `n` = slot 592 `(*G_N)` (1 below 0, at most
/// 9,999); slot 237 `(*G_H, PARTS[*G_N], 0, P)` gives the source `P`
/// (`A - 0x98`, `w` 1.0), the first actor's bone 16 (slot 262 then 268
/// into `X`, `A - 0x88`, `w` 1.0) the aim; the pitch `atan2(dy,
/// hypot(dx, dz))` and the yaw `K - atan2(dz, dx)` (wrapped once) of `X -
/// P` (`_CIatan2`). For `n` of 1,000, 3,000 or 6,000 and more, 1, 2 or 3
/// sparks (`m`): `V = (X - P) / (m + K1)` (slots 234, 564), and spark `j`
/// is the object at `R_j = REC + 0x130 j`
/// ([`start_object_at`](crate::s4_misc::start_object_at) `(8, d (j +
/// 1), -, 0)`, `d = trunc(|P - X| / K2) / (m + 1)`) at `P + (j + 1) V`
/// (slot 248) with the pitch, the yaw, a random turn (slot 314 `(PI)`)
/// and `obj + 0x76` 0x800. Then the radius (slot 251 of the first actor's
/// byte below 32 when `*G_M` is 0, else slot 17 x slot 19 of the actor,
/// times `K3`) and the actor's ground (slot 12); while `n` is above 0 and
/// fewer than 24 pieces are out: the first `k` (kept between pieces, at
/// most 7) with `COST[k] <= n` gives the piece `R_i = *G_R + 0x4f0 i`
/// (`(KIND[k], 0, -, 0)`) from `P` to `X` plus a random offset turned by
/// the yaw (slots 235, 328, 329, 229, 248; `y` at most the ground), its
/// callbacks (`+0x378` `G_C`, `+0x37c`, `+0x380` the code pointers), `-1`,
/// `i`, 2, the first piece also clearing the shared record `G_S` (its
/// address in `+0x110`), the aim ([`aimed_emitter_angles`] with the speed
/// `K2` plus a random one after the first), three random spins and
/// `SIZE[k]`, `n -= COST[k]`. Last, the task's owner (`T + 8`) gets the
/// next step `+0x10`; returns the owner.
///
/// [`aimed_emitter_angles`]: crate::s7_misc::aimed_emitter_angles
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn debris_burst_0018<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::s4_misc::start_object_at;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let left = g.read::<i32>(task + 0x14).wrapping_sub(1);
    g.write::<i32>(task + 0x14, left);
    if left > 0 {
        return Ret::eax(task);
    }
    // push ebp; and esp, -8
    let a = fr.esp.wrapping_sub(4) & !7;
    let at = |o: u32| a.wrapping_sub(o);
    let (p, x, v, q, m, o) = (at(0x98), at(0x88), at(0x68), at(0x78), at(0x48), at(0x58));
    let tab = w(0x32);
    let n0 = g
        .call(slot_target(g, tab, 0x940 / 4), 0, &[g.read::<u32>(w(0x38))])
        .eax
        .cast_signed();
    let n = if n0 < 0 { 1 } else { n0.min(9999) };
    let idx = g.read::<u32>(w(0x64));
    let part = g.read::<u32>(w(0x6d).wrapping_add(4 * idx));
    let h = g.read::<u32>(w(0x78));
    // call width: 4
    g.call(slot_target(g, tab, 0x3b4 / 4), 0, &[h, part, 0, p]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let sel = g.read::<u32>(w(0x89));
    let actor = g.read::<u32>(w(0x9b).wrapping_add(4 * u32::from(g.read::<u8>(sel))));
    // call width: 4
    g.call(slot_target(g, tab, 0x418 / 4), 0, &[actor, x, x + 4, x + 8]);
    // call width: 3
    g.call(slot_target(g, tab, 0x430 / 4), 0, &[actor, 0x10, x]);
    let c = |g: &G, b: u32, i: u32| g.read::<f32>(b + 4 * i);
    let dy = sub(c(g, x, 1), c(g, p, 1));
    let dx = sub(c(g, x, 0), c(g, p, 0));
    let dz = sub(c(g, x, 2), c(g, p, 2));
    g.write::<u32>(x + 0xc, 0x3f80_0000);
    let s = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise` (exact), then `_CIatan2` twice on x87
    let pitch = g.atan2(f64::from(dy), f64::from(s).sqrt());
    #[allow(clippy::cast_possible_truncation)] // fstp qword; cvtpd2ps
    let yaw0 = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let yaw = wrap_once(
        g,
        img,
        f,
        sub(k(g, 0x183), yaw0),
        (0x192, 0x1a2, 0x1ac, 0x1b9),
    );
    let d = [0u32, 1, 2].map(|i| sub(c(g, p, i), c(g, x, i)));
    let len2 = add(add(mul(d[0], d[0]), mul(d[1], d[1])), mul(d[2], d[2]));
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
    let len = f64::from(len2).sqrt() as f32;
    let dist = crate::s4_summon::trunc(div(len, k(g, 0x202))).cast_signed();
    let sparks = match n {
        6000.. => 3,
        3000.. => 2,
        1000.. => 1,
        _ => 0,
    };
    if sparks > 0 {
        // call width: 3
        g.call(slot_target(g, tab, 0x3a8 / 4), 0, &[v, x, p]);
        #[allow(clippy::cast_precision_loss)] // 1..3: exact
        let by = add(sparks as f32, k(g, 0x267));
        // call width: 3
        g.call(slot_target(g, tab, 0x8d0 / 4), 0, &[v, v, by.to_bits()]);
        let step = dist.wrapping_div(sparks + 1);
        for i in 0..4 {
            g.write::<f32>(q + 4 * i, c(g, p, i));
        }
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let pitch32 = pitch as f32;
        let mut owner = step;
        let mut rec = w(0x2cf);
        for _ in 0..sparks {
            let r = rec.wrapping_sub(0xd4);
            let obj = start_object_at(
                g,
                img,
                call(0x2ea),
                r,
                k(g, 0x2d9),
                8,
                owner.cast_unsigned(),
                0,
            );
            // call width: 3
            g.call(slot_target(g, tab, 0x3e0 / 4), 0, &[q, q, v]);
            for i in 0..3 {
                g.write::<f32>(r + 0xd0 + 4 * i, c(g, q, i));
            }
            g.write::<u32>(r + 0xdc, 0x3f80_0000);
            g.write::<f32>(r + 0xc0, pitch32);
            g.write::<f32>(r + 0xc4, yaw);
            let turn = random(g, tab, std::f32::consts::PI);
            g.write::<f32>(r + 0xc8, turn);
            g.write::<u16>(obj + 0x76, 0x800);
            owner = owner.wrapping_add(step);
            rec = rec.wrapping_add(0x130);
        }
    }
    let mut count = n;
    let first = g.read::<u32>(w(0x392));
    let sel = g.read::<u32>(w(0x398));
    let radius = if g.read::<u32>(w(0x3a4)) == 0 && g.read::<u8>(sel) < 0x20 {
        let b = u32::from(g.read::<u8>(sel));
        st0_f32(g.call(slot_target(g, tab, 0x3ec / 4), 0, &[b]).st0)
    } else {
        let b = u32::from(g.read::<u8>(sel));
        let actor = g.read::<u32>(w(0x3d5).wrapping_add(4 * b));
        let thick = st0_f32(g.call(slot_target(g, tab, 0x4c / 4), 0, &[actor]).st0);
        let sz = st0_f32(g.call(slot_target(g, tab, 0x44 / 4), 0, &[actor]).st0);
        mul(sz, thick)
    };
    let radius = mul(radius, k(g, 0x411));
    let sel = g.read::<u32>(w(0x409));
    let actor = g.read::<u32>(w(0x421).wrapping_add(4 * u32::from(g.read::<u8>(sel))));
    let ground = st0_f32(g.call(slot_target(g, tab, 0x30 / 4), 0, &[actor]).st0);
    let cost = |g: &G, j: u32| g.read::<i32>(w(0x452).wrapping_add(4 * j));
    let mut j = 0u32;
    let mut i = 0u32;
    while count > 0 {
        while cost(g, j) > count {
            j += 1;
            if j >= 7 {
                break;
            }
        }
        if j >= 7 || i >= 24 {
            break;
        }
        let r = first.wrapping_add(0x4f0 * i);
        let kind = g.read::<u32>(w(0x481).wrapping_add(4 * j));
        start_object_at(g, img, call(0x48c), r, k(g, 0x476), kind, 0, 0);
        let t = r + 0x358;
        for e in 0..4 {
            g.write::<f32>(r + 0xd0 + 4 * e, c(g, p, e));
            g.write::<f32>(t + 4 * e, c(g, x, e));
        }
        let vx = random(g, tab, 5.0);
        g.write::<f32>(v, vx);
        let vy = random(g, tab, 5.0);
        g.write::<f32>(v + 4, vy);
        let vz = add(random(g, tab, 3.0), radius);
        g.write::<f32>(v + 8, vz);
        g.call(slot_target(g, tab, 0x3ac / 4), 0, &[m]);
        // call width: 3
        g.call(slot_target(g, tab, 0x520 / 4), 0, &[m, m, 0]);
        // call width: 3
        g.call(slot_target(g, tab, 0x524 / 4), 0, &[m, m, yaw.to_bits()]);
        g.write::<u32>(v + 0xc, 0x3f80_0000);
        // call width: 3
        g.call(slot_target(g, tab, 0x394 / 4), 0, &[o, m, v]);
        // call width: 3
        g.call(slot_target(g, tab, 0x3e0 / 4), 0, &[t, t, o]);
        g.write::<u32>(t + 0xc, 0x3f80_0000);
        if g.read::<f32>(t + 4) > ground {
            g.write::<f32>(t + 4, ground);
        }
        g.write::<u32>(r + 0x378, w(0x626));
        g.write::<u32>(r + 0x37c, w(0x630));
        g.write::<u32>(r + 0x380, w(0x63a));
        g.write::<u32>(r + 0x38c, 0xffff_ffff);
        g.write::<u32>(r + 0x390, i);
        g.write::<u16>(r + 0x394, 2);
        let speed = if i == 0 {
            zeros(g, img, f, &[0x663, 0x669, 0x66f, 0x675]);
            stores(g, img, f, &[0x67b, 0x685]);
            zeros(g, img, f, &[0x68f, 0x695]);
            g.write::<u16>(w(0x69d), 0);
            g.write::<u32>(r + 0x110, w(0x6a4));
            k(g, 0x65d)
        } else {
            add(random(g, tab, 1.0), k(g, 0x6cd))
        };
        // add esp, 4 (or none pushed): the call's return address at A - 0xd4
        let aim = Frame {
            at: call(0x6db),
            esp: at(0xd4),
            ecx: r,
            xmm: [0, u128::from(speed.to_bits()), 0, 0],
            ..*fr
        };
        crate::s7_misc::aimed_emitter_angles(g, img, &aim);
        for e in 0..3 {
            let spin = random(g, tab, f32::from_bits(0x3ea0_d97c));
            g.write::<f32>(r + 0x4d0 + 4 * e, spin);
        }
        let size = g.read::<u32>(w(0x730).wrapping_add(4 * j));
        g.write::<u32>(r + 0x4e0, size);
        count = count.wrapping_sub(cost(g, j));
        i += 1;
    }
    let owner = g.read::<u32>(task + 8);
    g.write::<u32>(owner + 0x10, w(0x768));
    Ret::eax(owner)
}

/// [`debris_burst_0018`] (1921 bytes, 2 overlays).
pub const DEBRIS_BURST_0018: Sig = Sig {
    len: 1921,
    hash: 0x8dc5_a301_1db9_82fb,
    calls: &[
        (0x12f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x142, Callee::Import("_CIatan2")),
        (0x171, Callee::Import("_CIatan2")),
        (0x1f6, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x2ea, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x48c, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x6db, Callee::Sig(&crate::s7_misc::AIMED_EMITTER_ANGLES)),
        // __security_check_cookie
        (0x779, Callee::Any),
    ],
    imports: &[],
};

/// The spark a debris piece leaves (`magic_0018` 0x10001bc0, 769 bytes,
/// stored in the piece's `+0x380`; one argument, the piece `P`; a /GS
/// frame, `M` its matrix at `ebp - 0x54`, `V` its point at `ebp - 0x14`):
/// with `i` = `P + 0x390` and `R = *G_R + 0x4f0 i`, the object at `R +
/// 0x3a0` ([`start_object_at`](crate::s4_misc::start_object_at) `(7, 0,
/// -, CB or 0)`: the code pointer `+0x30` for the first piece, scale `R +
/// 0x4e0`) copies `P`'s target (slot 325 `(obj + 0xd0, P + 0x358)`) and
/// takes `P`'s yaw plus `K` (wrapped once). The first piece clears the
/// shared record `G_S` (its address in `obj + 0x110`). Then the record
/// `B = *G_B + 0x44 i`: three random words (slot 314 `(0.8)`), 0, `V` =
/// `(0, 0, r + K2, 1)` (slot 404 `(1.5)`) turned by a random yaw (slots
/// 235, 314 `(PI)`, 329, 229) gives `B + 0x10` and `+0x18`, `B + 0x14` the
/// negated slot 404 `(1.8)`, the piece, constants, the first actor's
/// ground (slot 12) less `K3`, and slot 404 `(1.0)`. Returns the last
/// slot 404 call's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn piece_spark_0018<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let (piece, i) = piece_object(g, img, fr, (0x27, 0x30, 0x52, 0x65));
    let tab = w(0x65);
    let obj = g.read::<u32>(w(0x27)).wrapping_add(0x4f0 * i) + 0x3a0;
    let y = add(g.read::<f32>(piece + 0xc4), k(g, 0x7d));
    let y = wrap_once(g, img, f, y, (0x87, 0x91, 0x9b, 0xa8));
    g.write::<f32>(obj + 0xc4, y);
    if g.read::<u32>(piece + 0x390) == 0 {
        zeros(g, img, f, &[0xbf, 0xc9, 0xd3, 0xdd]);
        stores(g, img, f, &[0xe7, 0xf1]);
        zeros(g, img, f, &[0xfb, 0x105]);
        g.write::<u16>(w(0x111), 0);
        g.write::<u32>(obj + 0x110, w(0x11b));
    }
    let i = g.read::<u32>(piece + 0x390);
    let rec = g.read::<u32>(w(0x127)).wrapping_add(0x44 * i);
    for e in 0..3 {
        let x = random(g, tab, f32::from_bits(0x3f4c_cccd));
        g.write::<f32>(rec + 4 * e, x);
    }
    g.write::<u32>(rec + 0xc, 0);
    g.call(slot_target(g, tab, 0x3ac / 4), 0, &[m]);
    let turn = random(g, tab, std::f32::consts::PI);
    // call width: 3
    g.call(slot_target(g, tab, 0x524 / 4), 0, &[m, m, turn.to_bits()]);
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    let r = st0_f32(
        g.call(slot_target(g, tab, 0x650 / 4), 0, &[0x3fc0_0000])
            .st0,
    );
    let z = add(r, k(g, 0x1fa));
    g.write::<f32>(v + 8, z);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    // call width: 3
    g.call(slot_target(g, tab, 0x394 / 4), 0, &[v, m, v]);
    let vx = g.read::<f32>(v);
    g.write::<f32>(rec + 0x10, vx);
    let r = st0_f32(
        g.call(slot_target(g, tab, 0x650 / 4), 0, &[0x3fe6_6666])
            .st0,
    );
    let neg = r.to_bits() ^ g.read::<u32>(w(0x250));
    g.write::<u32>(rec + 0x14, neg);
    let vz = g.read::<f32>(v + 8);
    g.write::<f32>(rec + 0x18, vz);
    g.write::<u32>(rec + 0x1c, 0);
    g.write::<u32>(rec + 0x20, piece);
    g.write::<u32>(rec + 0x24, 0);
    g.write::<u32>(rec + 0x28, 0x3e19_999a);
    let sel = g.read::<u32>(w(0x285));
    let actor = g.read::<u32>(w(0x28f).wrapping_add(4 * u32::from(g.read::<u8>(sel))));
    let ground = st0_f32(g.call(slot_target(g, tab, 0x30 / 4), 0, &[actor]).st0);
    let hy = sub(ground, k(g, 0x2a9));
    g.write::<u32>(rec + 0x30, 0x3f73_3333);
    g.write::<u32>(rec + 0x34, 0x3f19_999a);
    g.write::<f32>(rec + 0x2c, hy);
    let out = g.call(slot_target(g, tab, 0x650 / 4), 0, &[0x3f80_0000]);
    g.write::<f32>(rec + 0x38, st0_f32(out.st0));
    g.write::<u32>(rec + 0x40, 0);
    g.write::<u32>(rec + 0x3c, 0x14);
    Ret::eax(out.eax)
}

/// The start of [`piece_spark_0018`] and [`piece_spark_0019`]: the object
/// at `R + 0x3a0` started (operands: `*G_R`, the code pointer, the `rel32`
/// of the start, slot 325's table) and `P`'s target copied into its
/// `+0xd0`. Returns `(P, i)`.
fn piece_object<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    (recs, cb, start, tab): (u32, u32, u32, u32),
) -> (u32, u32) {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let piece = fr.arg(g, 0);
    let i = g.read::<u32>(piece + 0x390);
    let r = i.wrapping_mul(0x4f0).wrapping_add(g.read::<u32>(w(recs)));
    let owner_b = if i == 0 { w(cb) } else { 0 };
    let obj = r + 0x3a0;
    let scale = g.read::<f32>(r + 0x4e0);
    crate::s4_misc::start_object_at(
        g,
        img,
        img.rel32_target(f + start),
        obj,
        scale,
        7,
        0,
        owner_b,
    );
    g.call(
        slot_target(g, w(tab), 0x514 / 4),
        0,
        &[obj + 0xd0, piece + 0x358],
    );
    (piece, i)
}

/// [`piece_spark_0018`] (769 bytes, 1 overlay).
pub const PIECE_SPARK_0018: Sig = Sig {
    len: 769,
    hash: 0x2111_612f_576b_21e5,
    calls: &[
        (0x52, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        // __security_check_cookie
        (0x2f9, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0019`'s piece spark (0x100040d0, 737 bytes, stored in the
/// piece's `+0x380`; one argument, the piece `P`): [`piece_spark_0018`]'s
/// object, yaw and shared record; the record `B = *G_B + 0x34 i` gets
/// three random words (slot 314 `(0.4)`), 0, `V = (0, 0, r, 1)` (slot 404
/// `(0.5)`) turned by a random yaw: `B + 0x10` is `V.x`, then `P + 0x368`
/// times `K` plus `V.x`; `B + 0x14` the negated slot 404 `(0.5)` twice
/// (the second kept); `B + 0x18` `V.z`, then `P + 0x370` times `K` plus
/// `V.z`; the piece and constants. Returns the last slot 404 call's
/// `eax`.
pub fn piece_spark_0019<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let (piece, i) = piece_object(g, img, fr, (0x27, 0x30, 0x4f, 0x62));
    let tab = w(0x62);
    let obj = g.read::<u32>(w(0x27)).wrapping_add(0x4f0 * i) + 0x3a0;
    let y = add(g.read::<f32>(piece + 0xc4), k(g, 0x7a));
    let y = wrap_once(g, img, f, y, (0x84, 0x8e, 0x98, 0xa5));
    g.write::<f32>(obj + 0xc4, y);
    if g.read::<u32>(piece + 0x390) == 0 {
        zeros(g, img, f, &[0xbc, 0xc6, 0xd0, 0xda]);
        stores(g, img, f, &[0xe4, 0xee]);
        zeros(g, img, f, &[0xf8, 0x102]);
        g.write::<u16>(w(0x10e), 0);
        g.write::<u32>(obj + 0x110, w(0x118));
    }
    let i = g.read::<u32>(piece + 0x390);
    let rec = i.wrapping_mul(0x34).wrapping_add(g.read::<u32>(w(0x132)));
    for e in 0..3 {
        let x = random(g, tab, f32::from_bits(0x3ecc_cccd));
        g.write::<f32>(rec + 4 * e, x);
    }
    g.write::<u32>(rec + 0xc, 0);
    g.call(slot_target(g, tab, 0x3ac / 4), 0, &[m]);
    let turn = random(g, tab, std::f32::consts::PI);
    // call width: 3
    g.call(slot_target(g, tab, 0x524 / 4), 0, &[m, m, turn.to_bits()]);
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    let r = st0_f32(
        g.call(slot_target(g, tab, 0x650 / 4), 0, &[0x3f00_0000])
            .st0,
    );
    g.write::<f32>(v + 8, r);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    // call width: 3
    g.call(slot_target(g, tab, 0x394 / 4), 0, &[v, m, v]);
    g.write::<f32>(rec + 0x10, g.read::<f32>(v));
    let r = st0_f32(
        g.call(slot_target(g, tab, 0x650 / 4), 0, &[0x3f00_0000])
            .st0,
    );
    let mask = g.read::<u32>(w(0x232));
    g.write::<u32>(rec + 0x14, r.to_bits() ^ mask);
    g.write::<f32>(rec + 0x18, g.read::<f32>(v + 8));
    g.write::<u32>(rec + 0x1c, 0);
    let x = add(
        mul(g.read::<f32>(piece + 0x368), k(g, 0x25d)),
        g.read::<f32>(v),
    );
    g.write::<f32>(rec + 0x10, x);
    let out = g.call(slot_target(g, tab, 0x650 / 4), 0, &[0x3f00_0000]);
    let r = st0_f32(out.st0);
    g.write::<u32>(rec + 0x14, r.to_bits() ^ g.read::<u32>(w(0x288)));
    let z = add(
        mul(g.read::<f32>(piece + 0x370), k(g, 0x2a0)),
        g.read::<f32>(v + 8),
    );
    g.write::<u32>(rec + 0x20, piece);
    g.write::<u32>(rec + 0x1c, 0);
    g.write::<u32>(rec + 0x24, 0x3e4c_cccd);
    g.write::<f32>(rec + 0x18, z);
    g.write::<u32>(rec + 0x28, 0x3f6b_851f);
    g.write::<u32>(rec + 0x2c, 0x14);
    g.write::<u32>(rec + 0x30, 0x1e);
    Ret::eax(out.eax)
}

/// [`piece_spark_0019`] (737 bytes, 1 overlay).
pub const PIECE_SPARK_0019: Sig = Sig {
    len: 737,
    hash: 0x2156_39c9_6147_280b,
    calls: &[
        (0x4f, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        // __security_check_cookie
        (0x2d9, Callee::Any),
    ],
    imports: &[],
};

/// Who a block of [`mats_0168`] reads an actor from: `CHRS[*SEL]`
/// (operands of the selector pointer and the table) or a global.
#[derive(Clone, Copy)]
enum Who {
    Chr(u32, u32),
    Global(u32),
}

fn who<G: Game>(g: &G, img: &Image, f: u32, w: Who) -> u32 {
    match w {
        Who::Chr(sel, chrs) => {
            let b = u32::from(g.read::<u8>(g.read::<u32>(img.word(f + sel))));
            g.read::<u32>(img.word(f + chrs).wrapping_add(4 * b))
        }
        Who::Global(op) => g.read::<u32>(img.word(f + op)),
    }
}

/// One matrix of [`mats_0168`]: `M` (`!= 0`: slot 210 `(A, M)`; else
/// slots 217 and 255 `(i, L)` for the actor index `i` the caller gives)
/// set by slot 211 `(A, M)` then 235 `(M)`, the diagonal slot 277's ST0
/// times `K` (`k_op`) times slots 15, 16, 17 of `E` (`m16_first`: slot
/// 16's factor on the left), turned by slot 330 `(0)`, 328 `(0)`, 329
/// `(yaw)`.
#[allow(clippy::too_many_arguments)] // the block's actors and operands
fn mats_block<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    tab: u32,
    m: u32,
    (a, null, l): (u32, Option<u32>, u32),
    (scale_of, e_of, k_op, m16_first): (Who, Who, u32, bool),
    yaw: f32,
) {
    let s = |g: &G, n: u32| slot_target(g, tab, n);
    if m != 0 {
        g.call(s(g, 0x348 / 4), 0, &[a, m]);
    } else if let Some(i) = null {
        g.call(s(g, 0x364 / 4), 0, &[i, l]);
        g.call(s(g, 0x3fc / 4), 0, &[i, l]);
    }
    g.call(s(g, 0x34c / 4), 0, &[a, m]);
    g.call(s(g, 0x3ac / 4), 0, &[m]);
    let c = who(g, img, f, scale_of);
    let r = st0_f32(g.call(s(g, 0x454 / 4), 0, &[c]).st0);
    let d = mul(r, g.read::<f32>(img.word(f + k_op)));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + o, d);
    }
    let e = who(g, img, f, e_of);
    let r = st0_f32(g.call(s(g, 0x3c / 4), 0, &[e]).st0);
    let v = mul(r, g.read::<f32>(m));
    g.write::<f32>(m, v);
    let r = st0_f32(g.call(s(g, 0x40 / 4), 0, &[e]).st0);
    let x = g.read::<f32>(m + 0x14);
    g.write::<f32>(m + 0x14, if m16_first { mul(r, x) } else { mul(x, r) });
    let r = st0_f32(g.call(s(g, 0x44 / 4), 0, &[e]).st0);
    let v = mul(g.read::<f32>(m + 0x28), r);
    g.write::<f32>(m + 0x28, v);
    // call width: 3
    g.call(s(g, 0x528 / 4), 0, &[m, m, 0]);
    // call width: 3
    g.call(s(g, 0x520 / 4), 0, &[m, m, 0]);
    // call width: 3
    g.call(s(g, 0x524 / 4), 0, &[m, m, yaw.to_bits()]);
}

/// The two actors' matrices of `magic_0168` (0x10001620) and `magic_0599`
/// (0x10006010), 1547 bytes (internal, no arguments; a /GS frame, `esp`
/// at the return address; the point `P` at `ebp - 0x14`, the buffer `L`
/// at `ebp - 0x24`): with `E` = `*G_E`, the first actor `C = CHRS[*SEL]`
/// gives `P` (slot 262; into `E + 0xac0`, `w` 1.0) and the yaw `K - `
/// slot 333 `(C)` wrapped once; `E + 0xa80` `(0, yaw, 0, 0)` and `+0xa90`
/// `P`; the matrix `E + 0x980` of `C` ([`mats_block`]) and its point
/// `E + 0x9b0`; `E + 0xaa0` `(0, yaw, 0, 0)` and `+0xab0` `P`; the matrix
/// `E + 0x9c0` of the second actor `*G_B` and its point `+0x9f0`; slot
/// 238 `(*G_I, 1)`; slot 296 `(*G_I, 0)` (`*G_F` 0 and the index below
/// 32) or 257 `(*G_B, 1)`; the yaw into `E + 0x3150`, `E + 0xe70` `(0, 0,
/// 0, 0x80)`, the camera words of `*G_E` copied, two flags and words.
/// Returns the last word copied.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn mats_0168<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = esp.wrapping_sub(4);
    let (p, l) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let tab = w(0x33);
    let e = g.read::<u32>(w(0x1b));
    let c = who(g, img, f, Who::Chr(0x11, 0x2e));
    // call width: 4
    g.call(slot_target(g, tab, 0x418 / 4), 0, &[c, p, p + 4, p + 8]);
    for i in 0..3 {
        let v = g.read::<u32>(p + 4 * i);
        g.write::<u32>(e + 0xac0 + 4 * i, v);
    }
    g.write::<u32>(e + 0xacc, 0x3f80_0000);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let c = who(g, img, f, Who::Chr(0x71, 0x82));
    let r = st0_f32(g.call(slot_target(g, tab, 0x534 / 4), 0, &[c]).st0);
    let yaw = wrap_once(g, img, f, sub(k(g, 0x9a), r), (0xa9, 0xb8, 0xc2, 0xcf));
    let quad = |g: &mut G, at: u32| {
        g.write::<u32>(e + at, 0);
        g.write::<f32>(e + at + 4, yaw);
        g.write::<u32>(e + at + 8, 0);
        g.write::<u32>(e + at + 0xc, 0);
        for i in 0..4 {
            let v = g.read::<u32>(p + 4 * i);
            g.write::<u32>(e + at + 0x10 + 4 * i, v);
        }
    };
    quad(g, 0xa80);
    let m = e.wrapping_add(0x980);
    let c = who(g, img, f, Who::Chr(0x134, 0x145));
    let null = if m == 0 {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x134))));
        (g.read::<u32>(w(0x167)) == 0 && b < 0x20).then_some(b)
    } else {
        None
    };
    mats_block(
        g,
        img,
        f,
        tab,
        m,
        (c, null, l),
        (Who::Chr(0x1be, 0x1c8), Who::Chr(0x1f8, 0x202), 0x1e5, false),
        yaw,
    );
    for i in 0..4 {
        let v = g.read::<u32>(e + 0xa90 + 4 * i);
        g.write::<u32>(e + 0x9b0 + 4 * i, v);
    }
    quad(g, 0xaa0);
    let a2 = g.read::<u32>(w(0x33c));
    let m = e.wrapping_add(0x9c0);
    let null = if m == 0 {
        let i = g.read::<i32>(w(0x36c));
        (g.read::<u32>(w(0x363)) == 0 && i < 0x20).then_some(i.cast_unsigned())
    } else {
        None
    };
    mats_block(
        g,
        img,
        f,
        tab,
        m,
        (a2, null, l),
        (Who::Global(0x3c3), Who::Global(0x3f4), 0x3db, true),
        yaw,
    );
    for i in 0..4 {
        let v = g.read::<u32>(e + 0xab0 + 4 * i);
        g.write::<u32>(e + 0x9f0 + 4 * i, v);
    }
    let i = g.read::<u32>(w(0x4cf));
    g.call(slot_target(g, tab, 0x3b8 / 4), 0, &[i, 1]);
    let i = g.read::<i32>(w(0x4ea));
    if g.read::<u32>(w(0x4e1)) == 0 && i < 0x20 {
        g.call(slot_target(g, tab, 0x4a0 / 4), 0, &[i.cast_unsigned(), 0]);
    } else {
        let b = g.read::<u32>(w(0x50c));
        g.call(slot_target(g, tab, 0x404 / 4), 0, &[b, 1]);
    }
    let cam = g.read::<u32>(w(0x51a));
    g.write::<f32>(e + 0x3150, yaw);
    for o in [0xe70u32, 0xe74, 0xe78] {
        g.write::<u32>(e + o, 0);
    }
    g.write::<u32>(e + 0xe7c, 0x80);
    for i in 0..4 {
        let v = g.read::<u32>(cam + 0xe70 + 4 * i);
        g.write::<u32>(cam + 0xe80 + 4 * i, v);
    }
    let mut last = 0;
    for i in 0..4 {
        last = g.read::<u32>(e + 0xe70 + 4 * i);
        g.write::<u32>(cam + 0xe90 + 4 * i, last);
    }
    g.write::<u32>(cam + 0xea4, 1);
    g.write::<u32>(cam + 0xea0, 1);
    for o in [0x3148u32, 0x314c, 0x3160] {
        g.write::<u32>(e + o, 0);
    }
    stores(g, img, f, &[0x5ed, 0x5f7]);
    last
}

/// [`mats_0168`] (1547 bytes, 2 overlays).
pub const MATS_0168: Sig = Sig {
    len: 1547,
    hash: 0x9610_a8ac_7986_7638,
    calls: &[
        // __security_check_cookie
        (0x603, Callee::Any),
    ],
    imports: &[],
};

/// The start-up of `magic_0168` (0x10004840) and `magic_0599`
/// (0x100080d0), 612 bytes (internal, no arguments; the load step's
/// callee): slot 718 `(NAME)`, five words, slot 719 `(PART, 8, 0)` into
/// `G_P`; the part set-up ([`part_setup`](crate::sb_misc::part_setup)
/// `(G_S)`), the two matrices ([`mats_0168`]); slots 357 and 356 of `*G_B`
/// (`(-1, 0x3c00)`, `(-1, 0x2ee0)`, then `(-1, -1)` twice); the reset
/// ([`reset_and_scale`](crate::s4_small::reset_and_scale)); words and
/// flags, slot 271 `(4)`; two task nodes (kinds 1 and 2, the frame
/// `+0x12d` and the step `+0x169`, [`take`](crate::sa_pool::take)) last
/// on the task list. Then slot 520 (the game's version) picks a fix-up of
/// the DLL's floats: `0x2090000` and `0x2090046` lower three of them by
/// `K`, `0x2050000` one by `K2`, `0xdcXXXX` sets one to -0.8. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0168<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let tab = w(0x1);
    g.call(slot_target(g, tab, 0xb38 / 4), 0, &[w(0xe)]);
    stores(g, img, f, &[0x28, 0x32, 0x3c, 0x46, 0x50]);
    // call width: 3
    let part = g
        .call(slot_target(g, tab, 0xb3c / 4), 0, &[w(0x24), 8, 0])
        .eax;
    g.write::<u32>(w(0x60), part);
    // push esi, edi, NAME (left there), G_S: the calls' return address
    let esp = fr.esp.wrapping_sub(0x14);
    g.write::<u32>(esp + 4, w(0x65));
    crate::sb_misc::part_setup(g, img, call(0x6a), esp);
    mats_0168(g, img, call(0x6f), esp);
    for (op, s, a) in [
        (0x7a, 0x594, 0x3c00),
        (0x7a, 0x590, 0x2ee0),
        (0xaa, 0x594, 0xffff_ffff),
        (0xaa, 0x590, 0xffff_ffff),
    ] {
        let b = g.read::<u32>(w(op));
        // call width: 3
        g.call(slot_target(g, tab, s / 4), 0, &[b, 0xffff_ffff, a]);
    }
    let reset = Frame {
        at: call(0xce),
        esp: fr.esp.wrapping_sub(0x44),
        ..*fr
    };
    crate::s4_small::reset_and_scale(g, img, &reset);
    stores(g, img, f, &[0xd7, 0xe1, 0xfb, 0x105]);
    g.write::<u16>(w(0xee), 0);
    g.write::<u8>(w(0xf6), 1);
    g.write::<u16>(w(0x112), 0);
    g.write::<u8>(w(0x11a), 1);
    g.call(slot_target(g, tab, 0x43c / 4), 0, &[4]);
    let n1 = crate::sa_pool::take(g, img, call(0x132), w(0x12d));
    g.write::<u8>(n1.wrapping_add(0x18), 1);
    g.write::<u32>(n1.wrapping_add(4), 0);
    if g.read::<u32>(w(0x138)) == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x164), n1);
    } else {
        let tail = g.read::<u32>(w(0x14f));
        g.write::<u32>(n1, tail);
        g.write::<u32>(tail.wrapping_add(4), n1);
    }
    g.write::<u32>(n1.wrapping_add(8), n1);
    let n2 = crate::sa_pool::take(g, img, call(0x171), w(0x169));
    g.write::<u8>(n2.wrapping_add(0x18), 2);
    g.write::<u32>(n2.wrapping_add(4), 0);
    // (`edi`, the old head or the first node, is never 0 here)
    g.write::<u32>(n2, n1);
    g.write::<u32>(n1.wrapping_add(4), n2);
    g.write::<u32>(n2.wrapping_add(8), n2);
    g.write::<u32>(w(0x19a), n2);
    let version = |g: &mut G, o: u32| g.call(slot_target(g, w(o), 0x820 / 4), 0, &[]).eax;
    if version(g, 0x19f) == 0x0209_0000 || version(g, 0x1b5) == 0x0209_0046 {
        let d = k(g, 0x229);
        for o in [0x221u32, 0x23d, 0x251] {
            let v = sub(k(g, o), d);
            g.write::<f32>(w(o), v);
        }
    } else if version(g, 0x1c9) == 0x0205_0000 {
        let v = sub(k(g, 0x1e0), k(g, 0x1e8));
        g.write::<f32>(w(0x1f2), v);
    } else if version(g, 0x1f8) & 0xffff_0000 == 0x00dc_0000 {
        stores(g, img, f, &[0x210]);
    }
    Ret::eax(0)
}

/// [`start_up_0168`] (612 bytes, 2 overlays).
pub const START_UP_0168: Sig = Sig {
    len: 612,
    hash: 0x6692_f333_d701_c283,
    calls: &[
        (0x6a, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x6f, Callee::Sig(&MATS_0168)),
        (0xce, Callee::Sig(&crate::s4_small::RESET_AND_SCALE)),
        (0x132, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x171, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The trail of one of `magic_0168`'s four ribbons (0x10001ee0, 1161
/// bytes; `magic_0599` 0x100068d0; internal: `ecx` = the ribbon `R`,
/// `edx` = a matrix `M`, one stack word, the newest point `V`; an
/// 8-aligned /GS frame, `S` its locals' base: `C` `S+0x38`, `D` `S+0x48`,
/// `Q` `S+0x28`, the drawn matrix `N` `S+0x58`, slot 724's matrix
/// `S+0x98`, slot 723's buffer `S+0xd8`). Slots 723 and 724 fill their
/// buffers, the ribbon's animation steps
/// ([`anim_step`](crate::sb_common::anim_step)), `G_N` = 0 and `G_P` =
/// `N`'s address. `V` goes into the ring of 128 points (`R + 0xc` + 16
/// `h`, `h` = `R + 0x820`, its flag byte `R + 0x828 + h` set; `h` moves
/// on). Then from the newest point back, up to 127 segments, `R + 0x824`
/// pieces are drawn `R + 0x81c` apart: a new segment (from `C`, the
/// current point, to the one before it; slot 234 into `D`, its length
/// through `_libm_sse2_sqrt_precise`) whenever the distance walked `a`
/// passes the segment's length (stopping at a point never written, after
/// 127 segments, or skipping a piece on a segment of length 0). Each
/// piece: the colour bytes `G_C` mixed from `R + 0x80c` to `R + 0x810`
/// by `i / n`; `Q` = `C + (a / len) D` (slot 248; and `a` grows by the
/// step) or `C` itself past the end; `N` = the unit matrix (slot 235)
/// with `M`'s rows scaled by the width (`R + 0x814` to `R + 0x818` by
/// `(i + 1) / n`) and its last row slot 229 `(S+0x98, Q)`; when the alpha
/// of `G_C` is not 0 the part is drawn
/// ([`link_part`](crate::sa_misc::link_part) `(G_REC, *G_K << 16 | i)`,
/// `edx` the animation's entry). Returns 0x7f (the next head when `n` is
/// 0).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn trail_0168<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let tab = w(0x26);
    let r = fr.ecx;
    let m = fr.edx;
    let v = fr.arg(g, 0);
    // push ebp; and esp, -8; sub esp, 0x114; push ebx, esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x120);
    let (c, d, q, n) = (s + 0x38, s + 0x48, s + 0x28, s + 0x58);
    g.call(slot_target(g, tab, 0xb4c / 4), 0, &[s + 0xd8]);
    g.call(slot_target(g, w(0x44), 0xb50 / 4), 0, &[s + 0x98]);
    let entry = crate::sb_common::anim_step(g, img, img.rel32_target(f + 0x56), r);
    g.write::<u32>(w(0x5f), 0);
    g.write::<u32>(w(0x70), n);
    let h = g.read::<u32>(r + 0x820);
    for i in 0..4 {
        let x = g.read::<u32>(v + 4 * i);
        g.write::<u32>(r.wrapping_add(h.wrapping_mul(16)) + 0xc + 4 * i, x);
    }
    let h = g.read::<u32>(r + 0x820);
    g.write::<u8>(r.wrapping_add(h) + 0x828, 1);
    let h = g.read::<u32>(r + 0x820);
    let next = h.wrapping_add(1);
    let next = if next.cast_signed() >= 0x80 { 0 } else { next };
    g.write::<u32>(r + 0x820, next);
    let pieces = |g: &G| g.read::<i32>(r + 0x824);
    if pieces(g) <= 0 {
        return Ret::eax(h.wrapping_add(1));
    }
    let point = |i: u32| r.wrapping_add(i.wrapping_mul(16)) + 0xc;
    let (mut a, mut len, mut cur, mut left) = (0.0f32, 0.0f32, h, 0x80i32);
    let mut i = 0i32;
    while i < pieces(g) {
        let mut draw = true;
        // comiss + jb: a below the length, or unordered, draws
        if a >= len {
            let prev = cur.wrapping_sub(1);
            a = sub(a, len);
            let pi = if prev.cast_signed() < 0 { 0x7f } else { prev };
            if g.read::<u8>(r.wrapping_add(pi) + 0x828) == 0 {
                break;
            }
            for e in 0..4 {
                let x = g.read::<u32>(point(cur) + 4 * e);
                g.write::<u32>(c + 4 * e, x);
            }
            // call width: 3
            g.call(slot_target(g, w(0x160), 0x3a8 / 4), 0, &[d, point(pi), c]);
            let (dx, dy, dz) = (g.read::<f32>(d), g.read::<f32>(d + 4), g.read::<f32>(d + 8));
            let sq = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
            #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
            let l = f64::from(sq).sqrt() as f32;
            len = l;
            cur = if prev.cast_signed() < 0 { 0x7f } else { prev };
            left -= 1;
            if left <= 0 {
                break;
            }
            // ucomiss + lahf/jnp: exactly 0.0 skips the piece
            if len == 0.0 {
                draw = false;
            }
        }
        if draw {
            for (ch, out) in (0u32..).zip([0x20bu32, 0x233, 0x25b, 0x286]) {
                let n_now = pieces(g);
                let b0 = i32::from(g.read::<u8>(r + 0x80c + ch));
                let b1 = i32::from(g.read::<u8>(r + 0x810 + ch));
                let sum = b0
                    .wrapping_mul(n_now.wrapping_sub(i))
                    .wrapping_add(b1.wrapping_mul(i));
                // idiv: `n` is above `i` here (0 only if a callee changed it)
                let mix = sum.checked_div(n_now).unwrap_or(0);
                g.write::<u8>(w(out), mix.to_le_bytes()[0]);
            }
            if a >= len {
                for e in 0..4 {
                    let x = g.read::<u32>(c + 4 * e);
                    g.write::<u32>(q + 4 * e, x);
                }
            } else {
                let t = div(a, len);
                for e in 0..3 {
                    let x = mul(t, g.read::<f32>(d + 4 * e));
                    g.write::<f32>(q + 4 * e, x);
                }
                // call width: 3
                g.call(slot_target(g, w(0x2db), 0x3e0 / 4), 0, &[q, c, q]);
                a = add(g.read::<f32>(r + 0x81c), a);
            }
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps
            let u = div((i + 1) as f32, pieces(g) as f32);
            let width = add(
                mul(sub(g.read::<f32>(w(0x326)), u), g.read::<f32>(r + 0x814)),
                mul(g.read::<f32>(r + 0x818), u),
            );
            g.write::<u32>(q + 0xc, 0x3f80_0000);
            g.call(slot_target(g, w(0x34c), 0x3ac / 4), 0, &[n]);
            for (row, at) in [(0u32, 0u32), (0x10, 0x10), (0x20, 0x20)] {
                for e in 0..3 {
                    let x = mul(g.read::<f32>(m + row + 4 * e), width);
                    g.write::<f32>(n + at + 4 * e, x);
                }
            }
            // call width: 3
            g.call(
                slot_target(g, w(0x3c2), 0x394 / 4),
                0,
                &[n + 0x30, s + 0x98, q],
            );
            if g.read::<u8>(w(0x42d)) != 0 {
                let key = (g.read::<u32>(w(0x435)) << 16).wrapping_add(i.cast_unsigned());
                // push key, G_REC; the call's return address below them
                let esp = s.wrapping_sub(0xc);
                g.write::<u32>(esp + 4, w(0x444));
                g.write::<u32>(esp + 8, key);
                let link = Frame {
                    at: img.rel32_target(f + 0x44b),
                    esp,
                    ecx: r,
                    edx: entry,
                    ..*fr
                };
                crate::sa_misc::link_part(g, img, &link);
            }
        }
        i += 1;
    }
    Ret::eax(0x7f)
}

/// [`trail_0168`] (1161 bytes, 2 overlays).
pub const TRAIL_0168: Sig = Sig {
    len: 1161,
    hash: 0x42f7_0b1e_7504_67ee,
    calls: &[
        (0x56, Callee::Sig(&crate::sb_common::ANIM_STEP)),
        (0x19f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x44b, Callee::Sig(&crate::sa_misc::LINK_PART)),
        // __security_check_cookie
        (0x481, Callee::Any),
    ],
    imports: &[],
};

/// The ribbons' task step of `magic_0168` (0x10002610, 217 bytes;
/// `magic_0599` 0x10006d60; stored in a task node, its argument not
/// read; an 8-aligned /GS frame, `M` at `A - 0x48`, `V` at `A - 0x58`):
/// `M` the unit matrix (slot 235) with 0.3 on its diagonal; for each of
/// the four ribbons `R_i = *G_E + 0xea8 + 0x8a8 i`: `G_K = i`, slot 237
/// `(*G_B, PARTS[i], 0, V)` (`V.w` 1.0), the fade byte `R_i + 0x80f`
/// down by one to 0 unless `*G_E + 0x315c` is set, and its trail
/// ([`trail_0168`] `(R_i, M, V)`). Returns the last trail's `eax`.
pub fn ribbons_0168<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let a = fr.esp.wrapping_sub(4) & !7;
    let (m, v) = (a.wrapping_sub(0x48), a.wrapping_sub(0x58));
    let e = g.read::<u32>(w(0x17));
    g.call(slot_target(g, w(0x23), 0x3ac / 4), 0, &[m]);
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(m + o, 0x3e99_999a);
    }
    let mut ret = 0;
    for i in 0..4u32 {
        let rec = e.wrapping_add(0xea8 + 0x8a8 * i);
        let part = g.read::<u32>(w(0x61).wrapping_add(4 * i));
        let b = g.read::<u32>(w(0x6d));
        g.write::<u32>(w(0x73), i);
        // call width: 4
        g.call(slot_target(g, w(0x58), 0x3b4 / 4), 0, &[b, part, 0, v]);
        g.write::<u32>(v + 0xc, 0x3f80_0000);
        if g.read::<u32>(e.wrapping_add(0x315c)) == 0 {
            let fade = g.read::<u8>(rec + 0x80f);
            // cmp al, 1; jbe: 0 and 1 give 0
            g.write::<u8>(rec + 0x80f, fade.saturating_sub(1));
        }
        // push edi, esi, ebx (S = A - 0x68), V; the return address
        let esp = a.wrapping_sub(0x70);
        g.write::<u32>(esp + 4, v);
        let trail = Frame {
            at: img.rel32_target(f + 0xb4),
            esp,
            ecx: rec,
            edx: m,
            ..*fr
        };
        ret = trail_0168(g, img, &trail).eax;
    }
    Ret::eax(ret)
}

/// [`ribbons_0168`] (217 bytes, 2 overlays).
pub const RIBBONS_0168: Sig = Sig {
    len: 217,
    hash: 0x3999_905b_d6d2_ee9b,
    calls: &[
        (0xb4, Callee::Sig(&TRAIL_0168)),
        // __security_check_cookie
        (0xd1, Callee::Any),
    ],
    imports: &[],
};

/// The four ribbons `E + 0xea8 + 0x8a8 i` reset: the animation cursor
/// (`word`, running, frame `-1`, time 0x200), colours `0x80808080` to 0,
/// head 0, widths 1.0, the 128 point flags cleared, 150 pieces `step`
/// apart, the fade byte 0x18.
fn ribbons_reset<G: Game>(g: &mut G, e: u32, word: u32, step: u32) {
    for i in 0..4u32 {
        let rec = e.wrapping_add(0xea8 + 0x8a8 * i);
        g.write::<u16>(rec + 8, 0xffff);
        g.write::<u16>(rec + 0xa, 0x200);
        g.write::<u32>(rec, word);
        g.write::<u32>(rec + 4, 1);
        g.write::<u32>(rec + 0x80c, 0x8080_8080);
        g.write::<u32>(rec + 0x810, 0);
        g.write::<u32>(rec + 0x820, 0);
        g.write::<u32>(rec + 0x814, 0x3f80_0000);
        g.write::<u32>(rec + 0x818, 0x3f80_0000);
        for j in 0..0x20 {
            g.write::<u32>(rec + 0x828 + 4 * j, 0);
        }
        g.write::<u32>(rec + 0x824, 0x96);
        g.write::<u32>(rec + 0x81c, step);
        g.write::<u8>(rec + 0x80f, 0x18);
    }
}

/// The model `model` taken off the list `G_L`
/// ([`list_remove`](crate::l5_objhost::list_remove), its `rel32` at
/// `call`) and, when it was there, `**G_P = REC`, slot 713 `(model)` and
/// bit 15 of its `+0x124` cleared (operands `list`, `p`, `rec`, `tab`).
fn model_off<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    model: u32,
    (call, list, p, rec, tab): (u32, u32, u32, u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    let removed =
        crate::l5_objhost::list_remove(g, img, img.rel32_target(f + call), w(list), model);
    if removed != 0 {
        let at = g.read::<u32>(w(p));
        g.write::<u32>(at, w(rec));
        g.call(slot_target(g, w(tab), 0xb24 / 4), 0, &[model]);
        let fl = g.read::<u16>(model + 0x124) & 0x7fff;
        g.write::<u16>(model + 0x124, fl);
    }
}

/// The actor matrix `M = E + 0x9c0`: the unit matrix (slot 235), its
/// diagonal slot 277 `(*G_B)`'s ST0 times `K`, times slots 15, 16 and 17
/// of `*G_B` (`first[i]`: the slot's factor on the left, the original's
/// operand order), then slot 330, 328 and 329 `(M, M, a)` by `E + 0xaa8`,
/// `+0xaa0`, `+0xaa4` (operands: the slot table, `G_B` for 277, `K`,
/// `G_B` again). Returns slot 329's `eax`.
fn actor_matrix<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    e: u32,
    (tab, b1, k, b2): (u32, u32, u32, u32),
    first: [bool; 3],
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let tab = w(tab);
    let s = |g: &G, n: u32| slot_target(g, tab, n);
    let m = e.wrapping_add(0x9c0);
    g.call(s(g, 0x3ac / 4), 0, &[m]);
    let b = g.read::<u32>(w(b1));
    let r = st0_f32(g.call(s(g, 0x454 / 4), 0, &[b]).st0);
    let d = mul(r, g.read::<f32>(w(k)));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + o, d);
    }
    let b = g.read::<u32>(w(b2));
    for (i, (slot, o)) in [(0x3cu32, 0u32), (0x40, 0x14), (0x44, 0x28)]
        .into_iter()
        .enumerate()
    {
        let r = st0_f32(g.call(s(g, slot / 4), 0, &[b]).st0);
        let x = g.read::<f32>(m + o);
        g.write::<f32>(m + o, if first[i] { mul(r, x) } else { mul(x, r) });
    }
    let mut ret = 0;
    for (slot, at) in [(0x528u32, 0xaa8u32), (0x520, 0xaa0), (0x524, 0xaa4)] {
        let a = g.read::<u32>(e.wrapping_add(at));
        // call width: 3
        ret = g.call(s(g, slot / 4), 0, &[m, m, a]).eax;
    }
    ret
}

/// `E + 0xac0` (4 words) into the object's position `+0xd0`.
fn place_object<G: Game>(g: &mut G, e: u32, obj: u32) {
    for j in 0..4 {
        let x = g.read::<u32>(e.wrapping_add(0xac0 + 4 * j));
        g.write::<u32>(obj + 0xd0 + 4 * j, x);
    }
}

/// `magic_0168`'s set-up of the ribbons and the summon (0x10003930, 1375
/// bytes; `magic_0599` 0x100071c0; a table entry, no arguments): with `E`
/// = `*G_E`, the four ribbons `E + 0xea8 + 0x8a8 i` reset (the word
/// `*G_D + 0x20`, flags, colours, widths, the 128 point flags cleared, 150
/// pieces 2.0 apart, fade 0x18); `E + 0x315c` = 1; the model `E + 0x130`
/// taken off the list `G_L`
/// ([`list_remove`](crate::l5_objhost::list_remove)) and, when it was
/// there, `**G_P = REC`, slot 713 `(model)` and bit 15 of its `+0x124`
/// cleared; two objects
/// ([`start_object_anim`](crate::s4_misc::start_object_anim) `E + 0x720`
/// `(6, 0)` and `E + 0x850` `(7, 0)`) at `E + 0xac0` lowered by `K1`/`K2`
/// with the yaw `E + 0x3150` (the first also scaled by `K3`); `E + 0xab4`
/// lowered by `K4`; `E + 0xaa4` the yaw plus `K` wrapped once; the matrix
/// `E + 0x9c0` of the actor `*G_B` (slots 235, 277, 15, 16, 17, then 330,
/// 328, 329 by `E + 0xaa8`, `+0xaa0`, `+0xaa4`); the matrix `E + 0xa00`
/// set to constants; two words; a node of the free ring (taken as
/// [`ring_take`](crate::r15::ring_take); none for an empty ring, where
/// the original writes through address 0) into `E + 0x314c` and last on
/// the task list. Returns the old tail, `0xffff` or slot 329's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn ribbons_setup_0168<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::s4_misc::start_object_anim;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let word = g.read::<u32>(g.read::<u32>(w(0x7)).wrapping_add(0x20));
    let e = g.read::<u32>(w(0xe));
    ribbons_reset(g, e, word, 0x4000_0000);
    g.write::<u32>(e.wrapping_add(0x315c), 1);
    model_off(
        g,
        img,
        f,
        e.wrapping_add(0x130),
        (0xce, 0xb7, 0xd7, 0xde, 0xe3),
    );
    let place = |g: &mut G, obj: u32, k_lower: u32| {
        place_object(g, e, obj);
        let y = sub(g.read::<f32>(obj + 0xd4), k(g, k_lower));
        g.write::<f32>(obj + 0xd4, y);
    };
    let o1 = e.wrapping_add(0x720);
    start_object_anim(g, img, img.rel32_target(f + 0x116), o1, k(g, 0x102), 6, 0);
    place(g, o1, 0x14c);
    let sc = mul(g.read::<f32>(o1 + 0x64), k(g, 0x176));
    let yaw = g.read::<u32>(e.wrapping_add(0x3150));
    g.write::<u32>(o1 + 0xc4, yaw);
    g.write::<f32>(o1 + 0x64, sc);
    let o2 = e.wrapping_add(0x850);
    start_object_anim(g, img, img.rel32_target(f + 0x198), o2, k(g, 0x124), 7, 0);
    place(g, o2, 0x1c6);
    let yaw = g.read::<u32>(e.wrapping_add(0x3150));
    g.write::<u32>(o2 + 0xc4, yaw);
    let y = sub(g.read::<f32>(e.wrapping_add(0xac4)), k(g, 0x1ff));
    g.write::<f32>(e.wrapping_add(0xab4), y);
    let y = add(g.read::<f32>(e.wrapping_add(0x3150)), k(g, 0x217));
    let y = wrap_once(g, img, f, y, (0x21e, 0x228, 0x232, 0x23f));
    g.write::<f32>(e.wrapping_add(0xaa4), y);
    let mut ret = actor_matrix(
        g,
        img,
        f,
        e,
        (0x244, 0x266, 0x27e, 0x297),
        [true, false, false],
    );
    for (j, v) in (0u32..).zip([
        0x4000_0000u32,
        0x4000_0000,
        0x4000_0000,
        0x3f80_0000,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0xc1a0_0000,
        0x41a0_0000,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0x3f80_0000,
        0,
        0,
        0,
        0x3f80_0000,
    ]) {
        g.write::<u32>(e.wrapping_add(0xa00 + 4 * j), v);
    }
    let count = g.read::<u16>(w(0x3e4));
    g.write::<u32>(e.wrapping_add(0x3154), 1);
    g.write::<u32>(e.wrapping_add(0x3158), 0x64);
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x4b2));
        crate::r15::ring_take(g, img, f, node, count, (0x4cf, 0x4d7, 0x508, 0x513));
        ret = 0xffff;
        node
    };
    let head = g.read::<u32>(w(0x519));
    g.write::<u32>(e.wrapping_add(0x314c), node);
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x54e), node);
        g.write::<u32>(w(0x554), node);
    } else {
        let tail = g.read::<u32>(w(0x530));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(w(0x53b), node);
        ret = tail;
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(ret)
}

/// [`ribbons_setup_0168`] (1375 bytes, 2 overlays).
pub const RIBBONS_SETUP_0168: Sig = Sig {
    len: 1375,
    hash: 0xf507_029f_c1d5_a2eb,
    calls: &[
        (0xce, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x116, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x198, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};

/// `magic_0168`'s light records and the two light tasks (0x100023a0, 422
/// bytes, a table entry, no arguments; `E` = `*G_E`): slot 371 `(0,
/// 0x10000)` and `(1, 0x20000)`; the records `E + 0xb00` and `E + 0xba0`
/// (colours, sizes, times), each held by a task node
/// ([`take`](crate::sa_pool::take), owner the immediate `+0x3b`/`+0xca`;
/// `+0x14` the record) last on the task list; the record `E + 0xd48`
/// (colour, flags, three sizes), which goes on to
/// [`object_task`](crate::m0168::object_task) (a tail jump with `ecx` =
/// the record). Returns its node.
pub fn lights_0168<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let tab = w(0x1);
    g.call(slot_target(g, tab, 0x5cc / 4), 0, &[0, 0x10000]);
    g.call(slot_target(g, w(0x18), 0x5cc / 4), 0, &[1, 0x20000]);
    let light = |g: &mut G, e_op: u32, at: u32, (c0, c1, t, n): (u32, u32, u32, u32)| {
        let r = g.read::<u32>(w(e_op)).wrapping_add(at);
        g.write::<u32>(r + 0x80, c0);
        g.write::<u32>(r + 0x84, c1);
        g.write::<u32>(r + 0x88, t);
        g.write::<u16>(r + 0x98, 0x48);
        g.write::<u32>(r + 0x90, n);
        g.write::<u32>(r + 0x8c, n);
        g.write::<u32>(r + 0x94, 0);
        r
    };
    let r1 = light(g, 0x2d, 0xb00, (0x00ff_ffff, 0x80ff_ffff, 0x5a, 0x14));
    let n1 = crate::sa_pool::take(g, img, call(0x85), w(0x3b));
    g.write::<u32>(n1.wrapping_add(0x14), r1);
    g.write::<u32>(n1.wrapping_add(4), 0);
    if g.read::<u32>(w(0x8b)) == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0xb6), n1);
    } else {
        let tail = g.read::<u32>(w(0xa1));
        g.write::<u32>(n1, tail);
        g.write::<u32>(tail.wrapping_add(4), n1);
    }
    let r2 = light(g, 0xbc, 0xba0, (0x80ff_ffff, 0x00ff_ffff, 0x6e, 0x28));
    g.write::<u32>(n1.wrapping_add(8), n1);
    let n2 = crate::sa_pool::take(g, img, call(0x114), w(0xca));
    g.write::<u32>(n2.wrapping_add(0x14), r2);
    g.write::<u32>(n2.wrapping_add(4), 0);
    // (`ebx`, the old head or the first node, is never 0 here)
    g.write::<u32>(n2, n1);
    g.write::<u32>(n1.wrapping_add(4), n2);
    let r3 = g.read::<u32>(w(0x13a)).wrapping_add(0xd48);
    g.write::<u32>(n2.wrapping_add(8), n2);
    g.write::<u32>(w(0x149), n2);
    g.write::<u32>(r3 + 0xf0, 0x5080_8080);
    g.write::<u16>(r3 + 0x124, 2);
    g.write::<u32>(r3 + 0x120, 0x80);
    g.write::<u32>(r3 + 0x11c, 0);
    g.write::<u32>(r3 + 0x110, 0);
    g.write::<u32>(r3 + 0xf8, 0x3c23_d70a);
    g.write::<u32>(r3 + 0xfc, 0x3d23_d70a);
    g.write::<u32>(r3 + 0x100, 0x3ca3_d70a);
    Ret::eax(crate::m0168::object_task(g, img, call(0x1a2), r3))
}

/// [`lights_0168`] (422 bytes, 1 overlay).
pub const LIGHTS_0168: Sig = Sig {
    len: 422,
    hash: 0x7f5e_790c_a4ec_ad31,
    calls: &[
        (0x85, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x114, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1a2, Callee::Sig(&crate::m0168::OBJECT_TASK)),
    ],
    imports: &[],
};

/// `magic_0168`'s summon throw (0x100026f0, 1401 bytes, a table entry, no
/// arguments; a /GS frame, the matrix `L` at `ebp - 0x54`, the point `V`
/// at `ebp - 0x14`; `E` = `*G_E`): the object `E + 0x5f0`
/// ([`start_object_anim`](crate::s4_misc::start_object_anim) `(5, 0)`)
/// at `E + 0xac0` with the yaw `E + 0x3150`, moved by `(0, 0, 3700)`
/// turned by the yaw (slots 235, 329, 229; a share of it, `/ K`, into
/// `E + 0xaf4`/`+0xafc`); the angles `E + 0xaa0` less `K2` and `E +
/// 0xaa8` plus `HI` wrapped once; the actor matrix ([`actor_matrix`]);
/// the targets `E + 0xae0` (`(50, 0, -1200)` turned), `E + 0xab0` and
/// `E + 0xaf0` (plus `(10, 0, 0)` turned), shares into `E + 0xae0`/
/// `+0xae8`; slot 256 `(*G_B, 0.5, 0.5, 0.5, 1)`; the ribbons reset
/// ([`ribbons_reset`], 4.0 apart); `E + 0x315c` = 1; unless `E +
/// 0x3148` is set, a node of the free ring (the ribbons' step, as
/// [`ring_take`](crate::r15::ring_take); an empty ring writes through
/// address 0 as the original does) last on the task list into `E +
/// 0x3148`. Returns the old tail, `0xffff` or `0xffffffff`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_throw_0168<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let (l, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let e = g.read::<u32>(w(0x1b));
    let at = |o: u32| e.wrapping_add(o);
    let obj = at(0x5f0);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x33), obj, k(g, 0x14), 5, 0);
    place_object(g, e, obj);
    let yaw = g.read::<u32>(at(0x3150));
    g.write::<u32>(obj + 0xc4, yaw);
    let tab = w(0x78);
    let s = |g: &G, n: u32| slot_target(g, tab, n);
    let set_v = |g: &mut G, x: [u32; 4]| {
        for (i, x) in (0u32..).zip(x) {
            g.write::<u32>(v + 4 * i, x);
        }
    };
    g.call(s(g, 0x3ac / 4), 0, &[l]);
    let yaw = g.read::<u32>(at(0x3150));
    // call width: 3
    g.call(s(g, 0x524 / 4), 0, &[l, l, yaw]);
    set_v(g, [0, 0, 0x4567_4000, 0x3f80_0000]);
    // call width: 3
    g.call(s(g, 0x394 / 4), 0, &[v, l, v]);
    let (vx, vz) = (g.read::<f32>(v), g.read::<f32>(v + 8));
    let x = add(vx, g.read::<f32>(obj + 0xd0));
    g.write::<f32>(obj + 0xd0, x);
    let z = add(vz, g.read::<f32>(obj + 0xd8));
    g.write::<f32>(obj + 0xd8, z);
    g.write::<f32>(at(0xaf4), div(vx, k(g, 0xf4)));
    g.write::<f32>(at(0xafc), div(vz, k(g, 0x107)));
    let (lo, hi, turn) = (k(g, 0x117), k(g, 0x132), k(g, 0x152));
    let wrap = |y: f32| {
        if y > hi {
            sub(y, turn)
        } else if lo > y {
            add(y, turn)
        } else {
            y
        }
    };
    let y = wrap(sub(g.read::<f32>(at(0xaa0)), k(g, 0x14a)));
    g.write::<f32>(at(0xaa0), y);
    let y = wrap(add(g.read::<f32>(at(0xaa8)), hi));
    g.write::<f32>(at(0xaa8), y);
    actor_matrix(
        g,
        img,
        f,
        e,
        (0x193, 0x1b6, 0x1ce, 0x1e7),
        [false, false, false],
    );
    g.write::<u32>(at(0xae4), 0);
    set_v(g, [0x4248_0000, 0, 0xc496_0000, 0x3f80_0000]);
    // call width: 3
    g.call(s(g, 0x394 / 4), 0, &[at(0xae0), l, v]);
    let x = add(g.read::<f32>(at(0xae0)), g.read::<f32>(at(0xac0)));
    set_v(g, [0x4120_0000, 0, 0, 0x3f80_0000]);
    g.write::<f32>(at(0xab0), x);
    let y = add(g.read::<f32>(at(0xac4)), g.read::<f32>(at(0xae4)));
    g.write::<f32>(at(0xab4), y);
    let z = add(g.read::<f32>(at(0xac8)), g.read::<f32>(at(0xae8)));
    g.write::<f32>(at(0xab8), z);
    for (to, from) in [(0xabcu32, 0xaccu32), (0xaf0, 0xab0), (0xaf8, 0xab8)] {
        let x = g.read::<u32>(at(from));
        g.write::<u32>(at(to), x);
    }
    // call width: 3
    g.call(s(g, 0x394 / 4), 0, &[v, l, v]);
    let x = add(g.read::<f32>(at(0xaf0)), g.read::<f32>(v));
    g.write::<f32>(at(0xaf0), x);
    let z = add(g.read::<f32>(at(0xaf8)), g.read::<f32>(v + 8));
    g.write::<f32>(at(0xaf8), z);
    let x = div(g.read::<f32>(at(0xae0)), k(g, 0x3c6));
    g.write::<f32>(at(0xae0), x);
    let z = div(g.read::<f32>(at(0xae8)), k(g, 0x3e1));
    g.write::<f32>(at(0xae8), z);
    let b = g.read::<u32>(w(0x3ef));
    // call width: 5
    g.call(
        s(g, 0x400 / 4),
        0,
        &[b, 0x3f00_0000, 0x3f00_0000, 0x3f00_0000, 1],
    );
    let word = g.read::<u32>(g.read::<u32>(w(0x3fc)).wrapping_add(0x20));
    ribbons_reset(g, e, word, 0x4080_0000);
    let set = g.read::<u32>(at(0x3148));
    g.write::<u32>(at(0x315c), 1);
    if set != 0 {
        return Ret::eax(0xffff_ffff);
    }
    let mut ret = 0xffff_ffff;
    let count = g.read::<u16>(w(0x4be));
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x4cd));
        crate::r15::ring_take(g, img, f, node, count, (0x4ea, 0x4f2, 0x523, 0x52e));
        ret = 0xffff;
        node
    };
    g.write::<u32>(node.wrapping_add(4), 0);
    if g.read::<u32>(w(0x534)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x556), node);
    } else {
        let tail = g.read::<u32>(w(0x543));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        ret = tail;
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x55f), node);
    g.write::<u32>(at(0x3148), node);
    Ret::eax(ret)
}

/// [`summon_throw_0168`] (1401 bytes, 1 overlay).
pub const SUMMON_THROW_0168: Sig = Sig {
    len: 1401,
    hash: 0xe064_7d98_e6f5_9dcf,
    calls: &[
        (0x33, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        // __security_check_cookie
        (0x571, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0168`'s summon flight step (0x10002c70, 714 bytes, a table
/// entry, no arguments; a /GS frame, the matrices `L` at `ebp - 0x44` and
/// `N` at `ebp - 0x84`; `E` = `*G_E`): the targets `E + 0xaf0`/`+0xaf8`
/// less their shares `E + 0xae0`/`+0xae8`, `E + 0xab0`/`+0xab8` those
/// plus a random 0.1 each (slot 314); the actor matrix
/// ([`actor_matrix`]); the spin `E + 0xae4` plus `K` wrapped once; `L`
/// the unit matrix turned by the spin (slots 235, 329), slot 231 `(N, M,
/// L)` and 230 `(M, N)`; the model's three halfwords `E + 0x660..` its
/// `+0x664` less 17; `E + 0x6c0`/`+0x6c8` less `E + 0xaf4`/`+0xafc`.
/// Returns the halfword less 17 (32 bits).
pub fn summon_flight_0168<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let (l, n) = (ebp.wrapping_sub(0x44), ebp.wrapping_sub(0x84));
    let e = g.read::<u32>(w(0x23));
    let at = |o: u32| e.wrapping_add(o);
    let tab = w(0x14);
    let less = |g: &mut G, to: u32, by: u32| {
        let x = sub(g.read::<f32>(at(to)), g.read::<f32>(at(by)));
        g.write::<f32>(at(to), x);
    };
    less(g, 0xaf0, 0xae0);
    less(g, 0xaf8, 0xae8);
    let r = random(g, tab, f32::from_bits(0x3dcc_cccd));
    let x = add(r, g.read::<f32>(at(0xaf0)));
    g.write::<f32>(at(0xab0), x);
    let r = random(g, w(0x78), f32::from_bits(0x3dcc_cccd));
    let z = add(g.read::<f32>(at(0xaf8)), r);
    g.write::<f32>(at(0xab8), z);
    actor_matrix(
        g,
        img,
        f,
        e,
        (0xaa, 0xcc, 0xea, 0x103),
        [false, false, true],
    );
    let y = add(g.read::<f32>(at(0xae4)), k(g, 0x1d2));
    let y = wrap_once(g, img, f, y, (0x1dc, 0x1e6, 0x1f0, 0x1fd));
    g.write::<f32>(at(0xae4), y);
    let s = |g: &G, n: u32| slot_target(g, tab, n);
    g.call(s(g, 0x3ac / 4), 0, &[l]);
    let spin = g.read::<u32>(at(0xae4));
    // call width: 3
    g.call(s(g, 0x524 / 4), 0, &[l, l, spin]);
    let m = at(0x9c0);
    // call width: 3
    g.call(s(g, 0x39c / 4), 0, &[n, m, l]);
    g.call(s(g, 0x398 / 4), 0, &[m, n]);
    let h = u32::from(g.read::<u16>(at(0x664))).wrapping_add(0xffff_ffef);
    for o in [0x664u32, 0x662, 0x660] {
        g.write::<u16>(at(o), h as u16);
    }
    less(g, 0x6c0, 0xaf4);
    less(g, 0x6c8, 0xafc);
    Ret::eax(h)
}

/// [`summon_flight_0168`] (714 bytes, 1 overlay).
pub const SUMMON_FLIGHT_0168: Sig = Sig {
    len: 714,
    hash: 0x806f_8875_f764_7171,
    calls: &[
        // __security_check_cookie
        (0x2c2, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0168`'s summon landing (0x10002fb0, 1003 bytes, a table entry,
/// no arguments; `E` = `*G_E`): the model `E + 0x5f0` off the list (as
/// [`model_off`]); the object `E + 0x390`
/// ([`start_object_anim`](crate::s4_misc::start_object_anim) `(3, 0)`) at
/// `E + 0xac0` with the yaw `E + 0x3150`; the spin `E + 0xae4` 15.0, `E +
/// 0xaf4` 0.75, the angles `(0, yaw, 0)`; the actor matrix
/// ([`actor_matrix`]); `E + 0xab0` = `E + 0xac0` raised by `K`; the
/// ribbons reset ([`ribbons_reset`], 4.0 apart); slot 256 `(*G_B, 1.0,
/// 1.0, 1.0, 1)`; the record `E + 0xc40` (counts, times); a node of the
/// free ring (as [`ring_take`](crate::r15::ring_take); an empty ring
/// writes through address 0 as the original does) holding the record
/// (byte `+0x18` 1) last on the task list. Returns the old tail, `0xffff`
/// or 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_landing_0168<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = g.read::<u32>(w(0x9));
    let at = |o: u32| e.wrapping_add(o);
    model_off(g, img, f, at(0x5f0), (0x20, 0x18, 0x29, 0x30, 0x35));
    let obj = at(0x390);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x68), obj, k(g, 0x54), 3, 0);
    place_object(g, e, obj);
    let yaw = g.read::<u32>(at(0x3150));
    g.write::<u32>(obj + 0xc4, yaw);
    g.write::<u32>(at(0xae4), 0x4170_0000);
    g.write::<u32>(at(0xaf4), 0x3f40_0000);
    g.write::<u32>(at(0xaa0), 0);
    let yaw = g.read::<u32>(at(0x3150));
    g.write::<u32>(at(0xaa4), yaw);
    g.write::<u32>(at(0xaa8), 0);
    actor_matrix(
        g,
        img,
        f,
        e,
        (0xd3, 0xf7, 0x10f, 0x128),
        [false, true, false],
    );
    let x = g.read::<u32>(at(0xac0));
    let y = add(g.read::<f32>(at(0xac4)), k(g, 0x1eb));
    g.write::<u32>(at(0xab0), x);
    for (to, from) in [(0xab8u32, 0xac8u32), (0xabc, 0xacc)] {
        let x = g.read::<u32>(at(from));
        g.write::<u32>(at(to), x);
    }
    g.write::<f32>(at(0xab4), y);
    let word = g.read::<u32>(g.read::<u32>(w(0x216)).wrapping_add(0x20));
    ribbons_reset(g, e, word, 0x4080_0000);
    let b = g.read::<u32>(w(0x2e9));
    // call width: 5
    g.call(
        slot_target(g, w(0x2c1), 0x400 / 4),
        0,
        &[b, 0x3f80_0000, 0x3f80_0000, 0x3f80_0000, 1],
    );
    let count = g.read::<u16>(w(0x2f5));
    let rec = at(0xc40);
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x50),
        (8, 0),
        (0xc, 0),
        (0x14, 0x2d),
        (0x10, 0x2d),
    ] {
        g.write::<u32>(rec + o, v);
    }
    g.write::<u16>(rec + 0x18, 0);
    let mut ret = 0;
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x33c));
        crate::r15::ring_take(g, img, f, node, count, (0x359, 0x361, 0x392, 0x39d));
        ret = 0xffff;
        node
    };
    let head = g.read::<u32>(w(0x3a3));
    g.write::<u32>(node.wrapping_add(0x14), rec);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x3da), node);
        g.write::<u32>(w(0x3e0), node);
    } else {
        let tail = g.read::<u32>(w(0x3bc));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(w(0x3c7), node);
        ret = tail;
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(ret)
}

/// [`summon_landing_0168`] (1003 bytes, 1 overlay).
pub const SUMMON_LANDING_0168: Sig = Sig {
    len: 1003,
    hash: 0x3a0e_a27c_2717_8a32,
    calls: &[
        (0x20, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x68, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};

/// `magic_0168`'s summon spin step (0x100036c0, 456 bytes, a table entry,
/// no arguments; `E` = `*G_E`): the spin `E + 0xae4` less `E + 0xaf4`,
/// the yaw `E + 0xaa4` plus the spin wrapped once; the actor matrix
/// ([`actor_matrix`]); `E + 0xab4` raised by `K`; the four ribbons' fade
/// bytes up by one to 0x18. Returns slot 329's `eax` with the last fade
/// byte in its low byte.
pub fn summon_spin_0168<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = g.read::<u32>(w(0x7));
    let at = |o: u32| e.wrapping_add(o);
    let spin = sub(g.read::<f32>(at(0xae4)), g.read::<f32>(at(0xaf4)));
    g.write::<f32>(at(0xae4), spin);
    let y = add(g.read::<f32>(at(0xaa4)), spin);
    let y = wrap_once(g, img, f, y, (0x32, 0x3c, 0x46, 0x53));
    g.write::<f32>(at(0xaa4), y);
    let eax = actor_matrix(g, img, f, e, (0x58, 0x7c, 0x94, 0xad), [false, false, true]);
    let y = add(g.read::<f32>(at(0xab4)), g.read::<f32>(w(0x16a)));
    g.write::<f32>(at(0xab4), y);
    let mut al = 0u8;
    for o in [0x16b7u32, 0x1f5f, 0x2807, 0x30af] {
        al = g.read::<u8>(at(o));
        if al < 0x18 {
            al += 1;
            g.write::<u8>(at(o), al);
        }
    }
    Ret::eax((eax & !0xff) | u32::from(al))
}

/// [`summon_spin_0168`] (456 bytes, 1 overlay).
pub const SUMMON_SPIN_0168: Sig = Sig {
    len: 456,
    hash: 0x7fbc_fcee_3a53_1d98,
    calls: &[],
    imports: &[],
};

/// `magic_0333`'s summon task (0x10006650, 785 bytes with its byte and
/// jump tables; cdecl `(task)`, the record `R = task[1]`; an 8-aligned
/// frame, the group `L` at `A - 0x18`, `S = A - 0x30` after the pushes).
/// `R + 0x1c` and `+0x20` grow by `K`, the counter `R + 0x28` by one, then
/// by the state `R + 0x18`:
/// - 0: the summon's nodes
///   ([`summon_nodes_b`](crate::s4_small::summon_nodes_b), `L = [NODE,
///   LIST, *G_H, 0, 1]`, `NODE`'s vtable first), then state 10;
/// - 10: state 11, the counter 25 and the target index `R + 0x24` 0;
/// - 11: past a counter of 25, target `i`'s nodes: the node pair `A_i`/`B_i`
///   ([`node_append`](crate::sb_misc::node_append) to `LIST`, `B_i`'s
///   colour 1.0), the model node `M_i`
///   ([`node_insert`](crate::nodes::node_insert) `[M_i, LIST, ID, A_i,
///   PART, B_i]`, byte `+0x148` 1), `A_i + 0x1c` the target's bone 17
///   (slot 268; `w` 1.0; when the model does not take the direct way
///   ([`model_direct`](crate::s6_misc::model_direct)) its ground (slot 12)
///   less `K2` in `y`), the particle node `P_i`
///   ([`particle_node_with`](crate::s3_nodes::particle_node_with) `(M_i,
///   -, -, direct)`), the effect node `Q_i`
///   ([`effect_node`](crate::sb_nodes::effect_node) `[Q_i, LIST, LIST2,
///   M_i, NAME, 0]`, its vtable after) with its flag set and word cleared,
///   then state 12 and the counter 1;
/// - 12: past a counter of 20, slot 239 `(*G_H, target, 0)`, the next
///   target; after the last one the task's node leaves its ring; back to
///   state 11.
///
/// Other states only count. Returns the switch's `eax` (the state, the
/// table index or the last call's).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0333<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_misc::node_append;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let x = add(g.read::<f32>(r + 0x1c), g.read::<f32>(w(0x1b)));
    let state = g.read::<u32>(r + 0x18);
    let n = g.read::<u32>(r + 0x28).wrapping_add(1);
    g.write::<u32>(r + 0x28, n);
    g.write::<f32>(r + 0x1c, x);
    let y = add(g.read::<f32>(r + 0x20), g.read::<f32>(w(0x33)));
    g.write::<f32>(r + 0x20, y);
    // push ebp; and esp, -8; sub esp, 0x24; push ebx, esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    let l = s + 0x18;
    let count = |g: &mut G, by: u32| {
        let n = g.read::<u32>(r + 0x28).wrapping_add(by);
        g.write::<u32>(r + 0x28, n);
    };
    // the byte table: 0 -> 0, 10 -> 1, 11 -> 2, 12 -> 3, 1..9 -> 4
    let case = match state {
        0 => 0,
        10 => 1,
        11 => 2,
        12 => 3,
        1..=9 => 4,
        _ => {
            count(g, 1);
            return Ret::eax(state);
        }
    };
    let mut eax = case;
    if case == 0 {
        let h = g.read::<u32>(w(0x54));
        for (i, v) in (0u32..).zip([w(0x60), w(0x68), h, 0]) {
            g.write::<u32>(l + 4 * i, v);
        }
        g.write::<u8>(l + 0x10, 1);
        g.write::<u32>(w(0x7f), w(0x83));
        let nodes = Frame {
            at: call(0x88),
            esp: s - 4,
            ecx: l,
            ..*fr
        };
        eax = crate::s4_small::summon_nodes_b(g, img, &nodes).eax;
        g.write::<u32>(r + 0x18, 0xa);
    }
    if case <= 1 {
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x28, 0x19);
        g.write::<u32>(r + 0x24, 0);
    }
    if case <= 2 {
        if g.read::<i32>(r + 0x28) <= 0x19 {
            count(g, 1);
            return Ret::eax(eax);
        }
        let i = g.read::<u32>(r + 0x24);
        let n1 = i.wrapping_mul(0x2c).wrapping_add(w(0xc3));
        let part = (i << 8).wrapping_add(w(0xc9));
        let m = i.wrapping_mul(0x14c).wrapping_add(w(0xcf));
        let list = w(0xdc);
        let n2 = i.wrapping_mul(0x2c).wrapping_add(w(0xec));
        node_append(
            g,
            img,
            call(0xf1),
            i.wrapping_mul(0x2c).wrapping_add(w(0xe2)),
            list,
            0,
        );
        node_append(g, img, call(0xf8), n2, list, 0);
        for o in [0x1cu32, 0x20, 0x24, 0x28] {
            g.write::<u32>(n2 + o, 0x3f80_0000);
        }
        for (k, v) in (0u32..).zip([m, list, w(0x12c), n1, w(0x138), n2]) {
            g.write::<u32>(l + 4 * k, v);
        }
        let insert = Frame {
            at: call(0x141),
            esp: s - 4,
            ecx: l,
            ..*fr
        };
        crate::nodes::node_insert(g, img, &insert);
        g.write::<u8>(m + 0x148, 1);
        let user = g.read::<u32>(i.wrapping_mul(0x98).wrapping_add(w(0x157)));
        let direct = Frame {
            at: call(0x15c),
            esp: s - 4,
            ecx: user,
            ..*fr
        };
        let tab = w(0x180);
        let p = n1 + 0x1c;
        let flag = if crate::s6_misc::model_direct(g, img, &direct).eax != 0 {
            let t = i.wrapping_mul(0x98).wrapping_add(w(0x172));
            let c = g.read::<u32>(t + 0x54);
            // call width: 3
            g.call(slot_target(g, tab, 0x430 / 4), 0, &[c, 0x11, p]);
            g.write::<u32>(p + 0xc, 0x3f80_0000);
            1
        } else {
            let t = i.wrapping_mul(0x98).wrapping_add(w(0x1b8));
            let c = g.read::<u32>(t + 0x54);
            // call width: 3
            g.call(slot_target(g, w(0x1a8), 0x430 / 4), 0, &[c, 0x11, p]);
            g.write::<u32>(p + 0xc, 0x3f80_0000);
            let c = g.read::<u32>(t + 0x54);
            let ground = st0_f32(g.call(slot_target(g, w(0x1c4), 0x30 / 4), 0, &[c]).st0);
            g.write::<f32>(p + 4, ground);
            let y = sub(g.read::<f32>(n1 + 0x20), g.read::<f32>(w(0x1ee)));
            g.write::<f32>(n1 + 0x20, y);
            0
        };
        // push flag; sub esp, 8; push M: the return address at S - 0x14
        let esp = s.wrapping_sub(0x14);
        g.write::<u32>(esp + 4, m);
        g.write::<u32>(esp + 0x10, flag);
        let pn = Frame {
            at: call(0x205),
            esp,
            ecx: part,
            ..*fr
        };
        crate::s3_nodes::particle_node_with(g, img, &pn);
        let i = g.read::<u32>(r + 0x24);
        let q = i.wrapping_mul(56).wrapping_add(w(0x223));
        for (k, v) in (0u32..).zip([q, w(0x21c), w(0x233), m, w(0x23f), 0]) {
            g.write::<u32>(l + 4 * k, v);
        }
        crate::sb_nodes::effect_node(g, img, call(0x24c), l);
        let node = g.read::<u32>(l);
        g.write::<u32>(node, w(0x256));
        let i = g.read::<u32>(r + 0x24);
        let fl = i.wrapping_mul(56).wrapping_add(w(0x269));
        let v = g.read::<u32>(fl) | 1;
        g.write::<u32>(fl, v);
        g.write::<u32>(i.wrapping_mul(56).wrapping_add(w(0x271)), 0);
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x28, 1);
        return Ret::eax(i);
    }
    if case == 3 {
        if g.read::<i32>(r + 0x28) <= 0x14 {
            count(g, 1);
            return Ret::eax(eax);
        }
        let i = g.read::<u32>(r + 0x24);
        let target = g.read::<u32>(i.wrapping_mul(0x98).wrapping_add(w(0x2a0)));
        let h = g.read::<u32>(w(0x2ab));
        // call width: 3
        g.call(slot_target(g, w(0x2a5), 0x3bc / 4), 0, &[h, target, 0]);
        let i = i.wrapping_add(1);
        g.write::<u32>(r + 0x24, i);
        eax = i;
        if i.cast_signed() >= g.read::<i32>(w(0x2c2)) {
            let node = g.read::<u32>(task);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node + 8);
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(node + 8);
            eax = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, eax);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
        }
        let st = g.read::<u32>(r + 0x18).wrapping_sub(1);
        g.write::<u32>(r + 0x18, st);
    }
    count(g, 1);
    Ret::eax(eax)
}

/// [`summon_task_0333`] (785 bytes with its tables, 1 overlay).
pub const SUMMON_TASK_0333: Sig = Sig {
    len: 785,
    hash: 0x8ec3_f526_bfa7_1926,
    calls: &[
        (0x88, Callee::Sig(&crate::s4_small::SUMMON_NODES_B)),
        (0xf1, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xf8, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x141, Callee::Sig(&crate::nodes::INSERT)),
        (0x15c, Callee::Sig(&crate::s6_misc::MODEL_DIRECT)),
        (0x205, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x24c, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// A node appended to the summon list `LIST` as the original inlines it
/// (operands of `f`: `LIST` for `+0x14` and `+0x4`, its last child
/// `LIST + 0x10` read twice, `LIST` compared, its first child, its count,
/// its last child stored, the vtable): the node's own child list empty,
/// pass byte 0xff.
fn summon_append<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    [l1, l2, last1, last2, cmp, first, count, last_w, vt]: [u32; 9],
) {
    let w = |o: u32| img.word(f + o);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, w(l1));
    g.write::<u32>(node + 4, w(l2));
    let last = g.read::<u32>(w(last1));
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(w(last2));
    if last == w(cmp) {
        g.write::<u32>(w(first), node);
    } else {
        g.write::<u32>(last.wrapping_add(4), node);
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
    g.write::<u32>(w(last_w), node);
    g.write::<u32>(node, w(vt));
}

/// `magic_0332`'s summon task (0x10006550, 808 bytes; cdecl `(task)`, the
/// record `R = task[1]`; an 8-aligned frame, the group `L` at `A - 0x18`,
/// `S = A - 0x30` after the pushes). `R + 0x1c` and `+0x20` grow by `K`,
/// the counter `R + 0x28` by one, then by the state `R + 0x18`:
/// - 0: the summon's nodes
///   ([`summon_nodes_b`](crate::s4_small::summon_nodes_b), `L = [NODE,
///   LIST, *G_H, 0, 1]`), then state 10;
/// - 10: state 11, the counter 25, the target index `R + 0x24` 0;
/// - 11: past a counter of 25, target `i`: slot 239 `(*G_H, target, 0)`,
///   the nodes `A_i` (at the target's bone 17, slot 268, `w` 1.0) and
///   `B_i` (colour 1, 1, 1, 0.3) appended to `LIST` ([`summon_append`]),
///   the model node `M_i` ([`node_insert`](crate::nodes::node_insert)
///   `[M_i, LIST, ID, A_i, PART, B_i]`, byte `+0x148` 1), the particle node
///   `P_i` ([`particle_node`](crate::sb_nodes::particle_node) `[P_i,
///   LIST2, M_i, *G_O, PROG, 0]`, its vtable first), the effect node `Q_i`
///   ([`effect_node`](crate::sb_nodes::effect_node) `[Q_i, LIST, LIST3,
///   M_i, NAME, 0]`, its vtable after) with its flag set and word
///   cleared; the next target, after the last one state 10000; the
///   counter 1;
/// - 10000: the task's node leaves its ring.
///
/// Other states only count. Returns the state, the summon nodes' `eax`,
/// the next target or the node's old next.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0332<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let x = add(g.read::<f32>(r + 0x1c), g.read::<f32>(w(0x1b)));
    let state = g.read::<u32>(r + 0x18);
    let n = g.read::<u32>(r + 0x28).wrapping_add(1);
    g.write::<u32>(r + 0x28, n);
    g.write::<f32>(r + 0x1c, x);
    let y = add(g.read::<f32>(r + 0x20), g.read::<f32>(w(0x33)));
    g.write::<f32>(r + 0x20, y);
    // push ebp; and esp, -8; sub esp, 0x24; push ebx, esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    let l = s + 0x18;
    let count = |g: &mut G| {
        let n = g.read::<u32>(r + 0x28).wrapping_add(1);
        g.write::<u32>(r + 0x28, n);
    };
    let mut eax = state;
    match state {
        0 | 10 | 11 => {}
        0x2710 => {
            let node = g.read::<u32>(task);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node + 8);
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(node + 8);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            count(g);
            return Ret::eax(next);
        }
        _ => {
            count(g);
            return Ret::eax(state);
        }
    }
    if state == 0 {
        let h = g.read::<u32>(w(0x5b));
        for (i, v) in (0u32..).zip([w(0x67), w(0x6f), h, 0]) {
            g.write::<u32>(l + 4 * i, v);
        }
        g.write::<u8>(l + 0x10, 1);
        g.write::<u32>(w(0x86), w(0x8a));
        let nodes = Frame {
            at: call(0x8f),
            esp: s - 4,
            ecx: l,
            ..*fr
        };
        eax = crate::s4_small::summon_nodes_b(g, img, &nodes).eax;
        g.write::<u32>(r + 0x18, 0xa);
    }
    if state != 11 {
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x28, 0x19);
        g.write::<u32>(r + 0x24, 0);
    }
    if g.read::<i32>(r + 0x28) <= 0x19 {
        count(g);
        return Ret::eax(eax);
    }
    let i = g.read::<u32>(r + 0x24);
    let m = i.wrapping_mul(0x14c).wrapping_add(w(0xc1));
    let n1 = i.wrapping_mul(0x2c).wrapping_add(w(0xd2));
    let n2 = i.wrapping_mul(0x2c).wrapping_add(w(0xd8));
    let target = g.read::<u32>(i.wrapping_mul(0x98).wrapping_add(w(0xe6)));
    let part = (i << 8).wrapping_add(w(0xf4));
    let h = g.read::<u32>(w(0xef));
    // call width: 3
    g.call(slot_target(g, w(0xfd), 0x3bc / 4), 0, &[h, target, 0]);
    summon_append(
        g,
        img,
        f,
        n1,
        [
            0x11c, 0x123, 0x128, 0x130, 0x138, 0x140, 0x14c, 0x152, 0x158,
        ],
    );
    let i = g.read::<u32>(r + 0x24);
    let t = i.wrapping_mul(0x98).wrapping_add(w(0x166));
    let c = g.read::<u32>(t + 0x54);
    // call width: 3
    g.call(
        slot_target(g, w(0x174), 0x430 / 4),
        0,
        &[c, 0x11, n1 + 0x1c],
    );
    g.write::<u32>(n1 + 0x28, 0x3f80_0000);
    summon_append(
        g,
        img,
        f,
        n2,
        [
            0x196, 0x19d, 0x1a2, 0x1aa, 0x1b2, 0x1ba, 0x1c6, 0x1d4, 0x1da,
        ],
    );
    for o in [0x1cu32, 0x20, 0x24] {
        g.write::<u32>(n2 + o, 0x3f80_0000);
    }
    g.write::<u32>(n2 + 0x28, 0x3e99_999a);
    for (k, v) in (0u32..).zip([m, w(0x206), w(0x20e), n1, w(0x21a), n2]) {
        g.write::<u32>(l + 4 * k, v);
    }
    let insert = Frame {
        at: call(0x223),
        esp: s - 4,
        ecx: l,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &insert);
    let owner = g.read::<u32>(w(0x22c));
    g.write::<u8>(m + 0x148, 1);
    g.write::<u32>(l, part);
    g.write::<u32>(part, w(0x23d));
    for (k, v) in (1u32..).zip([w(0x249), m, owner, w(0x259), 0]) {
        g.write::<u32>(l + 4 * k, v);
    }
    crate::sb_nodes::particle_node(g, img, call(0x266), l);
    let i = g.read::<u32>(r + 0x24);
    g.write::<u32>(l + 4, w(0x271));
    g.write::<u32>(l + 8, w(0x282));
    let q = i.wrapping_mul(56).wrapping_add(w(0x289));
    g.write::<u32>(l, q);
    g.write::<u32>(l + 0xc, m);
    g.write::<u32>(l + 0x10, w(0x29d));
    g.write::<u32>(l + 0x14, 0);
    crate::sb_nodes::effect_node(g, img, call(0x2aa), l);
    let node = g.read::<u32>(l);
    g.write::<u32>(node, w(0x2b4));
    let i = g.read::<u32>(r + 0x24);
    let fl = i.wrapping_mul(56).wrapping_add(w(0x2c7));
    let v = g.read::<u32>(fl) | 1;
    g.write::<u32>(fl, v);
    g.write::<u32>(i.wrapping_mul(56).wrapping_add(w(0x2cf)), 0);
    let i = g.read::<u32>(r + 0x24).wrapping_add(1);
    g.write::<u32>(r + 0x24, i);
    if i.cast_signed() >= g.read::<i32>(w(0x2df)) {
        g.write::<u32>(r + 0x18, 0x2710);
    }
    g.write::<u32>(r + 0x28, 1);
    Ret::eax(i)
}

/// [`summon_task_0332`] (808 bytes, 1 overlay).
pub const SUMMON_TASK_0332: Sig = Sig {
    len: 808,
    hash: 0xb62a_3413_2b98_5029,
    calls: &[
        (0x8f, Callee::Sig(&crate::s4_small::SUMMON_NODES_B)),
        (0x223, Callee::Sig(&crate::nodes::INSERT)),
        (0x266, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x2aa, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// `magic_0399`'s summon task (0x100069d0, 1235 bytes; cdecl `(task)`,
/// the record `R = task[1]`; an 8-aligned frame, the groups at `A - 0x18`
/// and `A - 0x20`, `S = A - 0x38` after the pushes). `R + 0x1c` and
/// `+0x20` grow by `K`, the counter `R + 0x28` by one, then by the state
/// `R + 0x18`:
/// - 0: the summon's model nodes
///   ([`model_nodes`](crate::s3_summon::model_nodes) `[NODE, LIST, *G_H,
///   0, 0]`), the colour node ([`colour_node`](crate::s3_misc::colour_node)
///   `[CNODE, CLIST]`), seven light words 0.5 and the camera move
///   ([`move_start`](crate::sb_misc::move_start) by `K2`), then state 11,
///   the counter 100 and the target index `R + 0x24` 0 (and on to 11);
/// - 10: as the end of 0;
/// - 11: past a counter of 25, target `i`'s nodes as `magic_0332`'s
///   ([`summon_append`] `A_i`, `B_i`; [`node_insert`](crate::nodes::node_insert);
///   the bone 17 or the ground by [`model_direct`](crate::s6_misc::model_direct);
///   [`particle_node`](crate::sb_nodes::particle_node) `[P_i, PLIST, M_i,
///   *G_O, PROG, direct]`; [`effect_node`](crate::sb_nodes::effect_node)
///   `[Q_i, LIST, M_i, LIST3, NAME, 0]`), then state 12 and the counter 1;
/// - 12: past a counter of 20, slot 239 `(*G_C, target, 0)` and the next
///   target: back to 11, after the last one on to 13;
/// - 13: past a counter of 80, the light words 1.0 and the second camera
///   move ([`move_start_2`](crate::sb_misc::move_start_2) by `K3`, `K4`),
///   state 10000;
/// - 10000: the task's node leaves its ring.
///
/// Other states only count. Returns the switch's `eax` (the state, a
/// compare's left-over or the last call's).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0399<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let x = add(g.read::<f32>(r + 0x1c), k(g, 0x1a));
    let n = g.read::<u32>(r + 0x28).wrapping_add(1);
    g.write::<u32>(r + 0x28, n);
    let state = g.read::<u32>(r + 0x18);
    let counter = g.read::<i32>(r + 0x28);
    g.write::<f32>(r + 0x1c, x);
    let y = add(g.read::<f32>(r + 0x20), k(g, 0x36));
    g.write::<f32>(r + 0x20, y);
    // push ebp; and esp, -8; sub esp, 0x2c; push ebx, esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let l = s + 0x20;
    let count = |g: &mut G| {
        let n = g.read::<u32>(r + 0x28).wrapping_add(1);
        g.write::<u32>(r + 0x28, n);
    };
    let unlink = |g: &mut G| {
        let node = g.read::<u32>(task);
        let next = g.read::<u32>(node + 4);
        let prev = g.read::<u32>(node + 8);
        g.write::<u32>(next + 8, prev);
        let prev = g.read::<u32>(node + 8);
        let next = g.read::<u32>(node + 4);
        g.write::<u32>(prev + 4, next);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 4, node);
        next
    };
    let mut eax = state;
    match state {
        0 => {
            let h = g.read::<u32>(w(0x57));
            for (i, v) in (0u32..).zip([w(0x63), w(0x6b), h, 0]) {
                g.write::<u32>(l + 4 * i, v);
            }
            g.write::<u8>(l + 0x10, 0);
            g.write::<u32>(w(0x82), w(0x86));
            let nodes = Frame {
                at: call(0x8b),
                esp: s - 4,
                ecx: l,
                ..*fr
            };
            crate::s3_summon::model_nodes(g, img, &nodes);
            let l2 = s + 0x18;
            g.write::<u32>(l2, w(0x97));
            g.write::<u32>(l2 + 4, w(0x9f));
            g.write::<u32>(w(0xa5), w(0xa9));
            let colour = Frame {
                at: call(0xae),
                esp: s - 4,
                ecx: l2,
                ..*fr
            };
            let e = crate::s3_misc::colour_node(g, img, &colour);
            let t = k(g, 0xb6);
            stores(g, img, f, &[0xba, 0xc4, 0xce, 0xd8, 0xe2, 0xec, 0xf6]);
            eax = crate::sb_misc::move_start(g, img, call(0x101), t, e);
            g.write::<u32>(r + 0x18, 0xa);
        }
        10 => {}
        // sub eax, 0xb: 0 left in eax
        11 => eax = 0,
        12 => {
            if counter <= 0x14 {
                count(g);
                return Ret::eax(0);
            }
            let i = g.read::<u32>(r + 0x24);
            let target = g.read::<u32>(i.wrapping_mul(0x98).wrapping_add(w(0x1c8)));
            let c = g.read::<u32>(w(0x1d3));
            // call width: 3
            g.call(slot_target(g, w(0x1cd), 0x3bc / 4), 0, &[c, target, 0]);
            let i = g.read::<u32>(r + 0x24).wrapping_add(1);
            g.write::<u32>(r + 0x24, i);
            let st = g.read::<u32>(r + 0x18);
            let st = if i.cast_signed() < g.read::<i32>(w(0x1ea)) {
                st.wrapping_sub(1)
            } else {
                st.wrapping_add(1)
            };
            g.write::<u32>(r + 0x18, st);
            count(g);
            return Ret::eax(i);
        }
        13 => {
            if counter <= 0x50 {
                count(g);
                return Ret::eax(state);
            }
            let (t3, t2) = (k(g, 0x440), k(g, 0x448));
            stores(
                g,
                img,
                f,
                &[0x44c, 0x456, 0x460, 0x46a, 0x474, 0x47e, 0x488],
            );
            let e = crate::sb_misc::move_start_2(g, img, call(0x493), t2, t3);
            count(g);
            g.write::<u32>(r + 0x18, 0x2710);
            return Ret::eax(e);
        }
        0x2710 => {
            let next = unlink(g);
            count(g);
            return Ret::eax(next);
        }
        _ => {
            count(g);
            return Ret::eax(state);
        }
    }
    if state != 11 {
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x28, 0x64);
        g.write::<u32>(r + 0x24, 0);
    }
    if g.read::<i32>(r + 0x28) <= 0x19 {
        count(g);
        return Ret::eax(eax);
    }
    let i = g.read::<u32>(r + 0x24);
    let n1 = i.wrapping_mul(0x2c).wrapping_add(w(0x133));
    let n2 = i.wrapping_mul(0x2c).wrapping_add(w(0x13f));
    let m = i.wrapping_mul(0x14c).wrapping_add(w(0x16f));
    let part = (i << 8).wrapping_add(w(0x175));
    summon_append(
        g,
        img,
        f,
        n1,
        [
            0x152, 0x159, 0x15e, 0x169, 0x186, 0x18e, 0x210, 0x216, 0x21c,
        ],
    );
    summon_append(
        g,
        img,
        f,
        n2,
        [
            0x22f, 0x236, 0x23b, 0x243, 0x248, 0x250, 0x25c, 0x262, 0x268,
        ],
    );
    for o in [0x1cu32, 0x20, 0x24, 0x28] {
        g.write::<u32>(n2 + o, 0x3f80_0000);
    }
    for (k, v) in (0u32..).zip([m, w(0x298), w(0x2a0), n1, w(0x2ac), n2]) {
        g.write::<u32>(l + 4 * k, v);
    }
    let insert = Frame {
        at: call(0x2b1),
        esp: s - 4,
        ecx: l,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &insert);
    g.write::<u8>(m + 0x148, 1);
    let i = g.read::<u32>(r + 0x24);
    let user = g.read::<u32>(i.wrapping_mul(0x98).wrapping_add(w(0x2c7)));
    let direct = Frame {
        at: call(0x2cc),
        esp: s - 4,
        ecx: user,
        ..*fr
    };
    let p = n1 + 0x1c;
    let flag = if crate::s6_misc::model_direct(g, img, &direct).eax != 0 {
        let i = g.read::<u32>(r + 0x24);
        let t = i.wrapping_mul(0x98).wrapping_add(w(0x2de));
        let c = g.read::<u32>(t + 0x54);
        // call width: 3
        g.call(slot_target(g, w(0x2ec), 0x430 / 4), 0, &[c, 0x11, p]);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        g.write::<u32>(n1 + 0x28, 0x3f80_0000);
        1
    } else {
        let i = g.read::<u32>(r + 0x24);
        let t = i.wrapping_mul(0x98).wrapping_add(w(0x32d));
        let c = g.read::<u32>(t + 0x54);
        // call width: 3
        g.call(slot_target(g, w(0x317), 0x430 / 4), 0, &[c, 0x11, p]);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        let c = g.read::<u32>(t + 0x54);
        let ground = st0_f32(g.call(slot_target(g, w(0x339), 0x30 / 4), 0, &[c]).st0);
        g.write::<f32>(p + 4, ground);
        let y = sub(g.read::<f32>(n1 + 0x20), k(g, 0x362));
        g.write::<u32>(n1 + 0x28, 0x3f80_0000);
        g.write::<f32>(n1 + 0x20, y);
        0
    };
    let owner = g.read::<u32>(w(0x37f));
    g.write::<u32>(l, part);
    g.write::<u32>(part, w(0x38d));
    for (k, v) in (1u32..).zip([w(0x399), m, owner, w(0x3a9), flag]) {
        g.write::<u32>(l + 4 * k, v);
    }
    crate::sb_nodes::particle_node(g, img, call(0x3ae), l);
    let i = g.read::<u32>(r + 0x24);
    let q = i.wrapping_mul(56).wrapping_add(w(0x3cd));
    for (k, v) in (0u32..).zip([q, w(0x3b9), m, w(0x3dd), w(0x3e5), 0]) {
        g.write::<u32>(l + 4 * k, v);
    }
    crate::sb_nodes::effect_node(g, img, call(0x3f2), l);
    let node = g.read::<u32>(l);
    g.write::<u32>(node, w(0x3fc));
    let i = g.read::<u32>(r + 0x24);
    let fl = i.wrapping_mul(56).wrapping_add(w(0x40f));
    let v = g.read::<u32>(fl) | 1;
    g.write::<u32>(fl, v);
    g.write::<u32>(i.wrapping_mul(56).wrapping_add(w(0x417)), 0);
    let st = g.read::<u32>(r + 0x18).wrapping_add(1);
    g.write::<u32>(r + 0x18, st);
    g.write::<u32>(r + 0x28, 1);
    Ret::eax(i)
}

/// [`summon_task_0399`] (1235 bytes, 1 overlay).
pub const SUMMON_TASK_0399: Sig = Sig {
    len: 1235,
    hash: 0x8c20_8295_8c66_d310,
    calls: &[
        (0x8b, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0xae, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x101, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x2b1, Callee::Sig(&crate::nodes::INSERT)),
        (0x2cc, Callee::Sig(&crate::s6_misc::MODEL_DIRECT)),
        (0x3ae, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x3f2, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x493, Callee::Sig(&crate::sb_misc::MOVE_START_2)),
    ],
    imports: &[],
};

/// `magic_0330`'s per-target summon object set up (0x100068f0, 329
/// bytes; internal: `ecx` = the object `O`, `edx` = the target `j`; the
/// group `L` at `ebp - 0x1c`): `O` its own vtable and task step, put
/// first on the object ring `G_R` (whose flag loses bit 0); the node `O +
/// 0x580` appended to `LIST` ([`summon_append`]) at the target's bone 17
/// (slot 268 into `O + 0x59c`, `w` 1.0); the effect node `Q_j`
/// ([`effect_node`](crate::sb_nodes::effect_node) `[Q_j, LIST, LIST3, O +
/// 0x580, NAME, 0]`, its vtable after) with its flag set and word
/// cleared; `O + 0xae4 = j`, the clock and stage 0. Returns `j`.
pub fn summon_object_0330<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (o, j) = (fr.ecx, fr.edx);
    let l = fr.esp.wrapping_sub(4).wrapping_sub(0x1c);
    g.write::<u32>(o + 8, o);
    g.write::<u32>(o + 4, o);
    g.write::<u32>(o, w(0x16));
    g.write::<u32>(o + 0x10, w(0x1d));
    g.write::<u32>(o + 0x14, o);
    let last = g.read::<u32>(w(0x25));
    g.write::<u32>(o + 8, last);
    g.write::<u32>(o + 4, w(0x2f));
    let last = g.read::<u32>(w(0x34));
    let n = o + 0x580;
    g.write::<u32>(last.wrapping_add(4), o);
    let fl = g.read::<u32>(w(0x43)) & !1;
    g.write::<u32>(w(0x43), fl);
    g.write::<u32>(w(0x4a), o);
    summon_append(
        g,
        img,
        f,
        n,
        [0x5d, 0x64, 0x69, 0x71, 0x76, 0x7e, 0x8a, 0xa3, 0xaa],
    );
    let t = j.wrapping_mul(0x98).wrapping_add(w(0x97));
    let c = g.read::<u32>(t + 0x54);
    // call width: 3
    g.call(slot_target(g, w(0xb4), 0x430 / 4), 0, &[c, 0x11, o + 0x59c]);
    g.write::<u32>(o + 0x5a8, 0x3f80_0000);
    let q = j.wrapping_mul(56).wrapping_add(w(0xd9));
    g.write::<u32>(o + 0x5a8, 0x3f80_0000);
    for (k, v) in (0u32..).zip([q, w(0xf0), w(0xf7), n, w(0x101), 0]) {
        g.write::<u32>(l + 4 * k, v);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x10d), l);
    let node = g.read::<u32>(l);
    g.write::<u32>(node, w(0x116));
    let v = g.read::<u32>(q + 0x2c) | 1;
    g.write::<u32>(q + 0x2c, v);
    g.write::<u32>(q + 0x30, 0);
    g.write::<u32>(o + 0xae4, j);
    g.write::<u32>(o + 0xadc, 0);
    g.write::<u32>(o + 0xae0, 0);
    Ret::eax(j)
}

/// [`summon_object_0330`] (329 bytes, 1 overlay).
pub const SUMMON_OBJECT_0330: Sig = Sig {
    len: 329,
    hash: 0x56bc_b012_928f_3202,
    calls: &[(0x10d, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};

/// `magic_0330`'s summon object step (0x10006550, 914 bytes; cdecl
/// `(task)`, the object `O = task[1]`; an 8-aligned frame, the group `L`
/// at `S + 0x10`, `S = A - 0x28`): the clock `O + 0xadc` grows by `K`;
/// once it reaches the stage's time (`TIMES[O + 0xae0]`) stage `k`: (at 2
/// slot 239 `(*G_H, the target, 0)`) the node `O + 0x43c + 0x2c k`
/// appended to `LIST` ([`summon_append`]), the summon's point copied in
/// (slot 325) and moved by three randoms (slot 307), `w` 1.0; the node
/// `O + 0x4ec + 0x2c k` appended, colour 1.0 and a random alpha; the model
/// node ([`node_insert`](crate::nodes::node_insert) `[M_k, LIST, ID,
/// A_k, PART, B_k]`, `M_k = O + 0x5ac + 0x14c k`); the particle node
/// ([`particle_node`](crate::sb_nodes::particle_node) `[P_k, PLIST, M_k,
/// *G_O, PROG, 0]`, `P_k = O + 0x20 + 0x100 k`); the next stage; after
/// four the task's node leaves its ring. Returns the stage, the particle
/// node's `eax` or the node's old next.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_step_0330<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, op: u32| g.read::<f32>(w(op));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, 0x20; push esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let l = s + 0x10;
    let clock = add(g.read::<f32>(o + 0xadc), kf(g, 0x1d));
    let k = g.read::<u32>(o + 0xae0);
    g.write::<f32>(o + 0xadc, clock);
    let time = g.read::<f32>(w(0x33).wrapping_add(4 * k));
    // comiss + jb: below or unordered waits
    if clock < time || clock.is_nan() || time.is_nan() {
        return Ret::eax(k);
    }
    let tab = w(0x57);
    if k == 2 {
        let i = g.read::<u32>(o + 0xae4);
        let target = g.read::<u32>(i.wrapping_mul(0x98).wrapping_add(w(0x52)));
        let h = g.read::<u32>(w(0x5d));
        // call width: 3
        g.call(slot_target(g, tab, 0x3bc / 4), 0, &[h, target, 0]);
    }
    let stage = |g: &G| g.read::<u32>(o + 0xae0);
    let k = stage(g);
    let a = o.wrapping_add(k.wrapping_add(0x18).wrapping_mul(0x2c));
    summon_append(
        g,
        img,
        f,
        a,
        [0x89, 0x90, 0x95, 0x9d, 0xa2, 0xaa, 0xb6, 0xbc, 0xc9],
    );
    let d = |g: &G| o.wrapping_add(stage(g).wrapping_mul(0x2c));
    let p = d(g) + 0x43c;
    g.call(slot_target(g, w(0xdf), 0x514 / 4), 0, &[p, o + 0x59c]);
    let rnd = |g: &mut G, op: u32| st0_f32(g.call(slot_target(g, w(op), 0x4cc / 4), 0, &[]).st0);
    let r = rnd(g, 0xec);
    let x = add(
        sub(mul(sub(r, kf(g, 0x109)), kf(g, 0x11a)), kf(g, 0x122)),
        g.read::<f32>(d(g) + 0x43c),
    );
    let at = d(g) + 0x43c;
    g.write::<f32>(at, x);
    let r = rnd(g, 0x139);
    let y = sub(kf(g, 0x161), mul(sub(r, kf(g, 0x153)), kf(g, 0x16c)));
    let y = add(y, g.read::<f32>(d(g) + 0x440));
    let at = d(g) + 0x440;
    g.write::<f32>(at, y);
    let r = rnd(g, 0x187);
    let z = add(
        sub(mul(sub(r, kf(g, 0x1aa)), kf(g, 0x1b2)), kf(g, 0x1ba)),
        g.read::<f32>(d(g) + 0x444),
    );
    let at = d(g) + 0x444;
    g.write::<f32>(at, z);
    let at = d(g) + 0x448;
    g.write::<u32>(at, 0x3f80_0000);
    let b = o.wrapping_add(stage(g).wrapping_add(0x1c).wrapping_mul(0x2c));
    summon_append(
        g,
        img,
        f,
        b,
        [
            0x201, 0x208, 0x20d, 0x215, 0x21a, 0x222, 0x22e, 0x234, 0x23a,
        ],
    );
    for off in [0x4ecu32, 0x4f0, 0x4f4] {
        let at = d(g) + off;
        g.write::<u32>(at, 0x3f80_0000);
    }
    let r = rnd(g, 0x27b);
    let alpha = add(mul(sub(r, kf(g, 0x295)), kf(g, 0x2a6)), kf(g, 0x2be));
    let at = d(g) + 0x4f8;
    g.write::<f32>(at, alpha);
    let k = stage(g);
    let m = o.wrapping_add(k.wrapping_mul(0x14c)) + 0x5ac;
    let a = o.wrapping_add(k.wrapping_add(0x18).wrapping_mul(0x2c));
    let b = o.wrapping_add(k.wrapping_add(0x1c).wrapping_mul(0x2c));
    for (i, v) in (0u32..).zip([m, w(0x2ae), w(0x2b6), a, w(0x2c6), b]) {
        g.write::<u32>(l + 4 * i, v);
    }
    let insert = Frame {
        at: call(0x309),
        esp: s - 4,
        ecx: l,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &insert);
    let k = stage(g);
    let part = o.wrapping_add(0x20).wrapping_add(k << 8);
    let m = o.wrapping_add(k.wrapping_mul(0x14c)) + 0x5ac;
    let owner = g.read::<u32>(w(0x330));
    for (i, v) in (0u32..).zip([part, w(0x340), m, owner, w(0x34c), 0]) {
        g.write::<u32>(l + 4 * i, v);
    }
    g.write::<u32>(part, w(0x35a));
    let eax = crate::sb_nodes::particle_node(g, img, call(0x35f), l);
    let k = stage(g).wrapping_add(1);
    g.write::<u32>(o + 0xae0, k);
    if k.cast_signed() < 4 {
        return Ret::eax(eax);
    }
    let node = g.read::<u32>(task);
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node + 8);
    g.write::<u32>(next + 8, prev);
    let prev = g.read::<u32>(node + 8);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 4, next);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    Ret::eax(next)
}

/// [`summon_step_0330`] (914 bytes, 1 overlay).
pub const SUMMON_STEP_0330: Sig = Sig {
    len: 914,
    hash: 0x381a_4c4f_75a9_e957,
    calls: &[
        (0x309, Callee::Sig(&crate::nodes::INSERT)),
        (0x35f, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// `magic_0330`'s summon task (0x10006a40, 273 bytes; cdecl `(task)`,
/// the record `R = task[1]`; the group `L` at `ebp - 0x14`): `R + 0x1c`
/// and `+0x20` grow by `K`, the counter `R + 0x28` by one, then by the
/// state `R + 0x18`: 0 the summon's nodes
/// ([`model_nodes`](crate::m0330::model_nodes) `[NODE, LIST,
/// *G_H, 0, 1]`) and `R + 0x2c` 0, on to 10; 10: state 11, the counter
/// 25 and `R + 0x24` 0; 11: past a counter of 25 the object of target
/// `R + 0x2c` ([`summon_object_0330`], `OBJS + 0xaf0 j`), the next target,
/// after the last state 10000, the counter 1; 10000: the task's node
/// leaves its ring. Other states only count. Returns the state, the
/// summon nodes' `eax`, the next index or the node's old next.
pub fn summon_task_0330<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let x = add(g.read::<f32>(r + 0x1c), g.read::<f32>(w(0x16)));
    let state = g.read::<u32>(r + 0x18);
    let n = g.read::<u32>(r + 0x28).wrapping_add(1);
    g.write::<u32>(r + 0x28, n);
    g.write::<f32>(r + 0x1c, x);
    let y = add(g.read::<f32>(r + 0x20), g.read::<f32>(w(0x2e)));
    g.write::<f32>(r + 0x20, y);
    let ebp = fr.esp.wrapping_sub(4);
    let l = ebp.wrapping_sub(0x14);
    let count = |g: &mut G| {
        let n = g.read::<u32>(r + 0x28).wrapping_add(1);
        g.write::<u32>(r + 0x28, n);
    };
    let mut eax = state;
    match state {
        0 => {
            let h = g.read::<u32>(w(0x54));
            for (i, v) in (0u32..).zip([w(0x5e), w(0x65), h, 0]) {
                g.write::<u32>(l + 4 * i, v);
            }
            g.write::<u8>(l + 0x10, 1);
            g.write::<u32>(w(0x79), w(0x7d));
            // push esi; the call's return address
            let nodes = Frame {
                at: img.rel32_target(f + 0x82),
                esp: ebp.wrapping_sub(0x1c),
                ecx: l,
                ..*fr
            };
            eax = crate::m0330::model_nodes(g, img, &nodes).eax;
            g.write::<u32>(r + 0x2c, 0);
            g.write::<u32>(r + 0x18, 0xa);
        }
        10 | 11 => {}
        0x2710 => {
            let node = g.read::<u32>(task);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node + 8);
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(node + 8);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            count(g);
            return Ret::eax(next);
        }
        _ => {
            count(g);
            return Ret::eax(state);
        }
    }
    if state != 11 {
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x28, 0x19);
        g.write::<u32>(r + 0x24, 0);
    }
    if g.read::<i32>(r + 0x28) <= 0x19 {
        count(g);
        return Ret::eax(eax);
    }
    let j = g.read::<u32>(r + 0x2c);
    // push esi: the call's return address below it
    let obj = Frame {
        at: img.rel32_target(f + 0xbd),
        esp: ebp.wrapping_sub(0x1c),
        ecx: j.wrapping_mul(0xaf0).wrapping_add(w(0xb8)),
        edx: j,
        ..*fr
    };
    summon_object_0330(g, img, &obj);
    let i = g.read::<u32>(r + 0x24).wrapping_add(1);
    g.write::<u32>(r + 0x24, i);
    let j = g.read::<u32>(r + 0x2c).wrapping_add(1);
    g.write::<u32>(r + 0x2c, j);
    if i.cast_signed() >= g.read::<i32>(w(0xcc)) {
        g.write::<u32>(r + 0x18, 0x2710);
    }
    g.write::<u32>(r + 0x28, 1);
    Ret::eax(i)
}

/// [`summon_task_0330`] (273 bytes, 1 overlay).
pub const SUMMON_TASK_0330: Sig = Sig {
    len: 273,
    hash: 0x3036_e546_57be_5ea9,
    calls: &[
        (0x82, Callee::Sig(&crate::m0330::MODEL_NODES)),
        (0xbd, Callee::Sig(&SUMMON_OBJECT_0330)),
    ],
    imports: &[],
};

/// A global node appended to the summon list as `magic_0159` inlines it,
/// every address an operand: `u16` 0xff into `+0x1a` (`flag`, the
/// instruction of a `mov word [abs], imm16`), the stores `pre` (`mov dword
/// [abs], imm32`), `+0x8` the old last (`prev`, a `mov [abs], eax`), the
/// list's first child (the store `first`) or the old last's `+0x4` (the
/// immediate `link`), then the stores `post`. `last` is the old last as
/// read, `list` the operand it is compared with.
#[allow(clippy::too_many_arguments)] // the inlined steps' operands
fn summon_append_abs<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    last: u32,
    (flag, pre, prev, anchor, first, link): (u32, &[u32], u32, u32, u32, u32),
    count: Option<u32>,
    post: &[u32],
) {
    let w = |o: u32| img.word(f + o);
    g.write::<u16>(w(flag + 3), 0xff);
    stores(g, img, f, pre);
    g.write::<u32>(w(prev), last);
    if last == w(anchor) {
        stores(g, img, f, &[first]);
    } else {
        g.write::<u32>(last.wrapping_add(4), w(link));
    }
    if let Some(c) = count {
        let n = g.read::<u16>(w(c)).wrapping_add(1);
        g.write::<u16>(w(c), n);
    }
    stores(g, img, f, post);
}

/// `magic_0159`'s summon task (0x10006750, 1475 bytes; cdecl `(task)`, the
/// record `R = task[1]`; an 8-aligned frame, the group `L` at `S + 8`, `S
/// = A - 0x20`). By the state `R + 0x18`:
/// - 0 (state 10 first) and 10: the summon's model node
///   ([`model_node_init`](crate::sa_summon::model_node_init) `[NODE, LIST,
///   *G_A, 0]`), the node `C1` appended (at slot 262 of `*G_B`, its `y`
///   slot 12's ground, `w` 1.0), `C2` appended with the facing
///   ([`facing_angle`](crate::m0159::facing_angle)), the model node `M`
///   ([`node_insert`](crate::nodes::node_insert) `[M, LIST, ID, C1, C2,
///   PART]`) and its bone ([`bone_plain`](crate::sa_summon::bone_plain),
///   the flag byte `*G_F`), `C3`/`C4` appended (`C4`'s scale slot 277 of
///   slot 215's model times `K`), the effect model `E` (`[E, LIST, M, C3,
///   ID2, C4]`), slot 211 `(*G_B, POS)`, a flag, slot 228 `(*G_A, 0x98)`;
///   then the next state, the counters;
/// - 11: from a count of 15 on, the nodes `C5`/`C6` appended (slot 237
///   `(*G_B, 0, 0, C5+0x1c)`, `C5`'s `y` less `K2`), the model node `N`
///   ([`insert_with`](crate::sa_misc::insert_with)), the particle node
///   ([`particle_from`](crate::s4_small::particle_from)) and the effect
///   ([`effect_ctor_arg`](crate::o4_ctor::effect_ctor_arg)) with its flag
///   set and word cleared; the next state;
/// - 12: from a count of 5 on the character's fade
///   ([`chr_fade`](crate::l3_common::chr_fade) `[NODE, 0.0, 8]`) and state
///   13; then as 13;
/// - 13: `C3`'s `+0x24` grows by `K3`; once slot 411 `(*G_A)` says so,
///   state 10000;
/// - 10000: slot 695 `(*G_B)`, the word 2, the character shown
///   ([`chr_show`](crate::l3_common::chr_show)), the current list node
///   unlinked ([`list_unlink_current`](crate::l3_common::list_unlink_current)),
///   the model set up ([`model_setup`](crate::sa_summon::model_setup)) and
///   the task's node off its ring.
///
/// Every exit counts `R + 0x28` and `R + 0x2c` (the states that finish
/// reset `+0x2c` to 1). Returns the last call's `eax`, the state or 0.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0159<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, 0x18; push esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    let l = s + 8;
    let state = g.read::<u32>(r + 0x18);
    let bump = |g: &mut G, o: u32| {
        let n = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, n);
    };
    let both = |g: &mut G| {
        bump(g, 0x28);
        bump(g, 0x2c);
    };
    let finish = |g: &mut G| {
        bump(g, 0x18);
        bump(g, 0x28);
        g.write::<u32>(r + 0x2c, 1);
    };
    let set_l = |g: &mut G, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(l + 4 * i, v);
        }
    };
    match state {
        0 | 10 => {
            if state == 0 {
                g.write::<u32>(r + 0x18, 0xa);
            }
            let tab = w(0xb5);
            let a = g.read::<u32>(w(0x2f));
            set_l(g, &[w(0x3b), w(0x43), a]);
            g.write::<u8>(l + 0xc, 0);
            stores(g, img, f, &[0x50]);
            crate::sa_summon::model_node_init(g, img, call(0x5b), l);
            let last = g.read::<u32>(w(0x60));
            summon_append_abs(
                g,
                img,
                f,
                last,
                (0x64, &[0x6d, 0x77, 0x81, 0x8b], 0x96, 0x9b, 0xa1, 0xb0),
                Some(0xbc),
                &[0xd5, 0xdf],
            );
            let b = g.read::<u32>(w(0xd1));
            // call width: 4
            g.call(
                slot_target(g, tab, 0x418 / 4),
                0,
                &[b, w(0xcb), w(0xc6), w(0xc1)],
            );
            let b = g.read::<u32>(w(0xf8));
            let ground = st0_f32(g.call(slot_target(g, w(0xf2), 0x30 / 4), 0, &[b]).st0);
            g.write::<f32>(w(0x103), ground);
            let last = g.read::<u32>(w(0x108));
            stores(g, img, f, &[0x10f, 0x119]);
            summon_append_abs(
                g,
                img,
                f,
                last,
                (
                    0x123,
                    &[0x12c, 0x136, 0x140, 0x14a],
                    0x155,
                    0x15a,
                    0x160,
                    0x16f,
                ),
                Some(0x176),
                &[0x17f],
            );
            // the local's start value is the ecx slot 12 leaves: 0 here,
            // slot 601 writes it (brief 2026-10-24)
            crate::m0159::facing_angle(g, img, call(0x18a), s - 4, 0, w(0x17b));
            set_l(
                g,
                &[w(0x196), w(0x19e), w(0x1a6), w(0x1ae), w(0x1b6), w(0x1be)],
            );
            let ins = Frame {
                at: call(0x1c3),
                esp: s - 4,
                ecx: l,
                ..*fr
            };
            crate::nodes::node_insert(g, img, &ins);
            let fl = g.read::<u8>(w(0x1c8));
            g.write::<u32>(l, w(0x1d4));
            g.write::<u8>(l + 5, fl);
            let bone = Frame {
                at: call(0x1dd),
                esp: s - 4,
                ecx: l,
                ..*fr
            };
            crate::sa_summon::bone_plain(g, img, &bone);
            let last = g.read::<u32>(w(0x1e2));
            summon_append_abs(
                g,
                img,
                f,
                last,
                (
                    0x1e6,
                    &[0x1ef, 0x1f9, 0x203, 0x20d],
                    0x218,
                    0x21d,
                    0x223,
                    0x232,
                ),
                None,
                &[],
            );
            let count = g.read::<u16>(w(0x238));
            let c3 = w(0x23d);
            stores(g, img, f, &[0x243, 0x24d, 0x257, 0x261, 0x26b]);
            g.write::<u16>(w(0x278), 0xff);
            stores(g, img, f, &[0x27e, 0x288, 0x292, 0x29c, 0x2a6]);
            if c3 == w(0x2b2) {
                stores(g, img, f, &[0x2b8]);
            } else {
                stores(g, img, f, &[0x2c4]);
            }
            let a = g.read::<u32>(w(0x2d0));
            g.write::<u16>(w(0x2d8), count.wrapping_add(2));
            stores(g, img, f, &[0x2e1, 0x2eb, 0x2f5, 0x2ff, 0x309]);
            let model = g.call(slot_target(g, w(0x2dd), 0x35c / 4), 0, &[a]).eax;
            let r277 = st0_f32(g.call(slot_target(g, w(0x320), 0x454 / 4), 0, &[model]).st0);
            g.write::<f32>(w(0x349), mul(r277, kf(g, 0x33a)));
            set_l(
                g,
                &[w(0x351), w(0x359), w(0x361), w(0x369), w(0x371), w(0x379)],
            );
            let ins = Frame {
                at: call(0x37e),
                esp: s - 4,
                ecx: l,
                ..*fr
            };
            crate::nodes::node_insert(g, img, &ins);
            let b = g.read::<u32>(w(0x38e));
            g.call(slot_target(g, w(0x383), 0x34c / 4), 0, &[b, w(0x388)]);
            let fl = g.read::<u16>(w(0x3a2)) | 0x10;
            g.write::<u16>(w(0x3a2), fl);
            let a = g.read::<u32>(w(0x3ae));
            stores(g, img, f, &[0x3b2]);
            let eax = g
                .call(slot_target(g, w(0x39b), 0x390 / 4), 0, &[a, 0x98])
                .eax;
            finish(g);
            Ret::eax(eax)
        }
        11 => {
            if g.read::<i32>(r + 0x2c) < 0xf {
                both(g);
                return Ret::eax(0);
            }
            crate::sb_misc::node_append(g, img, call(0x478), w(0x473), w(0x46e), 0);
            let b = g.read::<u32>(w(0x492));
            // call width: 4
            g.call(slot_target(g, w(0x47d), 0x3b4 / 4), 0, &[b, 0, 0, w(0x482)]);
            let y = sub(kf(g, 0x49c), kf(g, 0x4a4));
            stores(g, img, f, &[0x4b2]);
            g.write::<f32>(w(0x4c0), y);
            crate::sb_misc::node_append(g, img, call(0x4c5), w(0x4ae), w(0x4a9), 0);
            stores(g, img, f, &[0x4de, 0x4e8, 0x4f2, 0x4fc]);
            // slot 237's four words stay; push ecx (still the node just
            // appended), C, B, A: the call's return address at S - 0x24
            let at = s.wrapping_sub(0x24);
            for (i, v) in (1u32..).zip([w(0x4d5), w(0x4d0), w(0x4cb), w(0x4ae)]) {
                g.write::<u32>(at + 4 * i, v);
            }
            let ins = Frame {
                at: call(0x507),
                esp: at,
                ecx: w(0x4da),
                edx: w(0x4a9),
                ..*fr
            };
            crate::sa_misc::insert_with(g, img, &ins);
            g.write::<u8>(w(0x51a), 1);
            // add esp, 0x14; push X: the return address at S - 0x14
            let esp = s.wrapping_sub(0x14);
            g.write::<u32>(esp + 4, w(0x514));
            let pf = Frame {
                at: call(0x520),
                esp,
                ecx: w(0x50f),
                ..*fr
            };
            crate::s4_small::particle_from(g, img, &pf);
            g.write::<u32>(esp + 4, w(0x528));
            let ec = Frame {
                at: call(0x52d),
                esp,
                ..*fr
            };
            let eax = crate::o4_ctor::effect_ctor_arg(g, img, &ec).eax;
            let fl = g.read::<u32>(w(0x533)) | 1;
            g.write::<u32>(w(0x533), fl);
            stores(g, img, f, &[0x538]);
            finish(g);
            Ret::eax(eax)
        }
        12 | 13 => {
            if state == 12 && g.read::<i32>(r + 0x2c) >= 5 {
                set_l(g, &[w(0x402), 0, 8]);
                crate::l3_common::chr_fade(g, img, call(0x413), l);
                bump(g, 0x18);
            }
            let x = add(kf(g, 0x41e), kf(g, 0x426));
            let a = g.read::<u32>(w(0x431));
            g.write::<f32>(w(0x439), x);
            let eax = g.call(slot_target(g, w(0x42b), 0x66c / 4), 0, &[a]).eax;
            both(g);
            if eax != 0 {
                g.write::<u32>(r + 0x18, 0x2710);
            }
            Ret::eax(eax)
        }
        0x2710 => {
            let b = g.read::<u32>(w(0x569));
            g.call(slot_target(g, w(0x563), 0xadc / 4), 0, &[b]);
            stores(g, img, f, &[0x578]);
            let e = crate::l3_common::chr_show(g, img, call(0x583));
            crate::l3_common::list_unlink_current(g, img, call(0x588), e);
            g.write::<u32>(l, w(0x594));
            let ms = Frame {
                at: call(0x599),
                esp: s - 4,
                ecx: l,
                ..*fr
            };
            crate::sa_summon::model_setup(g, img, &ms);
            let node = g.read::<u32>(task);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node + 8);
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(node + 8);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            both(g);
            Ret::eax(next)
        }
        _ => {
            both(g);
            Ret::eax(state)
        }
    }
}

/// [`summon_task_0159`] (1475 bytes, 1 overlay).
pub const SUMMON_TASK_0159: Sig = Sig {
    len: 1475,
    hash: 0x1250_25a0_bdb9_7a3b,
    calls: &[
        (0x5b, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x18a, Callee::Sig(&crate::m0159::FACING_ANGLE)),
        (0x1c3, Callee::Sig(&crate::nodes::INSERT)),
        (0x1dd, Callee::Sig(&crate::sa_summon::BONE)),
        (0x37e, Callee::Sig(&crate::nodes::INSERT)),
        (0x413, Callee::Sig(&crate::l3_common::CHR_FADE)),
        (0x478, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x4c5, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x507, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x520, Callee::Sig(&crate::s4_small::PARTICLE_FROM)),
        (
            0x52d,
            Callee::Sig(&crate::o4_ctor::EFFECT_CTOR_ARG_0159_6700),
        ),
        (0x583, Callee::Sig(&crate::l3_common::CHR_SHOW)),
        (0x588, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x599, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// `dst = K - src` (`movaps xmm0, xmm1; subss`) for each `(src, dst)`
/// operand pair, `K` the float given.
fn k_minus<G: Game>(g: &mut G, img: &Image, f: u32, k: f32, pairs: &[(u32, u32)]) {
    for &(src, dst) in pairs {
        let v = sub(k, g.read::<f32>(img.word(f + src)));
        g.write::<f32>(img.word(f + dst), v);
    }
}

/// `magic_0379`'s summon task (0x10006af0, 1578 bytes; cdecl `(task)`,
/// the record `R = task[1]`; an 8-aligned frame, the group `L` at `S +
/// 8`, `S = A - 0x20`). By the state `R + 0x18` (states 11 and 23 go on
/// to the state they set in the same call):
/// - 0: the model nodes ([`model_nodes`](crate::s3_summon::model_nodes)
///   `[NODE, LIST, *G_A, 0, 1]`), the static child
///   ([`static_child`](crate::sa_misc::static_child)), a task node
///   ([`task_node_tail`](crate::o4_single3::task_node_tail)) with `*G_D`'s
///   low byte in two places, the colour node
///   ([`colour_node`](crate::s3_misc::colour_node)); state 10;
/// - 10: two nodes appended ([`node_append`](crate::sb_misc::node_append);
///   slot 366 `(*G_A, P)` between them unless the flag byte's bit 0), the
///   model node ([`node_insert`](crate::nodes::node_insert)), the particle
///   node ([`particle_node`](crate::sb_nodes::particle_node)), slot 228
///   `(*G_A, 0x40)`, the effect node
///   ([`effect_node`](crate::sb_nodes::effect_node)) and the camera
///   block's differences `K - x`; state 11;
/// - 11: when slot 411 `(*G_A)` says so, state 20;
/// - 20: slot 228 `(*G_A, 0x18)`, state 21;
/// - 21: when slot 411 says so: a node
///   ([`node_init`](crate::m0003::node_init)), a node under the static
///   parent ([`node_under`](crate::m0379::node_under)), the model node
///   ([`insert_with`](crate::sa_misc::insert_with)), the particle node
///   ([`particle_node_with`](crate::s3_nodes::particle_node_with)), slot
///   228 `(*G_A, 0x19)`, the effect node
///   ([`effect_node_b`](crate::s4_fam446::effect_node_b)), the camera
///   block again (1.0 scales, flags); state 22, the count 1;
/// - 22: from a count of 30 the character's fade
///   ([`chr_fade`](crate::l3_common::chr_fade) `[NODE2, 0.0, 20]`), state
///   23, the count 1;
/// - 23: from a count of 30, state 10000;
/// - 10000: the task's node leaves its ring.
///
/// Every exit counts `R + 0x2c`. Returns the last call's `eax`, a sub's
/// left-over of the state, 0 or the node's old next.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0379<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_misc::node_append;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, 0x18; push esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    let l = s + 8;
    let bump = |g: &mut G, o: u32| {
        let n = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, n);
    };
    let set_l = |g: &mut G, words: &[u32]| {
        for (i, &v) in (0u32..).zip(words) {
            g.write::<u32>(l + 4 * i, v);
        }
    };
    let actor = |g: &G, o: u32| g.read::<u32>(w(o));
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            0 => {
                let a = actor(g, 0x2df);
                set_l(g, &[w(0x2eb), w(0x2f3), a, 0]);
                g.write::<u8>(l + 0x10, 1);
                stores(g, img, f, &[0x308]);
                let mn = Frame {
                    at: call(0x313),
                    esp: s - 4,
                    ecx: l,
                    ..*fr
                };
                crate::s3_summon::model_nodes(g, img, &mn);
                crate::sa_misc::static_child(g, img, call(0x31d), w(0x318));
                let dl = g.read::<u32>(w(0x323)).to_le_bytes()[0];
                g.write::<u8>(w(0x32e), dl);
                stores(g, img, f, &[0x332]);
                let tn = Frame {
                    at: call(0x33d),
                    esp: s - 4,
                    ecx: w(0x328),
                    ..*fr
                };
                crate::o4_single3::task_node_tail(g, img, &tn);
                g.write::<u8>(w(0x347), dl);
                set_l(g, &[w(0x34f), w(0x357)]);
                stores(g, img, f, &[0x35b]);
                let cn = Frame {
                    at: call(0x366),
                    esp: s - 4,
                    ecx: l,
                    ..*fr
                };
                let eax = crate::s3_misc::colour_node(g, img, &cn);
                bump(g, 0x2c);
                g.write::<u32>(r + 0x18, 0xa);
                return Ret::eax(eax);
            }
            10 => {
                node_append(g, img, call(0x88), w(0x83), w(0x7e), 0);
                if g.read::<u8>(w(0x8e)) & 1 == 0 {
                    let a = actor(g, 0xa1);
                    g.call(slot_target(g, w(0x96), 0x5b8 / 4), 0, &[a, w(0x9b)]);
                }
                node_append(g, img, call(0xbb), w(0xb6), w(0xb1), 0);
                stores(g, img, f, &[0xc7, 0xd1, 0xdb, 0xe5]);
                set_l(g, &[w(0xf3), w(0xb1), w(0xff), w(0x107), w(0x10f), w(0xb6)]);
                let ins = Frame {
                    at: call(0x114),
                    esp: s - 4,
                    ecx: l,
                    ..*fr
                };
                crate::nodes::node_insert(g, img, &ins);
                let owner = g.read::<u32>(w(0x119));
                g.write::<u8>(w(0x123), 1);
                set_l(g, &[w(0x12c), w(0x134), w(0x13c), owner, w(0x148), 0]);
                stores(g, img, f, &[0x154]);
                crate::sb_nodes::particle_node(g, img, call(0x15f), l);
                let a = actor(g, 0x16c);
                g.call(slot_target(g, w(0x164), 0x390 / 4), 0, &[a, 0x40]);
                set_l(g, &[w(0x183), w(0x18b), w(0x193), w(0x19b), w(0x1a3), 0]);
                crate::sb_nodes::effect_node(g, img, call(0x1b0), l);
                let k = kf(g, 0x1b8);
                let node = g.read::<u32>(l);
                g.write::<u32>(node, w(0x1c5));
                let v = sub(k, kf(g, 0x1cd));
                let fl = g.read::<u32>(w(0x1d3)) | 1;
                g.write::<u32>(w(0x1d3), fl);
                let fl = g.read::<u32>(w(0x1da)) & !1;
                g.write::<u32>(w(0x1da), fl);
                stores(g, img, f, &[0x1df, 0x1e9]);
                g.write::<f32>(w(0x1f7), v);
                let v = sub(k, kf(g, 0x202));
                stores(g, img, f, &[0x206, 0x210, 0x21a, 0x224]);
                g.write::<f32>(w(0x232), v);
                let v = sub(k, kf(g, 0x23d));
                stores(g, img, f, &[0x241, 0x24b, 0x255, 0x25f]);
                g.write::<f32>(w(0x26d), v);
                k_minus(g, img, f, k, &[(0x278, 0x280), (0x28b, 0x293)]);
                let x = sub(k, kf(g, 0x29e));
                let y = sub(k, kf(g, 0x2a6));
                g.write::<f32>(w(0x2ae), x);
                let z = sub(kf(g, 0x2b6), kf(g, 0x2be));
                g.write::<f32>(w(0x2c6), y);
                g.write::<f32>(w(0x2ce), z);
                bump(g, 0x18);
                bump(g, 0x2c);
                return Ret::eax(node);
            }
            11 | 21 => {
                let op = if state == 11 {
                    (0x35, 0x3b)
                } else {
                    (0x3e5, 0x3eb)
                };
                let a = actor(g, op.1);
                let yes = g.call(slot_target(g, w(op.0), 0x66c / 4), 0, &[a]).eax;
                if yes == 0 {
                    bump(g, 0x2c);
                    return Ret::eax(0);
                }
                if state == 11 {
                    g.write::<u32>(r + 0x18, 0x14);
                    continue;
                }
                set_l(g, &[w(0x40a), w(0x412), w(0x41a)]);
                crate::m0003::node_init(g, img, call(0x41f), l);
                // push X: the return address at S - 8
                let esp = s.wrapping_sub(8);
                g.write::<u32>(esp + 4, w(0x424));
                g.write::<u8>(w(0x42f), 6);
                stores(g, img, f, &[0x434]);
                let nu = Frame {
                    at: call(0x43f),
                    esp,
                    ecx: w(0x429),
                    ..*fr
                };
                crate::m0379::node_under(g, img, &nu);
                // push ecx (node_under keeps it), C, B, A
                let esp = s.wrapping_sub(0x14);
                for (i, v) in (1u32..).zip([w(0x457), w(0x452), w(0x44d), w(0x429)]) {
                    g.write::<u32>(esp + 4 * i, v);
                }
                stores(g, img, f, &[0x460, 0x46a]);
                let iw = Frame {
                    at: call(0x475),
                    esp,
                    ecx: w(0x45c),
                    edx: w(0x447),
                    ..*fr
                };
                crate::sa_misc::insert_with(g, img, &iw);
                // push 1; sub esp, 8; push M
                g.write::<u32>(esp + 0x10, 1);
                g.write::<u32>(esp + 4, w(0x48e));
                g.write::<u8>(w(0x488), 1);
                let pn = Frame {
                    at: call(0x493),
                    esp,
                    ecx: w(0x47d),
                    ..*fr
                };
                crate::s3_nodes::particle_node_with(g, img, &pn);
                let a = actor(g, 0x4a0);
                g.call(slot_target(g, w(0x498), 0x390 / 4), 0, &[a, 0x19]);
                // add esp, 0x18; push 1; sub esp, 8; push M
                g.write::<u32>(esp + 0x10, 1);
                g.write::<u32>(esp + 4, w(0x4ba));
                let en = Frame {
                    at: call(0x4bf),
                    esp,
                    ecx: w(0x4b0),
                    ..*fr
                };
                crate::s4_fam446::effect_node_b(g, img, &en);
                let k = kf(g, 0x4c7);
                let v = sub(k, kf(g, 0x4d2));
                let flags = (g.read::<u32>(w(0x4d7)) & !1) | 4;
                let fl = g.read::<u32>(w(0x4dd)) | 1;
                g.write::<u32>(w(0x4dd), fl);
                g.write::<f32>(w(0x4ec), v);
                let v = sub(k, kf(g, 0x4f7));
                stores(g, img, f, &[0x4fb, 0x505, 0x50f, 0x519]);
                g.write::<f32>(w(0x527), v);
                let v = sub(k, kf(g, 0x532));
                stores(g, img, f, &[0x536, 0x540, 0x54a, 0x554]);
                g.write::<f32>(w(0x562), v);
                let v = sub(k, kf(g, 0x56d));
                stores(g, img, f, &[0x571, 0x57b]);
                g.write::<u32>(w(0x586), flags);
                stores(g, img, f, &[0x58a]);
                g.write::<f32>(w(0x598), v);
                k_minus(g, img, f, k, &[(0x5a3, 0x5ae)]);
                let x = sub(k, kf(g, 0x5b9));
                let y = sub(k, kf(g, 0x5c1));
                g.write::<f32>(w(0x5c9), x);
                let z = sub(kf(g, 0x5d1), kf(g, 0x5d9));
                g.write::<f32>(w(0x5e1), y);
                g.write::<f32>(w(0x5e9), z);
                bump(g, 0x18);
                g.write::<u32>(r + 0x2c, 1);
                return Ret::eax(flags);
            }
            20 => {
                let a = actor(g, 0x383);
                let eax = g
                    .call(slot_target(g, w(0x37b), 0x390 / 4), 0, &[a, 0x18])
                    .eax;
                bump(g, 0x18);
                bump(g, 0x2c);
                return Ret::eax(eax);
            }
            22 => {
                if g.read::<i32>(r + 0x2c) < 0x1e {
                    bump(g, 0x2c);
                    return Ret::eax(0);
                }
                set_l(g, &[w(0x3bc), 0, 0x14]);
                let eax = crate::l3_common::chr_fade(g, img, call(0x3cd), l);
                bump(g, 0x18);
                g.write::<u32>(r + 0x2c, 1);
                return Ret::eax(eax);
            }
            23 => {
                if g.read::<i32>(r + 0x2c) < 0x1e {
                    bump(g, 0x2c);
                    return Ret::eax(state);
                }
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x2710 => {
                let node = g.read::<u32>(task);
                let next = g.read::<u32>(node + 4);
                let prev = g.read::<u32>(node + 8);
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(node + 8);
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(prev + 4, next);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                bump(g, 0x2c);
                return Ret::eax(next);
            }
            _ => {
                bump(g, 0x2c);
                // above 23 the state; below 20 `sub eax, 10; dec eax` left
                // it 11 less (20..23 have their cases)
                let eax = if state > 0x17 {
                    state
                } else {
                    state.wrapping_sub(0xb)
                };
                return Ret::eax(eax);
            }
        }
    }
}

/// [`summon_task_0379`] (1578 bytes, 1 overlay).
pub const SUMMON_TASK_0379: Sig = Sig {
    len: 1578,
    hash: 0xbc75_ab44_e113_0e72,
    calls: &[
        (0x88, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xbb, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x114, Callee::Sig(&crate::nodes::INSERT)),
        (0x15f, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x1b0, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x313, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x31d, Callee::Sig(&crate::sa_misc::STATIC_CHILD)),
        (
            0x33d,
            Callee::Sig(&crate::o4_single3::TASK_NODE_TAIL_0379_6910),
        ),
        (0x366, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x3cd, Callee::Sig(&crate::l3_common::CHR_FADE)),
        (0x41f, Callee::Sig(&crate::m0003::NODE_INIT)),
        (0x43f, Callee::Sig(&crate::m0379::NODE_UNDER)),
        (0x475, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x493, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x4bf, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_B)),
    ],
    imports: &[],
};

/// The actor `i` of `magic_0172`'s loops: `*G_A` for `i = -1` (operand
/// `a`), else `CHRS[SEL[i]]` (operands `sel`, `chrs`).
fn actor_0172<G: Game>(g: &G, img: &Image, f: u32, i: u32, (a, sel, chrs): (u32, u32, u32)) -> u32 {
    let w = |o: u32| img.word(f + o);
    if i == 0xffff_ffff {
        g.read::<u32>(w(a))
    } else {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(i)));
        g.read::<u32>(w(chrs).wrapping_add(4 * b))
    }
}

/// `magic_0172`'s start-up (0x10001340, 945 bytes; internal, no
/// arguments; an 8-aligned /GS frame, `S = A - 0x48`, the point `P` at
/// `S + 0x34`, the caster's point `Q` at `S + 0x14`, `D` at `S + 0x24`):
/// slot 719 `(NAME, 8, 0)` into `G_P` (two words first), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the object `O`
/// ([`start_object_at`](crate::s4_misc::start_object_at) `(1, 0x10, -,
/// 0)`) at the caster `*G_A` (slot 262 into three globals, slot 268's bone
/// 6 height, `w` 1.0) with its `+0x68` scaled by `K`; the record words;
/// `Q` = the caster's point with its ground (slots 262, 12), `w` 1.0; for
/// each target `i` its record `R_i = REC + 0x160 i`: the target's point
/// (slot 262) with its ground, `w` 1.0, `Q`, and `|Q - R_i|` in the
/// `x`/`z` plane (slot 234, `_libm_sse2_sqrt_precise`) and a cleared word;
/// slot 413 or 215 (by `*G_F`) of `*G_N`, the record's words and flag
/// halfword; a node of the free ring
/// ([`ring_take`](crate::r15::ring_take); an empty ring writes through
/// address 0 as the original does) last on the task list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0172<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x48);
    let (p, q, d) = (s + 0x34, s + 0x14, s + 0x24);
    let tab = w(0x15);
    stores(g, img, f, &[0x2b, 0x35]);
    // call width: 3
    let part = g
        .call(slot_target(g, tab, 0xb3c / 4), 0, &[w(0x27), 8, 0])
        .eax;
    g.write::<u32>(w(0x45), part);
    crate::l5_objhost::part_init(g, img, call(0x4a));
    let obj = crate::s4_misc::start_object_at(g, img, call(0x66), w(0x5a), kf(g, 0x52), 1, 0x10, 0);
    let a = g.read::<u32>(w(0x72));
    // call width: 4
    g.call(
        slot_target(g, w(0x6c), 0x418 / 4),
        0,
        &[a, w(0x87), w(0x82), w(0x7d)],
    );
    // call width: 3
    g.call(slot_target(g, w(0x96), 0x430 / 4), 0, &[a, 6, p]);
    let y = g.read::<f32>(p + 4);
    g.write::<f32>(w(0xaf), y);
    stores(g, img, f, &[0xb3]);
    let sc = mul(g.read::<f32>(obj + 0x68), kf(g, 0xc6));
    g.write::<f32>(obj + 0x68, sc);
    let a = g.read::<u32>(w(0xd6));
    g.write::<u16>(w(0xdc), 0);
    stores(
        g,
        img,
        f,
        &[
            0xf4, 0xfe, 0x108, 0x112, 0x11c, 0x126, 0x130, 0x13a, 0x144, 0x14e,
        ],
    );
    // call width: 4
    g.call(slot_target(g, w(0xf0), 0x418 / 4), 0, &[a, q, q + 4, q + 8]);
    let ground = st0_f32(g.call(slot_target(g, w(0x162), 0x30 / 4), 0, &[a]).st0);
    g.write::<f32>(q + 4, ground);
    g.write::<u32>(q + 0xc, 0x3f80_0000);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(if i == 0 { 0x17f } else { 0x26d })) {
        let rec = w(0x18a).wrapping_add(0x160 * i);
        let actor = actor_0172(g, img, f, i, (0x199, 0x1a0, 0x1ab));
        // call width: 4
        g.call(
            slot_target(g, w(0x1b4), 0x418 / 4),
            0,
            &[actor, rec - 4, rec, rec + 4],
        );
        let ground = st0_f32(g.call(slot_target(g, w(0x1c7), 0x30 / 4), 0, &[actor]).st0);
        g.write::<f32>(rec, ground);
        g.write::<u32>(rec + 8, 0x3f80_0000);
        for k in 0..4 {
            let v = g.read::<u32>(q + 4 * k);
            g.write::<u32>(rec + 0xc + 4 * k, v);
        }
        // call width: 3
        g.call(slot_target(g, w(0x209), 0x3a8 / 4), 0, &[d, q, rec - 4]);
        let (dx, dz) = (g.read::<f32>(d), g.read::<f32>(d + 8));
        let sq = add(mul(dz, dz), mul(dx, dx));
        #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
        let dist = f64::from(sq).sqrt() as f32;
        g.write::<f32>(rec + 0x1c, dist);
        g.write::<u32>(rec + 0x20, 0);
        i += 1;
    }
    let flag = g.read::<u32>(w(0x279));
    let n = g.read::<u32>(w(0x27f));
    g.write::<u32>(w(0x284), n);
    stores(g, img, f, &[0x28e, 0x298]);
    let slot = if flag == 0 { 0x35c } else { 0x674 };
    let v = g.call(slot_target(g, w(0x28a), slot / 4), 0, &[n]).eax;
    let flag = g.read::<u32>(w(0x2b9));
    g.write::<u32>(w(0x2c4), v);
    stores(g, img, f, &[0x2c8, 0x2d2]);
    g.write::<u32>(w(0x2de), 1);
    if flag == 0 {
        g.write::<u16>(w(0x2e8), 0);
    } else {
        g.write::<u16>(w(0x2f1), 1);
    }
    let count = g.read::<u16>(w(0x2f8));
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x307));
        crate::r15::ring_take(g, img, f, node, count, (0x324, 0x32c, 0x35d, 0x368));
        node
    };
    let head = g.read::<u32>(w(0x36e));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x390), node);
    } else {
        let tail = g.read::<u32>(w(0x37d));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x3a2), node);
    Ret::eax(0)
}

/// [`start_up_0172`] (945 bytes, 1 overlay).
pub const START_UP_0172: Sig = Sig {
    len: 945,
    hash: 0xde1c_a8d0_10ab_380d,
    calls: &[
        (0x4a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x66, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x244, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x3a9, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0172`'s throws (0x10001740, 474 bytes; internal, no arguments;
/// an 8-aligned /GS frame, `S = A - 0x40`, `V` at `S + 0xc`, `D` at `S +
/// 0x1c`, `P` at `S + 0x2c`): once `*G_T` is 1 or more, for each target
/// `i` whose record `R_i = REC + 0x160 i` has not thrown (`+0x80`): the
/// matrix `M` set to its constants, `V = (0, 0, -r, 1)` (`r` = `R_i +
/// 0x7c`) through `M` (slot 229) plus `R_i + 0x6c` (slot 248), and when
/// `r² K` is past `V`'s distance to `R_i + 0x5c` in the `x`/`z` plane
/// (slot 234, `_libm_sse2_sqrt_precise`): the object `R_i - 0xd4`
/// ([`start_object_at`](crate::s4_misc::start_object_at) `(0, 0, -,
/// CB)`) at the target (slot 262, slot 268's bone 6 height, `w` 1.0), its
/// selector byte into `+0x4c`, thrown. Returns 0 (no caller reads it).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn throws_0172<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = (esp.wrapping_sub(4) & !7).wrapping_sub(0x40);
    let (v, dv, p) = (s + 0xc, s + 0x1c, s + 0x2c);
    if g.read::<i32>(w(0x16)) < 1 {
        return 0;
    }
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(if i == 0 { 0x27 } else { 0x1bf })) {
        let rec = w(0x32).wrapping_add(0x160 * i);
        if g.read::<u32>(rec + 0x80) != 1 {
            for k in 0..4 {
                let x = g.read::<u32>(w(0x50).wrapping_add(4 * k));
                g.write::<u32>(w(0x5c).wrapping_add(4 * k), x);
            }
            let r = g.read::<u32>(rec + 0x7c) ^ g.read::<u32>(w(0x68));
            for (k, x) in (0u32..).zip([0, 0, r, 0x3f80_0000]) {
                g.write::<u32>(v + 4 * k, x);
            }
            // call width: 3
            g.call(slot_target(g, w(0x73), 0x394 / 4), 0, &[v, w(0x6d), v]);
            // call width: 3
            g.call(slot_target(g, w(0xa8), 0x3e0 / 4), 0, &[v, v, rec + 0x6c]);
            // call width: 3
            g.call(slot_target(g, w(0xc3), 0x3a8 / 4), 0, &[dv, v, rec + 0x5c]);
            let (dx, dz) = (g.read::<f32>(dv), g.read::<f32>(dv + 8));
            let sq = add(mul(dz, dz), mul(dx, dx));
            let rr = g.read::<f32>(rec + 0x7c);
            #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
            let dist = f64::from(sq).sqrt() as f32;
            let reach = mul(mul(rr, rr), kf(g, 0x103));
            if reach > dist {
                crate::s4_misc::start_object_at(
                    g,
                    img,
                    img.rel32_target(f + 0x129),
                    rec.wrapping_sub(0xd4),
                    kf(g, 0x114),
                    0,
                    0,
                    w(0x119),
                );
                let actor = actor_0172(g, img, f, i, (0x137, 0x13e, 0x149));
                // call width: 4
                g.call(
                    slot_target(g, w(0x157), 0x418 / 4),
                    0,
                    &[actor, rec - 4, rec, rec + 4],
                );
                // call width: 3
                g.call(slot_target(g, w(0x16e), 0x430 / 4), 0, &[actor, 6, p]);
                let y = g.read::<f32>(p + 4);
                g.write::<f32>(rec, y);
                g.write::<u32>(rec + 8, 0x3f80_0000);
                let b = if i == 0xffff_ffff {
                    g.read::<u32>(w(0x19a))
                } else {
                    u32::from(g.read::<u8>(g.read::<u32>(w(0x1a1)).wrapping_add(i)))
                };
                g.write::<u32>(rec + 0x4c, b);
                g.write::<u32>(rec + 0x80, 1);
            }
        }
        i += 1;
    }
    0
}

/// [`throws_0172`] (474 bytes, 1 overlay).
pub const THROWS_0172: Sig = Sig {
    len: 474,
    hash: 0x741b_5dd0_7580_383d,
    calls: &[
        (0xee, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x129, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        // __security_check_cookie
        (0x1d2, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0172`'s frame (0x10001700, 53 bytes; cdecl `(task)`): the
/// objects run ([`run_objects`](crate::l5_objhost::run_objects)), the
/// throws ([`throws_0172`]), the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)); when both the motion
/// and the objects are done (1), the task's `+0x14` = 3 and its owner's
/// step `+0x10` the next function. Returns the motion step's `eax`.
pub fn frame_0172<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; push esi; call
    let esp = fr.esp.wrapping_sub(0xc);
    let objs = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x5), esp);
    throws_0172(g, img, img.rel32_target(f + 0xc), esp);
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x11));
    if m != 1 || objs != m {
        return Ret::eax(m);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let owner = g.read::<u32>(task + 8);
    g.write::<u32>(owner + 0x10, img.word(f + 0x2e));
    Ret::eax(owner)
}

/// [`frame_0172`] (53 bytes, 1 overlay).
pub const FRAME_0172: Sig = Sig {
    len: 53,
    hash: 0x6fc0_a5ea_eb49_8073,
    calls: &[
        (0x5, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0xc, Callee::Sig(&THROWS_0172)),
        (0x11, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
    ],
    imports: &[],
};

/// `magic_0200`'s start-up (0x10001340, 1274 bytes; internal, no
/// arguments; an 8-aligned /GS frame, `S = A - 0x90`, the point `X` at
/// `S+0x18`, `Y` at `S+0x28`, `P` at `S+0x38`, the matrix `M` at
/// `S+0x48`): slot 719 `(NAME, 8, 0)` into `G_P` (two words first), the part
/// set up ([`part_init`](crate::l5_objhost::part_init)); `X` = the first
/// actor's point (slot 262) at its bone 6 height (slot 268), `w` 1.0; four
/// objects `R_i - 0xd4` (`R_i = REC + 0x130 i`;
/// [`start_object_first`](crate::o6_fam_object::start_object_first) `(-,
/// KIND[i], -, CB_i)`, the last with the other callback) at `X` with three
/// random angles (slot 314 `(2 PI)`), random offsets (`4, 3, 4`) and the
/// pick of 0 ([`pick`](crate::m0074::pick)); `Y` = the caster's point
/// (slot 262; slot 12 asked, its answer dropped), `X.y` 0, the yaw `K -
/// atan2(X.z - Y.z, X.x - Y.x)` (`_CIatan2`) wrapped once; `X` moved by
/// `(0, 0, 30)` turned by it (slots 235, 328, 329, 229, 248), `w` 1.0; the
/// flight to `X` ([`flight_task`](crate::s5_misc::flight_task) `(X, 0.7,
/// 0.99, 30)`), two words, the task
/// ([`task_start`](crate::s3_misc::task_start)); slot 413 or 215 (by `*G_F`)
/// of `*G_N` into a record with its words and flag halfword; a node of the
/// free ring ([`ring_take`](crate::r15::ring_take), kind 2; an empty ring
/// writes through address 0 as the original does) last on the task list.
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0200<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x90);
    let (x, y, p, m) = (s + 0x18, s + 0x28, s + 0x38, s + 0x48);
    let tab = w(0x1b);
    stores(g, img, f, &[0x31, 0x3b]);
    // call width: 3
    let part = g
        .call(slot_target(g, tab, 0xb3c / 4), 0, &[w(0x2d), 8, 0])
        .eax;
    g.write::<u32>(w(0x4b), part);
    crate::l5_objhost::part_init(g, img, call(0x50));
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x55))));
    let a = g.read::<u32>(w(0x5f).wrapping_add(4 * b));
    // call width: 4
    g.call(slot_target(g, w(0x73), 0x418 / 4), 0, &[a, x, x + 4, x + 8]);
    // call width: 3
    g.call(slot_target(g, w(0x86), 0x430 / 4), 0, &[a, 6, p]);
    let py = g.read::<u32>(p + 4);
    g.write::<u32>(x + 4, py);
    g.write::<u32>(x + 0xc, 0x3f80_0000);
    for i in 0..4u32 {
        let rec = w(0xaf).wrapping_add(0x130 * i);
        let cb = if i == 3 { w(0xb4) } else { w(0xc9) };
        let kind = g.read::<u32>(w(0xd8).wrapping_add(4 * i));
        // push cb, -, kind, -: the return address at S - 0x14
        let esp = s.wrapping_sub(0x14);
        for (k, v) in (1u32..).zip([0, kind, 0, cb]) {
            g.write::<u32>(esp + 4 * k, v);
        }
        let so = Frame {
            at: call(0xe4),
            esp,
            edx: rec.wrapping_sub(0xd4),
            xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0xc4)))],
            ..*fr
        };
        crate::o6_fam_object::start_object_first(g, img, so.at, &so);
        for k in 0..4 {
            let v = g.read::<u32>(x + 4 * k);
            g.write::<u32>(rec.wrapping_sub(4) + 4 * k, v);
        }
        let tab = w(0xef);
        for o in [0x14u32, 0x10, 0xc] {
            let r = random(g, tab, f32::from_bits(0x40c9_0fdb));
            g.write::<f32>(rec.wrapping_sub(o), r);
        }
        let r = random(g, tab, 4.0);
        let v = add(g.read::<f32>(rec + 0xc), r);
        g.write::<f32>(rec + 0xc, v);
        let r = random(g, tab, 3.0);
        let v = add(g.read::<f32>(rec + 0x10), r);
        g.write::<f32>(rec + 0x10, v);
        let r = random(g, tab, 4.0);
        let v = add(r, g.read::<f32>(rec + 0x14));
        g.write::<f32>(rec + 0x14, v);
        let pick = crate::m0074::pick(g, img, call(0x1d9), 0);
        g.write::<u32>(rec + 0x4c, pick);
    }
    let c = g.read::<u32>(w(0x1f2));
    g.write::<u32>(x + 4, 0);
    // call width: 4
    g.call(
        slot_target(g, w(0x206), 0x418 / 4),
        0,
        &[c, y, y + 4, y + 8],
    );
    g.call(slot_target(g, w(0x218), 0x30 / 4), 0, &[c]);
    let dz = sub(g.read::<f32>(x + 8), g.read::<f32>(y + 8));
    let dx = sub(g.read::<f32>(x), g.read::<f32>(y));
    #[allow(clippy::cast_possible_truncation)] // fstp qword; cvtpd2ps
    let yaw0 = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let yaw = wrap_once(
        g,
        img,
        f,
        sub(kf(g, 0x26c), yaw0),
        (0x27b, 0x28b, 0x295, 0x2a2),
    );
    for (k, v) in (0u32..).zip([0, 0, 0x41f0_0000, 0]) {
        g.write::<u32>(y + 4 * k, v);
    }
    g.call(slot_target(g, w(0x2b2), 0x3ac / 4), 0, &[m]);
    // call width: 3
    g.call(slot_target(g, w(0x2ec), 0x520 / 4), 0, &[m, m, 0]);
    // call width: 3
    g.call(
        slot_target(g, w(0x30d), 0x524 / 4),
        0,
        &[m, m, yaw.to_bits()],
    );
    g.write::<u32>(y + 0xc, 0x3f80_0000);
    // call width: 3
    g.call(slot_target(g, w(0x329), 0x394 / 4), 0, &[p, m, y]);
    // call width: 3
    g.call(slot_target(g, w(0x349), 0x3e0 / 4), 0, &[x, x, p]);
    // push 0x1e; sub esp, 8; push X: the return address at S - 0x14
    let esp = s.wrapping_sub(0x14);
    for (k, v) in (1u32..).zip([x, 0x3f33_3333, 0x3f7d_70a4, 0x1e]) {
        g.write::<u32>(esp + 4 * k, v);
    }
    g.write::<u32>(x + 0xc, 0x3f80_0000);
    let fl = Frame {
        at: call(0x387),
        esp,
        ecx: w(0x36a),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x359)))],
        ..*fr
    };
    crate::s5_misc::flight_task(g, img, &fl);
    stores(g, img, f, &[0x38e]);
    // add esp, 0xc; push FN: the return address at S - 0xc
    let esp = s.wrapping_sub(0xc);
    g.write::<u32>(esp + 4, w(0x399));
    stores(g, img, f, &[0x39d]);
    let ts = Frame {
        at: call(0x3a8),
        esp,
        ..*fr
    };
    crate::s3_misc::task_start(g, img, &ts);
    let n = g.read::<u32>(w(0x3ad));
    let flag = g.read::<u32>(w(0x3b6));
    g.write::<u32>(w(0x3bc), n);
    stores(g, img, f, &[0x3c6, 0x3d0, 0x3da]);
    let slot = if flag == 0 { 0x35c } else { 0x674 };
    let v = g.call(slot_target(g, w(0x3c2), slot / 4), 0, &[n]).eax;
    let flag = g.read::<u32>(w(0x3fb));
    g.write::<u32>(w(0x406), v);
    stores(g, img, f, &[0x40a, 0x414]);
    g.write::<u32>(w(0x420), 1);
    if flag == 0 {
        g.write::<u16>(w(0x42a), 0);
    } else {
        g.write::<u16>(w(0x433), 1);
    }
    let count = g.read::<u16>(w(0x43a));
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x449));
        crate::r15::ring_take(g, img, f, node, count, (0x466, 0x46e, 0x49f, 0x4aa));
        node
    };
    let head = g.read::<u32>(w(0x4b0));
    g.write::<u8>(node.wrapping_add(0x18), 2);
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x4d6), node);
    } else {
        let tail = g.read::<u32>(w(0x4c3));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x4eb), node);
    Ret::eax(0)
}

/// [`start_up_0200`] (1274 bytes, 1 overlay).
pub const START_UP_0200: Sig = Sig {
    len: 1274,
    hash: 0xfa24_c321_3e08_d32d,
    calls: &[
        (0x50, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (
            0xe4,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0200_2BE0),
        ),
        (0x1d9, Callee::Sig(&crate::m0074::PICK)),
        (0x25a, Callee::Import("_CIatan2")),
        (0x387, Callee::Sig(&crate::s5_misc::FLIGHT_TASK)),
        (0x3a8, Callee::Sig(&crate::s3_misc::TASK_START)),
        // __security_check_cookie
        (0x4f2, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0200`'s frame (0x10001840, 229 bytes; cdecl `(task)`; a /GS
/// frame, the point `L` at `ebp - 0x14`): until `*G_S` is set, when the
/// flight object's flags have bit 15: slot 531 `(*G_N, L)` and the flight
/// to `L` ([`flight_task`](crate::s5_misc::flight_task) `(L, 0.75, 0.99,
/// 24)`), three words (`G_S` 1); once set, the second flight's bit 15 is
/// the third condition. The motion script
/// ([`motion_script`](crate::s4_misc::motion_script)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)) run; when all three
/// are done, the task's `+0x14` = 3 and its owner's step the next
/// function. Returns the owner, or the objects' `eax`.
pub fn frame_0200<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let l = ebp.wrapping_sub(0x14);
    let mut third = 0u32;
    if g.read::<u32>(w(0x1a)) == 0 {
        if g.read::<u32>(w(0x22)) & 0x8000 != 0 {
            let n = g.read::<u32>(w(0x37));
            g.call(slot_target(g, w(0x31), 0x84c / 4), 0, &[n, l]);
            // push 0x18; sub esp, 8; push L: the return address at ebp - 0x34
            let esp = ebp.wrapping_sub(0x34);
            for (k, v) in (1u32..).zip([l, 0x3f40_0000, 0x3f7d_70a4, 0x18]) {
                g.write::<u32>(esp + 4 * k, v);
            }
            let fl = Frame {
                at: img.rel32_target(f + 0x6c),
                esp,
                ecx: w(0x57),
                xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x47)))],
                ..*fr
            };
            crate::s5_misc::flight_task(g, img, &fl);
            stores(g, img, f, &[0x73, 0x7d, 0x87]);
        }
    } else if g.read::<u32>(w(0x95)) & 0x8000 != 0 {
        third = 1;
    }
    let script = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0xab), w(0xa6));
    // push ebx, esi, edi: the call's return address at ebp - 0x24
    let objs =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0xb2), ebp.wrapping_sub(0x24));
    if objs != 1 || script != 1 || third != 1 {
        return Ret::eax(objs);
    }
    let task = fr.arg(g, 0);
    let owner = g.read::<u32>(task + 8);
    g.write::<u32>(task + 0x14, 3);
    g.write::<u32>(owner + 0x10, w(0xd0));
    Ret::eax(owner)
}

/// [`frame_0200`] (229 bytes, 1 overlay).
pub const FRAME_0200: Sig = Sig {
    len: 229,
    hash: 0xfbfb_c1ed_6ba7_cc00,
    calls: &[
        (0x6c, Callee::Sig(&crate::s5_misc::FLIGHT_TASK)),
        (0xab, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xb2, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        // __security_check_cookie
        (0xdd, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0177`'s start-up (0x10002f00, 1367 bytes; internal, no
/// arguments; an 8-aligned /GS frame, `S = A - 0x68`, the caster's point
/// `Q` at `S + 0x34`, the bone points at `S + 0x44` and `S + 0x54`): slot
/// 719 `(NAME, 8, 0)` into `G_P` (two words first), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the object `O`
/// ([`start_object_at`](crate::s4_misc::start_object_at) `(0, 0x14, -,
/// 0)`) at the caster `*G_A` (slot 262 into globals, `w` 1.0) with the
/// yaw `K - ` slot 333 wrapped once, its `y` raised by `K2`, a word
/// lowered by `K3`; the record words; `Q` = the caster's point at its bone
/// 6 height, `w` 1.0; for each target `i` two objects: `R_i = REC +
/// 0x260 i` (`(1, 0x23, -, CB)`) at the target's point and bone 6 height
/// with its yaw `K - ` slot 333 wrapped once, and `R_i + 0x130` (`(2,
/// 0x23, -, 0)`) at the target's point on its ground (slot 12) facing `Q`
/// (`K - atan2(Q.z - z, Q.x - x)`, `_CIatan2`, wrapped once) with its
/// `+0xe8` lowered by `K4`; slot 413 or 215 (by `*G_F`) of `*G_N` into a
/// record with its words and flag halfword; a node of the free ring
/// ([`ring_take`](crate::r15::ring_take); an empty ring writes through
/// address 0 as the original does) last on the task list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0177<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::s4_misc::start_object_at;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x68);
    let (q, p, p2) = (s + 0x34, s + 0x44, s + 0x54);
    let tab = w(0x15);
    let rot = |g: &mut G, a: u32| st0_f32(g.call(slot_target(g, tab, 0x534 / 4), 0, &[a]).st0);
    stores(g, img, f, &[0x2b, 0x35]);
    // call width: 3
    let part = g
        .call(slot_target(g, tab, 0xb3c / 4), 0, &[w(0x27), 8, 0])
        .eax;
    g.write::<u32>(w(0x45), part);
    crate::l5_objhost::part_init(g, img, call(0x4a));
    start_object_at(g, img, call(0x66), w(0x5a), kf(g, 0x52), 0, 0x14, 0);
    let a = g.read::<u32>(w(0x86));
    // call width: 4
    g.call(
        slot_target(g, w(0x6b), 0x418 / 4),
        0,
        &[a, w(0x80), w(0x7b), w(0x70)],
    );
    let a = g.read::<u32>(w(0x93));
    stores(g, img, f, &[0x97]);
    let r = rot(g, a);
    let yaw = wrap_once(g, img, f, sub(kf(g, 0xb1), r), (0xc9, 0xbf, 0xd9, 0xbf));
    let up = add(kf(g, 0xea), kf(g, 0xf2));
    let a = g.read::<u32>(w(0xf8));
    g.write::<u16>(w(0x100), 0);
    g.write::<f32>(w(0x10d), up);
    let low = sub(kf(g, 0x115), kf(g, 0x11d));
    g.write::<f32>(w(0x134), yaw);
    g.write::<f32>(w(0x13c), low);
    stores(
        g,
        img,
        f,
        &[
            0x140, 0x14a, 0x154, 0x15e, 0x168, 0x172, 0x17c, 0x186, 0x190,
        ],
    );
    // call width: 4
    g.call(
        slot_target(g, w(0x12c), 0x418 / 4),
        0,
        &[a, q, q + 4, q + 8],
    );
    // call width: 3
    g.call(slot_target(g, w(0x1a9), 0x430 / 4), 0, &[a, 6, p]);
    let py = g.read::<u32>(p + 4);
    g.write::<u32>(q + 4, py);
    g.write::<u32>(q + 0xc, 0x3f80_0000);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(if i == 0 { 0x1d7 } else { 0x413 })) {
        let r1 = i.wrapping_mul(0x260).wrapping_add(w(0x208));
        start_object_at(g, img, call(0x213), r1, kf(g, 0x1f4), 1, 0x23, w(0x201));
        let c = r1 + 0xd0;
        let actor = actor_0172(g, img, f, i, (0x227, 0x22e, 0x239));
        // call width: 4
        g.call(
            slot_target(g, w(0x242), 0x418 / 4),
            0,
            &[actor, c, c + 4, c + 8],
        );
        // call width: 3
        g.call(slot_target(g, w(0x25e), 0x430 / 4), 0, &[actor, 6, p2]);
        let y = g.read::<u32>(p2 + 4);
        g.write::<u32>(c + 4, y);
        g.write::<u32>(r1 + 0xdc, 0x3f80_0000);
        let actor = actor_0172(g, img, f, i, (0x28d, 0x294, 0x29f));
        let r = st0_f32(g.call(slot_target(g, w(0x2a5), 0x534 / 4), 0, &[actor]).st0);
        let yaw = wrap_once(
            g,
            img,
            f,
            sub(kf(g, 0x2b9), r),
            (0x2c9, 0x2d3, 0x2dd, 0x2ea),
        );
        g.write::<f32>(r1 + 0xc4, yaw);
        let r2 = r1 + 0x130;
        start_object_at(g, img, call(0x30e), r2, kf(g, 0x2f2), 2, 0x23, 0);
        let c = r2 + 0xd0;
        let actor = actor_0172(g, img, f, i, (0x321, 0x328, 0x333));
        // call width: 4
        g.call(
            slot_target(g, w(0x346), 0x418 / 4),
            0,
            &[actor, c, c + 4, c + 8],
        );
        let ground = st0_f32(g.call(slot_target(g, w(0x353), 0x30 / 4), 0, &[actor]).st0);
        g.write::<f32>(c + 4, ground);
        g.write::<u32>(r2 + 0xdc, 0x3f80_0000);
        let dz = sub(g.read::<f32>(q + 8), g.read::<f32>(c + 8));
        let dx = sub(g.read::<f32>(q), g.read::<f32>(c));
        #[allow(clippy::cast_possible_truncation)] // fstp qword; cvtpd2ps
        let yaw0 = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let yaw = wrap_once(
            g,
            img,
            f,
            sub(kf(g, 0x3b4), yaw0),
            (0x3c3, 0x3cd, 0x3d7, 0x3e4),
        );
        let e8 = sub(g.read::<f32>(r2 + 0xe8), kf(g, 0x3f4));
        g.write::<f32>(r2 + 0xc4, yaw);
        g.write::<f32>(r2 + 0xe8, e8);
        i += 1;
    }
    let flag = g.read::<u32>(w(0x41f));
    let n = g.read::<u32>(w(0x425));
    g.write::<u32>(w(0x42a), n);
    stores(g, img, f, &[0x434, 0x43e]);
    let slot = if flag == 0 { 0x35c } else { 0x674 };
    let v = g.call(slot_target(g, w(0x430), slot / 4), 0, &[n]).eax;
    let flag = g.read::<u32>(w(0x45f));
    g.write::<u32>(w(0x46a), v);
    stores(g, img, f, &[0x46e, 0x478]);
    g.write::<u32>(w(0x484), 1);
    if flag == 0 {
        g.write::<u16>(w(0x48e), 0);
    } else {
        g.write::<u16>(w(0x497), 1);
    }
    let count = g.read::<u16>(w(0x49e));
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x4ad));
        crate::r15::ring_take(g, img, f, node, count, (0x4ca, 0x4d2, 0x503, 0x50e));
        node
    };
    let head = g.read::<u32>(w(0x514));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x536), node);
    } else {
        let tail = g.read::<u32>(w(0x523));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x548), node);
    Ret::eax(0)
}

/// [`start_up_0177`] (1367 bytes, 1 overlay).
pub const START_UP_0177: Sig = Sig {
    len: 1367,
    hash: 0xc64f_ae0d_5cd0_24e8,
    calls: &[
        (0x4a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x66, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x213, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x30e, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x3a2, Callee::Import("_CIatan2")),
        // __security_check_cookie
        (0x54f, Callee::Any),
    ],
    imports: &[],
};
