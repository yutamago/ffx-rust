//! `magic_0203.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// A node's set-up (0x10007660, `ecx` = the node, cdecl `(a, b)`): the
/// vtable (the immediate `+0x05`), the pass `+0x1a` 0xff, its own child
/// ring (`+0x0c`, `+0x10`), the parent `R` (the immediate `+0x18`, also
/// `+0x04`) and linked at the end of `R`'s list (the last child `*L`,
/// `+0x24`; the first `*F` (`+0x39`) when the list was empty, else the
/// old last's `+0x04`), the count `*N` (`+0x45`, a halfword) up and `*L`
/// = the node; `a` and `b` into `+0x1c`/`+0x20`, `+0x24` 0xff. Returns
/// `b`.
pub fn node_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let n = fr.ecx;
    g.write::<u32>(n, w(0x05));
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, w(0x18));
    g.write::<u32>(n + 4, w(0x1f));
    let last = g.read::<u32>(w(0x24));
    g.write::<u32>(n + 8, last);
    let last = g.read::<u32>(w(0x2c));
    if last == w(0x31) {
        g.write::<u32>(w(0x39), n);
    } else {
        g.write::<u32>(last + 4, n);
    }
    let c = g.read::<u16>(w(0x45));
    g.write::<u16>(w(0x45), c.wrapping_add(1));
    g.write::<u32>(w(0x4e), n);
    let a = fr.arg(g, 0);
    g.write::<u32>(n + 0x1c, a);
    let b = fr.arg(g, 1);
    g.write::<u32>(n + 0x20, b);
    g.write::<u32>(n + 0x24, 0xff);
    Ret::eax(b)
}

/// [`node_setup`] (100 bytes, 1 overlay).
pub const NODE_SETUP: Sig = Sig {
    len: 100,
    hash: 0x5c2d_489d_d91b_d7a4,
    calls: &[],
    imports: &[],
};

/// A task step (0x100078c0, cdecl `(task)`, `r = task[4]`): nothing while
/// `r[0x1c]` is set (`eax` as it came) or slot 411 `(m)` says 0 (`m` the
/// word `+0x50` of `r[0x18]`); else slot 228 `(m, 0xb3)` and slot 213
/// `(m, 6)`, and the task's node `*task` leaves its ring. Returns the
/// node's old next (`+0x04`). Each slot call passes the words the
/// original leaves on the stack.
pub fn task_release<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    if g.read::<u32>(r + 0x1c) != 0 {
        return Ret::eax(fr.eax);
    }
    let m = g.read::<u32>(g.read::<u32>(r + 0x18) + 0x50);
    let s = slot_target(g, w(0x18), 411);
    let live = g.call(s, 0, &[m]).eax;
    if live == 0 {
        return Ret::eax(0);
    }
    let m = g.read::<u32>(g.read::<u32>(r + 0x18) + 0x50);
    let s = slot_target(g, w(0x37), 228);
    g.call(s, 0, &[m, 0xb3]);
    let m2 = g.read::<u32>(g.read::<u32>(r + 0x18) + 0x50);
    let s = slot_target(g, w(0x4c), 213);
    // call width: 4 (slot 228's words left on the stack)
    g.call(s, 0, &[m2, 6, m, 0xb3]);
    let n = g.read::<u32>(task);
    let (next, prev) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
    g.write::<u32>(next + 8, prev);
    let prev = g.read::<u32>(n + 8);
    let next = g.read::<u32>(n + 4);
    g.write::<u32>(prev + 4, next);
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 4, n);
    Ret::eax(next)
}

/// [`task_release`] (121 bytes, 1 overlay).
pub const TASK_RELEASE: Sig = Sig {
    len: 121,
    hash: 0xbd81_b35c_6cdf_ad6e,
    calls: &[],
    imports: &[],
};

/// A particle node's record set-up (0x100041b0, internal: `ecx` =
/// `[node, parent, a, table, b, index, c]`):
/// [`node_under`](crate::m0186::node_under); `+0x5c` = a, `+0x60` = b and
/// `**G` = b (`G` at `+0x45`), `+0xf8`/`+0xfc` 0; slot 941 `(table +
/// 0x10)`'s list gives the program `P = list[index]`, started at `+0x70`
/// ([`start_part`](crate::opu_parts::start_part) `(node + 0x70, 0x1000,
/// P, 1)`); slot 325 `(node + 0x1c, table + 16 (index + 2))`; `+0x104`
/// and `+0xf0` 0, `+0x108` = c and `+0x4c..+0x58` 4096.0. Returns c. Each
/// slot call passes the words the original leaves on the stack.
pub fn part_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = fr.ecx;
    let node = g.read::<u32>(r);
    let parent = g.read::<u32>(r + 4);
    crate::m0186::node_under(g, node, parent);
    let a = g.read::<u32>(r + 8);
    let b = g.read::<u32>(r + 0x10);
    g.write::<u32>(node + 0x5c, a);
    let gp = g.read::<u32>(w(0x45));
    g.write::<u32>(node + 0x60, b);
    g.write::<u32>(node + 0xf8, 0);
    g.write::<u32>(node + 0xfc, 0);
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
        node + 0x70,
        0x1000,
        prog,
        1,
    );
    let index = g.read::<u32>(r + 0x14);
    let table = g.read::<u32>(r + 0xc);
    let row = index.wrapping_add(2).wrapping_shl(4).wrapping_add(table);
    // the node is read again for this word; the stores below use the
    // first read (a register)
    let first = g.read::<u32>(r);
    p.push_all(&[row, first + 0x1c]);
    let s = slot_target(g, w(0xa0), 0x514 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let c = g.read::<u32>(r + 0x18);
    g.write::<u32>(node + 0x104, 0);
    g.write::<u32>(node + 0xf0, 0);
    g.write::<u32>(node + 0x108, c);
    for off in [0x4c, 0x50, 0x54, 0x58] {
        g.write::<u32>(node + off, 0x4580_0000);
    }
    Ret::eax(c)
}

/// [`part_record`] (235 bytes, 1 overlay).
pub const PART_RECORD: Sig = Sig {
    len: 235,
    hash: 0x2900_8859_7ebf_9422,
    calls: &[(0x88, crate::Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
