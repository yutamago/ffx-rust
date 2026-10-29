//! The effect starts of the `magic_0446` family (`0446`, `0565`, `0644`
//! and the other DLLs sharing their code), ported by the shared-4 list.
//! Each sets up effect objects inside the DLL's state block `G` (the word
//! an operand of each points at it) with
//! [`start_object_at`], then fills their fields from the block. The
//! object offsets, part numbers and constants are immediates of the code
//! (the same in every copy); the block, the scales and the callbacks are
//! operands. Each doc names the DLL and address it was ported from, its
//! size and how many DLLs carry it.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::s4_misc::{START_OBJECT_AT, start_object_at};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// [`start_object_at`] through the `rel32` at `f + call`: the object
/// `obj`, the scale the operand `k` points at, the part `index` and the
/// owners `(owner, owner_b)`.
#[allow(clippy::too_many_arguments)] // the original's registers and stack words
fn start<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    call: u32,
    obj: u32,
    k: u32,
    index: u32,
    (owner, owner_b): (u32, u32),
) -> u32 {
    let scale = f32::from_bits(g.read::<u32>(img.word(f + k)));
    start_object_at(
        g,
        img,
        img.rel32_target(f + call),
        obj,
        scale,
        index,
        owner,
        owner_b,
    )
}

/// The block's place `+0x2f80..+0x2f8c` copied into the object's
/// `+0xd0..+0xdc`, and the block's clock `+0xebf0` into its `+0xc4`.
/// Returns the clock.
fn place<G: Game>(g: &mut G, block: u32, obj: u32) -> u32 {
    for i in 0..4 {
        let x = g.read::<u32>(block + 0x2f80 + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, x);
    }
    let clock = g.read::<u32>(block + 0xebf0);
    g.write::<u32>(obj + 0xc4, clock);
    clock
}

/// One kind's object (cdecl `(task)`): the kind `task +0x2c` goes into
/// the block (`+kind_at`); the object is `G + 0x130 part`, `part` the
/// kind's entry in the DLL's table (operand `+0x24`), started with the
/// scale at `+0xa`; its `+0xc4` takes the block's clock, its `+0x104` the
/// callback (operand `+0x52`). Returns the object and the clock.
fn start_kind<G: Game>(g: &mut G, img: &Image, f: u32, task: u32, kind_at: u32) -> (u32, u32) {
    let w = |o: u32| img.word(f + o);
    let block = g.read::<u32>(w(0x15));
    let kind = g.read::<u32>(task + 0x2c);
    g.write::<u32>(block + kind_at, kind);
    let part = g.read::<u32>(w(0x24).wrapping_add(kind.wrapping_mul(4)));
    let obj = block.wrapping_add(part.wrapping_mul(0x130));
    start(g, img, f, 0x39, obj, 0x0a, part, (0, 0));
    let clock = g.read::<u32>(block + 0xebf0);
    g.write::<u32>(obj + 0xc4, clock);
    g.write::<u32>(obj + 0x104, w(0x52));
    (obj, clock)
}

/// [`start_kind`] with the kind at `+0x8524`; the object is kept at
/// `+0x8520`. Returns the block's clock.
///
/// `magic_0446` 0x10002980 (96 bytes; 15 DLLs).
pub fn start_kind_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let task = fr.arg(g, 0);
    let (obj, clock) = start_kind(g, img, fr.at, task, 0x8524);
    let block = g.read::<u32>(img.word(fr.at + 0x15));
    g.write::<u32>(block + 0x8520, obj);
    Ret::eax(clock)
}

/// [`start_kind_a`].
pub const START_KIND_A: Sig = Sig {
    len: 96,
    hash: 0xb37b_232f_4682_f74b,
    calls: &[(0x39, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};

/// [`start_kind`] with the kind at `+0x5c04`; kind 2 adds the object
/// `G + 0xd10` (part 0xb, owner 0x14, the scale at `+0x63`, the callback
/// at `+0x84`, mode byte `+0x126` 2). The last object is kept at
/// `+0x5bfc`. Returns the second object, or the block's clock.
///
/// `magic_0446` 0x10001530 (153 bytes; 10 DLLs).
pub fn start_kind_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let task = fr.arg(g, 0);
    let (first, clock) = start_kind(g, img, fr.at, task, 0x5c04);
    let block = g.read::<u32>(w(0x15));
    if g.read::<u32>(block + 0x5c04) != 2 {
        g.write::<u32>(block + 0x5bfc, first);
        return Ret::eax(clock);
    }
    let obj = block + 0xd10;
    let eax = start(g, img, fr.at, 0x77, obj, 0x63, 0xb, (0x14, 0));
    g.write::<u32>(obj + 0x104, w(0x84));
    g.write::<u8>(obj + 0x126, 2);
    g.write::<u32>(block + 0x5bfc, obj);
    Ret::eax(eax)
}

/// [`start_kind_b`].
pub const START_KIND_B: Sig = Sig {
    len: 153,
    hash: 0x8e5e_3a8f_1aa6_2806,
    calls: &[
        (0x39, Callee::Sig(&START_OBJECT_AT)),
        (0x77, Callee::Sig(&START_OBJECT_AT)),
    ],
    imports: &[],
};

/// The object `G + obj` (part `part`, the scale at operand `+0x4`, the
/// block at `block_op`) started and placed ([`place`]); `then` finishes
/// it. Returns the block's clock.
fn start_placed<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    (block_op, obj, part): (u32, u32, u32),
    then: impl FnOnce(&mut G, u32, u32),
) -> Ret {
    let block = g.read::<u32>(img.word(fr.at + block_op));
    let o = block + obj;
    start(g, img, fr.at, 0x20, o, 0x04, part, (0, 0));
    let clock = place(g, block, o);
    then(g, block, o);
    Ret::eax(clock)
}

