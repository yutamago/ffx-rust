//! The effect script commands of the `magic_0162` family (53 to 79
//! overlays; addresses are `magic_0162`'s). Each command is a cdecl
//! `(obj, pc)` from the DLL's command table and returns the next `pc`;
//! `pc` points at the command's words (`u16` at `+0x2` its first
//! operand, words from `+0x4`). The object's script state: `+0x14` a
//! return stack of 16 words, byte `+0x58` its depth, byte `+0x56` the
//! wait mode with `u16 +0x54`/`+0x24` the wait operand.
//!
//! An actor operand (`i16`) names the caller with -1, else an index into
//! the DLL's actor list (`G_ACTORS`, a byte per actor); [`who`] gives the
//! battle number, [`obj_of`] the actor's object (`G_ME_OBJ` for -1, else
//! `G_OBJS[number]`). Slot tables named by the instructions' operands
//! (`w(offset)`) as everywhere.

use crate::runtime::slot_target;
use crate::sa_misc::cvt;
use crate::sb_sse::{div, mul};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

fn slot<G: Game>(g: &G, img: &Image, f: u32, off: u32, slot: u32) -> u32 {
    slot_target(g, img.word(f + off), slot)
}

/// An `i16` operand as the original's `movsx` gives it.
fn i16_at(g: &impl crate::Mem, addr: u32) -> u32 {
    i32::from(g.read::<i16>(addr)).cast_unsigned()
}

/// The actor `i` (`i16`, -1 the caller) as a battle number: `*G_ME`
/// (operand at `me`) or the byte `G_ACTORS[i]` (operand at `list`).
fn who<G: Game>(g: &G, img: &Image, f: u32, (me, list): (u32, u32), i: u32) -> u32 {
    if i == u32::MAX {
        g.read::<u32>(img.word(f + me))
    } else {
        let actors = g.read::<u32>(img.word(f + list));
        u32::from(g.read::<u8>(actors.wrapping_add(i)))
    }
}

/// The actor `i`'s object: `*G_ME_OBJ` (operand at `me`) or
/// `G_OBJS[G_ACTORS[i]]` (operands at `list` and `objs`).
fn obj_of<G: Game>(g: &G, img: &Image, f: u32, (me, list, objs): (u32, u32, u32), i: u32) -> u32 {
    if i == u32::MAX {
        g.read::<u32>(img.word(f + me))
    } else {
        let actors = g.read::<u32>(img.word(f + list));
        let n = u32::from(g.read::<u8>(actors.wrapping_add(i)));
        g.read::<u32>(img.word(f + objs).wrapping_add(4 * n))
    }
}

/// The `(obj, pc)` of a command.
fn args<G: Game>(g: &G, fr: &Frame) -> (u32, u32) {
    (fr.arg(g, 0), fr.arg(g, 1))
}

/// 0x10006dc0: ends the wait (mode `+0x56` 0). Returns `obj`'s second
/// argument as it is (`pc`: the same command again).
pub fn wait_off<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (obj, pc) = args(g, fr);
    g.write::<u8>(obj + 0x56, 0);
    Ret::eax(pc)
}

/// `wait_off` (15 bytes, 79 overlays).
pub const WAIT_OFF: Sig = Sig {
    len: 15,
    hash: 0x6400_3748_919d_e787,
    calls: &[],
    imports: &[],
};

/// 0x10006d00: return: pops the return stack (`+0x58` down) and goes to
/// the address there. (`edx` is left holding `obj`.)
pub fn ret<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let obj = fr.arg(g, 0);
    let d = g.read::<u8>(obj + 0x58).wrapping_sub(1);
    g.write::<u8>(obj + 0x58, d);
    let to = g.read::<u32>(obj + 0x14 + 4 * u32::from(d));
    Ret {
        edx: obj,
        ..Ret::eax(to)
    }
}

/// `ret` (19 bytes, 79 overlays).
pub const RET: Sig = Sig {
    len: 19,
    hash: 0xb0bc_25ce_7913_7bbd,
    calls: &[],
    imports: &[],
};

/// 0x10006ce0: call: pushes `pc + 8` on the return stack and goes to
/// `[pc + 4]`.
pub fn call<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (obj, pc) = args(g, fr);
    let d = g.read::<u8>(obj + 0x58);
    g.write::<u32>(obj + 0x14 + 4 * u32::from(d), pc + 8);
    g.write::<u8>(obj + 0x58, d.wrapping_add(1));
    Ret::eax(g.read::<u32>(pc + 4))
}

