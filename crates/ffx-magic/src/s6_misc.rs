//! Shared functions of the shared-6 list (rows 843 on of the shared-4
//! work list) without a family module yet. Each doc names the DLL and
//! address it was ported from, its size and how many DLLs carry it;
//! "`G`" is a global of the DLL (an operand of the code).

use crate::l5_objhost::{LIST_CUR, LIST_REMOVE, POOL_ALLOC, list_cur, list_remove};
use crate::nodes::{INSERT, node_insert};
use crate::opu_parts::{START_PART, start_part};
use crate::s3_misc::{COLOUR_NODE, colour_node};
use crate::s3_summon::{MODEL_NODES, model_nodes};
use crate::s4_misc::{
    SCREEN_PLACE, START_OBJECT_ANIM, screen_place, start_object, start_object_anim,
};
use crate::s4_small::{FADER_TASK, fader_task};
use crate::s4_summon::trunc;
use crate::s5_misc::list_push;
use crate::sa_misc::{SPRITE_FADE, sprite_fade};
use crate::sa_pool::{GIVE_BACK, TAKE, give_back, take};
use crate::sa_script::{ACTOR_SLOT256, actor_slot256};
use crate::sb_misc::{
    ANIM_RESET_2, MOVE_START, MOVE_START_2, anim_reset_2, move_start, move_start_2,
};
use crate::sb_nodes::{EFFECT_NODE, PARTICLE_NODE, effect_node, particle_node};
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Out, Ret, Sig};

/// The word at `f + off` of the code (an operand).
fn op(img: &Image, f: u32, off: u32) -> u32 {
    img.word(f + off)
}

/// `+= 1` on the word at `a`.
fn bump<G: Game>(g: &mut G, a: u32) {
    let v = g.read::<u32>(a);
    g.write::<u32>(a, v.wrapping_add(1));
}

/// The ring node `node` taken out of its ring (`prev.next = next`,
/// `next.prev = prev`, each read again after the store before, as the
/// original does) and closed on itself. Returns what the original
/// leaves in `eax`: the `prev` read last.
fn unring<G: Game>(g: &mut G, node: u32) -> u32 {
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node + 8);
    g.write::<u32>(prev.wrapping_add(8), next);
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(next.wrapping_add(4), prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    prev
}

