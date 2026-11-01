//! `magic_0413.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// One wrap of an angle: above `hi` it loses `span`, else below `lo`
/// (`lo > x`) it gains it; unordered values stay (`comiss; jbe`).
fn wrap(x: f32, hi: f32, lo: f32, span: f32) -> f32 {
    if x > hi {
        sub(x, span)
    } else if lo > x {
        add(x, span)
    } else {
        x
    }
}

/// The frame callback (0x10003360, cdecl `(task)`): the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)); two angles turn by
/// their steps and wrap into range (`*A += K1`, `*B += K2`; `HI`
/// `+0x1d`, `SPAN` `+0x25`, `LO` `+0x2d`); the word `*X` (`+0x74`) is
/// copied to `+0x81` and `+0x86`; the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both answer 1
/// and `*N` (`+0x9a`) is not positive the task gets state 3 and its
/// record the callback `+0xb1`. Returns the record, else the objects'
/// answer.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let step = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x05));
    let hi = g.read::<f32>(w(0x1d));
    let span = g.read::<f32>(w(0x25));
    let lo = g.read::<f32>(w(0x2d));
    let a = wrap(
        add(g.read::<f32>(w(0x0d)), g.read::<f32>(w(0x15))),
        hi,
        lo,
        span,
    );
    g.write::<f32>(w(0x4b), a);
    let b = wrap(
        add(g.read::<f32>(w(0x53)), g.read::<f32>(w(0x5b))),
        hi,
        lo,
        span,
    );
    let x = g.read::<u32>(w(0x74));
    g.write::<f32>(w(0x7c), b);
    g.write::<u32>(w(0x81), x);
    g.write::<u32>(w(0x86), x);
    // push ebp; push esi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x8b),
        fr.esp.wrapping_sub(0xc),
    );
    if r != 1 || step != 1 || g.read::<i32>(w(0x9a)) > 0 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0xb1));
    Ret::eax(rec)
}

/// [`frame`] (184 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 184,
    hash: 0x7852_b402_82de_0f3a,
    calls: &[
        (0x05, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x8b, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
