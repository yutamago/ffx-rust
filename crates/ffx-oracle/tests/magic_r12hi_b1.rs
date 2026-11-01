//! Round 12 (own rows from `magic_0450` up), batch 1: the cue starts and
//! two-object starts of `magic_0469`, `0470`, `0551`, `0554` and `0590`
//! against their Rust ports (`ffx_magic::m0638`, `ffx_magic::r12h_start`).
//! Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod magic;
mod magic_hi;
mod magic_l3;
#[path = "magic/ownb.rs"]
mod ownb;

use magic::{Call, Eax, SCRATCH, Twin};
use ownb::rec;

/// Slot 333's angles in turn (ST0), from the cursor at `ANGLES`.
const ANGLES: u32 = SCRATCH + 0x20;

/// The objects start's slots and callees set up (the start at `f`, its
/// object starts' `rel32` at `objs`, its part set-up's at `+0x37`, its
/// free-list pop's at `pop`; `list` the task list's `G_FIRST` and
/// `G_LAST` operands): the lock and part words, slot 941's program
/// table, the object pool and the free ring of four task nodes, the list
/// empty or holding a node, the globals the start writes holding old
/// values.
fn objects_setup(
    t: &mut Twin,
    f: u32,
    objs: [u32; 2],
    pop: u32,
    list: [u32; 2],
    busy: bool,
    k: u32,
) {
    let part_init = t.img.rel32_target(f + 0x37);
    let lock = SCRATCH + 0x800;
    t.set_u32(t.img.word(part_init + 0x01), lock);
    t.set_u32(lock, 0x5555 + k);
    for (i, &o) in objs.iter().enumerate() {
        let s = t.img.rel32_target(f + o);
        t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        let pa = t.img.rel32_target(s + 0x9c);
        let nodes = t.img.word(pa + 0x27) - 8;
        for j in 0..8 {
            t.set_u32(nodes + 12 * j, 0x0dd0_0000 + j);
            t.set_u32(nodes + 12 * j + 4, 0x0ee0_0000 + j);
            t.set_u32(nodes + 12 * j + 8, 0);
        }
        t.set_u16(t.img.word(pa + 0x04), 3 + i as u16);
        let tail = if busy { SCRATCH + 0x600 } else { 0 };
        t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
        t.set_u32(t.img.word(s + 0xa7), tail);
    }
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    let pop = t.img.rel32_target(f + pop);
    magic_hi::pop_pool(t, pop, SCRATCH + 0x1000, 4);
    let node = SCRATCH + 0x3400;
    t.write(node, &[0x33; 0x20]);
    t.set_u32(t.img.word(f + list[0]), if busy { node } else { 0 });
    t.set_u32(t.img.word(f + list[1]), node);
    t.set_u32(ANGLES, 0);
}

/// The objects start's slots: 719 (the handle), 711/710 (the part),
/// 941 (the program table), 714, 262 (three floats written to its
/// pointers), 333 (the angles of [`ANGLE_LIST`] in turn), 271, and the
/// actor block's 215/413.
fn objects_stubs(t: &mut Twin) {
    rec(t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [(711u32, 3usize), (710, 4), (714, 5), (271, 1), (215, 1)] {
        rec(t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    rec(t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[1 + i], (1.5f32 + i as f32 * 0.75).to_bits());
        }
        0x262
    });
    rec(t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |m, _| {
        let k = m.r32(ANGLES);
        m.w32(ANGLES, k + 1);
        let pick = m.r32(ANGLES + 4);
        ANGLE_LIST[((pick + k) % ANGLE_LIST.len() as u32) as usize]
    });
}

/// Slot 333's angles: around `3pi/2 - pi` and `3pi/2 + pi` (the wrap's
/// bounds) and between.
const ANGLE_LIST: [f64; 7] = [-5.0, 0.5, 3.0, 9.0, 20.0, -0.25, 6.5];

