//! Small shared functions of the shared-4 list: one or two engine slot
//! calls with a store or a flag around them. Each doc names the DLL and
//! address it was ported from, its size and how many DLLs carry it;
//! "`G`" is a global of the DLL (an operand of the code).

use crate::l5_objhost::{RUN_OBJECTS, run_objects};
use crate::runtime::slot_target;
use crate::sa_pool::{GIVE_BACK, give_back};
use crate::sa_summon::{BONE, MODEL_SETUP, bone_plain, model_setup};
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
