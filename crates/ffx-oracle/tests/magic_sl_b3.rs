//! Small own functions below `magic_0450` (agent magic-small-lo), batch
//! 3: task steps, a model visitor, record starts, a spark. Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss, clippy::many_single_char_names)]

mod magic;
mod magic_hi;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, TRAPS, Twin};
use magic_owna::{rec, twin};

/// Junk under the entry on both sides.
fn dirty(t: &mut Twin) {
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
}

/// `n` bytes that differ from word to word (seeded by `seed`), so a port
/// that reads the wrong word of a block reads another value.
fn varied(n: usize, seed: u8) -> Vec<u8> {
    (0..n)
        .map(|i| u8::try_from((i * 7 + usize::from(seed)) % 251).unwrap())
        .collect()
}

/// Every slot in `slots` recorded with its word count, answering a
/// distinct value mixed with its first word.
fn recs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (0x0abc_0000 | s) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// A motion script record at `sc` done or waiting.
fn script(t: &mut Twin, sc: u32, done: bool) {
    t.write(sc, &varied(0x20, 0x5a));
    t.set_u32(sc + 0x14, u32::from(done));
    t.set_u16(sc + 0x18, 3);
}

/// `m0199::script_step` (0x10001960): the script waiting, the objects
/// running, the flag off, all done.
#[test]
fn script_step_0199() {
    let Some(mut t) = twin(199) else { return };
    let at = t.at(0x1960);
    let ro = t.img.rel32_target(at + 0x11);
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    for (done, running, flag) in [
        (false, false, 0x8000u32),
        (true, true, 0x8000),
        (true, false, 0x7fff),
        (true, false, 0x18000),
    ] {
        let sc = t.img.word(at + 0x5);
        script(&mut t, sc, done);
        magic_hi::objects(&mut t, ro, running);
        t.set_u32(t.img.word(at + 0x20), flag);
        t.write(task, &varied(0x20, 0x11));
        t.set_u32(task + 8, r);
        t.write(r, &varied(0x20, 0x22));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0203::model_visit` (0x10002e80) alone and through its thunk
/// (0x10002f20): visited, moved with the flag clear, moved with the flag
/// set (the model node set up, the owner reset, the node leaving).
#[test]
fn model_visit_0203() {
    let Some(mut t) = twin(203) else { return };
    let at = t.at(0x2e80);
    let thunk = t.at(0x2f20);
    let slot = 1000u32;
    t.set_u32(magic::TABLE + 4 * slot, TRAPS + 16 * slot);
    rec(&mut t, slot, 1, |m, a| m.r32(a[0]) ^ 0x1000);
    recs(
        &mut t,
        &[
            (275, 1),
            (538, 2),
            (213, 2),
            (211, 2),
            (35, 4),
            (34, 2),
            (547, 1),
            (276, 1),
        ],
    );
    let vt = SCRATCH + 0x900;
    t.set_u32(vt + 0x1c, TRAPS + 16 * slot);
    let (cur, node, model, owner, parent) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x1200,
        SCRATCH + 0x1400,
        SCRATCH + 0x1800,
    );
    for (k, (same, flag)) in [(true, 1u8), (false, 0), (false, 1)]
        .into_iter()
        .enumerate()
    {
        t.write(node, &varied(0x40, 0x22));
        t.set_u32(node, vt);
        t.set_u8(node + 0x1a, 5);
        t.set_u32(node + 0x10, node);
        t.set_u32(node + 0x14, 0);
        t.set_u32(node + 0x1c, model);
        t.set_u32(node + 0x20, owner);
        t.set_u32(node + 0x24, 0x0024_0024);
        t.set_u8(node + 0x28, flag);
        t.write(model, &varied(0xa0, 0x33));
        t.set_u32(model + 0x14, parent);
        t.set_u32(model + 0x90, 1);
        t.write(parent, &varied(0x20, 0x44));
        t.set_u16(parent + 0x18, 3);
        t.set_u32(parent + 0x10, model);
        t.set_u32(model + 8, parent);
        t.write(owner, &[0; 0x500]);
        t.set_u32(cur, node);
        t.set_u8(cur + 4, 0x7f);
        t.set_u8(cur + 5, if same { 5 } else { 6 });
        dirty(&mut t);
        let c = Call {
            eax: 0x1234_5678,
            ecx: cur,
            ..Call::default()
        };
        if k == 2 {
            t.check_entry(thunk, Call { ecx: 0, ..c }, &[cur], Eax::Checked);
        } else {
            t.check_entry(at, c, &[], Eax::Checked);
        }
    }
}

/// `m0206::records_start` (0x10001460): the records cleared.
#[test]
fn records_start_0206() {
    let Some(mut t) = twin(206) else { return };
    let at = t.at(0x1460);
    rec(&mut t, 719, 6, |_, a| a[0] ^ a[2] ^ 0x719);
    t.set_u32(t.img.word(at + 0x11), 0x00d0_0011);
    let rc = t.img.rel32_target(at + 0x5f);
    let recs_at = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(rc + 0x7), recs_at);
    t.write(recs_at, &varied(0x1000, 0x5a));
    for o in [0x68u32, 0x72, 0x7c] {
        t.set_u32(t.img.word(at + o), 0x7777_7777);
    }
    t.write(t.img.word(at + 0x38), &varied(0x30, 0x66));
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// `m0207::copy_node` (0x10006a80): the parent (a DLL global) with no
/// child and with one.
#[test]
fn copy_node_0207() {
    let Some(mut t) = twin(207) else { return };
    let at = t.at(0x6a80);
    rec(&mut t, 230, 3, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i) ^ 0x230;
            m.w32(a[0] + 4 * i, v);
        }
        0x230
    });
    rec(&mut t, 325, 2, |_, a| 0x0325_0000 ^ a[0]);
    let parent = t.img.word(at + 0x18);
    let (node, kid, src) = (SCRATCH + 0x1000, SCRATCH + 0x2800, SCRATCH + 0x3000);
    for i in 0..0x50u32 {
        t.set_u32(src + 4 * i, 0x5000_0000 + i);
    }
    for with_kid in [false, true] {
        t.write(node, &varied(0x200, 0x5c));
        t.write(kid, &varied(0x40, 0x2c));
        t.set_u32(parent + 0x10, if with_kid { kid } else { parent });
        t.set_u16(parent + 0x18, 5);
        dirty(&mut t);
        t.check(at, Call::this(node), &[src], Eax::Checked, |g, img, fr| {
            ffx_magic::m0207::copy_node(g, img, fr)
        });
    }
}
/// `m0250::grid_step` (0x10004940): a record without bit 15, the
/// objects running, all done (no grid: `G_N` 0).
#[test]
fn grid_step_0250() {
    let Some(mut t) = twin(250) else { return };
    let at = t.at(0x4940);
    let ro = t.img.rel32_target(at + 0x44);
    let grid = t.img.rel32_target(at + 0x6f);
    let _ = grid;
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    let recs_at = t.img.word(at + 0x1b);
    for (n, clear, done, running) in [
        (2u32, Some(1u32), true, false),
        (2, None, true, true),
        (u32::MAX, None, false, false),
        (0xffff_fffe, None, true, false),
        (2, None, true, false),
    ] {
        for i in 0..4 {
            // the record past the last one counted lacks the bit
            let v = if clear == Some(i) || i == 3 {
                0x7fff
            } else {
                0x8000 | i
            };
            t.set_u16(recs_at + 0xb4 * i, v.try_into().unwrap());
        }
        t.set_u32(t.img.word(at + 0x8), n);
        let sc = t.img.word(at + 0x38);
        script(&mut t, sc, done);
        magic_hi::objects(&mut t, ro, running);
        t.write(task, &varied(0x20, 0x11));
        t.set_u32(task + 8, r);
        t.write(r, &varied(0x20, 0x22));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0250::spark` (0x100049c0): flagged, the ring full, an entry taken.
#[test]
fn spark_0250() {
    let Some(mut t) = twin(250) else { return };
    let at = t.at(0x49c0);
    let rt = t.img.rel32_target(at + 0x23);
    let (rover, count, base) = (
        t.img.word(rt + 0x01),
        t.img.word(rt + 0x09),
        t.img.word(rt + 0x16),
    );
    let ring = SCRATCH + 0x1000;
    t.set_u32(base, ring);
    rec(&mut t, 217, 2, |m, a| {
        for i in 0..3 {
            m.w32(
                a[1] + 4 * i,
                (1.25 * i as f32 - 0.5 + (a[0] & 3) as f32).to_bits(),
            );
        }
        0x217
    });
    rec(&mut t, 314, 1, |_, a| a[0] ^ 0x314);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.3);
    rec(&mut t, 404, 1, |_, a| a[0] ^ 0x404);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) + 0.123_456_789);
    let a = SCRATCH + 0x300;
    t.set_u32(t.img.word(at + 0x4a), a);
    t.set_u32(a + 0xc, 0x00a0_000c);
    let p = SCRATCH + 0x400;
    for (flag, n) in [(1u8, 4u32), (0, 0), (0, 4)] {
        t.write(ring, &[0x55; 0x50 * 5]);
        for i in 0..5 {
            t.set_u32(ring + 0x50 * i + 0x48, u32::from(i == 0));
        }
        t.set_u32(count, n);
        t.set_u32(rover, 0);
        t.write(p, &varied(0xb0, 0x44));
        t.set_u8(p + 0xa4, flag);
        t.set_u32(p + 0xa0, 2);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[p], Eax::Checked);
    }
}

