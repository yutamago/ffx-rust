//! `magic_0443.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The sound's step (0x10001210, no arguments): once started (`*S`,
/// `+0x02`, not 0) slot 411 `(*V)` (`V` `+0x4e`) is asked and -1 gives 0;
/// before that, when the count `a` (`*A`, `+0x0a`) and the target `b`
/// (`*B`, `+0x10`) are both 1 or `a - 1` is `b`, slot 228 `(*V, 0x72)`
/// (`V` `+0x2c`) starts it and `*S` goes up. Returns 1 otherwise.
pub fn sound_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    if g.read::<u32>(w(0x02)) != 0 {
        let s = slot_target(g, w(0x48), 0x66c / 4);
        let v = g.read::<u32>(w(0x4e));
        let r = g.call(s, 0, &[v]).eax;
        return Ret::eax(u32::from(r != u32::MAX));
    }
    let a = g.read::<u32>(w(0x0a));
    let b = g.read::<u32>(w(0x10));
    if (a == 1 && b == 1) || a.wrapping_sub(1) == b {
        let s = slot_target(g, w(0x24), 0x390 / 4);
        let v = g.read::<u32>(w(0x2c));
        g.call(s, 0, &[v, 0x72]);
        let n = g.read::<u32>(w(0x3d));
        g.write::<u32>(w(0x3d), n.wrapping_add(1));
    }
    Ret::eax(1)
}

/// [`sound_step`] (101 bytes, 1 overlay).
pub const SOUND_STEP: Sig = Sig {
    len: 101,
    hash: 0x028d_affd_90b0_51dd,
    calls: &[],
    imports: &[],
};
