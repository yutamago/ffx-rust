//! `magic_0149.dll` against its Rust port (`ffx_magic::m0149`): the
//! countdown step. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 149;

/// 0x10003950: each countdown above 1, at 1 (the sound) and at 0 or
/// below, with the motion and objects done or not.
#[test]
fn countdown_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x3950);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[2]);
    rec(&mut t, 306, 3, |_, a| 0x306 ^ a[2]);
    let ms = t.img.rel32_target(at + 0x7d);
    let ro = t.img.rel32_target(at + 0x84);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    for (c1, c2, done, busy) in [
        (3i32, 0i32, true, false),
        (1, 1, true, false),
        (0, -2, false, false),
        (5, 1, true, true),
        (1, 9, false, true),
    ] {
        t.set_u32(w(&t, 0x04), c1.cast_unsigned());
        t.set_u32(w(&t, 0x40), c2.cast_unsigned());
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.set_u32(list + 8, if busy { n } else { 0 });
        t.set_u32(t.img.word(ro + 0x14), if busy { n } else { 0 });
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
