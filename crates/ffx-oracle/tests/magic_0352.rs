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

/// 0x10004e60 and 0x10004f60: the camera point and its show (junk under
/// the entry for the frame).
#[test]
fn camera_point() {
    use magic::StubMem;
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x4e60);
    for (s, k) in [(723u32, 0x0723u32), (724, 0x0724)] {
        rec(&mut t, s, 1, move |m: &mut dyn StubMem, a: &[u32]| {
            for i in 0..16 {
                m.w32(a[0] + 4 * i, k + i);
            }
            k
        });
    }
    rec(&mut t, 327, 2, |m: &mut dyn StubMem, a: &[u32]| {
        let v = m.r32(a[1]);
        m.w32(a[0], v ^ 0x327);
        0x327
    });
    rec(&mut t, 229, 3, |m: &mut dyn StubMem, a: &[u32]| {
        for i in 0..4 {
            let v = m.r32(a[1] + 4 * i) ^ m.r32(a[2] + 4 * i);
            m.w32(a[0] + 4 * i, v);
        }
        0x229
    });
    rec(&mut t, 904, 2, |_, a| 0x0904_0000 ^ a[0] ^ (a[1] << 4));
    let obj = SCRATCH + 0x1000;
    // 0x10004f60: the same, then slot 904
    for at in [at, t.at(0x4f60)] {
        t.write(obj, &[0x5c; 0x120]);
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}
