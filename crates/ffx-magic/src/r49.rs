//! Round 49's ports (own rows of `magic_0396`): the curves' start-ups,
//! the entry choosing between them and the curves' task.

use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
use crate::{Callee, Frame, Game, Image, Sig};

const ONE: u32 = 0x3f80_0000;

/// The second and third offsets of a curve record: the second's `x` and
/// result vector (frame offsets below `ebp`), the third's matrix, `x`
/// and result.
struct Moves {
    v2x: u32,
    out2: u32,
    m3: u32,
    v3x: u32,
    out3: u32,
}

/// One record of `magic_0396`'s curve start-ups: the object start's
/// call (its `rel32`), index word and scale (`true`: `xmm1`, else
/// `xmm2`); the first offset `(x, z)` and its result vector; whether the
/// first middle point starts as the summon point (else the target's `y`
/// is lowered by `K34c`); the second and third offsets (with them the
/// first two middle points move by the third's result); per middle point
/// and axis whether the sum is `A + d` (else `d + A`); the two middle
/// points' `y` drops (operands); the record's flag words (`+0x530`
/// written later when `None`).
struct Curve {
    start_call: u32,
    index: u32,
    scale_xmm1: bool,
    v1: (u32, u32),
    out1: u32,
    mid_from_summon: bool,
    moves: Option<Moves>,
    a_first: [[bool; 3]; 3],
    drops: (u32, u32),
    flags: (Option<u32>, u32),
}

/// The operands a curve start-up reads its globals and constants
/// through (each the first instruction naming it), and where the object
/// start's return address sits below the entry `esp`.
struct CurveOps {
    frame: u32,
    imports: u32,
    k33c: u32,
    k370: u32,
    sel: u32,
    handles: u32,
    k34c: u32,
    k31c: u32,
    k32c: u32,
    k338: u32,
}

/// `ebp` as the original has it in a /GS frame of `push ebp; mov ebp,
/// esp` under the entry `esp` (the return address).
fn ebp_of(esp: u32) -> u32 {
    esp.wrapping_sub(4)
}

