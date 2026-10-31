//! The own work list's smallest functions (round 4): flag and field
//! setters, single slot calls and float steps, one to three copies
//! each. Globals, fields, slots and constants are the instructions'
//! operands, read from each copy's code (`w(off)` = the word at `+off`).

use crate::Xmm;
use crate::runtime::slot_target;
use crate::sa_pool::give_back;
use crate::sa_script::run as script_run;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The slot of a `mov eax, [eax + 4 * slot]` whose `disp32` is at `at`.
fn slot_at(img: &Image, at: u32) -> u32 {
    img.word(at) / 4
}

/// The float at `at`.
fn f32_at(g: &impl crate::Mem, at: u32) -> f32 {
    f32::from_bits(g.read::<u32>(at))
}

/// `magic_0307` 0x10005220 (9 bytes): the halfword `G` (`+0x03`) gets
/// bit 0. `eax` as given.
pub fn set_bit_global<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let a = img.word(fr.at + 0x03);
    let v = g.read::<u16>(a);
    g.write::<u16>(a, v | 1);
    Ret::eax(fr.eax)
}

/// `magic_0269` 0x10001740 (11 bytes): `B = A` (operands `+0x06`,
/// `+0x01`). Returns the word.
pub fn copy_global<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let v = g.read::<u32>(img.word(fr.at + 0x01));
    g.write::<u32>(img.word(fr.at + 0x06), v);
    Ret::eax(v)
}

/// `magic_0671` 0x10001330 (13 bytes): `G = 1` (`+0x02`). Returns 0.
pub fn set_one_ret0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    g.write::<u32>(img.word(fr.at + 0x02), 1);
    Ret::eax(0)
}

/// `magic_0563` 0x100050f0 (14 bytes): the halfword `*G + 0x714` (`G`
/// at `+0x01`, the field at `+0x08`) gets bit 0. Returns `*G`.
pub fn set_bit_field<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let p = g.read::<u32>(img.word(fr.at + 0x01));
    let a = p.wrapping_add(img.word(fr.at + 0x08));
    let v = g.read::<u16>(a);
    g.write::<u16>(a, v | 1);
    Ret::eax(p)
}

/// `magic_0563` 0x10005100 (18 bytes): the halfword `*G + 0x714` (`G`
/// at `+0x01`, the field at `+0x0d`) loses bit 0. Returns `*G`.
pub fn clear_bit_field<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let p = g.read::<u32>(img.word(fr.at + 0x01));
    let a = p.wrapping_add(img.word(fr.at + 0x0d));
    let v = g.read::<u16>(a);
    g.write::<u16>(a, v & 0xfffe);
    Ret::eax(p)
}

/// `magic_0434` 0x100095c0 (18 bytes; 2 copies): whether the word at
/// `*G + D` (`G` at `+0x02`, `D` at `+0x0a`) is 0.
pub fn is_zero_field<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let p = g.read::<u32>(img.word(fr.at + 0x02));
    let v = g.read::<u32>(p.wrapping_add(img.word(fr.at + 0x0a)));
    Ret::eax(u32::from(v == 0))
}

/// `magic_0577` 0x1000eca0 (18 bytes, a register argument): the slot at
/// `+0x08` called with the caller's `ecx`. Returns its `eax`.
pub fn slot_ecx<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let s = slot_target(g, img.word(fr.at + 0x01), slot_at(img, fr.at + 0x08));
    Ret::eax(g.call(s, fr.ecx, &[fr.ecx]).eax)
}

/// `magic_0577` 0x1000ec80 (19 bytes): the slot at `+0x09` with the
/// `push imm8` at `+0x06`. Returns its `eax`.
pub fn slot_imm8<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let s = slot_target(g, img.word(fr.at + 0x01), slot_at(img, fr.at + 0x09));
    let k = crate::o4_parts::disp8(img, fr.at + 0x06);
    Ret::eax(g.call(s, fr.ecx, &[k]).eax)
}

