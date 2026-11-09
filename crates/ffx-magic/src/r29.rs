//! Round 29's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies of ported functions bound by their own
//! `Sig`.

use crate::ob_model::st0_f32;
use crate::pushes::Pushes;
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
    spin_states_with(
        g,
        img,
        fr,
        crate::r25::spin_setup_0612,
        |g, img, fr| crate::s4_small::spin_records_move(g, img, fr).eax,
        crate::s5_misc::models_around,
    )
}

/// [`spin_states_0612`] in `magic_0605` (0x1000a0c0; the state at `*G_S +
/// 0x26b8`): its own set-up ([`spin_setup_0605`]), records' move
/// ([`move_pieces`](crate::m0605::move_pieces)) and models' draw
/// ([`models_around_0605`]).
pub fn spin_states_0605<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    spin_states_with(
        g,
        img,
        fr,
        spin_setup_0605,
        crate::m0605::move_pieces,
        models_around_0605,
    )
}

/// The state step's body: the state's displacement read from the code
/// (`+0x07`), the three states' callees and the models' draw given.
fn spin_states_with<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    setup: fn(&mut G, &Image, u32, u32) -> u32,
    moves: fn(&mut G, &Image, &Frame) -> u32,
    around: fn(&mut G, &Image, &Frame) -> Ret,
) -> Ret {
    let f = fr.at;
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(img.word(f + 1));
    let esp = fr.esp.wrapping_sub(4);
    let eax = match g.read::<u32>(s + img.word(f + 7)) {
        0 => setup(g, img, call(0x2b), esp),
        1 => crate::r28::spin_step_0563(g, img, call(0x21), esp),
        2 => {
            let rm = Frame {
                at: call(0x17),
                esp,
                ..*fr
            };
            moves(g, img, &rm)
        }
        st => st.wrapping_sub(2),
    };
    let ma = Frame {
        at: call(0x30),
        eax,
        ..*fr
    };
    around(g, img, &ma)
}

/// [`spin_states_0605`].
pub const SPIN_STATES_0605: Sig = Sig {
    len: 52,
    hash: 0xf90a_c4ad_dc27_1098,
    calls: &[
        (0x17, Callee::Sig(&crate::m0605::MOVE_PIECES)),
        (0x1c, Callee::Sig(&MODELS_AROUND_0605)),
        (0x21, Callee::Sig(&SPIN_STEP_0605)),
        (0x26, Callee::Sig(&MODELS_AROUND_0605)),
        (0x2b, Callee::Sig(&SPIN_SETUP_0605)),
        (0x30, Callee::Sig(&MODELS_AROUND_0605)),
    ],
    imports: &[],
};

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

/// The operands of one model block of [`summon_frames_0602`]: the
/// selection byte, its (selector, actor table) for the first actor, the
/// battle word, the (selector, table) for slot 277's, for 15/16/17's and
/// for 31's actor, the constants (`y`'s, 277's factor, the scale), slot
/// 31's immediate, the slot table operands of 210, 217, 255, 211, 235,
/// 277, 15, 16, 17, 330, 328, 329, 31, and which products have the slot's
/// answer first (16, 17).
struct ModelBlock0602 {
    byte: u32,
    actor: (u32, u32),
    battle: u32,
    a277: (u32, u32),
    a15: (u32, u32),
    a31: (u32, u32),
    k: [u32; 3],
    imm31: u32,
    slots: [u32; 13],
    r_first: [bool; 2],
}

const MODEL_BLOCKS_0602: [ModelBlock0602; 3] = [
    ModelBlock0602 {
        byte: 3,
        actor: (0x369, 0x37d),
        battle: 0x3a1,
        a277: (0x464, 0x46f),
        a15: (0x4a5, 0x4b0),
        a31: (0x607, 0x612),
        k: [0x42b, 0x492, 0x580],
        imm31: 0x59b,
        slots: [
            0x38e, 0x3b2, 0x3c4, 0x3dd, 0x44e, 0x474, 0x4b5, 0x4d2, 0x4f4, 0x517, 0x543, 0x562,
            0x617,
        ],
        r_first: [false, false],
    },
    ModelBlock0602 {
        byte: 4,
        actor: (0x621, 0x635),
        battle: 0x659,
        a277: (0x71c, 0x727),
        a15: (0x75d, 0x768),
        a31: (0x8bf, 0x8ca),
        k: [0x6e3, 0x74a, 0x838],
        imm31: 0x853,
        slots: [
            0x646, 0x66a, 0x67c, 0x695, 0x706, 0x72c, 0x76d, 0x78a, 0x799, 0x7cf, 0x7fb, 0x81a,
            0x8cf,
        ],
        r_first: [true, true],
    },
    ModelBlock0602 {
        byte: 5,
        actor: (0x8d9, 0x8ed),
        battle: 0x911,
        a277: (0x9d4, 0x9df),
        a15: (0xa15, 0xa20),
        a31: (0xb77, 0xb82),
        k: [0x99b, 0xa02, 0xaf0],
        imm31: 0xb0b,
        slots: [
            0x8fe, 0x922, 0x934, 0x94d, 0x9be, 0x9e4, 0xa25, 0xa42, 0xa51, 0xa87, 0xab3, 0xad2,
            0xb87,
        ],
        r_first: [true, false],
    },
];

/// `magic_0602`'s summon frames (0x100078b0, 2980 bytes; a table
/// callback, no arguments; a /GS `ebp` frame: the matrices at `ebp - 0x74`
/// and `ebp - 0xb4`, the turned offset at `ebp - 0x24`, the offset at
/// `ebp - 0x14`, slot 217's buffer at `ebp - 0x34`). With `S = *G_S`:
/// - the sounds 0x301a..0x301c: each `h` = slot 0 `(id)`, slot 26 `(id,
///   1)`, its byte `0x20 + k` and `h` into the DLL's words, slot 10 `(h,
///   0)`; `*G_K` up by 3;
/// - three points `P_i = S + 0x19b0 + 0x20 i`: `S + 0x1a60` moved by
///   `(-16.5, 0, -0.8)`, `(0, 0, 15.5)`, `(16.5, 0, -0.8)` turned by the
///   yaw `S + 0x2258` (slots 235, 328 with 0, 329, 229, 248; `w` 1.0);
/// - for the third, fourth and fifth selected actors the model matrices
///   `M = S + 0x18c0 + 0x40 i` ([`ModelBlock0602`]): slots 210 (or 217/255
///   for a null `M`, which no `S` gives) and 211; the record `S + 0x1a00 +
///   0x20 i` = `S + 0x19a0 + 0x20 i` (its `y` less `K`); a unit `M`, its
///   diagonal slot 277 times `K'`, times slots 15/16/17, turned by the
///   record's angles (slots 330, 328, 329), every term times `K''`; slot
///   31 `(actor, c)`.
///
/// Returns the last slot 31's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn summon_frames_0602<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let s = rd(g, 0x1f);
    // the three sounds: (slot 0, 26, 10's table operands; the id; the
    // byte's store; the handle's store)
    for (s0, s26, s10, id, byte, keep) in [
        (0x14u32, 0x2cu32, 0x3fu32, 0x24u32, 0x45u32, 0x4cu32),
        (0x59, 0x68, 0x7b, 0x5e, 0x81, 0x88),
        (0x95, 0xa4, 0xb7, 0x9a, 0xbd, 0xc4),
    ] {
        // call width: 1
        let h = g.call(t(g, s0, 0), 0, &[w(id)]).eax;
        // call width: 2
        g.call(t(g, s26, 26), 0, &[w(id), 1]);
        g.write::<u8>(w(byte), w(byte + 4) as u8);
        g.write::<u32>(w(keep), h);
        // call width: 2
        g.call(t(g, s10, 10), 0, &[h, 0]);
    }
    let n = rd(g, 0xd2).wrapping_add(w(0xd6) & 0xff);
    g.write::<u32>(w(0xd2), n);
    // the three points
    let yaw = g.read::<u32>(s + 0x2258);
    let (lv, lo) = (l(0x24), l(0x14));
    g.write::<u32>(lo + 4, 0);
    g.write::<u32>(lo + 0xc, ONE);
    for (i, (x, z, m, ops)) in (0u32..).zip([
        (
            0xc184_0000u32,
            0xbf4c_cccdu32,
            l(0x74),
            [0x11au32, 0x15a, 0x17c, 0x195, 0x1ad],
        ),
        (0, 0x4178_0000, l(0xb4), [0x200, 0x22d, 0x252, 0x26e, 0x286]),
        (
            0x4184_0000,
            0xbf4c_cccd,
            l(0x74),
            [0x2d6, 0x300, 0x322, 0x33b, 0x353],
        ),
    ]) {
        let p = s + 0x19b0 + 0x20 * i;
        for j in 0..4 {
            let v = g.read::<u32>(s + 0x1a60 + 4 * j);
            g.write::<u32>(p + 4 * j, v);
        }
        g.write::<u32>(lo, x);
        g.write::<u32>(lo + 8, z);
        g.call(t(g, ops[0], 235), 0, &[m]);
        // call width: 3
        g.call(t(g, ops[1], 328), 0, &[m, m, 0]);
        // call width: 3
        g.call(t(g, ops[2], 329), 0, &[m, m, yaw]);
        g.write::<u32>(lo + 0xc, ONE);
        // call width: 3
        g.call(t(g, ops[3], 229), 0, &[lv, m, lo]);
        // call width: 3
        g.call(t(g, ops[4], 248), 0, &[p, p, lv]);
        g.write::<u32>(p + 0xc, ONE);
    }
    // the three model blocks
    let mut eax = 0;
    for (i, b) in (0u32..).zip(&MODEL_BLOCKS_0602) {
        let sl = b.slots;
        let pick = |g: &G, (sel, tbl): (u32, u32)| {
            let idx = u32::from(g.read::<u8>(rd(g, sel) + b.byte));
            (idx, g.read::<u32>(w(tbl).wrapping_add(4 * idx)))
        };
        let m = s + 0x18c0 + 0x40 * i;
        let (idx, act) = pick(g, b.actor);
        if m == 0 {
            // `S + 0x18c0` null: no caller's `S` gets here
            if rd(g, b.battle) == 0 && idx.cast_signed() < 0x20 {
                let buf = l(0x34);
                // call width: 2
                g.call(t(g, sl[1], 217), 0, &[idx, buf]);
                // call width: 2
                g.call(t(g, sl[2], 255), 0, &[idx, buf]);
            }
        } else {
            // call width: 2
            g.call(t(g, sl[0], 210), 0, &[act, m]);
        }
        // call width: 2
        g.call(t(g, sl[3], 211), 0, &[act, m]);
        let r = s + 0x1a00 + 0x20 * i;
        for j in 0..8 {
            let v = g.read::<u32>(s + 0x19a0 + 0x20 * i + 4 * j);
            g.write::<u32>(r + 4 * j, v);
        }
        let y = sub(g.read::<f32>(s + 0x19b4 + 0x20 * i), k(g, b.k[0]));
        g.write::<f32>(r + 0x14, y);
        g.call(t(g, sl[4], 235), 0, &[m]);
        let (_, a2) = pick(g, b.a277);
        let x = st0_f32(g.call(t(g, sl[5], 277), 0, &[a2]).st0);
        let x = mul(x, k(g, b.k[1]));
        for o in [0x28u32, 0x14, 0] {
            g.write::<f32>(m + o, x);
        }
        let (_, a3) = pick(g, b.a15);
        let x = st0_f32(g.call(t(g, sl[6], 15), 0, &[a3]).st0);
        g.write::<f32>(m, mul(x, g.read::<f32>(m)));
        for ((tb, n), (o, first)) in [(sl[7], 16u32), (sl[8], 17)]
            .into_iter()
            .zip([0x14u32, 0x28].into_iter().zip(b.r_first))
        {
            let x = st0_f32(g.call(t(g, tb, n), 0, &[a3]).st0);
            let y = g.read::<f32>(m + o);
            g.write::<f32>(m + o, if first { mul(x, y) } else { mul(y, x) });
        }
        for (tb, (n, d)) in
            [sl[9], sl[10], sl[11]]
                .into_iter()
                .zip([(330u32, 8u32), (328, 0), (329, 4)])
        {
            let ang = g.read::<u32>(r + d);
            // call width: 3
            g.call(t(g, tb, n), 0, &[m, m, ang]);
        }
        let c = k(g, b.k[2]);
        for o in [0u32, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28] {
            let x = mul(g.read::<f32>(m + o), c);
            g.write::<f32>(m + o, x);
        }
        let (_, a4) = pick(g, b.a31);
        // call width: 2
        eax = g.call(t(g, sl[12], 31), 0, &[a4, w(b.imm31)]).eax;
    }
    Ret::eax(eax)
}

