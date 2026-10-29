//! The opcodes of the effect script the `magic_0162` family runs (79
//! DLLs), ported by the shared-B list. Each is a vtable entry, cdecl
//! `(ctx, ip)`, and returns the next instruction (`ip` plus its size) or
//! a jump target. `ctx` keeps a register file at `+0x2c` (words), a loop
//! stack (counters, halfwords at `+0x4c`; return points at `+0x4`; the
//! depth byte at `+0x57`) and a word at `+0x28`. The DLL keeps a table of
//! 0x148-byte records (its "current" index in a global). Each doc names
//! the address in `magic_0162`, the size and how many DLLs carry the
//! opcode.

use crate::runtime::slot_target;
use crate::sb_sse::{cvt, div, mul};
use crate::{Frame, Game, Image, Ret, Sig};

const fn sig(len: u32, hash: u64) -> Sig {
    Sig {
        len,
        hash,
        calls: &[],
        imports: &[],
    }
}

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

/// `(ctx, ip)`.
fn args<G: Game>(g: &G, fr: &Frame) -> (u32, u32) {
    (fr.arg(g, 0), fr.arg(g, 1))
}

/// Jump: `[ip + 4]`. `0x10006c40` (11 bytes, 79 DLLs).
pub fn op_jump<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    Ret::eax(g.read::<u32>(ip + 4))
}

/// [`op_jump`].
pub const OP_JUMP: Sig = sig(11, 0x7645_72ba_95d5_bf26);

/// Jump to the address in register `u16 [ip + 2]`. `0x10006c50` (19
/// bytes, 79 DLLs).
pub fn op_jump_reg<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (ctx, ip) = args(g, fr);
    let r = u32::from(g.read::<u16>(ip + 2));
    Ret::eax(g.read::<u32>(ctx + 0x2c + 4 * r))
}

/// [`op_jump_reg`].
pub const OP_JUMP_REG: Sig = sig(19, 0xbb0b_3ea4_6e86_fe54);

/// `ctx +0x28 = [ip + 4]`. `0x10006d60` (20 bytes, 79 DLLs).
pub fn op_set_28<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (ctx, ip) = args(g, fr);
    g.write::<u32>(ctx + 0x28, g.read::<u32>(ip + 4));
    Ret::eax(ip + 8)
}

/// [`op_set_28`].
pub const OP_SET_28: Sig = sig(20, 0xb825_a08c_d750_524e);

/// Register `u16 [ip + 2]` = the next instruction's address (a label).
/// `0x10006da0` (22 bytes, 79 DLLs). Returns `ip + 4`.
pub fn op_label<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (ctx, ip) = args(g, fr);
    let r = u32::from(g.read::<u16>(ip + 2));
    g.write::<u32>(ctx + 0x2c + 4 * r, ip + 4);
    Ret::eax(ip + 4)
}

/// [`op_label`].
pub const OP_LABEL: Sig = sig(22, 0xedbc_de63_add6_1062);

/// Calls the function at `[ip + 4]` with `ctx` (`ecx` as it came).
/// `0x10006d20` (24 bytes, 79 DLLs).
pub fn op_call<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (ctx, ip) = args(g, fr);
    let target = g.read::<u32>(ip + 4);
    g.call(target, fr.ecx, &[ctx]);
    Ret::eax(ip + 8)
}

/// [`op_call`].
pub const OP_CALL: Sig = sig(24, 0x4683_805b_e646_3acb);

/// Register `u16 [ip + 2]` = `[ip + 4]`. `0x10006d80` (27 bytes, 79
/// DLLs).
pub fn op_set_reg<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (ctx, ip) = args(g, fr);
    let r = u32::from(g.read::<u16>(ip + 2));
    g.write::<u32>(ctx + 0x2c + 4 * r, g.read::<u32>(ip + 4));
    Ret::eax(ip + 8)
}

/// [`op_set_reg`].
pub const OP_SET_REG: Sig = sig(27, 0x1a37_3fdf_71e9_0c98);

/// Slot 543 `(u16 [ip + 2])`. `0x100075b0` (34 bytes, 79 DLLs).
pub fn op_543<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let a = u32::from(g.read::<u16>(ip + 2));
    let s = slot_target(g, img.word(fr.at + 0x0d), 543);
    g.call(s, fr.ecx, &[a]);
    Ret::eax(ip + 4)
}

