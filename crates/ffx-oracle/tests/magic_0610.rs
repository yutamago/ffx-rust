//! `magic_0610.dll` against its Rust port (`ffx_magic::m0610`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0610};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 610;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x1000a610: no targets; a target with a listed kind; one slot 580
/// rejects; all accepted.
#[test]
fn check_targets() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0xa610);
    recs(&mut t, &[(4, 1), (27, 3), (523, 2)]);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 580, 1, move |m, a| u32::from(m.r32(cell) == a[0]));
    let list = SCRATCH + 0x300;
    t.set_u32(t.img.word(at + 0x54), list);
    t.write(list, &[1, 2, 3, 4]);
    let objs = t.img.word(at + 0x5f);
    let kinds = t.img.word(at + 0x6a);
    let listed = t.u32(kinds + 8);
    for (k, id) in (0u32..).zip(1..5u32) {
        let o = SCRATCH + 0x400 + 0x10 * k;
        t.set_u32(objs + 4 * id, o);
        // the half-word after the kind non-zero (a kind read as u32 shows)
        t.set_u32(o, 0x5a5a_fff0 + id);
    }
    for (n, kind, reject) in [
        (0i32, None, 0u32),
        (4, Some(3u32), 0),
        (4, None, 2),
        (4, None, 0),
    ] {
        t.set_u32(t.img.word(at + 0x3b), n.cast_unsigned());
        for id in 1..5u32 {
            let o = SCRATCH + 0x400 + 0x10 * (id - 1);
            let k = if kind == Some(id) {
                listed
            } else {
                0xfff0 + id
            };
            t.set_u16(o, u16::try_from(k & 0xffff).unwrap());
        }
        t.set_u32(cell, reject);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0610::check_targets(g, img, fr))
        });
    }
}

/// 0x10006f60, `l5_objhost::start`'s copy with the target check after
/// slot 720: the object block's byte list empty and holding targets (a
/// listed kind, one slot 580 rejects, all accepted).
#[test]
fn objhost_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6f60);
    let bytes = SCRATCH + 0x2000;
    let count = SCRATCH + 0x20;
    rec(&mut t, 168, 1, |_, _| SCRATCH + 0x1000);
    rec(&mut t, 218, 2, move |m, a| {
        let n = m.r32(count);
        m.w32(a[1], n);
        bytes
    });
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 + a[0]);
    rec(&mut t, 413, 1, |_, a| 0x0413_0000 + a[0]);
    rec(&mut t, 291, 5, |_, a| 0x10 + a[1]);
    rec(&mut t, 367, 8, |_, a| 0x0367_0000 + a[1]);
    recs(
        &mut t,
        &[
            (556, 1),
            (555, 2),
            (331, 3),
            (720, 2),
            (718, 3),
            (222, 7),
            (221, 11),
            (4, 1),
            (27, 3),
            (523, 2),
        ],
    );
    rec(&mut t, 790, 0, |_, _| 0x790);
    t.stub_st0(790, |_, _| 0.1);
    let cell = SCRATCH + 0x10;
    rec(&mut t, 580, 1, move |m, a| u32::from(m.r32(cell) == a[0]));
    let check = t.img.rel32_target(at + 0xc7);
    let list = SCRATCH + 0x300;
    t.set_u32(t.img.word(check + 0x54), list);
    t.write(list, &[1, 2, 3, 4]);
    let objs = t.img.word(check + 0x5f);
    let listed = t.u32(t.img.word(check + 0x6a) + 8);
    let payloads = t.img.word(at + 0x81);
    for (k, (n, kind, reject)) in [
        (0i32, None, 0u32),
        (4, Some(3u32), 0),
        (4, None, 2),
        (4, None, 0),
    ]
    .into_iter()
    .enumerate()
    {
        t.write(payloads, &vec![0xa5; 12 * 0x3ff]);
        t.set_u32(count, k as u32 * 2);
        for (i, b) in [3u8, 1, 4, 1, 5, 9, 2, 6].iter().enumerate() {
            t.set_u8(bytes + i as u32, *b);
        }
        t.set_u32(t.img.word(check + 0x3b), n.cast_unsigned());
        for id in 1..5u32 {
            let o = SCRATCH + 0x400 + 0x10 * (id - 1);
            t.set_u32(objs + 4 * id, o);
            t.set_u32(o, 0x5a5a_fff0 + id);
            let kd = if kind == Some(id) {
                listed
            } else {
                0xfff0 + id
            };
            t.set_u16(o, u16::try_from(kd & 0xffff).unwrap());
        }
        t.set_u32(cell, reject);
        t.check_entry(
            at,
            Call {
                ecx: 0x55,
                ..Call::default()
            },
            &[0x00ab_c000 + k as u32],
            Eax::Checked,
        );
    }
}

