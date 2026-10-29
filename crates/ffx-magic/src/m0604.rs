//! `magic_0604.dll`'s own functions (besides the runtime and the OPU
//! host): the overlay table's `+0x0` (start-up) and `+0x4` (shutdown).
//! Both are shared with 3 and 13 other overlays.

use crate::opu;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Table `+0x0` (0x10001660, cdecl `(arg)`): allocates the OPU object's
/// 0xfa000 bytes (slot 803, alignment 0x10) into `G_OBJ`; picks the
/// effect data (`G_SLOT + 4` gets the control block's `+0xa8` when the
/// battle flag byte `+0x10` is set and the control word `+0x40` has bit
/// 4, else the default at `+0x52`'s operand), registers it
/// (`op_oef2_set2`, slot 718) and looks it up (`op_oef2_a_get(data, 1,
/// 0)`, slot 719); creates the OPU object (slot 715 with `(1, entry,
/// data, buffer, 0x200, 0, arg)`), sets it up ([`opu::setup`]) and
/// tail-jumps to slot 816 with the caller's stack.
pub fn e1660<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let slot803 = slot_target(g, w(0x04), 803);
    let ctrl = g.read::<u32>(w(0x13));
    let buffer = g.call(slot803, fr.ecx, &[0xfa000, 0x10]).eax;
    let flags = g.read::<u32>(w(0x22));
    let slots = g.read::<u32>(w(0x28)).wrapping_add(4);
    g.write::<u32>(w(0x33), buffer);
    let data = if g.read::<u8>(flags + 0x10) != 0 && g.read::<u8>(ctrl + 0x40) & 0x10 != 0 {
        g.read::<u32>(ctrl + 0xa8)
    } else {
        w(0x52)
    };
    g.write::<u32>(slots, data);
    let data = g.read::<u32>(slots);
    let slot718 = slot_target(g, w(0x57), 718);
    g.call(slot718, fr.ecx, &[data]);
    let slot719 = slot_target(g, w(0x67), 719);
    let entry = g.call(slot719, fr.ecx, &[data, 1, 0, data]).eax;
    let slot715 = slot_target(g, w(0x86), 715);
    let obj = g
        .call(
            slot715,
            fr.ecx,
            &[1, entry, data, buffer, 0x200, 0, arg, data, 1, 0, data],
        )
        .eax;
    opu::setup(g, img, img.rel32_target(f + 0x9a), obj);
    let slot816 = slot_target(g, w(0x9f), 816);
    Ret::jump(slot816)
}

/// 0x10001660 (175 bytes, 4 overlays).
pub const E1660: Sig = Sig {
    len: 175,
    hash: 0x87b6_b010_bebe_fc11,
    calls: &[(0x9a, Callee::Sig(&opu::SETUP))],
    imports: &[],
};

/// Table `+0x4` (0x10001820): clears the battle flag byte `+0x11`, frees
/// the OPU object's memory (slot 804) and tail-jumps to slot 817.
pub fn e1820<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let flags = g.read::<u32>(w(0x01));
    g.write::<u8>(flags + 0x11, 0);
    let obj = g.read::<u32>(w(0x0a));
    if obj != 0 {
        let slot804 = slot_target(g, w(0x14), 804);
        g.call(slot804, fr.ecx, &[obj]);
    }
    let table = w(0x24);
    g.write::<u32>(w(0x2a), 0);
    Ret::jump(slot_target(g, table, 817))
}

/// 0x10001820 (58 bytes, 14 overlays).
pub const E1820: Sig = Sig {
    len: 58,
    hash: 0x12d7_f1fb_b450_2603,
    calls: &[],
    imports: &[],
};
