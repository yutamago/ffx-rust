//! Node constructors of the own work list (round 4): each lays a set-up
//! record out on its stack (where the original's frame has it), calls a
//! node initialiser with `ecx` = the record and sets the node's vtable.
//! The record's words and the vtable are the instructions' operands.

use crate::m0070::{node_init_big, node_init_big_table};
use crate::m0071::node_init_vec;
use crate::sa_misc::colour_node_init;
use crate::sb_nodes::effect_node;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Writes `words` from `at` up.
fn record(g: &mut impl crate::Mem, at: u32, words: &[u32]) {
    for (k, &v) in (0u32..).zip(words) {
        g.write::<u32>(at + 4 * k, v);
    }
}

/// `(a)` (`magic_0159` 0x10006700, 68 bytes): the record `(A, B, a, C,
/// cb, 0)` at `ebp - 0x18` (the operands at `+0x0f`, `+0x16`, `+0x20`,
/// `+0x27`), [`effect_node`] (`+0x33`), then the node `rec[0]` gets the
/// vtable at `+0x3c`. Returns the node.
pub fn effect_ctor_arg<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.esp.wrapping_sub(0x1c);
    let a = fr.arg(g, 0);
    record(g, rec, &[w(0x0f), w(0x16), a, w(0x20), w(0x27), 0]);
    effect_node(g, img, img.rel32_target(f + 0x33), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x3c));
    Ret::eax(node)
}

/// `(_, a)` (`magic_0202` 0x10006f10, 68 bytes): as [`effect_ctor_arg`]
/// with the record `(A, B, C, a, cb, 0)` (`+0x0f`, `+0x16`, `+0x1d`,
/// `+0x27`; `a` the second stack word).
pub fn effect_ctor_arg2<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.esp.wrapping_sub(0x1c);
    let a = fr.arg(g, 1);
    record(g, rec, &[w(0x0f), w(0x16), w(0x1d), a, w(0x27), 0]);
    effect_node(g, img, img.rel32_target(f + 0x33), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x3c));
    Ret::eax(node)
}

/// `this` = `ecx` (`magic_0388` 0x10006d70, 65 bytes): as
/// [`effect_ctor_arg`] with the record `(this, A, B, C, cb, 0)` (`+0x0f`,
/// `+0x16`, `+0x1d`, `+0x24`), the call at `+0x30`, the vtable at
/// `+0x39`.
pub fn effect_ctor_this<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.esp.wrapping_sub(0x1c);
    record(g, rec, &[fr.ecx, w(0x0f), w(0x16), w(0x1d), w(0x24), 0]);
    effect_node(g, img, img.rel32_target(f + 0x30), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x39));
    Ret::eax(node)
}

/// `this` = `ecx` (`magic_0597` 0x10008f40, 59 bytes): the record
/// `(this, A, B, C, D)` at `ebp - 0x14` (`+0x12`, `+0x19`, `+0x20`,
/// `+0x27`), [`node_init_big`] (`+0x2c`), then `this` gets the vtable at
/// `+0x32`. Returns what the initialiser returned.
pub fn big_ctor_this<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.esp.wrapping_sub(0x18);
    record(g, rec, &[fr.ecx, w(0x12), w(0x19), w(0x20), w(0x27)]);
    let r = node_init_big(g, rec);
    g.write::<u32>(fr.ecx, w(0x32));
    Ret::eax(r)
}

/// No arguments (`magic_0262` 0x10006c10, 53 bytes): the record of five
/// operands (`+0x0c`, `+0x13`, `+0x1a`, `+0x21`, `+0x28`) at `ebp -
/// 0x14`, then [`node_init_big_table`] (`+0x2d`). Returns its result.
pub fn big_table_ctor<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.esp.wrapping_sub(0x18);
    record(g, rec, &[w(0x0c), w(0x13), w(0x1a), w(0x21), w(0x28)]);
    Ret::eax(node_init_big_table(g, img, img.rel32_target(f + 0x2d), rec))
}

