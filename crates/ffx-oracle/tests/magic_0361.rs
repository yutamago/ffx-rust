//! `magic_0361.dll` against its Rust ports (`ffx_magic::m0361`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH, Twin};
use magic_owna::{rec, twin};

const N: u16 = 361;
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

/// 0x100014e0 (the counted object another), 0x10001570.
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
    let at = t.at(0x14e0);
    context(&mut t, at, 0x04);
    let other = SCRATCH + 0x3000;
    t.set_u32(t.img.word(at + 0x21), other);
    t.write(other, &[0x6c; 0x100]);
    t.set_u32(other + 0xac, 1);
    run_obj(&mut t, at);
    let at = t.at(0x1570);
    context(&mut t, at, 0x04);
    run_obj(&mut t, at);
}

/// 0x10001000: the start with slot 390 answering 0x1041 and not.
#[test]
fn start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1000);
    let kind = SCRATCH + 0x6f0;
    rec(&mut t, 390, 2, move |m, _| m.r32(kind));
    for (s, n) in [(493u32, 1usize), (367, 10), (448, 2), (450, 2), (449, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x04), fx);
    t.set_u32(fx + 4, 0x00e0_0004);
    t.set_u32(fx + 0x198, 0x00e0_0198);
    t.set_u32(fx + 0x1a0, 0x00e0_01a0);
    let task = SCRATCH + 0x100;
    for k in [0x1041u32, 0x1040, 0] {
        t.set_u32(kind, k);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}

/// 0x10001af0: an object's start.
#[test]
fn object_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1af0);
    for (s, n) in [(441u32, 3usize), (440, 7), (450, 2), (449, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[1]);
    }
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x04), fx);
    t.set_u32(fx + 0x198, 0x00e0_0198);
    let obj = SCRATCH + 0x1000;
    t.write(obj, &[0x5c; 0x140]);
    t.check_entry(at, Call::default(), &[obj], Eax::Checked);
}

/// 0x10001110: the kinds 0, 1 and 2 with slot 492 answering above 0 and
/// not.
#[test]
fn sound_start() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1110);
    let answer = SCRATCH + 0x6f0;
    rec(&mut t, 492, 3, move |m, _| m.r32(answer));
    for (s, n) in [(447u32, 2usize), (486, 11), (444, 1), (450, 2), (449, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let fx = SCRATCH + 0x8000;
    t.set_u32(t.img.word(at + 0x04), fx);
    t.set_u32(fx + 0x198, 0x00e0_0198);
    t.set_u32(fx + 0x1a0, 0x00e0_01a0);
    let task = SCRATCH + 0x1000;
    for (kind, r) in [(0u32, 1u32), (0, 0), (1, 5), (1, u32::MAX), (2, 1)] {
        t.set_u32(t.img.word(at + 0x47), kind);
        t.set_u32(answer, r);
        t.write(task, &[0; 0x100]);
        t.set_u32(task + 0xac, kind + 1);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
