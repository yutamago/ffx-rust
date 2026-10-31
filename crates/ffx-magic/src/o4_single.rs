//! Single functions of the own work list (round 4): one DLL each, small
//! slot calls, list searches and tail jumps. Operands are read from the
//! code (`w(off)` = the word at `+off`), so a later copy elsewhere binds
//! by its own signature.

use crate::l5_objhost::{list_remove, run_objects};
use crate::runtime::slot_target;
use crate::sb_misc::timeline_run;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The slot of a `mov eax, [eax + disp32]` with the `disp32` at `at`.
fn slot_at(img: &Image, at: u32) -> u32 {
    img.word(at) / 4
}

/// An actor's character byte or the effect's own: `*A` (the operand at
/// `own`) for index -1, else the byte `i` of the list `*B` (`list`).
fn actor_word(g: &impl crate::Mem, img: &Image, f: u32, own: u32, list: u32, i: u32) -> u32 {
    if i == u32::MAX {
        g.read::<u32>(img.word(f + own))
    } else {
        u32::from(g.read::<u8>(g.read::<u32>(img.word(f + list)).wrapping_add(i)))
    }
}

/// Slot 537 for an actor (`magic_0171` 0x100029c0, 51 bytes, internal:
/// `ecx` = the actor index): `(*A, 2, 0, c, 2, 0xb)` with `c` the actor's
/// byte (`*A` itself for index -1; `A` at `+0x02`, the list at `+0x10`;
/// the immediates at `+0x2b`, `+0x23`, `+0x1b`, `+0x19`). Returns the
/// slot's `eax`.
pub fn actor_slot<G: Game>(g: &mut G, img: &Image, f: u32, i: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x02));
    let c = actor_word(g, img, f, 0x02, 0x10, i);
    let s = slot_target(g, w(0x1e), slot_at(img, f + 0x26));
    let d = |off: u32| crate::o4_parts::disp8(img, f + off);
    g.call(s, i, &[a, d(0x2b), d(0x23), c, d(0x1b), d(0x19)])
        .eax
}

/// Slot 538 for an actor (`magic_0298` 0x10002250, 57 bytes, internal:
/// `ecx` = the actor index, `edx` the second word): `(c, edx)`, `c` the
/// actor's byte or `*A` for index -1 (`A` at `+0x06`, the list at
/// `+0x1e`; the slot at `+0x13`/`+0x2f`). Returns the slot's `eax`.
pub fn actor_slot_edx<G: Game>(g: &mut G, img: &Image, f: u32, i: u32, edx: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let c = actor_word(g, img, f, 0x06, 0x1e, i);
    let (table, at) = if i == u32::MAX {
        (0x0d, 0x13)
    } else {
        (0x29, 0x2f)
    };
    let s = slot_target(g, w(table), slot_at(img, f + at));
    g.call(s, i, &[c, edx]).eax
}

/// `(rec)` (`magic_0270` 0x10001c70, 69 bytes): slot 537 (`+0x35`) with
/// `(*A, 2, 0, c, rec[0x30], rec[0x34])`, `c` the byte of the actor
/// `rec[0x2c]` (`*A` for -1; `A` at `+0x08`, the list at `+0x21`).
/// Returns the slot's `eax`.
pub fn record_actor_slot<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.arg(g, 0);
    let a = g.read::<u32>(w(0x08));
    let i = g.read::<u32>(rec.wrapping_add(0x2c));
    let z = g.read::<u32>(rec.wrapping_add(0x34));
    let y = g.read::<u32>(rec.wrapping_add(0x30));
    let c = actor_word(g, img, f, 0x08, 0x21, i);
    let s = slot_target(g, w(0x2d), slot_at(img, f + 0x35));
    let d = |off: u32| crate::o4_parts::disp8(img, f + off);
    Ret::eax(g.call(s, fr.ecx, &[a, d(0x3a), d(0x32), c, y, z]).eax)
}

/// `(p)` (`magic_0203` 0x100045f0, 51 bytes): unless the byte `(*p) +
/// 0xf8` has bit 2, the word `(*p)[0x60]` goes to `**G` (`G` at `+0x12`),
/// the argument becomes `*p + 0x70` and the function tail-jumps to slot
/// 713 (`+0x2a`) with it. Returns `eax` as given otherwise.
pub fn tail_free<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let p = fr.arg(g, 0);
    let q = g.read::<u32>(p);
    if g.read::<u8>(q.wrapping_add(0xf8)) & 4 != 0 {
        return Ret::eax(fr.eax);
    }
    let dst = g.read::<u32>(w(0x12));
    let v = g.read::<u32>(q.wrapping_add(0x60));
    g.write::<u32>(dst, v);
    let q = g.read::<u32>(p);
    // mov [ebp + 8], eax: the caller's argument word, then jmp eax
    g.write::<u32>(fr.esp + 4, q.wrapping_add(0x70));
    Ret::jump(slot_target(g, w(0x24), slot_at(img, f + 0x2a)))
}