/// `magic_0519` 0x10004e40 (23 bytes): the slot at `+0x09` (287) with
/// the three `push imm8` at `+0x10`, `+0x0e`, `+0x06` (first word
/// first). Returns its `eax`.
pub fn slot_imm8x3<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let d = |off: u32| crate::o4_parts::disp8(img, f + off);
    let s = slot_target(g, img.word(f + 0x01), slot_at(img, f + 0x09));
    Ret::eax(g.call(s, fr.ecx, &[d(0x10), d(0x0e), d(0x06)]).eax)
}

/// `(obj)` (`magic_0673` 0x10001b40, 25 bytes): `obj + E = *G + D` (`G`
/// at `+0x04`, `D` at `+0x0a`, `E` at `+0x13`). Returns `obj`.
pub fn copy_to_arg<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0a)));
    let obj = fr.arg(g, 0);
    g.write::<u32>(obj.wrapping_add(w(0x13)), v);
    Ret::eax(obj)
}

/// `magic_0350` 0x10006300 (26 bytes): two words of `*G` set (`mov
/// dword [eax + d], imm32` at `+0x05` and `+0x0f`). Returns `*G`.
pub fn two_stores<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    for at in [0x05, 0x0f] {
        g.write::<u32>(p.wrapping_add(w(at + 2)), w(at + 6));
    }
    Ret::eax(p)
}

/// `(a)` (`magic_0150` 0x1000eaf0, 28 bytes): slot 306 `(7, 0, a)`
/// (the slot at `+0x0f`, the immediates at `+0x14`, `+0x0c`). Returns
/// its `eax`.
pub fn slot_7_0_arg<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let d = |off: u32| crate::o4_parts::disp8(img, f + off);
    let a = fr.arg(g, 0);
    let s = slot_target(g, img.word(f + 0x07), slot_at(img, f + 0x0f));
    Ret::eax(g.call(s, fr.ecx, &[d(0x14), d(0x0c), a]).eax)
}

/// `(obj)` (`magic_0353` 0x10002440, 28 bytes): `obj + 0x104` = the
/// callback at `+0x0c`, `obj + 0x120` = the immediate at `+0x16`.
/// Returns `obj`.
pub fn set_callback<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    g.write::<u32>(obj.wrapping_add(w(0x08)), w(0x0c));
    g.write::<u32>(obj.wrapping_add(w(0x12)), w(0x16));
    Ret::eax(obj)
}

/// `(obj)` (`magic_0195` 0x100019c0, 29 bytes): the slot at `+0x0c`
/// with the `push imm8` at `+0x09`, then `obj` gets bit 2. Returns
/// `obj`.
pub fn slot_then_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = slot_target(g, img.word(f + 0x04), slot_at(img, f + 0x0c));
    g.call(s, fr.ecx, &[crate::o4_parts::disp8(img, f + 0x09)]);
    let obj = fr.arg(g, 0);
    let v = g.read::<u32>(obj);
    g.write::<u32>(obj, v | 4);
    Ret::eax(obj)
}

/// `magic_0483` 0x10002650 (29 bytes): the slot at `+0x13` with `(*B,
/// *A)` (`B` at `+0x0d`, `A` at `+0x02`). Returns its `eax`.
pub fn slot_two_globals<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let a = g.read::<u32>(w(0x02));
    let b = g.read::<u32>(w(0x0d));
    let s = slot_target(g, w(0x07), slot_at(img, fr.at + 0x13));
    Ret::eax(g.call(s, fr.ecx, &[b, a]).eax)
}

/// `magic_0568` 0x10002de0 (30 bytes; 2 copies): the float at `*G + D`
/// (`D` at `+0x09`) plus the constant at `+0x11`, stored back (`+0x19`).
/// Returns `*G`.
pub fn add_const<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    let x = add(f32_at(g, p.wrapping_add(w(0x09))), f32_at(g, w(0x11)));
    g.write::<u32>(p.wrapping_add(w(0x19)), x.to_bits());
    Ret::eax(p)
}

