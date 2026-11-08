//! `magic_0588.dll` against its Rust port (`ffx_magic::m0588`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::pool;
use magic_owna::{rec, recs, twin};

const N: u16 = 588;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// The current actor `i` of the code at `at` (record operand `e`,
/// table operand `table`).
fn actor(t: &mut Twin, at: u32, (e, table): (u32, u32), i: u8) {
    let rec = SCRATCH + 0x80;
    t.set_u32(t.img.word(at + e), rec);
    t.set_u8(rec + 1, i);
    // the byte after the index non-zero (an index read as u16 shows)
    t.set_u8(rec + 2, 0x3c);
    let table = t.img.word(at + table);
    for k in 0..8 {
        t.set_u32(table + 4 * k, 0x00ac_0000 + k);
    }
}

/// 0x10001e10: actors 0 and 3.
#[test]
fn sound() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1e10);
    rec(&mut t, 0, 1, |_, a| a[0] ^ 0x00f0_0000);
    recs(&mut t, &[(26, 3), (10, 5), (257, 7)]);
    // the byte after the one the start sets, non-zero
    let byte = t.img.word(at + 0x2a);
    for i in [0u8, 3] {
        t.set_u32(byte, 0x6d6d_6d00);
        actor(&mut t, at, (0x3e, 0x55), i);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002510: the list empty and not.
#[test]
fn part_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2510);
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x3), r);
    let take = t.img.rel32_target(at + 0x17);
    let head = SCRATCH + 0x300;
    for empty in [true, false] {
        pool(&mut t, take, SCRATCH + 0x1000, 3);
        t.write(r + 0xd00, &[0x3b; 0x200]);
        t.write(head, &[0x4d; 0x20]);
        t.set_u32(t.img.word(at + 0x1d), u32::from(!empty));
        t.set_u32(t.img.word(at + 0x31), head);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002800: actors 1 and 5.
#[test]
fn colours() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2800);
    recs(&mut t, &[(357, 3), (356, 6), (355, 8), (354, 10)]);
    for i in [1u8, 5] {
        actor(&mut t, at, (0x1, 0x12), i);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002860: one record.
#[test]
fn light() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2860);
    recs(&mut t, &[(235, 1), (328, 3), (329, 3), (248, 9)]);
    // slot 229 keeps the vector it gets
    rec(&mut t, 229, 6, |m, a| {
        magic_owna::dump(m, 229, a[2], 4);
        0x5a5a_0229
    });
    let r = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x11), r);
    t.write(r + 0xb00, &[0x3e; 0x400]);
    t.set_u32(r + 0xdf4, 0x3fc9_0fdb);
    magic_h6::scrub(&mut t);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// The actor matrix frame (shared with `magic_0600`): two actors, the
/// float slots answering in ST0, the matrix slots keeping what they get.
#[test]
fn actor_matrix() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x25b0);
    let s = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x18), s);
    let actors = SCRATCH + 0x300;
    t.set_u32(t.img.word(at + 0x11), actors);
    let hs = t.img.word(at + 0x25);
    for i in 0..8u32 {
        t.set_u32(hs + 4 * i, 0x00ac_0000 + i);
    }
    for (slot, n) in [(277u32, 1usize), (15, 1), (16, 1), (17, 1)] {
        magic_owna::rec(&mut t, slot, n, move |_, _| slot);
        t.stub_st0(slot, move |_, a| {
            0.375 + f64::from(slot) / 64.0 + f64::from(a[0] & 7)
        });
    }
    for (slot, n) in [
        (210u32, 2usize),
        (211, 2),
        (235, 1),
        (330, 3),
        (328, 3),
        (329, 3),
        (257, 2),
    ] {
        magic_owna::rec(&mut t, slot, n, move |m, a| {
            if slot == 210 {
                for i in 0..16u32 {
                    m.w32(a[1] + 4 * i, (1.5 + i as f32).to_bits());
                }
            }
            if slot >= 328 {
                magic_owna::dump(m, 0x100 + slot % 0x40, a[0], 16);
            }
            slot
        });
    }
    for b in [1u8, 3] {
        t.write(s, &[0x2b; 0x1000]);
        for i in 0..0x40u32 {
            t.set_u32(s + 0xb00 + 4 * i, (0.5 + 0.25 * i as f32).to_bits());
        }
        t.write(actors, &[b; 4]);
        magic_h6::scrub(&mut t);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            ffx_magic::m0588::actor_matrix(g, img, fr)
        });
    }
}

