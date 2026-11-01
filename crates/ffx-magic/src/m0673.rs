//! `magic_0673.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::runtime::slot_target;
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

/// The effect's start (0x10002080, no arguments): six globals from
/// immediates (four buffers, a count 20, a word 0); the actors looked up
/// ([`crate::sb_misc::lookup_actors`]); slot 719 `(P, 8, 0)` into `G_H`;
/// the parts' buffer ([`crate::s4_small::parts_buffer`]); the record's
/// `+0x5e84` 0; the two script blocks' callbacks, words, halfwords and
/// bytes; a task node (kind 1, callback `CB1`) appended to the task list;
/// the records set up ([`crate::sa_misc::records_init`] on `R`); a second
/// task node (kind 2, `CB2`). Returns 0.
pub fn effect_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    for o in [0x00u32, 0x0a, 0x14, 0x1e, 0x28, 0x32] {
        g.write::<u32>(w(o + 2), w(o + 6));
    }
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + 0x3d));
    let t = slot_target(g, w(0x42), 719);
    // call width: 3
    let h = g.call(t, 0, &[w(0x51), 8, 0]).eax;
    g.write::<u32>(w(0x5b), h);
    // slot 719's two words left; the return address
    let callee = Frame {
        at: img.rel32_target(f + 0x60),
        esp: fr.esp.wrapping_sub(0xc),
        ..Frame::default()
    };
    crate::s4_small::parts_buffer(g, img, &callee);
    let rec = g.read::<u32>(w(0x65));
    g.write::<u32>(rec.wrapping_add(w(0x6e)), 0);
    for (word, half, byte) in [(0x7bu32, 0x8fu32, 0x98u32), (0x9f, 0xb3, 0xbc)] {
        g.write::<u32>(w(word + 2), w(word + 6));
        g.write::<u32>(w(word + 0xc), w(word + 0x10));
        g.write::<u16>(w(half + 3), w(half + 7) as u16);
        g.write::<u8>(w(byte + 2), w(byte + 6) as u8);
    }
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xc4), w(0x77));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    crate::r12h_start::append(g, img, f, node, [0xca, 0xde, 0xfd, 0xf0, 0xfd]);
    crate::sa_misc::records_init(g, img, img.rel32_target(f + 0x102), w(0xf5));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x10c), w(0x107));
    g.write::<u8>(node.wrapping_add(0x18), 2);
    crate::r12h_start::append(g, img, f, node, [0x112, 0x126, 0x133, 0x141, 0x149]);
    Ret::eax(0)
}

