//! Small shared functions of the shared-4 list: one or two engine slot
//! calls with a store or a flag around them. Each doc names the DLL and
//! address it was ported from, its size and how many DLLs carry it;
//! "`G`" is a global of the DLL (an operand of the code).

use crate::l5_objhost::{LIST_REMOVE, RUN_OBJECTS, list_remove, run_objects};
use crate::runtime::slot_target;
use crate::s4_misc::{REGISTER_80, SCALE_TASK, register_80, scale_task};
use crate::s4_summon::trunc;
use crate::sa_misc::{REGISTER_PARTS, register_parts};
use crate::sa_pool::{GIVE_BACK, TAKE, give_back, take};
use crate::sa_summon::{BONE, MODEL_SETUP, bone_plain, model_setup, st0_f32};
use crate::sb_misc::{DRAW_ALL, draw_all};
use crate::sb_nodes::{PARTICLE_NODE, particle_node};
use crate::{Callee, Frame, Game, Image, Out, Ret, Sig};

/// A slot's result handed on as the function's own (`eax` and, for a
/// slot that answers a float, `ST0`).
fn through(out: Out) -> Ret {
    Ret {
        eax: out.eax,
        st0: out.st0,
        ..Ret::default()
    }
}

/// The word at `f + off` of the code (an operand).
fn op(img: &Image, f: u32, off: u32) -> u32 {
    img.word(f + off)
}

/// Slot `slot` through the table pointer at operand `table`, with `args`.
fn slot_call<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (table, slot): (u32, u32),
    args: &[u32],
) -> Out {
    let t = slot_target(g, op(img, f, table), slot);
    g.call(t, 0, args)
}

/// Sets bit 2 of the word at `arg` (the caller's "done" flag). Returns
/// `arg`.
fn mark_done<G: Game>(g: &mut G, arg: u32) -> Ret {
    let x = g.read::<u32>(arg);
    g.write::<u32>(arg, x | 4);
    Ret::eax(arg)
}

/// 1 when the word `G` is 0, else 0 (no arguments).
///
/// `magic_0393` 0x10002f70 (12 bytes; 3 DLLs).
pub fn word_clear<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(u32::from(g.read::<u32>(op(img, fr.at, 0x4)) == 0))
}

/// [`word_clear`].
pub const WORD_CLEAR: Sig = Sig {
    len: 12,
    hash: 0x1e60_a066_3fc1_839e,
    calls: &[],
    imports: &[],
};

/// Slot 574 without arguments. Returns 0.
///
/// `magic_0106` 0x10007750 (16 bytes; 3 DLLs).
pub fn call_574<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    slot_call(g, img, fr.at, (0x1, 574), &[]);
    Ret::eax(0)
}

/// [`call_574`].
pub const CALL_574: Sig = Sig {
    len: 16,
    hash: 0x3e7e_cf9f_3642_9fd8,
    calls: &[],
    imports: &[],
};

/// The word `G` set to the code's immediate (operand `+0x6`). Returns 1.
///
/// `magic_0443` 0x100019c0 (1.0) and `magic_0536` 0x10001190 (0) (16
/// bytes; 2 DLLs each).
pub fn store_one<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    g.write::<u32>(op(img, fr.at, 0x2), op(img, fr.at, 0x6));
    Ret::eax(1)
}

/// [`store_one`] storing 1.0.
pub const STORE_ONE_F: Sig = Sig {
    len: 16,
    hash: 0x00f8_1014_0716_54e2,
    calls: &[],
    imports: &[],
};

/// [`store_one`] storing 0.
pub const STORE_ONE_0: Sig = Sig {
    len: 16,
    hash: 0x71a4_1010_571f_9863,
    calls: &[],
    imports: &[],
};

/// Slot 296 `(this, 0)` (internal, `ecx` = this). Returns its `eax`.
///
/// `magic_0155` 0x1000d280 (20 bytes; 2 DLLs).
pub fn call_296<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    through(slot_call(g, img, fr.at, (0x1, 296), &[fr.ecx, 0]))
}

/// [`call_296`].
pub const CALL_296: Sig = Sig {
    len: 20,
    hash: 0x7468_ff04_17eb_bb71,
    calls: &[],
    imports: &[],
};

/// Slot 276 `(this + 4, 0)` with the word at `this + 4` (internal, `ecx`
/// = this). Returns its `eax`.
///
/// `magic_0611` 0x10002960 (22 bytes; 2 DLLs).
pub fn call_276<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let x = g.read::<u32>(fr.ecx.wrapping_add(4));
    through(slot_call(g, img, fr.at, (0x1, 276), &[x, 0]))
}

/// [`call_276`].
pub const CALL_276: Sig = Sig {
    len: 22,
    hash: 0xe408_71fa_8e54_8330,
    calls: &[],
    imports: &[],
};

/// Slot 432 without arguments, then the done flag (bit 2) of `*flags`
/// (cdecl `(flags)`). Returns `flags`.
///
/// `magic_0300` 0x100012e0 (24 bytes; 3 DLLs).
pub fn done_432<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    slot_call(g, img, fr.at, (0x4, 432), &[]);
    mark_done(g, fr.arg(g, 0))
}

/// [`done_432`].
pub const DONE_432: Sig = Sig {
    len: 24,
    hash: 0x8485_cdc2_1186_5fed,
    calls: &[],
    imports: &[],
};

/// Slot 434 `(*G, callback)` (operands `+0xf`, `+0x9`), then the done
/// flag of `*flags` (cdecl `(flags)`). Returns `flags`.
///
/// `magic_0356` 0x10001550 (38 bytes; 3 DLLs).
pub fn done_434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let x = g.read::<u32>(op(img, f, 0xf));
    slot_call(g, img, f, (0x4, 434), &[x, op(img, f, 0x9)]);
    mark_done(g, fr.arg(g, 0))
}

/// [`done_434`].
pub const DONE_434: Sig = Sig {
    len: 38,
    hash: 0x8615_ab3e_56f4_37ca,
    calls: &[],
    imports: &[],
};

/// Slot 213 `(*(*G + 0x198), 0)`, then the done flag of `*flags` (cdecl
/// `(flags)`). Returns `flags`.
///
/// `magic_0193` 0x10001cd0 (40 bytes; 2 DLLs).
pub fn done_213<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = g.read::<u32>(op(img, f, 0x4));
    let x = g.read::<u32>(obj.wrapping_add(0x198));
    slot_call(g, img, f, (0x11, 213), &[x, 0]);
    mark_done(g, fr.arg(g, 0))
}

/// [`done_213`].
pub const DONE_213: Sig = Sig {
    len: 40,
    hash: 0x0db0_623e_149c_c81c,
    calls: &[],
    imports: &[],
};

/// Slot 941 `(x)` (cdecl `(x)`). Returns its `eax`.
///
/// `magic_0481` 0x100033a0 (24 bytes; 2 DLLs).
pub fn call_941<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let x = fr.arg(g, 0);
    through(slot_call(g, img, fr.at, (0x4, 941), &[x]))
}

/// [`call_941`].
pub const CALL_941: Sig = Sig {
    len: 24,
    hash: 0xbcb5_5771_5429_66d9,
    calls: &[],
    imports: &[],
};

/// Slot 579 `(*G, 0, 0)` (no arguments). Returns its `eax`.
///
/// `magic_0434` 0x10007bb0 (27 bytes; 2 DLLs).
pub fn call_579<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let x = g.read::<u32>(op(img, fr.at, 0x11));
    through(slot_call(g, img, fr.at, (0x1, 579), &[x, 0, 0]))
}

/// [`call_579`].
pub const CALL_579: Sig = Sig {
    len: 27,
    hash: 0xa5b9_e36f_2383_e479,
    calls: &[],
    imports: &[],
};

/// Slot 579 `(*G, callback, 0)` (operands `+0x14`, `+0xe`; no
/// arguments). Returns its `eax`.
///
/// `magic_0434` 0x10007b90 (30 bytes; 2 DLLs).
pub fn call_579_with<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let x = g.read::<u32>(op(img, f, 0x14));
    through(slot_call(g, img, f, (0x1, 579), &[x, op(img, f, 0xe), 0]))
}

/// [`call_579_with`].
pub const CALL_579_WITH: Sig = Sig {
    len: 30,
    hash: 0x593f_f8de_bc06_c04f,
    calls: &[],
    imports: &[],
};

/// Slot 37 `(this, 1, x)` (internal, `ecx` = this, cdecl `(x)`).
/// Returns its `eax`.
///
/// `magic_0152` 0x1000d5c0 (27 bytes; 2 DLLs).
pub fn call_37<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let x = fr.arg(g, 0);
    through(slot_call(g, img, fr.at, (0x7, 37), &[fr.ecx, 1, x]))
}

/// [`call_37`].
pub const CALL_37: Sig = Sig {
    len: 27,
    hash: 0x9577_e229_b470_1ae3,
    calls: &[],
    imports: &[],
};

/// Slot 239 `(*G, -5, 2)` (no arguments). Returns its `eax`.
///
/// `magic_0250` 0x100033e0 (27 bytes; 2 DLLs).
pub fn call_239_back<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let x = g.read::<u32>(op(img, fr.at, 0x11));
    through(slot_call(
        g,
        img,
        fr.at,
        (0x1, 239),
        &[x, (-5i32).cast_unsigned(), 2],
    ))
}

/// [`call_239_back`].
pub const CALL_239_BACK: Sig = Sig {
    len: 27,
    hash: 0xec16_d870_8f44_0eb8,
    calls: &[],
    imports: &[],
};

/// Slot 239 `(*G, obj + 0x120, 2)` with the word at `obj + 0x120` (cdecl
/// `(obj)`). Returns its `eax`.
///
/// `magic_0347` 0x10003040 (38 bytes; 2 DLLs).
pub fn call_239_obj<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let y = g.read::<u32>(obj.wrapping_add(0x120));
    let x = g.read::<u32>(op(img, f, 0x15));
    through(slot_call(g, img, f, (0xf, 239), &[x, y, 2]))
}

/// [`call_239_obj`].
pub const CALL_239_OBJ: Sig = Sig {
    len: 38,
    hash: 0x29c1_3955_af21_2c22,
    calls: &[],
    imports: &[],
};

/// Slot 239 `(*G, edx, 0)` (internal, `edx` = the value). Returns its
/// `eax`.
///
/// `magic_0164` 0x1000cf90 (33 bytes; 2 DLLs).
pub fn call_239_edx<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let x = g.read::<u32>(op(img, fr.at, 0x14));
    // the `push ecx` that makes the frame's local is a fourth word the
    // slot does not take
    through(slot_call(g, img, fr.at, (0x5, 239), &[x, fr.edx, 0]))
}

/// [`call_239_edx`].
pub const CALL_239_EDX: Sig = Sig {
    len: 33,
    hash: 0xcc83_80fa_ffe2_e1bb,
    calls: &[],
    imports: &[],
};

/// While the word `G_BUSY` is 0: slot 239 `(*G_A, *G_B, 0)` (operands
/// `+0x1e`, `+0x12`) and 1 returned; else `eax` as it came (no
/// arguments).
///
/// `magic_0188` 0x10001d60 (45 bytes; 2 DLLs).
pub fn call_239_idle<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    if g.read::<u32>(op(img, f, 0x2)) != 0 {
        return Ret::eax(fr.eax);
    }
    let a = g.read::<u32>(op(img, f, 0x1e));
    let b = g.read::<u32>(op(img, f, 0x12));
    slot_call(g, img, f, (0xa, 239), &[a, b, 0]);
    Ret::eax(1)
}

/// [`call_239_idle`].
pub const CALL_239_IDLE: Sig = Sig {
    len: 45,
    hash: 0x01ff_030b_1e62_2050,
    calls: &[],
    imports: &[],
};

/// [`bone_plain`] for a node with the byte `G` as its rate (internal,
/// `ecx` = the node): a cursor on the frame holds the node and, at `+5`,
/// the rate. Returns the bone's result.
///
/// `magic_0070` 0x10008e80 (29 bytes; 2 DLLs).
pub fn bone_rate<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; sub esp, 8: the cursor is [ebp - 8]
    let cursor = fr.esp.wrapping_sub(12);
    let rate = g.read::<u8>(op(img, f, 0x7));
    g.write::<u32>(cursor, fr.ecx);
    g.write::<u8>(cursor + 5, rate);
    let callee = Frame {
        at: img.rel32_target(f + 0x15),
        esp: cursor.wrapping_sub(4),
        ecx: cursor,
        ..*fr
    };
    bone_plain(g, img, &callee)
}

/// [`bone_rate`].
pub const BONE_RATE: Sig = Sig {
    len: 29,
    hash: 0x9c5c_7d1b_e833_cdc6,
    calls: &[(0x15, Callee::Sig(&BONE))],
    imports: &[],
};

/// Slot 450 `(x, callback)` (operand `+0x9`; cdecl `(x)`). Returns its
/// `eax`.
///
/// `magic_0382` 0x10001100 (29 bytes; 2 DLLs).
pub fn call_450<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let x = fr.arg(g, 0);
    through(slot_call(
        g,
        img,
        fr.at,
        (0x4, 450),
        &[x, op(img, fr.at, 0x9)],
    ))
}

/// [`call_450`].
pub const CALL_450: Sig = Sig {
    len: 29,
    hash: 0x205c_e32c_b99a_7aa4,
    calls: &[],
    imports: &[],
};

/// Slot 237 `(model, v, 0, x)` with the model `this + 0x54` (internal,
/// `ecx` = this, `edx` = v, cdecl `(x)`). Returns its `eax`.
///
/// `magic_0155` 0x10008e50 (30 bytes; 2 DLLs).
pub fn call_237<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let model = g.read::<u32>(fr.ecx.wrapping_add(0x54));
    let x = fr.arg(g, 0);
    through(slot_call(g, img, fr.at, (0x7, 237), &[model, fr.edx, 0, x]))
}

/// [`call_237`].
pub const CALL_237: Sig = Sig {
    len: 30,
    hash: 0xa008_0522_0ee0_b16a,
    calls: &[],
    imports: &[],
};

/// Slot 250 `(x, y)` with the floats of `xmm0` and `xmm1` (internal).
/// Returns its result.
///
/// `magic_0042` 0x1000bde0 (31 bytes; 3 DLLs).
pub fn call_250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (x, y) = (fr.xmm_f32(0).to_bits(), fr.xmm_f32(1).to_bits());
    through(slot_call(g, img, fr.at, (0x1, 250), &[x, y]))
}

/// [`call_250`].
pub const CALL_250: Sig = Sig {
    len: 31,
    hash: 0xc9e6_9194_93b2_260c,
    calls: &[],
    imports: &[],
};

/// The node at `*G + 0xeac`, when there is one, given back to its pool
/// ([`give_back`]) and the field cleared (no arguments). Returns the
/// give-back's `eax`, or `eax` as it came.
///
/// `magic_0430` 0x10003320 (34 bytes; 2 DLLs).
pub fn give_back_eac<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(op(img, f, 0x3));
    let node = g.read::<u32>(block.wrapping_add(0xeac));
    if node == 0 {
        return Ret::eax(fr.eax);
    }
    let eax = give_back(g, img, img.rel32_target(f + 0x12), node);
    g.write::<u32>(block.wrapping_add(0xeac), 0);
    Ret::eax(eax)
}

/// [`give_back_eac`].
pub const GIVE_BACK_EAC: Sig = Sig {
    len: 34,
    hash: 0x28b6_80e4_6291_cb13,
    calls: &[(0x12, Callee::Sig(&GIVE_BACK))],
    imports: &[],
};

/// Slot 231 `(*G_A, *G_B, *G_C)` (operands `+0x19`, `+0xd`, `+0x2`; no
/// arguments). Returns its `eax`.
///
/// `magic_0481` 0x10003170 (35 bytes; 2 DLLs).
pub fn call_231<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let words = [0x19, 0xd, 0x2].map(|o| g.read::<u32>(op(img, f, o)));
    through(slot_call(g, img, f, (0x7, 231), &words))
}

/// [`call_231`].
pub const CALL_231: Sig = Sig {
    len: 35,
    hash: 0x20c1_54b2_ec82_4f77,
    calls: &[],
    imports: &[],
};

/// Slot 4 `(0x3016)`, then slot 27 `(0x3016, 1)`, which also sees the
/// first call's word (no arguments). Returns slot 27's `eax`.
///
/// `magic_0430` 0x100021e0 (36 bytes; 2 DLLs).
pub fn state_3016<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    slot_call(g, img, f, (0x1, 4), &[0x3016]);
    through(slot_call(g, img, f, (0x10, 27), &[0x3016, 1, 0x3016]))
}

/// [`state_3016`].
pub const STATE_3016: Sig = Sig {
    len: 36,
    hash: 0xf6c9_602c_7680_b082,
    calls: &[],
    imports: &[],
};

/// Slot 632 `(T[k], 0, 0)`: `k` the byte `*G + 1`, `T` a table of words
/// (operand `+0x10`; no arguments). Returns its `eax`.
///
/// `magic_0393` 0x10003020 (37 bytes; 3 DLLs).
pub fn call_632<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = g.read::<u32>(op(img, f, 0x1));
    let k = u32::from(g.read::<u8>(p.wrapping_add(1)));
    let x = g.read::<u32>(op(img, f, 0x10).wrapping_add(4 * k));
    through(slot_call(g, img, f, (0x15, 632), &[x, 0, 0]))
}

/// [`call_632`].
pub const CALL_632: Sig = Sig {
    len: 37,
    hash: 0x3bdf_b778_a4c1_b932,
    calls: &[],
    imports: &[],
};

/// `G_READY` 1, then the object's handler `+0x100` (operand `+0x16`) and
/// its `+0x11c` 0x50 (cdecl `(obj)`). Returns `obj`.
///
/// `magic_0249` 0x10001ea0 (38 bytes; 2 DLLs).
pub fn set_handler<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    g.write::<u32>(op(img, f, 0x8), 1);
    g.write::<u32>(obj.wrapping_add(0x100), op(img, f, 0x16));
    g.write::<u32>(obj.wrapping_add(0x11c), 0x50);
    Ret::eax(obj)
}

/// [`set_handler`].
pub const SET_HANDLER: Sig = Sig {
    len: 38,
    hash: 0x727d_40f0_6a82_4224,
    calls: &[],
    imports: &[],
};

/// The first free record of a table of 0x18-byte records: its `u16 +0x16`
/// 0xffff (the scan's start, operand `+0x3`, is the table, operand
/// `+0x21`, plus 0x16; it ends before operand `+0x12`). Returns the
/// record, or 0 (no arguments).
///
/// `magic_0184` 0x10001000 (38 bytes; 2 DLLs).
pub fn first_free<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let (mut p, end, table) = (op(img, f, 0x3), op(img, f, 0x12), op(img, f, 0x21));
    let mut i = 0u32;
    loop {
        if g.read::<u16>(p) == 0xffff {
            return Ret::eax(table.wrapping_add(0x18 * i));
        }
        p = p.wrapping_add(0x18);
        i += 1;
        if p.cast_signed() >= end.cast_signed() {
            return Ret::eax(0);
        }
    }
}

/// [`first_free`].
pub const FIRST_FREE: Sig = Sig {
    len: 38,
    hash: 0x1ba2_c53e_5dde_a9c6,
    calls: &[],
    imports: &[],
};

/// Slot 878 `(this)` with the lock `**G_LOCK` 1 around it (internal,
/// `ecx` = this). Returns the lock's address (the last `eax` loaded).
///
/// `magic_0106` 0x100086f0 (40 bytes; 2 DLLs).
pub fn locked_878<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let lock = g.read::<u32>(op(img, f, 0x1));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    slot_call(g, img, f, (0xf, 878), &[fr.ecx]);
    let lock = g.read::<u32>(op(img, f, 0x1d));
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// [`locked_878`].
pub const LOCKED_878: Sig = Sig {
    len: 40,
    hash: 0x33ce_14a9_7331_5060,
    calls: &[],
    imports: &[],
};

/// The models of two nodes (operands `+0xa`, `+0x19`) set up
/// ([`model_setup`], a cursor on the frame; no arguments). Returns 0.
///
/// `magic_0178` 0x10007950 (40 bytes; 2 DLLs).
pub fn two_models<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; push ecx: the cursor is [ebp - 4]
    let cursor = fr.esp.wrapping_sub(8);
    for (node, call) in [(0xa, 0xf), (0x19, 0x1e)] {
        g.write::<u32>(cursor, op(img, f, node));
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp: cursor.wrapping_sub(4),
            ecx: cursor,
            ..*fr
        };
        model_setup(g, img, &callee);
    }
    Ret::eax(0)
}

/// [`two_models`].
pub const TWO_MODELS: Sig = Sig {
    len: 40,
    hash: 0x70e2_ea51_749a_4651,
    calls: &[
        (0xf, Callee::Sig(&MODEL_SETUP)),
        (0x1e, Callee::Sig(&MODEL_SETUP)),
    ],
    imports: &[],
};

/// Slot 215 `(*G)` gives a part; slot 276 `(part, 0)` (no arguments; the
/// first call's word stays under the second's). Returns 0.
///
/// `magic_0570` 0x1000cad0 (41 bytes; 2 DLLs).
pub fn part_276<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let x = g.read::<u32>(op(img, f, 0x7));
    let part = slot_call(g, img, f, (0x1, 215), &[x]).eax;
    slot_call(g, img, f, (0x17, 276), &[part, 0, x]);
    Ret::eax(0)
}

/// [`part_276`].
pub const PART_276: Sig = Sig {
    len: 41,
    hash: 0xba54_3afb_6d9b_391b,
    calls: &[],
    imports: &[],
};

/// Slot 624 `(*G + 0x1ee4)` with that word, then the word -1 (no
/// arguments). Returns slot 624's `eax`.
///
/// `magic_0393` 0x10001b20 (41 bytes; 2 DLLs).
pub fn release_1ee4<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let at = g.read::<u32>(op(img, f, 0x8)).wrapping_add(0x1ee4);
    let x = g.read::<u32>(at);
    let out = slot_call(g, img, f, (0x1, 624), &[x]);
    g.write::<u32>(at, u32::MAX);
    through(out)
}

/// [`release_1ee4`].
pub const RELEASE_1EE4: Sig = Sig {
    len: 41,
    hash: 0xe681_f80c_0e67_8f0b,
    calls: &[],
    imports: &[],
};

/// Unless slot 595 answers non-zero (returned): `G_DONE` 1 and the node
/// given back to its pool ([`give_back`], a tail jump; cdecl `(node)`).
/// Returns the give-back's `eax`.
///
/// `magic_0052` 0x10001550 (41 bytes; 2 DLLs).
pub fn finish_give_back<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let out = slot_call(g, img, f, (0x4, 595), &[]);
    if out.eax != 0 {
        return through(out);
    }
    g.write::<u32>(op(img, f, 0x16), 1);
    let node = fr.arg(g, 0);
    Ret::eax(give_back(g, img, img.rel32_target(f + 0x23), node))
}

/// [`finish_give_back`].
pub const FINISH_GIVE_BACK: Sig = Sig {
    len: 41,
    hash: 0x7452_8d98_4f6b_58e9,
    calls: &[(0x23, Callee::Sig(&GIVE_BACK))],
    imports: &[],
};

/// While the `u16 G_BUSY` is 0, slot 213 `(*G, edx)`; then `G_SET` 1
/// either way (internal, `edx` = the value). Returns slot 213's `eax`, or
/// `eax` as it came.
///
/// `magic_0205` 0x10002420 (44 bytes; 3 DLLs).
pub fn call_213_idle<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let mut ret = Ret::eax(fr.eax);
    if g.read::<u16>(op(img, f, 0x3)) == 0 {
        let x = g.read::<u32>(op(img, f, 0x12));
        ret = through(slot_call(g, img, f, (0xb, 213), &[x, fr.edx]));
    }
    g.write::<u32>(op(img, f, 0x23), 1);
    ret
}

/// [`call_213_idle`].
pub const CALL_213_IDLE: Sig = Sig {
    len: 44,
    hash: 0x0bf1_e5e3_53b5_b170,
    calls: &[],
    imports: &[],
};

/// Slot 371 `(0, 0x10000)` then `(1, 0x20000)` (the first call's words
/// stay under the second's; no arguments). Returns the second's `eax`.
///
/// `magic_0304` 0x10001ca0 (44 bytes; 3 DLLs).
pub fn call_371_twice<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    slot_call(g, img, f, (0x1, 371), &[0, 0x1_0000]);
    through(slot_call(
        g,
        img,
        f,
        (0x15, 371),
        &[1, 0x2_0000, 0, 0x1_0000],
    ))
}

/// [`call_371_twice`].
pub const CALL_371_TWICE: Sig = Sig {
    len: 44,
    hash: 0xf86e_4ef0_6786_ec07,
    calls: &[],
    imports: &[],
};

/// Slot 719 `(this, 6, edx)` gives a handle; slot 393 `(handle, a, b)`
/// (internal, `ecx` = this, `edx`, cdecl `(a, b)`; the first call's
/// words stay under the second's). Returns slot 393's `eax`.
///
/// `magic_0391` 0x10004870 (45 bytes; 2 DLLs).
pub fn handle_393<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let h = slot_call(g, img, f, (0x4, 719), &[fr.ecx, 6, fr.edx]).eax;
    let (a, b) = (fr.arg(g, 0), fr.arg(g, 1));
    through(slot_call(
        g,
        img,
        f,
        (0x1c, 393),
        &[h, a, b, fr.ecx, 6, fr.edx],
    ))
}

/// [`handle_393`].
pub const HANDLE_393: Sig = Sig {
    len: 45,
    hash: 0x17a0_eea3_5ab0_f5e4,
    calls: &[],
    imports: &[],
};

/// Slot 211 `(*G, block)` (operands `+0xc`, `+0x6`), then bit 4 of the
/// `u16 G_FLAGS` (operand `+0x1b`) and `G_CUR` = `G_START` (operands
/// `+0x25`, `+0x29`; no arguments). Returns slot 211's `eax`.
///
/// `magic_0152` 0x10008230 (46 bytes; 3 DLLs).
pub fn call_211_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let x = g.read::<u32>(op(img, f, 0xc));
    let out = slot_call(g, img, f, (0x1, 211), &[x, op(img, f, 0x6)]);
    let flags = op(img, f, 0x1b);
    let v = g.read::<u16>(flags);
    g.write::<u16>(flags, v | 0x10);
    g.write::<u32>(op(img, f, 0x25), op(img, f, 0x29));
    through(out)
}

