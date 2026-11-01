//! `magic_0431.dll` against its Rust ports (`ffx_magic::m0431`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 431;

/// 0x10001620: each of the four answers not 0 in turn, and all 0.
#[test]
fn sounds_ready() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1620);
    let answers = SCRATCH + 0x600;
    for (s, n) in [(5u32, 1usize), (28, 2)] {
        rec(&mut t, s, n, move |m, _| {
            // the k-th call (both slots counted together) answers word k
            let k = m.r32(answers + 0x20);
            m.w32(answers + 0x20, k + 1);
            m.r32(answers + 4 * k)
        });
    }
    for bad in [0u32, 1, 2, 3, 4] {
        for k in 0..4 {
            t.set_u32(answers + 4 * k, u32::from(k == bad) * (0x10 + k));
        }
        t.set_u32(answers + 0x20, 0);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0431::sounds_ready,
        );
    }
}

/// 0x10001a10: the colour reset for two actors.
#[test]
fn colour_reset() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a10);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(357u32, 3usize), (356, 3), (362, 4)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let b = SCRATCH + 0x700;
    t.set_u32(w(&t, 0x01), b);
    t.set_u32(w(&t, 0x4c), 0x00ab_004c);
    for i in 0..4 {
        t.set_u32(w(&t, 0x11) + 4 * i, 0x0a00_0000 + i);
    }
    for i in [0u8, 3] {
        t.set_u8(b, i);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0431::colour_reset,
        );
    }
}

/// 0x10001760 (internal): the camera's angle above, inside and below the
/// range (and NaN).
#[test]
fn camera_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1760);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 262, 4, |m, a| {
        for (i, at) in (0u32..).zip(&a[1..4]) {
            m.w32(*at, 0x0262_0000 + i);
        }
        0x262
    });
    let angle = SCRATCH + 0x600;
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    t.stub_st0(333, move |m, _| f64::from(f32::from_bits(m.r32(angle))));
    let fx = SCRATCH + 0x8000;
    t.set_u32(w(&t, 0x08), fx);
    t.set_u32(w(&t, 0x22), 0x00ab_0022);
    t.set_u32(w(&t, 0x3c), 0x00ab_003c);
    let base = f32::from_bits(t.u32(w(&t, 0x4f)));
    let hi = f32::from_bits(t.u32(w(&t, 0x5e)));
    let lo = f32::from_bits(t.u32(w(&t, 0x72)));
    let mut angles = vec![base, base - hi - 1.0, base - lo + 1.0, f32::NAN];
    for bound in [hi, lo] {
        let guess = (base - bound).to_bits();
        if let Some(r) = (0..256u32)
            .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
            .map(f32::from_bits)
            .find(|&r| (base - r).to_bits() == bound.to_bits())
        {
            angles.push(r);
        }
    }
    for a in angles {
        t.set_u32(angle, a.to_bits());
        t.write(fx + 0x810, &[0x33; 0x20]);
        t.write(fx + 0xc60, &[0x55; 0xc0]);
        t.set_u32(w(&t, 0xac), 7);
        t.set_u32(w(&t, 0xb6), 7);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0431::camera_start,
        );
    }
}

/// 0x10001830 (internal): the actors' show and the sounds, then the
/// parts' registration (one part, no banks).
#[test]
fn show_and_sounds() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1830);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 384, 1, |_, a| 0x0384_0000 ^ a[0]);
    for s in [25u32, 24] {
        rec(&mut t, s, 2, move |_, a| {
            (0x0100_0000 + s) ^ a[0] ^ (a[1] << 4)
        });
    }
    rec(&mut t, 941, 1, |_, a| a[0] ^ 0x0941_0000);
    rec(&mut t, 772, 2, |_, a| a[0] ^ 0x0772_0000);
    t.stub_edx(772, |_, a| a[1] ^ 0x0edd_0000);
    rec(&mut t, 779, 4, |_, a| a[3] ^ 0x779);
    let b = SCRATCH + 0x700;
    t.set_u32(w(&t, 0x01), b);
    for i in 0..8 {
        t.set_u32(w(&t, 0x0c) + 4 * i, 0x0a00_0000 + i);
    }
    let rp = t.img.rel32_target(at + 0xbc);
    let parts = SCRATCH + 0xc000;
    t.set_u32(t.img.word(rp + 0x16), parts);
    t.set_u32(parts + 0x30, 0x0a30);
    let lock = SCRATCH + 0x5e0;
    t.set_u32(t.img.word(rp + 0x43), lock);
    t.set_u32(t.img.word(rp + 0x5f), lock);
    t.set_u32(t.img.word(rp + 0x76), 0);
    t.set_u32(t.img.word(rp + 0x0e), 1);
    for bytes in [[0u8, 1, 2, 3, 4], [9, 7, 0, 5, 6]] {
        t.write(b, &bytes);
        t.set_u32(lock, 0x77);
        t.check(
            at,
            Call::default(),
            &[],
            Eax::Checked,
            ffx_magic::m0431::show_and_sounds,
        );
    }
}
