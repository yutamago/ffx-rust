//! `magic_0304.dll` against its Rust ports (`ffx_magic::m0304`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{f, rec, twin};

const N: u16 = 304;

/// 0x10001cd0: before and from frame 0x54, two table bytes.
#[test]
fn rise() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1cd0);
    let w = |t: &Twin, off: u32| t.img.word(at + off);
    rec(&mut t, 12, 1, |_, a| 0x0012_0000 ^ a[0]);
    t.stub_st0(12, |_, a| f64::from(a[0] & 0xff) * 0.5);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(w(&t, 0x1b), fx);
    let pb = SCRATCH + 0x700;
    t.set_u32(w(&t, 0x11), pb);
    let table = w(&t, 0x22);
    for i in 0..4 {
        t.set_u32(table + 4 * i, 0x0a00_0000 + i);
    }
    let obj = SCRATCH + 0x1000;
    for (frames, b) in [(0x53u32, 1u8), (0x54, 1), (0x100, 3), (0xffff_fff0, 0)] {
        t.write(pb, &[b, 9, 9, 9]);
        t.set_u32(fx + 0x1720, f(0.75));
        t.write(obj, &[0x5c; 0x120]);
        t.set_u32(obj + 0x118, frames);
        t.set_u32(obj + 0xd4, f(-2.5));
        let c = Call {
            eax: 0x00ea_00ea,
            ..Call::default()
        };
        t.check_entry(at, c, &[obj], Eax::Checked);
    }
}

/// 0x10002150: slot 520 answering the battle mode and another.
#[test]
fn drift() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2150);
    rec(&mut t, 520, 0, |m, _| m.r32(SCRATCH + 0x6f0));
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x15), fx);
    for mode in [0x018b_0046u32, 0x018b_0047, 0] {
        t.set_u32(SCRATCH + 0x6f0, mode);
        t.set_u32(fx + 0x334, f(1.25));
        // -0.0 + 0.0 for the other modes, a value for the battle's
        let x = if mode == 0 { -0.0f32 } else { -0.75 };
        t.set_u32(fx + 0x204, f(x));
        // the neighbours other values
        t.set_u32(fx + 0x200, f(3.5));
        t.set_u32(fx + 0x208, f(5.25));
        t.set_u32(fx + 0x330, f(-6.5));
        t.set_u32(fx + 0x338, f(7.75));
        t.check_entry(at, Call::default(), &[], Eax::Checked);
    }
}

/// 0x100021c0: the eased height at several times across the keys.
#[test]
fn eased_height() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x21c0);
    let fx = SCRATCH + 0x1_0000;
    t.set_u32(t.img.word(at + 0x03), fx);
    let keys = t.img.word(at + 0x17);
    for (i, v) in (0u32..).zip([0.0f32, 2.0, -1.0, 4.5, 3.0, 7.0]) {
        t.set_u32(keys + 4 * i, f(v));
    }
    for time in [0.0f32, 0.3, 1.75, 2.5, 3.9] {
        // the frame and the caller's words scrubbed each case
        t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
        t.set_u32(fx + 0x1724, f(time));
        t.set_u32(fx + 0x149c, f(10.0));
        t.check_entry(at, Call::default(), &[], Eax::Junk);
    }
}
