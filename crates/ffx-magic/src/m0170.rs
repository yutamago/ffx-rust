//! `magic_0170.dll`'s own function (table `+0x0`, the start-up of an OPU
//! effect): the same shape as `m0604::e1660`, with another choice of the
//! effect data (the battle flag is a global byte and the control block's
//! byte `+0x10` decides, where `e1660` reads the control word `+0x40`).
//! Its operands are this DLL's globals, read from the instructions.

use crate::opu;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// 0x10001690 (188 bytes, 2 overlays): allocates the OPU object's 0xfa000
/// bytes (slot 803, alignment 0x10) into the global at `+0x2c`; the effect
/// data is the global at `+0x43` when the control block's byte `+0x10` and
/// the battle flag byte at `+0x3b` (bit 4) are set, else the constant at
/// `+0x4d`; stores it through the slots pointer (`[+0x21] + 4`); calls slot
/// 718 with the data, slot 173 with the data and the constant at `+0x67`,
/// slot 719 with (data, 1, 0) for the effect's entry, slot 715 with the OPU
/// object's eleven words and [`opu::setup`] on the object; then tail-jumps
/// to slot 816.
///
/// The slots are cdecl and the original leaves each call's arguments on the
/// stack until the function's `add esp, 0x34`; each call-out gets the words
/// the callee reads. The `ecx` of a call is the entry's (as `m0604` passes),
/// except slot 718, which the original calls right after loading the control
/// block into `ecx`.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let slot803 = slot_target(g, w(0x04), 803);
    let buffer = g.call(slot803, fr.ecx, &[0xfa000, 0x10]).eax;
    let ctrl = g.read::<u32>(w(0x1b));
    let slots = g.read::<u32>(w(0x21)).wrapping_add(4);
    g.write::<u32>(w(0x2c), buffer);
    let data = if g.read::<u8>(ctrl + 0x10) != 0 && g.read::<u8>(w(0x3b)) & 0x10 != 0 {
        g.read::<u32>(w(0x43))
    } else {
        w(0x4d)
    };
    g.write::<u32>(slots, data);
    let slot718 = slot_target(g, w(0x52), 718);
    g.call(slot718, ctrl, &[data]);
    let slot173 = slot_target(g, w(0x62), 173);
    g.call(slot173, fr.ecx, &[data, w(0x67)]);
    let slot719 = slot_target(g, w(0x75), 719);
    let entry = g.call(slot719, fr.ecx, &[data, 1, 0]).eax;
    // the words the original leaves above the return address for slot 715
    // (its arguments are the first seven; the rest are the earlier pushes)
    let slot715 = slot_target(g, w(0x94), 715);
    let obj = g
        .call(
            slot715,
            fr.ecx,
            &[1, entry, data, buffer, 0x200, 0, arg, data, 1, 0, data],
        )
        .eax;
    opu::setup(g, img, img.rel32_target(f + 0xa8), obj);
    let slot816 = slot_target(g, w(0xad), 816);
    Ret::jump(slot816)
}

/// 0x10001690 (188 bytes).
pub const START: Sig = Sig {
    len: 188,
    hash: 0xee43_b8df_c597_efaa,
    calls: &[(0xa8, Callee::Sig(&opu::SETUP))],
    imports: &[],
};
