//! `magic_0686.dll` against its Rust port (`ffx_magic::m0686`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_h6;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax};
use magic_owna::{rec, recs, twin};

const N: u16 = 686;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10002020: the model placed, the motion script stepped (no record:
/// its done word set), `G_DONE` 0 and not.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2020);
    rec(&mut t, 235, 1, |m, a| {
        for i in 0..16 {
            m.w32(a[0] + 4 * i, 0x3f80_0000 + i);
        }
        0x235
    });
    recs(&mut t, &[(231, 3), (211, 2)]);
    t.set_u32(t.img.word(at + 0x5d), 0x005d_0001);
    // the step's record table: the current record's motion 0 (it stops)
    let step = t.img.rel32_target(at + 0x6d);
    let tab = magic::SCRATCH + 0x4000;
    t.set_u32(t.img.word(step + 0xb), tab);
    t.set_u32(t.img.word(step + 0x2), 1);
    t.write(tab, &[0; 0x40]);
    for done in [0u32, 1] {
        t.set_u32(t.img.word(at + 0x78), done);
        magic_h6::scrub(&mut t);
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100020c0: the start-up (`r15::start_up_0686`) with its light set-up
/// (0x10001290): every slot answers per call, the matrix and vectors the
/// slots fill differ by call, the flag word restored, the block cleared
/// (non-zero before).
#[test]
fn start_up() {
    start_up_in(N, 0x20c0);
    // `magic_0443`'s copy (0x10001f10): its own light set-up (the scale
    // first) and nine more block words
    start_up_in(443, 0x1f10);
}

fn start_up_in(n: u16, rva: u32) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let w = |t: &magic::Twin, o: u32| t.img.word(at + o);
    let rec_tab = magic::SCRATCH + 0x5000;
    for i in 0..16 {
        t.set_u32(rec_tab + 4 * i, 0x0e00_0000 + i);
    }
    rec(&mut t, 0xc8c / 4, 2, |_, _| 0x2100_0000);
    rec(&mut t, 0xb3c / 4, 3, move |_, a| rec_tab + 4 * (a[2] & 3));
    rec(&mut t, 0xeb4 / 4, 1, |_, a| 0x0eb4_0000 ^ a[0]);
    rec(&mut t, 0x2a0 / 4, 1, |_, a| 0x02a0_0000 ^ a[0]);
    rec(&mut t, 0x35c / 4, 1, |_, a| 0x035c_0000 ^ a[0]);
    rec(&mut t, 0x670 / 4, 3, |_, a| 0x0670_0000 ^ a[1]);
    // slot 711 sees the flag word set to 1
    let flag = magic::SCRATCH + 0x5100;
    rec(&mut t, 0xb1c / 4, 3, move |m, a| {
        // (kept where the test compares)
        let v = m.r32(flag);
        m.w32(magic::SCRATCH + 0x5104, v);
        v ^ a[0]
    });
    recs(
        &mut t,
        &[
            (0xb38 / 4, 1),
            (0x378 / 4, 4),
            (0x374 / 4, 4),
            (0xb18 / 4, 4),
            (0x10 / 4, 1),
            (0x6c / 4, 2),
            (0x514 / 4, 2),
        ],
    );
    // slot 230 reads the light set-up's matrix (its last row the vector
    // slot 248 made, less K2)
    rec(&mut t, 0x398 / 4, 2, |m, a| {
        magic_owna::dump(m, 0x398 / 4, a[1], 16);
        0x398
    });
    // the light set-up's slots: the matrix, its rows, the point and the vector
    rec(&mut t, 0x348 / 4, 2, |m, a| {
        for i in 0..16 {
            m.w32(a[1] + 4 * i, (0.5 + i as f32 * 0.375).to_bits());
        }
        0x348
    });
    rec(&mut t, 0x3a0 / 4, 2, |m, a| {
        for i in 0..4 {
            let x = f32::from_bits(m.r32(a[1] + 4 * i));
            m.w32(a[0] + 4 * i, (x * 0.25 - 1.0).to_bits());
        }
        0x3a0
    });
    rec(&mut t, 0x4c / 4, 1, |_, a| a[0]);
    t.stub_st0(0x4c / 4, |_, _| 2.75);
    rec(&mut t, 0x3e0 / 4, 3, |m, a| {
        // (the point is three floats: its fourth word is never written)
        for i in 0..3 {
            let x = f32::from_bits(m.r32(a[2] + 4 * i));
            m.w32(a[0] + 4 * i, (x + 1.5).to_bits());
        }
        m.w32(a[0] + 12, 1.0f32.to_bits());
        0x3e0
    });
    let ev = t.img.rel32_target(at + 0x1fd);
    t.set_u32(t.img.word(ev + 0x13), 0x0e0e_0001);
    t.set_u32(w(&t, 0xf1), flag);
    t.set_u32(flag, 0x5a5a_0007);
    t.write(w(&t, 0x209), &[0x5a; 0x110]);
    // the globals the start-up sets: other values before
    for op in [
        0x19u32, 0x23, 0x2d, 0x37, 0x3f, 0x55, 0xb5, 0xd4, 0xfa, 0x157, 0x171, 0x18b, 0x1c6, 0x1d9,
        0x1f8, 0x217,
    ] {
        t.set_u32(w(&t, op), 0x0bad_0000 + op);
    }
    magic_h6::scrub(&mut t);
    t.check_entry(at, Call::default(), &[0x0a86_0001], Eax::Checked);
}
