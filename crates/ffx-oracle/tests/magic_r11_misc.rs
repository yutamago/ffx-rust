//! Round 11's small own functions (`ffx_magic::r11_misc`) against each
//! original. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;
mod magic_r11;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

/// `magic_0383` 0x10001370: the main object starts and 16 bytes are
/// copied (junk in the target first).
#[test]
fn main_object_0383() {
    let Some(mut t) = twin(383) else { return };
    let f = t.at(0x1370);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x11);
    magic_r11::object_start_setup(&mut t, start, (0x7d, 0x8f));
    let to = t.img.word(f + 0x22);
    t.write(to, &[0x5e; 16]);
    t.check_entry(f, Call::default(), &[], Eax::Checked);
}

/// `magic_0369` 0x10001370: the yaw on each side of the wrap, between
/// and NaN.
#[test]
fn main_object_0369() {
    let Some(mut t) = twin(369) else { return };
    let f = t.at(0x1370);
    let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x16);
    magic_r11::object_start_setup(&mut t, start, (0x7d, 0x8f));
    t.set_u32(op(&t, 0x21), 0x00a5_0000);
    t.stub_st0(12, |_, a| f64::from(a[0] >> 16) * 0.5);
    let yaw = SCRATCH + 0x90;
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw))));
    let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(op(t, o)));
    let k3 = kf(&t, 0x8a);
    for v in [
        0.0f32,
        8.0,
        -8.0,
        f32::NAN,
        k3 - kf(&t, 0x9a),
        k3 - kf(&t, 0xb8),
    ] {
        t.set_u32(yaw, v.to_bits());
        t.write(op(&t, 0xac), &[0x5f; 4]);
        t.check_entry(f, Call::default(), &[], Eax::Checked);
    }
}

/// `magic_0065` 0x10002fd0: the flag bit set and clear, the task list
/// empty and not.
#[test]
fn colour_start_0065() {
    let Some(mut t) = twin(65) else { return };
    let f = t.at(0x2fd0);
    for (bit, listed) in [(false, false), (true, false), (true, true), (false, true)] {
        magic_r11::colour_start_setup(&mut t, f, bit, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r11_misc::colour_start_0065,
        );
    }
}

/// `magic_0054` 0x10001370: the task list empty and not.
#[test]
fn two_anim_objects_0054() {
    let Some(mut t) = twin(54) else { return };
    let f = t.at(0x1370);
    for listed in [false, true] {
        magic_r11::two_anim_setup(&mut t, f, listed);
        t.check(
            f,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::r11_misc::two_anim_objects_0054,
        );
    }
}