/// [`summon_frames_0602`] (the `rel32`: `__security_check_cookie`).
pub const SUMMON_FRAMES_0602: Sig = Sig {
    len: 2980,
    hash: 0xa8d0_947f_86ec_5295,
    calls: &[(0xb9c, Callee::Any)],
    imports: &[],
};

/// [`spin_step_0563`](crate::r28::spin_step_0563) in `magic_0605`
/// (0x10009990, 1581 bytes): its own record layout (read from the code)
/// and its object started by
/// [`start_object_at`](crate::s4_misc::start_object_at).
pub const SPIN_STEP_0605: Sig = Sig {
    len: 1581,
    hash: 0x08ce_a074_f43d_2713,
    calls: &[
        (0x265, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x625, Callee::Any),
    ],
    imports: &[],
};

/// The spin set-up of `magic_0605` (0x100092e0) and `magic_0307`
/// (0x10005da0), 1591 bytes (family P011; internal, no arguments, run by
/// the state step): [`spin_setup_0612`](crate::r25::spin_setup_0612)'s
/// steps on each copy's record layout (read from the code), with three
/// adds in the other operand order and its own closing steps
/// ([`spin_setup_with`](crate::r25::spin_setup_with) `late`).
pub fn spin_setup_0605<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    crate::r25::spin_setup_with(g, img, f, esp, true)
}

/// [`spin_setup_0605`] in `magic_0605`.
pub const SPIN_SETUP_0605: Sig = Sig {
    len: 1591,
    hash: 0xd7a9_ca43_c9c9_a33c,
    calls: &[
        (0x309, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x62f, Callee::Any),
    ],
    imports: &[],
};

/// [`spin_setup_0605`] in `magic_0307`.
pub const SPIN_SETUP_0307: Sig = Sig {
    len: 1591,
    hash: 0x066d_82d6_e030_39a2,
    calls: SPIN_SETUP_0605.calls,
    imports: &[],
};

/// `magic_0605`'s models drawn around the spin (0x10008030, 608 bytes; a
/// table callback, no arguments; an 8-aligned /GS frame, `B` = `esp`
/// after the three pushes: the matrix `M` at `B + 0x18`, the points `V1`
/// at `B + 0x48` and `V2` at `B + 0x5c`):
/// [`models_around_0307`](crate::s5_misc::models_around_0307)'s steps with
/// `magic_0605`'s layout and operand orders. The sound record
/// ([`effect_record`](crate::o4_fam4::effect_record)); with `S = *G_S` and
/// the model table `T = *G_T` (read once): for the records `R` (0x60 apart
/// from `S + 0x1f28`) and the models `i` = 0x1a0 .. 0x3e0 (0x20 apart): a
/// unit `M` (slot 235) turned by `R + 4`, `R - 4`, `R` (slots 330, 328,
/// 329), slot 231 `(M, P, M)` with `P = S + 0x25f4`; `V2` = slot 229 `(V2,
/// M, R - 0x14)`, `V1` = slot 229 `(V1, P, R - 0x14)`; `V1 = R + 0x1c +
/// (Q + (V1 - V2) K)` per axis (`Q = S + 0x2674`), `M`'s three rows times
/// `K`, and the model `T[i + 0x1c]` drawn with it
/// ([`part_matrix_handover`](crate::o6_fam_pairs3::part_matrix_handover)).
/// Returns 0x3e0 (the models' end).
pub fn models_around_0605<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // push ebp; and esp, -8; sub esp, 0x64; three pushes
    let top = fr.esp.wrapping_sub(4) & !7;
    let b0 = top.wrapping_sub(0x70);
    let call_esp = b0.wrapping_sub(4);
    let sound = Frame {
        at: img.rel32_target(f + 0x1e),
        esp: call_esp,
        ..*fr
    };
    crate::o4_fam4::effect_record(g, img, &sound);
    let s = g.read::<u32>(w(0x17));
    let (m, v1, v2) = (b0 + 0x18, b0 + 0x48, b0 + 0x5c);
    let p = s.wrapping_add(w(0xb2));
    let q = s.wrapping_add(w(0x11c));
    let mut r = s.wrapping_add(w(0x24));
    let mut i = w(0x30);
    let models = g.read::<u32>(w(0x36));
    let k = g.read::<f32>(w(0x114));
    loop {
        g.write::<u32>(b0 + 0x14, r);
        g.write::<u32>(b0 + 0x10, i);
        let mut st = Pushes::default();
        st.push(m);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x40, 235), 0);
        for (table, slot, a) in [
            (0x5du32, 330u32, 4u32),
            (0x7d, 328, 0xffff_fffc),
            (0x9c, 329, 0),
        ] {
            st.set_top(g.read::<u32>(r.wrapping_add(a)));
            st.push_all(&[m, m]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, table, slot), 0);
            st.pop(2);
        }
        // call width: 3
        g.call(t(g, 0xbd, 231), 0, &[m, p, m]);
        let src = r.wrapping_sub(0x14);
        // call width: 3
        g.call(t(g, 0xdb, 229), 0, &[v2, m, src]);
        // call width: 3
        g.call(t(g, 0xf8, 229), 0, &[v1, p, src]);
        let d = [0u32, 4, 8].map(|o| mul(sub(rf(g, v1 + o), rf(g, v2 + o)), k));
        let x = add(rf(g, r + 0x1c), add(rf(g, q), d[0]));
        let y = add(rf(g, r + 0x20), add(rf(g, q + 4), d[1]));
        let z = add(rf(g, r + 0x24), add(d[2], rf(g, q + 8)));
        g.write::<f32>(v1, x);
        g.write::<f32>(v1 + 4, y);
        g.write::<f32>(v1 + 8, z);
        for o in [0u32, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28] {
            let x = mul(rf(g, m + o), k);
            g.write::<f32>(m + o, x);
        }
        let model = g.read::<u32>(models.wrapping_add(i).wrapping_add(0x1c));
        let draw = Frame {
            at: img.rel32_target(f + 0x22d),
            esp: call_esp,
            ecx: model,
            edx: m,
            ..*fr
        };
        crate::o6_fam_pairs3::part_matrix_handover(g, img, &draw);
        i += 0x20;
        r = r.wrapping_add(0x60);
        g.write::<u32>(b0 + 0x14, r);
        g.write::<u32>(b0 + 0x10, i);
        if i.cast_signed() >= w(0x244).cast_signed() {
            break;
        }
    }
    Ret::eax(i)
}

