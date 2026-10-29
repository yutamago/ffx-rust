//! The effect starts of the `magic_0446` family (`0446`, `0565`, `0644`
//! and the other DLLs sharing their code), ported by the shared-4 list.
//! Each sets up effect objects inside the DLL's state block `G` (the word
//! an operand of each points at it) with
//! [`start_object_at`], then fills their fields from the block. The
//! object offsets, part numbers and constants are immediates of the code
//! (the same in every copy); the block, the scales and the callbacks are
//! operands. Each doc names the DLL and address it was ported from, its
//! size and how many DLLs carry it.

use crate::l5_objhost::{LIST_REMOVE, list_remove};
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::s3_misc::append_task;
use crate::s4_misc::{START_OBJECT_AT, start_object_at};
use crate::sa_pool::{TAKE, take};
use crate::sb_sse::{add, mul};
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
    // call width: 12 (Pushes caps the words at 12)
    let elem = s.call(g, slot_target(g, w(0x24), 219), 0).eax;
    let pos = block + 0x330;
    s.push_all(&[pos, 0, 0, elem]);
    // call width: 12 (Pushes caps the words at 12)
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

/// A task node for `owner` ([`take`] through the `rel32` at `f + call`)
/// appended to the DLL's task list (`head`/`tail` operands; with `kind`,
/// its byte `+0x18` set first). Returns the node.
fn new_task<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    call: u32,
    owner: u32,
    kind: Option<u8>,
    (head, tail): (u32, u32),
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let node = take(g, img, img.rel32_target(f + call), owner);
    let empty = g.read::<u32>(w(head)) == 0;
    if let Some(k) = kind {
        g.write::<u8>(node + 0x18, k);
    }
    g.write::<u32>(node + 4, 0);
    append_task(g, node, w(head), w(tail), empty);
    node
}

/// The family's start-up (a table entry, no arguments): the DLL's lists
/// cleared and its blocks set (immediates of the code: `G` among them);
/// slot 719 `(part, 8, 0)` gives the part's handle (`+0x4c`);
/// [`part_setup`] with the part block; the light ranges
/// ([`light_ranges`]) and the camera target ([`camera_target`]); slot
/// 271 (4); then two tasks (kinds 1 and 2, the callbacks at operands
/// `+0x77` and `+0xbf`), the records ([`records_init`], the block at
/// `+0xad`) set up between them. Returns 0.
///
/// `magic_0446` 0x100049c0 (264 bytes; 10 DLLs).
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(w(0x16), 0);
    for (dst, src) in [(0x20, 0x24), (0x2a, 0x2e), (0x34, 0x38), (0x3e, 0x42)] {
        g.write::<u32>(w(dst), w(src));
    }
    let t = slot_target(g, w(0x1), 719);
    let handle = g.call(t, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x4c), handle);
    // add esp, 8 leaves slot 719's last word (0) under the block pushed
    let esp = fr.esp.wrapping_sub(12);
    g.write::<u32>(esp + 8, 0);
    g.write::<u32>(esp + 4, w(0x51));
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x56), esp);
    crate::sb_misc::light_ranges(g, img, img.rel32_target(f + 0x5b));
    crate::sa_misc::camera_target(g, img, img.rel32_target(f + 0x60));
    let t = slot_target(g, w(0x65), 271);
    g.call(t, 0, &[4, w(0x51), 0]);
    new_task(g, img, f, 0x7c, w(0x77), Some(1), (0x82, 0x96));
    crate::sa_misc::records_init(g, img, img.rel32_target(f + 0xba), w(0xad));
    new_task(g, img, f, 0xc4, w(0xbf), Some(2), (0xca, 0xde));
    Ret::eax(0)
}

/// [`start_up`].
pub const START_UP: Sig = Sig {
    len: 264,
    hash: 0x8d33_9182_067f_87a0,
    calls: &[
        (0x56, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x5b, Callee::Sig(&crate::sb_misc::LIGHT_RANGES)),
        (0x60, Callee::Sig(&crate::sa_misc::CAMERA_TARGET)),
        (0x7c, Callee::Sig(&TAKE)),
        (0xba, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0xc4, Callee::Sig(&TAKE)),
    ],
    imports: &[],
};

/// The mode's set-up (a table entry, no arguments): mode 0 (`G +
/// 0x5c04`) sets the sprite block `G + 0x59b0` up
/// ([`crate::sb_misc::sprite_init`] with the owner `*(*R + 0x24)`,
/// operand `+0x12`) with frames 0x10, scales 1.0 and 0.5, byte 0x40;
/// any mode but 1 then starts a task (the callback at `+0x55`, `G +
/// 0x5c00` = 0x71). Returns the task, the set-up's `eax`, or `eax` as it
/// came for mode 1.
///
/// `magic_0446` 0x10002010 (162 bytes; 10 DLLs).
pub fn mode_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let block = g.read::<u32>(w(0x3));
    let mut eax = fr.eax;
    if g.read::<u32>(block + 0x5c04) == 0 {
        let res = g.read::<u32>(g.read::<u32>(w(0x12)).wrapping_add(0x24));
        eax = crate::sb_misc::sprite_init(g, block + 0x59b0, res);
        g.write::<u32>(block + 0x5bd8, 0x10);
        g.write::<u32>(block + 0x5bcc, 0x3f80_0000);
        g.write::<u32>(block + 0x5bc8, 0x3f00_0000);
        g.write::<u8>(block + 0x5bbf, 0x40);
    }
    if g.read::<u32>(block + 0x5c04) == 1 {
        return Ret::eax(eax);
    }
    g.write::<u32>(block + 0x5c00, 0x71);
    Ret::eax(new_task(g, img, f, 0x64, w(0x55), None, (0x6a, 0x7a)))
}