/// `call` (30 bytes, 79 overlays).
pub const CALL: Sig = Sig {
    len: 30,
    hash: 0x6fa4_0f54_8508_d01b,
    calls: &[],
    imports: &[],
};

/// 0x10006d40: wait (mode 3) for the word `[pc + 4]` (into `+0x24`).
pub fn wait_word<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (obj, pc) = args(g, fr);
    g.write::<u8>(obj + 0x56, 3);
    let v = g.read::<u32>(pc + 4);
    g.write::<u32>(obj + 0x24, v);
    Ret::eax(pc + 8)
}

/// `wait_word` (24 bytes, 79 overlays).
pub const WAIT_WORD: Sig = Sig {
    len: 24,
    hash: 0x488c_1211_0e92_bc2d,
    calls: &[],
    imports: &[],
};

/// 0x10006c20: wait (mode 2) for the `u16` operand (into `+0x54`).
pub fn wait_short<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (obj, pc) = args(g, fr);
    g.write::<u8>(obj + 0x56, 2);
    let v = g.read::<u16>(pc + 2);
    g.write::<u16>(obj + 0x54, v);
    Ret::eax(pc + 4)
}

/// `wait_short` (26 bytes, 79 overlays).
pub const WAIT_SHORT: Sig = Sig {
    len: 26,
    hash: 0x7e68_8082_2cb0_78a6,
    calls: &[],
    imports: &[],
};

/// 0x100078f0: branch on a callback: `[pc + 4]` called with `obj`
/// (`ecx` = the callback); 1 goes to `[pc + 8]`, anything else to
/// `pc + 0xc`.
pub fn branch_on<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (obj, pc) = args(g, fr);
    let cb = g.read::<u32>(pc + 4);
    if g.call(cb, cb, &[obj]).eax == 1 {
        Ret::eax(g.read::<u32>(pc + 8))
    } else {
        Ret::eax(pc + 0xc)
    }
}

/// `branch_on` (35 bytes, 79 overlays).
pub const BRANCH_ON: Sig = Sig {
    len: 35,
    hash: 0xd2af_06e5_6b59_e35a,
    calls: &[],
    imports: &[],
};

/// The record `i` (the `u16` operand) of the DLL's 0x148-byte table
/// whose field the operand at `op` names. (The original's `cmove` for
/// -1 can never fire after a `movzx`.)
fn record<G: Game>(g: &G, img: &Image, f: u32, op: u32, pc: u32) -> u32 {
    let i = u32::from(g.read::<u16>(pc + 2));
    img.word(f + op).wrapping_add(i.wrapping_mul(0x148))
}

/// 0x100076a0: clears the record's byte (`+0x144` of the table).
pub fn record_clear<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let at = record(g, img, fr.at, 0x1c, pc);
    g.write::<u8>(at, 0);
    Ret::eax(pc + 4)
}

/// `record_clear` (38 bytes, 79 overlays).
pub const RECORD_CLEAR: Sig = Sig {
    len: 38,
    hash: 0x01e1_ef2e_acc9_518c,
    calls: &[],
    imports: &[],
};

/// 0x10007050: sets the record's word to `*[pc + 4]`.
pub fn record_set<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let v = g.read::<u32>(g.read::<u32>(pc + 4));
    let at = record(g, img, fr.at, 0x21, pc);
    g.write::<u32>(at, v);
    Ret::eax(pc + 8)
}

/// `record_set` (42 bytes, 79 overlays).
pub const RECORD_SET: Sig = Sig {
    len: 42,
    hash: 0xb6c6_d384_2a21_bdbc,
    calls: &[],
    imports: &[],
};

/// 0x10007080: copies the vector `*[pc + 4]` (three words) into the
/// record's `+0x50`.
pub fn record_vector<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let src = g.read::<u32>(pc + 4);
    let v = [
        g.read::<u32>(src + 4),
        g.read::<u32>(src + 8),
        g.read::<u32>(src),
    ];
    let at = record(g, img, fr.at, 0x2b, pc);
    g.write::<u32>(at + 0x50, v[2]);
    g.write::<u32>(at + 0x54, v[0]);
    g.write::<u32>(at + 0x58, v[1]);
    Ret::eax(pc + 8)
}

/// `record_vector` (65 bytes, 79 overlays).
pub const RECORD_VECTOR: Sig = Sig {
    len: 65,
    hash: 0x0dfa_8859_b7f4_3332,
    calls: &[],
    imports: &[],
};