/// `(rec)` (`magic_0351` 0x10001bf0, 72 bytes): for a record of the
/// table at `BASE` (`+0x0f`, 0x80 bytes each) whose index is below `N`
/// (`+0x07`) and whose entry of `TABLE` (`+0x1e`) is set: `*G_P = ITEM`
/// (`+0x26`, `+0x2d`) and slot 713 (`+0x38`) frees its part. Returns 1.
pub fn free_by_index<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let rec = fr.arg(g, 0);
    let last = g.read::<u32>(w(0x07)).wrapping_sub(1).cast_signed();
    let i = rec.wrapping_sub(w(0x0f)).cast_signed() >> 7;
    if i <= last && g.read::<u32>(w(0x1e).wrapping_add(i.cast_unsigned().wrapping_mul(4))) != 0 {
        let gp = g.read::<u32>(w(0x26));
        g.write::<u32>(gp, w(0x2d));
        let s = slot_target(g, w(0x32), slot_at(img, f + 0x38));
        g.call(s, fr.ecx, &[rec]);
    }
    Ret::eax(1)
}

/// No arguments (`magic_0275` 0x10004310, 72 bytes): slot 852 (`+0x07`)
/// starts an iteration and slot 853 (`+0x14`, `+0x3b`) gives each node;
/// returns the first whose `u16` equals that of `**G` (`G` at `+0x22`)
/// without being it, or 0.
pub fn find_same_id<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x01), slot_at(img, f + 0x07));
    g.call(s, fr.ecx, &[]);
    let s = slot_target(g, w(0x0e), slot_at(img, f + 0x14));
    let mut node = g.call(s, fr.ecx, &[]).eax;
    while node != 0 {
        let me = g.read::<u32>(g.read::<u32>(w(0x22)));
        if g.read::<u16>(node) == g.read::<u16>(me) && node != me {
            return Ret::eax(node);
        }
        let s = slot_target(g, w(0x35), slot_at(img, f + 0x3b));
        node = g.call(s, fr.ecx, &[]).eax;
    }
    Ret::eax(0)
}

/// No arguments (`magic_0601` 0x10008070, 54 bytes): with bit 0 of the
/// byte `A` (`+0x02`), slot 257 (`+0x10`) with `(*B, 0)` (`B` at
/// `+0x18`); without it, slot 296 (`+0x24`) with `(*C, 1)` (`C` at
/// `+0x2c`). Returns the slot's `eax`.
pub fn flag_pick_slot<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = if g.read::<u8>(w(0x02)) & 1 != 0 {
        let b = g.read::<u32>(w(0x18));
        let s = slot_target(g, w(0x08), slot_at(img, f + 0x10));
        g.call(s, fr.ecx, &[b, 0])
    } else {
        let c = g.read::<u32>(w(0x2c));
        let s = slot_target(g, w(0x08), slot_at(img, f + 0x24));
        g.call(s, fr.ecx, &[c, 1])
    };
    Ret::eax(r.eax)
}

/// No arguments (`magic_0263` 0x10006b90, 76 bytes): on the model `M` =
/// `*G` (`+0x03`), unless bit 0 of the byte `M + 0x90` is set: slot 211
/// (`+0x1c`) with `(M[0x54], 0)`, the halfword `M + 0x90` loses 0x10,
/// slot 255 (`+0x41`) with `(M[0x50], M[0x4c] + 0xcc)` (the first call's
/// words after them). Returns the last slot's `eax` (`eax` as given when
/// the bit is set).
pub fn model_slots<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = g.read::<u32>(w(0x03));
    if g.read::<u8>(m + 0x90) & 1 != 0 {
        return Ret::eax(fr.eax);
    }
    let a = g.read::<u32>(m + 0x54);
    let s = slot_target(g, w(0x11), slot_at(img, f + 0x1c));
    g.call(s, fr.ecx, &[a, 0]);
    let flags = g.read::<u16>(m + 0x90);
    g.write::<u16>(m + 0x90, flags & 0xffef);
    let b = g.read::<u32>(m + 0x4c).wrapping_add(0xcc);
    let c = g.read::<u32>(m + 0x50);
    let s = slot_target(g, w(0x38), slot_at(img, f + 0x41));
    Ret::eax(g.call(s, fr.ecx, &[c, b, a, 0]).eax)
}