/// `(_, _, a)` (`magic_0070` 0x100097b0, 76 bytes): the record `(A, B,
/// C, D, a, E, F)` at `ebp - 0x1c` (`+0x0f`, `+0x16`, `+0x1d`, `+0x24`,
/// `+0x2e`, `+0x35`; `a` the third stack word), the vtable at `+0x3f`
/// into `*V`'s place (`V` at `+0x3b`), then [`colour_node_init`]
/// (`+0x44`). Returns its result.
pub fn colour_ctor<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.esp.wrapping_sub(0x20);
    let a = fr.arg(g, 2);
    record(
        g,
        rec,
        &[w(0x0f), w(0x16), w(0x1d), w(0x24), a, w(0x2e), w(0x35)],
    );
    g.write::<u32>(w(0x3b), w(0x3f));
    Ret::eax(colour_node_init(g, img, img.rel32_target(f + 0x44), rec))
}

/// `(_, _, _, a)` (`magic_0071` 0x10007530, 60 bytes): the record `(A,
/// B, 0, 0, a)` at `ebp - 0x14` (`+0x14`, `+0x1b`; `a` the fourth stack
/// word), the vtable at `+0x2f` into `V` (`+0x2b`), then
/// [`node_init_vec`] (`+0x34`). Returns its result.
pub fn vec_ctor<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.esp.wrapping_sub(0x18);
    let a = fr.arg(g, 3);
    record(g, rec, &[w(0x14), w(0x1b), 0, 0, a]);
    g.write::<u32>(w(0x2b), w(0x2f));
    Ret::eax(node_init_vec(g, img, img.rel32_target(f + 0x34), rec))
}

/// [`effect_ctor_arg`] in `magic_0159` (0x10006700, 68 bytes).
pub const EFFECT_CTOR_ARG_0159_6700: Sig = Sig {
    len: 68,
    hash: 0x9d66_40a1_d3e6_ddc3,
    calls: &[(0x33, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};

/// [`effect_ctor_arg2`] in `magic_0202` (0x10006f10, 68 bytes).
pub const EFFECT_CTOR_ARG2_0202_6F10: Sig = Sig {
    len: 68,
    hash: 0x0e65_9fb1_e3fd_005f,
    calls: &[(0x33, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};

/// [`effect_ctor_this`] in `magic_0388` (0x10006d70, 65 bytes).
pub const EFFECT_CTOR_THIS_0388_6D70: Sig = Sig {
    len: 65,
    hash: 0x2d4c_2988_181c_0da7,
    calls: &[(0x30, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};

/// [`big_ctor_this`] in `magic_0597` (0x10008f40, 59 bytes).
pub const BIG_CTOR_THIS_0597_8F40: Sig = Sig {
    len: 59,
    hash: 0xfb24_31ac_c3d4_1ffb,
    calls: &[(0x2c, Callee::Sig(&crate::m0070::NODE_INIT_BIG))],
    imports: &[],
};

/// [`big_table_ctor`] in `magic_0262` (0x10006c10, 53 bytes).
pub const BIG_TABLE_CTOR_0262_6C10: Sig = Sig {
    len: 53,
    hash: 0x4baf_814d_6dc5_ce2a,
    calls: &[(0x2d, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE))],
    imports: &[],
};

/// [`colour_ctor`] in `magic_0070` (0x100097b0, 76 bytes).
pub const COLOUR_CTOR_0070_97B0: Sig = Sig {
    len: 76,
    hash: 0x08a9_3195_c362_a130,
    calls: &[(0x44, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT))],
    imports: &[],
};

/// [`vec_ctor`] in `magic_0071` (0x10007530, 60 bytes).
pub const VEC_CTOR_0071_7530: Sig = Sig {
    len: 60,
    hash: 0x724d_be70_57fc_7be1,
    calls: &[(0x34, Callee::Sig(&crate::m0071::NODE_INIT_VEC))],
    imports: &[],
};
