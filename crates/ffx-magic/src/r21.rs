//! Round 21's ports (magic-r21): the shared rows and the own rows of
//! overlays below `magic_0450`, mostly copies of earlier ports that now
//! read their differing operands from the code.

use crate::m0018::st0_f32;
use crate::r11_fades::stores;
use crate::r13_start::append_kind;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0201`'s second motion towards a point (0x100026e0, 306 bytes;
/// `ecx` = `[obj, motion, to, len, start]`, an aligned /GS frame):
/// [`motion_towards`](crate::s5_misc::motion_towards) with the record's
/// u16 `+0x10` copied to `obj + 0x94` first (also when bit 0 of `obj +
/// 0x90` stops it: the original then returns the cookie word with that
/// halfword in its low half) and the motion's frame less the record's
/// `+0x10` word. Returns slot 248's `eax` with the flags in its low half.
pub fn motion_from_0201<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let this = fr.ecx;
    // push ebp; and esp, -8; sub esp, 0x14: the vector `d` at the bottom
    let d = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x14);
    let obj = g.read::<u32>(this);
    let start = g.read::<u16>(this + 0x10);
    g.write::<u16>(obj + 0x94, start);
    if g.read::<u16>(obj + 0x90) & 1 != 0 {
        let cookie = g.read::<u32>(w(0xa)) ^ d;
        return Ret::eax((cookie & 0xffff_0000) | u32::from(start));
    }
    let model = g.read::<u32>(obj + 0x50);
    let motion = g.read::<u32>(this + 4);
    // call width: 2
    let m = g.call(t(g, 0x37, 228), 0, &[model, motion]).eax;
    let m = m.wrapping_sub(g.read::<u32>(this + 0x10));
    g.write::<u32>(obj + 0x5c, m);
    // cvtdq2pd, plus 2^32 (the table's second double) when negative
    let bias = g.read::<f64>(w(0x60) + 8 * (m >> 31));
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let frames = (f64::from(m.cast_signed()) + bias) as f32;
    let pos = obj.wrapping_add(0x1c);
    let model = g.read::<u32>(obj + 0x50);
    g.write::<f32>(obj + 0x70, frames);
    // call width: 4 (slot 228's words stay)
    g.call(t(g, 0x65, 217), 0, &[model, pos, model, motion]);
    let to = g.read::<u32>(this + 8);
    // call width: 7
    g.call(t(g, 0x87, 234), 0, &[d, to, pos, model, pos, model, motion]);
    let [x, y, z] = [0u32, 4, 8].map(|o| g.read::<f32>(d + o));
    let s = add(add(mul(y, y), mul(x, x)), mul(z, z));
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
    let r = f64::from(s).sqrt() as f32;
    let k = div(sub(r, g.read::<f32>(this + 0xc)), r);
    // call width: 3
    g.call(t(g, 0xe0, 233), 0, &[d, d, k.to_bits()]);
    let out = obj.wrapping_add(0x2c);
    // call width: 3
    let e = g.call(t(g, 0xf7, 248), 0, &[out, pos, d]).eax;
    let flags = (g.read::<u16>(obj + 0x90) & 0xfff7) | 4;
    g.write::<u16>(obj + 0x90, flags);
    Ret::eax((e & 0xffff_0000) | u32::from(flags))
}

/// [`motion_from_0201`] (the `rel32`s: the square root and the cookie
/// check).
pub const MOTION_FROM_0201: Sig = Sig {
    len: 306,
    hash: 0xe478_8545_e26a_b3d4,
    calls: &[
        (0xbd, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x12a, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0304`'s pair of host objects (0x10002270, 312 bytes):
/// [`pair_start`](crate::m0568::pair_start) with its own displacements
/// and one more float moved.
pub const PAIR_START_0304: Sig = Sig {
    len: 312,
    hash: 0xf5eb_b021_8732_e764,
    calls: &[
        (0x36, Callee::Sig(&crate::s4_small::START_HOST_OBJECT)),
        (0xac, Callee::Sig(&crate::s4_small::START_HOST_OBJECT)),
    ],
    imports: &[],
};

/// `magic_0382`'s object step (0x10001270, cdecl `(obj)`, an `ebp` frame,
/// the scale `s` at `ebp - 4`; 624 bytes): with `A = *G_A`, slot 474
/// `(A[0x198], 0x29, P2, P1)` after `P2` = `(2.5, 2.5, 0, 0)`; the
/// counter `n = *G_N`: at 0 first slot 440 `(*G_A, obj, 0, 2)` (and `n`
/// read again); `n` one up. Below 0x19 `s = n K / K2`, below 0x32 `s =
/// K`; from 0x32 on (`s = K`): slot 457 `(A[0x1a0], 2, P3)`, `Q = P3 -
/// P1` (four floats), `x` = slot 453 `(Q, ...)`'s ST0 (stored as f32
/// over the argument), `m = cvttss2si(x K3)` into `*G_I` and `*G_J`, `Q *=
/// K4 / m`, slot 434 `(*G_A, CB, ...)` into `*G_R` and slot 461 `(obj,
/// CB2, ...)`. Then `obj + 0xd0..` = `(P1.xyz, 0)`, `obj + 0x130..` =
/// `(s, s, s, 0)`. Returns `n`, or slot 461's `eax` from 0x32 on.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn step_0382<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let obj = fr.arg(g, 0);
    let a = g.read::<u32>(w(0x5));
    let cam_a = g.read::<u32>(a.wrapping_add(0x198));
    let cam_b = g.read::<u32>(a.wrapping_add(0x1a0));
    let (p1, p2) = (w(0x1d), w(0x22));
    stores(g, img, f, &[0x26, 0x30, 0x3a, 0x44]);
    // call width: 4
    g.call(t(g, 0x18, 474), 0, &[cam_a, 0x29, p2, p1]);
    let mut n = g.read::<u32>(w(0x5a));
    // `s` starts as `K`; the scale below uses `K` as read last
    let mut kk = k(g, 0x62);
    let mut s = kk;
    if n == 0 {
        let ga = g.read::<u32>(w(0x87));
        // call width: 4
        g.call(t(g, 0x76, 440), 0, &[ga, obj, 0, 2]);
        n = g.read::<u32>(w(0x8e));
        kk = k(g, 0x96);
    }
    let n = n.wrapping_add(1);
    g.write::<u32>(w(0x9f), n);
    let mut eax = n;
    if n.cast_signed() < 0x19 {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let nf = n.cast_signed() as f32;
        s = div(mul(nf, kk), k(g, 0xc0));
    } else if n.cast_signed() >= 0x32 {
        let p3 = w(0xd4);
        let q = w(0xf9);
        // call width: 3
        g.call(t(g, 0xcf, 457), 0, &[cam_b, 2, p3]);
        for (from, minus, to) in [
            (0xe7u32, 0xefu32, 0x101u32),
            (0x109, 0x111, 0x119),
            (0x121, 0x129, 0x131),
            (0x139, 0x141, 0x149),
        ] {
            let v = sub(k(g, from), k(g, minus));
            g.write::<f32>(w(to), v);
        }
        // call width: 4 (slot 457's words stay under it)
        let x = st0_f32(g.call(t(g, 0xf4, 453), 0, &[q, cam_b, 2, p3]).st0);
        g.write::<f32>(fr.esp.wrapping_add(4), x);
        let m = crate::sb_sse::cvtt(mul(x, k(g, 0x161))).cast_unsigned();
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let r = div(k(g, 0x169), m.cast_signed() as f32);
        g.write::<u32>(w(0x184), m);
        for (from, to) in [
            (0x190u32, 0x1a6u32),
            (0x1ae, 0x1ba),
            (0x1c2, 0x1ce),
            (0x1d6, 0x1e2),
        ] {
            let v = mul(k(g, from), r);
            g.write::<f32>(w(to), v);
        }
        g.write::<u32>(w(0x199), m);
        let ga = g.read::<u32>(w(0x178));
        let cb = w(0x16e);
        // call width: 6 (the earlier words stay under it)
        let h = g.call(t(g, 0x19e, 434), 0, &[ga, cb, q, cam_b, 2, p3]).eax;
        g.write::<u32>(w(0x1ef), h);
        // call width: 8
        eax = g
            .call(
                t(g, 0x1f4, 461),
                0,
                &[obj, w(0x1f9), ga, cb, q, cam_b, 2, p3],
            )
            .eax;
    }
    for (i, o) in [0x20du32, 0x21d, 0x22d].into_iter().enumerate() {
        let v = g.read::<u32>(w(o));
        g.write::<u32>(obj + 0xd0 + 4 * i as u32, v);
    }
    g.write::<u32>(obj + 0xdc, 0);
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<f32>(obj + o, s);
    }
    g.write::<u32>(obj + 0x13c, 0);
    Ret::eax(eax)
}

/// [`step_0382`] (624 bytes, 1 overlay).
pub const STEP_0382: Sig = Sig {
    len: 624,
    hash: 0xb2de_04ed_b4b2_f8b4,
    calls: &[],
    imports: &[],
};

/// `magic_0382`'s throw set-up (0x100014e0, cdecl `(obj)`, slot 461's
/// callback in [`step_0382`]; a cookie frame, `ebp = E - 4`, the locals
/// `V` at `ebp - 0x24`, `O` at `ebp - 0x14`, `M` at `ebp - 0x34`): `V`
/// = `Q` (four floats), slot 464 `(V)`, `M` = `(0, -1, 0, 0)`, slot 512
/// `(O, V, M)` (`O` its output); `d = O K` (four lanes), `c = *G_I / 2 + 1`
/// (signed); `Q.xyz -= d.xyz c`; `obj + 0xd0..` = `(P1.xyz, 0)`, `+0xe0..`
/// = `(Q.xyz, 0)`, `+0xf0..` = `(d.xyz, 0)`, `+0x130..` = `(0.03, 0.03,
/// 0.03, 0)`; slot 434 `(*G_A, CB3)` into `R`: `R + 0xd0..` = `(P1.x,
/// -0.1, P1.z, 0)`; slot 450 `(obj, CB4)`, slot 451 `(obj, *G_I)`. Each
/// call sees the words the earlier ones left. Returns slot 451's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn throw_0382<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (v, o, m) = (
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x34),
    );
    for (i, q) in [0x14u32, 0x21, 0x32, 0x48].into_iter().enumerate() {
        let x = g.read::<u32>(w(q));
        g.write::<u32>(v + 4 * i as u32, x);
    }
    // call width: 1
    g.call(t(g, 0x3b, 464), 0, &[v]);
    for (i, x) in [0u32, 0xbf80_0000, 0, 0].into_iter().enumerate() {
        g.write::<u32>(m + 4 * i as u32, x);
    }
    // call width: 4 (slot 464's word stays)
    g.call(t(g, 0x66, 512), 0, &[o, v, m, v]);
    let n = g.read::<u32>(w(0x8f)).cast_signed();
    let kk = k(g, 0x97);
    let d: [f32; 4] = [0u32, 4, 8, 0xc].map(|x| mul(g.read::<f32>(o + x), kk));
    // cdq; sub eax, edx; sar eax, 1; inc eax: n / 2 toward 0, plus 1
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let c = (n / 2).wrapping_add(1) as f32;
    for (i, x) in d.into_iter().enumerate() {
        g.write::<f32>(o + 4 * i as u32, x);
    }
    for (i, (from, to)) in [(0xd5u32, 0xfeu32), (0x106, 0x119), (0x121, 0x12d)]
        .into_iter()
        .enumerate()
    {
        let x = sub(k(g, from), mul(d[i], c));
        g.write::<f32>(w(to), x);
    }
    let rows: [(u32, [u32; 3]); 2] = [(0xd0, [0x135, 0x145, 0x155]), (0xe0, [0x16f, 0x17f, 0x18f])];
    for (at, ops) in rows {
        for (i, op) in ops.into_iter().enumerate() {
            let x = g.read::<u32>(w(op));
            g.write::<u32>(obj + at + 4 * i as u32, x);
        }
        g.write::<u32>(obj + at + 0xc, 0);
    }
    for (i, x) in d.into_iter().take(3).enumerate() {
        g.write::<f32>(obj + 0xf0 + 4 * i as u32, x);
    }
    g.write::<u32>(obj + 0xfc, 0);
    let ga = g.read::<u32>(w(0x1d3));
    let cb3 = w(0x1cd);
    for x in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(obj + x, 0x3cf5_c28f);
    }
    g.write::<u32>(obj + 0x13c, 0);
    // call width: 6 (slot 512's words stay under it)
    g.call(t(g, 0x1c8, 434), 0, &[ga, cb3, o, v, m, v]);
    let r = g.read::<u32>(w(0x208));
    let x = g.read::<u32>(w(0x210));
    g.write::<u32>(r + 0xd0, x);
    g.write::<u32>(r + 0xd4, 0xbdcc_cccd);
    let z = g.read::<u32>(w(0x22a));
    g.write::<u32>(r + 0xd8, z);
    g.write::<u32>(r + 0xdc, 0);
    let cb4 = w(0x246);
    // call width: 8
    g.call(t(g, 0x241, 450), 0, &[obj, cb4, ga, cb3, o, v, m, v]);
    let i = g.read::<u32>(w(0x255));
    // call width: 10
    let eax = g
        .call(
            t(g, 0x25a, 451),
            0,
            &[obj, i, obj, cb4, ga, cb3, o, v, m, v],
        )
        .eax;
    Ret::eax(eax)
}

