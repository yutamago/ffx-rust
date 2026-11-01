//! `magic_0052.dll`'s functions shared with `0055`, `0064`, `0067`,
//! `0266`, `0353`, `0435`: internal helpers around the effect's model
//! (`TABLE[*G_INDEX]`, a byte index into a table of model handles).
//!
//! Frames: a port that passes the address of a local to the game keeps
//! the local where the original's frame has it (below the entry `esp`),
//! so the words the game sees are the original's; the hosts leave that
//! space to the port (the 32-bit hook runs ports 64 KiB below the
//! caller's stack, docs/formats/magic.md "Porting the overlays").

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The model handle `TABLE[byte *G_INDEX]` (operands of a `mov reg,
/// [G_INDEX]` and a `[eax*4 + TABLE]`).
fn model(g: &impl crate::Mem, img: &Image, f: u32, index_op: u32, table_op: u32) -> u32 {
    let i = u32::from(g.read::<u8>(g.read::<u32>(img.word(f + index_op))));
    g.read::<u32>(img.word(f + table_op) + 4 * i)
}

/// 0x10001580 (internal, no arguments): slot 357 (model, -1, -1) and
/// slot 356 (model, -1, -1; the first call's words stay on the stack).
/// Returns slot 356's `eax`.
pub fn reset_model<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let m = model(g, img, f, 0x01, 0x0e);
    let all = u32::MAX;
    let s357 = slot_target(g, w(0x13), 357);
    g.call(s357, 0, &[m, all, all]);
    let s356 = slot_target(g, w(0x23), 356);
    g.call(s356, 0, &[m, all, all, m, all, all]).eax
}

/// `reset_model` (57 bytes; `magic_0052`, `0064`).
pub const RESET_MODEL: Sig = Sig {
    len: 57,
    hash: 0xa138_7f1b_1433_be05,
    calls: &[],
    imports: &[],
};

/// 0x10001cd0 (internal; `edx` = a position, 3 floats): with `G_MODE`
/// 0 and a model index below 0x20, slot 255 (index, pos); else, the
/// model's handle `h`: slot 11 (h) says whether it is a model; if not,
/// slot 33 (h, x, y, z), else slot 32 (h, x, z). The floats are passed
/// as their bits. Returns the last slot's `eax`.
pub fn place<G: Game>(g: &mut G, img: &Image, f: u32, pos: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mode = g.read::<u32>(w(0x02));
    let index = g.read::<u32>(w(0x09));
    if mode == 0 {
        let i = u32::from(g.read::<u8>(index));
        if i < 0x20 {
            let s255 = slot_target(g, w(0x1d), 255);
            return g.call(s255, 0, &[i, pos]).eax;
        }
    }
    let i = u32::from(g.read::<u8>(index));
    let h = g.read::<u32>(w(0x35) + 4 * i);
    let s11 = slot_target(g, w(0x3a), 11);
    let r = g.call(s11, 0, &[h]).eax;
    let z = g.read::<u32>(pos + 8);
    if r == 0 {
        let s33 = slot_target(g, w(0x4f), 33);
        let y = g.read::<u32>(pos + 4);
        let x = g.read::<u32>(pos);
        g.call(s33, 0, &[h, x, y, z]).eax
    } else {
        let s32 = slot_target(g, w(0x4f), 32);
        let x = g.read::<u32>(pos);
        g.call(s32, 0, &[h, x, z]).eax
    }
}

/// `place` (162 bytes; 5 overlays).
pub const PLACE: Sig = Sig {
    len: 162,
    hash: 0x3ae0_4741_274e_a00f,
    calls: &[],
    imports: &[],
};