/// A summon's staged nodes task (cdecl `(task)`, `task[0]` its ring node,
/// `task[1]` the state `S`: step `+0x18`, two clocks `+0x1c`/`+0x20`
/// (`+= K` each frame), the index `+0x24`, the frame count `+0x28`, which
/// goes up once on entry and once at the end; an aligned frame). The
/// steps:
/// - 0: the model nodes of `[G_A, L, *G_C, 0, 0]` (`*G_A = VT`) and the
///   colour node of `[G_D, G_E]` (`*G_D = VT2`), the seven move targets
///   0.5, the camera move started (time `K2`), then as step 10;
/// - 10: step 11, the count 100, the index 0;
/// - 11, once the count is past 15: for index `i`, the two nodes
///   `N1 = G_N1 + 0x2c i` and `N2 = G_N2 + 0x2c i` go on the DLL's list
///   `L` (vtable `VT3`; [`list_push`]) with `N1`'s point `+0x1c` the
///   option point 0x11 of record `G_R + 0x98 i` (slot 268 on `+0x54`,
///   `w` and `N1+0x28` 1.0) and `N2`'s colour `(1, 1, 1, A)` (`A` an
///   immediate: 0.3 in `magic_0401`); the node
///   `P = G_P + 0x14c i` inserted ([`node_insert`]: `(P, L, G_Q, N1,
///   G_Q2, N2)`), `P+0x148 = 1`, the particle `G_T + 0x100 i` set up
///   ([`particle_node`]: `(t, G_E, P, *G_O, G_O2, 0)`, `t[0] = VT4`)
///   and the effect node `G_F + 0x38 i` ([`effect_node`]: `(e, L, P,
///   G_F2, CB, 0)`, then vtable `VT5`, `+0x2c |= 1`, `+0x30 = 0`); the
///   index goes up (past `*G_K` the step too) and the count restarts;
/// - 12, once the count is past 80: the seven targets 1.0, the second
///   move started (times `K3`, `K4`), step 0x2710;
/// - 0x2710: the task's node leaves its ring.
///
/// Returns what the original leaves in `eax`: the step (0x2710: the
/// node's `prev`; after step 11's nodes the new index; after step 0 the
/// colour node's result; after step 12's move the move's flags).
///
/// `magic_0401` 0x10006960 (1026 bytes; 2 DLLs).
#[allow(clippy::too_many_lines)] // the original's nodes, one by one
pub fn staged_nodes_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 4);
    let k = g.read::<f32>(w(0x1b));
    let c1 = add(g.read::<f32>(s + 0x1c), k);
    let step = g.read::<u32>(s + 0x18);
    bump(g, s + 0x28);
    g.write::<f32>(s + 0x1c, c1);
    let c2 = add(g.read::<f32>(s + 0x20), g.read::<f32>(w(0x33)));
    g.write::<f32>(s + 0x20, c2);
    let done = |g: &mut G, eax: u32| {
        bump(g, s + 0x28);
        Ret::eax(eax)
    };
    // the aligned frame: the set-up records the callees get in `ecx`
    let top = fr.esp.wrapping_sub(4) & !7;
    let rec = top.wrapping_sub(0x18);
    let callee = |at: u32, ecx: u32| Frame {
        at,
        esp: top.wrapping_sub(0x3c),
        ecx,
        ..*fr
    };
    let targets = |g: &mut G, v: u32, ops: [u32; 7]| {
        for o in ops {
            g.write::<u32>(w(o), v);
        }
    };
    let mut eax = step;
    match step {
        0 | 10 | 11 => {}
        12 => {
            if g.read::<i32>(s + 0x28) <= 0x50 {
                return done(g, step);
            }
            let (t2, t1) = (g.read::<f32>(w(0x39a)), g.read::<f32>(w(0x3a2)));
            targets(
                g,
                0x3f80_0000,
                [0x3a8, 0x3b2, 0x3bc, 0x3c6, 0x3d0, 0x3da, 0x3e4],
            );
            let r = move_start_2(g, img, img.rel32_target(f + 0x3ed), t1, t2);
            g.write::<u32>(s + 0x18, 0x2710);
            return done(g, r);
        }
        0x2710 => {
            let node = g.read::<u32>(task);
            let prev = unring(g, node);
            return done(g, prev);
        }
        _ => return done(g, step),
    }
    if step == 0 {
        let c = g.read::<u32>(w(0x63));
        for (i, v) in (0u32..).zip([w(0x6f), w(0x77), c, 0]) {
            g.write::<u32>(rec + 4 * i, v);
        }
        g.write::<u8>(rec + 0x10, 0);
        g.write::<u32>(w(0x8e), w(0x92));
        model_nodes(g, img, &callee(img.rel32_target(f + 0x97), rec));
        let rec2 = top.wrapping_sub(0x20);
        g.write::<u32>(rec2, w(0xa3));
        g.write::<u32>(rec2 + 4, w(0xab));
        g.write::<u32>(w(0xb1), w(0xb5));
        let r = colour_node(g, img, &callee(img.rel32_target(f + 0xba), rec2));
        let t = g.read::<f32>(w(0xc2));
        targets(g, 0x3f00_0000, [0xc8, 0xd2, 0xdc, 0xe6, 0xf0, 0xfa, 0x104]);
        eax = move_start(g, img, img.rel32_target(f + 0x10d), t, r);
        g.write::<u32>(s + 0x18, 10);
    }
    if step != 11 {
        bump(g, s + 0x18);
        g.write::<u32>(s + 0x28, 100);
        g.write::<u32>(s + 0x24, 0);
    }
    if g.read::<i32>(s + 0x28) <= 0xf {
        return done(g, eax);
    }
    let i = g.read::<u32>(s + 0x24);
    let p = i.wrapping_mul(0x14c).wrapping_add(w(0x13f));
    g.write::<u32>(top.wrapping_sub(0x24), p);
    let n1 = i.wrapping_mul(0x2c).wrapping_add(w(0x157));
    let n2 = i.wrapping_mul(0x2c).wrapping_add(w(0x15d));
    g.write::<u32>(top.wrapping_sub(0x28), n2);
    g.write::<u16>(i.wrapping_mul(0x2c).wrapping_add(w(0x14f)), 0xff);
    let part = (i << 8).wrapping_add(w(0x18b));
    g.write::<u32>(top.wrapping_sub(0x20), part);
    list_push(
        g,
        img,
        f,
        n1,
        [
            0x16e, 0x175, 0x17a, 0x185, 0x194, 0x19c, 0x1a8, 0x1ae, 0x1b4,
        ],
    );
    let r = g
        .read::<u32>(s + 0x24)
        .wrapping_mul(0x98)
        .wrapping_add(w(0x1c2));
    let h = g.read::<u32>(r.wrapping_add(0x54));
    let pt = n1.wrapping_add(0x1c);
    let t = crate::runtime::slot_target(g, w(0x1d0), 268);
    g.call(t, 0, &[h, 0x11, pt]);
    g.write::<u32>(pt + 0xc, 0x3f80_0000);
    g.write::<u32>(n1 + 0x28, 0x3f80_0000);
    list_push(
        g,
        img,
        f,
        n2,
        [
            0x1fd, 0x204, 0x209, 0x211, 0x219, 0x221, 0x22d, 0x237, 0x23d,
        ],
    );
    for (o, v) in [
        (0x1cu32, 0x3f80_0000u32),
        (0x20, 0x3f80_0000),
        (0x24, 0x3f80_0000),
        // an immediate that differs between the copies (0.3 in 0401)
        (0x28, w(0x259)),
    ] {
        g.write::<u32>(n2 + o, v);
    }
    for (i, v) in (0u32..).zip([p, w(0x26d), w(0x275), n1, w(0x281), n2]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    node_insert(g, img, &callee(img.rel32_target(f + 0x286), rec));
    let part = g.read::<u32>(top.wrapping_sub(0x20));
    let o = g.read::<u32>(w(0x28f));
    g.write::<u8>(p + 0x148, 1);
    g.write::<u32>(rec, part);
    g.write::<u32>(part, w(0x2a0));
    for (i, v) in (1u32..).zip([w(0x2ac), p, o, w(0x2bc), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    particle_node(g, img, img.rel32_target(f + 0x2c9), rec);
    let i = g.read::<u32>(s + 0x24);
    let e = i.wrapping_mul(0x38).wrapping_add(w(0x2e8));
    for (i, v) in (0u32..).zip([e, w(0x2d4), p, w(0x2f8), w(0x300), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    effect_node(g, img, img.rel32_target(f + 0x30d), rec);
    let e = g.read::<u32>(rec);
    g.write::<u32>(e, w(0x317));
    let i = g.read::<u32>(s + 0x24).wrapping_mul(0x38);
    let fl = g.read::<u32>(i.wrapping_add(w(0x32a))) | 1;
    g.write::<u32>(i.wrapping_add(w(0x32a)), fl);
    g.write::<u32>(i.wrapping_add(w(0x332)), 0);
    bump(g, s + 0x24);
    let i = g.read::<u32>(s + 0x24);
    if i.cast_signed() >= g.read::<i32>(w(0x342)) {
        bump(g, s + 0x18);
    }
    g.write::<u32>(s + 0x28, 0);
    done(g, i)
}

/// [`staged_nodes_task`] (the `rel32`s: [`model_nodes`],
/// [`colour_node`], [`move_start`], [`node_insert`], [`particle_node`],
/// [`effect_node`] and [`move_start_2`]).
pub const STAGED_NODES_TASK: Sig = Sig {
    len: 1026,
    hash: 0xc40a_029d_ea97_4c23,
    calls: &[
        (0x97, Callee::Sig(&MODEL_NODES)),
        (0xba, Callee::Sig(&COLOUR_NODE)),
        (0x10d, Callee::Sig(&MOVE_START)),
        (0x286, Callee::Sig(&INSERT)),
        (0x2c9, Callee::Sig(&PARTICLE_NODE)),
        (0x30d, Callee::Sig(&EFFECT_NODE)),
        (0x3ed, Callee::Sig(&MOVE_START_2)),
    ],
    imports: &[],
};

/// [`staged_nodes_task`]'s copy in `magic_0426` 0x10007130 (another
/// colour immediate).
pub const STAGED_NODES_TASK_B: Sig = Sig {
    hash: 0x60a4_0c9f_7a13_2d1a,
    ..STAGED_NODES_TASK
};

/// The vertex `i` of a part's `i16` vertex table at `verts` (three
/// halfwords each) as floats.
fn vertex<G: Game>(g: &G, verts: u32, i: u16) -> [f32; 3] {
    let at = verts.wrapping_add(6 * u32::from(i));
    [0u32, 2, 4].map(|o| f32::from(g.read::<i16>(at.wrapping_add(o))))
}

/// The centres of 18 models' parts (cdecl, no arguments; a /GS frame).
/// For each record `c` (0x20 apart) of the table at `*G_T + 0x23c`: its
/// model `m`, the strips at `m + m[1]` and the `i16` vertices at `m +
/// m[2]`. For the `(i16) m+0x10` polygons, strip by strip (kind byte
/// `+1`, count `+2`, the polygons from `+0x10`): each triangle (kind 2,
/// 0x20 bytes, vertex numbers at `+0xc..`) or quad (kind 6, 0x28 bytes,
/// at `+0x10..`) has its vertices summed in a local `v` (`w` 0 once per
/// model), `v /= 3` or 4 (slot 564 `(v, v, n)`) and `v` added to the sum
/// (slot 248 `(sum, sum, v)`, a local cleared per model); other kinds
/// are passed over. Then `sum /= count` (slot 564) when the count is
/// positive, and the sum, `w` 1.0, goes to `*G_S + 0x2ec0 + 0x60 i`.
/// Returns the last slot's `eax` (the last count's halfword when the last
/// model has none).
///
/// `magic_0563` 0x10001b20 (1045 bytes; 2 DLLs).
pub fn part_centres<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ebp = fr.esp.wrapping_sub(4);
    let (sum, v) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let mut out = g.read::<u32>(w(0x14)).wrapping_add(0x2ec0);
    let mut eax = 0;
    for c in (0..0x240u32).step_by(0x20) {
        let m = g.read::<u32>(g.read::<u32>(w(0x31)).wrapping_add(c + 0x23c));
        let mut strip = g.read::<u32>(m.wrapping_add(4)).wrapping_add(m);
        let verts = g.read::<u32>(m.wrapping_add(8)).wrapping_add(m);
        for o in [0u32, 4, 8, 0xc] {
            g.write::<u32>(sum + o, 0);
        }
        g.write::<u32>(v + 0xc, 0);
        let mut left = i32::from(g.read::<i16>(m.wrapping_add(0x10)));
        while left > 0 {
            let count = i32::from(g.read::<i16>(strip.wrapping_add(2)));
            let mut p = strip.wrapping_add(0x10);
            // the vertex numbers' offset, the vertices, the polygon's
            // size, its divisor and the slot table operands of its calls
            let shape = match g.read::<i8>(strip.wrapping_add(1)) {
                2 => Some((0xcu32, 3u32, 0x20u32, 3.0f32, 0x347u32, 0x362u32)),
                6 => Some((0x10, 4, 0x28, 4.0, 0x1fe, 0x214)),
                _ => None,
            };
            if let Some((at, n, size, by, t1, t2)) = shape {
                for _ in 0..count.max(0) {
                    let mut s = vertex(g, verts, g.read::<u16>(p.wrapping_add(at)));
                    for k in 1..n {
                        let x = vertex(g, verts, g.read::<u16>(p.wrapping_add(at + 2 * k)));
                        s = [add(x[0], s[0]), add(x[1], s[1]), add(x[2], s[2])];
                    }
                    for (o, x) in [0u32, 4, 8].into_iter().zip(s) {
                        g.write::<f32>(v + o, x);
                    }
                    let t = crate::runtime::slot_target(g, w(t1), 564);
                    g.call(t, 0, &[v, v, by.to_bits()]);
                    let t = crate::runtime::slot_target(g, w(t2), 248);
                    g.call(t, 0, &[sum, sum, v]);
                    p = p.wrapping_add(size);
                }
            }
            left -= count;
            strip = p;
        }
        let n = g.read::<u16>(m.wrapping_add(0x10));
        eax = u32::from(n);
        if n.cast_signed() > 0 {
            let t = crate::runtime::slot_target(g, w(0x3c0), 564);
            let by = f32::from(n.cast_signed());
            eax = g.call(t, 0, &[sum, sum, by.to_bits()]).eax;
        }
        for o in [0u32, 4, 8] {
            let x = g.read::<u32>(sum + o);
            g.write::<u32>(out + o, x);
        }
        g.write::<u32>(out + 0xc, 0x3f80_0000);
        out = out.wrapping_add(0x60);
    }
    Ret::eax(eax)
}

/// A spinning summon's second stage set up (cdecl, no arguments; a /GS
/// frame). With `B = *G_B`: the focus rolled (slot 371 `(0, 0x10000)`,
/// `(1, 0x20000)`); the objects `B+0x720` and `B+0x850` each taken off
/// the object list `L` when they are on it ([`list_remove`]): `*G_P =
/// ITEM`, freed (slot 713) and bit 15 of their `+0x124` cleared. The
/// point `(0, y, -50, 1)` (`y` 1.8 in the battle scene 0x18b0046 of slot
/// 520, else 0) turned by a matrix of yaw `B+0x3150` (slots 235, 328 with
/// 0, 329, 229) is added to `B+0xa90` (slot 248, `w` 1.0); `B+0xaa4` =
/// the yaw, `B+0xab4 = B+0xac4`. The matrix `M = B+0x9c0`: a unit one
/// (slot 235), its diagonal `K` times the data scale of `*G_CH` (slot
/// 277, ST0) and times the character's scales (slots 15, 16, 17; ST0
/// each), turned by `B+0xaa8`, `B+0xaa0`, `B+0xaa4` (slots 330, 328,
/// 329). The objects `B+0x260` (part 2, 0x15; scale `K2`) and `B` (part
/// 0, 0x3a; scale `K3`) started ([`start_object_anim`]), each with
/// `+0xd0..+0xdc = B+0xac0..`, `+0xc4` the yaw and `+0xd4` lowered by
/// `K4`/`K5`. The fader `F = B+0xc5c` reset (`(0, 40, 0, 40, 150, 150)`,
/// u16 `+0x18` 0) and a task node for callback `CB` ([`take`], kind 1,
/// `+0x14 = F`) appended to the task list, the fader state `B+0xc78`
/// started ([`fader_task`]) and set (`+0x2c |= 3`, `(1, 1, 0, 4, 2, 0)`
/// from `+0x4`, `+0x24 = 0x3a`), and draw mask bit 2 set (slots 557,
/// 560). Each call sees the words the earlier ones left on the stack.
/// Returns slot 560's `eax`.
///
/// `magic_0168` 0x100043d0 (1125 bytes; 2 DLLs).
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn spin_stage_two<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let slot = |g: &mut G, t: u32, s: u32, args: &[u32]| {
        let target = crate::runtime::slot_target(g, w(t), s);
        g.call(target, 0, args)
    };
    let st0 = |o: Out| crate::m0018::st0_f32(o.st0);
    slot(g, 0x11, 371, &[0, 0x10000]);
    slot(g, 0x28, 371, &[1, 0x20000]);
    let b = g.read::<u32>(w(0x3d));
    for (obj, ops) in [
        (b.wrapping_add(0x720), [0x52u32, 0x4d, 0x60, 0x67, 0x6c]),
        (b.wrapping_add(0x850), [0x90, 0x8b, 0x99, 0xa0, 0xa5]),
    ] {
        let [call, list, p, item, table] = ops;
        if list_remove(g, img, img.rel32_target(f + call), w(list), obj) != 0 {
            g.write::<u32>(g.read::<u32>(w(p)), w(item));
            slot(g, table, 713, &[obj]);
            let v = g.read::<u16>(obj + 0x124) & 0x7fff;
            g.write::<u16>(obj + 0x124, v);
        }
    }
    // the point (0, y, -50, 1) turned by the yaw, added to B+0xa90
    let yaw = g.read::<f32>(b + 0x3150);
    let (m, out, pt) = (at(0x64), at(0x24), at(0x14));
    g.write::<u32>(pt, 0);
    g.write::<u32>(pt + 4, 0);
    if slot(g, 0xc4, 520, &[]).eax == 0x018b_0046 {
        g.write::<u32>(pt + 4, 0x3fe6_6666);
    }
    g.write::<u32>(pt + 8, 0xc248_0000);
    g.write::<u32>(pt + 0xc, 0x3f80_0000);
    slot(g, 0xf6, 235, &[m]);
    slot(g, 0x123, 328, &[m, m, 0]);
    slot(g, 0x142, 329, &[m, m, yaw.to_bits()]);
    g.write::<u32>(pt + 0xc, 0x3f80_0000);
    slot(g, 0x15b, 229, &[out, m, pt]);
    let p = b.wrapping_add(0xa90);
    slot(g, 0x173, 248, &[p, p, out]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let x = g.read::<u32>(b + 0x3150);
    g.write::<u32>(b + 0xaa4, x);
    let x = g.read::<u32>(b + 0xac4);
    g.write::<u32>(b + 0xab4, x);
    // the character's matrix
    let mm = b.wrapping_add(0x9c0);
    slot(g, 0x1a1, 235, &[mm]);
    let ch = g.read::<u32>(w(0x1bb));
    let s = mul(st0(slot(g, 0x1b5, 277, &[ch])), g.read::<f32>(w(0x1d3)));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(mm + o, s);
    }
    let ch = g.read::<u32>(w(0x1ec));
    let sx = st0(slot(g, 0x1d8, 15, &[ch]));
    let x = mul(g.read::<f32>(mm), sx);
    g.write::<f32>(mm, x);
    let sy = st0(slot(g, 0x203, 16, &[ch]));
    let y = mul(sy, g.read::<f32>(mm + 0x14));
    g.write::<f32>(mm + 0x14, y);
    let sz = st0(slot(g, 0x212, 17, &[ch]));
    let z = mul(g.read::<f32>(mm + 0x28), sz);
    g.write::<f32>(mm + 0x28, z);
    for (t, s, o) in [
        (0x23cu32, 330u32, 0xaa8u32),
        (0x268, 328, 0xaa0),
        (0x287, 329, 0xaa4),
    ] {
        let a = g.read::<u32>(b + o);
        slot(g, t, s, &[mm, mm, a]);
    }
    // the two objects: started, `+0xd0..+0xdc = B+0xac0..`, `+0xc4` the
    // yaw, `+0xd4 = y - K` (`y`: B+0xab4 for the first, its own `+0xd4`,
    // just copied, for the second)
    let started = |g: &mut G, obj: u32, y_at: u32, k: u32| {
        for o in [0u32, 4, 8, 0xc] {
            let x = g.read::<u32>(b + 0xac0 + o);
            g.write::<u32>(obj + 0xd0 + o, x);
        }
        let yaw = g.read::<u32>(b + 0x3150);
        g.write::<u32>(obj + 0xc4, yaw);
        let y = sub(g.read::<f32>(y_at), g.read::<f32>(w(k)));
        g.write::<f32>(obj + 0xd4, y);
    };
    let a = b.wrapping_add(0x260);
    let k2 = g.read::<f32>(w(0x2a1));
    start_object_anim(g, img, img.rel32_target(f + 0x2b5), a, k2, 2, 0x15);
    started(g, a, b + 0xab4, 0x309);
    let k3 = g.read::<f32>(w(0x2c3));
    start_object_anim(g, img, img.rel32_target(f + 0x31f), b, k3, 0, 0x3a);
    started(g, b, b + 0xd4, 0x365);
    // the fader and its task node
    let fd = b.wrapping_add(0xc5c);
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x28),
        (8, 0),
        (0xc, 0x28),
        (0x14, 0x96),
        (0x10, 0x96),
    ] {
        g.write::<u32>(fd + o, v);
    }
    g.write::<u16>(fd + 0x18, 0);
    let node = take(g, img, img.rel32_target(f + 0x3b5), w(0x383));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(0x14), fd);
    g.write::<u32>(node.wrapping_add(4), 0);
    if g.read::<u32>(w(0x3bf)) != 0 {
        let tail = g.read::<u32>(w(0x3d2));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x3e4), node);
    }
    let st = b.wrapping_add(0xc78);
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x3f4), node);
    let fader = Frame {
        at: img.rel32_target(f + 0x3f9),
        esp: ebp.wrapping_sub(0x78),
        ecx: st,
        ..*fr
    };
    fader_task(g, img, &fader);
    let fl = g.read::<u16>(st + 0x2c) | 3;
    g.write::<u16>(st + 0x2c, fl);
    for (o, v) in [
        (4u32, 0x3f80_0000u32),
        (8, 0x3f80_0000),
        (0xc, 0),
        (0x10, 0x4080_0000),
        (0x14, 0x4000_0000),
        (0x18, 0),
        (0x24, 0x3a),
    ] {
        g.write::<u32>(st + o, v);
    }
    let mask = slot(g, 0x3fe, 557, &[]).eax | 4;
    Ret::eax(slot(g, 0x445, 560, &[mask]).eax)
}

