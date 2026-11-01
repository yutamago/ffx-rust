//! Shared functions of the shared-6 list (rows 843 on of the shared-4
//! work list) without a family module yet. Each doc names the DLL and
//! address it was ported from, its size and how many DLLs carry it;
//! "`G`" is a global of the DLL (an operand of the code).

use crate::l5_objhost::{LIST_CUR, LIST_REMOVE, POOL_ALLOC, list_cur, list_remove};
use crate::l5_objhost::{PART_INIT, part_init};
use crate::m0032::{POP_NODE, RESET_STATE, pop_node, reset_state};
use crate::nodes::{INSERT, node_insert};
use crate::opu_parts::{START_PART, start_part};
use crate::pushes::Pushes;
use crate::s3_misc::{COLOUR_NODE, colour_node};
use crate::s3_summon::{MODEL_NODES, model_nodes};
use crate::s4_misc::{FREE_RECORD, TRAIL_DRAW, append_object, free_record, trail_draw};
use crate::s4_misc::{MOTION_SCRIPT, motion_script};
use crate::s4_misc::{
    SCREEN_PLACE, START_OBJECT_ANIM, screen_place, start_object, start_object_anim,
};
use crate::s4_small::{FADER_TASK, MOVE_ENDS, fader_task, move_ends};
use crate::s4_small::{THREE_ACTORS_SHOWN, three_actors_shown};
use crate::s4_summon::trunc;
use crate::s5_misc::list_push;
use crate::sa_misc::{
    LINK_PART, RING_RESERVE, RING_TAKE_68, SPRITE_FADE, link_part, ring_reserve, ring_take_68,
    sprite_fade,
};
use crate::sa_pool::{GIVE_BACK, TAKE, give_back, take};
use crate::sa_script::{ACTOR_SLOT256, RUN, actor_slot256, run};
use crate::sa_summon::{BONE, bone_plain};
use crate::sb_misc::{
    ANIM_RESET_2, MOVE_START, MOVE_START_2, anim_reset_2, move_start, move_start_2,
};
use crate::sb_misc::{
    EASE_ANGLE, EASE_SAMPLE, MEAN_POSITION, ease_angle, ease_sample, mean_position,
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

/// [`staged_nodes_task`]'s copy in `magic_0426` 0x10007130 and `magic_0404`
/// 0x10006960 (another colour immediate).
pub const STAGED_NODES_TASK_B: Sig = Sig {
    hash: 0x60a4_0c9f_7a13_2d1a,
    ..STAGED_NODES_TASK
};

/// [`staged_nodes_task`]'s copy in `magic_0400` 0x10006960 (another
/// colour immediate; an own row of that DLL).
pub const STAGED_NODES_TASK_C: Sig = Sig {
    hash: 0x1d43_486b_415a_0b57,
    ..STAGED_NODES_TASK
};

/// [`staged_nodes_task`]'s copy in `magic_0428` 0x10006960 (another
/// colour immediate; an own row of that DLL).
pub const STAGED_NODES_TASK_D: Sig = Sig {
    hash: 0x0700_3223_8f08_ced0,
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
/// its battle position (slot 531, a local) minus `v` is summed; then
/// `out = v + sum * (K / n)` (`n` the live count), `w = 1`. Returns the
/// last slot's `eax`.
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

/// A spring node's walk step (internal, `ecx` = the walk cursor `C`:
/// `+0` the node, `+4` the "moved" byte, `+5` the pass byte; a /GS
/// frame). A node not yet in this pass walks its children first (each
/// child's method `+0x1c` with `C` pointing at it, the next child read
/// before the call), then takes the pass and sets "moved". When moved:
/// the node `o`'s parent `P = o+0x3c` walks with a local cursor (its
/// method `+0x1c`, the pass byte kept), then `o+0x48 += o+0x4c`, `o+0x44
/// += o+0x48`, `o+0x50 += K`, `o+0x40 = o+0x44² o+0x50` (1.0 from `K`
/// on), the offset `P+0x1c - o+0x2c` (slot 234, a local) scaled by
/// `o+0x40` (slot 233) and `o+0x1c.. = o+0x2c.. + it`. Returns slot
/// 233's `eax`, or the frame's cookie word with the node's pass byte in
/// `al` when the node was already in the pass.
///
/// `magic_0070` 0x10005ac0 (319 bytes, `ecx` = the cursor; 5 DLLs).
pub fn spring_walk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let c = fr.ecx;
    let n = g.read::<u32>(c);
    let pass = g.read::<u8>(c + 5);
    let mine = g.read::<u8>(n + 0x1a);
    // eax: the frame's cookie (`*G_COOKIE ^ ebp`) with the byte in `al`
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(op(img, f, 0x7)) ^ ebp;
    let eax = (cookie & !0xff) | u32::from(mine);
    if mine == pass {
        g.write::<u8>(c + 4, 0);
    } else {
        let mut e = g.read::<u32>(n + 0x10);
        while e != n {
            g.write::<u32>(c, e);
            let next = g.read::<u32>(e + 8);
            let m = g.read::<u32>(g.read::<u32>(g.read::<u32>(c)).wrapping_add(0x1c));
            g.call(m, c, &[c]);
            e = next;
        }
        let pass = g.read::<u8>(c + 5);
        g.write::<u32>(c, n);
        g.write::<u8>(n + 0x1a, pass);
        g.write::<u8>(c + 4, 1);
    }
    if g.read::<u8>(c + 4) & 1 == 0 {
        return eax;
    }
    let o = g.read::<u32>(c);
    let pass = g.read::<u8>(c + 5);
    let (local, v) = (ebp.wrapping_sub(0x1c), ebp.wrapping_sub(0x14));
    let parent = g.read::<u32>(o + 0x3c);
    g.write::<u32>(local, parent);
    g.write::<u8>(local + 5, pass);
    let m = g.read::<u32>(g.read::<u32>(parent).wrapping_add(0x1c));
    g.call(m, local, &[local]);
    let k = g.read::<f32>(op(img, f, 0x8d));
    let x0 = add(g.read::<f32>(o + 0x4c), g.read::<f32>(o + 0x48));
    let x2 = add(g.read::<f32>(o + 0x50), k);
    let x1 = add(g.read::<f32>(o + 0x44), x0);
    g.write::<f32>(o + 0x48, x0);
    g.write::<f32>(o + 0x50, x2);
    g.write::<f32>(o + 0x44, x1);
    let s = mul(mul(x1, x1), x2);
    g.write::<f32>(o + 0x40, s);
    // comiss s, K; jb: kept below `K` or unordered
    if !(s < g.read::<f32>(op(img, f, 0xb2)) || s.is_nan()) {
        g.write::<u32>(o + 0x40, 0x3f80_0000);
    }
    let base = o.wrapping_add(0x2c);
    let to = g.read::<u32>(o + 0x3c).wrapping_add(0x1c);
    slot(g, img, f, 0xd4, 234, &[v, to, base]);
    let scale = g.read::<u32>(o + 0x40);
    let eax = slot(g, img, f, 0xf3, 233, &[v, v, scale]).eax;
    for i in 0..3u32 {
        let x = add(g.read::<f32>(base + 4 * i), g.read::<f32>(v + 4 * i));
        g.write::<f32>(o + 0x1c + 4 * i, x);
    }
    eax
}

/// [`spring_walk`] (the `rel32`: `__security_check_cookie`).
pub const SPRING_WALK: Sig = Sig {
    len: 319,
    hash: 0x17cc_493a_ae93_ba16,
    calls: &[(0x137, Callee::Any)],
    imports: &[],
};

/// The 12-byte register thunk ([`crate::s3_wrap`]'s shape: `ecx` = the
/// first stack word, then a jump to the handler with the caller's stack)
/// over this list's handlers: [`spring_walk`]. A target that is none of
/// them cannot bind (the thunk's `Sig` names its callee).
pub fn ecx_jmp<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.arg(g, 0);
    let target = img.rel32_target(fr.at + 8);
    let fr = Frame {
        at: target,
        ecx,
        ..*fr
    };
    if img.matches(target, &SPRING_WALK) {
        return Ret::eax(spring_walk(g, img, &fr));
    }
    Ret::eax(fr.eax)
}

/// `magic_0070` 0x10005c00 (5 DLLs) over [`spring_walk`].
pub const JMP_SPRING_WALK: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&SPRING_WALK))],
    imports: &[],
};

/// Three counters' step (internal, `ecx` = the object `O`; each counter
/// `C = O + 0x1c i`: `+0` a word pointer `P`, `+4` the value `V`, `+8`
/// the step `D`, `+0xc` the timer `T`, `+0x10` a base `B`, `+0x18` the
/// state; the original's jump table is in the code). State 0 counts as
/// done; 1 counts `T` down, `D = (20 - T) O+0x54 / 20` (truncated) and
/// `V -= D`, at 0 `D = O+0x54` and state 2; 2 waits (`V -= D`) until
/// counter `O+0x64` is this one and the flags `*G_F` have bit 5 or slot
/// 651 (ST0) is 0, then a sound starts (slot 719 `(S, 6, 0)`, slot 220
/// `(it, 0x40, 0x100)`) and it goes on as 3: `V -= D` while `V`'s low
/// half is above `D`, else `V` keeps its high half (also into `B`), `T =
/// 8`, state 4 and the counter "moved"; 4 counts `T` down with `V = B +
/// K[T]` (the DLL's table), at 0 `V = B`, `D = 0` and state 0. A
/// negative `V` gets `*P << 16` added. When one moved, `O+0x64` goes up.
/// Returns whether all three are done.
///
/// `magic_0644` 0x10005b80 (416 bytes with its jump table, `ecx` = the
/// object; 5 DLLs).
#[allow(clippy::too_many_lines)] // the original's states
pub fn counters_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let o = fr.ecx;
    let mut done = 0u32;
    let mut moved = false;
    for i in 0..3u32 {
        let c = o + 0x1c * i;
        let (v, d, t, b, state) = (c + 4, c + 8, c + 0xc, c + 0x10, c + 0x18);
        let rd = |g: &G, a: u32| g.read::<i32>(a);
        let mut s = g.read::<u32>(state);
        if s == 1 {
            let n = rd(g, t) - 1;
            g.write::<i32>(t, n);
            if n > 0 {
                let q = 20i32.wrapping_sub(n).wrapping_mul(rd(g, o + 0x54)) / 20;
                g.write::<i32>(v, rd(g, v).wrapping_sub(q));
                g.write::<i32>(d, q);
            } else {
                let a = rd(g, o + 0x54);
                g.write::<i32>(v, rd(g, v).wrapping_sub(a));
                g.write::<u32>(state, 2);
                g.write::<i32>(d, a);
            }
        } else if s == 0 {
            done += 1;
        } else if s <= 4 {
            if s == 2 {
                let start = g.read::<u32>(o + 0x64) == i && {
                    let flags = g.read::<u16>(w(0x91));
                    let x = crate::m0018::st0_f32(slot(g, img, f, 0x8a, 651, &[]).st0);
                    // ucomiss; lahf; test ah, 0x44; jp: on only for 0
                    #[allow(clippy::float_cmp)] // the original's exact test
                    let zero = x == 0.0;
                    flags & 0x20 != 0 || zero
                };
                if start {
                    let h = slot(g, img, f, 0xb7, 719, &[w(0xc6), 6, 0]).eax;
                    slot(g, img, f, 0xd5, 220, &[h, 0x40, 0x100]);
                    g.write::<u32>(state, 3);
                    s = 3;
                } else {
                    g.write::<i32>(v, rd(g, v).wrapping_sub(rd(g, d)));
                }
            }
            if s == 3 {
                let (x, dd) = (rd(g, v), rd(g, d));
                let low = x & 0xffff;
                if low > dd {
                    g.write::<i32>(v, x.wrapping_sub(dd));
                } else {
                    let high = x.wrapping_sub(low);
                    g.write::<i32>(v, high);
                    g.write::<i32>(b, high);
                    g.write::<u32>(t, 8);
                    g.write::<u32>(state, 4);
                    moved = true;
                    s = 4;
                }
            }
            if s == 4 {
                let n = rd(g, t) - 1;
                g.write::<i32>(t, n);
                let x = if n > 0 {
                    let k = g.read::<i32>(w(0x145).wrapping_add(4 * n.cast_unsigned()));
                    k.wrapping_add(rd(g, b))
                } else {
                    let x = rd(g, b);
                    g.write::<u32>(d, 0);
                    g.write::<u32>(state, 0);
                    x
                };
                g.write::<i32>(v, x);
            }
        }
        let x = rd(g, v);
        if x < 0 {
            let p = g.read::<u32>(c);
            let hi = g.read::<u32>(p) << 16;
            g.write::<u32>(v, hi.wrapping_add(x.cast_unsigned()));
        }
    }
    if moved {
        let n = g.read::<u32>(o + 0x64).wrapping_add(1);
        g.write::<u32>(o + 0x64, n);
    }
    u32::from(done.cast_signed() >= 3)
}

/// [`counters_step`] (the `jmp rel32`s inside stay in the hash).
pub const COUNTERS_STEP: Sig = Sig {
    len: 416,
    hash: 0xf601_a2fb_0bb3_4377,
    calls: &[],
    imports: &[],
};

/// The groups of four lists matched against the battle selection (cdecl,
/// no arguments): for each `k` of 0..4, slot 645 `(*G_A, &c, k, &h)`
/// gives a byte list of `c` entries and a handle `h`; a list with a
/// handle becomes the next record `R` (0x28 bytes from `*G_S + 0xebf4`):
/// `R[0] = h`, `R[1] = c`, and for each entry the index of the first
/// selected actor (`*G_SEL[i]` bytes, `i < *G_N`; `*G_OWN` stands for
/// -1) with that byte into `R+8+j` (left as it is when none). The count
/// goes to `*G_S + 0xec98`. Returns it.
///
/// `magic_0565` 0x10006ba0 (230 bytes; 5 DLLs).
pub fn selection_groups<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let s = g.read::<u32>(w(0x7));
    let mut out = s.wrapping_add(0xebf4);
    let n = g.read::<i32>(w(0x18));
    let ebp = fr.esp.wrapping_sub(4);
    let (c_at, h_at) = (ebp.wrapping_sub(0xc), ebp.wrapping_sub(0x14));
    let mut count = 0u32;
    for k in 0..4u32 {
        let a = g.read::<u32>(w(0x40));
        let list = slot(g, img, f, 0x3a, 645, &[a, c_at, k, h_at]).eax;
        let h = g.read::<u32>(h_at);
        if h == 0 {
            continue;
        }
        let c = g.read::<i32>(c_at);
        g.write::<u32>(out, h);
        g.write::<i32>(out + 4, c);
        let mut j = 0i32;
        while j < c {
            let own = g.read::<u32>(w(0x69));
            let b = u32::from(g.read::<u8>(list.wrapping_add(j.cast_unsigned())));
            let mut i = 0i32;
            while i < n {
                let id = if i == -1 {
                    own
                } else {
                    let sel = g.read::<u32>(w(0x8b));
                    u32::from(g.read::<u8>(sel.wrapping_add(i.cast_unsigned())))
                };
                if b == id {
                    g.write::<u8>(
                        out.wrapping_add(8).wrapping_add(j.cast_unsigned()),
                        i.cast_unsigned() as u8,
                    );
                    break;
                }
                i += 1;
            }
            j += 1;
        }
        count += 1;
        out = out.wrapping_add(0x28);
    }
    g.write::<u32>(s.wrapping_add(0xec98), count);
    Ret::eax(count)
}

/// [`selection_groups`].
pub const SELECTION_GROUPS: Sig = Sig {
    len: 230,
    hash: 0x57ac_355d_9dae_6f86,
    calls: &[],
    imports: &[],
};