/// [`call_211_flag`].
pub const CALL_211_FLAG: Sig = Sig {
    len: 46,
    hash: 0xcace_696f_543e_3f39,
    calls: &[],
    imports: &[],
};

/// Slot 211 `(model, v + 0x9c)` with the model `this + 0x54`, then bit 4
/// of `u16 this + 0x90` and `this + 0x4c` = v (internal, `ecx` = this,
/// `edx` = v). Returns slot 211's `eax`.
///
/// `magic_0070` 0x100095f0 (46 bytes; 2 DLLs).
pub fn attach_211<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (obj, v) = (fr.ecx, fr.edx);
    let model = g.read::<u32>(obj.wrapping_add(0x54));
    let out = slot_call(g, img, fr.at, (0xc, 211), &[model, v.wrapping_add(0x9c)]);
    let flags = g.read::<u16>(obj.wrapping_add(0x90));
    g.write::<u16>(obj.wrapping_add(0x90), flags | 0x10);
    g.write::<u32>(obj.wrapping_add(0x4c), v);
    through(out)
}

/// [`attach_211`].
pub const ATTACH_211: Sig = Sig {
    len: 46,
    hash: 0xcd5a_d4b3_7243_ce8f,
    calls: &[],
    imports: &[],
};

/// Slot 568 `(0)` and slot 566 `(0, 0)` (the first's word stays under
/// the second's), then a tail jump to slot 575 with the caller's stack (no
/// arguments).
///
/// `magic_0168` 0x10002370 (46 bytes; 2 DLLs).
pub fn reset_then_575<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    slot_call(g, img, f, (0x1, 568), &[0]);
    slot_call(g, img, f, (0x10, 566), &[0, 0]);
    Ret::jump(slot_target(g, op(img, f, 0x1f), 575))
}

/// [`reset_then_575`].
pub const RESET_THEN_575: Sig = Sig {
    len: 46,
    hash: 0xa94b_bd21_621f_116f,
    calls: &[],
    imports: &[],
};

/// The push of an 8-bit immediate at `f + off` (sign-extended).
fn imm8(img: &Image, f: u32, off: u32) -> u32 {
    let b = img.word(f + off).to_le_bytes()[0];
    i32::from(i8::from_le_bytes([b])).cast_unsigned()
}

/// Slot 450 `(obj, callback)` (operand `+0x9`), then slot 451 `(obj, n)`
/// with the code's 8-bit `n` (operand `+0x1e`; the first call's words
/// stay under the second's; cdecl `(obj)`). Returns slot 451's `eax`.
///
/// `magic_0356` 0x10001770 (n 15) and `magic_0301` 0x10002220 (n 30)
/// (47 bytes; 2 DLLs each).
pub fn callback_and_count<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let cb = op(img, f, 0x9);
    slot_call(g, img, f, (0x4, 450), &[obj, cb]);
    through(slot_call(
        g,
        img,
        f,
        (0x19, 451),
        &[obj, imm8(img, f, 0x1e), obj, cb],
    ))
}

/// [`callback_and_count`] with 15.
pub const CALLBACK_AND_15: Sig = Sig {
    len: 47,
    hash: 0xf726_725a_acda_e8bb,
    calls: &[],
    imports: &[],
};

/// [`callback_and_count`] with 30.
pub const CALLBACK_AND_30: Sig = Sig {
    len: 47,
    hash: 0xda5b_23ac_0b60_e044,
    calls: &[],
    imports: &[],
};

/// `obj + 0x18c` -0.1, slot 450 `(obj, callback)` (operand `+0x13`), then
/// slot 451 `(obj, 10)` (the first call's words stay under the second's;
/// cdecl `(obj)`). Returns slot 451's `eax`.
///
/// `magic_0384` 0x100015a0 (58 bytes; 3 DLLs).
pub fn callback_and_10<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let cb = op(img, f, 0x13);
    g.write::<u32>(obj.wrapping_add(0x18c), 0xbdcc_cccd);
    slot_call(g, img, f, (0x4, 450), &[obj, cb]);
    through(slot_call(g, img, f, (0x25, 451), &[obj, 10, obj, cb]))
}

/// [`callback_and_10`].
pub const CALLBACK_AND_10: Sig = Sig {
    len: 58,
    hash: 0xe3dd_2922_8e58_96d0,
    calls: &[],
    imports: &[],
};

/// `G_READY` 1, slot 432 without arguments, then slot 450 `(obj,
/// callback)` (operand `+0x20`; cdecl `(obj)`). Returns slot 450's `eax`.
///
/// `magic_0384` 0x10001460 (52 bytes; 3 DLLs).
pub fn ready_callback<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    g.write::<u32>(op(img, f, 0xa), 1);
    slot_call(g, img, f, (0x4, 432), &[]);
    let obj = fr.arg(g, 0);
    through(slot_call(g, img, f, (0x1b, 450), &[obj, op(img, f, 0x20)]))
}

/// [`ready_callback`].
pub const READY_CALLBACK: Sig = Sig {
    len: 52,
    hash: 0xb034_5c56_bfa5_008e,
    calls: &[],
    imports: &[],
};

/// Slot 447 `(x, 0xc1)` with `x` the word `*G + 0x198`, then slot 461
/// `(obj, callback)` (operand `+0x26`; the first call's words stay under
/// the second's; cdecl `(obj)`). Returns slot 461's `eax`.
///
/// `magic_0415` 0x10001100 (58 bytes; 2 DLLs).
pub fn callback_461<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let base = g.read::<u32>(op(img, f, 0x4));
    let x = g.read::<u32>(base.wrapping_add(0x198));
    slot_call(g, img, f, (0x14, 447), &[x, 0xc1]);
    let obj = fr.arg(g, 0);
    through(slot_call(
        g,
        img,
        f,
        (0x21, 461),
        &[obj, op(img, f, 0x26), x, 0xc1],
    ))
}

/// [`callback_461`].
pub const CALLBACK_461: Sig = Sig {
    len: 58,
    hash: 0x491e_7de3_73f6_1cff,
    calls: &[],
    imports: &[],
};

/// Slot 623 `(*G, 0, 0, 0, 0, 0, 0)` gives a handle, kept at the block's
/// `+0x1ee4` (the block `*G_BLOCK` read before the call; no arguments).
/// Returns the handle.
///
/// `magic_0393` 0x10001ae0 (49 bytes; 2 DLLs).
pub fn take_1ee4<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(op(img, f, 0xe));
    let x = g.read::<u32>(op(img, f, 0x20));
    let h = slot_call(g, img, f, (0x1, 623), &[x, 0, 0, 0, 0, 0, 0]).eax;
    g.write::<u32>(block.wrapping_add(0x1ee4), h);
    Ret::eax(h)
}

/// [`take_1ee4`].
pub const TAKE_1EE4: Sig = Sig {
    len: 49,
    hash: 0x3c8b_1ef2_2d5f_3528,
    calls: &[],
    imports: &[],
};

/// The objects run ([`run_objects`]); when they are all done (1) and the
/// block's `+0x1c08` is 1 too, slot 214 `(*G)` (no arguments). Returns
/// slot 214's `eax`, or the run's.
///
/// `magic_0349` 0x10004760 (49 bytes; 2 DLLs).
pub fn run_then_214<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(op(img, f, 0x3));
    // push esi; call: the run's return address
    let done = run_objects(g, img, img.rel32_target(f + 0x8), fr.esp.wrapping_sub(8));
    if done != 1 || g.read::<u32>(block.wrapping_add(0x1c08)) != 1 {
        return Ret::eax(done);
    }
    let x = g.read::<u32>(op(img, f, 0x20));
    through(slot_call(g, img, f, (0x1a, 214), &[x]))
}

/// [`run_then_214`].
pub const RUN_THEN_214: Sig = Sig {
    len: 49,
    hash: 0x370e_2474_38db_a398,
    calls: &[(0x8, Callee::Sig(&RUN_OBJECTS))],
    imports: &[],
};

/// A task step: the script or motion (`first`, the `rel32` at `+0x5`)
/// and the objects ([`run_objects`], `+0xc`) run; when both are done (1),
/// and `guard` (an operand) is not above 0 when there is one, the task
/// `t` (cdecl `(t)`) gets state 3 (`+0x14`) and its record (`+0x8`) the
/// next function (operand `next`). Returns the record, or the objects'
/// result.
fn step_done<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    first: fn(&mut G, &Image, u32) -> u32,
    (guard, next): (Option<u32>, u32),
) -> Ret {
    step_done_at(g, img, fr, first, (0x5, 0xc), (guard, next))
}

/// [`step_done`] with its two `rel32`s at `rels`.
fn step_done_at<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    first: fn(&mut G, &Image, u32) -> u32,
    rels: (u32, u32),
    (guard, next): (Option<u32>, u32),
) -> Ret {
    let f = fr.at;
    let a = first(g, img, img.rel32_target(f + rels.0));
    // push ebp; push esi; call: the run's return address
    let b = run_objects(
        g,
        img,
        img.rel32_target(f + rels.1),
        fr.esp.wrapping_sub(12),
    );
    if b != 1 || a != 1 {
        return Ret::eax(b);
    }
    if let Some(gd) = guard
        && g.read::<i32>(op(img, f, gd)) > 0
    {
        return Ret::eax(b);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), op(img, f, next));
    Ret::eax(rec)
}

/// [`step_done`] with the script ([`crate::sa_script::run`]).
///
/// `magic_0369` 0x10001340 (48 bytes; 2 DLLs).
pub fn script_step_done<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    step_done(g, img, fr, crate::sa_script::run, (None, 0x29))
}

/// [`script_step_done`].
pub const SCRIPT_STEP_DONE: Sig = Sig {
    len: 48,
    hash: 0x98ce_01e7_f9a5_0997,
    calls: &[
        (0x5, Callee::Sig(&crate::sa_script::RUN)),
        (0xc, Callee::Sig(&RUN_OBJECTS)),
    ],
    imports: &[],
};

/// [`step_done`] with the motion ([`crate::s3_motion::motion_step`]) and
/// the guard `G_WAIT` (operand `+0x1b`, signed).
///
/// `magic_0442` 0x10002f10 (57 bytes; 2 DLLs).
pub fn motion_step_done<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    step_done(
        g,
        img,
        fr,
        crate::s3_motion::motion_step,
        (Some(0x1b), 0x32),
    )
}

/// [`motion_step_done`].
pub const MOTION_STEP_DONE: Sig = Sig {
    len: 57,
    hash: 0xfd14_9868_796d_0654,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&RUN_OBJECTS)),
    ],
    imports: &[],
};

/// Slot 219 `(k, 1)` with `k` the byte `**G` gives a part; when there is
/// one, slot 257 `(part, 1)` (no arguments). Returns slot 257's `eax`, or
/// 0.
///
/// `magic_0162` 0x10004f30 (51 bytes; 2 DLLs).
pub fn part_257<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = g.read::<u32>(op(img, f, 0x1));
    let k = u32::from(g.read::<u8>(p));
    let part = slot_call(g, img, f, (0xc, 219), &[k, 1]).eax;
    if part == 0 {
        return Ret::eax(0);
    }
    through(slot_call(g, img, f, (0x23, 257), &[part, 1]))
}

/// [`part_257`].
pub const PART_257: Sig = Sig {
    len: 51,
    hash: 0xa7ae_d42c_f621_983b,
    calls: &[],
    imports: &[],
};

/// Slot 402 without arguments, `*G_P` = `ITEM` (operands `+0x11`,
/// `+0x1a`), then slot 713 `(x)` (cdecl `(x)`). Returns 1.
///
/// `magic_0484` 0x10001320 (53 bytes; 2 DLLs).
pub fn free_713<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    slot_call(g, img, f, (0x4, 402), &[]);
    let x = fr.arg(g, 0);
    let p = g.read::<u32>(op(img, f, 0x11));
    g.write::<u32>(p, op(img, f, 0x1a));
    slot_call(g, img, f, (0x1f, 713), &[x]);
    Ret::eax(1)
}

/// [`free_713`].
pub const FREE_713: Sig = Sig {
    len: 53,
    hash: 0xb470_ba28_e665_13a2,
    calls: &[],
    imports: &[],
};

/// Slot 237 `(*G, bone, 0, obj + 0xd0)` puts the object's place (`+0xd0`)
/// at a bone of the model `*G`; `+0xdc` (w) 1.0. Returns slot 237's
/// `eax`.
fn place_at_bone<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    (table, model): (u32, u32),
    bone: u32,
) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let m = g.read::<u32>(op(img, f, model));
    let out = slot_call(
        g,
        img,
        f,
        (table, 237),
        &[m, bone, 0, obj.wrapping_add(0xd0)],
    );
    g.write::<u32>(obj.wrapping_add(0xdc), 0x3f80_0000);
    through(out)
}

/// [`place_at_bone`] with bone 0x99 (cdecl `(obj)`).
///
/// `magic_0470` 0x10001640 (53 bytes; 2 DLLs).
pub fn place_bone_99<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    place_at_bone(g, img, fr, (0x4, 0x22), 0x99)
}

/// [`place_bone_99`].
pub const PLACE_BONE_99: Sig = Sig {
    len: 53,
    hash: 0x14f6_8d85_2863_14ec,
    calls: &[],
    imports: &[],
};

/// [`place_at_bone`] with the object's bone `+0x120` (cdecl `(obj)`).
///
/// `magic_0307` 0x10002b30 (54 bytes; 2 DLLs).
pub fn place_bone_own<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let obj = fr.arg(g, 0);
    let bone = g.read::<u32>(obj.wrapping_add(0x120));
    place_at_bone(g, img, fr, (0x17, 0x1d), bone)
}

/// [`place_bone_own`].
pub const PLACE_BONE_OWN: Sig = Sig {
    len: 54,
    hash: 0x6ba1_d7d8_c2b1_abb3,
    calls: &[],
    imports: &[],
};

/// `*G_P` = `ITEM` (operands `+0x1`, `+0x8`), then slot 712 `(edx, 0x30)`
/// with the lock `**G_LOCK` 1 around it (internal, `edx` = the part).
/// Returns slot 712's `eax`.
///
/// `magic_0481` 0x10001000 (54 bytes; 2 DLLs).
pub fn locked_712<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = g.read::<u32>(op(img, f, 0x1));
    g.write::<u32>(p, op(img, f, 0x8));
    let lock = g.read::<u32>(op(img, f, 0xd));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let out = slot_call(g, img, f, (0x1c, 712), &[fr.edx, 0x30]);
    let lock = g.read::<u32>(op(img, f, 0x2b));
    g.write::<u32>(lock, saved);
    through(out)
}

/// [`locked_712`].
pub const LOCKED_712: Sig = Sig {
    len: 54,
    hash: 0x74be_dae2_eece_9fd3,
    calls: &[],
    imports: &[],
};

/// Slot 388 `(x, 14, 1)`: `x` the byte `G_MAP[i]` for the object's index
/// `i` (`+0x120`), or the word `G_NONE` when it is -1 (cdecl `(obj)`).
/// Returns slot 388's `eax`.
///
/// `magic_0349` 0x10004e90 (56 bytes; 2 DLLs).
pub fn call_388<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let i = g.read::<u32>(obj.wrapping_add(0x120));
    let x = if i == u32::MAX {
        g.read::<u32>(op(img, f, 0x12))
    } else {
        let map = g.read::<u32>(op(img, f, 0x19));
        u32::from(g.read::<u8>(map.wrapping_add(i)))
    };
    through(slot_call(g, img, f, (0x27, 388), &[x, 0xe, 1]))
}

/// [`call_388`].
pub const CALL_388: Sig = Sig {
    len: 56,
    hash: 0x282c_aeea_9626_a22e,
    calls: &[],
    imports: &[],
};

/// The frame count `edx` kept as `u16 G_N` and turned into a time `t =
/// edx * K1 * K2` (operands `+0x12`, `+0x24`) stored twice (`+0x2c`,
/// `+0x34`), `G_T` cleared (internal). Returns `t` in `xmm0`, `eax` as it
/// came.
///
/// `magic_0155` 0x10008d70 (57 bytes; 3 DLLs).
pub fn frames_time<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let n = fr.edx;
    let [lo, hi, _, _] = n.to_le_bytes();
    g.write::<u16>(op(img, f, 0xa), u16::from_le_bytes([lo, hi]));
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(op(img, f, o)));
    let mut t = crate::sb_sse::mul(crate::sb_sse::cvt(n), k(g, 0x12));
    g.write::<u32>(op(img, f, 0x18), 0);
    t = crate::sb_sse::mul(t, k(g, 0x24));
    g.write::<u32>(op(img, f, 0x2c), t.to_bits());
    g.write::<u32>(op(img, f, 0x34), t.to_bits());
    Ret::xmm0(crate::Xmm::F32(t.to_bits()), fr.eax)
}

/// [`frames_time`].
pub const FRAMES_TIME: Sig = Sig {
    len: 57,
    hash: 0x4d5f_17df_eee5_ccc9,
    calls: &[],
    imports: &[],
};

/// A big node set up ([`crate::m0070::node_init_big`], a five-word record
/// on the frame: the node `this`, the parent (operand `+0x15`), `x`, and
/// two words (operands `+0x1f`, `+0x26`)), then the node's handler table
/// (operand `+0x31`; internal, `ecx` = the node, cdecl `(x)`). Returns
/// the set-up's result.
///
/// `magic_0070` 0x100098a0 (58 bytes; 2 DLLs).
pub fn big_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let node = fr.ecx;
    // push ebp; sub esp, 0x14: the record is [ebp - 0x14]
    let rec = fr.esp.wrapping_sub(0x18);
    let x = fr.arg(g, 0);
    let words = [
        node,
        op(img, f, 0x15),
        x,
        op(img, f, 0x1f),
        op(img, f, 0x26),
    ];
    for (i, w) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, w);
    }
    let eax = crate::m0070::node_init_big(g, rec);
    g.write::<u32>(node, op(img, f, 0x31));
    Ret::eax(eax)
}

/// [`big_node`].
pub const BIG_NODE: Sig = Sig {
    len: 58,
    hash: 0x9f15_40a9_05fc_7fbd,
    calls: &[(0x2b, Callee::Sig(&crate::m0070::NODE_INIT_BIG))],
    imports: &[],
};

/// Slot 411 `(*G)`; when it answers -1, slot 213 `(*G, 0)` and 0 is
/// returned, else 1 (no arguments).
///
/// `magic_0530` 0x10001100 (60 bytes; 2 DLLs).
pub fn check_411<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let x = g.read::<u32>(op(img, f, 0x7));
    if slot_call(g, img, f, (0x1, 411), &[x]).eax != u32::MAX {
        return Ret::eax(1);
    }
    let x = g.read::<u32>(op(img, f, 0x24));
    slot_call(g, img, f, (0x1c, 213), &[x, 0]);
    Ret::eax(0)
}

/// [`check_411`].
pub const CHECK_411: Sig = Sig {
    len: 60,
    hash: 0x5e8c_c040_8a31_9f5e,
    calls: &[],
    imports: &[],
};

/// The light ranges of the actor's model (`**task + 0x88`, its `+0x54`):
/// slot 357 `(m, -1, -1)` and 356 `(m, -1, -1)` (the first call's words
/// stay under the second's; cdecl `(task)`). Returns slot 356's `eax`.
///
/// `magic_0297` 0x10003ce0 (60 bytes; 2 DLLs).
pub fn ranges_off<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let task = fr.arg(g, 0);
    let actor = g.read::<u32>(g.read::<u32>(task).wrapping_add(0x88));
    let m = g.read::<u32>(actor.wrapping_add(0x54));
    let all = u32::MAX;
    slot_call(g, img, f, (0x17, 357), &[m, all, all]);
    through(slot_call(
        g,
        img,
        f,
        (0x25, 356),
        &[m, all, all, m, all, all],
    ))
}

/// [`ranges_off`].
pub const RANGES_OFF: Sig = Sig {
    len: 60,
    hash: 0x1030_323f_6fcb_4d8f,
    calls: &[],
    imports: &[],
};

/// A particle node ([`particle_node`]) from a six-word record on the
/// frame: the node `this` (its handler table, operand `+0x16`, set
/// first), its list (operand `+0x20`), `x`, the owner `*G` (operand
/// `+0xd`), the part (operand `+0x2a`) and index 0 (internal, `ecx` = the
/// node, cdecl `(x)`). Returns the node's set-up result.
///
/// `magic_0159` 0x100066c0 (62 bytes; 2 DLLs).
pub fn particle_from<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let node = fr.ecx;
    // push ebp; sub esp, 0x18: the record is [ebp - 0x18]
    let rec = fr.esp.wrapping_sub(0x1c);
    let x = fr.arg(g, 0);
    let owner = g.read::<u32>(op(img, f, 0xd));
    g.write::<u32>(node, op(img, f, 0x16));
    let words = [node, op(img, f, 0x20), x, owner, op(img, f, 0x2a), 0];
    for (i, w) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, w);
    }
    Ret::eax(particle_node(g, img, img.rel32_target(f + 0x36), rec))
}

/// [`particle_from`].
pub const PARTICLE_FROM: Sig = Sig {
    len: 62,
    hash: 0x2782_0af5_e3eb_90ad,
    calls: &[(0x36, Callee::Sig(&PARTICLE_NODE))],
    imports: &[],
};

/// The object's place (`+0xd0..+0xdc`) from the block's `+0x1c00..`
/// (`*G`; cdecl `(obj)`). Returns the last word copied.
///
/// `magic_0393` 0x100030a0 (62 bytes; 2 DLLs).
pub fn place_from_block<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let block = g.read::<u32>(op(img, fr.at, 0x5));
    let obj = fr.arg(g, 0);
    let mut x = 0;
    for i in 0..4 {
        x = g.read::<u32>(block.wrapping_add(0x1c00 + 4 * i));
        g.write::<u32>(obj.wrapping_add(0xd0 + 4 * i), x);
    }
    Ret::eax(x)
}

/// [`place_from_block`].
pub const PLACE_FROM_BLOCK: Sig = Sig {
    len: 62,
    hash: 0x43f8_0e30_0f04_3e79,
    calls: &[],
    imports: &[],
};

/// While `G_DONE` is 0 and the count `G_N` below 32 (signed): slot 296
/// `(n, 0)`; else slot 257 `(*G_PART, 1)` (no arguments). Returns the
/// slot's `eax`.
///
/// `magic_0249` 0x100024b0 (64 bytes; 3 DLLs).
pub fn count_or_257<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    if g.read::<u32>(op(img, f, 0x2)) == 0 {
        let n = g.read::<u32>(op(img, f, 0xa));
        if n.cast_signed() < 0x20 {
            return through(slot_call(g, img, f, (0x17, 296), &[n, 0]));
        }
    }
    let part = g.read::<u32>(op(img, f, 0x30));
    through(slot_call(g, img, f, (0x28, 257), &[part, 1]))
}

/// [`count_or_257`].
pub const COUNT_OR_257: Sig = Sig {
    len: 64,
    hash: 0x4b07_bd41_bd14_a92e,
    calls: &[],
    imports: &[],
};

/// A record's fields reset (internal, `ecx` = the record, cdecl `(x)`):
/// `+0x10..+0x1c` and `+0x28`, `+0x2c` 0, `u16 +0x30` 0, `+0x24` = x and
/// the callback `+0x20` (operand `+0x2a`). Returns 0.
///
/// `magic_0205` 0x10002450 (66 bytes; 2 DLLs).
pub fn record_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let r = fr.ecx;
    let x = fr.arg(g, 0);
    for off in [0x18, 0x14, 0x10, 0x1c] {
        g.write::<u32>(r.wrapping_add(off), 0);
    }
    g.write::<u32>(r.wrapping_add(0x24), x);
    g.write::<u32>(r.wrapping_add(0x20), op(img, fr.at, 0x2a));
    g.write::<u32>(r.wrapping_add(0x28), 0);
    g.write::<u32>(r.wrapping_add(0x2c), 0);
    g.write::<u16>(r.wrapping_add(0x30), 0);
    Ret::eax(0)
}

/// [`record_reset`].
pub const RECORD_RESET: Sig = Sig {
    len: 66,
    hash: 0x8651_69b0_c2ce_b792,
    calls: &[],
    imports: &[],
};

/// Slot 296 `(id, second)` for each of the `G_N` records (0xcc bytes from
/// operand `p`) whose first word is not 0xff. Returns the last slot's
/// `eax` or the last id read, or `eax` as it came with no records.
fn records_296<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    (n, p, table): (u32, u32, u32),
    second: u32,
) -> Ret {
    let f = fr.at;
    let mut left = g.read::<u32>(op(img, f, n));
    let mut rec = op(img, f, p);
    let mut ret = Ret::eax(fr.eax);
    while left != 0 {
        let id = g.read::<u32>(rec);
        left = left.wrapping_sub(1);
        ret = if id == 0xff {
            Ret::eax(id)
        } else {
            through(slot_call(g, img, f, (table, 296), &[id, second]))
        };
        rec = rec.wrapping_add(0xcc);
    }
    ret
}

/// [`records_296`] with 0 (no arguments).
///
/// `magic_0667` 0x10002660 (66 bytes; 2 DLLs).
pub fn records_296_off<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    records_296(g, img, fr, (0x8, 0xd, 0x23), 0)
}

/// [`records_296_off`].
pub const RECORDS_296_OFF: Sig = Sig {
    len: 66,
    hash: 0x64f6_e6b0_e03b_51f3,
    calls: &[],
    imports: &[],
};

/// [`records_296`] with `x` (cdecl `(x)`).
///
/// `magic_0515` 0x10002660 (77 bytes; 2 DLLs).
pub fn records_296_with<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let x = fr.arg(g, 0);
    records_296(g, img, fr, (0x7, 0xd, 0x2d), x)
}

/// [`records_296_with`].
pub const RECORDS_296_WITH: Sig = Sig {
    len: 77,
    hash: 0x7b59_3a94_d995_fe6e,
    calls: &[],
    imports: &[],
};

/// `G_A` and `G_B` cleared (operands `a`, `b`), slot 450 `(obj,
/// callback)` (operand `+0x9`), then slot `then` `(obj, y)` (operand
/// `then_table`; the first call's words stay under the second's). Returns
/// the second's `eax`.
fn clear_and_callback<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    (a, b): (u32, u32),
    (then_table, then, y): (u32, u32, u32),
) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let cb = op(img, f, 0x9);
    g.write::<u32>(op(img, f, a), 0);
    g.write::<u32>(op(img, f, b), 0);
    slot_call(g, img, f, (0x4, 450), &[obj, cb]);
    through(slot_call(g, img, f, (then_table, then), &[obj, y, obj, cb]))
}