/// [`spin_stage_two`] (the `rel32`s: two [`list_remove`]s, two
/// [`start_object_anim`]s, [`take`], [`fader_task`] and
/// `__security_check_cookie`).
pub const SPIN_STAGE_TWO: Sig = Sig {
    len: 1125,
    hash: 0x9c4f_6c91_9a1b_88c0,
    calls: &[
        (0x52, Callee::Sig(&LIST_REMOVE)),
        (0x90, Callee::Sig(&LIST_REMOVE)),
        (0x2b5, Callee::Sig(&START_OBJECT_ANIM)),
        (0x31f, Callee::Sig(&START_OBJECT_ANIM)),
        (0x3b5, Callee::Sig(&TAKE)),
        (0x3f9, Callee::Sig(&FADER_TASK)),
        (0x45d, Callee::Any),
    ],
    imports: &[],
};

/// [`part_centres`] (the `rel32`: `__security_check_cookie`).
pub const PART_CENTRES: Sig = Sig {
    len: 1045,
    hash: 0x1a8d_e3eb_6bb9_5171,
    calls: &[(0x40d, Callee::Any)],
    imports: &[],
};

/// Slot `s` through the table pointer at operand `t` of `f`, with `args`.
fn slot<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, s: u32, args: &[u32]) -> Out {
    let target = crate::runtime::slot_target(g, op(img, f, t), s);
    g.call(target, 0, args)
}

