//! `magic_0146.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::l3_common::{ListAt, attach};
use crate::runtime::slot_target;
use crate::sa_summon::{MODEL_NODE_INIT, model_node_init};
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Sig};

/// The model nodes of a group with a picked one (0x10006000, internal:
/// `ecx` = the group `{node, parent, actor, word, flags byte}`): the node
/// set up as a child of the parent (pass byte `+0x1a` 0xff, an empty
/// child list, appended to the parent's list when there is one); the
/// word, actor and flags kept (`+0x4e8`, `+0x4ec`, `+0x4f0`); slot 218
/// `(actor, &count)` gives a model number `pick` (its first byte), slot
/// 368 `(actor, &count)` the list of model numbers (`+0x4e4`, the count
/// at `+0x4e0`); each becomes a model node (`+0x1c + 0x98 i`, the vtable
/// at operand `+0xbd`) through [`model_node_init`] with a record on the
/// frame `[node, the group's parent, the number, flags 0]`, and the one
/// whose number is `pick`'s byte becomes `+0x4dc`. Returns slot 368's
/// `eax` for no models, else `pick` with its low byte the byte it points
/// at.
///
/// Each slot call passes the words the original leaves on the stack.
pub fn model_nodes_pick<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let group = fr.ecx;
    let node = g.read::<u32>(group);
    let parent = g.read::<u32>(group + 4);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        attach(g, node, parent, ListAt::of(parent));
    }
    let word = g.read::<u32>(group + 0xc);
    let actor = g.read::<u32>(group + 8);
    g.write::<u32>(node + 0x4e8, word);
    g.write::<u8>(node + 0x4f0, g.read::<u8>(group + 0x10));
    let count = node + 0x4e0;
    g.write::<u32>(node + 0x4ec, actor);
    let s = slot_target(g, w(0x5b), 218);
    let pick = g.call(s, 0, &[actor, count]).eax;
    let a = g.read::<u32>(node + 0x4ec);
    let s = slot_target(g, w(0x77), 368);
    let mut eax = g.call(s, 0, &[a, count, actor, count]).eax;
    g.write::<u32>(node + 0x4e4, eax);
    // push ebp; sub esp, 0x1c; three pushes: the record at [ebp - 0x18]
    let rec = fr.esp.wrapping_sub(0x1c);
    let init = img.rel32_target(f + 0xc2);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(count) {
        let model = node + 0x1c + 0x98 * i;
        let list = g.read::<u32>(node + 0x4e4);
        g.write::<u32>(rec, model);
        g.write::<u32>(rec + 4, g.read::<u32>(group + 4));
        g.write::<u32>(rec + 8, u32::from(g.read::<u8>(list.wrapping_add(i))));
        g.write::<u8>(rec + 0xc, 0);
        g.write::<u32>(model, w(0xbd));
        model_node_init(g, img, init, rec);
        let b = g.read::<u8>(pick);
        eax = (pick & !0xff) | u32::from(b);
        let list = g.read::<u32>(node + 0x4e4);
        if b == g.read::<u8>(list.wrapping_add(i)) {
            g.write::<u32>(node + 0x4dc, model);
        }
        i += 1;
    }
    eax
}

/// [`model_nodes_pick`] (242 bytes, 1 overlay).
pub const MODEL_NODES_PICK: Sig = Sig {
    len: 242,
    hash: 0xa8fc_b2dc_5aff_b2c2,
    calls: &[(0xc2, Callee::Sig(&MODEL_NODE_INIT))],
    imports: &[],
};

