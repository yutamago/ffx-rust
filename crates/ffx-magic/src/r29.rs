//! Round 29's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies of ported functions bound by their own
//! `Sig`.

use crate::ob_model::st0_f32;
use crate::r27::{ThreeObj, ThreeObjects, Turn};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// `magic_0511`'s three objects (0x10003260, 1133 bytes; a table callback,
/// no arguments): [`three_objects_0589`](crate::r27::three_objects_0589)
/// with the yaw at `S + 0x2258`, the place copied from `S + 0x1a60`, the
/// places kept at `S + 0x19b0 + 0x20 i`, and each object's height raised by
/// the DLL's float `*G_H` after the constant is taken off.
pub fn three_objects_0511<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let l = ThreeObjects {
        s: 0x1b,
        yaw: 0x2258,
        src: 0x1a60,
        keep: 0x19b0,
        objs: [
            ThreeObj {
                o: 0x980,
                part: 8,
                sc: 0x14,
                c: 0x4c,
                x: 0xc184_0000,
                z: 0xbf4c_cccd,
                ops: [0x8b, 0xb5, 0xd4, 0xed, 0x105],
                turn: Turn::Wrap {
                    add: false,
                    ops: [0x15a, 0x164, 0x117, 0x174, 0x117],
                },
                ky: 0x19d,
                h: Some(0x1ac),
            },
            ThreeObj {
                o: 0x850,
                part: 7,
                sc: 0x185,
                c: 0x1c1,
                x: 0,
                z: 0x4178_0000,
                ops: [0x200, 0x22a, 0x249, 0x262, 0x27a],
                turn: Turn::Kept,
                ky: 0x2c7,
                h: Some(0x2e0),
            },
            ThreeObj {
                o: 0x720,
                part: 6,
                sc: 0x2f8,
                c: 0x2ff,
                x: 0x4184_0000,
                z: 0xbf4c_cccd,
                ops: [0x33e, 0x368, 0x387, 0x3a0, 0x3b8],
                turn: Turn::Wrap {
                    add: true,
                    ops: [0x405, 0x40f, 0x419, 0x423, 0x430],
                },
                ky: 0x44b,
                h: Some(0x455),
            },
        ],
    };
    crate::r27::three_objects_with(g, img, fr, &l)
}