/// One curve record `R` (`edi = R + 0x4b0` in the original): the object
/// start ([`start_object_anim_2`](crate::s6_misc::start_object_anim_2),
/// `edx` = `R`, the stack `(index, R, 0, 0)`, the scale in `xmm3`), the
/// yaw `R + 0xc4`; the summon point `P` (frame `ebp - 0x14`, the summon's
/// `S + 0xbe0` moved by the first offset turned by the yaw, slots
/// 235/328/329/229/248), its `y` raised by `K33c`, into `R + 0xd0`, `+
/// 0x488`, `+0x498` (`A`, whose `y` becomes `S[0xbe4] - K370`) and, by the
/// record, `+0x4a8`; the target point `B` (`R + 0x4d8`, slots 262 and 268
/// on the selected actor's handle, `w` 1) moved by the second offset;
/// the third offset alone into `P`; the three middle points `M1`..`M3`
/// (`R + 0x4a8`, `+0x4b8`, `+0x4c8`): `A + (B - A) k` with `k` = `K31c`,
/// `K32c`, `K338` (each sum in the original's operand order), `M1` and
/// `M2` lowered by the drops and, with the offsets, moved by `P`'s
/// `x`/`z`; each `w` = `B`'s; the flag words.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn curve_points<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (esp, s, r, yaw): (u32, u32, u32, f32),
    c: &Curve,
    ops: &CurveOps,
    xmm: [u128; 4],
) {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, at: u32| g.read::<f32>(at);
    let t = |g: &G, n: u32| slot_target(g, ops.imports, n);
    let ebp = ebp_of(esp);
    let (v, p, m1) = (ebp - 0x24, ebp - 0x14, ebp - 0x64);
    // push 0, 0, R, index; call (the three registers pushed before)
    let call_esp = esp.wrapping_sub(ops.frame);
    for (i, word) in (1u32..).zip([c.index, r, 0, 0]) {
        g.write::<u32>(call_esp + 4 * i, word);
    }
    let mut xmm = xmm;
    xmm[3] = if c.scale_xmm1 { xmm[1] } else { xmm[2] };
    let start = Frame {
        at: img.rel32_target(f + c.start_call),
        esp: call_esp,
        eax: c.index,
        ecx: r,
        edx: r,
        xmm,
    };
    crate::s6_misc::start_object_anim_2(g, img, &start);
    let yaw_word = g.read::<u32>(s + 0x2450);
    g.write::<u32>(r + 0xc4, yaw_word);
    // an offset `(x, z)` turned by the yaw (slots 235, 328 0, 329 yaw,
    // 229) and added to `to` (slot 248)
    let offset = |g: &mut G, m: u32, out: u32, (x, z): (u32, u32), to: u32| {
        g.write::<u32>(v, x);
        g.write::<u32>(v + 8, z);
        // call width: 1
        g.call(t(g, 235), 0, &[m]);
        // call width: 3
        g.call(t(g, 328), 0, &[m, m, 0]);
        // call width: 3
        g.call(t(g, 329), 0, &[m, m, yaw.to_bits()]);
        g.write::<u32>(v + 0xc, ONE);
        // call width: 3
        g.call(t(g, 229), 0, &[out, m, v]);
        // call width: 3
        g.call(t(g, 248), 0, &[to, to, out]);
    };
    offset(g, m1, ebp - c.out1, c.v1, p);
    let x = g.read::<u32>(p);
    let y = add(k(g, p + 4), k(g, ops.k33c)).to_bits();
    let z = g.read::<u32>(p + 8);
    let points: &[u32] = if c.mid_from_summon {
        &[0xd0, 0x488, 0x498, 0x4a8]
    } else {
        &[0xd0, 0x488, 0x498]
    };
    for &o in points {
        for (j, word) in (0u32..).zip([x, y, z, ONE]) {
            g.write::<u32>(r + o + 4 * j, word);
        }
    }
    let ay = sub(k(g, s + 0xbe4), k(g, ops.k370));
    g.write::<u32>(p + 0xc, ONE);
    g.write::<u32>(p + 4, y);
    g.write::<f32>(r + 0x49c, ay);
    let sel = g.read::<u8>(g.read::<u32>(ops.sel));
    let h = g.read::<u32>(ops.handles.wrapping_add(4 * u32::from(sel)));
    let b = r + 0x4d8;
    // call width: 4
    g.call(t(g, 262), 0, &[h, b, b + 4, b + 8]);
    // call width: 3
    g.call(t(g, 268), 0, &[h, 0x10, b]);
    g.write::<u32>(b + 0xc, ONE);
    if !c.mid_from_summon {
        let by = sub(k(g, b + 4), k(g, ops.k34c));
        g.write::<f32>(b + 4, by);
    }
    if let Some(mv) = &c.moves {
        offset(g, m1, ebp - mv.out2, (mv.v2x, 0), b);
        g.write::<u32>(b + 0xc, ONE);
        for (j, word) in (0u32..).zip([0, 0, 0, ONE]) {
            g.write::<u32>(p + 4 * j, word);
        }
        offset(g, ebp - mv.m3, ebp - mv.out3, (mv.v3x, 0), p);
        g.write::<u32>(p + 0xc, ONE);
    }
    let a = r + 0x498;
    let read3 = |g: &G, at: u32| [k(g, at), k(g, at + 4), k(g, at + 8)];
    let (av, bv) = (read3(g, a), read3(g, b));
    let bw = g.read::<u32>(b + 0xc);
    // A + (B - A) k, each axis's sum in the original's order
    let mid = |kk: f32, first: [bool; 3]| {
        let mut out = [0.0f32; 3];
        for j in 0..3 {
            let d = mul(sub(bv[j], av[j]), kk);
            out[j] = if first[j] {
                add(av[j], d)
            } else {
                add(d, av[j])
            };
        }
        out
    };
    let mut m1v = mid(k(g, ops.k31c), c.a_first[0]);
    let mut m2v = mid(k(g, ops.k32c), c.a_first[1]);
    let m3v = mid(k(g, ops.k338), c.a_first[2]);
    m1v[1] = sub(m1v[1], k(g, w(c.drops.0)));
    m2v[1] = sub(m2v[1], k(g, w(c.drops.1)));
    if c.moves.is_some() {
        let (px, pz) = (k(g, p), k(g, p + 8));
        for m in [&mut m1v, &mut m2v] {
            m[0] = add(m[0], px);
            m[2] = add(m[2], pz);
        }
    }
    for (o, pt) in [(0x4a8u32, m1v), (0x4b8, m2v), (0x4c8, m3v)] {
        for (j, x) in (0u32..).zip(pt) {
            g.write::<f32>(r + o + 4 * j, x);
        }
        g.write::<u32>(r + o + 0xc, bw);
    }
    if let Some(x) = c.flags.0 {
        g.write::<u32>(r + 0x530, x);
    }
    g.write::<u32>(r + 0x534, c.flags.1);
}