/// [`models_around_0605`] (the `rel32`s: the sound record, the draw,
/// `__security_check_cookie`).
pub const MODELS_AROUND_0605: Sig = Sig {
    len: 608,
    hash: 0x0abc_cfde_e798_19df,
    calls: &[
        (0x1e, Callee::Sig(&crate::o4_fam4::EFFECT_RECORD_0605_7E40)),
        (
            0x22d,
            Callee::Sig(&crate::o6_fam_pairs3::PART_MATRIX_HANDOVER_0605_7ED0),
        ),
        (0x258, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0609`'s two-part visit (0x10001000, 856 bytes; `ecx` = the
/// cursor `C`, reached by its 12-byte thunk 0x10001370; a /GS frame, the
/// part cursor `L` at `ebp - 0x50`):
/// [`two_part_visit`](crate::o6_fam_pairs5::two_part_visit)'s steps, with
/// the light block of `magic_0609`. Unless the node `N = *C` is marked
/// already (`N + 0x1a` = `C + 5`: `C + 4` cleared), the children are
/// visited, `N` marked and both parts `N + 0x6c`, `N + 0x70` visited
/// through `L`. When bit 0 of `N + 0x68` is set and `N + 0x28` is above
/// `K`: slot 231 `(ebp - 0x44, M, N[0x70] + 0x9c)`, slot 245 `(ebp -
/// 0x4c)`, slot 563 `(*G_C, 0, 0, 0)` with `*G_C` then 8 further on, slot
/// 821 `(0)`, 822 with `N + 0x1c..0x28` times `K2`, 823 `(1 / N + 0x5c, 1 /
/// N + 0x5c)`, 824 `(A + 0x1c, A + 0x20, A + 0x24, 1.0)` (`A = N[0x6c]`).
/// Then the moves of `N + 0x1c..` and `N + 0x5c` as the family's.
///
/// Returns `eax` as the original leaves it: the last call's or the last
/// word copied; when marked only its low byte is known (the /GS cookie's
/// bits above).
#[allow(clippy::too_many_lines)] // the original's branches, one by one
pub fn two_part_visit_0609<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let c = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let node = g.read::<u32>(c);
    let mark = g.read::<u8>(node.wrapping_add(0x1a));
    if mark == g.read::<u8>(c.wrapping_add(5)) {
        g.write::<u8>(c.wrapping_add(4), 0);
        return Ret::eax(u32::from(mark));
    }
    let mut eax = 0;
    let mut child = g.read::<u32>(node.wrapping_add(0x10));
    if child != node {
        loop {
            g.write::<u32>(c, child);
            let cur = g.read::<u32>(c);
            child = g.read::<u32>(child.wrapping_add(8));
            let visit = g.read::<u32>(g.read::<u32>(cur).wrapping_add(0x1c));
            // call width: 1
            g.call(visit, c, &[c]);
            if child == node {
                break;
            }
        }
    }
    let byte5 = g.read::<u8>(c.wrapping_add(5));
    g.write::<u32>(c, node);
    g.write::<u8>(node.wrapping_add(0x1a), byte5);
    g.write::<u8>(c.wrapping_add(4), 1);
    // test byte [esi + 4], 1: just set
    let s = g.read::<u32>(c);
    let l = ebp.wrapping_sub(0x50);
    let mut words = Pushes::default();
    for part in [0x6cu32, 0x70] {
        let byte5 = g.read::<u8>(c.wrapping_add(5));
        let p = g.read::<u32>(s.wrapping_add(part));
        g.write::<u32>(l, p);
        g.write::<u8>(l.wrapping_add(5), byte5);
        let visit = g.read::<u32>(g.read::<u32>(p).wrapping_add(0x1c));
        words.push(l);
        // call width: 12 (Pushes caps the words at 12)
        eax = words.call(g, visit, l).eax;
    }
    let f32_at = |g: &G, o: u32| g.read::<f32>(s.wrapping_add(o));
    let mut one = k(g, 0x92);
    if g.read::<u8>(s.wrapping_add(0x68)) & 1 != 0 && f32_at(g, 0x28) > k(g, 0xab) {
        let b = g.read::<u32>(s.wrapping_add(0x70)).wrapping_add(0x9c);
        let mut st = Pushes::default();
        st.push_all(&[b, w(0xc2), ebp.wrapping_sub(0x44)]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0xc8, 231), 0);
        st.push(ebp.wrapping_sub(0x4c));
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0xd9, 245), 0);
        let cur = g.read::<u32>(w(0xe7));
        let next = cur.wrapping_add(8);
        let speed = f32_at(g, 0x5c);
        // (`(next - cur) >> 4`: 0)
        st.push_all(&[0, 0, next.wrapping_sub(cur) >> 4, cur]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x100, 563), 0);
        let inv = div(k(g, 0x116), speed);
        g.write::<u32>(w(0x126), next);
        st.push(0);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x120, 821), 0);
        // add esp, 0x14: the four words of slot 245 and 231 rewritten
        st.pop(5);
        let k2 = k(g, 0x13d);
        let scaled = |g: &G, o: u32| mul(f32_at(g, o), k2).to_bits();
        st.pop(4);
        st.push_all(&[
            scaled(g, 0x28),
            scaled(g, 0x24),
            scaled(g, 0x20),
            scaled(g, 0x1c),
        ]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x147, 822), 0);
        // add esp, 8: both words left written over with 1 / speed
        st.pop(4);
        st.push_all(&[inv.to_bits(), inv.to_bits()]);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t(g, 0x192, 823), 0);
        let a = g.read::<u32>(s.wrapping_add(0x6c));
        // the two words rewritten, two more below them
        st.pop(2);
        st.push_all(&[
            ONE,
            g.read::<u32>(a.wrapping_add(0x24)),
            g.read::<u32>(a.wrapping_add(0x20)),
            g.read::<u32>(a.wrapping_add(0x1c)),
        ]);
        // call width: 12 (Pushes caps the words at 12)
        eax = st.call(g, t(g, 0x1d5, 824), 0).eax;
        one = k(g, 0x1ea);
    }
    if g.read::<u8>(s.wrapping_add(0x68)) & 2 != 0 {
        let rate = |g: &G| f32_at(g, 0x4c);
        for (x, to) in [(0x1cu32, 0x2cu32), (0x20, 0x30), (0x24, 0x34), (0x28, 0x38)] {
            let v = add(mul(sub(f32_at(g, to), f32_at(g, x)), rate(g)), f32_at(g, x));
            g.write::<f32>(s.wrapping_add(x), v);
        }
        let v = add(
            mul(sub(f32_at(g, 0x60), f32_at(g, 0x5c)), f32_at(g, 0x54)),
            f32_at(g, 0x5c),
        );
        g.write::<f32>(s.wrapping_add(0x5c), v);
        return Ret::eax(eax);
    }
    // comiss, jae: taken when not below (ordered); jb: below or unordered
    let at_end = |x: f32| !(x < one || x.is_nan());
    let copy = |g: &mut G, from: u32, to: u32| {
        let v = g.read::<u32>(s.wrapping_add(from));
        g.write::<u32>(s.wrapping_add(to), v);
        v
    };
    let mut done = at_end(f32_at(g, 0x50));
    if !done {
        for (x, v) in [(0x1cu32, 0x3cu32), (0x20, 0x40), (0x24, 0x44), (0x28, 0x48)] {
            let y = add(mul(f32_at(g, v), f32_at(g, 0x4c)), f32_at(g, x));
            g.write::<f32>(s.wrapping_add(x), y);
        }
        let step = add(f32_at(g, 0x50), f32_at(g, 0x4c));
        g.write::<f32>(s.wrapping_add(0x50), step);
        done = at_end(step);
    }
    if done {
        for (to, from) in [(0x1cu32, 0x2cu32), (0x20, 0x30), (0x24, 0x34), (0x28, 0x38)] {
            eax = copy(g, from, to);
        }
    }
    let fade = f32_at(g, 0x58);
    if !at_end(fade) {
        let y = mul(f32_at(g, 0x64), f32_at(g, 0x54));
        let fade = add(fade, f32_at(g, 0x54));
        let y = add(y, f32_at(g, 0x5c));
        g.write::<f32>(s.wrapping_add(0x58), fade);
        g.write::<f32>(s.wrapping_add(0x5c), y);
        if !at_end(f32_at(g, 0x50)) {
            return Ret::eax(eax);
        }
    }
    eax = copy(g, 0x60, 0x5c);
    Ret::eax(eax)
}

/// [`two_part_visit_0609`] (the `rel32`s: `__security_check_cookie`
/// twice).
pub const TWO_PART_VISIT_0609: Sig = Sig {
    len: 856,
    hash: 0x0d16_9998_5ab2_a8fa,
    calls: &[(0x281, Callee::Any), (0x350, Callee::Any)],
    imports: &[],
};

/// `magic_0563`'s eight sparks (0x10003310, 660 bytes; cdecl, no
/// arguments; a /GS frame: the offset `V` at `ebp - 0x14`, the matrix `M`
/// at `ebp - 0x64`, the turned offset `O` at `ebp - 0x24`, the count at
/// `ebp - 0x6c`): [`sparks_thrown`](crate::s5_misc::sparks_thrown)'s kind.
/// For each of eight, a record from the pool
/// ([`pool_take`](crate::sb_misc::pool_take); none: returns 0): u16 `+0x38
/// = 0xffff`, `+0x3a = 0x200`, `+0x30` the effect's `*(*G_A + 0x14)`,
/// `+0x34 = 1`, the position `*G_S + 0x2b40..` moved by `(0, 0, frnd(80),
/// 1)` turned by a unit matrix rotated by `frnd(pi)` about y (slots 235,
/// 314, 329, 229) in `x` and `z` and by `frnd(40)` in `y`; the speeds
/// `+0x10 = (frnd_p(0.01) + K) O.x`, `+0x14 = -frnd_p(5)`, `+0x18 =
/// (frnd_p(0.01) + K) O.z` (slot 404), the turn `+0x1c = frnd(pi)`,
/// `+0x20..+0x2c = (+0x10 K2, 0.1, +0x18 K2, 1)`, the size `+0x3c =
/// (frnd(0.2) + K3) K4`, `+0x40 = 0.5`, `+0x4c`, `+0x50`, `+0x58` 0x40,
/// `+0x5c`, `+0x60` 0, byte `+0x6b` 0. Each call sees the words the earlier
/// ones of its spark left on the stack. Returns 8.
pub fn sparks_eight_0563<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x14));
    let ebp = fr.esp.wrapping_sub(4);
    let [v, m, out, count] = [0x14u32, 0x64, 0x24, 0x6c].map(|o| ebp.wrapping_sub(o));
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    g.write::<u32>(v + 0xc, ONE);
    g.write::<u32>(count, 0);
    let (pi, hundredth) = (0x4049_0fdb, 0x3c23_d70a);
    let mut n = 0u32;
    while n < 8 {
        let p = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x35));
        if p == 0 {
            return Ret::eax(0);
        }
        let e = g.read::<u32>(g.read::<u32>(w(0x45)).wrapping_add(0x14));
        g.write::<u16>(p + 0x38, 0xffff);
        g.write::<u32>(p + 0x30, e);
        g.write::<u32>(p + 0x34, 1);
        g.write::<u16>(p + 0x3a, 0x200);
        for i in 0..4 {
            let x = g.read::<u32>(s.wrapping_add(0x2b40 + 4 * i));
            g.write::<u32>(p + 4 * i, x);
        }
        let mut st = Pushes::default();
        st.push(m);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, slot_target(g, w(0x8e), 235), 0);
        let rnd = |g: &mut G, st: &mut Pushes, table: u32, slot: u32, x: u32| {
            st.set_top(x);
            // call width: 12 (the most `Pushes::words` holds)
            st0_f32(st.call(g, slot_target(g, w(table), slot), 0).st0)
        };
        let a = rnd(g, &mut st, 0x9b, 314, pi);
        st.set_top(a.to_bits());
        st.push_all(&[m, m]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, slot_target(g, w(0xc1), 329), 0);
        st.pop(2);
        let z = rnd(g, &mut st, 0xce, 314, 0x42a0_0000);
        g.write::<f32>(v + 8, z);
        st.push_all(&[v, m, out]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, slot_target(g, w(0xf4), 229), 0);
        st.pop(3);
        let o = |g: &G, d: u32| g.read::<f32>(out + d);
        let pf = |g: &G, d: u32| g.read::<f32>(p + d);
        g.write::<f32>(p, add(pf(g, 0), o(g, 0)));
        let r = rnd(g, &mut st, 0x10a, 314, 0x4220_0000);
        g.write::<f32>(p + 4, add(pf(g, 4), r));
        g.write::<f32>(p + 8, add(pf(g, 8), o(g, 8)));
        let q = rnd(g, &mut st, 0x132, 404, hundredth);
        g.write::<f32>(p + 0x10, mul(add(q, k(g, 0x165)), o(g, 0)));
        let q = rnd(g, &mut st, 0x16a, 404, 0x40a0_0000);
        let mask = g.read::<u32>(w(0x192));
        g.write::<u32>(p + 0x14, q.to_bits() ^ mask);
        let q = rnd(g, &mut st, 0x19c, 404, hundredth);
        g.write::<f32>(p + 0x18, mul(add(q, k(g, 0x1bb)), o(g, 8)));
        let t = rnd(g, &mut st, 0x1c0, 314, pi);
        g.write::<f32>(p + 0x1c, t);
        g.write::<u32>(p + 0x24, 0x3dcc_cccd);
        g.write::<f32>(p + 0x20, mul(pf(g, 0x10), k(g, 0x1e9)));
        g.write::<u32>(p + 0x2c, ONE);
        g.write::<f32>(p + 0x28, mul(pf(g, 0x18), k(g, 0x207)));
        let r = rnd(g, &mut st, 0x1ee, 314, 0x3e4c_cccd);
        n = g.read::<u32>(count).wrapping_add(1);
        g.write::<u32>(p + 0x40, 0x3f00_0000);
        g.write::<u32>(p + 0x58, 0x40);
        g.write::<f32>(p + 0x3c, mul(add(r, k(g, 0x235)), k(g, 0x24f)));
        g.write::<u32>(p + 0x5c, 0);
        g.write::<u32>(p + 0x4c, 0x40);
        g.write::<u32>(p + 0x50, 0x40);
        g.write::<u8>(p + 0x6b, 0);
        g.write::<u32>(p + 0x60, 0);
        g.write::<u32>(count, n);
    }
    Ret::eax(n)
}

