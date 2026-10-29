//! More of the summon family ([`crate::sa_summon`]: `magic_0003`'s code,
//! 38 to 53 overlays), ported by the shared-3 list. Addresses are
//! `magic_0003`'s.

use crate::l3_common::{ListAt, attach};
use crate::nodes::{GATED, visit_gated};
use crate::runtime::slot_target;
use crate::sa_summon::{
    BIND_MODELS, BONE, MODEL_NODE_INIT, bind_models, bone_plain, model_node_init,
};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The model nodes of a group (`ecx`: `+0x0` the node, `+0x4` its
/// parent, `+0x8` the actor, `+0xc` a word, `+0x10` a flags byte): the
/// node set up as a child of the parent (pass byte `+0x1a` 0xff, an
/// empty child list, appended to the parent's list when there is one),
/// the actor, word and flags kept (`+0x4e8`, `+0x4e4`, `+0x4ec`), slot
/// 218 gives the actor's list of model numbers (`+0x4e0`, the count into
/// `+0x4dc`), and each one becomes a model node (`+0x1c + 0x98 * i`, the
/// DLL's vtable at operand `+0xa9`) through
/// [`model_node_init`] with a record on the frame `[node, the group's
/// parent, the number, flags 0]` (only the flags' low byte is written,
/// and only it is read). Returns the last call's `eax` (slot 218's for no
/// models).
///
/// `magic_0003` 0x10005db0 (200 bytes, `ecx` = the group; 38 DLLs).
pub fn model_nodes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let group = fr.ecx;
    let parent = g.read::<u32>(group + 4);
    let node = g.read::<u32>(group);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        attach(g, node, parent, ListAt::of(parent));
    }
    g.write::<u32>(node + 0x4e4, g.read::<u32>(group + 0xc));
    let actor = g.read::<u32>(group + 8);
    g.write::<u8>(node + 0x4ec, g.read::<u8>(group + 0x10));
    g.write::<u32>(node + 0x4e8, actor);
    let s = slot_target(g, img.word(f + 0x5b), 218);
    let mut eax = g.call(s, 0, &[actor, node + 0x4dc]).eax;
    g.write::<u32>(node + 0x4e0, eax);
    // push ebp; sub esp, 0x14: the record at [ebp - 0x14]
    let rec = fr.esp.wrapping_sub(0x18);
    let init = img.rel32_target(f + 0xae);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(node + 0x4dc) {
        let sub = node + 0x1c + 0x98 * i;
        let parent = g.read::<u32>(group + 4);
        g.write::<u32>(rec + 4, parent);
        let list = g.read::<u32>(node + 0x4e0);
        g.write::<u32>(rec, sub);
        g.write::<u32>(rec + 8, u32::from(g.read::<u8>(list.wrapping_add(i))));
        g.write::<u8>(rec + 0xc, 0);
        g.write::<u32>(sub, img.word(f + 0xa9));
        eax = model_node_init(g, img, init, rec);
        i += 1;
    }
    eax
}

/// [`model_nodes`].
pub const MODEL_NODES: Sig = Sig {
    len: 200,
    hash: 0x2e72_3a5d_98c1_d4f8,
    calls: &[(0xae, Callee::Sig(&MODEL_NODE_INIT))],
    imports: &[],
};

