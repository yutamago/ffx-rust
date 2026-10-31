//! `magic_0078.dll`'s functions (shared with `magic_0079`).

use crate::runtime::slot_target;
use crate::sa_summon::{MODEL_NODE_INIT, model_node_init};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// 0x10006470 (a callback, cdecl `(task)`): the effect's clock (`+0x2c4`
/// of the object at `task +0x4`) advances by `K0`; when its hold count
/// u16 `+0x2c8` is 0 and the clock reached `K1`, slot 239
/// (`G_HANDLE`, -5, 0) and the task (`*task`) is unlinked from its ring
/// (`+0x4` prev, `+0x8` next) and left pointing at itself. Returns the
/// object, or after the unlink the task's old previous.
pub fn clock<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let task = fr.arg(g, 0);
    let obj = g.read::<u32>(task + 4);
    let held = g.read::<u16>(obj + 0x2c8) != 0;
    let t = f32::from_bits(g.read::<u32>(obj + 0x2c4)) + f32::from_bits(g.read::<u32>(w(0x1e)));
    g.write::<u32>(obj + 0x2c4, t.to_bits());
    // jne; comiss; jb: unordered skips too
    let due = matches!(
        t.partial_cmp(&f32::from_bits(g.read::<u32>(w(0x2f)))),
        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
    );
    if held || !due {
        return Ret::eax(obj);
    }
    let slot239 = slot_target(g, w(0x36), 239);
    let handle = g.read::<u32>(w(0x46));
    g.call(slot239, 0, &[handle, (-5i32).cast_unsigned(), 0]);
    let node = g.read::<u32>(task);
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node + 8);
    g.write::<u32>(prev + 8, next);
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    Ret::eax(prev)
}

/// `clock` (108 bytes; 2 overlays).
pub const CLOCK: Sig = Sig {
    len: 108,
    hash: 0x9c13_5c9a_7e59_6503,
    calls: &[],
    imports: &[],
};

/// `mov dword [w(dst)], w(val)` for each pair, in order.
fn stores<G: Game>(g: &mut G, img: &Image, f: u32, pairs: &[(u32, u32)]) {
    for &(dst, val) in pairs {
        g.write::<u32>(img.word(f + dst), img.word(f + val));
    }
}

/// Operand offsets of the two static nodes [`two_nodes`] appends.
struct TwoNodes {
    /// The root list's last node (read), `N1`'s halfword, its four
    /// stores, the last node's copy, the sentinel, the first-word store
    /// (empty list) and the old last's `+0x4` (else).
    last_r: u32,
    half1: u32,
    st1: [(u32, u32); 4],
    last_w1: u32,
    sentinel1: u32,
    first1: (u32, u32),
    next1: u32,
    /// The u16 count (read), `N1`'s address, its vtable store.
    count_r: u32,
    n1: u32,
    vt1: (u32, u32),
    /// `N2`'s halfword and five stores, the sentinel test against `N1`,
    /// the first-word store or `N1`'s `+0x4` store, the list's last,
    /// the count, `N2`'s vtable, its three 1.0 words and the word set to
    /// the immediate (`dst`, the immediate's offset).
    half2: u32,
    st2: [(u32, u32); 5],
    sentinel2: u32,
    first2: (u32, u32),
    next2: (u32, u32),
    last_w: (u32, u32),
    count_w: u32,
    vt2: (u32, u32),
    ones: [u32; 3],
    imm: (u32, u32),
}

/// Two static nodes `N1`, `N2` appended to the root list as the
/// start-ups do inline (all constant stores; the count goes up by two).
fn two_nodes<G: Game>(g: &mut G, img: &Image, f: u32, o: &TwoNodes) {
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(o.last_r));
    g.write::<u16>(w(o.half1), 0xff);
    stores(g, img, f, &o.st1);
    g.write::<u32>(w(o.last_w1), last);
    if last == w(o.sentinel1) {
        stores(g, img, f, &[o.first1]);
    } else {
        g.write::<u32>(last + 4, w(o.next1));
    }
    let count = g.read::<u16>(w(o.count_r));
    stores(g, img, f, &[o.vt1]);
    g.write::<u16>(w(o.half2), 0xff);
    stores(g, img, f, &o.st2);
    if w(o.n1) == w(o.sentinel2) {
        stores(g, img, f, &[o.first2]);
    } else {
        stores(g, img, f, &[o.next2]);
    }
    stores(g, img, f, &[o.last_w]);
    g.write::<u16>(w(o.count_w), count.wrapping_add(2));
    stores(g, img, f, &[o.vt2]);
    for at in o.ones {
        g.write::<u32>(w(at), 0x3f80_0000);
    }
    g.write::<u32>(w(o.imm.0), img.word(f + o.imm.1));
}