/// [`op_543`].
pub const OP_543: Sig = sig(34, 0x7db3_28d3_a5b6_c5ec);

/// Slot 371 `(u16 [ip + 2], [ip + 4])`. `0x100076d0` (37 bytes, 79
/// DLLs).
pub fn op_371<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let a = [u32::from(g.read::<u16>(ip + 2)), g.read::<u32>(ip + 4)];
    let s = slot_target(g, img.word(fr.at + 0x10), 371);
    g.call(s, fr.ecx, &a);
    Ret::eax(ip + 8)
}

/// [`op_371`].
pub const OP_371: Sig = sig(37, 0x5734_ede3_dc69_2c8a);

/// Loop start: pushes the count `u16 [ip + 2]` and the body's address
/// (`ip + 4`). `0x10006c70` (40 bytes, 79 DLLs).
pub fn op_loop<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (ctx, ip) = args(g, fr);
    let n = g.read::<u16>(ip + 2);
    let d = u32::from(g.read::<u8>(ctx + 0x57));
    g.write::<u16>(ctx + 0x4c + 2 * d, n);
    let d = u32::from(g.read::<u8>(ctx + 0x57));
    g.write::<u32>(ctx + 4 + 4 * d, ip + 4);
    let b = g.read::<u8>(ctx + 0x57);
    g.write::<u8>(ctx + 0x57, b.wrapping_add(1));
    Ret::eax(ip + 4)
}

/// [`op_loop`].
pub const OP_LOOP: Sig = sig(40, 0xa5b5_a200_2756_e6b4);

/// Slot 222 `([ip + 4], u16 [ip + 8], u16 [ip + 0xa], 1)`. `0x10006e10`
/// (44 bytes, 79 DLLs).
pub fn op_222<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let a = [
        g.read::<u32>(ip + 4),
        u32::from(g.read::<u16>(ip + 8)),
        u32::from(g.read::<u16>(ip + 0xa)),
        1,
    ];
    let s = slot_target(g, img.word(fr.at + 0x14), 222);
    g.call(s, fr.ecx, &a);
    Ret::eax(ip + 0xc)
}

/// [`op_222`].
pub const OP_222: Sig = sig(44, 0xc706_99f1_d759_e903);

/// Slot 550 `(1)` when slot 549 `(1)` says so. `0x10007570` (51 bytes,
/// 79 DLLs).
pub fn op_549_550<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let s = slot_target(g, img.word(fr.at + 0x04), 549);
    if g.call(s, fr.ecx, &[1]).eax != 0 {
        let s = slot_target(g, img.word(fr.at + 0x1a), 550);
        g.call(s, fr.ecx, &[1]);
    }
    let (_, ip) = args(g, fr);
    Ret::eax(ip + 4)
}

/// [`op_549_550`].
pub const OP_549_550: Sig = sig(51, 0x321f_4579_d31c_0f49);

/// Slot 271 `(1)` when `[ip + 4]` is 1, else slot 267 `(1)`.
/// `0x100078b0` (54 bytes, 79 DLLs).
pub fn op_271_267<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let slot = if g.read::<u32>(ip + 4) == 1 { 271 } else { 267 };
    let s = slot_target(g, img.word(fr.at + 0x04), slot);
    g.call(s, fr.ecx, &[1]);
    Ret::eax(ip + 8)
}

/// [`op_271_267`].
pub const OP_271_267: Sig = sig(54, 0x64a6_ad52_0afb_a7bd);

/// Selects record `u16 [ip + 2]` of the DLL's table (its current index
/// at operand `+0x04`, written through `+0x29`): the old one's flag (the
/// table's word at operand `+0x17`) is cleared, the new one's (`+0x2f`)
/// set. `0x10007010` (60 bytes, 79 DLLs).
pub fn op_select<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let (_, ip) = args(g, fr);
    let old = g.read::<u32>(w(0x04)).wrapping_mul(0x148);
    g.write::<u32>(w(0x17).wrapping_add(old), 0);
    let n = u32::from(g.read::<u16>(ip + 2));
    g.write::<u32>(w(0x29), n);
    g.write::<u32>(w(0x2f).wrapping_add(n.wrapping_mul(0x148)), 1);
    Ret::eax(ip + 4)
}