/// `m0249::camera_step` (0x10001a20): the motion running, the objects
/// running, the lights off, all done.
#[test]
fn camera_step_0249() {
    let Some(mut t) = twin(249) else { return };
    let at = t.at(0x1a20);
    let ms = t.img.rel32_target(at + 0x1a);
    let ro = t.img.rel32_target(at + 0x65);
    let pl = t.img.rel32_target(at + 0x21);
    recs(
        &mut t,
        &[
            (210, 2),
            (13, 4),
            (544, 2),
            (553, 6),
            (6, 2),
            (547, 3),
            (357, 6),
            (356, 9),
            (296, 2),
            (257, 2),
            (856, 0),
        ],
    );
    rec(&mut t, 404, 1, |_, a| 0x404 ^ a[0]);
    t.stub_st0(404, |_, _| 3.25);
    rec(&mut t, 857, 6, |_, _| 0x857);
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    for (done, running, on) in [
        (false, false, 0u32),
        (true, true, 0),
        (true, false, 1),
        (true, false, 0),
        (true, true, 1),
        (false, false, 1),
    ] {
        magic_hi::motion(&mut t, ms, done);
        magic_hi::objects(&mut t, ro, running);
        t.set_u32(t.img.word(pl + 0x12), on);
        // every light fading past its cap: the pulse answers 1
        let lights = t.img.word(pl + 0x3a);
        let cap = f32::from_bits(t.u32(t.img.word(pl + 0x187)));
        let step = f32::from_bits(t.u32(t.img.word(pl + 0x180)));
        for i in 0..6u32 {
            let l = lights + 0x34 * i;
            t.set_u32(l, 5);
            t.set_u32(l + 8, 1);
            t.set_u32(l - 8, (cap - step + 1.0).to_bits());
        }
        t.write(task, &varied(0x20, 0x11));
        t.set_u32(task + 8, r);
        t.write(r, &varied(0x20, 0x22));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// `m0253::record_start` (0x100018e0): the record's words dirty first.
#[test]
fn record_start_0253() {
    let Some(mut t) = twin(253) else { return };
    let at = t.at(0x18e0);
    let node = t.img.rel32_target(at + 0x1);
    let take = (0..0x80)
        .map(|p| node + p)
        .find(|&p| {
            t.img.word(p).to_le_bytes()[0] == 0xe8
                && t.img
                    .matches(t.img.rel32_target(p + 1), &ffx_magic::sa_pool::TAKE)
        })
        .map(|p| t.img.rel32_target(p + 1));
    for k in 0..2u32 {
        if let Some(tk) = take {
            magic_hi::pool(&mut t, tk, SCRATCH + 0x8000, 2 + k);
        }
        for o in [0x7u32, 0x11, 0x1b, 0x25, 0x2f, 0x39] {
            t.set_u32(t.img.word(at + o), 0x7777_0000 + o);
        }
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `m0260::programs_step` (0x10003bf0): the motion running, the objects
/// running, both done (the part programs with no parts).
#[test]
fn programs_step_0260() {
    let Some(mut t) = twin(260) else { return };
    let at = t.at(0x3bf0);
    let ms = t.img.rel32_target(at + 0x5);
    let ro = t.img.rel32_target(at + 0xc);
    recs(&mut t, &[(941, 1), (772, 2), (779, 2), (713, 1), (719, 3)]);
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    for (done, running) in [(false, false), (true, true), (true, false)] {
        magic_hi::motion(&mut t, ms, done);
        magic_hi::objects(&mut t, ro, running);
        t.write(task, &varied(0x20, 0x11));
        t.set_u32(task + 8, r);
        t.write(r, &varied(0x20, 0x22));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// The object start's set-up (`s4_misc::start_object_at` at `s`).
fn at_setup(t: &mut Twin, s: u32, count: u16, tail: u32) {
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    let pa = t.img.rel32_target(s + 0x97);
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), count);
    t.set_u32(t.img.word(s + 0xa2), tail);
}

/// `m0260::actor_start` (0x10003c30): actor 0 and 2, the angle above,
/// inside and below its range.
#[test]
fn actor_start_0260() {
    let Some(mut t) = twin(260) else { return };
    let at = t.at(0x3c30);
    let s = t.img.rel32_target(at + 0x39);
    rec(&mut t, 325, 6, |_, a| a[0] ^ a[1] ^ 0x325);
    let p = SCRATCH + 0x3000;
    let base = t.img.word(at + 0x22);
    for (k, (i, angle)) in [(0u32, 9.0f32), (2, 0.5), (0, -9.0), (1, 0.0)]
        .into_iter()
        .enumerate()
    {
        at_setup(&mut t, s, [3u16, 2, 0, 1][k], 0);
        t.write(base, &[0x5a; 0x130 * 3]);
        t.write(p, &varied(0x400, 0x44));
        t.set_u32(p + 0x390, i);
        t.set_u32(p + 0xc4, angle.to_bits());
        for o in [0xa8u32, 0xb2, 0xbc, 0xc6, 0xd0, 0xda, 0xe4, 0xee, 0xf8] {
            t.set_u32(t.img.word(at + o), 0x7777_7777);
        }
        dirty(&mut t);
        t.check_entry(at, Call::this(0x0ecc_0000 + i), &[p], Eax::Checked);
    }
}

/// `m0271::point_below` (0x100020c0).
#[test]
fn point_below_0271() {
    let Some(mut t) = twin(271) else { return };
    let at = t.at(0x20c0);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ a[1];
            m.w32(a[3] + 4 * i, v);
        }
        0x237
    });
    t.set_u32(t.img.word(at + 0x28), 0x00e0_0028);
    let a = SCRATCH + 0x3000;
    t.write(a, &varied(0x100, 0x44));
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[a], Eax::Checked);
}

