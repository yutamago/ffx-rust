//! `magic_0367.dll`'s own functions, and `magic_0369`'s kin of its
//! start-up (besides the runtime, the object host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The start-ups' common head: `stores` words get their immediates
/// (`mov dword [addr], imm32`), the actors are looked up
/// ([`lookup_actors`](crate::sb_misc::lookup_actors), `rel32` at
/// `lookup`), slot 719 `(F, 8, 0)` (`F` at `name`) into `handle`, the
/// part set up ([`part_init`](crate::l5_objhost::part_init), `rel32` at
/// `part`).
fn head<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    stores: &[u32],
    (lookup, table, name, handle, part): (u32, u32, u32, u32, u32),
) {
    let w = |off: u32| img.word(f + off);
    for &at in stores {
        g.write::<u32>(w(at), w(at + 4));
    }
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + lookup));
    let s = slot_target(g, w(table), 0xb3c / 4);
    let h = g.call(s, 0, &[w(name), 8, 0]).eax;
    g.write::<u32>(w(handle), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + part));
}

/// Puts `node` last on a list (operands: the first word tested, the last
/// read, the first and the last written).
fn append<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    (first, last_r, first_w, last_w): (u32, u32, u32, u32),
) {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(node + 4, 0);
    if g.read::<u32>(w(first)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(first_w), node);
    } else {
        let last = g.read::<u32>(w(last_r));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(last_w), node);
}

/// The start-up (`magic_0367` 0x100019b0, no arguments, no frame): the
/// [`head`] (five words), `+0x64` its immediate, `+0x6e` 0, a u16 0 and
/// a byte 1; two pool nodes ([`take`](crate::sa_pool::take), owners
/// `+0x5e`, `+0xca`; bytes `+0x18` 1 and 2) go last on the task list
/// (first `+0x8d`, last `+0xa1`), the records block `+0xb8` set up
/// between them ([`records_init_100`](crate::sa_misc::records_init_100)).
/// Returns 0.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    head(
        g,
        img,
        f,
        &[0x02, 0x0c, 0x16, 0x20, 0x2a],
        (0x33, 0x38, 0x47, 0x51, 0x56),
    );
    g.write::<u32>(w(0x64), w(0x68));
    g.write::<u32>(w(0x6e), 0);
    g.write::<u16>(w(0x79), 0);
    g.write::<u8>(w(0x81), 1);
    let take = img.rel32_target(f + 0x87);
    let first = crate::sa_pool::take(g, img, take, w(0x5e));
    let head0 = g.read::<u32>(w(0x8d));
    g.write::<u8>(first + 0x18, 1);
    g.write::<u32>(first + 4, 0);
    if head0 == 0 {
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0xb3), first);
    } else {
        let last = g.read::<u32>(w(0xa1));
        g.write::<u32>(first, last);
        g.write::<u32>(last + 4, first);
    }
    g.write::<u32>(first + 8, first);
    g.write::<u32>(w(0xc0), first);
    crate::sa_misc::records_init_100(g, img, img.rel32_target(f + 0xc5), w(0xb8));
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0xcf), w(0xca));
    g.write::<u8>(second + 0x18, 2);
    let last_w = if g.read::<u32>(w(0xd5)) == 0 {
        0x10c
    } else {
        0xf6
    };
    append(g, img, f, second, (0xd5, 0xe9, 0x104, last_w));
    Ret::eax(0)
}

