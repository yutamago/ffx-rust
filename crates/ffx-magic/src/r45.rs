//! Round 45's own ports (own rows of `magic_0250` .. `magic_0449`): the
//! DLLs' own code written out.

use crate::m0018::st0_f32;
use crate::ob_model::wrap_once;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
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