/// [`mode_setup`].
pub const MODE_SETUP: Sig = Sig {
    len: 162,
    hash: 0xaa3c_fa5c_475c_d77e,
    calls: &[
        (0x21, Callee::Sig(&crate::sb_misc::SPRITE_INIT)),
        (0x64, Callee::Sig(&TAKE)),
    ],
    imports: &[],
};

/// The family's load step (a table entry, no arguments): while slot 225
/// `(1)` answers non-zero nothing happens; the resource `*G_RES` (operand
/// `+0x26`) is requested once (slot 226 `(res, 0)`, `G_ASKED` 1, operand
/// `+0x37`); once slot 227 says it is there (1 or more) the effect starts
/// ([`start_up`]) and two table entries (operands `+0x61`, `+0x6b`) get
/// their next functions (`+0x65`, `+0x6f`). Returns 0.
///
/// `magic_0446` 0x10001240 (118 bytes; 10 DLLs).
pub fn load_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = slot_target(g, w(0x1), 225);
    if g.call(t, 0, &[1]).eax != 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x18)) == 0 {
        let t = slot_target(g, w(0x20), 226);
        let res = g.read::<u32>(w(0x26));
        g.call(t, 0, &[res, 0]);
        g.write::<u32>(w(0x37), 1);
    }
    let t = slot_target(g, w(0x40), 227);
    let res = g.read::<u32>(w(0x46));
    if g.call(t, 0, &[res]).eax.cast_signed() < 1 {
        return Ret::eax(0);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x5b),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    start_up(g, img, &callee);
    g.write::<u32>(w(0x61), w(0x65));
    g.write::<u32>(w(0x6b), w(0x6f));
    Ret::eax(0)
}

/// [`load_step`].
pub const LOAD_STEP: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP))],
    imports: &[],
};

/// The mode's object (a table entry, no arguments): with a mode (`G +
/// 0x5c04`) the previous object (`G + 0x5bfc`) leaves the list (operand
/// `+0x1b`, [`list_remove`]); when it was there, `*G_P` = `ITEM`
/// (operands `+0x29`, `+0x30`), slot 713 frees its part and its flag
/// bit 15 (`u16 +0x124`) is cleared. Then the mode's object `G + 0x130
/// part` (part from the table at `+0x61`, the scale at `+0x5a`) starts,
/// slot 219 `(*G_A, 0)` gives an element whose position slot 237 writes
/// into its `+0xd0` (`w` 1.0); its `+0xc4` the clock, `+0xe4` 3.0,
/// `+0xe8` -3.0. Returns the clock.
///
/// `magic_0446` 0x100020c0 (216 bytes; 10 DLLs).
pub fn mode_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let block = g.read::<u32>(w(0x3));
    if g.read::<u32>(block + 0x5c04) != 0 {
        let old = g.read::<u32>(block + 0x5bfc);
        if list_remove(g, img, img.rel32_target(f + 0x20), w(0x1b), old) != 0 {
            let p = g.read::<u32>(w(0x29));
            g.write::<u32>(p, w(0x30));
            let t = slot_target(g, w(0x35), 713);
            g.call(t, 0, &[old]);
            let flags = g.read::<u16>(old + 0x124) & 0x7fff;
            g.write::<u16>(old + 0x124, flags);
        }
    }
    let mode = g.read::<u32>(block + 0x5c04);
    let part = g.read::<u32>(w(0x61).wrapping_add(mode.wrapping_mul(4)));
    let obj = block.wrapping_add(part.wrapping_mul(0x130));
    start(g, img, f, 0x78, obj, 0x5a, part, (0, 0));
    // start_object_at's words stay below (the ones above `ecx`, which is
    // what list_remove left there)
    let mut s = Pushes::default();
    s.push_all(&[0, part]);
    let x = g.read::<u32>(w(0x85));
    s.push_all(&[0, x]);
    // call width: 12 (Pushes caps the words at 12)
    let elem = s.call(g, slot_target(g, w(0x7d), 219), 0).eax;
    let pos = obj + 0xd0;
    s.push_all(&[pos, 0, 0, elem]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x9e), 237), 0);
    g.write::<u32>(pos + 0xc, 0x3f80_0000);
    let clock = g.read::<u32>(block + 0xebf0);
    g.write::<u32>(obj + 0xc4, clock);
    g.write::<u32>(obj + 0xe4, 0x4040_0000);
    g.write::<u32>(obj + 0xe8, 0xc040_0000);
    Ret::eax(clock)
}

/// [`mode_object`].
pub const MODE_OBJECT: Sig = Sig {
    len: 216,
    hash: 0xde99_8eb0_0830_052d,
    calls: &[
        (0x20, Callee::Sig(&LIST_REMOVE)),
        (0x78, Callee::Sig(&START_OBJECT_AT)),
    ],
    imports: &[],
};