/// [`sparks_eight_0563`] (the `rel32`s: the pool take,
/// `__security_check_cookie`).
pub const SPARKS_EIGHT_0563: Sig = Sig {
    len: 660,
    hash: 0xf10a_54cd_7d6c_4cf0,
    calls: &[
        (0x35, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x28c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0563`'s light start (0x10003000, 778 bytes; cdecl, no
/// arguments; `S = *G_S`): the colour record `C1 = S + 0x3670` filled
/// (colour 0xff80ffff, times 0x10), the fader `C2 = S + 0x37b0` reset
/// (`(0, 100, 0, 3, 100, 100)`, u16 `+0x18` 0), the light `C3 = S + 0x3804`
/// reset (`+0x20` 1/48, `+0x28` 0x30), each with a pool node
/// ([`take`](crate::sa_pool::take), owners `CB1..CB3`, `+0x14` the record;
/// the second and third kind 1) appended to the task list (the first
/// after the old tail, the others after it); then `C3`'s steps `(4, 2,
/// 0)` twice and its flags `| 3`, the list's tail the third node. The
/// objects `S + 0x5f0` (part 5) and `S + 0x4c0` (part 4) started
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), value 0,
/// scales `K1`, `K2`) at `S + 0x2b30`, their `y` less `K3`/`K4`, turned by
/// `S + 0x3ba4`; the first's place kept at `S + 0x2b40`. Then slot 388 `(B[i],
/// 6, 1)` for each of the `*G_N` target bytes (`*G_X` for an index -1).
/// Returns the last slot 388's `eax`, else the turn word.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn light_start_0563<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(w(0x9));
    let (head, tail) = (w(0x74), w(0x89));
    // the colour record
    let c1 = s + 0x3670;
    g.write::<u32>(c1 + 0x81, 0xff80_ffff);
    g.write::<u8>(c1 + 0x80, 0xff);
    g.write::<u16>(c1 + 0x85, 0xffff);
    g.write::<u8>(c1 + 0x87, 0);
    g.write::<u32>(c1 + 0x88, 0);
    g.write::<u16>(c1 + 0x98, 0x48);
    g.write::<u32>(c1 + 0x90, 0x10);
    g.write::<u32>(c1 + 0x8c, 0x10);
    g.write::<u32>(c1 + 0x94, 0);
    let n1 = crate::sa_pool::take(g, img, call(0x6e), w(0xe));
    let listed = g.read::<u32>(head) != 0;
    g.write::<u32>(n1 + 0x14, c1);
    g.write::<u32>(n1 + 4, 0);
    if listed {
        let last = g.read::<u32>(tail);
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    } else {
        g.write::<u32>(n1, 0);
        g.write::<u32>(head, n1);
    }
    g.write::<u32>(n1 + 8, n1);
    // the fader and the light, each after the node before
    let c2 = s + 0x37b0;
    for (o, v) in [
        (0u32, 0u32),
        (4, 100),
        (8, 0),
        (0xc, 3),
        (0x14, 100),
        (0x10, 100),
    ] {
        g.write::<u32>(c2 + o, v);
    }
    g.write::<u16>(c2 + 0x18, 0);
    let c3 = s + 0x3804;
    let append = |g: &mut G, prev: u32, node: u32, rec: u32| {
        let listed = g.read::<u32>(head) != 0;
        g.write::<u8>(node + 0x18, 1);
        g.write::<u32>(node + 0x14, rec);
        g.write::<u32>(node + 4, 0);
        if listed {
            g.write::<u32>(node, prev);
            g.write::<u32>(prev + 4, node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(head, node);
        }
        g.write::<u32>(node + 8, node);
    };
    let n2 = crate::sa_pool::take(g, img, call(0xde), w(0xac));
    append(g, n1, n2, c2);
    for (o, v) in [
        (0u32, 0u32),
        (0x1c, 0),
        (0x20, 0x3caa_aaab),
        (0x24, 0),
        (0x28, 0x30),
    ] {
        g.write::<u32>(c3 + o, v);
    }
    g.write::<u16>(c3 + 0x2c, 0);
    let n3 = crate::sa_pool::take(g, img, call(0x148), w(0x11d));
    append(g, n2, n3, c3);
    for (o, v) in [
        (4u32, 0x4080_0000u32),
        (8, 0x4000_0000),
        (0xc, 0),
        (0x10, 0x4080_0000),
        (0x14, 0x4000_0000),
        (0x18, 0),
    ] {
        g.write::<u32>(c3 + o, v);
    }
    let fl = g.read::<u16>(c3 + 0x2c) | 3;
    g.write::<u16>(c3 + 0x2c, fl);
    g.write::<u32>(w(0x1bf), n3);
    // the two objects
    let mut eax = 0;
    for (o, sc, part, c, ky) in [
        (0x5f0u32, 0x179u32, 5u32, 0x1c4u32, 0x20au32),
        (0x4c0, 0x212, 4, 0x264, 0x2a4),
    ] {
        let obj = s + o;
        let scale = k(g, sc);
        crate::s4_misc::start_object_anim(g, img, call(c), obj, scale, part, 0);
        for i in 0..4 {
            let x = g.read::<u32>(s + 0x2b30 + 4 * i);
            g.write::<u32>(obj + 0xd0 + 4 * i, x);
        }
        let y = sub(g.read::<f32>(obj + 0xd4), k(g, ky));
        eax = g.read::<u32>(s + 0x3ba4);
        g.write::<u32>(obj + 0xc4, eax);
        g.write::<f32>(obj + 0xd4, y);
        if part == 5 {
            for i in 0..4 {
                let x = g.read::<u32>(obj + 0xd0 + 4 * i);
                g.write::<u32>(s + 0x2b40 + 4 * i, x);
            }
        }
    }
    // the target bytes
    let mut i = 0i32;
    while i < g.read::<i32>(w(0x2c3)) {
        let b = if i == -1 {
            g.read::<u32>(w(0x2d6))
        } else {
            u32::from(g.read::<u8>(g.read::<u32>(w(0x2dd)).wrapping_add(i.cast_unsigned())))
        };
        // call width: 3
        eax = g.call(slot_target(g, w(0x2eb), 388), 0, &[b, 6, 1]).eax;
        i += 1;
    }
    Ret::eax(eax)
}

/// [`light_start_0563`] (the `rel32`s: the pool take three times, the
/// object start twice).
pub const LIGHT_START_0563: Sig = Sig {
    len: 778,
    hash: 0xa780_f41e_9758_68c4,
    calls: &[
        (0x6e, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xde, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x148, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1c4, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x264, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};

/// `magic_0563`'s object aimed from `P` to `Q` (0x10002af0, 849 bytes;
/// internal: `ecx` = the object `O`, `edx` = `P`, `xmm3` the offset's
/// length `L`, the stack word `Q`; an 8-aligned /GS frame, `B` = `esp`
/// after the three pushes: the matrix `M` at `B + 0x38`, the vector `V`
/// at `B + 0x28`). With `d = Q - P`: the pitch `atan2(d.y, sqrt(d.z^2 +
/// d.x^2))` (`_libm_sse2_sqrt_precise`, `_CIatan2`, in double) and the yaw
/// `K - atan2(d.z, d.x)` wrapped once; `V = (L, 0, 0, 1)` through `M`
/// (slots 235, 328 with the pitch, 329 with the yaw, 229); `O + 0x130` =
/// `P`, `+0x140` = `P + d / K1 + V`, `+0x150` = `d / K1 K2 + P - V` (`w`
/// 1.0 each), `+0x160` = `Q`, `+0x170` = `P`, `+0x180..` = `(0, 1/30,
/// 0x1e)`; `O` started ([`start_object_anim`](crate::s4_misc::start_object_anim),
/// part 2, value 0, scale `K3`) at `P`. Returns `P.w`'s word.
#[allow(clippy::too_many_arguments)] // the original's registers and stack
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn aim_object_0563<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    obj: u32,
    p: u32,
    len: f32,
) -> u32 {
    use crate::sb_sse::{add, div, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let q = g.read::<u32>(esp + 4);
    // push ebp; and esp, -8; sub esp, 0x74; three pushes
    let b0 = (esp.wrapping_sub(4) & !7).wrapping_sub(0x80);
    let (m, v) = (b0 + 0x38, b0 + 0x28);
    let dy = sub(rf(g, q + 4), rf(g, p + 4));
    let dx = sub(rf(g, q), rf(g, p));
    let dz = sub(rf(g, q + 8), rf(g, p + 8));
    let flat = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
    let root = f64::from(flat).sqrt();
    // `_CIatan2` on x87 (`ST1` = y), each result rounded once
    #[allow(clippy::cast_possible_truncation)]
    let pitch = g.atan2(f64::from(dy), root) as f32;
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let mut yaw = sub(k(g, 0xc3), ang);
    // comiss; jbe: unordered keeps the value
    if yaw > k(g, 0xd2) {
        yaw = sub(yaw, k(g, 0xe2));
    } else if k(g, 0xec) > yaw {
        yaw = add(yaw, k(g, 0xf9));
    }
    let mut st = Pushes::default();
    st.push(m);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x109, 235), 0);
    st.set_top(pitch.to_bits());
    st.push_all(&[m, m]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x12b, 328), 0);
    st.pop(2);
    st.set_top(yaw.to_bits());
    st.push_all(&[m, m]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x14c, 329), 0);
    for (i, x) in (0u32..).zip([len.to_bits(), 0, 0, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    st.push_all(&[v, m, v]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x16e, 229), 0);
    let k1 = k(g, 0x1a4);
    let (px, py, pz) = (rf(g, p), rf(g, p + 4), rf(g, p + 8));
    let qz = rf(g, q + 8);
    let sy = div(sub(rf(g, q + 4), py), k1);
    let sx = div(sub(rf(g, q), px), k1);
    let sz = div(sub(qz, pz), k1);
    for i in 0..4 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(obj + 0x130 + 4 * i, x);
    }
    let (vx, vy, vz) = (rf(g, v), rf(g, v + 4), rf(g, v + 8));
    g.write::<f32>(obj + 0x140, add(add(rf(g, p), sx), vx));
    g.write::<f32>(obj + 0x144, add(add(rf(g, p + 4), sy), vy));
    g.write::<u32>(obj + 0x14c, ONE);
    g.write::<f32>(obj + 0x148, add(add(sz, rf(g, p + 8)), vz));
    let k2 = k(g, 0x243);
    g.write::<f32>(obj + 0x150, sub(add(mul(sx, k2), rf(g, p)), vx));
    g.write::<f32>(obj + 0x154, sub(add(mul(sy, k2), rf(g, p + 4)), vy));
    let z2 = sub(add(mul(sz, k2), rf(g, p + 8)), vz);
    let qw = g.read::<u32>(q + 0xc);
    let scale = k(g, 0x28e);
    g.write::<f32>(obj + 0x158, z2);
    g.write::<u32>(obj + 0x15c, ONE);
    let (qx, qy) = (g.read::<u32>(q), g.read::<u32>(q + 4));
    g.write::<u32>(obj + 0x160, qx);
    g.write::<u32>(obj + 0x164, qy);
    g.write::<f32>(obj + 0x168, qz);
    g.write::<u32>(obj + 0x16c, qw);
    g.write::<u32>(obj + 0x180, 0);
    g.write::<u32>(obj + 0x184, 0x3d08_8889);
    g.write::<u32>(obj + 0x188, 0x1e);
    for i in 0..4 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(obj + 0x170 + 4 * i, x);
    }
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x312), obj, scale, 2, 0);
    let mut eax = 0;
    for i in 0..4 {
        eax = g.read::<u32>(p + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, eax);
    }
    eax
}

