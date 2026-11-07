//! Round 13's start-ups (`ffx_magic::r13_start`) against each original.
//! Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_owna;
mod magic_r11;
mod magic_r13h;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

/// `magic_0318` 0x10001340: the task list empty and not.
#[test]
fn two_objects_0318() {
    let Some(mut t) = twin(318) else { return };
    let f = t.at(0x1340);
    for listed in [false, true] {
        magic_r13h::two_objects_0318(&mut t, f, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r13_start::two_objects_0318,
        );
    }
}

/// Family P055 (`r13_misc::object_task`) in both copies: the task list
/// empty and not.
#[test]
fn object_tasks() {
    for (dll, rva) in [(610u16, 0x4670u32), (673, 0x43a0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        let o = SCRATCH + 0x7000;
        for listed in [false, true] {
            magic_r11::pools(&mut t, f, 112);
            t.write(o, &[0x6b; 0x140]);
            t.write(SCRATCH + 0x600, &[0x44; 0x10]);
            t.set_u32(op(&t, 0x36), if listed { SCRATCH + 0x600 } else { 0 });
            t.set_u32(op(&t, 0x4a), SCRATCH + 0x600);
            t.check(
                f,
                Call::this(o),
                &[],
                Eax::Checked,
                ffx_magic::r13_misc::object_task,
            );
        }
    }
}

/// Family P057 (`r13_misc::lists_done`) in both copies: the three host
/// lists empty with the scene word 1 (the sound stopped) and 0, and a
/// waiting object in each list in turn (that list's run answers 0).
#[test]
fn lists_done() {
    for (dll, rva) in [(304u16, 0x1560u32), (568, 0x1500)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        let run = t.img.rel32_target(f + 0x15);
        // the lists' run (`magic_s9_misc.rs`' `host_objects_run`): its
        // pointer and value globals, its slots
        for (o, at) in [
            (0x22u32, SCRATCH + 0x7c0),
            (0x33, SCRATCH + 0x7c4),
            (0x351, SCRATCH + 0x7c8),
            (0x35f, SCRATCH + 0x7d0),
            (0x3d7, SCRATCH + 0x7e0),
        ] {
            let g = t.img.word(run + o);
            t.set_u32(g, at);
        }
        for o in [0x40u32, 0x4c, 0x337, 0x382, 0x3ac] {
            let g = t.img.word(run + o);
            t.set_u32(g, 0x00a0_0000 + o);
        }
        for (s, n) in [
            (723u32, 1usize),
            (724, 1),
            (231, 3),
            (713, 1),
            (160, 3),
            (719, 3),
            (220, 6),
            (235, 1),
            (229, 3),
            (230, 2),
            (712, 2),
        ] {
            rec(&mut t, s, n, move |_, a| (s << 16) ^ a[0]);
        }
        rec(&mut t, 214, 1, |_, a| 0x0214_0000 ^ a[0]);
        let s_block = SCRATCH + 0x8000;
        t.set_u32(op(&t, 0x0b), s_block);
        t.set_u32(op(&t, 0x02), 0x0e0d_0001);
        t.set_u32(op(&t, 0x51), 0x0051_0051);
        let obj = SCRATCH + 0x9000;
        let node = SCRATCH + 0x9200;
        // (the list with a waiting object, the scene word)
        for (busy, scene) in [(9u32, 1u32), (9, 0), (0, 1), (1, 1), (2, 1)] {
            t.write(s_block, &[0x31; 0x1800]);
            t.set_u32(s_block + op(&t, 0x44), scene);
            for (i, o) in (0u32..).zip([0x10u32, 0x1c, 0x2a]) {
                let host = op(&t, o);
                t.write(host, &[0; 0x40]);
                if i == busy {
                    t.write(obj, &[0x22; 0x140]);
                    t.set_u16(obj + 0x124, 0x8000);
                    t.set_u32(obj + 0x114, 2);
                    t.set_u8(obj + 0x126, 0);
                    for w in [0x100u32, 0x104, 0x108, 0x10c, 0x110] {
                        t.set_u32(obj + w, 0);
                    }
                    t.set_u32(node, 0);
                    t.set_u32(node + 4, 0);
                    t.set_u32(node + 8, obj);
                    t.set_u32(host + 0x30, node);
                    t.set_u32(host + 0x34, node);
                    t.set_u32(host + 0x3c, 1);
                }
            }
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// A summon start of round 13 at `rva` (its `**G_LOCK` operand
/// `lock_op`; `r13_start::summon_start_0343`, `_0106`, `_0513`): the
/// slots answering distinct words (slot 711 copying the lock out), the
/// targets' slots, two actors and the lock word 0 and set.
fn summon(n: u16, rva: u32, lock_op: u32) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let lock = SCRATCH + 0x1f0;
    for (slot, k) in [
        (718u32, 1usize),
        (719, 3),
        (710, 4),
        (222, 4),
        (221, 4),
        (551, 3),
        (168, 1),
        (531, 2),
    ] {
        rec(&mut t, slot, k, move |_, a| (0x0700_0000 + slot) ^ a[0]);
    }
    rec(&mut t, 575, 0, |_, _| 0x0575);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x1f4, m.r32(lock));
        0x0711_0000 ^ a[0]
    });
    t.stub_st0(289, |_, _| -1.25);
    t.set_u32(t.img.word(at + lock_op), lock);
    for (actor, lk) in [(3u32, 0u32), (0x21, 7)] {
        t.set_u32(lock, lk);
        t.check_entry(at, Call::default(), &[actor], Eax::Checked);
    }
}

