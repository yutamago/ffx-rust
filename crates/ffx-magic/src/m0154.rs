//! `magic_0154.dll`'s own functions (besides the runtime and the OPU
//! host).

use crate::opu;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Table `+0x0` (0x10001690, cdecl `(arg)`): [`crate::l3_opu::start_up`]
/// with a slot 173 call `(data, 0x100a2940)` (operand `+0x67`) after the
/// registration, and a tail jump into [`opu::setup`] (`jmp`, not `call`:
/// the set-up's `this + 0x21200` is the result) instead of slot 816.
pub fn e1690<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let slot803 = slot_target(g, w(0x04), 803);
    let buffer = g.call(slot803, fr.ecx, &[0xfa000, 0x10]).eax;
    let flags = g.read::<u32>(w(0x1b));
    let slots = g.read::<u32>(w(0x21)).wrapping_add(4);
    g.write::<u32>(w(0x2c), buffer);
    let data = if g.read::<u8>(flags + 0x10) != 0 && g.read::<u8>(w(0x3b)) & 0x10 != 0 {
        g.read::<u32>(w(0x43))
    } else {
        w(0x4d)
    };
    g.write::<u32>(slots, data);
    let data = g.read::<u32>(slots);
    let slot718 = slot_target(g, w(0x52), 718);
    g.call(slot718, fr.ecx, &[data]);
    let name = w(0x67);
    let slot173 = slot_target(g, w(0x62), 173);
    g.call(slot173, fr.ecx, &[data, name, data]);
    let slot719 = slot_target(g, w(0x75), 719);
    let entry = g.call(slot719, fr.ecx, &[data, 1, 0, data, name, data]).eax;
    let slot715 = slot_target(g, w(0x94), 715);
    let obj = g
        .call(
            slot715,
            fr.ecx,
            &[
                1, entry, data, buffer, 0x200, 0, arg, data, 1, 0, data, name, data,
            ],
        )
        .eax;
    Ret::eax(opu::setup(g, img, img.rel32_target(f + 0xab), obj))
}

/// 0x10001690 (175 bytes, 1 overlay).
pub const E1690: Sig = Sig {
    len: 175,
    hash: 0x7a41_d92e_83b0_1f02,
    calls: &[(0xab, Callee::Sig(&opu::SETUP))],
    imports: &[],
};
