//! `magic_0356.dll` against its Rust ports (`ffx_magic::m0356`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 356;
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

/// 0x10001160 (the counter below, at and past 60) and 0x100011c0.
#[test]
fn object_steps() {
    let Some(mut t) = twin(N) else { return };
    stubs(
        &mut t,
        &[
            (434, 2),
            (444, 1),
            (446, 2),
            (447, 2),
            (449, 2),
            (450, 2),
            (451, 2),
            (486, 11),
            (491, 5),
            (478, 3),
            (388, 3),
        ],
    );
    let at = t.at(0x1160);
    let n_at = t.img.word(at + 0x34);
    for n in [0u32, 58, 59, 100] {
        t.set_u32(n_at, n);
        run_obj(&mut t, at);
    }
    let at = t.at(0x11c0);
    context(&mut t, at, 0x04);
    run_obj(&mut t, at);
}
