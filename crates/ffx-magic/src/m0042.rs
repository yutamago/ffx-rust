//! `magic_0042.dll`'s functions shared with other overlays (the
//! `magic_0003` family's node code in another shape, and small
//! wrappers). Internal functions with register arguments; addresses are
//! `magic_0042`'s.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Mem, Ret, Sig};

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

/// 0x100035c0 (internal; `ecx` = `[node, parent, d, a, b, c]`): the
/// [`crate::m0070::node_init_big`] shape with one more word: a fresh node
/// under the parent (when not 0), `+0x13c` = a, `+0x144` = b, `+0x140` =
/// c, `+0x128` = d, byte `+0x148` 0. Returns d.
pub fn node_init_big(g: &mut impl Mem, args: u32) -> u32 {
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
    for (src, dst) in [(0xc, 0x13c), (0x10, 0x144), (0x14, 0x140)] {
        let v = g.read::<u32>(args + src);
        g.write::<u32>(node + dst, v);
    }
    let d = g.read::<u32>(args + 8);
    g.write::<u32>(node + 0x128, d);
    g.write::<u8>(node + 0x148, 0);
    d
}

/// `node_init_big` (101 bytes; 3 overlays).
pub const NODE_INIT_BIG: Sig = Sig {
    len: 101,
    hash: 0xd2e7_2a94_5455_2199,
    calls: &[],
    imports: &[],
};

/// 0x10003de0 (internal; `ecx` = `[effect, out]`, `esp` the frame's
/// entry stack pointer): clears `out[0]` and `out[2]`, takes the
/// effect's time `t` (with bit 0 of byte `+0x90`, slot 333 (model
/// `+0x54`) in ST0, stored as f32; else slot 601 (`+0x50`, &local) into
/// the frame's local, which holds `ecx` until the slot writes it) and
/// sets `out[1]` = `(int)((K0 - t) * K1 / K2)` (SSE, truncated). Returns
/// `out`.
pub fn time_left<G: Game>(g: &mut G, img: &Image, f: u32, this: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    // push ebp; push ecx: the local is [ebp - 4]
    let local = esp.wrapping_sub(8);
    g.write::<u32>(local, this);
    let out = g.read::<u32>(this + 4);
    let obj = g.read::<u32>(this);
    g.write::<u32>(out + 8, 0);
    g.write::<u32>(out, 0);
    if g.read::<u8>(obj + 0x90) & 1 != 0 {
        let s333 = slot_target(g, w(0x23), 333);
        let m = g.read::<u32>(obj + 0x54);
        let r = g.call(s333, 0, &[m]);
        #[allow(clippy::cast_possible_truncation)] // fstp dword: round to f32
        let t = r.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32);
        g.write::<u32>(local, t.to_bits());
    } else {
        let s601 = slot_target(g, w(0x3f), 601);
        let p = g.read::<u32>(obj + 0x50);
        g.call(s601, 0, &[p, local]);
    }
    let f32_at = |g: &G, a: u32| f32::from_bits(g.read::<u32>(a));
    let x = f32_at(g, w(0x55)) - f32_at(g, local);
    let out = g.read::<u32>(this + 4);
    let x = x * f32_at(g, w(0x66)) / f32_at(g, w(0x6e));
    let n = ffx_bridge::x87::cvttsd2si(f64::from(x));
    g.write::<u32>(out + 4, n);
    out
}

/// `time_left` (125 bytes; 4 overlays).
pub const TIME_LEFT: Sig = Sig {
    len: 125,
    hash: 0x58c1_32f9_d5c6_02d1,
    calls: &[],
    imports: &[],
};

/// A group's visit (0x10001000; `ecx` = the group, as a vtable method or
/// through the thunk at 0x100010b0): the node walk
/// ([`crate::nodes::walk`]); then, when the owner's byte `+0x1c` is set,
/// slot 245 `(&t)` (a local, `ebp - 4`) and slot 581 `(G_POOL, 0, 0x1000,
/// 0, &t)` before `G_POOL` (operand `+0x5e`) moves on by 8. Returns slot
/// 581's result, or the owner.
pub fn visit_pool<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let this = fr.ecx;
    let _ = crate::nodes::walk(g, this, this, fr.eax);
    let owner = g.read::<u32>(this);
    if g.read::<u8>(owner + 0x1c) == 0 {
        return Ret::eax(owner);
    }
    let t = fr.esp - 8;
    let s245 = slot_target(g, w(0x50), 245);
    g.call(s245, this, &[t]);
    let pool = g.read::<u32>(w(0x5e));
    let next = pool.wrapping_add(8);
    let s581 = slot_target(g, w(0x75), 581);
    // (next - pool) >> 4: always 0
    // call width: 5 (four words and slot 245's)
    let r = g
        .call(s581, pool, &[pool, (next - pool) >> 4, 0x1000, 0, t])
        .eax;
    g.write::<u32>(w(0x87), next);
    Ret::eax(r)
}

