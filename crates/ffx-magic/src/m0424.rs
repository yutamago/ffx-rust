//! `magic_0424.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A node's release (0x100066b0, internal: `ecx` = `[node]`; no frame):
/// slot 376 `(node +0xf0)`; then, with a parent (`+0x14`), the node
/// leaves it (the parent's count `+0x18` drops, the neighbours joined,
/// the parent's first/last `+0xc`/`+0x10` where the node was at an end)
/// and is left alone (`+4`, `+8` itself, no parent). Returns the last
/// word the code read (slot 376's answer without a parent).
pub fn release<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let node = g.read::<u32>(fr.ecx);
    let s = slot_target(g, w(0x0c), 0x5e0 / 4);
    let x = g.read::<u32>(node + 0xf0);
    let eax = g.call(s, 0, &[x]).eax;
    let node = g.read::<u32>(fr.ecx);
    let parent = g.read::<u32>(node + 0x14);
    if parent == 0 {
        return Ret::eax(eax);
    }
    let n = g.read::<u16>(parent + 0x18);
    g.write::<u16>(parent + 0x18, n.wrapping_add(0xffff));
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node + 8);
    if prev == parent {
        g.write::<u32>(parent + 0x10, next);
    } else {
        g.write::<u32>(prev + 8, next);
    }
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    if next == parent {
        g.write::<u32>(parent + 0xc, prev);
    } else {
        g.write::<u32>(next + 4, prev);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(node + 0x14, 0);
    Ret::eax(prev)
}

/// [`release`] (109 bytes, 1 overlay).
pub const RELEASE: Sig = Sig {
    len: 109,
    hash: 0x71c2_1a8f_3c9a_f45f,
    calls: &[],
    imports: &[],
};

/// A particle node's record set-up (0x100065a0, internal: `ecx` =
/// `[node, parent, a, table, b, index]`; no frame;
/// `m0203::part_record`'s kin): [`node_under`](crate::m0186::node_under);
/// `+0x4c` = a, `+0x50` = b and `**G` = b (`G` `+0x45`), `+0xe8`/`+0xec`
/// 0; slot 941 `(table + 0x10)`'s list gives the program `list[index]`,
/// started at `+0x60` ([`start_part`](crate::opu_parts::start_part)
/// `(node + 0x60, 0x1000, P, 1)`); slot 325 `(node + 0x1c, table + 16
/// (index + 2))`; `+0xf8`/`+0xe0` 0; slot 942 `(node)` and slot 942 `(0)`
/// go to slot 377 `(F, n, z, 0)` (`F` `+0xef`) into `+0xf0`. Returns
/// slot 377's `eax`. Each slot call passes the words the original leaves
/// on the stack.
pub fn part_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = fr.ecx;
    let node = g.read::<u32>(r);
    crate::m0186::node_under(g, node, g.read::<u32>(r + 4));
    let a = g.read::<u32>(r + 8);
    let b = g.read::<u32>(r + 0x10);
    g.write::<u32>(node + 0x4c, a);
    let gp = g.read::<u32>(w(0x45));
    g.write::<u32>(node + 0x50, b);
    g.write::<u32>(node + 0xe8, 0);
    g.write::<u32>(node + 0xec, 0);
    g.write::<u32>(gp, b);
    let table = g.read::<u32>(r + 0xc);
    let mut p = crate::pushes::Pushes::default();
    p.push(g.read::<u32>(table + 0x10));
    let s = slot_target(g, w(0x69), 0xeb4 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let list = p.call(g, s, 0).eax;
    let index = g.read::<u32>(r + 0x14);
    let prog = g.read::<u32>(list.wrapping_add(index.wrapping_mul(4)));
    p.push_all(&[0, 1, prog]);
    crate::opu_parts::start_part(
        g,
        img,
        img.rel32_target(f + 0x88),
        node + 0x60,
        0x1000,
        prog,
        1,
    );
    let index = g.read::<u32>(r + 0x14);
    let table = g.read::<u32>(r + 0xc);
    let row = index.wrapping_add(2).wrapping_shl(4).wrapping_add(table);
    let first = g.read::<u32>(r);
    p.push_all(&[row, first + 0x1c]);
    let s = slot_target(g, w(0xa0), 0x514 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    g.write::<u32>(node + 0xf8, 0);
    g.write::<u32>(node + 0xe0, 0);
    p.push(node);
    let s = slot_target(g, w(0xad), 0xeb8 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let n = p.call(g, s, 0).eax;
    p.push(0);
    let s = slot_target(g, w(0xd1), 0xeb8 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let z = p.call(g, s, 0).eax;
    p.push_all(&[0, z, n, w(0xef)]);
    let s = slot_target(g, w(0xe3), 0x5e4 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let eax = p.call(g, s, 0).eax;
    g.write::<u32>(node + 0xf0, eax);
    Ret::eax(eax)
}

/// [`part_record`] (257 bytes, 1 overlay).
pub const PART_RECORD: Sig = Sig {
    len: 257,
    hash: 0x4c5d_e77d_f344_5341,
    calls: &[(0x88, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
