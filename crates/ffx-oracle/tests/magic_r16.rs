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

/// `r16::frame_0661` (`magic_0661` 0x10001f50): off, and on with slot 261
/// answering 0 and 1 (the cues with no timeline cued).
#[test]
fn frame_0661() {
    let Some(mut t) = twin(661) else { return };
    let f = t.at(0x1f50);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let answer = SCRATCH + 0x718;
    rec(
        &mut t,
        261,
        1,
        move |m, a| if a[0] == 0x1c { m.r32(answer) } else { 0 },
    );
    for (s, k) in [(723u32, 1usize), (724, 1), (214, 1)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    for (o, v) in [
        (0x13u32, 0x00aa_0013u32),
        (0x31, 0x00aa_0031),
        (0x44, 0x00aa_0044),
        (0x6c, 0x00aa_006c),
    ] {
        t.set_u32(w(&t, o), v);
    }
    for o in [0x0du32, 0x3f] {
        t.set_u32(w(&t, o), SCRATCH + 0x7e0 + o);
    }
    for (on, ans) in [(0u32, 0u32), (1, 0), (1, 1)] {
        t.set_u32(w(&t, 0x02), on);
        t.set_u32(answer, ans);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r16::setup_0661` (`magic_0661` 0x10001ce0): the slots' words and
/// answers, the game flag kept around slot 711, the timelines over junk.
#[test]
fn setup_0661() {
    let Some(mut t) = twin(661) else { return };
    let f = t.at(0x1ce0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    rec(&mut t, 803, 2, |_, _| SCRATCH + 0x1_0000);
    rec(&mut t, 719, 3, |_, _| SCRATCH + 0x7000);
    for (s, k) in [(718u32, 1usize), (222, 4), (710, 4)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x99), lock);
    t.set_u32(lock, 0x00a1_0000);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f4, m.r32(lock));
        a[1]
    });
    rec(&mut t, 941, 1, |_, a| a[0] ^ 0x0941_0000);
    t.write(SCRATCH + 0x7000, &[0x3c; 0x20]);
    let first = w(&t, 0x13e) - 4;
    let end = w(&t, 0x198);
    t.write(first, &vec![0x5a; (end - first) as usize]);
    t.check_entry(f, Call::default(), &[0x00aa_0001], Eax::Checked);
}

/// `r16::texture_entry_0661` and `r16::texture_0661` (`magic_0661`
/// 0x10001530, 0x10001000): the texture word's widths and slot 885
/// answering 0 and 1, the offsets of either sign, the colour sums below
/// 0, inside and above their bound.
#[test]
fn texture_0661() {
    let Some(mut t) = twin(661) else { return };
    let f = t.at(0x1530);
    let x = t.img.rel32_target(f + 0x2e);
    let w = |t: &magic::Twin, o: u32| t.img.word(x + o);
    let tex = SCRATCH + 0x7000;
    rec(&mut t, 719, 3, move |_, _| tex);
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..0x10 {
            m.w32(a[1] + 4 * i, 0x2100_0000 + i);
        }
        0x210
    });
    for (s, k) in [(235u32, 1usize), (230, 2), (825, 1)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    // slot 879 keeps the four colours it is handed (a frame local)
    rec(&mut t, 879, 10, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[4] + 4 * i);
            m.w32(SCRATCH + 0x7c00 + 4 * i, v);
        }
        0x879
    });
    let answer = SCRATCH + 0x718;
    rec(&mut t, 885, 7, move |m, a| {
        for i in 0..8 {
            m.w32(a[6] + 4 * i, 0x0885_0000 + i);
        }
        m.r32(answer)
    });
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x15d), lock);
    t.set_u32(lock, 0x00a1_0000);
    for o in [0x8bu32, 0x27, 0x7f, 0x85, 0x95] {
        t.set_u32(w(&t, o), 0x00ab_0000 + o);
    }
    let colours = w(&t, 0x222);
    // (the colour bytes are set by the function itself before it reads
    // them; written here only so that a wrong address shows)
    for (lo, hi, xo, yo, bytes, ans) in [
        (0u32, 0u32, 0i32, 0i32, [0u8; 8], 0u32),
        (
            0x1234_5678,
            0x0000_0003,
            0x0012_3456,
            -0x0003_2100,
            [0x10, 0x20, 0x30, 0x40, 0x80, 0xff, 0x7f, 0x01],
            1,
        ),
        (
            0xbfff_ffff,
            0x0000_0001,
            -0x0abc_def0,
            0x0765_4321,
            [0xff, 0x00, 0x90, 0xfe, 0x81, 0x7f, 0x05, 0x00],
            1,
        ),
    ] {
        t.set_u32(tex, lo);
        t.set_u32(tex + 4, hi);
        t.set_u32(w(&t, 0x1ca), xo.cast_unsigned());
        t.set_u32(w(&t, 0x1f1), yo.cast_unsigned());
        // the unsigned bytes at +0x0.., the signed ones 4 later
        t.write(colours, &bytes);
        t.set_u32(answer, ans);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0489`'s slots for the sprite spawns: slot 324 a fixed sequence
/// (its count at `SCRATCH + 0x710`), slot 403's ST0, the sprite blocks'
/// resource slots (942, 941 with a header).
fn spawn_slots_0489(t: &mut magic::Twin) {
    t.stub(324, |m, _| {
        let k = m.r32(SCRATCH + 0x710);
        m.w32(SCRATCH + 0x710, k + 1);
        k.wrapping_mul(0x9e37_79b9) >> 7
    });
    rec(t, 403, 1, |_, a| a[0] ^ 0x403);
    t.stub_st0(403, |_, a| f64::from(f32::from_bits(a[0])) * 0.75 - 0.125);
    rec(t, 942, 1, |_, a| a[0] ^ 0x0942_0000);
    let data = SCRATCH + 0x3400;
    t.write(data, &[0x11; 0x40]);
    t.set_u16(data + 0x10, 0x20);
    t.set_u16(data + 0x12, 0x30);
    rec(t, 941, 1, move |_, _| data);
}

/// A sprite pool of three at `SCRATCH + 0x2000` (0x94 bytes each from
/// `+0x100`): the first `live` taken.
fn pool_0489(t: &mut magic::Twin, live: u32) -> u32 {
    let pool = SCRATCH + 0x2000;
    let first = pool + 0x100;
    t.write(first, &[0x3c; 0x94 * 3]);
    for i in 0..3 {
        t.set_u32(first + 0x94 * i, u32::from(i < live));
    }
    t.set_u32(pool, 3);
    t.set_u32(pool + 4, first);
    pool
}

/// `r16::spawn_0489` and `r16::spawn2_0489` (`magic_0489` 0x10001000,
/// 0x10001430): a free sprite and none, three draws each.
#[test]
fn spawns_0489() {
    let Some(mut t) = twin(489) else { return };
    spawn_slots_0489(&mut t);
    let (pos, vel) = (SCRATCH + 0x600, SCRATCH + 0x620);
    for (i, x) in [1.5f32, -2.25, 4.0, 0.5, -0.75, 3.0]
        .into_iter()
        .enumerate()
    {
        t.set_u32(
            SCRATCH + 0x600 + 4 * i as u32 + if i < 3 { 0 } else { 0x14 },
            x.to_bits(),
        );
    }
    for (rva, port) in [
        (
            0x1000u32,
            ffx_magic::r16::spawn_0489 as fn(&mut _, &_, &_) -> _,
        ),
        (0x1430, ffx_magic::r16::spawn2_0489),
    ] {
        let f = t.at(rva);
        t.set_u32(t.img.word(f + 0x05), 0x00ae_0005);
        for (k, live) in [1u32, 3, 0].into_iter().enumerate() {
            t.set_u32(SCRATCH + 0x710, 7 * k as u32);
            let pool = pool_0489(&mut t, live);
            let c = Call {
                ecx: pool,
                ..Call::default()
            };
            t.check(f, c, &[pos, vel], Eax::Checked, port);
        }
    }
}

/// `r16::spray_0489` and `r16::spray2_0489` (`magic_0489` 0x10001260,
/// 0x100016e0): the way of zero length and not, slot 261 answering 0 and
/// 1; the pool full (the spawns find no sprite, the step and frames see
/// live sprites as junk-free records) and empty.
#[test]
fn sprays_0489() {
    let Some(mut t) = twin(489) else { return };
    spawn_slots_0489(&mut t);
    rec(&mut t, 0x3b4 / 4, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, (1.0f32 + i as f32).to_bits());
        }
        0x3b4
    });
    let answer = SCRATCH + 0x718;
    rec(&mut t, 261, 1, move |m, _| m.r32(answer));
    // the sprites' draw handle is 64 bits (slot 772's edx)
    rec(&mut t, 772, 2, |_, a| a[0] ^ a[1]);
    t.stub_edx(772, |_, _| 0x0772_0002);
    for rva in [0x1260u32, 0x16e0] {
        let f = t.at(rva);
        let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
        let g0 = if rva == 0x1260 { 0x39u32 } else { 0x38 };
        for (k, (way, ans)) in [(0.0f32, 0u32), (2.5, 0), (-1.25, 1)]
            .into_iter()
            .enumerate()
        {
            for c in 0..3u32 {
                let to = t.img.word(f + g0 + 0x10 * c);
                let from = t.img.word(f + g0 + 8 + 0x10 * c);
                t.set_u32(to, (way * (c + 1) as f32).to_bits());
                t.set_u32(from, 0);
            }
            t.set_u32(answer, ans);
            t.set_u32(SCRATCH + 0x710, 5 * k as u32);
            // the last case: three free sprites (spawned, their table words
            // set, stepped and drawn)
            let pool = if k == 2 {
                pool_0489(&mut t, 0)
            } else {
                let pool = SCRATCH + 0x2000;
                t.set_u32(pool, 0);
                t.set_u32(pool + 4, pool + 0x100);
                pool
            };
            t.set_u32(w(&t, 0x2f - (0x39 - g0)), 0x00ab_0237);
            t.check_entry(f, Call::default(), &[pool], Eax::Checked);
        }
    }
}