/// `m0284::smoke_start` (0x10001ee0): no object, actor -1 and actor 2.
#[test]
fn smoke_start_0284() {
    let Some(mut t) = twin(284) else { return };
    let at = t.at(0x1ee0);
    let pt = t.img.rel32_target(at + 0x5);
    let s = t.img.rel32_target(at + 0x25);
    rec(&mut t, 325, 6, |_, a| a[0] ^ a[1] ^ 0x325);
    rec(&mut t, 314, 1, |_, a| a[0] ^ 0x314);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.5);
    rec(&mut t, 404, 1, |_, a| a[0] ^ 0x404);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) + 0.25);
    let list = SCRATCH + 0x700;
    t.set_u32(t.img.word(at + 0x6b), list);
    t.write(list, &[9, 8, 7, 6]);
    t.set_u32(t.img.word(at + 0x56), 0x00ee_0056);
    let p = SCRATCH + 0x3000;
    // the 0x140-byte records: two, both busy in case 0, the second free
    // after
    let recs_at = SCRATCH + 0x6000;
    t.set_u32(t.img.word(pt + 0x09), 2);
    t.set_u32(t.img.word(pt + 0x16), recs_at);
    for (k, actor) in [0u32, u32::MAX, 2].into_iter().enumerate() {
        t.set_u32(t.img.word(pt + 0x01), 0);
        t.write(recs_at, &varied(0x280, 0x41));
        for i in 0..2u32 {
            let busy = k == 0 || i == 0;
            t.set_u32(recs_at + 0x140 * i + 0x130, u32::from(busy));
            t.set_u16(recs_at + 0x140 * i + 0x124, 0x8000);
        }
        at_setup(&mut t, s, if k == 0 { 0 } else { 3 }, 0);
        t.write(p, &varied(0x300, 0x44));
        t.set_u32(p + 0x280, actor);
        dirty(&mut t);
        t.check_entry(at, Call::this(0x0ecc_0001), &[p], Eax::Checked);
    }
}

