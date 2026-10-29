//! Functions of the `magic_0003` family (the summon overlays `0003` to
//! `0008`, `0042`, `0070`, `0071`, `0076`, `0077` and others in other
//! lists): a ring of effect nodes walked with a per-node callback, a
//! loading state machine and node set-up. Addresses are `magic_0003`'s
//! unless a comment names another DLL; constants in `.rdata` are read
//! through the instructions' operands like every other address.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Mem, Ret, Sig};

fn f32_at(g: &impl Mem, addr: u32) -> f32 {
    f32::from_bits(g.read::<u32>(addr))
}

/// The walk every ring user starts with (`this` = a cursor: `+0x0` the
/// current node, byte `+0x4` "moved", byte `+0x5` the pass number):
/// when the current node's pass byte `+0x1a` already holds this pass,
/// "not moved"; else each child of the node (the list at `+0x10`, linked
/// through `+0x8`) becomes current in turn and its handler
/// (`[[child] + 0x1c]`, cdecl `(cursor)`) runs; then the node is current
/// again, takes the pass number, and the cursor says "moved".
///
/// Returns the `eax` the walk leaves: `eax_in` (the caller's) or the last
/// handler's, with the low byte the last pass byte loaded.
fn walk<G: Game>(g: &mut G, this: u32, eax_in: u32) -> u32 {
    let node = g.read::<u32>(this);
    let pass = g.read::<u8>(node + 0x1a);
    if pass == g.read::<u8>(this + 5) {
        g.write::<u8>(this + 4, 0);
        return (eax_in & !0xff) | u32::from(pass);
    }
    let mut eax = eax_in;
    let mut child = g.read::<u32>(node + 0x10);
    if child != node {
        loop {
            g.write::<u32>(this, child);
            let cur = g.read::<u32>(this);
            child = g.read::<u32>(child + 8);
            let handler = g.read::<u32>(g.read::<u32>(cur) + 0x1c);
            eax = g.call(handler, this, &[this]).eax;
            if child == node {
                break;
            }
        }
    }
    let pass = g.read::<u8>(this + 5);
    g.write::<u32>(this, node);
    g.write::<u8>(node + 0x1a, pass);
    g.write::<u8>(this + 4, 1);
    (eax & !0xff) | u32::from(pass)
}

/// 0x10002a80 (internal; `ecx` = the cursor, `esp` the frame's entry
/// stack pointer): [`walk`], then the node's time: slot 601 with the
/// node's byte `+0x30` and a pointer to a local (holding `this` until the
/// slot writes it, from the frame's `push ecx`) gives `t`;
/// `n = (int)((t * K0 + K1) / K2)` (SSE, truncated) and the node's `+0x20`
/// becomes `+0x2c - n`. Returns `n`.
pub fn tick<G: Game>(g: &mut G, img: &Image, f: u32, this: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    // push ebp; push ecx: the local is [ebp - 4]
    let local = esp.wrapping_sub(8);
    g.write::<u32>(local, this);
    walk(g, this, 0);
    let node = g.read::<u32>(this);
    let which = u32::from(g.read::<u8>(node + 0x30));
    let slot601 = slot_target(g, w(0x4f), 601);
    g.call(slot601, 0, &[which, local]);
    let t = f32_at(g, local) * f32_at(g, w(0x64));
    let node = g.read::<u32>(this);
    let t = t + f32_at(g, w(0x71));
    let base = g.read::<u32>(node + 0x2c);
    let n = ffx_bridge::x87::cvttsd2si(f64::from(t / f32_at(g, w(0x7e))));
    g.write::<u32>(node + 0x20, base.wrapping_sub(n));
    n
}

/// `tick` (143 bytes; 32 overlays).
pub const TICK: Sig = Sig {
    len: 143,
    hash: 0x1b98_37f6_c9e5_2dc3,
    calls: &[],
    imports: &[],
};

/// 0x10002b10 (a node handler, cdecl `(cursor)`): `ecx` = the cursor and
/// a tail jump into [`tick`] (its frame starts where this one's did).
pub fn tick_handler<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let n = tick(g, img, img.rel32_target(fr.at + 8), this, fr.esp);
    Ret::eax(n)
}

/// `tick_handler` (12 bytes; 102 overlays carry the shape, those whose
/// callee is [`TICK`] bind).
pub const TICK_HANDLER: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&TICK))],
    imports: &[],
};