/// [`aim_object_0563`] (the `rel32`s: the square root, `_CIatan2` twice,
/// the object start, `__security_check_cookie`).
pub const AIM_OBJECT_0563: Sig = Sig {
    len: 849,
    hash: 0x44f5_15d9_cf13_504f,
    calls: &[
        (0x6f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x82, Callee::Import("_CIatan2")),
        (0xb1, Callee::Import("_CIatan2")),
        (0x312, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x349, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0563`'s two aimed objects (0x10002e50, 409 bytes; cdecl, no
/// arguments; the point `P` at `ebp - 0x24`, the target `Q` at `ebp -
/// 0x14`; `S = *G_S`): `Q` = `S + 0x2b30` (its `y` less `K`); for the bone
/// points 0x11 and 0x37 of `*G_O` (slot 237 into `P`, `w` 1.0) the objects
/// `S + 0x2ba0` and `S + 0x2d30` aimed from `P` to `Q`
/// ([`aim_object_0563`], lengths `K1`, `K2`); then two pool nodes
/// ([`take`](crate::sa_pool::take), owners `CB1`, `CB2`) on the task
/// list, the second (kind 1) with the fader `S + 0x37cc` reset (`(0, 60,
/// 0, 2, 14, 14)`, u16 `+0x18` 0) and made the list's tail. Returns the
/// second node.
pub fn aim_objects_0563<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (p, q) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let s = g.read::<u32>(w(0x14));
    let y = sub(g.read::<f32>(s + 0x2b34), k(g, 0x2d));
    let x = g.read::<u32>(s + 0x2b30);
    g.write::<u32>(q, x);
    let z = g.read::<u32>(s + 0x2b38);
    g.write::<u32>(q + 8, z);
    let qw = g.read::<u32>(s + 0x2b3c);
    g.write::<u32>(q + 0xc, qw);
    g.write::<f32>(q + 4, y);
    // (bone, slot table operand, actor operand, object, length operand,
    // call, the callee's return address below the entry)
    for (bone, ts, ta, o, kl, c, ret) in [
        (
            0x11u32, 0x36u32, 0x60u32, 0x2ba0u32, 0x74u32, 0x8du32, 0x4cu32,
        ),
        (0x37, 0x96, 0xa6, 0x2d30, 0xb0, 0xc9, 0x60),
    ] {
        let actor = g.read::<u32>(w(ta));
        // call width: 4
        g.call(t(g, ts, 237), 0, &[actor, bone, 0, p]);
        g.write::<u32>(p + 0xc, ONE);
        // the words under the call: Q, then slot 237's four
        let esp = fr.esp.wrapping_sub(ret);
        for (i, x) in (1u32..).zip([q, actor, bone, 0, p]) {
            g.write::<u32>(esp + 4 * i, x);
        }
        aim_object_0563(g, img, call(c), esp, s + o, p, k(g, kl));
    }
    let n1 = crate::sa_pool::take(g, img, call(0xd6), w(0xd1));
    let mut head = g.read::<u32>(w(0xdc));
    g.write::<u32>(n1 + 4, 0);
    if head == 0 {
        head = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x104), n1);
    } else {
        let last = g.read::<u32>(w(0xef));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    }
    g.write::<u32>(n1 + 8, n1);
    let c = s + 0x37cc;
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x3c),
        (8, 0),
        (0xc, 2),
        (0x14, 0xe),
        (0x10, 0xe),
    ] {
        g.write::<u32>(c + o, v);
    }
    g.write::<u16>(c + 0x18, 0);
    let n2 = crate::sa_pool::take(g, img, call(0x146), w(0x114));
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, c);
    g.write::<u32>(n2 + 4, 0);
    g.write::<u32>(w(0x159), n2);
    if head == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x188), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    Ret::eax(n2)
}

/// [`aim_objects_0563`] (the `rel32`s: the aims, the pool take twice,
/// `__security_check_cookie` twice).
pub const AIM_OBJECTS_0563: Sig = Sig {
    len: 409,
    hash: 0x9c96_e3ac_af5c_0b50,
    calls: &[
        (0x8d, Callee::Sig(&AIM_OBJECT_0563)),
        (0xc9, Callee::Sig(&AIM_OBJECT_0563)),
        (0xd6, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x146, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x172, Callee::Any),
        (0x191, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0563`'s summon start (0x10003880, 850 bytes; cdecl, no
/// arguments; `S = *G_S`): the turn `S + 0x3ba4 - K` wrapped once into
/// `S + 0x2b04` (`S + 0x2b00`, `+0x2b08` 0), the model matrix `S + 0x2ac0`
/// scaled and turned by it ([`scale_rotate`](crate::sa_misc::scale_rotate));
/// `S + 0x2b10` = `S + 0x2b50`; the object `S + 0x720` started
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), part 6, value
/// 0x11, scale `K1`) at `S + 0x2b50`, its `y` less `K2`, turned by the
/// turn, `+0xe0` 10 and `+0xe8` 30; two lights (`S + 0x3804`: `(2, 1,
/// 0)` twice, the times 1/16 and 0x10, 15; `S + 0x3834`: `(1.5, 0, 1)`
/// twice, 0x3c2237c3 and 0x65, 0x7c; flags `| 3`) and the sprite fan
/// record `S + 0x38d8`, each with a pool node
/// ([`take`](crate::sa_pool::take), the lights' kind 1) on the task list;
/// a fourth node with the word 0x2e000a made the list's tail. Returns the
/// fourth node.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_start_0563<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(w(0x7));
    let mut a = sub(g.read::<f32>(s + 0x3ba4), k(g, 0x18));
    g.write::<u32>(s + 0x2b00, 0);
    // comiss; jbe: unordered keeps the value
    if a > k(g, 0x2e) {
        a = sub(a, k(g, 0x38));
    } else if k(g, 0x42) > a {
        a = add(a, k(g, 0x4f));
    }
    g.write::<f32>(s + 0x2b04, a);
    g.write::<u32>(s + 0x2b08, 0);
    crate::sa_misc::scale_rotate(g, img, call(0x6e), s + 0x2ac0, s + 0x2b00);
    for i in 0..4 {
        let x = g.read::<u32>(s + 0x2b50 + 4 * i);
        g.write::<u32>(s + 0x2b10 + 4 * i, x);
    }
    let obj = s + 0x720;
    crate::s4_misc::start_object_anim(g, img, call(0xb8), obj, k(g, 0x7c), 6, 0x11);
    for i in 0..4 {
        let x = g.read::<u32>(s + 0x2b50 + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, x);
    }
    let y = sub(g.read::<f32>(obj + 0xd4), k(g, 0xfe));
    let turn = g.read::<u32>(s + 0x2b04);
    g.write::<u32>(obj + 0xc4, turn);
    g.write::<u32>(obj + 0xe0, 0x4120_0000);
    g.write::<u32>(obj + 0xe8, 0x41f0_0000);
    g.write::<f32>(obj + 0xd4, y);
    // a pool node on the task list after `prev` (the head's operand, the
    // store of an empty list's head)
    let append = |g: &mut G, prev: u32, node: u32, (head, first): (u32, u32)| {
        if g.read::<u32>(w(head)) == 0 {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(first), node);
        } else {
            g.write::<u32>(node, prev);
            g.write::<u32>(prev + 4, node);
        }
        g.write::<u32>(node + 8, node);
    };
    // the two lights
    let mut prev = 0;
    for (l, (step, time, count, flag), (c, owner, head, last, first)) in [
        (
            0x3804u32,
            (
                (0x4000_0000u32, 0x3f80_0000u32, 0u32),
                0x3d80_0000u32,
                0x10u32,
                0xfu32,
            ),
            (0x15bu32, 0x130u32, 0x161u32, 0x17au32, 0x18du32),
        ),
        (
            0x3834,
            ((0x3fc0_0000, 0, 0x3f80_0000), 0x3c22_37c3, 0x65, 0x7c),
            (0x1fe, 0x1d3, 0x204, 0, 0x22a),
        ),
    ] {
        let r = s + l;
        for (o, v) in [
            (0u32, 0u32),
            (0x1c, 0),
            (0x20, time),
            (0x24, 0),
            (0x28, count),
        ] {
            g.write::<u32>(r + o, v);
        }
        g.write::<u16>(r + 0x2c, 0);
        let node = crate::sa_pool::take(g, img, call(c), w(owner));
        let listed = g.read::<u32>(w(head)) != 0;
        g.write::<u8>(node + 0x18, 1);
        g.write::<u32>(node + 0x14, r);
        g.write::<u32>(node + 4, 0);
        if last != 0 && listed {
            // the first appends after the list's tail
            prev = g.read::<u32>(w(last));
        }
        append(g, prev, node, (head, first));
        let (x, y, z) = step;
        for (o, v) in [(4u32, x), (8, y), (0xc, z), (0x10, x), (0x14, y), (0x18, z)] {
            g.write::<u32>(r + o, v);
        }
        let fl = g.read::<u16>(r + 0x2c) | 3;
        g.write::<u16>(r + 0x2c, fl);
        g.write::<u32>(r + 0x24, flag);
        prev = node;
    }
    // the sprite fan record
    let e = s + 0x38d8;
    for (o, v) in [
        (0xf0u32, 0x4080_8080u32),
        (0x124, 4),
        (0x120, 0),
        (0x110, 0x7c),
        (0xf8, 0),
        (0xfc, 0x3d4c_cccd),
        (0x100, 0),
        (0x108, 0x50),
        (0x104, 0x50),
        (0x10c, 8),
    ] {
        g.write::<u32>(e + o, v);
    }
    let n3 = crate::sa_pool::take(g, img, call(0x2da), w(0x271));
    g.write::<u32>(n3 + 0x14, e);
    g.write::<u32>(n3 + 4, 0);
    append(g, prev, n3, (0x2e0, 0x303));
    let n4 = crate::sa_pool::take(g, img, call(0x310), w(0x308));
    g.write::<u32>(n4 + 0x14, 0x002e_000a);
    g.write::<u32>(n4 + 4, 0);
    g.write::<u32>(w(0x32a), n4);
    append(g, n3, n4, (0x316, 0x346));
    Ret::eax(n4)
}

/// [`summon_start_0563`] (the `rel32`s: the scale and turn, the object
/// start, the pool take four times).
pub const SUMMON_START_0563: Sig = Sig {
    len: 850,
    hash: 0x3d36_de8b_e714_ac42,
    calls: &[
        (0x6e, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        (0xb8, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x15b, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1fe, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x2da, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x310, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0605`'s summon's second stage (0x10008ca0, 752 bytes; cdecl, no
/// arguments; `S = *G_S`): the record `S + 0x1470` = `(0, T, 0)` with `T =
/// S + 0x30b8` less `K` wrapped once; the model matrix `M = S + 0x1430` a
/// unit one (slot 235), its diagonal the actor's scale (slot 277) times
/// `K1`, then times slots 15, 16, 17 of `*G_A`, turned by the record (slots
/// 330, 328, 329); `S + 0x1460` = `S + 0x14e0`; the light `S + 0x2bf4`
/// (`(1.5, 0, 1)` twice, 0x3c2237c3 and 0x65, 0x28, flags `| 3`) and the
/// sprite fan record `S + 0x2de8`, each with a pool node
/// ([`take`](crate::sa_pool::take); the light's kind 1) on the task list,
/// the second made its tail. Returns the second node.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn second_stage_0605<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(w(0x7));
    let turn = g.read::<u32>(s + 0x30b8);
    g.write::<u32>(s + 0x1470, 0);
    g.write::<u32>(s + 0x1474, turn);
    g.write::<u32>(s + 0x1478, 0);
    let mut a = sub(g.read::<f32>(s + 0x1474), k(g, 0x38));
    // comiss; jbe: unordered keeps the value
    if a > k(g, 0x40) {
        a = sub(a, k(g, 0x4a));
    } else if k(g, 0x54) > a {
        a = add(a, k(g, 0x61));
    }
    let m = s + 0x1430;
    g.write::<f32>(s + 0x1474, a);
    g.call(t(g, 0x66, 235), 0, &[m]);
    let actor = g.read::<u32>(w(0x88));
    let x = st0_f32(g.call(t(g, 0x82, 277), 0, &[actor]).st0);
    let x = mul(x, k(g, 0xa0));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + o, x);
    }
    let a3 = g.read::<u32>(w(0xb9));
    let r = st0_f32(g.call(t(g, 0xa5, 15), 0, &[a3]).st0);
    g.write::<f32>(m, mul(r, g.read::<f32>(m)));
    let r = st0_f32(g.call(t(g, 0xd0, 16), 0, &[a3]).st0);
    g.write::<f32>(m + 0x14, mul(g.read::<f32>(m + 0x14), r));
    let r = st0_f32(g.call(t(g, 0xec, 17), 0, &[a3]).st0);
    g.write::<f32>(m + 0x28, mul(r, g.read::<f32>(m + 0x28)));
    for (op, (n, d)) in
        [0xfcu32, 0x135, 0x154]
            .into_iter()
            .zip([(330u32, 0x1478u32), (328, 0x1470), (329, 0x1474)])
    {
        let ang = g.read::<u32>(s + d);
        // call width: 3
        g.call(t(g, op, n), 0, &[m, m, ang]);
    }
    for i in 0..4 {
        let x = g.read::<u32>(s + 0x14e0 + 4 * i);
        g.write::<u32>(s + 0x1460 + 4 * i, x);
    }
    // the light and its node
    let l = s + 0x2bf4;
    for (o, v) in [
        (0u32, 0u32),
        (0x1c, 0),
        (0x20, 0x3c22_37c3),
        (0x24, 0),
        (0x28, 0x65),
    ] {
        g.write::<u32>(l + o, v);
    }
    g.write::<u16>(l + 0x2c, 0);
    let n1 = crate::sa_pool::take(g, img, call(0x1d1), w(0x1c8));
    let listed = g.read::<u32>(w(0x1d7)) != 0;
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 0x14, l);
    g.write::<u32>(n1 + 4, 0);
    if listed {
        let last = g.read::<u32>(w(0x1f0));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    } else {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x203), n1);
    }
    g.write::<u32>(n1 + 8, n1);
    for (o, v) in [
        (4u32, 0x3fc0_0000u32),
        (8, 0),
        (0xc, ONE),
        (0x10, 0x3fc0_0000),
        (0x14, 0),
        (0x18, ONE),
    ] {
        g.write::<u32>(l + o, v);
    }
    let fl = g.read::<u16>(l + 0x2c) | 3;
    g.write::<u16>(l + 0x2c, fl);
    g.write::<u32>(l + 0x24, 0x28);
    // the sprite fan record and its node
    let e = s + 0x2de8;
    for (o, v) in [
        (0xf0u32, 0x4080_8080u32),
        (0x124, 4),
        (0x120, 0),
        (0x110, 0x28),
        (0xf8, 0),
        (0xfc, 0x3d4c_cccd),
        (0x100, 0),
        (0x108, 0x50),
        (0x104, 0x50),
        (0x10c, 8),
    ] {
        g.write::<u32>(e + o, v);
    }
    let n2 = crate::sa_pool::take(g, img, call(0x2b0), w(0x247));
    let listed = g.read::<u32>(w(0x2b6)) != 0;
    g.write::<u32>(n2 + 0x14, e);
    g.write::<u32>(n2 + 4, 0);
    g.write::<u32>(w(0x2c6), n2);
    if listed {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    } else {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x2e4), n2);
    }
    g.write::<u32>(n2 + 8, n2);
    Ret::eax(n2)
}