/// `r16::caster_start_0450` (`magic_0450` 0x10005b40): the block's
/// object, matrix and scale task over junk, two yaws.
#[test]
fn caster_start_0450() {
    let Some(mut t) = twin(450) else { return };
    let f = t.at(0x5b40);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 689);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x31);
    magic_r11::object_start_auto(&mut t, start);
    for (s, v) in [(277u32, 0.75f64), (15, 0.5), (16, -0.25), (17, 2.0)] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 0xf));
    }
    for (s, k) in [
        (235u32, 3usize),
        (330, 3),
        (328, 3),
        (329, 3),
        (210, 2),
        (211, 2),
    ] {
        rec(&mut t, s, k, move |m, a| {
            // the matrix slots keep what they see
            let n = m.r32(SCRATCH + 0x7c00);
            m.w32(SCRATCH + 0x7c00, n + 1);
            for i in 0..12 {
                let v = m.r32(a[0].wrapping_add(4 * i));
                m.w32(SCRATCH + 0x7c04 + 0x30 * (n % 8) + 4 * i, v);
            }
            (s << 16) ^ a[0]
        });
    }
    for o in [0xceu32, 0xff, 0x211] {
        t.set_u32(w(&t, o), 0x00ab_0000 + o);
    }
    let b = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x1b), b);
    for (k, yaw) in [(0u32, 0.5f32), (1, -1.75)] {
        t.write(b, &vec![0x3c + k as u8; 0x2000]);
        t.set_u32(b + 0x1ee0, yaw.to_bits());
        for c in 0..4u32 {
            t.set_u32(b + 0x1c10 + 4 * c, (1.0 + c as f32 + k as f32).to_bits());
        }
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r16::glide_0450` (`magic_0450` 0x10007d50): the counter at 0, at 1
/// (the scale task starts) and above; the keys and points over varied
/// floats.
#[test]
fn glide_0450() {
    let Some(mut t) = twin(450) else { return };
    let f = t.at(0x7d50);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 609);
    for (s, v) in [(277u32, 0.75f64), (15, 0.5), (16, -0.25), (17, 2.0)] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 0xf));
    }
    for (s, k) in [(235u32, 1usize), (330, 3), (328, 3), (329, 3)] {
        rec(&mut t, s, k, move |m, a| {
            let n = m.r32(SCRATCH + 0x7c00);
            m.w32(SCRATCH + 0x7c00, n + 1);
            for i in 0..12 {
                let v = m.r32(a[0].wrapping_add(4 * i));
                m.w32(SCRATCH + 0x7c04 + 0x30 * (n % 8) + 4 * i, v);
            }
            (s << 16) ^ a[0]
        });
    }
    for o in [0x14cu32, 0x17d] {
        t.set_u32(w(&t, o), 0x00ab_0000 + o);
    }
    for (o, n) in [(0x39u32, 8u32), (0xcd, 12)] {
        for i in 0..n {
            t.set_u32(w(&t, o) + 4 * i, (0.375 * i as f32 - 1.0).to_bits());
        }
    }
    // the yaw keys past half a turn, so the wrap runs
    for i in 0..12u32 {
        t.set_u32(w(&t, 0xcd) + 4 * i, (2.75 + 0.0625 * i as f32).to_bits());
    }
    let b = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x07), b);
    for (k, n) in [0u32, 1, 5].into_iter().enumerate() {
        t.write(b, &vec![0x3c + k as u8; 0x2000]);
        for c in 0..12u32 {
            t.set_u32(
                b + 0x1c10 + 4 * c,
                (1.0 + 0.5 * c as f32 + k as f32).to_bits(),
            );
        }
        t.set_u32(b + 0x1ef8, n);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `r16::rise_0450` (`magic_0450` 0x10007980): the counter below 0 (the
/// entry's `eax`), 0 (the objects, the record and the node), 1 and 2
/// (the sparks first, `sparks_p034`'s set-up as its own test has it);
/// the object listed and not, outside a battle (actor below 0x20 and
/// not) and in one; the ring with two free nodes, the task list empty
/// and not.
#[test]
fn rise_0450() {
    let Some(mut t) = twin(450) else { return };
    let f = t.at(0x7980);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let sp = t.img.rel32_target(f + 0x19);
    let sop = |t: &magic::Twin, o: u32| t.img.word(sp + o);
    let t140 = t.img.rel32_target(sp + 0x1b);
    let t74 = t.img.rel32_target(sp + 0xe2);
    let calls = SCRATCH + 0x88;
    t.stub_st0(314, move |m, a| {
        let k = m.r32(calls);
        m.w32(calls, k + 1);
        f64::from(f32::from_bits(a[0])) * 0.5 + f64::from(k) * 0.125
    });
    rec(&mut t, 314, 1, |_, a| a[0] ^ 0x314);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(sp + 0x64);
    magic_r11::object_start_auto(&mut t, start);
    let start = t.img.rel32_target(f + 0x11f);
    magic_r11::object_start_auto(&mut t, start);
    for (s, k) in [(713u32, 1usize), (296, 2), (257, 2)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let b = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x0b), b);
    // the DLL's list (the operand is the list itself)
    let list = w(&t, 0xcf);
    t.set_u32(w(&t, 0xdd), SCRATCH + 0x5900);
    let actor = SCRATCH + 0x5a00;
    t.set_u32(w(&t, 0x26a), actor);
    for i in 0..0x40 {
        t.set_u32(w(&t, 0x2a3) + 4 * i, 0x00c0_0000 + i);
    }
    let other = SCRATCH + 0x5f0;
    for (k, (counter, listed, battle, a, tasks)) in [
        (-1i32, false, 0u32, 3u8, false),
        (0, true, 0, 3, false),
        (0, false, 0, 0x20, true),
        (0, true, 1, 5, true),
        (2, false, 0, 3, false),
        (1, true, 0, 3, true),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(calls, 0);
        t.write(b, &vec![0x3b + k as u8; 0x2000]);
        for c in 0..12u32 {
            t.set_u32(
                b + 0x1c30 + 4 * c,
                (1.0 + 0.5 * c as f32 + k as f32).to_bits(),
            );
        }
        t.set_u32(b + 0x1ef0, counter.cast_unsigned());
        t.set_u32(b + sop(&t, 0x10), 5);
        // the sparks' records (as `sparks_p034`'s own test)
        let p = SCRATCH + 0x700;
        t.set_u32(sop(&t, 0xf2), p);
        t.set_u32(p + 0x1c, 0x001c_001c);
        let objs_at = SCRATCH + 0x1_4000;
        t.write(objs_at, &vec![0x3c; 0x280]);
        t.set_u32(t.img.word(t140 + 0x01), 0);
        t.set_u32(t.img.word(t140 + 0x09), 2);
        t.set_u32(t.img.word(t140 + 0x16), objs_at);
        for i in 0..2 {
            t.set_u32(objs_at + 0x140 * i + 0x130, 0);
            t.set_u16(objs_at + 0x140 * i + 0x124, 0x8000);
        }
        let spr = SCRATCH + 0x1_6000;
        t.write(spr, &vec![0x3d; 0x74 * 4]);
        t.set_u32(t.img.word(t74 + 0x08), 0);
        t.set_u32(t.img.word(t74 + 0x0f), 4);
        t.set_u32(t.img.word(t74 + 0x37), spr);
        t.set_u32(t.img.word(t74 + 0x2b), 0);
        for i in 0..4 {
            t.set_u32(spr + 0x74 * i + 0x6c, 0);
        }
        // the DLL's list: the object's node at its head, or empty
        let node = SCRATCH + 0x5880;
        t.write(list, &[0; 0x10]);
        t.write(node, &[0; 0xc]);
        if listed {
            t.set_u32(list, node);
            t.set_u32(list + 4, node);
            t.set_u32(list + 0xc, 1);
            t.set_u32(node + 8, b + 0x130);
        }
        let (cnt, head) = (w(&t, 0x12c), w(&t, 0x1cc));
        magic_r11::ring(&mut t, cnt, head, 2, SCRATCH + 0x1800);
        t.write(other, &[0x44; 0x20]);
        t.set_u32(w(&t, 0x233), if tasks { other } else { 0 });
        t.set_u32(w(&t, 0x247), other);
        t.set_u32(w(&t, 0x263), battle);
        t.set_u8(actor + 1, a);
        let c = Call {
            eax: 0x00ea_0000 + k as u32,
            ..Call::default()
        };
        t.check_entry(f, c, &[], Eax::Checked);
    }
}

/// `r16::launch_0450` (`magic_0450` 0x10007100): the object listed and
/// not; the vector and matrix slots write from their inputs (each kept in
/// a log), the actor's point and the launch way of two lengths.
#[test]
fn launch_0450() {
    let Some(mut t) = twin(450) else { return };
    let f = t.at(0x7100);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x40f);
    magic_r11::object_start_auto(&mut t, start);
    let point = SCRATCH + 0x7d00;
    rec(&mut t, 268, 3, move |m, a| {
        for i in 0..4 {
            m.w32(a[2] + 4 * i, m.r32(point + 4 * i));
        }
        0x268
    });
    for (s, v) in [(277u32, 0.75f64), (15, 0.5), (16, -0.25), (17, 2.0)] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
        t.stub_st0(s, move |_, a| v + f64::from(a[0] & 0xf));
    }
    // the vector and matrix slots: the output (first word) from the
    // second word's floats, logged
    for (s, k) in [
        (235u32, 1usize),
        (328, 3),
        (329, 3),
        (330, 3),
        (229, 3),
        (248, 3),
        (210, 2),
        (211, 2),
        (713, 1),
    ] {
        rec(&mut t, s, k, move |m, a| {
            let n = m.r32(SCRATCH + 0x7c00);
            m.w32(SCRATCH + 0x7c00, n + 1);
            // (229 and 248: their vector, the last word)
            let src = match (s, k) {
                (229 | 248, _) => a[2],
                (_, 1) => a[0],
                _ => a[1],
            };
            for i in 0..4 {
                let v = m.r32(src.wrapping_add(4 * i));
                m.w32(SCRATCH + 0x7c04 + 0x10 * (n % 16) + 4 * i, v);
                if matches!(s, 229 | 248) {
                    let x = f32::from_bits(v) * 0.5 + i as f32;
                    m.w32(a[0] + 4 * i, x.to_bits());
                }
            }
            (s << 16) ^ a[0]
        });
    }
    let b = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x1f), b);
    let list = w(&t, 0x24);
    t.set_u32(w(&t, 0x3a), SCRATCH + 0x5900);
    let sel = SCRATCH + 0x5a00;
    for o in [0x62u32, 0x221, 0x25e, 0x381] {
        t.set_u32(w(&t, o), sel);
    }
    t.set_u8(sel, 3);
    t.set_u8(sel + 1, 5);
    // (the byte after the second actor's is not 0, so a wider read shows)
    t.set_u8(sel + 2, 0x77);
    for o in [0x6cu32, 0x22c, 0x269, 0x38c] {
        for i in 0..0x10 {
            t.set_u32(w(&t, o) + 4 * i, 0x00c0_0000 + 0x100 * o + i);
        }
    }
    // (the third point far behind on both axes: the yaw wraps)
    for (k, (listed, p)) in [
        (true, [1.5f32, 2.0, -3.0, 1.0]),
        (false, [0.25, -4.0, 6.5, 1.0]),
        (true, [-50.0, 3.0, -60.0, 1.0]),
    ]
    .into_iter()
    .enumerate()
    {
        t.write(b, &vec![0x3b + k as u8; 0x2000]);
        t.set_u32(b + 0x1ee0, (0.5f32 + k as f32).to_bits());
        t.set_u32(b + 0x1ba4, (2.5f32 - k as f32).to_bits());
        for (i, x) in (0u32..).zip(p) {
            t.set_u32(point + 4 * i, x.to_bits());
        }
        let node = SCRATCH + 0x5880;
        t.write(list, &[0; 0x10]);
        t.write(node, &[0; 0xc]);
        if listed {
            t.set_u32(list, node);
            t.set_u32(list + 4, node);
            t.set_u32(list + 0xc, 1);
            t.set_u32(node + 8, b + 0x5f0);
            t.set_u16(b + 0x5f0 + 0x124, 0x8123 + k as u16);
        }
        t.check_entry(f, Call::default(), &[], Eax::Junk);
    }
}

/// `r16::dealt_kind_start_0675` (`magic_0675` 0x100013e0) and its load
/// step (0x100012b0): two and three objects whose dealt actors are of
/// kind 0 and not (slot 534 by the actor's number), the yaw past each
/// bound and inside, the ring with three and four free nodes, the task
/// list empty and not.
#[test]
fn dealt_kind_start_0675() {
    let Some(mut t) = twin(675) else { return };
    let f = t.at(0x13e0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 0x60);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x14f);
    magic_r11::object_start_auto(&mut t, start);
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
    rec(&mut t, 12, 1, |_, a| a[0] ^ 0x12);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.25);
    // the kinds: none in the DLL's list, slot 534 by the number's parity
    rec(&mut t, 390, 1, |_, a| 0x7777_0000 ^ a[0]);
    rec(&mut t, 534, 1, |_, a| a[0] & 1);
    let list = SCRATCH + 0x180;
    let kc = t.img.rel32_target(f + 0x130);
    for o in [0x164u32, 0x1eb, 0x252] {
        t.set_u32(w(&t, o), list);
    }
    for o in [0x10u32, 0x74] {
        t.set_u32(t.img.word(kc + o), list);
    }
    for i in 0..8u8 {
        t.set_u8(list + u32::from(i), 3 + 5 * i);
    }
    // (0x20 entries: the deal's counts follow the tables)
    for o in [0x16fu32, 0x1f6] {
        for i in 0..0x20 {
            t.set_u32(w(&t, o) + 4 * i, 0x00c0_0000 + 0x100 * o + i);
        }
    }
    t.set_u32(w(&t, 0x62), 0x00ac_7000);
    t.write(w(&t, 0xdf) - 0xd4, &[0x5c; 0x130 * 3]);
    t.write(w(&t, 0xe4) - 0x14, &[0x5d; 0x34 * 3]);
    let fb = f + 0xb5;
    let wb = |t: &magic::Twin, o: u32| t.img.word(fb + o);
    let (count, head) = (wb(&t, 0x211), wb(&t, 0x2c6));
    let (first, last) = (wb(&t, 0x326), wb(&t, 0x340));
    let other = SCRATCH + 0x5f0;
    let ka = t.img.word(f + 0x7a);
    for (k, (n, total, yaw, free, listed)) in [
        (2u32, 4u32, -3.0f32, 3u16, false),
        (3, 6, 1.0, 4, true),
        (3, 3, 0.25, 3, true),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(SCRATCH + 0x710, k as u32);
        t.set_u32(t.img.word(deal + 0x1e), total);
        t.set_u32(w(&t, 0xd4), n);
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
            ffx_magic::r16::dealt_kind_start_0675,
        );
    }
    magic_r11::ring(&mut t, count, head, 9, SCRATCH + 0x1400);
    let at = t.at(0x12b0);
    load_cases(&mut t, at);
}

/// `r16::dealt_nodes_start_0492` (`magic_0492` 0x10002a30) and its load
/// step (0x100028f0): no, two and three objects whose dealt actors are
/// of kind 0 and not, in a battle and not, the yaw past each bound and
/// inside, the ring with enough free nodes and one short at the end, the
/// task list empty and not.
#[test]
fn dealt_nodes_start_0492() {
    let Some(mut t) = twin(492) else { return };
    let f = t.at(0x2a30);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 0x50);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x13b);
    magic_r11::object_start_auto(&mut t, start);
    let deal = t.img.rel32_target(f + 0x4b);
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
    rec(&mut t, 12, 1, |_, a| a[0] ^ 0x12);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.25);
    rec(&mut t, 390, 1, |_, a| 0x7777_0000 ^ a[0]);
    rec(&mut t, 534, 1, |_, a| a[0] & 1);
    for s in [413u32, 215] {
        rec(&mut t, s, 1, move |_, a| (s << 16) ^ a[0]);
    }
    let list = SCRATCH + 0x180;
    let kc = t.img.rel32_target(f + 0x120);
    for o in [0x150u32, 0x1c9, 0x229] {
        t.set_u32(w(&t, o), list);
    }
    for o in [0x10u32, 0x74] {
        t.set_u32(t.img.word(kc + o), list);
    }
    for i in 0..8u8 {
        t.set_u8(list + u32::from(i), 3 + 5 * i);
    }
    // (0x20 entries: the deal's counts follow the tables)
    for o in [0x15bu32, 0x1d4] {
        for i in 0..0x20 {
            t.set_u32(w(&t, o) + 4 * i, 0x00c0_0000 + 0x100 * o + i);
        }
    }
    t.set_u32(w(&t, 0x56), 0x00ac_7000);
    t.set_u32(w(&t, 0x3c1), 0x00ac_7001);
    t.write(w(&t, 0xcd) - 0xd4, &[0x5c; 0x130 * 3]);
    t.write(w(&t, 0xd4) - 0x14, &[0x5d; 0x34 * 3]);
    t.write(w(&t, 0xc8) - 8, &[0x5e; 0x34 * 3 + 8]);
    let (count, head) = (w(&t, 0x2cd), w(&t, 0x2d9));
    let (first, last) = (w(&t, 0x347), w(&t, 0x35c));
    let other = SCRATCH + 0x5f0;
    let ka = t.img.word(f + 0x70);
    for (k, (n, total, yaw, free, listed, battle)) in [
        (0u32, 0u32, 0.5f32, 3u16, false, false),
        (2, 4, -3.0, 6, false, true),
        (3, 6, 1.0, 7, true, false),
        (3, 3, 0.25, 9, true, true),
    ]
    .into_iter()
    .enumerate()
    {
        t.set_u32(SCRATCH + 0x710, k as u32);
        t.set_u32(t.img.word(deal + 0x1e), total);
        t.set_u32(w(&t, 0xbd), n);
        t.set_u32(w(&t, 0x3d4), u32::from(battle));
        let kv = f32::from_bits(t.img.word(ka));
        let y = kv - yaw * if k == 3 { 0.0 } else { 7.0 };
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
            ffx_magic::r16::dealt_nodes_start_0492,
        );
    }
    magic_r11::ring(&mut t, count, head, 9, SCRATCH + 0x1400);
    let at = t.at(0x28f0);
    load_cases_d(&mut t, at, 0);
}
