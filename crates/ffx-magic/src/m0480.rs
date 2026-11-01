//! `magic_0480.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sa_pool::give_back;
use crate::{Callee, Game, Image, Sig};

/// Clears the effect's lights and frees its node (0x10002760, internal,
/// no arguments): for each 16-byte entry `e` of the table at operand
/// `+0x05` (up to `+0x3a`), slot 590 `(i, e - 0x30)` and slot 589 `(i,
/// e)`; then slot 589 `(n, T + 16 n)` for the index past the table. The
/// node `*G` (`+0x5b`), when there is one, goes back to the pool as
/// [`give_back`] does it, inlined (its children through `give_back`
/// itself, the pool's globals at the operands here), and `*G` becomes 0.
/// Returns the `eax` the original leaves (the free count in the low half
/// when a node was freed).
pub fn clear<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut i = 0u32;
    let mut e = w(0x05);
    loop {
        let s590 = slot_target(g, w(0x15), 590);
        g.call(s590, 0, &[i, e.wrapping_sub(0x30)]);
        let s589 = slot_target(g, w(0x23), 589);
        g.call(s589, 0, &[i, e, i, e.wrapping_sub(0x30)]);
        e = e.wrapping_add(0x10);
        i += 1;
        if e.cast_signed() >= w(0x3a).cast_signed() {
            break;
        }
    }
    let s589 = slot_target(g, w(0x4c), 589);
    let mut eax = g.call(s589, 0, &[i, (i << 4).wrapping_add(w(0x46))]).eax;
    let node = g.read::<u32>(w(0x5b));
    if node == 0 {
        return eax;
    }
    eax = g.read::<u32>(node + 8);
    if g.read::<u32>(eax + 0xc) != 0 {
        loop {
            let child = g.read::<u32>(node + 8);
            let next = g.read::<u32>(child + 0xc);
            g.write::<u32>(child + 0xc, 0);
            let c = g.read::<u32>(node + 8);
            eax = give_back(g, img, img.rel32_target(f + 0x84), c);
            g.write::<u32>(node + 8, next);
            if g.read::<u32>(next + 0xc) == 0 {
                break;
            }
        }
    }
    let prev = g.read::<u32>(node);
    if prev != 0 {
        eax = g.read::<u32>(node + 4);
        g.write::<u32>(prev + 4, eax);
    }
    let next = g.read::<u32>(node + 4);
    if next != 0 {
        eax = g.read::<u32>(node);
        g.write::<u32>(next, eax);
    }
    let mut cur = g.read::<u32>(w(0xaa));
    if cur == node {
        cur = g.read::<u32>(node + 4);
    }
    if g.read::<u32>(w(0xb7)) == node {
        eax = g.read::<u32>(node);
        g.write::<u32>(w(0xc0), eax);
    }
    if g.read::<u32>(w(0xc6)) == node {
        eax = g.read::<u32>(node + 4);
        g.write::<u32>(w(0xd0), eax);
    }
    let head = g.read::<u32>(w(0xd6));
    if head == 0 {
        g.write::<u32>(node + 4, node);
        g.write::<u32>(node, node);
    } else {
        g.write::<u32>(node + 4, head);
        eax = g.read::<u32>(head);
        g.write::<u32>(node, eax);
        g.write::<u32>(head, node);
    }
    let count = g.read::<u16>(w(0xf0)).wrapping_add(1);
    if count == 0x100 {
        cur = 0;
    }
    g.write::<u32>(w(0x105), node);
    g.write::<u16>(w(0x10b), count);
    g.write::<u32>(w(0x111), cur);
    g.write::<u32>(w(0x117), 0);
    (eax & 0xffff_0000) | u32::from(count)
}

/// [`clear`] (286 bytes, 1 overlay).
pub const CLEAR: Sig = Sig {
    len: 286,
    hash: 0x83f3_6084_63a9_14a4,
    calls: &[(0x84, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// The effect's per-frame task callback (0x100043a0, cdecl `(node)`):
/// the script runs ([`run`](crate::sa_script::run)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both say 1
/// the lights are cleared ([`clear`]), the node's `+0x14` = 3 and its
/// task's `+0x10` the callback (the immediate `+0x2e`), returning the
/// task; else the objects' answer.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    let f = fr.at;
    let s = crate::sa_script::run(g, img, img.rel32_target(f + 0x5));
    // push ebp, esi, the return address
    let r =
        crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0xc), fr.esp.wrapping_sub(12));
    if r != 1 || s != 1 {
        return crate::Ret::eax(r);
    }
    clear(g, img, img.rel32_target(f + 0x1a));
    let node = fr.arg(g, 0);
    g.write::<u32>(node.wrapping_add(0x14), 3);
    let task = g.read::<u32>(node.wrapping_add(8));
    g.write::<u32>(task.wrapping_add(0x10), img.word(f + 0x2e));
    crate::Ret::eax(task)
}

/// [`task`] (53 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 53,
    hash: 0xce0e_88e5_39ce_40c1,
    calls: &[
        (0x5, Callee::Sig(&crate::sa_script::RUN)),
        (0xc, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x1a, Callee::Sig(&CLEAR)),
    ],
    imports: &[],
};

/// Resets the records (0x10004530, an overlay table entry, no arguments;
/// a cookie frame): the local `(x, y, z, w)` at `ebp - 0x14` given to
/// the two record sets ([`record_copy`](crate::o4_fam3::record_copy),
/// `edx` = the index): `(0, -2.39)` for record 1 and `(0, 1.75)` for 2,
/// then nine globals (operands `+0x5a`..`+0xaa`: three 2.0 pairs of
/// rows, three 0) and `(0, 0, 0)` to the second set's records 1, 2, 3.
/// The local's last word is never written (a junk read the record set
/// copies: the port reads the same stack word). Returns the last set's
/// `eax`.
pub fn records_reset<G: Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let local = fr.esp.wrapping_sub(0x18);
    let (a, b) = (img.rel32_target(f + 0x29), img.rel32_target(f + 0xc8));
    for (i, y) in [(1, 0xc019_07cd), (2, 0x3fdf_66f3)] {
        g.write::<u32>(local, 0);
        g.write::<u32>(local + 4, y);
        crate::o4_fam3::record_copy(g, img, a, i, local);
    }
    for op in [0x5a, 0x64, 0x6e, 0x78, 0x82, 0x8c] {
        g.write::<u32>(w(op), 0x4000_0000);
    }
    for op in [0x96, 0xa0, 0xaa] {
        g.write::<u32>(w(op), 0);
    }
    for o in [0, 4, 8] {
        g.write::<u32>(local + o, 0);
    }
    let mut eax = 0;
    for i in 1..=3 {
        eax = crate::o4_fam3::record_copy(g, img, b, i, local);
    }
    crate::Ret::eax(eax)
}

/// [`records_reset`] (255 bytes, 1 overlay).
pub const RECORDS_RESET: Sig = Sig {
    len: 255,
    hash: 0xb097_a9b3_ffb0_c4ee,
    calls: &[
        (0x29, Callee::Sig(&crate::o4_fam3::RECORD_COPY_0480_2880)),
        (0x48, Callee::Sig(&crate::o4_fam3::RECORD_COPY_0480_2880)),
        (0xc8, Callee::Sig(&crate::o4_fam3::RECORD_COPY_0480_2910)),
        (0xd9, Callee::Sig(&crate::o4_fam3::RECORD_COPY_0480_2910)),
        (0xea, Callee::Sig(&crate::o4_fam3::RECORD_COPY_0480_2910)),
        // __security_check_cookie
        (0xf7, Callee::Any),
    ],
    imports: &[],
};