/// [`op_select`].
pub const OP_SELECT: Sig = sig(60, 0x8a58_4f1b_fe4b_0e5c);

/// Slot 306 `(7, [ip + 8], 719([ip + 4], 9, i16 [ip + 2]))`.
/// `0x10007920` (61 bytes, 79 DLLs).
pub fn op_306<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let n = i32::from(g.read::<u16>(ip + 2).cast_signed()).cast_unsigned();
    let b = g.read::<u32>(ip + 8);
    let s719 = slot_target(g, img.word(fr.at + 0x11), 719);
    let e = g.call(s719, fr.ecx, &[g.read::<u32>(ip + 4), 9, n]).eax;
    let s306 = slot_target(g, img.word(fr.at + 0x24), 306);
    g.call(s306, fr.ecx, &[7, b, e]);
    Ret::eax(ip + 0xc)
}

/// [`op_306`].
pub const OP_306: Sig = sig(61, 0x6fc2_c7a5_613b_9402);

/// Loop end: the innermost counter (0xffff: forever) goes down; while it
/// is not 0 the loop's body address is returned, else the loop is popped
/// and `ip + 4` returned. `0x10006ca0` (62 bytes, 79 DLLs).
pub fn op_loop_end<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let (ctx, ip) = args(g, fr);
    let d = u32::from(g.read::<u8>(ctx + 0x57).wrapping_sub(1));
    let n = g.read::<u16>(ctx + 0x4c + 2 * d);
    if n != 0xffff {
        g.write::<u16>(ctx + 0x4c + 2 * d, n.wrapping_sub(1));
    }
    if g.read::<u16>(ctx + 0x4c + 2 * d) == 0 {
        let b = g.read::<u8>(ctx + 0x57);
        g.write::<u8>(ctx + 0x57, b.wrapping_sub(1));
        return Ret::eax(ip + 4);
    }
    Ret::eax(g.read::<u32>(ctx + 4 + 4 * d))
}

/// [`op_loop_end`].
pub const OP_LOOP_END: Sig = sig(62, 0x1f89_7dde_0693_d14e);

/// The record of `u16 [ip + 2]` (the compare with -1, which a halfword
/// never is, would pick the current one, operand `cur`) at the table
/// base operand `base`.
fn record<G: Game>(g: &G, img: &Image, f: u32, ip: u32, cur: u32, base: u32) -> u32 {
    let mut n = u32::from(g.read::<u16>(ip + 2));
    let c = g.read::<u32>(img.word(f + cur));
    if n == u32::MAX {
        n = c;
    }
    n.wrapping_mul(0x148).wrapping_add(img.word(f + base))
}

/// A record (base operand `+0x2b`): `+0x60..+0x68` = the three words
/// `[ip + 4]` points at. `0x100070d0` (65 bytes, 79 DLLs).
pub fn op_set_vector<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let v = g.read::<u32>(ip + 4);
    let rec = record(g, img, fr.at, ip, 0x13, 0x2b);
    let (y, z) = (g.read::<u32>(v + 4), g.read::<u32>(v + 8));
    g.write::<u32>(rec + 0x60, g.read::<u32>(v));
    g.write::<u32>(rec + 0x64, y);
    g.write::<u32>(rec + 0x68, z);
    Ret::eax(ip + 8)
}

/// [`op_set_vector`].
pub const OP_SET_VECTOR: Sig = sig(65, 0xbdf7_44e9_ecfa_c402);

/// The character of actor `i16 [ip + 2]` (-1: the effect's own, operand
/// `ops[0]`; else the character table `ops[2]` at the actor's byte,
/// table pointer `ops[1]`).
fn actor_chr<G: Game>(g: &G, img: &Image, f: u32, ip: u32, ops: [u32; 3]) -> u32 {
    let i = i32::from(g.read::<u16>(ip + 2).cast_signed());
    if i == -1 {
        g.read::<u32>(img.word(f + ops[0]))
    } else {
        let bytes = g.read::<u32>(img.word(f + ops[1]));
        let b = u32::from(g.read::<u8>(bytes.wrapping_add(i.cast_unsigned())));
        g.read::<u32>(img.word(f + ops[2]).wrapping_add(4 * b))
    }
}

