//! `magic_0198.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The effect's load (0x10001000, no arguments): slot 718 `(NAME)`, then
/// for `i` below the u16 count `+0x1d`/`+0x4b` slot 209 `(slot 719
/// (NAME, 2, i), 1)` and for `i` below `+0x5e`/`+0x8c` slot 208 `(slot
/// 719 (NAME, 3, i), 1)`, slot 221 `(NAME, 0, 1000, 0x100)` and the
/// handle of slot 719 `(NAME, 6, 0)` into `+0xd0` (`NAME` the immediate
/// at each push). Returns the handle. Each slot call passes the words
/// the original leaves on the stack.
pub fn load<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let table = |g: &G, at: u32, slot: u32| slot_target(g, w(at), slot);
    let s = table(g, 0x01, 718);
    g.call(s, 0, &[w(0x0d)]);
    for (first, count, t719, name, kind, t, slot) in [
        (0x1d, 0x4b, 0x24, 0x32, 2, 0x3c, 209),
        (0x5e, 0x8c, 0x65, 0x73, 3, 0x7d, 208),
    ] {
        if g.read::<u16>(w(first)) == 0 {
            continue;
        }
        let mut i = 0u32;
        loop {
            let s = table(g, t719, 719);
            let h = g.call(s, 0, &[w(name), kind, i]).eax;
            let s = table(g, t, slot);
            g.call(s, 0, &[h, 1, w(name), kind, i]);
            i += 1;
            if i.cast_signed() >= i32::from(g.read::<u16>(w(count))) {
                break;
            }
        }
    }
    let s = table(g, 0x99, 221);
    g.call(s, 0, &[w(0xb0), 0, 0x3e8, 0x100]);
    let s = table(g, 0xb7, 719);
    // call width: 7 (slot 221's words left on the stack)
    let h = g.call(s, 0, &[w(0xc6), 6, 0, w(0xb0), 0, 0x3e8, 0x100]).eax;
    g.write::<u32>(w(0xd0), h);
    Ret::eax(h)
}

/// [`load`] (214 bytes, 1 overlay).
pub const LOAD: Sig = Sig {
    len: 214,
    hash: 0x42e7_07f2_8b9e_a44a,
    calls: &[],
    imports: &[],
};
