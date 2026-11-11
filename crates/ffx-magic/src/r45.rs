//! Round 45's own ports (own rows of `magic_0250` .. `magic_0449`): the
//! DLLs' own code written out.

use crate::m0018::st0_f32;
use crate::ob_model::wrap_once;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0394`'s summon start (0x10005640, 1159 bytes; a table entry, no
/// arguments; a /GS `ebp` frame: the matrix `M` at `ebp - 0x64`, the
/// turned point at `ebp - 0x24`, the offset `V` at `ebp - 0x14`). With
/// the state block `S = *G_S`:
/// - `S` starts as an effect object
///   ([`start_object_first`](crate::o6_fam_object::start_object_first),
///   scale `K`; below `sub esp, 0x10`, four words it never reads); its
///   place `S + 0xd0..` = `S + 0xb40..` with `y` = `S + 0xb34` less `K1`,
///   `+0xe8` 8.0, `+0xc0` 0.251, the yaw `+0xc4` = `S + 0xc84` plus `PI`
///   wrapped once; the record `S + 0xc50` (callback `CB`, `D`) into
///   `S + 0x110`; slot 239 `(*G_A, -5, 0)`;
/// - when `*G_ON` is 1: slot 238 `(b, 1)` (`b` the selection's byte),
///   the selected character's root matrix into `S + 0xab0`
///   ([`root`](crate::m0394::root)), `S + 0xaf0` 0, `S + 0xaf4` = `K2`
///   less slot 333 `(T[b])` wrapped once, `+0xaf8`, `+0xafc` 0; the
///   character's place (slot 262) into `S + 0xb00..` (`w` 1.0); `S +
///   0xb10..` = `(0, -4, 0, 1)` moved by `V = (0, 0, -10, 1)` turned by X 0
///   and Y `S + 0xc84` (slots 235, 328, 329, 229, 248), `+0xb20`,
///   `+0xb28` its `x` and `z` times `K3`, `+0xb24` 0.1, `+0xb2c` 0,
///   `+0xc90` 0x41; slot 295 `(b, 0)`; a task node (the turning step)
///   appended to the task list;
/// - the light record `S + 0xb60` (colour words, sizes 0x14, u16 `+0x98`
///   0x48) on a task node of its own appended after the last one;
/// - slot 531 `(*G_A, S + 0xb50)`, `S + 0xc88` = 0xe, slot 719 `(X, 9,
///   1)` and slot 306 `(7, 2 or 1, that, X, 9, 1)` (2 when `*G_ON` is 0).
///
/// Returns slot 306's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence of steps
pub fn summon_start_0394<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let (mat, out, v) = (
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
    );
    // after the locals and `push ebx; push esi; push edi`
    let base = ebp.wrapping_sub(0x74);
    let s = g.read::<u32>(w(0x1d));
    let mut xmm = fr.xmm;
    xmm[3] = u128::from(g.read::<u32>(w(0x14)));
    let at = img.rel32_target(f + 0x27);
    let call = Frame {
        at,
        esp: base.wrapping_sub(0x14),
        edx: s,
        xmm,
        ..*fr
    };
    crate::o6_fam_object::start_object_first(g, img, at, &call);
    for i in 0..4u32 {
        let x = g.read::<u32>(s + 0xb40 + 4 * i);
        g.write::<u32>(s + 0xd0 + 4 * i, x);
    }
    let y = sub(g.read::<f32>(s + 0xb34), k(g, 0x6f));
    g.write::<f32>(s + 0xd4, y);
    g.write::<u32>(s + 0xe8, 0x4100_0000);
    g.write::<u32>(s + 0xc0, 0x3e80_adfd);
    let yaw = add(g.read::<f32>(s + 0xc84), k(g, 0x9e));
    let yaw = wrap_once(g, img, f, yaw, [0xa5, 0x4d, 0xb5, 0x4d]);
    g.write::<f32>(s + 0xc4, yaw);
    let rec = s + 0xc50;
    for d in [0x18u32, 0x14, 0x10, 0x1c] {
        g.write::<u32>(rec + d, 0);
    }
    g.write::<u32>(rec + 0x20, w(0xef));
    g.write::<u32>(rec + 0x24, w(0xf6));
    g.write::<u32>(rec + 0x28, 0);
    g.write::<u32>(rec + 0x2c, 0);
    g.write::<u16>(rec + 0x30, 0);
    g.write::<u32>(s + 0x110, rec);
    let a = g.read::<u32>(w(0x124));
    g.call(t(g, 0x115, 239), 0, &[a, 0xffff_fffb, 0]);
    let take = img.rel32_target(f + 0x351);
    let (first, last) = (w(0x357), w(0x36d));
    let last_node = if g.read::<u32>(w(0x12f)) == 1 {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x13b))));
        let mut st = Pushes::default();
        st.push_all(&[1, b]);
        st.call(g, t(g, 0x146, 238), 0);
        crate::m0394::root(
            g,
            img,
            img.rel32_target(f + 0x159),
            s + 0xab0,
            base.wrapping_sub(0xc),
        );
        g.write::<u32>(s + 0xaf0, 0);
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x168))));
        st.push(g.read::<u32>(w(0x172).wrapping_add(4 * b)));
        let rot = st0_f32(st.call(g, t(g, 0x177, 333), 0).st0);
        st.pop(3);
        let x = sub(k(g, 0x18a), rot);
        let x = wrap_once(g, img, f, x, [0x199, 0x1a3, 0x1ad, 0x1ba]);
        g.write::<f32>(s + 0xaf4, x);
        g.write::<u32>(s + 0xaf8, 0);
        g.write::<u32>(s + 0xafc, 0);
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x1db))));
        let chr = g.read::<u32>(w(0x1eb).wrapping_add(4 * b));
        let p = s + 0xb00;
        st.push_all(&[p + 8, p + 4, p, chr]);
        st.call(g, t(g, 0x1f8, 262), chr);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        let yaw = g.read::<u32>(s + 0xc84);
        let q = s + 0xb10;
        g.write::<u32>(q + 4, 0xc080_0000);
        g.write::<u32>(q, 0);
        g.write::<u32>(q + 8, 0);
        g.write::<u32>(q + 0xc, 0);
        for (i, x) in (0u32..).zip([0, 0, 0xc120_0000, 0x3f80_0000]) {
            g.write::<u32>(v + 4 * i, x);
        }
        st.push(mat);
        st.call(g, t(g, 0x21a, 235), 0);
        st.pop(4);
        st.set_top(0);
        st.push_all(&[mat, mat]);
        st.call(g, t(g, 0x281, 328), 0);
        st.pop(2);
        st.set_top(yaw);
        st.push_all(&[mat, mat]);
        st.call(g, t(g, 0x2a0, 329), 0);
        st.push_all(&[v, mat, out]);
        g.write::<u32>(v + 0xc, 0x3f80_0000);
        st.call(g, t(g, 0x2b9, 229), 0);
        st.push_all(&[out, q, q]);
        st.call(g, t(g, 0x2d1, 248), 0);
        g.write::<u32>(q + 0xc, 0x3f80_0000);
        let x = mul(g.read::<f32>(q), k(g, 0x2ee));
        g.write::<u32>(s + 0xb24, 0x3dcc_cccd);
        g.write::<u32>(s + 0xb2c, 0);
        g.write::<f32>(s + 0xb20, x);
        let z = mul(g.read::<f32>(s + 0xb18), k(g, 0x31a));
        g.write::<u32>(s + 0xc90, 0x41);
        g.write::<f32>(s + 0xb28, z);
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x333))));
        st.push_all(&[0, b]);
        st.call(g, t(g, 0x33c, 295), 0);
        // the turning step's node, appended to the task list
        let node = crate::sa_pool::take(g, img, take, w(0x34c));
        g.write::<u8>(node + 0x18, 1);
        g.write::<u32>(node + 4, 0);
        if g.read::<u32>(first) == 0 {
            g.write::<u32>(node, 0);
            g.write::<u32>(first, node);
        } else {
            let prev = g.read::<u32>(last);
            g.write::<u32>(node, prev);
            g.write::<u32>(prev + 4, node);
        }
        g.write::<u32>(node + 8, node);
        node
    } else {
        g.read::<u32>(w(0x38e))
    };
    // the light record on a node of its own, after `last_node`
    let r = s + 0xb60;
    g.write::<u32>(r + 0x80, 0x8080_8080);
    g.write::<u32>(r + 0x84, 0x0080_8080);
    g.write::<u32>(r + 0x88, 0);
    g.write::<u16>(r + 0x98, 0x48);
    g.write::<u32>(r + 0x90, 0x14);
    g.write::<u32>(r + 0x8c, 0x14);
    g.write::<u32>(r + 0x94, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x3e3), w(0x399));
    g.write::<u32>(node + 0x14, r);
    g.write::<u32>(node + 4, 0);
    if g.read::<u32>(w(0x3e9)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x408), node);
    } else {
        g.write::<u32>(node, last_node);
        g.write::<u32>(last_node + 4, node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(0x410), node);
    let a = g.read::<u32>(w(0x422));
    g.call(t(g, 0x41c, 531), 0, &[a, s + 0xb50]);
    let on = g.read::<u32>(w(0x438));
    g.write::<u32>(s + 0xc88, 0xe);
    let x = w(0x455);
    let r = g.call(t(g, 0x42f, 719), 0, &[x, 9, 1]).eax;
    let kind = if on == 0 { 2 } else { 1 };
    // call width: 6
    let eax = g.call(t(g, 0x468, 306), 0, &[7, kind, r, x, 9, 1]).eax;
    Ret::eax(eax)
}

/// [`summon_start_0394`] (the `rel32`s: the object start, the root
/// matrix, two pool takes, the cookie check).
pub const SUMMON_START_0394: Sig = Sig {
    len: 1159,
    hash: 0xce31_c753_0cb3_4982,
    calls: &[
        (
            0x27,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
        ),
        (0x159, Callee::Sig(&crate::m0394::ROOT)),
        (0x351, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x3e3, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x47f, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0348`'s object start (0x10006940, 193 bytes; a table entry, no
/// arguments): with the state block `S = *G_S`, the object `O = S +
/// 0x260` starts ([`start_object_anim`](crate::s4_misc::start_object_anim),
/// scale `*K`, index 2, value 0; below `sub esp, 8`, two words it never
/// reads) at `S + 0xd20..` with `y` less `K1` and the yaw `+0xc4` = `S +
/// 0xd14`; the record `R = S + 0xce78` goes on a task node
/// ([`record_node`](crate::o4_small::record_node)), its halfword `+0x2c`
/// gets bits 0 and 1, `+0x4` 0, `+0x8` 0.2, `+0xc`, `+0x10`, `+0x18` 0,
/// `+0x14` 5.0, `+0x24` 30. Returns the node.
pub fn object_start_0348<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x4));
    let s = g.read::<u32>(w(0xc));
    let o = s + 0x260;
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x20), o, scale, 2, 0);
    for i in 0..4u32 {
        let x = g.read::<u32>(s + 0xd20 + 4 * i);
        g.write::<u32>(o + 0xd0 + 4 * i, x);
    }
    let y = sub(g.read::<f32>(o + 0xd4), g.read::<f32>(w(0x60)));
    let yaw = g.read::<u32>(s + 0xd14);
    g.write::<u32>(o + 0xc4, yaw);
    g.write::<f32>(o + 0xd4, y);
    let r = s + 0xce78;
    let rn = Frame {
        at: img.rel32_target(f + 0x84),
        esp: fr.esp.wrapping_sub(0xc),
        ecx: r,
        ..*fr
    };
    let node = crate::o4_small::record_node(g, img, &rn).eax;
    let bits = g.read::<u16>(r + 0x2c);
    g.write::<u16>(r + 0x2c, bits | 3);
    g.write::<u32>(r + 4, 0);
    g.write::<u32>(r + 8, 0x3e4c_cccd);
    g.write::<u32>(r + 0xc, 0);
    g.write::<u32>(r + 0x10, 0);
    g.write::<u32>(r + 0x14, 0x40a0_0000);
    g.write::<u32>(r + 0x18, 0);
    g.write::<u32>(r + 0x24, 0x1e);
    Ret::eax(node)
}

/// [`object_start_0348`] (the `rel32`s: the object start, the record
/// node).
pub const OBJECT_START_0348: Sig = Sig {
    len: 193,
    hash: 0x27c6_186d_86f3_4fff,
    calls: &[
        (0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x84, Callee::Sig(&crate::o4_small::RECORD_NODE_0348_29A0)),
    ],
    imports: &[],
};

/// `magic_0251`'s sparks (0x10005490, 1986 bytes): `magic_0250`'s
/// ([`spawn_0250`](crate::r43::spawn_0250)) with the second pool's `+0x34`
/// 2.0 instead of 1.0 (the immediate is read from the code).
pub const SPAWN_0251: Sig = Sig {
    len: 1986,
    hash: 0x2971_b6b3_f0d9_57a3,
    calls: crate::r43::SPAWN_0250.calls,
    imports: &[],
};

/// `magic_0392`'s actors' pass (0x100014a0, 152 bytes; internal, no
/// arguments; an aligned /GS frame, the point `M` at `(ebp & !7) - 0x14`):
/// unless `*G_OFF` is set, an actor `i = *I` below 32 gets slot 217 `(i,
/// &M)` and slot 255 `(i, &M)`; slot 211 `(m, 0)` (`m` = `*Q`, read
/// first); then each set word `a` of the 16-word table `T` registers its
/// parts ([`register_loop`](crate::o4_fam2::register_loop), `ecx` = `a`).
/// Returns the last call's `eax`.
pub fn actors_pass_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let m_at = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x14);
    let off = g.read::<u32>(w(0x16));
    let m = g.read::<u32>(w(0x1f));
    let mut st = Pushes::default();
    if off == 0 {
        let i = g.read::<u32>(w(0x27));
        if i.cast_signed() < 0x20 {
            st.push_all(&[m_at, i]);
            st.call(g, slot_target(g, w(0x36), 217), 0);
            st.push_all(&[m_at, i]);
            st.call(g, slot_target(g, w(0x49), 255), 0);
            st.pop(4);
        }
    }
    st.push_all(&[0, m]);
    let mut eax = st.call(g, slot_target(g, w(0x5a), 211), 0).eax;
    let reg = img.rel32_target(f + 0x78);
    let mut p = w(0x6d);
    while p < w(0x81) {
        let a = g.read::<u32>(p);
        if a != 0 {
            eax = crate::o4_fam2::register_loop(g, img, reg, a);
        }
        p += 4;
    }
    Ret::eax(eax)
}

/// [`actors_pass_0392`] (the `rel32`s: the parts' registration, the
/// cookie check).
pub const ACTORS_PASS_0392: Sig = Sig {
    len: 152,
    hash: 0xb4c8_90c5_bec2_84f3,
    calls: &[
        (0x78, Callee::Sig(&crate::o4_fam2::REGISTER_LOOP_0392_4830)),
        (0x90, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0392`'s task step (0x100013c0, 148 bytes):
/// [`task_step_pass`](crate::m0449::task_step_pass) with this DLL's
/// actors' pass ([`actors_pass_0392`]).
pub const TASK_STEP_PASS_0392: Sig = Sig {
    len: 148,
    hash: 0x244d_f820_b97c_d0ae,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x33, Callee::Sig(&crate::sa_script::RUN)),
        (0x3a, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x51, Callee::Sig(&ACTORS_PASS_0392)),
        (0x5c, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0385`'s shard burst (0x10004b00, 835 bytes; a callback, no
/// arguments; an aligned /GS frame, `E` its base after the pushes):
/// - while `*G_WAIT` is positive it counts down (the new value returned);
/// - else while `*G_N` is positive: `*G_N` drops and `*G_TICK` grows;
///   below 4 the tick is returned, at 4 it restarts at 0 and a burst
///   fires (otherwise `*G_N`, at most 0, is returned);
/// - the burst: the actors' mean position `C`
///   ([`mean_position`](crate::sb_misc::mean_position)) at `E + 0x18`, its
///   `y` replaced by slot 12 `(T[b])`'s height less `K1` (`b` the
///   selection's byte); the caster's place `P` (slot 262 `(*G_C, ...)`,
///   at `E + 0x28`); the yaw `K2 - atan2(P.z - C.z, P.x - C.x)`
///   (`_CIatan2` of the doubles) wrapped once; the matrix `M1` (slot 235,
///   `M1[10]` 0.5, slot 329 by the yaw);
/// - three shards, each from the 0x140 pool
///   ([`pool_take_140`](crate::sb_misc::pool_take_140); none skips it):
///   `V = (0, 0, frnd(58), 1)`, the matrix `M2` (slot 235, slot 329 by
///   `frnd(pi)`), `M2 = M2 M1` (slot 231), `W = M2 V` (slot 229); the
///   object starts ([`start_object_at`](crate::s4_misc::start_object_at),
///   scale `K3`, index 0, owners 0) with the angles `frnd(0.174)`,
///   `frnd(2 pi)`, `frnd(0.174)` and the place `W + C` (`y` the height).
///
/// Returns the last `frnd`'s `eax` (0 when the last take found none).
#[allow(clippy::too_many_lines)] // the timers, the aim and the three shards
pub fn shard_burst_0385<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let wait = g.read::<u32>(w(0x1b));
    if wait.cast_signed() > 0 {
        let v = wait.wrapping_sub(1);
        g.write::<u32>(w(0x27), v);
        return Ret::eax(v);
    }
    let n = g.read::<u32>(w(0x40));
    if n.cast_signed() <= 0 {
        return Ret::eax(n);
    }
    g.write::<u32>(w(0x4e), n.wrapping_sub(1));
    let tick = g.read::<u32>(w(0x53)).wrapping_add(1);
    g.write::<u32>(w(0x59), tick);
    if tick.cast_signed() < 4 {
        return Ret::eax(tick);
    }
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xe0);
    let (c, p, v, out, m2, m1) = (e + 0x18, e + 0x28, e + 0x38, e + 0x48, e + 0x58, e + 0x98);
    g.write::<u32>(w(0x6c), 0);
    crate::sb_misc::mean_position(g, img, img.rel32_target(f + 0x75), e.wrapping_sub(4), c);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x7a))));
    let chr = g.read::<u32>(w(0x84).wrapping_add(4 * b));
    let h = st0_f32(g.call(t(g, 0x89, 12), 0, &[chr]).st0);
    let height = sub(h, k(g, 0xa0));
    g.write::<f32>(e + 0x14, height);
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let caster = g.read::<u32>(w(0xba));
    // call width: 5 (slot 12's word under it)
    g.call(t(g, 0xb4, 262), 0, &[caster, p, p + 4, p + 8, chr]);
    let dx = sub(g.read::<f32>(p), g.read::<f32>(c));
    g.write::<f32>(p + 4, height);
    let dz = sub(g.read::<f32>(p + 8), g.read::<f32>(c + 8));
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    // fstp qword; cvtpd2ps
    #[allow(clippy::cast_possible_truncation)]
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let yaw = sub(k(g, 0x140), a);
    let yaw = wrap_once(g, img, f, yaw, [0x157, 0x148, 0x16d, 0x148]);
    g.call(t(g, 0x189, 235), 0, &[m1]);
    g.write::<u32>(m1 + 0x28, 0x3f00_0000);
    g.call(t(g, 0x1aa, 329), 0, &[m1, m1, yaw.to_bits()]);
    let take = img.rel32_target(f + 0x1d1);
    let frnd = |g: &mut G, o: u32, x: u32| g.call(t(g, o, 314), 0, &[x]);
    let mut eax = 0;
    for _ in 0..3 {
        let obj = crate::sb_misc::pool_take_140(g, img, take);
        if obj == 0 {
            eax = 0;
            continue;
        }
        let r = st0_f32(frnd(g, 0x1e2, 0x4268_0000).st0);
        g.write::<f32>(v + 8, r);
        // call width: 2 (frnd's word under it)
        g.call(t(g, 0x1ff, 235), 0, &[m2, 0x4268_0000]);
        let r2 = st0_f32(frnd(g, 0x20c, 0x4049_0fdb).st0);
        g.call(t(g, 0x238, 329), 0, &[m2, m2, r2.to_bits()]);
        // call width: 6 (slot 329's words under it)
        g.call(t(g, 0x257, 231), 0, &[m2, m1, m2, m2, m2, r2.to_bits()]);
        // call width: 9 (the earlier words stay)
        g.call(
            t(g, 0x273, 229),
            0,
            &[out, m2, v, m2, m1, m2, m2, m2, r2.to_bits()],
        );
        let scale = k(g, 0x283);
        crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0x294), obj, scale, 0, 0, 0);
        // the angles: frnd(0.174), frnd(2 pi), frnd(0.174)
        for (d, x, o) in [
            (0xc0u32, 0x3e32_b8c3u32, 0x299),
            (0xc4, 0x40c9_0fdb, 0x2b6),
            (0xc8, 0x3e32_b8c3, 0x2d0),
        ] {
            let r = frnd(g, o, x);
            eax = r.eax;
            g.write::<f32>(obj + d, st0_f32(r.st0));
        }
        let x = add(g.read::<f32>(out), g.read::<f32>(c));
        g.write::<f32>(obj + 0xd0, x);
        let y = add(g.read::<f32>(out + 4), g.read::<f32>(e + 0x14));
        g.write::<f32>(obj + 0xd4, y);
        let z = add(g.read::<f32>(out + 8), g.read::<f32>(c + 8));
        g.write::<f32>(obj + 0xd8, z);
    }
    Ret::eax(eax)
}

