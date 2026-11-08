//! Round 16's ports (`ffx_magic::r16`) against each original. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod magic;
mod magic_owna;
mod magic_r11;
mod magic_r13h;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

/// A load step at `at` (`r9_load::load_step`, the 119-byte shape; `d` 0
/// for the 118-byte one): busy, asking, not loaded, loaded (the start-up
/// runs on both sides).
fn load_cases(t: &mut magic::Twin, at: u32) {
    load_cases_d(t, at, 1);
}

fn load_cases_d(t: &mut magic::Twin, at: u32, d: u32) {
    let busy = SCRATCH + 0x5e0;
    let loaded = SCRATCH + 0x5e4;
    rec(t, 225, 1, move |m, _| m.r32(busy));
    rec(t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(t.img.word(at + 0x26 + d), 0x0000_5e5e);
    for (b, asked, l) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 2)] {
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(t.img.word(at + 0x18), asked);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `r16::dealt_start` (`magic_0619` 0x100031a0, `magic_0681`
/// 0x10003220) and its load step: two and three objects from three
/// dealt kinds, the yaw above, below and between its bounds, the ring
/// with three and four free nodes, the task list empty and not. (The
/// dealt bytes are kind numbers, so the -1 branches never run.)
#[test]
fn dealt_start() {
    for (dll, rva, load) in [(681u16, 0x3220u32, 0x30f0u32), (619, 0x31a0, 0x3070)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
        magic_r11::pools(&mut t, f, 0x60);
        magic_r11::object_slots(&mut t);
        let start = t.img.rel32_target(f + 0x121);
        magic_r11::object_start_auto(&mut t, start);
        // the deal: up to 6 draws from 3 kinds, 2 of each
        let deal = t.img.rel32_target(f + 0x57);
        t.set_u32(t.img.word(deal + 0x17), 3);
        for i in 0..3 {
            t.set_u32(t.img.word(deal + 0x46) + 0x1c * i, 2);
        }
        t.stub(324, |m, _| {
            let k = m.r32(SCRATCH + 0x710);
            m.w32(SCRATCH + 0x710, k + 1);
            k * 7 + 1
        });
        let yaw_in = SCRATCH + 0x714;
        rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
        t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw_in))));
        rec(&mut t, 404, 1, |_, a| a[0] ^ 0x404);
        t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 1.5 + 0.25);
        // the actors: the battle list's bytes and their table
        let list = SCRATCH + 0x180;
        for o in [0x136u32, 0x196] {
            t.set_u32(w(&t, o), list);
        }
        for i in 0..8u8 {
            t.set_u8(list + u32::from(i), 3 + 5 * i);
        }
        // (0x20 entries: the deal's counts follow the table in `magic_0619`)
        for i in 0..0x20 {
            t.set_u32(w(&t, 0x141) + 4 * i, 0x00c0_0000 + i);
        }
        t.set_u32(w(&t, 0x62), 0x00ac_7000);
        // junk in the objects and records the start-up sets
        t.write(w(&t, 0xd5) - 0xd4, &[0x5c; 0x130 * 3]);
        t.write(w(&t, 0xda) - 0x14, &[0x5d; 0x34 * 3]);
        let (count, head) = (w(&t, 0x211), w(&t, 0x2c6));
        let (first, last) = (w(&t, 0x326), w(&t, 0x340));
        let other = SCRATCH + 0x5f0;
        let ka = t.img.word(f + 0x7a);
        for (k, (n, total, yaw, free, listed)) in [
            (0u32, 0u32, -3.0f32, 3u16, false),
            (2, 4, -3.0, 3, false),
            (3, 6, 1.0, 4, true),
            (3, 3, 0.25, 3, true),
        ]
        .into_iter()
        .enumerate()
        {
            t.set_u32(SCRATCH + 0x710, k as u32);
            t.set_u32(t.img.word(deal + 0x1e), total);
            t.set_u32(w(&t, 0xca), n);
            // K - yaw: below, above and inside the bounds
            let kv = f32::from_bits(t.img.word(ka));
            let y = kv - yaw * if k == 2 { 0.0 } else { 7.0 };
            t.set_u32(yaw_in, y.to_bits());
            magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1400);
            t.write(other, &[0x44; 0x20]);
            t.set_u32(first, if listed { other } else { 0 });
            t.set_u32(last, other);
            t.check(
                f,
                Call::default(),
                &[],
                Eax::Checked,
                ffx_magic::r16::dealt_start,
            );
        }
        // the load step runs the start-up three times (a fresh ring each)
        magic_r11::ring(&mut t, count, head, 9, SCRATCH + 0x1400);
        let at = t.at(load);
        load_cases(&mut t, at);
    }
}