/// `Ch_SetHideEffect(chr, [ip + 4])` (slot 257) for the actor at `+2`
/// (operands `+0x11`, `+0x18`, `+0x23`). `0x10006f80` (65 bytes, 79
/// DLLs).
pub fn op_hide<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let chr = actor_chr(g, img, fr.at, ip, [0x11, 0x18, 0x23]);
    let s = slot_target(g, img.word(fr.at + 0x2c), 257);
    g.call(s, fr.ecx, &[chr, g.read::<u32>(ip + 4)]);
    Ret::eax(ip + 8)
}

/// [`op_hide`].
pub const OP_HIDE: Sig = sig(65, 0xfab3_8364_1a49_bf80);

/// Each actor of group 9 (slot 288 `(9, &count)` gives the actor bytes
/// and their count, a frame word at `esp - 8`) gets slot 296 `(actor,
/// [ip + 4])`. `0x10007960` (89 bytes, 79 DLLs).
pub fn op_show_group<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let count = fr.esp.wrapping_sub(8);
    let v = g.read::<u32>(ip + 4);
    let s288 = slot_target(g, img.word(fr.at + 0x12), 288);
    let bytes = g.call(s288, fr.ecx, &[9, count]).eax;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(count).cast_signed() {
        let a = u32::from(g.read::<u8>(bytes.wrapping_add(i)));
        let s296 = slot_target(g, img.word(fr.at + 0x35), 296);
        g.call(s296, fr.ecx, &[a, v]);
        i += 1;
    }
    let (_, ip) = args(g, fr);
    Ret::eax(ip + 8)
}

/// [`op_show_group`].
pub const OP_SHOW_GROUP: Sig = sig(89, 0x87c1_d93d_8a1f_b3e0);

/// A record (base operand `+0x3a`): `+0x0` = `[ip + 4]`, `+0x4` = 0,
/// `+0x8` = K / `[ip + 0xc]` (K at `+0x07`), `+0x10` = `[ip + 8] *
/// [ip + 0xc]`, `+0x14` = `[ip + 8]`, `+0x18` = 1. `0x100072d0` (101
/// bytes, 79 DLLs).
pub fn op_timer<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let k = f32_at(g, img.word(fr.at + 0x07));
    let c = g.read::<u32>(ip + 0xc);
    let a = g.read::<u32>(ip + 4);
    let d = g.read::<u32>(ip + 8);
    let rec = record(g, img, fr.at, ip, 0x2a, 0x3a);
    let step = div(k, cvt(c));
    g.write::<u32>(rec, a);
    g.write::<u32>(rec + 0x10, d.wrapping_mul(c));
    g.write::<u32>(rec + 4, 0);
    g.write::<u32>(rec + 8, step.to_bits());
    g.write::<u32>(rec + 0x14, d);
    g.write::<u32>(rec + 0x18, 1);
    Ret::eax(ip + 0x10)
}

/// [`op_timer`].
pub const OP_TIMER: Sig = sig(101, 0xa838_3efa_e010_80f5);

/// A record (base `+0x45`): `+0x128`/`+0x12c` = the integers `[ip + 4]`
/// and `[ip + 8]` times K (`+0x37`, `+0x3f`), `+0x130` = `u16 [ip +
/// 0xc]`, `+0x134` = `u16 [ip + 0xe]`, byte `+0x144` = 1. `0x10007620`
/// (114 bytes, 79 DLLs).
pub fn op_set_scaled<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let (_, ip) = args(g, fr);
    let x = cvt(g.read::<u32>(ip + 4));
    let y = cvt(g.read::<u32>(ip + 8));
    let e = u32::from(g.read::<u16>(ip + 0xe));
    let c = u32::from(g.read::<u16>(ip + 0xc));
    let rec = record(g, img, fr.at, ip, 0x23, 0x45);
    let x = mul(x, f32_at(g, w(0x37)));
    let y = mul(y, f32_at(g, w(0x3f)));
    g.write::<u32>(rec + 0x134, e);
    g.write::<u32>(rec + 0x128, x.to_bits());
    g.write::<u32>(rec + 0x12c, y.to_bits());
    g.write::<u32>(rec + 0x130, c);
    g.write::<u8>(rec + 0x144, 1);
    Ret::eax(ip + 0x10)
}