/// [`three_objects_0511`] (1133 bytes, 1 overlay).
pub const THREE_OBJECTS_0511: Sig = Sig {
    len: 1133,
    hash: 0xb11f_b6d3_49dc_e42a,
    calls: &[
        (0x4c, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x1c1, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x2ff, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        // __security_check_cookie
        (0x465, Callee::Any),
    ],
    imports: &[],
};

/// The operands of one block of [`fading_summons_0511`]: slot 235, slot
/// 277 (selector, actor table, slot), its constant, slot 15 (selector,
/// actor table, slot), 16, 17, 330/328/329, slot 31 (selector, actor
/// table, slot).
struct FadeBlock0511 {
    s235: u32,
    a277: [u32; 3],
    k277: u32,
    a15: [u32; 3],
    s16: u32,
    s17: u32,
    turn: [u32; 3],
    a31: [u32; 3],
}

const FADE_BLOCKS_0511: [FadeBlock0511; 3] = [
    FadeBlock0511 {
        s235: 0x26,
        a277: [0x47, 0x52, 0x57],
        k277: 0x6f,
        a15: [0x82, 0x8d, 0x92],
        s16: 0xa9,
        s17: 0xc5,
        turn: [0xd5, 0x10e, 0x12d],
        a31: [0x203, 0x213, 0x218],
    },
    FadeBlock0511 {
        s235: 0x222,
        a277: [0x236, 0x241, 0x246],
        k277: 0x25e,
        a15: [0x271, 0x27c, 0x281],
        s16: 0x298,
        s17: 0x2b4,
        turn: [0x2d1, 0x2fd, 0x31c],
        a31: [0x3d2, 0x3dd, 0x3e2],
    },
    FadeBlock0511 {
        s235: 0x3ec,
        a277: [0x400, 0x40b, 0x410],
        k277: 0x428,
        a15: [0x43b, 0x446, 0x44b],
        s16: 0x462,
        s17: 0x47e,
        turn: [0x49b, 0x4c7, 0x4e6],
        a31: [0x57a, 0x58a, 0x58f],
    },
];

/// `magic_0511`'s fading summons (0x100036d0, 1442 bytes; a task
/// callback, cdecl `(task)`; an `ebp` frame keeping a float in the
/// argument's own word). With `S = *G_S`: the count `task + 0x2c` one
/// down while above 0, `s` = it over `K`; for the third, fourth and fifth
/// selected actors the model matrices `M = S + 0x18c0 + 0x40 i`: a unit
/// one (slot 235), its diagonal the actor's scale (slot 277, ST0) times
/// `K'`, then times the actor's three scales (slots 15, 16, 17), turned
/// by `S + 0x1a08 + 0x20 i`, `+0x1a00`, `+0x1a04` (slots 330, 328, 329);
/// every term then times `c` and slot 31 `(actor, v)`: with `u = K0 - s`,
/// `z = s K2`, the first `c = z + u K1`, `v = s K3 + u K1`; the second
/// `c' = u K4 + z`, `v' = u K4 + s K3`; the third `c'` and `v'` again. The
/// third block's scales go through the argument's word (`fstp`).
///
/// Returns the last slot 31's `eax`.
pub fn fading_summons_0511<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let task = fr.arg(g, 0);
    let argw = fr.esp + 4;
    let s = g.read::<u32>(w(0xf));
    let n = g.read::<i32>(task + 0x2c);
    if n > 0 {
        g.write::<i32>(task + 0x2c, n - 1);
    }
    let sv = div(cvt(g.read::<u32>(task + 0x2c)), k(g, 0x2e));
    g.write::<f32>(argw, sv);
    let terms = [0u32, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28];
    let (mut u, mut z, mut x3) = (0.0, 0.0, 0.0);
    let (mut c, mut v) = (0.0, 0.0);
    let mut eax = 0;
    for (i, b) in (0u32..).zip(&FADE_BLOCKS_0511) {
        let byte = 3 + i;
        let actor = |g: &G, sel: u32, tbl: u32| {
            let x = u32::from(g.read::<u8>(g.read::<u32>(w(sel)) + byte));
            g.read::<u32>(w(tbl).wrapping_add(4 * x))
        };
        // the third block keeps its scales in the argument's word
        let keep = |g: &mut G, x: f32| {
            if i == 2 {
                g.write::<f32>(argw, x);
            }
        };
        let m = s + 0x18c0 + 0x40 * i;
        g.call(t(g, b.s235, 235), 0, &[m]);
        let a2 = actor(g, b.a277[0], b.a277[1]);
        let x = st0_f32(g.call(t(g, b.a277[2], 277), 0, &[a2]).st0);
        keep(g, x);
        let x = mul(x, k(g, b.k277));
        for o in [0x28u32, 0x14, 0] {
            g.write::<f32>(m + o, x);
        }
        let a3 = actor(g, b.a15[0], b.a15[1]);
        for (o, (tb, n)) in
            [0u32, 0x14, 0x28]
                .into_iter()
                .zip([(b.a15[2], 15u32), (b.s16, 16), (b.s17, 17)])
        {
            let r = st0_f32(g.call(t(g, tb, n), 0, &[a3]).st0);
            keep(g, r);
            let y = g.read::<f32>(m + o);
            // the first block's `+0x28` multiplies the other way round
            let p = if i == 0 && o == 0x28 {
                mul(r, y)
            } else {
                mul(y, r)
            };
            g.write::<f32>(m + o, p);
        }
        for (tb, (n, d)) in
            b.turn
                .into_iter()
                .zip([(330u32, 0x1a08u32), (328, 0x1a00), (329, 0x1a04)])
        {
            let ang = g.read::<u32>(s + d + 0x20 * i);
            // call width: 3
            g.call(t(g, tb, n), 0, &[m, m, ang]);
        }
        match i {
            0 => {
                u = sub(k(g, 0x147), sv);
                let x2 = mul(u, k(g, 0x15b));
                z = mul(sv, k(g, 0x16b));
                c = add(z, x2);
                x3 = mul(sv, k(g, 0x17f));
                g.write::<f32>(argw, x3);
                v = add(x3, x2);
            }
            1 => {
                let x2 = mul(u, k(g, 0x33b));
                c = add(x2, z);
                v = add(x2, x3);
            }
            _ => {}
        }
        for o in terms {
            let y = mul(c, g.read::<f32>(m + o));
            g.write::<f32>(m + o, y);
        }
        let a4 = actor(g, b.a31[0], b.a31[1]);
        // call width: 2
        eax = g.call(t(g, b.a31[2], 31), 0, &[a4, v.to_bits()]).eax;
    }
    Ret::eax(eax)
}