/// [`throw_0382`] (633 bytes, 1 overlay; the cookie check).
pub const THROW_0382: Sig = Sig {
    len: 633,
    hash: 0x4769_d153_6d95_2d78,
    calls: &[(0x271, Callee::Any)],
    imports: &[],
};

/// `magic_0382`'s throw start (0x100018c0, cdecl `(obj)`, slot 449's
/// callback; an `ebp` frame): `s = rand / K1 K2 + K2`; `obj + 0xbc` gets
/// bit 14; with `S = *G_S`: `obj + 0xd0..` = `(S.d0, S.d4, S.d8, 0)`, `v =
/// S.e0.. K3`, each lane plus `rand / K1 K4 - K5` (three more `rand`s;
/// the first kept over the argument) into `obj + 0xe0..` (`w` 0); words
/// at `+0x13c`, `+0x170..+0x18c`, `+0x130..` = `(s, s, s)`; slot 458
/// `(*G_A, obj, 1)`, slot 450 `(obj, CB)`, slot 449 `(obj, CB2)` (each
/// over the earlier words). Returns slot 449's `eax`.
pub fn throw_start_0382<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps of rand (0..=0x7fff)
    let rnd = |g: &mut G| g.rand().cast_signed() as f32;
    let obj = fr.arg(g, 0);
    let r = rnd(g);
    let s = add(mul(div(r, k(g, 0x1e)), k(g, 0x30)), k(g, 0x3d));
    let flags = g.read::<u32>(obj + 0xbc) | 0x4000;
    g.write::<u32>(obj + 0xbc, flags);
    let src = g.read::<u32>(w(0x35));
    for x in [0xd0u32, 0xd4, 0xd8] {
        let v = g.read::<u32>(src + x);
        g.write::<u32>(obj + x, v);
    }
    g.write::<u32>(obj + 0xdc, 0);
    let k3 = k(g, 0x6a);
    let v: [f32; 3] = [0xe0u32, 0xe4, 0xe8].map(|x| mul(g.read::<f32>(src + x), k3));
    let mut out = [0f32; 3];
    for (i, (k1, k4, k5)) in [
        (0xc8u32, 0xd0u32, 0xd8u32),
        (0xf3, 0xfb, 0x103),
        (0x123, 0x130, 0x145),
    ]
    .into_iter()
    .enumerate()
    {
        let r = rnd(g);
        let x = sub(mul(div(r, k(g, k1)), k(g, k4)), k(g, k5));
        out[i] = add(x, v[i]);
    }
    // the first lane's sum is kept over the argument
    g.write::<f32>(fr.esp.wrapping_add(4), out[0]);
    g.write::<f32>(obj + 0xe0, out[0]);
    g.write::<f32>(obj + 0xe4, out[1]);
    for (x, val) in [
        (0xecu32, 0u32),
        (0x13c, 0),
        (0x170, 0x3f80_0000),
        (0x174, 0x3e4c_cccd),
    ] {
        g.write::<u32>(obj + x, val);
    }
    g.write::<f32>(obj + 0xe8, out[2]);
    for x in [0x130u32, 0x134, 0x138] {
        g.write::<f32>(obj + x, s);
    }
    for (x, val) in [
        (0x178u32, 0x3e4c_cccdu32),
        (0x17c, 0x3f80_0000),
        (0x180, 0),
        (0x184, 0),
        (0x188, 0),
        (0x18c, 0xbdb8_51ec),
    ] {
        g.write::<u32>(obj + x, val);
    }
    let ga = g.read::<u32>(w(0x14e));
    // call width: 3
    g.call(t(g, 0x128, 458), 0, &[ga, obj, 1]);
    let cb = w(0x1f6);
    // call width: 5 (slot 458's words stay)
    g.call(t(g, 0x1f1, 450), 0, &[obj, cb, ga, obj, 1]);
    // call width: 7
    let eax = g
        .call(t(g, 0x204, 449), 0, &[obj, w(0x209), obj, cb, ga, obj, 1])
        .eax;
    Ret::eax(eax)
}

/// [`throw_start_0382`] (543 bytes, 1 overlay; `rand` through the IAT).
pub const THROW_START_0382: Sig = Sig {
    len: 543,
    hash: 0xf67a_abd2_f84f_81a1,
    calls: &[],
    imports: &[(0x9, "rand")],
};

/// `magic_0198`'s effect set-up (0x100010e0, cdecl `(a)`; a cookie frame,
/// `ebp = E - 4`, the locals `B` at `ebp - 0x2c`, `U` at `ebp - 0x24`,
/// `V` at `ebp - 0x14`; 564 bytes): `*G0 = a`; `*G1` = slot 168 `(a)`;
/// slot 218 `(*G0, &B)` gives a pointer whose byte goes to `*G2`; `*G3`
/// and `*G4` = slot 215 of `*G1` and `*G2`; slot 217 `(*G2, P)` and
/// `(*G1, Q)`; slot 234 `(&V, Q, P)`, slot 232 `(&V, &V)`; `x` = slot 251
/// `(*G1)` (ST0 as f32, written over the first word left on the stack),
/// slot 233 `(&U, &V, x)`; `P.x += U.x`; `y` = slot 216 `(*G2)` (ST0),
/// `*G5` 0x10, `P.y += y K + U.y`, `P.z += U.z`; the load
/// ([`load`](crate::m0198::load)); slot 711 `(S, N, 0)` with `**G6` 1
/// around it; slot 710 `(T, S, R, 0x100000)`; `**G7 = T`; the part
/// `(slot 941 (*G8))[0]` started on `BUF`
/// ([`start_part`](crate::opu_parts::start_part), 0x1000, 1); two
/// 16-byte copies. Each call sees the words the earlier ones left.
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn set_up_0198<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let (b, u, v) = (
        ebp.wrapping_sub(0x2c),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
    );
    let a = fr.arg(g, 0);
    g.write::<u32>(w(0x15), a);
    let mut st = Pushes::default();
    st.push(a);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, t(g, 0x1b, 168), 0).eax;
    g.write::<u32>(w(0x28), r);
    st.push_all(&[b, rd(g, 0x37)]);
    // call width: 12 (Pushes caps the words at 12)
    let p = st.call(g, t(g, 0x31, 218), 0).eax;
    let byte = u32::from(g.read::<u8>(p));
    st.push(rd(g, 0x48));
    g.write::<u32>(w(0x4d), byte);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, t(g, 0x52, 215), 0).eax;
    st.push(rd(g, 0x60));
    g.write::<u32>(w(0x65), r);
    // call width: 12 (Pushes caps the words at 12)
    let r = st.call(g, t(g, 0x6a, 215), 0).eax;
    g.write::<u32>(w(0x77), r);
    st.push_all(&[w(0x81), rd(g, 0x87)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x7c, 217), 0);
    st.push_all(&[w(0x99), rd(g, 0x9f)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x94, 217), 0);
    st.push_all(&[w(0xac), w(0xb4), v]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0xba, 234), 0);
    st.push_all(&[v, v]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0xcc, 232), 0);
    st.push(rd(g, 0xdf));
    // call width: 12 (Pushes caps the words at 12)
    let x = st0_f32(st.call(g, t(g, 0xd9, 251), 0).st0);
    // add esp, 0x38: the first word stays, `x` written over it
    st.pop(14);
    st.set_top(x.to_bits());
    st.push_all(&[v, u]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x104, 233), 0);
    let px = add(g.read::<f32>(w(0x114)), g.read::<f32>(u));
    st.push(rd(g, 0x124));
    g.write::<f32>(w(0x12c), px);
    // call width: 12 (Pushes caps the words at 12)
    let y = st0_f32(st.call(g, t(g, 0x11e, 216), 0).st0);
    let y = mul(y, g.read::<f32>(w(0x144)));
    g.write::<u32>(w(0x14a), 0x10);
    let py = add(add(y, g.read::<f32>(u + 4)), g.read::<f32>(w(0x15b)));
    g.write::<f32>(w(0x163), py);
    let pz = add(g.read::<f32>(w(0x16b)), g.read::<f32>(u + 8));
    g.write::<f32>(w(0x178), pz);
    // the load: slot 216's four words above its return address
    let load = Frame {
        at: img.rel32_target(f + 0x17d),
        esp: ebp.wrapping_sub(0x30).wrapping_sub(0x14),
        ..*fr
    };
    crate::m0198::load(g, img, &load);
    let lock = rd(g, 0x182);
    st.push(0);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    st.push_all(&[w(0x196), w(0x1a1)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x191, 711), 0);
    let lock = rd(g, 0x1a8);
    st.push(0x0010_0000);
    g.write::<u32>(lock, saved);
    st.push_all(&[w(0x1b9), w(0x1c4), w(0x1c9)]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x1b4, 710), 0);
    let at = rd(g, 0x1d0);
    g.write::<u32>(at, w(0x1d6));
    st.push(rd(g, 0x1e1));
    // call width: 12 (Pushes caps the words at 12)
    let table = st.call(g, t(g, 0x1db, 941), 0).eax;
    let part = g.read::<u32>(table);
    crate::opu_parts::start_part(
        g,
        img,
        img.rel32_target(f + 0x1fe),
        w(0x1f9),
        0x1000,
        part,
        1,
    );
    for (from, to) in [(0x206u32, 0x214u32), (0x21b, 0x224)] {
        for i in 0..4u32 {
            let x = g.read::<u32>(w(from) + 4 * i);
            g.write::<u32>(w(to) + 4 * i, x);
        }
    }
    Ret::eax(0)
}

