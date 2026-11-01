//! `magic_0260.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The parts' programs (0x100021f0, internal: `ecx` = the effect): for
/// each of five part blocks (`+0x354`, 0x3a0 bytes apart) the count word
/// is set to 0x40 and, for i below it (read again each time), slot 772
/// `(p, i)`'s 64-bit result goes to slot 779 `(lo, hi, p, i)` (`p` the
/// block's `-0x224`) with the game flag `*L` 1 around it (`L` at `+0x3f`,
/// again at `+0x5b`). Returns `L` (the last read; the caller's `eax` is
/// never left).
pub fn part_programs<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut count = fr.ecx.wrapping_add(0x354);
    let mut lock = 0;
    for _ in 0..5 {
        g.write::<u32>(count, 0x40);
        let part = count.wrapping_sub(0x224);
        let mut i = 0u32;
        loop {
            let s = slot_target(g, w(0x31), 0xc10 / 4);
            let v = g.call(s, 0, &[part, i]);
            let l = g.read::<u32>(w(0x3f));
            let saved = g.read::<u32>(l);
            g.write::<u32>(l, 1);
            let s = slot_target(g, w(0x48), 0xc2c / 4);
            g.call(s, 0, &[v.eax, v.edx, part, i]);
            lock = g.read::<u32>(w(0x5b));
            g.write::<u32>(lock, saved);
            i += 1;
            if i.cast_signed() >= g.read::<i32>(count) {
                break;
            }
        }
        count = count.wrapping_add(0x3a0);
    }
    Ret::eax(lock)
}

/// [`part_programs`] (124 bytes, 1 overlay).
pub const PART_PROGRAMS: Sig = Sig {
    len: 124,
    hash: 0x5f73_9990_3239_8b69,
    calls: &[],
    imports: &[],
};