/// A fade of the record: `ramp` = the record's ramp block; `target` the
/// value, `step` per frame, `frames` the count (`int`): the ramp's
/// `+0x20` target, `+0x34` `step * frames`, `+0x2c` 0, `+0x30`
/// `K / frames` (SSE), `+0x38` the step, `+0x3c` the mode.
fn ramp<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    k_op: u32,
    block: u32,
    words: [u32; 3],
    mode: u32,
) {
    let [target, step, frames] = words;
    let k = f32::from_bits(g.read::<u32>(img.word(f + k_op)));
    let inv = div(k, cvt(frames.cast_signed()));
    g.write::<u32>(block + 0x38, step);
    g.write::<u32>(block + 0x20, target);
    g.write::<u32>(block + 0x34, step.wrapping_mul(frames));
    g.write::<u32>(block + 0x2c, 0);
    g.write::<u32>(block + 0x30, inv.to_bits());
    g.write::<u32>(block + 0x3c, mode);
}

/// 0x10007260: starts the record's ramp (`+0x18` of the table... the
/// block at the operand): value `[pc + 4]`, step `[pc + 8]`, frames
/// `[pc + 0xc]`; mode 2.
pub fn record_ramp<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let frames = g.read::<u32>(pc + 0xc);
    let a = g.read::<u32>(pc + 4);
    let d = g.read::<u32>(pc + 8);
    let k = f32::from_bits(g.read::<u32>(img.word(f + 7)));
    let inv = div(k, cvt(frames.cast_signed()));
    let at = record(g, img, f, 0x3a, pc);
    g.write::<u32>(at, a);
    g.write::<u32>(at + 0x10, d.wrapping_mul(frames));
    g.write::<u32>(at + 4, 0);
    g.write::<u32>(at + 8, inv.to_bits());
    g.write::<u32>(at + 0x14, d);
    g.write::<u32>(at + 0x18, 2);
    Ret::eax(pc + 0x10)
}

/// `record_ramp` (101 bytes, 79 overlays).
pub const RECORD_RAMP: Sig = Sig {
    len: 101,
    hash: 0x36cc_9484_8232_1634,
    calls: &[],
    imports: &[],
};

/// 0x10007150: a colour ramp of the record: block `+0x70` (or `+0xb0`
/// when `[pc + 4]` is set), target `[pc + 8]`, step `[pc + 0xc]`, frames
/// `[pc + 0x10]`; mode 3.
pub fn record_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let words = [
        g.read::<u32>(pc + 8),
        g.read::<u32>(pc + 0xc),
        g.read::<u32>(pc + 0x10),
    ];
    let at = record(g, img, f, 0x23, pc);
    let block = if g.read::<u32>(pc + 4) == 0 {
        at + 0x70
    } else {
        at + 0xb0
    };
    ramp(g, img, f, 0x3e, block, words, 3);
    Ret::eax(pc + 0x14)
}

/// `record_fade` (118 bytes, 79 overlays).
pub const RECORD_FADE: Sig = Sig {
    len: 118,
    hash: 0x4c87_2a57_fe6e_39a2,
    calls: &[],
    imports: &[],
};

/// 0x100071d0: [`record_fade`] with the ramp pointing at its colour
/// (`+0x24` = the record's `+0x60`, or `+0x50` with `+0xb0`); mode 5.
pub fn record_fade_to<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let words = [
        g.read::<u32>(pc + 8),
        g.read::<u32>(pc + 0xc),
        g.read::<u32>(pc + 0x10),
    ];
    let at = record(g, img, f, 0x23, pc);
    let (block, colour) = if g.read::<u32>(pc + 4) == 0 {
        (at + 0x70, at + 0x60)
    } else {
        (at + 0xb0, at + 0x50)
    };
    g.write::<u32>(block + 0x24, colour);
    ramp(g, img, f, 0x46, block, words, 5);
    Ret::eax(pc + 0x14)
}

/// `record_fade_to` (129 bytes, 79 overlays).
pub const RECORD_FADE_TO: Sig = Sig {
    len: 129,
    hash: 0x286b_a7f0_3112_9ae5,
    calls: &[],
    imports: &[],
};

/// 0x10006ed0: sets (`u16` operand 1) or clears bit 0 of the object
/// `[pc + 4]`'s flags `+0x124`.
pub fn flag_bit<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let o = g.read::<u32>(pc + 4);
    let v = g.read::<u16>(o + 0x124);
    let v = if g.read::<u16>(pc + 2) == 1 {
        v | 1
    } else {
        v & 0xfffe
    };
    g.write::<u16>(o + 0x124, v);
    Ret::eax(pc + 8)
}

