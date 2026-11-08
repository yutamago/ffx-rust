//! `magic_0535.dll` against its Rust port (`ffx_magic::m0535`). Needs
//! `FFX_GAME_DIR`.

#![allow(clippy::cast_precision_loss)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, recs, twin};

const N: u16 = 535;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10001000: the framing step for 0, 1 and 3 actors (heights and
/// distances rising and not, the bound larger and smaller). Also
/// `magic_0384`'s copy (kind 0x4a, `V[0]` -4.0, one more range word).
#[test]
fn frame_step() {
    for n in [N, 384] {
        frame_step_in(n);
    }
}

fn frame_step_in(dll: u16) {
    let Some(mut t) = twin(dll) else { return };
    let at = t.at(0x1000);
    let s = SCRATCH + 0x2000;
    t.write(s, &[0x2b; 0x200]);
    t.set_u32(t.img.word(at + 0x2d), s);
    let cell = SCRATCH + 0x10;
    // the float slots: 515 the centre, 518/516/517 by actor, 514 the
    // distance (its two words' sum)
    for slot in [515u32, 518, 516, 517, 514] {
        let words = match slot {
            515 => 0,
            514 => 2,
            _ => 1,
        };
        rec(&mut t, slot, words, move |_, _| slot);
        t.stub_st0(slot, move |m, a| {
            let base = f64::from(f32::from_bits(m.r32(cell)));
            match slot {
                515 => base,
                514 => f64::from(f32::from_bits(a[0])) + f64::from(f32::from_bits(a[1])),
                _ => base * f64::from(a[0] + 1) + f64::from(slot - 513),
            }
        });
    }
    rec(&mut t, 474, 4, |m, a| {
        for i in 0..4u32 {
            // `V`'s words show in the result
            let v = m.r32(a[2] + 4 * i);
            m.w32(a[3] + 4 * i, (1.5 + i as f32).to_bits() ^ v);
        }
        0x474
    });
    recs(&mut t, &[(519, 3), (513, 3), (450, 2), (449, 2)]);
    let blk = t.img.word(at + 0x1cf);
    let seq = SCRATCH + 0x1000;
    for (n, base) in [(0u32, 1.5f32), (1, -2.0), (3, 0.75), (3, 40.0)] {
        t.set_u32(s + 0x19c, n);
        for i in 0..4u32 {
            t.set_u32(s + 0x1a0 + 4 * i, 0x00a0_0000 + i);
        }
        t.set_u32(cell, base.to_bits());
        t.write(blk, &[0x5b; 0x60]);
        // the range words after the block (the copy clears one more)
        let last = if dll == 384 { 0x203 } else { 0x1f9 };
        for o in [0x1e5u32, 0x1ef, 0x1f9, last] {
            t.set_u32(t.img.word(at + o), 0x7777_0000 + o);
        }
        t.write(seq, &[0x3c; 0x40]);
        t.check_entry(at, Call::default(), &[seq], Eax::Checked);
    }
}
