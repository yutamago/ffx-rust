//! Small shared functions of the shared-4 list: one or two engine slot
//! calls with a store or a flag around them. Each doc names the DLL and
//! address it was ported from, its size and how many DLLs carry it;
//! "`G`" is a global of the DLL (an operand of the code).

use crate::l5_objhost::{LIST_REMOVE, RUN_OBJECTS, list_remove, run_objects};
use crate::runtime::slot_target;
use crate::s4_misc::{REGISTER_80, SCALE_TASK, register_80, scale_task};
use crate::s4_summon::trunc;
use crate::sa_misc::{REGISTER_PARTS, register_parts};
use crate::sa_pool::{GIVE_BACK, give_back};
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
    let f = fr.at;
    let a = first(g, img, img.rel32_target(f + 0x5));
    // push ebp; push esi; call: the run's return address
    let b = run_objects(g, img, img.rel32_target(f + 0xc), fr.esp.wrapping_sub(12));
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