/// [`set_up_0198`] (the `rel32`s: the load, the part start, the cookie
/// check).
pub const SET_UP_0198: Sig = Sig {
    len: 564,
    hash: 0xa0b7_e0b6_13d7_328d,
    calls: &[
        (0x17d, Callee::Sig(&crate::m0198::LOAD)),
        (0x1fe, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x22c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0198`'s frame step (0x10001360, no arguments; a cookie frame,
/// `ebp = E - 4`, the colour `V` at `ebp - 0x14`; 773 bytes). Once (`*S0`
/// 0) busy while slot 225 `(*G0)` answers, then `*S0` one up; once (`*S1`
/// 0, set first) slot 226 `(*G0, 0)`; once (`*S2` 0) busy while slot 227
/// `(*G0)` is below 1, then `*S2` one up. `**G9` = 1 when slot 253 `()`
/// answers 0, else 0. With the count `n = *G5`: at 0x10 slot 228 `(*G1,
/// 0xb0)` (`n` read again); `e = n - 0x1e`: at 0 slot 254 `(*G3, 0)`,
/// `(*G4, 0)` and slot 228 `(*G1, 0xb1)`; for `e` in 0..=0x26 the blend
/// `u` = `(T[i] + D1) D2` (doubles, `T = *G10`, `i = ((e << 15) / 38 +
/// 0x4000) >> 4 & 0xfff`) mixes `V = P (K - u) + Q u` (four lanes), slot
/// 255 `(*G1, &V)`, and at 0x26 slots 249 `(*G3, 0)`, 237 `(*G3, 0x24,
/// 0, Z)` and 256 `(*G3, 1.0, 0.5, 0.5, 0xf)`; otherwise at `n` 0x55
/// slots 220 `(*G11, 0x40, 0x100)`, 257 `(*G3, 1)` and 239 `(*G0, -5,
/// 0)`. Each call sees the words the earlier ones left. Returns 0.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn step_0198<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let v = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    if rd(g, 0x12) == 0 {
        // call width: 1
        if g.call(t(g, 0x1a, 225), 0, &[rd(g, 0x20)]).eax != 0 {
            return Ret::eax(0);
        }
        let n = rd(g, 0x39).wrapping_add(1);
        g.write::<u32>(w(0x39), n);
    }
    if rd(g, 0x3f) == 0 {
        let a = rd(g, 0x4f);
        g.write::<u32>(w(0x5b), 1);
        // call width: 2
        g.call(t(g, 0x47, 226), 0, &[a, 0]);
    }
    if rd(g, 0x6a) == 0 {
        // call width: 1
        if g.call(t(g, 0x72, 227), 0, &[rd(g, 0x78)]).eax.cast_signed() < 1 {
            return Ret::eax(0);
        }
        let n = rd(g, 0x92).wrapping_add(1);
        g.write::<u32>(w(0x92), n);
    }
    // call width: 0
    let r = g.call(t(g, 0x97, 253), 0, &[]).eax;
    let flag = rd(g, 0xa5);
    g.write::<u32>(flag, u32::from(r == 0));
    let mut n = rd(g, 0xb1);
    if n == 0x10 {
        // call width: 2
        g.call(t(g, 0xbb, 228), 0, &[rd(g, 0xc6), 0xb0]);
        n = rd(g, 0xd3);
    }
    let e = n.wrapping_sub(0x1e).cast_signed();
    if e == 0 {
        let mut st = Pushes::default();
        st.push_all(&[0, rd(g, 0xea)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0xe3, 254), 0);
        st.push_all(&[0, rd(g, 0xfe)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0xf7, 254), 0);
        st.push_all(&[0xb1, rd(g, 0x116)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x10b, 228), 0);
    }
    if (0..=0x26).contains(&e) {
        // imul by 0x6bca1af3, sar 4: (e << 15) / 38, then + 0x4000, >> 4
        let q = ((i64::from(e << 15) * 0x6bca_1af3) >> 36) as i32;
        let i = (q.wrapping_add(0x4000) >> 4).cast_unsigned() & 0xfff;
        let tab = rd(g, 0x143);
        let c = f64::from(g.read::<f32>(tab.wrapping_add(4 * i)));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let u = ((c + g.read::<f64>(w(0x17f))) * g.read::<f64>(w(0x199))) as f32;
        let rest = sub(g.read::<f32>(w(0x15e)), u);
        for (k, (p, q)) in [
            (0x16cu32, 0x1a5u32),
            (0x1ce, 0x1bd),
            (0x1ef, 0x1de),
            (0x210, 0x1ff),
        ]
        .into_iter()
        .enumerate()
        {
            let x = add(mul(g.read::<f32>(w(p)), rest), mul(g.read::<f32>(w(q)), u));
            g.write::<f32>(v + 4 * k as u32, x);
        }
        // call width: 2
        g.call(t(g, 0x185, 255), 0, &[rd(g, 0x18b), v]);
        if e == 0x26 {
            let a = rd(g, 0x238);
            // call width: 2
            g.call(t(g, 0x230, 249), 0, &[a, 0]);
            let a = rd(g, 0x25a);
            // call width: 6 (slot 249's words stay)
            g.call(t(g, 0x245, 237), 0, &[a, 0x24, 0, w(0x24a), a, 0]);
            let a = rd(g, 0x28c);
            // call width: 5
            g.call(
                t(g, 0x261, 256),
                0,
                &[a, 0x3f80_0000, 0x3f00_0000, 0x3f00_0000, 0xf],
            );
        }
    } else if n == 0x55 {
        let mut st = Pushes::default();
        st.push_all(&[0x100, 0x40, rd(g, 0x2bf)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x2ac, 220), 0);
        st.push_all(&[1, rd(g, 0x2ce)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x2c6, 257), 0);
        st.push_all(&[0, 0xffff_fffb, rd(g, 0x2eb)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x2db, 239), 0);
    }
    Ret::eax(0)
}

/// [`step_0198`] (the cookie check twice).
pub const STEP_0198: Sig = Sig {
    len: 773,
    hash: 0x7aaa_42f7_c6d5_0e5d,
    calls: &[(0x29e, Callee::Any), (0x2fd, Callee::Any)],
    imports: &[],
};

/// `magic_0198`'s end step (0x10001670, no arguments, no frame; 476
/// bytes): nothing while `*S2` is 0. From the count `*G5` 0x53 on: the
/// three part tables (slot 941 of `*A1`, `*A2`, `*A3` into `*H1..*H3`),
/// `**G7 = T`, slots 723 `(*B)`, 724 `(*C)`, 258 `(BUF)`, 231 `(*E, *C,
/// *D)`, 230 `(*F, *E)`, `**G12 = W`, `**G7 = T` and slot 712 `(BUF,
/// 0x30)` with `**G6` 1 around it; when that answers, slots 254 `(*G3,
/// 1)`, `(*G4, 1)`, 249 `(*G3, 0x100)`, 256 `(*G3, 1.0, 1.0, 1.0, 0)`,
/// 213 `(*G1, 1)` and 214 `(*G0)`. Then, unless `**G9` is set, the count
/// one up. (The caller's `esi`, pushed under the calls, is no callee's
/// argument and is left out of their words.) Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn end_0198<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    if rd(g, 0x2) == 0 {
        return Ret::eax(0);
    }
    if rd(g, 0xe).wrapping_sub(0x53).cast_signed() >= 0 {
        let mut st = Pushes::default();
        for (arg, table, out) in [
            (0x23u32, 0x1cu32, 0x36u32),
            (0x31, 0x3b, 0x4e),
            (0x49, 0x53, 0x66),
        ] {
            st.push(rd(g, arg));
            // call width: 12 (Pushes caps the words at 12)
            let r = st.call(g, t(g, table, 941), 0).eax;
            g.write::<u32>(w(out), r);
        }
        st.push(rd(g, 0x61));
        let at = rd(g, 0x6b);
        g.write::<u32>(at, w(0x71));
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x76, 723), 0);
        st.push(rd(g, 0x89));
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x83, 724), 0);
        st.push(w(0x9b));
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x96, 258), 0);
        st.push_all(&[rd(g, 0xa9), rd(g, 0xb4), rd(g, 0xc0)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0xae, 231), 0);
        st.push_all(&[rd(g, 0xc8), rd(g, 0xd3)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0xcd, 230), 0);
        let at = rd(g, 0xe0);
        st.push(0x30);
        g.write::<u32>(at, w(0xe8));
        st.push(w(0xf2));
        let at = rd(g, 0xed);
        g.write::<u32>(at, w(0xf8));
        let lock = rd(g, 0xfd);
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        // call width: 12 (Pushes caps the words at 12)
        let r = st.call(g, t(g, 0x10a, 712), 0).eax;
        let lock = rd(g, 0x118);
        g.write::<u32>(lock, saved);
        if r != 0 {
            let mut st = Pushes::default();
            st.push_all(&[1, rd(g, 0x133)]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t(g, 0x12b, 254), 0);
            st.push_all(&[1, rd(g, 0x148)]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t(g, 0x140, 254), 0);
            st.push_all(&[0x100, rd(g, 0x160)]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t(g, 0x155, 249), 0);
            st.pop(6);
            st.push_all(&[0, 0x3f80_0000, 0x3f80_0000, 0x3f80_0000, rd(g, 0x198)]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t(g, 0x16d, 256), 0);
            st.push_all(&[1, rd(g, 0x1a7)]);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t(g, 0x19f, 213), 0);
            st.push(rd(g, 0x1ba));
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t(g, 0x1b4, 214), 0);
        }
    }
    if g.read::<u32>(rd(g, 0x1ca)) == 0 {
        let n = rd(g, 0x1d5).wrapping_add(1);
        g.write::<u32>(w(0x1d5), n);
    }
    Ret::eax(0)
}

/// [`end_0198`] (476 bytes, 1 overlay).
pub const END_0198: Sig = Sig {
    len: 476,
    hash: 0x3138_612d_7b58_8eac,
    calls: &[],
    imports: &[],
};