/// A path follower's step (internal: `ecx` = the follower `F`, `edx` =
/// the point out; `F+0x20` the key table (three floats a key), `+0x28`
/// five weights `w`, `+0x2c` the time, `+0x30` its step, `+0x34` the
/// frames left, `+0x38` the key count `n`). On the last frame the last
/// key is the point and `F+0x3c = 0`. Otherwise the time steps on (`t`)
/// and the key position is the quartic Bernstein mix `x = w0 u⁴ + K w1 u³
/// t + K w2 u² t² + K w3 u t³ + w4 t⁴` (`u = K1 - t`), held below `n - D`
/// (`D` a double); each coordinate is [`ease_sample`]'s at `x`. Returns
/// the last key's `z` word, or the sample's scratch `eax` (junk: 0 here).
///
/// `magic_0515` 0x10001c00 (288 bytes, `ecx`/`edx`; 5 DLLs).
pub fn path_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (s, out) = (fr.ecx, fr.edx);
    let left = g.read::<u32>(s + 0x34).wrapping_sub(1);
    g.write::<u32>(s + 0x34, left);
    let tab = g.read::<u32>(s + 0x20);
    if left == 0 {
        let n = g.read::<u32>(s + 0x38);
        let last = tab.wrapping_add(n.wrapping_mul(12)).wrapping_sub(0xc);
        let mut eax = 0;
        for o in [0u32, 4, 8] {
            eax = g.read::<u32>(last.wrapping_add(o));
            g.write::<u32>(out + o, eax);
        }
        g.write::<u32>(s + 0x3c, 0);
        return eax;
    }
    let t = add(g.read::<f32>(s + 0x30), g.read::<f32>(s + 0x2c));
    let (one, k) = (g.read::<f32>(w(0x21)), g.read::<f32>(w(0x29)));
    let wt = g.read::<u32>(s + 0x28);
    let n = g.read::<i32>(s + 0x38);
    let wk = |g: &G, i: u32| g.read::<f32>(wt.wrapping_add(4 * i));
    let u = sub(one, t);
    g.write::<f32>(s + 0x2c, t);
    let (uu, ut, tt) = (mul(u, u), mul(u, t), mul(t, t));
    let b0 = mul(mul(uu, uu), wk(g, 0));
    let b1 = mul(mul(mul(ut, uu), k), wk(g, 1));
    let b2 = mul(mul(mul(tt, uu), k), wk(g, 2));
    let b3 = mul(mul(mul(tt, ut), k), wk(g, 3));
    let b4 = mul(mul(tt, tt), wk(g, 4));
    let x = add(add(add(add(b1, b0), b2), b3), b4);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let top = n as f32;
    // comiss x, n; jb: kept below `n` or unordered
    let x = if x < top || x.is_nan() {
        x
    } else {
        (f64::from(top) - g.read::<f64>(w(0xb2))) as f32
    };
    // push 3 (the stride) for each sample, popped after the third
    for (i, call) in [0xc2u32, 0xd6, 0xeb].into_iter().enumerate() {
        let esp = fr.esp.wrapping_sub(0x14 + 4 * i as u32);
        g.write::<u32>(esp + 4, 3);
        let n = g.read::<u32>(s + 0x38);
        let tab = tab.wrapping_add(4 * i as u32);
        let v = ease_sample(g, img, img.rel32_target(f + call), esp, tab, n, x);
        g.write::<f32>(out + 4 * i as u32, v);
    }
    // eax: the sample's scratch (junk; 0 here)
    0
}

/// [`path_step`] (the `rel32`s: three [`ease_sample`]s).
pub const PATH_STEP: Sig = Sig {
    len: 288,
    hash: 0xee26_1caf_5fab_1f47,
    calls: &[
        (0xc2, Callee::Sig(&EASE_SAMPLE)),
        (0xd6, Callee::Sig(&EASE_SAMPLE)),
        (0xeb, Callee::Sig(&EASE_SAMPLE)),
    ],
    imports: &[],
};

/// A character's motion script step (cdecl, no arguments). The script
/// `*G_T` has records of four words (motion, frame, callback, its
/// argument), the step `*G_I`, the character `*G_CH`, the playing motion
/// `*G_M`. Nothing at the end (motion 0). With no motion playing the
/// step's is started (slot 37 `(ch, 1, motion)`, slot 18 gives `*G_M`;
/// -1 skips to the script's end marker). Then the motion's frame (slot
/// 22, `>> 8`, plus 1) is compared: with the record's frame (when it has
/// one) or with `*G_M`; when it is reached the script steps on and, unless
/// at the end, the next motion starts (slot 30, slot 37, slot 18). The
/// step's callback (if any) runs with its argument. Returns the
/// callback's `eax`, 0, or the frame compared at the end.
///
/// `magic_0188` 0x10003970 (307 bytes; 5 DLLs).
pub fn motion_script_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let tab = g.read::<u32>(w(0xb));
    let rec = |i: u32, o: u32| tab.wrapping_add(i.wrapping_mul(0x10)).wrapping_add(o);
    let i = g.read::<u32>(w(0x2));
    let motion = g.read::<u32>(rec(i, 0));
    if motion == 0 {
        return Ret::eax(0);
    }
    let mut i = i;
    if g.read::<u32>(w(0x1e)) == 0 {
        let ch = g.read::<u32>(w(0x2f));
        slot(g, img, f, 0x27, 37, &[ch, 1, motion]);
        let ch = g.read::<u32>(w(0x42));
        let m = slot(g, img, f, 0x3c, 18, &[ch]).eax;
        i = g.read::<u32>(w(0x4d));
        g.write::<u32>(w(0x55), m);
        if m == u32::MAX && g.read::<u32>(rec(i, 0)) != 0 {
            loop {
                i = i.wrapping_add(1);
                g.write::<u32>(w(0x6f), i);
                if g.read::<u32>(rec(i, 0)) == 0 {
                    break;
                }
            }
        }
    }
    let ch = g.read::<u32>(w(0x80));
    let has_frame = g.read::<u32>(rec(i, 4)) != 0;
    let r = slot(g, img, f, 0x7a, 22, &[ch]).eax;
    let at = (r.cast_signed() >> 8).wrapping_add(1).cast_unsigned();
    let reached = if has_frame {
        let i = g.read::<u32>(w(0xad));
        g.read::<u32>(rec(i, 4)) == at
    } else {
        g.read::<u32>(w(0xa1)) == at
    };
    if reached {
        let i = g
            .read::<u32>(if has_frame { w(0xad) } else { w(0x94) })
            .wrapping_add(1);
        g.write::<u32>(w(0xc5), i);
        if g.read::<u32>(rec(i, 0)) == 0 {
            return Ret::eax(at);
        }
        let ch = g.read::<u32>(w(0xd8));
        slot(g, img, f, 0xd2, 30, &[ch]);
        let i = g.read::<u32>(w(0xe2));
        let motion = g.read::<u32>(rec(i, 0));
        let ch = g.read::<u32>(w(0xf4));
        slot(g, img, f, 0xec, 37, &[ch, 1, motion]);
        let ch = g.read::<u32>(w(0x107));
        let m = slot(g, img, f, 0x101, 18, &[ch]).eax;
        g.write::<u32>(w(0x11a), m);
    }
    let i = g.read::<u32>(if reached {
        w(0x112)
    } else if has_frame {
        w(0xad)
    } else {
        w(0x94)
    });
    let cb = g.read::<u32>(rec(i, 8));
    if cb == 0 {
        return Ret::eax(0);
    }
    let arg = g.read::<u32>(rec(i, 0xc));
    Ret::eax(g.call(cb, i.wrapping_mul(2), &[arg]).eax)
}

/// [`motion_script_step`].
pub const MOTION_SCRIPT_STEP: Sig = Sig {
    len: 307,
    hash: 0x885e_8550_4943_3b93,
    calls: &[],
    imports: &[],
};

/// A model node taken out of its parent's children and its motions
/// released (internal, `ecx` = a pointer to the node `N`): with a parent
/// `P = N+0x14` its child count (u16 `+0x18`) drops and the node leaves
/// the child list (`P+0x10`/`P+0xc` when it was the first/last), closed
/// on itself and `N+0x14 = 0`. Then for both motion lists (records at
/// `N+0x24 + 4j`, 0x40 bytes each, count u16 `N+0x2c + 2j`) each record's
/// handle (`+0x10`, as a 64-bit number) goes to slot 779 with the game
/// lock held. Returns `N + 0x30`.
///
/// `magic_0070` 0x10001f10 (201 bytes, `ecx` = the node's pointer; 4 DLLs).
pub fn node_release<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let n = g.read::<u32>(fr.ecx);
    let p = g.read::<u32>(n + 0x14);
    if p != 0 {
        let c = g.read::<u16>(p + 0x18).wrapping_add(0xffff);
        g.write::<u16>(p + 0x18, c);
        let (prev, next) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
        if prev == p {
            g.write::<u32>(p + 0x10, next);
        } else {
            g.write::<u32>(prev + 8, next);
        }
        let (next, prev) = (g.read::<u32>(n + 8), g.read::<u32>(n + 4));
        if next == p {
            g.write::<u32>(p + 0xc, prev);
        } else {
            g.write::<u32>(next + 4, prev);
        }
        g.write::<u32>(n + 8, n);
        g.write::<u32>(n + 4, n);
        g.write::<u32>(n + 0x14, 0);
    }
    for j in 0..2u32 {
        let mut lock_at = w(0x6b);
        let mut rec = g.read::<u32>(n + 0x24 + 4 * j).wrapping_add(0x10);
        let mut k = 0u32;
        while k < u32::from(g.read::<u16>(n + 0x2c + 2 * j)) {
            let lock = g.read::<u32>(lock_at);
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let hi = (rec.cast_signed() >> 31).cast_unsigned();
            slot(g, img, f, 0x7c, 779, &[rec, hi]);
            lock_at = w(0x90);
            g.write::<u32>(g.read::<u32>(lock_at), saved);
            k += 1;
            rec = rec.wrapping_add(0x40);
        }
    }
    n + 0x30
}

/// [`node_release`].
pub const NODE_RELEASE: Sig = Sig {
    len: 201,
    hash: 0xe261_e5e5_601a_2599,
    calls: &[],
    imports: &[],
};

/// Parts' three axes turned (internal: `ecx` = the parts' base, `edx` =
/// the count; a /GS frame): for each part `p` (0xcc bytes from `+0xac`)
/// a rotation matrix `M` (a local) of the angle `-0x4000 - p[-0x96]`
/// (an `i16`; slot 247), then its three rows, scaled by `p[-0x84]`, into
/// `p-0x20`, `p-0x10` and `p` (slot 233 each), and `p+0x10..+0x1c` a
/// copy of `p-0x70..`. Returns slot 233's `eax` (the frame's cookie
/// word `*G_COOKIE ^ ebp` for no parts).
///
/// `magic_0515` 0x100026f0 (225 bytes, `ecx`/`edx`; 4 DLLs).
pub fn parts_axes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let ebp = fr.esp.wrapping_sub(4);
    let mut eax = g.read::<u32>(op(img, f, 0x7)) ^ ebp;
    let m = ebp.wrapping_sub(0x44);
    let mut p = fr.ecx.wrapping_add(0xac);
    for _ in 0..fr.edx {
        let a = i32::from(g.read::<i16>(p.wrapping_sub(0x96)));
        let angle = 0xffff_c000u32.wrapping_sub(a.cast_unsigned());
        slot(g, img, f, 0x36, 247, &[m, angle]);
        let s = g.read::<u32>(p.wrapping_sub(0x84));
        for (t, out, row) in [
            (0x5bu32, 0x20u32, 0u32),
            (0x82, 0x10, 0x10),
            (0xa0, 0, 0x20),
        ] {
            eax = slot(g, img, f, t, 233, &[p.wrapping_sub(out), m + row, s]).eax;
        }
        for o in [0u32, 4, 8, 0xc] {
            let x = g.read::<u32>(p.wrapping_sub(0x70).wrapping_add(o));
            g.write::<u32>(p.wrapping_add(0x10 + o), x);
        }
        p = p.wrapping_add(0xcc);
    }
    eax
}

/// [`parts_axes`] (the `rel32`: `__security_check_cookie`).
pub const PARTS_AXES: Sig = Sig {
    len: 225,
    hash: 0xeae1_2499_7413_c9bc,
    calls: &[(0xd9, Callee::Any)],
    imports: &[],
};

/// [`thrown_effect`](crate::s5_misc::thrown_effect) without the model
/// switch (internal: `ecx` = the object, `edx` = the user's record,
/// `xmm2` = a size; an aligned frame): the task node (vtable `VT`,
/// callback `CB`) appended after `*G_TAIL` (the list `S`, bit 0 of `*G_F`
/// cleared); the nodes `+0x120` and `+0x14c` on the DLL's list
/// ([`list_push`]), the first's point `+0x13c` the option point 0x11 of
/// `rec+0x54` (slot 268, `w` 1.0); `+0x168..+0x170 = 1.0`, `+0x174 = size
/// K`; the node `+0x178` inserted ([`node_insert`]: `(node, L, Q, +0x120,
/// Q2, +0x14c)`), the particle `+0x20` ([`particle_node`]: `(p, P,
/// +0x178, *G_O, Q3, 1)`, `p[0] = VT2`), the effect node `+0x2d8`
/// ([`effect_node`], vtable `VT3`, `+0x2c |= 1`, `+0x30 = 0`); `+0x13c`,
/// `+0x140`, `+0x144` moved by `(frnd - K1) K2 - K3` each (slot 307,
/// ST0); `+0x2d4 = rec`, `+0x2c4`, `+0x2cc`, `+0x2d0` cleared and
/// `+0x2c8` the size. Returns `rec`.
///
/// `magic_0425` 0x10006aa0 (769 bytes; 3 DLLs).
#[allow(clippy::too_many_lines)] // the original's nodes, one by one
pub fn thrown_effect_plain<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let (obj, rec) = (fr.ecx, fr.edx);
    let size = fr.xmm[2] as u32;
    let top = fr.esp.wrapping_sub(4) & !7;
    g.write::<u32>(top.wrapping_sub(0x24), size);
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(0x1c));
    g.write::<u32>(obj + 0x10, w(0x23));
    g.write::<u32>(obj + 0x14, obj);
    let tail = g.read::<u32>(w(0x2b));
    g.write::<u32>(obj + 8, tail);
    g.write::<u32>(obj + 4, w(0x35));
    let tail = g.read::<u32>(w(0x3a));
    g.write::<u32>(tail.wrapping_add(4), obj);
    let fl = g.read::<u32>(w(0x49)) & !1;
    g.write::<u32>(w(0x49), fl);
    g.write::<u32>(w(0x50), obj);
    let first = obj.wrapping_add(0x120);
    list_push(
        g,
        img,
        f,
        first,
        [0x63, 0x6a, 0x6f, 0x77, 0x80, 0x88, 0x94, 0xa5, 0xac],
    );
    g.write::<u32>(top.wrapping_sub(0x1c), rec);
    let pt = obj.wrapping_add(0x13c);
    let h = g.read::<u32>(rec.wrapping_add(0x54));
    slot(g, img, f, 0x99, 268, &[h, 0x11, pt]);
    g.write::<u32>(pt + 0xc, 0x3f80_0000);
    let second = obj.wrapping_add(0x14c);
    list_push(
        g,
        img,
        f,
        second,
        [0xdc, 0xe3, 0xe8, 0xf0, 0xf5, 0xfd, 0x109, 0x11d, 0x123],
    );
    let s = mul(f32::from_bits(size), k(g, 0x117));
    let group = top.wrapping_sub(0x18);
    g.write::<u32>(group + 0x14, second);
    for o in [0x168u32, 0x16c, 0x170] {
        g.write::<u32>(obj + o, 0x3f80_0000);
    }
    g.write::<f32>(obj + 0x174, s);
    let node = obj.wrapping_add(0x178);
    for (i, v) in (0u32..).zip([node, w(0x163), w(0x16b), first, w(0x177)]) {
        g.write::<u32>(group + 4 * i, v);
    }
    let callee = |at: u32| Frame {
        at,
        esp: top.wrapping_sub(0x34),
        ecx: group,
        ..*fr
    };
    node_insert(g, img, &callee(img.rel32_target(f + 0x17c)));
    let o = g.read::<u32>(w(0x181));
    let part = obj.wrapping_add(0x20);
    g.write::<u32>(group, part);
    g.write::<u32>(part, w(0x18e));
    for (i, v) in (1u32..).zip([w(0x19a), node, o, w(0x1aa), 1]) {
        g.write::<u32>(group + 4 * i, v);
    }
    particle_node(g, img, img.rel32_target(f + 0x1b7), group);
    let en = obj.wrapping_add(0x2d8);
    for (i, v) in (0u32..).zip([en, w(0x1cd), w(0x1d5), first, w(0x1e1), 0]) {
        g.write::<u32>(group + 4 * i, v);
    }
    effect_node(g, img, img.rel32_target(f + 0x1ee), group);
    let n = g.read::<u32>(group);
    g.write::<u32>(n, w(0x1f8));
    let fl = g.read::<u32>(en + 0x2c) | 1;
    g.write::<u32>(en + 0x2c, fl);
    g.write::<u32>(en + 0x30, 0);
    let moved = |g: &mut G, t: u32, ks: [u32; 3]| {
        let r = crate::m0018::st0_f32(slot(g, img, f, t, 307, &[]).st0);
        sub(mul(sub(r, k(g, ks[0])), k(g, ks[1])), k(g, ks[2]))
    };
    let d = moved(g, 0x1fd, [0x222, 0x22f, 0x237]);
    g.write::<f32>(obj + 0x13c, add(d, g.read::<f32>(obj + 0x13c)));
    let d = moved(g, 0x227, [0x261, 0x26e, 0x276]);
    g.write::<f32>(obj + 0x140, add(d, g.read::<f32>(obj + 0x140)));
    let d = moved(g, 0x266, [0x2a0, 0x2c6, 0x2d8]);
    let rec = g.read::<u32>(top.wrapping_sub(0x1c));
    g.write::<u32>(obj + 0x2d4, rec);
    for o in [0x2c4u32, 0x2cc, 0x2d0] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<f32>(obj + 0x144, add(d, g.read::<f32>(obj + 0x144)));
    g.write::<u32>(obj + 0x2c8, g.read::<u32>(top.wrapping_sub(0x24)));
    rec
}