/// The object start's set-up (`s4_misc::start_object_owners` at `s`).
fn owners_setup(t: &mut Twin, s: u32, count: u16) {
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    // the part pool: the first rel32 call after +0x80 is its init
    let pa = (0x80..0xb0)
        .map(|o| s + o)
        .find(|&p| t.img.word(p).to_le_bytes()[0] == 0xe8)
        .map(|p| t.img.rel32_target(p + 1))
        .unwrap();
    let nodes = t.img.word(pa + 0x27) - 8;
    for i in 0..8 {
        t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
        t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
        t.set_u32(nodes + 12 * i + 8, 0);
    }
    t.set_u16(t.img.word(pa + 0x04), count);
}

/// `m0287::smoke_main` (0x10004e00): the pool free and empty.
#[test]
fn smoke_main_0287() {
    let Some(mut t) = twin(287) else { return };
    let at = t.at(0x4e00);
    let s = t.img.rel32_target(at + 0x1d);
    rec(&mut t, 325, 6, |_, a| a[0] ^ a[1] ^ 0x325);
    rec(&mut t, 314, 1, |_, a| a[0] ^ 0x314);
    t.stub_st0(314, |_, a| f64::from(f32::from_bits(a[0])) * 0.5);
    rec(&mut t, 404, 1, |_, a| a[0] ^ 0x404);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) + 0.25);
    let p = SCRATCH + 0x3000;
    for count in [3u16, 0] {
        owners_setup(&mut t, s, count);
        t.write(t.img.word(at + 0x18), &varied(0x130, 0x5a));
        t.write(p, &varied(0x300, 0x44));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[p], Eax::Checked);
    }
}