/// [`clear_and_callback`], then slot 451 `(obj, 18)` (cdecl `(obj)`).
///
/// `magic_0312` 0x10001d30 (67 bytes; 2 DLLs).
pub fn clear_count_18<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    clear_and_callback(g, img, fr, (0x12, 0x1c), (0x2d, 451, 0x12))
}

/// [`clear_count_18`].
pub const CLEAR_COUNT_18: Sig = Sig {
    len: 67,
    hash: 0xf199_02b6_8840_2040,
    calls: &[],
    imports: &[],
};

/// [`clear_and_callback`], then slot 449 `(obj, callback 2)` (operand
/// `+0x32`; cdecl `(obj)`).
///
/// `magic_0194` 0x10001000 (70 bytes; 3 DLLs).
pub fn clear_two_callbacks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let y = op(img, fr.at, 0x32);
    clear_and_callback(g, img, fr, (0x18, 0x22), (0x2d, 449, y))
}

/// [`clear_two_callbacks`].
pub const CLEAR_TWO_CALLBACKS: Sig = Sig {
    len: 70,
    hash: 0x3f81_bd66_337a_9f00,
    calls: &[],
    imports: &[],
};

/// Slot 542 `(x, this)` for each index `i` below `G_N` (signed): `x` the
/// byte `G_MAP[i]` (the shared index helper's -1 case, the word
/// `G_NONE`, cannot come up: `i` counts up from 0) (internal, `ecx` =
/// this). Returns the last slot's `eax`, or `eax` as it came.
///
/// `magic_0391` 0x10001630 (67 bytes; 2 DLLs).
pub fn each_542<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let mut ret = Ret::eax(fr.eax);
    if g.read::<i32>(op(img, f, 0x8)) <= 0 {
        return ret;
    }
    let mut i = 0u32;
    loop {
        let x = if i == u32::MAX {
            g.read::<u32>(op(img, f, 0x16))
        } else {
            let map = g.read::<u32>(op(img, f, 0x1d));
            u32::from(g.read::<u8>(map.wrapping_add(i)))
        };
        ret = through(slot_call(g, img, f, (0x28, 542), &[x, fr.ecx]));
        i = i.wrapping_add(1);
        if i.cast_signed() >= g.read::<i32>(op(img, f, 0x3a)) {
            return ret;
        }
    }
}

/// [`each_542`].
pub const EACH_542: Sig = Sig {
    len: 67,
    hash: 0x4989_0ae7_fa8b_201b,
    calls: &[],
    imports: &[],
};

/// Slots 4 `(s)` and 27 `(s, 1)` for the states 0x3011 then 0x3013, each
/// call seeing the words the earlier ones left (no arguments). Returns
/// the last slot's `eax`.
///
/// `magic_0162` 0x10002d60 (68 bytes; 2 DLLs).
pub fn states_3011_3013<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let (a, b) = (0x3011, 0x3013);
    slot_call(g, img, f, (0x1, 4), &[a]);
    slot_call(g, img, f, (0x10, 27), &[a, 1, a]);
    slot_call(g, img, f, (0x21, 4), &[b, a, 1, a]);
    through(slot_call(g, img, f, (0x30, 27), &[b, 1, b, a, 1, a]))
}

/// [`states_3011_3013`].
pub const STATES_3011_3013: Sig = Sig {
    len: 68,
    hash: 0x8c13_ef7a_06ab_720b,
    calls: &[],
    imports: &[],
};

/// The object's place (`+0xd0`) from slot `slot` `(model, a, b, obj +
/// 0xd0)`; then `+0xdc` (w) 1.0 and `+0x104` 0. Returns the slot's `eax`.
fn place_and_clear<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    (table, slot): (u32, u32),
    words: &[u32],
) -> Ret {
    let obj = fr.arg(g, 0);
    let mut args = words.to_vec();
    args.push(obj.wrapping_add(0xd0));
    // call width: 3..4 (the callers' two or three words and the place)
    let out = slot_call(g, img, fr.at, (table, slot), &args);
    g.write::<u32>(obj.wrapping_add(0xdc), 0x3f80_0000);
    g.write::<u32>(obj.wrapping_add(0x104), 0);
    through(out)
}

/// [`place_and_clear`] by slot 268 `(T[k], 6, ..)`: `k` the byte `*G +
/// 1`, `T` a word table (operand `+0x1d`; cdecl `(obj)`).
///
/// `magic_0393` 0x10002f80 (70 bytes; 2 DLLs).
pub fn place_268<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = g.read::<u32>(op(img, f, 0x4));
    let k = u32::from(g.read::<u8>(p.wrapping_add(1)));
    let x = g.read::<u32>(op(img, f, 0x1d).wrapping_add(4 * k));
    place_and_clear(g, img, fr, (0x22, 268), &[x, 6])
}

/// [`place_268`].
pub const PLACE_268: Sig = Sig {
    len: 70,
    hash: 0xa939_1d71_8c26_22a5,
    calls: &[],
    imports: &[],
};

/// [`place_and_clear`] by slot 237 `(*G_MODEL, T[k], 0, ..)`: `k` the
/// word `G_K`, `T` a word table (operand `+0x19`; cdecl `(obj)`).
///
/// `magic_0055` 0x10001950 (72 bytes; 2 DLLs).
pub fn place_237_table<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let k = g.read::<u32>(op(img, f, 0x4));
    let x = g.read::<u32>(op(img, f, 0x19).wrapping_add(k.wrapping_mul(4)));
    let model = g.read::<u32>(op(img, f, 0x24));
    place_and_clear(g, img, fr, (0x1e, 237), &[model, x, 0])
}

/// [`place_237_table`].
pub const PLACE_237_TABLE: Sig = Sig {
    len: 72,
    hash: 0xacf0_654c_c1d1_c13c,
    calls: &[],
    imports: &[],
};

/// The actor `A = T[k]` (`k` the byte `*G`, `T` operand `+0x12`): slot
/// 364 `(A)` and slot 16 `(A)` answer floats (rounded to `f32`; the first
/// call's word stays under the second's); their product `slot 16 * slot
/// 364` (no arguments). Returns the product in `xmm0`, `eax` slot 16's.
///
/// `magic_0104` 0x10001830 (71 bytes; 2 DLLs).
pub fn scale_product<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = g.read::<u32>(op(img, f, 0x7));
    let k = u32::from(g.read::<u8>(p));
    let actor = g.read::<u32>(op(img, f, 0x12).wrapping_add(4 * k));
    let a = f32::from_bits(st0_f32(slot_call(g, img, f, (0x17, 364), &[actor])));
    let out = slot_call(g, img, f, (0x28, 16), &[actor, actor]);
    let b = f32::from_bits(st0_f32(out));
    Ret::xmm0(crate::Xmm::F32(crate::sb_sse::mul(b, a).to_bits()), out.eax)
}

/// [`scale_product`].
pub const SCALE_PRODUCT: Sig = Sig {
    len: 71,
    hash: 0xadf5_ebca_dbe5_63b9,
    calls: &[],
    imports: &[],
};

/// The block's object (`*G + 0x850`) taken out of the DLL's list
/// ([`list_remove`], the list operand `+0x10`); when it was there, `*G_P`
/// = `ITEM` (operands `+0x1e`, `+0x25`), slot 713 frees it and its flag
/// bit 15 (`u16 +0x124`) is cleared (no arguments). Returns 0x7fff, or 0.
///
/// `magic_0393` 0x10003050 (71 bytes; 2 DLLs).
pub fn unlist_block_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = g.read::<u32>(op(img, f, 0x3)).wrapping_add(0x850);
    if list_remove(g, img, img.rel32_target(f + 0x15), op(img, f, 0x10), obj) == 0 {
        return Ret::eax(0);
    }
    let p = g.read::<u32>(op(img, f, 0x1e));
    g.write::<u32>(p, op(img, f, 0x25));
    slot_call(g, img, f, (0x2a, 713), &[obj]);
    let flags = g.read::<u16>(obj.wrapping_add(0x124));
    g.write::<u16>(obj.wrapping_add(0x124), flags & 0x7fff);
    Ret::eax(0x7fff)
}

/// [`unlist_block_object`].
pub const UNLIST_BLOCK_OBJECT: Sig = Sig {
    len: 71,
    hash: 0x857c_b3f9_2b38_c701,
    calls: &[(0x15, Callee::Sig(&LIST_REMOVE))],
    imports: &[],
};

/// Slot 434 `(*G, callback)` (operands `+0xf`, `+0x9`), slot 450 `(obj,
/// callback 2)` (operand `+0x21`), then slot 451 `(obj, n)` with the
/// code's 8-bit `n` (operand `+0x36`), each call seeing the words the
/// earlier ones left (cdecl `(obj)`). Returns slot 451's `eax`.
///
/// `magic_0194` 0x10001170 (n 2) and `magic_0378` 0x100010d0 (n 5) (71
/// bytes; 2 DLLs each).
pub fn three_callbacks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let x = g.read::<u32>(op(img, f, 0xf));
    let (cb, cb2) = (op(img, f, 0x9), op(img, f, 0x21));
    slot_call(g, img, f, (0x4, 434), &[x, cb]);
    slot_call(g, img, f, (0x1c, 450), &[obj, cb2, x, cb]);
    let n = imm8(img, f, 0x36);
    through(slot_call(
        g,
        img,
        f,
        (0x31, 451),
        &[obj, n, obj, cb2, x, cb],
    ))
}

/// [`three_callbacks`] with 2.
pub const THREE_CALLBACKS_2: Sig = Sig {
    len: 71,
    hash: 0x7be2_622a_de65_12fb,
    calls: &[],
    imports: &[],
};

/// [`three_callbacks`] with 5.
pub const THREE_CALLBACKS_5: Sig = Sig {
    len: 71,
    hash: 0x6fa0_8d9f_9899_5aea,
    calls: &[],
    imports: &[],
};

/// The fade counter `obj + 0x120` down by one: from 64 up nothing more;
/// above 0 the four `u16 +0x70..+0x76` get `count * 64` (`(count << 12) /
/// 64`); at 0 or below the object is flagged 0x4000 (`u16 +0x124`)
/// (cdecl `(obj)`). Returns the count, the value or 0x4000.
///
/// `magic_0673` 0x10001900 (71 bytes; 2 DLLs).
pub fn fade_down<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let obj = fr.arg(g, 0);
    let count = g.read::<i32>(obj.wrapping_add(0x120)).wrapping_sub(1);
    g.write::<i32>(obj.wrapping_add(0x120), count);
    if count >= 0x40 {
        return Ret::eax(count.cast_unsigned());
    }
    if count <= 0 {
        let flags = g.read::<u16>(obj.wrapping_add(0x124));
        g.write::<u16>(obj.wrapping_add(0x124), flags | 0x4000);
        return Ret::eax(0x4000);
    }
    let v = (count << 12) / 64;
    let [lo, hi, _, _] = v.to_le_bytes();
    for off in [0x76, 0x74, 0x72, 0x70] {
        g.write::<u16>(obj.wrapping_add(off), u16::from_le_bytes([lo, hi]));
    }
    Ret::eax(v.cast_unsigned())
}

/// [`fade_down`].
pub const FADE_DOWN: Sig = Sig {
    len: 71,
    hash: 0x62f3_954d_f1eb_d15d,
    calls: &[],
    imports: &[],
};

/// The motion step and the objects run ([`step_done`]'s shape) with two
/// guards (`G_WAIT` not above 0, `G_BUSY` 0); then the task's state 3
/// and next function (operand `+0x3c`) and a tail jump into
/// [`register_parts`] with the record in `eax` (cdecl `(task)`). Returns
/// the parts' result, or the objects'.
///
/// `magic_0347` 0x10003c10 (73 bytes; 2 DLLs).
pub fn motion_step_register<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let a = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x5));
    // push ebp; push esi; call: the run's return address
    let b = run_objects(g, img, img.rel32_target(f + 0xc), fr.esp.wrapping_sub(12));
    if b != 1
        || a != 1
        || g.read::<i32>(op(img, f, 0x1b)) > 0
        || g.read::<u32>(op(img, f, 0x24)) != 0
    {
        return Ret::eax(b);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), op(img, f, 0x3c));
    Ret::eax(register_parts(g, img, img.rel32_target(f + 0x42), rec))
}

/// [`motion_step_register`].
pub const MOTION_STEP_REGISTER: Sig = Sig {
    len: 73,
    hash: 0x48d6_abf0_a2dc_1950,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&RUN_OBJECTS)),
        (0x42, Callee::Sig(&REGISTER_PARTS)),
    ],
    imports: &[],
};

/// The light's radius `xmm2` kept twice (operands `+0xa`, `+0x12`) and
/// its flag byte 1 (`+0x18`); slot 325 `(v, edx)` and slot 234 `(a, v,
/// b)` on the light's vectors (operands `+0x24`, `+0x40`, `+0x30`; the
/// first call's words stay under the second's) (internal, `edx`, `xmm2`).
/// Returns slot 234's `eax`.
///
/// `magic_0070` 0x10009480 (74 bytes; 3 DLLs).
pub fn light_set<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let r = fr.xmm_f32(2).to_bits();
    g.write::<u32>(op(img, f, 0xa), r);
    g.write::<u32>(op(img, f, 0x12), r);
    g.write::<u8>(op(img, f, 0x18), 1);
    let v = op(img, f, 0x24);
    slot_call(g, img, f, (0x1, 325), &[v, fr.edx]);
    let words = [
        op(img, f, 0x40),
        op(img, f, 0x3b),
        op(img, f, 0x30),
        v,
        fr.edx,
    ];
    through(slot_call(g, img, f, (0x2b, 234), &words))
}

/// [`light_set`].
pub const LIGHT_SET: Sig = Sig {
    len: 74,
    hash: 0xef60_4c74_b9a7_a9c7,
    calls: &[],
    imports: &[],
};

/// Slot 234 `(a, b, c)` on three vectors (operands `+0x1a`, `+0x15`,
/// `+0xa`), then `G_ON` 1, `G_R` = `xmm1` and `G_T` 0 (operands `+0x2f`,
/// `+0x3b`, `+0x41`) (internal, `xmm1`). Returns `xmm1` in `xmm0`, `eax`
/// slot 234's.
///
/// `magic_0070` 0x10009950 (77 bytes; 3 DLLs).
pub fn light_on<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let r = fr.xmm_f32(1).to_bits();
    let args = [op(img, f, 0x1a), op(img, f, 0x15), op(img, f, 0xa)];
    let eax = slot_call(g, img, f, (0x5, 234), &args).eax;
    g.write::<u32>(op(img, f, 0x2f), 1);
    g.write::<u32>(op(img, f, 0x3b), r);
    g.write::<u32>(op(img, f, 0x41), 0);
    Ret::xmm0(crate::Xmm::F32(r), eax)
}

/// [`light_on`].
pub const LIGHT_ON: Sig = Sig {
    len: 77,
    hash: 0x7d81_aa62_4cfc_84f5,
    calls: &[],
    imports: &[],
};

/// Each 16-byte entry of a table (operand `+0x6` minus 4, to operand
/// `+0x43`) copied into the 0x34-byte records from operand `+0x1` minus 8,
/// with `+0x20` 1 and `+0x18`, `+0x28` 0 (relative to the record + 8; no
/// arguments). Returns the last word copied.
///
/// `magic_0249` 0x10001ee0 (74 bytes; 2 DLLs).
pub fn records_from_table<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let (mut dst, mut src, end) = (op(img, f, 0x1), op(img, f, 0x6), op(img, f, 0x43));
    loop {
        let mut last = 0;
        for i in 0..4u32 {
            last = g.read::<u32>(src.wrapping_sub(4).wrapping_add(4 * i));
            g.write::<u32>(dst.wrapping_sub(8).wrapping_add(4 * i), last);
        }
        src = src.wrapping_add(0x10);
        g.write::<u32>(dst.wrapping_add(0x20), 1);
        g.write::<u32>(dst.wrapping_add(0x18), 0);
        g.write::<u32>(dst.wrapping_add(0x28), 0);
        dst = dst.wrapping_add(0x34);
        if src.cast_signed() >= end.cast_signed() {
            return Ret::eax(last);
        }
    }
}

/// [`records_from_table`].
pub const RECORDS_FROM_TABLE: Sig = Sig {
    len: 74,
    hash: 0x1b32_2b21_fc85_dbc1,
    calls: &[],
    imports: &[],
};

/// Slot 440 `(*G, obj, 0, 2)`, slot 450 `(obj, callback)` and slot 449
/// `(obj, callback 2)` (operands `+0x19`, `+0x25`, `+0x38`), each seeing
/// the words the earlier ones left (cdecl `(obj)`). Returns slot 449's
/// `eax`.
///
/// `magic_0312` 0x100016f0 (75 bytes; 2 DLLs).
pub fn three_440<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let x = g.read::<u32>(op(img, f, 0x19));
    let (cb, cb2) = (op(img, f, 0x25), op(img, f, 0x38));
    slot_call(g, img, f, (0x4, 440), &[x, obj, 0, 2]);
    slot_call(g, img, f, (0x20, 450), &[obj, cb, x, obj, 0, 2]);
    through(slot_call(
        g,
        img,
        f,
        (0x33, 449),
        &[obj, cb2, obj, cb, x, obj, 0, 2],
    ))
}

/// [`three_440`].
pub const THREE_440: Sig = Sig {
    len: 75,
    hash: 0x2dc7_dfdf_97e2_9775,
    calls: &[],
    imports: &[],
};

/// Slot 450 `(obj, callback)` and slot 449 `(obj, callback 2)` (operands
/// `+0x19`, `+0x2c`; the first call's words stay under the second's),
/// then the object's entry `obj + 0x2c + 16 * k` (`k` = `obj + 0xac`)
/// gets the word `*G + 0x198` read before (cdecl `(obj)`). Returns `2k`.
///
/// `magic_0312` 0x10001300 (76 bytes; 2 DLLs).
pub fn two_callbacks_entry<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let base = g.read::<u32>(op(img, f, 0x4));
    let x = g.read::<u32>(base.wrapping_add(0x198));
    let (cb, cb2) = (op(img, f, 0x19), op(img, f, 0x2c));
    slot_call(g, img, f, (0x14, 450), &[obj, cb]);
    slot_call(g, img, f, (0x27, 449), &[obj, cb2, obj, cb]);
    let k2 = g.read::<u32>(obj.wrapping_add(0xac)).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(k2.wrapping_mul(8)).wrapping_add(0x2c), x);
    Ret::eax(k2)
}

/// [`two_callbacks_entry`].
pub const TWO_CALLBACKS_ENTRY: Sig = Sig {
    len: 76,
    hash: 0x3346_6e58_6e39_e910,
    calls: &[],
    imports: &[],
};

/// Slot 447 `(x, 0xc1)` with `x` the word `*G + 0x198`, slot 450 `(obj,
/// callback)` (operand `+0x26`) and slot 451 `(obj, 90)`, each seeing the
/// words the earlier ones left (cdecl `(obj)`). Returns slot 451's `eax`.
///
/// `magic_0421` 0x100013d0 (76 bytes; 2 DLLs).
pub fn callback_451_90<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let base = g.read::<u32>(op(img, f, 0x4));
    let x = g.read::<u32>(base.wrapping_add(0x198));
    let obj = fr.arg(g, 0);
    let cb = op(img, f, 0x26);
    slot_call(g, img, f, (0x14, 447), &[x, 0xc1]);
    slot_call(g, img, f, (0x21, 450), &[obj, cb, x, 0xc1]);
    through(slot_call(
        g,
        img,
        f,
        (0x36, 451),
        &[obj, 0x5a, obj, cb, x, 0xc1],
    ))
}

/// [`callback_451_90`].
pub const CALLBACK_451_90: Sig = Sig {
    len: 76,
    hash: 0x233c_d2fe_ab38_2873,
    calls: &[],
    imports: &[],
};

/// The index of the actor whose character (`+0x830`) is `this`, among
/// the `G_N` actors from 1 (`T[G_MAP[i]]`; the shared index helper's -1
/// case, the word `G_SELF`, cannot come up) (internal, `ecx` = this).
/// Returns the index, or -1.
///
/// `magic_0434` 0x10003c90 (77 bytes; 2 DLLs).
pub fn actor_index<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let n = g.read::<i32>(op(img, f, 0x2));
    if n <= 1 {
        return Ret::eax(u32::MAX);
    }
    let map = g.read::<u32>(op(img, f, 0x15));
    let own = g.read::<u32>(op(img, f, 0x1c));
    let mut i = 1i32;
    loop {
        let chr = if i == -1 {
            own
        } else {
            let k = u32::from(g.read::<u8>(map.wrapping_add(i.cast_unsigned())));
            g.read::<u32>(op(img, f, 0x30).wrapping_add(4 * k))
        };
        if g.read::<u32>(chr.wrapping_add(0x830)) == fr.ecx {
            return Ret::eax(i.cast_unsigned());
        }
        i += 1;
        if i >= n {
            return Ret::eax(u32::MAX);
        }
    }
}

/// [`actor_index`].
pub const ACTOR_INDEX: Sig = Sig {
    len: 77,
    hash: 0x4909_e951_6062_80ee,
    calls: &[],
    imports: &[],
};

/// `G_A` cleared (operand `+0x14`), slot 448 `(2, 0)`, slot 450 `(obj,
/// callback)` and slot 449 `(obj, callback 2)` (operands `+0x24`,
/// `+0x39`), each seeing the words the earlier ones left (cdecl `(obj)`).
/// Returns slot 449's `eax`.
///
/// `magic_0193` 0x10001000 (77 bytes; 2 DLLs).
pub fn three_448<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let (cb, cb2) = (op(img, f, 0x24), op(img, f, 0x39));
    g.write::<u32>(op(img, f, 0x14), 0);
    slot_call(g, img, f, (0x4, 448), &[2, 0]);
    slot_call(g, img, f, (0x1f, 450), &[obj, cb, 2, 0]);
    through(slot_call(
        g,
        img,
        f,
        (0x34, 449),
        &[obj, cb2, obj, cb, 2, 0],
    ))
}

/// [`three_448`].
pub const THREE_448: Sig = Sig {
    len: 77,
    hash: 0x2800_239b_b843_9caa,
    calls: &[],
    imports: &[],
};

/// While `G_DONE` is 0 and the count `G_N` below 32 (signed): slot 541
/// `(n, 1.0, 10)`; else slot 38 `(*G_PART, 1.0, 10)` (no arguments; the
/// `push ecx` that makes the float's room leaves no junk). Returns the
/// slot's `eax`.
///
/// `magic_0168` 0x10003670 (80 bytes; 2 DLLs).
pub fn fade_541_or_38<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let one = 0x3f80_0000;
    if g.read::<u32>(op(img, f, 0x2)) == 0 {
        let n = g.read::<u32>(op(img, f, 0xa));
        if n.cast_signed() < 0x20 {
            return through(slot_call(g, img, f, (0x1f, 541), &[n, one, 10]));
        }
    }
    let part = g.read::<u32>(op(img, f, 0x46));
    through(slot_call(g, img, f, (0x30, 38), &[part, one, 10]))
}

/// [`fade_541_or_38`].
pub const FADE_541_OR_38: Sig = Sig {
    len: 80,
    hash: 0x62d7_ae31_3a87_1513,
    calls: &[],
    imports: &[],
};

/// While `G_DONE` is 0 and the byte `*G_K` below 32: slot 541 `(k,
/// xmm1, 10)`; else slot 38 `(T[k], xmm1, 10)` (`T` operand `+0x3f`)
/// (internal, `xmm1` the rate). Returns the slot's `eax`.
///
/// `magic_0055` 0x10002320 (84 bytes; 3 DLLs).
pub fn fade_541_or_38_by<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let rate = fr.xmm_f32(1).to_bits();
    let p = g.read::<u32>(op(img, f, 0x9));
    let k = u32::from(g.read::<u8>(p));
    if g.read::<u32>(op(img, f, 0x2)) == 0 && k < 0x20 {
        return through(slot_call(g, img, f, (0x21, 541), &[k, rate, 10]));
    }
    let part = g.read::<u32>(op(img, f, 0x3f).wrapping_add(4 * k));
    through(slot_call(g, img, f, (0x44, 38), &[part, rate, 10]))
}

/// [`fade_541_or_38_by`].
pub const FADE_541_OR_38_BY: Sig = Sig {
    len: 84,
    hash: 0x839a_37c2_5065_66ca,
    calls: &[],
    imports: &[],
};

/// Slot 371 `(0, 0x8000)` then `(1, 0x10000)` (the first call's words
/// stay under the second's), then the block's scale task (`*G + 0x1768`,
/// [`scale_task`] with the two scales of operands `+0x3b`, `+0x33` and
/// the count 0; no arguments). Returns the task's result.
///
/// `magic_0434` 0x10006890 (81 bytes; 2 DLLs).
pub fn scales_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(op(img, f, 0xe));
    slot_call(g, img, f, (0x1, 371), &[0, 0x8000]);
    slot_call(g, img, f, (0x1c, 371), &[1, 0x1_0000, 0, 0x8000]);
    // push esi, the five words, then the count: the task's frame
    let esp = fr.esp.wrapping_sub(28);
    for (i, w) in (1u32..).zip([0, 1, 0x1_0000, 0, 0x8000]) {
        g.write::<u32>(esp + 4 * i, w);
    }
    let k = |g: &G, o: u32| u128::from(g.read::<u32>(op(img, f, o)));
    let mut xmm = fr.xmm;
    xmm[1] = k(g, 0x3b);
    xmm[2] = k(g, 0x33);
    let callee = Frame {
        at: img.rel32_target(f + 0x48),
        esp,
        ecx: block.wrapping_add(0x1768),
        xmm,
        ..*fr
    };
    scale_task(g, img, &callee)
}

/// [`scales_task`].
pub const SCALES_TASK: Sig = Sig {
    len: 81,
    hash: 0xc018_8207_e380_ee1f,
    calls: &[(0x48, Callee::Sig(&SCALE_TASK))],
    imports: &[],
};

