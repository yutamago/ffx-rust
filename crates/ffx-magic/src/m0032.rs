//! Helpers of the `magic_0032` family (`0032`, `0040`, `0046`, `0048`,
//! `0050`, `0054` and others): internal functions with LTCG register
//! conventions, called only from the overlays' own code. Addresses are
//! `magic_0032`'s.

use crate::{Image, Sig};

/// Takes the head of a free list of 0x1c-byte nodes (`magic_0032`
/// 0x10002200; `G_HEAD` 0x101c8f24, `G_COUNT` u16 0x101c8f28): none left
/// gives 0; else the node is unlinked from its ring (`prev.next = next`,
/// `next.prev = prev`, words `+0x4` and `+0x0`), the head moves to its
/// next (or 0 when it was the last), the count drops, and the node is
/// reset: links and `+0xc`/`+0x14` zero, `+0x8` itself, `+0x10` the
/// family's default callback (an address in the DLL), byte `+0x18` 0.
/// Returns the node (`eax`; `edx` too).
pub fn pop_node(g: &mut impl crate::Mem, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let count = g.read::<u16>(w(0x04));
    if count == 0 {
        return 0;
    }
    let node = g.read::<u32>(w(0x13));
    let prev = g.read::<u32>(node);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 4, next);
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node);
    g.write::<u32>(next, prev);
    if count > 1 {
        let next = g.read::<u32>(node + 4);
        g.write::<u32>(w(0x30), next);
    } else {
        g.write::<u32>(w(0x38), 0);
    }
    g.write::<u16>(w(0x4b), count.wrapping_sub(1));
    g.write::<u32>(node, 0);
    g.write::<u32>(node + 4, 0);
    g.write::<u32>(node + 0xc, 0);
    g.write::<u32>(node + 0x14, 0);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 0x10, w(0x70));
    g.write::<u8>(node + 0x18, 0);
    node
}

/// `pop_node` (124 bytes, 15 overlays).
pub const POP_NODE: Sig = Sig {
    len: 124,
    hash: 0xe49b_1b56_333b_21b3,
    calls: &[],
    imports: &[],
};

/// Resets an effect object's state block (`magic_0032` 0x10001a10,
/// `ecx` = `this`, two stack words of which only the second is read,
/// cdecl): `+0x100` = the argument, `+0x114` = `x114` (0, or `edx` in
/// `magic_0074`'s copy); `+0x104`..`+0x120` zero except `+0x11c` =
/// `count` (0x14 in `0032`, 0 in `0046`, 0x12 in `0074`); byte `+0x126` 0,
/// u16 `+0x124` 0x8000; `+0xc0`..`+0xe8` zero; the four scales
/// `+0xdc`, `+0xec`..`+0xfc` 1.0.
pub fn reset_state(g: &mut impl crate::Mem, this: u32, arg: u32, x114: u32, count: u32) {
    let one = 1.0f32.to_bits();
    g.write::<u32>(this + 0x100, arg);
    g.write::<u32>(this + 0x114, x114);
    g.write::<u32>(this + 0x11c, count);
    for off in [0x104, 0x108, 0x10c, 0x110, 0x118, 0x120] {
        g.write::<u32>(this + off, 0);
    }
    g.write::<u8>(this + 0x126, 0);
    g.write::<u16>(this + 0x124, 0x8000);
    for off in [0xcc, 0xc8, 0xc4, 0xc0, 0xd8, 0xd4, 0xd0] {
        g.write::<u32>(this + off, 0);
    }
    g.write::<u32>(this + 0xdc, one);
    for off in [0xe8, 0xe4, 0xe0] {
        g.write::<u32>(this + off, 0);
    }
    for off in [0xec, 0xfc, 0xf8, 0xf4, 0xf0] {
        g.write::<u32>(this + off, one);
    }
}

/// `reset_state` with `+0x11c` = 0x14 (273 bytes, 7 overlays).
pub const RESET_STATE: Sig = Sig {
    len: 273,
    hash: 0x46df_f7f9_8a99_2933,
    calls: &[],
    imports: &[],
};

/// `reset_state` with `+0x11c` = 0 (`magic_0046` 0x10001a10).
pub const RESET_STATE_0: Sig = Sig {
    len: 273,
    hash: 0x389b_acfc_4272_2df7,
    calls: &[],
    imports: &[],
};

/// `reset_state` with `+0x114` = `edx`, `+0x11c` = 0x12 (`magic_0074`
/// 0x10001b60, 269 bytes; 5 overlays).
pub const RESET_STATE_EDX: Sig = Sig {
    len: 269,
    hash: 0xc517_ca09_4079_8dbb,
    calls: &[],
    imports: &[],
};