/// `r16::object_records_0480` (`magic_0480` 0x100043e0): the record's
/// index 0, 1 and 2, the yaw above, below and inside its bounds. The
/// frame words the record copies read and nothing writes are 0 on the
/// original's stack (junk reads, the port's 0).
#[test]
fn object_records_0480() {
    let Some(mut t) = twin(480) else { return };
    let f = t.at(0x43e0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x33);
    magic_r11::object_start_auto(&mut t, start);
    let yaw_in = SCRATCH + 0x714;
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw_in))));
    t.set_u32(w(&t, 0x4d), 0x00ac_7000);
    t.write(w(&t, 0x29), &[0x5c; 0x130 * 3]);
    let table = t.img.word(t.img.rel32_target(f + 0xf4) + 0x13);
    t.write(table, &[0x3c; 0x38 * 4]);
    let s = SCRATCH + 0x6000;
    let kv = f32::from_bits(t.img.word(w(&t, 0x9c)));
    for (i, yaw) in [(0u32, -3.0f32), (1, 3.0), (2, 0.0)] {
        t.write(s, &[0x2d; 0x40]);
        t.set_u32(s + 0x2c, i);
        t.set_u32(yaw_in, (kv - yaw * 7.0).to_bits());
        t.write(magic::STACK_TOP - 0x118, &[0; 0x18]);
        t.check_entry(f, Call::default(), &[s], Eax::Checked);
    }
}

/// `m0367::start_up_0369` in `magic_0480` (0x10004630, with the aim
/// records in the DLL's data and a pool node) and its load step
/// (0x10004270): the task list empty and not, one and two nodes free in
/// the aim records' ring.
#[test]
fn start_up_0480() {
    let Some(mut t) = twin(480) else { return };
    let f = t.at(0x4630);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 194);
    let lookup = t.img.rel32_target(f + 0x29);
    magic_r13h::lookup(&mut t, lookup);
    for (s, k) in [(719u32, 3usize), (271, 1)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    for s in [588u32, 544] {
        rec(&mut t, s, 2, move |m, a| {
            for i in 0..4 {
                m.w32(a[1] + 4 * i, ((s + a[0] + i) as f32 * 0.25 - 4.0).to_bits());
            }
            s
        });
    }
    let aim = t.img.rel32_target(f + 0x78);
    let o = t.img.word(aim + 0x1a) - 0x1e8;
    t.write(o, &[0x3a; 0x240]);
    let (count, head) = (t.img.word(aim + 0x278), t.img.word(aim + 0x2d7));
    let other = SCRATCH + 0x5f0;
    // (the start-up's own node comes off the same ring after the aim
    // records' one)
    for (free, listed) in [(2u16, false), (3, true), (2, true)] {
        magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1800);
        t.write(other, &[0x44; 0x20]);
        t.set_u32(w(&t, 0x88), if listed { other } else { 0 });
        t.set_u32(w(&t, 0x98), other);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0367::start_up_0369,
        );
    }
    magic_r11::ring(&mut t, count, head, 4, SCRATCH + 0x1800);
    let at = t.at(0x4270);
    load_cases_d(&mut t, at, 0);
}

