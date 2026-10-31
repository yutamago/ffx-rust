//! `magic_0514.dll` against its Rust port (`ffx_magic::m0514`). Needs
//! `FFX_GAME_DIR`.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::many_single_char_names
)]

mod magic;
mod magic_hi;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0514};
use magic::{Call, Eax, SCRATCH};
use magic_hi::f;
use magic_owna::twin;

const N: u16 = 514;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

/// 0x10007d70 and 0x10006e30: the two set-ups, the node list empty and
/// not.
#[test]
fn inits() {
    let Some(mut t) = twin(N) else { return };
    let (obj, owner, model) = (SCRATCH + 0x1000, SCRATCH + 0x2000, SCRATCH + 0x2400);
    let ring = SCRATCH + 0x3000;
    let last = SCRATCH + 0x3100;
    t.set_u32(owner + 0x140, model);
    t.set_u32(model + 0x28, f(2.5));
    let at = t.at(0x7d70);
    t.set_u32(t.img.word(at + 0x1a), ring);
    t.set_u32(t.img.word(at + 0x32), 0xff);
    t.write(obj, &[0x4e; 0x200]);
    let c = Call {
        ecx: obj,
        edx: 0x00ed_0000,
        ..Call::default()
    };
    t.check(at, c, &[0xa, 0xb, 0xc], Eax::Checked, |g, img, fr| {
        Ret::eax(m0514::timed_init(g, img, fr))
    });
    let at = t.at(0x6e30);
    for full in [false, true] {
        t.write(obj, &[0x4e; 0x200]);
        t.write(ring, &[0x1b; 0x20]);
        t.set_u32(t.img.word(at + 0x1e), ring);
        let sentinel = t.img.word(at + 0x71);
        t.write(last, &[0x1c; 0x20]);
        t.set_u32(t.img.word(at + 0x64), if full { last } else { sentinel });
        t.set_u16(t.img.word(at + 0x85), 3);
        t.check(at, c, &[owner], Eax::Checked, |g, img, fr| {
            Ret::eax(m0514::scaled_init(g, img, fr))
        });
    }
}

/// 0x10007260 and 0x10007660: the tasks' states (the nodes without
/// parents).
#[test]
fn tasks() {
    let Some(mut t) = twin(N) else { return };
    let (task, r, node, prev, next) = (
        SCRATCH + 0x100,
        SCRATCH + 0x1000,
        SCRATCH + 0x200,
        SCRATCH + 0x240,
        SCRATCH + 0x280,
    );
    for (rva, state, wait, byte) in [
        (0x7260u32, 0u32, 0u32, 0u8),
        (0x7260, 1, 1, 0),
        (0x7260, 1, 0, 0),
        (0x7260, 1, 0, 4),
        (0x7260, 0x2710, 0, 4),
        (0x7260, 9, 0, 0),
        (0x7660, 0, 0, 0x40),
        (0x7660, 1, 0, 0x10),
        (0x7660, 0x2710, 0, 0),
        (0x7660, 0x4e20, 0, 0),
    ] {
        t.write(r, &[0; 0x200]);
        t.set_u32(task, node);
        t.set_u32(task + 4, r);
        t.set_u32(node + 4, next);
        t.set_u32(node + 8, prev);
        t.set_u32(r + 0x18, state);
        t.set_u32(r + 0x1c, wait);
        t.set_u8(r + 0x2b8, byte);
        t.set_u8(r + 0x1eb, byte);
        t.check_entry(t.at(rva), Call::default(), &[task], Eax::Checked);
    }
}