/// `visit_pool` (145 bytes, 1 overlay).
pub const VISIT_POOL: Sig = Sig {
    len: 145,
    hash: 0xaed3_58bf_e89b_31f2,
    calls: &[],
    imports: &[],
};

/// Builds an effect node's three children (0x10003960, internal: `ecx`
/// = `[node, parent, d, _, _, _, a, b, c, e]`, the [`node_init_big`]
/// arguments and four more words): the children at `node + 0x1d4` (table
/// `T1`, operand `+0x0f`), `+0x1a8` and `+0x17c` (table `T2`, `+0x67`,
/// `+0x8c`) start as empty lists with pass byte 0xff; `args[3..6]` get
/// their addresses; then [`node_init_big`], the three linked under the
/// node in that order, the node's words from `args[6..10]`, its fixed
/// words, slot 281 `(1)`, slot 869 `(0)` into `+0x170` and the handler
/// table `T3` (`+0x1bf`) into the node. Returns the node.
pub fn build<G: Game>(g: &mut G, img: &Image, f: u32, args: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let node = g.read::<u32>(args);
    let (c1, c2, c3) = (node + 0x1d4, node + 0x1a8, node + 0x17c);
    g.write::<u32>(c1, w(0x0f));
    g.write::<u16>(node + 0x1ee, 0xff);
    g.write::<u32>(node + 0x1e8, 0);
    g.write::<u32>(c1 + 0x10, c1);
    g.write::<u32>(c1 + 0xc, c1);
    g.write::<u32>(args + 0xc, c3);
    g.write::<u32>(args + 0x10, c1);
    for (c, t) in [(c2, 0x67u32), (c3, 0x8c)] {
        g.write::<u16>(c + 0x1a, 0xff);
        g.write::<u32>(c + 0x10, c);
        g.write::<u32>(c + 0xc, c);
        g.write::<u32>(c + 0x14, 0);
        g.write::<u32>(c, w(t));
    }
    g.write::<u32>(args + 0x14, c2);
    node_init_big(g, args);
    for c in [c1, c2, c3] {
        g.write::<u32>(c + 4, node);
        let last = g.read::<u32>(node + 0x10);
        g.write::<u32>(c + 8, last);
        let last = g.read::<u32>(node + 0x10);
        g.write::<u32>(last + 4, c);
        g.write::<u32>(node + 0x10, c);
        g.write::<u32>(c + 0x14, node);
        let n = g.read::<u16>(node + 0x18).wrapping_add(1);
        g.write::<u16>(node + 0x18, n);
    }
    for (dst, src) in [
        (0x15cu32, 0x1cu32),
        (0x164, 0x18),
        (0x160, 0x24),
        (0x168, 0x20),
    ] {
        let v = g.read::<u32>(args + src);
        g.write::<u32>(node + dst, v);
    }
    for (o, v) in [
        (0x1f8u32, 0u32),
        (0x1f4, 0),
        (0x1f0, 0),
        (0x1fc, 1),
        (0x1a0, 0),
        (0x19c, 0),
        (0x198, 0),
        (0x1a4, 0x3f80_0000),
        (0x1d0, 0x3f80_0000),
        (0x1cc, 0x3f80_0000),
        (0x1c8, 0x3f80_0000),
        (0x1c4, 0x3f80_0000),
    ] {
        g.write::<u32>(node + o, v);
    }
    g.write::<u8>(node + 0x178, 0);
    let s281 = slot_target(g, w(0x110), 281);
    g.call(s281, node, &[1]);
    let s869 = slot_target(g, w(0x19c), 869);
    let r = g.call(s869, node, &[0]).eax;
    g.write::<u32>(node + 0x170, r);
    let node = g.read::<u32>(args);
    g.write::<u32>(node, w(0x1bf));
    node
}

/// `build` (456 bytes, 1 overlay).
pub const BUILD: Sig = Sig {
    len: 456,
    hash: 0x3331_043d_57eb_6c8f,
    calls: &[(0x94, Callee::Sig(&NODE_INIT_BIG))],
    imports: &[],
};