/// `magic_0276`'s colour fade (0x10003330, no arguments; an `ebp` frame,
/// the three colour bytes at `ebp - 0xc`, `- 8`, `- 4`; 411 bytes): while
/// the wait `*W` is above 0 it counts down (returns the new wait). Else by
/// the state `*S` with the step `c = *C - 1` (stored back):
/// - 0: `(0x80, (c + 24) 64 / 24, (24 - c) 64 / 24)`; at `c` 0 or less
///   the state 1 and `*C` 0x20;
/// - 1: `((48 c + 0xa00) / 32, (0x8c0 - 6 c) / 32, 0x40)`; at 0 the
///   state 2 and `*C` 0x20;
/// - 2: `((0x1000 - 48 c) / 32, (0x1000 - 0x3a c) / 32, 64 c / 32)`; at
///   0 the state 3;
/// - any other: `*D` = 1 (returns the state less 2).
///
/// (Signed divisions toward 0, each byte the quotient's low byte.) Then,
/// unless `*D` is set, slot 860 `(0xc, 2, 0x40, b0, b1, 0x80, b2)` with
/// `**G_LOCK` 1 around it (returns the lock pointer); else the last
/// quotient. The bytes are pushed as whole words whose upper three bytes
/// the code never writes:
// junk read: [ebp-0xc], [ebp-8], [ebp-4] bytes 1..3 never written; 0 (brief 2026-10-24)
pub fn fade_0276<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let wait = rd(g, 0x4);
    if wait.cast_signed() > 0 {
        let left = wait.wrapping_sub(1);
        g.write::<u32>(w(0x11), left);
        return Ret::eax(left);
    }
    let state = rd(g, 0x1a);
    let (b, last) = match state {
        0 => {
            let c = rd(g, 0xf6).wrapping_sub(1).cast_signed();
            let b1 = c.wrapping_add(0x18).wrapping_shl(6) / 24;
            let b2 = 0x18i32.wrapping_sub(c).wrapping_shl(6) / 24;
            g.write::<i32>(w(0x132), c);
            if c <= 0 {
                g.write::<u32>(w(0x143), 1);
                g.write::<u32>(w(0x14d), 0x20);
            }
            ([0x80, b1, b2], b2)
        }
        1 => {
            let c = rd(g, 0xa6).wrapping_sub(1).cast_signed();
            g.write::<i32>(w(0xad), c);
            let b0 = c.wrapping_mul(48).wrapping_add(0xa00) / 32;
            let b1 = 0x8c0i32.wrapping_sub(c.wrapping_mul(6)) / 32;
            if c <= 0 {
                g.write::<u32>(w(0xea), 2);
                g.write::<u32>(w(0x14d), 0x20);
            }
            ([b0, b1, 0x40], b1)
        }
        2 => {
            let c = rd(g, 0x40).wrapping_sub(1).cast_signed();
            let b0 = 0x1000i32.wrapping_sub(c.wrapping_mul(48)) / 32;
            let b1 = 0x1000i32.wrapping_sub(c.wrapping_mul(0x3a)) / 32;
            let b2 = c.wrapping_shl(6) / 32;
            g.write::<i32>(w(0x85), c);
            if c <= 0 {
                g.write::<u32>(w(0x97), 3);
            }
            ([b0, b1, b2], b2)
        }
        _ => {
            g.write::<u32>(w(0x30), 1);
            return Ret::eax(state.wrapping_sub(2));
        }
    };
    if rd(g, 0x157) != 0 {
        return Ret::eax(last.cast_unsigned());
    }
    let byte = |x: i32| x.cast_unsigned() & 0xff;
    let lock = rd(g, 0x162);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 7
    g.call(
        slot_target(g, w(0x17a), 860),
        0,
        &[0xc, 2, 0x40, byte(b[0]), byte(b[1]), 0x80, byte(b[2])],
    );
    let lock = rd(g, 0x18d);
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// [`fade_0276`] (411 bytes, 1 overlay).
pub const FADE_0276: Sig = Sig {
    len: 411,
    hash: 0xa7ae_4ef7_c074_f1a9,
    calls: &[],
    imports: &[],
};

/// A node taken off a free ring (`[node]` the previous, `[node + 4]` the
/// next) and set up as a task node with the callback `cb`: the ring's
/// head `*G_FREE` moves to the next one (0 when it was the last of
/// `*G_COUNT`), the count goes down. The caller links it. Returns the
/// node (the head the caller had; 0 when the count was 0: the original
/// then writes through address 0, as the port does).
#[allow(clippy::too_many_arguments)] // the operands of the inlined take
fn ring_task_take<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    count: u16,
    (free_next, free_none): (u32, u32),
    (cb, count_out): (u32, u32),
) -> u16 {
    let w = |o: u32| img.word(f + o);
    let prev = g.read::<u32>(node);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev.wrapping_add(4), next);
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node);
    g.write::<u32>(next, prev);
    if count > 1 {
        let head = g.read::<u32>(node + 4);
        g.write::<u32>(w(free_next), head);
    } else {
        g.write::<u32>(w(free_none), 0);
    }
    let left = count.wrapping_sub(1);
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(node + o, 0);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 0x10, w(cb));
    g.write::<u8>(node + 0x18, 0);
    g.write::<u16>(w(count_out), left);
    left
}

/// `magic_0276`'s start-up (0x10002e20, 1158 bytes, run by its load
/// step; no arguments; an 8-aligned cookie frame, the locals from `A`,
/// the point `L2` at `A + 0x20`): two words set, slot 719 `(PART, 8, 0)` into `*H`, the
/// part set up ([`part_init`](crate::l5_objhost::part_init)); the object
/// `OBJ` started ([`start_object_at`](crate::s4_misc::start_object_at):
/// `K`, index 1, owner 0x16); the point `P` from slot 262 of the actor,
/// `w` 1.0; the yaw `K1 - slot 333` wrapped once; twelve words; for each
/// of the `*N` actors (objects from `OBJS - 0xd4`, 0x130 apart) an object
/// ([`start_object_at`](crate::s4_misc::start_object_at): `K5`, index 0,
/// owner 0x2e, `owner_b` the callback), its point from slot 262 of the
/// actor `T[B[i]]`, `y` from slot 268 `(a, 6, &L2)`, `w` 1.0 and the yaw
/// `K1 - atan2(P.z - z, P.x - x)` wrapped once; the fade's words and the
/// sound (slot 413 or 215 by `*G3`, as [`sounds_0440`]); two task nodes
/// off the free ring ([`fade_0276`] and its pair) appended to the task
/// list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0276<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r20::wrap_once;
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x34: the locals from `a`
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x34);
    let l2 = a.wrapping_add(0x20);
    stores(g, img, f, &[0x2b, 0x35]);
    // call width: 3
    let h = g.call(t(g, 0x15, 719), 0, &[w(0x27), 8, 0]).eax;
    g.write::<u32>(w(0x45), h);
    crate::l5_objhost::part_init(g, img, call(0x4a));
    crate::s4_misc::start_object_at(g, img, call(0x66), w(0x5a), k(g, 0x52), 1, 0x16, 0);
    let act = rd(g, 0x86);
    let p = w(0x80);
    // call width: 4 (the start's words under it are not read)
    g.call(t(g, 0x6b, 262), 0, &[act, p, w(0x7b), w(0x70)]);
    let act = rd(g, 0x93);
    g.write::<u32>(w(0x99), 0x3f80_0000);
    // call width: 1
    let r = st0_f32(g.call(t(g, 0x8d, 333), 0, &[act]).st0);
    let yaw = wrap_once(g, img, f, sub(k(g, 0xb1), r), (0xc9, 0xbf, 0xd9, 0xbf));
    g.write::<f32>(w(0xee), yaw);
    stores(
        g,
        img,
        f,
        &[0xf2, 0xfc, 0x106, 0x110, 0x11a, 0x124, 0x12e, 0x138, 0x142],
    );
    g.write::<u16>(w(0x14e), 0);
    stores(g, img, f, &[0x152]);
    let mut i = 0u32;
    let mut y = w(0x169);
    // the count read at the loop's entry (`+0x15e`) and at its end (`+0x28d`)
    let mut n_at = 0x15e;
    while i.cast_signed() < rd(g, n_at).cast_signed() {
        n_at = 0x28d;
        let obj = y.wrapping_sub(0xd4);
        crate::s4_misc::start_object_at(g, img, call(0x18a), obj, k(g, 0x175), 0, 0x2e, w(0x17a));
        let b = u32::from(g.read::<u8>(rd(g, 0x19e).wrapping_add(i)));
        let actor = g.read::<u32>(w(0x1a9).wrapping_add(4 * b));
        let (x, z) = (y.wrapping_sub(4), y.wrapping_add(4));
        // call width: 4
        g.call(t(g, 0x1bf, 262), 0, &[actor, x, y, z]);
        // call width: 3 (slot 262's words stay under it)
        g.call(t(g, 0x1d1, 268), 0, &[actor, 6, l2, actor, x, y, z]);
        let ly = g.read::<u32>(l2 + 4);
        g.write::<u32>(y, ly);
        g.write::<u32>(y.wrapping_add(8), 0x3f80_0000);
        let dz = sub(k(g, 0x1fd), g.read::<f32>(z));
        let dx = sub(k(g, 0x1f1), g.read::<f32>(x));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let yaw = wrap_once(
            g,
            img,
            f,
            sub(k(g, 0x240), ang),
            (0x24f, 0x259, 0x263, 0x270),
        );
        g.write::<f32>(y.wrapping_sub(0x10), yaw);
        i += 1;
        y = y.wrapping_add(0x130);
    }
    // the fade's words and the sound (`magic_0440`'s block, other
    // operands)
    let on = rd(g, 0x299) != 0;
    let id = rd(g, 0x29f);
    g.write::<u32>(w(0x2a4), id);
    stores(g, img, f, &[0x2ae, 0x2b8, 0x2c2, 0x2cc, 0x2d6, 0x2e0]);
    let slot = if on { 0x674 / 4 } else { 0x35c / 4 };
    // call width: 1
    let r = g.call(t(g, 0x2aa, slot), 0, &[id]).eax;
    let on = rd(g, 0x301) != 0;
    g.write::<u32>(w(0x30c), r);
    g.write::<u32>(w(0x312), u32::MAX);
    g.write::<u32>(w(0x31c), 0);
    g.write::<u32>(w(0x326), 1);
    if on {
        g.write::<u16>(w(0x339), 1);
    } else {
        g.write::<u16>(w(0x330), 0);
    }
    // two task nodes off the free ring
    let count = g.read::<u16>(w(0x340));
    let (first, left) = if count == 0 {
        (0, 0)
    } else {
        let node = rd(g, 0x35a);
        let left = ring_task_take(g, img, f, node, count, (0x37f, 0x37f), (0x3a4, 0x3af));
        (node, left)
    };
    let head = rd(g, 0x3b5);
    g.write::<u32>(first.wrapping_add(4), 0);
    let list = if head == 0 {
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0x3da), first);
        first
    } else {
        let last = rd(g, 0x3c5);
        g.write::<u32>(first, last);
        g.write::<u32>(last.wrapping_add(4), first);
        head
    };
    g.write::<u32>(first.wrapping_add(8), first);
    let second = if left == 0 {
        0
    } else {
        let node = rd(g, 0x37f);
        ring_task_take(g, img, f, node, left, (0x403, 0x40b), (0x43c, 0x447));
        node
    };
    g.write::<u32>(second.wrapping_add(4), 0);
    if list == 0 {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x465), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first.wrapping_add(4), second);
    }
    g.write::<u32>(second.wrapping_add(8), second);
    g.write::<u32>(w(0x477), second);
    Ret::eax(0)
}