/// 0x10002420 and `magic_0600` 0x10007250 (`m0588::spawn_pair`, family
/// P076): the count still running, the pool empty, both spawned, the
/// second record missing.
#[test]
fn spawn_pair() {
    for (n, rva) in [(588u16, 0x2420u32), (600, 0x7250)] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        let w = |t: &magic::Twin, op: u32| t.img.word(at + op);
        let fx = SCRATCH + 0x1_0000;
        t.set_u32(w(&t, 0x3), fx);
        let so = t.img.rel32_target(at + 0x44);
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        magic_owna::rec(&mut t, 941, 1, move |_, _| table);
        magic_owna::rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x2f0);
        t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x280);
        let sp = t.img.rel32_target(so + 0x84);
        t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
        let take = t.img.rel32_target(at + 0x26);
        let recs_at = SCRATCH + 0x2_0000;
        t.set_u32(t.img.word(take + 0x16), recs_at);
        let (count, point, angle) = (w(&t, 0x9), w(&t, 0x4a), w(&t, 0x7a));
        for (c, records) in [(5u32, 2u32), (1, 0), (1, 2), (0, 1)] {
            t.set_u32(fx + count, c);
            t.set_u32(t.img.word(take + 0x09), records);
            t.set_u32(t.img.word(take + 0x01), 0);
            t.write(recs_at, &[0x5a; 0x280]);
            for i in 0..2 {
                t.set_u32(recs_at + 0x140 * i + 0x130, 0);
                t.set_u16(recs_at + 0x140 * i + 0x124, 0x8000);
            }
            for i in 0..4 {
                t.set_u32(fx + point + 4 * i, 0x0c04_0000 + i);
            }
            t.set_u32(fx + angle, 0x0df4_0001);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// 0x10002230 (the two objects) and 0x10002320 (the old object listed
/// and not).
#[test]
fn objects_and_light() {
    let Some(mut t) = twin(N) else { return };
    let fx = SCRATCH + 0x1_0000;
    let at = t.at(0x2230);
    t.set_u32(t.img.word(at + 0xc), fx);
    let so = t.img.rel32_target(at + 0x1e);
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
    t.write(fx, &[0x3c; 0xe00]);
    for i in 0..4 {
        t.set_u32(fx + 0xc04 + 4 * i, magic_hi::f(1.5 + i as f32));
    }
    magic_h6::scrub(&mut t);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
    let at = t.at(0x2320);
    let st = t.img.rel32_target(at + 0x2b);
    let take = t.img.rel32_target(st + 0x7b);
    pool(&mut t, take, SCRATCH + 0x6000, 3);
    recs(&mut t, &[(713, 1), (332, 1)]);
    let list = t.img.word(at + 0x3b);
    let n = SCRATCH + 0x7000;
    t.set_u32(t.img.word(at + 0x5d), SCRATCH + 0x7100);
    for (k, op) in [0xa0u32, 0xad, 0xba, 0xca].into_iter().enumerate() {
        t.set_u32(t.img.word(at + op), magic_hi::f(0.5 + k as f32));
    }
    for listed in [false, true] {
        t.write(n, &[0; 0x10]);
        t.set_u32(n + 8, fx + 0x980);
        t.set_u32(list, if listed { n } else { 0 });
        t.set_u32(list + 4, if listed { n } else { 0 });
        t.set_u16(fx + 0x980 + 0x124, 0x8123);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::this(0x00ec_0001), &[], Eax::Checked);
    }
}

/// `r14::light_restart_0430` (`magic_0430` 0x10002d80): the old object
/// listed and not; the scale task's count from `edx`.
#[test]
fn light_restart_0430() {
    let Some(mut t) = twin(430) else { return };
    let fx = SCRATCH + 0x1_0000;
    let at = t.at(0x2d80);
    t.set_u32(t.img.word(at + 0x1c), fx);
    let st = t.img.rel32_target(at + 0x30);
    let take = t.img.rel32_target(st + 0x73);
    pool(&mut t, take, SCRATCH + 0x6000, 3);
    recs(&mut t, &[(713, 1), (332, 1)]);
    let list = t.img.word(at + 0x40);
    let n = SCRATCH + 0x7000;
    t.set_u32(t.img.word(at + 0x62), SCRATCH + 0x7100);
    for (k, op) in [0xbdu32, 0xca, 0xd7, 0xe7].into_iter().enumerate() {
        t.set_u32(t.img.word(at + op), magic_hi::f(0.5 + k as f32));
    }
    t.write(fx, &[0x3c; 0xf00]);
    for i in 0..4 {
        t.set_u32(fx + 0xcc4 + 4 * i, magic_hi::f(1.5 + i as f32));
    }
    t.set_u32(fx + 0x924, magic_hi::f(0.75));
    for listed in [false, true] {
        t.write(n, &[0; 0x10]);
        t.set_u32(n + 8, fx + 0x980);
        t.set_u32(list, if listed { n } else { 0 });
        t.set_u32(list + 4, if listed { n } else { 0 });
        t.set_u16(fx + 0x980 + 0x124, 0x8123);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::this(0x00ec_0001), &[], Eax::Checked);
    }
}