/// [`second_stage_0605`] (the `rel32`s: the pool take twice).
pub const SECOND_STAGE_0605: Sig = Sig {
    len: 752,
    hash: 0x5e40_10f7_29c7_b228,
    calls: &[
        (0x1d1, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x2b0, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0577`'s burst object set up (0x1000a870, 582 bytes; internal:
/// `ecx` = the object `O`, `edx` a word for its sprite node, the stack word
/// `A`; an `ebp` frame with a particle set-up record at `ebp - 0x1c`): `O`
/// a node (vtable, callback, links to itself) appended to the static list
/// `L` (its flag bit 0 cleared); the static node `O + 0x20` and the sprite
/// node `S = O + 0x78` children of the parent `P` (vtables, pass 0xff,
/// `P`'s count up); `O + 0x3c..` = `(1, 1, 1, 10.5)`; `S + 0x13c` = `A`,
/// `+0x140` = `O + 0x20`, `+0x144` the DLL's block, `+0x128` = `edx`,
/// byte `+0x148` 0; the particle node `O + 0x310`
/// ([`particle_node`](crate::sb_nodes::particle_node): list `L2`, part
/// `S`, the program table's owner, vector block, index 5); the flash off
/// ([`flash_off`](crate::m0577::flash_off)); the camera globals: three
/// spans `G_a - G_b`, the sprite node, the flags word with bit 1 cleared
/// and bit 0 set and the store runs. Returns the flags word.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn burst_setup_0577<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    obj: u32,
    edx: u32,
) -> u32 {
    use crate::r25::abs_stores;
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, o: u32| g.read::<f32>(w(o));
    let arg = g.read::<u32>(esp + 4);
    let ebp = esp.wrapping_sub(4);
    // the object on the static list
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(0x12));
    g.write::<u32>(obj + 0x10, w(0x19));
    g.write::<u32>(obj + 0x14, obj);
    let last = g.read::<u32>(w(0x21));
    g.write::<u32>(obj + 8, last);
    g.write::<u32>(obj + 4, w(0x2b));
    let last = g.read::<u32>(w(0x30));
    g.write::<u32>(last + 4, obj);
    let fl = g.read::<u32>(w(0x3c)) & !1;
    g.write::<u32>(w(0x3c), fl);
    g.write::<u32>(w(0x43), obj);
    // two children of the parent `P`: (the node, its operands: vtable,
    // the parent, the last child, the parent again, the first child's
    // store, the count, the last child's store)
    let child = |g: &mut G, c: u32, [p1, p2, l1, l2, cmp, first, count, lw]: [u32; 8]| {
        g.write::<u16>(c + 0x1a, 0xff);
        g.write::<u32>(c + 0x10, c);
        g.write::<u32>(c + 0xc, c);
        g.write::<u32>(c + 0x14, w(p1));
        g.write::<u32>(c + 4, w(p2));
        let l = g.read::<u32>(w(l1));
        g.write::<u32>(c + 8, l);
        let l = g.read::<u32>(w(l2));
        if l == w(cmp) {
            g.write::<u32>(w(first), c);
        } else {
            g.write::<u32>(l + 4, c);
        }
        let n = g.read::<u16>(w(count)).wrapping_add(1);
        g.write::<u16>(w(count), n);
        g.write::<u32>(w(lw), c);
    };
    let c = obj + 0x20;
    child(g, c, [0x56, 0x5d, 0x62, 0x6a, 0x6f, 0x77, 0x83, 0x89]);
    g.write::<u32>(c, w(0x8f));
    for (o, v) in [
        (0x3cu32, ONE),
        (0x40, ONE),
        (0x44, ONE),
        (0x48, 0x4128_0000),
    ] {
        g.write::<u32>(obj + o, v);
    }
    let s = obj + 0x78;
    g.write::<u32>(s, w(0xb4));
    child(g, s, [0xc7, 0xce, 0xd3, 0xdb, 0xe0, 0xe8, 0xf4, 0xfd]);
    g.write::<u32>(s + 0x140, c);
    g.write::<u32>(s + 0x13c, arg);
    let owner = g.read::<u32>(w(0x114));
    g.write::<u32>(s + 0x144, w(0x11e));
    g.write::<u32>(s + 0x128, edx);
    g.write::<u8>(s + 0x148, 0);
    // the particle node's set-up record
    let rec = ebp.wrapping_sub(0x1c);
    let node = obj + 0x310;
    g.write::<u32>(rec, node);
    g.write::<u32>(node, w(0x134));
    for (i, v) in (1u32..).zip([w(0x13e), s, owner, w(0x14b), 5]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x157), rec);
    crate::m0577::flash_off(g, img, img.rel32_target(f + 0x15c));
    // the camera globals
    let x = sub(rf(g, 0x164), rf(g, 0x16c));
    let one = sub(rf(g, 0x174), 0.0);
    let flags = (g.read::<u32>(w(0x179)) & !2) | 1;
    g.write::<f32>(w(0x188), x);
    let y = sub(rf(g, 0x190), rf(g, 0x198));
    g.write::<u32>(w(0x1a4), s);
    abs_stores(g, img, f, 0x1a8, 0x1b2);
    g.write::<f32>(w(0x1b6), y);
    let z = sub(rf(g, 0x1be), rf(g, 0x1c6));
    abs_stores(g, img, f, 0x1ca, 0x206);
    g.write::<f32>(w(0x20a), z);
    g.write::<f32>(w(0x212), one);
    abs_stores(g, img, f, 0x216, 0x234);
    g.write::<u32>(w(0x235), flags);
    g.write::<u32>(obj + 0x1c, 0);
    flags
}

/// [`burst_setup_0577`] (the `rel32`s: the particle node, the flash off).
pub const BURST_SETUP_0577: Sig = Sig {
    len: 582,
    hash: 0xf1a7_1c4e_422a_ab1c,
    calls: &[
        (0x157, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x15c, Callee::Sig(&crate::m0577::FLASH_OFF)),
    ],
    imports: &[],
};

/// `magic_0577`'s start-up (0x1000d8f0, 704 bytes; cdecl `(actor)`; an
/// 8-aligned frame, `B` = `esp` after `push esi`, the model node's set-up
/// record at `B + 8`): the actor's targets
/// ([`targets_init_c`](crate::s4_summon::targets_init_c)); the statics'
/// store run (the static list, the parent node and its children); the
/// model node `N` (vtable set, under the parent, the object `*G_O`;
/// [`model_node_init`](crate::sa_summon::model_node_init)); the DLL's
/// static node under the parent ([`static_child`](crate::s4_small::static_child));
/// the node with its blocks ([`link_node_blocks`](crate::s4_small::link_node_blocks));
/// the camera store run; slot 718 `(P0)`, `(P1)`; slot 719 `(P0, 8, 0)`
/// into `G_H`; slot 711 `(h, BUF, 0)` with the game lock `**G_LOCK` 1
/// around it; slot 710 `(OUT, *G_H, DST, 0x100000)`; slots 222 `(P1, 0,
/// 10, 2)`, 221 `(P1, 0, 1000, 2)`, 27 `(0x301e, 1)`, 4 `(0x301e)`; two
/// static nodes linked ([`static_node_link_b`](crate::s4_small::static_node_link_b),
/// [`static_node_link_c`](crate::s4_small::static_node_link_c)). Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0577<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let actor = fr.arg(g, 0);
    // push ebp; and esp, -8; sub esp, 0x14; push esi
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    let sub = |esp: u32, at: u32| Frame {
        at: call(at),
        esp,
        ..*fr
    };
    crate::s4_summon::targets_init_c(g, img, call(0xe), b0.wrapping_sub(4), actor);
    let object = g.read::<u32>(w(0x13));
    abs_stores(g, img, f, 0x1b, 0x11c);
    // the model node's record: (node, parent, object, flags byte)
    let rec = b0 + 8;
    g.write::<u32>(rec, w(0x120));
    g.write::<u32>(rec + 4, w(0x128));
    g.write::<u32>(rec + 8, object);
    g.write::<u8>(rec + 0xc, 0);
    abs_stores(g, img, f, 0x135, 0x13f);
    let eax = crate::sa_summon::model_node_init(g, img, call(0x140), rec);
    let sc = Frame {
        edx: w(0x145),
        eax,
        ..sub(b0.wrapping_sub(4), 0x14a)
    };
    crate::s4_small::static_child(g, img, &sc);
    // sub esp, 8
    let lb = Frame {
        ecx: w(0x152),
        ..sub(b0.wrapping_sub(0xc), 0x157)
    };
    crate::s4_small::link_node_blocks(g, img, &lb);
    let s718 = t(g, 0x15c, 718);
    abs_stores(g, img, f, 0x160, 0x1c4);
    // call width: 1 (the two words of `sub esp, 8` under it)
    g.call(s718, 0, &[w(0x1cb)]);
    g.call(t(g, 0x1d2, 718), 0, &[w(0x1e0)]);
    // call width: 3
    let h = g.call(t(g, 0x1e7, 719), 0, &[w(0x1f9), 8, 0]).eax;
    let lock = g.read::<u32>(w(0x201));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x20d), h);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0x218, 711), 0, &[h, w(0x212), 0]);
    let lock = g.read::<u32>(w(0x22b));
    g.write::<u32>(lock, saved);
    let hh = g.read::<u32>(w(0x245));
    let mut st = vec![w(0x250), hh, w(0x23f), 0x10_0000];
    // call width: 4
    g.call(t(g, 0x23a, 710), 0, &st);
    for (tab, slot, p, n) in [
        (0x257u32, 222u32, 0x268u32, 10u32),
        (0x26f, 221, 0x283, 1000),
    ] {
        st.splice(0..0, [w(p), 0, n, 2]);
        // call width: 12 (the earlier calls' words stay on the stack)
        g.call(t(g, tab, slot), 0, &st);
    }
    st.splice(0..0, [0x301e, 1]);
    // call width: 12 (the earlier calls' words stay on the stack)
    g.call(t(g, 0x28a, 27), 0, &st);
    st.insert(0, 0x301e);
    // call width: 12 (the earlier calls' words stay on the stack)
    g.call(t(g, 0x29b, 4), 0, &st);
    crate::s4_small::static_node_link_b(g, img, &sub(b0.wrapping_sub(8), 0x2ad));
    crate::s4_small::static_node_link_c(g, img, &sub(b0.wrapping_sub(8), 0x2b2));
    Ret::eax(0)
}