/// Three points around a summoner set up (cdecl, no arguments; a /GS
/// frame). With `B = *G_B`, its yaw `B+0x2258` and the height `H =
/// *G_H`: for each point `i` the records `B+0x1a70 + 0x10 i = B+0x19b0 +
/// 0x20 i` (four words) and `P_i = B+0x1aa0 + 0x10 i` a copy of
/// `B+0x1a60`, then the offset `(x_i, H + 0, z_i, 1)` (`(-16.5, -20)`,
/// `(0, -25)`, `(16.5, -20)`) turned by the yaw (slots 235, 328 with 0,
/// 329, 229; the second point with a matrix of its own) is added to
/// `P_i` (slot 248, `w` 1.0). Then `B+0x2268..` = `(0x43, 0x1c, 0x23)`
/// and the objects `B+0x1430` and `B+0x1560` started ([`start_object_anim`]:
/// part 0xd, 0x13 and 0x1b; scale `K`), each with `+0xd0..+0xdc` the
/// first two points, `+0xc4` the yaw and `+0xd4 = y - K2 + H`. Each call
/// sees the words the earlier ones left on the stack. Returns the yaw's
/// word.
///
/// `magic_0511` 0x10004d80 (1141 bytes; 2 DLLs).
#[allow(clippy::too_many_lines)] // the original's points, one by one
pub fn three_points_around<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let h = g.read::<f32>(w(0x17));
    let b = g.read::<u32>(w(0x1f));
    let yaw = g.read::<u32>(b + 0x2258);
    let (pt, out) = (at(0x14), at(0x24));
    // the offsets: (x, z, the matrix, the four operands of the slot
    // table for slots 235, 328, 329, 229, 248)
    let points = [
        (
            0xc184_0000u32,
            0xc1a0_0000u32,
            at(0x64),
            [0xa6u32, 0xdf, 0x101, 0x11a, 0x132],
        ),
        (
            0,
            0xc1c8_0000,
            at(0xa4),
            [0x1b1, 0x1de, 0x203, 0x21f, 0x237],
        ),
        (
            0x4184_0000,
            0xc1a0_0000,
            at(0x64),
            [0x2bd, 0x2e7, 0x309, 0x322, 0x33a],
        ),
    ];
    for (i, (x, z, m, t)) in (0u32..).zip(points) {
        for o in [0u32, 4, 8, 0xc] {
            let v = g.read::<u32>(b + 0x19b0 + 0x20 * i + o);
            g.write::<u32>(b + 0x1a70 + 0x10 * i + o, v);
        }
        let p = b + 0x1aa0 + 0x10 * i;
        for o in [0u32, 4, 8, 0xc] {
            let v = g.read::<u32>(b + 0x1a60 + o);
            g.write::<u32>(p + o, v);
        }
        if i == 0 {
            g.write::<f32>(pt + 4, add(h, 0.0));
            g.write::<u32>(pt + 0xc, 0x3f80_0000);
        }
        g.write::<u32>(pt, x);
        g.write::<u32>(pt + 8, z);
        slot(g, img, f, t[0], 235, &[m]);
        slot(g, img, f, t[1], 328, &[m, m, 0]);
        slot(g, img, f, t[2], 329, &[m, m, yaw]);
        g.write::<u32>(pt + 0xc, 0x3f80_0000);
        slot(g, img, f, t[3], 229, &[out, m, pt]);
        slot(g, img, f, t[4], 248, &[p, p, out]);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
    }
    for (o, v) in [(0x2268u32, 0x43u32), (0x226c, 0x1c), (0x2270, 0x23)] {
        g.write::<u32>(b + o, v);
    }
    let mut eax = 0;
    for (obj, call, kk, value, p, k2, hh) in [
        (
            b + 0x1430,
            0x38bu32,
            0x34cu32,
            0x13u32,
            b + 0x1aa0,
            0x3d5u32,
            0x3eeu32,
        ),
        (b + 0x1560, 0x405, 0x3a7, 0x1b, b + 0x1ab0, 0x444, 0x45c),
    ] {
        let k = g.read::<f32>(w(kk));
        start_object_anim(g, img, img.rel32_target(f + call), obj, k, 0xd, value);
        for o in [0u32, 4, 8, 0xc] {
            let v = g.read::<u32>(p + o);
            g.write::<u32>(obj + 0xd0 + o, v);
        }
        let y = sub(g.read::<f32>(obj + 0xd4), g.read::<f32>(w(k2)));
        eax = g.read::<u32>(b + 0x2258);
        g.write::<u32>(obj + 0xc4, eax);
        let y = add(y, g.read::<f32>(w(hh)));
        g.write::<f32>(obj + 0xd4, y);
    }
    Ret::eax(eax)
}

/// [`three_points_around`] (the `rel32`s: two [`start_object_anim`]s and
/// `__security_check_cookie`).
pub const THREE_POINTS_AROUND: Sig = Sig {
    len: 1141,
    hash: 0x9969_a176_2a42_c5c0,
    calls: &[
        (0x38b, Callee::Sig(&START_OBJECT_ANIM)),
        (0x405, Callee::Sig(&START_OBJECT_ANIM)),
        (0x46d, Callee::Any),
    ],
    imports: &[],
};