/// The level `G_L` stepped down by `K` (operand `+0xf`; `cvttss2si`,
/// negative to 0, capped at 128) and slot 391 `(0x60, 0x60, 0x60, L)`
/// (no arguments). Returns 1 while the level is not 0.
///
/// `magic_0530` 0x10001010 (81 bytes; 2 DLLs).
pub fn level_down<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let x = crate::sb_sse::cvt(g.read::<u32>(op(img, f, 0x4)));
    let k = f32::from_bits(g.read::<u32>(op(img, f, 0xf)));
    let l = trunc(crate::sb_sse::sub(x, k)).cast_signed();
    let l = if l < 0 { 0 } else { l.min(0x80) }.cast_unsigned();
    g.write::<u32>(op(img, f, 0x2b), l);
    slot_call(g, img, f, (0x30, 391), &[0x60, 0x60, 0x60, l]);
    Ret::eax(u32::from(g.read::<u32>(op(img, f, 0x49)) != 0))
}

/// [`level_down`].
pub const LEVEL_DOWN: Sig = Sig {
    len: 81,
    hash: 0x5975_d953_583a_f5bd,
    calls: &[],
    imports: &[],
};

/// Slot 10 `(*G_A, 1)`, then slot 804 for the handles `G_B` and `G_C`,
/// each cleared (each call seeing the words the earlier ones left; no
/// arguments). Returns 0.
///
/// `magic_0150` 0x1000ddf0 (82 bytes; 2 DLLs).
pub fn release_804<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let a = g.read::<u32>(op(img, f, 0x9));
    slot_call(g, img, f, (0x1, 10), &[a, 1]);
    let b = g.read::<u32>(op(img, f, 0x19));
    slot_call(g, img, f, (0x13, 804), &[b, a, 1]);
    let c = g.read::<u32>(op(img, f, 0x2c));
    g.write::<u32>(op(img, f, 0x38), 0);
    slot_call(g, img, f, (0x26, 804), &[c, b, a, 1]);
    g.write::<u32>(op(img, f, 0x47), 0);
    Ret::eax(0)
}

/// [`release_804`].
pub const RELEASE_804: Sig = Sig {
    len: 82,
    hash: 0x54f0_9519_b68c_1bf8,
    calls: &[],
    imports: &[],
};

/// Slots 4 `(s)` and 27 `(s, 1)` for the state 0x4029, then for the
/// block's handle `*G + 0x1eec` unless it is -1 (no arguments). Returns
/// the last slot's `eax`.
///
/// `magic_0393` 0x10001990 (82 bytes; 2 DLLs).
pub fn states_4029<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(op(img, f, 0xb));
    let s = 0x4029;
    slot_call(g, img, f, (0x1, 4), &[s]);
    let mut ret = through(slot_call(g, img, f, (0x17, 27), &[s, 1, s]));
    let h = g.read::<u32>(block.wrapping_add(0x1eec));
    if h != u32::MAX {
        slot_call(g, img, f, (0x36, 4), &[h]);
        ret = through(slot_call(g, img, f, (0x41, 27), &[h, 1, h]));
    }
    ret
}

/// [`states_4029`].
pub const STATES_4029: Sig = Sig {
    len: 82,
    hash: 0xb488_4754_2e96_1d5a,
    calls: &[],
    imports: &[],
};

/// Where a node-link function finds its list (operands): the parent
/// (`+0x14` and `+0x4`), the last node (read for `+0x8` and for the
/// test), the parent again (the empty list's last), the first-node, count
/// and last-node fields.
struct LinkOps {
    parent: (u32, u32),
    last: (u32, u32),
    empty: u32,
    first: u32,
    count: u32,
    tail: u32,
}

/// A fresh node `n` (pass `u16 +0x1a` 0xff, no children) put last in a
/// list (`+0x14`, `+0x4` the parent, `+0x8` the old last; the old last's
/// `+0x4`, or the list's first, becomes `n`; the count up). Returns the
/// old last.
fn link_last<G: Game>(g: &mut G, img: &Image, f: u32, n: u32, o: &LinkOps) -> u32 {
    g.write::<u16>(n.wrapping_add(0x1a), 0xff);
    g.write::<u32>(n.wrapping_add(0x10), n);
    g.write::<u32>(n.wrapping_add(0xc), n);
    g.write::<u32>(n.wrapping_add(0x14), op(img, f, o.parent.0));
    g.write::<u32>(n.wrapping_add(0x4), op(img, f, o.parent.1));
    let last = g.read::<u32>(op(img, f, o.last.0));
    g.write::<u32>(n.wrapping_add(0x8), last);
    let last = g.read::<u32>(op(img, f, o.last.1));
    if last == op(img, f, o.empty) {
        g.write::<u32>(op(img, f, o.first), n);
    } else {
        g.write::<u32>(last.wrapping_add(4), n);
    }
    let count = op(img, f, o.count);
    let c = g.read::<u16>(count).wrapping_add(1);
    g.write::<u16>(count, c);
    g.write::<u32>(op(img, f, o.tail), n);
    last
}

/// A node with its handler table (operand `+0x2`) put last in the DLL's
/// list ([`link_last`]), `+0x24` 0 (internal, `ecx` = the node). Returns
/// the old last.
///
/// `magic_0152` 0x100083e0 (84 bytes; 3 DLLs).
pub fn link_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    g.write::<u32>(n, op(img, f, 0x2));
    let o = LinkOps {
        parent: (0x15, 0x1c),
        last: (0x21, 0x29),
        empty: 0x2e,
        first: 0x36,
        count: 0x42,
        tail: 0x48,
    };
    let last = link_last(g, img, f, n, &o);
    g.write::<u32>(n.wrapping_add(0x24), 0);
    Ret::eax(last)
}

/// [`link_node`].
pub const LINK_NODE: Sig = Sig {
    len: 84,
    hash: 0xe350_0b0d_1ffd_5846,
    calls: &[],
    imports: &[],
};

/// [`link_node`]'s list with the handler table (operand `+0x5`) first and
/// `+0x1c` = x (internal, `ecx` = the node, cdecl `(x)`). Returns x.
///
/// `magic_0042` 0x10008360 (87 bytes; 3 DLLs).
pub fn link_node_x<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    g.write::<u32>(n, op(img, f, 0x5));
    let o = LinkOps {
        parent: (0x18, 0x1f),
        last: (0x24, 0x2c),
        empty: 0x31,
        first: 0x39,
        count: 0x48,
        tail: 0x4e,
    };
    link_last(g, img, f, n, &o);
    let x = fr.arg(g, 0);
    g.write::<u32>(n.wrapping_add(0x1c), x);
    Ret::eax(x)
}

/// [`link_node_x`].
pub const LINK_NODE_X: Sig = Sig {
    len: 87,
    hash: 0xd545_f20d_880b_86ff,
    calls: &[],
    imports: &[],
};

/// The node put last in the DLL's list ([`link_last`]), `+0x38` = x, then
/// its handler table (operand `+0x51`) (internal, `ecx` = the node, cdecl
/// `(x)`). Returns x.
///
/// `magic_0262` 0x10006cc0 (87 bytes; 2 DLLs).
pub fn link_node_38<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    let o = LinkOps {
        parent: (0x12, 0x19),
        last: (0x1e, 0x26),
        empty: 0x2b,
        first: 0x33,
        count: 0x3f,
        tail: 0x48,
    };
    link_last(g, img, f, n, &o);
    let x = fr.arg(g, 0);
    g.write::<u32>(n.wrapping_add(0x38), x);
    g.write::<u32>(n, op(img, f, 0x51));
    Ret::eax(x)
}

/// [`link_node_38`].
pub const LINK_NODE_38: Sig = Sig {
    len: 87,
    hash: 0xcc54_1a2c_3700_9ebf,
    calls: &[],
    imports: &[],
};

/// The node put last in the DLL's list ([`link_last`]), `+0x2c` = x,
/// `+0x30` = y, then its handler table (operand `+0x57`) (internal, `ecx`
/// = the node, cdecl `(x, y)`). Returns y.
///
/// `magic_0042` 0x100081a0 (93 bytes; 3 DLLs).
pub fn link_node_xy<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    let o = LinkOps {
        parent: (0x12, 0x19),
        last: (0x1e, 0x26),
        empty: 0x2b,
        first: 0x33,
        count: 0x3f,
        tail: 0x48,
    };
    link_last(g, img, f, n, &o);
    let (x, y) = (fr.arg(g, 0), fr.arg(g, 1));
    g.write::<u32>(n.wrapping_add(0x2c), x);
    g.write::<u32>(n.wrapping_add(0x30), y);
    g.write::<u32>(n, op(img, f, 0x57));
    Ret::eax(y)
}

/// [`link_node_xy`].
pub const LINK_NODE_XY: Sig = Sig {
    len: 93,
    hash: 0xf26b_d506_299d_67d1,
    calls: &[],
    imports: &[],
};

/// The object's `+0x180..+0x188` cleared and `+0x18c` the code's float
/// (operand `+0x3c`), slot 450 `(obj, callback)` (operand `+0x13`), then
/// slot 451 `(obj, n)` with the code's 8-bit `n` (operand `+0x48`; the
/// first call's words stay under the second's; cdecl `(obj)`). Returns
/// slot 451's `eax`.
///
/// `magic_0191` 0x10001b50 (-0.16, 6) and 0x10001940 (-0.08, 12) (88
/// bytes; 3 DLLs each).
pub fn drift_and_count<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let cb = op(img, f, 0x13);
    for off in [0x180, 0x184, 0x188] {
        g.write::<u32>(obj.wrapping_add(off), 0);
    }
    g.write::<u32>(obj.wrapping_add(0x18c), op(img, f, 0x3c));
    slot_call(g, img, f, (0x4, 450), &[obj, cb]);
    through(slot_call(
        g,
        img,
        f,
        (0x43, 451),
        &[obj, imm8(img, f, 0x48), obj, cb],
    ))
}

/// [`drift_and_count`] with -0.16 and 6.
pub const DRIFT_AND_6: Sig = Sig {
    len: 88,
    hash: 0x8b9a_8385_557d_608e,
    calls: &[],
    imports: &[],
};

/// [`drift_and_count`] with -0.08 and 12.
pub const DRIFT_AND_12: Sig = Sig {
    len: 88,
    hash: 0x1d59_4f2c_551b_23fd,
    calls: &[],
    imports: &[],
};

/// A header (`ecx`: 0x5c bytes cleared, then `+0x0` 100, `+0x4` the
/// records, `+0x8`, `+0x10`, `+0x18` 0) and its 100 records of 0x94 bytes
/// (`edx`; each cleared, its first word 0 again) (internal; `memset`).
/// Returns the last record (`memset`'s result).
///
/// `magic_0376` 0x10003a30 (88 bytes; 2 DLLs).
pub fn records_clear_100<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (head, mut rec) = (fr.ecx, fr.edx);
    g.write_bytes(head, &[0; 0x5c]);
    g.write::<u32>(head, 0x64);
    g.write::<u32>(head.wrapping_add(4), rec);
    for off in [0x8, 0x10, 0x18] {
        g.write::<u32>(head.wrapping_add(off), 0);
    }
    let mut last = rec;
    for _ in 0..0x64 {
        g.write_bytes(rec, &[0; 0x94]);
        g.write::<u32>(rec, 0);
        last = rec;
        rec = rec.wrapping_add(0x94);
    }
    Ret::eax(last)
}

/// [`records_clear_100`].
pub const RECORDS_CLEAR_100: Sig = Sig {
    len: 88,
    hash: 0xe9b6_3fa8_86fc_318a,
    calls: &[
        (0xc, Callee::Import("memset")),
        (0x3f, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The 320 part handles of `this` registered: slot 772 `(this, i)` gives
/// a 64-bit handle, slot 779 `(handle, this, i)` registers it with
/// `**G_LOCK` 1 around the call (internal, `ecx` = this; an aligned
/// frame). Returns the lock's address.
///
/// `magic_0511` 0x10005c80 (86 bytes; 2 DLLs).
pub fn register_320<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let this = fr.ecx;
    let mut lock = 0;
    for i in 0..0x140u32 {
        let h = slot_call(g, img, f, (0x11, 772), &[this, i]);
        lock = g.read::<u32>(op(img, f, 0x21));
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        slot_call(g, img, f, (0x2a, 779), &[h.eax, h.edx, this, i]);
        lock = g.read::<u32>(op(img, f, 0x3d));
        g.write::<u32>(lock, saved);
    }
    Ret::eax(lock)
}

/// [`register_320`].
pub const REGISTER_320: Sig = Sig {
    len: 86,
    hash: 0x6b3e_fcd5_eb1a_b152,
    calls: &[],
    imports: &[],
};

/// The objects run ([`run_objects`], an aligned frame); when they are all
/// done, `G_WAIT` not above 0 and `G_BUSY` 0: the task's state 3 and next
/// function (operand `+0x32`), everything drawn ([`draw_all`]) and the
/// block's parts registered ([`register_80`] on `*G + 0x260`) (cdecl
/// `(task)`). Returns the registration's result, or the objects'.
///
/// `magic_0132` 0x10001930 (80 bytes; 4 DLLs).
pub fn run_draw_register<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; and esp, -8; call: the callees' return address
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(4);
    let b = run_objects(g, img, img.rel32_target(f + 0x7), esp);
    if b != 1 || g.read::<i32>(op(img, f, 0x12)) > 0 || g.read::<u32>(op(img, f, 0x1b)) != 0 {
        return Ret::eax(b);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), op(img, f, 0x32));
    let eax = draw_all(g, img, img.rel32_target(f + 0x37), rec);
    let block = g.read::<u32>(op(img, f, 0x3d));
    let callee = Frame {
        at: img.rel32_target(f + 0x48),
        esp,
        eax,
        ecx: block.wrapping_add(0x260),
        ..*fr
    };
    register_80(g, img, &callee)
}

/// [`run_draw_register`].
pub const RUN_DRAW_REGISTER: Sig = Sig {
    len: 80,
    hash: 0x662e_af87_de80_fb6a,
    calls: &[
        (0x7, Callee::Sig(&RUN_OBJECTS)),
        (0x37, Callee::Sig(&DRAW_ALL)),
        (0x48, Callee::Sig(&REGISTER_80)),
    ],
    imports: &[],
};

/// Slot 447 `(x, n)` with `x` the word `*G + 0x198` and the code's `n`
/// (operand `+0x16`), slot 450 `(obj, callback)` and slot 449 `(obj,
/// callback 2)` (operands `+0x32`, `+0x3f`), each seeing the words the
/// earlier ones left; then the object's entry `obj + 0x2c + 16 * k` (`k` =
/// `obj + 0xac`) gets `x` (cdecl `(obj)`). Returns `2k`.
///
/// `magic_0357` 0x10001280 (n 0xb4), `magic_0382` 0x100011c0 (0xba),
/// `magic_0421` 0x10001420 (0xc2), `magic_0415` 0x100010a0 (0xc0) and
/// `magic_0312` 0x10001400 (0xb3) (95 bytes; 2-3 DLLs each).
pub fn motion_entry<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let base = g.read::<u32>(op(img, f, 0x4));
    let x = g.read::<u32>(base.wrapping_add(0x198));
    let n = op(img, f, 0x16);
    slot_call(g, img, f, (0x11, 447), &[x, n]);
    let obj = fr.arg(g, 0);
    let (cb, cb2) = (op(img, f, 0x32), op(img, f, 0x3f));
    slot_call(g, img, f, (0x24, 450), &[obj, cb, x, n]);
    slot_call(g, img, f, (0x3a, 449), &[obj, cb2, obj, cb, x, n]);
    let k2 = g.read::<u32>(obj.wrapping_add(0xac)).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(k2.wrapping_mul(8)).wrapping_add(0x2c), x);
    Ret::eax(k2)
}

/// [`motion_entry`] with 0xb4.
pub const MOTION_ENTRY_B4: Sig = Sig {
    len: 95,
    hash: 0x50da_201f_18a1_40c4,
    calls: &[],
    imports: &[],
};

/// [`motion_entry`] with 0xba.
pub const MOTION_ENTRY_BA: Sig = Sig {
    len: 95,
    hash: 0xc25c_9664_e95b_1532,
    calls: &[],
    imports: &[],
};

/// [`motion_entry`] with 0xc2.
pub const MOTION_ENTRY_C2: Sig = Sig {
    len: 95,
    hash: 0xc20c_46e5_b3bc_094a,
    calls: &[],
    imports: &[],
};

/// [`motion_entry`] with 0xc0.
pub const MOTION_ENTRY_C0: Sig = Sig {
    len: 95,
    hash: 0x2617_85cd_7080_7588,
    calls: &[],
    imports: &[],
};

/// [`motion_entry`] with 0xb3.
pub const MOTION_ENTRY_B3: Sig = Sig {
    len: 95,
    hash: 0xd757_2159_4c20_34d1,
    calls: &[],
    imports: &[],
};

/// The node put last in the DLL's list ([`link_last`]), `+0x38` = x,
/// `+0x28` 1.0, then its handler table (operand `+0x58`) (internal, `ecx`
/// = the node, cdecl `(x)`). Returns x.
///
/// `magic_0106` 0x10006b10 (94 bytes; 2 DLLs).
pub fn link_node_38_one<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    let o = LinkOps {
        parent: (0x12, 0x19),
        last: (0x1e, 0x26),
        empty: 0x2b,
        first: 0x33,
        count: 0x3f,
        tail: 0x48,
    };
    link_last(g, img, f, n, &o);
    let x = fr.arg(g, 0);
    g.write::<u32>(n.wrapping_add(0x38), x);
    g.write::<u32>(n.wrapping_add(0x28), 0x3f80_0000);
    g.write::<u32>(n, op(img, f, 0x58));
    Ret::eax(x)
}

/// [`link_node_38_one`].
pub const LINK_NODE_38_ONE: Sig = Sig {
    len: 94,
    hash: 0x4dbd_17fd_595b_2008,
    calls: &[],
    imports: &[],
};

/// The node with its handler table (operand `+0x5`) put last in the DLL's
/// list ([`link_last`]), `+0x1c` = x, `+0x20` 0xff (internal, `ecx` = the
/// node, cdecl `(x)`). Returns x.
///
/// `magic_0273` 0x100076d0 (94 bytes; 2 DLLs).
pub fn link_node_x_ff<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    g.write::<u32>(n, op(img, f, 0x5));
    let o = LinkOps {
        parent: (0x18, 0x1f),
        last: (0x24, 0x2c),
        empty: 0x31,
        first: 0x39,
        count: 0x45,
        tail: 0x4e,
    };
    link_last(g, img, f, n, &o);
    let x = fr.arg(g, 0);
    g.write::<u32>(n.wrapping_add(0x1c), x);
    g.write::<u32>(n.wrapping_add(0x20), 0xff);
    Ret::eax(x)
}

/// [`link_node_x_ff`].
pub const LINK_NODE_X_FF: Sig = Sig {
    len: 94,
    hash: 0x669a_5caa_6681_b372,
    calls: &[],
    imports: &[],
};

/// The node put last in the DLL's list ([`link_last`]), `+0x2c` the code's
/// word (operand `+0x49`), `+0x28` 1.0, `+0x34` 0, then its handler table
/// (operand `+0x5d`) (internal, `ecx` = the node). Returns the old last.
///
/// `magic_0609` 0x10008dc0 (98 bytes; 2 DLLs).
pub fn link_node_2c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    let o = LinkOps {
        parent: (0xf, 0x16),
        last: (0x1b, 0x23),
        empty: 0x28,
        first: 0x30,
        count: 0x3c,
        tail: 0x42,
    };
    let last = link_last(g, img, f, n, &o);
    g.write::<u32>(n.wrapping_add(0x2c), op(img, f, 0x49));
    g.write::<u32>(n.wrapping_add(0x28), 0x3f80_0000);
    g.write::<u32>(n.wrapping_add(0x34), 0);
    g.write::<u32>(n, op(img, f, 0x5d));
    Ret::eax(last)
}

/// [`link_node_2c`].
pub const LINK_NODE_2C: Sig = Sig {
    len: 98,
    hash: 0xa4ed_b216_fb9c_a6e3,
    calls: &[],
    imports: &[],
};

/// Slot 719 `(*G, 4, 0)` gives the part's handle (`G_H`, operand
/// `+0x26`); the header (operand `+0x21`) and records (`*G_RECS`) cleared
/// ([`records_clear_100`]); `G_A`, `G_B`, `G_C` 0 (no arguments).
/// Returns 1.
///
/// `magic_0376` 0x10001ca0 (86 bytes; 2 DLLs).
pub fn records_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let x = g.read::<u32>(op(img, f, 0x11));
    let h = slot_call(g, img, f, (0x1, 719), &[x, 4, 0]).eax;
    let recs = g.read::<u32>(op(img, f, 0x19));
    g.write::<u32>(op(img, f, 0x26), h);
    let callee = Frame {
        at: img.rel32_target(f + 0x2b),
        // add esp, 8 leaves one of slot 719's words; then the call
        esp: fr.esp.wrapping_sub(8),
        eax: h,
        ecx: op(img, f, 0x21),
        edx: recs,
        ..*fr
    };
    records_clear_100(g, img, &callee);
    for o in [0x34, 0x3e, 0x48] {
        g.write::<u32>(op(img, f, o), 0);
    }
    Ret::eax(1)
}

/// [`records_start`].
pub const RECORDS_START: Sig = Sig {
    len: 86,
    hash: 0xffda_0842_370e_825d,
    calls: &[(0x2b, Callee::Sig(&RECORDS_CLEAR_100))],
    imports: &[],
};

/// Slot 415 `(*G)` gives the motion; 0xb1/0xb2 then make slot 228 `(*G,
/// 0xb2)`, 0xb9/0xba `(*G, 0xba)`, anything else nothing (no arguments).
/// Returns 1.
///
/// `magic_0530` 0x100010a0 (90 bytes; 2 DLLs).
pub fn motion_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let x = g.read::<u32>(op(img, f, 0x7));
    let next = match slot_call(g, img, f, (0x1, 415), &[x]).eax {
        0xb1 | 0xb2 => 0xb2,
        0xb9 | 0xba => 0xba,
        _ => return Ret::eax(1),
    };
    let x = g.read::<u32>(op(img, f, 0x45));
    slot_call(g, img, f, (0x3f, 228), &[x, next]);
    Ret::eax(1)
}

/// [`motion_pair`].
pub const MOTION_PAIR: Sig = Sig {
    len: 90,
    hash: 0x6247_e8e7_bcc9_3e5d,
    calls: &[],
    imports: &[],
};

/// The object's drift `+0x18c` = `K / (n - n / 4)` (`K` operand `+0x7`,
/// `n` the word `G_N`), slot 450 `(obj, callback)` (operand `+0x3a`),
/// then slot 451 `(obj, n - n / 4)` (the first call's words stay under
/// the second's; cdecl `(obj)`). Returns slot 451's `eax`.
///
/// `magic_0384` 0x10002000 (95 bytes; 3 DLLs).
pub fn drift_by_count<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let k = f32::from_bits(g.read::<u32>(op(img, f, 0x7)));
    let n = g.read::<i32>(op(img, f, 0xf));
    let m = n.wrapping_sub(n / 4).cast_unsigned();
    let drift = crate::sb_sse::div(k, crate::sb_sse::cvt(m));
    let obj = fr.arg(g, 0);
    let cb = op(img, f, 0x3a);
    g.write::<u32>(obj.wrapping_add(0x18c), drift.to_bits());
    slot_call(g, img, f, (0x28, 450), &[obj, cb]);
    through(slot_call(g, img, f, (0x4a, 451), &[obj, m, obj, cb]))
}

/// [`drift_by_count`].
pub const DRIFT_BY_COUNT: Sig = Sig {
    len: 95,
    hash: 0x3e52_9be5_9c28_34b0,
    calls: &[],
    imports: &[],
};

/// Slot 434 `(*G, callback)` twice (callbacks at `+0x9`, `+0x21`; the
/// first call's words stay under the second's); the second's record gets
/// the word `G_X` at its entry `+0x2c + 16 * k` (`k` = `+0xac`); then
/// slot 461 `(obj, callback 3)` (operand `+0x42`; cdecl `(obj)`). Returns
/// slot 461's `eax`.
///
/// `magic_0415` 0x10001140 (95 bytes; 2 DLLs).
pub fn entry_434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let x = g.read::<u32>(op(img, f, 0xf));
    let cb = op(img, f, 0x9);
    slot_call(g, img, f, (0x4, 434), &[x, cb]);
    let y = g.read::<u32>(op(img, f, 0x27));
    let cb2 = op(img, f, 0x21);
    let rec = slot_call(g, img, f, (0x1c, 434), &[y, cb2, x, cb]).eax;
    let k2 = g.read::<u32>(rec.wrapping_add(0xac)).wrapping_mul(2);
    let v = g.read::<u32>(op(img, f, 0x3b));
    let obj = fr.arg(g, 0);
    g.write::<u32>(rec.wrapping_add(k2.wrapping_mul(8)).wrapping_add(0x2c), v);
    let cb3 = op(img, f, 0x42);
    through(slot_call(
        g,
        img,
        f,
        (0x4e, 461),
        &[obj, cb3, y, cb2, x, cb],
    ))
}

/// [`entry_434`].
pub const ENTRY_434: Sig = Sig {
    len: 95,
    hash: 0x7e75_14bf_64b6_298b,
    calls: &[],
    imports: &[],
};

/// The light ranges of the actor `T[k]` (`k` the byte `*G + 2`, `T`
/// operand `+0x12`): slots 357, 356, 355 and 354 `(m, -1, ..)` (357 and
/// 356 with 0x3540 and 0x3bd0; each call seeing the words the earlier
/// ones left; no arguments). Returns slot 354's `eax`.
///
/// `magic_0430` 0x10003880 (96 bytes; 2 DLLs).
pub fn ranges_four<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = g.read::<u32>(op(img, f, 0x1));
    let k = u32::from(g.read::<u8>(p.wrapping_add(2)));
    let m = g.read::<u32>(op(img, f, 0x12).wrapping_add(4 * k));
    let all = u32::MAX;
    let mut words = vec![m, all, 0x3540];
    slot_call(g, img, f, (0x17, 357), &words);
    words.splice(0..0, [m, all, 0x3bd0]);
    slot_call(g, img, f, (0x27, 356), &words);
    words.splice(0..0, [m, all]);
    slot_call(g, img, f, (0x3c, 355), &words);
    words.splice(0..0, [m, all]);
    // call width: 10 (the earlier calls' words stay on the stack)
    through(slot_call(g, img, f, (0x4c, 354), &words))
}

/// [`ranges_four`].
pub const RANGES_FOUR: Sig = Sig {
    len: 96,
    hash: 0x02e4_e338_28a7_3bf1,
    calls: &[],
    imports: &[],
};