/// [`start_child`], then, unless the container's `+0x518` is set, the
/// child's sound: slot 719 `(SOUND, 6, table[kind])` finds it and slot
/// 393 plays it at the child's place (`+0xd0`, 0x100); then slot 719
/// `(FX, 9, 1)` and slot 306 `(7, 1, it)`. `k` shifts the operands by one
/// byte (the scale by kind takes one more). Returns slot 306's `eax`.
fn start_child_sound<G: Game>(g: &mut G, img: &Image, fr: &Frame, o: &ChildOps, k: u32) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off + k);
    let (this, kind) = (fr.ecx, fr.arg(g, 0));
    start_child(g, img, f, (this, fr.edx), kind, o);
    if g.read::<u32>(this + 0x518) == 0 {
        let obj = this + 0x130;
        let id = g.read::<u32>(w(0xa5).wrapping_add(kind.wrapping_mul(4)));
        let t = slot_target(g, w(0xaa), 719);
        let h = g.call(t, 0, &[w(0xb7), 6, id]).eax;
        let t = slot_target(g, w(0xcb), 393);
        g.call(t, 0, &[h, obj + 0xd0, 0x100, w(0xb7), 6, id]);
    }
    let t = slot_target(g, w(0xdb), 719);
    let h = g.call(t, 0, &[w(0xea), 9, 1]).eax;
    let t = slot_target(g, w(0xf2), 306);
    Ret::eax(g.call(t, 0, &[7, 1, h, w(0xea), 9, 1]).eax)
}

/// [`start_child_sound`] with a fixed scale (operand `+0xa`).
///
/// `magic_0565` 0x10003d30 (266 bytes, `ecx`/`edx` and the kind; 10
/// DLLs).
pub fn start_child_sound_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = ChildOps {
        scale: Scale::Fixed(0xa),
        parts: 0x1e,
        callback: 0x12,
        call: 0x2d,
        globals: (0x81, 0x88),
    };
    start_child_sound(g, img, fr, &o, 0)
}

/// [`start_child_sound_a`].
pub const START_CHILD_SOUND_A: Sig = Sig {
    len: 266,
    hash: 0x6c88_94e0_e0c6_f45b,
    calls: &[(0x2d, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};

/// [`start_child_sound`] with the scale by kind (operand `+0xc`).
///
/// `magic_0565` 0x10004640 (267 bytes, `ecx`/`edx` and the kind; 10
/// DLLs).
pub fn start_child_sound_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let o = ChildOps {
        scale: Scale::Table(0xc),
        parts: 0x1f,
        callback: 0x13,
        call: 0x2e,
        globals: (0x82, 0x89),
    };
    start_child_sound(g, img, fr, &o, 1)
}

/// [`start_child_sound_b`].
pub const START_CHILD_SOUND_B: Sig = Sig {
    len: 267,
    hash: 0x8ef0_ff8a_8613_8e6d,
    calls: &[(0x2e, Callee::Sig(&START_OBJECT_AT))],
    imports: &[],
};

/// Resets the current record of the DLL's table (a table entry, no
/// arguments): `i = G_I` (operand `+0x1`); by the kind `G_K[i]` (`+0xf`;
/// the jump table: 1, 2, 3 or any other) a data block `D` (the
/// immediates at `+0x20`, `+0x27`, `+0x2e`, `+0x35`); the record's words
/// (each its own operand, 0x78 bytes per record) take `D`, `D + 8`, `D +
/// 0x10`, zeros and the constants 0x3000, 0x200, 0xa2, 0xf and 1. Returns
/// `D + 0x10`.
///
/// `magic_0565` 0x10001870 (312 bytes with its jump table; 10 DLLs).
pub fn record_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let i = g.read::<u32>(w(0x1));
    let kind = g.read::<u32>(w(0xf).wrapping_add(i.wrapping_mul(4)));
    let data = match kind {
        1 => w(0x20),
        2 => w(0x27),
        3 => w(0x2e),
        _ => w(0x35),
    };
    let rec = i.wrapping_mul(0x78);
    for (op, v) in [
        (0x3c, data),
        (0x43, 0),
        (0x4e, 0),
        (0x59, 0),
        (0x64, 0),
        (0x72, data.wrapping_add(8)),
        (0x79, 0),
        (0x84, 0),
        (0x8f, 0),
        (0x9a, 0),
        (0xa8, data.wrapping_add(0x10)),
        (0xaf, 0),
        (0xba, 0),
        (0xc5, 0),
        (0xd0, 0),
        (0xdb, 0x3000),
        (0xe6, 0x200),
        (0xf1, 0xa2),
        (0xfc, 0xf),
        (0x107, 0),
        (0x112, 0),
        (0x11d, 1),
    ] {
        g.write::<u32>(w(op).wrapping_add(rec), v);
    }
    Ret::eax(data.wrapping_add(0x10))
}

/// [`record_reset`].
pub const RECORD_RESET: Sig = Sig {
    len: 312,
    hash: 0xc474_3702_4518_6f8d,
    calls: &[],
    imports: &[],
};