/// [`op_set_scaled`].
pub const OP_SET_SCALED: Sig = sig(114, 0x4b40_4c80_6e42_6182);

/// A record's fade (base `+0x27`; `[ip + 4]` picks the part at `+0xb0`,
/// else `+0x70`): `+0x28` = `[ip + 0xc]`, `+0x20` = `[ip + 8]`, `+0x38`
/// = `a`, `+0x34` = `a * b`, `+0x2c` = 0, `+0x30` = K / (a * b) (K at
/// `+0x44`), `+0x3c` = 4 (`a`, `b` = `[ip + 0x10]`, `[ip + 0x14]`).
/// `0x100077a0` (127 bytes, 79 DLLs).
pub fn op_fade_4<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let c = g.read::<u32>(ip + 0xc);
    let b8 = g.read::<u32>(ip + 8);
    let rec = record(g, img, fr.at, ip, 0x17, 0x27);
    let a = g.read::<u32>(ip + 0x10);
    let sel = g.read::<u32>(ip + 4);
    let b = g.read::<u32>(ip + 0x14);
    let p = rec + if sel == 0 { 0x70 } else { 0xb0 };
    let k = f32_at(g, img.word(fr.at + 0x44));
    g.write::<u32>(p + 0x28, c);
    let ab = a.wrapping_mul(b);
    let step = div(k, cvt(ab));
    g.write::<u32>(p + 0x38, a);
    g.write::<u32>(p + 0x20, b8);
    g.write::<u32>(p + 0x34, ab);
    g.write::<u32>(p + 0x2c, 0);
    g.write::<u32>(p + 0x30, step.to_bits());
    g.write::<u32>(p + 0x3c, 4);
    Ret::eax(ip + 0x18)
}

/// [`op_fade_4`].
pub const OP_FADE_4: Sig = sig(127, 0xed61_f83b_bcb5_02a2);

/// Slot 256 `(chr, x, y, z, [ip + 0x10])` for the actor at `+2`
/// ([`actor_chr`] with operands `+0x3d`, `+0x44`, `+0x4f`), the integers
/// `[ip + 4..0xc]` times K (`+0x07`). `0x10007340` (129 bytes, 79 DLLs).
pub fn op_256<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let k = f32_at(g, img.word(fr.at + 0x07));
    let v: [f32; 3] = [4, 8, 0xc].map(|o| mul(cvt(g.read::<u32>(ip + o)), k));
    let chr = actor_chr(g, img, fr.at, ip, [0x3d, 0x44, 0x4f]);
    let s = slot_target(g, img.word(fr.at + 0x6c), 256);
    let a = [
        chr,
        v[0].to_bits(),
        v[1].to_bits(),
        v[2].to_bits(),
        g.read::<u32>(ip + 0x10),
    ];
    g.call(s, fr.ecx, &a);
    Ret::eax(ip + 0x14)
}

/// [`op_256`].
pub const OP_256: Sig = sig(129, 0x9039_c3cb_ccff_73b3);

/// [`op_fade_4`]'s kind 6 (base `+0x26`, K `+0x4c`): the part also gets
/// at `+0x24` the record's `+0x50` (part `+0xb0`) or `+0x60` (part
/// `+0x70`); `+0x20` = `[ip + 8]`, which the original keeps in its own
/// `ip` argument word (overwritten), `+0x3c` = 6. `0x10007820` (141
/// bytes, 79 DLLs).
pub fn op_fade_6<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (_, ip) = args(g, fr);
    let b8 = g.read::<u32>(ip + 8);
    let c = g.read::<u32>(ip + 0xc);
    let rec = record(g, img, fr.at, ip, 0x17, 0x26);
    let a = g.read::<u32>(ip + 0x10);
    let sel = g.read::<u32>(ip + 4);
    let b = g.read::<u32>(ip + 0x14);
    g.write::<u32>(fr.esp + 8, b8);
    let (p, v) = if sel == 0 {
        (rec + 0x70, rec + 0x60)
    } else {
        (rec + 0xb0, rec + 0x50)
    };
    let k = f32_at(g, img.word(fr.at + 0x4c));
    g.write::<u32>(p + 0x24, v);
    g.write::<u32>(p + 0x20, g.read::<u32>(fr.esp + 8));
    let ab = a.wrapping_mul(b);
    let step = div(k, cvt(ab));
    g.write::<u32>(p + 0x38, a);
    g.write::<u32>(p + 0x28, c);
    g.write::<u32>(p + 0x34, ab);
    g.write::<u32>(p + 0x2c, 0);
    g.write::<u32>(p + 0x30, step.to_bits());
    g.write::<u32>(p + 0x3c, 6);
    Ret::eax(ip + 0x18)
}

