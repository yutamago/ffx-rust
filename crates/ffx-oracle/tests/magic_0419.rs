//! `magic_0419.dll` against its Rust ports (`ffx_magic::m0419`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 419;

/// 0x10001a70: the countdown at 1 (both sound branches, the halfword set
/// and clear) and above; the objects busy or done; the done bits.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1a70);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    for (s, n) in [
        (296u32, 2usize),
        (257, 2),
        (256, 5),
        (213, 2),
        (719, 3),
        (306, 3),
        (941, 1),
        (772, 2),
        (779, 4),
    ] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let ro = t.img.rel32_target(at + 0xb6);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    for (count, flag, sound, half, busy, bits, left) in [
        (1i32, 0u32, 3i32, 0u16, false, 0x8000u32, 0i32),
        (1, 1, 3, 1, false, 0x8000, 0),
        (1, 0, 0x20, 0, true, 0x8000, 0),
        (4, 0, 3, 0, false, 0x8000, 0),
        (0, 0, 3, 0, false, 0, 0),
        (0, 0, 3, 0, false, 0x8000, 2),
    ] {
        t.set_u32(w(&t, 0x04), count.cast_unsigned());
        t.set_u32(w(&t, 0x1e), flag);
        t.set_u32(w(&t, 0x26), sound.cast_unsigned());
        t.set_u32(w(&t, 0x48), 0x00ab_0048);
        t.set_u16(w(&t, 0x8c), half);
        t.set_u32(w(&t, 0xc1), bits);
        t.set_u32(w(&t, 0xcd), left.cast_unsigned());
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