/// [`fading_summons_0511`] (1442 bytes, 1 overlay).
pub const FADING_SUMMONS_0511: Sig = Sig {
    len: 1442,
    hash: 0x44eb_cf63_3229_a64f,
    calls: &[],
    imports: &[],
};

/// The slot 314 table operands, the constants' operands and the kind of
/// one pass of [`spray_0511`]'s particle fill.
struct SprayOps {
    /// slot 314's seven calls (`1.0`, `1.0`, `1.0`, `0.1`, `0.05`, `0.1`,
    /// `0.2`)
    r: [u32; 7],
    /// the constants: `+0x14`'s, `+0x20`'s, `+0x24`'s, `+0x28`'s, the
    /// last draw's two
    k: [u32; 6],
    kind: u32,
}

/// One particle of [`spray_0511`]: the record `p` gets the texture, the
/// point `l` moved by three random 1.0s, random `+0x10` (0.1), `+0x14`
/// (0.05 plus `K0`), `+0x18` (0.1), their products with `K1`/`K2`/`K3`
/// in `+0x20..`, `w` 1.0, `+0x3c` = (rand 0.2 + `K4`) `K5`, the kind.
/// Returns the last slot 314's `eax`.
fn spray_particle<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (p, tex, l): (u32, u32, u32),
    ops: &SprayOps,
) -> u32 {
    use crate::sb_sse::{add, mul};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let random = |g: &mut G, o: u32, x: u32| {
        // call width: 1
        let out = g.call(slot_target(g, w(o), 314), 0, &[x]);
        (st0_f32(out.st0), out.eax)
    };
    g.write::<u16>(p + 0x38, 0xffff);
    g.write::<u32>(p + 0x30, tex);
    g.write::<u32>(p + 0x34, 1);
    g.write::<u16>(p + 0x3a, 0x200);
    for i in 0..4 {
        let x = g.read::<u32>(l + 4 * i);
        g.write::<u32>(p + 4 * i, x);
    }
    for (i, o) in (0u32..).zip(&ops.r[..3]) {
        let (r, _) = random(g, *o, ONE);
        g.write::<f32>(p + 4 * i, add(rf(g, p + 4 * i), r));
    }
    let (r, _) = random(g, ops.r[3], 0x3dcc_cccd);
    g.write::<f32>(p + 0x10, r);
    let (r, _) = random(g, ops.r[4], 0x3d4c_cccd);
    g.write::<f32>(p + 0x14, add(r, k(g, ops.k[0])));
    let (r, _) = random(g, ops.r[5], 0x3dcc_cccd);
    g.write::<f32>(p + 0x20, mul(rf(g, p + 0x10), k(g, ops.k[1])));
    g.write::<f32>(p + 0x18, r);
    g.write::<f32>(p + 0x24, mul(rf(g, p + 0x14), k(g, ops.k[2])));
    g.write::<u32>(p + 0x1c, 0);
    g.write::<f32>(p + 0x28, mul(r, k(g, ops.k[3])));
    g.write::<u32>(p + 0x2c, ONE);
    let (r, eax) = random(g, ops.r[6], 0x3e4c_cccd);
    g.write::<u32>(p + 0x4c, ops.kind);
    g.write::<u32>(p + 0x50, 0);
    g.write::<f32>(p + 0x3c, mul(add(r, k(g, ops.k[4])), k(g, ops.k[5])));
    eax
}

