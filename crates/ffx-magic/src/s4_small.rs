//! Small shared functions of the shared-4 list: one or two engine slot
//! calls with a store or a flag around them. Each doc names the DLL and
//! address it was ported from, its size and how many DLLs carry it;
//! "`G`" is a global of the DLL (an operand of the code).

use crate::runtime::slot_target;
use crate::sa_pool::{GIVE_BACK, give_back};
use crate::sa_summon::{BONE, MODEL_SETUP, bone_plain, model_setup};
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