/// The summon's frame (a table entry, no arguments; nothing when the
/// DLL's phase word, operand `+0x0b`, is below 4): the DLL's pointer
/// (`+0x18`) gets its block (`+0x1e`), a word is copied (`+0x23` to
/// `+0x28`), [`bind_models`], the pass byte (`+0x32`) goes up; unless the
/// DLL's flag (`+0x40`) is set, every child of the DLL's root (`+0x4e`,
/// its first child `+0x38`) gets its method `+0x20` with a cursor on the
/// frame; with no child left the done bit 0 of `+0x89` is set. Then three
/// static nodes are visited with the new pass: `+0x96` through
/// [`bone_plain`], `+0xb7` and `+0xd7` through [`visit_gated`]; when the
/// done bit is set and the halfword `+0xef` is 0, slot 214 `(+0xfd)`.
/// Returns 0.
///
/// The children's cursor gets only its node word: its pass byte is never
/// written (`[esp + 0xd]`, junk read: 0; brief 2026-10-24), and the
/// methods called through it read it.
///
/// `magic_0003` 0x10006b70 (275 bytes, an aligned frame; 53 DLLs).
pub fn summon_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    if g.read::<i32>(w(0x0b)) < 4 {
        return Ret::eax(0);
    }
    g.write::<u32>(g.read::<u32>(w(0x18)), w(0x1e));
    g.write::<u32>(w(0x28), g.read::<u32>(w(0x23)));
    bind_models(g, img, img.rel32_target(f + 0x2d));
    let pass = g.read::<u8>(w(0x32)).wrapping_add(1);
    let mut child = g.read::<u32>(w(0x38));
    g.write::<u8>(w(0x46), pass);
    // push ebp; and esp, -8; sub esp, 0xc; push esi: the cursor at
    // [esp + 8]
    let e = fr.esp.wrapping_sub(4) & !7;
    let cursor = e.wrapping_sub(8);
    let root = w(0x4e);
    let mut pass = pass;
    if g.read::<u32>(w(0x40)) == 0 && child != root {
        // junk read: [esp + 0xc] never written; 0 (brief 2026-10-24)
        g.write::<u32>(cursor + 4, 0);
        loop {
            g.write::<u32>(cursor, child);
            let method = g.read::<u32>(g.read::<u32>(child) + 0x20);
            child = g.read::<u32>(child + 8);
            // push ecx: the cursor's address, then the call
            g.write::<u32>(e.wrapping_sub(0x14), cursor);
            g.call(method, cursor, &[cursor]);
            if child == root {
                break;
            }
        }
        pass = g.read::<u8>(w(0x75));
        child = g.read::<u32>(w(0x7b));
    }
    if child == root {
        let done = w(0x89);
        g.write::<u32>(done, g.read::<u32>(done) | 1);
    }
    g.write::<u32>(cursor, w(0x96));
    g.write::<u8>(cursor + 5, pass);
    let bone = Frame {
        at: img.rel32_target(f + 0x9f),
        esp: e.wrapping_sub(0x14),
        ecx: cursor,
        ..*fr
    };
    bone_plain(g, img, &bone);
    for (pass_at, node_at, call_at) in [(0xa6u32, 0xb7u32, 0xbcu32), (0xc3, 0xd7, 0xdc)] {
        let pass = g.read::<u8>(w(pass_at));
        g.write::<u8>(cursor + 5, pass);
        // push eax: the cursor's address
        g.write::<u32>(e.wrapping_sub(0x14), cursor);
        g.write::<u32>(cursor, w(node_at));
        let fr = Frame {
            at: img.rel32_target(f + call_at),
            esp: e.wrapping_sub(0x18),
            ..*fr
        };
        visit_gated(g, img, &fr);
    }
    if g.read::<u32>(w(0xe5)) & 1 != 0 && g.read::<u16>(w(0xef)) == 0 {
        let s = slot_target(g, w(0xf7), 214);
        let x = g.read::<u32>(w(0xfd));
        g.call(s, 0, &[x]);
    }
    Ret::eax(0)
}

/// [`summon_frame`].
pub const SUMMON_FRAME: Sig = Sig {
    len: 275,
    hash: 0xe144_2023_8c49_2f00,
    calls: &[
        (0x2d, Callee::Sig(&BIND_MODELS)),
        (0x9f, Callee::Sig(&BONE)),
        (0xbc, Callee::Sig(&GATED)),
        (0xdc, Callee::Sig(&GATED)),
    ],
    imports: &[],
};
