//! `magic_0098.dll` against its Rust port (`ffx_magic::m0098`): `DllMain`
//! against every bound port, the target height and the view matrix
//! (internal). Needs `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0098};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{f, filler, rec, twin};

const N: u16 = 98;

/// Junk under the entry on both sides: the twins start each case with
/// equal stacks, so a local the port leaves unwritten would otherwise
/// hold the original's value from the case before.
fn dirty(t: &mut magic::Twin) {
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
}

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn target_height() {
    let Some(mut t) = twin(N) else { return };
    let id = SCRATCH + 0x100;
    rec(&mut t, 390, 1, move |m, _| m.r32(id));
    rec(&mut t, 215, 2, |_, a| a[0] ^ 0x215);
    rec(&mut t, 19, 1, |_, a| a[0] ^ 0x19);
    t.stub_st0(19, |_, a| f64::from(a[0] & 0xff) * 0.37 + 0.1);
    t.set_u32(t.img.word(t.at(0x11a0) + 0x0c), 0x44);
    for v in [0x1015u32, 0x1077, 0x1127, 0x1000, 0] {
        t.set_u32(id, v);
        t.check(
            t.at(0x11a0),
            Call::default(),
            &[],
            Eax::Checked,
            |g, img, fr| m0098::target_height(g, img, fr.at, fr.ecx),
        );
    }
}

#[test]
fn view_matrix() {
    let Some(mut t) = twin(N) else { return };
    filler(&mut t, 325, 2, 4);
    // the normalisation: its output from its input, so both are seen
    rec(&mut t, 232, 4, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ 0x0010_0000;
            m.w32(a[0] + 4 * i, v);
        }
        0x232
    });
    rec(&mut t, 266, 7, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x266
    });
    let mat = SCRATCH + 0x800;
    for k in [0u32, 1] {
        t.write(mat, &[0xa5; 0x40]);
        t.set_u32(mat, f(k as f32));
        t.check(
            t.at(0x1250),
            Call::this(mat),
            &[],
            Eax::Checked,
            |g, img, fr| Ret::eax(m0098::view_matrix(g, img, fr.at, fr.ecx, fr.esp)),
        );
    }
}

#[test]
fn steps_and_callbacks() {
    let Some(mut t) = twin(N) else { return };
    rec(&mut t, 276, 2, |_, a| a[0] ^ 0x276);
    t.set_u32(t.img.word(t.at(0x2130) + 0x09), 0x66);
    t.check_entry(t.at(0x2130), Call::default(), &[], Eax::Checked);
    t.check_entry(t.at(0x2210), Call::default(), &[], Eax::Checked);
    // draw: the camera slots and the part step's answer
    let d = t.at(0x2660);
    t.set_u32(t.img.word(d + 0x55), SCRATCH + 0x200);
    t.set_u32(t.img.word(d + 0x65), SCRATCH + 0x204);
    t.set_u32(SCRATCH + 0x204, 7);
    rec(&mut t, 230, 2, |_, a| a[0] ^ 0x230);
    rec(&mut t, 231, 3, |_, a| a[0] ^ 0x231);
    let ans = SCRATCH + 0x208;
    rec(&mut t, 712, 9, move |m, _| {
        let v = m.r32(SCRATCH + 0x204);
        m.w32(SCRATCH + 0x20c, v);
        m.r32(ans)
    });
    for r in [0u32, 1] {
        t.set_u32(ans, r);
        t.check_entry(d, Call::default(), &[SCRATCH + 0x1000], Eax::Checked);
    }
    // start: the part and the position
    let s = t.at(0x29d0);
    t.set_u32(t.img.word(s + 0x04), SCRATCH + 0x200);
    t.set_u32(t.img.word(s + 0x10), SCRATCH + 0x300);
    t.set_u32(SCRATCH + 0x310, 0x00ab_cdef);
    t.set_u32(SCRATCH + 0x400, 0x1111);
    rec(&mut t, 941, 1, |_, _| SCRATCH + 0x400);
    rec(&mut t, 714, 5, |_, _| 0x714);
    filler(&mut t, 325, 6, 4);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x5a; 0x80]);
    t.check_entry(s, Call::default(), &[obj], Eax::Checked);
}