/// 0x10001f00 (internal; `ecx` = an argument passed on, `esp` the
/// frame's entry stack pointer): the four rows of the camera matrix
/// (slot 544 (i, &row), 16 bytes each, into the frame's locals at
/// `ebp - 0x44`, `- 0x34`, `- 0x24`, `- 0x14`), the first three floats of
/// each scaled by `K` (SSE), then slot 553 (ecx, &rows...; the earlier
/// calls' words stay on the stack). Returns slot 553's `eax`. (The
/// frame's stack cookie is the CRT's; nothing else reads it.)
pub fn scaled_camera<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ebp = esp.wrapping_sub(4);
    let rows = [0x44u32, 0x34, 0x24, 0x14].map(|o| ebp.wrapping_sub(o));
    let mut pushed: Vec<u32> = Vec::new();
    for (i, (row, t)) in (0u32..).zip(rows.iter().zip([0x16, 0x2b, 0x3e, 0x51])) {
        let s544 = slot_target(g, w(t), 544);
        let mut args = vec![i, *row];
        args.extend(pushed.iter().rev());
        // call width: 8 (i, the row and three earlier calls' two words)
        g.call(s544, 0, &args);
        pushed.extend([*row, i]);
    }
    let k = f32::from_bits(g.read::<u32>(w(0x63)));
    for row in rows {
        for j in 0..3 {
            let v = f32::from_bits(g.read::<u32>(row + 4 * j)) * k;
            g.write::<u32>(row + 4 * j, v.to_bits());
        }
    }
    let s553 = slot_target(g, w(0xb9), 553);
    let mut args = vec![ecx, rows[0], rows[1], rows[2], rows[3]];
    args.extend(pushed.iter().rev());
    // call width: 13 (five words and the four earlier calls' eight)
    g.call(s553, 0, &args).eax
}

/// `scaled_camera` (319 bytes; 4 overlays).
pub const SCALED_CAMERA: Sig = Sig {
    len: 319,
    hash: 0x6e46_556c_b5a6_e524,
    calls: &[(0x137, crate::Callee::Any)],
    imports: &[],
};

/// Operand offsets of the start-ups' shared head ([`flag_head`]).
pub(crate) struct FlagHead {
    /// The flag's source word and its copy.
    pub flag: (u32, u32),
    /// The index global, the table it picks from, the copy.
    pub index: u32,
    pub table: u32,
    pub value: u32,
    /// A pointer global and its immediate; a word cleared.
    pub ptr: (u32, u32),
    pub zero: u32,
    /// The model byte's pointer, the model table, slot 262's table
    /// global and its three immediates.
    pub b: u32,
    pub models: u32,
    pub g262: u32,
    pub xyz: [u32; 3],
    /// The word set to 1.0; the second model's byte pointer and table;
    /// slots 6 and 548's table globals.
    pub one: u32,
    pub b2: u32,
    pub models2: u32,
    pub g6: u32,
    pub g548: u32,
    /// A copied word (from, to), the task's argument, the `rel32` of
    /// the task start, the word cleared after it.
    pub copy: (u32, u32),
    pub arg: u32,
    pub task: u32,
    pub zero2: u32,
    /// Where the table word + 15 goes without the flag.
    pub plus15: u32,
}

