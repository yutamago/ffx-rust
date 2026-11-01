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

/// The effect's start (0x100013e0, no arguments; a /GS frame with the
/// actor's point `L` at `ebp - 0x14` (`w` 1.0) and the angle at `ebp -
/// 0x18`): three buffer globals and two words from immediates; the
/// actors looked up ([`crate::sb_misc::lookup_actors`]); slot 719 `(P, 8,
/// 0)` into `G_H`; the part set up ([`crate::l5_objhost::part_init`]);
/// slot 262 `(*G_A, &L.x, &L.y, &L.z)`; the angle `K0 - slot 333 (*G_A)`
/// wrapped once. Four objects ([`crate::s4_misc::start_object_anim`],
/// parts 2, 0, 3, 1 with the values 0, 30, 150, 300; two never-written
/// stack words and the saved registers under them), each given `L` (the
/// fourth `L.y + DY` and no `w`) and the angle, the second also a word
/// from an immediate; a fifth record `L` and the angle; slot 345 `(*G_O)`
/// and slot 349 `(*G_O, *G_C)`; a word 120; two fade tasks
/// ([`crate::s3_misc::fade_task`] on `S1` `(60, K1, K2, 0)` and `S2`
/// `(30, K2, K1, 450)`); slot 271 `(4)`, slot 525 `(*G_A)`, slot 220
/// `(slot 719 (P, 6, 0), 0x40, 0x100)`; the actor block
/// ([`crate::sa_misc::me_block`] `(B)`); two task nodes
/// ([`crate::sa_pool::take`], callbacks `CB1`, `CB2`): the first linked
/// after the list's last node without moving `G_LAST` (or the list's
/// first), the second after it. Each slot call sees the words the
/// earlier ones left (down to the never-written ones). Returns 0.
#[allow(clippy::too_many_lines)] // the original's sequence, one by one
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::pushes::Pushes;
    use crate::r12h_start::{angle_from, konst, wrap};
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    for o in [0x10u32, 0x1a, 0x24, 0x2e, 0x38] {
        g.write::<u32>(w(o + 2), w(o + 6));
    }
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + 0x43));
    let mut p = Pushes::default();
    p.push_all(&[0, 8, w(0x57)]);
    let t = slot_target(g, w(0x48), 719);
    // call width: 12 (Pushes caps the words at 12)
    let h = p.call(g, t, 0).eax;
    p.pop(1);
    g.write::<u32>(w(0x61), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x66));
    let ebp = fr.esp.wrapping_sub(4);
    let l = ebp.wrapping_sub(0x14);
    let actor = g.read::<u32>(w(0x7d));
    p.push_all(&[l + 8, l + 4, l, actor]);
    let t = slot_target(g, w(0x77), 262);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    p.push(g.read::<u32>(w(0x90)));
    g.write::<u32>(l + 0xc, 0x3f80_0000);
    let a = angle_from(g, img, f, &p, (0x8a, 0xaa));
    let a = wrap(g, img, f, a, [0xb9, 0xc8, 0xd2, 0xdf]);
    g.write::<f32>(ebp.wrapping_sub(0x18), a);
    let point = |g: &G| [0u32, 4, 8, 0xc].map(|o| g.read::<u32>(l + o));
    let object = |g: &mut G, call: u32, edx: u32, k: u32, (index, value): (u32, u32)| {
        let scale = konst(g, img, f, k);
        crate::s4_misc::start_object_anim(
            g,
            img,
            img.rel32_target(f + call),
            w(edx),
            scale,
            index,
            value,
        );
    };
    object(g, 0xff, 0xf6, 0xec, (2, 0));
    let v = point(g);
    for (op, x) in [
        (0x114u32, v[0]),
        (0x121, v[1]),
        (0x12e, v[2]),
        (0x13e, v[3]),
    ] {
        g.write::<u32>(w(op), x);
    }
    g.write::<f32>(w(0x154), a);
    g.write::<u32>(w(0x15a), w(0x15e));
    object(g, 0x163, 0x14c, 0x10c, (0, 0x1e));
    let v = point(g);
    for (op, x) in [
        (0x178u32, v[0]),
        (0x185, v[1]),
        (0x192, v[2]),
        (0x1a2, v[3]),
    ] {
        g.write::<u32>(w(op), x);
    }
    g.write::<f32>(w(0x1bb), a);
    object(g, 0x1c0, 0x1b3, 0x170, (3, 0x96));
    let v = point(g);
    for (op, x) in [(0x1d5u32, v[0]), (0x1e2, v[2]), (0x1ef, v[3])] {
        g.write::<u32>(w(op), x);
    }
    g.write::<f32>(w(0x1fc), a);
    let y = add(f32::from_bits(v[1]), konst(g, img, f, 0x209));
    g.write::<f32>(w(0x220), y);
    object(g, 0x225, 0x211, 0x1cd, (1, 0x12c));
    let v = point(g);
    for (op, x) in [
        (0x23du32, v[0]),
        (0x24a, v[1]),
        (0x257, v[2]),
        (0x264, v[3]),
    ] {
        g.write::<u32>(w(op), x);
    }
    g.write::<f32>(w(0x271), a);
    // the saved esi/edi and `sub esp, 8` under the objects' words: the
    // calls get the words above them
    let o = g.read::<u32>(w(0x235));
    let mut p = Pushes::default();
    p.push(o);
    let t = slot_target(g, w(0x22f), 345);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    let c = g.read::<u32>(w(0x280));
    p.push_all(&[c, o]);
    let t = slot_target(g, w(0x285), 349);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    g.write::<u32>(w(0x294), w(0x298));
    // the fade tasks: their word on the stack (ebp, the 0x18 frame, two
    // saved registers, 8 bytes, the slots' three words, the return
    // address)
    for (state, edx, xmm1, xmm2, word, call, esp) in [
        (
            0x2b4u32,
            0x3c,
            0x2af,
            0x2a7,
            0,
            0x2b9,
            fr.esp.wrapping_sub(0x40),
        ),
        (
            0x2d8,
            0x1e,
            0x2c9,
            0x2c1,
            0x1c2,
            0x2dd,
            fr.esp.wrapping_sub(0x44),
        ),
    ] {
        p.push(word);
        g.write::<u32>(esp + 4, word);
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: w(state),
            edx,
            xmm: [
                0,
                u128::from(g.read::<u32>(w(xmm1))),
                u128::from(g.read::<u32>(w(xmm2))),
                0,
            ],
            ..Frame::default()
        };
        crate::s3_misc::fade_task(g, img, &callee);
    }
    p.push(4);
    let t = slot_target(g, w(0x2e2), 271);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    p.push(g.read::<u32>(w(0x2f7)));
    let t = slot_target(g, w(0x2f1), 525);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    p.push_all(&[0, 6, w(0x313)]);
    let t = slot_target(g, w(0x304), 719);
    // call width: 12 (Pushes caps the words at 12)
    let h2 = p.call(g, t, 0).eax;
    p.push_all(&[0x100, 0x40, h2]);
    let t = slot_target(g, w(0x322), 220);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, t, 0);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x337), w(0x332));
    let first = crate::sa_pool::take(g, img, img.rel32_target(f + 0x344), w(0x33f));
    let mut head = g.read::<u32>(w(0x34a));
    g.write::<u32>(first.wrapping_add(4), 0);
    if head == 0 {
        head = first;
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0x372), first);
    } else {
        let last = g.read::<u32>(w(0x35d));
        g.write::<u32>(first, last);
        g.write::<u32>(last.wrapping_add(4), first);
    }
    g.write::<u32>(first.wrapping_add(8), first);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0x37f), w(0x377));
    g.write::<u32>(second.wrapping_add(4), 0);
    // the head is only 0 here when the first take gave 0 (which faults
    // before in the original); ported as written
    if head == 0 {
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x39c), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first.wrapping_add(4), second);
    }
    g.write::<u32>(second.wrapping_add(8), second);
    g.write::<u32>(w(0x3a8), second);
    Ret::eax(0)
}

/// [`start`] (954 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 954,
    hash: 0x1b66_a7b3_97d6_f186,
    calls: &[
        (0x43, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x66, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xff, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x163, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x1c0, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x225, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x2b9, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x2dd, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x337, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x344, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x37f, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0x3b2, Callee::Any),
    ],
    imports: &[],
};
