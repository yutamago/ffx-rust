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