/// The model load step: first time (both command records) and later.
#[test]
fn load() {
    let Some(mut t) = twin(N) else { return };
    let e = t.at(0x2240);
    let h = t.img.word(e + 0x01);
    let alt = t.img.word(e + 0x99);
    let model = SCRATCH + 0x1000;
    t.set_u32(model + 0x830, 0x830);
    rec(&mut t, 26, 2, |_, a| a[0] ^ 0x26);
    rec(&mut t, 0, 3, move |_, _| model);
    for (s, n) in [(387u32, 5usize), (385, 7), (386, 9), (922, 11)] {
        rec(&mut t, s, n, move |_, a| a[0] ^ s);
    }
    for (first, a) in [
        (0u32, 0u32),
        (0, 1),
        (SCRATCH + 0x2000, 0),
        (SCRATCH + 0x2000, 1),
    ] {
        t.set_u32(h, first);
        t.set_u32(alt, a);
        // the words it sets, dirty before
        for op in [0xc8u32, 0xd2, 0xdc, 0xe6, 0x104, 0x129] {
            let g = t.img.word(e + op);
            t.write(g, &[0x5a; 8]);
        }
        t.check_entry(e, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10003fc0: the motion sequence at the end of its list, starting an
/// entry (slot 18 answering a motion, or -1 skipping to the end), with
/// the entry not yet over and over (by its frame or, with frame 0, the
/// playing motion), into the end of the list or the next entry, with and
/// without a callback.
#[test]
fn motion_sequence() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3fc0);
    rec(&mut t, 37, 3, |_, a| 0x0370_0000 ^ a[2]);
    rec(&mut t, 30, 1, |_, _| 0x30);
    let (play, frame) = (SCRATCH + 0x600, SCRATCH + 0x604);
    rec(&mut t, 18, 1, move |m, _| m.r32(play));
    rec(&mut t, 22, 1, move |m, _| m.r32(frame));
    let cb = 1001u32;
    t.set_u32(magic::TABLE + 4 * cb, magic::TRAPS + 16 * cb);
    rec(&mut t, cb, 1, |_, a| 0x0cb0_0000 ^ a[0]);
    let (seq, entries) = (SCRATCH + 0x100, SCRATCH + 0x200);
    // (index, playing word, slot 18, frame from slot 22, entry 0's frame)
    for (case, (index, playing, s18, now, fr0, callback)) in [
        (3u32, 0u32, 5u32, 0u32, 0u32, true),
        (0, 0, 7, 0x0600, 0, true),
        (0, 0, u32::MAX, 0x0600, 0, false),
        (0, 9, 7, 0x0600, 7, true),
        (0, 9, 7, 0x0600, 0, true),
        (0, 9, 7, 0x0800, 0, false),
        (1, 9, 7, 0x0400, 5, true),
        (2, 9, 7, 0x0400, 5, true),
        (1, 9, 7, 0x0400, 0, false),
        (2, 9, 7, 0x0500, 5, true),
        (0, 0, 8, 0x07ff, 0, true),
    ]
    .into_iter()
    .enumerate()
    {
        t.write(entries, &[0; 0x50]);
        for i in 0..3u32 {
            let e = entries + 16 * i;
            t.set_u32(e, 0x0010_0000 + i);
            t.set_u32(e + 4, if i == 0 { fr0 } else { 4 + i });
            if callback {
                t.set_u32(e + 8, magic::TRAPS + 16 * cb);
            }
            t.set_u32(e + 0xc, 0x00a0_0000 + i);
        }
        t.set_u32(seq, entries);
        t.set_u32(seq + 4, 0x0040_0000 + u32::try_from(case).unwrap());
        t.set_u32(seq + 8, index);
        t.set_u32(seq + 0xc, playing);
        t.set_u32(play, s18);
        t.set_u32(frame, now);
        t.check(at, Call::this(seq), &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0098::motion_sequence(g, img, fr.at, fr.ecx))
        });
    }
}