/// [`thrown_effect_plain`] (the `rel32`s: [`node_insert`],
/// [`particle_node`] and [`effect_node`]).
pub const THROWN_EFFECT_PLAIN: Sig = Sig {
    len: 769,
    hash: 0xe2d1_9650_e39b_e587,
    calls: &[
        (0x17c, Callee::Sig(&INSERT)),
        (0x1b7, Callee::Sig(&PARTICLE_NODE)),
        (0x1ee, Callee::Sig(&EFFECT_NODE)),
    ],
    imports: &[],
};

/// Starts an effect object with its first part and a reset state block
/// (internal: `edx` = the object, `xmm3` its scale): the model words
/// (scale `+0x60..+0x6c`), `*G_P = ITEM`, the first program of slot 941
/// `(*G_D + 0x10)` started on it ([`start_part`], 0x1000, 1), its state
/// block reset ([`reset_state`] with the callback immediate `CB`, `+0x11c
/// = 0x14`), then a pool node holding it appended to the object list
/// ([`append_object`]). Returns the object.
///
/// `magic_0032` 0x10001b30 (231 bytes; 6 DLLs).
pub fn start_object_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let obj = fr.edx;
    let scale = fr.xmm[3] as u32;
    let p = g.read::<u32>(w(0x1));
    for o in [0x48u32, 0x44, 0x40] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<u32>(obj + 0x4c, 0x3f80_0000);
    for o in [0x5cu32, 0x58, 0x54, 0x50] {
        g.write::<u32>(obj + o, 0);
    }
    for o in [0x6cu32, 0x68, 0x64, 0x60] {
        g.write::<u32>(obj + o, scale);
    }
    g.write::<u32>(p, w(0x56));
    let d = g.read::<u32>(g.read::<u32>(w(0x5b)).wrapping_add(0x10));
    let table = slot(g, img, f, 0x63, 941, &[d]).eax;
    let program = g.read::<u32>(table);
    start_part(g, img, img.rel32_target(f + 0x7d), obj, 0x1000, program, 1);
    reset_state(g, obj, w(0x85), 0, 0x14);
    let list = [0xa0, 0xb3, 0xb9, 0xc2, 0xcd, 0xd6, 0xdf];
    append_object(g, img, f, img.rel32_target(f + 0x95), obj, list);
    obj
}

/// [`start_object_reset`] (the `rel32`s: [`start_part`],
/// [`reset_state`] and the pool's take,
/// [`pool_alloc`](crate::l5_objhost::pool_alloc)).
pub const START_OBJECT_RESET: Sig = Sig {
    len: 231,
    hash: 0xa281_110d_314d_b0b4,
    calls: &[
        (0x7d, Callee::Sig(&START_PART)),
        (0x8d, Callee::Sig(&RESET_STATE)),
        (0x95, Callee::Sig(&POOL_ALLOC)),
    ],
    imports: &[],
};

/// Four sparks spawned on a model (internal: `ecx` = the spawner `S`,
/// `edx` = a word, `xmm2` = a speed): `n` = slot 263 `(S+4)` (the
/// model's vertex count); four times a free record `p`
/// ([`free_record`]; none ends it, returning 0): `p+0x2c = 0`, `+0x30 =
/// CB`, `+0x28` the word, `+0x44 = 0x40808080`, `+0x34 = (frnd - K)
/// speed` and `+0x48 = 0` (slot 307, ST0), u16 `+0x40` slot 324's word,
/// and the record placed at the vertex `trunc((frnd - K) n)` (slot 237
/// `(S+4, vertex, 0, p)`), `+0xc = 1.0`. Returns slot 237's `eax`.
///
/// `magic_0515` 0x10003d90 (243 bytes, `ecx`/`edx`/`xmm2`; 4 DLLs).
pub fn sparks_on_model<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let (s, word) = (fr.ecx, fr.edx);
    let speed = fr.xmm_f32(2);
    let st0 = |o: Out| crate::m0018::st0_f32(o.st0);
    let m = g.read::<u32>(s + 4);
    let n = slot(g, img, f, 0x7, 263, &[m]).eax;
    let take = Frame {
        at: img.rel32_target(f + 0x32),
        ..*fr
    };
    let mut eax = 0;
    for _ in 0..4 {
        let p = free_record(g, img, &take).eax;
        if p == 0 {
            return 0;
        }
        g.write::<u32>(p + 0x2c, 0);
        g.write::<u32>(p + 0x30, w(0x53));
        g.write::<u32>(p + 0x28, word);
        g.write::<u32>(p + 0x44, 0x4080_8080);
        let r = st0(slot(g, img, f, 0x45, 307, &[]));
        g.write::<u32>(p + 0x48, 0);
        g.write::<f32>(p + 0x34, mul(sub(r, k(g, 0x75)), speed));
        let h = slot(g, img, f, 0x7f, 324, &[]).eax;
        g.write::<u16>(p + 0x40, h as u16);
        let r = st0(slot(g, img, f, 0x9a, 307, &[]));
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
        let v = trunc(mul(sub(r, k(g, 0xb7)), n.cast_signed() as f32));
        let m = g.read::<u32>(s + 4);
        eax = slot(g, img, f, 0xcb, 237, &[m, v, 0, p]).eax;
        g.write::<u32>(p + 0xc, 0x3f80_0000);
    }
    eax
}

/// [`sparks_on_model`] (the `rel32`: [`free_record`]).
pub const SPARKS_ON_MODEL: Sig = Sig {
    len: 243,
    hash: 0xcfb0_4f02_d920_b290,
    calls: &[(0x32, Callee::Sig(&FREE_RECORD))],
    imports: &[],
};

/// A ring of parts placed on a circle (cdecl: `ecx` = the parts base,
/// `edx` = the count `n`, `xmm3` = a radius, the stack `(-, base_angle,
/// word)`). With `n == 1` the single part's point `+0x3c..+0x48` is the
/// centre `G_V` (three immediates, `w` 1.0) and u16 `+0x16` the word.
/// With `n > 1`, for each part `i` (0xcc bytes apart) the angle `a =
/// (i << 16) / n + base_angle` (u16): `+0x3c = sin[(a + 0x4000 >> 4) &
/// 0xfff] r + Kx`, `+0x44 = sin[(a >> 4) & 0xfff] r + Kz`, `+0x40 = Ky`,
/// `+0x48 = 1.0`, u16 `+0x16 = word + a`. (`sin` is `*G_SIN`, Kx/Ky/Kz
/// the centre immediates.) Returns the last `word + a` (the
/// caller's `eax` for no parts, with the word in `ax` for one).
///
/// `magic_0515` 0x10002b70 (253 bytes, `ecx`/`edx`/`xmm3` and three stack
/// words; 4 DLLs).
pub fn parts_ring<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<u32>(w(o));
    let base = fr.ecx;
    let n = fr.edx;
    let r = fr.xmm_f32(3);
    // the stack: an unread word, the base angle, the word
    let (base_angle, word) = (fr.arg(g, 1), fr.arg(g, 2));
    if n == 1 {
        g.write::<u32>(base + 0x3c, k(g, 0x15));
        g.write::<u32>(base + 0x40, k(g, 0x26));
        g.write::<u32>(base + 0x44, k(g, 0x33));
        g.write::<u32>(base + 0x48, 0x3f80_0000);
        g.write::<u16>(base + 0x16, word as u16);
        return (fr.eax & 0xffff_0000) | (word & 0xffff);
    }
    if n.cast_signed() <= 0 {
        return fr.eax;
    }
    let tab = g.read::<u32>(w(0x67));
    let sin = |g: &G, idx: u32| g.read::<f32>(tab.wrapping_add(4 * (idx & 0xfff)));
    let mut eax = fr.eax;
    for i in 0..n {
        let a = (i << 16)
            .cast_signed()
            .wrapping_div(n.cast_signed())
            .cast_unsigned();
        let angle = a.wrapping_add(base_angle) & 0xffff;
        let p = base.wrapping_add(0xcc * i);
        let cos = sin(
            g,
            angle
                .wrapping_add(0x4000)
                .cast_signed()
                .wrapping_shr(4)
                .cast_unsigned(),
        );
        let x = add(mul(cos, r), g.read::<f32>(w(0xa2)));
        g.write::<f32>(p + 0x3c, x);
        let z = add(mul(sin(g, angle >> 4), r), g.read::<f32>(w(0xc3)));
        g.write::<f32>(p + 0x44, z);
        g.write::<u32>(p + 0x40, k(g, 0xd3));
        g.write::<u32>(p + 0x48, 0x3f80_0000);
        eax = word.wrapping_add(angle);
        g.write::<u16>(p + 0x16, eax as u16);
    }
    eax
}

/// [`parts_ring`].
pub const PARTS_RING: Sig = Sig {
    len: 253,
    hash: 0xb62d_de48_5a4e_c5bd,
    calls: &[],
    imports: &[],
};

/// [`parts_axes`] for live parts only, each then handed on (internal:
/// `ecx` = the parts' base, `edx` = the count; a /GS frame): a part
/// whose word `p[-0xac]` is 0xff is passed over; the others get their
/// axes as there and slot 211 `(p[-0xa8], p-0x20)`. Returns slot 211's
/// `eax` (the frame's cookie word `*G_COOKIE ^ ebp` when no part runs).
///
/// `magic_0515` 0x100027e0 (265 bytes, `ecx`/`edx`; 4 DLLs).
pub fn live_parts_axes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let ebp = fr.esp.wrapping_sub(4);
    let mut eax = g.read::<u32>(op(img, f, 0x7)) ^ ebp;
    let m = ebp.wrapping_sub(0x44);
    let mut p = fr.ecx.wrapping_add(0xac);
    for _ in 0..fr.edx {
        if g.read::<u32>(p.wrapping_sub(0xac)) != 0xff {
            let a = i32::from(g.read::<i16>(p.wrapping_sub(0x96)));
            let angle = 0xffff_c000u32.wrapping_sub(a.cast_unsigned());
            slot(g, img, f, 0x48, 247, &[m, angle]);
            let s = g.read::<u32>(p.wrapping_sub(0x84));
            for (t, out, row) in [
                (0x69u32, 0x20u32, 0u32),
                (0x94, 0x10, 0x10),
                (0xb2, 0, 0x20),
            ] {
                slot(g, img, f, t, 233, &[p.wrapping_sub(out), m + row, s]);
            }
            for o in [0u32, 4, 8, 0xc] {
                let x = g.read::<u32>(p.wrapping_sub(0x70).wrapping_add(o));
                g.write::<u32>(p.wrapping_add(0x10 + o), x);
            }
            let h = g.read::<u32>(p.wrapping_sub(0xa8));
            eax = slot(g, img, f, 0xc5, 211, &[h, p.wrapping_sub(0x20)]).eax;
        }
        p = p.wrapping_add(0xcc);
    }
    eax
}

/// [`live_parts_axes`] (the `rel32`: `__security_check_cookie`).
pub const LIVE_PARTS_AXES: Sig = Sig {
    len: 265,
    hash: 0xd56a_a09c_96d0_dc80,
    calls: &[(0x101, Callee::Any)],
    imports: &[],
};

/// An object host's start with one object (cdecl, no arguments): the
/// two host words `*G_A = A`, `*G_B = B` (immediates), the effect data
/// (slot 719 `(E, 8, 0)` into `*G_E`), the part's set-up
/// ([`part_init`]), the object `O` started ([`start_object_reset`],
/// scale `K`), the battle position and option point 0x11 of the
/// selection's first actor into `G_P` (slots 262 and 268, `w` 1.0), the
/// host record `G_R` cleared (but `+0x20 = E`, `+0x24 = T`, u16 `+0x30 =
/// 0`, `*G_C = G_R`), and a task node ([`pop_node`]) appended to the
/// task list. Each call sees the words the earlier ones left on the
/// stack. Returns 0.
///
/// `magic_0032` 0x10001370 (319 bytes; 4 DLLs).
pub fn host_start_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let mut s = Pushes::default();
    s.push_all(&[0, 8, w(0x11)]);
    g.write::<u32>(w(0x17), w(0x1b));
    g.write::<u32>(w(0x21), w(0x25));
    // call width: 12 (the most `Pushes::words` holds)
    let e = s
        .call(g, crate::runtime::slot_target(g, w(0x1), 719), 0)
        .eax;
    s.pop(1);
    g.write::<u32>(w(0x2f), e);
    part_init(g, img, img.rel32_target(f + 0x34));
    // sub esp, 8; call: the start's frame
    let start = Frame {
        at: img.rel32_target(f + 0x49),
        esp: fr.esp.wrapping_sub(0x18),
        edx: w(0x44),
        xmm: [0, 0, 0, u128::from(g.read::<u32>(w(0x3c)))],
        ..*fr
    };
    start_object_reset(g, img, &start);
    // sub esp, 8: the words there are the part set-up's return address
    // (its call at +0x33) and its first push (the 0 of slot 711's words)
    s.push_all(&[f + 0x38, 0]);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x4e))));
    let ch = g.read::<u32>(w(0x62).wrapping_add(4 * b));
    s.push_all(&[w(0x53), w(0x5b), w(0x6c), ch]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, crate::runtime::slot_target(g, w(0x67), 262), 0);
    s.push_all(&[w(0x7f), 0x11, ch]);
    // call width: 12 (the most `Pushes::words` holds)
    s.call(g, crate::runtime::slot_target(g, w(0x7a), 268), 0);
    g.write::<u32>(w(0x95), 0x3f80_0000);
    for o in [0x9fu32, 0xa9, 0xb3, 0xbd] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0xc7), w(0xcb));
    g.write::<u32>(w(0xd1), w(0xd5));
    g.write::<u32>(w(0xdb), 0);
    g.write::<u32>(w(0xe5), 0);
    g.write::<u16>(w(0xef), 0);
    g.write::<u32>(w(0xf5), w(0xf9));
    let node = pop_node(g, img, img.rel32_target(f + 0xfe));
    g.write::<u32>(node.wrapping_add(4), 0);
    if g.read::<u32>(w(0x104)) != 0 {
        let tail = g.read::<u32>(w(0x115));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x122), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x130), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(w(0x138), node);
    }
    Ret::eax(0)
}

/// [`host_start_object`] (the `rel32`s: [`part_init`],
/// [`start_object_reset`] and [`pop_node`]).
pub const HOST_START_OBJECT: Sig = Sig {
    len: 319,
    hash: 0x20ef_39c3_853f_8f8c,
    calls: &[
        (0x34, Callee::Sig(&PART_INIT)),
        (0x49, Callee::Sig(&START_OBJECT_RESET)),
        (0xfe, Callee::Sig(&POP_NODE)),
    ],
    imports: &[],
};

/// The operand offsets of a turning follower's step: the three eases'
/// `rel32`s, the slot table operands of slots 235, 330, 328, 329 and
/// 229, and (for the eased time) the operands of `K1` and `K` of
/// [`path_step`]'s mix.
struct TurnOps {
    eases: [u32; 3],
    slots: [u32; 5],
    mix: Option<(u32, u32)>,
}