/// [`start_up_0276`] (the `rel32`s: the part's set-up, the object start
/// twice, `_CIatan2` and the cookie check).
pub const START_UP_0276: Sig = Sig {
    len: 1158,
    hash: 0xec1d_e298_8c5d_a78d,
    calls: &[
        (0x4a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x66, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x18a, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x22e, Callee::Any),
        (0x47e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0276`'s load step (0x10002cf0, the 118-byte shape) running
/// [`start_up_0276`].
pub fn load_0276<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0276)
}

/// [`load_0276`].
pub const LOAD_0276: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0276))],
    imports: &[],
};

/// A task node `n` (from [`take`](crate::sa_pool::take)) appended to the
/// DLL's task list as its inlined append does (the list's first word at
/// the operand `first`; `last` read at `last_r` when the list had nodes;
/// `first_w` written when it was empty): `[n+4]` 0, `[n]` the old last
/// (or 0), the old last's `+4` = `n`.
fn append_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    n: u32,
    (first, last_r, first_w): (u32, u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    let head = g.read::<u32>(w(first));
    g.write::<u32>(n.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(first_w), n);
    } else {
        let last = g.read::<u32>(w(last_r));
        g.write::<u32>(n, last);
        g.write::<u32>(last.wrapping_add(4), n);
    }
}

/// `magic_0352`'s objects restart (0x10005410, 716 bytes; no arguments,
/// an `ebp` frame, the scale kept at `ebp - 4`): slot 904 `(0, 2)`; with
/// the block `S = *G_S`: when `S + 0x5f0` leaves the list at the
/// immediate `+0x29` ([`list_remove`](crate::l5_objhost::list_remove)),
/// `**G_P` = the immediate `+0x3e`, slot 713 `(it)` and bit 15 of its
/// halfword `+0x124` cleared. The scale `s = K1 / slot 282 () * K2`
/// (ST0); four objects `S + 0x4c0`, `+0x390`, `+0x260`, `+0x130`
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), indices
/// 4..1), each with its callback and byte 2 at `+0x104`/`+0x126`, `+0x60`
/// times `K3` and `+0x64` = `s`; the light record `P = S + 0x9a0` (its
/// colour and sizes) in a task node; the fan `C = S + 0xab0` started
/// ([`sprite_fan_start`](crate::s6_misc::sprite_fan_start)); `S + 0xbdc`
/// 12, `+0xbe0` 30; a second task node. Returns the second node.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn restart_0352<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // call width: 2
    g.call(t(g, 0x5, 904), 0, &[0, 2]);
    let s = g.read::<u32>(w(0x19));
    let o = s.wrapping_add(0x5f0);
    if crate::l5_objhost::list_remove(g, img, call(0x2e), w(0x29), o) != 0 {
        g.write::<u32>(g.read::<u32>(w(0x37)), w(0x3e));
        // call width: 1
        g.call(t(g, 0x43, 713), 0, &[o]);
        let h = g.read::<u16>(o + 0x124);
        g.write::<u16>(o + 0x124, h & 0x7fff);
    }
    // call width: 0
    let v = st0_f32(g.call(t(g, 0x5f, 282), 0, &[]).st0);
    let scale = mul(div(k(g, 0x72), v), k(g, 0x82));
    for (at, d, index, cb, cbd, k3) in [
        (0x97u32, 0x4c0u32, 4u32, 0xa1u32, 0x5c4u32, 0xb5u32),
        (0xd7, 0x390, 3, 0xe1, 0x494, 0xf5),
        (0x117, 0x260, 2, 0x121, 0x364, 0x135),
        (0x157, 0x130, 1, 0x161, 0x234, 0x175),
    ] {
        let obj =
            crate::s4_misc::start_object_plain(g, img, call(at), s.wrapping_add(d), scale, index);
        g.write::<u32>(s.wrapping_add(cbd), w(cb));
        g.write::<u8>(s.wrapping_add(cbd + 0x22), 2);
        let x = mul(g.read::<f32>(obj.wrapping_add(0x60)), k(g, k3));
        g.write::<f32>(obj.wrapping_add(0x60), x);
        g.write::<f32>(obj.wrapping_add(0x64), scale);
    }
    let p = s.wrapping_add(0x9a0);
    g.write::<u32>(p + 0x81, 0xff80_ffff);
    g.write::<u8>(p + 0x80, 0xff);
    g.write::<u16>(p + 0x85, 0xffff);
    g.write::<u8>(p + 0x87, 0);
    g.write::<u32>(p + 0x88, 0);
    g.write::<u16>(p + 0x98, 0x48);
    g.write::<u32>(p + 0x90, 0xc);
    g.write::<u32>(p + 0x8c, 0xc);
    g.write::<u32>(p + 0x94, 0);
    let n = crate::sa_pool::take(g, img, call(0x1e9), w(0x192));
    g.write::<u32>(n.wrapping_add(0x14), p);
    append_node(g, img, f, n, (0x1ef, 0x202, 0x214));
    g.write::<u32>(n.wrapping_add(8), n);
    g.write::<u32>(w(0x222), n);
    let c = s.wrapping_add(0xab0);
    g.write::<u32>(c + 0xf0, 0x3080_8080);
    g.write::<u16>(c + 0x124, 0);
    g.write::<u32>(c + 0x114, 0x100);
    g.write::<u32>(c + 0x118, 0xd0);
    g.write::<u32>(c + 0x110, 0);
    g.write::<u32>(c + 0xf8, 0);
    g.write::<u32>(c + 0xfc, 0x3d4c_cccd);
    g.write::<u32>(c + 0x100, 0);
    let fan = Frame {
        at: call(0x26a),
        ecx: c,
        ..*fr
    };
    crate::s6_misc::sprite_fan_start(g, img, &fan);
    g.write::<u32>(s.wrapping_add(0xbdc), 0xc);
    g.write::<u32>(s.wrapping_add(0xbe0), 0x1e);
    let n2 = crate::sa_pool::take(g, img, call(0x288), w(0x26f));
    if g.read::<u32>(w(0x28e)) == 0 {
        g.write::<u32>(n2.wrapping_add(4), 0);
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x2bc), n2);
        g.write::<u32>(w(0x2c1), n2);
    } else {
        g.write::<u32>(n2.wrapping_add(4), 0);
        let last = g.read::<u32>(w(0x2a0));
        g.write::<u32>(n2, last);
        g.write::<u32>(last.wrapping_add(4), n2);
        g.write::<u32>(w(0x2aa), n2);
    }
    g.write::<u32>(n2.wrapping_add(8), n2);
    Ret::eax(n2)
}

/// [`restart_0352`] (the `rel32`s: the list removal, four object starts,
/// the pool's take twice and the fan start).
pub const RESTART_0352: Sig = Sig {
    len: 716,
    hash: 0xc304_4152_85e2_2b38,
    calls: &[
        (0x2e, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x97, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0xd7, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x117, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x157, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x1e9, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x26a, Callee::Sig(&crate::s6_misc::SPRITE_FAN_START)),
        (0x288, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0435`'s task (0x10001570, cdecl `(task)`, an `ebp` frame; 229
/// bytes): the wait `*W` counts down while positive, and as it reaches 0
/// the caster is shown (slot 296 `(*N, 0)` outside a battle with `*N`
/// below 0x20, else slot 257 `(*A, 1)`); then the two motion scripts
/// `R1`, `R2` ([`motion_script`](crate::s4_misc::motion_script)) and the
/// objects ([`run_objects`](crate::l5_objhost::run_objects)). When that
/// gives 1 and `R1`'s word `+0x24` is 1: unless `R1`'s halfword `+0x1c`
/// is set slot 213 `(R1[2], 2)`; `R1 + 0x14` = 1; unless `R2`'s halfword
/// `+0x1c` is set slot 213 `(R2[2], 0)`; `R2 + 0x14` = 1, the task's
/// `+0x14` 3 and its record's `+0x10` the next callback. Returns the
/// objects' answer, 1 or the record.
pub fn task_0435<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let wait = rd(g, 0x4);
    if wait.cast_signed() > 0 {
        let left = wait.wrapping_sub(1);
        g.write::<u32>(w(0xe), left);
        if left.cast_signed() <= 0 {
            let n = rd(g, 0x20);
            if rd(g, 0x18) == 0 && n.cast_signed() < 0x20 {
                // call width: 2
                g.call(t(g, 0x2d, 296), 0, &[n, 0]);
            } else {
                // call width: 2
                g.call(t(g, 0x3a, 257), 0, &[rd(g, 0x42), 1]);
            }
        }
    }
    crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x57), w(0x52));
    crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x61), w(0x5c));
    // push ebp; the call's return address
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x66), fr.esp.wrapping_sub(8));
    if r != 1 {
        return Ret::eax(r);
    }
    if rd(g, 0x71) != 1 {
        return Ret::eax(1);
    }
    if g.read::<u16>(w(0x7a)) == 0 {
        // call width: 2
        g.call(t(g, 0x82, 213), 0, &[rd(g, 0x8a), 2]);
    }
    let h2 = g.read::<u16>(w(0x9c));
    g.write::<u32>(w(0xa3), 1);
    if h2 == 0 {
        // call width: 2
        g.call(t(g, 0xae, 213), 0, &[rd(g, 0xb6), 0]);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(w(0xca), 1);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0xdf));
    Ret::eax(rec)
}