#[test]
fn summon_start_0343() {
    summon(343, 0x7210, 0xd4);
}

#[test]
fn summon_start_0106() {
    summon(106, 0x7560, 0xd4);
}

#[test]
fn summon_start_0513() {
    summon(513, 0x87d0, 0xde);
}

/// `magic_0429` 0x10001240 (`r13_start::set_up_0429`): the slots
/// answering distinct words, the lists' handle a block of words, slot 218
/// writing the argument and pointing at a byte, the cleared block dirty.
#[test]
fn set_up_0429() {
    let Some(mut t) = twin(429) else { return };
    let at = t.at(0x1240);
    let w = |t: &magic::Twin, o: u32| t.img.word(at + o);
    let h = SCRATCH + 0x700;
    for i in 0..16 {
        t.set_u32(h + 4 * i, 0x00d0_0000 + i);
    }
    rec(&mut t, 803, 2, |_, a| 0x0080_0000 ^ a[1]);
    rec(&mut t, 719, 3, move |_, a| {
        if a[1] == 8 { h } else { 0x0719_0000 ^ a[1] }
    });
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0xbe), lock);
    t.set_u32(lock, 0x00a1_0000);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f4, m.r32(lock));
        a[1]
    });
    let byte = SCRATCH + 0x7e0;
    t.set_u32(byte, 0x0000_0037);
    rec(&mut t, 218, 2, move |m, a| {
        m.w32(a[1], 0x0218_0000 ^ a[0]);
        byte
    });
    rec(&mut t, 215, 1, |_, a| 0x0215_0000 ^ a[0]);
    for (s, k) in [(718u32, 1usize), (222, 4), (221, 4), (710, 4)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    t.write(w(&t, 0x167), &[0x3c; 0x110]);
    t.check_entry(at, Call::default(), &[0x0a00_0429], Eax::Checked);
}