/// `magic_0671` 0x10001310 (31 bytes): the script runs ([`script_run`],
/// the `rel32` at `+0x01`); when it ended (1) slot 214 `(*H)` ends the
/// effect (`H` at `+0x11`) and its `eax` is returned, else the script's.
pub fn script_then_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let r = script_run(g, img, img.rel32_target(fr.at + 0x01));
    if r != 1 {
        return Ret::eax(r);
    }
    let h = g.read::<u32>(w(0x11));
    let s = slot_target(g, w(0x0b), slot_at(img, fr.at + 0x17));
    Ret::eax(g.call(s, fr.ecx, &[h]).eax)
}

/// `magic_0263` 0x10008980 (33 bytes, internal: `ecx` the argument):
/// slot 277 `(ecx)` (`+0x0c`), its ST0 result rounded to a float
/// (`fstp dword`) and returned in `xmm0`.
pub fn slot_float<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let s = slot_target(g, img.word(fr.at + 0x05), slot_at(img, fr.at + 0x0c));
    let r = g.call(s, fr.ecx, &[fr.ecx]);
    let x = r.st0.unwrap_or(0.0) as f32;
    Ret::xmm0(Xmm::f32(x), r.eax)
}

/// `(_, b)` (`magic_0483` 0x100026a0, 33 bytes, `edx` too): slot 237
/// `(*G, edx, 0, b)` (`G` at `+0x16`, the slot at `+0x0f`). Returns its
/// `eax`.
pub fn slot237_edx<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let b = fr.arg(g, 1);
    let h = g.read::<u32>(w(0x16));
    let s = slot_target(g, w(0x07), slot_at(img, fr.at + 0x0f));
    Ret::eax(g.call(s, fr.ecx, &[h, fr.edx, 0, b]).eax)
}

/// `magic_0661` 0x100015b0 (33 bytes): `G = ` slot 260 `(0x14)` (the
/// slot at `+0x09`, the immediate at `+0x06`, `G` at `+0x1a`). Returns
/// whether it is not 0.
pub fn slot_keep_nonzero<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = slot_target(g, img.word(f + 0x01), slot_at(img, f + 0x09));
    let v = g
        .call(s, fr.ecx, &[crate::o4_parts::disp8(img, f + 0x06)])
        .eax;
    g.write::<u32>(img.word(f + 0x1a), v);
    Ret::eax(u32::from(v != 0))
}

/// `(a)` (`magic_0343` 0x10006970, 34 bytes): slot 237 `(*G, 1, 0, a)`
/// (`G` at `+0x17`, the slot at `+0x0f`, the immediates at `+0x14`,
/// `+0x0c`). Returns its `eax`.
pub fn slot237_arg<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let d = |off: u32| crate::o4_parts::disp8(img, f + off);
    let a = fr.arg(g, 0);
    let h = g.read::<u32>(img.word(f + 0x17));
    let s = slot_target(g, img.word(f + 0x07), slot_at(img, f + 0x0f));
    Ret::eax(g.call(s, fr.ecx, &[h, d(0x14), d(0x0c), a]).eax)
}

/// `magic_0443` 0x10001ab0 (34 bytes): the slot at `+0x09` with the
/// `push imm8` at `+0x06`, then `G = K` (operands `+0x14`, `+0x18`).
/// Returns 1.
pub fn slot_then_store_ret1<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = slot_target(g, img.word(f + 0x01), slot_at(img, f + 0x09));
    g.call(s, fr.ecx, &[crate::o4_parts::disp8(img, f + 0x06)]);
    g.write::<u32>(img.word(f + 0x14), img.word(f + 0x18));
    Ret::eax(1)
}

/// `magic_0499` 0x100013c0 (35 bytes): the slot at `+0x0d` with `(*G)`
/// (`+0x07`), then `G = 0` (`+0x18`). Returns 0.
pub fn slot_then_clear<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let v = g.read::<u32>(img.word(f + 0x07));
    let s = slot_target(g, img.word(f + 0x01), slot_at(img, f + 0x0d));
    g.call(s, fr.ecx, &[v]);
    g.write::<u32>(img.word(f + 0x18), 0);
    Ret::eax(0)
}

