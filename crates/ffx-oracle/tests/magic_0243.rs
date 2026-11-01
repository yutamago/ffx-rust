//! `magic_0243.dll` against its Rust ports (`ffx_magic::m0243`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{f, rec, twin};

const N: u16 = 243;

/// The floats slot 333 answers, in turn (the angle `K - f` then lands
/// above, inside and below the wrap's range, and on NaN).
const ANGLES: [f32; 7] = [-9.0, 0.5, 3.5, 12.0, -0.25, f32::NAN, 6.5];

/// The floats that put `K - f` exactly on the wrap's bounds (`K`, `HI`,
/// `LO` at the operands given), for slot 333 to answer. `LO` (-pi with
/// `K` 3pi/2) is out of reach: the difference of two floats in `[4, 8)`
/// is a multiple of 2^-21 and pi's last bit is 2^-22, so `>` and `>=`
/// there are the same.
fn on_bounds(t: &Twin, at: u32, (k, hi, lo): (u32, u32, u32)) -> Vec<f32> {
    let fl = |op: u32| f32::from_bits(t.u32(t.img.word(at + op)));
    let exact = |b: f32| {
        let guess = (fl(k) - b).to_bits();
        (0..256u32)
            .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
            .map(f32::from_bits)
            .find(|&r| (fl(k) - r).to_bits() == b.to_bits())
    };
    [exact(fl(hi)), exact(fl(lo))]
        .into_iter()
        .flatten()
        .collect()
}

/// 0x10001d50: the parts' facing for no, one and several parts.
#[test]
fn facing() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1d50);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    let calls = magic_owna::row(333);
    let mut angles = ANGLES.to_vec();
    angles.extend(on_bounds(&t, at, (0x3a, 0x47, 0x4f)));
    t.stub_st0(333, move |m, _| {
        let k = m.r32(calls);
        f64::from(angles[(k as usize) % angles.len()])
    });
    t.set_u32(w(&t, 0x27), 0x00ab_0027);
    let parts = w(&t, 0x15);
    for n in [0i32, 1, 5, -1, 3] {
        for i in 0..6 {
            t.set_u32(parts + 0x130 * i, f(9.0));
        }
        t.set_u32(w(&t, 0x09), n.cast_unsigned());
        let c = Call {
            eax: 0x00ea_00ea,
            ..Call::default()
        };
        t.check_entry(at, c, &[], Eax::Checked);
    }
}

/// 0x10001e00: an object's facing while it runs (frame count set) and
/// at its start, the angle on each side of the range.
#[test]
fn object_facing() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1e00);
    rec(&mut t, 237, 4, |_, a| 0x0237_0000 ^ a[0]);
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    let calls = magic_owna::row(333);
    let mut angles = ANGLES.to_vec();
    angles.extend(on_bounds(&t, at, (0x54, 0x64, 0x78)));
    t.stub_st0(333, move |m, _| {
        let k = m.r32(calls);
        f64::from(angles[(k as usize) % angles.len()])
    });
    t.set_u32(t.img.word(at + 0x2d), 0x00ab_002d);
    let obj = SCRATCH + 0x1000;
    for frames in [3u32, 0, 0, 0, 0, 0, 0, 0, 0, 0] {
        t.write(obj, &[0x5c; 0x120]);
        t.set_u32(obj + 0x118, frames);
        let c = Call {
            eax: 0x00ea_00ea,
            ..Call::default()
        };
        t.check_entry(at, c, &[obj], Eax::Checked);
    }
}
