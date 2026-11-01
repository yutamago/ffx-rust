//! `magic_0305.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10001bf0, cdecl `(task)`): two countdowns (`*A`
/// `+0x04`, `*B` `+0x40`), when positive, drop; one reaching 0 sends
/// slot 306 `(7, 0 / 1, h)` with `h` slot 719 `(F, 9, 0)`'s handle (`F`
/// `+0x24`/`+0x60`). Then the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both answer 1
/// and bit 15 of `+0x8f` is set the task gets state 3, its record the
/// callback `+0xae`, and a tail jump goes into the parts' registration
/// ([`register_parts`](crate::sa_misc::register_parts)). Returns the
/// objects' answer otherwise. Each slot call passes the words the
/// original leaves on the stack.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    for (k, (count_r, count_w, t719, name, t306)) in [
        (0x04, 0x0e, 0x15, 0x24, 0x2c),
        (0x40, 0x4a, 0x51, 0x60, 0x68),
    ]
    .into_iter()
    .enumerate()
    {
        let c = g.read::<i32>(w(count_r));
        if c <= 0 {
            continue;
        }
        g.write::<i32>(w(count_w), c - 1);
        if c != 1 {
            continue;
        }
        let mut p = Pushes::default();
        p.push_all(&[0, 9, w(name)]);
        let s = slot_target(g, w(t719), 0xb3c / 4);
        // call width: 12 (Pushes caps the words at 12)
        let h = p.call(g, s, 0).eax;
        p.push_all(&[h, u32::try_from(k).unwrap_or(0), 7]);
        let s = slot_target(g, w(t306), 0x4c8 / 4);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, s, 0);
    }
    let step = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x7d));
    // push ebp; push esi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x84),
        fr.esp.wrapping_sub(0xc),
    );
    if r != 1 || g.read::<u32>(w(0x8f)) & 0x8000 == 0 || step != 1 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0xae));
    Ret::eax(crate::sa_misc::register_parts(
        g,
        img,
        img.rel32_target(f + 0xb4),
        rec,
    ))
}

/// [`frame`] (187 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 187,
    hash: 0x69de_a1aa_199a_0879,
    calls: &[
        (0x7d, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x84, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0xb4, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};