/// 0x10003340: off; on with slot 253 saying 0 or not, each timeline
/// pair live or not (the stop and slot 214, or the state 5 or not).
#[test]
fn frame_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3340);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let s253 = SCRATCH + 0x600;
    rec(&mut t, 253, 0, move |m, _| m.r32(s253));
    for (s, n) in [(723u32, 1usize), (724, 2), (214, 1), (393, 3)] {
        rec(&mut t, s, n, move |_, a| s ^ a[0]);
    }
    for (op, v) in [
        (0x0d, SCRATCH + 0x300),
        (0x34, SCRATCH + 0x304),
        (0x66, SCRATCH + 0x308),
    ] {
        t.set_u32(w(&t, op), v);
    }
    for (k, op) in (0u32..).zip([0x3a, 0x58, 0x6b, 0xae, 0xf5]) {
        t.set_u32(w(&t, op), 0x0a00_0000 + k);
    }
    let timeline = |t: &mut magic::Twin, tl: u32, live: bool| {
        t.set_u32(tl, 5);
        t.set_u32(tl + 4, 1);
        t.set_u32(tl + 8, u32::from(live));
        t.set_u32(tl + 0xc, u32::from(live));
        t.set_u32(tl + 0x10c, SCRATCH + 0x5000);
        t.write(SCRATCH + 0x5000, &[0; 0x18]);
        t.set_u32(SCRATCH + 0x5004, 9);
    };
    let base = t.save();
    for (on, r253, sel, live, state) in [
        (0u32, 0u32, 0u32, false, 0u32),
        (1, 0, 0, false, 0),
        (1, 3, 1, false, 0),
        (1, 0, 0, true, 5),
        (1, 0, 1, true, 5),
        (1, 3, 1, true, 4),
        (1, 0, 0, true, 6),
    ] {
        t.restore(&base);
        t.set_u32(w(&t, 0x02), on);
        t.set_u32(s253, r253);
        t.set_u32(w(&t, 0x74), sel);
        t.set_u32(w(&t, 0x97), sel);
        t.set_u32(w(&t, 0xc5), sel);
        t.set_u32(SCRATCH + 0x300, 0x77);
        t.set_u32(w(&t, 0x100), 0x0100_0010);
        // the timeline not chosen is the opposite (the stop shows which)
        for (op, chosen) in [(0x7c, sel != 0), (0x83, sel == 0)] {
            let tl = w(&t, op);
            timeline(&mut t, tl, live == chosen);
        }
        t.set_u32(w(&t, 0xce), state);
        t.set_u32(w(&t, 0xd7), state ^ (sel << 4));
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002730: the parts 0, 1 and another, and a part below the base.
#[test]
fn part_callback() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2730);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let table = SCRATCH + 0x400;
    for k in 0..4 {
        t.set_u32(table + 4 * k, 0x0f00_0000 + k);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, _| 0x714);
    rec(&mut t, 237, 8, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x3400 + 4 * i, v);
        }
        0x237
    });
    t.set_u32(w(&t, 0x3f), SCRATCH + 0x300);
    t.set_u32(w(&t, 0x4a), SCRATCH + 0x380);
    t.set_u32(SCRATCH + 0x390, 0x00ab_0010);
    t.set_u32(w(&t, 0xdb), 0x00db_0000);
    let base = w(&t, 0x17);
    for part in [base, base + 0x80, base + 0x100, base - 0x80] {
        t.write(part, &[0x3c; 0x80]);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[part], Eax::Checked);
    }
}