/// [`shard_burst_0385`] (the `rel32`s: the cookie checks, the mean
/// position, `_CIatan2`, the pool take, the object start).
pub const SHARD_BURST_0385: Sig = Sig {
    len: 835,
    hash: 0xf389_9218_bab4_9b72,
    calls: &[
        (0x37, Callee::Any),
        (0x75, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x12e, Callee::Import("_CIatan2")),
        (0x1d1, Callee::Sig(&crate::sb_misc::POOL_TAKE_140)),
        (0x294, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x33b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0297`'s summon start (0x10007e30, 490 bytes; a table entry,
/// cdecl `(actor)`): [`summon_start`](crate::s4_summon::summon_start)'s
/// `magic_0389` copy (the actor kept in `G_A`, the statics reset:
/// [`statics_init`](crate::sa_summon::statics_init)) with its environment
/// taken from the game: slot 803 `(0xfa000, 0x10)` into `G_ENV`, `G_ENV2`
/// = that plus 0x80000 (slot 710 loads 0x80000 bytes), the ring stores
/// one fewer, slot 803's two words left on the stack under the later
/// calls. Returns 0.
pub fn summon_start_0297<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    g.write::<u32>(w(0x8), fr.arg(g, 0));
    // push ebp; push esi; call
    crate::sa_summon::statics_init(g, img, img.rel32_target(f + 0xd), fr.esp.wrapping_sub(12));
    let mut s = Pushes::default();
    s.push_all(&[0x10, 0xf_a000]);
    let env = s.call(g, t(g, 0x12, 803), 0).eax;
    g.write::<u32>(w(0x26), env);
    g.write::<u32>(w(0x30), env.wrapping_add(0x8_0000));
    for (dst, src) in [(0x3b, 0x3f), (0x45, 0x49), (0x4f, 0x53), (0x59, 0x5d)] {
        g.write::<u32>(w(dst), w(src));
    }
    g.write::<u32>(w(0x63), 1);
    g.write::<u32>(w(0x6d), w(0x71));
    g.write::<u16>(w(0x78), 0x1ff);
    g.write::<u32>(w(0x80), w(0x84));
    g.write::<u32>(w(0x8a), w(0x8e));
    g.write::<u32>(w(0x94), 0);
    g.write::<u32>(w(0x9e), w(0xa2));
    g.write::<u16>(w(0xa9), 0xff);
    g.write::<u32>(w(0xb1), w(0xb5));
    g.write::<u32>(w(0xbb), w(0xbf));
    g.write::<u32>(w(0xc5), 0);
    s.push(w(0xd4));
    s.call(g, t(g, 0x35, 718), 0);
    s.push_all(&[0, 8, w(0xea)]);
    let handle = s.call(g, t(g, 0xdb, 719), 0).eax;
    let lock = g.read::<u32>(w(0xf2));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xfb), handle);
    s.push_all(&[0, w(0x100), handle]);
    g.write::<u32>(lock, 1);
    s.call(g, t(g, 0x106, 711), 0);
    let lock = g.read::<u32>(w(0x119));
    g.write::<u32>(lock, saved);
    let handle = g.read::<u32>(w(0x130));
    s.push_all(&[0x8_0000, w(0x12a), handle, w(0x13b)]);
    s.call(g, t(g, 0x125, 710), 0);
    s.push_all(&[2, 1000, 0, w(0x156)]);
    s.call(g, t(g, 0x142, 222), 0);
    s.pop(17);
    s.push_all(&[2, 1000, 0, w(0x174)]);
    s.call(g, t(g, 0x15d, 221), 0);
    let tail = g.read::<u32>(w(0x17b));
    for (dst, src) in [(0x181, 0x185), (0x18b, 0x18f), (0x195, 0x199)] {
        g.write::<u32>(w(dst), w(src));
    }
    g.write::<u32>(w(0x19e), tail);
    g.write::<u32>(w(0x1a4), w(0x1a8));
    g.write::<u32>(tail.wrapping_add(4), w(0x1af));
    let flags = g.read::<u32>(w(0x1b8));
    g.write::<u32>(w(0x1b8), flags & !1);
    g.write::<u32>(w(0x1bf), w(0x1c3));
    for o in [0x1c9, 0x1d3, 0x1dd] {
        g.write::<u32>(w(o), 0);
    }
    Ret::eax(0)
}

/// [`summon_start_0297`] (the `rel32`: the statics' reset).
pub const SUMMON_START_0297: Sig = Sig {
    len: 490,
    hash: 0x9358_d935_c847_439b,
    calls: &[(0xd, Callee::Sig(&crate::sa_summon::STATICS_INIT))],
    imports: &[],
};

/// `magic_0348`'s light set-up (0x100070b0, 987 bytes; a table entry, no
/// arguments): with the state block `S = *G_S`, the object `O = S +
/// 0x390` starts ([`start_object_anim`](crate::s4_misc::start_object_anim),
/// scale `*K`, index 3, value 0) at `S + 0xd20..` with `y` less `K2` and
/// the yaw `+0xc4` = `S + 0xd14`; seven light records `L` (0x38 apart from
/// `S + 0xcf48`, the last at `S + 0xd098`) get `L + 0x10..` = `L..`, `L +
/// 0x20..` = their colour (`S + 0xd0d0`, `+0xd0e0`, `+0xd0f0`, `+0xd130`
/// .. `+0xd160`), `+0x34` and `+0x30` 0x50; a scale task over `S +
/// 0xcf10` (80 frames, scales `K3` and `K2`:
/// [`scale_task_edx`](crate::s4_small::scale_task_edx)); slot 8 `(*G_D,
/// 2)`. Returns slot 8's `eax`.
pub fn light_setup_0348<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x4));
    let s = g.read::<u32>(w(0xc));
    let o = s + 0x390;
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x20), o, scale, 3, 0);
    for i in 0..4u32 {
        let x = g.read::<u32>(s + 0xd20 + 4 * i);
        g.write::<u32>(o + 0xd0 + 4 * i, x);
    }
    let k2 = g.read::<u32>(w(0x6c));
    let y = sub(g.read::<f32>(o + 0xd4), f32::from_bits(k2));
    let yaw = g.read::<u32>(s + 0xd14);
    g.write::<u32>(o + 0xc4, yaw);
    g.write::<f32>(o + 0xd4, y);
    for (l, colour) in [
        (0xcf48u32, 0xd0d0u32),
        (0xcf80, 0xd0e0),
        (0xcfb8, 0xd0f0),
        (0xcff0, 0xd130),
        (0xd028, 0xd140),
        (0xd060, 0xd150),
        (0xd098, 0xd160),
    ] {
        let l = s + l;
        for i in 0..4u32 {
            let x = g.read::<u32>(l + 4 * i);
            g.write::<u32>(l + 0x10 + 4 * i, x);
        }
        for i in 0..4u32 {
            let x = g.read::<u32>(s + colour + 4 * i);
            g.write::<u32>(l + 0x20 + 4 * i, x);
        }
        g.write::<u32>(l + 0x34, 0x50);
        g.write::<u32>(l + 0x30, 0x50);
    }
    let mut xmm = fr.xmm;
    xmm[1] = u128::from(g.read::<u32>(w(0x34a)));
    xmm[2] = u128::from(k2);
    let st = Frame {
        at: img.rel32_target(f + 0x3bf),
        esp: fr.esp.wrapping_sub(0x10),
        ecx: s + 0xcf10,
        edx: 0x50,
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_edx(g, img, &st);
    let d = g.read::<u32>(w(0x3cc));
    let eax = g.call(slot_target(g, w(0x3c4), 8), 0, &[d, 2]).eax;
    Ret::eax(eax)
}

/// [`light_setup_0348`] (the `rel32`s: the object start, the scale
/// task).
pub const LIGHT_SETUP_0348: Sig = Sig {
    len: 987,
    hash: 0xbfe9_7227_9234_828c,
    calls: &[
        (0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x3bf, Callee::Sig(&crate::s4_small::SCALE_TASK_EDX)),
    ],
    imports: &[],
};

/// `magic_0348`'s packed sprite (0x10006a10, 291 bytes; a task callback,
/// cdecl `(node)`; a /GS frame of 0x8c bytes): `magic_0350`'s
/// ([`packed_0350`](crate::r38::packed_0350)) with one counter: with the
/// state `S = *G_S`, while `S + 0xd17c` is not above 0 the node goes back
/// to the pool (the give-back's `eax` returned); else it counts down and
/// the fade is the new value times 80 over `S + 0xd178` (a zero divisor
/// gives 0, where the original faults); the angle `S + 0xd174 -= DA`,
/// plus `T` below 0; slot 719 `(P, 2, 6)` gives the descriptor `(lo,
/// hi)`; slot 235 `(M)` (`ebp - 0x44`), slot 210 `(*G_O2, M2)` (`ebp -
/// 0x84`); the texture drawn
/// ([`packed_texture`](crate::s5_misc::packed_texture), `ecx` = `*G_O`
/// read before slot 719, `(-, M, lo, hi, 0, angle, n)`; the first word is
/// the `ecx` slot 210 leaves, which the callee does not read: 0 here).
/// Returns the draw's `eax`.
pub fn packed_0348<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x15));
    let left = g.read::<i32>(s + 0xd17c);
    if left <= 0 {
        let node = fr.arg(g, 0);
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x2f),
            node,
        ));
    }
    let left = left - 1;
    g.write::<i32>(s + 0xd17c, left);
    let n = left
        .wrapping_mul(80)
        .checked_div(g.read::<i32>(s + 0xd178))
        .unwrap_or(0);
    let angle = sub(g.read::<f32>(s + 0xd174), k(g, 0x61));
    g.write::<f32>(s + 0xd174, angle);
    // comiss 0, a; jbe: only below 0 (ordered)
    if 0.0 > angle {
        g.write::<f32>(s + 0xd174, add(angle, k(g, 0x7f)));
    }
    let o = g.read::<u32>(w(0x99));
    // call width: 3
    let d = g
        .call(slot_target(g, w(0x8c), 719), 0, &[w(0xa4), 2, 6])
        .eax;
    let (lo, hi) = (g.read::<u32>(d), g.read::<u32>(d + 4));
    let ebp = fr.esp.wrapping_sub(4);
    let (m, m2) = (ebp.wrapping_sub(0x44), ebp.wrapping_sub(0x84));
    g.call(slot_target(g, w(0xb4), 235), 0, &[m]);
    let a = g.read::<u32>(w(0xce));
    g.call(slot_target(g, w(0xc8), 210), 0, &[a, m2]);
    let angle = g.read::<u32>(s + 0xd174);
    // the frame, ebx/esi/edi, n, 8 bytes and four pushes, the return
    // address
    let esp = ebp.wrapping_sub(0xb8);
    for (i, v) in (1u32..).zip([0, m, lo, hi, 0, angle, n.cast_unsigned()]) {
        g.write::<u32>(esp + 4 * i, v);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x10b),
        esp,
        ecx: o,
        ..Frame::default()
    };
    crate::s5_misc::packed_texture(g, img, &callee)
}

/// [`packed_0348`] (the `rel32`s: the give-back, the cookie checks, the
/// texture).
pub const PACKED_0348: Sig = Sig {
    len: 291,
    hash: 0x4659_91ee_202d_087d,
    calls: &[
        (0x2f, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x39, Callee::Any),
        (0x10b, Callee::Sig(&crate::s5_misc::PACKED_TEXTURE)),
        (0x11b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0348`'s glow start (0x10006810, 293 bytes; a table entry, no
/// arguments; a /GS frame of 0x14 bytes): the state block `S = *G_S`
/// starts as an effect object
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale `*K`,
/// index 0, value 0) at `S + 0xd20..` with `y` less `K1` and the yaw
/// `+0xc4` = `S + 0xd14`; the light record `L = S + 0xd098` gets `L +
/// 0x10..` = `L..`, `L + 0x20..` = `(0, 0, 0, a)` and `+0x34`, `+0x30` 30,
/// where `a` is a frame word the original never writes (`[ebp - 8]`);
/// then a scale task over `S + 0xced8` (30 frames, scales `K2`, `K3`:
/// [`scale_task_edx`](crate::s4_small::scale_task_edx)). Returns the
/// task's node.
pub fn glow_start_0348<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x14));
    let s = g.read::<u32>(w(0x1b));
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x29), s, scale, 0, 0);
    for i in 0..4u32 {
        let x = g.read::<u32>(s + 0xd20 + 4 * i);
        g.write::<u32>(s + 0xd0 + 4 * i, x);
    }
    let yaw = g.read::<u32>(s + 0xd14);
    let y = sub(g.read::<f32>(s + 0xd4), g.read::<f32>(w(0x6f)));
    g.write::<u32>(s + 0xc4, yaw);
    g.write::<f32>(s + 0xd4, y);
    // junk read: [ebp-0x8] never written; 0 (brief 2026-10-24)
    let a = 0u32;
    let l = s + 0xd098;
    for i in 0..4u32 {
        let x = g.read::<u32>(l + 4 * i);
        g.write::<u32>(l + 0x10 + 4 * i, x);
    }
    for (i, x) in (0u32..).zip([0, 0, 0, a]) {
        g.write::<u32>(l + 0x20 + 4 * i, x);
    }
    g.write::<u32>(l + 0x34, 0x1e);
    g.write::<u32>(l + 0x30, 0x1e);
    let mut xmm = fr.xmm;
    xmm[1] = u128::from(g.read::<u32>(w(0x7f)));
    xmm[2] = u128::from(g.read::<u32>(w(0x77)));
    let st = Frame {
        at: img.rel32_target(f + 0x10f),
        esp: fr.esp.wrapping_sub(0x24),
        ecx: s + 0xced8,
        edx: 0x1e,
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_edx(g, img, &st)
}

/// [`glow_start_0348`] (the `rel32`s: the object start, the scale task,
/// the cookie check).
pub const GLOW_START_0348: Sig = Sig {
    len: 293,
    hash: 0xdf30_9be7_4ec9_107b,
    calls: &[
        (0x29, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x10f, Callee::Sig(&crate::s4_small::SCALE_TASK_EDX)),
        (0x11d, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0348`'s orbs start (0x10006e40, 609 bytes; a table entry, no
/// arguments; a /GS frame of 0x14 bytes): with the state block `S =
/// *G_S`, five objects `O = S + 0x720 + 0x130 i` start
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale `*K`,
/// index 4, value `A[i] - 45`), each with the yaw `+0xc4` = `S + 0xd14`,
/// the callback `+0x104` = `CB` and `+0x120` = `B[i]` (`A`, `B` word
/// tables in the DLL's data); four light records `L` (`S + 0xcfb8`,
/// `+0xcff0`, `+0xd028`, `+0xd060`) get `L + 0x10..` = `L..` and a colour
/// at `L + 0x20..` (the first `L..` = `(2.79, 0)` set first, colour
/// `(0.7, 0, u, a)`; then `(0, 0, 0, a)` twice and `(1.5, 1.4, 1.3, a)`,
/// where `u` and `a` are frame words the original never writes, `[ebp -
/// 0xc]` and `[ebp - 8]`), and their counts (100, 30, 30, 100 in `+0x34`
/// and `+0x30`); slot 8 `(*G_D, 0)`. Returns slot 8's `eax`.
pub fn orbs_start_0348<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x13));
    let start = img.rel32_target(f + 0x3b);
    for i in 0..5u32 {
        let o = s + 0x720 + 0x130 * i;
        let value = g.read::<u32>(w(0x23) + 4 * i).wrapping_sub(0x2d);
        let scale = g.read::<f32>(w(0x2b));
        crate::s4_misc::start_object_anim(g, img, start, o, scale, 4, value);
        let yaw = g.read::<u32>(s + 0xd14);
        g.write::<u32>(o + 0xc4, yaw);
        g.write::<u32>(o + 0x104, w(0x51));
        let b = g.read::<u32>(w(0x57) + 4 * i);
        g.write::<u32>(o + 0x120, b);
    }
    // junk read: [ebp-0xc] never written; 0 (brief 2026-10-24)
    let u = 0u32;
    // junk read: [ebp-0x8] never written; 0 (brief 2026-10-24)
    let a = 0u32;
    g.write::<u32>(s + 0xcfb8, 0x4032_b8c3);
    g.write::<u32>(s + 0xcfbc, 0);
    let lights: [(u32, [u32; 4], u32); 4] = [
        (0xcfb8, [0x3f32_b8c0, 0, u, a], 0x64),
        (0xcff0, [0, 0, 0, a], 0x1e),
        (0xd028, [0, 0, 0, a], 0x1e),
        (0xd060, [0x3fc0_0000, 0x3fb3_3333, 0x3fa6_6666, a], 0x64),
    ];
    for (l, colour, n) in lights {
        let l = s + l;
        for i in 0..4u32 {
            let x = g.read::<u32>(l + 4 * i);
            g.write::<u32>(l + 0x10 + 4 * i, x);
        }
        for (i, x) in (0u32..).zip(colour) {
            g.write::<u32>(l + 0x20 + 4 * i, x);
        }
        g.write::<u32>(l + 0x34, n);
        g.write::<u32>(l + 0x30, n);
    }
    let d = g.read::<u32>(w(0x247));
    let eax = g.call(slot_target(g, w(0x202), 8), 0, &[d, 0]).eax;
    Ret::eax(eax)
}

/// [`orbs_start_0348`] (the `rel32`s: the object start, the cookie
/// check).
pub const ORBS_START_0348: Sig = Sig {
    len: 609,
    hash: 0x95c4_8308_7da2_046a,
    calls: &[
        (0x3b, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x259, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0431`'s script end (0x10001690, 135 bytes; a task callback,
/// cdecl `(node)`): the block `BLK` (0x30 bytes) cleared (`memset`), bit 7
/// of its halfword `+0x2` set and `*G_SRC` copied into `+0x14`; two
/// scripts run ([`script_run`](crate::sb_misc::script_run), `ecx` the
/// contexts `C1`, `C2`) and two motions
/// ([`motion_script`](crate::s4_misc::motion_script), `M1`, `M2`). Unless
/// the first of each reports done, the second motion's `eax` is returned;
/// then `S + 0xd10` = 1 (`S` = `*G_S`, read first), the actors' show and
/// sounds ([`show_and_sounds`](crate::m0431::show_and_sounds)) and the
/// node given back to its pool
/// ([`give_back`](crate::sa_pool::give_back), a tail jump with `ecx` =
/// `node`), whose result is returned.
pub fn script_end_0431<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x6));
    // memset(BLK, 0, 0x30)
    g.write_bytes(w(0x11), &[0; 0x30]);
    let h = g.read::<u16>(w(0x22));
    g.write::<u16>(w(0x22), h | 0x80);
    let v = g.read::<u32>(w(0x27));
    g.write::<u32>(w(0x34), v);
    let run = img.rel32_target(f + 0x39);
    let a = crate::sb_misc::script_run(g, img, run, w(0x2f));
    crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x45), w(0x3e));
    let m = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x4f), w(0x4a));
    let last = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x5b), w(0x54));
    if a == 0 || m == 0 {
        return Ret::eax(last);
    }
    g.write::<u32>(s + 0xd10, 1);
    // push ebp; push ebx; push esi; push edi; call
    let show = Frame {
        at: img.rel32_target(f + 0x72),
        esp: fr.esp.wrapping_sub(0x14),
        ..*fr
    };
    crate::m0431::show_and_sounds(g, img, &show);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x7e),
        node,
    ))
}

/// [`script_end_0431`] (the `rel32`s: `memset`, the scripts, the motions,
/// the show, the give-back).
pub const SCRIPT_END_0431: Sig = Sig {
    len: 135,
    hash: 0x866e_75fe_485c_a01d,
    calls: &[
        (0x16, Callee::Import("memset")),
        (0x39, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x45, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x4f, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x5b, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x72, Callee::Sig(&crate::m0431::SHOW_AND_SOUNDS)),
        (0x7e, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0431`'s packed sprite (0x100014d0, 326 bytes; internal: `ecx` =
/// the owner, stack `(-, buffer, lo, hi)`; a /GS frame, `ebp` = the entry
/// `esp - 4`): [`packed_sprite`](crate::o6_fam_pairs6::packed_sprite)
/// with a white colour and no placement: with the lock `**G_L` 1, slot
/// 885 `(lo & 0x3fff, (d >> 20) & 0x3f, 0, 0, 1 << ((d >> 26) & 15), 1 <<
/// ((d >> 30) & 15), &B)` (`d` the descriptor, the shifts 16-bit, `B`
/// 0x100 bytes at `ebp - 0x104`) and the lock put back; then, if slot 885
/// answered, slot 905 `(owner + 0x830, &B, buffer, 0, 0, 0, 0, &S, slot
/// 776 (0x48))` with `S` = `(1, 1, 1, 1)` at `ebp - 0x114`. Returns slot
/// 905's `eax`, or 0.
pub fn packed_sprite_0431<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (b, s) = (ebp.wrapping_sub(0x104), ebp.wrapping_sub(0x114));
    let buffer = fr.arg(g, 1);
    let owner = g.read::<u32>(fr.ecx.wrapping_add(0x830));
    let lock = g.read::<u32>(w(0x29));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let (lo, hi) = (fr.arg(g, 2), fr.arg(g, 3));
    let d = (u64::from(hi) << 32) | u64::from(lo);
    let bit = |sh: u32| u32::from(1u16 << ((d >> sh) & 0xf));
    for off in [0x10c, 0x110, 0x114, 0x108] {
        g.write::<u32>(ebp.wrapping_sub(off), 0x3f80_0000);
    }
    #[allow(clippy::cast_possible_truncation)] // six bits
    let size = ((d >> 20) & 0x3f) as u32;
    // call width: 7
    let made = g
        .call(
            slot_target(g, w(0xb1), 885),
            0,
            &[lo & 0x3fff, size, 0, 0, bit(26), bit(30), b],
        )
        .eax;
    let lock = g.read::<u32>(w(0xc9));
    g.write::<u32>(lock, saved);
    if made == 0 {
        return Ret::eax(0);
    }
    let kind = g.call(slot_target(g, w(0xda), 776), 0, &[0x48]).eax;
    // call width: 9
    let args = [owner, b, buffer, 0, 0, 0, 0, s, kind];
    Ret::eax(g.call(slot_target(g, w(0x123), 905), 0, &args).eax)
}

/// [`packed_sprite_0431`] (the `rel32`: the cookie check).
pub const PACKED_SPRITE_0431: Sig = Sig {
    len: 326,
    hash: 0x6909_1c74_e212_0b59,
    calls: &[(0x13e, Callee::Any)],
    imports: &[],
};

/// `magic_0431`'s fixed packet (0x10001360, 202 bytes; internal, no
/// arguments): the DLL's packet buffer `P` (0x60 bytes) cleared in 8-byte
/// steps (`movlpd`) and filled with its header 0x70000005 and the words
/// 0xe, 0x47, 0x21, 0x14, 0x4e, 8 (each at its operand, in the original's
/// order); slot 382 `(0, 0)`. Returns its `eax`.
pub fn packet_0431<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let zero8 = |g: &mut G, o: u32| {
        g.write::<u32>(w(o), 0);
        g.write::<u32>(w(o) + 4, 0);
    };
    zero8(g, 0xc);
    zero8(g, 0x14);
    g.write::<u32>(w(0x1a), 0x7000_0005);
    zero8(g, 0x26);
    g.write::<u32>(w(0x2c), 0xe);
    g.write::<u32>(w(0x36), 0);
    zero8(g, 0x42);
    for (o, v) in [
        (0x48, 0x47),
        (0x52, 0),
        (0x5c, 0x21),
        (0x66, 0),
        (0x70, 0x14),
        (0x7a, 0),
    ] {
        g.write::<u32>(w(o), v);
    }
    zero8(g, 0x86);
    g.write::<u32>(w(0x8c), 0x4e);
    g.write::<u32>(w(0x96), 0);
    zero8(g, 0xa2);
    g.write::<u32>(w(0xa8), 8);
    g.write::<u32>(w(0xb2), 0);
    g.call(slot_target(g, w(0x1), 382), 0, &[0, 0]).eax
}

/// [`packet_0431`].
pub const PACKET_0431: Sig = Sig {
    len: 202,
    hash: 0x2dfe_d904_5c56_1b5c,
    calls: &[],
    imports: &[],
};

/// `magic_0431`'s sprite frame (0x10003030, 369 bytes; a callback, no
/// arguments; an aligned /GS frame, `E` its base after the pushes): the
/// fixed packet ([`packet_0431`]); slot 719 `(P, 2, 5)` gives the
/// descriptor `(lo, hi)`, `lo` with its bits `0xade` cleared and
/// `0x3521` set; with `S = *G_S` the angle `S + 0xd04` steps by `K1` and
/// wraps once; the matrix `M` (`E + 0x18`, slot 235) turned (slot 329) by
/// `K2 - slot 333 (*G_C)` wrapped once, plus `K2` and the angle, wrapped
/// once; the owner `T[b]` (`b` the selection's second byte) draws the
/// sprite ([`packed_sprite_0431`], buffer `M`; its first word the `ecx`
/// slot 329 leaves, never read: 0 here); then the DLL's packet
/// ([`packet`](crate::o4_fam4::packet)). Returns the packet's `eax`.
pub fn sprite_frame_0431<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x60);
    let m = e + 0x18;
    let s = g.read::<u32>(w(0x19));
    packet_0431(g, img, img.rel32_target(f + 0x1e));
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x23)) + 1));
    let owner = g.read::<u32>(w(0x32).wrapping_add(4 * b));
    let d = g.call(t(g, 0x37, 719), 0, &[w(0x3c), 2, 5]).eax;
    let lo = g.read::<u32>(d) & 0xffff_f521 | 0x3521;
    let hi = g.read::<u32>(d + 4);
    let a = add(g.read::<f32>(s + 0xd04), k(g, 0x56));
    let a = wrap_once(g, img, f, a, [0x77, 0x61, 0x8b, 0x61]);
    g.write::<f32>(s + 0xd04, a);
    let mut st = Pushes::default();
    st.push(m);
    st.call(g, t(g, 0x9e, 235), 0);
    st.push(g.read::<u32>(w(0xb9)));
    let v = st0_f32(st.call(g, t(g, 0xb3, 333), 0).st0);
    st.pop(2);
    let k2 = k(g, 0xcd);
    let y = sub(k2, v);
    let y = wrap_once(g, img, f, y, [0xd5, 0xe5, 0xdd, 0xe5]);
    let y = add(add(y, k2), g.read::<f32>(s + 0xd04));
    let y = wrap_once(g, img, f, y, [0xd5, 0xe5, 0xdd, 0xe5]);
    st.push_all(&[y.to_bits(), m, m]);
    st.call(g, t(g, 0x136, 329), 0);
    // the frame, ebx/esi/edi, slot 329's yaw word, four pushes, the
    // return address
    let esp = e.wrapping_sub(0x18);
    for (i, x) in (1u32..).zip([0, m, lo, hi]) {
        g.write::<u32>(esp + 4 * i, x);
    }
    let sprite = Frame {
        at: img.rel32_target(f + 0x153),
        esp,
        ecx: owner,
        ..Frame::default()
    };
    packed_sprite_0431(g, img, &sprite);
    let packet = Frame {
        at: img.rel32_target(f + 0x15b),
        esp: e.wrapping_sub(8),
        ..*fr
    };
    crate::o4_fam4::packet(g, img, &packet)
}