/// `magic_0313` 0x10001000 (`r13_start::measures_0313`): the targets'
/// points placed so that each side of `K1` and a NaN distance come up;
/// counts 3 and 6.
#[test]
fn measures_0313() {
    let Some(mut t) = twin(313) else { return };
    let at = t.at(0x1000);
    let a = SCRATCH + 0x800;
    t.set_u32(t.img.word(at + 0x15), a);
    for i in 0..8u32 {
        t.set_u32(a + 0x1a0 + 4 * i, 0x40 + i);
    }
    t.set_u32(a + 0x198, 0x3f);
    rec(&mut t, 456, 3, |m, a| {
        for (i, x) in (0u32..).zip([1.0f32, 0.5, -2.0, 1.0]) {
            m.w32(a[2] + 4 * i, x.to_bits());
        }
        0x456
    });
    rec(&mut t, 457, 3, |m, a| {
        let k = (a[0] & 7) as f32;
        for (i, x) in (0u32..).zip([k * 1.5 - 4.0, 0.25, 3.0 - k, 1.0]) {
            m.w32(a[2] + 4 * i, x.to_bits());
        }
        0x457
    });
    rec(&mut t, 453, 1, |_, a| a[0]);
    t.stub_st0(453, |m, a| {
        let x = f64::from(f32::from_bits(m.r32(a[0])));
        let z = f64::from(f32::from_bits(m.r32(a[0] + 8)));
        if x > 3.0 { f64::NAN } else { x * x + z * z }
    });
    rec(&mut t, 464, 1, |m, a| {
        let z = f32::from_bits(m.r32(a[0] + 8));
        // the arc cosine's argument: inside -1..1, once past it (NaN)
        m.w32(a[0] + 8, (z * 0.125 + 0.25).to_bits());
        0x464
    });
    for (s, k) in [(448u32, 2usize), (450, 2), (449, 2)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let o = SCRATCH + 0x1000;
    for n in [3u32, 6] {
        t.set_u32(a + 0x19c, n);
        t.check_entry(at, Call::default(), &[o], Eax::Checked);
    }
}

/// `magic_0664` 0x10001f40 (`r13_start::set_up_0664`): the slots
/// answering distinct words (slot 711 keeping the lock), the lists'
/// handle a block of words, the cleared blocks dirty before.
#[test]
fn set_up_0664() {
    let Some(mut t) = twin(664) else { return };
    let at = t.at(0x1f40);
    let w = |t: &magic::Twin, o: u32| t.img.word(at + o);
    let h = SCRATCH + 0x700;
    for i in 0..16 {
        t.set_u32(h + 4 * i, 0x00d0_0000 + i);
    }
    rec(&mut t, 719, 3, move |_, a| {
        if a[1] == 8 { h } else { 0x0719_0000 ^ a[1] }
    });
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 260, 1, |_, a| 0x0260_0000 ^ a[0]);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x74), lock);
    t.set_u32(lock, 0x00a1_0000);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f4, m.r32(lock));
        a[1]
    });
    for (s, k) in [(718u32, 1usize), (222, 4), (710, 4)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    for o in [0x118u32, 0x12e] {
        t.write(w(&t, o), &[0x3c; 0x110]);
    }
    t.check_entry(at, Call::default(), &[0x0a00_0664], Eax::Checked);
}

/// The aim start-ups `r13_start::start_0348` and `start_0350` (and their
/// load steps through `r9_load`): the sound list (slot 288, three bytes),
/// the angle (slot 333), the aim records' points (slots 588, 544), the
/// pools free, the task list empty and not.
#[test]
fn start_aim() {
    for (dll, rva, load) in [(348u16, 0x7490u32, 0x50f0u32), (350, 0x6b80, 0x5730)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let d = if dll == 350 { 0xa } else { 0 };
        let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
        magic_r11::pools(&mut t, f, 0x140);
        let lr = t.img.rel32_target(f + 0x51 + d);
        t.set_u32(t.img.word(lr + 0x09), SCRATCH + 0x1_0000);
        let list = SCRATCH + 0x3000;
        t.write(list, &[7, 9, 11, 13]);
        t.stub(288, move |m, a| {
            m.w32(a[1], 3);
            list
        });
        rec(&mut t, 333, 1, |_, a| 0x333 ^ a[0]);
        t.stub_st0(333, |_, _| 0.375);
        rec(&mut t, 12, 1, |_, a| 0x12 ^ a[0]);
        t.stub_st0(12, |_, _| 0.75);
        for s in [588u32, 544] {
            rec(&mut t, s, 2, move |m, a| {
                for i in 0..4u32 {
                    m.w32(a[1] + 4 * i, ((s + i) as f32 * 0.25 - 4.0).to_bits());
                }
                s
            });
        }
        for (s, k) in [
            (719u32, 3usize),
            (271, 3),
            (296, 2),
            (262, 4),
            (357, 3),
            (356, 3),
            (362, 4),
            (711, 3),
            (710, 4),
        ] {
            rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
        }
        t.set_u32(w(&t, 0xc8 + d), SCRATCH + 0x2_0000);
        let other = SCRATCH + 0x5f0;
        for listed in [false, true] {
            t.write(other, &[0x44; 0x10]);
            t.set_u32(w(&t, 0x9c + d), if listed { other } else { 0 });
            t.set_u32(w(&t, 0xb0 + d), other);
            t.check(
                f,
                Call::default(),
                &[],
                Eax::Checked,
                if dll == 348 {
                    ffx_magic::r13_start::start_0348
                } else {
                    ffx_magic::r13_start::start_0350
                },
            );
        }
        let at = t.at(load);
        load_cases(&mut t, at);
    }
}

