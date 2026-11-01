//! Round 9's shared own-row functions (`ffx_magic::r9_misc`) against each
//! copy's original. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;
mod magic_r9;

use ffx_magic::{Ret, Xmm, r9_misc};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

/// `magic_0447` 0x10003320 and `magic_0100` 0x100021a0 (internal: `edx`
/// = the object, `xmm3` the scale, the stack `(index)`): appended to an
/// empty list, to a list with a tail, and with the pool empty.
#[test]
fn start_object_state() {
    for (n, rva) in [(447u16, 0x3320u32), (100, 0x21a0)] {
        let Some(mut t) = twin(n) else { return };
        let at = t.at(rva);
        t.set_u32(t.img.word(at + 0x04), SCRATCH + 0x200);
        t.set_u32(t.img.word(at + 0x5e), SCRATCH + 0x300);
        t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
        for i in 0..8 {
            t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
        }
        t.stub(941, |m, a| {
            m.w32(SCRATCH + 0x900, a[0]);
            SCRATCH + 0x400
        });
        t.stub(714, |m, a| {
            for (i, w) in (0u32..).zip(&a[..5]) {
                m.w32(SCRATCH + 0x910 + 4 * i, *w);
            }
            0x714
        });
        let pa = t.img.rel32_target(at + 0x9c);
        let free = t.img.word(pa + 0x04);
        let nodes = t.img.word(pa + 0x27) - 8;
        for i in 0..8 {
            t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
            t.set_u32(nodes + 12 * i + 8, 0);
        }
        let tail = t.img.word(at + 0xa7);
        let obj = SCRATCH + 0x1000;
        for (k, (count, scale, tl)) in [
            (3u16, 0.5f32, 0u32),
            (2, -2.0, SCRATCH + 0x600),
            (0, 1.0, 0),
            (1, f32::NAN, SCRATCH + 0x700),
        ]
        .into_iter()
        .enumerate()
        {
            t.write(obj, &[0x5a; 0x130]);
            t.set_u16(free, count);
            t.set_u32(tail, tl);
            let c = Call {
                edx: obj,
                ecx: 0x0bad_0ecc,
                ..Call::default()
            }
            .with_xmm(&[Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(0.0), Xmm::f32(scale)]);
            let k = u32::try_from(k).unwrap_or(0);
            t.check(
                at,
                c,
                &[k * 2 + 1, 0x0a00 + k],
                Eax::Checked,
                |g, img, fr| {
                    Ret::eax(r9_misc::start_object_state(
                        g,
                        img,
                        fr.at,
                        fr.edx,
                        fr.xmm_f32(3),
                        fr.arg(g, 0),
                    ))
                },
            );
        }
    }
}

/// The object start with the argument reset (`magic_0182` 0x10001b70 and
/// its copies; internal: `edx` = the object, `xmm3` the scale): the
/// object list empty and not, the pool empty.
#[test]
fn start_object_reset_arg() {
    for (n, rva) in [
        (182u16, 0x1b70u32),
        (197, 0x1ad0),
        (252, 0x2000),
        (266, 0x3a20),
        (311, 0x2c60),
        (365, 0x1680),
    ] {
        reset_cases(n, rva, Shape::Arg);
    }
}

/// The object start with the bare reset (`magic_0245` 0x10002500 and
/// its copies).
#[test]
fn start_object_reset_bare() {
    for (n, rva) in [
        (245u16, 0x2500u32),
        (298, 0x2df0),
        (308, 0x1fd0),
        (435, 0x28a0),
    ] {
        reset_cases(n, rva, Shape::Bare);
    }
}

