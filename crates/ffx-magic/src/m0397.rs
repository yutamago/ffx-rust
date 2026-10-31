//! `magic_0397.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task's end (0x10004250, cdecl `(node)`): the script runs
/// ([`run`](crate::sa_script::run), `+0x0c`); while the game is paused
/// slot 561's result is returned; else the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step), `+0x25`) and when
/// neither is 0 the effect's `+0x2408` (`*E`, `+0x07`) becomes 1, slot
/// 628 `(0)` runs and the node goes back to the pool
/// ([`give_back`](crate::sa_pool::give_back), a tail jump, `+0x54`).
/// Returns the motion step's result otherwise.
pub fn script_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x07));
    let a = crate::sa_script::run(g, img, img.rel32_target(f + 0x0c));
    let s = slot_target(g, w(0x12), 561);
    let paused = g.call(s, 0, &[]).eax;
    if paused != 0 {
        return Ret::eax(paused);
    }
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x25));
    if a == 0 || m == 0 {
        return Ret::eax(m);
    }
    g.write::<u32>(e + 0x2408, 1);
    let s = slot_target(g, w(0x32), 628);
    g.call(s, 0, &[0]);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x54),
        node,
    ))
}

/// [`script_end`] (92 bytes, 1 overlay).
pub const SCRIPT_END: Sig = Sig {
    len: 92,
    hash: 0xdb50_4c0e_96fd_55b7,
    calls: &[
        (0x0c, Callee::Sig(&crate::sa_script::RUN)),
        (0x25, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x54, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
