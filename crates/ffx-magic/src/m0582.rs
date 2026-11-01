//! `magic_0582.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code).

use crate::opu;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Table `+0x0` (0x10001690, cdecl `(arg)`): allocates the OPU object's
/// 0x3e8000 bytes (slot 803, alignment 0x10) into `G_OBJ`; picks the
/// effect data (`G_SLOT + 4` gets `G_DATA` when the battle flag byte
/// `+0x10` is set and the global control byte (operand `+0x3b`) has bit
/// 4, else the default at `+0x4d`'s operand), registers it (slot 718)
/// and looks it up (slot 719 `(data, 1, 0)`); creates the OPU object
/// (slot 715 with `(1, entry, data, buffer, 0x200, 0, arg)`, the earlier
/// calls' words under them) and tail-jumps to its set-up
/// ([`opu::setup`]). Returns the set-up's `eax`.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let slot803 = slot_target(g, w(0x04), 803);
    let buffer = g.call(slot803, fr.ecx, &[0x003e_8000, 0x10]).eax;
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
    g.call(slot718, flags, &[data]);
    let slot719 = slot_target(g, w(0x62), 719);
    let entry = g.call(slot719, flags, &[data, 1, 0, data]).eax;
    let slot715 = slot_target(g, w(0x81), 715);
    let obj = g
        .call(
            slot715,
            flags,
            &[1, entry, data, buffer, 0x200, 0, arg, data, 1, 0, data],
        )
        .eax;
    Ret::eax(opu::setup(g, img, img.rel32_target(f + 0x98), obj))
}

/// [`start`] (156 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 156,
    hash: 0xca2b_320e_1b0b_d25c,
    calls: &[(0x98, Callee::Sig(&opu::SETUP))],
    imports: &[],
};