/// The target point `B` (`R + 0x4d8`) copied to `R + 0x4e8` and `+0x4f8`.
fn copy_target<G: Game>(g: &mut G, r: u32) {
    let b = r + 0x4d8;
    for to in [0x4e8u32, 0x4f8] {
        for j in 0..4 {
            let x = g.read::<u32>(b + 4 * j);
            g.write::<u32>(r + to + 4 * j, x);
        }
    }
}

/// The part `R + 0x260` into the table word `at` and set up: the word
/// `word`, `u16` -1 and 0x200, `+0x264` 1, 32 copies of `R + 0x488`, the
/// colours `+0x46c` (0x80808080, then 0x40404040), sizes and counts.
fn part_setup<G: Game>(g: &mut G, r: u32, word: u32, at: u32) {
    let part = r + 0x260;
    g.write::<u32>(at, part);
    g.write::<u32>(part, word);
    g.write::<u16>(r + 0x268, 0xffff);
    g.write::<u32>(r + 0x264, 1);
    g.write::<u16>(r + 0x26a, 0x200);
    g.write::<u32>(r + 0x46c, 0x8080_8080);
    g.write::<u32>(r + 0x470, 0);
    g.write::<u32>(r + 0x480, 0);
    g.write::<u32>(r + 0x474, ONE);
    g.write::<u32>(r + 0x478, ONE);
    for n in 0..0x20 {
        for j in 0..4 {
            let x = g.read::<u32>(r + 0x488 + 4 * j);
            g.write::<u32>(r + 0x26c + 0x10 * n + 4 * j, x);
        }
    }
    g.write::<u32>(r + 0x484, 0x40);
    g.write::<u32>(r + 0x47c, 0x3f00_0000);
    g.write::<u32>(r + 0x46c, 0x4040_4040);
}

/// The operands of a curve start-up's inlined node take and append: the
/// head (read; written), the callback, the count's store, the first node
/// (read; written), the last (read; written).
struct NodeOps {
    head: u32,
    head_to: u32,
    cb: u32,
    count_to: u32,
    first: u32,
    first_to: u32,
    last: u32,
    last_to: u32,
}

/// A node off the free ring ([`ring_take`](crate::r47::ring_take); `free`
/// nodes left) with `R` in `+0x14`, appended to the task list. Returns
/// whether the list had a first node and its last node.
fn node_for<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    r: u32,
    free: u16,
    o: &NodeOps,
) -> (bool, u32) {
    let w = |x: u32| img.word(f + x);
    let mut left = free;
    let head = g.read::<u32>(w(o.head));
    let (node, _) =
        crate::r47::ring_take(g, img, f, head, &mut left, (o.head_to, o.cb, o.count_to));
    let first = g.read::<u32>(w(o.first));
    g.write::<u32>(node.wrapping_add(0x14), r);
    let last = g.read::<u32>(w(o.last));
    crate::r47::task_append(g, img, f, node, first, last, o.first_to);
    g.write::<u32>(w(o.last_to), node);
    (first != 0, last)
}

/// A curve start-up's prelude: the cookie, the word `*(*G_W + 0xc)`
/// (frame `ebp - word_at`), the summon `S = *G_S`, the yaw `S[0x2450] -
/// K320` wrapped once by `K364` (frame `ebp - yaw_at`), the offset
/// vector's `y` 0 and `w` 1. Returns `(word, S, yaw)`.
fn curves_prelude<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    (word_at, yaw_at): (u32, u32),
) -> (u32, u32, f32) {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = ebp_of(esp);
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    g.write::<u32>(ebp - 4, cookie);
    let word = g.read::<u32>(g.read::<u32>(w(0x14)).wrapping_add(0xc));
    let s = g.read::<u32>(w(0x1b));
    g.write::<u32>(ebp - word_at, word);
    let mut yaw = sub(g.read::<f32>(s + 0x2450), k(g, 0x2e));
    if yaw > k(g, 0x3d) {
        yaw = sub(yaw, k(g, 0x4f));
    } else if k(g, 0x59) > yaw {
        yaw = add(yaw, k(g, 0x66));
    }
    g.write::<f32>(ebp - yaw_at, yaw);
    g.write::<u32>(ebp - 0x20, 0);
    g.write::<u32>(ebp - 0x18, ONE);
    (word, s, yaw)
}

