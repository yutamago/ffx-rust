//! `magic_0076.dll`'s own functions (besides the runtime, the object
//! host and the shared code), and `magic_0077`'s copy of its start-up.

use crate::sa_summon::{MODEL_NODE_INIT, model_node_init};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The fifth word of an effect node's record: a global's value or an
/// immediate (operand offsets).
#[derive(Clone, Copy)]
enum Word5 {
    Load(u32),
    Imm(u32),
}

/// The second half of the model start-ups ([`model_start`]) where the
/// copies differ: the 1.0 store, the two effect node records (four
/// immediates, the fifth word, the `rel32`), their vtables, their flag
/// words (or 1, cleared next word) and the two words cleared last.
struct ModelTail {
    one: u32,
    recs: [([u32; 4], Word5, u32); 2],
    vt: [u32; 2],
    flags: [(u32, u32); 2],
    zeros: [u32; 2],
}

/// The model start-ups of `magic_0076`/`0077` (0x10006800, no arguments,
/// a frame of 0x18 bytes): a node put at the ring's tail (`+0x07`..
/// `+0x3b`), the root's flag cleared (`+0x48`), the model node (`+0x56`,
/// its vtable `+0x75`) set up ([`model_node_init`], a record on the
/// frame `[node, parent, *B, 0]`, the byte `*B` at `+0x42`), the static
/// node `N` (`+0x8e`..) appended to the root list (count `+0xdb`) with
/// its vtable (`+0xfc`) and slot 268 `(*M, 0x11, N + 0x1c)`, then two
/// effect nodes ([`effect_node`](crate::sb_nodes::effect_node), records
/// on the frame) with their vtables and flag words, and two words
/// cleared. Returns the second effect node.
fn model_start<G: Game>(g: &mut G, img: &Image, fr: &Frame, o: &ModelTail) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let tail = g.read::<u32>(w(0x07));
    g.write::<u32>(w(0x0c), tail);
    for (dst, v) in [(0x12, 0x16), (0x1c, 0x20), (0x26, 0x2a), (0x30, 0x34)] {
        g.write::<u32>(w(dst), w(v));
    }
    g.write::<u32>(tail + 4, w(0x3b));
    let b = u32::from(g.read::<u8>(w(0x42)));
    let v = g.read::<u32>(w(0x48));
    g.write::<u32>(w(0x48), v & !1);
    g.write::<u32>(w(0x52), w(0x56));
    let rec = ebp.wrapping_sub(0x10);
    for (k, v) in (0u32..).zip([w(0x5d), w(0x64), b]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    g.write::<u8>(rec + 0xc, 0);
    g.write::<u32>(w(0x71), w(0x75));
    model_node_init(g, img, img.rel32_target(f + 0x7a), rec);
    let last = g.read::<u32>(w(0x7f));
    g.write::<u16>(w(0x86), 0xff);
    for (dst, v) in [(0x8e, 0x92), (0x98, 0x9c), (0xa2, 0xa6), (0xac, 0xb0)] {
        g.write::<u32>(w(dst), w(v));
    }
    g.write::<u32>(w(0xb5), last);
    if last == w(0xba) {
        g.write::<u32>(w(0xc2), w(0xc6));
    } else {
        g.write::<u32>(last + 4, w(0xcf));
    }
    let table = g.read::<u32>(w(0xd4));
    let c = g.read::<u16>(w(0xdb));
    g.write::<u16>(w(0xdb), c.wrapping_add(1));
    let m = g.read::<u32>(w(0xe8));
    g.write::<u32>(w(0xee), w(0xf2));
    g.write::<u32>(w(0xf8), w(0xfc));
    let s = g.read::<u32>(table.wrapping_add(0x430));
    g.call(s, 0, &[m, 0x11, w(0xe0)]);
    let rec = ebp.wrapping_sub(0x18);
    let mut node = 0;
    for (i, &(words, fifth, call)) in (0u32..).zip(&o.recs) {
        let five = match fifth {
            Word5::Load(op) => g.read::<u32>(w(op)),
            Word5::Imm(op) => w(op),
        };
        if i == 0 {
            g.write::<u32>(w(o.one), 0x3f80_0000);
        }
        for (k, &op) in (0u32..).zip(&words) {
            g.write::<u32>(rec + 4 * k, w(op));
        }
        g.write::<u32>(rec + 16, five);
        g.write::<u32>(rec + 20, i);
        crate::sb_nodes::effect_node(g, img, img.rel32_target(f + call), rec);
        node = g.read::<u32>(rec);
        g.write::<u32>(node, w(o.vt[i as usize]));
        let (fl, fl2) = o.flags[i as usize];
        let v = g.read::<u32>(w(fl));
        g.write::<u32>(w(fl), v | 1);
        g.write::<u32>(w(fl2), 0);
    }
    for at in o.zeros {
        g.write::<u32>(w(at), 0);
    }
    Ret::eax(node)
}

/// `magic_0076` 0x10006800: [`model_start`], the effect records' fifth
/// words loaded from `+0x109` and `+0x155`.
pub fn model_start_0076<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = ModelTail {
        one: 0x115,
        recs: [
            ([0x120, 0x127, 0x12e, 0x135], Word5::Load(0x109), 0x144),
            ([0x16d, 0x174, 0x17b, 0x182], Word5::Load(0x155), 0x191),
        ],
        vt: [0x150, 0x19a],
        flags: [(0x15b, 0x162), (0x1a0, 0x1a7)],
        zeros: [0x1b1, 0x1bb],
    };
    model_start(g, img, fr, &o)
}

/// `magic_0077` 0x10006800: [`model_start`], the fifth words immediates
/// (`+0x137`, `+0x183`).
pub fn model_start_0077<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = ModelTail {
        one: 0x110,
        recs: [
            ([0x11b, 0x122, 0x129, 0x130], Word5::Imm(0x137), 0x143),
            ([0x167, 0x16e, 0x175, 0x17c], Word5::Imm(0x183), 0x18f),
        ],
        vt: [0x14f, 0x198],
        flags: [(0x155, 0x15c), (0x19e, 0x1a5)],
        zeros: [0x1af, 0x1b9],
    };
    model_start(g, img, fr, &o)
}

/// [`model_start_0076`] (455 bytes, 1 overlay).
pub const MODEL_START_0076: Sig = Sig {
    len: 455,
    hash: 0x4d77_c48b_7c23_ef21,
    calls: &[
        (0x7a, Callee::Sig(&MODEL_NODE_INIT)),
        (0x144, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x191, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// [`model_start_0077`] (453 bytes, 1 overlay).
pub const MODEL_START_0077: Sig = Sig {
    len: 453,
    hash: 0xdffa_cc41_9f8d_f563,
    calls: &[
        (0x7a, Callee::Sig(&MODEL_NODE_INIT)),
        (0x143, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x18f, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};