/// [`turning_path_step`] and [`eased_turning_path_step`]: see there.
fn turning_step<G: Game>(g: &mut G, img: &Image, fr: &Frame, ops: &TurnOps) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (s, out) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    let left = g.read::<u32>(s + 0x34).wrapping_sub(1);
    g.write::<u32>(s + 0x34, left);
    let tab = g.read::<u32>(s + 0x20);
    let (x, y, z) = if left == 0 {
        let n = g.read::<u32>(s + 0x38);
        let last = tab.wrapping_add(n.wrapping_mul(12)).wrapping_sub(0xc);
        let key = |g: &G, o: u32| g.read::<f32>(last.wrapping_add(o));
        let (x, y, z) = (key(g, 0), key(g, 4), key(g, 8));
        g.write::<u32>(s + 0x3c, 0);
        (x, y, z)
    } else {
        let t = add(g.read::<f32>(s + 0x30), g.read::<f32>(s + 0x2c));
        g.write::<f32>(s + 0x2c, t);
        // the key position: the time itself, or [`path_step`]'s quartic
        // Bernstein mix of the five weights `F+0x28`
        let t = match ops.mix {
            None => t,
            Some((one, k)) => {
                let (one, k) = (g.read::<f32>(w(one)), g.read::<f32>(w(k)));
                let wt = g.read::<u32>(s + 0x28);
                let wk = |g: &G, i: u32| g.read::<f32>(wt.wrapping_add(4 * i));
                let u = sub(one, t);
                let (uu, ut, tt) = (mul(u, u), mul(u, t), mul(t, t));
                let b0 = mul(mul(uu, uu), wk(g, 0));
                let b1 = mul(mul(mul(ut, uu), k), wk(g, 1));
                let b2 = mul(mul(mul(tt, uu), k), wk(g, 2));
                let b3 = mul(mul(mul(tt, ut), k), wk(g, 3));
                let b4 = mul(mul(tt, tt), wk(g, 4));
                add(add(add(add(b1, b0), b2), b3), b4)
            }
        };
        let n = g.read::<u32>(s + 0x38);
        // push 3 (the stride) for each sample, popped after the third
        let esp = |g: &mut G, i: u32| {
            let at = ebp.wrapping_sub(0x78 + 4 * i);
            g.write::<u32>(at + 4, 3);
            at
        };
        let [e0, e1, e2] = ops.eases;
        let at = esp(g, 0);
        let x = ease_angle(g, img, img.rel32_target(f + e0), at, tab, n, t);
        let at = esp(g, 1);
        let y = ease_angle(g, img, img.rel32_target(f + e1), at, tab + 4, n, t);
        let at = esp(g, 2);
        let z = ease_sample(g, img, img.rel32_target(f + e2), at, tab + 8, n, t);
        (x, y, z)
    };
    let (m, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    for (o, val) in [(0u32, 0u32), (4, 0), (8, z.to_bits()), (0xc, 0x3f80_0000)] {
        g.write::<u32>(v + o, val);
    }
    let target = |g: &G, t: u32, n: u32| crate::runtime::slot_target(g, w(t), n);
    let [s235, s330, s328, s329, s229] = ops.slots;
    let mut st = Pushes::default();
    st.push(m);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, s235, 235), 0);
    for (i, (t, n, a)) in [
        (s330, 330u32, 0u32),
        (s328, 328, x.to_bits()),
        (s329, 329, y.to_bits()),
    ]
    .into_iter()
    .enumerate()
    {
        // add esp, 8 after each but the last; `mov [esp], a`
        if i > 0 {
            st.pop(2);
        }
        st.set_top(a);
        st.push_all(&[m, m]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, target(g, t, n), 0);
    }
    st.push_all(&[v, m, v]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, s229, 229), 0);
    let base = g.read::<u32>(s + 0x24);
    for o in [0u32, 4, 8] {
        let p = add(g.read::<f32>(base.wrapping_add(o)), g.read::<f32>(v + o));
        g.write::<f32>(out + o, p);
    }
    base
}

/// [`path_step`] for a turning follower (internal: `ecx` = the follower
/// `F`, `edx` = the point out; a /GS frame): on the last frame the last
/// key's angles (and `F+0x3c = 0`), else the time steps on and the three
/// angles are [`ease_angle`], [`ease_angle`] and [`ease_sample`] at it.
/// The point `(0, 0, z, 1)` is turned by a matrix (a local: slot 235,
/// then slots 330 `0`, 328 `x`, 329 `y`) through slot 229, and the base
/// point `*F+0x24` plus it is the point out. Each call sees the words the
/// earlier ones left on the stack. Returns the base point's address.
///
/// `magic_0515` 0x10001d20 (385 bytes, `ecx`/`edx`; 5 DLLs).
pub fn turning_path_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let ops = TurnOps {
        eases: [0x40, 0x56, 0x6c],
        slots: [0xa9, 0xdc, 0xfb, 0x11a, 0x133],
        mix: None,
    };
    turning_step(g, img, fr, &ops)
}

/// [`turning_path_step`] with the key position eased by [`path_step`]'s
/// quartic mix (no bound).
///
/// `magic_0515` 0x10001eb0 (514 bytes, `ecx`/`edx`; 5 DLLs).
pub fn eased_turning_path_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let ops = TurnOps {
        eases: [0xc1, 0xd7, 0xed],
        slots: [0x12a, 0x15d, 0x17c, 0x19b, 0x1b4],
        mix: Some((0x14, 0x3c)),
    };
    turning_step(g, img, fr, &ops)
}

/// [`eased_turning_path_step`] (the `rel32`s: two [`ease_angle`]s,
/// [`ease_sample`] and `__security_check_cookie`).
pub const EASED_TURNING_PATH_STEP: Sig = Sig {
    len: 514,
    hash: 0x17b5_223d_4bc9_0670,
    calls: &[
        (0xc1, Callee::Sig(&EASE_ANGLE)),
        (0xd7, Callee::Sig(&EASE_ANGLE)),
        (0xed, Callee::Sig(&EASE_SAMPLE)),
        (0x1fa, Callee::Any),
    ],
    imports: &[],
};

/// [`turning_path_step`] (the `rel32`s: two [`ease_angle`]s,
/// [`ease_sample`] and `__security_check_cookie`).
pub const TURNING_PATH_STEP: Sig = Sig {
    len: 385,
    hash: 0x8d96_d614_a54a_7860,
    calls: &[
        (0x40, Callee::Sig(&EASE_ANGLE)),
        (0x56, Callee::Sig(&EASE_ANGLE)),
        (0x6c, Callee::Sig(&EASE_SAMPLE)),
        (0x179, Callee::Any),
    ],
    imports: &[],
};

/// A part's per-frame step with its keyed motion (internal: `ecx` = the
/// part `P`, `edx` = a 3x3 matrix `R`; a /GS frame): the screen and
/// camera matrices into locals (slots 723, 724), a third local `M` made
/// a unit one (slot 235; `*G_M = M`, `*G_N = 0`), `*G_A = P+0x1c`, `M`'s
/// three rows `R`'s scaled by `P+0x20`, and `P+0xc` through the camera
/// into a local point (slot 229). The key table is slot 941 `(P[0])`;
/// while `P+4` is set the key's time `P+0xa` runs down by 0x200, and at
/// its end the key `P+8` steps on: past the last (`table+6`) it wraps to
/// 0 when the last key's byte `+4` has bit 7, else it stays on the last
/// and `P+4` is cleared; a new key takes its time from its record
/// (`table + 0x10 + 8 key + 2`). Then [`link_part`] `(P, the key's record,
/// G_O, 0)`. Returns its `eax`.
///
/// `magic_0180` 0x10001080 (419 bytes, `ecx`/`edx`; 5 DLLs).
pub fn keyed_part_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let (p, r) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    let [screen, cam, m, pt] = [0xccu32, 0x8c, 0x4c, 0x1c].map(|o| ebp.wrapping_sub(o));
    slot(g, img, f, 0x1d, 723, &[screen]);
    slot(g, img, f, 0x38, 724, &[cam]);
    g.write::<u32>(w(0x4b), m);
    g.write::<u32>(w(0x57), 0);
    slot(g, img, f, 0x51, 235, &[m]);
    let a = g.read::<u32>(p + 0x1c);
    g.write::<u32>(w(0x6b), a);
    let s = g.read::<f32>(p + 0x20);
    for o in [0u32, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28] {
        let x = mul(g.read::<f32>(r + o), s);
        g.write::<f32>(m + o, x);
    }
    slot(g, img, f, 0xb9, 229, &[pt, cam, p + 0xc]);
    let d = g.read::<u32>(p);
    let table = slot(g, img, f, 0x111, 941, &[d]).eax;
    let rec = |key: u16| table.wrapping_add(8 * (i32::from(key.cast_signed()) + 2).cast_unsigned());
    let key = g.read::<u16>(p + 8);
    let mut at = rec(key);
    if g.read::<u32>(p + 4) != 0 {
        let time = g.read::<u16>(p + 0xa).wrapping_add(0xfe00);
        g.write::<u16>(p + 0xa, time);
        if time.cast_signed() <= 0 {
            let next = key.wrapping_add(1);
            g.write::<u16>(p + 8, next);
            let new_key = if next.cast_signed() < g.read::<i16>(table.wrapping_add(6)) {
                true
            } else if g.read::<u8>(at.wrapping_add(4)) & 0x80 != 0 {
                g.write::<u16>(p + 8, 0);
                true
            } else {
                g.write::<u16>(p + 8, key);
                g.write::<u32>(p + 4, 0);
                false
            };
            if new_key {
                at = rec(g.read::<u16>(p + 8));
                let t = g.read::<u16>(at.wrapping_add(2));
                g.write::<u16>(p + 0xa, t);
            }
        }
    }
    // push 0, push G_O; call: the callee's frame
    let esp = ebp.wrapping_sub(0xe0);
    g.write::<u32>(esp + 4, w(0x185));
    g.write::<u32>(esp + 8, 0);
    let link = Frame {
        at: img.rel32_target(f + 0x18c),
        esp,
        ecx: p,
        edx: at,
        ..*fr
    };
    link_part(g, img, &link)
}

/// [`keyed_part_step`] (the `rel32`s: [`link_part`] and
/// `__security_check_cookie`).
pub const KEYED_PART_STEP: Sig = Sig {
    len: 419,
    hash: 0x0c17_279d_cae8_af64,
    calls: &[(0x18c, Callee::Sig(&LINK_PART)), (0x19b, Callee::Any)],
    imports: &[],
};

/// [`reset_state`] with `+0x11c` = 0x1e (`magic_0100` 0x10002080, 273
/// bytes; 4 DLLs).
pub const RESET_STATE_1E: Sig = Sig {
    len: 273,
    hash: 0x6117_a5df_89ed_a129,
    calls: &[],
    imports: &[],
};

/// [`give_back`] as the overlays inline it (the operands, in `f`'s
/// code: the cursor `cur` and its store `cur_store`, the head and tail
/// checks with their stores, the free ring's head `free` and its store
/// `free_store`, the u16 count `count` and its store `count_store`; the
/// chained children go to the pool's own [`give_back`], `rel32` at
/// `call`). Returns the `eax` the original leaves (the count in the low
/// half, the last word loaded above it).
fn inline_give_back<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    call: u32,
    [
        cur,
        head,
        head_store,
        tail,
        tail_store,
        free,
        count,
        free_store,
        count_store,
        cur_store,
    ]: [u32; 10],
) -> u32 {
    let w = |o: u32| op(img, f, o);
    let mut eax = g.read::<u32>(node + 8);
    if g.read::<u32>(eax.wrapping_add(0xc)) != 0 {
        loop {
            let child = g.read::<u32>(node + 8);
            let next = g.read::<u32>(child.wrapping_add(0xc));
            g.write::<u32>(child.wrapping_add(0xc), 0);
            let c = g.read::<u32>(node + 8);
            eax = give_back(g, img, img.rel32_target(f + call), c);
            g.write::<u32>(node + 8, next);
            if g.read::<u32>(next.wrapping_add(0xc)) == 0 {
                break;
            }
        }
    }
    let prev = g.read::<u32>(node);
    if prev != 0 {
        eax = g.read::<u32>(node + 4);
        g.write::<u32>(prev.wrapping_add(4), eax);
    }
    let next = g.read::<u32>(node + 4);
    if next != 0 {
        eax = g.read::<u32>(node);
        g.write::<u32>(next, eax);
    }
    let mut c = g.read::<u32>(w(cur));
    if c == node {
        c = g.read::<u32>(node + 4);
    }
    if g.read::<u32>(w(head)) == node {
        eax = g.read::<u32>(node);
        g.write::<u32>(w(head_store), eax);
    }
    if g.read::<u32>(w(tail)) == node {
        eax = g.read::<u32>(node + 4);
        g.write::<u32>(w(tail_store), eax);
    }
    let h = g.read::<u32>(w(free));
    if h == 0 {
        g.write::<u32>(node + 4, node);
        g.write::<u32>(node, node);
    } else {
        g.write::<u32>(node + 4, h);
        eax = g.read::<u32>(h);
        g.write::<u32>(node, eax);
        g.write::<u32>(h, node);
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    if n == 0x100 {
        c = 0;
    }
    g.write::<u32>(w(free_store), node);
    g.write::<u16>(w(count_store), n);
    g.write::<u32>(w(cur_store), c);
    (eax & 0xffff_0000) | u32::from(n)
}

/// An effect's four light directions set and its task node given back
/// (internal, `ecx` = the effect `E`): for `i` in 0..3 slot 590 `(i, L_i -
/// 0x30)` and slot 589 `(i, L_i)` (`L_i = E + 0x1e8 + 0x10 i`), slot 589
/// `(3, L_3)`; then the node `E+0x228`, if any, goes back to the pool
/// ([`inline_give_back`]) and `E+0x228 = 0`. Returns slot 589's `eax`, or
/// the give-back's.
///
/// `magic_0348` 0x10003210 (286 bytes, `ecx` = the effect; 4 DLLs).
pub fn lights_and_give_back<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let e = fr.ecx;
    for i in 0..3u32 {
        let l = e + 0x1e8 + 0x10 * i;
        slot(g, img, f, 0x15, 590, &[i, l - 0x30]);
        slot(g, img, f, 0x23, 589, &[i, l]);
    }
    let mut eax = slot(g, img, f, 0x4b, 589, &[3, e + 0x218]).eax;
    let node = g.read::<u32>(e + 0x228);
    if node != 0 {
        let ops = [
            0xa9, 0xb6, 0xbf, 0xc5, 0xcf, 0xd5, 0xef, 0x104, 0x10a, 0x110,
        ];
        eax = inline_give_back(g, img, f, node, 0x83, ops);
        g.write::<u32>(e + 0x228, 0);
    }
    eax
}

/// [`lights_and_give_back`] (the `rel32`: [`give_back`]).
pub const LIGHTS_AND_GIVE_BACK: Sig = Sig {
    len: 286,
    hash: 0x088e_7763_cadd_42f4,
    calls: &[(0x83, Callee::Sig(&GIVE_BACK))],
    imports: &[],
};

/// A point by the battle scene (internal, `ecx` = the point out `P`):
/// the scene (slot 520) looked up in the DLL's list `G_L` (words ended by
/// -1). Not listed: `P` is the actors' mean position ([`mean_position`]);
/// returns 0. Listed at index `j`: from the first entry on, the first of
/// the `*G_N` selected actors whose model (slot 390) is 0x1084 gives the
/// point (its battle position into `P` by slot 262, its option point
/// 0x10 by slot 268, `w` 1.0) and its index is returned; with none, or
/// for `j > 0`, the first selected actor's, returning 0. (An index of -1,
/// the effect's own character `*G_OWN`, cannot occur: the searches start
/// at 0.) Each call sees the words the earlier ones left on the stack.
///
/// `magic_0165` 0x10006750 (291 bytes, `ecx` = the point; 4 DLLs).
pub fn scene_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let p = fr.ecx;
    let scene = slot(g, img, f, 0x1, 520, &[]).eax;
    let list = w(0x1a);
    let mut j = 0u32;
    loop {
        let x = g.read::<u32>(list.wrapping_add(4 * j));
        if x == u32::MAX {
            mean_position(
                g,
                img,
                img.rel32_target(f + 0x36),
                fr.esp.wrapping_sub(0xc),
                p,
            );
            return 0;
        }
        if x == scene {
            break;
        }
        j += 1;
    }
    let sel = |g: &G, o: u32, i: u32| {
        let s = g.read::<u32>(w(o));
        u32::from(g.read::<u8>(s.wrapping_add(i)))
    };
    let point = |g: &mut G, ch: u32, at: (u32, u32)| {
        let mut s = Pushes::default();
        s.push_all(&[p + 8, p + 4, p, ch]);
        // call width: 12 (the most `Pushes::words` holds)
        s.call(g, crate::runtime::slot_target(g, w(at.0), 262), 0);
        s.push_all(&[p, 0x10, ch]);
        // call width: 12 (the most `Pushes::words` holds)
        s.call(g, crate::runtime::slot_target(g, w(at.1), 268), 0);
        g.write::<u32>(p + 0xc, 0x3f80_0000);
    };
    if j == 0 {
        let mut i = 0u32;
        while i.cast_signed() < g.read::<i32>(w(0x4a)) {
            let id = sel(g, 0x5d, i);
            if slot(g, img, f, 0x67, 390, &[id]).eax == 0x1084 {
                let id = sel(g, 0xdb, i);
                let ch = g.read::<u32>(w(0xe6).wrapping_add(4 * id));
                point(g, ch, (0xf3, 0x103));
                return i;
            }
            i += 1;
        }
    }
    let id = sel(g, 0x87, 0);
    let ch = g.read::<u32>(w(0x91).wrapping_add(4 * id));
    point(g, ch, (0x9e, 0xad));
    0
}

