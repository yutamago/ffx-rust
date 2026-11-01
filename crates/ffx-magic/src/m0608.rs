//! `magic_0608.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code).

use crate::opu;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Table `+0x0` (0x10001690, cdecl `(arg)`): allocates the OPU object's
/// 0xfa000 bytes (slot 803, alignment 0x10) into `G_OBJ`; picks the
/// effect data (`G_SLOT + 4` gets `G_DATA` when the battle flag byte
/// `+0x10` is set and the global control byte (operand `+0x3c`) has bit
/// 4, else the default at `+0x4e`'s operand), registers it (slot 718),
/// looks it up (slot 719 `(data, 1, 0)`) and hands it slot 173 `(data,
/// TABLE)` (immediate `+0x7b`); creates the OPU object (slot 715 with
/// `(1, entry, data, buffer, 0x200, 0, arg)`, the earlier calls' words
/// under them) and tail-jumps to its set-up ([`opu::setup`]). Returns
/// the set-up's `eax`.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let slot803 = slot_target(g, w(0x04), 803);
    let buffer = g.call(slot803, fr.ecx, &[0xfa000, 0x10]).eax;
    let flags = g.read::<u32>(w(0x1c));
    let slots = g.read::<u32>(w(0x22)).wrapping_add(4);
    g.write::<u32>(w(0x2d), buffer);
    let data = if g.read::<u8>(flags + 0x10) != 0 && g.read::<u8>(w(0x3c)) & 0x10 != 0 {
        g.read::<u32>(w(0x44))
    } else {
        w(0x4e)
    };
    g.write::<u32>(slots, data);
    let data = g.read::<u32>(slots);
    let slot718 = slot_target(g, w(0x53), 718);
    g.call(slot718, flags, &[data]);
    let slot719 = slot_target(g, w(0x63), 719);
    let entry = g.call(slot719, flags, &[data, 1, 0, data]).eax;
    let slot173 = slot_target(g, w(0x76), 173);
    g.call(slot173, flags, &[data, w(0x7b), data, 1, 0, data]);
    let slot715 = slot_target(g, w(0x8e), 715);
    let obj = g
        .call(
            slot715,
            flags,
            // call width: 13 (the earlier calls' words stay on the stack)
            &[
                1,
                entry,
                data,
                buffer,
                0x200,
                0,
                arg,
                data,
                w(0x7b),
                data,
                1,
                0,
                data,
            ],
        )
        .eax;
    Ret::eax(opu::setup(g, img, img.rel32_target(f + 0xb0), obj))
}

/// [`start`] (180 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 180,
    hash: 0x1952_b406_a700_54dc,
    calls: &[(0xb0, Callee::Sig(&opu::SETUP))],
    imports: &[],
};