/// No arguments (`magic_0598` 0x100077a0, 76 bytes): `r` = slot 0 with
/// `(K)` (`K` at `+0x09`); slot 26 (`+0x19`) with `(K, 1)` (`K` again at
/// `+0x1b`); the byte `A` = the immediate at `+0x2e` (`+0x2a`), `B` = `r`
/// (`+0x31`); slot 10 (`+0x37`) with `(r, 0)`, each with the earlier
/// words after its own; then `C` = the immediate at `+0x46` (`+0x42`).
/// Returns slot 10's `eax`.
pub fn display_list<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = w(0x09);
    let s = slot_target(g, w(0x01), 0);
    let r = g.call(s, fr.ecx, &[k]).eax;
    let k2 = w(0x1b);
    let s = slot_target(g, w(0x11), crate::o4_parts::disp8(img, f + 0x19) / 4);
    g.call(s, fr.ecx, &[k2, 1, k]);
    g.write::<u8>(w(0x2a), w(0x2e) as u8);
    g.write::<u32>(w(0x31), r);
    let s = slot_target(g, w(0x24), crate::o4_parts::disp8(img, f + 0x37) / 4);
    let eax = g.call(s, fr.ecx, &[r, 0, k2, 1, k]).eax;
    g.write::<u32>(w(0x42), w(0x46));
    Ret::eax(eax)
}

/// No arguments (`magic_0671` 0x10001340, 70 bytes): slot 928 (`+0x07`),
/// slot 927 (`+0x16`) with the four floats at `+0x38`, `+0x31`, `+0x29`,
/// `+0x21` and 0, then a tail jump to the object run ([`run_objects`],
/// `+0x42`). Returns what it returns.
pub fn colour_then_run<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x01), slot_at(img, f + 0x07));
    g.call(s, fr.ecx, &[]);
    let s = slot_target(g, w(0x0e), slot_at(img, f + 0x16));
    g.call(s, fr.ecx, &[w(0x38), w(0x31), w(0x29), w(0x21), 0]);
    // jmp: the object run returns to the caller
    Ret::eax(run_objects(g, img, img.rel32_target(f + 0x42), fr.esp))
}

/// No arguments (`magic_0540` 0x10001080, 23 bytes): `**A = *B` (`A` at
/// `+0x02`, `B` at `+0x07`), then a tail jump to the timeline run
/// ([`timeline_run`], `+0x13`) with `this` the operand at `+0x0e`.
pub fn set_then_timeline<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let dst = g.read::<u32>(w(0x02));
    let v = g.read::<u32>(w(0x07));
    g.write::<u32>(dst, v);
    Ret::eax(timeline_run(g, img, img.rel32_target(f + 0x13), w(0x0e)))
}

/// `(obj)` (`magic_0195` 0x100012c0, 57 bytes): `rcefTaskSetSeq(obj,
/// cb)` (`+0x12`, `cb` at `+0x09`) and `rcefTaskWait(obj, T)` (`+0x2e`,
/// `T` = `TABLE[*I]`, `I` at `+0x19`, `TABLE` at `+0x20`; the first
/// call's words after them). Returns the last slot's `eax`.
pub fn seq_wait_table<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x12));
    g.call(s, fr.ecx, &[obj, w(0x09)]);
    let i = g.read::<u32>(w(0x19));
    let t = g.read::<u32>(w(0x20).wrapping_add(i.wrapping_mul(4)));
    let s = slot_target(g, w(0x25), slot_at(img, f + 0x2e));
    Ret::eax(g.call(s, fr.ecx, &[obj, t, obj, w(0x09)]).eax)
}

