//! `magic_0513.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::h6_setup::{mode_walk, ring_join, static_child, unlink_task};
use crate::m0018::st0_f32;
use crate::nodes::node_insert;
use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::sb_sse::{cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's end (0x100089a0, an overlay table entry, no arguments):
/// the node `*G_N` (operand `+0x02`), when there is one, leaves its
/// parent's child list (the parent's count `+0x18` down; the list's own
/// first/last words `+0xc`/`+0x10` or the neighbours' links joined, the
/// neighbours read from the DLL's link globals `+0x15`/`+0x1a`), the
/// links reset to the list object, `*G_N` 0; then slot 250 `(0, 0)`.
/// Returns 0.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let n = g.read::<u32>(w(0x2));
    if n != 0 {
        let c = g.read::<u16>(n + 0x18);
        g.write::<u16>(n + 0x18, c.wrapping_add(0xffff));
        let next = g.read::<u32>(w(0x15));
        let prev = g.read::<u32>(w(0x1a));
        if next == n {
            g.write::<u32>(n + 0x10, prev);
        } else {
            g.write::<u32>(next + 8, prev);
        }
        let prev = g.read::<u32>(w(0x2c));
        let next = g.read::<u32>(w(0x31));
        if prev == n {
            g.write::<u32>(n + 0xc, next);
        } else {
            g.write::<u32>(prev + 4, next);
        }
        g.write::<u32>(w(0x43), w(0x47));
        g.write::<u32>(w(0x4d), w(0x51));
        g.write::<u32>(w(0x57), 0);
    }
    let t = slot_target(g, w(0x60), 250);
    g.call(t, 0, &[0, 0]);
    Ret::eax(0)
}

/// [`end`] (132 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 132,
    hash: 0x809f_0171_8f1f_eb70,
    calls: &[],
    imports: &[],
};

/// A task (0x10007090, `(task)`, a task step; a frame with locals) on
/// the record `r = task[1]`, by its state `+0x18`:
/// - 0: state 1, a wait `+0x1c` of 38; 1: while the wait runs, it waits;
///   then the DLL's flag byte (operand `+0x61`) gets bit 0, its light
///   block `+0x83..+0xc9` = `(0, 0, 1, 2, 15, 15, 0, -K)` (the zeros as
///   `0 - 0` and `0 - K` in SSE), a particle node
///   ([`particle_node`](crate::sb_nodes::particle_node)) from the local
///   record `[r + 0x1f0, LIST, r + 0x20, *G_O, PART, 3]` (`ebp - 0x1c`;
///   the node's vtable `+0xda` first), slot 306 `(7, 0, slot 719 (NAME,
///   5, 0))` and state 2;
/// - 2: once bit 2 of `+0x2d8` is set, state 10000 and on;
/// - 10000: the task's node `*task` leaves its ring (linked to itself),
///   state 20000.
///
/// The wait goes down at every exit. Returns slot 306's `eax`, the
/// node's next after the unlink, or what is left of the state.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let state = g.read::<u32>(r + 0x18);
    let eax = match state {
        0 => {
            g.write::<u32>(r + 0x18, 1);
            g.write::<u32>(r + 0x1c, 0x26);
            0
        }
        1 if g.read::<i32>(r + 0x1c) > 0 => 0,
        1 => {
            let flag = g.read::<u8>(w(0x61));
            g.write::<u8>(w(0x61), flag | 1);
            let zero = sub(0.0, 0.0);
            let k = g.read::<f32>(w(0x74));
            let neg = sub(0.0, k);
            for (op, v) in [
                (0x83u32, 0u32),
                (0x8f, zero.to_bits()),
                (0x95, 0x3f80_0000),
                (0x9f, 0x4000_0000),
                (0xa9, 0x4170_0000),
                (0xb3, 0x4170_0000),
                (0xbd, 0),
                (0xc9, neg.to_bits()),
            ] {
                g.write::<u32>(w(op), v);
            }
            let rec = fr.esp.wrapping_sub(0x20);
            let node = r + 0x1f0;
            g.write::<u32>(rec + 8, r + 0x20);
            let o = g.read::<u32>(w(0xd1));
            g.write::<u32>(rec, node);
            g.write::<u32>(node, w(0xda));
            g.write::<u32>(rec + 4, w(0xe4));
            g.write::<u32>(rec + 0xc, o);
            g.write::<u32>(rec + 0x10, w(0xee));
            g.write::<u32>(rec + 0x14, 3);
            crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0xfa), rec);
            let t = slot_target(g, w(0xff), 719);
            let h = g.call(t, 0, &[w(0x10e), 5, 0]).eax;
            let t = slot_target(g, w(0x119), 306);
            let eax = g.call(t, 0, &[7, 0, h]).eax;
            let s = g.read::<u32>(r + 0x18);
            g.write::<u32>(r + 0x18, s.wrapping_add(1));
            eax
        }
        2 if g.read::<u8>(r + 0x2d8) & 4 == 0 => 2,
        2 | 0x2710 => {
            if state == 2 {
                g.write::<u32>(r + 0x18, 0x2710);
            }
            let next = unlink_task(g, task);
            g.write::<u32>(r + 0x18, 0x4e20);
            next
        }
        _ => state,
    };
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`task`] (359 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 359,
    hash: 0xb1ba_0742_de63_034e,
    calls: &[(0xfa, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE))],
    imports: &[],
};