/// [`scene_point`] (the `rel32`: [`mean_position`]).
pub const SCENE_POINT: Sig = Sig {
    len: 291,
    hash: 0xf401_2dae_95b9_e5fc,
    calls: &[(0x36, Callee::Sig(&MEAN_POSITION))],
    imports: &[],
};

/// A ring request's sparks scattered (internal, `ecx` = the request
/// `Q`): the ring span reserved ([`ring_reserve`]); then for the two
/// pieces `j` (their record at `Q+0xc + 4j`, the count u16 `Q+6 + 2j`),
/// each of the count steps sets the piece's record `r` (the same one
/// each time, as the original does): `+0x10 = 0`, u16 `+0x12` a random
/// word (slot 324), `+0x18 = 0`, `+0x14 = K - (frnd - K1) K2`, `+0x1c =
/// K - (frnd - K1) K2`, `+0x20 = K3 - (frnd - K1) K4` (slot 307, ST0),
/// and its point a copy of the ring owner's `[Q[0]+0x38]+0x1c` (slot
/// 325). Returns 0, or [`ring_reserve`]'s value when the first piece is
/// empty.
///
/// `magic_0042` 0x10001ac0 (328 bytes, `ecx` = the request; 4 DLLs).
pub fn ring_sparks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let q = fr.ecx;
    let r0 = ring_reserve(g, q);
    if g.read::<u16>(q + 6) == 0 {
        return r0;
    }
    let st0 = |o: Out| crate::m0018::st0_f32(o.st0);
    for j in 0..2u32 {
        let r = g.read::<u32>(q + 0xc + 4 * j);
        let mut i = 0u32;
        while i < u32::from(g.read::<u16>(q + 6 + 2 * j)) {
            g.write::<u32>(r + 0x10, 0);
            let h = slot(g, img, f, 0x46, 324, &[]).eax;
            g.write::<u16>(r + 0x12, h as u16);
            let a = st0(slot(g, img, f, 0x57, 307, &[]));
            g.write::<u32>(r + 0x18, 0);
            let x = sub(k(g, 0x77), mul(sub(a, k(g, 0x6f)), k(g, 0x8b)));
            g.write::<f32>(r + 0x14, x);
            let a = st0(slot(g, img, f, 0x7c, 307, &[]));
            let y = sub(k(g, 0xb4), mul(sub(a, k(g, 0xac)), k(g, 0xc1)));
            g.write::<f32>(r + 0x1c, y);
            let a = st0(slot(g, img, f, 0xb9, 307, &[]));
            let z = sub(k(g, 0xea), mul(sub(a, k(g, 0xe2)), k(g, 0xf7)));
            g.write::<f32>(r + 0x20, z);
            let o = g
                .read::<u32>(g.read::<u32>(q).wrapping_add(0x38))
                .wrapping_add(0x1c);
            slot(g, img, f, 0x10c, 325, &[r, o]);
            i += 1;
        }
    }
    0
}

/// [`ring_sparks`] (the `rel32`: [`ring_reserve`]).
pub const RING_SPARKS: Sig = Sig {
    len: 328,
    hash: 0x53e7_76f1_0014_6be3,
    calls: &[(0xe, Callee::Sig(&RING_RESERVE))],
    imports: &[],
};

/// A smoke puff started from an emitter (internal, `ecx` = the emitter
/// `E`): an entry `S` of the DLL's ring ([`ring_take_68`]; none: returns
/// 0). `S[0]` = slot 719 `(D, 9, 0)` (`D` an immediate), u16 `+8 = -1`,
/// `+4 = 1`, u16 `+0xa = 0x200`, `+0xc..+0x18` the emitter's
/// `+0x24c..+0x258` (`+0x10` set to -0.5 unless below `K` or unordered),
/// `+0x1c = CB`; four random floats (slot 314 `(0.02)`, `(0.2)`, `(0.02)`,
/// `(0.01)`, ST0) give `+0x20 = (r1 + K1) E+0x25c`, `+0x24 = r2 - K2`,
/// `+0x28 = (r3 + K1) E+0x264` and `+0x30 = r4 + K3`; then the constants
/// `+0x2c = 0`, `+0x34 = 2`, `+0x38 = 0.9`, `+0x3c = -0.5`, `+0x40`,
/// `+0x44 = 0`, `+0x48 = 0.07`, `+0x4c`, `+0x50 = 0x10`, `+0x54 = 0`,
/// byte `+0x5f = 0`, u16 `+0x64 = 0`. Each call sees the words the
/// earlier ones left on the stack. Returns 0.
///
/// `magic_0284` 0x10002040 (408 bytes, `ecx` = the emitter; 4 DLLs).
pub fn smoke_puff<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let e = fr.ecx;
    let s = ring_take_68(g, img, img.rel32_target(f + 0x9));
    if s == 0 {
        return 0;
    }
    let mut st = Pushes::default();
    st.push_all(&[0, 9, w(0x28)]);
    // call width: 12 (the most `Pushes::words` holds)
    let d = st
        .call(g, crate::runtime::slot_target(g, w(0x19), 719), 0)
        .eax;
    st.pop(3);
    g.write::<u32>(s, d);
    g.write::<u16>(s + 8, 0xffff);
    g.write::<u32>(s + 4, 1);
    g.write::<u16>(s + 0xa, 0x200);
    for o in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(e + 0x24c + o);
        g.write::<u32>(s + 0xc + o, v);
    }
    let x = g.read::<f32>(s + 0x10);
    // comiss x, K; jb: kept below `K` or unordered
    if !(x < k(g, 0x73) || x.is_nan()) {
        g.write::<u32>(s + 0x10, 0xbf00_0000);
    }
    g.write::<u32>(s + 0x1c, w(0x8c));
    let frnd = |g: &mut G, st: &mut Pushes, t: u32, a: u32| {
        // `mov [esp], a`: the pushed word replaced
        st.set_top(a);
        // call width: 12 (the most `Pushes::words` holds)
        let o = st.call(g, crate::runtime::slot_target(g, w(t), 314), 0);
        crate::m0018::st0_f32(o.st0)
    };
    // push ecx (slot 719's target, then replaced)
    st.push(crate::runtime::slot_target(g, w(0x19), 719));
    let r1 = frnd(g, &mut st, 0x84, 0x3ca3_d70a);
    g.write::<f32>(s + 0x20, mul(add(r1, k(g, 0xab)), g.read::<f32>(e + 0x25c)));
    let r2 = frnd(g, &mut st, 0xb0, 0x3e4c_cccd);
    g.write::<f32>(s + 0x24, sub(r2, k(g, 0xdc)));
    let r3 = frnd(g, &mut st, 0xe1, 0x3ca3_d70a);
    g.write::<u32>(s + 0x2c, 0);
    g.write::<f32>(
        s + 0x28,
        mul(add(r3, k(g, 0x105)), g.read::<f32>(e + 0x264)),
    );
    let r4 = frnd(g, &mut st, 0x10a, 0x3c23_d70a);
    for (o, v) in [(0x34u32, 0x4000_0000u32), (0x38, 0x3f66_6666)] {
        g.write::<u32>(s + o, v);
    }
    g.write::<f32>(s + 0x30, add(r4, k(g, 0x13d)));
    for (o, v) in [
        (0x3cu32, 0xbf00_0000u32),
        (0x40, 0),
        (0x44, 0),
        (0x48, 0x3d8f_5c29),
        (0x4c, 0x10),
        (0x50, 0x10),
        (0x54, 0),
    ] {
        g.write::<u32>(s + o, v);
    }
    g.write::<u8>(s + 0x5f, 0);
    g.write::<u16>(s + 0x64, 0);
    0
}

/// [`smoke_puff`] (the `rel32`: [`ring_take_68`]).
pub const SMOKE_PUFF: Sig = Sig {
    len: 408,
    hash: 0xb3a0_0e1f_220f_5cd1,
    calls: &[(0x9, Callee::Sig(&RING_TAKE_68))],
    imports: &[],
};

/// [`move_ends`] for the DLL's object (internal: `edx` = `from`, the
/// stack `(to)`): the record `[G_O, from, to, 0]` in a local. Returns
/// [`move_ends`]'s `eax`.
///
/// `magic_0070` 0x10009450 (41 bytes; 2 DLLs).
pub fn move_ends_of<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let ebp = fr.esp.wrapping_sub(4);
    let rec = ebp.wrapping_sub(0x10);
    let to = fr.arg(g, 0);
    for (i, v) in (0u32..).zip([op(img, f, 0xf), fr.edx, to, 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let call = Frame {
        at: img.rel32_target(f + 0x21),
        esp: ebp.wrapping_sub(0x14),
        ecx: rec,
        ..*fr
    };
    Ret::eax(move_ends(g, img, &call).eax)
}

/// [`move_ends_of`] (the `rel32`: [`move_ends`]).
pub const MOVE_ENDS_OF: Sig = Sig {
    len: 41,
    hash: 0x0f66_d207_2050_1d3a,
    calls: &[(0x21, Callee::Sig(&MOVE_ENDS))],
    imports: &[],
};

/// Whether a model takes the direct way (internal, `ecx` = the user's
/// handle; its model is slot 390's): models
/// 0x1071..0x108c the DLL's byte table maps to the first way are; the
/// others are when slot 534 `(handle)` says so (0 or 1). Returns the
/// answer.
///
/// `magic_0094` 0x10006420 (112 bytes with its tables, `ecx` = the
/// handle; 2 DLLs).
pub fn model_direct<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let m = fr.ecx;
    let id = slot(g, img, f, 0x1, 390, &[m]).eax;
    let i = id.wrapping_add(0xffff_ef8f);
    // the switch's tables are in the code: the byte table, then the
    // jump table's two ways
    if i <= 0x1b {
        let b = img.word(w(0x21).wrapping_add(i)) & 0xff;
        if img.word(w(0x28).wrapping_add(4 * b)) == f + 0x2c {
            return Ret::eax(1);
        }
    }
    let r = slot(g, img, f, 0x34, 534, &[m]).eax;
    Ret::eax(u32::from(r != 0))
}

/// [`model_direct`].
pub const MODEL_DIRECT: Sig = Sig {
    len: 112,
    hash: 0x1969_315b_858e_f59a,
    calls: &[],
    imports: &[],
};

/// A summon's script task (cdecl `(task)`): the DLL's script runs
/// ([`run`]), the main motion script `G_M` and the scripts `G_S..G_E`
/// (0x20 apart) step ([`motion_script`]), and the three vectors
/// `B+0x19e0 + 0x10 i` are copied to `B+0x1920 + 0x40 i` (`B = *G_B`).
/// When the script and the main motion both ended (1): `B+0x1c08 = 1`,
/// the three actors shown ([`three_actors_shown`]) and the task node
/// given back ([`give_back`]), whose `eax` it returns; else the last
/// word copied.
///
/// `magic_0349` 0x100046c0 (148 bytes; 2 DLLs).
pub fn summon_script_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let b = g.read::<u32>(w(0x9));
    let ended = run(g, img, img.rel32_target(f + 0xe));
    let main = motion_script(g, img, img.rel32_target(f + 0x1b), w(0x13));
    let mut s = w(0x22);
    loop {
        motion_script(g, img, img.rel32_target(f + 0x29), s);
        s = s.wrapping_add(0x20);
        if s.cast_signed() >= w(0x32).cast_signed() {
            break;
        }
    }
    let mut eax = 0;
    for i in 0..3u32 {
        for o in [0u32, 4, 8, 0xc] {
            eax = g.read::<u32>(b + 0x19e0 + 0x10 * i + o);
            g.write::<u32>(b + 0x1920 + 0x40 * i + o, eax);
        }
    }
    if ended != 1 || main != 1 {
        return Ret::eax(eax);
    }
    g.write::<u32>(b + 0x1c08, 1);
    let shown = Frame {
        at: img.rel32_target(f + 0x81),
        esp: fr.esp.wrapping_sub(0x18),
        ..*fr
    };
    three_actors_shown(g, img, &shown);
    Ret::eax(give_back(g, img, img.rel32_target(f + 0x89), task))
}

/// [`summon_script_task`] (the `rel32`s: [`run`], two
/// [`motion_script`]s, [`three_actors_shown`] and [`give_back`]).
pub const SUMMON_SCRIPT_TASK: Sig = Sig {
    len: 148,
    hash: 0x9d26_d9a1_7db4_74e3,
    calls: &[
        (0xe, Callee::Sig(&RUN)),
        (0x1b, Callee::Sig(&MOTION_SCRIPT)),
        (0x29, Callee::Sig(&MOTION_SCRIPT)),
        (0x81, Callee::Sig(&THREE_ACTORS_SHOWN)),
        (0x89, Callee::Sig(&GIVE_BACK)),
    ],
    imports: &[],
};

/// A static part started (cdecl, no arguments): `*G_P = ITEM`, the first
/// program of slot 941 `(*G_D + 0x10)` started on the part `G_A` (an
/// immediate; [`start_part`], 0x1000, 1), and three 16-byte blocks of
/// the DLL's constants copied to `G_A + 0x40`, `+0x50`, `+0x60` (in the
/// original's order: `+0x40`, `+0x50`, `+0x60` from `K1`, `K2`, `K3`).
/// Returns the start's `eax`.
///
/// `magic_0293` 0x10001000 (101 bytes; 2 DLLs).
pub fn static_part_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    g.write::<u32>(g.read::<u32>(w(0x1)), w(0x7));
    let d = g.read::<u32>(g.read::<u32>(w(0xc)).wrapping_add(0x10));
    let table = slot(g, img, f, 0x14, 941, &[d]).eax;
    let program = g.read::<u32>(table);
    let eax = start_part(
        g,
        img,
        img.rel32_target(f + 0x31),
        w(0x2c),
        0x1000,
        program,
        1,
    );
    for (src, dst) in [(0x38u32, 0x3fu32), (0x47, 0x4f), (0x56, 0x60)] {
        let mut b = [0u8; 16];
        g.read_bytes(w(src), &mut b);
        g.write_bytes(w(dst), &b);
    }
    Ret::eax(eax)
}

/// [`static_part_start`] (the `rel32`: [`start_part`]).
pub const STATIC_PART_START: Sig = Sig {
    len: 101,
    hash: 0x1655_1fdc_5051_f331,
    calls: &[(0x31, Callee::Sig(&START_PART))],
    imports: &[],
};

/// A summon's bone walk and its tail (cdecl `(c)`, `c` a walk cursor:
/// `+0` the node `N`, `+4` the "moved" byte, `+5` the pass; an aligned
/// /GS frame): the bone step ([`bone_plain`]); when moved, the four
/// nodes `N+0x164`, `+0x168`, `+0x15c`, `+0x160` walk with a local cursor
/// (their method `+0x1c`), the point `P` = `N[0x168]+0x1c` through
/// `N[0x160]+0x9c` (slot 229, a local) goes to slot 284 `(N[0x170], 1,
/// P)`; with bit 0 of `N+0x178` the offset `(0, 0, N+0x174, 1)` turned
/// by `N+0x9c` (slot 229) plus `P` (slot 248), else `N[0x164]+0x1c`
/// through `N[0x15c]+0x9c` (slot 229), is the target `v`; `v - N+0x14c`
/// (slot 234) scaled by `N+0x16c` (slot 233) moves `N+0x14c` (slot 248),
/// and slot 284 `(N[0x170], 0, N+0x14c)`. Each call sees the words the
/// earlier ones left on the stack. Returns slot 284's `eax`, or the bone
/// step's when not moved.
///
/// `magic_0042` 0x10003b30 (512 bytes; 4 DLLs).
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn bone_walk_tail<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let c = fr.arg(g, 0);
    // push ebp; and esp, -8; sub esp, 0x30; push esi, edi
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let bone = Frame {
        at: img.rel32_target(f + 0x1c),
        esp: s.wrapping_sub(4),
        ecx: c,
        ..*fr
    };
    let eax = bone_plain(g, img, &bone).eax;
    if g.read::<u8>(c + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(c);
    let (cur, v, p) = (s + 0xc, s + 0x14, s + 0x24);
    let walk = |g: &mut G, node_at: u32| {
        let node = g.read::<u32>(n + node_at);
        g.write::<u32>(cur, node);
        let m = g.read::<u32>(g.read::<u32>(node).wrapping_add(0x1c));
        g.call(m, cur, &[cur]);
    };
    let pass = g.read::<u8>(c + 5);
    g.write::<u8>(cur + 5, pass);
    walk(g, 0x164);
    walk(g, 0x168);
    let pass = g.read::<u8>(c + 5);
    g.write::<u8>(cur + 5, pass);
    walk(g, 0x15c);
    walk(g, 0x160);
    let field = |g: &G, at: u32, o: u32| g.read::<u32>(n + at).wrapping_add(o);
    let (a, b) = (field(g, 0x160, 0x9c), field(g, 0x168, 0x1c));
    slot(g, img, f, 0xbc, 229, &[p, a, b]);
    let h = g.read::<u32>(n + 0x170);
    slot(g, img, f, 0xd1, 284, &[h, 1, p]);
    if g.read::<u8>(n + 0x178) & 1 != 0 {
        for (o, x) in [
            (0u32, 0u32),
            (4, 0),
            (8, g.read::<u32>(n + 0x174)),
            (0xc, 0x3f80_0000),
        ] {
            g.write::<u32>(v + o, x);
        }
        slot(g, img, f, 0x11b, 229, &[v, n + 0x9c, v]);
        slot(g, img, f, 0x148, 248, &[v, p, v]);
    } else {
        let (a, b) = (field(g, 0x15c, 0x9c), field(g, 0x164, 0x1c));
        slot(g, img, f, 0x170, 229, &[v, a, b]);
    }
    let e = n + 0x14c;
    slot(g, img, f, 0x18d, 234, &[v, v, e]);
    let k = g.read::<u32>(n + 0x16c);
    slot(g, img, f, 0x1b0, 233, &[v, v, k]);
    slot(g, img, f, 0x1c5, 248, &[e, e, v]);
    let h = g.read::<u32>(n + 0x170);
    Ret::eax(slot(g, img, f, 0x1d4, 284, &[h, 0, e]).eax)
}

