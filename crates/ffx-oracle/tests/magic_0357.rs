//! `magic_0357.dll` against its Rust ports (`ffx_magic::m0357`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 357;

/// 0x100010f0: the object's start for two list indices.
#[test]
fn object_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x10f0);
    for (s, n) in [(444u32, 1usize), (434, 2), (450, 2), (449, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let ctx = SCRATCH + 0x2000;
    t.set_u32(t.img.word(at + 0x04), ctx);
    t.set_u32(ctx + 0x198, 0x0198_0198);
    let obj = SCRATCH + 0x1000;
    for i in [0u32, 3] {
        t.write(obj, &[0x5c; 0x100]);
        t.set_u32(obj + 0xac, i);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x100011e0 (the counter below and at 180) and 0x10001860 (below and
/// at 10).
#[test]
fn copies_and_marks() {
    let Some(mut t) = twin(N) else { return };
    for (s, n) in [(434u32, 2usize), (446, 2), (388, 3), (450, 2), (451, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let at = t.at(0x11e0);
    let n_at = t.img.word(at + 0x64);
    let obj = SCRATCH + 0x1000;
    for n in [0u32, 178, 179, 300] {
        t.set_u32(n_at, n);
        t.write(obj, &[0x5c; 0x40]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
    let at = t.at(0x1860);
    let ctx = SCRATCH + 0x2000;
    t.set_u32(t.img.word(at + 0x04), ctx);
    t.set_u32(ctx + 0x1a0, 0x01a0_01a0);
    let n_at = t.img.word(at + 0x0f);
    for n in [0u32, 8, 9, 20] {
        t.set_u32(n_at, n);
        t.write(obj, &[0x5c; 0x40]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