/// The summon point `S + 0xbe0` into the frame's `P` (`ebp - 0x14`).
fn summon_point<G: Game>(g: &mut G, s: u32, esp: u32) {
    let ebp = ebp_of(esp);
    for j in 0..4 {
        let x = g.read::<u32>(s + 0xbe0 + 4 * j);
        g.write::<u32>(ebp - 0x14 + 4 * j, x);
    }
}

/// `magic_0396`'s curves start-up (0x10005740, 5993 bytes; internal, no
/// arguments, run by `magic_0396`'s curves entry; a /GS frame;
/// [`curves_prelude`]: the yaw at `ebp - 0x228`). For each of the four
/// curve records `R` (0x540 bytes from `S + 0xf50`) in turn: its points
/// ([`curve_points`], each record its own offsets and flags; the scale
/// `xmm1` = `K308`, `xmm2` = `K300`); its target point copied
/// ([`copy_target`]); with the first flag word 2 (records 2 and 3) the
/// part set up ([`part_setup`] into `G_T[i]`); `R + 0x528` = `5 i`,
/// `+0x52c` 50; a task node for `R` ([`node_for`], callback `0x10005380`,
/// the curves' task). Returns 4.
#[allow(clippy::too_many_lines)] // the four records' parameters
pub fn curves_start_0396<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let esp = fr.esp;
    let w = |o: u32| img.word(f + o);
    let (word, s, yaw) = curves_prelude(g, img, f, esp, (0x238, 0x228));
    let ops = CurveOps {
        frame: 0x25c,
        imports: w(0x12d),
        k33c: w(0x1ca),
        k370: w(0x247),
        sel: w(0x261),
        handles: w(0x26e),
        k34c: w(0xccd),
        k31c: w(0x40f),
        k32c: w(0x474),
        k338: w(0x52e),
    };
    // the sums: d + A in M1, A + d in M2's x and y and in M3's z
    let a_first = [[false; 3], [true, true, false], [false, false, true]];
    // the drops: K374 and K368 (records 0, 1), K368 and K35c (2, 3)
    let curves = [
        Curve {
            start_call: 0x106f,
            index: 1,
            scale_xmm1: true,
            v1: (0, 0xc1a0_0000),
            out1: 0xd4,
            mid_from_summon: false,
            moves: Some(Moves {
                v2x: 0x4040_0000,
                out2: 0xf4,
                m3: 0x164,
                v3x: 0x4120_0000,
                out3: 0x114,
            }),
            a_first,
            drops: (0x1403, 0x14c0),
            flags: (Some(0), 1),
        },
        Curve {
            start_call: 0xb51,
            index: 2,
            scale_xmm1: true,
            v1: (0, 0xc1a0_0000),
            out1: 0x84,
            mid_from_summon: false,
            moves: Some(Moves {
                v2x: 0xc040_0000,
                out2: 0x94,
                m3: 0x1e4,
                v3x: 0xc120_0000,
                out3: 0xb4,
            }),
            a_first,
            drops: (0xee5, 0xfa2),
            flags: (Some(1), 0),
        },
        Curve {
            start_call: 0x635,
            index: 0,
            scale_xmm1: false,
            v1: (0x4000_0000, 0xc190_0000),
            out1: 0x124,
            mid_from_summon: true,
            moves: Some(Moves {
                v2x: 0x4040_0000,
                out2: 0x74,
                m3: 0x224,
                v3x: 0x41a0_0000,
                out3: 0xa4,
            }),
            a_first,
            drops: (0x9c7, 0xa84),
            flags: (Some(2), 0),
        },
        Curve {
            start_call: 0x118,
            index: 0,
            scale_xmm1: false,
            v1: (0xc000_0000, 0xc190_0000),
            out1: 0xc4,
            mid_from_summon: true,
            moves: Some(Moves {
                v2x: 0xc040_0000,
                out2: 0x104,
                m3: 0x1a4,
                v3x: 0xc1a0_0000,
                out3: 0xe4,
            }),
            a_first,
            drops: (0x4b0, 0x56d),
            flags: (Some(2), 0),
        },
    ];
    let nodes = NodeOps {
        head: 0x167a,
        head_to: 0x1697,
        cb: 0x16d0,
        count_to: 0x16db,
        first: 0x16e1,
        first_to: 0x170c,
        last: 0x16f9,
        last_to: 0x173f,
    };
    let mut xmm = fr.xmm;
    xmm[1] = u128::from(g.read::<u32>(w(0x7e)));
    xmm[2] = u128::from(g.read::<u32>(w(0x76)));
    let mut counter = 0u32;
    for (i, c) in (0u32..).zip(&curves) {
        let r = s + 0xf50 + 0x540 * i;
        summon_point(g, s, esp);
        curve_points(g, img, f, (esp, s, r, yaw), c, &ops, xmm);
        copy_target(g, r);
        if g.read::<u32>(r + 0x530) == 2 {
            part_setup(g, r, word, w(0x15cc).wrapping_add(4 * i));
        }
        let free = g.read::<u16>(w(0x165b));
        g.write::<u32>(r + 0x528, counter);
        g.write::<u32>(r + 0x52c, 0x32);
        node_for(g, img, f, r, free, &nodes);
        counter = counter.wrapping_add(5);
    }
    4
}

