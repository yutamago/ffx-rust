//! `magic_0472.dll`'s own functions (besides the runtime and the shared
//! code; its cue start is [`crate::m0638::cue_start`]).

use crate::pushes::Pushes;
use crate::r12h_start::{angle_from, append, konst, task_start, wrap};
use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start (0x100031d0, no arguments, `ebp` frame with one
/// local): two buffer globals from immediates; slot 719 `(P, 8, 0)`
/// into `G_H`; the part set up ([`crate::l5_objhost::part_init`]);
/// object 1 started ([`crate::s4_misc::start_object_anim`], `edx` =
/// `O1`, scale `K`, `(0, 100)`); slot 262 `(*G_A, X, Y, Z)` (the actor's
/// point), `W` = 1.0; the angle `K0 - slot 333 (*G_A)` wrapped once into
/// `O1`'s angle, `Y -= DY`; object 2 (`(1, 75)`, two never-written stack
/// words under them) with its own point, angle and `Y`; the task
/// ([`crate::s3_misc::task_start`] `(T)`), slot 539 `(*G_S, 0)`, a word
/// -1, the actor block ([`crate::sa_misc::me_block`] `(B)`); a pool node
/// ([`crate::sa_pool::take`], callback `CB`) appended to the task list.
/// Each slot call sees the words the earlier calls left (down to the
/// never-written ones). Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut p = Pushes::default();
    p.push_all(&[0, 8, w(0x14)]);
    g.write::<u32>(w(0x1a), w(0x1e));
    g.write::<u32>(w(0x24), w(0x28));
    let t = slot_target(g, w(0x5), 719);
    // call width: 12 (Pushes caps the words at 12)
    let h = p.call(g, t, 0).eax;
    p.pop(1);
    g.write::<u32>(w(0x32), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x37));
    let scale = konst(g, img, f, 0x3f);
    p.push_all(&[0x64, 0]);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x4d), w(0x48), scale, 0, 0x64);
    let actor = g.read::<u32>(w(0x6d));
    p.push_all(&[w(0x57), w(0x62), w(0x67), actor]);
    let t = slot_target(g, w(0x52), 262);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    p.push(g.read::<u32>(w(0x7a)));
    g.write::<u32>(w(0x80), w(0x84));
    let a = angle_from(g, img, f, &p, (0x74, 0x97));
    let a = wrap(g, img, f, a, [0xae, 0xa4, 0xbe, 0xa4]);
    let scale = konst(g, img, f, 0xcf);
    g.write::<f32>(w(0xd7), a);
    let y = sub(g.read::<f32>(w(0xdf)), konst(g, img, f, 0xe7));
    // sub esp, 8: two never-written words under object 2's (no callee
    // reads them; the calls get the words above them)
    let mut p = Pushes::default();
    p.push_all(&[0x4b, 1]);
    g.write::<f32>(w(0xfb), y);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x100), w(0xef), scale, 1, 0x4b);
    let actor = g.read::<u32>(w(0x120));
    p.push_all(&[w(0x10a), w(0x115), w(0x11a), actor]);
    let t = slot_target(g, w(0x105), 262);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    p.push(g.read::<u32>(w(0x12d)));
    g.write::<u32>(w(0x133), w(0x137));
    let a = angle_from(g, img, f, &p, (0x127, 0x14a));
    let a = wrap(g, img, f, a, [0x159, 0x163, 0x16d, 0x17a]);
    g.write::<f32>(w(0x182), a);
    let y = sub(g.read::<f32>(w(0x18a)), konst(g, img, f, 0x192));
    // push ecx (a register slot 333 left; the task reads only the word
    // above it); push T; the return address
    let esp = fr.esp.wrapping_sub(0x14);
    g.write::<f32>(w(0x1a0), y);
    task_start(g, img, img.rel32_target(f + 0x1a5), esp, w(0x198));
    let mut p = Pushes::default();
    p.push(w(0x198));
    p.push_all(&[0, g.read::<u32>(w(0x1b2))]);
    g.write::<u32>(w(0x1b8), w(0x1bc));
    let t = slot_target(g, w(0x1aa), 539);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x1ce), w(0x1c9));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x1db), w(0x1d6));
    append(g, img, f, node, [0x1e1, 0x1f1, 0x1fe, 0x20f, 0x217]);
    Ret::eax(0)
}

/// [`start`] (545 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 545,
    hash: 0xfbff_79f5_eec2_2cb3,
    calls: &[
        (0x37, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4d, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x100, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x1a5, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x1ce, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x1db, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