/// [`effect_start`] (336 bytes, 1 overlay).
pub const EFFECT_START: Sig = Sig {
    len: 336,
    hash: 0x60e1_9a3c_bd27_91cb,
    calls: &[
        (0x3d, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x60, Callee::Sig(&crate::s4_small::PARTS_BUFFER_100000)),
        (0xc4, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x102, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0x10c, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
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

/// A drifting record started (0x100013b0, no arguments): a record from
/// the pool ([`crate::sb_misc::pool_take`]; none: returns 0); slot 719
/// `(P, 9, 0)` its `+0x30`, `+0x38` -1 and `+0x3a` 0x200 (halfwords),
/// `+0x34` 1; slot 237 `(*G_M, 10, 0, rec)` (slot 719's words under it);
/// `+0xc` 1.0; `+0x0` and `+0x8` each moved by slot 314 `(2.0)`; `+0x10`,
/// `+0x14`, `+0x18`, `+0x1c` slot 314 `(0.1)`, `(0.2)`, `(0.1)`, `(pi)`;
/// `+0x20..+0x2c` `(0, -0.005, 0, 1.0)`; `+0x4c` -1, `+0x50` 0, `+0x3c`
/// `(slot 314 (0.5) + A) * B`. Each float slot gets the one word the
/// original keeps on its stack (`mov [esp], x`; ST0 as `f32`). Returns
/// the last slot 314's `eax`.
pub fn drift_record<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x06));
    if rec == 0 {
        return Ret::eax(0);
    }
    let t = slot_target(g, w(0x16), 719);
    // call width: 3
    let h = g.call(t, 0, &[w(0x25), 9, 0]).eax;
    g.write::<u32>(rec + 0x30, h);
    g.write::<u16>(rec + 0x38, 0xffff);
    g.write::<u16>(rec + 0x3a, 0x200);
    g.write::<u32>(rec + 0x34, 1);
    let m = g.read::<u32>(w(0x57));
    let t = slot_target(g, w(0x40), 237);
    // call width: 7 (slot 719's words under it)
    g.call(t, 0, &[m, 0xa, 0, rec, w(0x25), 9, 0]);
    g.write::<u32>(rec + 0xc, 0x3f80_0000);
    let float = |g: &mut G, table: u32, x: u32| {
        let t = slot_target(g, w(table), 314);
        // call width: 1
        let out = g.call(t, 0, &[x]);
        (crate::ob_model::st0_f32(out.st0), out.eax)
    };
    for (table, at) in [(0x5eu32, 0u32), (0x88, 8)] {
        let (r, _) = float(g, table, 0x4000_0000);
        let v = add(g.read::<f32>(rec + at), r);
        g.write::<f32>(rec + at, v);
    }
    for (table, x, at) in [
        (0xadu32, 0x3dcc_cccdu32, 0x10u32),
        (0xc9, 0x3e4c_cccd, 0x14),
        (0xe0, 0x3dcc_cccd, 0x18),
        (0xf7, 0x4049_0fdb, 0x1c),
    ] {
        let (r, _) = float(g, table, x);
        g.write::<f32>(rec + at, r);
    }
    for (o, v) in [
        (0x20u32, 0u32),
        (0x24, 0xbba3_d70a),
        (0x28, 0),
        (0x2c, 0x3f80_0000),
    ] {
        g.write::<u32>(rec + o, v);
    }
    let (r, eax) = float(g, 0x10e, 0x3f00_0000);
    let v = mul(add(r, g.read::<f32>(w(0x149))), g.read::<f32>(w(0x162)));
    g.write::<u32>(rec + 0x4c, u32::MAX);
    g.write::<u32>(rec + 0x50, 0);
    g.write::<f32>(rec + 0x3c, v);
    Ret::eax(eax)
}

/// [`drift_record`] (368 bytes, 1 overlay).
pub const DRIFT_RECORD: Sig = Sig {
    len: 368,
    hash: 0xdda4_ac72_6b4f_8040,
    calls: &[(0x06, Callee::Sig(&crate::sb_misc::POOL_TAKE))],
    imports: &[],
};

/// A timer block at `t`: its colour (`+0x81` the word, `+0x80` 0xff,
/// `+0x85` 0xffff, `+0x87` the byte), `+0x88` the steps, the halfword
/// `+0x98` 0x48, `+0x90`/`+0x8c` the count, `+0x94` 0.
fn timer<G: Game>(g: &mut G, t: u32, (colour, top, steps, count): (u32, u8, u32, u32)) {
    g.write::<u32>(t + 0x81, colour);
    g.write::<u8>(t + 0x80, 0xff);
    g.write::<u16>(t + 0x85, 0xffff);
    g.write::<u8>(t + 0x87, top);
    g.write::<u32>(t + 0x88, steps);
    g.write::<u16>(t + 0x98, 0x48);
    g.write::<u32>(t + 0x90, count);
    g.write::<u32>(t + 0x8c, count);
    g.write::<u32>(t + 0x94, 0);
}