/// [`bone_walk_tail`] (the `rel32`s: [`bone_plain`] and
/// `__security_check_cookie`).
pub const BONE_WALK_TAIL: Sig = Sig {
    len: 512,
    hash: 0x0844_8b5f_1bf6_4a9b,
    calls: &[(0x1c, Callee::Sig(&BONE)), (0x1f8, Callee::Any)],
    imports: &[],
};

/// A particle's drift set from its current record and three random
/// numbers (cdecl `(p)`; `R = p[0x2c + 0x10 p[0xac]]` the record, `D` the
/// divisor constant): `p+0x108 = (r1 / D) K1`, the flags `+0xbc |=
/// 0x4000`, `+0x130..+0x138 = (r2 / D) K2 + K2`; the record's point
/// `+0xd0..+0xd8` copied, its `+0xe0`, `+0xe8` times `K3` and `+0xe4 = R+0xe4
/// K3 - ((r3 / D) K4 + K4)`; `+0xdc`, `+0xec`, `+0x100`, `+0x104`,
/// `+0x10c`, `+0x13c`, `+0x14c` cleared and `+0x140..+0x148 = 0.008`
/// (`r1..r3` from `rand`). Then slot 458 `(*G_X, p, 0)`, slot 450 `(p,
/// CB)` and slot 451 `(p, 0x14)`, each seeing the words the earlier ones
/// left on the stack; the argument word is overwritten with the scaled
/// `R+0xe4`. Returns slot 451's `eax`.
///
/// `magic_0191` 0x10001720 (530 bytes; 3 DLLs).
pub fn particle_drift<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let p = fr.arg(g, 0);
    let idx = g.read::<u32>(p + 0xac);
    let r = g.read::<u32>(p.wrapping_add(idx.wrapping_mul(0x10)).wrapping_add(0x2c));
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let unit = |g: &mut G, d: u32| div(g.rand().cast_signed() as f32, k(g, d));
    let x1 = mul(unit(g, 0x2b), k(g, 0x33));
    let u2 = unit(g, 0x53);
    let fl = g.read::<u32>(p + 0xbc) | 0x4000;
    g.write::<u32>(p + 0xbc, fl);
    let y = add(mul(u2, k(g, 0x63)), k(g, 0x7b));
    let pt = [0xd0u32, 0xd4, 0xd8].map(|o| g.read::<u32>(r + o));
    let k3 = k(g, 0xaf);
    let [e0, e4, e8] = [0xe0u32, 0xe4, 0xe8].map(|o| mul(g.read::<f32>(r + o), k3));
    // the scaled +0xe4 is kept over the argument word (`movss [ebp+8]`)
    g.write::<f32>(fr.esp + 4, e4);
    let z = add(mul(unit(g, 0xe0), k(g, 0xed)), k(g, 0x107));
    for (o, v) in [0xd0u32, 0xd4, 0xd8].into_iter().zip(pt) {
        g.write::<u32>(p + o, v);
    }
    g.write::<f32>(p + 0xe0, e0);
    g.write::<f32>(p + 0xe8, e8);
    g.write::<f32>(p + 0x108, x1);
    g.write::<u32>(p + 0xdc, 0);
    g.write::<f32>(p + 0xe4, sub(e4, z));
    for o in [0xecu32, 0x100, 0x104, 0x10c] {
        g.write::<u32>(p + o, 0);
    }
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<f32>(p + o, y);
    }
    g.write::<u32>(p + 0x13c, 0);
    for o in [0x140u32, 0x144, 0x148] {
        g.write::<u32>(p + o, 0x3c03_126f);
    }
    g.write::<u32>(p + 0x14c, 0);
    let target = |g: &G, t: u32, n: u32| crate::runtime::slot_target(g, w(t), n);
    let mut st = Pushes::default();
    let x = g.read::<u32>(w(0x155));
    st.push_all(&[0, p, x]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0xf2, 458), 0);
    st.push_all(&[w(0x1e6), p]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x1ec, 450), 0);
    st.push_all(&[0x14, p]);
    // call width: 12 (the most `Pushes::words` holds)
    Ret::eax(st.call(g, target(g, 0x1f9, 451), 0).eax)
}

/// [`particle_drift`] (the imports: `rand`, through a register).
pub const PARTICLE_DRIFT: Sig = Sig {
    len: 530,
    hash: 0x5b48_823f_5a87_679c,
    calls: &[],
    imports: &[(0x16, "rand")],
};

/// The key `i` of a sorted time table `T` (`n` keys) for the time `x`:
/// the binary search of the overlays (`lo = 0`, `hi = n - 1`; `x > T[mid]`
/// moves `lo` past `mid`, anything else, unordered too, moves `hi` to it),
/// then the key before (key 0 for `lo` 0).
fn spline_key<G: Game>(g: &G, times: u32, n: i32, x: f32) -> u32 {
    let (mut lo, mut hi) = (0i32, n - 1);
    while lo < hi {
        // (lo + hi) / 2 as the original computes it (cdq; sub; sar: toward 0)
        let mid = lo.wrapping_add(hi) / 2;
        let t = g.read::<f32>(times.wrapping_add(4 * mid.cast_unsigned()));
        if x > t {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo > 0 { (lo - 1).cast_unsigned() } else { 0 }
}

/// A cubic Hermite spline of three coordinates sampled at a time
/// (internal: `ecx` = the point out, `edx` = the keys (four floats each),
/// `xmm0` = the time `x`, the stack `(T, D0, D1, D2, n)`: the key times
/// and the three coordinates' tangents). For each coordinate `c` the key
/// `i` ([`spline_key`]) gives `dt = x - T[i]`, `h = T[i+1] - T[i]`, `a =
/// Dc[i]`, `b = Dc[i+1]`, and the value `P[i] + dt ((P[i+1] - P[i]) / h -
/// (a K1 + b) h + dt (dt (b - a) / h + a K2))` (`K1`, `K2` the DLL's
/// constants; the second coordinate adds its terms the other way round).
/// The point's `w` is 1.0. Returns the point out.
///
/// `magic_0432` 0x10004cc0 (559 bytes; 3 DLLs).
pub fn spline_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let k = |g: &G, o: u32| g.read::<f32>(op(img, f, o));
    let (out, keys) = (fr.ecx, fr.edx);
    let x = fr.xmm_f32(0);
    let times = fr.arg(g, 0);
    let n = fr.arg(g, 4).cast_signed();
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    for (c, (dk, k1, k2)) in (0u32..).zip([
        (1u32, 0x74u32, 0x94u32),
        (2, 0x132, 0x124),
        (3, 0x1db, 0x1f2),
    ]) {
        let d = fr.arg(g, dk);
        let i = spline_key(g, times, n, x);
        let ti = times.wrapping_add(4 * i);
        let dt = sub(x, rd(g, ti));
        let h = sub(rd(g, ti + 4), rd(g, ti));
        let (a, b) = (
            rd(g, d.wrapping_add(4 * i)),
            rd(g, d.wrapping_add(4 * i + 4)),
        );
        let p0 = rd(g, keys.wrapping_add(0x10 * i + 4 * c));
        let p1 = rd(g, keys.wrapping_add(0x10 * i + 0x10 + 4 * c));
        let lin = mul(add(mul(a, k(g, k1)), b), h);
        let s = sub(div(sub(p1, p0), h), lin);
        let q = mul(add(div(mul(sub(b, a), dt), h), mul(a, k(g, k2))), dt);
        let v = if c == 1 { add(q, s) } else { add(s, q) };
        let v = add(mul(v, dt), p0);
        if c == 2 {
            g.write::<u32>(out + 0xc, 0x3f80_0000);
        }
        g.write::<f32>(out + 4 * c, v);
    }
    out
}

/// [`spline_point`].
pub const SPLINE_POINT: Sig = Sig {
    len: 559,
    hash: 0xa20d_d045_e524_971c,
    calls: &[],
    imports: &[],
};

/// A homing part's task (cdecl `(task)`, `task[5]` the object `O`; an
/// aligned /GS frame). While `O+0x27c` is positive it counts down. Else
/// the way `v` (a local) is the position `O+0xc` (count 0) or the
/// position moved by the speed `O+0x25c` (slot 248), less the target
/// `O+0x24c` (slot 234). Farther than `O+0x274`: the count -1 the first
/// time, else the position moves on (slot 248); the callback `O+0x26c`
/// `(O)`; unless bit 1 of `O+0x284`, the part keyed with a unit matrix
/// ([`keyed_part_step`], `ecx` = `O`). Within it (or unordered): the
/// position is the target, the callback `O+0x270` `(O)`, and without bit
/// 0 of `O+0x284` the flags get bit 15 and the task node goes back
/// ([`give_back`]); with it `task[2]+0x10 = CB`. Then with bit 0 the
/// trail is drawn ([`trail_draw`]: `ecx` = `O+0x24`, a matrix scaled by
/// `O+0x278`, the position). Returns what the original leaves in `eax`.
///
/// `magic_0180` 0x100027e0 (567 bytes; 5 DLLs).
#[allow(clippy::too_many_lines)] // the original's branches
pub fn homing_part_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 0x14);
    let n = g.read::<i32>(o + 0x27c);
    if n > 0 {
        g.write::<i32>(o + 0x27c, n - 1);
        return Ret::eax((n - 1).cast_unsigned());
    }
    // push ebp; and esp, -8; sub esp, 0x5c; three pushes
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x68);
    let (v, m) = (s + 0x10, s + 0x20);
    let pos = o + 0xc;
    if n == 0 {
        for k in [0u32, 4, 8, 0xc] {
            let x = g.read::<u32>(pos + k);
            g.write::<u32>(v + k, x);
        }
    } else {
        slot(g, img, f, 0x80, 248, &[v, pos, o + 0x25c]);
    }
    slot(g, img, f, 0x9d, 234, &[v, v, o + 0x24c]);
    let [x, y, z] = [0u32, 4, 8].map(|k| g.read::<f32>(v + k));
    let q = add(add(mul(y, y), mul(x, x)), mul(z, z));
    let d = f64::from(q).sqrt() as f32;
    let mut eax;
    if d > g.read::<f32>(o + 0x274) {
        if g.read::<u32>(o + 0x27c) == 0 {
            g.write::<u32>(o + 0x27c, u32::MAX);
        } else {
            slot(g, img, f, 0x108, 248, &[pos, pos, o + 0x25c]);
        }
        eax = g.read::<u32>(o + 0x26c);
        if eax != 0 {
            eax = g.call(eax, 0, &[o]).eax;
        }
        if g.read::<u8>(o + 0x284) & 2 == 0 {
            slot(g, img, f, 0x13c, 235, &[m]);
            for k in [0x28u32, 0x14, 0] {
                g.write::<u32>(m + k, 0x3f80_0000);
            }
            let key = Frame {
                at: img.rel32_target(f + 0x167),
                esp: s.wrapping_sub(8),
                ecx: o,
                edx: m,
                ..*fr
            };
            eax = keyed_part_step(g, img, &key);
        }
    } else {
        for k in [0u32, 4, 8, 0xc] {
            let x = g.read::<u32>(o + 0x24c + k);
            g.write::<u32>(pos + k, x);
        }
        let cb = g.read::<u32>(o + 0x270);
        if cb != 0 {
            g.call(cb, 0, &[o]);
        }
        let fl = g.read::<u16>(o + 0x284);
        if fl & 1 == 0 {
            g.write::<u16>(o + 0x284, fl | 0x8000);
            return Ret::eax(give_back(g, img, img.rel32_target(f + 0x221), task));
        }
        eax = g.read::<u32>(task + 8);
        g.write::<u32>(eax.wrapping_add(0x10), w(0x1b4));
    }
    if g.read::<u8>(o + 0x284) & 1 == 0 {
        return Ret::eax(eax);
    }
    slot(g, img, f, 0x1c7, 235, &[m]);
    let sc = g.read::<u32>(o + 0x278);
    for k in [0x28u32, 0x14, 0] {
        g.write::<u32>(m + k, sc);
    }
    // push edi; call: the callee's frame
    let esp = s.wrapping_sub(8);
    g.write::<u32>(esp + 4, pos);
    let draw = Frame {
        at: img.rel32_target(f + 0x1f9),
        esp,
        ecx: o + 0x24,
        edx: m,
        ..*fr
    };
    Ret::eax(trail_draw(g, img, &draw))
}

/// [`homing_part_task`] (the `rel32`s: [`keyed_part_step`], the square
/// root, [`trail_draw`], [`give_back`] and two `__security_check_cookie`s).
pub const HOMING_PART_TASK: Sig = Sig {
    len: 567,
    hash: 0xb936_3962_a71f_e9b5,
    calls: &[
        (0x38, Callee::Any),
        (0xd6, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x167, Callee::Sig(&KEYED_PART_STEP)),
        (0x1f9, Callee::Sig(&TRAIL_DRAW)),
        (0x20a, Callee::Any),
        (0x221, Callee::Sig(&GIVE_BACK)),
        (0x22f, Callee::Any),
    ],
    imports: &[],
};

