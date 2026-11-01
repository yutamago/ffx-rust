//! `magic_0360.dll` against its Rust ports (`ffx_magic::m0360`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 360;
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

/// 0x10001160, 0x10001260, 0x10001a80.
#[test]
fn object_steps() {
    let Some(mut t) = twin(N) else { return };
    stubs(
        &mut t,
        &[
            (432, 0),
            (434, 2),
            (440, 4),
            (444, 1),
            (446, 2),
            (447, 2),
            (449, 2),
            (450, 2),
            (451, 2),
            (485, 9),
            (486, 11),
        ],
    );
    for (rva, ctx) in [(0x1160u32, 0x04u32), (0x1260, 0x5a), (0x1a80, 0x19)] {
        let at = t.at(rva);
        context(&mut t, at, ctx);
        run_obj(&mut t, at);
    }
}