/// The `magic_0565` family's start-up (a table entry, cdecl `(arg)`):
/// `G_ARG` = arg (operand `+0x8`) and the object block set up
/// ([`crate::l5_objhost::obj_block_init`]); slots 556, 555 and 331 on the
/// DLL's three blocks (each call's words left for the next); three scales
/// 1.0 and slot 790's float into `G_F`; the task ring
/// ([`crate::l5_objhost::tasks_init`]); the 0x3ff nodes of pool A free
/// (payload 0, the free count 0x3ff, the rover 0); slot 720 (0x3400,
/// 0x800); slot 718 and 222 `(part, 0, 0x16, 1)` on part A; slot 718,
/// 222 `(part, 0, 1000, 1)` and 221 `(part, 0, 1, 1)` on part B; slot 718
/// and 221 `(part, 0, 1, 1)` on part A; `G_DONE` 0. Returns 0.
///
/// `magic_0565` 0x10001000 (371 bytes; 10 DLLs).
pub fn host_start_565<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x8), arg);
    crate::l5_objhost::obj_block_init(g, img, img.rel32_target(f + 0xd), arg);
    let mut s = Pushes::default();
    for (t, slot, block) in [(0x12, 556, 0x17), (0x24, 555, 0x29), (0x36, 331, 0x3b)] {
        s.push(w(block));
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(t), slot), 0);
    }
    s.pop(3);
    for op in [0x57, 0x61, 0x6b] {
        g.write::<u32>(w(op), 0x3f80_0000);
    }
    let t = slot_target(g, w(0x48), 790);
    let out = g.call(t, 0, &[]);
    g.write::<u32>(w(0x77), crate::sa_summon::st0_f32(out));
    crate::l5_objhost::tasks_init(g, img, img.rel32_target(f + 0x7c));
    for k in 0..0x3ff {
        g.write::<u32>(w(0x81) + 12 * k, 0);
    }
    g.write::<u16>(w(0xa3), 0x3ff);
    g.write::<u16>(w(0xab), 0);
    s.push_all(&[0x800, 0x3400]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xb0), 720), 0);
    let (a, b) = (w(0xcc), w(0xf6));
    s.push(a);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xc7), 718), 0);
    s.push_all(&[1, 0x16, 0, w(0xea)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xd9), 222), 0);
    s.push(b);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xf1), 718), 0);
    s.push_all(&[1, 1000, 0, w(0x117)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x103), 222), 0);
    s.push_all(&[1, 1, 0, w(0x12f)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x11e), 221), 0);
    s.pop(16);
    s.push(w(0x144));
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x136), 718), 0);
    s.push_all(&[1, 1, 0, w(0x15c)]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x14b), 221), 0);
    g.write::<u32>(w(0x167), 0);
    Ret::eax(0)
}

/// [`host_start_565`].
pub const HOST_START_565: Sig = Sig {
    len: 371,
    hash: 0xfa05_7819_3e8d_d38c,
    calls: &[
        (0xd, Callee::Sig(&crate::l5_objhost::OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&crate::l5_objhost::TASKS_INIT)),
    ],
    imports: &[],
};

/// The family's screen frame (a table entry, no arguments; an 8-aligned
/// frame): once, slot 628 `(1)` (`G_INIT`, operands `+0x8`, `+0x25`);
/// slot 821 `(1)`; then three bars drawn with slots 811 (a colour: `(0,
/// 0, 0, a, mode)`), 809 (a corner: `(x, y, mode)` as floats) and 812
/// (`(0, 0, 1, kind)`): the alphas `(10 - n) * 64 / 10` and `(10 - n) *
/// 128 / 10` (`n = G_N`, operands `+0x45`, `+0xf2`; `idiv` by 10 through
/// the multiply), the corners (0, 0)-(512, 416), (0, 0)-(512, 80) and (0,
/// 336)-(512, 416). With `G_M` 1 (`+0x22f`) slots 650 and 382 `(0, 0)`
/// follow. Each call sees the words the earlier ones left. Returns the
/// last call's `eax`.
///
/// `magic_0565` 0x100013f0 (605 bytes; 10 DLLs).
pub fn screen_bars<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    if g.read::<u32>(w(0x8)) == 0 {
        let t = slot_target(g, w(0x12), 628);
        g.call(t, 0, &[1]);
        g.write::<u32>(w(0x25), 1);
    }
    let alpha = |g: &G, op: u32, shift: u32| {
        let n = 10u32.wrapping_sub(g.read::<u32>(w(op)));
        (n << shift).cast_signed() / 10
    };
    let mut s = Pushes::default();
    s.push(1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x2e), 821), 0);
    let a = alpha(g, 0x45, 6).cast_unsigned();
    s.pop(1);
    let (x512, y416, y80, y336) = (0x4400_0000, 0x43d0_0000, 0x42a0_0000, 0x43a8_0000);
    let colour = |g: &mut G, s: &mut Pushes, op: u32, a: u32, mode: u32| {
        s.push_all(&[mode, a, 0, 0, 0]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(op), 811), 0)
    };
    let corner = |g: &mut G, s: &mut Pushes, op: u32, (x, y): (u32, u32), mode: u32| {
        s.push_all(&[mode, y, x]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(op), 809), 0);
        s.pop(3);
    };
    let bar = |g: &mut G, s: &mut Pushes, op: u32, kind: u32| {
        s.push_all(&[kind, 1, 0, 0]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, slot_target(g, w(op), 812), 0)
    };
    colour(g, &mut s, 0x54, a, 0);
    s.pop(5);
    corner(g, &mut s, 0x77, (0, 0), 0);
    colour(g, &mut s, 0x9b, a, 1);
    s.pop(5);
    corner(g, &mut s, 0xb4, (x512, y416), 1);
    bar(g, &mut s, 0xd9, 0);
    let a = alpha(g, 0xf2, 7).cast_unsigned();
    colour(g, &mut s, 0x101, a, 0);
    s.pop(9);
    corner(g, &mut s, 0x121, (0, 0), 0);
    colour(g, &mut s, 0x148, 0, 1);
    s.pop(5);
    corner(g, &mut s, 0x15f, (x512, y80), 1);
    bar(g, &mut s, 0x183, 2);
    colour(g, &mut s, 0x19b, 0, 0);
    s.pop(9);
    corner(g, &mut s, 0x1b2, (0, y336), 0);
    colour(g, &mut s, 0x1d6, a, 1);
    s.pop(5);
    corner(g, &mut s, 0x1ef, (x512, y416), 1);
    let mut eax = bar(g, &mut s, 0x213, 2).eax;
    if g.read::<u32>(w(0x22f)) == 1 {
        let t = slot_target(g, w(0x237), 650);
        g.call(t, 0, &[]);
        let t = slot_target(g, w(0x244), 382);
        eax = g.call(t, 0, &[0, 0]).eax;
    }
    Ret::eax(eax)
}

