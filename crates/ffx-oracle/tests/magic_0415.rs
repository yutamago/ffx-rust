//! `magic_0415.dll` against its Rust ports (`ffx_magic::m0415`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 415;
/// Every slot the object steps call records its words (with a call
/// count) and answers its own value.
fn stubs(t: &mut Twin, slots: &[(u32, usize)]) {
    for &(s, n) in slots {
        rec(t, s, n, move |_, a| {
            (0x0100_0000 + s) ^ a.first().copied().unwrap_or(0)
        });
    }
}

/// The effect context `*A` (operand `ctx_op`) with its model words.
fn context(t: &mut Twin, at: u32, ctx_op: u32) {
    let ctx = SCRATCH + 0x2000;
    t.set_u32(t.img.word(at + ctx_op), ctx);
    for k in 0..0x80 {
        t.set_u32(ctx + 0x180 + 4 * k, 0x0c00_0000 + k);
    }
}

/// Runs the step at `at` for two list indices, the object junk.
fn run_obj(t: &mut Twin, at: u32) {
    let obj = SCRATCH + 0x1000;
    for i in [0u32, 2] {
        t.write(obj, &[0x5c; 0x140]);
        t.set_u32(obj + 0xac, i);
        t.check_entry(at, Call::default(), &[obj], Eax::Checked);
    }
}

/// 0x100011a0: the counter below and at the count, counts 3 and 0.
#[test]
fn turns_step() {
    let Some(mut t) = twin(N) else { return };
    stubs(&mut t, &[(447, 2), (459, 3), (450, 2), (449, 2), (451, 2)]);
    let at = t.at(0x11a0);
    context(&mut t, at, 0x05);
    let ctx = t.u32(t.img.word(at + 0x05));
    for (n, k) in [
        (3u32, 0u32),
        (3, 1),
        (3, 2),
        (5, 9),
        (0, 0),
        (0xffff_fffe, 0xffff_fff0),
    ] {
        t.set_u32(ctx + 0x19c, n);
        t.set_u32(t.img.word(at + 0x0a), k);
        run_obj(&mut t, at);
    }
}