/// [`sprite_frame_0431`] (the `rel32`s: the two packets, the sprite, the
/// cookie check).
pub const SPRITE_FRAME_0431: Sig = Sig {
    len: 369,
    hash: 0x3bc2_16e3_6871_bf3f,
    calls: &[
        (0x1e, Callee::Sig(&PACKET_0431)),
        (0x153, Callee::Sig(&PACKED_SPRITE_0431)),
        (0x15b, Callee::Sig(&crate::o4_fam4::PACKET_0431_1430)),
        (0x169, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0431`'s actors' motion set-up (0x100018f0, 276 bytes; internal,
/// no arguments): for the selection's actors 1 and 2 (`c = T[b]`, the
/// first read again before slot 399): slot 357 `(c, -1, 0x3c00)`, slot
/// 356 `(c, -1, 0x2ee0)`, slot 362 `(*G_M, c, 1, 0)` (`G_M` one global per
/// actor), slots 357 and 356 `(c, -1, -1)`; between them slot 399 `(c,
/// 1)` for the first. Each call passes the words the original leaves on
/// the stack. Returns the last slot's `eax`.
pub fn actor_motions_0431<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let chr = |g: &G, sel: u32, nth: u32, table: u32| {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel)) + nth));
        g.read::<u32>(w(table).wrapping_add(4 * b))
    };
    let mut st = Pushes::default();
    let mut eax = 0;
    // (the selection, the byte, the table, the slots' operands, `G_M`)
    for (sel, nth, table, ops, gm) in [
        (0x1, 1, 0x12, [0x17, 0x27, 0x3c, 0x54, 0x66], 0x4d),
        (0x9a, 2, 0xac, [0xb1, 0xbf, 0xd4, 0xec, 0xfe], 0xe5),
    ] {
        let c = chr(g, sel, nth, table);
        st.push_all(&[0x3c00, u32::MAX, c]);
        st.call(g, t(g, ops[0], 357), 0);
        st.push_all(&[0x2ee0, u32::MAX, c]);
        st.call(g, t(g, ops[1], 356), 0);
        let m = g.read::<u32>(w(gm));
        st.push_all(&[0, 1, c, m]);
        st.call(g, t(g, ops[2], 362), 0);
        st.push_all(&[u32::MAX, u32::MAX, c]);
        st.call(g, t(g, ops[3], 357), 0);
        st.push_all(&[u32::MAX, u32::MAX, c]);
        eax = st.call(g, t(g, ops[4], 356), 0).eax;
        if nth == 1 {
            st.pop(16);
            let c = chr(g, 0x78, 1, 0x88);
            st.push_all(&[1, c]);
            st.call(g, t(g, 0x8d, 399), 0);
        }
    }
    eax
}

/// [`actor_motions_0431`].
pub const ACTOR_MOTIONS_0431: Sig = Sig {
    len: 276,
    hash: 0x8372_6bd3_442f_66d8,
    calls: &[],
    imports: &[],
};

/// `magic_0431`'s objects start (0x10004220, 318 bytes; a table entry, no
/// arguments; a /GS frame, the offset `V` at `ebp - 0x14`): with the
/// state block `S = *G_S`, the object `O = S + 0x130` starts
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale `K1`,
/// index 1, value 0) with the yaw `+0xc4` = `S + 0xd00`, placed by slot
/// 237 `(*G_C, 0x73, &V, O + 0xd0)` (`V` = `(-30, 0, 0, 1)`; the start's
/// two words under it), `O + 0xdc` 1.0; then `S` itself starts (scale
/// `K2`, index 0, value 0) with the yaw `S + 0xd00 - K3` wrapped once, its
/// place `S + 0x820..` with `y` less `K4`, and `+0xe0` -20. Returns the
/// place's last word.
pub fn objects_start_0431<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x1c));
    let o = s + 0x130;
    let start = img.rel32_target(f + 0x30);
    crate::s4_misc::start_object_anim(g, img, start, o, k(g, 0x14), 1, 0);
    let yaw = g.read::<u32>(s + 0xd00);
    g.write::<u32>(o + 0xc4, yaw);
    let p = o + 0xd0;
    let v = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    for (i, x) in (0u32..).zip([0xc1f0_0000, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    let c = g.read::<u32>(w(0x54));
    // call width: 6 (the start's index and value under it)
    g.call(slot_target(g, w(0x4c), 237), 0, &[c, 0x73, v, p, 1, 0]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x95), s, k(g, 0x80), 0, 0);
    let yaw = sub(g.read::<f32>(s + 0xd00), k(g, 0xa5));
    let yaw = wrap_once(g, img, f, yaw, [0xaf, 0xb9, 0xc3, 0xd0]);
    g.write::<f32>(s + 0xc4, yaw);
    let mut last = 0;
    for i in 0..4u32 {
        last = g.read::<u32>(s + 0x820 + 4 * i);
        g.write::<u32>(s + 0xd0 + 4 * i, last);
    }
    let y = sub(g.read::<f32>(s + 0xd4), k(g, 0x109));
    g.write::<f32>(s + 0xd4, y);
    g.write::<u32>(s + 0xe0, 0xc1a0_0000);
    Ret::eax(last)
}

/// [`objects_start_0431`] (the `rel32`s: the two object starts, the
/// cookie check).
pub const OBJECTS_START_0431: Sig = Sig {
    len: 318,
    hash: 0x49e2_966e_75e3_5d46,
    calls: &[
        (0x30, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x95, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x136, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0431`'s object place (0x10003d30, 434 bytes; a callback, cdecl
/// `(O)`; a /GS frame: the matrices `M1` at `ebp - 0x94`, `M2` at `ebp -
/// 0x54`, `M3` at `ebp - 0xd4`, the point `V` at `ebp - 0x14`). With the
/// state block `S = *G_S` and the record `R = S + 0xc8c + 0x30 i` (`i` =
/// `O + 0x120`): slot 210 `(T[b], M1)` (`b` the selection's first byte),
/// slot 230 `(M2, R[0])`, `M2`'s last row's `x`, `y`, `z` times `K1`,
/// slot 231 `(M3, M1, M2)`; `V` = `(a + b + c) / K2` per axis (the three
/// points `R[1]`, `R[2]`, `R[3]`, summed in the original's order; `w` =
/// `a.w`); slot 229 `(O + 0xd0, M3, V)`, `O + 0xdc` 1.0; slot 719 `(P, 9,
/// 0)` and slot 306 `(7, 1, that)`; `O + 0x104` 0. Each call passes the
/// words the original leaves on the stack. Returns slot 306's `eax`.
pub fn object_place_0431<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let (m1, m2, m3, v) = (
        ebp.wrapping_sub(0x94),
        ebp.wrapping_sub(0x54),
        ebp.wrapping_sub(0xd4),
        ebp.wrapping_sub(0x14),
    );
    let o = fr.arg(g, 0);
    let i = g.read::<u32>(o.wrapping_add(0x120));
    let s = g.read::<u32>(w(0x20));
    let r = s.wrapping_add(0xc8c).wrapping_add(i.wrapping_mul(0x30));
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x30))));
    let chr = g.read::<u32>(w(0x3c).wrapping_add(4 * b));
    let mut st = Pushes::default();
    st.push_all(&[m1, chr]);
    st.call(g, t(g, 0x41, 210), 0);
    st.push_all(&[g.read::<u32>(r), m2]);
    st.call(g, t(g, 0x59, 230), 0);
    let k1 = k(g, 0x6e);
    for d in [0x30u32, 0x34, 0x38] {
        let x = mul(g.read::<f32>(m2 + d), k1);
        g.write::<f32>(m2 + d, x);
    }
    st.push_all(&[m2, m1, m3]);
    st.call(g, t(g, 0xa5, 231), 0);
    let (pa, pb) = (g.read::<u32>(r + 4), g.read::<u32>(r + 8));
    let f32_at = |g: &G, p: u32| g.read::<f32>(p);
    let x = add(f32_at(g, pa), f32_at(g, pb));
    let y = add(f32_at(g, pa + 4), f32_at(g, pb + 4));
    let z = add(f32_at(g, pa + 8), f32_at(g, pb + 8));
    let wv = g.read::<u32>(pa + 0xc);
    g.write::<u32>(v + 0xc, wv);
    let pc = g.read::<u32>(r + 0xc);
    let k2 = k(g, 0x104);
    let x = div(add(x, f32_at(g, pc)), k2);
    let y = div(add(f32_at(g, pc + 4), y), k2);
    let z = div(add(f32_at(g, pc + 8), z), k2);
    for (d, c) in [(0u32, x), (4, y), (8, z)] {
        g.write::<f32>(v + d, c);
    }
    st.push_all(&[v, m3, o.wrapping_add(0xd0)]);
    st.call(g, t(g, 0x147, 229), 0);
    g.write::<u32>(o.wrapping_add(0xdc), 0x3f80_0000);
    let p = w(0x176);
    st.push_all(&[0, 9, p]);
    let h = st.call(g, t(g, 0x171, 719), 0).eax;
    st.push_all(&[h, 1, 7]);
    let eax = st.call(g, t(g, 0x184, 306), 0).eax;
    g.write::<u32>(o.wrapping_add(0x104), 0);
    Ret::eax(eax)
}

/// [`object_place_0431`] (the `rel32`: the cookie check).
pub const OBJECT_PLACE_0431: Sig = Sig {
    len: 434,
    hash: 0xe1ad_5431_e9e9_3551,
    calls: &[(0x1aa, Callee::Any)],
    imports: &[],
};

/// `magic_0266`'s fading sprite (0x10002500, 374 bytes; internal, no
/// arguments; a /GS frame: the matrices `M1` at `ebp - 0x44`, `M2` at
/// `ebp - 0x84`, `M3` at `ebp - 0xc4`, `M4` at `ebp - 0x104`). The fade
/// state `*G_MODE` with its counter `*G_N`:
/// - 0: fading in, `n = (32 - c) * 128 / 32` (`c` the counter less one,
///   stored; at 0 the mode becomes 1 and the counter 32);
/// - 1: holding, `n = 0x80` while the counter counts down (at 0 the mode
///   becomes 2 and the counter 32);
/// - 2: fading out, `n = c * 128 / 32` (at 0 the mode becomes 3);
/// - else nothing: the original returns the mode less 2.
///
/// Then slot 235 `(M1)`, slot 210 `(*G_C, M2)`, slot 13 `(*G_C, M3)`, the
/// scaled camera ([`scaled_camera`](crate::m0052::scaled_camera), `M4`),
/// the angle `*G_A += K` (less `T` above `T`), slot 719 `(P, 2, 3)`'s
/// descriptor `(lo, hi)` and the sprite
/// ([`packed_sprite`](crate::o6_fam_pairs6::packed_sprite), `ecx` =
/// `*G_C`, `(-, M1, lo, hi, 0, -, n)`; the first word the `ecx` slot 719
/// leaves and the sixth a word the original never writes, both unread by
/// the sprite: 0 here). Returns the sprite's `eax`.
pub fn fading_sprite_0266<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    // `shl 7; cdq; and edx, 0x1f; add; sar 5`: a division by 32 toward 0
    let fade = |x: u32| {
        let m = x << 7;
        (m.wrapping_add((m.cast_signed() >> 31).cast_unsigned() & 0x1f)).cast_signed() >> 5
    };
    let mode = g.read::<u32>(w(0x14));
    let n = match mode {
        0 => {
            let c = g.read::<u32>(w(0x6f)).wrapping_sub(1);
            g.write::<u32>(w(0x84), c);
            if c == 0 {
                g.write::<u32>(w(0x94), 1);
                g.write::<u32>(w(0x9e), 0x20);
            }
            fade(0x20u32.wrapping_sub(c))
        }
        1 => {
            let c = g.read::<u32>(w(0x56)).wrapping_sub(1);
            g.write::<u32>(w(0x56), c);
            if c == 0 {
                g.write::<u32>(w(0x63), 2);
                g.write::<u32>(w(0x9e), 0x20);
            }
            0x80
        }
        2 => {
            let c = g.read::<u32>(w(0x2a)).wrapping_sub(1);
            g.write::<u32>(w(0x3a), c);
            if c == 0 {
                g.write::<u32>(w(0x4a), 3);
            }
            fade(c)
        }
        _ => return Ret::eax(mode.wrapping_sub(2)),
    };
    let ebp = fr.esp.wrapping_sub(4);
    let (m1, m2, m3, m4) = (
        ebp.wrapping_sub(0x44),
        ebp.wrapping_sub(0x84),
        ebp.wrapping_sub(0xc4),
        ebp.wrapping_sub(0x104),
    );
    g.call(t(g, 0xac, 235), 0, &[m1]);
    let c = g.read::<u32>(w(0xc6));
    g.call(t(g, 0xc0, 210), 0, &[c, m2]);
    let c = g.read::<u32>(w(0xe0));
    g.call(t(g, 0xda, 13), 0, &[c, m3]);
    // the frame, esi and edi, the return address
    crate::m0052::scaled_camera(
        g,
        img,
        img.rel32_target(f + 0xf3),
        m4,
        ebp.wrapping_sub(0x110),
    );
    let a = add(g.read::<f32>(w(0xfb)), k(g, 0x103));
    g.write::<f32>(w(0x11c), a);
    let lim = k(g, 0x10b);
    // comiss a, T; jbe: only above (ordered)
    if a > lim {
        g.write::<f32>(w(0x12a), sub(a, lim));
    }
    let owner = g.read::<u32>(w(0x111));
    let d = g.call(t(g, 0x12f, 719), 0, &[w(0x13e), 2, 3]).eax;
    let (lo, hi) = (g.read::<u32>(d), g.read::<u32>(d + 4));
    // the frame, esi and edi, n, 8 bytes and four pushes, the return
    // address
    let esp = ebp.wrapping_sub(0x12c);
    for (i, x) in (1u32..).zip([0, m1, lo, hi, 0, 0, n.cast_unsigned()]) {
        g.write::<u32>(esp + 4 * i, x);
    }
    let sprite = Frame {
        at: img.rel32_target(f + 0x15f),
        esp,
        ecx: owner,
        ..Frame::default()
    };
    crate::o6_fam_pairs6::packed_sprite(g, img, &sprite)
}

