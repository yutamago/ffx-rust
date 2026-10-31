//! `magic_0475.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's task (0x10001600, `(task)`, a task callback): the motion
/// step ([`motion_step`](crate::s3_motion::motion_step)) and the objects
/// ([`run_objects`](crate::l5_objhost::run_objects)); when both say 1
/// and the count at operand `+0x23` is not above 0, the task's state
/// `+0x14` becomes 3 and its record (`task[8]`) gets the callback at
/// `+0x3e`. Then, when slot 917 `()` says 0xc9, the point `P` (`+0x5e`)
/// moves by `-K` per axis, slot 869 `(0)` gives a handle `e`, slot 916
/// `(e, 1, 0, 15, 0, 5, 0)`, slot 869 `(0)` again `e2`, and slot 284
/// `(e2, 1, P)` (the earlier words left on the stack). Returns the last
/// answer (`eax`).
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let step = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x5));
    // push ebp; push esi: the call's return address
    let run = img.rel32_target(f + 0xc);
    let r = crate::l5_objhost::run_objects(g, img, run, fr.esp.wrapping_sub(0xc));
    if r != 1 || step != 1 || g.read::<i32>(w(0x23)) > 0 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x3e));
    let s917 = slot_target(g, w(0x43), 917);
    let r = g.call(s917, 0, &[]).eax;
    if r != 0xc9 {
        return Ret::eax(r);
    }
    for (load, k, store) in [(0x5e, 0x66, 0x75), (0x7d, 0x85, 0x8d), (0x95, 0x9d, 0xa5)] {
        let v = sub(g.read::<f32>(w(load)), g.read::<f32>(w(k)));
        g.write::<f32>(w(store), v);
    }
    let s869 = slot_target(g, w(0x6b), 869);
    let e = g.call(s869, 0, &[0]).eax;
    let s916 = slot_target(g, w(0xbf), 916);
    let mut stack = vec![e, 1, 0, 0xf, 0, 5, 0, 0];
    g.call(s916, 0, &stack);
    let s869 = slot_target(g, w(0xcc), 869);
    stack.insert(0, 0);
    // call width: 12
    let e2 = g.call(s869, 0, &stack).eax;
    let s284 = slot_target(g, w(0xe3), 284);
    let mut args = vec![e2, 1, w(0xdb)];
    args.extend(stack);
    // call width: 12
    Ret::eax(g.call(s284, 0, &args).eax)
}

/// [`task`] (245 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 245,
    hash: 0xde88_8936_ee2d_f265,
    calls: &[
        (0x5, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xc, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
