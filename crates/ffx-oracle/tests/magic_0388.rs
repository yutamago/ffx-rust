//! `magic_0388.dll` against its Rust ports (`ffx_magic::m0388`). Needs
//! `FFX_GAME_DIR`.

mod magic;
mod magic_owna;

use magic::{Call, Eax, SCRATCH};
use magic_owna::{rec, twin};

const N: u16 = 388;

/// 0x10007610: state 0 (the motion ended or not, either model kind),
/// state 10 with the countdown at 0 and above, another state.
#[test]
fn motion_wait() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x7610);
    rec(&mut t, 20, 1, |m, _| m.r32(SCRATCH + 0x6f0));
    rec(&mut t, 411, 1, |m, _| m.r32(SCRATCH + 0x6f0));
    for (s, n) in [(30u32, 1usize), (36, 2), (228, 2)] {
        rec(&mut t, s, n, move |_, a| (0x0100_0000 + s) ^ a[0]);
    }
    let (task, st, obj) = (SCRATCH + 0x100, SCRATCH + 0x200, SCRATCH + 0x300);
    let node = SCRATCH + 0x400;
    let (prev, next) = (SCRATCH + 0x500, SCRATCH + 0x600);
    for (state, count, ended, kind) in [
        (0u32, 5u32, 1u32, 1u8),
        (0, 5, 1, 0),
        (0, 5, 0, 1),
        (0xa, 0, 0, 0),
        (0xa, 3, 0, 0),
        (7, 3, 0, 0),
    ] {
        t.set_u32(task, node);
        t.set_u32(task + 4, st);
        t.write(st, &[0x22; 0x30]);
        t.set_u32(st + 0x18, state);
        t.set_u32(st + 0x1c, count);
        t.set_u32(st + 0x20, obj);
        t.write(obj, &[0x33; 0xa0]);
        t.set_u8(obj + 0x90, kind);
        t.set_u32(node + 4, prev);
        t.set_u32(node + 8, next);
        t.write(prev, &[0x11; 0x10]);
        t.write(next, &[0x44; 0x10]);
        t.set_u32(SCRATCH + 0x6f0, ended);
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