/// [`task_0435`] (the `rel32`s: the motion scripts and the objects' run).
pub const TASK_0435: Sig = Sig {
    len: 229,
    hash: 0x7558_8f4b_6cf1_35dd,
    calls: &[
        (0x57, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x61, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x66, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};

/// `magic_0262`'s height from a distance (0x100024d0, 264 bytes; `ecx` =
/// the record `R = [obj, out, B, -, e]`, an 8-aligned /GS frame, the
/// locals from `L` after `sub esp, 0x30`). Nothing when bit 0 of the
/// object's `+0x90` is set (the original then returns its cookie word).
/// Else with the table `T = *G_T`: `c = T[(e + 0x4000) >> 4 & 0xfff]`,
/// `q = T[e >> 4 & 0xfff] / c`; slot 217 `(obj[0x50], A)` and slot 217
/// `(B, D)` (`A = L + 0x1c`, `D = L + 0xc`), slot 234 `(D, A, D)`; `R + 4`
/// = `|D| / c * q - D.y`. (The `eax` it leaves is the square root's.)
pub fn height_0262<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rec = fr.ecx;
    let l = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    let obj = g.read::<u32>(rec);
    if g.read::<u8>(obj + 0x90) & 1 != 0 {
        return Ret::eax(g.read::<u32>(w(0xa)) ^ l);
    }
    let e = g.read::<i32>(rec + 0xc);
    let tab = g.read::<u32>(w(0x2c));
    let at = |i: i32| tab.wrapping_add(4 * (i.cast_unsigned() & 0xfff));
    let c = g.read::<f32>(at(e.wrapping_add(0x4000) >> 4));
    let q = div(g.read::<f32>(at(e >> 4)), c);
    let (a, d) = (l + 0x1c, l + 0xc);
    let model = g.read::<u32>(obj + 0x50);
    // call width: 2
    g.call(t(g, 0x5b, 217), 0, &[model, a]);
    let b = g.read::<u32>(rec + 8);
    // call width: 4 (the first call's words stay)
    g.call(t(g, 0x7c, 217), 0, &[b, d, model, a]);
    // call width: 7
    g.call(t(g, 0x9b, 234), 0, &[d, a, d, b, d, model, a]);
    let [x, y, z] = [0u32, 4, 8].map(|o| g.read::<f32>(d + o));
    let s = add(add(mul(y, y), mul(x, x)), mul(z, z));
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
    let r = f64::from(s).sqrt() as f32;
    let h = sub(mul(div(r, c), q), y);
    g.write::<f32>(rec + 4, h);
    Ret::eax(0)
}

/// [`height_0262`] (the `rel32`s: the square root and the cookie check).
pub const HEIGHT_0262: Sig = Sig {
    len: 264,
    hash: 0x5b17_826a_95e8_9fdb,
    calls: &[
        (0xd8, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x100, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0246`'s landing spark (0x100019d0, cdecl `(obj)`; a cookie
/// frame, `ebp = E - 4`, the matrix `M` at `ebp - 0x54`, the vector `V`
/// at `ebp - 0x24`, the point `P` at `ebp - 0x14`; 349 bytes): `M` from
/// slot 235 with its diagonal `(*X0, *X1, *X2)`, slot 554 `(M, M, C)`,
/// slot 552 `(V, D)`; `P = (obj + 0xe0.., 1.0)` turned by slot 229 `(P,
/// M, P)`. When `P.y` is at least `*Y` (ordered): the first free of the
/// twelve flag bytes `F` is taken and its object `BASE + 0x130 i`
/// started ([`start_object_owners`](crate::s4_misc::start_object_owners):
/// `K`, index 3, owners 0) at `(P.x, *Y - K2, P.z)`; with none free
/// nothing more; either way `obj + 0xc` = `0xffff0000`. Returns the
/// object (or slot 229's `eax`).
pub fn spark_0246<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v, p) = (
        ebp.wrapping_sub(0x54),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
    );
    // call width: 1
    g.call(t(g, 0x19, 235), 0, &[m]);
    for (o, x) in [(0u32, 0x29u32), (0x14, 0x36), (0x28, 0x4c)] {
        let d = g.read::<u32>(w(x));
        g.write::<u32>(m + o, d);
    }
    // call width: 4 (slot 235's word stays)
    g.call(t(g, 0x52, 554), 0, &[m, m, w(0x3e), m]);
    // call width: 6
    g.call(t(g, 0x6d, 552), 0, &[v, w(0x67), m, m, w(0x3e), m]);
    for (i, o) in [0xe0u32, 0xe4, 0xe8].into_iter().enumerate() {
        let x = g.read::<u32>(obj + o);
        g.write::<u32>(p + 4 * i as u32, x);
    }
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    // call width: 9
    let mut eax = g
        .call(t(g, 0xa8, 229), 0, &[p, m, p, v, w(0x67), m, m, w(0x3e), m])
        .eax;
    let y = g.read::<f32>(p + 4);
    // comiss; jb: a smaller or unordered `y` ends it
    if y >= k(g, 0xcb) {
        let flags = w(0xd6);
        if let Some(i) = (0..0xcu32).find(|&i| g.read::<u8>(flags + i) == 0) {
            g.write::<u8>(w(0xe7).wrapping_add(i), 1);
            let o = i.wrapping_mul(0x130).wrapping_add(w(0xf4));
            if o != 0 {
                eax = crate::s4_misc::start_object_owners(
                    g,
                    img,
                    img.rel32_target(f + 0x10d),
                    o,
                    k(g, 0xfe),
                    3,
                    0,
                    (0, 0),
                );
                let x = g.read::<u32>(p);
                g.write::<u32>(o + 0xd0, x);
                let h = sub(k(g, 0x122), k(g, 0x12a));
                g.write::<f32>(o + 0xd4, h);
                let z = g.read::<u32>(p + 8);
                g.write::<u32>(o + 0xd8, z);
            }
        }
        g.write::<u32>(obj + 0xc, 0xffff_0000);
    }
    Ret::eax(eax)
}

/// [`spark_0246`] (the `rel32`s: the object start and the cookie check).
pub const SPARK_0246: Sig = Sig {
    len: 349,
    hash: 0xcfec_662e_7805_2198,
    calls: &[
        (0x10d, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x155, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0375`'s part object set-up (0x10001c80, cdecl `(obj)`, an
/// `ebp` frame; 324 bytes): `**G_P` = the immediate `+0xb`; the part
/// `(slot 941 ((*G_D)[4]))[1]` started on `obj`
/// ([`start_part`](crate::opu_parts::start_part), 0x1000, 1); by the
/// object's index `i = (obj - BASE) >> 7`: 0, 1 its point and scale,
/// `+0x54` 0; 2 another point and scale with `+0x54` = `0xffffb000`; 3
/// and 4 a fourth set, `+0x54` 0; then `+0x4c` 1.0, `+0x50` 0, `+0x58` 0,
/// `+0x5c` 1, `+0x6c` 1.0. Another index sets nothing more. Returns `i`.
pub fn part_object_0375<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let p = g.read::<u32>(w(0x4));
    g.write::<u32>(p, w(0xb));
    let d = g.read::<u32>(g.read::<u32>(w(0x10)).wrapping_add(0x10));
    // call width: 1
    let table = g.call(slot_target(g, w(0x18), 941), 0, &[d]).eax;
    let obj = fr.arg(g, 0);
    let program = g.read::<u32>(table.wrapping_add(4));
    crate::opu_parts::start_part(g, img, img.rel32_target(f + 0x36), obj, 0x1000, program, 1);
    let i = obj.wrapping_sub(w(0x3d)).cast_signed() >> 7;
    let set = |g: &mut G, words: &[(u32, u32)]| {
        for &(o, v) in words {
            g.write::<u32>(obj.wrapping_add(o), v);
        }
    };
    let s0 = 0x3c7d_e721;
    let s1 = 0x3ce1_4163;
    let s2 = 0x3d0d_4cba;
    let s3 = 0x3d19_9674;
    let zero54 = match i {
        0 => {
            set(
                g,
                &[
                    (0x40, 0x4120_0000),
                    (0x44, 0xc16f_fff7),
                    (0x48, 0x4292_0000),
                ],
            );
            set(g, &[(0x60, s0), (0x64, s0), (0x68, s0)]);
            true
        }
        1 => {
            set(
                g,
                &[
                    (0x40, 0x4080_0000),
                    (0x44, 0xc16f_fff7),
                    (0x48, 0x4258_0000),
                ],
            );
            set(g, &[(0x60, s1), (0x64, s1), (0x68, s1)]);
            true
        }
        2 => {
            set(
                g,
                &[
                    (0x40, 0xc079_99a2),
                    (0x44, 0xc171_9985),
                    (0x48, 0x4119_994f),
                ],
            );
            set(
                g,
                &[(0x54, 0xffff_b000), (0x60, s2), (0x64, s2), (0x68, s2)],
            );
            false
        }
        3 | 4 => {
            set(g, &[(0x68, s3), (0x64, s3), (0x60, s3)]);
            set(
                g,
                &[
                    (0x48, 0x41a2_6620),
                    (0x44, 0xc171_9985),
                    (0x40, 0x4131_9994),
                ],
            );
            true
        }
        _ => return Ret::eax(i.cast_unsigned()),
    };
    if zero54 {
        set(g, &[(0x54, 0)]);
    }
    set(
        g,
        &[
            (0x4c, 0x3f80_0000),
            (0x50, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x6c, 0x3f80_0000),
        ],
    );
    Ret::eax(i.cast_unsigned())
}

/// [`part_object_0375`] (the `rel32`: the part start).
pub const PART_OBJECT_0375: Sig = Sig {
    len: 324,
    hash: 0xbe8b_c5fe_ddf4_be98,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// `magic_0419`'s spark task (0x10001b60, cdecl `(task)`, an `ebp`
/// frame; 423 bytes): with the count `*W` at 0 or below the task is
/// given back ([`give_back`](crate::sa_pool::give_back), its `eax`
/// returned). Else two sparks, each a pool record
/// ([`pool_take`](crate::sb_misc::pool_take); none left ends it, 0):
/// slot 719 `(PART, 9, 0)` at `+0x30`, `+0x34` 1, u16 `+0x38` -1 and
/// `+0x3a` 0x200; slot 237 `(*A, rand % 155 + 2, 0, rec)`; `+0xc` 1.0;
/// three slot 307 numbers `(x - K) K'` into `+0x10..+0x18`; slot 314 `(pi)`
/// into `+0x1c`; `+0x20..+0x2c` = `(0, -0.02, 0, 1)`; slot 404 `(0.9)`
/// `(y + K5) K5'` into `+0x3c`; the sizes and colour words. Returns the
/// last slot 404's `eax`.
pub fn sparks_0419<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let task = fr.arg(g, 0);
    if g.read::<u32>(w(0x6)).cast_signed() <= 0 {
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x11),
            task,
        ));
    }
    let mut eax = 0;
    for _ in 0..2 {
        let rec = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x25));
        if rec == 0 {
            return Ret::eax(0);
        }
        let mut st = Pushes::default();
        st.push_all(&[0, 9, w(0x44)]);
        // call width: 12 (Pushes caps the words at 12)
        let h = st.call(g, t(g, 0x35, 719), 0).eax;
        g.write::<u32>(rec + 0x30, h);
        g.write::<u16>(rec + 0x38, 0xffff);
        g.write::<u32>(rec + 0x34, 1);
        g.write::<u16>(rec + 0x3a, 0x200);
        let r = g.rand().cast_signed() % 0x9b + 2;
        st.push_all(&[rec, 0, r.cast_unsigned(), g.read::<u32>(w(0x82))]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x6f, 237), 0);
        g.write::<u32>(rec + 0xc, 0x3f80_0000);
        for (table, minus, times, to) in [
            (0x89u32, 0xa8u32, 0xb5u32, 0x10u32),
            (0xad, 0xd2, 0xdf, 0x14),
            (0xd7, 0xfc, 0x113, 0x18),
        ] {
            // call width: 12 (Pushes caps the words at 12)
            let x = st0_f32(st.call(g, t(g, table, 307), 0).st0);
            g.write::<f32>(rec + to, mul(sub(x, k(g, minus)), k(g, times)));
        }
        // add esp, 0x18: slot 719's first word stays, pi written over it
        st.pop(6);
        st.set_top(0x4049_0fdb);
        // call width: 12 (Pushes caps the words at 12)
        let a = st0_f32(st.call(g, t(g, 0x101, 314), 0).st0);
        g.write::<f32>(rec + 0x1c, a);
        for (o, v) in [
            (0x20u32, 0u32),
            (0x24, 0xbca3_d70a),
            (0x28, 0),
            (0x2c, 0x3f80_0000),
        ] {
            g.write::<u32>(rec + o, v);
        }
        st.set_top(0x3f66_6666);
        // call width: 12 (Pushes caps the words at 12)
        let out = st.call(g, t(g, 0x128, 404), 0);
        eax = out.eax;
        let y = mul(add(st0_f32(out.st0), k(g, 0x163)), k(g, 0x17d));
        g.write::<u32>(rec + 0x4c, 0x20);
        g.write::<u32>(rec + 0x50, 0x20);
        g.write::<u16>(rec + 0x64, 0x70d0);
        g.write::<u8>(rec + 0x66, 0x20);
        g.write::<f32>(rec + 0x3c, y);
        g.write::<u32>(rec + 0x68, 0x0020_70d0);
    }
    Ret::eax(eax)
}