/// [`fading_sprite_0266`] (the `rel32`s: the scaled camera, the sprite,
/// the cookie check).
pub const FADING_SPRITE_0266: Sig = Sig {
    len: 374,
    hash: 0x6272_3dbb_cb44_ee64,
    calls: &[
        (0xf3, Callee::Sig(&crate::m0052::SCALED_CAMERA)),
        (
            0x15f,
            Callee::Sig(&crate::o6_fam_pairs6::PACKED_SPRITE_0266_1310),
        ),
        (0x16e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0266`'s actors' step (0x10001f10, 655 bytes; internal, no
/// arguments; a /GS frame: the matrices `M` at `ebp - 0x94`, `M2` at
/// `ebp - 0x54`, the point `V` at `ebp - 0x14`):
/// - while `*G_WAIT` is positive it counts down: 0;
/// - the timer `*G_T` counts down while positive; at 0 it returns 1;
/// - else slot 235 `(M)` and, for each actor `i` below `*G_N`, its record
///   `R` (0x88 apart from `G_R`, `R` pointing at its counter): while the
///   counter is positive it drops and the spin `R - 4` grows by `K1`; the
///   angles `R - 0x3c` and `R - 0x1c` grow by the spin and wrap once
///   (above `HI` less `T`, below `LO` plus `T`); the height `R - 0xc`
///   rises by `K3` (by `*G_T / K2 * K3` while the timer is at most 30);
///   `M2` = slot 330 `(M2, M, R[-0x1c])`, slots 328 and 329 by `R[-0x24]`
///   and `R[-0x20]`, `V` = slot 229 `(V, M2, R - 0x14)`; the actor's
///   matrix `A = R - 0x84` ([`actor_scale`](crate::sa_misc::actor_scale),
///   `ecx` = `i`) turned by slots 330, 328, 329 (`R[-0x3c]`, `R[-0x44]`,
///   `R[-0x40]`); its place `R - 0x54..` = `R - 0x34..` plus `V`.
///   Returns 0.
///
/// Each slot call passes the words the original leaves on the stack.
#[allow(clippy::too_many_lines)] // the timers and the per-actor steps
pub fn actors_step_0266<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let wait = g.read::<u32>(w(0x14));
    if wait.cast_signed() > 0 {
        g.write::<u32>(w(0x1e), wait.wrapping_sub(1));
        return Ret::eax(0);
    }
    let mut timer = g.read::<u32>(w(0x33));
    if timer.cast_signed() > 0 {
        timer = timer.wrapping_sub(1);
        g.write::<u32>(w(0x3d), timer);
    }
    if timer == 0 {
        return Ret::eax(1);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let (m, m2, v) = (
        ebp.wrapping_sub(0x94),
        ebp.wrapping_sub(0x54),
        ebp.wrapping_sub(0x14),
    );
    // (the registers the original saves stay under the calls' words)
    let mut st = Pushes::default();
    st.push(m);
    st.call(g, t(g, 0x61, 235), 0);
    st.pop(1);
    let n = g.read::<u32>(w(0x74));
    let (hi, tt, lo, k3) = (k(g, 0x82), k(g, 0x8a), k(g, 0xc9), k(g, 0x92));
    let wrap = |a: f32| {
        // comiss a, HI; jbe / comiss LO, a; jbe (NaN kept)
        if a > hi {
            sub(a, tt)
        } else if lo > a {
            add(a, tt)
        } else {
            a
        }
    };
    let scale = img.rel32_target(f + 0x1c0);
    // the constants the loop reloads (`+0x226`, `+0x22e`, `+0x272`) are
    // these globals again
    let mut r = w(0x99);
    let mut i = 0u32;
    while n.cast_signed() > 0 && i.cast_signed() < g.read::<u32>(w(0x24d)).cast_signed() {
        let c = g.read::<u32>(r);
        if c.cast_signed() > 0 {
            let spin = add(g.read::<f32>(r - 4), k(g, 0xaf));
            g.write::<u32>(r, c - 1);
            g.write::<f32>(r - 4, spin);
        }
        let a = wrap(add(g.read::<f32>(r - 4), g.read::<f32>(r - 0x3c)));
        g.write::<f32>(r - 0x3c, a);
        let b = wrap(add(g.read::<f32>(r - 4), g.read::<f32>(r - 0x1c)));
        let tm = g.read::<u32>(w(0x105));
        g.write::<f32>(r - 0x1c, b);
        let y = if tm.cast_signed() > 0x1e {
            add(g.read::<f32>(r - 0xc), k3)
        } else {
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps
            let x = div(tm.cast_signed() as f32, k(g, 0x129));
            add(mul(x, k3), g.read::<f32>(r - 0xc))
        };
        g.write::<f32>(r - 0xc, y);
        let word = |g: &G, d: u32| g.read::<u32>(r - d);
        st.push(word(g, 0x1c));
        st.push_all(&[m, m2]);
        st.call(g, t(g, 0x152, 330), 0);
        st.pop(2);
        st.set_top(word(g, 0x24));
        st.push_all(&[m2, m2]);
        st.call(g, t(g, 0x171, 328), 0);
        st.pop(2);
        st.set_top(word(g, 0x20));
        st.push_all(&[m2, m2]);
        st.call(g, t(g, 0x190, 329), 0);
        st.push_all(&[r - 0x14, m2, v]);
        st.call(g, t(g, 0x1a9, 229), 0);
        let a_m = r - 0x84;
        crate::sa_misc::actor_scale(g, img, scale, i, a_m);
        st.pop(5);
        for (op, slot, d) in [(0x1ca, 330, 0x3c), (0x1e6, 328, 0x44), (0x202, 329, 0x40)] {
            st.set_top(word(g, d));
            st.push_all(&[a_m, a_m]);
            st.call(g, t(g, op, slot), 0);
            st.pop(2);
        }
        st.pop(1);
        for (d, s, vd) in [(0x54u32, 0x34u32, 0u32), (0x50, 0x30, 4), (0x4c, 0x2c, 8)] {
            let x = add(g.read::<f32>(r - s), g.read::<f32>(v + vd));
            g.write::<f32>(r - d, x);
        }
        i += 1;
        r += 0x88;
    }
    Ret::eax(0)
}

/// [`actors_step_0266`] (the `rel32`s: the cookie checks, the actor's
/// scale).
pub const ACTORS_STEP_0266: Sig = Sig {
    len: 655,
    hash: 0x6f89_bbac_ee63_0f32,
    calls: &[
        (0x2a, Callee::Any),
        (0x50, Callee::Any),
        (0x1c0, Callee::Sig(&crate::sa_misc::ACTOR_SCALE)),
        (0x287, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0266`'s frame (0x10001e00, 258 bytes; a callback, no arguments;
/// a /GS frame, the point `V` at `ebp - 0x14`): the fading sprite
/// ([`fading_sprite_0266`]); the caster's place (slot 262 `(*G_C, &V)`)
/// into the object block `O..` (`w` 1.0); the object's script
/// ([`obj_script`](crate::l5_objhost::obj_script), `ecx` = `O`; its
/// answer the flag bit 15 of `O + 0x30`), the actors' step
/// ([`actors_step_0266`]), the motion
/// ([`motion_step`](crate::s3_motion::motion_step)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)). When all four answer
/// 1: slot 399 `(*G_C, 0)`, every actor's mode reset
/// ([`actor_mode`](crate::sa_script::actor_mode), `edx` 0), slot 214
/// `(*G_D)` and the grid drawn ([`draw_grid`](crate::sb_misc::draw_grid)),
/// whose `eax` is returned; else the objects' answer.
pub fn frame_0266<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    // the frame, ebx/esi/edi, the return address
    let inner = ebp.wrapping_sub(0x24);
    let sprite = Frame {
        at: call(0x14),
        esp: inner,
        ..*fr
    };
    fading_sprite_0266(g, img, &sprite);
    let v = ebp.wrapping_sub(0x14);
    let c = g.read::<u32>(w(0x2b));
    g.call(t(g, 0x25, 262), 0, &[c, v, v + 4, v + 8]);
    let o = w(0x5a);
    let x = g.read::<u32>(v);
    g.write::<u32>(w(0x40), x);
    let y = g.read::<u32>(v + 4);
    g.write::<u32>(w(0x4d), y);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    let z = g.read::<u32>(v + 8);
    g.write::<u32>(w(0x69), z);
    g.write::<u32>(w(0x6f), 0x3f80_0000);
    crate::l5_objhost::obj_script(g, img, call(0x78), o);
    // the script's `eax`: the object's flag bit 15 (`+0x30 >> 15`)
    let script = u32::from(g.read::<u16>(o + 0x30) >> 15);
    let step = Frame {
        at: call(0x7f),
        esp: inner,
        ..*fr
    };
    let actors = actors_step_0266(g, img, &step).eax;
    let motion = crate::s3_motion::motion_step(g, img, call(0x86));
    let eax = crate::l5_objhost::run_objects(g, img, call(0x8d), inner);
    if eax != 1 || actors != 1 || motion != 1 || script != 1 {
        return Ret::eax(eax);
    }
    let c = g.read::<u32>(w(0xab));
    g.call(t(g, 0xa3, 399), 0, &[c, 0]);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(w(0xbe)).cast_signed() {
        let mode = Frame {
            at: call(0xc9),
            esp: inner,
            ecx: i,
            edx: 0,
            ..*fr
        };
        crate::sa_script::actor_mode(g, img, &mode);
        i += 1;
    }
    let d = g.read::<u32>(w(0xdd));
    g.call(t(g, 0xd7, 214), 0, &[d]);
    Ret::eax(crate::sb_misc::draw_grid(g, img, call(0xed)))
}

/// [`frame_0266`] (the `rel32`s: its callees and the cookie check).
pub const FRAME_0266: Sig = Sig {
    len: 258,
    hash: 0xa100_5b02_9269_cb6b,
    calls: &[
        (0x14, Callee::Sig(&FADING_SPRITE_0266)),
        (0x78, Callee::Sig(&crate::l5_objhost::OBJ_SCRIPT)),
        (0x7f, Callee::Sig(&ACTORS_STEP_0266)),
        (0x86, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x8d, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0xc9, Callee::Sig(&crate::sa_script::ACTOR_MODE)),
        (0xed, Callee::Sig(&crate::sb_misc::DRAW_GRID)),
        (0xfa, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0266`'s two sparks (0x100021a0, 855 bytes; internal, no
/// arguments; a /GS frame: the offset `V` at `ebp - 0x24`, the point `P`
/// at `ebp - 0x14`): while `*G_N` is positive it counts down and two
/// sparks are taken from the DLL's ring
/// ([`ring_take`](crate::sa_misc::ring_take); none left ends it with 0):
/// `P` = slot 237 `(*G_C, 0x3e, &V, &P)` with `V` = `(-200, 0, 0, 1)`
/// (`P.w` 1.0); each spark gets its handle (the first `(*G_H)[1]`, the
/// second slot 719 `(PART, 9, 0)`), `u16 +0xa` 0x200, `+0x4` 1, `u16
/// +0x8` 0xffff, its place `P + frnd(5)` per axis (`+0x18` 1.0), three
/// spins `(slot 307 () - K1) * K2`, `+0x28` `frnd(pi)` (the first) or 0
/// (the second), `+0x2c` 0.17, byte `+0x47` 0 and the life `n = 32 +
/// (rand() % 33 & !1)` (the second `24 + (rand() % 25 & !1)`) in `+0x34`
/// with `+-n / 2` in `+0x38`, `+0x3c`. Returns `-n / 2` of the
/// second spark, or the answer that ended it (0, or `*G_N` when not
/// positive).
#[allow(clippy::too_many_lines)] // two sparks, as the original
pub fn sparks_0266<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let n = g.read::<u32>(w(0x11));
    if n.cast_signed() <= 0 {
        return Ret::eax(n);
    }
    g.write::<u32>(w(0x20), n.wrapping_sub(1));
    let take = img.rel32_target(f + 0x25);
    let spark = crate::sa_misc::ring_take(g, img, take);
    if spark == 0 {
        return Ret::eax(0);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let (v, p) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    for (i, x) in (0u32..).zip([0xc348_0000, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    let c = g.read::<u32>(w(0x45));
    g.call(t(g, 0x3d, 237), 0, &[c, 0x3e, v, p]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let frnd = |g: &mut G, o: u32, x: u32| st0_f32(g.call(t(g, o, 314), 0, &[x]).st0);
    let spin = |g: &mut G, o: u32, k1: u32, k2: u32| {
        let r = st0_f32(g.call(t(g, o, 307), 0, &[]).st0);
        mul(sub(r, k(g, k1)), k(g, k2))
    };
    let life = |g: &mut G, s: u32, m: u32, base: u32| {
        // rand() % m, its low bit cleared, plus `base`; half of it either way
        let r = g.rand().cast_signed() % m.cast_signed();
        let l = (r & !1) + base.cast_signed();
        g.write::<u8>(s + 0x47, 0);
        g.write::<i32>(s + 0x34, l);
        let half = l / 2;
        g.write::<i32>(s + 0x38, half);
        g.write::<i32>(s + 0x3c, -half);
        (-half).cast_unsigned()
    };
    let place = |g: &mut G, s: u32, ops: [u32; 3]| {
        let x = frnd(g, ops[0], 0x40a0_0000);
        let x = add(g.read::<f32>(p), x);
        g.write::<f32>(s + 0xc, x);
        let y = frnd(g, ops[1], 0x40a0_0000);
        let y = add(g.read::<f32>(p + 4), y);
        g.write::<f32>(s + 0x10, y);
        let z = frnd(g, ops[2], 0x40a0_0000);
        let z = add(g.read::<f32>(p + 8), z);
        g.write::<u32>(s + 0x18, 0x3f80_0000);
        g.write::<f32>(s + 0x14, z);
    };
    let h = g.read::<u32>(g.read::<u32>(w(0x6e)).wrapping_add(4));
    g.write::<u32>(spark, h);
    g.write::<u16>(spark + 0xa, 0x200);
    g.write::<u32>(spark + 4, 1);
    g.write::<u16>(spark + 8, 0xffff);
    place(g, spark, [0x88, 0xba, 0xe0]);
    for (o, d, k1, k2) in [(0x106, 0x1c, 0x12a, 0x137), (0x12f, 0x20, 0x154, 0x161)] {
        let x = spin(g, o, k1, k2);
        g.write::<f32>(spark + d, x);
    }
    let x = spin(g, 0x159, 0x17e, 0x192);
    g.write::<f32>(spark + 0x24, x);
    let a = frnd(g, 0x183, 0x4049_0fdb);
    g.write::<f32>(spark + 0x28, a);
    g.write::<u32>(spark + 0x2c, 0x3e2e_147b);
    life(g, spark, 0x21, 0x20);
    let spark = crate::sa_misc::ring_take(g, img, img.rel32_target(f + 0x1d9));
    if spark == 0 {
        return Ret::eax(0);
    }
    let h = g.call(t(g, 0x1e9, 719), 0, &[w(0x1f8), 9, 0]).eax;
    g.write::<u32>(spark, h);
    g.write::<u16>(spark + 0xa, 0x200);
    g.write::<u32>(spark + 4, 1);
    g.write::<u16>(spark + 8, 0xffff);
    place(g, spark, [0x20a, 0x239, 0x25f]);
    for (o, d, k1, k2) in [(0x285, 0x1c, 0x2ac, 0x2b9), (0x2b1, 0x20, 0x2d6, 0x2e3)] {
        let x = spin(g, o, k1, k2);
        g.write::<f32>(spark + d, x);
    }
    let x = spin(g, 0x2db, 0x300, 0x316);
    g.write::<u32>(spark + 0x28, 0);
    g.write::<u32>(spark + 0x2c, 0x3e2e_147b);
    g.write::<f32>(spark + 0x24, x);
    Ret::eax(life(g, spark, 0x19, 0x18))
}

/// [`sparks_0266`] (the `rel32`s: the two ring takes, the cookie check;
/// `rand` twice).
pub const SPARKS_0266: Sig = Sig {
    len: 855,
    hash: 0x1d8e_bf77_2806_6d8e,
    calls: &[
        (0x25, Callee::Sig(&crate::sa_misc::RING_TAKE)),
        (0x1d9, Callee::Sig(&crate::sa_misc::RING_TAKE)),
        (0x34f, Callee::Any),
    ],
    imports: &[(0x1b2, "rand"), (0x321, "rand")],
};

/// Two task nodes taken off the DLL's free ring (the count `u16 *G_N`,
/// the head `*G_FREE`) and appended to the task list (`G_FIRST`,
/// `G_LAST`), as `magic_0266`'s start-up inlines it twice: the first
/// node is the ring's head, the second the head after it (the register
/// the first take leaves); the first gets the kind `k1` and, with a
/// record operand (not 0), its record `+0x14`, and is linked after `*G_LAST` (`G_LAST` itself is
/// not written for it), the second gets `k2`, is linked after the first
/// and becomes `*G_LAST`. Operand offsets in order: the count read, the
/// head read, the head store after the first, its callback, its count
/// store, the list's first (read), the record, the last (read), the first
/// (store when empty), the head stores after the second (more, none), its
/// callback, its count store, the first (read), the first (store when
/// empty), the last (store). With an empty ring the original writes
/// through address 0, as this does.
fn take_pair_0266<G: Game>(g: &mut G, img: &Image, f: u32, ops: [u32; 16], k1: u8, k2: u8) {
    let w = |o: u32| img.word(f + o);
    let [
        n_op,
        free_op,
        free1,
        cb1,
        n1,
        first1,
        rec_op,
        last,
        first_e1,
        free_more,
        free_none,
        cb2,
        n2,
        first2,
        first_e2,
        last_w,
    ] = ops;
    let mut n = g.read::<u16>(w(n_op));
    let unlink = |g: &mut G, node: u32| {
        let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(prev.wrapping_add(4), next);
        let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(next, prev);
    };
    let reset = |g: &mut G, node: u32, cb: u32| {
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node.wrapping_add(o), 0);
        }
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(node.wrapping_add(0x10), w(cb));
        g.write::<u8>(node.wrapping_add(0x18), 0);
    };
    let (a, next) = if n == 0 {
        (0, g.read::<u32>(w(free_op)))
    } else {
        let a = g.read::<u32>(w(free_op));
        unlink(g, a);
        let next = if n > 1 { g.read::<u32>(a + 4) } else { 0 };
        n -= 1;
        g.write::<u32>(w(free1), next);
        reset(g, a, cb1);
        g.write::<u16>(w(n1), n);
        (a, next)
    };
    let listed = g.read::<u32>(w(first1)) != 0;
    g.write::<u8>(a.wrapping_add(0x18), k1);
    if rec_op != 0 {
        g.write::<u32>(a.wrapping_add(0x14), w(rec_op));
    }
    g.write::<u32>(a.wrapping_add(4), 0);
    if listed {
        let tail = g.read::<u32>(w(last));
        g.write::<u32>(a, tail);
        g.write::<u32>(tail.wrapping_add(4), a);
    } else {
        g.write::<u32>(a, 0);
        g.write::<u32>(w(first_e1), a);
    }
    g.write::<u32>(a.wrapping_add(8), a);
    let b = if n == 0 {
        0
    } else {
        let b = next;
        unlink(g, b);
        if n > 1 {
            let more = g.read::<u32>(b + 4);
            g.write::<u32>(w(free_more), more);
        } else {
            g.write::<u32>(w(free_none), 0);
        }
        n -= 1;
        reset(g, b, cb2);
        g.write::<u16>(w(n2), n);
        b
    };
    let listed = g.read::<u32>(w(first2)) != 0;
    g.write::<u8>(b.wrapping_add(0x18), k2);
    g.write::<u32>(b.wrapping_add(4), 0);
    if listed {
        g.write::<u32>(b, a);
        g.write::<u32>(a.wrapping_add(4), b);
    } else {
        g.write::<u32>(b, 0);
        g.write::<u32>(w(first_e2), b);
    }
    g.write::<u32>(b.wrapping_add(8), b);
    g.write::<u32>(w(last_w), b);
}

/// `magic_0266`'s start-up (0x10001490, 2356 bytes; internal, no
/// arguments, run by its load step; a /GS frame: the matrices `M` at
/// `ebp - 0xb4`, `M2` at `ebp - 0x74`, the points `Q` at `ebp - 0x14`,
/// `O2` at `ebp - 0x34`, `W` at `ebp - 0x24`):
/// - four pointers set, slot 719 `(PART, 8, 0)` into `G_H`, the part set
///   up ([`part_init`](crate::l5_objhost::part_init)), the object `O`
///   started ([`start_object_reset_arg`](crate::r9_misc::start_object_reset_arg),
///   scale `K`);
/// - the caster's place `P` (slot 262, `w` 1.0, the tilt 0.52), the yaw
///   `K1 - slot 333 (*G_C)` wrapped once, the spin `+= K2`, `P.y -= K3`;
///   `Q` = `P` moved by `V` turned by the tilt and the yaw (slots 235,
///   328, 329, 229, 248);
/// - each actor `i` below `*G_N` (its record `R`, 0x88 apart): its mode
///   ([`actor_mode`](crate::sa_script::actor_mode), `edx` = `R - 0x44`),
///   the angles `R - 4` 0, `R` = yaw + `PI` wrapped, `R + 4`, `R + 8` 0, its
///   matrix ([`actor_scale`](crate::sa_misc::actor_scale)) turned by
///   slots 330, 328, 329, its places `R - 0x14..` and `R + 0xc..` (slot
///   262 of `*G_C` for -1, else `T[sel[i]]`), `R + 0x18` 1.0, `R + 0x1c`
///   -0.52, `R + 0x20` the yaw + `PI` wrapped, `R + 0x2c` the `x` of
///   `M2 (R + 0xc)` with `M2` turned by `-yaw`, then `R + 0xc..` = `M2' W`
///   with `M2'` turned by the yaw, `+0x24` `PI`, `+0x30..+0x40` (0, 0, 1,
///   0, 8);
/// - the caster's motions (slots 357, 356, 362, 357, 356) and slot 399
///   `(*G_C, 1)`; a block of words and slot 215 or 413 (`*G_B` set or not)
///   `(*G_X)`; two task nodes (the first kind 1, the second kind 0:
///   [`take_pair_0266`]), the records (`records_init_100`), the record
///   `R2` and two more nodes (kind 1 holding `R2`, kind 2); the timers.
///
/// Each slot call passes the words the original leaves on the stack.
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0266<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, m2, q, o2, wv) = (
        ebp.wrapping_sub(0xb4),
        ebp.wrapping_sub(0x74),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x34),
        ebp.wrapping_sub(0x24),
    );
    let mut st = Pushes::default();
    for (dst, src) in [(0x29, 0x2d), (0x33, 0x37), (0x3d, 0x41), (0x47, 0x4b)] {
        g.write::<u32>(w(dst), w(src));
    }
    st.push_all(&[0, 8, w(0x23)]);
    let h = st.call(g, t(g, 0x14, 719), 0).eax;
    // (its two last words stay on the stack under two the object start
    // leaves unwritten; the later calls' words are compared up to there)
    st.pop(3);
    g.write::<u32>(w(0x55), h);
    crate::l5_objhost::part_init(g, img, call(0x5a));
    let mut xmm = fr.xmm;
    xmm[3] = u128::from(g.read::<u32>(w(0x62)));
    let start = Frame {
        at: call(0x6f),
        esp: ebp.wrapping_sub(0xcc),
        edx: w(0x6a),
        xmm,
        ..*fr
    };
    crate::r9_misc::start_object_reset_arg(g, img, &start);
    let c = g.read::<u32>(w(0x8f));
    st.push_all(&[w(0x79), w(0x84), w(0x89), c]);
    st.call(g, t(g, 0x74, 262), 0);
    st.push(g.read::<u32>(w(0x9c)));
    g.write::<u32>(w(0xa2), 0x3f80_0000);
    g.write::<u32>(w(0xac), 0x3f06_0a92);
    let v = st0_f32(st.call(g, t(g, 0x96, 333), 0).st0);
    st.pop(5);
    let yaw = sub(k(g, 0xc6), v);
    let yaw = wrap_once(g, img, f, yaw, [0xe0, 0xd6, 0xf0, 0xd6]);
    let spin = add(k(g, 0x101), k(g, 0x109));
    g.write::<f32>(w(0x111), yaw);
    let y = sub(k(g, 0x119), k(g, 0x121));
    g.write::<f32>(w(0x12a), spin);
    let px = g.read::<u32>(w(0x132));
    g.write::<u32>(q, px);
    let pz = g.read::<u32>(w(0x13f));
    g.write::<u32>(q + 8, pz);
    let pw = g.read::<u32>(w(0x153));
    g.write::<f32>(w(0x161), y);
    g.write::<f32>(q + 4, y);
    g.write::<u32>(q + 0xc, pw);
    st.push(m);
    st.call(g, t(g, 0x159, 235), 0);
    st.set_top(g.read::<u32>(w(0x17b)));
    st.push_all(&[m, m]);
    st.call(g, t(g, 0x18d, 328), 0);
    st.pop(2);
    st.set_top(g.read::<u32>(w(0x19d)));
    st.push_all(&[m, m]);
    st.call(g, t(g, 0x1b2, 329), 0);
    st.push_all(&[w(0x1bf), m, o2]);
    g.write::<u32>(w(0x1d5), 0x3f80_0000);
    st.call(g, t(g, 0x1cf, 229), 0);
    st.push_all(&[o2, q, q]);
    st.call(g, t(g, 0x1ef, 248), 0);
    st.pop(9);
    g.write::<u32>(q + 0xc, 0x3f80_0000);
    // the actors
    let scale = call(0x286);
    let mut i = 0u32;
    let mut r = w(0x21a);
    if g.read::<u32>(w(0x20f)).cast_signed() > 0 {
        loop {
            let a = r - 0x44;
            let mode = Frame {
                at: call(0x228),
                esp: ebp.wrapping_sub(0xc4),
                ecx: i,
                edx: a,
                ..*fr
            };
            crate::sa_script::actor_mode(g, img, &mode);
            let x = add(k(g, 0x230), k(g, 0x238));
            g.write::<u32>(r - 4, 0);
            let x = wrap_once(g, img, f, x, [0x246, 0x250, 0x25a, 0x267]);
            g.write::<f32>(r, x);
            g.write::<u32>(r + 4, 0);
            g.write::<u32>(r + 8, 0);
            crate::sa_misc::actor_scale(g, img, scale, i, a);
            for (op, slot, d) in [
                (0x290, 330, 4u32),
                (0x2aa, 328, 0u32.wrapping_sub(4)),
                (0x2c5, 329, 0),
            ] {
                st.push(g.read::<u32>(r.wrapping_add(d)));
                st.push_all(&[a, a]);
                st.call(g, t(g, op, slot), 0);
                st.pop(3);
            }
            let chr = |g: &G, me: u32, sel: u32, table: u32| {
                if i == u32::MAX {
                    g.read::<u32>(w(me))
                } else {
                    let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(i)));
                    g.read::<u32>(w(table).wrapping_add(4 * b))
                }
            };
            let c = chr(g, 0x2eb, 0x2f2, 0x2fd);
            g.call(t(g, 0x30e, 262), 0, &[c, r - 0x14, r - 0x10, r - 0xc]);
            g.write::<u32>(r - 8, 0x3f80_0000);
            let c = chr(g, 0x32c, 0x333, 0x33e);
            g.call(t(g, 0x34b, 262), 0, &[c, r + 0xc, r + 0x10, r + 0x14]);
            let x = add(k(g, 0x360), k(g, 0x368));
            g.write::<u32>(r + 0x18, 0x3f80_0000);
            g.write::<u32>(r + 0x1c, 0xbf06_0a92);
            let x = wrap_once(g, img, f, x, [0x380, 0x38a, 0x394, 0x3a1]);
            g.write::<f32>(r + 0x20, x);
            g.write::<u32>(r + 0x24, 0);
            g.write::<u32>(r + 0x28, 0);
            st.push(m2);
            st.call(g, t(g, 0x3aa, 235), 0);
            // xorps with the sign mask: the yaw negated
            let neg = g.read::<u32>(w(0x3cd)) ^ g.read::<u32>(w(0x3d4));
            st.set_top(neg);
            st.push_all(&[m2, m2]);
            st.call(g, t(g, 0x3e3, 329), 0);
            st.push_all(&[r + 0xc, m2, wv]);
            st.call(g, t(g, 0x3fc, 229), 0);
            g.write::<u32>(r + 0xc, 0);
            let wx = g.read::<u32>(wv);
            g.write::<u32>(r + 0x2c, wx);
            g.write::<u32>(wv, 0);
            g.write::<u32>(r + 0x24, 0x4049_0fdb);
            st.push(m2);
            st.call(g, t(g, 0x412, 235), 0);
            st.pop(6);
            st.set_top(g.read::<u32>(w(0x43c)));
            st.push_all(&[m2, m2]);
            st.call(g, t(g, 0x44e, 329), 0);
            st.push_all(&[wv, m2, r + 0xc]);
            st.call(g, t(g, 0x467, 229), 0);
            for (d, x) in [
                (0x30u32, 0u32),
                (0x34, 0),
                (0x38, 0x3f80_0000),
                (0x3c, 0),
                (0x40, 8),
            ] {
                g.write::<u32>(r + d, x);
            }
            st.pop(6);
            i += 1;
            r += 0x88;
            if i.cast_signed() >= g.read::<u32>(w(0x4aa)).cast_signed() {
                break;
            }
        }
    }
    // the caster's motions
    let c = g.read::<u32>(w(0x4bb));
    st.push_all(&[0x3c00, u32::MAX, c]);
    st.call(g, t(g, 0x4b5, 357), 0);
    st.push_all(&[0x2ee0, u32::MAX, c]);
    st.call(g, t(g, 0x4d0, 356), 0);
    let x = g.read::<u32>(w(0x4f6));
    st.push_all(&[0, 1, c, x]);
    st.call(g, t(g, 0x4e5, 362), 0);
    let c = g.read::<u32>(w(0x503));
    st.push_all(&[u32::MAX, u32::MAX, c]);
    st.call(g, t(g, 0x4fd, 357), 0);
    st.push_all(&[u32::MAX, u32::MAX, c]);
    st.call(g, t(g, 0x515, 356), 0);
    st.pop(16);
    let c = g.read::<u32>(w(0x538));
    st.push_all(&[1, c]);
    st.call(g, t(g, 0x527, 399), 0);
    st.pop(2);
    let battle = g.read::<u32>(w(0x545)) != 0;
    g.write::<u16>(w(0x54c), 0);
    let x = g.read::<u32>(w(0x551));
    g.write::<u32>(w(0x556), x);
    for (o, v) in [(0x562, 0), (0x56c, 0), (0x576, 0), (0x580, 0)] {
        g.write::<u32>(w(o), v);
    }
    g.write::<u32>(w(0x58a), w(0x58e));
    g.write::<u32>(w(0x594), w(0x598));
    g.write::<u32>(w(0x59e), 0);
    g.write::<u32>(w(0x5a8), 0);
    g.write::<u32>(w(0x5b2), w(0x5b6));
    g.write::<u32>(w(0x5bc), u32::MAX);
    let slot = if battle { 0x674 / 4 } else { 0x35c / 4 };
    let r = g.call(t(g, 0x55c, slot), 0, &[x]).eax;
    let battle = g.read::<u32>(w(0x5db)) != 0;
    g.write::<u32>(w(0x5e6), r);
    g.write::<u32>(w(0x5ec), u32::MAX);
    g.write::<u32>(w(0x5f6), 0);
    g.write::<u32>(w(0x600), 1);
    if battle {
        g.write::<u16>(w(0x613), 1);
    } else {
        g.write::<u16>(w(0x60a), 0);
    }
    take_pair_0266(
        g,
        img,
        f,
        [
            0x61a, 0x62f, 0x659, 0x67e, 0x689, 0x68f, 0, 0x6a2, 0x6b5, 0x6de, 0x6e6, 0x717, 0x722,
            0x728, 0x745, 0x753,
        ],
        1,
        0,
    );
    crate::sa_misc::records_init_100(g, img, call(0x758), w(0x74a));
    for (o, v) in [
        (0x767, 0),
        (0x771, 0x50),
        (0x77b, 0),
        (0x785, 0x76),
        (0x78f, 0x50),
        (0x799, 0x50),
    ] {
        g.write::<u32>(w(o), v);
    }
    g.write::<u16>(w(0x7a3), 0);
    take_pair_0266(
        g,
        img,
        f,
        [
            0x75f, 0x7b8, 0x7e2, 0x807, 0x812, 0x818, 0x824, 0x832, 0x845, 0x86e, 0x876, 0x8a7,
            0x8b2, 0x8b8, 0x8d9, 0x8e8,
        ],
        1,
        2,
    );
    for (o, v) in [
        (0x8ee, 0x9b),
        (0x8f8, 0x46),
        (0x902, 0x64),
        (0x90c, 0),
        (0x916, 0),
        (0x920, 0x20),
    ] {
        g.write::<u32>(w(o), v);
    }
    Ret::eax(0)
}

/// [`start_up_0266`] (the `rel32`s: the part's set-up, the object start,
/// the actors' modes and scales, the records, the cookie check).
pub const START_UP_0266: Sig = Sig {
    len: 2356,
    hash: 0x5ebe_bfab_ae36_906e,
    calls: &[
        (0x5a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x6f, Callee::Sig(&crate::r9_misc::START_OBJECT_RESET_0266)),
        (0x228, Callee::Sig(&crate::sa_script::ACTOR_MODE)),
        (0x286, Callee::Sig(&crate::sa_misc::ACTOR_SCALE)),
        (0x758, Callee::Sig(&crate::sa_misc::RECORDS_INIT_100)),
        (0x92c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0266`'s load step (0x10001210, the 118-byte shape) running
/// [`start_up_0266`].
pub fn load_0266<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0266)
}

/// [`load_0266`].
pub const LOAD_0266: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0266))],
    imports: &[],
};

/// `magic_0434`'s projected points (0x10004770, 207 bytes; internal:
/// `ecx` = the source points `A`, `edx` = the outputs `D` (16 bytes
/// each), stack `(n, M)`; a /GS frame, the point `T` at `ebp - 0x14`):
/// for each of `n` points, `T` = `A[i]` (slot 325), `T.x *= K1`, `D[i]` =
/// slot 229 `(D[i], M, T)`, then divided by its `w`: `x`, `y`, `z` over
/// `w` and `w` = `K2 / w`. Returns `A` past the last point (`A` when `n`
/// is not positive).
pub fn project_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let n = fr.arg(g, 0);
    let m = fr.arg(g, 1);
    let tmp = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let (mut a, mut d) = (fr.ecx, fr.edx);
    if n.cast_signed() <= 0 {
        return Ret::eax(a);
    }
    for _ in 0..n {
        g.call(slot_target(g, w(0x36), 325), 0, &[tmp, a]);
        let x = mul(g.read::<f32>(tmp), k(g, 0x4b));
        g.write::<f32>(tmp, x);
        // call width: 5 (slot 325's words under it)
        g.call(slot_target(g, w(0x54), 229), 0, &[d, m, tmp, tmp, a]);
        let ww = g.read::<f32>(d + 0xc);
        for o in [0u32, 4, 8] {
            let v = div(g.read::<f32>(d + o), ww);
            g.write::<f32>(d + o, v);
        }
        g.write::<f32>(d + 0xc, div(k(g, 0xa5), ww));
        a = a.wrapping_add(0x10);
        d = d.wrapping_add(0x10);
    }
    Ret::eax(a)
}

/// [`project_0434`] (the `rel32`: the cookie check).
pub const PROJECT_0434: Sig = Sig {
    len: 207,
    hash: 0x30d9_f6ac_caa4_2569,
    calls: &[(0xc7, Callee::Any)],
    imports: &[],
};

/// A sprite fan record `F` (`magic_0434`'s object starts): colour
/// 0x30808080 (`+0xf0`), `+0x124` 4, `+0x120`, `+0x110` 0, the rates
/// `+0xf8..+0x100` 0.025, the sizes `+0x108`, `+0x104` = `n`, `+0x10c` 8;
/// then a task node (`ops`: the owner immediate and the pool take's
/// `rel32`, the list's first (read), last (read), last (store), first
/// and last (stores when empty)) holding it, appended to the list.
/// Returns the node.
fn fan_node_0434<G: Game>(g: &mut G, img: &Image, f: u32, fan: u32, n: u32, ops: [u32; 7]) -> u32 {
    let w = |o: u32| img.word(f + o);
    let [owner, take, first, last, last_w, first_e, last_e] = ops;
    g.write::<u32>(fan + 0xf0, 0x3080_8080);
    g.write::<u32>(fan + 0x124, 4);
    g.write::<u32>(fan + 0x120, 0);
    g.write::<u32>(fan + 0x110, 0);
    for o in [0xf8u32, 0xfc, 0x100] {
        g.write::<u32>(fan + o, 0x3ccc_cccd);
    }
    g.write::<u32>(fan + 0x108, n);
    g.write::<u32>(fan + 0x104, n);
    g.write::<u32>(fan + 0x10c, 8);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + take), w(owner));
    let listed = g.read::<u32>(w(first)) != 0;
    g.write::<u32>(node.wrapping_add(0x14), fan);
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let tail = g.read::<u32>(w(last));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(w(last_w), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(first_e), node);
        g.write::<u32>(w(last_e), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    node
}

/// The scale `K1 / slot 282 () * K2` of `magic_0434`'s object starts
/// (slot 282's float stored as `f32` first).
fn fan_scale_0434<G: Game>(g: &mut G, img: &Image, f: u32, ops: [u32; 3]) -> f32 {
    let w = |o: u32| img.word(f + o);
    let v = st0_f32(g.call(slot_target(g, w(ops[0]), 282), 0, &[]).st0);
    mul(div(g.read::<f32>(w(ops[1])), v), g.read::<f32>(w(ops[2])))
}

/// `magic_0434`'s glow start (0x10007d80, 270 bytes; a table entry, no
/// arguments): with `S = *G_S`, the object `S + 0x850` starts
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), index 7,
/// value 0) at the scale `K1 / slot 282 () * K2`, its callback `S +
/// 0x954` = `CB` and byte `S + 0x976` 2; the fan `S + 0x1938`
/// ([`fan_node_0434`], 60). Returns the node.
pub fn glow_start_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x13));
    let scale = fan_scale_0434(g, img, f, [0x5, 0x20, 0x30]);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x3f), s + 0x850, scale, 7, 0);
    g.write::<u32>(s + 0x954, w(0x4f));
    g.write::<u8>(s + 0x976, 2);
    Ret::eax(fan_node_0434(
        g,
        img,
        f,
        s + 0x1938,
        0x3c,
        [0xa4, 0xc7, 0xcd, 0xe2, 0xec, 0xfe, 0x103],
    ))
}

