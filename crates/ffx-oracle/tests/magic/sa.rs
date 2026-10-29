//! Helpers of the sharedA overlay tests (`magic_sa_*.rs` files that
//! `#[path]` this in next to `magic/mod.rs`): node rings for the handlers'
//! walks and recording stubs.

#![allow(dead_code)]

use super::magic::{Call, Eax, SCRATCH, StubMem, TRAPS, Twin};
use ffx_magic::{Frame, Image, Ret};

/// The DLL started and `InitMagicPRX` run (and checked) on both sides.
pub fn twin(n: u16) -> Option<Twin> {
    let mut t = Twin::load(n)?;
    t.init();
    Some(t)
}

/// A slot no overlay uses (`Ch_Allocate`): the ring children's handler.
pub const HANDLER_SLOT: u32 = 0;
/// Another unused slot (`Ch_ClothReset`): a second ring's handler.
pub const HANDLER_SLOT_2: u32 = 2;
pub const CURSOR: u32 = SCRATCH;
/// The children's handler table (`+0x1c` the handler).
pub const VTABLE: u32 = SCRATCH + 0x100;
pub const VTABLE_2: u32 = SCRATCH + 0x180;

/// Node `i` of the test rings (0x200 bytes each).
#[must_use]
pub fn node(i: u32) -> u32 {
    SCRATCH + 0x1000 + 0x200 * i
}

/// Records the first `n` words of each call of `slot` into its own row
/// (a count, then the words, each xored with the call's number) and
/// answers `ret`.
pub fn rec(
    t: &mut Twin,
    slot: u32,
    n: usize,
    ret: impl Fn(&mut dyn StubMem, &[u32]) -> u32 + 'static,
) {
    let row = SCRATCH + 0x2_0000 + 0x80 * (slot % 0x400);
    t.stub(slot, move |m, a| {
        let k = m.r32(row);
        m.w32(row, k + 1);
        for (i, w) in (0u32..).zip(&a[..n]) {
            m.w32(row + 4 + 4 * i, *w ^ k);
        }
        ret(m, a)
    });
}

/// The handler stubs: each records its cursor and the cursor's node, and
/// answers a number per call.
pub fn handler_stubs(t: &mut Twin) {
    for s in [HANDLER_SLOT, HANDLER_SLOT_2] {
        rec(t, s, 1, move |m, a| {
            let row = SCRATCH + 0x2_0000 + 0x80 * s;
            let k = m.r32(row);
            m.w32(row + 0x40 + 4 * (k % 8), m.r32(a[0]));
            // the cursor's pass byte (+5; the rest of the word is the
            // caller's frame, the same on both sides)
            m.w32(row + 0x60 + 4 * (k % 8), m.r32(a[0] + 4));
            0x1234_5600 + k
        });
    }
    t.set_u32(VTABLE + 0x1c, TRAPS + 16 * HANDLER_SLOT);
    t.set_u32(VTABLE_2 + 0x1c, TRAPS + 16 * HANDLER_SLOT_2);
}

/// Node 0 with `children` (nodes 1..) and the cursor on it: the cursor
/// moves when `pass` differs from `cursor_pass`. Every node is filled
/// with `fill` first.
pub fn ring(t: &mut Twin, children: u32, pass: u8, cursor_pass: u8, fill: u8) {
    let root = node(0);
    for i in 0..=children {
        t.write(node(i), &[fill; 0x200]);
        t.set_u32(node(i), VTABLE);
    }
    t.set_u8(root + 0x1a, pass);
    let first = if children == 0 { root } else { node(1) };
    t.set_u32(root + 0x10, first);
    for i in 1..=children {
        let next = if i == children { root } else { node(i + 1) };
        t.set_u32(node(i) + 8, next);
    }
    t.set_u32(CURSOR, root);
    t.set_u8(CURSOR + 4, 0x7f);
    t.set_u8(CURSOR + 5, cursor_pass);
}

/// The ring shapes every handler test runs: (children, pass, cursor
/// pass); the first does not move.
pub const RINGS: &[(u32, u8, u8)] = &[(1, 3, 3), (0, 1, 2), (2, 1, 2), (3, 0, 9)];

/// Runs the internal handler at `f` (`ecx` = the cursor) against `port`.
pub fn check_this(
    t: &mut Twin,
    f: u32,
    port: fn(&mut super::magic::PortGame, &Image, &Frame) -> Ret,
) {
    check_this_eax(t, f, port, Eax::Checked);
}

/// [`check_this`] with the `eax` comparison given: a /GS frame's handler
/// leaves the cookie's junk in `eax` when the cursor does not move (its
/// prologue's `mov eax, [cookie]; xor eax, ebp`), which no caller reads.
pub fn check_this_eax(
    t: &mut Twin,
    f: u32,
    port: fn(&mut super::magic::PortGame, &Image, &Frame) -> Ret,
    eax: Eax,
) {
    t.check(
        f,
        Call {
            eax: 0xaabb_ccdd,
            ecx: CURSOR,
            edx: 0x55,
            ..Call::default()
        },
        &[0x0bad_f00d, 0x0bad_f00e],
        eax,
        port,
    );
}
