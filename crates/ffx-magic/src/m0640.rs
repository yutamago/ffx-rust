//! `magic_0640.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Registers an object's data (0x10001730, an overlay table entry,
/// `(obj)`; the data `d = obj[0xd0]`): the three program tables (slot
/// 941 `(d[5 + k])`, the earlier words left under each) into `+0x20`,
/// `+0x35`, `+0x4a`; `*G_P = ITEM` (`+0x4f`, `+0x58`). The entry list
/// `L = (*G_L)[1]` (`+0x5d`; none: 0) is searched (slot 719 `(L, 8, i)`
/// for each of its `L[0x50]` entries) for `d`; a found slot `i` that is
/// still free (`USED[i]`, `+0xaf`) gets `d` and slot 711 `(d, PARTS[i],
/// 0)` (`+0xbc`) with the game flag `*G_LOCK` (`+0xce`) 1 and restored.
/// Returns the flag pointer after slot 711, `d` for a slot already used,
/// else 0.
pub fn register<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let d = g.read::<u32>(obj + 0xd0);
    let mut stack = Vec::new();
    for (k, (t_op, out)) in (0u32..).zip([(0x4, 0x20), (0x25, 0x35), (0x3a, 0x4a)]) {
        stack.insert(0, g.read::<u32>(d + 0x14 + 4 * k));
        let t = slot_target(g, w(t_op), 941);
        // call width: 3 at most (the earlier calls' words stay on the stack)
        let table = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(out), table);
    }
    let gp = g.read::<u32>(w(0x4f));
    g.write::<u32>(gp, w(0x58));
    let l = g.read::<u32>(w(0x5d));
    let d = g.read::<u32>(obj + 0xd0);
    let list = g.read::<u32>(l + 4);
    if list == 0 {
        return Ret::eax(0);
    }
    let mut i = 0u32;
    while i < u32::from(g.read::<u16>(list + 0x50)) {
        let t = slot_target(g, w(0x81), 719);
        let eax = g.call(t, 0, &[list, 8, i]).eax;
        if eax == d {
            if i.cast_signed() < 0 {
                return Ret::eax(0);
            }
            let used = w(0xaf).wrapping_add(4 * i);
            if g.read::<u32>(used) != 0 {
                return Ret::eax(eax);
            }
            let parts = g.read::<u32>(w(0xbc).wrapping_add(4 * i));
            let d = g.read::<u32>(obj + 0xd0);
            g.write::<u32>(w(0xc9).wrapping_add(4 * i), d);
            let lock = g.read::<u32>(w(0xce));
            let d = g.read::<u32>(obj + 0xd0);
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let t = slot_target(g, w(0xe1), 711);
            g.call(t, 0, &[d, parts, 0]);
            let lock = g.read::<u32>(w(0xf2));
            g.write::<u32>(lock, saved);
            return Ret::eax(lock);
        }
        i += 1;
    }
    Ret::eax(0)
}

/// [`register`] (256 bytes, 1 overlay).
pub const REGISTER: Sig = Sig {
    len: 256,
    hash: 0x7256_de08_a9c5_9edd,
    calls: &[],
    imports: &[],
};