/// [`start_up_0577`] (the `rel32`s: the targets, the model node, the
/// static child, the node with its blocks, the two static links).
pub const START_UP_0577: Sig = Sig {
    len: 704,
    hash: 0xbb70_3af4_353a_ec38,
    calls: &[
        (0xe, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C)),
        (0x140, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x14a, Callee::Sig(&crate::s4_small::STATIC_CHILD)),
        (0x157, Callee::Sig(&crate::s4_small::LINK_NODE_BLOCKS)),
        (0x2ad, Callee::Sig(&crate::s4_small::STATIC_NODE_LINK_B)),
        (0x2b2, Callee::Sig(&crate::s4_small::STATIC_NODE_LINK_C)),
    ],
    imports: &[],
};

/// A node of `magic_0512`'s object put as a child of the parent `P`: the
/// pass halfword 0xff, its own child list, `P` as parent and list (the
/// operands `p1`, `p2`), after `P`'s last child (read through `l1`, `l2`;
/// `P` itself (`cmp`) makes it the first (`first`)), the count `count` up,
/// the last child's store `lw`.
fn child_of_0512<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    c: u32,
    [p1, p2, l1, l2, cmp, first, count, lw]: [u32; 8],
) {
    let w = |o: u32| img.word(f + o);
    g.write::<u16>(c + 0x1a, 0xff);
    g.write::<u32>(c + 0x10, c);
    g.write::<u32>(c + 0xc, c);
    g.write::<u32>(c + 0x14, w(p1));
    g.write::<u32>(c + 4, w(p2));
    let l = g.read::<u32>(w(l1));
    g.write::<u32>(c + 8, l);
    let l = g.read::<u32>(w(l2));
    if l == w(cmp) {
        g.write::<u32>(w(first), c);
    } else {
        g.write::<u32>(l + 4, c);
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
    g.write::<u32>(w(lw), c);
}

/// `magic_0512`'s "between" object set up (0x10008cc0, 768 bytes;
/// internal: `ecx` = the object `O`, `edx` = the actor's object `X`, the
/// stack words `(A, b, c)`; a /GS frame: the vector `V` at `ebp - 0x18`,
/// the insert group at `ebp - 0x34`): `O` a node on the static list `L`
/// (vtable, callback; the list's flag bit 0 cleared); the children `O +
/// 0x334`, `O + 0x2dc`, `O + 0x308` of the parent `P`
/// ([`child_of_0512`]); `O + 0x350` = slot 229 `(X + 0x9c, ((1 - 2b) K,
/// 0, -1500, 1))`; `O + 0x2fc` less `K1` then less `c K2`; slot 325 `(O +
/// 0x2f8, O + 0x350)`; `O + 0x324..` = `((K3 - b K4) K5, 0.4, -0.5, 1)`;
/// two model nodes inserted ([`insert`](crate::m0512::insert): `O +
/// 0x2c` with `O + 0x2dc`, kind 0; `O + 0x184` with `O + 0x334`, kind 1;
/// each with `O + 0x308` and `A`, then given their vtables); `O + 0x1c` =
/// `c`, `+0x24` = `10 - 10 b`, `+0x20` = `21 - 11 b`, `+0x28` = `A.z -
/// X.z` (`+0x20`, `+0xd0`), `+0x18` 0. Returns `21 - 11 b`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn between_setup_0512<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    obj: u32,
    x: u32,
) -> u32 {
    use crate::sb_sse::{cvt, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let a = g.read::<u32>(esp + 4);
    let b = g.read::<u32>(esp + 8);
    let c = g.read::<u32>(esp + 0xc);
    let ebp = esp.wrapping_sub(4);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // the object on the static list
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(0x22));
    g.write::<u32>(obj + 0x10, w(0x29));
    g.write::<u32>(obj + 0x14, obj);
    let last = g.read::<u32>(w(0x31));
    g.write::<u32>(obj + 8, last);
    g.write::<u32>(obj + 4, w(0x3b));
    let last = g.read::<u32>(w(0x40));
    g.write::<u32>(last + 4, obj);
    let fl = g.read::<u32>(w(0x4a)) & !1;
    g.write::<u32>(w(0x4a), fl);
    g.write::<u32>(w(0x57), obj);
    // the first child and the turned point
    let c1 = obj + 0x334;
    child_of_0512(
        g,
        img,
        f,
        c1,
        [0x6a, 0x71, 0x76, 0x7e, 0x86, 0x8e, 0x9d, 0xcb],
    );
    g.write::<u32>(c1, w(0xd1));
    let s = cvt(1u32.wrapping_sub(b.wrapping_add(b)));
    let v = ebp.wrapping_sub(0x18);
    g.write::<f32>(v, mul(s, k(g, 0xb8)));
    g.write::<u32>(v + 4, 0);
    g.write::<u32>(v + 8, 0xc4bb_8000);
    g.write::<u32>(v + 0xc, ONE);
    let p = obj + 0x350;
    // call width: 3
    g.call(t(g, 0xc5, 229), 0, &[p, x + 0x9c, v]);
    g.write::<f32>(obj + 0x2fc, sub(rf(g, obj + 0x2fc), k(g, 0x10a)));
    // the second child, slot 325
    let c2 = obj + 0x2dc;
    child_of_0512(
        g,
        img,
        f,
        c2,
        [0x12e, 0x135, 0x13a, 0x142, 0x147, 0x14f, 0x15b, 0x16e],
    );
    g.write::<u32>(c2, w(0x174));
    // call width: 2
    g.call(t(g, 0x168, 325), 0, &[obj + 0x2f8, p]);
    let d = mul(cvt(c), k(g, 0x19a));
    g.write::<f32>(obj + 0x2fc, sub(rf(g, obj + 0x2fc), d));
    // the third child and its record
    let c3 = obj + 0x308;
    child_of_0512(
        g,
        img,
        f,
        c3,
        [0x1bc, 0x1c3, 0x1c8, 0x1d0, 0x1d5, 0x1dd, 0x1f6, 0x20d],
    );
    g.write::<u32>(c3, w(0x21d));
    let y = mul(sub(k(g, 0x1ef), mul(cvt(b), k(g, 0x204))), k(g, 0x22b));
    g.write::<u32>(obj + 0x328, 0x3ecc_cccd);
    g.write::<f32>(obj + 0x324, y);
    g.write::<u32>(obj + 0x32c, 0xbf00_0000);
    g.write::<u32>(obj + 0x330, ONE);
    // the two model nodes
    let grp = ebp.wrapping_sub(0x34);
    for (i, v) in (0u32..).zip([obj + 0x2c, w(0x25b), c2, c3, a, 0]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    crate::m0512::insert(g, img, img.rel32_target(f + 0x26d), grp);
    let n = g.read::<u32>(grp);
    g.write::<u32>(n, w(0x27c));
    for (i, v) in (0u32..).zip([obj + 0x184, w(0x28f), c1, c3, a, 1]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    crate::m0512::insert(g, img, img.rel32_target(f + 0x2a1), grp);
    let n = g.read::<u32>(grp);
    g.write::<u32>(n, w(0x2ad));
    let dz = sub(rf(g, a + 0x20), rf(g, x + 0xd0));
    g.write::<u32>(obj + 0x1c, c);
    g.write::<u32>(obj + 0x24, 10u32.wrapping_sub(b.wrapping_mul(10)));
    g.write::<f32>(obj + 0x28, dz);
    let eax = 0x15u32.wrapping_sub(b.wrapping_mul(11));
    g.write::<u32>(obj + 0x20, eax);
    g.write::<u32>(obj + 0x18, 0);
    eax
}

/// [`between_setup_0512`] (the `rel32`s: the insert twice,
/// `__security_check_cookie`).
pub const BETWEEN_SETUP_0512: Sig = Sig {
    len: 768,
    hash: 0x4b3a_9b94_8114_93a1,
    calls: &[
        (0x26d, Callee::Sig(&crate::m0512::INSERT)),
        (0x2a1, Callee::Sig(&crate::m0512::INSERT)),
        (0x2f8, Callee::Any),
    ],
    imports: &[],
};

/// The summon's facing and its four places (`magic_0605` 0x10006f60 and
/// `magic_0307` 0x100015e0, 1843 bytes, family P007; internal, no
/// arguments; an 8-aligned /GS frame, `B` = `esp` after the three pushes:
/// the caster's point `C` at `B + 0x48`, the map centre `Q` at `B + 0x18`,
/// the turn at `B + 0xc`, the offset `V` at `B + 0x28`, its turned copy `O`
/// at `B + 0x38`, the matrices `M2` at `B + 0x58` and `M3` at `B + 0x98`;
/// `S = *G_S`, every record displacement read from the code):
/// - `C` = slot 262 `(*G_A)`'s point, `Q` = slot 292's (the map centre);
///   the turn `a = K - atan2(Q.z - C.z, Q.x - C.x)` (`_CIatan2`) wrapped
///   once;
/// - the model matrix `M = S + 0x1430` (slot 210, or 217/255 for a null
///   `M`, which no `S` gives; 211; 235), its diagonal slot 277 times `K1`,
///   times slots 15, 16, 17, turned by `(0, 0, a)` (330, 328, 329); the
///   record `S + 0x1470` = `(0, a, 0, junk)`;
/// - four places from `Q` moved by `(0, 0, z)` turned by `a` (slots 235,
///   328 with 0, 329, 229, 248; `w` 1.0): `S + 0x1480` (`z` 47.6, copied to
///   `S + 0x1460`), `S + 0x14a0` (-30, its `y` the ground of slot 12, kept
///   at `S + 0x1490` and, `y` less `K2`, at `S + 0x14b0`), `S + 0x14c0`
///   (10, `y` less `K3`), `S + 0x14e0` (-20); `S + 0x30b8` = `a`, `S +
///   0x30b0`, `+0x30b4`, `+0x30bc`, `+0x30c0` 0, `*G_F` 1;
/// - the turned box ([`turn_box`](crate::o6_fam_pairs5::turn_box)) and the
///   mesh centres ([`mesh_centres`](crate::o6_fam_pairs5::mesh_centres)).
///
/// Returns the mesh centres' `eax`.
///
/// Junk read: `[B + 0x44]` (`ebp`-free frame word, never written) goes to
/// the record's fourth word; 0 (brief 2026-10-24).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn facing_places_0605<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // push ebp; and esp, -8; sub esp, 0xd4; three pushes
    let b0 = (esp.wrapping_sub(4) & !7).wrapping_sub(0xe0);
    let (c, q, v, o) = (b0 + 0x48, b0 + 0x18, b0 + 0x28, b0 + 0x38);
    let (m2, m3) = (b0 + 0x58, b0 + 0x98);
    let s = g.read::<u32>(w(0x1d));
    let actor = g.read::<u32>(w(0x39));
    // call width: 4
    g.call(t(g, 0x33, 262), 0, &[actor, c, c + 4, c + 8]);
    g.write::<u32>(c + 0xc, ONE);
    g.call(t(g, 0x4b, 292), 0, &[q]);
    let dz = sub(rf(g, q + 8), rf(g, c + 8));
    g.write::<u32>(q + 0xc, ONE);
    let dx = sub(rf(g, q), rf(g, c));
    // `_CIatan2(dz, dx)` on x87, the result rounded once (`cvtpd2ps`)
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let mut a = sub(k(g, 0xaf), ang);
    // comiss; jbe: unordered keeps the value
    if a > k(g, 0xbe) {
        a = sub(a, k(g, 0xce));
    } else if k(g, 0xd8) > a {
        a = add(a, k(g, 0xe5));
    }
    g.write::<f32>(b0 + 0xc, a);
    // the model matrix
    let actor = g.read::<u32>(w(0xf0));
    let m = s.wrapping_add(w(0xf6));
    if m == 0 {
        // `S + d` null: no caller's `S` gets here
        let idx = g.read::<u32>(w(0x121));
        if g.read::<u32>(w(0x118)) == 0 && idx.cast_signed() < 0x20 {
            let buf = b0 + 0x38;
            // call width: 2
            g.call(t(g, 0x130, 217), 0, &[idx, buf]);
            // call width: 2
            g.call(t(g, 0x143, 255), 0, &[idx, buf]);
        }
    } else {
        // call width: 2
        g.call(t(g, 0x105, 210), 0, &[actor, m]);
    }
    // call width: 2
    g.call(t(g, 0x15a, 211), 0, &[actor, m]);
    let r = s.wrapping_add(w(0x188));
    g.write::<f32>(r + 4, a);
    g.write::<u32>(r, 0);
    g.write::<u32>(r + 8, 0);
    // junk read: [B + 0x44] never written; 0 (brief 2026-10-24)
    g.write::<u32>(r + 0xc, 0);
    g.call(t(g, 0x16d, 235), 0, &[m]);
    let x = st0_f32(g.call(t(g, 0x1a5, 277), 0, &[g.read::<u32>(w(0x1ab))]).st0);
    let x = mul(x, k(g, 0x1c5));
    for off in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + off, x);
    }
    let a3 = g.read::<u32>(w(0x1de));
    let r15 = st0_f32(g.call(t(g, 0x1ca, 15), 0, &[a3]).st0);
    g.write::<f32>(m, mul(rf(g, m), r15));
    let r16 = st0_f32(g.call(t(g, 0x1f7, 16), 0, &[a3]).st0);
    g.write::<f32>(m + 0x14, mul(r16, rf(g, m + 0x14)));
    let r17 = st0_f32(g.call(t(g, 0x206, 17), 0, &[a3]).st0);
    g.write::<f32>(m + 0x28, mul(rf(g, m + 0x28), r17));
    for (op, n, ang) in [
        (0x234u32, 330u32, 0u32),
        (0x252, 328, 0),
        (0x271, 329, a.to_bits()),
    ] {
        // call width: 3
        g.call(t(g, op, n), 0, &[m, m, ang]);
    }
    // a place: `Q` moved by `V` turned by `a` through `M` (the slots'
    // table operands: 235, 328, 329, 229, 248)
    let place = |g: &mut G, p: u32, mm: u32, ops: [u32; 5]| {
        g.call(t(g, ops[0], 235), 0, &[mm]);
        // call width: 3
        g.call(t(g, ops[1], 328), 0, &[mm, mm, 0]);
        // call width: 3
        g.call(t(g, ops[2], 329), 0, &[mm, mm, a.to_bits()]);
        g.write::<u32>(v + 0xc, ONE);
        // call width: 3
        g.call(t(g, ops[3], 229), 0, &[o, mm, v]);
        // call width: 3
        g.call(t(g, ops[4], 248), 0, &[p, p, o]);
        g.write::<u32>(p + 0xc, ONE);
    };
    let copy = |g: &mut G, from: u32, to: u32| {
        for i in 0..4 {
            let x = g.read::<u32>(from + 4 * i);
            g.write::<u32>(to + 4 * i, x);
        }
    };
    let set_v = |g: &mut G, z: u32| {
        for (i, x) in (0u32..).zip([0, 0, z, ONE]) {
            g.write::<u32>(v + 4 * i, x);
        }
    };
    // the first place
    let p2 = s.wrapping_add(w(0x28f));
    copy(g, q, p2);
    set_v(g, 0x423e_955c);
    place(g, p2, m2, [0x2d1, 0x304, 0x325, 0x341, 0x35b]);
    copy(g, p2, s.wrapping_add(w(0x37a)));
    g.write::<f32>(s.wrapping_add(w(0x3a6)), a);
    // the second, on the ground
    let p3 = s.wrapping_add(w(0x3b2));
    copy(g, q, p3);
    set_v(g, 0xc1f0_0000);
    place(g, p3, m3, [0x3d7, 0x42d, 0x451, 0x470, 0x48a]);
    let y = st0_f32(g.call(t(g, 0x499, 12), 0, &[g.read::<u32>(w(0x4a6))]).st0);
    g.write::<f32>(p3 + 4, y);
    copy(g, p3, s.wrapping_add(w(0x4b9)));
    let p3b = s.wrapping_add(w(0x4f5));
    copy(g, p3, p3b);
    g.write::<f32>(p3b + 4, sub(rf(g, p3 + 4), k(g, 0x4ef)));
    // the third
    let p4 = s.wrapping_add(w(0x521));
    copy(g, q, p4);
    g.write::<u32>(v + 8, 0x4120_0000);
    place(g, p4, m3, [0x546, 0x584, 0x5a8, 0x5cf, 0x5e1]);
    g.write::<f32>(p4 + 4, sub(rf(g, p4 + 4), k(g, 0x602)));
    // the fourth
    let p5 = s.wrapping_add(w(0x608));
    copy(g, q, p5);
    g.write::<u32>(v + 8, 0xc1a0_0000);
    place(g, p5, m2, [0x612, 0x673, 0x694, 0x6b0, 0x6ca]);
    for off in [0x6e1u32, 0x6eb, 0x6f5, 0x6ff] {
        g.write::<u32>(s.wrapping_add(w(off)), 0);
    }
    g.write::<u32>(w(0x70c), 1);
    let call_esp = b0.wrapping_sub(4);
    let tb = Frame {
        at: img.rel32_target(f + 0x715),
        esp: call_esp,
        ..Frame::default()
    };
    crate::o6_fam_pairs5::turn_box(g, img, &tb);
    let mc = Frame {
        at: img.rel32_target(f + 0x71a),
        esp: call_esp,
        ..Frame::default()
    };
    crate::o6_fam_pairs5::mesh_centres(g, img, &mc).eax
}

