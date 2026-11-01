//! `magic_0673.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::sa_pool::take;
use crate::sb_sse::{cvt, div};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts a fade entry (0x10001dd0, an overlay table entry, `(ctx)`; 18
/// table entries): with `k = ctx[0x38]`, the template `E` (32 bytes at
/// the immediate `+0x1d` + 32 k) and the record `R = *G_R + 0x59f8 +
/// 48 k` (`+0x23`): `R[0x28]` = the frame count `E[0x18]`, the step
/// `R[0x20]` = `K / count` (`+0x0a`), `+0x0`, `+0x1c`, `+0x24` 0 and the
/// flags halfword `+0x2c` 0; a pool node ([`take`], the owner the
/// immediate at `+0x41`; byte `+0x18` 1, `+0x14` = `R`, `+0x4` 0) at the
/// head of the list ([`push_head`]: the empty test `+0x6e`, head
/// `+0x85`/`+0x9f`, set `+0x97`/`+0x9f`); `R[0x4..0x18]` = `E[0..0x14]`,
/// flags 3, `R[0x24]` = `E[0x1c]`. Returns `E[0x1c]`.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ctx = fr.arg(g, 0);
    let k = g.read::<u32>(ctx + 0x38);
    let e = (k << 5).wrapping_add(w(0x1d));
    let r = g
        .read::<u32>(w(0x23))
        .wrapping_add(0x59f8)
        .wrapping_add(k.wrapping_mul(0x30));
    let count = g.read::<u32>(e.wrapping_add(0x18));
    let x = div(g.read::<f32>(w(0xa)), cvt(count));
    g.write::<u32>(r + 0x28, count);
    g.write::<u32>(r, 0);
    g.write::<u32>(r + 0x1c, 0);
    g.write::<f32>(r + 0x20, x);
    g.write::<u32>(r + 0x24, 0);
    g.write::<u16>(r + 0x2c, 0);
    let node = take(g, img, img.rel32_target(f + 0x68), w(0x41));
    let empty = g.read::<u32>(w(0x6e)) == 0;
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 0x14, r);
    g.write::<u32>(node + 4, 0);
    push_head(g, node, empty, (w(0x85), w(0x9f)), (w(0x97), w(0x9f)));
    for i in 0..6 {
        let v = g.read::<u32>(e.wrapping_add(4 * i));
        g.write::<u32>(r + 4 + 4 * i, v);
    }
    let fl = g.read::<u16>(r + 0x2c);
    g.write::<u16>(r + 0x2c, fl | 3);
    let v = g.read::<u32>(e.wrapping_add(0x1c));
    g.write::<u32>(r + 0x24, v);
    Ret::eax(v)
}

/// [`start`] (213 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 213,
    hash: 0xba7f_f0b5_06ac_63d1,
    calls: &[(0x68, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// The effect's per-frame task callback (0x10001300, cdecl `(node)`):
/// the 0x30-byte block `B` (operand `+0xf`) zeroed (`memset`), its
/// half-word `+2` (`+0x20`) gets bit 0x80, `+0x32` = `*G` (`+0x25`);
/// the two scripts (`+0x2d`, `+0x3c`) run
/// ([`script_run`](crate::sb_misc::script_run)); while the second one
/// runs, 0 is returned. Then `*G_FX + 0x5e84` = 1, the objects are freed
/// ([`objects_freed`](crate::s6_misc::objects_freed)), the parts
/// registered ([`register_parts`](crate::sa_misc::register_parts)) and
/// the node given back ([`give_back`](crate::sa_pool::give_back), a tail
/// jump with `ecx` = the node), whose `eax` it returns.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x6));
    g.write_bytes(w(0xf), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x20));
    g.write::<u16>(w(0x20), h | 0x80);
    let v = g.read::<u32>(w(0x25));
    g.write::<u32>(w(0x32), v);
    crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x37), w(0x2d));
    if crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x41), w(0x3c)) == 0 {
        return Ret::eax(0);
    }
    g.write::<u32>(fx + 0x5e84, 1);
    // push ebp, esi, the return address
    let freed = Frame {
        at: img.rel32_target(f + 0x54),
        esp: fr.esp.wrapping_sub(0xc),
        ..*fr
    };
    let eax = crate::s6_misc::objects_freed(g, img, &freed).eax;
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x59), eax);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x63),
        node,
    ))
}

/// [`task`] (106 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 106,
    hash: 0xb997_403c_5741_de1d,
    calls: &[
        (0x14, Callee::Import("memset")),
        (0x37, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x41, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x54, Callee::Sig(&crate::s6_misc::OBJECTS_FREED)),
        (0x59, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        (0x63, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// Starts object `i` (0x10001520, internal: `ecx` = `i`): the object
/// `*G_FX + 0x130 i` and its row `R = T + 0x1c i` of the table at
/// operand `+0x21` (`(x, y, z, w, scale, angle, index)`):
/// [`start_object_anim`](crate::s4_misc::start_object_anim) (scale
/// `R[4]`, index `R[6]`, 0), the point `R[0..4]` at its `+0xd0`, its
/// `+0xc4` = `R[5]`. Returns the object.
pub fn start_row<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let i = fr.ecx;
    let row = |op: u32| w(op).wrapping_add(i.wrapping_mul(0x1c));
    let obj = i.wrapping_mul(0x130).wrapping_add(g.read::<u32>(w(0x18)));
    let scale = g.read::<f32>(row(0x21));
    let index = g.read::<u32>(row(0x2a));
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x31), obj, scale, index, 0);
    for (k, op) in [0x38u32, 0x45, 0x52, 0x5f].into_iter().enumerate() {
        let v = g.read::<u32>(row(op));
        g.write::<u32>(obj.wrapping_add(0xd0 + 4 * k as u32), v);
    }
    let a = g.read::<u32>(row(0x6c));
    g.write::<u32>(obj.wrapping_add(0xc4), a);
    obj
}

