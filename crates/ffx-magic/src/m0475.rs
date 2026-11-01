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

/// The effect's start (0x100013e0, no arguments, `ebp` frame with one
/// local): two buffer globals from immediates; slot 719 `(P, 8, 0)`
/// into `G_H`; the part set up ([`crate::l5_objhost::part_init`]); the
/// object started ([`crate::o6_fam_object::start_object_first`], `edx` =
/// `O`, scale `K`; two never-written stack words, which it does not
/// read); a float global from an immediate; the angle `K0 - slot 333
/// (*G_A)` wrapped once into `O`'s angle; the task
/// ([`crate::s3_misc::task_start`] `(T)`), a word -1, two scale tasks
/// ([`crate::s4_misc::scale_task`] on the states `S1` `(K1, K2, 0)` and
/// `S2` `(K3, K1, 0xdc)`), slot 271 `(4)`, the actor block
/// ([`crate::sa_misc::me_block`] `(B)`); a pool node
/// ([`crate::sa_pool::take`], callback `CB`) appended to the task list.
/// Then, when slot 917 answers 201: slot 672 `(1, V)` fills `V`, `V.x -=
/// DX`, `V.y += *G_DY`, `V.z += DZ`; slot 916 `(slot 869 (0), 1, 0, 15,
/// 0, 5, 0)` and slot 284 `(slot 869 (0), 1, V)`. Each slot call sees
/// the words the earlier calls left (down to the never-written ones).
/// Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    use crate::r12h_start::{angle_from, append, konst, task_start, wrap};
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut p = Pushes::default();
    p.push_all(&[0, 8, w(0x14)]);
    g.write::<u32>(w(0x1a), w(0x1e));
    g.write::<u32>(w(0x24), w(0x28));
    let t = slot_target(g, w(0x5), 719);
    // call width: 12 (Pushes caps the words at 12)
    let h = p.call(g, t, 0).eax;
    g.write::<u32>(w(0x32), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x37));
    let scale = g.read::<u32>(w(0x3f));
    // push ebp; push ecx; the two words slot 719 left; sub esp, 8; the
    // return address
    let callee = Frame {
        at: img.rel32_target(f + 0x4c),
        esp: fr.esp.wrapping_sub(0x1c),
        edx: w(0x47),
        xmm: [0, 0, 0, u128::from(scale)],
        ..Frame::default()
    };
    crate::o6_fam_object::start_object_first(g, img, callee.at, &callee);
    // sub esp, 8: the words below are never written; the calls get the
    // words above them
    let mut p = Pushes::default();
    p.push(g.read::<u32>(w(0x57)));
    g.write::<u32>(w(0x5d), w(0x61));
    let a = angle_from(g, img, f, &p, (0x51, 0x74));
    let a = wrap(g, img, f, a, [0x83, 0x8d, 0x97, 0xa4]);
    g.write::<f32>(w(0xb2), a);
    // push ecx (a register slot 333 left; the task reads only the word
    // above it); push T; the return address
    let esp = fr.esp.wrapping_sub(0x14);
    task_start(g, img, img.rel32_target(f + 0xb7), esp, w(0xaa));
    let (k1, k2) = (konst(g, img, f, 0xbf), konst(g, img, f, 0xc7));
    g.write::<u32>(w(0xd7), w(0xdb));
    // each scale task's stack word above the earlier ones, T and the
    // register
    for (state, xmm1, xmm2, word, call, esp) in [
        (w(0xd1), k1, k2, 0u32, 0xe0u32, fr.esp.wrapping_sub(0x18)),
        (
            w(0xf5),
            konst(g, img, f, 0xe8),
            k1,
            0xdc,
            0xfa,
            fr.esp.wrapping_sub(0x1c),
        ),
    ] {
        g.write::<u32>(esp.wrapping_add(4), word);
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: state,
            xmm: [0, u128::from(xmm1.to_bits()), u128::from(xmm2.to_bits()), 0],
            ..Frame::default()
        };
        crate::s4_misc::scale_task(g, img, &callee);
    }
    let mut p = Pushes::default();
    p.push_all(&[w(0xaa), 0, 0xdc, 4]);
    let t = slot_target(g, w(0xff), 271);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x113), w(0x10e));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x120), w(0x11b));
    append(g, img, f, node, [0x126, 0x136, 0x150, 0x148, 0x150]);
    let t = slot_target(g, w(0x155), 917);
    // call width: 0
    if g.call(t, 0, &[]).eax != 0xc9 {
        return Ret::eax(0);
    }
    let v = w(0x172);
    let mut p = Pushes::default();
    p.push_all(&[v, 1]);
    let t = slot_target(g, w(0x16d), 672);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    let x = sub(g.read::<f32>(w(0x184)), konst(g, img, f, 0x18c));
    p.push(0);
    g.write::<f32>(w(0x19b), x);
    let y = add(g.read::<f32>(w(0x1a3)), g.read::<f32>(w(0x1ab)));
    g.write::<f32>(w(0x1b3), y);
    let z = add(g.read::<f32>(w(0x1bb)), konst(g, img, f, 0x1c3));
    g.write::<f32>(w(0x1cb), z);
    let t = slot_target(g, w(0x191), 869);
    // call width: 12 (Pushes caps the words at 12)
    let e = p.call(g, t, 0).eax;
    p.push_all(&[0, 5, 0, 0xf, 0, 1, e]);
    let t = slot_target(g, w(0x1e5), 916);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    p.push(0);
    let t = slot_target(g, w(0x1f2), 869);
    // call width: 12 (Pushes caps the words at 12)
    let e = p.call(g, t, 0).eax;
    p.push_all(&[w(0x201), 1, e]);
    let t = slot_target(g, w(0x209), 284);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    Ret::eax(0)
}

/// [`start`] (542 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 542,
    hash: 0x6cef_0690_5c37_1921,
    calls: &[
        (0x37, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (
            0x4c,
            Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0475_27E0),
        ),
        (0xb7, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0xe0, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        (0xfa, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        (0x113, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x120, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