/// [`screen_bars`].
pub const SCREEN_BARS: Sig = Sig {
    len: 605,
    hash: 0xa460_a890_153b_a0e9,
    calls: &[],
    imports: &[],
};

/// `f32` bits of 2.0, 0.25, 0.1, 0.2, 0.5, 1.0, 0.01 and pi.
const TWO: u32 = 0x4000_0000;
const QUARTER: u32 = 0x3e80_0000;
const TENTH: u32 = 0x3dcc_cccd;
const FIFTH: u32 = 0x3e4c_cccd;
const HALF: u32 = 0x3f00_0000;
const ONE: u32 = 0x3f80_0000;
const HUNDREDTH: u32 = 0x3c23_d70a;
const PI: u32 = 0x4049_0fdb;

/// One spark's numbers in [`spark`]: the `rel32` of the pool's take, the
/// operand offsets of the seven random calls' tables, the sign mask, `K1`
/// and the size scale; the spread of `+0x10` and `+0x18` (slot 314) and
/// of the negated `+0x14` (slot 404, `neg_add` the operand of a constant
/// added first); `+0x24`; `+0x4c`/`+0x50`; whether `z` comes first in its
/// sum; extra stores `(offset, value, bytes)` after the life.
struct Spark {
    take: u32,
    tables: [u32; 7],
    mask: u32,
    k1: u32,
    scale: u32,
    spread: (u32, u32, u32),
    neg_add: Option<u32>,
    drag: u32,
    life: (u32, u32),
    z_first: bool,
    extra: &'static [(u32, u32, u8)],
}

/// A spark from the sprite pool ([`crate::sb_misc::pool_take`]) at the
/// position `pos` (`x` and `z` jittered by slot 314 `(2.0)`, `y` as it
/// is), its resource `res`, frame -1, time 0x200, on; `+0x10..+0x1c`
/// random (slots 314 and 404), `+0x20..+0x2c` (0, `drag`, 0, 1), the
/// size `(rand 0.2 + K1) * K2` and the life words. Returns the spark (0
/// when the pool is empty) and the last slot 314's `eax`.
fn spark<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    pos: u32,
    res: impl FnOnce(&mut G) -> u32,
    sp: &Spark,
) -> (u32, u32) {
    let w = |o: u32| img.word(f + o);
    let p = crate::sb_misc::pool_take(g, img, img.rel32_target(f + sp.take));
    if p == 0 {
        return (0, 0);
    }
    let r = res(g);
    g.write::<u32>(p + 0x30, r);
    g.write::<u16>(p + 0x38, 0xffff);
    g.write::<u16>(p + 0x3a, 0x200);
    g.write::<u32>(p + 0x34, 1);
    let rand = |g: &mut G, op: u32, slot: u32, x: u32| {
        let target = slot_target(g, w(op), slot);
        let out = g.call(target, 0, &[x]);
        (f32::from_bits(crate::sa_summon::st0_f32(out)), out.eax)
    };
    let t = sp.tables;
    let (dx, _) = rand(g, t[0], 314, TWO);
    set_f32(g, p, add(f32_at(g, pos), dx));
    g.write::<u32>(p + 4, g.read::<u32>(pos + 4));
    let (dz, _) = rand(g, t[1], 314, TWO);
    g.write::<u32>(p + 0xc, ONE);
    let z = f32_at(g, pos + 8);
    set_f32(g, p + 8, if sp.z_first { add(z, dz) } else { add(dz, z) });
    let (a, _) = rand(g, t[2], 314, sp.spread.0);
    set_f32(g, p + 0x10, a);
    let (mut b, _) = rand(g, t[3], 404, sp.spread.1);
    if let Some(op) = sp.neg_add {
        b = add(b, f32_at(g, w(op)));
    }
    // xorps with the DLL's sign mask
    let mask = g.read::<u32>(w(sp.mask));
    set_f32(g, p + 0x14, f32::from_bits(b.to_bits() ^ mask));
    let (c, _) = rand(g, t[4], 314, sp.spread.2);
    set_f32(g, p + 0x18, c);
    let (d, _) = rand(g, t[5], 314, PI);
    set_f32(g, p + 0x1c, d);
    for (o, v) in [(0x20, 0), (0x24, sp.drag), (0x28, 0), (0x2c, ONE)] {
        g.write::<u32>(p + o, v);
    }
    let (e, eax) = rand(g, t[6], 314, FIFTH);
    g.write::<u32>(p + 0x4c, sp.life.0);
    g.write::<u32>(p + 0x50, sp.life.1);
    for &(o, v, bytes) in sp.extra {
        if bytes == 1 {
            g.write::<u8>(p + o, v as u8);
        } else {
            g.write::<u32>(p + o, v);
        }
    }
    let size = mul(add(e, f32_at(g, w(sp.k1))), f32_at(g, w(sp.scale)));
    set_f32(g, p + 0x3c, size);
    (p, eax)
}

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