/// 0x10001590: the height follow with the target's height, the matrix
/// rows and slot 268's vector chosen so `|c.y - a.y|` falls below,
/// at and above the limit (and huge).
#[test]
fn height_follow() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1590);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let id = SCRATCH + 0x100;
    rec(&mut t, 390, 1, move |m, _| m.r32(id));
    rec(&mut t, 215, 2, |_, a| a[0] ^ 0x215);
    rec(&mut t, 19, 1, |_, a| a[0] ^ 0x19);
    t.stub_st0(19, |_, a| f64::from(a[0] & 0xff) * 0.37 + 0.1);
    t.set_u32(t.img.word(t.at(0x11a0) + 0x0c), 0x44);
    // slot 210 fills the matrix (16 floats), 325 copies 4 words, 232
    // scales its vector by 0.5, 268 fills c from SCRATCH + 0x700
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            #[allow(clippy::cast_precision_loss)]
            m.w32(a[1] + 4 * i, (1.0 + i as f32 * 0.75).to_bits());
        }
        0x210
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) * 0.5;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x232
    });
    rec(&mut t, 364, 1, |_, _| 0x364);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(SCRATCH + 0x700 + 4 * i);
            m.w32(a[2] + 4 * i, v);
        }
        0x268
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..3 {
            let v = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x3800 + 4 * i, v);
        }
        0x248
    });
    t.set_u32(w(&t, 0x16), 0x00e0_0000);
    let g = w(&t, 0x168);
    // a.y is the matrix's row 3 y: 1.0 + 13 * 0.75
    let ay = 1.0 + 13.0 * 0.75;
    let k4 = f32::from_bits(t.u32(w(&t, 0x14d)));
    for (case, dy) in [0.0f32, k4 - 1.0, k4, k4 + 2.5, -k4 - 0.5, 3.0e9]
        .into_iter()
        .enumerate()
    {
        t.set_u32(id, [0x1015u32, 0x1077, 0x1000, 0x1127, 0, 9][case]);
        t.set_u32(SCRATCH + 0x704, (ay + dy).to_bits());
        t.set_u32(SCRATCH + 0x700, 0x4000_0000);
        t.set_u32(
            g,
            (7.5f32 + f32::from(u8::try_from(case).unwrap())).to_bits(),
        );
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10002870: the parts 0, 1 (the alpha at 0x80, inside, below 12 and
/// huge) and another; slot 712 saying 0 or not.
#[test]
fn part_draw() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2870);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let lock = SCRATCH + 0x200;
    t.set_u32(w(&t, 0x115), lock);
    t.set_u32(w(&t, 0x108), SCRATCH + 0x204);
    let r712 = SCRATCH + 0x208;
    for (s, n) in [
        (237u32, 4usize),
        (393, 3),
        (391, 4),
        (258, 1),
        (231, 3),
        (230, 2),
    ] {
        rec(&mut t, s, n, move |_, a| s ^ a[0]);
    }
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x3400 + 4 * i, v);
        }
        0x237
    });
    rec(&mut t, 712, 2, move |m, _| {
        m.w32(SCRATCH + 0x20c, m.r32(lock));
        m.r32(r712)
    });
    for (k, op) in (0u32..).zip([0x50, 0x84, 0xd1, 0xdc, 0xe8, 0xf0, 0xfb]) {
        t.set_u32(w(&t, op), 0x0a00_0000 + k);
    }
    let base = w(&t, 0x18);
    for (part, alpha, r) in [
        (base, 0u32, 0u32),
        (base + 0x80, 0x80, 1),
        (base + 0x80, 0x40, 0),
        (base + 0x80, 5, 0),
        (base + 0x80, 0x7fff_fff0, 2),
        (base + 0x80, 0x8000_0005, 0),
        (base + 0x100, 0, 0),
    ] {
        t.write(part, &[0x3c; 0x80]);
        t.set_u32(w(&t, 0x62), alpha);
        t.set_u32(r712, r);
        t.set_u32(lock, 7);
        t.check_entry(at, Call::default(), &[part], Eax::Checked);
    }
}

