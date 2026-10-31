//! `magic_0512.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Inserts a model node (0x10003680, internal: `ecx` = a group of words
/// `[node, parent, a, b, c, d]`): as `magic_0482`'s
/// [`insert`](crate::m0482::insert), with `+0x154` = `c` and `+0x150` =
/// `d` (the operands nine bytes further on). Returns slot 325's `eax`.
pub fn insert<G: Game>(g: &mut G, img: &Image, f: u32, group: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let parent = g.read::<u32>(group + 4);
    let node = g.read::<u32>(group);
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
        let n = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, n.wrapping_add(1));
        g.write::<u32>(parent + 0x10, node);
    }
    for (src, dst) in [(8u32, 0x13cu32), (0x10, 0x154), (0xc, 0x140), (0x14, 0x150)] {
        let v = g.read::<u32>(group + src);
        g.write::<u32>(node + dst, v);
    }
    g.write::<u32>(node + 0x144, w(0x68));
    g.write::<u32>(node + 0x128, w(0x72));
    g.write::<u8>(node + 0x148, 0);
    g.write::<u32>(node + 0x14c, ONE);
    g.write::<u32>(node + 0xac, 0);
    g.write::<u32>(node + 0xb0, ONE);
    for off in [0xb4, 0xb8, 0x124, 0x120, 0x11c] {
        g.write::<u32>(node + off, 0);
    }
    let m = node + 0x5c;
    let t = slot_target(g, w(0x5e), 235);
    g.call(t, 0, &[m]);
    let t = slot_target(g, w(0xe2), 230);
    g.call(t, 0, &[node + 0xdc, m, m]);
    let t = slot_target(g, w(0xf4), 230);
    g.call(t, 0, &[node + 0x1c, m, node + 0xdc, m, m]);
    let t = slot_target(g, w(0x10d), 325);
    g.call(
        t,
        0,
        &[node + 0x12c, w(0x107), node + 0x1c, m, node + 0xdc, m, m],
    )
    .eax
}

/// [`insert`] (287 bytes, 1 overlay).
pub const INSERT: Sig = Sig {
    len: 287,
    hash: 0xbfb0_02c9_f7ac_6b30,
    calls: &[],
    imports: &[],
};

/// A task (0x100088b0, `(task)`, pushed by 0x1000899a; a frame with
/// locals) on the record `r = task[1]`, by its state `+0x18`:
/// - 0: state 1, a wait `+0x1c` of 8; 1: while the wait runs, it waits;
///   then a particle node ([`particle_node`](crate::sb_nodes::particle_node))
///   from the local record `[r + 0x1f0, LIST, r + 0x20, *G_O, PART, 5]`
///   (`ebp - 0x1c`; the node's vtable `+0x63` first) and state 2;
/// - 2: once bit 2 of `+0x2d8` is set, state 10000 and on;
/// - 10000: the task's node `*task` leaves its ring (linked to itself),
///   state 20000.
///
/// The wait goes down at every exit. Returns the particle node's `eax`,
/// the node's next after the unlink, or the state's remainder.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let state = g.read::<u32>(r + 0x18);
    let eax = match state {
        0 | 1 => {
            if state == 0 {
                g.write::<u32>(r + 0x18, 1);
                g.write::<u32>(r + 0x1c, 8);
            }
            if g.read::<i32>(r + 0x1c) > 0 {
                0
            } else {
                let rec = fr.esp.wrapping_sub(0x20);
                let node = r + 0x1f0;
                g.write::<u32>(rec + 8, r + 0x20);
                let o = g.read::<u32>(w(0x5a));
                g.write::<u32>(rec, node);
                g.write::<u32>(node, w(0x63));
                g.write::<u32>(rec + 4, w(0x6d));
                g.write::<u32>(rec + 0xc, o);
                g.write::<u32>(rec + 0x10, w(0x77));
                g.write::<u32>(rec + 0x14, 5);
                let eax = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x83), rec);
                let s = g.read::<u32>(r + 0x18);
                g.write::<u32>(r + 0x18, s.wrapping_add(1));
                eax
            }
        }
        2 if g.read::<u8>(r + 0x2d8) & 4 == 0 => 2,
        2 | 0x2710 => {
            g.write::<u32>(r + 0x18, 0x2710);
            let node = g.read::<u32>(task);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node + 8);
            g.write::<u32>(next + 8, prev);
            let prev = g.read::<u32>(node + 8);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            g.write::<u32>(r + 0x18, 0x4e20);
            next
        }
        _ => state,
    };
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`task`] (194 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 194,
    hash: 0x5b27_20e2_f49f_5de0,
    calls: &[(0x83, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE))],
    imports: &[],
};
