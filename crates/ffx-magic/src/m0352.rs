//! `magic_0352.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10004c10, cdecl `(node)`, an aligned frame):
/// the script ([`run`](crate::sa_script::run)) and the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)); when both answer 1
/// the effect's (`*E`, `+0x0a`) `+0xbd8` becomes 1, the handle of the part
/// 0x160 is registered (slot 772 `(0x160, 0)`'s 64-bit result to slot 779
/// with the game flag `*L` 1 around it, `L` `+0x3f`/`+0x5b`) and the node
/// goes back to its pool ([`give_back`](crate::sa_pool::give_back)).
/// Returns that, else the motion step's answer.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x0a));
    let script = crate::sa_script::run(g, img, img.rel32_target(f + 0x0f));
    let step = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x16));
    if script != 1 || step != 1 {
        return Ret::eax(step);
    }
    g.write::<u32>(fx + 0xbd8, 1);
    let s = slot_target(g, w(0x24), 0xc10 / 4);
    let h = g.call(s, 0, &[0x160, 0]);
    let lock = g.read::<u32>(w(0x3f));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s = slot_target(g, w(0x48), 0xc2c / 4);
    g.call(s, 0, &[h.eax, h.edx, 0x160, 0]);
    let lock = g.read::<u32>(w(0x5b));
    g.write::<u32>(lock, saved);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x68),
        node,
    ))
}

/// [`frame`] (114 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 114,
    hash: 0xabe1_3f75_5211_f3de,
    calls: &[
        (0x0f, Callee::Sig(&crate::sa_script::RUN)),
        (0x16, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x68, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