/// Where the start-ups differ: the static nodes, the `node insert`
/// record (six immediates and the `rel32`), slot 268 (table global, the
/// model global, the immediate; before the nodes or after the insert),
/// the 1.0 store next to it and the pointer global the particle record
/// takes.
struct Start {
    nodes: Nodes,
    insert: ([u32; 6], u32),
    s268: (u32, u32, u32),
    early: bool,
    one: u32,
    a: u32,
    tail: PartTail,
}

/// How the start-up appends its two static nodes: [`two_nodes`] (the
/// count kept in a register), or each in turn ([`each_node`], slot 268
/// between them; then `N2`'s three 1.0 words and the word set to an
/// immediate, as in [`TwoNodes`]).
enum Nodes {
    Two(TwoNodes),
    Each([EachNode; 2], [u32; 3], (u32, u32)),
}

/// Operand offsets of one node [`each_node`] appends: the list's last
/// (read), the halfword, four stores, the last's copy, the sentinel,
/// the first-word store, the old last's `+0x4`, the u16 count, the
/// list's last store and the vtable store.
struct EachNode {
    last_r: u32,
    half: u32,
    st: [(u32, u32); 4],
    last_w1: u32,
    sentinel: u32,
    first: (u32, u32),
    next: u32,
    count: u32,
    last_w: (u32, u32),
    vt: (u32, u32),
}

/// One static node appended to the root list with its own count step.
fn each_node<G: Game>(g: &mut G, img: &Image, f: u32, o: &EachNode) {
    let w = |off: u32| img.word(f + off);
    let last = g.read::<u32>(w(o.last_r));
    g.write::<u16>(w(o.half), 0xff);
    stores(g, img, f, &o.st);
    g.write::<u32>(w(o.last_w1), last);
    if last == w(o.sentinel) {
        stores(g, img, f, &[o.first]);
    } else {
        g.write::<u32>(last + 4, w(o.next));
    }
    let c = g.read::<u16>(w(o.count));
    g.write::<u16>(w(o.count), c.wrapping_add(1));
    stores(g, img, f, &[o.last_w, o.vt]);
}

/// Operand offsets of the particle and effect nodes that end the
/// start-ups: the particle record's words 0..2 and 4, its vtable store
/// and `rel32`; per effect node its five words, `rel32`, vtable and flag
/// words.
struct PartTail {
    part: PartRec,
    fx: [FxRec; 2],
}

/// A particle record's operands: words 0..2, word 4, the vtable store,
/// the `rel32`.
type PartRec = ([u32; 3], u32, (u32, u32), u32);

/// An effect node's operands: its five words, the `rel32`, the vtable,
/// the flag words.
type FxRec = ([u32; 5], u32, u32, (u32, u32));

/// The particle and effect nodes of [`PartTail`] (records at `rec`, the
/// pointer global's value `a`). Returns the second effect node.
fn part_tail<G: Game>(g: &mut G, img: &Image, f: u32, rec: u32, a: u32, o: &PartTail) -> u32 {
    let w = |off: u32| img.word(f + off);
    let (words, fifth, vt, call) = o.part;
    for (k, v) in (0u32..).zip([w(words[0]), w(words[1]), w(words[2]), a, w(fifth), 0]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    stores(g, img, f, &[vt]);
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + call), rec);
    let mut node = 0;
    for (i, &(words, call, vt, flags)) in (0u32..).zip(&o.fx) {
        for (k, &op) in (0u32..).zip(&words) {
            g.write::<u32>(rec + 4 * k, w(op));
        }
        g.write::<u32>(rec + 20, i);
        crate::sb_nodes::effect_node(g, img, img.rel32_target(f + call), rec);
        node = g.read::<u32>(rec);
        g.write::<u32>(node, w(vt));
        let v = g.read::<u32>(w(flags.0));
        g.write::<u32>(w(flags.0), v | 1);
        g.write::<u32>(w(flags.1), 0);
    }
    node
}

/// The particle and effect nodes as `magic_0078`/`0079` place them.
const TAIL_0078: PartTail = PartTail {
    part: ([0x1fd, 0x204, 0x20b], 0x215, (0x222, 0x226), 0x22b),
    fx: [
        (
            [0x235, 0x23c, 0x243, 0x24a, 0x251],
            0x25d,
            0x269,
            (0x26f, 0x276),
        ),
        (
            [0x281, 0x288, 0x28f, 0x296, 0x29d],
            0x2a9,
            0x2b2,
            (0x2b8, 0x2bf),
        ),
    ],
};