/// `flag_bit` (46 bytes, 79 overlays).
pub const FLAG_BIT: Sig = Sig {
    len: 46,
    hash: 0xf570_4bf2_b7f2_a976,
    calls: &[],
    imports: &[],
};

/// 0x10006e40: slot 313(1); when it answers non-zero, slot 378(1).
pub fn check_then<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let t = slot(g, img, f, 0x04, 313);
    if g.call(t, 0, &[1]).eax != 0 {
        let t = slot(g, img, f, 0x1a, 378);
        g.call(t, 0, &[1]);
    }
    Ret::eax(pc + 4)
}

/// `check_then` (51 bytes, 79 overlays).
pub const CHECK_THEN: Sig = Sig {
    len: 51,
    hash: 0x1240_66e4_011b_6c14,
    calls: &[],
    imports: &[],
};

/// 0x100075e0: slot 295 with the actor's number and `[pc + 4]`.
pub fn actor_call<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let i = i16_at(g, pc + 2);
    let n = who(g, img, f, (0x11, 0x18), i);
    let x = g.read::<u32>(pc + 4);
    let t = slot(g, img, f, 0x25, 295);
    g.call(t, 0, &[n, x]);
    Ret::eax(pc + 8)
}

/// `actor_call` (58 bytes, 79 overlays).
pub const ACTOR_CALL: Sig = Sig {
    len: 58,
    hash: 0x3c36_16a1_73c4_74a7,
    calls: &[],
    imports: &[],
};

/// 0x10007530: slot 719 finds the effect `[pc + 4]` (kind 6, the
/// operand) and slot 551 starts it with `[pc + 8]` and 1 (slot 719's
/// words stay below).
pub fn effect_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let i = i16_at(g, pc + 2);
    let x = g.read::<u32>(pc + 8);
    let e = g.read::<u32>(pc + 4);
    let t = slot(g, img, f, 0x11, 719);
    let r = g.call(t, 0, &[e, 6, i]).eax;
    let t = slot(g, img, f, 0x27, 551);
    g.call(t, 0, &[r, x, 1, e, 6, i]);
    Ret::eax(pc + 0xc)
}

/// `effect_start` (61 bytes, 79 overlays).
pub const EFFECT_START: Sig = Sig {
    len: 61,
    hash: 0x0b2a_1f38_e3ed_aae1,
    calls: &[],
    imports: &[],
};

/// 0x100074e0: slot 719 finds the effect as in [`effect_start`] and
/// slot 220 gives it the `i16` pair at `pc + 8`.
pub fn effect_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let i = i16_at(g, pc + 2);
    let a = i16_at(g, pc + 8);
    let b = i16_at(g, pc + 0xa);
    let e = g.read::<u32>(pc + 4);
    let t = slot(g, img, f, 0x17, 719);
    let r = g.call(t, 0, &[e, 6, i]).eax;
    let t = slot(g, img, f, 0x2c, 220);
    g.call(t, 0, &[r, a, b, e, 6, i]);
    Ret::eax(pc + 0xc)
}

/// `effect_pair` (67 bytes, 79 overlays).
pub const EFFECT_PAIR: Sig = Sig {
    len: 67,
    hash: 0x0aca_7cf3_d14d_feb6,
    calls: &[],
    imports: &[],
};

/// 0x10007700 and 0x10007740: one slot on the actor's object.
fn actor_obj_slot<G: Game>(g: &mut G, img: &Image, fr: &Frame, s: u32) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let i = i16_at(g, pc + 2);
    let o = obj_of(g, img, f, (0x11, 0x18, 0x23), i);
    let t = slot(g, img, f, 0x29, s);
    g.call(t, 0, &[o]);
    Ret::eax(pc + 4)
}

/// 0x10007700: slot 546 on the actor's object.
pub fn actor_546<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    actor_obj_slot(g, img, fr, 546)
}

/// `actor_546` (62 bytes, 79 overlays).
pub const ACTOR_546: Sig = Sig {
    len: 62,
    hash: 0xda00_369e_be47_ce3d,
    calls: &[],
    imports: &[],
};

/// 0x10007740: slot 525 on the actor's object.
pub fn actor_525<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    actor_obj_slot(g, img, fr, 525)
}

