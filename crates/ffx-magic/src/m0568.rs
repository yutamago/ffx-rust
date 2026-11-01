//! `magic_0568.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Sways the summon's three heights (0x10001c30, internal, no
/// arguments): the time `t = S[0x16c4]` (`S = *G_S`) samples the DLL's
/// five-key table ([`ease_sample`](crate::sb_misc::ease_sample), stride 1
/// pushed) and steps by `K`; `S[0x464]`, `S[0x6c4]` and `S[0x142c]` =
/// `S[0x143c] + K_i + sample` (the base re-read each time). Returns the
/// sample's `eax` (scratch no caller reads).
pub fn sway<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x3));
    let t = g.read::<f32>(s + 0x16c4);
    // push esi; push 1; call: the stride word at entry - 8
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 4, 1);
    let x = crate::sb_misc::ease_sample(g, img, img.rel32_target(f + 0x1f), esp, w(0x17), 5, t);
    let t = add(t, g.read::<f32>(w(0x27)));
    g.write::<f32>(s + 0x16c4, t);
    for (k, dst) in [(0x45u32, 0x464u32), (0x61, 0x6c4), (0x7d, 0x142c)] {
        let v = add(add(g.read::<f32>(s + 0x143c), g.read::<f32>(w(k))), x);
        g.write::<f32>(s + dst, v);
    }
    Ret::eax(0)
}

/// [`sway`] (143 bytes, 1 overlay).
pub const SWAY: Sig = Sig {
    len: 143,
    hash: 0x9dfe_6c0a_73b2_ae66,
    calls: &[(0x1f, Callee::Sig(&crate::sb_misc::EASE_SAMPLE))],
    imports: &[],
};
