//! Round 15's ports (`ffx_magic::r15`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop
)]

mod magic;
mod magic_owna;
mod magic_r11;

use ffx_magic::r15;
use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, recs, twin};

/// The slots of an actor's point: 262 fills the position, 268 a point,
/// 12 the ground (ST0), 16 and 364 the scale's two factors.
fn point_slots(t: &mut Twin) {
    rec(t, 262, 4, |m, a| {
        for i in 1..4usize {
            let v = (a[0] & 0xff) as f32 * 0.5 + i as f32;
            m.w32(a[i], v.to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m, a| {
        for i in 0..3 {
            let v = (a[1] as f32) * 0.25 - i as f32;
            m.w32(a[2] + 4 * i, v.to_bits());
        }
        0x268
    });
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * -0.75);
    rec(t, 364, 1, |_, _| 364);
    rec(t, 16, 2, |_, _| 16);
    for s in [16u32, 364] {
        t.stub_st0(s, move |_, a| f64::from(a[0] & 0xf) + f64::from(s) * 0.001);
    }
}

/// The first actor `CHRS[*MAP]` (operands `map`, `chrs` of `f`).
fn first_actor(t: &mut Twin, f: u32, map: u32, chrs: u32) {
    let sel = SCRATCH + 0x5000;
    // (the byte after the first one non-zero: a wider read shows)
    t.write(sel, &[2, 1, 0, 3]);
    t.set_u32(t.img.word(f + map), sel);
    for i in 0..4 {
        t.set_u32(t.img.word(f + chrs) + 4 * i, 0x0c0c_0100 + i);
    }
}

/// `magic_0104` 0x100016f0: the first actor's point in every mode.
#[test]
fn actor_point_0104() {
    let Some(mut t) = twin(104) else { return };
    let f = t.at(0x16f0);
    first_actor(&mut t, f, 0x11, 0x1d);
    let sp = t.img.rel32_target(f + 0x78);
    first_actor(&mut t, sp, 0x7, 0x12);
    point_slots(&mut t);
    let out = SCRATCH + 0x200;
    for mode in 0u32..9 {
        t.write(out, &[0x44; 0x10]);
        let c = Call {
            edx: out,
            ..Call::default()
        };
        t.check(f, c, &[mode], Eax::Checked, r15::actor_point_0104);
    }
}

/// `magic_0104` 0x10001460: the start-up with the slot-390 id in the
/// table (mode 5) or not (mode 7), one of the ground ids (0x1158) or
/// not, the task list empty and not.
#[test]
fn start_up_0104() {
    let Some(mut t) = twin(104) else { return };
    let f = t.at(0x1460);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 0x209);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x97);
    magic_r11::object_start_setup(&mut t, start, (0x7d, 0x95));
    let ap = t.img.rel32_target(f + 0xa2);
    first_actor(&mut t, ap, 0x11, 0x1d);
    let sp = t.img.rel32_target(ap + 0x78);
    first_actor(&mut t, sp, 0x7, 0x12);
    point_slots(&mut t);
    let ids = SCRATCH + 0x5100;
    rec(&mut t, 0x618 / 4, 1, move |m, _| m.r32(ids));
    t.set_u32(w(&t, 0x3a), SCRATCH + 0x5000);
    t.set_u32(w(&t, 0xd0), 0x0d00_0001);
    let table = w(&t, 0x5b);
    // (the table's third id: the search walks it)
    let third = t.u32(table + 8);
    for (id, listed) in [
        (third, false),
        (0x1158, true),
        (0x0bad, false),
        (0x1077, true),
    ] {
        t.set_u32(ids, id);
        // the fades' records and the record block the start-up fills
        for o in [0x109u32, 0x192, 0x117] {
            t.write(w(&t, o), &[0x5c; 0x40]);
        }
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(w(&t, 0x1cd), if listed { SCRATCH + 0x600 } else { 0 });
        t.set_u32(w(&t, 0x1df), SCRATCH + 0x600);
        t.set_u32(w(&t, 0xe6), 2.5f32.to_bits());
        t.check(f, Call::default(), &[], Eax::Checked, r15::start_up_0104);
    }
}

/// What `r15::colour_start_0053` (at `f`) needs: the pools, the actor
/// block's slots, the index `k`, the flag bit, the task list.
fn colour_start_0053_setup(t: &mut Twin, f: u32, k: u32, bit: bool, listed: bool) {
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 500);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    t.set_u32(op(t, 0x05), k);
    t.set_u32(op(t, 0x14), if bit { 0x0000_0c01 } else { 0x0000_0801 });
    for o in [0x2a, 0x33, 0x3d, 0x6d, 0xb7, 0xce] {
        t.set_u32(op(t, o), 0x5a5a_5a5b);
    }
    t.write(SCRATCH + 0x600, &[0x44; 0x10]);
    t.set_u32(op(t, 0xdd), if listed { SCRATCH + 0x600 } else { 0 });
    t.set_u32(op(t, 0xf4), SCRATCH + 0x600);
}

/// `magic_0053` 0x100017c0: the index 0 and not, the flag bit set and
/// clear, the task list empty and not.
#[test]
fn colour_start_0053() {
    let Some(mut t) = twin(53) else { return };
    let f = t.at(0x17c0);
    for (k, bit, listed) in [
        (0u32, false, false),
        (1, true, false),
        (0, true, true),
        (2, false, true),
        (0, true, false),
    ] {
        colour_start_0053_setup(&mut t, f, k, bit, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            r15::colour_start_0053,
        );
    }
}