/// `(obj)` (`magic_0382` 0x10001120, 67 bytes): `B = *A` (`+0x09`,
/// `+0x04`), `C` = the immediate at `+0x26` (`+0x22`),
/// `rcefTaskSetSeq(obj, cb)` (`+0x1c`, `+0x13`) and `rcefTaskWait(obj,
/// T)` (`+0x38`, `T` at `+0x32`; the first call's words after them).
/// Returns the last slot's `eax`.
pub fn copy_seq_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = g.read::<u32>(w(0x04));
    g.write::<u32>(w(0x09), v);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x0e), slot_at(img, f + 0x1c));
    g.write::<u32>(w(0x22), w(0x26));
    g.call(s, fr.ecx, &[obj, w(0x13)]);
    let t = crate::o4_parts::disp8(img, f + 0x32);
    let s = slot_target(g, w(0x2d), slot_at(img, f + 0x38));
    Ret::eax(g.call(s, fr.ecx, &[obj, t, obj, w(0x13)]).eax)
}

/// `(obj)` (`magic_0415` 0x10001000, 72 bytes): `rcefTaskSetSeq(obj,
/// cb)` (`+0x12`, `cb` at `+0x09`) with `C` = the immediate at `+0x1c`
/// (`+0x18`) set first, `rcefTaskCallSeq(obj, cb2)` (`+0x31`, `+0x28`),
/// then a tail jump to slot 521 (`+0x41`) with the caller's stack.
pub fn seq_call_then_jump<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x04), slot_at(img, f + 0x12));
    g.write::<u32>(w(0x18), w(0x1c));
    g.call(s, fr.ecx, &[obj, w(0x09)]);
    let s = slot_target(g, w(0x23), slot_at(img, f + 0x31));
    g.call(s, fr.ecx, &[obj, w(0x28), obj, w(0x09)]);
    Ret::jump(slot_target(g, w(0x38), slot_at(img, f + 0x41)))
}

/// `(obj)` (`magic_0397` 0x10005870, 78 bytes): slot 239 (`+0x1b`) with
/// `(*G, obj[0x120], 0)` (`G` at `+0x15`); `r` = slot 719 (`+0x2a`) with
/// `(P, 9, 0)` (`P` at `+0x31`, the immediates at `+0x2f`, `+0x27`);
/// slot 306 (`+0x41`) with `(7, 3, r)` (`+0x46`, `+0x3e`), each with the
/// earlier words after its own. Returns its `eax`.
pub fn slot_chain_306<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let d = |off: u32| crate::o4_parts::disp8(img, f + off);
    let obj = fr.arg(g, 0);
    let a = g.read::<u32>(obj.wrapping_add(0x120));
    let h = g.read::<u32>(w(0x15));
    let s = slot_target(g, w(0x0f), slot_at(img, f + 0x1b));
    g.call(s, fr.ecx, &[h, a, 0]);
    let s = slot_target(g, w(0x22), slot_at(img, f + 0x2a));
    let r = g.call(s, fr.ecx, &[w(0x31), d(0x2f), d(0x27), h, a, 0]).eax;
    let s = slot_target(g, w(0x39), slot_at(img, f + 0x41));
    let args = [d(0x46), d(0x3e), r, w(0x31), d(0x2f), d(0x27), h, a, 0];
    Ret::eax(g.call(s, fr.ecx, &args).eax)
}

/// No arguments (`magic_0397` 0x10006390, 71 bytes): the object `*G +
/// E` (`G` at `+0x03`, `E` at `+0x09`) taken off the list `L`
/// ([`list_remove`], `+0x15`, `L` at `+0x10`); if it was there, `*G_P =
/// ITEM` (`+0x1e`, `+0x25`), slot 713 (`+0x30`) frees its part and its
/// flags (`u16 +0x124`) lose 0x8000, and 0x7fff is returned; else 0.
pub fn remove_and_free<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = g.read::<u32>(w(0x03)).wrapping_add(w(0x09));
    if list_remove(g, img, img.rel32_target(f + 0x15), w(0x10), obj) == 0 {
        return Ret::eax(0);
    }
    let gp = g.read::<u32>(w(0x1e));
    g.write::<u32>(gp, w(0x25));
    let s = slot_target(g, w(0x2a), slot_at(img, f + 0x30));
    g.call(s, fr.ecx, &[obj]);
    let flags = g.read::<u16>(obj.wrapping_add(0x124));
    g.write::<u16>(obj.wrapping_add(0x124), flags & 0x7fff);
    Ret::eax(0x7fff)
}

/// [`actor_slot`] in `magic_0171` (0x100029c0, 51 bytes).
pub const ACTOR_SLOT_0171_29C0: Sig = Sig {
    len: 51,
    hash: 0x38d8_06a9_e27a_74a8,
    calls: &[],
    imports: &[],
};

