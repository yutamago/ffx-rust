//! `magic_0253.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10001340, cdecl `(task)`): the script
/// ([`run`](crate::sa_script::run)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both answer 1
/// the effect ends: unless the halfword `+0x1c` is set slot 213 `(*X,
/// 1)` (`X` `+0x2b`), slot 238 `(*Y, 0)` (`Y` `+0x43`) with `+0x49` = 1,
/// the node `*Q` (`+0x5b`), when set, given back to its pool
/// ([`give_back`](crate::sa_pool::give_back)) and `*Q` cleared; the task
/// gets state 3 and its record the callback `+0x85`. Returns the record,
/// else the objects' answer.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let step = crate::sa_script::run(g, img, img.rel32_target(f + 0x05));
    // push ebp; push esi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x0c),
        fr.esp.wrapping_sub(0xc),
    );
    if r != 1 || step != 1 {
        return Ret::eax(r);
    }
    if g.read::<u16>(w(0x1c)) == 0 {
        let s = slot_target(g, w(0x25), 0x354 / 4);
        let x = g.read::<u32>(w(0x2b));
        g.call(s, 0, &[x, 1]);
    }
    let s = slot_target(g, w(0x3b), 0x3b8 / 4);
    let y = g.read::<u32>(w(0x43));
    g.write::<u32>(w(0x49), 1);
    g.call(s, 0, &[y, 0]);
    let node = g.read::<u32>(w(0x5b));
    if node != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x67), node);
        g.write::<u32>(w(0x6d), 0);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x85));
    Ret::eax(rec)
}

/// [`frame`] (140 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 140,
    hash: 0xeda2_c71b_00ef_4b6f,
    calls: &[
        (0x05, Callee::Sig(&crate::sa_script::RUN)),
        (0x0c, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x67, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// Two part slots of the current record (0x10001930, no arguments, no
/// frame): `e` = record `*I` (`I` `+0x01`) of the table `+0x0c` (0x148
/// bytes each); its two part blocks `+0x90` and `+0xd0` get their
/// handlers (`+0x16`, `+0x52`), 0, 0.025, 0x50, 2 and 3. Returns `e`.
pub fn part_slots<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let e = g
        .read::<u32>(w(0x01))
        .wrapping_mul(0x148)
        .wrapping_add(w(0x0c));
    for (block, handler) in [(0x90u32, 0x16u32), (0xd0, 0x52)] {
        g.write::<u32>(e + block, w(handler));
        for (off, v) in [
            (0xc, 0),
            (0x10, 0x3ccc_cccd),
            (0x14, 0x50),
            (0x18, 2),
            (0x1c, 3),
        ] {
            g.write::<u32>(e + block + off, v);
        }
    }
    Ret::eax(e)
}

/// [`part_slots`] (137 bytes, 1 overlay).
pub const PART_SLOTS: Sig = Sig {
    len: 137,
    hash: 0x5cb3_f2a0_d276_1baa,
    calls: &[],
    imports: &[],
};

/// The static record's node and its record (0x100018e0, a table entry,
/// no arguments): [`static_record_node`](crate::o4_small::static_record_node),
/// then the six words at operands `+0x07`..`+0x39` `[0, 2.0, 0, 0, 0,
/// 0]`. Returns the node call's `eax`.
pub fn record_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let callee = Frame {
        at: img.rel32_target(f + 0x1),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    let r = crate::o4_small::static_record_node(g, img, &callee);
    for (o, v) in [
        (0x7u32, 0u32),
        (0x11, 0x4000_0000),
        (0x1b, 0),
        (0x25, 0),
        (0x2f, 0),
        (0x39, 0),
    ] {
        g.write::<u32>(w(o), v);
    }
    Ret::eax(r.eax)
}

/// [`record_start`] (66 bytes, 1 overlay).
pub const RECORD_START: Sig = Sig {
    len: 66,
    hash: 0x4d2d_9850_3999_26fc,
    calls: &[(
        0x1,
        Callee::Sig(&crate::o4_small::STATIC_RECORD_NODE_0253_48D0),
    )],
    imports: &[],
};