/// The object start in a frame with the `edx` reset (`magic_0108`
/// 0x10001c00 and its copies; the stack `(index, owner)`).
#[test]
fn start_object_reset_edx() {
    for (n, rva) in [(108u16, 0x1c00u32), (221, 0x1c00), (419, 0x38d0)] {
        reset_cases(n, rva, Shape::Edx);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Arg,
    Bare,
    Edx,
}

/// [`start_object_reset_arg`]'s cases for each shape.
fn reset_cases(dll: u16, rva: u32, shape: Shape) {
    let shift = if shape == Shape::Edx { 3 } else { 0 };
    {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(rva);
        let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        let start = t.img.rel32_target(f + 0x7d + shift);
        t.set_u32(op(&t, start, 0x04), SCRATCH + 0x7f8);
        t.set_u32(op(&t, f, 0x1 + shift), SCRATCH + 0x2f0);
        t.set_u32(op(&t, f, 0x5b + shift), SCRATCH + 0x280);
        for (o, v) in [(0x270u32, 0x0d00_0270u32), (0x290, 0x0d00_0290)] {
            t.set_u32(SCRATCH + o, v);
        }
        let pa = t.img.rel32_target(f + [0x95, 0x8f, 0x9b][shape as usize]);
        let free = op(&t, pa, 0x04);
        let nodes = op(&t, pa, 0x27) - 8;
        for i in 0..8 {
            t.set_u32(nodes + 12 * i, 0x0dd0_0000 + i);
            t.set_u32(nodes + 12 * i + 4, 0x0ee0_0000 + i);
            t.set_u32(nodes + 12 * i + 8, 0);
        }
        let tail = op(&t, f, [0xa0, 0x9a, 0xa6][shape as usize]);
        let obj = SCRATCH + 0x1000;
        for (k, (count, listed)) in [(5u16, false), (2, true), (0, true)]
            .into_iter()
            .enumerate()
        {
            let k = u32::try_from(k).unwrap_or(0);
            t.write(obj, &[0x5a; 0x140]);
            t.set_u32(SCRATCH + 0x2f0, 0x02f0_0000 + k);
            t.set_u16(free, count);
            t.write(SCRATCH + 0x1400, &[0x14; 0x10]);
            t.set_u32(tail, if listed { SCRATCH + 0x1400 } else { 0 });
            let call = Call {
                edx: obj,
                ecx: 0x0ecc_0000 + k,
                ..Call::default()
            }
            .with_xmm(&[
                Xmm::f32(9.0),
                Xmm::f32(8.0),
                Xmm::f32(7.0),
                Xmm::f32(1.25 + 0.5 * f32::from(u8::try_from(k).unwrap_or(0))),
            ]);
            t.check(
                f,
                call,
                &[0x0123 + k, 0x0bb0_0000 + k],
                Eax::Checked,
                |g, img, fr| {
                    Ret::eax(match shape {
                        Shape::Arg => r9_misc::start_object_reset_arg(g, img, fr),
                        Shape::Bare => r9_misc::start_object_reset_bare(g, img, fr),
                        Shape::Edx => r9_misc::start_object_reset_edx(g, img, fr),
                    })
                },
            );
        }
    }
}

/// The start-up with one object per actor (`magic_0108`, `0221`,
/// `0229` 0x10001370): no, one and three actors, one and three free
/// nodes, the task list empty and not.
#[test]
fn objects_start() {
    for (dll, first) in [(108u16, false), (221, false), (229, true)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x1370);
        for (n, free, listed) in [(0u32, 3u16, false), (1, 1, true), (3, 3, true)] {
            magic_r9::objects_setup(&mut t, f, n, free, listed);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                if first {
                    r9_misc::objects_start_first(g, img, fr)
                } else {
                    r9_misc::objects_start_edx(g, img, fr)
                }
            });
        }
    }
}

/// The start-up with one object (`magic_0166`, `0308` 0x10001340): the
/// yaw on each side of the wrap, and NaN; the task list empty and not.
#[test]
fn effect_start() {
    for (dll, first) in [(166u16, true), (308, false)] {
        let Some(mut t) = twin(dll) else { return };
        let f = t.at(0x1340);
        let kf = |t: &magic::Twin, o: u32| f32::from_bits(t.u32(t.img.word(f + o)));
        let hi_v = f64::from(kf(&t, 0xae) - kf(&t, 0xbd));
        for (yaw, listed) in [
            (0.0f64, false),
            (8.0, true),
            (2.0, true),
            (f64::NAN, false),
            (hi_v, true),
        ] {
            magic_r9::effect_setup(&mut t, f, yaw, 3, listed);
            t.check(f, Call::default(), &[], Eax::Checked, |g, img, fr| {
                if first {
                    r9_misc::effect_start_first(g, img, fr)
                } else {
                    r9_misc::effect_start_bare(g, img, fr)
                }
            });
        }
    }
}

