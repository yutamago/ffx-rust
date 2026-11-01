//! `magic_0421.dll` against its Rust ports (`ffx_magic::m0421`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 421;
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

/// 0x10001300: no copies and three (each copy its own object).
#[test]
fn copies_start() {
    let Some(mut t) = twin(N) else { return };
    stubs(&mut t, &[(444, 1), (522, 2), (450, 2), (449, 2)]);
    let copies = SCRATCH + 0x4000;
    rec(&mut t, 434, 2, move |m, _| {
        let k = m.r32(magic_owna::row(434));
        copies + 0x100 * (k % 8)
    });
    let at = t.at(0x1300);
    context(&mut t, at, 0x04);
    for n in [0u32, 3] {
        t.write(copies, &[0x4c; 0x800]);
        for k in 0..8 {
            t.set_u32(copies + 0x100 * k + 0xac, k % 3);
        }
        t.set_u32(t.img.word(at + 0x14), n);
        run_obj(&mut t, at);
    }
}
