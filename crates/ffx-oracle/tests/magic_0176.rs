//! `magic_0176.dll` against its Rust port (`ffx_magic::m0176`): the
//! task step and the object colour callback. Needs `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 176;

/// 0x100016e0 (bound by the shared `s4_misc::effect_frame`; the census
/// counts it with the callback after it): the motion and objects done or
/// not.
#[test]
fn motion_objects_step() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x16e0);
    let ms = t.img.rel32_target(at + 0x05);
    let ro = t.img.rel32_target(at + 0x0c);
    let list = t.img.word(ro + 0x7c);
    let (n, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    t.set_u32(n + 8, obj);
    t.set_u16(obj + 0x124, 0x8001);
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    for (done, busy) in [(true, false), (false, false), (true, true), (false, true)] {
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

/// 0x10001710: the colour callback.
#[test]
fn part_colour() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1710);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 237, 4, |m, a| {
        for i in 0..4 {
            m.w32(a[3] + 4 * i, 0x0010_0000 + i);
        }
        0x237
    });
    t.set_u32(w(&t, 0x1f), 0x001f_0000);
    let obj = SCRATCH + 0x400;
    t.write(obj, &[0x33; 0x100]);
    t.check_entry(at, Call::default(), &[obj], Eax::Checked);
}
