//! `magic_0180.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task step (0x10004810, cdecl `(task)`, an aligned frame): the motion
/// step ([`motion_step`](crate::s3_motion::motion_step)) and the objects
/// run ([`run_objects`](crate::l5_objhost::run_objects)); when both give
/// 1 and `*G_D` (`+0x1f`) is 1, the task's state is 3 and its record's
/// `+0x10` the immediate `+0x35`, then the square registered
/// ([`register_square`](crate::s4_small::register_square)), the state at
/// `+0x3f` reset ([`state_reset`](crate::s4_small::state_reset)) and the
/// register loop of `+0x49` ([`register_loop`](crate::o4_fam2::register_loop)).
/// Returns the loop's result, else the objects'.
pub fn square_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -8; push ecx; push esi
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8);
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x9));
    let r = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x10), esp.wrapping_sub(4));
    if r != 1 || m != 1 || g.read::<u32>(w(0x1f)) != 1 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task.wrapping_add(0x14), 3);
    let rec = g.read::<u32>(task.wrapping_add(8));
    g.write::<u32>(rec.wrapping_add(0x10), w(0x35));
    let callee = |at: u32, ecx: u32| Frame {
        at,
        esp: esp.wrapping_sub(4),
        ecx,
        ..*fr
    };
    crate::s4_small::register_square(g, img, &callee(img.rel32_target(f + 0x3a), 0));
    crate::s4_small::state_reset(g, img, &callee(img.rel32_target(f + 0x44), w(0x3f)));
    Ret::eax(crate::o4_fam2::register_loop(
        g,
        img,
        img.rel32_target(f + 0x4e),
        w(0x49),
    ))
}

/// [`square_step`] (87 bytes, 1 overlay).
pub const SQUARE_STEP: Sig = Sig {
    len: 87,
    hash: 0x458e_891e_84c8_b841,
    calls: &[
        (0x9, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x10, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x3a, Callee::Sig(&crate::s4_small::REGISTER_SQUARE)),
        (0x44, Callee::Sig(&crate::s4_small::STATE_RESET)),
        (0x4e, Callee::Sig(&crate::o4_fam2::REGISTER_LOOP_0180_1870)),
    ],
    imports: &[],
};

/// Starts the main object (0x10004a80, cdecl `(a)`): the object at the
/// immediate `+0x16` ([`start_object_at`](crate::s4_misc::start_object_at),
/// scale the float at `+0x07`, index 1, owners 0 and the immediate
/// `+0x0c`); slot 325 `(P, a + 0x24c)` (`P` the immediate `+0x2e`; the
/// start's words left under it, `ecx` among them); the angle at `+0x3e`
/// plus `A` (`+0x46`), less `W` (`+0x5a`) above `H` (`+0x50`) or plus `W`
/// (`+0x71`) below `L` (`+0x64`), into `+0x7b`; then the record's words
/// (operands `+0x81`..`+0xc7` 0 but `+0xa9`/`+0xb3`, the immediates
/// `+0xad`/`+0xb7`; the halfword `+0xd1` 0), `+0xd7` = the immediate
/// `+0xdb`, `+0xe1` = 1. Returns 0.
pub fn main_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let scale = k(g, 0x7);
    crate::s4_misc::start_object_at(
        g,
        img,
        img.rel32_target(f + 0x1b),
        w(0x16),
        scale,
        1,
        0,
        w(0xc),
    );
    let a = fr.arg(g, 0);
    let t = crate::runtime::slot_target(g, w(0x29), 325);
    g.call(
        t,
        0,
        &[w(0x2e), a.wrapping_add(0x24c), 1, 0, fr.ecx, w(0xc)],
    );
    let mut x = crate::sb_sse::add(k(g, 0x3e), k(g, 0x46));
    // comiss x, H; jbe: below, equal or unordered
    if x > k(g, 0x50) {
        x = crate::sb_sse::sub(x, k(g, 0x5a));
    } else if k(g, 0x64) > x {
        x = crate::sb_sse::add(x, k(g, 0x71));
    }
    g.write::<f32>(w(0x7b), x);
    for o in [0x81, 0x8b, 0x95, 0x9f] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0xa9), w(0xad));
    g.write::<u32>(w(0xb3), w(0xb7));
    g.write::<u32>(w(0xbd), 0);
    g.write::<u32>(w(0xc7), 0);
    g.write::<u16>(w(0xd1), 0);
    g.write::<u32>(w(0xd7), w(0xdb));
    g.write::<u32>(w(0xe1), 1);
    Ret::eax(0)
}

/// [`main_start`] (235 bytes, 1 overlay).
pub const MAIN_START: Sig = Sig {
    len: 235,
    hash: 0x67e0_aef7_2b2e_775a,
    calls: &[(0x1b, Callee::Sig(&crate::s4_misc::START_OBJECT_AT))],
    imports: &[],
};