/// The two timers started (0x10001700, no arguments): with the record
/// `R = *G_R`, its callback `+0x104` and the byte `+0x126` 2; the timer
/// blocks `R + 0x57e0` (4 steps, 6) and `R + 0x5880` (10 steps, 15), each
/// on a task node ([`crate::sa_pool::take`], callback `CB`, `+0x14` the
/// block): the first linked after the list's last node without moving
/// `G_LAST` (or the list's first), the second after it as the last.
/// Returns the second node.
pub fn timers_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = g.read::<u32>(w(0x01));
    g.write::<u32>(r + 0x104, w(0x0c));
    g.write::<u8>(r + 0x126, 2);
    let t1 = r.wrapping_add(w(0x1a));
    timer(g, t1, (0xff00_ffff, 0x80, 4, 6));
    let first = take(g, img, img.rel32_target(f + 0x77), w(0x20));
    let mut head = g.read::<u32>(w(0x7d));
    g.write::<u32>(first.wrapping_add(0x14), t1);
    g.write::<u32>(first.wrapping_add(4), 0);
    if head == 0 {
        head = first;
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0xa8), first);
    } else {
        let last = g.read::<u32>(w(0x93));
        g.write::<u32>(first, last);
        g.write::<u32>(last.wrapping_add(4), first);
    }
    let t2 = g.read::<u32>(w(0xae)).wrapping_add(w(0xb4));
    g.write::<u32>(first.wrapping_add(8), first);
    timer(g, t2, (0xff80_ffff, 0, 0xa, 0xf));
    let second = take(g, img, img.rel32_target(f + 0x113), w(0xbc));
    g.write::<u32>(second.wrapping_add(0x14), t2);
    g.write::<u32>(second.wrapping_add(4), 0);
    g.write::<u32>(w(0x122), second);
    // the head is only 0 here when the first take gave 0 (which faults
    // before in the original); ported as written
    if head == 0 {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x13f), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first.wrapping_add(4), second);
    }
    g.write::<u32>(second.wrapping_add(8), second);
    Ret::eax(second)
}

/// [`timers_start`] (328 bytes, 1 overlay).
pub const TIMERS_START: Sig = Sig {
    len: 328,
    hash: 0x8572_92ef_ea02_f71d,
    calls: &[
        (0x77, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x113, Callee::Sig(&crate::sa_pool::TAKE)),
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

/// Starts a record's object pair (0x10001cd0, cdecl `(rec)`): with `i =
/// rec + 0x34`'s word, the objects `O = *G_FX + 0x260 (i + 0x17)` and
/// `O + 0x130`, each [`start_object_anim`](crate::s4_misc::start_object_anim)
/// with the scale `(slot 314 (0.1) + K1) * K2` (`+0x47`, `+0x55`; the
/// slot's ST0 stored as `f32` into the argument word, as the original's
/// `fstp [ebp + 8]`) and `(5, 0)`, `(7, 0x1e)`, at the point `P[i]` (the
/// 16-byte rows at `+0x2d`); the second gets the callback `+0x104` (the
/// immediate `+0xef`) and `+0x120` 0x24. Returns the point's `w`.
pub fn record_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.esp.wrapping_add(4);
    let i = g.read::<u32>(g.read::<u32>(arg).wrapping_add(0x34));
    let obj = i
        .wrapping_add(0x17)
        .wrapping_mul(0x260)
        .wrapping_add(g.read::<u32>(w(0x16)));
    let row = (i << 4).wrapping_add(w(0x2d));
    let mut last = 0;
    for (o, t_op, (call, index, value), (k1, k2)) in [
        (obj, 0x1d, (0x5a, 5, 0), (0x47, 0x55)),
        (obj.wrapping_add(0x130), 0x82, (0xbe, 7, 0x1e), (0xaa, 0xb9)),
    ] {
        let t = crate::runtime::slot_target(g, w(t_op), 314);
        let x = crate::ob_model::st0_f32(g.call(t, 0, &[0x3dcc_cccd]).st0);
        g.write::<f32>(arg, x);
        let scale = crate::sb_sse::mul(
            crate::sb_sse::add(x, g.read::<f32>(w(k1))),
            g.read::<f32>(w(k2)),
        );
        crate::s4_misc::start_object_anim(
            g,
            img,
            img.rel32_target(f + call),
            o,
            scale,
            index,
            value,
        );
        for k in 0..4 {
            last = g.read::<u32>(row + 4 * k);
            g.write::<u32>(o + 0xd0 + 4 * k, last);
        }
    }
    let o2 = obj.wrapping_add(0x130);
    g.write::<u32>(o2 + 0x104, w(0xef));
    g.write::<u32>(o2 + 0x120, 0x24);
    Ret::eax(last)
}

/// [`record_pair`] (256 bytes, 1 overlay).
pub const RECORD_PAIR: Sig = Sig {
    len: 256,
    hash: 0xb896_3975_ece2_45f3,
    calls: &[
        (0x5a, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xbe, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};