/// Sets up the effect's object over a model node (0x10007200, internal:
/// `ecx` = the object, `edx` a vector, one stack word `node`): the
/// object joins the DLL's ring ([`ring_join`]); children of the DLL's
/// node list `b = obj + 0x16c` (handlers `+0xa0`), `c = obj + 0x1c4`
/// (`+0xcf`, set first) and `d = obj + 0x198` (`+0x1c8`); slot 325
/// copies `edx` to `+0x188`, `+0x18c` drops by `K`; unless the node's
/// mode byte `+0x1a` is the DLL's mode (operand `+0x110`) already, each
/// child of the node gets its vtable's `+0x1c` method with the local
/// group `[child, ..., mode]` (`ebp - 0xc`, the mode byte at `ebp - 7`)
/// and the node takes the mode; the node's `+0x1c..+0x24` to
/// `+0x1e0..`, `+0x1b4..+0x1c0` 1.0; the node insert ([`node_insert`]) of
/// `[obj + 0x20, LIST, NAME, b, c, d]` on the frame (`ebp - 0x24`);
/// `+0x18` 0. Returns the insert's `eax`.
pub fn over_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let node = fr.arg(g, 0);
    ring_join(g, img, f, obj, [0x13, 0x1a, 0x22, 0x2c, 0x31, 0x40, 0x47]);
    let b = obj + 0x16c;
    static_child(
        g,
        img,
        f,
        b,
        [0x5a, 0x61, 0x66, 0x6e, 0x73, 0x7b, 0x87, 0x9a],
    );
    let t = slot_target(g, w(0x94), 325);
    g.write::<u32>(b, w(0xa0));
    g.call(t, 0, &[obj + 0x188, fr.edx]);
    let x = sub(g.read::<f32>(obj + 0x18c), g.read::<f32>(w(0xb8)));
    g.write::<f32>(obj + 0x18c, x);
    let c = obj + 0x1c4;
    g.write::<u32>(c, w(0xcf));
    static_child(
        g,
        img,
        f,
        c,
        [0xe2, 0xe9, 0xee, 0xf6, 0xfb, 0x103, 0x117, 0x11d],
    );
    let ebp = fr.esp.wrapping_sub(4);
    let mode = g.read::<u8>(w(0x110));
    mode_walk(g, ebp.wrapping_sub(0xc), node, mode);
    for (k, o) in [(0x1cu32, 0x1e0u32), (0x20, 0x1e4), (0x24, 0x1e8)] {
        let v = g.read::<u32>(node + k);
        g.write::<u32>(obj + o, v);
    }
    let d = obj + 0x198;
    static_child(
        g,
        img,
        f,
        d,
        [0x18f, 0x196, 0x19b, 0x1a3, 0x1a8, 0x1b0, 0x1bc, 0x1c2],
    );
    g.write::<u32>(d, w(0x1c8));
    for o in [0x1b4u32, 0x1b8, 0x1bc, 0x1c0] {
        g.write::<u32>(obj + o, 0x3f80_0000);
    }
    let group = ebp.wrapping_sub(0x24);
    for (i, v) in (0u32..).zip([obj + 0x20, w(0x203), w(0x20a), b, c, d]) {
        g.write::<u32>(group + 4 * i, v);
    }
    let r = node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x215),
            // push ebp; sub esp, 0x28; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x38),
            ecx: group,
            ..Frame::default()
        },
    );
    g.write::<u32>(obj + 0x18, 0);
    r.eax
}