/// [`curves_start_0396`] (the `rel32`s: the object start, four times,
/// and the cookie check).
pub const CURVES_START_0396: Sig = Sig {
    len: 5993,
    hash: 0x6f51_6064_6f96_3895,
    calls: &[
        (0x118, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x635, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0xb51, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x106f, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x1761, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0396`'s second curves start-up (0x10006eb0, 3737 bytes; the
/// curves entry's tail jump, no arguments; a /GS frame;
/// [`curves_prelude`]: the yaw at `ebp - 0x158`). [`curves_start_0396`]
/// for three records with these changes: every object start's index 0
/// and scale `K300`; record 0 has no second and third offsets (its
/// middle points stay where `A + (B - A) k` puts them), records 1 and 2
/// the summon's middle point start; their own sums' orders; every record
/// gets its part ([`part_setup`]) and the first flag word 2 (after it);
/// `R + 0x528` = `6 i`. Returns what the original's last instructions
/// leave in `eax`: the old last node with the task list not empty, else
/// 0xffff after a node off the ring (else the last record's `6 i`, but
/// the original writes through node 0 first then and faults).
#[allow(clippy::too_many_lines)] // the three records' parameters
pub fn curves_start2_0396<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    let f = fr.at;
    let esp = fr.esp;
    let w = |o: u32| img.word(f + o);
    let (word, s, yaw) = curves_prelude(g, img, f, esp, (0x168, 0x158));
    let ops = CurveOps {
        frame: 0x18c,
        imports: w(0x11a),
        k33c: w(0x1ba),
        k370: w(0x237),
        sel: w(0x251),
        handles: w(0x25e),
        k34c: w(0xada),
        k31c: w(0x3f8),
        k32c: w(0x7f7),
        k338: w(0x7ff),
    };
    // M2 (d + A, A + d, A + d) and M3 (A + d, d + A, A + d) in all three
    let tail = [[false, true, true], [true, false, true]];
    let curves = [
        Curve {
            start_call: 0xf6,
            index: 0,
            scale_xmm1: false,
            v1: (0, 0xc1a0_0000),
            out1: 0xc4,
            mid_from_summon: false,
            moves: None,
            a_first: [[false, false, true], tail[0], tail[1]],
            drops: (0xb89, 0xc22),
            flags: (None, 1),
        },
        Curve {
            start_call: 0xf6,
            index: 0,
            scale_xmm1: false,
            v1: (0x4000_0000, 0xc190_0000),
            out1: 0x84,
            mid_from_summon: true,
            moves: Some(Moves {
                v2x: 0x4040_0000,
                out2: 0xb4,
                m3: 0x114,
                v3x: 0x41a0_0000,
                out3: 0xd4,
            }),
            a_first: [[false; 3], tail[0], tail[1]],
            drops: (0x7ef, 0x8b4),
            flags: (None, 0),
        },
        Curve {
            start_call: 0xf6,
            index: 0,
            scale_xmm1: false,
            v1: (0xc000_0000, 0xc190_0000),
            out1: 0xa4,
            mid_from_summon: true,
            moves: Some(Moves {
                v2x: 0xc040_0000,
                out2: 0x74,
                m3: 0x154,
                v3x: 0xc1a0_0000,
                out3: 0x94,
            }),
            a_first: [[false, false, true], tail[0], tail[1]],
            drops: (0x7ef, 0x8b4),
            flags: (None, 0),
        },
    ];
    let nodes = NodeOps {
        head: 0xdba,
        head_to: 0xdd7,
        cb: 0xe10,
        count_to: 0xe1b,
        first: 0xe21,
        first_to: 0xe4c,
        last: 0xe39,
        last_to: 0xe6f,
    };
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(g.read::<u32>(w(0xbc)));
    let mut counter = 0u32;
    let mut eax = 0;
    for (i, c) in (0u32..).zip(&curves) {
        let r = s + 0xf50 + 0x540 * i;
        summon_point(g, s, esp);
        curve_points(g, img, f, (esp, s, r, yaw), c, &ops, xmm);
        copy_target(g, r);
        part_setup(g, r, word, w(0xd01).wrapping_add(4 * i));
        let free = g.read::<u16>(w(0xd80));
        g.write::<u32>(r + 0x528, counter);
        g.write::<u32>(r + 0x52c, 0x32);
        g.write::<u32>(r + 0x530, 2);
        let (listed, last) = node_for(g, img, f, r, free, &nodes);
        eax = if listed {
            last
        } else if free != 0 {
            0xffff
        } else {
            counter
        };
        counter = counter.wrapping_add(6);
    }
    crate::Ret::eax(eax)
}

/// [`curves_start2_0396`] (the `rel32`s: the object start and the cookie
/// check).
pub const CURVES_START2_0396: Sig = Sig {
    len: 3737,
    hash: 0xbff0_fa38_f154_fbc9,
    calls: &[
        (0xf6, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0xe91, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0396`'s curves entry (0x10007d50, 76 bytes; a table entry, no
/// arguments). With `*G_ON` set: the four curves
/// ([`curves_start_0396`]), then the fade record `S + 0xe90` (`S =
/// *G_S`; [`fade_task`](crate::s3_misc::fade_task), `edx` 30, `xmm1` and
/// `xmm2` two constants, the stack word 110) and `S + 0xe90` 0.7; returns
/// the fade's `eax`. Else the tail jump to the three curves
/// ([`curves_start2_0396`], the caller's stack).
pub fn curves_0396<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0xa));
    if g.read::<u32>(w(0x2)) == 0 {
        let tail = Frame {
            at: img.rel32_target(f + 0x48),
            ..*fr
        };
        return curves_start2_0396(g, img, &tail);
    }
    // push esi; call
    let start = Frame {
        at: img.rel32_target(f + 0x11),
        esp: fr.esp.wrapping_sub(8),
        ..*fr
    };
    curves_start_0396(g, img, &start);
    // push esi, the word; call
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 4, 0x6e);
    let mut xmm = fr.xmm;
    xmm[1] = u128::from(g.read::<u32>(w(0x21)));
    xmm[2] = u128::from(g.read::<u32>(w(0x19)));
    let fade = Frame {
        at: img.rel32_target(f + 0x33),
        esp,
        ecx: s + 0xe90,
        edx: 0x1e,
        xmm,
        ..*fr
    };
    let eax = crate::s3_misc::fade_task(g, img, &fade);
    g.write::<u32>(s + 0xe90, 0x3f33_3333);
    crate::Ret::eax(eax)
}