fn set_f32<G: Game>(g: &mut G, a: u32, x: f32) {
    g.write::<u32>(a, x.to_bits());
}

/// Two sparks at a position (internal, `ecx` = the position): the first
/// with the resource slot 719 `(R, 9, 0)` finds (operand `+0x28`), the
/// second (unless `**G_F & 3`, operand `+0x150`) with the word `res_off`
/// of `*R2` (`+0x190`). Returns the last slot 314's `eax`, `*G_F`'s
/// pointer, or 0 when the pool is empty.
fn sparks<G: Game>(g: &mut G, img: &Image, fr: &Frame, res_off: u32) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let pos = fr.ecx;
    let first = Spark {
        take: 0x9,
        tables: [0x42, 0x7d, 0x97, 0xba, 0xdd, 0xf9, 0x110],
        mask: 0xd8,
        k1: 0x14b,
        scale: 0x169,
        spread: (TENTH, HALF, TENTH),
        neg_add: None,
        drag: HUNDREDTH,
        life: (0x20, 0x20),
        z_first: false,
        extra: &[(0x6b, 0, 1)],
    };
    let res = |g: &mut G| {
        let t = slot_target(g, w(0x19), 719);
        g.call(t, 0, &[w(0x28), 9, 0]).eax
    };
    let (p, _) = spark(g, img, f, pos, res, &first);
    if p == 0 {
        return Ret::eax(0);
    }
    let flags = g.read::<u32>(w(0x150));
    if g.read::<u8>(flags) & 3 != 0 {
        return Ret::eax(flags);
    }
    let second = Spark {
        take: 0x180,
        tables: [0x1a8, 0x1e4, 0x1fe, 0x221, 0x244, 0x260, 0x277],
        mask: 0x23f,
        k1: 0x2b2,
        scale: 0x2cb,
        spread: (QUARTER, QUARTER, QUARTER),
        neg_add: None,
        drag: HUNDREDTH,
        life: (0xffff_ffff, 0),
        z_first: false,
        extra: &[],
    };
    let res = |g: &mut G| g.read::<u32>(g.read::<u32>(w(0x190)).wrapping_add(res_off));
    let (_, eax) = spark(g, img, f, pos, res, &second);
    Ret::eax(eax)
}

/// Two sparks at a position (internal, `ecx` = the position), the first
/// with the resource `**R` (operand `+0x5`, read first) and a lifted
/// fall (`K`, operand `+0xcd`); the second, unless `**G_F & 7` (operand
/// `+0x14c`), with the word `+0x28` of `*R2` (`+0x18f`). Both add the
/// jitter after `z`. Returns the last slot 314's `eax`, `*G_F`'s pointer
/// or 0.
///
/// `magic_0446` 0x100018b0 (729 bytes, `ecx`; 10 DLLs).
pub fn sparks_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let pos = fr.ecx;
    let res = g.read::<u32>(g.read::<u32>(w(0x5)));
    let first = Spark {
        take: 0x13,
        tables: [0x38, 0x71, 0x8b, 0xae, 0xd2, 0xf5, 0x10c],
        mask: 0xe0,
        k1: 0x147,
        scale: 0x165,
        spread: (TENTH, TENTH, TENTH),
        neg_add: Some(0xcd),
        drag: HUNDREDTH,
        life: (0xffff_ffff, 0),
        z_first: true,
        extra: &[(0x68, 0x18b4_968c, 4)],
    };
    let (p, _) = spark(g, img, f, pos, |_| res, &first);
    if p == 0 {
        return Ret::eax(0);
    }
    let flags = g.read::<u32>(w(0x14c));
    if g.read::<u8>(flags) & 7 != 0 {
        return Ret::eax(flags);
    }
    let second = Spark {
        take: 0x17f,
        tables: [0x1a7, 0x1e3, 0x1fd, 0x220, 0x243, 0x25f, 0x276],
        mask: 0x23e,
        k1: 0x2b1,
        scale: 0x2ca,
        spread: (QUARTER, HALF, QUARTER),
        neg_add: None,
        drag: 0x3ccc_cccd,
        life: (0xffff_ffff, 0),
        z_first: true,
        extra: &[],
    };
    let res = |g: &mut G| g.read::<u32>(g.read::<u32>(w(0x18f)).wrapping_add(0x28));
    let (_, eax) = spark(g, img, f, pos, res, &second);
    Ret::eax(eax)
}

