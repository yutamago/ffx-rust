//! `magic_0476.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sa_pool::give_back;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A fading node's step (0x100017f0, `(node)`, a node callback): the
/// count `n` at operand `+0x07` goes down by one (stored before the
/// call); with `d = n - 120`, slot 348 `(*G_A, *G_B, (120 - n) * 768 /
/// 120, d * 96 / 120, d * 52 / 120, 0)` (`G_A` at `+0x72`, `G_B` at
/// `+0x67`; signed divisions). While the count is above 0 that answer is
/// returned; at 0 the node goes back to the pool ([`give_back`], a tail
/// jump), whose `eax` it returns.
pub fn fade_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let n = g.read::<i32>(w(0x7)).wrapping_sub(1);
    let d = n.wrapping_sub(0x78);
    let a = d.wrapping_mul(0x34) / 120;
    let b = d.wrapping_mul(0x60) / 120;
    let c = 0x78i32.wrapping_sub(n).wrapping_mul(0x300) / 120;
    let ga = g.read::<u32>(w(0x72));
    let gb = g.read::<u32>(w(0x67));
    let s348 = slot_target(g, w(0x6c), 348);
    g.write::<i32>(w(0x7e), n);
    let r = g.call(
        s348,
        0,
        &[
            ga,
            gb,
            c.cast_unsigned(),
            b.cast_unsigned(),
            a.cast_unsigned(),
            0,
        ],
    );
    if g.read::<i32>(w(0x89)) > 0 {
        return Ret::eax(r.eax);
    }
    let node = fr.arg(g, 0);
    Ret::eax(give_back(g, img, img.rel32_target(f + 0x97), node))
}

/// [`fade_step`] (157 bytes, 1 overlay).
pub const FADE_STEP: Sig = Sig {
    len: 157,
    hash: 0x9895_0f14_a590_1ebd,
    calls: &[(0x97, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};

/// The effect's task (0x100017a0, `(task)`, a task callback): the motion
/// step ([`motion_step`](crate::s3_motion::motion_step)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both say 1,
/// slot 346 `(*G_A)` (`ecx` the cookie `run_objects`' check leaves), the
/// task's state `+0x14` becomes 3 and its record
/// (`task[8]`) gets the callback at `+0x3f`. Returns the record, or the
/// objects' answer.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let step = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x5));
    // push ebp; push esi: the call's return address
    let run = img.rel32_target(f + 0xc);
    let r = crate::l5_objhost::run_objects(g, img, run, fr.esp.wrapping_sub(0xc));
    if r != 1 || step != 1 {
        return Ret::eax(r);
    }
    let a = g.read::<u32>(w(0x20));
    // `ecx`: the security cookie run_objects' check left there (its
    // global, the operand at `+0x0a` of run_objects)
    let ecx = g.read::<u32>(img.word(run + 0x0a));
    let t = slot_target(g, w(0x1a), 346);
    // call width: 1
    g.call(t, ecx, &[a]);
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x3f));
    Ret::eax(rec)
}

/// [`task`] (70 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 70,
    hash: 0x2cd7_cc4b_9b4a_a63a,
    calls: &[
        (0x05, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x0c, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