/// `magic_0165` 0x100036f0 (36 bytes): `P = *G`; the halfword at `P +
/// 0x130 * P[0x5f1c] + 0x124` and the one at `P + 0x5e4` get bit 0 (the
/// fields at `+0x08`, `+0x0e`, `+0x16`, `+0x1e`). Returns the record's
/// offset.
pub fn set_bits_indexed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x02));
    let i = g.read::<u32>(p.wrapping_add(w(0x08)));
    let off = i.wrapping_mul(w(0x0e));
    let a = off.wrapping_add(p).wrapping_add(w(0x16));
    let v = g.read::<u16>(a);
    g.write::<u16>(a, v | 1);
    let b = p.wrapping_add(w(0x1e));
    let v = g.read::<u16>(b);
    g.write::<u16>(b, v | 1);
    Ret::eax(off)
}

/// `magic_0165` 0x10003720 (39 bytes): [`set_bits_indexed`]'s halfwords
/// lose bit 0 (the fields at `+0x0d`, `+0x13`, `+0x1b`, `+0x22`).
/// Returns the record's offset.
pub fn clear_bits_indexed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x02));
    let i = g.read::<u32>(p.wrapping_add(w(0x0d)));
    let off = i.wrapping_mul(w(0x13));
    let a = off.wrapping_add(p).wrapping_add(w(0x1b));
    let v = g.read::<u16>(a);
    g.write::<u16>(a, v & 0xfffe);
    let b = p.wrapping_add(w(0x22));
    let v = g.read::<u16>(b);
    g.write::<u16>(b, v & 0xfffe);
    Ret::eax(off)
}

/// `(obj)` (`magic_0540` 0x10001000, 36 bytes): slot 38 `(*obj, 0,
/// 0x14)` (the slot at `+0x19`, the `push imm8` at `+0x07`). Returns its
/// `eax`.
pub fn slot_obj_word<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let v = g.read::<u32>(obj);
    let s = slot_target(g, img.word(f + 0x13), slot_at(img, f + 0x19));
    let k = crate::o4_parts::disp8(img, f + 0x07);
    Ret::eax(g.call(s, fr.ecx, &[v, 0, k]).eax)
}

/// `magic_0263` 0x10006f40 (37 bytes, `edx` the argument): `A = K`,
/// `B = edx`, `C = 0`, `D = 0` (operands `+0x02`/`+0x06`, `+0x0c`,
/// `+0x12`, `+0x1c`). `eax` as given.
pub fn set_four<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    g.write::<u32>(w(0x02), w(0x06));
    g.write::<u32>(w(0x0c), fr.edx);
    g.write::<u32>(w(0x12), 0);
    g.write::<u32>(w(0x1c), 0);
    Ret::eax(fr.eax)
}

/// `magic_0162` 0x10003990 (38 bytes; 2 copies): the float at `*G + A`
/// plus the constant at `+0x11`, stored at `*G + A` and `*G + B` (`A` at
/// `+0x09`/`+0x19`, `B` at `+0x21`). Returns `*G`.
pub fn add_const_twice<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    let x = add(f32_at(g, p.wrapping_add(w(0x09))), f32_at(g, w(0x11)));
    g.write::<u32>(p.wrapping_add(w(0x19)), x.to_bits());
    g.write::<u32>(p.wrapping_add(w(0x21)), x.to_bits());
    Ret::eax(p)
}

/// `magic_0304` 0x10002df0 (38 bytes): [`add_const_twice`] with `subss`.
pub fn sub_const_twice<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x01));
    let x = sub(f32_at(g, p.wrapping_add(w(0x09))), f32_at(g, w(0x11)));
    g.write::<u32>(p.wrapping_add(w(0x19)), x.to_bits());
    g.write::<u32>(p.wrapping_add(w(0x21)), x.to_bits());
    Ret::eax(p)
}

/// `(obj)` (`magic_0525` 0x10001750, 38 bytes): the slot at `+0x15`
/// with the word at `*G + D` (`G` at `+0x04`, `D` at `+0x0a`), then
/// `obj` gets bit 2. Returns `obj`.
pub fn field_slot_then_flag<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x04));
    let v = g.read::<u32>(p.wrapping_add(w(0x0a)));
    let s = slot_target(g, w(0x0f), slot_at(img, fr.at + 0x15));
    g.call(s, fr.ecx, &[v]);
    let obj = fr.arg(g, 0);
    let x = g.read::<u32>(obj);
    g.write::<u32>(obj, x | 4);
    Ret::eax(obj)
}