/// `actor_525` (62 bytes, 79 overlays).
pub const ACTOR_525: Sig = Sig {
    len: 62,
    hash: 0x6aeb_97b3_80af_f429,
    calls: &[],
    imports: &[],
};

/// 0x10007470: slot 6 (the actor's object, `[pc + 4]`), then slot 547
/// (`[pc + 4]` zero) or 548 on the object.
pub fn actor_show<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let i = i16_at(g, pc + 2);
    let b = g.read::<u32>(pc + 4);
    let o = obj_of(g, img, f, (0x17, 0x1e, 0x29), i);
    let t = slot(g, img, f, 0x2e, 6);
    g.call(t, 0, &[o, b]);
    let s = if b == 0 { 547 } else { 548 };
    let t = slot(g, img, f, 0x3a, s);
    g.call(t, 0, &[o]);
    Ret::eax(pc + 8)
}

/// `actor_show` (108 bytes, 79 overlays).
pub const ACTOR_SHOW: Sig = Sig {
    len: 108,
    hash: 0x6c31_b7a2_2a44_ed7b,
    calls: &[],
    imports: &[],
};

/// The operands shared by the commands that call slot 296 for a party
/// member and slot 257 (or another) on any other actor's object.
struct PartyOps {
    battle: u32,
    list: u32,
    me: u32,
    me_obj: u32,
    objs: u32,
}

/// The two ways of 0x10006f00 and 0x100073d0: outside a battle
/// (`*G_BATTLE` 0) an actor numbered below 0x20 (a party member) is
/// `Some(number)`; else `None` (its object is used).
fn party_member<G: Game>(g: &G, img: &Image, f: u32, o: &PartyOps, i: u32) -> Option<u32> {
    if g.read::<u32>(img.word(f + o.battle)) != 0 {
        return None;
    }
    let n = who(g, img, f, (o.me, o.list), i);
    (n.cast_signed() < 0x20).then_some(n)
}

/// 0x10006f00: shows (`[pc + 4]` non-zero) or hides the actor: slot 296
/// `(number, [pc + 4])` for a party member, else slot 257 `(object,
/// [pc + 4] == 0)`.
pub fn actor_visible<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let i = i16_at(g, pc + 2);
    let o = PartyOps {
        battle: 0x05,
        list: 0x0c,
        me: 0x1c,
        me_obj: 0x4f,
        objs: 0x5c,
    };
    let v = g.read::<u32>(pc + 4);
    if let Some(n) = party_member(g, img, f, &o, i) {
        let t = slot(g, img, f, 0x32, 296);
        g.call(t, 0, &[n, v]);
    } else {
        let obj = obj_of(g, img, f, (o.me_obj, o.list, o.objs), i);
        let t = slot(g, img, f, 0x6a, 257);
        g.call(t, 0, &[obj, u32::from(v == 0)]);
    }
    Ret::eax(pc + 8)
}

/// `actor_visible` (128 bytes, 79 overlays).
pub const ACTOR_VISIBLE: Sig = Sig {
    len: 128,
    hash: 0x7a58_cef3_f667_0d3a,
    calls: &[],
    imports: &[],
};

/// 0x100073d0: `[pc + 4]` (an `int`) times `K` (SSE) as a float: slot
/// 541 `(number, x, [pc + 8])` for a party member, else slot 38
/// `(object, x, [pc + 8])`.
pub fn actor_scaled<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let pc = fr.arg(g, 1);
    let f = fr.at;
    let k = f32::from_bits(g.read::<u32>(img.word(f + 0x24)));
    let x = mul(cvt(g.read::<i32>(pc + 4)), k).to_bits();
    let i = i16_at(g, pc + 2);
    let o = PartyOps {
        battle: 0x05,
        list: 0x0c,
        me: 0x2c,
        me_obj: 0x64,
        objs: 0x71,
    };
    let y = g.read::<u32>(pc + 8);
    if let Some(n) = party_member(g, img, f, &o, i) {
        let t = slot(g, img, f, 0x42, 541);
        g.call(t, 0, &[n, x, y]);
    } else {
        let obj = obj_of(g, img, f, (o.me_obj, o.list, o.objs), i);
        let t = slot(g, img, f, 0x80, 38);
        g.call(t, 0, &[obj, x, y]);
    }
    Ret::eax(pc + 0xc)
}

/// `actor_scaled` (149 bytes, 79 overlays).
pub const ACTOR_SCALED: Sig = Sig {
    len: 149,
    hash: 0x46c9_1ff2_5495_b33e,
    calls: &[],
    imports: &[],
};