/// The camera tracks of a summon set from its script record (cdecl, no
/// arguments; `R = *G_R`, `d = R+0x24 - (i16) *G_T` the frames left, `1`
/// the constant `K1`). Track 1: `G_A = R[0]`, `G_B = 0`, `G_C = R[4]`,
/// `G_D = d`. Without `R+0x10`: its rate `1 / (d / R[4])` with kind 3;
/// track 2 `R[8]`, 0, the rate `1 / (d / R[0xc])`, `d`, `R[0xc]`, kind 3;
/// track 3 from `R+0x14` by `(R+0x18 - R+0x14) / d`, `R+0x10`, kind 1.
/// With it: track 1 also `R+0x10` and the rate `1 / d`, kind 4; track 2
/// `R[8]`, `R[0xc]`, `R+0x10`, 0, `1 / d`, `d`, kind 4; track 3 from the
/// record `R+0x14` (`R+0x10`, 0, `1 / d`, kinds 2 and 3, `2d`). Then the
/// angle track from `a = R+0x1c` to `b = R+0x20`, the end taken the
/// short way round (`b - K2 + K3` for `a < 0 < b` farther than `K2`,
/// `b + 2 K2` for `b < 0 <= a` likewise), `d` twice and on. Returns
/// `d`.
///
/// `magic_0515` 0x10003f70 (598 bytes; 4 DLLs).
#[allow(clippy::too_many_lines)] // the original's tracks, word by word
pub fn camera_tracks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let one = k(g, 0x4);
    let base = i32::from(g.read::<i16>(w(0xc)));
    let r = g.read::<u32>(w(0x14));
    let rd = |g: &G, o: u32| g.read::<u32>(r.wrapping_add(o));
    let left = |g: &G| rd(g, 0x24).wrapping_sub(base.cast_unsigned());
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let fl = |x: u32| x.cast_signed() as f32;
    let d = left(g);
    let flag = rd(g, 0x10);
    let set = |g: &mut G, o: u32, v: u32| g.write::<u32>(w(o), v);
    set(g, 0x2d, rd(g, 0));
    set(g, 0x33, 0);
    set(g, 0x3d, rd(g, 4));
    set(g, 0x43, d);
    let eax = if flag == 0 {
        set(g, 0x5b, 3);
        let rate = div(one, div(fl(d), fl(rd(g, 4))));
        g.write::<f32>(w(0x72), rate);
        let d = left(g);
        let c = rd(g, 0xc);
        let x = div(fl(d), fl(c));
        set(g, 0x94, rd(g, 8));
        set(g, 0x9e, flag);
        g.write::<f32>(w(0xa6), div(one, x));
        set(g, 0xac, d);
        set(g, 0xb2, c);
        set(g, 0xb8, 3);
        let (a, b) = (
            g.read::<f32>(r.wrapping_add(0x14)),
            g.read::<f32>(r.wrapping_add(0x18)),
        );
        let d = left(g);
        set(g, 0xd7, a.to_bits());
        set(g, 0xe4, flag);
        set(g, 0xee, 1);
        g.write::<f32>(w(0xfa), div(sub(b, a), fl(d)));
        d
    } else {
        g.write::<f32>(w(0x114), div(one, fl(d)));
        set(g, 0x10c, flag);
        set(g, 0x11a, 4);
        let d = left(g);
        let (c, fl10, e8) = (rd(g, 0xc), rd(g, 0x10), rd(g, 8));
        set(g, 0x13f, e8);
        set(g, 0x145, c);
        set(g, 0x14b, fl10);
        set(g, 0x151, 0);
        g.write::<f32>(w(0x15d), div(one, fl(d)));
        set(g, 0x163, d);
        set(g, 0x169, 4);
        let d = left(g);
        let fl10 = rd(g, 0x10);
        set(g, 0x188, r.wrapping_add(0x14));
        set(g, 0x18e, fl10);
        set(g, 0x194, 0);
        g.write::<f32>(w(0x1a0), div(one, fl(d)));
        set(g, 0x1a9, 2);
        set(g, 0x1b3, 3);
        d.wrapping_mul(2)
    };
    set(g, 0x1bc, eax);
    let (a, b) = (
        g.read::<f32>(r.wrapping_add(0x1c)),
        g.read::<f32>(r.wrapping_add(0x20)),
    );
    let d = left(g);
    // comiss: each test false when unordered
    let end = if 0.0 > a {
        let kk = k(g, 0x1e3);
        if b > 0.0 && sub(b, a) > kk {
            sub(k(g, 0x1f7), sub(kk, b))
        } else {
            b
        }
    } else {
        let kk = k(g, 0x20e);
        if 0.0 > b && sub(a, b) > kk {
            add(add(b, kk), kk)
        } else {
            b
        }
    };
    g.write::<f32>(w(0x22d), end);
    g.write::<f32>(w(0x235), a);
    g.write::<f32>(w(0x23d), b);
    set(g, 0x242, d);
    set(g, 0x247, d);
    set(g, 0x24d, 1);
    Ret::eax(d)
}

/// [`camera_tracks`] (the `jmp rel32` inside stays in the hash).
pub const CAMERA_TRACKS: Sig = Sig {
    len: 598,
    hash: 0x78db_7a9c_b5b3_52d5,
    calls: &[],
    imports: &[],
};

/// A level look matrix (internal: `ecx` = the matrix out, `edx` = a
/// source, `xmm2` = a height, `xmm3` = a scale; a /GS frame): a matrix
/// `A` from the source (slot 236 `(edx, 3, A)`, a local) with its three
/// rows normalised (slot 232); its second row flattened (`y = 0`, slot
/// 325, normalised into `N`), `C = up x N`, `R = N x C` (slot 266, `up`
/// = `(0, 1, 0, 1)`), both normalised; the rotation `M = [C, R, N, A's
/// fourth row with y = height]` (`w`s 0, the last 1; slot 325 copies the
/// row) and a scale matrix `S` (slot 235, diagonal `xmm3`) give the
/// matrix out (slot 231 `(out, M, S)`). Each call sees the words the
/// earlier ones left on the stack. Returns slot 231's `eax`.
///
/// `magic_0045` 0x10001000 (579 bytes, `ecx`/`edx`/`xmm2`/`xmm3`; 3
/// DLLs).
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn level_look_matrix<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let out = fr.ecx;
    let (height, scale) = (fr.xmm[2] as u32, fr.xmm[3] as u32);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let a = at(0xd4);
    let (p, n, c, r, up, m, s) = (
        at(0x94),
        at(0x24),
        at(0x14),
        at(0x34),
        at(0x44),
        at(0x84),
        at(0x114),
    );
    let target = |g: &G, t: u32, k: u32| crate::runtime::slot_target(g, w(t), k);
    let mut st = Pushes::default();
    let call = |g: &mut G, st: &mut Pushes, words: &[u32], t: u32, k: u32| {
        st.push_all(words);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, target(g, t, k), 0).eax
    };
    call(g, &mut st, &[a, 3, fr.edx], 0x1c, 236);
    for (row, t) in [(0u32, 0x46u32), (0x10, 0x5b), (0x20, 0x70)] {
        call(g, &mut st, &[a + row, a + row], t, 232);
    }
    call(g, &mut st, &[a + 0x10, p], 0x8b, 325);
    g.write::<u32>(p + 4, 0);
    for (o, v) in [(0u32, 0u32), (4, 0x3f80_0000), (8, 0), (0xc, 0x3f80_0000)] {
        g.write::<u32>(up + o, v);
    }
    call(g, &mut st, &[p, n], 0xa3, 232);
    call(g, &mut st, &[n, up, c], 0xe2, 266);
    st.pop(16);
    call(g, &mut st, &[c, n, r], 0xfe, 266);
    call(g, &mut st, &[c, c], 0x110, 232);
    call(g, &mut st, &[r, r], 0x122, 232);
    for (row, src) in [(0u32, c), (0x10, r), (0x20, n)] {
        for o in [0u32, 4, 8] {
            let x = g.read::<u32>(src + o);
            g.write::<u32>(m + row + o, x);
        }
        g.write::<u32>(m + row + 0xc, 0);
    }
    for (o, v) in [(0x30u32, 0u32), (0x34, 0), (0x38, 0), (0x3c, 0x3f80_0000)] {
        g.write::<u32>(m + o, v);
    }
    call(g, &mut st, &[a + 0x30, m + 0x30], 0x192, 325);
    g.write::<u32>(m + 0x34, height);
    call(g, &mut st, &[s], 0x1e4, 235);
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(s + o, scale);
    }
    call(g, &mut st, &[s, m, out], 0x20c, 231)
}

/// [`level_look_matrix`] (the `rel32`: `__security_check_cookie`).
pub const LEVEL_LOOK_MATRIX: Sig = Sig {
    len: 579,
    hash: 0x651a_167a_e72b_1b92,
    calls: &[(0x23b, Callee::Any)],
    imports: &[],
};

/// A sprite's GS packet written (internal, `ecx` = the sprite `O`; `P =
/// O+0x34` the packet buffer): the GIF tag from bit 0 of `O+0x20` (as
/// [`sprite_packet`](crate::s5_misc::sprite_packet) builds it), the
/// register words from the colour byte `c = O+0x48` (`c + 6`, `c + 0x47`,
/// `c + 0x42`, `c + 0x14`, `c + 8`, each as a 64-bit word) and `O+0x24`,
/// the corners from `O+0x10` (`a`), `O+0x14` (`b`) and the zoom `z =
/// O+0x2c`: `(a - z a) K`, `(b - z b) K`, `((K1 - a) z + a) K`, `((K2 - b)
/// z + b) K`; the colour `O+0..+0xc`, the texture words from u16 `O+0x28`
/// (`n`, `n + 0x2000`) and the constants. The buffer pointer moves on by
/// 0x100, which it returns.
///
/// `magic_0202` 0x10001000 (607 bytes, `ecx` = the sprite; 2 DLLs).
#[allow(clippy::too_many_lines)] // the original's packet, word by word
pub fn sprite_gs_packet<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let k = |g: &G, o: u32| g.read::<f32>(op(img, f, o));
    let o = fr.ecx;
    let e = g.read::<u32>(o + 0x20) & 1;
    let p = g.read::<u32>(o + 0x34);
    let n = u32::from(g.read::<u16>(o + 0x28));
    let kk = k(g, 0x3e);
    let tag = ((u64::from(e | 0x2a80) << 6) | u64::from(e)) << 14;
    let set = |g: &mut G, at: u32, v: u32| g.write::<u32>(p.wrapping_add(at), v);
    set(g, 0x14, (tag >> 32) as u32);
    set(g, 0x10, tag as u32 | (e << 5));
    set(g, 0, 0);
    let c = u32::from(g.read::<u8>(o + 0x48));
    set(g, 4, 0);
    for (at, add_) in [(0x18u32, 6u32), (0x28, 0x47)] {
        set(g, at, c + add_);
        set(g, at + 4, 0);
    }
    let x24 = g.read::<u32>(o + 0x24);
    set(g, 0x30, x24);
    for (at, add_) in [(0x38u32, 0x42u32), (0x48, 0x14)] {
        set(g, at, c + add_);
        set(g, at + 4, 0);
    }
    set(g, 0x58, c + 8);
    set(g, 8, 0xeeeee);
    let (z, a, b) = (
        g.read::<f32>(o + 0x2c),
        g.read::<f32>(o + 0x10),
        g.read::<f32>(o + 0x14),
    );
    let x70 = mul(sub(a, mul(z, a)), kk);
    for (at, v) in [
        (0xcu32, 0u32),
        (0x20, 0),
        (0x24, 0),
        (0x34, 0),
        (0x40, 0x61),
        (0x44, 0),
        (0x50, 0),
        (0x54, 0),
        (0x5c, 0),
    ] {
        set(g, at, v);
    }
    let x74 = mul(sub(b, mul(z, b)), kk);
    set(g, 0x70, x70.to_bits());
    for (at, v) in [
        (0x60u32, 0u32),
        (0x64, 0),
        (0x68, 0x2424_2412),
        (0x6c, 4),
        (0x78, 0x3f80_0000),
    ] {
        set(g, at, v);
    }
    set(g, 0x74, x74.to_bits());
    for i in 0..4u32 {
        let v = g.read::<u32>(o + 4 * i);
        set(g, 0x80 + 4 * i, v);
    }
    let xa = mul(add(mul(sub(k(g, 0x14e), a), z), a), kk);
    let xb = mul(add(mul(sub(k(g, 0x15e), b), z), b), kk);
    let x74 = g.read::<u32>(p.wrapping_add(0x74));
    set(g, 0xa4, x74);
    let x70 = g.read::<u32>(p.wrapping_add(0x70));
    set(g, 0xb0, n + 0x2000);
    set(g, 0xc0, x70);
    set(g, 0xf0, n + 0x2000);
    for (at, v) in [
        (0x90u32, n),
        (0x94, 0),
        (0x98, 0),
        (0x9c, 0),
        (0xa0, xa.to_bits()),
        (0xb4, 0),
        (0xb8, 0),
        (0xbc, 0),
        (0xc4, xb.to_bits()),
        (0xd0, n),
        (0xd4, 0x1a00),
        (0xd8, 0),
        (0xdc, 0),
        (0xe0, xa.to_bits()),
        (0xe4, xb.to_bits()),
        (0xf4, 0x1a00),
        (0xf8, 0),
        (0xfc, 0),
    ] {
        set(g, at, v);
    }
    let next = p.wrapping_add(0x100);
    g.write::<u32>(o + 0x34, next);
    next
}

/// [`sprite_gs_packet`].
pub const SPRITE_GS_PACKET: Sig = Sig {
    len: 607,
    hash: 0x527d_8d97_9bc6_f91b,
    calls: &[],
    imports: &[],
};

/// [`particle_drift`]'s variant with five random numbers (cdecl `(p)`;
/// `R = p[0x2c + 0x10 p[0xac]]`, `D` the divisor constant): `p+0xd0..+0xd8`
/// the record's point (its `x` also kept over the argument word),
/// `+0xe0..+0xe8 = r / D - K1` (three draws), `+0x108 = (r4 / D) K2`, the
/// flags `+0xbc |= 0x4000`, `+0x130..+0x138 = (r5 / D) K3 + K4`;
/// `+0x17c = *G_S`, `+0x18c = -0.05` and the other words cleared; then
/// slots 458 `(*G_X, p, 1)`, 450 `(p, CB)` and 451 `(p, 0x14)`, each seeing
/// the words the earlier ones left on the stack. Returns slot 451's
/// `eax`. The 643 bytes of its identity hold [`particle_hand_over`] too.
///
/// `magic_0384` 0x10002060 (528 bytes; 3 DLLs).
#[allow(clippy::too_many_lines)] // the original's words, one by one
pub fn particle_drift_5<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let p = fr.arg(g, 0);
    let idx = g.read::<u32>(p + 0xac);
    let r = g.read::<u32>(p.wrapping_add(idx.wrapping_mul(0x10)).wrapping_add(0x2c));
    let pt = [0xd0u32, 0xd4, 0xd8].map(|o| g.read::<u32>(r.wrapping_add(o)));
    // `movss [ebp+8], x`: the argument word holds the point's x
    g.write::<u32>(fr.esp + 4, pt[0]);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let unit = |g: &mut G, d: u32| div(g.rand().cast_signed() as f32, k(g, d));
    let e0 = sub(unit(g, 0x51), k(g, 0x59));
    let e4 = sub(unit(g, 0x6f), k(g, 0x77));
    let e8 = sub(unit(g, 0x8d), k(g, 0x95));
    let x108 = mul(unit(g, 0xab), k(g, 0xb3));
    let u5 = unit(g, 0xd8);
    let fl = g.read::<u32>(p + 0xbc) | 0x4000;
    g.write::<u32>(p + 0xbc, fl);
    let y = add(mul(u5, k(g, 0xe5)), k(g, 0xfa));
    for (o, v) in [0xd0u32, 0xd4, 0xd8].into_iter().zip(pt) {
        g.write::<u32>(p + o, v);
    }
    for (o, v) in [(0xe0u32, e0), (0xe4, e4), (0xe8, e8), (0x108, x108)] {
        g.write::<f32>(p + o, v);
    }
    let s = g.read::<u32>(w(0x14b));
    for o in [0xdcu32, 0xec, 0x100, 0x104, 0x10c] {
        g.write::<u32>(p + o, 0);
    }
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<f32>(p + o, y);
    }
    g.write::<u32>(p + 0x13c, 0);
    g.write::<u32>(p + 0x17c, s);
    for o in [0x180u32, 0x184, 0x188] {
        g.write::<u32>(p + o, 0);
    }
    g.write::<u32>(p + 0x18c, 0xbd4c_cccd);
    let target = |g: &G, t: u32, n: u32| crate::runtime::slot_target(g, w(t), n);
    let mut st = Pushes::default();
    let x = g.read::<u32>(w(0x154));
    st.push_all(&[1, p, x]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0xdd, 458), 0);
    st.push_all(&[w(0x1e5), p]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x1eb, 450), 0);
    st.push_all(&[0x14, p]);
    // call width: 12 (the most `Pushes::words` holds)
    Ret::eax(st.call(g, target(g, 0x1f8, 451), 0).eax)
}

/// [`particle_drift_5`] with [`particle_hand_over`] after it.
pub const PARTICLE_DRIFT_5: Sig = Sig {
    len: 643,
    hash: 0xcdb8_42fc_edac_efc6,
    calls: &[],
    imports: &[(0x9, "rand")],
};

/// A particle handed over to the next record (cdecl `(p)`; `e = p[0x2c +
/// 0x10 p[0xac]]`, a record index): when the DLL's table `G_T[e]` is set,
/// slot 446 `(G_A[0x1a0 + 4e], 0)`, a new particle `q` (slot 434 `(*G_A,
/// CB)`) gets `e` as its record and `p[0] |= 4`; with `*G_F` set, `p[0] |=
/// 4` too. Each call sees the words the earlier ones left on the stack.
/// Returns `q`, or `2 p[0xac]`.
///
/// `magic_0384` 0x10002270 (115 bytes; 3 DLLs).
pub fn particle_hand_over<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let p = fr.arg(g, 0);
    let idx = g.read::<u32>(p + 0xac);
    let mut eax = idx.wrapping_mul(2);
    let e = g.read::<u32>(p.wrapping_add(idx.wrapping_mul(0x10)).wrapping_add(0x2c));
    if g.read::<u32>(w(0x17).wrapping_add(e.wrapping_mul(4))) != 0 {
        let a = g.read::<u32>(w(0x1f));
        let h = g.read::<u32>(a.wrapping_add(0x1a0).wrapping_add(e.wrapping_mul(4)));
        let target = |g: &G, t: u32, n: u32| crate::runtime::slot_target(g, w(t), n);
        let mut st = Pushes::default();
        st.push_all(&[0, h]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, target(g, 0x2d, 446), 0);
        let a = g.read::<u32>(w(0x45));
        st.push_all(&[w(0x3f), a]);
        // call width: 12 (the most `Pushes::words` holds)
        let q = st.call(g, target(g, 0x3a, 434), 0).eax;
        let qi = g.read::<u32>(q.wrapping_add(0xac));
        g.write::<u32>(q.wrapping_add(qi.wrapping_mul(0x10)).wrapping_add(0x2c), e);
        let v = g.read::<u32>(p) | 4;
        g.write::<u32>(p, v);
        eax = q;
    }
    if g.read::<u32>(w(0x65)) != 0 {
        let v = g.read::<u32>(p) | 4;
        g.write::<u32>(p, v);
    }
    Ret::eax(eax)
}