/// `(node)` (`magic_0638` 0x10003170, 39 bytes; 2 copies): the script
/// runs ([`script_run`], `+0x0b`); when it ended (1), `*G + D` = 1 (`G`
/// at `+0x06`, `D` at `+0x19`) and the node goes back to the pool
/// ([`give_back`], a tail jump, `+0x20`). Returns the script's result
/// otherwise.
pub fn script_then_give_back<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let p = g.read::<u32>(w(0x06));
    let r = script_run(g, img, img.rel32_target(fr.at + 0x0b));
    if r != 1 {
        return Ret::eax(r);
    }
    let node = fr.arg(g, 0);
    g.write::<u32>(p.wrapping_add(w(0x19)), r);
    Ret::eax(give_back(g, img, img.rel32_target(fr.at + 0x20), node))
}

/// `magic_0661` 0x10001500 (39 bytes): `A = ` the slot at `+0x09` with
/// the `push imm8` at `+0x06` (`A` at `+0x10`), `B = K` (`+0x19`,
/// `+0x1d`). Returns 1.
pub fn slot_keep_store_ret1<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = slot_target(g, img.word(f + 0x01), slot_at(img, f + 0x09));
    let v = g
        .call(s, fr.ecx, &[crate::o4_parts::disp8(img, f + 0x06)])
        .eax;
    g.write::<u32>(img.word(f + 0x10), v);
    g.write::<u32>(img.word(f + 0x19), img.word(f + 0x1d));
    Ret::eax(1)
}

/// [`set_bit_global`] in `magic_0307` (0x10005220, 9 bytes).
pub const SET_BIT_GLOBAL_0307_5220: Sig = Sig {
    len: 9,
    hash: 0x5382_53c3_9de5_05e1,
    calls: &[],
    imports: &[],
};

/// [`copy_global`] in `magic_0269` (0x10001740, 11 bytes).
pub const COPY_GLOBAL_0269_1740: Sig = Sig {
    len: 11,
    hash: 0x5112_b55d_d338_59ea,
    calls: &[],
    imports: &[],
};

/// [`set_one_ret0`] in `magic_0671` (0x10001330, 13 bytes).
pub const SET_ONE_RET0_0671_1330: Sig = Sig {
    len: 13,
    hash: 0xace6_db7e_2c0d_69f0,
    calls: &[],
    imports: &[],
};

/// [`set_bit_field`] in `magic_0563` (0x100050f0, 14 bytes).
pub const SET_BIT_FIELD_0563_50F0: Sig = Sig {
    len: 14,
    hash: 0xe8cf_7f28_eb50_51d4,
    calls: &[],
    imports: &[],
};

/// [`clear_bit_field`] in `magic_0563` (0x10005100, 18 bytes).
pub const CLEAR_BIT_FIELD_0563_5100: Sig = Sig {
    len: 18,
    hash: 0x39f1_7f78_c4ed_dc3b,
    calls: &[],
    imports: &[],
};

/// [`is_zero_field`] in `magic_0434` (0x100095c0, 18 bytes).
pub const IS_ZERO_FIELD_0434_95C0: Sig = Sig {
    len: 18,
    hash: 0x6546_47a7_6c3b_ef74,
    calls: &[],
    imports: &[],
};

/// [`is_zero_field`] in `magic_0610` (0x1000d490, 18 bytes).
pub const IS_ZERO_FIELD_0610_D490: Sig = Sig {
    len: 18,
    hash: 0x47f0_73ee_69f6_5d88,
    calls: &[],
    imports: &[],
};

/// [`slot_ecx`] in `magic_0577` (0x1000eca0, 18 bytes).
pub const SLOT_ECX_0577_ECA0: Sig = Sig {
    len: 18,
    hash: 0x93c9_9cb7_8d9b_a7f3,
    calls: &[],
    imports: &[],
};

/// [`slot_imm8`] in `magic_0577` (0x1000ec80, 19 bytes).
pub const SLOT_IMM8_0577_EC80: Sig = Sig {
    len: 19,
    hash: 0x85fd_0965_76c0_4592,
    calls: &[],
    imports: &[],
};