/// [`op_fade_6`].
pub const OP_FADE_6: Sig = sig(141, 0x403d_9c62_9cee_9f81);

/// Shows the effect's actors except one (`ecx` = the argument for slot
/// 296): when slot 293 says the DLL's actor (operand `+0x14`) has no
/// group, `MsSetChrVisible(1, ecx)`-like slot 296 `(1, ecx)`; else every
/// actor of group 0xb (slot 288 `(0xb, &count)`, the count a frame word
/// at `esp - 8`) other than that one gets slot 296 `(actor, ecx)`.
/// Returns slot 296's `eax`, or the DLL's actor.
///
/// `0x10006330` (146 bytes, `ecx` the argument, 79 DLLs).
pub fn show_others<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, v: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let s293 = slot_target(g, w(0x04), 293);
    let me = g.read::<u32>(w(0x14));
    if g.call(s293, v, &[me]).eax == 0 {
        let s296 = slot_target(g, w(0x24), 296);
        return g.call(s296, v, &[1, v]).eax;
    }
    let count = esp.wrapping_sub(8);
    let s288 = slot_target(g, w(0x42), 288);
    let bytes = g.call(s288, v, &[0xb, count]).eax;
    let me = g.read::<u32>(w(0x56));
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(count).cast_signed() {
        let c = u32::from(g.read::<u8>(bytes.wrapping_add(i)));
        if c != me {
            let s296 = slot_target(g, w(0x6f), 296);
            g.call(s296, v, &[c, v]);
        }
        i += 1;
    }
    me
}

/// [`show_others`].
pub const SHOW_OTHERS: Sig = sig(146, 0x6e19_0a70_e3b1_3b17);

/// Sets an actor record up (`edx` = the record, `ecx` = the actor index,
/// -1 for the effect's own; the stack word = the owner): `+0x0` the
/// owner, `+0x4` the index, `+0x8` the actor number (operand `+0x14` for
/// -1, else the DLL's actor byte table `+0x1b`), `+0xc` its character
/// (slot 215, or slot 413 while the DLL's mode word `+0x28` is set),
/// `+0x10` = -1, `+0x14` = 0, `+0x18` = 1, `+0x1c` (halfword) = 1 in that
/// mode (`+0x63`), else 0. Returns the character (0 when not in the
/// mode).
///
/// `0x100065c0` (122 bytes, `ecx`/`edx` and one stack word, 79 DLLs).
pub fn actor_record<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    index: u32,
    rec: u32,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(rec, g.read::<u32>(esp + 4));
    g.write::<u32>(rec + 4, index);
    let n = if index == u32::MAX {
        g.read::<u32>(w(0x14))
    } else {
        let t = g.read::<u32>(w(0x1b));
        u32::from(g.read::<u8>(t.wrapping_add(index)))
    };
    g.write::<u32>(rec + 8, n);
    let slot = if g.read::<u32>(w(0x28)) == 0 {
        215
    } else {
        413
    };
    let s = slot_target(g, w(0x2f), slot);
    let chr = g.call(s, index, &[n]).eax;
    g.write::<u32>(rec + 0xc, chr);
    g.write::<u32>(rec + 0x10, u32::MAX);
    g.write::<u32>(rec + 0x14, 0);
    g.write::<u32>(rec + 0x18, 1);
    if g.read::<u32>(w(0x63)) == 0 {
        g.write::<u16>(rec + 0x1c, 0);
        0
    } else {
        g.write::<u16>(rec + 0x1c, 1);
        chr
    }
}

/// [`actor_record`].
pub const ACTOR_RECORD: Sig = sig(122, 0x622d_6e10_7012_b144);
