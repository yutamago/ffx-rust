//! `magic_0440.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10003440, cdecl `(task)`): the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step), `+0x05`); the angle
/// `*A` (`+0x0d`) plus `D` (`+0x15`) wrapped once by `T`
/// (`+0x28`/`+0x3f`) above `HI` (`+0x1e`) or below `LO` (`+0x32`) goes
/// back to `*A` (`+0x4c`) and `*V` (`+0x44`) into `+0x51` and `+0x56`;
/// the objects run ([`run_objects`](crate::l5_objhost::run_objects),
/// `+0x5b`); when they and the motion answer 1 and `*C` (`+0x6a`) is not
/// above 0 the task gets state 3 and its record the callback `+0x81`.
/// Returns the record, else the objects' answer or 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x05));
    let x = add(g.read::<f32>(w(0x0d)), g.read::<f32>(w(0x15)));
    let x = if x > g.read::<f32>(w(0x1e)) {
        sub(x, g.read::<f32>(w(0x28)))
    } else if g.read::<f32>(w(0x32)) > x {
        add(x, g.read::<f32>(w(0x3f)))
    } else {
        x
    };
    let v = g.read::<u32>(w(0x44));
    g.write::<f32>(w(0x4c), x);
    g.write::<u32>(w(0x51), v);
    g.write::<u32>(w(0x56), v);
    // push ebp; push esi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x5b),
        fr.esp.wrapping_sub(0xc),
    );
    if r != 1 || m != 1 || g.read::<i32>(w(0x6a)) > 0 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x81));
    Ret::eax(rec)
}

/// [`frame`] (136 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 136,
    hash: 0xc9cd_74bd_81fe_a5b6,
    calls: &[
        (0x05, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x5b, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