/// The start-ups of `magic_0078`/`0079` (0x100064e0, no arguments, a
/// frame of 0x18 bytes): a node put at the ring's tail (`+0x07`..
/// `+0x3b`), the root's flag cleared (`+0x41`), a u16 and a word
/// cleared (`+0x4a`, `+0x64`); the model node (`+0x5e`, vtable `+0x87`)
/// set up ([`model_node_init`], a record on the frame `[node, parent,
/// *B, 0]`, the byte `*B` at `+0x51`); slot 268 `(*M, 0x11, X)` and a
/// 1.0 store (here or after the insert), the two static nodes
/// ([`two_nodes`]) and `node insert`; the particle node (vtable
/// `+0x226`, record `[.., .., .., *A, .., 0]`) and two effect nodes with
/// their vtables and flag words. Returns the second effect node.
fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame, o: &Start) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let tail = g.read::<u32>(w(0x07));
    g.write::<u32>(w(0x0c), tail);
    stores(
        g,
        img,
        f,
        &[(0x12, 0x16), (0x1c, 0x20), (0x26, 0x2a), (0x30, 0x34)],
    );
    g.write::<u32>(tail + 4, w(0x3b));
    let v = g.read::<u32>(w(0x41));
    g.write::<u32>(w(0x41), v & !1);
    g.write::<u16>(w(0x4a), 0);
    let b = u32::from(g.read::<u8>(w(0x51)));
    stores(g, img, f, &[(0x5a, 0x5e)]);
    g.write::<u32>(w(0x64), 0);
    let rec = ebp.wrapping_sub(0x10);
    for (k, v) in (0u32..).zip([w(0x6f), w(0x76), b]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    g.write::<u8>(rec + 0xc, 0);
    stores(g, img, f, &[(0x83, 0x87)]);
    model_node_init(g, img, img.rel32_target(f + 0x8c), rec);
    let call_268 = |g: &mut G| {
        let (table, m, x) = o.s268;
        let s = slot_target(g, w(table), 0x430 / 4);
        let mv = g.read::<u32>(w(m));
        g.call(s, 0, &[mv, 0x11, w(x)]);
    };
    if o.early {
        call_268(g);
        g.write::<u32>(w(o.one), 0x3f80_0000);
    }
    match &o.nodes {
        Nodes::Two(n) => two_nodes(g, img, f, n),
        Nodes::Each([n1, n2], ones, imm) => {
            each_node(g, img, f, n1);
            call_268(g);
            g.write::<u32>(w(o.one), 0x3f80_0000);
            each_node(g, img, f, n2);
            for &at in ones {
                g.write::<u32>(w(at), 0x3f80_0000);
            }
            g.write::<u32>(w(imm.0), img.word(f + imm.1));
        }
    }
    let rec = ebp.wrapping_sub(0x18);
    let (words, call) = o.insert;
    for (k, &op) in (0u32..).zip(&words) {
        g.write::<u32>(rec + 4 * k, w(op));
    }
    let at = img.rel32_target(f + call);
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at,
            ecx: rec,
            ..Frame::default()
        },
    );
    let late = !o.early && matches!(o.nodes, Nodes::Two(_));
    if late {
        call_268(g);
    }
    let a = g.read::<u32>(w(o.a));
    if late {
        g.write::<u32>(w(o.one), 0x3f80_0000);
    }
    Ret::eax(part_tail(g, img, f, rec, a, &o.tail))
}

/// `magic_0078` 0x100064e0: [`start`], slot 268 after the insert.
pub fn start_0078<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = Start {
        nodes: Nodes::Two(TwoNodes {
            last_r: 0x91,
            half1: 0x98,
            st1: [(0xa0, 0xa4), (0xaa, 0xae), (0xb4, 0xb8), (0xbe, 0xc2)],
            last_w1: 0xc7,
            sentinel1: 0xcc,
            first1: (0xd4, 0xd8),
            next1: 0xe1,
            count_r: 0xe7,
            n1: 0xec,
            vt1: (0xf4, 0xf8),
            half2: 0xff,
            st2: [
                (0x107, 0x10b),
                (0x111, 0x115),
                (0x11b, 0x11f),
                (0x125, 0x129),
                (0x12f, 0x133),
            ],
            sentinel2: 0x139,
            first2: (0x141, 0x145),
            next2: (0x14d, 0x151),
            last_w: (0x15c, 0x160),
            count_w: 0x166,
            vt2: (0x16c, 0x170),
            ones: [0x176, 0x180, 0x18a],
            imm: (0x194, 0x198),
        }),
        insert: ([0x19f, 0x1a6, 0x1ad, 0x1b4, 0x1bb, 0x1c2], 0x1c7),
        s268: (0x1cc, 0x1df, 0x1d1),
        early: false,
        one: 0x1f2,
        a: 0x1e6,
        tail: TAIL_0078,
    };
    start(g, img, fr, &o)
}