/// The head of the start-ups of `magic_0052` and its kin: the flag
/// `(*F >> 11) & 1` copied, the value `table[*I]` copied, a pointer and
/// a cleared word set. With the flag: the model `M` = `models[**B]` gets
/// slot 262 `(M, X, Y, Z)`, a word 1.0, the model `M'` slots 6 `(M', 1)`
/// and 548 `(M')`, a word copied, the task started
/// ([`task_start`](crate::s3_misc::task_start)) with its immediate and
/// a word cleared; without it the value + 15 is stored. `esp` is the
/// entry's (the task's argument goes on the frame where the original
/// pushes it).
pub(crate) fn flag_head<G: Game>(g: &mut G, img: &Image, fr: &Frame, o: &FlagHead) {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    // sar 11; and 1: bit 11
    let flag = (g.read::<u32>(w(o.flag.0)) >> 11) & 1;
    g.write::<u32>(w(o.flag.1), flag);
    let i = g.read::<u32>(w(o.index));
    let v = g.read::<u32>(w(o.table).wrapping_add(i.wrapping_mul(4)));
    g.write::<u32>(w(o.ptr.0), w(o.ptr.1));
    g.write::<u32>(w(o.zero), 0);
    g.write::<u32>(w(o.value), v);
    if flag == 0 {
        g.write::<u32>(w(o.plus15), v.wrapping_add(0xf));
        return;
    }
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(o.b))));
    let m = g.read::<u32>(w(o.models).wrapping_add(4 * b));
    let s = slot_target(g, w(o.g262), 0x418 / 4);
    let [x, y, z] = o.xyz.map(w);
    g.call(s, 0, &[m, x, y, z]);
    g.write::<u32>(w(o.one), 0x3f80_0000);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(o.b2))));
    let m2 = g.read::<u32>(w(o.models2).wrapping_add(4 * b));
    let s = slot_target(g, w(o.g6), 0x18 / 4);
    g.call(s, 0, &[m2, 1, m, x]);
    let s = slot_target(g, w(o.g548), 0x890 / 4);
    g.call(s, 0, &[m2, m2, 1, m, x, y]);
    g.write::<u32>(w(o.copy.1), g.read::<u32>(w(o.copy.0)));
    // push esi (left from the slot 548 call); push imm; call: the
    // task's argument on this frame
    let esp = fr.esp.wrapping_sub(16);
    g.write::<u32>(esp + 4, w(o.arg));
    g.write::<u32>(esp + 8, m2);
    let at = img.rel32_target(f + o.task);
    crate::s3_misc::task_start(
        g,
        img,
        &Frame {
            at,
            esp,
            ..Frame::default()
        },
    );
    g.write::<u32>(w(o.zero2), 0);
}

/// A node appended to a task list (first `first_r`, read first; last
/// `last_r`; for an empty list the first `first_w` and last `last_w0`,
/// else the last `last_w`): `+0x4` 0, `+0x0` the old last (or 0), `+0x8`
/// itself. Operand offsets in the function at `f`. Returns whether the
/// list was empty.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_node<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    first_r: u32,
    last_r: u32,
    empty_w: (u32, u32),
    last_w: u32,
) -> bool {
    let w = |off: u32| img.word(f + off);
    let first = g.read::<u32>(w(first_r));
    g.write::<u32>(node + 4, 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(empty_w.0), node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(empty_w.1), node);
    } else {
        let last = g.read::<u32>(w(last_r));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(last_w), node);
    }
    first == 0
}

/// The start-up (0x10001a80, no arguments): [`flag_head`]; then `+0xd1`
/// cleared, the actor block ([`me_block`](crate::sa_misc::me_block)) for
/// the immediate `+0xcb`, and two pool nodes
/// ([`take`](crate::sa_pool::take), owners `+0xe2` and `+0x11e`, the
/// first flagged 1 at `+0x18`) appended to the task list (first
/// `+0xed`/`+0x119`/`+0x14e`, last `+0x104`/`+0x13f`/`+0x156`). Returns
/// 0. Each slot call passes the words the original leaves on the stack.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let head = FlagHead {
        flag: (0x01, 0x0c),
        index: 0x11,
        table: 0x19,
        value: 0x32,
        ptr: (0x1f, 0x23),
        zero: 0x29,
        b: 0x3d,
        models: 0x56,
        g262: 0x5b,
        xyz: [0x4f, 0x4a, 0x42],
        one: 0x6e,
        b2: 0x68,
        models2: 0x7e,
        g6: 0x83,
        g548: 0x8e,
        copy: (0x9c, 0xa4),
        arg: 0xa9,
        task: 0xae,
        zero2: 0xb7,
        plus15: 0xc5,
    };
    flag_head(g, img, fr, &head);
    g.write::<u32>(w(0xd1), 0);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0xda), w(0xcb));
    let take = img.rel32_target(f + 0xe7);
    let n1 = crate::sa_pool::take(g, img, take, w(0xe2));
    let mut first = g.read::<u32>(w(0xed));
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 4, 0);
    if first == 0 {
        first = n1;
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x119), first);
    } else {
        let last = g.read::<u32>(w(0x104));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    }
    g.write::<u32>(n1 + 8, n1);
    let n2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x126), w(0x11e));
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x14e), n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x156), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x13f), n2);
    }
    Ret::eax(0)
}