/// Slot 447 `(x, 0xb3)` with `x` the word `*G + 0x198`, slot 434 `(*G,
/// callback)` (operand `+0x26`), slot 444 `(0)` and slot 461 `(obj,
/// callback 2)` (operand `+0x4d`), each seeing the words the earlier ones
/// left (cdecl `(obj)`). Returns slot 461's `eax`.
///
/// `magic_0356` 0x100010f0 (97 bytes; 2 DLLs).
pub fn chain_461<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let base = g.read::<u32>(op(img, f, 0x4));
    let x = g.read::<u32>(base.wrapping_add(0x198));
    slot_call(g, img, f, (0x14, 447), &[x, 0xb3]);
    let y = g.read::<u32>(op(img, f, 0x2c));
    let cb = op(img, f, 0x26);
    slot_call(g, img, f, (0x21, 434), &[y, cb, x, 0xb3]);
    slot_call(g, img, f, (0x39, 444), &[0, y, cb, x, 0xb3]);
    let obj = fr.arg(g, 0);
    let cb2 = op(img, f, 0x4d);
    through(slot_call(
        g,
        img,
        f,
        (0x48, 461),
        &[obj, cb2, 0, y, cb, x, 0xb3],
    ))
}

/// [`chain_461`].
pub const CHAIN_461: Sig = Sig {
    len: 97,
    hash: 0x666e_b7b6_83d1_5ee7,
    calls: &[],
    imports: &[],
};

/// From the speed `G_S` (operand `+0x7`): `G_A` = `S * K3` and `G_B`
/// = `S * K1 / K2` (operands `+0x1a`, `+0x12`, `+0x22`; stored at `+0x37`,
/// `+0x3f`); slot 450 `(obj, callback)` (operand `+0x2c`), then slot 451
/// `(obj, 18)` (the first call's words stay under the second's; cdecl
/// `(obj)`). Returns slot 451's `eax`.
///
/// `magic_0312` 0x10001740 (98 bytes; 2 DLLs).
pub fn speeds_count<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let k = |g: &G, o: u32| f32::from_bits(g.read::<u32>(op(img, f, o)));
    let s = k(g, 0x7);
    let b = crate::sb_sse::div(crate::sb_sse::mul(s, k(g, 0x12)), k(g, 0x22));
    let a = crate::sb_sse::mul(s, k(g, 0x1a));
    let obj = fr.arg(g, 0);
    let cb = op(img, f, 0x2c);
    g.write::<u32>(op(img, f, 0x37), a.to_bits());
    g.write::<u32>(op(img, f, 0x3f), b.to_bits());
    slot_call(g, img, f, (0x27, 450), &[obj, cb]);
    through(slot_call(g, img, f, (0x4c, 451), &[obj, 0x12, obj, cb]))
}

/// [`speeds_count`].
pub const SPEEDS_COUNT: Sig = Sig {
    len: 98,
    hash: 0x4d8a_74b5_0688_e378,
    calls: &[],
    imports: &[],
};

/// The node walk ([`crate::nodes::walk`], `ecx` = the cursor); when it
/// ran, slot 275 `(owner + 0x1c)` with that word (internal, `ecx` = the
/// cursor). Returns slot 275's `eax`, or the walk's.
///
/// `magic_0070` 0x10005000 (97 bytes; 3 DLLs).
pub fn walk_275<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let cursor = fr.ecx;
    let (eax, _) = crate::nodes::walk(g, cursor, cursor, fr.eax);
    if g.read::<u8>(cursor + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let owner = g.read::<u32>(cursor);
    let x = g.read::<u32>(owner.wrapping_add(0x1c));
    through(slot_call(g, img, fr.at, (0x4f, 275), &[x]))
}

/// [`walk_275`].
pub const WALK_275: Sig = Sig {
    len: 97,
    hash: 0x2e69_41c5_08c5_55a0,
    calls: &[],
    imports: &[],
};

/// The DLL's handles registered: the counted ones
/// ([`crate::sa_misc::register_counted`]), the parts
/// ([`crate::s4_misc::register_parts_b`]), the records
/// ([`crate::sa_misc::register_records`]), then handle `(0x203, 0)` of
/// slot 772 through slot 779 with `**G_LOCK` 1 around it (no arguments;
/// an aligned frame). Returns 0.
///
/// `magic_0515` 0x100049d0 (89 bytes; 4 DLLs).
pub fn register_all<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let eax = crate::sa_misc::register_counted(g, img, img.rel32_target(f + 0x9), fr.eax);
    // push ebp; and esp, -8; push ecx; push esi; call
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(12);
    let callee = Frame {
        at: img.rel32_target(f + 0xe),
        esp,
        eax,
        ..*fr
    };
    crate::s4_misc::register_parts_b(g, img, &callee);
    crate::sa_misc::register_records(g, img, img.rel32_target(f + 0x13));
    let h = slot_call(g, img, f, (0x18, 772), &[0x203, 0]);
    let lock = g.read::<u32>(op(img, f, 0x2d));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    slot_call(g, img, f, (0x36, 779), &[h.eax, h.edx, 0x203, 0]);
    let lock = g.read::<u32>(op(img, f, 0x49));
    g.write::<u32>(lock, saved);
    Ret::eax(0)
}

/// [`register_all`].
pub const REGISTER_ALL: Sig = Sig {
    len: 89,
    hash: 0xe2f8_3d9c_3cf7_6c25,
    calls: &[
        (0x9, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0xe, Callee::Sig(&crate::s4_misc::REGISTER_PARTS_B)),
        (0x13, Callee::Sig(&crate::sa_misc::REGISTER_RECORDS)),
    ],
    imports: &[],
};

/// A task's object (`task + 4`) done: while its `+0x18` is 0 and its flag
/// bit 2 (byte `+0x2a8`) is set, its three nodes `+0x168`, `+0x194`,
/// `+0x1c` leave their parents ([`crate::sa_misc::leave`]), the task's
/// node leaves its ring, and `+0x18` becomes 20000 (cdecl `(task)`).
/// Returns the ring's last link read, or `eax` as it came.
///
/// `magic_0388` 0x10006b40 (93 bytes; 2 DLLs).
pub fn object_done<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let task = fr.arg(g, 0);
    let obj = g.read::<u32>(task.wrapping_add(4));
    if g.read::<u32>(obj.wrapping_add(0x18)) != 0 || g.read::<u8>(obj.wrapping_add(0x2a8)) & 4 == 0
    {
        return Ret::eax(fr.eax);
    }
    let mut eax = fr.eax;
    for off in [0x168, 0x194, 0x1c] {
        eax = crate::sa_misc::leave(g, obj.wrapping_add(off), eax);
    }
    let node = g.read::<u32>(task);
    let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
    g.write::<u32>(prev + 8, next);
    let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(obj.wrapping_add(0x18), 0x4e20);
    Ret::eax(prev)
}

/// [`object_done`].
pub const OBJECT_DONE: Sig = Sig {
    len: 93,
    hash: 0x064f_9e8b_7341_9fca,
    calls: &[
        (0x21, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x2c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x34, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// The objects run ([`run_objects`]); the first time they are all done
/// (`G_DONE` 0) slot 239 `(*G, -5, 2)` and `G_DONE` 1; then the motion
/// steps ([`crate::s3_motion::motion_step`]); when both are done the task
/// `t` (cdecl `(t)`) gets state 3 and its record the next function
/// (operand `+0x57`). Returns the record, or the motion's result.
///
/// `magic_0247` 0x100017a0 (94 bytes; 2 DLLs).
pub fn run_once_motion<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; push esi; call: the run's return address
    let a = run_objects(g, img, img.rel32_target(f + 0x5), fr.esp.wrapping_sub(12));
    if a == 1 && g.read::<u32>(op(img, f, 0x12)) == 0 {
        let x = g.read::<u32>(op(img, f, 0x2a));
        slot_call(g, img, f, (0x1a, 239), &[x, (-5i32).cast_unsigned(), 2]);
        g.write::<u32>(op(img, f, 0x35), a);
    }
    let b = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x3a));
    if b != 1 || a != 1 {
        return Ret::eax(b);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), op(img, f, 0x57));
    Ret::eax(rec)
}

/// [`run_once_motion`].
pub const RUN_ONCE_MOTION: Sig = Sig {
    len: 94,
    hash: 0xa3d6_1bc4_af2c_772e,
    calls: &[
        (0x5, Callee::Sig(&RUN_OBJECTS)),
        (0x3a, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
    ],
    imports: &[],
};

/// The node put last in the DLL's list ([`link_last`]), `+0x1c` = x,
/// `+0x20` = y, `+0x24` 0x80808080 (a grey), then its handler table
/// (operand `+0x5e`) (internal, `ecx` = the node, cdecl `(x, y)`).
/// Returns y.
///
/// `magic_0042` 0x10009420 (100 bytes; 3 DLLs).
pub fn link_node_grey<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    let o = LinkOps {
        parent: (0x12, 0x19),
        last: (0x1e, 0x26),
        empty: 0x2b,
        first: 0x33,
        count: 0x3f,
        tail: 0x48,
    };
    link_last(g, img, f, n, &o);
    let (x, y) = (fr.arg(g, 0), fr.arg(g, 1));
    g.write::<u32>(n.wrapping_add(0x1c), x);
    g.write::<u32>(n.wrapping_add(0x20), y);
    g.write::<u32>(n.wrapping_add(0x24), 0x8080_8080);
    g.write::<u32>(n, op(img, f, 0x5e));
    Ret::eax(y)
}

/// [`link_node_grey`].
pub const LINK_NODE_GREY: Sig = Sig {
    len: 100,
    hash: 0x03bb_1cf1_56b9_5527,
    calls: &[],
    imports: &[],
};

/// The node with its handler table (operand `+0x2`) put last in the DLL's
/// list ([`link_last`]), then `+0x70`, `+0x6c` two of the DLL's blocks
/// (operands `+0x4f`, `+0x56`), byte `+0x74` 0 and `+0x68` 2 (internal,
/// `ecx` = the node). Returns the old last.
///
/// `magic_0577` 0x10008f10 (102 bytes; 2 DLLs).
pub fn link_node_blocks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    g.write::<u32>(n, op(img, f, 0x2));
    let o = LinkOps {
        parent: (0x15, 0x1c),
        last: (0x21, 0x29),
        empty: 0x2e,
        first: 0x36,
        count: 0x42,
        tail: 0x48,
    };
    let last = link_last(g, img, f, n, &o);
    g.write::<u32>(n.wrapping_add(0x70), op(img, f, 0x4f));
    g.write::<u32>(n.wrapping_add(0x6c), op(img, f, 0x56));
    g.write::<u8>(n.wrapping_add(0x74), 0);
    g.write::<u32>(n.wrapping_add(0x68), 2);
    Ret::eax(last)
}

/// [`link_node_blocks`].
pub const LINK_NODE_BLOCKS: Sig = Sig {
    len: 102,
    hash: 0x88c0_3c69_4933_9959,
    calls: &[],
    imports: &[],
};

/// Slots 4 `(s)` and 27 `(s, 1)` for the states 0x301a, 0x301b and
/// 0x301c, each call seeing the words the earlier ones left (no
/// arguments). Returns the last slot's `eax`.
///
/// `magic_0511` 0x100021d0 (100 bytes; 3 DLLs).
pub fn states_301a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let mut words: Vec<u32> = Vec::new();
    let mut ret = Ret::eax(fr.eax);
    for (k, s) in (0u32..).zip([0x301a, 0x301b, 0x301c]) {
        words.insert(0, s);
        // call width: 1..7 (the earlier calls' words stay on the stack)
        slot_call(g, img, f, (0x1 + 0x20 * k, 4), &words);
        words.splice(0..0, [s, 1]);
        // call width: 3..9 (the earlier calls' words stay on the stack)
        ret = through(slot_call(g, img, f, (0x10 + 0x20 * k, 27), &words));
    }
    ret
}

/// [`states_301a`].
pub const STATES_301A: Sig = Sig {
    len: 100,
    hash: 0x1e91_811c_fb2a_9206,
    calls: &[],
    imports: &[],
};

/// Unless the object is flagged (bit 0 of `+0x90`): slot 217 `(model,
/// obj + 0x1c)` (the model `+0x50`), `v + 0xc` 0, slot 248 `(obj + 0x2c,
/// obj + 0x1c, v)` (the first call's words stay under the second's),
/// bit 3 of `+0x90` and `+0x74` = `xmm2` (internal, `ecx` = obj, `edx` =
/// v, `xmm2`). Returns slot 248's `eax` and `xmm2` in `xmm0`, or `eax`
/// as it came.
///
/// `magic_0262` 0x10006b60 (99 bytes; 2 DLLs).
pub fn model_place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let (obj, v) = (fr.ecx, fr.edx);
    if g.read::<u8>(obj.wrapping_add(0x90)) & 1 != 0 {
        return Ret::eax(fr.eax);
    }
    let r = fr.xmm_f32(2).to_bits();
    let model = g.read::<u32>(obj.wrapping_add(0x50));
    let pos = obj.wrapping_add(0x1c);
    slot_call(g, img, f, (0x19, 217), &[model, pos]);
    g.write::<u32>(v.wrapping_add(0xc), 0);
    let eax = slot_call(
        g,
        img,
        f,
        (0x34, 248),
        &[obj.wrapping_add(0x2c), pos, v, model, pos],
    )
    .eax;
    let flags = g.read::<u16>(obj.wrapping_add(0x90));
    g.write::<u16>(obj.wrapping_add(0x90), flags | 8);
    g.write::<u32>(obj.wrapping_add(0x74), r);
    Ret::xmm0(crate::Xmm::F32(r), eax)
}

/// [`model_place`].
pub const MODEL_PLACE: Sig = Sig {
    len: 99,
    hash: 0x366d_8b2c_55c8_f5e6,
    calls: &[],
    imports: &[],
};

/// The block's task (`*G + 0xeac`): `+0xec0` 1, a task node for the
/// callback (operand `+0x8`; [`take`], kind 1) appended to the DLL's task
/// list (operands `+0x1d`, `+0x31`, `+0x44`; `+0x5a`, `+0x5f` when it was
/// empty) (no arguments). Returns the node.
///
/// `magic_0430` 0x100032b0 (101 bytes; 2 DLLs).
pub fn block_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(op(img, f, 0x3));
    g.write::<u32>(block.wrapping_add(0xec0), 1);
    let node = take(g, img, img.rel32_target(f + 0x17), op(img, f, 0x8));
    let empty = g.read::<u32>(op(img, f, 0x1d)) == 0;
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(block.wrapping_add(0xeac), node);
        g.write::<u32>(op(img, f, 0x5a), node);
        g.write::<u32>(op(img, f, 0x5f), node);
    } else {
        let tail = g.read::<u32>(op(img, f, 0x31));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(node.wrapping_add(8), node);
        g.write::<u32>(block.wrapping_add(0xeac), node);
        g.write::<u32>(op(img, f, 0x44), node);
    }
    Ret::eax(node)
}

/// [`block_task`].
pub const BLOCK_TASK: Sig = Sig {
    len: 101,
    hash: 0x3c24_00e1_75bb_ac9c,
    calls: &[(0x17, Callee::Sig(&TAKE))],
    imports: &[],
};

/// The DLL's static node set up in place (operands from `+0x7` on: its
/// handler table, callback and owner, the list's last `*G_LAST` before
/// it and the list's head, `+0x4` of the old last), the list's flag bit 0
/// cleared, the node the new last; `+0x1c` = `edx`, `+0x2c`, `+0x18` 0
/// (internal, `edx`). Returns the old last.
///
/// `magic_0155` 0x10009500 (101 bytes; 2 DLLs).
pub fn static_node_link<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let last = static_link_head(g, img, f);
    g.write::<u32>(op(img, f, 0x4c), fr.edx);
    g.write::<u32>(op(img, f, 0x52), 0);
    g.write::<u32>(op(img, f, 0x5c), 0);
    Ret::eax(last)
}

/// The common start of [`static_node_link`] and [`static_node_link_b`]:
/// the static node's handler table, callback and owner, the list's last
/// before it and the list's head, the old last's `+0x4`, the list's flag
/// bit 0 cleared and the node the new last. Returns the old last.
fn static_link_head<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let last = g.read::<u32>(op(img, f, 0x1));
    for (dst, src) in [(0x7, 0xb), (0x11, 0x15), (0x1b, 0x1f)] {
        g.write::<u32>(op(img, f, dst), op(img, f, src));
    }
    g.write::<u32>(op(img, f, 0x24), last);
    g.write::<u32>(op(img, f, 0x2a), op(img, f, 0x2e));
    g.write::<u32>(last.wrapping_add(4), op(img, f, 0x35));
    let flags = op(img, f, 0x3b);
    let v = g.read::<u32>(flags);
    g.write::<u32>(flags, v & !1);
    g.write::<u32>(op(img, f, 0x42), op(img, f, 0x46));
    last
}

/// [`static_node_link`].
pub const STATIC_NODE_LINK: Sig = Sig {
    len: 101,
    hash: 0x20d7_6212_3538_6d83,
    calls: &[],
    imports: &[],
};

/// The counters `G_A`, `G_B` the code's `n` (operands `+0x7`/`+0xb`,
/// `+0x11`/`+0x15`), `G_C` 0, and a task node for the callback (operand
/// `+0x1`; [`take`]) with `+0x14` the DLL's block (operand `+0x32`)
/// appended to the task list (operands `+0x2a`, `+0x41`, `+0x4b`; `+0x5a`,
/// `+0x5f` when it was empty) (no arguments). Returns the node.
///
/// `magic_0249` 0x10002fc0 (n 24), `magic_0270` 0x100041e0 (n 16) and
/// `magic_0419` 0x10002900 (n 20) (103 bytes; 2 DLLs each).
pub fn counted_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    for (dst, src) in [(0x7, 0xb), (0x11, 0x15)] {
        g.write::<u32>(op(img, f, dst), op(img, f, src));
    }
    g.write::<u32>(op(img, f, 0x1b), 0);
    let node = take(g, img, img.rel32_target(f + 0x24), op(img, f, 0x1));
    let empty = g.read::<u32>(op(img, f, 0x2a)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), op(img, f, 0x32));
    g.write::<u32>(node.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(op(img, f, 0x5a), node);
        g.write::<u32>(op(img, f, 0x5f), node);
    } else {
        let tail = g.read::<u32>(op(img, f, 0x41));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(op(img, f, 0x4b), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`counted_task`] with 24.
pub const COUNTED_TASK_24: Sig = Sig {
    len: 103,
    hash: 0xccaf_6cfd_1ea0_6079,
    calls: &[(0x24, Callee::Sig(&TAKE))],
    imports: &[],
};

/// [`counted_task`] with 16.
pub const COUNTED_TASK_16: Sig = Sig {
    len: 103,
    hash: 0x8915_2ebd_458c_9fb9,
    calls: &[(0x24, Callee::Sig(&TAKE))],
    imports: &[],
};

/// [`counted_task`] with 20.
pub const COUNTED_TASK_20: Sig = Sig {
    len: 103,
    hash: 0xe0b2_5e5d_4ae8_05e9,
    calls: &[(0x24, Callee::Sig(&TAKE))],
    imports: &[],
};

/// The four floats at `edx` copied into two of the DLL's vectors (the
/// first `+0x8`/`+0x14`, then each next pair 0xd bytes on in the code)
/// (internal, `edx`). Returns the last float in `xmm0`, `eax` as it came.
///
/// `magic_0070` 0x100093e0 (103 bytes; 2 DLLs).
pub fn two_vectors<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let mut x = 0;
    for (i, (a, b)) in (0u32..).zip([(0x8, 0x14), (0x21, 0x2e), (0x3b, 0x48), (0x55, 0x62)]) {
        x = g.read::<u32>(fr.edx.wrapping_add(4 * i));
        g.write::<u32>(op(img, f, a), x);
        x = g.read::<u32>(fr.edx.wrapping_add(4 * i));
        g.write::<u32>(op(img, f, b), x);
    }
    Ret::xmm0(crate::Xmm::F32(x), fr.eax)
}

/// [`two_vectors`].
pub const TWO_VECTORS: Sig = Sig {
    len: 103,
    hash: 0x60e4_6907_b1e5_d5d6,
    calls: &[],
    imports: &[],
};

/// The state's `+0x25c..+0x268` cleared and slot 325 `(state + 0xc,
/// state + 0x24c)`; then bit 0 of `u16 +0x284` cleared and `+0x26c`,
/// `+0x270` 0 (internal, `ecx` = the state). Returns 0xfffe.
///
/// `magic_0180` 0x10002ad0 (104 bytes; 2 DLLs).
pub fn state_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let s = fr.ecx;
    for off in [0x25c, 0x260, 0x264, 0x268] {
        g.write::<u32>(s.wrapping_add(off), 0);
    }
    slot_call(
        g,
        img,
        fr.at,
        (0xf, 325),
        &[s.wrapping_add(0xc), s.wrapping_add(0x24c)],
    );
    let flags = g.read::<u16>(s.wrapping_add(0x284));
    g.write::<u16>(s.wrapping_add(0x284), flags & 0xfffe);
    g.write::<u32>(s.wrapping_add(0x26c), 0);
    g.write::<u32>(s.wrapping_add(0x270), 0);
    Ret::eax(0xfffe)
}

/// [`state_reset`].
pub const STATE_RESET: Sig = Sig {
    len: 104,
    hash: 0xf321_d8ba_93a4_b18f,
    calls: &[],
    imports: &[],
};

/// [`static_node_link`] with the node's three last words 0 (operands
/// `+0x4c`, `+0x56`, `+0x60`; no arguments). Returns the old last.
///
/// `magic_0152` 0x100099b0 (105 bytes; 3 DLLs).
pub fn static_node_link_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let last = static_link_head(g, img, f);
    for o in [0x4c, 0x56, 0x60] {
        g.write::<u32>(op(img, f, o), 0);
    }
    Ret::eax(last)
}

/// [`static_node_link_b`].
pub const STATIC_NODE_LINK_B: Sig = Sig {
    len: 105,
    hash: 0xd2e5_328d_7606_8b89,
    calls: &[],
    imports: &[],
};

/// A fresh node (`ecx`) under `parent` (`edx`, when not 0: `+0x4`, `+0x8`
/// the parent's last child, which links to it, or the parent's first
/// when it had none; the parent's count up and its last the node); then
/// `+0x2c` the DLL's block (operand `+0x3b`), `+0x30` = y, `+0x34..+0x3c`
/// 0, `u16 +0x4c` and byte `+0x4e` 0 and the handler table (operand
/// `+0x63`) (internal, cdecl `(x, y)`). Returns y.
///
/// `magic_0150` 0x1000cc70 (105 bytes; 2 DLLs).
pub fn child_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let (n, parent) = (fr.ecx, fr.edx);
    g.write::<u16>(n.wrapping_add(0x1a), 0xff);
    g.write::<u32>(n.wrapping_add(0x10), n);
    g.write::<u32>(n.wrapping_add(0xc), n);
    g.write::<u32>(n.wrapping_add(0x14), parent);
    if parent != 0 {
        g.write::<u32>(n.wrapping_add(4), parent);
        let last = g.read::<u32>(parent.wrapping_add(0x10));
        g.write::<u32>(n.wrapping_add(8), last);
        let last = g.read::<u32>(parent.wrapping_add(0x10));
        if last == parent {
            g.write::<u32>(parent.wrapping_add(0xc), n);
        } else {
            g.write::<u32>(last.wrapping_add(4), n);
        }
        let c = g.read::<u16>(parent.wrapping_add(0x18)).wrapping_add(1);
        g.write::<u16>(parent.wrapping_add(0x18), c);
        g.write::<u32>(parent.wrapping_add(0x10), n);
    }
    let y = fr.arg(g, 1);
    g.write::<u32>(n.wrapping_add(0x2c), op(img, f, 0x3b));
    g.write::<u32>(n.wrapping_add(0x30), y);
    for off in [0x34, 0x38, 0x3c] {
        g.write::<u32>(n.wrapping_add(off), 0);
    }
    g.write::<u16>(n.wrapping_add(0x4c), 0);
    g.write::<u8>(n.wrapping_add(0x4e), 0);
    g.write::<u32>(n, op(img, f, 0x63));
    Ret::eax(y)
}

/// [`child_node`].
pub const CHILD_NODE: Sig = Sig {
    len: 105,
    hash: 0x8807_da50_516c_e8de,
    calls: &[],
    imports: &[],
};

/// The block's sparkle (`*G`): with `+0xd748` set, slot 219 `(*G_P, 0)`
/// gives a part and slot 257 `(part, 1)` shows it (the first call's words
/// stay under the second's), `+0xd74c` 1; else `+0xd74c` 12. Then
/// `+0xd740`, `+0xd744`, `+0xd750` 0 (no arguments). Returns slot 257's
/// `eax`, or `eax` as it came.
///
/// `magic_0446` 0x10003ad0 (107 bytes; 2 DLLs).
pub fn sparkle_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(op(img, f, 0x3));
    let mut ret = Ret::eax(fr.eax);
    let v = if g.read::<u32>(block.wrapping_add(0xd748)) == 0 {
        0xc
    } else {
        let x = g.read::<u32>(op(img, f, 0x21));
        let part = slot_call(g, img, f, (0x19, 219), &[x, 0]).eax;
        ret = through(slot_call(g, img, f, (0x35, 257), &[part, 1, x, 0]));
        1
    };
    g.write::<u32>(block.wrapping_add(0xd74c), v);
    for off in [0xd740, 0xd744, 0xd750] {
        g.write::<u32>(block.wrapping_add(off), 0);
    }
    ret
}

/// [`sparkle_reset`].
pub const SPARKLE_RESET: Sig = Sig {
    len: 107,
    hash: 0x20e9_5b22_5481_b38b,
    calls: &[],
    imports: &[],
};

/// Slot 0 `(0x3016)` gives a handle `h` (`G_H`, operand `+0x31`), slot 26
/// `(0x3016, 1)`, byte `G_B` 0x20, slot 10 `(h, 0)`, `G_S` 3, then slot
/// 257 `(T[k], 1)` (`k` the byte `*G + 2`, `T` operand `+0x55`), each
/// call seeing the words the earlier ones left (no arguments). Returns
/// slot 257's `eax`.
///
/// `magic_0430` 0x10002210 (107 bytes; 2 DLLs).
pub fn handle_3016<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = 0x3016;
    let h = slot_call(g, img, f, (0x1, 0), &[s]).eax;
    slot_call(g, img, f, (0x11, 26), &[s, 1, s]);
    g.write::<u8>(op(img, f, 0x2a), 0x20);
    g.write::<u32>(op(img, f, 0x31), h);
    slot_call(g, img, f, (0x24, 10), &[h, 0, s, 1, s]);
    let p = g.read::<u32>(op(img, f, 0x3e));
    g.write::<u32>(op(img, f, 0x44), 3);
    let k = u32::from(g.read::<u8>(p.wrapping_add(2)));
    let part = g.read::<u32>(op(img, f, 0x55).wrapping_add(4 * k));
    through(slot_call(g, img, f, (0x5a, 257), &[part, 1, h, 0, s, 1, s]))
}

