//! Round 13's start-ups (`ffx_magic::r13_start`) against each original.
//! Needs `FFX_GAME_DIR`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::many_single_char_names
)]

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

/// `r13_start::start_0253` and its load step: the global node's slots
/// (217, 234, 233), the camera's (287, 283, 281), the character slot by
/// the battle flag both ways, the pools free, the task list empty and
/// not.
#[test]
fn start_0253() {
    let Some(mut t) = twin(253) else { return };
    let f = t.at(0x19c0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 498);
    rec(&mut t, 217, 2, |m, a| {
        for i in 0..4u32 {
            m.w32(a[1] + 4 * i, (1.5 + i as f32).to_bits());
        }
        0x217
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..3u32 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) - f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x234
    });
    rec(&mut t, 233, 3, |m, a| {
        for i in 0..4u32 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i)) * f32::from_bits(a[2]);
            m.w32(a[0] + 4 * i, x.to_bits());
        }
        0x233
    });
    for (s, k) in [
        (719u32, 3usize),
        (287, 3),
        (283, 2),
        (281, 1),
        (413, 1),
        (215, 1),
        (525, 1),
    ] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    t.set_u32(w(&t, 0x84), 0x00c0_0084);
    t.set_u32(w(&t, 0x10b), 0x00c0_010b);
    t.set_u32(w(&t, 0x120), 2);
    // the record word the start-up clears holds junk first
    t.set_u32(w(&t, 0x131) + 2 * 0x148, 0x5a5a_5a5b);
    let other = SCRATCH + 0x5f0;
    for (listed, battle) in [(false, 0u32), (true, 1)] {
        t.write(other, &[0x44; 0x10]);
        t.set_u32(w(&t, 0x1b4), if listed { other } else { 0 });
        t.set_u32(w(&t, 0x1c4), other);
        t.set_u32(w(&t, 0x8d), battle);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r13_start::start_0253,
        );
    }
    let at = t.at(0x1210);
    load_cases(&mut t, at);
}

/// `r13_start::start_0288` and its load step: the actor byte, the
/// character slots by the battle flag both ways, the free list, the task
/// list empty and not.
#[test]
fn start_0288() {
    let Some(mut t) = twin(288) else { return };
    let f = t.at(0x1340);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 504);
    for (s, k) in [(719u32, 3usize), (413, 1), (215, 1)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let sel = SCRATCH + 0x5d0;
    t.write(sel, &[0x0b, 0x0c]);
    t.set_u32(w(&t, 0x45), sel);
    t.set_u32(w(&t, 0x143), 0x00c0_0143);
    let other = SCRATCH + 0x5f0;
    for (listed, battle) in [(false, 0u32), (true, 1)] {
        t.write(other, &[0x44; 0x10]);
        t.set_u32(w(&t, 0x1bd), if listed { other } else { 0 });
        t.set_u32(w(&t, 0x1ce), other);
        t.set_u32(w(&t, 0xab), battle);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r13_start::start_0288,
        );
    }
    let at = t.at(0x1210);
    load_cases(&mut t, at);
}