/// Starts a node's move (0x10006ba0, internal: `ecx` = the node, `xmm2`
/// = its time): flags `+0xac` lose bit 0, `+0x9c` the time, `+0xa0` 0,
/// and the deltas `+0x3c..+0x48 = +0x2c.. - +0x1c..` and `+0x6c..+0x78 =
/// +0x5c.. - +0x4c..` (interleaved as the original stores them). Returns
/// the caller's `eax`.
pub fn move_start(g: &mut impl crate::Mem, node: u32, time: f32) {
    let v = g.read::<u32>(node + 0xac);
    g.write::<u32>(node + 0xac, v & !1);
    g.write::<u32>(node + 0x9c, time.to_bits());
    g.write::<u32>(node + 0xa0, 0);
    for i in 0..4 {
        for (dst, a, b) in [(0x3c, 0x2c, 0x1c), (0x6c, 0x5c, 0x4c)] {
            let d = sub(
                g.read::<f32>(node + a + 4 * i),
                g.read::<f32>(node + b + 4 * i),
            );
            g.write::<u32>(node + dst + 4 * i, d.to_bits());
        }
    }
}

/// [`move_start`] (146 bytes, 1 overlay).
pub const MOVE_START: Sig = Sig {
    len: 146,
    hash: 0x3b6f_66de_4b3e_bf7e,
    calls: &[],
    imports: &[],
};

