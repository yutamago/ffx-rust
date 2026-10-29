//! Node functions of the summon and node families ([`crate::nodes`],
//! [`crate::sb_nodes`]) ported by the shared-3 list. Each doc names the
//! DLL and address it was ported from, its size and how many DLLs carry
//! it.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Sig, sb_nodes};
use ffx_bridge::x87::cvttsd2si;

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// The node (`[this]`) leaves its parent (`+0x14`; the parent's child
/// count `u16 +0x18` goes down, its first/last child `+0x10`/`+0xc`
/// follow) and becomes a lone ring; then the display state is reset from
/// the node: slot 332 (`yiSetBackColor`, `node + 0x8c`), slot 353
/// (`yiSetColorScale`) with four 1.0 on the frame, slot 267
/// (`yiRenderDisable`, 4), slot 271 (`yiRenderEnable`, 1), slot 669 with
/// the node's four floats `+0x7c` truncated to integers on the frame,
/// [`sb_nodes::scale_ratio_801`] with four more 1.0 on the frame, and
/// slot 785 with the DLL's float (operand `+0xfc`). Each slot call sees
/// the words the earlier ones left on the stack, as in the original.
/// Returns slot 785's `eax`.
///
/// `magic_0003` 0x10001340 (291 bytes, `ecx` = the cursor; a /GS frame;
/// 107 DLLs).
pub fn leave_and_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let this = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let node = g.read::<u32>(this);
    let parent = g.read::<u32>(node + 0x14);
    if parent != 0 {
        let n = g.read::<u16>(parent + 0x18).wrapping_add(0xffff);
        g.write::<u16>(parent + 0x18, n);
        let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
        if prev == parent {
            g.write::<u32>(parent + 0x10, next);
        } else {
            g.write::<u32>(prev + 8, next);
        }
        let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
        if next == parent {
            g.write::<u32>(parent + 0xc, prev);
        } else {
            g.write::<u32>(next + 4, prev);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 4, node);
        g.write::<u32>(node + 0x14, 0);
    }
    let node = g.read::<u32>(this);
    let (scale, ints) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    for i in 0..4 {
        g.write::<u32>(scale + 4 * i, ONE);
        let x = f32::from_bits(g.read::<u32>(node + 0x7c + 4 * i));
        g.write::<u32>(ints + 4 * i, cvttsd2si(f64::from(x)));
    }
    let back = node + 0x8c;
    let s = slot_target(g, w(0x83), 332);
    g.call(s, 0, &[back]);
    let s = slot_target(g, w(0x94), 353);
    g.call(s, 0, &[scale, back]);
    let s = slot_target(g, w(0xa1), 267);
    g.call(s, 0, &[4, scale, back]);
    let s = slot_target(g, w(0xb0), 271);
    g.call(s, 0, &[1, 4, scale, back]);
    let s = slot_target(g, w(0xc3), 669);
    g.call(s, 0, &[ints, 1, 4, scale, back]);
    let four = ebp.wrapping_sub(0x34);
    for i in 0..4 {
        g.write::<u32>(four + 4 * i, ONE);
    }
    sb_nodes::scale_ratio_801(g, img, img.rel32_target(f + 0xef), four);
    let s = slot_target(g, w(0xf4), 785);
    let k = g.read::<u32>(w(0xfc));
    g.call(s, 0, &[k]).eax
}

/// [`leave_and_reset`].
pub const LEAVE_AND_RESET: Sig = Sig {
    len: 291,
    hash: 0x1a60_d5e6_59f5_eb5b,
    calls: &[
        (0xef, Callee::Sig(&sb_nodes::SCALE_RATIO_801)),
        (0x11b, Callee::Any),
    ],
    imports: &[],
};

/// A set-up record on the frame (`[ebp - 0x18]`, six words) handed to a
/// node builder in `ecx`.
fn record_on_frame<G: Game>(g: &mut G, fr: &Frame, words: [u32; 6]) -> u32 {
    // push ebp; sub esp, 0x18
    let rec = fr.esp.wrapping_sub(0x1c);
    for (i, w) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, w);
    }
    rec
}

/// A particle node (`ecx`) set up: its vtable (operand `+0x1c`), then
/// [`sb_nodes::particle_node`] with the record `[node, the DLL's list
/// (+0x26), arg 0, the DLL's owner global's word (+0x0d), the DLL's part
/// (+0x2d), arg 3]`. Returns its `eax`.
///
/// `magic_0042` 0x100083c0 (61 bytes, `ecx` = the node, stack `(a, _, _,
/// index)`; 35 DLLs).
pub fn particle_node_with<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |o: u32| img.word(fr.at + o);
    let node = fr.ecx;
    let owner = g.read::<u32>(w(0x0d));
    let words = [node, w(0x26), fr.arg(g, 0), owner, w(0x2d), fr.arg(g, 3)];
    g.write::<u32>(node, w(0x1c));
    let rec = record_on_frame(g, fr, words);
    sb_nodes::particle_node(g, img, img.rel32_target(fr.at + 0x35), rec)
}

/// [`particle_node_with`].
pub const PARTICLE_NODE_WITH: Sig = Sig {
    len: 61,
    hash: 0x88fc_eb22_06fd_9f65,
    calls: &[(0x35, Callee::Sig(&sb_nodes::PARTICLE_NODE))],
    imports: &[],
};

/// An effect node (`ecx`) set up: [`sb_nodes::effect_node`] with the
/// record `[node, the DLL's list (+0x1e), arg 0, arg 1, the DLL's word
/// (+0x25), arg 3]`, then the node (the record's first word, read again)
/// gets its vtable (operand `+0x36`). Returns the node.
///
/// `magic_0042` 0x10008570 (62 bytes, `ecx` = the node, stack `(a, b, _,
/// d)`; 15 DLLs).
pub fn effect_node_with<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |o: u32| img.word(fr.at + o);
    let words = [
        fr.ecx,
        w(0x1e),
        fr.arg(g, 0),
        fr.arg(g, 1),
        w(0x25),
        fr.arg(g, 3),
    ];
    let rec = record_on_frame(g, fr, words);
    sb_nodes::effect_node(g, img, img.rel32_target(fr.at + 0x2d), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x36));
    node
}

/// [`effect_node_with`].
pub const EFFECT_NODE_WITH: Sig = Sig {
    len: 62,
    hash: 0xf58c_3170_7876_f3a9,
    calls: &[(0x2d, Callee::Sig(&sb_nodes::EFFECT_NODE))],
    imports: &[],
};