/// 0x100063d0 (internal, `ecx` = a word passed on): the party's list:
/// slot 288 gives the actor list of kind 9 (when slot 293 says the
/// caller `*G_ME` is a party member) or 11, with its length into a local
/// (`[ebp - 4]` or `[ebp - 8]`); slot 296 runs for each entry with
/// `ecx`. Returns the last call's `eax`.
pub fn each_actor<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, esp: u32) -> u32 {
    let ebp = esp.wrapping_sub(4);
    let t = slot(g, img, f, 0x04, 293);
    let me = g.read::<u32>(img.word(f + 0x16));
    let (kind, count, o288, o296) = if g.call(t, 0, &[me]).eax != 0 {
        (9, ebp.wrapping_sub(4), 0x2a, 0x49)
    } else {
        (0xb, ebp.wrapping_sub(8), 0x6c, 0x8b)
    };
    let t = slot(g, img, f, o288, 288);
    let list = g.call(t, 0, &[kind, count]).eax;
    let mut eax = list;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(count) {
        let n = u32::from(g.read::<u8>(list.wrapping_add(i)));
        let t = slot(g, img, f, o296, 296);
        eax = g.call(t, 0, &[n, ecx]).eax;
        i += 1;
    }
    eax
}

/// `each_actor` (169 bytes, 79 overlays).
pub const EACH_ACTOR: Sig = Sig {
    len: 169,
    hash: 0x5c2f_63b9_c408_dfb1,
    calls: &[],
    imports: &[],
};

/// 0x100062b0 (internal, `ecx` = show): every actor of the DLL's list
/// (`G_COUNT` of them) shown or hidden: slot 296 `(number, ecx)` for a
/// party member outside a battle, else slot 257 `(object, ecx == 0)`.
/// Returns the last call's `eax` (`eax` as given for an empty list).
pub fn show_all<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32, eax_in: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut eax = eax_in;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x08)) {
        let list = g.read::<u32>(w(0x19));
        let mut party = None;
        if g.read::<u32>(w(0x12)) == 0 {
            let n = if i == u32::MAX {
                g.read::<u32>(w(0x20))
            } else {
                u32::from(g.read::<u8>(list.wrapping_add(i)))
            };
            if n.cast_signed() < 0x20 {
                party = Some(n);
            }
        }
        eax = if let Some(n) = party {
            let t = slot(g, img, f, 0x35, 296);
            g.call(t, 0, &[n, ecx]).eax
        } else {
            let o = if i == u32::MAX {
                g.read::<u32>(w(0x48))
            } else {
                let n = u32::from(g.read::<u8>(list.wrapping_add(i)));
                g.read::<u32>(w(0x55).wrapping_add(4 * n))
            };
            let t = slot(g, img, f, 0x62, 257);
            g.call(t, 0, &[o, u32::from(ecx == 0)]).eax
        };
        i = i.wrapping_add(1);
    }
    eax
}

/// `show_all` (126 bytes, 79 overlays).
pub const SHOW_ALL: Sig = Sig {
    len: 126,
    hash: 0x2fb4_a4cc_6e61_937a,
    calls: &[],
    imports: &[],
};

/// 0x10006640 (internal, `ecx` = the actor, `edx` = a block to fill,
/// the stack word its first): the block `{arg, -1, *G_ME, model, -1, 0,
/// 1, u16 2}`: the model from `*G_ME` (slot 413 in a battle, else 215),
/// then replaced by the actor's object (`*G_ME_OBJ` for -1). Returns
/// that object.
pub fn actor_block<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let out = fr.edx;
    let a = fr.arg(g, 0);
    g.write::<u32>(out, a);
    g.write::<u32>(out + 4, u32::MAX);
    let me = g.read::<u32>(w(0x14));
    g.write::<u32>(out + 8, me);
    let battle = g.read::<u32>(w(0x1d)) != 0;
    let t = slot(g, img, f, 0x24, if battle { 413 } else { 215 });
    let m = g.call(t, 0, &[me]).eax;
    g.write::<u32>(out + 0xc, m);
    g.write::<u32>(out + 0x10, u32::MAX);
    g.write::<u32>(out + 0x14, 0);
    g.write::<u32>(out + 0x18, 1);
    let battle = g.read::<u32>(w(0x5a)) != 0;
    g.write::<u16>(out + 0x1c, u16::from(battle));
    g.write::<u16>(out + 0x1c, 2);
    let o = obj_of(g, img, f, (0x7c, 0x88, 0x94), fr.ecx);
    g.write::<u32>(out + 0xc, o);
    o
}