/// 0x10001190 in `magic_0006` (a node handler, cdecl `(cursor)`):
/// [`walk`]; when the cursor moved, the node's position `+0x1c..+0x28`
/// (4 floats) follows: towards the target `+0x2c..` by the rate `+0x4c`
/// (`v += (target - v) * rate`) when `+0x54` is set; else, while the clock
/// `+0x50` is below `K`, by the velocity `+0x3c..` (`v += vel * rate`,
/// the clock advancing by the rate), at or past it (or NaN) snapping to
/// the target. Then slot 791 gets `v[i] * K1 + origin[i] - K` for the
/// first three (`+0x5c..`). Returns slot 791's `eax`, or the walk's.
pub fn follow<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let this = fr.arg(g, 0);
    let eax = walk(g, this, fr.eax);
    if g.read::<u8>(this + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(this);
    let k = f32_at(g, w(0x53));
    let rate = |g: &G| f32_at(g, n + 0x4c);
    if g.read::<u32>(n + 0x54) != 0 {
        for i in 0..4 {
            let v = f32_at(g, n + 0x1c + 4 * i);
            let to = f32_at(g, n + 0x2c + 4 * i);
            let x = (to - v) * rate(g) + f32_at(g, n + 0x1c + 4 * i);
            g.write::<u32>(n + 0x1c + 4 * i, x.to_bits());
        }
    } else if f32_at(g, n + 0x50) >= k {
        for i in 0..4 {
            let to = g.read::<u32>(n + 0x2c + 4 * i);
            g.write::<u32>(n + 0x1c + 4 * i, to);
        }
    } else {
        for i in 0..4 {
            let x = f32_at(g, n + 0x3c + 4 * i) * rate(g) + f32_at(g, n + 0x1c + 4 * i);
            g.write::<u32>(n + 0x1c + 4 * i, x.to_bits());
        }
        let clock = rate(g) + f32_at(g, n + 0x50);
        g.write::<u32>(n + 0x50, clock.to_bits());
    }
    let k1 = f32_at(g, w(0x14d));
    let slot791 = slot_target(g, w(0x15e), 791);
    let a2 = f32_at(g, n + 0x24) * k1 + f32_at(g, n + 0x64) - k;
    let a1 = f32_at(g, n + 0x20) * k1 + f32_at(g, n + 0x60) - k;
    let a0 = f32_at(g, n + 0x1c) * k1 + f32_at(g, n + 0x5c) - k;
    let r = g.call(slot791, 0, &[a0.to_bits(), a1.to_bits(), a2.to_bits()]);
    Ret::eax(r.eax)
}

/// `follow` (431 bytes; 11 overlays).
pub const FOLLOW: Sig = Sig {
    len: 431,
    hash: 0x1c6b_59b5_157b_857a,
    calls: &[],
    imports: &[],
};

/// 0x10006ab0 (a callback, no arguments): the loading steps, `G_STEP`
/// 0..3, each run until it is not ready: 0 waits for slot 225(2) to say
/// 0, 1 for slot 313(2), 2 for slot 549(2) and then starts slot 226
/// (`G_FILE`, 1); 3 waits for slot 227(`G_FILE`) to reach 1 and ends at
/// step 4. Returns 0.
pub fn load_steps<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut step = g.read::<u32>(w(0x01));
    loop {
        match step {
            0..=2 => {
                let (t, slot) = [(0x18, 225), (0x34, 313), (0x4c, 549)][step as usize];
                let s = slot_target(g, w(t), slot);
                if g.call(s, 0, &[2]).eax != 0 {
                    return Ret::eax(0);
                }
                if step == 2 {
                    let s = slot_target(g, w(0x62), 226);
                    let file = g.read::<u32>(w(0x6a));
                    g.call(s, 0, &[file, 1]);
                }
                step = g.read::<u32>(w(0x7a)).wrapping_add(1);
                g.write::<u32>(w(0x80), step);
                if step > 3 {
                    return Ret::eax(0);
                }
            }
            3 => {
                let s = slot_target(g, w(0x8d), 227);
                let file = g.read::<u32>(w(0x93));
                if g.call(s, 0, &[file]).eax.cast_signed() >= 1 {
                    let v = g.read::<u32>(w(0xa9));
                    g.write::<u32>(w(0xa9), v.wrapping_add(1));
                }
                return Ret::eax(0);
            }
            _ => return Ret::eax(0),
        }
    }
}

/// `load_steps` (176 bytes and its 4-entry jump table; 22 overlays).
pub const LOAD_STEPS: Sig = Sig {
    len: 192,
    hash: 0x93c3_a521_1509_b1db,
    calls: &[],
    imports: &[],
};

/// 0x100030f0 in `magic_0004` (internal; `ecx` = `[node, parent,
/// value]`): sets up a node: its handler table `+0x0`, pass byte
/// `+0x1a` 0xff (as a u16), an empty child list (`+0xc`, `+0x10` itself),
/// the parent `+0x14`; with a parent, appended to the parent's child list
/// (`+0x4`/`+0x8` links, the parent's `+0xc` first and `+0x10` last
/// child, u16 count `+0x18`); then scale `+0x28` 1.0, `+0x38` the value,
/// byte `+0x40` 1, `+0x3c` 6, `+0x2c..+0x34` 0. Returns the value.
pub fn node_init(g: &mut impl Mem, img: &Image, f: u32, args: u32) -> u32 {
    let node = g.read::<u32>(args);
    let parent = g.read::<u32>(args + 4);
    g.write::<u32>(node, img.word(f + 0x08));
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
    let value = g.read::<u32>(args + 8);
    g.write::<u32>(node + 0x28, 1.0f32.to_bits());
    g.write::<u32>(node + 0x38, value);
    g.write::<u8>(node + 0x40, 1);
    g.write::<u32>(node + 0x3c, 6);
    g.write::<u32>(node + 0x2c, 0);
    g.write::<u32>(node + 0x30, 0);
    g.write::<u32>(node + 0x34, 0);
    value
}

/// `node_init` (109 bytes; 10 overlays).
pub const NODE_INIT: Sig = Sig {
    len: 109,
    hash: 0x0df8_ba43_620a_39e9,
    calls: &[],
    imports: &[],
};