/// [`over_node`] (551 bytes, 1 overlay).
pub const OVER_NODE: Sig = Sig {
    len: 551,
    hash: 0x5394_7c77_b0df_12b0,
    calls: &[(0x215, Callee::Sig(&crate::nodes::INSERT))],
    imports: &[],
};

/// Sets up the effect's object between two model nodes (0x10007430,
/// internal: `ecx` = the object, `edx` = node `p`, one stack word node
/// `q`): the object joins the DLL's ring ([`ring_join`]); children of the
/// DLL's node list `b = obj + 0x16c` (handlers `+0x9a`), `c = obj +
/// 0x1c4` (`+0x118`, set first) and `d = obj + 0x198` (`+0x20f`); `p`
/// and then `q` take the DLL's mode (operand `+0x87`, then `+0x159`)
/// through the walk of their children ([`walk`](crate::nodes::walk), the
/// local group at `ebp - 0xc`); slot 325 copies `p + 0x1c` to `+0x188`,
/// `+0x18c` drops by `K`; `q`'s `+0x1c..+0x24` to `+0x1e0..`,
/// `+0x1b4..+0x1c0` 1.0; the node insert ([`node_insert`]) of `[obj +
/// 0x20, LIST, NAME, b, c, d]` and a particle node
/// ([`particle_node`](crate::sb_nodes::particle_node)) of `[obj + 0x1f0,
/// LIST2, obj + 0x20, *G_O, PART, 2]` (vtable `+0x276` first), both on
/// the frame (`ebp - 0x24`); `+0x18` 0. Returns the particle node's
/// `eax`.
pub fn between<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let p = fr.edx;
    let q = fr.arg(g, 0);
    ring_join(g, img, f, obj, [0x15, 0x1c, 0x24, 0x2e, 0x33, 0x42, 0x49]);
    let b = obj + 0x16c;
    static_child(
        g,
        img,
        f,
        b,
        [0x5c, 0x63, 0x68, 0x70, 0x75, 0x7d, 0x8e, 0x94],
    );
    let ebp = fr.esp.wrapping_sub(4);
    let group = ebp.wrapping_sub(0xc);
    let mode = g.read::<u8>(w(0x87));
    g.write::<u32>(b, w(0x9a));
    g.write::<u32>(group, p);
    g.write::<u8>(group + 5, mode);
    crate::nodes::walk(g, group, group, 0);
    let t = slot_target(g, w(0xe9), 325);
    g.call(t, 0, &[obj + 0x188, p + 0x1c]);
    let x = sub(g.read::<f32>(obj + 0x18c), g.read::<f32>(w(0x101)));
    g.write::<f32>(obj + 0x18c, x);
    let c = obj + 0x1c4;
    g.write::<u32>(c, w(0x118));
    static_child(
        g,
        img,
        f,
        c,
        [0x12b, 0x132, 0x137, 0x13f, 0x144, 0x14c, 0x160, 0x166],
    );
    let mode = g.read::<u8>(w(0x159));
    g.write::<u32>(group, q);
    g.write::<u8>(group + 5, mode);
    crate::nodes::walk(g, group, group, 0);
    for (k, o) in [(0x1cu32, 0x1e0u32), (0x20, 0x1e4), (0x24, 0x1e8)] {
        let v = g.read::<u32>(q + k);
        g.write::<u32>(obj + o, v);
    }
    let d = obj + 0x198;
    static_child(
        g,
        img,
        f,
        d,
        [0x1d6, 0x1dd, 0x1e2, 0x1ea, 0x1ef, 0x1f7, 0x203, 0x209],
    );
    g.write::<u32>(d, w(0x20f));
    for o in [0x1b4u32, 0x1b8, 0x1bc, 0x1c0] {
        g.write::<u32>(obj + o, 0x3f80_0000);
    }
    let rec = ebp.wrapping_sub(0x24);
    for (i, v) in (0u32..).zip([obj + 0x20, w(0x250), w(0x257), b, c, d]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x262),
            // push ebp; sub esp, 0x28; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x38),
            ecx: rec,
            ..Frame::default()
        },
    );
    let o = g.read::<u32>(w(0x267));
    let node = obj + 0x1f0;
    g.write::<u32>(rec, node);
    g.write::<u32>(node, w(0x276));
    for (i, v) in (1u32..).zip([w(0x280), obj + 0x20, o, w(0x28d), 2]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let r = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x299), rec);
    g.write::<u32>(obj + 0x18, 0);
    r
}