/// [`start_up`] (350 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 350,
    hash: 0x63db_bf50_f053_fa40,
    calls: &[
        (0xae, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0xda, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0xe7, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x126, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0064`'s start-up (0x10002800, no arguments): [`flag_head`];
/// the actor block ([`me_block`](crate::sa_misc::me_block)) for the
/// immediate `+0xcb`, and one pool node ([`take`](crate::sa_pool::take),
/// owner `+0xd8`) appended to the task list ([`append_node`]). Returns
/// 0.
pub fn start_up_0064<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let head = FlagHead {
        flag: (0x01, 0x0c),
        index: 0x11,
        table: 0x22,
        value: 0x31,
        ptr: (0x17, 0x1b),
        zero: 0x28,
        b: 0x3c,
        models: 0x56,
        g262: 0x5b,
        xyz: [0x4f, 0x4a, 0x45],
        one: 0x6e,
        b2: 0x68,
        models2: 0x7e,
        g6: 0x83,
        g548: 0x8e,
        copy: (0x9c, 0xa4),
        arg: 0xa9,
        task: 0xae,
        zero2: 0xb7,
        plus15: 0xc6,
    };
    flag_head(g, img, fr, &head);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0xd0), w(0xcb));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0xdd), w(0xd8));
    append_node(g, img, f, node, 0xe3, 0xf3, (0x10e, 0x116), 0x100);
    Ret::eax(0)
}

/// [`start_up_0064`] (285 bytes, 1 overlay).
pub const START_UP_0064: Sig = Sig {
    len: 285,
    hash: 0xf77b_dc97_7955_e7fc,
    calls: &[
        (0xae, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0xd0, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0xdd, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The load step (0x10001250, a table entry, no arguments): nothing more
/// while slot 225 `(1)` or slot 593 `()` is non-zero; the first time
/// (`*G_ASKED` 0, operand `+0x29`), slot 226 `(*G_RES, 0)` and `G_ASKED`
/// = 1; once slot 227 `(*G_RES)` reports the data loaded (1 or more),
/// the start-up ([`start_up`]) runs and two table words (operands
/// `+0x72`/`+0x7c`) get their next functions (the immediates `+0x76`,
/// `+0x80`). Returns 0.
pub fn load_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = slot_target(g, w(0x1), 225);
    if g.call(t, 0, &[1]).eax != 0 {
        return Ret::eax(0);
    }
    let t = slot_target(g, w(0x17), 593);
    if g.call(t, 0, &[]).eax != 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x29)) == 0 {
        let t = slot_target(g, w(0x31), 226);
        let res = g.read::<u32>(w(0x37));
        g.call(t, 0, &[res, 0]);
        g.write::<u32>(w(0x48), 1);
    }
    let t = slot_target(g, w(0x51), 227);
    let res = g.read::<u32>(w(0x57));
    if g.call(t, 0, &[res]).eax.cast_signed() < 1 {
        return Ret::eax(0);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x6c),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    start_up(g, img, &callee);
    g.write::<u32>(w(0x72), w(0x76));
    g.write::<u32>(w(0x7c), w(0x80));
    Ret::eax(0)
}

/// [`load_step`] (135 bytes, 1 overlay).
pub const LOAD_STEP: Sig = Sig {
    len: 135,
    hash: 0xc1cb_5c9f_8421_4ef8,
    calls: &[(0x6c, Callee::Sig(&START_UP))],
    imports: &[],
};