/// The task step with slot 542 per actor (`magic_0391` 0x10001310,
/// `magic_0448` 0x10004c10): the script and the motion each done or
/// not; when both are, the actors, the parts and the node given back.
#[test]
fn task_step_542() {
    for (dll, rva) in [(391u16, 0x1310u32), (448, 0x4c10)] {
        let Some(mut t) = twin(dll) else { return };
        let at = t.at(rva);
        let w = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
        let state = SCRATCH + 0x1_0000;
        t.set_u32(w(&t, at, 0x07), state);
        t.write(w(&t, at, 0x10), &[0x3b; 0x30]);
        t.set_u32(w(&t, at, 0x26), 0x0026_0026);
        let run = t.img.rel32_target(at + 0x33);
        let motion = t.img.rel32_target(at + 0x3a);
        let each = t.img.rel32_target(at + 0x56);
        let gb = t.img.rel32_target(at + 0x66);
        t.set_u32(w(&t, run, 0x07), SCRATCH + 0x700);
        rec(&mut t, 542, 2, |_, a| 0x0542_0000 ^ a[0]);
        let map = SCRATCH + 0x180;
        t.set_u32(w(&t, each, 0x1d), map);
        t.write(map, &[7, 9, 11, 13]);
        t.set_u32(w(&t, each, 0x08), 2);
        let node = SCRATCH + 0x200;
        let (prev, next) = (SCRATCH + 0x280, SCRATCH + 0x2c0);
        for (mode, done) in [(0u8, 1u32), (7, 1), (0, 0), (7, 0)] {
            t.set_u8(w(&t, run, 0x01), mode);
            t.set_u32(w(&t, motion, 0x02), done);
            t.set_u16(w(&t, motion, 0x11), 5);
            t.write(state, &[0x6c; 0x1000]);
            t.write(node, &[0x3c; 0x20]);
            t.set_u32(node, prev);
            t.set_u32(node + 4, next);
            t.set_u32(node + 8, node);
            t.set_u32(node + 0xc, 0);
            t.write(prev, &[0x11; 0x10]);
            t.write(next, &[0x22; 0x10]);
            t.set_u32(w(&t, gb, 0x74), 0);
            t.set_u16(w(&t, gb, 0x8e), 7);
            t.set_u32(w(&t, gb, 0x48), node);
            t.check_entry(at, Call::default(), &[node], Eax::Checked);
        }
    }
}

/// The emitter's start (`magic_0285` 0x10004e20, `magic_0286`
/// 0x10001cd0): the object start, slot 325 and the puff, the puff's ring
/// full and not.
#[test]
fn smoke_start() {
    for (dll, rva) in [(285u16, 0x4e20u32), (286, 0x1cd0)] {
        let Some(mut t) = twin(dll) else { return };
        let at = t.at(rva);
        let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
        // the object start's part table, lock and pool
        let so = t.img.rel32_target(at + 0x1c);
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        t.set_u32(op(&t, so, 0x04), SCRATCH + 0x2f0);
        t.set_u32(op(&t, so, 0x5e), SCRATCH + 0x280);
        let sp = t.img.rel32_target(so + 0x84);
        t.set_u32(op(&t, sp, 0x04), SCRATCH + 0x7f8);
        rec(&mut t, 325, 2, |_, a| 0x0325_0000 ^ a[1]);
        rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[0]);
        let calls = SCRATCH + 0x7e0;
        rec(&mut t, 314, 1, |_, a| 0x0314_0000 ^ (a[0] & 0xff));
        t.stub_st0(314, move |m, a| {
            let k = m.r32(calls);
            m.w32(calls, k + 1);
            f64::from(f32::from_bits(a[0])) * f64::from(k % 5 + 1)
        });
        let puff = t.img.rel32_target(at + 0x44);
        let take = t.img.rel32_target(puff + 0x9);
        let ring = SCRATCH + 0x1000;
        t.set_u32(op(&t, take, 0x17), ring);
        t.set_u32(op(&t, take, 0x0a), 4);
        let e = SCRATCH + 0x300;
        for (k, (full, y)) in [(true, 0.0f32), (false, -2.0), (false, 5.0)]
            .into_iter()
            .enumerate()
        {
            let k = u8::try_from(k).unwrap_or(0);
            t.write(ring, &[0x30 + k; 0x68 * 4]);
            for i in 0..4u32 {
                t.set_u32(ring + 0x68 * i + 0x60, u32::from(full || i != 2));
            }
            t.set_u32(op(&t, take, 0x02), 1);
            t.write(e, &[0x50 + k; 0x280]);
            t.set_u32(e + 0x250, y.to_bits());
            t.set_u32(e + 0x25c, 1.5f32.to_bits());
            t.set_u32(e + 0x264, (-0.75f32).to_bits());
            t.check_entry(at, Call::default(), &[e], Eax::Checked);
        }
    }
}

