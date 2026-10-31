//! `magic_0053.dll` against its Rust port (`ffx_magic::m0053`): the
//! sound step, and `magic_0065`'s copy without the slot 594 gate. Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, recs, twin};

const N: u16 = 53;

/// A sound step: the motion step not done, each wait, slot 594 busy or
/// the flag clear (with the gate), and the three sounds (with no parts,
/// and with parts and a bank to register). `ops`: the operands of `A`,
/// `B`, `C`, `D`, `E` (0: none), the sound pointer, `V`, the table;
/// `reg_at` the registration's `rel32`.
fn sound_cases(n: u16, rva: u32, ops: [u32; 8], reg_at: u32) {
    let Some(mut t) = twin(n) else { return };
    let at = t.at(rva);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    recs(&mut t, &[(249, 2), (530, 2)]);
    rec(&mut t, 941, 1, |_, a| 0x9410_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x7720_0000 ^ a[0] ^ a[1] << 8);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[1]);
    rec(&mut t, 779, 4, |_, a| a[0]);
    let busy = SCRATCH + 0x600;
    rec(&mut t, 594, 0, move |m, _| m.r32(busy));
    let ms = t.img.rel32_target(at + 0x04);
    let reg = t.img.rel32_target(at + reg_at);
    let r = |t: &Twin, off: u32| t.img.word(reg + off);
    let (count, table, lock, banks) = (r(&t, 0x0e), r(&t, 0x16), r(&t, 0x43), r(&t, 0x76));
    let parts = SCRATCH + 0x4000;
    t.set_u32(table, parts);
    for i in 0..3 {
        t.set_u32(parts + 0x74 * i + 0x30, 0x0300_0000 + i);
    }
    t.set_u32(lock, SCRATCH + 0x800);
    let snd = SCRATCH + 0x700;
    t.set_u32(w(&t, ops[5]), snd);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    let base = t.save();
    // (A, B, C, D, E, busy, V, byte, parts)
    for (i, c) in [
        [1u32, 0, 0, 1, 1, 0, 0, 3, 0],
        [1, 0, 0, 1, 1, 0, 0, 3, 0],
        [1, 0x8000, 1, 1, 1, 0, 0, 3, 0],
        [1, 0x8000, 0, 1, 1, 0, 0, 3, 0],
        [0, 0, 0, 0, 1, 7, 0, 3, 0],
        [0, 0, 0, 0, 0, 0, 0, 3, 0],
        [0, 0, 0, 0, 1, 0, 0, 0x1f, 2],
        [2, 0, 0, 1, 0, 0, 0, 0x20, 0],
        [0, 0, 0, 1, 0, 0, 1, 5, 2],
    ]
    .into_iter()
    .enumerate()
    {
        t.restore(&base);
        for (k, &off) in ops[..5].iter().enumerate() {
            if off != 0 {
                t.set_u32(w(&t, off), c[k]);
            }
        }
        t.set_u32(busy, c[5]);
        t.set_u32(w(&t, ops[6]), c[6]);
        t.write(snd, &[u8::try_from(c[7]).unwrap(), 0x40]);
        t.set_u32(w(&t, ops[7]) + 4 * c[7], 0x00ab_0000 + c[7]);
        t.set_u32(count, c[8]);
        t.set_u32(banks, c[8] / 2);
        t.set_u32(SCRATCH + 0x800, 0x5555);
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        // the motion step: the first case not done (its wait counting)
        t.set_u32(t.img.word(ms + 0x02), u32::from(i != 0));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

#[test]
fn sound_step() {
    sound_cases(
        N,
        0x13d0,
        [0x13, 0x1b, 0x2b, 0x38, 0x49, 0x68, 0x71, 0x9b],
        0xc6,
    );
    sound_cases(65, 0x31a0, [0x13, 0x1b, 0x27, 0, 0, 0x30, 0x39, 0x63], 0x8d);
}