/// 0x100013c0: the tilt for a few dot products (0, inside, near 1).
#[test]
fn tilt() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x13c0);
    filler(&mut t, 325, 2, 4);
    rec(&mut t, 232, 4, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ 0x0010_0000;
            m.w32(a[0] + 4 * i, v);
        }
        0x232
    });
    rec(&mut t, 266, 7, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x266
    });
    // slot 229 transforms: the vector's words into the output, scrambled
    rec(&mut t, 229, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    let dot = SCRATCH + 0x600;
    rec(&mut t, 394, 2, |_, a| a[0] ^ 0x394);
    t.stub_st0(394, move |m, _| f64::from(f32::from_bits(m.r32(dot))));
    for x in [0.0f32, 0.25, -0.5, 0.999, -0.03] {
        t.set_u32(dot, x.to_bits());
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001730: the pair follow with the target's height, the matrix
/// rows and slot 268's vector chosen so `|c.y - a.y|` falls below,
/// at and above the limit (and huge).
#[test]
fn pair_follow() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1730);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let id = SCRATCH + 0x100;
    rec(&mut t, 390, 1, move |m, _| m.r32(id));
    rec(&mut t, 215, 2, |_, a| a[0] ^ 0x215);
    rec(&mut t, 19, 1, |_, a| a[0] ^ 0x19);
    t.stub_st0(19, |_, a| f64::from(a[0] & 0xff) * 0.37 + 0.1);
    t.set_u32(t.img.word(t.at(0x11a0) + 0x0c), 0x44);
    // slot 210 fills the matrix (16 floats), 325 copies 4 words, 232
    // scales its vector by 0.5, 268 fills c from SCRATCH + 0x700
    rec(&mut t, 210, 2, |m, a| {
        for i in 0..16 {
            #[allow(clippy::cast_precision_loss)]
            m.w32(a[1] + 4 * i, (1.0 + i as f32 * 0.75).to_bits());
        }
        0x210
    });
    rec(&mut t, 325, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x325
    });
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = f32::from_bits(m.r32(a[1] + 4 * i)) * 0.5;
            m.w32(a[0] + 4 * i, v.to_bits());
        }
        0x232
    });
    rec(&mut t, 364, 1, |_, _| 0x364);
    rec(&mut t, 268, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(SCRATCH + 0x700 + 4 * i);
            m.w32(a[2] + 4 * i, v);
        }
        0x268
    });
    rec(&mut t, 248, 3, |m, a| {
        for i in 0..3 {
            let v = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x3800 + 4 * i, v);
        }
        0x248
    });
    t.set_u32(w(&t, 0x16), 0x00e0_0000);
    let g = w(&t, 0x177);
    rec(&mut t, 217, 2, |m, a| {
        for i in 0..4 {
            m.w32(a[1] + 4 * i, a[0] ^ (0x0300_0000 + i));
        }
        0x217
    });
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v & 0x3fff_ffff);
        }
        0x234
    });
    t.set_u32(w(&t, 0x4f), 0x0044_0000);
    t.set_u32(w(&t, 0x66), 0x0066_0000);
    t.set_u32(w(&t, 0xa6), 1.25f32.to_bits());
    // a.y is the matrix's row 3 y: 1.0 + 13 * 0.75
    let ay = 1.0 + 13.0 * 0.75;
    let k4 = f32::from_bits(t.u32(w(&t, 0x152)));
    for (case, dy) in [0.0f32, k4 - 1.0, k4, k4 + 2.5, -k4 - 0.5, 3.0e9]
        .into_iter()
        .enumerate()
    {
        t.set_u32(id, [0x1015u32, 0x1077, 0x1000, 0x1127, 0, 9][case]);
        t.set_u32(SCRATCH + 0x704, (ay + dy).to_bits());
        t.set_u32(SCRATCH + 0x700, 0x4000_0000);
        t.set_u32(
            g,
            (7.5f32 + f32::from(u8::try_from(case).unwrap())).to_bits(),
        );
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x10001f10: rising and falling, inside, past each limit, at the upper
/// limit, NaN, with the clock past its mark or not (both clocks).
#[test]
fn fade_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1f10);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(353u32, 1usize), (801, 6), (785, 1), (397, 2)] {
        rec(&mut t, s, n, move |_, a| s ^ a[0]);
    }
    rec(&mut t, 353, 1, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[0] + 4 * i);
            m.w32(SCRATCH + 0x3400 + 4 * i, v);
        }
        0x353
    });
    rec(&mut t, 397, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[0] + 4 * i);
            m.w32(SCRATCH + 0x3440 + 4 * i, v ^ a[1]);
        }
        0x397
    });
    let fl = |t: &magic::Twin, off: u32| f32::from_bits(t.u32(w(t, off)));
    let (k1, k2, d, d2) = (fl(&t, 0x38), fl(&t, 0x57), fl(&t, 0x42), fl(&t, 0x97));
    t.set_u32(w(&t, 0x12d), 0.5f32.to_bits());
    t.set_u32(w(&t, 0x16d), 3.0f32.to_bits());
    for (sel, clock, level) in [
        (1u32, 0xb0u32, k1.midpoint(k2)),
        (1, 0xaf, k1.midpoint(k2)),
        (0, 0x51, k2 - d),
        (0, 0x51, k2 - d * 0.5),
        (0, 0x51, k2 + d),
        (0, 0x50, k1 + d2 * 0.5),
        (1, 0x00, k2 + d2 * 2.0),
        (1, 0xb0, k1 - d * 2.0),
        (0, 0x60, f32::NAN),
        (0, 0x10, f32::NAN),
        (1, 0xb0, k1),
        (1, 0xb0, k1 - d),
        (1, 0xb0, k2 - d),
        (0, 0x10, k1 + d2),
        (0, 0x10, k2 + d2),
    ] {
        t.set_u32(w(&t, 0x12), sel);
        t.set_u32(w(&t, 0x1b), clock);
        t.set_u32(w(&t, 0x27), clock);
        t.set_u32(w(&t, 0x30), level.to_bits());
        // the second level (a global): above, at and below K1 by case
        t.set_u32(
            w(&t, 0x105),
            (k1 + (clock & 3) as f32 * 0.25 - 0.25).to_bits(),
        );
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100018e0 (internal, `edx` = the scales): the camera matrix for two
/// sets of scales.
#[test]
fn camera_matrix() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x18e0);
    filler(&mut t, 235, 1, 16);
    rec(&mut t, 234, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v & 0x3fff_ffff);
        }
        0x234
    });
    rec(&mut t, 232, 2, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ 0x0010_0000;
            m.w32(a[0] + 4 * i, v);
        }
        0x232
    });
    rec(&mut t, 266, 3, |m, a| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i) ^ 0x0100;
            m.w32(a[0] + 4 * i, v);
        }
        0x266
    });
    // slot 231 combines its sources into its output (each call's inputs
    // stay visible in the global matrix)
    rec(&mut t, 231, 3, |m, a| {
        let v: Vec<u32> = (0..16)
            .map(|i| m.r32(a[2] + 4 * i) ^ m.r32(a[1] + 4 * i).rotate_left(7))
            .collect();
        for (i, x) in (0u32..).zip(v) {
            m.w32(a[0] + 4 * i, x);
        }
        0x231
    });
    rec(&mut t, 395, 3, |_, _| 0x395);
    let scales = SCRATCH + 0x700;
    for k in [0u32, 1] {
        for i in 0..3 {
            t.set_u32(scales + 4 * i, f(0.5 + (k * 3 + i) as f32));
        }
        let c = Call {
            edx: scales,
            ..Call::default()
        };
        dirty(&mut t);
        t.check(at, c, &[], Eax::Checked, |g, img, fr| {
            Ret::eax(m0098::camera_matrix(g, img, fr.at, fr.edx, fr.esp))
        });
    }
}