/// `magic_0511`'s spray (0x10003e70, 1152 bytes; a task callback, cdecl
/// `(task)`; a /GS frame: the matrix `M` at `ebp - 0x54`, the point `L`
/// at `ebp - 0x14`). With `S = *G_S`: the count `S + 0x2260` one down
/// while above 0; once it is out, the fade `S + 0x2254` goes down by 8
/// while above 8, else the task is given back
/// ([`give_back`](crate::sa_pool::give_back), its `eax` returned). Then
/// `M` a unit matrix (slot 235) with the diagonal 0.04, the point `L` of
/// the actor `*(*G_B)` (slot 219 `(b, 0)`, slot 268 `(a, 0xe, L)`) and the
/// trail drawn there ([`trail_draw`](crate::s4_misc::trail_draw), `ecx`
/// `S + 0x2030`, `edx` `M`). While the count `S + 0x2264` is above 0 (one
/// down), two pool records
/// ([`pool_take`](crate::sb_misc::pool_take); none ends with 0) become
/// particles at `L` (texture `(*G_T)[2]`, kind 0x11), and with the count
/// still above 8 two more (texture slot 719 `(P, 9, 0)`, kind 0x1c).
/// Returns the count when it was out, else the last slot 314's `eax`.
pub fn spray_0511<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(w(0x16));
    let n = g.read::<i32>(s + 0x2260);
    if n > 0 {
        g.write::<i32>(s + 0x2260, n - 1);
    }
    if g.read::<i32>(s + 0x2260) <= 0 {
        let fade = g.read::<i32>(s + 0x2254);
        if fade <= 8 {
            let eax = crate::sa_pool::give_back(g, img, call(0x46d), task);
            return Ret::eax(eax);
        }
        g.write::<i32>(s + 0x2254, fade - 8);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let (m, l) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    g.call(t(g, 0x51, 235), 0, &[m]);
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(m + o, 0x3d23_d70a);
    }
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x5e))));
    // call width: 2
    let a = g.call(t(g, 0x7e, 219), 0, &[b, 0]).eax;
    // call width: 3
    g.call(t(g, 0x92, 268), 0, &[a, 0xe, l]);
    // the words pushed so far, the return address at `ebp - 0x7c`
    let esp = ebp.wrapping_sub(0x7c);
    for (i, x) in (1u32..).zip([l, a, 0xe, l, b, 0, m]) {
        g.write::<u32>(esp + 4 * i, x);
    }
    let td = Frame {
        at: call(0xac),
        esp,
        ecx: s + 0x2030,
        edx: m,
        ..*fr
    };
    crate::s4_misc::trail_draw(g, img, &td);
    let count = g.read::<i32>(s + 0x2264);
    if count <= 0 {
        return Ret::eax(count.cast_unsigned());
    }
    g.write::<i32>(s + 0x2264, count - 1);
    let first = SprayOps {
        r: [0x108, 0x14b, 0x170, 0x196, 0x1b2, 0x1d6, 0x204],
        k: [0x1d1, 0x1fa, 0x216, 0x223, 0x25d, 0x265],
        kind: 0x11,
    };
    let mut eax = 0;
    for _ in 0..2 {
        let p = crate::sb_misc::pool_take(g, img, call(0xd1));
        if p == 0 {
            return Ret::eax(0);
        }
        let tex = g.read::<u32>(g.read::<u32>(w(0xe1)) + 8);
        eax = spray_particle(g, img, f, (p, tex, l), &first);
    }
    if g.read::<i32>(s + 0x2264) <= 8 {
        return Ret::eax(eax);
    }
    let second = SprayOps {
        r: [0x2db, 0x320, 0x345, 0x36b, 0x387, 0x3ab, 0x3d9],
        k: [0x3a6, 0x3cf, 0x3eb, 0x3f8, 0x432, 0x449],
        kind: 0x1c,
    };
    for _ in 0..2 {
        let p = crate::sb_misc::pool_take(g, img, call(0x296));
        if p == 0 {
            return Ret::eax(0);
        }
        // call width: 3
        let tex = g.call(t(g, 0x2a6, 719), 0, &[w(0x2b5), 9, 0]).eax;
        eax = spray_particle(g, img, f, (p, tex, l), &second);
    }
    Ret::eax(eax)
}