/// `magic_0079` 0x100064e0: [`start`], slot 268 right after the model
/// node.
pub fn start_0079<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = Start {
        nodes: Nodes::Two(TwoNodes {
            last_r: 0xab,
            half1: 0xbf,
            st1: [(0xc7, 0xcb), (0xd1, 0xd5), (0xdb, 0xdf), (0xe5, 0xe9)],
            last_w1: 0xee,
            sentinel1: 0xf3,
            first1: (0xfb, 0xff),
            next1: 0x108,
            count_r: 0x10e,
            n1: 0x113,
            vt1: (0x11b, 0x11f),
            half2: 0x126,
            st2: [
                (0x12e, 0x132),
                (0x138, 0x13c),
                (0x142, 0x146),
                (0x14c, 0x150),
                (0x156, 0x15a),
            ],
            sentinel2: 0x160,
            first2: (0x168, 0x16c),
            next2: (0x174, 0x178),
            last_w: (0x183, 0x187),
            count_w: 0x18d,
            vt2: (0x193, 0x197),
            ones: [0x19d, 0x1a7, 0x1b1],
            imm: (0x1bb, 0x1bf),
        }),
        insert: ([0x1c6, 0x1cd, 0x1d4, 0x1db, 0x1e2, 0x1e9], 0x1ee),
        s268: (0x91, 0xa4, 0x96),
        early: true,
        one: 0xb4,
        a: 0x1f3,
        tail: TAIL_0078,
    };
    start(g, img, fr, &o)
}

/// [`start_0078`] (715 bytes, 1 overlay).
pub const START_0078: Sig = Sig {
    len: 715,
    hash: 0x4d31_1169_ea27_859c,
    calls: &[
        (0x8c, Callee::Sig(&MODEL_NODE_INIT)),
        (0x1c7, Callee::Sig(&crate::nodes::INSERT)),
        (0x22b, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x25d, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x2a9, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// [`start_0079`] (715 bytes, 1 overlay).
pub const START_0079: Sig = Sig {
    len: 715,
    hash: 0x1f4a_ba56_329d_549a,
    calls: &[
        (0x8c, Callee::Sig(&MODEL_NODE_INIT)),
        (0x1ee, Callee::Sig(&crate::nodes::INSERT)),
        (0x22b, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x25d, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x2a9, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// `magic_0081` 0x100064e0: [`start`], the static nodes appended one by
/// one with slot 268 between them ([`Nodes::Each`]).
pub fn start_0081<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = Start {
        nodes: Nodes::Each(
            [
                EachNode {
                    last_r: 0x91,
                    half: 0x98,
                    st: [(0xa0, 0xa4), (0xaa, 0xae), (0xb4, 0xb8), (0xbe, 0xc2)],
                    last_w1: 0xc7,
                    sentinel: 0xcc,
                    first: (0xd4, 0xd8),
                    next: 0xe1,
                    count: 0xed,
                    last_w: (0x100, 0x104),
                    vt: (0x10a, 0x10e),
                },
                EachNode {
                    last_r: 0x11b,
                    half: 0x12f,
                    st: [
                        (0x137, 0x13b),
                        (0x141, 0x145),
                        (0x14b, 0x14f),
                        (0x155, 0x159),
                    ],
                    last_w1: 0x15e,
                    sentinel: 0x163,
                    first: (0x16b, 0x16f),
                    next: 0x178,
                    count: 0x17f,
                    last_w: (0x188, 0x18c),
                    vt: (0x192, 0x196),
                },
            ],
            [0x19c, 0x1a6, 0x1b0],
            (0x1ba, 0x1be),
        ),
        insert: ([0x1c5, 0x1cc, 0x1d3, 0x1da, 0x1e1, 0x1e8], 0x1ed),
        s268: (0xe6, 0xfa, 0xf2),
        early: false,
        one: 0x124,
        a: 0x1f2,
        tail: PartTail {
            part: ([0x1fc, 0x203, 0x20a], 0x214, (0x221, 0x225), 0x22a),
            fx: [
                (
                    [0x234, 0x23b, 0x242, 0x249, 0x250],
                    0x25c,
                    0x268,
                    (0x26e, 0x275),
                ),
                (
                    [0x280, 0x287, 0x28e, 0x295, 0x29c],
                    0x2a8,
                    0x2b1,
                    (0x2b7, 0x2be),
                ),
            ],
        },
    };
    start(g, img, fr, &o)
}

/// [`start_0081`] (714 bytes, 1 overlay).
pub const START_0081: Sig = Sig {
    len: 714,
    hash: 0xc5d2_c814_6135_fc7d,
    calls: &[
        (0x8c, Callee::Sig(&MODEL_NODE_INIT)),
        (0x1ed, Callee::Sig(&crate::nodes::INSERT)),
        (0x22a, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x25c, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x2a8, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};
