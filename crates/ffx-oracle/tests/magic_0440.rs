//! `magic_0440.dll` against its Rust ports (`ffx_magic::m0440`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 440;

/// 0x10003440: the angle above, inside, below the range and NaN; the
/// objects busy, the motion running, the count above 0 and the end.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3440);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [(719u32, 3usize), (306, 3)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let ms = t.img.rel32_target(at + 0x05);
    let ro = t.img.rel32_target(at + 0x5b);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let fl = |t: &Twin, op: u32| f32::from_bits(t.u32(w(t, op)));
    let (d, hi, lo) = (fl(&t, 0x15), fl(&t, 0x1e), fl(&t, 0x32));
    let mut angles = vec![0.0f32, hi - d + 1.0, lo - d - 1.0, f32::NAN];
    for bound in [hi, lo] {
        let guess = (bound - d).to_bits();
        if let Some(a) = (0..256u32)
            .flat_map(|k| [guess.wrapping_add(k), guess.wrapping_sub(k)])
            .map(f32::from_bits)
            .find(|&a| (a + d).to_bits() == bound.to_bits())
        {
            angles.push(a);
        }
    }
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    let cases = [
        (true, true, 0u32),
        (false, false, 0),
        (false, true, 1),
        (false, true, u32::MAX),
        (false, true, 0),
    ];
    for (i, a) in angles.iter().enumerate() {
        let (busy, done, count) = cases[i % cases.len()];
        t.set_u32(w(&t, 0x0d), a.to_bits());
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.set_u32(w(&t, 0x6a), count);
        // the word copied (*X) and its two copies' old values
        t.set_u32(w(&t, 0x44), 0x00ab_0044);
        t.set_u32(w(&t, 0x51), 0x5151_5151);
        t.set_u32(w(&t, 0x56), 0x5656_5656);
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
