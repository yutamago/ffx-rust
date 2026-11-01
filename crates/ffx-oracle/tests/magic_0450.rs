//! `magic_0450.dll` against its Rust port (`ffx_magic::m0450`): `DllMain`
//! against every bound port, the effect's own functions. Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity
)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0450};
use magic::{Call, Eax, SCRATCH, Twin};
use magic_hi::lone_node;
use magic_owna::{f, rec, recs, twin};

const N: u16 = 450;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

fn fx(t: &mut Twin, at: u32, op: u32) -> u32 {
    let fx = SCRATCH + 0x1000;
    t.set_u32(t.img.word(at + op), fx);
    fx
}

/// 0x10005860: the angle wrapped down, up and not at all.
#[test]
fn orient() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5860);
    let fx = fx(&mut t, at, 0x8);
    let k = |t: &Twin, op: u32| f32::from_bits(t.u32(t.img.word(at + op)));
    let (c, hi, lo) = (k(&t, 0x4f), k(&t, 0x5e), k(&t, 0x72));
    t.set_u32(t.img.word(at + 0x22), 0x00ac_7000);
    t.set_u32(t.img.word(at + 0x3c), 0x00ac_7001);
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            m.w32(a[i], f(i as f32 * 1.5));
        }
        0x262
    });
    let cell = SCRATCH + 0x10;
    rec(&mut t, 333, 5, |_, _| 0x0333_0000);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(cell))));
    for angle in [c - (hi + 0.5), c - (lo - 0.25), c - 0.125, c - hi, c - lo] {
        t.set_u32(cell, f(angle));
        t.write(fx + 0x1c00, &[0x3c; 0x300]);
        magic_h6::scrub(&mut t);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0450::orient(g, img, fr.at))
        });
    }
}

/// 0x10005930: one model and two, into the parts' registration (one part,
/// no bank).
#[test]
fn show_models() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5930);
    let fx = fx(&mut t, at, 0xe);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x17), b);
    // the byte after each index non-zero (an index read as u16 shows)
    t.set_u32(b, 0x2a05_0300);
    let table = t.img.word(at + 0x22);
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0384_0000 + i);
    }
    let rp = t.img.rel32_target(at + 0x61);
    let parts = SCRATCH + 0x200;
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(parts + 0x30, 0x0941_0001);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    t.set_u32(t.img.word(rp + 0x76), 0);
    let lock = SCRATCH + 0x300;
    t.set_u32(t.img.word(rp + 0x43), lock);
    recs(&mut t, &[(628, 1), (384, 1), (941, 1), (772, 2), (779, 4)]);
    for model in [u32::MAX, 0x77] {
        t.set_u32(fx + 0x1eec, model);
        magic_h6::scrub(&mut t);
        t.check(at, Call::default(), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0450::show_models(g, img, fr.at))
        });
    }
}