/// `m0295::leave_and_register` (0x10001e80): no parent and no entries;
/// a parent (the node first, last, in the middle) and entries in both
/// lists.
#[test]
fn leave_and_register_0295() {
    let Some(mut t) = twin(295) else { return };
    let at = t.at(0x1e80);
    rec(&mut t, 779, 2, |m, a| {
        let l = m.r32(SCRATCH + 0x10);
        a[0] ^ a[1] ^ m.r32(l)
    });
    let lock = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x6b), lock);
    let (cur, node, parent, a, b) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x1400,
        SCRATCH + 0x1800,
        SCRATCH + 0x1900,
    );
    t.set_u32(SCRATCH + 0x10, lock);
    for (k, (par, prev, next, n0, n1)) in [
        (false, a, b, 0u16, 0u16),
        (true, parent, b, 2, 1),
        (true, a, parent, 0, 3),
        (true, a, b, 1, 0),
    ]
    .into_iter()
    .enumerate()
    {
        t.write(node, &varied(0x40, 0x22));
        t.write(parent, &varied(0x20, 0x33));
        t.write(a, &varied(0x10, 0x44));
        t.write(b, &varied(0x10, 0x55));
        t.set_u32(node + 0x14, if par { parent } else { 0 });
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.set_u32(node + 0x24, 0x00a0_0000 + 0x100 * u32::try_from(k).unwrap());
        t.set_u32(node + 0x28, 0x8000_0000);
        t.set_u16(node + 0x2c, n0);
        t.set_u16(node + 0x2e, n1);
        t.set_u32(lock, 0x7700 + u32::try_from(k).unwrap());
        t.set_u32(cur, node);
        dirty(&mut t);
        t.check(at, Call::this(cur), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::Ret::eax(ffx_magic::m0295::leave_and_register(g, img, fr.at, fr.ecx))
        });
    }
}