/// [`sparks_0419`] (the `rel32`s: the give-back and the pool's take;
/// `rand` through the IAT).
pub const SPARKS_0419: Sig = Sig {
    len: 423,
    hash: 0x6de1_d597_a553_df8a,
    calls: &[
        (0x11, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x25, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
    ],
    imports: &[(0x1c, "rand")],
};

/// `magic_0301`'s pair of lifted parts (0x10001a80, cdecl `(obj)`; a
/// cookie frame, `ebp = E - 4`, the vector `V` at `ebp - 0x14`, the point
/// `O` at `ebp - 0x24`; 389 bytes): slot 444 `(0)`; twice a record `R`
/// from slot 434 `(*G_A, CB)` with its current entry's kind (`R + 0x2c +
/// 16 R[0xac]`) 2 and then 3, `V` = `(0, 23, 0, 0)` and `(0, -23, 0, 0)`,
/// slot 474 `(A[0x198], 0x18 or 0x25, V, O)` and `R + 0xd0..` = `(O.xyz,
/// 0)`; then slots 450 `(obj, CB2)` and 451 `(obj, 5)`. Each call sees the
/// words the earlier ones left. Returns slot 451's `eax`.
///
/// `magic_0301` 0x10001810 (431 bytes, `r26::LIFT_0301_1810`; `push imm32`
/// at `+0x141`) has the kinds 0 and 1 (the immediates `+0x65`, `+0xf4`)
/// and its own tail: a third record from slot 434 `(*G_A, CB3)`, slot 450
/// `(obj, CB4)`, then (the stack cleared) slot 449 `(obj, CB5)` and the
/// object's current entry `obj + 0x2c + 16 obj[0xac]` set to `A[0x198]`.
/// It returns `2 obj[0xac]` (the index doubled for the store).
pub fn lift_0301<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    let f = fr.at;
    let v1810 = img.word(f + 0x141) as u8 == 0x68;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (v, o) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let bone = g.read::<u32>(g.read::<u32>(w(0x11)).wrapping_add(0x198));
    let mut st = Pushes::default();
    st.push(0);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x22, 444), 0);
    for (ga, cb, table, kind, y, id, t474) in [
        (
            0x3cu32,
            0x36u32,
            0x31u32,
            w(0x65),
            0x41b8_0000u32,
            0x18u32,
            0x59u32,
        ),
        (0xb9, 0xb3, 0x94, w(0xf4), 0xc1b8_0000, 0x25, 0xe8),
    ] {
        st.push_all(&[w(cb), g.read::<u32>(w(ga))]);
        // call width: 12 (Pushes caps the words at 12)
        let r = st.call(g, t(g, table, 434), 0).eax;
        let c = g.read::<u32>(r.wrapping_add(0xac));
        st.push_all(&[o, v, id]);
        g.write::<u32>(r.wrapping_add(c.wrapping_mul(16)).wrapping_add(0x2c), kind);
        for (i, x) in [0u32, y, 0, 0].into_iter().enumerate() {
            g.write::<u32>(v + 4 * i as u32, x);
        }
        st.push(bone);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, t474, 474), 0);
        for i in 0..3u32 {
            let x = g.read::<u32>(o + 4 * i);
            g.write::<u32>(r.wrapping_add(0xd0 + 4 * i), x);
        }
        g.write::<u32>(r.wrapping_add(0xdc), 0);
    }
    if v1810 {
        st.push_all(&[w(0x142), g.read::<u32>(w(0x148))]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x123, 434), 0);
        st.push_all(&[w(0x16c), obj]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x167, 450), 0);
        // `add esp, 0x44`: the words so far popped
        let mut st = Pushes::default();
        st.push_all(&[w(0x17d), obj]);
        // call width: 2
        st.call(g, t(g, 0x183, 449), 0);
        let i = g
            .read::<u32>(obj.wrapping_add(0xac))
            .wrapping_add(g.read::<u32>(obj.wrapping_add(0xac)));
        g.write::<u32>(obj.wrapping_add(0x2c).wrapping_add(i.wrapping_mul(8)), bone);
        return Ret::eax(i);
    }
    st.push_all(&[w(0x15a), obj]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t(g, 0x123, 450), 0);
    st.push_all(&[5, obj]);
    // call width: 12 (Pushes caps the words at 12)
    let eax = st.call(g, t(g, 0x162, 451), 0).eax;
    Ret::eax(eax)
}

/// [`lift_0301`] (the cookie check).
pub const LIFT_0301: Sig = Sig {
    len: 389,
    hash: 0x54a3_b8e3_b542_f6a5,
    calls: &[(0x17d, Callee::Any)],
    imports: &[],
};

/// `magic_0305`'s burst of four sparks (0x10001cf0, cdecl `(obj)`, an
/// `ebp` frame; 438 bytes): four pool records
/// ([`pool_take`](crate::sb_misc::pool_take); none left ends it, 0), each
/// with the part `(*G_P)[3]` at `+0x30`, `+0x34` 1, u16 `+0x38` -1 and
/// `+0x3a` 0x200; its point the object's `+0xd0..` with each of `x, y, z`
/// moved by a slot 314 `(4.0)` number; `+0x10..+0x18` slot 314 `(0.5)`
/// numbers, `+0x1c` slot 314 `(pi)`; `+0x20..+0x2c` = `(0, -0.05, 0, 1)`;
/// `+0x3c` = `(slot 314 (0.2) + K) K'`; the counts. Returns the last slot
/// 314's `eax`.
pub fn burst_0305<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32| slot_target(g, w(o), 314);
    let obj = fr.arg(g, 0);
    let mut eax = 0;
    for _ in 0..4 {
        let rec = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x11));
        if rec == 0 {
            return Ret::eax(0);
        }
        let part = g.read::<u32>(g.read::<u32>(w(0x21)).wrapping_add(0xc));
        g.write::<u16>(rec + 0x38, 0xffff);
        g.write::<u32>(rec + 0x30, part);
        g.write::<u32>(rec + 0x34, 1);
        g.write::<u16>(rec + 0x3a, 0x200);
        for i in 0..4u32 {
            let x = g.read::<u32>(obj.wrapping_add(0xd0 + 4 * i));
            g.write::<u32>(rec + 4 * i, x);
        }
        // one word pushed, each call's argument written over it
        let mut num = |g: &mut G, table: u32, x: u32| {
            let out = g.call(t(g, table), 0, &[x]);
            eax = out.eax;
            st0_f32(out.st0)
        };
        for (i, table) in [0x66u32, 0x87, 0xac].into_iter().enumerate() {
            let d = num(g, table, 0x4080_0000);
            let at = rec + 4 * i as u32;
            let v = add(g.read::<f32>(at), d);
            g.write::<f32>(at, v);
        }
        for (table, to) in [(0xd2u32, 0x10u32), (0xee, 0x14), (0x105, 0x18)] {
            let v = num(g, table, 0x3f00_0000);
            g.write::<f32>(rec + to, v);
        }
        let a = num(g, 0x11c, 0x4049_0fdb);
        g.write::<f32>(rec + 0x1c, a);
        for (o, v) in [
            (0x20u32, 0u32),
            (0x24, 0xbd4c_cccd),
            (0x28, 0),
            (0x2c, 0x3f80_0000),
        ] {
            g.write::<u32>(rec + o, v);
        }
        let y = num(g, 0x133, 0x3e4c_cccd);
        let y = mul(add(y, g.read::<f32>(w(0x16e))), g.read::<f32>(w(0x18e)));
        for (o, v) in [
            (0x40u32, 0u32),
            (0x58, 0x10),
            (0x5c, 0),
            (0x4c, 0x10),
            (0x50, 0),
        ] {
            g.write::<u32>(rec + o, v);
        }
        g.write::<f32>(rec + 0x3c, y);
    }
    Ret::eax(eax)
}

/// [`burst_0305`] (the `rel32`: the pool's take).
pub const BURST_0305: Sig = Sig {
    len: 438,
    hash: 0xa3fa_adbc_61a3_b39b,
    calls: &[(0x11, Callee::Sig(&crate::sb_misc::POOL_TAKE))],
    imports: &[],
};

/// `magic_0440`'s sound block in its start-up (the code at `+0x3d0` of
/// 0x10002ca0, inlined; `owner` the loop's last owner word): `*G1` =
/// `owner - 0x2d`; `*G4 = *G2`, `*G5` = `C1` when that is above 0 (signed),
/// else `C2`, `*G6` = -1; slot 413 `(*G2)` when `*G3` is set, else slot
/// 215 `(*G2)`, its `eax` into `*G7`; `*G8` -1, `*G9` 0, `*G10` 1 and the
/// halfword `*G11` 1 when `*G3` is set (read again), else 0. (`f` the
/// start-up; the operands are its own.)
pub fn sounds_0440<G: Game>(g: &mut G, img: &Image, f: u32, owner: u32) {
    let w = |o: u32| img.word(f + o);
    let n = owner.wrapping_sub(0x2d);
    g.write::<u32>(w(0x3d6), n);
    let id = g.read::<u32>(w(0x3db));
    let pick = if n.cast_signed() > 0 {
        w(0x3e0)
    } else {
        w(0x3e5)
    };
    let on = g.read::<u32>(w(0x3ee)) != 0;
    g.write::<u32>(w(0x3f4), id);
    g.write::<u32>(w(0x400), pick);
    g.write::<u32>(w(0x406), u32::MAX);
    let slot = if on { 0x674 / 4 } else { 0x35c / 4 };
    // call width: 1
    let r = g.call(slot_target(g, w(0x3fa), slot), 0, &[id]).eax;
    let on = g.read::<u32>(w(0x425)) != 0;
    g.write::<u32>(w(0x430), r);
    g.write::<u32>(w(0x436), u32::MAX);
    g.write::<u32>(w(0x440), 0);
    g.write::<u32>(w(0x44a), 1);
    if on {
        g.write::<u16>(w(0x45d), 1);
    } else {
        g.write::<u16>(w(0x454), 0);
    }
}