/// `r16::start_up_0474` (`magic_0474` 0x100013e0) and its load step
/// (0x100012b0): the kind byte in 8..17 (the timed object) and not, both
/// yaws past a bound and inside, the task list empty and not.
#[test]
fn start_up_0474() {
    let Some(mut t) = twin(474) else { return };
    let f = t.at(0x13e0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 935);
    magic_r11::object_slots(&mut t);
    magic_r13h::point_268(&mut t);
    let start = t.img.rel32_target(f + 0x5f);
    magic_r11::object_start_auto(&mut t, start);
    let yaw_in = SCRATCH + 0x714;
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw_in))));
    rec(&mut t, 535, 1, |_, a| a[0] & 3);
    for s in [413u32, 215] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    let kind = SCRATCH + 0x190;
    // the byte after the kind is not 0, so a wider read shows
    t.set_u8(kind + 1, 0x77);
    for o in [0xfeu32, 0x1df, 0x224] {
        t.set_u32(w(&t, o), kind);
    }
    for o in [0x1f3u32, 0x240, 0x351] {
        for i in 0..0x20 {
            t.set_u32(w(&t, o) + 4 * i, 0x00c0_0000 + 0x100 * o + i);
        }
    }
    for o in [0x7fu32, 0x8c, 0x342] {
        t.set_u32(w(&t, o), 0x00ac_7000 + o);
    }
    t.write(w(&t, 0x182), &[0x5c; 0x100]);
    t.write(w(&t, 0x2a4), &[0x5d; 0x80]);
    t.write(w(&t, 0x323), &[0x5e; 0x80]);
    let kv = f32::from_bits(t.img.word(w(&t, 0xa9)));
    let other = SCRATCH + 0x5f0;
    for (k, yaw, listed) in [
        (9u8, 3.0f32, false),
        (3, -3.0, true),
        (17, 0.0, true),
        (18, 0.0, false),
    ] {
        t.set_u8(kind, k);
        t.set_u32(yaw_in, (kv - yaw * 7.0).to_bits());
        t.write(other, &[0x44; 0x20]);
        t.set_u32(w(&t, 0x369), if listed { other } else { 0 });
        t.set_u32(w(&t, 0x379), other);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r16::start_up_0474,
        );
    }
    // fresh rings for the load step's run of the start-up
    magic_r11::pools(&mut t, f, 935);
    let at = t.at(0x12b0);
    load_cases_d(&mut t, at, 0);
}

/// `r16::dealt_start_0494` (`magic_0494` 0x10002550) and its load step
/// (0x10002420): no, two and three objects from three dealt kinds, in a
/// battle and not, the ring with one and three free nodes, the task list
/// empty and not.
#[test]
fn dealt_start_0494() {
    let Some(mut t) = twin(494) else { return };
    let f = t.at(0x2550);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 0x50);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0xd1);
    magic_r11::object_start_auto(&mut t, start);
    let deal = t.img.rel32_target(f + 0x4d);
    t.set_u32(t.img.word(deal + 0x17), 3);
    for i in 0..3 {
        t.set_u32(t.img.word(deal + 0x46) + 0x1c * i, 2);
    }
    t.stub(324, |m, _| {
        let k = m.r32(SCRATCH + 0x710);
        m.w32(SCRATCH + 0x710, k + 1);
        k * 7 + 1
    });
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, |_, _| 1.5);
    rec(&mut t, 404, 1, |_, a| a[0] ^ 0x404);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 1.5 + 0.25);
    for s in [413u32, 215] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    let list = SCRATCH + 0x180;
    for o in [0xe6u32, 0x13f] {
        t.set_u32(w(&t, o), list);
    }
    for i in 0..8u8 {
        t.set_u8(list + u32::from(i), 3 + 5 * i);
    }
    // (0x20 entries: the deal's counts follow the table)
    for i in 0..0x20 {
        t.set_u32(w(&t, 0xf1) + 4 * i, 0x00c0_0000 + i);
    }
    t.set_u32(w(&t, 0x58), 0x00ac_7000);
    t.set_u32(w(&t, 0x1b7), 0x00ac_7001);
    t.write(w(&t, 0x81) - 0xd4, &[0x5c; 0x130 * 3]);
    t.write(w(&t, 0x86) - 0x14, &[0x5d; 0x34 * 3]);
    let (count, head) = (w(&t, 0x24b), w(&t, 0x25a));
    let (first, last) = (w(&t, 0x2c1), w(&t, 0x2d2));
    let other = SCRATCH + 0x5f0;
    for (k, (n, battle, free, listed)) in [
        (0u32, false, 1u16, false),
        (2, true, 3, true),
        (3, false, 1, true),
        // one object: the last owner 0, the second callback
        (1, true, 3, false),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(SCRATCH + 0x710, k as u32);
        t.set_u32(w(&t, 0x75), n);
        t.set_u32(w(&t, 0x1cf), u32::from(battle));
        magic_r11::ring(&mut t, count, head, free, SCRATCH + 0x1400);
        t.write(other, &[0x44; 0x20]);
        t.set_u32(first, if listed { other } else { 0 });
        t.set_u32(last, other);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r16::dealt_start_0494,
        );
    }
    magic_r11::ring(&mut t, count, head, 4, SCRATCH + 0x1400);
    let at = t.at(0x2420);
    load_cases_d(&mut t, at, 0);
}

