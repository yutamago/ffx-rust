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

/// `r14::main_object_0367` (`magic_0367` 0x100014d0): the yaw on each side
/// of the wrap, between and NaN; the two words it sets after, old before.
#[test]
fn main_object_0367() {
    let Some(mut t) = twin(367) else { return };
    let f = t.at(0x14d0);
    let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
    magic_r11::object_slots(&mut t);
    let start = t.img.rel32_target(f + 0x18);
    magic_r11::object_start_setup(&mut t, start, (0x84, 0x96));
    t.set_u32(op(&t, 0x23), 0x00a5_0000);
    t.stub_st0(12, |_, a| f64::from(a[0] >> 16) * 0.5);
    let yaw = SCRATCH + 0x90;
    rec(&mut t, 333, 1, |_, a| a[0] ^ 0x333);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw))));
    let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(op(t, o)));
    let k3 = kf(&t, 0x8c);
    for v in [
        0.0f32,
        8.0,
        -8.0,
        f32::NAN,
        k3 - kf(&t, 0x9c),
        k3 - kf(&t, 0xb0),
    ] {
        t.set_u32(yaw, v.to_bits());
        for o in [0xc5u32, 0xcb, 0xd5] {
            t.write(op(&t, o), &[0x5f; 4]);
        }
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

/// Parameter family P061 (`r11_misc::anim_start_p061`, `magic_0168`
/// 0x10003500 and `magic_0587` 0x10003030): the flag set (slot 38), clear
/// with the count below 0x20 (slot 541) and at 0x20 (slot 38); the angle
/// on each side of its wrap and NaN.
#[test]
fn anim_start_p061() {
    for (dll, rva) in [(168u16, 0x3500u32), (587, 0x3030)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(op(t, o)));
        let (k4, hi, lo) = (kf(&t, 0x117), kf(&t, 0x121), kf(&t, 0x135));
        for (flag, count, angle) in [
            (1u32, 0u32, 0.0f32),
            (0, 0x1f, hi - k4 + 1.0),
            (0, 0x20, lo - k4 - 1.0),
            (0, 3, f32::NAN),
        ] {
            magic_r11::pools(&mut t, f, 367);
            magic_r11::object_slots(&mut t);
            let start = t.img.rel32_target(f + 0x20);
            magic_r11::object_start_auto(&mut t, start);
            rec(&mut t, 541, 3, |_, a| a[0] ^ 0x541);
            rec(&mut t, 38, 3, |_, a| a[0] ^ 0x38);
            let b = SCRATCH + 0x1_0000;
            t.write(b, &vec![0x3b; 0x3200]);
            t.set_u32(op(&t, 0x0c), b);
            t.set_u32(b + op(&t, 0x10f), angle.to_bits());
            t.set_u32(op(&t, 0xc5), flag);
            t.set_u32(op(&t, 0xcd), count);
            t.set_u32(op(&t, 0x105), 0x0105_0105);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Parameter family P055 (`r11_misc::launches_p055`, `magic_0421`
/// 0x10001b60 and `magic_0535` 0x10001b40): the count a multiple of 16
/// (the copied second object) and not, and past 0x26.
#[test]
fn launches_p055() {
    for (dll, rva) in [(421u16, 0x1b60u32), (535, 0x1b40)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        let launch = t.img.rel32_target(f + 0x7c);
        for o in [0x97, 0x193, 0x20d] {
            t.set_u32(t.img.word(launch + o), 5);
        }
        t.set_u32(t.img.word(launch + 0x67), 1.5f32.to_bits());
        t.set_u32(t.img.word(launch + 0xa2), 1.25f32.to_bits());
        t.set_u32(t.img.word(launch + 0xc7), 0.75f32.to_bits());
        let g_d = t.u32(t.img.word(launch + 0xd4));
        if g_d != 0 {
            t.set_u32(g_d + 0x198, 0x0198_0000);
        }
        t.stub(474, |m, a| {
            for (i, step) in (0u32..).zip([0.5f32, 1.0, 1.5, 2.0]) {
                let v = m.r32(a[2] + 4 * i);
                m.w32(a[3] + 4 * i, (f32::from_bits(v) + step).to_bits());
            }
            0x474
        });
        rec(&mut t, 458, 3, |_, a| a[1] ^ 0x458);
        t.stub_st0(482, |_, a| f64::from(a[3]) * 0.5 + 0.25);
        rec(&mut t, 482, 4, |_, a| a[0] ^ 0x482);
        // slot 434 hands out three objects in turn
        let k = SCRATCH + 0x98;
        rec(&mut t, 434, 2, move |m, _| {
            let n = m.r32(k);
            m.w32(k, n + 1);
            SCRATCH + 0x1000 + 0x400 * (n % 3)
        });
        let arg = SCRATCH + 0x900;
        for n in [16u32, 5, 0x26, 0x20] {
            t.set_u32(k, 0);
            for i in 0..3 {
                let p = SCRATCH + 0x1000 + 0x400 * i;
                t.write(p, &[0x11; 0x200]);
                t.set_u32(p + 0xac, 1);
            }
            t.set_u32(op(&t, 0x0d), n);
            t.set_u32(op(&t, 0x45), 2.0f32.to_bits());
            t.set_u32(op(&t, 0x37), 3.0f32.to_bits());
            t.set_u32(op(&t, 0x60), 0x0060_0060);
            t.set_u32(arg, 0x10);
            t.check_entry(f, Call::default(), &[arg], Eax::Checked);
        }
    }
}

/// `r14::launches_0384` (`magic_0384` 0x10001b60): the count below and
/// at the first callback's bound 0x19, a multiple of 16 and not, and on
/// both sides of 0x32.
#[test]
fn launches_0384() {
    for (dll, rva) in [(384u16, 0x1b60u32)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        let launch = t.img.rel32_target(f + 0x90);
        for o in [0x97, 0x193, 0x20d] {
            t.set_u32(t.img.word(launch + o), 5);
        }
        t.set_u32(t.img.word(launch + 0x67), 1.5f32.to_bits());
        t.set_u32(t.img.word(launch + 0xa2), 1.25f32.to_bits());
        t.set_u32(t.img.word(launch + 0xc7), 0.75f32.to_bits());
        let g_d = t.u32(t.img.word(launch + 0xd4));
        if g_d != 0 {
            t.set_u32(g_d + 0x198, 0x0198_0000);
        }
        t.stub(474, |m, a| {
            for (i, step) in (0u32..).zip([0.5f32, 1.0, 1.5, 2.0]) {
                let v = m.r32(a[2] + 4 * i);
                m.w32(a[3] + 4 * i, (f32::from_bits(v) + step).to_bits());
            }
            0x474
        });
        rec(&mut t, 458, 3, |_, a| a[1] ^ 0x458);
        t.stub_st0(482, |_, a| f64::from(a[3]) * 0.5 + 0.25);
        rec(&mut t, 482, 4, |_, a| a[0] ^ 0x482);
        // slot 434 hands out three objects in turn
        let k = SCRATCH + 0x98;
        rec(&mut t, 434, 2, move |m, _| {
            let n = m.r32(k);
            m.w32(k, n + 1);
            SCRATCH + 0x1000 + 0x400 * (n % 3)
        });
        let arg = SCRATCH + 0x900;
        for n in [16u32, 5, 0x18, 0x19, 0x31, 0x32, 0x20] {
            t.set_u32(k, 0);
            for i in 0..3 {
                let p = SCRATCH + 0x1000 + 0x400 * i;
                t.write(p, &[0x11; 0x200]);
                t.set_u32(p + 0xac, 1);
            }
            t.set_u32(op(&t, 0x0d), n);
            t.set_u32(op(&t, 0x45), 2.0f32.to_bits());
            t.set_u32(op(&t, 0x37), 3.0f32.to_bits());
            t.set_u32(op(&t, 0x79), 0x0079_0079);
            t.set_u32(arg, 0x10);
            t.check_entry(f, Call::default(), &[arg], Eax::Checked);
        }
    }
}

/// Parameter family P034 (`r11_misc::sparks_p034`, `magic_0393`
/// 0x10003690 and `magic_0450` 0x10007670): the counter reaching 0 (an
/// object record free and not) and not; three sprite records free, and
/// only one (the loop ends early).
#[test]
fn sparks_p034() {
    for (dll, rva) in [(393u16, 0x3690u32), (450, 0x7670)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        let t140 = t.img.rel32_target(f + 0x1b);
        let t74 = t.img.rel32_target(f + 0xe2);
        let calls = SCRATCH + 0x88;
        t.stub_st0(314, move |m, a| {
            let k = m.r32(calls);
            m.w32(calls, k + 1);
            f64::from(f32::from_bits(a[0])) * 0.5 + f64::from(k) * 0.125
        });
        rec(&mut t, 314, 1, |_, a| a[0] ^ 0x314);
        magic_r11::object_slots(&mut t);
        let start = t.img.rel32_target(f + 0x64);
        magic_r11::object_start_auto(&mut t, start);
        for (counter, objs, sprites) in [(1u32, 1u32, 3u32), (5, 1, 3), (1, 0, 1)] {
            t.set_u32(calls, 0);
            let b = SCRATCH + 0x1_0000;
            t.write(b, &vec![0x3b; 0x2000]);
            t.set_u32(op(&t, 0x0a), b);
            t.set_u32(b + op(&t, 0x10), counter);
            let p = SCRATCH + 0x700;
            t.set_u32(op(&t, 0xf2), p);
            t.set_u32(p + 0x1c, 0x001c_001c);
            // the object records (0x140 bytes; `+0x130` 0 = free)
            let objs_at = SCRATCH + 0x1_4000;
            t.write(objs_at, &vec![0x3c; 0x280]);
            t.set_u32(t.img.word(t140 + 0x01), 0);
            t.set_u32(t.img.word(t140 + 0x09), 2);
            t.set_u32(t.img.word(t140 + 0x16), objs_at);
            for i in 0..2 {
                t.set_u32(objs_at + 0x140 * i + 0x130, u32::from(i >= objs));
                t.set_u16(objs_at + 0x140 * i + 0x124, 0x8000);
            }
            // the sprite records (0x74 bytes; `+0x6c` 0 = free)
            let spr = SCRATCH + 0x1_6000;
            t.write(spr, &vec![0x3d; 0x74 * 4]);
            t.set_u32(t.img.word(t74 + 0x08), 0);
            t.set_u32(t.img.word(t74 + 0x0f), 4);
            t.set_u32(t.img.word(t74 + 0x37), spr);
            t.set_u32(t.img.word(t74 + 0x2b), 0);
            for i in 0..4 {
                t.set_u32(spr + 0x74 * i + 0x6c, u32::from(i >= sprites));
            }
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Parameter family P048 (`r11_misc::launches_p048`, `magic_0421`
/// 0x10001960 and `magic_0535` 0x10001940; `magic_0384` 0x10001960, its
/// bound above 0x19): the count 0 (the extra object), a multiple of 16,
/// neither, and on each side of both bounds (slot 450).
#[test]
fn launches_p048() {
    for (dll, rva) in [(421u16, 0x1960u32), (535, 0x1940), (384, 0x1960)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        let launch = t.img.rel32_target(f + 0xa3);
        for o in [0x97, 0x193, 0x20d] {
            t.set_u32(t.img.word(launch + o), 5);
        }
        t.set_u32(t.img.word(launch + 0x67), 1.5f32.to_bits());
        t.set_u32(t.img.word(launch + 0xa2), 1.25f32.to_bits());
        t.set_u32(t.img.word(launch + 0xc7), 0.75f32.to_bits());
        let g_d = t.u32(t.img.word(launch + 0xd4));
        if g_d != 0 {
            t.set_u32(g_d + 0x198, 0x0198_0000);
        }
        t.stub(474, |m, a| {
            for (i, step) in (0u32..).zip([0.5f32, 1.0, 1.5, 2.0]) {
                let v = m.r32(a[2] + 4 * i);
                m.w32(a[3] + 4 * i, (f32::from_bits(v) + step).to_bits());
            }
            0x474
        });
        rec(&mut t, 458, 3, |_, a| a[1] ^ 0x458);
        rec(&mut t, 450, 2, |_, a| a[0] ^ 0x450);
        t.stub_st0(482, |_, a| f64::from(a[3]) * 0.5 + 0.25);
        rec(&mut t, 482, 4, |_, a| a[0] ^ 0x482);
        let k = SCRATCH + 0x98;
        rec(&mut t, 434, 2, move |m, a| {
            let n = m.r32(k);
            m.w32(k, n + 1);
            m.w32(SCRATCH + 0x9c, a[1]);
            SCRATCH + 0x1000 + 0x400 * (n % 3)
        });
        let arg = SCRATCH + 0x900;
        for n in [0u32, 16, 5, 0x25, 0x24, 0x18, 0x19] {
            t.set_u32(k, 0);
            for i in 0..3 {
                let p = SCRATCH + 0x1000 + 0x400 * i;
                t.write(p, &[0x11; 0x200]);
                t.set_u32(p + 0xac, 1);
            }
            t.set_u32(op(&t, 0x0d), n);
            t.set_u32(op(&t, 0x45), 2.0f32.to_bits());
            t.set_u32(op(&t, 0x37), 3.0f32.to_bits());
            t.set_u32(op(&t, 0x6c), 0x0060_0060);
            t.set_u32(arg, 0x10);
            t.check_entry(f, Call::default(), &[arg], Eax::Checked);
        }
    }
}

/// Parameter family P063 (`r11_misc::records_start_p063`, `magic_0393`
/// 0x100041c0 and `magic_0450` 0x10007fe0): every slot answers its first
/// word mixed with its number; the facing's state block junk; the pools
/// with free nodes; the task list empty and not.
#[test]
fn records_start_p063() {
    for (dll, rva) in [(393u16, 0x41c0u32), (450, 0x7fe0)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        // every slot answers its number (the harness compares the words
        // each call gets; some slots take none)
        for s in 0..1024u32 {
            t.stub(s, move |_, _| (s << 20) | 5);
        }
        // the facing's angle
        t.stub_st0(333, |_, _| -1.25);
        for listed in [false, true] {
            magic_r11::pools(&mut t, f, 353);
            let ps = t.img.rel32_target(f + 0x56);
            t.set_u32(t.img.word(ps + 0x04), SCRATCH + 0x7e0);
            t.set_u32(t.img.word(ps + 0x2d), SCRATCH + 0x7e0);
            let facing = t.img.rel32_target(f + 0x5e);
            let state = SCRATCH + 0x1_0000;
            t.write(state, &vec![0x3b; 0x2000]);
            t.set_u32(t.img.word(facing + 0x08), state);
            // junk where the start-up writes zeros: the records, two words
            let recs = op(&t, 0x6d);
            t.write(recs, &vec![0x3e; 0x140 * 0x40]);
            for o in [0xa0, 0xbc] {
                t.set_u32(op(&t, o), 0x5a5a_5a5b);
            }
            let pool = op(&t, 0x106);
            t.write(pool, &[0x3c; 0x40]);
            t.write(SCRATCH + 0x600, &[0x44; 0x10]);
            t.set_u32(op(&t, 0xdb), if listed { SCRATCH + 0x600 } else { 0 });
            t.set_u32(op(&t, 0xef), SCRATCH + 0x600);
            t.check_entry(f, Call::default(), &[], Eax::Checked);
        }
    }
}

/// Parameter family P030 (`r11_misc::scene_ctor_p030`, `magic_0150`
/// 0x1000b720 and `magic_0603` 0x1000b650; `ecx` the effect, `edx` a
/// word): every slot answers its number (slot 324 counting up, negative
/// once); the root's child list empty and not; the flag word 1 and not.
#[test]
fn scene_ctor_p030() {
    for (dll, rva) in [(150u16, 0xb720u32), (603, 0xb650)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
        for s in 0..1024u32 {
            t.stub(s, move |_, _| (s << 20) | 5);
        }
        let k = SCRATCH + 0x98;
        t.stub(324, move |m, _| {
            let n = m.r32(k);
            m.w32(k, n + 1);
            if n == 1 { 0xffff_ff9b } else { 0x1234 + 97 * n }
        });
        // slot 390: a model id with slot 362 (the block's `+0x1c` used) or not
        let id = SCRATCH + 0x9c;
        t.stub(390, move |m, _| m.r32(id));
        for (listed, flag, model) in [(false, 1u32, 0x1071u32), (true, 0, 0x2000)] {
            t.set_u32(id, model);
            t.set_u32(k, 0);
            let e = SCRATCH + 0x1_0000;
            t.write(e, &vec![0x3b; 0x3000]);
            let node = SCRATCH + 0x600;
            t.write(node, &[0x44; 0x20]);
            for o in [0x38, 0x47, 0x7e, 0x8d] {
                t.set_u32(op(&t, o), node);
            }
            t.set_u32(op(&t, 0x52), 0x0000_00ff);
            let root = op(&t, 0xdc);
            let last = if listed { SCRATCH + 0x640 } else { root };
            t.write(SCRATCH + 0x640, &[0x45; 0x20]);
            t.set_u32(op(&t, 0xcc), last);
            t.set_u16(op(&t, 0xf0), 3);
            t.set_u32(op(&t, 0x24b), flag);
            t.set_u32(op(&t, 0x1d6), 0x01d6_01d6);
            // the sprite node's parent: an empty child ring
            let parent = op(&t, 0x2a8);
            t.set_u32(parent + 0xc, parent);
            t.set_u32(parent + 0x10, parent);
            let call = Call {
                ecx: e,
                edx: 0x00ed_00ed,
                ..Call::default()
            };
            t.check(
                f,
                call,
                &[],
                Eax::Checked,
                ffx_magic::r11_misc::scene_ctor_p030,
            );
        }
    }
}

/// `magic_0064` 0x10002920 (`r11_misc::task_step_0064`): every slot
/// answers its number; the motion done or waiting; mode 1 with the delay
/// running, reaching 0 (the actor's model index below 0x20 or not, `G`
/// set or not) and out (the counter to 4, to 0x46); mode 0 with the
/// second delay reaching 0 and out; the counter past 0x64 with mode 1 and
/// not.
#[test]
fn task_step_0064() {
    task_step_in(64, 0x2920);
}

/// `magic_0052` 0x10001390 (the same step with `*X` and slot 594 before
/// the model reset): as `magic_0064`'s, then the counter past 0x64 with
/// `*X` 0, with slot 594 busy, and with both clear.
#[test]
fn task_step_0052() {
    task_step_in(52, 0x1390);
}

fn task_step_in(dll: u16, rva: u32) {
    let Some(mut t) = twin(dll) else { return };
    let f = t.at(rva);
    let op = |t: &magic::Twin, o: u32| t.img.word(f + o);
    for s in 0..1024u32 {
        t.stub(s, move |_, _| (s << 20) | 5);
    }
    // the float slots the arc and the clock light read
    t.stub_st0(333, |_, _| 0.75);
    t.stub_st0(364, |_, _| 2.0);
    t.stub_st0(16, |_, _| 1.5);
    let motion = t.img.rel32_target(f + 0x05);
    let done = t.img.word(motion + 0x02);
    let wait = t.img.word(motion + 0x11);
    let sel = SCRATCH + 0x180;
    t.set_u32(op(&t, 0x37), sel);
    // junk next to the actor byte
    t.set_u8(sel + 1, 0x40);
    for i in 0..0x40 {
        t.set_u32(op(&t, 0xe1) + 4 * i, 0x00c0_0000 + i);
    }
    t.set_u32(op(&t, 0x99), 0x0099_0099);
    let task = SCRATCH + 0x900;
    let node = SCRATCH + 0x980;
    // (mode M, delay C, G, byte, counter N, motion done r); the motion
    // waits (its wait word 5) when not done
    for (mode, c, g, b, n, r) in [
        (1u32, 2u32, 0u32, 3u8, 0u32, 0u32),
        (1, 1, 0, 3, 0x70, 1),
        (1, 1, 1, 3, 0, 0),
        (1, 1, 0, 0x30, 0, 0),
        (1, 0, 0, 3, 3, 1),
        (1, 0, 0, 3, 0x10, 0),
        (1, 0, 1, 3, 0x45, 0),
        (1, 0, 0, 3, 0x45, 0),
        (1, 0, 0, 0x30, 0x45, 0),
        (0, 1, 0, 3, 0x70, 1),
        (0, 2, 0, 3, 0x70, 1),
        (0, 0, 0, 3, 0, 1),
        (2, 1, 0, 3, 0x70, 1),
        (1, 1, 0, 0x20, 0, 0),
        (1, 0, 0, 3, 0xe, 0),
        (1, 0, 0, 3, 0x63, 1),
    ] {
        t.set_u32(op(&t, 0x0b), mode);
        t.set_u32(op(&t, 0x1a), c);
        t.set_u32(op(&t, 0x31), g);
        t.set_u8(sel, b);
        t.set_u32(op(&t, 0x6a), n);
        t.set_u32(done, r);
        t.set_u32(wait, 5);
        t.write(task, &[0x5e; 0x20]);
        t.set_u32(task + 8, node);
        t.write(node, &[0x5f; 0x20]);
        t.check_entry(f, Call::default(), &[task], Eax::Checked);
    }
    if dll == 52 {
        let busy = SCRATCH + 0xa0;
        t.stub(594, move |m, _| m.r32(busy));
        for (x, b) in [(0u32, 0u32), (1, 7), (1, 0)] {
            t.set_u32(op(&t, 0x0b), 1);
            t.set_u32(op(&t, 0x1a), 0);
            t.set_u32(op(&t, 0x6a), 0x70);
            t.set_u32(done, 1);
            t.set_u32(op(&t, 0x14d), x);
            t.set_u32(busy, b);
            t.write(task, &[0x5e; 0x20]);
            t.set_u32(task + 8, node);
            t.write(node, &[0x5f; 0x20]);
            t.check_entry(f, Call::default(), &[task], Eax::Checked);
        }
    }
}