/// `magic_0053` 0x10001250: slot 225 busy, `*G_I` 0 with slot 593 busy
/// and not, the first and a later request, not loaded and loaded (the
/// start-up runs).
#[test]
fn load_0053() {
    let Some(mut t) = twin(53) else { return };
    let f = t.at(0x1250);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (busy, wait, loaded) = (SCRATCH + 0x100, SCRATCH + 0x104, SCRATCH + 0x108);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 0x944 / 4, 0, move |m, _| m.r32(wait));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(w(&t, 0x41), 0x0000_5e5e);
    let start = t.img.rel32_target(f + 0x76);
    for (b, k, wt, asked, l) in [
        (1u32, 0u32, 0u32, 0u32, 1u32),
        (0, 0, 1, 0, 1),
        (0, 0, 0, 0, 0),
        (0, 1, 1, 1, 0xffff_ffff),
        (0, 0, 0, 0, 1),
        (0, 3, 1, 1, 2),
    ] {
        colour_start_0053_setup(&mut t, start, k, true, false);
        t.set_u32(busy, b);
        t.set_u32(wait, wt);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x31), asked);
        for o in [0x7cu32, 0x86] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0094` 0x10006780 and `magic_0095` 0x10006710
/// (`r15::summon_task_0094`/`_0095`): every state below and at its
/// threshold (a NaN timer too), the state-0 height by both of
/// `model_direct`'s answers (`magic_0094`), the lists empty and not.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per callee
fn summon_tasks_0094_0095() {
    for (dll, rva) in [(94u16, 0x6780u32), (95, 0x6710)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &Twin, o: u32| t.img.word(f + o);
        let one = dll == 94;
        // (state-1 particle node `rel32`, its owner global)
        let (pn, owner) = if one { (0x27d, 0x244) } else { (0x285, 0x24c) };
        let pn = t.img.rel32_target(f + pn);
        let wrapper = t.img.rel32_target(pn + 0x88);
        let lock = SCRATCH + 0x7f00;
        t.set_u32(t.img.word(wrapper + 0x04), lock);
        t.set_u32(lock, 0x77);
        t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x7f10);
        let table = SCRATCH + 0x7c00;
        for i in 0..8 {
            t.set_u32(table + 4 * i, 0x9000 + i);
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, move |m, a| m.r32(lock) ^ a[0]);
        rec(&mut t, 325, 2, |_, a| 0x0325_0000 ^ a[1]);
        rec(&mut t, 268, 3, |m, a| {
            for i in 0..3 {
                m.w32(a[2] + 4 * i, (1.5 - i as f32).to_bits());
            }
            0x268
        });
        rec(&mut t, 239, 3, |_, a| 0x0239_0000 ^ a[0]);
        let model = SCRATCH + 0x110;
        let direct = SCRATCH + 0x114;
        rec(&mut t, 390, 1, move |m, _| m.r32(model));
        rec(&mut t, 534, 1, move |m, _| m.r32(direct));
        let owner_at = SCRATCH + 0x7a00;
        for i in 0..0x20 {
            t.set_u32(owner_at + 4 * i, (0.25 * i as f32).to_bits());
        }
        let b = SCRATCH + 0x1_0000;
        let task = SCRATCH + 0xe00;
        let ring = SCRATCH + 0xe40;
        let lists = [w(&t, 0x19b), if one { w(&t, 0x1db) } else { w(&t, 0x1e3) }];
        let lists0 = [
            if one { w(&t, 0x2a3) } else { w(&t, 0x2ab) },
            if one { w(&t, 0x2fd) } else { w(&t, 0x2f3) },
        ];
        // the particle nodes' parent lists (the records' second words)
        let plists = if one {
            [w(&t, 0x260), w(&t, 0x382)]
        } else {
            [w(&t, 0x268), w(&t, 0x378)]
        };
        let state_at = if one { 0x814 } else { 0x810 };
        let k = |t: &Twin, o: u32| f32::from_bits(t.u32(w(t, o)));
        let thr = [
            if one { 0.0 } else { k(&t, 0x29a) },
            k(&t, 0x18a),
            k(&t, 0x4b),
        ];
        let step = k(&t, 0x1d);
        // (state, timer above its threshold, model id, direct, lists empty)
        for (case, (state, above, id, dir, empty)) in [
            (2u16, true, 0u32, 0u32, false),
            (2, false, 0, 0, false),
            (1, true, 0, 0, true),
            (1, true, 0, 0, false),
            (1, false, 0, 0, false),
            (0, true, 0x2000, 0, true),
            (0, true, 0x2000, 1, false),
            (0, true, 0x1071, 0, false),
            (0, false, 0, 0, false),
            (3, true, 0, 0, false),
            (7, true, 0, 0, false),
            // (11..13: each state's timer exactly at its threshold)
            (2, true, 0, 0, false),
            (1, true, 0, 0, false),
            (0, true, 0x2000, 1, false),
        ]
        .into_iter()
        .enumerate()
        {
            let case = case as u32;
            t.write(b, &[0x5a; 0x820]);
            for i in 0..0x200 {
                t.set_u32(
                    b + 4 * i,
                    (0.125 * (i % 37) as f32 - 1.0 + case as f32).to_bits(),
                );
            }
            t.set_u16(b + state_at, state);
            let thr = thr[usize::from(state.min(2))];
            if one {
                // the state-0 threshold is the block's own word
                t.set_u32(b + 0x810, 4.0f32.to_bits());
            }
            let thr = if one && state == 0 { 4.0 } else { thr };
            let timer = if above { thr + 3.0 } else { thr - 3.0 } - step;
            let timer = match case {
                8 => f32::NAN,
                11.. => hits(thr, step),
                _ => timer,
            };
            // the state-2 colours' globals: values to subtract from, junk
            // in the ones written
            for (i, o) in (0u32..).zip([0x64, 0x95, 0xd0, 0x104, 0x11f, 0x132, 0x13a, 0x14a, 0x152])
            {
                t.set_u32(w(&t, o), (0.0625 * i as f32 + 0.25).to_bits());
            }
            for o in [0x69u32, 0x8a, 0xc5, 0xf9, 0x114, 0x127, 0x142, 0x15a, 0x162] {
                t.set_u32(w(&t, o), 0x5c5c_0000 + o);
            }
            if one {
                // the handle byte and a non-zero byte after it
                t.set_u32(w(&t, 0x2cc), 0x5a5a_5a07);
            }
            t.set_u32(b + 0x80c, timer.to_bits());
            t.set_u32(model, id);
            t.set_u32(direct, dir);
            for o in [owner, if one { 0x366 } else { 0x35c }] {
                t.set_u32(w(&t, o), owner_at);
            }
            for l in lists.into_iter().chain(lists0).chain(plists) {
                t.write(l, &[0x3c; 0x20]);
                if empty {
                    t.set_u32(l + 0x10, l);
                } else {
                    t.set_u32(l + 0x10, SCRATCH + 0x900);
                }
                t.set_u16(l + 0x18, 2);
            }
            t.write(SCRATCH + 0x900, &[0x3d; 0x20]);
            // the task's node in a ring of three
            for i in 0..3u32 {
                let n = ring + 0x10 * i;
                t.set_u32(n + 4, ring + 0x10 * ((i + 1) % 3));
                t.set_u32(n + 8, ring + 0x10 * ((i + 2) % 3));
            }
            t.set_u32(task, ring);
            t.set_u32(task + 4, b);
            scrub(&mut t);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// Junk on the stack below the entry (the frames' never-read words).
fn scrub(t: &mut Twin) {
    let entry = magic::STACK_TOP - 0x100;
    let junk: Vec<u8> = (0..0x1000u32)
        .map(|i| (i.wrapping_mul(0x2b) ^ 0x5b).to_le_bytes()[0])
        .collect();
    t.write(entry - 0x1000, &junk);
}

/// What `r15::start_up_0303` (at `f`) needs: the pools its takes use, the
/// object start, the slots (262 and 268 fill their points), the camera
/// records' free ring and the pool reset take's ring; `flag` picks slot
/// 413 or 215, `listed` the task lists' state, `k` varies the values.
fn start_up_0303_setup(t: &mut Twin, f: u32, flag: u32, listed: bool, k: u32) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    magic_r11::pools(t, f, 778);
    magic_r11::object_slots(t);
    let start = t.img.rel32_target(f + 0x83);
    magic_r11::object_start_setup(t, start, (0x84, 0x99));
    rec(t, 262, 4, move |m, a| {
        for i in 1..4 {
            m.w32(a[i], (0.5 * i as f32 + k as f32).to_bits());
        }
        0x262
    });
    rec(t, 268, 3, move |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (1.25 - i as f32 + k as f32).to_bits());
        }
        0x268
    });
    for s in [215u32, 413] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    rec(t, 287, 3, |_, a| 0x0287_0000 ^ a[2]);
    recs(t, &[(283, 2), (281, 1)]);
    t.set_u32(op(t, f, 0x8e), 0x0a00_0001);
    t.set_u32(op(t, f, 0x1a3), 0x0a00_0002);
    t.set_u32(op(t, f, 0xbf), 0x0bf0_0000 + k);
    t.set_u32(op(t, f, 0xcd), flag);
    t.set_u32(op(t, f, 0x11a), flag);
    t.set_u32(op(t, f, 0x1d6), k % 3);
    for o in [0x149u32, 0x152, 0x16d, 0x190] {
        t.set_u16(op(t, f, o), 0x5a5a);
    }
    for o in [0x175u32, 0x198] {
        // (the bytes after them non-zero too: a wider store shows)
        t.set_u32(op(t, f, o), 0x5a5a_5a5a);
    }
    // the records the index clears one word of
    t.write(op(t, f, 0x1ec), &[0x3b; 3 * 0x148]);
    // (the takes, the camera records' and the pool reset's share the
    // DLL's free ring: `pools` gave it 16 nodes)
    let cam = t.img.rel32_target(f + 0x19d);
    t.set_u32(op(t, cam, 0x42), k & 1);
    // the task lists
    let list = SCRATCH + 0x600;
    t.write(list, &[0x44; 0x10]);
    for (first, last) in [(0x275u32, 0x28a), (0x30d, 0x321)] {
        let a = if first == 0x30d { cam } else { f };
        t.set_u32(op(t, a, first), if listed { list } else { 0 });
        t.set_u32(op(t, a, last), list);
    }
}

/// `magic_0303` 0x100017a0: slot 215 and 413, the task lists empty and
/// not, the record index and the camera pause varied.
#[test]
fn start_up_0303() {
    let Some(mut t) = twin(303) else { return };
    let f = t.at(0x17a0);
    for (k, (flag, listed)) in [(0u32, false), (1, true), (0, true), (1, false)]
        .into_iter()
        .enumerate()
    {
        start_up_0303_setup(&mut t, f, flag, listed, k as u32);
        scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r15::start_up_0303);
    }
}

/// `magic_0303` 0x10001210 (`r9_load::load_0303`): slot 225 busy, the
/// first and a later request, not loaded and loaded (the start-up runs).
#[test]
fn load_0303() {
    let Some(mut t) = twin(303) else { return };
    let f = t.at(0x1210);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(w(&t, 0x26), 0x0000_5e5e);
    let start = t.img.rel32_target(f + 0x5b);
    for (b, asked, l) in [
        (1u32, 0u32, 1u32),
        (0, 0, 0),
        (0, 1, 0xffff_ffff),
        (0, 0, 1),
        (0, 1, 2),
    ] {
        start_up_0303_setup(&mut t, start, b, asked == 1, l);
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), asked);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// What `r15::start_up_0176` (at `f`) needs: the pools, the object
/// starts, the slots (262 and 268 fill their points, 333 the yaw), the
/// first actor, the task list.
fn start_up_0176_setup(t: &mut Twin, f: u32, listed: bool, k: u32) {
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 843);
    magic_r11::object_slots(t);
    for at in [0x64u32, 0x170] {
        let start = t.img.rel32_target(f + at);
        magic_r11::object_start_setup(t, start, (0x84, 0x9d));
    }
    // (each call other values: the aim's differences are not 0)
    rec(t, 262, 4, move |m, a| {
        let c = m.r32(magic_owna::row(262)) as f32;
        for i in 1..4 {
            m.w32(a[i], (0.75 * i as f32 * (c + 1.0) - k as f32).to_bits());
        }
        0x262
    });
    rec(t, 268, 3, move |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (2.5 + i as f32 * k as f32).to_bits());
        }
        0x268
    });
    rec(t, 333, 1, |_, a| a[0] ^ 0x333);
    t.set_u32(op(t, 0x161), 0x5a5a_5a5a);
    t.stub_st0(333, move |_, _| [8.0, -8.0, 0.5, 2.0][k as usize % 4]);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    t.set_u32(op(t, 0x84), 0x0a00_0001);
    t.set_u32(op(t, 0x91), 0x0a00_0002);
    first_actor(t, f, 0x175, 0x189);
    let list = SCRATCH + 0x600;
    t.write(list, &[0x44; 0x10]);
    t.set_u32(op(t, 0x30b), if listed { list } else { 0 });
    t.set_u32(op(t, 0x31b), list);
}

/// `magic_0176` 0x10001390: the yaw above, below and inside its wrap,
/// the task list empty and not.
#[test]
fn start_up_0176() {
    let Some(mut t) = twin(176) else { return };
    let f = t.at(0x1390);
    for (k, listed) in [false, true, false, true].into_iter().enumerate() {
        start_up_0176_setup(&mut t, f, listed, k as u32);
        scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r15::start_up_0176);
    }
}

/// `magic_0176` 0x10001260 (`r9_load::load_0176`): as [`load_0303`].
#[test]
fn load_0176() {
    let Some(mut t) = twin(176) else { return };
    let f = t.at(0x1260);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(w(&t, 0x26), 0x0000_5e5e);
    let start = t.img.rel32_target(f + 0x5b);
    for (b, asked, l) in [
        (1u32, 0u32, 1u32),
        (0, 0, 0),
        (0, 1, 0xffff_ffff),
        (0, 0, 1),
        (0, 1, 2),
    ] {
        start_up_0176_setup(&mut t, start, asked == 1, l);
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), asked);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// An `x` with `x + step == target` in `f32` (a timer exactly at a
/// threshold after its step), searched from `target - step`.
fn hits(target: f32, step: f32) -> f32 {
    let mut x = target - step;
    for _ in 0..64 {
        let s = x + step;
        if s.to_bits() == target.to_bits() {
            return x;
        }
        x = if s < target {
            x.next_up()
        } else {
            x.next_down()
        };
    }
    x
}

/// `magic_0367` 0x100018e0 with its sparks (0x100015c0): the motions 2
/// and 8 (two bones each), one below 2 and one between; a record table
/// with free and busy records, the cursor wrapping, and a full table
/// (the cursor returned).
#[test]
fn motion_sparks_0367() {
    let Some(mut t) = twin(367) else { return };
    let f = t.at(0x18e0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let sp = t.img.rel32_target(f + 0x60);
    let sw = |t: &Twin, o: u32| t.img.word(sp + o);
    let motion = SCRATCH + 0x100;
    rec(&mut t, 0x58 / 4, 1, move |m, _| m.r32(motion));
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..3 {
            m.w32(a[2] + 4 * i, (a[1] as f32 * 0.5 - i as f32).to_bits());
        }
        0x268
    });
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ (a[2] << 8) ^ a[1]);
    for s in [314u32, 404] {
        rec(&mut t, s, 1, move |_, a| s ^ a[0]);
        t.stub_st0(s, move |m, a| {
            let k = m.r32(magic_owna::row(s));
            f64::from(f32::from_bits(a[0])) * 0.5 + f64::from(k) * 0.125
        });
    }
    t.set_u32(w(&t, 0x17), 0x0a00_0001);
    let table = SCRATCH + 0x2000;
    for (case, (m, n, cursor, busy)) in [
        (2u32 << 8, 8u32, 6u32, 0b0000_0101u32),
        (8 << 8, 8, 0, 0b1010_1010),
        (8 << 8, 4, 2, 0b1111),
        (1 << 8, 8, 0, 0),
        (5 << 8, 8, 0, 0),
        (2 << 8, 0, 3, 0),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(motion, m.wrapping_add(case as u32));
        t.write(table, &[0x5c; 0x50 * 8]);
        for i in 0..8 {
            t.set_u32(table + 0x50 * i + 0x48, (busy >> i) & 1);
        }
        for (count, cur, tab) in [(0x32u32, 0x37u32, 0x47u32), (0x1b2, 0x1b7, 0x1c7)] {
            t.set_u32(sw(&t, count), n);
            t.set_u32(sw(&t, cur), cursor);
            t.set_u32(sw(&t, tab), table);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// What `magic_0349`'s sets (at `f`, the object start at `start`) need:
/// the effect block `E`, the object start's pools and slots, slot 404's
/// float, a free ring of `free` nodes (`count_op`, `head_op`) and the
/// task list (`first_op`, `last_op`) empty or not.
#[allow(clippy::too_many_arguments)] // one word per operand the sets read
fn sets_0349_setup(
    t: &mut Twin,
    f: u32,
    start: u32,
    e_op: u32,
    (count_op, head_op, first_op, last_op): (u32, u32, u32, u32),
    free: u16,
    listed: bool,
) {
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(t);
    magic_r11::object_start_setup(t, start, (0x84, 0x9d));
    rec(t, 0x650 / 4, 1, |_, a| a[0] ^ 0x404);
    t.stub_st0(0x650 / 4, |m, _| {
        f64::from(m.r32(magic_owna::row(0x650 / 4))) * 0.375
    });
    let e = SCRATCH + 0x1_0000;
    t.write(e, &[0x3e; 0x1b00]);
    for i in 0..0x20 {
        t.set_u32(e + 0x1a14 + 4 * i, 0x1a14_0000 + i);
    }
    t.set_u32(op(t, e_op), e);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for i in 0..4 {
        t.write(node(i), &[0x77; 0x20]);
        t.set_u32(node(i), node((i + 3) % 4));
        t.set_u32(node(i) + 4, node((i + 1) % 4));
    }
    t.set_u32(op(t, head_op), node(1));
    t.set_u16(op(t, count_op), free);
    let list = SCRATCH + 0x600;
    t.write(list, &[0x44; 0x10]);
    t.set_u32(op(t, first_op), if listed { list } else { 0 });
    t.set_u32(op(t, last_op), list);
}

/// `magic_0349` 0x10004ed0 and 0x10004b00 (`r15::first_set_0349`,
/// `second_set_0349`): the free ring with one node and more (the head
/// moves on or becomes 0), the task list empty and not.
#[test]
fn sets_0349() {
    let Some(mut t) = twin(349) else { return };
    for (rva, call, e_op, ops) in [
        (
            0x4ed0u32,
            0x5eu32,
            0x0bu32,
            (0xe6u32, 0x15cu32, 0x1c3u32, 0x1d9u32),
        ),
        (0x4b00, 0x61, 0x09, (0xe9, 0x126, 0x186, 0x19c)),
    ] {
        let f = t.at(rva);
        let start = t.img.rel32_target(f + call);
        // (the second set takes two nodes: one free node leaves none for
        // its second take, which the original writes through 0 with)
        let ones: &[u16] = if rva == 0x4ed0 { &[1, 3] } else { &[2, 3] };
        for &free in ones {
            for listed in [false, true] {
                sets_0349_setup(&mut t, f, start, e_op, ops, free, listed);
                scrub(&mut t);
                t.check_entry(f, Call::default(), &[], Eax::Checked);
            }
        }
    }
}

/// `magic_0325` 0x10001510 (`r15::delayed_start_0325`): the countdown
/// still running, then the start with the yaw above, below and inside
/// its wrap, the free ring with two nodes and more (the task start takes one too), slot 215 and 413, the
/// task list empty and not.
#[test]
#[allow(clippy::too_many_lines)] // one set-up per callee
fn delayed_start_0325() {
    let Some(mut t) = twin(325) else { return };
    let f = t.at(0x1510);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 1015);
    magic_r11::object_slots(&mut t);
    for at in [0x93u32, 0x12a, 0x1cc] {
        let start = t.img.rel32_target(f + at);
        magic_r11::object_start_setup(&mut t, start, (0x84, 0x99));
    }
    rec(&mut t, 262, 4, |m, a| {
        let c = m.r32(magic_owna::row(262)) as f32;
        for i in 1..4 {
            m.w32(a[i], (0.5 * i as f32 + c).to_bits());
        }
        0x262
    });
    rec(&mut t, 268, 3, |m, a| {
        let c = m.r32(magic_owna::row(268)) as f32;
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (1.25 - i as f32 + c * 0.5).to_bits());
        }
        0x268
    });
    let yaw = SCRATCH + 0x110;
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw))));
    for s in [215u32, 413] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    for o in [0x2au32, 0x9e, 0x130, 0x1d7] {
        t.set_u32(w(&t, o), 0x0a00_0000 + o);
    }
    let task = SCRATCH + 0xe00;
    let next = SCRATCH + 0xe40;
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for (case, (wait, y, free, flag, listed)) in [
        (3i32, 0.0f32, 3u16, 0u32, false),
        (1, -9.0, 3, 0, false),
        (0, 9.0, 2, 1, true),
        (-2, 2.0, 2, 1, false),
        (1, 4.0, 3, 0, true),
    ]
    .into_iter()
    .enumerate()
    {
        let c = case as u32;
        t.write(task, &[0x5a; 0x20]);
        t.set_u32(task + 0x14, wait.cast_unsigned());
        t.set_u32(task + 8, next);
        t.write(next, &[0x5b; 0x20]);
        t.set_u32(yaw, y.to_bits());
        for i in 0..4 {
            t.write(node(i), &[0x77; 0x20]);
            t.set_u32(node(i), node((i + 3) % 4));
            t.set_u32(node(i) + 4, node((i + 1) % 4));
        }
        t.set_u32(w(&t, 0x29c), node(1));
        t.set_u16(w(&t, 0x20f), free);
        let list = SCRATCH + 0x600;
        t.write(list, &[0x44; 0x10]);
        t.set_u32(w(&t, 0x303), if listed { list } else { 0 });
        t.set_u32(w(&t, 0x31c), list);
        t.set_u32(w(&t, 0x35b), flag);
        t.set_u32(w(&t, 0x3a0), flag);
        t.set_u32(w(&t, 0x352), 0x0352_0000 + c);
        // the globals and bytes the start sets: other values before
        for o in [0x24bu32, 0x253, 0x25b, 0x26d, 0x3cf, 0x3d8] {
            t.set_u32(w(&t, o), 0x5a5a_5a5a);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `magic_0195` 0x10001000 (`r15::scatter_0195`): the effect's count 7
/// (every slot by its own entry), 3 and 1 (the rest at random entries
/// with their steps).
#[test]
fn scatter_0195() {
    let Some(mut t) = twin(195) else { return };
    let f = t.at(0x1000);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 0x724 / 4, 3, |m, a| {
        for i in 0..4 {
            m.w32(
                a[2] + 4 * i,
                (0.5 * i as f32 + (a[0] & 0xf) as f32).to_bits(),
            );
        }
        0x724
    });
    rec(&mut t, 0x754 / 4, 1, |m, a| {
        let k = m.r32(magic_owna::row(0x754 / 4)) as f32;
        for i in 0..4 {
            m.w32(a[0] + 4 * i, (k * 0.25 - i as f32).to_bits());
        }
        0x754
    });
    recs(&mut t, &[(0x708 / 4, 2), (0x704 / 4, 2)]);
    let e = SCRATCH + 0x4000;
    t.write(e, &[0x3a; 0x200]);
    for i in 0..7 {
        t.set_u32(e + 0x1a0 + 4 * i, 0x0e00_0000 + i);
    }
    for o in [0xa2u32, 0xce, 0xf5] {
        t.set_u32(w(&t, o), e);
    }
    for count in [7u32, 3, 1] {
        t.set_u32(e + 0x19c, count);
        t.write(w(&t, 0xb6), &[0x3f; 0x70]);
        t.write(w(&t, 0x99), &[0x5d; 0x1c]);
        t.write(w(&t, 0xe4), &[0x5e; 0x1c]);
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[0x0a19_5000 + count], Eax::Checked);
    }
}

/// `magic_0195` 0x10001520 (`r15::aim_0195`): two slots `i` and key
/// counts, the slots' vectors other per call.
#[test]
fn aim_0195() {
    let Some(mut t) = twin(195) else { return };
    let f = t.at(0x1520);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 0x720 / 4, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (1.5 * i as f32 - 2.0).to_bits());
        }
        0x720
    });
    rec(&mut t, 0x740 / 4, 1, |m, a| {
        for i in 0..4 {
            let x = f32::from_bits(m.r32(a[0] + 4 * i));
            m.w32(a[0] + 4 * i, (x * 0.5 + 0.125).to_bits());
        }
        0x740
    });
    recs(&mut t, &[(0x708 / 4, 2), (0x70c / 4, 2)]);
    let e = SCRATCH + 0x4000;
    t.set_u32(e + 0x198, 0x0198_0001);
    t.set_u32(w(&t, 0x48), e);
    let obj = SCRATCH + 0x5000;
    for (i, n) in [(2u32, 1u32), (5, 3)] {
        t.write(obj, &[0x4b; 0x200]);
        t.set_u32(obj + 0xac, n);
        t.set_u32(w(&t, 0x12), i);
        t.set_u32(w(&t, 0x67), i);
        for c in 0..0x40u32 {
            t.set_u32(w(&t, 0x6f) + 4 * c, (0.375 * c as f32 - 3.0).to_bits());
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// What `r15::start_up_0314` (at `f`) needs: the object starts, the
/// slots, the first actor, the free ring (`free` nodes) and the task list.
fn start_up_0314_setup(t: &mut Twin, f: u32, k: u32, free: u16, flag: u32, listed: bool) {
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(t);
    for at in [0xb5u32, 0x1a2, 0x266] {
        let start = t.img.rel32_target(f + at);
        magic_r11::object_start_setup(t, start, (0x84, 0x9d));
    }
    rec(t, 262, 4, |m, a| {
        let c = m.r32(magic_owna::row(262)) as f32;
        for i in 1..4 {
            m.w32(a[i], (0.5 * i as f32 + c).to_bits());
        }
        0x262
    });
    rec(t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (1.25 - i as f32).to_bits());
        }
        0x268
    });
    rec(t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |_, _| [8.0, -8.0, 0.5, 2.0][k as usize % 4]);
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, _| -1.75);
    for s in [215u32, 413] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    t.set_u32(op(t, 0x4e), 0x0a00_0001);
    t.set_u32(op(t, 0xc0), 0x0a00_0002);
    t.set_u32(op(t, 0x294), 0x0294_0000 + k);
    t.set_u32(op(t, 0x28e), flag);
    t.set_u32(op(t, 0x2cf), flag);
    t.set_u32(op(t, 0x193), 0x5a5a_5a5a);
    t.set_u32(op(t, 0x2fe), 0x5a5a_5a5a);
    t.set_u32(op(t, 0x307), 0x5a5a_5a5a);
    first_actor(t, f, 0x1a7, 0x1bb);
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for i in 0..4 {
        t.write(node(i), &[0x77; 0x20]);
        t.set_u32(node(i), node((i + 3) % 4));
        t.set_u32(node(i) + 4, node((i + 1) % 4));
    }
    t.set_u32(op(t, 0x31d), node(1));
    t.set_u16(op(t, 0x30e), free);
    let list = SCRATCH + 0x600;
    t.write(list, &[0x44; 0x10]);
    t.set_u32(op(t, 0x384), if listed { list } else { 0 });
    t.set_u32(op(t, 0x394), list);
}

/// `magic_0314` 0x10001340: the yaws above, below and inside their
/// wraps, slot 215 and 413, the free ring's last node and more, the task
/// list empty and not.
#[test]
fn start_up_0314() {
    let Some(mut t) = twin(314) else { return };
    let f = t.at(0x1340);
    for (k, (free, flag, listed)) in [
        (3u16, 0u32, false),
        (1, 1, true),
        (2, 0, true),
        (3, 1, false),
    ]
    .into_iter()
    .enumerate()
    {
        start_up_0314_setup(&mut t, f, k as u32, free, flag, listed);
        scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r15::start_up_0314);
    }
}

/// `magic_0314` 0x10001210 (`r9_load::load_0314`): as [`load_0303`].
#[test]
fn load_0314() {
    let Some(mut t) = twin(314) else { return };
    let f = t.at(0x1210);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(w(&t, 0x26), 0x0000_5e5e);
    let start = t.img.rel32_target(f + 0x5b);
    for (b, asked, l) in [
        (1u32, 0u32, 1u32),
        (0, 0, 0),
        (0, 1, 0xffff_ffff),
        (0, 0, 1),
        (0, 1, 2),
    ] {
        start_up_0314_setup(&mut t, start, l, 2, asked, asked == 1);
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), asked);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// What `r15::start_up_0381` (at `f`) needs: the pools, the object
/// starts, the slots, the selection byte and the task list.
fn start_up_0381_setup(t: &mut Twin, f: u32, k: u32, flags: [u32; 3], listed: bool) {
    let op = |t: &Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(t, f, 940);
    magic_r11::object_slots(t);
    for at in [0x75u32, 0x13b] {
        let start = t.img.rel32_target(f + at);
        magic_r11::object_start_setup(t, start, (0x84, 0x9c));
    }
    rec(t, 262, 4, |m, a| {
        let c = m.r32(magic_owna::row(262)) as f32;
        for i in 1..4 {
            m.w32(a[i], (0.5 * i as f32 + c).to_bits());
        }
        0x262
    });
    rec(t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, _| 2.25);
    rec(t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| {
        let c = m.r32(magic_owna::row(333));
        [8.0, -8.0, 0.5, 2.0][(k + c) as usize % 4]
    });
    for s in [215u32, 413] {
        rec(t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    rec(t, 271, 3, |_, a| a[0] ^ 0x271);
    rec(t, 0x370 / 4, 3, |_, a| a[0] ^ 0x370);
    for o in [0x80u32, 0xb4, 0x15b, 0x168] {
        t.set_u32(op(t, o), 0x0a00_0000 + o);
    }
    t.set_u32(op(t, 0x267), 0x0267_0000 + k);
    for (o, v) in [0x270u32, 0x2a8, 0x328].into_iter().zip(flags) {
        t.set_u32(op(t, o), v);
    }
    for o in [0x2db, 0x2e4, 0x352, 0x35b] {
        t.set_u32(op(t, o), 0x5a5a_5a5a);
    }
    // (the byte after it non-zero: a wider store shows)
    t.set_u32(op(t, 0x25), 0x5a5a_5a5a);
    let sel = SCRATCH + 0x5000;
    t.write(sel, &[2, 1, 0, 3]);
    t.set_u32(op(t, 0x2e9), sel);
    for o in [0x1e8u32, 0x20f] {
        t.write(op(t, o), &[0x5c; 0x40]);
    }
    let list = SCRATCH + 0x600;
    t.write(list, &[0x44; 0x10]);
    t.set_u32(op(t, 0x36b), if listed { list } else { 0 });
    t.set_u32(op(t, 0x37c), list);
}

/// `magic_0381` 0x100014f0: the yaws on each side of their wraps and
/// between, slot 215 and 413 in each record, the task list empty and not.
#[test]
fn start_up_0381() {
    let Some(mut t) = twin(381) else { return };
    let f = t.at(0x14f0);
    for (k, (flags, listed)) in [
        ([0u32, 0, 0], false),
        ([1, 1, 1], true),
        ([0, 1, 0], true),
        ([1, 0, 1], false),
    ]
    .into_iter()
    .enumerate()
    {
        start_up_0381_setup(&mut t, f, k as u32, flags, listed);
        scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r15::start_up_0381);
    }
}

/// `magic_0381` 0x100012b0 (`r9_load::load_0381`): as [`load_0303`].
#[test]
fn load_0381() {
    let Some(mut t) = twin(381) else { return };
    let f = t.at(0x12b0);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(w(&t, 0x26), 0x0000_5e5e);
    let start = t.img.rel32_target(f + 0x5b);
    for (b, asked, l) in [
        (1u32, 0u32, 1u32),
        (0, 0, 0),
        (0, 1, 0xffff_ffff),
        (0, 0, 1),
        (0, 1, 2),
    ] {
        start_up_0381_setup(&mut t, start, l, [asked; 3], asked == 1);
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), asked);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// What `r15::start_up_0245` (at `f`) needs: the parts buffer's lock, the
/// object start, the slots, the actors, the free ring and the task list.
fn start_up_0245_setup(t: &mut Twin, f: u32, k: u32, actors: u32, free: u16, listed: bool) {
    let op = |t: &Twin, a: u32, o: u32| t.img.word(a + o);
    let pb = t.img.rel32_target(f + 0x4a);
    let lock = SCRATCH + 0x7f00;
    t.set_u32(op(t, pb, 0x1), lock);
    t.set_u32(op(t, pb, 0x31), lock);
    t.set_u32(lock, 0x77);
    rec(t, 711, 3, |_, a| a[0] ^ 0x711);
    rec(t, 710, 4, |_, a| a[0] ^ 0x710);
    magic_r11::object_slots(t);
    let start = t.img.rel32_target(f + 0x5f);
    magic_r11::object_start_setup(t, start, (0x7d, 0x8f));
    rec(t, 262, 4, |m, a| {
        let c = m.r32(magic_owna::row(262)) as f32;
        for i in 1..4 {
            m.w32(a[i], (1.5 * i as f32 - c).to_bits());
        }
        0x262
    });
    rec(t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |_, _| [8.0, -8.0, 0.5, 2.0][k as usize % 4]);
    rec(t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(t, 413, 1, |_, a| 0x0413_0000 ^ a[0]);
    for o in [0x80u32, 0x8f] {
        t.set_u32(op(t, f, o), 0x0a00_0000 + o);
    }
    t.set_u32(op(t, f, 0x11a), actors);
    let sel = SCRATCH + 0x5000;
    t.write(sel, &[2, 1, 0, 3]);
    t.set_u32(op(t, f, 0x131), sel);
    for i in 0..4 {
        t.set_u32(op(t, f, 0x13c) + 4 * i, 0x0c0c_0100 + i);
    }
    t.write(op(t, f, 0x1bf), &[0x5d; 0x20]);
    for o in [0x22au32, 0x23a] {
        t.set_u32(op(t, f, o), 0x5a5a_5a5a);
    }
    let node = |i: u32| SCRATCH + 0x1000 + 0x40 * i;
    for i in 0..4 {
        t.write(node(i), &[0x77; 0x20]);
        t.set_u32(node(i), node((i + 3) % 4));
        t.set_u32(node(i) + 4, node((i + 1) % 4));
    }
    t.set_u32(op(t, f, 0x254), node(1));
    t.set_u16(op(t, f, 0x1d2), free);
    let list = SCRATCH + 0x600;
    t.write(list, &[0x44; 0x10]);
    t.set_u32(op(t, f, 0x2af), if listed { list } else { 0 });
    t.set_u32(op(t, f, 0x2c6), list);
}

/// `magic_0245` 0x10001340: the yaw above, below and inside its wrap,
/// none to three actors measured, the free ring's last two nodes and
/// more, the task list empty and not.
#[test]
fn start_up_0245() {
    let Some(mut t) = twin(245) else { return };
    let f = t.at(0x1340);
    for (k, (actors, free, listed)) in [
        (3u32, 3u16, false),
        (0, 2, true),
        (1, 3, true),
        (2, 4, false),
    ]
    .into_iter()
    .enumerate()
    {
        start_up_0245_setup(&mut t, f, k as u32, actors, free, listed);
        scrub(&mut t);
        t.check(f, Call::default(), &[], Eax::Checked, r15::start_up_0245);
    }
}

/// `magic_0245` 0x10001210 (`r9_load::load_0245`): as [`load_0303`].
#[test]
fn load_0245() {
    let Some(mut t) = twin(245) else { return };
    let f = t.at(0x1210);
    let w = |t: &Twin, o: u32| t.img.word(f + o);
    let (busy, loaded) = (SCRATCH + 0x100, SCRATCH + 0x108);
    rec(&mut t, 225, 1, move |m, _| m.r32(busy));
    rec(&mut t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(&mut t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(w(&t, 0x26), 0x0000_5e5e);
    let start = t.img.rel32_target(f + 0x5b);
    for (b, asked, l) in [
        (1u32, 0u32, 1u32),
        (0, 0, 0),
        (0, 1, 0xffff_ffff),
        (0, 0, 1),
        (0, 1, 2),
    ] {
        start_up_0245_setup(&mut t, start, l, 2, 3, asked == 1);
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(w(&t, 0x18), asked);
        for o in [0x61u32, 0x6b] {
            t.set_u32(w(&t, o), 0x0bad_0000 + o);
        }
        scrub(&mut t);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}