/// [`glow_start_0434`] (the `rel32`s: the object start, the pool take).
pub const GLOW_START_0434: Sig = Sig {
    len: 270,
    hash: 0x6935_a280_1563_c9b5,
    calls: &[
        (0x3f, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xc7, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0434`'s glow and summon start (0x100069a0, 347 bytes; a table
/// entry, no arguments): as [`glow_start_0434`] with the object `S +
/// 0x980` (index 8; its callback `S + 0xa84`, byte `S + 0xaa6` 2), then
/// the object `O = S + 0x130` (scale `K3`, index 1, value 0) with the yaw
/// `+0xc4` = `S + 0x1a68`, placed by slot 237 `(*G_C, 0x73, 0, O +
/// 0xd0)` (the start's words under it), `w` 1.0; the fan `S + 0x1810`
/// (80). Returns the node.
pub fn summon_glow_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x13));
    let scale = fan_scale_0434(g, img, f, [0x5, 0x20, 0x30]);
    let start = img.rel32_target(f + 0x3f);
    crate::s4_misc::start_object_anim(g, img, start, s + 0x980, scale, 8, 0);
    let k3 = g.read::<f32>(w(0x47));
    g.write::<u32>(s + 0xa84, w(0x60));
    g.write::<u8>(s + 0xaa6, 2);
    let o = s + 0x130;
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x6c), o, k3, 1, 0);
    let yaw = g.read::<u32>(s + 0x1a68);
    g.write::<u32>(o + 0xc4, yaw);
    let p = o + 0xd0;
    let c = g.read::<u32>(w(0x94));
    // call width: 6 (the start's index and value under it)
    g.call(slot_target(g, w(0x7d), 237), 0, &[c, 0x73, 0, p, 1, 0]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    Ret::eax(fan_node_0434(
        g,
        img,
        f,
        s + 0x1810,
        0x50,
        [0xf1, 0x114, 0x11a, 0x12f, 0x139, 0x14b, 0x150],
    ))
}

/// [`summon_glow_0434`] (the `rel32`s: the two object starts, the pool
/// take).
pub const SUMMON_GLOW_0434: Sig = Sig {
    len: 347,
    hash: 0x78a1_4e4b_ff3d_e66a,
    calls: &[
        (0x3f, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x6c, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x114, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0434`'s bone matrix (0x10005600, 303 bytes; internal: `ecx` =
/// the model `O`, `edx` = a bone `j`, stack `(B)`; a /GS frame: the
/// matrices `M` at `ebp - 0x44`, `M2` at `ebp - 0x84`, `M3` at `ebp -
/// 0xc4`). Without a model handle (`O + 0x830` 0) it returns the frame's
/// cookie word (`cookie ^ ebp`); else `M` = slot 235 scaled by `K` on its
/// diagonal (`M[0]`, `M[5]`, `M[10]`), `M2` = slot 231 `(M2, B, M)`; bone
/// 0 gets slot 842 `(O[0x830], M2, 0)`; another bone `j` gets `M3` =
/// slot 231 `(M3, M2, X)` with `X` = `O[0x328] + 0x88 + 0x160 n` (`n` the
/// halfword at the bone's record `(O[0x1bc])[0x38 j]`), then slot 842
/// `(O[0x830], M3, j)`. Returns slot 842's `eax`.
pub fn bone_matrix_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let (o, j) = (fr.ecx, fr.edx);
    let b = fr.arg(g, 0);
    if g.read::<u32>(o.wrapping_add(0x830)) == 0 {
        return Ret::eax(g.read::<u32>(w(0xa)) ^ ebp);
    }
    let (m, m2, m3) = (
        ebp.wrapping_sub(0x44),
        ebp.wrapping_sub(0x84),
        ebp.wrapping_sub(0xc4),
    );
    g.call(t(g, 0x2f, 235), 0, &[m]);
    let k = g.read::<f32>(w(0x44));
    for d in [0u32, 0x14, 0x28] {
        let x = mul(g.read::<f32>(m + d), k);
        g.write::<f32>(m + d, x);
    }
    // call width: 4 (slot 235's word under it)
    g.call(t(g, 0x75, 231), 0, &[m2, b, m, m]);
    let h = g.read::<u32>(o.wrapping_add(0x830));
    if j == 0 {
        return Ret::eax(g.call(t(g, 0x96, 842), 0, &[h, m2, 0]).eax);
    }
    let rec = g.read::<u32>(
        g.read::<u32>(o.wrapping_add(0x1bc))
            .wrapping_add(j.wrapping_mul(0x38)),
    );
    let n = u32::from(g.read::<u16>(rec));
    let x = g
        .read::<u32>(o.wrapping_add(0x328))
        .wrapping_add(0x88)
        .wrapping_add(n.wrapping_mul(0x160));
    g.call(t(g, 0xf4, 231), 0, &[m3, m2, x]);
    let h = g.read::<u32>(o.wrapping_add(0x830));
    // call width: 6 (slot 231's words under it)
    Ret::eax(g.call(t(g, 0x109, 842), 0, &[h, m3, j, m3, m2, x]).eax)
}

/// [`bone_matrix_0434`] (the `rel32`s: the cookie checks).
pub const BONE_MATRIX_0434: Sig = Sig {
    len: 303,
    hash: 0x5b23_b6a8_3780_952e,
    calls: &[(0xb4, Callee::Any), (0x127, Callee::Any)],
    imports: &[],
};

/// `magic_0434`'s objects start (0x10006de0, 372 bytes; a table entry, no
/// arguments): the parts registered
/// ([`register_parts`](crate::sa_misc::register_parts)); with `S = *G_S`
/// the object `O = S + 0x5f0` starts
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale `K1`,
/// index 5, value 0) with the yaw `S + 0x1a68`, placed by slot 237
/// `(*G_C, 0x4c, 0, O + 0xd0)` (`w` 1.0); `S + 0xab0` starts (scale `K2`,
/// index 9, value 60) at `S + 0xca0..` with `y` less `K3` and the yaw;
/// the light record `S + 0x1450` (colour words, sizes 0x14, u16 `+0x98`
/// 0x48) on a task node appended to the list; a scale task over `S +
/// 0x17a0` ([`scale_task`](crate::s4_misc::scale_task), scales `K4` and
/// `K3`, count 0). Returns the scale task's node.
pub fn objects_start_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x4));
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x9), fr.eax);
    let start = img.rel32_target(f + 0x25);
    let o = s + 0x5f0;
    crate::s4_misc::start_object_anim(g, img, start, o, k(g, 0x11), 5, 0);
    let yaw = g.read::<u32>(s + 0x1a68);
    g.write::<u32>(o + 0xc4, yaw);
    let p = o + 0xd0;
    let c = g.read::<u32>(w(0x4d));
    // call width: 6 (the start's index and value under it)
    g.call(slot_target(g, w(0x36), 237), 0, &[c, 0x4c, 0, p, 5, 0]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let o2 = s + 0xab0;
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x72), o2, k(g, 0x57), 9, 0x3c);
    let k3 = k(g, 0x80);
    for i in 0..4u32 {
        let x = g.read::<u32>(s + 0xca0 + 4 * i);
        g.write::<u32>(o2 + 0xd0 + 4 * i, x);
    }
    let y = sub(g.read::<f32>(o2 + 0xd4), k3);
    g.write::<f32>(o2 + 0xd4, y);
    let yaw = g.read::<u32>(s + 0x1a68);
    g.write::<u32>(o2 + 0xc4, yaw);
    let r = s + 0x1450;
    g.write::<u32>(r + 0x80, 0x8080_8080);
    g.write::<u32>(r + 0x84, 0x0080_8080);
    g.write::<u32>(r + 0x88, 0);
    g.write::<u16>(r + 0x98, 0x48);
    g.write::<u32>(r + 0x90, 0x14);
    g.write::<u32>(r + 0x8c, 0x14);
    g.write::<u32>(r + 0x94, 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x122), w(0xbe));
    let listed = g.read::<u32>(w(0x128)) != 0;
    g.write::<u32>(node.wrapping_add(0x14), r);
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let tail = g.read::<u32>(w(0x13b));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x14d), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x165), node);
    let mut xmm = fr.xmm;
    xmm[1] = u128::from(g.read::<u32>(w(0x155)));
    xmm[2] = u128::from(k3.to_bits());
    // push esi, edi; push 0; the return address
    let esp = fr.esp.wrapping_sub(0x10);
    g.write::<u32>(esp + 4, 0);
    let st = Frame {
        at: img.rel32_target(f + 0x16a),
        esp,
        ecx: s + 0x17a0,
        xmm,
        ..*fr
    };
    crate::s4_misc::scale_task(g, img, &st)
}

