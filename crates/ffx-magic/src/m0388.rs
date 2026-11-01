//! `magic_0388.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task's motion wait (0x10007610, cdecl `(task)`; `s` = the task's
/// state `+4`): in state 0 the object (`s + 0x20`) is asked whether its
/// motion ended (slot 20 of the model `+0x54` when bit 0 of `+0x90` is
/// set, else slot 411 of the object `+0x50`); when it did, the object
/// gets the value 0x108e101b ([`set_value`](crate::m0042::set_value)),
/// the countdown `+0x1c` becomes 0xef and the state 10. In state 10 with
/// the countdown at 0 the task's node leaves its ring (and points at
/// itself), the countdown drops (to -1) and the state becomes 20000; in
/// every other case the countdown drops. Returns the last value the code
/// leaves in `eax` (`set_value`'s, the node's old previous, 0 or the
/// state).
pub fn motion_wait<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 4);
    let state = g.read::<u32>(s + 0x18);
    if state == 0 {
        let obj = g.read::<u32>(s + 0x20);
        let ended = if g.read::<u8>(obj + 0x90) & 1 != 0 {
            let t = slot_target(g, w(0x53), 0x50 / 4);
            let m = g.read::<u32>(obj + 0x54);
            g.call(t, 0, &[m]).eax
        } else {
            let t = slot_target(g, w(0x60), 0x66c / 4);
            let o = g.read::<u32>(obj + 0x50);
            g.call(t, 0, &[o]).eax
        };
        if ended == 0 {
            let c = g.read::<u32>(s + 0x1c);
            g.write::<u32>(s + 0x1c, c.wrapping_sub(1));
            return Ret::eax(0);
        }
        let obj = g.read::<u32>(s + 0x20);
        let eax = crate::m0042::set_value(g, img, img.rel32_target(f + 0x7c), obj, 0x108e_101b);
        g.write::<u32>(s + 0x1c, 0xf0);
        g.write::<u32>(s + 0x18, 0xa);
        g.write::<u32>(s + 0x1c, 0xef);
        return Ret::eax(eax);
    }
    if state != 0xa || g.read::<u32>(s + 0x1c) != 0 {
        let c = g.read::<u32>(s + 0x1c);
        g.write::<u32>(s + 0x1c, c.wrapping_sub(1));
        return Ret::eax(state);
    }
    let node = g.read::<u32>(task);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 8, g.read::<u32>(node + 8));
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    let c = g.read::<u32>(s + 0x1c);
    g.write::<u32>(s + 0x1c, c.wrapping_sub(1));
    g.write::<u32>(s + 0x18, 0x4e20);
    Ret::eax(prev)
}

/// [`motion_wait`] (148 bytes, 1 overlay).
pub const MOTION_WAIT: Sig = Sig {
    len: 148,
    hash: 0xaaaa_552a_05ca_e7b8,
    calls: &[(0x7c, Callee::Sig(&crate::m0042::SET_VALUE))],
    imports: &[],
};