/// [`handle_3016`].
pub const HANDLE_3016: Sig = Sig {
    len: 107,
    hash: 0xe3f1_5f5f_2f81_d5e1,
    calls: &[],
    imports: &[],
};

/// A move's set-up (internal, `ecx` = the move, `xmm2`, `xmm3`): flag bit
/// 1 of `+0x68` cleared, `+0x4c` = `xmm2`, `+0x54` = `xmm3`, the deltas
/// `+0x3c..+0x48` = `+0x2c..+0x38` minus `+0x1c..+0x28` and `+0x64` =
/// `+0x60 - +0x5c`, flag bit 0 set, `+0x58` and `+0x50` 0. Returns the
/// last delta in `xmm0`, `eax` as it came.
///
/// `magic_0070` 0x10009310 (108 bytes; 3 DLLs).
pub fn move_setup<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let m = fr.ecx;
    let at = |o: u32| m.wrapping_add(o);
    let fl = |g: &G, o: u32| f32::from_bits(g.read::<u32>(m.wrapping_add(o)));
    let flags = g.read::<u32>(at(0x68));
    g.write::<u32>(at(0x68), flags & !2);
    g.write::<u32>(at(0x4c), fr.xmm_f32(2).to_bits());
    g.write::<u32>(at(0x54), fr.xmm_f32(3).to_bits());
    for i in 0..4 {
        let d = crate::sb_sse::sub(fl(g, 0x2c + 4 * i), fl(g, 0x1c + 4 * i));
        g.write::<u32>(at(0x3c + 4 * i), d.to_bits());
    }
    let d = crate::sb_sse::sub(fl(g, 0x60), fl(g, 0x5c));
    let flags = g.read::<u32>(at(0x68));
    g.write::<u32>(at(0x68), flags | 1);
    g.write::<u32>(at(0x58), 0);
    g.write::<u32>(at(0x50), 0);
    g.write::<u32>(at(0x64), d.to_bits());
    Ret::xmm0(crate::Xmm::F32(d.to_bits()), fr.eax)
}

/// [`move_setup`].
pub const MOVE_SETUP: Sig = Sig {
    len: 108,
    hash: 0x4581_107b_7979_518e,
    calls: &[],
    imports: &[],
};

/// The DLL's static node (`G_NODE`'s parent, operand `+0x2`) loses it:
/// when there is a parent, its count `u16 +0x18` drops and the node's
/// neighbours (`G_PREV`, `G_NEXT`) are joined (the parent's `+0x10` or
/// `+0xc` when the neighbour is the parent itself); the node's links go
/// back to itself and `G_NODE` 0; then a tail jump to slot 846 (no
/// arguments).
///
/// `magic_0070` 0x100096e0 (108 bytes; 2 DLLs).
pub fn static_leave_846<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    static_leave(g, img, f, 0);
    Ret::jump(slot_target(g, op(img, f, 0x60), 846))
}

/// The static node leaving its parent (see [`static_leave_846`]); the
/// operands sit `shift` bytes later than in `magic_0070`'s copy.
fn static_leave<G: Game>(g: &mut G, img: &Image, f: u32, shift: u32) {
    let o = |off: u32| op(img, f, off + shift);
    let parent = g.read::<u32>(o(0x2));
    if parent == 0 {
        return;
    }
    let c = g
        .read::<u16>(parent.wrapping_add(0x18))
        .wrapping_add(0xffff);
    g.write::<u16>(parent.wrapping_add(0x18), c);
    let (prev, next) = (g.read::<u32>(o(0x15)), g.read::<u32>(o(0x1a)));
    let at = if prev == parent {
        parent.wrapping_add(0x10)
    } else {
        prev.wrapping_add(8)
    };
    g.write::<u32>(at, next);
    let (next, prev) = (g.read::<u32>(o(0x2c)), g.read::<u32>(o(0x31)));
    let at = if next == parent {
        parent.wrapping_add(0xc)
    } else {
        next.wrapping_add(4)
    };
    g.write::<u32>(at, prev);
    g.write::<u32>(o(0x43), o(0x47));
    g.write::<u32>(o(0x4d), o(0x51));
    g.write::<u32>(o(0x57), 0);
}

/// [`static_leave_846`].
pub const STATIC_LEAVE_846: Sig = Sig {
    len: 108,
    hash: 0x4d09_27d8_4dde_d6b2,
    calls: &[],
    imports: &[],
};

/// The node with its handler table (operand `+0x5`) put last in the DLL's
/// list ([`link_last`]), `+0x20` = a, `+0x30..+0x35` 0, `+0x1c` the
/// code's word (operand `+0x64`), `+0x38` = d (internal, `ecx` = the
/// node, cdecl `(a, b, c, d)`). Returns d.
///
/// `magic_0042` 0x100082f0 (109 bytes; 3 DLLs).
pub fn link_node_ad<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    g.write::<u32>(n, op(img, f, 0x5));
    let o = LinkOps {
        parent: (0x18, 0x1f),
        last: (0x24, 0x2c),
        empty: 0x31,
        first: 0x39,
        count: 0x45,
        tail: 0x4e,
    };
    link_last(g, img, f, n, &o);
    g.write::<u32>(n.wrapping_add(0x20), fr.arg(g, 0));
    g.write::<u32>(n.wrapping_add(0x32), 0);
    g.write::<u16>(n.wrapping_add(0x30), 0);
    let d = fr.arg(g, 3);
    g.write::<u32>(n.wrapping_add(0x1c), op(img, f, 0x64));
    g.write::<u32>(n.wrapping_add(0x38), d);
    Ret::eax(d)
}

/// [`link_node_ad`].
pub const LINK_NODE_AD: Sig = Sig {
    len: 109,
    hash: 0x720f_d8e8_d6db_5c42,
    calls: &[],
    imports: &[],
};

/// A key of a part's track (internal, `ecx` = the cursor: `+0x0` the
/// track's owner, `+0x4` the output vector, `+0x8` the time): slot 941
/// maps the owner's `+0x30` to the track (none: 0 returned); the key
/// index is `time / step` (the owner's `+0x40` record's `+0x20`,
/// `cvttss2si`), capped at the track's count (`+0x18`, a float,
/// `cvttss2si`), where `+0xc` becomes 1 (else 0); the key's three words
/// (48-byte keys from `+0x40`) go to the vector. Returns the key's third
/// word.
///
/// `magic_0155` 0x10008060 (114 bytes; 4 DLLs).
pub fn track_key<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = fr.ecx;
    let owner = g.read::<u32>(c);
    let step = g.read::<u32>(g.read::<u32>(owner.wrapping_add(0x40)).wrapping_add(0x20));
    let time = f32::from_bits(g.read::<u32>(c.wrapping_add(8)));
    let i = trunc(crate::sb_sse::div(time, f32::from_bits(step))).cast_signed();
    let x = g.read::<u32>(owner.wrapping_add(0x30));
    let track = slot_call(g, img, fr.at, (0x17, 941), &[x]).eax;
    if track == 0 {
        return Ret::eax(0);
    }
    let n = trunc(f32::from_bits(g.read::<u32>(track.wrapping_add(0x18)))).cast_signed();
    let keys = track.wrapping_add(0x40);
    let (k, end) = if i < n { (i, 0) } else { (n, 1) };
    let key = keys.wrapping_add(k.cast_unsigned().wrapping_mul(0x30));
    g.write::<u32>(c.wrapping_add(0xc), end);
    let out = g.read::<u32>(c.wrapping_add(4));
    let mut last = 0;
    for j in 0..3 {
        last = g.read::<u32>(key.wrapping_add(4 * j));
        g.write::<u32>(out.wrapping_add(4 * j), last);
    }
    Ret::eax(last)
}

/// [`track_key`].
pub const TRACK_KEY: Sig = Sig {
    len: 114,
    hash: 0x7051_a126_e362_5f73,
    calls: &[],
    imports: &[],
};

/// The light ranges of the model `*G` (operand `+0x8`): slot 357 `(m,
/// -1, 0x3c00)`, slot 356 `(m, -1, 0x2ee0)`, slot 362 `(*G_H, m, 1, 0)`
/// (operand `+0x43`), slots 357 and 356 `(m, -1, -1)`, each call seeing
/// the words the earlier ones left (no arguments). Returns slot 356's
/// `eax`.
///
/// `magic_0348` 0x10006790 (114 bytes; 2 DLLs).
pub fn model_ranges<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let m = g.read::<u32>(op(img, f, 0x8));
    let all = u32::MAX;
    let mut words = vec![m, all, 0x3c00];
    slot_call(g, img, f, (0x1, 357), &words);
    words.splice(0..0, [m, all, 0x2ee0]);
    slot_call(g, img, f, (0x1d, 356), &words);
    let h = g.read::<u32>(op(img, f, 0x43));
    words.splice(0..0, [h, m, 1, 0]);
    slot_call(g, img, f, (0x32, 362), &words);
    words.splice(0..0, [m, all, all]);
    // call width: 13 (the earlier calls' words stay on the stack)
    slot_call(g, img, f, (0x4a, 357), &words);
    words.splice(0..0, [m, all, all]);
    // call width: 16 (the earlier calls' words stay on the stack)
    through(slot_call(g, img, f, (0x5c, 356), &words))
}

/// [`model_ranges`].
pub const MODEL_RANGES: Sig = Sig {
    len: 114,
    hash: 0x9284_7d9e_2fd2_15a4,
    calls: &[],
    imports: &[],
};

/// Slot 933 `(1.0, 1.0, 1.0, 1.0)`, `G_A` 1.0 (operand `+0x47`), then
/// slot 785 twice with the float `G_K` (operands `+0x38`, `+0x5f`) (no
/// arguments). Returns the last slot's `eax`.
///
/// `magic_0390` 0x10001530 (114 bytes; 2 DLLs).
pub fn colour_one<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let one = 0x3f80_0000;
    slot_call(g, img, f, (0x1, 933), &[one; 4]);
    let k = g.read::<u32>(op(img, f, 0x38));
    g.write::<u32>(op(img, f, 0x47), one);
    slot_call(g, img, f, (0x30, 785), &[k]);
    let k = g.read::<u32>(op(img, f, 0x5f));
    through(slot_call(g, img, f, (0x57, 785), &[k]))
}

/// [`colour_one`].
pub const COLOUR_ONE: Sig = Sig {
    len: 114,
    hash: 0x764a_84ce_27b0_5351,
    calls: &[],
    imports: &[],
};

/// [`static_node_link`] with the node's four last words 0 (operands
/// `+0x4c`, `+0x56`, `+0x60`, `+0x6a`; no arguments). Returns the old
/// last.
///
/// `magic_0374` 0x10007d30 (115 bytes; 2 DLLs).
pub fn static_node_link_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let last = static_link_head(g, img, f);
    for o in [0x4c, 0x56, 0x60, 0x6a] {
        g.write::<u32>(op(img, f, o), 0);
    }
    Ret::eax(last)
}

/// [`static_node_link_c`].
pub const STATIC_NODE_LINK_C: Sig = Sig {
    len: 115,
    hash: 0x0cb4_e0e0_304d_17a9,
    calls: &[],
    imports: &[],
};

/// The DLL's static node (operand `+0x2`, its handler table `+0x6`) set
/// up fresh under `parent` (`edx`, when not 0, it goes last among the
/// parent's children: the old last, or the parent's first, links to it
/// and the count goes up); then byte `G_A` and `G_B` 0 (internal).
/// Returns the old last, or `eax` as it came without a parent.
///
/// `magic_0513` 0x10006a10 (115 bytes; 2 DLLs).
pub fn static_child<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let parent = fr.edx;
    g.write::<u32>(op(img, f, 0x2), op(img, f, 0x6));
    g.write::<u16>(op(img, f, 0xd), 0xff);
    g.write::<u32>(op(img, f, 0x15), op(img, f, 0x19));
    g.write::<u32>(op(img, f, 0x1f), op(img, f, 0x23));
    g.write::<u32>(op(img, f, 0x29), parent);
    let mut eax = fr.eax;
    if parent != 0 {
        g.write::<u32>(op(img, f, 0x33), parent);
        let last = g.read::<u32>(parent.wrapping_add(0x10));
        g.write::<u32>(op(img, f, 0x3b), last);
        let last = g.read::<u32>(parent.wrapping_add(0x10));
        if last == parent {
            g.write::<u32>(parent.wrapping_add(0xc), op(img, f, 0x49));
        } else {
            g.write::<u32>(last.wrapping_add(4), op(img, f, 0x52));
        }
        let c = g.read::<u16>(parent.wrapping_add(0x18)).wrapping_add(1);
        g.write::<u16>(parent.wrapping_add(0x18), c);
        g.write::<u32>(parent.wrapping_add(0x10), op(img, f, 0x5d));
        eax = last;
    }
    g.write::<u8>(op(img, f, 0x63), 0);
    g.write::<u32>(op(img, f, 0x6a), 0);
    Ret::eax(eax)
}

/// [`static_child`].
pub const STATIC_CHILD: Sig = Sig {
    len: 115,
    hash: 0xd3a2_8f41_8f4b_07c7,
    calls: &[],
    imports: &[],
};

/// A strip cursor's window (internal, `ecx` = the cursor, `+0x0` the
/// strip: `u16 +0x20` its length, `+0x22` the stride, `+0x26` the
/// window, `+0x28` the start, `+0x1c` the data): `+0x8` the start's data;
/// when start + window passes the length (signed `u16`), `u16 +0x4` the
/// part up to the end and `+0x6` the rest, `+0xc` the data; else `+0x4`
/// the window, `+0x6` 0 and `+0xc` the window's end. Returns the data, or
/// the stride.
///
/// `magic_0150` 0x10006f40 (115 bytes; 2 DLLs).
pub fn strip_window<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let c = fr.ecx;
    let u16_at = |g: &G, p: u32, o: u32| u32::from(g.read::<u16>(p.wrapping_add(o)));
    let p = g.read::<u32>(c);
    let data = g.read::<u32>(p.wrapping_add(0x1c));
    let start = u16_at(g, p, 0x22)
        .wrapping_mul(u16_at(g, p, 0x28))
        .wrapping_add(data);
    g.write::<u32>(c.wrapping_add(8), start);
    let p = g.read::<u32>(c);
    let (from, window) = (u16_at(g, p, 0x28), u16_at(g, p, 0x26));
    let len = g.read::<u16>(p.wrapping_add(0x20));
    let end = window.wrapping_add(from) & 0xffff;
    if end.cast_signed() > i32::from(len.cast_signed()) {
        let head = u32::from(len).wrapping_sub(from);
        let [h0, h1, _, _] = head.to_le_bytes();
        let head16 = u16::from_le_bytes([h0, h1]);
        g.write::<u16>(c.wrapping_add(4), head16);
        let p = g.read::<u32>(c);
        let rest = g.read::<u16>(p.wrapping_add(0x26)).wrapping_sub(head16);
        g.write::<u16>(c.wrapping_add(6), rest);
        let p = g.read::<u32>(c);
        let data = g.read::<u32>(p.wrapping_add(0x1c));
        g.write::<u32>(c.wrapping_add(0xc), data);
        return Ret::eax(data);
    }
    let [w0, w1, _, _] = window.to_le_bytes();
    g.write::<u16>(c.wrapping_add(4), u16::from_le_bytes([w0, w1]));
    g.write::<u16>(c.wrapping_add(6), 0);
    let p = g.read::<u32>(c);
    let stride = u16_at(g, p, 0x22);
    let tail = u16_at(g, p, 0x26)
        .wrapping_mul(stride)
        .wrapping_add(g.read::<u32>(p.wrapping_add(0x1c)));
    g.write::<u32>(c.wrapping_add(0xc), tail);
    Ret::eax(stride)
}

/// [`strip_window`].
pub const STRIP_WINDOW: Sig = Sig {
    len: 115,
    hash: 0xb3aa_337e_5d82_bec5,
    calls: &[],
    imports: &[],
};

/// A fader's state reset (internal, `ecx` = the state: `+0x0`, `+0x1c`,
/// `+0x24` 0, `+0x20` the step and `+0x28` the count, the code's
/// immediates at `+0x1a`, `+0x28`, `u16 +0x2c` 0) and a task node
/// for the callback (operand `+0x6`; [`take`], kind 1, `+0x14` the state)
/// appended to the task list (operands `+0x37`, `+0x4f`, `+0x59`;
/// `+0x68`, `+0x6d` when it was empty). Returns the node.
///
/// `magic_0168` 0x10007680 (1/30, 30; 3 DLLs), `magic_0162` 0x10007c90
/// (0.025, 40) and `magic_0392` 0x10006580 (0.02, 50) (117 bytes; 2
/// DLLs each).
pub fn fader_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = fr.ecx;
    let (step, count) = (op(img, f, 0x1a), op(img, f, 0x28));
    for (off, v) in [(0x0, 0), (0x1c, 0), (0x20, step), (0x24, 0), (0x28, count)] {
        g.write::<u32>(s.wrapping_add(off), v);
    }
    g.write::<u16>(s.wrapping_add(0x2c), 0);
    let node = take(g, img, img.rel32_target(f + 0x31), op(img, f, 0x6));
    let empty = g.read::<u32>(op(img, f, 0x37)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), s);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(op(img, f, 0x68), node);
        g.write::<u32>(op(img, f, 0x6d), node);
    } else {
        let tail = g.read::<u32>(op(img, f, 0x4f));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(op(img, f, 0x59), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`fader_task`].
pub const FADER_TASK: Sig = Sig {
    len: 117,
    hash: 0xfcb8_c95f_ab2e_a737,
    calls: &[(0x31, Callee::Sig(&TAKE))],
    imports: &[],
};

/// [`fader_task`] with 0.025 and 40.
pub const FADER_TASK_40: Sig = Sig {
    len: 117,
    hash: 0xe699_da99_acdf_6e10,
    calls: &[(0x31, Callee::Sig(&TAKE))],
    imports: &[],
};

/// [`fader_task`] with 0.02 and 50.
pub const FADER_TASK_50: Sig = Sig {
    len: 117,
    hash: 0x3ff1_7735_63b1_3235,
    calls: &[(0x31, Callee::Sig(&TAKE))],
    imports: &[],
};

/// The DLL's record header (operand `+0x11`; 0x5c bytes cleared, then
/// the count, the records `*G` and `+0x8`, `+0x10`, `+0x18` 0) and its
/// records of 0x94 bytes (each cleared, its first word 0 again; the
/// count the code's, operand `+0x4c`) (no arguments; `memset`). Returns
/// the last record (`memset`'s result).
///
/// `magic_0206` 0x10002cb0 (500 records) and `magic_0371` 0x10002a00
/// (1000) (117 bytes; 2-3 DLLs).
pub fn records_clear<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let mut rec = g.read::<u32>(op(img, f, 0x7));
    g.write_bytes(op(img, f, 0x11), &[0; 0x5c]);
    g.write::<u32>(op(img, f, 0x1f), op(img, f, 0x23));
    g.write::<u32>(op(img, f, 0x29), rec);
    for o in [0x2f, 0x39, 0x43] {
        g.write::<u32>(op(img, f, o), 0);
    }
    let mut last = rec;
    for _ in 0..op(img, f, 0x4c) {
        g.write_bytes(rec, &[0; 0x94]);
        g.write::<u32>(rec, 0);
        last = rec;
        rec = rec.wrapping_add(0x94);
    }
    Ret::eax(last)
}

/// [`records_clear`] with 500.
pub const RECORDS_CLEAR_500: Sig = Sig {
    len: 117,
    hash: 0x6aa3_b3cc_b7a8_627c,
    calls: &[
        (0x16, Callee::Import("memset")),
        (0x59, Callee::Import("memset")),
    ],
    imports: &[],
};

/// [`records_clear`] with 1000.
pub const RECORDS_CLEAR_1000: Sig = Sig {
    len: 117,
    hash: 0x6128_b91d_cec3_b314,
    calls: &[
        (0x16, Callee::Import("memset")),
        (0x59, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The colour `G_C` (three floats, operands `+0x9`, `+0x13`, `+0x1d`)
/// 1.0, `u16 G_T` 0, and a task node for the callback (operand `+0x3`;
/// [`take`]) with `+0x14` the DLL's block (operand `+0x3a`) appended to
/// the task list (operands `+0x32`, `+0x49`, `+0x53`; `+0x62`, `+0x67`
/// when it was empty) (no arguments). Returns the node.
///
/// `magic_0053` 0x10002af0 (111 bytes; 4 DLLs).
pub fn colour_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    for o in [0x9, 0x13, 0x1d] {
        g.write::<u32>(op(img, f, o), 0x3f80_0000);
    }
    g.write::<u16>(op(img, f, 0x27), 0);
    let node = take(g, img, img.rel32_target(f + 0x2c), op(img, f, 0x3));
    let empty = g.read::<u32>(op(img, f, 0x32)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), op(img, f, 0x3a));
    g.write::<u32>(node.wrapping_add(4), 0);
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(op(img, f, 0x62), node);
        g.write::<u32>(op(img, f, 0x67), node);
    } else {
        let tail = g.read::<u32>(op(img, f, 0x49));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
        g.write::<u32>(op(img, f, 0x53), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`colour_task`].
pub const COLOUR_TASK: Sig = Sig {
    len: 111,
    hash: 0x6bdd_c410_0fa1_9ef6,
    calls: &[(0x2c, Callee::Sig(&TAKE))],
    imports: &[],
};

/// One step of an integer colour fade (internal, `ecx` = the fade: the
/// steps left `+0x34`, counted down while above 0, of `+0x30`): each of
/// the three channels `+0x0..+0x8` = `((n - k) * to + from * k) / n`
/// (`to` at `+0x20`, `from` at `+0x10`, signed `idiv`), `+0xc` 0x80; then
/// slot 332 `(fade)`. Returns slot 332's `eax`.
///
/// `magic_0168` 0x10001e60 (119 bytes; 3 DLLs).
pub fn int_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let s = fr.ecx;
    let int = |g: &G, o: u32| g.read::<i32>(s.wrapping_add(o));
    let k = int(g, 0x34);
    if k > 0 {
        g.write::<i32>(s.wrapping_add(0x34), k - 1);
    }
    for i in 0..3 {
        let (n, k) = (int(g, 0x30), int(g, 0x34));
        let to = int(g, 0x20 + 4 * i);
        let from = int(g, 0x10 + 4 * i);
        let mixed = n
            .wrapping_sub(k)
            .wrapping_mul(to)
            .wrapping_add(from.wrapping_mul(k));
        // idiv: a zero step count faults in the original as well
        g.write::<i32>(s.wrapping_add(4 * i), mixed.wrapping_div(n));
    }
    g.write::<u32>(s.wrapping_add(0xc), 0x80);
    through(slot_call(g, img, fr.at, (0x65, 332), &[s]))
}

/// [`int_fade`].
pub const INT_FADE: Sig = Sig {
    len: 119,
    hash: 0xd5be_90c9_f4b8_e190,
    calls: &[],
    imports: &[],
};

/// The node with its handler table (operand `+0x8`) put last in the DLL's
/// list ([`link_last`]), `+0x3c` = x, `+0x40` 0 and `+0x38` slot 942's
/// answer for 0 (internal, `ecx` = the node, cdecl `(x)`). Returns that
/// answer.
///
/// `magic_0146` 0x10006ca0 (119 bytes; 2 DLLs).
pub fn link_node_942<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (f, n) = (fr.at, fr.ecx);
    g.write::<u32>(n, op(img, f, 0x8));
    let o = LinkOps {
        parent: (0x1b, 0x22),
        last: (0x27, 0x2f),
        empty: 0x34,
        first: 0x3c,
        count: 0x48,
        tail: 0x51,
    };
    link_last(g, img, f, n, &o);
    g.write::<u32>(n.wrapping_add(0x3c), fr.arg(g, 0));
    g.write::<u32>(n.wrapping_add(0x40), 0);
    let h = slot_call(g, img, f, (0x59, 942), &[0]).eax;
    g.write::<u32>(n.wrapping_add(0x38), h);
    Ret::eax(h)
}

/// [`link_node_942`].
pub const LINK_NODE_942: Sig = Sig {
    len: 119,
    hash: 0x69c9_d6d9_d7b6_579d,
    calls: &[],
    imports: &[],
};

/// The static node leaving its parent ([`static_leave`]), then the model
/// of the node at operand `+0x69` set up ([`model_setup`], a cursor on
/// the frame) (no arguments). Returns 0.
///
/// `magic_0164` 0x1000bfc0 (120 bytes; 2 DLLs).
pub fn static_leave_model<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    static_leave(g, img, f, 4);
    // push ebp; push ecx: the cursor is [ebp - 4]
    let cursor = fr.esp.wrapping_sub(8);
    g.write::<u32>(cursor, op(img, f, 0x69));
    let callee = Frame {
        at: img.rel32_target(f + 0x6e),
        esp: cursor.wrapping_sub(4),
        ecx: cursor,
        ..*fr
    };
    model_setup(g, img, &callee);
    Ret::eax(0)
}

/// [`static_leave_model`].
pub const STATIC_LEAVE_MODEL: Sig = Sig {
    len: 120,
    hash: 0xe6e7_d5f5_03d4_7af8,
    calls: &[(0x6e, Callee::Sig(&MODEL_SETUP))],
    imports: &[],
};

/// The DLL's static node (operand `+0x7`, its handler table `+0xb`) set
/// up fresh and put last under the DLL's list (operands: the parent
/// `+0x30`, the list's last `*G_LAST` `+0x1`, its first `+0x4e`, count
/// `+0x62`, last `+0x68`), byte `+0x30` of the node 0 (no arguments).
/// Returns the old last.
///
/// `magic_0201` 0x10006e00 (120 bytes; 3 DLLs).
pub fn static_link_last<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let last = static_link_table(g, img, f);
    g.write::<u8>(op(img, f, 0x72), 0);
    Ret::eax(last)
}