/// `magic_0191` 0x10001440 (`r11_misc::aim_at_target_0191`): the target
/// actor through the object's index, slot 453's length on both sides of
/// a whole step count (and NaN: the count 0x80000000).
#[test]
fn aim_at_target_0191() {
    let Some(mut t) = twin(191) else { return };
    let f = t.at(0x1440);
    let gp = SCRATCH + 0x100;
    t.set_u32(t.img.word(f + 0x12), gp);
    t.set_u32(gp + 0x198, 0x00a1_0000);
    for i in 0..8 {
        t.set_u32(gp + 0x1a0 + 4 * i, 0x00b0_0000 + i);
    }
    let fill = |m: &mut dyn magic::StubMem, a: &[u32]| {
        for i in 0..4 {
            m.w32(
                a[2] + 4 * i,
                0x3f00_0000 + (a[0] & 0xf) * 0x10_0000 + i * 0x1000,
            );
        }
        a[1]
    };
    rec(&mut t, 456, 3, fill);
    rec(&mut t, 457, 3, fill);
    let len = SCRATCH + 0x90;
    rec(&mut t, 453, 1, |m, a| {
        magic_owna::dump(m, 453, a[0], 4);
        0x453
    });
    t.stub_st0(453, move |m, _| f64::from(f32::from_bits(m.r32(len))));
    rec(&mut t, 444, 1, |_, a| a[0] ^ 0x444);
    rec(&mut t, 440, 4, |_, a| a[1] ^ 0x440);
    rec(&mut t, 450, 2, |_, a| a[0] ^ 0x450);
    let obj = SCRATCH + 0x1000;
    for (k, l) in [(0u32, 3.0f32), (1, 0.5), (2, 100.25), (3, f32::NAN)] {
        t.write(obj, &[0x5d; 0x200]);
        t.set_u32(obj + 0xac, k);
        t.set_u32(obj + 0x2c + 16 * k, 2 + k);
        t.set_u32(len, l.to_bits());
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `magic_0194` 0x10001370 (`r11_misc::between_actors_0194`): two pairs
/// of actor points, slot 464 changing the direction it gets.
#[test]
fn between_actors_0194() {
    let Some(mut t) = twin(194) else { return };
    let f = t.at(0x1370);
    let gp = SCRATCH + 0x100;
    t.set_u32(t.img.word(f + 0x23), gp);
    t.set_u32(t.img.word(f + 0x44), gp);
    t.set_u32(gp + 0x198, 0x00a1_0000);
    t.set_u32(gp + 0x1a0, 0x00b1_0000);
    let run = SCRATCH + 0x80;
    let fill = move |m: &mut dyn magic::StubMem, a: &[u32]| {
        let k = m.r32(run);
        for i in 0..4 {
            m.w32(
                a[2] + 4 * i,
                0x3f00_0000 + (a[0] >> 16) * 0x1000 + (k + i) * 0x8_0000,
            );
        }
        a[1]
    };
    rec(&mut t, 456, 3, fill);
    rec(&mut t, 457, 3, fill);
    rec(&mut t, 464, 1, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[0] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x0001_0000);
        }
        0x464
    });
    rec(&mut t, 450, 2, |_, a| a[0] ^ 0x450);
    rec(&mut t, 451, 2, |_, a| a[1] ^ 0x451);
    let obj = SCRATCH + 0x1000;
    for k in 0..3u32 {
        t.set_u32(run, k);
        t.write(obj, &[0x5d; 0x200]);
        t.set_u32(obj + 0xbc, 0x0101_0101 * k);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// `magic_0429` 0x10001030 (`r11_misc::part_at_effect_0429`): the
/// height difference below and above the limit.
#[test]
fn part_at_effect_0429() {
    let Some(mut t) = twin(429) else { return };
    let f = t.at(0x1030);
    let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    let sp = t.img.rel32_target(f + 0x44);
    t.set_u32(op(&t, 0x11), SCRATCH + 0x7f8);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f0);
    t.set_u32(op(&t, 0x20), SCRATCH + 0x280);
    t.set_u32(SCRATCH + 0x290, 0x0290_0290);
    t.set_u32(op(&t, 0x5b), 0x5b5b);
    t.set_u32(op(&t, 0x6c), 0x6c6c);
    t.set_u32(op(&t, 0xda), 0xdada);
    rec(&mut t, 210, 2, |m, a| {
        m.w32(a[1], 0x210);
        0x210
    });
    let py = SCRATCH + 0x90;
    rec(&mut t, 325, 2, move |m, a| {
        for i in 0..4 {
            m.w32(a[0] + 4 * i, m.r32(py) + i);
            m.w32(a[1] + 4 * i, 0x4000_0000 + i);
        }
        0x325
    });
    rec(&mut t, 393, 3, |_, a| a[2] ^ 0x393);
    let obj = SCRATCH + 0x1000;
    for (y, p) in [(1.0f32, 0.5f32), (1000.0, -1000.0), (-3.0, 2.0)] {
        t.write(obj, &[0x5d; 0x100]);
        t.set_u32(obj + 0x44, y.to_bits());
        t.set_u32(py, p.to_bits() - 1);
        t.check_entry(f, Call::default(), &[obj], Eax::Checked);
    }
}

/// Parameter family P070 (`r11_misc::fader_start_p070`, `magic_0162`
/// 0x10002f90 and `magic_0586` 0x10002ba0): the task list empty and not.
#[test]
fn fader_start_p070() {
    for (dll, rva) in [(162u16, 0x2f90u32), (586, 0x2ba0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        for listed in [false, true] {
            magic_r11::pools(&mut t, f, 309);
            magic_r11::object_slots(&mut t);
            let start = t.img.rel32_target(f + 0x1e);
            magic_r11::object_start_auto(&mut t, start);
            let b = SCRATCH + 0x1_0000;
            t.write(b, &vec![0x3b; 0x9000]);
            t.set_u32(op(&t, 0x0c), b);
            t.write(SCRATCH + 0x600, &[0x44; 0x10]);
            t.set_u32(op(&t, 0xba), if listed { SCRATCH + 0x600 } else { 0 });
            t.set_u32(op(&t, 0xd1), SCRATCH + 0x600);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}
