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

/// A motion task (0x100076b0, internal: `ecx` = the task node, `edx` =
/// the object; no frame): the node gets its vtable (`+0x0d`), its
/// callback (`+0x10` = `+0x14`, [`motion_wait`]) and itself (`+0x14`),
/// goes at the ring's tail (`+0x1c` the last, `+0x26` the root, the last
/// `+0x42`), the root's flag `+0x36` loses bit 0; the object gets the
/// value 0x108e101a as [`set_value`](crate::m0042::set_value) gives it
/// (its own slots: 30 and 36 of the model `+0x54` with bit 9, else slot
/// 228 of `+0x50`; `+0x60` = the value, bit 8); the node's `+0x20` = the
/// object and `+0x18` 0. Returns 0x100.
pub fn motion_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let (n, obj) = (fr.ecx, fr.edx);
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 4, n);
    g.write::<u32>(n, w(0x0d));
    g.write::<u32>(n + 0x10, w(0x14));
    g.write::<u32>(n + 0x14, n);
    g.write::<u32>(n + 8, g.read::<u32>(w(0x1c)));
    g.write::<u32>(n + 4, w(0x26));
    let last = g.read::<u32>(w(0x2b));
    g.write::<u32>(last + 4, n);
    let fl = g.read::<u32>(w(0x36));
    g.write::<u32>(w(0x36), fl & !1);
    g.write::<u32>(w(0x42), n);
    let v = 0x108e_101a;
    if g.read::<u8>(obj + 0x90) & 1 != 0 {
        let s = slot_target(g, w(0x3c), 0x78 / 4);
        let m = g.read::<u32>(obj + 0x54);
        g.call(s, 0, &[m]);
        let s = slot_target(g, w(0x58), 0x90 / 4);
        let m = g.read::<u32>(obj + 0x54);
        g.call(s, 0, &[m, v]);
        let f = g.read::<u16>(obj + 0x90);
        g.write::<u16>(obj + 0x90, f | 0x200);
    } else {
        let s = slot_target(g, w(0x3c), 0x390 / 4);
        let o = g.read::<u32>(obj + 0x50);
        g.call(s, 0, &[o, v]);
    }
    g.write::<u32>(obj + 0x60, v);
    let f = g.read::<u16>(obj + 0x90);
    g.write::<u16>(obj + 0x90, f | 0x100);
    g.write::<u32>(n + 0x20, obj);
    g.write::<u32>(n + 0x18, 0);
    Ret::eax(0x100)
}

/// [`motion_task`] (180 bytes, 1 overlay).
pub const MOTION_TASK: Sig = Sig {
    len: 180,
    hash: 0xf427_ed9c_dbfc_a26d,
    calls: &[],
    imports: &[],
};
