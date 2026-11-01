//! `magic_0245.dll` against its Rust ports (`ffx_magic::m0245`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 245;

/// 0x100016e0: the timers (positive, reaching 0, at 0 and negative),
/// then the motion step and the objects done or busy.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x16e0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 239, 3, |_, a| 0x0239_0000 ^ a[1]);
    rec(&mut t, 719, 3, |_, a| 0x0719_0000 ^ a[2]);
    rec(&mut t, 306, 3, |_, a| 0x306 ^ a[2]);
    let ms = t.img.rel32_target(at + 0x5d);
    let ro = t.img.rel32_target(at + 0x64);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let bytes = SCRATCH + 0x700;
    t.write(bytes, &[0x31, 0x32, 0x33, 0x34, 0x35, 0x36]);
    t.set_u32(w(&t, 0x35), bytes);
    t.set_u32(w(&t, 0x27), 0x00ab_0027);
    let timers = w(&t, 0x13);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    for (count, times, done, busy) in [
        (4i32, [1i32, 2, 0, -3], true, false),
        (3, [1, 1, 5, 0], true, true),
        (0, [1, 1, 1, 1], false, false),
        (2, [9, 1, 0, 0], true, false),
    ] {
        t.set_u32(w(&t, 0x05), count.cast_unsigned());
        for (i, v) in (0u32..).zip(times) {
            t.set_u32(timers + 4 * i, v.cast_unsigned());
        }
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