/// `actor_block` (158 bytes, 79 overlays).
pub const ACTOR_BLOCK: Sig = Sig {
    len: 158,
    hash: 0xc664_1a1f_84e8_49fa,
    calls: &[],
    imports: &[],
};

/// `magic_0253` 0x100042c0 (internal, no arguments): runs the DLL's
/// script (state `S`: `+0x0` the pc, byte `+0x56` the mode, `u16 +0x54`
/// the wait, `+0x24`/`+0x28` callbacks): while the mode is 1 (running),
/// the command `G_COMMANDS[u16 *pc]` runs as `(S, pc)` and gives the next
/// pc; then by the mode: 0 (ended) returns 1; 2 counts the wait down by
/// the frame step `**G_STEP` (at or below it: wait 0 and running again;
/// 0xffff waits for ever); 3 calls the `+0x24` callback with `S`. After
/// modes 2 and 3 the `+0x28` callback, when set, gets `S`. Returns 0.
pub fn run<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let s = w(0x1c);
    let mut pc = g.read::<u32>(w(0x07));
    let mut mode = g.read::<u8>(w(0x01));
    while mode == 1 {
        let op = u32::from(g.read::<u16>(pc));
        let cmd = g.read::<u32>(w(0x17).wrapping_add(4 * op));
        pc = g.call(cmd, pc, &[s, pc]).eax;
        mode = g.read::<u8>(w(0x25));
    }
    g.write::<u32>(w(0x35), pc);
    let ecx = match mode {
        0 => return 1,
        2 => {
            let wait = g.read::<u16>(w(0x59));
            if wait == 0xffff {
                (pc & !0xffff) | u32::from(wait)
            } else {
                let step = g.read::<u32>(g.read::<u32>(w(0x68)));
                if u32::from(wait).cast_signed() > step.cast_signed() {
                    g.write::<u16>(w(0x8c), wait.wrapping_sub(step as u16));
                    (pc & !0xffff) | u32::from(wait.wrapping_sub(step as u16))
                } else {
                    g.write::<u16>(w(0x79), 0);
                    g.write::<u8>(w(0x7f), 1);
                    (pc & !0xffff) | u32::from(wait)
                }
            }
        }
        3 => {
            let cb = g.read::<u32>(w(0x4d));
            g.call(cb, pc, &[w(0x47)]);
            pc
        }
        _ => return 0,
    };
    let cb = g.read::<u32>(w(0x91));
    if cb != 0 {
        g.call(cb, ecx, &[w(0x9a)]);
    }
    0
}

/// `run` (172 bytes, 55 overlays).
pub const RUN: Sig = Sig {
    len: 172,
    hash: 0x54b3_65f2_2416_f5b8,
    calls: &[],
    imports: &[],
};

/// `magic_0162` 0x100061e0 (internal, `ecx` = an actor, `edx` and the
/// stack `(a, out)`): slot 237 `(the actor's object, edx, a, out)`, then
/// `out + 0xc` 1.0. Returns slot 237's `eax`.
pub fn actor_vector<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let o = obj_of(g, img, f, (0x09, 0x10, 0x1b), fr.ecx);
    let (a, out) = (fr.arg(g, 0), fr.arg(g, 1));
    let t = slot(g, img, f, 0x2a, 237);
    let r = g.call(t, 0, &[o, fr.edx, a, out]).eax;
    g.write::<u32>(out + 0xc, 0x3f80_0000);
    r
}

/// `actor_vector` (67 bytes, 19 overlays).
pub const ACTOR_VECTOR: Sig = Sig {
    len: 67,
    hash: 0x2e50_8a31_7726_93f1,
    calls: &[],
    imports: &[],
};

/// The wait commands of `magic_0446` 0x10004050 and 0x100021a0: when
/// the effect's counter pair at `off` (`*G_FX + off`, `+ off + 4`) is
/// equal, its `+ off + 8` becomes 1 and the object's mode `+0x56` 1
/// (returns the object); else returns the counter.
fn wait_pair<G: Game>(g: &mut G, img: &Image, fr: &Frame, off: u32) -> Ret {
    let fx = g.read::<u32>(img.word(fr.at + 0x05));
    let a = g.read::<u32>(fx + off);
    if a != g.read::<u32>(fx + off + 4) {
        return Ret::eax(a);
    }
    let obj = fr.arg(g, 0);
    g.write::<u32>(fx + off + 8, 1);
    g.write::<u8>(obj + 0x56, 1);
    Ret::eax(obj)
}