/// [`sparks_c`].
pub const SPARKS_C: Sig = Sig {
    len: 729,
    hash: 0x3d41_fb53_4067_eb1d,
    calls: &[
        (0x13, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x17f, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
    ],
    imports: &[],
};

/// Two sparks at a position (internal, `ecx` = the position), unless
/// `**G_F & 3` (operand `+0x5`): the first with the word `+0x38` of `*R`
/// (`+0x27`), a lifted fall (`+0xd7`), bytes `+0x67` 0xa0 and `+0x6b` 0;
/// the second with the word `+0x3c` of `*R2` (`+0x18c`), a lifted fall
/// (`+0x23c`) and `+0x68` set. Returns the last slot 314's `eax`,
/// `*G_F`'s pointer or 0.
///
/// `magic_0446` 0x10001b90 (741 bytes, `ecx`; 10 DLLs).
pub fn sparks_d<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let pos = fr.ecx;
    let flags = g.read::<u32>(w(0x5));
    if g.read::<u8>(flags) & 3 != 0 {
        return Ret::eax(flags);
    }
    let first = Spark {
        take: 0x17,
        tables: [0x3f, 0x7b, 0x95, 0xb8, 0xdc, 0xff, 0x116],
        mask: 0xea,
        k1: 0x151,
        scale: 0x16e,
        spread: (QUARTER, HALF, QUARTER),
        neg_add: Some(0xd7),
        drag: 0x3d4c_cccd,
        life: (0x20, 0x20),
        z_first: false,
        extra: &[(0x67, 0xa0, 1), (0x6b, 0, 1)],
    };
    let res = |g: &mut G| g.read::<u32>(g.read::<u32>(w(0x27)).wrapping_add(0x38));
    let (p, _) = spark(g, img, f, pos, res, &first);
    if p == 0 {
        return Ret::eax(0);
    }
    let second = Spark {
        take: 0x17c,
        tables: [0x1a4, 0x1e0, 0x1fa, 0x21d, 0x241, 0x264, 0x27b],
        mask: 0x24f,
        k1: 0x2b6,
        scale: 0x2d6,
        spread: (TENTH, 0x3e99_999a, TENTH),
        neg_add: Some(0x23c),
        drag: HUNDREDTH,
        life: (0xffff_ffff, 0),
        z_first: false,
        extra: &[(0x68, 0x40b4_c8a0, 4)],
    };
    let res = |g: &mut G| g.read::<u32>(g.read::<u32>(w(0x18c)).wrapping_add(0x3c));
    let (_, eax) = spark(g, img, f, pos, res, &second);
    Ret::eax(eax)
}

/// [`sparks_d`].
pub const SPARKS_D: Sig = Sig {
    len: 741,
    hash: 0x2996_adbb_e15b_5519,
    calls: &[
        (0x17, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x17c, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
    ],
    imports: &[],
};

/// [`sparks`] with the second resource at `*R2 + 0x20`.
///
/// `magic_0446` 0x100015d0 (730 bytes, `ecx`; 10 DLLs).
pub fn sparks_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    sparks(g, img, fr, 0x20)
}

/// [`sparks_a`].
pub const SPARKS_A: Sig = Sig {
    len: 730,
    hash: 0x3913_a026_843b_1a5e,
    calls: &[
        (0x9, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x180, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
    ],
    imports: &[],
};

/// [`sparks`] with the second resource at `*R2 + 0x18`.
///
/// `magic_0565` 0x10003170 (730 bytes, `ecx`; 10 DLLs).
pub fn sparks_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    sparks(g, img, fr, 0x18)
}

/// [`sparks_b`].
pub const SPARKS_B: Sig = Sig {
    len: 730,
    hash: 0x6787_ea3f_2011_eb16,
    calls: &[
        (0x9, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x180, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
    ],
    imports: &[],
};

/// The family's per-frame task (cdecl `(task)`; an 8-aligned frame): the
/// objects run ([`crate::l5_objhost::run_objects`]); when none ran and
/// the block's `+0xebfc` is 1, the task ends: its `+0x14` is 3, its
/// owner's (`+0x8`) callback `+0x10` the next function (operand `+0x30`),
/// the models are drawn ([`crate::sb_misc::draw_all`]) and the part
/// handles registered ([`crate::sa_misc::register_16`]). Returns the
/// last one's `eax`, or the objects' answer.
///
/// `magic_0543` 0x10004a90 (67 bytes; 9 DLLs).
pub fn objects_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let block = g.read::<u32>(w(0xa));
    // push ebp; and esp, -8; push ecx; push esi; call
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(12);
    let ran = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0xf), esp);
    if ran != 1 || g.read::<u32>(block + 0xebfc) != 1 {
        return Ret::eax(ran);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let owner = g.read::<u32>(task + 8);
    g.write::<u32>(owner + 0x10, w(0x30));
    crate::sb_misc::draw_all(g, img, img.rel32_target(f + 0x35), owner);
    Ret::eax(crate::sa_misc::register_16(
        g,
        img,
        img.rel32_target(f + 0x3a),
    ))
}