/// `magic_0440`'s start-up (0x10002ca0, 1947 bytes):
/// [`start_up_0680`](crate::r20::start_up_0680) with the sound block
/// [`sounds_0440`] after the loop.
pub const START_UP_0440: Sig = Sig {
    len: 1947,
    hash: 0xce47_6ae9_d6eb_4ef6,
    calls: &[
        (0x50, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x58, Callee::Sig(&crate::sb_misc::DEAL)),
        (0x155, Callee::Sig(&crate::sa_misc::ACTOR_KIND_CHECK)),
        (0x17d, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x24c, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x2f8, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        // __security_check_cookie
        (0x793, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0440`'s load step (0x10002b70, the 118-byte shape) running
/// [`start_up_0680`](crate::r20::start_up_0680)'s copy.
pub fn load_0440<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(
        g,
        img,
        fr,
        crate::r9_load::Shape::Zero,
        crate::r20::start_up_0680,
    )
}

/// [`load_0440`].
pub const LOAD_0440: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0440))],
    imports: &[],
};

/// `magic_0431`'s start-up (0x10004390, 325 bytes, run by its load step
/// [`load_0431`]; no arguments, no frame): eight words set (`+0x14` ..
/// `+0x5a`) and slot 719 `(PART, 8, 0)` (`+0x10`) into `+0x6a`; the part
/// set up ([`part_setup`](crate::sb_misc::part_setup), the pushed buffer
/// `+0x6f`); the camera started ([`camera_start`](crate::m0431::camera_start));
/// two words, a halfword 0 and a byte 1 set (`+0x82` .. `+0xa1`); slot
/// 271 `(4)` (the buffer's words under it); a pool node
/// ([`take`](crate::sa_pool::take), owner `+0xb4`, kind 1) appended to
/// the task list, the 0x400 records set up
/// ([`records_init_400`](crate::sa_misc::records_init_400), the block
/// `+0xea`), a second node (owner `+0xfc`, kind 2) appended. Returns 0.
pub fn start_0431<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let e = fr.esp;
    let s719 = slot_target(g, w(0x01), 719);
    stores(g, img, f, &[0x14, 0x1e, 0x28, 0x32, 0x3c, 0x46, 0x50, 0x5a]);
    // call width: 3
    let part = g.call(s719, fr.ecx, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x6a), part);
    // add esp, 8: the `push 0` stays; push BUF; call
    g.write::<u32>(e.wrapping_sub(4), 0);
    g.write::<u32>(e.wrapping_sub(8), w(0x6f));
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x74), e.wrapping_sub(0xc));
    let camera = Frame {
        at: img.rel32_target(f + 0x79),
        esp: e.wrapping_sub(0xc),
        ..*fr
    };
    crate::m0431::camera_start(g, img, &camera);
    let s271 = slot_target(g, w(0x7e), 271);
    stores(g, img, f, &[0x82, 0x8c]);
    g.write::<u16>(w(0x99), 0);
    g.write::<u8>(w(0xa1), 1);
    // call width: 3 (the buffer's words stay under it)
    g.call(s271, fr.ecx, &[4, w(0x6f), 0]);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xb9), w(0xb4));
    append_kind(g, img, f, node, 1, [0xbf, 0xd3, 0xf2, 0xe5, 0xf2]);
    crate::sa_misc::records_init_400(g, img, img.rel32_target(f + 0xf7), w(0xea));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x101), w(0xfc));
    append_kind(g, img, f, node, 2, [0x107, 0x11b, 0x128, 0x136, 0x13e]);
    Ret::eax(0)
}

/// [`start_0431`] (the `rel32`s: the part's set-up, the camera start,
/// the pool's take twice and the records).
pub const START_0431: Sig = Sig {
    len: 325,
    hash: 0x37d3_f6c0_3016_9568,
    calls: &[
        (0x74, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x79, Callee::Sig(&crate::m0431::CAMERA_START)),
        (0xb9, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xf7, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x101, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0431`'s load step (0x10001250, 130 bytes; cdecl, no
/// arguments): busy (0) while slot 225 `(1)` answers, or until both
/// sounds are ready ([`sounds_ready`](crate::m0431::sounds_ready) 1);
/// once, with `*G_A` 0, slot 226 `(*G_B, 0)` and `*G_A = 1`; busy while
/// slot 227 `(*G_B)` is below 1; then [`start_0431`] and the two
/// callbacks set (`+0x6b`, `+0x75`). Returns 0.
pub fn load_0431<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    load_with_0431(
        g,
        img,
        fr,
        |g, img, fr| crate::m0431::sounds_ready(g, img, fr).eax,
        start_0431,
    )
}

/// [`load_0431`] with the DLL's own sounds' check (`ready`, the `rel32`
/// at `+0x17`) and start-up (`start`, `+0x67`): `magic_0602`'s copy.
pub(crate) fn load_with_0431<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    ready: fn(&mut G, &Image, &Frame) -> u32,
    start: fn(&mut G, &Image, &Frame) -> Ret,
) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // call width: 1
    if g.call(slot_target(g, w(0x01), 225), fr.ecx, &[1]).eax != 0 {
        return Ret::eax(0);
    }
    let sub = |at: u32| Frame {
        at: img.rel32_target(f + at),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    if ready(g, img, &sub(0x17)) != 1 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x22)) == 0 {
        let b = g.read::<u32>(w(0x32));
        let flag = crate::r11_misc::imm8(img, f + 0x2f);
        // call width: 2
        g.call(slot_target(g, w(0x2a), 226), fr.ecx, &[b, flag]);
        g.write::<u32>(w(0x43), 1);
    }
    let b = g.read::<u32>(w(0x52));
    // call width: 1
    if g.call(slot_target(g, w(0x4c), 227), fr.ecx, &[b])
        .eax
        .cast_signed()
        < 1
    {
        return Ret::eax(0);
    }
    start(g, img, &sub(0x67));
    stores(g, img, f, &[0x6b, 0x75]);
    Ret::eax(0)
}

/// [`load_0431`] (the `rel32`s: the sounds' check and the start-up).
pub const LOAD_0431: Sig = Sig {
    len: 130,
    hash: 0xfede_8aed_fe98_dfa9,
    calls: &[
        (0x17, Callee::Sig(&crate::m0431::SOUNDS_READY)),
        (0x67, Callee::Sig(&START_0431)),
    ],
    imports: &[],
};

/// `magic_0304`'s caster and summon placement (0x100015d0, 1,588
/// bytes): [`place_pair`](crate::m0596::place_pair) with the state's
/// fields 0x1048 higher.
pub const PLACE_PAIR_0304: Sig = Sig {
    len: 1588,
    hash: 0x9df1_b64f_333c_0110,
    // __security_check_cookie
    calls: &[(0x62c, Callee::Any)],
    imports: &[],
};

/// `magic_0430`'s wave tick (0x10002580, 457 bytes): the code of
/// [`wave_tick_0588`](crate::r18::wave_tick_0588) with the count and
/// state at `E + 0xeb8`/`0xebc`.
pub const WAVE_TICK_0430: Sig = Sig {
    len: 457,
    hash: 0xbfcf_266e_f1f6_af01,
    calls: &[
        (0x149, Callee::Sig(&crate::s5_misc::WAVE_TABLE)),
        (0x1c1, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0162`'s twenty model passes (0x100039e0, 400 bytes): the code
/// of [`passes_0586`](crate::r18::passes_0586) with the angle, point,
/// translation and records 0x60 further into the scene.
pub const PASSES_0162: Sig = Sig {
    len: 400,
    hash: 0x0e44_80d0_e415_a72b,
    calls: &[
        (
            0x16a,
            Callee::Sig(&crate::o6_fam_pairs4::THREE_PASS_MODEL_0162_1A80),
        ),
        (0x188, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0434`'s summon frame (0x10005730, 226 bytes):
/// [`summon_frame_fade`](crate::s8_misc::summon_frame_fade) in
/// `magic_0610`'s shape, its own displacements and scene end.
pub const SUMMON_FRAME_0434: Sig = Sig {
    len: 226,
    hash: 0x4066_581a_4e6b_7643,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0xb2, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xbc, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xc4, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (
            0xd3,
            Callee::Sig(&crate::o6_fam_pairs6::SCENE_END_0434_5B90),
        ),
        (0xde, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0434`'s summon matrix set-up (0x10006610, 627 bytes):
/// [`summon_matrix_0610`](crate::r20::summon_matrix_0610) with the
/// colour show called before slot 257.
pub const SUMMON_MATRIX_0434: Sig = Sig {
    len: 627,
    hash: 0x83ef_6267_6166_da64,
    calls: &[
        (0x23d, Callee::Sig(&crate::m0434::COLOUR_SHOW)),
        // __security_check_cookie
        (0x26b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0434`'s sixth object and light records (0x10007fc0, 409
/// bytes): [`lights_start_0610`](crate::r18::lights_start_0610) with
/// the animated object start.
pub const LIGHTS_START_0434: Sig = Sig {
    len: 409,
    hash: 0xc32c_4cc5_bf72_ee55,
    calls: &[
        (0x45, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xbb, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x15e, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0347`'s summon object between two actors (0x10003c60, 1,336
/// bytes): [`summon_object_between`](crate::s9_misc::summon_object_between)
/// with the animated object start and its callback word.
pub const SUMMON_OBJECT_BETWEEN_0347: Sig = Sig {
    len: 1336,
    hash: 0xe618_41c7_8e9f_21e3,
    calls: &[
        (0xd3, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x2ce, Callee::Any),
        (0x4ff, Callee::Sig(&crate::sb_misc::SPRITE_INIT_COLOUR)),
        (0x530, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0347`'s burst start (0x100041a0, 845 bytes):
/// [`burst_0491`](crate::r20::burst_0491) with slot 618's words in
/// another order and the indices swapped.
pub const BURST_0347: Sig = Sig {
    len: 845,
    hash: 0x3bbb_c2d7_f22f_5f75,
    calls: &[
        (0xcc, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x179, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x345, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0347`'s spark task (0x100044f0, 934 bytes): the code of
/// [`spark_task`](crate::r14::spark_task) with its own object between
/// and `magic_0491`'s burst.
pub const SPARK_TASK_0347: Sig = Sig {
    len: 934,
    hash: 0xe954_e609_449f_510a,
    calls: &[
        (0x34, Callee::Sig(&SUMMON_OBJECT_BETWEEN_0347)),
        (0x3f, Callee::Any),
        (0x5f, Callee::Any),
        (0x217, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x222, Callee::Any),
        (0x236, Callee::Sig(&BURST_0347)),
        (0x242, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x390, Callee::Sig(&crate::s4_misc::TRAIL_DRAW)),
        (0x39e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0430`'s caster object (0x10002900, 349 bytes):
/// [`caster_object_0588`](crate::r18::caster_object_0588) with the
/// animated object start and its own block offsets.
pub const CASTER_OBJECT_0430: Sig = Sig {
    len: 349,
    hash: 0xa469_85b7_a7eb_a071,
    calls: &[
        (0x30, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xe6, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x155, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0388`'s colour object set-up (0x10006ba0, 460 bytes):
/// [`colour_setup_0645`](crate::r20::colour_setup_0645) with the alpha
/// from `xmm2`.
pub const COLOUR_SETUP_0388: Sig = Sig {
    len: 460,
    hash: 0x161e_a27c_b2d8_06e7,
    calls: &[
        (0x17c, Callee::Sig(&crate::nodes::INSERT)),
        (0x1ba, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// `magic_0393`'s caster object start (0x10001b50, 689 bytes): the code
/// of [`caster_start_0450`](crate::r16::caster_start_0450) with the
/// object at `B+0xbe0` and index 10 (the `lea` and `push` operands).
pub const CASTER_START_0393: Sig = Sig {
    len: 689,
    hash: 0x8794_6a44_5809_aef2,
    calls: &[
        (0x31, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x299, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        (0x2a9, Callee::Any),
    ],
    imports: &[],
};
