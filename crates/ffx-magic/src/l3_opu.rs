//! OPU start-up variants list 3 owns (docs/formats/magic.md, "Porting the
//! overlays"): the overlay table's `+0x0` of the OPU-hosted effects whose
//! control block lives in the DLL's own data (a byte flag global instead
//! of `magic_0604`'s pointer, [`crate::m0604::e1660`]).

use crate::opu;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Table `+0x0` (`magic_0151` 0x10001660, cdecl `(arg)`): allocates the
/// OPU object's 0xfa000 bytes (slot 803, alignment 0x10) into `G_OBJ`
/// (operand `+0x2c`); the effect data word (`G_SLOTS + 4`) gets the
/// control block's `+0xa8` (operand `+0x43`) when the battle flag byte
/// `+0x10` is set and the control byte (operand `+0x3b`) has bit 4, else
/// the default (`+0x4d`); registers it (slot 718), looks it up (slot 719
/// `(data, 1, 0)`), creates the OPU object (slot 715 `(1, entry, data,
/// buffer, 0x200, 0, arg)`), sets it up ([`opu::setup`]) and tail-jumps
/// to slot 816 with the caller's stack. The words earlier calls pushed
/// stay on the stack, so each call sees them after its own.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
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
    let slot719 = slot_target(g, w(0x62), 719);
    let entry = g.call(slot719, fr.ecx, &[data, 1, 0, data]).eax;
    let slot715 = slot_target(g, w(0x81), 715);
    let obj = g
        .call(
            slot715,
            fr.ecx,
            &[1, entry, data, buffer, 0x200, 0, arg, data, 1, 0, data],
        )
        .eax;
    opu::setup(g, img, img.rel32_target(f + 0x95), obj);
    Ret::jump(slot_target(g, w(0x9a), 816))
}

/// `start_up` (`magic_0151` 0x10001660, 169 bytes, 13 overlays:
/// 0148..0167).
pub const START_UP: Sig = Sig {
    len: 169,
    hash: 0x899b_65ad_801a_2f2d,
    calls: &[(0x95, Callee::Sig(&opu::SETUP))],
    imports: &[],
};