/// [`slot_imm8x3`] in `magic_0519` (0x10004e40, 23 bytes).
pub const SLOT_IMM8X3_0519_4E40: Sig = Sig {
    len: 23,
    hash: 0xb224_0cec_bad8_be51,
    calls: &[],
    imports: &[],
};

/// [`copy_to_arg`] in `magic_0673` (0x10001b40, 25 bytes).
pub const COPY_TO_ARG_0673_1B40: Sig = Sig {
    len: 25,
    hash: 0x8d24_1129_81a4_3448,
    calls: &[],
    imports: &[],
};

/// [`two_stores`] in `magic_0350` (0x10006300, 26 bytes).
pub const TWO_STORES_0350_6300: Sig = Sig {
    len: 26,
    hash: 0xf2d9_5d95_0e0e_71e5,
    calls: &[],
    imports: &[],
};

/// [`slot_7_0_arg`] in `magic_0150` (0x1000eaf0, 28 bytes).
pub const SLOT_7_0_ARG_0150_EAF0: Sig = Sig {
    len: 28,
    hash: 0x9184_60f8_d089_1d3a,
    calls: &[],
    imports: &[],
};

/// [`set_callback`] in `magic_0353` (0x10002440, 28 bytes).
pub const SET_CALLBACK_0353_2440: Sig = Sig {
    len: 28,
    hash: 0x68f5_df3a_a6aa_2e5d,
    calls: &[],
    imports: &[],
};

/// [`slot_then_flag`] in `magic_0195` (0x100019c0, 29 bytes).
pub const SLOT_THEN_FLAG_0195_19C0: Sig = Sig {
    len: 29,
    hash: 0x6fb2_a193_7ea1_6e1c,
    calls: &[],
    imports: &[],
};

/// [`slot_two_globals`] in `magic_0483` (0x10002650, 29 bytes).
pub const SLOT_TWO_GLOBALS_0483_2650: Sig = Sig {
    len: 29,
    hash: 0x9dfb_41fe_2872_4b23,
    calls: &[],
    imports: &[],
};

/// [`add_const`] in `magic_0568` (0x10002de0, 30 bytes).
pub const ADD_CONST_0568_2DE0: Sig = Sig {
    len: 30,
    hash: 0xa2ac_6381_2923_7df9,
    calls: &[],
    imports: &[],
};

/// [`add_const`] in `magic_0673` (0x10002040, 30 bytes).
pub const ADD_CONST_0673_2040: Sig = Sig {
    len: 30,
    hash: 0x7ce5_25f3_bc6b_6045,
    calls: &[],
    imports: &[],
};

/// [`script_then_end`] in `magic_0671` (0x10001310, 31 bytes).
pub const SCRIPT_THEN_END_0671_1310: Sig = Sig {
    len: 31,
    hash: 0xfe6f_7c35_4604_d670,
    calls: &[(0x1, Callee::Sig(&crate::sa_script::RUN))],
    imports: &[],
};

/// [`slot_float`] in `magic_0263` (0x10008980, 33 bytes).
pub const SLOT_FLOAT_0263_8980: Sig = Sig {
    len: 33,
    hash: 0x6365_a865_9ccf_4b46,
    calls: &[],
    imports: &[],
};

/// [`slot237_edx`] in `magic_0483` (0x100026a0, 33 bytes).
pub const SLOT237_EDX_0483_26A0: Sig = Sig {
    len: 33,
    hash: 0xee82_73ee_1365_3432,
    calls: &[],
    imports: &[],
};

/// [`slot_keep_nonzero`] in `magic_0661` (0x100015b0, 33 bytes).
pub const SLOT_KEEP_NONZERO_0661_15B0: Sig = Sig {
    len: 33,
    hash: 0x9dc1_54a1_f00c_7957,
    calls: &[],
    imports: &[],
};

/// [`slot237_arg`] in `magic_0343` (0x10006970, 34 bytes).
pub const SLOT237_ARG_0343_6970: Sig = Sig {
    len: 34,
    hash: 0x07c6_b612_1a49_65f3,
    calls: &[],
    imports: &[],
};