/// Family P049 (`r13_misc::scripts_step`) in both copies: both scripts
/// done (no held node, and one held), the second still waiting, and the
/// first waiting (the record's words copied, the motion scripts and the
/// rigs' step on the DLL's own data); the task's node goes back.
#[test]
fn scripts_steps() {
    for (dll, rva) in [(596u16, 0x6900u32), (304, 0x1490)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
        for (s, k) in [
            (941u32, 1usize),
            (772, 2),
            (779, 4),
            (326, 4),
            (229, 3),
            (284, 3),
            (285, 2),
            (286, 2),
            (235, 1),
            (330, 3),
            (328, 3),
            (329, 3),
        ] {
            rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
        }
        t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
        rec(&mut t, 314, 1, |_, a| a[0]);
        t.stub_st0(314, |_, a| f64::from(a[0] & 0xff) * 0.0625 - 7.0);
        let r = SCRATCH + 0x1_0000;
        t.set_u32(w(&t, 0x07), r);
        t.set_u32(w(&t, 0x26), 0x00c0_0026);
        let (task, held) = (SCRATCH + 0x5000, SCRATCH + 0x5200);
        for (k, (s1, s2, hold)) in [(0u8, 0u8, false), (0, 2, true), (2, 0, false), (2, 2, true)]
            .into_iter()
            .enumerate()
        {
            let k = k as u32;
            t.write(r, &vec![0x37; 0x1800]);
            for (i, x) in (0u32..).zip([1.5f32, -2.0, 0.25, 1.0]) {
                t.set_u32(r + t.img.word(f + 0x4e) + 4 * i, (x + k as f32).to_bits());
            }
            t.write(w(&t, 0x10), &[0x5c; 0x30]);
            for (ctx, st) in [(w(&t, 0x2e), s1), (w(&t, 0x3d), s2)] {
                t.write(ctx, &[0; 0x58]);
                t.set_u32(ctx, SCRATCH + 0x700);
                t.set_u8(ctx + 0x56, st);
                t.set_u16(ctx + 0x54, 0xffff);
            }
            // (the second context ends right before `G_R`)
            t.set_u32(w(&t, 0x07), r);
            // the nodes at rest for the pool's give-back
            for n in [task, held] {
                t.write(n, &[0x21; 0x20]);
                t.set_u32(n, 0);
                t.set_u32(n + 4, 0);
                t.set_u32(n + 8, SCRATCH + 0x5100);
            }
            t.set_u32(SCRATCH + 0x510c, 0);
            t.set_u32(w(&t, 0x9a), if hold { held } else { 0 });
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// What `s9_misc::host_objects_run` at `run` needs (as `lists_done`'s
/// set-up): its pointer and value globals, its slots.
fn host_run_setup(t: &mut magic::Twin, run: u32) {
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
        rec(t, s, n, move |_, a| (s << 16) ^ a[0]);
    }
}

/// A host's list (`host + 0x30`) empty, or holding one waiting object.
fn host_list(t: &mut magic::Twin, host: u32, busy: bool) {
    let (obj, node) = (SCRATCH + 0x9000, SCRATCH + 0x9200);
    t.write(host, &[0; 0x40]);
    if busy {
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

/// `magic_0244` 0x10001560 (`r13_misc::lists_two_done_0244`): both
/// lists empty (the task moves on), and a waiting object in each in turn.
#[test]
fn lists_two_done_0244() {
    let Some(mut t) = twin(244) else { return };
    let f = t.at(0x1560);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let run = t.img.rel32_target(f + 0x10);
    host_run_setup(&mut t, run);
    t.set_u32(w(&t, 0x05), 0x0e0d_0001);
    t.set_u32(w(&t, 0x16), 0x0e0d_0002);
    let (task, node) = (SCRATCH + 0x5000, SCRATCH + 0x5100);
    for busy in [9u32, 0, 1] {
        for (i, o) in (0u32..).zip([0x0bu32, 0x1b]) {
            let host = w(&t, o);
            host_list(&mut t, host, i == busy);
        }
        t.write(task, &[0x31; 0x20]);
        t.set_u32(task + 8, node);
        t.write(node, &[0x32; 0x20]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}

/// `r13_start::start_0244` and its load step: the two parts' slots (711
/// keeping the lock), the objects' starts (slot 941's list, slot 714, the
/// pool), the free list, the task list empty and not.
#[test]
fn start_0244() {
    let Some(mut t) = twin(244) else { return };
    let f = t.at(0x1340);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 533);
    magic_r11::object_slots(&mut t);
    for s in [0xd4u32, 0x1a3] {
        let start = t.img.rel32_target(f + s);
        magic_r11::object_start_auto(&mut t, start);
    }
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[2]);
    rec(&mut t, 710, 4, |_, a| 0x0710_0000 ^ a[0]);
    let lock = SCRATCH + 0x7f0;
    t.set_u32(w(&t, 0x42), lock);
    t.set_u32(lock, 0x00a1_0000);
    rec(&mut t, 711, 3, move |m, a| {
        m.w32(SCRATCH + 0x7f4, m.r32(lock));
        a[1]
    });
    for o in [0xa7u32, 0x19e] {
        t.write(w(&t, o), &[0; 0x40]);
    }
    let other = SCRATCH + 0x5f0;
    for listed in [false, true] {
        t.write(other, &[0x44; 0x10]);
        t.set_u32(w(&t, 0x1d9), if listed { other } else { 0 });
        t.set_u32(w(&t, 0x1eb), other);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r13_start::start_0244,
        );
    }
    let at = t.at(0x1210);
    load_cases(&mut t, at);
}

/// `magic_0371` 0x100014a0 (`r13_start::sprites_frame_0371`): live and
/// free records coloured from the table, then the sprites stepped and
/// drawn (their slots stubbed); an empty pool.
#[test]
fn sprites_frame_0371() {
    let Some(mut t) = twin(371) else { return };
    let f = t.at(0x14a0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let table = SCRATCH + 0x200;
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0f00_0000 + i);
    }
    rec(&mut t, 941, 1, move |_, a| table ^ (a[0] & 0xf0));
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 ^ a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    for (s, k) in [
        (325u32, 2usize),
        (229, 3),
        (248, 3),
        (773, 4),
        (774, 5),
        (775, 2),
        (776, 1),
        (777, 1),
        (778, 6),
        (779, 2),
        (942, 1),
    ] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let colours = w(&t, 0x2c);
    for i in 0..0x40u32 {
        t.set_u32(
            colours.wrapping_sub(4 * i),
            (0.25 + i as f32 * 0.5).to_bits(),
        );
    }
    let pool = w(&t, 0x01);
    let sprites = SCRATCH + 0x1000;
    for n in [5u32, 0] {
        t.write(sprites, &vec![0x2a; 0x94 * 5]);
        for i in 0..5u32 {
            t.set_u32(sprites + 0x94 * i, u32::from(i % 2 == 0));
            t.set_u32(sprites + 0x94 * i + 0x28, 0x20 + i);
        }
        t.set_u32(pool, n);
        t.set_u32(pool + 4, sprites);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// The load steps of the P063 records' start-ups (`magic_0393`
/// 0x100012e0, `magic_0450` 0x10005390; the start-up's set-up as in
/// `magic_r11_misc.rs`' `records_start_p063`).
#[test]
fn load_records_p063() {
    for (dll, rva, load) in [(393u16, 0x41c0u32, 0x12e0u32), (450, 0x7fe0, 0x5390)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        for s in 0..1024u32 {
            t.stub(s, move |_, _| (s << 20) | 5);
        }
        t.stub_st0(333, |_, _| -1.25);
        magic_r11::pools(&mut t, f, 353);
        let ps = t.img.rel32_target(f + 0x56);
        t.set_u32(t.img.word(ps + 0x04), SCRATCH + 0x7e0);
        t.set_u32(t.img.word(ps + 0x2d), SCRATCH + 0x7e0);
        let facing = t.img.rel32_target(f + 0x5e);
        let state = SCRATCH + 0x1_0000;
        t.write(state, &vec![0x3b; 0x2000]);
        t.set_u32(t.img.word(facing + 0x08), state);
        let recs = op(&t, 0x6d);
        t.write(recs, &vec![0x3e; 0x140 * 0x40]);
        t.write(SCRATCH + 0x600, &[0x44; 0x10]);
        t.set_u32(op(&t, 0xdb), SCRATCH + 0x600);
        t.set_u32(op(&t, 0xef), SCRATCH + 0x600);
        let at = t.at(load);
        load_cases(&mut t, at);
    }
}

/// `magic_0673` 0x10001eb0 (`r13_start::object_listed_0673`): the object
/// block junk, the task list empty and not (the pools free).
#[test]
fn object_listed_0673() {
    let Some(mut t) = twin(673) else { return };
    let f = t.at(0x1eb0);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let task = t.img.rel32_target(f + 0x54);
    let b = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x02), b);
    let other = SCRATCH + 0x600;
    for listed in [false, true] {
        magic_r11::pools(&mut t, task, 112);
        t.write(b + 0x5d00, &[0x6b; 0x200]);
        t.write(other, &[0x44; 0x10]);
        t.set_u32(t.img.word(task + 0x36), if listed { other } else { 0 });
        t.set_u32(t.img.word(task + 0x4a), other);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0673` 0x10001950 (`r13_start::rows_0673`): rows 3 to 9 started
/// from the row table (the object starts' slots and pool), slot 404's
/// float for the scales.
#[test]
fn rows_0673() {
    let Some(mut t) = twin(673) else { return };
    let f = t.at(0x1950);
    let row = t.img.rel32_target(f + 0x0b);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(row + 0x31);
    magic_r11::object_start_auto(&mut t, start);
    let fx = SCRATCH + 0x2_0000;
    t.set_u32(t.img.word(row + 0x18), fx);
    t.write(fx, &vec![0x2c; 0x130 * 10]);
    rec(&mut t, 404, 1, |_, a| 0x0404_0000 ^ a[0]);
    t.stub_st0(404, |_, a| f64::from(f32::from_bits(a[0])) * 3.0 + 0.125);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

/// `magic_0415` 0x10001390 (`r13_start::send_0415`): slot 520 in a
/// battle and not; slot 453's distance giving 0, a few and many steps.
#[test]
fn send_0415() {
    let Some(mut t) = twin(415) else { return };
    let f = t.at(0x1390);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let c = SCRATCH + 0x800;
    t.write(c, &[0; 0x200]);
    t.set_u32(c + 0x198, 0x00c0_0198);
    for i in 0..8u32 {
        t.set_u32(c + 0x1a0 + 4 * i, 0x40 + i);
    }
    t.set_u32(w(&t, 0x12), c);
    t.set_u32(w(&t, 0x240), c);
    for s in [456u32, 457] {
        rec(&mut t, s, 3, move |m, a| {
            for i in 0..4u32 {
                let x = (a[0] & 0xff) as f32 * 0.5 + i as f32 + if s == 457 { 3.25 } else { 0.0 };
                m.w32(a[2] + 4 * i, x.to_bits());
            }
            s
        });
    }
    let mode = SCRATCH + 0x7f0;
    rec(&mut t, 520, 0, move |m, _| m.r32(mode));
    let dist = SCRATCH + 0x7f4;
    rec(&mut t, 453, 1, |_, a| a[0]);
    t.stub_st0(453, move |m, _| f64::from(f32::from_bits(m.r32(dist))));
    for (s, k) in [(444u32, 1usize), (522, 2), (440, 4), (450, 2)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let o = SCRATCH + 0x1000;
    for (battle, d) in [(true, 12.5f32), (false, 3.0), (false, 0.25), (true, 1000.0)] {
        t.set_u32(mode, if battle { 0x0238_0000 } else { 0x0100_0000 });
        t.set_u32(dist, d.to_bits());
        t.write(o, &[0x3d; 0x140]);
        t.set_u32(o + 0xac, 1);
        t.set_u32(o + 0x3c, 3);
        t.check_entry(f, Call::default(), &[o], Eax::Checked);
    }
}

/// `magic_0421` 0x10001000 (`r13_start::targets_0421`): slot 520 in a
/// battle and not; no targets, and three (slot 518's values crossing
/// `G_M` up and down, slot 514's crossing `m`, once NaN).
#[test]
fn targets_0421() {
    let Some(mut t) = twin(421) else { return };
    let f = t.at(0x1000);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    let c = SCRATCH + 0x800;
    t.write(c, &[0; 0x200]);
    t.set_u32(c + 0x198, 0x00c0_0198);
    for i in 0..8u32 {
        t.set_u32(c + 0x1a0 + 4 * i, 0x40 + i);
    }
    t.set_u32(w(&t, 0x44), c);
    let mode = SCRATCH + 0x7f0;
    rec(&mut t, 520, 0, move |m, _| m.r32(mode));
    for (s, k) in [(519u32, 4usize), (474, 4), (513, 3), (450, 2), (449, 2)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    for (s, vals) in [
        (515u32, [2.5f64, 2.5, 2.5]),
        (518, [1.0, 4.0, 2.0]),
        (516, [0.5, -1.5, 3.0]),
        (517, [0.75, 1.25, -2.0]),
        (514, [3.0, f64::NAN, 7.0]),
    ] {
        // slot 515 takes no word, 514 two, the others one
        let n = match s {
            515 => 0,
            514 => 2,
            _ => 1,
        };
        rec(&mut t, s, n, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
        let row = magic_owna::row(s);
        t.stub_st0(s, move |m, _| vals[((m.r32(row) + 2) % 3) as usize]);
    }
    let o = SCRATCH + 0x1000;
    for (battle, n) in [(true, 0u32), (false, 3), (true, 3)] {
        t.set_u32(mode, if battle { 0x0238_0000 } else { 0x0100_0000 });
        t.set_u32(c + 0x19c, n);
        t.write(w(&t, 0x1e3), &[0x5e; 0x40]);
        t.check_entry(f, Call::default(), &[o], Eax::Checked);
    }
}

/// `r13_start::start_0352` and its load step: the camera's slots and
/// block (as `magic_0352.rs`' `camera_start`), slot 620 over the records,
/// the pools free, the task list empty and not.
#[test]
fn start_0352() {
    let Some(mut t) = twin(352) else { return };
    let f = t.at(0x5700);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::pools(&mut t, f, 345);
    let cam = t.img.rel32_target(f + 0x5c);
    rec(&mut t, 262, 4, |m, a| {
        for (i, at) in (0u32..).zip(&a[1..4]) {
            m.w32(*at, 0x0262_0000 + i);
        }
        0x262
    });
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, |_, _| 0.375);
    rec(&mut t, 12, 2, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, _| 7.5);
    t.set_u32(t.img.word(cam + 0x0e), SCRATCH + 0x2_0000);
    let recs = SCRATCH + 0x3_8000;
    t.set_u32(w(&t, 0x71), recs);
    for i in 0..0x40u32 {
        t.set_u32(recs + 0x20 * i + 0x1c, 0x00f0_0000 + i);
    }
    for (s, k) in [(719u32, 3usize), (620, 3), (271, 1), (711, 3), (710, 4)] {
        rec(&mut t, s, k, move |_, a| (s << 16) ^ a[0]);
    }
    let other = SCRATCH + 0x5f0;
    for listed in [false, true] {
        t.write(other, &[0x44; 0x10]);
        // the halfword and byte the start-up clears and sets start out not so
        t.write(w(&t, 0xb7), &[0x44; 2]);
        t.write(w(&t, 0xbf), &[0x44; 2]);
        t.set_u32(w(&t, 0xdd), if listed { other } else { 0 });
        t.set_u32(w(&t, 0xf4), other);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r13_start::start_0352,
        );
    }
    let at = t.at(0x4b10);
    load_cases(&mut t, at);
}

/// `magic_0612` 0x10008320 (`m0563::scale_start`, family P092: the
/// 20-step scale task): slot 371 twice, the scale task with a free pool.
#[test]
fn scale_start_0612() {
    let Some(mut t) = twin(612) else { return };
    let at = t.at(0x8320);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x2a), fx);
    rec(&mut t, 371, 4, |_, a| 0x0371_0000 ^ a[0]);
    let st = t.img.rel32_target(at + 0x48);
    magic_r11::pools(&mut t, st, 191);
    t.write(fx + 0x3864, &[0x3d; 0x40]);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// `magic_0613` 0x10004980 (`m0164::mean_visit`, family P081: the parts'
/// centre `m0613::centre` instead of the mean point): visited, not visited
/// with no object, with an object of two parts and of none.
#[test]
fn mean_visit_0613() {
    let Some(mut t) = twin(613) else { return };
    let at = t.at(0x4980);
    let slot = 1000u32;
    t.set_u32(magic::TABLE + 4 * slot, magic::TRAPS + 16 * slot);
    rec(&mut t, slot, 1, |m, a| {
        let v = m.r32(a[0] + 4);
        m.w32(SCRATCH + 0x37f0, v);
        m.r32(a[0]) ^ v ^ 0x1000
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4u32 {
            m.w32(a[0] + 4 * i, (0.5 + i as f32).to_bits());
        }
        0x325
    });
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4u32 {
            m.w32(
                a[2] + 4 * i,
                ((a[0] & 0xf) as f32 + i as f32 * 2.0).to_bits(),
            );
        }
        0x268
    });
    let vt = SCRATCH + 0x900;
    t.set_u32(vt + 0x1c, magic::TRAPS + 16 * slot);
    let (cur, node, kid, obj, out) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x1200,
        SCRATCH + 0x2000,
        SCRATCH + 0x1600,
    );
    for (same, kids, has, parts) in [
        (true, false, true, 2u32),
        (false, false, false, 2),
        (false, true, true, 2),
        (false, true, true, 0),
    ] {
        t.write(node, &[0x22; 0x40]);
        t.set_u32(node, vt);
        t.set_u8(node + 0x1a, 5);
        t.set_u32(node + 0x10, if kids { kid } else { node });
        t.write(kid, &[0x44; 0x20]);
        t.set_u32(kid, vt);
        t.set_u32(kid + 8, node);
        t.set_u32(node + 0x2c, if has { obj } else { 0 });
        t.set_u32(node + 0x1c, out);
        t.write(obj, &vec![0x33; 0x1700]);
        t.set_u32(obj, vt);
        t.set_u32(obj + 0x162c, parts);
        for i in 0..2u32 {
            t.set_u32(obj + 0x110 + 0x274 * i, 0x40 + i);
        }
        t.set_u32(cur, node);
        t.set_u8(cur + 4, 0x7f);
        t.set_u8(cur + 5, if same { 5 } else { 6 });
        let c = Call {
            eax: 0x1234_5678,
            ecx: cur,
            ..Call::default()
        };
        t.check_entry(at, c, &[], Eax::Checked);
    }
}

/// The rigs' step's and the actors' slots (the running paths' tail).
fn rig_slots(t: &mut magic::Twin) {
    for (s, k) in [
        (941u32, 1usize),
        (772, 2),
        (779, 4),
        (326, 4),
        (229, 3),
        (284, 3),
        (285, 2),
        (286, 2),
        (235, 1),
        (330, 3),
        (328, 3),
        (329, 3),
        (413, 1),
        (215, 1),
        (19, 1),
        (21, 2),
    ] {
        rec(t, s, k, move |_, a| {
            (s << 16) ^ a.first().copied().unwrap_or(0)
        });
    }
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    rec(t, 314, 1, |_, a| a[0]);
    t.stub_st0(314, |_, a| f64::from(a[0] & 0xff) * 0.0625 - 7.0);
}

/// Family P080 (`r13_misc::scripts_step_motion`) in both copies: both
/// scripts done, the second waiting, the first waiting (the record's
/// words, the motion step, the rigs' step on the DLL's own data).
#[test]
fn scripts_steps_motion() {
    for (dll, rva) in [(165u16, 0x16b0u32), (606, 0x6930)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
        rig_slots(&mut t);
        let r = SCRATCH + 0x1_0000;
        t.set_u32(w(&t, 0x26), 0x00c0_0026);
        let task = SCRATCH + 0x5000;
        for (k, (s1, s2)) in [(0u8, 0u8), (0, 2), (2, 0), (2, 2)].into_iter().enumerate() {
            let k = k as u32;
            t.write(r, &vec![0x37; 0x6000]);
            for (i, x) in (0u32..).zip([1.5f32, -2.0, 0.25, 1.0]) {
                t.set_u32(r + t.img.word(f + 0x4e) + 4 * i, (x + k as f32).to_bits());
            }
            t.write(w(&t, 0x10), &[0x5c; 0x30]);
            for (ctx, st) in [(w(&t, 0x2e), s1), (w(&t, 0x3d), s2)] {
                t.write(ctx, &[0; 0x58]);
                t.set_u32(ctx, SCRATCH + 0x700);
                t.set_u8(ctx + 0x56, st);
                t.set_u16(ctx + 0x54, 0xffff);
            }
            t.set_u32(w(&t, 0x07), r);
            t.write(task, &[0x21; 0x20]);
            t.set_u32(task, 0);
            t.set_u32(task + 4, 0);
            t.set_u32(task + 8, SCRATCH + 0x5100);
            t.set_u32(SCRATCH + 0x510c, 0);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// Family P082 (`r13_misc::script_step`) in both copies: the DLL's script
/// running (the motion script, the rigs' step), done with the record's
/// word not 1, and 1 with and without a held node.
#[test]
fn script_steps() {
    for (dll, rva) in [(307u16, 0x1510u32), (605, 0x6e60)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
        rig_slots(&mut t);
        let run = t.img.rel32_target(f + 0x3c);
        let mode = t.img.word(run + 0x01);
        let s = SCRATCH + 0x1_0000;
        t.set_u32(w(&t, 0x06), s);
        let (task, held) = (SCRATCH + 0x5000, SCRATCH + 0x5200);
        for (st, word, hold) in [
            (2u8, 1u32, false),
            (0, 0, false),
            (0, 1, true),
            (0, 1, false),
        ] {
            t.write(s, &vec![0x37; 0x4000]);
            t.set_u32(s + t.img.word(f + 0x57), word);
            t.write(w(&t, 0x0f), &[0x5c; 0x30]);
            t.set_u8(mode, st);
            t.set_u16(mode - 2, 0xffff);
            for n in [task, held] {
                t.write(n, &[0x21; 0x20]);
                t.set_u32(n, 0);
                t.set_u32(n + 4, 0);
                t.set_u32(n + 8, SCRATCH + 0x5100);
            }
            t.set_u32(SCRATCH + 0x510c, 0);
            t.set_u32(w(&t, 0x60), if hold { held } else { 0 });
            t.set_u32(w(&t, 0x2f), 0x2f2f_0000 | u32::from(st));
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}

/// `r14::task_0288` (`magic_0288` 0x10001540): the two timers before,
/// at and past their end (the objects started), the light's counter on
/// each side of its range, the motion scripts done and waiting, the
/// camera's script stopping or not, the scripts' done words set and not.
#[test]
fn task_0288() {
    let Some(mut t) = twin(288) else { return };
    let f = t.at(0x1540);
    let w = |t: &magic::Twin, o: u32| t.img.word(f + o);
    // every slot answers a value of its own (the objects' run reaches
    // many); the task's own slots record their words
    for s in 0..1024u32 {
        t.stub(s, move |_, _| s << 20 | 5);
    }
    for (s, n) in [(250u32, 2usize), (278, 3), (12, 1)] {
        rec(&mut t, s, n, move |_, a| (s << 20) ^ a[0]);
    }
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.25);
    // the points: slot 262 fills its three pointers
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4usize {
            m.w32(a[i], (0.5 * i as f32 + (a[0] & 7) as f32).to_bits());
        }
        0x262
    });
    for (s, n) in [(941u32, 1usize), (714, 5)] {
        let table = SCRATCH + 0x3400;
        rec(&mut t, s, n, move |_, a| {
            if s == 941 { table } else { 0x0714_0000 ^ a[2] }
        });
    }
    for i in 0..0x10 {
        t.set_u32(SCRATCH + 0x3400 + 4 * i, 0x1111 * (i + 1));
    }
    for start in [t.img.rel32_target(f + 0x3b), t.img.rel32_target(f + 0xb8)] {
        magic_r11::object_start_setup(&mut t, start, (0x84, 0x96));
    }
    // the actor byte and the tables of character handles
    let sel = SCRATCH + 0x50;
    t.set_u8(sel, 2);
    // (the byte after: a wider read shows)
    t.set_u8(sel + 1, 0x70);
    t.set_u32(w(&t, 0x1ac), 0x0ac7_0001);
    // the objects' list empty: the run says 1 (no object ran)
    let run = t.img.rel32_target(f + 0x17c);
    let list = t.img.word(run + 0x7c);
    t.write(list, &[0; 0x10]);
    for o in [0x40u32, 0xbd, 0x22b, 0x249, 0x269] {
        t.set_u32(w(&t, o), sel);
    }
    for o in [0x59u32, 0xd1, 0x237, 0x257, 0x277] {
        for i in 0..4 {
            t.set_u32(w(&t, o) + 4 * i, 0x0c0c_0000 + 0x100 * o + i);
        }
    }
    // the camera script's commands: stop (op 5)
    for i in 0..4u32 {
        t.set_u32(SCRATCH + 0x4100 + 8 * i, 0xffff_ffff);
    }
    let cmds = SCRATCH + 0x4000;
    t.write(cmds, &[0; 0x30]);
    t.set_u32(cmds, 5);
    let q = w(&t, 0x1db);
    let task = SCRATCH + 0x5000;
    let record = SCRATCH + 0x5100;
    // (timer A, timer B, counter, scripts done, camera wait, done words)
    for (k, (a, b, c, done, wait, words)) in [
        (0i32, 0i32, 3i32, 1u32, 5u32, 1u32),
        (1, 1, 0, 1, 1, 1),
        (2, 7, -15, 0, 1, 0),
        (1, 0, -16, 1, 1, 1),
        (0, 1, -1, 1, 1, 2),
        (-3, 2, 1, 1, 0, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let k = k as u32;
        t.set_u32(w(&t, 0x11), a.cast_unsigned());
        t.set_u32(w(&t, 0x98), b.cast_unsigned());
        t.set_u32(w(&t, 0x11f), c.cast_unsigned());
        t.set_u32(w(&t, 0x14b), (0.25 + k as f32).to_bits());
        let m0 = w(&t, 0x183);
        for i in 0..2u32 {
            let m = m0 + 0x20 * i;
            t.write(m, &[0; 0x20]);
            t.set_u32(m + 0x14, done);
            // a running script without a wait asks for its motion (slot
            // 411), then stops at its next command (-1)
            t.set_u16(m + 0x18, if done == 0 && i == 0 { 0 } else { 3 });
            t.set_u32(m, SCRATCH + 0x4100);
        }
        t.write(q, &[0; 0x34]);
        t.set_u32(q + 0x24, cmds);
        t.set_u32(q + 0x2c, wait);
        t.set_u32(w(&t, 0x213), words);
        // no objects from earlier cases: the run's list head and cursor 0
        t.set_u32(t.img.word(run + 0x14), 0);
        t.write(list, &[0; 0x10]);
        // the globals the step stores: other values first
        for o in [
            0x87u32, 0x93, 0x10e, 0x11a, 0x15c, 0x1c1, 0x1ce, 0x1ea, 0x1f0,
        ] {
            t.set_u32(w(&t, o), 0x5a5a_0000 + o + k);
        }
        t.set_u32(w(&t, 0x21f), 1);
        t.write(task, &[0x5a; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x6b; 0x20]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
}
