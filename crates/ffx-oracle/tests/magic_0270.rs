//! `magic_0270.dll` against its Rust ports (`ffx_magic::m0270`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, row, twin};

const N: u16 = 270;

/// The floats slot 333 answers, in turn.
const ANGLES: [f32; 5] = [-9.0, 0.5, 3.5, 12.0, f32::NAN];

/// 0x10001b10: the facing on each side of the range, next to each bound,
/// exactly on the upper bound (when a float reaches it) and NaN.
#[test]
fn object_facing() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1b10);
    let fl = |t: &magic::Twin, op: u32| f32::from_bits(t.u32(t.img.word(at + op)));
    let (base, hi, lo) = (fl(&t, 0x1e), fl(&t, 0x2d), fl(&t, 0x4e));
    let mut angles = ANGLES.to_vec();
    // K - f just inside and just outside each bound
    for b in [hi, lo] {
        for d in [-0.01f32, 0.01] {
            angles.push(base - b + d);
        }
    }
    let guess = (base - hi).to_bits();
    if let Some(r) = (0..256u32)
        .flat_map(|d| [guess.wrapping_add(d), guess.wrapping_sub(d)])
        .map(f32::from_bits)
        .find(|&r| (base - r).to_bits() == hi.to_bits())
    {
        angles.push(r);
    }
    let table = SCRATCH + 0x6000;
    for (k, a) in (0u32..16).zip(angles.iter().cycle()) {
        t.set_u32(table + 4 * k, a.to_bits());
    }
    rec(&mut t, 333, 1, |_, a| 0x0333_0000 ^ a[0]);
    let calls = row(333);
    t.stub_st0(333, move |m, _| {
        let k = m.r32(calls);
        f64::from(f32::from_bits(m.r32(table + 4 * (k % 16))))
    });
    t.set_u32(t.img.word(at + 0x0b), 0x00ab_000b);
    let obj = SCRATCH + 0x1000;
    for _ in 0..16 {
        t.write(obj, &[0x5c; 0xd0]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x10001bd0: the sound task for sounds 0 and 2 (and -1), into an empty
/// and a filled task list.
#[test]
fn sound_task() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1bd0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 239, 3, |_, a| 0x0239_0000 ^ a[1]);
    let bytes = SCRATCH + 0x700;
    t.write(bytes, &[0x31, 0x32, 0x33, 0x34]);
    t.set_u32(w(&t, 0x1c), bytes);
    t.set_u32(w(&t, 0x05), 0x00ab_0005);
    let take = t.img.rel32_target(at + 0x4d);
    let (count, head) = (t.img.word(take + 0x04), t.img.word(take + 0x17));
    let node = |i: u32| SCRATCH + 0x1800 + 0x40 * i;
    let last = SCRATCH + 0x1a00;
    let rec_ = SCRATCH + 0x100;
    for (i, first) in [(0u32, 0u32), (2, last), (u32::MAX, last), (1, 0)] {
        for k in 0..2 {
            t.write(node(k), &[0x5a; 0x40]);
            t.set_u32(node(k), node(1 - k));
            t.set_u32(node(k) + 4, node(1 - k));
        }
        t.set_u32(head, node(0));
        t.set_u16(count, 2);
        t.write(last, &[0x66; 0x10]);
        t.set_u32(w(&t, 0x53), first);
        t.set_u32(w(&t, 0x69), first);
        t.write(rec_, &[0x44; 0x40]);
        t.set_u32(rec_ + 0x2c, i);
        t.set_u32(rec_ + 0x30, 0x0030_0030);
        // the pair table: only the cases' rows (sound -1 writes below it)
        t.check_entry(at, Call::default(), &[rec_], Eax::Checked);
    }
}
