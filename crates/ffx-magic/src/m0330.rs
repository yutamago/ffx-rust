//! `magic_0330.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The model nodes of a group (0x10005db0, internal: `ecx` = `[node,
/// parent, actor, word, flags]`; an aligned frame; `s3_summon::model_nodes`
/// with its block at `+0x261c`): the node under its parent
/// ([`node_under`](crate::m0186::node_under)), the word, flags byte and
/// actor kept (`+0x2624`, `+0x262c`, `+0x2628`), slot 412 `(actor, node +
/// 0x261c, 1)` gives the model list (`+0x2620`, its count into
/// `+0x261c`), and each model becomes a model node (`+0x1c + 0x98 i`, the
/// vtable `+0xb8`) through
/// [`model_node_init`](crate::sa_summon::model_node_init) with a record on
/// the frame `[node, the group's parent, the number, flags 0]`. Returns
/// the last call's `eax`.
pub fn model_nodes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let grp = fr.ecx;
    let node = g.read::<u32>(grp);
    let parent = g.read::<u32>(grp + 4);
    crate::m0186::node_under(g, node, parent);
    let word = g.read::<u32>(grp + 0xc);
    let actor = g.read::<u32>(grp + 8);
    g.write::<u32>(node + 0x2624, word);
    g.write::<u8>(node + 0x262c, g.read::<u8>(grp + 0x10));
    let s = slot_target(g, w(0x5f), 0x670 / 4);
    g.write::<u32>(node + 0x2628, actor);
    let mut eax = g.call(s, 0, &[actor, node + 0x261c, 1]).eax;
    g.write::<u32>(node + 0x2620, eax);
    // push ebp; and esp, -8; sub esp, 0x14; three pushes: the record at
    // [esp + 0x10]
    let rec = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x10);
    let init = img.rel32_target(f + 0xbd);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(node + 0x261c) {
        let sub = node + 0x1c + 0x98 * i;
        g.write::<u32>(rec + 4, g.read::<u32>(grp + 4));
        let list = g.read::<u32>(node + 0x2620);
        g.write::<u32>(rec, sub);
        g.write::<u32>(rec + 8, u32::from(g.read::<u8>(list.wrapping_add(i))));
        g.write::<u8>(rec + 0xc, 0);
        g.write::<u32>(sub, w(0xb8));
        eax = crate::sa_summon::model_node_init(g, img, init, rec);
        i += 1;
    }
    Ret::eax(eax)
}

/// [`model_nodes`] (215 bytes, 1 overlay).
pub const MODEL_NODES: Sig = Sig {
    len: 215,
    hash: 0x5840_01f9_d32b_5760,
    calls: &[(0xbd, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT))],
    imports: &[],
};