/// [`between`] (683 bytes, 1 overlay).
pub const BETWEEN: Sig = Sig {
    len: 683,
    hash: 0xa0aa_f824_b5a1_c412,
    calls: &[
        (0x262, Callee::Sig(&crate::nodes::INSERT)),
        (0x299, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// Sets up the effect's object for a target (0x100077d0, internal: `ecx`
/// = the object, `edx` = the source, one stack word, the target `a`):
/// the object joins the DLL's ring ([`ring_join`]); children of the DLL's
/// node list `b = obj + 0x16c` (handlers `+0xa2`), `c = obj + 0x1c4`
/// (`+0xcf`, set first) and `d = obj + 0x198` (`+0x1ed`); slot 268
/// `(src[0x54], 0x11, obj + 0x188)`, `+0x194` 1.0; `+0x1e0`/`+0x1e8` 0;
/// the target's size `s` (slot 333 `(a[0x54])` with bit 0 of `a[0x90]`,
/// else slot 601 `(a[0x50], &s)`; either way through the stack word, as
/// the original keeps it there) gives `+0x1e4` = `(K1 - s) * K2 / K3`
/// truncated; `+0x1b4..+0x1bc` 1.0; `+0x1c0` = slot 251 `(src[0x50])`
/// (also through the stack word) raised to 10 below `K4`, set to 20 above
/// `K5`, times `K6`; the node insert ([`node_insert`]) of `[obj + 0x20,
/// LIST, NAME, b, c, d]` on the frame (`ebp - 0x20`); `+0x18` 0. Returns
/// the insert's `eax`.
pub fn for_target<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let src = fr.edx;
    // the stack word: the target, then the original's float scratch
    let word = fr.esp + 4;
    ring_join(g, img, f, obj, [0x16, 0x1d, 0x25, 0x2f, 0x34, 0x43, 0x4a]);
    let b = obj + 0x16c;
    static_child(
        g,
        img,
        f,
        b,
        [0x5d, 0x64, 0x69, 0x71, 0x76, 0x7e, 0x8a, 0x9b],
    );
    let t = slot_target(g, w(0x8f), 268);
    g.write::<u32>(b, w(0xa2));
    let h = g.read::<u32>(src + 0x54);
    g.call(t, 0, &[h, 0x11, obj + 0x188]);
    g.write::<u32>(obj + 0x194, 0x3f80_0000);
    g.write::<u32>(obj + 0x194, 0x3f80_0000);
    let c = obj + 0x1c4;
    g.write::<u32>(c, w(0xcf));
    static_child(
        g,
        img,
        f,
        c,
        [0xe2, 0xe9, 0xee, 0xf6, 0xfb, 0x103, 0x10f, 0x118],
    );
    let a = g.read::<u32>(word);
    g.write::<u32>(obj + 0x1e8, 0);
    g.write::<u32>(obj + 0x1e0, 0);
    let x = if g.read::<u8>(a + 0x90) & 1 != 0 {
        let t = slot_target(g, w(0x13d), 333);
        let h = g.read::<u32>(a + 0x54);
        let s = st0_f32(g.call(t, 0, &[h]).st0);
        g.write::<f32>(word, s);
        sub(g.read::<f32>(w(0x150)), g.read::<f32>(word))
    } else {
        let t = slot_target(g, w(0x166), 601);
        let h = g.read::<u32>(a + 0x50);
        g.call(t, 0, &[h, word]);
        sub(g.read::<f32>(w(0x176)), g.read::<f32>(word))
    };
    let x = div(mul(x, g.read::<f32>(w(0x186))), g.read::<f32>(w(0x194)));
    g.write::<u32>(obj + 0x1e4, trunc(x));
    let d = obj + 0x198;
    static_child(
        g,
        img,
        f,
        d,
        [0x1b1, 0x1b8, 0x1bd, 0x1c5, 0x1ca, 0x1d2, 0x1de, 0x1e7],
    );
    g.write::<u32>(d, w(0x1ed));
    for o in [0x1b4u32, 0x1b8, 0x1bc] {
        g.write::<u32>(obj + o, 0x3f80_0000);
    }
    let t = slot_target(g, w(0x21a), 251);
    let h = g.read::<u32>(src + 0x50);
    let s = st0_f32(g.call(t, 0, &[h]).st0);
    g.write::<f32>(word, s);
    let y = g.read::<f32>(word);
    g.write::<f32>(obj + 0x1c0, y);
    // comiss + jbe: only an ordered "greater" sets the bound
    if g.read::<f32>(w(0x232)) > y {
        g.write::<u32>(obj + 0x1c0, 0x4120_0000);
    }
    if g.read::<f32>(obj + 0x1c0) > g.read::<f32>(w(0x25b)) {
        g.write::<u32>(obj + 0x1c0, 0x41a0_0000);
    }
    let z = mul(g.read::<f32>(obj + 0x1c0), g.read::<f32>(w(0x277)));
    let ebp = fr.esp.wrapping_sub(4);
    let group = ebp.wrapping_sub(0x20);
    g.write::<u32>(group, obj + 0x20);
    g.write::<f32>(obj + 0x1c0, z);
    for (i, v) in (1u32..).zip([w(0x295), w(0x29c), b, c, d]) {
        g.write::<u32>(group + 4 * i, v);
    }
    let r = node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x2aa),
            // push ebp; sub esp, 0x20; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x34),
            ecx: group,
            ..Frame::default()
        },
    );
    g.write::<u32>(obj + 0x18, 0);
    r.eax
}