/// [`facing_places_0605`] in `magic_0605`.
pub const FACING_PLACES_0605: Sig = Sig {
    len: 1843,
    hash: 0x2095_2c4f_c8f4_2cda,
    calls: &[
        (0x9d, Callee::Import("_CIatan2")),
        (
            0x715,
            Callee::Sig(&crate::o6_fam_pairs5::TURN_BOX_0605_76A0),
        ),
        (
            0x71a,
            Callee::Sig(&crate::o6_fam_pairs5::MESH_CENTRES_0605_7990),
        ),
        (0x72b, Callee::Any),
    ],
    imports: &[],
};

/// [`facing_places_0605`] in `magic_0307`.
pub const FACING_PLACES_0307: Sig = Sig {
    len: 1843,
    hash: 0x197b_af51_7d53_5599,
    calls: &[
        (0x9d, Callee::Import("_CIatan2")),
        (
            0x715,
            Callee::Sig(&crate::o6_fam_pairs5::TURN_BOX_0307_1D20),
        ),
        (
            0x71a,
            Callee::Sig(&crate::o6_fam_pairs5::MESH_CENTRES_0307_2010),
        ),
        (0x72b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0605`'s start-up (0x1000afb0, 318 bytes; no arguments, run by
/// its load step [`load_0605`]; no frame): slot 718 `(P0)`; slot 719
/// `(P, 8, 0)` into `G_H` (the statics' store run between); the part set
/// up ([`part_setup`](crate::sb_misc::part_setup) `(BUF)`, slot 718's word
/// under it), the summon's facing and places ([`facing_places_0605`]),
/// the turned start ([`start_turned`](crate::o4_effect::start_turned));
/// slot 271 `(4)`; the script block's words; two task nodes (kinds 1 and
/// 2, [`take`](crate::sa_pool::take)) on the task list, the records block
/// after the first ([`records_init`](crate::sa_misc::records_init)).
/// Returns 0.
pub fn start_up_0605<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r13_start::append_kind;
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    g.call(t(g, 0x1, 718), 0, &[w(0x6)]);
    let s719 = t(g, 0x13, 719);
    abs_stores(g, img, f, 0x26, 0x58);
    // call width: 3
    let h = g.call(s719, 0, &[w(0x22), 8, 0]).eax;
    g.write::<u32>(w(0x5e), h);
    // slot 718's word stays; push BUF: the return address at `E - 0xc`
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 8, w(0x6));
    g.write::<u32>(esp + 4, w(0x63));
    crate::sb_misc::part_setup(g, img, call(0x68), esp);
    facing_places_0605(g, img, call(0x6d), esp);
    let st = Frame {
        at: call(0x72),
        esp,
        ..Frame::default()
    };
    crate::o4_effect::start_turned(g, img, &st);
    // call width: 3 (BUF and slot 718's word under it)
    g.call(t(g, 0x77, 271), 0, &[4, w(0x63), w(0x6)]);
    abs_stores(g, img, f, 0x8d, 0xb1);
    let n1 = crate::sa_pool::take(g, img, call(0xb2), w(0x89));
    append_kind(g, img, f, n1, 1, [0xb8, 0xcc, 0xeb, 0xde, 0xeb]);
    crate::sa_misc::records_init(g, img, call(0xf0), w(0xe3));
    let n2 = crate::sa_pool::take(g, img, call(0xfa), w(0xf5));
    let listed = g.read::<u32>(w(0x100)) != 0;
    g.write::<u8>(n2 + 0x18, 2);
    g.write::<u32>(n2 + 4, 0);
    if listed {
        let last = g.read::<u32>(w(0x114));
        g.write::<u32>(n2, last);
        g.write::<u32>(last + 4, n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x121), n2);
    } else {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x12f), n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x137), n2);
    }
    Ret::eax(0)
}

/// [`start_up_0605`].
pub const START_UP_0605: Sig = Sig {
    len: 318,
    hash: 0xde4a_59f6_89f5_b32e,
    calls: &[
        (0x68, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x6d, Callee::Sig(&FACING_PLACES_0605)),
        (0x72, Callee::Sig(&crate::o4_effect::START_TURNED_0605_7DB0)),
        (0xb2, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xf0, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0xfa, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0605`'s load step (0x10006c40, the 118-byte shape) running
/// [`start_up_0605`].
pub fn load_0605<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0605)
}

/// [`load_0605`].
pub const LOAD_0605: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0605))],
    imports: &[],
};
