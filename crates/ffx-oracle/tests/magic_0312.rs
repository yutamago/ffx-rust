//! `magic_0312.dll` against its Rust ports (`ffx_magic::m0312`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 312;

/// The context and the slots the object steps call.
fn setup(t: &mut Twin, at: u32) {
    let ctx = SCRATCH + 0x2000;
    t.set_u32(t.img.word(at + 0x04), ctx);
    t.set_u32(ctx, 0x00c0_0000);
    t.set_u32(ctx + 4, 0x00c0_0004);
    t.set_u32(ctx + 0x198, 0x0198_0198);
    t.set_u32(ctx + 0x1a0, 0x01a0_01a0);
    for (s, n) in [
        (481u32, 1usize),
        (14, 4),
        (448, 2),
        (450, 2),
        (449, 2),
        (451, 2),
        (444, 1),
        (434, 2),
        (486, 11),
        (447, 2),
        (443, 2),
        (442, 3),
        (440, 4),
    ] {
        rec(t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    // slot 367 writes the flags local (4th argument word 4)
    rec(t, 367, 8, |m, a| {
        m.w32(a[4], m.r32(SCRATCH + 0x6f0));
        0x0367
    });
}

/// Every function here with the object's words junk and two list
/// indices (0x10001150 with the flags bit 8 set and clear).
#[test]
fn object_steps() {
    let Some(mut t) = twin(N) else { return };
    let obj = SCRATCH + 0x1000;
    t.write(magic::STACK_TOP - 0x2000, &[0x6b; 0x1f00]);
    for rva in [0x1000u32, 0x1150, 0x1260, 0x1350, 0x14b0, 0x1640] {
        let at = t.at(rva);
        setup(&mut t, at);
        if rva == 0x1000 {
            for op in [
                0x0a, 0x20, 0x2a, 0x34, 0x3e, 0x48, 0x52, 0x5c, 0x66, 0x70, 0x7a, 0x84, 0x8e, 0xa9,
            ] {
                t.set_u32(t.img.word(at + op), 0x5a5a_5a5b);
            }
        }
        for (i, flags) in [(0u32, 0x100u32), (2, 0x0ff)] {
            t.set_u32(SCRATCH + 0x6f0, flags);
            t.write(obj, &[0x5c; 0x140]);
            t.set_u32(obj + 0xac, i);
            t.check_entry(at, Call::default(), &[obj], Eax::Checked);
        }
    }
}