/// A sprite fan's task started (internal, `ecx` = its state `S`):
/// `S+0x104` and `+0x108` 50, `+0x10c` 8, u16 `+0x126` 0; a task node for
/// [`sprite_fan_task`] (the callback immediate `CB`; [`take`], `+0x14 =
/// S`, `+0x4` 0) appended to the task list (`G_HEAD`, `G_TAIL`). Returns
/// the node. The 1204 bytes of its identity hold the task function too
/// (no padding between them).
///
/// `magic_0598` 0x10004530 (112 bytes, `ecx` = the state; 2 DLLs).
pub fn sprite_fan_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = fr.ecx;
    g.write::<u32>(s + 0x108, 0x32);
    g.write::<u32>(s + 0x104, 0x32);
    g.write::<u32>(s + 0x10c, 8);
    g.write::<u16>(s + 0x126, 0);
    let node = take(g, img, img.rel32_target(f + 0x30), w(0x6));
    g.write::<u32>(node.wrapping_add(0x14), s);
    g.write::<u32>(node.wrapping_add(4), 0);
    if g.read::<u32>(w(0x36)) != 0 {
        let tail = g.read::<u32>(w(0x4a));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(w(0x54), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x63), node);
        g.write::<u32>(w(0x68), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`sprite_fan_start`] with [`sprite_fan_task`] after it (the `rel32`s:
/// [`take`], and the task's [`screen_place`] and [`give_back`]).
pub const SPRITE_FAN_START: Sig = Sig {
    len: 1204,
    hash: 0x1413_1a57_8d92_eb25,
    calls: &[
        (0x30, Callee::Sig(&TAKE)),
        (0x15d, Callee::Sig(&SCREEN_PLACE)),
        (0x493, Callee::Sig(&GIVE_BACK)),
    ],
    imports: &[],
};

/// The sprite fan's task (cdecl `(task)`, `task[5]` the state `S`).
/// While `S+0x110` is positive it counts down. Else: the pad bit (slot
/// 245 into a local: bit 0 picks the packet word `0x64120000 | 0xd00`)
/// and the sprite packet's fixed words from `S+0x18` (its colour byte
/// `+0xf3` into `+0x34`); the object placed on screen ([`screen_place`]);
/// with `t = (n - m) / n` (`n = S+0x104`, `m = S+0x108`, `u = K1 - t`)
/// the scale `s = u t K2 S+0xfc + u u S+0xf8 + t t S+0x100`, the corners
/// from the screen point `(p, q)` (`S+0x114`, `S+0x118`): `x0 = p K4 s`,
/// `x1 = K1 - (K3 - p) K4 s`, `y0 = q K6 s K7`, `y1 = (K1 - (K5 - q) K6
/// s) K7` into the packet with the colour bytes `+0xf0..` and the
/// texture words of `*G_C`; the packet sent (slot 821 `(0)`), its colour
/// (slot 822: the four bytes `+0xf0..` times `K8`), its scale (slot 823
/// `(K1 / (x1 - x0), K9 / (y1 - y0))`) and the four floats at `G_F..`
/// (slot 824). Then `+0x108` counts down to 0, then `+0x10c` (each step
/// halving the colour byte `+0xf3`), and at the end u16 `+0x126` gets
/// bit 15 and the task node goes back to the pool ([`give_back`]).
/// Returns the count left, or [`give_back`]'s `eax`.
///
/// `magic_0598` 0x100045a0 (1140 bytes; 2 DLLs).
#[allow(clippy::too_many_lines)] // the original's packet, word by word
pub fn sprite_fan_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 0x14);
    let n = g.read::<i32>(s + 0x110);
    if n > 0 {
        g.write::<i32>(s + 0x110, n - 1);
        return Ret::eax((n - 1).cast_unsigned());
    }
    let ebp = fr.esp.wrapping_sub(4);
    let pad = ebp.wrapping_sub(4);
    slot(g, img, f, 0x2a, 245, &[pad]);
    let bit = if g.read::<u8>(pad) & 1 != 0 { 0xd00 } else { 0 };
    for (o, v) in [
        (0x18u32, 0xeu32),
        (0x1c, 0),
        (0x20, 0),
        (0x24, 0),
        (0x28, 0x47),
        (0x2c, 0),
    ] {
        g.write::<u32>(s + o, v);
    }
    let colour = u32::from(g.read::<u8>(s + 0xf3));
    g.write::<u32>(s + 0x34, colour);
    for (o, v) in [
        (0x38u32, 0x42u32),
        (0x3c, 0),
        (0x30, 0x64),
        (0x40, 0),
        (0x44, 0),
        (0x48, 8),
        (0x4c, 0),
        (0x58, 6),
        (0x5c, 0),
        (0x50, bit | 0x6412_0000),
        (0x54, 6),
        (0x60, 0x8080),
        (0x64, 0x80),
        (0x68, 0x3b),
        (0x6c, 0),
    ] {
        g.write::<u32>(s + o, v);
    }
    let place = Frame {
        at: img.rel32_target(f + 0xed),
        esp: ebp.wrapping_sub(0x28),
        ecx: s,
        ..*fr
    };
    screen_place(g, img, &place);
    // the scale from the count
    let total = g.read::<i32>(s + 0x104);
    let left = total.wrapping_sub(g.read::<i32>(s + 0x108));
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let t = div(left as f32, total as f32);
    let one = k(g, 0x111);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let (p, q) = (
        g.read::<i32>(s + 0x114) as f32,
        g.read::<i32>(s + 0x118) as f32,
    );
    let u = sub(one, t);
    let ends = mul(mul(u, u), g.read::<f32>(s + 0xf8));
    let mid = mul(mul(mul(u, t), k(g, 0x142)), g.read::<f32>(s + 0xfc));
    let tail = mul(mul(t, t), g.read::<f32>(s + 0x100));
    let scale = add(add(mid, ends), tail);
    // the corners: x0 at `[ebp-0x14]`, x1 `-0x10`, y0 `-0xc`, y1 `-0x8`
    let x0 = mul(mul(p, k(g, 0x179)), scale);
    let x1 = sub(one, mul(mul(sub(k(g, 0x162), p), k(g, 0x185)), scale));
    let y0 = mul(mul(mul(q, k(g, 0x1dc)), scale), k(g, 0x1f4));
    let y1 = mul(
        sub(
            k(g, 0x1b1),
            mul(mul(sub(k(g, 0x1a9), q), k(g, 0x1fc)), scale),
        ),
        k(g, 0x284),
    );
    for (o, v) in [(0x14u32, x0), (0x10, x1), (0xc, y0), (8, y1)] {
        g.write::<f32>(ebp.wrapping_sub(o), v);
    }
    let c = g.read::<u32>(w(0x1e2));
    g.write::<u32>(s + 0x78, 0x00e5_2502);
    g.write::<u32>(s + 0x7c, 0);
    g.write::<f32>(s + 0x80, x0);
    g.write::<u32>(s + 0x88, 0x3f80_0000);
    g.write::<f32>(s + 0x84, y0);
    for i in 0..3u32 {
        let b = u32::from(g.read::<u8>(s + 0xf0 + i));
        g.write::<u32>(s + 0x90 + 4 * i, b);
    }
    for (o, v) in [
        (0x9cu32, 0x80u32),
        (0xa0, 0x6ff8),
        (0xa4, 0x72f8),
        (0xa8, 0),
        (0xac, 0),
    ] {
        g.write::<u32>(s + o, v);
    }
    g.write::<f32>(s + 0xb0, x1);
    g.write::<u32>(s + 0xb8, 0x3f80_0000);
    g.write::<f32>(s + 0xb4, y1);
    for (o, v) in [(0xc0u32, 0x8ff8u32), (0xc4, 0x8cf8), (0xc8, 0), (0xcc, 0)] {
        g.write::<u32>(s + o, v);
    }
    let tex = [0x58u32, 0x5c].map(|o| g.read::<u32>(c.wrapping_add(o)));
    g.write::<u32>(s + 0xd0, tex[0]);
    g.write::<u32>(s + 0xd4, tex[1]);
    g.write::<u32>(s + 0xd8, 0x47);
    g.write::<u32>(s + 0xdc, 0);
    slot(g, img, f, 0x2cb, 821, &[0]);
    let k8 = k(g, 0x2fb);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps of a byte: exact
    let cols = [0u32, 1, 2, 3].map(|i| mul(f32::from(g.read::<u8>(s + 0xf0 + i)), k8).to_bits());
    slot(g, img, f, 0x34f, 822, &cols);
    let sy = div(k(g, 0x375), sub(y1, y0));
    let sx = div(k(g, 0x394), sub(x1, x0));
    slot(g, img, f, 0x399, 823, &[sx.to_bits(), sy.to_bits()]);
    let fl = [0x3e9u32, 0x3d8, 0x3c5, 0x3b2].map(|o| g.read::<u32>(w(o)));
    slot(g, img, f, 0x3b7, 824, &fl);
    let m = g.read::<i32>(s + 0x108);
    if m > 0 {
        g.write::<i32>(s + 0x108, m - 1);
        return Ret::eax((m - 1).cast_unsigned());
    }
    let r = g.read::<i32>(s + 0x10c);
    let half = g.read::<u8>(s + 0xf3) >> 1;
    g.write::<u8>(s + 0xf3, half);
    if r > 0 {
        g.write::<i32>(s + 0x10c, r - 1);
        return Ret::eax((r - 1).cast_unsigned());
    }
    let fl = g.read::<u16>(s + 0x126) | 0x8000;
    g.write::<u16>(s + 0x126, fl);
    Ret::eax(give_back(g, img, img.rel32_target(f + 0x423), task))
}