/// 0x100057a0: slot 561 busy, an object still running, the state not 1,
/// the flag set, the count at 0x20, and the full path.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x57a0);
    let fx = fx(&mut t, at, 0x1e);
    let ro = t.img.rel32_target(at + 0x29);
    let busy = SCRATCH + 0x40;
    rec(&mut t, 561, 0, move |m, _| m.r32(busy));
    rec(&mut t, 217, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, 0x0217_0000 + i);
        }
        0x217
    });
    rec(&mut t, 255, 4, |m, a| m.r32(a[1] + 8));
    recs(&mut t, &[(211, 2), (214, 3)]);
    t.set_u32(t.img.word(at + 0x44), 0x00ac_7000);
    t.set_u32(t.img.word(at + 0x93), 0x00cc_0000);
    let (node, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    let list = t.img.word(ro + 0x7c);
    for (b, running, state, flag, n) in [
        (1u32, false, 1u32, 0u32, 3u32),
        (0, true, 1, 0, 3),
        (0, false, 0, 0, 3),
        (0, false, 1, 1, 3),
        (0, false, 1, 0, 0x20),
        (0, false, 1, 0, 0x1f),
        (0, false, 1, 0, 0xffff_fff0),
    ] {
        t.set_u32(busy, b);
        t.set_u32(node + 8, obj);
        t.set_u16(obj + 0x124, 0x8001);
        t.set_u32(list + 8, if running { node } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if running { node } else { 0 });
        t.set_u32(fx + 0x1efc, state);
        t.set_u32(t.img.word(at + 0x3c), flag);
        t.set_u32(t.img.word(at + 0x4c), n);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10005a00: no first model, one with the mode 6 and another.
#[test]
fn load_models() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5a00);
    let fx = fx(&mut t, at, 0x3);
    rec(&mut t, 0, 1, |_, a| {
        if a[0] == 0x4029 {
            0x00ab_0000
        } else {
            0x00a0_0000 ^ a[0]
        }
    });
    recs(&mut t, &[(26, 3), (10, 5), (632, 3)]);
    // the byte after the mode byte the load sets, non-zero
    let mode_byte = t.img.word(at + 0x37);
    for (model, mode) in [(u32::MAX, 6u32), (0x10, 6), (0x11, 2)] {
        t.set_u32(mode_byte, 0x6d6d_6d00);
        t.set_u32(fx + 0x1eec, model);
        t.set_u32(fx + 0x1ee8, mode);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10005e00: the height from slot 251, and from slots 19 and 17 (the
/// flag set, the actor index 0x20 and up).
#[test]
fn place_camera() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5e00);
    let fx = fx(&mut t, at, 0x21);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x19), b);
    t.set_u32(t.img.word(at + 0xa1), b);
    let table = t.img.word(at + 0x6b);
    for i in 0..0x40 {
        t.set_u32(table + 4 * i, 0x0019_0000 + i);
    }
    rec(&mut t, 251, 1, |_, a| 0x0251_0000 ^ a[0]);
    t.stub_st0(251, |_, a| f64::from(a[0]) * 0.75 + 0.1);
    rec(&mut t, 19, 1, |_, _| 0x19);
    t.stub_st0(19, |_, a| f64::from(a[0] & 0xff) * 1.25 - 3.0);
    rec(&mut t, 17, 2, |_, _| 0x17);
    t.stub_st0(17, |_, a| f64::from(a[0] & 0xff) * -0.5 + 0.3);
    rec(&mut t, 262, 4, |m, a| {
        for i in 1..4 {
            m.w32(a[i], f(i as f32 + 0.25));
        }
        0x262
    });
    rec(&mut t, 235, 5, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, f(i as f32 * 0.5));
        }
        0x235
    });
    rec(&mut t, 328, 3, |m, a| {
        let v = m.r32(a[0] + 4);
        m.w32(a[0] + 4, v ^ 0x40);
        0x328
    });
    rec(&mut t, 329, 3, |m, a| {
        let v = m.r32(a[0] + 8);
        m.w32(a[0] + 8, v ^ a[2]);
        0x329
    });
    rec(&mut t, 229, 6, |m, a| {
        // the vector's w mixed into every lane, so a w left out shows
        let w = m.r32(a[2] + 0xc);
        for i in 0..4 {
            let v = (m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i)).wrapping_add(w);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(&mut t, 248, 3, |m, a| {
        let w = m.r32(a[1] + 0xc);
        for i in 0..4 {
            let v = m
                .r32(a[1] + 4 * i)
                .wrapping_add(m.r32(a[2] + 4 * i))
                .wrapping_add(w);
            m.w32(a[0] + 4 * i, v);
        }
        0x248
    });
    for (flag, actor) in [(0u32, 3u8), (0, 0x1f), (1, 2), (0, 0x20), (0, 0x3f)] {
        t.set_u32(t.img.word(at + 0x12), flag);
        // the byte after the actor index non-zero
        t.set_u32(b, 0x0017_3c00 | u32::from(actor));
        t.write(fx + 0x1b80, &[0x5a; 0x380]);
        t.set_u32(fx + 0x1ee0, f(0.5 + f32::from(actor)));
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100063e0: the count running down (2, 1) and run out (0, -3).
#[test]
fn blend_camera() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x63e0);
    let fx = fx(&mut t, at, 0x2);
    for i in 0..3 {
        t.set_u32(fx + 0x1c30 + 4 * i, f(1.5 + i as f32));
        t.set_u32(fx + 0x1c40 + 4 * i, f(-7.25 * (i as f32 + 1.0)));
    }
    for n in [2u32, 1, 0, 0xffff_fffd, 40] {
        t.set_u32(fx + 0x1ef0, n);
        t.write(fx + 0x1ba0, &[0x77; 0x10]);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// Stubs for the matrix code: 235 fills a matrix, 277/15/16/17 give
/// floats, 328/329/330 turn (change a word by the angle), 229 and 248
/// mix vectors.
fn matrix_stubs(t: &mut Twin) {
    rec(t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, f(i as f32 * 0.5 - 1.0));
        }
        0x235
    });
    for (slot, k) in [(277u32, 0.75f64), (15, 1.5), (16, -2.25), (17, 3.125)] {
        rec(t, slot, 1, move |_, _| slot);
        t.stub_st0(slot, move |_, a| f64::from(a[0] & 0xff) * k + 0.0625);
    }
    for (slot, i) in [(328u32, 1u32), (329, 2), (330, 6)] {
        rec(t, slot, 3, move |m, a| {
            let v = m.r32(a[0] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ a[2]);
            slot << 8
        });
    }
    rec(t, 229, 6, |m, a| {
        // the vector's w mixed into every lane, so a w left out shows
        let w = m.r32(a[2] + 0xc);
        for i in 0..4 {
            let v = (m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i)).wrapping_add(w);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(t, 248, 3, |m, a| {
        let w = m.r32(a[1] + 0xc);
        for i in 0..4 {
            let v = m
                .r32(a[1] + 4 * i)
                .wrapping_add(m.r32(a[2] + 4 * i))
                .wrapping_add(w);
            m.w32(a[0] + 4 * i, v);
        }
        0x248
    });
}

/// 0x10006190 and 0x10006490: the swing's start and the model reset.
#[test]
fn swing_and_reset() {
    let Some(mut t) = twin(N) else { return };
    matrix_stubs(&mut t);
    for (rva, op, actors) in [
        (0x6190u32, 0x13u32, [0x148u32, 0x174]),
        (0x6490, 0xc, [0x32, 0x63]),
    ] {
        let at = t.at(rva);
        let fx = fx(&mut t, at, op);
        for (k, actor) in [0x11u32, 0x20].into_iter().enumerate() {
            t.set_u32(t.img.word(at + actors[0]), actor);
            t.set_u32(t.img.word(at + actors[1]), actor + 1);
            t.write(fx + 0x1b40, &[0x1d; 0x400]);
            for i in 0..4 {
                t.set_u32(fx + 0x1ba0 + 4 * i, f(2.5 * i as f32 - k as f32));
                t.set_u32(fx + 0x1c10 + 4 * i, f(-0.75 * i as f32));
                t.set_u32(fx + 0x1b90 + 4 * i, f(0.375 * i as f32 + 0.1));
            }
            magic_h6::scrub(&mut t);
            t.check_entry(at, Call::default(), &[], Eax::Checked);
        }
    }
}

/// The `f32` at the address in the operand `op` of the code at `at`.
fn konst(t: &Twin, at: u32, op: u32) -> f32 {
    f32::from_bits(t.u32(t.img.word(at + op)))
}

/// Angles that [`wrap_once`](ffx_magic::ob_model::wrap_once) takes down,
/// up and leaves, after `step` is added (`hi`, `lo` the limits).
fn angles(hi: f32, lo: f32, step: f32) -> [f32; 5] {
    [
        hi + 0.5 - step,
        lo - 0.25 - step,
        0.125,
        hi - step,
        lo - step,
    ]
}

/// 0x100065e0: the flight's start with the angle wrapped three ways and
/// two actors.
#[test]
fn flight_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x65e0);
    let fx = fx(&mut t, at, 0x18);
    matrix_stubs(&mut t);
    recs(&mut t, &[(210, 2), (211, 2), (217, 2), (255, 4)]);
    let b = SCRATCH + 0x20;
    for op in [0x2a2, 0x2e3, 0x40b] {
        t.set_u32(t.img.word(at + op), b);
    }
    let table = t.img.word(at + 0x2ad);
    for i in 0..0x40 {
        t.set_u32(table + 4 * i, 0x00ac_0000 + i);
    }
    let (hi, lo) = (konst(&t, at, 0x51), konst(&t, at, 0x7f));
    let step = konst(&t, at, 0x30);
    for (k, a) in angles(hi, lo, step).into_iter().enumerate() {
        t.set_u32(b, 0x0019_0300 + ((k as u32 * 7) << 8));
        t.write(fx + 0x1b40, &[0x2e; 0x400]);
        t.set_u32(fx + 0x1ee0, f(a));
        for i in 0..4 {
            t.set_u32(fx + 0x1c10 + 4 * i, f(1.25 * i as f32 - k as f32));
        }
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10006a70: the flight's frame, the count running and run out, the
/// angle wrapped three ways.
#[test]
fn flight_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6a70);
    let fx = fx(&mut t, at, 0xf);
    matrix_stubs(&mut t);
    let b = SCRATCH + 0x20;
    for op in [0x109, 0x144] {
        t.set_u32(t.img.word(at + op), b);
    }
    t.set_u32(b, 0x0019_0500);
    let table = t.img.word(at + 0x114);
    for i in 0..0x40 {
        t.set_u32(table + 4 * i, 0x00ac_0000 + i);
    }
    let (hi, lo) = (konst(&t, at, 0xc7), konst(&t, at, 0xdb));
    let step = konst(&t, at, 0xc0);
    let counts = [5u32, 1, 0, 0xffff_fffe, 30];
    for (k, a) in angles(hi, lo, step).into_iter().enumerate() {
        t.write(fx + 0x1b40, &[0x2e; 0x400]);
        t.set_u32(fx + 0x1bf0, f(a));
        t.set_u32(fx + 0x1ef0, counts[k]);
        for i in 0..4 {
            t.set_u32(fx + 0x1c30 + 4 * i, f(1.25 * i as f32 - k as f32));
            t.set_u32(fx + 0x1c40 + 4 * i, f(-3.5 * i as f32 + 0.5));
        }
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10006da0: before and after frame 0x1b, the count above 0, at 0 and
/// below.
#[test]
fn shrink() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6da0);
    rec(&mut t, 723, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, f(i as f32 + 0.5));
        }
        0x723
    });
    rec(&mut t, 724, 2, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v ^ 0x8000_0000);
        }
        0x724
    });
    rec(&mut t, 327, 2, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x327
    });
    rec(&mut t, 229, 3, |m, a| {
        // the vector's w mixed into every lane, so a w left out shows
        let w = m.r32(a[2] + 0xc);
        for i in 0..4 {
            let v = (m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i)).wrapping_add(w);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    let cell = SCRATCH + 0x10;
    rec(&mut t, 282, 0, |_, _| 0x0282_0000);
    t.stub_st0(282, move |m, _| f64::from(f32::from_bits(m.r32(cell))));
    let obj = SCRATCH + 0x2000;
    for (frame, n, s) in [
        (0x1bu32, 4u32, 2.0f32),
        (0x1c, 4, 0.5),
        (0x40, 1, -3.0),
        (0x40, 0, 7.0),
        (0x40, 0xffff_fffb, 1.0e-3),
        (0x1c, 0x0012_3456, 1.0),
        (0xffff_ff00, 3, 2.0),
    ] {
        t.write(obj, &[0x4b; 0x140]);
        t.set_u32(obj + 0x118, frame);
        t.set_u32(obj + 0x120, n);
        t.set_u32(cell, f(s));
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10007ca0: the angle wrapped three ways.
#[test]
fn turn_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7ca0);
    let fx = fx(&mut t, at, 0x2);
    let (hi, lo) = (konst(&t, at, 0x61), konst(&t, at, 0x86));
    let step = konst(&t, at, 0x5a);
    for (k, a) in angles(hi, lo, step).into_iter().enumerate() {
        t.write(fx + 0x1b80, &[0x6c; 0x400]);
        for i in 0..4 {
            t.set_u32(fx + 0x1ba0 + 4 * i, f(2.0 * i as f32 + k as f32));
        }
        t.set_u32(fx + 0x1c14, f(4.5 - k as f32));
        t.set_u32(fx + 0x1b90, f(a));
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// The object start's (`s4_misc::start_object_at` at `so`) part table,
/// lock and pool.
fn object_start(t: &mut Twin, so: u32) {
    let table = SCRATCH + 0x3400;
    for i in 0..0x10 {
        t.set_u32(table + 4 * i, 0x1111 * (i + 1));
    }
    rec(t, 941, 1, move |_, _| table);
    rec(t, 714, 5, |_, a| 0x0714_0000 ^ a[2]);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x2f0);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x280);
    let sp = t.img.rel32_target(so + 0x84);
    t.set_u32(t.img.word(sp + 0x04), SCRATCH + 0x7f8);
}

/// 0x100056b0: slot 561 busy; the script done and not, the first motion
/// script done and not (done: the models shown and the node given back).
#[test]
fn task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x56b0);
    let fx = fx(&mut t, at, 0x7);
    let busy = SCRATCH + 0x40;
    rec(&mut t, 561, 0, move |m, _| m.r32(busy));
    // the block memset clears, dirty; the half-word or'ed
    let block = t.img.word(at + 0x10);
    t.set_u32(t.img.word(at + 0x26), 0x0026_1234);
    let run = t.img.rel32_target(at + 0x33);
    let (m1, m2) = (t.img.word(at + 0x51), t.img.word(at + 0x5b));
    // show_models: the model table and the parts' registration
    let sm = t.img.rel32_target(at + 0x79);
    t.set_u32(t.img.word(sm + 0xe), fx);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(sm + 0x17), b);
    t.set_u32(b, 0x2a05_0300);
    let table = t.img.word(sm + 0x22);
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0384_0000 + i);
    }
    let rp = t.img.rel32_target(sm + 0x61);
    let parts = SCRATCH + 0x200;
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(parts + 0x30, 0x0941_0001);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    t.set_u32(t.img.word(rp + 0x76), 0);
    t.set_u32(t.img.word(rp + 0x43), SCRATCH + 0x300);
    recs(&mut t, &[(628, 1), (384, 1), (941, 1), (772, 2), (779, 4)]);
    let give = t.img.rel32_target(at + 0x85);
    let node = SCRATCH + 0x5000;
    for (bz, mode, done1, done2) in [
        (7u32, 0u8, 1u32, 1u32),
        (0, 0, 1, 0),
        (0, 0, 0, 1),
        (0, 2, 1, 1),
    ] {
        t.set_u32(busy, bz);
        t.write(block, &[0x5c; 0x30]);
        t.set_u8(t.img.word(run + 0x01), mode);
        t.set_u16(t.img.word(run + 0x59), 0xffff);
        for (ms, done) in [(m1, done1), (m2, done2)] {
            t.set_u32(ms + 0x14, done);
            t.set_u16(ms + 0x18, 3);
        }
        t.set_u32(fx + 0x1eec, u32::MAX);
        t.write(fx + 0x1b80, &[0x61; 0xa0]);
        for i in 0..4 {
            t.set_u32(fx + 0x1ba0 + 4 * i, 0x1ba0_0000 + i);
            t.set_u32(fx + 0x1c00 + 4 * i, 0x1c00_0000 + i);
        }
        lone_node(&mut t, give, node);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}

