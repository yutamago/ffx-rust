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

/// 0x10001930 (`r13_start::object_start_0360`): the object started from
/// the caster's place with two draws of `rand`, its flags kept.
#[test]
fn object_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1930);
    rec(&mut t, 458, 3, |_, a| 0x0458_0000 ^ a[0]);
    rec(&mut t, 450, 2, |_, a| 0x0450_0000 ^ a[1]);
    rec(&mut t, 449, 2, |_, a| 0x0449_0000 ^ a[1]);
    let c = SCRATCH + 0x800;
    for (i, x) in (0u32..).zip([2.5f32, 1.5, 0.5, -0.5]) {
        t.set_u32(c + 0xd0 + 4 * i, x.to_bits());
    }
    t.set_u32(t.img.word(at + 0x3e), c);
    t.set_u32(t.img.word(at + 0x8b), 0x00c0_008b);
    let o = SCRATCH + 0x1000;
    for flags in [0u32, 0x8001] {
        t.write(o, &[0x5d; 0x180]);
        t.set_u32(o + 0xbc, flags);
        t.check_entry(at, Call::default(), &[o], Eax::Checked);
    }
}