/// [`for_target`] (700 bytes, 1 overlay).
pub const FOR_TARGET: Sig = Sig {
    len: 700,
    hash: 0xc401_8318_c7ab_7fbc,
    calls: &[(0x2aa, Callee::Sig(&crate::nodes::INSERT))],
    imports: &[],
};

/// The motion task (0x10006a90, `(task)`, a task step; a /GS frame) on
/// the record `r = task[1]` and its object `o = r[0x48]`, by the state
/// `+0x18`. Most states play motion `id` on the object's model `o[0x50]`
/// (slot 228 `(model, id)`) and go on to the next state, some only once
/// the motion running is done (slot 411 `(model)` non-zero):
/// - 0: motion 0xb8; 1: when done, 0xb9; 10: 0xba; 11: when done, 0xbb;
///   30: 0xbc; 31: when done, 0xbd; 10000: 0xb5;
/// - 10001: when done, motion 0xb6 gives a length `n` (its `eax`): `r +
///   0x20` and `+0x24` = `n`, and slot 234 `(r + 0x38, o2 + 0x1c, r +
///   0x28)` (`o2 = r[0x4c]`) a vector; the next state;
/// - 10002: the time `r + 0x20` drops by `K`; slot 233 `(&L, r + 0x38, t
///   / r[0x24])` and slot 248 `(o2 + 0x1c, r + 0x28, &L)` move `o2` (`L`
///   at `ebp - 0x14`); when done, motion 0xb7;
/// - 10003: when done, slot 213 `(model, 0)`, the object leaves the
///   model ([`object_to_node`](crate::sa_misc::object_to_node)), the task's
///   node `*task` leaves its ring and state 20000;
/// - any other: nothing.
///
/// The wait `+0x1c` goes down at every exit. Returns the last call's
/// `eax`, the node's next after the unlink, or what the original's
/// dispatch left in `eax`.
pub fn motion_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let state = g.read::<u32>(r + 0x18);
    let o = g.read::<u32>(r + 0x48);
    let done = |g: &mut G, op: u32| {
        let t = slot_target(g, w(op), 411);
        let m = g.read::<u32>(o + 0x50);
        g.call(t, 0, &[m]).eax
    };
    // slot 228 `(model, id)` (the table global at `+0x47`), then the next
    // state
    let play = |g: &mut G, id: u32| {
        let t = slot_target(g, w(0x47), 228);
        let m = g.read::<u32>(o + 0x50);
        let r2 = g.call(t, 0, &[m, id]).eax;
        let s = g.read::<u32>(r + 0x18);
        g.write::<u32>(r + 0x18, s.wrapping_add(1));
        r2
    };
    let gated = |g: &mut G, op: u32, id: u32| {
        let d = done(g, op);
        if d == 0 { 0 } else { play(g, id) }
    };
    let eax = match state {
        0 => play(g, 0xb8),
        1 => gated(g, 0x5f, 0xb9),
        10 => play(g, 0xba),
        11 => gated(g, 0x88, 0xbb),
        2..=9 => 4,
        0x1e => play(g, 0xbc),
        0x1f => gated(g, 0xc3, 0xbd),
        0x2710 => play(g, 0xb5),
        0x2711 => {
            if done(g, 0x1e2) == 0 {
                0
            } else {
                let t = slot_target(g, w(0x1f9), 228);
                let m = g.read::<u32>(o + 0x50);
                let n = g.call(t, 0, &[m, 0xb6]).eax;
                let x = cvt(n);
                let o2 = g.read::<u32>(r + 0x4c) + 0x1c;
                g.write::<f32>(r + 0x20, x);
                g.write::<f32>(r + 0x24, x);
                let t = slot_target(g, w(0x224), 234);
                // call width: 5 (motion 0xb6's words left on the stack)
                let e = g.call(t, 0, &[r + 0x38, o2, r + 0x28, m, 0xb6]).eax;
                let s = g.read::<u32>(r + 0x18);
                g.write::<u32>(r + 0x18, s.wrapping_add(1));
                e
            }
        }
        0x2712 => {
            let x = sub(g.read::<f32>(r + 0x20), g.read::<f32>(w(0x17b)));
            g.write::<f32>(r + 0x20, x);
            let y = div(x, g.read::<f32>(r + 0x24));
            let l = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
            let t = slot_target(g, w(0x198), 233);
            let mut stack = vec![l, r + 0x38, y.to_bits()];
            g.call(t, 0, &stack);
            let o2 = g.read::<u32>(r + 0x4c) + 0x1c;
            let t = slot_target(g, w(0x1b4), 248);
            stack.splice(0..0, [o2, r + 0x28, l]);
            // call width: 6
            g.call(t, 0, &stack);
            let t = slot_target(g, w(0x1c1), 411);
            let m = g.read::<u32>(o + 0x50);
            stack.insert(0, m);
            // call width: 7
            if g.call(t, 0, &stack).eax == 0 {
                0
            } else {
                play(g, 0xb7)
            }
        }
        0x2713 => {
            if done(g, 0x107) == 0 {
                0
            } else {
                let t = slot_target(g, w(0x122), 213);
                let m = g.read::<u32>(o + 0x50);
                let e = g.call(t, 0, &[m, 0]).eax;
                crate::sa_misc::object_to_node(g, img, img.rel32_target(f + 0x139), o, e);
                let next = unlink_task(g, task);
                let n = g.read::<u32>(r + 0x1c);
                g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
                g.write::<u32>(r + 0x18, 0x4e20);
                return Ret::eax(next);
            }
        }
        // the dispatch's remainder: `s - 0x2713` past the last case
        s if s > 0x2713 => s - 0x2713,
        s => s,
    };
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`motion_task`] (628 bytes, 1 overlay).
pub const MOTION_TASK: Sig = Sig {
    len: 628,
    hash: 0x2683_8fa1_2df5_4c38,
    calls: &[
        (0x139, Callee::Sig(&crate::sa_misc::OBJECT_TO_NODE)),
        // __security_check_cookie
        (0x16a, Callee::Any),
        (0x24c, Callee::Any),
    ],
    imports: &[],
};