/// 0x10004050 of `magic_0446`: [`wait_pair`] at `+0xd74c`.
pub fn wait_pair_d74c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    wait_pair(g, img, fr, 0xd74c)
}

/// `wait_pair_d74c` (42 bytes, 15 overlays).
pub const WAIT_PAIR_D74C: Sig = Sig {
    len: 42,
    hash: 0x7424_4a42_05aa_a2a4,
    calls: &[],
    imports: &[],
};

/// 0x100021a0 of `magic_0446`: [`wait_pair`] at `+0x5c08`.
pub fn wait_pair_5c08<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    wait_pair(g, img, fr, 0x5c08)
}

/// `wait_pair_5c08` (42 bytes, 15 overlays).
pub const WAIT_PAIR_5C08: Sig = Sig {
    len: 42,
    hash: 0x16df_0e22_4bc3_cdbd,
    calls: &[],
    imports: &[],
};

/// `magic_0266` 0x100028d0 (internal, `ecx` = an actor, `edx` = a mode;
/// a /GS frame): with a mode, slot 210 `(the actor's object, mode)`;
/// without one, outside a battle, a party member's colour is read and
/// put back (slots 217 and 255 with a local vector, `[ebp - 0x14]`);
/// then slot 211 `(object, mode)`. Returns slot 211's `eax`.
pub fn actor_mode<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let (i, mode) = (fr.ecx, fr.edx);
    let o = obj_of(g, img, f, (0x21, 0x15, 0x2e), i);
    if mode != 0 {
        let t = slot(g, img, f, 0x37, 210);
        g.call(t, 0, &[o, mode]);
    } else if g.read::<u32>(img.word(f + 0x4c)) == 0 {
        let n = who(g, img, f, (0x56, 0x15), i);
        if n.cast_signed() < 0x20 {
            // push ebp; sub esp, 0x14: the vector is at [ebp - 0x14]
            let v = fr.esp.wrapping_sub(0x18);
            let t = slot(g, img, f, 0x6d, 217);
            g.call(t, 0, &[n, v]);
            let t = slot(g, img, f, 0x7f, 255);
            g.call(t, 0, &[n, v, n, v]);
        }
    }
    let t = slot(g, img, f, 0x91, 211);
    g.call(t, 0, &[o, mode]).eax
}

/// `actor_mode` (178 bytes, 7 overlays; the `rel32` is
/// `__security_check_cookie`).
pub const ACTOR_MODE: Sig = Sig {
    len: 178,
    hash: 0x5d2a_3a3e_6f53_b122,
    calls: &[(0xaa, Callee::Any)],
    imports: &[],
};

/// `magic_0053` 0x10001c30 (internal, `ecx` = an actor, `edx` = a word,
/// `xmm1`..`xmm3` a vector): slot 256 `(the actor's object, x, y, z,
/// edx)`. Returns its `eax`.
pub fn actor_slot256<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let o = obj_of(g, img, f, (0x06, 0x0d, 0x18), fr.ecx);
    let t = slot(g, img, f, 0x33, 256);
    let v = [fr.xmm[1] as u32, fr.xmm[2] as u32, fr.xmm[3] as u32];
    g.call(t, 0, &[o, v[0], v[1], v[2], fr.edx]).eax
}

/// `actor_slot256` (67 bytes, 6 overlays).
pub const ACTOR_SLOT256: Sig = Sig {
    len: 67,
    hash: 0x77c1_17d0_eab2_aba7,
    calls: &[],
    imports: &[],
};

/// 0x10008d50 of `magic_0644`: [`wait_pair`] at `+0x83f8`.
pub fn wait_pair_83f8<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    wait_pair(g, img, fr, 0x83f8)
}

/// `wait_pair_83f8` (42 bytes, 5 overlays).
pub const WAIT_PAIR_83F8: Sig = Sig {
    len: 42,
    hash: 0xf02d_ce35_cd94_1709,
    calls: &[],
    imports: &[],
};

/// 0x1000a940 of `magic_0644`: [`wait_pair`] at `+0xeab0`.
pub fn wait_pair_eab0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    wait_pair(g, img, fr, 0xeab0)
}

/// `wait_pair_eab0` (42 bytes, 5 overlays).
pub const WAIT_PAIR_EAB0: Sig = Sig {
    len: 42,
    hash: 0x1160_4b2f_f558_4ee7,
    calls: &[],
    imports: &[],
};