/// [`objects_start_0434`] (the `rel32`s: the parts, two object starts,
/// the pool take, the scale task).
pub const OBJECTS_START_0434: Sig = Sig {
    len: 372,
    hash: 0x887e_b3c5_9f6a_c510,
    calls: &[
        (0x9, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        (0x25, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x72, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x122, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x16a, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
    ],
    imports: &[],
};

/// Operand offsets of [`scaled_turn_0434`]: slot 235's table, the
/// selection and character table (twice), slot 277's table and `K`, slots
/// 15, 16, 17, 330, 328, 329's tables; and whether slot 16's product has
/// its operands swapped (`v * M[5]`).
struct TurnOps0434 {
    s235: u32,
    chr: [(u32, u32); 2],
    s277: u32,
    k: u32,
    scales: [u32; 3],
    turns: [u32; 3],
    swapped: bool,
}

/// `magic_0434`'s scaled turn of the matrix `M = S + 0xc20`: slot 235
/// `(M)`; the selected character's slot 277 scale times `K` on `M`'s
/// diagonal, each then times slots 15, 16, 17 of the character; turned by
/// slots 330, 328, 329 with `S + 0xc88`, `S + 0xc80`, `S + 0xc84`. Each
/// call passes the words the original leaves on the stack. Returns slot
/// 329's `eax`.
fn scaled_turn_0434<G: Game>(g: &mut G, img: &Image, f: u32, s: u32, o: &TurnOps0434) -> u32 {
    let w = |x: u32| img.word(f + x);
    let t = |g: &G, x: u32, n: u32| slot_target(g, w(x), n);
    let chr = |g: &G, (sel, table): (u32, u32)| {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
        g.read::<u32>(w(table).wrapping_add(4 * b))
    };
    let m = s + 0xc20;
    let mut st = Pushes::default();
    st.push(m);
    st.call(g, t(g, o.s235, 235), 0);
    st.push(chr(g, o.chr[0]));
    let v = st0_f32(st.call(g, t(g, o.s277, 277), 0).st0);
    let v = mul(v, g.read::<f32>(w(o.k)));
    for d in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + d, v);
    }
    let c = chr(g, o.chr[1]);
    for (i, (op, d)) in o.scales.iter().zip([0u32, 0x14, 0x28]).enumerate() {
        st.push(c);
        let x = st0_f32(st.call(g, t(g, *op, 15 + i as u32), 0).st0);
        let y = if i == 1 && o.swapped {
            mul(x, g.read::<f32>(m + d))
        } else {
            mul(g.read::<f32>(m + d), x)
        };
        g.write::<f32>(m + d, y);
    }
    st.pop(4);
    let mut eax = 0;
    for (op, slot, d) in [
        (o.turns[0], 330, 0xc88u32),
        (o.turns[1], 328, 0xc80),
        (o.turns[2], 329, 0xc84),
    ] {
        st.set_top(g.read::<u32>(s + d));
        st.push_all(&[m, m]);
        eax = st.call(g, t(g, op, slot), 0).eax;
        st.pop(2);
    }
    eax
}

/// `magic_0434`'s scaled turn start (0x10007580, 488 bytes; a table entry,
/// no arguments): with `S = *G_S`, the spin `S + 0x1a74` 0.157, `+0x1a78`
/// 15, `+0x1a7c` 48, `+0x1a6c`, `+0x1a70` 0; the angles `S + 0xc80`,
/// `+0xc84`, `+0xc8c` from `S + 0xc60`, `+0xc64`, `+0xc6c` and `+0xc88`
/// `-pi/2`; `S + 0xc94` less `K`; the matrix turned
/// ([`scaled_turn_0434`]); a task node (no record) into `S + 0x1a60`
/// appended to the list. Returns the node.
pub fn turn_start_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x7));
    for (d, v) in [
        (0x1a74u32, 0x3e20_d97cu32),
        (0x1a78, 0xf),
        (0x1a7c, 0x30),
        (0x1a6c, 0),
        (0x1a70, 0),
    ] {
        g.write::<u32>(s + d, v);
    }
    for (dst, src) in [(0xc80u32, 0xc60u32), (0xc84, 0xc64), (0xc8c, 0xc6c)] {
        let x = g.read::<u32>(s + src);
        g.write::<u32>(s + dst, x);
    }
    g.write::<u32>(s + 0xc88, 0xbfc9_0fdb);
    let y = sub(g.read::<f32>(s + 0xc94), g.read::<f32>(w(0x7d)));
    g.write::<f32>(s + 0xc94, y);
    let ops = TurnOps0434 {
        s235: 0x63,
        chr: [(0x9a, 0xa4), (0xd4, 0xde)],
        s277: 0xa9,
        k: 0xc1,
        scales: [0xe3, 0xfa, 0x116],
        turns: [0x133, 0x15f, 0x181],
        swapped: false,
    };
    scaled_turn_0434(g, img, f, s, &ops);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x19d), w(0x198));
    let listed = g.read::<u32>(w(0x1a3)) != 0;
    g.write::<u32>(s + 0x1a60, node);
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let tail = g.read::<u32>(w(0x1bc));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(w(0x1c6), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x1d8), node);
        g.write::<u32>(w(0x1dd), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`turn_start_0434`] (the `rel32`: the pool take).
pub const TURN_START_0434: Sig = Sig {
    len: 488,
    hash: 0x327e_4abe_3707_321b,
    calls: &[(0x19d, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// `magic_0434`'s scaled turn end (0x10007770, 468 bytes; a table entry,
/// no arguments): with `S = *G_S`, the angles `S + 0xc80..+0xc9c` from
/// `S + 0xc60..`; the matrix turned ([`scaled_turn_0434`], slot 16's
/// product swapped); when `S + 0x1a60` holds a node, it goes back to its
/// pool ([`give_back`](crate::sa_pool::give_back)), `S + 0x1a60` 0, and
/// slot 829 `((*G_M) + 0x30, 0, 0, 0, 0)` runs with the lock `**G_L` 1.
/// Returns the lock pointer, or slot 329's `eax` without a node.
pub fn turn_end_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x7));
    for i in 0..8u32 {
        let x = g.read::<u32>(s + 0xc60 + 4 * i);
        g.write::<u32>(s + 0xc80 + 4 * i, x);
    }
    let ops = TurnOps0434 {
        s235: 0x6d,
        chr: [(0x82, 0x8c), (0xbc, 0xc6)],
        s277: 0x91,
        k: 0xa9,
        scales: [0xcb, 0xe2, 0xf1],
        turns: [0x11b, 0x147, 0x169],
        swapped: true,
    };
    let eax = scaled_turn_0434(g, img, f, s, &ops);
    let node = g.read::<u32>(s + 0x1a60);
    if node == 0 {
        return Ret::eax(eax);
    }
    crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x18a), node);
    let lock = g.read::<u32>(w(0x18f));
    g.write::<u32>(s + 0x1a60, 0);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let m = g.read::<u32>(w(0x1a8)).wrapping_add(0x30);
    // call width: 5
    g.call(slot_target(g, w(0x1b7), 829), 0, &[m, 0, 0, 0, 0]);
    let lock = g.read::<u32>(w(0x1c4));
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// [`turn_end_0434`] (the `rel32`: the give-back).
pub const TURN_END_0434: Sig = Sig {
    len: 468,
    hash: 0xa965_4a14_57a1_80e5,
    calls: &[(0x18a, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// `magic_0434`'s sparks (0x10006bd0, 527 bytes; a callback, cdecl
/// `(O)`; a /GS frame, the point `P` at `ebp - 0x14`): slot 268 `(T[b],
/// 12, &P)` (`b` the selection's byte, `P.w` 1.0); `O + 0x2c` counts down
/// while positive and the fade is `c * 128 / 80` (toward 0); then two
/// records of the DLL's pool ([`pool_take`](crate::sb_misc::pool_take);
/// none left ends it with 0): the handle slot 719 `(PART, 9, 0)` in
/// `+0x30`, `+0x34` 1, `u16 +0x38` 0xffff, `u16 +0x3a` 0x200; the place
/// `P` with `x` and `z` plus `frnd(2)`; the spins `+0x10` `frnd(0.03)`,
/// `+0x14` `frnd(0.04) + K`, `+0x18` `frnd(0.03)`, `+0x1c` `frnd(pi)`,
/// `+0x20`, `+0x28` the first and third times `K2`, `+0x24` 0.003,
/// `+0x2c` 1.0, `+0x3c` `frnd(0.025) + K`, `+0x4c` 32, `+0x50` 0 and the
/// fade byte `+0x6b`. Returns the last `frnd`'s `eax`.
pub fn sparks_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let o = fr.arg(g, 0);
    let p = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x1b))));
    let chr = g.read::<u32>(w(0x27).wrapping_add(4 * b));
    g.call(t(g, 0x2c, 268), 0, &[chr, 0xc, p]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let mut c = g.read::<u32>(o.wrapping_add(0x2c));
    if c.cast_signed() > 0 {
        c -= 1;
        g.write::<u32>(o.wrapping_add(0x2c), c);
    }
    // `shl 7; imul 0x66666667; sar 5` and the sign fix: c * 128 / 80
    #[allow(clippy::cast_possible_truncation)] // a byte of it
    let fade = (c.wrapping_shl(7).cast_signed() / 80).cast_unsigned() as u8;
    let frnd = |g: &mut G, op: u32, x: u32| {
        let out = g.call(t(g, op, 314), 0, &[x]);
        (st0_f32(out.st0), out.eax)
    };
    let take = img.rel32_target(f + 0x64);
    let mut eax = 0;
    for _ in 0..2 {
        let s = crate::sb_misc::pool_take(g, img, take);
        if s == 0 {
            return Ret::eax(0);
        }
        let h = g.call(t(g, 0x74, 719), 0, &[w(0x83), 9, 0]).eax;
        g.write::<u32>(s + 0x30, h);
        g.write::<u32>(s + 0x34, 1);
        g.write::<u16>(s + 0x38, 0xffff);
        g.write::<u16>(s + 0x3a, 0x200);
        for i in 0..4u32 {
            let x = g.read::<u32>(p + 4 * i);
            g.write::<u32>(s + 4 * i, x);
        }
        let (r, _) = frnd(g, 0xa9, 0x4000_0000);
        let x = add(g.read::<f32>(s), r);
        g.write::<f32>(s, x);
        let (r, _) = frnd(g, 0xee, 0x4000_0000);
        let z = add(g.read::<f32>(s + 8), r);
        g.write::<f32>(s + 8, z);
        let (r, _) = frnd(g, 0x113, 0x3cf5_c28f);
        g.write::<f32>(s + 0x10, r);
        let (r, _) = frnd(g, 0x12f, 0x3d23_d70a);
        g.write::<f32>(s + 0x14, add(r, k(g, 0x14e)));
        let (r, _) = frnd(g, 0x153, 0x3cf5_c28f);
        g.write::<f32>(s + 0x18, r);
        let (r, _) = frnd(g, 0x16f, 0x4049_0fdb);
        g.write::<f32>(s + 0x1c, r);
        let x = mul(g.read::<f32>(s + 0x10), k(g, 0x18e));
        g.write::<u32>(s + 0x24, 0x3b44_9ba6);
        g.write::<f32>(s + 0x20, x);
        let x = mul(g.read::<f32>(s + 0x18), k(g, 0x1ac));
        g.write::<u32>(s + 0x2c, 0x3f80_0000);
        g.write::<f32>(s + 0x28, x);
        let (r, e) = frnd(g, 0x193, 0x3ccc_cccd);
        eax = e;
        g.write::<u32>(s + 0x4c, 0x20);
        g.write::<u32>(s + 0x50, 0);
        g.write::<u8>(s + 0x6b, fade);
        g.write::<f32>(s + 0x3c, add(r, k(g, 0x1da)));
    }
    Ret::eax(eax)
}

/// [`sparks_0434`] (the `rel32`s: the pool take, the cookie check).
pub const SPARKS_0434: Sig = Sig {
    len: 527,
    hash: 0x61c1_8f0f_c1ce_ae3c,
    calls: &[
        (0x64, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x207, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0397`'s aim start (0x10004300, 495 bytes; a table entry, no
/// arguments; an aligned frame): with `S = *G_S`, the caster's place `P
/// = S + 0x18f0` (slot 262 `(*G_C, ...)`, `w` 1.0), its facing `K -
/// slot 333 (*G_C)` wrapped once into `S + 0x23f4`; the target `Q = S +
/// 0x1900`: when slot 520's id is in the DLL's -1-ended table, the
/// selected character's place (slots 262 and 268 `(c, 16, &Q)`, `w`
/// 1.0), else the party's mean ([`party_mean`](crate::m0397::party_mean));
/// `Q.y` = `P.y`, the aim `K - atan2(Q.z - P.z, Q.x - P.x)` (`_CIatan2`
/// of the doubles) wrapped once into `S + 0x23f8`, `+0x23fc` 16, `+0x2404`
/// -1, `+0x2408` 0, `*G_ON` 1, `*G_I` -1. The original's `eax` is the
/// `_CIatan2` import's leftover (junk; 0 here).
pub fn aim_start_0397<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let s = g.read::<u32>(w(0xe));
    let p = s + 0x18f0;
    let c = g.read::<u32>(w(0x28));
    let mut st = Pushes::default();
    st.push_all(&[p + 8, p + 4, p, c]);
    st.call(g, t(g, 0x21, 262), 0);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    st.push(g.read::<u32>(w(0x42)));
    let v = st0_f32(st.call(g, t(g, 0x35, 333), 0).st0);
    let x = sub(k(g, 0x56), v);
    let x = wrap_once(g, img, f, x, [0x6e, 0x64, 0x7e, 0x64]);
    g.write::<f32>(s + 0x23f4, x);
    let id = g.call(t(g, 0x8c, 520), 0, &[]).eax;
    let q = s + 0x1900;
    let i = crate::r34::find_word(g, w(0xa9), id);
    if i == u32::MAX {
        // push ebp; and esp, -8; sub esp, 0xc; three pushes; the call
        let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
        let pm = Frame {
            at: img.rel32_target(f + 0xcf),
            esp,
            ecx: q,
            ..*fr
        };
        crate::m0397::party_mean(g, img, &pm);
    } else {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x14e))));
        let c = g.read::<u32>(w(0x15e).wrapping_add(4 * b));
        let mut st = Pushes::default();
        st.push_all(&[q + 8, q + 4, q, c]);
        st.call(g, t(g, 0x16b, 262), 0);
        st.push_all(&[q, 0x10, c]);
        st.call(g, t(g, 0x17a, 268), 0);
        g.write::<u32>(q + 0xc, 0x3f80_0000);
    }
    let y = g.read::<u32>(s + 0x18f4);
    g.write::<u32>(s + 0x1904, y);
    let dz = sub(g.read::<f32>(q + 8), g.read::<f32>(s + 0x18f8));
    let dx = sub(g.read::<f32>(q), g.read::<f32>(s + 0x18f0));
    // fstp qword; cvtpd2ps
    #[allow(clippy::cast_possible_truncation)]
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let y = sub(k(g, 0x125), a);
    let y = wrap_once(g, img, f, y, [0x134, 0x13e, 0x19d, 0x1aa]);
    g.write::<f32>(s + 0x23f8, y);
    g.write::<u32>(s + 0x23fc, 0x10);
    g.write::<u32>(s + 0x2404, u32::MAX);
    g.write::<u32>(s + 0x2408, 0);
    g.write::<u32>(w(0x1d8), 1);
    g.write::<u32>(w(0x1e2), u32::MAX);
    Ret::eax(0)
}

/// [`aim_start_0397`] (the `rel32`s: the party's mean, `_CIatan2`).
pub const AIM_START_0397: Sig = Sig {
    len: 495,
    hash: 0xce33_b6b6_0c17_8626,
    calls: &[
        (0xcf, Callee::Sig(&crate::m0397::PARTY_MEAN)),
        (0x113, Callee::Import("_CIatan2")),
    ],
    imports: &[],
};

/// `magic_0397`'s start-up (0x10006530, 266 bytes; internal, no
/// arguments, run by its load step):
/// [`start_up_0447`](crate::m0390::start_up_0447)'s shape with four
/// pointers set, the part set up without a buffer word
/// ([`part_init`](crate::l5_objhost::part_init)) and the aim start
/// ([`aim_start_0397`]) where `magic_0447` places the caster (operands
/// five bytes later from there): slot 719 `(PART, 8, 0)` into `G_H`;
/// four words set; slot 271 `(4)`; two task nodes (kinds 1 and 2) from
/// the pool appended to the task list. Each slot call passes the words
/// the original leaves on the stack. Returns 0.
pub fn start_up_0397<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let mut s = Pushes::default();
    s.push_all(&[0, 8, w(0x12)]);
    for (dst, src) in [(0x18, 0x1c), (0x22, 0x26), (0x2c, 0x30), (0x36, 0x3a)] {
        g.write::<u32>(w(dst), w(src));
    }
    let h = s.call(g, slot_target(g, w(0x1), 719), 0).eax;
    s.pop(1);
    g.write::<u32>(w(0x44), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x49));
    // push esi; push edi; slot 719's two words; the call
    let aim = Frame {
        at: img.rel32_target(f + 0x4e),
        esp: fr.esp.wrapping_sub(0x14),
        ..*fr
    };
    aim_start_0397(g, img, &aim);
    g.write::<u32>(w(0x59), w(0x5d));
    g.write::<u32>(w(0x63), 0);
    g.write::<u16>(w(0x6e), 0);
    g.write::<u8>(w(0x76), 1);
    s.push(4);
    s.call(g, slot_target(g, w(0x53), 271), 0);
    let first = crate::sa_pool::take(g, img, img.rel32_target(f + 0x8e), w(0x89));
    let head = g.read::<u32>(w(0x94));
    g.write::<u8>(first.wrapping_add(0x18), 1);
    g.write::<u32>(first.wrapping_add(4), 0);
    let linked = if head == 0 {
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0xc0), first);
        first
    } else {
        let last = g.read::<u32>(w(0xab));
        g.write::<u32>(first, last);
        g.write::<u32>(last.wrapping_add(4), first);
        head
    };
    g.write::<u32>(first.wrapping_add(8), first);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0xcd), w(0xc5));
    g.write::<u8>(second.wrapping_add(0x18), 2);
    g.write::<u32>(second.wrapping_add(4), 0);
    if linked == 0 {
        // only with no first node (an empty pool)
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0xfa), second);
        g.write::<u32>(second.wrapping_add(8), second);
        g.write::<u32>(w(0x102), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first.wrapping_add(4), second);
        g.write::<u32>(second.wrapping_add(8), second);
        g.write::<u32>(w(0xea), second);
    }
    Ret::eax(0)
}

/// [`start_up_0397`] (the `rel32`s: the part, the aim start, two pool
/// takes).
pub const START_UP_0397: Sig = Sig {
    len: 266,
    hash: 0x6a72_00b5_eca4_54e0,
    calls: &[
        (0x49, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4e, Callee::Sig(&AIM_START_0397)),
        (0x8e, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xcd, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0397`'s load step (0x10003f30, the 118-byte shape) running
/// [`start_up_0397`].
pub fn load_0397<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0397)
}

/// [`load_0397`].
pub const LOAD_0397: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0397))],
    imports: &[],
};

/// `magic_0392`'s glide step (0x10003c30, 291 bytes; a callback, cdecl
/// `(O)`): with `S = *G_S`, the count `c = S + 0x5380` drops while
/// positive; `t = c / S[0x5384]`: `S + 0xf60` = `S[0xf80] (K - t) +
/// S[0xf70] t`, `S + 0xf68` likewise from `+0xf88` and `+0xf78`. At
/// `c` 8 a record is filled: `G_R` (`*G_A`), the callback `CB`, -1, the
/// handle slot 215 `(*G_A)` (slot 413 in a battle), -1, 0, 1 and the
/// halfword 0 (1 in a battle). Once `c` is not positive, `O + 0x56` = 1
/// and `O` is returned; else the last value in `eax` (the count, or the
/// record's halfword or slot's answer at 8).
pub fn glide_step_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x6));
    let mut eax = g.read::<u32>(s + 0x5380);
    if eax.cast_signed() > 0 {
        eax -= 1;
        g.write::<u32>(s + 0x5380, eax);
    }
    {
        let c = crate::sb_sse::cvt(g.read::<u32>(s + 0x5380));
        let n = crate::sb_sse::cvt(g.read::<u32>(s + 0x5384));
        let t = div(c, n);
        let u = sub(g.read::<f32>(w(0x35)), t);
        let x = add(
            mul(g.read::<f32>(s + 0xf80), u),
            mul(g.read::<f32>(s + 0xf70), t),
        );
        g.write::<f32>(s + 0xf60, x);
        let z = add(
            mul(g.read::<f32>(s + 0xf88), u),
            mul(g.read::<f32>(s + 0xf78), t),
        );
        g.write::<f32>(s + 0xf68, z);
        if g.read::<u32>(s + 0x5380) == 8 {
            let battle = g.read::<u32>(w(0x94)) != 0;
            let a = g.read::<u32>(w(0x9a));
            g.write::<u32>(w(0x9f), a);
            g.write::<u32>(w(0xab), w(0xaf));
            g.write::<u32>(w(0xb5), u32::MAX);
            let slot = if battle { 0x674 / 4 } else { 0x35c / 4 };
            let h = g.call(slot_target(g, w(0xa5), slot), 0, &[a]).eax;
            let battle = g.read::<u32>(w(0xd4)) != 0;
            g.write::<u32>(w(0xdf), h);
            g.write::<u32>(w(0xe5), u32::MAX);
            g.write::<u32>(w(0xef), 0);
            g.write::<u32>(w(0xf9), 1);
            if battle {
                g.write::<u16>(w(0x10c), 1);
                eax = h;
            } else {
                g.write::<u16>(w(0x103), 0);
                eax = 0;
            }
        }
    }
    if g.read::<u32>(s + 0x5380).cast_signed() > 0 {
        return Ret::eax(eax);
    }
    let o = fr.arg(g, 0);
    g.write::<u8>(o.wrapping_add(0x56), 1);
    Ret::eax(o)
}

/// [`glide_step_0392`].
pub const GLIDE_STEP_0392: Sig = Sig {
    len: 291,
    hash: 0xac32_fafe_7a90_6df0,
    calls: &[],
    imports: &[],
};