/// 0x1000bb00: an odd count (nothing), the pool empty, a spark. Also
/// `magic_0434`'s copy (0x10007bd0, the animated object start).
#[test]
fn spark() {
    for (n, rva) in [(N, 0xbb00u32), (434, 0x7bd0)] {
        spark_in(n, rva);
    }
}

fn spark_in(n: u16, rva: u32) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x16), fx);
    t.set_u32(fx + 0x1a68, 0x1a68_0001);
    let so = t.img.rel32_target(at + 0x61);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x2f0);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x280);
    let sp = t.img.rel32_target(so + 0x84);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |_, _| 0.625);
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, _| -3.25);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, 0x4000_0000 + i);
        }
        0x268
    });
    let b = SCRATCH + 0x60;
    t.set_u32(t.img.word(at + 0x66), b);
    t.set_u32(b, 0x0000_0302);
    let actors = t.img.word(at + 0x79);
    for i in 0..8 {
        t.set_u32(actors + 4 * i, 0x00ac_0000 + i);
    }
    t.set_u32(t.img.word(at + 0xa4), 0x00a4_0001);
    let take = t.img.rel32_target(at + 0x1c);
    let recs_at = SCRATCH + 0x2_0000;
    t.set_u32(t.img.word(take + 0x16), recs_at);
    let r = SCRATCH + 0x100;
    for (count, records) in [(4u32, 1u32), (5, 0), (7, 1)] {
        t.write(r, &[0x3c; 0x40]);
        t.set_u32(r + 0x2c, count);
        t.set_u32(t.img.word(take + 0x09), records);
        t.set_u32(t.img.word(take + 0x01), 0);
        t.write(recs_at, &[0x5a; 0x140]);
        t.set_u32(recs_at + 0x130, 0);
        t.set_u16(recs_at + 0x124, 0x8000);
        t.check_entry(at, Call::default(), &[r], Eax::Checked);
    }
}

/// 0x1000ba00: `k` above 1 (nothing), the pool empty, sides 0 and 1.
/// Also `magic_0434`'s copy (0x10007ad0, the animated object start).
#[test]
fn side_object() {
    for (n, rva) in [(N, 0xba00u32), (434, 0x7ad0)] {
        side_object_in(n, rva);
    }
}

fn side_object_in(n: u16, rva: u32) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x12), fx);
    t.set_u32(fx + 0x1a68, 0x1a68_0001);
    let so = t.img.rel32_target(at + 0x60);
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x2f0);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x280);
    let sp = t.img.rel32_target(so + 0x84);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |_, _| 0.625);
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, _| -3.25);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, (0x4000_0000 + i) ^ a[1]);
        }
        0x237
    });
    t.set_u32(t.img.word(at + 0x81), 0x0081_0001);
    let take = t.img.rel32_target(at + 0x1b);
    let recs_at = SCRATCH + 0x2_0000;
    t.set_u32(t.img.word(take + 0x16), recs_at);
    for (k, records) in [(2u32, 1u32), (0, 0), (0, 1), (1, 1)] {
        t.set_u32(t.img.word(take + 0x09), records);
        t.set_u32(t.img.word(take + 0x01), 0);
        t.write(recs_at, &[0x5a; 0x140]);
        t.set_u32(recs_at + 0x130, 0);
        t.set_u16(recs_at + 0x124, 0x8000);
        t.check_entry(at, Call::default(), &[0x77, k], Eax::Checked);
    }
}