/// [`objects_task`].
pub const OBJECTS_TASK: Sig = Sig {
    len: 67,
    hash: 0x7e45_6214_745a_a5a7,
    calls: &[
        (0xf, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x35, Callee::Sig(&crate::sb_misc::DRAW_ALL)),
        (0x3a, Callee::Sig(&crate::sa_misc::REGISTER_16)),
    ],
    imports: &[],
};

/// The caster shown again (a table entry, no arguments): unless the word
/// `G_W` (operand `+0x3`) is set, slot 213 `(*G_A, 0)` (`+0x13`); then
/// `G_C` (`+0x31`) is 1 and slot 257 `(slot 219 (*G_B, 0), 0)` (`+0x2b`)
/// shows the element. Returns slot 257's `eax`.
///
/// `magic_0543` 0x10004c30 (85 bytes; 9 DLLs).
pub fn show_caster<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    if g.read::<u16>(w(0x3)) == 0 {
        let t = slot_target(g, w(0xb), 213);
        g.call(t, 0, &[g.read::<u32>(w(0x13)), 0]);
    }
    let b = g.read::<u32>(w(0x2b));
    g.write::<u32>(w(0x31), 1);
    let t = slot_target(g, w(0x23), 219);
    let elem = g.call(t, 0, &[b, 0]).eax;
    let t = slot_target(g, w(0x45), 257);
    Ret::eax(g.call(t, 0, &[elem, 0, b, 0]).eax)
}

/// [`show_caster`].
pub const SHOW_CASTER: Sig = Sig {
    len: 85,
    hash: 0x7b8d_769e_d1ae_d123,
    calls: &[],
    imports: &[],
};

/// Resets the block's counter (a table entry, no arguments): `+0xd74c`
/// is 0xc when `+0xd748` is 0, else 1; `+0xd740`, `+0xd744` and `+0xd750`
/// are cleared. Returns the new `+0xd74c`.
///
/// `magic_0543` 0x10003ad0 (63 bytes; 8 DLLs).
pub fn counter_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let block = g.read::<u32>(img.word(fr.at + 0x2));
    let n = if g.read::<u32>(block + 0xd748) == 0 {
        0xc
    } else {
        1
    };
    g.write::<u32>(block + 0xd740, 0);
    g.write::<u32>(block + 0xd744, 0);
    g.write::<u32>(block + 0xd74c, n);
    g.write::<u32>(block + 0xd750, 0);
    Ret::eax(n)
}

/// [`counter_reset`].
pub const COUNTER_RESET: Sig = Sig {
    len: 63,
    hash: 0x739b_3521_82bc_6aab,
    calls: &[],
    imports: &[],
};

/// An effect node (`ecx`) set up from a record on the frame (`[ebp -
/// 0x18]`: `[node, A, x, y, B, arg 3]`, `A`, `B` the operands at `+0x18`
/// and the third word slot's, `x`/`y` arg 1 and the operand `+0x1f`, or
/// arg 0 and `+0x1f`, per `args`) with [`crate::sb_nodes::effect_node`],
/// then the node (read again) gets its vtable (operand `+0x37`). Returns
/// the node.
fn effect_node_record<G: Game>(g: &mut G, img: &Image, fr: &Frame, words: [u32; 6]) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    // push ebp; sub esp, 0x18
    let rec = fr.esp.wrapping_sub(0x1c);
    for (i, x) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, x);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(fr.at + 0x2e), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x37));
    Ret::eax(node)
}

/// [`effect_node_record`] with `[node, +0x18, +0x1f, arg 1, +0x26, arg
/// 3]`.
///
/// `magic_0146` 0x10006d20 (63 bytes, `ecx` and four stack words; 8
/// DLLs).
pub fn effect_node_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let words = [
        fr.ecx,
        w(0x18),
        w(0x1f),
        fr.arg(g, 1),
        w(0x26),
        fr.arg(g, 3),
    ];
    effect_node_record(g, img, fr, words)
}

/// [`effect_node_a`].
pub const EFFECT_NODE_A: Sig = Sig {
    len: 63,
    hash: 0x1340_7922_cc62_eacf,
    calls: &[(0x2e, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};

/// [`effect_node_record`] with `[node, +0x18, arg 0, +0x1f, +0x26, arg
/// 3]`.
///
/// `magic_0186` 0x10007780 (63 bytes, `ecx` and four stack words; 8
/// DLLs).
pub fn effect_node_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let words = [
        fr.ecx,
        w(0x18),
        fr.arg(g, 0),
        w(0x1f),
        w(0x26),
        fr.arg(g, 3),
    ];
    effect_node_record(g, img, fr, words)
}

/// [`effect_node_b`].
pub const EFFECT_NODE_B: Sig = Sig {
    len: 63,
    hash: 0xc3da_433a_3544_8ec3,
    calls: &[(0x2e, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};
