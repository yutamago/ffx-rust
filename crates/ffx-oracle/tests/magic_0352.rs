//! `magic_0352.dll` against its Rust ports (`ffx_magic::m0352`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, row, twin};

const N: u16 = 352;

/// 0x10004c10: the script and the motion step running or done.
#[test]
fn frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4c10);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 772, 2, |m, a| {
        let k = m.r32(row(772));
        0x0772_0000 ^ a[0] ^ (k << 8)
    });
    t.stub_edx(772, |_, a| 0x00ed_0000 ^ a[0]);
    rec(&mut t, 779, 4, |_, a| 0x0779_0000 ^ a[2]);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x0a), fx);
    t.set_u32(w(&t, 0x3f), SCRATCH + 0x600);
    let sr = t.img.rel32_target(at + 0x0f);
    let ms = t.img.rel32_target(at + 0x16);
    let node = SCRATCH + 0x3000;
    for (ended, done) in [(true, true), (false, true), (true, false)] {
        t.set_u8(t.img.word(sr + 0x01), if ended { 0 } else { 2 });
        t.set_u16(t.img.word(sr + 0x59), 0xffff);
        t.set_u32(t.img.word(sr + 0x91), 0);
        t.set_u32(t.img.word(ms + 0x02), u32::from(done));
        t.set_u16(t.img.word(ms + 0x11), 5);
        t.write(node, &[0x4c; 0x20]);
        t.write(fx + 0xbd0, &[0x44; 0x10]);
        t.check_entry(at, Call::default(), &[node], Eax::Checked);
    }
}