/// `r12h_start::objects_start` (0x10001340 in `magic_0469` and
/// `magic_0551`): every angle branch, the task list empty and not.
#[test]
fn objects_start() {
    for n in [469u16, 551] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(0x1340);
        objects_stubs(&mut t);
        let actor = SCRATCH + 0x700;
        t.set_u32(t.img.word(f + 0x77), actor);
        // the actor block: in a battle and not
        let me = t.img.rel32_target(f + 0x24a);
        for k in 0..ANGLE_LIST.len() as u32 {
            objects_setup(
                &mut t,
                f,
                [0x57, 0x17e],
                0x252,
                [0x258, 0x268],
                k % 2 == 0,
                k,
            );
            t.set_u32(ANGLES + 4, k);
            t.set_u32(t.img.word(me + 0x05), k % 3);
            t.set_u32(t.img.word(me + 0x43), k % 2);
            t.set_u32(t.img.word(me + 0x13), 0x0a0a_0013 + k);
            for op in [0xe3u32, 0x105, 0x10b, 0x115, 0x1db, 0x32, 0x1a, 0x24] {
                t.set_u32(t.img.word(f + op), 0x0a0a_0000 + op);
            }
            for op in [0x16fu32, 0x23b] {
                t.set_u16(t.img.word(f + op), 0x5a5a);
            }
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `r12h_start::objects_start_b_up`/`_down` (0x10001370 in `magic_0470`,
/// `magic_0554` and `magic_0590`): every angle branch of both wraps, the
/// task list empty and not.
#[test]
fn objects_start_b() {
    for (n, shift) in [(470u16, 0u32), (554, 0), (590, 10)] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(0x1370);
        let o = |x: u32| if x > 0x142 { x - shift } else { x };
        objects_stubs(&mut t);
        let actor = SCRATCH + 0x700;
        t.set_u32(t.img.word(f + 0x42), actor);
        for k in 0..ANGLE_LIST.len() as u32 {
            objects_setup(
                &mut t,
                f,
                [0xcf, o(0x1ad)],
                o(0x286),
                [o(0x28c), o(0x29c)],
                k % 2 == 1,
                k,
            );
            t.set_u32(ANGLES + 4, k);
            for op in [0x108u32, 0x128, 0x134, 0x13a, 0x144, 0x32, 0x1a, 0x24] {
                t.set_u32(t.img.word(f + op), 0x0a0a_0000 + op);
            }
            for op in [0x19e, 0x203, 0x210, 0x216, 0x270] {
                t.set_u32(t.img.word(f + o(op)), 0x0b0b_0000 + op);
            }
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `m0638::cue_start`/`cue_start_1` (0x10001210 in `magic_0469`,
/// `0551`, `0470`, `0554`, `0590`): slot 225 says wait or go, the
/// once-flag clear and set, slot 227 below, at and above 1 (the objects
/// started).
#[test]
fn cue_starts() {
    for (n, d) in [(469u16, 0u32), (551, 0), (470, 1), (554, 1), (590, 1)] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(0x1210);
        let g = |t: &Twin, op: u32| t.img.word(f + op);
        let start = t.img.rel32_target(f + 0x5b + d);
        objects_stubs(&mut t);
        rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
        rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
        rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x14));
        let h = g(&t, 0x26 + d);
        t.set_u32(h, 0x00a5_0026);
        // the objects start's operands (`magic_0590` is 10 bytes shorter
        // after +0x142)
        let (objs, pop, list, actor) = match n {
            469 | 551 => ([0x57, 0x17e], 0x252, [0x258, 0x268], 0x77),
            590 => ([0xcf, 0x1a3], 0x27c, [0x282, 0x292], 0x42),
            _ => ([0xcf, 0x1ad], 0x286, [0x28c, 0x29c], 0x42),
        };
        t.set_u32(t.img.word(start + actor), SCRATCH + 0x700);
        for (k, (go, once, r227)) in [
            (1u32, 0u32, 1u32),
            (0, 0, 0),
            (0, 1, 1),
            (0, 0, 2),
            (0, 1, 0xffff_ffff),
        ]
        .into_iter()
        .enumerate()
        {
            objects_setup(&mut t, start, objs, pop, list, k % 2 == 1, k as u32);
            t.set_u32(SCRATCH + 0x10, go);
            t.set_u32(SCRATCH + 0x14, r227);
            t.set_u32(g(&t, 0x18), once);
            for op in [0x61u32, 0x6b] {
                t.set_u32(g(&t, op + d), 0x7777_0000 | op);
            }
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// The unframed `start_object_first` copy at `s` set up (its `call
/// pool_alloc` at `+0x8e`): the item and program words, slot 941's
/// table, `count` pool nodes free, the object list's tail `tail`.
fn first_setup(t: &mut Twin, s: u32, count: u16, tail: u32) {
    t.set_u32(t.img.word(s + 0x01), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5b), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    let pa = t.img.rel32_target(s + 0x8f);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), count);
    t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
    t.set_u32(t.img.word(s + 0x9a), tail);
}

/// `m0479::record_object` (0x10003ba0): records with object indices 0,
/// 1 and 2, the pool free and empty, the list empty and not.
#[test]
fn record_object_0479() {
    let Some(mut t) = magic_l3::twin(479) else {
        return;
    };
    let f = t.at(0x3ba0);
    let s = t.img.rel32_target(f + 0x24);
    let rec_at = SCRATCH + 0x800;
    for (k, (index, count, tail)) in [(0u32, 3u16, 0u32), (1, 2, SCRATCH + 0x600), (2, 0, 0)]
        .into_iter()
        .enumerate()
    {
        first_setup(&mut t, s, count, tail);
        t.write(rec_at, &[0x3c; 0x40]);
        t.set_u32(rec_at + 0x2c, index);
        let obj = t.img.word(f + 0x1a) + 0x130 * index;
        t.write(obj, &[0x5a + k as u8; 0x130]);
        t.check_entry(f, Call::default(), &[rec_at], Eax::Checked);
    }
}

/// `m0671::object_start` (0x10001480): the pool free and empty, the
/// list empty and not; the words it sets hold old values.
#[test]
fn object_start_0671() {
    let Some(mut t) = magic_l3::twin(671) else {
        return;
    };
    let f = t.at(0x1480);
    let s = t.img.rel32_target(f + 0x20);
    rec(&mut t, 583, 1, |_, a| 0x0583_0000 ^ a[0]);
    rec(&mut t, 312, 4, |_, a| 0x0312_0000 ^ a[0]);
    for (k, (count, tail)) in [(3u16, 0u32), (2, SCRATCH + 0x600), (0, 0)]
        .into_iter()
        .enumerate()
    {
        first_setup(&mut t, s, count, tail);
        t.write(t.img.word(f + 0x1b), &[0x5a + k as u8; 0x130]);
        t.set_u32(t.img.word(f + 0x2f), 0x00a5_002f);
        t.set_u32(t.img.word(f + 0x39), 0x3939_3939);
        t.set_u32(t.img.word(f + 0x51), 0x00a5_0051);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0638::cue_start_0479`/`_0671` (0x10003a40, 0x10001210): the waits,
/// and the start ([`m0479::start`], [`m0671::start`]) with the list empty
/// and not.
#[test]
fn cue_starts_0479_0671() {
    for (n, rva, head_op, tail_op, pop_op) in [
        (479u16, 0x3a40u32, 0x75u32, 0x65u32, 0x5fu32),
        (671, 0x1210, 0x8d, 0x79, 0x73),
    ] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        let start = t.img.rel32_target(f + 0x5b);
        rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
        for (s, k) in [(711u32, 3usize), (710, 4)] {
            rec(&mut t, s, k, move |_, a| (0x0abc_0000 | s) ^ a[0]);
        }
        rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
        rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
        rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x14));
        t.set_u32(t.img.word(f + 0x26), 0x00a5_0026);
        let part_init = t.img.rel32_target(start + 0x33);
        t.set_u32(t.img.word(part_init + 0x01), SCRATCH + 0x800);
        let pop = t.img.rel32_target(start + pop_op);
        for (k, (go, once, r227)) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 1), (0, 0, 2)]
            .into_iter()
            .enumerate()
        {
            magic_hi::pop_pool(&mut t, pop, SCRATCH + 0x2000, 3);
            let (head, tail) = (t.img.word(start + head_op), t.img.word(start + tail_op));
            magic_hi::list(&mut t, head, tail, SCRATCH + 0x3000, k % 2 == 1);
            t.set_u32(SCRATCH + 0x10, go);
            t.set_u32(SCRATCH + 0x14, r227);
            t.set_u32(t.img.word(f + 0x18), once);
            for op in [0x61u32, 0x6b] {
                t.set_u32(t.img.word(f + op), 0x7777_0000 | op);
            }
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `DllMain` writes no word the ports read, in the DLLs this batch
/// completes.
#[test]
fn dll_main_writes_only_crt_state() {
    for n in [469u16, 470, 472, 475, 479, 551, 554, 590, 671] {
        magic_l3::dll_main(n);
    }
}

/// The slots of the 0472/0475 starts besides [`objects_stubs`]': 539,
/// 917 (from `SCRATCH + 0x18`), 672 (fills its vector), 869, 916, 284.
fn start_stubs(t: &mut Twin) {
    rec(t, 539, 2, |_, a| 0x0539_0000 ^ a[0]);
    rec(t, 917, 0, |m, _| m.r32(SCRATCH + 0x18));
    rec(t, 672, 2, |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, (0.25f32 + i as f32 * 1.5).to_bits());
        }
        0x672
    });
    rec(t, 869, 1, |_, a| 0x0869_0000 ^ a[0]);
    rec(t, 916, 7, |_, a| 0x0916_0000 ^ a[0]);
    rec(t, 284, 3, |_, a| 0x0284_0000 ^ a[0]);
}

/// The task pool (the take at `take`) with six free nodes, the task
/// list (head and tail operands of `task_start` at `ts`) empty or not.
fn task_setup(t: &mut Twin, take: u32, ts: u32, busy: bool) {
    magic_hi::pool(t, take, SCRATCH + 0x4000, 6);
    let node = SCRATCH + 0x3800;
    t.write(node, &[0x38; 0x1c]);
    t.set_u32(t.img.word(ts + 0x75), if busy { node } else { 0 });
    t.set_u32(t.img.word(ts + 0x8c), node);
}

/// `m0472::start` (0x100031d0): every angle branch, the lists empty and
/// not.
#[test]
fn start_0472() {
    let Some(mut t) = magic_l3::twin(472) else {
        return;
    };
    let f = t.at(0x31d0);
    objects_stubs(&mut t);
    start_stubs(&mut t);
    for op in [0x6du32, 0x120] {
        t.set_u32(t.img.word(f + op), SCRATCH + 0x700);
    }
    let take = t.img.rel32_target(f + 0x1db);
    let ts = t.img.rel32_target(f + 0x1a5);
    let me = t.img.rel32_target(f + 0x1ce);
    let part_init = t.img.rel32_target(f + 0x37);
    t.set_u32(t.img.word(part_init + 0x01), SCRATCH + 0x800);
    for k in 0..ANGLE_LIST.len() as u32 {
        for o in [0x4du32, 0x100] {
            let s = t.img.rel32_target(f + o);
            t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
            t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
            let pa = t.img.rel32_target(s + 0x99);
            let nodes = t.img.word(pa + 0x27) - 8;
            for j in 0..8 {
                t.set_u32(nodes + 12 * j, 0x0dd0_0000 + j);
                t.set_u32(nodes + 12 * j + 4, 0x0ee0_0000 + j);
                t.set_u32(nodes + 12 * j + 8, 0);
            }
            t.set_u16(t.img.word(pa + 0x04), if k == 2 { 0 } else { 3 });
            t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
            t.set_u32(
                t.img.word(s + 0xa4),
                if k % 2 == 0 { 0 } else { SCRATCH + 0x600 },
            );
        }
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        for i in 0..8 {
            t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
        }
        task_setup(&mut t, take, ts, k % 2 == 1);
        let node = SCRATCH + 0x3400;
        t.write(node, &[0x33; 0x20]);
        t.set_u32(t.img.word(f + 0x1e1), if k % 3 == 0 { 0 } else { node });
        t.set_u32(t.img.word(f + 0x1f1), node);
        t.set_u32(ANGLES, 0);
        t.set_u32(ANGLES + 4, k);
        t.set_u32(t.img.word(me + 0x05), k % 2);
        t.set_u32(t.img.word(me + 0x43), k % 2);
        for op in [0xd7u32, 0xfb, 0x182, 0x1a0, 0x1b8, 0x80, 0x133] {
            t.set_u32(t.img.word(f + op), 0x0a0a_0000 + op);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0475::start` (0x100013e0): every angle branch, slot 917 201 and
/// not, the lists empty and not, scales above and not above 0.
#[test]
fn start_0475() {
    let Some(mut t) = magic_l3::twin(475) else {
        return;
    };
    let f = t.at(0x13e0);
    objects_stubs(&mut t);
    start_stubs(&mut t);
    t.set_u32(t.img.word(f + 0x57), SCRATCH + 0x700);
    let take = t.img.rel32_target(f + 0x120);
    let ts = t.img.rel32_target(f + 0xb7);
    let me = t.img.rel32_target(f + 0x113);
    let s = t.img.rel32_target(f + 0x4c);
    let part_init = t.img.rel32_target(f + 0x37);
    t.set_u32(t.img.word(part_init + 0x01), SCRATCH + 0x800);
    // the second word of the state reset: its own record
    for k in 0..ANGLE_LIST.len() as u32 {
        t.set_u32(t.img.word(s + 0x01), SCRATCH + 0x200);
        t.set_u32(t.img.word(s + 0x5b), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        for i in 0..8 {
            t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
        }
        let pa = t.img.rel32_target(s + 0x95);
        let nodes = t.img.word(pa + 0x27) - 8;
        for j in 0..8 {
            t.set_u32(nodes + 12 * j, 0x0dd0_0000 + j);
            t.set_u32(nodes + 12 * j + 4, 0x0ee0_0000 + j);
            t.set_u32(nodes + 12 * j + 8, 0);
        }
        t.set_u16(t.img.word(pa + 0x04), if k == 2 { 0 } else { 3 });
        t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
        t.set_u32(
            t.img.word(s + 0xa0),
            if k % 2 == 0 { 0 } else { SCRATCH + 0x600 },
        );
        t.write(t.img.word(f + 0x47), &[0x47; 0x130]);
        task_setup(&mut t, take, ts, k % 2 == 1);
        let node = SCRATCH + 0x3400;
        t.write(node, &[0x33; 0x20]);
        t.set_u32(t.img.word(f + 0x126), if k % 3 == 0 { 0 } else { node });
        t.set_u32(t.img.word(f + 0x136), node);
        t.set_u32(ANGLES, 0);
        t.set_u32(ANGLES + 4, k);
        t.set_u32(SCRATCH + 0x18, if k % 2 == 0 { 0xc9 } else { 0xc8 + k });
        t.set_u32(t.img.word(me + 0x05), k % 2);
        t.set_u32(t.img.word(me + 0x43), k % 2);
        // a DY global, and the scale constants' sign through the states
        t.set_u32(t.img.word(f + 0x1ab), (0.5f32 + k as f32).to_bits());
        for op in [0x5du32, 0xb2, 0xd7, 0x19b, 0x1b3, 0x1cb] {
            t.set_u32(t.img.word(f + op), 0x0a0a_0000 + op);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0638::cue_start` in `magic_0472` and `magic_0475` (0x100030a0,
/// 0x100012b0): the waits (the starts themselves in [`start_0472`],
/// [`start_0475`]).
#[test]
fn cue_starts_0472_0475() {
    for (n, rva) in [(472u16, 0x30a0u32), (475, 0x12b0)] {
        let Some(mut t) = magic_l3::twin(n) else {
            return;
        };
        let f = t.at(rva);
        rec(&mut t, 225, 1, |m, _| m.r32(SCRATCH + 0x10));
        rec(&mut t, 226, 2, |_, a| 0x226 ^ a[0]);
        rec(&mut t, 227, 1, |m, _| m.r32(SCRATCH + 0x14));
        t.set_u32(t.img.word(f + 0x26), 0x00a5_0026);
        for (go, once, r227) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 0xffff_ffff)] {
            t.set_u32(SCRATCH + 0x10, go);
            t.set_u32(SCRATCH + 0x14, r227);
            t.set_u32(t.img.word(f + 0x18), once);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// `m0499::tick` (0x100018d0): not running, and running with slot 253
/// 0 and not, the flag word 0 and not afterwards (the parts' flags off:
/// their refresh has its own test).
#[test]
fn tick_0499() {
    let Some(mut t) = magic_l3::twin(499) else {
        return;
    };
    let f = t.at(0x18d0);
    let pr = t.img.rel32_target(f + 0x65);
    let prb = t.img.rel32_target(pr + 0x1e1);
    rec(&mut t, 253, 0, |m, _| m.r32(SCRATCH + 0x10));
    for (s, n) in [
        (723u32, 1usize),
        (724, 2),
        (236, 3),
        (243, 2),
        (231, 3),
        (233, 3),
        (237, 4),
        (230, 2),
        (258, 1),
        (712, 2),
    ] {
        rec(&mut t, s, n, move |_, a| (0x5a5a_0000 + s) ^ a[0]);
    }
    for (at, op, v) in [
        (pr, 0xb7u32, SCRATCH + 0x400),
        (pr, 0x13e, 0x00c0_013e),
        (pr, 0x14a, 0x00c0_014a),
        (pr, 0x15d, 0x00c0_015d),
        (pr, 0x16b, SCRATCH + 0x20),
        (pr, 0x170, SCRATCH + 0x30),
        (prb, 0x56, SCRATCH + 0x50),
        (prb, 0x63, SCRATCH + 0x40),
        (prb, 0x7f, SCRATCH + 0x40),
        (f, 0x18, SCRATCH + 0x60),
        (f, 0x50, SCRATCH + 0x64),
        (f, 0x74, SCRATCH + 0x68),
        (f, 0x1e, 0x00a5_001e),
        (f, 0x3d, 0x00a5_003d),
        (f, 0x4b, 0x00a5_004b),
    ] {
        t.set_u32(t.img.word(at + op), v);
    }
    for op in [0x1fu32, 0x2a, 0x36, 0x3e, 0x49] {
        let g = t.img.word(prb + op);
        if t.u32(g) == 0 {
            t.set_u32(g, 0x00d0_0000 + op);
        }
    }
    for (k, (run, busy, flag)) in [(0u32, 0u32, 0u32), (1, 0, 0), (1, 1, 0), (1, 0, 5)]
        .into_iter()
        .enumerate()
    {
        t.set_u32(t.img.word(f + 0x02), run);
        t.set_u32(SCRATCH + 0x10, busy);
        t.set_u32(SCRATCH + 0x60, 0x7777_0000 + k as u32);
        t.set_u32(SCRATCH + 0x64, 0x6464_6464);
        t.set_u32(SCRATCH + 0x68, flag);
        t.set_u32(t.img.word(f + 0x7f), 0x10 + k as u32);
        // each block's first word (the refresh's slot 237 gets it)
        for op in [0x60u32, 0x6a] {
            let block = t.img.word(f + op);
            t.set_u32(block, 0x00b1_0000 | op);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0537::restart` (0x100010c0): records 0, 1 and 3 (the start's random
/// draw from slot 324, its program list from slot 941).
#[test]
fn restart_0537() {
    let Some(mut t) = magic_l3::twin(537) else {
        return;
    };
    let f = t.at(0x10c0);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(0x1003_0fa4, o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 324, 0, |m, _| m.r32(SCRATCH + 0x10) ^ 0x5a5a);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[0]);
    rec(&mut t, 325, 2, |_, a| 0x0325_0000 ^ a[0]);
    let recs_at = t.img.word(f + 0x0a);
    let vecs = t.img.word(f + 0x28);
    for (k, i) in [0u32, 1, 3].into_iter().enumerate() {
        t.set_u32(SCRATCH + 0x10, 0x1234 * (k as u32 + 1));
        for j in 0..3 {
            t.set_u32(
                vecs + 0x10 * i + 4 * j,
                (1.25f32 + j as f32 + k as f32).to_bits(),
            );
        }
        t.check_entry(f, Call::default(), &[recs_at + 4 * i], Eax::Checked);
    }
}

/// `m0540::part_start_of4`/`part_start_5` (0x10001460, 0x100015c0):
/// each of the four parts, one past them, and the fifth part.
#[test]
fn part_starts_0540() {
    let Some(mut t) = magic_l3::twin(540) else {
        return;
    };
    let f = t.at(0x1460);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + 0x11 * i);
    }
    let o = SCRATCH + 0x100;
    t.set_u32(t.img.word(f + 0x13), o);
    t.set_u32(o + 0x10, 0x00b0_0010);
    t.set_u32(t.img.word(f + 0x04), SCRATCH + 0x180);
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[0]);
    let parts = t.img.word(f + 0x2a);
    for i in 0..6u32 {
        let part = parts + 0x80 * i;
        t.write(part, &[0x5a + i as u8; 0x80]);
        t.check_entry(f, Call::default(), &[part], Eax::Checked);
    }
    let f5 = t.at(0x15c0);
    t.write(parts + 0x200, &[0x3c; 0x80]);
    t.check_entry(f5, Call::default(), &[parts + 0x200], Eax::Checked);
}

/// The 0x74-record pool of `sb_misc::pool_take` at `take`: `count`
/// records from `recs`, the ones in `busy` taken.
fn pool74(t: &mut Twin, take: u32, recs: u32, count: u32, busy: u32) {
    let w = |t: &Twin, o: u32| t.img.word(take + o);
    t.set_u32(w(t, 0x37), recs);
    for i in 0..8 {
        t.write(recs + 0x74 * i, &[0x5a; 0x74]);
        t.set_u32(recs + 0x74 * i + 0x30, 0x300 + i);
        t.set_u32(recs + 0x74 * i + 0x6c, (busy >> i) & 1);
    }
    t.set_u32(w(t, 0x0f), count);
    t.set_u32(w(t, 0x08), 0);
    t.set_u32(w(t, 0x2b), 0);
}

/// `m0688::start` (0x10003da0): the task list empty and not.
#[test]
fn start_0688() {
    let Some(mut t) = magic_l3::twin(688) else {
        return;
    };
    let f = t.at(0x3da0);
    let la = t.img.rel32_target(f + 0x3d);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [(711u32, 3usize), (710, 4), (413, 1), (215, 1)] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    let part_init = t.img.rel32_target(f + 0x60);
    t.set_u32(t.img.word(part_init + 0x01), SCRATCH + 0x800);
    let ri = t.img.rel32_target(f + 0xde);
    let take = t.img.rel32_target(f + 0xa0);
    let rec_at = SCRATCH + 0x900;
    for (k, busy) in [false, true].into_iter().enumerate() {
        t.set_u32(t.img.word(la + 0x34), 0);
        t.set_u32(t.img.word(la + 0x01), 0x0106_0000 + k as u32);
        t.set_u32(t.img.word(la + 0x12), 0x0c12_0000 + k as u32);
        // the records' own free ring, then the task pool's
        let node = |i: u32| SCRATCH + 0x5000 + 0x40 * i;
        for i in 0..3u32 {
            t.write(node(i), &[0x33; 0x40]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(t.img.word(ri + 0x3e), node(0));
        t.set_u16(t.img.word(ri + 0x20), 3);
        t.write(SCRATCH + 0x5800, &[0x58; 0x40]);
        t.set_u32(
            t.img.word(ri + 0xa6),
            if busy { SCRATCH + 0x5800 } else { 0 },
        );
        t.set_u32(t.img.word(ri + 0xb7), SCRATCH + 0x5800);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(t.img.word(f + 0xa6), if busy { last } else { 0 });
        t.set_u32(t.img.word(f + 0xba), last);
        t.write(rec_at, &[0x99; 0x140]);
        t.set_u32(t.img.word(f + 0x65), rec_at);
        // the record block the start points the record global at
        t.write(t.img.word(f + 0x24), &[0x24; 0x140]);
        for op in [0x87u32, 0x92, 0x9a] {
            t.set_u32(t.img.word(f + op), 0x5a5a_5a5a);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0688::script_step` (0x10003a40): the script ended (the node given
/// back) and waiting.
#[test]
fn script_step_0688() {
    let Some(mut t) = magic_l3::twin(688) else {
        return;
    };
    let f = t.at(0x3a40);
    let run = t.img.rel32_target(f + 0x32);
    let give = t.img.rel32_target(f + 0x47);
    let rec_at = SCRATCH + 0x900;
    t.set_u32(t.img.word(f + 0x06), rec_at);
    for mode in [0u8, 2] {
        t.write(rec_at, &[0x99; 0x140]);
        t.write(t.img.word(f + 0x0f), &[0x6b; 0x30]);
        t.set_u32(t.img.word(f + 0x25), 0x0a0a_0025);
        // the script's mode byte: 0 ended (returns 1), 2 waiting forever
        t.set_u8(t.img.word(run + 0x01), mode);
        let s = t.img.word(run + 0x1c);
        t.set_u16(s + 0x54, 0xffff);
        t.set_u32(s + 0x24, 0);
        t.set_u32(s + 0x28, 0);
        let node = SCRATCH + 0x3000;
        magic_hi::lone_node(&mut t, give, node);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `m0688::spark` (0x10003b60): no record free, and records taken with
/// several random draws.
#[test]
fn spark_0688() {
    let Some(mut t) = magic_l3::twin(688) else {
        return;
    };
    let f = t.at(0x3b60);
    let take = t.img.rel32_target(f + 0x12);
    rec(&mut t, 719, 3, |m, a| {
        m.w32(SCRATCH + 0x700, 0x7001_0000 ^ a[0]);
        m.w32(SCRATCH + 0x704, 0x7002_0000 ^ a[1]);
        SCRATCH + 0x700
    });
    rec(&mut t, 941, 1, |_, a| a[0] + 1);
    rec(&mut t, 237, 5, |m, a| {
        let v = m.r32(a[2])
            ^ m.r32(a[2] + 4).rotate_left(3)
            ^ m.r32(a[2] + 8).rotate_left(7)
            ^ m.r32(a[2] + 12).rotate_left(11);
        m.w32(SCRATCH + 0x710, v);
        0x237
    });
    rec(&mut t, 314, 1, |m, a| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        0x0314_0000 ^ a[0] ^ k
    });
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x20);
        f64::from(f32::from_bits(a[0])) * (0.125 + f64::from(k % 7) * 0.0625)
    });
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |_, a| -f64::from(f32::from_bits(a[0])) * 0.375);
    t.set_u32(t.img.word(f + 0xf8), 0x00f8_00f8);
    let recs = SCRATCH + 0x1000;
    for (count, busy) in [(0u32, 0u32), (4, 0), (4, 0b0101), (4, 0b0011)] {
        pool74(&mut t, take, recs, count, busy);
        t.set_u32(SCRATCH + 0x20, count + busy);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0476::task` (0x100017a0): the motion step and the objects saying
/// done or not.
#[test]
fn task_0476() {
    let Some(mut t) = magic_l3::twin(476) else {
        return;
    };
    let f = t.at(0x17a0);
    let ms = t.img.rel32_target(f + 0x05);
    let ro = t.img.rel32_target(f + 0x0c);
    rec(&mut t, 346, 1, |_, a| 0x0346_0000 ^ a[0]);
    t.set_u32(t.img.word(f + 0x20), 0x0a0a_0020);
    for (k, (done, running)) in [(true, false), (false, false), (true, true)]
        .into_iter()
        .enumerate()
    {
        magic_hi::motion(&mut t, ms, done);
        magic_hi::objects(&mut t, ro, running);
        let task = SCRATCH + 0x6000;
        let rec_at = SCRATCH + 0x6100;
        t.write(task, &[0x11 + k as u8; 0x20]);
        t.set_u32(task + 8, rec_at);
        t.write(rec_at, &[0x22; 0x20]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// The look-matrix draws' slots (the matrix callee's and the tail's).
fn look_stubs(t: &mut Twin) {
    for (s, n) in [
        (230u32, 2usize),
        (231, 3),
        (714, 5),
        (229, 3),
        (237, 4),
        (266, 3),
        (235, 2),
        (325, 2),
        (233, 3),
    ] {
        rec(t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    rec(t, 941, 2, |_, _| SCRATCH + 0x400);
    rec(t, 712, 2, |m, _| {
        m.w32(SCRATCH + 0x830, m.r32(SCRATCH + 0x800));
        // (the item word moved on, as the game's part code may)
        m.w32(SCRATCH + 0x810, 0x0712_0712);
        m.r32(SCRATCH + 0x10)
    });
    rec(t, 713, 1, |m, a| {
        m.w32(SCRATCH + 0x834, m.r32(SCRATCH + 0x810));
        m.w32(SCRATCH + 0x810, 0x0713_0713);
        0x0713_0000 ^ a[0]
    });
}

/// `m0664::look_draw` and `look_draw_fade` (0x10001e20, 0x100014e0):
/// slot 712 0 and not; the fading object before and after the frame
/// count, its halfword going below 1 and not, and another object.
#[test]
fn look_draws_0664() {
    let Some(mut t) = magic_l3::twin(664) else {
        return;
    };
    look_stubs(&mut t);
    for i in 0..16 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x0f00_0000 + i);
    }
    let f = t.at(0x1e20);
    let fade = t.at(0x14e0);
    let lock = SCRATCH + 0x800;
    for (at, op) in [(f, 0x8d), (f, 0xa9), (fade, 0xd3), (fade, 0xef)] {
        t.set_u32(t.img.word(at + op), lock);
    }
    for (at, op) in [
        (f, 0x80),
        (f, 0xb7),
        (f, 0xd0),
        (fade, 0xc6),
        (fade, 0xfe),
        (fade, 0x117),
    ] {
        t.set_u32(t.img.word(at + op), SCRATCH + 0x810);
    }
    let q = SCRATCH + 0x820;
    t.set_u32(t.img.word(f + 0xdb), q);
    t.set_u32(q + 0x10, 0x0010_0010);
    for (at, op, v) in [
        (f, 0x1a, 0x001a_0000u32),
        (fade, 0x34, 0x0034_0000),
        (fade, 0x6c, 0x006c_0000),
    ] {
        t.set_u32(t.img.word(at + op), v);
    }
    let base = t.img.word(fade + 0x24);
    let other = SCRATCH + 0x7000;
    for (k, (s712, count, half)) in [
        (0u32, 0xb00u32, 0x300u16),
        (1, 0xb00, 0x300),
        (1, 0xb91, 0x300),
        (1, 0xb91, 0x100),
        (1, 0xb91, 0x8000),
        (1, 0xb90, 0x100),
        (1, 0xb91, 0x19a),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(SCRATCH + 0x10, s712);
        t.set_u32(lock, 0x4444 + k as u32);
        t.set_u32(t.img.word(fade + 0x3f), count);
        for obj in [base, other] {
            t.write(obj, &[0x40 + k as u8; 0x80]);
            t.set_u32(obj + 0x60, (1.5f32 + k as f32).to_bits());
            t.set_u16(obj + 0x76, half);
        }
        t.check_entry(f, Call::default(), &[other], Eax::Checked);
        t.check_entry(fade, Call::default(), &[base], Eax::Checked);
        t.check_entry(fade, Call::default(), &[other], Eax::Checked);
    }
}

/// `m0672::start` (0x10001d50): the task list empty and not (the camera
/// block's angle set from slot 333, slot 12's float).
#[test]
fn start_0672() {
    let Some(mut t) = magic_l3::twin(672) else {
        return;
    };
    let f = t.at(0x1d50);
    let la = t.img.rel32_target(f + 0x48);
    let ar = t.img.rel32_target(f + 0x75);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
    for (s, n) in [
        (711u32, 3usize),
        (710, 4),
        (413, 1),
        (215, 1),
        (262, 4),
        (357, 3),
        (356, 3),
        (399, 2),
        (345, 1),
        (349, 2),
        (271, 1),
    ] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, _| 2.5);
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, _| 0.75);
    let part = t.img.rel32_target(f + 0x70);
    t.set_u32(t.img.word(part + 0x04), SCRATCH + 0x800);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(ar + 0x0e), blk);
    let take = t.img.rel32_target(f + 0x116);
    for (k, busy) in [false, true].into_iter().enumerate() {
        t.set_u32(t.img.word(la + 0x34), 0);
        t.set_u32(t.img.word(la + 0x01), 0x0106_0000 + k as u32);
        t.set_u32(t.img.word(la + 0x12), 0x0c12_0000 + k as u32);
        t.write(blk, &vec![0x4b; 0x1400]);
        let recs = t.img.word(f + 0xb4);
        t.write(recs - 0x2800, &vec![0x6c; 0x5000]);
        magic_hi::pool(&mut t, take, SCRATCH + 0x4000, 4);
        let last = SCRATCH + 0x3400;
        t.write(last, &[0x34; 0x20]);
        t.set_u32(t.img.word(f + 0x11c), if busy { last } else { 0 });
        t.set_u32(t.img.word(f + 0x133), last);
        for op in [0x03u32, 0xe1, 0xe8, 0xfd] {
            t.set_u32(t.img.word(f + op), 0x5a5a_5a5a);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `m0672::task` (0x100014c0): the objects running or not, the block's
/// flag 1 and not.
#[test]
fn task_0672() {
    let Some(mut t) = magic_l3::twin(672) else {
        return;
    };
    let f = t.at(0x14c0);
    let ro = t.img.rel32_target(f + 0x08);
    for (s, n) in [(214u32, 1usize), (346, 1), (399, 2), (357, 3), (356, 3)] {
        rec(&mut t, s, n, move |_, a| (0x0abc_0000 | s) ^ a[0]);
    }
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x03), blk);
    t.set_u32(t.img.word(f + 0x25), 0x0025_0025);
    for (running, flag) in [(false, 1u32), (true, 1), (false, 0)] {
        magic_hi::objects(&mut t, ro, running);
        t.write(blk, &vec![0x4b; 0x800]);
        t.set_u32(blk + 0x5c0, flag);
        t.check_entry(f, Call::default(), &[SCRATCH + 0x6000], Eax::Checked);
    }
}

/// `m0672::packed_frame` (0x10001710): counting up, counting down past
/// 0 (the node given back), the angle wrapping and not.
#[test]
fn packed_frame_0672() {
    let Some(mut t) = magic_l3::twin(672) else {
        return;
    };
    let f = t.at(0x1710);
    let give = t.img.rel32_target(f + 0x13e);
    rec(&mut t, 719, 3, |m, a| {
        m.w32(SCRATCH + 0x700, 0x0012_3456 ^ a[0]);
        m.w32(SCRATCH + 0x704, 0x0abc_0def);
        SCRATCH + 0x700
    });
    for (s, n) in [(235u32, 1usize), (210, 2), (885, 7), (905, 9), (776, 1)] {
        rec(&mut t, s, n, move |m, a| {
            if s == 905 {
                for i in 0..4 {
                    let v = m.r32(a[7] + 4 * i);
                    m.w32(SCRATCH + 0x840 + 4 * i, v);
                }
            }
            if s == 885 {
                return m.r32(SCRATCH + 0x14);
            }
            (0x0abc_0000 | s) ^ a[0]
        });
    }
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(f + 0x19), blk);
    t.set_u32(t.img.word(f + 0xa8), SCRATCH + 0x2000);
    t.set_u32(t.img.word(f + 0xe1), 0x00e1_00e1);
    let lock = t.img.rel32_target(f + 0x11e);
    let _ = lock;
    for (k, (up, total, left, angle)) in [
        (3i32, 10i32, 5i32, 0.5f32),
        (0, 10, 5, 6.0),
        (0, 10, 1, 1.0),
        (1, 4, 0, 7.0),
    ]
    .into_iter()
    .enumerate()
    {
        t.write(blk, &vec![0x4b; 0x800]);
        t.set_u32(blk + 0x5a8, up.cast_unsigned());
        t.set_u32(blk + 0x5ac, total.cast_unsigned());
        t.set_u32(blk + 0x5b0, left.cast_unsigned());
        t.set_u32(blk + 0x5b4, 7);
        t.set_u32(blk + 0x5a4, angle.to_bits());
        t.set_u32(SCRATCH + 0x14, k as u32 % 2);
        let node = SCRATCH + 0x3000;
        magic_hi::lone_node(&mut t, give, node);
        t.check_entry(f, Call::default(), &[node], Eax::Checked);
    }
}

/// `m0672::spawn` (0x100018e0, 0x10001a30): counting down, the record
/// pool empty and with a record.
#[test]
fn spawn_0672() {
    let Some(mut t) = magic_l3::twin(672) else {
        return;
    };
    rec(&mut t, 314, 1, |m, a| {
        let k = m.r32(SCRATCH + 0x20);
        m.w32(SCRATCH + 0x20, k + 1);
        0x0314_0000 ^ a[0] ^ k
    });
    t.stub_st0(314, |m, a| {
        let k = m.r32(SCRATCH + 0x20);
        f64::from(f32::from_bits(a[0])) * (0.25 + f64::from(k % 5) * 0.125)
    });
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[0]);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    let blk = SCRATCH + 0x1_0000;
    for rva in [0x18e0u32, 0x1a30] {
        let f = t.at(rva);
        t.set_u32(t.img.word(f + 0x11), blk);
        let pt = t.img.rel32_target(f + 0x30);
        let so = t.img.rel32_target(f + 0x74);
        t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        let pa = t.img.rel32_target(so + 0x96);
        let nodes = t.img.word(pa + 0x27) - 8;
        let recs = SCRATCH + 0x8000;
        t.set_u32(t.img.word(pt + 0x16), recs);
        for (k, (count, free)) in [(3u32, 0u32), (1, 4), (1, 0), (1, 2)]
            .into_iter()
            .enumerate()
        {
            for j in 0..8 {
                t.set_u32(nodes + 12 * j, 0x0dd0_0000 + j);
                t.set_u32(nodes + 12 * j + 4, 0x0ee0_0000 + j);
                t.set_u32(nodes + 12 * j + 8, 0);
            }
            t.set_u16(t.img.word(pa + 0x04), 3);
            t.set_u32(t.img.word(so + 0xa1), 0);
            t.write(blk, &vec![0x4b + k as u8; 0x300]);
            for i in 0..4u32 {
                let r = recs + 0x140 * i;
                t.write(r, &[0x60 + i as u8; 0x140]);
                t.set_u32(r + 0x130, u32::from(free != 4 && i != free));
                t.set_u16(r + 0x124, 0x8000);
            }
            t.set_u32(t.img.word(pt + 0x09), 4);
            t.set_u32(t.img.word(pt + 0x01), 0);
            let rec_at = SCRATCH + 0x6000;
            t.write(rec_at, &[0x22; 0x40]);
            t.set_u32(rec_at + 0x2c, count);
            t.set_u32(SCRATCH + 0x20, k as u32);
            t.check_entry(
                f,
                Call {
                    eax: 0x00ea_0000,
                    ..Call::default()
                },
                &[rec_at],
                Eax::Checked,
            );
        }
    }
}