/// [`sprite_fan_task`] (the `rel32`s: [`screen_place`] and [`give_back`]).
pub const SPRITE_FAN_TASK: Sig = Sig {
    len: 1140,
    hash: 0xa98e_636c_7f0a_a1ef,
    calls: &[
        (0xed, Callee::Sig(&SCREEN_PLACE)),
        (0x423, Callee::Sig(&GIVE_BACK)),
    ],
    imports: &[],
};

/// A part started and given its motion (internal: `ecx` = the part,
/// `edx` = the program index, `xmm2`/`xmm3` two floats, the stack `(a,
/// b, c, d, e, f, h)` floats): `*G_P = ITEM`, the program `index` of slot
/// 941 `(*G_D)` started on the part ([`start_part`], 0x1000, 1); u16
/// `+0x1e = 1`, `+0x40 = xmm2`, `+0x44 = xmm3`, `+0x48 = a`, `+0x4c =
/// 1.0`, `+0x50..+0x58` the stack's `b`, `c`, `d` truncated
/// (`cvttss2si`), `+0x5c = 1`, `+0x60..+0x68 = e, f, h`, `+0x6c = 1.0`.
/// Returns `d` truncated.
///
/// `magic_0341` 0x10001000 (194 bytes; 8 DLLs).
pub fn part_motion<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (part, index) = (fr.ecx, fr.edx);
    g.write::<u32>(g.read::<u32>(w(0x7)), w(0xe));
    let d = g.read::<u32>(w(0x1a));
    let table = slot(g, img, f, 0x13, 941, &[d]).eax;
    let program = g.read::<u32>(table.wrapping_add(4 * index));
    start_part(g, img, img.rel32_target(f + 0x43), part, 0x1000, program, 1);
    let arg = |g: &G, i: u32| g.read::<u32>(fr.esp + 4 + 4 * i);
    let int = |g: &G, i: u32| trunc(f32::from_bits(arg(g, i)));
    g.write::<u16>(part + 0x1e, 1);
    g.write::<u32>(part + 0x40, fr.xmm[2] as u32);
    g.write::<u32>(part + 0x44, fr.xmm[3] as u32);
    let a = arg(g, 0);
    g.write::<u32>(part + 0x48, a);
    let b = int(g, 1);
    g.write::<u32>(part + 0x50, b);
    let c = int(g, 2);
    g.write::<u32>(part + 0x54, c);
    let dd = int(g, 3);
    for (o, i) in [(0x60u32, 4u32), (0x64, 5)] {
        let v = arg(g, i);
        g.write::<u32>(part + o, v);
    }
    g.write::<u32>(part + 0x4c, 0x3f80_0000);
    g.write::<u32>(part + 0x58, dd);
    g.write::<u32>(part + 0x5c, 1);
    let h = arg(g, 6);
    g.write::<u32>(part + 0x68, h);
    g.write::<u32>(part + 0x6c, 0x3f80_0000);
    dd
}

/// [`part_motion`] (the `rel32`: [`start_part`]).
pub const PART_MOTION: Sig = Sig {
    len: 194,
    hash: 0x387c_e7e4_e1d2_b21a,
    calls: &[(0x43, Callee::Sig(&START_PART))],
    imports: &[],
};

/// One step of a model's motion sequence (internal: `ecx` = the state
/// `S` (u16 step `+0`, u16 time `+2`), `edx` = the sequence `Q` (`i16`
/// count `+6`, the steps' records from `+0x10`, 8 bytes each), the stack
/// `(m)`, the model record). The handle is `S` as a 64-bit number (`(S,
/// S >> 31)`); every call runs with the game lock `*G_LOCK` set to 1 and
/// then restored. Past the last step: slot 779 `(handle)`, returns 0.
/// Else, for the step's strip `R = Q + rec[0]`: when slot 773
/// `(handle)` says the model is loaded, slot 774 `(handle, &q, m,
/// step)` (`q` a local holding `Q`) and slot 775 `(handle)`; otherwise
/// the flags of slot 776 `(R's byte, or 0x48 for 0)` or slot 777
/// `(m+0x1c)` go with slot 778 `(handle, &q, m, step, flags)`. Then the
/// time goes up by 0x100, and past the record's `i16` length (`rec+2`)
/// wraps into the next step. Returns 1.
///
/// `magic_0070` 0x100015b0 (399 bytes; 8 DLLs).
pub fn motion_sequence_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (s, q) = (fr.ecx, fr.edx);
    let m = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let local = ebp.wrapping_sub(8);
    let (lo, hi) = (s, (s.cast_signed() >> 31).cast_unsigned());
    let count = i32::from(g.read::<i16>(q + 6));
    let step = u32::from(g.read::<u16>(s));
    // a call with the game lock held: the lock word is read through its
    // operand before (`at`) and after (`back`) the call
    let locked = |g: &mut G, (at, back): (u32, u32), t: u32, n: u32, args: &[u32]| {
        let lock = g.read::<u32>(w(at));
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        let eax = slot(g, img, f, t, n, args).eax;
        g.write::<u32>(g.read::<u32>(w(back)), saved);
        eax
    };
    if step.cast_signed() >= count {
        locked(g, (0x24, 0x3f), 0x2a, 779, &[lo, hi]);
        return 0;
    }
    g.write::<u32>(local, q);
    let rec = q.wrapping_add(8 * (step + 2));
    let strip = i32::from(g.read::<i16>(rec))
        .cast_unsigned()
        .wrapping_add(q);
    let lock = g.read::<u32>(w(0x62));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let loaded = slot(g, img, f, 0x74, 773, &[lo, hi]).eax;
    let lock = g.read::<u32>(w(0x86));
    g.write::<u32>(lock, saved);
    if loaded != 0 {
        let step = u32::from(g.read::<u16>(s));
        g.write::<u32>(lock, 1);
        slot(g, img, f, 0xa5, 774, &[lo, hi, local, m, step]);
        g.write::<u32>(g.read::<u32>(w(0xba)), 1);
        slot(g, img, f, 0xc8, 775, &[lo, hi]);
        g.write::<u32>(g.read::<u32>(w(0xd7)), saved);
    } else {
        let at = g.read::<u32>(strip.wrapping_add(4)).wrapping_add(strip);
        let b = g.read::<u8>(at.wrapping_add(4));
        let x = if b == 0 { 0x48 } else { u32::from(b) };
        let a = slot(g, img, f, 0xfa, 776, &[x]).eax;
        let c = g.read::<u32>(m.wrapping_add(0x1c));
        let flags = a | slot(g, img, f, 0x10c, 777, &[c]).eax;
        let step = u32::from(g.read::<u16>(s));
        locked(
            g,
            (0x124, 0x154),
            0x143,
            778,
            &[lo, hi, local, m, step, flags],
        );
    }
    let t = g.read::<u16>(s + 2).wrapping_add(0x100);
    g.write::<u16>(s + 2, t);
    let len = g.read::<u16>(rec + 2);
    if i32::from(t) >= i32::from(len.cast_signed()) {
        let n = g.read::<u16>(s).wrapping_add(1);
        g.write::<u16>(s, n);
        g.write::<u16>(s + 2, t.wrapping_sub(len));
    }
    1
}

/// [`motion_sequence_step`].
pub const MOTION_SEQUENCE_STEP: Sig = Sig {
    len: 399,
    hash: 0xc306_cea3_716a_2f59,
    calls: &[],
    imports: &[],
};

/// Starts an effect object with its animation block reset from two
/// stack words (internal: `edx` = the object, `xmm3` its scale, the
/// stack `(index, -, a, b)`): [`start_object`] with [`anim_reset_2`]
/// (`+0x11c = a`, `+0x100 = b`), the two words pushed for it below the
/// start's three. Returns the object.
///
/// `magic_0102` 0x10002140 (237 bytes; 7 DLLs).
pub fn start_object_anim_2<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let obj = fr.edx;
    let (a, b) = (fr.arg(g, 2), fr.arg(g, 3));
    let index = fr.arg(g, 0);
    // push ebp, esi, the slot's word and the start's three, then the two
    // words: the reset's frame
    let esp = fr.esp.wrapping_sub(0x24);
    let reset = |g: &mut G| {
        g.write::<u32>(esp + 4, a);
        g.write::<u32>(esp + 8, b);
        anim_reset_2(g, esp, obj);
    };
    let list = [0xa4, 0xb7, 0xbd, 0xc6, 0xd2, 0xdb, 0xe4];
    start_object(
        g,
        img,
        f,
        obj,
        fr.xmm_f32(3),
        index,
        reset,
        (0x84, 0x99, list),
    )
}

