//! Single functions of the own work list (round 4): a fade counter, a
//! table of halfwords cleared before a slot call and a slot chain that
//! ends in a tail jump. Operands are read from the code (`w(off)` = the
//! word at `+off`).

use crate::runtime::slot_target;
use crate::sa_misc::register_parts;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `(a)` (`magic_0673` 0x10001c70, 88 bytes): `a[0xe4]` = the word
/// `+0x5e80` of `*G` (`G` at `+0x04`), the count `a[0x120]` down by one;
/// a count of 1 to 15 sets the four halfwords `a[0x70..0x78]` to
/// `(count << 12) / 16` (returned), one of 0 or less sets bit 0x4000 of
/// the halfword `a[0x124]` (0x4000 returned); 16 and more are returned
/// as they are.
pub fn count_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = fr.arg(g, 0);
    let v = g.read::<u32>(g.read::<u32>(w(0x04)).wrapping_add(w(0x0d)));
    let n = g.read::<u32>(a.wrapping_add(0x120)).wrapping_sub(1);
    g.write::<u32>(a.wrapping_add(0x120), n);
    g.write::<u32>(a.wrapping_add(0xe4), v);
    let n = n.cast_signed();
    if n >= 0x10 {
        return Ret::eax(n.cast_unsigned());
    }
    if n > 0 {
        let x = (n << 12) / 16;
        for off in [0x76, 0x74, 0x72, 0x70] {
            g.write::<u16>(a.wrapping_add(off), x.cast_unsigned() as u16);
        }
        return Ret::eax(x.cast_unsigned());
    }
    let flags = a.wrapping_add(0x124);
    let bit = w(0x4b);
    let v = g.read::<u16>(flags) | bit as u16;
    g.write::<u16>(flags, v);
    Ret::eax(bit)
}

/// No arguments (`magic_0519` 0x10002cf0, 89 bytes): two blocks (from
/// `+0x03`, 0x790 bytes apart, `+0x30`) of 12 halfwords each 0xa0 bytes
/// apart (`+0x27`) set to 0, then slot 337 (`+0x43`) with the immediates
/// at `+0x4d`, `+0x48`, `+0x3d`. Returns the slot's `eax`.
pub fn clear_halves_slot<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut block = w(0x03);
    for _ in 0..w(0x08) {
        let mut p = block;
        for _ in 0..w(0x13) {
            g.write::<u16>(p, 0);
            p = p.wrapping_add(w(0x27));
        }
        block = block.wrapping_add(w(0x30));
    }
    let s = slot_target(g, w(0x38), w(0x43) / 4);
    Ret::eax(g.call(s, fr.ecx, &[w(0x4d), w(0x48), w(0x3d)]).eax)
}

/// No arguments (`magic_0446` 0x10004d30, 89 bytes): when the halfword
/// `*X` (`+0x03`) is 0, slot 213 (`+0x19`) with `(*A, 0)` (`A` at
/// `+0x13`); then `*C` (`+0x31`) = the immediate at `+0x35`, slot 219
/// (`+0x3b`) with `(*B, 0)` (`B` at `+0x2b`) and slot 257 (`+0x4b`) with
/// its `eax`, 0 and the words still on the stack; then a tail jump
/// (`+0x55`) to [`register_parts`] with that slot's `eax`.
pub fn slot_chain_jump<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u16>(w(0x03)) == 0 {
        let a = g.read::<u32>(w(0x13));
        let s = slot_target(g, w(0x0b), w(0x19) / 4);
        g.call(s, fr.ecx, &[a, 0]);
    }
    let b = g.read::<u32>(w(0x2b));
    g.write::<u32>(w(0x31), w(0x35));
    let s = slot_target(g, w(0x23), w(0x3b) / 4);
    let r = g.call(s, fr.ecx, &[b, 0]).eax;
    let s = slot_target(g, w(0x45), w(0x4b) / 4);
    let r = g.call(s, fr.ecx, &[r, 0, b, 0]).eax;
    Ret::eax(register_parts(g, img, img.rel32_target(f + 0x55), r))
}

/// [`count_fade`] in `magic_0673` (0x10001c70, 88 bytes).
pub const COUNT_FADE_0673_1C70: Sig = Sig {
    len: 88,
    hash: 0x8f90_1046_3b80_25c3,
    calls: &[],
    imports: &[],
};

/// [`clear_halves_slot`] in `magic_0519` (0x10002cf0, 89 bytes).
pub const CLEAR_HALVES_SLOT_0519_2CF0: Sig = Sig {
    len: 89,
    hash: 0xa0b6_a884_7b48_a749,
    calls: &[],
    imports: &[],
};

/// [`slot_chain_jump`] in `magic_0446` (0x10004d30, 89 bytes).
pub const SLOT_CHAIN_JUMP_0446_4D30: Sig = Sig {
    len: 89,
    hash: 0xd768_4774_c2e0_9d11,
    calls: &[(0x55, Callee::Sig(&crate::sa_misc::REGISTER_PARTS))],
    imports: &[],
};