/// A load step at `at` (`r9_load`): busy, asking, not loaded, loaded
/// (the start-up runs on both sides).
fn load_cases(t: &mut magic::Twin, at: u32) {
    let busy = SCRATCH + 0x5e0;
    let loaded = SCRATCH + 0x5e4;
    rec(t, 225, 1, move |m, _| m.r32(busy));
    rec(t, 226, 2, |_, a| a[0] ^ 0x226);
    rec(t, 227, 1, move |m, _| m.r32(loaded));
    t.set_u32(t.img.word(at + 0x26), 0x0000_5e5e);
    for (b, asked, l) in [(1u32, 0u32, 1u32), (0, 0, 0), (0, 1, 2)] {
        t.set_u32(busy, b);
        t.set_u32(loaded, l);
        t.set_u32(t.img.word(at + 0x18), asked);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// `r13_start::start_0473` and its load step: the aim records' points
/// (slots 588, 544), the pools free, the task list empty and not.
#[test]
fn start_0473() {
    let Some(mut t) = twin(473) else { return };
    let f = t.at(0x2100);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 377);
    for s in [588u32, 544] {
        rec(&mut t, s, 2, move |m, a| {
            for i in 0..4u32 {
                m.w32(a[1] + 4 * i, ((s + i) as f32 * 0.25 - 4.0).to_bits());
            }
            s
        });
    }
    for (s, k) in [(719u32, 3usize), (271, 3)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let b = SCRATCH + 0x2_0000;
    t.write(b, &vec![0x3a; 0x700]);
    t.set_u32(w(&t, 0x4c), b);
    let other = SCRATCH + 0x5f0;
    for listed in [false, true] {
        t.write(other, &[0x44; 0x10]);
        t.set_u32(w(&t, 0xec), if listed { other } else { 0 });
        t.set_u32(w(&t, 0x100), other);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r13_start::start_0473,
        );
    }
    let at = t.at(0x12b0);
    load_cases(&mut t, at);
}

/// `r13_start::start_0325` and its load step: the actors' lookup, the
/// part's buffer (slots 711, 710 with the lock), the fades' records and
/// pool, the task list empty and not.
#[test]
fn start_0325() {
    let Some(mut t) = twin(325) else { return };
    let f = t.at(0x13e0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 298);
    let lookup = t.img.rel32_target(f + 0x29);
    magic_r13h::lookup(&mut t, lookup);
    for (s, k) in [(719u32, 3usize), (271, 5), (710, 4)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let lock_at = t.img.word(t.img.rel32_target(f + 0x4c) + 0x1);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(lock_at, lock);
    t.set_u32(lock, 0x00a1_0000);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f4, m.r32(lock));
        a[1]
    });
    for o in [0x63u32, 0x82] {
        t.write(w(&t, o), &[0x2b; 0x38]);
    }
    let other = SCRATCH + 0x5f0;
    for listed in [false, true] {
        t.write(other, &[0x44; 0x10]);
        t.set_u32(w(&t, 0xe5), if listed { other } else { 0 });
        t.set_u32(w(&t, 0x100), other);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r13_start::start_0325,
        );
    }
    let at = t.at(0x12b0);
    load_cases(&mut t, at);
}