/// `m0298::models_step` (0x10001da0): the script waiting, the objects
/// running, a record empty, no records, all set.
#[test]
fn models_step_0298() {
    let Some(mut t) = twin(298) else { return };
    let at = t.at(0x1da0);
    let ro = t.img.rel32_target(at + 0x13);
    rec(&mut t, 275, 1, |_, a| 0x0275_0000 ^ a[0]);
    let bytes = SCRATCH + 0x600;
    t.write(bytes, &[3, 5, 7, 9]);
    rec(&mut t, 288, 2, move |m, a| {
        m.w32(a[1], a[0] & 3);
        bytes + (a[0] & 1)
    });
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    rec(&mut t, 276, 2, |_, a| a[0] ^ a[1]);
    rec(&mut t, 219, 2, |_, a| if a[1] == 0 { a[0] } else { 0 });
    let (task, r) = (SCRATCH + 0x2000, SCRATCH + 0x2100);
    let recs_at = t.img.word(at + 0x42);
    for (done, running, n, empty) in [
        (false, false, 3u32, None),
        (true, true, 3, None),
        (true, false, 3, Some(1u32)),
        (true, false, 0, None),
        (true, false, 3, None),
    ] {
        let sc = t.img.word(at + 0x7);
        script(&mut t, sc, done);
        magic_hi::objects(&mut t, ro, running);
        t.set_u32(t.img.word(at + 0x37), n);
        for i in 0..4 {
            t.set_u32(recs_at + 0x54 * i, if empty == Some(i) { 0 } else { 1 + i });
        }
        t.write(task, &varied(0x20, 0x11));
        t.set_u32(task + 8, r);
        t.write(r, &varied(0x20, 0x22));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// Free rings of `n` nodes for every pool take reached from `at` (`len`
/// bytes, two calls deep).
fn pools(t: &mut Twin, at: u32, len: u32, n: u32) {
    fn takes(img: &ffx_magic::Image, at: u32, len: u32, depth: u32, out: &mut Vec<u32>) {
        for p in at..at + len {
            if img.word(p).to_le_bytes()[0] != 0xe8 {
                continue;
            }
            let t = img.rel32_target(p + 1);
            if !img.in_text(t) {
                continue;
            }
            if img.matches(t, &ffx_magic::sa_pool::TAKE) {
                if !out.contains(&t) {
                    out.push(t);
                }
            } else if depth > 0 {
                takes(img, t, 0x200, depth - 1, out);
            }
        }
    }
    let mut v = Vec::new();
    takes(&t.img, at, len, 2, &mut v);
    for (k, tk) in (0u32..).zip(v) {
        magic_hi::pool(t, tk, SCRATCH + 0x8000 + 0x400 * k, n);
    }
}

/// `m0301::object_setup` (0x100020c0) and `callbacks_setup`
/// (0x10002140).
#[test]
fn setups_0301() {
    let Some(mut t) = twin(301) else { return };
    recs(
        &mut t,
        &[(443, 2), (440, 6), (450, 3), (451, 10), (444, 1), (449, 5)],
    );
    t.set_u32(t.img.word(t.at(0x20c0) + 0x23), 0x00e0_0023);
    let a = SCRATCH + 0x3000;
    for rva in [0x20c0u32, 0x2140] {
        let at = t.at(rva);
        t.write(a, &varied(0x140, 0x44));
        dirty(&mut t);
        t.check_entry(at, Call::this(0x0ecc_0001), &[a], Eax::Checked);
    }
}

/// `m0303::record_nodes` (0x10001480): the record's words dirty first,
/// the lists empty.
#[test]
fn record_nodes_0303() {
    let Some(mut t) = twin(303) else { return };
    let at = t.at(0x1480);
    let node = t.img.rel32_target(at + 0x29);
    for k in 0..2u32 {
        pools(&mut t, node, 0x100, 3 + k);
        for o in [0x2u32, 0xc, 0x16, 0x20] {
            t.set_u32(t.img.word(at + o), 0x7777_0000 + o);
        }
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// A start's set-up found by its callees: the part lock and block, slot
/// 941's table and slot 714, and `count` free nodes in the pool of the
/// first `l5_objhost::pool_alloc` called from `s` (one call deep).
fn obj_setup(t: &mut Twin, s: u32, count: u16) {
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(t, 714, 5, |_, _| 0x714);
    let calls: Vec<u32> = (0..0x100)
        .map(|o| s + o)
        .filter(|&p| t.img.word(p).to_le_bytes()[0] == 0xe8)
        .map(|p| t.img.rel32_target(p + 1))
        .filter(|&c| t.img.in_text(c))
        .collect();
    for c in calls {
        if t.img.matches(c, &ffx_magic::l5_objhost::POOL_ALLOC) {
            let nodes = t.img.word(c + 0x27) - 8;
            for i in 0..8 {
                t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
                t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
                t.set_u32(nodes + 12 * i + 8, 0);
            }
            t.set_u16(t.img.word(c + 0x04), count);
            return;
        }
    }
}

/// `m0303::light_start` (0x100015c0): the pool free and empty.
#[test]
fn light_start_0303() {
    let Some(mut t) = twin(303) else { return };
    let at = t.at(0x15c0);
    let s = t.img.rel32_target(at + 0x29);
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x5e), SCRATCH + 0x300);
    rec(&mut t, 262, 4, |_, a| a[0] ^ a[1] ^ 0x262);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, (0.5 + i as f32 + (a[1] as f32)).to_bits());
        }
        0x268
    });
    t.set_u32(t.img.word(at + 0x34), 0x00e0_0034);
    for count in [3u16, 0] {
        obj_setup(&mut t, s, count);
        t.write(t.img.word(at + 0x24), &varied(0x130, 0x5a));
        dirty(&mut t);
        t.check_entry(at, Call::this(0x0ecc_0003), &[], Eax::Checked);
    }
}