/// Sets up the effect's object over a model node with its light
/// (0x10006db0, internal: `ecx` = the object, `edx` a vector, one stack
/// word `node`): as [`over_node`] up to the node insert (handlers `+0xa0`,
/// `+0xb7`, `+0x1af`; the mode operand `+0xf8`; no `+0x18c` change), then
/// a particle node ([`particle_node`](crate::sb_nodes::particle_node)) of
/// `[obj + 0x1f0, LIST2, obj + 0x20, *G_O, PART, 0]` (vtable `+0x216`
/// first, on the frame at `ebp - 0x24`), the DLL's flag byte (`+0x244`)
/// bit 0, its light block `+0x25f..+0x2a5` = `(0, 0, 5, 7, 5, 5, 0, -K)`
/// and slot 306 `(7, 1, slot 719 (NAME, 5, 0))`; `+0x18` 0. Returns slot
/// 306's `eax`.
pub fn lit_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let node = fr.arg(g, 0);
    ring_join(g, img, f, obj, [0x13, 0x1a, 0x22, 0x2c, 0x31, 0x40, 0x47]);
    let b = obj + 0x16c;
    static_child(
        g,
        img,
        f,
        b,
        [0x5a, 0x61, 0x66, 0x6e, 0x73, 0x7b, 0x87, 0x9a],
    );
    let t = slot_target(g, w(0x94), 325);
    g.write::<u32>(b, w(0xa0));
    g.call(t, 0, &[obj + 0x188, fr.edx]);
    let c = obj + 0x1c4;
    g.write::<u32>(c, w(0xb7));
    static_child(
        g,
        img,
        f,
        c,
        [0xca, 0xd1, 0xd6, 0xde, 0xe3, 0xeb, 0xff, 0x105],
    );
    let ebp = fr.esp.wrapping_sub(4);
    let mode = g.read::<u8>(w(0xf8));
    mode_walk(g, ebp.wrapping_sub(0xc), node, mode);
    for (k, o) in [(0x1cu32, 0x1e0u32), (0x20, 0x1e4), (0x24, 0x1e8)] {
        let v = g.read::<u32>(node + k);
        g.write::<u32>(obj + o, v);
    }
    let d = obj + 0x198;
    static_child(
        g,
        img,
        f,
        d,
        [0x176, 0x17d, 0x182, 0x18a, 0x18f, 0x197, 0x1a3, 0x1a9],
    );
    g.write::<u32>(d, w(0x1af));
    for o in [0x1b4u32, 0x1b8, 0x1bc, 0x1c0] {
        g.write::<u32>(obj + o, 0x3f80_0000);
    }
    let rec = ebp.wrapping_sub(0x24);
    for (i, v) in (0u32..).zip([obj + 0x20, w(0x1f0), w(0x1f7), b, c, d]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x202),
            // push ebp; sub esp, 0x28; three pushes: the call's esp
            esp: fr.esp.wrapping_sub(0x38),
            ecx: rec,
            ..Frame::default()
        },
    );
    let o = g.read::<u32>(w(0x207));
    let pn = obj + 0x1f0;
    g.write::<u32>(rec, pn);
    g.write::<u32>(pn, w(0x216));
    for (i, v) in (1u32..).zip([w(0x220), obj + 0x20, o, w(0x22d), 0]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x239), rec);
    let t = slot_target(g, w(0x23e), 719);
    let flag = g.read::<u8>(w(0x244));
    g.write::<u8>(w(0x244), flag | 1);
    let zero = sub(0.0, 0.0);
    let neg = sub(0.0, g.read::<f32>(w(0x257)));
    for (op, v) in [
        (0x25fu32, 0u32),
        (0x269, 0x40a0_0000),
        (0x275, zero.to_bits()),
        (0x27b, 0x40e0_0000),
        (0x285, 0x40a0_0000),
        (0x28f, 0x40a0_0000),
        (0x299, 0),
        (0x2a5, neg.to_bits()),
    ] {
        g.write::<u32>(w(op), v);
    }
    let h = g.call(t, 0, &[w(0x2b2), 5, 0]).eax;
    let t = slot_target(g, w(0x2bd), 306);
    let r = g.call(t, 0, &[7, 1, h]).eax;
    g.write::<u32>(obj + 0x18, 0);
    r
}