/// [`start_up`] (275 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 275,
    hash: 0x2345_2d98_a1cb_1fdb,
    calls: &[
        (0x33, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x56, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x87, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xc5, Callee::Sig(&crate::sa_misc::RECORDS_INIT_100)),
        (0xcf, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The start-up (`magic_0369` 0x10001450, no arguments, no frame): the
/// [`head`] (four words), `+0x55` its immediate, `+0x5f` 0, a u16 0 and
/// a byte 1; a node off the free list
/// ([`pop_node`](crate::m0032::pop_node)) goes last on the task list
/// (first `+0x7e`, last `+0x8e`). Returns 0.
///
/// `magic_0480`'s copy (0x10004630, 194 bytes; `mov ecx, imm32` at
/// `+0x7c`) sets its aim records first
/// ([`aim_records`](crate::s7_misc::aim_records), the global layout) and
/// takes its node from the pool ([`take`](crate::sa_pool::take), the
/// owner that immediate); its later operands are 10 bytes on.
pub fn start_up_0369<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    head(
        g,
        img,
        f,
        &[0x02, 0x0c, 0x16, 0x20],
        (0x29, 0x2e, 0x3d, 0x47, 0x4c),
    );
    g.write::<u32>(w(0x55), w(0x59));
    g.write::<u32>(w(0x5f), 0);
    g.write::<u16>(w(0x6a), 0);
    g.write::<u8>(w(0x72), 1);
    let aim = img.word(f + 0x7c) & 0xff == 0xb9;
    let (node, d) = if aim {
        // call aim_records; mov ecx, OWNER; call take
        let callee = Frame {
            at: img.rel32_target(f + 0x78),
            esp: fr.esp.wrapping_sub(4),
            ..*fr
        };
        crate::s7_misc::aim_records(g, img, &callee);
        let take = img.rel32_target(f + 0x82);
        (crate::sa_pool::take(g, img, take, w(0x7d)), 10)
    } else {
        (
            crate::m0032::pop_node(g, img, img.rel32_target(f + 0x78)),
            0,
        )
    };
    let last_w = if g.read::<u32>(w(0x7e + d)) == 0 {
        0xb1
    } else {
        0x9b
    };
    append(g, img, f, node, (0x7e + d, 0x8e + d, 0xa9 + d, last_w + d));
    Ret::eax(0)
}

/// [`start_up_0369`] (184 bytes, 1 overlay).
pub const START_UP_0369: Sig = Sig {
    len: 184,
    hash: 0xc9fc_07ea_603d_b6da,
    calls: &[
        (0x29, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x4c, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x78, Callee::Sig(&crate::m0032::POP_NODE)),
    ],
    imports: &[],
};

/// Starts the fourth object (0x100014a0, a table entry, no arguments):
/// the object at the immediate `+0x0c`
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale the
/// float at `+0x04`, index 4), then the 16 bytes at `+0x1a` copied to
/// `+0x24`. Returns the object.
pub fn fourth_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x4));
    let o =
        crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x13), w(0xc), scale, 4);
    for k in 0..4u32 {
        let v = g.read::<u32>(w(0x1a) + 4 * k);
        g.write::<u32>(w(0x24) + 4 * k, v);
    }
    Ret::eax(o)
}

/// [`fourth_start`] (41 bytes, 1 overlay).
pub const FOURTH_START: Sig = Sig {
    len: 41,
    hash: 0x6905_fd44_96bb_19ce,
    calls: &[(0x13, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN))],
    imports: &[],
};

/// Starts the second objects of two records (0x10001410, a table entry,
/// no arguments): the objects at the immediates `+0x0c` and `+0x23`
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale the
/// float at `+0x04`/`+0x1b`, index 2), each followed by its callback word
/// (`+0x2b`/`+0x5b` = the immediate `+0x2f`/`+0x5f`) and its point
/// (`+0x35`.. / `+0x65`..: `(3000, 100, 650)`, the second's `z` -650).
/// Returns the second object.
pub fn pair_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x4));
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x13), w(0xc), scale, 2);
    let scale = g.read::<f32>(w(0x1b));
    g.write::<u32>(w(0x2b), w(0x2f));
    for (o, v) in [
        (0x35u32, 0x453b_8000u32),
        (0x3f, 0x42c8_0000),
        (0x49, 0x4422_8000),
    ] {
        g.write::<u32>(w(o), v);
    }
    let o =
        crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x52), w(0x23), scale, 2);
    g.write::<u32>(w(0x5b), w(0x5f));
    for (o, v) in [
        (0x65u32, 0x453b_8000u32),
        (0x6f, 0x42c8_0000),
        (0x79, 0xc422_8000),
    ] {
        g.write::<u32>(w(o), v);
    }
    Ret::eax(o)
}

/// [`pair_start`] (130 bytes, 1 overlay).
pub const PAIR_START: Sig = Sig {
    len: 130,
    hash: 0xa7bd_07f0_5f3d_edef,
    calls: &[
        (0x13, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x52, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
    ],
    imports: &[],
};
