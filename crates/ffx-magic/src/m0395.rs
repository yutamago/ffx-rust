//! `magic_0395.dll`'s own functions (besides the runtime, the object
//! host and [`crate::ob_summon`], which has its facing): the controller
//! vibration and the end's resets.

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// A callback (0x10004110, no arguments): the controller vibrates with
/// the effect's stream (slot 306 `(7, mode, slot 719 (E, 9, 0))`, mode 0
/// when `*G_VIBRATE` is set, else 1). Returns slot 306's `eax`.
pub fn vibrate<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let set = g.read::<u32>(w(0x02)) != 0;
    let s719 = slot_target(g, w(0x08), 719);
    let mut stack = vec![w(0x17), 9, 0];
    let e = g.call(s719, 0, &stack).eax;
    let (table, mode) = if set { (w(0x21), 0) } else { (w(0x39), 1) };
    let s306 = slot_target(g, table, 306);
    stack.splice(0..0, [7, mode, e]);
    Ret::eax(g.call(s306, 0, &stack).eax)
}

/// [`vibrate`] (77 bytes, 1 overlay).
pub const VIBRATE: Sig = Sig {
    len: 77,
    hash: 0x6634_e0fb_c497_9bfe,
    calls: &[],
    imports: &[],
};

/// The end's resets (0x100042c0, no arguments): the game unpaused
/// (slot 628 `(0)`), slot 933 `(1.0, 1.0, 1.0, 1.0)`, 1.0 into a data
/// word and slot 785 `(*G_SCALE)` twice (the same stack word carries each
/// call's argument). Returns slot 785's `eax`.
pub fn resets<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    const ONE: u32 = 0x3f80_0000;
    let w = |off: u32| img.word(f + off);
    let s628 = slot_target(g, w(0x01), 628);
    g.call(s628, 0, &[0]);
    let s933 = slot_target(g, w(0x10), 933);
    g.call(s933, 0, &[ONE, ONE, ONE, ONE]);
    let s785 = slot_target(g, w(0x3e), 785);
    let x = g.read::<u32>(w(0x46));
    g.write::<u32>(w(0x55), ONE);
    g.call(s785, 0, &[x]);
    let s785 = slot_target(g, w(0x65), 785);
    let x = g.read::<u32>(w(0x6d));
    g.call(s785, 0, &[x]).eax
}

/// [`resets`] (128 bytes, 1 overlay).
pub const RESETS: Sig = Sig {
    len: 128,
    hash: 0x78f1_f943_208a_582d,
    calls: &[],
    imports: &[],
};