/// The static node with its handler table first, put last under the
/// DLL's list: [`static_link_last`] and [`static_link_last_word`] up to
/// their last store. Returns the old last.
fn static_link_table<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let last = g.read::<u32>(op(img, f, 0x1));
    g.write::<u32>(op(img, f, 0x7), op(img, f, 0xb));
    g.write::<u16>(op(img, f, 0x12), 0xff);
    for (dst, src) in [(0x1a, 0x1e), (0x24, 0x28), (0x2e, 0x32), (0x38, 0x3c)] {
        g.write::<u32>(op(img, f, dst), op(img, f, src));
    }
    g.write::<u32>(op(img, f, 0x41), last);
    if last == op(img, f, 0x46) {
        g.write::<u32>(op(img, f, 0x4e), op(img, f, 0x52));
    } else {
        g.write::<u32>(last.wrapping_add(4), op(img, f, 0x5b));
    }
    let count = op(img, f, 0x62);
    let c = g.read::<u16>(count).wrapping_add(1);
    g.write::<u16>(count, c);
    g.write::<u32>(op(img, f, 0x68), op(img, f, 0x6c));
    last
}

/// [`static_link_last`].
pub const STATIC_LINK_LAST: Sig = Sig {
    len: 120,
    hash: 0x52b1_c517_e284_096b,
    calls: &[],
    imports: &[],
};

/// Slot 363 `(*G_A, 2, 1)`; the first time (`G_SET` 0) slot 362 `(c,
/// *G_A, 2, **G_P)` with `c` the word `G_C`, which `G_SET` keeps; then
/// `G_X`, `G_Y` 0 and `G_T`, `G_U` 0x1ccc8 (no arguments). Returns the
/// last slot's `eax`.
///
/// `magic_0188` 0x100011a0 (120 bytes; 2 DLLs).
pub fn light_once<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let a = g.read::<u32>(op(img, f, 0x11));
    let mut ret = through(slot_call(g, img, f, (0x1, 363), &[a, 2, 1]));
    if g.read::<u32>(op(img, f, 0x1c)) == 0 {
        let p = g.read::<u32>(op(img, f, 0x24));
        let c = g.read::<u32>(op(img, f, 0x2a));
        let pv = g.read::<u32>(p);
        let a = g.read::<u32>(op(img, f, 0x39));
        g.write::<u32>(op(img, f, 0x46), c);
        ret = through(slot_call(g, img, f, (0x31, 362), &[c, a, 2, pv]));
    }
    g.write::<u32>(op(img, f, 0x51), 0);
    g.write::<u32>(op(img, f, 0x5b), 0);
    g.write::<u32>(op(img, f, 0x65), 0x1_ccc8);
    g.write::<u32>(op(img, f, 0x6f), 0x1_ccc8);
    ret
}

/// [`light_once`].
pub const LIGHT_ONCE: Sig = Sig {
    len: 120,
    hash: 0x3b1c_c4d3_4d4d_1129,
    calls: &[],
    imports: &[],
};

/// The DLL's block (operand `+0x8`, 0x110 bytes) cleared, its pointer
/// `G_P` set (operands `+0x16`, `+0x1a`), the two words at `+0x20`,
/// `+0x2a` 7 and seven more (operands `+0x34` to `+0x70`) 1 (no
/// arguments; `memset`). Returns the block (`memset`'s result).
///
/// `magic_0188` 0x10003ab0 (121 bytes; 3 DLLs).
pub fn block_defaults<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = op(img, f, 0x8);
    g.write_bytes(block, &[0; 0x110]);
    g.write::<u32>(op(img, f, 0x16), op(img, f, 0x1a));
    for o in [0x20, 0x2a] {
        g.write::<u32>(op(img, f, o), 7);
    }
    for o in (0x34..=0x70).step_by(0xa) {
        g.write::<u32>(op(img, f, o), 1);
    }
    Ret::eax(block)
}

/// [`block_defaults`].
pub const BLOCK_DEFAULTS: Sig = Sig {
    len: 121,
    hash: 0x4093_732f_36c1_ac2b,
    calls: &[(0xd, Callee::Import("memset"))],
    imports: &[],
};

/// One 16-byte entry written into a ring buffer (internal, `ecx` = the
/// writer: `+0x0` the ring, `+0x4` the entry): the ring (`u16 +0x20` its
/// size, `+0x22` the stride, `+0x24` the write index, `+0x26` the fill,
/// `+0x2a` the read index, `+0x1c` the data, byte `+0x28` flags) is full
/// (bit 0) when the fill reached the size; the write index steps (back
/// to 0 at the size) and the entry goes to its slot; a full ring moves
/// the read index on (back to 0 at the size), else the fill grows; bit 1
/// cleared. Returns the read index (in the low half of the entry
/// pointer), 0, or the entry pointer.
///
/// `magic_0150` 0x1000aa80 (121 bytes; 2 DLLs).
pub fn ring_write<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let w = fr.ecx;
    let r = g.read::<u32>(w);
    let at = |o: u32| r.wrapping_add(o);
    let size = g.read::<u16>(at(0x20));
    let flags = g.read::<u8>(at(0x28));
    let flags = if g.read::<u16>(at(0x26)) == size {
        flags | 1
    } else {
        flags & 0xfe
    };
    g.write::<u8>(at(0x28), flags);
    let mut i = g.read::<u16>(at(0x24)).wrapping_add(1);
    if i == size {
        i = 0;
    }
    g.write::<u16>(at(0x24), i);
    let stride = u32::from(g.read::<u16>(at(0x22)));
    let slot = i32::from(i.cast_signed())
        .cast_unsigned()
        .wrapping_mul(stride);
    let dst = slot.wrapping_add(g.read::<u32>(at(0x1c)));
    let entry = g.read::<u32>(w.wrapping_add(4));
    for k in 0..4 {
        let x = g.read::<u32>(entry.wrapping_add(4 * k));
        g.write::<u32>(dst.wrapping_add(4 * k), x);
    }
    let eax = if g.read::<u8>(at(0x28)) & 1 == 0 {
        let n = g.read::<u16>(at(0x26)).wrapping_add(1);
        g.write::<u16>(at(0x26), n);
        entry
    } else {
        let j = g.read::<u16>(at(0x2a)).wrapping_add(1);
        g.write::<u16>(at(0x2a), j);
        if j == g.read::<u16>(at(0x20)) {
            g.write::<u16>(at(0x2a), 0);
            0
        } else {
            (entry & 0xffff_0000) | u32::from(j)
        }
    };
    let flags = g.read::<u8>(at(0x28));
    g.write::<u8>(at(0x28), flags & 0xfd);
    Ret::eax(eax)
}

/// [`ring_write`].
pub const RING_WRITE: Sig = Sig {
    len: 121,
    hash: 0x5cf7_eb1d_209c_93cb,
    calls: &[],
    imports: &[],
};

/// The DLL's static node (operands from `+0x8`) set up fresh and put
/// last under the DLL's list (its parent `+0x24`, the list's last
/// `*G_LAST` `+0x1`, first `+0x44`, count `+0x58`, last `+0x5e`), then
/// `+0x44` of the node `xmm2` (operand `+0x6a`) and its handler table
/// (`+0x70`) (internal, `xmm2`). Returns the old last.
///
/// `magic_0262` 0x10006f50 (121 bytes; 2 DLLs).
pub fn static_link_xmm<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let last = static_link_fresh(g, img, f);
    g.write::<u32>(op(img, f, 0x6a), fr.xmm_f32(2).to_bits());
    g.write::<u32>(op(img, f, 0x70), op(img, f, 0x74));
    Ret::eax(last)
}

/// The fresh static node put last under the DLL's list, the common part
/// of [`static_link_xmm`] and [`static_link_word`]. Returns the old last.
fn static_link_fresh<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let last = g.read::<u32>(op(img, f, 0x1));
    g.write::<u16>(op(img, f, 0x8), 0xff);
    for (dst, src) in [(0x10, 0x14), (0x1a, 0x1e), (0x24, 0x28), (0x2e, 0x32)] {
        g.write::<u32>(op(img, f, dst), op(img, f, src));
    }
    g.write::<u32>(op(img, f, 0x37), last);
    if last == op(img, f, 0x3c) {
        g.write::<u32>(op(img, f, 0x44), op(img, f, 0x48));
    } else {
        g.write::<u32>(last.wrapping_add(4), op(img, f, 0x51));
    }
    let count = op(img, f, 0x58);
    let c = g.read::<u16>(count).wrapping_add(1);
    g.write::<u16>(count, c);
    g.write::<u32>(op(img, f, 0x5e), op(img, f, 0x62));
    last
}

/// [`static_link_xmm`].
pub const STATIC_LINK_XMM: Sig = Sig {
    len: 121,
    hash: 0x2f8b_3236_a61a_9dc8,
    calls: &[],
    imports: &[],
};

/// While `G_DONE` is 0 and the actor `k` (the byte `*G + 1`) is below 32:
/// slot 217 `(k, v)` reads its position into a frame vector and slot 255
/// `(k, v)` takes it (the first call's words stay under the second's);
/// then slot 211 `(T[k], 0)` (`T` operand `+0x25`) (no arguments; a /GS
/// frame). Returns slot 211's `eax`.
///
/// `magic_0393` 0x10002b70 (122 bytes; 2 DLLs).
pub fn actor_place_211<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let v = fr.esp.wrapping_sub(0x18);
    let p = g.read::<u32>(op(img, f, 0x18));
    let k = u32::from(g.read::<u8>(p.wrapping_add(1)));
    let actor = g.read::<u32>(op(img, f, 0x25).wrapping_add(4 * k));
    if g.read::<u32>(op(img, f, 0x12)) == 0 && k < 0x20 {
        slot_call(g, img, f, (0x35, 217), &[k, v]);
        slot_call(g, img, f, (0x47, 255), &[k, v, k, v]);
    }
    through(slot_call(g, img, f, (0x58, 211), &[actor, 0]))
}

/// [`actor_place_211`] (the `rel32` is `__security_check_cookie`).
pub const ACTOR_PLACE_211: Sig = Sig {
    len: 122,
    hash: 0x7048_2c69_240c_76bd,
    calls: &[(0x72, Callee::Any)],
    imports: &[],
};

/// Unless the object is flagged (bit 0 of `+0x90`): slot 217 `(m, v)`
/// reads the model `+0x50`'s position into a frame vector, slot 234 `(v,
/// edx, v)` takes `edx` from it (the first call's words stay under), and
/// slot 666 `(m, angle)` turns the model to the vector's angle
/// ([`crate::m0153::vector_angle`] on `+0x0`/`+0x8`) (internal, `ecx` =
/// obj, `edx`; a /GS frame). Returns slot 666's `eax`; a flagged object
/// returns the cookie's junk, 0 here.
///
/// `magic_0201` 0x10006cb0 (122 bytes; 2 DLLs).
pub fn face_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.ecx;
    if g.read::<u8>(obj.wrapping_add(0x90)) & 1 != 0 {
        // eax holds the frame cookie (`cookie ^ ebp`): junk, 0 here
        return Ret::eax(0);
    }
    let v = fr.esp.wrapping_sub(0x18);
    let m = g.read::<u32>(obj.wrapping_add(0x50));
    slot_call(g, img, f, (0x24, 217), &[m, v]);
    slot_call(g, img, f, (0x3a, 234), &[v, fr.edx, v, m, v]);
    let angle = crate::m0153::vector_angle(g, img, img.rel32_target(f + 0x4d), v, false);
    let m = g.read::<u32>(obj.wrapping_add(0x50));
    through(slot_call(g, img, f, (0x52, 666), &[m, angle.to_bits()]))
}

/// [`face_point`] (the `rel32` at `+0x72` is `__security_check_cookie`).
pub const FACE_POINT: Sig = Sig {
    len: 122,
    hash: 0xea1c_ebd1_2e6b_a1d7,
    calls: &[
        (0x4d, Callee::Sig(&crate::m0153::VECTOR_ANGLE_XZ)),
        (0x72, Callee::Any),
    ],
    imports: &[],
};

/// [`static_link_xmm`]'s set-up with `+0x2c` of the node the code's word
/// (operand `+0x68`, imm `+0x6c`) instead of `xmm2`, then its handler
/// table (`+0x72`, imm `+0x76`) (no arguments). Returns the old last.
///
/// `magic_0155` 0x10008ff0 (123 bytes; 4 DLLs).
pub fn static_link_word<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let last = static_link_fresh(g, img, f);
    g.write::<u32>(op(img, f, 0x68), op(img, f, 0x6c));
    g.write::<u32>(op(img, f, 0x72), op(img, f, 0x76));
    Ret::eax(last)
}

/// [`static_link_word`].
pub const STATIC_LINK_WORD: Sig = Sig {
    len: 123,
    hash: 0x39fc_a7d9_582c_2460,
    calls: &[],
    imports: &[],
};

/// [`static_link_last`] ending with a word 0 (operand `+0x72`) (no
/// arguments). Returns the old last.
///
/// `magic_0070` 0x100091c0 (123 bytes; 2 DLLs).
pub fn static_link_last_word<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let last = static_link_table(g, img, f);
    g.write::<u32>(op(img, f, 0x72), 0);
    Ret::eax(last)
}

/// [`static_link_last_word`].
pub const STATIC_LINK_LAST_WORD: Sig = Sig {
    len: 123,
    hash: 0x75c4_032a_4432_bf78,
    calls: &[],
    imports: &[],
};

/// The object's place (`+0xd0`) at a bone of the actor `x` (slot 237
/// `(x, 0x10, 0, obj + 0xd0)`; `x` the word `T[G_MAP[i]]` for the
/// object's index `i` `+0x120`, or `G_NONE` for -1), `+0xdc` 1.0, its
/// height `+0xd4` the block's `+0x1a24` minus `K` (operand `+0x5d`) and
/// `+0x104` 0 (cdecl `(obj)`). Returns slot 237's `eax`.
///
/// `magic_0349` 0x10004e10 (123 bytes; 2 DLLs).
pub fn place_bone_10<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(op(img, f, 0x6));
    let obj = fr.arg(g, 0);
    let i = g.read::<u32>(obj.wrapping_add(0x120));
    let x = if i == u32::MAX {
        g.read::<u32>(op(img, f, 0x21))
    } else {
        let map = g.read::<u32>(op(img, f, 0x28));
        let k = u32::from(g.read::<u8>(map.wrapping_add(i)));
        g.read::<u32>(op(img, f, 0x33).wrapping_add(4 * k))
    };
    let out = slot_call(
        g,
        img,
        f,
        (0x3e, 237),
        &[x, 0x10, 0, obj.wrapping_add(0xd0)],
    );
    g.write::<u32>(obj.wrapping_add(0xdc), 0x3f80_0000);
    let fl = |g: &G, a: u32| f32::from_bits(g.read::<u32>(a));
    let h = crate::sb_sse::sub(fl(g, block.wrapping_add(0x1a24)), fl(g, op(img, f, 0x5d)));
    g.write::<u32>(obj.wrapping_add(0xd4), h.to_bits());
    g.write::<u32>(obj.wrapping_add(0x104), 0);
    through(out)
}

/// [`place_bone_10`].
pub const PLACE_BONE_10: Sig = Sig {
    len: 123,
    hash: 0x2cc7_6cb3_df68_1a75,
    calls: &[],
    imports: &[],
};

/// The DLL's object `i` (`*G + 0x130 i`, `ecx` = i) started
/// ([`crate::s4_misc::start_object_plain`] with the scale and part of the
/// DLL's table row `i`, seven words from operand `+0x36`), then its
/// place `+0xd0..+0xdc` from the row's first four words and `+0xc4` from
/// its sixth (internal). Returns the object.
///
/// `magic_0638` 0x100031e0 (124 bytes; 2 DLLs).
pub fn table_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let i = fr.ecx;
    let obj = i
        .wrapping_mul(0x130)
        .wrapping_add(g.read::<u32>(op(img, f, 0x18)));
    let row = op(img, f, 0x36).wrapping_add(i.wrapping_mul(0x1c));
    let part = g.read::<u32>(row.wrapping_add(0x18));
    let scale = f32::from_bits(g.read::<u32>(row.wrapping_add(0x10)));
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x2f), obj, scale, part);
    for k in 0..4 {
        let x = g.read::<u32>(row.wrapping_add(4 * k));
        g.write::<u32>(obj.wrapping_add(0xd0 + 4 * k), x);
    }
    let x = g.read::<u32>(row.wrapping_add(0x14));
    g.write::<u32>(obj.wrapping_add(0xc4), x);
    Ret::eax(obj)
}

/// [`table_object`].
pub const TABLE_OBJECT: Sig = Sig {
    len: 124,
    hash: 0x4eeb_7418_d326_1128,
    calls: &[(0x2f, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN))],
    imports: &[],
};

/// For each of the `G_N` actors `i` (`k` the byte `G_MAP[i]`; the index
/// helper's -1 case cannot come up): while `G_DONE` is 0 and `k` is below
/// 32, slot 530 `(k, -1)`; else slot 249 `(T[k], -1)`, or `(T[k], 0x100)`
/// once done (`T` operand `+0x5c`) (no arguments). Returns the last
/// slot's `eax`, or `eax` as it came.
///
/// `magic_0165` 0x10003670 (125 bytes; 2 DLLs).
pub fn actors_530<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let mut ret = Ret::eax(fr.eax);
    if g.read::<i32>(op(img, f, 0x5)) <= 0 {
        return ret;
    }
    let mut i = 0u32;
    loop {
        let map = g.read::<u32>(op(img, f, 0x12));
        let k = u32::from(g.read::<u8>(map.wrapping_add(i)));
        let done = g.read::<u32>(op(img, f, 0x1b)) != 0;
        ret = if !done && k < 0x20 {
            through(slot_call(g, img, f, (0x38, 530), &[k, u32::MAX]))
        } else {
            let x = g.read::<u32>(op(img, f, 0x5c).wrapping_add(4 * k));
            let y = if done { 0x100 } else { u32::MAX };
            through(slot_call(g, img, f, (0x63, 249), &[x, y]))
        };
        i += 1;
        if i.cast_signed() >= g.read::<i32>(op(img, f, 0x75)) {
            return ret;
        }
    }
}

/// [`actors_530`].
pub const ACTORS_530: Sig = Sig {
    len: 125,
    hash: 0xd840_ba6b_7c18_8b24,
    calls: &[],
    imports: &[],
};

/// The cursor's node leaves its parent ([`crate::nodes::leave_parent`]),
/// then slot 771 `(node)` gives its 64-bit handle and slot 779 registers
/// it with `**G_LOCK` 1 around the call (internal, `ecx` = the cursor).
/// Returns the lock's address.
///
/// `magic_0070` 0x10008850 (126 bytes; 2 DLLs).
pub fn leave_register<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let node = g.read::<u32>(fr.ecx);
    let parent = g.read::<u32>(node.wrapping_add(0x14));
    if parent != 0 {
        crate::nodes::leave_parent(g, node, parent);
    }
    let h = slot_call(g, img, f, (0x45, 771), &[node]);
    let lock = g.read::<u32>(op(img, f, 0x54));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    slot_call(g, img, f, (0x60, 779), &[h.eax, h.edx]);
    let lock = g.read::<u32>(op(img, f, 0x73));
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// [`leave_register`].
pub const LEAVE_REGISTER: Sig = Sig {
    len: 126,
    hash: 0x0ea9_049d_7b9c_56c6,
    calls: &[],
    imports: &[],
};

/// A wait `G_W` counted down; when it runs out, [`count_or_257`]'s
/// choice (slot 296 `(n, 0)` or slot 257 `(*G_PART, 1)`); then the motion
/// and the objects as in [`step_done`] (`rel32`s at `+0x53`, `+0x5a`;
/// the next function operand `+0x77`) (cdecl `(task)`). Returns the
/// record, or the objects' result.
///
/// `magic_0166` 0x100014f0 (126 bytes; 2 DLLs).
pub fn wait_then_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = g.read::<i32>(op(img, f, 0x4));
    if w > 0 {
        g.write::<i32>(op(img, f, 0x4), w - 1);
        if w - 1 <= 0 {
            let n = g.read::<u32>(op(img, f, 0x20));
            if g.read::<u32>(op(img, f, 0x18)) == 0 && n.cast_signed() < 0x20 {
                slot_call(g, img, f, (0x2d, 296), &[n, 0]);
            } else {
                let part = g.read::<u32>(op(img, f, 0x42));
                slot_call(g, img, f, (0x3a, 257), &[part, 1]);
            }
        }
    }
    let first = crate::s3_motion::motion_step;
    step_done_at(g, img, fr, first, (0x53, 0x5a), (None, 0x77))
}

/// [`wait_then_step`].
pub const WAIT_THEN_STEP: Sig = Sig {
    len: 126,
    hash: 0xf56a_9fc9_5bed_ae5d,
    calls: &[
        (0x53, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x5a, Callee::Sig(&RUN_OBJECTS)),
    ],
    imports: &[],
};

/// The camera point: slot 327 `(m, src)` fills a frame matrix from the
/// DLL's camera (operand `+0x1b`), slot 229 `(v, m, block + 0x2b30)`
/// turns the block's point into a vector (the first call's words stay
/// under), and `G_X`, `G_Z` take its `x`, `z` with `G_Y` the code's
/// height (operands `+0x58`, `+0x71`, `+0x65`/`+0x69`) (no arguments; a
/// /GS frame). Returns slot 229's `eax`.
///
/// `magic_0563` 0x100056f0 (height -50) and 0x10005770 (-10) (127
/// bytes; 2 DLLs each).
pub fn camera_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let block = g.read::<u32>(op(img, f, 0x13));
    let src = op(img, f, 0x1b);
    slot_call(g, img, f, (0x21, 327), &[m, src]);
    let eax = slot_call(
        g,
        img,
        f,
        (0x3d, 229),
        &[v, m, block.wrapping_add(0x2b30), m, src],
    )
    .eax;
    let x = g.read::<u32>(v);
    g.write::<u32>(op(img, f, 0x58), x);
    g.write::<u32>(op(img, f, 0x65), op(img, f, 0x69));
    let z = g.read::<u32>(v.wrapping_add(8));
    g.write::<u32>(op(img, f, 0x71), z);
    Ret::eax(eax)
}

/// [`camera_point`] with -50 (the `rel32` is `__security_check_cookie`).
pub const CAMERA_POINT_50: Sig = Sig {
    len: 127,
    hash: 0x32a7_cd20_b48b_bfa8,
    calls: &[(0x77, Callee::Any)],
    imports: &[],
};

/// [`camera_point`] with -10.
pub const CAMERA_POINT_10: Sig = Sig {
    len: 127,
    hash: 0x45d5_6546_efc4_d1ad,
    calls: &[(0x77, Callee::Any)],
    imports: &[],
};

/// A resource's load, step by step (no arguments): until `G_A` counts
/// (operand `+0x2`), slot 225 `(*R)` must answer 0 first (else nothing
/// more); then the resource is asked for once (slot 226 `(*R, 0)`,
/// `G_ASKED` 1) and, until `G_B` counts, slot 227 `(*R)` answering 1 or
/// more makes it count. Returns 0.
///
/// `magic_0341` 0x10001370 (129 bytes; 3 DLLs).
pub fn load_steps<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let bump = |g: &mut G, o: u32| {
        let a = op(img, f, o);
        let x = g.read::<u32>(a).wrapping_add(1);
        g.write::<u32>(a, x);
    };
    if g.read::<u32>(op(img, f, 0x2)) == 0 {
        let r = g.read::<u32>(op(img, f, 0x10));
        if slot_call(g, img, f, (0xa, 225), &[r]).eax != 0 {
            return Ret::eax(0);
        }
        bump(g, 0x25);
    }
    if g.read::<u32>(op(img, f, 0x2b)) == 0 {
        let r = g.read::<u32>(op(img, f, 0x3b));
        g.write::<u32>(op(img, f, 0x47), 1);
        slot_call(g, img, f, (0x33, 226), &[r, 0]);
    }
    if g.read::<u32>(op(img, f, 0x56)) == 0 {
        let r = g.read::<u32>(op(img, f, 0x64));
        if slot_call(g, img, f, (0x5e, 227), &[r]).eax.cast_signed() >= 1 {
            bump(g, 0x7a);
        }
    }
    Ret::eax(0)
}

/// [`load_steps`].
pub const LOAD_STEPS: Sig = Sig {
    len: 129,
    hash: 0xfbe7_c313_02cf_6702,
    calls: &[],
    imports: &[],
};

/// Slot 711 `(*G_C, callback, 0)` (operand `+0x29`) with `**G_LOCK` 1
/// around it, then slot 710 `(block, *G_C, *G_D, size)` (operands
/// `+0x4f`, `+0x11`, the code's size `+0x36`; the first call's words stay
/// under), then four words 0 (operands `+0x5a`, `+0x65`, `+0x6f`, `+0x79`)
/// (no arguments). Returns slot 710's `eax`.
///
/// `magic_0149` 0x10001cb0 (size 0x80000), `magic_0126` 0x10002100
/// (0x50000) and `magic_0243` 0x10003110 (0x100000) (131 bytes; 2-3 DLLs
/// each).
pub fn parts_buffer<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let lock = g.read::<u32>(op(img, f, 0x1));
    let c = g.read::<u32>(op(img, f, 0x7));
    let saved = g.read::<u32>(lock);
    let d = g.read::<u32>(op(img, f, 0x11));
    g.write::<u32>(lock, 1);
    let cb = op(img, f, 0x29);
    slot_call(g, img, f, (0x1c, 711), &[c, cb, 0]);
    let lock = g.read::<u32>(op(img, f, 0x31));
    g.write::<u32>(lock, saved);
    let c = g.read::<u32>(op(img, f, 0x44));
    let words = [op(img, f, 0x4f), c, d, op(img, f, 0x36), c, cb, 0];
    // call width: 7 (slot 711's words are still under these)
    let out = slot_call(g, img, f, (0x3d, 710), &words);
    for o in [0x5a, 0x65, 0x6f, 0x79] {
        g.write::<u32>(op(img, f, o), 0);
    }
    through(out)
}

/// [`parts_buffer`] with 0x80000.
pub const PARTS_BUFFER_80000: Sig = Sig {
    len: 131,
    hash: 0x6151_c901_c9fc_bfe1,
    calls: &[],
    imports: &[],
};

/// [`parts_buffer`] with 0x50000.
pub const PARTS_BUFFER_50000: Sig = Sig {
    len: 131,
    hash: 0xb9de_23c5_e276_8ede,
    calls: &[],
    imports: &[],
};

