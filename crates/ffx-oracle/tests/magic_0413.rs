//! `magic_0413.dll` against its Rust ports (`ffx_magic::m0413`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 413;

/// 0x10003360: the angles inside, above and below their range; the
/// motion step and the objects running or done; the count set and not.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3360);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(719u32, 3usize), (306, 3)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let ms = t.img.rel32_target(at + 0x05);
    let ro = t.img.rel32_target(at + 0x8b);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    t.set_u32(w(&t, 0x74), 0x00ab_0074);
    // the steps added to the angles (non-zero, so add and sub differ)
    let (da, db) = (0.25f32, -0.375f32);
    t.set_u32(w(&t, 0x15), da.to_bits());
    t.set_u32(w(&t, 0x5b), db.to_bits());
    let hi = f32::from_bits(t.u32(w(&t, 0x1d)));
    let lo = f32::from_bits(t.u32(w(&t, 0x2d)));
    // an angle whose sum with the step is the bound itself, when one is
    let exact = |bound: f32, d: f32| {
        let guess = (bound - d).to_bits();
        (0..256u32)
            .flat_map(|k| [guess.wrapping_add(k), guess.wrapping_sub(k)])
            .map(f32::from_bits)
            .find(|&a| (a + d).to_bits() == bound.to_bits())
            .unwrap_or(bound - d)
    };
    let mut cases = vec![
        (0.5f32, -0.5f32, true, false, 0i32),
        (10.0, -10.0, true, false, 0),
        (f32::NAN, 3.0, false, false, 0),
        (0.0, 0.0, true, true, 0),
        (0.0, 0.0, true, false, 2),
    ];
    cases.push((exact(hi, da), exact(lo, db), true, false, 0));
    cases.push((exact(lo, da), exact(hi, db), true, false, 0));
    for (a, b, done, busy, count) in cases {
        t.set_u32(w(&t, 0x0d), a.to_bits());
        t.set_u32(w(&t, 0x53), b.to_bits());
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        t.set_u32(w(&t, 0x9a), count.cast_unsigned());
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