/// `m0304::place_point` (0x10002e20).
#[test]
fn place_point_0304() {
    let Some(mut t) = twin(304) else { return };
    let at = t.at(0x2e20);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, (i as f32 * 0.25 - 1.0).to_bits());
        }
        0x235
    });
    rec(&mut t, 329, 3, |m, a| {
        let v = m.r32(a[1]) ^ a[2];
        m.w32(a[0], v);
        0x329
    });
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[2] + 4 * i)) + f32::from_bits(m.r32(a[1] + 4 * i));
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x229
    });
    let s = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x13), s);
    t.write(s + 0x1400, &varied(0x100, 0x3f));
    t.set_u32(s + 0x145c, 0.75f32.to_bits());
    dirty(&mut t);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// `m0304::host_start` (0x10002f30): the pool free and empty.
#[test]
fn host_start_0304() {
    let Some(mut t) = twin(304) else { return };
    let at = t.at(0x2f30);
    let s = t.img.rel32_target(at + 0x25);
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(&mut t, 213, 2, |_, a| a[0] ^ 0x213);
    rec(&mut t, 538, 2, |_, a| a[0] ^ 0x538);
    let host = t.img.word(at + 0x20);
    t.write(host, &[0; 0x40]);
    t.set_u32(host + 0x2c, SCRATCH + 0x500);
    let blk = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0xb), blk);
    for count in [3u16, 0] {
        obj_setup(&mut t, s, count);
        t.write(blk, &varied(0x1800, 0x5a));
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}