/// [`parts_buffer`] with 0x100000.
pub const PARTS_BUFFER_100000: Sig = Sig {
    len: 131,
    hash: 0xbc20_74e9_f4db_ec29,
    calls: &[],
    imports: &[],
};

/// A part dropped (internal, `ecx` = the part): slot 258 `(part)`, slot
/// 230 twice on the DLL's matrices (`(*G_A, *G_B)`, `(*G_C, *G_A)`), `*G_P`
/// = `ITEM`, slot 712 `(part, 0x30)` with `**G_LOCK` 1 around it (each
/// call seeing the words the earlier ones left); when it answers non-zero
/// the part's `u16 +0x1e` 0. Returns 0.
///
/// `magic_0481` 0x10001760 (132 bytes; 2 DLLs).
pub fn part_drop<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let part = fr.ecx;
    let rd = |g: &G, o: u32| g.read::<u32>(op(img, f, o));
    slot_call(g, img, f, (0x1, 258), &[part]);
    let (b, a) = (rd(g, 0x14), rd(g, 0x1f));
    slot_call(g, img, f, (0x19, 230), &[a, b, part]);
    let (a2, c) = (rd(g, 0x2d), rd(g, 0x38));
    slot_call(g, img, f, (0x32, 230), &[c, a2, a, b, part]);
    let p = rd(g, 0x45);
    g.write::<u32>(p, op(img, f, 0x4d));
    let lock = rd(g, 0x52);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let out = slot_call(g, img, f, (0x60, 712), &[part, 0x30, c, a2, a, b, part]);
    let lock = rd(g, 0x6e);
    g.write::<u32>(lock, saved);
    if out.eax != 0 {
        g.write::<u16>(part.wrapping_add(0x1e), 0);
    }
    Ret::eax(0)
}

/// [`part_drop`].
pub const PART_DROP: Sig = Sig {
    len: 132,
    hash: 0xdf39_1e8c_95dc_ed74,
    calls: &[],
    imports: &[],
};

/// The node walk ([`crate::nodes::walk`], `ecx` = the cursor); when it
/// ran, slot 275 `(owner + 0x20)` with that word, and with the owner's
/// bit 0 of `+0x24` its node `+0x1c` leaves its parent
/// ([`crate::sa_misc::leave`]), that node's models are set up again
/// ([`crate::s4_misc::nodes_reset`], a cursor on the frame) and the owner
/// leaves its parent (internal, `ecx` = the cursor). Returns the last
/// step's `eax`.
///
/// `magic_0201` 0x10002fe0 (135 bytes; 3 DLLs).
pub fn walk_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let cursor = fr.ecx;
    let (eax, _) = crate::nodes::walk(g, cursor, cursor, fr.eax);
    if g.read::<u8>(cursor + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let owner = g.read::<u32>(cursor);
    let x = g.read::<u32>(owner.wrapping_add(0x20));
    let eax = slot_call(g, img, f, (0x4c, 275), &[x]).eax;
    if g.read::<u8>(owner.wrapping_add(0x24)) & 1 == 0 {
        return Ret::eax(eax);
    }
    let node = g.read::<u32>(owner.wrapping_add(0x1c));
    // its result is replaced by the node (`mov eax, [esi + 0x1c]`)
    crate::sa_misc::leave(g, node, eax);
    // push ebp; push ecx; push ebx; push esi: the cursor is [ebp - 4]
    let local = fr.esp.wrapping_sub(8);
    let node = g.read::<u32>(owner.wrapping_add(0x1c));
    g.write::<u32>(local, node);
    let callee = Frame {
        at: img.rel32_target(f + 0x76),
        esp: fr.esp.wrapping_sub(0x14),
        eax: node,
        ecx: local,
        ..*fr
    };
    let eax = crate::s4_misc::nodes_reset(g, img, &callee).eax;
    Ret::eax(crate::sa_misc::leave(g, owner, eax))
}

/// [`walk_reset`].
pub const WALK_RESET: Sig = Sig {
    len: 135,
    hash: 0x9bbe_9abe_42f8_8a17,
    calls: &[
        (0x68, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x76, Callee::Sig(&crate::s4_misc::NODES_RESET)),
        (0x7d, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// [`light_once`], then `G_ON` 1.0 (operand `+0x7`) (no arguments).
/// Returns 1.
///
/// `magic_0188` 0x10001d90 (21 bytes; 2 DLLs).
pub fn light_once_on<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let callee = Frame {
        at: img.rel32_target(f + 0x1),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    light_once(g, img, &callee);
    g.write::<u32>(op(img, f, 0x7), 0x3f80_0000);
    Ret::eax(1)
}

/// [`light_once_on`].
pub const LIGHT_ONCE_ON: Sig = Sig {
    len: 21,
    hash: 0x2da0_fcee_9248_a170,
    calls: &[(0x1, Callee::Sig(&LIGHT_ONCE))],
    imports: &[],
};

/// The node walk ([`crate::nodes::walk`], `ecx` = the cursor); then the
/// owner's ring `+0x2c` gets its handler (`+0x1c` of its table) with a
/// cursor on the frame (the ring, the walk's pass byte at `+5`), and the
/// owner and the ring's `+0x1c` are written into it ([`ring_write`])
/// (internal, `ecx` = the cursor). Returns the ring write's result.
///
/// `magic_0150` 0x1000a9f0 (121 bytes; 2 DLLs).
pub fn walk_ring<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let cursor = fr.ecx;
    let owner = g.read::<u32>(cursor);
    crate::nodes::walk(g, cursor, cursor, fr.eax);
    // push ebp; sub esp, 8: the frame's cursor is [ebp - 8]
    let local = fr.esp.wrapping_sub(0xc);
    let ring = g.read::<u32>(owner.wrapping_add(0x2c));
    let pass = g.read::<u8>(cursor + 5);
    g.write::<u32>(local, ring);
    g.write::<u8>(local + 5, pass);
    let handler = g.read::<u32>(g.read::<u32>(ring).wrapping_add(0x1c));
    g.call(handler, local, &[local]);
    let entry = g.read::<u32>(owner.wrapping_add(0x2c)).wrapping_add(0x1c);
    g.write::<u32>(local, owner);
    g.write::<u32>(local + 4, entry);
    let callee = Frame {
        at: img.rel32_target(f + 0x6f),
        esp: fr.esp.wrapping_sub(0x18),
        ecx: local,
        ..*fr
    };
    ring_write(g, img, &callee)
}

/// [`walk_ring`].
pub const WALK_RING: Sig = Sig {
    len: 121,
    hash: 0x37c2_73ee_e7f7_c6cd,
    calls: &[(0x6f, Callee::Sig(&RING_WRITE))],
    imports: &[],
};

/// The first free record (first word 0) of the DLL's table (`*G_RECS`,
/// `G_N` records of 0x94 bytes) taken: its first word 1, `+0x28` 60, its
/// sprite `+0x30` set up ([`crate::sa_misc::sprite_init`] with `edx` as
/// it came), `+0x4..+0x20` cleared but `+0x10` 1.0 (internal). Returns the
/// record, or 0 when none is free.
///
/// `magic_0206` 0x10002eb0 (120 bytes; 4 DLLs).
pub fn record_take<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let n = g.read::<i32>(op(img, f, 0x2));
    let mut r = g.read::<u32>(op(img, f, 0xb));
    let mut i = 0i32;
    loop {
        if i >= n {
            return Ret::eax(0);
        }
        if g.read::<u32>(r) == 0 {
            break;
        }
        i += 1;
        r = r.wrapping_add(0x94);
    }
    g.write::<u32>(r, 1);
    g.write::<u32>(r.wrapping_add(0x28), 0x3c);
    crate::sa_misc::sprite_init(
        g,
        img,
        img.rel32_target(f + 0x38),
        r.wrapping_add(0x30),
        fr.edx,
    );
    for (off, v) in [
        (0x4, 0),
        (0x8, 0),
        (0xc, 0),
        (0x10, 0x3f80_0000),
        (0x14, 0),
        (0x18, 0),
        (0x1c, 0),
        (0x20, 0),
    ] {
        g.write::<u32>(r.wrapping_add(off), v);
    }
    Ret::eax(r)
}

/// [`record_take`].
pub const RECORD_TAKE: Sig = Sig {
    len: 120,
    hash: 0xeab0_807c_092f_0304,
    calls: &[(0x38, Callee::Sig(&crate::sa_misc::SPRITE_INIT))],
    imports: &[],
};

/// The DLL's static node (handler table first) put last under the DLL's
/// list as in [`static_link_last`], its `+0x1c` 0xff, then a tail jump to
/// slot 845 (no arguments).
///
/// `magic_0070` 0x10009650 (135 bytes; 2 DLLs).
pub fn static_link_845<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let last = g.read::<u32>(op(img, f, 0x1));
    g.write::<u32>(op(img, f, 0x7), op(img, f, 0xb));
    g.write::<u16>(op(img, f, 0x12), 0xff);
    for (dst, src) in [(0x1a, 0x1e), (0x24, 0x28), (0x2e, 0x32), (0x38, 0x3c)] {
        g.write::<u32>(op(img, f, dst), op(img, f, src));
    }
    g.write::<u32>(op(img, f, 0x41), last);
    if last == op(img, f, 0x46) {
        g.write::<u32>(op(img, f, 0x4e), op(img, f, 0x52));
    } else {
        g.write::<u32>(last.wrapping_add(4), op(img, f, 0x5b));
    }
    let count = op(img, f, 0x67);
    let c = g.read::<u16>(count).wrapping_add(1);
    g.write::<u16>(count, c);
    g.write::<u32>(op(img, f, 0x6d), op(img, f, 0x71));
    g.write::<u32>(op(img, f, 0x77), 0xff);
    Ret::jump(slot_target(g, op(img, f, 0x60), 845))
}

/// [`static_link_845`].
pub const STATIC_LINK_845: Sig = Sig {
    len: 135,
    hash: 0x4f71_0938_8833_a4bc,
    calls: &[],
    imports: &[],
};

/// The DLL's part `P` (operand `+0x6`) set up: slot 718 `(P)`, slots 222
/// and 221 `(P, 0, 1000, 0x100)`, then slot 719 `(P, 6, 0)` and `(P, 6,
/// 1)` give two handles (`G_A`, `G_B`), each call seeing the words the
/// earlier ones left (no arguments). Returns the second handle.
///
/// `magic_0183` 0x10001000 (136 bytes; 2 DLLs).
pub fn part_handles<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = op(img, f, 0x6);
    let mut words = vec![p];
    slot_call(g, img, f, (0x1, 718), &words);
    words.splice(0..0, [p, 0, 1000, 0x100]);
    slot_call(g, img, f, (0x13, 222), &words);
    words.splice(0..0, [p, 0, 1000, 0x100]);
    slot_call(g, img, f, (0x31, 221), &words);
    words.splice(0..0, [p, 6, 0]);
    // call width: 12 (the earlier calls' words stay on the stack)
    let a = slot_call(g, img, f, (0x4f, 719), &words).eax;
    g.write::<u32>(op(img, f, 0x65), a);
    words.splice(0..0, [p, 6, 1]);
    // call width: 15 (the earlier calls' words stay on the stack)
    let b = slot_call(g, img, f, (0x6a, 719), &words).eax;
    g.write::<u32>(op(img, f, 0x83), b);
    Ret::eax(b)
}

/// [`part_handles`].
pub const PART_HANDLES: Sig = Sig {
    len: 136,
    hash: 0xf2ec_54db_a01d_ba7a,
    calls: &[],
    imports: &[],
};

/// For the states 0x3011 and 0x3013: slot 0 `(s)` gives a handle `h`,
/// slot 26 `(s, 1)`, a byte of the DLL's (0x20, then 0x21) and `G_H` the
/// handle, slot 10 `(h, 0)`, each call seeing the words the earlier ones
/// left; then `G_S` 3 (no arguments). Returns the last slot's `eax`.
///
/// `magic_0162` 0x10002db0 (136 bytes; 2 DLLs).
pub fn handles_3011_3013<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let mut words: Vec<u32> = Vec::new();
    let mut ret = Ret::eax(fr.eax);
    for (s, (t0, t26, t10), (byte, v), h_at) in [
        (0x3011, (0x1, 0x11, 0x24), (0x2a, 0x20), 0x31),
        (0x3013, (0x3e, 0x4d, 0x60), (0x66, 0x21), 0x6d),
    ] {
        words.insert(0, s);
        // call width: 1..6 (the earlier calls' words stay on the stack)
        let h = slot_call(g, img, f, (t0, 0), &words).eax;
        words.splice(0..0, [s, 1]);
        // call width: 3..8 (the earlier calls' words stay on the stack)
        slot_call(g, img, f, (t26, 26), &words);
        g.write::<u8>(op(img, f, byte), v);
        g.write::<u32>(op(img, f, h_at), h);
        words.splice(0..0, [h, 0]);
        // call width: 5..10 (the earlier calls' words stay on the stack)
        ret = through(slot_call(g, img, f, (t10, 10), &words));
    }
    g.write::<u32>(op(img, f, 0x7e), 3);
    ret
}

/// [`handles_3011_3013`].
pub const HANDLES_3011_3013: Sig = Sig {
    len: 136,
    hash: 0x262c_06fc_5117_8223,
    calls: &[],
    imports: &[],
};

/// The node walk ([`crate::nodes::walk`], `ecx` = the cursor); when it
/// ran and the owner has a track (`+0x2c`), the track's key at the
/// owner's time `+0x30` goes to `+0x1c` ([`track_key`], a cursor on the
/// frame) and bit 0 of `+0x34` says whether the track ended (internal,
/// `ecx` = the cursor). Returns the key's result, or the walk's.
///
/// `magic_0155` 0x10008350 (137 bytes; 2 DLLs).
pub fn walk_track<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let cursor = fr.ecx;
    let (eax, _) = crate::nodes::walk(g, cursor, cursor, fr.eax);
    if g.read::<u8>(cursor + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let owner = g.read::<u32>(cursor);
    let track = g.read::<u32>(owner.wrapping_add(0x2c));
    if track == 0 {
        return Ret::eax(track);
    }
    // push ebp; sub esp, 0x10: the cursor is [ebp - 0x10]
    let local = fr.esp.wrapping_sub(0x14);
    let time = g.read::<u32>(owner.wrapping_add(0x30));
    g.write::<u32>(local, track);
    g.write::<u32>(local + 4, owner.wrapping_add(0x1c));
    g.write::<u32>(local + 8, time);
    let callee = Frame {
        at: img.rel32_target(f + 0x6b),
        esp: fr.esp.wrapping_sub(0x20),
        ecx: local,
        ..*fr
    };
    let ret = track_key(g, img, &callee);
    let flags = g.read::<u32>(owner.wrapping_add(0x34));
    let ended = g.read::<u32>(local + 0xc) != 0;
    g.write::<u32>(
        owner.wrapping_add(0x34),
        if ended { flags | 1 } else { flags & !1 },
    );
    ret
}

/// [`walk_track`].
pub const WALK_TRACK: Sig = Sig {
    len: 137,
    hash: 0x8587_893d_81f7_3619,
    calls: &[(0x6b, Callee::Sig(&TRACK_KEY))],
    imports: &[],
};

/// A task's object (`task + 4`): its clock `+0x2c4` steps by `K`
/// (operand `+0x1e`); while `+0x2d0` is 0 and flag bit 2 (byte `+0x108`)
/// is set, its four nodes `+0x120`, `+0x14c`, `+0x178`, `+0x2d8` leave
/// their parents ([`crate::sa_misc::leave`]), the task's node leaves its
/// ring and `+0x2d0` becomes 0xff (cdecl `(task)`). Returns the ring's
/// last link read, or `eax` as it came.
///
/// `magic_0412` 0x100069d0 (137 bytes; 2 DLLs).
pub fn clock_done<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let task = fr.arg(g, 0);
    let obj = g.read::<u32>(task.wrapping_add(4));
    let fl = |g: &G, a: u32| f32::from_bits(g.read::<u32>(a));
    let c = crate::sb_sse::add(fl(g, obj.wrapping_add(0x2c4)), fl(g, op(img, f, 0x1e)));
    g.write::<u32>(obj.wrapping_add(0x2c4), c.to_bits());
    if g.read::<u32>(obj.wrapping_add(0x2d0)) != 0 || g.read::<u8>(obj.wrapping_add(0x108)) & 4 == 0
    {
        return Ret::eax(fr.eax);
    }
    let mut eax = fr.eax;
    for off in [0x120, 0x14c, 0x178, 0x2d8] {
        eax = crate::sa_misc::leave(g, obj.wrapping_add(off), eax);
    }
    let node = g.read::<u32>(task);
    let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
    g.write::<u32>(prev + 8, next);
    let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(obj.wrapping_add(0x2d0), 0xff);
    Ret::eax(prev)
}

/// [`clock_done`].
pub const CLOCK_DONE: Sig = Sig {
    len: 137,
    hash: 0x3639_0c12_4ff8_2528,
    calls: &[
        (0x3c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x47, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x52, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x5d, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// Which of two of the game's lists holds the id `ecx`: slot 288 `(9,
/// &n)` gives the first list of `n` signed bytes, slot 288 `(11, &n)` the
/// second (the counts in the frame) (internal). Returns 1 for the first,
/// 0 for the second, -1 for neither.
///
/// `magic_0098` 0x10002b10 (137 bytes; 2 DLLs).
pub fn id_list<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let id = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    for (table, kind, count, found) in [(0xd, 9, 4, 1), (0x42, 0xb, 8, 0)] {
        let at = ebp.wrapping_sub(count);
        let list = slot_call(g, img, f, (table, 288), &[kind, at]).eax;
        let n = g.read::<i32>(at);
        for i in 0..n.max(0).cast_unsigned() {
            let b = g.read::<u8>(list.wrapping_add(i)).cast_signed();
            if i32::from(b).cast_unsigned() == id {
                return Ret::eax(found);
            }
        }
    }
    Ret::eax(u32::MAX)
}

/// [`id_list`].
pub const ID_LIST: Sig = Sig {
    len: 137,
    hash: 0xe63d_951b_088f_50ac,
    calls: &[],
    imports: &[],
};

/// Three of the DLL's floats 1.0 (operands `+0x1c`, `+0x26`, `+0x30`),
/// slot 786 `(a, b, c)` on three of its words (operands `+0x16`, `+0x11`,
/// `+0x6`), then slots 788, 787, 789 and 790 each give a float (ST0)
/// stored to the DLL (operands `+0x4c`, `+0x5f`, `+0x72`, `+0x85`) (no
/// arguments). Returns slot 790's `eax`.
///
/// `magic_0098` 0x10001000 (138 bytes; 2 DLLs).
pub fn camera_floats<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    for o in [0x1c, 0x26, 0x30] {
        g.write::<u32>(op(img, f, o), 0x3f80_0000);
    }
    let words = [op(img, f, 0x16), op(img, f, 0x11), op(img, f, 0x6)];
    let mut eax = slot_call(g, img, f, (0x1, 786), &words).eax;
    for (table, slot, dst) in [
        (0x3b, 788, 0x4c),
        (0x51, 787, 0x5f),
        (0x64, 789, 0x72),
        (0x77, 790, 0x85),
    ] {
        let out = slot_call(g, img, f, (table, slot), &[]);
        g.write::<u32>(op(img, f, dst), st0_f32(out));
        eax = out.eax;
    }
    Ret::eax(eax)
}

/// [`camera_floats`].
pub const CAMERA_FLOATS: Sig = Sig {
    len: 138,
    hash: 0x011b_e51f_1242_0437,
    calls: &[],
    imports: &[],
};

/// The actor's model (slot 168 `(*(*G + 0x24))` gives the handle `h`)
/// gets back its own value (slot 169 `(h)`, a float) through slot 170
/// `(h, x)`; then the DLL's handle `G_X`, when set, is released (slot
/// 804 `(G_X)`), slot 817, the byte `+0x11` of `*G_B` and `G_X` cleared
/// (no arguments). Returns `*G_B`.
///
/// `magic_0148` 0x10001820 (139 bytes; 2 DLLs).
pub fn model_reset_release<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let base = g.read::<u32>(op(img, f, 0x5));
    let x24 = g.read::<u32>(base.wrapping_add(0x24));
    let h = slot_call(g, img, f, (0xe, 168), &[x24]).eax;
    let x = st0_f32(slot_call(g, img, f, (0x1c, 169), &[h, x24]));
    slot_call(g, img, f, (0x34, 170), &[h, x]);
    let handle = g.read::<u32>(op(img, f, 0x4a));
    if handle != 0 {
        slot_call(g, img, f, (0x58, 804), &[handle]);
    }
    // the float slot 170 got is still on the stack
    slot_call(g, img, f, (0x68, 817), &[x]);
    let b = g.read::<u32>(op(img, f, 0x75));
    g.write::<u8>(b.wrapping_add(0x11), 0);
    g.write::<u32>(op(img, f, 0x7f), 0);
    Ret::eax(b)
}

/// [`model_reset_release`].
pub const MODEL_RESET_RELEASE: Sig = Sig {
    len: 139,
    hash: 0x3981_7b96_8bbb_338b,
    calls: &[],
    imports: &[],
};

/// An actor's scale (internal, `ecx` = its index `i`): while `G_DONE` is
/// 0 and the actor `k` (`G_MAP[i]`, or `G_NONE` for -1) is below 32, slot
/// 251 `(k)`'s float; else the product slot 17 `(A)` * slot 19 `(A)` for
/// the actor `A` (`T[G_MAP[i]]`, or `G_SELF` for -1; the first call's
/// word stays under the second's). Returns the float in `xmm0`, `eax` the
/// last slot's.
///
/// `magic_0391` 0x10004000 (140 bytes; 3 DLLs).
pub fn actor_scale_by<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let i = fr.ecx;
    let map = g.read::<u32>(op(img, f, 0x5));
    if g.read::<u32>(op(img, f, 0xe)) == 0 {
        let k = if i == u32::MAX {
            g.read::<u32>(op(img, f, 0x16))
        } else {
            u32::from(g.read::<u8>(map.wrapping_add(i)))
        };
        if k.cast_signed() < 0x20 {
            let out = slot_call(g, img, f, (0x2a, 251), &[k]);
            return Ret::xmm0(crate::Xmm::F32(st0_f32(out)), out.eax);
        }
    }
    let actor = if i == u32::MAX {
        g.read::<u32>(op(img, f, 0x4d))
    } else {
        let k = u32::from(g.read::<u8>(map.wrapping_add(i)));
        g.read::<u32>(op(img, f, 0x5a).wrapping_add(4 * k))
    };
    let a = f32::from_bits(st0_f32(slot_call(g, img, f, (0x5f, 19), &[actor])));
    let out = slot_call(g, img, f, (0x6d, 17), &[actor, actor]);
    let b = f32::from_bits(st0_f32(out));
    Ret::xmm0(crate::Xmm::F32(crate::sb_sse::mul(b, a).to_bits()), out.eax)
}

/// [`actor_scale_by`].
pub const ACTOR_SCALE_BY: Sig = Sig {
    len: 140,
    hash: 0xf3c9_b35c_ccc4_1676,
    calls: &[],
    imports: &[],
};

/// The DLL's part `P` (operand `+0xd`) dropped: slot 258 `(P)`, slot 231
/// `(*G_A, *G_B, *G_C)` and slot 230 `(*G_D, *G_C')` on its matrices,
/// `*G_P` = `ITEM`, slot 712 `(P, 0x30)` with `**G_LOCK` 1 around it,
/// each call seeing the words the earlier ones left (no arguments).
/// Returns 1 when slot 712 answers 0, else 0.
///
/// `magic_0490` 0x10001070 (141 bytes; 2 DLLs).
pub fn part_drop_fixed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let rd = |g: &G, o: u32| g.read::<u32>(op(img, f, o));
    let p = op(img, f, 0xd);
    slot_call(g, img, f, (0x1, 258), &[p]);
    let (c, b, a) = (rd(g, 0x15), rd(g, 0x20), rd(g, 0x2c));
    slot_call(g, img, f, (0x1a, 231), &[a, b, c, p]);
    let (e, d) = (rd(g, 0x34), rd(g, 0x3f));
    slot_call(g, img, f, (0x39, 230), &[d, e, a, b, c, p]);
    let ptr = rd(g, 0x4c);
    g.write::<u32>(ptr, op(img, f, 0x54));
    let lock = rd(g, 0x59);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 8 (the earlier calls' words stay on the stack)
    let r = slot_call(g, img, f, (0x6b, 712), &[p, 0x30, d, e, a, b, c, p]).eax;
    let lock = rd(g, 0x79);
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// [`part_drop_fixed`].
pub const PART_DROP_FIXED: Sig = Sig {
    len: 141,
    hash: 0x1a8e_f379_0966_94c0,
    calls: &[],
    imports: &[],
};

/// The effect reset ([`crate::sb_misc::effect_reset`]), byte `G_B` 1,
/// slot 235 `(V)` on the DLL's vector `V` (operand `+0x1f`), slot 329
/// `(V, V, s)` with the block's scale `*G + 0x3150`, then the block's
/// `+0xa90..+0xa9c` into the DLL's four floats (operands `+0x55`, `+0x65`,
/// `+0x78`, `+0x88`) (no arguments). Returns slot 329's `eax`.
///
/// `magic_0168` 0x10001c30 (142 bytes; 3 DLLs).
pub fn reset_and_scale<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let block = g.read::<u32>(op(img, f, 0x3));
    crate::sb_misc::effect_reset(g, img, img.rel32_target(f + 0x8));
    g.write::<u8>(op(img, f, 0x13), 1);
    let v = op(img, f, 0x1f);
    slot_call(g, img, f, (0xd, 235), &[v]);
    let s = g.read::<u32>(block.wrapping_add(0x3150));
    let eax = slot_call(g, img, f, (0x2e, 329), &[v, v, s]).eax;
    for (i, o) in (0u32..).zip([0x55, 0x65, 0x78, 0x88]) {
        let x = g.read::<u32>(block.wrapping_add(0xa90 + 4 * i));
        g.write::<u32>(op(img, f, o), x);
    }
    Ret::eax(eax)
}

/// [`reset_and_scale`].
pub const RESET_AND_SCALE: Sig = Sig {
    len: 142,
    hash: 0x3679_2406_345d_17b3,
    calls: &[(0x8, Callee::Sig(&crate::sb_misc::EFFECT_RESET))],
    imports: &[],
};
