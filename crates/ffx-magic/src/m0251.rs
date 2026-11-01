//! `magic_0251.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The parts' programs (0x100024b0, no arguments): `n` = `*N` (`+0x08`,
/// read again after every inner step); for each of `n` part records
/// (`*E` on, `E` at `+0x0f`, 0x68 bytes apart) and each `i` below `n`:
/// slot 941 `(record word)` gives the part, slot 772 `(part, i)`'s 64-bit
/// result goes to slot 779 `(lo, hi, part, i)` with the game flag `*L`
/// 1 around it (`L` `+0x53`, again `+0x6f`). Returns the last `n` read.
pub fn part_programs<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut n = g.read::<i32>(w(0x08));
    let mut rec = g.read::<u32>(w(0x0f));
    let mut k = 0i32;
    if n <= 0 {
        return Ret::eax(n.cast_unsigned());
    }
    loop {
        let mut i = 0i32;
        while i < n {
            let s = slot_target(g, w(0x31), 0xeb4 / 4);
            let part = g.call(s, 0, &[g.read::<u32>(rec)]).eax;
            let s = slot_target(g, w(0x45), 0xc10 / 4);
            let v = g.call(s, 0, &[part, i.cast_unsigned()]);
            let l = g.read::<u32>(w(0x53));
            let saved = g.read::<u32>(l);
            g.write::<u32>(l, 1);
            let s = slot_target(g, w(0x5c), 0xc2c / 4);
            g.call(s, 0, &[v.eax, v.edx, part, i.cast_unsigned()]);
            let l = g.read::<u32>(w(0x6f));
            g.write::<u32>(l, saved);
            i += 1;
            n = g.read::<i32>(w(0x77));
        }
        k += 1;
        rec = rec.wrapping_add(0x68);
        if k >= n {
            break;
        }
    }
    Ret::eax(n.cast_unsigned())
}

/// [`part_programs`] (153 bytes, 1 overlay).
pub const PART_PROGRAMS: Sig = Sig {
    len: 153,
    hash: 0xefeb_cb47_abe6_3f9a,
    calls: &[],
    imports: &[],
};