/// `magic_0392`'s glide start (0x10001920, 381 bytes; internal, no
/// arguments): with `S = *G_S`, the actor `i = S + 0x537c` (-1 the
/// caster `*G_C`, else `T[sel[i]]`) puts its place into `P = S + 0xf90`
/// (slot 268 `(c, 1, P)`, `w` 1.0); `P.y` rises by `K1`; the steps `n` =
/// the distance from `S + 0xf80..` (`_libm_sse2_sqrt_precise` of the
/// double) times `K2`, truncated and kept in 9..=15; `S + 0xfd0..` =
/// `(P - S[0xf80..]) * (K3 / n)` (`w` 0), `S + 0x5388` = `n`. Returns
/// `P.w`'s word (1.0).
pub fn glide_start_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x9));
    let i = g.read::<u32>(s + 0x537c);
    let p = s + 0xf90;
    let c = if i == u32::MAX {
        g.read::<u32>(w(0x20))
    } else {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x27)).wrapping_add(i)));
        g.read::<u32>(w(0x32).wrapping_add(4 * b))
    };
    g.call(slot_target(g, w(0x3b), 268), 0, &[c, 1, p]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let y = add(g.read::<f32>(s + 0xf94), g.read::<f32>(w(0x5a)));
    g.write::<f32>(s + 0xf94, y);
    let dx = sub(g.read::<f32>(s + 0xf80), g.read::<f32>(p));
    let dy = sub(g.read::<f32>(s + 0xf84), y);
    let dz = sub(g.read::<f32>(s + 0xf88), g.read::<f32>(s + 0xf98));
    let sum = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
    // _libm_sse2_sqrt_precise of the double, rounded back to f32
    #[allow(clippy::cast_possible_truncation)]
    let d = f64::from(sum).sqrt() as f32;
    let k = mul(d, g.read::<f32>(w(0xb5)));
    // cvttss2si (0x80000000 for NaN and out of range)
    #[allow(clippy::cast_possible_truncation)]
    let n = if k.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&k) {
        i32::MIN
    } else {
        k as i32
    };
    let n = n.clamp(9, 15);
    for (j, d) in [(0u32, 0xf80u32), (4, 0xf84), (8, 0xf88)] {
        let v = sub(g.read::<f32>(p + j), g.read::<f32>(s + d));
        g.write::<f32>(s + 0xfd0 + j, v);
    }
    let one = g.read::<u32>(p + 0xc);
    g.write::<u32>(s + 0xfdc, one);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps of 9..=15
    let r = div(g.read::<f32>(w(0xe3)), n as f32);
    for j in [0u32, 4] {
        let v = mul(r, g.read::<f32>(s + 0xfd0 + j));
        g.write::<f32>(s + 0xfd0 + j, v);
    }
    let v = mul(g.read::<f32>(s + 0xfd8), r);
    g.write::<f32>(s + 0xfd8, v);
    g.write::<u32>(s + 0xfdc, 0);
    g.write::<i32>(s + 0x5388, n);
    Ret::eax(one)
}

/// [`glide_start_0392`] (the `rel32`: `_libm_sse2_sqrt_precise`).
pub const GLIDE_START_0392: Sig = Sig {
    len: 381,
    hash: 0x3961_dfdd_2620_bf22,
    calls: &[(0xa9, Callee::Import("_libm_sse2_sqrt_precise"))],
    imports: &[],
};

/// The character `magic_0392`'s code names for an actor `i` (operands of
/// the code at `f`): `*G_C` for -1, else `T[sel[i]]`.
fn actor_chr_0392<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    i: u32,
    (me, sel, table): (u32, u32, u32),
) -> u32 {
    let w = |o: u32| img.word(f + o);
    if i == u32::MAX {
        g.read::<u32>(w(me))
    } else {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(i)));
        g.read::<u32>(w(table).wrapping_add(4 * b))
    }
}

/// `magic_0392`'s nearest actor (0x10001540, 496 bytes; internal, no
/// arguments; an aligned /GS frame, `E` its base: the points `A` at `E +
/// 0x10`, `C` at `E + 0x20`): with `S = *G_S`, each actor `i` below
/// `*G_N` puts its place into `A` (slot 262) and its distance from `S +
/// 0xf70..` (`_libm_sse2_sqrt_precise` of the double) is compared with
/// the nearest so far (from `K`; a strictly smaller one wins, the first
/// is actor 0); then the caster's place into `C`, the nearest actor's
/// into `A` again, and `S + 0x5378` = `K1 - atan2(A.z - C.z, A.x - C.x)`
/// (`_CIatan2` of the doubles) wrapped once, `S + 0x537c` the actor. The
/// original's `eax` is the `_CIatan2` import's leftover (junk; 0 here).
pub fn nearest_actor_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x34);
    let (a, c) = (e + 0x10, e + 0x20);
    let s = g.read::<u32>(w(0x1f));
    let mut best = g.read::<f32>(w(0x18));
    let mut pick = 0u32;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(w(0x31)).cast_signed() {
        let chr = actor_chr_0392(g, img, f, i, (0x46, 0x4d, 0x58));
        g.call(t(g, 0x6d, 262), 0, &[chr, a, a + 4, a + 8]);
        let dx = sub(g.read::<f32>(s + 0xf70), g.read::<f32>(a));
        let dy = sub(g.read::<f32>(s + 0xf74), g.read::<f32>(a + 4));
        let dz = sub(g.read::<f32>(s + 0xf78), g.read::<f32>(a + 8));
        g.write::<u32>(a + 0xc, 0x3f80_0000);
        let sum = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
        // _libm_sse2_sqrt_precise of the double, rounded back to f32
        #[allow(clippy::cast_possible_truncation)]
        let d = f64::from(sum).sqrt() as f32;
        // comiss best, d; jbe: only strictly nearer (ordered)
        if best > d {
            best = d;
            pick = i;
        }
        i += 1;
    }
    let caster = g.read::<u32>(w(0x104));
    g.call(t(g, 0xfe, 262), 0, &[caster, c, c + 4, c + 8]);
    g.write::<u32>(c + 0xc, 0x3f80_0000);
    let chr = actor_chr_0392(g, img, f, pick, (0x121, 0x128, 0x133));
    g.call(t(g, 0x148, 262), 0, &[chr, a, a + 4, a + 8]);
    let dz = sub(g.read::<f32>(a + 8), g.read::<f32>(c + 8));
    let dx = sub(g.read::<f32>(a), g.read::<f32>(c));
    // fstp qword; cvtpd2ps
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let y = sub(g.read::<f32>(w(0x19c)), ang);
    let y = wrap_once(g, img, f, y, [0x1ab, 0x1b5, 0x1bf, 0x1cc]);
    g.write::<u32>(s + 0x537c, pick);
    g.write::<f32>(s + 0x5378, y);
    Ret::eax(0)
}

/// [`nearest_actor_0392`] (the `rel32`s: `_libm_sse2_sqrt_precise`,
/// `_CIatan2`, the cookie check).
pub const NEAREST_ACTOR_0392: Sig = Sig {
    len: 496,
    hash: 0x29e3_06bd_3b12_7b77,
    calls: &[
        (0xc6, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x18a, Callee::Import("_CIatan2")),
        (0x1e8, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0392`'s glide target (0x10001730, 493 bytes; internal, no
/// arguments; an aligned /GS frame, `E` its base after the pushes: the
/// place `P` at `E + 4`, the offset `V` at `E + 0x14`, the turned `O` at
/// `E + 0x24`, the matrix `M` at `E + 0x34`). With `S = *G_S` and the
/// actor `i = S + 0x537c`: its place `P` (slot 262), `V` = `(0, 0, s +
/// K, 1)` with `s` the actor's scale
/// ([`actor_scale_by`](crate::s4_small::actor_scale_by)), `M` = slot 235
/// turned by X 0 and Y `S + 0x5378` (slots 328, 329), `O` = `M V` (slot
/// 229), `P += O` (slot 248); `S + 0xf80..` = `(P.x, S[0xf74], P.z, 1)`
/// and the steps `n` = the distance from `S + 0xf70..` in `x`/`z`
/// (`_libm_sse2_sqrt_precise` of the double; the `y` term is `S[0xf74]`
/// less itself) over `K2`, truncated, at least 9, into `S + 0x5380` and
/// `+0x5384`. Each slot call passes the words the original leaves on the
/// stack. Returns `n`.
pub fn glide_target_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x7c);
    let (p, v, o, m) = (e + 4, e + 0x14, e + 0x24, e + 0x34);
    let s = g.read::<u32>(w(0x17));
    let i = g.read::<u32>(s + 0x537c);
    let chr = actor_chr_0392(g, img, f, i, (0x27, 0x2e, 0x39));
    let mut st = Pushes::default();
    st.push_all(&[p + 8, p + 4, p, chr]);
    st.call(g, t(g, 0x4e, 262), 0);
    let yaw = g.read::<u32>(s + 0x5378);
    let i = g.read::<u32>(s + 0x537c);
    g.write::<u32>(e + 0x10, 0x3f80_0000);
    g.write::<u32>(e, yaw);
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    let sc = Frame {
        at: img.rel32_target(f + 0x87),
        ecx: i,
        ..Frame::default()
    };
    let r = crate::s4_small::actor_scale_by(g, img, &sc);
    let r = r.xmm0.map_or(0.0, |x| f32::from_bits(x.reg() as u32));
    let d = add(r, g.read::<f32>(w(0x8f)));
    st.push(m);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    g.write::<f32>(v + 8, d);
    st.call(g, t(g, 0x99, 235), 0);
    st.pop(4);
    st.set_top(0);
    st.push_all(&[m, m]);
    st.call(g, t(g, 0xc4, 328), 0);
    st.pop(2);
    st.set_top(g.read::<u32>(e));
    st.push_all(&[m, m]);
    st.call(g, t(g, 0xe5, 329), 0);
    st.push_all(&[v, m, o]);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    st.call(g, t(g, 0x101, 229), 0);
    st.push_all(&[o, p, p]);
    st.call(g, t(g, 0x121, 248), 0);
    let y = g.read::<f32>(s + 0xf74);
    let dx = sub(g.read::<f32>(s + 0xf70), g.read::<f32>(p));
    let dy = sub(y, y);
    g.write::<f32>(e, y);
    let dz = sub(g.read::<f32>(s + 0xf78), g.read::<f32>(p + 8));
    let sum = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
    // _libm_sse2_sqrt_precise of the double, rounded back to f32
    #[allow(clippy::cast_possible_truncation)]
    let d = f64::from(sum).sqrt() as f32;
    let px = g.read::<u32>(p);
    g.write::<u32>(s + 0xf80, px);
    let k = div(d, g.read::<f32>(w(0x19d)));
    g.write::<f32>(s + 0xf84, y);
    // cvttss2si (0x80000000 for NaN and out of range)
    #[allow(clippy::cast_possible_truncation)]
    let n = if k.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&k) {
        i32::MIN
    } else {
        k as i32
    };
    let n = n.max(9);
    let pz = g.read::<u32>(p + 8);
    g.write::<u32>(s + 0xf88, pz);
    g.write::<u32>(s + 0xf8c, 0x3f80_0000);
    g.write::<i32>(s + 0x5380, n);
    g.write::<i32>(s + 0x5384, n);
    Ret::eax(n.cast_unsigned())
}

/// [`glide_target_0392`] (the `rel32`s: the actor's scale,
/// `_libm_sse2_sqrt_precise`, the cookie check).
pub const GLIDE_TARGET_0392: Sig = Sig {
    len: 493,
    hash: 0x1204_43f9_8b68_bdaf,
    calls: &[
        (0x87, Callee::Sig(&crate::s4_small::ACTOR_SCALE_BY)),
        (0x17d, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x1e5, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0392`'s glide run (0x10003a40, 485 bytes; a callback, cdecl
/// `(O)`): with `S = *G_S`, the steps `n = S + 0x5388` drop while
/// positive and `S + 0xf60..` moves by `S + 0xfd0..`. At `n` 8 the record
/// of [`glide_step_0392`] is filled (callback `CB`); at `n` 1 the object
/// `S + 0x720` starts ([`start_object_plain`](crate::s4_misc::start_object_plain),
/// scale `K`, index 6) at `S + 0xf90..` with the yaw `S + 0x5378`, `+0xe4`
/// -4 and `+0xe8` -6, and slot 388 `(a, 13, 1)` runs for the actor `S +
/// 0x537c` (`*G_A` for -1, else its selection byte). Once `n` is not
/// positive, `S + 0xf60..` = `S + 0xf90..`, `O + 0x56` = 1 and `O` is
/// returned; else the last value in `eax`.
pub fn glide_run_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x6));
    let n = g.read::<u32>(s + 0x5388);
    if n.cast_signed() > 0 {
        g.write::<u32>(s + 0x5388, n - 1);
    }
    for j in [0u32, 4, 8] {
        let v = add(g.read::<f32>(s + 0xfd0 + j), g.read::<f32>(s + 0xf60 + j));
        g.write::<f32>(s + 0xf60 + j, v);
    }
    let mut eax = g.read::<u32>(s + 0xf6c);
    g.write::<u32>(s + 0xf6c, eax);
    if g.read::<u32>(s + 0x5388) == 8 {
        let battle = g.read::<u32>(w(0x7a)) != 0;
        let a = g.read::<u32>(w(0x80));
        g.write::<u32>(w(0x85), a);
        g.write::<u32>(w(0x91), w(0x95));
        g.write::<u32>(w(0x9b), u32::MAX);
        let slot = if battle { 0x674 / 4 } else { 0x35c / 4 };
        let h = g.call(slot_target(g, w(0x8b), slot), 0, &[a]).eax;
        let battle = g.read::<u32>(w(0xba)) != 0;
        g.write::<u32>(w(0xc5), h);
        g.write::<u32>(w(0xcb), u32::MAX);
        g.write::<u32>(w(0xd5), 0);
        g.write::<u32>(w(0xdf), 1);
        if battle {
            g.write::<u16>(w(0xf2), 1);
            eax = h;
        } else {
            g.write::<u16>(w(0xe9), 0);
            eax = 0;
        }
    }
    if g.read::<u32>(s + 0x5388) == 1 {
        let o = s + 0x720;
        let scale = g.read::<f32>(w(0x107));
        crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x11a), o, scale, 6);
        for j in 0..4u32 {
            let x = g.read::<u32>(s + 0xf90 + 4 * j);
            g.write::<u32>(o + 0xd0 + 4 * j, x);
        }
        let yaw = g.read::<u32>(s + 0x5378);
        g.write::<u32>(o + 0xc4, yaw);
        g.write::<u32>(o + 0xe4, 0xc080_0000);
        g.write::<u32>(o + 0xe8, 0xc0c0_0000);
        let i = g.read::<u32>(s + 0x537c);
        let a = if i == u32::MAX {
            g.read::<u32>(w(0x17e))
        } else {
            u32::from(g.read::<u8>(g.read::<u32>(w(0x185)).wrapping_add(i)))
        };
        eax = g.call(slot_target(g, w(0x193), 388), 0, &[a, 0xd, 1]).eax;
    }
    if g.read::<u32>(s + 0x5388).cast_signed() > 0 {
        return Ret::eax(eax);
    }
    for j in 0..4u32 {
        let x = g.read::<u32>(s + 0xf90 + 4 * j);
        g.write::<u32>(s + 0xf60 + 4 * j, x);
    }
    let o = fr.arg(g, 0);
    g.write::<u8>(o.wrapping_add(0x56), 1);
    Ret::eax(o)
}

/// [`glide_run_0392`] (the `rel32`: the object start).
pub const GLIDE_RUN_0392: Sig = Sig {
    len: 485,
    hash: 0x83f0_9692_e8a9_db67,
    calls: &[(0x11a, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN))],
    imports: &[],
};

/// `magic_0392`'s summon place (0x10001e50, 416 bytes; a table entry, no
/// arguments; a /GS frame, the point `M` at `ebp - 0x14`): with `S =
/// *G_S`, the object `O = S + 0x850` starts
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale
/// `K`, index 7) at `S + 0xf70..` with `y` less `K1` and the yaw `S +
/// 0x5378`; the angles `S + 0xf50..` = `(0, yaw, 0, 0)` turn the scaled
/// matrix `S + 0xd10` ([`scale_rotate`](crate::sa_misc::scale_rotate))
/// whose place `S + 0xd40..` and `S + 0xf60..` are `S + 0xf70..`; then
/// slots 210 and 211 `(*G_C, S + 0xd10)`. (The original's other path,
/// for a zero matrix address, gets a party member's place (slots 217,
/// 255) instead of slot 210; `S + 0xd10` is never 0, but it is kept.)
/// Returns slot 211's `eax`.
pub fn summon_place_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let s = g.read::<u32>(w(0x1d));
    let o = s + 0x850;
    let scale = g.read::<f32>(w(0x14));
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x2f), o, scale, 7);
    for j in 0..4u32 {
        let x = g.read::<u32>(s + 0xf70 + 4 * j);
        g.write::<u32>(o + 0xd0 + 4 * j, x);
    }
    let yaw = g.read::<u32>(s + 0x5378);
    let y = sub(g.read::<f32>(o + 0xd4), g.read::<f32>(w(0x75)));
    g.write::<u32>(o + 0xc4, yaw);
    let a = s + 0xf50;
    let m = s + 0xd10;
    g.write::<f32>(o + 0xd4, y);
    g.write::<u32>(a, 0);
    let yaw = g.read::<u32>(s + 0x5378);
    g.write::<u32>(a + 4, yaw);
    g.write::<u32>(a + 8, 0);
    g.write::<u32>(a + 0xc, 0);
    crate::sa_misc::scale_rotate(g, img, img.rel32_target(f + 0xbd), m, a);
    for j in 0..4u32 {
        let x = g.read::<u32>(s + 0xf70 + 4 * j);
        g.write::<u32>(s + 0xd40 + 4 * j, x);
    }
    for j in 0..4u32 {
        let x = g.read::<u32>(s + 0xf70 + 4 * j);
        g.write::<u32>(s + 0xf60 + 4 * j, x);
    }
    let c = g.read::<u32>(w(0x126));
    let mut st = Pushes::default();
    if m != 0 {
        st.push_all(&[m, c]);
        st.call(g, t(g, 0x12f, 210), 0);
        st.pop(2);
    } else if g.read::<u32>(w(0x144)) == 0 {
        let i = g.read::<u32>(w(0x14d));
        if i.cast_signed() < 0x20 {
            let v = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
            st.push_all(&[v, i]);
            st.call(g, t(g, 0x15b, 217), 0);
            st.push_all(&[v, i]);
            st.call(g, t(g, 0x16d, 255), 0);
            st.pop(4);
        }
    }
    st.push_all(&[m, c]);
    Ret::eax(st.call(g, t(g, 0x17e, 211), 0).eax)
}

/// [`summon_place_0392`] (the `rel32`s: the object start, the scaled
/// turn, the cookie check).
pub const SUMMON_PLACE_0392: Sig = Sig {
    len: 416,
    hash: 0xeab5_38b6_6bb2_d866,
    calls: &[
        (0x2f, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0xbd, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        (0x198, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0392`'s throw aim (0x10001aa0, 873 bytes; internal, no
/// arguments; an aligned /GS frame, `E` its base after the pushes: the
/// place `P` at `E + 8`, the offset `V` at `E + 0x18`, the turned `O` at
/// `E + 0x28`, the matrix `M` at `E + 0x38`). With `S = *G_S` and the
/// actor `i = S + 0x537c`: its place `P` (slot 262) with `y` = `S +
/// 0xf74`, `V` = `(0, h, s + K, 1)` (`h` -80, or -30 when `S + 0x539c` is
/// not -1; `s` the actor's scale,
/// [`actor_scale_by`](crate::s4_small::actor_scale_by)), `M` = slot 235
/// turned by X 0 and Y `S + 0x5378`, `P += M V` (slots 229, 248); `S +
/// 0xfa0..` = `(P, 1)`, `S + 0xfb4` = the yaw; the steps `n` = the
/// distance from `S + 0xf90..` (`_libm_sse2_sqrt_precise`) times `K2`,
/// truncated, at least 35, into `S + 0x538c` and `+0x5390`; the actors'
/// mean `T = S + 0xfc0` ([`mean_position`](crate::sb_misc::mean_position))
/// with `T.y` = `S + 0xf74` when slot 520's id is `0x18b0046`, else the
/// first selected character's slot 12 height; then from `d = T - P`:
/// `S + 0xfb0` = `atan2(d.y, |d.xz|)` (the square root's double) and
/// `S + 0xfb4` = `K3 - atan2(d.z, d.x)` wrapped once, `S + 0xfb8` 0. Each slot
/// call passes the words the original leaves on the stack. The
/// original's `eax` is the `_CIatan2` import's leftover (junk; 0 here).
#[allow(clippy::too_many_lines)] // the place, the steps, the aim
pub fn throw_aim_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x80);
    let (p, v, o, m) = (e + 8, e + 0x18, e + 0x28, e + 0x38);
    let s = g.read::<u32>(w(0x1a));
    let h = if g.read::<u32>(s + 0x539c) == u32::MAX {
        0xc2a0_0000u32
    } else {
        0xc1f0_0000
    };
    g.write::<u32>(v + 4, h);
    let i = g.read::<u32>(s + 0x537c);
    let chr = actor_chr_0392(g, img, f, i, (0x44, 0x4b, 0x56));
    let mut st = Pushes::default();
    st.push_all(&[p + 8, p + 4, p, chr]);
    st.call(g, t(g, 0x6b, 262), 0);
    let yaw = g.read::<u32>(s + 0x5378);
    let i = g.read::<u32>(s + 0x537c);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    g.write::<u32>(e, yaw);
    g.write::<u32>(v, 0);
    let sc = Frame {
        at: img.rel32_target(f + 0x9c),
        ecx: i,
        ..Frame::default()
    };
    let r = crate::s4_small::actor_scale_by(g, img, &sc);
    let r = r.xmm0.map_or(0.0, |x| f32::from_bits(x.reg() as u32));
    let d = add(r, k(g, 0xa4));
    st.push(m);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    g.write::<f32>(v + 8, d);
    let y = g.read::<u32>(s + 0xf74);
    g.write::<u32>(p + 4, y);
    st.call(g, t(g, 0xae, 235), 0);
    st.pop(4);
    st.set_top(0);
    st.push_all(&[m, m]);
    st.call(g, t(g, 0xe7, 328), 0);
    st.pop(2);
    st.set_top(g.read::<u32>(e));
    st.push_all(&[m, m]);
    st.call(g, t(g, 0x108, 329), 0);
    st.push_all(&[v, m, o]);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    st.call(g, t(g, 0x124, 229), 0);
    st.push_all(&[o, p, p]);
    st.call(g, t(g, 0x144, 248), 0);
    let (px, py, pz) = (g.read::<f32>(p), g.read::<f32>(p + 4), g.read::<f32>(p + 8));
    g.write::<f32>(s + 0xfa0, px);
    g.write::<f32>(s + 0xfa4, py);
    g.write::<f32>(s + 0xfa8, pz);
    g.write::<u32>(s + 0xfac, 0x3f80_0000);
    let yaw = g.read::<u32>(s + 0x5378);
    g.write::<u32>(s + 0xfb4, yaw);
    let dx = sub(g.read::<f32>(s + 0xf90), px);
    let dy = sub(g.read::<f32>(s + 0xf94), py);
    let dz = sub(g.read::<f32>(s + 0xf98), pz);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let sum = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
    // _libm_sse2_sqrt_precise of the double, rounded back to f32
    #[allow(clippy::cast_possible_truncation)]
    let dist = f64::from(sum).sqrt() as f32;
    let kn = mul(dist, k(g, 0x1e8));
    // cvttss2si (0x80000000 for NaN and out of range)
    #[allow(clippy::cast_possible_truncation)]
    let n = if kn.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&kn) {
        i32::MIN
    } else {
        kn as i32
    };
    let n = n.max(0x23);
    g.write::<i32>(s + 0x538c, n);
    g.write::<i32>(s + 0x5390, n);
    let tp = s + 0xfc0;
    crate::sb_misc::mean_position(g, img, img.rel32_target(f + 0x208), e.wrapping_sub(0xc), tp);
    if g.call(t(g, 0x20d, 520), 0, &[]).eax == 0x018b_0046 {
        let y = g.read::<u32>(s + 0xf74);
        g.write::<u32>(tp + 4, y);
    } else {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x22f))));
        let c = g.read::<u32>(w(0x239).wrapping_add(4 * b));
        let v = st0_f32(g.call(t(g, 0x23e, 12), 0, &[c]).st0);
        g.write::<f32>(tp + 4, v);
    }
    let dy = sub(g.read::<f32>(tp + 4), g.read::<f32>(p + 4));
    let dx = sub(g.read::<f32>(tp), g.read::<f32>(p));
    let dz = sub(g.read::<f32>(tp + 8), g.read::<f32>(p + 8));
    let flat = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt();
    // fstp qword; cvtpd2ps
    #[allow(clippy::cast_possible_truncation)]
    let pitch = g.atan2(f64::from(dy), flat) as f32;
    g.write::<f32>(s + 0xfb0, pitch);
    #[allow(clippy::cast_possible_truncation)]
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let y = sub(k(g, 0x30f), a);
    let y = wrap_once(g, img, f, y, [0x31e, 0x328, 0x332, 0x33f]);
    g.write::<f32>(s + 0xfb4, y);
    g.write::<u32>(s + 0xfb8, 0);
    Ret::eax(0)
}

