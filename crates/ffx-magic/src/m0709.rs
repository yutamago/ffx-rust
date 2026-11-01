//! `magic_0709.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code).

use crate::opu;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The OPU start-up without a battle variant (`magic_0709` and
/// `magic_0706` at 0x10001690, cdecl `(arg)`): allocates the object's
/// 0xfa000 bytes (slot 803) into `G_OBJ` (`+0x22`), `G_SLOT[1]` (`+0x1a`)
/// the data (the immediate `+0x29`), registers it (slot 718, through
/// `ecx`), then `extra` (each `(table operand, slot, words)`), looks it
/// up (slot 719 `(data, 1, 0)`), creates the OPU object (slot 715 `(1,
/// entry, data, buffer, 0x200, 0, arg)`) and tail-jumps to its set-up
/// ([`opu::setup`]; the two tables' operands and the call's `rel32` in
/// `ops`). Every call sees the
/// words the earlier ones left (803's too). Returns the set-up's `eax`.
pub(crate) fn opu_start<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    extra: &[(u32, u32, [u32; 2])],
    ops: (u32, u32, u32),
) -> Ret {
    let (o719, o715, call) = ops;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let t = slot_target(g, w(0x4), 803);
    let mut stack = vec![0xfa000, 0x10];
    let buffer = g.call(t, fr.ecx, &stack).eax;
    let slots = g.read::<u32>(w(0x1a));
    g.write::<u32>(w(0x22), buffer);
    let data = w(0x29);
    g.write::<u32>(slots.wrapping_add(4), data);
    let t718 = slot_target(g, w(0x2f), 718);
    stack.insert(0, w(0x34));
    // call width: 3
    g.call(t718, t718, &stack);
    for &(op, slot, words) in extra {
        let t = slot_target(g, w(op), slot);
        stack.splice(0..0, words);
        // call width: 5 at most (the earlier calls' words stay on the stack)
        g.call(t, t718, &stack);
    }
    let t = slot_target(g, w(o719), 719);
    stack.splice(0..0, [data, 1, 0]);
    // call width: 8 at most (the earlier calls' words stay on the stack)
    let entry = g.call(t, t718, &stack).eax;
    let t = slot_target(g, w(o715), 715);
    stack.splice(0..0, [1, entry, data, buffer, 0x200, 0, arg]);
    // call width: 15 at most (the earlier calls' words stay on the stack)
    let obj = g.call(t, t718, &stack).eax;
    Ret::eax(opu::setup(g, img, img.rel32_target(f + call), obj))
}

/// Table `+0x0` (0x10001690, `(arg)`): [`opu_start`].
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    opu_start(g, img, fr, &[], (0x41, 0x68, 0x7e))
}

/// [`start`] (130 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 130,
    hash: 0xca06_117f_d1fa_a2c1,
    calls: &[(0x7e, Callee::Sig(&opu::SETUP))],
    imports: &[],
};
