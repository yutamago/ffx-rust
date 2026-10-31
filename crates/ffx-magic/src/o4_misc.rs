//! Single functions of the own work list on static records (round 4).
//! Operands are read from the code (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// No arguments (`magic_0343` 0x10006900, 97 bytes): the static model's
/// flag step: with bit 0 of the flags `F` (`+0x02`), slot 36 (`+0x18`)
/// with `(*A, K)` (`A` at `+0x12`) and `F | 0x200 | 0x120`; else slot 228
/// (`+0x39`) with `(*B, K)` (`B` at `+0x33`) and `F | 0x120` (`K` at
/// `+0x0d`); then `C` = the immediate at `+0x56` (`+0x52`) and the
/// flags stored (`+0x5c`). Returns the slot's `eax` with the flags in its
/// low half (`mov ax`).
pub fn static_flag_slots<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = disp8(img, f + 0x0d);
    let (r, flags) = if g.read::<u8>(w(0x02)) & 1 != 0 {
        let a = g.read::<u32>(w(0x12));
        let s = slot_target(g, w(0x08), w(0x18) / 4);
        let r = g.call(s, fr.ecx, &[a, k]).eax;
        let flags = g.read::<u16>(w(0x20)) | w(0x25) as u16;
        (r, flags | w(0x49) as u16)
    } else {
        let b = g.read::<u32>(w(0x33));
        let s = slot_target(g, w(0x08), w(0x39) / 4);
        let r = g.call(s, fr.ecx, &[b, k]).eax;
        (r, g.read::<u16>(w(0x41)) | w(0x49) as u16)
    };
    g.write::<u32>(w(0x52), w(0x56));
    g.write::<u16>(w(0x5c), flags);
    Ret::eax(r & 0xffff_0000 | u32::from(flags))
}

/// A vector set (`magic_0343` 0x100069e0, 105 bytes, internal: `edx` =
/// the vector, `xmm2` a float): unless bit 0 of the flags `F` (`+0x06`)
/// is set, slot 217 (`+0x27`) with `(*B, A)` (`B` at `+0x21`, `A` at
/// `+0x1b`), the vector's `+0xc` cleared, slot 248 (`+0x35`) with `(C, A,
/// vec)` (`C` at `+0x3f`, `A` at `+0x3a`; the first call's words after
/// them), the flags get bit 3 (`+0x57`) and `X` = the float (`+0x60`).
/// Returns slot 248's `eax`, or `eax` as given when the bit is set.
pub fn vector_set<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u8>(w(0x06)) & 1 != 0 {
        return Ret::eax(fr.eax);
    }
    let vec = fr.edx;
    let b = g.read::<u32>(w(0x21));
    let s = slot_target(g, w(0x16), w(0x27) / 4);
    g.call(s, fr.ecx, &[b, w(0x1b)]);
    let s = slot_target(g, w(0x2e), w(0x35) / 4);
    g.write::<u32>(vec.wrapping_add(0xc), 0);
    let r = g.call(s, fr.ecx, &[w(0x3f), w(0x3a), vec, b, w(0x1b)]).eax;
    let flags = g.read::<u16>(w(0x57));
    g.write::<u16>(w(0x57), flags | 8);
    g.write::<u32>(w(0x60), fr.xmm[2] as u32);
    Ret::eax(r)
}

/// No arguments (`magic_0351` 0x10002450, 83 bytes): slot 271 (`+0x09`)
/// with `(K)` (`+0x06`); four floats their immediates (`mov dword
/// [addr], imm32` from `+0x1d`, 10 bytes each); the float slot 790
/// (`+0x19`, no words), its ST0 rounded to a float into `Y` (`+0x49`).
/// Returns 1.
pub fn reset_then_float<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = slot_target(g, w(0x01), w(0x09) / 4);
    g.call(s, fr.ecx, &[disp8(img, f + 0x06)]);
    let s = slot_target(g, w(0x10), w(0x19) / 4);
    for k in 0..4 {
        let insn = f + 0x1d + 10 * k;
        g.write::<u32>(img.word(insn + 2), img.word(insn + 6));
    }
    let x = g.call(s, fr.ecx, &[]).st0.unwrap_or(0.0) as f32;
    g.write::<u32>(w(0x49), x.to_bits());
    Ret::eax(1)
}

/// [`static_flag_slots`] in `magic_0343` (0x10006900, 97 bytes).
pub const STATIC_FLAG_SLOTS_0343_6900: Sig = Sig {
    len: 97,
    hash: 0x18e1_0587_874e_355f,
    calls: &[],
    imports: &[],
};

/// [`vector_set`] in `magic_0343` (0x100069e0, 105 bytes).
pub const VECTOR_SET_0343_69E0: Sig = Sig {
    len: 105,
    hash: 0x5390_249b_ad56_78dc,
    calls: &[],
    imports: &[],
};

/// [`reset_then_float`] in `magic_0351` (0x10002450, 83 bytes).
pub const RESET_THEN_FLOAT_0351_2450: Sig = Sig {
    len: 83,
    hash: 0x0eae_963d_ee32_a462,
    calls: &[],
    imports: &[],
};
