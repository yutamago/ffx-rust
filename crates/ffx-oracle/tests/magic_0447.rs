//! `magic_0447.dll` against its Rust ports (`ffx_magic::m0447`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{dump, f, rec, twin};

const N: u16 = 447;

/// 0x10005430: the camera's quads copied.
#[test]
fn camera_copy() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5430);
    let e = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x02), e);
    let bytes: Vec<u8> = (0..0x240u32)
        .map(|i| (i * 7 + 3).to_le_bytes()[0])
        .collect();
    t.write(e + 0x5c0, &bytes);
    t.check_entry(at, Call::default(), &[], Eax::Checked);
}

/// 0x10005290: object 1 or 3, into an empty and a filled task list.
#[test]
fn object_fade() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x5290);
    let state = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x07), state);
    // the object start's part and program table
    let so = t.img.rel32_target(at + 0x4d);
    t.set_u32(t.img.word(so + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(so + 0x5e), SCRATCH + 0x300);
    for i in 0..8 {
        t.set_u32(SCRATCH + 0x400 + 4 * i, 0x1111 * (i + 1));
    }
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    let sel = SCRATCH + 0x180;
    t.set_u32(t.img.word(at + 0x52), sel);
    t.set_u8(sel, 4);
    for i in 0..8 {
        t.set_u32(t.img.word(at + 0x62) + 4 * i, 0x00c0_0000 + i);
    }
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, 0x3f00_0000 + i);
        }
        0x262
    });
    rec(&mut t, 268, 3, |_, a| a[0] ^ 0x268);
    let take = t.img.rel32_target(at + 0x139);
    let fade = t.img.rel32_target(at + 0x18e);
    let ftake = t.img.rel32_target(fade + 0x75);
    assert_eq!(take, ftake);
    for (d4, last) in [(0u32, 0u32), (1, SCRATCH + 0x600), (5, 0)] {
        t.write(state, &[0x6c; 0x800]);
        t.set_u32(state + 0x7d4, d4);
        t.set_u32(state + 0x7c4, 0x3e80_0000);
        let node = |i: u32| SCRATCH + 0x8000 + 0x20 * i;
        for i in 0..4 {
            t.write(node(i), &[0x5a; 0x1c]);
            t.set_u32(node(i), node((i + 3) % 4));
            t.set_u32(node(i) + 4, node((i + 1) % 4));
        }
        t.set_u32(t.img.word(take + 0x17), node(0));
        t.set_u16(t.img.word(take + 0x04), 4);
        t.write(SCRATCH + 0x600, &[0x44; 0x1c]);
        t.set_u32(t.img.word(at + 0x13f), last);
        t.set_u32(t.img.word(at + 0x152), last);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10004cb0: the height from slot 251 or slots 17 and 19, the yaw on
/// each side of the wrap.
#[test]
#[allow(clippy::cast_precision_loss)]
fn camera_turn() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4cb0);
    let state = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x1b), state);
    let sel = SCRATCH + 0x180;
    t.set_u32(t.img.word(at + 0x11), sel);
    t.set_u32(t.img.word(at + 0xac), sel);
    for i in 0..0x30 {
        t.set_u32(t.img.word(at + 0x22) + 4 * i, 0x00c0_0000 + i);
    }
    t.set_u32(t.img.word(at + 0x52), 0x52);
    t.set_u32(t.img.word(at + 0x231), 0x231);
    rec(&mut t, 262, 4, |m, a| {
        for i in 0..3 {
            m.w32(a[1] + 4 * i, f(-2.0 + i as f32));
        }
        0x262
    });
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, f(0.25 * i as f32 - 1.0));
        }
        0x235
    });
    rec(&mut t, 229, 3, |m, a| {
        dump(m, 229, a[1], 16);
        dump(m, 230, a[2], 4);
        for i in 0..4 {
            m.w32(a[0] + 4 * i, f(3.0 + i as f32));
        }
        0x229
    });
    rec(&mut t, 248, 3, |m, a| {
        dump(m, 248, a[2], 4);
        0x248
    });
    for (s, n) in [(328u32, 3usize), (329, 3), (623, 7), (17, 2), (19, 1)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ s);
    }
    let (yaw, h) = (SCRATCH + 0x1c0, SCRATCH + 0x1c8);
    rec(&mut t, 333, 5, |_, a| a[0]);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(yaw))));
    rec(&mut t, 251, 1, |_, a| a[0]);
    t.stub_st0(251, move |m, _| f64::from(f32::from_bits(m.r32(h))));
    t.stub_st0(17, |_, _| 1.75);
    t.stub_st0(19, |_, _| -0.5);
    t.stub_st0(790, |_, _| 0.625);
    let fade = t.img.rel32_target(at + 0x28f);
    let take = t.img.rel32_target(fade + 0x75);
    for (flag, s, y, hh) in [
        (0u32, 3u8, 0.0f32, 1.5f32),
        (0, 0x20, 8.0, 1.5),
        (1, 3, 2.0, 1.5),
        (0, 0x1f, f32::NAN, -4.0),
        (2, 0x2f, -1.0, 0.0),
    ] {
        t.set_u32(t.img.word(at + 0xa5), flag);
        t.set_u8(sel, s);
        t.set_u32(yaw, f(y));
        t.set_u32(h, f(hh));
        t.write(state, &[0x6c; 0x800]);
        for i in 0..8 {
            t.set_u32(state + 0x5f0 + 4 * i, 0x5f00_0000 + i);
        }
        let node = |i: u32| SCRATCH + 0x8000 + 0x20 * i;
        for i in 0..3 {
            t.write(node(i), &[0x5a; 0x1c]);
            t.set_u32(node(i), node((i + 2) % 3));
            t.set_u32(node(i) + 4, node((i + 1) % 3));
        }
        t.set_u32(t.img.word(take + 0x17), node(0));
        t.set_u16(t.img.word(take + 0x04), 3);
        for op in [0x24du32, 0x257, 0x261, 0x26d] {
            t.set_u32(t.img.word(at + op), 0x0b0b_0000 + op);
        }
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10004f60: the countdown positive, at 0 and negative, three curves,
/// every slot path.
#[test]
fn camera_curve() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4f60);
    let state = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x12), state);
    let (flag, index, h) = (
        t.img.word(at + 0x1a7),
        t.img.word(at + 0x277),
        t.img.word(at + 0x2ab),
    );
    t.set_u32(h, 0x99);
    let pick = SCRATCH + 0x104;
    rec(&mut t, 255, 2, |m, a| {
        dump(m, 255, a[1], 3);
        0x255
    });
    rec(&mut t, 11, 1, move |m, _| m.r32(pick));
    rec(&mut t, 33, 4, |_, a| a[0] ^ 0x33);
    rec(&mut t, 32, 3, |_, a| a[0] ^ 0x32);
    let points = [
        [1.0f32, 2.0, 3.0, -4.0, 5.5, 0.25],
        [-10.0, 0.5, 100.0, 3.0, -2.0, 1.75],
    ];
    for (n, total) in [
        (30u32, 30u32),
        (17, 30),
        (1, 30),
        (0, 30),
        (0xffff_fff0, 30),
        (5, 7),
    ] {
        for p in &points {
            for (fl, ix, pk) in [
                (0u32, 3u32, 0u32),
                (0, 0x20, 0),
                (1, 3, 1),
                (2, 3, 2),
                (0, 0xffff_ffff, 0),
            ] {
                t.set_u32(state + 0x7c8, n);
                t.set_u32(state + 0x7cc, total);
                for (i, v) in (0u32..).zip(p) {
                    let o = if i < 3 {
                        0x610 + 4 * i
                    } else {
                        0x620 + 4 * (i - 3)
                    };
                    t.set_u32(state + o, f(*v));
                }
                t.set_u32(flag, fl);
                t.set_u32(index, ix);
                t.set_u32(pick, pk);
                t.check_entry(at, Call::default(), &[], Eax::Checked);
            }
        }
    }
}