/// 0x10006ed0: one model and two.
#[test]
fn second_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6ed0);
    let fx = fx(&mut t, at, 0x3);
    let b = SCRATCH + 0x20;
    t.set_u32(t.img.word(at + 0x40), b);
    t.set_u32(b, 0x2a05_0300);
    let table = t.img.word(at + 0x4b);
    for i in 0..8 {
        t.set_u32(table + 4 * i, 0x0632_0000 + i);
    }
    t.set_u32(t.img.word(at + 0x6f), 0x006f_0001);
    recs(&mut t, &[(632, 3)]);
    let so = t.img.rel32_target(at + 0x95);
    object_start(&mut t, so);
    for model in [u32::MAX, 0x77] {
        t.write(fx + 0x800, &[0x3c; 0x200]);
        t.write(fx + 0x1ba0, &[0x2b; 0x80]);
        t.set_u32(fx + 0x1eec, model);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::this(0x00ec_0001), &[], Eax::Checked);
    }
}

/// 0x10006ff0 and 0x10007c40: the first object's start, the scale task.
#[test]
fn first_and_scale_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x6ff0);
    let fx = fx(&mut t, at, 0xb);
    let so = t.img.rel32_target(at + 0x1e);
    object_start(&mut t, so);
    t.write(fx + 0x5f0, &[0x3c; 0x200]);
    t.set_u32(fx + 0x1ee0, f(1.25));
    magic_h6::scrub(&mut t);
    t.check_entry(at, Call::this(0x00ec_0002), &[], Eax::Checked);
    let at = t.at(0x7c40);
    let st = t.img.rel32_target(at + 0x4f);
    let take = t.img.rel32_target(st + 0x7d);
    magic_hi::pool(&mut t, take, SCRATCH + 0x6000, 3);
    t.write(fx + 0x1ba0, &[0x2b; 0x80]);
    t.write(fx + 0x1e08, &[0x3d; 0x40]);
    t.set_u32(fx + 0x1c10, f(2.5));
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}