/// [`start_row`] (126 bytes, 1 overlay).
pub const START_ROW: Sig = Sig {
    len: 126,
    hash: 0x1af4_aeeb_a79d_e574,
    calls: &[(0x31, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
    imports: &[],
};

/// [`start_row`] at the code `f + call` for row `i`.
fn row_at<G: Game>(g: &mut G, img: &Image, fr: &Frame, call: u32, i: u32) -> u32 {
    let callee = Frame {
        at: img.rel32_target(fr.at + call),
        ecx: i,
        ..*fr
    };
    start_row(g, img, &callee)
}

/// Starts the object of a record's row (0x100015a0, cdecl `(rec)`): a
/// tail jump into [`start_row`] with `ecx` = `rec + 0x2c`'s word, whose
/// object it returns.
pub fn start_row_of<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let i = g.read::<u32>(fr.arg(g, 0).wrapping_add(0x2c));
    Ret::eax(row_at(g, img, fr, 0xb, i))
}

/// [`start_row_of`] (15 bytes, 1 overlay).
pub const START_ROW_OF: Sig = Sig {
    len: 15,
    hash: 0xbd26_2909_0742_fc39,
    calls: &[(0xb, Callee::Sig(&START_ROW))],
    imports: &[],
};

/// Starts rows 11 and 12 (0x10001b00, an overlay table entry, no
/// arguments): each object ([`start_row`]) gets the callback `+0x104`
/// (the immediates `+0x15`, `+0x2e`) and `+0x120` 0x90. Returns the
/// second.
pub fn rows_b_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let mut o = 0;
    for (i, call, cb) in [(0xb, 0x6, 0x15), (0xc, 0x24, 0x2e)] {
        o = row_at(g, img, fr, call, i);
        g.write::<u32>(o.wrapping_add(0x104), img.word(fr.at + cb));
        g.write::<u32>(o.wrapping_add(0x120), 0x90);
    }
    Ret::eax(o)
}

/// [`rows_b_c`] (61 bytes, 1 overlay).
pub const ROWS_B_C: Sig = Sig {
    len: 61,
    hash: 0x7e38_598b_3923_bbf1,
    calls: &[
        (0x6, Callee::Sig(&START_ROW)),
        (0x24, Callee::Sig(&START_ROW)),
    ],
    imports: &[],
};

/// Starts rows 15, 16 and 19..28 (0x10001b60, an overlay table entry, no
/// arguments): each object ([`start_row`]) gets the callback `+0x104`
/// (the immediate after its call), the first two also `+0x120` 0xc2.
/// Returns the last.
pub fn rows_many<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    const ROWS: [(u32, u32, u32); 12] = [
        (0xf, 0x6, 0x15),
        (0x10, 0x24, 0x33),
        (0x13, 0x42, 0x51),
        (0x14, 0x56, 0x65),
        (0x15, 0x6a, 0x79),
        (0x16, 0x7e, 0x8d),
        (0x17, 0x92, 0xa1),
        (0x18, 0xa6, 0xb5),
        (0x19, 0xba, 0xc9),
        (0x1a, 0xce, 0xdd),
        (0x1b, 0xe2, 0xf1),
        (0x1c, 0xf6, 0x100),
    ];
    let mut o = 0;
    for (k, &(i, call, cb)) in ROWS.iter().enumerate() {
        o = row_at(g, img, fr, call, i);
        g.write::<u32>(o.wrapping_add(0x104), img.word(fr.at + cb));
        if k < 2 {
            g.write::<u32>(o.wrapping_add(0x120), 0xc2);
        }
    }
    Ret::eax(o)
}

/// [`rows_many`] (261 bytes, 1 overlay).
pub const ROWS_MANY: Sig = Sig {
    len: 261,
    hash: 0x7806_aa38_1e14_91b2,
    calls: &[
        (0x6, Callee::Sig(&START_ROW)),
        (0x24, Callee::Sig(&START_ROW)),
        (0x42, Callee::Sig(&START_ROW)),
        (0x56, Callee::Sig(&START_ROW)),
        (0x6a, Callee::Sig(&START_ROW)),
        (0x7e, Callee::Sig(&START_ROW)),
        (0x92, Callee::Sig(&START_ROW)),
        (0xa6, Callee::Sig(&START_ROW)),
        (0xba, Callee::Sig(&START_ROW)),
        (0xce, Callee::Sig(&START_ROW)),
        (0xe2, Callee::Sig(&START_ROW)),
        (0xf6, Callee::Sig(&START_ROW)),
    ],
    imports: &[],
};