/// [`slot_then_store_ret1`] in `magic_0443` (0x10001ab0, 34 bytes).
pub const SLOT_THEN_STORE_RET1_0443_1AB0: Sig = Sig {
    len: 34,
    hash: 0xd369_4f8a_5b1a_a880,
    calls: &[],
    imports: &[],
};

/// [`slot_then_clear`] in `magic_0499` (0x100013c0, 35 bytes).
pub const SLOT_THEN_CLEAR_0499_13C0: Sig = Sig {
    len: 35,
    hash: 0xe04f_8452_45f7_a893,
    calls: &[],
    imports: &[],
};

/// [`set_bits_indexed`] in `magic_0165` (0x100036f0, 36 bytes).
pub const SET_BITS_INDEXED_0165_36F0: Sig = Sig {
    len: 36,
    hash: 0xf2ec_a600_b265_cd06,
    calls: &[],
    imports: &[],
};

/// [`clear_bits_indexed`] in `magic_0165` (0x10003720, 39 bytes).
pub const CLEAR_BITS_INDEXED_0165_3720: Sig = Sig {
    len: 39,
    hash: 0xc7a3_a415_3f0e_e80f,
    calls: &[],
    imports: &[],
};

/// [`slot_obj_word`] in `magic_0540` (0x10001000, 36 bytes).
pub const SLOT_OBJ_WORD_0540_1000: Sig = Sig {
    len: 36,
    hash: 0xc027_9245_0702_3248,
    calls: &[],
    imports: &[],
};

/// [`set_four`] in `magic_0263` (0x10006f40, 37 bytes).
pub const SET_FOUR_0263_6F40: Sig = Sig {
    len: 37,
    hash: 0xb30c_bc9a_4632_3114,
    calls: &[],
    imports: &[],
};

/// [`add_const_twice`] in `magic_0162` (0x10003990, 38 bytes).
pub const ADD_CONST_TWICE_0162_3990: Sig = Sig {
    len: 38,
    hash: 0x4f1e_0180_e8a3_9c49,
    calls: &[],
    imports: &[],
};

/// [`add_const_twice`] in `magic_0586` (0x100035a0, 38 bytes).
pub const ADD_CONST_TWICE_0586_35A0: Sig = Sig {
    len: 38,
    hash: 0xdc84_0aa2_dff8_b329,
    calls: &[],
    imports: &[],
};

/// [`sub_const_twice`] in `magic_0304` (0x10002df0, 38 bytes).
pub const SUB_CONST_TWICE_0304_2DF0: Sig = Sig {
    len: 38,
    hash: 0x9faa_96dd_1e93_d86c,
    calls: &[],
    imports: &[],
};

/// [`field_slot_then_flag`] in `magic_0525` (0x10001750, 38 bytes).
pub const FIELD_SLOT_THEN_FLAG_0525_1750: Sig = Sig {
    len: 38,
    hash: 0x0f3a_71c8_41fe_9575,
    calls: &[],
    imports: &[],
};

/// [`script_then_give_back`] in `magic_0638` (0x10003170, 39 bytes).
pub const SCRIPT_THEN_GIVE_BACK_0638_3170: Sig = Sig {
    len: 39,
    hash: 0xc06e_87c2_ff15_c656,
    calls: &[
        (0xb, Callee::Sig(&crate::sa_script::RUN)),
        (0x20, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`script_then_give_back`] in `magic_0689` (0x10003170, 39 bytes).
pub const SCRIPT_THEN_GIVE_BACK_0689_3170: Sig = Sig {
    len: 39,
    hash: 0x3e1b_c36a_d3b6_2a10,
    calls: &[
        (0xb, Callee::Sig(&crate::sa_script::RUN)),
        (0x20, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`slot_keep_store_ret1`] in `magic_0661` (0x10001500, 39 bytes).
pub const SLOT_KEEP_STORE_RET1_0661_1500: Sig = Sig {
    len: 39,
    hash: 0xe5fa_27ff_2cf2_aa1f,
    calls: &[],
    imports: &[],
};