/// [`actor_slot_edx`] in `magic_0298` (0x10002250, 57 bytes).
pub const ACTOR_SLOT_EDX_0298_2250: Sig = Sig {
    len: 57,
    hash: 0x90df_f013_709c_9a9e,
    calls: &[],
    imports: &[],
};

/// [`record_actor_slot`] in `magic_0270` (0x10001c70, 69 bytes).
pub const RECORD_ACTOR_SLOT_0270_1C70: Sig = Sig {
    len: 69,
    hash: 0x5b50_cf11_5b8b_f80d,
    calls: &[],
    imports: &[],
};

/// [`tail_free`] in `magic_0203` (0x100045f0, 51 bytes).
pub const TAIL_FREE_0203_45F0: Sig = Sig {
    len: 51,
    hash: 0x88ed_8981_8d36_09bd,
    calls: &[],
    imports: &[],
};

/// [`free_by_index`] in `magic_0351` (0x10001bf0, 72 bytes).
pub const FREE_BY_INDEX_0351_1BF0: Sig = Sig {
    len: 72,
    hash: 0xd4c6_a126_aced_f396,
    calls: &[],
    imports: &[],
};

/// [`find_same_id`] in `magic_0275` (0x10004310, 72 bytes).
pub const FIND_SAME_ID_0275_4310: Sig = Sig {
    len: 72,
    hash: 0x2b25_5d90_7661_5b90,
    calls: &[],
    imports: &[],
};

/// [`flag_pick_slot`] in `magic_0601` (0x10008070, 54 bytes).
pub const FLAG_PICK_SLOT_0601_8070: Sig = Sig {
    len: 54,
    hash: 0x4184_5916_fe3e_5d81,
    calls: &[],
    imports: &[],
};

/// [`model_slots`] in `magic_0263` (0x10006b90, 76 bytes).
pub const MODEL_SLOTS_0263_6B90: Sig = Sig {
    len: 76,
    hash: 0xc378_a326_8db6_3e40,
    calls: &[],
    imports: &[],
};

/// [`display_list`] in `magic_0598` (0x100077a0, 76 bytes).
pub const DISPLAY_LIST_0598_77A0: Sig = Sig {
    len: 76,
    hash: 0xe089_506b_ceb9_9f3e,
    calls: &[],
    imports: &[],
};

/// [`colour_then_run`] in `magic_0671` (0x10001340, 70 bytes).
pub const COLOUR_THEN_RUN_0671_1340: Sig = Sig {
    len: 70,
    hash: 0xc0bc_4b8b_c866_2904,
    calls: &[(0x42, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// [`set_then_timeline`] in `magic_0540` (0x10001080, 23 bytes).
pub const SET_THEN_TIMELINE_0540_1080: Sig = Sig {
    len: 23,
    hash: 0x372e_3fcb_93a3_df90,
    calls: &[(0x13, Callee::Sig(&crate::sb_misc::TIMELINE_RUN))],
    imports: &[],
};

/// [`seq_wait_table`] in `magic_0195` (0x100012c0, 57 bytes).
pub const SEQ_WAIT_TABLE_0195_12C0: Sig = Sig {
    len: 57,
    hash: 0x6289_e482_b734_9665,
    calls: &[],
    imports: &[],
};

/// [`copy_seq_wait`] in `magic_0382` (0x10001120, 67 bytes).
pub const COPY_SEQ_WAIT_0382_1120: Sig = Sig {
    len: 67,
    hash: 0x4306_ce7a_ab3f_3cae,
    calls: &[],
    imports: &[],
};

/// [`seq_call_then_jump`] in `magic_0415` (0x10001000, 72 bytes).
pub const SEQ_CALL_THEN_JUMP_0415_1000: Sig = Sig {
    len: 72,
    hash: 0x9868_6b66_4c1e_df3c,
    calls: &[],
    imports: &[],
};

/// [`slot_chain_306`] in `magic_0397` (0x10005870, 78 bytes).
pub const SLOT_CHAIN_306_0397_5870: Sig = Sig {
    len: 78,
    hash: 0x3b78_58e5_98b8_e55a,
    calls: &[],
    imports: &[],
};

/// [`remove_and_free`] in `magic_0397` (0x10006390, 71 bytes).
pub const REMOVE_AND_FREE_0397_6390: Sig = Sig {
    len: 71,
    hash: 0x865e_9ae9_f933_1ec8,
    calls: &[(0x15, Callee::Sig(&crate::l5_objhost::LIST_REMOVE))],
    imports: &[],
};