/// The object `G + 0x1430` (part 0x11) started and placed.
///
/// `magic_0446` 0x10002910 (102 bytes; 10 DLLs).
pub fn start_1430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_placed(g, img, fr, (0xc, 0x1430, 0x11), |_, _, _| {})
}

/// [`start_1430`].
pub const START_1430: Sig = Sig {
    len: 102,
    hash: 0x60fa_3f73_4a93_53ab,
    calls: &[(0x20, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};

/// The object `G + 0x1300` (part 0x10) started and placed; the block's
/// `+0xd748` cleared.
///
/// `magic_0446` 0x10003050 (112 bytes; 10 DLLs).
pub fn start_1300<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_placed(g, img, fr, (0xc, 0x1300, 0x10), |g, block, _| {
        g.write::<u32>(block + 0xd748, 0);
    })
}

/// [`start_1300`].
pub const START_1300: Sig = Sig {
    len: 112,
    hash: 0x25cd_0da7_bf51_5422,
    calls: &[(0x20, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};

/// The object `G + 0x1430` (part 0x11) started and placed, its `+0xe8`
/// -17.0.
///
/// `magic_0446` 0x10004180 (112 bytes; 10 DLLs).
pub fn start_1430_low<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start_placed(g, img, fr, (0xb, 0x1430, 0x11), |g, _, o| {
        g.write::<u32>(o + 0xe8, 0xc188_0000);
    })
}

/// [`start_1430_low`].
pub const START_1430_LOW: Sig = Sig {
    len: 112,
    hash: 0xb482_c5a4_6fb1_87f8,
    calls: &[(0x20, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};

/// The object `G + 0x720` (part 6) started and placed, its `+0xe8` -5.0;
/// then the object `G + 0x2860` (part 0x22, the scale at `+0x2e`), its
/// `+0xc4` (`G + 0x2924`) the clock and `+0x104` (`G + 0x2964`) the
/// callback (operand `+0x9c`).
///
/// `magic_0446` 0x100040d0 (163 bytes; 10 DLLs).
pub fn start_720<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(img.word(f + 0xc));
    let o = block + 0x720;
    start(g, img, f, 0x20, o, 0x04, 6, (0, 0));
    place(g, block, o);
    g.write::<u32>(o + 0xe8, 0xc0a0_0000);
    start(g, img, f, 0x83, block + 0x2860, 0x2e, 0x22, (0, 0));
    let clock = g.read::<u32>(block + 0xebf0);
    g.write::<u32>(block + 0x2924, clock);
    g.write::<u32>(block + 0x2964, img.word(f + 0x9c));
    Ret::eax(clock)
}

/// [`start_720`].
pub const START_720: Sig = Sig {
    len: 163,
    hash: 0x2028_54aa_302c_87d6,
    calls: &[
        (0x20, Callee::Sig(&START_OBJECT_AT)),
        (0x83, Callee::Sig(&START_OBJECT_AT)),
    ],
    imports: &[],
};

/// The object `G + 0x260` (part 2) started; slot 219 `(*G_X, 0)` (operand
/// `+0x2c`) gives an element whose position slot 237 writes to `G +
/// 0x330` (`w` 1.0); `+0x320` 0.0314, `+0x324` the clock, `+0x344` 4.0.
/// Returns the clock.
///
/// `magic_0446` 0x100041f0 (127 bytes; 15 DLLs).
pub fn start_260<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let block = g.read::<u32>(w(0xd));
    start(g, img, f, 0x1f, block + 0x260, 0x04, 2, (0, 0));
    // start_object_at's words stay below
    let mut s = Pushes::default();
    s.push_all(&[0, fr.ecx, 0, 2]);
    let x = g.read::<u32>(w(0x2c));
    s.push_all(&[0, x]);
    let elem = s.call(g, slot_target(g, w(0x24), 219), 0).eax;
    let pos = block + 0x330;
    s.push_all(&[pos, 0, 0, elem]);
    s.call(g, slot_target(g, w(0x45), 237), 0);
    g.write::<u32>(pos + 0xc, 0x3f80_0000);
    g.write::<u32>(block + 0x320, 0x3d00_adfd);
    let clock = g.read::<u32>(block + 0xebf0);
    g.write::<u32>(block + 0x324, clock);
    g.write::<u32>(block + 0x344, 0x4080_0000);
    Ret::eax(clock)
}

/// [`start_260`].
pub const START_260: Sig = Sig {
    len: 127,
    hash: 0xd337_1612_b58a_a93d,
    calls: &[(0x1f, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};

/// Where a child start of [`start_child`] finds its words: the scale
/// (`Fixed`: the operand points at it; `Table`: at the operand plus 4
/// `kind`), the part table, the callback `owner_b`, the `rel32` of
/// [`start_object_at`] and the operands of `G_A`/`G_B`.
struct ChildOps {
    scale: Scale,
    parts: u32,
    callback: u32,
    call: u32,
    globals: (u32, u32),
}

#[derive(Clone, Copy)]
enum Scale {
    Fixed(u32),
    Table(u32),
}

/// A child object of a container (internal, `ecx` = the container,
/// `edx` = two words): the object `this + 0x130` started with the
/// kind's part and scale, its owner the callback; the container's place
/// `+0x270..+0x27c` copied into its `+0xd0..+0xdc`, the two words into
/// `+0xc0`/`+0xc4`, and its `+0x120` the word `*G_A` when the
/// container's `+0x514` is -1, else the byte `G_B[+0x514]`. Returns that
/// value.
fn start_child<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (this, pair): (u32, u32),
    kind: u32,
    o: &ChildOps,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let entry = |op: u32| w(op).wrapping_add(kind.wrapping_mul(4));
    let scale = match o.scale {
        Scale::Fixed(op) => w(op),
        Scale::Table(op) => entry(op),
    };
    let scale = f32::from_bits(g.read::<u32>(scale));
    let part = g.read::<u32>(entry(o.parts));
    let obj = this + 0x130;
    start_object_at(
        g,
        img,
        img.rel32_target(f + o.call),
        obj,
        scale,
        part,
        0,
        w(o.callback),
    );
    for i in 0..4 {
        let x = g.read::<u32>(this + 0x270 + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, x);
    }
    let (a, b) = (g.read::<u32>(pair), g.read::<u32>(pair + 4));
    g.write::<u32>(obj + 0xc0, a);
    g.write::<u32>(obj + 0xc4, b);
    let i = g.read::<u32>(this + 0x514);
    let v = if i == u32::MAX {
        g.read::<u32>(w(o.globals.0))
    } else {
        u32::from(g.read::<u8>(g.read::<u32>(w(o.globals.1)).wrapping_add(i)))
    };
    g.write::<u32>(obj + 0x120, v);
    v
}

/// [`start_child`] with a fixed scale (operand `+0xa`) and the kind on
/// the stack (cdecl, `ecx`/`edx` and one word).
///
/// `magic_0446` 0x100021d0 (164 bytes; 10 DLLs).
pub fn start_child_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = ChildOps {
        scale: Scale::Fixed(0xa),
        parts: 0x1e,
        callback: 0x12,
        call: 0x2d,
        globals: (0x81, 0x91),
    };
    let kind = fr.arg(g, 0);
    Ret::eax(start_child(g, img, fr.at, (fr.ecx, fr.edx), kind, &o))
}

/// [`start_child_a`].
pub const START_CHILD_A: Sig = Sig {
    len: 164,
    hash: 0xddd3_ffbd_d549_fa1c,
    calls: &[(0x2d, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};

/// [`start_child`] with the scale by kind (operand `+0xc`) and the kind
/// on the stack.
///
/// `magic_0446` 0x100029e0 (165 bytes; 10 DLLs).
pub fn start_child_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = ChildOps {
        scale: Scale::Table(0xc),
        parts: 0x1f,
        callback: 0x13,
        call: 0x2e,
        globals: (0x82, 0x92),
    };
    let kind = fr.arg(g, 0);
    Ret::eax(start_child(g, img, fr.at, (fr.ecx, fr.edx), kind, &o))
}

/// [`start_child_b`].
pub const START_CHILD_B: Sig = Sig {
    len: 165,
    hash: 0x224b_54de_3e62_006a,
    calls: &[(0x2e, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};

/// [`start_child`] with the kind slot 641 gives for `*G_A` (operand
/// `+0x10`), the scale by kind (`+0x1f`).
///
/// `magic_0446` 0x100030c0 (179 bytes, `ecx`/`edx`; 10 DLLs).
pub fn start_child_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let o = ChildOps {
        scale: Scale::Table(0x1f),
        parts: 0x37,
        callback: 0x2d,
        call: 0x3e,
        globals: (0x92, 0xa1),
    };
    let t = slot_target(g, w(0x1), 641);
    let kind = g.call(t, 0, &[g.read::<u32>(w(0x10))]).eax;
    Ret::eax(start_child(g, img, fr.at, (fr.ecx, fr.edx), kind, &o))
}

/// [`start_child_c`].
pub const START_CHILD_C: Sig = Sig {
    len: 179,
    hash: 0x79aa_bbb1_fe0e_6001,
    calls: &[(0x3e, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};