/// A node handler (0x10006190, `ecx` = the cursor: node, moved flag,
/// pass byte; an aligned /GS frame): the walk (each child of the
/// cursor's node gets its method `+0x1c` with the cursor; then the node
/// takes the pass). When the cursor moved, the node's three sub-nodes
/// `+0x44`, `+0x48`, `+0x4c` are visited with a cursor on the frame
/// (`esp + 8`, their method `+0x1c`); the game's function at the DLL's
/// pointer global (operand `+0x10a`) copies the position of sub-node
/// `+0x48` into a local `w` (`esp + 0x50`); slot 229 gets `(v, w, v)`
/// with `v` = `(0, 1, 0, 1)` (`esp + 0x40`). With bit 0 of `+0x54`,
/// slot 677 gets `(node + 0x30, node[0x50], sub44 + 0x1c, v)` and the
/// bit is cleared. Slot 679 gets a record on the frame (`esp + 0x18`):
/// `node + 0x30`, `sub4c + 0x1c`, `node[0x40]`, `node[0x50]`,
/// `sub44 + 0x1c`, `v`. Slot 680 gets `(node + 0x30, A, s, node[0x40],
/// B, 0x48, 0, 0, 0, node + 0x1c, node)` with `s` = `(1.0, 1.0833)`
/// (`esp + 0x10`; `A`, `B` the immediates `+0x1c0`, `+0x1ae`). Returns
/// slot 680's `eax`, else the walk's (the low byte the pass, the rest
/// what the frame's cookie or the last method left).
///
/// `magic_0173` 0x10006000 ([`SUB_NODE_VISIT_0173`]): `v` = `(0, 0, -1,
/// 1)`, and the copy and slot 229 come after the slot 677 block.
#[allow(clippy::too_many_lines)]
pub fn sub_node_visit<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let cur = fr.ecx;
    // push ebp; and esp, -16; sub esp, 0x98: `e`; the cookie is
    // `*COOKIE ^ e`
    let e = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x98);
    let mut eax = g.read::<u32>(w(0x0d)) ^ e;
    let (v, s, d, rec, wv) = (e + 0x40, e + 0x10, e + 8, e + 0x18, e + 0x50);
    // `v`'s immediates (`magic_0173`: `(0, 0, -1, 1)`)
    for (i, op) in (0u32..).zip([0x21, 0x2b, 0x33, 0x3b]) {
        g.write::<u32>(v + 4 * i, w(op));
    }
    // `magic_0173` 0x10006000: the copy and slot 229 after the slot 677
    // block (`call [copy]` at `+0x135`)
    let late = w(0x135) as u16 == 0x15ff;
    g.write::<u32>(s, 0x3f80_0000);
    g.write::<u32>(s + 4, 0x3f8a_aaab);
    let node = g.read::<u32>(cur);
    let pass = g.read::<u8>(node + 0x1a);
    g.write::<u32>(d, node);
    if pass == g.read::<u8>(cur + 5) {
        g.write::<u8>(cur + 4, 0);
        eax = (eax & !0xff) | u32::from(pass);
    } else {
        let mut child = g.read::<u32>(node + 0x10);
        while child != node {
            g.write::<u32>(cur, child);
            let c = g.read::<u32>(cur);
            child = g.read::<u32>(child + 8);
            let m = g.read::<u32>(g.read::<u32>(c) + 0x1c);
            eax = g.call(m, node, &[cur]).eax;
        }
        let pass = g.read::<u8>(cur + 5);
        g.write::<u32>(cur, node);
        g.write::<u8>(node + 0x1a, pass);
        g.write::<u8>(cur + 4, 1);
        eax = (eax & !0xff) | u32::from(pass);
    }
    if g.read::<u8>(cur + 4) & 1 == 0 {
        return crate::Ret::eax(eax);
    }
    let node = g.read::<u32>(cur);
    for off in [0x44, 0x48, 0x4c] {
        let pass = g.read::<u8>(cur + 5);
        let sub = g.read::<u32>(node + off);
        g.write::<u32>(d, sub);
        g.write::<u8>(d + 5, pass);
        let m = g.read::<u32>(g.read::<u32>(sub) + 0x1c);
        g.call(m, d, &[d]);
    }
    let copy_229 = |g: &mut G, (c, t): (u32, u32)| {
        let src = g.read::<u32>(node + 0x48).wrapping_add(0x1c);
        let copy = g.read::<u32>(w(c));
        g.call(copy, d, &[wv, src]);
        let s229 = slot_target(g, w(t), 229);
        g.call(s229, 0, &[v, wv, v, wv, src]);
    };
    if !late {
        copy_229(g, (0x10a, 0x11e));
    }
    if g.read::<u8>(node + 0x54) & 1 != 0 {
        let a44 = g.read::<u32>(node + 0x44).wrapping_add(0x1c);
        let x50 = g.read::<u32>(node + 0x50);
        let s677 = slot_target(g, w(if late { 0x116 } else { 0x147 }), 677);
        // call width: 4
        g.call(s677, 0, &[node + 0x30, x50, a44, v]);
        let b = g.read::<u8>(node + 0x54);
        g.write::<u8>(node + 0x54, b & !1);
    }
    if late {
        copy_229(g, (0x137, 0x14b));
    }
    let r = [
        node + 0x30,
        g.read::<u32>(node + 0x4c).wrapping_add(0x1c),
        g.read::<u32>(node + 0x40),
        g.read::<u32>(node + 0x50),
        g.read::<u32>(node + 0x44).wrapping_add(0x1c),
        v,
    ];
    // the stores' order: +0, +4, +0xc, +8, +0x10, +0x14
    for (i, x) in [
        (0u32, r[0]),
        (1, r[1]),
        (3, r[3]),
        (2, r[2]),
        (4, r[4]),
        (5, r[5]),
    ] {
        g.write::<u32>(rec + 4 * i, x);
    }
    let s679 = slot_target(g, w(0x191), 679);
    g.call(s679, 0, &[rec]);
    let x40 = g.read::<u32>(node + 0x40);
    let s680 = slot_target(g, w(0x1bb), 680);
    let words = [
        node + 0x30,
        w(0x1c0),
        s,
        x40,
        w(0x1ae),
        0x48,
        0,
        0,
        0,
        node + 0x1c,
        node,
    ];
    // call width: 11
    crate::Ret::eax(g.call(s680, 0, &words).eax)
}

/// [`sub_node_visit`] (484 bytes, 1 overlay; the `rel32` at `+0x1dc` is
/// `__security_check_cookie`).
pub const SUB_NODE_VISIT: Sig = Sig {
    len: 484,
    hash: 0xe3ce_1e70_bf1e_1c7f,
    calls: &[(0x1dc, Callee::Any)],
    imports: &[],
};

/// [`sub_node_visit`] at `magic_0173` 0x10006000 (agent magic-r26): `v`
/// `(0, 0, -1, 1)`, the copy and slot 229 after the slot 677 block.
pub const SUB_NODE_VISIT_0173: Sig = Sig {
    len: 484,
    hash: 0xe665_0567_ae20_0abb,
    calls: &[(0x1dc, Callee::Any)],
    imports: &[],
};