/// [`curves_0396`] (the `rel32`s: the four curves, the fade, the three
/// curves).
pub const CURVES_0396: Sig = Sig {
    len: 76,
    hash: 0xf93f_761a_5c50_8a88,
    calls: &[
        (0x11, Callee::Sig(&CURVES_START_0396)),
        (0x33, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x48, Callee::Sig(&CURVES_START2_0396)),
    ],
    imports: &[],
};

/// `magic_0396`'s curves task (0x10005380, 931 bytes; a task node's
/// callback, cdecl `(node)`; an 8-aligned /GS frame, `S0` = `esp` after
/// the three pushes: the point `Q` at `S0 + 0x10`, the matrix `M` at
/// `S0 + 0x20`). For the record `R = node[0x14]`:
/// - while the wait `R + 0x528` is positive it counts down;
/// - else the step `c = R + 0x52c` counts down to 0; with `c` not
///   negative `Q` is the curve's sample at `T`, a quartic in `t = c / K`
///   (`u = 1 - t`: `8 u^3 t + 6 u^4 + 6 t^2 u^2 + 16 t^3 u + 0 t^4` in
///   the original's order), each axis an
///   [`ease_sample`](crate::sb_misc::ease_sample) of the seven keys from
///   `R + 0x488`; with the flag `R + 0x530` 2 and `Q` away from `R + 0xd0`
///   the angles `R + 0xc0` turn toward it
///   ([`aim`](crate::m0171::aim)); `Q` into `R + 0xd0`;
/// - with `c` negative the count `R + 0x484` drops by 2 (the node given
///   back at 1 or less, [`give_back`](crate::sa_pool::give_back)) and
///   `Q` = `R + 0xd0`;
/// - with the flag 2 the trail of the part `R + 0x260` is drawn
///   ([`trail_draw`](crate::s4_misc::trail_draw), `M` a unit matrix at
///   scale 0.1, the stack `(Q, M)`);
/// - at `c` = 0: the record leaves the object list (`G_L`,
///   [`list_remove`](crate::l5_objhost::list_remove); when it was there
///   `**G_X` = `X`, slot 713 `(R)` and `u16 R + 0x124` loses bit 15), the
///   object `R + 0x130` starts
///   ([`start_object_anim_2`](crate::s6_misc::start_object_anim_2),
///   index 4/5/3 by the flag, scale `K310`, the stack `(i, i, 0, cb)`,
///   `cb` the object finish for `R + 0x534` = 1) at `Q` with the yaw `R +
///   0xc4`; with the flag 2 `c` becomes -1, else the node is given back.
///
/// `Q`'s `w` from the samples is a never-written frame word (`S0 +
/// 0x1c`); the port uses 0 (junk read, brief 2026-10-24). The task runner
/// (0x10004353) overwrites `eax` at once: the port returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn curves_task_0396<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    use crate::sb_sse::div;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x5c; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x5c);
    let cookie = g.read::<u32>(w(0xa)) ^ b0;
    g.write::<u32>(b0 + 0x58, cookie);
    let s0 = b0.wrapping_sub(0xc);
    let (q, m) = (s0 + 0x10, s0 + 0x20);
    let node = fr.arg(g, 0);
    let r = g.read::<u32>(node + 0x14);
    let wait = g.read::<i32>(r + 0x528);
    if wait > 0 {
        g.write::<i32>(r + 0x528, wait - 1);
        return crate::Ret::eax(0);
    }
    let c = g.read::<i32>(r + 0x52c);
    if c > 0 {
        g.write::<i32>(r + 0x52c, c - 1);
    }
    let c = g.read::<i32>(r + 0x52c);
    if c >= 0 {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let t = div(c as f32, k(g, 0x72));
        let u = sub(k(g, 0x6a), t);
        let four = k(g, 0x7a);
        let u2 = mul(u, u);
        let ut = mul(u, t);
        let t2 = mul(t, t);
        let mut x = mul(mul(mul(ut, u2), four), k(g, 0xbc));
        x = add(x, mul(mul(u2, u2), k(g, 0xac)));
        x = add(x, mul(mul(mul(t2, u2), four), k(g, 0xd3)));
        x = add(x, mul(mul(mul(t2, ut), four), four));
        x = add(x, mul(mul(t2, t2), 0.0));
        let ease = call(0x101);
        let sample = |g: &mut G, key: u32, esp: u32| {
            g.write::<u32>(esp + 4, 4);
            crate::sb_misc::ease_sample(g, img, ease, esp, r + key, 7, x)
        };
        let qx = sample(g, 0x488, s0 - 8);
        g.write::<f32>(q, qx);
        let qy = sample(g, 0x48c, s0 - 0xc);
        g.write::<f32>(q + 4, qy);
        let qz = sample(g, 0x490, s0 - 0x10);
        g.write::<f32>(q + 8, qz);
        // junk read: [S0 + 0x1c] never written; 0 (brief 2026-10-24)
        g.write::<u32>(q + 0xc, 0);
        if g.read::<u32>(r + 0x530) == 2 {
            let rd = |g: &G, o: u32| g.read::<f32>(r + o);
            let dy = sub(rd(g, 0xd4), qy);
            let dx = sub(rd(g, 0xd0), qx);
            let dz = sub(rd(g, 0xd8), qz);
            let sum = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
            #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
            let len = f64::from(sum).sqrt() as f32;
            if len > 0.0 {
                crate::m0171::aim(g, img, call(0x1b2), r + 0xc0, r + 0xd0, q);
            }
        }
        for j in 0..4 {
            let x = g.read::<u32>(q + 4 * j);
            g.write::<u32>(r + 0xd0 + 4 * j, x);
        }
    } else {
        let left = g.read::<i32>(r + 0x484);
        if left <= 1 {
            crate::sa_pool::give_back(g, img, call(0x38d), node);
            return crate::Ret::eax(0);
        }
        g.write::<i32>(r + 0x484, left - 2);
        for j in 0..4 {
            let x = g.read::<u32>(r + 0xd0 + 4 * j);
            g.write::<u32>(q + 4 * j, x);
        }
    }
    let flag = g.read::<u32>(r + 0x530);
    if flag == 2 {
        // call width: 1
        g.call(slot_target(g, w(0x252), 235), 0, &[m]);
        for o in [0x28u32, 0x14, 0] {
            g.write::<u32>(m + o, 0x3dcc_cccd);
        }
        // push M (235's), push Q; call
        let esp = s0.wrapping_sub(0xc);
        g.write::<u32>(esp + 4, q);
        g.write::<u32>(esp + 8, m);
        let draw = Frame {
            at: call(0x286),
            esp,
            ecx: r + 0x260,
            edx: m,
            ..*fr
        };
        crate::s4_misc::trail_draw(g, img, &draw);
    }
    if g.read::<u32>(r + 0x52c) != 0 {
        return crate::Ret::eax(0);
    }
    if crate::l5_objhost::list_remove(g, img, call(0x2a2), w(0x29d), r) != 0 {
        let at = g.read::<u32>(w(0x2ab));
        g.write::<u32>(at, w(0x2b2));
        // call width: 1
        g.call(slot_target(g, w(0x2b7), 713), 0, &[r]);
        let bits = g.read::<u16>(r + 0x124) & 0x7fff;
        g.write::<u16>(r + 0x124, bits);
    }
    let flag = g.read::<u32>(r + 0x530);
    let index = match flag {
        0 => 4,
        1 => 5,
        _ => 3,
    };
    let cb = if g.read::<u32>(r + 0x534) == 1 {
        w(0x305)
    } else {
        0
    };
    // push cb, 0, i, i; call
    let esp = s0.wrapping_sub(0x14);
    for (j, word) in (1u32..).zip([index, index, 0, cb]) {
        g.write::<u32>(esp + 4 * j, word);
    }
    let o = r + 0x130;
    let mut xmm = fr.xmm;
    xmm[3] = u128::from(g.read::<u32>(w(0x2f7)));
    let start = Frame {
        at: call(0x31a),
        esp,
        ecx: index,
        edx: o,
        xmm,
        ..*fr
    };
    crate::s6_misc::start_object_anim_2(g, img, &start);
    let yaw = g.read::<u32>(r + 0xc4);
    g.write::<u32>(o + 0xc4, yaw);
    for j in 0..4 {
        let x = g.read::<u32>(q + 4 * j);
        g.write::<u32>(o + 0xd0 + 4 * j, x);
    }
    if g.read::<u32>(r + 0x530) == 2 {
        g.write::<u32>(r + 0x52c, u32::MAX);
        return crate::Ret::eax(0);
    }
    crate::sa_pool::give_back(g, img, call(0x38d), node);
    crate::Ret::eax(0)
}

/// [`curves_task_0396`] (the `rel32`s: the three samples, the square
/// root, the aim, the trail draw, the list removal, the object start, the
/// cookie checks, the give-back).
pub const CURVES_TASK_0396: Sig = Sig {
    len: 931,
    hash: 0xc16c_217c_dac5_d93a,
    calls: &[
        (0x38, Callee::Any),
        (0x101, Callee::Sig(&crate::sb_misc::EASE_SAMPLE)),
        (0x11c, Callee::Sig(&crate::sb_misc::EASE_SAMPLE)),
        (0x137, Callee::Sig(&crate::sb_misc::EASE_SAMPLE)),
        (0x190, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x1b2, Callee::Sig(&crate::m0171::AIM)),
        (0x286, Callee::Sig(&crate::s4_misc::TRAIL_DRAW)),
        (0x2a2, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x31a, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x382, Callee::Any),
        (0x38d, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x39b, Callee::Any),
    ],
    imports: &[],
};