/// 0x100023d0: a part's start with each of the two handles.
#[test]
fn part_begin() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x23d0);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let table = SCRATCH + 0x400;
    for k in 0..4 {
        t.set_u32(table + 4 * k, 0x0f00_0000 + k);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    rec(&mut t, 714, 5, |_, _| 0x714);
    for (s, n) in [(388u32, 3usize), (393, 3)] {
        rec(&mut t, s, n, move |_, a| s ^ a[0]);
    }
    rec(&mut t, 395, 3, |m, a| {
        for i in 0..16 {
            let v = m.r32(a[1] + 4 * i);
            m.w32(SCRATCH + 0x3900 + 4 * i, v);
        }
        for i in 0..4 {
            let v = m.r32(a[2] + 4 * i);
            m.w32(SCRATCH + 0x3980 + 4 * i, v);
        }
        0x395
    });
    filler(&mut t, 235, 1, 16);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, f(1.0 + i as f32));
        }
        0x237
    });
    rec(&mut t, 236, 3, |m, a| {
        for i in 0..16 {
            m.w32(a[2] + 4 * i, f(0.25 * i as f32));
        }
        0x236
    });
    // the matrix and vector slots combine their sources into the output
    for (s, n) in [(231u32, 3usize), (229, 3), (248, 3), (325, 2)] {
        rec(&mut t, s, n, move |m, a| {
            let v: Vec<u32> = (0..16)
                .map(|i| m.r32(a[n - 1] + 4 * i) ^ m.r32(a[1] + 4 * i).rotate_left(3) ^ (s << 8))
                .collect();
            let len = if s == 231 { 16 } else { 4 };
            for (i, x) in (0u32..).zip(v).take(len) {
                m.w32(a[0] + 4 * i, x);
            }
            s
        });
    }
    t.set_u32(w(&t, 0x14), SCRATCH + 0x300);
    t.set_u32(w(&t, 0x23), SCRATCH + 0x380);
    t.set_u32(SCRATCH + 0x390, 0x00ab_0010);
    for (k, op) in (0u32..).zip([0x55, 0xca, 0xf9, 0x265, 0x26d]) {
        t.set_u32(w(&t, op), 0x0a00_0000 + k);
    }
    let part = SCRATCH + 0x1000;
    for sel in [0u32, 1] {
        t.write(part, &[0x3c; 0x80]);
        t.set_u32(w(&t, 0x24b), sel);
        dirty(&mut t);
        t.check_entry(at, Call::default(), &[part], Eax::Checked);
    }
}