/// [`start_object_anim_2`] (the `rel32`s: [`start_part`],
/// [`anim_reset_2`] and [`pool_alloc`](crate::l5_objhost::pool_alloc)).
pub const START_OBJECT_ANIM_2: Sig = Sig {
    len: 237,
    hash: 0x3418_41fd_bb67_fb6e,
    calls: &[
        (0x84, Callee::Sig(&START_PART)),
        (0x91, Callee::Sig(&ANIM_RESET_2)),
        (0x99, Callee::Sig(&POOL_ALLOC)),
    ],
    imports: &[],
};

/// Every object on the DLL's object list freed (cdecl, no arguments; the
/// list `L` an immediate: head, tail, cursor, count). The cursor starts
/// at the head; for each object [`list_cur`] gives: `*G_P = ITEM`, slot
/// 713 `(obj)` frees it, bit 15 of its `+0x124` is cleared, and the node
/// holding it leaves the list as the overlays inline it (head, tail or
/// middle; the cursor steps past it; the free count `*G_FREE` up, `L`'s
/// count down). The cursor steps once more when it still shows the same
/// object, and the next object is the cursor's. Returns the number of
/// objects.
///
/// `magic_0430` 0x10007f70 (324 bytes; 7 DLLs).
pub fn objects_freed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let list = w(0x8);
    let head = g.read::<u32>(w(0x1));
    g.write::<u32>(w(0xd), head);
    let mut n = 0u32;
    let mut obj = list_cur(g, list);
    while obj != 0 {
        g.write::<u32>(g.read::<u32>(w(0x31)), w(0x38));
        slot(g, img, f, 0x3d, 713, &[obj]);
        let v = g.read::<u16>(obj + 0x124) & 0x7fff;
        g.write::<u16>(obj + 0x124, v);
        let head = g.read::<u32>(w(0x52));
        let mut node = head;
        while node != 0 && g.read::<u32>(node + 8) != obj {
            node = g.read::<u32>(node + 4);
        }
        if node != 0 {
            // the head or the tail: relinked, then the cursor checked
            let end = if node == head {
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(w(0x7c), next);
                if next == 0 {
                    g.write::<u32>(w(0x85), 0);
                } else {
                    g.write::<u32>(next, 0);
                }
                true
            } else if node == g.read::<u32>(w(0xad)) {
                let prev = g.read::<u32>(node);
                g.write::<u32>(w(0xbc), prev);
                if prev == 0 {
                    g.write::<u32>(w(0xc5), 0);
                } else {
                    g.write::<u32>(prev + 4, 0);
                }
                true
            } else {
                false
            };
            if end {
                let cur = g.read::<u32>(w(0x92));
                if cur == node {
                    let next = g.read::<u32>(cur + 4);
                    g.write::<u32>(w(0x9e), next);
                }
            } else {
                let cur = g.read::<u32>(w(0xd6));
                if cur == node {
                    let next = g.read::<u32>(cur + 4);
                    g.write::<u32>(w(0xe3), next);
                }
                let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node + 4));
                g.write::<u32>(prev.wrapping_add(4), next);
                let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node));
                g.write::<u32>(next, prev);
            }
            g.write::<u32>(node + 8, 0);
            let free = g.read::<u16>(w(0x100)).wrapping_add(1);
            g.write::<u16>(w(0x100), free);
            let count = g.read::<u32>(w(0x106)).wrapping_sub(1);
            g.write::<u32>(w(0x106), count);
        }
        n += 1;
        if list_cur(g, list) == obj {
            let cur = g.read::<u32>(w(0x11a));
            if cur != 0 {
                let next = g.read::<u32>(cur + 4);
                g.write::<u32>(w(0x126), next);
            }
        }
        obj = list_cur(g, list);
    }
    Ret::eax(n)
}

/// [`objects_freed`] (the `rel32`s: three [`list_cur`]s; the `jmp rel32`
/// inside stays in the hash).
pub const OBJECTS_FREED: Sig = Sig {
    len: 324,
    hash: 0x1086_f60c_242b_8368,
    calls: &[
        (0x14, Callee::Sig(&LIST_CUR)),
        (0x111, Callee::Sig(&LIST_CUR)),
        (0x130, Callee::Sig(&LIST_CUR)),
    ],
    imports: &[],
};

/// A sprite system's step (internal, `ecx` = the system: count `+0`,
/// the sprites `+4` (0x94 bytes each), gravity on `+8` and its step
/// `+0xc`, a speed cap on `+0x10` and its limit `+0x14`). For each live
/// sprite (word `+0` set): its fade ([`sprite_fade`]); one that ended is
/// released (slot 941 `(+0x30)`'s table and the sprite's index to slot
/// 772, the 64-bit handle to slot 779 with the game lock held) and
/// cleared (0x94 bytes). Otherwise gravity adds to `+0x18`, and with the
/// cap the velocity `v = +0x14..+0x1c` of length `l` (not 0) becomes `v
/// (l - limit) / l` when `l` is at least the limit (or unordered), else
/// `v 0 / l`. Returns the `eax` the original leaves: the entry's with no
/// sprite, the fade's or the cleared sprite's address; after the square
/// root it is the CRT's with the flags in `ah` (junk: the port gives 0).
///
/// `magic_0370` 0x10002d90 (368 bytes, `ecx` = the system; 7 DLLs).
pub fn sprites_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let sys = fr.ecx;
    let mut eax = fr.eax;
    let mut p = g.read::<u32>(sys + 4);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(sys) {
        if g.read::<u32>(p) != 0 {
            let fade = sprite_fade(g, img, img.rel32_target(f + 0x2c), p);
            eax = fade;
            if fade == 0 {
                let d = g.read::<u32>(p + 0x30);
                let table = slot(g, img, f, 0x35, 941, &[d]).eax;
                let h = slot(g, img, f, 0x4a, 772, &[table, i]);
                let lock = g.read::<u32>(w(0x58));
                let saved = g.read::<u32>(lock);
                g.write::<u32>(lock, 1);
                slot(g, img, f, 0x61, 779, &[h.eax, h.edx]);
                g.write::<u32>(g.read::<u32>(w(0x74)), saved);
                g.write_bytes(p, &[0u8; 0x94]);
                eax = p;
                g.write::<u32>(p, 0);
            } else {
                if g.read::<u32>(sys + 8) != 0 {
                    let y = add(g.read::<f32>(sys + 0xc), g.read::<f32>(p + 0x18));
                    g.write::<f32>(p + 0x18, y);
                }
                if g.read::<u32>(sys + 0x10) != 0 {
                    let v = [0x14u32, 0x18, 0x1c].map(|o| g.read::<f32>(p + o));
                    let s = add(add(mul(v[0], v[0]), mul(v[1], v[1])), mul(v[2], v[2]));
                    let l = f64::from(s).sqrt() as f32;
                    // eax: the CRT's, with lahf's flags (junk; 0 here)
                    eax = 0;
                    // ucomiss; lahf; test ah, 0x44; jnp: skipped when 0
                    #[allow(clippy::float_cmp)] // the original's exact test
                    if l != 0.0 {
                        let d = sub(l, g.read::<f32>(sys + 0x14));
                        // comiss 0, d; jbe: the excess kept when 0 <= d or
                        // unordered, else 0
                        let top = if 0.0 > d { 0.0 } else { d };
                        let k = div(top, l);
                        for (o, x) in [0x14u32, 0x18, 0x1c].into_iter().zip(v) {
                            g.write::<f32>(p + o, mul(x, k));
                        }
                    }
                }
            }
        }
        i += 1;
        p = p.wrapping_add(0x94);
    }
    eax
}

