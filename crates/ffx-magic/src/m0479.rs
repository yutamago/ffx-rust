//! `magic_0479.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start (0x10003c60, internal, no arguments): the two
/// buffer globals (operands `+0x16`/`+0x1a`, `+0x20`/`+0x24`) set, slot
/// 719 `(NAME, 8, 0)` gives the data (`+0x2e`), the part's set-up
/// ([`part_init`](crate::l5_objhost::part_init)), the object's callback
/// (`+0x3c`/`+0x40`), `+0x46` 0, the halfword `+0x51` 0, the byte `+0x59`
/// 1; then a free-list node ([`pop_node`](crate::m0032::pop_node), `+0x4`
/// 0) at the head of the list (`G` at `+0x65` 0: empty). Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let s719 = slot_target(g, w(0x1), 719);
    g.write::<u32>(w(0x16), w(0x1a));
    g.write::<u32>(w(0x20), w(0x24));
    let d = g.call(s719, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x2e), d);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x33));
    g.write::<u32>(w(0x3c), w(0x40));
    g.write::<u32>(w(0x46), 0);
    g.write::<u16>(w(0x51), 0);
    g.write::<u8>(w(0x59), 1);
    let node = crate::m0032::pop_node(g, img, img.rel32_target(f + 0x5f));
    let empty = g.read::<u32>(w(0x65)) == 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    push_head(g, node, empty, (w(0x75), w(0x82)), (w(0x90), w(0x98)));
    0
}

/// [`start`] (159 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 159,
    hash: 0x85b9_ad2e_c05a_3b8c,
    calls: &[
        (0x33, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x5f, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// Starts the object of a record (0x10003ba0, cdecl `(rec)`): the
/// object `O + 0x130 * rec[+0x2c]` (operand `+0x1a`) started with the
/// scale `K` (`+0x0a`)
/// ([`start_object_first`](crate::o6_fam_object::start_object_first),
/// the `rel32` at `+0x24`;
/// its four stack words are never written and it reads none), then its
/// `+0xd0`, `+0xd4`, `+0xd8` and `+0xc4` from immediates. Returns the
/// object.
pub fn record_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.arg(g, 0);
    let index = g.read::<u32>(rec.wrapping_add(0x2c));
    let obj = index.wrapping_mul(0x130).wrapping_add(w(0x1a));
    let scale = g.read::<u32>(w(0x0a));
    // push ebp; push esi; sub esp, 0x10; the return address
    let callee = Frame {
        at: img.rel32_target(f + 0x24),
        esp: fr.esp.wrapping_sub(0x1c),
        edx: obj,
        xmm: [0, 0, 0, u128::from(scale)],
        ..Frame::default()
    };
    let eax = crate::o6_fam_object::start_object_first(g, img, callee.at, &callee);
    for (d, op) in [(0xd0u32, 0x31u32), (0xd4, 0x3b), (0xd8, 0x45), (0xc4, 0x4f)] {
        g.write::<u32>(obj.wrapping_add(d), w(op));
    }
    Ret::eax(eax)
}

/// [`record_object`] (86 bytes, 1 overlay).
pub const RECORD_OBJECT: Sig = Sig {
    len: 86,
    hash: 0x6c1a_cc05_b16e_d629,
    calls: &[(
        0x24,
        Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0369_2C10),
    )],
    imports: &[],
};