/// [`particle_hand_over`].
pub const PARTICLE_HAND_OVER: Sig = Sig {
    len: 115,
    hash: 0x8110_77ff_acea_e4c0,
    calls: &[],
    imports: &[],
};

/// A summon's three camera blocks reset (cdecl, no arguments): the
/// camera made (slot 287 `(8, 0, 7)` into `*G_C`), slot 281 `(1)` and
/// slot 283 `(*G_C, 0)`, `(*G_C, 1)`, each seeing the words the earlier
/// ones left on the stack; then the three blocks' words (u16, byte and
/// word stores at the DLL's globals, in the original's order: zeros, the
/// two scales 1.0 and the distance 1024 each) and `*G_D = 0`. Returns
/// the last slot 283's `eax`.
///
/// `magic_0515` 0x100012d0 (653 bytes; 5 DLLs).
pub fn camera_blocks_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let target = |g: &G, t: u32, n: u32| crate::runtime::slot_target(g, w(t), n);
    let mut st = Pushes::default();
    st.push_all(&[7, 0, 8]);
    // call width: 12 (the most `Pushes::words` holds)
    let c = st.call(g, target(g, 0x1, 287), 0).eax;
    g.write::<u32>(w(0x14), c);
    st.push(1);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x19, 281), 0);
    let mut eax = 0;
    for (t, n) in [(0x28u32, 0u32), (0x3d, 1)] {
        let c = g.read::<u32>(w(if n == 0 { 0x30 } else { 0x45 }));
        st.push_all(&[n, c]);
        // call width: 12 (the most `Pushes::words` holds)
        eax = st.call(g, target(g, t, 283), 0).eax;
    }
    for (o, size, v) in [
        (0x57u32, 2u8, 0x0u32),
        (0x5f, 1, 0x0),
        (0x66, 4, 0x0),
        (0x70, 4, 0x3f80_0000),
        (0x7a, 4, 0x3f80_0000),
        (0x84, 4, 0x0),
        (0x8e, 4, 0x0),
        (0x98, 4, 0x0),
        (0xa2, 4, 0x0),
        (0xac, 4, 0x0),
        (0xb6, 4, 0x0),
        (0xc0, 4, 0x0),
        (0xca, 4, 0x0),
        (0xd4, 4, 0x0),
        (0xde, 4, 0x0),
        (0xe8, 4, 0x0),
        (0xf2, 4, 0x0),
        (0xfc, 4, 0x0),
        (0x106, 4, 0x4480_0000),
        (0x111, 2, 0x0),
        (0x119, 1, 0x0),
        (0x120, 4, 0x0),
        (0x12a, 4, 0x3f80_0000),
        (0x134, 4, 0x3f80_0000),
        (0x13e, 4, 0x0),
        (0x148, 4, 0x0),
        (0x152, 4, 0x0),
        (0x15c, 4, 0x0),
        (0x166, 4, 0x0),
        (0x170, 4, 0x0),
        (0x17a, 4, 0x0),
        (0x184, 4, 0x0),
        (0x18e, 4, 0x0),
        (0x198, 4, 0x0),
        (0x1a2, 4, 0x0),
        (0x1ac, 4, 0x0),
        (0x1b6, 4, 0x0),
        (0x1c0, 4, 0x4480_0000),
        (0x1cb, 2, 0x0),
        (0x1d3, 1, 0x0),
        (0x1da, 4, 0x0),
        (0x1e4, 4, 0x3f80_0000),
        (0x1ee, 4, 0x3f80_0000),
        (0x1f8, 4, 0x0),
        (0x202, 4, 0x0),
        (0x20c, 4, 0x0),
        (0x216, 4, 0x0),
        (0x220, 4, 0x0),
        (0x22a, 4, 0x0),
        (0x234, 4, 0x0),
        (0x23e, 4, 0x0),
        (0x248, 4, 0x0),
        (0x252, 4, 0x0),
        (0x25c, 4, 0x0),
        (0x266, 4, 0x0),
        (0x270, 4, 0x0),
        (0x27a, 4, 0x4480_0000),
        (0x284, 4, 0x0),
    ] {
        match size {
            1 => g.write::<u8>(w(o), v as u8),
            2 => g.write::<u16>(w(o), v as u16),
            _ => g.write::<u32>(w(o), v),
        }
    }
    Ret::eax(eax)
}

/// [`camera_blocks_reset`].
pub const CAMERA_BLOCKS_RESET: Sig = Sig {
    len: 653,
    hash: 0x00fd_0e68_c041_258a,
    calls: &[],
    imports: &[],
};

/// A node aimed between three others (internal, `ecx` = the walk cursor
/// `C`: `+0` the node `N`, `+4` the "moved" byte, `+5` the pass; a /GS
/// frame). A node not yet in this pass walks its children first (their
/// method `+0x1c` with `C` on them), then takes the pass and is moved.
/// When moved: the nodes `N+0x13c`, `+0x150`, `+0x140` walk with a local
/// cursor; the way `v` from `N[0x150]+0x1c` to `N[0x13c]+0x1c` (slot 234)
/// is normalised and turned round (slots 232, 233 `-1`), eased toward the
/// old axis `N+0x3c` by `d = N+0x14c` (`v d + N+0x3c (K - d)`) and
/// normalised into it (slot 232); the axes `N+0x1c = up x N+0x3c` and
/// `N+0x2c = N+0x3c x N+0x1c` (slot 266, `up = (0, 1, 0, 1)`), the scale
/// `N+0x12c` a copy of `N[0x140]+0x1c` (slot 325) times `N+0x138` (slot
/// 233) scaling the three axes (slot 233), the matrix `N+0x9c` (slot 230)
/// and `N+0xcc` a copy of `N[0x13c]+0x1c` (slot 325). Each call sees the
/// words the earlier ones left on the stack. Returns slot 325's `eax`,
/// or the frame's cookie word with the pass byte in `al` when the node
/// was already in the pass.
///
/// `magic_0482` 0x10003670 (708 bytes, `ecx` = the cursor; 2 DLLs).
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn aimed_node_walk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let c = fr.ecx;
    let n = g.read::<u32>(c);
    let pass = g.read::<u8>(c + 5);
    let mine = g.read::<u8>(n + 0x1a);
    let ebp = fr.esp.wrapping_sub(4);
    if mine == pass {
        g.write::<u8>(c + 4, 0);
        let cookie = g.read::<u32>(w(0x7)) ^ ebp;
        return (cookie & !0xff) | u32::from(mine);
    }
    let mut e = g.read::<u32>(n + 0x10);
    while e != n {
        g.write::<u32>(c, e);
        let next = g.read::<u32>(e + 8);
        let m = g.read::<u32>(g.read::<u32>(g.read::<u32>(c)).wrapping_add(0x1c));
        g.call(m, c, &[c]);
        e = next;
    }
    let pass = g.read::<u8>(c + 5);
    g.write::<u32>(c, n);
    g.write::<u8>(n + 0x1a, pass);
    g.write::<u8>(c + 4, 1);
    let (lc, v) = (ebp.wrapping_sub(0x20), ebp.wrapping_sub(0x14));
    let target = |g: &G, t: u32, k: u32| crate::runtime::slot_target(g, w(t), k);
    let field = |g: &G, o: u32| g.read::<u32>(n + o).wrapping_add(0x1c);
    let mut st = Pushes::default();
    for o in [0x13cu32, 0x150, 0x140] {
        let node = g.read::<u32>(n + o);
        let pass = g.read::<u8>(c + 5);
        g.write::<u32>(lc, node);
        g.write::<u8>(lc + 5, pass);
        let m = g.read::<u32>(g.read::<u32>(node).wrapping_add(0x1c));
        st.push(lc);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, m, lc);
    }
    let (a, b) = (field(g, 0x13c), field(g, 0x150));
    st.push_all(&[a, b, v]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0xc6, 234), 0);
    st.push_all(&[v, v]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0xd8, 232), 0);
    st.pop(7);
    st.set_top(0xbf80_0000);
    st.push_all(&[v, v]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0xf4, 233), 0);
    let d = g.read::<f32>(n + 0x14c);
    let kd = sub(g.read::<f32>(w(0x10c)), d);
    let axis = n + 0x3c;
    for i in 0..4u32 {
        let x = add(
            mul(g.read::<f32>(axis + 4 * i), kd),
            mul(g.read::<f32>(v + 4 * i), d),
        );
        g.write::<f32>(v + 4 * i, x);
    }
    st.push_all(&[v, axis]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x188, 232), 0);
    st.push_all(&[axis, v]);
    for (o, x) in [(0u32, 0u32), (4, 0x3f80_0000), (8, 0), (0xc, 0x3f80_0000)] {
        g.write::<u32>(v + o, x);
    }
    let (n1c, n2c) = (n + 0x1c, n + 0x2c);
    st.push(n1c);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x19b, 266), 0);
    st.push_all(&[n1c, axis, n2c]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x1cb, 266), 0);
    let sc = n + 0x12c;
    let src = field(g, 0x140);
    st.push_all(&[src, sc]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x1f1, 325), 0);
    st.pop(12);
    st.set_top(g.read::<u32>(n + 0x138));
    st.push_all(&[sc, v]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x217, 233), 0);
    for (i, (t, row)) in [(0x229u32, n1c), (0x245, n2c), (0x271, axis)]
        .into_iter()
        .enumerate()
    {
        st.pop(2);
        st.set_top(g.read::<u32>(v + 4 * i as u32));
        st.push_all(&[row, row]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, target(g, t, 233), 0);
    }
    st.push_all(&[n1c, n + 0x9c]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x286, 230), 0);
    let src = field(g, 0x13c);
    st.push_all(&[src, n + 0xcc]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, target(g, 0x2a4, 325), 0).eax
}

/// [`aimed_node_walk`] (the `rel32`: `__security_check_cookie`).
pub const AIMED_NODE_WALK: Sig = Sig {
    len: 708,
    hash: 0xe524_388a_fba6_d5d9,
    calls: &[(0x2bc, Callee::Any)],
    imports: &[],
};

/// A task pool reset and its first task (internal, `ecx` = the pool `P`):
/// `*G_P = P`, the `n` entries' first words `P + a + s i` cleared, the
/// counters `G_A`, `G_B` zero and the size `G_N = n` (`a`, `s` and `n`
/// read from the code: 0x48, 0x50, 128 in `magic_0180`; 0x6c, 0x74, 256
/// in `magic_0205`). When the free
/// count `G_F` (u16) is not zero the free head `T = *G_H` is unlinked
/// (`T.prev.next = T.next`, `T.next.prev = T.prev`), `*G_H` becomes
/// `T.next` (0 when it was the last), the count drops and `T` is cleared
/// with `T+8 = T` and the step `T+0x10` the DLL's handler (read from the
/// code); `T` (0 when nothing was free) then goes to the end of the live
/// list (`G_L` its head, `G_E` its tail). Returns `T`.
///
/// `magic_0180` 0x10002140 (221 bytes, `ecx` = the pool; 7 DLLs) and
/// `magic_0205` 0x10002a60 (5 DLLs).
pub fn task_pool_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let p = fr.ecx;
    g.write::<u32>(w(0x2), p);
    // `add ecx, imm8` and `lea ecx, [ecx + imm8]`: sign-extended bytes
    let imm8 = |o: u32| i32::from((w(o) as u8).cast_signed()).cast_unsigned();
    let (first, stride, n) = (imm8(0x8), imm8(0x18), w(0xa));
    let mut e = p.wrapping_add(first);
    for _ in 0..n {
        g.write::<u32>(e, 0);
        e = e.wrapping_add(stride);
    }
    let mut count = g.read::<u16>(w(0x20));
    g.write::<u32>(w(0x25), 0);
    g.write::<u32>(w(0x2b), w(0x2f));
    g.write::<u32>(w(0x34), 0);
    let mut t = 0u32;
    if count != 0 {
        t = g.read::<u32>(w(0x3e));
        let (prev, next) = (g.read::<u32>(t), g.read::<u32>(t + 4));
        g.write::<u32>(prev + 4, next);
        let (prev, next) = (g.read::<u32>(t), g.read::<u32>(t + 4));
        g.write::<u32>(next, prev);
        let head = if count > 1 { g.read::<u32>(t + 4) } else { 0 };
        g.write::<u32>(w(0x5c), head);
        count = count.wrapping_sub(1);
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(t + o, 0);
        }
        g.write::<u32>(t + 8, t);
        g.write::<u32>(t + 0x10, w(0x95));
        g.write::<u8>(t + 0x18, 0);
        g.write::<u16>(w(0xa0), count);
    }
    let listed = g.read::<u32>(w(0xa6)) != 0;
    g.write::<u32>(t + 4, 0);
    if listed {
        let tail = g.read::<u32>(w(0xb7));
        g.write::<u32>(t, tail);
        g.write::<u32>(tail + 4, t);
        g.write::<u32>(t + 8, t);
        g.write::<u32>(w(0xc4), t);
    } else {
        g.write::<u32>(t, 0);
        g.write::<u32>(w(0xd0), t);
        g.write::<u32>(t + 8, t);
        g.write::<u32>(w(0xd8), t);
    }
    Ret::eax(t)
}

/// [`task_pool_reset`].
pub const TASK_POOL_RESET: Sig = Sig {
    len: 221,
    hash: 0xe221_f81b_ac54_7c93,
    calls: &[],
    imports: &[],
};

/// [`task_pool_reset`] (`magic_0205`'s pool: other entry offset, stride
/// and count).
pub const TASK_POOL_RESET_B: Sig = Sig {
    len: 221,
    hash: 0x4b70_3b1d_1565_601d,
    calls: &[],
    imports: &[],
};

/// A colour sampled from byte curves (cdecl, no arguments; a /GS frame).
/// The local colour bytes `C` (12 bytes at `ebp-0x10`) start as 0xff;
/// for each of the `*G_N` records `R_i = G_R + 0x78 i` (signed count) and
/// its index `j = G_J[i]`, the three curves at `R_i - 0x1c`, `R_i` and
/// `R_i + 0x1c` (a pointer to `{count, bytes}` and the current key, i16
/// at `+6`) each give `bytes[k]` with `k` the key plus one (0 from the
/// count on) into `C[3 j + c]`. Then `x` = slot 651's `st0` and slot 808
/// `(*G_H, C, x, 20.0)`. Returns slot 808's `eax`.
///
/// `magic_0644` 0x10006150 (253 bytes; 5 DLLs) and `magic_0565`
/// 0x10001fc0 (5 DLLs).
pub fn curve_colour<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let n = g.read::<i32>(w(0x13));
    let c = ebp.wrapping_sub(0x10);
    for k in 0..3u32 {
        g.write::<u32>(c + 4 * k, u32::MAX);
    }
    let zero = ebp.wrapping_sub(0x14);
    let (mut r, mut jx) = (w(0x33), w(0x38));
    for _ in 0..n.max(0) {
        let base = g.read::<u32>(jx).wrapping_mul(3);
        for (k, at) in [
            (0u32, r.wrapping_sub(0x1c)),
            (1, r),
            (2, r.wrapping_add(0x1c)),
        ] {
            let curve = g.read::<u32>(at);
            let key = i32::from(g.read::<i16>(at + 6)).wrapping_add(1);
            g.write::<u32>(zero, 0);
            let i = if key >= g.read::<i32>(curve) { 0 } else { key };
            let b = g.read::<u8>(g.read::<u32>(curve + 4).wrapping_add(i.cast_unsigned()));
            g.write::<u8>(c.wrapping_add(base).wrapping_add(k), b);
        }
        r = r.wrapping_add(0x78);
        jx = jx.wrapping_add(4);
    }
    let x = crate::m0018::st0_f32(slot(g, img, f, 0xb0, 651, &[]).st0);
    g.write::<f32>(zero, x);
    let h = g.read::<u32>(w(0xdf));
    Ret::eax(slot(g, img, f, 0xd9, 808, &[h, c, x.to_bits(), 0x41a0_0000]).eax)
}

/// [`curve_colour`] (the `rel32`: `__security_check_cookie`).
pub const CURVE_COLOUR: Sig = Sig {
    len: 253,
    hash: 0x5dbd_0833_49f2_caf2,
    calls: &[(0xf5, Callee::Any)],
    imports: &[],
};