/// The main object and its fade (`magic_0392` 0x10002120, `magic_0449`
/// 0x10005c40, internal): the task list empty and not.
#[test]
fn main_object_fade() {
    for (dll, rva) in [(392u16, 0x2120u32), (449, 0x5c40)] {
        let Some(mut t) = twin(dll) else { return };
        let at = t.at(rva);
        let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
        let s = SCRATCH + 0x1_0000;
        t.set_u32(op(&t, at, 0x0c), s);
        let so = t.img.rel32_target(at + 0x1c);
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        t.set_u32(op(&t, so, 0x04), SCRATCH + 0x2f0);
        t.set_u32(op(&t, so, 0x5e), SCRATCH + 0x280);
        let fade = t.img.rel32_target(at + 0x9d);
        let take = t.img.rel32_target(fade + 0x75);
        for listed in [false, true] {
            t.write(s, &vec![0x6c; 0x6000]);
            let node = |i: u32| SCRATCH + 0x8000 + 0x20 * i;
            for i in 0..3 {
                t.write(node(i), &[0x5a; 0x20]);
                t.set_u32(node(i), node((i + 2) % 3));
                t.set_u32(node(i) + 4, node((i + 1) % 3));
            }
            t.set_u32(op(&t, take, 0x17), node(0));
            t.set_u16(op(&t, take, 0x04), 3);
            t.write(SCRATCH + 0x600, &[0x44; 0x10]);
            let head = op(&t, fade, 0x7b);
            t.set_u32(head, if listed { SCRATCH + 0x600 } else { 0 });
            t.set_u32(op(&t, fade, 0x93), SCRATCH + 0x600);
            t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
                r9_misc::main_object_fade(g, img, fr)
            });
        }
    }
}

/// Two objects of the block (`magic_0350` 0x10006780, `magic_0352`
/// 0x10004db0): the object list empty, then filled.
#[test]
fn two_objects() {
    for (dll, rva) in [(350u16, 0x6780u32), (352, 0x4db0)] {
        let Some(mut t) = twin(dll) else { return };
        let at = t.at(rva);
        let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
        let b = SCRATCH + 0x1_0000;
        t.set_u32(op(&t, at, 0x0b), b);
        t.set_u32(op(&t, at, 0x39), 0x3939);
        t.set_u32(op(&t, at, 0x82), 0x8282);
        let so = t.img.rel32_target(at + 0x1d);
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        rec(&mut t, 237, 4, |m, a| {
            for i in 0..3 {
                m.w32(a[3] + 4 * i, 0x3e00_0000 + a[1] + i);
            }
            0x237
        });
        t.set_u32(op(&t, so, 0x04), SCRATCH + 0x2f0);
        t.set_u32(op(&t, so, 0x5e), SCRATCH + 0x280);
        for k in 0..2u8 {
            t.write(b, &[0x6c + k; 0xa00]);
            t.set_u32(b + 0x984, 0x0984_0000 + u32::from(k));
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// The object at the block's point (`magic_0165` 0x10002e40 and
/// 0x10003310): the object list empty, then filled.
#[test]
fn object_at_point() {
    for rva in [0x2e40u32, 0x3310] {
        let Some(mut t) = twin(165) else { return };
        let at = t.at(rva);
        let op = |t: &magic::Twin, a: u32, o: u32| t.img.word(a + o);
        let b = SCRATCH + 0x1_0000;
        t.set_u32(op(&t, at, 0x0b), b);
        let so = t.img.rel32_target(at + 0x1e);
        let table = SCRATCH + 0x3400;
        for i in 0..0x10 {
            t.set_u32(table + 4 * i, 0x1111 * (i + 1));
        }
        rec(&mut t, 941, 1, move |_, _| table);
        rec(&mut t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
        t.set_u32(op(&t, so, 0x04), SCRATCH + 0x2f0);
        t.set_u32(op(&t, so, 0x5e), SCRATCH + 0x280);
        for k in 0..2u32 {
            t.write(b, &vec![0x6c; 0x6000]);
            for (i, o) in (0u32..).zip([0x24u32, 0x30, 0x3c, 0x48, 0x64]) {
                let v = (1.5 + 0.25 * f32::from(u16::try_from(i + 4 * k).unwrap_or(0))).to_bits();
                t.set_u32(b + op(&t, at, o), v);
            }
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}
