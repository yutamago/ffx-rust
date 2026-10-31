//! `magic_0082.dll` against its Rust port (`ffx_magic::m0082`): the
//! camera step. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 82;

/// 0x100068e0: each mode, the clock below, at and past its limits (and
/// NaN), the camera's points junk before the move.
#[test]
fn camera_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x68e0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 239, 3, |_, a| 0x0239_0000 ^ a[0]);
    t.set_u32(w(&t, 0xd4), 0x00d4_0000);
    let k = f32::from_bits(t.u32(w(&t, 0x16)));
    let k0 = f32::from_bits(t.u32(w(&t, 0xbd)));
    let k3 = f32::from_bits(t.u32(w(&t, 0x3d)));
    // the camera's points (move_start_2's operands) hold junk
    let ms = t.img.rel32_target(at + 0x99);
    let cam = t.img.word(ms + 0x04) - 0x20;
    for i in 0..0x30 {
        #[allow(clippy::cast_precision_loss)]
        t.set_u32(cam + 4 * i, (0.5 + i as f32 * 0.25).to_bits());
    }
    let (task, s, ring) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x200);
    let base = t.save();
    for (mode, clock) in [
        (0u16, 0.0f32),
        (0, k0 - k),
        (0, k0),
        (0, f32::NAN),
        (1, 0.0),
        (1, k3 - k),
        (1, k3 + 3.0),
        (1, f32::NAN),
        (2, 0.0),
        (7, 1.0),
    ] {
        t.restore(&base);
        t.write(s + 0x560, &[0x77; 0x20]);
        t.set_u32(s + 0x568, clock.to_bits());
        t.set_u16(s + 0x56c, mode);
        t.write(ring, &[0x66; 0x60]);
        t.set_u32(task, ring + 0x20);
        t.set_u32(task + 4, s);
        t.set_u32(ring + 0x24, ring);
        t.set_u32(ring + 0x28, ring + 0x40);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x100069d0: each mode, the clock below, at and past its limits, mode
/// 0 building the nodes into empty and filled lists.
#[test]
#[allow(clippy::many_single_char_names)]
fn camera_parts_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x69d0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 239, 3, |_, a| 0x0239_0000 ^ a[0]);
    rec(&mut t, 268, 3, |m, a| {
        m.w32(a[2], 0x3f40_0000);
        0x268
    });
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) + 0.25);
    t.stub(941, |_, _| SCRATCH + 0x400);
    for k in 0..4 {
        t.set_u32(SCRATCH + 0x400 + 4 * k, 0x0f00_0000 + k);
    }
    rec(&mut t, 714, 5, |_, _| 0x714);
    rec(&mut t, 325, 2, |_, _| 0x325);
    let pn = t.img.rel32_target(at + 0x1da);
    t.set_u32(t.img.word(pn + 0x45), SCRATCH + 0x700);
    t.set_u32(w(&t, 0x1a1), SCRATCH + 0x680);
    t.write(SCRATCH + 0x680, &[0x31; 0x40]);
    for op in [0xd4, 0xfb, 0x112, 0x1ef] {
        t.set_u32(w(&t, op), 0x00ab_0000 + op);
    }
    let k = f32::from_bits(t.u32(w(&t, 0x1d)));
    let k0 = f32::from_bits(t.u32(w(&t, 0xc9)));
    let k3 = f32::from_bits(t.u32(w(&t, 0x45)));
    let (task, s, ring) = (SCRATCH + 0x100, SCRATCH + 0x1000, SCRATCH + 0x200);
    let other = SCRATCH + 0x800;
    let base = t.save();
    for (mode, clock, filled) in [
        (0u16, 0.0f32, false),
        (0, k0 - k, false),
        (0, k0 + 1.0, true),
        (0, f32::NAN, false),
        (1, 0.0, false),
        (1, k3, false),
        (3, 0.0, false),
    ] {
        t.restore(&base);
        t.write(s, &[0x77; 0x600]);
        t.set_u32(s + 0x568, clock.to_bits());
        t.set_u16(s + 0x56c, mode);
        for op in [0xda, 0x138, 0x1bd] {
            let l = w(&t, op);
            t.set_u32(l + 0x10, if filled { other } else { l });
        }
        t.write(other, &[0x66; 0x20]);
        t.write(ring, &[0x66; 0x60]);
        t.set_u32(task, ring + 0x20);
        t.set_u32(task + 4, s);
        t.set_u32(ring + 0x24, ring);
        t.set_u32(ring + 0x28, ring + 0x40);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