/// [`lit_node`] (734 bytes, 1 overlay).
pub const LIT_NODE: Sig = Sig {
    len: 734,
    hash: 0x29e4_2777_ac79_f18b,
    calls: &[
        (0x202, Callee::Sig(&crate::nodes::INSERT)),
        (0x239, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// The light task (0x100076e0, `(task)`, a task step) on the record `r =
/// task[1]`, by its state `+0x18`:
/// - 0: state 1, a wait `+0x1c` of 3; 1: while the wait runs, it waits;
///   then a particle node from `r + 0x1f0`
///   ([`particle_node_with`](crate::s3_nodes::particle_node_with) `(r +
///   0x20, _, _, 1)`: the two words between are the original's `sub esp,
///   8`, which the callee never reads), slot 239 `(*G_A, B, 0)` (`B` the
///   DLL's byte at `+0x5d`), slot 306 `(7, 2, slot 719 (NAME, 5, 0))` and
///   state 2;
/// - 2: once bit 2 of `+0x2d8` is set, state 10000 and on;
/// - 10000: the task's node `*task` leaves its ring, state 20000.
///
/// The wait goes down at every exit. Returns slot 306's `eax`, the
/// node's next after the unlink, or what is left of the state.
pub fn light_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let state = g.read::<u32>(r + 0x18);
    let eax = match state {
        0 => {
            g.write::<u32>(r + 0x18, 1);
            g.write::<u32>(r + 0x1c, 3);
            0
        }
        1 if g.read::<i32>(r + 0x1c) > 0 => 0,
        1 => {
            // push ebp; push esi; push 1; sub esp, 8; push: the callee's
            // words at entry - 0x18 (its esp at entry - 0x1c)
            let sp = fr.esp.wrapping_sub(0x1c);
            g.write::<u32>(sp + 4, r + 0x20);
            g.write::<u32>(sp + 0x10, 1);
            crate::s3_nodes::particle_node_with(
                g,
                img,
                &Frame {
                    at: img.rel32_target(f + 0x56),
                    esp: sp,
                    ecx: r + 0x1f0,
                    ..Frame::default()
                },
            );
            let b = u32::from(g.read::<u8>(w(0x5d)));
            let t = slot_target(g, w(0x65), 239);
            let a = g.read::<u32>(w(0x6b));
            g.call(t, 0, &[a, b, 0, r + 0x20]);
            let t = slot_target(g, w(0x78), 719);
            let h = g.call(t, 0, &[w(0x8a), 5, 0]).eax;
            let t = slot_target(g, w(0x95), 306);
            let e = g.call(t, 0, &[7, 2, h]).eax;
            let s = g.read::<u32>(r + 0x18);
            g.write::<u32>(r + 0x18, s.wrapping_add(1));
            e
        }
        2 if g.read::<u8>(r + 0x2d8) & 4 == 0 => 2,
        2 | 0x2710 => {
            if state == 2 {
                g.write::<u32>(r + 0x18, 0x2710);
            }
            let next = unlink_task(g, task);
            g.write::<u32>(r + 0x18, 0x4e20);
            next
        }
        _ => state,
    };
    let n = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`light_task`] (239 bytes, 1 overlay).
pub const LIGHT_TASK: Sig = Sig {
    len: 239,
    hash: 0x015c_7831_a0f6_bca2,
    calls: &[(0x56, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH))],
    imports: &[],
};