/// [`sprites_step`] (the `rel32`s: [`sprite_fade`], `memset` and the
/// square root; the `jmp rel32` inside stays in the hash).
pub const SPRITES_STEP: Sig = Sig {
    len: 368,
    hash: 0x2919_7cda_4268_3f62,
    calls: &[
        (0x2c, Callee::Sig(&SPRITE_FADE)),
        (0x83, Callee::Import("memset")),
        (0xfe, Callee::Import("_libm_sse2_sqrt_precise")),
    ],
    imports: &[],
};

/// A screen tint by a countdown (cdecl, no arguments): once (`*G_ON`)
/// slot 628 `(1)`; slot 821 `(1)`; with `v = 10 - *G_T`, slot 915 `(0,
/// 0, 0, (v * 64 / 10) K, 0x44)` and slot 802 `(v * 128 / 10)` (each
/// division truncating, as the original's multiply by 0x66666667 does),
/// the second seeing the first's words. Returns slot 802's `eax`.
///
/// `magic_0393` 0x100013e0 (197 bytes; 6 DLLs).
pub fn countdown_tint<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    if g.read::<u32>(w(0x2)) == 0 {
        slot(g, img, f, 0xa, 628, &[1]);
        g.write::<u32>(w(0x1d), 1);
    }
    slot(g, img, f, 0x26, 821, &[1]);
    let v = |g: &G, o: u32| 10i32.wrapping_sub(g.read::<i32>(w(o)));
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let x = mul(
        (v(g, 0x3e).wrapping_shl(6) / 10) as f32,
        g.read::<f32>(w(0x70)),
    );
    slot(g, img, f, 0x4e, 915, &[0, 0, 0, x.to_bits(), 0x44]);
    let b = (v(g, 0x9a).wrapping_shl(7) / 10).cast_unsigned();
    let eax = slot(g, img, f, 0xb4, 802, &[b, 0, 0, 0, x.to_bits(), 0x44]).eax;
    Ret::eax(eax)
}

/// [`countdown_tint`].
pub const COUNTDOWN_TINT: Sig = Sig {
    len: 197,
    hash: 0x39be_2d11_c49d_81af,
    calls: &[],
    imports: &[],
};

/// A timed actor effect's task (cdecl `(task)`, `task[5]` the state `S`:
/// the point `+0..+8`, its second point `+0xc..+0x14`, a word `+0x18`,
/// three waits `+0x1c`, `+0x24`, `+0x2c`, the actor `+0x28`, a word
/// `+0x20`). The first wait counts down; at 0 slot 256 `(the actor's
/// object (*G_OWN for -1, else the table entry of the selection's
/// byte), the point, +0x18)` runs; then the second counts down; at 0
/// [`actor_slot256`] `(actor, +0x20, the second point)` and the third
/// wait is `+0x20`; then the third counts down, and at the end bit 15 of
/// u16 `+0x30` is set and the task node goes back to the pool (a tail
/// jump to [`give_back`]). Returns the count left or the last call's
/// `eax`.
///
/// `magic_0053` 0x10002b60 (201 bytes; 6 DLLs).
pub fn actor_timed_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 0x14);
    let first = g.read::<i32>(s + 0x1c);
    if first > 0 {
        g.write::<i32>(s + 0x1c, first - 1);
        return Ret::eax((first - 1).cast_unsigned());
    }
    if first == 0 {
        let id = g.read::<u32>(s + 0x28);
        let p = [0u32, 4, 8].map(|o| g.read::<u32>(s + o));
        let ch = if id == u32::MAX {
            g.read::<u32>(w(0x31))
        } else {
            let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x38)).wrapping_add(id)));
            g.read::<u32>(w(0x43).wrapping_add(4 * b))
        };
        let x = g.read::<u32>(s + 0x18);
        let eax = slot(g, img, f, 0x60, 256, &[ch, p[0], p[1], p[2], x]).eax;
        let n = g.read::<u32>(s + 0x1c).wrapping_sub(1);
        g.write::<u32>(s + 0x1c, n);
        return Ret::eax(eax);
    }
    let second = g.read::<i32>(s + 0x24);
    if second > 0 {
        g.write::<i32>(s + 0x24, second - 1);
        return Ret::eax((second - 1).cast_unsigned());
    }
    if second == 0 {
        let v = [0x14u32, 0x10, 0xc].map(|o| u128::from(g.read::<u32>(s + o)));
        let call = Frame {
            at: img.rel32_target(f + 0x9b),
            esp: fr.esp.wrapping_sub(0xc),
            edx: g.read::<u32>(s + 0x20),
            ecx: g.read::<u32>(s + 0x28),
            xmm: [fr.xmm[0], v[2], v[1], v[0]],
            ..*fr
        };
        actor_slot256(g, img, &call);
        let x = g.read::<u32>(s + 0x20);
        let n = g.read::<u32>(s + 0x24).wrapping_sub(1);
        g.write::<u32>(s + 0x24, n);
        g.write::<u32>(s + 0x2c, x);
        return Ret::eax(x);
    }
    let third = g.read::<i32>(s + 0x2c);
    if third > 0 {
        g.write::<i32>(s + 0x2c, third - 1);
        return Ret::eax((third - 1).cast_unsigned());
    }
    let fl = g.read::<u16>(s + 0x30) | 0x8000;
    g.write::<u16>(s + 0x30, fl);
    // jmp give_back: the caller's stack, `(task)`
    Ret::eax(give_back(g, img, img.rel32_target(f + 0xc5), task))
}

/// [`actor_timed_task`] (the `rel32`s: [`actor_slot256`] and the tail
/// jump to [`give_back`]).
pub const ACTOR_TIMED_TASK: Sig = Sig {
    len: 201,
    hash: 0xf965_5647_22bd_8e52,
    calls: &[
        (0x9b, Callee::Sig(&ACTOR_SLOT256)),
        (0xc5, Callee::Sig(&GIVE_BACK)),
    ],
    imports: &[],
};

/// [`targets_mean_offset`](crate::s5_misc::targets_mean_offset) with the
/// point in `edx` (internal: `ecx` = `out`, `edx` = the point; a /GS
/// frame): `v` = a copy of the point (slot 325, a local), and for each
/// of the `*G_N` targets `G_IDS[i]` (bytes) that slot 684 says is live,
/// its battle position (slot 531, a local) minus `v` is summed; `out = v
/// + sum * (K / n)` (`n` the live count), `w = 1`. Returns the last
/// slot's `eax`.
///
/// `magic_0106` 0x100069b0 (352 bytes, `ecx` = `out`, `edx` = the point;
/// 6 DLLs).
pub fn targets_mean_offset_of<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let out = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let p = ebp.wrapping_sub(0x24);
    let mut eax = slot(g, img, f, 0x19, 325, &[v, fr.edx]).eax;
    let (mut sx, mut sy, mut sz) = (0.0f32, 0.0f32, 0.0f32);
    let mut n = 0i32;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x48)) {
        let id = |g: &G, o: u32| u32::from(g.read::<u8>(w(o).wrapping_add(i)));
        let t = id(g, 0x55);
        eax = slot(g, img, f, 0x5b, 684, &[t]).eax;
        if eax != 0 {
            let t = id(g, 0x75);
            eax = slot(g, img, f, 0x7b, 531, &[t, p]).eax;
            let dy = sub(g.read::<f32>(p + 4), g.read::<f32>(v + 4));
            let dz = sub(g.read::<f32>(p + 8), g.read::<f32>(v + 8));
            let dx = sub(g.read::<f32>(p), g.read::<f32>(v));
            n += 1;
            g.write::<f32>(p + 4, dy);
            sy = add(dy, sy);
            g.write::<f32>(p + 8, dz);
            sz = add(dz, sz);
            g.write::<f32>(p, dx);
            sx = add(dx, sx);
        }
        i += 1;
    }
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let k = div(g.read::<f32>(w(0xfe)), n as f32);
    let [vx, vy, vz] = [0u32, 4, 8].map(|o| g.read::<f32>(v + o));
    g.write::<f32>(out, add(vx, mul(k, sx)));
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    g.write::<f32>(out + 4, add(vy, mul(k, sy)));
    g.write::<f32>(out + 8, add(vz, mul(k, sz)));
    eax
}

/// [`targets_mean_offset_of`] (the `rel32`: `__security_check_cookie`).
pub const TARGETS_MEAN_OFFSET_OF: Sig = Sig {
    len: 352,
    hash: 0x24ce_1d62_2215_2740,
    calls: &[(0x158, Callee::Any)],
    imports: &[],
};