/// `r16::setup_0666` (`magic_0666` 0x10002770): the slots' words and
/// answers, the game flag kept around slot 711, the three blocks over
/// junk.
#[test]
fn setup_0666() {
    let Some(mut t) = twin(666) else { return };
    let f = t.at(0x2770);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 719, 3, |_, _| SCRATCH + 0x7000);
    for (s, k) in [(718u32, 1usize), (222, 4), (710, 4), (260, 1)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x74), lock);
    t.set_u32(lock, 0x00a1_0000);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f4, m.r32(lock));
        a[1]
    });
    rec(&mut t, 941, 1, |_, a| a[0] ^ 0x0941_0000);
    // slot 719's handle points at its words +0x14 .. +0x1c
    t.write(SCRATCH + 0x7000, &[0x3c; 0x20]);
    for o in [0x118u32, 0x12e, 0x1f8] {
        t.write(w(&t, o), &[0x5a; 0x110]);
    }
    t.check_entry(f, Call::default(), &[0x00aa_0001], Eax::Checked);
}

/// `r16::place_0666` (`magic_0666` 0x10001ca0): the object inside the
/// DLL's table and outside it, slot 712 answering 0 and 1; the matrix
/// slots write their outputs from their inputs. The never-written local
/// `L(0xa4)` is 0 on the original's stack (junk read, the port's 0).
#[test]
fn place_0666() {
    let Some(mut t) = twin(666) else { return };
    let f = t.at(0x1ca0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    // the vector and matrix slots: each output word from the inputs
    for (s, n, words) in [
        (236u32, 3usize, 12u32),
        (0x3a0 / 4, 2, 4),
        (0x514 / 4, 2, 4),
        (0x428 / 4, 3, 4),
        (0x394 / 4, 3, 16),
        (0x3ac / 4, 1, 16),
        (0x39c / 4, 3, 16),
    ] {
        rec(&mut t, s, n, move |m, a| {
            let dst = if s == 236 { a[2] } else { a[0] };
            let src = a.get(1).copied().unwrap_or(0);
            for i in 0..words {
                let x = if s == 236 || s == 0x3ac / 4 {
                    0.25 * (i + s) as f32 - 3.0
                } else {
                    let x = f32::from_bits(m.r32(src.wrapping_add(4 * (i % 4))));
                    // the product's second matrix counts too
                    let y = match a.get(2) {
                        Some(&b) if s == 0x39c / 4 => f32::from_bits(m.r32(b + 4 * i)),
                        _ => 0.0,
                    };
                    x * 0.5 + y + i as f32
                };
                m.w32(dst + 4 * i, x.to_bits());
            }
            s
        });
    }
    let answer = SCRATCH + 0x718;
    // slot 230 keeps the matrix it gets, so the locals' values show
    rec(&mut t, 0x398 / 4, 2, |m, a| {
        let k = m.r32(SCRATCH + 0xa000);
        m.w32(SCRATCH + 0xa000, k + 1);
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(SCRATCH + 0xa004 + 0x40 * (k % 4) + 4 * i, v);
        }
        a[0] ^ 0x398
    });
    rec(&mut t, 712, 2, move |m, _| m.r32(answer));
    rec(&mut t, 713, 1, |_, a| a[0] ^ 0x713);
    let table = SCRATCH + 0x3400;
    t.write(table, &[0x3c; 0x40]);
    rec(&mut t, 941, 1, move |_, _| table);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x352), lock);
    t.set_u32(lock, 0x00a1_0000);
    for o in [0x345u32, 0x37d, 0x396] {
        t.set_u32(w(&t, o), SCRATCH + 0x7e0);
    }
    let h = SCRATCH + 0x7000;
    t.write(h, &[0x3c; 0x20]);
    t.set_u32(w(&t, 0x3a1), h);
    let sp = t.img.rel32_target(f + 0x3c4);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
    for o in [0x300u32, 0x30e, 0x319, 0x325, 0x32d, 0x338] {
        t.set_u32(w(&t, o), SCRATCH + 0x8000 + 0x40 * o);
    }
    t.set_u32(w(&t, 0x3b), 0x00ca_0054);
    let base = w(&t, 0x1a);
    for (obj, ans) in [(base + 0x40, 0u32), (SCRATCH + 0x9000, 1), (base, 1)] {
        t.write(obj, &[0x33; 0x80]);
        // the scale (and nothing like it around)
        t.set_u32(obj + 0x60, 1.5f32.to_bits());
        t.set_u32(answer, ans);
        t.write(magic::STACK_TOP - 0x1a8, &[0; 0x10]);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}