/// [`spray_0511`] (the `rel32`s: the trail draw, the pool take twice,
/// `__security_check_cookie` twice, the give-back).
pub const SPRAY_0511: Sig = Sig {
    len: 1152,
    hash: 0xd4ae_e7fa_9520_e3ab,
    calls: &[
        (0xac, Callee::Sig(&crate::s4_misc::TRAIL_DRAW)),
        (0xd1, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x296, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x464, Callee::Any),
        (0x46d, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x478, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0511`'s start-up (0x100053f0, 449 bytes) and `magic_0602`'s
/// (0x100094d0, 439 bytes), run by their load steps; no arguments, no
/// frame (`ebx` pushed): slot 718 `(NAME)`; with `n = *G_N`, the bytes
/// `G_B1 = n + 1`, `G_B2 = n + 2` and `ACTORS[n + 1]`/`[n + 2]` = slot 215
/// `(n + 1)`/`(n + 2)` (the statics' store run between); `*G_K` 3; slot
/// 719 `(P, 8, 0)` into `G_H`; the part set up
/// ([`part_setup`](crate::sb_misc::part_setup) `(BUF)`, slot 718's word
/// under it), the summons' matrices
/// ([`mats_0511`](crate::r28::mats_0511)), the reset
/// ([`reset_and_scale`](crate::s4_small::reset_and_scale)); the two
/// script blocks' words; slot 271 `(4)`; two task nodes (kinds 1 and 2,
/// [`take`](crate::sa_pool::take)) on the task list, the records block
/// after the first ([`records_init_400`](crate::sa_misc::records_init_400));
/// in the scene slot 520 names 0x18b0046 the closing store run (one
/// float in `magic_0602`, two in `magic_0511`). Returns 0.
pub fn start_up_0511<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r13_start::append_kind;
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    g.call(t(g, 0x1, 718), 0, &[w(0xd)]);
    let n = g.read::<u32>(w(0x15)).wrapping_add(1);
    let s215 = t(g, 0x1a, 215);
    g.write::<u8>(w(0x21), n as u8);
    abs_stores(g, img, f, 0x2c, 0x54);
    let r = g.call(s215, 0, &[n]).eax;
    g.write::<u32>(w(0x59).wrapping_add(4 * n), r);
    let n = n.wrapping_add(1);
    let s215 = t(g, 0x5e, 215);
    g.write::<u8>(w(0x65), n as u8);
    let r = g.call(s215, 0, &[n]).eax;
    g.write::<u32>(w(0x75).wrapping_add(4 * n), r);
    let s719 = t(g, 0x7a, 719);
    abs_stores(g, img, f, 0x80, 0x8a);
    // call width: 3
    let h = g.call(s719, 0, &[w(0x93), 8, 0]).eax;
    g.write::<u32>(w(0x9d), h);
    // push ebx; slot 718's word stays; push BUF: the return address at
    // `E - 0x10`
    let esp = fr.esp.wrapping_sub(0x10);
    g.write::<u32>(esp + 8, w(0xd));
    g.write::<u32>(esp + 4, w(0xa2));
    crate::sb_misc::part_setup(g, img, call(0xa7), esp);
    crate::r28::mats_0511(g, img, call(0xac), esp);
    let rs = Frame {
        at: call(0xb1),
        esp,
        ..Frame::default()
    };
    crate::s4_small::reset_and_scale(g, img, &rs);
    let s271 = t(g, 0xb6, 271);
    abs_stores(g, img, f, 0xba, 0x102);
    // call width: 3 (BUF and slot 718's word under it)
    g.call(s271, 0, &[4, w(0xa2), w(0xd)]);
    let n1 = crate::sa_pool::take(g, img, call(0x115), w(0x110));
    append_kind(g, img, f, n1, 1, [0x11b, 0x130, 0x14f, 0x142, 0x14f]);
    crate::sa_misc::records_init_400(g, img, call(0x154), w(0x147));
    let n2 = crate::sa_pool::take(g, img, call(0x15e), w(0x159));
    append_kind(g, img, f, n2, 2, [0x164, 0x178, 0x192, 0x18a, 0x192]);
    // call width: 0
    if g.call(t(g, 0x197, 520), 0, &[]).eax == 0x018b_0046 {
        // to the closing `xor eax, eax`
        abs_stores(g, img, f, 0x1aa, 0x1c0);
    }
    Ret::eax(0)
}

/// [`start_up_0511`] in `magic_0511`.
pub const START_UP_0511: Sig = Sig {
    len: 449,
    hash: 0xea4e_a910_57e6_02eb,
    calls: &[
        (0xa7, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0xac, Callee::Sig(&crate::r28::MATS_0511)),
        (0xb1, Callee::Sig(&crate::s4_small::RESET_AND_SCALE_511)),
        (0x115, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x154, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x15e, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// [`start_up_0511`] in `magic_0602` (one float in the closing run).
pub const START_UP_0602: Sig = Sig {
    len: 439,
    hash: 0x22f2_0936_f724_152c,
    calls: START_UP_0511.calls,
    imports: &[],
};

/// `magic_0511`'s load step (0x100012e0, the 118-byte shape) running
/// [`start_up_0511`].
pub fn load_0511<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0511)
}

/// [`load_0511`].
pub const LOAD_0511: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0511))],
    imports: &[],
};

/// `magic_0602`'s load step (0x100068a0, 130 bytes): `magic_0431`'s
/// ([`load_0431`](crate::r21::load_0431)) waiting on the DLL's three
/// sounds ([`sounds_free`](crate::m0602::sounds_free)) and running
/// [`start_up_0511`].
pub fn load_0602<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r21::load_with_0431(g, img, fr, crate::m0602::sounds_free, start_up_0511)
}

/// [`load_0602`].
pub const LOAD_0602: Sig = Sig {
    len: 130,
    hash: 0xfede_8aed_fe98_dfa9,
    calls: &[
        (0x17, Callee::Sig(&crate::m0602::SOUNDS_FREE)),
        (0x67, Callee::Sig(&START_UP_0602)),
    ],
    imports: &[],
};

/// The spin's state step (`magic_0612` 0x10009650 and `magic_0563`
/// 0x10004d20, 52 bytes; a table callback, no arguments): by the state
/// `*G_S + 0x3664`, 0 runs the set-up
/// ([`spin_setup_0612`](crate::r25::spin_setup_0612)), 1 the step
/// ([`spin_step_0563`](crate::r28::spin_step_0563)), 2 the records' move
/// ([`spin_records_move`](crate::s4_small::spin_records_move)), any other
/// nothing; every way ends in a tail jump to the models' draw
/// ([`models_around`](crate::s5_misc::models_around)), whose answer it
/// returns.
pub fn spin_states_0612<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(img.word(f + 1));
    let esp = fr.esp.wrapping_sub(4);
    let eax = match g.read::<u32>(s + 0x3664) {
        0 => crate::r25::spin_setup_0612(g, img, call(0x2b), esp),
        1 => crate::r28::spin_step_0563(g, img, call(0x21), esp),
        2 => {
            let rm = Frame {
                at: call(0x17),
                esp,
                ..*fr
            };
            crate::s4_small::spin_records_move(g, img, &rm).eax
        }
        st => st.wrapping_sub(2),
    };
    let ma = Frame {
        at: call(0x30),
        eax,
        ..*fr
    };
    crate::s5_misc::models_around(g, img, &ma)
}

/// [`spin_states_0612`] in `magic_0612`.
pub const SPIN_STATES_0612: Sig = Sig {
    len: 52,
    hash: 0x911a_885e_a511_ee5c,
    calls: &[
        (0x17, Callee::Sig(&crate::s4_small::SPIN_RECORDS_MOVE)),
        (0x1c, Callee::Sig(&crate::s5_misc::MODELS_AROUND)),
        (0x21, Callee::Sig(&crate::r28::SPIN_STEP_0563)),
        (0x26, Callee::Sig(&crate::s5_misc::MODELS_AROUND)),
        (0x2b, Callee::Sig(&crate::r25::SPIN_SETUP_0612)),
        (0x30, Callee::Sig(&crate::s5_misc::MODELS_AROUND)),
    ],
    imports: &[],
};

/// [`spin_states_0612`] in `magic_0563` (its own set-up's `Sig`).
pub const SPIN_STATES_0563: Sig = Sig {
    len: 52,
    hash: 0x911a_885e_a511_ee5c,
    calls: &[
        (0x17, Callee::Sig(&crate::s4_small::SPIN_RECORDS_MOVE)),
        (0x1c, Callee::Sig(&crate::s5_misc::MODELS_AROUND)),
        (0x21, Callee::Sig(&crate::r28::SPIN_STEP_0563)),
        (0x26, Callee::Sig(&crate::s5_misc::MODELS_AROUND)),
        (0x2b, Callee::Sig(&crate::r25::SPIN_SETUP_0563)),
        (0x30, Callee::Sig(&crate::s5_misc::MODELS_AROUND)),
    ],
    imports: &[],
};

/// [`sparks_0563`](crate::r28::sparks_0563) in `magic_0605` (0x10008f90,
/// 847 bytes): the texture at `(*G_T)[5]` and the records from `S +
/// 0x1f14` (both read from the code).
pub const SPARKS_0605: Sig = Sig {
    len: 847,
    hash: 0x00cc_ce96_da47_ec1a,
    calls: crate::r28::SPARKS_0563.calls,
    imports: &[],
};
