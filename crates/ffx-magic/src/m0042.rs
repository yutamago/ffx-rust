//! `magic_0042.dll`'s functions shared with other overlays (the
//! `magic_0003` family's node code in another shape, and small
//! wrappers). Internal functions with register arguments; addresses are
//! `magic_0042`'s.

use crate::runtime::slot_target;
use crate::{Game, Image, Mem, Sig};

/// 0x100034d0 (internal; `ecx` = `[node, parent, a, b]`): the
/// [`crate::m0003::node_init`] of this family: pass byte `+0x1a` 0xff (as
/// a u16), an empty child list, the parent `+0x14` and the link into the
/// parent's children; then scale `+0x28` 1.0, `+0x3c` = a, `+0x40` = b,
/// `+0x2c..+0x38` 0 and the handler table `+0x0` last. Returns the
/// handler table.
pub fn node_init(g: &mut impl Mem, img: &Image, f: u32, args: u32) -> u32 {
    let node = g.read::<u32>(args);
    let parent = g.read::<u32>(args + 4);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        g.write::<u32>(node + 4, parent);
        let last = g.read::<u32>(parent + 0x10);
        g.write::<u32>(node + 8, last);
        let last = g.read::<u32>(parent + 0x10);
        if last == parent {
            g.write::<u32>(parent + 0xc, node);
        } else {
            g.write::<u32>(last + 4, node);
        }
        let count = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, count.wrapping_add(1));
        g.write::<u32>(parent + 0x10, node);
    }
    let a = g.read::<u32>(args + 8);
    g.write::<u32>(node + 0x28, 1.0f32.to_bits());
    g.write::<u32>(node + 0x3c, a);
    let b = g.read::<u32>(args + 0xc);
    g.write::<u32>(node + 0x40, b);
    for off in [0x2c, 0x30, 0x34, 0x38] {
        g.write::<u32>(node + off, 0);
    }
    // the last store's immediate; eax still holds b
    g.write::<u32>(node, img.word(f + 0x69));
    b
}

/// `node_init` (111 bytes; 9 overlays).
pub const NODE_INIT: Sig = Sig {
    len: 111,
    hash: 0x37a8_01d4_7a57_f853,
    calls: &[],
    imports: &[],
};

/// 0x1000be50 (internal; `ecx` = a handle): slot 670 (handle, 0).
pub fn release<G: Game>(g: &mut G, img: &Image, f: u32, handle: u32) -> u32 {
    let s = slot_target(g, img.word(f + 0x01), 670);
    g.call(s, 0, &[handle, 0]).eax
}

/// `release` (20 bytes; 5 overlays).
pub const RELEASE: Sig = Sig {
    len: 20,
    hash: 0x7a3c_219a_2197_d48f,
    calls: &[],
    imports: &[],
};

/// 0x10008400 (internal; `ecx` = an effect object, `edx` = a value):
/// an object with bit 0 of u16 `+0x90` restarts its model (slot 30 of
/// `+0x54`) and gives it the value (slot 36 (`+0x54`, value); the first
/// call's word stays on the stack) and sets bit 9; else slot 228
/// (`+0x50`, value). Either way `+0x60` = the value and bit 8 is set.
/// Returns 0x100.
pub fn set_value<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, value: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let table = w(0x01);
    if g.read::<u8>(obj + 0x90) & 1 != 0 {
        let s30 = slot_target(g, table, 30);
        let model = g.read::<u32>(obj + 0x54);
        g.call(s30, 0, &[model]);
        let s36 = slot_target(g, w(0x1d), 36);
        let model2 = g.read::<u32>(obj + 0x54);
        g.call(s36, 0, &[model2, value, model]);
        let bits = g.read::<u16>(obj + 0x90) | 0x200;
        g.write::<u16>(obj + 0x90, bits);
        let bits = g.read::<u16>(obj + 0x90) | 0x100;
        g.write::<u16>(obj + 0x90, bits);
        g.write::<u32>(obj + 0x60, value);
    } else {
        let s228 = slot_target(g, table, 228);
        let x = g.read::<u32>(obj + 0x50);
        g.call(s228, 0, &[x, value]);
        g.write::<u32>(obj + 0x60, value);
        let bits = g.read::<u16>(obj + 0x90) | 0x100;
        g.write::<u16>(obj + 0x90, bits);
    }
    0x100
}

/// `set_value` (111 bytes; 11 overlays).
pub const SET_VALUE: Sig = Sig {
    len: 111,
    hash: 0xf754_a4ac_964f_f8b9,
    calls: &[],
    imports: &[],
};

/// 0x10005cd0 (internal; `ecx` = a pointer to the node): takes the node
/// out of its parent's children (the u16 count `+0x18` drops; the
/// parent's first `+0xc` / last `+0x10` child or the neighbours' links
/// `+0x4`/`+0x8` close the gap), leaves it linked to itself with no
/// parent, then releases its part (slot 779 (`+0x20`, 0)) with the game
/// flag `*G_LOCK` set. Returns the flag's address.
pub fn unlink<G: Game>(g: &mut G, img: &Image, f: u32, this: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let node = g.read::<u32>(this);
    let parent = g.read::<u32>(node + 0x14);
    if parent != 0 {
        let count = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, count.wrapping_sub(1));
        let prev = g.read::<u32>(node + 4);
        let next = g.read::<u32>(node + 8);
        if prev == parent {
            g.write::<u32>(parent + 0x10, next);
        } else {
            g.write::<u32>(prev + 8, next);
        }
        let next = g.read::<u32>(node + 8);
        let prev = g.read::<u32>(node + 4);
        if next == parent {
            g.write::<u32>(parent + 0xc, prev);
        } else {
            g.write::<u32>(next + 4, prev);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 4, node);
        g.write::<u32>(node + 0x14, 0);
    }
    let lock = g.read::<u32>(w(0x45));
    let part = g.read::<u32>(node + 0x20);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s779 = slot_target(g, w(0x55), 779);
    g.call(s779, 0, &[part, 0]);
    let lock = g.read::<u32>(w(0x65));
    g.write::<u32>(lock, saved);
    lock
}

/// `unlink` (112 bytes; 6 overlays).
pub const UNLINK: Sig = Sig {
    len: 112,
    hash: 0x38cd_870b_a1de_d501,
    calls: &[],
    imports: &[],
};