/// [`throw_aim_0392`] (the `rel32`s: the actor's scale, the square
/// roots, the mean, `_CIatan2` twice, the cookie check).
pub const THROW_AIM_0392: Sig = Sig {
    len: 873,
    hash: 0xc631_90b3_5c2b_cbe3,
    calls: &[
        (0x9c, Callee::Sig(&crate::s4_small::ACTOR_SCALE_BY)),
        (0x1d7, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x208, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x2a9, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x2bc, Callee::Import("_CIatan2")),
        (0x2fd, Callee::Import("_CIatan2")),
        (0x361, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0392`'s aim set-up (0x10004100, 254 bytes; internal, no
/// arguments): `*G_S` = the block `B`; the caster's place (slot 262
/// `(*G_C, ...)` into `G_P..`, `w` 1.0); the index of slot 520's id in
/// the DLL's -1-ended table into `G_I`; the nearest actor, the glide
/// target and start, the throw aim ([`nearest_actor_0392`],
/// [`glide_target_0392`], [`glide_start_0392`], [`throw_aim_0392`]);
/// then slot 523 `(k, a_k)` for `k` 0..3 (`a_0` the actor `*G_J` as
/// [`glide_run_0392`] picks it, then three words). Each slot call passes
/// the words the original leaves on the stack. Returns the last slot's
/// `eax`.
pub fn aim_setup_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let c = g.read::<u32>(w(0x1e));
    g.write::<u32>(w(0x24), w(0x28));
    g.call(t(g, 0x1, 262), 0, &[c, w(0x18), w(0x13), w(0xe)]);
    g.write::<u32>(w(0x35), 0x3f80_0000);
    let id = g.call(t(g, 0x2f, 520), 0, &[]).eax;
    let i = crate::r34::find_word(g, w(0x54), id);
    g.write::<u32>(w(0x76), i);
    // push esi; push edi; the call
    let inner = Frame {
        esp: fr.esp.wrapping_sub(0xc),
        ..*fr
    };
    for (op, port) in [
        (
            0x7b,
            nearest_actor_0392 as fn(&mut G, &Image, &Frame) -> Ret,
        ),
        (0x80, glide_target_0392),
        (0x85, glide_start_0392),
        (0x8a, throw_aim_0392),
    ] {
        let call = Frame {
            at: img.rel32_target(f + op),
            ..inner
        };
        port(g, img, &call);
    }
    let j = g.read::<u32>(w(0x90));
    let a = if j == u32::MAX {
        g.read::<u32>(w(0x9c))
    } else {
        u32::from(g.read::<u8>(g.read::<u32>(w(0xa3)).wrapping_add(j)))
    };
    let mut st = Pushes::default();
    st.push_all(&[a, 0]);
    st.call(g, t(g, 0xad, 523), 0);
    let mut eax = 0;
    for (k, (op, x)) in (1u32..).zip([(0xc2, 0xbd), (0xd7, 0xd2), (0xec, 0xe7)]) {
        st.push_all(&[g.read::<u32>(w(x)), k]);
        eax = st.call(g, t(g, op, 523), 0).eax;
    }
    Ret::eax(eax)
}

/// [`aim_setup_0392`] (the `rel32`s: the four steps).
pub const AIM_SETUP_0392: Sig = Sig {
    len: 254,
    hash: 0x128f_cdc6_f92b_2119,
    calls: &[
        (0x7b, Callee::Sig(&NEAREST_ACTOR_0392)),
        (0x80, Callee::Sig(&GLIDE_TARGET_0392)),
        (0x85, Callee::Sig(&GLIDE_START_0392)),
        (0x8a, Callee::Sig(&THROW_AIM_0392)),
    ],
    imports: &[],
};

/// `magic_0392`'s object host start (0x100010a0, 292 bytes):
/// [`start`](crate::l5_objhost::start)'s `magic_0610` shape calling
/// [`aim_setup_0392`].
pub const START_0392: Sig = Sig {
    len: 292,
    hash: 0x8728_2a0b_a314_536e,
    calls: &[
        (0x0d, Callee::Sig(&crate::l5_objhost::OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&crate::l5_objhost::TASKS_INIT)),
        (0xc7, Callee::Sig(&AIM_SETUP_0392)),
    ],
    imports: &[],
};

/// `magic_0392`'s start-up (0x10003d60, 778 bytes; internal, no
/// arguments, run by its load step): the actors' table `T` (16 words)
/// cleared, four words set; slot 719 `(PART, 8, 0)` into `G_H`, slot 711
/// `(h, P2, 0)` with the lock `**G_L` 1, slot 710 `(ENV, h, BUF,
/// 0x40000)`; four words 0; the model's mesh flags
/// ([`mesh_flags`](crate::o4_fam4::mesh_flags), `h`); `S + 0x5398` = slot
/// 622 `(*G_X, b)` (`b` the selection's byte); slot 537 `(*G_A, 2, 0, a,
/// 10, 4)` (`a` the actor `S + 0x537c` as [`glide_run_0392`] picks it);
/// `S + 0x5394` -1, `S + 0x53a0` 0, a block of words, slot 271 `(4)`; two
/// task nodes (kinds 1 and 2) appended to the task list
/// ([`take_pair_0266`]'s shape). Each slot call passes the words the
/// original leaves on the stack. Returns 0.
pub fn start_up_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    for o in [0xfu32, 0x17, 0x1f, 0x27, 0x2f, 0x37, 0x3f, 0x47] {
        g.write::<u32>(w(o), 0);
        g.write::<u32>(w(o) + 4, 0);
    }
    let mut st = Pushes::default();
    st.push_all(&[0, 8, w(0x54)]);
    g.write::<u32>(w(0x5a), 0);
    for (dst, src) in [(0x64, 0x68), (0x6e, 0x72), (0x78, 0x7c)] {
        g.write::<u32>(w(dst), w(src));
    }
    let h = st.call(g, t(g, 0x1, 719), 0).eax;
    let lock = g.read::<u32>(w(0x84));
    g.write::<u32>(w(0x8b), h);
    let saved = g.read::<u32>(lock);
    st.push_all(&[0, w(0x92), h]);
    g.write::<u32>(lock, 1);
    st.call(g, t(g, 0x98, 711), 0);
    let lock = g.read::<u32>(w(0xab));
    g.write::<u32>(lock, saved);
    let h = g.read::<u32>(w(0xc2));
    st.push_all(&[0x4_0000, w(0xbc), h, w(0xcd)]);
    st.call(g, t(g, 0xb7, 710), 0);
    let model = g.read::<u32>(w(0xd5));
    for o in [0xdbu32, 0xe5, 0xef, 0xf9] {
        g.write::<u32>(w(o), 0);
    }
    crate::o4_fam4::mesh_flags(g, img, img.rel32_target(f + 0x102), model);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x107))));
    let s = g.read::<u32>(w(0x10d));
    st.push_all(&[b, g.read::<u32>(w(0x11c))]);
    let r = st.call(g, t(g, 0x116, 622), 0).eax;
    st.pop(12);
    let i = g.read::<u32>(s + 0x537c);
    g.write::<u32>(s + 0x5398, r);
    let c = g.read::<u32>(w(0x139));
    let a = if i == u32::MAX {
        c
    } else {
        u32::from(g.read::<u8>(g.read::<u32>(w(0x147)).wrapping_add(i)))
    };
    st.push_all(&[4, 0xa, a, 0, 2, c]);
    st.call(g, t(g, 0x157, 537), 0);
    g.write::<u32>(s + 0x5394, u32::MAX);
    g.write::<u32>(s + 0x53a0, 0);
    g.write::<u32>(w(0x183), 1);
    g.write::<u32>(w(0x18d), w(0x191));
    g.write::<u32>(w(0x197), 0);
    g.write::<u16>(w(0x1a2), 0);
    g.write::<u8>(w(0x1aa), 1);
    st.push(4);
    st.call(g, t(g, 0x169, 271), 0);
    take_pair_0266(
        g,
        img,
        f,
        [
            0x1bc, 0x1d9, 0x1fe, 0x223, 0x22e, 0x234, 0, 0x248, 0x25d, 0x286, 0x28e, 0x2bf, 0x2ca,
            0x234, 0x2f9, 0x2e9,
        ],
        1,
        2,
    );
    Ret::eax(0)
}

/// [`start_up_0392`] (the `rel32`: the mesh flags).
pub const START_UP_0392: Sig = Sig {
    len: 778,
    hash: 0xb43d_ce96_d4be_4d56,
    calls: &[(0x102, Callee::Sig(&crate::o4_fam4::MESH_FLAGS_0392_4070))],
    imports: &[],
};

/// `magic_0392`'s load step (0x100012c0, the 118-byte shape) running
/// [`start_up_0392`].
pub fn load_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0392)
}

/// [`load_0392`].
pub const LOAD_0392: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0392))],
    imports: &[],
};

/// `magic_0392`'s actors' tilt (0x10003180, 630 bytes; internal, no
/// arguments; a /GS frame, the matrix `M` at `ebp - 0x44`): for each
/// actor `i` below `*G_N` with its record `R = S + 0x4c10 + 0xe0 i` (`S =
/// *G_S`) whose count `R + 0xd0` is positive: the count drops; with `R +
/// 0xdc` 1, the spin `R + 0xb4 += R[0xc4]`, the height `R + 0x34 +=` it
/// (kept at most `R[0xa4]`), `t` = count / `R[0xd4]` and the angles `R +
/// 0x80..+0x88` = `R[0x90..] (K - t) + 0 t` wrapped once; with `R + 0xd8`
/// also 1, `M` = slot 235 turned by Z, X, Y (slots 330, 328, 329 by `R +
/// 0x88`, `+0x80`, `+0x84`) and `R + 0x40` = slot 231 `(R + 0x40, R,
/// M)`; then `R + 0x70..` = `R + 0x30..`. At count 0 the actor's mode is reset
/// ([`actor_mode`](crate::sa_script::actor_mode), `edx` 0) and slot 239
/// `(*G_A, b, 0)` runs (`b` its selection byte). Each slot call passes
/// the words the original leaves on the stack. Returns the last value in
/// `eax` (the frame's cookie word `cookie ^ ebp` with no actors).
#[allow(clippy::too_many_lines)] // the counts, the angles, the matrix
pub fn actors_tilt_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let m = fr.esp.wrapping_sub(4).wrapping_sub(0x44);
    let mut r = g.read::<u32>(w(0x13)).wrapping_add(0x4c10);
    // the frame's cookie word, `cookie ^ ebp`, until an actor runs
    let mut eax = g.read::<u32>(w(0x7)) ^ fr.esp.wrapping_sub(4);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(w(0x22)).cast_signed() {
        let c = g.read::<u32>(r + 0xd0);
        eax = c;
        if c.cast_signed() > 0 {
            eax = c - 1;
            g.write::<u32>(r + 0xd0, eax);
            if g.read::<u32>(r + 0xdc) == 1 {
                let spin = add(g.read::<f32>(r + 0xc4), g.read::<f32>(r + 0xb4));
                g.write::<f32>(r + 0xb4, spin);
                let h = add(g.read::<f32>(r + 0x34), spin);
                g.write::<f32>(r + 0x34, h);
                let top = g.read::<f32>(r + 0xa4);
                // comiss h, top; jbe: kept unless above (ordered)
                if h > top {
                    g.write::<f32>(r + 0x34, top);
                }
                let n = crate::sb_sse::cvt(g.read::<u32>(r + 0xd0));
                let total = crate::sb_sse::cvt(g.read::<u32>(r + 0xd4));
                let tt = div(n, total);
                let u = sub(k(g, 0xb4), tt);
                let zero_t = mul(tt, 0.0);
                let (hi, tw, lo) = (k(g, 0x30), k(g, 0x38), k(g, 0xd0));
                let wrap = |a: f32| {
                    if a > hi {
                        sub(a, tw)
                    } else if lo > a {
                        add(a, tw)
                    } else {
                        a
                    }
                };
                for d in [0u32, 4, 8] {
                    let a = wrap(add(mul(g.read::<f32>(r + 0x90 + d), u), zero_t));
                    g.write::<f32>(r + 0x80 + d, a);
                }
                if g.read::<u32>(r + 0xd8) == 1 {
                    let mut st = Pushes::default();
                    st.push(m);
                    st.call(g, t(g, 0x162, 235), 0);
                    for (op, slot, d) in [
                        (0x181, 330, 0x88u32),
                        (0x1a3, 328, 0x80),
                        (0x1c5, 329, 0x84),
                    ] {
                        if slot != 330 {
                            st.pop(2);
                        }
                        st.set_top(g.read::<u32>(r + d));
                        st.push_all(&[m, m]);
                        st.call(g, t(g, op, slot), 0);
                    }
                    st.push_all(&[m, r, r + 0x40]);
                    st.call(g, t(g, 0x1db, 231), 0);
                }
                for j in 0..4u32 {
                    eax = g.read::<u32>(r + 0x30 + 4 * j);
                    g.write::<u32>(r + 0x70 + 4 * j, eax);
                }
            }
            if g.read::<u32>(r + 0xd0) == 0 {
                let mode = Frame {
                    at: img.rel32_target(f + 0x210),
                    // push ebp; sub esp, 0x44; push esi; push edi; the call
                    esp: fr.esp.wrapping_sub(0x54),
                    ecx: i,
                    edx: 0,
                    ..Frame::default()
                };
                crate::sa_script::actor_mode(g, img, &mode);
                let c = g.read::<u32>(w(0x216));
                let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x224)).wrapping_add(i)));
                eax = g.call(t(g, 0x230, 239), 0, &[c, b, 0]).eax;
            }
        }
        i += 1;
        r += 0xe0;
    }
    Ret::eax(eax)
}

/// [`actors_tilt_0392`] (the `rel32`s: the actor's mode, the cookie
/// check).
pub const ACTORS_TILT_0392: Sig = Sig {
    len: 630,
    hash: 0xc670_3397_2b45_5d2b,
    calls: &[
        (0x210, Callee::Sig(&crate::sa_script::ACTOR_MODE)),
        (0x26e, Callee::Any),
    ],
    imports: &[],
};

/// One axis of `magic_0392`'s throw curve from `b` towards `a` (`d = a -
/// b`; `t`, `u = K - t`; `k` the constants `K9c` (three loads), `K88`,
/// `K7`, `K54`): `(((((ut uu) K9c)(d K88) + (uu uu) d) + ((tt uu) K9c)(d
/// K7)) + ((tt ut) K9c) d) + (tt tt) K54) + b`, each step one `f32`
/// rounding in this order.
#[allow(clippy::many_single_char_names)] // the curve's own terms
fn throw_axis_0392(a: f32, b: f32, t: f32, u: f32, k: [f32; 6]) -> f32 {
    let [k1, k2, k3, k88, k7, k54] = k;
    let d = sub(a, b);
    let (tt, uu, ut) = (mul(t, t), mul(u, u), mul(u, t));
    let t1 = mul(mul(mul(ut, uu), k1), mul(d, k88));
    let t2 = mul(mul(uu, uu), d);
    let t3 = mul(mul(mul(tt, uu), k2), mul(d, k7));
    let t4 = mul(mul(mul(tt, ut), k3), d);
    let t5 = mul(mul(tt, tt), k54);
    add(add(add(add(add(t1, t2), t3), t4), t5), b)
}

/// `magic_0392`'s throw flight (0x100021d0, 861 bytes; a callback, cdecl
/// `(O)`; a frame of three locals): with `S = *G_S`, the steps `S +
/// 0x538c` drop while positive; `t` = steps / `S[0x5390]`, `u = K - t`;
/// `S + 0xf60..+0xf68` = the curve from `S + 0xf90..` to `S + 0xfa0..`
/// ([`throw_axis_0392`]; the `z` axis with a literal 0 for `K54`); the
/// yaw `S + 0xf54` = `c t + u g` wrapped once, `g` the aim `S + 0xfb4`
/// unwrapped towards `c` ([`unwrap_to`](crate::r34::unwrap_to)); the
/// matrix `S + 0xd10` turned by `S + 0xf50..`
/// ([`scale_rotate`](crate::sa_misc::scale_rotate)); at 10 steps the main
/// object and its fade start
/// ([`main_object_fade`](crate::r9_misc::main_object_fade)). Once no
/// steps are left, `S + 0xf60..` = `S + 0xfa0..`, `O + 0x56` = 1 and `O`
/// is returned; else the last call's `eax`.
pub fn throw_flight_0392<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x9));
    let n = g.read::<u32>(s + 0x538c);
    if n.cast_signed() > 0 {
        g.write::<u32>(s + 0x538c, n - 1);
    }
    let c = crate::sb_sse::cvt(g.read::<u32>(s + 0x538c));
    let total = crate::sb_sse::cvt(g.read::<u32>(s + 0x5390));
    let t = div(c, total);
    let u = sub(k(g, 0x44), t);
    let k7 = k(g, 0x7b);
    for (j, ops, k54) in [
        (0u32, [0x93u32, 0x9b, 0xe1, 0xa6], Some(0x10cu32)),
        (4, [0x148, 0x131, 0x189, 0x153], Some(0x1a0)),
        (8, [0x1f0, 0x1f8, 0x245, 0x203], None),
    ] {
        let kk = [
            k(g, ops[0]),
            k(g, ops[1]),
            k(g, ops[2]),
            k(g, ops[3]),
            k7,
            k54.map_or(0.0, |o| k(g, o)),
        ];
        let x = throw_axis_0392(
            g.read::<f32>(s + 0xfa0 + j),
            g.read::<f32>(s + 0xf90 + j),
            t,
            u,
            kk,
        );
        g.write::<f32>(s + 0xf60 + j, x);
    }
    let cur = g.read::<f32>(s + 0xf54);
    let (hi, lo) = (k(g, 0x239), k(g, 0x27f));
    let aim = crate::r34::unwrap_to(cur, g.read::<f32>(s + 0xfb4), hi, lo);
    let y = add(mul(cur, t), mul(u, aim));
    let tw = k(g, 0x2da);
    let y = if y > hi {
        sub(y, tw)
    } else if lo > y {
        add(y, k(g, 0x2e9))
    } else {
        y
    };
    g.write::<f32>(s + 0xf54, y);
    let mut eax =
        crate::sa_misc::scale_rotate(g, img, img.rel32_target(f + 0x303), s + 0xd10, s + 0xf50);
    if g.read::<u32>(s + 0x538c) == 0xa {
        let fade = Frame {
            at: img.rel32_target(f + 0x314),
            // push ebp; sub esp, 0xc; push esi; the call
            esp: fr.esp.wrapping_sub(0x18),
            ..*fr
        };
        eax = crate::r9_misc::main_object_fade(g, img, &fade).eax;
    }
    if g.read::<u32>(s + 0x538c).cast_signed() > 0 {
        return Ret::eax(eax);
    }
    for j in 0..4u32 {
        let x = g.read::<u32>(s + 0xfa0 + 4 * j);
        g.write::<u32>(s + 0xf60 + 4 * j, x);
    }
    let o = fr.arg(g, 0);
    g.write::<u8>(o.wrapping_add(0x56), 1);
    Ret::eax(o)
}

/// [`throw_flight_0392`] (the `rel32`s: the scaled turn, the main
/// object's fade).
pub const THROW_FLIGHT_0392: Sig = Sig {
    len: 861,
    hash: 0x711b_efb2_09cb_b5a8,
    calls: &[
        (0x303, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        (0x314, Callee::Sig(&crate::r9_misc::MAIN_OBJECT_FADE_0392)),
    ],
    imports: &[],
};
